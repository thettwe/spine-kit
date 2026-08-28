//! The `Spine-*` line: what one is, what its name may be, where it ranks, and
//! how its payload splits into fields.
//!
//! Two acts, kept apart because EV §2.3 keeps them apart:
//!
//! - **Selection is purely lexical and total** — [`is_spine_line`] never parses
//!   anything. "A digest that could only be computed over a *valid* envelope
//!   would be uncomputable for the malformed envelope a verifier most needs to
//!   diagnose. Every reader can always compute `envelope=`" (EV §2.3).
//! - **Validation is a separate act, and it fails closed** — [`parse_line`].
//!
//! The two compose the safe way round: `Spine-Review:x` is hashed *and*
//! refused, "where a validity-gated selection would have hashed neither and an
//! unwary indexer might still have read it" (EV §2.3).
//!
//! The typed payload grammars of PB §11 live in [`crate::payload`]; this module
//! owns everything above them.

use crate::refusal::{EnvelopeError, Refusal};
use core::fmt;

/// The six bytes that select a line, `case-sensitive` (EV §2.3).
const PREFIX: &[u8] = b"Spine-";

/// EV §2.3: "a line whose first six bytes are `S`, `p`, `i`, `n`, `e`, `-`
/// (`0x53 0x70 0x69 0x6E 0x65 0x2D`), case-sensitive, followed by at least one
/// further byte."
///
/// Case matters, and the near-misses are not near-misses: "`spine-seal: …` and
/// `SPINE-SEAL: …` are not `Spine-*` lines: they are ordinary message text,
/// outside the digest" (EV §2.3) — and separately refused, so no reader is left
/// honouring a spelling the digest ignored.
pub fn is_spine_line(line: &[u8]) -> bool {
    line.len() > PREFIX.len() && &line[..PREFIX.len()] == PREFIX
}

/// EV §2.4's closed set of twenty-six names, in rank order.
///
/// "An unknown `Spine-*` name is `envelope-malformed`, never ignored: there is
/// no forward-compatibility relaxation … `Spine-Envelope` already versions the
/// format, so a reader that meets a name it does not know is reading a version
/// it does not implement and must say so."
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TrailerName {
    Envelope,
    Event,
    Lane,
    Intent,
    Signoff,
    SignoffSig,
    Upgrade,
    UpgradeSig,
    Reopen,
    ReopenSig,
    Withdraw,
    WithdrawSig,
    Approve,
    ApproveSig,
    Approval,
    Frozen,
    Test,
    Review,
    ReviewSig,
    Gates,
    Strategy,
    Supersedes,
    Reverts,
    TrustRootPrev,
    Seal,
    SealSig,
}

