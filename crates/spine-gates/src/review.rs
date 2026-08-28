//! Reviews, containment, and the one review state a landing has.
//!
//! PB §11: "wires accumulate, **`protected` dominates `tripwire`, and a landing
//! has exactly one review state.** A `protected` wire anywhere in the set makes
//! the landing `protected-review` — the only reading under which team mode's
//! reviewer separation cannot be lost to ordering — and that review's signed
//! `wires=` must cover the complete set, not merely the wires of its own class.
//! … There is no first-match rule and no combined state."

use crate::gate::Gate;
use crate::wire::WireSet;
use core::fmt;
use std::collections::BTreeSet;

/// The `class=` of a `Spine-Review`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReviewClass {
    Tripwire,
    Protected,
    /// PB §7.6. **Not a class in the aggregation ordering** — PB §11: it
    /// "records which gates a human chose to bypass; it never relaxes who must
    /// sign", and "a break-glass review sits alongside the review that
    /// discharges the state; it never replaces it".
    BreakGlass,
}

impl ReviewClass {
    pub fn as_str(self) -> &'static str {
        match self {
            ReviewClass::Tripwire => "tripwire",
            ReviewClass::Protected => "protected",
            ReviewClass::BreakGlass => "break-glass",
        }
    }
}

impl fmt::Display for ReviewClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One verified `Spine-Review` bearing on this landing.
///
/// The `wires` field holds **tokens**, already `tok`-encoded, exactly as the
/// signed line carries them: GR §6.2 makes containment "set containment over
/// wire tokens, byte-for-byte", so re-encoding anything here would be a second
/// spelling of the thing being compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
    pub class: ReviewClass,
    /// The fingerprint G13's verification recorded (MF §4.8.1), never the
    /// principal.
    pub fingerprint: String,
    /// GR §5.5's computed member: `true` iff this review's fingerprint equals
    /// the landing's signer key.
    pub self_approved: bool,
    /// The `wires=` value, split on `,`.
    pub wires: Vec<String>,
    /// **What this review is bound to**, and whether it is still current.
    ///
    /// MF §5.10 and §4.8.6 both require each review to carry `head=Hc` and "a
    /// `tree=` equal to the tree under evaluation", and PB §6 spends two rows
    /// on what happens when it does not: "`H ≠ review.head`, or
    /// `merge-tree(review.base, H) ≠ review.tree` — the branch changed → same
    /// state, **review void**."
    ///
    /// Without it every `override` was granted by a review that might have been
    /// signed against a different branch state — a stale review discharging a
    /// current finding, which is the whole thing a binding exists to prevent.
    ///
    /// `None` is unreachable through [`Review::new`], which takes the binding
    /// as an argument precisely so it cannot be forgotten; it remains
    /// representable so that a caller constructing the struct literally still
    /// fails closed rather than defaulting to current.
    pub binding: Option<Binding>,
}

/// A review's binding to the branch state it was signed against (PB §6, MF
/// §5.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// `H == review.head` and `merge-tree(review.base, H) == review.tree`.
    Current,
    /// PB §6: "the branch changed → same state, **review void**."
    ///
    /// Void, not absent: the review exists and is verified, and it discharges
    /// nothing. PB §5.4's retention rules decide whether it is *kept* across a
    /// base move; this says only that it does not cover today's findings.
    Void,
}

impl Review {
    /// The binding is a **constructor argument and not a setter**.
    ///
    /// A builder method can be forgotten, and forgetting this one grants every
    /// `override` to a review signed against a branch state that no longer
    /// exists — PB §6's "the branch changed → same state, **review void**".
    /// Fifty-three construction sites had to be visited to make this change,
    /// and that is the point: each one now says which state it is asserting.
    pub fn new(class: ReviewClass, fingerprint: impl Into<String>, binding: Binding) -> Self {
        Review {
            class,
            fingerprint: fingerprint.into(),
            self_approved: false,
            wires: Vec::new(),
            binding: Some(binding),
        }
    }

    pub fn naming(mut self, tokens: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.wires = tokens.into_iter().map(Into::into).collect();
        self
    }

    pub fn self_approved(mut self, yes: bool) -> Self {
        self.self_approved = yes;
        self
    }

    /// Does this review name `token` **and** still bind?
    ///
    /// The two are one question, deliberately: a caller that could ask them
    /// separately is a caller that can forget the second, and forgetting it
    /// grants every override to a review signed against a branch state that no
    /// longer exists.
    pub fn names(&self, token: &str) -> bool {
        self.binding == Some(Binding::Current) && self.wires.iter().any(|w| w == token)
    }

    /// What the review names, ignoring the binding — for a caller reporting
    /// what a *void* review said, which PB §5.4's retention rules need.
    pub fn names_regardless_of_binding(&self, token: &str) -> bool {
        self.wires.iter().any(|w| w == token)
    }
}

