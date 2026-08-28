//! `envelope-vectors.md`'s four published vectors, reproduced from the bytes.
//!
//! EV §18 item 25 fixes the order and the reason: "Vector C (§10) reproduces,
//! then vector A's `freeze=` (§8.2), then vector A's `envelope=` (§8.3), then
//! vector B (§9) and vector D (§11). Debug in that order: C isolates the sort,
//! A's freeze adds the quoting, A's envelope adds the selection and the join,
//! and B and D add the lane and strategy variations."
//!
//! Every fixture under `tests/vectors/` is the spec's own block, copied byte
//! for byte — with one exception the spec forces: EV §11 prints vector D's
//! seal and its counts but not its whole message, so `d-message.txt` is the
//! derivation §11 states, and
//! [`vector_d_is_exactly_vector_a_with_the_manifest_and_the_strategy_moved`]
//! rebuilds it from the other three fixtures and asserts byte equality. Nothing here asserts a digest that is not recomputed from those
//! bytes in the same test — including the two **wrong** values EV publishes
//! "so a mis-implementation recognises itself" (EV §8.2), which are asserted
//! against a deliberate trailing-LF join computed here.

use spine_envelope::digest::{
    above_seal, freeze_cmp, freeze_digest, freeze_digest_over, freeze_lines, join_lf, lines,
};
use spine_envelope::message::Shape;
use spine_envelope::payload::{Frozen, Review, Seal};
use spine_envelope::trailer::TrailerName;
use spine_envelope::{Envelope, envelope_digest};

const A_MESSAGE: &[u8] = include_bytes!("vectors/a-message.txt");
const A_APPROVAL: &[u8] = include_bytes!("vectors/a-approval-commit.txt");
const B_MESSAGE: &[u8] = include_bytes!("vectors/b-message.txt");
const C_AUTHORED: &[u8] = include_bytes!("vectors/c-authored.txt");
const C_SORTED: &[u8] = include_bytes!("vectors/c-sorted.txt");
const D_MESSAGE: &[u8] = include_bytes!("vectors/d-message.txt");
/// EV §11's seal and its `-Sig`, "resealed over the new digest".
const D_SEAL: &[u8] = include_bytes!("vectors/d-seal.txt");

/// The digest of a join that wrongly terminates its last line. EV publishes one
/// for `freeze=` and one for `envelope=`; both are computed here rather than
/// transcribed, so a change to the join shows up as two failures and not one.
fn digest_with_trailing_lf(block: &[&[u8]]) -> String {
    let mut joined = join_lf(block);
    joined.push(b'\n');
    spine_canon::sha256_prefixed(&joined)
}

// ---------------------------------------------------------------------------
// Vector C — the `freeze=` sort (EV §10)
// ---------------------------------------------------------------------------

#[test]
fn vector_c_sorts_and_digests() {
    let mut authored = lines(C_AUTHORED);
    let expected = lines(C_SORTED);
    authored.sort_by(|a, b| freeze_cmp(a, b));
    assert_eq!(authored, expected, "EV §10's sorted block");

    let joined = join_lf(&expected);
    assert_eq!(joined.len(), 382, "six lines joined by five 0x0A");
    assert_eq!(
        freeze_digest_over(&authored),
        "sha256:bbf3ba10080d190a1ba224483f4ad760083efa861d073f7a0d5f16df92bf45d4"
    );
}

#[test]
fn vector_c_pins_frozen_before_test_from_the_whole_line_comparison() {
    let sorted = lines(C_SORTED);
    let frozen = format!("{}: ", TrailerName::Frozen);
    let split = sorted
        .iter()
        .position(|l| !l.starts_with(frozen.as_bytes()))
        .unwrap();
    assert_eq!(split, 3, "three Spine-Frozen lines, then two Spine-Test");
    assert!(
        sorted[split..]
            .iter()
            .all(|l| l.starts_with(b"Spine-Test: "))
    );
}

#[test]
fn vector_c_pins_byte_order_and_not_numeric_order() {
    // EV §10: "`AC10` precedes `AC2`. An implementation that sorts ids
    // 'naturally' produces a different digest over identical facts."
    let sorted = lines(C_SORTED);
    let ten = sorted.iter().position(|l| l.ends_with(b"AC10 rounding")).unwrap();
    let two = sorted.iter().position(|l| l.ends_with(b"AC2 zero-rated")).unwrap();
    assert!(ten < two);
    // And runner-major within Spine-Test: pytest before vitest, for every id.
    let pytest = sorted.iter().position(|l| l.starts_with(b"Spine-Test: pytest")).unwrap();
    assert!(pytest < ten);
}

