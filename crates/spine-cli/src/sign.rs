//! `spine new --sign` — PB §3.4's one mandatory human gate.
//!
//! > "The transition to `signed` is a signed, empty commit on the intent
//! > branch" — PB §11's state table: "`Spine-Signoff` + `-Sig` verify under
//! > `spine-signoff@v1`; `blob=` equals the intent blob at head; `reopens=`
//! > equals the branch's reopen count".
//!
//! The four preconditions are ID §8.1's **Layer 2**, and they are
//! `spine-intent`'s: this reads the facts out of the repository and
//! `check_signoff` decides. ID §11.5 draws the line — "Layer 2 is about the
//! document's *stage*, Layer 1 about its *shape*, and only shape can be checked
//! years later against a sealed envelope."

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use spine_envelope::sign::Key;
use spine_envelope::{Namespace, TrailerName, sig_line, sign_line};
use spine_init::Repo;
use spine_intent::parse::{SignoffFacts, check_signoff, parse};

use crate::exit;

/// ID §2.4: the blob is "the object named by `intents/<ID>.md` in the tree of
/// the branch head at the moment of signing — `git rev-parse HEAD:intents/<ID>.md`".
fn intent_path(id: &str) -> String {
    format!("intents/{id}.md")
}

pub fn run(id: &str, override_lease: Option<&str>) -> ExitCode {
    match inner(id, override_lease) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("spine new --sign: {e}");
            ExitCode::from(exit::ERROR)
        }
    }
}

fn inner(id: &str, override_lease: Option<&str>) -> Result<u8, Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let repo = Repo::discover(&cwd)?;
    let doc = intent_path(id);

    // **The blob, never the worktree.** ID §2.4: "the canonical-form rules of
    // §2.1 are checked over `git cat-file blob <oid>`, not over the file on
    // disk."
    let Some(head_blob) = repo.blob_id_at_head(&doc) else {
        eprintln!("spine new --sign: HEAD has no {doc}; commit the intent before signing it");
        return Ok(exit::REFUSED);
    };
    let Some(bytes) = repo.read_head(&doc) else {
        eprintln!("spine new --sign: {doc} is in HEAD's tree but its blob will not read");
        return Ok(exit::ERROR);
    };

    let intent_id =
        spine_intent::IntentId::parse(id).ok_or_else(|| format!("{id} is not an intent id"))?;
    let parsed = match parse(&bytes, &intent_id) {
        Ok(parsed) => parsed,
        Err(refusal) => {
            eprintln!("spine new --sign: {doc} does not parse — {refusal}");
            return Ok(exit::REFUSED);
        }
    };

    // ---- ID §8.1's Layer 2, in that table's order. -----------------------
    let branch = repo
        .current_branch()
        .map(|b| format!("refs/heads/{b}"))
        .unwrap_or_default();

    // "The worktree file at `intents/<ID>.md`, **if it exists**, must hash —
    // via `git hash-object --path` — to the head blob's id." Absent is clean:
    // the bytes being signed are HEAD's either way, and a human who deleted
    // the file is not being shown different bytes.
    let worktree = repo.root().join(&doc);
    let worktree_clean = match std::fs::read(&worktree) {
        Err(_) => true,
        Ok(on_disk) => repo
            .hash_object_filtered(&doc, &on_disk)
            .map(|id| id == head_blob)
            .unwrap_or(false),
    };

    let resign_floor = trunk_resign_floor(&repo, parsed.variant)?;
    if let Err(refusal) = check_signoff(
        &parsed,
        &SignoffFacts {
            branch: &branch,
            worktree_clean,
            resign_floor,
        },
    ) {
        eprintln!("spine new --sign: {refusal}");
        if !worktree_clean {
            eprintln!(
                "spine new --sign: {doc} on disk differs from the blob at HEAD — \
                 signing bytes a human is not looking at is what this refuses (ID §2.4)"
            );
        }
        return Ok(exit::REFUSED);
    }

    // ---- The line. -------------------------------------------------------
    //
    // PB §11: `INT-042 blob=<oid> template=<variant>@<n> constitution=v3
    // reopens=n [lease_override="…"] signer=<p>`. `reopens=` is "the count of
    // signed reopens on the branch at signing, so a sign-off cannot be replayed
    // after a reopen".
    let reopens = repo.count_trailer_on_branch("Spine-Reopen")?;
    let signer = repo.config("user.email").ok_or(
        "git has no `user.email`, and the sign-off's `signer=` is the principal that signs it",
    )?;
    let key = signing_key(&repo)?;

    let mut payload = format!(
        "{id} blob={head_blob} template={}@{} constitution=v{} reopens={reopens}",
        parsed.variant.token(),
        parsed.template,
        parsed.constitution
    );
    if let Some(reason) = override_lease {
        // PB §5.4: "recorded as `lease_override=` on the sign-off line — the
        // lease still trips at landing." `reason=` values are JSON string
        // literals (PB §7.2).
        payload.push_str(&format!(" lease_override={}", json_string(reason)));
    }
    payload.push_str(&format!(" signer={signer}"));

    let line = spine_envelope::render_line(TrailerName::Signoff, payload.as_bytes());
    let line = line.strip_suffix(b"\n").unwrap_or(&line).to_vec();

    eprintln!("spine new --sign: signing under spine-signoff@v1 — your key may ask to confirm");
    let signature = sign_line(
        TrailerName::Signoff,
        &line,
        Namespace::Signoff,
        &Key::File(&key),
    )?;

    // ---- The commit: signed, and empty. ----------------------------------
    //
    // Empty because the transition is the *statement*, not a change: the intent
    // blob is already at HEAD and signing must not move it.
    let mut message = line.clone();
    message.push(b'\n');
    message.extend_from_slice(&sig_line(TrailerName::Signoff, &signature));
    let sha = repo.commit_empty(&message)?;

    println!("{sha}");
    eprintln!(
        "spine new --sign: {id} is signed — blob {} , reopens {reopens}",
        &head_blob[..12.min(head_blob.len())]
    );
    Ok(exit::OK)
}

