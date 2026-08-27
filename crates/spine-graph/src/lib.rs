//! The traceability graph and the `spine index --dump` format.
//!
//! PB §6.1 fixes what this crate may and may not be: *"This workflow already
//! contains a latent graph … Spine-kit's job is not to ask anyone to draw a
//! graph; it is to **extract the graph that already exists** in the artifacts."*
//! Nothing here has a constructor a human would call by hand, and every node
//! and edge carries a [`schema::Src`] because PB §6.1's corollary is total:
//! *"An edge that cannot say where it came from does not exist."*
//!
//! Reading order:
//!
//! 1. [`schema`] — nine node kinds, fifteen edge kinds, their attrs, the node-id
//!    grammar (DM §5.2), the provenance grammar (PB §6.1, DM §5.4)
//! 2. [`store`] — the in-memory graph, DM §5.5's collapse and DM §6's total
//!    order
//! 3. [`dump`] — the serializer G10 diffs, and G10's comparison itself
//! 4. [`status`] — DM §4.4's five refusals and their exit codes
//!
//! **What is not here.** PB §6.2 puts the canonical store in SQLite; a SQLite
//! binding is a dependency this crate may not add, so [`store::Graph`] is the
//! in-memory model and the persistence layer is unimplemented. The *derivation*
//! — which commits are walked, how an envelope is parsed, how a touchpoint
//! string becomes a `code_unit` id — is the indexer's, and DM §16 says so
//! explicitly: *"PB §6.2's derivation table is the indexer's spec; this document
//! serializes its output."*
//!
//! Citations: `PB §n` is `PLAYBOOK.md`; `DM §n` is `docs/spec/dump.md`;
//! `MF`, `RF`, `CN`, `ID`, `GR` are the manifest, result-file, constitution,
//! intent-doc and gate-report specs.

pub mod dump;
pub mod schema;
pub mod status;
pub mod store;

pub use dump::{DUMP_VERSION, Dump, G10, Header, edge_line, g10_compare, node_line, serialize};
pub use schema::{Attrs, EdgeKind, NodeKind, SCHEMA_VERSION, Src, tool_version_from_seal};
pub use status::{Refusal, Status};
pub use store::{Edge, Graph, Node};
