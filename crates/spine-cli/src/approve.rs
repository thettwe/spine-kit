//! `spine check --approve <id>` — PB §4.3's approval record.
//!
//! > "The transition to `tests-approved` is a signed, empty commit on the
//! > intent branch … which refuses a dirty worktree and freezes the branch
//! > HEAD's tree."
//!
//! Three things it computes and one it signs: the freeze **closure**
//! (`spine-resolve`), the base-restored **tree** (`spine-init`), the **red**
//! count over that tree (`crate::g12`), and then one `Spine-Approve` line under
//! `spine-review@v1` with the `Spine-Frozen` and `Spine-Test` lines beneath it.

use std::collections::BTreeSet;
use std::process::ExitCode;

use spine_envelope::{Namespace, TrailerName};
use spine_init::Repo;
use spine_resolve::closure::{Closure, Inputs, closure};
use spine_resolve::glob::Pattern;

use crate::exit;
use crate::sign::{Commit, signing_key, write_statement};

pub fn run(id: &str) -> ExitCode {
    match inner(id) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("spine check --approve: {e}");
            ExitCode::from(exit::ERROR)
        }
    }
}

fn inner(id: &str) -> Result<u8, Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let repo = Repo::discover(&cwd)?;
    let doc = format!("intents/{id}.md");

    // "refuses a dirty worktree" — the whole tree, not just the intent path.
    // `--approve` freezes the branch HEAD's **tree**, and an uncommitted edit
    // anywhere in it is a byte the approval would claim to have frozen and did
    // not.
    let dirty = repo.dirty_paths().unwrap_or_default();
    if !dirty.is_empty() {
        eprintln!(
            "spine check --approve: the working tree is not clean, and the approval freezes \
             the branch HEAD's tree. Dirty: {}",
            dirty.join(", ")
        );
        return Ok(exit::REFUSED);
    }

    let Some(intent_blob) = repo.blob_id_at_head(&doc) else {
        eprintln!("spine check --approve: HEAD has no {doc}");
        return Ok(exit::REFUSED);
    };
    let bytes = repo
        .read_head(&doc)
        .ok_or_else(|| format!("{doc} is in HEAD's tree but will not read"))?;
    let intent_id =
        spine_intent::IntentId::parse(id).ok_or_else(|| format!("{id} is not an intent id"))?;
    let parsed = spine_intent::parse(&bytes, &intent_id)
        .map_err(|r| format!("{doc} does not parse — {r}"))?;

    // PB §11's state table puts `--approve` after a sign-off, and G13 checks it
    // at landing. Refusing here is the cheaper place to learn.
    if repo
        .last_field_on_branch("Spine-Signoff", "blob")?
        .as_deref()
        != Some(intent_blob.as_str())
    {
        eprintln!(
            "spine check --approve: no `Spine-Signoff` on this branch names the intent blob at \
             HEAD — sign it first, or reopen if the document changed"
        );
        return Ok(exit::REFUSED);
    }

    // `base=` is "the trunk tip at approval (audit data — the keyring that
    // verifies a landed approval is the seal's)".
    let trunk = trunk_name(&repo)?;
    let base = repo
        .rev_parse(&format!("refs/heads/{trunk}"))
        .map_err(|_| format!("refs/heads/{trunk} does not resolve"))?;
    let head = repo.rev_parse("HEAD")?;

    // ---- The closure. IR §2.1's inputs, and nothing else. ----------------
    let c_t1 = constitution_patterns(&repo, &base, "C-T1")?;
    let c_t2 = constitution_patterns(&repo, &base, "C-T2")?;
    let approval_tree = crate::tree_source::GitTree::read(&repo, &head)?;
    let base_tree = crate::tree_source::GitTree::read(&repo, &base)?;

    let frozen = closure(&Inputs {
        approval: &approval_tree,
        base: &base_tree,
        c_t1: &c_t1,
        c_t2: &c_t2,
        expected: &parsed.expected,
        intent: &intent_id,
        ac_count: parsed.acs.len().min(u8::MAX as usize) as u8,
    });

    if frozen.frozen.is_empty() {
        eprintln!(
            "spine check --approve: the closure is empty — no file under a `C-T1` root carries a \
             `@verifies {id}/AC-n` pragma, so there is nothing to freeze (PB §4.3 clause 1)"
        );
        return Ok(exit::REFUSED);
    }
    report(&frozen);

    // ---- G12's tree and its number. --------------------------------------
    let expected = parsed.expected.clone();
    let restored = repo.restored_base_tree(&head, &base, &|p: &str| {
        expected.iter().any(|pat| pat.matches(p.as_bytes()))
    })?;
    let langs = trunk_langs(&repo, &base)?;
    let frozen_paths: BTreeSet<String> = frozen.paths().map(str::to_string).collect();
    let measured = crate::g12::measure(&repo, &restored, &frozen_paths, &langs, 1800)?;

    for path in &measured.uncollected {
        eprintln!("spine check --approve: no test collected from frozen path {path}");
    }

    // PB §11: "`reason=` is mandatory, and G13 refuses its absence, on
    // `red=0/n`, `held=false`, or a closure tripwire." PB §11's signature gives
    // `--approve` no `--reason`, so each of those is a refusal here rather than
    // a reason invented on the operator's behalf. Filed as open question 10.
    if measured.red == 0 {
        eprintln!(
            "spine check --approve: red=0/{} — no frozen test failed against the base-restored \
             tree, which G12 makes a wire a human must sign with a `reason=` (PB §6.3). \
             PB §11's signature gives `--approve` no `--reason`, so this refuses rather than \
             invent one: see .build-notes/OPEN-questions.md #10.",
            measured.total
        );
        return Ok(exit::REFUSED);
    }
    if !frozen.unresolvable.is_empty() {
        eprintln!(
            "spine check --approve: a closure tripwire also needs a `reason=`, which \
             `--approve` has no flag for (open question 10)"
        );
        return Ok(exit::REFUSED);
    }

    // ---- The lines. ------------------------------------------------------
    let mut lines: Vec<Vec<u8>> = Vec::new();
    for path in frozen.paths() {
        // "`Spine-Frozen` lines are `<blob id> <path>` with `git ls-tree`
        // quoting: a path says where a test lives; a blob id says what it
        // asserts."
        let blob = repo
            .blob_id_at_head(path)
            .ok_or_else(|| format!("{path} is in the closure and not in HEAD's tree"))?;
        let mut payload = format!("{blob} ").into_bytes();
        payload.extend_from_slice(&spine_envelope::quote::quote_path(path.as_bytes()));
        lines.push(rendered(TrailerName::Frozen, &payload));
    }
    for test in &measured.tests {
        lines.push(rendered(TrailerName::Test, test.as_bytes()));
    }

    // "`freeze=` a SHA-256 over the sorted `Spine-Frozen` and `Spine-Test`
    // lines … Sorted how, and over which bytes, is not a detail" — EV fixes it
    // and `spine-envelope` owns it.
    let refs: Vec<&[u8]> = lines.iter().map(Vec::as_slice).collect();
    let freeze = spine_envelope::digest::freeze_digest_over(&refs);

    let reopens = repo.count_trailer_on_branch("Spine-Reopen")?;
    // "`rounds=` the A↔B bounce-backs consumed this time and `total_rounds=`
    // across reopens". v1 has no A↔B loop to count, so one round is what this
    // approval consumed; the total accumulates across earlier approve lines
    // while they are reachable on the branch, which is what G13 checks.
    let rounds = 1u64;
    let total_rounds = repo
        .last_field_on_branch("Spine-Approve", "total_rounds")?
        .and_then(|n| n.parse::<u64>().ok())
        .unwrap_or(0)
        + rounds;
    let signer = repo.config("user.email").ok_or("git has no `user.email`")?;
    let key = signing_key(&repo)?;

    let payload = format!(
        "{id} intent={intent_blob} base={base} rounds={rounds} total_rounds={total_rounds} \
         reopens={reopens} red={}/{} freeze={freeze} signer={signer}",
        measured.red, measured.total
    );

    write_statement(
        &repo,
        TrailerName::Approve,
        &payload,
        Namespace::Review,
        &key,
        Commit::EmptyWith(lines),
    )?;
    eprintln!(
        "spine check --approve: {id} is approved — {} frozen, {} test(s), red {}/{}",
        frozen.frozen.len(),
        measured.tests.len(),
        measured.red,
        measured.total
    );
    Ok(exit::OK)
}

