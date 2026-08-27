//! RF §4.4's three record kinds, and the `end` record's closed status set.
//!
//! "Exactly three kinds. Every record carries `t`. **Unknown `t` values,
//! unknown keys, missing keys and unknown `out`/`status`/`runner` values are
//! all malformed** — there is no forward-compatibility relaxation, because
//! §7.4 rule 3 already refuses a header whose `tool=` is not the base's pin, so
//! writer and reader are the same build by construction."
//!
//! That last clause is why every reader here refuses rather than skips. A
//! tolerant parser would be tolerant of exactly one thing: a file no conforming
//! collector can produce.

use crate::malformed::Malformed;
use crate::outcome::{BaseOutcome, Outcome};
use core::fmt;
use spine_canon::{Value, canonicalize_to_string};

/// The `runner` token — RF §4.4's lexical form, `[a-z][a-z0-9_-]{0,31}`.
///
/// The three exclusions are each load-bearing and each cited by RF §4.4:
///
/// - **No uppercase**, "so byte order and case-insensitive order coincide and
///   two spellings of one runner cannot both exist" — which is what makes
///   §4.5's sort on `runner` bytes a total order over the adapters.
/// - **No `U+0020`**, "because §4.3's `Spine-Test` payload is `<runner>`
///   `U+0020` `<function id>` and the function id may itself contain spaces
///   (vitest's `>`-joined names), so the split is at the **first** space and
///   only a space-free token makes it exact."
/// - **No `U+003A`**, "because §6.2 spells a `test` node id `test:<runner>:<id>`
///   and only a colon-free token makes *that* split exact."
///
/// The *set* of tokens is `import-resolver.md`'s, never this crate's: RF §6.4
/// says "an implementer must not infer either from the examples here", and a
/// token is permanent because "`Spine-Test` lines carrying it are sealed into
/// landings forever".
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RunnerToken(String);

