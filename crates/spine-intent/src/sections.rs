//! Sections: how one is located (ID §4.6), its key (ID §4.7), the closed
//! ordered table per variant (ID §4.8, TM §4.2–§4.3), the four refusals a
//! wrong set produces (ID §4.9), the body line classes (ID §4.10), and each
//! body grammar (ID §5.1–§5.5).
//!
//! **The parse is line-oriented and is not Markdown** (ID §4.1). "It knows
//! nothing about fenced code blocks, inline code spans, HTML blocks, link
//! reference definitions, list nesting, or lazy continuation." ID §11.1
//! defends the cost: "the alternative is 'parse the Markdown', which means
//! picking a CommonMark implementation and inheriting every corner of it …
//! Two implementations in four languages will not agree on those, and PB §1.1's
//! offline re-verification requires that they do."

use crate::ac::{self, Ac};
use crate::header::Variant;
use crate::status::{Refusal, Status};
use spine_resolve::Pattern;
use spine_resolve::pragma::IntentPrefix;

/// ID §2.3's per-polarity touchpoint bound.
pub const MAX_TOUCHPOINTS: usize = 256;

/// ID §5.2's bullet-item bound, shared by `non-goals` and TM §4.2's
/// `invariants`.
pub const MAX_BULLETS: usize = 256;

/// ID §5.2's minimum. PB §3.2 calls non-goals "the highest-leverage sixty
/// seconds in the document", and ID §5.2 refuses to give the minimum an
/// escape: "There is no override flag, no warn mode and no `--force` … a cap
/// with an escape hatch is advice."
pub const MIN_NON_GOALS: usize = 2;

/// TM §4.2's minimum, and it is **1, not 2**: "PB §3.2's minimum-two argument
/// is specific to non-goals and is about an agent *over-serving* a goal … an
/// invariant is a single positive claim about what the delta may not break, and
/// one is a real claim."
pub const MIN_INVARIANTS: usize = 1;

/// ID §4.8's "Body grammar" column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyGrammar {
    /// ID §5.1. At least one non-empty line; no bullet, no AC.
    Prose,
    /// ID §5.2. Bullets and continuations only.
    Bullet,
    /// ID §5.3. AC lines and continuations only.
    Ac,
    /// ID §5.4. Label lines only — "There is no prose in this section, which is
    /// what makes a mistyped label loud instead of silent."
    Touchpoints,
    /// ID §5.5. "Any non-empty line is permitted, of any class, and none of it
    /// is parsed."
    Free,
}

/// One row of a variant's section table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionSpec {
    pub key: &'static str,
    pub mandatory: bool,
    pub body: BodyGrammar,
}

/// ID §4.8 — variant `intent`, template version 2. Closed, ordered, complete.
const INTENT: &[SectionSpec] = &[
    SectionSpec { key: "goal", mandatory: true, body: BodyGrammar::Prose },
    SectionSpec { key: "non-goals", mandatory: true, body: BodyGrammar::Bullet },
    SectionSpec { key: "acceptance criteria", mandatory: true, body: BodyGrammar::Ac },
    SectionSpec { key: "touchpoints", mandatory: true, body: BodyGrammar::Touchpoints },
    SectionSpec { key: "open questions", mandatory: false, body: BodyGrammar::Free },
];

/// TM §4.2 — variant `intent-change`, template version 2. Seven sections, and
/// `goal` is **not** one of them: "PB §3.5 says Goal is *replaced*, and a
/// Change document carrying `## Goal` is `unknown-section`, refused rather than
/// tolerated."
const INTENT_CHANGE: &[SectionSpec] = &[
    SectionSpec { key: "current behavior", mandatory: true, body: BodyGrammar::Prose },
    SectionSpec { key: "target behavior", mandatory: true, body: BodyGrammar::Prose },
    SectionSpec { key: "non-goals", mandatory: true, body: BodyGrammar::Bullet },
    SectionSpec { key: "invariants", mandatory: true, body: BodyGrammar::Bullet },
    SectionSpec { key: "acceptance criteria", mandatory: true, body: BodyGrammar::Ac },
    SectionSpec { key: "touchpoints", mandatory: true, body: BodyGrammar::Touchpoints },
    SectionSpec { key: "open questions", mandatory: false, body: BodyGrammar::Free },
];

/// TM §4.3 — variant `intent-bug`, template version 2. "**The section table is
/// identical to `intent`'s** — same keys, same ordinals, same presence, same
/// body grammars." The variant is three things and none of them is a section:
/// the `BUG` prefix, AC-1 being the reproduction, and two heading
/// parentheticals ID §4.7 discards.
const INTENT_BUG: &[SectionSpec] = INTENT;

/// The variant's table.
pub fn table(variant: Variant) -> &'static [SectionSpec] {
    match variant {
        Variant::Intent => INTENT,
        Variant::IntentChange => INTENT_CHANGE,
        Variant::IntentBug => INTENT_BUG,
    }
}

