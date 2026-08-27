//! The in-memory traceability graph and its total order.
//!
//! PB §6.2 puts the canonical store in SQLite (`.spine/cache/graph.sqlite`,
//! `PRAGMA user_version = 7`). That file is **not** built here — a SQLite
//! binding is a dependency this crate may not add — so what follows is the
//! model the store holds and the ordering the dump projects, and the
//! persistence layer is reported as not implemented.
//!
//! Losing persistence loses less than it sounds: DM §4.3 makes `--dump` imply
//! `--fresh`, and PB §7.4 rule 3 forbids a persisted, fetched or restored store
//! from reaching a dump at all. The projection is computed "from a graph built
//! in this process from git objects alone", which is exactly what a [`Graph`]
//! is.
//!
//! **The dump is a projection of the graph, not the graph** (DM §1). A
//! [`Graph`] built for `spine check` legitimately holds in-flight intents,
//! provisional changesets, volatile results and the shipped floor; the
//! serializer refuses those (DM §8), so building the projection is the caller's
//! job and refusing a non-projection is this crate's.

use crate::schema::{Attrs, EdgeKind, NodeKind, Src};

/// A node record. PB §6.1's law is in the type: `src` is not an `Option`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: NodeKind,
    pub id: String,
    pub attrs: Attrs,
    pub src: Src,
}

impl Node {
    pub fn new(kind: NodeKind, id: impl Into<String>, attrs: Attrs, src: Src) -> Self {
        Node {
            kind,
            id: id.into(),
            attrs,
            src,
        }
    }
}

/// An edge record.
///
/// Directions are [`EdgeKind::endpoints`]'s. Two are this implementation's,
/// because DM §5.3's direction paragraph names twelve of the fifteen and
/// DM §13.4 claims all fifteen:
///
/// **DERIVED** — `supersedes` runs superseding → superseded and
/// `superseded_by` runs superseded → superseding, and **both are emitted** for
/// one supersession. PB §6.6's only clue is *"the indexer emits
/// `superseded_by`"*, and PB §6.2 lists both kinds; emitting one would leave
/// the other kind dead in a closed set, and emitting them in one direction
/// would make the names lie. Two edges cost two lines and make PB §6.6's
/// promise — *"archaeology queries return the current truth first and the
/// history behind it"* — answerable from either end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub kind: EdgeKind,
    pub from: String,
    pub to: String,
    pub attrs: Attrs,
    pub src: Src,
}

impl Edge {
    pub fn new(
        kind: EdgeKind,
        from: impl Into<String>,
        to: impl Into<String>,
        attrs: Attrs,
        src: Src,
    ) -> Self {
        Edge {
            kind,
            from: from.into(),
            to: to.into(),
            attrs,
            src,
        }
    }
}