impl TrailerName {
    /// All twenty-six, in EV §2.4's rank order. `Ord` on this type is that
    /// order, so a sort over names is a sort over ranks with `-Sig` lines
    /// already adjacent to what they sign.
    pub const ALL: [TrailerName; 26] = [
        TrailerName::Envelope,
        TrailerName::Event,
        TrailerName::Lane,
        TrailerName::Intent,
        TrailerName::Signoff,
        TrailerName::SignoffSig,
        TrailerName::Upgrade,
        TrailerName::UpgradeSig,
        TrailerName::Reopen,
        TrailerName::ReopenSig,
        TrailerName::Withdraw,
        TrailerName::WithdrawSig,
        TrailerName::Approve,
        TrailerName::ApproveSig,
        TrailerName::Approval,
        TrailerName::Frozen,
        TrailerName::Test,
        TrailerName::Review,
        TrailerName::ReviewSig,
        TrailerName::Gates,
        TrailerName::Strategy,
        TrailerName::Supersedes,
        TrailerName::Reverts,
        TrailerName::TrustRootPrev,
        TrailerName::Seal,
        TrailerName::SealSig,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            TrailerName::Envelope => "Spine-Envelope",
            TrailerName::Event => "Spine-Event",
            TrailerName::Lane => "Spine-Lane",
            TrailerName::Intent => "Spine-Intent",
            TrailerName::Signoff => "Spine-Signoff",
            TrailerName::SignoffSig => "Spine-Signoff-Sig",
            TrailerName::Upgrade => "Spine-Upgrade",
            TrailerName::UpgradeSig => "Spine-Upgrade-Sig",
            TrailerName::Reopen => "Spine-Reopen",
            TrailerName::ReopenSig => "Spine-Reopen-Sig",
            TrailerName::Withdraw => "Spine-Withdraw",
            TrailerName::WithdrawSig => "Spine-Withdraw-Sig",
            TrailerName::Approve => "Spine-Approve",
            TrailerName::ApproveSig => "Spine-Approve-Sig",
            TrailerName::Approval => "Spine-Approval",
            TrailerName::Frozen => "Spine-Frozen",
            TrailerName::Test => "Spine-Test",
            TrailerName::Review => "Spine-Review",
            TrailerName::ReviewSig => "Spine-Review-Sig",
            TrailerName::Gates => "Spine-Gates",
            TrailerName::Strategy => "Spine-Strategy",
            TrailerName::Supersedes => "Spine-Supersedes",
            TrailerName::Reverts => "Spine-Reverts",
            TrailerName::TrustRootPrev => "Spine-Trust-Root-Prev",
            TrailerName::Seal => "Spine-Seal",
            TrailerName::SealSig => "Spine-Seal-Sig",
        }
    }

    /// Exact-byte lookup. No case folding: EV §2.3 makes `SPINE-SEAL` a
    /// different thing entirely, not an alias.
    pub fn parse(name: &[u8]) -> Option<Self> {
        TrailerName::ALL
            .into_iter()
            .find(|n| n.as_str().as_bytes() == name)
    }

    /// EV §2.4's Rank column, 1..=20. "a trailer absent from a landing consumes
    /// no rank; a `-Sig` line immediately follows the line it signs" — so a
    /// `-Sig` shares its statement's rank and adjacency is checked separately.
    pub fn rank(self) -> u8 {
        match self {
            TrailerName::Envelope => 1,
            TrailerName::Event => 2,
            TrailerName::Lane => 3,
            TrailerName::Intent => 4,
            TrailerName::Signoff | TrailerName::SignoffSig => 5,
            TrailerName::Upgrade | TrailerName::UpgradeSig => 6,
            TrailerName::Reopen | TrailerName::ReopenSig => 7,
            TrailerName::Withdraw | TrailerName::WithdrawSig => 8,
            TrailerName::Approve | TrailerName::ApproveSig => 9,
            TrailerName::Approval => 10,
            TrailerName::Frozen => 11,
            TrailerName::Test => 12,
            TrailerName::Review | TrailerName::ReviewSig => 13,
            TrailerName::Gates => 14,
            TrailerName::Strategy => 15,
            TrailerName::Supersedes => 16,
            TrailerName::Reverts => 17,
            TrailerName::TrustRootPrev => 18,
            TrailerName::Seal => 19,
            TrailerName::SealSig => 20,
        }
    }

    /// The statement a `-Sig` line signs, or `None` for a statement line.
    /// Seven of the twenty-six are `-Sig` lines (EV §2.4).
    pub fn signs(self) -> Option<TrailerName> {
        match self {
            TrailerName::SignoffSig => Some(TrailerName::Signoff),
            TrailerName::UpgradeSig => Some(TrailerName::Upgrade),
            TrailerName::ReopenSig => Some(TrailerName::Reopen),
            TrailerName::WithdrawSig => Some(TrailerName::Withdraw),
            TrailerName::ApproveSig => Some(TrailerName::Approve),
            TrailerName::ReviewSig => Some(TrailerName::Review),
            TrailerName::SealSig => Some(TrailerName::Seal),
            _ => None,
        }
    }

    /// The `-Sig` line this statement takes, or `None` for an unsigned trailer.
    pub fn sig(self) -> Option<TrailerName> {
        TrailerName::ALL
            .into_iter()
            .find(|n| n.signs() == Some(self))
    }

    pub fn is_sig(self) -> bool {
        self.signs().is_some()
    }

    /// EV §2.4: `Spine-Frozen`, `Spine-Test`, `Spine-Reopen` and `Spine-Review`
    /// (with their `-Sig`s) are the repeatable ranks. Everything else is 0–1.
    pub fn is_repeatable(self) -> bool {
        matches!(
            self,
            TrailerName::Frozen
                | TrailerName::Test
                | TrailerName::Reopen
                | TrailerName::ReopenSig
                | TrailerName::Review
                | TrailerName::ReviewSig
        )
    }
}

