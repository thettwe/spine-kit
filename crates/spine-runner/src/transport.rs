//! RF §6.6's transport: what a runner must tell the collector, and the parse.
//!
//! > "The transport by which the collector obtains a runner's stream is an
//! > implementation choice, constrained: it is read over a pipe the collector
//! > holds, it is not supplied by the candidate's environment, and it
//! > preserves, per item, **four** signals — the runner-native id, the
//! > per-phase outcome, the expected-failure polarity, **and deselection**."
//!
//! **The four are the type.** [`Item`] cannot be constructed without all of
//! them, which is what makes "A transport that loses any of the four is not
//! conforming, for any runner" structural rather than a rule to remember.
//! Deselection is the one adapters drop: "runners commonly report it outside
//! the per-item report (pytest through `pytest_deselected`), so a transport
//! carrying only the first three cannot distinguish a `deselected` id from an
//! absent one — and the two differ under §8.5 clause 3."
//!
//! The wire form is JSONL because the collector already carries a canonical
//! JSON parser and a runner plugin in any language can emit it. It is **not**
//! the runner's own stdout: IR §11.1 ratifies the argv and a parse of human
//! output is neither one of the four signals nor stable across a runner's
//! versions.

use spine_canon::{Value, parse};

/// One phase of one item's execution.
///
/// RF §6.7's mapping is stated over phases — "failure or exception in
/// `setup`/`teardown`" is `error` where the same failure in `call` is `failed`
/// — so the phase is carried and not collapsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Setup,
    Call,
    Teardown,
}

impl Phase {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "setup" => Some(Phase::Setup),
            "call" => Some(Phase::Call),
            "teardown" => Some(Phase::Teardown),
            _ => None,
        }
    }
}

/// What one phase did. Deliberately narrow: three words, and everything a
/// runner might add is [`PhaseOutcome::Other`], which the mapping reads as
/// "any other terminal report".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseOutcome {
    Passed,
    Failed,
    Skipped,
    Other,
}

impl PhaseOutcome {
    pub fn parse(s: &str) -> Self {
        match s {
            "passed" => PhaseOutcome::Passed,
            "failed" => PhaseOutcome::Failed,
            "skipped" => PhaseOutcome::Skipped,
            _ => PhaseOutcome::Other,
        }
    }
}

/// One item, carrying RF §6.6's four signals and nothing else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Signal 1: "the runner-native id" — pytest's nodeid, verbatim.
    pub id: String,
    /// Signal 2: "the per-phase outcome", in report order.
    pub phases: Vec<(Phase, PhaseOutcome)>,
    /// Signal 3: "the expected-failure polarity" — whether the item carried an
    /// expected-failure marker at all, which is what separates `xfail` from
    /// `failed` and `xpass` from `passed`.
    pub expected_failure: bool,
    /// Signal 4: "**and deselection**", which is mandatory and is the one a
    /// transport that reports only per-item results cannot carry.
    pub deselected: bool,
}

/// A whole stream, as the collector read it off the pipe.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub items: Vec<Item>,
    /// RF §6.6: "A collection error that yields no item id is recorded as one
    /// `error` record whose `id` and `fn` are the runner's own id for the
    /// failing collector — for pytest, the file's nodeid".
    pub collection_errors: Vec<String>,
    /// The count the runner reported for itself, for the completeness check.
    ///
    /// IR §11.2: pytest "reports its own collected-and-selected count — `4
    /// tests collected`, or `3/4 tests collected (1 deselected)`", and IR §11.1
    /// defines the floor as "collected **and selected**". So under deselection
    /// the number to compare against is the **numerator**.
    pub reported_count: Option<u64>,
    /// Whether the terminal session-end event arrived, which is what RF §7.3
    /// defines `complete` in terms of.
    ///
    /// IR §11.6 claims "Every adapter names its terminal session-end event" and
    /// names two of four — pytest's and vitest's are missing, filed as open
    /// question 8. The transport carries the fact explicitly so the adapter
    /// does not have to infer it from the stream ending, which is exactly what
    /// a killed process looks like.
    pub session_ended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamError {
    /// RF §7.3 and IR §11.1 both list it: a stream that will not parse leaves
    /// its ids absent rather than failing the run.
    Unparsable { line: usize, why: String },
}

