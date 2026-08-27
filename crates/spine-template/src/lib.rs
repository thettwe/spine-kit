//! The twelve templates the release ships, and the bytes they render to.
//!
//! MF §3.6 and PB §6.7 fix the set at twelve, and PB §6.7 fixes where they
//! live: "Templates and agent prompts are embedded in the binary and never
//! written to the repo: there is nothing to customise, which is what 'the
//! template never expands' (§3.3) means mechanically, and prompt tuning is a
//! toolkit release, not a repo edit."
//!
//! So every body here is a compile-time constant, and the only variance is the
//! handful of substitution spans each template's spec names.

pub mod constitution;
pub mod harness;
pub mod regions;
pub mod release;
pub mod scaffold;
pub mod substitute;

pub use constitution::Seed;
pub use release::{NoReleaseManifest, ReleaseManifest};
pub use substitute::{RenderError, Table};
