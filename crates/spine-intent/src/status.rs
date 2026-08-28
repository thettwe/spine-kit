//! The closed status vocabulary, its five exit classes, and the refusal a
//! parse returns.
//!
//! ID §8.2 fixes the classes and their exit codes, and fixes them *because*
//! two implementations must agree on the token, not merely on the fact of a
//! refusal: ID §1, "another branch's leases are evaluated by my binary", and
//! ID §8.3, a landed document that does not parse makes its landing
//! `unattested` — "reported and counted forever". A status token reaches a
//! reviewer's signed `wires=` and a ledger's permanent record, so each one is
//! a variant here whose `Display` is that exact byte string and nothing else.

use core::fmt;
use spine_resolve::PatternError;

/// ID §8.2's exit table. The discriminants are the process exit codes.
///
/// | Exit | Status class |
/// |---|---|
/// | 0 | `parsed` |
/// | 2 | `not-canonical` |
/// | 3 | `template-version-unknown` |
/// | 4 | `malformed` |
/// | 5 | `signoff-refused` |
///
/// There is deliberately no exit 1: ID §8.2 skips it, and a parser that
/// invented one would report a code no other implementation produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Class {
    Parsed = 0,
    NotCanonical = 2,
    TemplateVersionUnknown = 3,
    Malformed = 4,
    SignoffRefused = 5,
}