/// ID §4.6's heading test: "a line whose first three bytes are exactly `## ` —
/// two U+0023 and one U+0020. Nothing else is a heading."
///
/// So `###` and deeper are body text, `##Goal` is body text, and an indented
/// `  ## Goal` is a continuation line — never a heading.
pub fn is_heading(line: &str) -> bool {
    line.as_bytes().starts_with(b"## ")
}

/// ID §4.7's three steps, in order:
///
/// 1. take the bytes after the leading `## `;
/// 2. strip leading and trailing `0x20` and `0x09`;
/// 3. take the bytes before the first `0x28` (`(`), strip trailing `0x20` and
///    `0x09` from what remains, and ASCII-lowercase it.
///
/// The parenthetical is "advisory in every respect: it may be present, absent,
/// or reworded, and the key is unchanged. Nothing reads it." That is what lets
/// `spine new` scaffold `## Non-goals (mandatory, minimum 2)` while an author
/// who deletes the hint still has a parsing document.
pub fn section_key(line: &str) -> Option<String> {
    let after = line.strip_prefix("## ")?;
    let trimmed = after.trim_matches([' ', '\t']);
    let before_paren = match trimmed.find('(') {
        Some(i) => &trimmed[..i],
        None => trimmed,
    };
    Some(before_paren.trim_end_matches([' ', '\t']).to_ascii_lowercase())
}

/// ID §3.3's legacy derivation, which runs for a bare `Template: v<n>` value
/// **and for nothing else**.
///
/// ```text
/// variant_legacy(d) :=
///   "intent-bug"     if the id's prefix is "BUG"
///   "intent-change"  else if d contains a line whose section key is "invariants"
///   "intent"         otherwise
/// ```
///
/// "The probe is a **pre-pass**: scan every line whose first three bytes are
/// `## `, compute its key by §4.7, and test for `invariants`. It runs before the
/// section table is chosen and reads nothing else."
///
/// Its totality rests on TM §3.2's disjointness invariant — `invariants` is a
/// key of exactly one variant's table and is mandatory there — which is why
/// [`INTENT`] and [`INTENT_BUG`] may never gain it.
pub fn variant_legacy(lines: &[&str], prefix: IntentPrefix) -> Variant {
    if prefix == IntentPrefix::Bug {
        return Variant::IntentBug;
    }
    if lines
        .iter()
        .any(|line| section_key(line).is_some_and(|k| k == "invariants"))
    {
        return Variant::IntentChange;
    }
    Variant::Intent
}

/// ID §4.10's body line classes, "classified by its first bytes, in this
/// order". A blank line is a separator and is classified as nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineClass<'a> {
    /// First two bytes are `- `; the text is the rest.
    Bullet(&'a str),
    /// First three bytes are `AC-`, and the line matches ID §5.3's grammar.
    Ac(Ac),
    /// First byte is `0x20` or `0x09`; the leading run is stripped and the
    /// remainder joins the preceding item with one `0x20`.
    Continuation,
    /// Anything else.
    Prose,
}

/// Classify one non-empty body line, raising the two refusals ID §4.10 closes
/// the traps with.
///
/// "Without these, an author who indents `AC-2` under `AC-1` silently ships a
/// document with one AC, the second AC has no node, G1's coverage clause is
/// vacuous over it, and nothing anywhere says so."
pub fn classify(line: &str, line_no: usize) -> Result<LineClass<'_>, Status> {
    if let Some(text) = line.strip_prefix("- ") {
        return Ok(LineClass::Bullet(text));
    }
    if line.as_bytes().starts_with(b"AC-") {
        return ac::parse_ac_line(line, line_no).map(LineClass::Ac);
    }
    if line.starts_with([' ', '\t']) {
        let stripped = line.trim_start_matches([' ', '\t']);
        if stripped.starts_with("- ") {
            return Err(Status::IndentedItem);
        }
        if stripped.starts_with("AC-") {
            return Err(Status::IndentedAc);
        }
        return Ok(LineClass::Continuation);
    }
    Ok(LineClass::Prose)
}

/// A located section: its row, the line its heading is on, and its body.
///
/// ID §4.6: "A section's **body** runs from the line after its heading to the
/// line before the next heading line, or to the end of the document. A section
/// is terminated by the next heading line and by nothing else — not by a blank
/// line, not by indentation, not by the end of a list."
#[derive(Debug, Clone)]
pub struct Located<'a> {
    pub spec: &'static SectionSpec,
    pub ordinal: usize,
    pub heading_line: usize,
    pub body: Vec<&'a str>,
    /// The 1-based line of `body[0]`.
    pub body_first_line: usize,
}

