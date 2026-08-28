//! The git plumbing the derivation reads, and nothing more.
//!
//! Shelled out to `git`, as `spine_init::git` does, and for the same reason:
//! the objects are git's and every rule this crate implements is stated in
//! terms of git's own output (`rev-list --first-parent`, `cat-file commit`,
//! `diff --name-only`, `patch-id --stable`).
//!
//! Three properties are load-bearing here and are not in `spine-init`'s copy:
//!
//! - **Bytes, not strings.** A path in a tree is a byte string — DM §12.1's own
//!   worked example carries `src/billing/caf` + `0xE9` + `.py`, "a Latin-1 `é`
//!   … which no amount of normalization can make into text". Every method that
//!   returns a path returns `Vec<u8>`, and every listing uses git's `-z` form
//!   so that git's C-quoting never enters the picture.
//! - **No working tree.** DM §8.7: *"A dump is a function of trees and refs.
//!   Running `--dump` in a bare repository, with a dirty working tree, with a
//!   stale index, or with untracked files present produces identical bytes."*
//!   Nothing here reads the index, `git status`, or a file on disk. [`Repo`] is
//!   addressed by a directory only so that `git -C` has somewhere to run.
//! - **Options pinned by the release, never read from config.** DM §10 rule 12:
//!   *"Every invocation the derivation makes — `diff`, `rev-list`,
//!   `merge-tree`, `patch-id`, `ls-tree` — runs with its diff algorithm, rename
//!   and copy detection, and every other output-affecting option fixed by the
//!   release and never read from repository, user or system config … A
//!   repository that sets `diff.algorithm` must not thereby change its own
//!   dump."* [`PINNED`] is that fixing, applied to every invocation.

use spine_canon::ObjectFormat;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The config DM §10 rule 12 requires every invocation to fix.
///
/// **DERIVED in its membership, not in its existence.** DM §10 rule 12 says
/// *"The exact option set is the release's and the indexer spec's (§16); the
/// rule that it is pinned is this document's."* No indexer spec exists yet, so
/// these three are this release's:
///
/// - `diff.algorithm=myers` — git's own default, so pinning it changes nothing
///   for a repository that sets none and everything for one that does;
/// - `diff.renames=false` — a `modifies` edge names a path that changed, and
///   rename detection is a similarity heuristic whose result moves with git's
///   version, which is exactly the class DM §8.5 excluded `introduced_by` for;
/// - `core.quotePath=false` — belt and braces beside the `-z` forms below,
///   since a quoted path would reach `esc` already escaped once.
const PINNED: [&str; 6] = [
    "-c",
    "diff.algorithm=myers",
    "-c",
    "diff.renames=false",
    "-c",
    "core.quotePath=false",
];

#[derive(Debug)]
pub enum GitError {
    /// `git` is not on PATH, or could not be spawned.
    NotAvailable(String),
    /// The command ran and failed; the stderr is carried for the diagnostic.
    Failed { argv: String, stderr: String },
    /// The command succeeded and its output was not what it must be.
    Unexpected(String),
}

impl core::fmt::Display for GitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GitError::NotAvailable(e) => write!(f, "git is not available: {e}"),
            GitError::Failed { argv, stderr } => write!(f, "git {argv} failed: {}", stderr.trim()),
            GitError::Unexpected(what) => write!(f, "unexpected git output: {what}"),
        }
    }
}

impl core::error::Error for GitError {}

pub type Result<T> = core::result::Result<T, GitError>;

/// One entry of a tree listing (`git ls-tree -z`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub mode: String,
    pub kind: String,
    pub oid: String,
    /// The tree's bytes, verbatim. `-z` means git never quotes them.
    pub path: Vec<u8>,
}

/// A repository, addressed by a directory `git -C` can run in.
///
/// Bare or checked out — DM §8.7 requires the two to dump identically, so
/// nothing here distinguishes them.
#[derive(Debug, Clone)]
pub struct Repo {
    dir: PathBuf,
    object_format: ObjectFormat,
}

