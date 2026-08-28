//! The lifecycle paths against real git repositories: `--rollback`,
//! `--uninstall`, `--abort`, `--merge`, `--adopt`, `--force`.
//!
//! Every one of these turns on something only git knows — a file mode, a tree
//! blob at an ancestor, what `git merge-file` calls a conflict — so none of
//! them is honestly testable against a fake tree. Two of the traps here are
//! invisible from a unit test and both are checked below:
//!
//! - **The rollback compares against the blob in the tree at `<sha>`, not the
//!   record's `blob`** (MF §6.7, *On step 5*). For a `user-modified` path those
//!   are two different blobs, and only a repository holds both.
//! - **It compares the mode too.** A path whose content never changed but whose
//!   mode did is absent from every content diff and must still be restored.

use spine_canon::ObjectFormat;
use spine_init::plan::State;
use spine_init::rollback::{RestoreAction, RestoreRefusal};
use spine_init::{HeadTree, Repo, rollback, uninstall};
use spine_manifest::Manifest;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---- a scratch repository ------------------------------------------------

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Option<Self> {
        let dir = std::env::temp_dir().join(format!("spine-lifecycle-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok()?;
        git(&dir, &["init", "-q", "-b", "main", "."])?;
        git(&dir, &["config", "user.email", "t@example.invalid"])?;
        git(&dir, &["config", "user.name", "Test"])?;
        Some(Scratch(dir))
    }

    fn write(&self, path: &str, content: &str) {
        let full = self.0.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(full, content).unwrap();
    }

    fn chmod_x(&self, path: &str) {
        // Mode is the point of one of these tests, so it is set explicitly
        // rather than inherited from whatever umask the runner has.
        Command::new("chmod")
            .args(["+x", self.0.join(path).to_str().unwrap()])
            .status()
            .unwrap();
    }

    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.0.join(path)).unwrap_or_default()
    }

    fn exists(&self, path: &str) -> bool {
        self.0.join(path).exists()
    }

    fn commit(&self, message: &str) -> String {
        git(&self.0, &["add", "-A"]).unwrap();
        git(&self.0, &["commit", "-q", "-m", message]).unwrap();
        git(&self.0, &["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string()
    }

    fn mode(&self, commit: &str, path: &str) -> String {
        let out = git(&self.0, &["ls-tree", commit, "--", path]).unwrap();
        out.split(' ').next().unwrap_or_default().to_string()
    }

    fn repo(&self) -> Repo {
        Repo::discover(&self.0).unwrap()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn blob(content: &str) -> String {
    spine_canon::git_blob_id(content.as_bytes(), ObjectFormat::Sha1)
}

// ---- a manifest, canonical by construction -------------------------------

struct Rec<'a> {
    path: &'a str,
    owner: &'a str,
    blob: String,
    template: String,
    base: Option<String>,
}

/// Canonical JCS by construction: members are emitted in the order RFC 8785
/// sorts them, so `Manifest::parse` accepts it and `manifest-noncanonical`
/// never fires on the fixture itself.
fn manifest_json(version: &str, ci_version: u64, files: &[Rec], paths: &[(&str, &str)]) -> String {
    let mut records: Vec<&Rec> = files.iter().collect();
    records.sort_by_key(|r| r.path);
    let files_json: Vec<String> = records
        .iter()
        .map(|r| {
            let mut members = Vec::new();
            if let Some(base) = &r.base {
                members.push(format!("\"base\":\"{base}\""));
            }
            members.push(format!("\"blob\":\"{}\"", r.blob));
            members.push(format!("\"owner\":\"{}\"", r.owner));
            members.push(format!("\"path\":\"{}\"", r.path));
            members.push(format!("\"template\":\"{}\"", r.template));
            format!("{{{}}}", members.join(","))
        })
        .collect();
    let mut paths_sorted: Vec<&(&str, &str)> = paths.iter().collect();
    paths_sorted.sort_by_key(|(k, _)| *k);
    let paths_json: Vec<String> = paths_sorted
        .iter()
        .map(|(k, v)| format!("\"{k}\":\"{v}\""))
        .collect();
    format!(
        "{{\"cli\":{{\"dist_hash\":\"sha256:{h}\",\"version\":\"{version}\"}},\
         \"envelope\":1,\"files\":[{files}],\"manifest_version\":1,\
         \"object_format\":\"sha1\",\
         \"params\":{{\"ci\":\"generic\",\"isolation\":\"none\",\"langs\":[\"python\"],\
         \"timeout\":1800,\"trunk\":\"main\"}},\
         \"paths\":{{{paths}}},\"repo\":\"myrepo\",\
         \"resign\":{{\"intent\":1,\"intent-bug\":1,\"intent-change\":1}},\"schema\":7,\
         \"templates\":{{\"agents-block\":2,\"ci-generic\":{ci},\"ci-github-collect\":{ci},\
         \"ci-github-land\":{ci},\"ci-gitlab\":{ci},\"constitution\":1,\"gitattributes\":1,\
         \"gitignore\":1,\"intent\":2,\"intent-bug\":2,\"intent-change\":2,\"keyring\":1}}}}\n",
        h = "0".repeat(64),
        files = files_json.join(","),
        paths = paths_json.join(","),
        ci = ci_version,
    )
}

fn agents_md(body: &str) -> String {
    format!(
        "# Notes\n\nhand written above\n\n\
         <!-- spine:begin agents-block@2 -->\n{body}<!-- spine:end -->\n\n\
         hand written below\n"
    )
}

/// The two-commit repository every rollback test runs against.
///
/// `U^` is 1.3.0; `U` is the 1.3.0 → 1.4.0 upgrade. Six paths of interest:
///
/// | path | at `U^` | at `U` | why it is here |
/// |---|---|---|---|
/// | `.spine/ci.sh` | `old ci`, `100755` | `new ci`, `100644` | bytes and mode both move |
/// | `mode-only.sh` | `same`, `100755` | `same`, `100644` | **mode only** — invisible to a content diff |
/// | `unchanged.sh` | `same2` | `same2` | in `P` from the manifests, absent from the diff |
/// | `land.yml` | human's `v3`, record names the render | human's `v4` | the `user-modified` trap |
/// | `newthing.sh` | absent | present | "`git rm` for paths `U` created" |
/// | `AGENTS.md#spine` | `old region` | `new region` | a region, spliced not checked out |
/// | `CONSTITUTION.md` | `old`, `user-owned` | `new`, `user-owned` | never restored, never in `P` |
struct Fixture {
    scratch: Scratch,
    ancestor: String,
    upgrade: String,
}

fn fixture(name: &str) -> Option<Fixture> {
    let scratch = Scratch::new(name)?;

    // ---- U^ : 1.3.0 -------------------------------------------------------
    scratch.write(".spine/ci.sh", "old ci\n");
    scratch.chmod_x(".spine/ci.sh");
    scratch.write("mode-only.sh", "same\n");
    scratch.chmod_x("mode-only.sh");
    scratch.write("unchanged.sh", "same2\n");
    scratch.write("land.yml", "human tuned v3\n");
    scratch.write("AGENTS.md", &agents_md("old region\n"));
    scratch.write("CONSTITUTION.md", "old constitution\n");
    scratch.write(".spine/allowed_signers", "old keyring\n");
    scratch.write(
        ".spine/manifest.json",
        &manifest_json(
            "1.3.0",
            3,
            &[
                Rec {
                    path: ".spine/ci.sh",
                    owner: "spine-owned",
                    blob: blob("old ci\n"),
                    template: "ci-generic@3".into(),
                    base: None,
                },
                Rec {
                    path: "mode-only.sh",
                    owner: "spine-owned",
                    blob: blob("same\n"),
                    template: "ci-generic@3".into(),
                    base: None,
                },
                Rec {
                    path: "unchanged.sh",
                    owner: "spine-owned",
                    blob: blob("same2\n"),
                    template: "ci-generic@3".into(),
                    base: None,
                },
                // The `user-modified` trap: the record names the **render**
                // (`render v3`), the tree holds the human's copy.
                Rec {
                    path: "land.yml",
                    owner: "user-modified",
                    blob: blob("render v3\n"),
                    template: "ci-github-land@3".into(),
                    base: Some(blob("render v3\n")),
                },
                Rec {
                    path: "AGENTS.md#spine",
                    owner: "spine-owned",
                    blob: blob("old region\n"),
                    template: "agents-block@2".into(),
                    base: None,
                },
                Rec {
                    path: "CONSTITUTION.md",
                    owner: "user-owned",
                    blob: blob("old constitution\n"),
                    template: "constitution@1".into(),
                    base: None,
                },
                Rec {
                    path: ".spine/allowed_signers",
                    owner: "user-owned",
                    blob: blob("old keyring\n"),
                    template: "keyring@1".into(),
                    base: None,
                },
            ],
            &[("constitution", "CONSTITUTION.md")],
        ),
    );
    let ancestor = scratch.commit("1.3.0");

    // ---- U : the 1.4.0 upgrade -------------------------------------------
    scratch.write(".spine/ci.sh", "new ci\n");
    Command::new("chmod")
        .args(["644", scratch.0.join(".spine/ci.sh").to_str().unwrap()])
        .status()
        .unwrap();
    Command::new("chmod")
        .args(["644", scratch.0.join("mode-only.sh").to_str().unwrap()])
        .status()
        .unwrap();
    scratch.write("land.yml", "human tuned v4\n");
    scratch.write("newthing.sh", "created by the upgrade\n");
    scratch.write("AGENTS.md", &agents_md("new region\n"));
    scratch.write("CONSTITUTION.md", "new constitution\n");
    scratch.write(
        ".spine/manifest.json",
        &manifest_json(
            "1.4.0",
            4,
            &[
                Rec {
                    path: ".spine/ci.sh",
                    owner: "spine-owned",
                    blob: blob("new ci\n"),
                    template: "ci-generic@4".into(),
                    base: None,
                },
                Rec {
                    path: "mode-only.sh",
                    owner: "spine-owned",
                    blob: blob("same\n"),
                    template: "ci-generic@4".into(),
                    base: None,
                },
                Rec {
                    path: "unchanged.sh",
                    owner: "spine-owned",
                    blob: blob("same2\n"),
                    template: "ci-generic@4".into(),
                    base: None,
                },
                Rec {
                    path: "newthing.sh",
                    owner: "spine-owned",
                    blob: blob("created by the upgrade\n"),
                    template: "ci-generic@4".into(),
                    base: None,
                },
                Rec {
                    path: "land.yml",
                    owner: "user-modified",
                    blob: blob("render v4\n"),
                    template: "ci-github-land@4".into(),
                    base: Some(blob("render v4\n")),
                },
                Rec {
                    path: "AGENTS.md#spine",
                    owner: "spine-owned",
                    blob: blob("new region\n"),
                    template: "agents-block@2".into(),
                    base: None,
                },
                Rec {
                    path: "CONSTITUTION.md",
                    owner: "user-owned",
                    blob: blob("new constitution\n"),
                    template: "constitution@1".into(),
                    base: None,
                },
                Rec {
                    path: ".spine/allowed_signers",
                    owner: "user-owned",
                    blob: blob("old keyring\n"),
                    template: "keyring@1".into(),
                    base: None,
                },
            ],
            // `B`'s floor has grown a key since `U^` — the union must keep it.
            &[
                ("agent_context", "AGENTS.md"),
                ("constitution", "CONSTITUTION.md"),
            ],
        ),
    );
    let upgrade = scratch.commit("1.4.0");

    Some(Fixture {
        scratch,
        ancestor,
        upgrade,
    })
}

fn manifests(f: &Fixture) -> (Manifest, Manifest) {
    let repo = f.scratch.repo();
    (
        rollback::manifest_at(&repo, &f.ancestor, ObjectFormat::Sha1).unwrap(),
        rollback::manifest_at(&repo, "HEAD", ObjectFormat::Sha1).unwrap(),
    )
}

fn row<'a>(plan: &'a rollback::RollbackPlan, path: &str) -> &'a rollback::RestoreRow {
    plan.rows
        .iter()
        .find(|r| r.path == path)
        .unwrap_or_else(|| panic!("{path} is not in P"))
}

