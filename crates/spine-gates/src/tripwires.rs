//! G2, G3, G4, G5, G7 and G12 — the gates whose **wire** the corpus fixes and
//! whose **predicate** belongs to another document.
//!
//! Each function here takes the predicate's answer and produces the entry
//! GR §6.3 fixes: the token, the `class` and the `kind`. That division is the
//! point. GR §6.3: "`class` is required, two-valued and **digest-bearing**, and
//! it decides the landing's review state through PB §11's aggregation — which
//! decides who must sign. A gate whose class is unassigned is a gate two
//! implementations route differently, producing a different `wires` array, a
//! different `report=` and a different `envelope=` over identical facts."
//!
//! The predicates themselves live where their inputs do: `spine_match` and the
//! touchpoint dialect in ID §6.3 (`spine-resolve`), the package-manifest paths
//! and the pragma join in IR §12, the lease registry in PB §5.4.

use crate::gate::Gate;
use crate::wire::{Wire, WireClass, WireKind};

/// GR §9.8: "the threshold is exactly **1 209 600 seconds** (14 days), a
/// constant of the pinned release."
///
/// GR §10 OPEN-3 settled that it never becomes a constitution rule: "A team
/// that wants a different window still has no lever, and that is the answer
/// rather than the cost of it." An implementer must not add a `C-F1`.
pub const STALENESS_WINDOW_SECS: i64 = 1_209_600;

/// PB §6.3 and PB §9: whether Drift is still earning trust.
///
/// PB §11 and GR §6.1 bound the reach: "Only G2, G3 and G7's *soft* clause can
/// produce it; a `forbidden` hit and a hard lease over another intent's
/// forbidden or frozen set are `finding` in **every** mode."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Calibration {
    /// PB §6.3: "In warn mode a Drift finding still enters the report's wire set
    /// and `wires=` — it merely does not block on its own."
    Warn,
    Block,
}

impl Calibration {
    fn kind(self) -> WireKind {
        match self {
            Calibration::Warn => WireKind::Warn,
            Calibration::Block => WireKind::Finding,
        }
    }
}

// ---------------------------------------------------------------------------
// G2 — Drift · Containment
// ---------------------------------------------------------------------------

/// Which G2 sub-check raised a wire. The token differs by sub-check and the
/// difference is not free (contradiction C2, below).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum G2SubCheck {
    /// "`modifies` of the synthetic merge ⊆ declared `expected` touchpoints."
    OutsideExpected,
    /// "any `forbidden` hit is a hard fail." PB §11: it is a `finding` in every
    /// mode.
    ForbiddenHit,
    /// "**New dependency** is a change to a package manifest, whose
    /// per-language paths `docs/spec/import-resolver.md` lists."
    PackageManifestChanged,
    /// The repository-wide count. **Bare `G2`.**
    DiffSize,
}

/// The wire for one G2 sub-check.
///
/// **Contradiction C2, resolved in GR's favour via PB §11.** PB §6.3's G2 row
/// says "the diff-size and new-dependency wires of §5.2 are G2 sub-checks,
/// recorded as `G2:<path>`". GR §6.3 says the diff-size sub-check is "**a
/// repository-wide count that names no path**, so it takes the bare id under
/// PB §11's 'gates without a path use the bare id'", and `docs/spec/README.md`
/// line 9 gives §11 precedence over prose. GR adds the retention argument: "a
/// base move changes `merge-base` and therefore the count, and a pathless wire
/// never survives a base move." An implementation writing `G2:<path>` for the
/// count "produces a different `wires` array, `report=` and `envelope=`".
pub fn g2_wire(sub: G2SubCheck, path: Option<&[u8]>, calibration: Calibration) -> Wire {
    let kind = match sub {
        // PB §11: "a `forbidden` hit, and G7's hard lease over another intent's
        // forbidden or frozen set, block in every mode."
        G2SubCheck::ForbiddenHit => WireKind::Finding,
        _ => calibration.kind(),
    };
    match (sub, path) {
        (G2SubCheck::DiffSize, _) | (_, None) => {
            Wire::pathless(Gate::G2, WireClass::Tripwire, kind)
        }
        (_, Some(p)) => Wire::at(Gate::G2, p.to_vec(), WireClass::Tripwire, kind),
    }
}