#[test]
fn vector_c_pins_the_two_frozen_tie_breaks() {
    let sorted = lines(C_SORTED);
    // "the quoted `café.py` first *because of its oid*, not because `\"` sorts
    // low" (EV §10).
    let first = Frozen::parse(&sorted[0][b"Spine-Frozen: ".len()..]).unwrap();
    assert_eq!(first.oid, "0a12f7d3e5b96c08a41d7e2f39c05b6a8d14e037");
    assert_eq!(first.path, "tests/café.py".as_bytes().to_vec());
    // "the path breaks the tie when two blobs are identical".
    let second = Frozen::parse(&sorted[1][b"Spine-Frozen: ".len()..]).unwrap();
    let third = Frozen::parse(&sorted[2][b"Spine-Frozen: ".len()..]).unwrap();
    assert_eq!(second.oid, third.oid);
    assert_eq!(second.path, b"tests/a b.py".to_vec());
    assert_eq!(third.path, b"tests/z.py".to_vec());
}

// ---------------------------------------------------------------------------
// Vector A — a gated, merge-strategy landing (EV §8)
// ---------------------------------------------------------------------------

#[test]
fn vector_a_freeze_from_the_approval_commit() {
    let all = lines(A_APPROVAL);
    let manifest = freeze_lines(&all);
    assert_eq!(manifest.len(), 7, "five Spine-Frozen and two Spine-Test");
    assert_eq!(join_lf(&manifest).len(), 573, "EV §8.2: 573, not 580");

    assert_eq!(
        freeze_digest(A_APPROVAL),
        "sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2"
    );
    assert_eq!(
        digest_with_trailing_lf(&manifest),
        "sha256:8262e6d9f2a911d564bd706c0c8adb33a271fc6a467fc1dd75a5c91f3009e19c",
        "EV §8.2's published wrong value: the trailing-LF error"
    );
}

#[test]
fn the_frozen_manifest_is_ordered_by_blob_id_and_not_by_path() {
    // EV §13.5, accepted rather than patched: a whole-line sort is oid-major.
    let all = lines(A_APPROVAL);
    let frozen: Vec<Frozen> = freeze_lines(&all)
        .iter()
        .filter(|l| l.starts_with(b"Spine-Frozen: "))
        .map(|l| Frozen::parse(&l[b"Spine-Frozen: ".len()..]).unwrap())
        .collect();
    assert_eq!(frozen.len(), 5);
    let oids: Vec<&str> = frozen.iter().map(|f| f.oid.as_str()).collect();
    let mut sorted_oids = oids.clone();
    sorted_oids.sort_unstable();
    assert_eq!(oids, sorted_oids, "ascending by blob id");

    let paths: Vec<&[u8]> = frozen.iter().map(|f| f.path.as_slice()).collect();
    let mut sorted_paths = paths.clone();
    sorted_paths.sort_unstable();
    assert_ne!(paths, sorted_paths, "EV §8.2 point 1: not the path order");
    assert_eq!(frozen[0].path, "tests/fixtures/café.json".as_bytes().to_vec());
    assert_eq!(frozen[4].path, b"tests/setup.ts".to_vec());
}

#[test]
fn vector_a_envelope_over_the_fifteen_lines_above_the_seal() {
    let block = above_seal(&lines(A_MESSAGE));
    assert_eq!(block.len(), 15, "EV §8.3: fifteen lines");
    assert_eq!(join_lf(&block).len(), 2379, "fourteen separators");
    assert_eq!(
        envelope_digest(A_MESSAGE).unwrap(),
        "sha256:e1652897b251b001fe7e03e343d40bbdc7fb9b112ef920c8b53987916b14682f"
    );
    assert_eq!(
        digest_with_trailing_lf(&block),
        "sha256:a0c024c23ffad492901a72ee2fa48537793b64d0a74506b33dfccd070f5e0ac5",
        "EV §8.3's published wrong value"
    );
}

