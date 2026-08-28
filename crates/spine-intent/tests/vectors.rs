//! Every vector `intent-doc.md` §9 and `templates.md` §10 publish, reproduced.
//!
//! The four documents in `tests/vectors/` are the bytes between the fence
//! markers of ID §9.1, ID §9.4, TM §10.2 and TM §10.3. Nothing about them is
//! asserted that is not first *computed*: each one's blob id is recomputed here
//! from the bytes the test holds, and the whole point of the byte-count and
//! non-ASCII rows the corpus publishes is that "an implementer who transcribes
//! an em dash as a hyphen produces a different blob" (TM §6.4.4).

use spine_canon::{ObjectFormat, esc, git_blob_id, sha256_hex};
use spine_intent::status::Status;
use spine_intent::{IntentId, Variant, parse};

const ID_9_1: &[u8] = include_bytes!("vectors/id-9.1-INT-042.md");
const ID_9_4: &[u8] = include_bytes!("vectors/id-9.4-INT-001.md");
const TM_10_2: &[u8] = include_bytes!("vectors/tm-10.2-INT-043.md");
const TM_10_3: &[u8] = include_bytes!("vectors/tm-10.3-BUG-051.md");

fn id(s: &str) -> IntentId {
    IntentId::parse(s).unwrap()
}

/// The three identity rows every published document carries.
///
/// ID §9.2: the `sha256sum` row "is not a spine digest and appears in no
/// trailer. It is published so a reader who reproduces the bytes can check them
/// **without** a git repository, and then check their git installation against
/// the row above it."
fn assert_identity(bytes: &[u8], len: usize, sha1: &str, sha256_blob: &str, sha256_file: &str) {
    assert_eq!(bytes.len(), len, "byte length");
    assert_eq!(git_blob_id(bytes, ObjectFormat::Sha1), sha1);
    assert_eq!(git_blob_id(bytes, ObjectFormat::Sha256), sha256_blob);
    assert_eq!(sha256_hex(bytes), sha256_file);
}

// ---------------------------------------------------------------------------
// ID §9.1 – §9.3
// ---------------------------------------------------------------------------

/// ID §15 item 27, and ID §9.2's table.
#[test]
fn id_9_1s_bytes_are_1258_and_hash_to_the_published_ids() {
    assert_identity(
        ID_9_1,
        1258,
        "1b9e758012b85f788e3b3f16f6e81383bfdc54be",
        "1e594dc7885e7902d7e3125fc80394c53ef57aa716cf62119df0cea7be3cf39a",
        "b93064833e0e0fbf05ed39237dcab9dce1ed407b9a19373cc69749504a3b1d99",
    );
}

/// ID §9.1's non-ASCII table: "Every byte is ASCII except six characters,
/// listed so the document is reproducible without copy-paste ambiguity."
#[test]
fn id_9_1_carries_exactly_the_six_non_ascii_characters_it_enumerates() {
    let text = core::str::from_utf8(ID_9_1).unwrap();
    assert_eq!(text.chars().count(), 1249, "character count");
    assert_eq!(text.matches('\n').count(), 26, "line count");
    assert_eq!(text.matches('\u{00B7}').count(), 3, "MIDDLE DOT");
    assert_eq!(text.matches('\u{2013}').count(), 1, "EN DASH");
    assert_eq!(text.matches('\u{2014}').count(), 2, "EM DASH");
    assert_eq!(
        text.chars().filter(|c| !c.is_ascii()).count(),
        6,
        "and no others"
    );
}

