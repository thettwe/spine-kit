//! Canonicalization, escaping and digests — the primitives every spine
//! artifact's identity is computed with.
//!
//! Nothing in this crate touches the filesystem, the network or the clock. That
//! is deliberate: `gate-report.md` §7 collects the determinism rules, and the
//! cheapest way to keep them is a layer that has no way to break them. Two
//! implementations that agree here can disagree about everything else and still
//! verify each other's landings; two that disagree here cannot agree about
//! anything (PB §1.1).
//!
//! Reading order, which is also the order the corpus says to build in
//! (GR §8.3: "Debug your canonicalizer against this before attempting §8.2"):
//!
//! 1. [`value`] — the value model, narrower than JSON by design
//! 2. [`jcs`] — RFC 8785 canonical serialization
//! 3. [`esc`] — repository bytes into ASCII, and the wire-token variant
//! 4. [`digest`] — SHA-256 for artifacts, git object ids for git objects
//! 5. [`parse`] — reading untrusted JSON back, strictly

pub mod digest;
pub mod esc;
pub mod jcs;
pub mod parse;
pub mod value;

pub use digest::{ObjectFormat, git_blob_id, sha256_hex, sha256_prefixed};
pub use esc::{esc, tok, unesc};
pub use jcs::{canonicalize, canonicalize_to_string};
pub use parse::parse;
pub use value::Value;
