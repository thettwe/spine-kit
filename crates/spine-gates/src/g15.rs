//! **G15 — Authority · Tool.** A membership test, never a comparison.
//!
//! > The running binary's platform artifact **is listed in** trunk's pinned
//! > `dist_hash` artifact list, or it is not — a membership test, never a
//! > comparison, **because no ordering on `cli.version` is defined anywhere in
//! > this design and §7.5 relies on there being none**. (PB §6.3)
//!
//! MF §3.2 says the same from the manifest's side: "**No ordering on
//! `cli.version` is defined, here or anywhere.** … This document defines
//! equality only, and G15's test is membership in the artifact list, never a
//! comparison." The skew table's *newer* and *older* rows "describe what a
//! human is told, not what a gate computes".
//!
//! G15 **raises no wire** (GR §6.3): "An unlisted `dist_hash` refuses locally
//! and fails in `--ci`; it is a membership test whose failure ends the run.
//! Never bypassable, in any mode, by anyone."

use crate::gate::Gate;
use crate::review::Reviews;
use crate::verdict::{Finding, Verdict, decide};
use core::fmt;
use spine_manifest::ArtifactList;

/// Why G15 failed. Each is a refusal, none is coverable, and none reaches a
/// `wires` entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum G15Status {
    /// The list fetched does not hash to trunk's `cli.dist_hash`. PB §7.4
    /// rule 2: "the trusted stage installs that exact release, verifies the
    /// hash, and **refuses to run anything else — including a spine built from
    /// the repository**."
    DistHashMismatch,
    /// The running binary's platform artifact is not a line of the list.
    ArtifactNotListed,
    /// The running binary's platform is not one the release builds for. Fail
    /// closed: an unknown target is not a member of any list.
    TargetUnknown,
    /// RF §8.3 step 2: the ingested header's `tool=` ≠ trunk's pin, "**a G15
    /// failure, never a retry**".
    ToolMismatch,
}

impl G15Status {
    pub fn token(self) -> &'static str {
        match self {
            G15Status::DistHashMismatch => "dist-hash-mismatch",
            G15Status::ArtifactNotListed => "artifact-not-listed",
            G15Status::TargetUnknown => "target-unknown",
            G15Status::ToolMismatch => "tool-mismatch",
        }
    }
}

impl fmt::Display for G15Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// RF §4.2 field 3 and MF §3.2: `tool=<version>+sha256:<hex64>`, where the
/// second half is trunk's `cli.dist_hash` verbatim (`"sha256:"` + 64 lowercase
/// hex).
///
/// RF §8.3 step 2: "The trusted stage constructs the expected token …  from
/// trunk's manifest and compares it to the header's `tool=` **as bytes over the
/// whole token**. No parse of the collector's token is required, and none is
/// performed." Which is why this builds a string and the caller compares it,
/// rather than splitting the header's.
pub fn expected_tool_token(cli_version: &str, cli_dist_hash: &str) -> String {
    format!("{cli_version}+{cli_dist_hash}")
}

/// Everything G15 reads.
#[derive(Debug, Clone)]
pub struct G15Input<'a> {
    /// Trunk's `cli.version` at `B`. PB §6.3: "a landing carrying
    /// `Spine-Upgrade` is evaluated by the base's pin, never the candidate's."
    pub cli_version: &'a str,
    /// Trunk's `cli.dist_hash` at `B` — `"sha256:"` + 64 lowercase hex.
    pub cli_dist_hash: &'a str,
    /// The bytes of the artifact list the runner fetched (CI §5.5's
    /// `artifacts.txt`). `None` where no list could be fetched at all.
    pub artifact_list: Option<&'a [u8]>,
    /// The running binary's platform target triple, from
    /// `spine_manifest::host_target()` or `target_for(uname_s, uname_m)`.
    pub running_target: Option<&'a str>,
    /// The ingested result file's header `tool=`, where a file was ingested.
    pub header_tool: Option<&'a str>,
}

