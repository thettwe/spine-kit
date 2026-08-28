//! **G1 — Integrity · Coverage**, and the part of **G8 — Integrity · Freeze**
//! that RF §8.5 clause 2 allocates against it.
//!
//! The two gates are in one module because clause 2 is *one* decision with two
//! outputs, and splitting it is how an implementation ends up raising both
//! findings for one id, or neither. GR §5.6.1: "The allocation is made when the
//! outcomes are read, before any status is written."
//!
//! Three rules decide more `wires` arrays than anything else in this crate:
//!
//! - **The carve-out has two conjuncts.** `b.out` is `xfail` or `skipped`
//!   **and** the shape is *did not pass*. RF §8.5: "It does **not** reach the
//!   *went away* shape, and that is the boundary."
//! - **The per-id token is `G1:` + `tok(path)`** (RF §8.5, RF §13 R32), and the
//!   bare `G1` is a closed list of five.
//! - **Clause 3 matches on `(runner, fn)`, never on `id`.** "Matching on `id`
//!   would fail every parametrized AC test; matching on a bare `fn` across
//!   runners would let one runner's collection satisfy an AC verified in
//!   another."

use crate::gate::{Gate, LandingShape};
use crate::review::Reviews;
use crate::status::{G1Status, G8Status};
use crate::verdict::{Finding, FindingKind, Verdict, decide};
use crate::wire::{Wire, WireClass, WireKind};
use spine_collect::{Outcome, ResultFile};

/// What ingestion produced. RF §8.2's two failures name no path and are G1
/// findings, "a state is **not** entered".
#[derive(Debug, Clone, Copy)]
pub enum Ingestion<'a> {
    /// "No result file at the fixed path."
    Missing,
    /// "File found and §4 grammar or §8.3 step 3 rejects it."
    Malformed,
    Ingested(&'a ResultFile),
}

/// One frozen `Spine-Test` entry, parsed to `(R, F)` by RF §6.5.
///
/// "A failure names the runner and the function id, **in the `Spine-Test`
/// line's own spelling**, so a reviewer reading the wire and a reviewer reading
/// the approval commit see the same bytes" (RF §8.5 clause 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenTest {
    pub runner: String,
    pub function: String,
}

/// One acceptance criterion and the `(R, F)` pairs its `verified_by` edges
/// parse to (RF §8.5 clause 3; the join is IR §12's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcCoverage {
    pub ac: String,
    pub verified_by: Vec<FrozenTest>,
}

/// Everything G1 reads.
#[derive(Debug, Clone)]
pub struct G1Input<'a> {
    pub ingestion: Ingestion<'a>,
    /// Clause 1's entries. RF §8.6: "**Quick-lane and toolkit-lifecycle
    /// landings** … having no frozen ids, their G1 is the `B` floor alone."
    pub frozen: Vec<FrozenTest>,
    /// Clause 3's ACs. Vacuous on a landing with no `tests-approved`+ intent.
    pub acs: Vec<AcCoverage>,
    pub shape: LandingShape,
}

/// G1's verdict, plus the G8 findings clause 2 allocated.
///
/// The G8 half is **not** a whole G8 verdict: PB §6.3's G8 row has six other
/// clauses — the freeze comparison, `C-T3`, the intent blob, the `--ci`
/// closure — whose inputs are trees and envelopes rather than a result file.
/// A caller assembles them with [`crate::verdict::decide`].
#[derive(Debug, Clone)]
pub struct Coverage {
    pub g1: Verdict<G1Status>,
    /// Clause 2's `G8:<b.path>` findings, in the order the base records were
    /// read.
    pub g8_landed_id: Vec<Finding<G8Status>>,
}

