//! IR §12: the `@verifies` pragma, the file-granular join, and the `AC<n>`
//! naming sugar.
//!
//! `docs/spec/README.md` listed this as a known gap — "How a `@verifies` pragma
//! or a `test_AC1_*` name in a blob maps to a runner-native test id is assumed
//! by G1's coverage clause, by `Spine-Test`, and by both specs — and no
//! document defines it." IR §12 is what closes it.
//!
//! IR §12: "§12.1 is also what §2.1.1's seed rule reads, so the pragma's
//! grammar is **load-bearing three times over**: for the join, for G5's orphan
//! clause, and for which files the freeze closure starts from."

use crate::ids::sugar_field;
use crate::lang::Lang;
use crate::lex::{self, TokenKind};
use crate::runner::{Runner, TestKey};
use core::fmt;

/// `intent-doc.md` §3.1's two prefixes. "`BUG-` selects the Bug variant (§3.3)
/// and, at approval, PB §4.3's rule that the reproduction AC must be red or
/// `--approve` is refused outright. Nothing else … distinguishes the prefixes."
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IntentPrefix {
    Int,
    Bug,
}

impl IntentPrefix {
    pub const fn token(self) -> &'static str {
        match self {
            IntentPrefix::Int => "INT",
            IntentPrefix::Bug => "BUG",
        }
    }
}

/// An intent id, `intent-doc.md` §3.1:
///
/// ```text
/// id        := prefix "-" numeral
/// prefix    := "INT" | "BUG"
/// numeral   := a decimal integer 1 … 9007199254740991, written in ASCII digits,
///              left-padded with "0" to a minimum width of 3, and padded no further
/// ```
///
/// **The padding rule is not decoration.** ID §3.1: "The padding rule makes id
/// and integer a bijection, which three mechanisms need and none states: `spine
/// new` allocates `max+1` …, G9 requires 'exactly one `Spine-Event: land` per
/// intent id', and G7's 'the lower intent id holds the lease' is a numeric
/// comparison. Two spellings of one number would break all three."
///
/// IR §12.1 records that this is exactly the domain the pragma uses, and what
/// happened when it was not: "**Version 2 wrote `^(INT|BUG)-[0-9]+$` here and
/// that was a second id domain.** … two implementations disagreeing on it
/// disagree about whether `@verifies INT-42/AC-1` is an occurrence at all, hence
/// about whether the file is a seed (§2.1.1), hence about the whole closure."
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntentId {
    prefix: IntentPrefix,
    number: u64,
    text: String,
}

impl IntentId {
    /// ID §2.3's bound on the numeral, and JCS's on any integer that reaches a
    /// canonical form: `2^53 - 1`.
    pub const MAX_NUMBER: u64 = 9_007_199_254_740_991;

    /// Parse the **canonical spelling only**. `INT-42` (under-padded),
    /// `INT-0042` (over-padded), `INT-000` (zero), `INT-+42` and `int-042`
    /// (case) are not ids.
    pub fn parse(s: &str) -> Option<IntentId> {
        let (prefix, rest) = match s.strip_prefix("INT-") {
            Some(rest) => (IntentPrefix::Int, rest),
            None => (IntentPrefix::Bug, s.strip_prefix("BUG-")?),
        };
        let number = parse_numeral(rest)?;
        Some(IntentId {
            prefix,
            number,
            text: s.to_string(),
        })
    }

    pub fn prefix(&self) -> IntentPrefix {
        self.prefix
    }