impl fmt::Display for TrailerName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A well-formed `Spine-*` line, split at its `": "`.
///
/// The payload is bytes, not `str`: EV §4.4 lets a `Spine-Test` id be "emitted
/// verbatim as UTF-8 (or as whatever bytes the runner produced)", so a
/// conforming line need not be valid UTF-8 and a `String` here would refuse
/// envelopes the spec admits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trailer<'a> {
    pub name: TrailerName,
    pub payload: &'a [u8],
}

/// EV §2.3's well-formedness, all three clauses:
///
/// - "its name matches `Spine-[A-Za-z][A-Za-z0-9-]*` and is one of the
///   twenty-six names in §2.4's closed set";
/// - "the name is followed by exactly `:` `U+0020` (one colon, one space)";
/// - "the remainder — the **payload** — is non-empty and contains no `0x0D` and
///   no `0x00`."
pub fn parse_line(line: &[u8]) -> Result<Trailer<'_>, EnvelopeError> {
    if !is_spine_line(line) {
        return Err(EnvelopeError::malformed(format!(
            "not a Spine-* line: {}",
            show(line)
        )));
    }
    let colon = line
        .iter()
        .position(|&b| b == b':')
        .ok_or_else(|| EnvelopeError::malformed(format!("no colon in {}", show(line))))?;
    let (name, rest) = line.split_at(colon);

    if !name_in_grammar(name) {
        return Err(EnvelopeError::malformed(format!(
            "name out of grammar: {}",
            show(name)
        )));
    }
    let name = TrailerName::parse(name)
        .ok_or_else(|| EnvelopeError::malformed(format!("unknown trailer name: {}", show(name))))?;

    // "exactly `:` `U+0020`" — not a tab, not two spaces, not a bare colon.
    if !rest.starts_with(b": ") {
        return Err(EnvelopeError::malformed(format!(
            "{name} is not followed by ': '"
        )));
    }
    let payload = &rest[2..];
    if payload.is_empty() {
        return Err(EnvelopeError::malformed(format!(
            "{name} has empty payload"
        )));
    }
    if let Some(i) = payload.iter().position(|&b| b == 0x0D || b == 0x00) {
        return Err(EnvelopeError::malformed(format!(
            "{name} payload holds 0x{:02X} at byte {i}",
            payload[i]
        )));
    }
    Ok(Trailer { name, payload })
}

/// `Spine-[A-Za-z][A-Za-z0-9-]*` (EV §2.3). Checked before the closed-set
/// lookup so that a name that is merely unknown is distinguishable from one
/// that could never be a trailer name at all.
fn name_in_grammar(name: &[u8]) -> bool {
    let Some(tail) = name.strip_prefix(PREFIX) else {
        return false;
    };
    match tail.split_first() {
        Some((first, rest)) => {
            first.is_ascii_alphabetic()
                && rest.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'-')
        }
        None => false,
    }
}

/// Render `<name>: <payload>` with no terminator. The terminator is the caller's
/// because it is outside every signature (EV §2.7) and outside both joins
/// (EV §3.1, §4.1).
pub fn render_line(name: TrailerName, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(name.as_str().len() + 2 + payload.len());
    out.extend_from_slice(name.as_str().as_bytes());
    out.extend_from_slice(b": ");
    out.extend_from_slice(payload);
    out
}

