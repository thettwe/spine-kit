//! The derivation, against a repository git actually built.
//!
//! DM §12's vectors test a serializer; this file tests the *walk*. It builds
//! DM §12.1's `myrepo` for real — a trust root carrying the keyring, the
//! constitution at v3 and an ADR; five member commits; a merge landing whose
//! message is PB §5.5's envelope — with **real ed25519 keys and real
//! `ssh-keygen -Y sign` signatures**, so `approval.verified`,
//! `approval.role`, `signer.fingerprint` and `changeset.seal_verified` are
//! computed rather than asserted.
//!
//! Nothing here quotes a digest, an oid or a fingerprint: every one of them is
//! read back out of the repository or out of `ssh-keygen -lf`. The one thing
//! the fabricated vectors could not test is the one thing this file exists for
//! — that PB §6.2's derivation table, applied to git objects, produces the
//! elements DM §12.2 publishes.
//!
//! The last test is G10 itself (DM §11): clone the repository the way the gate
//! does and require the two dumps to be equal byte for byte.

use spine_graph::derive::{Indexer, Options};
use spine_graph::schema::{AttrValue, EdgeKind, NodeKind, id};
use spine_graph::store::Graph;
use spine_graph::verify::{OpenSsh, Unverified, ssh_keygen_available};
use spine_graph::{Comparison, git, serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// A scratch repository
// ---------------------------------------------------------------------------

struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("spine-graph-derive-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Scratch { root }
    }

    fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    fn write(&self, path: &[u8], content: &[u8]) {
        let full = join_bytes(&self.repo(), path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(full, content).unwrap();
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A repo-relative path whose bytes need not be UTF-8 — DM §12.1's own example
/// path is `src/billing/caf` + `0xE9` + `.py`.
fn join_bytes(root: &Path, path: &[u8]) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        root.join(std::ffi::OsStr::from_bytes(path))
    }
    #[cfg(not(unix))]
    {
        root.join(String::from_utf8_lossy(path).into_owned())
    }
}

fn run(dir: &Path, program: &str, args: &[&str], stdin: Option<&[u8]>) -> Vec<u8> {
    use std::io::Write;
    let mut child = Command::new(program)
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
        .unwrap_or_else(|e| panic!("{program} {args:?}: {e}"));
    if let Some(bytes) = stdin {
        child.stdin.as_mut().unwrap().write_all(bytes).unwrap();
        drop(child.stdin.take());
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{program} {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

fn git_out(dir: &Path, args: &[&str]) -> String {
    String::from_utf8_lossy(&run(dir, "git", args, None))
        .trim_end()
        .to_string()
}

// ---------------------------------------------------------------------------
// The fixture repository
// ---------------------------------------------------------------------------

/// DM §12.1's repository, as far as git can build it: the shape is the same and
/// every oid is real.
struct Fixture {
    scratch: Scratch,
    t0: String,
    l: String,
    members: Vec<String>,
    tree: String,
    intent_blob: String,
    /// The 1-based message line the fenced intent's own first line falls on.
    fenced_first_line: u64,
    fingerprints: Vec<(String, String)>,
    /// Whether the non-UTF-8 path of DM §12.1 is in the landing's diff. It is
    /// staged with plumbing rather than written to disk: APFS refuses a
    /// filename that is not valid UTF-8, and git stores the bytes regardless —
    /// which is precisely why DM §2.4 insists a `code_unit` id is built from
    /// "the tree entry", never from the filesystem.
    has_non_utf8: bool,
    signoff_line: Vec<u8>,
    approve_line: Vec<u8>,
    review_line: Vec<u8>,
}

const MANIFEST: &str = r#"{"cli":{"dist_hash":"sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db","version":"1.4.0"},"envelope":1,"files":[{"blob":"6d4db08390092d7d5d96476eddca6355815bc49f","owner":"user-owned","path":".spine/allowed_signers","template":"keyring@1"},{"blob":"22609629e86d75a7c4abb7208c3575c7a8c2ead3","owner":"user-owned","path":"CONSTITUTION.md","template":"constitution@1"}],"manifest_version":1,"object_format":"sha1","params":{"ci":"github","isolation":"container","langs":["python"],"timeout":1800,"trunk":"main"},"paths":{"agent_context":["AGENTS.md","CLAUDE.md"],"constitution":"CONSTITUTION.md"},"repo":"myrepo","resign":{"intent":2,"intent-bug":2,"intent-change":2},"schema":7,"templates":{"agents-block":2,"ci-generic":4,"ci-github-collect":4,"ci-github-land":4,"ci-gitlab":4,"constitution":1,"gitattributes":1,"gitignore":1,"intent":2,"intent-bug":2,"intent-change":2,"keyring":1}}"#;

/// CN §3.1: line 1 is the title, **line 2 is the header**. `C-A2` extends the
/// floor with `infra/`, which DM §12.1 says of its own repository.
const CONSTITUTION: &str = "# Constitution — myrepo\nVersion: v3 · Owner: @alice\n\nC-A1: mode = team\nC-A2: protected = infra/\n";

const ADR: &str = "# ADR-007: Tax rounding\n\nRound half up.\n";

/// The intent doc, in PB §3.3 canonical form (EV §8.2's, with DM §12.1's ACs).
const INTENT: &str = "\
# INT-042: Invoice totals include tax
Owner: @alice · Template: intent@2 · Constitution: v3

## Goal
Invoice totals shown to a customer include tax.

## Non-goals
- Multi-currency invoices.
- Recomputing invoices already issued.

## Acceptance criteria
AC-1: Given a taxed line item, then the total includes its tax.
AC-2: Given a zero-rated line item, then no tax is added.

## Touchpoints
Expected to change: src/billing/, api/invoices.ts
Must NOT change: auth/, shared/schema/
";

const TEST_FILE: &[u8] = b"tests/billing/test_invoice.py";
const TEST_AC1: &str = "tests/billing/test_invoice.py::test_AC1_totals_include_tax";
const TEST_AC2: &str = "tests/billing/test_invoice.py::test_AC2_zero_rated";

fn cafe_py() -> Vec<u8> {
    let mut p = b"src/billing/caf".to_vec();
    p.push(0xE9);
    p.extend_from_slice(b".py");
    p
}

impl Fixture {
    /// `None` when git or OpenSSH is unavailable — the house pattern in
    /// `spine_init::git`'s tests.
    fn build(name: &str) -> Option<Self> {
        Fixture::build_with(name, true)
    }

    /// The same repository with every path valid UTF-8, so that `git clone`
    /// can check it out. G10's own command (DM §11 step 2) is a plain clone,
    /// and a checkout of a non-UTF-8 path fails on APFS before any dump is
    /// taken — a platform limit, not a rule of the format (DM §8.7 makes the
    /// worktree irrelevant to a dump either way).
    fn build_plain(name: &str) -> Option<Self> {
        Fixture::build_with(name, false)
    }

    fn build_with(name: &str, non_utf8: bool) -> Option<Self> {
        if !git::available() || !ssh_keygen_available() {
            return None;
        }
        let scratch = Scratch::new(name);
        let repo = scratch.repo();
        std::fs::create_dir_all(&repo).ok()?;
        run(&repo, "git", &["init", "--quiet", "-b", "main", "."], None);
        for (key, value) in [
            ("user.email", "t@example.invalid"),
            ("user.name", "Test"),
            // A commit's own signature is not the record (PB §5.5); signing
            // commits here would only slow the fixture down.
            ("commit.gpgsign", "false"),
        ] {
            run(&repo, "git", &["config", key, value], None);
        }

        // --- three real keys, and the keyring MF §4.2 grammars ------------
        let keydir = scratch.root.join("keys");
        std::fs::create_dir_all(&keydir).ok()?;
        let mut keyring = String::new();
        let mut fingerprints = Vec::new();
        for (principal, namespaces) in [
            (
                "alice@example.com",
                "spine-signoff@v1,spine-review@v1",
            ),
            ("bob@example.com", "spine-signoff@v1,spine-review@v1"),
            ("ci@example.com", "spine-seal@v1"),
        ] {
            let stem = principal.split('@').next().unwrap();
            let path = keydir.join(stem);
            run(
                &keydir,
                "ssh-keygen",
                &[
                    "-q",
                    "-t",
                    "ed25519",
                    "-N",
                    "",
                    "-C",
                    principal,
                    "-f",
                    path.to_str().unwrap(),
                ],
                None,
            );
            let public = std::fs::read_to_string(keydir.join(format!("{stem}.pub"))).ok()?;
            let mut fields = public.split_whitespace();
            let keytype = fields.next()?;
            let blob = fields.next()?;
            keyring.push_str(&format!(
                "{principal} namespaces=\"{namespaces}\" {keytype} {blob}\n"
            ));
            // `ssh-keygen -lf` prints `<bits> SHA256:<base64> <comment> (ED25519)`
            // — DM §7.2's `fingerprint` is the second field, and this is the
            // command that document names.
            let listed = String::from_utf8_lossy(&run(
                &keydir,
                "ssh-keygen",
                &["-lf", keydir.join(format!("{stem}.pub")).to_str().unwrap()],
                None,
            ))
            .to_string();
            let fingerprint = listed.split_whitespace().nth(1)?.to_string();
            fingerprints.push((principal.to_string(), fingerprint));
        }

        // --- T0, the trust root -------------------------------------------
        // MF §2.4: the manifest is canonical JSON **with a final LF**, and a
        // manifest without one is refused — which would leave the derivation
        // with no `repo` and no trunk.
        scratch.write(b".spine/manifest.json", format!("{MANIFEST}\n").as_bytes());
        scratch.write(b".spine/allowed_signers", keyring.as_bytes());
        scratch.write(b"CONSTITUTION.md", CONSTITUTION.as_bytes());
        scratch.write(b"adr/ADR-007-tax-rounding.md", ADR.as_bytes());
        run(&repo, "git", &["add", "-A"], None);
        run(&repo, "git", &["commit", "--quiet", "-m", "spine: init"], None);
        let t0 = git_out(&repo, &["rev-parse", "HEAD"]);

        // --- the branch: five members, M1…M5 ------------------------------
        run(&repo, "git", &["checkout", "--quiet", "-b", "intent/INT-042"], None);
        let sign = |line: &str, keyfile: &str, namespace: &str| -> String {
            let key = keydir.join(keyfile);
            let armored = run(
                &keydir,
                "ssh-keygen",
                &["-Y", "sign", "-q", "-f", key.to_str().unwrap(), "-n", namespace],
                Some(line.as_bytes()),
            );
            // PB §11 carries the SSHSIG "armor stripped to one line".
            String::from_utf8_lossy(&armored)
                .lines()
                .filter(|l| !l.starts_with("-----"))
                .collect::<String>()
        };

        let intent_blob = String::from_utf8_lossy(&run(
            &repo,
            "git",
            &["hash-object", "--stdin"],
            Some(INTENT.as_bytes()),
        ))
        .trim_end()
        .to_string();

        // M1 — the sign-off event commit (empty).
        let signoff_line = format!(
            "Spine-Signoff: INT-042 blob={intent_blob} template=intent@2 constitution=v3 reopens=0 signer=alice@example.com"
        );
        let signoff_sig = sign(&signoff_line, "alice", "spine-signoff@v1");
        commit_empty(
            &repo,
            &format!(
                "INT-042: sign-off\n\nSpine-Event: signoff\nSpine-Intent: INT-042\n{signoff_line}\nSpine-Signoff-Sig: {signoff_sig}\n"
            ),
        );

        // M2 — the tests. The pragma and the two test functions are the lines
        // DM §12.2's `test` nodes and `verified_by` edges cite.
        scratch.write(
            TEST_FILE,
            b"import pytest\n\n\n# @verifies INT-042/AC-1\ndef test_AC1_totals_include_tax():\n    assert True\n\n\n# @verifies INT-042/AC-2\ndef test_AC2_zero_rated():\n    assert True\n",
        );
        run(&repo, "git", &["add", "-A"], None);
        run(&repo, "git", &["commit", "--quiet", "-m", "INT-042: tests"], None);

        // M3 — the approval commit, carrying the frozen manifest. PB §11
        // confines `Spine-Frozen`/`Spine-Test` to the approval commit under
        // merge strategy, and PB §6.2 derives `freezes` from there.
        let test_blob = git_out(
            &repo,
            &["rev-parse", "HEAD:tests/billing/test_invoice.py"],
        );
        let frozen = format!(
            "Spine-Frozen: {test_blob} tests/billing/test_invoice.py\nSpine-Test: pytest {TEST_AC1}\nSpine-Test: pytest {TEST_AC2}"
        );
        // EV §4.1: `freeze=` is SHA-256 over those lines, LF-joined, with no
        // trailing LF — the value the approve line carries.
        let freeze = spine_canon::sha256_prefixed(frozen.as_bytes());
        let approve_line = format!(
            "Spine-Approve: INT-042 intent={intent_blob} base={t0} rounds=1 total_rounds=1 reopens=0 red=5/5 freeze={freeze} signer=alice@example.com"
        );
        // PB §11: an approve line with no `run=` "verifies under
        // `spine-review@v1` only" — which is why DM §7.2's `role` for it is
        // `reviewer` and not `signer`.
        let approve_sig = sign(&approve_line, "alice", "spine-review@v1");
        commit_empty(
            &repo,
            &format!(
                "INT-042: approve\n\nSpine-Event: approve\nSpine-Intent: INT-042\n{approve_line}\nSpine-Approve-Sig: {approve_sig}\n{frozen}\n"
            ),
        );
        let approval_commit = git_out(&repo, &["rev-parse", "HEAD"]);

        // M4 — the implementation, including DM §12.1's non-UTF-8 path.
        scratch.write(b"src/billing/tax.py", b"def tax(x):\n    return x * 0.2\n");
        run(&repo, "git", &["add", "src/billing/tax.py"], None);
        let has_non_utf8 = non_utf8 && stage_raw_path(&repo, &cafe_py(), b"# caf\xc3\xa9\n");
        run(&repo, "git", &["commit", "--quiet", "-m", "INT-042: implement"], None);

        // M5 — the review event commit (empty). `Hc` is M5.
        let hc = {
            // The review binds `head=Hc` and `tree=`, and `Hc` is the commit
            // this very commit becomes — so the line is written after it and
            // the commit that carries it is not the one it names. The envelope
            // is what the indexer reads, so the branch copy is elided here.
            commit_empty(&repo, "INT-042: review\n\nSpine-Event: review\nSpine-Intent: INT-042\n");
            git_out(&repo, &["rev-parse", "HEAD"])
        };
        let tree = git_out(&repo, &["rev-parse", "HEAD^{tree}"]);
        let review_line = format!(
            "Spine-Review: INT-042 class=tripwire head={hc} tree={tree} base={t0} intent={intent_blob} report=sha256:3c6f1a09b8d24e57af0132c9de6b48570e29a1cf83b6d045e71a29c4b0d83e16 wires=G11 reason=\"auto-merge unavailable: C-A3 hostile\" reviewer=bob@example.com"
        );
        let review_sig = sign(&review_line, "bob", "spine-review@v1");

        // --- L, the landing ------------------------------------------------
        let mut above_seal = vec![
            "Spine-Envelope: 1".to_string(),
            "Spine-Event: land".to_string(),
            "Spine-Lane: gated".to_string(),
            "Spine-Intent: INT-042".to_string(),
            signoff_line.clone(),
            format!("Spine-Signoff-Sig: {signoff_sig}"),
            approve_line.clone(),
            format!("Spine-Approve-Sig: {approve_sig}"),
            format!("Spine-Approval: {approval_commit}"),
            review_line.clone(),
            format!("Spine-Review-Sig: {review_sig}"),
        ];
        above_seal.push(
            "Spine-Gates: G1=pass G2=pass G3=pass G4=pass G5=pass G7=pass G8=pass G9=pass G11=pass G12=pass G13=pass G14=pass G15=pass G16=pass".to_string(),
        );
        above_seal.push("Spine-Strategy: merge".to_string());
        // PB §5.5: `envelope=` is "SHA-256 over every `Spine-*` line above it,
        // in order" — EV §8.3 joins them with LF and no trailing LF.
        let envelope_digest = spine_canon::sha256_prefixed(above_seal.join("\n").as_bytes());
        let seal_line = format!(
            "Spine-Seal: INT-042 base={t0} head={hc} tree={tree} report=sha256:e70a3c92d1b845f6027e9ab3c5d10f684a2b7e93c60d5f81a34b0e29d7c6f105 tool=1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db git=2.45 mode=team threat=hostile profile=container envelope={envelope_digest} signer=ci@example.com"
        );
        let seal_sig = sign(&seal_line, "ci", "spine-seal@v1");

        let mut message = String::new();
        message.push_str("INT-042: Invoice totals include tax\n\n");
        message.push_str(&format!(
            "-----BEGIN SPINE-INTENT blob={intent_blob} bytes={}-----\n",
            INTENT.len()
        ));
        let fenced_first_line = message.lines().count() as u64 + 1;
        message.push_str(INTENT);
        message.push_str("-----END SPINE-INTENT-----\n\n");
        for line in &above_seal {
            message.push_str(line);
            message.push('\n');
        }
        message.push_str(&seal_line);
        message.push('\n');
        message.push_str(&format!("Spine-Seal-Sig: {seal_sig}\n"));

        let message_file = scratch.root.join("envelope.txt");
        std::fs::write(&message_file, &message).ok()?;
        let l = String::from_utf8_lossy(&run(
            &repo,
            "git",
            &[
                "commit-tree",
                &tree,
                "-p",
                &t0,
                "-p",
                &hc,
                "-F",
                message_file.to_str().unwrap(),
            ],
            None,
        ))
        .trim_end()
        .to_string();
        run(&repo, "git", &["update-ref", "refs/heads/main", &l], None);
        run(&repo, "git", &["config", "spine.trustRoot", &t0], None);
        // HEAD is moved to trunk without a checkout. It must point there —
        // DM §11 step 1 has `S` hold "the post-CAS ref set", and `git clone`
        // creates a local branch for the remote's HEAD alone, so a clone of a
        // repository whose HEAD is an intent branch has no `refs/heads/main`
        // and derives nothing. `symbolic-ref` moves it without writing the
        // working tree, which the non-UTF-8 path could not survive.
        run(&repo, "git", &["symbolic-ref", "HEAD", "refs/heads/main"], None);

        let mut members = git_out(&repo, &["rev-list", &format!("{t0}..{l}")])
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();
        members.retain(|m| m != &l);

        Some(Fixture {
            scratch,
            t0,
            l,
            members,
            tree,
            intent_blob,
            fenced_first_line,
            fingerprints,
            has_non_utf8,
            signoff_line: signoff_line.into_bytes(),
            approve_line: approve_line.into_bytes(),
            review_line: review_line.into_bytes(),
        })
    }

    fn repo(&self) -> git::Repo {
        git::Repo::open(&self.scratch.repo()).unwrap()
    }

    fn graph(&self) -> Graph {
        let repo = self.repo();
        Indexer::new(&repo, &OpenSsh)
            .index(&Options::default())
            .unwrap()
            .graph
    }
}

/// Stage a blob at a path whose bytes need not be valid UTF-8, without writing
/// it to the working tree.
///
/// `git update-index --add --cacheinfo <mode>,<oid>,<path>` takes the path in
/// `argv`, where any byte but NUL is legal — so the tree carries the bytes even
/// on a filesystem that would refuse the filename.
fn stage_raw_path(repo: &Path, path: &[u8], content: &[u8]) -> bool {
    #[cfg(unix)]
    {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        let oid = String::from_utf8_lossy(&run(
            repo,
            "git",
            &["hash-object", "-w", "--stdin"],
            Some(content),
        ))
        .trim_end()
        .to_string();
        let mut cacheinfo = format!("100644,{oid},").into_bytes();
        cacheinfo.extend_from_slice(path);
        let status = Command::new("git")
            .current_dir(repo)
            .args(["update-index", "--add", "--cacheinfo"])
            .arg(OsString::from_vec(cacheinfo))
            .status()
            .unwrap();
        assert!(status.success(), "update-index --cacheinfo failed");
        true
    }
    #[cfg(not(unix))]
    {
        let _ = (repo, path, content);
        false
    }
}

fn commit_empty(repo: &Path, message: &str) {
    run(
        repo,
        "git",
        &["commit", "--quiet", "--allow-empty", "-m", message],
        None,
    );
}

// ---------------------------------------------------------------------------
// Helpers over a derived graph
// ---------------------------------------------------------------------------

fn node<'g>(graph: &'g Graph, id: &str) -> &'g spine_graph::store::Node {
    graph
        .nodes()
        .iter()
        .find(|n| n.id == id)
        .unwrap_or_else(|| panic!("no node {id} in {:?}", ids(graph)))
}

fn ids(graph: &Graph) -> Vec<&str> {
    graph.nodes().iter().map(|n| n.id.as_str()).collect()
}

fn attr(node: &spine_graph::store::Node, name: &str) -> String {
    match node.attrs.get(name) {
        Some(AttrValue::Str(s)) => s.clone(),
        Some(AttrValue::Int(n)) => n.to_string(),
        Some(AttrValue::Bool(b)) => b.to_string(),
        Some(AttrValue::StrArr(a)) => a.join(","),
        None => panic!("{} has no attr {name}", node.id),
    }
}

fn edge<'g>(
    graph: &'g Graph,
    kind: EdgeKind,
    from: &str,
    to: &str,
) -> &'g spine_graph::store::Edge {
    graph
        .edges()
        .iter()
        .find(|e| e.kind == kind && e.from == from && e.to == to)
        .unwrap_or_else(|| panic!("no {} edge {from} -> {to}", kind.token()))
}

