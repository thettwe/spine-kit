//! `envelope=` and `freeze=` — two digests, one join.
//!
//! **The join has no trailing separator** (EV §7 rule 10, EV §3.2 question 4).
//! "'LF-joined' is separator semantics: `n` lines yield `n − 1` separators …
//! it makes the one-line self-test hold — the envelope digest of a single-line
//! trailer block equals the SHA-256 of that line, which is the first thing
//! anyone checks by hand."
//!
//! This is the one rule in the corpus that inverts by artifact — the gate report
//! carries no trailing newline, the manifest carries exactly one, `dump.md`
//! terminates every line *including the last* — so both wrong values EV
//! publishes are pinned in this module's tests, "published so a
//! mis-implementation recognises itself" (EV §8.2).

use crate::refusal::EnvelopeError;
use crate::trailer::{TrailerName, is_spine_line};
use core::cmp::Ordering;

/// The boundary EV §3.3's `awk` program uses: `/^Spine-Seal: /{exit}`.
///
/// The trailing space matters. `Spine-Seal-Sig: …` does not start with these
/// twelve bytes, which is what keeps the seal's own `-Sig` below the boundary
/// without a second rule.
const SEAL_PREFIX: &[u8] = b"Spine-Seal: ";

/// EV §2.2: "A **line** is a maximal run of bytes containing no `0x0A`,
/// delimited by `0x0A` or by the start of the message. **The terminating
/// `0x0A` is not part of the line.**"
///
/// "Lines are taken **raw**. Nothing is trimmed, unfolded, case-folded,
/// Unicode-normalized, or re-encoded, at any point, for any purpose … A
/// trailing `0x0D` is part of the line and is hashed as such." Detection, never
/// repair — so this function has no options.
pub fn lines(message: &[u8]) -> Vec<&[u8]> {
    if message.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<&[u8]> = message.split(|&b| b == b'\n').collect();
    // A message ending in `0x0A` (EV §2.1 requires exactly one) yields a final
    // empty element that is a terminator, not a line.
    if message.last() == Some(&b'\n') {
        out.pop();
    }
    out
}

/// EV §3.1 / §4.1's join: "joining them with a single `0x0A` between
/// consecutive lines — **no separator before the first, and none after the
/// last**."
pub fn join_lf(lines: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push(b'\n');
        }
        out.extend_from_slice(line);
    }
    out
}

/// Select the lines `envelope=` covers: "in message order, every `Spine-*` line
/// (§2.3) that appears **above the `Spine-Seal` line**" (EV §3.1).
///
/// Total by construction — selection "never requires parsing it, knowing its
/// name, or judging it well-formed" (EV §2.3). With no seal line the whole
/// message's `Spine-*` lines are returned; [`envelope_digest`] is the one that
/// refuses, because *that* is a structural judgement.
pub fn above_seal<'a>(lines: &[&'a [u8]]) -> Vec<&'a [u8]> {
    let mut out = Vec::new();
    for line in lines {
        if line.starts_with(SEAL_PREFIX) {
            break;
        }
        if is_spine_line(line) {
            out.push(*line);
        }
    }
    out
}

/// `sha256:<hex>` over a joined line sequence. Both digests share it because
/// they share the join (EV §4.1: "The join is `envelope=`'s join, for the same
/// reasons … this document fixes one and uses it in both places rather than
/// two").
fn digest_of(lines: &[&[u8]]) -> String {
    spine_canon::sha256_prefixed(&join_lf(lines))
}

/// `envelope=` over an already-selected line sequence.
///
/// "The digest of an empty sequence is the SHA-256 of the empty string …
/// No conforming landing produces one … but the function is total and an
/// implementation must not special-case it" (EV §3.1).
pub fn envelope_digest_over(lines: &[&[u8]]) -> String {
    digest_of(lines)
}

