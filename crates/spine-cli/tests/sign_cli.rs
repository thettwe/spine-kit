//! `spine new --sign` against a live repository, with a **real key**.
//!
//! PB §3.4's one mandatory human gate, end to end: the intent blob is read from
//! HEAD, ID §8.1's Layer-2 preconditions are checked, the `Spine-Signoff` line
//! is rendered and signed under `spine-signoff@v1`, and the transition is a
//! signed, empty commit.
//!
//! The signature is verified by **`ssh-keygen -Y verify` directly**, not by
//! spine's own verifier: the two agreeing is the property that matters, and a
//! test that only asked spine would pass on a signature nothing else accepts.

use std::path::{Path, PathBuf};
use std::process::Command;

fn spine() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_spine"))
}

fn tools() -> bool {
    ["git", "ssh-keygen"]
        .iter()
        .all(|t| Command::new(t).arg("--version").output().is_ok())
        // `ssh-keygen --version` exits non-zero but runs; `-Q` is the probe.
        && Command::new("ssh-keygen").arg("-Q").output().is_ok()
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
Expected to change: src/billing/
Must NOT change: auth/

## Open questions (optional — must be empty before implementation)
";

struct Repo(PathBuf);

impl Repo {
    /// An initialised repository standing on a filled-in intent branch.
    ///
    /// Returns `None` where the tools are absent or the binary carries no
    /// release — `spine init` refuses every row in a development build, which
    /// is `ci.md` §3.4's specified behaviour and not a gap.
    fn new(name: &str) -> Option<Self> {
        if !tools() {
            return None;
        }
        let dir = std::env::temp_dir().join(format!("spine-sign-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok()?;

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
        std::fs::write(dir.join("pyproject.toml"), "[project]\n").ok()?;
        std::fs::write(dir.join("AGENTS.md"), "# N\n\nH.\n").ok()?;
        std::fs::write(dir.join(".gitignore"), "node_modules/\n").ok()?;
        git(&dir, &["add", "-A"])?;
        git(&dir, &["commit", "-q", "-m", "seed"])?;

        let repo = Repo(dir);
        repo.spine(&["init", "--ci", "generic", "--identity", "alice@example.com"])
            .0
            .eq(&0)
            .then_some(())?;
        git(&repo.0, &["add", "-A"])?;
        git(&repo.0, &["commit", "-q", "-m", "init"])?;

        let (code, id, _) = repo.spine(&["new"]);
        (code == 0).then_some(())?;
        let id = id.trim().to_string();
        std::fs::create_dir_all(repo.0.join("intents")).ok()?;
        std::fs::write(repo.0.join(format!("intents/{id}.md")), INTENT).ok()?;
        git(&repo.0, &["add", "-A"])?;
        git(&repo.0, &["commit", "-q", "-m", "the intent"])?;
        Some(repo)
    }

    fn spine(&self, args: &[&str]) -> (i32, String, String) {
        let out = Command::new(spine())
            .current_dir(&self.0)
            .env("SPINE_SIGNING_KEY", self.0.join("key"))
            .args(args)
            .output()
            .expect("spine runs");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    /// The same, on a pty, so PB §7.1's TTY-only rule is satisfied. `script`
    /// is the portable way to get one without a pty crate.
    fn sign(&self, args: &[&str]) -> (i32, String) {
        let mut command = Command::new("script");
        command
            .current_dir(&self.0)
            .env("SPINE_SIGNING_KEY", self.0.join("key"));
        // BSD `script -q /dev/null cmd…`; GNU wants `-c`.
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

/// The whole gate: a signed, empty commit whose `Spine-Signoff` names HEAD's
/// intent blob and whose signature `ssh-keygen` accepts.
#[test]
fn a_sign_off_is_a_signed_empty_commit_that_ssh_keygen_verifies() {
    let Some(repo) = Repo::new("roundtrip") else {
        return;
    };

    let head_before = git(&repo.0, &["rev-parse", "HEAD"]).unwrap();
    let (code, text) = repo.sign(&["new", "--sign", "INT-001"]);
    assert_eq!(code, 0, "{text}");

    let message = repo.message();
    let line = message
        .lines()
        .find(|l| l.starts_with("Spine-Signoff: "))
        .expect("a Spine-Signoff line");
    let signature = message
        .lines()
        .find_map(|l| l.strip_prefix("Spine-Signoff-Sig: "))
        .expect("a -Sig line");

    // PB §11: "`blob=` equals the intent blob at head".
    let blob = git(
        &repo.0,
        &["rev-parse", &format!("{head_before}:intents/INT-001.md")],
    )
    .unwrap();
    assert!(line.contains(&format!("blob={blob}")), "{line}");
    // "`reopens=` equals the branch's reopen count", which is zero here.
    assert!(line.contains("reopens=0"), "{line}");
    assert!(line.contains("template=intent@2"), "{line}");
    assert!(line.ends_with("signer=alice@example.com"), "{line}");

    // "a signed, **empty** commit": the statement is the transition, and
    // signing must not move the blob it names.
    let changed = git(&repo.0, &["diff", "--name-only", "HEAD~1", "HEAD"]).unwrap();
    assert!(changed.is_empty(), "the sign-off changed {changed:?}");
    assert_eq!(
        git(&repo.0, &["rev-parse", "HEAD:intents/INT-001.md"]).unwrap(),
        blob
    );

    // **Verified by `ssh-keygen` itself**, over the exact bytes EV §2.7 fixes.
    let armored = {
        let mut out = String::from("-----BEGIN SSH SIGNATURE-----\n");
        for chunk in signature.as_bytes().chunks(70) {
            out.push_str(&String::from_utf8_lossy(chunk));
            out.push('\n');
        }
        out.push_str("-----END SSH SIGNATURE-----\n");
        out
    };
    std::fs::write(repo.0.join("sig.asc"), &armored).unwrap();
    let public = std::fs::read_to_string(repo.0.join("key.pub")).unwrap();
    std::fs::write(
        repo.0.join("allowed"),
        format!(
            "alice@example.com namespaces=\"spine-signoff@v1\" {}",
            public.trim()
        ),
    )
    .unwrap();

    let mut verify = Command::new("ssh-keygen")
        .current_dir(&repo.0)
        .args([
            "-Y",
            "verify",
            "-f",
            "allowed",
            "-I",
            "alice@example.com",
            "-n",
            "spine-signoff@v1",
            "-s",
            "sig.asc",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("ssh-keygen runs");
    {
        use std::io::Write;
        verify
            .stdin
            .take()
            .unwrap()
            .write_all(line.as_bytes())
            .unwrap();
    }
    let out = verify.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "ssh-keygen rejected spine's own signature: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// ID §2.4: "The worktree file at `intents/<ID>.md`, if it exists, must hash …
/// to the head blob's id. A dirty intent path is refused (`worktree-dirty`);
/// signing bytes a human is not looking at is the failure this closes."
#[test]
fn a_dirty_intent_path_refuses() {
    let Some(repo) = Repo::new("dirty") else {
        return;
    };
    let path = repo.0.join("intents/INT-001.md");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("\n<!-- an edit the signer would not see -->\n");
    std::fs::write(&path, text).unwrap();

    let (code, out) = repo.sign(&["new", "--sign", "INT-001"]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("worktree-dirty"), "{out}");
    assert!(
        !repo.message().contains("Spine-Signoff:"),
        "a refused sign-off writes no commit"
    );
}

/// PB §7.1's TTY rule, on the command that most needs it: without a terminal
/// there is nothing to confirm a key touch on.
#[test]
fn signing_without_a_terminal_refuses() {
    let Some(repo) = Repo::new("no-tty") else {
        return;
    };
    let (code, _, stderr) = repo.spine(&["new", "--sign", "INT-001"]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("TTY-only"), "{stderr}");
    assert!(!repo.message().contains("Spine-Signoff:"));
}

/// PB §4.3: "**A reopen must change the blob — a no-op reopen is refused.**"
/// And it is the commit that carries the edit, not an empty one: "the commit
/// that changes the intent blob carries a signed `Spine-Reopen` line".
#[test]
fn a_reopen_must_change_the_blob_and_carries_the_edit() {
    let Some(repo) = Repo::new("reopen") else {
        return;
    };
    assert_eq!(repo.sign(&["new", "--sign", "INT-001"]).0, 0);

    // Nothing edited: refused, and nothing committed.
    let head = git(&repo.0, &["rev-parse", "HEAD"]).unwrap();
    let (code, out) = repo.sign(&["new", "--reopen", "INT-001", "--reason", "why"]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("no-op reopen is refused"), "{out}");
    assert_eq!(git(&repo.0, &["rev-parse", "HEAD"]).unwrap(), head);

    // A real edit: the reopen commits it alongside the statement.
    let path = repo.0.join("intents/INT-001.md");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("AC-2: A zero-rate", "AC-2: A zero rate");
    std::fs::write(&path, text).unwrap();

    let (code, out) = repo.sign(&[
        "new",
        "--reopen",
        "INT-001",
        "--reason",
        "AC-2 was untestable",
    ]);
    assert_eq!(code, 0, "{out}");
    let message = repo.message();
    let line = message
        .lines()
        .find(|l| l.starts_with("Spine-Reopen: "))
        .expect("a Spine-Reopen line");
    // "`voids=` names the binding approval's freeze, `none` only when no
    // approval exists" — and none does here.
    assert!(line.contains("voids=none"), "{line}");
    assert!(line.contains("reopens=1"), "{line}");
    assert!(line.contains(r#"reason="AC-2 was untestable""#), "{line}");
    assert_eq!(
        git(&repo.0, &["diff", "--name-only", "HEAD~1", "HEAD"]).unwrap(),
        "intents/INT-001.md",
        "the reopen is the commit that changes the blob"
    );
}

/// PB §7.2: "`reopens=` is the count of signed reopens on the branch at
/// signing, **so a sign-off cannot be replayed after a reopen**."
#[test]
fn a_sign_off_after_a_reopen_carries_the_new_count() {
    let Some(repo) = Repo::new("reopens-count") else {
        return;
    };
    assert_eq!(repo.sign(&["new", "--sign", "INT-001"]).0, 0);
    assert!(repo.message().contains("reopens=0"));

    let path = repo.0.join("intents/INT-001.md");
    for (n, from) in [(1, "AC-2: A zero-rate"), (2, "AC-1: A single-line")] {
        let text = std::fs::read_to_string(&path)
            .unwrap()
            .replace(from, &format!("{from} invoice"));
        std::fs::write(&path, text).unwrap();
        let (code, out) = repo.sign(&["new", "--reopen", "INT-001", "--reason", "narrowed"]);
        assert_eq!(code, 0, "{out}");
        assert!(
            repo.message().contains(&format!("reopens={n}")),
            "{}",
            repo.message()
        );
    }

    let (code, out) = repo.sign(&["new", "--sign", "INT-001"]);
    assert_eq!(code, 0, "{out}");
    let message = repo.message();
    let line = message
        .lines()
        .find(|l| l.starts_with("Spine-Signoff: "))
        .unwrap();
    assert!(line.contains("reopens=2"), "{line}");
}

/// PB §11's `Spine-Withdraw` payload: `INT-042 blob=<oid> reason="…"
/// signer=<p>`, on a commit that changes nothing — the tombstone it lands has
/// "tree identical to `B`'s".
#[test]
fn a_withdrawal_names_the_blob_and_changes_nothing() {
    let Some(repo) = Repo::new("withdraw") else {
        return;
    };
    assert_eq!(repo.sign(&["new", "--sign", "INT-001"]).0, 0);
    let blob = git(&repo.0, &["rev-parse", "HEAD:intents/INT-001.md"]).unwrap();

    let (code, out) = repo.sign(&["new", "--withdraw", "INT-001", "--reason", "superseded"]);
    assert_eq!(code, 0, "{out}");
    let message = repo.message();
    let line = message
        .lines()
        .find(|l| l.starts_with("Spine-Withdraw: "))
        .expect("a Spine-Withdraw line");
    assert!(line.contains(&format!("blob={blob}")), "{line}");
    assert!(line.contains(r#"reason="superseded""#), "{line}");
    assert!(
        git(&repo.0, &["diff", "--name-only", "HEAD~1", "HEAD"])
            .unwrap()
            .is_empty()
    );
}

/// PB §7.2: "`reason=` values are JSON string literals." A quote or a
/// backslash in the reason must not end the field or the line.
#[test]
fn a_reason_is_a_json_string_literal() {
    let Some(repo) = Repo::new("reason-json") else {
        return;
    };
    assert_eq!(repo.sign(&["new", "--sign", "INT-001"]).0, 0);

    let awkward = r#"it said "no" \ and stopped"#;
    let (code, out) = repo.sign(&["new", "--withdraw", "INT-001", "--reason", awkward]);
    assert_eq!(code, 0, "{out}");
    let message = repo.message();
    let line = message
        .lines()
        .find(|l| l.starts_with("Spine-Withdraw: "))
        .unwrap();
    assert!(
        line.contains(r#"reason="it said \"no\" \\ and stopped""#),
        "{line}"
    );
    // And the statement is still one line, which is what PB §7.2's shape needs.
    assert_eq!(
        message
            .lines()
            .filter(|l| l.starts_with("Spine-Withdraw: "))
            .count(),
        1
    );
}
