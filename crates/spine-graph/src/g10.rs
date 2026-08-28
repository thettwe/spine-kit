//! G10 — Reconstruction. The comparison, and what a failure costs.
//!
//! **G10 is unbypassable, terminal, and never re-queued.** PB §6.3's own words:
//! a failure *"**refuses the push**, ends the run as `reconstruction-failed`
//! without a retry"*; PB §6's transition table gives the state one row —
//! *"reconstruction-failed — discarded, reported, never re-queued: a
//! deterministic failure re-runs identically"* — and no row out of it. Break
//! glass does not reach it: PB §7.6's bypass list names G1, G2, G3, G4, G6, G7,
//! G8 and G12, and G10 is not among them.
//!
//! **What that costs, stated plainly.** A landing that fails here is destroyed.
//! *"The discarded `L` never becomes a git object, so the run's own report is
//! the only record"* — there is no commit to inspect, no branch to re-run, and
//! the human who wrote the code sees a gate naming the *ledger* while the
//! defect is in the *indexer*. PB §6.3 says so: *"It is still an indexer defect
//! to file against spine, not a ledger defect — the envelope G9 accepted is
//! valid — but a landing a clean clone cannot reproduce does not reach trunk."*
//! So the price of one nondeterministic byte in this crate is not a flaky test;
//! it is a repository whose every landing dies, and dies pointing at the wrong
//! author.
//!
//! **Which is why the dump's determinism is the whole defence.** Nothing here
//! makes a difference legible, reconciles two graphs, or offers a diff to
//! adjudicate — DM §11 step 5 is *"`D_S == D_C` as byte strings. Nothing parses
//! either stream."* The gate cannot be made kinder; it can only be made
//! *never to fire spuriously*, and every rule of DM §10 — no wall clock, no
//! environment, no restored store, JCS key order, `esc`-byte record order, git
//! plumbing pinned by the release — exists to buy exactly that. The one
//! admitted exception is `changeset.tree`'s `unverifiable(git-version)`
//! sentinel (DM §7.2.1), which is safe here precisely because *"both sides run
//! one binary and one git on one host"*.
//!
//! There is also no deferred mode, and PB §6.3 explains the deletion: a
//! deferred proof's *failure* is not a git object, so *"a fresh clone could not
//! tell which landing failed"*. *"A repo too large to pay is a repo whose
//! landings are not proved reconstructible, and it should have to say that in
//! its own words rather than select it from a menu."*

use crate::derive::{Error, Indexer, Options};
use crate::dump::{Dump, G10, g10_compare, serialize};
use crate::verify::Verifier;
use crate::{git, status::Refusal};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The result of one comparison, with both streams kept.
///
/// The dumps travel with the verdict because the run's report is the only
/// record a failure leaves (PB §6.3), and a report that could not print the two
/// streams would leave nothing to file the indexer defect with.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub verdict: G10,
    pub source: Dump,
    pub clone: Dump,
}

impl Outcome {
    pub fn passed(&self) -> bool {
        self.verdict == G10::Pass
    }

    /// The first line at which the two streams differ, 1-based, or `None` when
    /// they do not.
    ///
    /// DM §11's second responsibility of the format: *"**A legible failure.**
    /// Because the streams are line-sorted JSONL, `diff` names the record. The
    /// header is line 1 in both, so an input-set disagreement — different tip,
    /// different trust root, different trunk name — surfaces before the body
    /// and can be read at a glance."* This is a diagnostic over bytes already
    /// compared; the verdict never consults it.
    pub fn first_differing_line(&self) -> Option<usize> {
        let s: Vec<&[u8]> = self.source.bytes().split(|&b| b == b'\n').collect();
        let c: Vec<&[u8]> = self.clone.bytes().split(|&b| b == b'\n').collect();
        (0..s.len().max(c.len()))
            .find(|&i| s.get(i) != c.get(i))
            .map(|i| i + 1)
    }
}

/// DM §11's procedure, over a scratch clone the runner has already built.
///
/// Step 1 — *"`L` is pushed into the scratch clone `S` as `refs/heads/<trunk>`
/// with the intent ref deleted, so `S` holds the post-CAS ref set both sides
/// index"* — is the runner's, because the candidate landing is the runner's and
/// does not exist in this crate. Steps 2 to 5 are here.
pub struct Comparison<'a> {
    /// *"the runner's pinned trust root"*, written into **both** sides.
    ///
    /// DM §11 step 3 and DM §14 D1: PB §6.3 named only the clone, and *"`S` is
    /// itself a fresh clone and carries no local config, and a side without a
    /// pin would trust on first use or refuse, either way diverging from the
    /// other on every landing (TOFU is for humans, never for G10)"*.
    pub trust_root: &'a str,
    pub verifier: &'a dyn Verifier,
}

impl core::fmt::Debug for Comparison<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Comparison")
            .field("trust_root", &self.trust_root)
            .finish_non_exhaustive()
    }
}

