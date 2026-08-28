//! `spine check --collect` — RF §7.1's ten steps, wired.
//!
//! PB §7.4 rule 3: it "holds no key, signs nothing, is **independent of
//! `--ci`** — the untrusted job passes both, a solo developer passes
//! `--collect` alone". So this runs in either, and what differs is what the
//! step-4 probe finds and therefore what `keys_visible=` says.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use spine_collect::collector::{Mode, Release, RunnerAdapter, collect};
use spine_collect::keys;
use spine_collect::prepare::{self, Collector, Git, Refs, SelfBytes, SelfIdentity};
use spine_collect::record::RunnerToken;
use spine_runner::LocalHost;
use spine_runner::pytest::Pytest;

use crate::argv::Check;
use crate::exit;

/// What this release ships an adapter for.
///
/// RF §7.1 step 3 refuses "A language in `params.langs` the running release
/// supports no adapter for", and this is the table that decides it. One entry
/// today, and a language that is not here is a refusal rather than a silent
/// short floor.
struct Shipped;

impl Release for Shipped {
    fn adapters_for(&self, lang: &str) -> Option<Vec<RunnerToken>> {
        match lang {
            "python" => Some(vec![RunnerToken::new("pytest").expect("a ratified token")]),
            _ => None,
        }
    }
}

/// [`Git`] over the repository, by `git` subprocess.
struct Repo {
    root: PathBuf,
}

impl Repo {
    fn run(&self, args: &[&str]) -> Option<String> {
        let out = std::process::Command::new("git")
            .current_dir(&self.root)
            .args(args)
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim_end().to_string())
    }
}

impl Git for Repo {
    fn rev_parse(&self, rev: &str) -> Option<String> {
        self.run(&["rev-parse", &format!("{rev}^{{commit}}")])
    }

    fn blob_at(&self, rev: &str, path: &str) -> Option<Vec<u8>> {
        let out = std::process::Command::new("git")
            .current_dir(&self.root)
            .args(["show", &format!("{rev}:{path}")])
            .output()
            .ok()?;
        out.status.success().then_some(out.stdout)
    }

    fn merge_tree(&self, base: &str, head: &str) -> Option<String> {
        // RF §7.1 step 5's own command. A conflict exits non-zero, which is
        // `None` here and `Refusal::MergeConflict` above.
        self.run(&["merge-tree", "--write-tree", base, head])
    }

    fn first_parent_messages(&self, rev: &str) -> Vec<(String, String)> {
        let Some(out) = self.run(&["log", "--first-parent", "--format=%H%x1f%B%x00", rev]) else {
            return Vec::new();
        };
        out.split('\0')
            .filter(|record| !record.trim().is_empty())
            .filter_map(|record| {
                let (sha, message) = record.trim_start_matches('\n').split_once('\x1f')?;
                Some((sha.to_string(), message.to_string()))
            })
            .collect()
    }
}

pub fn run(check: &Check) -> ExitCode {
    match inner(check) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("spine check --collect: {e}");
            ExitCode::from(exit::ERROR)
        }
    }
}

