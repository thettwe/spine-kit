//! IR §3.4's lexical preliminaries, and the four per-language lexers of §4.1,
//! §5.1, §6.1 and §7.1.
//!
//! IR §3.4, the level and why it is that level: "The resolver does not parse.
//! It **lexes**, and matches token patterns. That is a deliberate level: a full
//! grammar for four languages is not writable in one document and would not be
//! implemented identically twice, while the lexical rules below are small,
//! closed, and sufficient — because in all four languages the import forms are
//! anchored on a reserved word and terminated by a string literal or a dotted
//! name."
//!
//! Two consumers rest on this module and both are load-bearing:
//!
//! - [`crate::pragma`] scans `comment` tokens, which is why IR §3.4 rule 4 says
//!   comments are discarded **after** §12's scan and why this lexer emits them
//!   rather than dropping them. Case J2 is the whole reason the distinction
//!   between a comment and a string must be real: "`\"\"\"@verifies
//!   INT-042/AC-1\"\"\"` in a Python docstring — **no** occurrence — a docstring
//!   is a string, not a comment."
//! - IR §11.5's `id → path` for `swift-test` reads "its token stream (lexed by
//!   §7.1, with comments and string literals discarded)" for the two-token
//!   sequence `class` `<name>`.

use crate::lang::{FileNotUtf8, Lang};

/// IR §3.4 rule 3's four kinds, plus `newline`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// "a maximal run of `[A-Za-z0-9_$]`, plus `.` never — `.` is punctuation".
    Word,
    Str(StrInfo),
    /// "any other single byte"
    Punct,
    /// Per language (§4.1, §5.1, §6.1, §7.1), and "discarded **after** the
    /// pragma scan of §12, which reads them".
    Comment,
    /// "produced for Python, where a line break is syntactically significant;
    /// discarded elsewhere".
    Newline,
}

/// What §3.4 rule 5 needs to know about a string literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrInfo {
    /// IR §3.4 rule 5: "A literal is **simple** iff it is a single literal
    /// token, contains no interpolation, and contains no backslash. A specifier
    /// that is not a simple literal is `unresolvable` … This is over-strict by
    /// design: a specifier with an escape in it is either exotic or evasive,
    /// and the cost of refusing it is one tripwire."
    ///
    /// The *single-token* half of the rule cannot be decided here — it is about
    /// adjacent-literal concatenation, which is two tokens — so this flag
    /// carries the other two halves and the caller checks the first.
    pub simple: bool,
    /// Byte range of the literal's content: inside the quotes and after any
    /// prefix. This is the specifier's own bytes for a simple literal.
    pub content: (usize, usize),
}

/// One token, located by byte offset so that IR §3.2's "`(path, byte offset of
/// the first token)`" site identity is available to a caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn text<'a>(&self, src: &'a str) -> &'a str {
        &src[self.start..self.end]
    }

    /// True iff this is a `word` token whose bytes are exactly `word`.
    pub fn is_word(&self, src: &str, word: &str) -> bool {
        self.kind == TokenKind::Word && self.text(src) == word
    }

    /// True iff this is a single-byte `punct` token equal to `b`.
    pub fn is_punct(&self, src: &str, b: u8) -> bool {
        self.kind == TokenKind::Punct && self.text(src).as_bytes() == [b]
    }

    /// A simple string literal's content bytes, or `None`.
    pub fn simple_literal<'a>(&self, src: &'a str) -> Option<&'a str> {
        match &self.kind {
            TokenKind::Str(info) if info.simple => Some(&src[info.content.0..info.content.1]),
            _ => None,
        }
    }
}

/// IR §3.4 rule 1: "A file is decoded as UTF-8. A file that is not valid UTF-8
/// is not lexed: it contributes no edges and raises `file-not-utf8` (§2.11). No
/// encoding declaration is honoured — not PEP 263's coding cookie, not a BOM,
/// not an XML declaration."
///
/// Case C17 names the failure this forbids: "**must not** fall back to latin-1
/// or to a coding declaration."
pub fn decode(bytes: &[u8]) -> Result<&str, FileNotUtf8> {
    core::str::from_utf8(bytes).map_err(|_| FileNotUtf8)
}

