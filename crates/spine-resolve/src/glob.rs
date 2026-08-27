//! The one path-pattern dialect: `intent-doc.md` §6.1 (byte grammar and
//! refusals), §6.2 (the glob dialect) and §6.3 (`match(P, p)`).
//!
//! IR §2.4 adopts this by reference and defines nothing of its own: "**one
//! dialect governs all of them: `intent-doc.md` §6.1 … §6.3, adopted here by
//! reference and unaltered.**" `C-T1`, `C-T2`, `C-Q1`, `C-A2` and every
//! touchpoint entry are patterns in this dialect, and G2's quick-lane clause
//! compares a constitution list and a touchpoint list against one diff with one
//! gate — "one semantics is not a preference there" (IR §2.4.1).
//!
//! Two rules carry the whole design and are the two a second implementation
//! gets wrong:
//!
//! 1. **`**` crosses separators; `*` does not.** ID §6.2: "That is the one
//!    question every glob dialect answers differently, and it is answered here
//!    for both."
//! 2. **`match` is segment-boundary, not byte-prefix.** ID §6.3: "`src/bill`
//!    does not match `src/billing/x.ts`, because the only prefixes of that path
//!    ending at a `/` are `src` and `src/billing`, and `src/bill` is neither."
//!    IR §2.4.1 records what the byte-prefix reading cost: the shipped value
//!    `src/**/__tests__/` "matched **nothing**", so "`--approve` refused
//!    outright" for every TypeScript repository laying its tests out that way.

use core::fmt;

/// ID §6.1's refusal list. `Display` is the sub-status the refusal carries —
/// "all `bad-pattern` at exit 4 with the sub-status named" (ID §9.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternError {
    /// "empty"
    Empty,
    /// "longer than 255 bytes" (ID §2.3's bound)
    TooLong,
    /// A byte outside `0x21 … 0x7E`, or one of `,` `"` `\`. ID §6.1's table
    /// gives each exclusion its own reason: `,` is "the list separator here
    /// (§5.4) and the `wires=` separator a review signs"; `"` is "`git
    /// ls-tree`'s quoting trigger, JSON's string delimiter, and a trailer
    /// `reason=`'s"; `\` is "`esc`'s escape byte … and never a path separator
    /// in git". Space and every byte above `0x7E` fall outside the range, so
    /// **a pattern is ASCII**.
    IllegalByte { at: usize, byte: u8 },
    /// "begins `!`" — "negation makes the declared set order-dependent; G2 and
    /// G7 are set operations, and an ordered pattern list is a second semantics
    /// for `templates.md` and `constitution.md` to get wrong".
    BadNegation,
    /// "begins `/`" — "every pattern is root-anchored already; gitignore's
    /// meaning for a leading slash is *anchoring*, so accepting it would teach
    /// a false lesson".
    LeadingSlash,
    /// "contains `//`"
    EmptySegment,
    /// "has a segment `.` or `..`" — "git paths have neither; accepting them
    /// invites a matcher to resolve them".
    DotSegment,
    /// "a segment contains `**` but is not exactly `**`". ID §6.2: "Bash's
    /// globstar, minimatch and git's pathspec all disagree about what those
    /// mean; refusing removes the disagreement instead of picking a winner."
    BadGlobstar,
    /// ID §6.2's bracket rules, each with the corner it closes.
    BadBracket(&'static str),
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatternError::Empty => f.write_str("pattern-empty"),
            PatternError::TooLong => f.write_str("pattern-too-long"),
            PatternError::IllegalByte { at, byte } => {
                write!(f, "pattern-illegal-byte: 0x{byte:02x} at offset {at}")
            }
            PatternError::BadNegation => f.write_str("bad-negation"),
            PatternError::LeadingSlash => f.write_str("leading-slash"),
            PatternError::EmptySegment => f.write_str("empty-segment"),
            PatternError::DotSegment => f.write_str("dot-segment"),
            PatternError::BadGlobstar => f.write_str("bad-globstar"),
            PatternError::BadBracket(why) => write!(f, "bad-bracket: {why}"),
        }
    }
}