/// `envelope=` recomputed from a whole commit message.
///
/// EV §3.4: with no `Spine-Seal` line "the commit is not a landing" — that is
/// `envelope-malformed`, and this returns it rather than a digest over
/// everything, so a caller cannot accidentally compare a sealless message's
/// digest against a seal that is not there.
pub fn envelope_digest(message: &[u8]) -> Result<String, EnvelopeError> {
    let all = lines(message);
    if !all.iter().any(|l| l.starts_with(SEAL_PREFIX)) {
        return Err(EnvelopeError::malformed(
            "no Spine-Seal line: the commit is not a landing",
        ));
    }
    Ok(envelope_digest_over(&above_seal(&all)))
}

/// EV §4.2's comparison: "**Ascending by unsigned byte value, over the entire
/// line, `memcmp` order, shorter-is-smaller on a prefix tie.**"
///
/// Not locale collation, and **not** the `esc` order `dump.md` §6.4 uses:
/// "here the artifact is the raw trailer line, and the sort is over exactly the
/// bytes that are hashed. Sorting one thing and hashing another is how a spec
/// grows a second place to disagree."
pub fn freeze_cmp(a: &[u8], b: &[u8]) -> Ordering {
    a.cmp(b)
}

/// Select every `Spine-Frozen` and `Spine-Test` line of a commit, in message
/// order (EV §4.1: "the commit in question" is the approval commit under merge
/// and the landing itself under squash — same lines, same function).
pub fn freeze_lines<'a>(lines: &[&'a [u8]]) -> Vec<&'a [u8]> {
    let frozen = format!("{}: ", TrailerName::Frozen);
    let test = format!("{}: ", TrailerName::Test);
    lines
        .iter()
        .filter(|l| l.starts_with(frozen.as_bytes()) || l.starts_with(test.as_bytes()))
        .copied()
        .collect()
}

/// Sort a manifest into `freeze=` order.
///
/// EV §2.4 requires the lines be *emitted* in this order too, "so the digest
/// recomputes from the copied lines by `sort`-free concatenation and a reader
/// can check the order as well as the value."
pub fn sort_freeze_lines(lines: &mut [&[u8]]) {
    lines.sort_by(|a, b| freeze_cmp(a, b));
}

/// `freeze=` over the manifest lines of one commit (EV §4.1).
///
/// "**The whole line is hashed, not the payload.** … It removes any need to
/// unquote a path before hashing, so the digest cannot diverge on a quoting
/// disagreement — the quoting is hashed, not the decoded bytes."
pub fn freeze_digest(message: &[u8]) -> String {
    let all = lines(message);
    let mut manifest = freeze_lines(&all);
    sort_freeze_lines(&mut manifest);
    digest_of(&manifest)
}

/// `freeze=` over lines the caller already holds, sorted here.
pub fn freeze_digest_over(lines: &[&[u8]]) -> String {
    let mut sorted = lines.to_vec();
    sort_freeze_lines(&mut sorted);
    digest_of(&sorted)
}

