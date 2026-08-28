//! PB §7.4 rule 5, made a record: the five auto-merge preconditions, and the
//! one `G11` wire a failure raises.
//!
//! > `C-M4: merge.auto = on` is a **request**, not a capability. Whether a run
//! > may act on it is computed per run, from these five preconditions, "each
//! > read from trunk or produced by this run, never asserted by the branch
//! > asking to merge".
//!
//! PB §5.2 closes the same argument from the other end: "**Auto-merge is not a
//! button.** It is the compare-and-swap of §5.4 … A green branch is not a fact
//! about trunk."

use crate::gate::{Gate, LandingShape};
use crate::wire::{Wire, WireClass, WireKind};
use core::fmt;

/// GR §5.8's three values, and no fourth.
///
/// "Do not widen the precondition status domain. `\"unverifiable\"`, or a split
/// of `\"unmet\"` by cause, is **rejected**: it would put a new token inside a
/// digest-bearing member and force a `report_version` bump for a distinction no
/// gate reads."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreconditionStatus {
    Met,
    Unmet,
    /// "used **only** where the design grants exemption, and the single grant
    /// is the **tombstone**" (GR §5.8).
    Exempt,
}

impl PreconditionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PreconditionStatus::Met => "met",
            PreconditionStatus::Unmet => "unmet",
            PreconditionStatus::Exempt => "exempt",
        }
    }

    fn permits_effective(self) -> bool {
        self != PreconditionStatus::Unmet
    }
}

impl fmt::Display for PreconditionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a run observed, before it is reduced to five statuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observations {
    /// Precondition 0. "`C-A3: threat.candidate` is `trusted`. Under
    /// `hostile` — the default — `C-M4` can **never** evaluate `on`, whatever
    /// the rest say."
    pub threat_candidate_trusted: bool,
    /// Precondition 1, first conjunct: trunk's `params.isolation` is
    /// `container`. "`params.isolation` absent means `none`, so it fails; `uid`
    /// is not an alternative here" (RF §8.4).
    pub manifest_isolation_is_container: bool,
    /// Precondition 1, second conjunct: the ingested header's `profile=` equals
    /// it. "A disagreement — including a manifest claiming `container` against
    /// a header reporting `none` — fails the precondition and no more."
    pub header_profile_is_container: bool,
    /// Precondition 2, conjunct 1: the header's `keys_visible=` is `false`.
    /// "`keys_visible=true` is a **legal** header value producing a legal
    /// report" (GR §5.8) — it does not refuse ingestion.
    pub keys_not_visible: bool,
    /// Precondition 2, conjunct 2: RF §8.3 step 2 passed.
    pub tool_matches_pin: bool,
    /// Precondition 2, conjunct 3: "this run established that the ingested file
    /// came from a job whose definition was taken from trunk" (PB §7.4 rule 0,
    /// as amended 2026-08-26). Its absence "does nothing else at all"
    /// (RF §8.4).
    pub trunk_defined_origin: bool,
    /// Precondition 4: "this run performs step 6's compare-and-swap itself and
    /// the object it pushes is the object that becomes trunk's tip."
    pub performs_the_cas: bool,
}

impl Default for Observations {
    /// The shipped defaults, fail-closed: `C-A3: hostile` (PB §2.1), nothing
    /// established.
    fn default() -> Self {
        Observations {
            threat_candidate_trusted: false,
            manifest_isolation_is_container: false,
            header_profile_is_container: false,
            keys_not_visible: false,
            tool_matches_pin: false,
            trunk_defined_origin: false,
            performs_the_cas: false,
        }
    }
}

/// GR §5.8's record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Automerge {
    /// "`policy.rules.c_m4 == \"on\"`".
    pub requested: bool,
    /// Five entries, `id` ascending — the array index **is** the id.
    pub preconditions: [PreconditionStatus; 5],
    /// "`requested` **and** every precondition's `status` is `\"met\"` or
    /// `\"exempt\"`".
    pub effective: bool,
}

impl Automerge {
    /// The first precondition that is `unmet`, which is what a review's
    /// `reason=` must name.
    ///
    /// GR §5.8, §9.13: the `reason=` "lives in the review line only; it is
    /// **not** a report member" — so this returns an id for a human to write
    /// about, and nothing here is serialized.
    pub fn first_unmet(&self) -> Option<usize> {
        self.preconditions
            .iter()
            .position(|s| *s == PreconditionStatus::Unmet)
    }
}

