//! The plan's rules, exercised against an in-memory tree.
//!
//! Only the **REFUSE** triggers are normative (MF §12 declines the other four
//! tokens by name), so the refusal tests cite PB §6.7 and the derived tests say
//! they are derived.

use spine_canon::ObjectFormat;
use spine_init::plan::{self, Action, Desired, RefuseReason, State};
use spine_manifest::schema::Owner;

/// A tree with no filters configured, so `hash-object --path` and plain
/// `hash-object` agree. That is the ordinary case; the filtered path is
/// exercised against real git in the integration tests.
struct MemTree(Vec<(String, Vec<u8>)>);

impl MemTree {
    fn new(files: &[(&str, &str)]) -> Self {
        MemTree(
            files
                .iter()
                .map(|(p, c)| (p.to_string(), c.as_bytes().to_vec()))
                .collect(),
        )
    }
}

impl plan::TreeSource for MemTree {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.0
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, c)| c.clone())
    }
    fn object_format(&self) -> ObjectFormat {
        ObjectFormat::Sha1
    }
    fn hash_object_filtered(&self, _path: &str, content: &[u8]) -> String {
        spine_canon::git_blob_id(content, ObjectFormat::Sha1)
    }
}

fn want(path: &str, owner: Owner, template: &str, content: &str) -> Desired {
    Desired {
        path: path.to_string(),
        owner,
        template: template.to_string(),
        content: content.as_bytes().to_vec(),
    }
}

fn blob(content: &str) -> String {
    spine_canon::git_blob_id(content.as_bytes(), ObjectFormat::Sha1)
}

/// A manifest carrying one `files[]` record, built by hand so the plan can be
/// tested against a known recorded blob.
fn manifest_with(records: &[(&str, Owner, &str, &str)]) -> spine_manifest::Manifest {
    let files: Vec<spine_canon::Value> = records
        .iter()
        .map(|(path, owner, template, recorded_blob)| {
            let mut members = vec![
                ("blob", spine_canon::Value::str(*recorded_blob)),
                ("owner", spine_canon::Value::str(owner.as_str())),
                ("path", spine_canon::Value::str(*path)),
                ("template", spine_canon::Value::str(*template)),
            ];
            if *owner == Owner::UserModified {
                members.push(("base", spine_canon::Value::str(*recorded_blob)));
            }
            spine_canon::Value::obj(members)
        })
        .collect();

    let value = spine_canon::Value::obj([
        (
            "cli",
            spine_canon::Value::obj([
                (
                    "dist_hash",
                    spine_canon::Value::str(
                        "sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db",
                    ),
                ),
                ("version", spine_canon::Value::str("1.4.0")),
            ]),
        ),
        ("envelope", spine_canon::Value::Int(1)),
        ("files", spine_canon::Value::arr(files)),
        ("manifest_version", spine_canon::Value::Int(1)),
        ("object_format", spine_canon::Value::str("sha1")),
        (
            "params",
            spine_canon::Value::obj([
                ("ci", spine_canon::Value::str("github")),
                ("isolation", spine_canon::Value::str("container")),
                (
                    "langs",
                    spine_canon::Value::arr([spine_canon::Value::str("python")]),
                ),
                ("timeout", spine_canon::Value::Int(1800)),
                ("trunk", spine_canon::Value::str("main")),
            ]),
        ),
        (
            "paths",
            spine_canon::Value::obj([("constitution", spine_canon::Value::str("CONSTITUTION.md"))]),
        ),
        ("repo", spine_canon::Value::str("myrepo")),
        (
            "resign",
            spine_canon::Value::obj([
                ("intent", spine_canon::Value::Int(2)),
                ("intent-bug", spine_canon::Value::Int(2)),
                ("intent-change", spine_canon::Value::Int(2)),
            ]),
        ),
        ("schema", spine_canon::Value::Int(7)),
        (
            "templates",
            spine_canon::Value::obj([
                ("agents-block", spine_canon::Value::Int(2)),
                ("ci-generic", spine_canon::Value::Int(4)),
                ("ci-github-collect", spine_canon::Value::Int(4)),
                ("ci-github-land", spine_canon::Value::Int(4)),
                ("ci-gitlab", spine_canon::Value::Int(4)),
                ("constitution", spine_canon::Value::Int(1)),
                ("gitattributes", spine_canon::Value::Int(1)),
                ("gitignore", spine_canon::Value::Int(1)),
                ("intent", spine_canon::Value::Int(2)),
                ("intent-bug", spine_canon::Value::Int(2)),
                ("intent-change", spine_canon::Value::Int(2)),
                ("keyring", spine_canon::Value::Int(1)),
            ]),
        ),
    ]);
    spine_manifest::Manifest::from_value(value, Some(ObjectFormat::Sha1))
        .expect("the fixture manifest is conforming")
}

