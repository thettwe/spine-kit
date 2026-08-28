//! ID §2.1's twelve byte rules and ID §2.3's resource bounds — step 1 of
//! ID §8.2's order, and the whole of exit 2.
//!
//! **Why the bounds are normative rather than implementation advice.** ID §2.3:
//! "a document another person wrote, on a branch anyone with push access
//! created, is parsed by my binary during my landing. A 2 GiB
//! `intents/INT-999.md` on a pushed branch must cost my landing a bounded
//! amount of work and then contribute no lease, rather than exhausting the
//! trusted stage."
//!
//! **No normalisation, in either direction** (ID §2.1). "An NFC document and
//! its NFD counterpart are two different documents with two different blob ids
//! and two different signatures … a report computed on macOS and one computed
//! in a Linux container must agree, and a normalising step is a place they can
//! differ."

use crate::status::{Refusal, Status};

/// ID §2.3's document bound: 65536 bytes, `document-too-large`.
pub const MAX_DOCUMENT: usize = 65536;

/// ID §2.3's line bound: 4096 bytes, `line-too-long`.
pub const MAX_LINE: usize = 4096;

/// A document that has passed every rule of ID §2.1, decoded once.
///
/// ID §2.1: "A **line** is a maximal run of bytes containing no `0x0A`; rule 8
/// makes the last line the one before the file's final `0x0A`, and there is no
/// line after it. `d` has `k` lines where `k` is the count of `0x0A` in `d`."
#[derive(Debug, Clone)]
pub struct Canonical<'a> {
    text: &'a str,
    lines: Vec<&'a str>,
}

impl<'a> Canonical<'a> {
    /// The decoded document, including its single trailing `0x0A`.
    pub fn text(&self) -> &'a str {
        self.text
    }

    /// The document's lines, in order. Index `i` is line `i + 1`.
    pub fn lines(&self) -> &[&'a str] {
        &self.lines
    }

    /// Line `n`, 1-based, or `None` past the end.
    pub fn line(&self, n: usize) -> Option<&'a str> {
        self.lines.get(n.checked_sub(1)?).copied()
    }
}

