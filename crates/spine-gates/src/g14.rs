//! **G14 — Authority · Floor.** `docs/spec/manifest.md` §5, whole.
//!
//! > the `merge-base..head` diff — renames, deletions, mode changes, symlinks
//! > (`120000`), submodule pointers (`160000`) included, paths casefolded —
//! > ∩ (shipped floor ∪ `C-A2`) = ∅, **or** a `Spine-Review class=protected`
//! > verifies … Declared touchpoints are not consulted. (PB §6.3)
//!
//! The last clause is load-bearing and is why no touchpoint set appears in this
//! module's inputs: PB §5.2, §7.3 and §6.3 all say it, and MF §5.10 spells the
//! consequence — "an intent declaring `.github/workflows/` as expected has
//! declared nothing."

use crate::gate::Gate;
use crate::review::Reviews;
use crate::status::G14Status;
use crate::verdict::{Finding, Verdict, decide};
use crate::wire::{Wire, WireClass, WireKind};
use spine_canon::esc;
use spine_resolve::glob::{Pattern, PatternError};
use std::collections::BTreeMap;

/// MF §5.2, verbatim:
///
/// ```text
/// cf(s)[i] = s[i] + 0x20   if 0x41 ≤ s[i] ≤ 0x5A
///          = s[i]          otherwise
/// ```
///
/// "**ASCII only. Over raw path bytes. Length-preserving. Total on every byte
/// string, valid UTF-8 or not.** No Unicode table, no locale, no normalization,
/// no allocation that can fail."
///
/// Not `str::to_lowercase`, and not `to_ascii_lowercase` on a `str`: MF §5.2's
/// first reason is decisive — "A Unicode fold is versioned. `İ`, `ẞ` and the
/// Cherokee syllabary all changed fold behaviour between Unicode releases. Two
/// implementations built against two ICU versions would disagree on
/// `floor_hits` over identical git objects."
///
/// Because it touches only `0x41..=0x5A`, which never occur inside a multi-byte
/// UTF-8 sequence, `cf` preserves UTF-8 validity — which is what lets
/// [`FloorPattern`] fold a pattern and re-parse it.
pub fn cf(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .map(|&b| if b.is_ascii_uppercase() { b + 0x20 } else { b })
        .collect()
}

/// A pattern in `F0` or `effective(C-A2)`, folded once at construction.
///
/// MF §5.6: `pmatch(P, p) := match( cf(P), cf(p) )`, where `match` is
/// "**ID §6.3's, adopted verbatim** — segment-boundary, `**` crosses
/// separators, `*` does not, trailing `/` means contents-only."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloorPattern {
    source: String,
    folded: Pattern,
}

/// Why a floor pattern was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloorPatternError {
    /// MF §5.6's one exclusion: "`cf` maps `A–Z` to `a–z` and touches nothing
    /// else, so it is the identity on `/`, `*`, `?`, `[`, `]`, `!` and `-`. It
    /// is *not* safe inside a bracket expression: `cf(\"[A-Z]\")` is `[a-z]`, a
    /// different set. Therefore: **A floor pattern containing an ASCII
    /// uppercase letter inside a bracket expression is refused.**"
    BracketCase,
    /// ID §6.1's own refusals, unchanged: "This document defines no dialect and
    /// changes none of ID §6.1's refusals" (MF §5.6).
    Dialect(PatternError),
}

impl FloorPattern {
    pub fn new(source: &str) -> Result<Self, FloorPatternError> {
        if has_uppercase_in_bracket(source.as_bytes()) {
            return Err(FloorPatternError::BracketCase);
        }
        // `cf` is length-preserving over ASCII and preserves UTF-8 validity, so
        // this cannot fail; the pattern dialect refuses every non-ASCII byte
        // anyway (ID §6.1's `legal_byte`).
        let folded = String::from_utf8(cf(source.as_bytes()))
            .expect("cf preserves UTF-8 validity: it rewrites 0x41..=0x5A only");
        Pattern::parse(&folded)
            .map(|folded| FloorPattern {
                source: source.to_string(),
                folded,
            })
            .map_err(FloorPatternError::Dialect)
    }

    /// The pattern as written, unfolded. `esc` and `tok` are the identity on it
    /// (ID §6.1), which is why GR §5.4 can record it in `floor_extensions`
    /// without a second encoding.
    pub fn as_str(&self) -> &str {
        &self.source
    }