// ---- first init ---------------------------------------------------------

/// Derived: `create` where the record's path is absent from HEAD.
#[test]
fn a_first_init_creates_every_path() {
    let tree = MemTree::new(&[]);
    let desired = vec![
        want(
            ".spine/ci.sh",
            Owner::SpineOwned,
            "ci-generic@4",
            "#!/bin/sh\n",
        ),
        want(
            "CONSTITUTION.md",
            Owner::UserOwned,
            "constitution@1",
            "# Constitution — myrepo\n",
        ),
    ];
    let plan = plan::compute(&tree, &desired, None);
    assert!(!plan.refuses());
    assert_eq!(plan.exit_code(), 0);
    assert!(plan.rows.iter().all(|r| r.action == Action::Create));
    assert!(plan.rows.iter().all(|r| r.state == State::Missing));
}

/// The plan is emitted in `esc`-path order — the order the manifest stores
/// `files[]` in, so the plan a human reads and the record it becomes agree.
#[test]
fn rows_are_sorted_by_path() {
    let tree = MemTree::new(&[]);
    let desired = vec![
        want("CONSTITUTION.md", Owner::UserOwned, "constitution@1", "x"),
        want(".spine/ci.sh", Owner::SpineOwned, "ci-generic@4", "y"),
        want(".gitignore#spine", Owner::SpineOwned, "gitignore@1", "z\n"),
    ];
    let plan = plan::compute(&tree, &desired, None);
    let paths: Vec<&str> = plan.rows.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![".gitignore#spine", ".spine/ci.sh", "CONSTITUTION.md"]
    );
}

// ---- the ownership rules ------------------------------------------------

/// PB §6.7: `spine-owned` is "rewritten **only if** the HEAD blob equals the
/// manifest blob."
#[test]
fn a_clean_spine_owned_path_updates_when_the_render_moves() {
    let old = "#!/bin/sh\nold\n";
    let new = "#!/bin/sh\nnew\n";
    let tree = MemTree::new(&[(".spine/ci.sh", old)]);
    let previous = manifest_with(&[(
        ".spine/ci.sh",
        Owner::SpineOwned,
        "ci-generic@4",
        &blob(old),
    )]);
    let desired = vec![want(".spine/ci.sh", Owner::SpineOwned, "ci-generic@4", new)];

    let plan = plan::compute(&tree, &desired, Some(&previous));
    let row = &plan.rows[0];
    assert_eq!(row.state, State::Clean);
    assert_eq!(row.action, Action::Update);
    assert!(!plan.refuses());
}

/// And skips when it does not — idempotence, which is what makes `init` safe to
/// re-run.
#[test]
fn an_unchanged_render_skips() {
    let same = "#!/bin/sh\nsame\n";
    let tree = MemTree::new(&[(".spine/ci.sh", same)]);
    let previous = manifest_with(&[(
        ".spine/ci.sh",
        Owner::SpineOwned,
        "ci-generic@4",
        &blob(same),
    )]);
    let desired = vec![want(
        ".spine/ci.sh",
        Owner::SpineOwned,
        "ci-generic@4",
        same,
    )];

    let plan = plan::compute(&tree, &desired, Some(&previous));
    assert_eq!(plan.rows[0].action, Action::Skip);
    assert_eq!(plan.rows[0].state, State::Clean);
}

