//! The pytest adapter — RF §6.7's mapping, IR §11.1's argv, IR §11.2's floor.
//!
//! The argv and the id functions are `spine-resolve`'s and are not restated:
//! `Runner::Pytest` carries the ratified `pytest` and `pytest --collect-only`,
//! and `spine_resolve::ids` carries `fn_of` and `path_prefix`. IR §11.1 is
//! emphatic that nothing else may run — "**No adapter runs a command this
//! section has not already ratified.**" In particular `-q` is **not** ratified:
//! IR §11.7 reproduces `pytest --collect-only -q` as *evidence about pytest's
//! behaviour*, because that is the only form that prints one nodeid per line,
//! and a parse of that output could not carry RF §6.6's four signals anyway.

use spine_collect::outcome::Outcome;

use crate::transport::{Item, Phase, PhaseOutcome, Report};

/// RF §6.7's table, read over the phases and the marker.
///
/// > "**Precedence is phases plus polarity, never the transport's own outcome
/// > word.** Where a runner's own summary word disagrees with what the phases
/// > and the expected-failure marker say, the phases and the marker win. A
/// > strict expected-failure that passes is `xpass` here even where the runner
/// > itself reports it as `failed`."
///
/// Which is why this takes an [`Item`] and not a word: the transport carries no
/// summary word for it to be tempted by.
pub fn outcome_of(item: &Item) -> Outcome {
    // "collected, then excluded before running → `deselected`", and it is
    // decided before the phases because a deselected item has none.
    if item.deselected {
        return Outcome::Deselected;
    }

    let phase = |which: Phase| {
        item.phases
            .iter()
            .find(|(p, _)| *p == which)
            .map(|(_, o)| *o)
    };

    // "failure or exception in `setup`/`teardown`, or a collection error →
    // `error`", and it outranks the call's own result: an item whose teardown
    // exploded did not pass, whatever `call` said.
    for around in [Phase::Setup, Phase::Teardown] {
        if phase(around) == Some(PhaseOutcome::Failed) {
            return Outcome::Error;
        }
    }

    let call = phase(Phase::Call);
    match (call, item.expected_failure) {
        // "`call` failed or skipped, expected-failure marker set → `xfail`"
        (Some(PhaseOutcome::Failed | PhaseOutcome::Skipped), true) => Outcome::Xfail,
        // "`call` failed, no expected-failure marker → `failed`"
        (Some(PhaseOutcome::Failed), false) => Outcome::Failed,
        // "skipped, no expected-failure marker → `skipped`"
        (Some(PhaseOutcome::Skipped), false) => Outcome::Skipped,
        // "all phases passed, expected-failure marker set → `xpass`"
        (Some(PhaseOutcome::Passed), true) if all_passed(item) => Outcome::Xpass,
        // "all phases passed, no expected-failure marker → `passed`"
        (Some(PhaseOutcome::Passed), false) if all_passed(item) => Outcome::Passed,
        // "any other terminal report → `unknown`"
        _ => Outcome::Unknown,
    }
}

fn all_passed(item: &Item) -> bool {
    !item.phases.is_empty()
        && item
            .phases
            .iter()
            .all(|(_, outcome)| *outcome == PhaseOutcome::Passed)
}

/// IR §11.1: "the `B` floor is the set of ids the runner **collected and
/// selected** on the checkout of `B` — every id it enumerated, less any it
/// reported as *deselected*, and **irrespective of outcome**."
///
/// The last clause is the one an implementation gets wrong: an id trunk itself
/// fails, skips or xfails is still in the floor. Only deselection removes one,
/// because a deselected id "never runs on `B` and never runs on `T`".
pub fn floor_ids(report: &Report) -> Vec<&str> {
    report
        .items
        .iter()
        .filter(|item| !item.deselected)
        .map(|item| item.id.as_str())
        .collect()
}