fn rendered(name: TrailerName, payload: &[u8]) -> Vec<u8> {
    let line = spine_envelope::render_line(name, payload);
    line.strip_suffix(b"\n").unwrap_or(&line).to_vec()
}

fn report(frozen: &Closure) {
    for path in &frozen.excluded {
        eprintln!("spine check --approve: excluded as code under test — {path}");
    }
    for path in &frozen.unresolvable {
        eprintln!("spine check --approve: closure tripwire — unresolvable import in {path}");
    }
}

fn trunk_name(repo: &Repo) -> Result<String, Box<dyn std::error::Error>> {
    Ok(manifest_at(repo, "HEAD")?.trunk().to_string())
}

fn trunk_langs(repo: &Repo, base: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(manifest_at(repo, base)?
        .langs()
        .iter()
        .map(|s| (*s).to_string())
        .collect())
}

fn manifest_at(
    repo: &Repo,
    commit: &str,
) -> Result<spine_manifest::Manifest, Box<dyn std::error::Error>> {
    let bytes = repo
        .read_at(commit, ".spine/manifest.json")
        .ok_or("no .spine/manifest.json to read policy from")?;
    spine_manifest::Manifest::parse(&bytes, Some(repo.object_format()))
        .map_err(|r| format!("the manifest does not parse: {r}").into())
}

/// `effective(C-T1)` / `effective(C-T2)` from the constitution at `base`.
///
/// PB §7.4 rule 1: policy is read from trunk, and `base` is trunk's tip at
/// approval — never from the branch, which could otherwise narrow its own
/// harness and shrink the closure.
fn constitution_patterns(
    repo: &Repo,
    base: &str,
    rule: &str,
) -> Result<Vec<Pattern>, Box<dyn std::error::Error>> {
    let Some(bytes) = repo.read_at(base, "CONSTITUTION.md") else {
        return Ok(Vec::new());
    };
    let text = String::from_utf8_lossy(&bytes);
    let needle = format!("{rule}: ");
    for line in text.lines() {
        if !line.starts_with(&needle) {
            continue;
        }
        // CN §4.5: "split the body at its first `0x3D`"; §5.5 splits the value
        // on `,` and strips spaces and tabs.
        let Some((_, value)) = line.split_once('=') else {
            continue;
        };
        return Ok(value
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .filter_map(|p| Pattern::parse(p).ok())
            .collect());
    }
    Ok(Vec::new())
}
