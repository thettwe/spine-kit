//! `git ls-tree` C-style path quoting — the encoding `Spine-Frozen` uses, and
//! **not** the one a wire token uses.
//!
//! EV §2.5: "Two different path encodings live in one envelope, and both are
//! required. This is not a redundancy and an implementation must not unify
//! them." `tests/fixtures/café.json` is
//! `"tests/fixtures/caf\303\251.json"` in a `Spine-Frozen` line and
//! `G8:tests/fixtures/caf\xc3\xa9.json` inside a `wires=` token, in the same
//! commit message. The second is `spine_canon::tok` and is never re-derived
//! here; this module owns only the first.
//!
//! EV §4.3: the rendering "**must not depend on `core.quotePath`**: spine
//! always emits the quoted form, equivalent to `core.quotePath=true`, because a
//! digest that varied with a local git config would be no digest at all." That
//! is also why the rule is written out rather than shelled out to git.

use crate::refusal::EnvelopeError;

/// EV §4.3: quoted "**iff** it contains at least one byte in `0x00–0x1F`,
/// `0x7F–0xFF`, `"` (`0x22`) or `\` (`0x5C`). Otherwise it is emitted
/// literally, unwrapped."
///
/// "**A space does not trigger quoting.** `tests/fixtures/tax rates.json` is
/// emitted literally, spaces and all" — which is why the payload splits at its
/// *first* space and not at its last.
pub fn needs_quoting(path: &[u8]) -> bool {
    path.iter()
        .any(|&b| b <= 0x1F || b >= 0x7F || b == b'"' || b == b'\\')
}

/// Render a path as `git ls-tree` would with `core.quotePath=true`.
pub fn quote_path(path: &[u8]) -> Vec<u8> {
    if !needs_quoting(path) {
        return path.to_vec();
    }
    let mut out = Vec::with_capacity(path.len() + 2);
    out.push(b'"');
    for &b in path {
        match b {
            // EV §4.3's table, row for row.
            0x07 => out.extend_from_slice(b"\\a"),
            0x08 => out.extend_from_slice(b"\\b"),
            0x09 => out.extend_from_slice(b"\\t"),
            0x0A => out.extend_from_slice(b"\\n"),
            0x0B => out.extend_from_slice(b"\\v"),
            0x0C => out.extend_from_slice(b"\\f"),
            0x0D => out.extend_from_slice(b"\\r"),
            b'"' => out.extend_from_slice(b"\\\""),
            b'\\' => out.extend_from_slice(b"\\\\"),
            b if b <= 0x1F || b >= 0x7F => {
                // "`\` + exactly three octal digits, zero-padded".
                out.push(b'\\');
                out.push(b'0' + (b >> 6));
                out.push(b'0' + ((b >> 3) & 0o7));
                out.push(b'0' + (b & 0o7));
            }
            b => out.push(b),
        }
    }
    out.push(b'"');
    out
}