impl Class {
    /// The class's own token, which is what ID §8.2's "Status class" column
    /// names — distinct from the individual status a refusal carries.
    pub fn token(self) -> &'static str {
        match self {
            Class::Parsed => "parsed",
            Class::NotCanonical => "not-canonical",
            Class::TemplateVersionUnknown => "template-version-unknown",
            Class::Malformed => "malformed",
            Class::SignoffRefused => "signoff-refused",
        }
    }

    pub fn exit_code(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// Every status ID and TM name, in ID §8.2's step order.
///
/// The declaration order is the reporting order wherever a step checks several
/// conditions in a fixed sequence, so `Ord` on this type is usable as a
/// tie-break — but it is **not** the whole of §8.2's order, which also
/// interleaves line order. [`crate::parse`] owns that; this type owns the
/// tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    // ---- exit 2, `not-canonical`: ID §2.1's twelve rules, in table order ----
    /// Rule 1. `d` is non-empty.
    EmptyDocument,
    /// Rule 2. `len(d) ≤ 65536` (ID §2.3: "four times PB §5.5's 16 KiB
    /// envelope cap, so it never fires on a document that could land").
    DocumentTooLarge,
    /// Rule 3. Well-formed UTF-8, RFC 3629.
    NotUtf8,
    /// Rule 4. "no `U+FEFF`, at any position, byte-order mark or not".
    Bom,
    /// Rule 5.
    NulByte,
    /// Rule 6. "Not 'no CRLF' — no CR at all, lone or paired."
    CrByte,
    /// Rule 7. Every byte below `0x20` is `0x09` or `0x0A`, and `0x7F` never
    /// appears.
    ControlByte,
    /// Rule 8, first half.
    NoFinalNewline,
    /// Rule 8, second half: "no blank line at end of file".
    TrailingBlankLine,
    /// Rule 9.
    TrailingWhitespace,
    /// Rule 10. 4096 bytes.
    LineTooLong,
    /// Rule 11. A line beginning with the five bytes `-----`. ID §2.2: "Five is
    /// the count, not 'five or more': a line of six hyphens begins with five,
    /// so it is refused too."
    FenceCollision,
    /// Rule 12. A line whose first six bytes, ASCII-lowercased, are `spine-`.
    /// ID §11.12: "`git interpret-trailers` matches trailer tokens
    /// case-insensitively … The refusal exists to keep the document out of the
    /// envelope's syntax, and half a refusal does not."
    TrailerCollision,

    // ---- exit 3 ----
    /// ID §3.2: a reader that does not hold the parser for a document's
    /// `(variant, version)` pair refuses. "It never partially parses, never
    /// guesses a nearby version, never falls back to the newest it knows, and
    /// never substitutes another variant's parser for the same number."
    TemplateVersionUnknown,

    // ---- exit 4, `malformed`: step 2, the title line (ID §4.2) ----
    /// The title-line grammar failed, or its id is not ID §3.1's.
    BadId,
    /// The id's numeral is all digits but is not the canonical padding of its
    /// value — `INT-42`, `INT-0042`, `INT-000`.
    BadIdPadding,
    /// The title's id is well formed but is not the id from the path.
    IdPathMismatch,
    /// 72 bytes, hard. ID §11.11: "an unbounded field in a signed artifact is
    /// an unbounded field — and, since decision 6 makes G9 recompute the
    /// subject from these bytes, an unbounded title would be an unbounded gate
    /// input as well."
    TitleTooLong,
    /// A second line whose first two bytes are `# `. ID §4.2: "two concatenated
    /// documents must not parse as one".
    DuplicateTitle,

    // ---- step 3, the header line (ID §4.3) and `Template:` (ID §3.2) ----
    /// A field with no `": "`, a value outside its grammar, or a mandatory
    /// field absent.
    BadHeaderField,
    /// A name outside ID §4.3's closed table — including `Status` beside a
    /// qualified `Template:` value (ID §3.2).
    UnknownHeaderField,
    DuplicateHeaderField,
    HeaderFieldOrder,
    /// A `Template:` value that is neither `<variant>@<n>` nor a legacy bare
    /// `v<n>` at `n ∈ {1, 2}`.
    BadTemplate,
    /// A well-formed variant token outside the closed set of three. ID §3.2:
    /// "refused, never carried opaque".
    TemplateVariantUnknown,

    // ---- step 4, variant selection (ID §3.3) ----
    /// The id's prefix and the selected variant disagree. TM §3.3: before this
    /// check existed, "a `--bug` document carrying an `INT-` id derived to
    /// variant `intent`, parsed cleanly, and silently lost the one thing the
    /// Bug variant exists to buy".
    VariantPrefixMismatch,

    // ---- step 6, `Supersedes:` and the preamble (ID §4.4, §4.5) ----
    BadSupersedes,
    StrayPreamble,
    /// "A document with fewer than two lines is `truncated`" (ID §4.5).
    Truncated,

    // ---- step 7, section headings (ID §4.9) ----
    UnknownSection,
    DuplicateSection,
    MissingSection,
    SectionOrder,

    // ---- step 8, section bodies (ID §4.10, §5) ----
    /// A prose body with no non-empty line (ID §5.1). This is the status every
    /// scaffold refuses with (TM §6.3).
    EmptySection,
    /// A line of a class the section does not admit (ID §4.10).
    StrayText,
    /// A continuation with no preceding item (ID §4.10).
    StrayContinuation,
    /// A continuation whose stripped text begins `- ` (ID §4.10).
    IndentedItem,
    /// A continuation whose stripped text begins `AC-`. ID §4.10: without it,
    /// "an author who indents `AC-2` under `AC-1` silently ships a document
    /// with one AC … and nothing anywhere says so".
    IndentedAc,
    /// A bullet whose text is empty after `- ` (ID §4.10).
    EmptyItem,
    /// A line beginning `AC-` that does not match ID §5.3's grammar. "That
    /// clause is load-bearing: it is what stops `AC-3 the total is right` (no
    /// colon) from being silently reclassified as prose and dropped."
    MalformedAc,
    /// A non-empty line of a touchpoints body that is not a label line
    /// (ID §5.4).
    UnknownTouchpointLine,
    /// One of the two mandatory label lines is absent (ID §5.4).
    MissingTouchpointLine,
    DuplicateTouchpointLine,
    /// A pattern field that is empty after stripping — "a trailing comma, a
    /// doubled comma, a list of one comma" (ID §5.4).
    EmptyTouchpoint,
    /// ID §6.1's pattern refusals, which ID §9.5 groups as "all `bad-pattern`
    /// at exit 4 with the sub-status named". [`Status::token`] returns the
    /// sub-status, which is the byte string §9.5's table publishes.
    BadPattern(PatternError),

    // ---- step 9, shape bounds, in section ordinal order ----
    NonGoalsTooFew,
    TooManyNonGoals,
    /// TM §4.2. Minimum **1**, not 2: "an invariant is a single positive claim
    /// about what the delta may not break, and one is a real claim."
    InvariantsTooFew,
    TooManyInvariants,
    NoAcceptanceCriteria,
    TooManyAcs,
    /// "the numbers, in document order, are exactly `1, 2, …, k`" (ID §5.3).
    AcNumbering,
    NoExpectedTouchpoint,
    TooManyTouchpoints,
    /// A pattern appearing byte-identically in both polarities (ID §5.4): "it
    /// declares a path both expected and forbidden, and every landing that
    /// touched it would be a hard G2 failure."
    PolarityConflict,

    // ---- exit 5, ID §8.1's Layer 2, in that table's order ----
    WrongBranch,
    WorktreeDirty,
    OpenQuestionsNonempty,
    TemplateBelowResignFloor,
}