    /// The integer the id is a bijection with — what `spine new`'s `max+1` and
    /// G7's "the lower intent id holds the lease" compare.
    pub fn number(&self) -> u64 {
        self.number
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// `numeral` of ID §3.1: "a decimal integer 1 … 9007199254740991, written in
/// ASCII digits, left-padded with `0` to a minimum width of 3, and padded no
/// further."
///
/// Read as the canonical *spelling* of `n`, which is the only reading that
/// makes id and integer a bijection: the digits of `n`, left-padded to width 3.
/// So width 3 admits a leading zero and any greater width does not.
fn parse_numeral(digits: &str) -> Option<u64> {
    if digits.len() < 3 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if digits.len() > 3 && digits.starts_with('0') {
        return None;
    }
    let n: u64 = digits.parse().ok()?;
    if !(1..=IntentId::MAX_NUMBER).contains(&n) {
        return None;
    }
    Some(n)
}

impl fmt::Display for IntentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// An acceptance-criterion number **as the pragma wrote it**.
///
/// IR §12.1: "The AC number is captured as written, and compared canonically.
/// `<digit>+` is deliberately wider than `intent-doc.md` §5.3's `1 … 6`: a
/// pragma naming `AC-9` must be *recognized* in order to be reported, since PB
/// §6.3's G5 fails loudly on 'a `verified_by` edge to a nonexistent AC (typo'd
/// pragma)' and a grammar that declined to recognize it would make the orphan
/// invisible instead."
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AcNumber {
    as_written: String,
}

impl AcNumber {
    pub fn new(as_written: impl Into<String>) -> AcNumber {
        AcNumber {
            as_written: as_written.into(),
        }
    }

    /// The bytes the pragma or the test name carried.
    pub fn as_written(&self) -> &str {
        &self.as_written
    }

    /// The canonical comparison IR §12.1 fixes: "compare the captured digit run
    /// against §5.3's spelling — a decimal `1 … 6` with no leading zeros — so
    /// `AC-9`, `AC-01` and `AC-007` are occurrences that name no acceptance
    /// criterion, seed nothing, and are G5 findings."
    ///
    /// `intent-doc.md` §5.3 caps a document at six acceptance criteria, so the
    /// spelling and the cap are the same set.
    pub fn criterion(&self) -> Option<u8> {
        match self.as_written.as_bytes() {
            [d @ b'1'..=b'6'] => Some(d - b'0'),
            _ => None,
        }
    }

    /// Whether this names a criterion the intent actually has. `ac_count` is
    /// the number of acceptance criteria the intent's parse produced.
    ///
    /// A `false` here is exactly G5's orphan finding (case C25): "**Must not**
    /// be silently dropped: §12.1 recognizes the occurrence so that it can be
    /// reported."
    pub fn names_a_criterion_of(&self, ac_count: u8) -> bool {
        self.criterion().is_some_and(|n| n <= ac_count)
    }
}

impl fmt::Display for AcNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AC-{}", self.as_written)
    }
}

/// One `@verifies` occurrence, located by the byte offset of its `@`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub offset: usize,
    pub intent: IntentId,
    pub ac: AcNumber,
}

impl Occurrence {
    /// IR §2.1.1's seed test, less the `C-T1` match the caller supplies:
    /// "`p`'s bytes in `A` carry at least one §12.1 pragma occurrence whose
    /// intent id is this intent's and whose acceptance-criterion number is in
    /// `AC`."
    ///
    /// IR §2.1.1: "A pragma this intent does not own seeds nothing. … `@verifies
    /// INT-041/AC-1` in this intent's tree, and `@verifies INT-042/AC-9` where
    /// the intent has three criteria, are both occurrences … and neither is a
    /// seed."
    pub fn seeds(&self, intent: &IntentId, ac_count: u8) -> bool {
        &self.intent == intent && self.ac.names_a_criterion_of(ac_count)
    }
}