// ---- --rollback ----------------------------------------------------------

/// PB §6.7's default target: "the first-parent commit that last touched the
/// manifest", and `<sha> = U^`.
#[test]
fn the_default_target_is_the_last_commit_that_touched_the_manifest() {
    let Some(f) = fixture("locate") else { return };
    let repo = f.scratch.repo();
    let target = rollback::locate(&repo, "HEAD", None).unwrap();
    assert_eq!(target.upgrade, f.upgrade);
    assert_eq!(target.ancestor, f.ancestor);
}

/// MF §6.7 step 1: `restore-ancestor-unreachable`. A sha that is not on the
/// first-parent chain is refused rather than restored from.
#[test]
fn an_unreachable_target_is_refused() {
    let Some(f) = fixture("unreachable") else {
        return;
    };
    let repo = f.scratch.repo();
    // A commit on a side branch: reachable as an object, not first-parent.
    git(&f.scratch.0, &["checkout", "-q", "-b", "side", &f.ancestor]).unwrap();
    f.scratch.write("side.txt", "x\n");
    let side = f.scratch.commit("side");
    git(&f.scratch.0, &["checkout", "-q", "main"]).unwrap();

    assert!(matches!(
        rollback::locate(&repo, "HEAD", Some(&side)),
        Err(rollback::RollbackError::AncestorUnreachable(_))
    ));
}