impl Status {
    /// The wire spelling. These bytes reach a reviewer's `wires=` and a
    /// ledger's `unattested` count, so they are fixed here and nowhere else.
    pub fn token(&self) -> &'static str {
        match self {
            Status::EmptyDocument => "empty-document",
            Status::DocumentTooLarge => "document-too-large",
            Status::NotUtf8 => "not-utf8",
            Status::Bom => "bom",
            Status::NulByte => "nul-byte",
            Status::CrByte => "cr-byte",
            Status::ControlByte => "control-byte",
            Status::NoFinalNewline => "no-final-newline",
            Status::TrailingBlankLine => "trailing-blank-line",
            Status::TrailingWhitespace => "trailing-whitespace",
            Status::LineTooLong => "line-too-long",
            Status::FenceCollision => "fence-collision",
            Status::TrailerCollision => "trailer-collision",
            Status::TemplateVersionUnknown => "template-version-unknown",
            Status::BadId => "bad-id",
            Status::BadIdPadding => "bad-id-padding",
            Status::IdPathMismatch => "id-path-mismatch",
            Status::TitleTooLong => "title-too-long",
            Status::DuplicateTitle => "duplicate-title",
            Status::BadHeaderField => "bad-header-field",
            Status::UnknownHeaderField => "unknown-header-field",
            Status::DuplicateHeaderField => "duplicate-header-field",
            Status::HeaderFieldOrder => "header-field-order",
            Status::BadTemplate => "bad-template",
            Status::TemplateVariantUnknown => "template-variant-unknown",
            Status::VariantPrefixMismatch => "variant-prefix-mismatch",
            Status::BadSupersedes => "bad-supersedes",
            Status::StrayPreamble => "stray-preamble",
            Status::Truncated => "truncated",
            Status::UnknownSection => "unknown-section",
            Status::DuplicateSection => "duplicate-section",
            Status::MissingSection => "missing-section",
            Status::SectionOrder => "section-order",
            Status::EmptySection => "empty-section",
            Status::StrayText => "stray-text",
            Status::StrayContinuation => "stray-continuation",
            Status::IndentedItem => "indented-item",
            Status::IndentedAc => "indented-ac",
            Status::EmptyItem => "empty-item",
            Status::MalformedAc => "malformed-ac",
            Status::UnknownTouchpointLine => "unknown-touchpoint-line",
            Status::MissingTouchpointLine => "missing-touchpoint-line",
            Status::DuplicateTouchpointLine => "duplicate-touchpoint-line",
            Status::EmptyTouchpoint => "empty-touchpoint",
            Status::BadPattern(e) => e.token(),
            Status::NonGoalsTooFew => "non-goals-too-few",
            Status::TooManyNonGoals => "too-many-non-goals",
            Status::InvariantsTooFew => "invariants-too-few",
            Status::TooManyInvariants => "too-many-invariants",
            Status::NoAcceptanceCriteria => "no-acceptance-criteria",
            Status::TooManyAcs => "too-many-acs",
            Status::AcNumbering => "ac-numbering",
            Status::NoExpectedTouchpoint => "no-expected-touchpoint",
            Status::TooManyTouchpoints => "too-many-touchpoints",
            Status::PolarityConflict => "polarity-conflict",
            Status::WrongBranch => "wrong-branch",
            Status::WorktreeDirty => "worktree-dirty",
            Status::OpenQuestionsNonempty => "open-questions-nonempty",
            Status::TemplateBelowResignFloor => "template-below-resign-floor",
        }
    }

    /// ID §8.2's class, and with it the exit code.
    pub fn class(&self) -> Class {
        use Status::*;
        match self {
            EmptyDocument | DocumentTooLarge | NotUtf8 | Bom | NulByte | CrByte | ControlByte
            | NoFinalNewline | TrailingBlankLine | TrailingWhitespace | LineTooLong
            | FenceCollision | TrailerCollision => Class::NotCanonical,
            TemplateVersionUnknown => Class::TemplateVersionUnknown,
            WrongBranch | WorktreeDirty | OpenQuestionsNonempty | TemplateBelowResignFloor => {
                Class::SignoffRefused
            }
            _ => Class::Malformed,
        }
    }

    /// ID §9.5 groups every §6.1 pattern refusal under one heading: "Refusal
    /// vectors, all `bad-pattern` at exit 4 with the sub-status named". This is
    /// that heading; [`Status::token`] is the sub-status.
    pub fn is_bad_pattern(&self) -> bool {
        matches!(self, Status::BadPattern(_))
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// A refusal: one status, and the 1-based line it was found on where a line
/// can be named.
///
/// The line is diagnostic only. ID §10 rule 10: "Two parsers agree iff they
/// produce §5.6's value. Everything else — retained texts, diagnostics,
/// layout — is free." Two implementations must agree on `status`; they need
/// not agree on `line`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub status: Status,
    pub line: Option<usize>,
}

