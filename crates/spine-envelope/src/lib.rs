//! The intent envelope — PB §5.5's landing commit message.
//!
//! PB §5.5: "The disposal rule deletes the file. It must not delete the truth.
//! The record is a git object: the landing commit." Everything the ledger later
//! says about a landing — who signed what, over which base, against which
//! tests, under which policy, with which gates green — is read out of that one
//! byte string, "by `git cat-file commit`, on a clone with nothing but git
//! objects and OpenSSH" (EV §1).
//!
//! Three digests bind it, and this crate owns two of them (EV §1):
//!
//! | Digest | Where it lives | Covers | Owner |
//! |---|---|---|---|
//! | `envelope=` | `Spine-Seal` | every `Spine-*` line above the seal | [`digest`] |
//! | `freeze=` | `Spine-Approve` | the sorted `Spine-Frozen` and `Spine-Test` lines | [`digest`] |
//! | `report=` | `Spine-Review`, `Spine-Seal` | the canonical gate report | `gate-report.md`, not here (EV §5) |
//!
//! Reading order, which is also EV §18 item 25's debugging order — "C isolates
//! the sort, A's freeze adds the quoting, A's envelope adds the selection and
//! the join, and B and D add the lane and strategy variations":
//!
//! 1. [`refusal`] — EV §12's closed refusal list
//! 2. [`trailer`] — what a `Spine-*` line is, and how a payload splits
//! 3. [`quote`] — `git ls-tree` C-quoting, the encoding `Spine-Frozen` uses
//! 4. [`payload`] — PB §11's *Trailers* table, one type per grammar
//! 5. [`digest`] — the two joins, and the sort
//! 6. [`message`] — the message as bytes: regions, fence, cap, derived subject
//! 7. [`verify`] — `ssh-keygen -Y verify`, and the namespace that decides a role
//!
//! **Three traps this crate is built around**, each named because it produces
//! byte counts that match and digests that do not:
//!
//! - **The wire comparator.** `wires=` sorts "ascending by unsigned byte value
//!   over the whole token, so `G11` precedes `G2`" (PB §11), while
//!   `Spine-Gates` sorts by gate *number*. Re-sorting is a permutation, so
//!   every published byte count passes under both orders and only the digests
//!   separate them (EV §14 D3). The sort key is `tok(path)`, never `esc(path)`
//!   (GR §6.1).
//! - **Four encodings of one path.** `esc`, `tok`, `git ls-tree` C-quoting and
//!   the result file's escape set are four encodings of one path, and three can
//!   appear in one landing. "An implementation that reuses one encoder for both
//!   produces lines no conforming implementation reproduces" (EV §13.9).
//!   [`quote`] is the C-quoting and calls nothing in [`spine_canon::esc`].
//! - **The trailing LF.** Both joins here have **none** (EV §7 rule 10), while
//!   the manifest carries exactly one and `dump.md`'s stream terminates every
//!   line including the last. EV publishes the wrong value beside the right one
//!   for both of this crate's digests, and both are pinned in `tests/`.

pub mod digest;
pub mod message;
pub mod payload;
pub mod quote;
pub mod refusal;
pub mod trailer;
pub mod verify;

pub use digest::{envelope_digest, freeze_digest};
pub use message::{CAP, Envelope, Fence, Shape};
pub use refusal::{EnvelopeError, Refusal};
pub use trailer::{Trailer, TrailerName, is_spine_line, parse_line, render_line};
pub use verify::{Namespace, Role, Verified, verify_line};
