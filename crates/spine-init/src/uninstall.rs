//! `spine init --uninstall` — leaving costs what arriving cost.
//!
//! PB §6.7, as amended by the owner ruling of 2026-08-27:
//!
//! > `spine init --uninstall` removes **every** `spine-owned` path the base
//! > manifest lists and every managed region, whatever their blobs — naming
//! > each deleted-but-modified path in its output — leaves `user-owned` and
//! > `user-modified` files in place (reported), removes the manifest and cache,
//! > and lands with `Spine-Upgrade: to=none`.
//!
//! **Why every path and not only the clean ones.** PB §6.7 says it itself: "It
//! removes modified paths rather than clean ones only because
//! `docs/spec/manifest.md` §6.8 makes G16's check outright — *'every
//! `spine-owned` path listed in `M_B` is absent from `T`'* — so an uninstall
//! that left one behind is an uninstall the gate refuses, with no `--force`
//! documented for it; the human's bytes stay reachable through git history,
//! which is the whole reason the manifest records blobs rather than copies."
//!
//! So this module has no `--force`, no exemption and no "clean" test on the
//! delete path. The only thing divergence changes is how loudly the row is
//! printed.

use crate::git::{GitError, Repo};
use crate::plan::{State, TreeSource};
use spine_manifest::Manifest;
use spine_manifest::region::{self, MarkerStyle};
use spine_manifest::schema::Owner;
use std::fs;

/// `.spine/cache/` — gitignored, and removed whole by the uninstall.
pub const CACHE_DIR: &str = ".spine/cache";
/// The lockfile itself. MF §6.8: "`.spine/manifest.json` is absent from `T`".
pub const MANIFEST_PATH: &str = ".spine/manifest.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UninstallAction {
    /// A `spine-owned` file: gone from `T`, whatever its blob.
    Delete,
    /// A managed region: the host file stays, the markers do not. MF §3.7:
    /// "'Absent or marker-free' means: the host file contains neither marker
    /// line for `t`. The bytes that were the region may remain — an uninstall
    /// leaves the human's file readable — and nothing checks them."
    StripRegion,
    /// `user-owned` and `user-modified` files: "left in place (reported)".
    Keep,
}

