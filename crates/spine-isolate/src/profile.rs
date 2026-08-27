//! `profile=` — the header's field 5, and the whole of what step 6 reports.
//!
//! RF §7.1's verdict block, verbatim and fenced as printed:
//!
//! ```text
//! step 1:  params.isolation = "uid"        ->  refuse: fail the job, write nothing   (disposition 1)
//! step 6:  params.isolation = "container"  ->  "container"  if P1, P2, P3 and P4 all passed
//!                                              "none"       otherwise                (disposition 2)
//!          params.isolation = "none"       ->  "none", and no boundary is attempted
//! ```
//!
//! [`finding`] is that block and nothing else.

use crate::prereq::Prerequisite;
use crate::probe::ProbeReport;
use core::fmt;
use core::time::Duration;
use spine_manifest::schema::Isolation;

/// RF §4.2 field 5's domain. **Three** values: *"`profile=n/a` in a header is
/// malformed"* (RF §4.2). The seal's `profile=` admits a fourth, `n/a`, for a
/// tombstone (PB §11) — that is a different grammar, written by a different
/// stage, and is deliberately not representable here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Container,
    /// **No v1 collector ever writes this.** RF §7.1's profile table: *"nothing
    /// in v1 can license it"*. The variant exists because §4.2's grammar admits
    /// the token — an ingester must be able to name what it read — and
    /// `no_v1_verdict_ever_yields_uid` below pins that [`finding`] cannot
    /// produce it.
    Uid,
    None,
}

impl Profile {
    /// The header token. These bytes reach a reviewer's signed `wires=` through
    /// the seal (PB §11), so they are fixed here and nowhere else.
    pub fn as_str(self) -> &'static str {
        match self {
            Profile::Container => "container",
            Profile::Uid => "uid",
            Profile::None => "none",
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Which run this is.
///
/// RF §7.4, verbatim: *"outside `--ci` the collector attempts no boundary at
/// all … it attempts nothing, it **refuses nothing** — a manifest declaring
/// `uid` costs a solo developer no run, and disposition 1 of §7.1 is a `--ci`
/// rule — and it writes `none`."*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Ci,
    Solo,
}

/// A step-1 refusal: **fail the job and write nothing**, before `T` exists.
///
/// There is no status token for either variant, and that is not an omission:
/// a refusal writes no result file, so no `status` member and no wire carries
/// it. The only observable is `.spine/ci.sh`'s existing one — *"no file at the
/// expected path is `die 2`, exit 2"* (RF §7.1 disposition 1, CI §5.2). The
/// `Display` text below is therefore the collector's own stderr diagnostic.
/// DERIVED: the corpus fixes the *behaviour* (fail the job, write nothing) and
/// `ci.sh`'s message, never the collector's own wording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step1Refusal {
    /// Disposition 1. RF §7.1: *"This build ships no mechanism for the
    /// requested profile. In v1 that is exactly `params.isolation: "uid"`."*
    ///
    /// **It is never a downgrade to `none`**: *"`none` would spend a
    /// permanently sealed field on a defect the repository can neither see nor
    /// fix, dressing a refusal as a green run that merely cannot auto-merge."*
    NoMechanismForProfile(Isolation),
    /// RF §7.1 *The deadline*: `params.timeout` is *"a **strictly positive**
    /// integer number of seconds … It is present and not a positive integer:
    /// the collector fails the job and writes nothing (step 1's shape)."*
    TimeoutNotPositive(i64),
}

impl fmt::Display for Step1Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Step1Refusal::NoMechanismForProfile(request) => write!(
                f,
                "refused: params.isolation is \"{}\" and this build ships no mechanism for it; \
                 nothing ran and no result file exists",
                request.as_str()
            ),
            Step1Refusal::TimeoutNotPositive(v) => write!(
                f,
                "refused: params.timeout is {v} and must be a strictly positive integer \
                 number of seconds; nothing ran and no result file exists"
            ),
        }
    }
}

impl core::error::Error for Step1Refusal {}

