//! The `spine index --dump` serializer — the bytes G10 diffs.
//!
//! DM §1 says why this file is the one that has to be right:
//!
//! > *"the dump is the only artifact in spine-kit whose *format* can fail a
//! > landing. Every other artifact is checked against a signature or a digest;
//! > this one is checked against another copy of itself. A difference of one
//! > byte — a key emitted in insertion order, an integer with a leading zero, a
//! > path spelled NFD on one side and NFC on the other, a `git blame` heuristic
//! > that changed between git releases — is indistinguishable from a corrupted
//! > ledger and produces the same terminal refusal."*
//!
//! And the refusal is terminal in the strongest sense the design has: the run
//! ends `reconstruction-failed`, the candidate `L` is discarded and never
//! becomes a git object, no `C-M3` retry is consumed, the run is not re-queued,
//! and break-glass cannot bypass it — PB §7.6's list does not contain G10 and
//! "could not, since `L` already exists and its seal covers its own message".
//!
//! The format is JSON Lines, each line RFC 8785 JCS under DM §2.3's profile,
//! byte-valued data through `gate-report.md`'s `esc`, LF framing **with a
//! terminating LF on the last line**. That last clause is the one an
//! implementer gets wrong: the trailing-LF rule inverts by artifact across the
//! corpus — a gate report has none, a manifest has exactly one, a dump has one
//! per line including the last — and DM §2.5 puts the final LF inside the
//! digest.

use crate::schema::{AttrValue, Attrs, EdgeKind, NodeKind, SCHEMA_VERSION, check_node_id, is_oid};
use crate::status::{Refusal, Result, Status};
use crate::store::Graph;
use spine_canon::{ObjectFormat, Value, canonicalize, esc, sha256_prefixed};

/// DM §3.1: *"`1`. This document defines version 1."*
///
/// A constant and not a field, because DM §3.4 makes it a promise rather than a
/// report: *"a release that changes the projection **must** bump
/// `dump_version`, even for a change it believes is a bug fix. A silent
/// projection change is a fleet-wide `reconstruction-failed` on the first
/// landing after a rolling upgrade, and the report will name the graph, not the
/// release."*
pub const DUMP_VERSION: u64 = 1;

/// The framing byte. DM §2.2: *"each terminated by exactly one `0x0A` (LF). The
/// final line is terminated too … No CR anywhere, no BOM, no blank lines, no
/// comments, no trailing blank line."*
const LF: u8 = 0x0A;

/// The header record's inputs — DM §3.1's table, minus the two versions, which
/// are constants above.
///
/// DM §3.4 explains what is deliberately *not* here: `cli.version`,
/// `cli.dist_hash`, or any other name for the producing binary. *"two releases
/// carrying the same `dump_version` and `schema_version` **must** produce
/// identical bytes over identical objects. … Recording the release would let a
/// genuine divergence hide behind a version difference."*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// *"the **indexed repository's own** format (`extensions.objectFormat`;
    /// absent means `sha1`)"* — not the manifest's. DM §13.11: *"every oid in a
    /// dump is an object *in that repository*, and the dump should describe
    /// what it read."*
    pub object_format: ObjectFormat,
    /// The manifest's `repo` — "the prefix of every node id (PB §6.2)".
    ///
    /// Held as a `String` rather than bytes because MF §3.1 bounds it to
    /// `^[A-Za-z0-9._-]+$`, 1..=64 bytes: the grammar admits no byte `esc`
    /// would touch, so DM §3.1's "`esc`-encoded" and the raw form are the same
    /// string.
    pub repo: String,
    /// The resolved trunk **branch name**, not a full refname (DM §3.1, §4.2).
    /// Bytes, because a ref name is bytes.
    pub trunk: Vec<u8>,
    /// *"iff `refs/heads/<trunk>` resolves"*. Absent means the derivation had
    /// no trunk to walk, and the dump is empty (DM §9 case 2).
    ///
    /// It is in the artifact because PB §6.1 requires a rendering to be
    /// *"datable against the ledger it came from"*, and because it "converts
    /// the most confusing possible G10 failure into the most legible one".
    pub head: Option<String>,
    /// *"iff `spine.trustRoot` is configured"*.
    ///
    /// The chain walk of PB §7.5 decides which signer nodes exist and which
    /// `verified` / `seal_verified` attrs are `true`, so the trust root is an
    /// input to the dump — and it is a git config value, not an object, so
    /// nothing else in the artifact would reveal a mismatch. PB §6.3 G10 now
    /// writes it into **both** sides (DM §14 D1, CLOSED); recording it keeps a
    /// recurrence legible as a line-1 diff.
    pub trust_root: Option<String>,
}

