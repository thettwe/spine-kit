//! `spine init`, wired end to end.
//!
//! The order is CI §3.4's and it is load-bearing: **validate the release
//! manifest before any plan is computed**, render, scan every rendered CI file
//! for a surviving token, and only then compare blobs and write. "The scan
//! precedes every write, and one failure refuses the **whole** plan rather than
//! writing the paths that happened to pass. A repository half-scaffolded by a
//! bad release is worse than one not scaffolded at all."

use spine_init::plan::{self, Action, Desired};
use spine_init::{HeadTree, Repo};
use spine_manifest::schema::Owner;
use spine_template::constitution;
use std::path::Path;

use crate::argv::Init;
use crate::exit;

/// The embedded release manifest (CI §3.4).
///
/// **There is none, and that is the specified state of this build.** CI §18
/// OPEN-1 (the distribution host) and OPEN-7 (the three GitHub Action commit
/// pins) are the owner's and are not in the corpus: "Until both are chosen no
/// release manifest can be frozen, and therefore no binary built from this
/// corpus renders a CI definition at all — which is the correct behaviour for a
/// design whose CI argument rests on a pinned release."
///
/// So this build is a **development build** and `init` refuses every plan row
/// with `no-release-manifest`. That refusal is the feature, not a gap.
const EMBEDDED_RELEASE_MANIFEST: Option<&str> = None;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn run(options: &Init) -> Result<u8> {
    let cwd = std::env::current_dir()?;
    let repo = Repo::discover(&cwd)?;

    // PB §6.7: "`--rollback`, `--uninstall` and `--status` are exempt from the
    // version gate, so an *older* binary can always back out a yanked release
    // or leave."
    if options.abort || options.rollback.is_some() || options.uninstall {
        eprintln!("spine init: that lifecycle path is not yet implemented");
        return Ok(exit::ERROR);
    }

    // The manifest's `repo`: the basename of the toplevel (owner ruling,
    // 2026-08-27 — there is no `--repo` flag). DM §5.2 builds every node id
    // from it, so it is checked against MF §3.1's grammar here rather than
    // being discovered to be wrong at G16.
    let repo_name = repo
        .default_repo_name()
        .ok_or("the repository root has no usable basename")?;
    if let Err(refusal) = spine_manifest::grammar::check_repo(repo_name) {
        eprintln!(
            "spine init: {refusal} — the manifest's `repo` is the basename of {}, \
             and it must match ^[A-Za-z0-9._-]+$ within 64 bytes",
            repo.root().display()
        );
        return Ok(exit::REFUSED);
    }

    // Build plan B3: `--trunk` has no default in the corpus; the branch HEAD
    // names is it, and a detached HEAD refuses rather than guessing.
    let trunk = match options.trunk.clone().or_else(|| repo.current_branch()) {
        Some(trunk) => trunk,
        None => {
            eprintln!(
                "spine init: HEAD is detached and --trunk was not given; \
                 params.trunk is a frozen manifest field and is not guessed"
            );
            return Ok(exit::REFUSED);
        }
    };
    if let Err(e) = spine_template::substitute::check_trunk(&trunk) {
        eprintln!("spine init: {e}");
        return Ok(exit::REFUSED);
    }

    // PB §11: "given none, `init` detects from the tree … and **refuses** when
    // it finds none rather than guessing".
    let langs = match options.langs.clone().or_else(|| detect_langs(repo.root())) {
        Some(langs) if !langs.is_empty() => langs,
        _ => {
            eprintln!(
                "spine init: no --langs given and none detected \
                 (pyproject.toml or setup.cfg => python, package.json => ts, \
                 pubspec.yaml => dart, Package.swift => swift); \
                 refusing rather than guessing"
            );
            return Ok(exit::REFUSED);
        }
    };
    for lang in &langs {
        if !spine_manifest::schema::V1_LANGS.contains(&lang.as_str()) {
            eprintln!(
                "spine init: langs-unknown: {lang:?}. v1 ships python, ts, dart and swift; \
                 kotlin was dropped because an oracle in a .java file inside a mixed module is \
                 invisible to a Kotlin resolver and nothing reports the miss"
            );
            return Ok(exit::REFUSED);
        }
    }

    // Build plan B2: `--ci` has no default either, and detecting a provider
    // from the tree is the tempting wrong answer — a stale `.gitlab-ci.yml`
    // would silently pick `gitlab` and permanently retire auto-merge
    // precondition 2 (CI §8.1).
    let Some(ci) = options.ci.clone() else {
        eprintln!(
            "spine init: --ci is required (github, gitlab or generic). \
             It is not detected from the tree: a stale .gitlab-ci.yml would silently pick \
             `gitlab` and permanently retire auto-merge precondition 2"
        );
        return Ok(exit::REFUSED);
    };

    // The render set. Only the constitution seed is rendered by this build —
    // every CI path needs the release manifest, which a development build does
    // not have.
    let owner_principal = options
        .identity
        .clone()
        .unwrap_or_else(|| "unknown@localhost".to_string());
    let lang_refs: Vec<&str> = langs.iter().map(String::as_str).collect();
    let seed = constitution::render(&constitution::Seed {
        repo: repo_name,
        owner: &owner_principal,
        langs: &lang_refs,
    });

    let mut desired = vec![Desired {
        path: "CONSTITUTION.md".into(),
        owner: Owner::UserOwned,
        template: "constitution@1".into(),
        content: seed.into_bytes(),
    }];
    desired.extend(ci_paths(&ci));

    // CI §3.4 step 1: "Validate first … **before any plan is computed**."
    let plan = match EMBEDDED_RELEASE_MANIFEST {
        None => plan::development_build_plan(&desired),
        Some(bytes) => match spine_template::ReleaseManifest::parse(bytes.as_bytes()) {
            Err(e) => {
                eprintln!("spine init: {e}");
                plan::development_build_plan(&desired)
            }
            Ok(_release) => {
                let tree = HeadTree { repo: &repo };
                plan::compute(&tree, &desired, None, &|name| template_version(name))
            }
        },
    };

    print_plan(&plan, options.status);

    if plan.refuses() {
        eprintln!();
        for row in plan.refusals() {
            eprintln!(
                "spine init: REFUSE {} — {}",
                row.path,
                row.reason
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "no reason recorded".into())
            );
        }
        if EMBEDDED_RELEASE_MANIFEST.is_none() {
            eprintln!();
            eprintln!(
                "This is a development build: no release/release.json is frozen into it, so it \
                 renders no CI definition and writes no manifest (ci.md §3.4). The distribution \
                 root and the three GitHub Action commit pins are the owner's and are still open \
                 (ci.md §18 OPEN-1, OPEN-7). Refusing the whole plan leaves the repository exactly \
                 as it was."
            );
        }
        return Ok(exit::REFUSED);
    }

    if options.dry_run || options.status {
        return Ok(exit::OK);
    }

    eprintln!("spine init: the apply path is not yet wired; re-run with --dry-run");
    Ok(exit::ERROR)
}

