//! The sixteen gates: which of them run, what each records, and what each
//! *raises*.
//!
//! This module owns three tables the corpus states in three places and an
//! implementer must not re-derive:
//!
//! - **GR §5.6.2** — which gates run for each landing shape. PB §11 says
//!   `Spine-Gates` lists "every gate that ran" and enumerates only the
//!   tombstone; the rest is GR §9.12's, and getting it wrong changes the length
//!   of `gates[]`, hence `Spine-Gates`, hence `envelope=`.
//! - **GR §6.3** — the `class`, `kind` and token shape of the wire each gate
//!   raises. "A gate whose class is unassigned is a gate two implementations
//!   route differently, producing a different `wires` array, a different
//!   `report=` and a different `envelope=` over identical facts."
//! - **PB §7.6** — the eight gates a `class=break-glass` review may bypass.
//!
//! What is **not** here is gate semantics. GR §11: "This spec fixes how a
//! gate's *result* is recorded, never what the gate decides."

use core::fmt;

use crate::vocab::{GateStatus, LandingShape, WireClass, WireKind};

/// A gate id. Sixteen values, two of which never appear in a version-1
/// report's `gates` or `wires` — and both are variants anyway, because a
/// `class=break-glass` review may name `G6` in its `wires=` (GR §6.3) and
/// because a type that cannot spell `G10` cannot say that G10 is excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Gate {
    /// Coverage.
    G1,
    /// Containment.
    G2,
    /// Staleness.
    G3,
    /// Currency.
    G4,
    /// Orphans.
    G5,
    /// Mutation. "Roadmap 5, not v1" (PB §6.3, GR §5.6.2).
    G6,
    /// Interference.
    G7,
    /// Freeze.
    G8,
    /// Ledger.
    G9,
    /// Reconstruction. Never in `gates` and never in `Spine-Gates` (PB §11).
    G10,
    /// Base currency.
    G11,
    /// Red at approval.
    G12,
    /// Signers.
    G13,
    /// Floor.
    G14,
    /// Tool.
    G15,
    /// Scaffold.
    G16,
}

impl Gate {
    /// Every gate, ascending by **number** — which is `gates[]`'s order
    /// (GR §5.6) and emphatically not `wires[]`'s (GR §6.1).
    pub const ALL: [Gate; 16] = [
        Gate::G1,
        Gate::G2,
        Gate::G3,
        Gate::G4,
        Gate::G5,
        Gate::G6,
        Gate::G7,
        Gate::G8,
        Gate::G9,
        Gate::G10,
        Gate::G11,
        Gate::G12,
        Gate::G13,
        Gate::G14,
        Gate::G15,
        Gate::G16,
    ];

    /// The gate number. `gates[]` "sorts by gate number ascending — an array
    /// rather than an object because gate order is numeric and JCS would sort
    /// `g1, g10, g11, …, g2` by name" (GR §5.6).
    pub const fn number(self) -> u8 {
        match self {
            Gate::G1 => 1,
            Gate::G2 => 2,
            Gate::G3 => 3,
            Gate::G4 => 4,
            Gate::G5 => 5,
            Gate::G6 => 6,
            Gate::G7 => 7,
            Gate::G8 => 8,
            Gate::G9 => 9,
            Gate::G10 => 10,
            Gate::G11 => 11,
            Gate::G12 => 12,
            Gate::G13 => 13,
            Gate::G14 => 14,
            Gate::G15 => 15,
            Gate::G16 => 16,
        }
    }

    /// The token: `"G1"` … `"G16"` (GR §5.6, GR §6.1).
    pub const fn token(self) -> &'static str {
        match self {
            Gate::G1 => "G1",
            Gate::G2 => "G2",
            Gate::G3 => "G3",
            Gate::G4 => "G4",
            Gate::G5 => "G5",
            Gate::G6 => "G6",
            Gate::G7 => "G7",
            Gate::G8 => "G8",
            Gate::G9 => "G9",
            Gate::G10 => "G10",
            Gate::G11 => "G11",
            Gate::G12 => "G12",
            Gate::G13 => "G13",
            Gate::G14 => "G14",
            Gate::G15 => "G15",
            Gate::G16 => "G16",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Gate::ALL.into_iter().find(|g| g.token() == s)
    }

