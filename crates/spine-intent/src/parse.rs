//! ID §8.2's ordered parse, and ID §5.6's parse result.
//!
//! **Why the order is normative.** ID §8.2: "two implementations checking the
//! same things in a different order report different statuses for a document
//! that is wrong in several ways at once". And the status is not a diagnostic —
//! ID §8.3 makes a landing whose document does not parse `unattested`,
//! "reported and counted forever".
//!
//! ```text
//! 1. canonical form and the document bound          exit 2
//! 2. the title line, its id, and the id vs the path exit 4
//! 3. the header line and `Template:`'s syntax       exit 4
//! 4. variant selection, then the prefix agreement   exit 4
//! 5. the (variant, version) pair vs held parsers    exit 3
//! 6. `Supersedes:` and the preamble                 exit 4
//! 7. section headings                               exit 4
//! 8. each section's body, in ordinal order          exit 4
//! 9. shape bounds, in section ordinal order         exit 4
//! 10. Layer 2, in §8.1's table order                exit 5
//! ```
//!
//! **Why step 4 precedes step 5.** "Variant selection and the prefix check are
//! functions of the document alone; the parser lookup is a function of the
//! *reader*. Ordering the reader-independent failure first means two binaries
//! holding different parser sets still report the same status for a document
//! that is wrong in both ways."

use crate::ac::{self, Ac};
use crate::canon;
use crate::header::{self, Variant};
use crate::sections::{self, BodyGrammar, Polarity, TouchpointBody};
use crate::status::{Refusal, Status};
use spine_resolve::Pattern;
use spine_resolve::pragma::IntentId;

/// The template versions this binary holds a parser for, for every variant.
///
/// ID §3.2: "A binary keeps a parser for every pair it has ever shipped". Two
/// generations exist — version 2, and TM §9.2's uniform version 1 ("`v@1` is
/// `v@2` plus a permitted `Status` header field at order 5, parsed and
/// discarded") — and no release has shipped either, so this list is complete
/// by construction rather than by history.
pub const HELD_VERSIONS: &[u32] = &[1, 2];

/// ID §8.2 step 5. A pair outside this is `template-version-unknown`, exit 3:
/// "never a partial parse, never a guess, never a fall back to the newest
/// version held, and never another variant's parser for the same number".
pub fn holds_parser(_variant: Variant, version: u32) -> bool {
    HELD_VERSIONS.contains(&version)
}

/// ID §5.6's parse result, extended by TM §4.4's three variant-conditional
/// members.
///
/// "Two implementations agree iff they produce this value for every document."
/// Deliberately **not** members: the Goal's text, the non-goals' texts, the ACs'
/// texts, the heading parentheticals, the blank-line layout, and the header's
/// `Template:` spelling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    /// From the path, and equal to the title's.
    pub id: IntentId,
    /// Read from the header's variant token, or derived by ID §3.3 for a legacy
    /// bare value.
    pub variant: Variant,
    /// The `Template:` value's version, in either spelling.
    pub template: u32,
    /// The title line's text, verbatim.
    pub title: String,
    /// The `Owner` field's value, verbatim, `@` retained.
    pub owner: String,
    pub ticket: Option<String>,
    pub constitution: u32,
    pub supersedes: Option<IntentId>,
    /// TM §4.4: "`true` iff the variant's table has a `goal` section — so
    /// `true` for `intent` and `intent-bug`, **`false` for `intent-change`**".
    /// ID §5.6 introduced it "so the shape is total across variants where Goal
    /// is replaced".
    pub goal_present: bool,
    /// TM §4.4, present iff `variant = "intent-change"`.
    pub current_behavior_present: Option<bool>,
    /// TM §4.4, present iff `variant = "intent-change"`.
    pub target_behavior_present: Option<bool>,
    /// 2 … 256.
    pub non_goal_count: usize,
    /// TM §4.2, present iff `variant = "intent-change"`; 1 … 256.
    pub invariant_count: Option<usize>,
    /// `[1, 2, …, k]`, `1 ≤ k ≤ 6`.
    pub acs: Vec<u8>,
    /// Patterns as written, in document order, duplicates removed keeping the
    /// first occurrence; length ≥ 1.
    pub expected: Vec<Pattern>,
    /// As `expected`; length ≥ 0.
    pub forbidden: Vec<Pattern>,
    /// ID §5.5: `true` if the section is absent or its body has no non-empty
    /// line.
    pub open_questions_empty: bool,
    /// ID §6.6's provenance line for every `expected` declaration — the label
    /// line's, "not the individual pattern's, since several patterns share one
    /// line".
    pub expected_line: usize,
    /// As `expected_line`, for `forbidden`.
    pub forbidden_line: usize,
}

