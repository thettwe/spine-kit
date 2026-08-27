//! DM §12.4, the ordering vector — *"Debug your comparator against this before
//! attempting §12.2. It exercises every tie-break level and the `esc`-versus-raw
//! ordering trap, and nothing else."*
//!
//! It is a **fragment**, not a dump: eleven records, each LF-terminated, no
//! header. Its oids (`aa`, `bb`, `cc`) are deliberately not oids, which is why
//! it is exercised through the ordering and the record writers rather than
//! through `serialize`, whose job is to refuse exactly that.
//!
//! `tests/vectors/dm-12-4-ordering.jsonl` holds the canonical block's bytes,
//! copied from `docs/spec/dump.md`. Nothing here asserts a digest it did not
//! compute: the digest is taken over the fixture, and the fixture is what the
//! serializer's output is compared against.

use spine_canon::sha256_prefixed;
use spine_graph::schema::{Attrs, EdgeKind, NodeKind, Src, id};
use spine_graph::store::{Edge, Graph, Node};
use spine_graph::{edge_line, node_line};

const CANONICAL: &[u8] = include_bytes!("vectors/dm-12-4-ordering.jsonl");

/// The eleven records **in DM §12.4's authored order**, which the document
/// calls "an arbitrary authored order". Feeding them in that order is the
/// point: a serializer that emitted insertion order would reproduce the
/// *Authored* block, not the *Canonical* one.
fn authored() -> Graph {
    let mut g = Graph::new();
    let commit = |sha: &str| Src::Commit { sha: sha.into() };
    let msg = |sha: &str, line: u64| Src::MessageLine {
        sha: sha.into(),
        line,
    };

    // `src/` + 0xE9 + `.py` — a Latin-1 `é`, one byte in the tree.
    let mut latin1 = b"src/".to_vec();
    latin1.push(0xE9);
    latin1.extend_from_slice(b".py");

    g.add_node(Node::new(
        NodeKind::CodeUnit,
        id::code_unit("r", b"src/z.py"),
        Attrs::new(),
        commit("aa"),
    ));
    g.add_node(Node::new(
        NodeKind::CodeUnit,
        id::code_unit("r", &latin1),
        Attrs::new(),
        commit("aa"),
    ));
    g.add_node(Node::new(
        NodeKind::Changeset,
        id::changeset("r", "bb"),
        Attrs::new().bool("landing", false),
        commit("bb"),
    ));
    g.add_node(Node::new(
        NodeKind::Ac,
        id::ac("r", "INT-1", 2),
        Attrs::new(),
        msg("cc", 9),
    ));
    g.add_node(Node::new(
        NodeKind::Ac,
        id::ac("r", "INT-1", 10),
        Attrs::new(),
        msg("cc", 8),
    ));

    g.add_edge(Edge::new(
        EdgeKind::Declares,
        id::intent("r", "INT-1"),
        id::code_unit("r", b"src/z.py"),
        Attrs::new().str("polarity", "forbidden"),
        msg("cc", 3),
    ));
    g.add_edge(Edge::new(
        EdgeKind::Declares,
        id::intent("r", "INT-1"),
        id::code_unit("r", b"src/z.py"),
        Attrs::new().str("polarity", "expected"),
        msg("cc", 2),
    ));
    g.add_edge(Edge::new(
        EdgeKind::HasAc,
        id::intent("r", "INT-1"),
        id::ac("r", "INT-1", 10),
        Attrs::new(),
        msg("cc", 8),
    ));
    g.add_edge(Edge::new(
        EdgeKind::HasAc,
        id::intent("r", "INT-1"),
        id::ac("r", "INT-1", 2),
        Attrs::new(),
        msg("cc", 9),
    ));
    // The last two tie on `from`, `to`, `kind` *and* `attrs`, so DM §5.5
    // collapses them to one record with the minimum `src` — `git:aa`. The
    // canonical block prints both because it is a fragment of *authored*
    // records, not of stored ones; the collapse is asserted separately below.
    g.add_edge(Edge::new(
        EdgeKind::Modifies,
        id::changeset("r", "bb"),
        id::code_unit("r", &latin1),
        Attrs::new(),
        commit("bb"),
    ));
    g.add_edge(Edge::new(
        EdgeKind::Modifies,
        id::changeset("r", "bb"),
        id::code_unit("r", &latin1),
        Attrs::new(),
        commit("aa"),
    ));
    g
}