/// Lex `src` for `lang`. `src` must already be valid UTF-8 — see [`decode`].
///
/// IR §3.4 rule 1: "A leading UTF-8 BOM (`EF BB BF`) is skipped and is not part
/// of the first token." Offsets are into `src` and therefore start at 3 for a
/// file carrying one.
pub fn lex(src: &str, lang: Lang) -> Vec<Token> {
    let mut lexer = Lexer {
        src: src.as_bytes(),
        i: 0,
        out: Vec::new(),
        depth: 0,
    };
    // The BOM is skipped, not tokenized.
    if lexer.src.starts_with(b"\xEF\xBB\xBF") {
        lexer.i = 3;
    }
    match lang {
        Lang::Python => lexer.run_python(),
        Lang::Ts => lexer.run_ts(),
        Lang::Dart => lexer.run_dart(),
        Lang::Swift => lexer.run_swift(),
    }
    lexer.out
}

/// The token stream with `comment` tokens removed — IR §3.4 rule 4's "discarded
/// before matching", applied after §12's scan has already read them.
pub fn without_comments(tokens: Vec<Token>) -> Vec<Token> {
    tokens
        .into_iter()
        .filter(|t| t.kind != TokenKind::Comment)
        .collect()
}

struct Lexer<'a> {
    src: &'a [u8],
    i: usize,
    out: Vec<Token>,
    /// Python's bracket nesting: "a logical line ends at a newline that is not
    /// inside `(`/`[`/`{`".
    depth: usize,
}

/// IR §3.4 rule 3's `word` class.
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

impl<'a> Lexer<'a> {
    fn peek(&self, k: usize) -> Option<u8> {
        self.src.get(self.i + k).copied()
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        let end = self.i;
        self.out.push(Token { kind, start, end });
    }

    /// Consume a `word` token. The caller has checked the first byte.
    fn word(&mut self) -> (usize, usize) {
        let start = self.i;
        while self.peek(0).is_some_and(is_word_byte) {
            self.i += 1;
        }
        (start, self.i)
    }

    /// IR §3.4 rule 2: "LF, CRLF and CR each terminate a line." Returns true if
    /// a terminator was consumed.
    fn eat_line_terminator(&mut self) -> bool {
        match self.peek(0) {
            Some(b'\n') => {
                self.i += 1;
                true
            }
            Some(b'\r') => {
                self.i += 1;
                if self.peek(0) == Some(b'\n') {
                    self.i += 1;
                }
                true
            }
            _ => false,
        }
    }

    /// A `//` or `#` comment: to end of line, terminator excluded.
    fn line_comment(&mut self) {
        let start = self.i;
        while let Some(b) = self.peek(0) {
            if b == b'\n' || b == b'\r' {
                break;
            }
            self.i += 1;
        }
        self.push(TokenKind::Comment, start);
    }

    /// A `/* … */` block comment. `nested` is the one difference between the
    /// C-shaped languages: Dart §6.1 and Swift §7.1 nest them, TypeScript §5.1
    /// does not. IR §6.1 says why it matters: "a lexer that does not nest them
    /// mis-lexes a commented-out block containing `*/`."
    fn block_comment(&mut self, nested: bool) {
        let start = self.i;
        self.i += 2;
        let mut depth = 1usize;
        while self.i < self.src.len() {
            if self.peek(0) == Some(b'*') && self.peek(1) == Some(b'/') {
                self.i += 2;
                depth -= 1;
                if depth == 0 {
                    break;
                }
                continue;
            }
            if nested && self.peek(0) == Some(b'/') && self.peek(1) == Some(b'*') {
                self.i += 2;
                depth += 1;
                continue;
            }
            self.i += 1;
        }
        self.push(TokenKind::Comment, start);
    }

    fn punct(&mut self) {
        let start = self.i;
        self.i += 1;
        self.push(TokenKind::Punct, start);
    }

    /// Scan a quoted body and report `(content_end_exclusive, has_backslash,
    /// terminated)`. `quote` is the full delimiter (1 or 3 bytes).
    fn scan_quoted(&mut self, quote: &[u8]) -> (usize, bool, bool) {
        self.scan_quoted_raw(quote, false)
    }

