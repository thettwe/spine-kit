//! Every status token the corpus fixes for a gate, as a typed, total enum.
//!
//! These tokens are not diagnostics. A `G13` refusal ends a run, a `G16`
//! refusal names the repair a human must make, and the run-level tokens are
//! *states* PB §11 enumerates and `spine stats` counts forever. Reporting the
//! wrong one means a reviewer signs the wrong wire.

use core::fmt;
use spine_manifest::keyring::Lint;

/// MF §4.8.4's status column, plus §4.4's keyring lint which check 1 adopts.
///
/// Every one is **outright** except check 2 over a commit whose signed line
/// claims none of the five roles a landing rests on (GR §5.6.1, MF §4.8.4) —
/// and that case is [`G13Status::StatementUnverified`] or
/// [`G13Status::StatementNamespace`] too. The kind is therefore a property of
/// the *finding*, never of the token: see [`crate::g13`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum G13Status {
    /// Check 1. `K` absent or failing §4.4's lint at §4.5's key count.
    Keyring(Lint),
    /// Check 2. "the bytes and the signature disagree" (MF §4.8.4).
    StatementUnverified,
    /// Check 2. "the key holds a role the trailer does not admit."
    StatementNamespace,
    /// Check 3.
    EventLineDuplicate,
    /// Check 4.
    ApprovalVoided,
    /// Check 5.
    ReopenVoidsMismatch,
    /// Checks 6 and 13 — one token, two situations (MF §4.8.4 "On check 6").
    ApproveReasonMissing,
    /// Check 7.
    SelfApprovedProtected,
    /// Check 8.
    WithdrawKey,
    /// Check 9, the signerless overlay.
    SignerlessReviewCount,
    /// Check 10, limb 1.
    ChainReviewNotInParent,
    /// Check 10, limb 2.
    ChainRemoverRemoved,
    /// Check 10, limb 3 — landed only.
    ChainSealNotInParent,
    /// Check 11, in flight only.
    TotalRoundsMismatch,
    /// Check 12, in flight only, at `--approve`.
    ApprovalRedundant,
}

impl G13Status {
    pub fn token(self) -> &'static str {
        match self {
            G13Status::Keyring(lint) => lint.token(),
            G13Status::StatementUnverified => "statement-unverified",
            G13Status::StatementNamespace => "statement-namespace",
            G13Status::EventLineDuplicate => "event-line-duplicate",
            G13Status::ApprovalVoided => "approval-voided",
            G13Status::ReopenVoidsMismatch => "reopen-voids-mismatch",
            G13Status::ApproveReasonMissing => "approve-reason-missing",
            G13Status::SelfApprovedProtected => "self-approved-protected",
            G13Status::WithdrawKey => "withdraw-key",
            G13Status::SignerlessReviewCount => "signerless-review-count",
            G13Status::ChainReviewNotInParent => "chain-review-not-in-parent",
            G13Status::ChainRemoverRemoved => "chain-remover-removed",
            G13Status::ChainSealNotInParent => "chain-seal-not-in-parent",
            G13Status::TotalRoundsMismatch => "total-rounds-mismatch",
            G13Status::ApprovalRedundant => "approval-redundant",
        }
    }
}

impl fmt::Display for G13Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// MF §5.9's three outright clauses. G14 has no coverable *status*: its
/// coverable findings are the floor hits, and a floor hit's identity is its
/// path, not a token from a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum G14Status {
    /// "`E(M_B) ⊄ E(M_T)` (manifest `paths.*` shrank), except under
    /// `Spine-Upgrade: to=none`."
    PathsShrank,
    /// "`C-A2` pattern set at `T` ⊉ at `B` (by bytes)."
    CA2Shrank,
    /// "A `C-A2` pattern with an ASCII uppercase letter inside a bracket
    /// expression … for `F0` this is a release-build assertion."
    CA2BracketCase,
    /// A floor hit. Coverable by a `class=protected` review naming
    /// `G14:` + `tok(path)` (MF §5.10).
    FloorHit,
}

impl G14Status {
    pub fn token(self) -> &'static str {
        match self {
            G14Status::PathsShrank => "paths-shrank",
            G14Status::CA2Shrank => "c-a2-shrank",
            G14Status::CA2BracketCase => "c-a2-bracket-case",
            G14Status::FloorHit => "floor-hit",
        }
    }
}

