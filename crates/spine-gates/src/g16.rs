//! **G16 — Authority · Scaffold.** `docs/spec/manifest.md` §6.
//!
//! Seventeen ordered checks, plus §6.7's rollback restoration rule. "the order
//! matters in one way only: a manifest that does not parse cannot be checked
//! further, so checks 1–8 are a prefix that halts on first failure. From check
//! 9 onward every check runs and findings accumulate, because a reviewer
//! signing a protected review needs the whole list, not the first item."
//!
//! **Checks 1–8 and 11 are `spine_manifest::Manifest::parse`.** MF §3.11's
//! closed list is that crate's `Status`, and re-deriving it here would be a
//! second spelling of thirty tokens that must agree byte for byte with the
//! manifest the parser accepts. This module supplies the *ordering*, the
//! *kind*, and the nine checks the parse does not make.
//!
//! **G16's wires are `class=protected`, always** (MF §6.1, GR §6.3):
//! "Assigning `tripwire` would let a landing that rewrote `ci.sh` be signed off
//! by its own author in team mode." Break-glass cannot bypass it — PB §7.6's
//! list has no Authority gate on it.

use std::collections::BTreeMap;

use crate::gate::Gate;
use crate::review::Reviews;
use crate::status::G16Status;
use crate::verdict::{Finding, Verdict, decide};
use crate::wire::{Wire, WireClass, WireKind};
use spine_canon::unesc;
use spine_manifest::keyring::Keyring;
use spine_manifest::schema::RESIGN_KEYS;
use spine_manifest::{Manifest, Refusal};

/// Check 1's three outcomes, and checks 2–8's and 11's single one.
#[derive(Debug, Clone, Copy)]
pub enum ManifestAtT<'a> {
    /// Not in `T`. Check 1: outright `manifest-missing`, "unless the landing
    /// carries `Spine-Upgrade: to=none`, where it must be **absent**".
    Absent,
    /// In `T` and refused by MF §3.11's closed list.
    Malformed(&'a Refusal),
    Parsed(&'a Manifest),
}

/// MF §6.4's line, parsed.
///
/// ```text
/// Spine-Upgrade: from=<A> to=<B> manifest=<oid|none> forced=<list>
///                [from-manifest=<sha>] [since=<sha>] signer=<p>
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Upgrade {
    /// "a `cli.version` (§3.2), or `none` for a re-init."
    pub from: String,
    /// "a `cli.version`, or `none` for an uninstall."
    pub to: String,
    /// "the git blob id of `.spine/manifest.json` in `T`, or `none` when
    /// `to=none`."
    pub manifest: String,
    /// The decoded `forced=` set — raw path bytes.
    pub forced: Vec<Vec<u8>>,
    /// "a commit sha; **mandatory on a rollback**, absent otherwise."
    pub from_manifest: Option<String>,
    /// "a commit sha; **mandatory on a re-init** (`from=none`), absent
    /// otherwise."
    pub since: Option<String>,
    pub signer: String,
}

impl Upgrade {
    /// MF §6.8 and §5.9's `to=none`.
    pub fn is_uninstall(&self) -> bool {
        self.to == "none"
    }

    /// MF §6.9's `from=none`.
    pub fn is_reinit(&self) -> bool {
        self.from == "none"
    }

    /// MF §6.7: "PB §7.5 makes it mandatory on a rollback, so its presence *is*
    /// the trigger and no version comparison is needed — which is the property
    /// PB §7.5 relies on when it says 'no gate has to order two version
    /// strings'."
    pub fn is_rollback(&self) -> bool {
        self.from_manifest.is_some()
    }
}

/// Why a `forced=` value is malformed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForcedError {
    /// "A leading, trailing or doubled comma is malformed."
    EmptyMember,
    /// A member that is not a valid `tok` encoding (GR §2.3's escape set).
    NotTokEncoded,
}

/// MF §6.4's `forced=` grammar: `tok(path) [ "," tok(path) ]*`.
///
/// "**`forced=`'s grammar is fixed here and was fixed nowhere.** PB §11 writes
/// `forced=<paths>` — a list value inside a single-space-separated payload with
/// no separator, quoting or escaping — and EV declines to invent one. The line
/// is signed, copied into the landing, and inside `envelope=`, so two
/// implementations guessing differently produce different seals. The resolution
/// reuses machinery rather than adding any: **`tok` from GR §6.2**."
///
/// "The empty list is the **empty value** (`forced= signer=alice@example.com`)
/// and not a sentinel: `none` would be indistinguishable from `tok(\"none\")`,
/// which is a legal path."
pub fn parse_forced(value: &str) -> Result<Vec<Vec<u8>>, ForcedError> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(',')
        .map(|member| {
            if member.is_empty() {
                return Err(ForcedError::EmptyMember);
            }
            unesc(member).map_err(|_| ForcedError::NotTokEncoded)
        })
        .collect()
}

/// Check 9's per-record outcome. The tree read is the caller's; the *finding*
/// is this module's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaffoldState {
    Ok,
    /// "the path exists in `T` and its blob equals `blob`."
    BlobMismatch,
    ScaffoldPathMissing,
    /// "for a managed region, the markers for **this record's own template
    /// name** are well-formed (§3.7)."
    RegionMarkersMissing,
    RegionMarkersMalformed,
    /// "the begin marker's `@<n>` equals `templates[<that template name>]`."
    RegionVersionMismatch,
}

/// One `files[]` record's check-9 outcome, keyed by the record's own `path`
/// (the `path#region` spelling for a managed region, MF §3.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldObservation {
    /// The record's `path`, **decoded** — `FileRecord::path_raw`, not
    /// `FileRecord::path`. The wire built from it is `tok(path)`, and feeding
    /// it the stored `esc` spelling would escape it twice.
    pub path: Vec<u8>,
    pub state: ScaffoldState,
}

/// MF §6.5's lint. "No gate parses or checks the constitution: … GR §5.4.1
/// asserts the check exists as fact … and it did not. It does now."
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConstitutionLint {
    /// "the blob at `M_T.paths.constitution` exists in `T`" — outright.
    pub present: bool,
    /// "it parses under CN §6" — outright.
    pub parses: bool,
    /// "all twelve scaffolded rules are present" — outright.
    pub all_twelve_rules_present: bool,
    /// "each with a value in its declared domain (CN §6.4's table)" —
    /// outright.
    pub every_rule_in_domain: bool,
    /// "its `Version:` differs from the constitution at `B` whenever the blob
    /// differs" — **coverable**. "Two blobs both reading `v3` name two rule
    /// sets permanently, which is what the version exists to prevent."
    pub version_moved_if_blob_moved: bool,
}