/// The reviews discharging one landing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reviews {
    entries: Vec<Review>,
}

impl Reviews {
    pub fn new(entries: Vec<Review>) -> Self {
        Reviews { entries }
    }

    pub fn all(&self) -> &[Review] {
        &self.entries
    }

    pub fn of_class(&self, class: ReviewClass) -> impl Iterator<Item = &Review> {
        self.entries.iter().filter(move |r| r.class == class)
    }

    /// The union of every discharging review's `wires=`.
    ///
    /// GR §6.2: "PB §6's protected row says 'the union of their `wires=`' for
    /// exactly this reason: a signerless landing carries two `class=protected`
    /// reviews from distinct keys in team mode … and neither need individually
    /// cover the set."
    ///
    /// A `class=break-glass` review is **excluded**: its `wires=` "lists *the
    /// gates bypassed* as bare ids …, not the wire set, and is never used for
    /// containment" (GR §6.2).
    pub fn union_wires(&self) -> BTreeSet<&str> {
        self.entries
            .iter()
            .filter(|r| r.class != ReviewClass::BreakGlass)
            .flat_map(|r| r.wires.iter().map(String::as_str))
            .collect()
    }

    /// PB §11's containment condition: "A gate report whose wire set is not
    /// wholly covered by the review's `wires=` is not consumable."
    ///
    /// GR §6.2: "It includes `warn` and `advisory` wires — every entry of the
    /// array." And GR §5.6.1: outright findings are in it too — "**Outright is
    /// a coverage rule, never a containment rule**". So this reads the whole
    /// set and asks nothing about kind, class or outrightness.
    pub fn contain(&self, set: &WireSet) -> bool {
        let union = self.union_wires();
        set.tokens().iter().all(|t| union.contains(t.as_str()))
    }

    /// Whether a `class=protected` review names this token.
    ///
    /// This is the discharge for every coverable Authority finding (MF §4.8.6,
    /// §5.10, §6.10) and, on a reseal, for G1's and G8's (GR §5.6.1).
    pub fn protected_names(&self, token: &str) -> bool {
        self.of_class(ReviewClass::Protected)
            .any(|r| r.names(token))
    }

    /// GR §5.6.1 limb (a): is this wire "covered by a signed review **whose
    /// class admits that wire**"?
    ///
    /// The admitting relation is not equality and not "protected covers
    /// everything". PB §5.2's tiers are ordered — a `class=protected` review is
    /// the stronger statement and covers a `tripwire` wire, while a tripwire
    /// review does not reach a protected one. So:
    ///
    /// | wire class | admitted by |
    /// |---|---|
    /// | `tripwire`  | a `tripwire` **or** a `protected` review |
    /// | `protected` | a `protected` review only |
    ///
    /// `break-glass` is not a wire class here: GR §5.6.1 gives it limb (b),
    /// which reaches a gate that produced no finding at all and is applied by
    /// [`Verdict::with_break_glass`] rather than through this relation.
    pub fn admits(&self, class: crate::wire::WireClass, token: &str) -> bool {
        use crate::wire::WireClass;
        match class {
            WireClass::Tripwire => self
                .of_class(ReviewClass::Tripwire)
                .chain(self.of_class(ReviewClass::Protected))
                .any(|r| r.names(token)),
            WireClass::Protected => self.protected_names(token),
        }
    }

    /// GR §5.6.1 limb (b): a `class=break-glass` review naming the gate, "among
    /// the eight gates PB §7.6 permits it to bypass".
    ///
    /// A break-glass `wires=` carries **bare ids** (GR §6.2), so this compares
    /// against `gate.id()` and never against a path-bearing token. The gate's
    /// own eligibility is checked here rather than at the caller so that a
    /// review naming `G14` — which PB §7.6 forbids — buys nothing anywhere.
    pub fn break_glass_bypasses(&self, gate: Gate) -> bool {
        gate.break_glass_bypassable()
            && self
                .of_class(ReviewClass::BreakGlass)
                .any(|r| r.names(gate.id()))
    }

    /// Distinct fingerprints among `class=protected` reviews — check 9's datum
    /// (MF §4.8.4).
    pub fn distinct_protected_fingerprints(&self) -> usize {
        self.of_class(ReviewClass::Protected)
            .map(|r| r.fingerprint.as_str())
            .collect::<BTreeSet<_>>()
            .len()
    }
}