impl RunnerToken {
    /// `None` for any value outside the grammar. RF §4.4: "A value outside it
    /// is malformed."
    pub fn new(s: &str) -> Option<Self> {
        let mut bytes = s.bytes();
        match bytes.next() {
            Some(b'a'..=b'z') => {}
            _ => return None,
        }
        // The `{0,31}` tail: at most 31 bytes after the leading letter.
        if s.len() > 32 {
            return None;
        }
        for b in bytes {
            if !matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_') {
                return None;
            }
        }
        Some(RunnerToken(s.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RunnerToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// RF §7.3's closed status set, **in the table's own order**.
///
/// The order is the whole of the fold: "`end.status` is `complete` **iff
/// every** invoked runner contributed `complete`. Otherwise it is the **first
/// row in this table's order, after `complete`, contributed by any runner**.
/// The fold is over the table's fixed order and not over invocation order or
/// wall time, so it is deterministic and independent of which runner ran
/// first." `Ord` here is declaration order, so the fold is a `min`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    /// "The runner terminated of its own accord and its stream was parsed to a
    /// terminal event."
    ///
    /// RF §7.3 adds the two conjuncts and the one non-conjunct: "`complete`
    /// requires **both** that the adapter parsed that runner's terminal
    /// session-end event **and** that no member of its process group was
    /// terminated by a signal … The runner's *exit code* is never the
    /// discriminator — a red suite exits non-zero on every runner that ships,
    /// so an exit-code test would make `complete` unreachable for exactly the
    /// runs G1 exists to judge."
    Complete,
    /// "The **enumeration** of the id set on the checkout of `B` failed, or its
    /// deadline expired during that enumeration. A failure of the separate `B`
    /// **outcome** run, where an adapter has one, is *not* this row."
    BaseCollectFailed,
    /// "The runner could not be started at all — no runner configuration in the
    /// tree under test included."
    SpawnFailed,
    /// "The runner started and terminated but emitted no parsable stream
    /// event."
    NoOutput,
    /// "Its stream contained an event the adapter cannot parse, or an id that
    /// is not valid UTF-8."
    StreamInvalid,
    /// "The runner terminated abnormally, or its stream ended mid-record."
    RunnerFailed,
    /// "The collector's deadline expired on the `T` run and it killed that
    /// process group."
    RunnerTimeout,
}

impl Status {
    pub fn token(self) -> &'static str {
        match self {
            Status::Complete => "complete",
            Status::BaseCollectFailed => "base-collect-failed",
            Status::SpawnFailed => "spawn-failed",
            Status::NoOutput => "no-output",
            Status::StreamInvalid => "stream-invalid",
            Status::RunnerFailed => "runner-failed",
            Status::RunnerTimeout => "runner-timeout",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "complete" => Some(Status::Complete),
            "base-collect-failed" => Some(Status::BaseCollectFailed),
            "spawn-failed" => Some(Status::SpawnFailed),
            "no-output" => Some(Status::NoOutput),
            "stream-invalid" => Some(Status::StreamInvalid),
            "runner-failed" => Some(Status::RunnerFailed),
            "runner-timeout" => Some(Status::RunnerTimeout),
            _ => None,
        }
    }

    /// RF §7.3: "**`status ≠ complete` ⇒ no pair counts as passed**, whatever
    /// any `result` record says, and whichever runner produced it. Records are
    /// still written because they are evidence for the human who will read the
    /// wire; they are never credit."
    ///
    /// RF §10 spends a worked example on the consequence: one runner timing out
    /// makes the *other* runner's seven green records "as unaccounted-for as"
    /// the killed runner's. A gate that skips this check credits them.
    pub fn credits_outcomes(self) -> bool {
        matches!(self, Status::Complete)
    }

    /// The third column of RF §7.3's table: whether this contribution's runner
    /// gets its `result` records into the body.
    ///
    /// The rule is the collector's to enforce and not the adapter's to promise.
    /// A `spawn-failed`, `no-output` or `stream-invalid` runner contributes
    /// "Its `base` records; no `result` records" — and for `stream-invalid`
    /// RF §7.2 spells out why: an unrepresentable id means "**That runner**
    /// contributes **no** `result` records at all", so records parsed either
    /// side of the bad one are dropped with it.
    pub fn keeps_result_records(self) -> bool {
        match self {
            Status::Complete | Status::RunnerFailed | Status::RunnerTimeout => true,
            Status::SpawnFailed | Status::NoOutput | Status::StreamInvalid => false,
            // RF §7.3: "**Nothing, from any runner**". Handled globally by the
            // all-or-nothing rule in `collector`, never per-runner here.
            Status::BaseCollectFailed => false,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// RF §4.4: "**`base` — one per `(runner, id)` pair collected on `B`.**"
///
/// ```text
/// {"id":"<runner-native id>","out":"<outcome on B>","path":"<repo-relative path>","runner":"<runner token>","t":"base"}
/// ```
///
/// There is no `fn` here, and that is a rule rather than an omission: "the `B`
/// floor matches on full ids within a runner, never by roll-up (§6)."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseRecord {
    /// "the token of the adapter that collected it".
    pub runner: RunnerToken,
    /// "the full runner-native id as collected on a checkout of `B`,
    /// **including any parametrization suffix**. Non-empty."
    pub id: String,
    /// "**the id's own outcome on the checkout of `B`**".
    pub out: BaseOutcome,
    /// "the repo-relative, `/`-separated path of the file the id was collected
    /// from, **byte for byte as git stores it in `B`'s tree**."
    ///
    /// RF §4.4 fixes both halves of why: "a macOS runner reports NFD where git
    /// stores NFC, and the `G8:<path>` a finding cites has to be the tree's
    /// spelling or it names nothing", and "No tree entry matches: the empty
    /// string. An id with an empty `path` can never satisfy G1's `G8:<path>`
    /// exemption, which is the fail-closed direction."
    pub path: String,
}

/// RF §4.4: "**`result` — one per `(runner, id)` pair a runner reported on
/// `T`.**"
///
/// ```text
/// {"fn":"<function id>","id":"<runner-native id>","out":"<outcome>","path":"<repo-relative path>","runner":"<runner token>","t":"result"}
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultRecord {
    pub runner: RunnerToken,
    pub id: String,
    /// The JSON member is `fn`, which Rust reserves. Same string, different
    /// spelling, and the serializer below is the only place the two meet.
    ///
    /// RF §4.4: "the runner-native **function** id the parametrization rolls up
    /// to (§6). Non-empty, and **a prefix of `id`**; equal to `id` when the id
    /// is not parametrized." RF §6.5 adds where it is computed and why: "`fn`
    /// is computed by the collector, not by the trusted stage" — the trusted
    /// stage "is deliberately **runner**-unaware: it never parses a
    /// runner-native id, and it treats `fn` as an opaque string grouped by
    /// equality **within a `runner` value**."
    pub function: String,
    pub out: Outcome,
    /// "as above, resolved against **`T`'s** tree and emitted as `T`'s bytes.
    /// May differ from the same pair's `base` record".
    pub path: String,
}

/// RF §4.4: "**`end` — exactly one, the last line.**"
///
/// "The runner's exit code and signal are deliberately not recorded: they are
/// platform-divergent, no gate reads them, and `status` already carries every
/// distinction the mechanism makes. **There is no per-runner status record**."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndRecord {
    pub status: Status,
}

/// The `(runner, id)` pair RF §1 calls the identity: "A repository may run
/// several runners, so a test id alone is not an identity."
///
/// Borrowed rather than owned so that a sort key costs no allocation; §4.5
/// sorts "by the **bytes** of `runner`, then by the **bytes** of `id`", which
/// is exactly this tuple's `Ord` on `&[u8]`.
pub(crate) fn sort_key<'a>(runner: &'a RunnerToken, id: &'a str) -> (&'a [u8], &'a [u8]) {
    (runner.as_str().as_bytes(), id.as_bytes())
}

impl BaseRecord {
    pub fn to_value(&self) -> Value {
        Value::obj([
            ("id", Value::str(&self.id)),
            ("out", Value::str(self.out.token())),
            ("path", Value::str(&self.path)),
            ("runner", Value::str(self.runner.as_str())),
            ("t", Value::str("base")),
        ])
    }

