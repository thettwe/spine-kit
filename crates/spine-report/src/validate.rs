//! The cross-member invariants GR §5–§7 state and no single field can enforce.
//!
//! Everything a *type* can hold is already held by one: a `Gate` cannot spell
//! `G17`, an `Oid` cannot be uppercase, a `PreconditionStatus` cannot be
//! `"unverifiable"`. What is left is the class of rule that relates two members
//! — "`profile` is `n/a` iff the landing runs no suite", "for each `floor_hits`
//! entry, `wires` contains exactly one `G14` entry" — and those are here.
//!
//! **The tokens below are DERIVED.** GR fixes refusal tokens for a reader
//! meeting an unknown version or member (`report-version-unknown`, §3.2) and
//! for `--verify`'s six outcomes (§4.3), and fixes none for a report that
//! parses and then contradicts itself. These names are this crate's, they reach
//! no signed line, and a document that later fixes tokens for them wins.
//!
//! Every check is **fail-closed**: a report that trips one is refused rather
//! than repaired, because each of these members is inside `report=` and a
//! silent repair is a digest two implementations disagree about.

use core::fmt;

use crate::gate::Gate;
use crate::report::Report;
use crate::vocab::{
    AutoMerge, Event, LandingShape, Mode, PreconditionStatus, SealProfile, WireClass, WireKind,
};
use crate::wire::Wire;
use crate::{KindRule, WireClassRule};

/// GR §2.2 / §7 rule 7: "Integers only, `0 ≤ n ≤ 2^53 − 1`."
pub const MAX_INT: u64 = 9_007_199_254_740_991;

