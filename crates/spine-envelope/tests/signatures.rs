//! Every signature `envelope-vectors.md` publishes, verified the way PB §11
//! says a human verifies one — `ssh-keygen -Y verify`, no spine binary in the
//! loop, no ed25519 arithmetic of our own.
//!
//! EV §8.1 publishes three throwaway ed25519 public keys and no private key, so
//! a broken signature here can only mean this crate mis-assembled the signed
//! bytes: the armor, the namespace, the principal, or — the one that matters —
//! the **signed byte range**. EV §13.8 fixes that range as the whole line,
//! trailer name included, terminator excluded, "or a `Spine-Approve` payload
//! could be replayed as a `Spine-Signoff` with the same signature."

use spine_envelope::trailer::{TrailerName, parse_line};
use spine_envelope::verify::{Namespace, VerifyError, armor, expected_namespace, strip_armor};
use spine_envelope::{Envelope, verify_line};

const A_MESSAGE: &[u8] = include_bytes!("vectors/a-message.txt");
const A_APPROVAL: &[u8] = include_bytes!("vectors/a-approval-commit.txt");
const B_MESSAGE: &[u8] = include_bytes!("vectors/b-message.txt");
const D_MESSAGE: &[u8] = include_bytes!("vectors/d-message.txt");
const TEAM_KEYRING: &[u8] = include_bytes!("vectors/allowed_signers");
const SOLO_KEYRING: &[u8] = include_bytes!("vectors/allowed_signers.solo");

/// The statement line and its `-Sig` payload, located the way EV §8.6's shell
/// function does: the line whose name is `<Name>`, and the one whose name is
/// `<Name>-Sig`.
fn statement(message: &[u8], name: TrailerName) -> (&[u8], String) {
    let sig_name = name.sig().expect("a signed statement");
    let mut line = None;
    let mut sig = None;
    for l in message.split(|&b| b == b'\n') {
        if l.is_empty() {
            continue;
        }
        let Ok(t) = parse_line(l) else { continue };
        if t.name == name && line.is_none() {
            line = Some(l);
        } else if t.name == sig_name && sig.is_none() {
            sig = Some(String::from_utf8(t.payload.to_vec()).expect("base64 is ASCII"));
        }
    }
    (line.expect("statement line"), sig.expect("-Sig line"))
}

fn check(message: &[u8], keyring: &[u8], name: TrailerName, principal: &str, ns: Namespace) {
    let (line, sig) = statement(message, name);
    assert_eq!(
        expected_namespace(name, parse_line(line).unwrap().payload).unwrap(),
        Some(ns),
        "{name}'s namespace is decided by its payload, not by this test"
    );
    match verify_line(line, &sig, principal, ns, keyring) {
        Ok(v) => {
            assert_eq!(v.namespace, ns);
            assert!(
                v.key_fingerprint.starts_with("SHA256:"),
                "PB §7.2 compares fingerprints, so one must come back"
            );
        }
        Err(VerifyError::OpenSshUnavailable(e)) => panic!(
            "PB §11's Git requirements make OpenSSH >= 8.2 a prerequisite: {e}"
        ),
        Err(e) => panic!("{name} did not verify: {e}"),
    }
}

#[test]
fn vector_a_all_five_signatures_verify() {
    // EV §8.6's five `verify` invocations, one for one.
    check(A_MESSAGE, TEAM_KEYRING, TrailerName::Signoff, "alice@example.com", Namespace::Signoff);
    check(A_MESSAGE, TEAM_KEYRING, TrailerName::Reopen, "alice@example.com", Namespace::Signoff);
    check(A_MESSAGE, TEAM_KEYRING, TrailerName::Approve, "alice@example.com", Namespace::Review);
    check(A_MESSAGE, TEAM_KEYRING, TrailerName::Review, "bob@example.com", Namespace::Review);
    check(A_MESSAGE, TEAM_KEYRING, TrailerName::Seal, "ci@example.com", Namespace::Seal);
}

#[test]
fn the_approval_commits_own_approve_line_verifies() {
    // EV §8.2: "the `Spine-Approve` line here is the line copied verbatim into
    // the envelope", so the same signature must verify on both commits.
    check(A_APPROVAL, TEAM_KEYRING, TrailerName::Approve, "alice@example.com", Namespace::Review);
    let (from_approval, sig_a) = statement(A_APPROVAL, TrailerName::Approve);
    let (from_landing, sig_l) = statement(A_MESSAGE, TrailerName::Approve);
    assert_eq!(from_approval, from_landing, "copied verbatim");
    assert_eq!(sig_a, sig_l);
}