    /// `raw` suppresses backslash escaping.
    ///
    /// In a Dart raw literal (`r'…'`) a backslash is an ordinary byte, so
    /// `r'a\'` is a complete literal ending in a backslash. Treating it as an
    /// escape consumed the closing quote, ran the literal on past the newline,
    /// and swallowed every statement until the next quote — an import on the
    /// following line simply vanished, with no diagnostic. A fail-open lexer
    /// bug, and branch-controlled: the branch chooses whether its file
    /// contains one.
    ///
    /// `backslash` is still reported for a raw literal, because §3.4 rule 5's
    /// "simple literal" test is about the *content*, and a raw literal
    /// containing a backslash is still not simple.
    fn scan_quoted_raw(&mut self, quote: &[u8], raw: bool) -> (usize, bool, bool) {
        let mut backslash = false;
        loop {
            if self.i >= self.src.len() {
                return (self.i, backslash, false);
            }
            if self.src[self.i] == b'\\' {
                backslash = true;
                if raw {
                    // An ordinary byte: it neither escapes the next one nor
                    // stops the closing quote from closing.
                    self.i += 1;
                    continue;
                }
                // An escape consumes the next byte, which is what stops `\"`
                // from closing the literal.
                self.i += core::cmp::min(2, self.src.len() - self.i);
                continue;
            }
            if self.src[self.i..].starts_with(quote) {
                let content_end = self.i;
                self.i += quote.len();
                return (content_end, backslash, true);
            }
            self.i += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Python — IR §4.1
// ---------------------------------------------------------------------------

/// IR §4.1: "each optionally prefixed by any case-insensitive combination of
/// `r`, `b`, `f`, `u`, `rb`, `br`."
fn python_string_prefix(word: &str) -> Option<PyPrefix> {
    let lower = word.to_ascii_lowercase();
    let ok = matches!(
        lower.as_str(),
        "r" | "u" | "b" | "f" | "rb" | "br" | "rf" | "fr"
    );
    if !ok {
        return None;
    }
    Some(PyPrefix {
        // "A literal is **simple** … only when its prefix is empty or `r`/`u`,
        // it is not an f-string, and it contains no backslash."
        simple_prefix: matches!(lower.as_str(), "r" | "u"),
    })
}

struct PyPrefix {
    simple_prefix: bool,
}

impl Lexer<'_> {
    fn run_python(&mut self) {
        while self.i < self.src.len() {
            let b = self.src[self.i];
            match b {
                b' ' | b'\t' | 0x0C => self.i += 1,
                b'#' => self.line_comment(),
                b'\n' | b'\r' => {
                    let start = self.i;
                    self.eat_line_terminator();
                    // "a logical line ends at a newline that is not inside
                    // `(`/`[`/`{`". Inside brackets the break is whitespace,
                    // which is what makes case P12's parenthesized multi-line
                    // `from a import (b, c)` **one** site.
                    if self.depth == 0 {
                        self.out.push(Token {
                            kind: TokenKind::Newline,
                            start,
                            end: self.i,
                        });
                    }
                }
                b'\\' => {
                    // "and is not preceded by a backslash continuation" — the
                    // backslash and the terminator both vanish, so the logical
                    // line runs on.
                    let start = self.i;
                    self.i += 1;
                    if !self.eat_line_terminator() {
                        self.out.push(Token {
                            kind: TokenKind::Punct,
                            start,
                            end: self.i,
                        });
                    }
                }
                b'(' | b'[' | b'{' => {
                    self.depth += 1;
                    self.punct();
                }
                b')' | b']' | b'}' => {
                    self.depth = self.depth.saturating_sub(1);
                    self.punct();
                }
                b'\'' | b'"' => self.python_string(self.i, None),
                b if is_word_byte(b) => {
                    let (start, end) = self.word();
                    // A prefixed literal is one token, not a word followed by a
                    // string: `rb'x'` must never lex as the word `rb`.
                    let word = core::str::from_utf8(&self.src[start..end]).unwrap_or("");
                    match (python_string_prefix(word), self.peek(0)) {
                        (Some(prefix), Some(b'\'')) | (Some(prefix), Some(b'"')) => {
                            self.python_string(start, Some(prefix));
                        }
                        _ => self.out.push(Token {
                            kind: TokenKind::Word,
                            start,
                            end,
                        }),
                    }
                }
                _ => self.punct(),
            }
        }
    }

    /// `start` is the token's first byte — the prefix's, where there is one.
    fn python_string(&mut self, start: usize, prefix: Option<PyPrefix>) {
        let q = self.src[self.i];
        let triple = self.peek(1) == Some(q) && self.peek(2) == Some(q);
        let quote_buf = [q, q, q];
        let quote: &[u8] = if triple { &quote_buf } else { &quote_buf[..1] };
        self.i += quote.len();
        let content_start = self.i;
        let (content_end, backslash, _terminated) = self.scan_quoted(quote);
        let simple_prefix = prefix.map(|p| p.simple_prefix).unwrap_or(true);
        self.push(
            TokenKind::Str(StrInfo {
                simple: simple_prefix && !backslash,
                content: (content_start, content_end),
            }),
            start,
        );
    }
}

// ---------------------------------------------------------------------------
// TypeScript / JavaScript — IR §5.1
// ---------------------------------------------------------------------------

impl Lexer<'_> {
    fn run_ts(&mut self) {
        // IR §5.1's regex rule: "a `/` that follows a `word`, `)`, `]` or a
        // numeric literal is division, otherwise it opens a regex. (This rule
        // exists only so that a `//` inside a regex is not read as a comment;
        // no import form can occur inside one.)" A numeric literal lexes as a
        // `word` under §3.4 rule 3, so the four cases are three.
        while self.i < self.src.len() {
            let b = self.src[self.i];
            match b {
                b' ' | b'\t' | 0x0C => self.i += 1,
                b'\n' | b'\r' => {
                    self.eat_line_terminator();
                }
                b'/' if self.peek(1) == Some(b'/') => self.line_comment(),
                b'/' if self.peek(1) == Some(b'*') => self.block_comment(false),
                b'/' if !self.division_position() => self.ts_regex(),
                b'\'' | b'"' => self.ts_quoted(),
                b'`' => self.ts_template(),
                b if is_word_byte(b) => {
                    let (start, end) = self.word();
                    self.out.push(Token {
                        kind: TokenKind::Word,
                        start,
                        end,
                    });
                }
                _ => self.punct(),
            }
        }
    }

    /// True iff the previous significant token makes a `/` division.
    fn division_position(&self) -> bool {
        // Comments are not significant here; a `/* c */ /re/` still opens a
        // regex, and reading the comment as the previous token would make it
        // division and swallow the rest of the file.
        let prev = self
            .out
            .iter()
            .rev()
            .find(|t| t.kind != TokenKind::Comment && t.kind != TokenKind::Newline);
        match prev {
            Some(t) if t.kind == TokenKind::Word => true,
            Some(t) if t.kind == TokenKind::Punct => {
                matches!(self.src[t.start], b')' | b']')
            }
            _ => false,
        }
    }

    /// A regex literal, emitted as `punct` bytes: IR §5.1, "Regular-expression
    /// literals are lexed as `punct` runs and never as strings."
    fn ts_regex(&mut self) {
        let start = self.i;
        self.i += 1; // the opening `/`
        let mut in_class = false;
        while self.i < self.src.len() {
            match self.src[self.i] {
                b'\\' => self.i += core::cmp::min(2, self.src.len() - self.i),
                b'[' => {
                    in_class = true;
                    self.i += 1;
                }
                b']' => {
                    in_class = false;
                    self.i += 1;
                }
                b'/' if !in_class => {
                    self.i += 1;
                    break;
                }
                // An unterminated regex cannot span a line; stopping at the
                // terminator keeps a stray `/` from eating the rest of the file.
                b'\n' | b'\r' => break,
                _ => self.i += 1,
            }
        }
        // Flags.
        while self.peek(0).is_some_and(|b| b.is_ascii_alphabetic()) {
            self.i += 1;
        }
        // One `punct` per byte, so that no `word` or `string` can be read out
        // of a regex body by a later pattern match.
        let end = self.i;
        for offset in start..end {
            self.out.push(Token {
                kind: TokenKind::Punct,
                start: offset,
                end: offset + 1,
            });
        }
    }

    fn ts_quoted(&mut self) {
        let start = self.i;
        let q = self.src[self.i];
        self.i += 1;
        let content_start = self.i;
        let quote_buf = [q];
        let (content_end, backslash, _) = self.scan_quoted(&quote_buf);
        self.push(
            TokenKind::Str(StrInfo {
                simple: !backslash,
                content: (content_start, content_end),
            }),
            start,
        );
    }

    /// A template literal. **Never simple**, whatever it contains.
    ///
    /// IR §5.1 says only "A template literal containing `${` is not simple", but
    /// §3.4 rule 5 is wider and governs: a non-simple literal includes
    /// "template literals with no substitution (`` `./x` `` in TypeScript)".
    /// Case T20 requires the substitution-free form to be `unresolvable`, so
    /// the wider rule is the operative one and the flag is set here rather than
    /// patched at each use.
    fn ts_template(&mut self) {
        let start = self.i;
        self.i += 1;
        let content_start = self.i;
        let mut backslash = false;
        let mut interpolated = false;
        while self.i < self.src.len() {
            match self.src[self.i] {
                b'\\' => {
                    backslash = true;
                    self.i += core::cmp::min(2, self.src.len() - self.i);
                }
                b'$' if self.peek(1) == Some(b'{') => {
                    interpolated = true;
                    self.i += 2;
                }
                b'`' => break,
                _ => self.i += 1,
            }
        }
        let content_end = self.i;
        if self.i < self.src.len() {
            self.i += 1;
        }
        let _ = (backslash, interpolated);
        self.push(
            TokenKind::Str(StrInfo {
                simple: false,
                content: (content_start, content_end),
            }),
            start,
        );
    }
}

// ---------------------------------------------------------------------------
// Dart — IR §6.1
// ---------------------------------------------------------------------------

impl Lexer<'_> {
    fn run_dart(&mut self) {
        while self.i < self.src.len() {
            let b = self.src[self.i];
            match b {
                b' ' | b'\t' | 0x0C => self.i += 1,
                b'\n' | b'\r' => {
                    self.eat_line_terminator();
                }
                b'/' if self.peek(1) == Some(b'/') => self.line_comment(),
                // "(**nested**, unlike C — Dart's block comments nest, and a
                // lexer that does not nest them mis-lexes a commented-out block
                // containing `*/`)". Case D11.
                b'/' if self.peek(1) == Some(b'*') => self.block_comment(true),
                b'\'' | b'"' => self.dart_string(self.i, false),
                b'r' if matches!(self.peek(1), Some(b'\'') | Some(b'"')) => {
                    let start = self.i;
                    self.i += 1;
                    self.dart_string(start, true);
                }
                b if is_word_byte(b) => {
                    let (start, end) = self.word();
                    self.out.push(Token {
                        kind: TokenKind::Word,
                        start,
                        end,
                    });
                }
                _ => self.punct(),
            }
        }
    }

