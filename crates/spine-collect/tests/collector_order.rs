//! RF §7.1's order, §7.2's reduction and §7.3's fold, over scripted runners.
//!
//! The isolation boundary and the adapters are traits (`spine-isolate` and
//! `import-resolver.md` own them), so what a test can pin here is exactly what
//! this crate is responsible for: *when* each invocation happens, *what*
//! survives reduction, and *which* status the file ends on.

use std::cell::RefCell;
use std::rc::Rc;

use spine_canon::ObjectFormat;
use spine_collect::collector::{
    BaseEnumeration, BaseId, BaseOutcomeRun, CandidateRun, Checkout, Host, Mode, Policy,
    ResultItem, Run, RunnerAdapter, collect,
};
use spine_collect::{BaseOutcome, Outcome, Profile, RunnerToken, Status};
use spine_manifest::Isolation;

const TREE: &str = "3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28";
const BASE: &str = "7b0d4a1f2c3e5d6a8b9c0d1e2f3a4b5c6d7e8f90";
const TOOL: &str = "1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db";

/// Every phase the collector drives, in the order it drove them.
type Journal = Rc<RefCell<Vec<String>>>;

struct FakeHost {
    journal: Journal,
    profile: Profile,
}

impl Host for FakeHost {
    fn profile(&self) -> Profile {
        self.profile
    }

    fn checkout(&mut self, which: Checkout) {
        self.journal
            .borrow_mut()
            .push(format!("checkout:{which:?}"));
    }

    fn restore(&mut self, which: Checkout, _timeout_secs: u64) {
        self.journal.borrow_mut().push(format!("restore:{which:?}"));
    }

    fn reap_all(&mut self) {
        self.journal.borrow_mut().push("reap".into());
    }
}

/// A runner whose three invocations are scripted.
struct FakeRunner {
    journal: Journal,
    token: RunnerToken,
    enumeration: BaseEnumeration,
    outcomes: BaseOutcomeRun,
    run: CandidateRun,
}

impl RunnerAdapter for FakeRunner {
    fn token(&self) -> &RunnerToken {
        &self.token
    }

    fn enumerate_base(&mut self, _host: &mut dyn Host, _timeout_secs: u64) -> BaseEnumeration {
        self.journal
            .borrow_mut()
            .push(format!("B-enumerate:{}", self.token));
        self.enumeration.clone()
    }

    fn base_outcomes(&mut self, _host: &mut dyn Host, _timeout_secs: u64) -> BaseOutcomeRun {
        self.journal
            .borrow_mut()
            .push(format!("B-outcomes:{}", self.token));
        self.outcomes.clone()
    }

    fn run_candidate(&mut self, _host: &mut dyn Host, _timeout_secs: u64) -> CandidateRun {
        self.journal
            .borrow_mut()
            .push(format!("T-run:{}", self.token));
        self.run.clone()
    }
}

fn token(s: &str) -> RunnerToken {
    RunnerToken::new(s).expect("token in grammar")
}

fn policy() -> Policy {
    Policy {
        cli_version: "1.4.0".into(),
        dist_hash: "6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db".into(),
        isolation: Isolation::Container,
        langs: vec!["python".into(), "ts".into()],
        timeout_secs: 1800,
        object_format: ObjectFormat::Sha1,
    }
}

fn run() -> Run {
    Run {
        tree: TREE.into(),
        base: BASE.into(),
        tool: TOOL.into(),
        keys_visible: false,
    }
}

fn base_id(id: &str, path: &str) -> BaseId {
    BaseId {
        id: id.into(),
        path: path.into(),
    }
}

fn item(id: &str, function: &str, path: &str, out: Outcome) -> ResultItem {
    ResultItem {
        id: id.into(),
        function: function.into(),
        path: path.into(),
        out,
    }
}

fn complete(items: Vec<ResultItem>) -> CandidateRun {
    CandidateRun {
        items,
        contribution: Status::Complete,
    }
}

