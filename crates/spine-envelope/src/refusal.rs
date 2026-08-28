//! `envelope-vectors.md` §12's closed refusal list.
//!
//! Every token here reaches a human: at `--land` it is the refusal printed
//! before anything is sealed, and on an already-landed commit it is what makes
//! G9 index the landing `unattested` — "reported and counted forever, never
//! silently repaired" (EV §12). So the spelling is fixed once, here, and
//! [`Refusal::token`] is the only place a byte of it exists.

use core::fmt;

/// EV §12, verbatim and in that table's order.
///
/// A landing that fails any of these "is not fixed by editing the message: that
/// would rewrite the commit, which the non-fast-forward rule on trunk denies.
/// The repair is a reseal" (EV §12) — which is why nothing in this crate
/// repairs anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Refusal {
    /// "the capped quantity of §2.9 exceeds 16384. Never a truncation."
    EnvelopeTooLarge,
    /// "any structural violation of §2."
    EnvelopeMalformed,
    /// "`Spine-Envelope` is not `1`. Refuse before computing anything."
    EnvelopeVersionUnknown,
    /// "the fenced bytes do not hash to `blob=`, or their count is not
    /// `bytes=`."
    FenceMismatch,
    /// "the subject is not the line §13.10 derives for the landing's shape."
    SubjectMismatch,
    /// "a recomputed `envelope=`, `freeze=` or `report=` differs from the
    /// sealed value."
    DigestMismatch,
    /// "two identical `Spine-Frozen` or `Spine-Test` lines (§4.2)."
    FreezeDuplicate,
    /// "a `Spine-Test` function id contains `0x0A`, `0x0D` or `0x00` (§4.4)."
    TestIdUnrepresentable,
}

impl Refusal {
    /// The wire spelling, byte for byte as EV §12 prints it.
    pub fn token(self) -> &'static str {
        match self {
            Refusal::EnvelopeTooLarge => "envelope-too-large",
            Refusal::EnvelopeMalformed => "envelope-malformed",
            Refusal::EnvelopeVersionUnknown => "envelope-version-unknown",
            Refusal::FenceMismatch => "fence-mismatch",
            Refusal::SubjectMismatch => "subject-mismatch",
            Refusal::DigestMismatch => "digest-mismatch",
            Refusal::FreezeDuplicate => "freeze-duplicate",
            Refusal::TestIdUnrepresentable => "test-id-unrepresentable",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// A refusal plus the diagnostic that produced it.
///
/// The two are kept apart on purpose: the *token* is what a reviewer's report
/// carries and it must never grow a suffix, while a bare token is useless to
/// the person holding the malformed envelope. [`EnvelopeError::refusal`] is the
/// record; [`EnvelopeError::detail`] is for the human.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeError {
    refusal: Refusal,
    detail: String,
}

impl EnvelopeError {
    pub fn new(refusal: Refusal, detail: impl Into<String>) -> Self {
        EnvelopeError {
            refusal,
            detail: detail.into(),
        }
    }

    /// The overwhelmingly common case: EV §12's `envelope-malformed` covers
    /// "any structural violation of §2", so it has one constructor and every
    /// structural check reaches for it.
    pub fn malformed(detail: impl Into<String>) -> Self {
        EnvelopeError::new(Refusal::EnvelopeMalformed, detail)
    }

    pub fn refusal(&self) -> Refusal {
        self.refusal
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.refusal.token(), self.detail)
    }
}

impl core::error::Error for EnvelopeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_refusal_token_is_ev_12s_spelling() {
        // Transcribed from EV §12's Code column, top to bottom.
        assert_eq!(Refusal::EnvelopeTooLarge.token(), "envelope-too-large");
        assert_eq!(Refusal::EnvelopeMalformed.token(), "envelope-malformed");
        assert_eq!(
            Refusal::EnvelopeVersionUnknown.token(),
            "envelope-version-unknown"
        );
        assert_eq!(Refusal::FenceMismatch.token(), "fence-mismatch");
        assert_eq!(Refusal::SubjectMismatch.token(), "subject-mismatch");
        assert_eq!(Refusal::DigestMismatch.token(), "digest-mismatch");
        assert_eq!(Refusal::FreezeDuplicate.token(), "freeze-duplicate");
        assert_eq!(
            Refusal::TestIdUnrepresentable.token(),
            "test-id-unrepresentable"
        );
    }

    #[test]
    fn a_details_suffix_never_reaches_the_token() {
        let e = EnvelopeError::malformed("two Spine-Seal lines");
        assert_eq!(e.refusal().to_string(), "envelope-malformed");
        assert_eq!(e.to_string(), "envelope-malformed: two Spine-Seal lines");
    }
}
