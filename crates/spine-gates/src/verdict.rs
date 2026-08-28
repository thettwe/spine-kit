//! The status domain, the two kinds of finding, and one verdict shape.
//!
//! GR §5.6.1 fixes three values and PB §11 fixes which two seal:
//!
//! - **`pass`** — "the gate ran and produced no *finding*, and no break-glass
//!   review names it. A gate may read `pass` while having raised wires."
//! - **`override`** — "(a) produced at least one finding and every finding is
//!   covered by a signed review whose class admits that wire … or (b) is named
//!   in the `wires=` of a `class=break-glass` review, among the eight gates
//!   PB §7.6 permits it to bypass."
//! - **`fail`** — "the gate ran, produced at least one finding, and at least
//!   one is uncovered."

use crate::gate::Gate;
use crate::review::Reviews;
use crate::wire::{Wire, WireSet};
use core::fmt;

/// A `gates[]` entry's `status`, and a `Spine-Gates` entry's value.
///
/// PB §11's sealed vocabulary is `pass` or `override` only. GR §5.6.1 adds
/// `fail` "for evaluations that do not seal", and it is terminal: "**A report
/// containing any `fail` is a non-landing report.** … A run that would seal a
/// report containing a `fail` refuses: status `report-not-landable`."
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GateStatus {
    Pass,
    Override,
    Fail,
}

impl GateStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            GateStatus::Pass => "pass",
            GateStatus::Override => "override",
            GateStatus::Fail => "fail",
        }
    }

    /// PB §11: "a tombstone lists the four that ran" — and every entry a seal
    /// carries is `pass` or `override`.
    pub fn seals(self) -> bool {
        self != GateStatus::Fail
    }
}

impl fmt::Display for GateStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// MF §4.8.1 and §6.1 define these identically for G13 and G16, and GR §5.6.1
/// generalizes *outright* to the whole report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FindingKind {
    /// "the gate reads `fail` whatever any review names, and a review whose
    /// `wires=` carries its token changes nothing about the status. Limb (b)
    /// still applies where PB §7.6 lists the gate, and only there."
    Outright,
    /// "a `class=protected` wire, dischargeable by a protected review whose
    /// `wires=` contains the token."
    Coverable,
}

/// One finding: its status token and the wire it raises, if any.
///
/// The status token is typed per gate (see [`crate::status`]) so that "every
/// status token the spec fixes gets an enum variant whose `Display` is that
/// exact token" — those tokens are what a run refuses with and what a reviewer
/// reads beside the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding<S> {
    pub status: S,
    pub kind: FindingKind,
    /// `None` where the finding raises no `wires` entry — every outright G16
    /// check 1–8 failure, for instance, refuses the run before a wire could be
    /// read by anyone.
    pub wire: Option<Wire>,
}

impl<S> Finding<S> {
    pub fn outright(status: S) -> Self {
        Finding {
            status,
            kind: FindingKind::Outright,
            wire: None,
        }
    }

    pub fn outright_with_wire(status: S, wire: Wire) -> Self {
        Finding {
            status,
            kind: FindingKind::Outright,
            wire: Some(wire),
        }
    }

    pub fn coverable(status: S, wire: Wire) -> Self {
        Finding {
            status,
            kind: FindingKind::Coverable,
            wire: Some(wire),
        }
    }
}

/// What one gate produced: a status, the wires it contributes to the report's
/// set, and the findings behind them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict<S> {
    pub gate: Gate,
    pub status: GateStatus,
    pub wires: WireSet,
    pub findings: Vec<Finding<S>>,
}

impl<S> Verdict<S> {
    pub fn pass(gate: Gate) -> Self {
        Verdict {
            gate,
            status: GateStatus::Pass,
            wires: WireSet::new(),
            findings: Vec::new(),
        }
    }

    pub fn has_outright(&self) -> bool {
        self.findings
            .iter()
            .any(|f| f.kind == FindingKind::Outright)
    }

    /// The status tokens of every finding, in the order they were raised.
    pub fn statuses(&self) -> Vec<&S> {
        self.findings.iter().map(|f| &f.status).collect()
    }
}