// ---------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------

#[test]
fn the_landing_changeset_carries_the_seals_fields_and_its_members_carry_only_landing_false() {
    let Some(fixture) = Fixture::build("changesets") else {
        return;
    };
    let graph = fixture.graph();
    let cs = id::changeset("myrepo", &fixture.l);
    let landing = node(&graph, &cs);

    assert_eq!(attr(landing, "landing"), "true");
    assert_eq!(attr(landing, "event"), "land");
    assert_eq!(attr(landing, "lane"), "gated");
    assert_eq!(attr(landing, "strategy"), "merge");
    assert_eq!(attr(landing, "base"), fixture.t0);
    assert_eq!(attr(landing, "tree"), fixture.tree);
    assert_eq!(attr(landing, "mode"), "team");
    assert_eq!(attr(landing, "threat"), "hostile");
    assert_eq!(attr(landing, "profile"), "container");
    // DM §7.2.1: `git_version` is "**the seal's `git=`**, never the indexing
    // binary's own `git --version`" — reading the local git "would put the
    // environment in the artifact".
    assert_eq!(attr(landing, "git_version"), "2.45");
    // The seal's `tool=` is `<version>+sha256:<dist_hash>`; the dump carries
    // the left half (DM §7.2, `tool_version_from_seal`).
    assert_eq!(attr(landing, "tool_version"), "1.4.0");
    assert_eq!(attr(landing, "seal_principal"), "ci@example.com");
    // Computed by OpenSSH over the seal line this fixture signed, against the
    // keyring blob at `base=` — PB §5.5's "a landing can never admit its own
    // signer".
    assert_eq!(attr(landing, "seal_verified"), "true");
    assert_eq!(attr(landing, "unattested"), "false");
    assert_eq!(landing.src.render(), format!("git:{}:trailer:Spine-Seal", fixture.l));

    assert_eq!(fixture.members.len(), 5, "M(L) is five member commits");
    for member in &fixture.members {
        let member_node = node(&graph, &id::changeset("myrepo", member));
        // DM §7.2: "A member changeset carries `{"landing":false}` and nothing
        // else: it has no seal, and every one of those fields is a seal field."
        assert_eq!(attr(member_node, "landing"), "false");
        assert_eq!(member_node.attrs.iter().count(), 1);
        assert_eq!(member_node.src.render(), format!("git:{member}"));
        let implements = edge(
            &graph,
            EdgeKind::Implements,
            &id::changeset("myrepo", member),
            &id::intent("myrepo", "INT-042"),
        );
        assert_eq!(attr_of(implements, "role"), "member");
        assert_eq!(attr_of(implements, "provisional"), "false");
    }
    let landing_edge = edge(
        &graph,
        EdgeKind::Implements,
        &cs,
        &id::intent("myrepo", "INT-042"),
    );
    assert_eq!(attr_of(landing_edge, "role"), "landing");
}