/// **The trap.** MF §6.7, *On step 5*: the comparison is "against **the blob in
/// the tree at `<sha>`**, not against the record's `blob` — which is the only
/// reading that works for a `user-modified` path, whose tree blob at `<sha>` is
/// the human's copy and whose recorded `blob` is the render they diverged from."
#[test]
fn a_user_modified_path_is_restored_to_the_tree_blob_and_not_the_record_blob() {
    let Some(f) = fixture("user-modified") else {
        return;
    };
    let repo = f.scratch.repo();
    let target = rollback::locate(&repo, "HEAD", None).unwrap();
    let (ancestor, base) = manifests(&f);
    let plan = rollback::compute(&repo, &target, &ancestor, &base, &[]).unwrap();

    let land = row(&plan, "land.yml");
    let RestoreAction::Restore { oid, .. } = &land.action else {
        panic!("land.yml must be restored, got {:?}", land.action)
    };
    assert_eq!(
        *oid,
        blob("human tuned v3\n"),
        "the human's copy in the tree at <sha>"
    );
    assert_ne!(
        *oid,
        blob("render v3\n"),
        "the record's blob is the render they diverged from, and restoring it \
         would overwrite the hand-tune the rollback exists to preserve"
    );

    rollback::execute(&repo, &target, &plan).unwrap();
    assert_eq!(f.scratch.read("land.yml"), "human tuned v3\n");
}

/// MF §6.7 step 5: a restored path "exists in `T` with the same blob **and
/// mode**". `mode-only.sh` has one blob in both trees, so a blob-only check
/// calls it already-restored and leaves it `100644` for ever.
#[test]
fn a_path_whose_only_change_was_its_mode_is_still_restored() {
    let Some(f) = fixture("mode") else { return };
    let repo = f.scratch.repo();
    let target = rollback::locate(&repo, "HEAD", None).unwrap();
    let (ancestor, base) = manifests(&f);

    // The premise: one blob, two modes.
    assert_eq!(f.scratch.mode(&f.ancestor, "mode-only.sh"), "100755");
    assert_eq!(f.scratch.mode(&f.upgrade, "mode-only.sh"), "100644");

    let plan = rollback::compute(&repo, &target, &ancestor, &base, &[]).unwrap();
    let mode_only = row(&plan, "mode-only.sh");
    assert!(
        matches!(&mode_only.action, RestoreAction::Restore { mode, .. } if mode == "100755"),
        "mode is part of step 5, got {:?}",
        mode_only.action
    );

    rollback::execute(&repo, &target, &plan).unwrap();
    f.scratch.commit("rollback");
    assert_eq!(f.scratch.mode("HEAD", "mode-only.sh"), "100755");
}