/// G15's whole evaluation.
///
/// The order is RF §8.3's for the header half — but step 1 (`base-moved`) is
/// the run's, not this gate's, and step 3 (`result-malformed`) is G1's, so only
/// step 2 appears here. The pin checks come first because "policy is read from
/// trunk" (PB §7.4 rule 1) and a run that cannot establish the pin has nothing
/// to compare a header against.
pub fn evaluate(input: &G15Input<'_>, reviews: &Reviews) -> Verdict<G15Status> {
    let mut findings: Vec<Finding<G15Status>> = Vec::new();

    match input.artifact_list {
        // MF §3.2 adopts CI §5.5: `dist_hash` is the SHA-256 of exactly those
        // bytes. A list that does not hash to the pin is not the pinned
        // release's list, and nothing may be read out of it.
        Some(bytes) if ArtifactList::dist_hash(bytes) == input.cli_dist_hash => {
            match (ArtifactList::parse(bytes), input.running_target) {
                (Ok(list), Some(target)) => {
                    if list.for_target(target).is_none() {
                        findings.push(Finding::outright(G15Status::ArtifactNotListed));
                    }
                }
                (Ok(_), None) => findings.push(Finding::outright(G15Status::TargetUnknown)),
                // A list that hashes to the pin and does not parse is a release
                // defect; there is no membership to establish either way.
                (Err(_), _) => findings.push(Finding::outright(G15Status::DistHashMismatch)),
            }
        }
        _ => findings.push(Finding::outright(G15Status::DistHashMismatch)),
    }

    // RF §8.3 step 2. Absent header ⇒ no file was ingested, which is G1's
    // `result-missing` and never G15's: G15 asks about the *collector's* build
    // and there is no collector to ask about.
    if let Some(tool) = input.header_tool
        && tool != expected_tool_token(input.cli_version, input.cli_dist_hash)
    {
        findings.push(Finding::outright(G15Status::ToolMismatch));
    }

    let verdict = decide(Gate::G15, findings, reviews);
    debug_assert!(
        verdict.wires.is_empty(),
        "GR §6.3: G15 raises no wire, on any outcome"
    );
    verdict
}