    /// `pmatch(P, p)`.
    pub fn matches(&self, path: &[u8]) -> bool {
        self.folded.matches(&cf(path))
    }
}

/// Scan for an ASCII uppercase letter inside a bracket expression.
///
/// The bracket grammar is ID §6.2's, mirrored here rather than borrowed
/// because `spine-resolve` does not expose its scanner: `[` opens, an optional
/// `!` negates, a `]` immediately after `[` or `[!` is a literal member, and
/// the first later `]` closes. An unterminated `[` is `bad-bracket` there, so
/// the bytes after it are treated as inside a bracket here — the fail-closed
/// direction, and the pattern is refused either way.
fn has_uppercase_in_bracket(bytes: &[u8]) -> bool {
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        i += 1;
        if bytes.get(i) == Some(&b'!') {
            i += 1;
        }
        // "A `]` immediately after `[` or `[!` is a literal member" (ID §6.2).
        let mut first = true;
        while i < bytes.len() {
            let b = bytes[i];
            if b == b']' && !first {
                break;
            }
            first = false;
            if b.is_ascii_uppercase() {
                return true;
            }
            i += 1;
        }
        i += 1;
    }
    false
}

/// MF §5.5's closed list of seventeen patterns — "the v1 release constant …
/// derived from PB §7.3's four bullets with no addition".
///
/// Two depth rules, both stated in MF §5.5 because "PB's own clause covers only
/// three of the five categories":
///
/// - a floor entry named by a **file or directory name** matches at any depth,
///   "because **over-inclusion costs a protected review while under-inclusion
///   costs the boundary**. That asymmetry is the tie-breaker for every
///   ambiguity in this list";
/// - a floor entry named by a **provider directory prefix** stays root-anchored,
///   because `sub/.github/workflows/x.yml` executes nothing. `Jenkinsfile*` is
///   root-anchored for the same reason and "is the weakest entry in the list".
///
/// Symlinks and submodules are absent "because they are not paths"; that is
/// [`mode_hit`].
pub const F0_PATTERNS: [&str; 17] = [
    ".spine/",
    ".github/workflows/",
    ".github/actions/",
    ".circleci/",
    ".buildkite/",
    "Jenkinsfile*",
    "**/.gitlab-ci.yml",
    "**/AGENTS.md",
    "**/CLAUDE.md",
    "**/.claude/",
    "**/.cursor/",
    "**/CODEOWNERS",
    "**/.gitattributes",
    "**/.gitmodules",
    "**/.githooks/",
    "**/.husky/",
    "**/.pre-commit-config.yaml",
];

/// `F0`, compiled.
///
/// "For `F0` this is a release-build assertion (no entry has a bracket)"
/// (MF §5.6) — so a failure here is a defect in this binary, not in a
/// repository, and it panics rather than producing a status token no
/// repository could act on.
pub fn f0() -> Vec<FloorPattern> {
    F0_PATTERNS
        .iter()
        .map(|p| FloorPattern::new(p).expect("F0 is a release constant and must compile"))
        .collect()
}

/// One record of `git -c core.quotePath=false diff --raw -z --no-renames <mb>
/// <Hc>`, reduced to the triple MF §5.3 keeps.
///
/// "`D` is the set of triples `(src_mode, dst_mode, path)`, with `path` the raw
/// bytes." The oids and the status letter are read by nothing here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEntry {
    /// **The six digits git prints, read as a decimal number** — `100644`
    /// stays `100644`, not `0o100644`. MF §5.8's clause is a comparison against
    /// two literal spellings (`120000`, `160000`) and never an arithmetic
    /// operation on a file mode, so re-basing them would buy nothing and cost a
    /// conversion an implementation could get wrong in one direction only.
    /// [`DiffEntry::from_raw`] is the parse that keeps callers on this reading.
    pub src_mode: u32,
    pub dst_mode: u32,
    pub path: Vec<u8>,
}

impl DiffEntry {
    pub fn new(src_mode: u32, dst_mode: u32, path: impl Into<Vec<u8>>) -> Self {
        DiffEntry {
            src_mode,
            dst_mode,
            path: path.into(),
        }
    }

