//! DM §12.2/§12.3 — the full published dump: **62 lines, 14054 bytes,
//! `sha256:3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da`**.
//!
//! The repository is DM §12.1's `myrepo`: `object_format: sha1`,
//! `params.trunk: main`, pinned release 1.4.0, team mode, `C-A3: hostile`,
//! `C-M1: merge`. Trunk's first-parent history is two commits — the trust root
//! `T0` and the landing `L` for `INT-042: Invoice totals include tax` — with
//! five member commits between `B` and `Hc`.
//!
//! Nothing here is asserted that was not computed. The three `approval` node
//! ids are SHA-256 over the signed trailer lines `myrepo/mod.rs` holds verbatim
//! (DM §5.2.1); the dump's digest is taken over the fixture's bytes; and the
//! serializer's output is compared against those same bytes, line by line, so a
//! divergence names the record rather than the file.
//!
//! The example exercises every node kind, eleven of the fifteen edge kinds, a
//! non-UTF-8 path through `esc`, four of PB §6.1's provenance productions, an
//! absent optional attr (`signer.valid_to`), an array attr, and DM §5.5's
//! minimum-`src` rule for `code_unit`.

mod myrepo;

use myrepo::{L, header, myrepo};
use spine_canon::sha256_prefixed;
use spine_graph::schema::NodeKind;
use spine_graph::serialize;

const PUBLISHED: &[u8] = include_bytes!("vectors/dm-12-2-dump.jsonl");

#[test]
fn the_published_dump_is_62_lines_of_14054_bytes_hashing_to_3321e7bd() {
    // The vector's own three numbers, recomputed from the fixture's bytes.
    assert_eq!(PUBLISHED.len(), 14054);
    assert_eq!(PUBLISHED.iter().filter(|&&b| b == 0x0A).count(), 62);
    assert_eq!(
        sha256_prefixed(PUBLISHED),
        "sha256:3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da"
    );
    // "including the final LF, excluding nothing" (DM §2.5).
    assert_eq!(*PUBLISHED.last().unwrap(), 0x0A);
}

#[test]
fn the_serializer_reproduces_dm_12_2_byte_for_byte() {
    let dump = serialize(&header(), &myrepo()).expect("the vector's graph is conforming");

    // Compare line by line first: a whole-artifact `assert_eq!` on 14 KB names
    // nothing, and DM §2.2 chose JSON Lines precisely so a diff names the
    // record — "a character offset into a 40 MB JSON document does not".
    let produced: Vec<&[u8]> = dump.bytes().split_inclusive(|&b| b == 0x0A).collect();
    let expected: Vec<&[u8]> = PUBLISHED.split_inclusive(|&b| b == 0x0A).collect();
    for (i, (got, want)) in produced.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            String::from_utf8_lossy(got),
            String::from_utf8_lossy(want),
            "line {}",
            i + 1
        );
    }
    assert_eq!(produced.len(), expected.len(), "line count");

    assert_eq!(dump.bytes(), PUBLISHED);
    assert_eq!(dump.len(), 14054);
    assert_eq!(dump.line_count(), 62);
    assert_eq!(
        dump.digest(),
        "sha256:3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da"
    );
}

#[test]
fn the_node_order_is_kind_then_id_so_the_intent_sits_below_its_own_acs() {
    // DM §12.3 check 1: "`myrepo/INT-042` (kind `intent`) is the twenty-third
    // node, far below `myrepo/INT-042/AC-1` (kind `ac`), which is the first.
    // PB §6.3's order, not an id order."
    let g = myrepo();
    let ids: Vec<&str> = g.ordered_nodes().iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids[0], "myrepo/INT-042/AC-1");
    assert_eq!(ids[22], "myrepo/INT-042");
    assert!(ids.iter().position(|i| *i == "myrepo/INT-042").unwrap() > 0);
}

#[test]
fn the_non_utf8_path_precedes_tax_py_which_raw_byte_order_would_reverse() {
    // DM §12.3 check 2.
    let g = myrepo();
    let ids: Vec<&str> = g.ordered_nodes().iter().map(|n| n.id.as_str()).collect();
    let cafe = ids
        .iter()
        .position(|i| *i == "myrepo/code:src/billing/caf\\xe9.py")
        .expect("the é path is a node");
    let tax = ids
        .iter()
        .position(|i| *i == "myrepo/code:src/billing/tax.py")
        .expect("tax.py is a node");
    assert!(cafe < tax);
}

#[test]
fn the_test_file_code_unit_takes_the_least_of_its_three_citations() {
    // DM §12.3 check 3: cited by `modifies` from `L` and from `M2` and by
    // `freezes` from `M3`; it takes `git:1b2c…6789`.
    let g = myrepo();
    let node = g
        .nodes()
        .iter()
        .find(|n| n.id == "myrepo/code:tests/billing/test_invoice.py")
        .expect("the test file is a code_unit");
    assert_eq!(node.src.render(), format!("git:{L}"));
}

#[test]
fn eleven_edge_kinds_appear_and_four_do_not() {
    // DM §12.1: `reverts`, `supersedes` and `superseded_by` "have no occasion
    // in a two-commit history"; `exercises` is excluded from every dump.
    let g = myrepo();
    let mut kinds: Vec<&str> = g.edges().iter().map(|e| e.kind.token()).collect();
    kinds.sort_unstable();
    kinds.dedup();
    assert_eq!(
        kinds,
        [
            "approves",
            "attested_by",
            "built_under",
            "declares",
            "freezes",
            "has_ac",
            "implements",
            "modifies",
            "protects",
            "signed_by",
            "verified_by"
        ]
    );
}

#[test]
fn every_node_kind_appears_exactly_as_the_vector_describes() {
    let g = myrepo();
    let mut kinds: Vec<&str> = g.nodes().iter().map(|n| n.kind.token()).collect();
    kinds.sort_unstable();
    kinds.dedup();
    assert_eq!(kinds, NodeKind::ALL.map(|k| k.token()));
}

#[test]
fn re_serializing_the_same_graph_produces_the_same_bytes() {
    // DM §17 item 21, and DM §11's "No false positive": "Two indexings of the
    // same objects by the same release produce identical bytes." Two `Graph`s
    // built by the same code from the same facts are the closest this crate can
    // come to two indexings, and a HashMap iteration order leaking into the
    // output would show here.
    let a = serialize(&header(), &myrepo()).unwrap();
    let b = serialize(&header(), &myrepo()).unwrap();
    assert_eq!(a.bytes(), b.bytes());
    assert_eq!(
        spine_graph::g10_compare(&a, &b).unwrap(),
        spine_graph::G10::Pass
    );
}