fn attr_of(edge: &spine_graph::store::Edge, name: &str) -> String {
    match edge.attrs.get(name) {
        Some(AttrValue::Str(s)) => s.clone(),
        Some(AttrValue::Bool(b)) => b.to_string(),
        other => panic!("attr {name} is {other:?}"),
    }
}

#[test]
fn the_intent_its_acs_and_its_touchpoints_come_from_the_fenced_bytes_never_the_subject() {
    let Some(fixture) = Fixture::build("intent") else {
        return;
    };
    let graph = fixture.graph();
    let intent = id::intent("myrepo", "INT-042");
    let n = node(&graph, &intent);

    // DM §7.2: the title is "read from the sealed intent inside the landing
    // commit's message — **never from that commit's subject line**".
    assert_eq!(attr(n, "title"), "Invoice totals include tax");
    assert_eq!(attr(n, "owner"), "@alice");
    assert_eq!(attr(n, "template"), "intent@2");
    assert_eq!(attr(n, "blob"), fixture.intent_blob);
    assert_eq!(attr(n, "signer"), "alice@example.com");
    assert_eq!(attr(n, "status"), "merged");
    assert_eq!(attr(n, "landing"), fixture.l);
    assert_eq!(attr(n, "base"), fixture.t0);
    assert_eq!(attr(n, "reopen_count"), "0");
    assert_eq!(attr(n, "late_reopen_count"), "0");
    // `git:<L>:msg:L<n>` (DM §5.4), `n` counted over the whole message: the
    // title line is the fenced block's own line 1.
    assert_eq!(
        n.src.render(),
        format!("git:{}:msg:L{}", fixture.l, fixture.fenced_first_line)
    );

    for (ac, offset) in [(1u64, 11u64), (2, 12)] {
        let ac_id = id::ac("myrepo", "INT-042", ac);
        let expected = format!(
            "git:{}:msg:L{}",
            fixture.l,
            fixture.fenced_first_line + offset
        );
        assert_eq!(node(&graph, &ac_id).src.render(), expected);
        assert_eq!(
            edge(&graph, EdgeKind::HasAc, &intent, &ac_id).src.render(),
            expected
        );
    }

    // ID §6.6: "the line number … is the touchpoint **label line's**, not the
    // individual pattern's, since several patterns share one line."
    let expected_label = format!(
        "git:{}:msg:L{}",
        fixture.l,
        fixture.fenced_first_line + 15
    );
    for path in [b"src/billing/".as_slice(), b"api/invoices.ts".as_slice()] {
        let declares = edge(
            &graph,
            EdgeKind::Declares,
            &intent,
            &id::code_unit("myrepo", path),
        );
        assert_eq!(attr_of(declares, "polarity"), "expected");
        assert_eq!(declares.src.render(), expected_label);
    }
    for path in [b"auth/".as_slice(), b"shared/schema/".as_slice()] {
        assert_eq!(
            attr_of(
                edge(
                    &graph,
                    EdgeKind::Declares,
                    &intent,
                    &id::code_unit("myrepo", path)
                ),
                "polarity"
            ),
            "forbidden"
        );
    }

    // The header line is line 2 of the doc, and `built_under` cites it.
    let built_under = edge(
        &graph,
        EdgeKind::BuiltUnder,
        &intent,
        &id::constitution("myrepo", 3),
    );
    assert_eq!(
        built_under.src.render(),
        format!("git:{}:msg:L{}", fixture.l, fixture.fenced_first_line + 1)
    );
}