    /// PB §7.6's bypass list: "It bypasses G2, G3, G4, G6, G7, G12 and — of
    /// Integrity — G8 and G1 only … never G5, G9, G10, G11, and never
    /// Authority."
    ///
    /// This is limb (b) of GR §5.6.1's `override`. Encoding it as a property of
    /// the gate is what stops a break-glass review's `wires=` marking G14
    /// `override`: "break-glass may not bypass G14" (PB §11).
    pub const fn break_glass_bypassable(self) -> bool {
        matches!(
            self,
            Gate::G1 | Gate::G2 | Gate::G3 | Gate::G4 | Gate::G6 | Gate::G7 | Gate::G8 | Gate::G12
        )
    }

    /// GR §5.6.2's table. `true` iff every input this gate's PB §6.3 check
    /// reads exists for this landing shape.
    ///
    /// The tombstone row is the playbook's own: `Spine-Gates` lists only G9,
    /// G13, G14, G15 (PB §5.4 step 2, PB §11). "G3, G4 and G12 read an
    /// in-flight intent, an approval, or both, and a subjectless landing has
    /// neither."
    pub const fn runs_on(self, shape: LandingShape) -> bool {
        use LandingShape::{GatedLand, QuickLand, Reseal};
        match self {
            // "Never in a version-1 report" (GR §5.6.2). G6 has no
            // configuration source anywhere in the playbook, and "iff
            // configured" over a configuration that does not exist is two
            // implementations disagreeing about the length of `gates[]`.
            Gate::G6 => false,
            // "It runs after the seal, and its failure refuses the push"
            // (GR §6.3). PB §11 excludes it from `Spine-Gates` by name.
            Gate::G10 => false,
            // Authority and the ledger walk run on every shape, the tombstone
            // included — they are the four PB §5.4 step 2 names.
            Gate::G9 | Gate::G13 | Gate::G14 | Gate::G15 => true,
            // Read an in-flight intent, an approval, or both.
            Gate::G3 | Gate::G4 | Gate::G12 => matches!(shape, GatedLand),
            // Everything else runs wherever there is a tree and a suite.
            Gate::G1 | Gate::G2 | Gate::G5 | Gate::G7 | Gate::G8 | Gate::G11 | Gate::G16 => {
                matches!(shape, GatedLand | QuickLand | Reseal)
            }
        }
    }

    /// The gates that run for `shape`, ascending by number — exactly the
    /// `gates[]` array's membership and order.
    pub fn running_on(shape: LandingShape) -> Vec<Gate> {
        Gate::ALL.into_iter().filter(|g| g.runs_on(shape)).collect()
    }

