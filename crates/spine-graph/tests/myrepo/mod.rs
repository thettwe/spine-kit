//! DM §12.1's repository, as a graph — shared by the vector test and the
//! conformance checklist so that both check the same object.
//!
//! `myrepo`, `object_format: sha1`, `params.trunk: main`, pinned release 1.4.0,
//! team mode, `C-A3: hostile`, `C-M1: merge`. Two trunk commits — the trust
//! root `T0` and the landing `L` for `INT-042: Invoice totals include tax` —
//! with five member commits between `B` and `Hc`.

#![allow(dead_code)]

use spine_canon::ObjectFormat;
use spine_graph::Header;
use spine_graph::schema::{Attrs, EdgeKind, NodeKind, Src, id};
use spine_graph::store::{Edge, Graph, Node};

pub const REPO: &str = "myrepo";
/// The trust root `T0` — the bootstrap `init` commit, carrying the keyring,
/// `CONSTITUTION.md` at v3 and `adr/ADR-007-tax-rounding.md`.
pub const T0: &str = "0a1b2c3d4e5f60718293a4b5c6d7e8f901234567";
/// The landing `L`, whose base is `T0`.
pub const L: &str = "1b2c3d4e5f60718293a4b5c6d7e8f90123456789";
/// `M(L)` — five member commits. `Hc` is `M5`.
pub const M1: &str = "2c3d4e5f60718293a4b5c6d7e8f9012345678901";
pub const M2: &str = "3d4e5f60718293a4b5c6d7e8f90123456789012a";
pub const M3: &str = "4e5f60718293a4b5c6d7e8f90123456789012ab3";
pub const M4: &str = "5f60718293a4b5c6d7e8f90123456789012ab3c4";
pub const M5: &str = "60718293a4b5c6d7e8f90123456789012ab3c4d5";
/// The signed intent blob, and `L`'s tree.
pub const BLOB: &str = "9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2";
pub const TREE: &str = "7b0dc1f4a2e58d3906bb4c7e21f5a8d90c3e64b7";

/// The three signed lines copied into the envelope, byte for byte as DM §12.1
/// prints them. DM §5.2.1 hashes exactly these bytes — "from the first byte of
/// the trailer name … through the last byte before its terminating LF, with no
/// LF included" — so the `approval` node ids below are computed, never quoted.
const SIGNOFF_LINE: &[u8] = b"Spine-Signoff: INT-042 blob=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 template=intent@2 constitution=v3 reopens=0 signer=alice@example.com";
const APPROVE_LINE: &[u8] = b"Spine-Approve: INT-042 intent=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 base=0a1b2c3d4e5f60718293a4b5c6d7e8f901234567 rounds=1 total_rounds=1 reopens=0 red=5/5 freeze=sha256:5c9e2a71b0463df8951ce2a4708b3d61f0492c8ad735be106f4a2c9d80e37b45 signer=alice@example.com";
const REVIEW_LINE: &[u8] = b"Spine-Review: INT-042 class=tripwire head=60718293a4b5c6d7e8f90123456789012ab3c4d5 tree=7b0dc1f4a2e58d3906bb4c7e21f5a8d90c3e64b7 base=0a1b2c3d4e5f60718293a4b5c6d7e8f901234567 intent=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 report=sha256:3c6f1a09b8d24e57af0132c9de6b48570e29a1cf83b6d045e71a29c4b0d83e16 wires=G11 reason=\"auto-merge unavailable: C-A3 hostile\" reviewer=bob@example.com";

fn commit(sha: &str) -> Src {
    Src::Commit { sha: sha.into() }
}
fn msg(line: u64) -> Src {
    Src::MessageLine {
        sha: L.into(),
        line,
    }
}
fn trailer(sha: &str, name: &str) -> Src {
    Src::Trailer {
        sha: sha.into(),
        name: name.into(),
    }
}
fn at(sha: &str, path: &[u8], line: u64) -> Src {
    Src::FileLineAt {
        sha: sha.into(),
        path: path.to_vec(),
        line,
    }
}

/// `src/billing/caf` + `0xE9` + `.py` — "a Latin-1 `é`, which git stores as one
/// byte and which no amount of normalization can make into text" (DM §12.1).
fn cafe_py() -> Vec<u8> {
    let mut p = b"src/billing/caf".to_vec();
    p.push(0xE9);
    p.extend_from_slice(b".py");
    p
}