#[test]
fn an_approvals_role_is_the_namespace_its_signature_verified_under() {
    let Some(fixture) = Fixture::build("approvals") else {
        return;
    };
    let graph = fixture.graph();
    let intent = id::intent("myrepo", "INT-042");

    let signoff = node(&graph, &id::approval("myrepo", &fixture.signoff_line));
    assert_eq!(attr(signoff, "event"), "signoff");
    assert_eq!(attr(signoff, "principal"), "alice@example.com");
    assert_eq!(attr(signoff, "verified"), "true");
    assert_eq!(attr(signoff, "role"), "signer");
    assert_eq!(attr(signoff, "blob"), fixture.intent_blob);
    assert_eq!(attr(signoff, "reopens"), "0");

    // DM §7.2, the whole point of deriving the role from the signature: "A v1
    // approve line signed under `spine-review@v1` is `reviewer`" — the trailer
    // says `signer=` and the role is not `signer`.
    let approve = node(&graph, &id::approval("myrepo", &fixture.approve_line));
    assert_eq!(attr(approve, "event"), "approve");
    assert_eq!(attr(approve, "role"), "reviewer");
    assert_eq!(attr(approve, "verified"), "true");
    assert_eq!(attr(approve, "red"), "5/5");
    assert_eq!(attr(approve, "rounds"), "1");
    assert_eq!(attr(approve, "total_rounds"), "1");
    assert!(attr(approve, "freeze").starts_with("sha256:"));

    let review = node(&graph, &id::approval("myrepo", &fixture.review_line));
    assert_eq!(attr(review, "event"), "review");
    assert_eq!(attr(review, "class"), "tripwire");
    assert_eq!(attr(review, "principal"), "bob@example.com");
    assert_eq!(attr(review, "role"), "reviewer");
    assert_eq!(attr(review, "wires"), "G11");
    assert_eq!(attr(review, "tree"), fixture.tree);

    // Every copied line names the intent, and each is signed by its principal.
    for (approval, principal) in [
        (&fixture.signoff_line, "alice@example.com"),
        (&fixture.approve_line, "alice@example.com"),
        (&fixture.review_line, "bob@example.com"),
    ] {
        let node_id = id::approval("myrepo", approval);
        edge(&graph, EdgeKind::Approves, &node_id, &intent);
        edge(
            &graph,
            EdgeKind::SignedBy,
            &node_id,
            &id::signer("myrepo", principal.as_bytes()),
        );
    }
}