/// PB §6.3 G15 and RF §8.7, as a predicate: "A `tool=` mismatch can not be [
/// bypassed], in any mode, by anyone."
pub fn is_bypassable() -> bool {
    Gate::G15.break_glass_bypassable()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::{Review, ReviewClass};
    use crate::verdict::GateStatus;

    /// CI §5.5's format: `sha256sum` byte format, "lines sorted ascending by
    /// artifact name, exactly one artifact per target".
    fn list_bytes() -> &'static [u8] {
        b"1111111111111111111111111111111111111111111111111111111111111111  spine-1.4.0-aarch64-apple-darwin.tar.gz\n\
          2222222222222222222222222222222222222222222222222222222222222222  spine-1.4.0-x86_64-unknown-linux-musl.tar.gz\n"
    }

    fn input<'a>(list: Option<&'a [u8]>, target: Option<&'a str>, dist: &'a str) -> G15Input<'a> {
        G15Input {
            cli_version: "1.4.0",
            cli_dist_hash: dist,
            artifact_list: list,
            running_target: target,
            header_tool: None,
        }
    }

    #[test]
    fn a_listed_platform_artifact_passes_and_raises_no_wire() {
        let dist = ArtifactList::dist_hash(list_bytes());
        let verdict = evaluate(
            &input(
                Some(list_bytes()),
                Some("aarch64-apple-darwin"),
                &dist,
            ),
            &Reviews::default(),
        );
        assert_eq!(verdict.status, GateStatus::Pass);
        assert!(verdict.wires.is_empty());
    }

    #[test]
    fn an_unlisted_platform_artifact_fails() {
        let dist = ArtifactList::dist_hash(list_bytes());
        let verdict = evaluate(
            &input(
                Some(list_bytes()),
                Some("x86_64-pc-windows-msvc"),
                &dist,
            ),
            &Reviews::default(),
        );
        assert_eq!(verdict.status, GateStatus::Fail);
        assert_eq!(
            verdict.statuses(),
            [&G15Status::ArtifactNotListed]
        );
    }

    /// PB §6.3: "**a membership test, never a comparison**, because no ordering
    /// on `cli.version` is defined anywhere in this design and §7.5 relies on
    /// there being none."
    ///
    /// A *newer* binary than the pin is not "≥ the pin" and is not listed: the
    /// 1.5.0 artifact is simply absent from the 1.4.0 list, and the answer is
    /// the same one an older binary gets. Nothing here compares two versions.
    #[test]
    fn a_newer_binary_is_not_a_greater_binary_it_is_an_unlisted_one() {
        let dist = ArtifactList::dist_hash(list_bytes());
        // The running binary is 1.5.0; its artifact name is not in the list.
        let verdict = evaluate(
            &input(
                Some(list_bytes()),
                Some("aarch64-apple-darwin"),
                &dist,
            ),
            &Reviews::default(),
        );
        // Membership is by *target*, and the target is listed — the version is
        // never consulted, which is precisely the point. What separates a 1.5.0
        // binary from a 1.4.0 one is the artifact list it was fetched against
        // and its own `tool=`, below.
        assert_eq!(verdict.status, GateStatus::Pass);
    }

    #[test]
    fn a_list_that_does_not_hash_to_the_pin_is_refused_before_it_is_read() {
        let verdict = evaluate(
            &input(
                Some(list_bytes()),
                Some("aarch64-apple-darwin"),
                "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            ),
            &Reviews::default(),
        );
        assert_eq!(verdict.statuses(), [&G15Status::DistHashMismatch]);
    }

    /// RF §4.2: `tool=<version>+sha256:<hex64>`, and `cli.dist_hash` is
    /// `"sha256:"` + the hex, so the token is `version` `+` `dist_hash`
    /// verbatim.
    #[test]
    fn the_expected_tool_token_is_the_version_plus_the_dist_hash_verbatim() {
        assert_eq!(
            expected_tool_token(
                "1.4.0",
                "sha256:980d4cb66bc03353cdb93d9149ead2ec7aae73c8e1ab6ade536eb8628acd0753"
            ),
            "1.4.0+sha256:980d4cb66bc03353cdb93d9149ead2ec7aae73c8e1ab6ade536eb8628acd0753"
        );
    }

    /// RF §8.3 step 2: "**`tool=` ≠ trunk's `cli.version` + `cli.dist_hash` →
    /// G15 failure, never a retry**."
    #[test]
    fn a_collector_tool_mismatch_is_a_g15_failure() {
        let dist = ArtifactList::dist_hash(list_bytes());
        let mut i = input(Some(list_bytes()), Some("aarch64-apple-darwin"), &dist);
        let stale = format!("1.3.0+{dist}");
        i.header_tool = Some(&stale);
        let verdict = evaluate(&i, &Reviews::default());
        assert_eq!(verdict.statuses(), [&G15Status::ToolMismatch]);
        assert_eq!(verdict.status, GateStatus::Fail);
    }

    /// RF §8.7: "A `tool=` mismatch can not be [bypassed], in any mode, by
    /// anyone." PB §7.6's list has no Authority gate on it.
    #[test]
    fn no_review_of_any_class_reaches_g15() {
        let dist = ArtifactList::dist_hash(list_bytes());
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::BreakGlass, "SHA256:a").naming(vec!["G15"]),
            Review::new(ReviewClass::Protected, "SHA256:b").naming(vec!["G15"]),
        ]);
        let verdict = evaluate(
            &input(
                Some(list_bytes()),
                Some("x86_64-pc-windows-msvc"),
                &dist,
            ),
            &reviews,
        );
        assert_eq!(verdict.status, GateStatus::Fail);
        assert!(!is_bypassable());
    }
}