/// MF §6.8's four outright checks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Uninstall {
    pub every_spine_owned_path_absent: bool,
    pub every_managed_region_marker_free: bool,
    /// "`diff(tree(B), T)` touches no `user-owned` path of `M_B`."
    pub no_user_owned_touched: bool,
    pub keyring_byte_identical: bool,
    pub constitution_byte_identical: bool,
}

/// MF §6.9's two outright checks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Reinit {
    /// "`since=<sha>` is present and names a first-parent ancestor of `B` that
    /// is a **valid landing** carrying `Spine-Upgrade: to=none`."
    pub since_present: bool,
    pub since_is_a_valid_uninstall_landing: bool,
    /// "`.spine/allowed_signers` in `T` is byte-identical to the keyring at
    /// `since=`."
    pub keyring_matches_since: bool,
}

/// MF §6.7's six steps. **Every step is outright** (PB §6.3: "any landing
/// failing it fails G16, and a recovery-sealed one also indexes
/// `unattested`").
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rollback {
    /// Step 1.
    pub ancestor_reachable: bool,
    pub ancestor_manifest_well_formed: bool,
    /// Step 2: "`<sha> = U^`, where `U` is the **newest first-parent landing at
    /// or below `B` whose envelope carries a copied `Spine-Upgrade`**."
    pub is_one_step: bool,
    /// Step 3: `eq(M_T with paths removed, A with paths removed)` — MF §6.3's
    /// canonical-bytes comparison, "stronger than PB §7.5's 'every frozen field
    /// and every `files[]` record'".
    pub manifest_equals_ancestor_but_paths: bool,
    /// Step 4's input: `A`, the ancestor manifest. The union itself is
    /// **computed** by [`paths_are_the_monotone_union`], not asserted — the
    /// caller supplies the manifest and the gate supplies the rule.
    ///
    /// `None` when `A` did not parse, in which case step 2's finding above
    /// already stands and step 4 is not applicable.
    pub ancestor: Option<Manifest>,
    /// Step 5, per path of MF §6.7.2's `P`: paths that should have been
    /// restored and were not.
    /// Raw path bytes, decoded from the `files(A) \u{222a} files(M_B)` records
    /// they are drawn from.
    pub paths_not_restored: Vec<Vec<u8>>,
    /// Step 5: paths that should have been deleted and were not. Raw bytes.
    pub paths_not_deleted: Vec<Vec<u8>>,
    /// Step 6: "no `user-owned` path of either manifest appears in
    /// `diff(tree(B), T)`."
    pub no_user_owned_touched: bool,
}

/// Everything G16 reads.
#[derive(Debug, Clone)]
pub struct G16Input<'a> {
    pub manifest_t: ManifestAtT<'a>,
    /// `M_B`. `None` under a verifying `Spine-Upgrade: from=none` — "A re-init
    /// lands on a base with no manifest … so `M_B` does not exist and the
    /// checks that compare against it are **not applicable** rather than
    /// failing" (MF §6.2).
    pub manifest_b: Option<&'a Manifest>,
    /// The git blob id of `.spine/manifest.json` in `T`, for check 10's
    /// `manifest=` agreement.
    pub manifest_blob_in_t: Option<&'a str>,
    /// Whether the manifest blob changed between `B` and `T`.
    pub manifest_blob_changed: bool,
    /// The copied `Spine-Upgrade`, **already verified by G13** (MF §6.4: "G13
    /// verifies, G16 reads"). An unverified line must not be passed: MF §6.2,
    /// "an unsigned or absent line buys nothing, so a landing cannot exempt
    /// itself by claiming a re-init."
    pub upgrade: Option<&'a Upgrade>,
    /// MF §6.4's `derived_forced`, computed by the caller from blobs.
    pub derived_forced: Vec<Vec<u8>>,
    pub scaffold: Vec<ScaffoldObservation>,
    /// `K_T` — check 13. G13 reads `K_B`; "a landing may be refused by G13 for
    /// the keyring it is landing *onto* and by G16 for the keyring it is
    /// landing. Both readings are wanted" (MF §4.8.4).
    pub keyring_t: &'a Keyring,
    /// Check 14: "`T` contains no path under `.spine/cache/`." Raw bytes,
    /// straight from `ls-tree -z --name-only`, which is where they come from.
    pub staging_residue: Vec<Vec<u8>>,
    pub constitution: ConstitutionLint,
    pub uninstall: Uninstall,
    pub reinit: Reinit,
    pub rollback: Rollback,
}

/// `path` is **raw bytes**, per `Wire`'s contract. A manifest value arrives
/// `esc`-encoded (MF §2.3) and must be decoded first — `Manifest::path_raw`
/// and `floor_entries_raw` exist for exactly this crossing. Passing the
/// encoded spelling produces `tok(esc(p))`, doubly escaped, for any path
/// carrying a backslash or a byte outside printable ASCII.
fn at(path: impl AsRef<[u8]>) -> Wire {
    Wire::at(
        Gate::G16,
        path.as_ref().to_vec(),
        WireClass::Protected,
        WireKind::Finding,
    )
}

/// GR §6.3's G16 row assigns the gate a token **unconditionally**: "`G16:` +
/// `tok(path)` where a path is implicated, **bare `G16`** where none is."
///
/// Twenty outright findings carried no wire at all until 2026-08-28, on the
/// reading that they "refuse the run before a wire could be read by anyone".
/// GR §5.6.1 says the opposite in terms — "**Outright is a coverage rule, never
/// a containment rule** … [containment] includes every entry of the array,
/// **outright findings among them**. So a landing that carries an outright wire
/// and reaches a review state at all needs that wire **named**" — and it is the
/// report a reviewer reads and binds with `report=`. A short `wires` array is a
/// different array, hence a different `report=` and `envelope=`, over identical
/// objects.
///
/// G14 is the one gate the corpus exempts, and `g14.rs` argues that position
/// explicitly; G16 has no such exemption.
fn bare() -> Wire {
    Wire::pathless(Gate::G16, WireClass::Protected, WireKind::Finding)
}

/// The repository's constitution path, which every `constitution-*` finding
/// implicates.
///
/// DERIVED: `G16Input` does not carry `paths.constitution`, so the wire names
/// the seeded default. A repository that moved its constitution gets a wire
/// naming a path it does not have — worth carrying the real value, and a change
/// to the input type rather than to this gate's rule.
fn constitution_path() -> Wire {
    at("CONSTITUTION.md")
}

