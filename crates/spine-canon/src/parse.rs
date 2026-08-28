//! A strict JSON reader for spine's value profile.
//!
//! This parser reads *untrusted* bytes — the result file is written inside the
//! collector's boundary by code the candidate controls (RF §9), and a gate
//! report handed to `--verify` came from wherever the operator found it. So it
//! refuses rather than repairs, and it is bounded.
//!
//! `gate-report.md` §2.2 fixes what it accepts:
//!
//! - **Numbers**: "Integers only, `0 <= n <= 2^53 - 1`. No sign, no leading
//!   zero, no fraction, no exponent, no `-0`."
//! - **Duplicate names**: "Invalid. A parser that meets one refuses the
//!   document."
//! - **Depth**: "Bounded by this document's schema; no recursion."

use crate::value::Value;

/// GR §2.2 bounds depth by schema. No spine artifact nests beyond a handful of
/// levels; this cap exists so a hostile result file cannot exhaust the stack of
/// the trusted stage that ingests it.
pub const MAX_DEPTH: usize = 32;

/// GR §2.2's integer ceiling, `2^53 - 1`.
pub const MAX_INT: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub at: usize,
    pub kind: ParseErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// Input ended inside a value.
    UnexpectedEof,
    /// A byte that cannot start or continue the value being read.
    Unexpected(char),
    /// Bytes remained after the top-level value. A file holding a report
    /// "contains exactly the canonical bytes and nothing else" (GR §2.1).
    TrailingBytes,
    /// GR §2.2: "Duplicate names | Invalid."
    DuplicateMember(String),
    /// A `-`, `.` or `e` in a number, or a leading zero.
    NonIntegerNumber,
    /// An integer above `2^53 - 1`.
    IntegerOutOfRange,
    /// Nesting past [`MAX_DEPTH`].
    TooDeep,
    /// An unescaped control character inside a string, or a bad `\` escape, or
    /// a lone surrogate in a `\u` pair.
    BadString(&'static str),
    /// The input was not valid UTF-8.
    NotUtf8,
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "at byte {}: ", self.at)?;
        match &self.kind {
            ParseErrorKind::UnexpectedEof => write!(f, "unexpected end of input"),
            ParseErrorKind::Unexpected(c) => write!(f, "unexpected {c:?}"),
            ParseErrorKind::TrailingBytes => write!(f, "trailing bytes after the top-level value"),
            ParseErrorKind::DuplicateMember(name) => write!(f, "duplicate member name {name:?}"),
            ParseErrorKind::NonIntegerNumber => {
                write!(
                    f,
                    "not an integer: no sign, leading zero, fraction or exponent is permitted"
                )
            }
            ParseErrorKind::IntegerOutOfRange => write!(f, "integer exceeds 2^53 - 1"),
            ParseErrorKind::TooDeep => write!(f, "nesting deeper than {MAX_DEPTH}"),
            ParseErrorKind::BadString(why) => write!(f, "invalid string: {why}"),
            ParseErrorKind::NotUtf8 => write!(f, "input is not valid UTF-8"),
        }
    }
}

impl core::error::Error for ParseError {}