    /// GR §6.3's row for this gate.
    pub const fn wire_spec(self) -> WireSpec {
        use KindRule as K;
        use TokenShape as T;
        use WireClassRule as C;
        match self {
            // "`protected` is the class because break-glass *never relaxes who
            // must sign* (PB §11) … a landing overriding the frozen-test floor
            // is exactly the emergency PB §7.6 says needs a second human."
            // `G1:` + tok(path) per-id; bare for the five that name no path.
            Gate::G1 => WireSpec::new(
                C::Fixed(WireClass::Protected),
                T::PathOrBare,
                K::Fixed(WireKind::Finding),
            ),
            // Drift. Bare `G2` is the diff-size sub-check: "a repository-wide
            // count that names no path". A `forbidden` hit is `finding` in
            // every mode (PB §11), which is why the kind is `MayWarn` and not
            // simply `Warn`.
            Gate::G2 => WireSpec::new(C::Fixed(WireClass::Tripwire), T::PathOrBare, K::MayWarn),
            // "Staleness is a fact about the in-flight intent's committer
            // dates, not about a path, so there is nothing to put after the
            // colon."
            Gate::G3 => WireSpec::new(C::Fixed(WireClass::Tripwire), T::Bare, K::MayWarn),
            Gate::G4 => WireSpec::new(
                C::Fixed(WireClass::Tripwire),
                T::Bare,
                K::Fixed(WireKind::Finding),
            ),
            // "One wire per offending pragma, token `G5:<path>`,
            // `class=tripwire`" — and two pragmas in one blob collapse to one
            // entry under GR §6.1's `(gate, path)` key.
            Gate::G5 => WireSpec::new(
                C::Fixed(WireClass::Tripwire),
                T::Path,
                K::Fixed(WireKind::Finding),
            ),
            Gate::G6 => WireSpec::new(C::NoWire, T::NoToken, K::NoWire),
            // Both clauses take `G7:` + tok(path); "the class is what separates
            // them" (PB §6.3), so the class is per clause.
            Gate::G7 => WireSpec::new(C::PerClause, T::Path, K::MayWarn),
            // Per clause: tripwire for harness-moved; protected for
            // branch-edited-before-approval, the landed-id clause, and `C-T3`.
            Gate::G8 => WireSpec::new(C::PerClause, T::Path, K::Fixed(WireKind::Finding)),
            // "G9 raises no wire. Its failures are refusals and index states,
            // not review material."
            Gate::G9 => WireSpec::new(C::NoWire, T::NoToken, K::NoWire),
            Gate::G10 => WireSpec::new(C::NoWire, T::NoToken, K::NoWire),
            // "`class=tripwire`, not protected — no floor path is touched"
            // (PB §5.5). Always advisory: G11's own check is base currency,
            // "whose failure ends the run rather than raising a wire".
            Gate::G11 => WireSpec::new(
                C::Fixed(WireClass::Tripwire),
                T::Bare,
                K::Fixed(WireKind::Advisory),
            ),
            Gate::G12 => WireSpec::new(
                C::Fixed(WireClass::Tripwire),
                T::Bare,
                K::Fixed(WireKind::Finding),
            ),
            // The `path` carries a **commit oid**, not a path (GR §6.1).
            Gate::G13 => WireSpec::new(
                C::Fixed(WireClass::Protected),
                T::CommitOid,
                K::Fixed(WireKind::Finding),
            ),
            Gate::G14 => WireSpec::new(
                C::Fixed(WireClass::Protected),
                T::Path,
                K::Fixed(WireKind::Finding),
            ),
            // "An unlisted `dist_hash` refuses locally and fails in `--ci`; it
            // is a membership test whose failure ends the run."
            Gate::G15 => WireSpec::new(C::NoWire, T::NoToken, K::NoWire),
            Gate::G16 => WireSpec::new(
                C::Fixed(WireClass::Protected),
                T::PathOrBare,
                K::Fixed(WireKind::Finding),
            ),
        }
    }
}

impl fmt::Display for Gate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// GR §6.3's `class` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireClassRule {
    /// "A gate that raises no wire in v1 is listed with an em dash rather than
    /// omitted, because *no row* and *no wire* are two different things to an
    /// implementer reading a table for the value to write."
    NoWire,
    /// Every wire this gate raises takes this class.
    Fixed(WireClass),
    /// G7 and G8: "**per clause**". The clause, not the gate, decides — see
    /// [`G7Clause`] and [`G8Clause`].
    PerClause,
}

/// GR §6.3's `kind` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindRule {
    NoWire,
    /// GR §6.1's two v1 invariants: "a **`G1` wire is always a `finding`** …
    /// and a **`G11` wire is always an `advisory`**."
    Fixed(WireKind),
    /// "`warn` under warn-before-block calibration, `finding` otherwise."
    /// Only G2, G3 and G7's soft clause can produce a `warn` (GR §6.1).
    MayWarn,
}

/// GR §6.3's token column: whether a wire from this gate carries a `:`-suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenShape {
    NoToken,
    /// Always `G<n>`, never a suffix.
    Bare,
    /// Always `G<n>:` + `tok(path)`.
    Path,
    /// Both forms occur, and which is which is the gate's own rule: G1's five
    /// pathless findings (GR §6.3), G2's diff-size sub-check, G16's checks that
    /// implicate no path.
    PathOrBare,
    /// `G13:` + the commit oid — "lowercase hex at the length `object_format`
    /// implies, for which `esc` — and `tok` — is the identity … nothing else in
    /// v1 puts a non-path there."
    CommitOid,
}

/// One row of GR §6.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireSpec {
    pub class: WireClassRule,
    pub token: TokenShape,
    pub kind: KindRule,
}