/// PB §6.3 G2, verbatim: "**Diff size** is `git diff --numstat --no-renames`
/// over `merge-base..Hc`, additions plus deletions summed, binaries refused
/// rather than counted, floor and spine-owned paths exempt — a count two
/// implementations compute differently is a wire that fires on one and not the
/// other."
///
/// `None` for a binary entry: "binaries **refused** rather than counted", so a
/// binary in the diff ends the measurement rather than contributing zero.
pub fn diff_size(
    numstat: &[(Option<u64>, Option<u64>, Vec<u8>)],
    exempt: &dyn Fn(&[u8]) -> bool,
) -> Option<u64> {
    let mut total: u64 = 0;
    for (added, deleted, path) in numstat {
        if exempt(path) {
            continue;
        }
        // git prints `-` for both columns of a binary entry.
        //
        // Saturating, not wrapping: a debug build panics on overflow and a
        // release build wraps to a small number, and the small number is a
        // count that passes a bound it is astronomically above. `u64::MAX`
        // lines is not reachable from a real diff, but the two builds
        // disagreeing about a wire is exactly what PB §6.3 objects to.
        let entry = added.and_then(|a| deleted.map(|d| a.saturating_add(d)))?;
        total = total.saturating_add(entry);
    }
    Some(total)
}

/// Whether the diff-size sub-check fires.
///
/// **Contradiction C6 — the corpus fixes no gated-lane bound, and this
/// implementation invents none.** PB §2.1 makes `C-Q2` `quick.max_lines` and
/// PB §6.3's G2 row scopes the line count to the quick lane ("quick lane: ⊆
/// `C-Q1` ∪ floor ∪ spine-owned paths, and under `C-Q2` lines"), while PB §5.2
/// bullet 5 states it as a condition of *every* green pipeline. GR §6.3 "fixes
/// only the *measurement* and the *token*, not the bound." So the bound is a
/// caller's input and `None` fires nothing — the fail-**open** direction, taken
/// deliberately because the fail-closed one would be a threshold no document
/// states, firing a signed wire two implementations disagree about.
///
/// The bound is therefore a named lane and not an `Option`. `None` read as
/// "no bound" is the same value as `None` read as "the caller forgot", and on
/// the quick lane — where PB §6.3 does fix the bound at `C-Q2` — forgetting is
/// the only way to get one: CN §7.1's `effective` supplies a value for every
/// rule, always, so a quick-lane caller always has a number to pass.
pub fn diff_size_fires(count: u64, bound: DiffSizeBound) -> bool {
    match bound {
        DiffSizeBound::Quick(max) => count > max,
        DiffSizeBound::GatedUnbounded => false,
    }
}

/// Which bound the diff-size sub-check is measured against, per lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSizeBound {
    /// The quick lane's `C-Q2` (`quick.max_lines`). Fires strictly above it —
    /// a diff exactly at the bound is under it.
    Quick(u64),
    /// A gated lane. The corpus fixes no bound there (C6), so nothing fires,
    /// and this spelling makes that a decision the caller states rather than
    /// one it falls into.
    GatedUnbounded,
}

// ---------------------------------------------------------------------------
// G3 — Freshness · Staleness
// ---------------------------------------------------------------------------

/// GR §9.8: "at landing, G3 compares the sign-off event commit's committer date
/// to the committer date of `objects.base`".
///
/// GR §7 rule 1 is why both arguments are committer dates and neither is
/// "now": "No member holds a time, a duration, a date or anything derived from
/// one." PB §6.3 concedes what that buys: committer dates are "forgeable,
/// acceptable for a warning".
pub fn g3_is_stale(signoff_committer_date: i64, base_committer_date: i64) -> bool {
    base_committer_date - signoff_committer_date > STALENESS_WINDOW_SECS
}

/// GR §6.3: "bare `G3` … Staleness is a fact about the in-flight intent's
/// committer dates, not about a path, so there is nothing to put after the
/// colon."
pub fn g3_wire(calibration: Calibration) -> Wire {
    Wire::pathless(Gate::G3, WireClass::Tripwire, calibration.kind())
}

// ---------------------------------------------------------------------------
// G4 — Freshness · Currency
// ---------------------------------------------------------------------------

/// PB §6.3 G4: "An in-flight intent `built_under` a constitution bump flagged
/// `resign`, or stamped with a template version below the manifest's `resign`
/// floor (§6.7), trips a wire."
///
/// The `resign` map is indexed **by variant** — `intent`, `intent-bug`,
/// `intent-change` (MF §3.6, `spine_manifest::schema::RESIGN_KEYS`) — so the
/// comparison is per variant and never against a single floor.
pub fn g4_trips(
    constitution_bump_flagged_resign: bool,
    stamped_version: u64,
    resign_floor_for_variant: u64,
) -> bool {
    constitution_bump_flagged_resign || stamped_version < resign_floor_for_variant
}

/// GR §6.3: "bare `G4` … PB §6.3 G4 states both: 'trips a wire:
/// `landing-review` with `G4` — proceed by tripwire review, or a human
/// reopens'." G4 is **not** warn-calibrated (PB §6.3's *Warn* column reads no).
pub fn g4_wire() -> Wire {
    Wire::pathless(Gate::G4, WireClass::Tripwire, WireKind::Finding)
}

