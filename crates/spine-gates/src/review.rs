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
}

impl Review {
    pub fn new(class: ReviewClass, fingerprint: impl Into<String>) -> Self {
        Review {
            class,
            fingerprint: fingerprint.into(),
            self_approved: false,
            wires: Vec::new(),
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

    pub fn names(&self, token: &str) -> bool {
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
        self.of_class(ReviewClass::Protected).any(|r| r.names(token))
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
        Review::new(ReviewClass::Protected, fp).naming(tokens.to_vec())
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
            Review::new(ReviewClass::BreakGlass, "SHA256:a").naming(vec!["G1"]),
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
            Review::new(ReviewClass::BreakGlass, "SHA256:a").naming(vec!["G14", "G13", "G5", "G9"]),
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