#[test]
fn vector_b_both_signatures_verify_against_the_solo_keyring() {
    // EV §9: solo mode, "one signoff key, whose principal then holds all three
    // namespaces (PB §11)".
    check(B_MESSAGE, SOLO_KEYRING, TrailerName::Review, "alice@example.com", Namespace::Review);
    check(B_MESSAGE, SOLO_KEYRING, TrailerName::Seal, "alice@example.com", Namespace::Seal);
}

#[test]
fn vector_ds_reseal_of_the_same_landing_verifies() {
    // EV §11: the seal was "resealed over the new digest", so it is a different
    // line and a different signature from vector A's.
    check(D_MESSAGE, TEAM_KEYRING, TrailerName::Seal, "ci@example.com", Namespace::Seal);
    assert_ne!(
        statement(D_MESSAGE, TrailerName::Seal).1,
        statement(A_MESSAGE, TrailerName::Seal).1
    );
}

#[test]
fn the_signed_range_includes_the_trailer_name() {
    // EV §13.8's reason, executed: signing only the payload after `: ` would
    // let one statement be replayed as another. Strip the name and the same
    // signature must stop verifying.
    let (line, sig) = statement(A_MESSAGE, TrailerName::Signoff);
    let payload = parse_line(line).unwrap().payload;
    assert!(
        verify_line(payload, &sig, "alice@example.com", Namespace::Signoff, TEAM_KEYRING).is_err()
    );
}

#[test]
fn a_signature_does_not_verify_under_another_namespace() {
    // This is what makes the namespace, and only the namespace, the source of a
    // role (DM §7.2): "the namespace the signature verified under, never a
    // claim in the trailer".
    let (line, sig) = statement(A_MESSAGE, TrailerName::Seal);
    assert!(
        verify_line(line, &sig, "ci@example.com", Namespace::Review, TEAM_KEYRING).is_err(),
        "ci@example.com holds spine-seal@v1 alone"
    );
}

#[test]
fn a_signature_does_not_verify_for_another_principal() {
    let (line, sig) = statement(A_MESSAGE, TrailerName::Review);
    assert!(
        verify_line(line, &sig, "alice@example.com", Namespace::Review, TEAM_KEYRING).is_err(),
        "bob signed it; PB §7.2's team mode binds reviewer != signer on fingerprints"
    );
}

#[test]
fn one_edited_byte_breaks_the_signature() {
    // PB §5.5: "Nothing is trusted because it says so; everything is trusted
    // because it hashes."
    let (line, sig) = statement(A_MESSAGE, TrailerName::Signoff);
    let mut tampered = line.to_vec();
    let n = tampered.len();
    tampered[n - 1] = b'M';
    assert!(
        verify_line(&tampered, &sig, "alice@example.com", Namespace::Signoff, TEAM_KEYRING)
            .is_err()
    );
}

#[test]
fn the_armor_round_trip_is_byte_identical_to_what_ssh_keygen_writes() {
    // EV §2.7: "the base64 wrapped at **70 characters per line** … This
    // round-trip is byte-identical to what `ssh-keygen -Y sign` writes."
    for name in [
        TrailerName::Signoff,
        TrailerName::Reopen,
        TrailerName::Approve,
        TrailerName::Review,
        TrailerName::Seal,
    ] {
        let (_, sig) = statement(A_MESSAGE, name);
        assert!(!sig.contains('\n'), "a -Sig payload is one unbroken run");
        let armored = armor(&sig);
        assert_eq!(strip_armor(&armored), sig);
        let body: Vec<&str> = armored.lines().collect();
        assert_eq!(body[0], "-----BEGIN SSH SIGNATURE-----");
        assert_eq!(*body.last().unwrap(), "-----END SSH SIGNATURE-----");
        assert!(body[1..body.len() - 1].iter().all(|l| l.len() <= 70));
    }
}

#[test]
fn every_landing_in_the_corpus_parses_and_recomputes_its_own_seal() {
    for message in [A_MESSAGE, B_MESSAGE, D_MESSAGE] {
        let env = Envelope::parse(message).unwrap();
        env.check_envelope_digest().unwrap();
        env.check_subject().unwrap();
        env.check_cap().unwrap();
    }
}