// ---------------------------------------------------------------------------
// G5 — Integrity · Orphans
// ---------------------------------------------------------------------------

/// GR §6.3 and PB §6.3 G5 as of v0.19: "One wire per offending pragma, token
/// `G5:<path>`, `class=tripwire`" — **the path is the blob the pragma sits in**,
/// "so a reviewer can find it without the report".
///
/// "**Two offending pragmas in one blob collapse to one entry** under §6.1's
/// `(gate, path)` uniqueness rule — the wire set is per path, the diagnostic is
/// per pragma, and the diagnostic is not in the report."
///
/// G5 is on **neither** bypass list: PB §7.6 says "never G5".
pub fn g5_wire(blob_path: &[u8]) -> Wire {
    Wire::at(
        Gate::G5,
        blob_path.to_vec(),
        WireClass::Tripwire,
        WireKind::Finding,
    )
}

// ---------------------------------------------------------------------------
// G7 — Drift · Interference
// ---------------------------------------------------------------------------

/// PB §6.3 G7's two clauses. "the class is what separates them."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum G7Clause {
    /// "`expected ∩ expected` → soft, surfaced to both owners."
    Soft,
    /// "the diff ∩ another intent's forbidden or frozen set → hard, a
    /// `class=protected` wire at landing."
    Hard,
}

/// GR §6.3's two G7 rows.
pub fn g7_wire(clause: G7Clause, path: &[u8], calibration: Calibration) -> Wire {
    match clause {
        G7Clause::Soft => Wire::at(
            Gate::G7,
            path.to_vec(),
            WireClass::Tripwire,
            calibration.kind(),
        ),
        // PB §11: "`finding` in **every** mode." GR §6.3: "The **ground-moved**
        // clause is anchored on the **binding approval's `base=`** … its
        // `∩ forbidden` half is a hard-clause wire and takes this row; its
        // `∩ touchpoints` half is a `spine check` diagnostic and is **not a
        // landing wire at all**."
        G7Clause::Hard => Wire::at(
            Gate::G7,
            path.to_vec(),
            WireClass::Protected,
            WireKind::Finding,
        ),
    }
}

// ---------------------------------------------------------------------------
// G12 — Strength · Red at approval
// ---------------------------------------------------------------------------

/// PB §6.3 G12: "`class=tripwire`, token `G12`, raised by `--approve` and
/// **never** by `--land`".
///
/// The argument is that the measurement "needs the base-restored tree and the
/// landing has no reason to recompute a number the signed approve line already
/// carries".
pub fn g12_approval_wire() -> Wire {
    Wire::pathless(Gate::G12, WireClass::Tripwire, WireKind::Finding)
}