/// Check ID §2.1, and split the document into lines.
///
/// **The order is ID §8.2's**: "canonical form and the document bound, §2.1's
/// rules in their table order, then per line in line order". So a document that
/// is both un-decodable and over-long reports `document-too-large`, and one
/// with both a trailing space on line 9 and a 5000-byte line 5 reports
/// `trailing-whitespace` — rule 9 precedes rule 10 in the table.
///
/// DERIVED: "then per line in line order" is read as the tie-break *inside* a
/// rule, not as a second pass that visits each line applying rules 9…12. The
/// two readings differ only for a document that breaks two different per-line
/// rules on two different lines; the table-order reading is taken because
/// §8.2's sentence names the table first and the line order second.
pub fn check(d: &[u8]) -> Result<Canonical<'_>, Refusal> {
    // Rule 1.
    if d.is_empty() {
        return Err(Refusal::whole(Status::EmptyDocument));
    }
    // Rule 2. Checked before decoding, so an adversarial branch's oversized
    // document costs one length comparison rather than a UTF-8 sweep.
    if d.len() > MAX_DOCUMENT {
        return Err(Refusal::whole(Status::DocumentTooLarge));
    }
    // Rule 3. `str::from_utf8` is RFC 3629 exactly: it rejects overlong forms,
    // the surrogate range U+D800…U+DFFF, and every value above U+10FFFF.
    let text = match core::str::from_utf8(d) {
        Ok(text) => text,
        Err(e) => return Err(Refusal::at(Status::NotUtf8, line_of(d, e.valid_up_to()))),
    };
    // Rule 4. "at any position, byte-order mark or not" — so this is a search,
    // not a prefix test.
    if let Some(at) = text.find('\u{FEFF}') {
        return Err(Refusal::at(Status::Bom, line_of(d, at)));
    }
    // Rule 5.
    if let Some(at) = d.iter().position(|&b| b == 0x00) {
        return Err(Refusal::at(Status::NulByte, line_of(d, at)));
    }
    // Rule 6. "Not 'no CRLF' — no CR at all, lone or paired."
    if let Some(at) = d.iter().position(|&b| b == 0x0D) {
        return Err(Refusal::at(Status::CrByte, line_of(d, at)));
    }
    // Rule 7. Tabs and newlines are the two exceptions; `0x7F` is the third
    // byte the rule names and it is not below `0x20`.
    if let Some(at) = d
        .iter()
        .position(|&b| (b < 0x20 && b != 0x09 && b != 0x0A) || b == 0x7F)
    {
        return Err(Refusal::at(Status::ControlByte, line_of(d, at)));
    }
    // Rule 8. Three clauses, and the second and third are one status: a
    // document of one `0x0A` is a blank line at end of file, and so is one
    // ending `0x0A 0x0A`.
    if !d.ends_with(b"\n") {
        return Err(Refusal::whole(Status::NoFinalNewline));
    }
    if d == b"\n" || d.ends_with(b"\n\n") {
        return Err(Refusal::whole(Status::TrailingBlankLine));
    }

    // Rule 8 guarantees the final byte is `0x0A`, so dropping the empty tail
    // `split` produces is exactly ID §2.1's "there is no line after it".
    let lines: Vec<&str> = text[..text.len() - 1].split('\n').collect();

    // Rules 9 … 12, each across every line in line order.
    for (i, line) in lines.iter().enumerate() {
        if line.ends_with(' ') || line.ends_with('\t') {
            return Err(Refusal::at(Status::TrailingWhitespace, i + 1));
        }
    }
    for (i, line) in lines.iter().enumerate() {
        if line.len() > MAX_LINE {
            return Err(Refusal::at(Status::LineTooLong, i + 1));
        }
    }
    // Rule 11. ID §2.2: "Case is not a question — the byte is `0x2D`. Five is
    // the count, not 'five or more'." Refused because PB §5.5's envelope
    // delimits the fenced intent with `-----BEGIN SPINE-INTENT …-----` and
    // because `ssh-keygen -Y` armour is `-----BEGIN SSH SIGNATURE-----`.
    for (i, line) in lines.iter().enumerate() {
        if line.as_bytes().starts_with(b"-----") {
            return Err(Refusal::at(Status::FenceCollision, i + 1));
        }
    }
    // Rule 12, ASCII case-insensitive, "covers all 64 spellings. Bytes after
    // the sixth are not examined: `Spine-anything`, and `Spine-` alone, are
    // refused. `Spinel: x` is not (the sixth byte is `l`, not `-`)."
    for (i, line) in lines.iter().enumerate() {
        let b = line.as_bytes();
        if b.len() >= 6 && b[..6].eq_ignore_ascii_case(b"spine-") {
            return Err(Refusal::at(Status::TrailerCollision, i + 1));
        }
    }

    Ok(Canonical { text, lines })
}

