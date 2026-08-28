//! Signature verification — OpenSSH's, not ours.
//!
//! `manifest.md` §12: "Signature verification. OpenSSH's." PB §7.2 chooses SSH
//! signatures precisely so that "`ssh-keygen -Y verify` enforces role
//! membership with zero spine code, on an offline clone with only git objects
//! and OpenSSH". **No ed25519 arithmetic lives in this crate**, and none should:
//! a second implementation of the primitive is a second thing that can be wrong
//! about a landing's authority.
//!
//! PB §11 gives the command a human runs by hand, and this module runs exactly
//! it:
//!
//! ```text
//! ssh-keygen -Y verify -f .spine/allowed_signers -I <principal> -n <namespace> -s <sig> < <line>
//! ```
//!
//! **The role is the namespace the signature verified under.** `dump.md` §7.2:
//! "the namespace the signature verified under, never a claim in the trailer."
//! So [`verify_line`] takes the namespace as an input and the caller reads the
//! role out of the result, never out of the line's own text.

use crate::payload::{Approve, Mode, Seal, Withdraw};
use crate::refusal::EnvelopeError;
use crate::trailer::TrailerName;
use core::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// PB §11's three namespaces, and PB §7.2's three roles: "Three roles, no more
/// (§10 budgets them), expressed as SSH signature **namespaces**".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Namespace {
    Signoff,
    Review,
    Seal,
}

impl Namespace {
    pub fn as_str(self) -> &'static str {
        match self {
            Namespace::Signoff => "spine-signoff@v1",
            Namespace::Review => "spine-review@v1",
            Namespace::Seal => "spine-seal@v1",
        }
    }

    /// PB §7.2's Role column. One-to-one with the namespace, which is the whole
    /// point: the keyring says which namespaces a key holds, so ssh-keygen
    /// decides the role and spine reads it off.
    pub fn role(self) -> Role {
        match self {
            Namespace::Signoff => Role::Signer,
            Namespace::Review => Role::Reviewer,
            Namespace::Seal => Role::Pipeline,
        }
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// PB §7.2's three roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// "sign-off, reopen, withdraw, toolkit upgrade events" — humans.
    Signer,
    /// "reviews (tripwire, protected, break-glass); approvals in v1; the seal of
    /// a recovery landing (§7.5)" — humans.
    Reviewer,
    /// "the seal; approvals carrying `run=` once B runs in the trusted stage".
    Pipeline,
}

/// The namespace a signed statement must verify under.
///
/// Three of these are conditional, and each condition is a payload field the
/// corpus makes load-bearing:
///
/// - `Spine-Approve` — PB §11: "`run=` present ⇒ verifies under
///   `spine-seal@v1` only; absent ⇒ `spine-review@v1` only".
/// - `Spine-Seal` — PB §7.5's recovery landing "may be sealed under
///   `spine-review@v1` by one of two distinct protected reviewers", and "its
///   seal carries `mode=recovery`".
/// - `Spine-Withdraw` — PB §11: "(`--protected`: signed under
///   `spine-review@v1` by a reviewer ≠ the original signer, for an orphaned
///   branch)". **DERIVED**: no field names the `--protected` form directly, and
///   the corpus fixes only that the orphaned form is the one whose sign-off
///   could not be copied and which "names it `orphaned=<principal>`" (PB §5.5).
///   `orphaned=` is taken as the selector. The alternative — trying both
///   namespaces — is the reading that fails open, because it would let an
///   ordinary withdrawal be signed by a reviewer who is not the intent's signer.
pub fn expected_namespace(
    name: TrailerName,
    payload: &[u8],
) -> Result<Option<Namespace>, EnvelopeError> {
    Ok(match name {
        TrailerName::Signoff | TrailerName::Reopen | TrailerName::Upgrade => {
            Some(Namespace::Signoff)
        }
        TrailerName::Review => Some(Namespace::Review),
        TrailerName::Approve => Some(if Approve::parse(payload)?.run.is_some() {
            Namespace::Seal
        } else {
            Namespace::Review
        }),
        TrailerName::Seal => Some(if Seal::parse(payload)?.mode == Mode::Recovery {
            Namespace::Review
        } else {
            Namespace::Seal
        }),
        TrailerName::Withdraw => Some(if Withdraw::parse(payload)?.orphaned.is_some() {
            Namespace::Review
        } else {
            Namespace::Signoff
        }),
        _ => None,
    })
}

/// EV §2.7: "The **`-Sig` payload** is the SSHSIG blob's base64 with the PEM
/// armor removed: the `-----BEGIN SSH SIGNATURE-----` line, the
/// `-----END SSH SIGNATURE-----` line and every `0x0A` inside are deleted,
/// leaving one unbroken base64 run."
pub fn strip_armor(armored: &str) -> String {
    armored
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect::<String>()
}