/// RF §7.1 step 7, the multi-runner sharpening: "interleaving — collect on `B`
/// with pytest, run pytest on `T`, then collect on `B` with vitest — would let
/// code the candidate ran under the first runner reach the second runner's
/// collection of the floor, which is exactly the attack rule 3 forbids. Every
/// `B` collection precedes every `T` execution, without exception."
#[test]
fn every_b_invocation_of_every_runner_precedes_every_t_execution() {
    let journal: Journal = Rc::new(RefCell::new(Vec::new()));
    let mut host = FakeHost {
        journal: Rc::clone(&journal),
        profile: Profile::Container,
    };
    let mut adapters: Vec<Box<dyn RunnerAdapter>> = vec![
        Box::new(FakeRunner {
            journal: Rc::clone(&journal),
            token: token("pytest"),
            enumeration: BaseEnumeration::Collected(vec![base_id("a.py::x", "a.py")]),
            outcomes: BaseOutcomeRun {
                reported: vec![("a.py::x".into(), Outcome::Passed)],
            },
            run: complete(vec![item("a.py::x", "a.py::x", "a.py", Outcome::Passed)]),
        }),
        Box::new(FakeRunner {
            journal: Rc::clone(&journal),
            token: token("vitest"),
            enumeration: BaseEnumeration::Collected(vec![base_id("a.test.ts > x", "a.test.ts")]),
            outcomes: BaseOutcomeRun {
                reported: vec![("a.test.ts > x".into(), Outcome::Passed)],
            },
            run: complete(vec![item(
                "a.test.ts > x",
                "a.test.ts > x",
                "a.test.ts",
                Outcome::Passed,
            )]),
        }),
    ];

    let file = collect(&run(), &policy(), Mode::Ci, &mut host, &mut adapters);
    assert_eq!(file.status, Status::Complete);

    let events = journal.borrow().clone();
    // RF §7.1 steps 7-9, in the order the spec numbers them. The restore phase
    // is "**Two per run, never one per runner**, whatever the invocation set
    // holds", and it runs "After each checkout and **before the first runner
    // invocation against it**".
    assert_eq!(
        events,
        vec![
            "checkout:Base",
            "restore:Base",
            "B-enumerate:pytest",
            "B-enumerate:vitest",
            "B-outcomes:pytest",
            "B-outcomes:vitest",
            "checkout:Candidate",
            "restore:Candidate",
            "T-run:pytest",
            "T-run:vitest",
            "reap",
        ]
    );

    // The property stated as the invariant rather than as a transcript: no `B`
    // work of any runner may appear after any `T` work of any runner.
    let last_b = events.iter().rposition(|e| e.starts_with("B-")).unwrap();
    let first_t = events.iter().position(|e| e.starts_with("T-")).unwrap();
    assert!(last_b < first_t);
}

/// RF §7.3, R23: "The runner's *exit code* is never the discriminator — a red
/// suite exits non-zero on every runner that ships, so an exit-code test would
/// make `complete` unreachable for exactly the runs G1 exists to judge."
///
/// The pin is that a suite whose every outcome is `failed` still folds to
/// `complete`, so G1 gets a file it can judge rather than a `runner-failed` one
/// it must refuse.
#[test]
fn a_red_suite_still_folds_to_complete() {
    let journal: Journal = Rc::new(RefCell::new(Vec::new()));
    let mut host = FakeHost {
        journal: Rc::clone(&journal),
        profile: Profile::Container,
    };
    let mut adapters: Vec<Box<dyn RunnerAdapter>> = vec![Box::new(FakeRunner {
        journal: Rc::clone(&journal),
        token: token("pytest"),
        enumeration: BaseEnumeration::Collected(vec![base_id("a.py::x", "a.py")]),
        outcomes: BaseOutcomeRun {
            reported: vec![("a.py::x".into(), Outcome::Passed)],
        },
        run: complete(vec![item("a.py::x", "a.py::x", "a.py", Outcome::Failed)]),
    })];

    let file = collect(&run(), &policy(), Mode::Ci, &mut host, &mut adapters);
    assert_eq!(file.status, Status::Complete);
    assert_eq!(file.results[0].out, Outcome::Failed);
    // The file is judgeable: the status credits outcomes, and the floor is what
    // fails — which is the finding G1 exists to raise.
    assert!(file.status.credits_outcomes());
    assert!(!file.floor_holds());
}

