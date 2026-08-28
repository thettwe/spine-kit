//! The landing shapes PB §5.5 and PB §6.6 define beyond the gated merge one.
//!
//! Each test builds a small trunk with `git commit-tree` and asks one question:
//! a tombstone's status, a supersession's two edges, a revert detected by patch
//! id, the squash tree sentinel, an orphan's absence, and a quick landing's
//! `approves` target. Signatures are out of scope here — [`Unverified`] answers
//! "no" to every namespace, so every landing indexes `unattested` by design and
//! the tests assert what does *not* depend on a signature.

use spine_graph::derive::{Indexer, Options};
use spine_graph::schema::{AttrValue, EdgeKind, NodeKind, id};
use spine_graph::store::Graph;
use spine_graph::verify::Unverified;
use spine_graph::git;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const MANIFEST: &str = r#"{"cli":{"dist_hash":"sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db","version":"1.4.0"},"envelope":1,"files":[{"blob":"6d4db08390092d7d5d96476eddca6355815bc49f","owner":"user-owned","path":".spine/allowed_signers","template":"keyring@1"}],"manifest_version":1,"object_format":"sha1","params":{"ci":"github","isolation":"container","langs":["python"],"timeout":1800,"trunk":"main"},"paths":{"agent_context":["AGENTS.md","CLAUDE.md"],"constitution":"CONSTITUTION.md"},"repo":"myrepo","resign":{"intent":2,"intent-bug":2,"intent-change":2},"schema":7,"templates":{"agents-block":2,"ci-generic":4,"ci-github-collect":4,"ci-github-land":4,"ci-gitlab":4,"constitution":1,"gitattributes":1,"gitignore":1,"intent":2,"intent-bug":2,"intent-change":2,"keyring":1}}
"#;

/// MF §8.7's published keyring, byte for byte — three public keys, "no private
/// key is published and none is needed to verify". These tests never verify a
/// signature; the entries are here so `signer` nodes exist at all.
const KEYRING: &str = "alice@example.com namespaces=\"spine-signoff@v1,spine-review@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla\nbob@example.com namespaces=\"spine-signoff@v1,spine-review@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINZJBgqcpDmx19xO9D29xeFtCCUMyfe/ti+lY7c+rvim\nci@example.com namespaces=\"spine-seal@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICE3UkO6VDz+9ag4xQScwsfdP8PHJhLh+QWkIwzUjWze\n";

const CONSTITUTION: &str = "# Constitution — myrepo\nVersion: v3 · Owner: @alice\n\nC-A2: protected = infra/\n";

/// A minimal intent document (ID §4) for the id given.
fn intent_doc(id: &str, title: &str) -> String {
    format!(
        "# {id}: {title}\nOwner: @alice · Template: intent@2 · Constitution: v3\n\n## Goal\nA goal.\n\n## Non-goals\n- One.\n- Two.\n\n## Acceptance criteria\nAC-1: A criterion.\n\n## Touchpoints\nExpected to change: src/\nMust NOT change:\n"
    )
}

struct Trunk {
    root: PathBuf,
    tip: String,
    t0: String,
}