/// IR §12.1's grammar, scanned over one comment's decoded bytes:
///
/// ```text
/// @verifies <SP>+ <intent-id> "/" "AC-" <digit>+
/// ```
///
/// "where `<SP>` is `U+0020` or `U+0009` … The scan is over the comment's
/// decoded bytes; a comment may carry several occurrences, separated by any
/// bytes. `@verifies` must be preceded by a byte outside `[A-Za-z0-9_@]` or be
/// at the comment's start, so `x@verifies` is not one."
///
/// `offset_base` is added to every reported offset so that a caller scanning a
/// file's comments gets file offsets.
pub fn scan_comment(text: &str, offset_base: usize) -> Vec<Occurrence> {
    const KEYWORD: &str = "@verifies";
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = text[from..].find(KEYWORD) {
        let at = from + rel;
        // Advance by one so that a failed tail does not hide a later
        // occurrence: "a comment may carry several occurrences, separated by
        // any bytes."
        from = at + 1;
        if at > 0 {
            let prev = bytes[at - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' || prev == b'@' {
                continue;
            }
        }
        let mut i = at + KEYWORD.len();
        // `<SP>+` — at least one, and only these two bytes. A newline cannot
        // occur inside a line comment and would not be `<SP>` if it did.
        let space_start = i;
        while matches!(bytes.get(i), Some(b' ') | Some(b'\t')) {
            i += 1;
        }
        if i == space_start {
            continue;
        }
        let Some((intent, next)) = take_intent_id(bytes, i) else {
            continue;
        };
        i = next;
        if bytes.get(i) != Some(&b'/') {
            continue;
        }
        i += 1;
        if !bytes[i..].starts_with(b"AC-") {
            continue;
        }
        i += 3;
        let digits_start = i;
        while bytes.get(i).is_some_and(|b| b.is_ascii_digit()) {
            i += 1;
        }
        if i == digits_start {
            continue;
        }
        out.push(Occurrence {
            offset: offset_base + at,
            intent,
            ac: AcNumber::new(&text[digits_start..i]),
        });
    }
    out
}

/// Read an intent id starting at `i`. The digit run is maximal, so a numeral
/// the padding rule refuses is not silently truncated to one it accepts.
fn take_intent_id(bytes: &[u8], i: usize) -> Option<(IntentId, usize)> {
    let prefix_len = 4; // "INT-" / "BUG-"
    let head = bytes.get(i..i + prefix_len)?;
    if head != b"INT-" && head != b"BUG-" {
        return None;
    }
    let mut j = i + prefix_len;
    while bytes.get(j).is_some_and(|b| b.is_ascii_digit()) {
        j += 1;
    }
    let text = core::str::from_utf8(&bytes[i..j]).ok()?;
    Some((IntentId::parse(text)?, j))
}

/// Every pragma occurrence in a source file, found by lexing it and scanning
/// its `comment` tokens.
///
/// IR §3.4 rule 4: comments "are discarded **after** the pragma scan of §12,
/// which reads them." IR §12.1: "The comment forms are the four languages'
/// own … **Docstrings are not comments and are not scanned** — a `@verifies` in
/// a Python docstring does not count, because a docstring is a string literal
/// and the resolver's lexer classifies it as one." (Cases J2, C30.)
pub fn scan_file(src: &str, lang: Lang) -> Vec<Occurrence> {
    let mut out = Vec::new();
    for token in lex::lex(src, lang) {
        if token.kind == TokenKind::Comment {
            out.extend(scan_comment(token.text(src), token.start));
        }
    }
    out
}

/// A test id a runner collected, together with the path its adapter's
/// `id -> path` produced. The empty path is the fail-closed answer §11 uses,
/// and a record carrying it joins to no pragma.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collected {
    pub test: TestKey,
    pub path: String,
}

/// Where a `verified_by` edge came from. IR §12.3: "The sugar is sugar: where a
/// file carries both a pragma and a matching name, the edges are the union and
/// no rule prefers one."
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeSource {
    Pragma,
    Sugar,
}

/// One `verified_by` edge: a collected test id, the criterion it names, and how
/// it named it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    pub test: TestKey,
    pub intent: IntentId,
    pub ac: AcNumber,
    pub source: EdgeSource,
}

