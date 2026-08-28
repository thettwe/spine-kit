//! The whole file — RF §4.1's framing, §4.5's ordering, and the round trip.
//!
//! RF §4.5 fixes four things and the reason for the sort is the point:
//!
//! > 1. Header line.
//! > 2. Every `base` record, sorted ascending by the **bytes** of `runner`,
//! >    then by the **bytes** of `id`.
//! > 3. Every `result` record, sorted ascending by the bytes of `runner`, then
//! >    by the bytes of `id`.
//! > 4. The `end` record.
//! >
//! > … Byte-order sorting removes the runner's report order — which is not
//! > deterministic and would otherwise be the file's only clock — and sorting
//! > on `runner` first removes the *invocation* order of the runners
//! > themselves, which is the second clock multi-runner would otherwise
//! > introduce. The file therefore does not record, and cannot be made to
//! > record, which runner ran first.
//!
//! [`ResultFile::new`] sorts rather than trusting its caller, for exactly that
//! reason: a collector that wrote records in invocation order would produce a
//! file whose bytes are a function of which runner it happened to start first,
//! and RF §4.5's determinism claim is that two conforming implementations
//! produce identical files.

use crate::header::{Header, Provenance};
use crate::malformed::{Malformed, Section};
use crate::record::{BaseRecord, EndRecord, ResultRecord, Status, record_kind};
use spine_canon::{ObjectFormat, canonicalize, parse};

/// A whole result file, parsed or about to be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultFile {
    pub header: Header,
    /// Section 2, sorted. RF §6.5: "The `B` floor does not roll up. Section 2
    /// holds full ids as collected, each with its runner."
    pub base: Vec<BaseRecord>,
    /// Section 3, sorted.
    pub results: Vec<ResultRecord>,
    /// The `end` record's status — the fold of RF §7.3.
    pub status: Status,
}

impl ResultFile {
    /// Assemble a file for writing. Sorts both sections and derives `ids=`.
    ///
    /// `ids` is derived rather than passed because RF §4.2 makes it a
    /// cross-check of the collector against itself — "a collector that emitted
    /// fewer `base` records than it counted is the case nothing else catches" —
    /// and a value the caller supplies is a value the caller can get wrong. The
    /// cross-check lives on the *read* side, in [`ResultFile::parse`], where a
    /// disagreement is another implementation's bug rather than this one's.
    pub fn new(
        provenance: Provenance,
        base: Vec<BaseRecord>,
        results: Vec<ResultRecord>,
        status: Status,
    ) -> Self {
        Self::build(provenance, base, results, status)
            .expect("callers that may produce a duplicate pair must use `try_new`")
    }

    /// The fallible form, and the one the collector uses.
    ///
    /// Two invariants are enforced here rather than at a call site, for the
    /// reason `new` already gives about `ids=`: "a value the caller supplies is
    /// a value the caller can get wrong."
    ///
    /// **Pair uniqueness.** RF §11 item 5 makes it a conformance obligation on
    /// the *writer*, not only the reader. Sorting without checking strict
    /// ascension let two adapters sharing a `runner` token emit a file this
    /// crate's own [`ResultFile::parse`] then rejects as `result-malformed` —
    /// a writer that cannot read back what it wrote.
    ///
    /// **The all-or-nothing statuses.** RF §7.3: on a failed `B` enumeration
    /// "`ids=0`, and **no `base` and no `result` records are written at all**,
    /// from any runner." Enforcing it in the constructor makes it a property of
    /// the type instead of a property of one call site.
    pub fn try_new(
        provenance: Provenance,
        base: Vec<BaseRecord>,
        results: Vec<ResultRecord>,
        status: Status,
    ) -> Result<Self, WriteRefusal> {
        Self::build(provenance, base, results, status)
    }