// ---------------------------------------------------------------------------
// Payload fields
// ---------------------------------------------------------------------------

/// Split a `key=value` payload into fields.
///
/// EV §2.5: "One `U+0020` between fields, none before the first, none after the
/// last." The splitter is nonetheless quote-aware, because the same section
/// makes `reason=` values "JSON string literals … so a reason containing a
/// quote, a backslash, a newline or any non-ASCII character is representable
/// and the line stays one line" — and a reason routinely contains spaces. A
/// naive split on `0x20` would tear vector A's review line into eleven pieces.
///
/// This is not for `Spine-Frozen` or `Spine-Test`, whose payloads are not
/// `key=value` at all and split at their **first** space (EV §4.3, §4.4).
pub fn split_fields(payload: &[u8]) -> Result<Vec<&[u8]>, EnvelopeError> {
    if payload.first() == Some(&b' ') {
        return Err(EnvelopeError::malformed("payload has a leading space"));
    }
    if payload.last() == Some(&b' ') {
        return Err(EnvelopeError::malformed("payload has a trailing space"));
    }

    let mut out = Vec::new();
    let mut start = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (i, &b) in payload.iter().enumerate() {
        if in_string {
            // JSON escaping, and only enough of it to find the closing quote:
            // a `\` consumes whatever follows, so `\"` never closes a literal.
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
        } else if b == b'"' {
            in_string = true;
        } else if b == b' ' {
            if i == start {
                return Err(EnvelopeError::malformed("two spaces between fields"));
            }
            out.push(&payload[start..i]);
            start = i + 1;
        }
    }
    if in_string {
        return Err(EnvelopeError::malformed(
            "unterminated JSON string literal in payload",
        ));
    }
    out.push(&payload[start..]);
    Ok(out)
}

/// GR §6.2, restated because it governs every field here: "a trailer field
/// splits on its first `=`, so `wires=G2:src/a=b.ts` parses as the field
/// `wires` with the value `G2:src/a=b.ts`."
pub fn split_kv(field: &[u8]) -> Option<(&[u8], &[u8])> {
    let eq = field.iter().position(|&b| b == b'=')?;
    Some((&field[..eq], &field[eq + 1..]))
}

/// What [`take`] returns: the positional fields, then one slot per key in
/// PB §11's order — `None` where an optional key was absent.
pub type Fields<'a> = (Vec<&'a [u8]>, Vec<Option<&'a [u8]>>);

/// One key in a payload grammar, in PB §11's printed order.
#[derive(Debug, Clone, Copy)]
pub struct Key {
    pub name: &'static str,
    pub required: bool,
}

impl Key {
    pub const fn req(name: &'static str) -> Key {
        Key {
            name,
            required: true,
        }
    }
    pub const fn opt(name: &'static str) -> Key {
        Key {
            name,
            required: false,
        }
    }
}