/// The landing's single review state, derived from the wire set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    /// No wire at all. DERIVED: PB §6's transition table names `landing-review`
    /// and `protected-review` as the states a *wire* routes to and names no
    /// state for an empty set. Under the shipped defaults (`C-A3: hostile`,
    /// `C-M4: off`) the rule-5 `G11` advisory is in every set of every landing
    /// that tests anything (PB §11), so this variant is reachable only for a
    /// tombstone and for a repository that has turned auto-merge on and met
    /// every precondition.
    NoWire,
    /// PB §6: "wire tripped on `T` (Drift / Freshness / Strength…) **and no
    /// protected-class wire present** → landing-review".
    LandingReview,
    /// PB §11: "A `protected` wire anywhere in the set makes the landing
    /// `protected-review`."
    ProtectedReview,
}

impl ReviewState {
    pub fn as_str(self) -> &'static str {
        match self {
            ReviewState::NoWire => "no-wire",
            ReviewState::LandingReview => "landing-review",
            ReviewState::ProtectedReview => "protected-review",
        }
    }
}

impl fmt::Display for ReviewState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// PB §11's aggregation, in one function. There is no first-match rule: the
/// whole set is read before a state exists.
pub fn review_state(set: &WireSet) -> ReviewState {
    if set.has_protected() {
        // PB §11: "the only reading under which team mode's reviewer separation
        // cannot be lost to ordering."
        ReviewState::ProtectedReview
    } else if set.is_empty() {
        ReviewState::NoWire
    } else {
        // PB §11: a `class=tripwire` `G11` wire alone does not make the landing
        // `protected-review`, "though any genuine protected wire in the same set
        // still does" — the branch above.
        ReviewState::LandingReview
    }
}

/// PB §4/§7.2's mode, which is §4.5's key count and not `C-A1`'s declaration
/// (MF §4.8.5).
pub use spine_manifest::Mode;

/// PB §11's signerless overlay, "evaluated after aggregation".
///
/// "A landing with no signer — every quick-lane landing, every reseal — carries
/// **at least two distinct `class=protected` reviews in team mode**, whatever
/// class the wire set produced, and **one** in solo mode."
///
/// **`≥`, not `=`.** PB §11 calls it "a floor and never an exact count, since a
/// third reviewer signing a contentious reseal is diligence and must not be the
/// thing that refuses the landing", while MF §4.8.4 check 9 writes "holds
/// **two** … and **one** in solo mode". Read as equality MF refuses the
/// three-reviewer reseal PB protects, and `docs/spec/README.md` line 9 gives
/// PB §11 precedence. See `.build-notes/11-gates-queries.md` C3; this is the
/// resolution, and it is the fail-open direction only in the sense that more
/// humans signed.
pub fn signerless_review_count_holds(mode: Mode, reviews: &Reviews) -> bool {
    let required = match mode {
        Mode::Team => 2,
        // "solo mode has exactly one signoff key by definition, so requiring
        // two would make a quick landing, a reseal and a keyring change
        // unlandable in every solo repository" (MF §4.8.4).
        Mode::Solo => 1,
    };
    reviews.distinct_protected_fingerprints() >= required
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{Wire, WireClass, WireKind};

    fn protected(fp: &str, tokens: &[&str]) -> Review {
        Review::new(ReviewClass::Protected, fp, Binding::Current).naming(tokens.to_vec())
    }

    #[test]
    fn a_protected_wire_anywhere_makes_the_landing_protected_review() {
        let mut set = WireSet::new();
        set.insert(Wire::pathless(
            Gate::G11,
            WireClass::Tripwire,
            WireKind::Advisory,
        ));
        assert_eq!(review_state(&set), ReviewState::LandingReview);
        set.insert(Wire::at(
            Gate::G14,
            ".spine/ci.sh",
            WireClass::Protected,
            WireKind::Finding,
        ));
        assert_eq!(review_state(&set), ReviewState::ProtectedReview);
    }

    /// PB §11: the review's `wires=` "must cover the complete set, not merely
    /// the wires of its own class."
    #[test]
    fn containment_is_over_the_complete_set_not_one_class() {
        let mut set = WireSet::new();
        set.insert(Wire::pathless(
            Gate::G11,
            WireClass::Tripwire,
            WireKind::Advisory,
        ));
        set.insert(Wire::at(
            Gate::G14,
            "CODEOWNERS",
            WireClass::Protected,
            WireKind::Finding,
        ));
        let only_its_own = Reviews::new(vec![protected("SHA256:a", &["G14:CODEOWNERS"])]);
        assert!(!only_its_own.contain(&set));
        let complete = Reviews::new(vec![protected("SHA256:a", &["G11", "G14:CODEOWNERS"])]);
        assert!(complete.contain(&set));
    }

    /// GR §6.2: "A review's `wires=` may name tokens absent from the report."
    #[test]
    fn a_review_may_name_more_than_the_report_carries() {
        let mut set = WireSet::new();
        set.insert(Wire::pathless(
            Gate::G11,
            WireClass::Tripwire,
            WireKind::Advisory,
        ));
        let larger = Reviews::new(vec![protected("SHA256:a", &["G11", "G2:src/gone.ts"])]);
        assert!(larger.contain(&set));
    }

    /// GR §6.2: a break-glass review's `wires=` "is never used for containment".
    #[test]
    fn a_break_glass_review_never_contributes_to_containment() {
        let mut set = WireSet::new();
        set.insert(Wire::at(
            Gate::G1,
            "tests/a.py",
            WireClass::Protected,
            WireKind::Finding,
        ));
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::BreakGlass, "SHA256:a", Binding::Current).naming(vec!["G1"]),
        ]);
        assert!(!reviews.contain(&set));
        assert!(reviews.break_glass_bypasses(Gate::G1));
    }

    /// PB §7.6 and PB §11: "never Authority". A break-glass review naming
    /// `G14` buys nothing, and PB §11 says why — "the floor's authorization is
    /// a property of the landing, not of the emergency."
    #[test]
    fn break_glass_naming_an_authority_gate_bypasses_nothing() {
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::BreakGlass, "SHA256:a", Binding::Current)
                .naming(vec!["G14", "G13", "G5", "G9"]),
        ]);
        for gate in [Gate::G14, Gate::G13, Gate::G5, Gate::G9] {
            assert!(!reviews.break_glass_bypasses(gate), "{gate}");
        }
    }

    /// PB §11: "at least two … a floor and never an exact count". C3's
    /// resolution against MF §4.8.4 check 9's literal `two`.
    #[test]
    fn a_third_reviewer_on_a_signerless_landing_is_diligence_not_a_refusal() {
        let three = Reviews::new(vec![
            protected("SHA256:a", &[]),
            protected("SHA256:b", &[]),
            protected("SHA256:c", &[]),
        ]);
        assert!(signerless_review_count_holds(Mode::Team, &three));
        let one = Reviews::new(vec![protected("SHA256:a", &[])]);
        assert!(!signerless_review_count_holds(Mode::Team, &one));
        assert!(signerless_review_count_holds(Mode::Solo, &one));
    }

    /// "two `class=protected` reviews with **distinct fingerprints**"
    /// (MF §4.8.4 check 9).
    #[test]
    fn two_reviews_from_one_key_do_not_satisfy_the_signerless_overlay() {
        let same_key = Reviews::new(vec![protected("SHA256:a", &[]), protected("SHA256:a", &[])]);
        assert!(!signerless_review_count_holds(Mode::Team, &same_key));
    }
}