/// The derived graph: nodes keyed by id, edges keyed by
/// `(from, to, kind, attrs)`.
#[derive(Debug, Clone, Default)]
pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl Graph {
    pub fn new() -> Self {
        Graph::default()
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Add a node, applying DM §5.5's collapse.
    ///
    /// > *"When a derivation produces the same element from more than one
    /// > citation, the dump emits one record whose `src` is the minimum, under
    /// > §6.4's ordering, of those citations."*
    ///
    /// This is what makes a `code_unit` deterministic: it "is named by every
    /// edge that touches it, and 'the first one the walk happened to reach' is
    /// walk order, which is not a specification" (DM §13.5). The minimum is
    /// total, cheap, and independent of traversal.
    ///
    /// **DERIVED** in one corner: DM §5.5 fixes the choice of `src` for records
    /// that agree in everything else, and PB §6.2 makes `id` a `PRIMARY KEY`,
    /// so two records sharing an id and disagreeing in `attrs` are an indexer
    /// defect the store could not hold either. Rather than pick by insertion
    /// order — which is walk order, the thing §13.5 refuses — the minimum is
    /// taken over `canonical(attrs) ‖ NUL ‖ src`, the tail of the node key.
    /// Where the attrs agree this reduces to §5.5's rule exactly.
    pub fn add_node(&mut self, node: Node) {
        match self.nodes.iter_mut().find(|n| n.id == node.id) {
            Some(existing) => {
                if node_tail(&node) < node_tail(existing) {
                    *existing = node;
                }
            }
            None => self.nodes.push(node),
        }
    }

    /// Add an edge, applying DM §5.5's collapse.
    ///
    /// > *"records that are equal in `from`, `to`, `kind` and `attrs` collapse
    /// > to one with the minimum `src`; records that differ in `attrs` are two
    /// > edges and both are emitted (§6.3)."*
    ///
    /// The second half is why `attrs` is a sort key and not part of the
    /// identity: DM §6.3 points at "a `declares` edge naming one path as both
    /// `expected` and `forbidden`, for instance, which is a malformed intent
    /// doc and not this format's to refuse."
    pub fn add_edge(&mut self, edge: Edge) {
        match self.edges.iter_mut().find(|e| {
            e.from == edge.from && e.to == edge.to && e.kind == edge.kind && e.attrs == edge.attrs
        }) {
            Some(existing) => {
                if edge.src.render() < existing.src.render() {
                    *existing = edge;
                }
            }
            None => self.edges.push(edge),
        }
    }

    /// Record one supersession: **both** edges, in the two directions
    /// [`Edge`]'s doc comment fixes.
    ///
    /// PB §6.6: *"a later intent whose `Supersedes:` header names this one
    /// lands with a `Spine-Supersedes` trailer; the indexer emits
    /// `superseded_by`, so archaeology queries return the current truth first
    /// and the history behind it."* PB §6.2 lists both kinds and DM §5.3's
    /// direction paragraph names neither, so the pair is emitted together —
    /// one call, so the two can never be derived apart.
    pub fn add_supersession(&mut self, superseding: &str, superseded: &str, src: Src) {
        self.add_edge(Edge::new(
            EdgeKind::Supersedes,
            superseding,
            superseded,
            Attrs::new(),
            src.clone(),
        ));
        self.add_edge(Edge::new(
            EdgeKind::SupersededBy,
            superseded,
            superseding,
            Attrs::new(),
            src,
        ));
    }

    /// The node section, in DM §6.2's order.
    pub fn ordered_nodes(&self) -> Vec<&Node> {
        let mut out: Vec<&Node> = self.nodes.iter().collect();
        out.sort_by_cached_key(|n| node_key(n));
        out
    }

    /// The edge section, in DM §6.3's order.
    pub fn ordered_edges(&self) -> Vec<&Edge> {
        let mut out: Vec<&Edge> = self.edges.iter().collect();
        out.sort_by_cached_key(|e| edge_key(e));
        out
    }

    /// G5's linter, over this graph: every `from` and every `to` that names no
    /// node record.
    ///
    /// PB §6.3 G5: *"in a derived graph, **dangling edges are the linter**.
    /// Traditional traceability systems rot because broken links fail silently;
    /// under the provenance law, a broken link is a build failure with a
    /// `file:line` to fix."*
    ///
    /// The dump cannot detect this: DM §5.3 says *"a well-formed id naming no
    /// node is an indexer defect that this format cannot detect and G5 must"*.
    /// It is offered here, beside the model, for the gate that must.
    pub fn dangling_endpoints(&self) -> Vec<(&Edge, &str)> {
        let mut out = Vec::new();
        for edge in &self.edges {
            for endpoint in [edge.from.as_str(), edge.to.as_str()] {
                if !self.nodes.iter().any(|n| n.id == endpoint) {
                    out.push((edge, endpoint));
                }
            }
        }
        out
    }
}

/// The byte `0x00`, DM §6.4's separator.
///
/// *"No component can contain `0x00`: `esc` maps it to the four characters
/// `\x00`, and a JCS-serialized `attrs` is JSON text. So comparing the
/// concatenations is exactly comparing the components in order, and the classic
/// separator hazard — `a/b` sorting against `a-b` — cannot arise."*
const NUL: u8 = 0x00;

/// DM §6.2: `key_node(r) = r.kind ‖ NUL ‖ r.id ‖ NUL ‖ canonical(r.attrs) ‖ NUL ‖ r.src`.
///
/// `kind` before `id` is PB §6.3's own clause — *"nodes sorted by kind,id"* —
/// and DM §6.2 keeps it "even though `id` alone is unique … the two orders
/// differ, and PB's is the one G10 was specified against."
///
/// Comparison of the result is plain unsigned byte order. Every component is
/// ASCII after `esc`, so "byte order, code-point order and UTF-16 code-unit
/// order coincide" (DM §6.4) — which is also why `AC-10` precedes `AC-2` and
/// `src/\xe9.py` precedes `src/z.py`.
pub fn node_key(node: &Node) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(node.kind.token().as_bytes());
    key.push(NUL);
    key.extend_from_slice(node.id.as_bytes());
    key.push(NUL);
    key.extend_from_slice(&node_tail(node));
    key
}

