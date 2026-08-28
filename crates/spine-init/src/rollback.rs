//! `spine init --rollback [<sha>]` — revert the upgrade **by path, not by
//! trailer**.
//!
//! PB §6.7:
//!
//! > `spine init --rollback [<sha>]` locates the upgrade landing `U` (default:
//! > the first-parent commit that last touched the manifest), reads the *old*
//! > manifest from `U^`, restores every `spine-owned` and `user-modified` path
//! > listed in either manifest to its `U^` blob (`git checkout U^ -- <path>`,
//! > or `git rm` for paths `U` created) — never a `user-owned` path: the
//! > keyring and constitution change only through their own protected PRs, and
//! > a toolkit rollback is not a governance rollback — writes `U^`'s manifest
//! > with `paths.*` replaced by the union of `U^`'s and `B`'s entries (the
//! > floor never shrinks, not even on rollback, and `B` is what the floor has
//! > become since), and lands it with `Spine-Upgrade: from=<B> to=<A>`.
//!
//! Three rules in there are easy to implement wrongly, and MF §6.7 names all
//! three:
//!
//! 1. **The comparison is against the blob in the tree at `<sha>`, not the
//!    record's `blob`.** MF §6.7, *On step 5*: "which is the only reading that
//!    works for a `user-modified` path, whose tree blob at `<sha>` is the
//!    human's copy and whose recorded `blob` is the render they diverged from."
//! 2. **Mode is compared too** — step 5: "exists in `T` with the same blob
//!    **and mode**".
//! 3. **The path set comes from the two manifests, never from the diff.** MF
//!    §6.7: "A diff-driven check sees only what changed; a manifest-driven
//!    check sees what should be true", so "a path left wrongly untouched cannot
//!    pass by being absent from `diff(B, L)`".
//!
//! And one rule is easy to implement *too eagerly*: a `user-owned` path is
//! never restored, and its appearance in the diff at all fails outright (MF
//! §6.7 step 6).

use crate::git::{GitError, Repo, TreeEntry};
use spine_canon::{ObjectFormat, Value};
use spine_manifest::region::{self, MarkerStyle};
use spine_manifest::schema::Owner;
use spine_manifest::{Manifest, Refusal};

/// `.spine/manifest.json`, the path whose history locates `U`.
pub const MANIFEST_PATH: &str = ".spine/manifest.json";

/// What a rollback does to one path of `P`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreAction {
    /// The path exists in `tree(<sha>)`: restore its blob **and mode**.
    Restore { oid: String, mode: String },
    /// The path does not exist at `<sha>` — `U` created it — so it goes.
    /// PB §6.7's "`git rm` for paths `U` created".
    Delete,
    /// A region absent at `<sha>`: the host stays, marker-free.
    StripRegion,
    /// Already correct at `B`. Still enumerated, because step 5 reads the
    /// manifests and a row that is already right must be *shown* to be right.
    AlreadyRestored,
}

impl RestoreAction {
    pub fn token(&self) -> &'static str {
        match self {
            RestoreAction::Restore { .. } => "restore",
            RestoreAction::Delete => "delete",
            RestoreAction::StripRegion => "strip-region",
            RestoreAction::AlreadyRestored => "skip",
        }
    }
}

/// Why one path of `P` cannot be rolled back without `--force`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreRefusal {
    /// PB §6.7: "A path whose HEAD blob ≠ its `U` blob was modified after the
    /// upgrade and is refused unless `--force`."
    ///
    /// Note *its `U` blob* — the blob in the tree at the upgrade landing, not
    /// the record's. A human edit after the upgrade is exactly the difference
    /// between those two trees.
    ModifiedSinceUpgrade { at_u: Option<String>, at_b: Option<String> },
}

