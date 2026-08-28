//! PB §6.7 step 4 — the atomic apply, and the only code in the crate that
//! writes into someone else's repository.
//!
//! Step 4, verbatim, because every clause of it is a rule this module keeps:
//!
//! > Everything is rendered into gitignored `.spine/cache/staging/<run>/` —
//! > with the renders of the binary that started the run recorded in
//! > `staging/<run>/manifest.json` before any rename — and parse-validated
//! > (YAML, JSON) before a single tree file changes; each file then moves into
//! > place by atomic rename; the manifest is written **last**; staging is
//! > deleted. **The manifest therefore always describes the last *completed*
//! > upgrade.**
//!
//! The ordering is not stylistic. It is what makes PB §6.7's three interrupted
//! states distinguishable by hash alone, and what makes "the manifest describes
//! the last *completed* upgrade" true rather than aspirational.

use crate::plan::{Action, Desired, Plan};
use crate::staging::{Staging, StagingError};
use spine_manifest::region::{self, MarkerStyle};
use spine_manifest::schema::Owner;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum ApplyError {
    /// The plan refused. Nothing is written, and this is not an error case so
    /// much as the plan's own answer, carried where a caller cannot ignore it.
    PlanRefused(usize),
    Staging(StagingError),
    Io(String),
    /// A region's host file could not be rendered — markers missing, or a
    /// template with no marker style.
    Region {
        path: String,
        why: String,
    },
}

impl core::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ApplyError::PlanRefused(n) => write!(
                f,
                "the plan refuses {n} path(s); nothing was written \
                 (PB §6.7: one refusing path stops the whole upgrade)"
            ),
            ApplyError::Staging(e) => write!(f, "{e}"),
            ApplyError::Io(e) => write!(f, "{e}"),
            ApplyError::Region { path, why } => write!(f, "{path}: {why}"),
        }
    }
}

impl core::error::Error for ApplyError {}

impl From<StagingError> for ApplyError {
    fn from(e: StagingError) -> Self {
        ApplyError::Staging(e)
    }
}

/// What one applied path did, for the report a human reads afterwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    pub path: String,
    pub action: Action,
    /// The blob the manifest will record — of the whole file, or of the region.
    pub blob: String,
}