const TEST_FILE: &[u8] = b"tests/billing/test_invoice.py";
const TEST_AC1: &[u8] = b"tests/billing/test_invoice.py::test_AC1_totals_include_tax";
const TEST_AC2: &[u8] = b"tests/billing/test_invoice.py::test_AC2_zero_rated";

pub fn myrepo() -> Graph {
    let mut g = Graph::new();

    let intent = id::intent(REPO, "INT-042");
    let ac1 = id::ac(REPO, "INT-042", 1);
    let ac2 = id::ac(REPO, "INT-042", 2);
    let constitution = id::constitution(REPO, 3);
    let signoff = id::approval(REPO, SIGNOFF_LINE);
    let approve = id::approval(REPO, APPROVE_LINE);
    let review = id::approval(REPO, REVIEW_LINE);
    let alice = id::signer(REPO, b"alice@example.com");
    let bob = id::signer(REPO, b"bob@example.com");
    let ci = id::signer(REPO, b"ci@example.com");
    let test_ac1 = id::test(REPO, "pytest", TEST_AC1);
    let test_ac2 = id::test(REPO, "pytest", TEST_AC2);

    // --- intent and its ACs, from the envelope's fenced intent bytes --------
    g.add_node(Node::new(
        NodeKind::Intent,
        intent.clone(),
        Attrs::new()
            .str("base", T0)
            .str("blob", BLOB)
            .str("landing", L)
            .int("late_reopen_count", 0)
            .bytes("owner", b"@alice")
            .int("reopen_count", 0)
            .bytes("signer", b"alice@example.com")
            .str("status", "merged")
            .str("template", "intent@2")
            // "read from the sealed intent inside the landing commit's message
            // — **never** from that commit's subject line" (DM §7.2).
            .bytes("title", b"Invoice totals include tax"),
        msg(14),
    ));
    g.add_node(Node::new(NodeKind::Ac, ac1.clone(), Attrs::new(), msg(24)));
    g.add_node(Node::new(NodeKind::Ac, ac2.clone(), Attrs::new(), msg(25)));
    g.add_edge(Edge::new(
        EdgeKind::HasAc,
        intent.clone(),
        ac1.clone(),
        Attrs::new(),
        msg(24),
    ));
    g.add_edge(Edge::new(
        EdgeKind::HasAc,
        intent.clone(),
        ac2.clone(),
        Attrs::new(),
        msg(25),
    ));
    g.add_edge(Edge::new(
        EdgeKind::BuiltUnder,
        intent.clone(),
        constitution.clone(),
        Attrs::new(),
        msg(15),
    ));

    // --- touchpoints -------------------------------------------------------
    // Two `declares` edges share `src` because one line of the touchpoints
    // block names both paths; they are ordered by `to`, PB's second key
    // (DM §12.3 check 5).
    for (path, polarity, line) in [
        (b"api/invoices.ts".as_slice(), "expected", 29u64),
        (b"src/billing/".as_slice(), "expected", 29),
        (b"auth/".as_slice(), "forbidden", 30),
        (b"shared/schema/".as_slice(), "forbidden", 30),
    ] {
        let unit = id::code_unit(REPO, path);
        g.add_node(Node::new(
            NodeKind::CodeUnit,
            unit.clone(),
            Attrs::new(),
            msg(line),
        ));
        g.add_edge(Edge::new(
            EdgeKind::Declares,
            intent.clone(),
            unit,
            Attrs::new().str("polarity", polarity),
            msg(line),
        ));
    }

    // --- the constitution, its C-A2 floor extension, and the ADR -----------
    g.add_node(Node::new(
        NodeKind::Constitution,
        constitution.clone(),
        Attrs::new(),
        at(L, b"CONSTITUTION.md", 2),
    ));
    let infra = id::code_unit(REPO, b"infra/");
    g.add_node(Node::new(
        NodeKind::CodeUnit,
        infra.clone(),
        Attrs::new(),
        at(L, b"CONSTITUTION.md", 96),
    ));
    // `floor: false` — the shipped half is excluded (DM §8.5 clause 2), so only
    // the `C-A2` limb survives and every dumped `protects` is `false`.
    g.add_edge(Edge::new(
        EdgeKind::Protects,
        constitution,
        infra,
        Attrs::new().bool("floor", false),
        at(L, b"CONSTITUTION.md", 96),
    ));
    g.add_node(Node::new(
        NodeKind::Adr,
        id::adr(REPO, "ADR-007"),
        Attrs::new(),
        at(L, b"adr/ADR-007-tax-rounding.md", 1),
    ));

    // --- signers, from `.spine/allowed_signers` at the trust root ----------
    // `valid_to` is absent on all three, "not null" (DM §12.3 check 6).
    for (node_id, fingerprint, roles, line) in [
        (
            &alice,
            "SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM",
            vec!["spine-review@v1", "spine-signoff@v1"],
            1u64,
        ),
        (
            &bob,
            "SHA256:V2dasTIGWUnhlaxa7vr3Qmqpe/qYBoxi+C7Cs6yxxEs",
            vec!["spine-review@v1", "spine-signoff@v1"],
            2,
        ),
        (
            &ci,
            "SHA256:eQ0ZoC+rlhZstRuvhLXwJlwqLAreBcmnyFedpvPmTVY",
            vec!["spine-seal@v1"],
            3,
        ),
    ] {
        g.add_node(Node::new(
            NodeKind::Signer,
            node_id.clone(),
            Attrs::new()
                .str("fingerprint", fingerprint)
                .arr("roles", roles)
                .str("valid_from", T0),
            at(T0, b".spine/allowed_signers", line),
        ));
    }

    // --- the three copied approvals ----------------------------------------
    g.add_node(Node::new(
        NodeKind::Approval,
        signoff.clone(),
        Attrs::new()
            .str("blob", BLOB)
            .str("event", "signoff")
            .bytes("principal", b"alice@example.com")
            .int("reopens", 0)
            .str("role", "signer")
            .bool("verified", true),
        trailer(L, "Spine-Signoff"),
    ));
    g.add_node(Node::new(
        NodeKind::Approval,
        approve.clone(),
        Attrs::new()
            .str("base", T0)
            .str("blob", BLOB)
            .str("event", "approve")
            .str(
                "freeze",
                "sha256:5c9e2a71b0463df8951ce2a4708b3d61f0492c8ad735be106f4a2c9d80e37b45",
            )
            .bytes("principal", b"alice@example.com")
            .str("red", "5/5")
            .int("reopens", 0)
            // "the namespace the signature verified under, never a claim in the
            // trailer": a v1 approve line signed under `spine-review@v1`.
            .str("role", "reviewer")
            .int("rounds", 1)
            .int("total_rounds", 1)
            .bool("verified", true),
        trailer(L, "Spine-Approve"),
    ));
    g.add_node(Node::new(
        NodeKind::Approval,
        review.clone(),
        Attrs::new()
            .str("base", T0)
            .str("blob", BLOB)
            .str("class", "tripwire")
            .str("event", "review")
            .str("head", M5)
            .bytes("principal", b"bob@example.com")
            .str("role", "reviewer")
            .str("tree", TREE)
            .bool("verified", true)
            // In the line's order, never re-sorted (DM §7.2).
            .arr("wires", ["G11"]),
        trailer(L, "Spine-Review"),
    ));
    for (approval, name, signer) in [
        (&signoff, "Spine-Signoff", &alice),
        (&approve, "Spine-Approve", &alice),
        (&review, "Spine-Review", &bob),
    ] {
        g.add_edge(Edge::new(
            EdgeKind::Approves,
            approval.clone(),
            intent.clone(),
            Attrs::new(),
            trailer(L, name),
        ));
        g.add_edge(Edge::new(
            EdgeKind::SignedBy,
            approval.clone(),
            signer.clone(),
            Attrs::new(),
            trailer(L, name),
        ));
    }

    // --- what the binding approval froze -----------------------------------
    // Two `src` shapes in one kind: a `freezes` to a `code_unit` cites the
    // member commit's `Spine-Frozen`, to a `test` its `Spine-Test`.
    for (path, oid) in [
        (
            b"pytest.ini".as_slice(),
            "1e9f4c7a20d63b8859e04f1a7cd6b325908e4f71",
        ),
        (TEST_FILE, "a41bd9c2e70f83615a4d2b8c09e7f1436d5028ba"),
    ] {
        let unit = id::code_unit(REPO, path);
        g.add_node(Node::new(
            NodeKind::CodeUnit,
            unit.clone(),
            Attrs::new(),
            trailer(M3, "Spine-Frozen"),
        ));
        g.add_edge(Edge::new(
            EdgeKind::Freezes,
            approve.clone(),
            unit,
            Attrs::new().str("oid", oid),
            trailer(M3, "Spine-Frozen"),
        ));
    }
    for test in [&test_ac1, &test_ac2] {
        // A `freezes` edge to a `test` carries `{}` — PB §6.2 says so.
        g.add_edge(Edge::new(
            EdgeKind::Freezes,
            approve.clone(),
            test.clone(),
            Attrs::new(),
            trailer(M3, "Spine-Test"),
        ));
    }

    // --- tests, parsed from `<L>:<path>` — the frozen blob, reachable
    // through `L`'s tree forever -------------------------------------------
    for (test, node_line, edge_line, ac) in
        [(&test_ac1, 7u64, 6u64, &ac1), (&test_ac2, 19, 18, &ac2)]
    {
        g.add_node(Node::new(
            NodeKind::Test,
            test.clone(),
            // `{}`: `result_at` is the kind's only attr and §8.4 excludes it.
            Attrs::new(),
            at(L, TEST_FILE, node_line),
        ));
        // test -> ac, because G5 fails on "a `verified_by` edge to a
        // nonexistent AC".
        g.add_edge(Edge::new(
            EdgeKind::VerifiedBy,
            test.clone(),
            ac.clone(),
            Attrs::new().bool("attributed", true),
            at(L, TEST_FILE, edge_line),
        ));
    }

    // --- the landing changeset and its members -----------------------------
    let cs_l = id::changeset(REPO, L);
    g.add_node(Node::new(
        NodeKind::Changeset,
        cs_l.clone(),
        Attrs::new()
            .str("base", T0)
            .str("event", "land")
            // The **seal's** `git=`, never the indexing binary's own git
            // (DM §7.2.1) — reading the local git would put the environment in
            // the artifact.
            .str("git_version", "2.45")
            .str("head", M5)
            .bool("landing", true)
            .str("lane", "gated")
            .str("mode", "team")
            .str("profile", "container")
            .str(
                "report_sha256",
                "sha256:3c6f1a09b8d24e57af0132c9de6b48570e29a1cf83b6d045e71a29c4b0d83e16",
            )
            .bool("resealed", false)
            .bytes("seal_principal", b"ci@example.com")
            .bool("seal_verified", true)
            .str("strategy", "merge")
            .str("threat", "hostile")
            .str("tool_version", "1.4.0")
            .str("tree", TREE)
            .bool("unattested", false),
        trailer(L, "Spine-Seal"),
    ));
    g.add_edge(Edge::new(
        EdgeKind::Implements,
        cs_l.clone(),
        intent.clone(),
        Attrs::new()
            .str("role", "landing")
            .bool("provisional", false)
            .bool("verified", true),
        trailer(L, "Spine-Seal"),
    ));
    g.add_edge(Edge::new(
        EdgeKind::AttestedBy,
        cs_l.clone(),
        ci,
        Attrs::new(),
        trailer(L, "Spine-Seal"),
    ));
    for member in [M1, M2, M3, M4, M5] {
        let cs = id::changeset(REPO, member);
        // "A member changeset carries `{"landing":false}` and nothing else: it
        // has no seal, and every one of those fields is a seal field."
        g.add_node(Node::new(
            NodeKind::Changeset,
            cs.clone(),
            Attrs::new().bool("landing", false),
            commit(member),
        ));
        g.add_edge(Edge::new(
            EdgeKind::Implements,
            cs,
            intent.clone(),
            Attrs::new()
                .str("role", "member")
                .bool("provisional", false)
                .bool("verified", true),
            trailer(L, "Spine-Seal"),
        ));
    }

    // --- `modifies`: the landing's integrated delta, plus the per-member
    // diffs PB §6.2 keeps for archaeology ----------------------------------
    // Each touched path is added as a node once per citing edge; DM §5.5 keeps
    // the minimum, which for all three is `git:<L>` (DM §12.3 check 3).
    for (changeset_sha, paths) in [
        (
            L,
            vec![
                cafe_py(),
                b"src/billing/tax.py".to_vec(),
                TEST_FILE.to_vec(),
            ],
        ),
        (M2, vec![TEST_FILE.to_vec()]),
        (M4, vec![cafe_py(), b"src/billing/tax.py".to_vec()]),
    ] {
        for path in paths {
            let unit = id::code_unit(REPO, &path);
            g.add_node(Node::new(
                NodeKind::CodeUnit,
                unit.clone(),
                Attrs::new(),
                commit(changeset_sha),
            ));
            g.add_edge(Edge::new(
                EdgeKind::Modifies,
                id::changeset(REPO, changeset_sha),
                unit,
                Attrs::new(),
                commit(changeset_sha),
            ));
        }
    }

    g
}

pub fn header() -> Header {
    Header {
        object_format: ObjectFormat::Sha1,
        repo: REPO.to_string(),
        trunk: b"main".to_vec(),
        head: Some(L.to_string()),
        trust_root: Some(T0.to_string()),
    }
}
