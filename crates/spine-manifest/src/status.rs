//! The closed malformed-status list, and the rule that only the first one is
//! ever reported.
//!
//! `manifest.md` §3.11 fixes these thirty-two tokens **and their order**, and
//! requires reporting "the first in document order and not continuing past it,
//! *because a manifest that does not parse cannot be checked further*".
//!
//! The order is load-bearing rather than cosmetic: the token reaches a
//! reviewer's `wires=` when G16 raises it, so two implementations that report
//! different tokens for one manifest produce reviews that do not discharge each
//! other's findings.

use core::fmt;

/// `manifest.md` §3.11, verbatim and in document order. `Ord` on this type is
/// declaration order, which is the reporting priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    ManifestMissing,
    ManifestNotJson,
    ManifestDuplicateMember,
    ManifestTooLarge,
    ManifestNoncanonical,
    ManifestUnknownMemberValue,
    MemberNameOutOfGrammar,
    ReservedMemberName,
    FrozenMemberMissing,
    FrozenMemberType,
    RepoOutOfGrammar,
    CliVersionOutOfGrammar,
    DistHashMalformed,
    ObjectFormatUnknown,
    TrunkNotABranchName,
    IsolationUnknown,
    IsolationUnsupported,
    CiUnknown,
    LangsUnknown,
    LangsEmpty,
    TimeoutOutOfRange,
    PathsValueMalformed,
    FilesDuplicatePath,
    FilesBaseMisplaced,
    OwnerUnknown,
    BlobMalformed,
    TemplateMalformed,
    TemplateVersionMismatch,
    PathHashAmbiguous,
    RegionNameOutOfGrammar,
    ResignKeyUnknown,
    ResignFloorAboveCurrent,
    /// Not in §3.11's list: `object_format` parsed but disagrees with the
    /// repository's `extensions.objectFormat`. §3.1 names it against the
    /// `object_format` row and G16 check 8 raises it, so it is a status but not
    /// a *malformed* status — it is reported only after parsing succeeds.
    ObjectFormatMismatch,
}

impl Status {
    /// The wire spelling. This string is what a reviewer signs, so it is fixed
    /// here and nowhere else.
    pub fn token(self) -> &'static str {
        match self {
            Status::ManifestMissing => "manifest-missing",
            Status::ManifestNotJson => "manifest-not-json",
            Status::ManifestDuplicateMember => "manifest-duplicate-member",
            Status::ManifestTooLarge => "manifest-too-large",
            Status::ManifestNoncanonical => "manifest-noncanonical",
            Status::ManifestUnknownMemberValue => "manifest-unknown-member-value",
            Status::MemberNameOutOfGrammar => "member-name-out-of-grammar",
            Status::ReservedMemberName => "reserved-member-name",
            Status::FrozenMemberMissing => "frozen-member-missing",
            Status::FrozenMemberType => "frozen-member-type",
            Status::RepoOutOfGrammar => "repo-out-of-grammar",
            Status::CliVersionOutOfGrammar => "cli-version-out-of-grammar",
            Status::DistHashMalformed => "dist-hash-malformed",
            Status::ObjectFormatUnknown => "object-format-unknown",
            Status::TrunkNotABranchName => "trunk-not-a-branch-name",
            Status::IsolationUnknown => "isolation-unknown",
            Status::IsolationUnsupported => "isolation-unsupported",
            Status::CiUnknown => "ci-unknown",
            Status::LangsUnknown => "langs-unknown",
            Status::LangsEmpty => "langs-empty",
            Status::TimeoutOutOfRange => "timeout-out-of-range",
            Status::PathsValueMalformed => "paths-value-malformed",
            Status::FilesDuplicatePath => "files-duplicate-path",
            Status::FilesBaseMisplaced => "files-base-misplaced",
            Status::OwnerUnknown => "owner-unknown",
            Status::BlobMalformed => "blob-malformed",
            Status::TemplateMalformed => "template-malformed",
            Status::TemplateVersionMismatch => "template-version-mismatch",
            Status::PathHashAmbiguous => "path-hash-ambiguous",
            Status::RegionNameOutOfGrammar => "region-name-out-of-grammar",
            Status::ResignKeyUnknown => "resign-key-unknown",
            Status::ResignFloorAboveCurrent => "resign-floor-above-current",
            Status::ObjectFormatMismatch => "object-format-mismatch",
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// A refusal: the status, and where in the manifest it was found.
///
/// `where_` is a diagnostic only — it reaches stderr and never a wire, because
/// §3.11's token is the whole of what a review names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub status: Status,
    pub where_: String,
}

impl Refusal {
    pub fn new(status: Status, where_: impl Into<String>) -> Self {
        Refusal {
            status,
            where_: where_.into(),
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.where_.is_empty() {
            write!(f, "{}", self.status)
        } else {
            write!(f, "{} at {}", self.status, self.where_)
        }
    }
}

impl core::error::Error for Refusal {}

pub type Result<T> = core::result::Result<T, Refusal>;

#[cfg(test)]
mod tests {
    use super::*;