    fn build(
        provenance: Provenance,
        mut base: Vec<BaseRecord>,
        mut results: Vec<ResultRecord>,
        status: Status,
    ) -> Result<Self, WriteRefusal> {
        base.sort_by(|a, b| a.key().cmp(&b.key()));
        results.sort_by(|a, b| a.key().cmp(&b.key()));

        for pair in base.windows(2) {
            if pair[0].key() == pair[1].key() {
                return Err(WriteRefusal::DuplicatePair {
                    section: "base",
                    runner: pair[0].runner.to_string(),
                    id: pair[0].id.clone(),
                });
            }
        }
        for pair in results.windows(2) {
            if pair[0].key() == pair[1].key() {
                return Err(WriteRefusal::DuplicatePair {
                    section: "result",
                    runner: pair[0].runner.to_string(),
                    id: pair[0].id.clone(),
                });
            }
        }

        // All-or-nothing applies to the failed **enumeration** and to nothing
        // else. RF §7.3 draws the line precisely, and it is the asymmetry the
        // build plan's R14 names: a failed `B` enumeration is all-or-nothing
        // across runners ("`ids=0`, and no `base` and no `result` records are
        // written at all, from any runner"), while a failed `B` *outcome* run
        // "is not a status at all", and a `stream-invalid` runner keeps its
        // `base` records and loses only its `result` records.
        //
        // So the test is `== BaseCollectFailed`, not `!credits_outcomes()`.
        // The wider reading was written here first and a `stream-invalid`
        // integration test refused it — correctly: the two statuses credit
        // nothing for different reasons and only one of them empties the file.
        if status == Status::BaseCollectFailed && !(base.is_empty() && results.is_empty()) {
            return Err(WriteRefusal::BodyBesideAllOrNothingStatus {
                status: status.token(),
                base: base.len(),
                results: results.len(),
            });
        }

        let header = provenance.into_header(base.len() as u64);
        Ok(ResultFile {
            header,
            base,
            results,
            status,
        })
    }