/// The exact inverse: "the BEGIN line, the base64 wrapped at **70 characters
/// per line**, the END line, each terminated by `0x0A`. This round-trip is
/// byte-identical to what `ssh-keygen -Y sign` writes" (EV §2.7).
pub fn armor(sig_payload: &str) -> String {
    let mut out = String::from("-----BEGIN SSH SIGNATURE-----\n");
    let bytes = sig_payload.as_bytes();
    for chunk in bytes.chunks(70) {
        out.push_str(core::str::from_utf8(chunk).unwrap_or_default());
        out.push('\n');
    }
    out.push_str("-----END SSH SIGNATURE-----\n");
    out
}

/// What a successful verification establishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    /// The namespace the signature verified under — the role's only source
    /// (DM §7.2).
    pub namespace: Namespace,
    pub principal: String,
    /// `SHA256:…` as `ssh-keygen` prints it. PB §7.2: "reviewer ≠ signer
    /// compares fingerprints", so the comparison needs this and not a principal.
    pub key_fingerprint: String,
}

impl Verified {
    pub fn role(&self) -> Role {
        self.namespace.role()
    }
}

#[derive(Debug)]
pub enum VerifyError {
    /// `ssh-keygen` could not be run. PB §11's *Git requirements* make
    /// "OpenSSH ≥ 8.2 (`ssh-keygen -Y`)" a prerequisite, so this is an
    /// environment failure and never a verdict about the signature.
    OpenSshUnavailable(std::io::Error),
    Io(std::io::Error),
    /// `ssh-keygen -Y verify` exited non-zero: the signature, the principal, the
    /// namespace or the keyring did not agree.
    NotVerified {
        status: Option<i32>,
        stderr: String,
    },
    /// A principal that could be read as an `ssh-keygen` option. Refused before
    /// the process is spawned rather than passed through.
    UnusablePrincipal(String),
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifyError::OpenSshUnavailable(e) => {
                write!(
                    f,
                    "ssh-keygen could not be run (PB §11 requires OpenSSH ≥ 8.2): {e}"
                )
            }
            VerifyError::Io(e) => write!(f, "{e}"),
            VerifyError::NotVerified { status, stderr } => {
                write!(
                    f,
                    "signature did not verify (exit {status:?}): {}",
                    stderr.trim()
                )
            }
            VerifyError::UnusablePrincipal(p) => write!(f, "unusable principal: {p}"),
        }
    }
}

impl core::error::Error for VerifyError {}