/// MF §6.7: `P` is "enumerated from the manifests and never from the diff …  so
/// a path left wrongly untouched cannot pass by being absent from `diff(B, L)`".
/// `unchanged.sh` is in neither commit's diff and is still a row.
#[test]
fn a_path_absent_from_the_diff_is_still_a_row_of_p() {
    let Some(f) = fixture("from-manifests") else {
        return;
    };
    let repo = f.scratch.repo();
    let target = rollback::locate(&repo, "HEAD", None).unwrap();
    let (ancestor, base) = manifests(&f);
    let plan = rollback::compute(&repo, &target, &ancestor, &base, &[]).unwrap();

    let diff = git(
        &f.scratch.0,
        &["diff", "--name-only", &f.ancestor, &f.upgrade],
    )
    .unwrap();
    assert!(
        !diff.contains("unchanged.sh"),
        "the premise: not in the diff"
    );
    assert_eq!(
        row(&plan, "unchanged.sh").action,
        RestoreAction::AlreadyRestored
    );
}

/// PB §6.7: "`git rm` for paths `U` created".
#[test]
fn a_path_the_upgrade_created_is_deleted() {
    let Some(f) = fixture("created") else { return };
    let repo = f.scratch.repo();
    let target = rollback::locate(&repo, "HEAD", None).unwrap();
    let (ancestor, base) = manifests(&f);
    let plan = rollback::compute(&repo, &target, &ancestor, &base, &[]).unwrap();

    assert_eq!(row(&plan, "newthing.sh").action, RestoreAction::Delete);
    rollback::execute(&repo, &target, &plan).unwrap();
    assert!(!f.scratch.exists("newthing.sh"));
}

/// PB §6.7: "never a `user-owned` path: the keyring and constitution change
/// only through their own protected PRs, and a toolkit rollback is not a
/// governance rollback." MF §6.7 step 6 makes its appearance in the diff an
/// outright failure, so it must not even be a row.
#[test]
fn a_user_owned_path_is_neither_in_p_nor_touched() {
    let Some(f) = fixture("user-owned") else {
        return;
    };
    let repo = f.scratch.repo();
    let target = rollback::locate(&repo, "HEAD", None).unwrap();
    let (ancestor, base) = manifests(&f);
    let plan = rollback::compute(&repo, &target, &ancestor, &base, &[]).unwrap();

    for path in ["CONSTITUTION.md", ".spine/allowed_signers"] {
        assert!(
            !plan.rows.iter().any(|r| r.path == path),
            "{path} must not be in P"
        );
    }
    rollback::execute(&repo, &target, &plan).unwrap();
    assert_eq!(
        f.scratch.read("CONSTITUTION.md"),
        "new constitution\n",
        "a toolkit rollback is not a governance rollback"
    );
}

/// A region is spliced, not checked out: `git checkout <sha> -- AGENTS.md`
/// would revert the human's prose along with the block.
#[test]
fn a_region_is_restored_without_disturbing_its_host() {
    let Some(f) = fixture("region") else { return };
    let repo = f.scratch.repo();
    let target = rollback::locate(&repo, "HEAD", None).unwrap();
    let (ancestor, base) = manifests(&f);

    // A human edits the prose *outside* the block after the upgrade. The
    // rollback must not undo that.
    f.scratch.write(
        "AGENTS.md",
        &agents_md("new region\n").replace("hand written below", "edited below"),
    );
    f.scratch.commit("prose edit");

    let plan = rollback::compute(&repo, &target, &ancestor, &base, &[]).unwrap();
    rollback::execute(&repo, &target, &plan).unwrap();

    let host = f.scratch.read("AGENTS.md");
    assert!(host.contains("old region\n"), "the block came back");
    assert!(!host.contains("new region"), "and the new one went");
    assert!(host.contains("edited below"), "the human's prose survived");
    assert!(host.contains("hand written above"));
}

/// PB §6.7: "A path whose HEAD blob ≠ its `U` blob was modified after the
/// upgrade and is refused unless `--force`."
#[test]
fn a_path_edited_after_the_upgrade_refuses_until_forced() {
    let Some(f) = fixture("force") else { return };
    let repo = f.scratch.repo();
    let target = rollback::locate(&repo, "HEAD", None).unwrap();

    f.scratch
        .write(".spine/ci.sh", "edited after the upgrade\n");
    f.scratch.commit("hand edit");
    let (ancestor, base) = manifests(&f);

    let plan = rollback::compute(&repo, &target, &ancestor, &base, &[]).unwrap();
    assert!(plan.refuses());
    assert!(matches!(
        row(&plan, ".spine/ci.sh").refusal,
        Some(RestoreRefusal::ModifiedSinceUpgrade { .. })
    ));

    // **The refusal is executed, not merely recorded.** `execute` iterated
    // every row and acted regardless: `refuses()` existed and nothing
    // consulted it, so a human's committed work was reverted by a plan that
    // said it would not be.
    assert!(rollback::execute(&repo, &target, &plan).is_err());
    assert_eq!(
        f.scratch.read(".spine/ci.sh"),
        "edited after the upgrade\n",
        "a refused plan writes nothing"
    );

    let forced =
        rollback::compute(&repo, &target, &ancestor, &base, &[".spine/ci.sh".into()]).unwrap();
    assert!(!forced.refuses());
    rollback::execute(&repo, &target, &forced).unwrap();
    assert_eq!(f.scratch.read(".spine/ci.sh"), "old ci\n");
}