    /// One body line, without its LF. Canonical by construction: the bytes come
    /// from `spine-canon`'s RFC 8785 serializer, which is what RF §4.3 calls
    /// "RFC 8785-compatible over the value space this file uses".
    pub fn to_line(&self) -> String {
        canonicalize_to_string(&self.to_value())
    }

    pub(crate) fn from_value(value: &Value, line: usize) -> Result<Self, Malformed> {
        let fields = Fields::new(value, line, &["id", "out", "path", "runner", "t"])?;
        fields.kind("base")?;
        let out_token = fields.get("out")?;
        let out = BaseOutcome::parse(out_token).ok_or_else(|| Malformed::UnknownOutcome {
            line,
            out: out_token.to_owned(),
        })?;
        Ok(BaseRecord {
            runner: fields.runner()?,
            id: fields.non_empty("id")?.to_owned(),
            out,
            // `path` is deliberately absent from `non_empty`: RF §4.4 makes the
            // empty string its value for "no tree entry matches".
            path: fields.get("path")?.to_owned(),
        })
    }

    pub(crate) fn key(&self) -> (&[u8], &[u8]) {
        sort_key(&self.runner, &self.id)
    }
}

impl ResultRecord {
    pub fn to_value(&self) -> Value {
        Value::obj([
            ("fn", Value::str(&self.function)),
            ("id", Value::str(&self.id)),
            ("out", Value::str(self.out.token())),
            ("path", Value::str(&self.path)),
            ("runner", Value::str(self.runner.as_str())),
            ("t", Value::str("result")),
        ])
    }

    pub fn to_line(&self) -> String {
        canonicalize_to_string(&self.to_value())
    }

    pub(crate) fn from_value(value: &Value, line: usize) -> Result<Self, Malformed> {
        let fields = Fields::new(value, line, &["fn", "id", "out", "path", "runner", "t"])?;
        fields.kind("result")?;
        let out_token = fields.get("out")?;
        // RF §4.4: "`absent` is a legal `out` on a `base` record and an unknown
        // value — hence malformed — on a `result` one." Reported separately
        // from `UnknownOutcome` because the two mean different mistakes: a
        // typo, against a collector that merged the two kinds' domains.
        if out_token == "absent" {
            return Err(Malformed::AbsentOutcomeOnResult { line });
        }
        let out = Outcome::parse(out_token).ok_or_else(|| Malformed::UnknownOutcome {
            line,
            out: out_token.to_owned(),
        })?;
        let id = fields.non_empty("id")?.to_owned();
        let function = fields.non_empty("fn")?.to_owned();
        // RF §4.4: "A `result` record whose `fn` is not a prefix of its `id` is
        // malformed. The prefix test is within the record and therefore within
        // one runner; `fn` is never compared across `runner` values (§6)."
        if !id.starts_with(&function) {
            return Err(Malformed::FnNotPrefixOfId { line });
        }
        Ok(ResultRecord {
            runner: fields.runner()?,
            id,
            function,
            out,
            path: fields.get("path")?.to_owned(),
        })
    }

