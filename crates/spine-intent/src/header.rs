//! The preamble: the title line (ID §4.2), the header line and its closed
//! field table (ID §4.3), `Template:` (ID §3.2), and `Supersedes:` (ID §4.4).
//!
//! The one rule here that reaches every other artifact is decision 4 of
//! PB v0.19: **`Template:` names the variant as well as the version**. ID §3.2
//! gives the mechanism rather than the taste — "G4 must index the `resign` map
//! by variant and the indexer must pick a parser by name, and neither is
//! decidable from a bare `v2`" — and TM §3.3 gives what the old derivation
//! cost: "a `--bug` document carrying an `INT-` id derived to variant `intent`,
//! parsed cleanly, and silently lost the one thing the Bug variant exists to
//! buy."
//!
//! So `Variant` is *read* here, never inferred, except on the one legacy path
//! [`crate::sections::variant_legacy`] serves.

use crate::status::Status;
use core::fmt;
use spine_resolve::pragma::{IntentId, IntentPrefix};

/// The field separator: the three bytes `0x20 0xC2 0xB7 0x20` — space,
/// U+00B7 MIDDLE DOT, space (ID §4.3).
///
/// TM §6.4.4: "The middle dot is ID §4.3's field separator … and it is grammar
/// rather than typography."
pub const FIELD_SEPARATOR: &str = " \u{00B7} ";

/// ID §4.2's title bound: 72 bytes, hard.
///
/// "`INT-042: ` plus 72 is 81 columns, so a landing's subject stays one line in
/// every tool that shows one, and the bound makes the envelope's 16 KiB
/// projection computable at `--approve` from the parse alone."
pub const MAX_TITLE: usize = 72;

/// ID §4.3's `Owner` / `Ticket` value bound.
pub const MAX_FIELD_VALUE: usize = 128;

/// TM §3.1's three variants. The token is simultaneously the `Template:`
/// variant, the manifest's `templates` key and its `resign` key — one
/// `name@version` vocabulary across all four sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Variant {
    Intent,
    IntentChange,
    IntentBug,
}

impl Variant {
    /// The closed set of three, in TM §3.1's table order.
    pub const ALL: [Variant; 3] = [Variant::Intent, Variant::IntentChange, Variant::IntentBug];

    pub fn token(self) -> &'static str {
        match self {
            Variant::Intent => "intent",
            Variant::IntentChange => "intent-change",
            Variant::IntentBug => "intent-bug",
        }
    }

    /// ID §3.2: "The variant token is matched **byte-exactly and
    /// case-sensitively**. `Intent@2` and `INTENT-CHANGE@2` are `bad-template`,
    /// not variants. The three tokens are simultaneously the manifest's
    /// `templates` and `resign` keys … and a header that casefolded where a
    /// JSON member name does not would be two spellings of one map key."
    pub fn from_token(token: &str) -> Option<Variant> {
        match token {
            "intent" => Some(Variant::Intent),
            "intent-change" => Some(Variant::IntentChange),
            "intent-bug" => Some(Variant::IntentBug),
            _ => None,
        }
    }

    /// ID §3.3's consistency table.
    ///
    /// | Id prefix | Permitted variant token |
    /// |---|---|
    /// | `BUG` | `intent-bug` |
    /// | `INT` | `intent`, `intent-change` |
    pub fn agrees_with_prefix(self, prefix: IntentPrefix) -> bool {
        match prefix {
            IntentPrefix::Bug => self == Variant::IntentBug,
            IntentPrefix::Int => self != Variant::IntentBug,
        }
    }
}

impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// How the `Template:` value was spelled.
///
/// ID §5.6: "**The header's spelling is not a member, and leaves no trace in
/// the graph.**" It is carried here only because two rules read it — the
/// legacy variant derivation (ID §3.3) and `Status`'s admissibility (ID §3.2) —
/// and it is deliberately absent from [`crate::Parsed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spelling {
    /// `Template: <variant>@<n>`.
    Qualified,
    /// `Template: v<n>`, `n ∈ {1, 2}`. ID §11.9: "no document at version 1 and
    /// no document carrying the bare spelling exists in any repository — no
    /// release has shipped".
    Legacy,
}

