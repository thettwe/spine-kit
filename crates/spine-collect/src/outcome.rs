//! RF §5's outcome vocabulary — closed, runner-independent, and the one enum
//! other specs reference.
//!
//! "This enum is closed and is the one other specs reference." Every adapter
//! "maps its own runner's terminal reports onto these eight and no others
//! (§6)". A runner with no notion of expected failure simply never produces
//! `xfail` or `xpass`; RF §5 says in terms that "that is not a gap in the
//! enum", which is why there is no ninth value and no escape hatch beyond
//! [`Outcome::Unknown`].
//!
//! The value that is *not* in the enum lives here too. RF §4.4: "`absent` means
//! the `B` outcome run reported no terminal outcome for the pair … It is not
//! `unknown`, and the two must not be merged: `unknown` is a terminal report
//! the adapter could not map, `absent` is no terminal report at all." Keeping
//! them in two types is how this crate makes merging them impossible rather
//! than merely discouraged.

use core::fmt;

/// RF §5's eight values, in the document's own table order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Outcome {
    /// "The runner ran the id and reported it passed, with no expected-failure
    /// marker in play." The only value that is ever a pass.
    Passed,
    /// "The id ran and its assertion phase failed."
    Failed,
    /// "The id could not run, or failed outside its assertion phase: a
    /// collection error, a setup or teardown error, an import failure."
    Error,
    /// "The id was collected and not run, by a marker, a runtime skip, or an
    /// environment condition."
    Skipped,
    /// "The id was declared an expected failure and did not pass."
    Xfail,
    /// "The id was declared an expected failure and passed." RF §13 R4 keeps it
    /// out of the pass column: "A record whose declared polarity is 'expected
    /// to fail' is not evidence that an acceptance criterion holds, whatever it
    /// did this run."
    Xpass,
    /// "The id was collected and excluded before running — a selection
    /// expression, a collection hook." Distinguished from absence on purpose:
    /// "a `deselected` record is a *collected* id and satisfies the AC-coverage
    /// clause of §8.5 where an absent one does not" (RF §5).
    Deselected,
    /// "The runner reported a terminal outcome the collector's adapter does not
    /// map." RF §6.3 obligation 4 makes it "the defined home for anything
    /// unmapped", so an adapter's mapping is total by construction.
    Unknown,
}