    pub(crate) fn key(&self) -> (&[u8], &[u8]) {
        sort_key(&self.runner, &self.id)
    }
}

impl EndRecord {
    pub fn to_value(self) -> Value {
        Value::obj([
            ("status", Value::str(self.status.token())),
            ("t", Value::str("end")),
        ])
    }

    pub fn to_line(self) -> String {
        canonicalize_to_string(&self.to_value())
    }

    pub(crate) fn from_value(value: &Value, line: usize) -> Result<Self, Malformed> {
        let fields = Fields::new(value, line, &["status", "t"])?;
        fields.kind("end")?;
        let token = fields.get("status")?;
        let status = Status::parse(token).ok_or_else(|| Malformed::UnknownStatus {
            line,
            status: token.to_owned(),
        })?;
        Ok(EndRecord { status })
    }
}

/// A record's members, checked against the kind's exact key set.
///
/// RF §4.4 makes "unknown keys" and "missing keys" malformed with no
/// relaxation, so this checks both directions at once rather than looking up
/// what it happens to need.
struct Fields<'a> {
    members: &'a [(String, Value)],
    line: usize,
}

impl<'a> Fields<'a> {
    fn new(value: &'a Value, line: usize, expected: &[&'static str]) -> Result<Self, Malformed> {
        let Value::Obj(members) = value else {
            return Err(Malformed::LineNotAnObject { line });
        };
        for (name, _) in members {
            if !expected.contains(&name.as_str()) {
                return Err(Malformed::UnknownKey {
                    line,
                    key: name.clone(),
                });
            }
        }
        for key in expected {
            if !members.iter().any(|(name, _)| name == key) {
                return Err(Malformed::MissingKey { line, key });
            }
        }
        Ok(Fields { members, line })
    }

    fn get(&self, key: &'static str) -> Result<&'a str, Malformed> {
        let value = self
            .members
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, v)| v)
            .ok_or(Malformed::MissingKey {
                line: self.line,
                key,
            })?;
        // RF §4.3 rule 4: "v1 record kinds contain string values only." A
        // number, a bool or a nested object here is not a surprising record, it
        // is a record no conforming collector wrote.
        value.as_str().ok_or(Malformed::NonStringValue {
            line: self.line,
            key,
        })
    }

    fn non_empty(&self, key: &'static str) -> Result<&'a str, Malformed> {
        let value = self.get(key)?;
        if value.is_empty() {
            return Err(Malformed::EmptyValue {
                line: self.line,
                key,
            });
        }
        Ok(value)
    }

    /// `t` is a member like any other and its value is a string like any
    /// other. Checked here as well as at dispatch because a record kind that
    /// only ever validates `t` on the way *in* to the match would let a
    /// directly-constructed record carry `{"t":7}` — and RF §4.3 rule 4 admits
    /// "string values only".
    fn kind(&self, expected: &'static str) -> Result<(), Malformed> {
        let t = self.get("t")?;
        if t == expected {
            return Ok(());
        }
        Err(Malformed::UnknownRecordKind {
            line: self.line,
            t: t.to_owned(),
        })
    }

    fn runner(&self) -> Result<RunnerToken, Malformed> {
        let token = self.get("runner")?;
        RunnerToken::new(token).ok_or_else(|| Malformed::RunnerTokenOutOfGrammar {
            line: self.line,
            runner: token.to_owned(),
        })
    }
}