/// ID §8.2 step 7: "section headings: keys, unknown, duplicate, missing,
/// order".
///
/// The sub-order is that sentence's, and TM §4.5 pins one edge of it: a Change
/// document carrying `## Goal` reports `unknown-section` at `goal` rather than
/// `missing-section`, because "ID §8.2 step 7 checks unknown before missing".
pub fn locate<'a>(
    lines: &[&'a str],
    table: &'static [SectionSpec],
) -> Result<Vec<Located<'a>>, Refusal> {
    let headings: Vec<(usize, String)> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| section_key(line).map(|k| (i + 1, k)))
        .collect();

    for (line_no, key) in &headings {
        if !table.iter().any(|s| s.key == *key) {
            return Err(Refusal::at(Status::UnknownSection, *line_no));
        }
    }
    for (i, (line_no, key)) in headings.iter().enumerate() {
        if headings[..i].iter().any(|(_, k)| k == key) {
            return Err(Refusal::at(Status::DuplicateSection, *line_no));
        }
    }
    for spec in table.iter().filter(|s| s.mandatory) {
        if !headings.iter().any(|(_, k)| k == spec.key) {
            return Err(Refusal::whole(Status::MissingSection));
        }
    }
    // "Sections present but not in ascending ordinal order." ID §4.9 defends
    // the rule on two grounds, and the second is mechanical: PB §4.3's reopen
    // "inserts each new mandatory section as an empty stub" and needs a defined
    // insertion position, which fixed order gives it.
    let mut highest = 0usize;
    for (line_no, key) in &headings {
        let ordinal = table.iter().position(|s| s.key == *key).expect("checked above") + 1;
        if ordinal < highest {
            return Err(Refusal::at(Status::SectionOrder, *line_no));
        }
        highest = ordinal;
    }

    let mut located = Vec::with_capacity(headings.len());
    for (idx, (line_no, key)) in headings.iter().enumerate() {
        let ordinal = table.iter().position(|s| s.key == *key).expect("checked above");
        let end = headings
            .get(idx + 1)
            .map(|(next, _)| next - 1)
            .unwrap_or(lines.len());
        located.push(Located {
            spec: &table[ordinal],
            ordinal: ordinal + 1,
            heading_line: *line_no,
            body: lines[*line_no..end].to_vec(),
            body_first_line: line_no + 1,
        });
    }
    Ok(located)
}