/// Render, stage, validate, rename, then write the manifest.
///
/// `manifest_bytes` is a closure and not a value because the manifest records
/// the blobs of what was written, so it cannot be built until the renders are
/// known — and it must still be *recorded in staging before any rename*, which
/// is why it is built at the top and written to the tree at the bottom.
pub fn apply(
    repo_root: &Path,
    plan: &Plan,
    desired: &[Desired],
    object_format: spine_canon::ObjectFormat,
    template_version: &dyn Fn(&str) -> Option<u64>,
    manifest_bytes: &dyn Fn(&[Applied]) -> Vec<u8>,
) -> Result<Vec<Applied>, ApplyError> {
    // PB §6.7 step 3: "Refusal is the default. One `spine-owned` path with HEAD
    // blob ≠ manifest blob stops the whole upgrade — a partial upgrade is the
    // interrupted case by another name." So this is checked before a directory
    // is created, not before each write.
    let refused = plan.refusals().count();
    if refused > 0 {
        return Err(ApplyError::PlanRefused(refused));
    }

    let staging = Staging::create(repo_root)?;

    // ---- Render into staging. Nothing in the tree moves. ------------------
    //
    // A region's staged file is the WHOLE HOST FILE with the region written
    // into it, not the region's bytes: the rename replaces a file, and a file
    // is what has to be staged. The recorded blob is still the region's, which
    // is MF §3.5's rule and the reason the two are computed separately here.
    let mut applied: Vec<Applied> = Vec::new();

    for want in desired {
        let row = plan
            .rows
            .iter()
            .find(|r| r.path == want.path)
            .expect("every desired path has a plan row");
        if !matches!(row.action, Action::Create | Action::Update) {
            continue;
        }

        let (file_path, region_key) = spine_manifest::grammar::split_region(&want.path);
        let (bytes, blob) = match region_key {
            None => (
                want.content.clone(),
                row.render_blob.clone().expect("a written row has a render"),
            ),
            Some(_) => {
                let template = want.template.split('@').next().unwrap_or_default();
                let version = template_version(template).unwrap_or(0);
                let style =
                    MarkerStyle::for_template(template).ok_or_else(|| ApplyError::Region {
                        path: want.path.clone(),
                        why: format!("{template} has no marker style (MF §3.7)"),
                    })?;
                let host = fs::read(repo_root.join(file_path)).unwrap_or_default();
                let body = core::str::from_utf8(&want.content).map_err(|_| ApplyError::Region {
                    path: want.path.clone(),
                    why: "a region body must be UTF-8".into(),
                })?;

                let rendered = if region::find(&host, template, version, style).is_ok() {
                    spine_template::regions::replace_in(&host, template, version, body)
                } else {
                    spine_template::regions::create_in(&host, template, version, body)
                }
                .ok_or_else(|| ApplyError::Region {
                    path: want.path.clone(),
                    why: "the region could not be written into its host".into(),
                })?;

                // MF §3.5: a region's blob is `git hash-object` over the
                // region's bytes **with no filters** — "those bytes are already
                // in-blob bytes". So it is the body, never the host file.
                (
                    rendered,
                    spine_canon::git_blob_id(&want.content, object_format),
                )
            }
        };

        // Parse validation happens inside `stage`, before any rename.
        staging.stage(file_path, &bytes)?;
        applied.push(Applied {
            path: want.path.clone(),
            action: row.action,
            blob,
        });
    }

    // ---- Record the renders BEFORE any rename. ---------------------------
    //
    // "with the renders of the binary that started the run recorded in
    // `staging/<run>/manifest.json` before any rename". This is what lets a
    // re-run after a crash recognise its own work: interrupted state 2 is
    // decided by comparing the tree's blobs against these.
    let manifest = manifest_bytes(&applied);
    staging.record_manifest(&manifest)?;

    // ---- Rename into place. ----------------------------------------------
    let mut renamed: Vec<&str> = Vec::new();
    for entry in &applied {
        let (file_path, _) = spine_manifest::grammar::split_region(&entry.path);
        if renamed.contains(&file_path) {
            // Two regions in one host file would be two staged copies of one
            // path. v1 ships one region per host, and MF §3.7 requires two on
            // one file to differ in both key and template name; renaming twice
            // would silently drop the first.
            continue;
        }
        staging.apply_one(repo_root, file_path)?;
        renamed.push(file_path);
    }

    // ---- Delete the paths the plan retires. ------------------------------
    //
    // Build plan B7: a `files[]` path is deleted iff its owner is
    // `spine-owned` and the new render set does not name it. `user-owned` and
    // `user-modified` paths are left in place and reported — spine never
    // removes bytes a human owns.
    for row in &plan.rows {
        if row.action == Action::Delete && row.owner == Owner::SpineOwned {
            let (file_path, _) = spine_manifest::grammar::split_region(&row.path);
            fs::remove_file(repo_root.join(file_path))
                .map_err(|e| ApplyError::Io(e.to_string()))?;
        }
    }

    // ---- The manifest, LAST. ---------------------------------------------
    //
    // "the manifest is written **last**; staging is deleted. The manifest
    // therefore always describes the last *completed* upgrade." A crash before
    // this line leaves interrupted state 2 — files renamed, manifest old —
    // which a re-run recognises by hash. A crash after it but before the
    // discard leaves state 3.
    let manifest_path = repo_root.join(".spine/manifest.json");
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).map_err(|e| ApplyError::Io(e.to_string()))?;
    }
    fs::write(&manifest_path, &manifest).map_err(|e| ApplyError::Io(e.to_string()))?;

    staging.discard()?;
    Ok(applied)
}