impl fmt::Display for G14Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// MF §6.2's status column, plus §6.5's constitution lint, §6.7's restoration
/// rule, §6.8's uninstall and §6.9's re-init.
///
/// Checks 2, 4, 5, 6, 7, 8, 11 and 12b resolve onto `docs/spec/manifest.md`
/// §3.11's closed list, which `spine-manifest` already owns — hence
/// [`G16Status::Manifest`] rather than a second spelling of thirty tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum G16Status {
    /// §3.11's closed list, reached by checks 2, 4–8, 11 and 12b.
    Manifest(spine_manifest::Status),
    /// Check 1, `to=none` limb: the manifest "must be **absent**".
    ManifestNotRemoved,
    /// Check 9.
    ScaffoldBlobMismatch,
    ScaffoldPathMissing,
    RegionMarkersMissing,
    RegionMarkersMalformed,
    RegionVersionMismatch,
    /// Check 10.
    ManifestChangedWithoutUpgrade,
    UpgradeWithoutManifestChange,
    UpgradeManifestMismatch,
    UpgradeVersionMismatch,
    ForcedDisagrees,
    /// Check 11b.
    ResignLowered,
    /// Check 12.
    LangsShrank,
    /// Check 13, over `K_T`.
    Keyring(Lint),
    /// Check 14.
    StagingResidue,
    /// Check 15, §6.5's lint.
    ConstitutionMissing,
    ConstitutionUnparseable,
    ConstitutionRuleMissing,
    ConstitutionRuleOutOfDomain,
    ConstitutionVersionRegressed,
    /// Check 16, §6.8.
    UninstallPathRemains,
    UninstallRegionRemains,
    UninstallUserOwnedTouched,
    UninstallKeyringChanged,
    UninstallConstitutionChanged,
    /// Check 17, §6.9.
    ReinitSinceMissing,
    ReinitSinceNotUninstall,
    ReinitKeyringDiffers,
    /// §6.7's rollback restoration rule — every step outright.
    RestoreAncestorUnreachable,
    RestoreAncestorManifestMalformed,
    RestoreNotOneStep,
    RestoreManifestDiffers,
    RestorePathsNotUnion,
    RestorePathNotRestored,
    RestorePathNotDeleted,
    RestoreUserOwnedTouched,
}

impl G16Status {
    pub fn token(self) -> &'static str {
        match self {
            G16Status::Manifest(status) => status.token(),
            G16Status::ManifestNotRemoved => "manifest-not-removed",
            G16Status::ScaffoldBlobMismatch => "scaffold-blob-mismatch",
            G16Status::ScaffoldPathMissing => "scaffold-path-missing",
            G16Status::RegionMarkersMissing => "region-markers-missing",
            G16Status::RegionMarkersMalformed => "region-markers-malformed",
            G16Status::RegionVersionMismatch => "region-version-mismatch",
            G16Status::ManifestChangedWithoutUpgrade => "manifest-changed-without-upgrade",
            G16Status::UpgradeWithoutManifestChange => "upgrade-without-manifest-change",
            G16Status::UpgradeManifestMismatch => "upgrade-manifest-mismatch",
            G16Status::UpgradeVersionMismatch => "upgrade-version-mismatch",
            G16Status::ForcedDisagrees => "forced-disagrees",
            G16Status::ResignLowered => "resign-lowered",
            G16Status::LangsShrank => "langs-shrank",
            G16Status::Keyring(lint) => lint.token(),
            G16Status::StagingResidue => "staging-residue",
            G16Status::ConstitutionMissing => "constitution-missing",
            G16Status::ConstitutionUnparseable => "constitution-unparseable",
            G16Status::ConstitutionRuleMissing => "constitution-rule-missing",
            G16Status::ConstitutionRuleOutOfDomain => "constitution-rule-out-of-domain",
            G16Status::ConstitutionVersionRegressed => "constitution-version-regressed",
            G16Status::UninstallPathRemains => "uninstall-path-remains",
            G16Status::UninstallRegionRemains => "uninstall-region-remains",
            G16Status::UninstallUserOwnedTouched => "uninstall-user-owned-touched",
            G16Status::UninstallKeyringChanged => "uninstall-keyring-changed",
            G16Status::UninstallConstitutionChanged => "uninstall-constitution-changed",
            G16Status::ReinitSinceMissing => "reinit-since-missing",
            G16Status::ReinitSinceNotUninstall => "reinit-since-not-uninstall",
            G16Status::ReinitKeyringDiffers => "reinit-keyring-differs",
            G16Status::RestoreAncestorUnreachable => "restore-ancestor-unreachable",
            G16Status::RestoreAncestorManifestMalformed => "restore-ancestor-manifest-malformed",
            G16Status::RestoreNotOneStep => "restore-not-one-step",
            G16Status::RestoreManifestDiffers => "restore-manifest-differs",
            G16Status::RestorePathsNotUnion => "restore-paths-not-union",
            G16Status::RestorePathNotRestored => "restore-path-not-restored",
            G16Status::RestorePathNotDeleted => "restore-path-not-deleted",
            G16Status::RestoreUserOwnedTouched => "restore-user-owned-touched",
        }
    }
}

