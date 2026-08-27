//! One finding label, many reasons.
//!
//! RF §8.2 gives the trusted stage exactly two vocabulary items for a file it
//! could not use: "Absent, unreadable, or violating §4 in any particular: the
//! finding is `result-missing` or `result-malformed`." GR §9.25 repeats the
//! split — "`result-missing` is *no file at the path §8.1 fixes*;
//! `result-malformed` is *a file was found and §4's grammar or §8.3 step 3's
//! runner-token check rejected it*" — and adds that the two are exhaustive.
//!
//! So every way §4 can reject a file collapses to **one** token, and that is
//! why [`Malformed`]'s `Display` is that token and nothing else. The variant is
//! the diagnostic for the job log; the token is what reaches a gate report's
//! finding and, through a break-glass review, a reviewer's signature. Spelling
//! a variant name into the wire would produce a `wires` array no other
//! implementation reproduces (GR §6.3, G1 Coverage row).
//!
//! RF §8.2 also fixes what an implementation may *not* do with these: "**There
//! is no partial ingestion** — a malformed file yields no outcomes at all,
//! never 'read what parsed'." Every constructor here therefore aborts the whole
//! parse; nothing in this crate returns records beside an error.

use core::fmt;

/// Which of §4.5's two record sections a positional complaint is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Base,
    Result,
}

impl Section {
    fn name(self) -> &'static str {
        match self {
            Section::Base => "base",
            Section::Result => "result",
        }
    }
}