    /// From the two mode fields of a `--raw` record, as printed. The leading
    /// `:` of the first field is stripped if present.
    pub fn from_raw(src_mode: &str, dst_mode: &str, path: impl Into<Vec<u8>>) -> Option<Self> {
        let src = src_mode.strip_prefix(':').unwrap_or(src_mode);
        Some(DiffEntry::new(
            src.parse().ok()?,
            dst_mode.parse().ok()?,
            path,
        ))
    }
}

/// MF §5.8, verbatim: `modehit(sm, dm) := sm ∈ {120000, 160000} ∨ dm ∈ {120000,
/// 160000}`.
///
/// "**Both sides**, so a symlink *deleted* or *replaced by a regular file* is a
/// hit as well as one added. PB says 'adds or changes'; a deletion is the third
/// way the same mechanism moves, and the asymmetry would be a hole with no
/// argument behind it. The path itself is irrelevant — this is a hit wherever
/// it lands."
pub fn mode_hit(src_mode: u32, dst_mode: u32) -> bool {
    const SYMLINK: u32 = 120_000;
    const SUBMODULE: u32 = 160_000;
    matches!(src_mode, SYMLINK | SUBMODULE) || matches!(dst_mode, SYMLINK | SUBMODULE)
}

/// MF §5.6: `lmatch(v, p) := cf(p) = cf(v) ∨ cf(p) begins with cf(v) ++ "/"`.
///
/// A literal, never a pattern: "A `paths` value is *a repository path*, not a
/// pattern, and a real path may contain `*`, `?` or `[`. Treating
/// `docs/notes[draft].md` as a pattern would make it protect a set of paths
/// that does not include itself."
pub fn literal_match(entry: &[u8], path: &[u8]) -> bool {
    let v = cf(entry);
    let p = cf(path);
    if p == v {
        return true;
    }
    // "`lmatch`'s second clause is the directory case."
    p.len() > v.len() && p.starts_with(&v) && p[v.len()] == b'/'
}

/// Everything G14 reads. MF §5.1: "**Everything but `M_T` is read from the base
/// side.** Policy from trunk (PB §7.4 rule 1)."
#[derive(Debug, Clone, Default)]
pub struct G14Input {
    /// `D` — empty for a tombstone (MF §5.3: "A tombstone's `D` is empty by
    /// construction, and that is what makes `G14=pass` honest").
    pub diff: Vec<DiffEntry>,
    /// `paths(T)` — `git ls-tree -r -z --name-only <T>`, raw bytes (MF §5.7).
    pub tree_paths: Vec<Vec<u8>>,
    /// `effective(C-A2)` at `B`, as written.
    pub c_a2_at_b: Vec<String>,
    /// The `C-A2` pattern set in `T`, for MF §5.9's outright 2. Compared **by
    /// bytes** (CN §6.5), so these are the sources and not the compiled forms.
    pub c_a2_at_t: Vec<String>,
    /// `E(M_B)` — MF §3.4's flattened `paths.*` value set at `B`. `∅` when the
    /// landing carries `Spine-Upgrade: from=none` (MF §5.4).
    pub e_m_b: Vec<Vec<u8>>,
    /// `E(M_T)`, for outright 1.
    pub e_m_t: Vec<Vec<u8>>,
    /// Whether the landing carries a verifying `Spine-Upgrade: to=none`. G14's
    /// one exception: "an uninstall removes the manifest, so every entry is
    /// dropped, and the design's answer is that leaving costs what arriving
    /// cost, under a review" (MF §5.9).
    pub carries_to_none: bool,
}

/// G14's output: the verdict, plus `floor_hits`, which GR §5.7 makes "the
/// authoritative list; the `G14` wires are derived from it".
#[derive(Debug, Clone)]
pub struct G14Outcome {
    pub verdict: Verdict<G14Status>,
    /// `esc(d)` for every `d ∈ hits`, deduplicated, sorted ascending by the
    /// `esc`-encoded bytes (MF §5.10, GR §5.7).
    pub floor_hits: Vec<String>,
}