impl fmt::Display for G16Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// G1's five pathless findings, and the two that also end the run.
///
/// RF §8.5: "**The bare `G1` is what is left, and it is a closed list of
/// five.** Each names no path, so under PB §11's *gates without a path use the
/// bare id* it takes the bare form."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum G1Status {
    /// RF §8.2: no result file at the fixed path. "a state is **not** entered."
    ResultMissing,
    /// RF §8.2: found, and §4's grammar or §8.3 step 3 rejected it.
    ResultMalformed,
    /// RF §8.5 clause 0. "raised *in addition to* whatever clauses 1–3 then
    /// raise, not instead of them."
    RunIncomplete,
    /// Clause 3: an AC with no collected `verified_by` edge.
    AcUncovered,
    /// Clause 1 where `P(R, F)` is empty: "the frozen entry collected nothing."
    FrozenIdUncollected,
    /// Clause 1: a frozen `Spine-Test` entry that collected and did not pass.
    FrozenIdNotPassed,
    /// Clause 2: a landed id that collected on `T` and did not pass.
    LandedIdNotPassed,
    /// Clause 2: a landed id gone from `T`'s collection.
    LandedIdWentAway,
}

impl G1Status {
    pub fn token(self) -> &'static str {
        match self {
            G1Status::ResultMissing => "result-missing",
            G1Status::ResultMalformed => "result-malformed",
            G1Status::RunIncomplete => "run-incomplete",
            G1Status::AcUncovered => "ac-uncovered",
            G1Status::FrozenIdUncollected => "frozen-id-uncollected",
            G1Status::FrozenIdNotPassed => "frozen-id-not-passed",
            G1Status::LandedIdNotPassed => "landed-id-not-passed",
            G1Status::LandedIdWentAway => "landed-id-went-away",
        }
    }

    /// The two the corpus fixes as *file* statuses rather than descriptions.
    /// RF §8.2 and §8.7 spell these; the other six are this crate's names for
    /// findings the corpus describes but does not tokenize, and they are
    /// recorded as such. **They never reach a wire** — a wire carries a gate id
    /// and a path, never a status (GR §6.1) — so naming them costs no digest.
    pub fn is_corpus_fixed(self) -> bool {
        matches!(self, G1Status::ResultMissing | G1Status::ResultMalformed)
    }
}

impl fmt::Display for G1Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// G8's clauses, each with its own `class` (GR §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum G8Status {
    /// "`T`'s blob equals it, or equals trunk's (harness moved → rerun +
    /// landing-review)" — the one `tripwire` clause.
    HarnessMoved,
    /// "one present at `B` whose blob in `T` differs from `B`'s — edited,
    /// deleted or renamed by the branch before approval."
    BranchEditedBeforeApproval,
    /// "a landed id `T` no longer collects or does not pass."
    LandedId,
    /// `C-T3`'s tree grep: "no test-framework import or hook definition outside
    /// the harness (`C-T1` ∪ `C-T2`)".
    FrameworkIsolation,
    /// "any `C-T1`/`C-T2`/runner-config path, or one frozen by an approval …
    /// whose blob in `T` differs from both the approval tree and trunk."
    /// **Outright** (GR §5.6.1), save on a reseal.
    DiffersFromBoth,
    /// "intent blob equals the signed blob." **Outright.**
    IntentBlobDiffers,
    /// "in `--ci` the closure recomputed by the pinned release ⊆
    /// `Spine-Frozen`" (PB §4.3).
    ClosureNotContained,
}

