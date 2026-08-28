//! The git operations `spine init` needs, and nothing more.
//!
//! Shelled out to `git` rather than reimplemented. Two reasons, and the second
//! is the load-bearing one:
//!
//! - PB §6.7 records a file's `blob` as `git hash-object --path`, "so
//!   `.gitattributes` and CRLF churn are not drift". The `--path` form runs the
//!   content through *the repository's own* clean filters — which may include a
//!   `filter.*.clean` a team configured. Reimplementing that means
//!   reimplementing git's filter machinery, and getting it subtly wrong means
//!   recording a blob `git ls-tree` disagrees with, which fails G16 for a
//!   reason nobody can see.
//! - `git check-ref-format`, object-format detection and the porcelain status
//!   are all cheap to ask for and expensive to be wrong about.
//!
//! `spine-canon::git_blob_id` remains the right answer wherever filters must
//! *not* apply — a managed region's bytes are already in-blob bytes (MF §3.5).

use crate::plan::TreeSource;
use spine_canon::ObjectFormat;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
            GitError::Failed { argv, stderr } => {
                write!(f, "git {argv} failed: {}", stderr.trim())
            }
            GitError::Unexpected(what) => write!(f, "unexpected git output: {what}"),
        }
    }
}

impl core::error::Error for GitError {}

type Result<T> = core::result::Result<T, GitError>;

/// A repository, addressed by its working-tree root.
#[derive(Debug, Clone)]
pub struct Repo {
    root: PathBuf,
    object_format: ObjectFormat,
}

/// A `git ls-tree` entry: the file mode and the object id, both of which the
/// rollback restoration rule compares (MF §6.7 step 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    /// The six-digit octal mode, as `git ls-tree` prints it — `100644`,
    /// `100755`, `120000`.
    pub mode: String,
    pub oid: String,
}