/// MF §5.10's verdict expression, MF §5.11's assembly.
///
/// **On ordering.** MF §5.11's pseudocode returns `FAIL_OUTRIGHT` before it
/// builds `wires`. This implementation computes the hits, the `floor_hits` and
/// the wires **first** and reports the outright findings alongside them.
/// DERIVED, and it is the fail-closed direction: GR §5.6.1 makes a report
/// carrying a `fail` "the report a reviewer reads and binds with `report=` on
/// their `Spine-Review`", so it is emitted, and GR §5.7's invariant — "for each
/// entry `p`, `wires` contains exactly one `{gate: \"G14\", path: p, …}`" — has
/// to hold of it. Dropping the wires would also hide every floor hit from the
/// human reading the refusal, which is the opposite of what an outright status
/// is for. The status is `fail` either way.
pub fn evaluate(input: &G14Input, reviews: &Reviews) -> G14Outcome {
    let mut findings: Vec<Finding<G14Status>> = Vec::new();

    // `F_pat := F0 ∪ effective(C-A2_B)` (MF §5.11). The bracket refusal is
    // checked over the C-A2 half only: `F0`'s is a release-build assertion,
    // discharged by `f0()`'s expect and by this module's own test.
    let mut patterns = f0();
    for source in &input.c_a2_at_b {
        match FloorPattern::new(source) {
            Ok(pattern) => patterns.push(pattern),
            Err(FloorPatternError::BracketCase) => {
                findings.push(Finding::outright(G14Status::CA2BracketCase));
            }
            // ID §6.1's refusals are CN's to raise, not G14's — a pattern the
            // dialect rejects never became `effective(C-A2)` at all (CN §6.2).
            // Fail closed: it matches nothing here, and the landing is refused
            // by the gate that owns the parse.
            Err(FloorPatternError::Dialect(_)) => {}
        }
    }

    // The collision clause's index, built once over `paths(T)` rather than
    // per diff entry: `collides(d) := ∃ x ∈ paths(T) : x ≠ d ∧ cf(x) = cf(d)`.
    let mut fold: BTreeMap<Vec<u8>, Vec<&[u8]>> = BTreeMap::new();
    for path in &input.tree_paths {
        fold.entry(cf(path)).or_default().push(path);
    }

    let mut hits: Vec<Vec<u8>> = Vec::new();
    for entry in &input.diff {
        let d = entry.path.as_slice();
        let hit = mode_hit(entry.src_mode, entry.dst_mode)
            || patterns.iter().any(|p| p.matches(d))
            || input.e_m_b.iter().any(|v| literal_match(v, d))
            || fold
                .get(&cf(d))
                .is_some_and(|xs| xs.iter().any(|x| *x != d));
        if hit {
            hits.push(entry.path.clone());
        }
    }

    // "`floor_hits` := `esc(d)` for every `d ∈ hits`, deduplicated, **sorted
    // ascending by the `esc`-encoded bytes**" (MF §5.10, GR §5.7). The sort is
    // over `esc`, and the wires' sort is over `tok` — R2's four encodings of
    // one path, two of which meet in this function.
    let mut floor_hits: Vec<String> = hits.iter().map(|d| esc(d)).collect();
    floor_hits.sort_unstable();
    floor_hits.dedup();

    // One wire per hit "and no other `G14` entry" (MF §5.10). The `WireSet`'s
    // own `(gate, path)` key deduplicates, so a diff that somehow presented one
    // path twice still yields one entry — the invariant GR §5.7 states.
    for d in &hits {
        findings.push(Finding::coverable(
            G14Status::FloorHit,
            Wire::at(Gate::G14, d.clone(), WireClass::Protected, WireKind::Finding),
        ));
    }

    // Outright 1 — "the `paths.*` floor never shrinks" (MF §5.9). `E` is a
    // flattened value set, so moving an entry between keys drops nothing
    // (MF §3.4).
    if !input.carries_to_none && !input.e_m_b.iter().all(|v| input.e_m_t.contains(v)) {
        findings.push(Finding::outright(G14Status::PathsShrank));
    }

    // Outright 2 — "`C-A2` never shrinks … byte-identical pattern sets,
    // `P_B ⊆ P_T`" (MF §5.9, CN §6.5).
    if !input
        .c_a2_at_b
        .iter()
        .all(|p| input.c_a2_at_t.contains(p))
    {
        findings.push(Finding::outright(G14Status::CA2Shrank));
    }

    let verdict = decide(Gate::G14, findings, reviews);
    G14Outcome {
        verdict,
        floor_hits,
    }
}

