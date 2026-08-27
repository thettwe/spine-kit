//! The refusals `spine index --dump` can produce, and their exit codes.
//!
//! DM §4.4 fixes five status tokens and their exits. They are typed here rather
//! than spelled at each call site because DM §4.4 explains what a wrong one
//! costs: *"emitting a partial dump would produce a spurious terminal G10
//! failure with a misleading diff. Refusing loudly, in the same process that
//! built the graph, names the defect instead."* A token nobody can read is a
//! defect nobody can name.

use core::fmt;

/// DM §4.4's table, in its published order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// Exit 0. Not a refusal — present so the table is total and so a caller
    /// can report the run's status without a second vocabulary.
    Ok,
    /// Exit 2. "Trunk resolution reached step 4 of §4.2. Nothing is written to
    /// stdout." A dump of nothing and a dump of a repository spine does not
    /// manage are different facts (DM §9 case 1).
    NotInstalled,
    /// Exit 3. "The derivation produced a `src` outside PB §6.1's grammar."
    ProvenanceInvalid,
    /// Exit 3. "The derivation produced a node id outside §5.2."
    IdOutOfGrammar,
    /// Exit 3. "An attr value outside §2.3 or §7.2 — a float, a `null`, a
    /// nested object, an unknown name."
    AttrsOutOfProfile,
    /// Exit 3. "G10 only: two dumps with different `dump_version` or
    /// `schema_version` were offered for comparison (§3.2)."
    DumpVersionSkew,
}

impl Status {
    /// The wire spelling, DM §4.4 verbatim. Fixed here and nowhere else.
    pub fn token(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::NotInstalled => "not-installed",
            Status::ProvenanceInvalid => "provenance-invalid",
            Status::IdOutOfGrammar => "id-out-of-grammar",
            Status::AttrsOutOfProfile => "attrs-out-of-profile",
            Status::DumpVersionSkew => "dump-version-skew",
        }
    }

    /// DM §4.4's exit column. Three of the five share exit 3 because they share
    /// a cause: "Exits 3 are internal-consistency refusals: the derivation
    /// produced something this format cannot represent."
    pub fn exit_code(self) -> i32 {
        match self {
            Status::Ok => 0,
            Status::NotInstalled => 2,
            Status::ProvenanceInvalid
            | Status::IdOutOfGrammar
            | Status::AttrsOutOfProfile
            | Status::DumpVersionSkew => 3,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// A refusal: the status, and which element provoked it.
///
/// `where_` is a diagnostic only. It goes to stderr, which DM §2.2 says is not
/// part of the artifact — stdout carries the dump and nothing else.
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

    #[test]
    fn the_five_refusal_tokens_are_dm_4_4s_spellings_with_its_exit_codes() {
        let rows = [
            (Status::Ok, "ok", 0),
            (Status::NotInstalled, "not-installed", 2),
            (Status::ProvenanceInvalid, "provenance-invalid", 3),
            (Status::IdOutOfGrammar, "id-out-of-grammar", 3),
            (Status::AttrsOutOfProfile, "attrs-out-of-profile", 3),
            (Status::DumpVersionSkew, "dump-version-skew", 3),
        ];
        for (status, token, exit) in rows {
            assert_eq!(status.token(), token);
            assert_eq!(status.exit_code(), exit);
            assert_eq!(status.to_string(), token);
        }
    }

    #[test]
    fn a_refusal_names_the_element_without_changing_the_token() {
        let r = Refusal::new(Status::IdOutOfGrammar, "myrepo/cs:abc");
        assert_eq!(r.to_string(), "id-out-of-grammar at myrepo/cs:abc");
        assert_eq!(r.status.token(), "id-out-of-grammar");
    }
}