/// ID §9.3, member by member.
#[test]
fn id_9_3s_parse_result_reproduces() {
    let p = parse(ID_9_1, &id("INT-042")).unwrap();
    assert_eq!(p.id.as_str(), "INT-042");
    assert_eq!(p.variant, Variant::Intent);
    assert_eq!(p.template, 2);
    assert_eq!(p.title, "Invoice totals include tax");
    assert_eq!(p.owner, "@alice");
    assert_eq!(p.ticket.as_deref(), Some("https://tracker.example.com/T-1187"));
    assert_eq!(p.constitution, 3);
    assert!(p.goal_present);
    assert_eq!(p.non_goal_count, 3);
    assert_eq!(p.acs, [1, 2, 3]);
    assert_eq!(
        p.expected.iter().map(|q| q.as_str()).collect::<Vec<_>>(),
        ["src/billing/", "api/invoices.ts"]
    );
    assert_eq!(
        p.forbidden.iter().map(|q| q.as_str()).collect::<Vec<_>>(),
        ["auth/", "shared/schema/"]
    );
    assert!(p.open_questions_empty);

    // "`supersedes` is absent, the line not being present."
    assert_eq!(p.supersedes, None);
    // TM §4.4: the three Change members are absent for this variant.
    assert_eq!(p.current_behavior_present, None);
    assert_eq!(p.target_behavior_present, None);
    assert_eq!(p.invariant_count, None);

    // "`variant` and `template` are read from the header rather than derived —
    // `intent@2` splits into exactly those two members."
    assert_eq!(p.template_attr(), "intent@2");
}

/// ID §9.3's graph elements, with `repo = myrepo`. The `code_unit` node id is
/// `dump.md` §5.2's `<repo> "/" "code:" esc(pattern bytes)`, and ID §6.1 makes
/// `esc` the identity on every legal pattern — which this asserts rather than
/// assumes.
#[test]
fn id_9_3s_node_ids_reproduce() {
    let p = parse(ID_9_1, &id("INT-042")).unwrap();
    let mut nodes = vec![format!("myrepo/{}", p.id)];
    nodes.extend(p.ac_labels().iter().map(|l| format!("myrepo/{}/{l}", p.id)));
    nodes.extend(
        p.declares()
            .iter()
            .map(|d| format!("myrepo/code:{}", esc(d.pattern.as_str().as_bytes()))),
    );
    nodes.push(format!("myrepo/constitution:v{}", p.constitution));

    assert_eq!(
        nodes,
        [
            "myrepo/INT-042",
            "myrepo/INT-042/AC-1",
            "myrepo/INT-042/AC-2",
            "myrepo/INT-042/AC-3",
            "myrepo/code:src/billing/",
            "myrepo/code:api/invoices.ts",
            "myrepo/code:auth/",
            "myrepo/code:shared/schema/",
            "myrepo/constitution:v3",
        ]
    );

    // "edges `has_ac` ×3; `declares` ×4, two with `{"polarity":"expected"}` and
    // two with `{"polarity":"forbidden"}`; `built_under` ×1."
    let declares = p.declares();
    assert_eq!(declares.len(), 4);
    assert_eq!(
        declares.iter().filter(|d| d.polarity.attr() == "expected").count(),
        2
    );
    assert_eq!(
        declares.iter().filter(|d| d.polarity.attr() == "forbidden").count(),
        2
    );
}

/// ID §6.6: provenance is the touchpoint **label line's**, "not the individual
/// pattern's, since several patterns share one line".
///
/// **DEFECT in ID §9.3.** It publishes `intents/INT-042.md:22` for the expected
/// pair and `:23` for the forbidden pair. In §9.1's own bytes — whose blob id
/// this file reproduces, so the bytes are not in question — line 22 is the
/// heading `## Touchpoints (expected blast radius)`, and the two label lines are
/// 23 and 24. TM §10.2 publishes `:29` and `:30` for the same construction and
/// **is** correct against its own bytes, which fixes the convention as 1-based
/// and makes §9.3 off by one rather than differently based.
#[test]
fn id_9_3s_published_provenance_line_numbers_are_off_by_one() {
    let text = core::str::from_utf8(ID_9_1).unwrap();
    let lines: Vec<&str> = text.trim_end_matches('\n').split('\n').collect();
    assert_eq!(lines[21], "## Touchpoints (expected blast radius)");
    assert!(lines[22].starts_with("Expected to change:"));
    assert!(lines[23].starts_with("Must NOT change:"));

    let p = parse(ID_9_1, &id("INT-042")).unwrap();
    assert_eq!((p.expected_line, p.forbidden_line), (23, 24));

    // TM §10.2's rows, which the same code reproduces exactly as published.
    let q = parse(TM_10_2, &id("INT-043")).unwrap();
    assert_eq!((q.expected_line, q.forbidden_line), (29, 30));
}