/// MF §6.2, executed in order.
pub fn evaluate(input: &G16Input<'_>, reviews: &Reviews) -> Verdict<G16Status> {
    let mut findings: Vec<Finding<G16Status>> = Vec::new();
    let uninstalling = input.upgrade.is_some_and(Upgrade::is_uninstall);
    let reiniting = input.upgrade.is_some_and(Upgrade::is_reinit);

    // ---- 1..8, 11 — the prefix that halts ------------------------------
    let manifest = match input.manifest_t {
        ManifestAtT::Absent => {
            if !uninstalling {
                findings.push(Finding::outright_with_wire(
                    G16Status::Manifest(spine_manifest::Status::ManifestMissing),
                    at(".spine/manifest.json"),
                ));
                return decide(Gate::G16, findings, reviews);
            }
            None
        }
        ManifestAtT::Malformed(refusal) => {
            findings.push(Finding::outright_with_wire(
                G16Status::Manifest(refusal.status),
                at(".spine/manifest.json"),
            ));
            return decide(Gate::G16, findings, reviews);
        }
        ManifestAtT::Parsed(manifest) => {
            // Check 1's `to=none` limb: the manifest "must be **absent**".
            if uninstalling {
                findings.push(Finding::outright_with_wire(
                    G16Status::ManifestNotRemoved,
                    at(".spine/manifest.json"),
                ));
                return decide(Gate::G16, findings, reviews);
            }
            Some(manifest)
        }
    };

    // "*(if the landing is a rollback, §6.7 runs here, before everything
    // below)*" (MF §6.2).
    if input.upgrade.is_some_and(Upgrade::is_rollback) {
        rollback_findings(&input.rollback, manifest, input.manifest_b, &mut findings);
    }

    // ---- 9 — the scaffold blobs (coverable) ----------------------------
    for observation in &input.scaffold {
        let status = match observation.state {
            ScaffoldState::Ok => continue,
            ScaffoldState::BlobMismatch => G16Status::ScaffoldBlobMismatch,
            ScaffoldState::ScaffoldPathMissing => G16Status::ScaffoldPathMissing,
            ScaffoldState::RegionMarkersMissing => G16Status::RegionMarkersMissing,
            ScaffoldState::RegionMarkersMalformed => G16Status::RegionMarkersMalformed,
            ScaffoldState::RegionVersionMismatch => G16Status::RegionVersionMismatch,
        };
        findings.push(Finding::coverable(status, at(&observation.path)));
    }

    // ---- 10 — the manifest changes only under a signed `Spine-Upgrade` --
    match (input.manifest_blob_changed, input.upgrade) {
        (true, None) => findings.push(Finding::outright_with_wire(
            G16Status::ManifestChangedWithoutUpgrade,
            at(".spine/manifest.json"),
        )),
        // "the manifest blob did not change ⇒ the landing carries no
        // `Spine-Upgrade` other than `to=none`."
        (false, Some(upgrade)) if !upgrade.is_uninstall() => {
            findings.push(Finding::outright_with_wire(
                G16Status::UpgradeWithoutManifestChange,
                at(".spine/manifest.json"),
            ));
        }
        _ => {}
    }
    if let Some(upgrade) = input.upgrade {
        // MF §6.4: `manifest` is "the git blob id of `.spine/manifest.json` in
        // `T`, or `none` when `to=none`".
        // A non-uninstall upgrade whose `T` has no `.spine/manifest.json` has
        // nothing for `manifest=` to agree with, and `unwrap_or_default` made
        // that the empty string — which an envelope carrying an empty
        // `manifest=` then matched. Absent is a mismatch, not a blank to be
        // filled: `None` here means the blob the line names is not in the
        // tree.
        let expected: Option<&str> = if upgrade.is_uninstall() {
            Some("none")
        } else {
            input.manifest_blob_in_t
        };
        if Some(upgrade.manifest.as_str()) != expected {
            findings.push(Finding::outright_with_wire(
                G16Status::UpgradeManifestMismatch,
                at(".spine/manifest.json"),
            ));
        }
        if let Some(manifest) = manifest
            && !upgrade.is_uninstall()
            && upgrade.to != manifest.cli_version()
        {
            findings.push(Finding::outright_with_wire(
                G16Status::UpgradeVersionMismatch,
                at(".spine/manifest.json"),
            ));
        }
        // MF §6.4: "`forced=`'s decoded set must equal `derived_forced`
        // exactly. A path in the line and not in the set is a claim of an
        // override that did not happen; a path in the set and not in the line
        // is an override with no signed record."
        let mut declared = upgrade.forced.clone();
        let mut derived = input.derived_forced.clone();
        declared.sort_unstable();
        declared.dedup();
        derived.sort_unstable();
        derived.dedup();
        if declared != derived {
            findings.push(Finding::outright_with_wire(
                G16Status::ForcedDisagrees,
                bare(),
            ));
        }
    }

    if let Some(manifest) = manifest {
        // ---- 11b — `resign` monotone; skipped under `from=none` --------
        if let Some(m_b) = input.manifest_b
            && !reiniting
        {
            for variant in RESIGN_KEYS {
                if let (Some(t), Some(b)) = (
                    manifest.resign_version(variant),
                    m_b.resign_version(variant),
                ) && t < b
                {
                    findings.push(Finding::coverable(G16Status::ResignLowered, bare()));
                }
            }
            // ---- 12 — `params.langs` monotone -------------------------
            // PB §6.3: removing a language "retires part of the G1 floor, so it
            // takes the same protected review as any other floor change rather
            // than passing as an ordinary edit". Coverable, "exactly as
            // written" (MF §6.2).
            let langs_t = manifest.langs();
            if !m_b.langs().iter().all(|l| langs_t.contains(l)) {
                findings.push(Finding::coverable(G16Status::LangsShrank, bare()));
            }
        }

        // ---- 12b — `params.isolation` is not `uid` ---------------------
        // "**Outright and not coverable**, because no protected reviewer can
        // make a mechanism exist: a dischargeable wire would let two humans
        // sign a repository into the brick."
        if manifest.isolation() == spine_manifest::Isolation::Uid {
            findings.push(Finding::outright_with_wire(
                G16Status::Manifest(spine_manifest::Status::IsolationUnsupported),
                at(".spine/manifest.json"),
            ));
        }
    }

    // ---- 13 — `K_T` lints ---------------------------------------------
    for finding in &input.keyring_t.findings {
        findings.push(Finding::coverable(
            G16Status::Keyring(finding.lint),
            at(".spine/allowed_signers"),
        ));
    }

    // ---- 14 — no staging residue --------------------------------------
    for path in &input.staging_residue {
        findings.push(Finding::coverable(G16Status::StagingResidue, at(path)));
    }

    // ---- 15 — the constitution lint ------------------------------------
    let lint = &input.constitution;
    if !lint.present {
        findings.push(Finding::outright_with_wire(
            G16Status::ConstitutionMissing,
            constitution_path(),
        ));
    } else if !lint.parses {
        findings.push(Finding::outright_with_wire(
            G16Status::ConstitutionUnparseable,
            constitution_path(),
        ));
    } else {
        if !lint.all_twelve_rules_present {
            findings.push(Finding::outright_with_wire(
                G16Status::ConstitutionRuleMissing,
                constitution_path(),
            ));
        }
        if !lint.every_rule_in_domain {
            findings.push(Finding::outright_with_wire(
                G16Status::ConstitutionRuleOutOfDomain,
                constitution_path(),
            ));
        }
        if !lint.version_moved_if_blob_moved {
            findings.push(Finding::coverable(
                G16Status::ConstitutionVersionRegressed,
                bare(),
            ));
        }
    }

    // ---- 16 — `to=none` -------------------------------------------------
    if uninstalling {
        let u = &input.uninstall;
        if !u.every_spine_owned_path_absent {
            findings.push(Finding::outright_with_wire(
                G16Status::UninstallPathRemains,
                bare(),
            ));
        }
        if !u.every_managed_region_marker_free {
            findings.push(Finding::outright_with_wire(
                G16Status::UninstallRegionRemains,
                bare(),
            ));
        }
        if !u.no_user_owned_touched {
            findings.push(Finding::outright_with_wire(
                G16Status::UninstallUserOwnedTouched,
                bare(),
            ));
        }
        // "The keyring clause is not redundant with the `user-owned` clause: it
        // is what makes a later re-init's `since=` check meaningful."
        if !u.keyring_byte_identical {
            findings.push(Finding::outright_with_wire(
                G16Status::UninstallKeyringChanged,
                at(".spine/allowed_signers"),
            ));
        }
        if !u.constitution_byte_identical {
            findings.push(Finding::outright_with_wire(
                G16Status::UninstallConstitutionChanged,
                constitution_path(),
            ));
        }
    }

    // ---- 17 — `from=none` -----------------------------------------------
    if reiniting {
        let r = &input.reinit;
        if !r.since_present {
            findings.push(Finding::outright_with_wire(
                G16Status::ReinitSinceMissing,
                bare(),
            ));
        } else if !r.since_is_a_valid_uninstall_landing {
            findings.push(Finding::outright_with_wire(
                G16Status::ReinitSinceNotUninstall,
                bare(),
            ));
        }
        if !r.keyring_matches_since {
            findings.push(Finding::outright_with_wire(
                G16Status::ReinitKeyringDiffers,
                at(".spine/allowed_signers"),
            ));
        }
    }

    decide(Gate::G16, findings, reviews)
}