/// Every particular of RF §4 a file can violate.
///
/// Line numbers are 1-based and count the header as line 1, so they match what
/// a human sees in the job log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Malformed {
    // ---- §4.1 Encoding and framing ----
    /// "UTF-8, no BOM."
    NotUtf8,
    /// A file with no header line is not a file with an empty header.
    Empty,
    /// "Every line, including the last, is terminated."
    UnterminatedFinalLine,
    /// "A CR (`U+000D`) anywhere outside a JSON string escape makes the file
    /// malformed — the same rule that keeps envelopes hashing (§5.5)." Since
    /// §4.3 escapes `U+000D` as `\r`, a raw CR byte anywhere is outside a
    /// string escape by construction.
    CarriageReturn { line: usize },
    /// "No blank lines, no comment lines, no leading or trailing whitespace on
    /// any line, no bytes after the final LF."
    BlankLine { line: usize },

    // ---- §4.2 The header line ----
    /// "Six fields, `key=value`, separated by exactly one `U+0020`." A value
    /// containing a space, a doubled separator and a missing field all land
    /// here, because each changes the count.
    HeaderFieldCount { found: usize },
    /// A field with no `=`, or with an empty key.
    HeaderFieldShape { position: usize },
    /// "**A repeated key rejects the file** (§11)."
    HeaderRepeatedKey { key: String },
    /// "The field order is fixed. A header whose keys appear in any other order
    /// is malformed." An unknown key and a missing key land here too: at some
    /// position the key is not the one §4.2's table fixes for it.
    HeaderKeyOutOfOrder {
        position: usize,
        expected: &'static str,
        found: String,
    },
    /// "So does … an empty value".
    HeaderEmptyValue { key: &'static str },
    /// "…and any value outside its grammar."
    HeaderValueOutOfGrammar {
        key: &'static str,
        why: &'static str,
    },
    /// RF §3: "The filename stem equals the header's `tree=` value byte for
    /// byte. A file whose stem and header disagree is malformed (§8)."
    StemDisagreesWithTree { stem: String, tree: String },
    /// "A count that disagrees with the number of `base` records present is
    /// malformed."
    IdsDisagreesWithBaseRecords { header: u64, records: usize },

    // ---- §4.3 Canonical JSON ----
    /// Not JSON at all, or not one JSON text per line.
    LineNotJson { line: usize, why: String },
    /// "Every line after line 1 is one JSON **object**".
    LineNotAnObject { line: usize },
    /// "**Canonical form is required on read, not only on write.** A body line
    /// that parses as JSON but is not in canonical form is malformed."
    LineNotCanonical { line: usize },

    // ---- §4.4 Record kinds ----
    /// "Unknown `t` values … are all malformed".
    UnknownRecordKind { line: usize, t: String },
    /// "…missing keys…"
    MissingKey { line: usize, key: &'static str },
    /// "…unknown keys…"
    UnknownKey { line: usize, key: String },
    /// §4.3 rule 4: "v1 record kinds contain string values only."
    NonStringValue { line: usize, key: &'static str },
    /// `id`, `fn` and `runner` are each specified "Non-empty". `path` is not:
    /// the empty string is its defined value for "no tree entry matches".
    EmptyValue { line: usize, key: &'static str },
    /// "…unknown `out`/`status`/`runner` values are all malformed".
    UnknownOutcome { line: usize, out: String },
    /// "`absent` is a legal `out` on a `base` record and an unknown value —
    /// hence malformed — on a `result` one."
    AbsentOutcomeOnResult { line: usize },
    /// The `runner` token's lexical form, `[a-z][a-z0-9_-]{0,31}`.
    RunnerTokenOutOfGrammar { line: usize, runner: String },
    /// "A `result` record whose `fn` is not a prefix of its `id` is malformed."
    FnNotPrefixOfId { line: usize },
    /// `end.status` outside §7.3's closed set.
    UnknownStatus { line: usize, status: String },
    /// "**The pair `(runner, id)` is unique across the section.** A repeated
    /// pair is malformed."
    DuplicatePair {
        section: Section,
        runner: String,
        id: String,
    },

    // ---- §4.5 Ordering ----
    /// "A `base` record after a `result` record … malformed."
    BaseAfterResult { line: usize },
    /// "…a record after `end`…"
    RecordAfterEnd { line: usize },
    /// "…a missing `end`…"
    MissingEnd,
    /// "…a second `end`…"
    SecondEnd { line: usize },
    /// "…or a section out of sort order: malformed."
    OutOfSortOrder { section: Section, line: usize },
}

impl Malformed {
    /// RF §8.2's finding label. One token for every variant, deliberately.
    pub fn token(&self) -> &'static str {
        "result-malformed"
    }

    /// The job-log diagnostic. It never reaches a wire, a report or a
    /// signature — only stderr, where `ci.md` §5.1 sends all of the
    /// collector's output.
    pub fn detail(&self) -> String {
        match self {
            Malformed::NotUtf8 => "file is not UTF-8 (RF §4.1)".into(),
            Malformed::Empty => "file is empty; line 1 must be the header (RF §4.1)".into(),
            Malformed::UnterminatedFinalLine => {
                "final line is not terminated by LF (RF §4.1)".into()
            }
            Malformed::CarriageReturn { line } => {
                format!("line {line}: carriage return outside a JSON string escape (RF §4.1)")
            }
            Malformed::BlankLine { line } => format!("line {line}: blank line (RF §4.1)"),
            Malformed::HeaderFieldCount { found } => {
                format!("header has {found} space-separated fields, not 6 (RF §4.2)")
            }
            Malformed::HeaderFieldShape { position } => {
                format!("header field {position} is not `key=value` (RF §4.2)")
            }
            Malformed::HeaderRepeatedKey { key } => {
                format!("header repeats the key `{key}`, which rejects the file (RF §4.2, PB §11)")
            }
            Malformed::HeaderKeyOutOfOrder {
                position,
                expected,
                found,
            } => format!(
                "header field {position} is `{found}`, expected `{expected}`; the order is fixed (RF §4.2)"
            ),
            Malformed::HeaderEmptyValue { key } => format!("header `{key}=` is empty (RF §4.2)"),
            Malformed::HeaderValueOutOfGrammar { key, why } => {
                format!("header `{key}=` is outside its grammar: {why} (RF §4.2)")
            }
            Malformed::StemDisagreesWithTree { stem, tree } => {
                format!("filename stem `{stem}` is not the header's tree= `{tree}` (RF §3)")
            }
            Malformed::IdsDisagreesWithBaseRecords { header, records } => {
                format!("header says ids={header}, body carries {records} base records (RF §4.2)")
            }
            Malformed::LineNotJson { line, why } => format!("line {line}: not JSON: {why}"),
            Malformed::LineNotAnObject { line } => {
                format!("line {line}: body lines are JSON objects (RF §4.3)")
            }
            Malformed::LineNotCanonical { line } => {
                format!("line {line}: parses but is not in canonical form (RF §4.3)")
            }
            Malformed::UnknownRecordKind { line, t } => {
                format!("line {line}: unknown record kind `t`=`{t}` (RF §4.4)")
            }
            Malformed::MissingKey { line, key } => {
                format!("line {line}: missing key `{key}` (RF §4.4)")
            }
            Malformed::UnknownKey { line, key } => {
                format!("line {line}: unknown key `{key}` (RF §4.4)")
            }
            Malformed::NonStringValue { line, key } => {
                format!("line {line}: `{key}` is not a string (RF §4.3)")
            }
            Malformed::EmptyValue { line, key } => {
                format!("line {line}: `{key}` is empty (RF §4.4)")
            }
            Malformed::UnknownOutcome { line, out } => {
                format!("line {line}: `out`=`{out}` is outside RF §5's enum")
            }
            Malformed::AbsentOutcomeOnResult { line } => format!(
                "line {line}: `out`=`absent` is legal on a base record and on no other kind (RF §4.4)"
            ),
            Malformed::RunnerTokenOutOfGrammar { line, runner } => format!(
                "line {line}: `runner`=`{runner}` is outside `[a-z][a-z0-9_-]{{0,31}}` (RF §4.4)"
            ),
            Malformed::FnNotPrefixOfId { line } => {
                format!("line {line}: `fn` is not a prefix of `id` (RF §4.4)")
            }
            Malformed::UnknownStatus { line, status } => {
                format!("line {line}: `status`=`{status}` is outside RF §7.3's set")
            }
            Malformed::DuplicatePair {
                section,
                runner,
                id,
            } => format!(
                "{} section: the pair (`{runner}`, `{id}`) appears twice (RF §4.4)",
                section.name()
            ),
            Malformed::BaseAfterResult { line } => {
                format!("line {line}: a base record after a result record (RF §4.5)")
            }
            Malformed::RecordAfterEnd { line } => {
                format!("line {line}: a record after `end` (RF §4.5)")
            }
            Malformed::MissingEnd => "no `end` record (RF §4.5)".into(),
            Malformed::SecondEnd { line } => {
                format!("line {line}: a second `end` record (RF §4.5)")
            }
            Malformed::OutOfSortOrder { section, line } => format!(
                "line {line}: the {} section is out of sort order (RF §4.5)",
                section.name()
            ),
        }
    }
}

impl fmt::Display for Malformed {
    /// The finding label alone. See the module docs for why the detail is not
    /// here.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

impl core::error::Error for Malformed {}

#[cfg(test)]
mod tests {
    use super::*;

    /// GR §6.3's G1 Coverage row makes `result-malformed` a **bare `G1`** wire
    /// entry, so the label is what a break-glass reviewer signs over. Two
    /// implementations spelling it differently write different `wires` arrays.
    #[test]
    fn every_variant_displays_the_one_finding_label() {
        let variants = [
            Malformed::NotUtf8,
            Malformed::MissingEnd,
            Malformed::CarriageReturn { line: 3 },
            Malformed::HeaderRepeatedKey { key: "tree".into() },
            Malformed::FnNotPrefixOfId { line: 9 },
        ];
        for v in variants {
            assert_eq!(v.to_string(), "result-malformed");
        }
    }

    /// The diagnostic is a different string from the token, and it names the
    /// section of RF the rule comes from — that is the whole of its job.
    #[test]
    fn the_detail_is_not_the_token() {
        let m = Malformed::MissingEnd;
        assert_ne!(m.detail(), m.token());
        assert!(m.detail().contains("§4.5"));
    }
}