/// PB §6.7 step 3, and the one rule the whole lifecycle rests on: "One
/// `spine-owned` path with HEAD blob ≠ manifest blob stops the **whole**
/// upgrade — a partial upgrade is the interrupted case by another name."
#[test]
fn one_diverged_spine_owned_path_refuses_the_whole_plan() {
    let recorded = "#!/bin/sh\nrecorded\n";
    let edited = "#!/bin/sh\nedited by a human\n";
    let tree = MemTree::new(&[
        (".spine/ci.sh", edited),
        ("CONSTITUTION.md", "# Constitution — myrepo\n"),
    ]);
    let previous = manifest_with(&[
        (
            ".spine/ci.sh",
            Owner::SpineOwned,
            "ci-generic@4",
            &blob(recorded),
        ),
        (
            "CONSTITUTION.md",
            Owner::UserOwned,
            "constitution@1",
            &blob("# Constitution — myrepo\n"),
        ),
    ]);
    let desired = vec![
        want(
            ".spine/ci.sh",
            Owner::SpineOwned,
            "ci-generic@4",
            "#!/bin/sh\nnew\n",
        ),
        want(
            "CONSTITUTION.md",
            Owner::UserOwned,
            "constitution@1",
            "# Constitution — myrepo\n",
        ),
    ];

    let plan = plan::compute(&tree, &desired, Some(&previous));
    let ci = plan.rows.iter().find(|r| r.path == ".spine/ci.sh").unwrap();
    assert_eq!(ci.state, State::Modified);
    assert_eq!(ci.action, Action::Refuse);
    assert_eq!(ci.reason, Some(RefuseReason::SpineOwnedDiverged));

    // The verdict is over the plan, not the row.
    assert!(plan.refuses());
    assert_eq!(plan.exit_code(), 2, "--dry-run exits 2 if it would refuse");
    assert_eq!(plan.refusals().count(), 1);
}

/// PB §6.7: `user-owned` is "never touched again — by upgrade, by `--force`, or
/// by rollback". `--status` reports the divergence; the plan does not act.
#[test]
fn a_user_owned_path_is_never_rewritten_however_far_it_has_drifted() {
    let seed = "# Constitution — myrepo\nVersion: v1 · Owner: alice\n";
    let edited = "# Constitution — myrepo\nVersion: v9 · Owner: alice\nlots of team rules\n";
    let tree = MemTree::new(&[("CONSTITUTION.md", edited)]);
    let previous = manifest_with(&[(
        "CONSTITUTION.md",
        Owner::UserOwned,
        "constitution@1",
        &blob(seed),
    )]);
    let desired = vec![want(
        "CONSTITUTION.md",
        Owner::UserOwned,
        "constitution@1",
        seed,
    )];

    let plan = plan::compute(&tree, &desired, Some(&previous));
    assert_eq!(plan.rows[0].state, State::Modified);
    assert_eq!(plan.rows[0].action, Action::Skip);
    assert!(
        !plan.refuses(),
        "a drifted user-owned file is not a refusal"
    );
}

/// A `user-owned` seed the human deleted is not re-seeded — "never touched
/// again" cuts both ways.
#[test]
fn a_deleted_user_owned_seed_is_not_recreated() {
    let seed = "# Constitution\n";
    let tree = MemTree::new(&[]);
    let previous = manifest_with(&[(
        "CONSTITUTION.md",
        Owner::UserOwned,
        "constitution@1",
        &blob(seed),
    )]);
    let desired = vec![want(
        "CONSTITUTION.md",
        Owner::UserOwned,
        "constitution@1",
        seed,
    )];

    let plan = plan::compute(&tree, &desired, Some(&previous));
    assert_eq!(plan.rows[0].action, Action::Skip);
    assert_eq!(plan.rows[0].state, State::Missing);
}

/// PB §6.7: `user-modified` is "never rewritten silently; upgrade reports
/// 'template moved'".
#[test]
fn a_user_modified_path_is_never_rewritten_silently() {
    let tuned = "name: spine-land\n# hand-tuned\n";
    let tree = MemTree::new(&[(".github/workflows/spine-land.yml", tuned)]);
    let previous = manifest_with(&[(
        ".github/workflows/spine-land.yml",
        Owner::UserModified,
        "ci-github-land@4",
        &blob(tuned),
    )]);
    let desired = vec![want(
        ".github/workflows/spine-land.yml",
        Owner::UserModified,
        "ci-github-land@4",
        "name: spine-land\n# the new render\n",
    )];

    let plan = plan::compute(&tree, &desired, Some(&previous));
    assert_eq!(plan.rows[0].action, Action::Skip);
    assert!(!plan.refuses());
}

