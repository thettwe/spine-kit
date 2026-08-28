//! `--merge`, `--adopt`, `--force` — PB §6.7 step 3's three exits from a
//! refusal.
//!
//! Step 3, verbatim, because every clause is a rule here:
//!
//! > **Refusal is the default.** One `spine-owned` path with HEAD blob ≠
//! > manifest blob stops the whole upgrade — a partial upgrade is the
//! > interrupted case by another name. Resolution is explicit: `--merge` runs
//! > `git merge-file` (base = manifest blob, ours = HEAD, theirs = new render);
//! > a clean merge lands and reclassifies the path `user-modified`; a conflict
//! > refuses (conflict markers never touch the tree). `--adopt <path>`
//! > reclassifies without merging — spec-kit preserves such files with a
//! > warning; spine refuses until you say which class they are. `--force
//! > <path>` overwrites — recorded on the upgrade line and counted by `spine
//! > stats`, the same loud-override rule as break-glass.
//!
//! The refusal is the product. Every exit is something a human typed, and none
//! of them is reachable by a heuristic: MF §3.5, "Nothing infers a class change
//! from a hash; reclassification is `--adopt` or a successful `--merge`."

use crate::git::{GitError, Repo};
use crate::plan::{Action, Desired, Plan, RefuseReason};
use spine_manifest::Manifest;
use spine_manifest::region::RegionError;
use spine_manifest::schema::Owner;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// What the operator typed.
#[derive(Debug, Clone, Default)]
pub struct Resolutions {
    /// `--merge` is a whole-run flag: it has no path argument in PB §11's
    /// signature, so it offers the three-way to every diverged path at once.
    pub merge: bool,
    /// `--adopt <path|file#region>`, repeatable. `esc`-encoded, as the manifest
    /// spells a path.
    pub adopt: Vec<String>,
    /// `--force <path>`, repeatable.
    pub force: Vec<String>,
}

/// A record whose ownership class this run changed.
///
/// MF §3.5: "reclassification is `--adopt` or a successful `--merge`, and it
/// lands as a manifest change like any other."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reclassification {
    pub path: String,
    /// Always [`Owner::UserModified`] in v1 — both exits move a path into the
    /// one class whose rule is "never rewritten silently".
    pub owner: Owner,
    /// The `base` member the record gains: "the pristine render the human
    /// diverged from, updated on every `--merge`" (MF §3.5).
    pub base: String,
    /// Which flag did it, for the line a human reads.
    pub by: By,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    Merge,
    Adopt,
}

impl By {
    pub fn token(self) -> &'static str {
        match self {
            By::Merge => "--merge",
            By::Adopt => "--adopt",
        }
    }
}

/// The plan after resolution, plus what the manifest and the `Spine-Upgrade`
/// line must record about it.
#[derive(Debug, Clone)]
pub struct Resolved {
    pub plan: Plan,
    /// The render set, with a merged path's content replaced by the **merge
    /// result** rather than the render — which is the whole point of `--merge`.
    pub desired: Vec<Desired>,
    /// `esc`-encoded paths overwritten under `--force`, for `forced=`.
    ///
    /// A `--force` naming a path that was not diverged contributes nothing:
    /// MF §6.4 derives the set from blobs — "a path in the line and not in the
    /// set is a claim of an override that did not happen" — and a claim that
    /// did not happen fails G16.
    pub forced: Vec<String>,
    pub reclassified: Vec<Reclassification>,
}