/// IR §11.2's completeness check, over the count pytest reports for itself.
///
/// > "pytest reports its own collected-and-selected count — `4 tests
/// > collected`, or `3/4 tests collected (1 deselected)`"
///
/// **The numerator, never the denominator.** IR §11.1 defines the floor as
/// "collected **and selected**", so `3/4 tests collected (1 deselected)` is a
/// floor of three and a count of three. A check against the four raises
/// `base-collect-failed` on every repository with a
/// `pytest_collection_modifyitems` hook — which IR §11's own conformance vector
/// requires to land, with a three-id floor.
///
/// `None` where the runner reported no count: RF §7.3 already has a row for a
/// stream that will not parse, and an absent count is not a mismatch.
pub fn collection_is_complete(report: &Report) -> Option<bool> {
    report
        .reported_count
        .map(|count| count == floor_ids(report).len() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, phases: &[(Phase, PhaseOutcome)], xfail: bool) -> Item {
        Item {
            id: id.to_string(),
            phases: phases.to_vec(),
            expected_failure: xfail,
            deselected: false,
        }
    }

    fn deselected(id: &str) -> Item {
        Item {
            id: id.to_string(),
            phases: Vec::new(),
            expected_failure: false,
            deselected: true,
        }
    }

    const SETUP_OK: (Phase, PhaseOutcome) = (Phase::Setup, PhaseOutcome::Passed);
    const TEARDOWN_OK: (Phase, PhaseOutcome) = (Phase::Teardown, PhaseOutcome::Passed);

    fn call(outcome: PhaseOutcome) -> [(Phase, PhaseOutcome); 3] {
        [SETUP_OK, (Phase::Call, outcome), TEARDOWN_OK]
    }

    /// RF §6.7's table, row by row.
    #[test]
    fn the_mapping_is_rf_6_7s_table() {
        use PhaseOutcome::*;
        assert_eq!(
            outcome_of(&item("a", &call(Passed), false)),
            Outcome::Passed
        );
        assert_eq!(outcome_of(&item("a", &call(Passed), true)), Outcome::Xpass);
        assert_eq!(
            outcome_of(&item("a", &call(Failed), false)),
            Outcome::Failed
        );
        assert_eq!(outcome_of(&item("a", &call(Failed), true)), Outcome::Xfail);
        assert_eq!(outcome_of(&item("a", &call(Skipped), true)), Outcome::Xfail);
        assert_eq!(
            outcome_of(&item("a", &call(Skipped), false)),
            Outcome::Skipped
        );
        assert_eq!(outcome_of(&deselected("a")), Outcome::Deselected);

        // "failure or exception in `setup`/`teardown` … → `error`", and it
        // outranks the call: an item whose teardown exploded did not pass.
        let bad_setup = [(Phase::Setup, Failed)];
        assert_eq!(outcome_of(&item("a", &bad_setup, false)), Outcome::Error);
        let bad_teardown = [SETUP_OK, (Phase::Call, Passed), (Phase::Teardown, Failed)];
        assert_eq!(outcome_of(&item("a", &bad_teardown, false)), Outcome::Error);

        // "any other terminal report → `unknown`".
        let odd = [SETUP_OK, (Phase::Call, Other), TEARDOWN_OK];
        assert_eq!(outcome_of(&item("a", &odd, false)), Outcome::Unknown);
        assert_eq!(outcome_of(&item("a", &[], false)), Outcome::Unknown);
    }

    /// RF §6.6: "Precedence is phases plus polarity, never the transport's own
    /// outcome word … A strict expected-failure that passes is `xpass` here
    /// even where the runner itself reports it as `failed`."
    ///
    /// The transport carries no summary word at all, so this is structural —
    /// what the test shows is that the same phases with the marker flipped give
    /// the two different answers, and nothing else could have.
    #[test]
    fn polarity_and_phases_decide_and_nothing_else_is_available() {
        let phases = call(PhaseOutcome::Passed);
        assert_eq!(outcome_of(&item("a", &phases, true)), Outcome::Xpass);
        assert_eq!(outcome_of(&item("a", &phases, false)), Outcome::Passed);
    }

    /// IR §11.1: the floor is "every id it enumerated, less any it reported as
    /// *deselected*, and **irrespective of outcome**".
    #[test]
    fn the_floor_keeps_every_outcome_and_drops_only_deselection() {
        let report = Report {
            items: vec![
                item("t.py::passes", &call(PhaseOutcome::Passed), false),
                item("t.py::fails", &call(PhaseOutcome::Failed), false),
                item("t.py::skips", &call(PhaseOutcome::Skipped), false),
                item("t.py::xfails", &call(PhaseOutcome::Failed), true),
                deselected("t.py::excluded"),
            ],
            ..Default::default()
        };
        assert_eq!(
            floor_ids(&report),
            ["t.py::passes", "t.py::fails", "t.py::skips", "t.py::xfails"],
            "a failing or skipped id trunk owns is still floor"
        );
    }

    /// IR §11.2's count is the **numerator**: `3/4 tests collected (1
    /// deselected)` is a floor of three. Comparing against the four raises
    /// `base-collect-failed` on every repository with a collection hook.
    #[test]
    fn the_completeness_count_is_the_selected_one() {
        let report = Report {
            items: vec![
                item("a", &call(PhaseOutcome::Passed), false),
                item("b", &call(PhaseOutcome::Passed), false),
                item("c", &call(PhaseOutcome::Passed), false),
                deselected("d"),
            ],
            reported_count: Some(3),
            ..Default::default()
        };
        assert_eq!(collection_is_complete(&report), Some(true));

        // The trap: the denominator.
        let denominator = Report {
            reported_count: Some(4),
            ..report.clone()
        };
        assert_eq!(collection_is_complete(&denominator), Some(false));

        // No count reported is not a mismatch.
        let silent = Report {
            reported_count: None,
            ..report
        };
        assert_eq!(collection_is_complete(&silent), None);
    }

    /// IR §11.1 ratifies the argv, and `spine-resolve` owns it. Restating it
    /// here would be a second table to keep identical; what this pins is that
    /// the adapter takes it from there — and that `-q`, which IR §11.7 shows
    /// only as evidence about pytest's output, is not in it.
    #[test]
    fn the_argv_is_the_ratified_one_and_carries_no_selection_argument() {
        use spine_resolve::runner::Runner;
        assert_eq!(Runner::Pytest.token(), "pytest");
        assert_eq!(Runner::Pytest.invocation(), ["pytest"]);
        assert_eq!(
            Runner::Pytest.base_enumeration(),
            ["pytest", "--collect-only"]
        );
        for argv in [
            Runner::Pytest.invocation(),
            Runner::Pytest.base_enumeration(),
        ] {
            assert!(!argv.contains(&"-q"), "-q is not ratified: {argv:?}");
            assert!(!argv.iter().any(|a| a.starts_with('-') && *a == "-k"));
        }
    }
}

