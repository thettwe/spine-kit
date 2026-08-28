//! The runner adapters — `spine-collect`'s [`RunnerAdapter`] seam, filled.
//!
//! This crate exists because the two halves of an adapter live in two other
//! crates and neither may depend on the other: `spine-resolve` owns
//! `import-resolver.md`'s argv table and its id/`fn`/`path` functions, and
//! `spine-collect` owns `result-file.md`'s records and the trait. An adapter
//! needs both, so it lives above them.
//!
//! **What is here and what is not.** The [`transport`] protocol, its parser and
//! the per-runner outcome mapping are here, and they are pure: bytes in,
//! records out. Spawning the runner is [`transport::Transport`]'s, behind a
//! seam, because a process is the one thing a test cannot make deterministic
//! and because RF §6.6 fixes the transport's *properties* — "read over a pipe
//! the collector holds, it is not supplied by the candidate's environment, and
//! it preserves, per item, **four** signals" — and calls the mechanism "an
//! implementation choice".
//!
//! [`RunnerAdapter`]: spine_collect::RunnerAdapter

pub mod pytest;
pub mod transport;

pub use transport::{Item, Phase, PhaseOutcome, Report, StreamError};