/// Compute the record.
///
/// GR §5.8 and PB §7.4 rule 5, precondition by precondition. Precondition 3 is
/// "structurally `\"met\"` in v1: there is no deferred mode (PB §6.3 G10)" —
/// and GR §5.8 states why the ordering is not a bug: "G10 runs at PB §5.4
/// step 5, after the report is hashed into the envelope at step 4 … it is
/// stated because an implementer who notices the ordering will otherwise 'fix'
/// it by moving the hash after step 5, which breaks the envelope."
pub fn evaluate(requested: bool, shape: LandingShape, obs: &Observations) -> Automerge {
    // GR §5.8's single grant: "the **tombstone**: all five `\"exempt\"`,
    // `profile: \"n/a\"`. A tombstone under `C-M4: on` therefore records
    // `effective: true`, and that is not a bug."
    //
    // "**A reseal is exempt from nothing.**" — so the match is on `Tombstone`
    // alone and a reseal falls through to the computed arm.
    let preconditions = if shape == LandingShape::Tombstone {
        [PreconditionStatus::Exempt; 5]
    } else {
        let met = |yes: bool| {
            if yes {
                PreconditionStatus::Met
            } else {
                PreconditionStatus::Unmet
            }
        };
        [
            met(obs.threat_candidate_trusted),
            met(obs.manifest_isolation_is_container && obs.header_profile_is_container),
            // "**three conjuncts**" — and one status. RF §8.4: "**One wire,
            // however many conjuncts failed**".
            met(obs.keys_not_visible && obs.tool_matches_pin && obs.trunk_defined_origin),
            PreconditionStatus::Met,
            met(obs.performs_the_cas),
        ]
    };

    Automerge {
        requested,
        preconditions,
        effective: requested && preconditions.iter().all(|s| s.permits_effective()),
    }
}

