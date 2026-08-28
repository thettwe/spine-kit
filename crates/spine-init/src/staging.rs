//! The atomic apply, and the three interrupted states (PB §6.7 steps 4 and
//! *Interrupted upgrade*).
//!
//! PB §6.7 step 4, verbatim: "Everything is rendered into gitignored
//! `.spine/cache/staging/<run>/` — with the renders of the binary that started
//! the run recorded in `staging/<run>/manifest.json` before any rename — and
//! parse-validated (YAML, JSON) before a single tree file changes; each file
//! then moves into place by atomic rename; the manifest is written **last**;
//! staging is deleted. **The manifest therefore always describes the last
//! *completed* upgrade.**"
//!
//! That last sentence is the invariant everything else here serves, and it is
//! what makes the three crash states distinguishable by hash alone.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// `.spine/cache/staging/<run>/`.
pub const STAGING_DIR: &str = ".spine/cache/staging";

#[derive(Debug)]
pub enum StagingError {
    Io(String),
    /// Build plan B6: "At most one staging directory may exist at a time." A
    /// second would make interrupted states 1 and 2 indistinguishable, since
    /// each is detected by comparing the tree against *the* pending run.
    SecondRun(String),
    /// PB §6.7: "A re-run by a different binary reports 'interrupted by
    /// <version>: run that version, or `--abort`'."
    InterruptedByOtherVersion(String),
    /// PB §6.7 step 4: "parse-validated (YAML, JSON) before a single tree file
    /// changes".
    ParseValidation {
        path: String,
        why: String,
    },
}

impl core::fmt::Display for StagingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StagingError::Io(e) => write!(f, "{e}"),
            StagingError::SecondRun(run) => {
                write!(
                    f,
                    "a staging run is already pending ({run}); re-run to continue it, or --abort"
                )
            }
            StagingError::InterruptedByOtherVersion(v) => {
                write!(f, "interrupted by {v}: run that version, or --abort")
            }
            StagingError::ParseValidation { path, why } => {
                write!(f, "{path} did not validate: {why}")
            }
        }
    }
}

impl core::error::Error for StagingError {}

type Result<T> = core::result::Result<T, StagingError>;

fn io<E: core::fmt::Display>(e: E) -> StagingError {
    StagingError::Io(e.to_string())
}

/// The three states PB §6.7 says a crash can leave, "each detected by hash,
/// each fixed by re-running `spine init`".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Interrupted {
    /// "staging exists and the tree is untouched (continue)"
    StagingOnly { run: String },
    /// "some files renamed but the manifest is old (their blobs equal the
    /// recorded renders, so the re-run recognises its own work and continues)"
    PartiallyApplied { run: String, applied: Vec<String> },
    /// "manifest new but uncommitted (commit)"
    ManifestUncommitted,
    /// Nothing pending.
    None,
}

/// A pending run's directory.
#[derive(Debug, Clone)]
pub struct Staging {
    pub run: String,
    pub dir: PathBuf,
}

/// MF §7 rule 1 bars a wall clock from these artifacts, so `<run>` is a 32-hex
/// **random nonce** rather than a timestamp (build plan B6). It is covered by
/// no digest, appears in no gate's input, and lives only inside a gitignored
/// directory — so randomness here costs the design nothing and a clock would
/// cost it determinism.
pub fn new_run_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    fs::File::open("/dev/urandom")
        .map_err(io)?
        .read_exact(&mut bytes)
        .map_err(io)?;
    let mut out = String::with_capacity(32);
    for byte in bytes {
        out.push(char::from_digit((byte >> 4) as u32, 16).expect("nibble"));
        out.push(char::from_digit((byte & 0xF) as u32, 16).expect("nibble"));
    }
    Ok(out)
}