/// A `spine-owned` path present in the tree that no record claims is refused
/// rather than adopted: overwriting a file spine did not write is the thing the
/// lockfile exists to prevent.
#[test]
fn a_foreign_file_at_a_spine_owned_path_refuses() {
    let tree = MemTree::new(&[(".spine/ci.sh", "someone else's script\n")]);
    let desired = vec![want(
        ".spine/ci.sh",
        Owner::SpineOwned,
        "ci-generic@4",
        "#!/bin/sh\n",
    )];

    let plan = plan::compute(&tree, &desired, None);
    assert_eq!(plan.rows[0].state, State::Foreign);
    assert_eq!(plan.rows[0].action, Action::Refuse);
    assert!(plan.refuses());
}

// ---- regions ------------------------------------------------------------

const AGENTS_REGION: &str = "This repository is governed by spine-kit.\n";

fn agents_host(region: &str) -> String {
    format!(
        "# Agent notes\n\n<!-- spine:begin agents-block@2 -->\n{region}<!-- spine:end -->\n\nHouse style.\n"
    )
}

#[test]
fn a_region_is_located_by_its_markers_and_hashed_unfiltered() {
    let host = agents_host(AGENTS_REGION);
    let tree = MemTree::new(&[("AGENTS.md", &host)]);
    let previous = manifest_with(&[(
        "AGENTS.md#spine",
        Owner::SpineOwned,
        "agents-block@2",
        &blob(AGENTS_REGION),
    )]);
    let desired = vec![want(
        "AGENTS.md#spine",
        Owner::SpineOwned,
        "agents-block@2",
        AGENTS_REGION,
    )];

    let plan = plan::compute(&tree, &desired, Some(&previous));
    let row = &plan.rows[0];
    assert_eq!(row.state, State::Clean);
    assert_eq!(row.action, Action::Skip);
    assert_eq!(
        row.head_blob.as_deref(),
        Some(blob(AGENTS_REGION).as_str()),
        "the region's blob, not the host file's"
    );
}

/// PB §6.7, verbatim: `init` "never re-creates a region whose recorded content
/// still appears in the file without markers (it refuses with 'markers
/// removed')". Without it, `init` would append a second copy of the block.
#[test]
fn stripped_markers_with_surviving_content_refuse() {
    let stripped = format!("# Agent notes\n\n{AGENTS_REGION}\nHouse style.\n");
    let tree = MemTree::new(&[("AGENTS.md", &stripped)]);
    let previous = manifest_with(&[(
        "AGENTS.md#spine",
        Owner::SpineOwned,
        "agents-block@2",
        &blob(AGENTS_REGION),
    )]);
    let desired = vec![want(
        "AGENTS.md#spine",
        Owner::SpineOwned,
        "agents-block@2",
        AGENTS_REGION,
    )];

    let plan = plan::compute(&tree, &desired, Some(&previous));
    assert_eq!(plan.rows[0].action, Action::Refuse);
    assert_eq!(plan.rows[0].reason, Some(RefuseReason::MarkersRemoved));
}

/// A host file the human genuinely deleted the block from is not that case: the
/// region is simply gone and `init` re-creates it.
#[test]
fn a_genuinely_removed_region_is_recreated() {
    let tree = MemTree::new(&[("AGENTS.md", "# Agent notes\n\nHouse style.\n")]);
    let previous = manifest_with(&[(
        "AGENTS.md#spine",
        Owner::SpineOwned,
        "agents-block@2",
        &blob(AGENTS_REGION),
    )]);
    let desired = vec![want(
        "AGENTS.md#spine",
        Owner::SpineOwned,
        "agents-block@2",
        AGENTS_REGION,
    )];

    let plan = plan::compute(&tree, &desired, Some(&previous));
    assert_eq!(plan.rows[0].action, Action::Create);
    assert!(!plan.refuses());
}