/// RF §7.3: "**Collection on `B` is all-or-nothing across runners.** If *any*
/// invoked runner's collection on `B` fails, the file's `status` is
/// `base-collect-failed`, `ids=0`, and **no `base` and no `result` records are
/// written at all**, from any runner."
#[test]
fn one_runners_failed_b_enumeration_empties_the_whole_body() {
    let journal: Journal = Rc::new(RefCell::new(Vec::new()));
    let mut host = FakeHost {
        journal: Rc::clone(&journal),
        profile: Profile::Container,
    };
    let mut adapters: Vec<Box<dyn RunnerAdapter>> = vec![
        Box::new(FakeRunner {
            journal: Rc::clone(&journal),
            token: token("pytest"),
            enumeration: BaseEnumeration::Collected(vec![base_id("a.py::x", "a.py")]),
            outcomes: BaseOutcomeRun {
                reported: vec![("a.py::x".into(), Outcome::Passed)],
            },
            run: complete(vec![item("a.py::x", "a.py::x", "a.py", Outcome::Passed)]),
        }),
        Box::new(FakeRunner {
            journal: Rc::clone(&journal),
            token: token("vitest"),
            enumeration: BaseEnumeration::Failed,
            outcomes: BaseOutcomeRun::default(),
            run: complete(Vec::new()),
        }),
    ];

    let file = collect(&run(), &policy(), Mode::Ci, &mut host, &mut adapters);
    assert_eq!(file.status, Status::BaseCollectFailed);
    assert_eq!(file.header.ids, 0);
    assert!(file.base.is_empty(), "no base records, from any runner");
    assert!(
        file.results.is_empty(),
        "no result records, from any runner"
    );
    // "`ids=0` here means *no `base` records follow* — the cardinality of `B`'s
    // pair set is unknown and `status` carries that truth."
    let bytes = String::from_utf8(file.to_bytes()).expect("UTF-8");
    assert_eq!(bytes.lines().count(), 2);
    assert!(bytes.contains(" ids=0\n"));
}

/// RF §7.3, the asymmetry: "**A failed `B` outcome run is not all-or-nothing,
/// and is not a status at all.** … a failure of the second … leaves the `base`
/// section whole and gives every id it did not report a terminal outcome for
/// `out: "absent"`. No status contribution is made for it and `end.status` does
/// not move."
#[test]
fn a_failed_b_outcome_run_leaves_the_floor_whole_and_the_status_unmoved() {
    let journal: Journal = Rc::new(RefCell::new(Vec::new()));
    let mut host = FakeHost {
        journal: Rc::clone(&journal),
        profile: Profile::Container,
    };
    let mut adapters: Vec<Box<dyn RunnerAdapter>> = vec![Box::new(FakeRunner {
        journal: Rc::clone(&journal),
        token: token("pytest"),
        enumeration: BaseEnumeration::Collected(vec![
            base_id("a.py::x", "a.py"),
            base_id("a.py::y", "a.py"),
            base_id("a.py::z", "a.py"),
        ]),
        // The outcome run died after the first id. RF §7.1: "the run is killed,
        // every id it had not reached takes `out: "absent"`, and the file's
        // `status` is unaffected."
        outcomes: BaseOutcomeRun {
            reported: vec![("a.py::x".into(), Outcome::Xfail)],
        },
        run: complete(vec![
            item("a.py::x", "a.py::x", "a.py", Outcome::Passed),
            item("a.py::y", "a.py::y", "a.py", Outcome::Passed),
            item("a.py::z", "a.py::z", "a.py", Outcome::Passed),
        ]),
    })];

    let file = collect(&run(), &policy(), Mode::Ci, &mut host, &mut adapters);
    assert_eq!(file.status, Status::Complete, "end.status does not move");
    assert_eq!(file.header.ids, 3, "the base section stays whole");
    assert_eq!(file.base[0].out, BaseOutcome::Reported(Outcome::Xfail));
    assert_eq!(file.base[1].out, BaseOutcome::Absent);
    assert_eq!(file.base[2].out, BaseOutcome::Absent);
    // RF §4.4: `absent` "is not `unknown`" — the carve-out is denied, never
    // granted, by an outcome run that stopped early.
    assert!(!file.base[1].out.exempts_from_findings());
    assert!(file.base[0].out.exempts_from_findings());
}

