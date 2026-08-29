//! `spine check --approve` end to end: closure, base-restored tree, a real
//! pytest run, and a signed approval record.
//!
//! The repository is deliberately one where the answer is checkable by hand:
//! `total()` on trunk ignores the rate, so the AC-1 test (which expects tax)
//! fails against the base-restored tree and the AC-2 test (zero rate) passes.
//! `red=1/2` is the only correct answer, and a measurement that ran against the
//! *candidate's* code — the thing G12 exists to prevent — would give `0/2`.

use std::path::{Path, PathBuf};
use std::process::Command;

fn spine() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_spine"))
}

fn pytest() -> Option<PathBuf> {
    let path = std::env::var_os("SPINE_TEST_PYTEST").map(PathBuf::from)?;
    path.exists().then_some(path)
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

const INTENT: &str = "\
# INT-001: Invoice totals include tax
Owner: alice@example.com · Template: intent@2 · Constitution: v1

## Goal (2–3 sentences)
Invoice totals omit tax, so the printed total disagrees with the ledger.
Compute tax at the line level and include it in the total.

## Non-goals (mandatory, minimum 2)
- Changing the tax rate table.
- Reworking the invoice PDF layout.

## Acceptance criteria (maximum 6 — more means split the task)
AC-1: A single-line invoice at a non-zero rate includes tax in its total.
AC-2: A zero-rate invoice's total equals its subtotal.

## Touchpoints (expected blast radius)
Expected to change: src/
Must NOT change: auth/

## Open questions (optional — must be empty before implementation)
";

const TESTS: &str = "\
# @verifies INT-001/AC-1
# @verifies INT-001/AC-2
from src.billing import total


def test_AC1_includes_tax():
    assert total([{\"amount\": 100, \"rate\": 0.1}]) == 110


def test_AC2_zero_rate_is_subtotal():
    assert total([{\"amount\": 100, \"rate\": 0.0}]) == 100
";

/// Trunk's implementation: sums amounts and ignores the rate.
const BASE_BILLING: &str = "def total(lines):\n    return sum(l[\"amount\"] for l in lines)\n";

struct Repo(PathBuf);

impl Repo {
    fn new(name: &str) -> Option<Self> {
        let pytest = pytest()?;
        let dir = std::env::temp_dir().join(format!("spine-approve-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("bin")).ok()?;
        std::os::unix::fs::symlink(&pytest, dir.join("bin/pytest")).ok()?;

        Command::new("ssh-keygen")
            .args([
                "-q",
                "-t",
                "ed25519",
                "-N",
                "",
                "-C",
                "alice@example.com",
                "-f",
            ])
            .arg(dir.join("key"))
            .status()
            .ok()?
            .success()
            .then_some(())?;

        git(&dir, &["init", "-q", "-b", "main", "."])?;
        git(&dir, &["config", "user.email", "alice@example.com"])?;
        git(&dir, &["config", "user.name", "Alice"])?;
        for (path, body) in [
            ("pyproject.toml", "[project]\n"),
            ("AGENTS.md", "# N\n\nH.\n"),
            (".gitignore", "node_modules/\n"),
            ("src/billing.py", BASE_BILLING),
            ("src/__init__.py", ""),
        ] {
            let full = dir.join(path);
            std::fs::create_dir_all(full.parent().unwrap()).ok()?;
            std::fs::write(full, body).ok()?;
        }
        git(&dir, &["add", "-A"])?;
        git(&dir, &["commit", "-q", "-m", "seed"])?;

        let repo = Repo(dir);
        (repo
            .spine(&["init", "--ci", "generic", "--identity", "alice@example.com"])
            .0
            == 0)
            .then_some(())?;
        git(&repo.0, &["add", "-A"])?;
        git(&repo.0, &["commit", "-q", "-m", "init"])?;
        (repo.spine(&["new"]).0 == 0).then_some(())?;

        for (path, body) in [
            ("intents/INT-001.md", INTENT),
            ("tests/test_billing.py", TESTS),
            ("tests/__init__.py", ""),
        ] {
            let full = repo.0.join(path);
            std::fs::create_dir_all(full.parent().unwrap()).ok()?;
            std::fs::write(full, body).ok()?;
        }
        git(&repo.0, &["add", "-A"])?;
        git(
            &repo.0,
            &["commit", "-q", "-m", "the intent and its red tests"],
        )?;
        Some(repo)
    }

    fn spine(&self, args: &[&str]) -> (i32, String, String) {
        let out = Command::new(spine())
            .current_dir(&self.0)
            .env("SPINE_SIGNING_KEY", self.0.join("key"))
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.0.join("bin").display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .args(args)
            .output()
            .expect("spine runs");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    /// On a pty, for PB §7.1's TTY-only rule.
    fn sign(&self, args: &[&str]) -> (i32, String) {
        let mut command = Command::new("script");
        command
            .current_dir(&self.0)
            .env("SPINE_SIGNING_KEY", self.0.join("key"))
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.0.join("bin").display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            );
        if cfg!(target_os = "macos") {
            command.args(["-q", "/dev/null"]).arg(spine()).args(args);
        } else {
            let line = format!("{} {}", spine().display(), args.join(" "));
            command.args(["-q", "-e", "-c", &line, "/dev/null"]);
        }
        let out = command.output().expect("script runs");
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        (out.status.code().unwrap_or(-1), text)
    }

    fn message(&self) -> String {
        git(&self.0, &["log", "-1", "--format=%B"]).unwrap_or_default()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The whole record, and the number that is the point of it.
#[test]
fn an_approval_freezes_the_closure_and_measures_red_against_base() {
    let Some(repo) = Repo::new("record") else {
        return;
    };
    assert_eq!(repo.sign(&["new", "--sign", "INT-001"]).0, 0);

    let (code, out) = repo.sign(&["check", "--approve", "INT-001"]);
    assert_eq!(code, 0, "{out}");

    let message = repo.message();
    let approve = message
        .lines()
        .find(|l| l.starts_with("Spine-Approve: "))
        .expect("a Spine-Approve line");

    // **`red=1/2` is the only correct answer.** Trunk's `total()` ignores the
    // rate, so AC-1 (which expects tax) fails against the base-restored tree
    // and AC-2 (zero rate) passes. A measurement that ran against the
    // candidate's own code — the thing PB §6.3's base restoration exists to
    // prevent — would report `0/2`.
    assert!(approve.contains("red=1/2"), "{approve}");
    assert!(approve.contains("rounds=1 total_rounds=1"), "{approve}");
    assert!(approve.contains("reopens=0"), "{approve}");
    assert!(approve.contains("freeze=sha256:"), "{approve}");
    assert!(approve.ends_with("signer=alice@example.com"), "{approve}");

    // PB §4.3: "`Spine-Frozen` lines are `<blob id> <path>`".
    let frozen: Vec<&str> = message
        .lines()
        .filter_map(|l| l.strip_prefix("Spine-Frozen: "))
        .collect();
    assert!(
        frozen.iter().any(|l| l.ends_with(" tests/test_billing.py")),
        "the seeded test is frozen: {frozen:?}"
    );
    for line in &frozen {
        let (blob, _path) = line.split_once(' ').expect("<blob> <path>");
        assert_eq!(blob.len(), 40, "a full blob id, never abbreviated: {line}");
    }

    // "`Spine-Test` ids are collected *function* ids without parametrization
    // suffixes, qualified by the runner that collected them."
    let tests: Vec<&str> = message
        .lines()
        .filter_map(|l| l.strip_prefix("Spine-Test: "))
        .collect();
    assert_eq!(tests.len(), 2, "{tests:?}");
    for line in &tests {
        assert!(line.starts_with("pytest "), "qualified by runner: {line}");
        assert!(!line.contains('['), "no parametrization suffix: {line}");
    }

    // "Why empty: the state *is* the commit; nothing in the tree changes."
    assert!(
        git(&repo.0, &["diff", "--name-only", "HEAD~1", "HEAD"])
            .unwrap()
            .is_empty()
    );
}

/// PB §6.3: "`k = 0` is a wire at **approval** (a human signs with a reason)",
/// and PB §11 makes `reason=` mandatory there. The flag was added to the
/// signature on 2026-08-30; without it the approval refuses, and with it the
/// reason reaches the line as a JSON string literal (PB §7.2).
#[test]
fn a_green_measurement_needs_a_reason_and_records_it() {
    let Some(repo) = Repo::new("green") else {
        return;
    };
    // Make the tests pass against trunk's own code: now nothing is red.
    std::fs::write(
        repo.0.join("tests/test_billing.py"),
        "# @verifies INT-001/AC-1\n# @verifies INT-001/AC-2\n\
         from src.billing import total\n\n\n\
         def test_AC1_includes_tax():\n    assert total([{\"amount\": 100}]) == 100\n\n\n\
         def test_AC2_zero_rate_is_subtotal():\n    assert total([{\"amount\": 5}]) == 5\n",
    )
    .unwrap();
    git(&repo.0, &["add", "-A"]).unwrap();
    git(&repo.0, &["commit", "-q", "-m", "green tests"]).unwrap();
    assert_eq!(repo.sign(&["new", "--sign", "INT-001"]).0, 0);

    let (code, out) = repo.sign(&["check", "--approve", "INT-001"]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("red=0/"), "{out}");
    assert!(out.contains("--reason"), "{out}");
    assert!(!repo.message().contains("Spine-Approve:"));

    // With one, it lands — and the reason is on the line G13 reads.
    let (code, out) = repo.sign(&[
        "check",
        "--approve",
        "INT-001",
        "--reason",
        "the suite already covered this on trunk",
    ]);
    assert_eq!(code, 0, "{out}");
    let message = repo.message();
    let line = message
        .lines()
        .find(|l| l.starts_with("Spine-Approve: "))
        .expect("a Spine-Approve line");
    assert!(line.contains("red=0/2"), "{line}");
    assert!(
        line.contains(r#"reason="the suite already covered this on trunk""#),
        "{line}"
    );
    // PB §11's field order: `reason=` before `signer=`, and `signer=` last.
    assert!(line.ends_with("signer=alice@example.com"), "{line}");
}

/// The flag is optional, and an approval that needs no reason does not carry
/// one: G13 requires it in three cases and ignores it otherwise, so a line that
/// always had one would say nothing.
#[test]
fn a_red_approval_carries_no_reason_unless_given() {
    let Some(repo) = Repo::new("no-reason") else {
        return;
    };
    assert_eq!(repo.sign(&["new", "--sign", "INT-001"]).0, 0);
    let (code, out) = repo.sign(&["check", "--approve", "INT-001"]);
    assert_eq!(code, 0, "{out}");
    let message = repo.message();
    let line = message
        .lines()
        .find(|l| l.starts_with("Spine-Approve: "))
        .unwrap();
    assert!(!line.contains("reason="), "{line}");
}

/// "which refuses a dirty worktree" — the approval freezes the branch HEAD's
/// **tree**, and an uncommitted edit anywhere in it is a byte the record would
/// claim to have frozen and did not.
#[test]
fn a_dirty_worktree_refuses() {
    let Some(repo) = Repo::new("dirty") else {
        return;
    };
    assert_eq!(repo.sign(&["new", "--sign", "INT-001"]).0, 0);
    std::fs::write(repo.0.join("src/billing.py"), "# edited\n").unwrap();

    let (code, out) = repo.sign(&["check", "--approve", "INT-001"]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("not clean"), "{out}");
    assert!(!repo.message().contains("Spine-Approve:"));
}

/// PB §4.3 clause 1: the seed is "every file under a `C-T1` test root …
/// carrying a pragma naming an acceptance criterion of this intent". With none,
/// there is nothing to freeze and the approval would assert an empty closure.
#[test]
fn an_empty_closure_refuses() {
    let Some(repo) = Repo::new("empty-closure") else {
        return;
    };
    // Strip the pragmas: the file is still a test, and seeds nothing.
    let text = std::fs::read_to_string(repo.0.join("tests/test_billing.py")).unwrap();
    let stripped: String = text
        .lines()
        .filter(|l| !l.contains("@verifies"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(repo.0.join("tests/test_billing.py"), stripped).unwrap();
    git(&repo.0, &["add", "-A"]).unwrap();
    git(&repo.0, &["commit", "-q", "-m", "no pragmas"]).unwrap();
    assert_eq!(repo.sign(&["new", "--sign", "INT-001"]).0, 0);

    let (code, out) = repo.sign(&["check", "--approve", "INT-001"]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("closure is empty"), "{out}");
}