// ---------------------------------------------------------------------------
// The adapter
// ---------------------------------------------------------------------------

use spine_collect::collector::{
    BaseEnumeration, BaseId, BaseOutcomeRun, CandidateRun, Checkout, Host, ResultItem,
    RunnerAdapter, Spawn, Spawned,
};
use spine_collect::record::{RunnerToken, Status};
use spine_resolve::runner::Runner;

/// The environment the child needs to reach the transport's pipe.
///
/// RF §6.6: the stream "is read over a pipe the collector holds, it is **not
/// supplied by the candidate's environment**". The names are the collector's
/// and the values are the host's; a candidate that sets them is overwritten,
/// which is the point.
///
/// `PYTEST_PLUGINS` rather than `-p`: IR §11.1 ratifies the argv and "**No
/// adapter runs a command this section has not already ratified**", so the
/// plugin arrives by environment. `keys::Probe::spawned_environment_is_a_subset`
/// admits it — it is not key material and does not move `HOME`.
pub const PLUGIN_VARIABLE: &str = "PYTEST_PLUGINS";
pub const PLUGIN_MODULE: &str = "spine_pytest_transport";
/// The descriptor the plugin writes the stream to.
pub const CHANNEL_VARIABLE: &str = "SPINE_TRANSPORT_FD";