// ---------------------------------------------------------------------------
// ID §9.4 — the minimal document
// ---------------------------------------------------------------------------

/// ID §15 item 28.
#[test]
fn id_9_4s_bytes_are_415_and_hash_to_the_published_ids() {
    assert_identity(
        ID_9_4,
        415,
        "59deb4027988c87c4423ced5a4eb74550b74a218",
        "bbab2c9ff6a30140eaa90faf910cedf473f2a0b0662497d2509447024eccde69",
        "66802409b97a1d0bff2d5aa43e19284f016d2a90089a7c91e781a27cdf45acd0",
    );
}

/// "The smallest thing this grammar accepts … It exercises an omitted `Ticket`,
/// an absent Open questions section, exactly two non-goals, exactly one AC with
/// no continuation, headings with no parenthetical, and an empty forbidden list
/// written as a bare label line."
#[test]
fn id_9_4s_parse_result_reproduces() {
    let p = parse(ID_9_4, &id("INT-001")).unwrap();
    assert_eq!(p.title, "Add a health endpoint");
    assert_eq!(p.ticket, None);
    assert_eq!(p.supersedes, None);
    assert_eq!(p.non_goal_count, 2);
    assert_eq!(p.acs, [1]);
    assert_eq!(
        p.expected.iter().map(|q| q.as_str()).collect::<Vec<_>>(),
        ["src/http/"]
    );
    assert!(p.forbidden.is_empty(), "`forbidden` is `[]`");
    // "`open_questions_empty` `true` (the section is absent)."
    assert!(p.open_questions_empty);
    assert_eq!(p.constitution, 1);

    // "Its last line is `Must NOT change:` and its last byte is one `0x0A`."
    let text = core::str::from_utf8(ID_9_4).unwrap();
    assert!(text.ends_with("Must NOT change:\n"));
    assert!(!text.ends_with("\n\n"));
}

// ---------------------------------------------------------------------------
// TM §10.2 / §10.3 — the other two variants
// ---------------------------------------------------------------------------

/// TM §16 item 37.
#[test]
fn tm_10_2s_bytes_are_1502_and_hash_to_the_published_ids() {
    assert_identity(
        TM_10_2,
        1502,
        "89f6a976879cd598f2341d6d873b2c4eac808096",
        "dc2cb930a5efb00f1884f5089314adf600e7c95363f7b730d18f7e6044009bf0",
        "2c50528306b06c256bd5b5a7011f577c552e118e1d1bb9a311aed173422dab2a",
    );
    let text = core::str::from_utf8(TM_10_2).unwrap();
    assert_eq!(text.chars().count(), 1490);
    assert_eq!(text.matches('\n').count(), 32);
    // "Non-ASCII: `·` ×2, `–` ×2, `—` ×3 — all in the header line and the
    // heading parentheticals, none in a body."
    assert_eq!(text.matches('\u{00B7}').count(), 2);
    assert_eq!(text.matches('\u{2013}').count(), 2);
    assert_eq!(text.matches('\u{2014}').count(), 3);
}

/// TM §10.2's printed parse, member by member — including the reading TM §4.4
/// says is "the only reading under which the sentence means anything":
/// `goal_present` is **false** for a Change document.
#[test]
fn tm_10_2s_parse_result_reproduces() {
    let p = parse(TM_10_2, &id("INT-043")).unwrap();
    assert_eq!(p.variant, Variant::IntentChange);
    assert_eq!(p.template, 2);
    assert_eq!(p.title, "Retry failed webhook deliveries");
    assert_eq!(p.owner, "alice@example.com");
    assert_eq!(p.constitution, 3);
    assert!(!p.goal_present);
    assert_eq!(p.current_behavior_present, Some(true));
    assert_eq!(p.target_behavior_present, Some(true));
    assert_eq!(p.non_goal_count, 2);
    assert_eq!(p.invariant_count, Some(2));
    assert_eq!(p.acs, [1, 2, 3]);
    assert_eq!(
        p.expected.iter().map(|q| q.as_str()).collect::<Vec<_>>(),
        ["src/webhooks/", "api/deliveries.ts"]
    );
    assert_eq!(
        p.forbidden.iter().map(|q| q.as_str()).collect::<Vec<_>>(),
        ["auth/", "src/webhooks/signing.ts"]
    );
    assert!(p.open_questions_empty);
    assert_eq!(p.ticket, None);
    assert_eq!(p.supersedes, None);
    assert_eq!(p.template_attr(), "intent-change@2");
}