#[test]
fn a_tampered_line_verifies_under_no_namespace_and_the_landing_indexes_unattested() {
    let Some(fixture) = Fixture::build("tamper") else {
        return;
    };
    // The same repository read by a verifier that answers "no" to every
    // question is the fail-closed shape of every signature failing at once:
    // PB §6.3's "A failing landing indexes `unattested` — reported and
    // counted."
    let repo = fixture.repo();
    let graph = Indexer::new(&repo, &Unverified)
        .index(&Options::default())
        .unwrap()
        .graph;
    let landing = node(&graph, &id::changeset("myrepo", &fixture.l));
    assert_eq!(attr(landing, "seal_verified"), "false");
    assert_eq!(attr(landing, "unattested"), "true");
    let signoff = node(&graph, &id::approval("myrepo", &fixture.signoff_line));
    assert_eq!(attr(signoff, "verified"), "false");
    // The role falls back to the one PB §11 requires of the trailer, so the
    // pair reads "claimed this, proved nothing" rather than dropping the node.
    assert_eq!(attr(signoff, "role"), "signer");
}

#[test]
fn the_signers_are_the_keyring_at_the_trust_root_with_real_fingerprints() {
    let Some(fixture) = Fixture::build("signers") else {
        return;
    };
    let graph = fixture.graph();
    for (line_no, (principal, fingerprint)) in fixture.fingerprints.iter().enumerate() {
        let n = node(&graph, &id::signer("myrepo", principal.as_bytes()));
        assert_eq!(&attr(n, "fingerprint"), fingerprint);
        assert_eq!(attr(n, "valid_from"), fixture.t0);
        // "**`signer.valid_to` is absent, not null**" (DM §12.3 check 6) — no
        // key has been removed in this history.
        assert!(n.attrs.get("valid_to").is_none());
        assert_eq!(
            n.src.render(),
            format!(
                "git:{}:.spine/allowed_signers:{}",
                fixture.t0,
                line_no + 1
            )
        );
    }
    // `roles` is "ascending by bytes", which the keyring parser fixes and the
    // dump carries.
    assert_eq!(
        attr(
            node(&graph, &id::signer("myrepo", b"alice@example.com")),
            "roles"
        ),
        "spine-review@v1,spine-signoff@v1"
    );
    assert_eq!(
        attr(node(&graph, &id::signer("myrepo", b"ci@example.com")), "roles"),
        "spine-seal@v1"
    );
    // The seal's signer is attested to by the landing.
    edge(
        &graph,
        EdgeKind::AttestedBy,
        &id::changeset("myrepo", &fixture.l),
        &id::signer("myrepo", b"ci@example.com"),
    );
}