    /// RF §4.1's framing: "UTF-8, no BOM … Lines are terminated by a single LF
    /// (`U+000A`). Every line, including the last, is terminated."
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = String::new();
        out.push_str(&self.header.to_line());
        out.push('\n');
        for record in &self.base {
            out.push_str(&record.to_line());
            out.push('\n');
        }
        for record in &self.results {
            out.push_str(&record.to_line());
            out.push('\n');
        }
        out.push_str(
            &EndRecord {
                status: self.status,
            }
            .to_line(),
        );
        out.push('\n');
        out.into_bytes()
    }

    /// Read a file back, refusing every particular of RF §4.
    ///
    /// `stem` is the `<T>` of the path the bytes were read from, checked
    /// against `tree=` per RF §3. `format` is trunk's `object_format`. Neither
    /// is taken from the file, because both are what the file is checked
    /// *against*.
    ///
    /// RF §8.2: "**There is no partial ingestion** — a malformed file yields no
    /// outcomes at all, never 'read what parsed'. Reading a truncated file
    /// partially is how a cut-short `base` section would silently shrink the
    /// floor." So every path out of here is either a whole file or an error.
    pub fn parse(bytes: &[u8], stem: &str, format: ObjectFormat) -> Result<Self, Malformed> {
        let text = core::str::from_utf8(bytes).map_err(|_| Malformed::NotUtf8)?;
        if text.is_empty() {
            return Err(Malformed::Empty);
        }
        // "no bytes after the final LF" — so the file ends with one, and
        // stripping it leaves exactly the lines.
        let Some(body) = text.strip_suffix('\n') else {
            return Err(Malformed::UnterminatedFinalLine);
        };

        let lines: Vec<&str> = body.split('\n').collect();
        for (index, line) in lines.iter().enumerate() {
            let number = index + 1;
            // RF §4.1: "A CR (`U+000D`) anywhere outside a JSON string escape
            // makes the file malformed — the same rule that keeps envelopes
            // hashing (§5.5)." §4.3 escapes `U+000D` as `\r`, so a raw CR byte
            // is outside a string escape by construction and needs no parse to
            // find. This is also what refuses a CRLF-translated file, which is
            // the way this rule is actually broken in the field.
            if line.contains('\r') {
                return Err(Malformed::CarriageReturn { line: number });
            }
            if line.is_empty() {
                return Err(Malformed::BlankLine { line: number });
            }
        }

        let header = Header::parse(lines[0], format)?;
        header.check_stem(stem)?;

        let mut base: Vec<BaseRecord> = Vec::new();
        let mut results: Vec<ResultRecord> = Vec::new();
        let mut end: Option<Status> = None;

        for (index, line) in lines.iter().enumerate().skip(1) {
            let number = index + 1;
            let value = parse(line.as_bytes()).map_err(|why| Malformed::LineNotJson {
                line: number,
                why: why.to_string(),
            })?;
            // RF §4.3: "**Canonical form is required on read, not only on
            // write.** A body line that parses as JSON but is not in canonical
            // form is malformed. This is affordable because writer and reader
            // are the same pinned release … and it is what makes the file's
            // bytes a function of its content."
            //
            // Re-serializing and comparing bytes is the whole check: it catches
            // member order, whitespace, a non-minimal escape (`A` for `A`)
            // and an uppercase `\u00XX` in one comparison, and it cannot drift
            // from the writer because it *is* the writer.
            if canonicalize(&value) != line.as_bytes() {
                return Err(Malformed::LineNotCanonical { line: number });
            }

            let kind = record_kind(&value, number)?;
            // RF §4.5 lists "a record after `end`" and "a second `end`" as two
            // faults, and they are kept apart here because they are two
            // different bugs: an appended stray record, against a collector
            // that flushed per runner — which RF §3 forbids in terms ("There is
            // no append, no per-runner flush and no partial publish").
            if end.is_some() {
                return Err(if kind == "end" {
                    Malformed::SecondEnd { line: number }
                } else {
                    Malformed::RecordAfterEnd { line: number }
                });
            }

            match kind {
                "base" => {
                    // "A `base` record after a `result` record … malformed."
                    if !results.is_empty() {
                        return Err(Malformed::BaseAfterResult { line: number });
                    }
                    let record = BaseRecord::from_value(&value, number)?;
                    check_order(
                        base.last().map(BaseRecord::key),
                        record.key(),
                        Section::Base,
                        number,
                    )?;
                    base.push(record);
                }
                "result" => {
                    let record = ResultRecord::from_value(&value, number)?;
                    check_order(
                        results.last().map(ResultRecord::key),
                        record.key(),
                        Section::Result,
                        number,
                    )?;
                    results.push(record);
                }
                "end" => end = Some(EndRecord::from_value(&value, number)?.status),
                other => {
                    return Err(Malformed::UnknownRecordKind {
                        line: number,
                        t: other.to_owned(),
                    });
                }
            }
        }

        // "a missing `end` … malformed." RF §4.2 names this the truncation
        // guard: "truncation removes the `end` record, which §4.5 already makes
        // malformed", which is why `ids=` is not asked to be one.
        let status = end.ok_or(Malformed::MissingEnd)?;

        // "A count that disagrees with the number of `base` records present is
        // malformed."
        if header.ids != base.len() as u64 {
            return Err(Malformed::IdsDisagreesWithBaseRecords {
                header: header.ids,
                records: base.len(),
            });
        }

        Ok(ResultFile {
            header,
            base,
            results,
            status,
        })
    }

    /// RF §5's mapping, both conjuncts: "G1 counts a pair `(R, i)` as passed
    /// **iff** the body contains exactly one `result` record with `runner == R`
    /// and `id == i` whose `out` is `passed`, **and** the `end` record's
    /// `status` is `complete`."
    ///
    /// The `status` conjunct is inside this function rather than left to the
    /// caller because RF §7.3 makes forgetting it the one way an empty file
    /// passes vacuously: "a quick-lane landing has no frozen ids, so its G1 is
    /// the `B` floor alone, and a `base-collect-failed` file would otherwise
    /// satisfy it by emptiness."
    /// Clause 2's **second** conjunct: does this pair still collect on `T`?
    ///
    /// RF §8.5's carve-out releases an id that was `xfail` or `skipped` on `B`
    /// **and whose pair still collects on `T`**. The boundary is stated in
    /// terms:
    ///
    /// > **It does not reach the *went away* shape, and that is the boundary.**
    /// > … a vanished `xfail` or `skipped` id is allocated below exactly as any
    /// > other vanished id is, and still takes a `class=protected` `G8:<path>`
    /// > review.
    ///
    /// "Collects" and "passes" are different questions, which is why this is
    /// not [`Self::pair_passed`]: an id that ran on `T` and failed has not gone
    /// away, and an id that vanished did — the carve-out turns on presence, and
    /// the finding it releases is about the outcome.
    pub fn pair_collected_on_t(&self, runner: &str, id: &str) -> bool {
        self.results
            .iter()
            .any(|r| r.runner.as_str() == runner && r.id == id)
    }

    pub fn pair_passed(&self, runner: &str, id: &str) -> bool {
        if !self.status.credits_outcomes() {
            return false;
        }
        let mut matches = self
            .results
            .iter()
            .filter(|r| r.runner.as_str() == runner && r.id == id);
        // "exactly one" — and §4.4 already makes a repeated pair malformed, so
        // a parsed file cannot hold two. A file this crate assembled could, if
        // reduction were skipped, and this is where that would show.
        match (matches.next(), matches.next()) {
            (Some(only), None) => only.out.is_pass(),
            _ => false,
        }
    }

    /// RF §6.5: "**Frozen ids roll up, within their runner.** … Let
    /// `P(R, F) = { r ∈ result records : r.runner == R and r.fn == F }`,
    /// compared by exact string equality on both members. `(R, F)` passes iff
    /// `P(R, F)` is non-empty and every member has `out == "passed"`.
    /// `P(R, F)` empty means the frozen entry is absent, which is not a pass."
    ///
    /// The runner qualifier is not optional here: "Neither roll-up looks at the
    /// other runner's records, and neither would find them if it did" (RF §10).
    pub fn frozen_entry_passed(&self, runner: &str, function: &str) -> bool {
        if !self.status.credits_outcomes() {
            return false;
        }
        let mut any = false;
        for record in &self.results {
            if record.runner.as_str() == runner && record.function == function {
                any = true;
                if !record.out.is_pass() {
                    return false;
                }
            }
        }
        any
    }

    /// RF §6.5: "**The `B` floor does not roll up.** … A `base` record `b`
    /// passes iff some `result` record has `r.runner == b.runner` **and**
    /// `r.id == b.id` and `r.out == "passed"`, by exact string equality on
    /// both."
    pub fn floor_holds(&self) -> bool {
        // The status conjunct FIRST, and `all` is exactly why.
        //
        // RF §7.3: "This closes the one place where an empty or near-empty file
        // could pass vacuously: a quick-lane landing has no frozen ids, so its
        // G1 is the `B` floor alone, and a `base-collect-failed` file would
        // otherwise satisfy it by emptiness."
        //
        // `pair_passed` carries the same conjunct, but over an EMPTY floor it
        // is never called — `all` on an empty iterator is `true` — so the guard
        // inside it does not reach this case. A file with `ids=0` and
        // `end.status=base-collect-failed` is precisely the shape §7.3 spends a
        // paragraph closing, and without this line it is the shape that passes.
        if !self.status.credits_outcomes() {
            return false;
        }
        self.base
            .iter()
            .all(|b| self.pair_passed(b.runner.as_str(), &b.id))
    }
}