impl Repo {
    /// Discover the repository containing `start`.
    pub fn discover(start: &Path) -> Result<Self> {
        let root = run(start, &["rev-parse", "--show-toplevel"])?;
        let root = PathBuf::from(root.trim_end());
        // `extensions.objectFormat` is absent in a sha1 repository, which is
        // git's default and therefore the fallback.
        let object_format = match run(&root, &["config", "--get", "extensions.objectFormat"]) {
            Ok(value) => ObjectFormat::parse(value.trim()).ok_or_else(|| {
                GitError::Unexpected(format!("extensions.objectFormat = {value:?}"))
            })?,
            Err(_) => ObjectFormat::Sha1,
        };
        Ok(Repo {
            root,
            object_format,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn object_format(&self) -> ObjectFormat {
        self.object_format
    }

    /// The manifest's `repo` — the basename of the toplevel (owner ruling,
    /// 2026-08-27; there is no `--repo` flag).
    ///
    /// The value is frozen into the manifest and DM §5.2 builds every node id
    /// from it, so a later rename of the checkout directory makes the computed
    /// value disagree with the recorded one — loudly, at G16, rather than
    /// silently.
    pub fn default_repo_name(&self) -> Option<&str> {
        self.root.file_name()?.to_str()
    }

    /// The branch HEAD names. `None` on a detached HEAD, which `--trunk`
    /// refuses (build plan B3).
    pub fn current_branch(&self) -> Option<String> {
        run(&self.root, &["symbolic-ref", "--quiet", "--short", "HEAD"])
            .ok()
            .map(|s| s.trim_end().to_string())
            .filter(|s| !s.is_empty())
    }

    /// PB §6.7 step 1: "Working tree clean, except paths whose blob equals a
    /// render of a pending run (the interrupted case)."
    pub fn is_clean(&self) -> Result<bool> {
        Ok(run(&self.root, &["status", "--porcelain"])?
            .trim()
            .is_empty())
    }

    /// Paths the porcelain reports as dirty, so a refusal can name them.
    pub fn dirty_paths(&self) -> Result<Vec<String>> {
        Ok(self
            .dirty_entries()?
            .into_iter()
            .map(|(_, path)| path)
            .collect())
    }

    /// The same, with the porcelain's two status characters kept.
    ///
    /// `??` is untracked, and the caller needs to tell it apart: an upgrade
    /// cannot destroy an untracked file it does not write, and neither can
    /// `--abort`, which checks out only the paths the two manifests name. See
    /// `spine init`'s use of this.
    pub fn dirty_entries(&self) -> Result<Vec<(String, String)>> {
        Ok(run(&self.root, &["status", "--porcelain", "-z"])?
            .split('\0')
            .filter(|entry| entry.len() > 3)
            .map(|entry| (entry[..2].to_string(), entry[3..].to_string()))
            .collect())
    }

    /// Whether HEAD exists at all — false in a repository with no commits,
    /// which is the ordinary state for a first `init`.
    pub fn has_head(&self) -> bool {
        run(&self.root, &["rev-parse", "--verify", "--quiet", "HEAD"]).is_ok()
    }

    /// `git hash-object --path <path> --stdin` — the **filtered** form.
    pub fn hash_object_filtered(&self, path: &str, content: &[u8]) -> Result<String> {
        let mut child = Command::new("git")
            .current_dir(&self.root)
            .args(["hash-object", "--path", path, "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| GitError::NotAvailable(e.to_string()))?;
        child
            .stdin
            .as_mut()
            .expect("piped")
            .write_all(content)
            .map_err(|e| GitError::NotAvailable(e.to_string()))?;
        let out = child
            .wait_with_output()
            .map_err(|e| GitError::NotAvailable(e.to_string()))?;
        if !out.status.success() {
            return Err(GitError::Failed {
                argv: format!("hash-object --path {path}"),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
    }

    /// One `git ls-tree` entry: the mode and the object id.
    ///
    /// **The mode is carried because the rollback restoration rule compares it.**
    /// MF §6.7 step 5: a restored path "exists in `T` with the same blob **and
    /// mode**". A restore that put the right bytes back at the wrong mode —
    /// `.spine/ci.sh` restored `100644` where the ancestor had `100755` — would
    /// pass a blob-only check and leave the collector's entry point
    /// unexecutable.
    pub fn ls_tree(&self, commit: &str, path: &str) -> Result<Option<TreeEntry>> {
        // `-z` so the path is raw: `git ls-tree` C-quotes a path containing a
        // quote, a backslash or a control byte otherwise, which is a fourth
        // encoding of one path (R2) and not one this lookup wants.
        let out = run(&self.root, &["ls-tree", "-z", commit, "--", path])?;
        let Some(entry) = out.split('\0').find(|e| !e.is_empty()) else {
            return Ok(None);
        };
        // `<mode> SP <type> SP <oid> TAB <path>`
        let (meta, entry_path) = entry
            .split_once('\t')
            .ok_or_else(|| GitError::Unexpected(format!("ls-tree entry {entry:?}")))?;
        if entry_path != path {
            // A pathspec matched something else — a directory prefix, say. The
            // exact path is what the caller asked about.
            return Ok(None);
        }
        let mut fields = meta.split(' ');
        let mode = fields
            .next()
            .ok_or_else(|| GitError::Unexpected(format!("ls-tree entry {entry:?}")))?;
        let kind = fields.next().unwrap_or_default();
        let oid = fields
            .next()
            .ok_or_else(|| GitError::Unexpected(format!("ls-tree entry {entry:?}")))?;
        if kind != "blob" {
            return Ok(None);
        }
        Ok(Some(TreeEntry {
            mode: mode.to_string(),
            oid: oid.to_string(),
        }))
    }

    /// The bytes at `path` in `commit`, or `None` when that tree lacks it.
    pub fn read_at(&self, commit: &str, path: &str) -> Option<Vec<u8>> {
        let out = Command::new("git")
            .current_dir(&self.root)
            .args(["cat-file", "blob", &format!("{commit}:{path}")])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        out.status.success().then_some(out.stdout)
    }

    /// The bytes of a blob named by its own object id.
    ///
    /// This is what makes `--merge`'s three-way possible on an offline clone:
    /// PB §6.7 records `base` as a blob id precisely so "the pristine content
    /// stays reachable forever through the upgrade commit".
    pub fn read_blob(&self, oid: &str) -> Option<Vec<u8>> {
        let out = Command::new("git")
            .current_dir(&self.root)
            .args(["cat-file", "blob", oid])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        out.status.success().then_some(out.stdout)
    }

    pub fn rev_parse(&self, rev: &str) -> Result<String> {
        Ok(run(&self.root, &["rev-parse", "--verify", "--quiet", rev])?
            .trim_end()
            .to_string())
    }

    /// PB §6.7's `--rollback` default target: "the first-parent commit that last
    /// touched the manifest".
    ///
    /// MF §6.7 is explicit that this is **the tool's heuristic** and not the
    /// gate's rule — the gate locates `U` by the ledger — "and where they
    /// disagree, the gate wins and the tool refuses". So this returns a
    /// candidate, never a verdict.
    pub fn first_parent_commit_touching(&self, from: &str, path: &str) -> Result<Option<String>> {
        let out = run(
            &self.root,
            &[
                "rev-list",
                "--first-parent",
                "--max-count=1",
                from,
                "--",
                path,
            ],
        )?;
        Ok(out.split_whitespace().next().map(str::to_string))
    }

    /// Whether `candidate` is on the first-parent chain of `of` — the
    /// reachability MF §6.7 step 1 requires of `from-manifest=<sha>`
    /// (`restore-ancestor-unreachable`).
    pub fn is_first_parent_ancestor(&self, candidate: &str, of: &str) -> Result<bool> {
        let candidate = self.rev_parse(candidate)?;
        let chain = run(&self.root, &["rev-list", "--first-parent", of])?;
        Ok(chain.split_whitespace().any(|sha| sha == candidate))
    }

    /// The first-parent walk from `from`, newest first, as `(sha, message)`.
    ///
    /// The whole commit message, because a `Spine-Upgrade` line is a trailer and
    /// a re-init has to find the uninstall landing by reading one (MF §6.9).
    /// `%x00` separates records so a message containing blank lines — every
    /// envelope does — cannot be mistaken for a record boundary.
    pub fn first_parent_log(&self, from: &str) -> Result<Vec<(String, String)>> {
        let out = run(
            &self.root,
            &["log", "--first-parent", "--format=%H%x1f%B%x00", from],
        )?;
        Ok(out
            .split('\0')
            .filter(|record| !record.trim().is_empty())
            .filter_map(|record| {
                let (sha, message) = record.trim_start_matches('\n').split_once('\x1f')?;
                Some((sha.to_string(), message.to_string()))
            })
            .collect())
    }

    /// `git checkout <commit> -- <path>` — PB §6.7's own verb for a rollback's
    /// restore. It restores the mode along with the bytes, which is why the
    /// restore does not write the file itself.
    pub fn checkout_path(&self, commit: &str, path: &str) -> Result<()> {
        run(&self.root, &["checkout", commit, "--", path]).map(|_| ())
    }

    /// `git checkout HEAD -- <path>`, for `--abort`.
    pub fn checkout_head_path(&self, path: &str) -> Result<()> {
        self.checkout_path("HEAD", path)
    }

    /// `git rm` for a path a rollback or an uninstall retires.
    ///
    /// `--ignore-unmatch` so removing a path git never tracked is not an error:
    /// the uninstall's rule is stated over the *result* ("absent from `T`"),
    /// not over how many objects had to move to get there.
    pub fn remove_path(&self, path: &str) -> Result<()> {
        run(
            &self.root,
            &["rm", "-f", "--quiet", "--ignore-unmatch", "--", path],
        )
        .map(|_| ())
    }

    /// The bytes at `path` in HEAD, or `None` when HEAD does not have it.
    ///
    /// **HEAD, not the working tree.** PB §6.7 compares the *HEAD* blob against
    /// the manifest blob; reading the working tree would make an uncommitted
    /// edit look like a landed one.
    pub fn read_head(&self, path: &str) -> Option<Vec<u8>> {
        if !self.has_head() {
            return None;
        }
        let out = Command::new("git")
            .current_dir(&self.root)
            .args(["cat-file", "blob", &format!("HEAD:{path}")])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        out.status.success().then_some(out.stdout)
    }
}

fn run(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .current_dir(dir)
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
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// A [`TreeSource`] over a repository's HEAD.
#[derive(Debug)]
pub struct HeadTree<'a> {
    pub repo: &'a Repo,
}

impl TreeSource for HeadTree<'_> {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.repo.read_head(path)
    }

    fn object_format(&self) -> ObjectFormat {
        self.repo.object_format()
    }

    fn hash_object_filtered(&self, path: &str, content: &[u8]) -> String {
        // A failure here is a broken repository, not a plan outcome. Falling
        // back to the unfiltered id would silently record a blob `git ls-tree`
        // disagrees with, so fall back to the unfiltered id **only** so the
        // plan can be printed, and let the apply step fail loudly instead.
        self.repo
            .hash_object_filtered(path, content)
            .unwrap_or_else(|_| spine_canon::git_blob_id(content, self.object_format()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A scratch repository, built with real git.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str, extra_init: &[&str]) -> Option<Self> {
            let base = std::env::temp_dir().join(format!("spine-git-test-{name}"));
            let _ = fs::remove_dir_all(&base);
            fs::create_dir_all(&base).ok()?;
            let mut args = vec!["init", "--quiet"];
            args.extend_from_slice(extra_init);
            args.push(".");
            run(&base, &args).ok()?;
            run(&base, &["config", "user.email", "t@example.invalid"]).ok()?;
            run(&base, &["config", "user.name", "Test"]).ok()?;
            Some(Scratch(base))
        }

        fn write(&self, path: &str, content: &str) {
            let full = self.0.join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(full, content).unwrap();
        }

        fn commit(&self, message: &str) {
            run(&self.0, &["add", "-A"]).unwrap();
            run(&self.0, &["commit", "--quiet", "-m", message]).unwrap();
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn discovers_a_repository_and_its_object_format() {
        let Some(scratch) = Scratch::new("discover", &[]) else {
            return; // git unavailable
        };
        let repo = Repo::discover(&scratch.0).unwrap();
        assert_eq!(repo.object_format(), ObjectFormat::Sha1);
        assert!(repo.default_repo_name().is_some());
        assert!(!repo.has_head(), "a fresh repository has no HEAD");
    }

    #[test]
    fn detects_a_sha256_repository() {
        let Some(scratch) = Scratch::new("sha256", &["--object-format=sha256"]) else {
            return;
        };
        let repo = Repo::discover(&scratch.0).unwrap();
        assert_eq!(repo.object_format(), ObjectFormat::Sha256);
    }

    /// The whole reason this module shells out: `--path` runs the content
    /// through the repository's own filters, so a `.gitattributes` that
    /// normalises line endings does not read as drift.
    #[test]
    fn hash_object_path_honours_gitattributes_and_plain_hashing_does_not() {
        let Some(scratch) = Scratch::new("filters", &[]) else {
            return;
        };
        scratch.write(".gitattributes", "*.txt text eol=lf\n");
        scratch.write("a.txt", "x\n");
        scratch.commit("seed");
        let repo = Repo::discover(&scratch.0).unwrap();

        let crlf = b"line one\r\nline two\r\n";
        let lf = b"line one\nline two\n";

        let filtered_crlf = repo.hash_object_filtered("a.txt", crlf).unwrap();
        let filtered_lf = repo.hash_object_filtered("a.txt", lf).unwrap();
        assert_eq!(
            filtered_crlf, filtered_lf,
            "under `text eol=lf`, CRLF and LF are one blob — which is what \
             makes CRLF churn not drift (PB §6.7)"
        );

        // And the unfiltered id, which is what a region takes, sees them as
        // different — so the two forms are genuinely not interchangeable.
        assert_ne!(
            spine_canon::git_blob_id(crlf, ObjectFormat::Sha1),
            spine_canon::git_blob_id(lf, ObjectFormat::Sha1)
        );
        assert_eq!(
            filtered_lf,
            spine_canon::git_blob_id(lf, ObjectFormat::Sha1),
            "with no filter to apply, the two agree"
        );
    }

    /// The plan reads HEAD, not the working tree — an uncommitted edit must not
    /// read as a landed one.
    #[test]
    fn read_head_ignores_uncommitted_changes() {
        let Some(scratch) = Scratch::new("head", &[]) else {
            return;
        };
        scratch.write("f.txt", "committed\n");
        scratch.commit("seed");
        let repo = Repo::discover(&scratch.0).unwrap();
        assert_eq!(
            repo.read_head("f.txt").as_deref(),
            Some(&b"committed\n"[..])
        );

        scratch.write("f.txt", "edited but not committed\n");
        assert_eq!(
            repo.read_head("f.txt").as_deref(),
            Some(&b"committed\n"[..]),
            "the plan compares HEAD blobs"
        );
        assert!(!repo.is_clean().unwrap());
        assert_eq!(repo.dirty_paths().unwrap(), vec!["f.txt".to_string()]);

        assert!(repo.read_head("absent.txt").is_none());
    }

    #[test]
    fn the_current_branch_is_none_when_head_is_detached() {
        let Some(scratch) = Scratch::new("detached", &[]) else {
            return;
        };
        scratch.write("f.txt", "x\n");
        scratch.commit("one");
        let repo = Repo::discover(&scratch.0).unwrap();
        assert!(repo.current_branch().is_some());

        let sha = run(&scratch.0, &["rev-parse", "HEAD"]).unwrap();
        run(&scratch.0, &["checkout", "--quiet", sha.trim()]).unwrap();
        assert!(
            repo.current_branch().is_none(),
            "a detached HEAD names no branch, and --trunk refuses it"
        );
    }

    /// The plan, computed against a real repository rather than a fake tree.
    #[test]
    fn the_plan_runs_against_a_real_repository() {
        use crate::plan::{self, Action, Desired};
        use spine_manifest::schema::Owner;

        let Some(scratch) = Scratch::new("plan", &[]) else {
            return;
        };
        scratch.write("README.md", "hello\n");
        scratch.commit("seed");
        let repo = Repo::discover(&scratch.0).unwrap();
        let tree = HeadTree { repo: &repo };

        let desired = vec![Desired {
            path: ".spine/ci.sh".into(),
            owner: Owner::SpineOwned,
            template: "ci-generic@4".into(),
            content: b"#!/bin/sh\n".to_vec(),
        }];
        let plan = plan::compute(&tree, &desired, None);
        assert_eq!(plan.rows.len(), 1);
        assert_eq!(plan.rows[0].action, Action::Create);
        assert_eq!(
            plan.rows[0].render_blob.as_deref(),
            Some(
                repo.hash_object_filtered(".spine/ci.sh", b"#!/bin/sh\n")
                    .unwrap()
                    .as_str()
            )
        );
    }
}