#[derive(Debug)]
pub enum ResolveError {
    /// "a conflict refuses (conflict markers never touch the tree)" — so the
    /// merge result is dropped on the floor rather than staged.
    MergeConflict(String),
    /// MF §6.6: "A `base` naming an unreachable blob costs `--merge`, not a
    /// landing." The pristine render is gone from the object store, so there is
    /// no three-way to run.
    BaseUnreachable {
        path: String,
        blob: String,
    },
    /// A managed region, whose recorded `blob` names bytes git never stored.
    ///
    /// **A gap in the corpus, and the one this crate found.** PB §6.7 argues
    /// that "the pristine content stays reachable forever through the upgrade
    /// commit, which is what makes three-way merge and rollback work on an
    /// offline clone holding nothing but git objects". That holds for a file
    /// record, whose `blob` *is* an object git wrote. It does not hold for a
    /// region: MF §3.5 records a region's `blob` as "`git hash-object` over the
    /// region's bytes with no filters", and those bytes are a **sub-range of a
    /// host file** — git stores the host, never the range, so the id names no
    /// object in any clone.
    ///
    /// The rollback is unaffected: it reads the region out of the host file at
    /// `<sha>`, which is reachable (MF §6.7.2). Only `--merge` loses its base.
    /// MF §6.6 already licenses the outcome — "A `base` naming an unreachable
    /// blob costs `--merge`, not a landing" — so this refuses and names the two
    /// exits that remain rather than inventing a base.
    RegionHasNoReachableBase {
        path: String,
        blob: String,
    },
    /// `--adopt` on a path no record claims. There is no pristine render the
    /// human diverged from, so `base` cannot be written and the class change
    /// would be a lie; `--force` is the exit for a foreign path.
    ///
    /// DERIVED: PB §6.7 names `--adopt` only for paths a record already claims
    /// (a diverged `spine-owned` blob, a marker-stripped region). Refusing the
    /// unrecorded case is the fail-closed reading; inventing a `base` is not.
    AdoptWithoutRecord(String),
    /// PB §6.7 names exactly two exits from "markers removed": "restoring them
    /// or `--adopt AGENTS.md#spine`". `--force` is not one of them — a region
    /// whose markers are gone cannot be located, so an overwrite would append a
    /// second copy of the block rather than replace one.
    ForceIsNotAnExit(String),
    /// The same path named by `--adopt` and `--force`. Two contradictory
    /// instructions about one path are refused rather than ordered.
    Contradictory(String),
    /// A path named by `--adopt` or `--force` that the plan did not refuse.
    /// Silently ignoring it would let a typo look like a resolution.
    NotRefused(String),
    Git(GitError),
    Io(String),
}

impl core::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ResolveError::MergeConflict(p) => write!(
                f,
                "{p}: --merge conflicted; nothing was written \
                 (PB §6.7: conflict markers never touch the tree)"
            ),
            ResolveError::BaseUnreachable { path, blob } => write!(
                f,
                "{path}: the pristine blob {blob} is not in this clone's object store, \
                 so there is no three-way base"
            ),
            ResolveError::RegionHasNoReachableBase { path, blob } => write!(
                f,
                "{path}: a region's blob ({blob}) hashes a range inside a host file, which \
                 git never stored as an object, so --merge has no base; the exits are \
                 --adopt {path} and --force {path}"
            ),
            ResolveError::AdoptWithoutRecord(p) => write!(
                f,
                "{p}: --adopt needs a record to reclassify; no manifest record claims it. \
                 --force is the exit for a path spine did not write"
            ),
            ResolveError::ForceIsNotAnExit(p) => write!(
                f,
                "{p}: markers removed — the exits are restoring them or --adopt {p}, \
                 never --force"
            ),
            ResolveError::Contradictory(p) => {
                write!(f, "{p}: named by both --adopt and --force")
            }
            ResolveError::NotRefused(p) => {
                write!(
                    f,
                    "{p}: the plan did not refuse it, so there is nothing to resolve"
                )
            }
            ResolveError::Git(e) => write!(f, "{e}"),
            ResolveError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl core::error::Error for ResolveError {}

impl From<GitError> for ResolveError {
    fn from(e: GitError) -> Self {
        ResolveError::Git(e)
    }
}