/// One violated invariant. Names the rule, never echoes the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invariant {
    /// GR §7 rule 9: "lowercase hex at the full length `object_format` implies".
    /// The member's own type checked the alphabet; only the report knows the
    /// format.
    OidWidth { member: &'static str },
    /// GR §5.2: `intent_blob` is "present iff `subject.intent` is present".
    IntentBlobPresence,
    /// GR §5.5: `withdraw` is "present iff `subject.event == "withdraw"`".
    WithdrawPresence,
    /// GR §5.5: `approve` is "gated landing only; never on a tombstone, quick,
    /// lifecycle or reseal landing".
    ApproveOnNonGatedLanding,
    /// GR §5: `mode` "equals `policy.rules.c_a1` except on a recovery-sealed
    /// landing (PB §7.5), where it is `recovery`".
    ModeDisagreesWithCA1,
    /// GR §5: `profile` is `"n/a"` "iff the landing runs no suite, which PB §11
    /// makes the tombstone and nothing else".
    ProfileNotApplicableOffTombstone,
    /// GR §5.9: `evidence` is "present iff a result file was ingested", and a
    /// landing that runs no suite ingests none.
    EvidenceOnALandingThatRanNoSuite,
    /// GR §5.6.2's table: `Spine-Gates` lists "every gate that ran", no more
    /// and no fewer.
    GateSetDisagreesWithTheShape,
    /// GR §5.8: `"exempt"` "is used only where the design grants exemption, and
    /// the grant is PB §7.4 rule 5's own, singular: a **tombstone** … **A
    /// reseal is exempt from nothing.**"
    ExemptOffTombstone,
    /// GR §5.7: "for each entry `p`, `wires` contains exactly one
    /// `{gate: "G14", path: p, class: "protected", kind: "finding"}`, and
    /// `wires` contains no other `G14` entry."
    FloorHitsAndG14WiresDisagree,
    /// GR §5.10: `reverifications` is "≥ 0, ≤ `policy.rules.c_m3`".
    ReverificationsAboveCM3,
    /// GR §2.2 / §7 rule 7.
    IntegerOutOfProfile { member: &'static str },
    /// GR §5.4.1: "`c_t3` is `true` in every version-1 report, and that is the
    /// answer, not an oversight."
    CT3NotTrue,
    /// GR §5.4: `floor_extensions` is "every entry of `C-A2` plus every value
    /// of every `paths.*` key". GR §9.10 names the `C-A2` half as one of the
    /// schema's three checkable redundancies: "`policy.floor_extensions`
    /// restates every entry of `policy.rules.c_a2` under a second ordering."
    /// The `paths.*` half is not checkable here — `policy.manifest` pins it as
    /// an oid and this crate reads no blob.
    FloorExtensionsMissingCA2Entry,
    /// GR §6.3's em-dash rows: G6, G9, G10 and G15 raise no wire in v1.
    WireFromAGateThatRaisesNone { gate: Gate },
    /// GR §6.3's G12 row, which is not an em-dash row and is still forbidden
    /// here: G12's wire is "raised by `--approve` and **never** by `--land`",
    /// so "no version-1 landing report carries a `G12` entry in `wires`, and
    /// `gates[].G12` reads `pass`."
    G12WireInALandingReport,
    /// A wire from a gate that did not run for this landing shape (GR §5.6.2).
    WireFromAGateThatDidNotRun { gate: Gate },
    /// GR §6.3's `class` column, where the row fixes one value.
    WireClassDisagreesWith63 { gate: Gate },
    /// GR §6.1: "a **`G1` wire is always a `finding`** … and a **`G11` wire is
    /// always an `advisory`**."
    WireKindDisagreesWith61 { gate: Gate },
    /// GR §5.8: the rule-5 wire "attaches to every landing rule 5 applies to",
    /// and is raised iff auto-merge is off or a precondition is unmet.
    RuleFiveWirePresence { expected: bool },
}

impl fmt::Display for Invariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Invariant::OidWidth { member } => {
                write!(
                    f,
                    "oid-width: {member} is not the width object_format implies"
                )
            }
            Invariant::IntentBlobPresence => f.write_str(
                "intent-blob-presence: objects.intent_blob is present iff subject.intent is",
            ),
            Invariant::WithdrawPresence => f.write_str(
                "withdraw-presence: authority.withdraw is present iff subject.event is withdraw",
            ),
            Invariant::ApproveOnNonGatedLanding => {
                f.write_str("approve-on-non-gated-landing: authority.approve is gated-land only")
            }
            Invariant::ModeDisagreesWithCA1 => {
                f.write_str("mode-disagrees-with-c-a1: mode equals c_a1 unless it is recovery")
            }
            Invariant::ProfileNotApplicableOffTombstone => f.write_str(
                "profile-n-a-off-tombstone: profile is \"n/a\" iff the landing runs no suite",
            ),
            Invariant::EvidenceOnALandingThatRanNoSuite => f.write_str(
                "evidence-on-a-landing-that-ran-no-suite: no suite ingests no result file",
            ),
            Invariant::GateSetDisagreesWithTheShape => f.write_str(
                "gate-set-disagrees-with-the-shape: gates[] lists exactly the gates that ran",
            ),
            Invariant::ExemptOffTombstone => f.write_str(
                "exempt-off-tombstone: \"exempt\" is granted to the tombstone and nothing else",
            ),
            Invariant::FloorHitsAndG14WiresDisagree => f.write_str(
                "floor-hits-and-g14-wires-disagree: one protected G14 finding per floor hit, \
                 and no other G14 entry",
            ),
            Invariant::ReverificationsAboveCM3 => {
                f.write_str("reverifications-above-c-m3: run.reverifications exceeds c_m3")
            }
            Invariant::IntegerOutOfProfile { member } => {
                write!(f, "integer-out-of-profile: {member} exceeds 2^53 - 1")
            }
            Invariant::CT3NotTrue => {
                f.write_str("c-t3-not-true: c_t3 is true in every version-1 report")
            }
            Invariant::FloorExtensionsMissingCA2Entry => f.write_str(
                "floor-extensions-missing-c-a2-entry: floor_extensions restates every C-A2 entry",
            ),
            Invariant::WireFromAGateThatRaisesNone { gate } => {
                write!(f, "wire-from-a-gate-that-raises-none: {gate}")
            }
            Invariant::G12WireInALandingReport => f.write_str(
                "g12-wire-in-a-landing-report: G12 raises its wire at --approve, never at --land",
            ),
            Invariant::WireFromAGateThatDidNotRun { gate } => {
                write!(f, "wire-from-a-gate-that-did-not-run: {gate}")
            }
            Invariant::WireClassDisagreesWith63 { gate } => {
                write!(f, "wire-class-disagrees-with-6-3: {gate}")
            }
            Invariant::WireKindDisagreesWith61 { gate } => {
                write!(f, "wire-kind-disagrees-with-6-1: {gate}")
            }
            Invariant::RuleFiveWirePresence { expected } => write!(
                f,
                "rule-five-wire-presence: a bare G11 advisory {} be in this set",
                if *expected { "must" } else { "must not" }
            ),
        }
    }
}

