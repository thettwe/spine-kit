//! `spine-intent` — the intent document: `intents/<ID>.md`, its parse grammar,
//! and the two gate predicates that read nothing else.
//!
//! **Why this artifact gets a grammar at all.** A pre-implementation audit
//! found that the intent document "has no parse grammar" while two gates are
//! pure functions of its parse. ID §1 states the shape of the gap plainly:
//!
//! > **Another branch's leases are evaluated by my binary.** … when
//! > `spine check --land INT-042` runs, it fetches every other in-flight intent
//! > branch and parses **their** documents to compute G7. … Two parsers that
//! > disagree about where a section ends, about whether `- ` continues a list,
//! > or about whether `src/bill` covers `src/billing/x.ts` do not merely render
//! > the document differently: they produce different gate verdicts over
//! > identical git objects, one binary rejects the other's landings, and
//! > PB §1.1's headline — an offline clone that re-verifies — is false.
//!
//! Four consumers parse this document and all four must agree byte for byte:
//! `spine new --sign`, `spine check --approve`, `spine check --land`, and
//! `spine index`.
//!
//! | Module | What it fixes |
//! |---|---|
//! | [`status`] | the closed status vocabulary and ID §8.2's five exit classes |
//! | [`canon`] | ID §2.1's twelve byte rules and ID §2.3's bounds |
//! | [`header`] | the title, the header's field table, `Template:`, `Supersedes:` |
//! | [`sections`] | headings, keys, the three variants' tables, the body grammars |
//! | [`ac`] | ID §5.3's `AC-<n>` domain, its bounds and its ordering |
//! | [`parse`] | ID §8.2's one failure order, and ID §5.6's parse result |
//! | [`gates`] | ID §7's G2 and G7 predicates, and `overlap` |
//!
//! **The pattern dialect is not here.** ID §6.1–§6.3 — the byte grammar, the
//! glob dialect and `match` — live in [`spine_resolve::glob`], which IR §2.4
//! adopts "by reference and unaltered" so that a constitution list and a
//! touchpoint list are compared against one diff with one semantics. This crate
//! calls it and reproduces ID §9.5's vectors against it rather than beside it.
//!
//! **Three determinism rules worth restating** (ID §10):
//!
//! - **No clock.** No member of the parse result is a time, a duration or a
//!   date, and no rule consults one.
//! - **No tree lookup.** "A pattern is never expanded to the paths it currently
//!   matches; a directory is distinguished from a file by a trailing `/` and
//!   never by a stat."
//! - **Closed sets refuse.** An unknown section, header field, template
//!   variant, template version or touchpoint label is refused — "never ignore,
//!   never carry opaque".

pub mod ac;
pub mod canon;
pub mod gates;
pub mod header;
pub mod parse;
pub mod sections;
pub mod status;

pub use ac::Ac;
pub use header::{Spelling, Variant};
pub use parse::{Declaration, Parsed, SignoffFacts, check_signoff, parse};
pub use sections::{BodyGrammar, Polarity, SectionSpec};
pub use status::{Class, Refusal, Status};

// Re-exported so a caller holding a `Parsed` need not also depend on
// `spine-resolve` to read its patterns or name its id.
pub use spine_resolve::Pattern;
pub use spine_resolve::pragma::{IntentId, IntentPrefix};