/// The fragment's bytes: every ordered node line, then every ordered edge line.
fn render(g: &Graph) -> Vec<u8> {
    let mut out = Vec::new();
    for node in g.ordered_nodes() {
        out.extend_from_slice(&node_line(node));
    }
    for edge in g.ordered_edges() {
        out.extend_from_slice(&edge_line(edge));
    }
    out
}

#[test]
fn the_published_fragment_is_1063_bytes_and_hashes_to_a849ec34() {
    // The vector's own header, reproduced from the fixture's bytes rather than
    // taken on trust.
    assert_eq!(CANONICAL.len(), 1063);
    assert_eq!(CANONICAL.iter().filter(|&&b| b == 0x0A).count(), 11);
    assert_eq!(
        sha256_prefixed(CANONICAL),
        "sha256:a849ec349ef8f20ec1f40423ae6a7d3358745f4c9027545f55cf74ef9b72a139"
    );
}

#[test]
fn the_ten_stored_records_are_the_published_fragment_less_the_collapsed_duplicate() {
    // DM §5.5 collapses the two `modifies` records, which are equal in `from`,
    // `to`, `kind` and `attrs`, to one with the minimum `src`. The published
    // fragment shows both because it is authored records, not stored ones — so
    // the store's rendering is the fragment's first ten lines.
    let produced = render(&authored());
    let expected_lines: Vec<&[u8]> = CANONICAL.split_inclusive(|&b| b == 0x0A).take(10).collect();
    let expected: Vec<u8> = expected_lines.concat();
    assert_eq!(
        String::from_utf8_lossy(&produced),
        String::from_utf8_lossy(&expected)
    );
}

#[test]
fn every_tie_break_level_of_dm_12_4_holds_line_by_line() {
    let produced = render(&authored());
    let lines: Vec<String> = produced
        .split_inclusive(|&b| b == 0x0A)
        .map(|l| String::from_utf8_lossy(l).trim_end().to_string())
        .collect();

    // "`AC-10` before `AC-2` — byte order, not numeric (§6.4)"
    assert!(lines[0].contains("r/INT-1/AC-10"));
    assert!(lines[1].contains("r/INT-1/AC-2"));
    // "`ac` < `changeset` < `code_unit` — `kind` is the first node key"
    assert!(lines[2].contains("\"kind\":\"changeset\""));
    // "`\xe9.py` before `z.py` — `esc` order, the reverse of raw-byte order"
    assert!(lines[3].contains("r/code:src/\\\\xe9.py"));
    assert!(lines[4].contains("r/code:src/z.py"));
    // "edges under one `from` are ordered by `to` before `kind`, so both
    // `has_ac` edges precede both `declares` edges — PB §6.3's `from,to,kind`"
    assert!(lines[5].contains("\"kind\":\"has_ac\""));
    assert!(lines[6].contains("\"kind\":\"has_ac\""));
    // "the two `declares` edges tie on `from`, `to` and `kind` and break on
    // `attrs`: `expected` before `forbidden`"
    assert!(lines[7].contains("\"polarity\":\"expected\""));
    assert!(lines[8].contains("\"polarity\":\"forbidden\""));
    // "the two `modifies` edges tie on … `attrs` and break on `src`: `git:aa`
    // before `git:bb`" — and DM §5.5 keeps only the minimum.
    assert!(lines[9].contains("\"src\":\"git:aa\""));
    assert_eq!(lines.len(), 10);
}

#[test]
fn the_esc_encoded_path_is_two_backslashes_on_the_wire() {
    // Two layers, and both are visible in the artifact: `esc` turns 0xE9 into
    // the four characters `\x e 9`, and JSON then escapes that backslash. A
    // serializer that applied only one of the two produces a byte-different
    // dump, which is a terminal G10 failure.
    let text = String::from_utf8(render(&authored())).unwrap();
    assert!(text.contains(r#""id":"r/code:src/\\xe9.py""#));
}