/// Assemble a verdict from a gate's findings.
///
/// The rule is MF §4.8.6's, §5.10's and §6.10's — one rule written three times:
///
/// > **`pass`** — no finding. **`override`** — every coverable finding's token
/// > appears in the union of the `wires=` of the `class=protected` reviews
/// > discharging this landing … and no outright finding fired. **`fail`** — any
/// > outright finding, or any uncovered coverable finding.
///
/// GR §5.6.1's limb (b) is applied on top by [`with_break_glass`], because it
/// reaches a gate that produced no finding at all.
pub fn decide<S>(gate: Gate, findings: Vec<Finding<S>>, reviews: &Reviews) -> Verdict<S> {
    let mut wires = WireSet::new();
    for finding in &findings {
        if let Some(wire) = &finding.wire {
            wires.insert(wire.clone());
        }
    }

    let status = if findings.is_empty() {
        GateStatus::Pass
    } else if findings.iter().any(|f| f.kind == FindingKind::Outright) {
        GateStatus::Fail
    } else if findings.iter().all(|f| {
        // GR §5.6.1 limb (a): "every finding is covered by a signed review
        // **whose class admits that wire**" — the wire's OWN class, not
        // `protected` for everything.
        //
        // This read `reviews.protected_names(...)` until 2026-08-28, and the
        // consequence was not marginal: the corpus's flagship published report
        // is GR §8.2, whose `{"gate": "G2", "status": "override"}` is
        // discharged by bob's **`class=tripwire`** review. Under the narrow
        // reading that landing reads `fail` and refuses — the one worked
        // example the whole document is built around could not land.
        //
        // PB §6's transition table is the other half: it discharges
        // `landing-review` with a tripwire review, so requiring `protected`
        // everywhere makes the tripwire lane unreachable and every tripwire
        // landing an escalation.
        f.wire
            .as_ref()
            .is_some_and(|w| reviews.admits(w.class, &w.token()))
    }) {
        GateStatus::Override
    } else {
        GateStatus::Fail
    };

    Verdict {
        gate,
        status,
        wires,
        findings,
    }
}