/// TM §10.2: "This document exercises ID §7.1's precedence deliberately:
/// `src/webhooks/` is expected and `src/webhooks/signing.ts` is forbidden …
/// It is not a `polarity-conflict` — ID §5.4 refuses only a **byte-identical**
/// pattern in both polarities — and under ID §7.1 a change to
/// `src/webhooks/signing.ts` is reported once, as a hard forbidden hit, and not
/// also as a containment miss."
#[test]
fn tm_10_2_exercises_the_forbidden_precedence_and_is_not_a_conflict() {
    let p = parse(TM_10_2, &id("INT-043")).unwrap();
    let delta: Vec<&[u8]> = vec![b"src/webhooks/signing.ts", b"src/webhooks/retry.ts"];
    let v = spine_intent::gates::g2(&p, &delta, &[]);
    assert_eq!(v.forbidden_hits, vec![b"src/webhooks/signing.ts".to_vec()]);
    assert!(v.outside.is_empty());
}

#[test]
fn tm_10_3s_bytes_are_1096_and_hash_to_the_published_ids() {
    assert_identity(
        TM_10_3,
        1096,
        "213288695f3037c75b94229a7ee21ae5f4c940b3",
        "5f59718dbd881dee8ac93e4472236ca0d0a1a2b1738614561139517910643879",
        "d7d25fe63465ae63ce41789fbf21cc3aa3ab3dcf01b883b5aed6ad56c5319293",
    );
    let text = core::str::from_utf8(TM_10_3).unwrap();
    assert_eq!(text.chars().count(), 1086);
    assert_eq!(text.matches('\n').count(), 24);
    assert_eq!(text.matches('\u{00B7}').count(), 2);
    assert_eq!(text.matches('\u{2013}').count(), 1);
    assert_eq!(text.matches('\u{2014}').count(), 3);
}

#[test]
fn tm_10_3s_parse_result_reproduces_and_its_reproduction_ac_is_ac_1() {
    let p = parse(TM_10_3, &id("BUG-051")).unwrap();
    assert_eq!(p.variant, Variant::IntentBug);
    assert_eq!(p.title, "Zero-rated lines are taxed at the default rate");
    assert_eq!(p.owner, "bob@example.com");
    assert_eq!(p.constitution, 3);
    assert!(p.goal_present);
    assert_eq!(p.non_goal_count, 2);
    assert_eq!(p.acs, [1, 2]);
    assert_eq!(
        p.expected.iter().map(|q| q.as_str()).collect::<Vec<_>>(),
        ["src/billing/tax.ts"]
    );
    assert_eq!(
        p.forbidden.iter().map(|q| q.as_str()).collect::<Vec<_>>(),
        ["auth/", "shared/schema/"]
    );
    assert!(p.open_questions_empty);
    // TM §5.3: "the reproduction AC is the AC numbered 1. Nothing marks it; its
    // position is its identity."
    assert_eq!(p.reproduction_ac(), Some(1));
}

/// TM §5.3's closing argument, run as a test: the same document under an `INT-`
/// id "gets no such refusal, and an implementation that applies the clause by
/// content rather than by prefix is non-conforming" (TM §16 item 34).
#[test]
fn the_bug_clause_keys_off_the_variant_and_never_off_the_content() {
    let bug = parse(TM_10_3, &id("BUG-051")).unwrap();
    assert_eq!(bug.reproduction_ac(), Some(1));

    // The same bytes with the id and the variant token both moved to `INT-`.
    let feature = core::str::from_utf8(TM_10_3)
        .unwrap()
        .replacen("BUG-051", "INT-051", 1)
        .replacen("Template: intent-bug@2", "Template: intent@2", 1);
    let p = parse(feature.as_bytes(), &id("INT-051")).unwrap();
    assert_eq!(p.variant, Variant::Intent);
    assert_eq!(p.reproduction_ac(), None);

    // And moving only one of the two is the refusal TM §3.3 says used to have
    // no detector at all.
    let half = core::str::from_utf8(TM_10_3)
        .unwrap()
        .replacen("BUG-051", "INT-051", 1);
    assert_eq!(
        parse(half.as_bytes(), &id("INT-051")).unwrap_err().status,
        Status::VariantPrefixMismatch
    );
}