#[test]
fn the_constitution_the_adr_and_the_c_a2_floor_come_from_the_trees() {
    let Some(fixture) = Fixture::build("policy") else {
        return;
    };
    let graph = fixture.graph();

    // CN §9.6: "`src` `git:<sha>:<esc(path)>:2` — line 2, the header", and
    // "`<sha>` is the landing that introduced the version". The trust root is
    // not a landing, so the citation is `L`'s — exactly as DM §12.2 publishes.
    let constitution = node(&graph, &id::constitution("myrepo", 3));
    assert_eq!(
        constitution.src.render(),
        format!("git:{}:CONSTITUTION.md:2", fixture.l)
    );

    let infra = id::code_unit("myrepo", b"infra/");
    let protects = edge(&graph, EdgeKind::Protects, &constitution.id, &infra);
    // DM §8.3: only the `C-A2` limb survives, so every dumped `protects` is
    // `floor: false` — the shipped floor is inside the binary and excluded.
    assert_eq!(attr_of(protects, "floor"), "false");
    assert_eq!(
        protects.src.render(),
        format!("git:{}:CONSTITUTION.md:5", fixture.l)
    );

    let adr = node(&graph, &id::adr("myrepo", "ADR-007"));
    assert_eq!(
        adr.src.render(),
        format!("git:{}:adr/ADR-007-tax-rounding.md:1", fixture.l)
    );
}