/// A parsed `Template:` value: version and spelling, plus the variant on the
/// qualified path only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemplateValue {
    /// `None` on the legacy path, where ID §3.3's derivation supplies it.
    pub variant: Option<Variant>,
    pub version: u32,
    pub spelling: Spelling,
}

/// ID §3.2's version production: "a decimal integer 0 … 999, in ASCII digits,
/// no leading zeros except the single digit `0`".
///
/// "The version's spelling is unique because leading zeros are forbidden, which
/// §8.4 of `templates.md` depends on."
fn parse_version(digits: &str) -> Option<u32> {
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    let n: u32 = digits.parse().ok()?;
    if n > 999 { None } else { Some(n) }
}

/// A well-formed variant token's own byte grammar: lowercase ASCII letters and
/// hyphens.
///
/// DERIVED. ID §3.2 states two clauses that a literal reading makes
/// contradictory: "A value that is not `variant "@" version` is `bad-template`;
/// a value whose variant token is outside the closed set is
/// `template-variant-unknown`" — yet the very next paragraph says "`Intent@2`
/// and `INTENT-CHANGE@2` are `bad-template`, not variants", and TM §4.5's table
/// says `Template: chore@2` is `template-variant-unknown`. All three are true
/// only if a token *shaped* like a variant but outside the set is
/// `template-variant-unknown`, while a token that is not even shaped like one
/// fails the production and is `bad-template`. This is that shape test, and it
/// classifies every example the corpus publishes correctly.
fn looks_like_a_variant_token(token: &str) -> bool {
    !token.is_empty()
        && token.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
        && !token.starts_with('-')
        && !token.ends_with('-')
}

/// ID §3.2's `Template:` value.
///
/// ```text
/// template-value := variant "@" version
/// variant        := "intent" | "intent-change" | "intent-bug"
/// version        := 0 … 999, no leading zeros except the single digit "0"
/// ```
///
/// plus the legacy bare `v<n>` at `n ∈ {1, 2}` only: "A bare `v3` or higher is
/// `bad-template`: there is no generation it could name, and accepting it would
/// create a second permanent spelling for every version yet to ship."
pub fn parse_template_value(value: &str) -> Result<TemplateValue, Status> {
    if let Some((token, digits)) = value.split_once('@') {
        let version = parse_version(digits).ok_or(Status::BadTemplate)?;
        return match Variant::from_token(token) {
            Some(variant) => Ok(TemplateValue {
                variant: Some(variant),
                version,
                spelling: Spelling::Qualified,
            }),
            None if looks_like_a_variant_token(token) => Err(Status::TemplateVariantUnknown),
            None => Err(Status::BadTemplate),
        };
    }
    // The legacy spelling. Bounded at 2 so it "can never become permanent":
    // "every version from 3 on has exactly one spelling, so a reader never has
    // to decide which of two forms a future document meant" (ID §11.9).
    let digits = value.strip_prefix('v').ok_or(Status::BadTemplate)?;
    let version = parse_version(digits).ok_or(Status::BadTemplate)?;
    if version != 1 && version != 2 {
        return Err(Status::BadTemplate);
    }
    Ok(TemplateValue {
        variant: None,
        version,
        spelling: Spelling::Legacy,
    })
}

/// ID §4.3's closed field table, in the table's order. A name outside it is
/// `unknown-header-field`; the fields must appear in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldName {
    /// Order 1, mandatory. "**A hint, never authority** (PB §3.1): the truth is
    /// `signed_by`. A leading `@` is retained, not stripped."
    Owner,
    /// Order 2, mandatory.
    Template,
    /// Order 3, optional. "Recorded by no node, read by no gate."
    Ticket,
    /// Order 4, mandatory.
    Constitution,
    /// Order 5, and **only** at template version 1 through the legacy spelling
    /// (ID §3.2). "Parsed and discarded."
    Status,
}

