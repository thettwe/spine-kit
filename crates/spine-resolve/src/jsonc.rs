//! JSON with comments and trailing commas — "the dialect `tsc` accepts"
//! (IR §5.3 step 1).
//!
//! This is deliberately **not** `spine-canon`'s `parse`: that one is strict JSON
//! by design (it refuses a duplicate member and admits only integers), and a
//! `tsconfig.json` legitimately carries `//` comments, trailing commas and
//! floats. Canonical JSON is what spine *writes*; this is what a repository
//! *has*.
//!
//! Only what IR §5.3 reads is modelled. "No other key is read" — `include`,
//! `exclude`, `files`, `references` and `moduleResolution` are ignored — so
//! numbers, booleans and nulls parse to [`Json::Other`] and carry no value. A
//! parser that decoded them would have to decide a float's spelling, which is
//! the kind of thing two implementations disagree about for free.

/// The value model IR §5.3 needs: strings, arrays, objects, and "something
/// else".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Json {
    Str(String),
    Arr(Vec<Json>),
    /// Members in file order. IR §5.3: "`paths` is a list of `(pattern,
    /// [substitution, …])` **in the file's own key order**", so the order is
    /// load-bearing and a map would destroy it.
    Obj(Vec<(String, Json)>),
    /// A number, `true`, `false` or `null` — parsed, and never read.
    Other,
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
}