/// What step 1 hands step 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step6Plan {
    /// `params.isolation == "container"` under `--ci`: attempt M1, and only M1.
    AttemptM1,
    /// *"`none`, and no boundary is attempted"* — and every solo run, whatever
    /// the manifest asked for.
    NoBoundary,
}

/// Step 1's isolation decision. Reads the request from trunk (the caller's job:
/// *"read from `origin/<trunk>`, never from the checkout"*, RF §7.1 step 1).
///
/// The request is **absent ⇒ `none`** before it reaches here — `Manifest::
/// isolation()` already folds that (MF §3.3), fail-closed.
pub fn step_one(mode: Mode, request: Isolation) -> Result<Step6Plan, Step1Refusal> {
    match (mode, request) {
        // §7.4. Not "refuse then continue": the solo collector never reaches
        // disposition 1, so a laptop whose trunk says `uid` still runs its
        // suite. Putting this arm first is what makes that true.
        (Mode::Solo, _) => Ok(Step6Plan::NoBoundary),
        (Mode::Ci, Isolation::Uid) => Err(Step1Refusal::NoMechanismForProfile(Isolation::Uid)),
        (Mode::Ci, Isolation::Container) => Ok(Step6Plan::AttemptM1),
        (Mode::Ci, Isolation::None) => Ok(Step6Plan::NoBoundary),
    }
}

/// RF §7.1 *The deadline*: *"`params.timeout` absent means `1800`"* (§6.7).
pub const DEFAULT_TIMEOUT_SECS: u64 = 1800;

/// The one deadline, read from trunk at step 1. It bounds **one runner
/// invocation** and **each of the two restore phases** (RF §7.1).
///
/// *"A collector that enforces no deadline is non-conformant, whatever the
/// manifest says: the field's absence selects the default, never the absence of
/// the control."* That is why this returns a `Duration` and never an `Option`.
pub fn deadline(params_timeout: Option<i64>) -> Result<Duration, Step1Refusal> {
    match params_timeout {
        None => Ok(Duration::from_secs(DEFAULT_TIMEOUT_SECS)),
        Some(secs) if secs > 0 => Ok(Duration::from_secs(secs as u64)),
        Some(secs) => Err(Step1Refusal::TimeoutNotPositive(secs)),
    }
}

/// Worst-case wall time, RF §7.1: *"`params.timeout` times the number of
/// invocations **plus two**"* — the two restore phases, which are not
/// invocations and are two per run whatever the invocation set holds.
pub fn worst_case_wall_time(deadline: Duration, invocations: u32) -> Duration {
    deadline.saturating_mul(invocations.saturating_add(2))
}

/// What step 6 came back with under an `AttemptM1` plan.
///
/// There is deliberately no `Established` variant. A boundary is "established"
/// only by [`ProbeReport::all_passed`], so the only way to reach
/// [`Profile::Container`] is to hold a report in which all four tests passed —
/// RF §11 item 16's first non-conformance is *"writes `container` without
/// having run the four tests"*, and a type that cannot express that claim
/// cannot make it by accident.
#[derive(Debug)]
pub enum BoundaryOutcome {
    /// Disposition 2, cause A: one of M1's five host prerequisites was absent.
    /// Discovered at step 6, *"before any repository process has run"*.
    PrerequisiteAbsent(Prerequisite),
    /// Disposition 2, cause B: *"creation failed"* mid-way through the mount
    /// sequence or the namespace set.
    CreationFailed(String),
    /// Disposition 2, cause C — or the licence for `container`. The boundary was
    /// built and the probe ran inside it.
    Tested(ProbeReport),
}