/// PB §6.7's `--force` overrides a **refusal**, so a `--force` that overrides
/// nothing is refused. Membership in `P` was the old test, and the error text
/// already described the check that was not being performed.
///
/// It is not cosmetic: a spurious `--force` carried into the landing's
/// `forced=` is `forced-disagrees` at G16 check 10 — MF §6.4, "A path in the
/// line and not in the set is a claim of an override that did not happen."
#[test]
fn forcing_a_path_the_rollback_did_not_refuse_is_refused() {
    let Some(f) = fixture("force-spurious") else {
        return;
    };
    let repo = f.scratch.repo();
    let target = rollback::locate(&repo, "HEAD", None).unwrap();
    let (ancestor, base) = manifests(&f);

    // `unchanged.sh` is in `P` and nobody touched it after the upgrade.
    let err =
        rollback::compute(&repo, &target, &ancestor, &base, &["unchanged.sh".into()]).unwrap_err();
    assert!(
        err.to_string().contains("unchanged.sh"),
        "the refusal names the path: {err}"
    );
}

/// The union is over `A` and `B`, so a `paths` key `B` gained since the upgrade
/// survives the rollback: "the floor never shrinks, not even on rollback".
#[test]
fn the_rollback_manifest_keeps_a_floor_key_the_upgrade_added() {
    let Some(f) = fixture("union") else { return };
    let (ancestor, base) = manifests(&f);
    assert_eq!(ancestor.floor_entries(), vec!["CONSTITUTION.md"]);

    let rolled = rollback::rollback_manifest(&ancestor, &base, ObjectFormat::Sha1).unwrap();
    assert_eq!(
        rolled.floor_entries(),
        vec!["AGENTS.md", "CONSTITUTION.md"],
        "B is what the floor has become since"
    );
    assert_eq!(rolled.cli_version(), "1.3.0", "and everything else is A's");
    assert_eq!(rolled.template_version("ci-generic"), Some(3));
}

// ---- --uninstall ---------------------------------------------------------

/// The owner ruling, end to end: **every** `spine-owned` path goes, whatever
/// its blob, and each deleted-but-modified one is named.
#[test]
fn uninstall_removes_every_spine_owned_path_and_names_the_modified_ones() {
    let Some(f) = fixture("uninstall") else {
        return;
    };
    let repo = f.scratch.repo();

    // A hand edit to a spine-owned path, uncommitted-then-committed, so the
    // manifest and the tree disagree.
    f.scratch.write(".spine/ci.sh", "hand tuned\n");
    f.scratch.commit("hand edit");

    let base = rollback::manifest_at(&repo, "HEAD", ObjectFormat::Sha1).unwrap();
    let tree = HeadTree { repo: &repo };
    let plan = uninstall::compute(&tree, &base).unwrap();

    let ci = plan.rows.iter().find(|r| r.path == ".spine/ci.sh").unwrap();
    assert_eq!(ci.action, uninstall::UninstallAction::Delete);
    assert_eq!(ci.state, State::Modified);
    assert_eq!(
        plan.deleted_but_modified().count(),
        1,
        "MF §6.8 is outright — no --force is documented for it"
    );

    uninstall::execute(&repo, &plan).unwrap();

    for path in [
        ".spine/ci.sh",
        "mode-only.sh",
        "unchanged.sh",
        "newthing.sh",
    ] {
        assert!(!f.scratch.exists(path), "{path} must be absent from T");
    }
    assert!(!f.scratch.exists(".spine/manifest.json"));

    // "leaves `user-owned` and `user-modified` files in place (reported)", and
    // MF §6.8's separate keyring/constitution clause.
    assert_eq!(f.scratch.read("CONSTITUTION.md"), "new constitution\n");
    assert_eq!(f.scratch.read(".spine/allowed_signers"), "old keyring\n");
    assert_eq!(f.scratch.read("land.yml"), "human tuned v4\n");
}

/// MF §6.8: "every managed region listed in `M_B` is marker-free in `T`" — and
/// MF §3.7: "The bytes that were the region may remain — an uninstall leaves the
/// human's file readable". Here the whole block goes and the prose stays.
#[test]
fn uninstall_leaves_every_managed_region_marker_free() {
    let Some(f) = fixture("uninstall-region") else {
        return;
    };
    let repo = f.scratch.repo();
    let base = rollback::manifest_at(&repo, "HEAD", ObjectFormat::Sha1).unwrap();
    let tree = HeadTree { repo: &repo };
    let plan = uninstall::compute(&tree, &base).unwrap();
    uninstall::execute(&repo, &plan).unwrap();

    let host = f.scratch.read("AGENTS.md");
    assert!(spine_manifest::region::is_marker_free(
        host.as_bytes(),
        "agents-block",
        spine_manifest::region::MarkerStyle::Html
    ));
    assert!(host.contains("hand written above"));
    assert!(host.contains("hand written below"));
    assert!(!host.contains("spine:begin"));
}

// ---- --abort -------------------------------------------------------------

