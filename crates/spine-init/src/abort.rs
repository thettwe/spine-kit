//! `spine init --abort` — discard an interrupted run instead of continuing it.
//!
//! PB §6.7, *Interrupted upgrade*, verbatim:
//!
//! > Crash anywhere and one of three states remains, each detected by hash,
//! > each fixed by re-running `spine init`: staging exists and the tree is
//! > untouched (continue); some files renamed but the manifest is old (their
//! > blobs equal the recorded renders, so the re-run recognises its own work
//! > and continues); manifest new but uncommitted (commit). A re-run by a
//! > different binary reports "interrupted by <version>: run that version, or
//! > `--abort`". `spine init --abort` discards instead: `git checkout` every
//! > manifest path, delete created paths, delete staging. **Because the tree
//! > was clean before, abort is total.**
//!
//! That last sentence is the whole argument, and it is also the precondition:
//! step 1 requires a clean tree before any run, so **HEAD is the ground truth**
//! for what the tree looked like. Nothing here needs to guess at what the run
//! changed — `git checkout HEAD -- <path>` restores everything HEAD has, and
//! everything the run created is by definition a path HEAD does not have.
//!
//! **Which paths.** "Every manifest path" is read from *both* manifests: the
//! one committed at HEAD (the last **completed** run, PB §6.7 step 4) and the
//! one the interrupted run recorded in `staging/<run>/manifest.json` "before
//! any rename". The second is what names the paths the run created, and without
//! it an abort would leave behind exactly the files a crash between the first
//! rename and the manifest write had produced.

use crate::git::{GitError, Repo};
use crate::staging::{self, Staging, StagingError};
use spine_manifest::Manifest;
use std::fs;

/// What the abort did, for the line a human reads afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Aborted {
    /// Paths restored from HEAD.
    pub checked_out: Vec<String>,
    /// Paths the run created — absent from HEAD — and therefore deleted.
    pub deleted: Vec<String>,
    /// The staging run that was discarded, if there was one.
    pub staging: Option<String>,
}

impl Aborted {
    pub fn is_empty(&self) -> bool {
        self.checked_out.is_empty() && self.deleted.is_empty() && self.staging.is_none()
    }
}

#[derive(Debug)]
pub enum AbortError {
    Staging(StagingError),
    Git(GitError),
    Io(String),
}

impl core::fmt::Display for AbortError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AbortError::Staging(e) => write!(f, "{e}"),
            AbortError::Git(e) => write!(f, "{e}"),
            AbortError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl core::error::Error for AbortError {}

impl From<StagingError> for AbortError {
    fn from(e: StagingError) -> Self {
        AbortError::Staging(e)
    }
}

impl From<GitError> for AbortError {
    fn from(e: GitError) -> Self {
        AbortError::Git(e)
    }
}

/// Discard an interrupted run.
///
/// Total by construction, and in this order:
///
/// 1. restore every manifest path HEAD has,
/// 2. delete every manifest path HEAD does not have — including the manifest
///    itself, which a run that reached step 4's last line will have written,
/// 3. delete staging.
///
/// Staging goes **last** for the same reason the manifest is written last on
/// the way in: while it survives, a re-run can still tell what the crashed run
/// intended. An abort that deleted it first and then failed would leave a tree
/// nothing could classify.
pub fn abort(repo: &Repo) -> Result<Aborted, AbortError> {
    let mut result = Aborted::default();
    let pending = staging::pending(repo.root())?;

    let mut paths = manifest_paths_at_head(repo);
    if let Some(staging) = &pending {
        paths.extend(recorded_paths(staging));
    }
    // `.spine/manifest.json` is not one of its own `files[]` records, and it is
    // precisely the path interrupted state 3 leaves behind ("manifest new but
    // uncommitted"), so it is added by name.
    paths.push(crate::uninstall::MANIFEST_PATH.to_string());
    paths.sort_unstable();
    paths.dedup();

    for path in paths {
        // A managed region lives inside a host file; restoring the host is what
        // restores the region, and the host is the only thing `git checkout`
        // can name.
        let (file_path, _) = spine_manifest::grammar::split_region(&path);
        if let Some(at_head) = repo.read_head(file_path) {
            // **Report what changed, not what was visited.** A path already
            // equal to HEAD is checked out to no effect, and listing it as
            // `restore` told a human that an abort had undone work when it had
            // undone nothing — and made `Aborted::is_empty` false on a clean
            // tree, so `--abort` never says "nothing to abort".
            //
            // The checkout still runs where the bytes differ: comparing first
            // is cheaper than a `git checkout` per path and is the only way to
            // know which of the two happened.
            let on_disk = fs::read(repo.root().join(file_path)).ok();
            if on_disk.as_deref() == Some(at_head.as_slice()) {
                continue;
            }
            repo.checkout_head_path(file_path)?;
            let entry = file_path.to_string();
            if !result.checked_out.contains(&entry) {
                result.checked_out.push(entry);
            }
        } else {
            let full = repo.root().join(file_path);
            if full.exists() {
                fs::remove_file(&full).map_err(|e| AbortError::Io(e.to_string()))?;
                result.deleted.push(file_path.to_string());
            }
        }
    }

    if let Some(staging) = pending {
        staging.discard()?;
        result.staging = Some(staging.run);
    }
    Ok(result)
}

/// The `files[]` paths of the manifest committed at HEAD — the last *completed*
/// run's record of what spine owns.
fn manifest_paths_at_head(repo: &Repo) -> Vec<String> {
    repo.read_head(crate::uninstall::MANIFEST_PATH)
        .and_then(|bytes| Manifest::parse(&bytes, Some(repo.object_format())).ok())
        .map(|m| m.files().into_iter().map(|r| r.path).collect())
        .unwrap_or_default()
}

/// The `files[]` paths the interrupted run recorded before its first rename.
///
/// A malformed or absent record contributes nothing rather than failing the
/// abort: PB §6.7's guarantee rests on HEAD, not on this file, and an abort
/// that refused because a crashed run left half a JSON document would be an
/// abort that cannot clean up after the crash it exists for.
fn recorded_paths(staging: &Staging) -> Vec<String> {
    let Some(bytes) = staging.recorded_manifest() else {
        return Vec::new();
    };
    // Parsed leniently — the recorded manifest is spine's own render and not a
    // repository input, and any path it names is a path this abort should look
    // at even if some other member of the document is unreadable.
    let Ok(value) = spine_canon::parse(bytes.strip_suffix(b"\n").unwrap_or(&bytes)) else {
        return Vec::new();
    };
    let Some(spine_canon::Value::Arr(records)) = value.get("files") else {
        return Vec::new();
    };
    records
        .iter()
        .filter_map(|r| r.get("path").and_then(spine_canon::Value::as_str))
        .map(str::to_string)
        .collect()
}