/// RF §6.7: `path` is "the component before the first `::`, resolved to
/// repo-relative POSIX form".
///
/// **The tree-entry half is not done here.** `spine_resolve::ids::id_to_path`
/// takes an `exists` predicate and answers the empty string "where no tree
/// entry matches the runner's reported path" — which RF §8.5 makes meaningful,
/// since an empty `path` is a **bare** `G1` wire rather than `G1:`. The adapter
/// holds no tree: it reads a stream. The predicate belongs to whatever assembles
/// records against `paths(T)`, and until that exists this is the prefix alone,
/// which is the same answer for every id whose file is in the tree — every id a
/// runner actually collected.
fn path_of(id: &str) -> String {
    spine_resolve::ids::path_prefix(Runner::Pytest, id)
        .unwrap_or_default()
        .to_string()
}

/// A pytest adapter over a [`Host`]'s spawn.
#[derive(Debug)]
pub struct Pytest {
    token: RunnerToken,
    /// IR §11.1's floor, kept from the enumeration so the outcome run and the
    /// `T` run can be read against it.
    floor: Vec<BaseId>,
}

impl Default for Pytest {
    fn default() -> Self {
        Self::new()
    }
}

impl Pytest {
    pub fn new() -> Self {
        Pytest {
            token: RunnerToken::new(Runner::Pytest.token()).expect("a ratified token"),
            floor: Vec::new(),
        }
    }

    fn env() -> Vec<(&'static str, String)> {
        vec![
            (PLUGIN_VARIABLE, PLUGIN_MODULE.to_string()),
            // The host replaces this with the descriptor it holds the read end
            // of; the adapter names the variable and never the number.
            (CHANNEL_VARIABLE, String::new()),
        ]
    }

    fn run(host: &mut dyn Host, argv: &[&str], checkout: Checkout, timeout: u64) -> Spawned {
        host.spawn(&Spawn {
            argv,
            checkout,
            env: &Self::env(),
            timeout_secs: timeout,
        })
    }
}

impl RunnerAdapter for Pytest {
    fn token(&self) -> &RunnerToken {
        &self.token
    }

    /// IR §11.2: "**`B` enumeration: `pytest --collect-only`**, run at the
    /// repository root on the checkout of `B`, through the same transport the
    /// `T` invocation uses."
    ///
    /// Every failure here is `BaseEnumeration::Failed`, which RF §7.3 ranks as
    /// `base-collect-failed` above every `T`-run row — because "A full run can
    /// die part-way … and report fewer ids than it collected, which is a floor
    /// smaller than `B`'s real one". A short floor is the one failure that
    /// weakens the gate silently.
    fn enumerate_base(&mut self, host: &mut dyn Host, timeout_secs: u64) -> BaseEnumeration {
        let argv = Runner::Pytest.base_enumeration();
        let Spawned::Stream { bytes, .. } = Self::run(host, argv, Checkout::Base, timeout_secs)
        else {
            return BaseEnumeration::Failed;
        };
        let Ok(report) = crate::transport::parse_stream(&bytes) else {
            return BaseEnumeration::Failed;
        };
        // IR §11.2's completeness check, against the **selected** count.
        if collection_is_complete(&report) == Some(false) {
            return BaseEnumeration::Failed;
        }
        self.floor = floor_ids(&report)
            .into_iter()
            .map(|id| BaseId {
                id: id.to_string(),
                path: path_of(id),
            })
            .collect();
        BaseEnumeration::Collected(self.floor.clone())
    }

