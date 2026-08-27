//! `.spine/manifest.json` — the lockfile, its grammar, and the gates that read it.
//!
//! PB §6.7: "It is machine-written, never hand-edited (G16 enforces this) — a
//! lockfile, not a document, and not a fourth prose artifact: it records a
//! *decision* (which toolkit this repo agreed to run), which is the one thing a
//! derived graph cannot reconstruct."
//!
//! The property this crate exists to keep is that **an old binary can judge a
//! new manifest**. Twelve fields are frozen — their names, their types and the
//! `owner` set never change at any `manifest_version` — and everything else is
//! opaque data a binary preserves without understanding (PB §6.7, PB §11).

pub mod artifacts;
pub mod grammar;
pub mod keyring;
pub mod region;
pub mod schema;
pub mod status;

pub use artifacts::{ArtifactList, host_target, target_for};
pub use keyring::{Keyring, Lint, Mode};
pub use region::{MarkerStyle, Region, RegionError};
pub use schema::{FileRecord, Isolation, Manifest, Owner};
pub use status::{Refusal, Status};