/// Verify one signed statement.
///
/// `line` is the **signed byte range** of EV §13.8: "from the first byte of the
/// trailer name — the `S` of `Spine-` — through the last byte before the
/// terminating `0x0A`, with the `0x0A` **excluded**. The trailer name and the
/// `: ` are inside the signature. Signing only the payload after `: ` is
/// non-conforming: it would let a `Spine-Approve` payload be replayed as a
/// `Spine-Signoff`."
///
/// `allowed_signers` is the keyring **as it existed at the seal's `base=`** for
/// a landed statement (PB §5.5: "a landing can never admit its own signer") and
/// trunk's current tip for an in-flight one (PB §7.5's two clocks). Choosing
/// between them is the caller's; this function verifies against whichever bytes
/// it is handed.
pub fn verify_line(
    line: &[u8],
    sig_payload: &str,
    principal: &str,
    namespace: Namespace,
    allowed_signers: &[u8],
) -> Result<Verified, VerifyError> {
    if principal.starts_with('-') || principal.is_empty() {
        return Err(VerifyError::UnusablePrincipal(principal.to_owned()));
    }

    let dir = TempDir::new().map_err(VerifyError::Io)?;
    let signers_path = dir.path().join("allowed_signers");
    let sig_path = dir.path().join("signature");
    std::fs::write(&signers_path, allowed_signers).map_err(VerifyError::Io)?;
    std::fs::write(&sig_path, armor(sig_payload)).map_err(VerifyError::Io)?;

    let mut child = std::process::Command::new("ssh-keygen")
        .arg("-Y")
        .arg("verify")
        .arg("-f")
        .arg(&signers_path)
        .arg("-I")
        .arg(principal)
        .arg("-n")
        .arg(namespace.as_str())
        .arg("-s")
        .arg(&sig_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(VerifyError::OpenSshUnavailable)?;

    // The message is the line's exact bytes with no terminator — EV §8.6 pipes
    // it through `tr -d '\n'` for the same reason.
    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(line)
        .map_err(VerifyError::Io)?;
    let out = child.wait_with_output().map_err(VerifyError::Io)?;

    if !out.status.success() {
        return Err(VerifyError::NotVerified {
            status: out.status.code(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        });
    }
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    Ok(Verified {
        namespace,
        principal: principal.to_owned(),
        key_fingerprint: fingerprint_of(&stdout).unwrap_or_default(),
    })
}

/// `ssh-keygen` prints `Good "<ns>" signature for <principal> with ED25519 key
/// SHA256:<b64>`. Only the fingerprint is taken, and its absence is not an
/// error: the verdict is the exit status, never the wording.
fn fingerprint_of(stdout: &str) -> Option<String> {
    stdout
        .split_whitespace()
        .find(|w| w.starts_with("SHA256:"))
        .map(str::to_owned)
}

/// A directory removed on drop. `verify_line` writes the keyring and the
/// armored signature to disk because `ssh-keygen -Y verify` takes both by path
/// and reads only the message from stdin.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> std::io::Result<Self> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("spine-envelope-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path)?;
        Ok(TempDir(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_namespace_is_the_only_source_of_a_role() {
        assert_eq!(Namespace::Signoff.role(), Role::Signer);
        assert_eq!(Namespace::Review.role(), Role::Reviewer);
        assert_eq!(Namespace::Seal.role(), Role::Pipeline);
        assert_eq!(Namespace::Seal.as_str(), "spine-seal@v1");
    }

    #[test]
    fn armor_round_trips_at_seventy_columns() {
        let payload = "A".repeat(155);
        let armored = armor(&payload);
        let lines: Vec<&str> = armored.lines().collect();
        assert_eq!(lines[0], "-----BEGIN SSH SIGNATURE-----");
        assert_eq!(lines[1].len(), 70);
        assert_eq!(lines[2].len(), 70);
        assert_eq!(lines[3].len(), 15);
        assert_eq!(*lines.last().unwrap(), "-----END SSH SIGNATURE-----");
        assert_eq!(strip_armor(&armored), payload);
    }

    #[test]
    fn an_approve_carrying_run_verifies_under_the_seal_namespace() {
        // PB §11: "`run=` present ⇒ verifies under `spine-seal@v1` only;
        // absent ⇒ `spine-review@v1` only".
        let without = b"INT-042 intent=dfb4079e22de55ec377468b9b697fdf86085ea37 base=5f8e2a10cd47b6390e5a2c8db14f70369ba2e0d7 rounds=1 total_rounds=3 reopens=1 red=5/5 freeze=sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2 signer=alice@example.com";
        assert_eq!(
            expected_namespace(TrailerName::Approve, without).unwrap(),
            Some(Namespace::Review)
        );
        let with = b"INT-042 intent=dfb4079e22de55ec377468b9b697fdf86085ea37 base=5f8e2a10cd47b6390e5a2c8db14f70369ba2e0d7 rounds=1 total_rounds=3 reopens=1 red=5/5 freeze=sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2 run=sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2 signer=ci@example.com";
        assert_eq!(
            expected_namespace(TrailerName::Approve, with).unwrap(),
            Some(Namespace::Seal)
        );
    }

    #[test]
    fn a_recovery_seal_verifies_under_the_review_namespace() {
        // PB §7.5: with no usable pipeline key "a landing may be sealed under
        // `spine-review@v1` … its seal carries `mode=recovery`".
        let team = b"INT-042 base=7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51 head=77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9 tree=8b47d0e6c2a915f37e04b8d1c6a2f905e37b1d48 report=sha256:e70a3c92d1b845f6027e9ab3c5d10f684a2b7e93c60d5f81a34b0e29d7c6f105 tool=1.4.0+sha256:41d0e9b7c2a6538f10bd47e29c05a3f6b81d24e70c9a5b3f68d1027ae4c95b3d git=2.45 mode=team threat=hostile profile=container envelope=sha256:e1652897b251b001fe7e03e343d40bbdc7fb9b112ef920c8b53987916b14682f signer=ci@example.com";
        assert_eq!(
            expected_namespace(TrailerName::Seal, team).unwrap(),
            Some(Namespace::Seal)
        );
        let recovery = String::from_utf8(team.to_vec())
            .unwrap()
            .replace("mode=team", "mode=recovery");
        assert_eq!(
            expected_namespace(TrailerName::Seal, recovery.as_bytes()).unwrap(),
            Some(Namespace::Review)
        );
    }

    #[test]
    fn an_unsigned_trailer_expects_no_namespace() {
        assert_eq!(
            expected_namespace(TrailerName::Gates, b"G1=pass").unwrap(),
            None
        );
    }

    #[test]
    fn a_principal_that_looks_like_an_option_is_refused_before_spawning() {
        let e = verify_line(b"x", "AAAA", "-oProxyCommand=x", Namespace::Seal, b"").unwrap_err();
        assert!(matches!(e, VerifyError::UnusablePrincipal(_)));
    }
}