impl WireSpec {
    const fn new(class: WireClassRule, token: TokenShape, kind: KindRule) -> Self {
        WireSpec { class, token, kind }
    }

    /// Whether this gate raises any wire at all in a version-1 report.
    pub const fn raises_a_wire(self) -> bool {
        !matches!(self.class, WireClassRule::NoWire)
    }
}

/// G7's two clauses, which differ in class and in nothing else (PB §6.3: "the
/// class is what separates them").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum G7Clause {
    /// `expected ∩ expected`, surfaced to both owners.
    Soft,
    /// The integrated diff ∩ another intent's `forbidden` or frozen set. The
    /// ground-moved clause's `∩ forbidden` half takes this row; its
    /// `∩ touchpoints` half "is a `spine check` diagnostic and is **not a
    /// landing wire at all**" (GR §6.3).
    Hard,
}

impl G7Clause {
    pub const fn class(self) -> WireClass {
        match self {
            G7Clause::Soft => WireClass::Tripwire,
            // "`finding` in **every** mode (PB §11)" — a hard lease never
            // degrades to `warn` under calibration.
            G7Clause::Hard => WireClass::Protected,
        }
    }
}

/// G8's four clauses (GR §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum G8Clause {
    /// "A frozen path whose blob in `T` equals trunk's but not the approved one
    /// was changed by a landing on trunk" — PB §6's row: "landing-review, wire
    /// `G8:<path>`".
    HarnessMoved,
    /// PB §4.3: "is a `class=protected` wire `G8:<path>`".
    BranchEditedBeforeApproval,
    /// "A landed id `T` no longer collects or does not pass … unless that
    /// review names its path", which PB §6.3 G1 and GR §5.6.1 both spell
    /// `class=protected`.
    LandedId,
    /// "`C-T3` is assigned `protected` here: it is the tree grep PB §7.4 rests
    /// the isolation argument on, and a boundary the branch moved is not a
    /// finding its own author may sign away."
    CT3,
}

impl G8Clause {
    pub const fn class(self) -> WireClass {
        match self {
            G8Clause::HarnessMoved => WireClass::Tripwire,
            G8Clause::BranchEditedBeforeApproval | G8Clause::LandedId | G8Clause::CT3 => {
                WireClass::Protected
            }
        }
    }
}

/// One `gates[]` entry: `{"gate": "G<n>", "status": …}` (GR §5.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateResult {
    pub gate: Gate,
    pub status: GateStatus,
}

impl GateResult {
    pub const fn new(gate: Gate, status: GateStatus) -> Self {
        GateResult { gate, status }
    }
}