/// A produced dump: its bytes, and the two versions G10 needs before it is
/// allowed to compare.
///
/// Nothing here parses. DM §1: *"Nothing in spine ever reads a dump. G10
/// compares two byte strings; it never parses one into nodes and edges … A byte
/// comparator is not a reader."* The versions travel beside the bytes because
/// the producer knows them, not because anyone read them back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dump {
    bytes: Vec<u8>,
    pub dump_version: u64,
    pub schema_version: u64,
}

impl Dump {
    /// The artifact. DM §2.2: *"`spine index --dump` writes exactly these bytes
    /// to **stdout** and nothing else to stdout."*
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Lines, counting the last — which is terminated like every other.
    pub fn line_count(&self) -> usize {
        self.bytes.iter().filter(|&&b| b == LF).count()
    }

    /// DM §2.5: *"`sha256:` + 64 lowercase hex digits over exactly the byte
    /// stream of §2.2 — including the final LF, excluding nothing."*
    ///
    /// *"It is never sealed, never signed, never a trailer field, and never a
    /// member of a gate report."* It is a convenience for G10 and for humans,
    /// permitted as an implementation of the byte comparison only because
    /// SHA-256 collision resistance makes the two equivalent.
    pub fn digest(&self) -> String {
        sha256_prefixed(&self.bytes)
    }
}

/// Serialize a graph as a dump.
///
/// The caller supplies the *projection*, not the store: DM §1 draws the line
/// — the store holds in-flight intents, provisional changesets, volatile test
/// results and the shipped floor because `spine check` needs them, and "the
/// dump holds only what a fresh clone of trunk can rederive". What this
/// function does enforce is DM §8's consequences, which are mechanical: no
/// `exercises` edge, no `provisional: true`, no `floor: true`, no `test` attr,
/// no `introduced_by`, no † status, no worktree or shipped-floor citation.
pub fn serialize(header: &Header, graph: &Graph) -> Result<Dump> {
    // The `repo` grammar is MF §3.1's, adopted from DM §5.2 unchanged. The
    // token differs by document — MF raises `repo-out-of-grammar` against a
    // manifest, DM §5.2 raises `id-out-of-grammar` against a dump — because a
    // dump refuses over the *ids* the repo prefixes, not over the manifest.
    spine_manifest::grammar::check_repo(&header.repo)
        .map_err(|_| Refusal::new(Status::IdOutOfGrammar, format!("repo {:?}", header.repo)))?;

    // DM §3.1 makes `trunk` "the resolved trunk **branch name**", and DM §4.2
    // makes an unresolved one `not-installed` rather than an empty string — so
    // an empty `trunk` is a header that reached the happy path without a value
    // any reader can use.
    if header.trunk.is_empty() {
        return Err(Refusal::new(
            Status::IdOutOfGrammar,
            "header trunk is empty; DM §4.2 makes an unresolved trunk `not-installed`",
        ));
    }

    for (name, oid) in [("head", &header.head), ("trust_root", &header.trust_root)] {
        if let Some(oid) = oid
            && !is_oid(oid, header.object_format)
        {
            return Err(Refusal::new(
                Status::IdOutOfGrammar,
                format!("header {name} {oid:?}"),
            ));
        }
    }

    let mut out = Vec::new();
    write_line(&mut out, &header_record(header));

    // DM §2.2: nodes first, then edges. The header is line 1 by framing, never
    // by sort (DM §6.1).
    let nodes = graph.ordered_nodes();
    let mut seen_ids: Vec<&str> = Vec::with_capacity(nodes.len());
    for node in &nodes {
        check_node_id(&header.repo, node.kind, &node.id, header.object_format)?;
        // DM §17 item 14: "`id` is unique across the node section." PB §6.2
        // makes it the store's `PRIMARY KEY`; a duplicate here means the
        // §5.5 collapse was bypassed.
        if seen_ids.contains(&node.id.as_str()) {
            return Err(Refusal::new(Status::IdOutOfGrammar, node.id.clone()));
        }
        seen_ids.push(&node.id);

        node.attrs
            .check_profile(|n| node.kind.attr_type(n), &node.id)?;
        check_node_attr_domains(node.kind, &node.attrs, &node.id, header.object_format)?;
        node.src.check(header.object_format)?;

        out.extend_from_slice(&node_line(node));
    }

    // Every node id the dump emitted, so an edge endpoint can be checked
    // against the kind it actually names where the node is present.
    let node_kinds: Vec<(&str, NodeKind)> = nodes.iter().map(|n| (n.id.as_str(), n.kind)).collect();

    for edge in graph.ordered_edges() {
        // **Endpoints are validated.** They were not, until 2026-08-28: an edge
        // could carry a `from` outside the repo prefix, outside ASCII, or of a
        // kind DM §13.4 forbids, and serialize cleanly. DM §5.3 says a
        // malformed endpoint "surfaces as `id-out-of-grammar`", and DM §17
        // items 4 and 10 require every string to be ASCII-after-`esc` and every
        // id to match its kind's row.
        //
        // The kind is checked only where the node is present in this dump. A
        // dangling endpoint is not a serialization fault — it is G5's finding,
        // and refusing it here would make the dump unable to represent the
        // thing a gate exists to report.
        if let Some((from_kind, to_kind)) = edge.kind.endpoints() {
            for (role, id, expected) in [("from", &edge.from, from_kind), ("to", &edge.to, to_kind)]
            {
                let observed = node_kinds
                    .iter()
                    .find(|(node_id, _)| *node_id == id.as_str())
                    .map(|(_, kind)| *kind);
                match observed {
                    // Present: DM §13.4's table decides, and it is the table
                    // §13.4 calls load-bearing — "Getting one backwards is a
                    // whole-dump diff, so all fifteen are written down."
                    Some(kind) if kind != expected => {
                        return Err(Refusal::new(
                            Status::IdOutOfGrammar,
                            format!(
                                "{} edge {role} {id:?} is a {} node, not a {}",
                                edge.kind.token(),
                                kind.token(),
                                expected.token()
                            ),
                        ));
                    }
                    Some(_) => {}
                    // Absent: the id must still be well formed for the kind the
                    // table says it should be.
                    None => check_node_id(&header.repo, expected, id, header.object_format)?,
                }
            }
        }

        // DM §5.3 and §17 item 9: `exercises` is in the closed `kind` domain
        // and is emitted by nothing. Its source is a CI coverage report, which
        // DM §8.1's generating rule excludes for the same reason it excludes a
        // result file.
        if edge.kind == EdgeKind::Exercises {
            return Err(Refusal::new(
                Status::AttrsOutOfProfile,
                format!("exercises edge {} -> {}", edge.from, edge.to),
            ));
        }
        edge.attrs
            .check_profile(|n| edge.kind.attr_type(n), edge.kind.token())?;
        check_edge_attr_domains(edge.kind, &edge.attrs)?;
        edge.src.check(header.object_format)?;

        out.extend_from_slice(&edge_line(edge));
    }

    Ok(Dump {
        bytes: out,
        dump_version: DUMP_VERSION,
        schema_version: SCHEMA_VERSION,
    })
}

