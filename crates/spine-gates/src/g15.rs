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
    /// The list names an artifact for this version and platform, and its digest
    /// is not the running binary's own bytes. CI §5.5's second party
    /// disagreeing with the first.
    SelfBytesMismatch,
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
            G15Status::SelfBytesMismatch => "self-bytes-mismatch",
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
    /// **The running binary's own version.**
    ///
    /// Without it there is no membership test. PB §6.3 says "the **running
    /// binary's** platform artifact is listed in trunk's pinned list", and an
    /// artifact is named `spine-<version>-<target>.tar.gz` (CI §5.5) — so the
    /// version is half of the thing whose membership is in question.
    ///
    /// Asking only whether the list holds *some* artifact for this platform is
    /// a question every release answers yes to, for every binary: a 1.3.0 or a
    /// 1.5.0 laptop binary against a 1.4.0 pin fetches the 1.4.0 list, hashes
    /// it to the pin, finds `spine-1.4.0-<target>.tar.gz`, and passes. PB
    /// §7.5's skew table makes the *newer* row an explicit **fail (G15)** —
    /// "CI runs the pinned hash or nothing" — and G15 is on no bypass list
    /// precisely because nothing else catches it.
    ///
    /// This is still a **membership test and not a comparison**: the version
    /// is compared for *equality* inside an artifact name, never ordered. "No
    /// ordering on `cli.version` is defined anywhere in this design and §7.5
    /// relies on there being none" — a `newer` binary fails not because it is
    /// greater but because its artifact is absent from this list.
    pub running_version: &'a str,
    /// The SHA-256 of the running binary's own bytes, where it knows them.
    ///
    /// CI §5.5: "The binary independently verifies its own bytes against the
    /// same list at start-up … **so the check is made twice by two different
    /// parties**." Start-up checks against the list the binary was fetched
    /// with; this checks against *trunk's pinned* list, which is the different
    /// party. `None` where the binary cannot read its own image.
    pub running_self_sha256: Option<&'a str>,
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
                    // The membership test, over the running binary's OWN
                    // artifact name — `spine-<version>-<target>.tar.gz`
                    // (CI §5.5) — and not over "any artifact for this
                    // platform", which every release satisfies for every
                    // binary.
                    let wanted = format!("spine-{}-{target}.tar.gz", input.running_version);
                    match list.entries.iter().find(|e| e.name == wanted) {
                        None => findings.push(Finding::outright(G15Status::ArtifactNotListed)),
                        Some(entry) => {
                            // CI §5.5's second party. Only when the binary
                            // knows its own bytes; absence is not a finding,
                            // because start-up already made the first check and
                            // §5.5 asks for two, not for this one twice.
                            if let Some(self_sha) = input.running_self_sha256
                                && self_sha != entry.sha256
                            {
                                findings
                                    .push(Finding::outright(G15Status::SelfBytesMismatch));
                            }
                        }
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

    /// The running binary defaults to the pinned version — the ordinary case,
    /// where the tool CI installed is the tool the pin names.
    fn input<'a>(list: Option<&'a [u8]>, target: Option<&'a str>, dist: &'a str) -> G15Input<'a> {
        running("1.4.0", list, target, dist)
    }

    /// The same, with the running binary's version made explicit — which is
    /// what G15's membership test is actually over.
    fn running<'a>(
        version: &'a str,
        list: Option<&'a [u8]>,
        target: Option<&'a str>,
        dist: &'a str,
    ) -> G15Input<'a> {
        G15Input {
            cli_version: "1.4.0",
            cli_dist_hash: dist,
            artifact_list: list,
            running_target: target,
            running_version: version,
            running_self_sha256: None,
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
    /// the same one an older binary gets. Nothing here compares two versions —
    /// the equality is inside an artifact name.
    ///
    /// **This test asserted `Pass` until 2026-08-28**, with a body
    /// byte-identical to the passing test above and a comment conceding "the
    /// version is never consulted". It claimed a rule it inverted, and the rule
    /// it claimed is the one PB §7.5's skew table makes an explicit
    /// `fail (G15) — CI runs the pinned hash or nothing`.
    #[test]
    fn a_newer_binary_is_not_a_greater_binary_it_is_an_unlisted_one() {
        let dist = ArtifactList::dist_hash(list_bytes());
        for version in ["1.5.0", "1.3.0", "1.4.0+local", "0.0.0-synthetic"] {
            let verdict = evaluate(
                &running(version, Some(list_bytes()), Some("aarch64-apple-darwin"), &dist),
                &Reviews::default(),
            );
            assert_eq!(
                verdict.status,
                GateStatus::Fail,
                "a {version} binary against a 1.4.0 pin is unlisted"
            );
            assert_eq!(verdict.statuses(), [&G15Status::ArtifactNotListed]);
        }

        // And the pinned version still passes, so the test separates the two
        // rather than failing everything.
        let verdict = evaluate(
            &running("1.4.0", Some(list_bytes()), Some("aarch64-apple-darwin"), &dist),
            &Reviews::default(),
        );
        assert_eq!(verdict.status, GateStatus::Pass);
    }

    /// CI §5.5: "The binary independently verifies its own bytes against the
    /// same list at start-up … so the check is made twice by two different
    /// parties." Start-up checks against the list the binary was fetched with;
    /// G15 checks against trunk's pinned list, which is the other party.
    #[test]
    fn a_listed_name_whose_digest_is_not_our_bytes_fails() {
        let dist = ArtifactList::dist_hash(list_bytes());
        let mut input = running("1.4.0", Some(list_bytes()), Some("aarch64-apple-darwin"), &dist);

        // The list's entry for this artifact is all-ones.
        input.running_self_sha256 = Some(
            "1111111111111111111111111111111111111111111111111111111111111111",
        );
        assert_eq!(
            evaluate(&input, &Reviews::default()).status,
            GateStatus::Pass
        );

        input.running_self_sha256 = Some(
            "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        );
        let verdict = evaluate(&input, &Reviews::default());
        assert_eq!(verdict.status, GateStatus::Fail);
        assert_eq!(verdict.statuses(), [&G15Status::SelfBytesMismatch]);

        // Absence is not a finding: start-up already made the first check, and
        // §5.5 asks for two parties, not for this one twice.
        input.running_self_sha256 = None;
        assert_eq!(
            evaluate(&input, &Reviews::default()).status,
            GateStatus::Pass
        );
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