/// Decode a `Spine-Frozen` path field back to its repository bytes.
///
/// EV §4.3: "**Deciding whether the path field is quoted is exact**: it is
/// quoted iff its first byte is `"`. A real path beginning with `"` contains
/// `"` and is therefore always quoted, so the test can never misfire."
///
/// "A path field that begins with `"` and is not a valid C-quoted string —
/// unterminated, a bad escape, a trailing byte after the closing quote — is
/// `envelope-malformed`."
pub fn unquote_path(field: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
    if field.first() != Some(&b'"') {
        return Ok(field.to_vec());
    }
    let mut out = Vec::with_capacity(field.len());
    let mut i = 1usize;
    loop {
        let Some(&b) = field.get(i) else {
            return Err(EnvelopeError::malformed("unterminated C-quoted path"));
        };
        match b {
            b'"' => {
                i += 1;
                break;
            }
            b'\\' => {
                let Some(&e) = field.get(i + 1) else {
                    return Err(EnvelopeError::malformed("C-quoted path ends in an escape"));
                };
                match e {
                    b'a' => out.push(0x07),
                    b'b' => out.push(0x08),
                    b't' => out.push(0x09),
                    b'n' => out.push(0x0A),
                    b'v' => out.push(0x0B),
                    b'f' => out.push(0x0C),
                    b'r' => out.push(0x0D),
                    b'"' => out.push(b'"'),
                    b'\\' => out.push(b'\\'),
                    b'0'..=b'3' => {
                        // Exactly three octal digits. Two would be ambiguous
                        // against a following literal digit, and the encoder
                        // zero-pads for that reason.
                        let digits = field.get(i + 1..i + 4).ok_or_else(|| {
                            EnvelopeError::malformed("truncated octal escape in C-quoted path")
                        })?;
                        if !digits.iter().all(|d| (b'0'..=b'7').contains(d)) {
                            return Err(EnvelopeError::malformed(
                                "octal escape in C-quoted path is not three octal digits",
                            ));
                        }
                        let v = (digits[0] - b'0') * 64 + (digits[1] - b'0') * 8 + (digits[2] - b'0');
                        out.push(v);
                        i += 2; // plus the 2 added below
                    }
                    other => {
                        return Err(EnvelopeError::malformed(format!(
                            "bad escape \\{} in C-quoted path",
                            other as char
                        )));
                    }
                }
                i += 2;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    if i != field.len() {
        return Err(EnvelopeError::malformed(
            "trailing bytes after the closing quote of a C-quoted path",
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vector_a_path_is_quoted_because_of_its_bytes() {
        // EV §8.2 point 2: `tests/fixtures/café.json` holds 0xC3 0xA9, "so it
        // is wrapped and the two bytes become `\303\251`".
        let path = "tests/fixtures/café.json".as_bytes();
        assert!(path.windows(2).any(|w| w == [0xC3, 0xA9]));
        assert_eq!(
            quote_path(path),
            br#""tests/fixtures/caf\303\251.json""#.to_vec()
        );
        assert_eq!(unquote_path(&quote_path(path)).unwrap(), path);
    }

    #[test]
    fn a_space_does_not_trigger_quoting() {
        // EV §4.3, and vector C's `tests/a b.py`.
        let path = b"tests/fixtures/tax rates.json";
        assert!(!needs_quoting(path));
        assert_eq!(quote_path(path), path.to_vec());
        assert_eq!(unquote_path(path).unwrap(), path.to_vec());
    }

    #[test]
    fn a_c_quoted_path_is_never_the_wire_encoding() {
        // EV §13.9: the same path, two encodings, in one commit message. An
        // implementation that reuses one encoder for both "produces lines no
        // conforming implementation reproduces".
        let path = "tests/fixtures/café.json".as_bytes();
        assert_eq!(
            spine_canon::tok(path),
            "tests/fixtures/caf\\xc3\\xa9.json",
            "the wire token is GR §6.2's tok, lowercase hex, no wrapping quotes"
        );
        assert_ne!(quote_path(path), spine_canon::tok(path).into_bytes());
    }

    #[test]
    fn every_control_byte_round_trips() {
        for b in 0u8..=255 {
            let path = vec![b'a', b, b'b'];
            let quoted = quote_path(&path);
            assert_eq!(unquote_path(&quoted).unwrap(), path, "byte 0x{b:02X}");
        }
    }

    #[test]
    fn the_named_escapes_are_gits_and_not_rusts() {
        assert_eq!(quote_path(&[b'a', 0x07]), br#""a\a""#.to_vec());
        assert_eq!(quote_path(&[b'a', 0x0B]), br#""a\v""#.to_vec());
        // 0x1B has no named escape and takes three zero-padded octal digits.
        assert_eq!(quote_path(&[b'a', 0x1B]), br#""a\033""#.to_vec());
        assert_eq!(quote_path(&[b'a', 0x7F]), br#""a\177""#.to_vec());
        assert_eq!(quote_path(&[b'a', 0xC3]), br#""a\303""#.to_vec());
    }

    #[test]
    fn a_malformed_quoted_path_is_refused_not_repaired() {
        assert!(unquote_path(br#""abc"#).is_err(), "unterminated");
        assert!(unquote_path(br#""a\q""#).is_err(), "bad escape");
        assert!(unquote_path(br#""a\30""#).is_err(), "truncated octal");
        assert!(unquote_path(br#""a"x"#).is_err(), "trailing byte");
    }
}