impl core::fmt::Display for RestoreRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RestoreRefusal::ModifiedSinceUpgrade { at_u, at_b } => write!(
                f,
                "modified after the upgrade ({} at U, {} at B); --force overrides",
                at_u.as_deref().unwrap_or("absent"),
                at_b.as_deref().unwrap_or("absent"),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreRow {
    /// `esc`-encoded, including any `#<region key>`.
    pub path: String,
    /// The class the **ancestor's** manifest gives it where it has one, else
    /// `B`'s. A rollback restores the class along with the bytes (MF §8.6).
    pub owner: Owner,
    /// The record's own template name, and the version the **ancestor** records
    /// for it — a rollback restores the class and the template version along
    /// with the bytes (MF §8.6).
    pub template: Option<String>,
    pub template_version: Option<u64>,
    /// Whether the path names a managed region (MF §6.7.2 reads those "as
    /// regions": marker bytes, not host bytes).
    pub is_region: bool,
    pub action: RestoreAction,
    pub refusal: Option<RestoreRefusal>,
}

#[derive(Debug, Clone)]
pub struct RollbackPlan {
    pub rows: Vec<RestoreRow>,
}

impl RollbackPlan {
    pub fn refusals(&self) -> impl Iterator<Item = &RestoreRow> {
        self.rows.iter().filter(|r| r.refusal.is_some())
    }

    pub fn refuses(&self) -> bool {
        self.rows.iter().any(|r| r.refusal.is_some())
    }
}

#[derive(Debug)]
pub enum RollbackError {
    /// No first-parent commit touches the manifest: nothing to roll back.
    NoUpgradeFound,
    /// `U` is the root commit, so `U^` does not exist. A rollback of the trust
    /// root is an uninstall, not a rollback.
    UpgradeIsRoot(String),
    /// MF §6.7 step 1: "`<sha>` is a first-parent ancestor of `B`"
    /// (`restore-ancestor-unreachable`).
    AncestorUnreachable(String),
    /// MF §6.7 step 1: "and holds a well-formed manifest"
    /// (`restore-ancestor-manifest-malformed`).
    AncestorManifestMissing(String),
    AncestorManifestMalformed { sha: String, why: String },
    /// A path named by `--force` that the plan did not refuse.
    NotRefused(String),
    Git(GitError),
    Io(String),
}

impl core::fmt::Display for RollbackError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RollbackError::NoUpgradeFound => f.write_str(
                "no first-parent commit touches .spine/manifest.json; there is no upgrade to undo",
            ),
            RollbackError::UpgradeIsRoot(sha) => write!(
                f,
                "{sha} has no first parent, so there is no U^ to restore; \
                 backing out the first init is --uninstall"
            ),
            RollbackError::AncestorUnreachable(sha) => {
                write!(f, "restore-ancestor-unreachable: {sha} is not a first-parent ancestor")
            }
            RollbackError::AncestorManifestMissing(sha) => write!(
                f,
                "restore-ancestor-manifest-malformed: {sha} holds no {MANIFEST_PATH}"
            ),
            RollbackError::AncestorManifestMalformed { sha, why } => {
                write!(f, "restore-ancestor-manifest-malformed: at {sha}: {why}")
            }
            RollbackError::NotRefused(p) => {
                write!(f, "{p}: the rollback did not refuse it, so --force overrides nothing")
            }
            RollbackError::Git(e) => write!(f, "{e}"),
            RollbackError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl core::error::Error for RollbackError {}

impl From<GitError> for RollbackError {
    fn from(e: GitError) -> Self {
        RollbackError::Git(e)
    }
}

/// The target: `U`, and the `<sha> = U^` a rollback restores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub upgrade: String,
    pub ancestor: String,
}

/// Locate the rollback target.
///
/// **This is the tool's heuristic and not the gate's rule**, and MF §6.7 says
/// so in as many words: "PB §6.7's `--rollback` default — *'the first-parent
/// commit that last touched the manifest'* — is the **tool's** heuristic for
/// choosing a target and is not the gate's rule; where they disagree, the gate
/// wins and the tool refuses." The gate locates `U` by the ledger — the newest
/// first-parent *valid landing* carrying `Spine-Upgrade` — which this crate
/// cannot evaluate, because it holds no envelope parser and no keyring check.
///
/// So the contract here is narrow: produce a candidate, verify the two
/// properties that are checkable from git alone (reachability and a well-formed
/// ancestor manifest), and leave `restore-not-one-step` to G16.
pub fn locate(repo: &Repo, from: &str, explicit: Option<&str>) -> Result<Target, RollbackError> {
    let upgrade = match explicit {
        Some(sha) => repo.rev_parse(sha)?,
        None => repo
            .first_parent_commit_touching(from, MANIFEST_PATH)?
            .ok_or(RollbackError::NoUpgradeFound)?,
    };
    if !repo.is_first_parent_ancestor(&upgrade, from)? {
        return Err(RollbackError::AncestorUnreachable(upgrade));
    }
    let ancestor = repo
        .rev_parse(&format!("{upgrade}^"))
        .map_err(|_| RollbackError::UpgradeIsRoot(upgrade.clone()))?;
    if ancestor.is_empty() {
        return Err(RollbackError::UpgradeIsRoot(upgrade));
    }
    Ok(Target { upgrade, ancestor })
}