/// IR §12.2 — **the join is file-granular**.
///
/// "A pragma occurrence in file `P` attributes to **every collected test id
/// whose `id → path` equals `P`**, for every runner in the invocation set."
///
/// That is PB §6.2's own granularity, in its own words: "A pragma counts only
/// when a runner collected an id from **its file**". IR §12.2 states the price
/// and why it is the right one: "The consequence is that a pragma attributes to
/// every test in its file, not to the test it sits above. That is coarser than
/// a reader might expect and it is the right trade: G1's coverage clause asks
/// whether an AC has *at least one* verifying collected id (PB §6.3), so
/// coarseness can only make coverage easier to satisfy, never harder — and G5's
/// orphan clause fails on a pragma naming a nonexistent AC, which is unaffected
/// by granularity."
///
/// `pragmas` maps a repository path to the occurrences that file's bytes carry.
pub fn join_pragmas(pragmas: &[(String, Vec<Occurrence>)], collected: &[Collected]) -> Vec<Edge> {
    let mut edges = Vec::new();
    for (path, occurrences) in pragmas {
        // An adapter that could not locate a file wrote the empty string; it
        // names no file and must join to nothing, or every id whose path lookup
        // failed would inherit every pragma in a file called "".
        if path.is_empty() {
            continue;
        }
        for item in collected.iter().filter(|c| &c.path == path) {
            for occurrence in occurrences {
                edges.push(Edge {
                    test: item.test.clone(),
                    intent: occurrence.intent.clone(),
                    ac: occurrence.ac.clone(),
                    source: EdgeSource::Pragma,
                });
            }
        }
    }
    normalize(edges)
}

/// IR §12.3 — the `AC<n>` naming sugar.
///
/// "The pattern is: the byte sequence `AC` followed by one or more digits,
/// preceded by a byte outside `[A-Za-z0-9]` or at the start of the field, and
/// followed by a byte outside `[0-9]` or at the end of the field. The capture is
/// the digit run, and the intent is the branch's single gated intent (PB §4.3,
/// 'one gated intent per branch')."
///
/// The trailing condition needs no code: the digit run is maximal, so the byte
/// after it is never a digit.
pub fn sugar_ac_numbers(field: &str) -> Vec<AcNumber> {
    let bytes = field.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 2 <= bytes.len() {
        if &bytes[i..i + 2] != b"AC" {
            i += 1;
            continue;
        }
        // "preceded by a byte outside `[A-Za-z0-9]` or at the start of the
        // field". Case J7: `test_MAC1_x` yields no edge.
        if i > 0 && bytes[i - 1].is_ascii_alphanumeric() {
            i += 1;
            continue;
        }
        let digits_start = i + 2;
        let mut j = digits_start;
        while bytes.get(j).is_some_and(|b| b.is_ascii_digit()) {
            j += 1;
        }
        if j == digits_start {
            i += 1;
            continue;
        }
        out.push(AcNumber::new(&field[digits_start..j]));
        // "Several `AC<n>` matches in one field yield several edges." Resume
        // after the digit run so one match cannot start inside another.
        i = j;
    }
    out
}

/// The sugar's edges for one collected id, against the branch's single gated
/// intent.
pub fn sugar_edges(item: &Collected, intent: &IntentId) -> Vec<Edge> {
    let field = sugar_field(item.test.runner, &item.test.id);
    sugar_ac_numbers(field)
        .into_iter()
        .map(|ac| Edge {
            test: item.test.clone(),
            intent: intent.clone(),
            ac,
            source: EdgeSource::Sugar,
        })
        .collect()
}

/// The union of §12.2's join and §12.3's sugar — "the edges are the union and
/// no rule prefers one".
pub fn edges(
    pragmas: &[(String, Vec<Occurrence>)],
    collected: &[Collected],
    intent: &IntentId,
) -> Vec<Edge> {
    let mut all = join_pragmas(pragmas, collected);
    for item in collected {
        all.extend(sugar_edges(item, intent));
    }
    normalize(all)
}

/// Sorted and deduplicated. The edge set is a **set**: IR §15 rule 5, "the
/// walk's order is immaterial and the output is a set", and an index that
/// depended on the order two collections happened to report ids in would not be
/// recomputable by a second implementation.
fn normalize(mut edges: Vec<Edge>) -> Vec<Edge> {
    edges.sort();
    edges.dedup();
    edges
}

/// The four runners' comment forms, for a caller that has a path rather than a
/// language in hand. IR §12.1: "`#` for Python; `//` and `/* */` for the other
/// three (nested for Dart and Swift, §6.1/§7.1); Python has no block comment."
pub fn scan_path(src: &str, path: &str) -> Vec<Occurrence> {
    match crate::lang::lang(path) {
        Some(lang) => scan_file(src, lang),
        // "A file whose `lang` is `none` is not scanned at all" (IR §12.4.3) —
        // the same residual §10 states for the closure, in a second place.
        None => Vec::new(),
    }
}

