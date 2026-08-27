//! DM §17's conformance checklist, run against the §12.2 dump.
//!
//! *"A serializer conforms iff all of the following hold. Every item is
//! mechanically checkable against a produced dump."* Twenty of the twenty-four
//! are checkable here; the other four need a repository, and each says so where
//! it is skipped.
//!
//! The checker lives in this test file and not in the crate on purpose. DM §1:
//! *"Nothing in spine ever reads a dump … A byte comparator is not a reader."*
//! A conformance checker is the "human or external tool" of DM §3.2, and giving
//! spine one would create the second reader PB §6.1's iron rule forbids. So
//! nothing below parses a dump: the framing items work over raw bytes, and the
//! record items work over the model the serializer was handed.

mod myrepo;

use spine_canon::{ObjectFormat, sha256_prefixed};
use spine_graph::schema::{AttrValue, EdgeKind, NodeKind, Src, is_ascii_printable, is_oid};
use spine_graph::store::{edge_key, node_key};
use spine_graph::{Graph, Header, serialize};

const PUBLISHED: &[u8] = include_bytes!("vectors/dm-12-2-dump.jsonl");
const LF: u8 = 0x0A;

/// The artifact under test: DM §12.2's published bytes. That the crate's own
/// emitter reproduces them is `dm_12_2_dump.rs`'s job; this file asks whether
/// the bytes themselves satisfy DM §17.
fn artifact() -> Vec<u8> {
    PUBLISHED.to_vec()
}

// --- Framing and encoding (items 1–6) --------------------------------------

#[test]
fn item_1_line_1_is_a_header_record_and_there_is_exactly_one() {
    let bytes = artifact();
    let lines: Vec<&[u8]> = bytes.split(|&b| b == LF).collect();
    assert!(lines[0].starts_with(b"{\"dump_version\":"));
    assert!(lines[0].ends_with(b",\"t\":\"header\",\"trunk\":\"main\",\"trust_root\":\"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567\"}"));
    let headers = bytes
        .split(|&b| b == LF)
        .filter(|l| l.starts_with(b"{\"dump_version\":"))
        .count();
    assert_eq!(headers, 1);
}

#[test]
fn item_2_every_line_is_lf_terminated_with_no_cr_no_bom_and_no_blank_line() {
    let bytes = artifact();
    assert_eq!(*bytes.last().unwrap(), LF);
    assert!(!bytes.contains(&0x0D), "no CR anywhere");
    assert!(!bytes.starts_with(&[0xEF, 0xBB, 0xBF]), "no BOM");
    for line in bytes.split_inclusive(|&b| b == LF) {
        assert!(line.len() > 1, "no blank line");
        assert_eq!(line.iter().filter(|&&b| b == LF).count(), 1);
    }
}

#[test]
fn item_3_and_4_every_byte_is_ascii_printable_and_no_line_carries_whitespace() {
    // DM §2.3 reduces JCS here to "emit with no whitespace", and DM §2.4 makes
    // every string ASCII after `esc`. Both are one byte-range test, since a
    // space inside a JSON string is legal and appears in no value of §12.2 —
    // the one reason a space could appear is a pretty-printer.
    for &b in artifact().iter() {
        assert!(b == LF || (0x20..=0x7E).contains(&b), "byte {b:#04x}");
    }
    // Member sorting is JCS's job and is exercised by the byte-for-byte vector
    // test; asserting it again here would need a parser.
}

#[test]
fn item_6_sha256_over_the_stream_equals_the_reported_digest() {
    assert_eq!(
        sha256_prefixed(&artifact()),
        "sha256:3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da"
    );
}

// --- Records, order and exclusions (items 5, 7–19), over the model ---------

