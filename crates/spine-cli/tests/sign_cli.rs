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