    /// IR §6.1: "A literal containing `$` followed by `{` or an identifier
    /// character is interpolated and not simple; a raw (`r`) literal with no
    /// interpolation and no backslash is simple."
    fn dart_string(&mut self, start: usize, raw: bool) {
        let q = self.src[self.i];
        let triple = self.peek(1) == Some(q) && self.peek(2) == Some(q);
        let quote_buf = [q, q, q];
        let quote: &[u8] = if triple { &quote_buf } else { &quote_buf[..1] };
        self.i += quote.len();
        let content_start = self.i;
        let (content_end, backslash, _) = self.scan_quoted_raw(quote, raw);
        let body = &self.src[content_start..content_end];
        // A raw literal interpolates nothing either: `r'$x'` is three
        // characters, not a reference to `x`.
        let interpolated = !raw
            && body.windows(2).any(|w| {
                w[0] == b'$' && (w[1] == b'{' || w[1].is_ascii_alphabetic() || w[1] == b'_')
            });
        self.push(
            TokenKind::Str(StrInfo {
                simple: !backslash && !interpolated,
                content: (content_start, content_end),
            }),
            start,
        );
    }
}

// ---------------------------------------------------------------------------
// Swift — IR §7.1
// ---------------------------------------------------------------------------

impl Lexer<'_> {
    fn run_swift(&mut self) {
        while self.i < self.src.len() {
            let b = self.src[self.i];
            match b {
                b' ' | b'\t' | 0x0C => self.i += 1,
                b'\n' | b'\r' => {
                    self.eat_line_terminator();
                }
                b'/' if self.peek(1) == Some(b'/') => self.line_comment(),
                b'/' if self.peek(1) == Some(b'*') => self.block_comment(true),
                b'"' => self.swift_string(self.i, 0),
                // Extended delimiters `#"…"#`, `##"…"##`. The hash run must be
                // followed by a quote; otherwise `#` is ordinary punctuation —
                // `#if`, `#elseif`, `#else` are punct plus word.
                b'#' => {
                    let start = self.i;
                    let mut hashes = 0usize;
                    while self.peek(hashes) == Some(b'#') {
                        hashes += 1;
                    }
                    if self.peek(hashes) == Some(b'"') {
                        self.i += hashes;
                        self.swift_string(start, hashes);
                    } else {
                        self.punct();
                    }
                }
                b if is_word_byte(b) => {
                    let (start, end) = self.word();
                    self.out.push(Token {
                        kind: TokenKind::Word,
                        start,
                        end,
                    });
                }
                _ => self.punct(),
            }
        }
    }

    /// IR §7.1: "Interpolation is `\(` (or `#\(` at matching delimiter depth).
    /// **No import specifier is a string in Swift**, so simplicity does not
    /// arise; the rules exist only to lex correctly."
    fn swift_string(&mut self, start: usize, hashes: usize) {
        let triple = self.peek(1) == Some(b'"') && self.peek(2) == Some(b'"');
        let mut close: Vec<u8> = if triple {
            vec![b'"', b'"', b'"']
        } else {
            vec![b'"']
        };
        close.extend(core::iter::repeat_n(b'#', hashes));
        self.i += if triple { 3 } else { 1 };
        let content_start = self.i;

        // With `n` extended delimiters the escape is `\` followed by `n`
        // hashes; a bare `\` is then an ordinary byte, which is what stops
        // `#"a\"#` from being read as an escaped quote and swallowing the rest
        // of the file.
        //
        // **DEFECT (IR §7.1).** The spec writes the extended-delimiter
        // interpolation as `#\(`, hash before backslash. Swift spells it
        // `\#(` — the hashes follow the backslash — and `#\(` is an ordinary
        // two-byte sequence inside such a literal. Nothing in v1 turns on it
        // (§7.1's own next sentence: "No import specifier is a string in
        // Swift, so simplicity does not arise"), so the real rule is
        // implemented and the spelling is reported rather than followed.
        let escape_run = vec![b'#'; hashes];
        let mut backslash = false;
        loop {
            if self.i >= self.src.len() {
                break;
            }
            if self.src[self.i..].starts_with(&close) {
                break;
            }
            if self.src[self.i] == b'\\' && self.src[self.i + 1..].starts_with(&escape_run) {
                backslash = true;
                self.i += core::cmp::min(2 + hashes, self.src.len() - self.i);
                continue;
            }
            self.i += 1;
        }
        let content_end = self.i;
        if self.i < self.src.len() {
            self.i += close.len();
        }
        self.push(
            TokenKind::Str(StrInfo {
                simple: !backslash,
                content: (content_start, content_end),
            }),
            start,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str, lang: Lang) -> Vec<(TokenKind, String)> {
        lex(src, lang)
            .into_iter()
            .map(|t| (t.kind.clone(), t.text(src).to_string()))
            .collect()
    }

    fn comments(src: &str, lang: Lang) -> Vec<String> {
        lex(src, lang)
            .into_iter()
            .filter(|t| t.kind == TokenKind::Comment)
            .map(|t| t.text(src).to_string())
            .collect()
    }

    /// IR §3.4 rule 1: no encoding declaration is honoured, and a file that is
    /// not valid UTF-8 is not lexed at all (case C17).
    #[test]
    fn a_file_that_is_not_utf8_is_refused_and_never_decoded_lossily() {
        assert_eq!(decode(b"# ok\n").unwrap(), "# ok\n");
        assert_eq!(decode(b"# \xff\n").unwrap_err(), FileNotUtf8);
    }

    /// IR §3.4 rule 1: "A leading UTF-8 BOM (`EF BB BF`) is skipped and is not
    /// part of the first token."
    #[test]
    fn a_leading_bom_is_skipped_and_is_not_part_of_the_first_token() {
        let src = "\u{feff}import a";
        let tokens = lex(src, Lang::Python);
        assert_eq!(tokens[0].start, 3);
        assert_eq!(tokens[0].text(src), "import");
    }

    /// IR §3.4 rule 3: "`word` (a maximal run of `[A-Za-z0-9_$]`, plus `.`
    /// never — `.` is punctuation)".
    #[test]
    fn a_dot_is_punctuation_and_never_part_of_a_word() {
        let src = "import a.b.c";
        let texts: Vec<String> = kinds(src, Lang::Python)
            .into_iter()
            .filter(|(k, _)| *k != TokenKind::Newline)
            .map(|(_, t)| t)
            .collect();
        assert_eq!(texts, ["import", "a", ".", "b", ".", "c"]);
    }

    /// Case P13: `# import a.b` yields no import site — because it yields no
    /// `word` token at all.
    #[test]
    fn a_python_comment_yields_one_comment_token_and_no_words() {
        let src = "# import a.b\n";
        assert_eq!(comments(src, Lang::Python), ["# import a.b"]);
        assert!(!lex(src, Lang::Python).iter().any(|t| t.kind == TokenKind::Word));
    }

    /// Case P14: `x = "import a.b"` yields no site — the import bytes are
    /// inside a `string` token.
    #[test]
    fn python_import_bytes_inside_a_string_are_one_string_token() {
        let src = "x = \"import a.b\"";
        let tokens = lex(src, Lang::Python);
        let strings: Vec<&Token> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Str(_)))
            .collect();
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].text(src), "\"import a.b\"");
        assert!(!tokens.iter().any(|t| t.is_word(src, "import")));
    }

    /// Case J2 and IR §12.1: "Docstrings are **not** comments and are not
    /// scanned — a `@verifies` in a Python docstring does not count, because a
    /// docstring is a string literal and the resolver's lexer classifies it as
    /// one."
    #[test]
    fn a_python_docstring_lexes_as_a_string_and_never_as_a_comment() {
        let src = "\"\"\"@verifies INT-042/AC-1\"\"\"\n";
        assert!(comments(src, Lang::Python).is_empty());
        let tokens = lex(src, Lang::Python);
        assert!(matches!(tokens[0].kind, TokenKind::Str(_)));
        assert_eq!(tokens[0].text(src), src.trim_end());
    }

    /// IR §4.1's prefixes: a prefixed literal is one token, and only an empty,
    /// `r` or `u` prefix with no backslash is simple.
    #[test]
    fn python_string_prefixes_are_part_of_the_token_and_decide_simplicity() {
        for (src, simple) in [
            ("'pack'", true),
            ("r'pack'", true),
            ("U'pack'", true),
            ("b'pack'", false),
            ("f'pack{x}'", false),
            ("RB'pack'", false),
            ("'pa\\ck'", false),
        ] {
            let tokens = lex(src, Lang::Python);
            let TokenKind::Str(info) = &tokens[0].kind else {
                panic!("{src} did not lex as a string: {tokens:?}");
            };
            assert_eq!(info.simple, simple, "{src}");
            assert_eq!(tokens[0].text(src), src, "{src} must be one token");
        }
    }

    /// IR §4.1: "a logical line ends at a newline that is not inside
    /// `(`/`[`/`{` and is not preceded by a backslash continuation."
    /// Case P12: the parenthesized multi-line form is **one** logical line.
    #[test]
    fn a_newline_inside_brackets_or_after_a_continuation_is_not_a_logical_line_end() {
        let inside = "from a import (\n  b,\n  c,\n)\n";
        let newlines = lex(inside, Lang::Python)
            .iter()
            .filter(|t| t.kind == TokenKind::Newline)
            .count();
        assert_eq!(newlines, 1, "one logical line: {inside:?}");

        let continued = "import \\\n  a.b\n";
        let newlines = lex(continued, Lang::Python)
            .iter()
            .filter(|t| t.kind == TokenKind::Newline)
            .count();
        assert_eq!(newlines, 1);

        let two = "import os\nimport sys\n";
        let newlines = lex(two, Lang::Python)
            .iter()
            .filter(|t| t.kind == TokenKind::Newline)
            .count();
        assert_eq!(newlines, 2);
    }

    /// IR §3.4 rule 2: "LF, CRLF and CR each terminate a line."
    #[test]
    fn lf_crlf_and_cr_each_end_a_python_logical_line() {
        for src in ["a\nb\n", "a\r\nb\r\n", "a\rb\r"] {
            let newlines = lex(src, Lang::Python)
                .iter()
                .filter(|t| t.kind == TokenKind::Newline)
                .count();
            assert_eq!(newlines, 2, "{src:?}");
        }
    }

    /// Case T22: "A `//` inside a regex literal, followed on a later line by a
    /// real import | the import is found; the regex did not open a comment."
    #[test]
    fn a_double_slash_inside_a_ts_regex_does_not_open_a_comment() {
        let src = "const re = /a\\/\\/b/;\nimport { x } from './x';\n";
        assert!(comments(src, Lang::Ts).is_empty(), "{:?}", comments(src, Lang::Ts));
        let tokens = lex(src, Lang::Ts);
        assert!(tokens.iter().any(|t| t.is_word(src, "import")));
        assert!(tokens.iter().any(|t| t.simple_literal(src) == Some("./x")));
    }

    /// IR §5.1's division rule: "a `/` that follows a `word`, `)`, `]` or a
    /// numeric literal is division, otherwise it opens a regex."
    #[test]
    fn a_slash_after_a_word_or_a_closing_bracket_is_division_not_a_regex() {
        let src = "const q = a / b;\nimport './y';\n";
        let tokens = lex(src, Lang::Ts);
        assert!(tokens.iter().any(|t| t.simple_literal(src) == Some("./y")));
        assert!(comments(src, Lang::Ts).is_empty());
    }

    /// IR §5.1: "`/* … */` (not nested)" for TypeScript.
    #[test]
    fn a_ts_block_comment_does_not_nest() {
        let src = "/* /* */ import './x';\n";
        assert_eq!(comments(src, Lang::Ts), ["/* /* */"]);
        assert!(lex(src, Lang::Ts).iter().any(|t| t.is_word(src, "import")));
    }

    /// Case D11: "A nested block comment `/* /* */ */` followed by an import |
    /// the import is found."
    #[test]
    fn a_dart_block_comment_nests_so_the_import_after_it_is_found() {
        let src = "/* /* */ */ import 'x.dart';\n";
        assert_eq!(comments(src, Lang::Dart), ["/* /* */ */"]);
        let tokens = lex(src, Lang::Dart);
        assert!(tokens.iter().any(|t| t.is_word(src, "import")));
        assert!(tokens.iter().any(|t| t.simple_literal(src) == Some("x.dart")));
    }

    /// IR §6.1: "A literal containing `$` followed by `{` or an identifier
    /// character is interpolated and not simple."
    #[test]
    fn a_dart_interpolated_literal_is_not_simple() {
        for (src, simple) in [
            ("'package:a/b.dart'", true),
            ("r'package:a/b.dart'", true),
            ("'package:$name/b.dart'", false),
            ("'package:${n}/b.dart'", false),
            ("'a\\tb'", false),
            ("'costs \\$5'", false),
        ] {
            let tokens = lex(src, Lang::Dart);
            let TokenKind::Str(info) = &tokens[0].kind else {
                panic!("{src}: {tokens:?}");
            };
            assert_eq!(info.simple, simple, "{src}");
        }
    }

    /// IR §7.1's extended delimiters. A `\` inside `#"…"#` is an ordinary byte,
    /// so the literal must not end early.
    #[test]
    fn swift_extended_delimiters_close_only_on_a_matching_hash_run() {
        let src = "let s = #\"a\\\"b\"#\nimport XCTest\n";
        let tokens = lex(src, Lang::Swift);
        let strings: Vec<&Token> = tokens
            .iter()
            .filter(|t| matches!(t.kind, TokenKind::Str(_)))
            .collect();
        assert_eq!(strings.len(), 1, "{tokens:?}");
        assert_eq!(strings[0].text(src), "#\"a\\\"b\"#");
        assert!(tokens.iter().any(|t| t.is_word(src, "XCTest")));
    }

    /// IR §7.1: `/* … */` nests in Swift too.
    #[test]
    fn a_swift_block_comment_nests() {
        let src = "/* /* */ */ import XCTest\n";
        assert_eq!(comments(src, Lang::Swift), ["/* /* */ */"]);
        assert!(lex(src, Lang::Swift).iter().any(|t| t.is_word(src, "XCTest")));
    }

    /// Swift's `#if` is punctuation plus a word, never a string opener — the
    /// hash run only starts a literal when a quote follows it.
    #[test]
    fn a_swift_hash_directive_is_not_an_extended_string_delimiter() {
        let src = "#if os(Linux)\nimport A\n#else\nimport B\n#endif\n";
        let tokens = lex(src, Lang::Swift);
        assert!(!tokens.iter().any(|t| matches!(t.kind, TokenKind::Str(_))));
        // Case S4: both branches are there for the site rule to find.
        assert!(tokens.iter().any(|t| t.is_word(src, "A")));
        assert!(tokens.iter().any(|t| t.is_word(src, "B")));
    }

    /// IR §3.4 rule 4: comments are discarded **after** the pragma scan. This
    /// is the discard.
    #[test]
    fn without_comments_removes_exactly_the_comment_tokens() {
        let src = "# c\nimport a\n";
        let all = lex(src, Lang::Python);
        let stripped = without_comments(all.clone());
        assert_eq!(all.len(), stripped.len() + 1);
        assert!(!stripped.iter().any(|t| t.kind == TokenKind::Comment));
    }

    /// An unterminated construct must not panic and must not run past the end
    /// of the file: a branch controls these bytes.
    #[test]
    fn unterminated_constructs_terminate_at_end_of_file() {
        for (src, lang) in [
            ("'unterminated", Lang::Python),
            ("\"\"\"unterminated", Lang::Python),
            ("/* unterminated", Lang::Ts),
            ("`unterminated", Lang::Ts),
            ("/* /* unterminated", Lang::Dart),
            ("#\"unterminated", Lang::Swift),
            ("x = a \\", Lang::Python),
        ] {
            let tokens = lex(src, lang);
            assert!(
                tokens.last().is_none_or(|t| t.end <= src.len()),
                "{src:?} ran past the end"
            );
        }
    }
}