impl PatternError {
    /// The bare sub-status token, without the diagnostic tail `Display` adds.
    /// This is the byte string a report carries.
    pub fn token(&self) -> &'static str {
        match self {
            PatternError::Empty => "pattern-empty",
            PatternError::TooLong => "pattern-too-long",
            PatternError::IllegalByte { .. } => "pattern-illegal-byte",
            PatternError::BadNegation => "bad-negation",
            PatternError::LeadingSlash => "leading-slash",
            PatternError::EmptySegment => "empty-segment",
            PatternError::DotSegment => "dot-segment",
            PatternError::BadGlobstar => "bad-globstar",
            PatternError::BadBracket(_) => "bad-bracket",
        }
    }
}

impl core::error::Error for PatternError {}

/// A validated pattern. Holding the parse rather than the string is what keeps
/// [`Pattern::matches`] from re-validating on every path in a diff.
///
/// ID §6.1: "**`esc` and `tok` are the identity on every legal pattern.** … So
/// a pattern's bytes, its `code_unit` node id suffix, and its `G2:<path>` wire
/// token are the same bytes. Nothing here needs a second encoding." That is why
/// [`Pattern::as_str`] is safe to splice straight into a wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    source: String,
    /// ID §6.3: "A pattern that ends in `/` is a *directory* pattern and gives
    /// up the first clause: it matches things *under* the named directory and
    /// never the directory's own path."
    directory: bool,
    /// The segments of the pattern with any final empty segment removed —
    /// "a trailing `/` yields a final empty segment which §6.3 removes before
    /// splitting" (ID §6.2).
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    /// A whole segment equal to `**`: "matches **zero or more complete
    /// segments**, and `**` may appear only as a whole segment."
    Globstar,
    Literal(Vec<u8>),
    Glob(Vec<Item>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Item {
    /// "exactly one byte, and it is never `/` — a segment holds no `/`"
    Any,
    /// "zero or more bytes, none of them `/`. **`*` does not cross a
    /// separator.**"
    Star,
    Byte(u8),
    Bracket(Bracket),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Bracket {
    negated: bool,
    /// Inclusive byte ranges. A single member byte is the range `b..=b`, which
    /// keeps membership one loop rather than two.
    ranges: Vec<(u8, u8)>,
}

impl Bracket {
    fn contains(&self, b: u8) -> bool {
        // "A bracket never matches `/`, which follows from segments containing
        // none" — and it is stated here too, because a *negated* bracket would
        // otherwise admit one and let a segment swallow a separator.
        if b == b'/' {
            return false;
        }
        let hit = self.ranges.iter().any(|&(lo, hi)| lo <= b && b <= hi);
        hit != self.negated
    }
}

/// ID §6.1's byte range, less the three excluded bytes.
fn legal_byte(b: u8) -> bool {
    (0x21..=0x7E).contains(&b) && b != b',' && b != b'"' && b != b'\\'
}

impl Pattern {
    /// Parse and validate. ID §6.1's refusals are total and each is named.
    ///
    /// The check order is this implementation's, and the corpus fixes only one
    /// edge of it: "Brackets are validated over the whole pattern **before** it
    /// is split into segments, so `[a/b]` is refused as `bad-bracket` rather
    /// than silently becoming two malformed segments" (ID §6.2). The rest —
    /// length before bytes, negation before slash — is DERIVED: ID §8.2 fixes
    /// one failure order for a *document*, and no section orders the pattern
    /// sub-statuses against each other. A pattern wrong in two ways therefore
    /// gets one of two defensible tokens, and two implementations that ordered
    /// them differently would disagree on that pattern's sub-status only.
    pub fn parse(source: &str) -> Result<Self, PatternError> {
        let bytes = source.as_bytes();
        if bytes.is_empty() {
            return Err(PatternError::Empty);
        }
        if bytes.len() > 255 {
            return Err(PatternError::TooLong);
        }
        if let Some((at, &byte)) = bytes.iter().enumerate().find(|(_, b)| !legal_byte(**b)) {
            return Err(PatternError::IllegalByte { at, byte });
        }
        if bytes[0] == b'!' {
            return Err(PatternError::BadNegation);
        }
        if bytes[0] == b'/' {
            return Err(PatternError::LeadingSlash);
        }

        // Brackets first, over the whole pattern, so `[a/b]` is `bad-bracket`.
        // The scan also records where each bracket ends, which is what lets the
        // per-segment parse below reuse the validation instead of repeating it.
        validate_brackets(bytes)?;

        if source.contains("//") {
            return Err(PatternError::EmptySegment);
        }

        let directory = bytes[bytes.len() - 1] == b'/';
        let body = if directory {
            &source[..source.len() - 1]
        } else {
            source
        };
        // `body` is empty only for the pattern `/`, which `LeadingSlash` already
        // refused, so `split('/')` below yields at least one non-empty segment.
        let mut segments = Vec::new();
        for seg in body.split('/') {
            segments.push(parse_segment(seg.as_bytes())?);
        }

        Ok(Pattern {
            source: source.to_string(),
            directory,
            segments,
        })
    }

    /// The pattern's own bytes, unchanged. `esc` and `tok` are the identity on
    /// them (ID §6.1), so these are also its wire token's bytes.
    pub fn as_str(&self) -> &str {
        &self.source
    }

    /// ID §6.3's directory form — a pattern ending in `/`.
    pub fn is_directory(&self) -> bool {
        self.directory
    }

    /// `match(P, p)` of ID §6.3, over a repository path "as git produces it: a
    /// byte string, `/`-separated, no leading `/`, no `.` or `..` component, no
    /// trailing `/`".
    ///
    /// ID §6.4: "**No path is normalised, casefolded or decomposed** before
    /// matching. It is compared as the diff produced it." And ID §6.5:
    /// "**Touchpoint matching is byte-exact and case-sensitive.**" — deliberately
    /// unlike G14's floor comparison, because "a case-insensitive touchpoint
    /// match would produce one answer on a case-sensitive filesystem and
    /// another on a case-insensitive one for the same objects."
    pub fn matches(&self, path: &[u8]) -> bool {
        let ss = split_path(path);
        // "∃ a split p = q ++ "/" ++ r, with r non-empty" — so `q` is a
        // **non-empty proper** prefix of the segment list. `k = 0` would mean
        // `p` begins with `/`, which git never produces; `k = |ss|` would leave
        // `r` empty. Both ends are excluded, and excluding them is what makes
        // the directory form give up the whole-path clause rather than merely
        // re-spell it.
        let boundary = || (1..ss.len()).any(|k| self.gmatch(&ss[..k]));
        if self.directory {
            boundary()
        } else {
            // "gmatch(P, p) ∨ ∃ a split …"
            self.gmatch(&ss) || boundary()
        }
    }

    /// Convenience for the common case where the caller already holds a `&str`.
    pub fn matches_str(&self, path: &str) -> bool {
        self.matches(path.as_bytes())
    }

    /// `gmatch(P, s)` — whole-segment-list matching, ID §6.3's `go(i, j)`.
    ///
    /// Computed as a forward reachability sweep rather than by recursion: a
    /// pattern with several `**` segments makes the recursive form exponential
    /// on an adversarial path, and a matcher whose cost a branch controls is a
    /// denial of service against every landing that reads its patterns.
    fn gmatch(&self, ss: &[&[u8]]) -> bool {
        // reach[j] == "some prefix of the pattern consumed ss[..j]".
        let mut reach = vec![false; ss.len() + 1];
        reach[0] = true;
        for seg in &self.segments {
            let mut next = vec![false; ss.len() + 1];
            match seg {
                // "∃ k ∈ [j, |ss|] : go(i+1, k)" — zero or more segments, and
                // **zero is uniform**: "`a/**/b` matches `a/b`; `**/x` matches
                // `x`; and, following the same rule with no special case,
                // `a/**` matches `a`."
                Segment::Globstar => {
                    let mut open = false;
                    for j in 0..=ss.len() {
                        open |= reach[j];
                        next[j] = open;
                    }
                }
                _ => {
                    for j in 0..ss.len() {
                        if reach[j] && segmatch(seg, ss[j]) {
                            next[j + 1] = true;
                        }
                    }
                }
            }
            reach = next;
            if !reach.iter().any(|&r| r) {
                return false;
            }
        }
        // "if i = |ps| : j = |ss|" — the pattern must have consumed the whole
        // segment list, not merely a prefix of it.
        reach[ss.len()]
    }
}

/// A path's segments. An empty path has none — which is what makes the k = 0
/// term of the directory clause match nothing, since no pattern has zero
/// segments.
fn split_path(path: &[u8]) -> Vec<&[u8]> {
    if path.is_empty() {
        return Vec::new();
    }
    path.split(|&b| b == b'/').collect()
}

fn segmatch(seg: &Segment, s: &[u8]) -> bool {
    match seg {
        // Handled by the caller; a `**` never reaches here.
        Segment::Globstar => false,
        Segment::Literal(lit) => lit == s,
        Segment::Glob(items) => glob_items(items, s),
    }
}

/// Item-list matching within one segment. Same reachability sweep, same reason:
/// `a*b*c*d*e` against a long segment is exponential under backtracking.
fn glob_items(items: &[Item], s: &[u8]) -> bool {
    let mut reach = vec![false; s.len() + 1];
    reach[0] = true;
    for item in items {
        let mut next = vec![false; s.len() + 1];
        match item {
            Item::Star => {
                let mut open = false;
                for j in 0..=s.len() {
                    open |= reach[j];
                    next[j] = open;
                }
            }
            Item::Any => {
                for j in 0..s.len() {
                    // "exactly one byte, and it is never `/`". A path segment
                    // holds no `/` by construction, so the guard is redundant
                    // and is kept because the rule is stated, not inferred.
                    if reach[j] && s[j] != b'/' {
                        next[j + 1] = true;
                    }
                }
            }
            Item::Byte(b) => {
                for j in 0..s.len() {
                    if reach[j] && s[j] == *b {
                        next[j + 1] = true;
                    }
                }
            }
            Item::Bracket(bracket) => {
                for j in 0..s.len() {
                    if reach[j] && bracket.contains(s[j]) {
                        next[j + 1] = true;
                    }
                }
            }
        }
        reach = next;
        if !reach.iter().any(|&r| r) {
            return false;
        }
    }
    reach[s.len()]
}

/// ID §6.2's bracket grammar, validated over the **whole pattern** before any
/// split:
///
/// ```text
/// bracket := "[" [ "!" ] [ "]" ] member* "]"
/// member  := byte | byte "-" byte
/// ```
fn validate_brackets(bytes: &[u8]) -> Result<(), PatternError> {
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        let (_, end) = parse_bracket(bytes, i)?;
        i = end;
    }
    Ok(())
}

/// Parse one bracket starting at `bytes[start] == b'['`. Returns the bracket
/// and the index one past its closing `]`.
fn parse_bracket(bytes: &[u8], start: usize) -> Result<(Bracket, usize), PatternError> {
    debug_assert_eq!(bytes[start], b'[');
    let mut i = start + 1;
    // "A leading `!` negates. `^` does **not** negate; it is an ordinary member
    // byte. One spelling, not two."
    let negated = bytes.get(i) == Some(&b'!');
    if negated {
        i += 1;
    }
    // "POSIX classes, collating symbols and equivalence classes are refused: a
    // bracket whose first member byte (after an optional `!`) is `:`, `.` or
    // `=` … is `bad-bracket`. Their meaning is locale-dependent, and a locale
    // is exactly the kind of environment input this design keeps out of a
    // verdict."
    if matches!(bytes.get(i), Some(b':') | Some(b'.') | Some(b'=')) {
        return Err(PatternError::BadBracket(
            "a POSIX class, collating symbol or equivalence class",
        ));
    }
    let mut ranges: Vec<(u8, u8)> = Vec::new();
    // "A `]` immediately after `[` or `[!` is a literal member. `[]]` is the
    // set `{ ] }`."
    let mut first = true;
    loop {
        let Some(&b) = bytes.get(i) else {
            // "An unterminated `[` is `bad-bracket`. It is **not** treated as a
            // literal `[`."
            return Err(PatternError::BadBracket("unterminated"));
        };
        if b == b']' && !first {
            return Ok((Bracket { negated, ranges }, i + 1));
        }
        first = false;
        // "`/` inside a bracket is `bad-bracket`."
        if b == b'/' {
            return Err(PatternError::BadBracket("a `/` inside a bracket"));
        }
        // "or which contains the two-byte sequence `[:`, `[.` or `[=`".
        if b == b'[' && matches!(bytes.get(i + 1), Some(b':') | Some(b'.') | Some(b'=')) {
            return Err(PatternError::BadBracket(
                "a POSIX class, collating symbol or equivalence class",
            ));
        }
        // `member := byte | byte "-" byte`, read left to right and greedily.
        // A `-` whose right-hand side is the closing `]` is an ordinary member
        // byte: that is DERIVED — the grammar admits no trailing `-` and the
        // only alternative reading leaves the bracket unterminated, which would
        // refuse `[a-]` for having a member the grammar does not name.
        if bytes.get(i + 1) == Some(&b'-')
            && let Some(&hi) = bytes.get(i + 2)
            && hi != b']'
        {
            if hi == b'/' {
                return Err(PatternError::BadBracket("a `/` inside a bracket"));
            }
            // "A range `a-b` requires `a ≤ b` as byte values; `[z-a]` is
            // `bad-bracket`."
            if b > hi {
                return Err(PatternError::BadBracket("a reversed range"));
            }
            ranges.push((b, hi));
            i += 3;
            continue;
        }
        ranges.push((b, b));
        i += 1;
    }
}

fn parse_segment(seg: &[u8]) -> Result<Segment, PatternError> {
    // ID §6.1: "a segment contains `**` but is not exactly `**` is
    // `bad-globstar`." Read literally, over the segment's raw bytes: a `**`
    // inside a bracket is refused too, which is the reading that needs no
    // second rule about where the scan looks.
    if seg == b"**" {
        return Ok(Segment::Globstar);
    }
    if seg.windows(2).any(|w| w == b"**") {
        return Err(PatternError::BadGlobstar);
    }
    if seg == b"." || seg == b".." {
        return Err(PatternError::DotSegment);
    }

    let mut items = Vec::new();
    let mut literal = true;
    let mut i = 0;
    while i < seg.len() {
        match seg[i] {
            b'?' => {
                literal = false;
                items.push(Item::Any);
                i += 1;
            }
            b'*' => {
                literal = false;
                items.push(Item::Star);
                i += 1;
            }
            b'[' => {
                literal = false;
                // Already validated over the whole pattern; re-parsing here is
                // the same function and cannot disagree with itself.
                let (bracket, end) = parse_bracket(seg, i)?;
                items.push(Item::Bracket(bracket));
                i = end;
            }
            // "any other byte | itself, exactly". `{` and `}` are ordinary
            // bytes: "**No brace expansion.** … `{a,b}` cannot arise anyway —
            // the comma is the list separator."
            b => {
                items.push(Item::Byte(b));
                i += 1;
            }
        }
    }
    if literal {
        // A metacharacter-free segment compares as bytes, which is both faster
        // and the shape the `Literal` arm of `segmatch` needs.
        Ok(Segment::Literal(seg.to_vec()))
    } else {
        Ok(Segment::Glob(items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(pattern: &str, path: &str) -> bool {
        Pattern::parse(pattern)
            .unwrap_or_else(|e| panic!("{pattern}: {e}"))
            .matches_str(path)
    }

    /// `intent-doc.md` §9.5's matching vectors, every row, in the published
    /// order. "The first row is the one the audit named."
    #[test]
    fn id_9_5_the_published_matching_vectors() {
        for (pattern, path, expected) in [
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
            assert_eq!(m(pattern, path), expected, "match({pattern:?}, {path:?})");
        }
    }

    /// `import-resolver.md` §2.4.2's vector, every row. Its last column is the
    /// dialect this module replaces: "Under version 1 §2.4" is what the shipped
    /// `C-T1` value matched, and it matched nothing.
    #[test]
    fn ir_2_4_2_the_vector_including_the_defect_row() {
        for (pattern, path, expected) in [
            ("src/**/__tests__/", "src/billing/__tests__/x.test.ts", true),
            (
                "src/**/__tests__/",
                "src/billing/__tests__/nested/y.test.ts",
                true,
            ),
            ("src/**/__tests__/", "src/__tests__/z.test.ts", true),
            ("src/**/__tests__/", "src/billing/__tests__", false),
            ("src/**/__tests__/", "src/billing/x.test.ts", false),
            ("tests/", "tests/a/b.py", true),
            ("tests/", "tests", false),
            ("tests/", "testsuite/x.py", false),
            ("**/conftest.py", "conftest.py", true),
            ("**/conftest.py", "tests/billing/conftest.py", true),
            ("pytest.ini", "pytest.ini", true),
            ("pytest.ini", "tools/pytest.ini", false),
            ("tests/support/**", "tests/support", true),
            ("tests/support/**", "tests/support/factories.py", true),
            ("vitest.config.*", "vitest.config.ts", true),
            ("vitest.config.*", "packages/a/vitest.config.ts", false),
            ("Tests/Support/**", "Tests/Support/Fixtures.swift", true),
            ("test/support/**", "test/support/index.dart", true),
            ("src/bill", "src/billing/x.ts", false),
        ] {
            assert_eq!(m(pattern, path), expected, "match({pattern:?}, {path:?})");
        }
    }

    /// ID §9.5's refusal vectors, every row, with the sub-status each names.
    #[test]
    fn id_9_5_the_published_refusal_vectors() {
        for (pattern, token) in [
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
            ("a,b", "pattern-illegal-byte"),
            ("a\"b", "pattern-illegal-byte"),
            ("a\\b", "pattern-illegal-byte"),
            ("a b", "pattern-illegal-byte"),
            ("é/x", "pattern-illegal-byte"),
            ("", "pattern-empty"),
        ] {
            let err = Pattern::parse(pattern)
                .expect_err(&format!("{pattern:?} must be refused"))
                .token();
            assert_eq!(err, token, "{pattern:?}");
        }
    }

    /// ID §9.5: "Accepted, to pin the boundary".
    #[test]
    fn id_9_5_the_accepted_boundary_patterns() {
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
            assert!(Pattern::parse(pattern).is_ok(), "{pattern:?} must be legal");
        }
    }

    /// ID §2.3's bound, and the boundary on both sides of it.
    #[test]
    fn a_pattern_is_one_to_255_bytes() {
        assert_eq!(Pattern::parse(&"a".repeat(255)).map(|_| ()), Ok(()));
        assert_eq!(
            Pattern::parse(&"a".repeat(256)).unwrap_err(),
            PatternError::TooLong
        );
    }

    /// ID §6.2: "`[]]` is the set `{ ] }`."
    #[test]
    fn a_close_bracket_immediately_after_the_open_is_a_literal_member() {
        assert!(m("src/[]]x", "src/]x"));
        assert!(!m("src/[]]x", "src/ax"));
        assert!(m("[!]]", "a"));
        assert!(!m("[!]]", "]"));
    }

    /// ID §6.2: "`^` does **not** negate; it is an ordinary member byte. One
    /// spelling, not two."
    #[test]
    fn caret_is_an_ordinary_member_byte_and_never_a_negation() {
        assert!(m("[^a]", "^"));
        assert!(m("[^a]", "a"));
        assert!(!m("[^a]", "b"));
    }

    /// ID §6.2: "A range `a-b` requires `a ≤ b` as byte values; `[z-a]` is
    /// `bad-bracket`."
    #[test]
    fn a_reversed_range_is_bad_bracket() {
        assert_eq!(
            Pattern::parse("[z-a]").unwrap_err().token(),
            "bad-bracket"
        );
        assert!(m("[a-c]x", "bx"));
        assert!(!m("[a-c]x", "dx"));
    }

    /// ID §6.2: "`/` inside a bracket is `bad-bracket`" — and the reason the
    /// validation runs over the whole pattern first: "so `[a/b]` is refused as
    /// `bad-bracket` rather than silently becoming two malformed segments."
    #[test]
    fn a_slash_inside_a_bracket_is_bad_bracket_and_not_two_segments() {
        assert_eq!(Pattern::parse("[a/b]").unwrap_err().token(), "bad-bracket");
        assert_eq!(Pattern::parse("x/[a/b]/y").unwrap_err().token(), "bad-bracket");
    }

    /// A negated bracket must not admit `/` either — a segment holds none, and
    /// a matcher that let one through would let `*`-free segments cross a
    /// separator.
    #[test]
    fn a_negated_bracket_never_matches_a_separator() {
        assert!(!m("a[!x]b", "a/b"));
        assert!(m("a[!x]b", "ayb"));
    }

    /// ID §6.2: "**No brace expansion.** `{` and `}` are ordinary bytes."
    #[test]
    fn braces_are_ordinary_bytes() {
        assert!(m("a{b}c", "a{b}c"));
        assert!(!m("a{b}c", "ab"));
    }

    /// ID §6.2: "Multiple single `*` in one segment are fine: `a*b*c` is
    /// legal." And `*` still does not cross a separator.
    #[test]
    fn star_never_crosses_a_separator_however_many_of_them_there_are() {
        assert!(m("a*b*c", "axxbyyc"));
        assert!(!m("a*b*c", "ax/byyc"));
        assert!(!m("src/*", "src"));
    }

    /// ID §6.2: "**`**` matching zero segments is uniform.**"
    #[test]
    fn globstar_matches_zero_segments_with_no_special_case() {
        assert!(m("a/**/b", "a/b"));
        assert!(m("**/x", "x"));
        assert!(m("a/**", "a"));
        assert!(m("**", "anything"));
    }

    /// ID §6.3's table, "Directory versus file, distinguished."
    #[test]
    fn a_trailing_slash_gives_up_the_whole_path_clause() {
        assert!(!m("src/billing/", "src/billing"));
        assert!(m("src/billing", "src/billing"));
        assert!(m("src/billing/", "src/billing/x.ts"));
        assert!(m("src/billing", "src/billing/x.ts"));
        assert!(!m("src/billing/", "src/billingx/y.ts"));
        assert!(!m("src/billing", "src/billingx/y.ts"));
    }

    /// ID §6.3: "**Vacuous patterns are legal.** `api/invoices.ts/` matches
    /// nothing unless a directory of that name exists. The parse cannot know,
    /// and does not guess."
    #[test]
    fn a_vacuous_directory_pattern_is_legal_and_matches_only_below_itself() {
        let p = Pattern::parse("api/invoices.ts/").unwrap();
        assert!(!p.matches_str("api/invoices.ts"));
        assert!(p.matches_str("api/invoices.ts/x"));
    }

    /// ID §6.5: "**Touchpoint matching is byte-exact and case-sensitive.**
    /// `src/Billing/x.ts` does not match `src/billing/`."
    #[test]
    fn matching_is_byte_exact_and_never_casefolded() {
        assert!(!m("src/billing/", "src/Billing/x.ts"));
        assert!(m("src/Billing/", "src/Billing/x.ts"));
    }

    /// ID §6.4: paths are compared "as the diff produced it", so a path that is
    /// not UTF-8 must still be matchable — which is why `matches` takes bytes.
    #[test]
    fn a_non_utf8_path_still_matches() {
        let p = Pattern::parse("src/*").unwrap();
        assert!(p.matches(b"src/\xff\xfe.bin"));
        assert!(p.matches(b"src/\xff/deeper"));
    }

    /// ID §6.1: "**`esc` and `tok` are the identity on every legal pattern.**
    /// No legal pattern contains a byte `esc` escapes (`\\`, or anything outside
    /// `0x20…0x7E`) or a byte `tok` additionally escapes (`,`, space, `"`). So
    /// a pattern's bytes, its `code_unit` node id suffix, and its `G2:<path>`
    /// wire token are the same bytes."
    ///
    /// Run against `spine-canon`'s own encoders rather than restated, because
    /// the claim is about *those two functions* and a restatement of it here
    /// would go stale the moment either widened.
    #[test]
    fn esc_and_tok_are_the_identity_on_every_legal_pattern() {
        let corpus = [
            "src/billing/",
            "src/**/__tests__/",
            "**/conftest.py",
            "src/[!abc]*.ts",
            "a*b*c",
            "src/[]]x",
            "vitest.config.*",
            "Tests/Support/**",
            "a/**/b",
            "**",
            "{a}",
            "x?y",
        ];
        for pattern in corpus {
            let parsed = Pattern::parse(pattern).unwrap_or_else(|e| panic!("{pattern}: {e}"));
            let bytes = parsed.as_str().as_bytes();
            assert_eq!(spine_canon::esc(bytes), pattern, "esc({pattern})");
            assert_eq!(spine_canon::tok(bytes), pattern, "tok({pattern})");
        }

        // …and the property holds byte-wise, over the whole legal range, which
        // is what makes the corpus above a sample rather than the proof.
        let mut legal = 0usize;
        for b in 0u8..=255 {
            if !legal_byte(b) {
                continue;
            }
            legal += 1;
            let one = [b];
            assert_eq!(spine_canon::esc(&one).as_bytes(), one, "esc(0x{b:02x})");
            assert_eq!(spine_canon::tok(&one).as_bytes(), one, "tok(0x{b:02x})");
        }
        // 0x21..=0x7E is 94 bytes; `,`, `"` and `\` are inside it and excluded.
        assert_eq!(legal, 94 - 3);
    }

    /// A pattern a branch controls must not be able to make matching
    /// exponential — every landing reads trunk's and the branch's pattern
    /// lists, so a matcher whose cost a pattern chooses is a denial of service.
    #[test]
    fn many_globstars_against_a_long_path_stay_cheap() {
        let pattern = Pattern::parse("a/**/**/**/**/**/**/**/**/**/**/z").unwrap();
        let path = format!("a/{}", "x/".repeat(200));
        assert!(!pattern.matches_str(path.trim_end_matches('/')));
        assert!(pattern.matches_str(&format!("a/{}z", "x/".repeat(200))));
    }
}