impl Located<'_> {
    /// Iterate the body's non-empty lines with their 1-based line numbers.
    fn non_empty(&self) -> impl Iterator<Item = (usize, &str)> {
        self.body
            .iter()
            .enumerate()
            .filter(|(_, line)| !line.is_empty())
            .map(move |(i, line)| (self.body_first_line + i, *line))
    }

    /// ID §5.1's `prose`: "Every non-empty line must be **prose** or
    /// **continuation**; a bullet or an AC line is `stray-text`. At least one
    /// non-empty line is required (`empty-section`)."
    ///
    /// This is the grammar every scaffold refuses at (TM §6.3).
    pub fn parse_prose(&self) -> Result<(), Refusal> {
        let mut any = false;
        let mut have_item = false;
        for (i, line) in self.body.iter().enumerate() {
            let line_no = self.body_first_line + i;
            if line.is_empty() {
                // DERIVED: a blank line clears the preceding item, so a
                // continuation after one is `stray-continuation`. ID §4.10 calls
                // a blank line "a separator" and gives `stray-continuation` to a
                // continuation "with no preceding item"; a separator that did
                // not separate would leave the status unreachable in the one
                // shape an author actually produces. This is the fail-closed
                // reading of the two available.
                have_item = false;
                continue;
            }
            any = true;
            match classify(line, line_no).map_err(|s| Refusal::at(s, line_no))? {
                LineClass::Bullet(_) | LineClass::Ac(_) => {
                    return Err(Refusal::at(Status::StrayText, line_no));
                }
                LineClass::Continuation if !have_item => {
                    return Err(Refusal::at(Status::StrayContinuation, line_no));
                }
                LineClass::Continuation => {}
                LineClass::Prose => have_item = true,
            }
        }
        if any {
            Ok(())
        } else {
            Err(Refusal::at(Status::EmptySection, self.heading_line))
        }
    }

    /// ID §5.2's `bullet`, shared by `non-goals` and TM §4.2's `invariants`.
    /// Returns the item count, which is the section's whole mechanical
    /// content: "**The text is never read.** … Non-goals are not nodes."
    pub fn parse_bullets(&self) -> Result<usize, Refusal> {
        let mut count = 0usize;
        let mut have_item = false;
        for (i, line) in self.body.iter().enumerate() {
            let line_no = self.body_first_line + i;
            if line.is_empty() {
                have_item = false;
                continue;
            }
            match classify(line, line_no).map_err(|s| Refusal::at(s, line_no))? {
                LineClass::Bullet(text) => {
                    if text.is_empty() {
                        return Err(Refusal::at(Status::EmptyItem, line_no));
                    }
                    count += 1;
                    have_item = true;
                }
                LineClass::Continuation if !have_item => {
                    return Err(Refusal::at(Status::StrayContinuation, line_no));
                }
                LineClass::Continuation => {}
                LineClass::Ac(_) | LineClass::Prose => {
                    return Err(Refusal::at(Status::StrayText, line_no));
                }
            }
        }
        Ok(count)
    }

    /// ID §5.3's `ac`. Returns the criteria in document order; the bounds are
    /// step 9's and live in [`crate::ac::check_bounds`].
    pub fn parse_acs(&self) -> Result<Vec<Ac>, Refusal> {
        let mut acs = Vec::new();
        let mut have_item = false;
        for (i, line) in self.body.iter().enumerate() {
            let line_no = self.body_first_line + i;
            if line.is_empty() {
                have_item = false;
                continue;
            }
            match classify(line, line_no).map_err(|s| Refusal::at(s, line_no))? {
                LineClass::Ac(a) => {
                    acs.push(a);
                    have_item = true;
                }
                LineClass::Continuation if !have_item => {
                    return Err(Refusal::at(Status::StrayContinuation, line_no));
                }
                LineClass::Continuation => {}
                LineClass::Bullet(_) | LineClass::Prose => {
                    return Err(Refusal::at(Status::StrayText, line_no));
                }
            }
        }
        Ok(acs)
    }

    /// ID §5.5's `free`. The section is *empty* "iff its body contains no
    /// non-empty line. Not 'no bullets': a body of prose, or of a single line
    /// reading `None`, or `- (none)`, is **not** empty." This is the strictest
    /// available reading, and ID §5.5 takes it because the condition it feeds is
    /// PB §3.2's "this converts 'the agent assumed' into 'the agent asked'" — "a
    /// section with words in it has words in it".
    pub fn is_empty_body(&self) -> bool {
        self.non_empty().next().is_none()
    }

    /// ID §5.4's `touchpoints`.
    pub fn parse_touchpoints(&self) -> Result<TouchpointBody, Refusal> {
        let mut expected: Option<(usize, Vec<Pattern>)> = None;
        let mut forbidden: Option<(usize, Vec<Pattern>)> = None;

        for (line_no, line) in self.non_empty() {
            let (polarity, patterns) =
                parse_label_line(line, line_no).map_err(|s| Refusal::at(s, line_no))?;
            let slot = match polarity {
                Polarity::Expected => &mut expected,
                Polarity::Forbidden => &mut forbidden,
            };
            if slot.is_some() {
                return Err(Refusal::at(Status::DuplicateTouchpointLine, line_no));
            }
            *slot = Some((line_no, patterns));
        }

        // "A missing label line is `missing-touchpoint-line`" — and the label is
        // still mandatory when its list is empty, "because an absent line and an
        // empty line are different claims and only one of them was made
        // deliberately."
        let (expected_line, expected) = expected
            .ok_or_else(|| Refusal::at(Status::MissingTouchpointLine, self.heading_line))?;
        let (forbidden_line, forbidden) = forbidden
            .ok_or_else(|| Refusal::at(Status::MissingTouchpointLine, self.heading_line))?;

        Ok(TouchpointBody {
            expected,
            forbidden,
            expected_line,
            forbidden_line,
        })
    }
}

/// PB §6.2's `declares` edge attr.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Polarity {
    Expected,
    Forbidden,
}

impl Polarity {
    /// The attr value the `declares` edge carries: `{"polarity": "expected"}`
    /// or `{"polarity": "forbidden"}`.
    pub fn attr(self) -> &'static str {
        match self {
            Polarity::Expected => "expected",
            Polarity::Forbidden => "forbidden",
        }
    }
}

/// A touchpoints body as written — before ID §5.6's dedup and before step 9's
/// bounds.
#[derive(Debug, Clone)]
pub struct TouchpointBody {
    pub expected: Vec<Pattern>,
    pub forbidden: Vec<Pattern>,
    /// ID §6.6: provenance is "the touchpoint **label line's**, not the
    /// individual pattern's, since several patterns share one line".
    pub expected_line: usize,
    pub forbidden_line: usize,
}