/// PB §6.7: "`spine init --abort` discards instead: `git checkout` every
/// manifest path, delete created paths, delete staging. Because the tree was
/// clean before, abort is total."
#[test]
fn abort_restores_head_deletes_created_paths_and_discards_staging() {
    let Some(f) = fixture("abort") else { return };
    let repo = f.scratch.repo();

    // A crashed run: staging exists, some files are already renamed into place,
    // one path was created that HEAD does not have, and the manifest is old.
    let staging = spine_init::Staging::create(repo.root()).unwrap();
    staging
        .record_manifest(
            manifest_json(
                "1.5.0",
                5,
                &[Rec {
                    path: "created-by-the-run.sh",
                    owner: "spine-owned",
                    blob: blob("half written\n"),
                    template: "ci-generic@5".into(),
                    base: None,
                }],
                &[("constitution", "CONSTITUTION.md")],
            )
            .as_bytes(),
        )
        .unwrap();
    f.scratch.write(".spine/ci.sh", "half applied\n");
    f.scratch.write("created-by-the-run.sh", "half written\n");

    let aborted = spine_init::abort::abort(&repo).unwrap();

    assert_eq!(
        f.scratch.read(".spine/ci.sh"),
        "new ci\n",
        "restored from HEAD"
    );
    assert!(
        !f.scratch.exists("created-by-the-run.sh"),
        "a path HEAD does not have is a path the run created"
    );
    assert!(
        aborted
            .deleted
            .contains(&"created-by-the-run.sh".to_string())
    );
    assert!(aborted.staging.is_some());
    assert!(spine_init::staging::pending(repo.root()).unwrap().is_none());

    // "abort is total": nothing is left for git to report.
    assert!(repo.is_clean().unwrap(), "the tree was clean before");
}

/// The manifest itself is interrupted state 3 — "manifest new but uncommitted"
/// — and an abort takes it back to HEAD's rather than leaving a lockfile that
/// describes a run that never completed.
#[test]
fn abort_restores_the_manifest_itself() {
    let Some(f) = fixture("abort-manifest") else {
        return;
    };
    let repo = f.scratch.repo();
    let before = f.scratch.read(".spine/manifest.json");
    f.scratch.write(".spine/manifest.json", "{\"half\":1}\n");

    spine_init::abort::abort(&repo).unwrap();
    assert_eq!(f.scratch.read(".spine/manifest.json"), before);
}

// ---- --merge / --adopt / --force ------------------------------------------

/// PB §6.7 step 3: "`--merge` runs `git merge-file` (base = manifest blob, ours
/// = HEAD, theirs = new render); a clean merge lands and reclassifies the path
/// `user-modified`; a conflict refuses (conflict markers never touch the tree)."
#[test]
fn merge_file_is_three_way_and_a_conflict_yields_no_bytes() {
    let Some(scratch) = Scratch::new("merge-file") else {
        return;
    };
    scratch.write("seed", "x\n");
    scratch.commit("seed");
    let repo = scratch.repo();

    // Disjoint edits: the human touched the first line, the render the last.
    let base = b"one\ntwo\nthree\n";
    let ours = b"ONE\ntwo\nthree\n";
    let theirs = b"one\ntwo\nTHREE\n";
    assert_eq!(
        spine_init::resolve::merge_file(&repo, ours, base, theirs).unwrap(),
        Some(b"ONE\ntwo\nTHREE\n".to_vec()),
        "a clean three-way keeps both"
    );

    // The same line, two ways: a conflict.
    let ours = b"one\nHUMAN\nthree\n";
    let theirs = b"one\nRENDER\nthree\n";
    assert_eq!(
        spine_init::resolve::merge_file(&repo, ours, base, theirs).unwrap(),
        None,
        "a conflict yields no bytes at all — markers never reach the tree"
    );
}

/// A repository with one diverged `spine-owned` path, which is the state every
/// one of PB §6.7 step 3's three exits exists for.
///
/// `.spine/ci.sh` was written by spine as `one/two/three`, committed, and then
/// hand-edited on its first line. The next render moves the last line, so the
/// three-way is clean and the conflict case has to be provoked deliberately.
fn diverged(name: &str) -> Option<(Scratch, Manifest, Vec<spine_init::plan::Desired>)> {
    let scratch = Scratch::new(name)?;
    let recorded = "one\ntwo\nthree\n";
    scratch.write(".spine/ci.sh", recorded);
    scratch.write(
        ".spine/manifest.json",
        &manifest_json(
            "1.4.0",
            4,
            &[Rec {
                path: ".spine/ci.sh",
                owner: "spine-owned",
                blob: blob(recorded),
                template: "ci-generic@4".into(),
                base: None,
            }],
            &[("constitution", "CONSTITUTION.md")],
        ),
    );
    scratch.commit("install");

    // The human edit, committed — the plan compares HEAD blobs.
    scratch.write(".spine/ci.sh", "ONE\ntwo\nthree\n");
    scratch.commit("hand edit");

    let repo = scratch.repo();
    let manifest = rollback::manifest_at(&repo, "HEAD", ObjectFormat::Sha1).unwrap();
    let desired = vec![spine_init::plan::Desired {
        path: ".spine/ci.sh".into(),
        owner: spine_manifest::schema::Owner::SpineOwned,
        template: "ci-generic@4".into(),
        content: b"one\ntwo\nTHREE\n".to_vec(),
    }];
    Some((scratch, manifest, desired))
}

fn refusing_plan(
    repo: &Repo,
    desired: &[spine_init::plan::Desired],
    manifest: &Manifest,
) -> spine_init::Plan {
    let tree = HeadTree { repo };
    // The template versions come from the manifest, never from a constant: a
    // region is located by a marker carrying `templates[t]`, so a closure that
    let plan = spine_init::plan::compute(&tree, desired, Some(manifest));
    assert!(plan.refuses(), "the premise: a diverged spine-owned path");
    plan
}