impl core::fmt::Display for StreamError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StreamError::Unparsable { line, why } => {
                write!(f, "unparsable stream at line {line}: {why}")
            }
        }
    }
}

impl core::error::Error for StreamError {}

/// Parse the transport's JSONL.
///
/// One object per line, each with a `t` discriminator: `item`, `collect-error`,
/// `count`, `end`. Unknown `t` values are **skipped**, not refused — the
/// stream is a private channel between two halves of one release, and a
/// forward-compatible reader is what lets a plugin add a signal before every
/// collector understands it. What is refused is a line that is not JSON at all,
/// or an `item` missing one of the four.
pub fn parse_stream(bytes: &[u8]) -> Result<Report, StreamError> {
    let mut report = Report::default();

    for (index, line) in bytes.split(|&b| b == b'\n').enumerate() {
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let bad = |why: &str| StreamError::Unparsable {
            line: index + 1,
            why: why.to_string(),
        };
        let value = parse(line).map_err(|e| bad(&e.to_string()))?;
        let kind = value
            .get("t")
            .and_then(Value::as_str)
            .ok_or_else(|| bad("no `t` discriminator"))?;

        match kind {
            "item" => report.items.push(item_from(&value, &bad)?),
            "collect-error" => {
                let id = value
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| bad("a collect-error carries the failing collector's id"))?;
                report.collection_errors.push(id.to_string());
            }
            "count" => {
                report.reported_count = Some(
                    value
                        .get("selected")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| bad("a count carries `selected`"))?,
                );
            }
            "end" => report.session_ended = true,
            _ => {}
        }
    }

    Ok(report)
}