/// PB §5.2 bullet 9 and PB §7.4 rule 5's wire.
///
/// "Either missing → a `class=tripwire` wire to `landing-review`: `G11`
/// (`C-M4`) where the constitution says off, and `G11` naming the precondition
/// where the run computed it off — **one gate, two reasons**, distinguished by
/// `reason=`."
///
/// GR §5.8 fixes the consequence of getting the cardinality wrong: "An
/// implementation that emits two `G11` entries produces a wire array — and
/// therefore a `wires=` line and an `envelope=` — that no conforming
/// implementation reproduces." The `WireSet`'s `(gate, path)` key would collapse
/// them anyway; returning at most one is saying so.
///
/// PB §11 and GR §6.1: the wire is **never spelled `G1`** — "a `G1` wire is a
/// finding that named tests did not pass, and the two must never share a token
/// a reviewer signs over" — and it is always `advisory`, "the rule-5 `G11`
/// precondition wire is not a finding about G11".
pub fn rule_5_wire(record: &Automerge, shape: LandingShape) -> Option<Wire> {
    // RF §8.6 and PB §11's *Landings that run no suite*: a tombstone "is exempt
    // from §7.4 rule 5 entirely".
    if shape == LandingShape::Tombstone {
        return None;
    }
    if record.effective {
        return None;
    }
    Some(Wire::pathless(
        Gate::G11,
        // PB §5.5: "`class=tripwire`, not protected — no floor path is touched."
        // PB §7.4 rule 5 argues it from the threat model: "`C-A3: hostile` names
        // the *coding agent* as the adversary, while reviewer ≠ signer is a
        // control against a malicious human insider."
        WireClass::Tripwire,
        WireKind::Advisory,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_met() -> Observations {
        Observations {
            threat_candidate_trusted: true,
            manifest_isolation_is_container: true,
            header_profile_is_container: true,
            keys_not_visible: true,
            tool_matches_pin: true,
            trunk_defined_origin: true,
            performs_the_cas: true,
        }
    }

    /// PB §11: "Under the shipped defaults `C-A3: hostile` and `C-M4: off`,
    /// precondition 0 fails on every run that tests anything, so the `G11`
    /// precondition wire is present in every such set, in every lane."
    #[test]
    fn under_the_shipped_defaults_the_rule_5_wire_is_on_every_landing_that_tests() {
        for shape in [
            LandingShape::GatedLand,
            LandingShape::Quick,
            LandingShape::Reseal,
        ] {
            let record = evaluate(false, shape, &Observations::default());
            assert!(!record.effective);
            assert_eq!(record.preconditions[0], PreconditionStatus::Unmet);
            let wire = rule_5_wire(&record, shape).expect("present in every such set");
            assert_eq!(wire.token(), "G11");
            assert_eq!(wire.class, WireClass::Tripwire);
            assert_eq!(wire.kind, WireKind::Advisory);
        }
    }

    /// PB §11, verbatim: "It is never spelled `G1`: a `G1` wire is a finding
    /// that named tests did not pass, and the two must never share a token a
    /// reviewer signs over."
    #[test]
    fn the_rule_5_advisory_is_never_spelled_g1() {
        let record = evaluate(true, LandingShape::GatedLand, &Observations::default());
        let wire = rule_5_wire(&record, LandingShape::GatedLand).unwrap();
        assert_eq!(wire.gate, Gate::G11);
        assert_ne!(wire.gate, Gate::G1);
    }

    /// RF §8.4 and GR §5.8: "**One wire, however many conjuncts failed**."
    #[test]
    fn several_failed_preconditions_still_produce_one_wire() {
        let mut set = crate::wire::WireSet::new();
        let record = evaluate(true, LandingShape::GatedLand, &Observations::default());
        assert_eq!(
            record.preconditions,
            [
                PreconditionStatus::Unmet,
                PreconditionStatus::Unmet,
                PreconditionStatus::Unmet,
                PreconditionStatus::Met,
                PreconditionStatus::Unmet,
            ]
        );
        set.extend(rule_5_wire(&record, LandingShape::GatedLand));
        assert_eq!(set.len(), 1);
        assert_eq!(record.first_unmet(), Some(0));
    }

    /// PB §7.4 rule 5: under `hostile` "`C-M4` can **never** evaluate `on`,
    /// whatever the rest say".
    #[test]
    fn precondition_0_alone_defeats_every_other_one() {
        let mut obs = all_met();
        obs.threat_candidate_trusted = false;
        let record = evaluate(true, LandingShape::GatedLand, &obs);
        assert!(!record.effective);
        assert_eq!(record.first_unmet(), Some(0));
    }

    #[test]
    fn everything_met_and_requested_is_effective_and_raises_nothing() {
        let record = evaluate(true, LandingShape::GatedLand, &all_met());
        assert!(record.effective);
        assert!(rule_5_wire(&record, LandingShape::GatedLand).is_none());
    }

    /// GR §5.8: "`C-M4 == off` and 'a precondition is unmet' are two reasons
    /// that produce **one** `(G11, pathless)` entry."
    #[test]
    fn c_m4_off_with_every_precondition_met_still_raises_the_wire() {
        let record = evaluate(false, LandingShape::GatedLand, &all_met());
        assert!(!record.effective);
        assert_eq!(record.first_unmet(), None);
        assert!(rule_5_wire(&record, LandingShape::GatedLand).is_some());
    }

    /// GR §5.8: "the single grant is the **tombstone**: all five `\"exempt\"`
    /// … A tombstone under `C-M4: on` therefore records `effective: true`, and
    /// that is not a bug."
    #[test]
    fn a_tombstone_is_exempt_from_all_five_and_raises_no_wire() {
        let record = evaluate(true, LandingShape::Tombstone, &Observations::default());
        assert_eq!(record.preconditions, [PreconditionStatus::Exempt; 5]);
        assert!(record.effective);
        assert!(rule_5_wire(&record, LandingShape::Tombstone).is_none());
    }

    /// GR §5.8, PB §7.4 rule 5, RF §8.6: "**A reseal is exempt from
    /// nothing.**" It records all five as computed; under the shipped
    /// `C-A3: hostile`, precondition 0 is `"unmet"`.
    #[test]
    fn a_reseal_is_exempt_from_nothing() {
        let record = evaluate(true, LandingShape::Reseal, &Observations::default());
        assert!(!record.preconditions.contains(&PreconditionStatus::Exempt));
        assert_eq!(record.preconditions[0], PreconditionStatus::Unmet);
    }

    /// GR §5.8, §9.22 and RF §8.4: "`keys_visible=true` is a **legal** header
    /// value producing a legal report: it does not refuse ingestion;
    /// `preconditions[2].status` is `\"unmet\"`."
    #[test]
    fn keys_visible_true_is_legal_and_costs_precondition_2_alone() {
        let mut obs = all_met();
        obs.keys_not_visible = false;
        let record = evaluate(true, LandingShape::GatedLand, &obs);
        assert_eq!(record.preconditions[2], PreconditionStatus::Unmet);
        assert_eq!(record.preconditions[0], PreconditionStatus::Met);
        assert_eq!(record.preconditions[1], PreconditionStatus::Met);
        assert!(!record.effective);
    }

    /// RF §8.1 and §8.4: a file whose trunk-defined origin cannot be
    /// demonstrated "**is ingested**", and "the whole of the consequence is
    /// that **auto-merge precondition 2 fails**".
    #[test]
    fn an_unestablished_trunk_defined_origin_costs_precondition_2_and_nothing_else() {
        let mut obs = all_met();
        obs.trunk_defined_origin = false;
        let record = evaluate(true, LandingShape::GatedLand, &obs);
        assert_eq!(record.first_unmet(), Some(2));
    }

    /// GR §5.8: precondition 3 is "structurally `\"met\"` in v1: there is no
    /// deferred mode".
    #[test]
    fn precondition_3_is_structurally_met() {
        let record = evaluate(true, LandingShape::GatedLand, &Observations::default());
        assert_eq!(record.preconditions[3], PreconditionStatus::Met);
    }

    /// RF §8.4: "A disagreement — including a manifest claiming `container`
    /// against a header reporting `none` — fails the precondition and no more."
    #[test]
    fn precondition_1_needs_both_sides_to_agree() {
        let mut obs = all_met();
        obs.header_profile_is_container = false;
        assert_eq!(
            evaluate(true, LandingShape::GatedLand, &obs).preconditions[1],
            PreconditionStatus::Unmet
        );
    }
}