// ---------------------------------------------------------------------------
// ID §9.5 — matching, through the parser
// ---------------------------------------------------------------------------

/// Build a document declaring exactly one expected pattern.
fn declaring(pattern: &str) -> String {
    format!(
        "# INT-042: t\nOwner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: v1\n\n\
         ## Goal\ng\n\n## Non-goals\n- a\n- b\n\n## Acceptance criteria\nAC-1: x\n\n\
         ## Touchpoints\nExpected to change: {pattern}\nMust NOT change:\n"
    )
}

/// ID §9.5's matching table, every row, evaluated through the pattern the
/// **parser** produced rather than one built beside it.
///
/// The dialect itself is `spine_resolve::glob`'s and is already green there
/// (IR §2.4 adopts ID §6.1–§6.3 "by reference and unaltered"); what this pins
/// is that the touchpoint splitter hands it the right bytes.
#[test]
fn id_9_5s_matching_table_reproduces_through_the_parse() {
    for (pattern, path, expected) in [
        // "The first row is the one the audit named."
        ("src/bill", "src/billing/x.ts", false),
        ("src/bill", "src/bill", true),
        ("src/billing", "src/billing/x.ts", true),
        ("src/billing", "src/billing", true),
        ("src/billing", "src/billingx/y.ts", false),
        ("src/billing/", "src/billing/x.ts", true),
        ("src/billing/", "src/billing/a/b.ts", true),
        ("src/billing/", "src/billing", false),
        ("src/billing/", "src/billingx/y.ts", false),
        ("api/invoices.ts", "api/invoices.ts", true),
        ("api/invoices.ts", "api/invoices.tsx", false),
        ("api/invoices.ts", "api/invoices.ts/x", true),
        ("src/*", "src/a.ts", true),
        ("src/*", "src/a/b.ts", true),
        ("src/*", "src", false),
        ("src/**", "src/a/b.ts", true),
        ("src/**", "src", true),
        ("**", "anything/at/all", true),
        ("**/util.ts", "util.ts", true),
        ("**/util.ts", "src/shared/util.ts", true),
        ("**/util.ts", "src/shared/xutil.ts", false),
        ("a/**/b", "a/b", true),
        ("a/**/b", "a/x/y/b", true),
        ("a/**/b", "a/x/y/bc", false),
        ("src/**/__tests__/", "src/a/__tests__/t.ts", true),
        ("src/?.ts", "src/a.ts", true),
        ("src/?.ts", "src/ab.ts", false),
        ("src/[abc]*.ts", "src/b1.ts", true),
        ("src/[abc]*.ts", "src/d1.ts", false),
        ("src/[!abc]*.ts", "src/d1.ts", true),
        ("auth/", "auth", false),
        ("auth/", "authz/x.ts", false),
    ] {
        let doc = declaring(pattern);
        let p = parse(doc.as_bytes(), &id("INT-042"))
            .unwrap_or_else(|e| panic!("{pattern} should parse, got {e}"));
        assert_eq!(p.expected.len(), 1);
        assert_eq!(p.expected[0].as_str(), pattern);
        assert_eq!(
            p.expected[0].matches_str(path),
            expected,
            "match({pattern}, {path})"
        );
    }
}

/// ID §9.5's second list: "Accepted, to pin the boundary".
#[test]
fn id_9_5s_accepted_patterns_all_parse() {
    for pattern in [
        "a*b*c",
        "sr*c/**",
        "**",
        "src/*/x",
        "src/[!abc]*.ts",
        "src/[]]x",
        "docs/",
        "a/**/b",
    ] {
        let doc = declaring(pattern);
        let p = parse(doc.as_bytes(), &id("INT-042"))
            .unwrap_or_else(|e| panic!("{pattern} should parse, got {e}"));
        assert_eq!(p.expected[0].as_str(), pattern);
    }
}