/// GR §6.3: "**no version-1 landing report carries a `G12` entry in `wires`**,
/// and `gates[].G12` reads `pass`."
///
/// "A landing's only G12 check is that the copied approve line's `red=` is
/// present and well-formed, and a malformed one is an envelope G9's parse
/// refuses before any report seals."
pub fn g12_landing_raises_a_wire() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Contradiction C2's resolution: GR §6.3 wins via PB §11.
    #[test]
    fn the_diff_size_sub_check_takes_the_bare_g2_and_not_a_path() {
        let wire = g2_wire(G2SubCheck::DiffSize, None, Calibration::Block);
        assert_eq!(wire.token(), "G2");
        // Even handed a path, the count names none.
        let wire = g2_wire(G2SubCheck::DiffSize, Some(b"src/a.ts"), Calibration::Block);
        assert_eq!(wire.token(), "G2");
    }

    #[test]
    fn a_drift_path_takes_g2_plus_tok_of_the_path() {
        let wire = g2_wire(
            G2SubCheck::OutsideExpected,
            Some(b"src/shared/util.ts"),
            Calibration::Block,
        );
        assert_eq!(wire.token(), "G2:src/shared/util.ts");
        assert_eq!(wire.class, WireClass::Tripwire);
    }

    /// PB §11 and PB §6.3's closing paragraph: "a `forbidden` hit … block[s] in
    /// every mode."
    #[test]
    fn a_forbidden_hit_is_a_finding_even_under_calibration() {
        let warned = g2_wire(
            G2SubCheck::OutsideExpected,
            Some(b"src/a.ts"),
            Calibration::Warn,
        );
        assert_eq!(warned.kind, WireKind::Warn);
        let forbidden = g2_wire(
            G2SubCheck::ForbiddenHit,
            Some(b"src/secret.ts"),
            Calibration::Warn,
        );
        assert_eq!(forbidden.kind, WireKind::Finding);
    }

    /// PB §6.3 G2: "binaries refused rather than counted."
    #[test]
    fn a_binary_entry_refuses_the_diff_size_measurement() {
        let never_exempt = |_: &[u8]| false;
        let numstat = vec![
            (Some(10), Some(2), b"src/a.ts".to_vec()),
            (None, None, b"assets/logo.png".to_vec()),
        ];
        assert_eq!(diff_size(&numstat, &never_exempt), None);
    }

    /// "floor and spine-owned paths exempt."
    #[test]
    fn floor_and_spine_owned_paths_are_exempt_from_the_count() {
        let exempt = |p: &[u8]| p.starts_with(b".spine/");
        let numstat = vec![
            (Some(10), Some(2), b"src/a.ts".to_vec()),
            (Some(400), Some(400), b".spine/ci.sh".to_vec()),
        ];
        assert_eq!(diff_size(&numstat, &exempt), Some(12));
    }

    /// Contradiction C6: "an implementer must not invent a gated-lane bound."
    #[test]
    fn no_bound_fires_no_diff_size_wire() {
        assert!(!diff_size_fires(100_000, DiffSizeBound::GatedUnbounded));
        assert!(diff_size_fires(401, DiffSizeBound::Quick(400)));
        assert!(!diff_size_fires(400, DiffSizeBound::Quick(400)));
    }

    /// GR §9.8: "exactly **1 209 600 seconds** (14 days)".
    #[test]
    fn the_staleness_window_is_1_209_600_seconds_and_has_no_lever() {
        assert_eq!(STALENESS_WINDOW_SECS, 14 * 24 * 60 * 60);
        assert!(!g3_is_stale(0, STALENESS_WINDOW_SECS));
        assert!(g3_is_stale(0, STALENESS_WINDOW_SECS + 1));
    }

    #[test]
    fn g3_and_g4_take_the_bare_id() {
        assert_eq!(g3_wire(Calibration::Warn).token(), "G3");
        assert_eq!(g3_wire(Calibration::Warn).kind, WireKind::Warn);
        assert_eq!(g4_wire().token(), "G4");
        // PB §6.3's *Warn* column reads `no` for G4: it blocks from day one.
        assert_eq!(g4_wire().kind, WireKind::Finding);
    }

    /// MF §3.6: the `resign` map is indexed by variant, `intent@2` style.
    #[test]
    fn g4_compares_a_stamp_against_its_own_variants_floor() {
        assert!(g4_trips(false, 1, 2));
        assert!(!g4_trips(false, 2, 2));
        assert!(g4_trips(true, 4, 2));
    }

    /// GR §6.3: two offending pragmas in one blob collapse to one entry.
    #[test]
    fn two_pragmas_in_one_blob_collapse_to_one_g5_entry() {
        let mut set = crate::wire::WireSet::new();
        set.insert(g5_wire(b"src/billing/invoice.py"));
        set.insert(g5_wire(b"src/billing/invoice.py"));
        assert_eq!(set.tokens(), ["G5:src/billing/invoice.py"]);
    }

    /// PB §6.3 G7: "the class is what separates them."
    #[test]
    fn the_class_is_what_separates_g7s_two_clauses() {
        let soft = g7_wire(G7Clause::Soft, b"src/a.ts", Calibration::Block);
        let hard = g7_wire(G7Clause::Hard, b"src/a.ts", Calibration::Block);
        assert_eq!(soft.token(), hard.token());
        assert_eq!(soft.class, WireClass::Tripwire);
        assert_eq!(hard.class, WireClass::Protected);
    }

    /// PB §11: the hard clause "never warns".
    #[test]
    fn the_hard_lease_is_a_finding_in_every_mode() {
        let hard = g7_wire(G7Clause::Hard, b"src/a.ts", Calibration::Warn);
        assert_eq!(hard.kind, WireKind::Finding);
        let soft = g7_wire(G7Clause::Soft, b"src/a.ts", Calibration::Warn);
        assert_eq!(soft.kind, WireKind::Warn);
    }

    /// GR §6.3: G12 is "raised by `--approve` and **never** by `--land`".
    #[test]
    fn g12_is_raised_at_approval_and_never_at_a_landing() {
        assert_eq!(g12_approval_wire().token(), "G12");
        assert!(!g12_landing_raises_a_wire());
    }

    /// PB §6.3's *Warn* column, as a table.
    #[test]
    fn only_g2_g3_and_g7_soft_participate_in_calibration() {
        assert!(Gate::G2.warn_calibrated());
        assert!(Gate::G3.warn_calibrated());
        assert!(Gate::G7.warn_calibrated());
        for gate in [Gate::G1, Gate::G4, Gate::G5, Gate::G8, Gate::G12] {
            assert!(!gate.warn_calibrated(), "{gate}");
        }
    }
}