/// DM §3.1's header record.
fn header_record(header: &Header) -> Value {
    let mut members: Vec<(String, Value)> = vec![
        ("dump_version".into(), Value::Int(DUMP_VERSION)),
        (
            "object_format".into(),
            Value::str(header.object_format.as_str()),
        ),
        ("repo".into(), Value::str(esc(header.repo.as_bytes()))),
        ("schema_version".into(), Value::Int(SCHEMA_VERSION)),
        ("t".into(), Value::str("header")),
        ("trunk".into(), Value::str(esc(&header.trunk))),
    ];
    // DM §7.3 and §10 rule 6: "`null` never appears; a member is present or
    // absent". Absence means "does not apply", never "unknown" and never
    // "empty".
    if let Some(head) = &header.head {
        members.push(("head".into(), Value::str(head.clone())));
    }
    if let Some(trust_root) = &header.trust_root {
        members.push(("trust_root".into(), Value::str(trust_root.clone())));
    }
    Value::Obj(members)
}

/// Canonicalize one record and terminate it. JCS sorts the members, so the
/// insertion order at each call site is irrelevant to the bytes — DM §10 rule
/// 4: *"Key ordering inside a record is JCS's: ascending by member-name bytes.
/// Never insertion order, never a hand-written order."*
fn write_line(out: &mut Vec<u8>, record: &Value) {
    out.extend_from_slice(&canonicalize(record));
    out.push(LF);
}

/// One node record's bytes, terminated (DM §5.1).
///
/// Exposed because DM §12.4 publishes a *fragment* — "11 lines, each
/// LF-terminated, no header — this is a fragment, not a dump" — which exists to
/// debug a comparator against. A dump is [`serialize`], which validates; this
/// function validates nothing.
pub fn node_line(node: &crate::store::Node) -> Vec<u8> {
    let mut out = Vec::new();
    write_line(
        &mut out,
        &Value::obj([
            ("attrs", node.attrs.to_value()),
            ("id", Value::str(node.id.clone())),
            ("kind", Value::str(node.kind.token())),
            ("src", Value::str(node.src.render())),
            ("t", Value::str("node")),
        ]),
    );
    out
}

/// One edge record's bytes, terminated (DM §5.3). See [`node_line`].
pub fn edge_line(edge: &crate::store::Edge) -> Vec<u8> {
    let mut out = Vec::new();
    write_line(
        &mut out,
        &Value::obj([
            ("attrs", edge.attrs.to_value()),
            ("from", Value::str(edge.from.clone())),
            ("kind", Value::str(edge.kind.token())),
            ("src", Value::str(edge.src.render())),
            ("t", Value::str("edge")),
            ("to", Value::str(edge.to.clone())),
        ]),
    );
    out
}

