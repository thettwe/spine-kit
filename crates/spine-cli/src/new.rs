//! `spine new` — dispatch, and what each of PB §11's four forms still owes.
//!
//! Three of the four *sign*, and PB §7.1 puts them behind the same rule
//! `spine check`'s signing invocations sit behind: "any invocation that
//! produces a `-Sig` line with a key that is not the `--ci` pipeline secret —
//! `--sign`, `--reopen`, `--withdraw` … — is TTY-only and refuses under
//! `SPINE_AGENT=1`". That refusal is a property of the invocation, so it is
//! decided from the parsed form before anything is read.

use std::process::ExitCode;

use spine_init::Repo;
use spine_template::scaffold;

use crate::argv::{New, Variant};
use crate::{allocate, exit};

/// MF §3, not configurable.
const MANIFEST_PATH: &str = ".spine/manifest.json";

/// Read before the manifest is parsed, so the trunk name is not yet known:
/// `HEAD` is the fallback the first read uses, and the second read below uses
/// the real trunk ref.
const TRUNK_PLACEHOLDER: &str = "HEAD";

/// PB §7.1's three signing forms of this command. The creation form writes a
/// branch and a scaffold and signs nothing.
fn signs_with_a_human_key(new: &New) -> bool {
    !matches!(new, New::Create { .. })
}

pub fn run(new: &New) -> ExitCode {
    if signs_with_a_human_key(new) && std::env::var_os("SPINE_AGENT").is_some_and(|v| v == "1") {
        eprintln!(
            "spine new: this invocation signs under a human key and refuses under \
             SPINE_AGENT=1 (PB §7.1)"
        );
        return ExitCode::from(exit::REFUSED);
    }

    let owed = match new {
        New::Create { variant, from } => return create(*variant, from.as_deref()),
        New::Sign { .. } => "--sign",
        New::Reopen { .. } => "--reopen",
        New::Withdraw { .. } => "--withdraw",
    };
    eprintln!("spine new: {owed} is not yet implemented");
    ExitCode::from(exit::ERROR)
}

/// `spine new [--change|--bug]` — PB §11: "runs the interview (§3.4) on a fresh
/// `intent/<ID>` branch and emits the filled template, stamped with the
/// manifest's template version".
///
/// The *interview* is not here. PB §3.4's interview is a conversation that
/// produces the document's content, and what this writes is the scaffold it
/// starts from — which is what "emits the filled template" names and what
/// TM §6.1 renders.
/// `--from <branch>` "promotes an escalated quick-lane branch" (PB §11). The
/// owner ruled the mechanism on 2026-08-28 and PB §11 now carries it: the
/// branch is created from **trunk** — never from the quick branch, since "a
/// stacked intent would misattribute members after the first lands" (PB §5.4)
/// — and the quick branch's commits are **cherry-picked** onto it. Not
/// squashed: `M(L) = git rev-list B..L` is the changeset PB §6.2 derives and
/// G10 reconstructs, so the shape is ledger content, and `spine stats` "counts
/// promotions separately", which a squash makes unanswerable.
fn create(variant: Variant, from: Option<&str>) -> ExitCode {
    match create_inner(variant, from) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("spine new: {e}");
            ExitCode::from(exit::ERROR)
        }
    }
}