/// ID §9.5's refusal table: "all `bad-pattern` at exit 4 with the sub-status
/// named".
#[test]
fn id_9_5s_refusal_table_reproduces_through_the_parse() {
    for (pattern, sub_status) in [
        ("src/**.ts", "bad-globstar"),
        ("a**b", "bad-globstar"),
        ("!src/", "bad-negation"),
        ("/src/", "leading-slash"),
        ("src//a", "empty-segment"),
        ("src/./a", "dot-segment"),
        ("src/../a", "dot-segment"),
        ("src/[abc", "bad-bracket"),
        ("x[:alpha:]y", "bad-bracket"),
        ("x[[:alpha:]]y", "bad-bracket"),
        ("a b", "pattern-illegal-byte"),
        ("a\"b", "pattern-illegal-byte"),
        ("a\\b", "pattern-illegal-byte"),
        ("é/x", "pattern-illegal-byte"),
    ] {
        let doc = declaring(pattern);
        let e = parse(doc.as_bytes(), &id("INT-042")).expect_err(pattern);
        assert!(e.status.is_bad_pattern(), "{pattern}");
        assert_eq!(e.status.token(), sub_status, "{pattern}");
        assert_eq!(e.exit_code(), 4, "{pattern}");
    }
}

/// The two rows of ID §9.5's refusal table that a **touchpoint line** cannot
/// reach, and why — which is itself a rule (ID §5.4): "This split is unambiguous
/// because §6.1 forbids `,` and space inside a pattern."
///
/// So `a,b` is two legal patterns on a touchpoint line, and the empty pattern is
/// `empty-touchpoint` — the section's own status — rather than `pattern-empty`.
/// Both statuses still exist and still fire, from the dialect's own entry point.
#[test]
fn the_comma_and_the_empty_field_are_the_lists_business_not_the_dialects() {
    let doc = declaring("a,b");
    let p = parse(doc.as_bytes(), &id("INT-042")).unwrap();
    assert_eq!(
        p.expected.iter().map(|q| q.as_str()).collect::<Vec<_>>(),
        ["a", "b"]
    );

    let doc = declaring("a,,b");
    assert_eq!(
        parse(doc.as_bytes(), &id("INT-042")).unwrap_err().status,
        Status::EmptyTouchpoint
    );

    // The dialect's own two statuses, from the dialect's own entry point.
    use spine_intent::Pattern;
    assert_eq!(Pattern::parse("").unwrap_err().token(), "pattern-empty");
    assert_eq!(
        Pattern::parse("a,b").unwrap_err().token(),
        "pattern-illegal-byte"
    );
    assert_eq!(
        Pattern::parse(&"a".repeat(256)).unwrap_err().token(),
        "pattern-too-long"
    );
}

// ---------------------------------------------------------------------------
// TM §6.3 — a scaffold does not parse, and that is the design
// ---------------------------------------------------------------------------

/// TM §6.3's table, over the renderer's own bytes.
///
/// > Run ID §8.2's order over any scaffold of §6.4 and the answer is the same
/// > for all three: canonical form passes, the title passes, the header passes,
/// > the template version is known, variant selection succeeds, the preamble
/// > passes, every section heading is present, in order, known and unique — and
/// > then step 8 reaches the first mandatory body, which is empty.
///
/// This is the strongest available check of both crates at once: it asserts the
/// renderer's bytes reach step 8 of the parser's order, which every earlier step
/// passing is a precondition of.
#[test]
fn tm_6_3s_first_refusal_column_reproduces() {
    use spine_intent::sections::section_key;
    use spine_template::scaffold::{self, Instance};

    for (variant, doc_id, owner, expected_key) in [
        (scaffold::Variant::Intent, "INT-042", "alice@example.com", "goal"),
        (
            scaffold::Variant::IntentChange,
            "INT-043",
            "alice@example.com",
            "current behavior",
        ),
        (scaffold::Variant::IntentBug, "BUG-051", "bob@example.com", "goal"),
    ] {
        let rendered = scaffold::render(
            variant,
            &Instance {
                id: doc_id,
                owner,
                template_version: 2,
                constitution_version: 3,
            },
        )
        .unwrap();

        let e = parse(rendered.as_bytes(), &id(doc_id)).expect_err(variant.name());
        assert_eq!(e.status, Status::EmptySection, "{variant}");
        assert_eq!(e.exit_code(), 4, "{variant}");

        // The refusal is at the variant's *first mandatory body*, which the
        // status alone does not say — so the line is checked against the
        // heading TM §6.3's table names.
        let line = e.line.expect("empty-section names its heading");
        let heading = rendered.split('\n').nth(line - 1).unwrap();
        assert_eq!(
            section_key(heading).as_deref(),
            Some(expected_key),
            "{variant} refuses at the wrong section"
        );
    }
}