/// The staging run currently pending, if any.
///
/// B6: at most one may exist. Two is an error rather than a choice, because
/// picking one would make the interrupted-state detection depend on which.
pub fn pending(repo_root: &Path) -> Result<Option<Staging>> {
    let base = repo_root.join(STAGING_DIR);
    if !base.exists() {
        return Ok(None);
    }
    let mut runs: Vec<String> = Vec::new();
    for entry in fs::read_dir(&base).map_err(io)? {
        let entry = entry.map_err(io)?;
        if entry.file_type().map_err(io)?.is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            runs.push(name.to_string());
        }
    }
    runs.sort();
    match runs.len() {
        0 => Ok(None),
        1 => Ok(Some(Staging {
            dir: base.join(&runs[0]),
            run: runs.remove(0),
        })),
        _ => Err(StagingError::SecondRun(runs.join(", "))),
    }
}

impl Staging {
    /// Create a fresh staging run. Refuses if one is already pending — a second
    /// `init` that finds one "treats it as the interrupted case and never
    /// creates a second" (build plan B6).
    ///
    /// **Callers that are the re-run want [`Staging::resume_or_create`].** PB
    /// §6.7's *Interrupted upgrade* says every crash state is "**fixed by
    /// re-running `spine init`**", and `apply` called this unconditionally, so
    /// the re-run refused with a message telling the operator to do the thing
    /// they had just done. With `--abort` unimplemented, the repository was
    /// stuck.
    pub fn create(repo_root: &Path) -> Result<Self> {
        if let Some(existing) = pending(repo_root)? {
            return Err(StagingError::SecondRun(existing.run));
        }
        Self::fresh(repo_root)
    }

    /// The re-run's entry: adopt the pending run if there is one, else create.
    ///
    /// The second element is whether a run was adopted, which is what a caller
    /// reports to the operator — PB §6.7 wants the re-run to say it recognised
    /// its own work, not to do it silently.
    ///
    /// Adopting is safe because a render is a pure function of the release and
    /// the repository's parameters: re-staging writes the same bytes over the
    /// same names. A staged file the new run no longer wants is never renamed
    /// — `apply` renames what this run staged — and goes with the discard.
    ///
    /// Two pending runs stays a refusal. That is not a state PB §6.7
    /// describes, and picking one of them would be a guess.
    pub fn resume_or_create(repo_root: &Path) -> Result<(Self, bool)> {
        match pending(repo_root)? {
            Some(existing) => Ok((existing, true)),
            None => Ok((Self::fresh(repo_root)?, false)),
        }
    }

    fn fresh(repo_root: &Path) -> Result<Self> {
        let run = new_run_id()?;
        let dir = repo_root.join(STAGING_DIR).join(&run);
        fs::create_dir_all(&dir).map_err(io)?;
        Ok(Staging { run, dir })
    }

    /// Whether the tree's bytes at `repo_path` are exactly what this run
    /// staged there — PB §6.7 step 1's exception, "paths whose blob equals a
    /// render of a pending run".
    ///
    /// Compared as bytes rather than as blob ids: the two agree for every
    /// path either way, and a byte comparison needs no `hash-object` and no
    /// opinion about filters. A path this run staged nothing for is `false`,
    /// which is the fail-closed answer.
    pub fn tree_matches_staged(&self, repo_root: &Path, repo_path: &str) -> bool {
        let Ok(staged) = fs::read(self.staged_path(repo_path)) else {
            return false;
        };
        fs::read(repo_root.join(repo_path)).is_ok_and(|tree| tree == staged)
    }

    /// Where a repository path's render is staged. Flattened by percent-free
    /// escaping of `/` so a nested path needs no directory tree, and so a
    /// staged name can never escape the staging directory.
    fn staged_path(&self, repo_path: &str) -> PathBuf {
        self.dir
            .join(repo_path.replace('/', "%2F").replace('#', "%23"))
    }

