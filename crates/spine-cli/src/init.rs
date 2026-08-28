//! `spine init`, wired end to end.
//!
//! The order is CI §3.4's and it is load-bearing: **validate the release
//! manifest before any plan is computed**, render, scan every rendered CI file
//! for a surviving token, and only then compare blobs and write. "The scan
//! precedes every write, and one failure refuses the **whole** plan rather than
//! writing the paths that happened to pass. A repository half-scaffolded by a
//! bad release is worse than one not scaffolded at all."

use spine_init::plan::{self, Action, Desired, TreeSource};
use spine_init::{HeadTree, Repo};
use spine_manifest::schema::Owner;
use spine_template::constitution;
use std::path::Path;

use crate::argv::Init;
use crate::exit;

/// The embedded release manifest (CI §3.4).
///
/// **Ordinarily there is none, and that is the specified state of this build.**
/// CI §18 OPEN-1 (the distribution host) and OPEN-7 (the three GitHub Action
/// commit pins) are the owner's and are not in the corpus: "Until both are
/// chosen no release manifest can be frozen, and therefore no binary built from
/// this corpus renders a CI definition at all — which is the correct behaviour
/// for a design whose CI argument rests on a pinned release."
///
/// So the default build is a **development build** and `init` refuses every
/// plan row with `no-release-manifest`. That refusal is the feature, not a gap.
///
/// The `synthetic-release` feature compiles in `release/release.synthetic.json`
/// instead, whose every value is deliberately unusable — a `.invalid` host and
/// three commits that exist in no repository — so the apply path can be
/// exercised without anything pretending to be a release.
///
/// **A feature and not an environment variable**, because CI §3.4 makes this a
/// build input read once and frozen: "Nothing at run time re-reads it from
/// disk, so a repository cannot supply one and a candidate cannot forge one." A
/// runtime override would hand a candidate the one input the whole
/// trusted-execution argument rests on.
#[cfg(not(feature = "synthetic-release"))]
const EMBEDDED_RELEASE_MANIFEST: Option<&str> = None;

