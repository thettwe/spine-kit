//! `spine check --verify <landing-sha>` — GR §4's outcomes, exit codes and
//! **normative order**.
//!
//! GR §4.3: "**The order is normative**, because two implementations that check
//! the same things in a different order report different statuses for a clone
//! that is wrong in two ways at once."
//!
//! What this module owns is the decision procedure. What it does not own is any
//! I/O: it neither reads `refs/notes/spine` nor runs `git`, because GR §4.4.6
//! is emphatic that a note "is never a source" and the discipline that keeps
//! that true is that the *candidate is an argument* — "it is admitted only
//! because a signed trailer already fixes its digest, every recomputable member
//! is then thrown away and rebuilt, the attested members are copied in, and the
//! result is hashed against `report=` in that *signed* trailer."

use core::fmt;

use crate::ids::Sha256Digest;
use crate::read::ReadError;
use crate::report::Report;

/// GR §4.3's table. `Display` is the exact status token, which reaches a user
/// and a CI log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyStatus {
    /// "Recomputed report's digest equals the seal's `report=`."
    Verified,
    /// "The candidate was the sealed report and the recomputation disagrees
    /// with it. The recomputed report is printed; the difference is a defect to
    /// file against spine, or evidence of tampering."
    ReportMismatch {
        sealed: Sha256Digest,
        recomputed: Sha256Digest,
    },
    /// "The candidate's own bytes do not hash to the seal's `report=` (§4.1).
    /// It is not the report this seal names — a stale, truncated or forged copy
    /// — and nothing was recomputed from it."
    CandidateMismatch {
        sealed: Sha256Digest,
        candidate: Sha256Digest,
    },
    /// "No candidate report: no `--report`, and no note on the landing in this
    /// clone (§4.4.4)."
    ReportUnavailable,
    /// GR §3.3: the running binary's platform artifact hash does not equal the
    /// `dist_hash` in the seal's `tool=`.
    WrongRelease,
    /// GR §3.3: `git --version`, parsed by GR §5.3's rule, does not equal the
    /// seal's `git=`. "PB §7.4 rule 4 makes this a requirement, not a warning,
    /// because `merge-tree` output is a git version's contract."
    WrongGit,
    /// GR §3.2's refusal, reached at step 4.
    ReportVersionUnknown,
    /// GR §4.2: "`objects.head` is unreachable and the evaluation needed it —
    /// a `land` under squash strategy."
    NotRecomputable,
}

impl VerifyStatus {
    /// GR §4.3's exit column.
    ///
    /// Two statuses share exit 1 and that is deliberate: "Both exit-1 statuses
    /// are failures of the *copy* or of the *record*, never of the landing."
    pub const fn exit_code(&self) -> i32 {
        match self {
            VerifyStatus::Verified => 0,
            VerifyStatus::ReportMismatch { .. } | VerifyStatus::CandidateMismatch { .. } => 1,
            VerifyStatus::ReportUnavailable => 2,
            VerifyStatus::WrongRelease
            | VerifyStatus::WrongGit
            | VerifyStatus::ReportVersionUnknown => 3,
            VerifyStatus::NotRecomputable => 4,
        }
    }

    /// The status token GR §4.3's middle column fixes.
    pub const fn token(&self) -> &'static str {
        match self {
            VerifyStatus::Verified => "verified",
            VerifyStatus::ReportMismatch { .. } => "report-mismatch",
            VerifyStatus::CandidateMismatch { .. } => "candidate-mismatch",
            VerifyStatus::ReportUnavailable => "report-unavailable",
            VerifyStatus::WrongRelease => "wrong-release",
            VerifyStatus::WrongGit => "wrong-git",
            VerifyStatus::ReportVersionUnknown => "report-version-unknown",
            VerifyStatus::NotRecomputable => "not-recomputable",
        }
    }
}

impl fmt::Display for VerifyStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// The three fields of the landing's `Spine-Seal` that `--verify` reads
/// (PB §11), and the two facts about the running environment it compares them
/// against.
#[derive(Debug, Clone)]
pub struct Preconditions<'a> {
    /// The `dist_hash` half of the seal's `tool=`.
    pub seal_dist_hash: &'a Sha256Digest,
    /// The seal's `git=`, in GR §5.3's `<major>.<minor>` form.
    pub seal_git_version: &'a str,
    /// The seal's `report=`.
    pub seal_report: &'a Sha256Digest,
    /// This binary's platform artifact hash.
    pub running_dist_hash: &'a Sha256Digest,
    /// This machine's `git --version`, parsed by [`crate::git_version::parse`].
    pub running_git_version: &'a str,
    /// Whether `objects.head` is reachable in this clone. Combined with
    /// [`Report::needs_head`], never read off `subject.strategy` (GR §4.2).
    pub head_reachable: bool,
}