impl Drop for Trunk {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn run(dir: &Path, args: &[&str], stdin: Option<&[u8]>) -> Vec<u8> {
    use std::io::Write;
    let mut child = Command::new("git")
        .current_dir(dir)
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(bytes) = stdin {
        child.stdin.as_mut().unwrap().write_all(bytes).unwrap();
        drop(child.stdin.take());
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

fn text(dir: &Path, args: &[&str]) -> String {
    String::from_utf8_lossy(&run(dir, args, None))
        .trim_end()
        .to_string()
}

impl Trunk {
    fn new(name: &str) -> Option<Self> {
        if !git::available() {
            return None;
        }
        let root = std::env::temp_dir().join(format!("spine-graph-shapes-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).ok()?;
        run(&root, &["init", "--quiet", "-b", "main", "."], None);
        for (k, v) in [("user.email", "t@example.invalid"), ("user.name", "Test")] {
            run(&root, &["config", k, v], None);
        }
        std::fs::create_dir_all(root.join(".spine")).ok()?;
        std::fs::write(root.join(".spine/manifest.json"), MANIFEST).ok()?;
        std::fs::write(root.join(".spine/allowed_signers"), KEYRING).ok()?;
        std::fs::write(root.join("CONSTITUTION.md"), CONSTITUTION).ok()?;
        run(&root, &["add", "-A"], None);
        run(&root, &["commit", "--quiet", "-m", "spine: init"], None);
        let t0 = text(&root, &["rev-parse", "HEAD"]);
        run(&root, &["config", "spine.trustRoot", &t0], None);
        Some(Trunk {
            root,
            tip: t0.clone(),
            t0,
        })
    }

    /// Stage a file (or delete one, with `None`) and return the resulting tree.
    fn stage(&self, path: &str, content: Option<&str>) -> String {
        match content {
            Some(content) => {
                let full = self.root.join(path);
                std::fs::create_dir_all(full.parent().unwrap()).unwrap();
                std::fs::write(full, content).unwrap();
                run(&self.root, &["add", path], None);
            }
            None => {
                run(&self.root, &["rm", "--quiet", path], None);
            }
        }
        text(&self.root, &["write-tree"])
    }

    /// Land a commit on trunk with the given message and tree.
    ///
    /// PB §5.5: *"`L` is created with `git commit-tree`"* — never `git commit`,
    /// so no cleanup rule can touch the envelope's bytes.
    fn land(&mut self, message: &str, tree: &str) -> String {
        let file = self.root.join("message.txt");
        std::fs::write(&file, message).unwrap();
        let sha = text(
            &self.root,
            &[
                "commit-tree",
                tree,
                "-p",
                &self.tip,
                "-F",
                file.to_str().unwrap(),
            ],
        );
        run(&self.root, &["update-ref", "refs/heads/main", &sha], None);
        self.tip = sha.clone();
        sha
    }

    fn graph(&self) -> Graph {
        let repo = git::Repo::open(&self.root).unwrap();
        Indexer::new(&repo, &Unverified)
            .index(&Options::default())
            .unwrap()
            .graph
    }
}

/// A gated landing's envelope, in EV §2.4's rank order.
fn gated_envelope(id: &str, title: &str, base: &str, tree: &str, extra: &[&str]) -> String {
    let doc = intent_doc(id, title);
    let mut message = format!("{id}: {title}\n\n");
    message.push_str(&format!(
        "-----BEGIN SPINE-INTENT blob=0000000000000000000000000000000000000000 bytes={}-----\n",
        doc.len()
    ));
    message.push_str(&doc);
    message.push_str("-----END SPINE-INTENT-----\n\n");
    message.push_str("Spine-Envelope: 1\nSpine-Event: land\nSpine-Lane: gated\n");
    message.push_str(&format!("Spine-Intent: {id}\n"));
    for line in extra {
        message.push_str(line);
        message.push('\n');
    }
    message.push_str("Spine-Gates: G1=pass\nSpine-Strategy: merge\n");
    message.push_str(&format!(
        "Spine-Seal: {id} base={base} head={base} tree={tree} report=sha256:00 tool=1.4.0+sha256:00 git=2.45 mode=solo threat=trusted profile=none envelope=sha256:00 signer=ci@example.com\n"
    ));
    message.push_str("Spine-Seal-Sig: AAAA\n");
    message
}

fn node<'g>(graph: &'g Graph, id: &str) -> &'g spine_graph::store::Node {
    graph
        .nodes()
        .iter()
        .find(|n| n.id == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

fn attr(node: &spine_graph::store::Node, name: &str) -> String {
    match node.attrs.get(name) {
        Some(AttrValue::Str(s)) => s.clone(),
        Some(AttrValue::Bool(b)) => b.to_string(),
        Some(AttrValue::Int(n)) => n.to_string(),
        other => panic!("{} has attr {name} = {other:?}", node.id),
    }
}

#[test]
fn a_tombstone_retires_its_id_with_status_withdrawn_and_its_parents_tree() {
    let Some(mut trunk) = Trunk::new("withdraw") else {
        return;
    };
    // PB §5.5: a tombstone has "parent `B`, tree identical to `B`'s,
    // `Spine-Event: withdraw`, the fenced intent … the signed `Spine-Withdraw`
    // line, the seal — so abandonment is countable from trunk alone".
    let tree = text(&trunk.root, &["rev-parse", "HEAD^{tree}"]);
    let base = trunk.tip.clone();
    let envelope = gated_envelope("INT-042", "Abandoned", &base, &tree, &[])
        .replace("Spine-Event: land", "Spine-Event: withdraw")
        .replace(
            "Spine-Gates: G1=pass",
            "Spine-Withdraw: INT-042 blob=0000000000000000000000000000000000000000 reason=\"superseded by a different approach\" signer=alice@example.com\nSpine-Withdraw-Sig: AAAA\nSpine-Gates: G1=pass",
        );
    trunk.land(&envelope, &tree);

    let graph = trunk.graph();
    assert_eq!(
        attr(node(&graph, &id::intent("myrepo", "INT-042")), "status"),
        "withdrawn"
    );
    // The withdraw line is an approval node like any other statement line.
    let withdraw = graph
        .nodes()
        .iter()
        .find(|n| n.kind == NodeKind::Approval && attr(n, "event") == "withdraw")
        .expect("a withdraw approval node");
    assert_eq!(attr(withdraw, "principal"), "alice@example.com");
}

#[test]
fn a_supersession_emits_both_directions_and_flips_the_earlier_intents_status() {
    let Some(mut trunk) = Trunk::new("supersede") else {
        return;
    };
    let tree = trunk.stage("src/a.py", Some("a\n"));
    let base = trunk.tip.clone();
    trunk.land(&gated_envelope("INT-042", "First", &base, &tree, &[]), &tree);

    let tree2 = trunk.stage("src/b.py", Some("b\n"));
    let base2 = trunk.tip.clone();
    trunk.land(
        &gated_envelope(
            "INT-043",
            "Second",
            &base2,
            &tree2,
            &["Spine-Supersedes: INT-042"],
        ),
        &tree2,
    );

    let graph = trunk.graph();
    let first = id::intent("myrepo", "INT-042");
    let second = id::intent("myrepo", "INT-043");
    // PB §6.6: "the indexer emits `superseded_by`, so archaeology queries
    // return the current truth first and the history behind it."
    assert!(
        graph.edges().iter().any(|e| e.kind == EdgeKind::Supersedes
            && e.from == second
            && e.to == first)
    );
    assert!(
        graph.edges().iter().any(|e| e.kind == EdgeKind::SupersededBy
            && e.from == first
            && e.to == second)
    );
    assert_eq!(attr(node(&graph, &first), "status"), "superseded");
    assert_eq!(attr(node(&graph, &second), "status"), "merged");
}

#[test]
fn a_revert_is_detected_by_patch_id_and_is_never_declared() {
    let Some(mut trunk) = Trunk::new("revert") else {
        return;
    };
    let tree = trunk.stage("src/a.py", Some("added by INT-042\n"));
    let base = trunk.tip.clone();
    let l = trunk.land(&gated_envelope("INT-042", "Adds a", &base, &tree, &[]), &tree);

    // The reverting landing removes exactly what `L` added. Nothing in its
    // envelope says so: PB §6.6's "A revert is detected, never declared."
    let reverted_tree = trunk.stage("src/a.py", None);
    let base2 = trunk.tip.clone();
    let r = trunk.land(
        &gated_envelope("BUG-051", "Reverts a", &base2, &reverted_tree, &[]),
        &reverted_tree,
    );

    let graph = trunk.graph();
    let edge = graph
        .edges()
        .iter()
        .find(|e| e.kind == EdgeKind::Reverts)
        .expect("a reverts edge");
    assert_eq!(edge.from, id::changeset("myrepo", &r));
    assert_eq!(edge.to, id::changeset("myrepo", &l));
    assert_eq!(edge.src.render(), format!("git:{r}:patch-id"));
    assert_eq!(
        edge.attrs.get("partial"),
        Some(&AttrValue::Bool(false)),
        "a whole reversal, not the partial case"
    );
    assert_eq!(
        attr(node(&graph, &id::intent("myrepo", "INT-042")), "status"),
        "reverted"
    );
}

#[test]
fn a_squash_landing_records_the_tree_sentinel_and_reads_its_freeze_from_the_envelope() {
    let Some(mut trunk) = Trunk::new("squash") else {
        return;
    };
    let tree = trunk.stage("src/a.py", Some("a\n"));
    let base = trunk.tip.clone();
    let envelope = gated_envelope(
        "INT-042",
        "Squashed",
        &base,
        &tree,
        &[
            "Spine-Approve: INT-042 intent=0000000000000000000000000000000000000000 base=0000000000000000000000000000000000000000 rounds=1 total_rounds=1 reopens=0 red=1/1 freeze=sha256:00 signer=alice@example.com",
            "Spine-Approve-Sig: AAAA",
            "Spine-Frozen: 1e9f4c7a20d63b8859e04f1a7cd6b325908e4f71 pytest.ini",
            "Spine-Test: pytest tests/test_a.py::test_AC1_a",
        ],
    )
    .replace("Spine-Strategy: merge", "Spine-Strategy: squash");
    let l = trunk.land(&envelope, &tree);

    let graph = trunk.graph();
    // DM §7.2.1: under squash "`H` is unreachable by design and the tree rule
    // is never consulted, *so a source-side index and the G10 clone derive the
    // same thing*" — PB §6.3's G9 records the sentinel instead of the oid.
    assert_eq!(
        attr(node(&graph, &id::changeset("myrepo", &l)), "tree"),
        "unverifiable(squash)"
    );
    assert_eq!(
        attr(node(&graph, &id::changeset("myrepo", &l)), "strategy"),
        "squash"
    );

    // PB §11 confines `Spine-Frozen`/`Spine-Test` to the squash envelope, so
    // both are cited at the landing itself rather than at an approval commit
    // that is by design unreachable.
    let frozen = graph
        .edges()
        .iter()
        .find(|e| e.kind == EdgeKind::Freezes && e.to.ends_with("code:pytest.ini"))
        .expect("a freezes edge to the frozen runner config");
    assert_eq!(frozen.src.render(), format!("git:{l}:trailer:Spine-Frozen"));
    assert_eq!(
        frozen.attrs.get("oid"),
        Some(&AttrValue::Str(
            "1e9f4c7a20d63b8859e04f1a7cd6b325908e4f71".into()
        ))
    );
    let test = node(
        &graph,
        &id::test("myrepo", "pytest", b"tests/test_a.py::test_AC1_a"),
    );
    assert!(test.attrs.is_empty());
}

#[test]
fn an_orphan_on_trunk_is_no_changeset_at_all() {
    let Some(mut trunk) = Trunk::new("orphan") else {
        return;
    };
    let tree = trunk.stage("src/a.py", Some("pushed around the pipeline\n"));
    // PB §5.5: "A first-parent trunk commit that is neither a landing nor the
    // trust root … is an **orphan**: a push around the pipeline." DM §8.2 says
    // the same from the dump's side — a changeset is included when a commit
    // carries `Spine-Seal`, and this one does not.
    let orphan = trunk.land("just committed on trunk\n", &tree);

    let graph = trunk.graph();
    assert!(
        !graph
            .nodes()
            .iter()
            .any(|n| n.id == id::changeset("myrepo", &orphan)),
        "an orphan has no seal, so it has no changeset node"
    );
    assert!(
        !graph.nodes().iter().any(|n| n.kind == NodeKind::Changeset),
        "and nothing else in this history is sealed either"
    );
    // The trunk-derived kinds are still there: PB §6.2 derives `signer` and
    // `constitution` nodes from trees, not from landings — except that the
    // constitution's citation is a *landing's*, so a trunk with none has no
    // constitution node either (CN §9.6).
    assert!(!graph.nodes().is_empty(), "the keyring still yields signers");
}

#[test]
fn a_quick_landing_has_no_intent_so_its_review_approves_the_landing_changeset() {
    let Some(mut trunk) = Trunk::new("quick") else {
        return;
    };
    let tree = trunk.stage("docs/readme.md", Some("docs\n"));
    let base = trunk.tip.clone();
    // PB §5.5: "A quick-lane change lands with a minimal envelope (subject
    // `quick: <summary>`, `Spine-Envelope`, `Spine-Event: land`, `Spine-Lane:
    // quick`, gates, strategy, seal, **and the copied `Spine-Review` + `-Sig`
    // that every quick landing carries** … No fenced block, no sign-off, no
    // approval)."
    let review = format!(
        "Spine-Review: quick class=protected head={base} tree={tree} base={base} report=sha256:00 wires=G14:docs/readme.md reason=\"floor touched\" reviewer=bob@example.com"
    );
    let message = format!(
        "quick: update docs\n\nSpine-Envelope: 1\nSpine-Event: land\nSpine-Lane: quick\n{review}\nSpine-Review-Sig: AAAA\nSpine-Gates: G1=pass\nSpine-Strategy: merge\nSpine-Seal: quick base={base} head={base} tree={tree} report=sha256:00 tool=1.4.0+sha256:00 git=2.45 mode=solo threat=trusted profile=none envelope=sha256:00 signer=ci@example.com\nSpine-Seal-Sig: AAAA\n"
    );
    let l = trunk.land(&message, &tree);

    let graph = trunk.graph();
    let cs = id::changeset("myrepo", &l);
    assert_eq!(attr(node(&graph, &cs), "lane"), "quick");
    assert!(
        !graph.nodes().iter().any(|n| n.kind == NodeKind::Intent),
        "a quick landing has no intent document and no intent node"
    );
    // PB §6.2: `approves` "names the intent for every line carrying an id and
    // the landing changeset `cs:<L>` for those that do not".
    let approves = graph
        .edges()
        .iter()
        .find(|e| e.kind == EdgeKind::Approves)
        .expect("the copied review approves something");
    assert_eq!(approves.to, cs);
    // The wire token survives verbatim, in the line's order (DM §7.2).
    let review_node = graph
        .nodes()
        .iter()
        .find(|n| n.kind == NodeKind::Approval)
        .unwrap();
    assert_eq!(
        review_node.attrs.get("wires"),
        Some(&AttrValue::StrArr(vec!["G14:docs/readme.md".into()]))
    );
    assert_eq!(attr(review_node, "class"), "protected");
}

#[test]
fn a_landing_below_the_trust_root_is_not_walked() {
    let Some(mut trunk) = Trunk::new("trust-root") else {
        return;
    };
    // A landing *before* the pin: DM §8.2 excludes a changeset "below the trust
    // root", which is what makes PB §7.5's pin the bottom of the ledger.
    let tree = trunk.stage("src/old.py", Some("old\n"));
    let base = trunk.tip.clone();
    let below = trunk.land(&gated_envelope("INT-001", "Old", &base, &tree, &[]), &tree);

    let tree2 = trunk.stage("src/new.py", Some("new\n"));
    let base2 = trunk.tip.clone();
    let above = trunk.land(&gated_envelope("INT-002", "New", &base2, &tree2, &[]), &tree2);

    // Move the pin to the second landing: the walk keeps the trust root and
    // everything above it, so the first landing falls out.
    run(&trunk.root, &["config", "spine.trustRoot", &above], None);
    let graph = trunk.graph();
    assert!(
        !graph
            .nodes()
            .iter()
            .any(|n| n.id == id::changeset("myrepo", &below)),
        "the landing below the trust root is not in the graph"
    );
    assert!(
        graph
            .nodes()
            .iter()
            .any(|n| n.id == id::changeset("myrepo", &above))
    );
    assert!(trunk.t0.len() == 40 || trunk.t0.len() == 64);
}