/// TM §16 item 22: "Filling every mandatory body of a scaffold, with no other
/// edit, yields a document that parses."
///
/// The fills are the minima and nothing more — one prose line, two non-goals,
/// one AC, one expected pattern — so what is being tested is that a scaffold's
/// headings, order and structural lines are the ones the parser wants.
#[test]
fn tm_16_22_filling_only_the_mandatory_bodies_yields_a_parsing_document() {
    use spine_template::scaffold::{self, Instance};

    for (variant, doc_id, owner, parsed_variant) in [
        (scaffold::Variant::Intent, "INT-042", "alice@example.com", Variant::Intent),
        (
            scaffold::Variant::IntentChange,
            "INT-043",
            "alice@example.com",
            Variant::IntentChange,
        ),
        (
            scaffold::Variant::IntentBug,
            "BUG-051",
            "bob@example.com",
            Variant::IntentBug,
        ),
    ] {
        let rendered = scaffold::render(
            variant,
            &Instance {
                id: doc_id,
                owner,
                template_version: 2,
                constitution_version: 3,
            },
        )
        .unwrap();

        let mut out = String::new();
        for line in rendered.trim_end_matches('\n').split('\n') {
            out.push_str(line);
            out.push('\n');
            let fill = match spine_intent::sections::section_key(line).as_deref() {
                Some("goal") | Some("current behavior") | Some("target behavior") => "one sentence.",
                Some("non-goals") => "- first\n- second",
                Some("invariants") => "- stays true",
                Some("acceptance criteria") => "AC-1: Given a, when b, then c.",
                _ => continue,
            };
            out.push_str(fill);
            out.push('\n');
        }
        // The touchpoints body's two label lines are already there; only the
        // expected list is missing, and an empty one is `no-expected-touchpoint`
        // — "which names what to add" (TM §6.2).
        let out = out.replace("Expected to change:\n", "Expected to change: src/\n");

        let p = parse(out.as_bytes(), &id(doc_id))
            .unwrap_or_else(|e| panic!("{variant} filled must parse, got {e}"));
        assert_eq!(p.variant, parsed_variant);
        assert_eq!(p.non_goal_count, 2);
        assert_eq!(p.acs, [1]);
        assert_eq!(p.expected[0].as_str(), "src/");
        assert!(p.open_questions_empty, "the scaffolded body must be empty");
    }
}

/// TM §6.2's normative constraint on the scaffold, checked from the parse side:
/// "A scaffold that seeds a prose line here makes every freshly created intent
/// unsignable."
#[test]
fn tm_6_2_the_scaffolded_open_questions_body_is_empty() {
    use spine_template::scaffold::{self, Instance};

    for (variant, doc_id, owner) in [
        (scaffold::Variant::Intent, "INT-042", "alice@example.com"),
        (scaffold::Variant::IntentChange, "INT-043", "alice@example.com"),
        (scaffold::Variant::IntentBug, "BUG-051", "bob@example.com"),
    ] {
        let rendered = scaffold::render(
            variant,
            &Instance {
                id: doc_id,
                owner,
                template_version: 2,
                constitution_version: 3,
            },
        )
        .unwrap();
        assert!(
            rendered.ends_with("## Open questions (optional — must be empty before implementation)\n"),
            "{variant}: the last heading must be Open questions with no body"
        );
    }
}