/// GR §5.6.1's reseal row, as a predicate.
///
/// "**none** — the G1 and G8 rows above are suspended for this landing shape:
/// every G1 and G8 finding is admitted by the reseal's own `class=protected`
/// review naming that finding's token, and the gate reads `override`."
///
/// The row "is exactly G1 and G8 because those are the two gates PB §5.5
/// names": G13's, G14's and G16's outright findings "stay outright on every
/// shape, a reseal included".
fn kind_for(shape: LandingShape) -> FindingKind {
    match shape {
        LandingShape::Reseal => FindingKind::Coverable,
        // "Every G1 finding is **outright** … on every landing shape but a
        // reseal: PB §6's `tests-approved` row gives it no route into a review
        // state, and the only discharge is a `class=break-glass` review naming
        // `G1`" (GR §6.3), which is limb (b) and is applied by
        // `crate::verdict::with_break_glass`, not here.
        _ => FindingKind::Outright,
    }
}

/// GR §6.3: a G1 wire is `protected` and `finding`, always. "`protected` is the
/// class because break-glass 'never relaxes who must sign' (PB §11) and the
/// companion review that discharges the state must carry team mode's reviewer
/// separation."
fn g1_wire(path: Option<&str>) -> Wire {
    match path {
        None => Wire::pathless(Gate::G1, WireClass::Protected, WireKind::Finding),
        Some(p) => Wire::at(Gate::G1, p, WireClass::Protected, WireKind::Finding),
    }
}

fn g1_finding(shape: LandingShape, status: G1Status, path: Option<&str>) -> Finding<G1Status> {
    Finding {
        status,
        kind: kind_for(shape),
        wire: Some(g1_wire(path)),
    }
}