impl core::error::Error for Invariant {}

impl Report {
    /// Every invariant this report violates, in a fixed order. Empty is
    /// conforming.
    ///
    /// A `Vec` rather than a first-failure `Result`: a report is assembled once
    /// and inspected by a human when it is wrong, and reporting one violation
    /// at a time turns one debugging session into five.
    pub fn validate(&self) -> Vec<Invariant> {
        let mut out = Vec::new();
        let shape = self.shape();

        // GR §7 rule 9. Each id's alphabet was checked by `Oid`; the width
        // depends on a member of *this* report.
        for (member, oid) in [
            ("objects.base", &self.objects.base),
            ("objects.head", &self.objects.head),
            ("objects.merge_base", &self.objects.merge_base),
            ("objects.tree", &self.objects.tree),
            ("policy.manifest", &self.policy.manifest),
            ("policy.keyring", &self.policy.keyring),
            ("policy.constitution", &self.policy.constitution),
            ("policy.ci_sh", &self.policy.ci_sh),
        ] {
            if !oid.fits(self.object_format) {
                out.push(Invariant::OidWidth { member });
            }
        }
        if let Some(blob) = &self.objects.intent_blob
            && !blob.fits(self.object_format)
        {
            out.push(Invariant::OidWidth {
                member: "objects.intent_blob",
            });
        }

        if self.subject.intent.is_some() != self.objects.intent_blob.is_some() {
            out.push(Invariant::IntentBlobPresence);
        }

        if self.authority.withdraw.is_some() != matches!(self.subject.event, Event::Withdraw) {
            out.push(Invariant::WithdrawPresence);
        }

        if self.authority.approve.is_some() && shape != LandingShape::GatedLand {
            out.push(Invariant::ApproveOnNonGatedLanding);
        }

        if self.mode != Mode::Recovery && self.mode != self.policy.rules.c_a1.as_mode() {
            out.push(Invariant::ModeDisagreesWithCA1);
        }

        // GR §5: "`n/a` iff the landing runs no suite" — an *iff*, so both
        // directions are checked. A reseal recording `n/a` is the error PB §7.4
        // rule 5 corrects by name.
        let claims_no_suite = self.profile == SealProfile::NotApplicable;
        if claims_no_suite != !shape.runs_suite() {
            out.push(Invariant::ProfileNotApplicableOffTombstone);
        }

        if self.evidence.is_some() && !shape.runs_suite() {
            out.push(Invariant::EvidenceOnALandingThatRanNoSuite);
        }

        let expected_gates = Gate::running_on(shape);
        let mut actual_gates: Vec<Gate> = self.gates.iter().map(|g| g.gate).collect();
        actual_gates.sort_by_key(|g| g.number());
        actual_gates.dedup();
        if actual_gates != expected_gates || actual_gates.len() != self.gates.len() {
            out.push(Invariant::GateSetDisagreesWithTheShape);
        }

        // GR §5.8's exemption is the tombstone's alone, and it is all-or-none:
        // "all five `exempt`".
        let any_exempt = self
            .automerge
            .preconditions
            .contains(&PreconditionStatus::Exempt);
        if any_exempt != (shape == LandingShape::Tombstone) {
            out.push(Invariant::ExemptOffTombstone);
        }

        out.extend(self.validate_floor());
        out.extend(self.validate_wires(shape));

        if self.run.reverifications > self.policy.rules.c_m3 {
            out.push(Invariant::ReverificationsAboveCM3);
        }

        for (member, n) in [
            ("policy.rules.c_m3", self.policy.rules.c_m3),
            ("policy.rules.c_q2", self.policy.rules.c_q2),
            ("run.reverifications", self.run.reverifications),
        ] {
            if n > MAX_INT {
                out.push(Invariant::IntegerOutOfProfile { member });
            }
        }
        if let Some(e) = &self.evidence
            && e.ids > MAX_INT
        {
            out.push(Invariant::IntegerOutOfProfile {
                member: "evidence.ids",
            });
        }

        if !self.policy.rules.c_t3 {
            out.push(Invariant::CT3NotTrue);
        }

        // GR §9.10's third redundancy. Compared over the raw bytes: both
        // members hold the constitution's own bytes, and `esc` is injective, so
        // the raw comparison and the encoded one agree.
        if !self
            .policy
            .rules
            .c_a2
            .iter()
            .all(|p| self.policy.floor_extensions.contains(p))
        {
            out.push(Invariant::FloorExtensionsMissingCA2Entry);
        }

        out
    }