impl BoundaryOutcome {
    /// The stderr diagnostic RF §7.1 makes mandatory, and **the reason it must
    /// distinguish its two causes**, verbatim: *"a host that cannot build the
    /// boundary and a boundary that failed P1, P2, P3 or P4 differ to the human
    /// reading the `G11` wire, even though the header field they produce is the
    /// same."*
    ///
    /// `None` where the boundary was established: there is nothing to report.
    /// CI §5.1 reserves stderr for exactly this — *"every diagnostic, and all of
    /// the collector's own output, goes to stderr"* — while stdout on `collect`
    /// carries one line, `result=<path>`.
    pub fn diagnostic(&self) -> Option<String> {
        match self {
            BoundaryOutcome::PrerequisiteAbsent(p) => Some(format!(
                "isolation: the host could not build the boundary — M1 prerequisite {} ({}) is absent; \
                 profile=none, the suite runs unisolated",
                p.number(),
                p.summary()
            )),
            BoundaryOutcome::CreationFailed(why) => Some(format!(
                "isolation: the host could not build the boundary — creation failed: {why}; \
                 profile=none, the suite runs unisolated"
            )),
            BoundaryOutcome::Tested(report) if !report.all_passed() => {
                let failed: Vec<&str> = report.failed().iter().map(|t| t.name()).collect();
                Some(format!(
                    "isolation: the boundary was built and failed its test — {} failed ({}); \
                     no runner was spawned inside it; profile=none, the suite runs unisolated",
                    failed.join(", "),
                    report.failure_reasons().join("; ")
                ))
            }
            BoundaryOutcome::Tested(_) => None,
        }
    }
}

/// RF §7.1's verdict block, and nothing else.
///
/// *"There is no third outcome and no partial one: three tests out of four is
/// `none`."* And RF §7.1 *The collector never upgrades, and never substitutes*:
/// *"Where `params.isolation` is `none` no boundary is attempted and the header
/// says `none`; a collector that builds one anyway and writes `container` is
/// non-conformant"* — which is why the `NoBoundary` arm ignores `boundary`
/// entirely rather than reading it.
pub fn finding(plan: Step6Plan, boundary: Option<&BoundaryOutcome>) -> Profile {
    match (plan, boundary) {
        (Step6Plan::NoBoundary, _) => Profile::None,
        (Step6Plan::AttemptM1, Some(BoundaryOutcome::Tested(report))) if report.all_passed() => {
            Profile::Container
        }
        (Step6Plan::AttemptM1, _) => Profile::None,
    }
}

