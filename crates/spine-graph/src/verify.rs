//! Whether a signed line's `-Sig` verifies, and under which namespace.
//!
//! Two attrs of the dump are functions of this and of nothing else:
//! `approval.verified` — *"the `-Sig` verified against the keyring at the
//! seal's `base=`"* — and `approval.role`, which DM §7.2 defines as **the
//! namespace the signature verified under, never a claim in the trailer**. The
//! second is why this is a question about namespaces rather than a boolean: *"A
//! v1 approve line signed under `spine-review@v1` is `reviewer`"*, and the
//! trailer's own name says `signer`.
//!
//! **The implementation is OpenSSH's, not this crate's.** MF §12 fixes it —
//! *"Signature verification. OpenSSH's."* — and PB §11 prints the command a
//! human runs to check the same fact by hand:
//!
//! ```text
//! ssh-keygen -Y verify -f .spine/allowed_signers -I alice@example.com \
//!            -n spine-signoff@v1 -s <sig> < <line>
//! ```
//!
//! Reimplementing SSHSIG would mean reimplementing Ed25519, RSA-SHA2 and ECDSA
//! verification and the SSHSIG framing, in a crate that may take no
//! dependencies — and a verifier that is subtly wrong does not fail loudly, it
//! admits a signature.

use spine_manifest::keyring::NAMESPACES;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// Which namespace, if any, a signature verifies under.
pub trait Verifier {
    /// `Some(namespace)` when `signature` is a valid SSHSIG by `principal` over
    /// `statement` under that namespace, checked against the `allowed_signers`
    /// bytes given.
    ///
    /// The keyring bytes are passed rather than a path because PB §5.5 fixes
    /// *which* keyring: *"as it existed at the seal's `base=`"* — a blob, not a
    /// file. *"A landing can never admit its own signer."*
    fn namespace_that_verifies(
        &self,
        allowed_signers: &[u8],
        principal: &str,
        statement: &[u8],
        signature: &str,
    ) -> Option<String>;
}

/// The real one: `ssh-keygen -Y verify`, once per namespace.
///
/// **Trying each namespace is the derivation, not a shortcut.** DM §7.2 asks
/// for *"the namespace the signature verified under"*, and SSHSIG binds the
/// namespace into the signed blob, so the only way to learn it without decoding
/// the blob by hand is to ask OpenSSH which one satisfies it. Three closed
/// namespaces (PB §11) means at most three cheap subprocesses per signed line,
/// and the order is [`NAMESPACES`]' — fixed, so two runs of one binary ask the
/// same questions in the same order.
#[derive(Debug, Clone, Copy, Default)]
pub struct OpenSsh;

impl Verifier for OpenSsh {
    fn namespace_that_verifies(
        &self,
        allowed_signers: &[u8],
        principal: &str,
        statement: &[u8],
        signature: &str,
    ) -> Option<String> {
        if allowed_signers.is_empty() || principal.is_empty() || signature.is_empty() {
            return None;
        }
        let scratch = Scratch::new()?;
        let keyring = scratch.write("allowed_signers", allowed_signers)?;
        let sig = scratch.write("sig", armor(signature).as_bytes())?;
        for namespace in NAMESPACES {
            if verify_under(&keyring, &sig, principal, namespace, statement) {
                return Some(namespace.to_string());
            }
        }
        None
    }
}

/// A verifier that verifies nothing.
///
/// For an environment with no OpenSSH, and for tests that are about the
/// derivation rather than about cryptography. **It is fail-closed, not
/// neutral**: every `approval.verified` becomes `false`, every landing's
/// `seal_verified` becomes `false` and every landing therefore indexes
/// `unattested`, which PB §6.3 says is *"reported and counted"*. That is the
/// right direction to be wrong in, and it is visible in the dump rather than
/// silent.
#[derive(Debug, Clone, Copy, Default)]
pub struct Unverified;