/// Apply the operator's resolutions to a refusing plan.
///
/// Rows the operator did not name keep refusing, and one refusing row still
/// stops the whole upgrade — resolution narrows the refusal set, it never
/// waives the rule.
pub fn resolve(
    repo: &Repo,
    plan: &Plan,
    desired: &[Desired],
    previous: Option<&Manifest>,
    options: &Resolutions,
) -> Result<Resolved, ResolveError> {
    for path in &options.adopt {
        if options.force.contains(path) {
            return Err(ResolveError::Contradictory(path.clone()));
        }
    }
    for named in options.adopt.iter().chain(options.force.iter()) {
        let refused = plan
            .rows
            .iter()
            .any(|r| r.path == *named && r.action == Action::Refuse);
        if !refused {
            return Err(ResolveError::NotRefused(named.clone()));
        }
    }

    let records = previous.map(Manifest::files).unwrap_or_default();
    let record_for = |path: &str| records.iter().find(|r| r.path == path);

    let mut plan = plan.clone();
    let mut desired = desired.to_vec();
    let mut forced: Vec<String> = Vec::new();
    let mut reclassified: Vec<Reclassification> = Vec::new();

    for row in &mut plan.rows {
        if row.action != Action::Refuse {
            continue;
        }
        let markers_removed = matches!(
            row.reason,
            Some(RefuseReason::MarkersRemoved)
                | Some(RefuseReason::Region(RegionError::MarkersRemoved))
        );
        let diverged = matches!(row.reason, Some(RefuseReason::SpineOwnedDiverged));

        if options.force.contains(&row.path) {
            if markers_removed {
                return Err(ResolveError::ForceIsNotAnExit(row.path.clone()));
            }
            if !diverged {
                // Every other refusal is a fault in the render or the path, not
                // a human edit; `--force` has nothing to override there.
                return Err(ResolveError::NotRefused(row.path.clone()));
            }
            // "`--force <path>` overwrites — recorded on the upgrade line."
            row.action = Action::Update;
            row.reason = None;
            forced.push(row.path.clone());
            continue;
        }

        if options.adopt.contains(&row.path) {
            if !(diverged || markers_removed) {
                return Err(ResolveError::NotRefused(row.path.clone()));
            }
            let Some(record) = record_for(&row.path) else {
                return Err(ResolveError::AdoptWithoutRecord(row.path.clone()));
            };
            // "`--adopt <path>` reclassifies without merging", and "after which
            // spine stops writing it": the row becomes a skip forever, because
            // `user-modified` is "never rewritten silently".
            row.action = Action::Skip;
            row.reason = None;
            row.owner = Owner::UserModified;
            reclassified.push(Reclassification {
                path: row.path.clone(),
                owner: Owner::UserModified,
                // The pristine render the human diverged from is exactly what
                // the record still names: `blob` is what spine last wrote there.
                base: record.blob.clone(),
                by: By::Adopt,
            });
            continue;
        }

        if options.merge && diverged {
            let Some(record) = record_for(&row.path) else {
                // A foreign path has no recorded blob, so there is no base and
                // no three-way. `--adopt` cannot claim it either; `--force` can.
                continue;
            };
            let base = match repo.read_blob(&record.blob) {
                Some(bytes) => bytes,
                None if record.region.is_some() => {
                    return Err(ResolveError::RegionHasNoReachableBase {
                        path: row.path.clone(),
                        blob: record.blob.clone(),
                    });
                }
                None => {
                    return Err(ResolveError::BaseUnreachable {
                        path: row.path.clone(),
                        blob: record.blob.clone(),
                    });
                }
            };
            let want = desired
                .iter()
                .find(|d| d.path == row.path)
                .expect("a refusing row the plan computed from a render");
            let Some(ours) = head_bytes(repo, record) else {
                // Nothing to merge against: the path — or the region inside it —
                // is not in HEAD at all, so there is no human edit to preserve.
                continue;
            };

            let merged = match merge_file(repo, &ours, &base, &want.content)? {
                Some(bytes) => bytes,
                None => return Err(ResolveError::MergeConflict(row.path.clone())),
            };

            // "a clean merge lands and reclassifies the path `user-modified`".
            // It lands: the merge result is written, not the render — so the
            // row is an `update` whose content is no longer what the template
            // produced.
            //
            // R2: the render's blob is `hash-object --path` for a file and the
            // **unfiltered** id for a region, "because those bytes are already
            // in-blob bytes" (MF §3.5). Using the filtered form on a region
            // would record a `base` no `git ls-tree` ever produces.
            let render_blob = if record.region.is_some() {
                spine_canon::git_blob_id(&want.content, repo.object_format())
            } else {
                repo.hash_object_filtered(&record.file_path, &want.content)?
            };
            row.action = Action::Update;
            row.reason = None;
            row.owner = Owner::UserModified;
            reclassified.push(Reclassification {
                path: row.path.clone(),
                owner: Owner::UserModified,
                // "the recorded `base` blob lets `--merge` offer a three-way
                // merge" — and MF §3.5 says it is "updated on every `--merge`",
                // so it becomes *this* render: the point the human's next
                // divergence will be measured from.
                base: render_blob,
                by: By::Merge,
            });
            for entry in &mut desired {
                if entry.path == row.path {
                    entry.content = merged.clone();
                    entry.owner = Owner::UserModified;
                }
            }
        }
    }

    forced.sort_unstable();
    forced.dedup();
    Ok(Resolved {
        plan,
        desired,
        forced,
        reclassified,
    })
}