/// Read the manifest at a commit — `A`, in MF §6.7's notation.
pub fn manifest_at(
    repo: &Repo,
    sha: &str,
    format: ObjectFormat,
) -> Result<Manifest, RollbackError> {
    let bytes = repo
        .read_at(sha, MANIFEST_PATH)
        .ok_or_else(|| RollbackError::AncestorManifestMissing(sha.to_string()))?;
    Manifest::parse(&bytes, Some(format)).map_err(|e: Refusal| {
        RollbackError::AncestorManifestMalformed {
            sha: sha.to_string(),
            why: e.to_string(),
        }
    })
}

/// MF §6.7.2's path set `P`, computed from the two manifests.
///
/// ```text
/// P := { r.path : r ∈ files(A) ∪ files(M_B),  r.owner ∈ { "spine-owned", "user-modified" } }
/// ```
///
/// "Union over **both** manifests, so a path `A` created and the upgrade
/// deleted is restored, and a path the upgrade created and `A` never had is
/// deleted. A path listed `spine-owned` in one and `user-modified` in the other
/// is in `P` once."
///
/// The result is sorted by the `esc` path, matching `files[]`'s own order.
pub fn path_set(ancestor: &Manifest, base: &Manifest) -> Vec<String> {
    let mut paths: Vec<String> = ancestor
        .files()
        .into_iter()
        .chain(base.files())
        .filter(|r| matches!(r.owner, Owner::SpineOwned | Owner::UserModified))
        .map(|r| r.path)
        .collect();
    paths.sort_unstable();
    paths.dedup();
    paths
}

/// Compute the rollback, path by path.
///
/// `forced` are `esc` paths the operator passed to `--force`.
pub fn compute(
    repo: &Repo,
    target: &Target,
    ancestor: &Manifest,
    base: &Manifest,
    forced: &[String],
) -> Result<RollbackPlan, RollbackError> {
    let ancestor_records = ancestor.files();
    let base_records = base.files();
    let mut rows: Vec<RestoreRow> = Vec::new();

    for path in path_set(ancestor, base) {
        let in_ancestor = ancestor_records.iter().find(|r| r.path == path);
        let in_base = base_records.iter().find(|r| r.path == path);
        // "The rollback restores the class along with the bytes, which is what a
        // rollback is" (MF §8.6) — so the ancestor's class wins where it has
        // one, and `B`'s stands only for a path the ancestor never named.
        let record = in_ancestor.or(in_base).expect("P is built from the two");
        let (file_path, is_region) = {
            let (file, key) = spine_manifest::grammar::split_region(&path);
            (file.to_string(), key.is_some())
        };
        let template = record.template.as_ref().map(|(n, _)| n.clone());
        let template_version = record.template.as_ref().map(|(_, v)| *v);

        // The three tree reads: at `<sha>` (what to restore), at `U` (what the
        // upgrade left), at `B` (what is there now).
        let at_ancestor = tree_state(repo, &target.ancestor, &file_path, is_region, record)?;
        let at_upgrade = tree_state(repo, &target.upgrade, &file_path, is_region, record)?;
        let at_base = tree_state(repo, "HEAD", &file_path, is_region, record)?;

        // PB §6.7: "A path whose HEAD blob ≠ its `U` blob was modified after the
        // upgrade and is refused unless `--force`."
        let modified_since = at_base.as_ref().map(|s| &s.oid) != at_upgrade.as_ref().map(|s| &s.oid);
        let refusal = if modified_since && !forced.contains(&path) {
            Some(RestoreRefusal::ModifiedSinceUpgrade {
                at_u: at_upgrade.as_ref().map(|s| s.oid.clone()),
                at_b: at_base.as_ref().map(|s| s.oid.clone()),
            })
        } else {
            None
        };

        // MF §6.7 step 5 compares blob **and mode** — for a file. A region's
        // test is stated over bytes alone ("the region bytes in `T` hash to the
        // region bytes at `<sha>`"), and the host file it lives in is not a
        // spine path, so its mode is not a rollback's to restore.
        // DERIVED: mode is compared for a file and not for a region, which is
        // the only reading under which a rollback can satisfy its own check.
        let action = match (&at_ancestor, &at_base) {
            (Some(want), Some(have))
                if want.oid == have.oid && (is_region || want.mode == have.mode) =>
            {
                RestoreAction::AlreadyRestored
            }
            (Some(want), _) => RestoreAction::Restore {
                oid: want.oid.clone(),
                mode: want.mode.clone(),
            },
            (None, None) => RestoreAction::AlreadyRestored,
            (None, Some(_)) if is_region => RestoreAction::StripRegion,
            (None, Some(_)) => RestoreAction::Delete,
        };

        rows.push(RestoreRow {
            path,
            owner: record.owner,
            template,
            template_version,
            is_region,
            action,
            refusal,
        });
    }

    for path in forced {
        if !rows.iter().any(|r| r.path == *path) {
            return Err(RollbackError::NotRefused(path.clone()));
        }
    }

    Ok(RollbackPlan { rows })
}