impl G8Status {
    pub fn token(self) -> &'static str {
        match self {
            G8Status::HarnessMoved => "harness-moved",
            G8Status::BranchEditedBeforeApproval => "branch-edited-before-approval",
            G8Status::LandedId => "landed-id",
            G8Status::FrameworkIsolation => "framework-isolation",
            G8Status::DiffersFromBoth => "differs-from-both",
            G8Status::IntentBlobDiffers => "intent-blob-differs",
            G8Status::ClosureNotContained => "closure-not-contained",
        }
    }

    /// GR §6.3's per-clause class column: "`tripwire` for the harness-moved
    /// clause; `protected` for the branch-edited-before-approval clause, the
    /// landed-id clause, and `C-T3`."
    ///
    /// DERIVED: GR §6.3 names four of the seven clauses. `differs-from-both`,
    /// `intent-blob-differs` and `closure-not-contained` are unnamed, and they
    /// take `protected`. Two reasons, both structural rather than a
    /// preference: `tripwire` is the *narrower* class — PB §11 makes
    /// `protected` dominate it, and a `tripwire` wire a `protected` review
    /// also discharges (`Review::admits`) — so guessing `tripwire` for an
    /// unnamed clause is the guess that can under-review a landing, while
    /// guessing `protected` can only over-review one. And the three unnamed
    /// clauses are the ones about the *intent's own identity* — a closure that
    /// escaped the declared set, a blob that is not the signed one — which is
    /// the company `branch-edited-before-approval` keeps, not `harness-moved`.
    ///
    /// The wildcard is deliberate for the same reason: a clause added later
    /// and left out of GR §6.3 lands on `protected` rather than on whatever
    /// the last arm happened to be.
    pub fn class(self) -> crate::wire::WireClass {
        match self {
            G8Status::HarnessMoved => crate::wire::WireClass::Tripwire,
            _ => crate::wire::WireClass::Protected,
        }
    }
}

impl fmt::Display for G8Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// Run-level states and refusals, from PB §11's *States* and the specs that
/// name each. These are not gate statuses: a gate's status is one of three
/// (GR §5.6.1), and these are what the *run* does about them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// GR §5.6.1: "A run that would seal a report containing a `fail`
    /// refuses."
    ReportNotLandable,
    /// PB §6.3 G9: a landing whose ledger walk fails, "reported and counted".
    Unattested,
    /// PB §5.5: a trunk commit pushed around the pipeline.
    Orphan,
    /// RF §8.3 step 1: `tree=`/`base=` ≠ the run's `(T, B)`.
    BaseMoved,
    /// PB §6.3 G11: re-verifications inside one run exceed `C-M3`.
    Starved,
    /// PB §6.3 G10: "A failure **refuses the push**, ends the run … without a
    /// retry."
    ReconstructionFailed,
    /// RF §8.7: promotion to the gated lane via `spine new --from <branch>`.
    Escalated,
}

impl RunStatus {
    pub fn token(self) -> &'static str {
        match self {
            RunStatus::ReportNotLandable => "report-not-landable",
            RunStatus::Unattested => "unattested",
            RunStatus::Orphan => "orphan",
            RunStatus::BaseMoved => "base-moved",
            RunStatus::Starved => "starved",
            RunStatus::ReconstructionFailed => "reconstruction-failed",
            RunStatus::Escalated => "escalated",
        }
    }
}

impl fmt::Display for RunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three MF §5.9 names, spelled exactly as `.build-notes` and MF §5.9
    /// print them. A reviewer's signed `wires=` never carries these, but a
    /// refusing run's message does, and `docs/spec/README.md` treats a token
    /// drift as a defect.
    #[test]
    fn g14s_outright_tokens_are_the_three_mf_5_9_names() {
        assert_eq!(G14Status::PathsShrank.to_string(), "paths-shrank");
        assert_eq!(G14Status::CA2Shrank.to_string(), "c-a2-shrank");
        assert_eq!(G14Status::CA2BracketCase.to_string(), "c-a2-bracket-case");
    }

    #[test]
    fn g13s_keyring_status_is_the_manifest_crates_lint_token_unchanged() {
        assert_eq!(
            G13Status::Keyring(Lint::KeyringSealMixed).to_string(),
            "keyring-seal-mixed"
        );
        assert_eq!(
            G13Status::Keyring(Lint::KeyringDuplicatePrincipal).to_string(),
            "keyring-duplicate-principal"
        );
    }

    #[test]
    fn g16s_check_12b_token_is_isolation_unsupported() {
        assert_eq!(
            G16Status::Manifest(spine_manifest::Status::IsolationUnsupported).to_string(),
            "isolation-unsupported"
        );
    }

    /// GR §6.3: G8's class is per clause, and only the harness-moved clause is
    /// `tripwire`. "a boundary the branch moved is not a finding its own author
    /// may sign away."
    #[test]
    fn only_the_harness_moved_clause_of_g8_is_a_tripwire() {
        use crate::wire::WireClass;
        assert_eq!(G8Status::HarnessMoved.class(), WireClass::Tripwire);
        for clause in [
            G8Status::BranchEditedBeforeApproval,
            G8Status::LandedId,
            G8Status::FrameworkIsolation,
            G8Status::DiffersFromBoth,
            G8Status::IntentBlobDiffers,
            G8Status::ClosureNotContained,
        ] {
            assert_eq!(clause.class(), WireClass::Protected, "{clause}");
        }
    }
}