/// Why the **write** path refused to build a file.
///
/// Distinct from the read path's refusals on purpose: these are this
/// implementation's own bugs caught before they reach a byte, whereas a
/// `result-malformed` on the read side is another implementation's. Neither
/// token ever reaches a wire — a refused run writes no file at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteRefusal {
    /// Two records share a `(runner, id)` pair. RF §11 item 5 makes uniqueness
    /// a writer's obligation, not only a reader's.
    DuplicatePair {
        section: &'static str,
        runner: String,
        id: String,
    },
    /// A status RF §7.3 makes all-or-nothing, carrying records anyway.
    BodyBesideAllOrNothingStatus {
        status: &'static str,
        base: usize,
        results: usize,
    },
}

impl core::fmt::Display for WriteRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WriteRefusal::DuplicatePair {
                section,
                runner,
                id,
            } => write!(
                f,
                "two {section} records share the pair ({runner}, {id}); \
                 RF §11 item 5 makes pair uniqueness the writer's obligation too"
            ),
            WriteRefusal::BodyBesideAllOrNothingStatus {
                status,
                base,
                results,
            } => write!(
                f,
                "status {status} requires ids=0 and no records from any runner \
                 (RF §7.3), but {base} base and {results} result records were supplied"
            ),
        }
    }
}

impl core::error::Error for WriteRefusal {}