/// ID §5.4's `label-line`:
///
/// ```text
/// label-line    := label ":" [ " " pattern-list ]
/// label         := "Expected to change" | "Must NOT change"   -- ASCII case-insensitive
/// pattern-list  := pattern-field ("," pattern-field)*
/// pattern-field := [space-or-tab*] pattern [space-or-tab*]
/// ```
///
/// "The label is matched by ASCII-lowercasing the bytes before the first `:`
/// and stripping leading and trailing spaces and tabs … `Must NOT chnage` is
/// `unknown-touchpoint-line`."
///
/// DERIVED: ID §5.4 makes every non-label line in this body
/// `unknown-touchpoint-line` — "Prose, bullets, AC lines and continuations are
/// all `unknown-touchpoint-line`" — so ID §4.10's classification, and with it
/// `malformed-ac`, is not applied here. The section-specific rule is total and
/// is the one taken; the general one would give a line beginning `AC-` a status
/// naming a section it is not in.
fn parse_label_line(line: &str, _line_no: usize) -> Result<(Polarity, Vec<Pattern>), Status> {
    let (before, after) = line.split_once(':').ok_or(Status::UnknownTouchpointLine)?;
    let label = before.trim_matches([' ', '\t']).to_ascii_lowercase();
    let polarity = match label.as_str() {
        "expected to change" => Polarity::Expected,
        "must not change" => Polarity::Forbidden,
        _ => return Err(Status::UnknownTouchpointLine),
    };

    // "The empty forbidden set is written `Must NOT change:` with nothing after
    // the colon — the trailing space is already forbidden by §2.1 rule 9, so
    // there is exactly one spelling."
    if after.is_empty() {
        return Ok((polarity, Vec::new()));
    }
    // The grammar's optional tail begins with exactly one `" "`. A list that
    // starts flush against the colon does not match it.
    let list = after.strip_prefix(' ').ok_or(Status::UnknownTouchpointLine)?;

    // "Split the value on `,` (`0x2C`), then strip leading and trailing spaces
    // and tabs from each field. … This split is unambiguous because §6.1
    // forbids `,` and space inside a pattern."
    let mut patterns = Vec::new();
    for field in list.split(',') {
        let text = field.trim_matches([' ', '\t']);
        if text.is_empty() {
            return Err(Status::EmptyTouchpoint);
        }
        patterns.push(Pattern::parse(text).map_err(Status::BadPattern)?);
    }
    Ok((polarity, patterns))
}