/// MF §3.7: a host file whose own name contains `#` cannot be spine-managed.
#[test]
fn a_host_file_whose_name_contains_a_hash_refuses() {
    let tree = MemTree::new(&[]);
    let desired = vec![want(
        "weird#name#spine",
        Owner::SpineOwned,
        "gitignore@1",
        "x\n",
    )];
    let plan = plan::compute(&tree, &desired, None);
    assert_eq!(plan.rows[0].action, Action::Refuse);
    assert_eq!(plan.rows[0].reason, Some(RefuseReason::PathHashAmbiguous));
}

// ---- provider change, and the development build -------------------------

/// Build plan B7's derived rule, and the one derived token that is a **write**
/// decision: a `--ci github` → `gitlab` re-run retires the GitHub workflows.
/// `user-owned` and `user-modified` paths are left alone and reported.
#[test]
fn a_provider_change_deletes_only_the_spine_owned_paths_it_retires() {
    let collect = "name: spine-collect\n";
    let land = "name: spine-land\n";
    let tree = MemTree::new(&[
        (".github/workflows/spine-collect.yml", collect),
        (".github/workflows/spine-land.yml", land),
        (".spine/ci.sh", "#!/bin/sh\n"),
    ]);
    let previous = manifest_with(&[
        (
            ".github/workflows/spine-collect.yml",
            Owner::SpineOwned,
            "ci-github-collect@4",
            &blob(collect),
        ),
        (
            ".github/workflows/spine-land.yml",
            Owner::UserModified,
            "ci-github-land@4",
            &blob(land),
        ),
        (
            ".spine/ci.sh",
            Owner::SpineOwned,
            "ci-generic@4",
            &blob("#!/bin/sh\n"),
        ),
    ]);
    // The new render set is the gitlab one: ci.sh stays, the workflows go.
    let desired = vec![
        want(
            ".spine/ci.sh",
            Owner::SpineOwned,
            "ci-generic@4",
            "#!/bin/sh\n",
        ),
        want(
            ".gitlab-ci.yml",
            Owner::SpineOwned,
            "ci-gitlab@4",
            "stages:\n",
        ),
    ];

    let plan = plan::compute(&tree, &desired, Some(&previous));
    let by = |p: &str| plan.rows.iter().find(|r| r.path == p).unwrap().action;

    assert_eq!(by(".gitlab-ci.yml"), Action::Create);
    assert_eq!(by(".spine/ci.sh"), Action::Skip);
    assert_eq!(
        by(".github/workflows/spine-collect.yml"),
        Action::Delete,
        "a spine-owned path the new render set does not name"
    );
    assert_eq!(
        by(".github/workflows/spine-land.yml"),
        Action::Skip,
        "a user-modified path is left in place and reported, never deleted"
    );
}

/// CI §3.4: a development build "renders no CI definition, writes no
/// `.spine/manifest.json`, creates no path, and reports `REFUSE` for every row
/// of the plan … It does **not** fall back on a default host, a tag in place of
/// a commit, an empty string, or a rendered file with the token left in."
#[test]
fn a_development_build_refuses_every_row() {
    let desired = vec![
        want(
            ".spine/ci.sh",
            Owner::SpineOwned,
            "ci-generic@4",
            "#!/bin/sh\n",
        ),
        want(
            "CONSTITUTION.md",
            Owner::UserOwned,
            "constitution@1",
            "# C\n",
        ),
        want(
            "AGENTS.md#spine",
            Owner::SpineOwned,
            "agents-block@2",
            "x\n",
        ),
    ];
    let plan = plan::development_build_plan(&desired);

    assert_eq!(plan.rows.len(), 3);
    assert!(plan.rows.iter().all(|r| r.action == Action::Refuse));
    assert!(
        plan.rows
            .iter()
            .all(|r| r.reason == Some(RefuseReason::NoReleaseManifest))
    );
    assert!(plan.rows.iter().all(|r| r.render_blob.is_none()));
    assert_eq!(plan.exit_code(), 2);
}

