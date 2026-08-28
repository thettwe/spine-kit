//! EV §12's refusal table, one test per code, over envelopes built here.
//!
//! "At `--land`, every one of these is a refusal before anything is sealed. On
//! an already-landed commit, every one makes G9 index the landing `unattested`
//! — reported and counted forever, never silently repaired" (EV §12). So each
//! test asserts the *token*, not merely that something failed: the token is
//! what reaches the record.

use spine_envelope::digest::check_no_duplicates;
use spine_envelope::message::Shape;
use spine_envelope::payload::Test;
use spine_envelope::{CAP, Envelope, Refusal};

const A_MESSAGE: &[u8] = include_bytes!("vectors/a-message.txt");

const OID_A: &str = "7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51";
const OID_B: &str = "77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9";
const OID_C: &str = "8b47d0e6c2a915f37e04b8d1c6a2f905e37b1d48";
const D256: &str = "sha256:e70a3c92d1b845f6027e9ab3c5d10f684a2b7e93c60d5f81a34b0e29d7c6f105";

fn seal(subject: &str) -> String {
    format!(
        "Spine-Seal: {subject} base={OID_A} head={OID_B} tree={OID_C} report={D256} \
         tool=1.4.0+{D256} git=2.45 mode=team threat=hostile profile=container envelope={D256} \
         signer=ci@example.com\nSpine-Seal-Sig: AAAA\n"
    )
}

/// A reseal: `Spine-Event: reseal`, on the quick lane, with a review whose
/// `reason=` is as long as the caller asks for. PB §5.5 makes a reseal "a
/// quick-lane landing with `Spine-Event: reseal`".
fn reseal(reason_len: usize) -> Vec<u8> {
    let mut m = format!("reseal: {OID_B}\n\n");
    m.push_str("Spine-Envelope: 1\nSpine-Event: reseal\nSpine-Lane: quick\n");
    m.push_str(&format!(
        "Spine-Review: reseal class=protected head={OID_B} tree={OID_C} base={OID_A} \
         report={D256} wires=G11 reason=\"{}\" reviewer=bob@example.com\nSpine-Review-Sig: AAAA\n",
        "x".repeat(reason_len)
    ));
    m.push_str("Spine-Gates: G1=pass G2=pass G5=pass G7=pass G8=pass G9=pass G11=pass G13=pass G14=pass G15=pass G16=pass\n");
    m.push_str("Spine-Strategy: squash\n");
    m.push_str(&seal("reseal"));
    m.into_bytes()
}

#[test]
fn envelope_version_unknown_refuses_before_computing_anything() {
    let m = String::from_utf8(A_MESSAGE.to_vec())
        .unwrap()
        .replace("Spine-Envelope: 1", "Spine-Envelope: 2");
    let e = Envelope::parse(m.as_bytes()).unwrap_err();
    assert_eq!(e.refusal(), Refusal::EnvelopeVersionUnknown);
}

#[test]
fn fence_mismatch_on_a_blob_that_does_not_reproduce() {
    // EV §12: "the fenced bytes do not hash to `blob=`, or their count is not
    // `bytes=`."
    let m = String::from_utf8(A_MESSAGE.to_vec())
        .unwrap()
        // A same-length edit inside the fenced body: `bytes=` still lands on
        // the END line, so only `blob=` can catch it.
        .replace("- Multi-currency invoices.", "- Multi-currency invoiceX.");
    let e = Envelope::parse(m.as_bytes()).unwrap_err();
    assert_eq!(e.refusal(), Refusal::FenceMismatch);
}

#[test]
fn fence_mismatch_when_the_byte_count_does_not_land_on_the_end_line() {
    let m = String::from_utf8(A_MESSAGE.to_vec())
        .unwrap()
        .replace("bytes=765-----", "bytes=760-----");
    let e = Envelope::parse(m.as_bytes()).unwrap_err();
    assert_eq!(e.refusal(), Refusal::FenceMismatch);
}

#[test]
fn subject_mismatch_when_a_gated_subject_is_not_the_derived_line() {
    let m = String::from_utf8(A_MESSAGE.to_vec())
        .unwrap()
        .replacen(
            "INT-042: Invoice totals include tax\n\n-----BEGIN",
            "INT-042: invoice totals include tax\n\n-----BEGIN",
            1,
        );
    let env = Envelope::parse(m.as_bytes()).unwrap();
    assert_eq!(
        env.check_subject().unwrap_err().refusal(),
        Refusal::SubjectMismatch
    );
}

#[test]
fn a_reseal_subject_carries_the_full_object_id() {
    // EV §13.10: an abbreviation "would make a *derived* subject a function of
    // the reader's configuration and break §7 rule 1 on the one shape whose
    // subject is fully computable."
    let env = Envelope::parse(&reseal(4)).unwrap();
    assert_eq!(env.shape().unwrap(), Shape::Reseal);
    assert_eq!(
        env.derive_subject().unwrap().unwrap(),
        format!("reseal: {OID_B}").into_bytes()
    );
    env.check_subject().unwrap();

    let short = String::from_utf8(reseal(4))
        .unwrap()
        .replacen(&format!("reseal: {OID_B}"), "reseal: 77aa3c1", 1);
    assert_eq!(
        Envelope::parse(short.as_bytes())
            .unwrap()
            .check_subject()
            .unwrap_err()
            .refusal(),
        Refusal::SubjectMismatch
    );
}