/// One `declares` edge's material (ID §6.6): the pattern, its polarity, and the
/// line its label sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Declaration<'a> {
    pub pattern: &'a Pattern,
    pub polarity: Polarity,
    pub line: usize,
}

impl Parsed {
    /// The `intent.template` attr `dump.md` §7.2 records: the canonical
    /// `<variant>@<n>`, "reconstructed rather than copied, so a legacy
    /// `Template: v2` document and a `Template: intent@2` document with
    /// otherwise identical bytes … would still yield the same attr."
    pub fn template_attr(&self) -> String {
        format!("{}@{}", self.variant, self.template)
    }

    /// The AC labels, `AC-<n>`, in document order.
    pub fn ac_labels(&self) -> Vec<String> {
        self.acs.iter().map(|n| format!("AC-{n}")).collect()
    }

    /// Every `declares` edge this document yields, expected first.
    ///
    /// "A pattern appearing in both polarities is impossible
    /// (`polarity-conflict`), so no `code_unit` carries two `declares` edges
    /// from one intent, and the edge set is a set under `(from, to, kind)`."
    pub fn declares(&self) -> Vec<Declaration<'_>> {
        let expected = self.expected.iter().map(|pattern| Declaration {
            pattern,
            polarity: Polarity::Expected,
            line: self.expected_line,
        });
        let forbidden = self.forbidden.iter().map(|pattern| Declaration {
            pattern,
            polarity: Polarity::Forbidden,
            line: self.forbidden_line,
        });
        expected.chain(forbidden).collect()
    }

    /// TM §5.3's reproduction AC: "For a document whose variant is
    /// `intent-bug`, the **reproduction AC** is the AC numbered 1. Nothing
    /// marks it; its position is its identity."
    pub fn reproduction_ac(&self) -> Option<u8> {
        (self.variant == Variant::IntentBug).then_some(1)
    }

    /// The landing commit's subject line: PB §5.5's `<ID>: ` prefix and the
    /// title's bytes.
    ///
    /// ID §4.2: "**The landing subject is derived from these bytes, not written
    /// beside them** — decision 6 of PB v0.19 — and **G9 recomputes it and
    /// checks it**. So the title is a gate input, not only a display string: a
    /// gated landing whose subject is not `<ID>: ` ++ these exact bytes fails
    /// G9." The 72-byte bound keeps this inside 81 columns.
    ///
    /// It stays **outside `envelope=`**, "so nothing here changes a digest —
    /// deriving it cost no digest change, which is why it was decided at all".
    pub fn landing_subject(&self) -> String {
        format!("{}: {}", self.id, self.title)
    }
}