impl UninstallAction {
    pub fn token(self) -> &'static str {
        match self {
            UninstallAction::Delete => "delete",
            UninstallAction::StripRegion => "strip-region",
            UninstallAction::Keep => "keep",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UninstallRow {
    pub path: String,
    pub owner: Owner,
    pub action: UninstallAction,
    /// The record's **own template name** — the part of `template` before `@`.
    ///
    /// MF §3.7: "The region key and the template name are two different
    /// strings, and reading them as one was a defect." All three regions v1
    /// ships are keyed `spine` while their templates are `agents-block`,
    /// `gitignore` and `gitattributes`, so a marker built from the key finds
    /// nothing in any of the three hosts.
    pub template: Option<String>,
    /// `clean | modified | missing` against the record's blob. A `Modified`
    /// row on a `Delete` is what PB §6.7 requires be named in the output.
    pub state: State,
}

impl UninstallRow {
    /// PB §6.7: "naming each deleted-but-modified path in its output".
    ///
    /// The human's bytes are not lost — they stay reachable in git history —
    /// but they leave the working tree, and a silent deletion of an edit spine
    /// can see is the one thing this whole lifecycle exists to prevent.
    pub fn is_deleted_but_modified(&self) -> bool {
        self.action != UninstallAction::Keep && self.state == State::Modified
    }
}

#[derive(Debug, Clone)]
pub struct UninstallPlan {
    pub rows: Vec<UninstallRow>,
}

impl UninstallPlan {
    pub fn deleted_but_modified(&self) -> impl Iterator<Item = &UninstallRow> {
        self.rows.iter().filter(|r| r.is_deleted_but_modified())
    }
}

#[derive(Debug)]
pub enum UninstallError {
    /// A managed region recorded `user-owned`. There is no uninstall that
    /// satisfies MF §6.8 from here: stripping its markers fails
    /// `uninstall-user-owned-touched`, and leaving them fails
    /// `uninstall-region-remains`.
    ///
    /// DERIVED: the corpus never spells this record out, because a region is by
    /// definition "a block inside a file spine does not own" (MF §3.7) and the
    /// block is spine's. Refusing is the fail-closed reading; either action
    /// would produce a landing the gate refuses.
    UserOwnedRegion(String),
    /// A region record whose template has no marker style (MF §3.7's table).
    UnknownRegionTemplate(String),
    Git(GitError),
    Io(String),
}

impl core::fmt::Display for UninstallError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UninstallError::UserOwnedRegion(p) => write!(
                f,
                "{p}: a managed region recorded `user-owned` has no uninstall — \
                 stripping it fails uninstall-user-owned-touched and leaving it \
                 fails uninstall-region-remains"
            ),
            UninstallError::UnknownRegionTemplate(p) => {
                write!(
                    f,
                    "{p}: no marker style is defined for its template (MF §3.7)"
                )
            }
            UninstallError::Git(e) => write!(f, "{e}"),
            UninstallError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl core::error::Error for UninstallError {}

impl From<GitError> for UninstallError {
    fn from(e: GitError) -> Self {
        UninstallError::Git(e)
    }
}

/// The per-path plan, computed from the **base** manifest.
///
/// MF §6.8 states its checks over `M_B` and never over the tree's own guesswork:
/// the manifest is what says which paths were spine's, and a path spine wrote
/// and a human later renamed is simply absent — `Missing`, still deleted, and
/// still absent from `T`, which is all the gate asks.
pub fn compute(tree: &dyn TreeSource, base: &Manifest) -> Result<UninstallPlan, UninstallError> {
    let mut rows: Vec<UninstallRow> = Vec::new();

    for record in base.files() {
        let is_region = record.region.is_some();
        if is_region && record.owner == Owner::UserOwned {
            return Err(UninstallError::UserOwnedRegion(record.path.clone()));
        }

        let action = match (is_region, record.owner) {
            // "every managed region" — MF §6.8's clause is not qualified by
            // owner, and a `user-modified` region is still a region.
            (true, _) => UninstallAction::StripRegion,
            (false, Owner::SpineOwned) => UninstallAction::Delete,
            (false, Owner::UserOwned | Owner::UserModified) => UninstallAction::Keep,
        };

        let head_blob = current_blob(tree, &record);
        let state = match &head_blob {
            None => State::Missing,
            Some(blob) if *blob == record.blob => State::Clean,
            Some(_) => State::Modified,
        };

        rows.push(UninstallRow {
            path: record.path.clone(),
            owner: record.owner,
            action,
            template: record.template.as_ref().map(|(name, _)| name.clone()),
            state,
        });
    }

    // The manifest's own `files[]` is sorted by `esc` path (MF §3.5), so this
    // is already in that order; sorting keeps it true if a caller hands over a
    // record set from somewhere else.
    rows.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(UninstallPlan { rows })
}

/// The blob at a record's path in the tree — the region's bytes for a region,
/// the filtered file blob otherwise.
fn current_blob(tree: &dyn TreeSource, record: &spine_manifest::FileRecord) -> Option<String> {
    let host = tree.read(&record.file_path)?;
    match &record.region {
        None => Some(tree.hash_object_filtered(&record.file_path, &host)),
        Some(_) => {
            let (name, version) = record.template.as_ref()?;
            let style = MarkerStyle::for_template(name)?;
            let found = region::find(&host, name, *version, style).ok()?;
            Some(spine_canon::git_blob_id(
                found.bytes(&host),
                tree.object_format(),
            ))
        }
    }
}

/// Carry out the plan: delete, strip, then remove the manifest and the cache.
///
/// **The manifest goes last**, mirroring PB §6.7 step 4's rule for the install.
/// A crash between the deletions and the manifest leaves a manifest naming
/// paths that are gone, which is exactly interrupted state 2 read backwards and
/// is what a re-run recognises; a crash the other way round would leave a tree
/// with no record of what spine had written and nothing able to finish the job.
pub fn execute(repo: &Repo, plan: &UninstallPlan) -> Result<(), UninstallError> {
    for row in &plan.rows {
        match row.action {
            UninstallAction::Keep => {}
            UninstallAction::Delete => remove(repo, &row.path)?,
            UninstallAction::StripRegion => {
                let (file_path, _) = spine_manifest::grammar::split_region(&row.path);
                let Some(template) = row.template.clone() else {
                    return Err(UninstallError::UnknownRegionTemplate(row.path.clone()));
                };
                let Some(style) = MarkerStyle::for_template(&template) else {
                    return Err(UninstallError::UnknownRegionTemplate(row.path.clone()));
                };
                let full = repo.root().join(file_path);
                let Ok(host) = fs::read(&full) else {
                    // The host file is gone; a file that does not exist is
                    // marker-free.
                    continue;
                };
                if let Some(stripped) = strip_region(&host, &template, style) {
                    fs::write(&full, stripped).map_err(|e| UninstallError::Io(e.to_string()))?;
                }
            }
        }
    }

    remove(repo, MANIFEST_PATH)?;
    // The cache is gitignored, so git has nothing to say about it.
    let cache = repo.root().join(CACHE_DIR);
    if cache.exists() {
        fs::remove_dir_all(&cache).map_err(|e| UninstallError::Io(e.to_string()))?;
    }
    Ok(())
}

/// Remove a path from the index and the working tree.
fn remove(repo: &Repo, path: &str) -> Result<(), UninstallError> {
    repo.remove_path(path)?;
    let full = repo.root().join(path);
    if full.exists() {
        // `git rm --ignore-unmatch` is a no-op on an untracked path, and MF
        // §6.8's check is over the *result*: "absent from `T`".
        fs::remove_file(&full).map_err(|e| UninstallError::Io(e.to_string()))?;
    }
    Ok(())
}

/// The begin marker up to and including its `@`, at **any** version.
///
/// [`MarkerStyle`] keeps its own version-agnostic prefix private, so this
/// derives one from the public renderer rather than restating MF §3.7's table
/// in a second crate: a template name matches `^[a-z][a-z0-9-]{0,63}$` and
/// neither marker suffix contains `@`, so the last `@` in `begin(t, 0)` is
/// always the version's.
fn begin_prefix(style: MarkerStyle, template: &str) -> String {
    let rendered = style.begin(template, 0);
    let at = rendered
        .rfind('@')
        .expect("every begin marker carries @<n>");
    rendered[..=at].to_string()
}

/// Cut a managed region — both marker lines and everything between them — out
/// of its host file.
///
/// Returns `None` when the host is already marker-free, so a caller does not
/// rewrite a file it did not change.
///
/// **The version is not matched.** [`region::find`] takes the expected version
/// because check 9 compares it; an uninstall does not care what version the
/// markers claim, only that none of them survives (MF §3.7's "marker-free").
/// A host whose marker was hand-edited to `@99` must still come out clean.
pub fn strip_region(host: &[u8], template: &str, style: MarkerStyle) -> Option<Vec<u8>> {
    let (start, end) = block_range(host, template, style)?;
    let mut out = Vec::with_capacity(host.len());
    out.extend_from_slice(&host[..start]);
    out.extend_from_slice(&host[end..]);
    Some(out)
}

/// The byte range of a managed **block** — from the begin marker's first byte
/// through the end marker's terminating LF, markers included.
///
/// [`region::find`] returns the range *between* the markers, at one fixed
/// version; this is the range *including* them, at any version. Both callers
/// need the wider range for the same reason: an uninstall must remove the
/// marker lines themselves (MF §3.7's "marker-free"), and a rollback must
/// replace a block whose recorded version may differ on the two sides.
pub fn block_range(host: &[u8], template: &str, style: MarkerStyle) -> Option<(usize, usize)> {
    let text = core::str::from_utf8(host).ok()?;
    let begin_prefix = begin_prefix(style, template);
    let end_marker = style.end();

    let mut begin: Option<(usize, usize)> = None;
    let mut end: Option<usize> = None;
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        let trimmed = line.strip_suffix('\n').unwrap_or(line);
        if begin.is_none() && trimmed.starts_with(begin_prefix.as_str()) {
            begin = Some((offset, offset + line.len()));
        } else if begin.is_some() && end.is_none() && trimmed == end_marker {
            end = Some(offset + line.len());
        }
        offset += line.len();
    }

    let (start, begin_end) = begin?;
    // An end marker that never arrived leaves the begin line alone as the
    // block: removing more would delete bytes no marker claimed.
    Some((start, end.unwrap_or(begin_end)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use spine_canon::ObjectFormat;

    /// The uninstall's own rule: markers out, the human's prose untouched.
    #[test]
    fn stripping_a_region_leaves_every_byte_outside_the_markers() {
        let host = b"# Agent notes\n\nHand-written.\n\n\
<!-- spine:begin agents-block@2 -->\nmanaged\n<!-- spine:end -->\n";
        let stripped = strip_region(host, "agents-block", MarkerStyle::Html).unwrap();
        assert_eq!(stripped, b"# Agent notes\n\nHand-written.\n\n");
        assert!(region::is_marker_free(
            &stripped,
            "agents-block",
            MarkerStyle::Html
        ));
    }

    /// MF §3.7's marker-free rule is stated over the marker *lines*, at no
    /// particular version — so a hand-edited `@99` must still come out.
    #[test]
    fn a_marker_at_an_unexpected_version_is_still_stripped() {
        let host = b"x\n<!-- spine:begin agents-block@99 -->\nb\n<!-- spine:end -->\ny\n";
        // The version-aware locator refuses it, which is check 9's job...
        assert!(region::find(host, "agents-block", 2, MarkerStyle::Html).is_err());
        // ...and the uninstall removes it anyway, which is §6.8's.
        let stripped = strip_region(host, "agents-block", MarkerStyle::Html).unwrap();
        assert_eq!(stripped, b"x\ny\n");
    }

    /// The hash-comment style, so `.gitignore` and `.gitattributes` are not
    /// left carrying an HTML comment that is not a comment there.
    #[test]
    fn the_hash_marker_style_strips_too() {
        let host = b"node_modules/\n\n# spine:begin gitignore@1\n.spine/cache/\n# spine:end\n";
        let stripped = strip_region(host, "gitignore", MarkerStyle::Hash).unwrap();
        assert_eq!(stripped, b"node_modules/\n\n");
    }

    #[test]
    fn a_marker_free_host_is_left_alone() {
        assert_eq!(
            strip_region(b"nothing here\n", "agents-block", MarkerStyle::Html),
            None
        );
    }

    /// A fake tree, so the plan is testable without a repository.
    struct Tree(Vec<(String, Vec<u8>)>);

    impl TreeSource for Tree {
        fn read(&self, path: &str) -> Option<Vec<u8>> {
            self.0
                .iter()
                .find(|(p, _)| p == path)
                .map(|(_, b)| b.clone())
        }
        fn object_format(&self) -> ObjectFormat {
            ObjectFormat::Sha1
        }
        fn hash_object_filtered(&self, _path: &str, content: &[u8]) -> String {
            spine_canon::git_blob_id(content, ObjectFormat::Sha1)
        }
    }

    fn manifest_with(files: &str) -> Manifest {
        let json = format!(
            "{{\"cli\":{{\"dist_hash\":\"sha256:{h}\",\"version\":\"1.4.0\"}},\"envelope\":1,\
             \"files\":[{files}],\"manifest_version\":1,\"object_format\":\"sha1\",\
             \"params\":{{\"ci\":\"generic\",\"isolation\":\"none\",\"langs\":[\"python\"],\
             \"timeout\":1800,\"trunk\":\"main\"}},\
             \"paths\":{{\"constitution\":\"CONSTITUTION.md\"}},\"repo\":\"myrepo\",\
             \"resign\":{{\"intent\":1,\"intent-bug\":1,\"intent-change\":1}},\"schema\":7,\
             \"templates\":{{\"agents-block\":2,\"ci-generic\":4,\"ci-github-collect\":4,\
             \"ci-github-land\":4,\"ci-gitlab\":4,\"constitution\":1,\"gitattributes\":1,\
             \"gitignore\":1,\"intent\":2,\"intent-bug\":2,\"intent-change\":2,\"keyring\":1}}}}\n",
            h = "0".repeat(64)
        );
        Manifest::parse(json.as_bytes(), Some(ObjectFormat::Sha1)).expect("a conforming manifest")
    }

    /// The owner ruling, mechanically: a modified `spine-owned` path is deleted
    /// like any other, and named.
    #[test]
    fn a_modified_spine_owned_path_is_deleted_and_named() {
        let ci = b"#!/bin/sh\n# hand-tuned\n";
        let recorded = spine_canon::git_blob_id(b"#!/bin/sh\n", ObjectFormat::Sha1);
        let base = manifest_with(&format!(
            "{{\"blob\":\"{recorded}\",\"owner\":\"spine-owned\",\"path\":\".spine/ci.sh\",\
             \"template\":\"ci-generic@4\"}}"
        ));
        let tree = Tree(vec![(".spine/ci.sh".into(), ci.to_vec())]);
        let plan = compute(&tree, &base).unwrap();

        assert_eq!(plan.rows[0].action, UninstallAction::Delete);
        assert_eq!(plan.rows[0].state, State::Modified);
        assert_eq!(
            plan.deleted_but_modified().count(),
            1,
            "MF §6.8 is outright: an uninstall that left it behind is refused"
        );
    }

    /// "leaves `user-owned` and `user-modified` files in place (reported)".
    #[test]
    fn user_owned_and_user_modified_files_are_kept() {
        let keyring = spine_canon::git_blob_id(b"k\n", ObjectFormat::Sha1);
        let workflow = spine_canon::git_blob_id(b"w\n", ObjectFormat::Sha1);
        let base = manifest_with(&format!(
            "{{\"blob\":\"{keyring}\",\"owner\":\"user-owned\",\
             \"path\":\".spine/allowed_signers\",\"template\":\"keyring@1\"}},\
             {{\"base\":\"{workflow}\",\"blob\":\"{workflow}\",\"owner\":\"user-modified\",\
             \"path\":\"ci.yml\",\"template\":\"ci-generic@4\"}}"
        ));
        let tree = Tree(vec![
            (".spine/allowed_signers".into(), b"k\n".to_vec()),
            ("ci.yml".into(), b"w\n".to_vec()),
        ]);
        let plan = compute(&tree, &base).unwrap();
        assert!(
            plan.rows.iter().all(|r| r.action == UninstallAction::Keep),
            "the keyring and the constitution change only through their own PRs"
        );
    }

    /// A `user-owned` region has no uninstall that satisfies MF §6.8's two
    /// checks at once, so it refuses instead of picking one to fail.
    #[test]
    fn a_user_owned_region_refuses_rather_than_choosing_which_check_to_fail() {
        let blob = spine_canon::git_blob_id(b"managed\n", ObjectFormat::Sha1);
        let base = manifest_with(&format!(
            "{{\"blob\":\"{blob}\",\"owner\":\"user-owned\",\"path\":\"AGENTS.md#spine\",\
             \"template\":\"agents-block@2\"}}"
        ));
        let tree = Tree(vec![("AGENTS.md".into(), b"x\n".to_vec())]);
        assert!(matches!(
            compute(&tree, &base),
            Err(UninstallError::UserOwnedRegion(_))
        ));
    }

    /// MF §3.7: the region key and the template name are two different strings.
    /// A marker built from the key `spine` finds nothing in any of the three
    /// hosts v1 ships, so the plan carries the record's own template name.
    #[test]
    fn the_marker_is_built_from_the_template_name_and_never_the_region_key() {
        assert_eq!(
            begin_prefix(MarkerStyle::Html, "agents-block"),
            "<!-- spine:begin agents-block@"
        );
        assert_eq!(
            begin_prefix(MarkerStyle::Hash, "gitignore"),
            "# spine:begin gitignore@"
        );
        // The key, used as a template name, matches nothing.
        assert!(
            strip_region(
                b"x\n<!-- spine:begin agents-block@2 -->\nb\n<!-- spine:end -->\n",
                "spine",
                MarkerStyle::Html
            )
            .is_none()
        );

        let blob = spine_canon::git_blob_id(b"managed\n", ObjectFormat::Sha1);
        let base = manifest_with(&format!(
            "{{\"blob\":\"{blob}\",\"owner\":\"spine-owned\",\"path\":\"AGENTS.md#spine\",\
             \"template\":\"agents-block@2\"}}"
        ));
        let tree = Tree(vec![("AGENTS.md".into(), b"x\n".to_vec())]);
        let plan = compute(&tree, &base).unwrap();
        assert_eq!(plan.rows[0].template.as_deref(), Some("agents-block"));
        assert_eq!(plan.rows[0].action, UninstallAction::StripRegion);
    }
}