impl Repo {
    /// Open the repository containing (or being) `dir`.
    ///
    /// `extensions.objectFormat` is one of the two config values DM §4.3 allows
    /// a dump to depend on; the other is `spine.trustRoot` ([`Repo::config`]).
    /// Everything else is pinned or ignored.
    pub fn open(dir: &Path) -> Result<Self> {
        let mut repo = Repo {
            dir: dir.to_path_buf(),
            object_format: ObjectFormat::Sha1,
        };
        // Proves the directory is a repository, and fails loudly if it is not.
        repo.run_bytes(&["rev-parse", "--git-dir"])?;
        // Absent in a sha1 repository, which is git's default and the fallback.
        if let Some(value) = repo.config("extensions.objectFormat") {
            repo.object_format = ObjectFormat::parse(value.trim()).ok_or_else(|| {
                GitError::Unexpected(format!("extensions.objectFormat {value:?}"))
            })?;
        }
        Ok(repo)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn object_format(&self) -> ObjectFormat {
        self.object_format
    }

    /// A config value, or `None` when it is unset.
    ///
    /// Only two keys may be read (DM §4.3) and this method is how both are.
    pub fn config(&self, key: &str) -> Option<String> {
        let out = self.run_bytes(&["config", "--get", key]).ok()?;
        let value = String::from_utf8_lossy(&out).trim_end().to_string();
        (!value.is_empty()).then_some(value)
    }

    /// The full oid `rev` resolves to, or `None`.
    pub fn rev_parse(&self, rev: &str) -> Option<String> {
        let out = self
            .run_bytes(&[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("{rev}^{{commit}}"),
            ])
            .ok()?;
        let oid = String::from_utf8_lossy(&out).trim_end().to_string();
        (!oid.is_empty()).then_some(oid)
    }

    /// `git rev-list --first-parent <tip>` — newest first, to the root.
    ///
    /// PB §6.3 G9: *"First-parent walk of trunk"*. Every rule this crate
    /// derives about trunk is a rule about this list.
    pub fn first_parent(&self, tip: &str) -> Result<Vec<String>> {
        let out = self.run_bytes(&["rev-list", "--first-parent", tip])?;
        Ok(String::from_utf8_lossy(&out)
            .lines()
            .map(str::to_string)
            .collect())
    }

    /// `M(L) = git rev-list B..L` (PB §5.5), *including* `L` — the caller drops
    /// it where the landing is not its own member.
    pub fn rev_list_range(&self, base: &str, head: &str) -> Result<Vec<String>> {
        let out = self.run_bytes(&["rev-list", &format!("{base}..{head}")])?;
        Ok(String::from_utf8_lossy(&out)
            .lines()
            .map(str::to_string)
            .collect())
    }

    /// The commit's parents, in order.
    pub fn parents(&self, sha: &str) -> Result<Vec<String>> {
        let out = self.run_bytes(&["rev-list", "--parents", "-n", "1", sha])?;
        let line = String::from_utf8_lossy(&out);
        Ok(line
            .split_whitespace()
            .skip(1)
            .map(str::to_string)
            .collect())
    }

    /// The commit message's bytes, exactly as the object carries them.
    ///
    /// PB §5.5: *"the indexer reads messages with `git cat-file commit`, never
    /// `git log`, so no cleanup rule ever touches the fenced bytes."* The
    /// header block ends at the first empty line; everything after it is the
    /// message, including a trailing LF.
    pub fn commit_message(&self, sha: &str) -> Result<Vec<u8>> {
        let object = self.run_bytes(&["cat-file", "commit", sha])?;
        // A commit header may carry a multi-line `gpgsig`, whose continuation
        // lines begin with a space — so the header ends at the first `\n\n` and
        // never at the first line that "looks like" a blank.
        let at = object
            .windows(2)
            .position(|w| w == b"\n\n")
            .ok_or_else(|| GitError::Unexpected(format!("commit {sha} has no message")))?;
        Ok(object[at + 2..].to_vec())
    }

    /// The bytes of a blob at `<sha>:<path>`, or `None` when the tree has no
    /// such path.
    ///
    /// `path` is ASCII by policy: the only blobs this crate reads by path are
    /// `.spine/manifest.json`, `.spine/allowed_signers` and the constitution,
    /// whose names spine itself writes. Arbitrary tree paths are read through
    /// [`Repo::ls_tree`] and then by oid, so a non-UTF-8 path never has to
    /// survive a round trip through `argv`.
    pub fn blob_at(&self, sha: &str, path: &str) -> Option<Vec<u8>> {
        self.run_bytes(&["cat-file", "blob", &format!("{sha}:{path}")])
            .ok()
    }

    /// The bytes of a blob by oid.
    pub fn blob(&self, oid: &str) -> Result<Vec<u8>> {
        self.run_bytes(&["cat-file", "blob", oid])
    }

    /// `git ls-tree -z <sha> -- <prefix>` — one level, raw bytes.
    ///
    /// `-z` is what makes the paths raw: with it git emits the tree's bytes and
    /// never its C-quoted rendering, so `core.quotePath` cannot reach a node id.
    pub fn ls_tree(&self, sha: &str, prefix: &str) -> Result<Vec<TreeEntry>> {
        let out = self.run_bytes(&["ls-tree", "-z", sha, "--", prefix])?;
        let mut entries = Vec::new();
        for record in out.split(|&b| b == 0) {
            if record.is_empty() {
                continue;
            }
            // `<mode> SP <type> SP <oid> TAB <path>`
            let tab = record
                .iter()
                .position(|&b| b == b'\t')
                .ok_or_else(|| GitError::Unexpected("ls-tree record has no TAB".into()))?;
            let meta = String::from_utf8_lossy(&record[..tab]);
            let mut fields = meta.split_whitespace();
            let (Some(mode), Some(kind), Some(oid)) = (fields.next(), fields.next(), fields.next())
            else {
                return Err(GitError::Unexpected(format!("ls-tree record {meta:?}")));
            };
            entries.push(TreeEntry {
                mode: mode.to_string(),
                kind: kind.to_string(),
                oid: oid.to_string(),
                path: record[tab + 1..].to_vec(),
            });
        }
        Ok(entries)
    }

    /// `git diff --name-only -z --no-renames <a> <b>` — the integrated delta
    /// PB §6.2 derives `modifies` from, as raw path bytes.
    pub fn diff_names(&self, a: &str, b: &str) -> Result<Vec<Vec<u8>>> {
        let out = self.run_bytes(&["diff", "--name-only", "-z", a, b])?;
        Ok(out
            .split(|&b| b == 0)
            .filter(|p| !p.is_empty())
            .map(<[u8]>::to_vec)
            .collect())
    }

    /// `git diff <a> <b> [-- <paths>] | git patch-id --stable`, or `None` when
    /// the diff is empty (`patch-id` then prints nothing).
    ///
    /// PB §6.2 derives `reverts` from exactly this pipeline. `--stable` is the
    /// half of it that is a fact about the patch rather than about git's
    /// internal ordering.
    pub fn patch_id(&self, a: &str, b: &str, paths: &[Vec<u8>]) -> Result<Option<String>> {
        let mut args: Vec<String> = PINNED.iter().map(|s| (*s).to_string()).collect();
        args.extend(["diff".into(), a.into(), b.into()]);
        if !paths.is_empty() {
            args.push("--".into());
            for path in paths {
                // Only paths that survive as text can be passed in `argv`; a
                // path that does not is dropped from the restriction rather
                // than silently widening it, and the caller is told by the
                // count. See `derive::reverts`.
                args.push(String::from_utf8_lossy(path).into_owned());
            }
        }
        let diff = Command::new("git")
            .current_dir(&self.dir)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| GitError::NotAvailable(e.to_string()))?;
        if !diff.status.success() {
            return Err(GitError::Failed {
                argv: args.join(" "),
                stderr: String::from_utf8_lossy(&diff.stderr).into_owned(),
            });
        }
        let out = self.run_stdin(&["patch-id", "--stable"], &diff.stdout)?;
        Ok(String::from_utf8_lossy(&out)
            .split_whitespace()
            .next()
            .map(str::to_string))
    }