    /// GR §5.7's invariant, both halves.
    fn validate_floor(&self) -> Option<Invariant> {
        let expected: Vec<Wire> = Report::floor_wires(&self.floor_hits);
        let actual: Vec<Wire> = self
            .wires
            .as_slice()
            .iter()
            .filter(|w| w.gate == Gate::G14)
            .cloned()
            .collect();
        // Compared as *sets over the token*, because `expected` is in
        // `floor_hits` order and `actual` is in the array's byte order — the
        // rule is about membership, and re-sorting one to match the other would
        // hide a duplicate.
        let mut want: Vec<String> = expected.iter().map(Wire::token).collect();
        let mut have: Vec<String> = actual.iter().map(Wire::token).collect();
        want.sort();
        have.sort();
        let classes_and_kinds_hold = actual
            .iter()
            .all(|w| w.class == WireClass::Protected && w.kind == WireKind::Finding);
        if want != have || !classes_and_kinds_hold {
            return Some(Invariant::FloorHitsAndG14WiresDisagree);
        }
        None
    }

    /// GR §6.3's table, applied to the array this report actually carries.
    fn validate_wires(&self, shape: LandingShape) -> Vec<Invariant> {
        let mut out = Vec::new();
        for w in self.wires.as_slice() {
            let spec = w.gate.wire_spec();
            match spec.class {
                WireClassRule::NoWire => {
                    out.push(Invariant::WireFromAGateThatRaisesNone { gate: w.gate });
                    // A gate that raises nothing has no class or kind to check.
                    continue;
                }
                WireClassRule::Fixed(c) if c != w.class => {
                    out.push(Invariant::WireClassDisagreesWith63 { gate: w.gate });
                }
                _ => {}
            }
            // GR §6.3's G12 row: "no version-1 landing report carries a `G12`
            // entry in `wires`, and `gates[].G12` reads `pass`." G12's wire is
            // "raised by `--approve` and **never** by `--land`" (PB §6.3).
            if w.gate == Gate::G12 {
                out.push(Invariant::G12WireInALandingReport);
            }
            if !w.gate.runs_on(shape) {
                out.push(Invariant::WireFromAGateThatDidNotRun { gate: w.gate });
            }
            if let KindRule::Fixed(k) = spec.kind
                && k != w.kind
            {
                out.push(Invariant::WireKindDisagreesWith61 { gate: w.gate });
            }
        }

        // GR §5.8: the rule-5 wire is raised iff rule 5 applies to this landing
        // *and* either reason holds. PB §11 pins the common case: precondition
        // 0 "fails on every run that tests anything, so the `G11` precondition
        // wire is present in every such set, in every lane".
        let requested = crate::Automerge::requested(self.policy.rules.c_m4);
        let any_unmet = self
            .automerge
            .preconditions
            .contains(&PreconditionStatus::Unmet);
        let expected = shape.automerge_rule_applies() && (!requested || any_unmet);
        let present = self
            .wires
            .as_slice()
            .iter()
            .any(|w| w.gate == Gate::G11 && w.path.is_none());
        if present != expected {
            out.push(Invariant::RuleFiveWirePresence { expected });
        }
        out
    }
}