/// GR §5.6.1 limb (b), applied after [`decide`].
///
/// "**A break-glass bypass reads `=override` whether or not the gate produced a
/// finding.** PB §7.6: 'The bypassed gates are likewise marked `=override`.' …
/// G9's check reads override → named, never named → override."
///
/// So this raises a `pass` to `override` as well as a `fail`, and it is
/// deliberately not folded into [`decide`]: the wires a break-glass review
/// names are bare gate ids, not tokens, and mixing the two comparisons is how
/// an implementation ends up discharging a `G14:<path>` finding with a
/// `class=break-glass` review (PB §11 forbids it in terms).
pub fn with_break_glass<S>(mut verdict: Verdict<S>, reviews: &Reviews) -> Verdict<S> {
    if reviews.break_glass_bypasses(verdict.gate) {
        verdict.status = GateStatus::Override;
    }
    verdict
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::{Review, ReviewClass};
    use crate::wire::{WireClass, WireKind};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Token(&'static str);

    fn wire(path: &str) -> Wire {
        Wire::at(Gate::G16, path, WireClass::Protected, WireKind::Finding)
    }

    #[test]
    fn no_finding_is_pass() {
        let verdict: Verdict<Token> = decide(Gate::G16, vec![], &Reviews::default());
        assert_eq!(verdict.status, GateStatus::Pass);
        assert!(verdict.wires.is_empty());
    }

    #[test]
    fn a_covered_coverable_finding_is_override() {
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a").naming(vec!["G16:.spine/ci.sh"]),
        ]);
        let verdict = decide(
            Gate::G16,
            vec![Finding::coverable(
                Token("scaffold-blob-mismatch"),
                wire(".spine/ci.sh"),
            )],
            &reviews,
        );
        assert_eq!(verdict.status, GateStatus::Override);
    }

    /// GR §5.6.1: "a review whose `wires=` carries its token changes nothing
    /// about the status." PB §12 records this seam being closed once already —
    /// a review naming `G14:<path>` would otherwise discharge `paths-shrank`.
    #[test]
    fn naming_an_outright_findings_token_does_not_make_it_override() {
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a").naming(vec!["G16:.spine/ci.sh"]),
        ]);
        let verdict = decide(
            Gate::G16,
            vec![Finding::outright_with_wire(
                Token("manifest-changed-without-upgrade"),
                wire(".spine/ci.sh"),
            )],
            &reviews,
        );
        assert_eq!(verdict.status, GateStatus::Fail);
    }

    #[test]
    fn one_uncovered_coverable_finding_fails_the_whole_gate() {
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a").naming(vec!["G16:.spine/ci.sh"]),
        ]);
        let verdict = decide(
            Gate::G16,
            vec![
                Finding::coverable(Token("scaffold-blob-mismatch"), wire(".spine/ci.sh")),
                Finding::coverable(Token("scaffold-path-missing"), wire("AGENTS.md")),
            ],
            &reviews,
        );
        assert_eq!(verdict.status, GateStatus::Fail);
    }

    /// PB §7.6: "The bypassed gates are likewise marked `=override`" — whether
    /// or not the gate produced a finding.
    #[test]
    fn break_glass_raises_a_passing_gate_to_override() {
        let reviews =
            Reviews::new(vec![Review::new(ReviewClass::BreakGlass, "SHA256:a").naming(vec!["G3"])]);
        let verdict: Verdict<Token> = with_break_glass(decide(Gate::G3, vec![], &reviews), &reviews);
        assert_eq!(verdict.status, GateStatus::Override);
    }

    #[test]
    fn break_glass_cannot_raise_an_authority_gate() {
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::BreakGlass, "SHA256:a").naming(vec!["G14"]),
        ]);
        let verdict = with_break_glass(
            decide(
                Gate::G14,
                vec![Finding::outright(Token("paths-shrank"))],
                &reviews,
            ),
            &reviews,
        );
        assert_eq!(verdict.status, GateStatus::Fail);
    }

    #[test]
    fn a_fail_never_seals() {
        assert!(!GateStatus::Fail.seals());
        assert!(GateStatus::Pass.seals());
        assert!(GateStatus::Override.seals());
    }

    /// GR §8.2's flagship published report, as a discharge question.
    ///
    /// Its sealed value is `{"gate": "G2", "status": "override"}`, and the
    /// review discharging it is bob's **`class=tripwire`**. Requiring a
    /// `class=protected` review for every wire made that landing read `fail`
    /// and refuse — the one worked example the whole document is built around
    /// could not land.
    ///
    /// PB §6's transition table is the other half: it discharges
    /// `landing-review` with a tripwire review, so the narrow reading also made
    /// the tripwire lane unreachable and every tripwire landing an escalation.
    #[test]
    fn a_tripwire_wire_is_discharged_by_a_tripwire_review() {
        let wire = Wire::at(Gate::G2, &b"src/shared/util.ts"[..], WireClass::Tripwire, WireKind::Finding);
        let token = wire.token();
        let findings = vec![Finding::coverable((), wire)];

        let tripwire = Reviews::new(vec![
            Review::new(ReviewClass::Tripwire, "SHA256:bob").naming([token.clone()]),
        ]);
        assert_eq!(
            decide(Gate::G2, findings.clone(), &tripwire).status,
            GateStatus::Override,
            "GR §5.6.1 limb (a): a review whose class admits that wire"
        );

        // A protected review is the stronger statement and covers it too —
        // PB §5.2's tiers are ordered.
        let protected = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:bob").naming([token.clone()]),
        ]);
        assert_eq!(
            decide(Gate::G2, findings.clone(), &protected).status,
            GateStatus::Override
        );

        // And a review naming nothing discharges nothing.
        assert_eq!(
            decide(Gate::G2, findings, &Reviews::default()).status,
            GateStatus::Fail
        );
    }

    /// The relation does not run the other way. A `class=protected` wire is
    /// the floor's, and a tripwire review is not the statement PB §7.3 asks
    /// for — "a protected review" — so it must not discharge one.
    #[test]
    fn a_tripwire_review_does_not_discharge_a_protected_wire() {
        let wire = Wire::at(Gate::G14, &b".spine/ci.sh"[..], WireClass::Protected, WireKind::Finding);
        let token = wire.token();
        let findings = vec![Finding::coverable((), wire)];

        let tripwire = Reviews::new(vec![
            Review::new(ReviewClass::Tripwire, "SHA256:bob").naming([token.clone()]),
        ]);
        assert_eq!(
            decide(Gate::G14, findings.clone(), &tripwire).status,
            GateStatus::Fail,
            "a floor change takes a protected review, not a tripwire one"
        );

        let protected = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:bob").naming([token]),
        ]);
        assert_eq!(
            decide(Gate::G14, findings, &protected).status,
            GateStatus::Override
        );
    }
}