    /// Run a git command, returning its stdout bytes.
    pub fn run_bytes(&self, args: &[&str]) -> Result<Vec<u8>> {
        let out = Command::new("git")
            .current_dir(&self.dir)
            .args(PINNED)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| GitError::NotAvailable(e.to_string()))?;
        if !out.status.success() {
            return Err(GitError::Failed {
                argv: args.join(" "),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            });
        }
        Ok(out.stdout)
    }

    fn run_stdin(&self, args: &[&str], stdin: &[u8]) -> Result<Vec<u8>> {
        use std::io::Write;
        let mut child = Command::new("git")
            .current_dir(&self.dir)
            .args(PINNED)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| GitError::NotAvailable(e.to_string()))?;
        child
            .stdin
            .as_mut()
            .expect("piped")
            .write_all(stdin)
            .map_err(|e| GitError::NotAvailable(e.to_string()))?;
        let out = child
            .wait_with_output()
            .map_err(|e| GitError::NotAvailable(e.to_string()))?;
        if !out.status.success() {
            return Err(GitError::Failed {
                argv: args.join(" "),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            });
        }
        Ok(out.stdout)
    }
}

/// Whether `git` can be spawned at all.
///
/// Tests that need a repository return early without it rather than fail: the
/// house pattern in `spine_init::git`'s own tests.
pub fn available() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