/// Parse the JSONC dialect. `None` is IR §5.3's `tsconfig-unparseable`.
pub fn parse(source: &str) -> Option<Json> {
    let bytes = source.as_bytes();
    let mut p = Parser { bytes, i: 0 };
    // A leading BOM is skipped for the same reason IR §3.4 rule 1 skips one.
    if bytes.starts_with(b"\xEF\xBB\xBF") {
        p.i = 3;
    }
    let value = p.value()?;
    p.trivia();
    if p.i == bytes.len() {
        Some(value)
    } else {
        None
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    /// Whitespace, `//` comments and `/* */` comments — the three things that
    /// separate JSONC from JSON on the way in.
    fn trivia(&mut self) {
        loop {
            while self
                .bytes
                .get(self.i)
                .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
            {
                self.i += 1;
            }
            if self.bytes[self.i..].starts_with(b"//") {
                while self
                    .bytes
                    .get(self.i)
                    .is_some_and(|b| *b != b'\n' && *b != b'\r')
                {
                    self.i += 1;
                }
                continue;
            }
            if self.bytes[self.i..].starts_with(b"/*") {
                self.i += 2;
                while self.i < self.bytes.len() && !self.bytes[self.i..].starts_with(b"*/") {
                    self.i += 1;
                }
                // An unterminated block comment consumes the rest of the file,
                // which then fails the "consumed everything" check in `parse`.
                self.i = core::cmp::min(self.i + 2, self.bytes.len());
                continue;
            }
            return;
        }
    }

    fn eat(&mut self, b: u8) -> bool {
        if self.bytes.get(self.i) == Some(&b) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Option<Json> {
        self.trivia();
        match self.bytes.get(self.i)? {
            b'"' => self.string().map(Json::Str),
            b'[' => self.array(),
            b'{' => self.object(),
            _ => self.scalar(),
        }
    }

    fn array(&mut self) -> Option<Json> {
        self.i += 1;
        let mut items = Vec::new();
        loop {
            self.trivia();
            if self.eat(b']') {
                return Some(Json::Arr(items));
            }
            items.push(self.value()?);
            self.trivia();
            if self.eat(b',') {
                continue;
            }
            self.trivia();
            if self.eat(b']') {
                return Some(Json::Arr(items));
            }
            return None;
        }
    }

    fn object(&mut self) -> Option<Json> {
        self.i += 1;
        let mut members: Vec<(String, Json)> = Vec::new();
        loop {
            self.trivia();
            if self.eat(b'}') {
                return Some(Json::Obj(members));
            }
            let key = self.string()?;
            self.trivia();
            if !self.eat(b':') {
                return None;
            }
            let value = self.value()?;
            // A duplicate key is *not* refused here, unlike `spine-canon`'s
            // strict parser: `tsc` takes the last one and this file is the
            // repository's, not spine's. Last wins, so pushing and letting the
            // lookup find the first would be wrong — replace instead.
            match members.iter().position(|(k, _)| *k == key) {
                Some(i) => members[i].1 = value,
                None => members.push((key, value)),
            }
            self.trivia();
            if self.eat(b',') {
                continue;
            }
            self.trivia();
            if self.eat(b'}') {
                return Some(Json::Obj(members));
            }
            return None;
        }
    }

    fn string(&mut self) -> Option<String> {
        self.trivia();
        if !self.eat(b'"') {
            return None;
        }
        let mut out = String::new();
        loop {
            let b = *self.bytes.get(self.i)?;
            match b {
                b'"' => {
                    self.i += 1;
                    return Some(out);
                }
                b'\\' => {
                    self.i += 1;
                    let esc = *self.bytes.get(self.i)?;
                    self.i += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hex = self.bytes.get(self.i..self.i + 4)?;
                            let code =
                                u32::from_str_radix(core::str::from_utf8(hex).ok()?, 16).ok()?;
                            self.i += 4;
                            // A lone surrogate is not a character. Refusing is
                            // the fail-closed answer and matches the rest of
                            // this crate's habit of refusing rather than
                            // substituting U+FFFD.
                            out.push(char::from_u32(code)?);
                        }
                        _ => return None,
                    }
                }
                _ => {
                    // Copy whole UTF-8 characters so the output stays valid.
                    let rest = core::str::from_utf8(&self.bytes[self.i..]).ok()?;
                    let ch = rest.chars().next()?;
                    out.push(ch);
                    self.i += ch.len_utf8();
                }
            }
        }
    }

    /// A number, `true`, `false` or `null`. Consumed and discarded.
    fn scalar(&mut self) -> Option<Json> {
        let start = self.i;
        while self
            .bytes
            .get(self.i)
            .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'+' | b'.' | b'_'))
        {
            self.i += 1;
        }
        if self.i == start {
            None
        } else {
            Some(Json::Other)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// IR §5.3 step 1's dialect: "JSON with comments and trailing commas (the
    /// dialect `tsc` accepts)".
    #[test]
    fn comments_and_trailing_commas_parse() {
        let source = r#"{
  // a line comment
  "compilerOptions": {
    /* a block comment */
    "baseUrl": ".",
    "paths": { "@shared/*": ["src/shared/*"], },
  },
}"#;
        let json = parse(source).expect("should parse");
        let options = json.get("compilerOptions").unwrap();
        assert_eq!(options.get("baseUrl").unwrap().as_str(), Some("."));
        assert_eq!(
            options.get("paths").unwrap().get("@shared/*"),
            Some(&Json::Arr(vec![Json::Str("src/shared/*".into())]))
        );
    }

    /// "in the file's own key order" — IR §5.3 makes `paths` an ordered list,
    /// so the parse must not sort or hash them.
    #[test]
    fn object_members_keep_the_files_own_key_order() {
        let json = parse(r#"{"z":"1","a":"2","m":"3"}"#).unwrap();
        let Json::Obj(members) = json else { panic!() };
        let keys: Vec<&str> = members.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["z", "a", "m"]);
    }

    /// A file that does not parse in the dialect is `tsconfig-unparseable`.
    #[test]
    fn malformed_input_is_refused_rather_than_recovered() {
        for source in [
            "{",
            "{\"a\"}",
            "{\"a\": }",
            "{} trailing",
            "{\"a\": \"b\" \"c\": \"d\"}",
            "/* unterminated",
        ] {
            assert_eq!(parse(source), None, "{source:?}");
        }
    }

    /// Numbers, booleans and nulls parse and carry nothing — "No other key is
    /// read."
    #[test]
    fn unread_scalars_parse_to_other() {
        let json = parse(r#"{"strict":true,"n":-1.5e3,"z":null}"#).unwrap();
        assert_eq!(json.get("strict"), Some(&Json::Other));
        assert_eq!(json.get("n"), Some(&Json::Other));
        assert_eq!(json.get("z"), Some(&Json::Other));
    }

    /// `tsc` takes the last duplicate key; a strict JSON parser would refuse
    /// the file outright, which would make an ordinary tsconfig
    /// `tsconfig-unparseable`.
    #[test]
    fn a_duplicate_key_takes_the_last_value_rather_than_refusing() {
        let json = parse(r#"{"a":"first","a":"second"}"#).unwrap();
        assert_eq!(json.get("a").unwrap().as_str(), Some("second"));
    }

    #[test]
    fn escapes_decode_and_a_lone_surrogate_is_refused() {
        assert_eq!(parse(r#""aA\n\"b""#).unwrap().as_str(), Some("aA\n\"b"));
        assert_eq!(parse(r#""\ud800""#), None);
    }
}