/// PB §7.2's `reason=` values "are JSON string literals".
fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `resign[variant]` from the manifest **at trunk** — ID §8.1 reads it there,
/// not from the branch, because a candidate that could lower its own floor
/// would choose the terms it is judged on (PB §7.4 rule 1).
fn trunk_resign_floor(
    repo: &Repo,
    variant: spine_intent::header::Variant,
) -> Result<u32, Box<dyn std::error::Error>> {
    let Some(bytes) = repo
        .read_at("HEAD", ".spine/manifest.json")
        .or_else(|| repo.read_head(".spine/manifest.json"))
    else {
        return Err("no .spine/manifest.json to read `resign` from".into());
    };
    let manifest = spine_manifest::Manifest::parse(&bytes, Some(repo.object_format()))
        .map_err(|r| format!("the manifest does not parse: {r}"))?;
    Ok(manifest
        .resign_version(variant.token())
        .unwrap_or(1)
        .try_into()
        .unwrap_or(u32::MAX))
}

/// Where the operator's signing key is.
///
/// `SPINE_SIGNING_KEY` names it. There is no search of `~/.ssh`: a signing key
/// picked by a heuristic is a signature under an identity the operator did not
/// choose, and MF §4.5 grants namespaces per principal — the wrong key is a
/// refusal at G13 rather than a mistake anyone sees here.
fn signing_key(repo: &Repo) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = std::env::var_os("SPINE_SIGNING_KEY") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
        return Err(format!(
            "SPINE_SIGNING_KEY names {}, which is not there",
            path.display()
        )
        .into());
    }
    // git's own configuration, which a repository signing commits already has.
    if let Some(configured) = repo.config("user.signingkey") {
        let path = Path::new(&configured);
        if path.exists() {
            return Ok(path.to_path_buf());
        }
    }
    Err(
        "no signing key: set SPINE_SIGNING_KEY, or git's `user.signingkey`, to the key \
         whose principal the keyring grants `spine-signoff@v1`"
            .into(),
    )
}