/// Every attr DM §7.2 types as a git object id.
///
/// Held as one list because the check is one check: DM's four attr types have
/// no oid among them, so the type layer cannot carry this and a per-kind match
/// would be fifteen places to forget it.
const OID_VALUED_ATTRS: [&str; 9] = [
    "base",
    "blob",
    "freezes_oid",
    "head",
    "landing",
    "tree",
    "valid_from",
    "valid_to",
    "voided_by",
];

/// The node-attr domains DM §7.2 and §7.3 close, and the exclusions DM §17
/// items 15 and 18 make mechanically checkable.
fn check_node_attr_domains(
    kind: NodeKind,
    attrs: &Attrs,
    where_: &str,
    format: ObjectFormat,
) -> Result<()> {
    let bad = |what: String| {
        Err(Refusal::new(
            Status::AttrsOutOfProfile,
            format!("{where_}: {what}"),
        ))
    };

    // Every attr DM §7.2 types as a git object id must BE one.
    //
    // `attr_type` says `Str` for all of these, because DM's profile has four
    // types and "an oid" is not one of them — so without this the schema
    // accepts `blob: "abc123f"` and `landing: "ABC"` and serializes them into
    // the artifact G10 diffs. An abbreviated or uppercase oid compares unequal
    // to every id git produces, so it is a value that can never match anything
    // and can never be noticed either.
    for name in OID_VALUED_ATTRS {
        if let Some(AttrValue::Str(value)) = attrs.get(name)
            && !is_oid(value, format)
        {
            return bad(format!(
                "{name} {value:?} is not a {} object id",
                format.as_str()
            ));
        }
    }

    // DM §7.2's presence column. An attr marked "always" and absent is a node
    // the dump can hold and no reader can use — and for `changeset.landing` in
    // particular its absence silently escaped the member-changeset rule below,
    // which is guarded on the value being present and `false`.
    for name in kind.always_present_attrs() {
        if attrs.get(name).is_none() {
            return bad(format!(
                "{name} is always present on a {} node",
                kind.token()
            ));
        }
    }

    match kind {
        NodeKind::Intent => {
            // DM §7.3 and §17 item 15. The three names PB §11 lists alongside
            // these are absent on purpose: `orphan`, `unattested` and
            // `resealed` are **changeset** facts in PB §6.2's schema, and "a
            // landing can be `unattested` while its intent is plainly
            // `merged`". A † status cannot appear at all, because §8.2
            // excluded in-flight intents.
            if let Some(AttrValue::Str(status)) = attrs.get("status")
                && !matches!(
                    status.as_str(),
                    "merged" | "withdrawn" | "reverted" | "superseded"
                )
            {
                return bad(format!("intent.status {status:?}"));
            }
        }
        NodeKind::Changeset => {
            // DM §7.2: "A member changeset carries `{"landing":false}` and
            // nothing else: it has no seal, and every one of those fields is a
            // seal field."
            if let Some(AttrValue::Bool(false)) = attrs.get("landing")
                && attrs.iter().count() != 1
            {
                return bad("a member changeset carries only `landing`".to_string());
            }
        }
        NodeKind::Approval => {
            check_enum(
                attrs,
                "event",
                &[
                    "signoff", "approve", "review", "reopen", "withdraw", "upgrade",
                ],
                where_,
            )?;
            // DM §7.2: role is "**the namespace the signature verified
            // under**, never a claim in the trailer (PB §4.3, PB §7.2). A v1
            // approve line signed under `spine-review@v1` is `reviewer`."
            check_enum(attrs, "role", &["signer", "reviewer", "pipeline"], where_)?;
            check_enum(
                attrs,
                "class",
                &["tripwire", "protected", "break-glass"],
                where_,
            )?;
        }
        NodeKind::Signer => {
            // DM §7.2: "the namespaces this key is listed under, ascending by
            // bytes: a subset of `spine-review@v1`, `spine-seal@v1`,
            // `spine-signoff@v1`."
            if let Some(AttrValue::StrArr(roles)) = attrs.get("roles") {
                let known = ["spine-review@v1", "spine-seal@v1", "spine-signoff@v1"];
                if let Some(unknown) = roles.iter().find(|r| !known.contains(&r.as_str())) {
                    return bad(format!("signer.roles {unknown:?}"));
                }
                if roles.windows(2).any(|w| w[0] >= w[1]) {
                    return bad("signer.roles is not ascending by bytes".to_string());
                }
            }
        }
        // DM §17 item 18: "every `test` node's `attrs` is `{}`" — enforced by
        // `NodeKind::attr_type` returning `None` for every name, `result_at`
        // included. Nothing left to check.
        NodeKind::Test
        | NodeKind::Ac
        | NodeKind::Adr
        | NodeKind::CodeUnit
        | NodeKind::Constitution => {}
    }
    Ok(())
}

