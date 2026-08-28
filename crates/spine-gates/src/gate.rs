//! The sixteen gate ids, the five families, and which gates run.
//!
//! PB §6.3: "**five families** are the public vocabulary … G-numbers are
//! internal check IDs." Both spellings are here because both are written into
//! artifacts: the family names a human reads, the G-number a `gates[]` entry
//! and a `Spine-Gates` rendering carry.

use core::fmt;

/// PB §6.3's public vocabulary. §10 argues the fifth: "the four existing
/// families judge *what changed*; none judged *who may cause a landing*, and
/// hiding that under Integrity would bury a security boundary in a quality
/// label."
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Family {
    Integrity,
    Drift,
    Freshness,
    Strength,
    Authority,
}

impl Family {
    pub fn as_str(self) -> &'static str {
        match self {
            Family::Integrity => "Integrity",
            Family::Drift => "Drift",
            Family::Freshness => "Freshness",
            Family::Strength => "Strength",
            Family::Authority => "Authority",
        }
    }
}

impl fmt::Display for Family {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The four landing shapes GR §5.6.2's table is indexed by.
///
/// PB §11 puts every lifecycle landing — upgrade, rollback, uninstall,
/// re-init — on the quick lane, so they share [`LandingShape::Quick`]'s row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LandingShape {
    /// A gated `Spine-Event: land`.
    GatedLand,
    /// PB §5.4 step 2's withdrawal. "a landing nobody may sign is not a
    /// landing, whatever it does to the tree" (MF §4.8.1).
    Tombstone,
    /// The quick lane, and every toolkit lifecycle landing with it.
    Quick,
    /// `Spine-Event: reseal` (PB §5.5).
    Reseal,
}

/// `"G1"` … `"G16"`.
///
/// The `Ord` derived here is **numeric** — the discriminants are declared in
/// gate-number order — and that is `gates[]`'s order, never `wires[]`'s.
/// GR §5.6: "`gates[]` sorts by gate number ascending … **`wires[]` does
/// not** … The two orders differ deliberately and an implementation that
/// applies one to the other produces a different `report=` over identical
/// findings." [`crate::wire`] carries the other order and does not reuse this
/// one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Gate {
    G1,
    G2,
    G3,
    G4,
    G5,
    G6,
    G7,
    G8,
    G9,
    G10,
    G11,
    G12,
    G13,
    G14,
    G15,
    G16,
}