#[test]
fn digest_mismatch_when_one_byte_above_the_seal_moved() {
    // EV §3.4's last row: "The recomputed digest differs from the seal's
    // `envelope=`; G9 indexes the landing `unattested`. This is the case the
    // digest exists for."
    let m = String::from_utf8(A_MESSAGE.to_vec())
        .unwrap()
        .replace("Spine-Gates: G1=pass", "Spine-Gates: G1=override");
    let env = Envelope::parse(m.as_bytes()).unwrap();
    assert_eq!(
        env.check_envelope_digest().unwrap_err().refusal(),
        Refusal::DigestMismatch
    );
}

#[test]
fn freeze_duplicate_when_one_pair_is_named_twice() {
    let l: &[u8] = b"Spine-Test: vitest a > b";
    assert_eq!(
        check_no_duplicates(&[l, l]).unwrap_err().refusal(),
        Refusal::FreezeDuplicate
    );
}

#[test]
fn test_id_unrepresentable_rather_than_mangled() {
    for byte in [0x0Au8, 0x0D, 0x00] {
        let id = [b'a', byte, b'b'];
        assert_eq!(
            Test::check_representable(&id).unwrap_err().refusal(),
            Refusal::TestIdUnrepresentable
        );
    }
}

#[test]
fn envelope_too_large_above_the_cap() {
    let big = reseal(0).len();
    let m = String::from_utf8(reseal(CAP - big + 64)).unwrap();
    // Same bytes, but a shape the cap governs.
    let capped = m.replace("Spine-Event: reseal", "Spine-Event: land")
        .replace("Spine-Review: reseal ", "Spine-Review: quick ")
        .replace("Spine-Seal: reseal ", "Spine-Seal: quick ")
        .replacen(&format!("reseal: {OID_B}"), "quick: a long one", 1);
    let env = Envelope::parse(capped.as_bytes()).unwrap();
    assert!(env.capped_quantity() > CAP);
    assert_eq!(
        env.check_cap().unwrap_err().refusal(),
        Refusal::EnvelopeTooLarge
    );
}

#[test]
fn a_reseal_is_not_measured_against_the_cap_at_all() {
    // EV §2.9, the owner's decision of 2026-08-26: "for that shape no cap is
    // evaluated … and `envelope-too-large` (§12) is never raised" — because a
    // reseal has no exit from it and G9 refuses to land on top of an unresealed
    // orphan, so a capped reseal would leave trunk permanently unlandable.
    let m = reseal(CAP);
    let env = Envelope::parse(&m).unwrap();
    assert!(env.capped_quantity() > CAP);
    env.check_cap().unwrap();
}

#[test]
fn a_reseal_carries_no_fenced_intent_and_no_signoff() {
    let env = Envelope::parse(&reseal(4)).unwrap();
    assert!(env.fence().is_none());
    assert!(
        env.first(spine_envelope::TrailerName::Signoff).is_none(),
        "a reseal is not an intent and has no signer (PB §5.5)"
    );
}

#[test]
fn a_frozen_path_that_is_badly_quoted_is_envelope_malformed() {
    // EV §4.3: "unterminated, a bad escape, a trailing byte after the closing
    // quote".
    let m = String::from_utf8(A_MESSAGE.to_vec())
        .unwrap()
        .replace("Spine-Review: INT-042", "Spine-Frozen: 0c3a7f18e2b56d94a0c7f3e18b52d6a4907c1e3f \"a\\q\"\nSpine-Review: INT-042");
    let e = Envelope::parse(m.as_bytes()).unwrap_err();
    assert_eq!(e.refusal(), Refusal::EnvelopeMalformed);
}

#[test]
fn a_manifest_out_of_freeze_order_is_envelope_malformed() {
    // EV §18 item 7 and §2.4: emission order is the digest's order, so a reader
    // "can check the order as well as the value".
    let a = "Spine-Frozen: 0a12f7d3e5b96c08a41d7e2f39c05b6a8d14e037 tests/a.py";
    let b = "Spine-Frozen: 7f3aa0c19b48d6250e3f7a1c85b09d24e6f31a70 tests/z.py";
    let ordered = String::from_utf8(A_MESSAGE.to_vec())
        .unwrap()
        .replace("Spine-Review: INT-042", &format!("{a}\n{b}\nSpine-Review: INT-042"));
    Envelope::parse(ordered.as_bytes()).unwrap();

    let reversed = String::from_utf8(A_MESSAGE.to_vec())
        .unwrap()
        .replace("Spine-Review: INT-042", &format!("{b}\n{a}\nSpine-Review: INT-042"));
    assert_eq!(
        Envelope::parse(reversed.as_bytes()).unwrap_err().refusal(),
        Refusal::EnvelopeMalformed
    );
}

#[test]
fn a_sig_payload_that_is_not_one_base64_run_is_envelope_malformed() {
    // EV §18 item 15.
    let m = String::from_utf8(A_MESSAGE.to_vec())
        .unwrap()
        .replacen("Spine-Seal-Sig: U1NIU0lH", "Spine-Seal-Sig: -----BEGIN", 1);
    assert_eq!(
        Envelope::parse(m.as_bytes()).unwrap_err().refusal(),
        Refusal::EnvelopeMalformed
    );
}