/// RF §8.5, whole, plus clause 2's allocation to G8.
pub fn evaluate(input: &G1Input<'_>, reviews: &Reviews) -> Coverage {
    let shape = input.shape;
    let mut g1: Vec<Finding<G1Status>> = Vec::new();
    let mut g8: Vec<Finding<G8Status>> = Vec::new();

    let file = match input.ingestion {
        Ingestion::Missing => {
            // RF §8.7: "the review that bypasses them carries a **bare** `G1` in
            // `wires=`", and "a review whose wire set carries a pathless wire
            // never survives a base move".
            g1.push(g1_finding(shape, G1Status::ResultMissing, None));
            return finish(g1, g8, reviews);
        }
        Ingestion::Malformed => {
            g1.push(g1_finding(shape, G1Status::ResultMalformed, None));
            return finish(g1, g8, reviews);
        }
        Ingestion::Ingested(file) => file,
    };

    // ---- Clause 0 ------------------------------------------------------
    // "**`end.status` is `complete`.** Otherwise **G1 fails**: no pair counts as
    // passed (§7.3), and no pair counts as *collected* either … Clauses 1–3 are
    // still evaluated and reported … **but the G8 allocation of clause 2 is not
    // made**: a killed or crashed run is no evidence that an id stopped
    // collecting, which is the question G8 asks and the one a partial run
    // cannot answer."
    let complete = file.status.credits_outcomes();
    if !complete {
        g1.push(g1_finding(shape, G1Status::RunIncomplete, None));
    }

    // ---- Clause 1 — every frozen `Spine-Test` entry --------------------
    for frozen in &input.frozen {
        let members: Vec<_> = file
            .results
            .iter()
            .filter(|r| r.runner.as_str() == frozen.runner && r.function == frozen.function)
            .collect();
        if members.is_empty() {
            // Bare finding 5: "the frozen entry collected nothing, so there is
            // no `result` record, and a frozen `Spine-Test` entry carries no
            // path of its own."
            g1.push(g1_finding(shape, G1Status::FrozenIdUncollected, None));
            continue;
        }
        // "**Clause 1** raises one entry per member of `P(R, F)` whose
        // `out ≠ \"passed\"`, over that member's `result` record `path`." Under
        // clause 0 no member counts as passed, so every member is one.
        for member in members {
            if !complete || member.out != Outcome::Passed {
                g1.push(g1_finding(
                    shape,
                    G1Status::FrozenIdNotPassed,
                    Some(&member.path),
                ));
            }
        }
    }

    // ---- Clause 2 — every `base` record, and the allocation ------------
    for b in &file.base {
        // `pair_passed` carries clause 0's conjunct itself (RF §7.3), so a
        // partial run makes every base record a candidate here — which is what
        // "no pair counts as passed" means.
        if file.pair_passed(b.runner.as_str(), &b.id) {
            continue;
        }
        let collected_on_t = file.pair_collected_on_t(b.runner.as_str(), &b.id);

        // **The carve-out is evaluated first, and it is a carve-out from both
        // gates.** Two conjuncts: `b.out` is `xfail` or `skipped`, *and* the
        // shape is *did not pass*. "**The carve-out is unconditional on every
        // landing shape, reseal included**" (GR §5.6.1) — hence no `shape` test.
        if b.out.was_xfail_or_skipped_on_b() && collected_on_t {
            continue;
        }

        // Which record supplies the path: "the pair's `result` record `path`
        // where the file carries one, its `base` record `path` where it does
        // not" (RF §8.5). "Where the two records for one pair disagree on
        // `path`, that is not an error and neither record is rejected."
        let result_path = file
            .results
            .iter()
            .find(|r| r.runner.as_str() == b.runner.as_str() && r.id == b.id)
            .map(|r| r.path.as_str());
        let path = result_path.unwrap_or(b.path.as_str());

        // The G8 half — suppressed entirely under clause 0.
        let g8_token = Wire::at(Gate::G8, path, WireClass::Protected, WireKind::Finding).token();
        if complete {
            g8.push(Finding {
                status: G8Status::LandedId,
                // GR §5.6.1's reseal row suspends G8's outright row too; off a
                // reseal the landed-id clause is coverable by a
                // `class=protected` `G8:<path>` review, which is what PB §6.3
                // means by "unless that review names its path".
                kind: FindingKind::Coverable,
                wire: Some(Wire::at(
                    Gate::G8,
                    path,
                    G8Status::LandedId.class(),
                    WireKind::Finding,
                )),
            });
        }

        // "**G1 fails on `b` as well**, save — for the *went-away* shape
        // only — where that same review names `G8:<b.path>`." GR §5.6.1: "the
        // wire set carries one `G8:<path>` finding and **no G1 finding**, and
        // the landing records **`G1=pass`, `G8=override`**."
        //
        // The `!path.is_empty()` conjunct is RF §8.5's and §13 R19's, and it is
        // not decoration: "An empty `b.path` names no path, **can satisfy no
        // exemption**, and is a G1 finding." Without it the empty path's own
        // token degrades to the bare `G8` (`Wire::at`'s fail-closed rule), and a
        // break-glass-shaped review naming `G8` would silently excuse G1 for
        // every id the collector could not locate in the tree.
        let excused = !collected_on_t
            && complete
            && !path.is_empty()
            && reviews.protected_names(&g8_token);
        if !excused {
            let status = if collected_on_t {
                G1Status::LandedIdNotPassed
            } else {
                G1Status::LandedIdWentAway
            };
            g1.push(g1_finding(shape, status, Some(path)));
        }
    }

    // ---- Clause 3 — AC coverage ---------------------------------------
    // "**Outcome is irrelevant; collection is the test.**" Under clause 0 "no
    // pair counts as *collected* either, so no clause below can be satisfied".
    for ac in &input.acs {
        let covered = complete
            && ac.verified_by.iter().any(|v| {
                file.results
                    .iter()
                    .any(|r| r.runner.as_str() == v.runner && r.function == v.function)
            });
        if !covered {
            g1.push(g1_finding(shape, G1Status::AcUncovered, None));
        }
    }

    finish(g1, g8, reviews)
}

