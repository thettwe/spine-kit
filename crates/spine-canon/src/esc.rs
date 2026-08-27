//! `esc` and `tok` — how repository bytes become JSON strings and wire tokens.
//!
//! `gate-report.md` §2.3: "Repository paths are byte strings. Git does not
//! require them to be UTF-8, and macOS filesystems disagree with Linux ones
//! about normalization. […] JSON has no byte-string type." So every value that
//! carries repository or human bytes is `esc`-encoded and is thereafter pure
//! ASCII in `U+0020..=U+007E`.
//!
//! Nothing here normalizes: "No NFC, no NFD, no case folding, no separator
//! rewriting" (GR §2.3). That is the reason a report computed on macOS and one
//! computed in a Linux container agree.

/// GR §2.3. For each byte `b`:
///
/// | `b` | emits |
/// |---|---|
/// | `0x5C` (`\`) | the two characters `\` `\` |
/// | `0x20 ..= 0x7E`, other than `0x5C` | the character with that code point |
/// | anything else (`0x00-0x1F`, `0x7F-0xFF`) | `\` `x` and two **lowercase** hex digits |
pub fn esc(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        match b {
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7E => out.push(b as char),
            _ => {
                out.push_str("\\x");
                out.push(hex_lower(b >> 4));
                out.push(hex_lower(b & 0xF));
            }
        }
    }
    out
}

/// GR §6.2. `tok(s)` is `esc(s)` with three bytes moved out of the printable
/// row into the `\xHH` row: `,` (`0x2C`), ` ` (`0x20`), `"` (`0x22`).
///
/// "`tok` is **one pass over the bytes of `s`**, not `esc` composed with a
/// second escaping step: a second pass would re-escape the `\` that the first
/// pass emitted and turn `,` into `\\x2c`." — which is why this is its own loop
/// and not `esc(...)` piped through anything.
///
/// `=` is deliberately **not** escaped: "a trailer field splits on its first
/// `=`, so `wires=G2:src/a=b.ts` parses as the field `wires` with the value
/// `G2:src/a=b.ts`. Three escapes, not four."
pub fn tok(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        match b {
            b'\\' => out.push_str("\\\\"),
            b',' | b' ' | b'"' => {
                out.push_str("\\x");
                out.push(hex_lower(b >> 4));
                out.push(hex_lower(b & 0xF));
            }
            0x20..=0x7E => out.push(b as char),
            _ => {
                out.push_str("\\x");
                out.push(hex_lower(b >> 4));
                out.push(hex_lower(b & 0xF));
            }
        }
    }
    out
}

/// Why a string is not a valid `esc` encoding. GR §2.3: "Any other sequence
/// after `\` is an invalid report."
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnescError {
    /// A `\` at the very end of the input, introducing nothing.
    TrailingBackslash,
    /// `\` followed by something that is neither `\` nor `x`.
    BadEscape { at: usize, found: char },
    /// `\x` not followed by exactly two lowercase hex digits. Uppercase is a
    /// failure, not an alias: `esc` emits lowercase, so uppercase here means
    /// the value was produced by something that is not `esc`.
    BadHex { at: usize },
    /// A byte outside `U+0020..=U+007E` appearing literally. `esc` output is
    /// printable ASCII by construction, so anything else was not `esc` output.
    NotPrintableAscii { at: usize, found: char },
}

impl core::fmt::Display for UnescError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UnescError::TrailingBackslash => write!(f, "trailing backslash"),
            UnescError::BadEscape { at, found } => {
                write!(f, "invalid escape \\{found} at byte {at}")
            }
            UnescError::BadHex { at } => {
                write!(f, "\\x at byte {at} needs two lowercase hex digits")
            }
            UnescError::NotPrintableAscii { at, found } => {
                write!(f, "unencoded character {found:?} at byte {at}")
            }
        }
    }
}

impl core::error::Error for UnescError {}