fn rollback_findings(
    rollback: &Rollback,
    manifest_t: Option<&Manifest>,
    manifest_b: Option<&Manifest>,
    findings: &mut Vec<Finding<G16Status>>,
) {
    if !rollback.ancestor_reachable {
        findings.push(Finding::outright_with_wire(
            G16Status::RestoreAncestorUnreachable,
            bare(),
        ));
    }
    if !rollback.ancestor_manifest_well_formed {
        findings.push(Finding::outright_with_wire(
            G16Status::RestoreAncestorManifestMalformed,
            bare(),
        ));
    }
    // "*Recovery undoes one lifecycle landing per landing and no more*
    // (PB §7.5). A deeper rollback is a chain of single steps."
    if !rollback.is_one_step {
        findings.push(Finding::outright_with_wire(
            G16Status::RestoreNotOneStep,
            bare(),
        ));
    }
    if !rollback.manifest_equals_ancestor_but_paths {
        findings.push(Finding::outright_with_wire(
            G16Status::RestoreManifestDiffers,
            at(".spine/manifest.json"),
        ));
    }
    if let (Some(ancestor), Some(m_t), Some(m_b)) = (&rollback.ancestor, manifest_t, manifest_b)
        && !paths_are_the_monotone_union(m_t, ancestor, m_b)
    {
        findings.push(Finding::outright_with_wire(
            G16Status::RestorePathsNotUnion,
            at(".spine/manifest.json"),
        ));
    }
    // "**On step 5, and why it is not read from the diff.** … A diff-driven
    // check sees only what changed; a manifest-driven check sees what should be
    // true."
    for path in &rollback.paths_not_restored {
        findings.push(Finding::outright_with_wire(
            G16Status::RestorePathNotRestored,
            at(path),
        ));
    }
    for path in &rollback.paths_not_deleted {
        findings.push(Finding::outright_with_wire(
            G16Status::RestorePathNotDeleted,
            at(path),
        ));
    }
    if !rollback.no_user_owned_touched {
        findings.push(Finding::outright_with_wire(
            G16Status::RestoreUserOwnedTouched,
            bare(),
        ));
    }
}

/// MF §6.7.1's monotone union, computed over one key's value sets.
///
/// ```text
/// keys(M_T.paths) = keys(A.paths) ∪ keys(M_B.paths)
/// for every k :  values(M_T.paths[k]) = values(A.paths[k]) ∪ values(M_B.paths[k])
/// ```
///
/// "with an absent key contributing the empty set, and each result written in
/// §3.4's canonical shape — a string for a singleton, a sorted array for two or
/// more. *The floor never shrinks, not even on rollback, and `B` is what the
/// floor has become since*."
/// **Decided, not asserted.** Step 4 was a caller-supplied bool with this
/// function sitting unused beside it — the shape the corpus objects to
/// wherever it appears, since the one place the rule is written down is then
/// the one place nothing executes.
///
/// The canonical shape is not re-checked here: `Manifest::parse` refuses a
/// one-element array, an unsorted array and a duplicate (MF §3.4), so a
/// `Manifest` that exists has it. That is why this takes three manifests and
/// not three JSON values.
pub fn paths_are_the_monotone_union(m_t: &Manifest, ancestor: &Manifest, m_b: &Manifest) -> bool {
    let t = paths_map(m_t);
    let a = paths_map(ancestor);
    let b = paths_map(m_b);

    let mut expected_keys: Vec<&str> = a.keys().chain(b.keys()).copied().collect();
    expected_keys.sort_unstable();
    expected_keys.dedup();

    let actual_keys: Vec<&str> = {
        let mut k: Vec<&str> = t.keys().copied().collect();
        k.sort_unstable();
        k
    };
    if actual_keys != expected_keys {
        return false;
    }

    expected_keys.iter().all(|key| {
        // "with an absent key contributing the empty set".
        let union = monotone_union(
            a.get(key).map(Vec::as_slice).unwrap_or_default(),
            b.get(key).map(Vec::as_slice).unwrap_or_default(),
        );
        t.get(key).map(|v| {
            let mut v = v.clone();
            v.sort_unstable();
            v.dedup();
            v
        }) == Some(union)
    })
}

