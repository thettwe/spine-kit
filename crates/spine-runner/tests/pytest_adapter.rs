//! The pytest adapter driven through a host, from transport bytes to records.
//!
//! The host is a fake because a process is the one thing a test cannot make
//! deterministic; what is real is every decision the adapter makes about the
//! bytes it is handed, which is where RF §6.7's mapping and IR §11.2's floor
//! rules live.

use spine_collect::collector::{BaseEnumeration, Checkout, Host, RunnerAdapter, Spawn, Spawned};
use spine_collect::header::Profile;
use spine_collect::outcome::Outcome;
use spine_collect::record::Status;
use spine_runner::pytest::{CHANNEL_VARIABLE, PLUGIN_MODULE, PLUGIN_VARIABLE, Pytest};

/// A host that hands back a canned stream per argv, and records what it was
/// asked to spawn.
#[derive(Default)]
struct FakeHost {
    asked: Vec<(String, Checkout)>,
    answers: Vec<Spawned>,
    env_seen: Vec<Vec<(String, String)>>,
}

impl Host for FakeHost {
    fn profile(&self) -> Profile {
        Profile::None
    }
    fn checkout(&mut self, _which: Checkout) {}
    fn restore(&mut self, _which: Checkout, _timeout_secs: u64) {}
    fn reap_all(&mut self) {}

    fn spawn(&mut self, spec: &Spawn<'_>) -> Spawned {
        self.asked.push((spec.argv.join(" "), spec.checkout));
        self.env_seen.push(
            spec.env
                .iter()
                .map(|(k, v)| ((*k).to_string(), v.clone()))
                .collect(),
        );
        if self.answers.is_empty() {
            Spawned::SpawnFailed
        } else {
            self.answers.remove(0)
        }
    }
}

fn stream(lines: &[&str]) -> Spawned {
    let mut bytes = String::new();
    for line in lines {
        bytes.push_str(line);
        bytes.push('\n');
    }
    Spawned::Stream {
        bytes: bytes.into_bytes(),
        signalled: false,
    }
}

fn item(id: &str, call: &str, xfail: bool) -> String {
    format!(
        r#"{{"t":"item","id":"{id}","phases":[{{"phase":"setup","outcome":"passed"}},{{"phase":"call","outcome":"{call}"}},{{"phase":"teardown","outcome":"passed"}}],"expected_failure":{xfail},"deselected":false}}"#
    )
}