/// RF §4.5's sort, and RF §4.4's uniqueness, in one comparison.
///
/// They are one check because they are one relation: a section sorted strictly
/// ascending on `(runner, id)` is exactly a section that is both sorted and
/// free of repeated pairs. Splitting them would need a second pass over the
/// records and would give a file with two faults a report that depends on which
/// pass ran first.
fn check_order(
    previous: Option<(&[u8], &[u8])>,
    current: (&[u8], &[u8]),
    section: Section,
    line: usize,
) -> Result<(), Malformed> {
    let Some(previous) = previous else {
        return Ok(());
    };
    match previous.cmp(&current) {
        core::cmp::Ordering::Less => Ok(()),
        core::cmp::Ordering::Equal => Err(Malformed::DuplicatePair {
            section,
            runner: String::from_utf8_lossy(current.0).into_owned(),
            id: String::from_utf8_lossy(current.1).into_owned(),
        }),
        core::cmp::Ordering::Greater => Err(Malformed::OutOfSortOrder { section, line }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::Profile;
    use crate::outcome::{BaseOutcome, Outcome};
    use crate::record::RunnerToken;

    /// RF §10's worked example, verbatim: "`.spine/cache/results/3f7b…7.jsonl`,
    /// complete, 20 lines". Every test below reads these bytes.
    const VECTOR: &str = include_str!("../tests/vectors/rf-10-complete.jsonl");
    const TREE: &str = "3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28";

    fn vector() -> ResultFile {
        ResultFile::parse(VECTOR.as_bytes(), TREE, ObjectFormat::Sha1)
            .expect("RF §10's file is conforming")
    }

    fn runner(s: &str) -> RunnerToken {
        RunnerToken::new(s).expect("token in grammar")
    }

    fn provenance(profile: Profile) -> Provenance {
        Provenance {
            tree: TREE.into(),
            base: "7b0d4a1f2c3e5d6a8b9c0d1e2f3a4b5c6d7e8f90".into(),
            tool: "1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db"
                .into(),
            keys_visible: false,
            profile,
        }
    }

    /// RF §10 says "20 lines" and prints them. A file that parses but
    /// re-serializes to different bytes is a file two implementations would
    /// hash differently, which is what `evidence.result_sha256` (GR §5.9) is
    /// taken over.
    #[test]
    fn the_worked_example_reproduces_byte_for_byte() {
        let file = vector();
        assert_eq!(VECTOR.lines().count(), 20, "RF §10: twenty lines");
        assert_eq!(
            String::from_utf8(file.to_bytes()).expect("UTF-8 by construction"),
            VECTOR
        );
    }

    /// RF §10: "`ids=7` equals the seven `base` records, four pytest and three
    /// vitest."
    #[test]
    fn the_worked_example_carries_seven_base_pairs_across_two_runners() {
        let file = vector();
        assert_eq!(file.header.ids, 7);
        assert_eq!(file.base.len(), 7);
        assert_eq!(
            file.base
                .iter()
                .filter(|b| b.runner.as_str() == "pytest")
                .count(),
            4
        );
        assert_eq!(
            file.base
                .iter()
                .filter(|b| b.runner.as_str() == "vitest")
                .count(),
            3
        );
        assert_eq!(file.results.len(), 11);
        assert_eq!(file.status, Status::Complete);
    }

    /// RF §10: "Both sections are sorted on `runner` bytes first: every
    /// `pytest` record precedes every `vitest` one, because `p` < `v`. Within
    /// each runner the sort is on `id` bytes, which puts `…[reduced-rate]`
    /// before `[standard-rate]` before `[zero-rate]`, and `…half-even` before
    /// `…half-up`."
    #[test]
    fn both_sections_sort_on_runner_bytes_then_id_bytes() {
        let file = vector();
        let runners: Vec<&str> = file.results.iter().map(|r| r.runner.as_str()).collect();
        let boundary = runners.iter().position(|r| *r == "vitest").expect("vitest");
        assert!(runners[..boundary].iter().all(|r| *r == "pytest"));
        assert!(runners[boundary..].iter().all(|r| *r == "vitest"));

        let vitest_ids: Vec<&str> = file
            .results
            .iter()
            .filter(|r| r.runner.as_str() == "vitest")
            .map(|r| r.id.as_str())
            .collect();
        assert!(
            vitest_ids
                .windows(2)
                .all(|w| w[0].as_bytes() < w[1].as_bytes())
        );
        assert!(
            vitest_ids
                .iter()
                .position(|i| i.ends_with("half-even"))
                .unwrap()
                < vitest_ids
                    .iter()
                    .position(|i| i.ends_with("half-up"))
                    .unwrap()
        );
    }

    /// RF §10: "Frozen `("pytest", …::test_AC1_totals_include_tax)` →
    /// `P(R, F)` is the three parametrized records, all `passed` → pass."
    #[test]
    fn a_frozen_entry_rolls_up_within_its_runner_and_passes() {
        let file = vector();
        assert!(file.frozen_entry_passed(
            "pytest",
            "tests/billing/test_invoice.py::test_AC1_totals_include_tax"
        ));
        assert!(file.frozen_entry_passed(
            "vitest",
            "tests/billing/invoice.test.ts > invoice totals > AC2 zero-rated lines"
        ));
        // "Neither roll-up looks at the other runner's records, and neither
        // would find them if it did."
        assert!(!file.frozen_entry_passed(
            "vitest",
            "tests/billing/test_invoice.py::test_AC1_totals_include_tax"
        ));
    }

    /// RF §10: "The seven `base` pairs each have a `result` record with the
    /// same `runner` **and** the same `id` and `out: passed` → the floor
    /// holds."
    #[test]
    fn the_floor_holds_pair_by_pair() {
        assert!(vector().floor_holds());
    }

    /// RF §10, the second published file: the same run with vitest killed at
    /// the deadline. "`status ≠ complete`, so clause 0 fails and pytest's seven
    /// `passed` records credit nothing."
    #[test]
    fn a_timed_out_run_credits_nothing_including_the_green_runners_records() {
        const TIMED_OUT: &str = include_str!("../tests/vectors/rf-10-runner-timeout.jsonl");
        let file = ResultFile::parse(TIMED_OUT.as_bytes(), TREE, ObjectFormat::Sha1)
            .expect("the timed-out file is well-formed and ingestible");
        assert_eq!(file.status, Status::RunnerTimeout);
        assert_eq!(file.header.ids, 7, "both B collections had succeeded");
        assert_eq!(file.results.len(), 8);
        assert_eq!(
            String::from_utf8(file.to_bytes()).expect("UTF-8"),
            TIMED_OUT
        );

        // pytest completed and its records are all `passed`; none of them is a
        // pass, because the fold is not `complete`.
        assert!(!file.floor_holds());
        assert!(!file.frozen_entry_passed(
            "pytest",
            "tests/billing/test_invoice.py::test_AC1_totals_include_tax"
        ));
        assert!(!file.pair_passed(
            "pytest",
            "tests/billing/test_discounts.py::test_percentage_discount"
        ));
    }

    /// RF §10, "**One `xfail` on trunk, which is the case `out` exists for.**"
    ///
    /// "Its `base` record reads `out: "xfail"` and its `result` record reads
    /// `out: "xfail"` as well. … Every other byte of the file above is the
    /// same, `ids=7` is the same, and the two `out` members that moved are the
    /// only difference."
    #[test]
    fn the_xfail_on_trunk_variation_moves_exactly_two_out_members() {
        let xfailed = VECTOR.replace(r#"discount","out":"passed""#, r#"discount","out":"xfail""#);
        let file = ResultFile::parse(xfailed.as_bytes(), TREE, ObjectFormat::Sha1)
            .expect("the variation is a conforming file");

        assert_eq!(file.header.ids, 7, "ids= is the same");
        assert_eq!(
            xfailed.lines().count(),
            VECTOR.lines().count(),
            "still twenty lines"
        );
        let moved = VECTOR
            .lines()
            .zip(xfailed.lines())
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(
            moved, 2,
            "the two `out` members that moved are the only difference"
        );

        // "Under §8.5 clause 2 it is the *did not pass* shape with
        // `b.out == "xfail"`, so it produces **no finding in either gate**."
        let carved = file
            .base
            .iter()
            .find(|b| b.id == "tests/billing/test_discounts.py::test_percentage_discount")
            .expect("the pair is in the floor");
        assert!(carved.out.was_xfail_or_skipped_on_b());
        // And it is still not a pass: RF §5, "Neither `xfail` nor `skipped` is
        // a pass, and every clause of §8.5 that asks *did this id pass* still
        // answers no. What changed is not the enum but the **allocation**."
        assert!(!file.pair_passed(carved.runner.as_str(), &carved.id));
        assert!(!file.floor_holds());
    }

    /// RF §10: "**Had the two runners collided on an id.** … That is two `base`
    /// records and two `result` records, `ids=` counts two, and neither section
    /// is malformed — the pair is the identity, and a duplicate is a repeated
    /// *pair*."
    #[test]
    fn one_id_under_two_runners_is_two_records_and_no_duplicate() {
        let id = "tests/core/util.test.ts > rounding > half-even";
        let file = ResultFile::new(
            provenance(Profile::Container),
            vec![
                BaseRecord {
                    runner: runner("vitest"),
                    id: id.into(),
                    out: BaseOutcome::Reported(Outcome::Passed),
                    path: "tests/core/util.test.ts".into(),
                },
                BaseRecord {
                    runner: runner("jest"),
                    id: id.into(),
                    out: BaseOutcome::Reported(Outcome::Passed),
                    path: "tests/core/util.test.ts".into(),
                },
            ],
            Vec::new(),
            Status::Complete,
        );
        assert_eq!(file.header.ids, 2);
        // `j` < `v`, so sorting on runner bytes puts jest first whatever order
        // the collector invoked them in.
        assert_eq!(file.base[0].runner.as_str(), "jest");
        let bytes = file.to_bytes();
        assert!(ResultFile::parse(&bytes, TREE, ObjectFormat::Sha1).is_ok());
    }

    /// RF §4.4: "**The pair `(runner, id)` is unique across the section.** A
    /// repeated pair is malformed."
    #[test]
    fn a_repeated_pair_within_one_runner_is_malformed() {
        let line = r#"{"id":"tests/billing/test_discounts.py::test_percentage_discount","out":"passed","path":"tests/billing/test_discounts.py","runner":"pytest","t":"base"}"#;
        let mut lines: Vec<&str> = VECTOR.lines().collect();
        lines.insert(2, line);
        // ids= must still agree, or that is what gets reported instead.
        let text = lines.join("\n").replace("ids=7", "ids=8") + "\n";
        assert!(matches!(
            ResultFile::parse(text.as_bytes(), TREE, ObjectFormat::Sha1),
            Err(Malformed::DuplicatePair {
                section: Section::Base,
                ..
            })
        ));
    }

    /// RF §4.5: "…or a section out of sort order: malformed."
    #[test]
    fn a_section_out_of_sort_order_is_malformed() {
        let mut lines: Vec<&str> = VECTOR.lines().collect();
        lines.swap(1, 2);
        let text = lines.join("\n") + "\n";
        assert!(matches!(
            ResultFile::parse(text.as_bytes(), TREE, ObjectFormat::Sha1),
            Err(Malformed::OutOfSortOrder {
                section: Section::Base,
                line: 3
            })
        ));
    }

    /// RF §4.5: "A `base` record after a `result` record, a record after `end`,
    /// a missing `end`, a second `end` … malformed."
    #[test]
    fn the_four_ordering_faults_are_each_malformed() {
        let lines: Vec<&str> = VECTOR.lines().collect();

        // A base record after a result record.
        let mut moved = lines.clone();
        let first_base = moved.remove(1);
        moved.insert(10, first_base);
        let text = moved.join("\n") + "\n";
        assert!(matches!(
            ResultFile::parse(text.as_bytes(), TREE, ObjectFormat::Sha1),
            Err(Malformed::BaseAfterResult { .. })
        ));

        // A record after `end` — here the last `result` line, repeated.
        let text = format!("{VECTOR}{}\n", lines[18]);
        assert_eq!(
            ResultFile::parse(text.as_bytes(), TREE, ObjectFormat::Sha1),
            Err(Malformed::RecordAfterEnd { line: 21 })
        );

        // A missing `end` — the truncation RF §4.2 says `ids=` is not the guard
        // for.
        let text = lines[..19].join("\n") + "\n";
        assert_eq!(
            ResultFile::parse(text.as_bytes(), TREE, ObjectFormat::Sha1),
            Err(Malformed::MissingEnd)
        );

        // A second `end`.
        let text = format!("{VECTOR}{}\n", r#"{"status":"complete","t":"end"}"#);
        assert_eq!(
            ResultFile::parse(text.as_bytes(), TREE, ObjectFormat::Sha1),
            Err(Malformed::SecondEnd { line: 21 })
        );
    }

    /// RF §4.2: "A count that disagrees with the number of `base` records
    /// present is malformed."
    #[test]
    fn an_ids_count_that_disagrees_with_the_base_records_is_malformed() {
        let text = VECTOR.replace("ids=7", "ids=6");
        assert_eq!(
            ResultFile::parse(text.as_bytes(), TREE, ObjectFormat::Sha1),
            Err(Malformed::IdsDisagreesWithBaseRecords {
                header: 6,
                records: 7
            })
        );
    }

    /// RF §4.1: "A CR (`U+000D`) anywhere outside a JSON string escape makes
    /// the file malformed." A CRLF-translated checkout is how this happens.
    #[test]
    fn a_carriage_return_anywhere_makes_the_file_malformed() {
        let text = VECTOR.replace('\n', "\r\n");
        assert_eq!(
            ResultFile::parse(text.as_bytes(), TREE, ObjectFormat::Sha1),
            Err(Malformed::CarriageReturn { line: 1 })
        );
    }

    /// RF §4.1: "Every line, including the last, is terminated." and "no bytes
    /// after the final LF".
    #[test]
    fn the_final_line_must_be_terminated() {
        let text = VECTOR.trim_end_matches('\n');
        assert_eq!(
            ResultFile::parse(text.as_bytes(), TREE, ObjectFormat::Sha1),
            Err(Malformed::UnterminatedFinalLine)
        );
    }

    /// RF §4.1: "No blank lines, no comment lines, no leading or trailing
    /// whitespace on any line".
    #[test]
    fn a_blank_line_makes_the_file_malformed() {
        let text = VECTOR.replacen('\n', "\n\n", 1);
        assert_eq!(
            ResultFile::parse(text.as_bytes(), TREE, ObjectFormat::Sha1),
            Err(Malformed::BlankLine { line: 2 })
        );
    }

    /// RF §4.3: "**Canonical form is required on read, not only on write.** A
    /// body line that parses as JSON but is not in canonical form is
    /// malformed."
    #[test]
    fn a_body_line_that_parses_but_is_not_canonical_is_malformed() {
        // Member order first: the same object, keys swapped.
        let reordered = r#"{"out":"passed","id":"tests/billing/test_discounts.py::test_percentage_discount","path":"tests/billing/test_discounts.py","runner":"pytest","t":"base"}"#;
        let text = VECTOR.replacen(
            r#"{"id":"tests/billing/test_discounts.py::test_percentage_discount","out":"passed","path":"tests/billing/test_discounts.py","runner":"pytest","t":"base"}"#,
            reordered,
            1,
        );
        assert_eq!(
            ResultFile::parse(text.as_bytes(), TREE, ObjectFormat::Sha1),
            Err(Malformed::LineNotCanonical { line: 2 })
        );

        // Then a non-minimal escape: `t` is `t`, and RF §4.3 says "No
        // other escape is produced and none is accepted." A tolerant reader
        // here would let two byte strings hash to one file's meaning, which is
        // exactly what `evidence.result_sha256` cannot survive.
        let escaped = r#"{"id":"tests/billing/test_discounts.py::test_percentage_discount","out":"passed","path":"tests/billing/test_discounts.py","runner":"pytest","t":"\u0062ase"}"#;
        let text = VECTOR.replacen(
            r#"{"id":"tests/billing/test_discounts.py::test_percentage_discount","out":"passed","path":"tests/billing/test_discounts.py","runner":"pytest","t":"base"}"#,
            escaped,
            1,
        );
        assert_eq!(
            ResultFile::parse(text.as_bytes(), TREE, ObjectFormat::Sha1),
            Err(Malformed::LineNotCanonical { line: 2 })
        );
    }

    /// RF §4.4 makes duplicate keys malformed, and `spine-canon`'s parser is
    /// where that rule already lives (GR §2.2: "A parser that meets one refuses
    /// the document").
    #[test]
    fn a_duplicate_member_within_a_record_is_malformed() {
        let dup = r#"{"id":"a","id":"b","out":"passed","path":"","runner":"pytest","t":"base"}"#;
        let text = format!(
            "tree={TREE} base=7b0d4a1f2c3e5d6a8b9c0d1e2f3a4b5c6d7e8f90 \
tool=1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db \
keys_visible=false profile=none ids=1\n{dup}\n{{\"status\":\"complete\",\"t\":\"end\"}}\n"
        );
        assert!(matches!(
            ResultFile::parse(text.as_bytes(), TREE, ObjectFormat::Sha1),
            Err(Malformed::LineNotJson { line: 2, .. })
        ));
    }

    /// RF §7.3's `base-collect-failed` body: "`ids=0`, and **no `base` and no
    /// `result` records are written at all**, from any runner."
    #[test]
    fn a_base_collect_failed_file_is_a_header_and_an_end_record() {
        let file = ResultFile::new(
            provenance(Profile::None),
            Vec::new(),
            Vec::new(),
            Status::BaseCollectFailed,
        );
        let bytes = file.to_bytes();
        assert_eq!(String::from_utf8_lossy(&bytes).lines().count(), 2);
        assert!(String::from_utf8_lossy(&bytes).contains(" ids=0\n"));
        let read = ResultFile::parse(&bytes, TREE, ObjectFormat::Sha1).expect("well formed");
        // RF §7.3: "This closes the one place where an empty or near-empty file
        // could pass vacuously: a quick-lane landing has no frozen ids, so its
        // G1 is the `B` floor alone, and a `base-collect-failed` file would
        // otherwise satisfy it by emptiness."
        //
        // This assertion is the closure. It read `assert!(read.floor_holds())`
        // until 2026-08-28 — pinning the vacuity rather than the rule — on the
        // reasoning that `pair_passed` carries the status conjunct. It does,
        // and over an empty floor it is never called, because `all` on an empty
        // iterator is `true`. The one file shape §7.3 names was the one shape
        // the guard could not reach.
        assert!(
            !read.floor_holds(),
            "a file that collected no floor at all must not satisfy the floor"
        );
        assert!(
            !read.status.credits_outcomes(),
            "and it is the status that decides it, not the emptiness"
        );

        // The rule is about what the status credits, not about being empty: a
        // `complete` file with a genuinely empty floor is a different thing and
        // still holds.
        let empty_but_complete = ResultFile::new(
            provenance(Profile::None),
            Vec::new(),
            Vec::new(),
            Status::Complete,
        );
        assert!(empty_but_complete.floor_holds());
    }
}