impl FieldName {
    fn from_name(name: &str) -> Option<FieldName> {
        match name {
            "Owner" => Some(FieldName::Owner),
            "Template" => Some(FieldName::Template),
            "Ticket" => Some(FieldName::Ticket),
            "Constitution" => Some(FieldName::Constitution),
            "Status" => Some(FieldName::Status),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            FieldName::Owner => "Owner",
            FieldName::Template => "Template",
            FieldName::Ticket => "Ticket",
            FieldName::Constitution => "Constitution",
            FieldName::Status => "Status",
        }
    }
}

/// The title line's two facts (ID §4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Title {
    pub id: IntentId,
    /// "recorded verbatim as the `intent` node's `title` attr … and are the
    /// subject line of the landing commit `L` after PB §5.5's `<ID>: ` prefix",
    /// which decision 6 makes G9 recompute and check.
    pub text: String,
}

/// Parse line 1 against `title-line := "# " id ": " title`, and check the id
/// against the id from the path.
///
/// DERIVED: the corpus names four statuses for this line — `bad-id`,
/// `bad-id-padding`, `id-path-mismatch`, `title-too-long` — and no fifth. A
/// title line that fails its grammar in any other way (no `# ` prefix, no
/// `": "`, an empty title, a title with a leading space) therefore reports
/// `bad-id`, the step's own catch-all. The alternative is inventing a token,
/// which ID §10 rule 6's closed-set discipline forbids more strongly than it
/// forbids a broad one.
pub fn parse_title(line: &str, path_id: &IntentId) -> Result<Title, Status> {
    let rest = line.strip_prefix("# ").ok_or(Status::BadId)?;
    let (id_text, text) = rest.split_once(": ").ok_or(Status::BadId)?;

    let id = match IntentId::parse(id_text) {
        Some(id) => id,
        None => return Err(id_refusal(id_text)),
    };
    if &id != path_id {
        return Err(Status::IdPathMismatch);
    }

    // `title := 1 … 72 bytes, containing no U+000A, with no leading or
    // trailing U+0020 or U+0009`. A trailing space is already refused by
    // §2.1 rule 9 and a U+000A cannot occur inside a line, so the reachable
    // failures are emptiness, a leading blank, and the length bound.
    if text.len() > MAX_TITLE {
        return Err(Status::TitleTooLong);
    }
    if text.is_empty() || text.starts_with([' ', '\t']) || text.ends_with([' ', '\t']) {
        return Err(Status::BadId);
    }
    Ok(Title {
        id,
        text: text.to_string(),
    })
}

/// Which of ID §3.1's two id statuses a rejected id takes.
///
/// DERIVED: §3.1 lists six non-ids — `INT-42`, `INT-0042`, `INT-000`, `INT-+42`,
/// `int-042`, `TASK-042` — and gives two tokens without mapping them. A numeral
/// that is all ASCII digits but is not the canonical left-padded spelling of
/// its value is a *padding* failure (`INT-42`, `INT-0042`, and `INT-000`, which
/// is the canonical spelling of no `n ≥ 1`); everything else — a wrong prefix,
/// a sign, a case difference, a non-digit — is `bad-id`.
fn id_refusal(id_text: &str) -> Status {
    let digits = match id_text.strip_prefix("INT-").or_else(|| id_text.strip_prefix("BUG-")) {
        Some(d) => d,
        None => return Status::BadId,
    };
    if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
        Status::BadIdPadding
    } else {
        Status::BadId
    }
}

/// The header line's parse (ID §4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub owner: String,
    pub template: TemplateValue,
    pub ticket: Option<String>,
    pub constitution: u32,
}

