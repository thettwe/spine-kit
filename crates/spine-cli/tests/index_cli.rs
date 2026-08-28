//! `spine index` at the CLI boundary, where DM §2.2's rule about stdout lives.
//!
//! The derivation itself is `spine-graph`'s and is tested there against real
//! repositories. What is only testable here is the property the command has and
//! the library cannot: **stdout carries the dump and nothing else**.

use std::path::{Path, PathBuf};
use std::process::Command;

fn spine() -> PathBuf {
    // `CARGO_BIN_EXE_<name>` is the binary this test was built against, so the
    // test never runs a stale one from `PATH`.
    PathBuf::from(env!("CARGO_BIN_EXE_spine"))
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

/// A conforming `.spine/manifest.json`, canonical (MF §2.4: one line, one LF).
///
/// `dist_hash` is the SHA-256 of the 26 ASCII bytes `spine-index-test-artifact`
/// — a computed value that is unmistakably not a real artifact list, since
/// nothing here compares it to one and a plausible-looking digest in a tree is
/// worse than an obviously synthetic one.
const MANIFEST: &str = concat!(
    r#"{"cli":{"dist_hash":"sha256:"#,
    "0e0d3a2fc7ad3ff23f2d6bd8fb6f2e0d4f3a9e0f2f4f7f9e4c8f4a0d5b1c6e2a",
    r#"","version":"1.4.0"},"envelope":1,"files":[],"manifest_version":1,"#,
    r#""object_format":"sha1","params":{"ci":"generic","isolation":"none","#,
    r#""langs":["python"],"timeout":1800,"trunk":"main"},"#,
    r#""paths":{"constitution":"CONSTITUTION.md"},"repo":"scratch","#,
    r#""resign":{"intent":1,"intent-bug":1,"intent-change":1},"schema":7,"#,
    r#""templates":{"intent":1,"intent-bug":1,"intent-change":1}}"#,
    "\n"
);

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Option<Self> {
        let dir = std::env::temp_dir().join(format!("spine-index-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok()?;
        git(&dir, &["init", "-q", "-b", "main", "."])?;
        git(&dir, &["config", "user.email", "t@example.invalid"])?;
        git(&dir, &["config", "user.name", "Test"])?;
        std::fs::write(dir.join("README.md"), "# scratch\n").ok()?;
        // DM §4.2 step 4 refuses `not-installed` where no manifest is reachable
        // on the first-parent walk, so a repository to index is a repository
        // with one. Written by hand rather than by `spine init`, which needs a
        // release frozen into the binary; only `params.trunk` and `repo` are
        // read by the derivation this test exercises.
        std::fs::create_dir_all(dir.join(".spine")).ok()?;
        std::fs::write(dir.join(".spine/manifest.json"), MANIFEST).ok()?;
        git(&dir, &["add", "-A"])?;
        git(&dir, &["commit", "-q", "-m", "seed"])?;
        Some(Scratch(dir))
    }

    /// `(exit, stdout, stderr)` — kept apart, which is the whole point here.
    fn index(&self, args: &[&str]) -> (i32, Vec<u8>, String) {
        let mut all = vec!["index"];
        all.extend_from_slice(args);
        let out = Command::new(spine())
            .current_dir(&self.0)
            .args(&all)
            .output()
            .expect("spine runs");
        (
            out.status.code().unwrap_or(-1),
            out.stdout,
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git_available() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

/// DM §2.2: "`spine index --dump` writes exactly these bytes to **stdout** and
/// nothing else to stdout."
///
/// Everything the command has to say — the unverified-signature warning, the
/// missing trust root, the node counts — goes to stderr, so a caller may pipe
/// stdout into `sha256sum` and get the dump's own digest.
#[test]
fn dump_writes_the_artifact_to_stdout_and_diagnostics_to_stderr() {
    let Some(scratch) = Scratch::new("stdout") else {
        return;
    };
    if !git_available() {
        return;
    }

    let (code, stdout, _) = scratch.index(&["--dump"]);
    assert_eq!(code, 0);

    // A dump is at least its header line, and every line is LF-terminated
    // (DM §2.2), so the last byte is an LF and nothing follows it.
    assert!(!stdout.is_empty());
    assert_eq!(stdout.last(), Some(&b'\n'));
    let text = String::from_utf8(stdout.clone()).expect("this dump is ASCII");
    let header = text.lines().next().expect("a header line");
    assert!(header.contains(r#""t":"header""#), "{header}");
    assert!(header.contains(r#""dump_version":1"#), "{header}");

    // Nothing conversational reached stdout.
    for noise in ["indexed", "spine index:", "node(s)"] {
        assert!(!text.contains(noise), "{noise:?} reached stdout:\n{text}");
    }
}

/// Without `--dump` the artifact is not written at all: the command reports and
/// stdout stays empty, so `spine index` in a pipeline cannot be mistaken for
/// `spine index --dump`.
#[test]
fn a_plain_index_writes_nothing_to_stdout() {
    let Some(scratch) = Scratch::new("plain") else {
        return;
    };
    if !git_available() {
        return;
    }

    let (code, stdout, stderr) = scratch.index(&[]);
    assert_eq!(code, 0);
    assert!(stdout.is_empty(), "stdout: {stdout:?}");
    assert!(stderr.contains("indexed"), "{stderr}");
}

/// PB §11's signature carries `--fresh`, and PB §7.4 rule 3 requires what it
/// asks for on every run anyway: "The graph is rebuilt from git objects, every
/// run … no SQLite file is fetched, cached or trusted from anywhere." So it is
/// accepted and changes nothing — and, in particular, writes no cache file.
#[test]
fn fresh_is_accepted_and_produces_the_same_bytes() {
    let Some(scratch) = Scratch::new("fresh") else {
        return;
    };
    if !git_available() {
        return;
    }

    let (_, plain, _) = scratch.index(&["--dump"]);
    let (code, fresh, _) = scratch.index(&["--fresh", "--dump"]);
    assert_eq!(code, 0);
    assert_eq!(plain, fresh);
    assert!(
        !scratch.0.join(".spine/cache/graph.sqlite").exists(),
        "no cache is written, so none can be stale or read under the wrong schema"
    );
}

/// DM §4.1: "A dump is a function of exactly four things: the trunk tip's oid,
/// the git objects reachable from it, the trust root, and the pinned release."
///
/// The trust root is a git config value naming an object, so nothing else in
/// the artifact would reveal a mismatch — which is why DM §3.1 records it.
#[test]
fn the_trust_root_reaches_the_header() {
    let Some(scratch) = Scratch::new("trust-root") else {
        return;
    };
    if !git_available() {
        return;
    }

    let (_, without, _) = scratch.index(&["--dump"]);
    let text = String::from_utf8_lossy(&without).to_string();
    assert!(!text.contains("trust_root"), "{text}");

    let root = git(&scratch.0, &["rev-parse", "HEAD"]).expect("a commit");
    git(&scratch.0, &["config", "spine.trustRoot", &root]).expect("config set");

    let (_, with, _) = scratch.index(&["--dump"]);
    let text = String::from_utf8_lossy(&with).to_string();
    assert!(
        text.contains(&format!(r#""trust_root":"{root}""#)),
        "{text}"
    );
    assert_ne!(without, with, "the trust root is an input to the bytes");
}