/// PB §11's detection table, verbatim: "`pyproject.toml` or `setup.cfg` ⇒
/// `python`, `package.json` ⇒ `ts`, `pubspec.yaml` ⇒ `dart`, `Package.swift` ⇒
/// `swift`".
///
/// Returns `None` when nothing matched, which the caller turns into a refusal —
/// PB §11 says `init` "**refuses** when it finds none rather than guessing".
fn detect_langs(root: &Path) -> Option<Vec<String>> {
    let mut found: Vec<String> = Vec::new();
    let mut add = |lang: &str| {
        if !found.iter().any(|l| l == lang) {
            found.push(lang.to_string());
        }
    };
    if root.join("pyproject.toml").exists() || root.join("setup.cfg").exists() {
        add("python");
    }
    if root.join("package.json").exists() {
        add("ts");
    }
    if root.join("pubspec.yaml").exists() {
        add("dart");
    }
    if root.join("Package.swift").exists() {
        add("swift");
    }
    // MF §3.3: `params.langs` is sorted ascending by bytes and deduplicated.
    found.sort();
    (!found.is_empty()).then_some(found)
}

/// CI §3.1's per-provider path table.
///
/// Note `ci-generic` "names the provider-independent shell, not the `generic`
/// provider": a `--ci github` repository carries `.spine/ci.sh` with
/// `"template": "ci-generic@4"`.
///
/// The bodies are empty here because a development build renders no CI
/// definition; the paths and templates are correct so the plan reports the
/// right rows.
fn ci_paths(provider: &str) -> Vec<Desired> {
    let owned = |path: &str, template: &str| Desired {
        path: path.into(),
        owner: Owner::SpineOwned,
        template: template.into(),
        content: Vec::new(),
    };
    let mut paths = vec![owned(".spine/ci.sh", "ci-generic@4")];
    match provider {
        "github" => {
            // Two files, not one: `workflow_run` selects its trigger by the
            // triggering workflow's `name:`, so a single self-named workflow
            // chains from its own completion and runs for ever (CI §3.2).
            paths.push(owned(
                ".github/workflows/spine-collect.yml",
                "ci-github-collect@4",
            ));
            paths.push(owned(".github/workflows/spine-land.yml", "ci-github-land@4"));
        }
        "gitlab" => {
            paths.push(owned(".gitlab-ci.yml", "ci-gitlab@4"));
            paths.push(owned(".spine/gitlab/untrusted.yml", "ci-gitlab@4"));
            paths.push(owned(".spine/gitlab/trusted.yml", "ci-gitlab@4"));
        }
        // `generic` writes nothing beyond `.spine/ci.sh`.
        _ => {}
    }
    paths
}

/// MF §3.6's twelve templates at their v1 versions.
fn template_version(name: &str) -> Option<u64> {
    match name {
        "agents-block" | "intent" | "intent-bug" | "intent-change" => Some(2),
        "constitution" | "gitattributes" | "gitignore" | "keyring" => Some(1),
        "ci-generic" | "ci-github-collect" | "ci-github-land" | "ci-gitlab" => Some(4),
        _ => None,
    }
}

/// PB §6.7 step 2: `--dry-run` "prints the plan"; `--status` prints "per path:
/// owner · template@version · `clean | modified | missing | foreign` · planned
/// action".
fn print_plan(plan: &plan::Plan, status: bool) {
    let width = plan
        .rows
        .iter()
        .map(|r| r.path.len())
        .max()
        .unwrap_or(0)
        .max(4);
    for row in &plan.rows {
        if status {
            println!("{:width$}  {}", row.path, row.status_line());
        } else {
            println!("{:width$}  {}", row.path, row.action.token());
        }
    }
    let refused = plan.refusals().count();
    let changed = plan
        .rows
        .iter()
        .filter(|r| matches!(r.action, Action::Create | Action::Update | Action::Delete))
        .count();
    println!();
    println!(
        "{} path(s), {} to change, {} refused",
        plan.rows.len(),
        changed,
        refused
    );
}