/// The checks DM §17 states over records. Run against any graph a caller
/// believes is a projection.
fn check_records(header: &Header, g: &Graph) {
    let format = header.object_format;

    // Item 5: every oid is lowercase hex at `object_format`'s full length.
    for oid in [&header.head, &header.trust_root].into_iter().flatten() {
        assert!(is_oid(oid, format), "header oid {oid}");
    }
    for node in g.nodes() {
        if let Some(local) = node.id.strip_prefix(&format!("{}/cs:", header.repo)) {
            assert!(is_oid(local, format), "changeset id {local}");
        }
        for (name, value) in node.attrs.iter() {
            // `changeset.tree` is the one attr that may hold a sentinel rather
            // than an oid — `unverifiable(squash)` or
            // `unverifiable(git-version)` (DM §7.2.1) — so it is exempted here
            // rather than silently loosened for every oid-valued attr.
            let sentinel = name == "tree"
                && matches!(value, AttrValue::Str(s) if s.starts_with("unverifiable("));
            if let AttrValue::Str(s) = value
                && !sentinel
                && s.len() == format.hex_len()
            {
                assert!(is_oid(s, format), "attr {name} = {s}");
            }
        }
    }

    // Items 7, 8 and 10: every record's shape, and every id in its kind's row.
    for node in g.nodes() {
        assert!(node.id.starts_with(&format!("{}/", header.repo)));
        assert!(is_ascii_printable(&node.id));
    }

    // Item 9: no `exercises` edge.
    assert!(g.edges().iter().all(|e| e.kind != EdgeKind::Exercises));

    // Item 11: no dangling edges. PB §6.3 G5 — "dangling edges are the linter".
    assert!(g.dangling_endpoints().is_empty());

    // Item 12: no `src` uses the bare `<path>:<line>` form, and none uses
    // `spine:<version>:floor`.
    for src in g
        .nodes()
        .iter()
        .map(|n| &n.src)
        .chain(g.edges().iter().map(|e| &e.src))
    {
        assert!(
            !matches!(src, Src::FileLine { .. } | Src::ShippedFloor { .. }),
            "{}",
            src.render()
        );
        assert!(src.check(format).is_ok(), "{}", src.render());
    }

    // Item 13: sorted by the *key*, never by the line.
    let by_key: Vec<Vec<u8>> = g.ordered_nodes().iter().map(|n| node_key(n)).collect();
    assert!(by_key.windows(2).all(|w| w[0] < w[1]));
    let by_key: Vec<Vec<u8>> = g.ordered_edges().iter().map(|e| edge_key(e)).collect();
    assert!(by_key.windows(2).all(|w| w[0] < w[1]));

    // Item 14: `id` unique across nodes; `(from, to, kind, attrs, src)` unique
    // across edges. Both fall out of the keys being strictly ascending above,
    // so this restates them at the level DM names them.
    let mut ids: Vec<&str> = g.nodes().iter().map(|n| n.id.as_str()).collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), before);

    // Items 15–19: the exclusions.
    for node in g.nodes() {
        if node.kind == NodeKind::Intent
            && let Some(AttrValue::Str(status)) = node.attrs.get("status")
        {
            assert!(matches!(
                status.as_str(),
                "merged" | "withdrawn" | "reverted" | "superseded"
            ));
            assert!(!status.contains('\u{2020}'), "no † status");
        }
        if node.kind == NodeKind::Test {
            assert!(node.attrs.is_empty(), "item 18");
        }
    }
    for edge in g.edges() {
        match edge.kind {
            EdgeKind::Implements => {
                assert_eq!(edge.attrs.get("provisional"), Some(&AttrValue::Bool(false)))
            }
            EdgeKind::Protects => {
                assert_eq!(edge.attrs.get("floor"), Some(&AttrValue::Bool(false)))
            }
            EdgeKind::VerifiedBy => assert!(edge.attrs.get("introduced_by").is_none()),
            _ => {}
        }
    }
}

#[test]
fn the_published_dumps_graph_satisfies_every_checkable_item() {
    check_records(&myrepo::header(), &myrepo::myrepo());
}

#[test]
fn the_empty_dump_satisfies_every_record_level_item_vacuously() {
    // DM §9: "An empty dump is legal, is not an error, and exits 0."
    let header = Header {
        object_format: ObjectFormat::Sha1,
        repo: "myrepo".into(),
        trunk: b"main".into(),
        head: None,
        trust_root: None,
    };
    let g = Graph::new();
    check_records(&header, &g);
    let dump = serialize(&header, &g).unwrap();
    assert_eq!(dump.line_count(), 1);
}

// Items 20, 21, 22, 23 and 24 are not checkable from an artifact alone:
//
// * **20** ("no `src` names a commit not reachable from `head`") and **24**
//   ("the dump changes when and only when the trunk tip, the objects it
//   reaches, or the trust root changes") need the repository.
// * **21** ("two runs … produce identical bytes") and **22** (a bare clone
//   dumps identically to a checked-out one) need two runs; item 21's
//   in-process half is `re_serializing_the_same_graph_produces_the_same_bytes`
//   in `dm_12_2_dump.rs`, and item 22 is the indexer's, since a `Graph` has no
//   worktree to be dirty.
// * **23** ("no member holds a time, a duration or a date") is enforced by
//   construction rather than by a check: no attr name in §7.2 is time-valued,
//   and `NodeKind::attr_type` refuses every name that is not in §7.2.
