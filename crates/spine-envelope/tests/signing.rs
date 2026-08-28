//! Signing with a **real key**, verified by the path that already existed.
//!
//! The round trip is the whole point: `sign_line` produces what `verify_line`
//! accepts, over the byte range EV §2.7 fixes, under the namespace PB §7.2
//! names. Nothing here mocks `ssh-keygen` — a signature format is exactly the
//! thing a mock would get wrong.

use std::path::{Path, PathBuf};
use std::process::Command;

use spine_envelope::sign::{Key, ssh_keygen_available};
use spine_envelope::{Namespace, TrailerName, sig_line, sign_line, verify_line};

const PRINCIPAL: &str = "alice@example.com";

struct Keys(PathBuf);

impl Keys {
    /// One ed25519 key, and the `allowed_signers` line naming it under every
    /// namespace a test needs.
    fn new(name: &str) -> Option<Self> {
        if !ssh_keygen_available() {
            return None;
        }
        let dir = std::env::temp_dir().join(format!("spine-signing-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok()?;
        let key = dir.join("id");
        let ok = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", PRINCIPAL, "-f"])
            .arg(&key)
            .status()
            .ok()?
            .success();
        ok.then_some(Keys(dir))
    }

    fn private(&self) -> PathBuf {
        self.0.join("id")
    }

    /// `allowed_signers` granting `PRINCIPAL` the given namespaces.
    fn keyring(&self, namespaces: &[Namespace]) -> Vec<u8> {
        let public = std::fs::read_to_string(self.0.join("id.pub")).expect("a public key");
        let list: Vec<&str> = namespaces.iter().map(|n| n.as_str()).collect();
        format!(
            "{PRINCIPAL} namespaces=\"{}\" {}",
            list.join(","),
            public.trim()
        )
        .into_bytes()
    }
}

impl Drop for Keys {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn signoff_line() -> Vec<u8> {
    format!(
        "Spine-Signoff: INT-042 blob=dfb4079e22de55ec377468b9b697fdf86085ea37 \
         template=intent@2 constitution=v3 reopens=0 signer={PRINCIPAL}"
    )
    .into_bytes()
}

/// PB §7.2: "`<Name>-Sig: <SSHSIG, armor stripped to one line>` produced by
/// `ssh-keygen -Y sign -n <namespace>` over the exact bytes of that line".
#[test]
fn a_signed_statement_verifies_under_the_namespace_it_was_signed_with() {
    let Some(keys) = Keys::new("roundtrip") else {
        return;
    };
    let line = signoff_line();
    let payload = sign_line(
        TrailerName::Signoff,
        &line,
        Namespace::Signoff,
        &Key::File(&keys.private()),
    )
    .expect("a real key signs");

    // "armor stripped to one line".
    assert!(
        !payload.contains('\n'),
        "a -Sig payload is one unbroken run"
    );
    assert!(!payload.contains("BEGIN SSH SIGNATURE"));

    let keyring = keys.keyring(&[Namespace::Signoff]);
    let verified = verify_line(&line, &payload, PRINCIPAL, Namespace::Signoff, &keyring)
        .expect("what sign_line produced is what verify_line accepts");
    assert_eq!(verified.namespace, Namespace::Signoff);
}

/// The namespace is not decoration. PB §7.2 versions the payload format in it,
/// and MF §4.5 grants a principal namespaces one at a time — so a signature
/// made under one must not verify under another.
#[test]
fn a_signature_does_not_verify_under_a_namespace_it_was_not_made_with() {
    let Some(keys) = Keys::new("namespace") else {
        return;
    };
    let line = signoff_line();
    let payload = sign_line(
        TrailerName::Signoff,
        &line,
        Namespace::Signoff,
        &Key::File(&keys.private()),
    )
    .expect("signs");

    // The key holds both namespaces, so only the signature's own namespace can
    // be what separates them.
    let keyring = keys.keyring(&[Namespace::Signoff, Namespace::Review]);
    assert!(
        verify_line(&line, &payload, PRINCIPAL, Namespace::Review, &keyring).is_err(),
        "a spine-signoff@v1 signature must not verify as spine-review@v1"
    );
}

/// EV §2.7: "The trailer name and the `: ` are inside the signature. Signing
/// only the payload after `: ` is non-conforming: it would let a
/// `Spine-Approve` payload be replayed as a `Spine-Signoff`."
///
/// Demonstrated rather than asserted: a signature over a real `Spine-Approve`
/// line does not verify when the same payload is presented under the other
/// name, because the bytes differ by the name itself.
#[test]
fn a_signature_cannot_be_replayed_under_another_trailer_name() {
    let Some(keys) = Keys::new("replay") else {
        return;
    };
    let approve = format!(
        "Spine-Approve: INT-042 intent=dfb4079e22de55ec377468b9b697fdf86085ea37 \
         base=5f8e2a10cd47b6390e5a2c8db14f70369ba2e0d7 rounds=1 total_rounds=1 reopens=0 \
         red=5/5 freeze=sha256:3a8f signer={PRINCIPAL}"
    );
    let payload = sign_line(
        TrailerName::Approve,
        approve.as_bytes(),
        Namespace::Review,
        &Key::File(&keys.private()),
    )
    .expect("signs");

    let keyring = keys.keyring(&[Namespace::Review]);
    // As itself: verifies.
    assert!(
        verify_line(
            approve.as_bytes(),
            &payload,
            PRINCIPAL,
            Namespace::Review,
            &keyring
        )
        .is_ok()
    );

    // The same payload, presented under the other name. The signed bytes
    // included `Spine-Approve: `, so the substitution is visible.
    let replayed = approve.replace("Spine-Approve: ", "Spine-Signoff: ");
    assert!(
        verify_line(
            replayed.as_bytes(),
            &payload,
            PRINCIPAL,
            Namespace::Review,
            &keyring
        )
        .is_err(),
        "the trailer name is inside the signature"
    );
}

/// The terminator is excluded, so a caller holding a line out of a message and
/// one rendering it produce the same signature.
#[test]
fn the_terminating_newline_is_outside_the_signature() {
    let Some(keys) = Keys::new("terminator") else {
        return;
    };
    let line = signoff_line();
    let mut with_lf = line.clone();
    with_lf.push(b'\n');

    let key = Key::File(&keys.private());
    let bare = sign_line(TrailerName::Signoff, &line, Namespace::Signoff, &key).expect("signs");
    let keyring = keys.keyring(&[Namespace::Signoff]);

    // A signature made over the LF-terminated form verifies against the bare
    // line, which is only true if the LF was excluded from both.
    let terminated =
        sign_line(TrailerName::Signoff, &with_lf, Namespace::Signoff, &key).expect("signs");
    assert!(verify_line(&line, &terminated, PRINCIPAL, Namespace::Signoff, &keyring).is_ok());
    assert!(verify_line(&line, &bare, PRINCIPAL, Namespace::Signoff, &keyring).is_ok());
}

/// The `-Sig` line, as it goes into a message.
#[test]
fn the_sig_line_is_the_name_plus_the_payload() {
    let Some(keys) = Keys::new("sigline") else {
        return;
    };
    let line = signoff_line();
    let payload = sign_line(
        TrailerName::Signoff,
        &line,
        Namespace::Signoff,
        &Key::File(&keys.private()),
    )
    .expect("signs");
    let rendered = sig_line(TrailerName::Signoff, &payload);
    let text = String::from_utf8(rendered).expect("ASCII");
    assert!(text.starts_with("Spine-Signoff-Sig: "));
    assert!(text.ends_with('\n'));
    assert_eq!(text.matches('\n').count(), 1, "one line");
}

/// A missing key is a refusal with a reason, not a panic and not a silent
/// unsigned statement.
#[test]
fn a_key_that_is_not_there_refuses() {
    if !ssh_keygen_available() {
        return;
    }
    let line = signoff_line();
    let missing = Path::new("/nonexistent/spine/key");
    assert!(
        sign_line(
            TrailerName::Signoff,
            &line,
            Namespace::Signoff,
            &Key::File(missing)
        )
        .is_err()
    );
}