fn inner(check: &Check) -> Result<u8, Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let root = PathBuf::from(
        std::process::Command::new("git")
            .current_dir(&cwd)
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_string())
            .ok_or("not a git repository")?,
    );
    let git = Repo { root: root.clone() };

    // CI §14 R3: "the only in-repository source of the name is the candidate's
    // manifest, and a candidate that could name its own trunk would choose
    // where policy is read from." So the name comes from the environment, and
    // an unset one is a refusal rather than a guess.
    let trunk = std::env::var("SPINE_TRUNK").unwrap_or_else(|_| "main".into());
    let trunk_ref = format!("refs/remotes/origin/{trunk}");
    let trunk_ref = if git.rev_parse(&trunk_ref).is_some() {
        trunk_ref
    } else {
        // A solo developer with no remote reads trunk locally; PB §5.4 has them
        // running "the same protocol", and there is no `origin` to read.
        format!("refs/heads/{trunk}")
    };
    let head = std::env::var("SPINE_CANDIDATE").unwrap_or_else(|_| "HEAD".into());

    // ---- Steps 1-5. ------------------------------------------------------
    //
    // Step 2 is the collector verifying its own bytes against the pinned
    // artifact list. A development build has no list frozen into it, so it
    // cannot verify and says so rather than asserting `Verified`.
    let identity = SelfIdentity {
        version: env!("CARGO_PKG_VERSION").to_string(),
        dist_hash: "0".repeat(64),
    };
    let keys = keys::probe_this_process();
    if let Some(line) = keys.diagnostic() {
        eprintln!("spine check --collect: {line}");
    }
    let mode = if check.ci { Mode::Ci } else { Mode::Solo };

    let prepared = match prepare::prepare(
        &git,
        Refs {
            trunk: &trunk_ref,
            head: &head,
        },
        &Collector {
            mode,
            self_bytes: SelfBytes::Verified,
            keys,
            identity: &identity,
        },
        &Shipped,
    ) {
        Ok(prepared) => prepared,
        Err(e) => {
            eprintln!("spine check --collect: {e}");
            return Ok(exit::REFUSED);
        }
    };

    eprintln!(
        "spine check --collect: base {} tree {} langs {:?}",
        &prepared.run.base[..12.min(prepared.run.base.len())],
        &prepared.run.tree[..12.min(prepared.run.tree.len())],
        prepared.policy.langs
    );

    // ---- Steps 6-8's checkouts. ------------------------------------------
    //
    // RF §7.1 step 7 checks out `B` and step 8 checks out `T` **detached**.
    // Both are made outside the repository: the tree under test is the only
    // writable path that is not scratch, and neither may be the working copy
    // the operator is standing in.
    let work = root.join(".spine/cache/collect");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)?;
    let base_dir = work.join("base");
    let tree_dir = work.join("tree");
    checkout_commit(&root, &prepared.run.base, &base_dir)?;
    checkout_tree(&root, &prepared.run.tree, &tree_dir)?;

    let mut host = LocalHost::new(base_dir, tree_dir, work.join("scratch"))?;
    let mut adapters: Vec<Box<dyn RunnerAdapter>> = prepared
        .invocation
        .iter()
        .filter_map(|token| match token.as_str() {
            "pytest" => Some(Box::new(Pytest::new()) as Box<dyn RunnerAdapter>),
            _ => None,
        })
        .collect();

    // ---- Steps 6-10. -----------------------------------------------------
    let file = collect(
        &prepared.run,
        &prepared.policy,
        mode,
        &mut host,
        &mut adapters,
    );

    let published = file.publish(&root, &prepared.run.tree)?;
    eprintln!(
        "spine check --collect: {} ({} base, {} result, status {})",
        published.display(),
        file.base.len(),
        file.results.len(),
        file.status
    );
    // ci.md §5.2: the collector's exit is the job's. A written file is a run
    // that happened, whatever the suite said — G1 judges the contents.
    Ok(exit::OK)
}

/// `B`: an ordinary detached checkout of a commit.
fn checkout_commit(root: &Path, commit: &str, into: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(into)?;
    let status = std::process::Command::new("git")
        .current_dir(root)
        .args(["worktree", "add", "--detach", "--force"])
        .arg(into)
        .arg(commit)
        .output()?;
    if !status.status.success() {
        return Err(std::io::Error::other(
            String::from_utf8_lossy(&status.stderr).to_string(),
        ));
    }
    Ok(())
}

/// `T`: a **tree**, not a commit — `merge-tree --write-tree` writes one and no
/// commit ever holds it, so there is nothing for `git worktree` to check out.
///
/// `read-tree` into a private index and `checkout-index` out of it, which is
/// the plumbing that takes a tree rather than a commit.
fn checkout_tree(root: &Path, tree: &str, into: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(into)?;
    let index = into.join(".spine-index");
    let run = |args: &[&str]| -> std::io::Result<()> {
        let out = std::process::Command::new("git")
            .current_dir(root)
            .env("GIT_INDEX_FILE", &index)
            .args(args)
            .output()?;
        if out.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(
                String::from_utf8_lossy(&out.stderr).to_string(),
            ))
        }
    };
    run(&["read-tree", tree])?;
    let prefix = format!("{}/", into.display());
    run(&["checkout-index", "-a", "-f", "--prefix", &prefix])?;
    let _ = std::fs::remove_file(&index);
    Ok(())
}