/// Inverse of [`esc`]. GR §2.3: "Decoding is total and unambiguous: `\`
/// introduces either `\` (one literal backslash) or `x` plus exactly two
/// lowercase hex digits (one byte)."
///
/// Also the inverse of [`tok`] — `tok` differs only in *which* bytes take the
/// `\xHH` form, never in how a `\xHH` form is read.
pub fn unesc(s: &str) -> Result<Vec<u8>, UnescError> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                let next = *bytes.get(i + 1).ok_or(UnescError::TrailingBackslash)?;
                match next {
                    b'\\' => {
                        out.push(b'\\');
                        i += 2;
                    }
                    b'x' => {
                        let hi = bytes.get(i + 2).copied().ok_or(UnescError::BadHex { at: i })?;
                        let lo = bytes.get(i + 3).copied().ok_or(UnescError::BadHex { at: i })?;
                        let hi = from_hex_lower(hi).ok_or(UnescError::BadHex { at: i })?;
                        let lo = from_hex_lower(lo).ok_or(UnescError::BadHex { at: i })?;
                        out.push((hi << 4) | lo);
                        i += 4;
                    }
                    other => {
                        return Err(UnescError::BadEscape {
                            at: i,
                            found: other as char,
                        });
                    }
                }
            }
            b @ 0x20..=0x7E => {
                out.push(b);
                i += 1;
            }
            b => {
                return Err(UnescError::NotPrintableAscii {
                    at: i,
                    found: b as char,
                });
            }
        }
    }
    Ok(out)
}

fn hex_lower(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        _ => (b'a' + (nibble - 10)) as char,
    }
}

/// Lowercase only. GR §2.3 fixes the case of what `esc` *emits*; accepting
/// uppercase on the way back in would make two spellings decode to one byte and
/// break the round-trip property the digests rely on.
fn from_hex_lower(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `gate-report.md` §2.3's worked cases, verbatim.
    #[test]
    fn gr_2_3_worked_cases() {
        assert_eq!(esc(b"src/shared/util.ts"), "src/shared/util.ts");
        assert_eq!(esc(br"a\b"), r"a\\b");
        // `caf` + 0xC3 0xA9 — the UTF-8 for "é", which esc never sees as a
        // character because it never decodes.
        assert_eq!(esc(b"caf\xc3\xa9"), r"caf\xc3\xa9");
        assert_eq!(esc(b"a\"b"), "a\"b");
        assert_eq!(esc(b"a,b"), "a,b");
    }

    /// The comma "is only escaped inside a *wire token*, §6.2".
    #[test]
    fn gr_6_2_tok_escapes_exactly_three_more_bytes_than_esc() {
        assert_eq!(tok(b"a,b"), r"a\x2cb");
        assert_eq!(tok(b"a b"), r"a\x20b");
        assert_eq!(tok(b"a\"b"), r"a\x22b");
        // `=` is deliberately not escaped.
        assert_eq!(tok(b"src/a=b.ts"), "src/a=b.ts");
        // And for a path containing none of the three, tok is esc.
        assert_eq!(tok(b"src/shared/util.ts"), esc(b"src/shared/util.ts"));
    }

    /// "a second pass would re-escape the `\` that the first pass emitted and
    /// turn `,` into `\\x2c`" — the bug this test exists to fail on.
    #[test]
    fn tok_is_one_pass_not_esc_then_escape() {
        assert_eq!(tok(br"a\,b"), r"a\\\x2cb");
        assert_ne!(tok(br"a\,b"), r"a\\\\x2cb");
    }

    #[test]
    fn esc_round_trips_every_byte() {
        let all: Vec<u8> = (0u8..=255).collect();
        assert_eq!(unesc(&esc(&all)).unwrap(), all);
        assert_eq!(unesc(&tok(&all)).unwrap(), all);
    }

    #[test]
    fn unesc_refuses_what_esc_never_emits() {
        assert_eq!(unesc(r"a\"), Err(UnescError::TrailingBackslash));
        assert_eq!(
            unesc(r"a\q"),
            Err(UnescError::BadEscape { at: 1, found: 'q' })
        );
        // Uppercase hex is not an alias for lowercase.
        assert_eq!(unesc(r"a\xC3"), Err(UnescError::BadHex { at: 1 }));
        assert_eq!(unesc(r"a\x0"), Err(UnescError::BadHex { at: 1 }));
        assert!(matches!(
            unesc("a\u{7f}"),
            Err(UnescError::NotPrintableAscii { at: 1, .. })
        ));
    }

    #[test]
    fn esc_output_is_printable_ascii_for_every_input() {
        let all: Vec<u8> = (0u8..=255).collect();
        for s in [esc(&all), tok(&all)] {
            assert!(
                s.bytes().all(|b| (0x20..=0x7E).contains(&b)),
                "esc/tok emitted a byte outside U+0020..U+007E"
            );
        }
    }
}
