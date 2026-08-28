//! Producing a `-Sig` line — `ssh-keygen -Y sign`, and the rules around it.
//!
//! PB §7.2, quoted whole because every clause is load-bearing:
//!
//! > "**Every signed statement has one shape.** One trailer line ending in
//! > `signer=<principal>` (reviews: `reviewer=`), plus `<Name>-Sig: <SSHSIG,
//! > armor stripped to one line>` produced by `ssh-keygen -Y sign -n
//! > <namespace>` over the exact bytes of that line".
//!
//! EV §2.7 fixes the range, "because `dump.md` §5.2.1 asks this document to fix
//! it and hashes the same range":
//!
//! > "from the **first byte of the trailer name** — the `S` of `Spine-` —
//! > through the **last byte before the terminating `0x0A`**, with the `0x0A`
//! > **excluded**. The trailer name and the `: ` are inside the signature.
//! > Signing only the payload after `: ` is non-conforming: it would let a
//! > `Spine-Approve` payload be replayed as a `Spine-Signoff`."
//!
//! **This crate signs and does not decide who may.** Whether an invocation is
//! allowed to reach a human key at all is PB §7.1's TTY rule, which is a
//! property of the *invocation* and belongs to the command — `spine-cli` checks
//! it before anything is read. What is here is the one place a `-Sig` line is
//! produced, so there is one answer to "what bytes were signed".

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::trailer::TrailerName;
use crate::verify::{Namespace, strip_armor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignError {
    /// `ssh-keygen` could not be run at all.
    NoSshKeygen(String),
    /// It ran and refused — a missing key, a locked agent, a declined touch.
    Refused(String),
    /// The line to sign is not one: PB §7.2's shape is "one trailer line", and
    /// a payload carrying an LF would be two.
    LineCarriesNewline,
    /// EV §2.7's range starts at the trailer name; a caller that handed over a
    /// bare payload would sign bytes that could be replayed under another name.
    NotATrailerLine,
}

impl core::fmt::Display for SignError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SignError::NoSshKeygen(e) => write!(f, "ssh-keygen could not be run: {e}"),
            SignError::Refused(e) => write!(f, "ssh-keygen refused: {e}"),
            SignError::LineCarriesNewline => {
                f.write_str("a signed statement is one line; this payload carries a newline")
            }
            SignError::NotATrailerLine => f.write_str(
                "the signature covers the trailer name too (EV §2.7), so the bytes must begin \
                 with it — signing the payload alone would let it be replayed under another name",
            ),
        }
    }
}

impl core::error::Error for SignError {}

/// The exact bytes a `-Sig` covers, from a rendered trailer line.
///
/// Takes the line **including** its name, and refuses one that does not carry
/// the name it claims: EV §2.7's whole argument for including the name is that
/// a payload signed alone is replayable under a different trailer, and a
/// function that accepted either shape would let a caller reintroduce that.
///
/// A trailing `0x0A` is stripped rather than refused, because a caller holding
/// a line out of a message has it and a caller rendering one does not — and
/// "the `0x0A` **excluded**" makes them the same signature either way.
pub fn signed_range(name: TrailerName, line: &[u8]) -> Result<&[u8], SignError> {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    if line.contains(&b'\n') {
        return Err(SignError::LineCarriesNewline);
    }
    let prefix = format!("{}: ", name.as_str());
    if !line.starts_with(prefix.as_bytes()) {
        return Err(SignError::NotATrailerLine);
    }
    Ok(line)
}

/// Where the signing key is. A path, because `ssh-keygen -Y sign -f` takes one
/// — and because a key *in memory* is a shape PB §7.1 gives only to `--ci`,
/// whose secret is the pipeline's and never a human's.
#[derive(Debug, Clone)]
pub enum Key<'a> {
    /// A private key file, or a public key when an agent holds the private half.
    File(&'a Path),
}