/// Parse one complete JSON value and refuse anything after it.
pub fn parse(bytes: &[u8]) -> Result<Value, ParseError> {
    let text = core::str::from_utf8(bytes).map_err(|e| ParseError {
        at: e.valid_up_to(),
        kind: ParseErrorKind::NotUtf8,
    })?;

    let mut p = Parser {
        bytes: text.as_bytes(),
        pos: 0,
        depth: 0,
    };
    p.skip_ws();
    let value = p.value()?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(p.err(ParseErrorKind::TrailingBytes));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, kind: ParseErrorKind) -> ParseError {
        ParseError { at: self.pos, kind }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    /// JSON's whitespace, and only JSON's: space, tab, LF, CR.
    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), ParseError> {
        match self.peek() {
            Some(b) if b == byte => {
                self.pos += 1;
                Ok(())
            }
            Some(b) => Err(self.err(ParseErrorKind::Unexpected(b as char))),
            None => Err(self.err(ParseErrorKind::UnexpectedEof)),
        }
    }

    fn value(&mut self) -> Result<Value, ParseError> {
        match self
            .peek()
            .ok_or_else(|| self.err(ParseErrorKind::UnexpectedEof))?
        {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => Ok(Value::Str(self.string()?)),
            b't' => self.literal("true", Value::Bool(true)),
            b'f' => self.literal("false", Value::Bool(false)),
            b'n' => self.literal("null", Value::Null),
            b'0'..=b'9' | b'-' => self.number(),
            b => Err(self.err(ParseErrorKind::Unexpected(b as char))),
        }
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value, ParseError> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(self.err(ParseErrorKind::Unexpected(self.bytes[self.pos] as char)))
        }
    }

    fn object(&mut self) -> Result<Value, ParseError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.err(ParseErrorKind::TooDeep));
        }
        self.expect(b'{')?;
        let mut members: Vec<(String, Value)> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            self.depth -= 1;
            return Ok(Value::Obj(members));
        }
        loop {
            self.skip_ws();
            let name_at = self.pos;
            let name = self.string()?;
            // GR §2.2: a parser that meets a duplicate refuses the document.
            // Linear scan: spine objects have a handful of members, and a hash
            // set would make the error's position dependent on iteration order.
            if members.iter().any(|(existing, _)| *existing == name) {
                return Err(ParseError {
                    at: name_at,
                    kind: ParseErrorKind::DuplicateMember(name),
                });
            }
            self.skip_ws();
            self.expect(b':')?;
            self.skip_ws();
            let value = self.value()?;
            members.push((name, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    self.depth -= 1;
                    return Ok(Value::Obj(members));
                }
                Some(b) => return Err(self.err(ParseErrorKind::Unexpected(b as char))),
                None => return Err(self.err(ParseErrorKind::UnexpectedEof)),
            }
        }
    }

    fn array(&mut self) -> Result<Value, ParseError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.err(ParseErrorKind::TooDeep));
        }
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            self.depth -= 1;
            return Ok(Value::Arr(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    self.depth -= 1;
                    return Ok(Value::Arr(items));
                }
                Some(b) => return Err(self.err(ParseErrorKind::Unexpected(b as char))),
                None => return Err(self.err(ParseErrorKind::UnexpectedEof)),
            }
        }
    }

    /// GR §2.2: integers only, `0 <= n <= 2^53 - 1`, no leading zero.
    ///
    /// `-` and `.` and `e` are read and *then* refused, rather than refused as
    /// a syntax error, so the message says what the rule is instead of pointing
    /// at a byte.
    fn number(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            return Err(self.err(ParseErrorKind::NonIntegerNumber));
        }
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        let digits = &self.bytes[start..self.pos];
        if digits.is_empty() {
            return Err(self.err(ParseErrorKind::Unexpected(
                self.peek().unwrap_or(b' ') as char
            )));
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err(ParseError {
                at: start,
                kind: ParseErrorKind::NonIntegerNumber,
            });
        }
        if digits.len() > 1 && digits[0] == b'0' {
            return Err(ParseError {
                at: start,
                kind: ParseErrorKind::NonIntegerNumber,
            });
        }
        // Refuse on width before parsing, so a 30-digit run cannot overflow.
        if digits.len() > 16 {
            return Err(ParseError {
                at: start,
                kind: ParseErrorKind::IntegerOutOfRange,
            });
        }
        let mut n: u64 = 0;
        for &d in digits {
            n = n * 10 + u64::from(d - b'0');
        }
        if n > MAX_INT {
            return Err(ParseError {
                at: start,
                kind: ParseErrorKind::IntegerOutOfRange,
            });
        }
        Ok(Value::Int(n))
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let byte = self
                .peek()
                .ok_or_else(|| self.err(ParseErrorKind::UnexpectedEof))?;
            match byte {
                b'"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.pos += 1;
                    let esc = self
                        .peek()
                        .ok_or_else(|| self.err(ParseErrorKind::UnexpectedEof))?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000C}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => out.push(self.unicode_escape()?),
                        _ => {
                            return Err(self.err(ParseErrorKind::BadString("unknown escape")));
                        }
                    }
                }
                // RFC 8259 §7: control characters must be escaped.
                0x00..=0x1F => {
                    return Err(self.err(ParseErrorKind::BadString("unescaped control character")));
                }
                _ => {
                    // The input is known-UTF-8, so step by whole characters.
                    let rest = core::str::from_utf8(&self.bytes[self.pos..])
                        .expect("input validated as UTF-8 in parse()");
                    let ch = rest.chars().next().expect("non-empty");
                    out.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    fn unicode_escape(&mut self) -> Result<char, ParseError> {
        let first = self.hex4()?;
        // Surrogates are only legal as a well-formed pair.
        if (0xD800..0xDC00).contains(&first) {
            if self.peek() != Some(b'\\') {
                return Err(self.err(ParseErrorKind::BadString("lone high surrogate")));
            }
            self.pos += 1;
            if self.peek() != Some(b'u') {
                return Err(self.err(ParseErrorKind::BadString("lone high surrogate")));
            }
            self.pos += 1;
            let second = self.hex4()?;
            if !(0xDC00..0xE000).contains(&second) {
                return Err(self.err(ParseErrorKind::BadString("bad low surrogate")));
            }
            let combined =
                0x10000u32 + ((u32::from(first) - 0xD800) << 10) + (u32::from(second) - 0xDC00);
            return char::from_u32(combined)
                .ok_or_else(|| self.err(ParseErrorKind::BadString("bad surrogate pair")));
        }
        if (0xDC00..0xE000).contains(&first) {
            return Err(self.err(ParseErrorKind::BadString("lone low surrogate")));
        }
        char::from_u32(u32::from(first))
            .ok_or_else(|| self.err(ParseErrorKind::BadString("not a scalar value")))
    }

    fn hex4(&mut self) -> Result<u16, ParseError> {
        let mut n: u16 = 0;
        for _ in 0..4 {
            let b = self
                .peek()
                .ok_or_else(|| self.err(ParseErrorKind::UnexpectedEof))?;
            let digit = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return Err(self.err(ParseErrorKind::BadString("bad \\u escape"))),
            };
            n = (n << 4) | u16::from(digit);
            self.pos += 1;
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jcs::canonicalize_to_string;

    #[test]
    fn round_trips_the_gr_8_3_vector() {
        let parsed = parse(br#"{"b":[1,2],"a":"x\\y","Z":true,"_c":{"n":0,"m":"q\"r"}}"#).unwrap();
        assert_eq!(
            canonicalize_to_string(&parsed),
            r#"{"Z":true,"_c":{"m":"q\"r","n":0},"a":"x\\y","b":[1,2]}"#
        );
    }

    #[test]
    fn canonicalizing_is_idempotent() {
        let source = br#" { "b" : [ 1 , 2 ] , "a" : "x" } "#;
        let once = canonicalize_to_string(&parse(source).unwrap());
        let twice = canonicalize_to_string(&parse(once.as_bytes()).unwrap());
        assert_eq!(once, twice);
    }

    /// GR §2.2: "Duplicate names | Invalid. A parser that meets one refuses the
    /// document." Silently taking the last is what most JSON libraries do, and
    /// it would let one result file mean two things.
    #[test]
    fn duplicate_member_names_are_refused() {
        let err = parse(br#"{"a":1,"a":2}"#).unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::DuplicateMember("a".into()));
    }

    #[test]
    fn non_integers_are_refused() {
        for source in [
            &b"1.5"[..],
            b"-1",
            b"1e3",
            b"1E3",
            b"01",
            b"-0",
            b"[1.0]",
            br#"{"a":0.0}"#,
        ] {
            let err = parse(source).unwrap_err();
            assert_eq!(
                err.kind,
                ParseErrorKind::NonIntegerNumber,
                "expected a non-integer refusal for {:?}",
                core::str::from_utf8(source).unwrap()
            );
        }
    }

    #[test]
    fn integers_are_bounded_at_2_pow_53_minus_1() {
        assert_eq!(parse(b"9007199254740991").unwrap(), Value::Int(MAX_INT));
        assert_eq!(
            parse(b"9007199254740992").unwrap_err().kind,
            ParseErrorKind::IntegerOutOfRange
        );
        // A digit run long enough to overflow u64 is refused on width, before
        // any arithmetic runs.
        assert_eq!(
            parse(b"999999999999999999999999999999").unwrap_err().kind,
            ParseErrorKind::IntegerOutOfRange
        );
    }

    #[test]
    fn nesting_is_bounded() {
        let deep = format!("{}{}", "[".repeat(MAX_DEPTH + 1), "]".repeat(MAX_DEPTH + 1));
        assert_eq!(
            parse(deep.as_bytes()).unwrap_err().kind,
            ParseErrorKind::TooDeep
        );
        let ok = format!("{}{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
        assert!(parse(ok.as_bytes()).is_ok());
    }

    #[test]
    fn trailing_bytes_are_refused() {
        assert_eq!(
            parse(br#"{"a":1} {"b":2}"#).unwrap_err().kind,
            ParseErrorKind::TrailingBytes
        );
        // A trailing newline counts as whitespace, not as trailing bytes — but
        // a canonical report file has none (GR §2.1).
        assert!(parse(b"{}\n").is_ok());
    }

    #[test]
    fn strings_refuse_what_json_refuses() {
        assert!(matches!(
            parse(b"\"a\nb\"").unwrap_err().kind,
            ParseErrorKind::BadString("unescaped control character")
        ));
        assert!(matches!(
            parse(br#""\ud800""#).unwrap_err().kind,
            ParseErrorKind::BadString("lone high surrogate")
        ));
        assert!(matches!(
            parse(br#""\udc00""#).unwrap_err().kind,
            ParseErrorKind::BadString("lone low surrogate")
        ));
        // A well-formed pair is the one way past the BMP by escape, and the
        // literal UTF-8 of the same scalar must reach the same value.
        assert_eq!(
            parse(br#""\ud83d\ude00""#).unwrap(),
            Value::Str("\u{1F600}".into())
        );
        assert_eq!(
            parse("\"\u{1F600}\"".as_bytes()).unwrap(),
            Value::Str("\u{1F600}".into())
        );
    }

    #[test]
    fn invalid_utf8_is_refused_before_anything_else() {
        assert_eq!(
            parse(b"\"\xff\"").unwrap_err().kind,
            ParseErrorKind::NotUtf8
        );
    }
}
