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

    write_statement(
        &repo,
        TrailerName::Signoff,
        &payload,
        Namespace::Signoff,
        &key,
        Commit::Empty,
    )?;

    eprintln!(
        "spine new --sign: {id} is signed — blob {}, reopens {reopens}",
        &head_blob[..12.min(head_blob.len())]
    );
    Ok(exit::OK)
}

/// Whether the statement's commit carries a change.
///
/// A sign-off is empty: PB §3.4 makes the transition "a signed, **empty**
/// commit", because the statement *is* the transition and the blob it names is
/// already at HEAD. A reopen is not: PB §4.3 says "the commit that changes the
/// intent blob carries a signed `Spine-Reopen` line", and "A reopen must change
/// the blob — a no-op reopen is refused."
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Commit {
    Empty,
    /// Empty, with further trailer lines beneath the signed statement.
    ///
    /// PB §4.3's approval record is one signed `Spine-Approve` line plus the
    /// `Spine-Frozen` and `Spine-Test` lines it covers by `freeze=` — those are
    /// not separately signed, and are inside the envelope by being in the
    /// message.
    EmptyWith(Vec<Vec<u8>>),
    /// The worktree's changes, staged and committed with the statement.
    WithWorktree,
}

/// Render, sign and commit one statement. The one place a signed line reaches a
/// commit, so every form gets the same bytes-to-signature relationship.
pub fn write_statement(
    repo: &Repo,
    name: TrailerName,
    payload: &str,
    namespace: Namespace,
    key: &Path,
    commit: Commit,
) -> Result<String, Box<dyn std::error::Error>> {
    let line = spine_envelope::render_line(name, payload.as_bytes());
    let line = line.strip_suffix(b"\n").unwrap_or(&line).to_vec();

    eprintln!(
        "spine new: signing under {} — your key may ask to confirm",
        namespace.as_str()
    );
    let signature = sign_line(name, &line, namespace, &Key::File(key))?;

    let mut message = line.clone();
    message.push(b'\n');
    message.extend_from_slice(&sig_line(name, &signature));

    if let Commit::EmptyWith(extra) = &commit {
        for line in extra {
            message.extend_from_slice(line);
            message.push(b'\n');
        }
    }

    let sha = match commit {
        Commit::Empty | Commit::EmptyWith(_) => repo.commit_empty(&message)?,
        Commit::WithWorktree => repo.commit_worktree(&message)?,
    };
    println!("{sha}");
    Ok(sha)
}

/// `spine new --reopen <id> --reason "…"` — PB §4.3's transition.
///
/// > "**Reopen is a transition, not an edit.** If implementation reveals the
/// > tests are wrong, that is an intent problem, and the only way to change a
/// > frozen byte is `spine new --reopen INT-042 --reason \"…\"`: the commit
/// > that changes the intent blob carries a signed `Spine-Reopen` line naming
/// > the freeze digest it voids, and returns the intent to
/// > `awaiting-sign-off`. **A reopen must change the blob — a no-op reopen is
/// > refused.**"
pub fn reopen(id: &str, reason: &str) -> ExitCode {
    match reopen_inner(id, reason) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("spine new --reopen: {e}");
            ExitCode::from(exit::ERROR)
        }
    }
}

fn reopen_inner(id: &str, reason: &str) -> Result<u8, Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let repo = Repo::discover(&cwd)?;
    let doc = intent_path(id);

    // "A reopen must change the blob — a no-op reopen is refused." Checked
    // against HEAD's blob, which is what the previous sign-off named.
    let head_blob = repo.blob_id_at_head(&doc);
    let on_disk = std::fs::read(repo.root().join(&doc)).map_err(|_| {
        format!("{doc} is not in the worktree; a reopen is the edit that changes it")
    })?;
    let edited = repo.hash_object_filtered(&doc, &on_disk)?;
    if head_blob.as_deref() == Some(edited.as_str()) {
        eprintln!(
            "spine new --reopen: {doc} is unchanged — a reopen is the commit that changes the \
             intent blob, and a no-op reopen is refused (PB §4.3)"
        );
        return Ok(exit::REFUSED);
    }

    // `voids=` names the binding approval's freeze, "`none` only when no
    // approval exists; G13 refuses otherwise".
    let voids = repo
        .last_field_on_branch("Spine-Approve", "freeze")?
        .unwrap_or_else(|| "none".to_string());
    let reopens = repo.count_trailer_on_branch("Spine-Reopen")? + 1;
    let signer = repo.config("user.email").ok_or("git has no `user.email`")?;
    let key = signing_key(&repo)?;

    let payload = format!(
        "{id} voids={voids} reopens={reopens} reason={} signer={signer}",
        json_string(reason)
    );
    write_statement(
        &repo,
        TrailerName::Reopen,
        &payload,
        Namespace::Signoff,
        &key,
        Commit::WithWorktree,
    )?;
    eprintln!(
        "spine new --reopen: {id} is back to awaiting-sign-off — voids {voids}, reopens {reopens}"
    );
    Ok(exit::OK)
}

/// `spine new --withdraw <id> --reason "…" [--protected]` — the exit that lands
/// a tombstone.
pub fn withdraw(id: &str, reason: &str, protected: bool) -> ExitCode {
    match withdraw_inner(id, reason, protected) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("spine new --withdraw: {e}");
            ExitCode::from(exit::ERROR)
        }
    }
}

fn withdraw_inner(
    id: &str,
    reason: &str,
    protected: bool,
) -> Result<u8, Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let repo = Repo::discover(&cwd)?;
    let doc = intent_path(id);

    let Some(head_blob) = repo.blob_id_at_head(&doc) else {
        eprintln!("spine new --withdraw: HEAD has no {doc}; there is no intent to withdraw");
        return Ok(exit::REFUSED);
    };
    let signer = repo.config("user.email").ok_or("git has no `user.email`")?;
    let key = signing_key(&repo)?;

    // MF §4.8.3: `Spine-Withdraw` verifies under "`spine-signoff@v1` **or**
    // `spine-review@v1` — check 8 decides which, by key". PB §11 gives
    // `--protected` the second: "signed under `spine-review@v1` by a reviewer ≠
    // the original signer, for an orphaned branch".
    let namespace = if protected {
        Namespace::Review
    } else {
        Namespace::Signoff
    };

    // `orphaned=<principal>` is GR §5.5's "orphaned tombstone": the sign-off's
    // key has left `K`, so the sign-off is omitted from `A` and the withdraw
    // line names the principal there is no fingerprint for.
    let orphaned = protected
        .then(|| {
            repo.last_field_on_branch("Spine-Signoff", "signer")
                .ok()
                .flatten()
        })
        .flatten();

    let mut payload = format!("{id} blob={head_blob}");
    if let Some(principal) = &orphaned {
        payload.push_str(&format!(" orphaned={principal}"));
    }
    payload.push_str(&format!(" reason={} signer={signer}", json_string(reason)));

    write_statement(
        &repo,
        TrailerName::Withdraw,
        &payload,
        namespace,
        &key,
        Commit::Empty,
    )?;
    eprintln!(
        "spine new --withdraw: {id} is withdrawn under {} — land it with `spine check --land {id}` \
         to seal the tombstone",
        namespace.as_str()
    );
    Ok(exit::OK)
}

/// PB §7.2's `reason=` values "are JSON string literals".
pub fn json_string(value: &str) -> String {
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
pub fn signing_key(repo: &Repo) -> Result<PathBuf, Box<dyn std::error::Error>> {
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