#[test]
fn vector_a_includes_its_sig_lines_and_excludes_the_fence_the_subject_and_the_seal() {
    let block = above_seal(&lines(A_MESSAGE));
    // EV §8.3 point 1: "The five `-Sig` lines are inside. Remove them and the
    // digest changes."
    //
    // **The count in that sentence is wrong and this is the corrected one.**
    // Vector A's above-seal block carries four `-Sig` lines — signoff, reopen,
    // approve, review. Five is the number of `-Sig` *names* EV §3.2 question 1
    // rules inside the digest (those four plus `Spine-Upgrade-Sig`, which this
    // landing does not carry); `Spine-Seal-Sig` is excluded by point 4 of this
    // very list. The digest is unaffected — nothing counts these lines — so the
    // sentence is a defect and not a divergence.
    let sigs = block
        .iter()
        .filter(|l| {
            let name = &l[..l.iter().position(|&b| b == b':').unwrap()];
            TrailerName::parse(name).is_some_and(TrailerName::is_sig)
        })
        .count();
    assert_eq!(sigs, 4);
    let without: Vec<&[u8]> = block
        .iter()
        .filter(|l| {
            let name = &l[..l.iter().position(|&b| b == b':').unwrap()];
            !TrailerName::parse(name).is_some_and(TrailerName::is_sig)
        })
        .copied()
        .collect();
    assert_ne!(
        spine_canon::sha256_prefixed(&join_lf(&without)),
        envelope_digest(A_MESSAGE).unwrap()
    );

    // Points 2, 3 and 4: no line of the fence, the subject or the seal begins
    // `Spine-`, above the boundary.
    assert!(block.iter().all(|l| l.starts_with(b"Spine-")));
    assert!(!block.iter().any(|l| l.starts_with(b"Spine-Seal")));
    assert!(!block.iter().any(|l| l.starts_with(b"-----")));
}

#[test]
fn vector_a_carries_no_frozen_or_test_line_under_merge_strategy() {
    // EV §8.3 point 7: "PB §11 confines them to squash."
    assert!(freeze_lines(&lines(A_MESSAGE)).is_empty());
}

#[test]
fn vector_a_signs_the_byte_order_of_wires_and_not_the_numeric_one() {
    // EV §8.3 point 5, and PB §11's `Spine-Review` row verbatim.
    let env = Envelope::parse(A_MESSAGE).unwrap();
    let review = Review::parse(env.first(TrailerName::Review).unwrap()).unwrap();
    assert_eq!(review.wires, vec!["G11", "G2:src/shared/util.ts"]);
    assert!(
        env.message()
            .windows(31)
            .any(|w| w == b"wires=G11,G2:src/shared/util.ts")
    );
}

#[test]
fn vector_a_message_is_forty_three_lines_and_4031_bytes() {
    assert_eq!(A_MESSAGE.len(), 4031, "EV §8.5");
    assert_eq!(lines(A_MESSAGE).len(), 43);
    assert_eq!(*A_MESSAGE.last().unwrap(), b'\n');
    assert_ne!(A_MESSAGE[A_MESSAGE.len() - 2], b'\n', "no trailing blank line");
}

#[test]
fn vector_a_fence_reproduces_its_blob_and_its_byte_count() {
    let env = Envelope::parse(A_MESSAGE).unwrap();
    let fence = env.fence().unwrap();
    assert_eq!(fence.bytes, 765, "EV §2.6: bytes, not characters");
    assert_eq!(fence.body.len(), 765);
    assert_eq!(
        String::from_utf8(fence.body.clone()).unwrap().chars().count(),
        762,
        "the three-character difference is three U+00B7, two bytes each"
    );
    assert_eq!(fence.blob, "dfb4079e22de55ec377468b9b697fdf86085ea37");
    assert_eq!(
        spine_canon::git_blob_id(&fence.body, spine_canon::ObjectFormat::Sha1),
        fence.blob,
        "git hash-object over exactly those bytes"
    );
}

#[test]
fn vector_a_is_a_gated_landing_whose_subject_is_derived_from_the_fence() {
    let env = Envelope::parse(A_MESSAGE).unwrap();
    assert_eq!(env.shape().unwrap(), Shape::Gated);
    assert_eq!(
        env.derive_subject().unwrap().unwrap(),
        b"INT-042: Invoice totals include tax".to_vec()
    );
    env.check_subject().unwrap();
    env.check_envelope_digest().unwrap();
}

