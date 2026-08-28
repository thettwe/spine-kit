//! The three identifier shapes GR §7 rules 9 and 10 fix, plus the intent id.
//!
//! Each is a newtype with a checked constructor rather than a `String`, because
//! every one of them is inside the digest and every one of them has a shape a
//! plausible-looking wrong value satisfies. GR §8's own history is the argument:
//! withdrawal (4) replaced three `fingerprint` members that were "fabricated
//! until then" with keys `ssh-keygen -lf` reproduces, and the fabricated ones
//! had been the right *length* all along.

use core::fmt;

use spine_canon::ObjectFormat;

/// Why an identifier was refused. Each variant names the rule it failed, not
/// the value it was given: these reach a refusal message, and echoing an
/// attacker-supplied 64-byte string into one is how a log becomes a payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdError {
    /// GR §7 rule 9: "lowercase hex at the full length `object_format` implies
    /// — 40 or 64 digits. Never abbreviated, never uppercase, never prefixed."
    OidLength {
        expected: usize,
        found: usize,
    },
    OidNotLowercaseHex,
    /// GR §7 rule 10: `"sha256:"` + 64 lowercase hex. "Never bare hex, never
    /// uppercase, never another algorithm."
    DigestPrefix,
    DigestBody,
    /// GR §5.5: `"SHA256:"` plus unpadded base64, in `ssh-keygen -lf` form.
    FingerprintPrefix,
    FingerprintBody,
    /// GR §5.1: `subject.intent` "matches `^(INT|BUG)-[0-9]+$`. Never the
    /// repo-scoped graph id (PB §6.2)."
    IntentGrammar,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdError::OidLength { expected, found } => {
                write!(f, "object id must be {expected} hex digits, found {found}")
            }
            IdError::OidNotLowercaseHex => f.write_str("object id must be lowercase hex"),
            IdError::DigestPrefix => f.write_str("digest must start with \"sha256:\""),
            IdError::DigestBody => {
                f.write_str("digest must carry 64 lowercase hex digits after \"sha256:\"")
            }
            IdError::FingerprintPrefix => f.write_str("fingerprint must start with \"SHA256:\""),
            IdError::FingerprintBody => {
                f.write_str("fingerprint must carry 43 unpadded base64 characters")
            }
            IdError::IntentGrammar => f.write_str("intent id must match ^(INT|BUG)-[0-9]+$"),
        }
    }
}

impl core::error::Error for IdError {}

/// A git object id, at the full length its repository's `object_format`
/// implies.
///
/// GR §7 rule 9: "The playbook's `9f2c…` is display, not a value." The type
/// carries no format of its own — a 40-digit `Oid` is a valid sha1 id and an
/// invalid sha256 one, and which it is depends on the report's `object_format`
/// member, so the format is a *parse* argument and the invariant is re-checked
/// by [`crate::Report::validate`] against the member the report actually holds.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Oid(String);

impl Oid {
    pub fn parse(s: &str, format: ObjectFormat) -> Result<Self, IdError> {
        let expected = format.hex_len();
        if s.len() != expected {
            return Err(IdError::OidLength {
                expected,
                found: s.len(),
            });
        }
        if !s.bytes().all(is_lower_hex) {
            return Err(IdError::OidNotLowercaseHex);
        }
        Ok(Oid(s.to_owned()))
    }

    /// Whether this id has the length `format` implies. `object_format` is a
    /// member of the report, so a report can carry an id of the wrong length
    /// for its own declared format and only this check finds it.
    pub fn fits(&self, format: ObjectFormat) -> bool {
        self.0.len() == format.hex_len()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Oid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Oid {
    /// Never abbreviated. An `Oid` printed in a test failure is compared by
    /// eye against a published vector, and `9f2c…` is the one form that makes
    /// two different ids look the same.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Oid({})", self.0)
    }
}

/// A non-git digest: `"sha256:"` + 64 lowercase hex (GR §7 rule 10, PB §11's
/// hash policy).
///
/// The report is itself named by one of these, over "exactly the canonical
/// bytes. No trailing newline, no BOM, no framing" (GR §2.1).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    /// The digest of some bytes. This is the only constructor that cannot be
    /// wrong, and it is what every test in this crate uses.
    pub fn of(bytes: &[u8]) -> Self {
        Sha256Digest(spine_canon::sha256_prefixed(bytes))
    }