/// Drive a payload against PB §11's field list.
///
/// EV §2.5 rule 1: "**Field order is normative.** Fields appear in exactly the
/// order PB §11's payload column gives … A different order is
/// `envelope-malformed`. Parsing is by key, but *emission* is by position —
/// without that, two implementations produce different bytes over identical
/// facts and every digest and every signature diverges."
///
/// So this walks the spec forward and never backward: a repeated key, a
/// reordered key and an unknown key are all the same failure, which is the
/// point — each of them is a line no conforming implementation would have
/// written.
///
/// `lead` counts the positional fields PB §11 prints before the first `key=`
/// (the intent id, or `quick`, or `reseal`).
pub fn take<'a>(payload: &'a [u8], lead: usize, keys: &[Key]) -> Result<Fields<'a>, EnvelopeError> {
    let fields = split_fields(payload)?;
    if fields.len() < lead {
        return Err(EnvelopeError::malformed(format!(
            "payload has {} fields, fewer than the {lead} positional ones",
            fields.len()
        )));
    }
    let (positional, rest) = fields.split_at(lead);
    for p in positional {
        // A positional field carrying an `=` would be indistinguishable from a
        // keyed one, and emission-by-position could not be inverted.
        if p.contains(&b'=') {
            return Err(EnvelopeError::malformed(format!(
                "positional field holds '=': {}",
                show(p)
            )));
        }
        if p.is_empty() {
            return Err(EnvelopeError::malformed("empty positional field"));
        }
    }

    let mut values: Vec<Option<&[u8]>> = vec![None; keys.len()];
    let mut ki = 0usize;
    for field in rest {
        let (key, value) = split_kv(field).ok_or_else(|| {
            EnvelopeError::malformed(format!("field is not key=value: {}", show(field)))
        })?;
        loop {
            let Some(spec) = keys.get(ki) else {
                return Err(EnvelopeError::malformed(format!(
                    "unknown, repeated or out-of-order field: {}",
                    show(key)
                )));
            };
            if spec.name.as_bytes() == key {
                break;
            }
            if spec.required {
                return Err(EnvelopeError::malformed(format!(
                    "missing required field {} before {}",
                    spec.name,
                    show(key)
                )));
            }
            ki += 1;
        }
        values[ki] = Some(value);
        ki += 1;
    }
    for spec in &keys[ki..] {
        if spec.required {
            return Err(EnvelopeError::malformed(format!(
                "missing required field {}",
                spec.name
            )));
        }
    }
    Ok((positional.to_vec(), values))
}

/// Render a payload from positional fields and `(key, value)` pairs already in
/// PB §11's order, "One `U+0020` between fields, none before the first, none
/// after the last" (EV §2.5).
pub fn render_fields(positional: &[&[u8]], keyed: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in positional {
        if !out.is_empty() {
            out.push(b' ');
        }
        out.extend_from_slice(p);
    }
    for (k, v) in keyed {
        if !out.is_empty() {
            out.push(b' ');
        }
        out.extend_from_slice(k.as_bytes());
        out.push(b'=');
        out.extend_from_slice(v);
    }
    out
}

/// A lossy rendering for diagnostics only. Never used to build a byte a digest
/// or a signature covers.
pub(crate) fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A helper for the many fields whose value is a bare ASCII token.
pub(crate) fn as_str<'a>(value: &'a [u8], what: &str) -> Result<&'a str, EnvelopeError> {
    core::str::from_utf8(value)
        .map_err(|_| EnvelopeError::malformed(format!("{what} is not UTF-8")))
}

/// EV §7 rule 9: "**Counters** are plain decimal, no sign, no leading zero."
pub(crate) fn counter(value: &[u8], what: &str) -> Result<u64, EnvelopeError> {
    let s = as_str(value, what)?;
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(EnvelopeError::malformed(format!(
            "{what} is not a plain decimal counter: {s}"
        )));
    }
    if s.len() > 1 && s.starts_with('0') {
        return Err(EnvelopeError::malformed(format!(
            "{what} has a leading zero: {s}"
        )));
    }
    s.parse()
        .map_err(|_| EnvelopeError::malformed(format!("{what} does not fit: {s}")))
}

/// EV §7 rule 7: "**Object ids** are lowercase hex at the full length the
/// repository's `object_format` implies — 40 or 64 digits. Never abbreviated,
/// never uppercase."
pub(crate) fn oid(value: &[u8], what: &str) -> Result<String, EnvelopeError> {
    let s = as_str(value, what)?;
    let ok = (s.len() == 40 || s.len() == 64)
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !ok {
        return Err(EnvelopeError::malformed(format!(
            "{what} is not a full-length lowercase object id: {s}"
        )));
    }
    Ok(s.to_owned())
}