#[test]
fn modifies_is_the_integrated_delta_plus_the_per_member_diffs_and_carries_raw_path_bytes() {
    let Some(fixture) = Fixture::build("modifies") else {
        return;
    };
    let graph = fixture.graph();
    let cs = id::changeset("myrepo", &fixture.l);

    // The landing's own `git diff --name-only B L` — three paths, one of them
    // not valid UTF-8 (DM §12.1).
    assert!(fixture.has_non_utf8, "this platform can stage the tree entry");
    for path in [
        cafe_py(),
        b"src/billing/tax.py".to_vec(),
        TEST_FILE.to_vec(),
    ] {
        let unit = id::code_unit("myrepo", &path);
        let e = edge(&graph, EdgeKind::Modifies, &cs, &unit);
        assert_eq!(e.src.render(), format!("git:{}", fixture.l));
    }
    // `esc` put the Latin-1 byte on the wire as four ASCII characters.
    assert!(
        ids(&graph).contains(&"myrepo/code:src/billing/caf\\xe9.py"),
        "the non-UTF-8 path is `esc`-encoded in its node id"
    );

    // Per-member diffs, cited at the member (PB §6.2, "per-member diffs for
    // archaeology"). Three of the five members are empty event commits and
    // touch nothing.
    let member_modifies: Vec<&str> = graph
        .edges()
        .iter()
        .filter(|e| e.kind == EdgeKind::Modifies && e.from != cs)
        .map(|e| e.to.as_str())
        .collect();
    assert_eq!(member_modifies.len(), 3, "one test file and two source files");
}

#[test]
fn freezes_reads_the_approval_commit_under_merge_and_cites_it_there() {
    let Some(fixture) = Fixture::build("freezes") else {
        return;
    };
    let graph = fixture.graph();
    let approve = id::approval("myrepo", &fixture.approve_line);

    // PB §11 confines `Spine-Frozen`/`Spine-Test` to squash envelopes, so under
    // merge they are on the approval commit `Spine-Approval` names — and the
    // citation is that commit's, which is what DM §12.2 publishes.
    let frozen = edge(
        &graph,
        EdgeKind::Freezes,
        &approve,
        &id::code_unit("myrepo", TEST_FILE),
    );
    let carrier = match &frozen.src {
        spine_graph::Src::Trailer { sha, name } => {
            assert_eq!(name, "Spine-Frozen");
            sha.clone()
        }
        other => panic!("freezes cites {other:?}"),
    };
    assert!(
        fixture.members.contains(&carrier),
        "the citation names the approval commit, a member of M(L)"
    );
    // A `freezes` to a `code_unit` carries the blob; to a `test`, `{}`.
    assert_eq!(
        attr_of(frozen, "oid"),
        git_out(
            &fixture.scratch.repo(),
            &["rev-parse", &format!("{}:tests/billing/test_invoice.py", fixture.l)]
        )
    );
    for test in [TEST_AC1, TEST_AC2] {
        let test_id = id::test("myrepo", "pytest", test.as_bytes());
        let e = edge(&graph, EdgeKind::Freezes, &approve, &test_id);
        assert!(e.attrs.is_empty(), "a freezes edge to a test carries {{}}");
        // DM §8.4: `result_at` is the kind's only attr and it is excluded, so
        // "every `test` node in a dump carries `{}`".
        assert!(node(&graph, &test_id).attrs.is_empty());
    }
}