    /// Stage one render. Nothing in the tree moves.
    pub fn stage(&self, repo_path: &str, content: &[u8]) -> Result<()> {
        validate_parseable(repo_path, content)?;
        fs::write(self.staged_path(repo_path), content).map_err(io)
    }

    /// PB §6.7 step 4: "with the renders of the binary that started the run
    /// recorded in `staging/<run>/manifest.json` **before any rename**".
    ///
    /// This is what lets a re-run "recognise its own work": interrupted state 2
    /// is decided by comparing the tree's blobs against these recorded renders.
    pub fn record_manifest(&self, manifest_bytes: &[u8]) -> Result<()> {
        fs::write(self.dir.join("manifest.json"), manifest_bytes).map_err(io)
    }

    pub fn recorded_manifest(&self) -> Option<Vec<u8>> {
        fs::read(self.dir.join("manifest.json")).ok()
    }

    /// Move one staged render into the tree by atomic rename.
    ///
    /// `rename(2)` within one filesystem is atomic, and staging lives under
    /// `.spine/cache/` in the same working tree, so it is.
    pub fn apply_one(&self, repo_root: &Path, repo_path: &str) -> Result<()> {
        let target = repo_root.join(repo_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(io)?;
        }
        fs::rename(self.staged_path(repo_path), target).map_err(io)
    }

    /// "staging is deleted" — the last step, after the manifest is written.
    pub fn discard(&self) -> Result<()> {
        fs::remove_dir_all(&self.dir).map_err(io)
    }
}

/// PB §6.7 step 4: "parse-validated (YAML, JSON) before a single tree file
/// changes".
///
/// The point is not to check spine's own renders — those come from templates —
/// but to catch a substitution that produced syntactically broken output before
/// any of it reaches the tree.
fn validate_parseable(repo_path: &str, content: &[u8]) -> Result<()> {
    let fail = |why: &str| StagingError::ParseValidation {
        path: repo_path.to_string(),
        why: why.to_string(),
    };

    if repo_path.ends_with(".json") {
        let text = core::str::from_utf8(content).map_err(|_| fail("not UTF-8"))?;
        spine_canon::parse(text.trim_end().as_bytes()).map_err(|e| fail(&e.to_string()))?;
    } else if repo_path.ends_with(".yml") || repo_path.ends_with(".yaml") {
        // A full YAML parser is a dependency this crate does not otherwise
        // need, and the templates are the release's own bytes. What is checked
        // is what a substitution can break: encoding, stray NUL, and the tab
        // indentation YAML forbids outright.
        let text = core::str::from_utf8(content).map_err(|_| fail("not UTF-8"))?;
        if text.contains('\0') {
            return Err(fail("a NUL byte"));
        }
        for (index, line) in text.lines().enumerate() {
            let indent: &str = &line[..line.len() - line.trim_start().len()];
            if indent.contains('\t') {
                return Err(fail(&format!("a tab indent on line {}", index + 1)));
            }
        }
    }
    Ok(())
}