/// RF §7.2: "When a runner reports an id more than once — a rerun plugin, a
/// repeated phase — **the last terminal outcome that runner reported for that
/// id wins**. The collector transcribes; it does not adjudicate."
///
/// The direction is fail-open on purpose (RF §13 R6): "where the repo's *own*
/// frozen configuration reruns, a `failed` followed by a `passed` is a pass,
/// because the repo chose that configuration under `C-T2`."
#[test]
fn a_repeated_id_reduces_to_the_last_terminal_outcome_that_runner_reported() {
    let journal: Journal = Rc::new(RefCell::new(Vec::new()));
    let mut host = FakeHost {
        journal: Rc::clone(&journal),
        profile: Profile::Container,
    };
    let mut adapters: Vec<Box<dyn RunnerAdapter>> = vec![Box::new(FakeRunner {
        journal: Rc::clone(&journal),
        token: token("pytest"),
        enumeration: BaseEnumeration::Collected(vec![
            base_id("a.py::x", "a.py"),
            // RF §7.2: "`base` pairs are a set: duplicates at collection are
            // reduced to one record, per runner."
            base_id("a.py::x", "a.py"),
        ]),
        outcomes: BaseOutcomeRun {
            reported: vec![("a.py::x".into(), Outcome::Passed)],
        },
        run: complete(vec![
            item("a.py::x", "a.py::x", "a.py", Outcome::Failed),
            item("a.py::x", "a.py::x", "a.py", Outcome::Passed),
        ]),
    })];

    let file = collect(&run(), &policy(), Mode::Ci, &mut host, &mut adapters);
    assert_eq!(file.header.ids, 1, "duplicate base pairs reduce to one");
    assert_eq!(file.results.len(), 1);
    assert_eq!(file.results[0].out, Outcome::Passed, "last wins");
}

/// RF §7.2: "Reduction never crosses runners: two runners reporting one id
/// string produce two records, and neither is the other's rerun."
#[test]
fn reduction_never_crosses_runners() {
    let id = "tests/core/util.test.ts > rounding > half-even";
    let journal: Journal = Rc::new(RefCell::new(Vec::new()));
    let mut host = FakeHost {
        journal: Rc::clone(&journal),
        profile: Profile::Container,
    };
    let mut adapters: Vec<Box<dyn RunnerAdapter>> = vec![
        Box::new(FakeRunner {
            journal: Rc::clone(&journal),
            token: token("vitest"),
            enumeration: BaseEnumeration::Collected(vec![base_id(id, "tests/core/util.test.ts")]),
            outcomes: BaseOutcomeRun {
                reported: vec![(id.into(), Outcome::Passed)],
            },
            run: complete(vec![item(
                id,
                id,
                "tests/core/util.test.ts",
                Outcome::Passed,
            )]),
        }),
        Box::new(FakeRunner {
            journal: Rc::clone(&journal),
            token: token("jest"),
            enumeration: BaseEnumeration::Collected(vec![base_id(id, "tests/core/util.test.ts")]),
            outcomes: BaseOutcomeRun {
                reported: vec![(id.into(), Outcome::Passed)],
            },
            run: complete(vec![item(
                id,
                id,
                "tests/core/util.test.ts",
                Outcome::Failed,
            )]),
        }),
    ];

    let file = collect(&run(), &policy(), Mode::Ci, &mut host, &mut adapters);
    assert_eq!(file.header.ids, 2, "the pair is the identity");
    assert_eq!(file.results.len(), 2);
    // `j` < `v`: RF §4.5 sorts on runner bytes first, which is also what
    // removes the invocation order this test supplied.
    assert_eq!(file.results[0].runner.as_str(), "jest");
    assert_eq!(file.results[0].out, Outcome::Failed);
    assert_eq!(file.results[1].runner.as_str(), "vitest");
    assert_eq!(file.results[1].out, Outcome::Passed);
    // Neither runner's record credits the other's floor entry.
    assert!(file.pair_passed("vitest", id));
    assert!(!file.pair_passed("jest", id));
}