#[test]
fn vector_a_capped_quantity_is_4031_of_16384() {
    let env = Envelope::parse(A_MESSAGE).unwrap();
    assert_eq!(env.capped_quantity(), 4031);
    assert_eq!(env.capped_quantity(), A_MESSAGE.len(), "no manifest to exclude");
    env.check_cap().unwrap();
}

#[test]
fn vector_a_seals_two_different_reports_and_two_different_trees() {
    // EV §8.4: "An implementation that puts one digest in both places has
    // collapsed two evaluations into one."
    let env = Envelope::parse(A_MESSAGE).unwrap();
    let seal = Seal::parse(env.first(TrailerName::Seal).unwrap()).unwrap();
    let review = Review::parse(env.first(TrailerName::Review).unwrap()).unwrap();
    assert_ne!(seal.report, review.report);
    assert_ne!(seal.tree, review.tree, "L's tree is T minus the intent file");
    assert_eq!(seal.head, review.head, "both name the content head Hc");
}

// ---------------------------------------------------------------------------
// Vector B — a quick-lane landing (EV §9)
// ---------------------------------------------------------------------------

#[test]
fn vector_b_envelope_over_seven_lines() {
    let block = above_seal(&lines(B_MESSAGE));
    assert_eq!(block.len(), 7);
    assert_eq!(join_lf(&block).len(), 859);
    assert_eq!(
        envelope_digest(B_MESSAGE).unwrap(),
        "sha256:9764852ed4bd33a9eb42ca0674b88195f03eeac20df829b6e845175a449be44d"
    );
    assert_eq!(B_MESSAGE.len(), 1636, "EV §9: eleven lines, 1636 bytes");
    assert_eq!(lines(B_MESSAGE).len(), 11);
}

#[test]
fn vector_b_is_a_quick_landing_whose_subject_is_checked_only_for_its_prefix() {
    let env = Envelope::parse(B_MESSAGE).unwrap();
    assert_eq!(env.shape().unwrap(), Shape::Quick);
    assert!(env.fence().is_none());
    assert!(
        env.derive_subject().unwrap().is_none(),
        "EV §13.10: the summary is free text"
    );
    env.check_subject().unwrap();
    env.check_envelope_digest().unwrap();
}

#[test]
fn vector_b_lists_eleven_gates_and_takes_a_protected_review() {
    let env = Envelope::parse(B_MESSAGE).unwrap();
    // EV §13.7: G3, G4 and G12 read an in-flight intent or an approval and a
    // quick landing has neither; G6 and G10 never appear.
    let gates = env.gates().unwrap();
    assert_eq!(gates.0.len(), 11);
    assert!(gates.0.iter().all(|&(n, _)| ![3, 4, 6, 10, 12].contains(&n)));

    let review = Review::parse(env.first(TrailerName::Review).unwrap()).unwrap();
    // EV §9 point 4: the signerless overlay "is evaluated after aggregation and
    // only ever raises", so the class is protected although the only wire is a
    // tripwire-class G11 advisory.
    assert_eq!(review.class, spine_envelope::payload::ReviewClass::Protected);
    assert_eq!(review.wires, vec!["G11"]);
    // Point 1: the first field is `quick`, and `intent=` is absent.
    assert_eq!(review.subject, "quick");
    assert!(review.intent.is_none());

    // Point 2: "The seal's `tree=` equals the review's `tree=` here."
    let seal = Seal::parse(env.first(TrailerName::Seal).unwrap()).unwrap();
    assert_eq!(seal.tree, review.tree);
    assert_eq!(seal.mode, spine_envelope::payload::Mode::Solo);
    assert_eq!(seal.profile, spine_envelope::payload::Profile::None);
}

// ---------------------------------------------------------------------------
// Vector D — the same landing under squash (EV §11)
// ---------------------------------------------------------------------------

#[test]
fn vector_d_envelope_over_twenty_two_lines() {
    let block = above_seal(&lines(D_MESSAGE));
    assert_eq!(block.len(), 22);
    assert_eq!(join_lf(&block).len(), 2954);
    assert_eq!(
        envelope_digest(D_MESSAGE).unwrap(),
        "sha256:9895816bcbc90400ac90cec50bbb6eec516e26712097c4eef1877ea739bfcc4b"
    );
    assert_eq!(D_MESSAGE.len(), 4606);
    assert_eq!(lines(D_MESSAGE).len(), 50);
}