    pub fn parse(s: &str) -> Result<Self, IdError> {
        let body = s.strip_prefix("sha256:").ok_or(IdError::DigestPrefix)?;
        if body.len() != 64 || !body.bytes().all(is_lower_hex) {
            return Err(IdError::DigestBody);
        }
        Ok(Sha256Digest(s.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sha256Digest({})", self.0)
    }
}

/// An SSH public key fingerprint in `ssh-keygen -lf` form (GR §5.5).
///
/// "This, not the principal, is what `reviewer ≠ signer` compares (PB §7.2)."
///
/// The 43-character body is **checked**, and that check is the reason this is a
/// type. A SHA-256 fingerprint is 32 bytes, which is 43 unpadded base64
/// characters and can be nothing else — so a value of any other length is not
/// in the value space of any key, whatever it looks like. GR §8.2.1 withdrawal
/// (4) records this document's own three fingerprints being replaced for
/// exactly that reason.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fingerprint(String);

/// 32 bytes of SHA-256, base64-encoded without padding: `ceil(32 / 3) * 4 - 1`.
const FINGERPRINT_BODY_LEN: usize = 43;

impl Fingerprint {
    pub fn parse(s: &str) -> Result<Self, IdError> {
        let body = s
            .strip_prefix("SHA256:")
            .ok_or(IdError::FingerprintPrefix)?;
        if body.len() != FINGERPRINT_BODY_LEN || !body.bytes().all(is_base64_standard) {
            return Err(IdError::FingerprintBody);
        }
        Ok(Fingerprint(s.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fingerprint({})", self.0)
    }
}

/// GR §5.1's `subject.intent`: `^(INT|BUG)-[0-9]+$`, the bare id.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntentId(String);

impl IntentId {
    pub fn parse(s: &str) -> Result<Self, IdError> {
        let rest = s
            .strip_prefix("INT-")
            .or_else(|| s.strip_prefix("BUG-"))
            .ok_or(IdError::IntentGrammar)?;
        // `[0-9]+` — at least one digit, and nothing else. A leading zero is
        // admitted: the grammar is `[0-9]+`, not a numeric literal, and this
        // spec never reads a number out of it.
        if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
            return Err(IdError::IntentGrammar);
        }
        Ok(IntentId(s.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IntentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for IntentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IntentId({})", self.0)
    }
}

const fn is_lower_hex(b: u8) -> bool {
    b.is_ascii_digit() || b.is_ascii_lowercase() && b <= b'f'
}

/// The standard base64 alphabet, which is what OpenSSH prints. Not base64url:
/// GR §8.2's own `SHA256:V2dasTIGWUnhlaxa7vr3Qmqpe/qYBoxi+C7Cs6yxxEs` carries
/// both `/` and `+`, so an implementation validating against base64url would
/// refuse a published vector.
const fn is_base64_standard(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'+' || b == b'/'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_oid_must_match_its_format_length() {
        let sha1 = "7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51";
        assert!(Oid::parse(sha1, ObjectFormat::Sha1).is_ok());
        assert_eq!(
            Oid::parse(sha1, ObjectFormat::Sha256),
            Err(IdError::OidLength {
                expected: 64,
                found: 40
            })
        );
    }

    /// GR §7 rule 9: "never abbreviated, never uppercase". The playbook's
    /// `9f2c…` is display.
    #[test]
    fn an_abbreviated_or_uppercase_oid_is_refused() {
        assert!(Oid::parse("7b0d1f4a", ObjectFormat::Sha1).is_err());
        assert_eq!(
            Oid::parse(
                "7B0D1F4A9C2E6B8D05F3A71C4E9B2D6F8A0C3E51",
                ObjectFormat::Sha1
            ),
            Err(IdError::OidNotLowercaseHex)
        );
    }

    /// GR §7 rule 10: "never bare hex, never uppercase, never another
    /// algorithm."
    #[test]
    fn a_digest_must_carry_its_prefix_and_sixty_four_lowercase_hex() {
        let hex = "e2bd8cb5da473701bf0dd8e9aa0c6a28cc1280e33ba06758165c5089ae3f5b47";
        assert!(Sha256Digest::parse(&format!("sha256:{hex}")).is_ok());
        assert_eq!(Sha256Digest::parse(hex), Err(IdError::DigestPrefix));
        assert_eq!(
            Sha256Digest::parse(&format!("sha1:{hex}")),
            Err(IdError::DigestPrefix)
        );
        assert_eq!(
            Sha256Digest::parse(&format!("sha256:{}", hex.to_uppercase())),
            Err(IdError::DigestBody)
        );
    }

    /// GR §2.1: "A file holding a report contains exactly the canonical bytes
    /// and nothing else, so `sha256sum` over the file reproduces `report=`."
    /// Computed here rather than transcribed — the digest of the empty input is
    /// the one SHA-256 value that needs no vector to check.
    #[test]
    fn digest_of_bytes_is_the_prefixed_sha256() {
        assert_eq!(
            Sha256Digest::of(b"").as_str(),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// GR §8.2's two published fingerprints, both 43 characters, one carrying
    /// `/` and `+`.
    #[test]
    fn the_published_fingerprints_parse() {
        for fp in [
            "SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM",
            "SHA256:V2dasTIGWUnhlaxa7vr3Qmqpe/qYBoxi+C7Cs6yxxEs",
        ] {
            assert_eq!(Fingerprint::parse(fp).unwrap().as_str(), fp);
            assert_eq!(fp.len() - "SHA256:".len(), FINGERPRINT_BODY_LEN);
        }
    }

    /// A fingerprint of any length but 43 is not in the value space of any key.
    /// This is the check GR §8.2.1's fourth withdrawal exists because of.
    #[test]
    fn a_fingerprint_outside_the_key_value_space_is_refused() {
        assert_eq!(
            Fingerprint::parse("SHA256:tooshort"),
            Err(IdError::FingerprintBody)
        );
        // Padded base64 is 44 characters and is what a naive encoder emits.
        assert_eq!(
            Fingerprint::parse("SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM="),
            Err(IdError::FingerprintBody)
        );
        assert_eq!(
            Fingerprint::parse("MD5:d4:1d:8c:d9:8f:00:b2:04:e9:80:09:98:ec:f8:42:7e"),
            Err(IdError::FingerprintPrefix)
        );
    }

    #[test]
    fn intent_ids_take_int_and_bug_and_nothing_else() {
        assert!(IntentId::parse("INT-042").is_ok());
        assert!(IntentId::parse("BUG-051").is_ok());
        assert_eq!(IntentId::parse("INT-"), Err(IdError::IntentGrammar));
        assert_eq!(IntentId::parse("TASK-1"), Err(IdError::IntentGrammar));
        assert_eq!(IntentId::parse("INT-4a"), Err(IdError::IntentGrammar));
    }

    /// GR §5.1: "Never the repo-scoped graph id (PB §6.2)."
    #[test]
    fn a_repo_scoped_graph_id_is_not_an_intent_id() {
        assert_eq!(
            IntentId::parse("spine-kit:INT-042"),
            Err(IdError::IntentGrammar)
        );
    }
}