/// Which of RF §4.4's three kinds a body line claims to be.
pub(crate) fn record_kind(value: &Value, line: usize) -> Result<&str, Malformed> {
    let Value::Obj(members) = value else {
        return Err(Malformed::LineNotAnObject { line });
    };
    let member = members
        .iter()
        .find(|(name, _)| name == "t")
        .map(|(_, v)| v)
        .ok_or(Malformed::MissingKey { line, key: "t" })?;
    member
        .as_str()
        .ok_or(Malformed::NonStringValue { line, key: "t" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use spine_canon::parse;

    fn runner(s: &str) -> RunnerToken {
        RunnerToken::new(s).expect("test token is in grammar")
    }

    /// RF §4.4's grammar is `[a-z][a-z0-9_-]{0,31}`: one leading lowercase
    /// letter, then at most thirty-one more bytes.
    #[test]
    fn the_runner_token_grammar_is_one_letter_then_thirty_one_more_bytes() {
        assert!(RunnerToken::new("a").is_some());
        assert!(RunnerToken::new(&format!("a{}", "z".repeat(31))).is_some());
        assert!(RunnerToken::new(&format!("a{}", "z".repeat(32))).is_none());
        assert!(RunnerToken::new("").is_none());
        assert!(RunnerToken::new("1pytest").is_none(), "must start a-z");
        assert!(RunnerToken::new("-pytest").is_none(), "must start a-z");
        assert!(RunnerToken::new("dart_test").is_some());
        assert!(RunnerToken::new("dart-test").is_some());
    }

    /// The three exclusions RF §4.4 argues for: uppercase would let one runner
    /// have two spellings, a space would break `Spine-Test`'s first-space
    /// split, and a colon would break `test:<runner>:<id>`.
    #[test]
    fn a_runner_token_admits_no_uppercase_no_space_and_no_colon() {
        assert!(RunnerToken::new("Pytest").is_none());
        assert!(RunnerToken::new("pyTest").is_none());
        assert!(RunnerToken::new("py test").is_none());
        assert!(RunnerToken::new("py:test").is_none());
    }

    /// RF §7.3's table order *is* the fold's priority order, so `Ord` must be
    /// declaration order and declaration order must be the table.
    #[test]
    fn the_status_set_is_seven_tokens_in_the_tables_order() {
        const IN_TABLE_ORDER: [Status; 7] = [
            Status::Complete,
            Status::BaseCollectFailed,
            Status::SpawnFailed,
            Status::NoOutput,
            Status::StreamInvalid,
            Status::RunnerFailed,
            Status::RunnerTimeout,
        ];
        let mut sorted = IN_TABLE_ORDER;
        sorted.sort();
        assert_eq!(sorted, IN_TABLE_ORDER);

        let tokens: Vec<&str> = IN_TABLE_ORDER.iter().map(|s| s.token()).collect();
        assert_eq!(
            tokens.join(" "),
            "complete base-collect-failed spawn-failed no-output stream-invalid \
             runner-failed runner-timeout"
        );
        for s in IN_TABLE_ORDER {
            assert_eq!(Status::parse(s.token()), Some(s));
        }
        assert_eq!(Status::parse("timeout"), None);
    }

    /// RF §7.3's third column, row by row.
    #[test]
    fn three_statuses_suppress_their_runners_result_records() {
        assert!(Status::Complete.keeps_result_records());
        assert!(Status::RunnerFailed.keeps_result_records());
        assert!(Status::RunnerTimeout.keeps_result_records());
        assert!(!Status::SpawnFailed.keeps_result_records());
        assert!(!Status::NoOutput.keeps_result_records());
        assert!(!Status::StreamInvalid.keeps_result_records());
        assert!(!Status::BaseCollectFailed.keeps_result_records());
    }

    /// RF §10's first `base` line, byte for byte. The record type has to lay
    /// its members down in the order canonical JSON puts them, not the order
    /// the struct declares them.
    #[test]
    fn a_base_record_serializes_to_the_specs_own_bytes() {
        let record = BaseRecord {
            runner: runner("pytest"),
            id: "tests/billing/test_discounts.py::test_percentage_discount".into(),
            out: BaseOutcome::Reported(Outcome::Passed),
            path: "tests/billing/test_discounts.py".into(),
        };
        assert_eq!(
            record.to_line(),
            r#"{"id":"tests/billing/test_discounts.py::test_percentage_discount","out":"passed","path":"tests/billing/test_discounts.py","runner":"pytest","t":"base"}"#
        );
    }

    /// RF §10's second `result` line, byte for byte — the parametrized one,
    /// where `fn` is a proper prefix of `id`.
    #[test]
    fn a_result_record_serializes_to_the_specs_own_bytes() {
        let record = ResultRecord {
            runner: runner("pytest"),
            id: "tests/billing/test_invoice.py::test_AC1_totals_include_tax[reduced-rate]".into(),
            function: "tests/billing/test_invoice.py::test_AC1_totals_include_tax".into(),
            out: Outcome::Passed,
            path: "tests/billing/test_invoice.py".into(),
        };
        assert_eq!(
            record.to_line(),
            r#"{"fn":"tests/billing/test_invoice.py::test_AC1_totals_include_tax","id":"tests/billing/test_invoice.py::test_AC1_totals_include_tax[reduced-rate]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"result"}"#
        );
    }

    #[test]
    fn the_end_record_serializes_to_the_specs_own_bytes() {
        assert_eq!(
            EndRecord {
                status: Status::Complete
            }
            .to_line(),
            r#"{"status":"complete","t":"end"}"#
        );
        assert_eq!(
            EndRecord {
                status: Status::RunnerTimeout
            }
            .to_line(),
            r#"{"status":"runner-timeout","t":"end"}"#
        );
    }

    fn value_of(line: &str) -> Value {
        parse(line.as_bytes()).expect("test fixture is JSON")
    }

    #[test]
    fn a_base_record_round_trips_through_its_bytes() {
        let line = r#"{"id":"a::b","out":"absent","path":"a.py","runner":"pytest","t":"base"}"#;
        let record = BaseRecord::from_value(&value_of(line), 2).expect("conforming record");
        assert_eq!(record.out, BaseOutcome::Absent);
        assert_eq!(record.to_line(), line);
    }

    /// RF §4.4: `absent` is "an unknown value — hence malformed — on a `result`
    /// one".
    #[test]
    fn absent_on_a_result_record_is_malformed() {
        let line = r#"{"fn":"a::b","id":"a::b","out":"absent","path":"a.py","runner":"pytest","t":"result"}"#;
        assert_eq!(
            ResultRecord::from_value(&value_of(line), 9),
            Err(Malformed::AbsentOutcomeOnResult { line: 9 })
        );
    }

    /// RF §4.4: "A `result` record whose `fn` is not a prefix of its `id` is
    /// malformed."
    #[test]
    fn a_result_record_whose_fn_is_not_a_prefix_of_its_id_is_malformed() {
        let line = r#"{"fn":"other","id":"a::b","out":"passed","path":"a.py","runner":"pytest","t":"result"}"#;
        assert_eq!(
            ResultRecord::from_value(&value_of(line), 4),
            Err(Malformed::FnNotPrefixOfId { line: 4 })
        );
    }

    /// RF §4.4: "unknown keys, missing keys … are all malformed — there is no
    /// forward-compatibility relaxation".
    #[test]
    fn an_unknown_key_and_a_missing_key_are_both_malformed() {
        let extra =
            r#"{"id":"a","out":"passed","path":"","runner":"pytest","t":"base","why":"later"}"#;
        assert_eq!(
            BaseRecord::from_value(&value_of(extra), 2),
            Err(Malformed::UnknownKey {
                line: 2,
                key: "why".into()
            })
        );
        let missing = r#"{"id":"a","out":"passed","runner":"pytest","t":"base"}"#;
        assert_eq!(
            BaseRecord::from_value(&value_of(missing), 2),
            Err(Malformed::MissingKey {
                line: 2,
                key: "path"
            })
        );
    }

    /// RF §4.4 requires `id` non-empty, and makes the empty `path` a defined
    /// value rather than an error — "No tree entry matches: the empty string."
    #[test]
    fn an_empty_path_is_legal_and_an_empty_id_is_not() {
        let empty_path = r#"{"id":"a","out":"passed","path":"","runner":"pytest","t":"base"}"#;
        assert!(BaseRecord::from_value(&value_of(empty_path), 2).is_ok());
        let empty_id = r#"{"id":"","out":"passed","path":"a.py","runner":"pytest","t":"base"}"#;
        assert_eq!(
            BaseRecord::from_value(&value_of(empty_id), 2),
            Err(Malformed::EmptyValue { line: 2, key: "id" })
        );
    }

    /// RF §4.3 rule 4: "No numbers, no `true`/`false`/`null`, no nested
    /// objects, no arrays."
    #[test]
    fn a_non_string_member_value_is_malformed() {
        let line = r#"{"id":"a","out":"passed","path":"a.py","runner":"pytest","t":7}"#;
        assert_eq!(
            BaseRecord::from_value(&value_of(line), 2),
            Err(Malformed::NonStringValue { line: 2, key: "t" })
        );
    }
}
