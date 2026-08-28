//! One landing's complete wire set, assembled across gates.
//!
//! The per-gate rules are unit-tested beside their algorithms. What this file
//! pins is what only shows up when several gates contribute to one array: the
//! order (PB §11), the aggregation (PB §11), the containment condition, and the
//! two encodings of one path that meet inside a single landing.

use spine_gates::automerge::{self, Observations};
use spine_gates::g14::{DiffEntry, G14Input};
use spine_gates::tripwires::{Calibration, G2SubCheck, g2_wire};
use spine_gates::{
    Gate, GateStatus, LandingShape, Review, ReviewClass, ReviewState, Reviews, Wire, WireClass,
    WireKind, WireSet, review_state,
};

/// GR §8's flagship landing, reduced to its wire set: `INT-042`, team mode,
/// `C-A3: hostile`, `C-M4: on`, "one `class=tripwire` review by `bob` over a
/// `G2` containment finding, with the universal rule-5 `G11` advisory wire
/// present because precondition 0 fails under `hostile`".
///
/// The canonical line appears four times in the corpus (PB §5.5, EV §8.3,
/// GR §8.1, GR §8.2) and reads `wires=G11,G2:src/shared/util.ts`.
#[test]
fn the_flagship_landings_wire_set_is_the_canonical_line() {
    let record = automerge::evaluate(true, LandingShape::GatedLand, &Observations::default());
    assert!(!record.effective, "precondition 0 fails under `hostile`");

    let mut wires = WireSet::new();
    wires.extend(automerge::rule_5_wire(&record, LandingShape::GatedLand));
    wires.insert(g2_wire(
        G2SubCheck::OutsideExpected,
        Some(b"src/shared/util.ts"),
        Calibration::Block,
    ));

    assert_eq!(wires.wires_line(), "wires=G11,G2:src/shared/util.ts"[6..]);
    assert_eq!(review_state(&wires), ReviewState::LandingReview);

    let bob = Reviews::new(vec![
        Review::new(ReviewClass::Tripwire, "SHA256:bob").naming(wires.tokens()),
    ]);
    assert!(bob.contain(&wires));
}

/// PB §11: "A `protected` wire anywhere in the set makes the landing
/// `protected-review` … and that review's signed `wires=` must cover the
/// complete set, not merely the wires of its own class."
///
/// So the G14 review must name the `G11` advisory too, and a review carrying
/// only the floor tokens leaves the report unconsumable.
#[test]
fn a_floor_hit_promotes_the_state_and_the_review_must_still_cover_the_advisory() {
    let record = automerge::evaluate(false, LandingShape::Quick, &Observations::default());
    let mut wires = WireSet::new();
    wires.extend(automerge::rule_5_wire(&record, LandingShape::Quick));

    let g14 = spine_gates::g14::evaluate(
        &G14Input {
            diff: vec![DiffEntry::new(100_644, 100_644, ".spine/ci.sh")],
            ..Default::default()
        },
        &Reviews::default(),
    );
    wires.extend(g14.verdict.wires.ordered());

    assert_eq!(wires.tokens(), ["G11", "G14:.spine/ci.sh"]);
    assert_eq!(review_state(&wires), ReviewState::ProtectedReview);

    let floor_only = Reviews::new(vec![
        Review::new(ReviewClass::Protected, "SHA256:a").naming(vec!["G14:.spine/ci.sh"]),
    ]);
    assert!(!floor_only.contain(&wires));

    let complete = Reviews::new(vec![
        Review::new(ReviewClass::Protected, "SHA256:a")
            .naming(vec!["G11", "G14:.spine/ci.sh"]),
    ]);
    assert!(complete.contain(&wires));
    assert_eq!(
        spine_gates::g14::evaluate(
            &G14Input {
                diff: vec![DiffEntry::new(100_644, 100_644, ".spine/ci.sh")],
                ..Default::default()
            },
            &complete,
        )
        .verdict
        .status,
        GateStatus::Override
    );
}