fn item_from(value: &Value, bad: &dyn Fn(&str) -> StreamError) -> Result<Item, StreamError> {
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("an item carries `id`"))?
        .to_string();

    // Every one of the four is required. An item missing `deselected` is the
    // three-signal transport RF §6.6 calls non-conforming, and defaulting it to
    // `false` would make a `deselected` id indistinguishable from an absent
    // one — which is the failure the paragraph is written about.
    let expected_failure = value
        .get("expected_failure")
        .and_then(Value::as_bool)
        .ok_or_else(|| bad("an item carries `expected_failure` (RF §6.6 signal 3)"))?;
    let deselected = value
        .get("deselected")
        .and_then(Value::as_bool)
        .ok_or_else(|| bad("an item carries `deselected` (RF §6.6 signal 4)"))?;

    let phases = match value.get("phases") {
        Some(Value::Arr(entries)) => entries
            .iter()
            .map(|entry| {
                let phase = entry
                    .get("phase")
                    .and_then(Value::as_str)
                    .and_then(Phase::parse)
                    .ok_or_else(|| bad("a phase is setup, call or teardown"))?;
                let outcome = entry
                    .get("outcome")
                    .and_then(Value::as_str)
                    .map(PhaseOutcome::parse)
                    .ok_or_else(|| bad("a phase carries `outcome`"))?;
                Ok((phase, outcome))
            })
            .collect::<Result<Vec<_>, StreamError>>()?,
        // A deselected item has no phases at all, which is legal and is the
        // whole reason signal 4 is separate from signal 2.
        None if deselected => Vec::new(),
        _ => return Err(bad("an item carries `phases` (RF §6.6 signal 2)")),
    };

    Ok(Item {
        id,
        phases,
        expected_failure,
        deselected,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stream_carries_all_four_signals() {
        let stream = concat!(
            r#"{"t":"item","id":"t.py::a","phases":[{"phase":"setup","outcome":"passed"},"#,
            r#"{"phase":"call","outcome":"passed"}],"expected_failure":false,"deselected":false}"#,
            "\n",
            r#"{"t":"count","selected":1}"#,
            "\n",
            r#"{"t":"end"}"#,
            "\n"
        );
        let report = parse_stream(stream.as_bytes()).expect("conforming");
        assert_eq!(report.items.len(), 1);
        assert_eq!(report.items[0].id, "t.py::a");
        assert_eq!(
            report.items[0].phases,
            [
                (Phase::Setup, PhaseOutcome::Passed),
                (Phase::Call, PhaseOutcome::Passed)
            ]
        );
        assert!(!report.items[0].expected_failure);
        assert!(!report.items[0].deselected);
        assert_eq!(report.reported_count, Some(1));
        assert!(report.session_ended);
    }

    /// RF §6.6: "A transport that loses any of the four is not conforming, for
    /// any runner." So an item missing one does not parse — defaulting it would
    /// make a `deselected` id indistinguishable from an absent one.
    #[test]
    fn an_item_missing_one_of_the_four_does_not_parse() {
        for line in [
            r#"{"t":"item","phases":[],"expected_failure":false,"deselected":true}"#,
            r#"{"t":"item","id":"a","expected_failure":false,"deselected":false}"#,
            r#"{"t":"item","id":"a","phases":[],"deselected":false}"#,
            r#"{"t":"item","id":"a","phases":[],"expected_failure":false}"#,
        ] {
            assert!(
                parse_stream(line.as_bytes()).is_err(),
                "{line} parsed and should not have"
            );
        }
    }

    /// A deselected item has no phases, which is legal and is why signal 4 is
    /// separate from signal 2.
    #[test]
    fn a_deselected_item_needs_no_phases() {
        let line = r#"{"t":"item","id":"t.py::a","expected_failure":false,"deselected":true}"#;
        let report = parse_stream(line.as_bytes()).expect("conforming");
        assert!(report.items[0].deselected);
        assert!(report.items[0].phases.is_empty());
    }

    /// The channel is private to one release, so a reader that refused an
    /// unknown record would stop a plugin from ever adding a signal. A line
    /// that is not JSON at all is a different matter: RF §7.3's "its stream was
    /// unparsable" is a real outcome with a defined consequence.
    #[test]
    fn an_unknown_record_is_skipped_and_a_non_json_line_is_not() {
        let stream = concat!(
            r#"{"t":"future","whatever":1}"#,
            "\n",
            r#"{"t":"end"}"#,
            "\n"
        );
        assert!(
            parse_stream(stream.as_bytes())
                .expect("skipped")
                .session_ended
        );

        let broken = "not json at all\n";
        assert!(matches!(
            parse_stream(broken.as_bytes()),
            Err(StreamError::Unparsable { line: 1, .. })
        ));
    }

    /// RF §6.6: "A collection error that yields no item id is recorded as one
    /// `error` record whose `id` and `fn` are the runner's own id for the
    /// failing collector — for pytest, the file's nodeid".
    #[test]
    fn a_collection_error_carries_the_failing_collectors_id() {
        let line = r#"{"t":"collect-error","id":"tests/broken.py"}"#;
        let report = parse_stream(line.as_bytes()).expect("conforming");
        assert_eq!(report.collection_errors, ["tests/broken.py"]);
    }

    /// RF §7.3 defines `complete` in terms of the terminal session-end event,
    /// and a killed process looks exactly like a stream that stopped. Carrying
    /// the fact explicitly is what tells them apart.
    #[test]
    fn a_stream_that_stops_short_did_not_end_its_session() {
        let stream = concat!(
            r#"{"t":"item","id":"a","phases":[],"expected_failure":false,"deselected":false}"#,
            "\n"
        );
        let report = parse_stream(stream.as_bytes()).expect("conforming");
        assert!(!report.session_ended, "no end record is not a complete run");
    }
}