impl Outcome {
    /// The wire spelling. These bytes are in the file a gate reads, so they are
    /// fixed here and nowhere else.
    pub fn token(self) -> &'static str {
        match self {
            Outcome::Passed => "passed",
            Outcome::Failed => "failed",
            Outcome::Error => "error",
            Outcome::Skipped => "skipped",
            Outcome::Xfail => "xfail",
            Outcome::Xpass => "xpass",
            Outcome::Deselected => "deselected",
            Outcome::Unknown => "unknown",
        }
    }

    /// RF §4.4: "unknown `out` … values are all malformed — there is no
    /// forward-compatibility relaxation". `None` is therefore an error and
    /// never a value to fall back on.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "passed" => Some(Outcome::Passed),
            "failed" => Some(Outcome::Failed),
            "error" => Some(Outcome::Error),
            "skipped" => Some(Outcome::Skipped),
            "xfail" => Some(Outcome::Xfail),
            "xpass" => Some(Outcome::Xpass),
            "deselected" => Some(Outcome::Deselected),
            "unknown" => Some(Outcome::Unknown),
            _ => None,
        }
    }

    /// Half of RF §5's mapping, and it is only half.
    ///
    /// "G1 counts a pair `(R, i)` as passed **iff** the body contains exactly
    /// one `result` record with `runner == R` and `id == i` whose `out` is
    /// `passed`, **and** the `end` record's `status` is `complete`." This
    /// answers the first conjunct only; the second is
    /// [`crate::record::Status::credits_outcomes`], and a caller that reads
    /// this one alone would credit a killed run's green half, which RF §7.3
    /// forbids in terms.
    pub fn is_pass(self) -> bool {
        matches!(self, Outcome::Passed)
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// What a `base` record's `out` can hold: one of RF §5's eight, or `absent`.
///
/// RF §4.4 gives `absent` exactly one home — "legal on a `base` record and on
/// no other kind" — and exactly one reason to exist: "a `base` record cannot be
/// omitted: the id is in the floor whatever the `B` outcome run said about it."
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BaseOutcome {
    /// The `B` outcome run reported a terminal outcome, mapped onto RF §5.
    Reported(Outcome),
    /// "the `B` outcome run reported no terminal outcome for the pair."
    ///
    /// RF §7.1: where an adapter's enumeration and outcome run are two
    /// invocations, this "is the fail-closed value for every id the outcome run
    /// did not reach", including every id left behind when that run is killed
    /// at the deadline.
    Absent,
}

impl BaseOutcome {
    pub fn token(self) -> &'static str {
        match self {
            BaseOutcome::Reported(o) => o.token(),
            BaseOutcome::Absent => "absent",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        if s == "absent" {
            return Some(BaseOutcome::Absent);
        }
        Outcome::parse(s).map(BaseOutcome::Reported)
    }

    /// The **only** question `base.out` is ever asked.
    ///
    /// RF §4.4: "It has exactly one consumer, §8.5 clause 2's `xfail`/`skipped`
    /// carve-out … The single question it answers is *was this id already
    /// `xfail` or `skipped` on trunk*, and every other value — `absent`
    /// included — answers it identically."
    ///
    /// That is why this returns `bool` and not the outcome: exposing the value
    /// to a caller invites a second consumer, and RF §4.4 says there is exactly
    /// one. `out` on a `base` record "is never a pass and never evidence".
    ///
    /// **This is one conjunct of clause 2, not the clause.** It was named
    /// `exempts_from_findings` until 2026-08-28, which claimed the whole
    /// carve-out and would have let a caller exempt a *deleted* test. RF §8.5:
    ///
    /// > **It does not reach the *went away* shape, and that is the boundary.**
    /// > … a vanished `xfail` or `skipped` id is allocated below exactly as any
    /// > other vanished id is, and still takes a `class=protected` `G8:<path>`
    /// > review.
    ///
    /// So clause 2 is *this* **and** the pair still collecting on `T`. The
    /// second conjunct is a property of the file, not of the outcome, and lives
    /// with the file: [`crate::ResultFile::pair_collected_on_t`]. A gate layer
    /// that reads this predicate alone exempts a deleted `xfail` from the
    /// review that is the whole point of the boundary — the fail-open
    /// direction, which is why the name now says only what it decides.
    pub fn was_xfail_or_skipped_on_b(self) -> bool {
        matches!(
            self,
            BaseOutcome::Reported(Outcome::Xfail) | BaseOutcome::Reported(Outcome::Skipped)
        )
    }
}

impl fmt::Display for BaseOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Outcome; 8] = [
        Outcome::Passed,
        Outcome::Failed,
        Outcome::Error,
        Outcome::Skipped,
        Outcome::Xfail,
        Outcome::Xpass,
        Outcome::Deselected,
        Outcome::Unknown,
    ];

    /// RF §5's table, in its own order. The enum is closed at eight; a ninth
    /// value would be a value some other implementation's parser refuses.
    #[test]
    fn the_enum_is_eight_tokens_in_the_specs_table_order() {
        let tokens: Vec<&str> = ALL.iter().map(|o| o.token()).collect();
        assert_eq!(
            tokens.join(" "),
            "passed failed error skipped xfail xpass deselected unknown"
        );
    }

    #[test]
    fn every_token_round_trips_and_nothing_else_parses() {
        for o in ALL {
            assert_eq!(Outcome::parse(o.token()), Some(o));
        }
        for bad in ["", "pass", "PASSED", "absent", "xFail", "errored"] {
            assert_eq!(Outcome::parse(bad), None, "{bad}");
        }
    }

    /// RF §5: "`passed` is the only value that is ever a pass, in any lane, in
    /// any mode, for any gate, under any runner."
    #[test]
    fn passed_is_the_only_value_that_is_ever_a_pass() {
        for o in ALL {
            assert_eq!(o.is_pass(), o == Outcome::Passed, "{o}");
        }
    }

    /// RF §5, R4: "**`xpass` is not a pass**."
    #[test]
    fn xpass_is_not_a_pass() {
        assert!(!Outcome::Xpass.is_pass());
    }

    /// RF §4.4: `absent` "is not `unknown`, and the two must not be merged".
    #[test]
    fn absent_is_not_unknown() {
        assert_ne!(
            BaseOutcome::parse("absent"),
            BaseOutcome::parse("unknown"),
            "absent and unknown are distinct values of base.out"
        );
        assert_eq!(BaseOutcome::parse("absent"), Some(BaseOutcome::Absent));
        assert_eq!(
            BaseOutcome::parse("unknown"),
            Some(BaseOutcome::Reported(Outcome::Unknown))
        );
    }

    /// RF §5's carve-out is over two values and no others, and RF §4.4 adds
    /// that "every other value — `absent` included — answers it identically".
    #[test]
    fn only_xfail_and_skipped_on_b_exempt_a_pair_from_findings() {
        for o in ALL {
            let exempt = BaseOutcome::Reported(o).was_xfail_or_skipped_on_b();
            assert_eq!(exempt, o == Outcome::Xfail || o == Outcome::Skipped, "{o}");
        }
        assert!(!BaseOutcome::Absent.was_xfail_or_skipped_on_b());
    }
}