/// The runner set a pragma attributes across. IR §12.2: "for **every** runner
/// in the invocation set" — the join never asks which language the file is.
pub fn joins_across_every_runner() -> [Runner; 4] {
    Runner::ALL
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(s: &str) -> IntentId {
        IntentId::parse(s).unwrap_or_else(|| panic!("{s} should be an id"))
    }

    /// `intent-doc.md` §3.1's own examples, both polarities.
    #[test]
    fn id_3_1_the_padding_rule_makes_id_and_integer_a_bijection() {
        for (s, n) in [
            ("INT-001", 1),
            ("INT-042", 42),
            ("BUG-051", 51),
            ("INT-1042", 1042),
            ("INT-999", 999),
            ("INT-100", 100),
        ] {
            assert_eq!(intent(s).number(), n, "{s}");
            assert_eq!(intent(s).as_str(), s);
        }
        for s in [
            "INT-42",   // under-padded
            "INT-0042", // over-padded
            "INT-000",  // zero
            "INT-+42",
            "int-042", // case
            "TASK-042",
            "INT-",
            "INT-00",
            "INT-042x",
        ] {
            assert_eq!(IntentId::parse(s), None, "{s} must not be an id");
        }
    }

    /// ID §2.3's bound, on both sides.
    #[test]
    fn the_numeral_is_bounded_at_two_to_the_fifty_three_minus_one() {
        assert_eq!(intent("INT-9007199254740991").number(), IntentId::MAX_NUMBER);
        assert_eq!(IntentId::parse("INT-9007199254740992"), None);
    }

    /// Case J1: "`# @verifies INT-042/AC-1` in a Python comment | one pragma
    /// occurrence."
    #[test]
    fn j1_a_python_comment_pragma_is_one_occurrence() {
        let src = "# @verifies INT-042/AC-1\ndef test_x(): pass\n";
        let found = scan_file(src, Lang::Python);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].intent.as_str(), "INT-042");
        assert_eq!(found[0].ac.as_written(), "1");
        assert_eq!(found[0].ac.to_string(), "AC-1");
    }

    /// Case J2: "`\"\"\"@verifies INT-042/AC-1\"\"\"` in a Python docstring |
    /// **no** occurrence — a docstring is a string, not a comment."
    /// Case C30 extends it to a string literal in any language.
    #[test]
    fn j2_a_docstring_or_a_string_literal_carries_no_occurrence() {
        assert!(scan_file("\"\"\"@verifies INT-042/AC-1\"\"\"\n", Lang::Python).is_empty());
        assert!(scan_file("const s = '@verifies INT-042/AC-1';\n", Lang::Ts).is_empty());
        assert!(scan_file("var s = '@verifies INT-042/AC-1';\n", Lang::Dart).is_empty());
        assert!(scan_file("let s = \"@verifies INT-042/AC-1\"\n", Lang::Swift).is_empty());
    }

    /// Case J3: "`// x@verifies INT-042/AC-1` | no occurrence — `@verifies` must
    /// not be preceded by `[A-Za-z0-9_@]`."
    #[test]
    fn j3_the_keyword_must_not_be_preceded_by_a_word_byte_or_an_at() {
        for src in [
            "// x@verifies INT-042/AC-1\n",
            "// 9@verifies INT-042/AC-1\n",
            "// _@verifies INT-042/AC-1\n",
            "// @@verifies INT-042/AC-1\n",
        ] {
            assert!(scan_file(src, Lang::Ts).is_empty(), "{src}");
        }
        // A byte outside the class, and the comment's own start, both admit it.
        for src in [
            "// @verifies INT-042/AC-1\n",
            "//@verifies INT-042/AC-1\n",
            "// see:@verifies INT-042/AC-1\n",
            "/* @verifies INT-042/AC-1 */\n",
        ] {
            assert_eq!(scan_file(src, Lang::Ts).len(), 1, "{src}");
        }
    }

    /// Case C26: "`@verifies INT-41/AC-1` | not an occurrence at all —
    /// `intent-doc.md` §3.1's padding rule — so not a seed and not an orphan.
    /// **Must not** be admitted by a looser id grammar that accepts an unpadded
    /// numeral (version 2's)."
    #[test]
    fn c26_an_unpadded_numeral_is_not_an_occurrence_at_all() {
        assert!(scan_file("# @verifies INT-41/AC-1\n", Lang::Python).is_empty());
        assert!(scan_file("# @verifies INT-0042/AC-1\n", Lang::Python).is_empty());
    }

    /// Case C25: "`@verifies INT-042/AC-9` | **not** a seed, and G5's orphan
    /// finding. **Must not** be silently dropped: §12.1 recognizes the
    /// occurrence so that it can be reported."
    #[test]
    fn c25_an_ac_the_intent_does_not_have_is_recognised_so_that_it_can_be_reported() {
        let found = scan_file("# @verifies INT-042/AC-9\n", Lang::Python);
        assert_eq!(found.len(), 1, "the occurrence must be recognized");
        assert_eq!(found[0].ac.as_written(), "9");
        // …and it names no criterion, so it seeds nothing.
        assert_eq!(found[0].ac.criterion(), None);
        assert!(!found[0].seeds(&intent("INT-042"), 3));
    }

    /// IR §12.1: "`AC-9`, `AC-01` and `AC-007` are occurrences that name no
    /// acceptance criterion, seed nothing, and are G5 findings."
    #[test]
    fn the_ac_number_is_captured_as_written_and_compared_canonically() {
        for (written, criterion) in [
            ("1", Some(1u8)),
            ("6", Some(6)),
            ("7", None),
            ("9", None),
            ("01", None),
            ("007", None),
            ("12", None),
        ] {
            assert_eq!(AcNumber::new(written).criterion(), criterion, "{written}");
        }
        // Membership also depends on how many criteria the intent has.
        assert!(AcNumber::new("3").names_a_criterion_of(3));
        assert!(!AcNumber::new("4").names_a_criterion_of(3));
    }

    /// IR §2.1.1: "A pragma this intent does not own seeds nothing."
    #[test]
    fn a_pragma_naming_another_intent_seeds_nothing() {
        let found = scan_file("# @verifies INT-041/AC-1\n", Lang::Python);
        assert_eq!(found.len(), 1);
        assert!(!found[0].seeds(&intent("INT-042"), 3));
        assert!(found[0].seeds(&intent("INT-041"), 3));
    }

    /// IR §12.1: "a comment may carry several occurrences, separated by any
    /// bytes."
    #[test]
    fn one_comment_may_carry_several_occurrences() {
        let src = "// @verifies INT-042/AC-1 and also @verifies BUG-051/AC-2\n";
        let found = scan_file(src, Lang::Ts);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].intent.as_str(), "INT-042");
        assert_eq!(found[1].intent.as_str(), "BUG-051");
        assert!(found[0].offset < found[1].offset);
    }

    /// A malformed occurrence must not hide a later well-formed one — the scan
    /// resumes one byte on, not past the whole failed tail.
    #[test]
    fn a_malformed_occurrence_does_not_hide_a_later_one() {
        let src = "// @verifies INT-42/AC-1 @verifies INT-042/AC-2\n";
        let found = scan_file(src, Lang::Ts);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].ac.as_written(), "2");
    }

    /// IR §12.1: "`<SP>` is `U+0020` or `U+0009`", and there must be at least
    /// one.
    #[test]
    fn the_separator_is_one_or_more_spaces_or_tabs_and_nothing_else() {
        assert_eq!(scan_file("# @verifies\tINT-042/AC-1\n", Lang::Python).len(), 1);
        assert_eq!(scan_file("# @verifies   INT-042/AC-1\n", Lang::Python).len(), 1);
        assert!(scan_file("# @verifiesINT-042/AC-1\n", Lang::Python).is_empty());
    }

    /// The four comment forms of IR §12.1, one file each.
    #[test]
    fn every_language_comment_form_is_scanned() {
        for (src, lang) in [
            ("# @verifies INT-042/AC-1\n", Lang::Python),
            ("// @verifies INT-042/AC-1\n", Lang::Ts),
            ("/* @verifies INT-042/AC-1 */\n", Lang::Ts),
            ("/* /* @verifies INT-042/AC-1 */ */\n", Lang::Dart),
            ("// @verifies INT-042/AC-1\n", Lang::Swift),
        ] {
            assert_eq!(scan_file(src, lang).len(), 1, "{lang}: {src}");
        }
    }

    /// Case J4: "A pragma in a file from which the runner collected three ids |
    /// three `verified_by` edges (file granularity, §12.2)."
    #[test]
    fn j4_a_pragma_attributes_to_every_collected_id_from_its_file() {
        let occurrences = scan_file("# @verifies INT-042/AC-1\n", Lang::Python);
        let pragmas = [("tests/test_totals.py".to_string(), occurrences)];
        let collected: Vec<Collected> = ["test_a", "test_b", "test_c"]
            .iter()
            .map(|name| Collected {
                test: TestKey::new(Runner::Pytest, format!("tests/test_totals.py::{name}")),
                path: "tests/test_totals.py".to_string(),
            })
            .collect();
        let edges = join_pragmas(&pragmas, &collected);
        assert_eq!(edges.len(), 3);
        assert!(edges.iter().all(|e| e.ac.as_written() == "1"));
        assert!(edges.iter().all(|e| e.source == EdgeSource::Pragma));
    }

    /// IR §12.2: "for every runner in the invocation set" — the join is on the
    /// path, never on the language of the file the pragma sits in.
    #[test]
    fn the_join_crosses_runners_because_it_is_keyed_on_the_path() {
        let occurrences = scan_file("// @verifies INT-042/AC-2\n", Lang::Ts);
        let pragmas = [("src/a.test.ts".to_string(), occurrences)];
        let collected = [
            Collected {
                test: TestKey::new(Runner::Vitest, "src/a.test.ts > case"),
                path: "src/a.test.ts".to_string(),
            },
            Collected {
                test: TestKey::new(Runner::Pytest, "src/a.test.ts::case"),
                path: "src/a.test.ts".to_string(),
            },
        ];
        assert_eq!(join_pragmas(&pragmas, &collected).len(), 2);
        assert_eq!(joins_across_every_runner().len(), 4);
    }

    /// An adapter that could not locate a file writes the empty string; it must
    /// join to nothing, or every unlocatable id would inherit every pragma.
    #[test]
    fn a_record_whose_id_to_path_failed_joins_to_no_pragma() {
        let occurrences = scan_file("// @verifies INT-042/AC-1\n", Lang::Swift);
        let pragmas = [(String::new(), occurrences)];
        let collected = [Collected {
            test: TestKey::new(Runner::SwiftTest, "M.C/testX"),
            path: String::new(),
        }];
        assert!(join_pragmas(&pragmas, &collected).is_empty());
    }

    /// Cases J5, J6 and J7 — the naming sugar's whole published vector.
    #[test]
    fn j5_j6_j7_the_naming_sugar_pattern() {
        // J5: two edges, AC-1 and AC-2.
        let field = sugar_field(Runner::Pytest, "tests/t.py::test_AC1_and_AC2_totals");
        let numbers: Vec<&str> = sugar_ac_numbers(field)
            .iter()
            .map(|a| a.as_written())
            .map(|s| Box::leak(s.to_string().into_boxed_str()) as &str)
            .collect();
        assert_eq!(numbers, ["1", "2"]);

        // J6: one edge, AC-12 — not AC-1.
        let field = sugar_field(Runner::Pytest, "tests/t.py::test_AC12_totals");
        let found = sugar_ac_numbers(field);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].as_written(), "12");
        assert_eq!(found[0].criterion(), None, "AC-12 names no criterion");

        // J7: no edge — `AC` must not be preceded by `[A-Za-z0-9]`.
        let field = sugar_field(Runner::Pytest, "tests/t.py::test_MAC1_x");
        assert!(sugar_ac_numbers(field).is_empty());
    }

    /// IR §12.3's first stated consequence: "**`swift-test` yields one edge
    /// where pytest yields two.** … a camelCase method naming two criteria —
    /// `testAC1AndAC2Totals` — gives AC-1 and **not** AC-2, because the second
    /// `AC` is preceded by `d`. `test_AC1_and_AC2` under pytest gives both,
    /// because `_` is outside the class."
    #[test]
    fn ir_12_3_swift_yields_one_edge_where_pytest_yields_two() {
        let swift = sugar_ac_numbers(sugar_field(
            Runner::SwiftTest,
            "Billing.InvoiceTests/testAC1AndAC2Totals",
        ));
        assert_eq!(swift.len(), 1);
        assert_eq!(swift[0].as_written(), "1");

        let pytest = sugar_ac_numbers(sugar_field(Runner::Pytest, "tests/t.py::test_AC1_and_AC2"));
        assert_eq!(pytest.len(), 2);
    }

    /// IR §12.3's second: "**`dart-test`'s field is the whole qualified name,
    /// so a group name counts.** `group('AC3 rounding'){ test('half even') }`
    /// gives the field `AC3 rounding half even` and yields AC-3 for every test
    /// in the group."
    #[test]
    fn ir_12_3_a_dart_group_name_yields_an_edge_for_every_test_in_it() {
        for name in ["AC3 rounding half even", "AC3 rounding banker"] {
            let id = crate::ids::dart_compose_id("test/a_test.dart", name);
            let found = sugar_ac_numbers(sugar_field(Runner::DartTest, &id));
            assert_eq!(found.len(), 1, "{id}");
            assert_eq!(found[0].as_written(), "3");
        }
    }

    /// IR §12.3: "where a file carries both a pragma and a matching name, the
    /// edges are the union and no rule prefers one."
    #[test]
    fn the_sugar_and_the_pragma_union_and_neither_wins() {
        let occurrences = scan_file("# @verifies INT-042/AC-1\n", Lang::Python);
        let pragmas = [("tests/t.py".to_string(), occurrences)];
        let collected = [Collected {
            test: TestKey::new(Runner::Pytest, "tests/t.py::test_AC1_totals"),
            path: "tests/t.py".to_string(),
        }];
        let all = edges(&pragmas, &collected, &intent("INT-042"));
        // One id, one criterion, two independently derived edges — the union
        // keeps both provenances rather than collapsing them.
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|e| e.source == EdgeSource::Pragma));
        assert!(all.iter().any(|e| e.source == EdgeSource::Sugar));
        assert!(all.iter().all(|e| e.ac.as_written() == "1"));
    }

    /// IR §15 rule 5: the output is a set, so the edge list must not depend on
    /// the order the collector reported ids in.
    #[test]
    fn the_edge_set_is_order_independent_and_deduplicated() {
        let occurrences = scan_file("# @verifies INT-042/AC-1\n", Lang::Python);
        let pragmas = [("tests/t.py".to_string(), occurrences)];
        let a = Collected {
            test: TestKey::new(Runner::Pytest, "tests/t.py::test_a"),
            path: "tests/t.py".to_string(),
        };
        let b = Collected {
            test: TestKey::new(Runner::Pytest, "tests/t.py::test_b"),
            path: "tests/t.py".to_string(),
        };
        let forward = join_pragmas(&pragmas, &[a.clone(), b.clone()]);
        let reverse = join_pragmas(&pragmas, &[b, a.clone()]);
        assert_eq!(forward, reverse);
        // And a repeated report of one id yields one edge.
        assert_eq!(join_pragmas(&pragmas, &[a.clone(), a]).len(), 1);
    }

    /// IR §12.4.3 and case T13: "A file whose `lang` is `none` is not scanned
    /// at all."
    #[test]
    fn a_file_whose_lang_is_none_is_not_scanned() {
        assert!(scan_path("// @verifies INT-042/AC-1\n", "src/Legacy.java").is_empty());
        assert_eq!(
            scan_path("// @verifies INT-042/AC-1\n", "src/a.ts").len(),
            1
        );
    }
}
