//! The per-path plan — `create · update · delete · skip · REFUSE` (PB §6.7).
//!
//! `spine init` is idempotent: on an initialised repository it "renders every
//! template the binary ships using the manifest's `params`, compares blob ids,
//! and emits a per-path plan". This module is the comparison and the emission.
//!
//! **What is normative and what is derived.** `manifest.md` §12 explicitly
//! declines the plan tokens — "the plan (`create · update · delete · skip ·
//! REFUSE`) … are PB §6.7's and are not restated" — and PB §6.7 fixes only the
//! **REFUSE** triggers. So every refusal here carries a citation and every
//! non-refusal token carries a comment saying it is derived. The build plan's
//! B7 records the one derived rule that is a *write* decision rather than a
//! display choice: which paths a provider change deletes.

use spine_canon::ObjectFormat;
use spine_manifest::region::{self, MarkerStyle};
use spine_manifest::schema::Owner;

/// PB §6.7's five plan tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Create,
    Update,
    Delete,
    Skip,
    /// "Refusal is the default. One `spine-owned` path with HEAD blob ≠
    /// manifest blob stops the whole upgrade."
    Refuse,
}

impl Action {
    pub fn token(self) -> &'static str {
        match self {
            Action::Create => "create",
            Action::Update => "update",
            Action::Delete => "delete",
            Action::Skip => "skip",
            Action::Refuse => "REFUSE",
        }
    }
}

/// Why a row refuses. Every variant is a trigger PB §6.7 or a spec fixes; the
/// token is what reaches the operator and, for the landing-time ones, a wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefuseReason {
    /// PB §6.7 step 3: "One `spine-owned` path with HEAD blob ≠ manifest blob
    /// stops the whole upgrade — a partial upgrade is the interrupted case by
    /// another name." Resolution is `--merge`, `--adopt` or `--force`.
    SpineOwnedDiverged,
    /// PB §6.7: `init` "never re-creates a region whose recorded content still
    /// appears in the file without markers".
    MarkersRemoved,
    /// MF §3.7's marker rules.
    Region(region::RegionError),
    /// CI §3.4: a rendered CI file still carries `@@` or a `PIN_` literal.
    UnsubstitutedToken(String),
    /// CI §3.4: a build with no conforming `release/release.json` "renders no
    /// CI definition, writes no `.spine/manifest.json`, creates no path, and
    /// reports `REFUSE` for every row of the plan".
    NoReleaseManifest,
    /// MF §3.7: a host file whose own name contains `#`.
    PathHashAmbiguous,
}

impl core::fmt::Display for RefuseReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RefuseReason::SpineOwnedDiverged => f.write_str(
                "spine-owned path was edited by hand (exits: --merge, --adopt, --force)",
            ),
            RefuseReason::MarkersRemoved => f.write_str("markers removed"),
            RefuseReason::Region(e) => write!(f, "{e}"),
            RefuseReason::UnsubstitutedToken(t) => write!(f, "unsubstituted-token: {t}"),
            RefuseReason::NoReleaseManifest => f.write_str("no-release-manifest"),
            RefuseReason::PathHashAmbiguous => f.write_str("path-hash-ambiguous"),
        }
    }
}

/// `--status`'s per-path state column (PB §6.7, verbatim: "per path: owner ·
/// template@version · `clean | modified | missing | foreign` · planned action").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// HEAD blob equals the manifest blob.
    Clean,
    /// The path exists and its blob differs from the manifest's — a human edit.
    Modified,
    /// The manifest names it and HEAD does not have it.
    Missing,
    /// HEAD has it and no manifest record does.
    Foreign,
}

