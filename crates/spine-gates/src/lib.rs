//! `spine-gates` — the sixteen gates, as queries.
//!
//! PB §6.3: "With this schema, every gate becomes a query." Each gate here is a
//! function from typed inputs to a verdict plus wires, and the inputs are typed
//! **in this crate** rather than borrowed from a producer's internals: a gate
//! that reached into the collector's parse, or the indexer's store, would be a
//! gate whose answer changed when a producer refactored.
//!
//! Three rules govern every module and are not re-argued in any of them.
//!
//! **The complete wire set is computed before any lane routes it** (PB §11),
//! "and it is computed the same way for every landing that runs gates — gated,
//! quick and lifecycle alike. Lane decides the ceremony; it never decides which
//! wires exist." So [`gate::LandingShape`] selects which gates run
//! ([`gate::Gate::runs_on`]) and, for G1 and G8, which findings are outright —
//! and nothing else.
//!
//! **Two orders, deliberately different** (GR §5.6). `gates[]` sorts by gate
//! number; `wires[]` sorts "ascending by unsigned byte value over the whole
//! token, so `G11` precedes `G2`". Re-sorting `wires` is a permutation, so
//! every published byte count passes under both orders and only the digests
//! separate them. [`gate::Gate`]'s derived `Ord` is the first;
//! [`wire::WireSet::ordered`] is the second, keyed on `tok(path)` and never on
//! `esc(path)`.
//!
//! **No wall clock, no environment, no prior run.** GR §7 rule 1: "No member
//! holds a time, a duration, a date or anything derived from one." G3's
//! staleness is a comparison of two committer dates the caller supplies, and
//! the window is a constant of the pinned release ([`tripwires::STALENESS_WINDOW_SECS`]).
//!
//! Reading order:
//!
//! 1. [`gate`] — the ids, the five families, and GR §5.6.2's which-gates-run table
//! 2. [`wire`] — the `wires` array, its uniqueness rule and its order
//! 3. [`review`] — containment, aggregation, and the signerless overlay
//! 4. [`verdict`] — `pass` / `override` / `fail`, and *outright*
//! 5. [`status`] — every status token the corpus fixes, typed
//! 6. [`g14`] · [`g16`] · [`g13`] — the three Authority gates with owned algorithms
//! 7. [`g15`] — the membership test
//! 8. [`g1`] — G1 and G8, and RF §8.5 clause 2's allocation between them
//! 9. [`g10`] — reconstruction, whose comparison `spine-graph` already owns
//! 10. [`automerge`] — PB §7.4 rule 5's five preconditions
//! 11. [`tripwires`] — G2, G3, G4, G5, G7, G12: the gates whose wire is fixed
//!     and whose predicate is another document's

pub mod automerge;
pub mod g1;
pub mod g10;
pub mod g13;
pub mod g14;
pub mod g15;
pub mod g16;
pub mod gate;
pub mod review;
pub mod status;
pub mod tripwires;
pub mod verdict;
pub mod wire;

pub use gate::{ALL_GATES, Family, Gate, LandingShape};
pub use review::{Review, ReviewClass, ReviewState, Reviews, review_state};
pub use status::{G1Status, G8Status, G13Status, G14Status, G16Status, RunStatus};
pub use verdict::{Finding, FindingKind, GateStatus, Verdict, decide, with_break_glass};
pub use wire::{Wire, WireClass, WireKind, WireSet};