/// "a clean merge lands and reclassifies the path `user-modified`" — and the
/// recorded `base` becomes this render, "updated on every `--merge`" (MF §3.5).
#[test]
fn merge_lands_the_merge_result_and_reclassifies() {
    let Some((scratch, manifest, desired)) = diverged("resolve-merge") else {
        return;
    };
    let repo = scratch.repo();
    let plan = refusing_plan(&repo, &desired, &manifest);

    let resolved = spine_init::resolve::resolve(
        &repo,
        &plan,
        &desired,
        Some(&manifest),
        &spine_init::Resolutions {
            merge: true,
            ..Default::default()
        },
    )
    .unwrap();

    assert!(!resolved.plan.refuses());
    assert_eq!(
        resolved.desired[0].content,
        b"ONE\ntwo\nTHREE\n".to_vec(),
        "what lands is the merge, not the render"
    );
    assert_eq!(resolved.reclassified.len(), 1);
    let re = &resolved.reclassified[0];
    assert_eq!(re.owner, spine_manifest::schema::Owner::UserModified);
    assert_eq!(re.by, spine_init::resolve::By::Merge);
    assert_eq!(
        re.base,
        blob("one\ntwo\nTHREE\n"),
        "base is the render the human will next diverge from"
    );
    assert!(resolved.forced.is_empty(), "a merge is not an override");
}

/// "a conflict refuses (conflict markers never touch the tree)."
#[test]
fn a_conflicting_merge_refuses_and_writes_nothing() {
    let Some((scratch, manifest, mut desired)) = diverged("resolve-conflict") else {
        return;
    };
    // The render moves the same line the human did.
    desired[0].content = b"RENDER\ntwo\nthree\n".to_vec();
    let repo = scratch.repo();
    let plan = refusing_plan(&repo, &desired, &manifest);

    let err = spine_init::resolve::resolve(
        &repo,
        &plan,
        &desired,
        Some(&manifest),
        &spine_init::Resolutions {
            merge: true,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err, spine_init::ResolveError::MergeConflict(_)));
    assert_eq!(
        scratch.read(".spine/ci.sh"),
        "ONE\ntwo\nthree\n",
        "the tree is untouched"
    );
}