/// Sign one trailer line, returning the `-Sig` payload: "SSHSIG, armor stripped
/// to one line".
///
/// The line goes to `ssh-keygen` on **stdin** rather than through a temporary
/// file. A statement is one line and fits any pipe, and a file would put the
/// bytes about to be signed on disk where something else could read or rewrite
/// them between the write and the read.
pub fn sign_line(
    name: TrailerName,
    line: &[u8],
    namespace: Namespace,
    key: &Key<'_>,
) -> Result<String, SignError> {
    let bytes = signed_range(name, line)?;
    let Key::File(path) = key;

    let mut child = Command::new("ssh-keygen")
        .arg("-Y")
        .arg("sign")
        .arg("-n")
        .arg(namespace.as_str())
        .arg("-f")
        .arg(path)
        .arg("-q")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| SignError::NoSshKeygen(e.to_string()))?;

    child
        .stdin
        .take()
        .ok_or_else(|| SignError::NoSshKeygen("no stdin".into()))?
        .write_all(bytes)
        .map_err(|e| SignError::NoSshKeygen(e.to_string()))?;

    let out = child
        .wait_with_output()
        .map_err(|e| SignError::NoSshKeygen(e.to_string()))?;
    if !out.status.success() {
        return Err(SignError::Refused(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(strip_armor(&String::from_utf8_lossy(&out.stdout)))
}

/// The `<Name>-Sig: <payload>` line itself.
pub fn sig_line(name: TrailerName, payload: &str) -> Vec<u8> {
    format!("{}-Sig: {payload}\n", name.as_str()).into_bytes()
}

/// Whether `ssh-keygen` can be spawned at all — for a diagnostic before a run
/// commits to anything, rather than a refusal in the middle of one.
pub fn ssh_keygen_available() -> bool {
    Command::new("ssh-keygen")
        .arg("-Q")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = "Spine-Signoff: INT-042 blob=dfb4079e22de55ec377468b9b697fdf86085ea37 \
                        template=intent@2 constitution=v3 reopens=0 signer=alice@example.com";

    /// EV §2.7: "from the **first byte of the trailer name** — the `S` of
    /// `Spine-` — through the **last byte before the terminating `0x0A`**".
    #[test]
    fn the_signed_range_is_the_whole_line_without_its_terminator() {
        let with_lf = format!("{LINE}\n");
        assert_eq!(
            signed_range(TrailerName::Signoff, with_lf.as_bytes()).unwrap(),
            LINE.as_bytes()
        );
        // A caller rendering a line has no terminator, and gets the same range.
        assert_eq!(
            signed_range(TrailerName::Signoff, LINE.as_bytes()).unwrap(),
            LINE.as_bytes()
        );
    }

    /// "Signing only the payload after `: ` is non-conforming: it would let a
    /// `Spine-Approve` payload be replayed as a `Spine-Signoff`."
    #[test]
    fn a_bare_payload_is_refused_because_it_would_be_replayable() {
        let payload = LINE.strip_prefix("Spine-Signoff: ").unwrap();
        assert_eq!(
            signed_range(TrailerName::Signoff, payload.as_bytes()),
            Err(SignError::NotATrailerLine)
        );
        // And a line signed under the wrong name is refused for the same
        // reason: the name is inside the signature, so it must be *this* name.
        assert_eq!(
            signed_range(TrailerName::Approve, LINE.as_bytes()),
            Err(SignError::NotATrailerLine)
        );
    }

    /// PB §7.2's shape is "**one** trailer line".
    #[test]
    fn a_payload_carrying_a_newline_is_not_one_statement() {
        let two = format!("{LINE}\nSpine-Seal: forged");
        assert_eq!(
            signed_range(TrailerName::Signoff, two.as_bytes()),
            Err(SignError::LineCarriesNewline)
        );
    }

    #[test]
    fn the_sig_line_carries_its_own_name() {
        assert_eq!(
            sig_line(TrailerName::Signoff, "AAAA"),
            b"Spine-Signoff-Sig: AAAA\n".to_vec()
        );
        assert_eq!(
            sig_line(TrailerName::Seal, "BBBB"),
            b"Spine-Seal-Sig: BBBB\n".to_vec()
        );
    }
}