/// PB §6.7's `--status` line shape, verbatim: "per path: owner ·
/// template@version · `clean | modified | missing | foreign` · planned action".
#[test]
fn the_status_line_has_the_four_fields_pb_6_7_fixes() {
    let tree = MemTree::new(&[]);
    let desired = vec![want(
        ".spine/ci.sh",
        Owner::SpineOwned,
        "ci-generic@4",
        "#!/bin/sh\n",
    )];
    let plan = plan::compute(&tree, &desired, None);
    assert_eq!(
        plan.rows[0].status_line(),
        "spine-owned · ci-generic@4 · missing · create"
    );
}

/// **A template bump is an upgrade of a region, not a refusal.** MF §3.7: "a
/// region is located by its markers only."
///
/// The plan located the existing block with the *binary's* new version, so a
/// bump made the lookup fail for a block byte-identical to what spine wrote:
/// the row refused, one refusing row stops the whole upgrade (PB §6.7 step 3),
/// and all three documented exits were closed because `resolve` keys off
/// `SpineOwnedDiverged`. The repository could not be upgraded past a bump by
/// any documented means.
#[test]
fn a_region_template_bump_updates_rather_than_refusing() {
    // The tree carries `@2`; the release now ships `@3`.
    let host = agents_host(AGENTS_REGION);
    let tree = MemTree::new(&[("AGENTS.md", &host)]);
    let previous = manifest_with(&[(
        "AGENTS.md#spine",
        Owner::SpineOwned,
        "agents-block@2",
        &blob(AGENTS_REGION),
    )]);
    let desired = vec![want(
        "AGENTS.md#spine",
        Owner::SpineOwned,
        "agents-block@3",
        "A new agents block.\n",
    )];

    let plan = plan::compute(&tree, &desired, Some(&previous));
    let row = &plan.rows[0];
    assert_eq!(row.reason, None, "a bump is not a refusal");
    // The recorded blob still matches what is in the tree, so the path is
    // clean and the render moved — which is `update`, as it is for a whole
    // file whose template moved.
    assert_eq!(row.state, State::Clean);
    assert_eq!(row.action, Action::Update);
    assert!(!plan.refuses());
}

/// The blob comparison still decides ownership across a bump: a block a human
/// edited is `modified` and refuses, bumped or not.
#[test]
fn a_hand_edited_region_still_refuses_across_a_bump() {
    let host = agents_host("Hand-edited by a person.\n");
    let tree = MemTree::new(&[("AGENTS.md", &host)]);
    let previous = manifest_with(&[(
        "AGENTS.md#spine",
        Owner::SpineOwned,
        "agents-block@2",
        &blob(AGENTS_REGION),
    )]);
    let desired = vec![want(
        "AGENTS.md#spine",
        Owner::SpineOwned,
        "agents-block@3",
        "A new agents block.\n",
    )];

    let plan = plan::compute(&tree, &desired, Some(&previous));
    assert_eq!(plan.rows[0].state, State::Modified);
    assert_eq!(plan.rows[0].reason, Some(RefuseReason::SpineOwnedDiverged));
    assert!(plan.refuses());
}

/// MF §3.7: a region is "a block inside **a file spine does not own**", and
/// "The bytes that were the region may remain — an uninstall leaves the
/// human's file readable."
///
/// Retiring a region template gave the row `Action::Delete`, and `apply`
/// removed the **host file** — the human's whole agent-context file, which is
/// also a `paths.agent_context` floor path.
#[test]
fn a_retired_region_strips_its_block_and_keeps_its_host() {
    let host = agents_host(AGENTS_REGION);
    let tree = MemTree::new(&[("AGENTS.md", &host)]);
    let previous = manifest_with(&[(
        "AGENTS.md#spine",
        Owner::SpineOwned,
        "agents-block@2",
        &blob(AGENTS_REGION),
    )]);

    // The next render set does not name it.
    let plan = plan::compute(&tree, &[], Some(&previous));
    let row = plan
        .rows
        .iter()
        .find(|r| r.path == "AGENTS.md#spine")
        .expect("the retired record is a row");
    assert_eq!(row.action, Action::StripRegion);
    assert_ne!(row.action, Action::Delete, "the host file is not spine's");
}