/// PB §11: "Break-glass is an overlay, not a class in the aggregation
/// ordering … A landing that hits the floor *and* needs a G1/G8 override takes
/// its `class=protected` review with team-mode reviewer separation intact
/// **and** a separately signed `class=break-glass` review — 'two reviews, one
/// state, no contradiction'."
#[test]
fn a_break_glass_review_sits_beside_the_protected_one_and_never_replaces_it() {
    let mut wires = WireSet::new();
    wires.insert(Wire::at(
        Gate::G14,
        "CODEOWNERS",
        WireClass::Protected,
        WireKind::Finding,
    ));
    wires.insert(Wire::at(
        Gate::G1,
        "tests/billing/test_tax.py",
        WireClass::Protected,
        WireKind::Finding,
    ));

    let reviews = Reviews::new(vec![
        Review::new(ReviewClass::Protected, "SHA256:a").naming(wires.tokens()),
        Review::new(ReviewClass::Protected, "SHA256:b").naming(wires.tokens()),
        Review::new(ReviewClass::BreakGlass, "SHA256:a").naming(vec!["G1"]),
    ]);

    // One state, and it is the protected one.
    assert_eq!(review_state(&wires), ReviewState::ProtectedReview);
    assert!(reviews.contain(&wires));
    // The break-glass review reaches G1 and never G14 (PB §7.6, PB §11:
    // "the floor's authorization is a property of the landing, not of the
    // emergency").
    assert!(reviews.break_glass_bypasses(Gate::G1));
    assert!(!reviews.break_glass_bypasses(Gate::G14));
}

/// R2, inside one landing. GR §5.7 stores `esc(path)` in `floor_hits`, the
/// derived G14 wire token is `tok(path)`, and the two differ on `,`, ` ` and
/// `"` — so a landing over such a path carries both spellings and neither is
/// derivable from the other by a second escaping pass.
#[test]
fn one_landing_carries_both_encodings_of_one_path() {
    let path = b"docs/release notes, v2/CODEOWNERS";
    let outcome = spine_gates::g14::evaluate(
        &G14Input {
            diff: vec![DiffEntry::new(0, 100_644, path.to_vec())],
            ..Default::default()
        },
        &Reviews::default(),
    );
    assert_eq!(outcome.floor_hits, ["docs/release notes, v2/CODEOWNERS"]);
    assert_eq!(
        outcome.verdict.wires.tokens(),
        ["G14:docs/release\\x20notes\\x2c\\x20v2/CODEOWNERS"]
    );
    assert!(spine_gates::g14::floor_hits_and_wires_agree(&outcome));
}

/// GR §5.6: the two orders, over one landing's findings. `gates[]` is numeric
/// and `wires[]` is byte-order, and "an implementation that applies one to the
/// other produces a different `report=` over identical findings".
#[test]
fn the_gates_array_and_the_wires_array_sort_by_different_keys() {
    let mut gates: Vec<Gate> = vec![Gate::G14, Gate::G2, Gate::G11, Gate::G1];
    gates.sort();
    assert_eq!(
        gates.iter().map(|g| g.id()).collect::<Vec<_>>(),
        ["G1", "G2", "G11", "G14"]
    );

    let mut wires = WireSet::new();
    for gate in gates {
        wires.insert(Wire::pathless(gate, WireClass::Tripwire, WireKind::Finding));
    }
    assert_eq!(wires.tokens(), ["G1", "G11", "G14", "G2"]);
}

/// GR §5.6.2 and PB §5.4 step 2, over the whole gate set: the four shapes each
/// select a different row, and the tombstone's is normative in the playbook.
#[test]
fn each_landing_shape_selects_its_own_gate_set() {
    let ran = |shape| {
        spine_gates::ALL_GATES
            .into_iter()
            .filter(|g| g.runs_on(shape))
            .map(Gate::id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ran(LandingShape::Tombstone), ["G9", "G13", "G14", "G15"]);
    assert_eq!(
        ran(LandingShape::GatedLand),
        [
            "G1", "G2", "G3", "G4", "G5", "G7", "G8", "G9", "G11", "G12", "G13", "G14", "G15",
            "G16"
        ]
    );
    // The quick lane and a reseal drop G3, G4 and G12 — "a subjectless landing
    // has neither [an in-flight intent nor an approval]".
    let quick = ["G1", "G2", "G5", "G7", "G8", "G9", "G11", "G13", "G14", "G15", "G16"];
    assert_eq!(ran(LandingShape::Quick), quick);
    assert_eq!(ran(LandingShape::Reseal), quick);
}