/// RF §7.3's third column for `stream-invalid`, and RF §7.2's reason: "**That
/// runner** contributes **no** `result` records at all … other runners
/// contribute theirs, and the fold of §7.3 makes the file's `status`
/// `stream-invalid`, so nothing credits either way."
#[test]
fn a_stream_invalid_runner_keeps_its_base_records_and_loses_every_result_record() {
    let journal: Journal = Rc::new(RefCell::new(Vec::new()));
    let mut host = FakeHost {
        journal: Rc::clone(&journal),
        profile: Profile::Container,
    };
    let mut adapters: Vec<Box<dyn RunnerAdapter>> = vec![
        Box::new(FakeRunner {
            journal: Rc::clone(&journal),
            token: token("pytest"),
            enumeration: BaseEnumeration::Collected(vec![base_id("a.py::x", "a.py")]),
            outcomes: BaseOutcomeRun {
                reported: vec![("a.py::x".into(), Outcome::Passed)],
            },
            run: complete(vec![item("a.py::x", "a.py::x", "a.py", Outcome::Passed)]),
        }),
        Box::new(FakeRunner {
            journal: Rc::clone(&journal),
            token: token("vitest"),
            enumeration: BaseEnumeration::Collected(vec![base_id("b.ts > x", "b.ts")]),
            outcomes: BaseOutcomeRun {
                reported: vec![("b.ts > x".into(), Outcome::Passed)],
            },
            // An adapter that reported items *and* an unparsable event: the
            // collector drops the items, because the spec's third column is the
            // collector's rule and not the adapter's promise.
            run: CandidateRun {
                items: vec![item("b.ts > x", "b.ts > x", "b.ts", Outcome::Passed)],
                contribution: Status::StreamInvalid,
            },
        }),
    ];

    let file = collect(&run(), &policy(), Mode::Ci, &mut host, &mut adapters);
    assert_eq!(file.status, Status::StreamInvalid);
    assert_eq!(file.header.ids, 2, "both runners' base records survive");
    assert_eq!(file.results.len(), 1, "only pytest's result record");
    assert_eq!(file.results[0].runner.as_str(), "pytest");
    // "so nothing credits either way": the fold is not `complete`, so pytest's
    // green record is not a pass.
    assert!(!file.pair_passed("pytest", "a.py::x"));
}

/// RF §7.4: "two header values are settled before any observation is made:
/// **`keys_visible=true`** … and `profile=none`, because **outside `--ci` the
/// collector attempts no boundary at all**."
///
/// The host here *claims* `container`, which is the case the rule has to
/// survive: on the solo path the claim is not a finding the header may carry,
/// because "a boundary between a collector and a runner that both answer to the
/// same person establishes nothing the header could honestly report".
#[test]
fn the_solo_path_writes_keys_visible_true_and_profile_none_whatever_the_host_claims() {
    let journal: Journal = Rc::new(RefCell::new(Vec::new()));
    let mut host = FakeHost {
        journal: Rc::clone(&journal),
        profile: Profile::Container,
    };
    let mut adapters: Vec<Box<dyn RunnerAdapter>> = vec![Box::new(FakeRunner {
        journal: Rc::clone(&journal),
        token: token("pytest"),
        enumeration: BaseEnumeration::Collected(vec![base_id("a.py::x", "a.py")]),
        outcomes: BaseOutcomeRun {
            reported: vec![("a.py::x".into(), Outcome::Passed)],
        },
        run: complete(vec![item("a.py::x", "a.py::x", "a.py", Outcome::Passed)]),
    })];

    let file = collect(&run(), &policy(), Mode::Solo, &mut host, &mut adapters);
    assert!(file.header.keys_visible);
    assert_eq!(file.header.profile, Profile::None);
    // PB §5.4's "preconditions 1 and 2 fail by construction" is a derivation
    // from exactly these two values, and RF §7.3 makes the run exit non-zero on
    // the second of them even though every record is green.
    assert_eq!(file.status, Status::Complete);
    assert!(!spine_collect::exit_is_zero(&file));
}