impl Verifier for Unverified {
    fn namespace_that_verifies(&self, _: &[u8], _: &str, _: &[u8], _: &str) -> Option<String> {
        None
    }
}

/// Whether `ssh-keygen` can be spawned at all.
pub fn ssh_keygen_available() -> bool {
    Command::new("ssh-keygen")
        .arg("-Q")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn verify_under(
    keyring: &PathBuf,
    sig: &PathBuf,
    principal: &str,
    namespace: &str,
    statement: &[u8],
) -> bool {
    let Ok(mut child) = Command::new("ssh-keygen")
        .args(["-Y", "verify", "-f"])
        .arg(keyring)
        .args(["-I", principal, "-n", namespace, "-s"])
        .arg(sig)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    // The statement is written to stdin exactly as the message carries it: EV
    // §2.2's "Lines are taken **raw**. Nothing is trimmed, unfolded,
    // case-folded, Unicode-normalized, or re-encoded, at any point, for any
    // purpose — not to hash, not to sign, not to verify, not to parse." A
    // trailing LF added here would verify a line nobody signed.
    if child
        .stdin
        .as_mut()
        .is_some_and(|stdin| stdin.write_all(statement).is_ok())
    {
        drop(child.stdin.take());
        return child.wait().map(|s| s.success()).unwrap_or(false);
    }
    let _ = child.wait();
    false
}

/// PB §11 carries an SSHSIG *"armor stripped to one line"*; `ssh-keygen -Y
/// verify` reads the armored form, so the wrapping is put back.
fn armor(one_line: &str) -> String {
    let mut out = String::from("-----BEGIN SSH SIGNATURE-----\n");
    let body: String = one_line.split_whitespace().collect();
    let bytes = body.as_bytes();
    for chunk in bytes.chunks(70) {
        out.push_str(&String::from_utf8_lossy(chunk));
        out.push('\n');
    }
    out.push_str("-----END SSH SIGNATURE-----\n");
    out
}

/// A scratch directory for the two files `ssh-keygen -Y verify` insists on
/// reading from disk, removed when the verification is done.
///
/// The path is environment-derived and never reaches a dump: DM §10 rule 2
/// forbids a temp path as a *value*, and this one is not one.
struct Scratch(PathBuf);

static COUNTER: AtomicU64 = AtomicU64::new(0);

impl Scratch {
    fn new() -> Option<Self> {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("spine-graph-verify-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).ok()?;
        Some(Scratch(dir))
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Option<PathBuf> {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).ok()?;
        Some(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unverified_verifier_answers_no_for_every_namespace() {
        assert!(
            Unverified
                .namespace_that_verifies(b"keyring", "alice@example.com", b"line", "sig")
                .is_none()
        );
    }

    #[test]
    fn the_armor_is_the_one_line_form_wrapped_back_into_pem() {
        let armored = armor("AAAABBBB");
        assert!(armored.starts_with("-----BEGIN SSH SIGNATURE-----\n"));
        assert!(armored.ends_with("-----END SSH SIGNATURE-----\n"));
        assert!(armored.contains("AAAABBBB\n"));
        // Whitespace inside the one-line form is not signature data; it is a
        // transcription artefact, and keeping it would break the base64.
        assert_eq!(armor("AAAA BBBB"), armored);
    }

    #[test]
    fn a_signature_that_openssh_never_sees_is_not_verified() {
        // Empty inputs short-circuit before any subprocess, so this holds with
        // or without ssh-keygen on the machine.
        assert!(
            OpenSsh
                .namespace_that_verifies(b"", "alice@example.com", b"line", "sig")
                .is_none()
        );
        assert!(
            OpenSsh
                .namespace_that_verifies(b"k", "", b"line", "sig")
                .is_none()
        );
        assert!(
            OpenSsh
                .namespace_that_verifies(b"k", "alice@example.com", b"line", "")
                .is_none()
        );
    }
}