/// The blob id and mode of one member of `P` in one tree.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeState {
    oid: String,
    mode: String,
}

fn tree_state(
    repo: &Repo,
    commit: &str,
    file_path: &str,
    is_region: bool,
    record: &spine_manifest::FileRecord,
) -> Result<Option<TreeState>, RollbackError> {
    let Some(TreeEntry { mode, oid }) = repo.ls_tree(commit, file_path)? else {
        return Ok(None);
    };
    if !is_region {
        return Ok(Some(TreeState { oid, mode }));
    }
    // MF §6.7.2: "Managed regions are members of `P` under their `path#region`
    // spelling, and step 5 reads them as regions: 'same blob' means the region
    // bytes in `T` hash to the region bytes at `<sha>`."
    let Some(host) = repo.read_at(commit, file_path) else {
        return Ok(None);
    };
    let Some((name, version)) = record.template.as_ref() else {
        return Ok(None);
    };
    let Some(style) = MarkerStyle::for_template(name) else {
        return Ok(None);
    };
    let Ok(found) = region::find(&host, name, *version, style) else {
        // No region there: "absent", which for a region means marker-free.
        return Ok(None);
    };
    Ok(Some(TreeState {
        // A region's bytes are already in-blob bytes (MF §3.5), so no filter.
        oid: spine_canon::git_blob_id(found.bytes(&host), repo.object_format()),
        mode,
    }))
}

/// MF §6.7.1's monotone union, over the two manifests' `paths` maps.
///
/// ```text
/// keys(M_T.paths) = keys(A.paths) ∪ keys(M_B.paths)
/// for every k :  values(M_T.paths[k]) = values(A.paths[k]) ∪ values(M_B.paths[k])
/// ```
///
/// with an absent key contributing the empty set, and each result in MF §3.4's
/// canonical shape: "A key with exactly one entry is written as a **string**; a
/// key with two or more is written as an **array**, sorted ascending by `esc`
/// bytes, with no duplicates."
///
/// "The floor never shrinks, not even backwards" — which is why this is a union
/// and not a copy of `A`, even though every other member of the rollback's
/// manifest is exactly `A`'s.
pub fn monotone_union(ancestor: &Manifest, base: &Manifest) -> Value {
    let entries = |m: &Manifest| -> Vec<(String, Vec<String>)> {
        let Some(Value::Obj(members)) = m.value().get("paths") else {
            return Vec::new();
        };
        members
            .iter()
            .map(|(key, value)| {
                let values = match value {
                    Value::Str(s) => vec![s.clone()],
                    Value::Arr(items) => items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect(),
                    _ => Vec::new(),
                };
                (key.clone(), values)
            })
            .collect()
    };

    let mut union: Vec<(String, Vec<String>)> = Vec::new();
    for (key, values) in entries(ancestor).into_iter().chain(entries(base)) {
        match union.iter_mut().find(|(k, _)| *k == key) {
            Some((_, existing)) => existing.extend(values),
            None => union.push((key, values)),
        }
    }

    let mut members: Vec<(String, Value)> = Vec::with_capacity(union.len());
    for (key, mut values) in union {
        // "sorted ascending by `esc` bytes, with no duplicates" — the stored
        // strings *are* the `esc` spelling (MF §3.4), so this is a byte sort
        // over them and not over any second encoding.
        values.sort_unstable_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        values.dedup();
        let value = match values.len() {
            1 => Value::Str(values.remove(0)),
            _ => Value::Arr(values.into_iter().map(Value::Str).collect()),
        };
        members.push((key, value));
    }
    // JCS sorts members on output, so insertion order here is immaterial; it is
    // sorted anyway so a caller comparing two `Value`s directly sees one shape.
    members.sort_by(|a, b| a.0.cmp(&b.0));
    Value::Obj(members)
}

/// `M_T` — the manifest a rollback writes.
///
/// MF §6.7 step 3 is one comparison of canonical bytes: `eq(M_T with paths
/// removed, A with paths removed)`. It is "stronger than PB §7.5's *'every
/// frozen field and every `files[]` record'*" and it "closes a real hole: under
/// the literal one, a rollback could restore every frozen field and every
/// `files[]` record while quietly lowering `resign`, dropping a `templates` key,
/// or renaming `repo` — the last of which changes every node id in the graph."
///
/// So this is built by *taking `A` whole* and replacing one member, never by
/// copying fields across.
pub fn rollback_manifest(
    ancestor: &Manifest,
    base: &Manifest,
    format: ObjectFormat,
) -> Result<Manifest, Refusal> {
    let Value::Obj(members) = ancestor.value().clone() else {
        unreachable!("a parsed manifest's root is an object")
    };
    let paths = monotone_union(ancestor, base);
    let members: Vec<(String, Value)> = members
        .into_iter()
        .map(|(key, value)| {
            if key == "paths" {
                (key, paths.clone())
            } else {
                (key, value)
            }
        })
        .collect();
    Manifest::from_value(Value::Obj(members), Some(format))
}