/// Parse line 2.
///
/// The fields are split on [`FIELD_SEPARATOR`]; ID §4.3's value grammars forbid
/// the separator inside a value, so the split is unambiguous.
///
/// Check order, DERIVED where the corpus is silent: left to right over the
/// fields, each field's structure (`bad-header-field`), then its name
/// (`unknown-header-field`), then repetition (`duplicate-header-field`), then
/// its position against the table (`header-field-order`); then `Template:`'s
/// own value, then the remaining values, then the mandatory-presence check.
/// ID §8.2 fixes the order *between* steps and, within a step, "the first
/// failure in document line order" — which on a one-line step is field order.
pub fn parse_header(line: &str) -> Result<Header, Status> {
    let mut seen: Vec<(FieldName, &str)> = Vec::new();
    let mut highest: Option<FieldName> = None;

    for field in line.split(FIELD_SEPARATOR) {
        let (name, value) = field.split_once(": ").ok_or(Status::BadHeaderField)?;
        let name = FieldName::from_name(name).ok_or(Status::UnknownHeaderField)?;
        if seen.iter().any(|(n, _)| *n == name) {
            return Err(Status::DuplicateHeaderField);
        }
        if highest.is_some_and(|h| name < h) {
            return Err(Status::HeaderFieldOrder);
        }
        highest = Some(name);
        seen.push((name, value));
    }

    let get = |want: FieldName| seen.iter().find(|(n, _)| *n == want).map(|(_, v)| *v);

    // `Template:` first, because whether `Status` is a known field at all is a
    // function of its value: ID §3.2, "a `Status` field beside a qualified
    // value is `unknown-header-field`, because a qualified value is by
    // construction a value stamped after decision 4 and no such generation ever
    // carried `Status`."
    let template = parse_template_value(get(FieldName::Template).ok_or(Status::BadHeaderField)?)?;
    if get(FieldName::Status).is_some()
        && !(template.spelling == Spelling::Legacy && template.version == 1)
    {
        return Err(Status::UnknownHeaderField);
    }

    let owner = get(FieldName::Owner).ok_or(Status::BadHeaderField)?;
    check_free_value(owner)?;
    if let Some(ticket) = get(FieldName::Ticket) {
        check_free_value(ticket)?;
    }
    if let Some(status) = get(FieldName::Status) {
        // "value is any non-empty free-text run and is parsed and discarded".
        check_free_value(status)?;
    }
    let constitution = parse_constitution(get(FieldName::Constitution).ok_or(Status::BadHeaderField)?)?;

    Ok(Header {
        owner: owner.to_string(),
        template,
        ticket: get(FieldName::Ticket).map(str::to_string),
        constitution,
    })
}

/// ID §4.3's `Owner` value grammar, which `Ticket` and v1's `Status` share:
/// "1 … 128 bytes, no U+000A, not containing `" · "`, no leading or trailing
/// space or tab".
///
/// The separator cannot occur post-split and U+000A cannot occur inside a line,
/// so this checks the two bounds that remain reachable. DERIVED: the corpus
/// names no status for a value outside its grammar; `bad-header-field` is the
/// field-level one and is used.
fn check_free_value(value: &str) -> Result<(), Status> {
    let ok = !value.is_empty()
        && value.len() <= MAX_FIELD_VALUE
        && !value.starts_with([' ', '\t'])
        && !value.ends_with([' ', '\t']);
    if ok { Ok(()) } else { Err(Status::BadHeaderField) }
}

/// ID §4.3's `Constitution` value: "`v` + a decimal integer `0 … 999`, no
/// leading zeros except `0`". It feeds the `built_under` edge to
/// `<repo>/constitution:v<n>` and G4's currency check.
fn parse_constitution(value: &str) -> Result<u32, Status> {
    value
        .strip_prefix('v')
        .and_then(parse_version)
        .ok_or(Status::BadHeaderField)
}