#[test]
fn the_manifest_lines_are_inside_the_digest_and_outside_the_cap() {
    // EV §11 points 1 and 2 — "two different subsets of one trailer block, and
    // both are correct".
    let block = above_seal(&lines(D_MESSAGE));
    assert_eq!(freeze_lines(&block).len(), 7, "inside envelope=");
    assert_ne!(
        envelope_digest(D_MESSAGE).unwrap(),
        envelope_digest(A_MESSAGE).unwrap(),
        "an implementation that carved them out would produce vector A's digest"
    );

    let env = Envelope::parse(D_MESSAGE).unwrap();
    assert_eq!(env.capped_quantity(), 4032);
    assert_eq!(
        D_MESSAGE.len() - env.capped_quantity(),
        574,
        "567 manifest bytes plus seven terminators"
    );
    env.check_cap().unwrap();
}

#[test]
fn vector_d_freeze_recomputes_from_the_envelope_alone() {
    // EV §11 point 3: "That is PB §6.3 G9's squash freeze audit, executed."
    assert_eq!(
        freeze_digest(D_MESSAGE),
        freeze_digest(A_APPROVAL),
        "the same seven lines, from the landing and from the approval commit"
    );
    assert_eq!(
        freeze_digest(D_MESSAGE),
        "sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2"
    );
}

#[test]
fn vector_d_keeps_spine_approval_and_moves_only_the_strategy() {
    let env = Envelope::parse(D_MESSAGE).unwrap();
    assert_eq!(env.strategy().unwrap(), spine_envelope::payload::Strategy::Squash);
    // EV §11 point 4 / §13.6: present under both strategies.
    assert!(env.first(TrailerName::Approval).is_some());
    env.check_subject().unwrap();
    env.check_envelope_digest().unwrap();

    // "one byte more, because `squash` is one byte longer than `merge`"
    let a = Envelope::parse(A_MESSAGE).unwrap();
    assert_eq!(env.capped_quantity(), a.capped_quantity() + 1);
}

#[test]
fn the_squash_manifest_is_emitted_in_freeze_order_between_approval_and_review() {
    // EV §11: "inserted at ranks 11 and 12, immediately after `Spine-Approval`
    // and before `Spine-Review`, in the same `freeze=` sort order".
    let env = Envelope::parse(D_MESSAGE).unwrap();
    let names: Vec<TrailerName> = env.trailers().iter().map(|(n, _)| *n).collect();
    let approval = names.iter().position(|n| *n == TrailerName::Approval).unwrap();
    let review = names.iter().position(|n| *n == TrailerName::Review).unwrap();
    let first_frozen = names.iter().position(|n| *n == TrailerName::Frozen).unwrap();
    let last_test = names.iter().rposition(|n| *n == TrailerName::Test).unwrap();
    assert!(approval < first_frozen && last_test < review);
}

#[test]
fn vector_d_is_exactly_vector_a_with_the_manifest_and_the_strategy_moved() {
    // EV §11 states the derivation in two clauses, and this is both of them:
    // "the seven manifest lines of §8.2 inserted at ranks 11 and 12, immediately
    // after `Spine-Approval` and before `Spine-Review`, in the same `freeze=`
    // sort order they carry on the approval commit" and "`Spine-Strategy: merge`
    // becomes `Spine-Strategy: squash`" — plus §11's own reprinted seal.
    let manifest = freeze_lines(&lines(A_APPROVAL));
    let mut built: Vec<&[u8]> = Vec::new();
    for line in lines(A_MESSAGE) {
        if line.starts_with(b"Spine-Review: ") {
            built.extend_from_slice(&manifest);
        }
        if line == b"Spine-Strategy: merge" {
            built.push(b"Spine-Strategy: squash");
            continue;
        }
        if line.starts_with(b"Spine-Seal: ") {
            built.extend(lines(D_SEAL));
            break;
        }
        built.push(line);
    }
    let mut bytes = join_lf(&built);
    bytes.push(b'\n');
    assert_eq!(bytes, D_MESSAGE.to_vec());
}