/// EV §4.2: "**Duplicate lines are impossible.** Two identical `Spine-Frozen`
/// lines would name one path twice; two identical `Spine-Test` lines would name
/// one `(runner, id)` pair twice. `--approve` refuses either
/// (`freeze-duplicate`). The order is therefore total."
pub fn check_no_duplicates(lines: &[&[u8]]) -> Result<(), EnvelopeError> {
    let mut sorted = lines.to_vec();
    sort_freeze_lines(&mut sorted);
    for pair in sorted.windows(2) {
        if pair[0] == pair[1] {
            return Err(EnvelopeError::new(
                crate::refusal::Refusal::FreezeDuplicate,
                crate::trailer::show(pair[0]),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_join_of_one_line_is_that_lines_digest() {
        // EV §3.2 question 4 reason (b): "it makes the one-line self-test hold".
        let line: &[u8] = b"Spine-Envelope: 1";
        assert_eq!(
            envelope_digest_over(&[line]),
            spine_canon::sha256_prefixed(line)
        );
    }

    #[test]
    fn the_empty_sequence_digests_the_empty_string() {
        // EV §3.1 names the value and forbids special-casing it.
        assert_eq!(
            envelope_digest_over(&[]),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn the_join_carries_no_trailing_lf() {
        let a: &[u8] = b"one";
        let b: &[u8] = b"two";
        assert_eq!(join_lf(&[a, b]), b"one\ntwo".to_vec());
        assert_eq!(join_lf(&[a]), b"one".to_vec());
        assert_eq!(join_lf(&[]), Vec::<u8>::new());
    }

    #[test]
    fn a_line_keeps_its_trailing_cr() {
        // EV §2.2: "an implementation that strips CR before hashing would accept
        // a CRLF body the seal does not cover".
        assert_eq!(lines(b"a\r\nb\n"), vec![&b"a\r"[..], &b"b"[..]]);
    }

    #[test]
    fn the_seal_boundary_needs_the_trailing_space() {
        let all = [
            &b"Spine-Envelope: 1"[..],
            &b"Spine-Seal: quick base=x"[..],
            &b"Spine-Seal-Sig: AAAA"[..],
        ];
        assert_eq!(above_seal(&all), vec![&b"Spine-Envelope: 1"[..]]);
    }

    #[test]
    fn the_fence_and_the_subject_contribute_no_line() {
        // EV §3.2 questions 2 and 3: neither region can satisfy `^Spine-`.
        let msg = b"INT-042: t\n\n-----BEGIN SPINE-INTENT blob=x bytes=0-----\n-----END SPINE-INTENT-----\n\nSpine-Envelope: 1\nSpine-Seal: q\nSpine-Seal-Sig: A\n";
        assert_eq!(above_seal(&lines(msg)), vec![&b"Spine-Envelope: 1"[..]]);
    }

    #[test]
    fn a_malformed_spine_line_above_the_seal_is_still_hashed() {
        // EV §3.4: "Hashed (selection is lexical), and `envelope-malformed`."
        let all = [&b"Spine-Review:x"[..], &b"Spine-Seal: q"[..]];
        assert_eq!(above_seal(&all), vec![&b"Spine-Review:x"[..]]);
    }

    #[test]
    fn a_message_with_no_seal_refuses_rather_than_digesting_everything() {
        assert!(envelope_digest(b"Spine-Envelope: 1\n").is_err());
    }

    #[test]
    fn frozen_precedes_test_by_the_whole_line_comparison_alone() {
        // EV §4.2: "`F` (`0x46`) precedes `T` (`0x54`) … This is a consequence
        // of §4.1's rule, not a separate rule."
        let f: &[u8] = b"Spine-Frozen: ffff tests/z.py";
        let t: &[u8] = b"Spine-Test: pytest a::b";
        assert_eq!(freeze_cmp(f, t), Ordering::Less);
    }

    #[test]
    fn a_numeric_id_order_is_non_conforming() {
        // EV §4.2 and vector C: "`… AC10 …` precedes `… AC2 …`."
        let ten: &[u8] = b"Spine-Test: vitest web/x.test.ts > totals > AC10 rounding";
        let two: &[u8] = b"Spine-Test: vitest web/x.test.ts > totals > AC2 zero-rated";
        assert_eq!(freeze_cmp(ten, two), Ordering::Less);
    }

    #[test]
    fn shorter_is_smaller_on_a_prefix_tie() {
        assert_eq!(freeze_cmp(b"Spine-Test: a", b"Spine-Test: ab"), Ordering::Less);
    }

    #[test]
    fn duplicate_manifest_lines_are_freeze_duplicate() {
        let l: &[u8] = b"Spine-Test: pytest a::b";
        let e = check_no_duplicates(&[l, l]).unwrap_err();
        assert_eq!(e.refusal(), crate::refusal::Refusal::FreezeDuplicate);
        assert!(check_no_duplicates(&[l]).is_ok());
    }
}