/// "`--adopt <path>` reclassifies without merging", after which "spine stops
/// writing it" — so the row becomes a skip and its `base` is what spine last
/// wrote there.
#[test]
fn adopt_reclassifies_without_writing() {
    let Some((scratch, manifest, desired)) = diverged("resolve-adopt") else {
        return;
    };
    let repo = scratch.repo();
    let plan = refusing_plan(&repo, &desired, &manifest);

    let resolved = spine_init::resolve::resolve(
        &repo,
        &plan,
        &desired,
        Some(&manifest),
        &spine_init::Resolutions {
            adopt: vec![".spine/ci.sh".into()],
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(resolved.plan.rows[0].action, spine_init::Action::Skip);
    assert_eq!(resolved.reclassified[0].by, spine_init::resolve::By::Adopt);
    assert_eq!(
        resolved.reclassified[0].base,
        blob("one\ntwo\nthree\n"),
        "the pristine render the human diverged from"
    );
    assert_eq!(
        resolved.desired[0].content,
        b"one\ntwo\nTHREE\n".to_vec(),
        "adopt does not merge, so the render is untouched — and unwritten"
    );
}

/// "`--force <path>` overwrites — recorded on the upgrade line", and MF §6.4
/// derives that record from blobs, so `forced=` carries exactly the paths that
/// were actually overwritten.
#[test]
fn force_overwrites_and_lands_in_the_forced_list() {
    let Some((scratch, manifest, desired)) = diverged("resolve-force") else {
        return;
    };
    let repo = scratch.repo();
    let plan = refusing_plan(&repo, &desired, &manifest);

    let resolved = spine_init::resolve::resolve(
        &repo,
        &plan,
        &desired,
        Some(&manifest),
        &spine_init::Resolutions {
            force: vec![".spine/ci.sh".into()],
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(resolved.plan.rows[0].action, spine_init::Action::Update);
    assert_eq!(resolved.forced, vec![".spine/ci.sh".to_string()]);
    assert!(
        resolved.reclassified.is_empty(),
        "an override is not a class change"
    );

    // And the line it becomes.
    let line = spine_init::UpgradeLine {
        from: spine_init::Endpoint::parse("1.4.0").unwrap(),
        to: spine_init::Endpoint::parse("1.5.0").unwrap(),
        manifest: Some(blob("x")),
        forced: resolved.forced.clone(),
        from_manifest: None,
        since: None,
        signer: "alice@example.com".into(),
    };
    assert!(line.render().unwrap().contains(" forced=.spine/ci.sh "));
}

/// Two contradictory instructions about one path are refused rather than
/// ordered — there is no precedence rule in the corpus to appeal to.
#[test]
fn adopt_and_force_on_one_path_is_refused() {
    let Some((scratch, manifest, desired)) = diverged("resolve-both") else {
        return;
    };
    let repo = scratch.repo();
    let plan = refusing_plan(&repo, &desired, &manifest);

    assert!(matches!(
        spine_init::resolve::resolve(
            &repo,
            &plan,
            &desired,
            Some(&manifest),
            &spine_init::Resolutions {
                adopt: vec![".spine/ci.sh".into()],
                force: vec![".spine/ci.sh".into()],
                ..Default::default()
            },
        ),
        Err(spine_init::ResolveError::Contradictory(_))
    ));
}

/// A named path the plan did not refuse resolves nothing, and saying so is how
/// a typo stops looking like a resolution.
#[test]
fn resolving_a_path_that_did_not_refuse_is_refused() {
    let Some((scratch, manifest, desired)) = diverged("resolve-typo") else {
        return;
    };
    let repo = scratch.repo();
    let plan = refusing_plan(&repo, &desired, &manifest);

    assert!(matches!(
        spine_init::resolve::resolve(
            &repo,
            &plan,
            &desired,
            Some(&manifest),
            &spine_init::Resolutions {
                force: vec![".spine/ci.hs".into()],
                ..Default::default()
            },
        ),
        Err(spine_init::ResolveError::NotRefused(_))
    ));
}

/// MF §6.9's `since=`, located against a real repository: the walk is
/// first-parent, the match is on the `Spine-Upgrade` line, and a message with
/// blank lines in it — which every envelope has — does not split a record.
#[test]
fn the_uninstall_landing_is_located_by_a_first_parent_walk() {
    let Some(scratch) = Scratch::new("since") else {
        return;
    };
    scratch.write("a", "1\n");
    scratch.commit("quick: install");

    scratch.write("a", "2\n");
    git(&scratch.0, &["add", "-A"]).unwrap();
    git(
        &scratch.0,
        &[
            "commit",
            "-q",
            "-m",
            "chore: update deps\n\nSpine-Event: land\n\
             Spine-Upgrade: from=1.4.0 to=none manifest=none forced= signer=alice@example.com",
        ],
    )
    .unwrap();
    let uninstall = git(&scratch.0, &["rev-parse", "HEAD"])
        .unwrap()
        .trim()
        .to_string();

    scratch.write("a", "3\n");
    scratch.commit("quick: an ordinary landing");

    let repo = scratch.repo();
    let log = repo.first_parent_log("HEAD").unwrap();
    assert_eq!(log.len(), 3, "one record per commit, blank lines and all");
    assert_eq!(
        spine_init::upgrade::find_uninstall_landing(&log),
        Some(uninstall)
    );
}

/// `--merge` on a **managed region** has no base, and the reason is a gap in
/// the corpus rather than a limit of this crate.
///
/// PB §6.7 argues that recording git's own hash is what "makes three-way merge
/// and rollback work on an offline clone holding nothing but git objects",
/// because "the pristine content stays reachable forever through the upgrade
/// commit". For a file record that is true. For a region it is not: MF §3.5
/// records a region's `blob` as `git hash-object` over **the region's bytes**,
/// a sub-range of a host file, and git stores the host file — never the range.
/// The id names no object in any clone, so `read_blob` finds nothing and there
/// is no three-way base to merge against.
///
/// MF §6.6 already fixes what that costs: "A `base` naming an unreachable blob
/// costs `--merge`, not a landing." So the run refuses and names the two exits
/// PB §6.7 leaves — `--adopt` and `--force`. The rollback is untouched, because
/// MF §6.7.2 reads a region out of the host file at `<sha>`, which is reachable.
#[test]
fn merging_a_region_has_no_reachable_base_and_says_so() {
    let Some(scratch) = Scratch::new("resolve-region") else {
        return;
    };
    let recorded_body = "one\ntwo\nthree\n";
    scratch.write("AGENTS.md", &agents_md(recorded_body));
    scratch.write(
        ".spine/manifest.json",
        &manifest_json(
            "1.4.0",
            4,
            &[Rec {
                path: "AGENTS.md#spine",
                owner: "spine-owned",
                blob: blob(recorded_body),
                template: "agents-block@2".into(),
                base: None,
            }],
            &[("constitution", "CONSTITUTION.md")],
        ),
    );
    scratch.commit("install");

    // The premise, stated as a fact about the repository: the region's recorded
    // blob is not an object in it, while the host file's is.
    let repo = scratch.repo();
    assert!(
        repo.read_blob(&blob(recorded_body)).is_none(),
        "git never stored the region's bytes as an object"
    );
    let host_blob = git(&scratch.0, &["rev-parse", "HEAD:AGENTS.md"]).unwrap();
    assert!(
        repo.read_blob(host_blob.trim()).is_some(),
        "the host file's is"
    );

    scratch.write(
        "AGENTS.md",
        &agents_md("ONE\ntwo\nthree\n").replace("hand written below", "edited below"),
    );
    scratch.commit("hand edit");

    let manifest = rollback::manifest_at(&repo, "HEAD", ObjectFormat::Sha1).unwrap();
    let desired = vec![spine_init::plan::Desired {
        path: "AGENTS.md#spine".into(),
        owner: spine_manifest::schema::Owner::SpineOwned,
        template: "agents-block@2".into(),
        content: b"one\ntwo\nTHREE\n".to_vec(),
    }];
    let plan = refusing_plan(&repo, &desired, &manifest);

    let err = spine_init::resolve::resolve(
        &repo,
        &plan,
        &desired,
        Some(&manifest),
        &spine_init::Resolutions {
            merge: true,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            spine_init::ResolveError::RegionHasNoReachableBase { .. }
        ),
        "got {err:?}"
    );
    assert_eq!(
        scratch.read("AGENTS.md"),
        agents_md("ONE\ntwo\nthree\n").replace("hand written below", "edited below"),
        "and nothing was written"
    );

    // The exits PB §6.7 leaves both work.
    let adopted = spine_init::resolve::resolve(
        &repo,
        &plan,
        &desired,
        Some(&manifest),
        &spine_init::Resolutions {
            adopt: vec!["AGENTS.md#spine".into()],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(adopted.plan.rows[0].action, spine_init::Action::Skip);
    assert_eq!(adopted.reclassified[0].base, blob(recorded_body));
}