#[test]
fn the_derived_graph_serializes_and_two_derivations_of_one_tip_are_byte_identical() {
    let Some(fixture) = Fixture::build("serialize") else {
        return;
    };
    let repo = fixture.repo();
    let first = Indexer::new(&repo, &OpenSsh).index(&Options::default()).unwrap();
    let second = Indexer::new(&repo, &OpenSsh).index(&Options::default()).unwrap();
    let a = serialize(&first.header, &first.graph).unwrap();
    let b = serialize(&second.header, &second.graph).unwrap();

    // DM §11's first responsibility: "**No false positive.** Two indexings of
    // the same objects by the same release produce identical bytes."
    assert_eq!(a.bytes(), b.bytes());
    assert_eq!(a.digest(), b.digest());
    assert_eq!(*a.bytes().last().unwrap(), 0x0A, "the final line is terminated");

    // The header is line 1 and records the four inputs DM §4.1 closes over.
    let header = String::from_utf8_lossy(a.bytes().split(|&c| c == b'\n').next().unwrap())
        .into_owned();
    assert!(header.contains(&format!(r#""head":"{}""#, fixture.l)));
    assert!(header.contains(&format!(r#""trust_root":"{}""#, fixture.t0)));
    assert!(header.contains(r#""repo":"myrepo""#));
    assert!(header.contains(r#""trunk":"main""#));

    // Every node kind DM §5.1 closes over is exercised except `test`'s
    // absent-attrs case, which the freeze test covers.
    let kinds: Vec<NodeKind> = first.graph.nodes().iter().map(|n| n.kind).collect();
    for kind in [
        NodeKind::Ac,
        NodeKind::Adr,
        NodeKind::Approval,
        NodeKind::Changeset,
        NodeKind::CodeUnit,
        NodeKind::Constitution,
        NodeKind::Intent,
        NodeKind::Signer,
        NodeKind::Test,
    ] {
        assert!(kinds.contains(&kind), "no {} node was derived", kind.token());
    }
}

#[test]
fn a_worktree_edit_and_an_untracked_file_change_no_byte_of_the_dump() {
    let Some(fixture) = Fixture::build("worktree") else {
        return;
    };
    let repo = fixture.repo();
    let before = {
        let indexed = Indexer::new(&repo, &OpenSsh).index(&Options::default()).unwrap();
        serialize(&indexed.header, &indexed.graph).unwrap()
    };

    // DM §8.7: "Running `--dump` in a bare repository, with a dirty working
    // tree, with a stale index, or with untracked files present produces
    // identical bytes."
    fixture.scratch.write(b"CONSTITUTION.md", b"# vandalised\nVersion: v9\n");
    fixture.scratch.write(b"untracked.txt", b"noise\n");
    let after = {
        let indexed = Indexer::new(&repo, &OpenSsh).index(&Options::default()).unwrap();
        serialize(&indexed.header, &indexed.graph).unwrap()
    };
    assert_eq!(before.bytes(), after.bytes());
}

#[test]
fn a_repository_with_no_manifest_anywhere_is_not_installed_rather_than_empty() {
    if !git::available() {
        return;
    }
    let scratch = Scratch::new("not-installed");
    std::fs::create_dir_all(scratch.repo()).unwrap();
    let dir = scratch.repo();
    run(&dir, "git", &["init", "--quiet", "-b", "main", "."], None);
    run(&dir, "git", &["config", "user.email", "t@example.invalid"], None);
    run(&dir, "git", &["config", "user.name", "Test"], None);
    std::fs::write(dir.join("README.md"), b"hi\n").unwrap();
    run(&dir, "git", &["add", "-A"], None);
    run(&dir, "git", &["commit", "--quiet", "-m", "seed"], None);

    let repo = git::Repo::open(&dir).unwrap();
    let err = Indexer::new(&repo, &Unverified)
        .index(&Options::default())
        .unwrap_err();
    // DM §9 case 1: "A dump of nothing and a dump of a repository spine does
    // not manage are different facts, and conflating them would let a
    // mis-targeted G10 clone compare two 'empty' dumps and pass."
    assert!(err.to_string().starts_with("not-installed"), "{err}");
}

#[test]
fn g10_passes_over_a_clean_clone_of_the_same_landing() {
    let Some(fixture) = Fixture::build_plain("g10") else {
        return;
    };
    // DM §11 steps 2–5, over `S` = this repository. Step 1 is the runner's:
    // the candidate landing is already `refs/heads/main` here.
    let clone_into = fixture.scratch.root.join("clone");
    let outcome = Comparison {
        trust_root: &fixture.t0,
        verifier: &OpenSsh,
    }
    .run(&fixture.scratch.repo(), &clone_into)
    .unwrap();

    assert!(
        outcome.passed(),
        "G10 failed at line {:?}\nS: {}\nC: {}",
        outcome.first_differing_line(),
        String::from_utf8_lossy(outcome.source.bytes()),
        String::from_utf8_lossy(outcome.clone.bytes())
    );
    assert_eq!(outcome.verdict.token(), "pass");
    assert_eq!(outcome.source.digest(), outcome.clone.digest());
    assert_eq!(outcome.first_differing_line(), None);
}