impl State {
    pub fn token(self) -> &'static str {
        match self {
            State::Clean => "clean",
            State::Modified => "modified",
            State::Missing => "missing",
            State::Foreign => "foreign",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRow {
    /// A repository path, or `<path>#<region key>`.
    pub path: String,
    pub owner: Owner,
    /// `<name>@<n>`.
    pub template: Option<String>,
    /// The blob at `path` in HEAD, or the region's blob.
    pub head_blob: Option<String>,
    /// `files[].blob` of the manifest being upgraded from.
    pub manifest_blob: Option<String>,
    /// `git hash-object --path` over the new render (no filters, for a region).
    pub render_blob: Option<String>,
    pub state: State,
    pub action: Action,
    pub reason: Option<RefuseReason>,
}

/// What the plan is computed against: the tree at HEAD.
///
/// A trait so the plan is testable without a repository, and so the collector
/// can later compute one against a tree it read from `origin/<trunk>` rather
/// than from a working copy.
pub trait TreeSource {
    /// The bytes at `path` in the tree, or `None` if absent.
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    /// The repository's object format.
    fn object_format(&self) -> ObjectFormat;
    /// `git hash-object --path <path>` over `content` — the filtered form, so
    /// `.gitattributes` and CRLF churn are not drift (PB §6.7).
    fn hash_object_filtered(&self, path: &str, content: &[u8]) -> String;
}

/// One thing `init` intends to write.
#[derive(Debug, Clone)]
pub struct Desired {
    pub path: String,
    pub owner: Owner,
    pub template: String,
    /// The rendered bytes. For a region, the region's content only.
    pub content: Vec<u8>,
}

/// The whole plan, plus the verdict PB §6.7 step 3 requires.
#[derive(Debug, Clone)]
pub struct Plan {
    pub rows: Vec<PlanRow>,
}

impl Plan {
    /// "Refusal is the default. One `spine-owned` path with HEAD blob ≠
    /// manifest blob stops the **whole** upgrade" — so the verdict is over the
    /// plan, never over a row.
    pub fn refuses(&self) -> bool {
        self.rows.iter().any(|r| r.action == Action::Refuse)
    }

    /// PB §6.7 step 2: `--dry-run` "exits 0, or 2 if it would refuse".
    pub fn exit_code(&self) -> i32 {
        if self.refuses() { 2 } else { 0 }
    }

    pub fn refusals(&self) -> impl Iterator<Item = &PlanRow> {
        self.rows.iter().filter(|r| r.action == Action::Refuse)
    }
}

/// CI §3.4: a development build "reports `REFUSE` for every row of the plan …
/// It does **not** fall back on a default host, a tag in place of a commit, an
/// empty string, or a rendered file with the token left in."
pub fn development_build_plan(desired: &[Desired]) -> Plan {
    // Sorted like every other plan: a plan a human reads should not change
    // order with the reason it refused.
    let mut rows: Vec<PlanRow> = desired
        .iter()
        .map(|d| PlanRow {
            path: d.path.clone(),
            owner: d.owner,
            template: Some(d.template.clone()),
            head_blob: None,
            manifest_blob: None,
            render_blob: None,
            state: State::Missing,
            action: Action::Refuse,
            reason: Some(RefuseReason::NoReleaseManifest),
        })
        .collect();
    rows.sort_by(|a, b| a.path.cmp(&b.path));
    Plan { rows }
}

/// Compute the plan.
///
/// `previous` is the manifest being upgraded from — `None` on a first `init`.
pub fn compute(
    tree: &dyn TreeSource,
    desired: &[Desired],
    previous: Option<&spine_manifest::Manifest>,
    template_versions: &dyn Fn(&str) -> Option<u64>,
) -> Plan {
    let mut rows: Vec<PlanRow> = Vec::new();

    for want in desired {
        rows.push(row_for(tree, want, previous, template_versions));
    }

    // B7's derived rule, and the one derived token that is a **write** decision
    // rather than a display choice: a `--ci github` -> `gitlab` re-run must
    // decide whether `.github/workflows/spine-*.yml` leaves the tree.
    //
    // Delete a record's path iff its owner is `spine-owned` and the new render
    // set does not name it. Every `user-owned` and `user-modified` path stays
    // and is reported — spine never removes bytes a human owns.
    if let Some(previous) = previous {
        for record in previous.files() {
            if desired.iter().any(|d| d.path == record.path) {
                continue;
            }
            let head_blob = blob_of(tree, &record, template_versions);
            let state = match &head_blob {
                None => State::Missing,
                Some(blob) if *blob == record.blob => State::Clean,
                Some(_) => State::Modified,
            };
            let action = match record.owner {
                Owner::SpineOwned if head_blob.is_some() => Action::Delete,
                Owner::SpineOwned => Action::Skip,
                // PB §6.7: user-owned is "never touched again — by upgrade, by
                // `--force`, or by rollback"; user-modified is "never rewritten
                // silently".
                Owner::UserOwned | Owner::UserModified => Action::Skip,
            };
            rows.push(PlanRow {
                path: record.path.clone(),
                owner: record.owner,
                template: record.template.as_ref().map(|(n, v)| format!("{n}@{v}")),
                head_blob,
                manifest_blob: Some(record.blob.clone()),
                render_blob: None,
                state,
                action,
                reason: None,
            });
        }
    }

    // Sorted by `esc`-encoded path, matching the order the manifest stores
    // `files[]` in — so the plan a human reads and the record it becomes are in
    // one order.
    rows.sort_by(|a, b| a.path.cmp(&b.path));
    Plan { rows }
}

fn row_for(
    tree: &dyn TreeSource,
    want: &Desired,
    previous: Option<&spine_manifest::Manifest>,
    template_versions: &dyn Fn(&str) -> Option<u64>,
) -> PlanRow {
    let (file_path, region_key) = spine_manifest::grammar::split_region(&want.path);

    let mut row = PlanRow {
        path: want.path.clone(),
        owner: want.owner,
        template: Some(want.template.clone()),
        head_blob: None,
        manifest_blob: None,
        render_blob: None,
        state: State::Missing,
        action: Action::Skip,
        reason: None,
    };

    // MF §3.7: a host file whose own name contains `#` cannot be recorded.
    if spine_manifest::grammar::check_recordable_path(&want.path).is_err() {
        row.action = Action::Refuse;
        row.reason = Some(RefuseReason::PathHashAmbiguous);
        return row;
    }

    let previous_record = previous
        .map(|m| m.files())
        .unwrap_or_default()
        .into_iter()
        .find(|r| r.path == want.path);
    row.manifest_blob = previous_record.as_ref().map(|r| r.blob.clone());

    // The render's blob: filtered for a file, unfiltered for a region.
    row.render_blob = Some(if region_key.is_some() {
        spine_canon::git_blob_id(&want.content, tree.object_format())
    } else {
        tree.hash_object_filtered(file_path, &want.content)
    });

    let host = tree.read(file_path);

    if let Some(region_key) = region_key {
        let _ = region_key;
        let template_name = want.template.split('@').next().unwrap_or_default();
        let Some(style) = MarkerStyle::for_template(template_name) else {
            row.action = Action::Refuse;
            row.reason = Some(RefuseReason::Region(region::RegionError::MarkersMissing));
            return row;
        };
        let expected = template_versions(template_name).unwrap_or(0);

        match host.as_deref() {
            None => {
                // The host file does not exist: `init` creates it with the
                // region in it.
                row.state = State::Missing;
                row.action = Action::Create;
            }
            Some(host_bytes) => match region::find(host_bytes, template_name, expected, style) {
                Ok(found) => {
                    let content = found.bytes(host_bytes);
                    let blob = spine_canon::git_blob_id(content, tree.object_format());
                    row.state = match &row.manifest_blob {
                        Some(recorded) if *recorded == blob => State::Clean,
                        Some(_) => State::Modified,
                        None => State::Foreign,
                    };
                    row.head_blob = Some(blob);
                    let (action, reason) = decide(&row, want.owner);
                    row.action = action;
                    row.reason = reason;
                }
                Err(region::RegionError::MarkersMissing) => {
                    // PB §6.7's "markers removed": only when the recorded
                    // content still appears. Otherwise the region is simply
                    // gone and `init` re-creates it.
                    // Only a region the manifest already recorded can have
                    // had its markers stripped; a first init has nothing to
                    // have lost.
                    let recorded_content = previous_record
                        .as_ref()
                        .map(|_| want.content.as_slice())
                        .unwrap_or_default();
                    if region::check_markers_removed(host_bytes, recorded_content).is_err() {
                        row.action = Action::Refuse;
                        row.reason = Some(RefuseReason::MarkersRemoved);
                    } else {
                        row.state = State::Missing;
                        row.action = Action::Create;
                    }
                }
                Err(other) => {
                    row.action = Action::Refuse;
                    row.reason = Some(RefuseReason::Region(other));
                }
            },
        }
        return row;
    }

    match host {
        None => {
            row.state = State::Missing;
            // Derived: `create` where the record's path is absent from HEAD.
            row.action = if want.owner == Owner::UserOwned && previous_record.is_some() {
                // A seed the human deleted is not re-seeded: "never touched
                // again" (PB §6.7).
                Action::Skip
            } else {
                Action::Create
            };
        }
        Some(bytes) => {
            let blob = tree.hash_object_filtered(file_path, &bytes);
            row.state = match &row.manifest_blob {
                Some(recorded) if *recorded == blob => State::Clean,
                Some(_) => State::Modified,
                None => State::Foreign,
            };
            row.head_blob = Some(blob);
            let (action, reason) = decide(&row, want.owner);
            row.action = action;
            row.reason = reason;
        }
    }

    row
}

/// The class rules of PB §6.7's ownership table, applied to a row whose blobs
/// are known.
///
/// Returns the reason alongside the action: a `Refuse` with no reason is a
/// refusal a human cannot act on, and PB §6.7 names an exit for every one of
/// them (`--merge`, `--adopt`, `--force`).
fn decide(row: &PlanRow, owner: Owner) -> (Action, Option<RefuseReason>) {
    match owner {
        // "Rewritten **only if** the HEAD blob equals the manifest blob. Any
        // other blob is a human edit, and the upgrade refuses."
        Owner::SpineOwned => match (&row.head_blob, &row.manifest_blob, &row.render_blob) {
            (Some(head), Some(recorded), Some(render)) if head == recorded => {
                if head == render {
                    (Action::Skip, None)
                } else {
                    (Action::Update, None)
                }
            }
            (Some(_), Some(_), _) => (Action::Refuse, Some(RefuseReason::SpineOwnedDiverged)),
            // Foreign: the path exists and no record claims it. Adopting it
            // silently would overwrite a file spine did not write, so this is
            // the same refusal by a different route.
            (Some(_), None, _) => (Action::Refuse, Some(RefuseReason::SpineOwnedDiverged)),
            _ => (Action::Create, None),
        },
        // "Never touched again — by upgrade, by `--force`, or by rollback."
        Owner::UserOwned => (Action::Skip, None),
        // "Never rewritten silently; upgrade reports 'template moved'."
        Owner::UserModified => (Action::Skip, None),
    }
}

fn blob_of(
    tree: &dyn TreeSource,
    record: &spine_manifest::FileRecord,
    template_versions: &dyn Fn(&str) -> Option<u64>,
) -> Option<String> {
    let host = tree.read(&record.file_path)?;
    match &record.region {
        None => Some(tree.hash_object_filtered(&record.file_path, &host)),
        Some(_) => {
            let (name, _) = record.template.as_ref()?;
            let style = MarkerStyle::for_template(name)?;
            let expected = template_versions(name)?;
            let found = region::find(&host, name, expected, style).ok()?;
            Some(spine_canon::git_blob_id(
                found.bytes(&host),
                tree.object_format(),
            ))
        }
    }
}

impl PlanRow {
    /// The `--status` line PB §6.7 fixes: "per path: owner · template@version ·
    /// `clean | modified | missing | foreign` · planned action".
    pub fn status_line(&self) -> String {
        format!(
            "{} · {} · {} · {}",
            self.owner.as_str(),
            self.template.as_deref().unwrap_or("-"),
            self.state.token(),
            self.action.token()
        )
    }
}