/// Parse a document.
///
/// ID §3.4: "The parse result is a function of exactly two inputs: the
/// document's bytes, and the id taken from its path. Not of the repository, not
/// of the tree, not of the manifest, not of the clock, not of the local git
/// version." Those two inputs are this signature.
pub fn parse(bytes: &[u8], path_id: &IntentId) -> Result<Parsed, Refusal> {
    // ---- step 1 ----
    let doc = canon::check(bytes)?;
    let lines = doc.lines();

    // ---- step 2: the title line ----
    let title = header::parse_title(lines[0], path_id).map_err(|s| Refusal::at(s, 1))?;
    // "A line whose first two bytes are `# ` may not appear anywhere else in
    // the document … because two concatenated documents must not parse as one."
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.as_bytes().starts_with(b"# ") {
            return Err(Refusal::at(Status::DuplicateTitle, i + 1));
        }
    }
    // ID §4.5 places `truncated` with the preamble; it is raised here because
    // steps 3 … 5 have no input without line 2, and no step-3, -4 or -5 status
    // is reachable for a one-line document — so the two positions are
    // observationally identical and only this one is well-posed.
    if lines.len() < 2 {
        return Err(Refusal::whole(Status::Truncated));
    }

    // ---- step 3: the header line ----
    let head = header::parse_header(lines[1]).map_err(|s| Refusal::at(s, 2))?;

    // ---- step 4: variant selection, then prefix agreement ----
    let variant = match head.template.variant {
        Some(v) => v,
        None => sections::variant_legacy(lines, title.id.prefix()),
    };
    if !variant.agrees_with_prefix(title.id.prefix()) {
        return Err(Refusal::at(Status::VariantPrefixMismatch, 2));
    }

    // ---- step 5: the (variant, version) pair ----
    if !holds_parser(variant, head.template.version) {
        return Err(Refusal::at(Status::TemplateVersionUnknown, 2));
    }

    // ---- step 6: `Supersedes:` and the preamble ----
    let first_heading = lines
        .iter()
        .position(|line| sections::is_heading(line))
        .map(|i| i + 1)
        .unwrap_or(lines.len() + 1);

    // DERIVED: ID §4.4's "a `Supersedes:` line may only be line 3" is scoped to
    // the preamble. Inside a section body, `Supersedes:` is body text under
    // ID §4.10's classification, which tests only `- `, `AC-`, and a leading
    // blank; ID §4.1's document production admits a `supersedes-line` in the
    // preamble alone. The motivating case — a second line written directly
    // beneath the first — falls inside the preamble either way.
    let mut supersedes = None;
    for line_no in 3..first_heading {
        let line = lines[line_no - 1];
        if !line.starts_with("Supersedes:") {
            continue;
        }
        if line_no != 3 {
            return Err(Refusal::at(Status::BadSupersedes, line_no));
        }
        supersedes = Some(header::parse_supersedes(line).map_err(|s| Refusal::at(s, line_no))?);
    }
    // "After the title line, the header line and the optional `Supersedes:`
    // line, every line up to the first heading line must be empty."
    let preamble_end = if supersedes.is_some() { 4 } else { 3 };
    for line_no in preamble_end..first_heading {
        if !lines[line_no - 1].is_empty() {
            return Err(Refusal::at(Status::StrayPreamble, line_no));
        }
    }
    // ---- step 7: section headings ----
    let table = sections::table(variant);
    let located = sections::locate(lines, table)?;

    // ---- step 8: each section's body, in ordinal order ----
    let mut non_goal_count = 0usize;
    let mut invariant_count: Option<usize> = None;
    let mut acs: Vec<Ac> = Vec::new();
    let mut touchpoints: Option<TouchpointBody> = None;
    let mut open_questions_empty = true;

    for section in &located {
        match section.spec.body {
            BodyGrammar::Prose => section.parse_prose()?,
            BodyGrammar::Bullet => {
                let count = section.parse_bullets()?;
                if section.spec.key == "invariants" {
                    invariant_count = Some(count);
                } else {
                    non_goal_count = count;
                }
            }
            BodyGrammar::Ac => acs = section.parse_acs()?,
            BodyGrammar::Touchpoints => touchpoints = Some(section.parse_touchpoints()?),
            BodyGrammar::Free => open_questions_empty = section.is_empty_body(),
        }
    }

    // ---- step 9: shape bounds, in section ordinal order ----
    let mut expected = Vec::new();
    let mut forbidden = Vec::new();
    let mut expected_line = 0usize;
    let mut forbidden_line = 0usize;

    for section in &located {
        let at = section.heading_line;
        match section.spec.key {
            "non-goals" => {
                if non_goal_count < sections::MIN_NON_GOALS {
                    return Err(Refusal::at(Status::NonGoalsTooFew, at));
                }
                if non_goal_count > sections::MAX_BULLETS {
                    return Err(Refusal::at(Status::TooManyNonGoals, at));
                }
            }
            "invariants" => {
                let count = invariant_count.unwrap_or(0);
                if count < sections::MIN_INVARIANTS {
                    return Err(Refusal::at(Status::InvariantsTooFew, at));
                }
                if count > sections::MAX_BULLETS {
                    return Err(Refusal::at(Status::TooManyInvariants, at));
                }
            }
            "acceptance criteria" => ac::check_bounds(&acs).map_err(|s| Refusal::at(s, at))?,
            "touchpoints" => {
                let body = touchpoints.as_ref().expect("located and parsed at step 8");
                // The bound is a resource bound on what a pushed branch may make
                // my landing do (ID §2.3), so it counts the fields as written
                // rather than the set they dedup to.
                if body.expected.len() > sections::MAX_TOUCHPOINTS
                    || body.forbidden.len() > sections::MAX_TOUCHPOINTS
                {
                    return Err(Refusal::at(Status::TooManyTouchpoints, at));
                }
                expected = sections::dedup(&body.expected);
                forbidden = sections::dedup(&body.forbidden);
                if expected.is_empty() {
                    return Err(Refusal::at(Status::NoExpectedTouchpoint, at));
                }
                // "A pattern appearing **byte-identically in both polarities**
                // is `polarity-conflict` … Overlap that is not byte-identical —
                // `expected: src/`, `forbidden: src/auth/` — is legal,
                // meaningful and common."
                if expected
                    .iter()
                    .any(|e| forbidden.iter().any(|f| f.as_str() == e.as_str()))
                {
                    return Err(Refusal::at(Status::PolarityConflict, at));
                }
                expected_line = body.expected_line;
                forbidden_line = body.forbidden_line;
            }
            _ => {}
        }
    }

    let is_change = variant == Variant::IntentChange;
    Ok(Parsed {
        id: title.id,
        variant,
        template: head.template.version,
        title: title.text,
        owner: head.owner,
        ticket: head.ticket,
        constitution: head.constitution,
        supersedes,
        goal_present: table.iter().any(|s| s.key == "goal"),
        current_behavior_present: is_change.then_some(true),
        target_behavior_present: is_change.then_some(true),
        non_goal_count,
        invariant_count,
        acs: acs.iter().map(|a| a.number).collect(),
        expected,
        forbidden,
        open_questions_empty,
        expected_line,
        forbidden_line,
    })
}