    /// IR §11.1's second `B` invocation, "`pytest`, the `T` invocation run
    /// against the checkout of `B`" — byte for byte the same argv.
    ///
    /// > "**A `B` outcome run that fails is not a `base-collect-failed`.** … a
    /// > `B` outcome run that will not start, dies, is killed at
    /// > `params.timeout` **or emits an unparsable stream** leaves every id it
    /// > did not report a terminal outcome for at `out: "absent"`, contributes
    /// > no status, and moves no byte of the `end` record."
    ///
    /// Which is why [`BaseOutcomeRun`] has no failure variant to return: every
    /// failure is the empty report.
    fn base_outcomes(&mut self, host: &mut dyn Host, timeout_secs: u64) -> BaseOutcomeRun {
        let argv = Runner::Pytest.base_outcome_run();
        let reported = match Self::run(host, argv, Checkout::Base, timeout_secs) {
            Spawned::Stream { bytes, .. } => crate::transport::parse_stream(&bytes)
                .map(|report| {
                    report
                        .items
                        .iter()
                        // A deselected item reported here is *not* a floor
                        // entry's outcome: IR §11.2 drops deselections from the
                        // floor, and folding them back in through the outcome
                        // run would undo that.
                        .filter(|item| !item.deselected)
                        .map(|item| (item.id.clone(), outcome_of(item)))
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        BaseOutcomeRun { reported }
    }

    /// RF §7.1 step 8's `T` run.
    fn run_candidate(&mut self, host: &mut dyn Host, timeout_secs: u64) -> CandidateRun {
        let argv = Runner::Pytest.invocation();
        let (bytes, signalled) = match Self::run(host, argv, Checkout::Candidate, timeout_secs) {
            Spawned::Stream { bytes, signalled } => (bytes, signalled),
            Spawned::SpawnFailed => {
                return CandidateRun {
                    items: Vec::new(),
                    contribution: Status::SpawnFailed,
                };
            }
            Spawned::TimedOut => {
                return CandidateRun {
                    items: Vec::new(),
                    contribution: Status::RunnerTimeout,
                };
            }
        };

        let report = match crate::transport::parse_stream(&bytes) {
            Ok(report) => report,
            Err(_) => {
                return CandidateRun {
                    items: Vec::new(),
                    contribution: Status::StreamInvalid,
                };
            }
        };

        let mut items: Vec<ResultItem> = report
            .items
            .iter()
            .map(|item| ResultItem {
                id: item.id.clone(),
                function: spine_resolve::ids::fn_of(Runner::Pytest, &item.id).to_string(),
                path: path_of(&item.id),
                out: outcome_of(item),
            })
            .collect();

        // RF §6.6: "A collection error that yields no item id is recorded as
        // one `error` record whose `id` and `fn` are the runner's own id for
        // the failing collector — for pytest, the file's nodeid — and whose
        // `path` is that file."
        for failing in &report.collection_errors {
            items.push(ResultItem {
                id: failing.clone(),
                function: failing.clone(),
                path: path_of(failing),
                out: spine_collect::outcome::Outcome::Error,
            });
        }

        // RF §7.3: "`complete` requires **both** that the adapter parsed that
        // runner's terminal session-end event **and** that no member of its
        // process group was terminated by a signal." The exit code is never
        // consulted — "a red suite exits non-zero on every runner that ships".
        let contribution = if signalled {
            Status::RunnerFailed
        } else if !report.session_ended {
            // "The runner terminated abnormally, or its stream ended
            // mid-record."
            Status::RunnerFailed
        } else if items.is_empty() {
            // "The runner started and terminated but emitted no parsable
            // stream event."
            Status::NoOutput
        } else {
            Status::Complete
        };

        CandidateRun {
            items,
            contribution,
        }
    }
}