fn paths_map(m: &Manifest) -> BTreeMap<&str, Vec<&str>> {
    m.paths_by_key().into_iter().collect()
}

/// One key's value union, the inner line of the rule above.
pub fn monotone_union<'a>(ancestor: &[&'a str], base: &[&'a str]) -> Vec<&'a str> {
    let mut out: Vec<&str> = ancestor.iter().chain(base.iter()).copied().collect();
    out.sort_unstable();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::{Binding, Review, ReviewClass};
    use crate::verdict::GateStatus;
    use spine_canon::ObjectFormat;

    /// MF §8.3's manifest, reduced to what G16's checks read. Its exact bytes
    /// and the published blob id `cb4cd490…` are `spine-manifest`'s vector, and
    /// this crate builds a minimal conforming manifest rather than duplicating
    /// them.
    fn manifest(langs: &[&str], resign: u64, isolation: &str) -> Manifest {
        let langs_json = langs
            .iter()
            .map(|l| format!("\"{l}\""))
            .collect::<Vec<_>>()
            .join(",");
        let json = format!(
            concat!(
                r#"{{"cli":{{"dist_hash":"sha256:{}","version":"1.4.0"}},"#,
                r#""envelope":1,"files":[],"manifest_version":1,"object_format":"sha1","#,
                r#""params":{{"ci":"github","isolation":"{}","langs":[{}],"timeout":1800,"trunk":"main"}},"#,
                r#""paths":{{"constitution":"CONSTITUTION.md"}},"#,
                r#""repo":"myrepo","#,
                r#""resign":{{"intent":{},"intent-bug":{},"intent-change":{}}},"#,
                r#""schema":7,"#,
                r#""templates":{{"intent":2,"intent-bug":2,"intent-change":2}}}}"#,
                "\n"
            ),
            // Not MF §8.3's `dist_hash`. That value is published elided
            // (`sha256:6f49644f…744db`), and filling an ellipsis with zeros
            // would put a digest in the tree that is in the value space of no
            // artifact list. This is the SHA-256 of the 26 ASCII bytes
            // `spine-gates-test-artifacts`, computed, and nothing here compares
            // it to anything: only its grammar is under test.
            "980d4cb66bc03353cdb93d9149ead2ec7aae73c8e1ab6ade536eb8628acd0753",
            isolation,
            langs_json,
            resign,
            resign,
            resign,
        );
        Manifest::parse(json.as_bytes(), Some(ObjectFormat::Sha1)).expect("a conforming manifest")
    }

    /// The same shape as [`manifest`] with `paths` supplied verbatim, for the
    /// step-4 union, which reads nothing else.
    fn manifest_with_paths(paths: &str) -> Manifest {
        let json = format!(
            concat!(
                r#"{{"cli":{{"dist_hash":"sha256:{}","version":"1.4.0"}},"#,
                r#""envelope":1,"files":[],"manifest_version":1,"object_format":"sha1","#,
                r#""params":{{"ci":"github","isolation":"container","langs":["python"],"#,
                r#""timeout":1800,"trunk":"main"}},"#,
                r#""paths":{{{}}},"#,
                r#""repo":"myrepo","#,
                r#""resign":{{"intent":2,"intent-bug":2,"intent-change":2}},"#,
                r#""schema":7,"#,
                r#""templates":{{"intent":2,"intent-bug":2,"intent-change":2}}}}"#,
                "\n"
            ),
            "980d4cb66bc03353cdb93d9149ead2ec7aae73c8e1ab6ade536eb8628acd0753", paths,
        );
        Manifest::parse(json.as_bytes(), Some(ObjectFormat::Sha1)).expect("a conforming manifest")
    }

    fn keyring_clean() -> Keyring {
        Keyring::parse(concat!(
            "alice@example.com namespaces=\"spine-signoff@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n",
            // MF §4.5: mode is the count of distinct `spine-signoff@v1`
            // fingerprints. Two of them is what makes this keyring `team`, and
            // checks 7 and 9 both read that and nothing else.
            "carol@example.com namespaces=\"spine-signoff@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD\n",
            "bob@example.com namespaces=\"spine-review@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB\n",
            "spine-pipeline namespaces=\"spine-seal@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC\n",
        ).as_bytes())
    }

    fn clean_constitution() -> ConstitutionLint {
        ConstitutionLint {
            present: true,
            parses: true,
            all_twelve_rules_present: true,
            every_rule_in_domain: true,
            version_moved_if_blob_moved: true,
        }
    }

    fn input<'a>(
        m_t: &'a Manifest,
        m_b: Option<&'a Manifest>,
        keyring: &'a Keyring,
    ) -> G16Input<'a> {
        G16Input {
            manifest_t: ManifestAtT::Parsed(m_t),
            manifest_b: m_b,
            manifest_blob_in_t: None,
            manifest_blob_changed: false,
            upgrade: None,
            derived_forced: Vec::new(),
            scaffold: Vec::new(),
            keyring_t: keyring,
            staging_residue: Vec::new(),
            constitution: clean_constitution(),
            uninstall: Uninstall::default(),
            reinit: Reinit::default(),
            rollback: Rollback::default(),
        }
    }

    /// MF §8.5: `G16 = pass`, no wires, over a landing that touches no
    /// `.spine/` path.
    #[test]
    fn a_clean_landing_passes_with_no_wires() {
        let m = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let verdict = evaluate(&input(&m, Some(&m), &k), &Reviews::default());
        assert_eq!(verdict.status, GateStatus::Pass);
        assert!(verdict.wires.is_empty());
    }

    /// MF §6.2: "checks 1–8 are a prefix that halts on first failure."
    #[test]
    fn a_manifest_that_does_not_parse_halts_before_check_9() {
        let m = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let refusal = Refusal::new(spine_manifest::Status::ManifestNoncanonical, "root");
        let mut i = input(&m, Some(&m), &k);
        i.manifest_t = ManifestAtT::Malformed(&refusal);
        i.staging_residue = vec![".spine/cache/x".into()];
        let verdict = evaluate(&i, &Reviews::default());
        assert_eq!(verdict.findings.len(), 1);
        assert_eq!(verdict.statuses()[0].to_string(), "manifest-noncanonical");
    }

    /// MF §6.2 check 1: the manifest is absent unless `to=none`, where it must
    /// be.
    #[test]
    fn an_absent_manifest_is_manifest_missing_unless_the_landing_uninstalls() {
        let m = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let mut i = input(&m, Some(&m), &k);
        i.manifest_t = ManifestAtT::Absent;
        assert_eq!(
            evaluate(&i, &Reviews::default()).statuses()[0].to_string(),
            "manifest-missing"
        );

        let uninstall = Upgrade {
            from: "1.4.0".into(),
            to: "none".into(),
            manifest: "none".into(),
            ..Default::default()
        };
        i.upgrade = Some(&uninstall);
        i.manifest_blob_changed = true;
        i.uninstall = Uninstall {
            every_spine_owned_path_absent: true,
            every_managed_region_marker_free: true,
            no_user_owned_touched: true,
            keyring_byte_identical: true,
            constitution_byte_identical: true,
        };
        let verdict = evaluate(&i, &Reviews::default());
        assert_eq!(verdict.status, GateStatus::Pass);
    }

    #[test]
    fn a_manifest_still_present_under_to_none_is_manifest_not_removed() {
        let m = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let uninstall = Upgrade {
            to: "none".into(),
            manifest: "none".into(),
            ..Default::default()
        };
        let mut i = input(&m, Some(&m), &k);
        i.upgrade = Some(&uninstall);
        assert_eq!(
            evaluate(&i, &Reviews::default()).statuses()[0].to_string(),
            "manifest-not-removed"
        );
    }

    /// MF §6.2 check 9: coverable, `G16:<tok(path)>`.
    #[test]
    fn a_scaffold_blob_mismatch_is_coverable_by_a_protected_review_naming_the_path() {
        let m = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let mut i = input(&m, Some(&m), &k);
        i.scaffold = vec![ScaffoldObservation {
            path: ".spine/ci.sh".into(),
            state: ScaffoldState::BlobMismatch,
        }];
        let uncovered = evaluate(&i, &Reviews::default());
        assert_eq!(uncovered.status, GateStatus::Fail);
        assert_eq!(uncovered.wires.tokens(), ["G16:.spine/ci.sh"]);

        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:b", Binding::Current)
                .naming(vec!["G16:.spine/ci.sh"]),
        ]);
        assert_eq!(evaluate(&i, &reviews).status, GateStatus::Override);
    }

    /// MF §6.2 check 10, both limbs.
    #[test]
    fn the_manifest_blob_changes_only_under_a_signed_upgrade() {
        let m = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let mut i = input(&m, Some(&m), &k);
        i.manifest_blob_changed = true;
        assert!(
            evaluate(&i, &Reviews::default())
                .statuses()
                .iter()
                .any(|s| s.to_string() == "manifest-changed-without-upgrade")
        );

        let upgrade = Upgrade {
            from: "1.3.0".into(),
            to: "1.4.0".into(),
            manifest: "74806e98701b50e958074dbaad0d7509d84751a3".into(),
            ..Default::default()
        };
        let mut j = input(&m, Some(&m), &k);
        j.manifest_blob_changed = false;
        j.upgrade = Some(&upgrade);
        assert!(
            evaluate(&j, &Reviews::default())
                .statuses()
                .iter()
                .any(|s| s.to_string() == "upgrade-without-manifest-change")
        );
    }

    /// MF §6.4: "**`forced=`'s grammar is fixed here and was fixed nowhere.**"
    /// The empty list is the empty value; a leading, trailing or doubled comma
    /// is malformed; members are `tok`-encoded.
    #[test]
    fn the_forced_list_is_tok_encoded_and_its_empty_value_is_the_empty_list() {
        assert_eq!(parse_forced("").unwrap(), Vec::<Vec<u8>>::new());
        assert_eq!(
            parse_forced(".spine/ci.sh").unwrap(),
            vec![b".spine/ci.sh".to_vec()]
        );
        // `tok` moves `,`, ` ` and `"` into the `\xHH` row, so a path with a
        // comma survives the comma-separated list.
        assert_eq!(
            parse_forced("docs/a\\x2cb.md,docs/c\\x20d.md").unwrap(),
            vec![b"docs/a,b.md".to_vec(), b"docs/c d.md".to_vec()]
        );
        assert_eq!(parse_forced(",a").unwrap_err(), ForcedError::EmptyMember);
        assert_eq!(parse_forced("a,").unwrap_err(), ForcedError::EmptyMember);
        assert_eq!(parse_forced("a,,b").unwrap_err(), ForcedError::EmptyMember);
    }

    /// MF §6.4: "A path in the line and not in the set is a claim of an
    /// override that did not happen; a path in the set and not in the line is
    /// an override with no signed record."
    #[test]
    fn a_forced_list_disagreeing_with_the_derived_set_is_outright() {
        let m = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let upgrade = Upgrade {
            from: "1.3.0".into(),
            to: "1.4.0".into(),
            manifest: "abc".into(),
            forced: vec![b".spine/ci.sh".to_vec()],
            ..Default::default()
        };
        let mut i = input(&m, Some(&m), &k);
        i.manifest_blob_changed = true;
        i.manifest_blob_in_t = Some("abc");
        i.upgrade = Some(&upgrade);
        i.derived_forced = vec![];
        assert!(
            evaluate(&i, &Reviews::default())
                .statuses()
                .iter()
                .any(|s| s.to_string() == "forced-disagrees")
        );
    }

    /// MF §6.2 check 12: "Coverable, not outright, exactly as written."
    #[test]
    fn dropping_a_language_is_coverable_and_never_an_ordinary_edit() {
        let m_b = manifest(&["python", "ts"], 2, "container");
        let m_t = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let i = input(&m_t, Some(&m_b), &k);
        let uncovered = evaluate(&i, &Reviews::default());
        assert_eq!(uncovered.status, GateStatus::Fail);
        assert_eq!(uncovered.wires.tokens(), ["G16"]);
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:b", Binding::Current).naming(vec!["G16"]),
        ]);
        assert_eq!(evaluate(&i, &reviews).status, GateStatus::Override);
    }

    /// MF §6.2 check 12b: "**Outright and not coverable**, because no protected
    /// reviewer can make a mechanism exist: a dischargeable wire would let two
    /// humans sign a repository into the brick."
    #[test]
    fn a_manifest_requesting_uid_isolation_is_outright() {
        let m = manifest(&["python"], 2, "uid");
        let k = keyring_clean();
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a", Binding::Current)
                .naming(vec!["G16:.spine/manifest.json"]),
            Review::new(ReviewClass::Protected, "SHA256:b", Binding::Current)
                .naming(vec!["G16:.spine/manifest.json"]),
        ]);
        let verdict = evaluate(&input(&m, Some(&m), &k), &reviews);
        assert_eq!(verdict.status, GateStatus::Fail);
        assert!(
            verdict
                .statuses()
                .iter()
                .any(|s| s.to_string() == "isolation-unsupported")
        );
    }

    /// MF §6.2 check 11b and 12: "**skipped under `from=none`**" — "there is no
    /// `resign` at `B` to be lower than".
    #[test]
    fn a_re_init_skips_every_comparison_against_m_b() {
        // `resign` lowered (2 -> 1) and `params.langs` shrunk: check 11b and
        // check 12 would both fire on any other landing.
        let m_b = manifest(&["python", "ts"], 2, "container");
        let m_t = manifest(&["python"], 1, "container");
        let k = keyring_clean();
        let reinit = Upgrade {
            from: "none".into(),
            to: "1.4.0".into(),
            manifest: "abc".into(),
            since: Some("1cbc18507888cb238c56ce00ba678c16564e0274".into()),
            ..Default::default()
        };
        let mut i = input(&m_t, Some(&m_b), &k);
        i.manifest_blob_changed = true;
        i.manifest_blob_in_t = Some("abc");
        i.upgrade = Some(&reinit);
        i.reinit = Reinit {
            since_present: true,
            since_is_a_valid_uninstall_landing: true,
            keyring_matches_since: true,
        };
        let verdict = evaluate(&i, &Reviews::default());
        assert_eq!(verdict.status, GateStatus::Pass);
    }

    /// MF §6.9: "`since=` must name a landing carrying `to=none`, or the
    /// re-init is refused and nothing is exempt."
    #[test]
    fn a_re_init_whose_since_is_not_an_uninstall_is_refused() {
        let m = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let reinit = Upgrade {
            from: "none".into(),
            to: "1.4.0".into(),
            manifest: "abc".into(),
            since: Some("1cbc18507888cb238c56ce00ba678c16564e0274".into()),
            ..Default::default()
        };
        let mut i = input(&m, None, &k);
        i.manifest_blob_changed = true;
        i.manifest_blob_in_t = Some("abc");
        i.upgrade = Some(&reinit);
        i.reinit = Reinit {
            since_present: true,
            since_is_a_valid_uninstall_landing: false,
            keyring_matches_since: true,
        };
        assert!(
            evaluate(&i, &Reviews::default())
                .statuses()
                .iter()
                .any(|s| s.to_string() == "reinit-since-not-uninstall")
        );
    }

    /// MF §6.7: "**Every step is outright.**"
    #[test]
    fn every_step_of_the_rollback_restoration_rule_is_outright() {
        let m = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let rollback_line = Upgrade {
            from: "1.4.0".into(),
            to: "1.3.0".into(),
            manifest: "abc".into(),
            from_manifest: Some("1cbc18507888cb238c56ce00ba678c16564e0274".into()),
            ..Default::default()
        };
        let mut i = input(&m, Some(&m), &k);
        i.manifest_blob_changed = true;
        i.manifest_blob_in_t = Some("abc");
        i.upgrade = Some(&rollback_line);
        i.rollback = Rollback {
            ancestor_reachable: true,
            ancestor_manifest_well_formed: true,
            is_one_step: false,
            manifest_equals_ancestor_but_paths: true,
            // `A` is `M_T` here, whose `paths` is trivially its own union with
            // itself — step 4 holds and this test is about steps 2 and 5.
            ancestor: Some(m.clone()),
            paths_not_restored: vec![".spine/ci.sh".into()],
            paths_not_deleted: vec![],
            no_user_owned_touched: false,
        };
        // `to=1.3.0` disagrees with the manifest's own `cli.version`, which is
        // check 10's `upgrade-version-mismatch` — deliberately left in, because
        // it is what a rollback with the wrong `to=` looks like.
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a", Binding::Current)
                .naming(vec!["G16", "G16:.spine/ci.sh"]),
            Review::new(ReviewClass::Protected, "SHA256:b", Binding::Current)
                .naming(vec!["G16", "G16:.spine/ci.sh"]),
        ]);
        let verdict = evaluate(&i, &reviews);
        assert_eq!(verdict.status, GateStatus::Fail);
        let tokens: Vec<String> = verdict.statuses().iter().map(|s| s.to_string()).collect();
        assert!(tokens.contains(&"restore-not-one-step".to_string()));
        assert!(tokens.contains(&"restore-path-not-restored".to_string()));
        assert!(tokens.contains(&"restore-user-owned-touched".to_string()));
    }

    /// MF §8.6, computed: "`agent_context` gains `CLAUDE.md` from `B` — *the
    /// floor never shrinks, not even backwards* — and the two-element result is
    /// written as a sorted array while `constitution` stays a string."
    #[test]
    fn the_monotone_union_reproduces_the_mf_8_6_rollback() {
        assert_eq!(
            monotone_union(&["AGENTS.md"], &["AGENTS.md", "CLAUDE.md"]),
            ["AGENTS.md", "CLAUDE.md"]
        );
        assert_eq!(
            monotone_union(&["CONSTITUTION.md"], &["CONSTITUTION.md"]),
            ["CONSTITUTION.md"]
        );
        // "a path `A` created and the upgrade deleted is restored" — the union
        // keeps both sides' entries whichever manifest holds them.
        assert_eq!(monotone_union(&["a"], &["b"]), ["a", "b"]);
    }

    /// Step 4 as a whole, over three manifests: the **key** union as well as
    /// the per-key value union. A `M_T` that drops a key `A` had, or drops one
    /// value of a key both had, is not the union — "*the floor never shrinks,
    /// not even on rollback*".
    #[test]
    fn step_4_decides_the_key_union_and_not_only_the_values() {
        let a = manifest_with_paths(r#""constitution":"CONSTITUTION.md","scaffold":"AGENTS.md""#);
        let b = manifest_with_paths(r#""constitution":"CONSTITUTION.md""#);

        // `keys(A) ∪ keys(M_B)` = both keys, and `M_T` carries both.
        let good =
            manifest_with_paths(r#""constitution":"CONSTITUTION.md","scaffold":"AGENTS.md""#);
        assert!(paths_are_the_monotone_union(&good, &a, &b));

        // `M_T` drops the key the ancestor had. The old per-key-only reading
        // never looked at a key `M_T` does not have.
        assert!(!paths_are_the_monotone_union(&b, &a, &b));

        // `M_T` drops one value of a key both manifests have.
        let wide = manifest_with_paths(r#""scaffold":["AGENTS.md","CLAUDE.md"]"#);
        let narrow = manifest_with_paths(r#""scaffold":"AGENTS.md""#);
        assert!(!paths_are_the_monotone_union(&narrow, &wide, &narrow));
        assert!(paths_are_the_monotone_union(&wide, &wide, &narrow));

        // And a key `M_T` invents, which neither `A` nor `M_B` had, is not the
        // union either — the rule is an equality, not a containment.
        let invented =
            manifest_with_paths(r#""constitution":"CONSTITUTION.md","extra":"docs/x.md""#);
        assert!(!paths_are_the_monotone_union(&invented, &b, &b));
    }

    /// MF §6.5: the version check is "what makes `Constitution: v<n>` mean
    /// something", and it is the one coverable constitution finding.
    #[test]
    fn a_constitution_edit_that_leaves_the_version_alone_is_coverable() {
        let m = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let mut i = input(&m, Some(&m), &k);
        i.constitution.version_moved_if_blob_moved = false;
        assert_eq!(
            evaluate(&i, &Reviews::default()).statuses()[0].to_string(),
            "constitution-version-regressed"
        );
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:b", Binding::Current).naming(vec!["G16"]),
        ]);
        assert_eq!(evaluate(&i, &reviews).status, GateStatus::Override);
    }

    /// MF §6.2 check 14, and PB §6.7's staging residue.
    #[test]
    fn a_staging_residue_path_is_coverable_and_names_itself() {
        let m = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let mut i = input(&m, Some(&m), &k);
        i.staging_residue = vec![".spine/cache/results/abc.jsonl".into()];
        let verdict = evaluate(&i, &Reviews::default());
        assert_eq!(
            verdict.wires.tokens(),
            ["G16:.spine/cache/results/abc.jsonl"]
        );
        assert_eq!(verdict.statuses()[0].to_string(), "staging-residue");
    }

    /// MF §4.8.4's note: "a landing may be refused by G13 for the keyring it is
    /// landing *onto* and by G16 for the keyring it is landing."
    #[test]
    fn g16_lints_the_keyring_in_t_and_the_finding_is_coverable() {
        let m = manifest(&["python"], 2, "container");
        let broken = Keyring::missing();
        let i = input(&m, Some(&m), &broken);
        let verdict = evaluate(&i, &Reviews::default());
        assert_eq!(verdict.statuses()[0].to_string(), "keyring-missing");
        assert_eq!(verdict.wires.tokens(), ["G16:.spine/allowed_signers"]);
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:b", Binding::Current)
                .naming(vec!["G16:.spine/allowed_signers"]),
        ]);
        assert_eq!(evaluate(&i, &reviews).status, GateStatus::Override);
    }

    /// MF §6.1 and GR §6.3: "**G16's wires are `class=protected`, always.**"
    #[test]
    fn every_g16_wire_is_protected() {
        let m = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let mut i = input(&m, Some(&m), &k);
        i.staging_residue = vec![".spine/cache/x".into()];
        i.scaffold = vec![ScaffoldObservation {
            path: "AGENTS.md#spine".into(),
            state: ScaffoldState::RegionVersionMismatch,
        }];
        let verdict = evaluate(&i, &Reviews::default());
        assert!(
            verdict
                .wires
                .ordered()
                .iter()
                .all(|w| w.class == WireClass::Protected && w.kind == WireKind::Finding)
        );
    }

    /// GR §6.3 gives G16 a token unconditionally, so an outright finding with
    /// no wire is a report whose `gates[G16]` fails with nothing in
    /// `wires` — a `fail` no review could ever name, which is not what G16's
    /// row says. Nineteen of the twenty statuses were once emitted that way.
    ///
    /// Live evaluation reaches only a handful of the statuses at a time
    /// (several are mutually exclusive: `re-init` skips every M_B comparison,
    /// `uninstall` skips the ordinary edit rules), so the guard is over the
    /// module's own source. G14 is the one gate the corpus exempts from a
    /// wire, and it argues that position in its own file; G16 has no such
    /// exemption, so the count here is zero and not a list.
    #[test]
    fn no_g16_finding_is_raised_without_a_wire() {
        let source = include_str!("g16.rs");
        let wireless: Vec<&str> = source
            .lines()
            // `coverable` takes its wire as a required argument; only
            // `outright` has a wireless constructor to fall into. The needle is
            // assembled rather than written, so that this line does not match
            // itself — as it did on the first run.
            .filter(|l| l.contains(&format!("Finding::{}(", "outright")))
            .collect();
        assert!(
            wireless.is_empty(),
            "every G16 finding must carry a wire; these do not: {wireless:#?}"
        );
    }

    /// Check 10's `manifest=` agreement, where absent is not blank: a
    /// non-uninstall upgrade whose tree has no `.spine/manifest.json` agrees
    /// with nothing, including an envelope that names nothing.
    #[test]
    fn an_upgrade_naming_no_manifest_blob_does_not_agree_with_a_missing_one() {
        let m = manifest(&["python"], 2, "container");
        let k = keyring_clean();
        let mut i = input(&m, Some(&m), &k);
        i.manifest_blob_changed = true;
        i.manifest_blob_in_t = None;
        let upgrade = Upgrade {
            from: "1.3.0".into(),
            to: "1.4.0".into(),
            manifest: String::new(),
            ..Default::default()
        };
        i.upgrade = Some(&upgrade);
        let verdict = evaluate(&i, &Reviews::default());
        assert!(
            verdict
                .statuses()
                .iter()
                .any(|s| s.to_string() == G16Status::UpgradeManifestMismatch.to_string()),
            "statuses: {:?}",
            verdict.statuses()
        );
    }
}