/// RF §7.1 disposition 2, and §11 item 16's fourth non-conformance: the
/// collector *"**runs no runner inside a boundary that did not pass its
/// test**"* — it *"tears down whatever it built … and proceeds with the run.
/// The suite runs unisolated and the file says so."*
///
/// This is the same predicate as `finding(..) == container` and is written as a
/// derivation of it on purpose: the boundary a runner is spawned under and the
/// profile the header records cannot be allowed to disagree, because that
/// disagreement is precisely what auto-merge precondition 1 is unable to see
/// (§8.4).
pub fn runners_are_spawned_inside_the_boundary(
    plan: Step6Plan,
    boundary: Option<&BoundaryOutcome>,
) -> bool {
    finding(plan, boundary) == Profile::Container
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::{Test, TestOutcome};

    fn all_four(pass: [bool; 4]) -> ProbeReport {
        let mk = |t: Test, ok: bool| {
            if ok {
                TestOutcome::passing(t)
            } else {
                TestOutcome::failing(t, "measured failure")
            }
        };
        ProbeReport::new(
            mk(Test::P1, pass[0]),
            mk(Test::P2, pass[1]),
            mk(Test::P3, pass[2]),
            mk(Test::P4, pass[3]),
        )
    }

    /// RF §7.1 disposition 1: *"The collector **refuses**: it fails the job and
    /// writes nothing, at step 1, where the value is read and before `T`
    /// exists."* And *"It is never a downgrade to `none`."*
    #[test]
    fn a_uid_request_under_ci_is_a_refusal_at_step_one_and_never_a_downgrade() {
        assert_eq!(
            step_one(Mode::Ci, Isolation::Uid),
            Err(Step1Refusal::NoMechanismForProfile(Isolation::Uid))
        );
    }

    /// RF §7.4: *"a manifest declaring `uid` costs a solo developer no run, and
    /// disposition 1 of §7.1 is a `--ci` rule"*.
    #[test]
    fn a_uid_request_outside_ci_costs_a_solo_developer_no_run() {
        assert_eq!(step_one(Mode::Solo, Isolation::Uid), Ok(Step6Plan::NoBoundary));
    }

    /// RF §7.4: *"outside `--ci` the collector attempts no boundary at all"*.
    #[test]
    fn the_solo_path_attempts_no_boundary_whatever_the_request_says() {
        for request in [Isolation::Container, Isolation::Uid, Isolation::None] {
            assert_eq!(
                step_one(Mode::Solo, request),
                Ok(Step6Plan::NoBoundary),
                "{}",
                request.as_str()
            );
        }
    }

    /// RF §7.1 *The deadline*, and §6.7: absent selects `1800`; present and not
    /// a strictly positive integer fails the job at step 1's shape.
    #[test]
    fn an_absent_timeout_is_1800_and_a_non_positive_one_fails_the_job() {
        assert_eq!(deadline(None), Ok(Duration::from_secs(1800)));
        assert_eq!(deadline(Some(1)), Ok(Duration::from_secs(1)));
        assert_eq!(deadline(Some(0)), Err(Step1Refusal::TimeoutNotPositive(0)));
        assert_eq!(deadline(Some(-1)), Err(Step1Refusal::TimeoutNotPositive(-1)));
    }

    /// RF §7.1 *The deadline*: worst case is the deadline *"times the number of
    /// invocations **plus two**"* — three languages at two or three invocations
    /// each is nine or ten times `params.timeout`.
    #[test]
    fn the_two_restore_phases_are_inside_the_worst_case_wall_time() {
        let t = Duration::from_secs(1800);
        // Three runners, one of which has a separate `B` outcome run: 3 `B`
        // enumerations + 1 outcome run + 3 `T` runs = 7 invocations.
        assert_eq!(worst_case_wall_time(t, 7), Duration::from_secs(1800 * 9));
        // A repository with one runner and no separate outcome run still pays
        // for two restore phases, "never one per runner".
        assert_eq!(worst_case_wall_time(t, 2), Duration::from_secs(1800 * 4));
    }

    /// RF §4.2 field 5, §7.1 profile table, §11 item 16: *"`uid` is written by
    /// no v1 collector"*. Exhaustive over every reachable input.
    #[test]
    fn no_v1_verdict_ever_yields_uid() {
        for mode in [Mode::Ci, Mode::Solo] {
            for request in [Isolation::Container, Isolation::Uid, Isolation::None] {
                let Ok(plan) = step_one(mode, request) else {
                    continue;
                };
                for outcome in [
                    None,
                    Some(BoundaryOutcome::PrerequisiteAbsent(Prerequisite::Namespaces)),
                    Some(BoundaryOutcome::CreationFailed("pivot_root".into())),
                    Some(BoundaryOutcome::Tested(all_four([true; 4]))),
                    Some(BoundaryOutcome::Tested(all_four([true, true, true, false]))),
                ] {
                    assert_ne!(finding(plan, outcome.as_ref()), Profile::Uid);
                }
            }
        }
    }

    /// RF §7.1: *"There is no third outcome and no partial one: three tests out
    /// of four is `none`."*
    #[test]
    fn three_tests_out_of_four_is_none() {
        for miss in 0..4 {
            let mut pass = [true; 4];
            pass[miss] = false;
            let outcome = BoundaryOutcome::Tested(all_four(pass));
            assert_eq!(
                finding(Step6Plan::AttemptM1, Some(&outcome)),
                Profile::None,
                "one failed test must not license container"
            );
        }
        let outcome = BoundaryOutcome::Tested(all_four([true; 4]));
        assert_eq!(
            finding(Step6Plan::AttemptM1, Some(&outcome)),
            Profile::Container
        );
    }

    /// RF §7.1: *"a collector that builds one anyway and writes `container` is
    /// non-conformant, because it would report a boundary trunk never requested
    /// and hand precondition 1 an agreement it was never meant to find."*
    #[test]
    fn a_none_request_that_somehow_built_a_boundary_still_writes_none() {
        let established = BoundaryOutcome::Tested(all_four([true; 4]));
        assert_eq!(
            finding(Step6Plan::NoBoundary, Some(&established)),
            Profile::None
        );
    }

    /// RF §7.1: *"A finding is never stronger than the request."* Exhaustive.
    #[test]
    fn the_finding_is_never_stronger_than_the_request() {
        for mode in [Mode::Ci, Mode::Solo] {
            for request in [Isolation::Container, Isolation::Uid, Isolation::None] {
                let Ok(plan) = step_one(mode, request) else {
                    continue;
                };
                let established = BoundaryOutcome::Tested(all_four([true; 4]));
                let profile = finding(plan, Some(&established));
                if profile == Profile::Container {
                    assert_eq!(mode, Mode::Ci);
                    assert_eq!(request, Isolation::Container);
                }
            }
        }
    }

    /// RF §7.1: *"a host that cannot build the boundary and a boundary that
    /// failed P1, P2, P3 or P4 differ to the human reading the `G11` wire, even
    /// though the header field they produce is the same."*
    #[test]
    fn the_diagnostic_distinguishes_a_missing_prerequisite_from_a_failed_test() {
        let absent = BoundaryOutcome::PrerequisiteAbsent(Prerequisite::NetworkNamespace);
        let failed = BoundaryOutcome::Tested(all_four([true, true, true, false]));

        let a = absent.diagnostic().expect("an absence is always diagnosed");
        let f = failed.diagnostic().expect("a failed test is always diagnosed");

        assert!(a.contains("could not build the boundary"), "{a}");
        assert!(a.contains("prerequisite 5"), "{a}");
        assert!(f.contains("failed its test"), "{f}");
        assert!(f.contains("P4"), "{f}");
        // Both fold to the same header field, which is exactly why the two
        // strings must not be confusable.
        assert_eq!(finding(Step6Plan::AttemptM1, Some(&absent)), Profile::None);
        assert_eq!(finding(Step6Plan::AttemptM1, Some(&failed)), Profile::None);
    }

    /// An established boundary is the one case with nothing to say.
    #[test]
    fn an_established_boundary_writes_no_diagnostic() {
        let established = BoundaryOutcome::Tested(all_four([true; 4]));
        assert_eq!(established.diagnostic(), None);
    }

    /// RF §11 item 16: an implementation *"that runs any runner inside a
    /// boundary whose test failed … is non-conformant"*. And the converse pin:
    /// what a runner is spawned under and what the header records are one
    /// decision, not two.
    #[test]
    fn no_runner_is_spawned_inside_a_boundary_whose_test_failed() {
        let outcomes = [
            BoundaryOutcome::PrerequisiteAbsent(Prerequisite::OverlayRoot),
            BoundaryOutcome::CreationFailed("ELOOP".into()),
            BoundaryOutcome::Tested(all_four([true, true, false, true])),
        ];
        for outcome in &outcomes {
            assert!(!runners_are_spawned_inside_the_boundary(
                Step6Plan::AttemptM1,
                Some(outcome)
            ));
            assert_eq!(finding(Step6Plan::AttemptM1, Some(outcome)), Profile::None);
        }
        let established = BoundaryOutcome::Tested(all_four([true; 4]));
        assert!(runners_are_spawned_inside_the_boundary(
            Step6Plan::AttemptM1,
            Some(&established)
        ));
        // And never under a request that named no boundary.
        assert!(!runners_are_spawned_inside_the_boundary(
            Step6Plan::NoBoundary,
            Some(&established)
        ));
    }

    /// RF §4.2: the three header tokens, byte for byte. They reach the ledger
    /// through `Spine-Seal`'s `profile=` and are never re-spelled.
    #[test]
    fn the_three_header_tokens_are_fixed() {
        assert_eq!(Profile::Container.as_str(), "container");
        assert_eq!(Profile::Uid.as_str(), "uid");
        assert_eq!(Profile::None.as_str(), "none");
    }
}