/// RF §7.3's rows are "evaluated **top to bottom, first match wins**", and
/// `base-collect-failed` sits above every `T`-run row — so a runner whose `B`
/// enumeration failed contributes that row whatever its `T` run reported.
#[test]
fn a_failed_b_enumeration_outranks_that_runners_own_t_run_status() {
    let journal: Journal = Rc::new(RefCell::new(Vec::new()));
    let mut host = FakeHost {
        journal: Rc::clone(&journal),
        profile: Profile::None,
    };
    let mut adapters: Vec<Box<dyn RunnerAdapter>> = vec![Box::new(FakeRunner {
        journal: Rc::clone(&journal),
        token: token("pytest"),
        enumeration: BaseEnumeration::Failed,
        outcomes: BaseOutcomeRun::default(),
        run: CandidateRun {
            items: Vec::new(),
            contribution: Status::RunnerTimeout,
        },
    })];

    let file = collect(&run(), &policy(), Mode::Ci, &mut host, &mut adapters);
    assert_eq!(file.status, Status::BaseCollectFailed);
}

/// RF §4.5's determinism claim: "For a fixed `(B, T)`, a fixed collector build,
/// a fixed invocation set, and runners that behave identically, a file whose
/// `end.status` is `complete` is fully determined byte for byte … Two
/// conforming implementations produce identical files."
///
/// RF §7.1 step 8 supplies the freedom that claim has to survive: "Invocation
/// order and concurrency are an implementation choice and cannot affect the
/// file's bytes." So the same two runners, invoked in the opposite order,
/// produce the same bytes — which is what §4.5's sort on `runner` first buys.
#[test]
fn reversing_the_invocation_order_changes_no_byte_of_the_file() {
    fn scripted() -> Vec<Box<dyn RunnerAdapter>> {
        let journal: Journal = Rc::new(RefCell::new(Vec::new()));
        vec![
            Box::new(FakeRunner {
                journal: Rc::clone(&journal),
                token: token("pytest"),
                enumeration: BaseEnumeration::Collected(vec![base_id("a.py::x", "a.py")]),
                outcomes: BaseOutcomeRun {
                    reported: vec![("a.py::x".into(), Outcome::Passed)],
                },
                run: complete(vec![item("a.py::x", "a.py::x", "a.py", Outcome::Passed)]),
            }),
            Box::new(FakeRunner {
                journal: Rc::clone(&journal),
                token: token("vitest"),
                enumeration: BaseEnumeration::Collected(vec![base_id("b.ts > x", "b.ts")]),
                outcomes: BaseOutcomeRun {
                    reported: vec![("b.ts > x".into(), Outcome::Passed)],
                },
                run: complete(vec![item("b.ts > x", "b.ts > x", "b.ts", Outcome::Passed)]),
            }),
        ]
    }

    let journal: Journal = Rc::new(RefCell::new(Vec::new()));
    let mut host = FakeHost {
        journal: Rc::clone(&journal),
        profile: Profile::Container,
    };

    let mut forward = scripted();
    let one = collect(&run(), &policy(), Mode::Ci, &mut host, &mut forward).to_bytes();

    let mut reversed = scripted();
    reversed.reverse();
    let two = collect(&run(), &policy(), Mode::Ci, &mut host, &mut reversed).to_bytes();

    assert_eq!(one, two);
    // And the file records nothing about which ran first — RF §4.5: "The file
    // therefore does not record, and cannot be made to record, which runner ran
    // first."
    let text = String::from_utf8(one).expect("UTF-8");
    assert!(
        text.find("\"runner\":\"pytest\"").unwrap() < text.find("\"runner\":\"vitest\"").unwrap()
    );
}