/// Numeric order, which is `gates[]`'s and `Spine-Gates`'s.
pub const ALL_GATES: [Gate; 16] = [
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

impl Gate {
    pub fn number(self) -> u8 {
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

    /// The `gate` member's value, and the prefix of every wire token.
    pub fn id(self) -> &'static str {
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
        ALL_GATES.into_iter().find(|g| g.id() == s)
    }

    /// PB §6.3's leftmost column.
    pub fn family(self) -> Family {
        match self {
            Gate::G1 | Gate::G5 | Gate::G8 | Gate::G9 | Gate::G10 => Family::Integrity,
            Gate::G2 | Gate::G7 => Family::Drift,
            Gate::G3 | Gate::G4 | Gate::G11 => Family::Freshness,
            Gate::G6 | Gate::G12 => Family::Strength,
            Gate::G13 | Gate::G14 | Gate::G15 | Gate::G16 => Family::Authority,
        }
    }

    /// GR §5.6.2's table, implemented literally. "The general rule: **a gate
    /// runs iff every input its PB §6.3 check reads exists for this landing.**
    /// The playbook enumerates only the tombstone case; the rest is resolved
    /// here."
    ///
    /// G6 and G10 are `false` on every shape and for two different reasons.
    /// G6 is "*roadmap 5, not v1*" and "arrives with the mechanism that
    /// configures it, under a `report_version` bump"; G10 "runs after the seal,
    /// and its own result cannot be inside the message `L`'s seal covers"
    /// (GR §5.6.1). Neither may produce a `gates` entry or a `wires` entry in a
    /// version-1 report.
    pub fn runs_on(self, shape: LandingShape) -> bool {
        use Gate::*;
        use LandingShape::*;
        match self {
            // Never in a version-1 report, on any shape.
            G6 | G10 => false,
            // PB §5.4 step 2 and PB §11: a tombstone's `Spine-Gates` lists
            // exactly these four.
            G9 | G13 | G14 | G15 => true,
            // "G3, G4 and G12 read an in-flight intent, an approval, or both,
            // and a subjectless landing has neither" — and the quick lane and a
            // reseal have no approval either.
            G3 | G4 | G12 => shape == GatedLand,
            // Everything else: every shape but the tombstone, which changes no
            // tree and runs no suite (RF §8.6).
            G1 | G2 | G5 | G7 | G8 | G11 | G16 => shape != Tombstone,
        }
    }

    /// PB §7.6's closed bypass list: "It bypasses G2, G3, G4, G6, G7, G12
    /// and — of Integrity — G8 and G1 only … never G5, G9, G10, G11, and never
    /// Authority."
    ///
    /// This is limb (b) of GR §5.6.1's `override`, and the **only** thing it
    /// decides. It never relaxes who must sign (PB §11), and it is unreachable
    /// on a reseal (PB §7.6: "a `Spine-Event: reseal` landing is not an intent,
    /// never reaches `tests-approved`, and therefore never gets a break-glass
    /// review").
    pub fn break_glass_bypassable(self) -> bool {
        matches!(
            self,
            Gate::G1 | Gate::G2 | Gate::G3 | Gate::G4 | Gate::G6 | Gate::G7 | Gate::G8 | Gate::G12
        )
    }

    /// PB §6.3's *Warn* column: the gates that participate in warn-before-block
    /// calibration. "every other gate blocks from day one."
    ///
    /// G7 is here for its **soft** clause only: PB §11 makes the hard lease
    /// "`finding` in every mode", and so is a `forbidden` hit under G2.
    pub fn warn_calibrated(self) -> bool {
        matches!(self, Gate::G2 | Gate::G3 | Gate::G7)
    }
}

impl fmt::Display for Gate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tombstone_runs_exactly_g9_g13_g14_g15() {
        let ran: Vec<&str> = ALL_GATES
            .into_iter()
            .filter(|g| g.runs_on(LandingShape::Tombstone))
            .map(Gate::id)
            .collect();
        assert_eq!(ran, ["G9", "G13", "G14", "G15"]);
    }

    #[test]
    fn g3_g4_and_g12_run_only_on_a_gated_land() {
        for gate in [Gate::G3, Gate::G4, Gate::G12] {
            assert!(gate.runs_on(LandingShape::GatedLand));
            for shape in [
                LandingShape::Tombstone,
                LandingShape::Quick,
                LandingShape::Reseal,
            ] {
                assert!(!gate.runs_on(shape), "{gate} must not run on {shape:?}");
            }
        }
    }

    #[test]
    fn no_version_1_report_runs_g6_or_g10() {
        for shape in [
            LandingShape::GatedLand,
            LandingShape::Tombstone,
            LandingShape::Quick,
            LandingShape::Reseal,
        ] {
            assert!(!Gate::G6.runs_on(shape));
            assert!(!Gate::G10.runs_on(shape));
        }
    }

    /// GR §5.6.2: "A **reseal** runs the suite." Its G1 evaluates every clause,
    /// so G1 must be in its gate set.
    #[test]
    fn a_reseal_runs_g1_and_g8() {
        assert!(Gate::G1.runs_on(LandingShape::Reseal));
        assert!(Gate::G8.runs_on(LandingShape::Reseal));
    }

    #[test]
    fn authority_is_never_break_glass_bypassable() {
        for gate in [Gate::G13, Gate::G14, Gate::G15, Gate::G16] {
            assert!(!gate.break_glass_bypassable(), "{gate}");
            assert_eq!(gate.family(), Family::Authority);
        }
        for gate in [Gate::G5, Gate::G9, Gate::G10, Gate::G11] {
            assert!(!gate.break_glass_bypassable(), "{gate}");
        }
    }

    /// GR §5.6: `gates[]` is numeric, and the derived `Ord` is what supplies
    /// it. A JCS object would have sorted these `g1, g10, g11, …, g2`.
    #[test]
    fn the_derived_order_on_gate_is_numeric_not_lexical() {
        let mut gates = [Gate::G2, Gate::G11, Gate::G1, Gate::G10];
        gates.sort();
        assert_eq!(
            gates.iter().map(|g| g.id()).collect::<Vec<_>>(),
            ["G1", "G2", "G10", "G11"]
        );
    }
}