/// EV §7 rule 8: "**Non-git digests** are `sha256:` + 64 lowercase hex (PB §11
/// hash policy)." Never bare hex, never abbreviated, never uppercase (EV §5).
pub(crate) fn sha256_field(value: &[u8], what: &str) -> Result<String, EnvelopeError> {
    let s = as_str(value, what)?;
    let hex = s.strip_prefix("sha256:").ok_or_else(|| {
        EnvelopeError::malformed(format!("{what} is not spelled sha256:<hex>: {s}"))
    })?;
    let ok = hex.len() == 64
        && hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !ok {
        return Err(EnvelopeError::malformed(format!(
            "{what} is not 64 lowercase hex digits: {s}"
        )));
    }
    Ok(s.to_owned())
}

/// `Spine-Envelope: 1` and nothing else. EV §3.4 and §12: a different version
/// is `envelope-version-unknown` and the reader must "refuse before computing
/// anything" — a distinct refusal from `envelope-malformed`, because the
/// envelope may be perfectly well-formed under a scheme this binary does not
/// implement.
pub const ENVELOPE_VERSION: &str = "1";

pub(crate) fn check_envelope_version(payload: &[u8]) -> Result<(), EnvelopeError> {
    if payload == ENVELOPE_VERSION.as_bytes() {
        Ok(())
    } else {
        Err(EnvelopeError::new(
            Refusal::EnvelopeVersionUnknown,
            format!("Spine-Envelope: {}", show(payload)),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_is_lexical_and_case_sensitive() {
        assert!(is_spine_line(b"Spine-Seal: x"));
        // EV §2.3: these "are ordinary message text, outside the digest".
        assert!(!is_spine_line(b"spine-seal: x"));
        assert!(!is_spine_line(b"SPINE-SEAL: x"));
        // "followed by at least one further byte"
        assert!(!is_spine_line(b"Spine-"));
        assert!(is_spine_line(b"Spine-x"));
    }

    #[test]
    fn a_malformed_line_is_still_selected() {
        // EV §2.3's composition: hashed *and* refused.
        assert!(is_spine_line(b"Spine-Review:x"));
        assert!(parse_line(b"Spine-Review:x").is_err());
    }

    #[test]
    fn the_name_set_is_closed_at_twenty_six() {
        assert_eq!(TrailerName::ALL.len(), 26);
        assert_eq!(
            TrailerName::ALL.iter().filter(|n| n.is_sig()).count(),
            7,
            "EV §2.4: seven of them are -Sig lines, so nineteen are statements"
        );
        assert!(TrailerName::parse(b"Spine-Future").is_none());
        assert!(parse_line(b"Spine-Future: 1").is_err());
    }

    #[test]
    fn ranks_are_ev_2_4s_and_a_sig_shares_its_statements_rank() {
        assert_eq!(TrailerName::Envelope.rank(), 1);
        assert_eq!(TrailerName::Signoff.rank(), TrailerName::SignoffSig.rank());
        assert_eq!(TrailerName::Frozen.rank(), 11);
        assert_eq!(TrailerName::Test.rank(), 12);
        assert_eq!(TrailerName::Seal.rank(), 19);
        assert_eq!(TrailerName::SealSig.rank(), 20);
        // Non-decreasing over the declaration order (EV §18 item 6).
        for pair in TrailerName::ALL.windows(2) {
            assert!(pair[0].rank() <= pair[1].rank());
        }
    }

    #[test]
    fn a_payload_must_be_non_empty_and_free_of_cr_and_nul() {
        assert!(parse_line(b"Spine-Lane: ").is_err());
        assert!(parse_line(b"Spine-Lane: quick\r").is_err());
        assert!(parse_line(b"Spine-Lane: qu\0ick").is_err());
        assert_eq!(parse_line(b"Spine-Lane: quick").unwrap().payload, b"quick");
    }

    #[test]
    fn exactly_one_space_after_the_colon() {
        assert!(parse_line(b"Spine-Lane:quick").is_err());
        assert!(
            parse_line(b"Spine-Lane:  quick").is_ok(),
            "payload is ' quick'"
        );
        assert_eq!(
            parse_line(b"Spine-Lane:  quick").unwrap().payload,
            b" quick",
            "the second space is payload, not separator; the grammar rejects it"
        );
    }

    #[test]
    fn a_reasons_spaces_do_not_split_the_field() {
        let payload =
            br#"INT-042 reason="AC-3 was not testable as written" signer=alice@example.com"#;
        let fields = split_fields(payload).unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[1], br#"reason="AC-3 was not testable as written""#);
    }

    #[test]
    fn an_escaped_quote_does_not_close_a_reason() {
        let payload = br#"a reason="he said \"no\" twice" signer=x"#;
        let fields = split_fields(payload).unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[1], br#"reason="he said \"no\" twice""#);
    }

    #[test]
    fn a_field_splits_on_its_first_equals() {
        // GR §6.2's own example.
        let (k, v) = split_kv(b"wires=G2:src/a=b.ts").unwrap();
        assert_eq!(k, b"wires");
        assert_eq!(v, b"G2:src/a=b.ts");
    }

    #[test]
    fn field_order_is_normative() {
        let keys = [Key::req("a"), Key::opt("b"), Key::req("c")];
        assert!(take(b"a=1 b=2 c=3", 0, &keys).is_ok());
        assert!(take(b"a=1 c=3", 0, &keys).is_ok(), "b is optional");
        assert!(take(b"b=2 a=1 c=3", 0, &keys).is_err(), "reordered");
        assert!(take(b"a=1 a=1 c=3", 0, &keys).is_err(), "repeated");
        assert!(take(b"a=1 c=3 d=4", 0, &keys).is_err(), "unknown");
        assert!(take(b"a=1 b=2", 0, &keys).is_err(), "c is required");
    }

    #[test]
    fn an_empty_field_value_is_legal() {
        // MF §6.4: "the empty list is the **empty value** (`forced= signer=…`)".
        let keys = [Key::req("forced"), Key::req("signer")];
        let (_, v) = take(b"forced= signer=alice@example.com", 0, &keys).unwrap();
        assert_eq!(v[0], Some(&b""[..]));
    }

    #[test]
    fn double_and_edge_spaces_are_malformed() {
        assert!(split_fields(b"a=1  b=2").is_err());
        assert!(split_fields(b" a=1").is_err());
        assert!(split_fields(b"a=1 ").is_err());
    }

    #[test]
    fn a_counter_has_no_sign_and_no_leading_zero() {
        assert_eq!(counter(b"0", "n").unwrap(), 0);
        assert_eq!(counter(b"765", "n").unwrap(), 765);
        assert!(counter(b"01", "n").is_err());
        assert!(counter(b"+1", "n").is_err());
        assert!(counter(b"-1", "n").is_err());
    }

    #[test]
    fn an_oid_is_never_abbreviated_and_never_uppercase() {
        let sha1 = "dfb4079e22de55ec377468b9b697fdf86085ea37";
        assert_eq!(oid(sha1.as_bytes(), "blob").unwrap(), sha1);
        assert!(oid(b"dfb4079e", "blob").is_err(), "PB's 9f2c... is display");
        assert!(oid(sha1.to_uppercase().as_bytes(), "blob").is_err());
    }

    #[test]
    fn a_non_git_digest_is_never_bare_hex() {
        let d = "sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2";
        assert_eq!(sha256_field(d.as_bytes(), "freeze").unwrap(), d);
        assert!(sha256_field(&d.as_bytes()[7..], "freeze").is_err());
    }

    #[test]
    fn an_unknown_envelope_version_is_its_own_refusal() {
        assert!(check_envelope_version(b"1").is_ok());
        let e = check_envelope_version(b"2").unwrap_err();
        assert_eq!(e.refusal(), Refusal::EnvelopeVersionUnknown);
    }
}