/// Carry out the plan.
///
/// `git checkout <sha> -- <path>` is PB §6.7's own verb, and it is used rather
/// than a byte write because it restores the **mode** alongside the blob, which
/// MF §6.7 step 5 compares.
pub fn execute(
    repo: &Repo,
    target: &Target,
    plan: &RollbackPlan,
) -> Result<(), RollbackError> {
    for row in &plan.rows {
        let (file_path, _) = spine_manifest::grammar::split_region(&row.path);
        match &row.action {
            RestoreAction::AlreadyRestored => {}
            // A region's host file is not a spine path: `git checkout <sha> --
            // AGENTS.md` would revert the human's prose above and below the
            // block along with it. Only the block moves.
            RestoreAction::Restore { .. } if row.is_region => {
                restore_region(repo, target, row, file_path)?;
            }
            RestoreAction::Restore { .. } => {
                repo.checkout_path(&target.ancestor, file_path)?;
            }
            RestoreAction::Delete => {
                repo.remove_path(file_path)?;
                let full = repo.root().join(file_path);
                if full.exists() {
                    std::fs::remove_file(&full).map_err(|e| RollbackError::Io(e.to_string()))?;
                }
            }
            RestoreAction::StripRegion => {
                let Some(template) = row.template.clone() else {
                    continue;
                };
                let Some(style) = MarkerStyle::for_template(&template) else {
                    continue;
                };
                let full = repo.root().join(file_path);
                let Ok(host) = std::fs::read(&full) else {
                    continue;
                };
                if let Some(stripped) = crate::uninstall::strip_region(&host, &template, style) {
                    std::fs::write(&full, stripped)
                        .map_err(|e| RollbackError::Io(e.to_string()))?;
                }
            }
        }
    }
    Ok(())
}

/// Put the block that stood at `<sha>` back where the current block stands.
///
/// The two blocks may carry **different versions** — that is what a template
/// bump is, and rolling one back is the ordinary case — so the current block is
/// located by [`crate::uninstall::block_range`] (version-agnostic) rather than
/// by [`region::find`] (which refuses a version mismatch), and the ancestor's
/// block is copied across whole, marker lines included. Splicing in place
/// rather than appending keeps the human's prose on both sides of it.
fn restore_region(
    repo: &Repo,
    target: &Target,
    row: &RestoreRow,
    file_path: &str,
) -> Result<(), RollbackError> {
    let (Some(template), Some(version)) = (row.template.clone(), row.template_version) else {
        return Ok(());
    };
    let Some(style) = MarkerStyle::for_template(&template) else {
        return Ok(());
    };
    let Some(host_at_sha) = repo.read_at(&target.ancestor, file_path) else {
        return Ok(());
    };
    let Some((start, end)) = crate::uninstall::block_range(&host_at_sha, &template, style) else {
        return Ok(());
    };
    let block = &host_at_sha[start..end];

    let full = repo.root().join(file_path);
    let host_now = std::fs::read(&full).unwrap_or_default();
    let restored = match crate::uninstall::block_range(&host_now, &template, style) {
        Some((now_start, now_end)) => {
            let mut out = Vec::with_capacity(host_now.len() + block.len());
            out.extend_from_slice(&host_now[..now_start]);
            out.extend_from_slice(block);
            out.extend_from_slice(&host_now[now_end..]);
            out
        }
        None => {
            // No block to replace: the upgrade had removed it, so it is created
            // the way `init` creates one — appended, because "a `.gitignore`'s
            // later rules can override earlier ones".
            let body = core::str::from_utf8(
                region::find(&host_at_sha, &template, version, style)
                    .map(|found| found.bytes(&host_at_sha))
                    .unwrap_or_default(),
            )
            .map_err(|_| RollbackError::Io("a region body must be UTF-8".into()))?;
            spine_template::regions::create_in(&host_now, &template, version, body)
                .ok_or_else(|| RollbackError::Io(format!("{file_path}: no marker style")))?
        }
    };
    std::fs::write(&full, restored).map_err(|e| RollbackError::Io(e.to_string()))
}