/// The 1-based line a byte offset falls on. Diagnostic only — ID §10 rule 10
/// leaves diagnostics free — but a `nul-byte` with no line is useless to the
/// author who has to find it.
fn line_of(d: &[u8], offset: usize) -> usize {
    d[..offset.min(d.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refuse(d: &[u8]) -> Status {
        check(d).expect_err("expected a refusal").status
    }

    #[test]
    fn an_empty_document_is_empty_document() {
        assert_eq!(refuse(b""), Status::EmptyDocument);
    }

    #[test]
    fn the_document_bound_is_65536_bytes() {
        // 65536 bytes exactly is legal; 65537 is not. The body is built so the
        // only rule in question is rule 2.
        let mut ok = vec![b'a'; MAX_DOCUMENT - 1];
        ok.push(b'\n');
        assert_eq!(ok.len(), MAX_DOCUMENT);
        // rule 10 would fire first on one 65535-byte line, so this document is
        // shaped as many short lines instead.
        let body: Vec<u8> = core::iter::repeat_n(b"aaaaaaaaa\n", MAX_DOCUMENT / 10)
            .flatten()
            .copied()
            .collect();
        assert_eq!(body.len(), 65530);
        assert!(check(&body).is_ok());

        let mut over = body.clone();
        over.extend_from_slice(&[b'a'; 7]);
        assert_eq!(over.len(), MAX_DOCUMENT + 1);
        assert_eq!(refuse(&over), Status::DocumentTooLarge);
    }

    /// ID §2.1 rule 3 cites RFC 3629, whose whole content beyond "UTF-8" is the
    /// three exclusions: overlongs, surrogates, and anything above U+10FFFF.
    #[test]
    fn rfc_3629_exclusions_are_all_not_utf8() {
        // Overlong encoding of `/`.
        assert_eq!(refuse(b"# a\n\xc0\xaf\n"), Status::NotUtf8);
        // CESU-8 style surrogate D800.
        assert_eq!(refuse(b"# a\n\xed\xa0\x80\n"), Status::NotUtf8);
        // Five-byte form, above U+10FFFF.
        assert_eq!(refuse(b"# a\n\xf8\x88\x80\x80\x80\n"), Status::NotUtf8);
    }

    #[test]
    fn a_byte_order_mark_is_refused_wherever_it_sits() {
        assert_eq!(refuse("\u{FEFF}# a\n".as_bytes()), Status::Bom);
        assert_eq!(refuse("# a\nb\u{FEFF}c\n".as_bytes()), Status::Bom);
    }

    #[test]
    fn a_lone_cr_is_refused_not_only_crlf() {
        assert_eq!(refuse(b"# a\rb\n"), Status::CrByte);
        assert_eq!(refuse(b"# a\r\n"), Status::CrByte);
    }

    #[test]
    fn a_tab_inside_a_line_is_permitted_and_del_is_not() {
        assert!(check(b"# a\n\tcontinuation\n").is_ok());
        assert_eq!(refuse(b"# a\n\x7f\n"), Status::ControlByte);
        assert_eq!(refuse(b"# a\n\x0b\n"), Status::ControlByte);
    }

    #[test]
    fn exactly_one_trailing_newline_and_no_blank_line_at_eof() {
        assert!(check(b"# a\n").is_ok());
        assert_eq!(refuse(b"# a"), Status::NoFinalNewline);
        assert_eq!(refuse(b"# a\n\n"), Status::TrailingBlankLine);
        assert_eq!(refuse(b"\n"), Status::TrailingBlankLine);
    }

    #[test]
    fn a_trailing_space_or_tab_is_refused_on_any_line() {
        assert_eq!(refuse(b"# a \nb\n"), Status::TrailingWhitespace);
        assert_eq!(refuse(b"# a\nb\t\n"), Status::TrailingWhitespace);
    }

    #[test]
    fn the_line_bound_is_4096_bytes() {
        let ok = format!("{}\n", "a".repeat(MAX_LINE));
        assert!(check(ok.as_bytes()).is_ok());
        let over = format!("{}\n", "a".repeat(MAX_LINE + 1));
        assert_eq!(refuse(over.as_bytes()), Status::LineTooLong);
    }

    /// ID §2.2: "a line of six hyphens begins with five, so it is refused too".
    #[test]
    fn five_hyphens_is_the_count_not_five_or_more() {
        assert!(check(b"# a\n----\n").is_ok());
        assert_eq!(refuse(b"# a\n-----\n"), Status::FenceCollision);
        assert_eq!(refuse(b"# a\n------\n"), Status::FenceCollision);
        assert_eq!(
            refuse(b"# a\n-----BEGIN SSH SIGNATURE-----\n"),
            Status::FenceCollision
        );
    }

    /// ID §2.2: the `Spine-` test "covers all 64 spellings", and stops at the
    /// sixth byte.
    #[test]
    fn the_trailer_refusal_is_ascii_case_insensitive_over_six_bytes() {
        for spelling in [
            "Spine-Seal: x",
            "SPINE-SEAL: x",
            "spine-approve: y",
            "sPiNe-anything",
            "Spine-",
        ] {
            let d = format!("# a\n{spelling}\n");
            assert_eq!(refuse(d.as_bytes()), Status::TrailerCollision, "{spelling}");
        }
    }

    #[test]
    fn spinel_is_not_a_trailer_because_its_sixth_byte_is_l() {
        assert!(check(b"# a\nSpinel: x\n").is_ok());
    }

    /// ID §8.2: "A document breaking rules in two steps reports the earlier
    /// step's status." Within step 1 that is the §2.1 table's own order.
    #[test]
    fn the_table_order_decides_when_several_rules_break_at_once() {
        // rule 6 (CR) precedes rule 9 (trailing whitespace).
        assert_eq!(refuse(b"# a \r\n"), Status::CrByte);
        // rule 9 precedes rule 11 (fence).
        assert_eq!(refuse(b"# a\n----- \n"), Status::TrailingWhitespace);
        // rule 11 precedes rule 12 (trailer).
        assert_eq!(
            refuse(b"# a\n-----\nSpine-Seal: x\n"),
            Status::FenceCollision
        );
    }

    #[test]
    fn the_line_split_leaves_no_line_after_the_final_newline() {
        let c = check(b"# a\nb\nc\n").unwrap();
        assert_eq!(c.lines(), &["# a", "b", "c"]);
        assert_eq!(c.line(1), Some("# a"));
        assert_eq!(c.line(4), None);
        assert_eq!(c.text(), "# a\nb\nc\n");
    }
}