/// The repository facts ID §8.1's Layer 2 reads. They are supplied, never
/// looked up: keeping them out of [`parse`] is what makes ID §3.4's "the parse
/// is a function of exactly two inputs" true.
#[derive(Debug, Clone, Copy)]
pub struct SignoffFacts<'a> {
    /// The full ref name the sign-off is running on, e.g.
    /// `refs/heads/intent/INT-042`.
    pub branch: &'a str,
    /// ID §2.4: the worktree file at `intents/<id>.md`, if it exists, hashes —
    /// via `git hash-object --path` — to the head blob's id. "Signing bytes a
    /// human is not looking at is the failure this closes."
    pub worktree_clean: bool,
    /// `resign[variant]` from the manifest at trunk, read for the variant the
    /// **header** names — ID §8.1: "not derived".
    pub resign_floor: u32,
}

/// ID §8.1's Layer 2, in that table's order (ID §8.2 step 10). Checked only by
/// `spine new --sign`, over a successful Layer 1 parse.
///
/// ID §11.5 defends the split: "Layer 2 is about the document's *stage*, Layer 1
/// about its *shape*, and only shape can be checked years later against a
/// sealed envelope."
pub fn check_signoff(parsed: &Parsed, facts: &SignoffFacts<'_>) -> Result<(), Refusal> {
    if facts.branch != format!("refs/heads/intent/{}", parsed.id) {
        return Err(Refusal::whole(Status::WrongBranch));
    }
    if !facts.worktree_clean {
        return Err(Refusal::whole(Status::WorktreeDirty));
    }
    if !parsed.open_questions_empty {
        return Err(Refusal::whole(Status::OpenQuestionsNonempty));
    }
    if parsed.template < facts.resign_floor {
        return Err(Refusal::whole(Status::TemplateBelowResignFloor));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> IntentId {
        IntentId::parse(s).unwrap()
    }

    /// A document assembled from parts, so a test can break exactly one rule.
    fn doc(body: &str) -> String {
        format!(
            "# INT-042: t\nOwner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: v1\n\n{body}"
        )
    }

    const SECTIONS: &str = "## Goal\ng\n\n## Non-goals\n- a\n- b\n\n## Acceptance criteria\nAC-1: x\n\n## Touchpoints\nExpected to change: src/\nMust NOT change:\n";

    fn refuse(d: &str) -> Status {
        parse(d.as_bytes(), &id("INT-042"))
            .expect_err("expected a refusal")
            .status
    }

    #[test]
    fn the_assembled_minimal_document_parses() {
        let p = parse(doc(SECTIONS).as_bytes(), &id("INT-042")).unwrap();
        assert_eq!(p.variant, Variant::Intent);
        assert_eq!(p.template, 2);
        assert_eq!(p.non_goal_count, 2);
        assert_eq!(p.acs, [1]);
        assert_eq!(p.forbidden.len(), 0);
        assert!(p.open_questions_empty);
        assert_eq!(p.template_attr(), "intent@2");
    }

    // ---- ID §8.2's order ----

    /// "A document breaking rules in two steps reports the earlier step's
    /// status." Each row breaks two steps and must report the earlier one.
    #[test]
    fn a_document_wrong_in_two_steps_reports_the_earlier_step() {
        // step 1 (CR) before step 2 (bad id).
        assert_eq!(refuse("# TASK-042: t\r\n"), Status::CrByte);
        // step 2 (bad id) before step 3 (unknown header field).
        let d = "# TASK-042: t\nReviewer: @a \u{00B7} Template: intent@2\n";
        assert_eq!(refuse(d), Status::BadId);
        // step 3 (bad template) before step 4 (prefix mismatch).
        let d = "# INT-042: t\nOwner: @a \u{00B7} Template: Intent@2 \u{00B7} Constitution: v1\n";
        assert_eq!(refuse(d), Status::BadTemplate);
        // step 4 (prefix mismatch) before step 5 (unknown version). ID §8.2:
        // ordering the reader-independent failure first means "a
        // `variant-prefix-mismatch` … is never masked by a version one of them
        // happens not to hold".
        let d = "# INT-042: t\nOwner: @a \u{00B7} Template: intent-bug@9 \u{00B7} Constitution: v1\n";
        assert_eq!(refuse(d), Status::VariantPrefixMismatch);
        // step 6 (stray preamble) before step 7 (missing section).
        let d = "# INT-042: t\nOwner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: v1\nstray\n";
        assert_eq!(refuse(d), Status::StrayPreamble);
        // step 7 (unknown section) before step 8 (empty section).
        let d = doc("## Goal\n\n## Nope\n");
        assert_eq!(refuse(&d), Status::UnknownSection);
        // step 8 (empty section) before step 9 (non-goals too few).
        let d = doc("## Goal\n\n## Non-goals\n- a\n\n## Acceptance criteria\nAC-1: x\n\n## Touchpoints\nExpected to change: src/\nMust NOT change:\n");
        assert_eq!(refuse(&d), Status::EmptySection);
    }

    /// ID §8.2 step 5's exit is 3, not 4 — a different class from every other
    /// refusal in the parse.
    #[test]
    fn an_unheld_version_exits_three() {
        let d = "# INT-042: t\nOwner: @a \u{00B7} Template: intent@3 \u{00B7} Constitution: v1\n";
        let e = parse(d.as_bytes(), &id("INT-042")).unwrap_err();
        assert_eq!(e.status, Status::TemplateVersionUnknown);
        assert_eq!(e.exit_code(), 3);
    }

    #[test]
    fn a_one_line_document_is_truncated() {
        assert_eq!(refuse("# INT-042: t\n"), Status::Truncated);
    }

    #[test]
    fn a_second_title_line_is_duplicate_title() {
        let d = doc(&format!("{SECTIONS}\n# INT-042: t\n"));
        assert_eq!(refuse(&d), Status::DuplicateTitle);
    }

    // ---- ID §3.3 / TM §4.5's qualified mis-templating table ----

    #[test]
    fn tm_4_5s_qualified_table_row_by_row() {
        // `INT-`, `intent@2`, `## Goal`, no `## Invariants` — parses.
        assert!(parse(doc(SECTIONS).as_bytes(), &id("INT-042")).is_ok());

        // `INT-`, `intent@2`, `## Invariants` present — `unknown-section`.
        let d = doc("## Goal\ng\n\n## Invariants\n- x\n");
        assert_eq!(refuse(&d), Status::UnknownSection);

        // `INT-`, `intent-bug@2` — `variant-prefix-mismatch`, "before any
        // section is read".
        let d = format!(
            "# INT-042: t\nOwner: @a \u{00B7} Template: intent-bug@2 \u{00B7} Constitution: v1\n\n{SECTIONS}"
        );
        assert_eq!(refuse(&d), Status::VariantPrefixMismatch);

        // `BUG-`, `intent@2` — the same.
        let d = format!(
            "# BUG-051: t\nOwner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: v1\n\n{SECTIONS}"
        );
        assert_eq!(
            parse(d.as_bytes(), &id("BUG-051")).unwrap_err().status,
            Status::VariantPrefixMismatch
        );

        // `BUG-`, `intent-bug@2`, feature sections — parses, "AC-1 **is** the
        // reproduction whether the author meant it or not".
        let d = format!(
            "# BUG-051: t\nOwner: @a \u{00B7} Template: intent-bug@2 \u{00B7} Constitution: v1\n\n{SECTIONS}"
        );
        let p = parse(d.as_bytes(), &id("BUG-051")).unwrap();
        assert_eq!(p.variant, Variant::IntentBug);
        assert_eq!(p.reproduction_ac(), Some(1));

        // `Template: chore@2` — `template-variant-unknown`.
        let d = format!(
            "# INT-042: t\nOwner: @a \u{00B7} Template: chore@2 \u{00B7} Constitution: v1\n\n{SECTIONS}"
        );
        assert_eq!(refuse(&d), Status::TemplateVariantUnknown);
    }

    #[test]
    fn tm_4_5s_legacy_table_derives_the_variant_from_the_probe() {
        // `INT-`, `v2`, `## Goal`, no `## Invariants` — `intent`.
        let d = format!(
            "# INT-042: t\nOwner: @a \u{00B7} Template: v2 \u{00B7} Constitution: v1\n\n{SECTIONS}"
        );
        let p = parse(d.as_bytes(), &id("INT-042")).unwrap();
        assert_eq!(p.variant, Variant::Intent);
        assert_eq!(p.template, 2);

        // `BUG-`, `v2`, feature sections — `intent-bug`, and it parses.
        let d = format!(
            "# BUG-051: t\nOwner: @a \u{00B7} Template: v2 \u{00B7} Constitution: v1\n\n{SECTIONS}"
        );
        let p = parse(d.as_bytes(), &id("BUG-051")).unwrap();
        assert_eq!(p.variant, Variant::IntentBug);

        // "`INT-` id carrying a bug's content, `v2` → parses as a Feature;
        // PB §4.3's outright refusal never applies — the failure with no
        // detector, and the reason §3.3 exists."
        let d = format!(
            "# INT-051: t\nOwner: @a \u{00B7} Template: v2 \u{00B7} Constitution: v1\n\n{SECTIONS}"
        );
        let p = parse(d.as_bytes(), &id("INT-051")).unwrap();
        assert_eq!(p.variant, Variant::Intent);
        assert_eq!(p.reproduction_ac(), None);

        // "any, bare `v3` or higher → `bad-template`".
        let d = format!(
            "# INT-042: t\nOwner: @a \u{00B7} Template: v3 \u{00B7} Constitution: v1\n\n{SECTIONS}"
        );
        assert_eq!(refuse(&d), Status::BadTemplate);
    }

    /// ID §5.6: "a legacy `Template: v2` document and a `Template: intent@2`
    /// document with otherwise identical bytes … would still yield the same
    /// attr", and the spelling "leaves no trace in the graph".
    #[test]
    fn the_two_spellings_of_version_two_yield_one_parse_result() {
        let qualified = doc(SECTIONS);
        let legacy = format!(
            "# INT-042: t\nOwner: @a \u{00B7} Template: v2 \u{00B7} Constitution: v1\n\n{SECTIONS}"
        );
        assert_ne!(qualified, legacy);
        let a = parse(qualified.as_bytes(), &id("INT-042")).unwrap();
        let b = parse(legacy.as_bytes(), &id("INT-042")).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.template_attr(), "intent@2");
    }

    // ---- ID §4.4, §4.5 ----

    #[test]
    fn supersedes_is_line_three_and_only_line_three() {
        let d = format!(
            "# INT-042: t\nOwner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: v1\nSupersedes: INT-017\n\n{SECTIONS}"
        );
        let p = parse(d.as_bytes(), &id("INT-042")).unwrap();
        assert_eq!(p.supersedes.unwrap().as_str(), "INT-017");

        let d = format!(
            "# INT-042: t\nOwner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: v1\n\nSupersedes: INT-017\n\n{SECTIONS}"
        );
        assert_eq!(refuse(&d), Status::BadSupersedes);

        let d = format!(
            "# INT-042: t\nOwner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: v1\nSupersedes: INT-017\nSupersedes: INT-018\n\n{SECTIONS}"
        );
        assert_eq!(refuse(&d), Status::BadSupersedes);
    }

    #[test]
    fn the_template_blank_line_is_permitted_but_not_required() {
        let d = format!(
            "# INT-042: t\nOwner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: v1\n{SECTIONS}"
        );
        assert!(parse(d.as_bytes(), &id("INT-042")).is_ok());
        let d = format!(
            "# INT-042: t\nOwner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: v1\n\n\n{SECTIONS}"
        );
        assert!(parse(d.as_bytes(), &id("INT-042")).is_ok());
    }

    // ---- step 9 ----

    #[test]
    fn the_shape_bounds_fire_in_section_ordinal_order() {
        // Non-goals (ordinal 2) before acceptance criteria (ordinal 3).
        let d = doc("## Goal\ng\n\n## Non-goals\n- a\n\n## Acceptance criteria\nAC-1: x\nAC-1: y\n\n## Touchpoints\nExpected to change: src/\nMust NOT change:\n");
        assert_eq!(refuse(&d), Status::NonGoalsTooFew);
    }

    #[test]
    fn an_empty_expected_list_is_no_expected_touchpoint() {
        let d = doc("## Goal\ng\n\n## Non-goals\n- a\n- b\n\n## Acceptance criteria\nAC-1: x\n\n## Touchpoints\nExpected to change:\nMust NOT change:\n");
        assert_eq!(refuse(&d), Status::NoExpectedTouchpoint);
    }

    /// ID §5.4, and the case ID §11.8 says is "legal, meaningful and common".
    #[test]
    fn only_a_byte_identical_pattern_in_both_polarities_is_a_conflict() {
        let d = doc("## Goal\ng\n\n## Non-goals\n- a\n- b\n\n## Acceptance criteria\nAC-1: x\n\n## Touchpoints\nExpected to change: src/\nMust NOT change: src/\n");
        assert_eq!(refuse(&d), Status::PolarityConflict);

        let d = doc("## Goal\ng\n\n## Non-goals\n- a\n- b\n\n## Acceptance criteria\nAC-1: x\n\n## Touchpoints\nExpected to change: src/\nMust NOT change: src/auth/\n");
        assert!(parse(d.as_bytes(), &id("INT-042")).is_ok());
    }

    #[test]
    fn a_pattern_repeated_within_one_polarity_is_deduplicated_not_refused() {
        let d = doc("## Goal\ng\n\n## Non-goals\n- a\n- b\n\n## Acceptance criteria\nAC-1: x\n\n## Touchpoints\nExpected to change: src/, src/\nMust NOT change:\n");
        let p = parse(d.as_bytes(), &id("INT-042")).unwrap();
        assert_eq!(p.expected.len(), 1);
    }

    /// ID §6.1's `**` decision, which OPEN-3 closed on the owner's terms: "an
    /// unbounded `forbidden` set stays legal".
    #[test]
    fn an_unbounded_forbidden_set_parses() {
        let d = doc("## Goal\ng\n\n## Non-goals\n- a\n- b\n\n## Acceptance criteria\nAC-1: x\n\n## Touchpoints\nExpected to change: src/\nMust NOT change: **\n");
        let p = parse(d.as_bytes(), &id("INT-042")).unwrap();
        assert_eq!(p.forbidden[0].as_str(), "**");
    }

    // ---- ID §5.5 / §8.1's Layer 2 ----

    #[test]
    fn open_questions_is_a_stage_condition_not_a_parse_condition() {
        let d = doc(&format!("{SECTIONS}\n## Open questions\nWhat about refunds?\n"));
        let p = parse(d.as_bytes(), &id("INT-042")).unwrap();
        assert!(!p.open_questions_empty);

        let facts = SignoffFacts {
            branch: "refs/heads/intent/INT-042",
            worktree_clean: true,
            resign_floor: 2,
        };
        let e = check_signoff(&p, &facts).unwrap_err();
        assert_eq!(e.status, Status::OpenQuestionsNonempty);
        assert_eq!(e.exit_code(), 5);
    }

    #[test]
    fn layer_two_is_checked_in_id_8_1s_table_order() {
        let p = parse(doc(SECTIONS).as_bytes(), &id("INT-042")).unwrap();
        let good = SignoffFacts {
            branch: "refs/heads/intent/INT-042",
            worktree_clean: true,
            resign_floor: 2,
        };
        assert!(check_signoff(&p, &good).is_ok());

        // Every precondition broken at once reports the first in the table.
        let all_bad = SignoffFacts {
            branch: "refs/heads/main",
            worktree_clean: false,
            resign_floor: 3,
        };
        assert_eq!(check_signoff(&p, &all_bad).unwrap_err().status, Status::WrongBranch);

        let dirty = SignoffFacts {
            worktree_clean: false,
            resign_floor: 3,
            ..good
        };
        assert_eq!(check_signoff(&p, &dirty).unwrap_err().status, Status::WorktreeDirty);

        let below = SignoffFacts {
            resign_floor: 3,
            ..good
        };
        assert_eq!(
            check_signoff(&p, &below).unwrap_err().status,
            Status::TemplateBelowResignFloor
        );
    }

    /// ID §8.1: the floor is read for "the variant read from the header (§3.2),
    /// not derived" — so a document at the floor passes and one below it does
    /// not, whatever the variant.
    #[test]
    fn a_document_at_the_floor_signs_and_one_below_it_does_not() {
        let p = parse(doc(SECTIONS).as_bytes(), &id("INT-042")).unwrap();
        for (floor, ok) in [(0u32, true), (1, true), (2, true), (3, false)] {
            let facts = SignoffFacts {
                branch: "refs/heads/intent/INT-042",
                worktree_clean: true,
                resign_floor: floor,
            };
            assert_eq!(check_signoff(&p, &facts).is_ok(), ok, "floor {floor}");
        }
    }

    // ---- the remaining shape bounds, each at its own edge ----

    fn with_sections(sections: &str) -> Status {
        refuse(&doc(sections))
    }

    #[test]
    fn the_bullet_and_touchpoint_maxima_are_256() {
        let two_fifty_six = (0..256).map(|i| format!("- n{i}")).collect::<Vec<_>>().join("\n");
        let two_fifty_seven = (0..257).map(|i| format!("- n{i}")).collect::<Vec<_>>().join("\n");
        let build = |bullets: &str| {
            format!("## Goal\ng\n\n## Non-goals\n{bullets}\n\n## Acceptance criteria\nAC-1: x\n\n## Touchpoints\nExpected to change: src/\nMust NOT change:\n")
        };
        assert!(parse(doc(&build(&two_fifty_six)).as_bytes(), &id("INT-042")).is_ok());
        assert_eq!(with_sections(&build(&two_fifty_seven)), Status::TooManyNonGoals);

        let list = |n: usize| (0..n).map(|i| format!("p{i}")).collect::<Vec<_>>().join(", ");
        let build = |patterns: String| {
            format!("## Goal\ng\n\n## Non-goals\n- a\n- b\n\n## Acceptance criteria\nAC-1: x\n\n## Touchpoints\nExpected to change: {patterns}\nMust NOT change:\n")
        };
        assert!(parse(doc(&build(list(256))).as_bytes(), &id("INT-042")).is_ok());
        assert_eq!(with_sections(&build(list(257))), Status::TooManyTouchpoints);
    }

    /// TM §4.2's own bounds, on the one variant that has the section.
    fn change_doc(invariants: &str) -> String {
        format!(
            "# INT-043: t\nOwner: @a \u{00B7} Template: intent-change@2 \u{00B7} Constitution: v1\n\n\
             ## Current behavior\nc\n\n## Target behavior\nt\n\n## Non-goals\n- a\n- b\n\n\
             ## Invariants\n{invariants}\n\n## Acceptance criteria\nAC-1: x\n\n\
             ## Touchpoints\nExpected to change: src/\nMust NOT change:\n"
        )
    }

    #[test]
    fn invariants_takes_one_to_256_and_zero_is_invariants_too_few() {
        let p = parse(change_doc("- stays true").as_bytes(), &id("INT-043")).unwrap();
        assert_eq!(p.invariant_count, Some(1));
        assert!(!p.goal_present);

        let empty = change_doc("").replace("## Invariants\n\n\n", "## Invariants\n\n");
        assert_eq!(
            parse(empty.as_bytes(), &id("INT-043")).unwrap_err().status,
            Status::InvariantsTooFew
        );

        let many = (0..257).map(|i| format!("- n{i}")).collect::<Vec<_>>().join("\n");
        assert_eq!(
            parse(change_doc(&many).as_bytes(), &id("INT-043")).unwrap_err().status,
            Status::TooManyInvariants
        );
    }

    // ---- ID §7.4 ----

    /// ID §7.4's three cases, all decided the same way: "`J` contributes **no
    /// lease**, and the condition is reported as a diagnostic, not as a wire on
    /// my landing. … In all three, my landing proceeds. **A branch cannot deny
    /// service by pushing a document my binary cannot read.**"
    ///
    /// The rule this crate owes is that each of the three *refuses* rather than
    /// yielding a partial parse a caller could mistake for a lease. The
    /// gate-side consequence — proceed, and report — is PB §5.4's.
    #[test]
    fn another_branchs_unreadable_document_refuses_rather_than_half_parsing() {
        // 1. It does not parse.
        let broken = doc("## Goal\n\n");
        assert!(parse(broken.as_bytes(), &id("INT-042")).is_err());

        // 2. Its `Template:` version is unknown to my binary.
        let future = format!(
            "# INT-042: t\nOwner: @a \u{00B7} Template: intent@7 \u{00B7} Constitution: v1\n\n{SECTIONS}"
        );
        assert_eq!(
            parse(future.as_bytes(), &id("INT-042")).unwrap_err().exit_code(),
            3
        );

        // 3. It exceeds a bound of ID §2.3 — "and my landing has spent at most
        // 64 KiB of parsing on it".
        let huge = vec![b'a'; crate::canon::MAX_DOCUMENT + 1];
        assert_eq!(
            parse(&huge, &id("INT-042")).unwrap_err().status,
            Status::DocumentTooLarge
        );
    }

    // ---- ID §2.1's no-normalisation rule, from the parse side ----

    /// ID §15 item 5: "No input is Unicode-normalised, and no document is
    /// refused for being un-normalised." ID §2.1: "An NFC document and its NFD
    /// counterpart are two different documents with two different blob ids and
    /// two different signatures."
    #[test]
    fn an_nfc_title_and_its_nfd_counterpart_both_parse_and_stay_different() {
        // U+00E9, and its decomposition U+0065 U+0301.
        let nfc = doc(SECTIONS).replace("# INT-042: t", "# INT-042: caf\u{00E9}");
        let nfd = doc(SECTIONS).replace("# INT-042: t", "# INT-042: cafe\u{0301}");
        let a = parse(nfc.as_bytes(), &id("INT-042")).unwrap();
        let b = parse(nfd.as_bytes(), &id("INT-042")).unwrap();
        assert_ne!(a.title, b.title);
        assert_eq!(a.title, "caf\u{00E9}");
        assert_eq!(b.title, "cafe\u{0301}");
    }

    /// ID §4.2 / decision 6: the subject G9 recomputes.
    #[test]
    fn the_landing_subject_is_derived_from_the_title_bytes() {
        let p = parse(doc(SECTIONS).as_bytes(), &id("INT-042")).unwrap();
        assert_eq!(p.landing_subject(), "INT-042: t");
        // 72 + the 9-byte prefix `INT-042: ` is 81 columns, and the bound is what
        // makes that true of every document that parses.
        assert!(p.landing_subject().len() <= crate::header::MAX_TITLE + 9);
    }

    // ---- ID §6.6 ----

    #[test]
    fn every_declaration_carries_its_label_lines_number_not_its_own() {
        let d = doc(SECTIONS);
        let p = parse(d.as_bytes(), &id("INT-042")).unwrap();
        let declares = p.declares();
        assert_eq!(declares.len(), 1);
        assert_eq!(declares[0].polarity.attr(), "expected");
        assert_eq!(declares[0].line, p.expected_line);
    }
}