fn create_inner(variant: Variant, from: Option<&str>) -> Result<u8, Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let repo = Repo::discover(&cwd)?;

    // PB §5.4: "`spine new` branches only from trunk", so trunk is where the
    // manifest is read from too — the branch about to be created inherits
    // trunk's parameters, not the checkout's.
    let trunk_manifest = repo
        .read_at(TRUNK_PLACEHOLDER, MANIFEST_PATH)
        .or_else(|| repo.read_head(MANIFEST_PATH));
    let Some(bytes) = trunk_manifest else {
        eprintln!(
            "spine new: no readable {MANIFEST_PATH}; run `spine init` before creating an intent"
        );
        return Ok(exit::REFUSED);
    };
    let manifest = spine_manifest::Manifest::parse(&bytes, Some(repo.object_format()))
        .map_err(|r| format!("trunk's manifest does not parse: {r}"))?;
    let trunk = manifest.trunk().to_string();

    // The trunk ref has to resolve before anything is created: PB §5.4's
    // "branches only from trunk" has no meaning without one.
    let trunk_ref = format!("refs/heads/{trunk}");
    if repo.rev_parse(&trunk_ref).is_err() {
        eprintln!("spine new: {trunk_ref} does not resolve, so there is no trunk to branch from");
        return Ok(exit::REFUSED);
    }

    let git = RepoRefs {
        repo: &repo,
        trunk: &trunk,
    };
    let id = match allocate::allocate(&git, variant) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("spine new: {e}");
            return Ok(exit::REFUSED);
        }
    };

    // ID §4.3's owner, from git's own notion of who is running this. There is
    // no `--identity` on PB §11's `spine new`, and `scaffold::Instance`
    // documents the field as "the principal of the signing identity … with no
    // `@` prefix added" — so a configured `user.email` is the value, and an
    // unconfigured one is a refusal rather than a guessed placeholder in a
    // document a human is about to sign.
    let Some(owner) = repo.config("user.email").filter(|v| !v.trim().is_empty()) else {
        eprintln!(
            "spine new: git has no `user.email`, and the scaffold's `Owner:` is the principal              that signs it — set one rather than have spine invent it"
        );
        return Ok(exit::REFUSED);
    };

    let template_variant = match variant {
        Variant::Intent => scaffold::Variant::Intent,
        Variant::Change => scaffold::Variant::IntentChange,
        Variant::Bug => scaffold::Variant::IntentBug,
    };
    let template_version = manifest
        .template_version(template_variant.name())
        .ok_or_else(|| {
            format!(
                "trunk's manifest records no `templates.{}`",
                template_variant.name()
            )
        })?;

    // CN §9.1: line 2 is the header and `Version:` is its first field. The
    // scaffold stamps it so `built_under` has something to name (PB §2.1).
    //
    // By **key**, not by taking the first floor entry: `floor_entries` is the
    // flattened value set of every `paths.*` key, so a repository that names
    // any other path would have had its constitution version read out of
    // whichever value sorted first.
    let constitution_path = manifest
        .paths_by_key()
        .into_iter()
        .find(|(key, _)| *key == "constitution")
        .and_then(|(_, values)| values.first().map(|v| (*v).to_string()));
    let constitution_version = constitution_path
        .and_then(|path| repo.read_at(&trunk_ref, &path))
        .and_then(|bytes| constitution_version(&bytes))
        .unwrap_or(1);

    let body = scaffold::render(
        template_variant,
        &scaffold::Instance {
            id: &id,
            owner: owner.trim(),
            template_version,
            constitution_version,
        },
    )?;

    let branch = format!("intent/{id}");
    repo.create_branch_from(&branch, &trunk_ref)?;
    let doc = format!("intents/{id}.md");
    let full = repo.root().join(&doc);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full, body.as_bytes())?;

    // The promotion, after the branch exists and before the scaffold is
    // announced: a conflict refuses, so the operator is not told about a
    // document sitting on a half-promoted branch.
    if let Some(quick) = from {
        let range = format!("{trunk_ref}..{quick}");
        match repo.cherry_pick_range(&range) {
            Ok(0) => eprintln!("spine new: {quick} has no commits above {trunk} to promote"),
            Ok(n) => eprintln!("spine new: promoted {n} commit(s) from {quick}"),
            Err(e) => {
                eprintln!("spine new: promoting {quick} failed: {e}");
                eprintln!(
                    "spine new: the cherry-pick was aborted; {branch} carries the scaffold \
                     and none of {quick}'s commits"
                );
                return Ok(exit::REFUSED);
            }
        }
    }

    println!("{id}");
    eprintln!(
        "spine new: on {branch}, wrote {doc} ({}@{template_version})",
        template_variant.name()
    );
    eprintln!(
        "spine new: fill it in, commit it, then `spine new --sign {id}` — the one human gate \
         (PB §3.4)"
    );
    Ok(exit::OK)
}

/// CN §9.1's `Version:` field, read out of the constitution's line 2.
fn constitution_version(bytes: &[u8]) -> Option<u32> {
    let line = bytes.split(|&b| b == b'\n').nth(1)?;
    for field in String::from_utf8_lossy(line).split(" \u{b7} ") {
        let (name, value) = field.split_once(": ")?;
        // "Names are matched **ASCII-case-insensitively**, values are not."
        if name.eq_ignore_ascii_case("version") {
            return value.strip_prefix('v')?.parse().ok();
        }
    }
    None
}

/// [`allocate::Refs`] over a real repository.
struct RepoRefs<'a> {
    repo: &'a Repo,
    trunk: &'a str,
}

impl allocate::Refs for RepoRefs<'_> {
    fn ref_names(&self) -> Vec<String> {
        self.repo.ref_names()
    }

    fn trunk_messages(&self) -> Vec<String> {
        self.repo
            .first_parent_log(&format!("refs/heads/{}", self.trunk))
            .unwrap_or_default()
            .into_iter()
            .map(|(_, message)| message)
            .collect()
    }

    fn fetch(&self) -> bool {
        self.repo.fetch_trunk_and_intents(self.trunk)
    }

    fn has_remote(&self) -> bool {
        self.repo.has_remote()
    }
}