/// GR §5.7's invariant, as a predicate a caller can assert over an assembled
/// report: every `floor_hits` entry has exactly one `G14` wire and there is no
/// other `G14` wire.
pub fn floor_hits_and_wires_agree(outcome: &G14Outcome) -> bool {
    let wires = outcome.verdict.wires.of(Gate::G14);
    let mut from_wires: Vec<String> = wires.iter().filter_map(Wire::esc_path).collect();
    from_wires.sort_unstable();
    from_wires == outcome.floor_hits && wires.iter().all(|w| w.path.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::{Review, ReviewClass};
    use crate::verdict::GateStatus;

    /// MF §5.6: "For `F0` this is a release-build assertion (no entry has a
    /// bracket)." This is that assertion, run.
    #[test]
    fn every_f0_pattern_compiles_and_holds_no_bracket() {
        assert_eq!(f0().len(), 17);
        for p in F0_PATTERNS {
            assert!(!p.contains('['), "{p}");
        }
    }

    #[test]
    fn cf_is_ascii_only_length_preserving_and_total_on_invalid_utf8() {
        assert_eq!(cf(b"AGENTS.md"), b"agents.md".to_vec());
        // Latin capital I with dot above, UTF-8 `c4 b0`: untouched, because
        // neither byte is in `0x41..=0x5A`.
        assert_eq!(cf(&[0xc4, 0xb0]), vec![0xc4, 0xb0]);
        let invalid = [0xff, b'A', 0x00, 0x5A, 0x5B];
        assert_eq!(cf(&invalid), vec![0xff, b'a', 0x00, 0x7a, 0x5b]);
        assert_eq!(cf(&invalid).len(), invalid.len());
    }

    /// MF §5.2's stated residual, kept as a test so it stays stated: "`CAFÉ.py`
    /// (UTF-8 `caf\xc3\x89.py`) and `café.py` (`caf\xc3\xa9.py`) do not collide
    /// under `cf`."
    #[test]
    fn a_non_ascii_second_spelling_is_not_detected_and_that_is_recorded() {
        assert_ne!(cf(b"caf\xc3\x89.py"), cf(b"caf\xc3\xa9.py"));
    }

    #[test]
    fn an_uppercase_letter_inside_a_bracket_is_refused_and_outside_one_is_not() {
        assert!(matches!(
            FloorPattern::new("src/[A-Z]*.py"),
            Err(FloorPatternError::BracketCase)
        ));
        assert!(has_uppercase_in_bracket(b"a[!B]c"));
        // `[]A]` — the `]` right after `[` is a literal member, so the `A` is
        // still inside the bracket.
        assert!(has_uppercase_in_bracket(b"[]A]"));
        assert!(!has_uppercase_in_bracket(b"**/AGENTS.md"));
        assert!(FloorPattern::new("src/[a-z]*.py").is_ok());
    }

    #[test]
    fn pmatch_folds_both_sides() {
        let p = FloorPattern::new("**/AGENTS.md").unwrap();
        assert!(p.matches(b"AGENTS.md"));
        assert!(p.matches(b"agents.md"));
        assert!(p.matches(b"docs/Agents.MD"));
        assert!(!p.matches(b"AGENTS.mdx"));
    }

    /// MF §5.5: a provider directory prefix stays root-anchored.
    /// `sub/.github/workflows/x.yml` executes nothing.
    #[test]
    fn a_provider_prefix_is_root_anchored_and_a_name_is_not() {
        let github = FloorPattern::new(".github/workflows/").unwrap();
        assert!(github.matches(b".github/workflows/ci.yml"));
        assert!(!github.matches(b"sub/.github/workflows/ci.yml"));
        let gitlab = FloorPattern::new("**/.gitlab-ci.yml").unwrap();
        assert!(gitlab.matches(b".gitlab-ci.yml"));
        assert!(gitlab.matches(b"sub/.gitlab-ci.yml"));
    }

    #[test]
    fn lmatch_is_equality_or_the_directory_case() {
        assert!(literal_match(b"CONSTITUTION.md", b"constitution.md"));
        assert!(literal_match(b"docs", b"DOCS/a.md"));
        assert!(!literal_match(b"docs", b"docsy/a.md"));
        // A real path containing a metacharacter protects itself, which is the
        // whole reason this is not `pmatch` (MF §5.6).
        assert!(literal_match(b"docs/notes[draft].md", b"docs/notes[draft].md"));
    }

    #[test]
    fn modehit_reads_both_sides() {
        assert!(mode_hit(0, 120_000));
        assert!(mode_hit(120_000, 100_644));
        assert!(mode_hit(160_000, 0));
        assert!(!mode_hit(100_644, 100_755));
    }

    /// The `--raw` record's mode fields, as git prints them: `:120000 000000
    /// e7f7c04 0000000 D\0tools/spine\0`.
    #[test]
    fn from_raw_reads_the_printed_digits_as_written() {
        let entry = DiffEntry::from_raw(":120000", "000000", "tools/spine").unwrap();
        assert_eq!(entry.src_mode, 120_000);
        assert_eq!(entry.dst_mode, 0);
        assert!(mode_hit(entry.src_mode, entry.dst_mode));
    }

    fn mf_8_4_input() -> G14Input {
        // MF §8.4's diff, transcribed. `mb` =
        // 1cbc18507888cb238c56ce00ba678c16564e0274,
        // `Hc` = de841d39b7a84111dfbcc11ddc7a75aa9886b218.
        G14Input {
            diff: vec![
                DiffEntry::new(100_644, 0, ".github/workflows/spine-land.yml"),
                DiffEntry::new(0, 100_644, ".github/workflows/spine-land.yml.bak"),
                DiffEntry::new(100_644, 100_644, "CONSTITUTION.md"),
                DiffEntry::new(0, 100_755, "infra/deploy.sh"),
                DiffEntry::new(0, 100_644, "src/billing/Tax.py"),
                DiffEntry::new(100_644, 100_644, "src/billing/invoice.py"),
                DiffEntry::new(0, 120_000, "tools/spine"),
            ],
            // "`T` also contains `src/billing/tax.py`, untouched by this diff."
            tree_paths: [
                ".github/workflows/spine-land.yml.bak",
                "CONSTITUTION.md",
                "infra/deploy.sh",
                "src/billing/Tax.py",
                "src/billing/tax.py",
                "src/billing/invoice.py",
                "tools/spine",
            ]
            .iter()
            .map(|p| p.as_bytes().to_vec())
            .collect(),
            c_a2_at_b: vec!["infra/".into()],
            c_a2_at_t: vec!["infra/".into()],
            e_m_b: vec![b"CONSTITUTION.md".to_vec(), b"AGENTS.md".to_vec()],
            e_m_t: vec![b"CONSTITUTION.md".to_vec(), b"AGENTS.md".to_vec()],
            carries_to_none: false,
        }
    }

    /// MF §8.4, computed. Six of seven entries hit, one clause each, and the
    /// seventh — `src/billing/invoice.py` — hits nothing.
    #[test]
    fn the_mf_8_4_run_reproduces_its_six_floor_hits() {
        let outcome = evaluate(&mf_8_4_input(), &Reviews::default());
        assert_eq!(
            outcome.floor_hits,
            [
                ".github/workflows/spine-land.yml",
                ".github/workflows/spine-land.yml.bak",
                "CONSTITUTION.md",
                "infra/deploy.sh",
                "src/billing/Tax.py",
                "tools/spine",
            ]
        );
        assert_eq!(outcome.verdict.status, GateStatus::Fail);
        assert!(floor_hits_and_wires_agree(&outcome));
    }

    /// MF §8.4: "`G14 = override` iff one `class=protected` `Spine-Review` with
    /// `head=Hc` names all six tokens." `esc` and `tok` are the identity on all
    /// six, so the tokens are the paths.
    #[test]
    fn one_protected_review_naming_all_six_tokens_makes_it_override() {
        let outcome = evaluate(&mf_8_4_input(), &Reviews::default());
        let tokens = outcome.verdict.wires.tokens();
        assert_eq!(
            tokens,
            [
                "G14:.github/workflows/spine-land.yml",
                "G14:.github/workflows/spine-land.yml.bak",
                "G14:CONSTITUTION.md",
                "G14:infra/deploy.sh",
                "G14:src/billing/Tax.py",
                "G14:tools/spine",
            ]
        );
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:bob").naming(tokens),
        ]);
        let covered = evaluate(&mf_8_4_input(), &reviews);
        assert_eq!(covered.verdict.status, GateStatus::Override);
    }

    #[test]
    fn a_review_missing_one_token_leaves_the_gate_failing() {
        let outcome = evaluate(&mf_8_4_input(), &Reviews::default());
        let mut tokens = outcome.verdict.wires.tokens();
        tokens.pop();
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:bob").naming(tokens),
        ]);
        assert_eq!(
            evaluate(&mf_8_4_input(), &reviews).verdict.status,
            GateStatus::Fail
        );
    }

    /// MF §8.4: "`src/billing/Tax.py` is the case only the collision clause
    /// catches. It is not a floor path by any pattern, in any source."
    #[test]
    fn the_collision_clause_is_the_only_thing_that_catches_a_second_spelling() {
        let mut input = mf_8_4_input();
        // Remove the collided-with path from `T` and the hit disappears.
        input.tree_paths.retain(|p| p != b"src/billing/tax.py");
        let outcome = evaluate(&input, &Reviews::default());
        assert!(!outcome.floor_hits.iter().any(|h| h == "src/billing/Tax.py"));
    }

    /// MF §5.7: "**A deleted entry can collide.** If the branch deletes
    /// `AGENTS.md` while `agents.md` remains in `T`, the deletion is a hit."
    #[test]
    fn a_deletion_can_collide() {
        let input = G14Input {
            diff: vec![DiffEntry::new(100_644, 0, "docs/README.txt")],
            tree_paths: vec![b"docs/readme.txt".to_vec()],
            ..Default::default()
        };
        let outcome = evaluate(&input, &Reviews::default());
        assert_eq!(outcome.floor_hits, ["docs/README.txt"]);
    }

    /// GR §5.7 and MF §5.7: "The entry in `floor_hits` is that **diff entry's**
    /// path, as the diff produced it; the existing path it collided with is not
    /// in the diff and is never recorded." R2's trap: G14 casefolds before
    /// comparison but records the path unfolded.
    #[test]
    fn a_hit_is_recorded_as_the_diff_produced_it_never_casefolded() {
        let input = G14Input {
            diff: vec![DiffEntry::new(0, 100_644, "AGENTS.md")],
            ..Default::default()
        };
        let outcome = evaluate(&input, &Reviews::default());
        assert_eq!(outcome.floor_hits, ["AGENTS.md"]);
        assert_eq!(outcome.verdict.wires.tokens(), ["G14:AGENTS.md"]);
    }

    /// MF §5.3: "**A tombstone's `D` is empty by construction, and that is what
    /// makes `G14=pass` honest.**"
    #[test]
    fn a_tombstone_passes_g14_with_no_hits_and_no_wires() {
        let outcome = evaluate(&G14Input::default(), &Reviews::default());
        assert_eq!(outcome.verdict.status, GateStatus::Pass);
        assert!(outcome.floor_hits.is_empty());
        assert!(outcome.verdict.wires.is_empty());
    }

    /// MF §5.9 outright 1, and PB §6.3: "fails outright, review or no review."
    #[test]
    fn a_shrunk_paths_set_fails_whatever_a_review_names() {
        let input = G14Input {
            e_m_b: vec![b"AGENTS.md".to_vec(), b"CLAUDE.md".to_vec()],
            e_m_t: vec![b"AGENTS.md".to_vec()],
            ..Default::default()
        };
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a").naming(vec!["G14", "G14:CLAUDE.md"]),
        ]);
        let outcome = evaluate(&input, &reviews);
        assert_eq!(outcome.verdict.status, GateStatus::Fail);
        assert!(outcome.verdict.statuses().contains(&&G14Status::PathsShrank));
    }

    /// MF §5.9's one exception: "except a landing carrying `Spine-Upgrade:
    /// to=none`, which needs only the protected review."
    #[test]
    fn an_uninstall_may_drop_every_paths_entry() {
        let input = G14Input {
            e_m_b: vec![b"AGENTS.md".to_vec()],
            e_m_t: vec![],
            carries_to_none: true,
            ..Default::default()
        };
        assert_eq!(
            evaluate(&input, &Reviews::default()).verdict.status,
            GateStatus::Pass
        );
    }

    /// MF §5.9 outright 2, CN §6.5: byte-identical pattern sets.
    #[test]
    fn a_c_a2_pattern_dropped_at_t_is_c_a2_shrank() {
        let input = G14Input {
            c_a2_at_b: vec!["infra/".into(), "ops/".into()],
            c_a2_at_t: vec!["infra/".into()],
            ..Default::default()
        };
        let outcome = evaluate(&input, &Reviews::default());
        assert!(outcome.verdict.statuses().contains(&&G14Status::CA2Shrank));
        assert_eq!(outcome.verdict.status, GateStatus::Fail);
    }

    #[test]
    fn a_bracket_case_pattern_in_c_a2_is_outright() {
        let input = G14Input {
            c_a2_at_b: vec!["src/[A-Z]*.py".into()],
            c_a2_at_t: vec!["src/[A-Z]*.py".into()],
            ..Default::default()
        };
        let outcome = evaluate(&input, &Reviews::default());
        assert!(
            outcome
                .verdict
                .statuses()
                .contains(&&G14Status::CA2BracketCase)
        );
        assert_eq!(outcome.verdict.status, GateStatus::Fail);
    }

    /// MF §5.4: "`F` is built from `B` alone. … A candidate that adds a `C-A2`
    /// entry or a `paths` key does not thereby protect its own new paths in the
    /// same landing; it protects them from the next one."
    #[test]
    fn a_candidates_own_new_c_a2_entry_does_not_protect_its_own_paths() {
        let input = G14Input {
            diff: vec![DiffEntry::new(0, 100_644, "ops/deploy.sh")],
            c_a2_at_b: vec![],
            c_a2_at_t: vec!["ops/".into()],
            ..Default::default()
        };
        let outcome = evaluate(&input, &Reviews::default());
        assert!(outcome.floor_hits.is_empty());
    }

    /// MF §5.4: "`E(M_B)` when there is no `M_B`" — a `from=none` landing has
    /// `E(M_B) = ∅`, "which makes §5.9's outright 1 vacuous".
    #[test]
    fn a_re_init_has_an_empty_literal_floor_and_a_vacuous_outright_one() {
        let input = G14Input {
            diff: vec![DiffEntry::new(0, 100_644, ".spine/manifest.json")],
            e_m_b: vec![],
            e_m_t: vec![b"CONSTITUTION.md".to_vec()],
            ..Default::default()
        };
        let outcome = evaluate(&input, &Reviews::default());
        // "a re-init writes `.spine/**`, which is `F0` entry 1, so the landing
        // takes a protected review from the shipped floor whatever the base
        // held" (MF §5.4).
        assert_eq!(outcome.floor_hits, [".spine/manifest.json"]);
        assert!(!outcome.verdict.statuses().contains(&&G14Status::PathsShrank));
    }

    /// A path with a comma, a space and a quote: `floor_hits` carries `esc`,
    /// the token carries `tok`, and the two differ (R2).
    #[test]
    fn a_hit_on_an_awkward_path_carries_esc_in_the_report_and_tok_in_the_token() {
        let input = G14Input {
            diff: vec![DiffEntry::new(0, 100_644, "docs/a b,\"c\"/CODEOWNERS")],
            ..Default::default()
        };
        let outcome = evaluate(&input, &Reviews::default());
        assert_eq!(outcome.floor_hits, ["docs/a b,\"c\"/CODEOWNERS"]);
        assert_eq!(
            outcome.verdict.wires.tokens(),
            ["G14:docs/a\\x20b\\x2c\\x22c\\x22/CODEOWNERS"]
        );
        assert!(floor_hits_and_wires_agree(&outcome));
    }

    /// PB §7.3, PB §5.2, PB §6.3: "**Declared touchpoints are never consulted
    /// by G14.**" There is nowhere to put them, and this test pins that the
    /// input struct has no such field by exercising the whole surface.
    #[test]
    fn a_floor_hit_stands_whatever_the_intent_declared() {
        let input = G14Input {
            diff: vec![DiffEntry::new(100_644, 100_644, ".github/workflows/ci.yml")],
            ..Default::default()
        };
        assert_eq!(
            evaluate(&input, &Reviews::default()).floor_hits,
            [".github/workflows/ci.yml"]
        );
    }
}