/// ID §4.4's `supersedes-line := "Supersedes: " id`.
///
/// "Exactly one id, and nothing after it. PB §3.1's template shows
/// `Supersedes: INT-017                        (optional)`; the parenthetical
/// is template annotation and is **not** part of the value — a document
/// carrying it is `bad-supersedes` (§12 D7)."
pub fn parse_supersedes(line: &str) -> Result<IntentId, Status> {
    let value = line.strip_prefix("Supersedes: ").ok_or(Status::BadSupersedes)?;
    IntentId::parse(value).ok_or(Status::BadSupersedes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> IntentId {
        IntentId::parse(s).unwrap()
    }

    // ---- ID §3.2, the `Template:` value ----

    #[test]
    fn a_qualified_value_splits_into_exactly_the_two_members_id_9_3_prints() {
        let t = parse_template_value("intent@2").unwrap();
        assert_eq!(t.variant, Some(Variant::Intent));
        assert_eq!(t.version, 2);
        assert_eq!(t.spelling, Spelling::Qualified);
    }

    #[test]
    fn all_three_variant_tokens_are_read_byte_exactly() {
        for (token, variant) in [
            ("intent", Variant::Intent),
            ("intent-change", Variant::IntentChange),
            ("intent-bug", Variant::IntentBug),
        ] {
            let t = parse_template_value(&format!("{token}@2")).unwrap();
            assert_eq!(t.variant, Some(variant));
            assert_eq!(variant.token(), token);
        }
    }

    /// ID §3.2: "`Intent@2` and `INTENT-CHANGE@2` are `bad-template`, not
    /// variants."
    #[test]
    fn a_miscapitalised_variant_token_is_bad_template_not_an_unknown_variant() {
        assert_eq!(parse_template_value("Intent@2"), Err(Status::BadTemplate));
        assert_eq!(
            parse_template_value("INTENT-CHANGE@2"),
            Err(Status::BadTemplate)
        );
    }

    /// TM §4.5: "any, `Template: chore@2` → `template-variant-unknown`".
    #[test]
    fn a_well_shaped_token_outside_the_three_is_template_variant_unknown() {
        assert_eq!(
            parse_template_value("chore@2"),
            Err(Status::TemplateVariantUnknown)
        );
        assert_eq!(
            parse_template_value("intent-spike@2"),
            Err(Status::TemplateVariantUnknown)
        );
    }

    #[test]
    fn a_version_with_a_leading_zero_has_no_second_spelling() {
        assert_eq!(parse_template_value("intent@02"), Err(Status::BadTemplate));
        assert_eq!(parse_template_value("intent@0").unwrap().version, 0);
        assert_eq!(parse_template_value("intent@999").unwrap().version, 999);
        assert_eq!(parse_template_value("intent@1000"), Err(Status::BadTemplate));
    }

    /// ID §3.2: the legacy form is "accepted for `n ∈ {1, 2}` only — the two
    /// generations that predate decision 4".
    #[test]
    fn the_legacy_bare_spelling_is_bounded_at_two() {
        for (value, version) in [("v1", 1u32), ("v2", 2)] {
            let t = parse_template_value(value).unwrap();
            assert_eq!(t.variant, None);
            assert_eq!(t.version, version);
            assert_eq!(t.spelling, Spelling::Legacy);
        }
        // TM §4.5: "any, bare `v3` or higher → `bad-template`".
        for value in ["v0", "v3", "v4", "v999"] {
            assert_eq!(parse_template_value(value), Err(Status::BadTemplate), "{value}");
        }
    }

    #[test]
    fn a_value_that_is_neither_shape_is_bad_template() {
        for value in ["", "2", "intent", "@2", "intent@", "V2", "intent@2@3"] {
            assert_eq!(parse_template_value(value), Err(Status::BadTemplate), "{value}");
        }
    }

    // ---- ID §3.3, prefix agreement ----

    #[test]
    fn the_prefix_agreement_table_is_id_3_3s() {
        assert!(Variant::IntentBug.agrees_with_prefix(IntentPrefix::Bug));
        assert!(!Variant::IntentBug.agrees_with_prefix(IntentPrefix::Int));
        assert!(Variant::Intent.agrees_with_prefix(IntentPrefix::Int));
        assert!(Variant::IntentChange.agrees_with_prefix(IntentPrefix::Int));
        assert!(!Variant::Intent.agrees_with_prefix(IntentPrefix::Bug));
        assert!(!Variant::IntentChange.agrees_with_prefix(IntentPrefix::Bug));
    }

    // ---- ID §4.2, the title line ----

    #[test]
    fn id_9_1s_title_line_parses_to_its_two_facts() {
        let t = parse_title(
            "# INT-042: Invoice totals include tax",
            &id("INT-042"),
        )
        .unwrap();
        assert_eq!(t.id.as_str(), "INT-042");
        assert_eq!(t.text, "Invoice totals include tax");
    }

    #[test]
    fn a_title_of_seventy_two_bytes_passes_and_seventy_three_does_not() {
        let ok = format!("# INT-042: {}", "t".repeat(MAX_TITLE));
        assert!(parse_title(&ok, &id("INT-042")).is_ok());
        let over = format!("# INT-042: {}", "t".repeat(MAX_TITLE + 1));
        assert_eq!(
            parse_title(&over, &id("INT-042")),
            Err(Status::TitleTooLong)
        );
    }

    #[test]
    fn a_title_naming_another_id_is_id_path_mismatch() {
        assert_eq!(
            parse_title("# INT-043: x", &id("INT-042")),
            Err(Status::IdPathMismatch)
        );
    }

    /// ID §3.1's six non-ids, split across the two statuses §3.1 names.
    #[test]
    fn the_six_non_ids_of_id_3_1_are_refused() {
        for (spelling, status) in [
            ("INT-42", Status::BadIdPadding),
            ("INT-0042", Status::BadIdPadding),
            ("INT-000", Status::BadIdPadding),
            ("INT-+42", Status::BadId),
            ("int-042", Status::BadId),
            ("TASK-042", Status::BadId),
        ] {
            let line = format!("# {spelling}: x");
            assert_eq!(parse_title(&line, &id("INT-042")), Err(status), "{spelling}");
        }
    }

    #[test]
    fn a_line_that_is_not_a_title_line_at_all_is_bad_id() {
        for line in ["INT-042: x", "## INT-042: x", "# INT-042 x", "#INT-042: x"] {
            assert_eq!(parse_title(line, &id("INT-042")), Err(Status::BadId), "{line}");
        }
    }

    // ---- ID §4.3, the header line ----

    #[test]
    fn id_9_1s_header_line_yields_every_field() {
        let h = parse_header(
            "Owner: @alice \u{00B7} Template: intent@2 \u{00B7} Ticket: https://tracker.example.com/T-1187 \u{00B7} Constitution: v3",
        )
        .unwrap();
        assert_eq!(h.owner, "@alice");
        assert_eq!(h.template.variant, Some(Variant::Intent));
        assert_eq!(h.template.version, 2);
        assert_eq!(h.ticket.as_deref(), Some("https://tracker.example.com/T-1187"));
        assert_eq!(h.constitution, 3);
    }

    /// ID §4.3: "A leading `@` is retained, not stripped." TM §6.1 renders the
    /// signing principal with no `@` at all, so both spellings must parse and
    /// neither may be rewritten.
    #[test]
    fn the_owner_value_is_verbatim_in_both_conventions() {
        let at = parse_header("Owner: @alice \u{00B7} Template: intent@2 \u{00B7} Constitution: v1").unwrap();
        assert_eq!(at.owner, "@alice");
        let principal =
            parse_header("Owner: alice@example.com \u{00B7} Template: intent@2 \u{00B7} Constitution: v1")
                .unwrap();
        assert_eq!(principal.owner, "alice@example.com");
    }

    #[test]
    fn a_field_out_of_the_tables_order_is_header_field_order() {
        assert_eq!(
            parse_header("Template: intent@2 \u{00B7} Owner: @a \u{00B7} Constitution: v1"),
            Err(Status::HeaderFieldOrder)
        );
        // TM §6.1: an author adding a ticket "must put it at order 3, between
        // `Template` and `Constitution`, or take `header-field-order`".
        assert_eq!(
            parse_header("Owner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: v1 \u{00B7} Ticket: t"),
            Err(Status::HeaderFieldOrder)
        );
    }

    #[test]
    fn a_repeated_field_is_duplicate_header_field() {
        assert_eq!(
            parse_header("Owner: @a \u{00B7} Owner: @b \u{00B7} Template: intent@2 \u{00B7} Constitution: v1"),
            Err(Status::DuplicateHeaderField)
        );
    }

    #[test]
    fn a_name_outside_the_table_is_unknown_header_field() {
        assert_eq!(
            parse_header("Owner: @a \u{00B7} Template: intent@2 \u{00B7} Reviewer: @b \u{00B7} Constitution: v1"),
            Err(Status::UnknownHeaderField)
        );
    }

    #[test]
    fn a_field_with_no_colon_space_is_bad_header_field() {
        assert_eq!(
            parse_header("Owner: @a \u{00B7} Template:intent@2 \u{00B7} Constitution: v1"),
            Err(Status::BadHeaderField)
        );
    }

    #[test]
    fn each_mandatory_field_is_mandatory() {
        assert_eq!(
            parse_header("Template: intent@2 \u{00B7} Constitution: v1"),
            Err(Status::BadHeaderField)
        );
        assert_eq!(
            parse_header("Owner: @a \u{00B7} Constitution: v1"),
            Err(Status::BadHeaderField)
        );
        assert_eq!(
            parse_header("Owner: @a \u{00B7} Template: intent@2"),
            Err(Status::BadHeaderField)
        );
    }

    /// ID §4.3: "A field with nothing to say is omitted, and only `Ticket` may
    /// be."
    #[test]
    fn ticket_is_the_only_optional_field() {
        let h = parse_header("Owner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: v1").unwrap();
        assert_eq!(h.ticket, None);
    }

    /// ID §3.2: "Version 1 is reachable **only** through the legacy spelling
    /// `Template: v1`: a `Status` field beside a qualified value is
    /// `unknown-header-field`."
    #[test]
    fn status_is_admitted_only_beside_a_legacy_v1_value() {
        let h = parse_header(
            "Owner: @a \u{00B7} Template: v1 \u{00B7} Constitution: v1 \u{00B7} Status: in progress",
        )
        .unwrap();
        assert_eq!(h.template.version, 1);
        assert_eq!(h.template.spelling, Spelling::Legacy);

        for template in ["v2", "intent@1", "intent@2"] {
            let line = format!(
                "Owner: @a \u{00B7} Template: {template} \u{00B7} Constitution: v1 \u{00B7} Status: in progress"
            );
            assert_eq!(
                parse_header(&line),
                Err(Status::UnknownHeaderField),
                "{template}"
            );
        }
    }

    #[test]
    fn the_constitution_value_has_one_spelling_per_number() {
        for (value, n) in [("v0", 0u32), ("v3", 3), ("v999", 999)] {
            let line = format!("Owner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: {value}");
            assert_eq!(parse_header(&line).unwrap().constitution, n);
        }
        for value in ["3", "v03", "v1000", "vx", "v"] {
            let line = format!("Owner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: {value}");
            assert_eq!(parse_header(&line), Err(Status::BadHeaderField), "{value}");
        }
    }

    #[test]
    fn an_owner_value_over_128_bytes_is_refused() {
        let ok = format!("Owner: {} \u{00B7} Template: intent@2 \u{00B7} Constitution: v1", "a".repeat(128));
        assert!(parse_header(&ok).is_ok());
        let over = format!("Owner: {} \u{00B7} Template: intent@2 \u{00B7} Constitution: v1", "a".repeat(129));
        assert_eq!(parse_header(&over), Err(Status::BadHeaderField));
    }

    // ---- ID §4.4, `Supersedes:` ----

    #[test]
    fn supersedes_carries_one_id_and_nothing_after_it() {
        assert_eq!(
            parse_supersedes("Supersedes: INT-017").unwrap().as_str(),
            "INT-017"
        );
    }

    /// ID §11.10 / §12 D7: PB §3.1's template block writes the annotation
    /// inside the transcribed value.
    #[test]
    fn the_playbooks_optional_annotation_is_not_part_of_the_value() {
        assert_eq!(
            parse_supersedes("Supersedes: INT-017                        (optional)"),
            Err(Status::BadSupersedes)
        );
    }

    #[test]
    fn supersedes_is_not_a_list() {
        assert_eq!(
            parse_supersedes("Supersedes: INT-017, INT-018"),
            Err(Status::BadSupersedes)
        );
        assert_eq!(parse_supersedes("Supersedes:"), Err(Status::BadSupersedes));
    }
}