/// GR §5.8's two reasons, both raising the one bare `G11` advisory.
///
/// "PB §5.2 states both in one clause: *`G11` (`C-M4`) where the constitution
/// says off, and `G11` naming the precondition where the run computed it off —
/// one gate, two reasons, distinguished by `reason=`.*"
///
/// The `reason=` is a review's, "which is not a member of this report"
/// (GR §9.13), so this function returns the wire and not the reason.
pub fn rule_five_wire(
    shape: LandingShape,
    c_m4: AutoMerge,
    preconditions: &[PreconditionStatus; 5],
) -> Option<Wire> {
    if !shape.automerge_rule_applies() {
        return None;
    }
    let off = !crate::Automerge::requested(c_m4);
    let unmet = preconditions.contains(&PreconditionStatus::Unmet);
    // "Both conditions can hold at once, and the wire set carries one entry."
    // Under the shipped defaults (`C-A3: hostile`, `C-M4: off`) both do.
    (off || unmet).then(|| Wire::bare(Gate::G11, WireClass::Tripwire, WireKind::Advisory))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GR §5.8 and PB §11: "a tombstone runs no gates that can produce a rule-5
    /// `G11` wire" and "is exempt from the rule entirely".
    #[test]
    fn a_tombstone_raises_no_rule_five_wire_under_either_reason() {
        assert!(
            rule_five_wire(
                LandingShape::Tombstone,
                AutoMerge::Off,
                &[PreconditionStatus::Exempt; 5]
            )
            .is_none()
        );
        assert!(
            rule_five_wire(
                LandingShape::Tombstone,
                AutoMerge::On,
                &[PreconditionStatus::Unmet; 5]
            )
            .is_none()
        );
    }

    /// GR §5.8: "An implementation that gates the precondition wire behind
    /// `requested == true` gets the shipped defaults wrong forever — under them
    /// `requested` is false *and* precondition 0 fails" (GR §9.13).
    #[test]
    fn the_precondition_wire_is_not_gated_behind_c_m4_on() {
        let shipped = [
            PreconditionStatus::Unmet, // C-A3: hostile
            PreconditionStatus::Met,
            PreconditionStatus::Met,
            PreconditionStatus::Met,
            PreconditionStatus::Met,
        ];
        let w = rule_five_wire(LandingShape::QuickLand, AutoMerge::Off, &shipped).unwrap();
        assert_eq!(w.token(), "G11");
        assert_eq!(w.class, WireClass::Tripwire);
        assert_eq!(w.kind, WireKind::Advisory);
    }

    /// A landing with auto-merge on and every precondition met raises nothing:
    /// the wire exists so a human reads a landing whose auto-merge is
    /// unavailable, and here it is available.
    #[test]
    fn a_fully_met_landing_under_c_m4_on_raises_no_rule_five_wire() {
        assert!(
            rule_five_wire(
                LandingShape::GatedLand,
                AutoMerge::On,
                &[PreconditionStatus::Met; 5]
            )
            .is_none()
        );
    }

    /// PB §7.4 rule 5 as of v0.18: "a reseal … raises it like any other landing
    /// that tests something."
    #[test]
    fn a_reseal_raises_the_rule_five_wire_like_anything_else() {
        let hostile = [
            PreconditionStatus::Unmet,
            PreconditionStatus::Met,
            PreconditionStatus::Met,
            PreconditionStatus::Met,
            PreconditionStatus::Met,
        ];
        assert!(rule_five_wire(LandingShape::Reseal, AutoMerge::On, &hostile).is_some());
    }
}