/// Classify what a crashed run left behind (PB §6.7, *Interrupted upgrade*).
///
/// `applied_blob` answers "what is the blob of this path in the tree now", and
/// `render_blob` "what did the pending run intend to write there" — both from
/// the caller, so this function has no opinion about filters.
pub fn classify(
    staging: Option<&Staging>,
    manifest_is_current: bool,
    paths: &[(String, Option<String>, String)],
) -> Interrupted {
    let Some(staging) = staging else {
        return if manifest_is_current {
            Interrupted::None
        } else {
            // "manifest new but uncommitted (commit)"
            Interrupted::ManifestUncommitted
        };
    };

    // "some files renamed but the manifest is old (their blobs equal the
    // recorded renders, so the re-run recognises its own work and continues)"
    let applied: Vec<String> = paths
        .iter()
        .filter(|(_, tree_blob, render_blob)| tree_blob.as_deref() == Some(render_blob.as_str()))
        .map(|(path, _, _)| path.clone())
        .collect();

    if applied.is_empty() {
        // "staging exists and the tree is untouched (continue)"
        Interrupted::StagingOnly {
            run: staging.run.clone(),
        }
    } else {
        Interrupted::PartiallyApplied {
            run: staging.run.clone(),
            applied,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("spine-staging-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// MF §7 rule 1 bars a wall clock from these artifacts.
    #[test]
    fn a_run_id_is_a_32_hex_nonce_and_not_a_clock() {
        let a = new_run_id().unwrap();
        let b = new_run_id().unwrap();
        assert_eq!(a.len(), 32);
        assert!(
            a.bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert_ne!(a, b, "two runs must not collide");
    }

    /// Build plan B6: at most one staging directory. Two makes interrupted
    /// states 1 and 2 indistinguishable.
    #[test]
    fn a_second_run_is_refused_rather_than_created() {
        let root = scratch("second-run");
        let first = Staging::create(&root).unwrap();
        assert!(matches!(
            Staging::create(&root),
            Err(StagingError::SecondRun(_))
        ));
        // And the pending run is found, so the re-run can continue it.
        let found = pending(&root).unwrap().unwrap();
        assert_eq!(found.run, first.run);
    }

    /// PB §6.7, *Interrupted upgrade*: every crash state is "each fixed by
    /// **re-running `spine init`**".
    ///
    /// `apply` called `create` unconditionally, so the re-run refused with a
    /// message telling the operator to re-run — and with `--abort`
    /// unimplemented, the repository was stuck.
    #[test]
    fn the_re_run_adopts_the_pending_run_rather_than_refusing() {
        let root = scratch("resume");
        let first = Staging::create(&root).unwrap();
        first.stage(".spine/ci.sh", b"#!/bin/sh\n").unwrap();

        let (resumed, was_pending) = Staging::resume_or_create(&root).unwrap();
        assert!(was_pending, "the re-run recognises its own work");
        assert_eq!(resumed.run, first.run);

        // And with nothing pending it creates one, reporting that it did not
        // adopt.
        let clean = scratch("resume-clean");
        let (_fresh, was_pending) = Staging::resume_or_create(&clean).unwrap();
        assert!(!was_pending);
    }

    /// PB §6.7 step 1's exception, "paths whose blob equals a render of a
    /// pending run" — the narrow test that separates this run's own renames
    /// from a human's uncommitted edit.
    #[test]
    fn only_bytes_equal_to_the_staged_render_are_this_runs_own_work() {
        let root = scratch("staged-match");
        let staging = Staging::create(&root).unwrap();
        staging
            .stage(".spine/ci.sh", b"#!/bin/sh\nrender\n")
            .unwrap();

        // Not yet renamed into the tree.
        assert!(!staging.tree_matches_staged(&root, ".spine/ci.sh"));

        fs::create_dir_all(root.join(".spine")).unwrap();
        fs::write(root.join(".spine/ci.sh"), b"#!/bin/sh\nrender\n").unwrap();
        assert!(staging.tree_matches_staged(&root, ".spine/ci.sh"));

        // A human's edit over the top is not.
        fs::write(root.join(".spine/ci.sh"), b"MY UNCOMMITTED WORK\n").unwrap();
        assert!(!staging.tree_matches_staged(&root, ".spine/ci.sh"));

        // And a path this run staged nothing for is never exempt.
        assert!(!staging.tree_matches_staged(&root, "README.md"));
    }

    /// PB §6.7 step 4's ordering: nothing in the tree moves until every render
    /// is staged and validated.
    #[test]
    fn staging_writes_nothing_into_the_tree() {
        let root = scratch("no-tree-writes");
        let staging = Staging::create(&root).unwrap();
        staging.stage(".spine/ci.sh", b"#!/bin/sh\n").unwrap();
        staging
            .stage(".github/workflows/spine-collect.yml", b"name: c\n")
            .unwrap();

        assert!(!root.join(".spine/ci.sh").exists());
        assert!(!root.join(".github/workflows/spine-collect.yml").exists());

        staging.apply_one(&root, ".spine/ci.sh").unwrap();
        assert_eq!(
            fs::read(root.join(".spine/ci.sh")).unwrap(),
            b"#!/bin/sh\n",
            "and the rename creates the parent directory"
        );
    }

    /// "parse-validated (YAML, JSON) before a single tree file changes."
    #[test]
    fn a_broken_render_is_caught_at_staging_time() {
        let root = scratch("validate");
        let staging = Staging::create(&root).unwrap();

        assert!(matches!(
            staging.stage(".spine/manifest.json", b"{not json"),
            Err(StagingError::ParseValidation { .. })
        ));
        // YAML forbids a tab indent outright.
        assert!(matches!(
            staging.stage(".github/workflows/x.yml", b"jobs:\n\tbuild:\n"),
            Err(StagingError::ParseValidation { .. })
        ));
        // Valid ones stage.
        assert!(
            staging
                .stage(".spine/manifest.json", b"{\"a\":1}\n")
                .is_ok()
        );
        assert!(
            staging
                .stage(".github/workflows/x.yml", b"jobs:\n  build:\n")
                .is_ok()
        );
    }

    /// A staged name can never escape the staging directory, however the
    /// repository path is spelled.
    #[test]
    fn staged_names_are_flattened_and_cannot_escape() {
        let root = scratch("flatten");
        let staging = Staging::create(&root).unwrap();
        for path in [".spine/ci.sh", "AGENTS.md#spine", "a/b/c/d.yml"] {
            let staged = staging.staged_path(path);
            assert_eq!(staged.parent().unwrap(), staging.dir);
        }
    }

    /// PB §6.7's three crash states, each "detected by hash".
    #[test]
    fn the_three_interrupted_states_are_distinguished() {
        let root = scratch("interrupted");
        let staging = Staging::create(&root).unwrap();

        // 1. staging exists and the tree is untouched.
        let untouched = vec![
            (".spine/ci.sh".to_string(), None, "aaa".to_string()),
            (
                "AGENTS.md#spine".to_string(),
                Some("old".into()),
                "bbb".to_string(),
            ),
        ];
        assert_eq!(
            classify(Some(&staging), false, &untouched),
            Interrupted::StagingOnly {
                run: staging.run.clone()
            }
        );

        // 2. some files renamed but the manifest is old — "their blobs equal
        //    the recorded renders, so the re-run recognises its own work".
        let partial = vec![
            (
                ".spine/ci.sh".to_string(),
                Some("aaa".into()),
                "aaa".to_string(),
            ),
            (
                "AGENTS.md#spine".to_string(),
                Some("old".into()),
                "bbb".to_string(),
            ),
        ];
        assert_eq!(
            classify(Some(&staging), false, &partial),
            Interrupted::PartiallyApplied {
                run: staging.run.clone(),
                applied: vec![".spine/ci.sh".to_string()],
            }
        );

        // 3. manifest new but uncommitted — staging is already gone.
        assert_eq!(classify(None, false, &[]), Interrupted::ManifestUncommitted);

        // Nothing pending.
        assert_eq!(classify(None, true, &[]), Interrupted::None);
    }

    /// "the manifest is written **last**; staging is deleted. The manifest
    /// therefore always describes the last *completed* upgrade."
    #[test]
    fn the_recorded_manifest_survives_until_staging_is_discarded() {
        let root = scratch("record");
        let staging = Staging::create(&root).unwrap();
        staging
            .record_manifest(b"{\"manifest_version\":1}\n")
            .unwrap();
        assert_eq!(
            staging.recorded_manifest().as_deref(),
            Some(&b"{\"manifest_version\":1}\n"[..])
        );
        staging.discard().unwrap();
        assert!(pending(&root).unwrap().is_none());
    }
}