/// PB §11's `Spine-Gates` value: "a rendering of this array, in the same order,
/// as `G<n>=<status>`, space-separated" (GR §5.6.1).
///
/// The `Spine-Gates: ` field name is the envelope's, not this function's — this
/// returns the value alone, which is what the trailer's writer places after the
/// name and one space (PB §7.2).
/// **Sorted, because the serializer sorts.** GR §5.6 fixes `gates[]` as
/// "sorts by gate number ascending", so the array's canonical order is the
/// ascending one and rendering "the same order" means rendering that.
///
/// Left as the caller's order, an unsorted array reached `report=` sorted (the
/// serializer sorts) and the trailer rendered as handed — one value, two
/// renderings, and the one that reaches `envelope=` is the wrong one. An
/// unsorted array is also `Invariant::GatesOutOfOrder`; this makes the two
/// spellings agree even when nothing consulted the invariant.
pub fn spine_gates_value(gates: &[GateResult]) -> String {
    let mut sorted: Vec<&GateResult> = gates.iter().collect();
    sorted.sort_by_key(|g| g.gate.number());
    let mut out = String::new();
    for (i, g) in sorted.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(g.gate.token());
        out.push('=');
        out.push_str(g.status.token());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocab::GateStatus;

    /// GR §5.6: `gates[]` sorts by gate **number**, so `G2` precedes `G11`.
    /// The wire array does the opposite, and GR §5.6 says the two "differ
    /// deliberately and an implementation that applies one to the other
    /// produces a different `report=` over identical findings."
    #[test]
    fn gates_sort_by_number_so_g2_precedes_g11() {
        let running = Gate::running_on(LandingShape::GatedLand);
        let g2 = running.iter().position(|g| *g == Gate::G2).unwrap();
        let g11 = running.iter().position(|g| *g == Gate::G11).unwrap();
        assert!(g2 < g11);
        assert!(running.windows(2).all(|w| w[0].number() < w[1].number()));
    }

    /// PB §5.4 step 2 and PB §11: a tombstone's `Spine-Gates` lists only G9,
    /// G13, G14, G15.
    #[test]
    fn a_tombstone_runs_exactly_g9_g13_g14_g15() {
        assert_eq!(
            Gate::running_on(LandingShape::Tombstone),
            vec![Gate::G9, Gate::G13, Gate::G14, Gate::G15]
        );
    }

    /// GR §5.6.2's table, column by column. G3, G4 and G12 read an in-flight
    /// intent, an approval, or both.
    #[test]
    fn g3_g4_and_g12_run_only_on_a_gated_land() {
        for gate in [Gate::G3, Gate::G4, Gate::G12] {
            assert!(gate.runs_on(LandingShape::GatedLand), "{gate}");
            for shape in [
                LandingShape::Tombstone,
                LandingShape::QuickLand,
                LandingShape::Reseal,
            ] {
                assert!(!gate.runs_on(shape), "{gate} on {shape:?}");
            }
        }
    }

    /// PB §7.4 rule 5: "a reseal does run the suite", so G1 runs on it. Its
    /// gate set differs from a quick landing's in nothing.
    #[test]
    fn a_reseal_runs_the_same_gates_as_a_quick_landing() {
        assert_eq!(
            Gate::running_on(LandingShape::Reseal),
            Gate::running_on(LandingShape::QuickLand)
        );
        assert!(Gate::G1.runs_on(LandingShape::Reseal));
    }

    /// GR §5.6.2 and PB §11. "G6 arrives with the mechanism that configures it,
    /// under a `report_version` bump"; G10 "runs after the seal".
    #[test]
    fn g6_and_g10_never_appear_in_a_version_one_report() {
        for shape in [
            LandingShape::GatedLand,
            LandingShape::Tombstone,
            LandingShape::QuickLand,
            LandingShape::Reseal,
        ] {
            assert!(!Gate::G6.runs_on(shape), "G6 on {shape:?}");
            assert!(!Gate::G10.runs_on(shape), "G10 on {shape:?}");
        }
    }

    /// GR §8.2's published rendering of that report's `gates` array.
    #[test]
    fn gr_8_2_spine_gates_rendering() {
        let mut gates: Vec<GateResult> = Gate::running_on(LandingShape::GatedLand)
            .into_iter()
            .map(|g| GateResult::new(g, GateStatus::Pass))
            .collect();
        for g in &mut gates {
            if g.gate == Gate::G2 {
                g.status = GateStatus::Override;
            }
        }
        assert_eq!(
            format!("Spine-Gates: {}", spine_gates_value(&gates)),
            "Spine-Gates: G1=pass G2=override G3=pass G4=pass G5=pass G7=pass G8=pass \
             G9=pass G11=pass G12=pass G13=pass G14=pass G15=pass G16=pass"
        );
    }

    /// PB §7.6's list, exactly eight. "Never G5, G9, G10, G11, and never
    /// Authority."
    #[test]
    fn exactly_eight_gates_are_break_glass_bypassable() {
        let bypassable: Vec<Gate> = Gate::ALL
            .into_iter()
            .filter(|g| g.break_glass_bypassable())
            .collect();
        assert_eq!(
            bypassable,
            vec![
                Gate::G1,
                Gate::G2,
                Gate::G3,
                Gate::G4,
                Gate::G6,
                Gate::G7,
                Gate::G8,
                Gate::G12
            ]
        );
        for authority in [Gate::G13, Gate::G14, Gate::G15, Gate::G16] {
            assert!(!authority.break_glass_bypassable(), "{authority}");
        }
        for never in [Gate::G5, Gate::G9, Gate::G10, Gate::G11] {
            assert!(!never.break_glass_bypassable(), "{never}");
        }
    }

    /// GR §6.3: "Authority (G13, G14, G15, G16) … Its wires are `protected`."
    /// G15 raises none, so the assertion is over the three that do.
    #[test]
    fn every_authority_wire_is_protected() {
        for gate in [Gate::G13, Gate::G14, Gate::G16] {
            assert_eq!(
                gate.wire_spec().class,
                WireClassRule::Fixed(WireClass::Protected),
                "{gate}"
            );
        }
        assert!(!Gate::G15.wire_spec().raises_a_wire());
    }

    /// GR §6.3: "Drift (G2, G7-soft), Freshness (G3, G4, G11) and Strength
    /// (G12) route to `landing-review` … Their wires are `tripwire`."
    #[test]
    fn drift_freshness_and_strength_wires_are_tripwire() {
        for gate in [Gate::G2, Gate::G3, Gate::G4, Gate::G11, Gate::G12] {
            assert_eq!(
                gate.wire_spec().class,
                WireClassRule::Fixed(WireClass::Tripwire),
                "{gate}"
            );
        }
        assert_eq!(G7Clause::Soft.class(), WireClass::Tripwire);
        assert_eq!(G7Clause::Hard.class(), WireClass::Protected);
    }

    /// GR §6.3's G8 row: tripwire for harness-moved alone.
    #[test]
    fn only_g8s_harness_moved_clause_is_tripwire() {
        assert_eq!(G8Clause::HarnessMoved.class(), WireClass::Tripwire);
        for clause in [
            G8Clause::BranchEditedBeforeApproval,
            G8Clause::LandedId,
            G8Clause::CT3,
        ] {
            assert_eq!(clause.class(), WireClass::Protected, "{clause:?}");
        }
        assert_eq!(Gate::G8.wire_spec().class, WireClassRule::PerClause);
    }

    /// GR §6.1: "a **`G1` wire is always a `finding`** … and a **`G11` wire is
    /// always an `advisory`**." PB §12 records the collision that made this
    /// worth stating.
    #[test]
    fn g1_is_always_a_finding_and_g11_always_an_advisory() {
        assert_eq!(
            Gate::G1.wire_spec().kind,
            KindRule::Fixed(WireKind::Finding)
        );
        assert_eq!(
            Gate::G11.wire_spec().kind,
            KindRule::Fixed(WireKind::Advisory)
        );
    }

    /// GR §6.1: only G2, G3 and G7's soft clause can produce a `warn`.
    #[test]
    fn only_g2_g3_and_g7_may_warn() {
        let may_warn: Vec<Gate> = Gate::ALL
            .into_iter()
            .filter(|g| g.wire_spec().kind == KindRule::MayWarn)
            .collect();
        assert_eq!(may_warn, vec![Gate::G2, Gate::G3, Gate::G7]);
    }

    /// GR §6.3: G3, G4, G11 and G12 name no path; G5, G7, G8 and G14 always do;
    /// G13's suffix is a commit oid.
    #[test]
    fn the_token_shapes_match_the_6_3_table() {
        for gate in [Gate::G3, Gate::G4, Gate::G11, Gate::G12] {
            assert_eq!(gate.wire_spec().token, TokenShape::Bare, "{gate}");
        }
        for gate in [Gate::G5, Gate::G7, Gate::G8, Gate::G14] {
            assert_eq!(gate.wire_spec().token, TokenShape::Path, "{gate}");
        }
        assert_eq!(Gate::G13.wire_spec().token, TokenShape::CommitOid);
        for gate in [Gate::G1, Gate::G2, Gate::G16] {
            assert_eq!(gate.wire_spec().token, TokenShape::PathOrBare, "{gate}");
        }
    }

    /// GR §6.3's em-dash rows: G6, G9, G10 and G15 raise nothing.
    #[test]
    fn four_gates_raise_no_wire_in_v1() {
        let silent: Vec<Gate> = Gate::ALL
            .into_iter()
            .filter(|g| !g.wire_spec().raises_a_wire())
            .collect();
        assert_eq!(silent, vec![Gate::G6, Gate::G9, Gate::G10, Gate::G15]);
    }

    #[test]
    fn gate_tokens_round_trip() {
        for g in Gate::ALL {
            assert_eq!(Gate::parse(g.token()), Some(g));
            assert_eq!(g.token(), format!("G{}", g.number()));
        }
        assert!(Gate::parse("G17").is_none());
        assert!(Gate::parse("g1").is_none());
    }
}
