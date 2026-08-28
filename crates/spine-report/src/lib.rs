//! The gate report — "the only artifact that records **why a landing was
//! permitted**" (GR §1).
//!
//! `report=` appears in two signed places and never leaves either: on a
//! `Spine-Review`, where a human's signature says *I read this evaluation of
//! this tree and accepted these wires*, and on a `Spine-Seal`, where the
//! pipeline's says *these gates, over these objects, under this policy,
//! produced this verdict*. "The envelope records the inputs; the report records
//! the judgement over them."
//!
//! # Three facts that shape every line of this crate
//!
//! **The artifact is a digest.** GR §2.1: `report=sha256:<hex>` is taken over
//! "exactly the canonical bytes. No trailing newline, no BOM, no framing." So
//! there is one serialization ([`Report::canonical_bytes`]), no pretty form,
//! and no second digest — GR §11 puts "any second digest, second format, or
//! exported rendering" out of scope.
//!
//! **Two implementations that canonicalize differently cannot verify each
//! other's landings** (GR §1). The canonicalization itself is `spine_canon`'s
//! and is not reimplemented here; what this crate owns is the *schema* — which
//! members exist, what each holds, and in what order the arrays come out.
//!
//! **Two array orders, and they are not the same order.** `gates[]` sorts by
//! gate **number**; `wires[]` sorts ascending by unsigned byte value over the
//! whole **wire token**, so `G11` precedes `G2` (PB §11, GR §5.6). GR §8.2.1
//! records the trap: "Both are permutations … *every length check in this
//! document passes under both orders and only the digests separate them*. An
//! implementation that sorts numerically will match all three published
//! lengths, the `first 96 canonical bytes`, and §8.3, and still reproduce
//! neither digest."
//!
//! # Reading order
//!
//! 1. [`vocab`] — every closed token domain (GR §5, §6)
//! 2. [`ids`] — oids, `sha256:` digests, fingerprints, intent ids (GR §7 rules 9–10)
//! 3. [`gate`] — the sixteen gates, which run, and what each raises (GR §5.6.2, §6.3)
//! 4. [`wire`] — the wire token, the array's order, containment (GR §6.1, §6.2)
//! 5. [`report`] — the schema, and its canonical bytes (GR §5, §2)
//! 6. [`validate`] — the cross-member invariants (GR §5–§7)
//! 7. [`read`] — reading one back, under a closed schema (GR §3.2)
//! 8. [`verify`] — `--verify`'s outcomes and their normative order (GR §4)
//! 9. [`git_version`] — GR §5.3's normative parse
//!
//! # What is deliberately absent
//!
//! - **A timestamp, a duration, a hostname, an environment capture.** GR §5:
//!   "PB §7.5's rule is the whole rule: **one clock and it is the chain.**"
//!   No type in this crate can hold one.
//! - **A `null`.** GR §7 rule 6: "An optional member is present or absent …
//!   Absence always means *this concept does not apply to this landing*, never
//!   *unknown* and never *empty*." Optional members are `Option`, and empty
//!   arrays are emitted: "`[]` is a value, not an absence" (rule 5).
//! - **The report's own digest, and `envelope=`.** GR §7 rule 11: "The report
//!   never contains its own digest, and never contains `envelope=` — the
//!   `Spine-Review` lines that carry `report=` are inside the envelope digest,
//!   and a report containing `envelope=` would be circular through them."
//! - **Any I/O.** Nothing here reads a file, a ref or a clock. GR §4.4.6: "A
//!   note is never a source", and the cheapest way to keep that true is a layer
//!   with no way to read one. Publication to `refs/notes/spine` is `ci.md`'s
//!   wiring over [`Report::canonical_bytes`], whose bytes GR §4.4.1 makes the
//!   note's content exactly.

pub mod gate;
pub mod git_version;
pub mod ids;
pub mod read;
pub mod report;
pub mod validate;
pub mod verify;
pub mod vocab;
pub mod wire;

pub use gate::{
    G7Clause, G8Clause, Gate, GateResult, KindRule, TokenShape, WireClassRule, WireSpec,
    spine_gates_value,
};
pub use git_version::GitVersionError;
pub use ids::{Fingerprint, IdError, IntentId, Oid, Sha256Digest};
pub use read::{ReadError, VersionUnknownCause};
pub use report::{
    Authority, Automerge, Collector, Evidence, Objects, Policy, REPORT_VERSION, Report, Rules, Run,
    Statement, Subject, Tool,
};
pub use validate::{Invariant, rule_five_wire};
pub use verify::{Outcome, Preconditions, VerifyStatus, verify};
pub use vocab::{
    AutoMerge, Event, GateStatus, LandingShape, Lane, Mode, Namespace, PreconditionStatus,
    Reverify, RuleMode, SealProfile, Strategy, Threat, WireClass, WireKind,
};
pub use wire::{Wire, WireSet, WireSetError};