fn finish(
    g1: Vec<Finding<G1Status>>,
    g8: Vec<Finding<G8Status>>,
    reviews: &Reviews,
) -> Coverage {
    Coverage {
        g1: decide(Gate::G1, g1, reviews),
        g8_landed_id: g8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::{Review, ReviewClass};
    use crate::verdict::GateStatus;
    use spine_collect::record::{BaseRecord, ResultRecord, RunnerToken, Status};
    use spine_collect::{BaseOutcome, Header, Profile, Provenance};

    fn runner(token: &str) -> RunnerToken {
        RunnerToken::new(token).expect("a v1 runner token")
    }

    fn header() -> Header {
        Provenance {
            tree: "de841d39b7a84111dfbcc11ddc7a75aa9886b218".into(),
            base: "1cbc18507888cb238c56ce00ba678c16564e0274".into(),
            // The SHA-256 of the ASCII bytes `spine-gates-test-artifacts`,
            // computed — a synthetic pin, not an elided corpus value.
            tool: "1.4.0+sha256:980d4cb66bc03353cdb93d9149ead2ec7aae73c8e1ab6ade536eb8628acd0753"
                .into(),
            keys_visible: false,
            profile: Profile::Container,
        }
        .into_header(0)
    }

    fn base(id: &str, out: BaseOutcome, path: &str) -> BaseRecord {
        BaseRecord {
            runner: runner("pytest"),
            id: id.into(),
            out,
            path: path.into(),
        }
    }

    fn result(id: &str, function: &str, out: Outcome, path: &str) -> ResultRecord {
        ResultRecord {
            runner: runner("pytest"),
            id: id.into(),
            function: function.into(),
            out,
            path: path.into(),
        }
    }

    fn file(status: Status, base: Vec<BaseRecord>, results: Vec<ResultRecord>) -> ResultFile {
        ResultFile {
            header: header(),
            base,
            results,
            status,
        }
    }

    fn input<'a>(file: &'a ResultFile, shape: LandingShape) -> G1Input<'a> {
        G1Input {
            ingestion: Ingestion::Ingested(file),
            frozen: Vec::new(),
            acs: Vec::new(),
            shape,
        }
    }

    /// RF §8.5, RF §13 R32, GR §6.3: "**A per-id finding takes `G1:` +
    /// `tok(path)`**".
    #[test]
    fn a_per_id_finding_takes_g1_plus_tok_of_the_path() {
        let f = file(
            Status::Complete,
            vec![base("t.py::a", BaseOutcome::Reported(Outcome::Passed), "t.py")],
            vec![result("t.py::a", "t.py::a", Outcome::Failed, "t b.py")],
        );
        let out = evaluate(&input(&f, LandingShape::GatedLand), &Reviews::default());
        assert_eq!(out.g1.wires.tokens(), ["G1:t\\x20b.py"]);
    }

    /// RF §8.5: the five that name no path take the bare form.
    #[test]
    fn result_missing_takes_the_bare_g1() {
        let out = evaluate(
            &G1Input {
                ingestion: Ingestion::Missing,
                frozen: Vec::new(),
                acs: Vec::new(),
                shape: LandingShape::Quick,
            },
            &Reviews::default(),
        );
        assert_eq!(out.g1.wires.tokens(), ["G1"]);
        assert_eq!(out.g1.status, GateStatus::Fail);
        assert_eq!(out.g1.statuses(), [&G1Status::ResultMissing]);
    }

    /// RF §8.7: "**in the quick lane a `result-missing`, `result-malformed` or
    /// G1 finding is terminal**" — no override of any class is reachable.
    /// PB §7.6 puts break-glass out of reach before `tests-approved`.
    #[test]
    fn a_quick_lane_result_missing_is_terminal_under_a_protected_review() {
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a").naming(vec!["G1"]),
        ]);
        let out = evaluate(
            &G1Input {
                ingestion: Ingestion::Missing,
                frozen: Vec::new(),
                acs: Vec::new(),
                shape: LandingShape::Quick,
            },
            &reviews,
        );
        assert_eq!(out.g1.status, GateStatus::Fail);
    }

    /// RF §8.6, §8.7 and GR §5.6.1's reseal row: "a reseal whose file is absent
    /// or malformed is not exempt and is not refused … that same review admits
    /// it."
    #[test]
    fn a_reseals_own_protected_review_admits_result_missing() {
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a").naming(vec!["G1"]),
            Review::new(ReviewClass::Protected, "SHA256:b").naming(vec!["G1"]),
        ]);
        let out = evaluate(
            &G1Input {
                ingestion: Ingestion::Missing,
                frozen: Vec::new(),
                acs: Vec::new(),
                shape: LandingShape::Reseal,
            },
            &reviews,
        );
        assert_eq!(out.g1.status, GateStatus::Override);
    }

    /// RF §8.5 clause 2, the carve-out's **two** conjuncts. An id trunk itself
    /// reported `xfail` that still collects on `T` "yields **no finding at
    /// all** — not G1's, not G8's".
    #[test]
    fn an_xfail_on_b_that_still_collects_on_t_is_no_finding_in_either_gate() {
        let f = file(
            Status::Complete,
            vec![base("t.py::a", BaseOutcome::Reported(Outcome::Xfail), "t.py")],
            vec![result("t.py::a", "t.py::a", Outcome::Failed, "t.py")],
        );
        let out = evaluate(&input(&f, LandingShape::GatedLand), &Reviews::default());
        assert_eq!(out.g1.status, GateStatus::Pass);
        assert!(out.g1.wires.is_empty());
        assert!(out.g8_landed_id.is_empty());
    }

    /// "It is decided on `b.out` **alone** and never on the `T` outcome —
    /// `xfail`, `failed`, `error`, `skipped`, `xpass`, `unknown` on `T` all
    /// leave it carved out."
    #[test]
    fn the_carve_out_reads_b_alone_and_never_the_t_outcome() {
        for t_outcome in [
            Outcome::Xfail,
            Outcome::Failed,
            Outcome::Error,
            Outcome::Skipped,
            Outcome::Xpass,
            Outcome::Unknown,
        ] {
            let f = file(
                Status::Complete,
                vec![base(
                    "t.py::a",
                    BaseOutcome::Reported(Outcome::Skipped),
                    "t.py",
                )],
                vec![result("t.py::a", "t.py::a", t_outcome, "t.py")],
            );
            let out = evaluate(&input(&f, LandingShape::GatedLand), &Reviews::default());
            assert_eq!(out.g1.status, GateStatus::Pass, "{t_outcome}");
        }
    }

    /// "**It does not reach the *went away* shape, and that is the
    /// boundary.**" A vanished `xfail` id "is a harness change like any other
    /// and remains G8's, review and all."
    #[test]
    fn a_vanished_xfail_id_is_still_g8s_finding() {
        let f = file(
            Status::Complete,
            vec![base("t.py::a", BaseOutcome::Reported(Outcome::Xfail), "t.py")],
            vec![],
        );
        let out = evaluate(&input(&f, LandingShape::GatedLand), &Reviews::default());
        assert_eq!(out.g8_landed_id.len(), 1);
        assert_eq!(
            out.g8_landed_id[0].wire.as_ref().unwrap().token(),
            "G8:t.py"
        );
        assert_eq!(out.g1.status, GateStatus::Fail);
    }

    /// GR §5.6.1: "**An id-loss a `class=protected` `G8:<path>` review names is
    /// G8's finding, never G1's.** … the landing records `G1=pass`,
    /// `G8=override`."
    #[test]
    fn a_g8_path_review_moves_the_went_away_finding_off_g1_entirely() {
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a").naming(vec!["G8:t.py"]),
        ]);
        let f = file(
            Status::Complete,
            vec![base("t.py::a", BaseOutcome::Reported(Outcome::Passed), "t.py")],
            vec![],
        );
        let out = evaluate(&input(&f, LandingShape::GatedLand), &reviews);
        assert_eq!(out.g1.status, GateStatus::Pass);
        assert!(out.g1.wires.is_empty());
        assert_eq!(out.g8_landed_id.len(), 1);
    }

    /// GR §5.6.1: "**The `G8:<path>` review exemption reaches the went-away
    /// shape only.** An id that collected on `T` and did not pass is a finding
    /// of both gates … not landable until a `class=break-glass` review bypasses
    /// G1."
    #[test]
    fn the_g8_review_exemption_does_not_reach_an_id_that_collected_and_failed() {
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a").naming(vec!["G8:t.py"]),
        ]);
        let f = file(
            Status::Complete,
            vec![base("t.py::a", BaseOutcome::Reported(Outcome::Passed), "t.py")],
            vec![result("t.py::a", "t.py::a", Outcome::Failed, "t.py")],
        );
        let out = evaluate(&input(&f, LandingShape::GatedLand), &reviews);
        assert_eq!(out.g1.status, GateStatus::Fail);
        assert_eq!(out.g1.wires.tokens(), ["G1:t.py"]);
        assert_eq!(out.g8_landed_id.len(), 1);
    }

    /// RF §8.5 clause 0: "the G8 allocation of clause 2 is **not** made — a
    /// killed or crashed run is no evidence that an id stopped collecting."
    #[test]
    fn a_partial_run_makes_no_g8_allocation_and_still_names_the_pairs() {
        let f = file(
            Status::RunnerTimeout,
            vec![base("t.py::a", BaseOutcome::Reported(Outcome::Passed), "t.py")],
            vec![result("t.py::a", "t.py::a", Outcome::Passed, "t.py")],
        );
        let out = evaluate(&input(&f, LandingShape::GatedLand), &Reviews::default());
        assert!(out.g8_landed_id.is_empty());
        // Clause 0's own bare finding, plus the pair the file failed to account
        // for — "raised *in addition to* whatever clauses 1–3 then raise".
        assert_eq!(out.g1.wires.tokens(), ["G1", "G1:t.py"]);
        assert_eq!(out.g1.status, GateStatus::Fail);
    }

    /// RF §7.3 and §10: "one runner timing out makes the *other* runner's seven
    /// green records as unaccounted-for as the killed runner's."
    #[test]
    fn a_green_record_under_a_non_complete_status_is_not_credit() {
        let f = file(
            Status::RunnerTimeout,
            vec![],
            vec![result("t.py::a", "t.py::a", Outcome::Passed, "t.py")],
        );
        let out = evaluate(
            &G1Input {
                ingestion: Ingestion::Ingested(&f),
                frozen: vec![FrozenTest {
                    runner: "pytest".into(),
                    function: "t.py::a".into(),
                }],
                acs: Vec::new(),
                shape: LandingShape::GatedLand,
            },
            &Reviews::default(),
        );
        assert!(out.g1.wires.tokens().contains(&"G1:t.py".to_string()));
    }

    /// RF §8.5: "**One entry per path, never per pair**: two failing ids from
    /// one file, and a parametrized function's several failing ids, collapse to
    /// a single `G1:<path>` entry."
    #[test]
    fn two_failing_ids_from_one_file_collapse_to_one_entry() {
        let f = file(
            Status::Complete,
            vec![
                base("t.py::a", BaseOutcome::Reported(Outcome::Passed), "t.py"),
                base("t.py::b", BaseOutcome::Reported(Outcome::Passed), "t.py"),
            ],
            vec![
                result("t.py::a", "t.py::a", Outcome::Failed, "t.py"),
                result("t.py::b", "t.py::b", Outcome::Failed, "t.py"),
            ],
        );
        let out = evaluate(&input(&f, LandingShape::GatedLand), &Reviews::default());
        assert_eq!(out.g1.wires.tokens(), ["G1:t.py"]);
    }

    /// RF §8.5 clause 3: "The match is on `(runner, fn)` … a parametrized
    /// function id never appears as a `result` record's `id`, only as its
    /// `fn`. Matching on `id` would fail every parametrized AC test."
    #[test]
    fn clause_3_matches_on_runner_and_fn_never_on_id() {
        let f = file(
            Status::Complete,
            vec![],
            vec![result(
                "t.py::totals[eu]",
                "t.py::totals",
                Outcome::Failed,
                "t.py",
            )],
        );
        let out = evaluate(
            &G1Input {
                ingestion: Ingestion::Ingested(&f),
                frozen: Vec::new(),
                acs: vec![AcCoverage {
                    ac: "AC1".into(),
                    verified_by: vec![FrozenTest {
                        runner: "pytest".into(),
                        function: "t.py::totals".into(),
                    }],
                }],
                shape: LandingShape::GatedLand,
            },
            &Reviews::default(),
        );
        // "**Outcome is irrelevant; collection is the test.**"
        assert_eq!(out.g1.status, GateStatus::Pass);
    }

    /// "matching on a bare `fn` across runners would let one runner's
    /// collection satisfy an AC verified in another."
    #[test]
    fn one_runners_collection_never_satisfies_an_ac_verified_in_another() {
        let f = file(
            Status::Complete,
            vec![],
            vec![result("t.py::totals", "t.py::totals", Outcome::Passed, "t.py")],
        );
        let out = evaluate(
            &G1Input {
                ingestion: Ingestion::Ingested(&f),
                frozen: Vec::new(),
                acs: vec![AcCoverage {
                    ac: "AC1".into(),
                    verified_by: vec![FrozenTest {
                        runner: "vitest".into(),
                        function: "t.py::totals".into(),
                    }],
                }],
                shape: LandingShape::GatedLand,
            },
            &Reviews::default(),
        );
        assert_eq!(out.g1.statuses(), [&G1Status::AcUncovered]);
        assert_eq!(out.g1.wires.tokens(), ["G1"]);
    }

    /// RF §8.5 bare finding 5: "**Clause 1 where `P(R, F)` is empty** — the
    /// frozen entry collected nothing."
    #[test]
    fn a_frozen_entry_that_collected_nothing_takes_the_bare_g1() {
        let f = file(Status::Complete, vec![], vec![]);
        let out = evaluate(
            &G1Input {
                ingestion: Ingestion::Ingested(&f),
                frozen: vec![FrozenTest {
                    runner: "pytest".into(),
                    function: "t.py::gone".into(),
                }],
                acs: Vec::new(),
                shape: LandingShape::GatedLand,
            },
            &Reviews::default(),
        );
        assert_eq!(out.g1.statuses(), [&G1Status::FrozenIdUncollected]);
        assert_eq!(out.g1.wires.tokens(), ["G1"]);
    }

    /// GR §5.6.1's third boundary: the carve-out "does **not** reach a frozen
    /// `Spine-Test` entry, only a `B`-floor id — so a landing whose frozen id
    /// is not passed is a G1 finding and outright, whatever `B` said about it."
    #[test]
    fn the_carve_out_does_not_reach_a_frozen_entry() {
        let f = file(
            Status::Complete,
            vec![base("t.py::a", BaseOutcome::Reported(Outcome::Xfail), "t.py")],
            vec![result("t.py::a", "t.py::a", Outcome::Xfail, "t.py")],
        );
        let out = evaluate(
            &G1Input {
                ingestion: Ingestion::Ingested(&f),
                frozen: vec![FrozenTest {
                    runner: "pytest".into(),
                    function: "t.py::a".into(),
                }],
                acs: Vec::new(),
                shape: LandingShape::GatedLand,
            },
            &Reviews::default(),
        );
        assert_eq!(out.g1.statuses(), [&G1Status::FrozenIdNotPassed]);
        assert_eq!(out.g1.status, GateStatus::Fail);
    }

    /// RF §8.5: "A finding about an id that collected on `T` and did not pass
    /// cites that pair's `result` record `path`; a finding about an id that
    /// went away cites its `base` record `path`."
    #[test]
    fn the_path_comes_from_the_result_record_where_one_exists_and_the_base_record_otherwise() {
        let went_away = file(
            Status::Complete,
            vec![base(
                "t.py::a",
                BaseOutcome::Reported(Outcome::Passed),
                "old/t.py",
            )],
            vec![],
        );
        let out = evaluate(&input(&went_away, LandingShape::GatedLand), &Reviews::default());
        assert_eq!(out.g1.wires.tokens(), ["G1:old/t.py"]);

        let disagreeing = file(
            Status::Complete,
            vec![base(
                "t.py::a",
                BaseOutcome::Reported(Outcome::Passed),
                "old/t.py",
            )],
            vec![result("t.py::a", "t.py::a", Outcome::Failed, "new/t.py")],
        );
        let out = evaluate(
            &input(&disagreeing, LandingShape::GatedLand),
            &Reviews::default(),
        );
        assert_eq!(out.g1.wires.tokens(), ["G1:new/t.py"]);
    }

    /// RF §8.5, §13 R19: "An empty `b.path` names no path, can satisfy no
    /// exemption, and is a G1 finding."
    #[test]
    fn an_empty_path_takes_the_bare_g1_and_satisfies_no_exemption() {
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a").naming(vec!["G8:", "G8"]),
        ]);
        let f = file(
            Status::Complete,
            vec![base("t.py::a", BaseOutcome::Reported(Outcome::Passed), "")],
            vec![],
        );
        let out = evaluate(&input(&f, LandingShape::GatedLand), &reviews);
        assert_eq!(out.g1.wires.tokens(), ["G1"]);
        assert_eq!(out.g1.status, GateStatus::Fail);
    }

    /// GR §5.6.1: "**The carve-out is unconditional on every landing shape,
    /// reseal included**", while "the three *boundary* cases read `override` on
    /// a reseal".
    #[test]
    fn the_carve_out_is_unconditional_on_a_reseal_and_its_boundaries_read_override() {
        let carved = file(
            Status::Complete,
            vec![base("t.py::a", BaseOutcome::Reported(Outcome::Xfail), "t.py")],
            vec![result("t.py::a", "t.py::a", Outcome::Failed, "t.py")],
        );
        let out = evaluate(&input(&carved, LandingShape::Reseal), &Reviews::default());
        assert_eq!(out.g1.status, GateStatus::Pass);

        // The boundary: `b.out` was neither `xfail` nor `skipped`.
        let boundary = file(
            Status::Complete,
            vec![base("t.py::a", BaseOutcome::Reported(Outcome::Passed), "t.py")],
            vec![result("t.py::a", "t.py::a", Outcome::Failed, "t.py")],
        );
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a").naming(vec!["G1:t.py"]),
            Review::new(ReviewClass::Protected, "SHA256:b").naming(vec!["G1:t.py"]),
        ]);
        let out = evaluate(&input(&boundary, LandingShape::Reseal), &reviews);
        assert_eq!(out.g1.status, GateStatus::Override);
    }

    /// RF §8.5: "`base` record `out` … `absent`" is not `xfail` or `skipped`,
    /// so it never carves out. RF §4.4: "every other value — `absent`
    /// included — answers it identically."
    #[test]
    fn an_absent_base_outcome_carves_out_nothing() {
        let f = file(
            Status::Complete,
            vec![base("t.py::a", BaseOutcome::Absent, "t.py")],
            vec![result("t.py::a", "t.py::a", Outcome::Failed, "t.py")],
        );
        let out = evaluate(&input(&f, LandingShape::GatedLand), &Reviews::default());
        assert_eq!(out.g1.status, GateStatus::Fail);
        assert_eq!(out.g8_landed_id.len(), 1);
    }

    /// PB §7.6 and GR §5.6.1 limb (b): break-glass names the bare gate id, and
    /// it reaches G1.
    #[test]
    fn a_break_glass_review_naming_g1_reads_override() {
        let reviews =
            Reviews::new(vec![Review::new(ReviewClass::BreakGlass, "SHA256:a").naming(vec!["G1"])]);
        let out = evaluate(
            &G1Input {
                ingestion: Ingestion::Missing,
                frozen: Vec::new(),
                acs: Vec::new(),
                shape: LandingShape::GatedLand,
            },
            &reviews,
        );
        let bypassed = crate::verdict::with_break_glass(out.g1, &reviews);
        assert_eq!(bypassed.status, GateStatus::Override);
    }
}