impl Comparison<'_> {
    /// Steps 2–5, returning the verdict and both streams.
    ///
    /// `scratch` is `S`; `clone_into` is a path that must not exist yet and
    /// that this call creates.
    pub fn run(&self, scratch: &Path, clone_into: &Path) -> Result<Outcome, Error> {
        let clone = self.clone_side(scratch, clone_into)?;
        // Step 3, in this order: the pin is written before either side is
        // indexed, and into both, so that neither can reach a keyring decision
        // without it.
        for side in [scratch, &clone] {
            set_trust_root(side, self.trust_root)?;
        }
        // Step 4: "`spine index --fresh --dump` runs in each". `--fresh` is not
        // a flag here because it is the only mode this crate has — DM §4.3
        // makes `--dump` imply it, and PB §7.4 rule 3 forbids a persisted,
        // fetched or restored store from reaching a dump at all.
        let source = self.dump_side(scratch)?;
        let clone = self.dump_side(&clone)?;
        // Step 5, and the version check inside it: DM §3.2 makes a skew between
        // two dumps of one process tree "a defect in that implementation", to
        // be refused rather than compared.
        let verdict = g10_compare(&source, &clone).map_err(Error::Refused)?;
        Ok(Outcome {
            verdict,
            source,
            clone,
        })
    }

    /// Step 2, verbatim: *"`S` is cloned with `--no-local --no-hardlinks
    /// file://S`, `GIT_CONFIG_GLOBAL=/dev/null`, no network, default refs only
    /// — no notes, no custom refs, no provider metadata."*
    ///
    /// `--no-local` is what makes this a real clone rather than a hardlink farm
    /// of `S`'s object database: the point of the exercise is that a second
    /// repository, built the way a stranger's `git clone` builds one, derives
    /// the same graph.
    fn clone_side(&self, scratch: &Path, into: &Path) -> Result<PathBuf, Error> {
        let url = format!("file://{}", scratch.display());
        let out = Command::new("git")
            // The system config is neutralised beside the global one: DM §4.3
            // allows a dump to depend on `extensions.objectFormat` and
            // `spine.trustRoot` and on no other config, and a machine-wide
            // `[init] defaultBranch` or `[core] symlinks` would otherwise
            // reach one side and not the other.
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            // No network: a `file://` clone needs none, and a prompt would hang
            // a run whose only output is a terminal gate verdict.
            .env("GIT_TERMINAL_PROMPT", "0")
            .args(["clone", "--no-local", "--no-hardlinks", "--quiet", &url])
            .arg(into)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| Error::Git(git::GitError::NotAvailable(e.to_string())))?;
        if !out.status.success() {
            return Err(Error::Git(git::GitError::Failed {
                argv: format!("clone --no-local --no-hardlinks {url}"),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            }));
        }
        Ok(into.to_path_buf())
    }

    fn dump_side(&self, dir: &Path) -> Result<Dump, Error> {
        let repo = git::Repo::open(dir)?;
        let indexed = Indexer::new(&repo, self.verifier).index(&Options {
            trunk: None,
            trust_root: Some(self.trust_root.to_string()),
        })?;
        serialize(&indexed.header, &indexed.graph).map_err(Error::Refused)
    }
}

/// Write `spine.trustRoot` into a repository's own config.
///
/// It is one of exactly two config values a dump may depend on (DM §4.3), and
/// the only one a caller sets.
pub fn set_trust_root(dir: &Path, trust_root: &str) -> Result<(), Error> {
    git::Repo::open(dir)?.run_bytes(&["config", "spine.trustRoot", trust_root])?;
    Ok(())
}

/// The refusal a caller reports when a comparison could not even be attempted.
///
/// Kept distinct from the verdict on purpose: a failure to *run* G10 is not a
/// `reconstruction-failed` landing, and reporting it as one would blame the
/// ledger for a broken runner.
pub fn refusal(status: crate::status::Status, where_: impl Into<String>) -> Refusal {
    Refusal::new(status, where_)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dump::Header;
    use crate::store::Graph;
    use spine_canon::ObjectFormat;

    fn header(trunk: &str) -> Header {
        Header {
            object_format: ObjectFormat::Sha1,
            repo: "myrepo".into(),
            trunk: trunk.as_bytes().to_vec(),
            head: None,
            trust_root: None,
        }
    }

    #[test]
    fn two_empty_dumps_of_one_tip_are_a_pass() {
        // DM §9: "G10 comparing two of them is a pass, and correctly so: two
        // clones that both derive nothing from the same tip agree."
        let d = serialize(&header("main"), &Graph::new()).unwrap();
        let outcome = Outcome {
            verdict: g10_compare(&d, &d).unwrap(),
            source: d.clone(),
            clone: d,
        };
        assert!(outcome.passed());
        assert_eq!(outcome.first_differing_line(), None);
    }

    #[test]
    fn a_trunk_name_disagreement_surfaces_on_line_1() {
        // DM §11: "The header is line 1 in both, so an input-set disagreement —
        // different tip, different trust root, different trunk name — surfaces
        // before the body and can be read at a glance."
        let s = serialize(&header("main"), &Graph::new()).unwrap();
        let c = serialize(&header("trunk"), &Graph::new()).unwrap();
        let outcome = Outcome {
            verdict: g10_compare(&s, &c).unwrap(),
            source: s,
            clone: c,
        };
        assert_eq!(outcome.verdict, G10::ReconstructionFailed);
        assert_eq!(outcome.verdict.token(), "reconstruction-failed");
        assert_eq!(outcome.first_differing_line(), Some(1));
    }
}