/// `canonical(attrs) ‖ NUL ‖ src` — the part of the node key below `id`.
fn node_tail(node: &Node) -> Vec<u8> {
    let mut tail = spine_canon::canonicalize(&node.attrs.to_value());
    tail.push(NUL);
    tail.extend_from_slice(node.src.render().as_bytes());
    tail
}

/// DM §6.3: `key_edge(r) = r.from ‖ NUL ‖ r.to ‖ NUL ‖ r.kind ‖ NUL ‖ canonical(r.attrs) ‖ NUL ‖ r.src`.
///
/// `from, to, kind` is PB §6.3's clause verbatim; `attrs` and `src` are the
/// tie-breakers DM appends beneath it. Nothing PB fixed is reordered.
pub fn edge_key(edge: &Edge) -> Vec<u8> {
    let mut key = Vec::new();
    key.extend_from_slice(edge.from.as_bytes());
    key.push(NUL);
    key.extend_from_slice(edge.to.as_bytes());
    key.push(NUL);
    key.extend_from_slice(edge.kind.token().as_bytes());
    key.push(NUL);
    key.extend_from_slice(&spine_canon::canonicalize(&edge.attrs.to_value()));
    key.push(NUL);
    key.extend_from_slice(edge.src.render().as_bytes());
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::id;

    fn commit(sha: &str) -> Src {
        Src::Commit {
            sha: sha.to_string(),
        }
    }

    #[test]
    fn a_code_unit_named_by_three_edges_takes_the_minimum_citation() {
        // DM §12.3 check 3: `tests/billing/test_invoice.py` "is cited by
        // `modifies` from `L` and from `M2` and by `freezes` from `M3`, and
        // takes `git:1b2c…6789`, the least of the three."
        let id = id::code_unit("myrepo", b"tests/billing/test_invoice.py");
        let mut g = Graph::new();
        for sha in [
            "3d4e5f60718293a4b5c6d7e8f90123456789012a",
            "1b2c3d4e5f60718293a4b5c6d7e8f90123456789",
            "4e5f60718293a4b5c6d7e8f90123456789012ab3",
        ] {
            g.add_node(Node::new(
                NodeKind::CodeUnit,
                id.clone(),
                Attrs::new(),
                commit(sha),
            ));
        }
        assert_eq!(g.nodes().len(), 1);
        assert_eq!(
            g.nodes()[0].src.render(),
            "git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789"
        );
    }

    #[test]
    fn two_edges_alike_but_for_src_collapse_and_two_alike_but_for_attrs_do_not() {
        // DM §12.4's last two pins, as a model-level rule: the two `modifies`
        // edges "tie on `from`, `to`, `kind` **and** `attrs` and break on
        // `src`", while the two `declares` edges "tie on `from`, `to` and
        // `kind` and break on `attrs`" — so the first pair is one edge and the
        // second is two.
        let mut g = Graph::new();
        g.add_edge(Edge::new(
            EdgeKind::Modifies,
            "r/cs:bb",
            "r/code:src/z.py",
            Attrs::new(),
            commit("bb"),
        ));
        g.add_edge(Edge::new(
            EdgeKind::Modifies,
            "r/cs:bb",
            "r/code:src/z.py",
            Attrs::new(),
            commit("aa"),
        ));
        assert_eq!(g.edges().len(), 1);
        assert_eq!(g.edges()[0].src.render(), "git:aa");

        g.add_edge(Edge::new(
            EdgeKind::Declares,
            "r/INT-1",
            "r/code:src/z.py",
            Attrs::new().str("polarity", "expected"),
            commit("cc"),
        ));
        g.add_edge(Edge::new(
            EdgeKind::Declares,
            "r/INT-1",
            "r/code:src/z.py",
            Attrs::new().str("polarity", "forbidden"),
            commit("cc"),
        ));
        assert_eq!(g.edges().len(), 3);
    }

    #[test]
    fn ac_10_precedes_ac_2_because_the_order_is_over_bytes_not_numbers() {
        // DM §6.4: "`AC-10` precedes `AC-2`; `G11` precedes `G2`. This is
        // deliberate: a byte order over `esc` output is the one order every
        // implementation already has."
        let ten = Node::new(
            NodeKind::Ac,
            id::ac("r", "INT-1", 10),
            Attrs::new(),
            Src::MessageLine {
                sha: "cc".into(),
                line: 8,
            },
        );
        let two = Node::new(
            NodeKind::Ac,
            id::ac("r", "INT-1", 2),
            Attrs::new(),
            Src::MessageLine {
                sha: "cc".into(),
                line: 9,
            },
        );
        assert!(node_key(&ten) < node_key(&two));
    }

    #[test]
    fn an_esc_encoded_path_sorts_before_z_which_raw_bytes_would_reverse() {
        // DM §6.4, the trap this crate's build plan calls R4: `esc` moves every
        // byte above 0x7E into a sequence beginning `\` (0x5C), which sorts
        // below every lowercase letter.
        let mut latin1 = b"src/".to_vec();
        latin1.push(0xE9);
        latin1.extend_from_slice(b".py");

        // Raw bytes put `z` first, because 0x7A < 0xE9.
        assert!(b"src/z.py".as_slice() < latin1.as_slice());

        // The encoded bytes — the ones in the artifact — reverse it.
        let encoded = Node::new(
            NodeKind::CodeUnit,
            id::code_unit("r", &latin1),
            Attrs::new(),
            commit("aa"),
        );
        let plain = Node::new(
            NodeKind::CodeUnit,
            id::code_unit("r", b"src/z.py"),
            Attrs::new(),
            commit("aa"),
        );
        assert!(node_key(&encoded) < node_key(&plain));
    }

    #[test]
    fn an_edge_orders_by_to_before_kind_which_is_pbs_own_clause() {
        // DM §12.4's last pin: "edges under one `from` are ordered by `to`
        // before `kind`, so both `has_ac` edges precede both `declares` edges
        // — PB §6.3's `from,to,kind` exactly." `has_ac` sorts after `declares`
        // as a token, so a kind-first key would reverse this pair.
        let has_ac = Edge::new(
            EdgeKind::HasAc,
            "r/INT-1",
            "r/INT-1/AC-2",
            Attrs::new(),
            Src::MessageLine {
                sha: "cc".into(),
                line: 9,
            },
        );
        let declares = Edge::new(
            EdgeKind::Declares,
            "r/INT-1",
            "r/code:src/z.py",
            Attrs::new().str("polarity", "expected"),
            Src::MessageLine {
                sha: "cc".into(),
                line: 2,
            },
        );
        assert!(edge_key(&has_ac) < edge_key(&declares));
        assert!(has_ac.kind.token() > declares.kind.token());
    }

    #[test]
    fn a_supersession_emits_both_edges_in_opposite_directions() {
        let mut g = Graph::new();
        g.add_supersession(
            &id::intent("r", "INT-9"),
            &id::intent("r", "INT-1"),
            Src::Trailer {
                sha: "aa".into(),
                name: "Spine-Supersedes".into(),
            },
        );
        assert_eq!(g.edges().len(), 2);
        let supersedes = g
            .edges()
            .iter()
            .find(|e| e.kind == EdgeKind::Supersedes)
            .unwrap();
        assert_eq!(
            (supersedes.from.as_str(), supersedes.to.as_str()),
            ("r/INT-9", "r/INT-1")
        );
        let superseded_by = g
            .edges()
            .iter()
            .find(|e| e.kind == EdgeKind::SupersededBy)
            .unwrap();
        assert_eq!(
            (superseded_by.from.as_str(), superseded_by.to.as_str()),
            ("r/INT-1", "r/INT-9")
        );
    }

    #[test]
    fn a_dangling_endpoint_is_reported_because_g5_must_find_what_the_dump_cannot() {
        let mut g = Graph::new();
        g.add_node(Node::new(
            NodeKind::Test,
            "r/test:pytest:t.py::a",
            Attrs::new(),
            commit("aa"),
        ));
        g.add_edge(Edge::new(
            EdgeKind::VerifiedBy,
            "r/test:pytest:t.py::a",
            "r/INT-1/AC-9",
            Attrs::new().bool("attributed", true),
            commit("aa"),
        ));
        let dangling = g.dangling_endpoints();
        assert_eq!(dangling.len(), 1);
        assert_eq!(dangling[0].1, "r/INT-1/AC-9");
    }
}