impl Refusal {
    pub fn at(status: Status, line: usize) -> Refusal {
        Refusal {
            status,
            line: Some(line),
        }
    }

    /// A refusal about the document as a whole, with no line to name.
    pub fn whole(status: Status) -> Refusal {
        Refusal { status, line: None }
    }

    pub fn class(&self) -> Class {
        self.status.class()
    }

    pub fn exit_code(&self) -> u8 {
        self.status.class().exit_code()
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(n) => write!(f, "{}:{}", n, self.status),
            None => fmt::Display::fmt(&self.status, f),
        }
    }
}

impl core::error::Error for Refusal {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_exit_class_is_the_number_id_8_2_publishes() {
        assert_eq!(Class::Parsed.exit_code(), 0);
        assert_eq!(Class::NotCanonical.exit_code(), 2);
        assert_eq!(Class::TemplateVersionUnknown.exit_code(), 3);
        assert_eq!(Class::Malformed.exit_code(), 4);
        assert_eq!(Class::SignoffRefused.exit_code(), 5);
    }

    #[test]
    fn there_is_no_exit_one() {
        for class in [
            Class::Parsed,
            Class::NotCanonical,
            Class::TemplateVersionUnknown,
            Class::Malformed,
            Class::SignoffRefused,
        ] {
            assert_ne!(class.exit_code(), 1, "ID §8.2's table skips exit 1");
        }
    }

    #[test]
    fn a_bad_pattern_status_displays_its_sub_status_not_the_group() {
        let s = Status::BadPattern(PatternError::BadGlobstar);
        assert_eq!(s.token(), "bad-globstar");
        assert!(s.is_bad_pattern());
        assert_eq!(s.class(), Class::Malformed);
    }

    #[test]
    fn the_canonical_form_statuses_all_exit_two() {
        for s in [
            Status::EmptyDocument,
            Status::DocumentTooLarge,
            Status::NotUtf8,
            Status::Bom,
            Status::NulByte,
            Status::CrByte,
            Status::ControlByte,
            Status::NoFinalNewline,
            Status::TrailingBlankLine,
            Status::TrailingWhitespace,
            Status::LineTooLong,
            Status::FenceCollision,
            Status::TrailerCollision,
        ] {
            assert_eq!(s.class().exit_code(), 2, "{s}");
        }
    }

    #[test]
    fn the_layer_two_statuses_all_exit_five() {
        for s in [
            Status::WrongBranch,
            Status::WorktreeDirty,
            Status::OpenQuestionsNonempty,
            Status::TemplateBelowResignFloor,
        ] {
            assert_eq!(s.class().exit_code(), 5, "{s}");
        }
    }

    /// ID §8.1: "In particular the non-goal minimum, the AC maximum, the AC
    /// numbering, the expected-touchpoint minimum and the polarity conflict are
    /// **Layer 1**, enforced identically by `--sign`, `--approve`, `--land` and
    /// the indexer."
    #[test]
    fn the_five_shape_bounds_id_8_1_names_are_layer_one_not_signoff() {
        for s in [
            Status::NonGoalsTooFew,
            Status::TooManyAcs,
            Status::AcNumbering,
            Status::NoExpectedTouchpoint,
            Status::PolarityConflict,
        ] {
            assert_eq!(s.class(), Class::Malformed, "{s}");
        }
    }

    #[test]
    fn a_refusal_renders_its_line_then_its_token() {
        let r = Refusal::at(Status::EmptySection, 4);
        assert_eq!(r.to_string(), "4:empty-section");
        assert_eq!(
            Refusal::whole(Status::EmptyDocument).to_string(),
            "empty-document"
        );
    }
}