/// The edge-attr domains DM §7.2 closes, and DM §17 items 16 and 17.
fn check_edge_attr_domains(kind: EdgeKind, attrs: &Attrs) -> Result<()> {
    let where_ = kind.token();
    match kind {
        EdgeKind::Declares => check_enum(attrs, "polarity", &["expected", "forbidden"], where_)?,
        EdgeKind::Implements => {
            check_enum(attrs, "role", &["landing", "member"], where_)?;
            // DM §17 item 16 and §8.3: "a provisional edge is an in-flight
            // changeset's, and §8.2 excluded the changeset". DM §7.2 calls a
            // `true` here "a conformance failure, and a cheap one to test for".
            if let Some(AttrValue::Bool(true)) = attrs.get("provisional") {
                return Err(Refusal::new(
                    Status::AttrsOutOfProfile,
                    "implements.provisional is true, but §8.2 excluded in-flight changesets",
                ));
            }
        }
        EdgeKind::Protects => {
            // DM §17 item 17 and §8.5 clause 2: the shipped floor "is not in
            // the repository at all; it is inside the binary", and including it
            // "would make the dump a function of the release, which §3.4
            // forbids". Only `C-A2` entries survive, and they are `false`.
            if let Some(AttrValue::Bool(true)) = attrs.get("floor") {
                return Err(Refusal::new(
                    Status::AttrsOutOfProfile,
                    "protects.floor is true, but §8.5 excluded the shipped floor",
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

fn check_enum(attrs: &Attrs, name: &str, domain: &[&str], where_: &str) -> Result<()> {
    match attrs.get(name) {
        Some(AttrValue::Str(v)) if !domain.contains(&v.as_str()) => Err(Refusal::new(
            Status::AttrsOutOfProfile,
            format!("{where_}: {name} {v:?}"),
        )),
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// G10
// ---------------------------------------------------------------------------

/// DM §11's outcome. G10 has no third result: PB §6.3 gives it no deferred
/// mode, and DM §11 step 6 gives the failure no retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum G10 {
    /// The two dumps are byte-identical. *"G10 comparing two [empty dumps] is a
    /// pass, and correctly so: two clones that both derive nothing from the
    /// same tip agree."* (DM §9)
    Pass,
    /// *"the push is refused, `L` is discarded, the run ends
    /// `reconstruction-failed` without a retry and without consuming a `C-M3`
    /// re-verification, and the run's own report is the only record."* (DM §11)
    ReconstructionFailed,
}

impl G10 {
    /// The intent state a failure produces (PB §5.5's transition table).
    pub fn token(self) -> &'static str {
        match self {
            G10::Pass => "pass",
            G10::ReconstructionFailed => "reconstruction-failed",
        }
    }
}

/// DM §11 step 5: *"The comparison is `D_S == D_C` as byte strings. Equal
/// digests (§2.5) are an equivalent implementation. Nothing parses either
/// stream."*
///
/// Steps 1–4 — the scratch clone, `git clone --no-local --no-hardlinks`,
/// `GIT_CONFIG_GLOBAL=/dev/null`, the trust root written into both sides — are
/// the runner's, not this crate's.
///
/// The version check comes first. DM §3.2: a skew "cannot arise" between two
/// dumps one binary produced in one process tree, so observing one "is a defect
/// in that implementation and the run refuses with `dump-version-skew`, exit 3,
/// rather than comparing." Comparing anyway would report the defect as a ledger
/// failure.
pub fn g10_compare(d_s: &Dump, d_c: &Dump) -> Result<G10> {
    if d_s.dump_version != d_c.dump_version || d_s.schema_version != d_c.schema_version {
        return Err(Refusal::new(
            Status::DumpVersionSkew,
            format!(
                "{}/{} vs {}/{}",
                d_s.dump_version, d_s.schema_version, d_c.dump_version, d_c.schema_version
            ),
        ));
    }
    if d_s.bytes() == d_c.bytes() {
        Ok(G10::Pass)
    } else {
        Ok(G10::ReconstructionFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Src, id};
    use crate::store::{Edge, Node};

    /// DM §5.3: a malformed endpoint "surfaces as `id-out-of-grammar`". Edge
    /// endpoints were validated by nothing until 2026-08-28.
    #[test]
    fn an_edge_endpoint_outside_the_repo_prefix_is_refused() {
        let src = Src::Trailer {
            sha: "de841d39b7a84111dfbcc11ddc7a75aa9886b218".into(),
            name: "Spine-Seal".into(),
        };
        let mut g = Graph::new();
        g.add_node(Node::new(
            NodeKind::Intent,
            id::intent("myrepo", "INT-042"),
            complete_intent_attrs(),
            src.clone(),
        ));
        g.add_edge(Edge::new(
            EdgeKind::HasAc,
            id::intent("myrepo", "INT-042"),
            // Another repository's ac id: well formed, and not this dump's.
            String::from("otherrepo/INT-042/AC-1"),
            Attrs::new(),
            src,
        ));
        let refusal = serialize(&empty_header(), &g).unwrap_err();
        assert_eq!(refusal.status, Status::IdOutOfGrammar);
    }

    /// DM §13.4's table is "load-bearing … Getting one backwards is a
    /// whole-dump diff, so all fifteen are written down" — and until
    /// 2026-08-28 nothing consulted it.
    #[test]
    fn an_edge_whose_endpoint_is_the_wrong_node_kind_is_refused() {
        let src = Src::Trailer {
            sha: "de841d39b7a84111dfbcc11ddc7a75aa9886b218".into(),
            name: "Spine-Seal".into(),
        };
        let mut g = Graph::new();
        g.add_node(Node::new(
            NodeKind::Intent,
            id::intent("myrepo", "INT-042"),
            complete_intent_attrs(),
            src.clone(),
        ));
        g.add_node(Node::new(
            NodeKind::Ac,
            id::ac("myrepo", "INT-042", 1),
            Attrs::new(),
            src.clone(),
        ));
        // `has_ac` runs intent -> ac. Backwards, both endpoints exist and both
        // ids are well formed, so only the kind table can catch it.
        g.add_edge(Edge::new(
            EdgeKind::HasAc,
            id::ac("myrepo", "INT-042", 1),
            id::intent("myrepo", "INT-042"),
            Attrs::new(),
            src,
        ));
        let refusal = serialize(&empty_header(), &g).unwrap_err();
        assert_eq!(refusal.status, Status::IdOutOfGrammar);
        assert!(
            refusal.where_.contains("has_ac"),
            "the refusal names the edge kind: {}",
            refusal.where_
        );
    }

    /// An abbreviated or uppercase oid compares unequal to every id git
    /// produces, so it is a value that can never match and can never be
    /// noticed. DM's four attr types have no oid among them, so the type layer
    /// cannot carry this.
    #[test]
    fn an_oid_valued_attr_that_is_not_an_oid_is_refused() {
        let src = Src::Trailer {
            sha: "de841d39b7a84111dfbcc11ddc7a75aa9886b218".into(),
            name: "Spine-Seal".into(),
        };
        for (name, bad) in [
            ("blob", "abc123f"),
            ("landing", "DE841D39B7A84111DFBCC11DDC7A75AA9886B218"),
            ("base", ""),
        ] {
            let mut attrs = complete_intent_attrs();
            attrs = attrs.str(name, bad);
            let mut g = Graph::new();
            g.add_node(Node::new(
                NodeKind::Intent,
                id::intent("myrepo", "INT-042"),
                attrs,
                src.clone(),
            ));
            let refusal = match serialize(&empty_header(), &g) {
                Err(refusal) => refusal,
                Ok(_) => panic!("{name}={bad:?} must be refused"),
            };
            assert_eq!(refusal.status, Status::AttrsOutOfProfile, "{name}");
        }
    }

    /// DM §7.2's presence column. `changeset.landing` is the one that mattered:
    /// the member-changeset rule is written over the value being present and
    /// `false`, so an omission escaped it entirely.
    #[test]
    fn an_always_present_attr_that_is_absent_is_refused() {
        let src = Src::Trailer {
            sha: "de841d39b7a84111dfbcc11ddc7a75aa9886b218".into(),
            name: "Spine-Seal".into(),
        };
        let mut g = Graph::new();
        g.add_node(Node::new(
            NodeKind::Changeset,
            id::changeset("myrepo", "de841d39b7a84111dfbcc11ddc7a75aa9886b218"),
            // No `landing`, but every seal field a landing carries.
            Attrs::new().str("lane", "quick").str("event", "land"),
            src,
        ));
        let refusal = serialize(&empty_header(), &g).unwrap_err();
        assert_eq!(refusal.status, Status::AttrsOutOfProfile);
        assert!(refusal.where_.contains("landing"), "{}", refusal.where_);
    }

    /// An `intent` node carrying every attr DM §7.2 marks `always`.
    fn complete_intent_attrs() -> Attrs {
        Attrs::new()
            .str("status", "merged")
            .bytes("title", b"Invoice totals include tax")
            .str("template", "intent@2")
            .str("blob", "1f0c0a1e2b3c4d5e6f708192a3b4c5d6e7f80912")
            .int("reopen_count", 0)
            .int("late_reopen_count", 0)
            .str("landing", "de841d39b7a84111dfbcc11ddc7a75aa9886b218")
            .str("base", "1cbc18507888cb238c56ce00ba678c16564e0274")
    }

    fn empty_header() -> Header {
        Header {
            object_format: ObjectFormat::Sha1,
            repo: "myrepo".to_string(),
            trunk: b"main".to_vec(),
            head: None,
            trust_root: None,
        }
    }

    /// DM §12.5, reproduced from the bytes rather than quoted at.
    #[test]
    fn the_empty_dump_is_one_header_line_of_105_bytes() {
        let dump = serialize(&empty_header(), &Graph::new()).unwrap();
        assert_eq!(
            dump.bytes(),
            b"{\"dump_version\":1,\"object_format\":\"sha1\",\"repo\":\"myrepo\",\"schema_version\":7,\"t\":\"header\",\"trunk\":\"main\"}\n"
        );
        assert_eq!(dump.len(), 105);
        assert_eq!(dump.line_count(), 1);
        assert_eq!(
            dump.digest(),
            "sha256:2a3fcea5debff8e17af36eef1a4acfa1fea9f86a0d6c942cf2993cce3e80f290"
        );
    }

    #[test]
    fn the_last_line_is_terminated_and_the_digest_covers_that_lf() {
        // DM §2.2 and §2.5. The trailing-LF rule inverts across the corpus, so
        // this is pinned rather than assumed: strip the final LF and the digest
        // moves, which is a terminal G10 failure.
        let dump = serialize(&empty_header(), &Graph::new()).unwrap();
        assert_eq!(*dump.bytes().last().unwrap(), LF);
        let without = &dump.bytes()[..dump.len() - 1];
        assert_ne!(sha256_prefixed(without), dump.digest());
    }

    #[test]
    fn an_absent_head_is_an_absent_member_never_a_null() {
        // DM §9 case 2: a manifest resolves but `refs/heads/<trunk>` does not
        // — "exactly what a mis-cloned G10 side looks like".
        let dump = serialize(&empty_header(), &Graph::new()).unwrap();
        let text = String::from_utf8(dump.bytes().to_vec()).unwrap();
        // `"head"` and not `head`: the record's own `t` is `"header"`, and a
        // substring test that cannot tell the two apart is a test that passes
        // for the wrong reason.
        assert!(!text.contains("\"head\":"));
        assert!(!text.contains("\"trust_root\":"));
        assert!(!text.contains("null"));
    }

    #[test]
    fn a_head_and_a_trust_root_appear_on_line_one_so_a_mismatch_is_a_line_one_diff() {
        let mut header = empty_header();
        header.head = Some("1b2c3d4e5f60718293a4b5c6d7e8f90123456789".into());
        header.trust_root = Some("0a1b2c3d4e5f60718293a4b5c6d7e8f901234567".into());
        let dump = serialize(&header, &Graph::new()).unwrap();
        assert_eq!(
            String::from_utf8(dump.bytes().to_vec()).unwrap(),
            "{\"dump_version\":1,\"head\":\"1b2c3d4e5f60718293a4b5c6d7e8f90123456789\",\"object_format\":\"sha1\",\"repo\":\"myrepo\",\"schema_version\":7,\"t\":\"header\",\"trunk\":\"main\",\"trust_root\":\"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567\"}\n"
        );
    }

    #[test]
    fn a_repo_outside_the_grammar_refuses_with_id_out_of_grammar() {
        // DM §5.2: "Without it, `myrepo/INT-042/AC-1` is ambiguous."
        let mut header = empty_header();
        header.repo = "my/repo".to_string();
        let err = serialize(&header, &Graph::new()).unwrap_err();
        assert_eq!(err.status.token(), "id-out-of-grammar");
    }

    #[test]
    fn an_exercises_edge_is_refused_because_its_source_is_a_coverage_report() {
        let mut g = Graph::new();
        g.add_edge(Edge::new(
            EdgeKind::Exercises,
            "myrepo/code:src/a.py",
            "myrepo/test:pytest:t.py::a",
            Attrs::new(),
            Src::Commit {
                sha: "1b2c3d4e5f60718293a4b5c6d7e8f90123456789".into(),
            },
        ));
        let err = serialize(&empty_header(), &g).unwrap_err();
        assert_eq!(err.status.token(), "attrs-out-of-profile");
    }

    #[test]
    fn a_provisional_implements_edge_is_refused() {
        let mut g = Graph::new();
        g.add_edge(Edge::new(
            EdgeKind::Implements,
            "myrepo/cs:1b2c3d4e5f60718293a4b5c6d7e8f90123456789",
            "myrepo/INT-042",
            Attrs::new()
                .str("role", "landing")
                .bool("provisional", true)
                .bool("verified", true),
            Src::Trailer {
                sha: "1b2c3d4e5f60718293a4b5c6d7e8f90123456789".into(),
                name: "Spine-Seal".into(),
            },
        ));
        assert!(serialize(&empty_header(), &g).is_err());
    }

    #[test]
    fn a_shipped_floor_protects_edge_is_refused() {
        let mut g = Graph::new();
        g.add_edge(Edge::new(
            EdgeKind::Protects,
            "myrepo/constitution:v3",
            "myrepo/code:infra/",
            Attrs::new().bool("floor", true),
            Src::FileLineAt {
                sha: "1b2c3d4e5f60718293a4b5c6d7e8f90123456789".into(),
                path: b"CONSTITUTION.md".to_vec(),
                line: 96,
            },
        ));
        assert!(serialize(&empty_header(), &g).is_err());
    }

    #[test]
    fn a_dagger_intent_status_is_refused_because_no_landed_intent_has_one() {
        // DM §8.6: the clause is vacuous under §8.2 and kept anyway, because
        // "it is a conformance check with teeth — a dump containing a † status
        // is non-conforming and the check is one string comparison".
        let mut g = Graph::new();
        g.add_node(Node::new(
            NodeKind::Intent,
            id::intent("myrepo", "INT-042"),
            Attrs::new().str("status", "checked†"),
            Src::MessageLine {
                sha: "1b2c3d4e5f60718293a4b5c6d7e8f90123456789".into(),
                line: 14,
            },
        ));
        let err = serialize(&empty_header(), &g).unwrap_err();
        assert_eq!(err.status.token(), "attrs-out-of-profile");
    }

    #[test]
    fn a_member_changeset_carrying_a_seal_field_is_refused() {
        let mut g = Graph::new();
        g.add_node(Node::new(
            NodeKind::Changeset,
            id::changeset("myrepo", "2c3d4e5f60718293a4b5c6d7e8f9012345678901"),
            Attrs::new().bool("landing", false).str("lane", "gated"),
            Src::Commit {
                sha: "2c3d4e5f60718293a4b5c6d7e8f9012345678901".into(),
            },
        ));
        assert!(serialize(&empty_header(), &g).is_err());
    }

    #[test]
    fn a_test_node_cannot_carry_result_at() {
        // RF §2: the result file "Populates the volatile `test.result_at`
        // attrs only … so a result file can never affect reconstruction."
        let mut g = Graph::new();
        g.add_node(Node::new(
            NodeKind::Test,
            id::test("myrepo", "pytest", b"t.py::a"),
            Attrs::new().bool("result_at", true),
            Src::FileLineAt {
                sha: "1b2c3d4e5f60718293a4b5c6d7e8f90123456789".into(),
                path: b"t.py".to_vec(),
                line: 7,
            },
        ));
        assert!(serialize(&empty_header(), &g).is_err());
    }

    #[test]
    fn two_identical_dumps_pass_and_one_differing_byte_is_terminal() {
        let a = serialize(&empty_header(), &Graph::new()).unwrap();
        let b = serialize(&empty_header(), &Graph::new()).unwrap();
        assert_eq!(g10_compare(&a, &b).unwrap(), G10::Pass);

        let mut other = empty_header();
        other.trunk = b"trunk".to_vec();
        let c = serialize(&other, &Graph::new()).unwrap();
        assert_eq!(g10_compare(&a, &c).unwrap(), G10::ReconstructionFailed);
        assert_eq!(G10::ReconstructionFailed.token(), "reconstruction-failed");
    }

    #[test]
    fn a_version_skew_refuses_rather_than_comparing() {
        let a = serialize(&empty_header(), &Graph::new()).unwrap();
        let mut b = a.clone();
        b.dump_version = 2;
        let err = g10_compare(&a, &b).unwrap_err();
        assert_eq!(err.status.token(), "dump-version-skew");
        assert_eq!(err.status.exit_code(), 3);
    }

    #[test]
    fn a_dump_is_not_a_fixpoint_of_a_line_sort() {
        // DM §6.5, stated "because an implementer will otherwise discover
        // [it] the hard way": JCS puts `{"attrs":` at the head of every node
        // and edge line, so the sort key is not a prefix of the line, an empty
        // `attrs` sorts *after* a non-empty one at the line level, and a line
        // sort would interleave the two sections.
        let mut g = Graph::new();
        let src = Src::MessageLine {
            sha: "1b2c3d4e5f60718293a4b5c6d7e8f90123456789".into(),
            line: 24,
        };
        g.add_node(Node::new(
            NodeKind::Ac,
            id::ac("myrepo", "INT-042", 1),
            Attrs::new(),
            src.clone(),
        ));
        // A complete intent node: DM §7.2 marks eight of its attrs `always`,
        // and `serialize` enforces the presence column, so a stub would refuse
        // here for a reason that has nothing to do with what this test pins.
        g.add_node(Node::new(
            NodeKind::Intent,
            id::intent("myrepo", "INT-042"),
            complete_intent_attrs(),
            src.clone(),
        ));
        g.add_edge(Edge::new(
            EdgeKind::HasAc,
            id::intent("myrepo", "INT-042"),
            id::ac("myrepo", "INT-042", 1),
            Attrs::new(),
            src,
        ));

        let dump = serialize(&empty_header(), &g).unwrap();
        let text = String::from_utf8(dump.bytes().to_vec()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let mut sorted = lines.clone();
        sorted.sort_unstable();
        assert_ne!(lines, sorted);
    }
}
