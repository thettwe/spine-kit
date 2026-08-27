//! The isolation boundary — step 6 of the collector's order of operations, the
//! two network dispositions, and the restore phase (RF §7.1).
//!
//! **`profile=` is a finding, never a request.** RF §7.1, verbatim: *"the
//! collector writes it only where a test it performed, and could have failed,
//! licensed it. A finding is never stronger than the request; the collector
//! never substitutes a mechanism for a profile the request did not name; and
//! comparing the two is the trusted stage's job, never the collector's."*
//! Nothing in this crate compares the finding with `params.isolation` — that
//! comparison is auto-merge precondition 1's (RF §8.4) and lives downstream.
//!
//! v1 ships **exactly one** mechanism, **M1**: each runner spawned as a child in
//! a new mount, PID, IPC, network and user namespace over a read-only overlay of
//! the job's own root. No image is pulled and none is named (RF §7.1).
//!
//! # The seam, and why it is where it is
//!
//! Every security property this crate asserts rests on facts about the host
//! kernel that no git object records — which is why RF §7.1 makes them *tests
//! the collector performs and can fail* rather than configuration. Two of the
//! four tests were unpassable as originally written, and the corpus was amended
//! from running the code rather than reading it (RF §13 R36). So this crate is
//! split at the line where a claim becomes a decision:
//!
//! - [`probe`] holds the **observation types** and the **deciders**. They are
//!   pure functions over recorded measurements, so P1–P4's verdicts are
//!   exercised on a host that cannot create a namespace at all.
//! - [`netlink`] holds P4(a)'s **parser**, likewise pure. RF §7.1 makes netlink
//!   normative *because a `sysfs` inherited across `unshare(2)` answers for the
//!   wrong namespace*; a parser that is a pure function of bytes is a parser a
//!   test can aim at the published measurement.
//! - [`sys`] and [`linux`] hold the syscalls, behind `#[cfg(target_os =
//!   "linux")]`. M1 needs kernel namespaces and therefore exists on Linux only
//!   (prerequisite 1); on a Darwin runner, which `ci.md` §5.5 ships a target
//!   for, *"M1 cannot be created at all; that is not a refusal but disposition
//!   2"* (RF §7.1).

pub mod m1;
pub mod netlink;
pub mod prereq;
pub mod probe;
pub mod profile;
pub mod sys;

#[cfg(target_os = "linux")]
pub mod linux;

pub use m1::{Checkout, Disposition, Phase, RestoreOutcome, RestoreScript};
pub use prereq::{IdentitySource, Prerequisite};
pub use probe::{Canary, DevIno, ProbeReport, Test, TestOutcome};
pub use profile::{BoundaryOutcome, Mode, Profile, Step1Refusal, Step6Plan};
