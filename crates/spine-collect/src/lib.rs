//! `.spine/cache/results/<T>.jsonl` — the entire untrusted→trusted interface.
//!
//! RF §1: "The collector result file is the only object that crosses from the
//! untrusted CI stage to the trusted one. The untrusted job produces it; the
//! trusted job ingests it and gates on it; nothing else crosses that boundary."
//!
//! Five constraints from the design govern this whole crate and are not
//! re-argued anywhere in it (RF §1):
//!
//! - **One clock, and it is the chain.** "The file carries no timestamp, no
//!   duration and no ordinal derived from wall time. The deadline of §7.1 is
//!   enforced with a clock and *records* nothing from it beyond the fact that
//!   it expired." No type here holds a time, and the only trace of the deadline
//!   in the file is [`record::Status::RunnerTimeout`].
//! - **No state.** "It is a cache artifact inside `.spine/cache/`. It is never
//!   committed, never a graph source, never read by a run other than the one
//!   that produced it, and nothing remembers that it existed."
//! - **Hash policy.** Git object ids for git objects; `sha256:` only for
//!   non-git artifacts. This file names a tree and a commit by object id and
//!   the release by `sha256:`, "and it introduces no other hash".
//! - **Identity is the pair.** "A repository may run several runners, so a test
//!   id alone is not an identity. **A test is identified by `(runner, id)`**."
//!   Every uniqueness rule, sort key and match in this crate is over the pair.
//! - **Provenance is a precondition, not an ingestion test.** "Nothing below
//!   refuses a file for want of provider evidence." Nothing in this crate reads
//!   a provider fact, and [`malformed::Malformed`] has no variant for one.
//!
//! Reading order:
//!
//! 1. [`outcome`] — RF §5's closed vocabulary, and `absent`
//! 2. [`record`] — RF §4.4's three record kinds and §7.3's status set
//! 3. [`header`] — RF §4.2's six fields, in their fixed order
//! 4. [`file`] — RF §4.1's framing and §4.5's ordering, and §10's vectors
//! 5. [`collector`] — RF §7.1's ten steps, §7.2's reduction, §7.3's fold
//!
//! What is **not** here: the isolation mechanism (RF §7.1's M1, the probe and
//! P1-P4) is `spine-isolate`'s, behind [`collector::Host`]; the per-language
//! adapters (RF §6.3, §6.4) are `import-resolver.md`'s, behind
//! [`collector::RunnerAdapter`]; and ingestion's evaluation (RF §8.5's clauses,
//! G1's findings and their wire tokens) belongs to the gate layer, which reads
//! [`file::ResultFile`] rather than these bytes.

pub mod collector;
pub mod file;
pub mod header;
pub mod malformed;
pub mod outcome;
pub mod prepare;
pub mod record;

pub use collector::{
    BaseEnumeration, BaseId, BaseOutcomeRun, CandidateRun, Checkout, DEFAULT_TIMEOUT_SECS, Host,
    Mode, Policy, Refusal, Release, ResultItem, Run, RunnerAdapter, collect, exit_is_zero, fold,
    invocation_set, parse_spine_test_payload,
};
pub use file::ResultFile;
pub use header::{Header, Profile, Provenance};
pub use malformed::{Malformed, Section};
pub use outcome::{BaseOutcome, Outcome};
pub use prepare::{Collector, Git, PrepareError, Prepared, Refs, SelfBytes, SelfIdentity, prepare};
pub use record::{BaseRecord, EndRecord, ResultRecord, RunnerToken, Status};