/// ID §5.6's dedup: "patterns as written, in document order, duplicates removed
/// keeping the first occurrence". Byte equality, because "the `declares` edge
/// set is a set" (ID §5.4).
pub fn dedup(patterns: &[Pattern]) -> Vec<Pattern> {
    let mut out: Vec<Pattern> = Vec::with_capacity(patterns.len());
    for p in patterns {
        if !out.iter().any(|q| q.as_str() == p.as_str()) {
            out.push(p.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- ID §4.6, §4.7 ----

    #[test]
    fn only_two_hashes_and_a_space_is_a_heading() {
        assert!(is_heading("## Goal"));
        assert!(!is_heading("### Detail"));
        assert!(!is_heading("##Goal"));
        assert!(!is_heading("  ## Goal"));
        assert!(!is_heading("# INT-042: t"));
    }

    /// ID §4.7's own three worked keys.
    #[test]
    fn a_key_is_the_text_before_the_first_paren_casefolded() {
        assert_eq!(
            section_key("## Acceptance criteria (maximum 6 — more means split the task)").unwrap(),
            "acceptance criteria"
        );
        assert_eq!(section_key("## Non-Goals").unwrap(), "non-goals");
        assert_eq!(section_key("## Goal (2–3 sentences)").unwrap(), "goal");
    }

    /// ID §4.7: "An empty key — a heading of `## ` alone, or `## (hint)` — is
    /// `unknown-section`", which it becomes by failing to match the table.
    #[test]
    fn an_empty_key_matches_no_row() {
        assert_eq!(section_key("## (hint)").unwrap(), "");
        assert!(!INTENT.iter().any(|s| s.key.is_empty()));
    }

    /// ID §4.7: "a key is casefolded but never normalised, so a heading spelled
    /// with a fullwidth letter is a different key."
    #[test]
    fn a_key_is_casefolded_but_never_normalised() {
        assert_eq!(section_key("## GOAL").unwrap(), "goal");
        assert_eq!(section_key("## \u{FF27}oal").unwrap(), "\u{FF27}oal");
    }

    // ---- ID §4.8, TM §4.2, TM §4.3 ----

    /// TM §16 item 13: "`intent-bug@2`'s table is byte-for-byte `intent@2`'s in
    /// keys, ordinals, presence and body grammars."
    #[test]
    fn the_bug_table_is_identical_to_the_feature_table() {
        assert_eq!(table(Variant::Intent), table(Variant::IntentBug));
    }

    /// TM §3.2's disjointness invariant, which ID §3.3's legacy derivation is
    /// total only because of: "`invariants` appears in exactly one variant's
    /// table — `intent-change` — and is **mandatory** there."
    #[test]
    fn invariants_is_a_key_of_exactly_one_variant_and_is_mandatory_there() {
        let holders: Vec<Variant> = Variant::ALL
            .into_iter()
            .filter(|v| table(*v).iter().any(|s| s.key == "invariants"))
            .collect();
        assert_eq!(holders, [Variant::IntentChange]);
        let row = table(Variant::IntentChange)
            .iter()
            .find(|s| s.key == "invariants")
            .unwrap();
        assert!(row.mandatory);
        assert_eq!(row.body, BodyGrammar::Bullet);
    }

    /// TM §4.2: "`goal` is **not** a key of this table."
    #[test]
    fn goal_is_absent_from_the_change_table() {
        assert!(!table(Variant::IntentChange).iter().any(|s| s.key == "goal"));
    }

    /// TM §5.4: `open questions` is "optional and last in every table".
    #[test]
    fn open_questions_is_optional_and_last_everywhere() {
        for v in Variant::ALL {
            let last = table(v).last().unwrap();
            assert_eq!(last.key, "open questions", "{v}");
            assert!(!last.mandatory, "{v}");
            assert_eq!(last.body, BodyGrammar::Free, "{v}");
        }
    }

    /// ID §2.3: "at most 7 [sections] at version 2, across the three variants".
    #[test]
    fn no_variant_has_more_than_seven_sections() {
        for v in Variant::ALL {
            assert!(table(v).len() <= 7, "{v}");
        }
        assert_eq!(table(Variant::IntentChange).len(), 7);
    }

    // ---- ID §3.3, the legacy derivation ----

    #[test]
    fn the_legacy_derivation_tests_the_prefix_before_the_probe() {
        // "a legacy `BUG-` document is never a Change document whatever headings
        // it carries" (TM §3.2).
        let with_invariants = ["## Invariants (mandatory, minimum 1)"];
        assert_eq!(
            variant_legacy(&with_invariants, IntentPrefix::Bug),
            Variant::IntentBug
        );
        assert_eq!(
            variant_legacy(&with_invariants, IntentPrefix::Int),
            Variant::IntentChange
        );
        assert_eq!(variant_legacy(&["## Goal"], IntentPrefix::Int), Variant::Intent);
    }

    #[test]
    fn the_probe_reads_only_level_two_headings() {
        // `### Invariants` is body text (ID §4.6), so it does not derive.
        assert_eq!(
            variant_legacy(&["### Invariants"], IntentPrefix::Int),
            Variant::Intent
        );
        // The parenthetical is discarded, so the hint does not change the key.
        assert_eq!(
            variant_legacy(&["## invariants (whatever)"], IntentPrefix::Int),
            Variant::IntentChange
        );
    }

    // ---- ID §4.10 ----

    #[test]
    fn the_four_classes_are_decided_by_leading_bytes_in_order() {
        assert_eq!(classify("- a non-goal", 1), Ok(LineClass::Bullet("a non-goal")));
        assert_eq!(classify("AC-1: text", 1).unwrap(), LineClass::Ac(Ac { number: 1, line: 1 }));
        assert_eq!(classify("  wrapped", 1), Ok(LineClass::Continuation));
        assert_eq!(classify("\twrapped", 1), Ok(LineClass::Continuation));
        assert_eq!(classify("plain prose", 1), Ok(LineClass::Prose));
        // `-` alone is not a bullet: the class needs the two bytes `- `.
        assert_eq!(classify("-nope", 1), Ok(LineClass::Prose));
    }

    /// The two traps ID §4.10 closes by refusal.
    #[test]
    fn an_indented_bullet_or_ac_is_refused_not_absorbed() {
        assert_eq!(classify("  - a", 1), Err(Status::IndentedItem));
        assert_eq!(classify("\tAC-2: b", 1), Err(Status::IndentedAc));
    }

    // ---- ID §5.4 ----

    fn body(lines: &[&'static str]) -> Located<'static> {
        Located {
            spec: &INTENT[3],
            ordinal: 4,
            heading_line: 10,
            body: lines.to_vec(),
            body_first_line: 11,
        }
    }

    #[test]
    fn id_9_1s_touchpoints_body_yields_both_polarities_and_their_label_lines() {
        let t = body(&[
            "Expected to change: src/billing/, api/invoices.ts",
            "Must NOT change: auth/, shared/schema/",
        ])
        .parse_touchpoints()
        .unwrap();
        let names = |ps: &[Pattern]| ps.iter().map(|p| p.as_str().to_string()).collect::<Vec<_>>();
        assert_eq!(names(&t.expected), ["src/billing/", "api/invoices.ts"]);
        assert_eq!(names(&t.forbidden), ["auth/", "shared/schema/"]);
        assert_eq!((t.expected_line, t.forbidden_line), (11, 12));
    }

    /// ID §5.4: "`Must not change`, `MUST NOT CHANGE` and `must not change` all
    /// parse; `Must NOT chnage` is `unknown-touchpoint-line`."
    #[test]
    fn the_label_is_ascii_case_insensitive_and_the_typo_is_loud() {
        for spelling in ["Must NOT change", "Must not change", "MUST NOT CHANGE", "must not change"] {
            let line = format!("{spelling}:");
            let t = body(&["Expected to change: a"]).parse_touchpoints();
            assert!(t.is_err());
            let (polarity, patterns) = parse_label_line(&line, 1).unwrap();
            assert_eq!(polarity, Polarity::Forbidden);
            assert!(patterns.is_empty());
        }
        assert_eq!(
            parse_label_line("Must NOT chnage: auth/", 1),
            Err(Status::UnknownTouchpointLine)
        );
    }

    #[test]
    fn the_empty_forbidden_set_has_exactly_one_spelling() {
        let t = body(&["Expected to change: src/http/", "Must NOT change:"])
            .parse_touchpoints()
            .unwrap();
        assert!(t.forbidden.is_empty());
    }

    #[test]
    fn a_missing_or_repeated_label_line_is_refused() {
        assert_eq!(
            body(&["Expected to change: a"]).parse_touchpoints().unwrap_err().status,
            Status::MissingTouchpointLine
        );
        assert_eq!(
            body(&["Must NOT change:"]).parse_touchpoints().unwrap_err().status,
            Status::MissingTouchpointLine
        );
        assert_eq!(
            body(&["Expected to change: a", "Expected to change: b", "Must NOT change:"])
                .parse_touchpoints()
                .unwrap_err()
                .status,
            Status::DuplicateTouchpointLine
        );
    }

    /// ID §5.4: "a trailing comma, a doubled comma, a list of one comma".
    #[test]
    fn an_empty_field_after_stripping_is_empty_touchpoint() {
        for list in ["a,", ",a", "a,,b", ","] {
            let line = format!("Expected to change: {list}");
            assert_eq!(parse_label_line(&line, 1), Err(Status::EmptyTouchpoint), "{list}");
        }
    }

    #[test]
    fn a_pattern_refusal_carries_id_6_1s_sub_status() {
        let e = parse_label_line("Expected to change: src/**.ts", 1).unwrap_err();
        assert_eq!(e.token(), "bad-globstar");
        assert!(e.is_bad_pattern());
    }

    #[test]
    fn every_other_line_class_in_this_body_is_unknown_touchpoint_line() {
        for line in ["- a bullet", "AC-1: a criterion", "  a continuation", "some prose", "no colon"] {
            assert_eq!(
                parse_label_line(line, 1),
                Err(Status::UnknownTouchpointLine),
                "{line}"
            );
        }
    }

    #[test]
    fn a_list_flush_against_the_colon_does_not_match_the_grammar() {
        assert_eq!(
            parse_label_line("Expected to change:src/", 1),
            Err(Status::UnknownTouchpointLine)
        );
    }

    // ---- ID §5.1, §5.2, §5.3 bodies ----

    fn goal(lines: &[&'static str]) -> Located<'static> {
        Located {
            spec: &INTENT[0],
            ordinal: 1,
            heading_line: 4,
            body: lines.to_vec(),
            body_first_line: 5,
        }
    }

    #[test]
    fn a_prose_body_needs_one_non_empty_line_and_admits_no_bullet_or_ac() {
        assert!(goal(&["Invoices show a tax-inclusive total."]).parse_prose().is_ok());
        assert_eq!(
            goal(&[]).parse_prose().unwrap_err(),
            Refusal::at(Status::EmptySection, 4)
        );
        assert_eq!(
            goal(&[""]).parse_prose().unwrap_err(),
            Refusal::at(Status::EmptySection, 4)
        );
        assert_eq!(
            goal(&["- a bullet"]).parse_prose().unwrap_err().status,
            Status::StrayText
        );
        assert_eq!(
            goal(&["AC-1: x"]).parse_prose().unwrap_err().status,
            Status::StrayText
        );
    }

    #[test]
    fn a_continuation_with_no_preceding_item_is_stray_continuation() {
        assert_eq!(
            goal(&["  wrapped"]).parse_prose().unwrap_err().status,
            Status::StrayContinuation
        );
        // DERIVED: the blank line separates, so the continuation after it has no
        // preceding item.
        assert_eq!(
            goal(&["prose", "", "  wrapped"]).parse_prose().unwrap_err().status,
            Status::StrayContinuation
        );
        assert!(goal(&["prose", "  wrapped"]).parse_prose().is_ok());
    }

    fn bullets(lines: &[&'static str]) -> Located<'static> {
        Located {
            spec: &INTENT[1],
            ordinal: 2,
            heading_line: 9,
            body: lines.to_vec(),
            body_first_line: 10,
        }
    }

    #[test]
    fn a_bullet_body_counts_items_and_admits_no_prose_or_ac() {
        assert_eq!(bullets(&["- a", "- b", "- c"]).parse_bullets().unwrap(), 3);
        assert_eq!(bullets(&["- a", "  wrapped", "- b"]).parse_bullets().unwrap(), 2);
        assert_eq!(
            bullets(&["prose"]).parse_bullets().unwrap_err().status,
            Status::StrayText
        );
        assert_eq!(
            bullets(&["AC-1: x"]).parse_bullets().unwrap_err().status,
            Status::StrayText
        );
        assert_eq!(bullets(&[]).parse_bullets().unwrap(), 0);
    }

    fn criteria(lines: &[&'static str]) -> Located<'static> {
        Located {
            spec: &INTENT[2],
            ordinal: 3,
            heading_line: 14,
            body: lines.to_vec(),
            body_first_line: 15,
        }
    }

    #[test]
    fn an_ac_body_collects_numbers_in_document_order() {
        let acs = criteria(&["AC-1: a", "  wrapped", "AC-2: b"]).parse_acs().unwrap();
        assert_eq!(acs.iter().map(|a| a.number).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(acs[0].line, 15);
        assert_eq!(acs[1].line, 17);
        assert_eq!(
            criteria(&["- a"]).parse_acs().unwrap_err().status,
            Status::StrayText
        );
        assert_eq!(
            criteria(&["AC-3 no colon"]).parse_acs().unwrap_err().status,
            Status::MalformedAc
        );
    }

    // ---- ID §5.5 ----

    #[test]
    fn open_questions_is_empty_only_when_no_line_has_any_bytes() {
        let mut oq = goal(&[]);
        oq.spec = &INTENT[4];
        assert!(oq.is_empty_body());
        let mut with_none = goal(&["None"]);
        with_none.spec = &INTENT[4];
        assert!(!with_none.is_empty_body());
        let mut with_bullet = goal(&["- (none)"]);
        with_bullet.spec = &INTENT[4];
        assert!(!with_bullet.is_empty_body());
    }

    // ---- ID §5.6's dedup ----

    #[test]
    fn duplicates_are_removed_keeping_the_first_occurrence() {
        let ps: Vec<Pattern> = ["src/", "auth/", "src/"]
            .iter()
            .map(|s| Pattern::parse(s).unwrap())
            .collect();
        let out = dedup(&ps);
        assert_eq!(
            out.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
            ["src/", "auth/"]
        );
    }

    // ---- ID §4.9 / §8.2 step 7 ----

    #[test]
    fn a_section_ends_at_the_next_heading_and_at_nothing_else() {
        let lines = [
            "# INT-042: t",
            "Owner: x",
            "",
            "## Goal",
            "a",
            "",
            "b",
            "### not a heading",
            "## Non-goals",
            "- x",
            "- y",
            "## Acceptance criteria",
            "AC-1: z",
            "## Touchpoints",
            "Expected to change: a",
            "Must NOT change:",
        ];
        let located = locate(&lines, INTENT).unwrap();
        assert_eq!(located.len(), 4);
        assert_eq!(located[0].heading_line, 4);
        assert_eq!(located[0].body, ["a", "", "b", "### not a heading"]);
        assert_eq!(located[0].body_first_line, 5);
        assert_eq!(located[3].body, ["Expected to change: a", "Must NOT change:"]);
    }

    #[test]
    fn unknown_duplicate_missing_and_misordered_are_all_refused() {
        let base = [
            "## Goal",
            "## Non-goals",
            "## Acceptance criteria",
            "## Touchpoints",
        ];
        assert!(locate(&base, INTENT).is_ok());

        let mut unknown = base.to_vec();
        unknown.push("## Touchpoint");
        assert_eq!(
            locate(&unknown, INTENT).unwrap_err().status,
            Status::UnknownSection
        );

        let mut duplicate = base.to_vec();
        duplicate.push("## Touchpoints");
        assert_eq!(
            locate(&duplicate, INTENT).unwrap_err().status,
            Status::DuplicateSection
        );

        assert_eq!(
            locate(&base[..3], INTENT).unwrap_err().status,
            Status::MissingSection
        );

        let reordered = ["## Non-goals", "## Goal", "## Acceptance criteria", "## Touchpoints"];
        assert_eq!(
            locate(&reordered, INTENT).unwrap_err().status,
            Status::SectionOrder
        );
    }

    /// TM §4.5: a Change document carrying `## Goal` reports `unknown-section`,
    /// "ID §8.2 step 7 checks unknown before missing".
    #[test]
    fn unknown_is_checked_before_missing() {
        let lines = ["## Goal", "## Current behavior"];
        assert_eq!(
            locate(&lines, INTENT_CHANGE).unwrap_err().status,
            Status::UnknownSection
        );
    }

    /// TM §4.2: "`## Current behaviour` has key `current behaviour`, is not in
    /// the table, and is `unknown-section`."
    #[test]
    fn the_british_spelling_is_a_different_key() {
        let lines = [
            "## Current behaviour",
            "## Target behavior",
            "## Non-goals",
            "## Invariants",
            "## Acceptance criteria",
            "## Touchpoints",
        ];
        assert_eq!(
            locate(&lines, INTENT_CHANGE).unwrap_err().status,
            Status::UnknownSection
        );
    }

    /// ID §4.1's one visible consequence: "a line whose first three bytes are
    /// `## ` begins a section **wherever it appears**, including inside what an
    /// author intended as a code fence."
    #[test]
    fn a_heading_inside_an_intended_code_fence_still_begins_a_section() {
        let lines = ["## Goal", "```", "## Non-goals", "```"];
        // `## Non-goals` opened section 2, so the Goal body stops at it.
        let located = locate(&lines, INTENT).unwrap_err();
        // The set is now {goal, non-goals} and the mandatory rest is absent.
        assert_eq!(located.status, Status::MissingSection);
    }
}