/// The result of a verification, with the bytes a `report-mismatch` must print.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub status: VerifyStatus,
    /// GR §4.3: on `report-mismatch`, "The recomputed report is printed."
    pub recomputed: Option<Vec<u8>>,
}

impl Outcome {
    fn of(status: VerifyStatus) -> Self {
        Outcome {
            status,
            recomputed: None,
        }
    }
}

/// Run GR §4.3's six steps, in GR §4.3's order.
///
/// `rebuild` is step 6's first half: given the parsed candidate, return the
/// report this run recomputes — "every recomputable member is then thrown away
/// and rebuilt, the attested members are copied in" (GR §4.1). The attested set
/// is GR §4's table and is the caller's to copy, because only the caller knows
/// which members its gate engine rebuilt.
///
/// The steps, quoted:
///
/// 1. "the seal's `tool=` against the running binary, and its `git=` against
///    the parsed `git --version` (§3.3) — exit 3, **before any candidate is
///    read**, since neither depends on one";
/// 2. "resolve a candidate (§4.1) — exit 2 if there is none";
/// 3. "`sha256` over the candidate's exact bytes against the seal's `report=` —
///    exit 1 `candidate-mismatch`, **before the candidate is parsed**, because
///    bytes that are not the sealed report are not worth parsing";
/// 4. "parse it; an unknown `report_version` or an unknown member name — exit 3
///    `report-version-unknown`";
/// 5. "recomputability of the objects the evaluation needed (§4.2) — exit 4";
/// 6. "rebuild, copy the attested members in, canonicalize, compare — exit 0 or
///    exit 1 `report-mismatch`".
pub fn verify(
    pre: &Preconditions<'_>,
    candidate: Option<&[u8]>,
    rebuild: impl FnOnce(&Report) -> Report,
) -> Outcome {
    // Step 1. Release before git, because GR §4.3 lists them in that order
    // inside one numbered step and a clone can be wrong in both.
    if pre.seal_dist_hash != pre.running_dist_hash {
        return Outcome::of(VerifyStatus::WrongRelease);
    }
    if pre.seal_git_version != pre.running_git_version {
        return Outcome::of(VerifyStatus::WrongGit);
    }

    // Step 2.
    let Some(bytes) = candidate else {
        return Outcome::of(VerifyStatus::ReportUnavailable);
    };

    // Step 3. "The check is redundant with the final comparison — a candidate
    // that fails it could never produce a matching recomputed digest — and it
    // is required anyway, because it is the only check that can distinguish
    // *this is not the report the seal names* from *this is the sealed report
    // and it does not describe these objects*."
    let candidate_digest = Sha256Digest::of(bytes);
    if &candidate_digest != pre.seal_report {
        return Outcome::of(VerifyStatus::CandidateMismatch {
            sealed: pre.seal_report.clone(),
            candidate: candidate_digest,
        });
    }

    // Step 4.
    let parsed = match Report::from_canonical(bytes) {
        Ok(r) => r,
        // Both of GR §3.2's causes carry this one token; a malformed value that
        // is neither is not a version question, and refusing it as one would
        // tell the operator to install a different release. It is reported as
        // the same exit-3 class because GR fixes no other, and exit 3 is
        // "preconditions for recomputation not met" — which it is.
        Err(ReadError::ReportVersionUnknown(_)) | Err(_) => {
            return Outcome::of(VerifyStatus::ReportVersionUnknown);
        }
    };

    // Step 5. GR §4.2's predicate: the evaluation needed `objects.head` and
    // this clone cannot reach it. Not `subject.strategy` — "a tombstone's
    // `objects.tree` is `B`'s tree … a reseal's `Hc` is `O`, a first-parent
    // commit of trunk, which no landing deletes."
    if parsed.needs_head() && !pre.head_reachable {
        return Outcome::of(VerifyStatus::NotRecomputable);
    }

    // Step 6.
    let recomputed = rebuild(&parsed);
    let recomputed_bytes = recomputed.canonical_bytes();
    let recomputed_digest = Sha256Digest::of(&recomputed_bytes);
    if &recomputed_digest == pre.seal_report {
        Outcome::of(VerifyStatus::Verified)
    } else {
        Outcome {
            status: VerifyStatus::ReportMismatch {
                sealed: pre.seal_report.clone(),
                recomputed: recomputed_digest,
            },
            recomputed: Some(recomputed_bytes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(byte: u8) -> Sha256Digest {
        Sha256Digest::of(&[byte])
    }

    fn pre<'a>(
        seal_dist: &'a Sha256Digest,
        running_dist: &'a Sha256Digest,
        seal_git: &'a str,
        running_git: &'a str,
        seal_report: &'a Sha256Digest,
        head_reachable: bool,
    ) -> Preconditions<'a> {
        Preconditions {
            seal_dist_hash: seal_dist,
            seal_git_version: seal_git,
            seal_report,
            running_dist_hash: running_dist,
            running_git_version: running_git,
            head_reachable,
        }
    }

    /// GR §4.3's table, column by column.
    #[test]
    fn every_status_carries_its_exit_code() {
        let a = d(1);
        let b = d(2);
        assert_eq!(VerifyStatus::Verified.exit_code(), 0);
        assert_eq!(
            VerifyStatus::ReportMismatch {
                sealed: a.clone(),
                recomputed: b.clone()
            }
            .exit_code(),
            1
        );
        assert_eq!(
            VerifyStatus::CandidateMismatch {
                sealed: a,
                candidate: b
            }
            .exit_code(),
            1
        );
        assert_eq!(VerifyStatus::ReportUnavailable.exit_code(), 2);
        assert_eq!(VerifyStatus::WrongRelease.exit_code(), 3);
        assert_eq!(VerifyStatus::WrongGit.exit_code(), 3);
        assert_eq!(VerifyStatus::ReportVersionUnknown.exit_code(), 3);
        assert_eq!(VerifyStatus::NotRecomputable.exit_code(), 4);
    }

    /// GR §4.3: step 1 runs "**before any candidate is read**, since neither
    /// depends on one". A clone with the wrong release and no candidate reports
    /// `wrong-release`, not `report-unavailable`.
    #[test]
    fn the_release_check_precedes_candidate_resolution() {
        let seal = d(1);
        let running = d(2);
        let report = d(3);
        let p = pre(&seal, &running, "2.45", "2.45", &report, true);
        let out = verify(&p, None, |r| r.clone());
        assert_eq!(out.status, VerifyStatus::WrongRelease);
    }

    /// A clone that is wrong in both ways at once reports the release first —
    /// which is what "the order is normative" buys.
    #[test]
    fn a_clone_wrong_in_two_ways_reports_the_release() {
        let seal = d(1);
        let running = d(2);
        let report = d(3);
        let p = pre(&seal, &running, "2.45", "2.39", &report, true);
        assert_eq!(
            verify(&p, None, |r| r.clone()).status,
            VerifyStatus::WrongRelease
        );
    }

    /// GR §4.3 step 2: "no `--report`, and no note on the landing in this
    /// clone".
    #[test]
    fn no_candidate_is_report_unavailable() {
        let dist = d(1);
        let report = d(3);
        let p = pre(&dist, &dist, "2.45", "2.45", &report, true);
        let out = verify(&p, None, |r| r.clone());
        assert_eq!(out.status, VerifyStatus::ReportUnavailable);
        assert_eq!(out.status.exit_code(), 2);
    }

    /// GR §4.3 step 3: the candidate's own bytes are checked "**before the
    /// candidate is parsed**". Bytes that are not even JSON therefore report
    /// `candidate-mismatch`, not a parse failure.
    #[test]
    fn a_candidate_that_is_not_the_sealed_report_is_not_parsed() {
        let dist = d(1);
        let sealed = Sha256Digest::of(b"the sealed report");
        let p = pre(&dist, &dist, "2.45", "2.45", &sealed, true);
        let out = verify(&p, Some(b"not json at all"), |r| r.clone());
        match out.status {
            VerifyStatus::CandidateMismatch { candidate, .. } => {
                assert_eq!(candidate, Sha256Digest::of(b"not json at all"));
            }
            other => panic!("expected candidate-mismatch, got {other}"),
        }
    }

    /// GR §4.1: "It does not proceed, because a candidate that is not the
    /// sealed report supplies attested members that no signature covers, and
    /// running the recomputation on them would produce a second mismatch whose
    /// message names the wrong culprit."
    #[test]
    fn a_candidate_mismatch_never_reaches_the_rebuild() {
        let dist = d(1);
        let sealed = Sha256Digest::of(b"sealed");
        let p = pre(&dist, &dist, "2.45", "2.45", &sealed, true);
        verify(&p, Some(b"other"), |_| unreachable!("rebuild must not run"));
    }

    #[test]
    fn the_status_tokens_are_the_ones_gr_4_3_fixes() {
        assert_eq!(VerifyStatus::Verified.to_string(), "verified");
        assert_eq!(
            VerifyStatus::ReportUnavailable.to_string(),
            "report-unavailable"
        );
        assert_eq!(VerifyStatus::WrongRelease.to_string(), "wrong-release");
        assert_eq!(VerifyStatus::WrongGit.to_string(), "wrong-git");
        assert_eq!(
            VerifyStatus::ReportVersionUnknown.to_string(),
            "report-version-unknown"
        );
        assert_eq!(
            VerifyStatus::NotRecomputable.to_string(),
            "not-recomputable"
        );
    }
}
