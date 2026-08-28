//! RF §6.7's mapping over **real pytest output**, not over bytes I wrote.
//!
//! The two fixtures are captured from pytest 9.1.1 under CPython 3.14.5; see
//! `tests/vectors/README.md` for the project shape and for the two defects
//! capturing them found. A hand-written fixture would have had a `call` phase
//! on the skipped test, because that is what the mapping's table looks like it
//! implies — and pytest does not produce one.
//!
//! To regenerate, in a scratch directory with the plugin on `PYTHONPATH`:
//!
//! ```text
//! PYTHONPATH=<plugin dir> PYTEST_PLUGINS=spine_pytest_transport \
//!   SPINE_TRANSPORT_FD=3 pytest -p no:cacheprovider 3>candidate.jsonl
//! ```

use spine_collect::outcome::Outcome;
use spine_runner::pytest::{collection_is_complete, floor_ids, outcome_of};
use spine_runner::transport::parse_stream;

const CANDIDATE: &[u8] = include_bytes!("vectors/pytest-9.1-candidate.jsonl");
const COLLECT_ONLY: &[u8] = include_bytes!("vectors/pytest-9.1-collect-only.jsonl");

fn outcome(bytes: &[u8], id: &str) -> Outcome {
    let report = parse_stream(bytes).expect("the plugin's own output parses");
    let item = report
        .items
        .iter()
        .find(|i| i.id == id)
        .unwrap_or_else(|| panic!("{id} is not in the vector"));
    outcome_of(item)
}

/// Every row of RF §6.7's table, as pytest actually reports it.
#[test]
fn the_mapping_holds_over_real_pytest_output() {
    for (id, expected) in [
        ("tests/test_all.py::test_passes", Outcome::Passed),
        ("tests/test_all.py::test_fails", Outcome::Failed),
        ("tests/test_all.py::test_xfails", Outcome::Xfail),
        ("tests/test_all.py::test_xpasses", Outcome::Xpass),
        ("tests/test_all.py::test_skips", Outcome::Skipped),
        ("tests/test_all.py::test_param[1]", Outcome::Passed),
        ("tests/test_all.py::test_deselect_me", Outcome::Deselected),
    ] {
        assert_eq!(outcome(CANDIDATE, id), expected, "{id}");
    }
}

/// The defect capturing the vector found: `@pytest.mark.skip` skips **at
/// setup** and produces no `call` phase, so a mapping that read the skip off
/// `call` answered `unknown` for every skipped test in every repository.
#[test]
fn a_marker_skip_has_no_call_phase_at_all() {
    let report = parse_stream(CANDIDATE).unwrap();
    let skipped = report
        .items
        .iter()
        .find(|i| i.id.ends_with("test_skips"))
        .expect("the vector carries one");
    assert!(
        !skipped
            .phases
            .iter()
            .any(|(p, _)| *p == spine_runner::Phase::Call),
        "pytest reports {:?}",
        skipped.phases
    );
    assert_eq!(outcome_of(skipped), Outcome::Skipped);
}

/// IR §11.2's count is the **numerator**. The project deselects one id of
/// eight, so a conforming enumeration reports seven and agrees with its own
/// floor — which is what keeps `base-collect-failed` off every repository with
/// a `pytest_collection_modifyitems` hook.
#[test]
fn the_enumeration_agrees_with_its_own_floor_under_deselection() {
    let report = parse_stream(COLLECT_ONLY).expect("parses");
    assert_eq!(report.items.len(), 8, "eight collected");
    assert_eq!(floor_ids(&report).len(), 7, "seven selected");
    assert_eq!(
        report.reported_count,
        Some(7),
        "the numerator, not the eight"
    );
    assert_eq!(collection_is_complete(&report), Some(true));
    assert!(report.session_ended);
}

/// IR §11.1: the floor is "every id it enumerated, less any it reported as
/// *deselected*, and **irrespective of outcome**" — so the failing and skipped
/// ids trunk owns are in it, and only the deselected one is not.
#[test]
fn the_real_floor_keeps_the_failing_and_skipped_ids() {
    let report = parse_stream(COLLECT_ONLY).unwrap();
    let floor = floor_ids(&report);
    assert!(floor.contains(&"tests/test_all.py::test_fails"));
    assert!(floor.contains(&"tests/test_all.py::test_skips"));
    assert!(floor.contains(&"tests/test_all.py::test_xfails"));
    assert!(!floor.contains(&"tests/test_all.py::test_deselect_me"));
}
