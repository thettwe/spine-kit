//! Every closed token vocabulary the gate report's schema fixes.
//!
//! These are not conveniences. GR §3.2 makes the schema **closed** — "forward
//! compatibility is bought with a version bump, not with tolerance, because a
//! tolerant reader and a strict one compute different digests over the same
//! document and the whole artifact is a digest" — so every domain here is a
//! total enum whose [`core::fmt::Display`] is the exact byte string that
//! reaches a reviewer's signed `wires=` and a seal's `Spine-Gates`.
//!
//! A `String` member with a documented domain would be the same schema with the
//! check moved to a comment.

use core::fmt;

/// Parse a token out of a closed domain, or nothing.
///
/// Every domain in this module implements this the same way, and none accepts
/// an alias, a case variant or surrounding space: GR §5.5's "a reader records
/// what it finds and normalizes nothing" is about the *bytes of a trailer
/// line*, and these are not those — these are schema tokens the serializer
/// writes, so a value that is not exactly one of them is a report to refuse.
macro_rules! closed_domain {
    (
        $(#[$meta:meta])*
        $name:ident { $( $(#[$vmeta:meta])* $variant:ident => $token:literal ),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $( $(#[$vmeta])* $variant ),+
        }

        impl $name {
            /// The exact token this value serializes to.
            pub const fn token(self) -> &'static str {
                match self { $( $name::$variant => $token ),+ }
            }

            /// Every value, in the order this enum declares them. Used by the
            /// tests that pin a domain's size, so widening a domain without
            /// touching `report_version` fails a test rather than a landing.
            pub const ALL: &'static [$name] = &[ $( $name::$variant ),+ ];

            pub fn parse(s: &str) -> Option<Self> {
                match s {
                    $( $token => Some($name::$variant), )+
                    _ => None,
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.token())
            }
        }
    };
}

closed_domain! {
    /// GR §5.1 `subject.lane`, PB §11 `Spine-Lane`.
    ///
    /// "Toolkit lifecycle landings are `quick`" (PB §6.7) — there is no third
    /// lane for them, which is why every lifecycle landing inherits the quick
    /// lane's signerless overlay.
    Lane {
        Gated => "gated",
        Quick => "quick",
    }
}

closed_domain! {
    /// GR §5.1 `subject.event`, PB §11 `Spine-Event`, **landing values only**.
    ///
    /// `review` and `approve` are event-commit values and never reach a
    /// report's `subject`: a report describes a landing.
    Event {
        Land => "land",
        Withdraw => "withdraw",
        Reseal => "reseal",
    }
}

closed_domain! {
    /// GR §5.1 `subject.strategy`, `C-M1`, PB §11 `Spine-Strategy`.
    ///
    /// GR §4.2: a reseal and a tombstone "record the repository's `C-M1` value;
    /// their tree rule is their parent's either way", and §4.2 "does not read
    /// this member" — recomputability is a property of the objects, not of this
    /// token. [`crate::Report::needs_head`] is where that distinction lives.
    Strategy {
        Merge => "merge",
        Squash => "squash",
    }
}

closed_domain! {
    /// GR §5's `mode` member. **Three-valued**, where `C-A1` is two-valued.
    ///
    /// "Equals `policy.rules.c_a1` except on a recovery-sealed landing (PB
    /// §7.5), where it is `recovery`." The two are separate types here for that
    /// reason: a single enum would let a recovery seal be written into `c_a1`,
    /// which the constitution's grammar cannot express.
    Mode {
        Solo => "solo",
        Team => "team",
        Recovery => "recovery",
    }
}

closed_domain! {
    /// `C-A1`'s own domain (GR §5.4.1). Two-valued; see [`Mode`].
    RuleMode {
        Solo => "solo",
        Team => "team",
    }
}

impl RuleMode {
    /// The `mode` member a landing under this rule records when it is not a
    /// recovery seal.
    pub const fn as_mode(self) -> Mode {
        match self {
            RuleMode::Solo => Mode::Solo,
            RuleMode::Team => Mode::Team,
        }
    }
}

closed_domain! {
    /// GR §5's `threat`, and `C-A3` (`threat.candidate`). One domain, two
    /// members: `threat` "equals `policy.rules.c_a3`" with no exception, so
    /// [`crate::Report`] derives it rather than storing it twice.
    Threat {
        Hostile => "hostile",
        Trusted => "trusted",
    }
}

closed_domain! {
    /// GR §5's `profile` — the seal's `profile=` (PB §7.4 rule 3, PB §11).
    ///
    /// Four values, and the fourth is not the collector's: `spine_collect`'s
    /// `Profile` is the *header's* three-valued domain, and RF §4.2 says "`n/a`
    /// … is never a header value, and a header carrying it is malformed". The
    /// seal's domain is one wider, which is why this is its own enum.
    ///
    /// `"n/a"` is "iff the landing runs no suite, which PB §11 makes the
    /// tombstone and nothing else" — a reseal runs the suite (PB §7.4 rule 5)
    /// and never records it.
    SealProfile {
        Container => "container",
        Uid => "uid",
        None => "none",
        /// The token carries a `/`. It is a schema token, not a path, so no
        /// `esc` applies to it.
        NotApplicable => "n/a",
    }
}

closed_domain! {
    /// `C-M2` (`merge.reverify`), GR §5.4.1.
    Reverify {
        Full => "full",
        Scoped => "scoped",
    }
}

closed_domain! {
    /// `C-M4` (`merge.auto`), GR §5.4.1. `automerge.requested` is derived from
    /// it (GR §5.8) and is not stored.
    AutoMerge {
        On => "on",
        Off => "off",
    }
}

closed_domain! {
    /// GR §5.5 — "the namespace the signature verified under … Roles are
    /// derived from this, never claimed" (PB §7.2).
    Namespace {
        Signoff => "spine-signoff@v1",
        Review => "spine-review@v1",
        Seal => "spine-seal@v1",
    }
}

closed_domain! {
    /// GR §5.6's `gates[].status`, and PB §11's `Spine-Gates` vocabulary plus
    /// the one value this spec adds.
    ///
    /// GR §5.6.1: "PB §11 fixes the sealed vocabulary: a `Spine-Gates` entry is
    /// `pass` or `override` … This spec adds exactly one value, `fail`, for
    /// evaluations that do not seal."
    GateStatus {
        Pass => "pass",
        Override => "override",
        Fail => "fail",
    }
}

impl GateStatus {
    /// GR §5.6.1's status rule, as a total function of the three facts that
    /// decide it. Written once so no gate implements its own reading.
    ///
    /// - **`pass`** — "the gate ran and produced no *finding*, and no
    ///   break-glass review names it. A gate may read `pass` while having
    ///   raised wires."
    /// - **`override`** — "(a) produced at least one finding and every finding
    ///   is covered by a signed review whose class admits that wire, or (b) is
    ///   named in the `wires=` of a `class=break-glass` review, among the eight
    ///   gates PB §7.6 permits it to bypass".
    /// - **`fail`** — "produced at least one finding, and at least one is
    ///   uncovered."
    ///
    /// `break_glass_named` is the *raw* fact — that some `class=break-glass`
    /// review's `wires=` carries this gate's bare id. Limb (b) is filtered here
    /// rather than at the call site, because PB §7.6's list is a property of
    /// the gate ([`crate::Gate::break_glass_bypassable`]) and a caller that
    /// forgot the filter would mark G14 `override` — the one thing PB §11 says
    /// break-glass may never do.
    pub fn derive(
        gate: crate::Gate,
        all_findings_covered: bool,
        had_finding: bool,
        break_glass_named: bool,
    ) -> Self {
        // "A break-glass bypass reads `=override` whether or not the gate
        // produced a finding" (GR §5.6.1, quoting PB §7.6 and PB §6's
        // transition row). Limb (b) is checked first because it does not ask
        // about findings at all.
        if break_glass_named && gate.break_glass_bypassable() {
            return GateStatus::Override;
        }
        if !had_finding {
            return GateStatus::Pass;
        }
        if all_findings_covered {
            GateStatus::Override
        } else {
            GateStatus::Fail
        }
    }
}

closed_domain! {
    /// GR §6.1's `class`. "Required, two-valued and **digest-bearing**, and it
    /// decides the landing's review state through PB §11's aggregation — which
    /// decides who must sign" (GR §6.3).
    WireClass {
        Tripwire => "tripwire",
        Protected => "protected",
    }
}

impl WireClass {
    /// GR §6.1's collapse rule: "the surviving `class` is `protected` if either
    /// was, per PB §11's *`protected` dominates `tripwire`*".
    pub const fn dominant(self, other: Self) -> Self {
        match (self, other) {
            (WireClass::Protected, _) | (_, WireClass::Protected) => WireClass::Protected,
            _ => WireClass::Tripwire,
        }
    }
}

closed_domain! {
    /// GR §6.1's `kind` — "three things PB §6.3 and PB §11 keep separate but
    /// never name together".
    WireKind {
        /// "The gate's own check was not satisfied."
        Finding => "finding",
        /// "The gate raised a wire that is not a finding about itself. **The
        /// only advisory wire in v1 is `G11`**."
        Advisory => "advisory",
        /// "A Drift finding under warn-before-block calibration … Enters the
        /// wire set and any review's `wires=`, does **not** route, does **not**
        /// affect gate status."
        Warn => "warn",
    }
}

impl WireKind {
    /// GR §6.1's collapse rule: "the surviving `kind` is the strongest of
    /// `finding` > `advisory` > `warn`."
    ///
    /// GR §6.1 immediately bounds when this may fire: "the collapse can never
    /// merge an advisory into a finding … in v1 the only advisory-bearing gate
    /// is `G11`, and `G11` raises no findings, so every collapse is between two
    /// entries of the same kind." [`crate::WireSet::from_raised`] refuses the
    /// cross-kind collapse for that reason; this ordering is what it refuses
    /// *against*.
    const fn strength(self) -> u8 {
        match self {
            WireKind::Finding => 2,
            WireKind::Advisory => 1,
            WireKind::Warn => 0,
        }
    }

    pub const fn strongest(self, other: Self) -> Self {
        if other.strength() > self.strength() {
            other
        } else {
            self
        }
    }
}

closed_domain! {
    /// GR §5.8's `automerge.preconditions[].status`.
    ///
    /// GR §5.8 refuses to widen this: "A fourth value — `unverifiable`, or a
    /// split of `unmet` by cause — was considered and **rejected**: it would
    /// put a new token inside a digest-bearing member, force a `report_version`
    /// bump for a distinction no gate reads … **The cause lives in the review's
    /// `reason=`**." Three values, and adding a fourth is a version bump.
    PreconditionStatus {
        Met => "met",
        Unmet => "unmet",
        /// "Used only where the design grants exemption, and the grant is PB
        /// §7.4 rule 5's own, singular: a **tombstone** is exempt from the rule
        /// entirely — all five `exempt`, `profile: n/a`. **A reseal is exempt
        /// from nothing.**"
        Exempt => "exempt",
    }
}

impl PreconditionStatus {
    /// GR §5.8's `effective` conjunct: "every precondition's `status` is `met`
    /// or `exempt`".
    pub const fn satisfied(self) -> bool {
        matches!(self, PreconditionStatus::Met | PreconditionStatus::Exempt)
    }
}

/// The four landing shapes GR §5.6.2's table is indexed by.
///
/// Not a serialized member — it is a function of `subject.lane` and
/// `subject.event`, and GR §5.1 already stores both. It exists so that "which
/// gates ran" is one table lookup rather than a condition each gate re-derives:
/// GR §9.12 records what happens otherwise — "one implementation emits
/// `G12=pass` for a quick-lane landing that has no approval and another omits
/// it, and the two `Spine-Gates` lines — and therefore the two `envelope=`
/// digests — differ over an identical landing."
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LandingShape {
    /// `lane=gated`, `event=land`.
    GatedLand,
    /// `event=withdraw`. PB §5.4 step 2 builds it with parent `B`, tree
    /// identical to `B`'s, and no test run.
    Tombstone,
    /// `lane=quick`, `event=land` — the quick lane and every toolkit lifecycle
    /// landing, which PB §6.7 routes through it.
    QuickLand,
    /// `event=reseal`. "A reseal runs the suite" (PB §7.4 rule 5).
    Reseal,
}

impl LandingShape {
    /// The shape a `(lane, event)` pair names.
    ///
    /// `event` dominates: a tombstone and a reseal are what they are whatever
    /// lane carries them, and PB §5.5 puts a reseal on the quick lane.
    pub const fn of(lane: Lane, event: Event) -> Self {
        match event {
            Event::Withdraw => LandingShape::Tombstone,
            Event::Reseal => LandingShape::Reseal,
            Event::Land => match lane {
                Lane::Gated => LandingShape::GatedLand,
                Lane::Quick => LandingShape::QuickLand,
            },
        }
    }

    /// GR §5.6.2 / PB §11: the tombstone is the one landing that runs no suite,
    /// and therefore the one that records `profile: "n/a"` and all five
    /// preconditions `"exempt"`.
    ///
    /// PB §7.4 rule 5 names the earlier reading as the defect it corrects: a
    /// reseal "does run the suite, does ingest a result file, and seals the
    /// real `profile=` that file reports — never `n/a`".
    pub const fn runs_suite(self) -> bool {
        !matches!(self, LandingShape::Tombstone)
    }

    /// GR §5.8: rule 5 "applies to every landing but a tombstone", and the
    /// rule-5 `G11` advisory attaches to every landing it applies to.
    pub const fn automerge_rule_applies(self) -> bool {
        !matches!(self, LandingShape::Tombstone)
    }

    /// GR §5.6.1's reseal row: on a `Spine-Event: reseal` landing, a G1 or G8
    /// finding is **not** outright — the reseal's own `class=protected` review
    /// naming the finding's token admits it (PB §5.5).
    ///
    /// "Without it a reseal can permanently block trunk": break-glass is
    /// unavailable to a reseal, and G9 refuses every landing above an orphan
    /// until the reseal lands, so reading G1's and G8's outright rows over a
    /// reseal "produces a trunk nobody can ever land on again, from one hand
    /// commit."
    pub const fn suspends_outright(self, gate: crate::Gate) -> bool {
        matches!(self, LandingShape::Reseal) && matches!(gate, crate::Gate::G1 | crate::Gate::G8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gate;

    #[test]
    fn every_token_round_trips_through_parse() {
        macro_rules! check {
            ($t:ty) => {
                for v in <$t>::ALL {
                    assert_eq!(<$t>::parse(v.token()), Some(*v), "{}", v.token());
                }
            };
        }
        check!(Lane);
        check!(Event);
        check!(Strategy);
        check!(Mode);
        check!(RuleMode);
        check!(Threat);
        check!(SealProfile);
        check!(Reverify);
        check!(AutoMerge);
        check!(Namespace);
        check!(GateStatus);
        check!(WireClass);
        check!(WireKind);
        check!(PreconditionStatus);
    }

    /// GR §5's member table and PB §11's seal grammar. A fifth value is a
    /// `report_version` bump, so the size is pinned rather than assumed.
    #[test]
    fn the_seal_profile_domain_has_exactly_four_values() {
        assert_eq!(SealProfile::ALL.len(), 4);
        assert_eq!(SealProfile::NotApplicable.token(), "n/a");
    }

    /// GR §5.8: a fourth precondition status "was considered and **rejected**".
    #[test]
    fn the_precondition_status_domain_has_exactly_three_values() {
        assert_eq!(PreconditionStatus::ALL.len(), 3);
        assert!(PreconditionStatus::parse("unverifiable").is_none());
    }

    /// RF §4.2: `n/a` "is never a header value, and a header carrying it is
    /// malformed". The collector's domain is three-valued; the seal's is four.
    #[test]
    fn n_a_is_a_seal_value_and_never_a_header_value() {
        assert!(spine_collect::Profile::parse("n/a").is_none());
        assert_eq!(SealProfile::parse("n/a"), Some(SealProfile::NotApplicable));
    }

    /// GR §5.6.1: "A break-glass bypass reads `=override` whether or not the
    /// gate produced a finding."
    #[test]
    fn break_glass_marks_a_silent_gate_override() {
        assert_eq!(
            GateStatus::derive(Gate::G3, true, false, true),
            GateStatus::Override
        );
    }

    /// PB §11: break-glass "never relaxes who must sign" and is "never
    /// Authority". A break-glass review naming G14 leaves G14 exactly where the
    /// findings put it.
    #[test]
    fn break_glass_cannot_override_an_authority_gate() {
        assert_eq!(
            GateStatus::derive(Gate::G14, false, true, true),
            GateStatus::Fail
        );
        assert_eq!(
            GateStatus::derive(Gate::G14, true, false, true),
            GateStatus::Pass
        );
    }

    /// GR §5.6.1: "A gate may read `pass` while having raised wires" — the
    /// rule-5 `G11` advisory is the v1 instance.
    #[test]
    fn a_gate_that_raised_only_an_advisory_reads_pass() {
        assert_eq!(
            GateStatus::derive(Gate::G11, true, false, false),
            GateStatus::Pass
        );
    }

    #[test]
    fn protected_dominates_tripwire_in_both_orders() {
        assert_eq!(
            WireClass::Tripwire.dominant(WireClass::Protected),
            WireClass::Protected
        );
        assert_eq!(
            WireClass::Protected.dominant(WireClass::Tripwire),
            WireClass::Protected
        );
        assert_eq!(
            WireClass::Tripwire.dominant(WireClass::Tripwire),
            WireClass::Tripwire
        );
    }

    #[test]
    fn kind_strength_orders_finding_above_advisory_above_warn() {
        assert_eq!(
            WireKind::Warn.strongest(WireKind::Advisory),
            WireKind::Advisory
        );
        assert_eq!(
            WireKind::Advisory.strongest(WireKind::Finding),
            WireKind::Finding
        );
        assert_eq!(
            WireKind::Warn.strongest(WireKind::Finding),
            WireKind::Finding
        );
    }

    /// PB §7.4 rule 5 as of v0.18, quoted by GR §5.8: a reseal "does run the
    /// suite … never `n/a`". The tombstone is the only shape that does not.
    #[test]
    fn only_a_tombstone_runs_no_suite() {
        for shape in [
            LandingShape::GatedLand,
            LandingShape::QuickLand,
            LandingShape::Reseal,
        ] {
            assert!(shape.runs_suite(), "{shape:?}");
            assert!(shape.automerge_rule_applies(), "{shape:?}");
        }
        assert!(!LandingShape::Tombstone.runs_suite());
        assert!(!LandingShape::Tombstone.automerge_rule_applies());
    }

    /// PB §5.5 puts a reseal on the quick lane; `event` still decides the
    /// shape.
    #[test]
    fn the_event_decides_the_shape_not_the_lane() {
        assert_eq!(
            LandingShape::of(Lane::Quick, Event::Reseal),
            LandingShape::Reseal
        );
        assert_eq!(
            LandingShape::of(Lane::Gated, Event::Withdraw),
            LandingShape::Tombstone
        );
        assert_eq!(
            LandingShape::of(Lane::Quick, Event::Land),
            LandingShape::QuickLand
        );
    }

    /// GR §5.6.1's reseal row is exactly G1 and G8 — "G13's, G14's and G16's
    /// outright findings are not in this row and stay outright on every shape,
    /// a reseal included."
    #[test]
    fn a_reseal_suspends_outright_for_g1_and_g8_alone() {
        assert!(LandingShape::Reseal.suspends_outright(Gate::G1));
        assert!(LandingShape::Reseal.suspends_outright(Gate::G8));
        for gate in [Gate::G13, Gate::G14, Gate::G16, Gate::G2] {
            assert!(!LandingShape::Reseal.suspends_outright(gate), "{gate}");
        }
        assert!(!LandingShape::GatedLand.suspends_outright(Gate::G1));
    }
}
