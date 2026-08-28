//! RFC 8785 JSON Canonicalization Scheme, restricted to the value model.
//!
//! `gate-report.md` §2.1 names JCS as *the* canonical form, and §2.2 reduces it
//! for spine's profile to: "sort each object's members by member-name bytes,
//! ascending; emit with no whitespace; emit integers in plain decimal; emit
//! strings with JSON's minimal escaping (`"` -> `\"`, `\` -> `\\`, nothing else
//! can occur); output UTF-8."
//!
//! That reduction is stated *under the profile*, where member names match
//! `^[a-z][a-z0-9_]*$` and strings are ASCII. This module implements the
//! unreduced rules instead — UTF-16 code-unit ordering, the full ECMAScript
//! escape set — because §8.3's vector pins ordering with names the profile
//! forbids, and because a canonicalizer that is only correct inside the profile
//! is a canonicalizer that fails silently the first time the profile widens.

use crate::value::Value;

/// Canonicalize to bytes. This is the artifact every `sha256:` digest in the
/// corpus is taken over: GR §2.1, "No trailing newline, no BOM, no framing."
pub fn canonicalize(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    write_value(&mut out, value);
    out
}

/// Canonicalize to a `String`. Total: the output of [`canonicalize`] is always
/// valid UTF-8, because every byte it writes is either ASCII or copied from an
/// already-valid `String`.
pub fn canonicalize_to_string(value: &Value) -> String {
    String::from_utf8(canonicalize(value)).expect("canonical output is UTF-8 by construction")
}

fn write_value(out: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        // RFC 8785 §3.2.2.3. Inside the profile every number is an integer in
        // `0 ..= 2^53 - 1`, whose shortest decimal form is its only form, so
        // the ES6 `Number::toString` algorithm the RFC cites reduces to this.
        Value::Int(n) => out.extend_from_slice(itoa(*n).as_bytes()),
        Value::Str(s) => write_string(out, s),
        Value::Arr(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_value(out, item);
            }
            out.push(b']');
        }
        Value::Obj(members) => {
            // RFC 8785 §3.2.3: members are sorted by the UTF-16 code units of
            // their names. Under GR §2.2's `^[a-z][a-z0-9_]*$` this is byte
            // order, which is what §2.2 says; outside it, it is not, and §8.3
            // reaches outside it.
            let mut order: Vec<usize> = (0..members.len()).collect();
            order.sort_by(|&a, &b| utf16_cmp(&members[a].0, &members[b].0));

            out.push(b'{');
            for (i, &idx) in order.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_string(out, &members[idx].0);
                out.push(b':');
                write_value(out, &members[idx].1);
            }
            out.push(b'}');
        }
    }
}

/// Lexicographic comparison over UTF-16 code units, without allocating.
///
/// Differs from byte order only where one string reaches beyond the BMP: a
/// surrogate pair leads with `0xD800..=0xDBFF`, which sorts *below* the
/// unpaired `0xE000..=0xFFFF` range that UTF-8 byte order puts it above.
fn utf16_cmp(a: &str, b: &str) -> core::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// RFC 8785 §3.2.2.2: the escape set of ECMAScript `JSON.stringify`, with
/// lowercase hex in `\u` escapes.
fn write_string(out: &mut Vec<u8>, s: &str) {
    out.push(b'"');
    for ch in s.chars() {
        match ch {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{0008}' => out.extend_from_slice(b"\\b"),
            '\u{0009}' => out.extend_from_slice(b"\\t"),
            '\u{000A}' => out.extend_from_slice(b"\\n"),
            '\u{000C}' => out.extend_from_slice(b"\\f"),
            '\u{000D}' => out.extend_from_slice(b"\\r"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(b"\\u00");
                let byte = c as u32;
                out.push(hex_lower(((byte >> 4) & 0xF) as u8));
                out.push(hex_lower((byte & 0xF) as u8));
            }
            // Everything else is emitted literally as UTF-8. RFC 8785 does not
            // escape non-ASCII, and GR §2.3's `esc` has already removed every
            // non-ASCII byte from any value that carries repository bytes.
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

fn hex_lower(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        _ => b'a' + (nibble - 10),
    }
}

/// Plain decimal, no separators, no sign. Hand-rolled so the output is fixed by
/// this file rather than by a formatting implementation that could change.
fn itoa(mut n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    core::str::from_utf8(&buf[i..])
        .expect("ASCII digits")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `gate-report.md` §8.3 — the minimal canonicalizer vector.
    ///
    /// The corpus says: "Debug your canonicalizer against this before
    /// attempting §8.2." This is that test, and it is the first thing in the
    /// implementation that has to be right.
    #[test]
    fn gr_8_3_minimal_canonicalizer_vector() {
        let value = Value::obj([
            ("b", Value::arr([Value::Int(1), Value::Int(2)])),
            ("a", Value::str("x\\y")),
            ("Z", Value::Bool(true)),
            (
                "_c",
                Value::obj([("n", Value::Int(0)), ("m", Value::str("q\"r"))]),
            ),
        ]);

        let canonical = canonicalize_to_string(&value);
        assert_eq!(
            canonical,
            r#"{"Z":true,"_c":{"m":"q\"r","n":0},"a":"x\\y","b":[1,2]}"#
        );
        assert_eq!(
            crate::digest::sha256_prefixed(canonical.as_bytes()),
            "sha256:a594772ccb6408158b6e76b170d5488c2454ba576e09ae379e24d743e21921f0"
        );
    }

    #[test]
    fn members_sort_by_utf16_code_units_not_utf8_bytes() {
        // U+FFFD is one UTF-16 code unit (0xFFFD); U+10000 is the surrogate
        // pair 0xD800 0xDC00. UTF-8 byte order puts U+10000 second (0xF0.. >
        // 0xEF..); UTF-16 code-unit order puts it first. RFC 8785 §3.2.3 says
        // UTF-16, so this is the ordering the digest depends on.
        let value = Value::obj([("\u{10000}", Value::Int(1)), ("\u{FFFD}", Value::Int(2))]);
        let canonical = canonicalize_to_string(&value);
        assert!(
            canonical.starts_with("{\"\u{10000}\""),
            "expected the surrogate-pair name first, got {canonical}"
        );
    }

    #[test]
    fn empty_containers_and_zero() {
        assert_eq!(canonicalize_to_string(&Value::arr([])), "[]");
        assert_eq!(canonicalize_to_string(&Value::Obj(Vec::new())), "{}");
        assert_eq!(canonicalize_to_string(&Value::Int(0)), "0");
    }

    #[test]
    fn control_characters_take_the_short_escapes_then_u00xx() {
        let value = Value::str("\u{8}\t\n\u{c}\r\u{1}\u{1f}");
        assert_eq!(
            canonicalize_to_string(&value),
            r#""\b\t\n\f\r\u0001\u001f""#
        );
    }

    #[test]
    fn max_safe_integer_round_trips() {
        // GR §2.2 bounds numbers at 2^53 - 1.
        assert_eq!(
            canonicalize_to_string(&Value::Int(9_007_199_254_740_991)),
            "9007199254740991"
        );
    }
}