#[cfg(test)]
mod binding_tests {
    use super::*;
    use crate::gate::Gate;
    use crate::verdict::{Finding, GateStatus, decide};
    use crate::wire::{Wire, WireClass, WireKind};

    /// PB §6: "`H ≠ review.head`, or `merge-tree(review.base, H) ≠
    /// review.tree` — the branch changed → same state, **review void**."
    ///
    /// MF §5.10 and §4.8.6 both require every discharging review to carry
    /// `head=Hc` and a `tree=` equal to the tree under evaluation. Until
    /// 2026-08-28 `Review` could not express either, so **every override was
    /// granted without the binding** and a review signed against a branch state
    /// that no longer exists discharged a current finding.
    #[test]
    fn a_void_review_discharges_nothing() {
        let wire = Wire::at(
            Gate::G2,
            &b"src/a.ts"[..],
            WireClass::Tripwire,
            WireKind::Finding,
        );
        let token = wire.token();
        let findings = vec![Finding::coverable((), wire)];

        let current = Reviews::new(vec![
            Review::new(ReviewClass::Tripwire, "SHA256:bob", Binding::Current)
                .naming([token.clone()]),
        ]);
        assert_eq!(
            decide(Gate::G2, findings.clone(), &current).status,
            GateStatus::Override
        );

        // Same reviewer, same wires, same class — signed against a branch state
        // that has moved.
        let void = Reviews::new(vec![
            Review::new(ReviewClass::Tripwire, "SHA256:bob", Binding::Void).naming([token.clone()]),
        ]);
        assert_eq!(
            decide(Gate::G2, findings, &void).status,
            GateStatus::Fail,
            "the branch changed, so the review is void and covers nothing"
        );

        // It still *said* what it said, which PB §5.4's retention rules need.
        assert!(
            void.all()[0].names_regardless_of_binding(&token),
            "a void review is void, not forgotten"
        );
        assert!(!void.all()[0].names(&token));
    }

    /// A struct literal that forgets the binding fails closed rather than
    /// defaulting to current — `Review::new` takes it as an argument so this
    /// state is unreachable through the constructor at all.
    #[test]
    fn an_unbound_review_fails_closed() {
        let review = Review {
            class: ReviewClass::Protected,
            fingerprint: "SHA256:bob".into(),
            self_approved: false,
            wires: vec!["G14:.spine/ci.sh".into()],
            binding: None,
        };
        assert!(!review.names("G14:.spine/ci.sh"));
    }
}