/// The three-way's "ours": the bytes at a record's path in HEAD.
///
/// For a region that is **the block between the markers**, never the host file.
/// Merging whole host files would hand the human's prose above and below the
/// block to `git merge-file` as though spine owned it, and a clean merge would
/// then rewrite prose spine never wrote.
fn head_bytes(repo: &Repo, record: &spine_manifest::FileRecord) -> Option<Vec<u8>> {
    let host = repo.read_head(&record.file_path)?;
    if record.region.is_none() {
        return Some(host);
    }
    let (name, version) = record.template.as_ref()?;
    let style = spine_manifest::region::MarkerStyle::for_template(name)?;
    let found = spine_manifest::region::find(&host, name, *version, style).ok()?;
    Some(found.bytes(&host).to_vec())
}

/// PB §6.7 step 3: "`--merge` runs `git merge-file` (base = manifest blob, ours
/// = HEAD, theirs = new render)".
///
/// `Ok(None)` is a conflict. The result is returned rather than written,
/// because "conflict markers never touch the tree" is only true if the caller
/// never sees conflicted bytes to write.
pub fn merge_file(
    repo: &Repo,
    ours: &[u8],
    base: &[u8],
    theirs: &[u8],
) -> Result<Option<Vec<u8>>, ResolveError> {
    // Scratch inside `.spine/cache/`, which `.gitignore#spine` already excludes,
    // so a crash mid-merge leaves nothing git will offer to commit.
    let dir = repo.root().join(".spine/cache/merge");
    fs::create_dir_all(&dir).map_err(|e| ResolveError::Io(e.to_string()))?;
    let write = |name: &str, bytes: &[u8]| -> Result<PathBuf, ResolveError> {
        let path = dir.join(name);
        fs::write(&path, bytes).map_err(|e| ResolveError::Io(e.to_string()))?;
        Ok(path)
    };
    let ours_path = write("ours", ours)?;
    let base_path = write("base", base)?;
    let theirs_path = write("theirs", theirs)?;

    let output = Command::new("git")
        .current_dir(repo.root())
        .args(["merge-file", "-q", "-p"])
        .args([&ours_path, &base_path, &theirs_path])
        .output()
        .map_err(|e| ResolveError::Git(GitError::NotAvailable(e.to_string())));
    let _ = fs::remove_dir_all(&dir);
    let output = output?;

    // `git merge-file` exits 0 on a clean merge and with the **number of
    // conflicts** otherwise; a negative status (255 here) is an error.
    match output.status.code() {
        Some(0) => Ok(Some(output.stdout)),
        Some(code) if (1..=127).contains(&code) => Ok(None),
        other => Err(ResolveError::Git(GitError::Unexpected(format!(
            "git merge-file exited {other:?}"
        )))),
    }
}