#[cfg(feature = "synthetic-release")]
const EMBEDDED_RELEASE_MANIFEST: Option<&str> =
    Some(include_str!("../../../release/release.synthetic.json"));

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

    // CI §3.4 step 1: validate the release manifest BEFORE any plan is
    // computed, and step 2: build the substitution table from exactly §3.3's
    // rows. A development build has no manifest and every row will refuse.
    let release = EMBEDDED_RELEASE_MANIFEST
        .and_then(|bytes| spine_template::ReleaseManifest::parse(bytes.as_bytes()).ok());

    // The render set, in `esc`-path order once the plan sorts it.
    let mut desired = vec![Desired {
        // `user-owned`: spine seeds it once and never touches it again, not by
        // upgrade, not by --force, not by rollback (PB §6.7).
        path: "CONSTITUTION.md".into(),
        owner: Owner::UserOwned,
        template: "constitution@1".into(),
        content: seed.into_bytes(),
    }];
    // The keyring, if a signing key was given.
    //
    // PB §11: "A first `init` with no signing key cannot produce a trust root
    // and says so." So `--signer-key` is what makes a repository signable, and
    // without it the seed is omitted rather than invented — `.spine/
    // allowed_signers` is `user-owned`, seeded once and never touched again.
    if let Some(path) = &options.signer_key {
        match seed_keyring(
            path,
            options.identity.as_deref(),
            options.pipeline_key.as_deref(),
        ) {
            Ok(bytes) => desired.push(Desired {
                path: ".spine/allowed_signers".into(),
                owner: Owner::UserOwned,
                template: "keyring@1".into(),
                content: bytes,
            }),
            Err(why) => {
                eprintln!("spine init: {why}");
                return Ok(exit::REFUSED);
            }
        }
    } else if options.pipeline_key.is_some() {
        eprintln!(
            "spine init: --pipeline-key without --signer-key would seed a keyring with no \
             signing key, which cannot produce a trust root (PB §11)"
        );
        return Ok(exit::REFUSED);
    }

    // PB §11's three managed regions. Each is a block inside a file spine does
    // not own, located by its markers only.
    for (path, template, body) in spine_template::regions::V1_REGIONS {
        let version = template_version(template).expect("a v1 template");
        desired.push(Desired {
            path: path.into(),
            owner: Owner::SpineOwned,
            template: format!("{template}@{version}"),
            content: body.as_bytes().to_vec(),
        });
    }
    if let Some(release) = &release {
        let table = match spine_template::Table::build(release, &trunk) {
            Ok(table) => table,
            Err(e) => {
                eprintln!("spine init: {e}");
                return Ok(exit::REFUSED);
            }
        };
        match ci_paths(&ci, &table) {
            Ok(rows) => desired.extend(rows),
            Err(why) => {
                // CI §3.4: "Any occurrence is `unsubstituted-token`: the whole
                // plan is REFUSE and nothing is written."
                eprintln!("spine init: {why}");
                eprintln!("spine init: the whole plan is refused; nothing was written");
                return Ok(exit::REFUSED);
            }
        }
    } else {
        // A development build still names the paths so the plan reports every
        // row it would have written, each REFUSE with `no-release-manifest`.
        desired.extend(
            spine_template::ci_templates::Provider::parse(&ci)
                .map(|p| {
                    p.files()
                        .iter()
                        .map(|f| Desired {
                            path: f.path.to_string(),
                            owner: Owner::SpineOwned,
                            template: f.template_ref(),
                            content: Vec::new(),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        );
    }

    // CI §3.4 step 1: "Validate first … **before any plan is computed**."
    let plan = match &release {
        None => plan::development_build_plan(&desired),
        Some(_) => {
            {
                let tree = HeadTree { repo: &repo };
                // **The manifest already in the tree, if there is one.**
                //
                // Without it every path a previous run wrote reads as
                // `foreign` — present in HEAD, claimed by no record — and
                // `spine-owned` + foreign is a refusal. So a clean re-run of an
                // initialised repository refused four of its own five paths.
                //
                // PB §6.7: "On an initialised repo, `init` is idempotent", and
                // idempotence is *this* comparison: "it renders every template
                // the binary ships using the manifest's `params`, compares blob
                // ids, and emits a per-path plan". A plan computed against no
                // manifest is a plan that cannot tell spine's own work from a
                // stranger's.
                //
                // Read from HEAD and not the working tree, for the reason the
                // rest of the plan is: an uncommitted manifest is not what the
                // last completed run landed.
                let previous = tree.read(".spine/manifest.json").and_then(|bytes| {
                    spine_manifest::Manifest::parse(&bytes, Some(format_of(&repo))).ok()
                });
                plan::compute(&tree, &desired, previous.as_ref(), &|name| {
                    template_version(name)
                })
            }
        }
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

    // ---- The apply. PB §6.7 step 4's ordering, in spine-init::apply. ------
    let format = repo.object_format();
    let root = repo.root().to_path_buf();
    let release = EMBEDDED_RELEASE_MANIFEST
        .and_then(|bytes| spine_template::ReleaseManifest::parse(bytes.as_bytes()).ok())
        .ok_or("a plan that did not refuse implies a release manifest")?;

    let applied = spine_init::apply::apply(
        &root,
        &plan,
        &desired,
        format,
        &template_version,
        &|applied| {
            build_manifest(
                repo_name,
                &release,
                format,
                &trunk,
                &ci,
                options.isolation.as_deref(),
                &langs,
                applied,
            )
        },
    )?;

    for entry in &applied {
        println!("{} {}", entry.action.token(), entry.path);
    }
    println!();
    println!(
        "wrote {} path(s) and .spine/manifest.json;          the manifest describes the last completed run (PB §6.7 step 4)",
        applied.len()
    );
    Ok(exit::OK)
}

/// The manifest the run just completed, built from the blobs it actually wrote.
///
/// It is a closure argument to `apply` rather than a value because it records
/// what was written — so it cannot be built before the renders are known, and
/// it must still be recorded in staging before any rename.
#[allow(clippy::too_many_arguments)]
fn build_manifest(
    repo_name: &str,
    release: &spine_template::ReleaseManifest,
    format: spine_canon::ObjectFormat,
    trunk: &str,
    ci: &str,
    isolation: Option<&str>,
    langs: &[String],
    applied: &[spine_init::Applied],
) -> Vec<u8> {
    let files = applied
        .iter()
        .map(|entry| spine_manifest::FileEntry {
            path: entry.path.clone(),
            owner: owner_of(&entry.path),
            blob: entry.blob.clone(),
            template: Some(template_of(&entry.path).to_string()),
            base: None,
        })
        .collect();

    let builder = spine_manifest::Builder {
        repo: repo_name.to_string(),
        cli_version: release.version.clone(),
        // CI §5.5 fixes `dist_hash` as the digest of the release's artifact
        // list, which is "fixed only once every artifact is built" — so it is
        // not a member of the release manifest and a build that has not cut a
        // release does not have one. A synthetic build carries the digest of
        // its own version string, which is a value, is 64 hex, and is
        // unmistakably not a real artifact list.
        cli_dist_hash: spine_canon::sha256_prefixed(release.version.as_bytes()),
        object_format: format,
        schema: 7,
        envelope: 1,
        manifest_version: 1,
        trunk: trunk.to_string(),
        ci: ci.to_string(),
        isolation: isolation.map(str::to_string),
        langs: langs.to_vec(),
        timeout: None,
        paths: vec![(
            "constitution".to_string(),
            vec!["CONSTITUTION.md".to_string()],
        )],
        templates: spine_manifest::schema::V1_TEMPLATES
            .iter()
            .map(|name| ((*name).to_string(), template_version(name).unwrap_or(1)))
            .collect(),
        resign: spine_manifest::schema::RESIGN_KEYS
            .iter()
            .map(|name| ((*name).to_string(), template_version(name).unwrap_or(1)))
            .collect(),
        files,
    };
    builder
        .build()
        .expect("init builds a conforming manifest")
        .to_bytes()
}

fn format_of(repo: &Repo) -> spine_canon::ObjectFormat {
    repo.object_format()
}

/// PB §6.7's three ownership classes, by path.
fn owner_of(path: &str) -> Owner {
    match path {
        // "spine once (seed), humans after. Never touched again."
        "CONSTITUTION.md" | ".spine/allowed_signers" => Owner::UserOwned,
        _ => Owner::SpineOwned,
    }
}

fn template_of(path: &str) -> &'static str {
    match path {
        "CONSTITUTION.md" => "constitution@1",
        ".spine/allowed_signers" => "keyring@1",
        "AGENTS.md#spine" => "agents-block@2",
        ".gitignore#spine" => "gitignore@1",
        ".gitattributes#spine" => "gitattributes@1",
        ".github/workflows/spine-collect.yml" => "ci-github-collect@4",
        ".github/workflows/spine-land.yml" => "ci-github-land@4",
        ".gitlab-ci.yml" | ".spine/gitlab/untrusted.yml" | ".spine/gitlab/trusted.yml" => {
            "ci-gitlab@4"
        }
        _ => "ci-generic@4",
    }
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

/// The `keyring@1` seed: `--signer-key`, optionally `--pipeline-key`.
///
/// PB §11 gives the principal exactly two sources and neither is a guess —
/// `--identity` if given, else the key's own comment — so a key with neither
/// refuses rather than being enrolled under a name nobody chose.
///
/// One call, not two, even when both flags are given. The namespace assignment
/// is a function of the whole entry set (solo holds all three, team holds none
/// of the seal), and computing it twice is what made `--signer-key A
/// --pipeline-key C` and two separate runs seed different keyrings for one
/// repository.
fn seed_keyring(
    signer_key: &str,
    identity: Option<&str>,
    pipeline_key: Option<&str>,
) -> std::result::Result<Vec<u8>, String> {
    use spine_template::keyring_seed::{Role, enrol, read_public_key, render_seed};

    let read = |path: &str| -> std::result::Result<_, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        read_public_key(&text).map_err(|e| format!("{path}: {e}"))
    };

    let mut entries =
        vec![enrol(read(signer_key)?, identity, Role::Human).map_err(|e| e.to_string())?];
    if let Some(path) = pipeline_key {
        // The pipeline key takes no `--identity`: that flag names the operator,
        // and the seal principal is the trusted stage's.
        entries.push(enrol(read(path)?, None, Role::Pipeline).map_err(|e| e.to_string())?);
    }

    render_seed(&entries)
        .map(String::into_bytes)
        .map_err(|e| e.to_string())
}

/// CI §3.1's per-provider path table, rendered.
///
/// The bodies and the table both live in `spine_template::ci_templates`; this
/// only substitutes and turns the result into plan rows. Rendering happens here
/// rather than at plan time because CI §3.4 puts the byte scan **before** the
/// plan compares blobs: "the scan precedes every write, and one failure refuses
/// the whole plan rather than writing the paths that happened to pass."
fn ci_paths(
    provider: &str,
    table: &spine_template::Table,
) -> std::result::Result<Vec<Desired>, String> {
    let provider = spine_template::ci_templates::Provider::parse(provider)
        .ok_or_else(|| format!("unknown provider {provider:?}"))?;

    let rendered = spine_template::ci_templates::render_all(provider, table)
        .map_err(|refusal| refusal.to_string())?;

    Ok(rendered
        .into_iter()
        .zip(provider.files())
        .map(|((path, body), file)| Desired {
            path: path.to_string(),
            // CI §3.1's fourth column is `spine-owned` on all six rows.
            owner: Owner::SpineOwned,
            template: file.template_ref(),
            content: body.into_bytes(),
        })
        .collect())
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