    /// §3.11's list is closed at thirty-two, and the order is the reporting
    /// priority. A token added out of order changes which refusal a manifest
    /// with two faults produces, and therefore which wire a reviewer signs.
    #[test]
    fn the_closed_list_is_thirty_two_tokens_in_document_order() {
        const IN_DOCUMENT_ORDER: [Status; 32] = [
            Status::ManifestMissing,
            Status::ManifestNotJson,
            Status::ManifestDuplicateMember,
            Status::ManifestTooLarge,
            Status::ManifestNoncanonical,
            Status::ManifestUnknownMemberValue,
            Status::MemberNameOutOfGrammar,
            Status::ReservedMemberName,
            Status::FrozenMemberMissing,
            Status::FrozenMemberType,
            Status::RepoOutOfGrammar,
            Status::CliVersionOutOfGrammar,
            Status::DistHashMalformed,
            Status::ObjectFormatUnknown,
            Status::TrunkNotABranchName,
            Status::IsolationUnknown,
            Status::IsolationUnsupported,
            Status::CiUnknown,
            Status::LangsUnknown,
            Status::LangsEmpty,
            Status::TimeoutOutOfRange,
            Status::PathsValueMalformed,
            Status::FilesDuplicatePath,
            Status::FilesBaseMisplaced,
            Status::OwnerUnknown,
            Status::BlobMalformed,
            Status::TemplateMalformed,
            Status::TemplateVersionMismatch,
            Status::PathHashAmbiguous,
            Status::RegionNameOutOfGrammar,
            Status::ResignKeyUnknown,
            Status::ResignFloorAboveCurrent,
        ];

        // Declaration order is the document order, so `Ord` sorts the list into
        // itself. If a variant is ever inserted in the wrong place this fails.
        let mut sorted = IN_DOCUMENT_ORDER;
        sorted.sort();
        assert_eq!(sorted, IN_DOCUMENT_ORDER);

        // And the tokens are exactly §3.11's spellings.
        let tokens: Vec<&str> = IN_DOCUMENT_ORDER.iter().map(|s| s.token()).collect();
        assert_eq!(
            tokens.join(" "),
            "manifest-missing manifest-not-json manifest-duplicate-member manifest-too-large \
             manifest-noncanonical manifest-unknown-member-value member-name-out-of-grammar \
             reserved-member-name frozen-member-missing frozen-member-type repo-out-of-grammar \
             cli-version-out-of-grammar dist-hash-malformed object-format-unknown \
             trunk-not-a-branch-name isolation-unknown isolation-unsupported ci-unknown \
             langs-unknown langs-empty timeout-out-of-range paths-value-malformed \
             files-duplicate-path files-base-misplaced owner-unknown blob-malformed \
             template-malformed template-version-mismatch path-hash-ambiguous \
             region-name-out-of-grammar resign-key-unknown resign-floor-above-current"
        );
    }
}