/// IR §11.1's ratified argv, and IR §11.2's "through the same transport the `T`
/// invocation uses" — so the enumeration and the `T` run carry the same
/// transport environment, and the `B` outcome run repeats the `T` argv "byte
/// for byte".
#[test]
fn the_three_invocations_are_the_ratified_ones_on_the_right_checkouts() {
    let mut host = FakeHost {
        answers: vec![
            stream(&[
                &item("t.py::a", "passed", false),
                r#"{"t":"count","selected":1}"#,
                r#"{"t":"end"}"#,
            ]),
            stream(&[&item("t.py::a", "passed", false), r#"{"t":"end"}"#]),
            stream(&[&item("t.py::a", "passed", false), r#"{"t":"end"}"#]),
        ],
        ..Default::default()
    };
    let mut adapter = Pytest::new();
    adapter.enumerate_base(&mut host, 1800);
    adapter.base_outcomes(&mut host, 1800);
    adapter.run_candidate(&mut host, 1800);

    assert_eq!(
        host.asked,
        [
            ("pytest --collect-only".to_string(), Checkout::Base),
            ("pytest".to_string(), Checkout::Base),
            ("pytest".to_string(), Checkout::Candidate),
        ]
    );
    // RF §6.6: the stream "is not supplied by the candidate's environment" —
    // the collector names the channel on every invocation.
    for env in &host.env_seen {
        assert!(
            env.iter()
                .any(|(k, v)| k == PLUGIN_VARIABLE && v == PLUGIN_MODULE)
        );
        assert!(env.iter().any(|(k, _)| k == CHANNEL_VARIABLE));
    }
}

/// IR §11.1: the floor is "every id it enumerated, less any it reported as
/// *deselected*, and **irrespective of outcome**" — and IR §11.2's completeness
/// count is the **selected** one, so a deselecting repository still lands.
#[test]
fn the_floor_drops_deselections_and_keeps_every_outcome() {
    let mut host = FakeHost {
        answers: vec![stream(&[
            &item("t.py::passes", "passed", false),
            &item("t.py::fails", "failed", false),
            &item("t.py::skips", "skipped", false),
            r#"{"t":"item","id":"t.py::excluded","expected_failure":false,"deselected":true}"#,
            r#"{"t":"count","selected":3}"#,
            r#"{"t":"end"}"#,
        ])],
        ..Default::default()
    };
    let BaseEnumeration::Collected(floor) = Pytest::new().enumerate_base(&mut host, 1800) else {
        panic!("a conforming enumeration collects");
    };
    let ids: Vec<&str> = floor.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(ids, ["t.py::passes", "t.py::fails", "t.py::skips"]);
    assert!(floor.iter().all(|b| b.path == "t.py"));
}

/// IR §11.2: `--collect-only` "either enumerates the whole set or interrupts
/// and is `base-collect-failed`". A count that disagrees with the ids is the
/// truncation the whole two-invocation shape exists to catch.
#[test]
fn a_short_enumeration_is_base_collect_failed() {
    for answer in [
        stream(&[
            &item("t.py::a", "passed", false),
            r#"{"t":"count","selected":9}"#,
            r#"{"t":"end"}"#,
        ]),
        Spawned::Stream {
            bytes: b"not json\n".to_vec(),
            signalled: false,
        },
        Spawned::SpawnFailed,
        Spawned::TimedOut,
    ] {
        let mut host = FakeHost {
            answers: vec![answer],
            ..Default::default()
        };
        assert_eq!(
            Pytest::new().enumerate_base(&mut host, 1800),
            BaseEnumeration::Failed
        );
    }
}

/// IR §11.1: "**A `B` outcome run that fails is not a `base-collect-failed`.**
/// … leaves every id it did not report a terminal outcome for at
/// `out: "absent"`, contributes no status".
#[test]
fn a_failed_outcome_run_reports_nothing_and_refuses_nothing() {
    for answer in [
        Spawned::SpawnFailed,
        Spawned::TimedOut,
        Spawned::Stream {
            bytes: b"garbage\n".to_vec(),
            signalled: false,
        },
    ] {
        let mut host = FakeHost {
            answers: vec![answer],
            ..Default::default()
        };
        assert!(
            Pytest::new()
                .base_outcomes(&mut host, 1800)
                .reported
                .is_empty()
        );
    }
}

/// RF §6.7's mapping, end to end through the `T` run.
#[test]
fn the_candidate_run_maps_every_row_of_rf_6_7() {
    let mut host = FakeHost {
        answers: vec![stream(&[
            &item("t.py::passes", "passed", false),
            &item("t.py::xpasses", "passed", true),
            &item("t.py::fails", "failed", false),
            &item("t.py::xfails", "failed", true),
            &item("t.py::skips", "skipped", false),
            r#"{"t":"collect-error","id":"tests/broken.py"}"#,
            r#"{"t":"end"}"#,
        ])],
        ..Default::default()
    };
    let run = Pytest::new().run_candidate(&mut host, 1800);
    assert_eq!(run.contribution, Status::Complete);

    let by_id = |id: &str| {
        run.items
            .iter()
            .find(|i| i.id == id)
            .unwrap_or_else(|| panic!("{id} is missing"))
    };
    assert_eq!(by_id("t.py::passes").out, Outcome::Passed);
    assert_eq!(by_id("t.py::xpasses").out, Outcome::Xpass);
    assert_eq!(by_id("t.py::fails").out, Outcome::Failed);
    assert_eq!(by_id("t.py::xfails").out, Outcome::Xfail);
    assert_eq!(by_id("t.py::skips").out, Outcome::Skipped);

    // RF §6.6: a collection error "is recorded as one `error` record whose `id`
    // and `fn` are the runner's own id for the failing collector … and whose
    // `path` is that file".
    let broken = by_id("tests/broken.py");
    assert_eq!(broken.out, Outcome::Error);
    assert_eq!(broken.function, "tests/broken.py");
    assert_eq!(broken.path, "tests/broken.py");
}

/// RF §6.7: `fn` is the nodeid with the parametrization suffix removed, and the
/// suffix "exists only if the component's last character is `]`".
#[test]
fn a_parametrized_id_keeps_its_id_and_strips_its_fn() {
    let mut host = FakeHost {
        answers: vec![stream(&[
            &item(
                "tests/billing/test_invoice.py::test_AC1[zero-rate]",
                "passed",
                false,
            ),
            r#"{"t":"end"}"#,
        ])],
        ..Default::default()
    };
    let run = Pytest::new().run_candidate(&mut host, 1800);
    let only = &run.items[0];
    assert_eq!(
        only.id,
        "tests/billing/test_invoice.py::test_AC1[zero-rate]"
    );
    assert_eq!(only.function, "tests/billing/test_invoice.py::test_AC1");
    assert_eq!(only.path, "tests/billing/test_invoice.py");
}

/// RF §7.3: "`complete` requires **both** that the adapter parsed that runner's
/// terminal session-end event **and** that no member of its process group was
/// terminated by a signal … The runner's *exit code* is never the
/// discriminator — a red suite exits non-zero on every runner that ships."
#[test]
fn complete_needs_the_session_end_and_no_signal() {
    // A red suite is still `complete`: no exit code is consulted anywhere.
    let mut host = FakeHost {
        answers: vec![stream(&[
            &item("t.py::a", "failed", false),
            r#"{"t":"end"}"#,
        ])],
        ..Default::default()
    };
    assert_eq!(
        Pytest::new().run_candidate(&mut host, 1800).contribution,
        Status::Complete
    );

    // No terminal event: the stream ended mid-record.
    let mut host = FakeHost {
        answers: vec![stream(&[&item("t.py::a", "passed", false)])],
        ..Default::default()
    };
    assert_eq!(
        Pytest::new().run_candidate(&mut host, 1800).contribution,
        Status::RunnerFailed
    );

    // A signalled process group, even with a terminal event.
    let Spawned::Stream { bytes, .. } =
        stream(&[&item("t.py::a", "passed", false), r#"{"t":"end"}"#])
    else {
        unreachable!()
    };
    let mut host = FakeHost {
        answers: vec![Spawned::Stream {
            bytes,
            signalled: true,
        }],
        ..Default::default()
    };
    assert_eq!(
        Pytest::new().run_candidate(&mut host, 1800).contribution,
        Status::RunnerFailed
    );
}

/// RF §7.3's other rows, taken from the spawn rather than the stream.
#[test]
fn a_spawn_failure_and_a_deadline_are_their_own_rows() {
    let mut host = FakeHost {
        answers: vec![Spawned::SpawnFailed],
        ..Default::default()
    };
    assert_eq!(
        Pytest::new().run_candidate(&mut host, 1800).contribution,
        Status::SpawnFailed
    );

    let mut host = FakeHost {
        answers: vec![Spawned::TimedOut],
        ..Default::default()
    };
    assert_eq!(
        Pytest::new().run_candidate(&mut host, 1800).contribution,
        Status::RunnerTimeout
    );

    // "The runner started and terminated but emitted no parsable stream event."
    let mut host = FakeHost {
        answers: vec![stream(&[r#"{"t":"end"}"#])],
        ..Default::default()
    };
    assert_eq!(
        Pytest::new().run_candidate(&mut host, 1800).contribution,
        Status::NoOutput
    );
}
