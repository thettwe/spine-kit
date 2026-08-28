//! `keyring@1` — the bytes `spine init` writes to `.spine/allowed_signers`.
//!
//! This module is the *writer*; `spine_manifest::keyring` is the reader. They
//! are asymmetric on purpose, and the asymmetry is the first thing to
//! understand here.
//!
//! **The keyring has no canonical byte form** (MF §4.1, verbatim): *"It is
//! `user-owned` (PB §6.7): humans edit it under a protected PR, `spine init
//! --pipeline-key` appends to it, and requiring canonical bytes would make
//! re-indenting a gate failure."* So nothing below is a *form* the reader
//! requires. [`render_seed`] fixes the bytes of the **seed** — the one file
//! spine writes, once — and after that the file belongs to humans. MF §13
//! OPEN-2 records emitting a canonical line shape from `--signer-key` and
//! `--pipeline-key` as unadopted option (b); v1 builds against (a), lint only.
//!
//! What *is* required is that the seed **lint clean**, so every function here
//! ends by running its own output through [`spine_manifest::Keyring::parse`]
//! and refusing on findings. That is not belt-and-braces: G16 lints `K_T` and
//! G13 lints `K_B` (MF §4.5), so a seed with a finding is a repository whose
//! very first landing fails, and there is no keyring below it to repair from.
//!
//! **Mode is never declared here.** MF §4.5: `mode` is the count of distinct
//! fingerprints under `spine-signoff@v1`, never `C-A1`. Every read of it below
//! goes through [`spine_manifest::Keyring::parse`]'s `mode`, which is the one
//! place MF §13's option (c) would have to change.

use core::fmt;

use spine_manifest::keyring::{Finding, KEYTYPES, Keyring, Lint, Mode};

/// MF §8.3's `files[]` record for `.spine/allowed_signers` — `name@version`,
/// the vocabulary `templates` and `files[].template` share (MF §3.6).
pub const TEMPLATE: &str = "keyring@1";

/// MF §4.3. Held by humans; signs sign-off, reopen, withdraw and toolkit
/// upgrade events.
pub const SIGNOFF: &str = "spine-signoff@v1";

/// MF §4.3. Held by humans; signs reviews, approvals in v1, and the seal of a
/// recovery landing.
pub const REVIEW: &str = "spine-review@v1";

/// MF §4.3. Held by the trusted stage — *"a CI secret no laptop holds; in solo
/// mode, the human's own key"* (PB §7.2).
pub const SEAL: &str = "spine-seal@v1";

/// The order namespaces are **written** in: MF §4.3's role order, signer then
/// reviewer then pipeline.
///
/// Deliberately **not** byte order. MF §4.6 sorts a `signer` node's derived
/// `roles` attr ascending by bytes (`review` < `seal` < `signoff`), but that is
/// the graph's order, not the file's: MF §8.7's published line reads
/// `namespaces="spine-signoff@v1,spine-review@v1"`, and PB §7.2's block reads
/// the same. Sorting here would move the published 411 bytes.
const WIRE_ORDER: [&str; 3] = [SIGNOFF, REVIEW, SEAL];

/// An SSH public key as an `.pub` file carries it: `<keytype> <base64>
/// [comment]` (PB §11, MF §4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey {
    /// One of MF §4.2's eight. `ssh-rsa` is not among them.
    pub keytype: String,
    pub keyblob: String,
    /// The key's own comment, if the file carried one. PB §11 makes this the
    /// default principal when `--identity` is absent — it is *not* written to
    /// the keyring line (MF §8.7's entries carry none).
    pub comment: Option<String>,
    /// `"SHA256:"` + unpadded base64, exactly what `ssh-keygen -lf` prints.
    /// Computed by the reader, so writer and reader can never disagree about
    /// the value `reviewer != signer` compares (MF §4.2, PB §7.2).
    pub fingerprint: String,
}

/// Which of MF §4.3's three roles a key is being enrolled for.
///
/// Not the same thing as the namespace set: a lone human key also holds
/// `spine-seal@v1` (PB §11), and which it is is [`render_seed`]'s call, not the
/// caller's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// `--signer-key`: *"enrols a human signing key in the keyring under
    /// `spine-signoff@v1` and `spine-review@v1`"* (PB §11).
    Human,
    /// `--pipeline-key`: the seal line.
    Pipeline,
}

/// One enrolled key, ready to be written as one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedEntry {
    /// MF §4.2's `principal`. From `--identity`, or the key's comment
    /// (PB §11).
    pub principal: String,
    pub key: PublicKey,
    pub role: Role,
}

/// Why a seed could not be produced.
///
/// Every variant is a refusal, never a repair. PB §11's discovery rule sets the
/// tone for the whole module: `init` *"refuses with instructions when neither
/// is unambiguous"* rather than guessing which key a repository's authority
/// will rest on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeedError {
    /// The `.pub` text is not one `<keytype> <base64> [comment]` line.
    NotOnePublicKey(String),
    /// MF §4.2's keytype list, and the `keyring-keytype-unknown` this would
    /// take. `ssh-rsa` lands here: *"OpenSSH >= 8.2 is a stated requirement
    /// (PB §11) and SHA-1 RSA signatures are the one thing that release
    /// deprecated."*
    KeytypeUnknown(String),
    /// MF §4.4's `keyring-key-not-base64`, both limbs: not base64, **or** a
    /// blob that does not decode to a key of the declared type.
    KeyNotBase64(String),
    /// No `--identity` and no comment on the key. PB §11 gives the principal
    /// exactly two sources and neither is a guess.
    NoPrincipal,
    /// The principal does not match MF §4.2's `principal` production. A comma
    /// would be `keyring-multi-principal`; whitespace would split the line
    /// into different fields entirely.
    PrincipalMalformed(String),
    /// The bytes produced (or the bytes handed in) do not lint clean under
    /// MF §4.4. Carries the reader's own findings, so the caller reports the
    /// same status tokens G13 and G16 would.
    Keyring(Vec<Finding>),
}

impl fmt::Display for SeedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SeedError::NotOnePublicKey(why) => write!(f, "not one SSH public key: {why}"),
            SeedError::KeytypeUnknown(keytype) => {
                write!(
                    f,
                    "keyring-keytype-unknown: {keytype:?} is not in MF §4.2's list"
                )
            }
            SeedError::KeyNotBase64(why) => write!(f, "keyring-key-not-base64: {why}"),
            SeedError::NoPrincipal => {
                f.write_str("no principal: the key carries no comment and --identity was not given")
            }
            SeedError::PrincipalMalformed(principal) => {
                write!(f, "{principal:?} is not MF §4.2's principal production")
            }
            // The lint tokens, in the order the reader sorted them, so the
            // message names the same statuses G13 and G16 would raise.
            SeedError::Keyring(findings) => {
                let tokens: Vec<&str> = findings.iter().map(|entry| entry.lint.token()).collect();
                write!(f, "the keyring would not lint clean: {}", tokens.join(", "))
            }
        }
    }
}

/// Read one `.pub` file's text into a key, with the two refusals MF §4.4
/// names.
///
/// The fingerprint is computed by handing a probe line to the reader rather
/// than by decoding base64 here. That is the point: the writer must not be able
/// to accept a blob the lint would reject, and one implementation of both
/// limbs of `keyring-key-not-base64` — *"not base64, or that does not decode to
/// a key of the declared type"* — is how that is guaranteed rather than
/// reviewed.
pub fn read_public_key(text: &str) -> Result<PublicKey, SeedError> {
    // `.pub` is one key. `ssh-add -L` with two keys is exactly the ambiguity
    // PB §11 refuses, and refusing it here too means no caller can launder two
    // keys into one enrolment by concatenation.
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let line = lines
        .next()
        .ok_or_else(|| SeedError::NotOnePublicKey("no key line".into()))?;
    if lines.next().is_some() {
        return Err(SeedError::NotOnePublicKey(
            "more than one key; --signer-key names exactly one".into(),
        ));
    }

    let line = line.trim_matches([' ', '\t']);
    let (keytype, rest) = line
        .split_once([' ', '\t'])
        .ok_or_else(|| SeedError::NotOnePublicKey("no key blob after the key type".into()))?;
    let rest = rest.trim_start_matches([' ', '\t']);
    // MF §4.2: "A trailing comment-text after the key blob is accepted and
    // ignored — it is where `ssh-keygen` puts a key's own comment". Kept whole,
    // spaces and all, because PB §11 reads it as a principal and a principal
    // with a space in it must be refused, not silently truncated.
    let (keyblob, comment) = match rest.split_once([' ', '\t']) {
        Some((blob, comment)) => (blob, Some(comment.trim_matches([' ', '\t']).to_string())),
        None => (rest, None),
    };
    if keyblob.is_empty() {
        return Err(SeedError::NotOnePublicKey("empty key blob".into()));
    }

    if !KEYTYPES.contains(&keytype) {
        return Err(SeedError::KeytypeUnknown(keytype.to_string()));
    }

    let fingerprint = fingerprint_of(keytype, keyblob)?;
    Ok(PublicKey {
        keytype: keytype.to_string(),
        keyblob: keyblob.to_string(),
        comment: comment.filter(|c| !c.is_empty()),
        fingerprint,
    })
}

/// Enrol a key read by [`read_public_key`] under a role.
///
/// PB §11: *"`--identity` names the principal, defaulting to the key's
/// comment."* Absent both, there is nothing to default to and
/// [`SeedError::NoPrincipal`] is the answer — a repository's authority is not
/// something to invent a name for.
pub fn enrol(key: PublicKey, identity: Option<&str>, role: Role) -> Result<SeedEntry, SeedError> {
    let principal = match identity {
        Some(id) => id.to_string(),
        None => key.comment.clone().ok_or(SeedError::NoPrincipal)?,
    };
    check_principal(&principal)?;
    Ok(SeedEntry {
        principal,
        key,
        role,
    })
}

/// Render the seed for a set of enrolled keys.
///
/// Two rules decide the namespace set of a human line, and only two:
///
/// - `--signer-key` *"enrols a human signing key in the keyring under
///   `spine-signoff@v1` and `spine-review@v1`"* (PB §11, CLI).
/// - PB §11, *Roles and namespaces*: *"Solo mode = exactly one signoff key
///   (`C-A1`), whose principal then holds all three namespaces."*
///
/// The second applies when this seed **is** a solo keyring and nothing else
/// holds the seal — PB §7.2's pipeline role is *"the trusted stage … in solo
/// mode, the human's own key"*, and a lone human with no CI has to be able to
/// seal their own landings or nothing lands at all.
///
/// DERIVED: the corpus does not say what a *solo* keyring with a separate
/// `--pipeline-key` looks like. Both readings lint clean — MF §4.5 evaluates
/// `keyring-seal-mixed` **only in team mode**, so a solo human may or may not
/// keep the seal without changing any verdict. This renders the narrower one:
/// once a pipeline key exists it holds the seal, because PB §7.2's justification
/// for a human seal ("in solo mode, the human's own key") is the absence of a
/// trusted stage, not the smallness of the team.
///
/// Line order is humans in the order given, then pipelines. PB §6.7 fixes the
/// second half — `--pipeline-key` *"appends the seal line"* — and MF §8.7's
/// published file is `alice`, `bob`, `ci` in exactly that shape.
pub fn render_seed(entries: &[SeedEntry]) -> Result<String, SeedError> {
    for entry in entries {
        check_principal(&entry.principal)?;
    }

    // MF §4.5's mode, computed the one way it is ever computed: distinct
    // signoff **fingerprints**. Counting `Role::Human` entries instead would
    // read `team` for the two-principals-one-key keyring
    // `keyring-key-two-principals` exists to refuse.
    let mut signoff: Vec<&str> = entries
        .iter()
        .filter(|e| e.role == Role::Human)
        .map(|e| e.key.fingerprint.as_str())
        .collect();
    signoff.sort_unstable();
    signoff.dedup();
    let solo = signoff.len() == 1;
    let has_pipeline = entries.iter().any(|e| e.role == Role::Pipeline);
    let human_seals = solo && !has_pipeline;

    let mut out = String::new();
    for role in [Role::Human, Role::Pipeline] {
        for entry in entries.iter().filter(|e| e.role == role) {
            let namespaces: Vec<&str> = match role {
                Role::Human if human_seals => vec![SIGNOFF, REVIEW, SEAL],
                Role::Human => vec![SIGNOFF, REVIEW],
                Role::Pipeline => vec![SEAL],
            };
            out.push_str(&render_line(&entry.principal, &namespaces, &entry.key));
        }
    }

    lint_clean(&out)?;
    Ok(out)
}

/// Append `--pipeline-key`'s seal line to an existing keyring.
///
/// PB §6.7, verbatim: *"`--pipeline-key`, which appends the seal line to the
/// keyring: that landing is a keyring change under the chain rule (§7.5), and
/// **in team mode it strips the seal namespace from every human line**; G13
/// refuses a team-mode keyring with no `spine-seal@v1` principal — so a repo
/// that starts solo and offline can grow a remote and a pipeline without a
/// second bootstrap."*
///
/// Three properties this function has to keep at once:
///
/// 1. **It edits, it does not re-render.** MF §4.1 gives the keyring no
///    canonical form so that re-indenting is not a gate failure; rewriting a
///    human's file into this module's line shape would impose one through the
///    back door and turn every later `--pipeline-key` into a whitespace diff
///    two protected reviewers have to read past (MF §13 OPEN-2). So the strip
///    is a splice inside one `namespaces="…"` value and every other byte of
///    every other line survives.
/// 2. **The strip is team-only.** MF §4.5: *"In **solo** mode the rule is
///    inverted by definition: the one principal holds all three namespaces …
///    so `keyring-seal-mixed` is evaluated only when `mode = \"team\"`."*
///    Stripping a solo human's seal would leave a repository that cannot seal
///    its own landings.
/// 3. **A malformed input is refused, with two exceptions.** `keyring-seal-mixed`
///    and `keyring-no-seal` are the two findings this call exists to repair —
///    they are exactly the state a keyring is in between entering team mode
///    (PB §7.2: *"the landing that enters team mode strips the seal namespace
///    from every human line"*) and acquiring a pipeline key. Refusing them
///    would make the strip above unreachable. Every other finding stands: this
///    call cannot fix it and must not bury it.
pub fn append_pipeline_key(existing: &[u8], entry: &SeedEntry) -> Result<String, SeedError> {
    check_principal(&entry.principal)?;

    // MF §4.2 admits only %x21-7E and WS in a line, so a non-UTF-8 byte cannot
    // sit in any of `blank`, `comment` or `entry` — it is
    // `keyring-line-malformed` and gets that token rather than a new one.
    let text = core::str::from_utf8(existing).map_err(|_| {
        SeedError::Keyring(vec![Finding {
            lint: Lint::KeyringLineMalformed,
            line_no: None,
            detail: "the file is not UTF-8; MF §4.2 admits %x21-7E and WS".into(),
        }])
    })?;

    let keyring = Keyring::parse(existing);
    let repairable = [Lint::KeyringSealMixed, Lint::KeyringNoSeal];
    let blocking: Vec<Finding> = keyring
        .findings
        .iter()
        .filter(|f| !repairable.contains(&f.lint))
        .cloned()
        .collect();
    if !blocking.is_empty() {
        return Err(SeedError::Keyring(blocking));
    }

    // Which 1-based lines carry a human entry still holding the seal. "Human
    // line" is read from the namespaces, not from a role the file does not
    // record: a line holding `spine-signoff@v1` or `spine-review@v1` is a
    // person's. A seal-only line is a previous pipeline key and is left alone —
    // stripping it would leave `namespaces=""`, MF §4.4's
    // `keyring-namespace-empty`, "a key with no role".
    let strip: Vec<usize> = if keyring.mode == Mode::Team {
        keyring
            .entries
            .iter()
            .filter(|e| {
                e.namespaces.iter().any(|n| n == SEAL)
                    && e.namespaces.iter().any(|n| n == SIGNOFF || n == REVIEW)
            })
            .map(|e| e.line_no)
            .collect()
    } else {
        Vec::new()
    };

    let mut out = String::with_capacity(text.len() + 160);
    // MF §4.2: "a final line without a terminator is accepted (OpenSSH accepts
    // it) and is not an error", so the last line may have no LF and appending
    // to it directly would fuse two entries into one.
    let mut lines: Vec<&str> = text.split('\n').collect();
    let trailing_lf = lines.last() == Some(&"");
    if trailing_lf {
        lines.pop();
    }
    for (index, raw) in lines.iter().enumerate() {
        if strip.contains(&(index + 1)) {
            out.push_str(&strip_seal(raw));
        } else {
            out.push_str(raw);
        }
        out.push('\n');
    }

    out.push_str(&render_line(&entry.principal, &[SEAL], &entry.key));

    // The two repairable findings were tolerated on the way in; they are not
    // tolerated on the way out.
    lint_clean(&out)?;
    Ok(out)
}

/// One entry line. Single-space separated, no trailing comment-text.
///
/// The key's own comment is deliberately dropped: MF §8.7's three published
/// entries carry none, and PB §11 spends the comment on the principal instead —
/// writing it twice would put a second, unauthoritative spelling of the
/// identity on the line that grants authority.
fn render_line(principal: &str, namespaces: &[&str], key: &PublicKey) -> String {
    // Driven off WIRE_ORDER rather than off `namespaces`, so the caller cannot
    // choose the wire order and the render can never emit a duplicate token.
    let ordered: Vec<&str> = WIRE_ORDER
        .iter()
        .copied()
        .filter(|n| namespaces.contains(n))
        .collect();
    format!(
        "{principal} namespaces=\"{}\" {} {}\n",
        ordered.join(","),
        key.keytype,
        key.keyblob
    )
}

/// MF §4.2's `principal := 1*( %x21-7E except "," and "#" and WS )`.
///
/// Each excluded byte is a distinct malformed keyring rather than a cosmetic
/// problem: a comma is `keyring-multi-principal` ("a comma list makes one key
/// reach several identities on one line"), whitespace shifts every later field
/// by one so the `namespaces=` option is read as the principal's continuation,
/// and `#` turns the line into a comment — an entry that vanishes rather than
/// fails.
fn check_principal(principal: &str) -> Result<(), SeedError> {
    if principal.is_empty() {
        return Err(SeedError::PrincipalMalformed("empty".into()));
    }
    for byte in principal.bytes() {
        if !(0x21..=0x7E).contains(&byte) || byte == b',' || byte == b'#' {
            return Err(SeedError::PrincipalMalformed(principal.to_string()));
        }
    }
    Ok(())
}

/// `ssh-keygen -lf` over `<keytype> <keyblob>`, via the reader.
///
/// The probe is a whole one-entry keyring because that is the only shape the
/// reader parses, and a one-signoff-key keyring is solo, so neither team-only
/// finding fires on it and any finding at all is the key's.
fn fingerprint_of(keytype: &str, keyblob: &str) -> Result<String, SeedError> {
    let probe = format!("spine-probe namespaces=\"{SIGNOFF}\" {keytype} {keyblob}\n");
    let keyring = Keyring::parse(probe.as_bytes());
    if let Some(finding) = keyring
        .findings
        .iter()
        .find(|f| f.lint == Lint::KeyringKeyNotBase64)
    {
        return Err(SeedError::KeyNotBase64(finding.detail.clone()));
    }
    if !keyring.findings.is_empty() {
        return Err(SeedError::Keyring(keyring.findings));
    }
    keyring
        .entries
        .first()
        .map(|e| e.fingerprint.clone())
        .ok_or_else(|| SeedError::KeyNotBase64("the probe line parsed to no entry".into()))
}

/// Remove `spine-seal@v1` from one line's `namespaces="…"` value, leaving every
/// other byte of the line untouched.
fn strip_seal(line: &str) -> String {
    let Some(start) = line.find("namespaces=\"") else {
        return line.to_string();
    };
    let value_start = start + "namespaces=\"".len();
    let Some(length) = line[value_start..].find('"') else {
        return line.to_string();
    };
    let value_end = value_start + length;
    let kept: Vec<&str> = line[value_start..value_end]
        .split(',')
        .filter(|token| *token != SEAL)
        .collect();
    format!(
        "{}{}{}",
        &line[..value_start],
        kept.join(","),
        &line[value_end..]
    )
}

/// Run bytes this module produced back through the reader and refuse on any
/// finding.
///
/// The writer never emits a keyring the lint would refuse — not as a courtesy,
/// but because there is no keyring below the seed to repair it from: G16 lints
/// `K_T` and G13 lints `K_B` (MF §4.5), and a seeded finding fails the first
/// landing of the repository.
fn lint_clean(text: &str) -> Result<(), SeedError> {
    let keyring = Keyring::parse(text.as_bytes());
    if keyring.findings.is_empty() {
        Ok(())
    } else {
        Err(SeedError::Keyring(keyring.findings))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spine_canon::{ObjectFormat, git_blob_id};

    /// MF §8.7's published keyring, extracted from the spec's own fenced block.
    const MF_8_7: &str = include_str!("../tests/vectors/mf-8.7-allowed_signers");

    /// EV §8.1's three keys as MF §8.7 prints them. No private key is
    /// published and none is needed to verify.
    const ALICE: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla alice@example.com";
    const BOB: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINZJBgqcpDmx19xO9D29xeFtCCUMyfe/ti+lY7c+rvim bob@example.com";
    const CI: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICE3UkO6VDz+9ag4xQScwsfdP8PHJhLh+QWkIwzUjWze ci@example.com";

    fn entry(pubkey: &str, role: Role) -> SeedEntry {
        enrol(read_public_key(pubkey).unwrap(), None, role).unwrap()
    }

    fn published() -> Vec<SeedEntry> {
        vec![
            entry(ALICE, Role::Human),
            entry(BOB, Role::Human),
            entry(CI, Role::Pipeline),
        ]
    }

    /// MF §8.7: "411 bytes, three entries, blob
    /// `6d4db08390092d7d5d96476eddca6355815bc49f`."
    ///
    /// Both numbers are computed here from the bytes the render produced, and
    /// the vector file is the spec's block verbatim, so the equality is the
    /// check rather than the claim.
    #[test]
    fn mf_8_7s_three_keys_render_the_published_411_bytes() {
        let rendered = render_seed(&published()).unwrap();
        assert_eq!(rendered, MF_8_7);
        assert_eq!(rendered.len(), 411);
        assert_eq!(
            git_blob_id(rendered.as_bytes(), ObjectFormat::Sha1),
            "6d4db08390092d7d5d96476eddca6355815bc49f"
        );
    }

    /// MF §8.7's walk of the lint, end to end: "two distinct signoff keys, so
    /// `mode = team`; `ci@example.com` holds `spine-seal@v1` and nothing else
    /// and no human holds it, so no `keyring-seal-mixed`; a seal principal
    /// exists, so no `keyring-no-seal`. Clean."
    #[test]
    fn the_published_seed_lints_clean_in_team_mode() {
        let rendered = render_seed(&published()).unwrap();
        let keyring = Keyring::parse(rendered.as_bytes());
        assert_eq!(keyring.findings, Vec::new());
        assert_eq!(keyring.mode, Mode::Team);
        assert_eq!(keyring.entries.len(), 3);
    }

    /// MF §8.7's fingerprint table, which that section publishes precisely so
    /// this reproduction can be the check.
    #[test]
    fn the_three_fingerprints_reproduce_mf_8_7s_table() {
        let entries = published();
        assert_eq!(
            entries[0].key.fingerprint,
            "SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM"
        );
        assert_eq!(
            entries[1].key.fingerprint,
            "SHA256:V2dasTIGWUnhlaxa7vr3Qmqpe/qYBoxi+C7Cs6yxxEs"
        );
        assert_eq!(
            entries[2].key.fingerprint,
            "SHA256:eQ0ZoC+rlhZstRuvhLXwJlwqLAreBcmnyFedpvPmTVY"
        );
    }

    /// MF §4.6 sorts a `signer` node's `roles` ascending by bytes, which would
    /// put `spine-review@v1` first. The **file** does not: MF §8.7's line reads
    /// signoff then review, and sorting would move the 411 bytes.
    #[test]
    fn the_namespace_wire_order_is_mf_4_3s_role_order_not_byte_order() {
        let rendered = render_seed(&published()).unwrap();
        assert!(rendered.contains("namespaces=\"spine-signoff@v1,spine-review@v1\""));
        assert!(!rendered.contains("namespaces=\"spine-review@v1,spine-signoff@v1\""));
    }

    /// PB §11, *Roles and namespaces*: "Solo mode = exactly one signoff key
    /// (`C-A1`), whose principal then holds all three namespaces." With no
    /// pipeline key the human is the pipeline (PB §7.2), and MF §4.5 does not
    /// evaluate `keyring-seal-mixed` in solo mode, so this lints clean.
    #[test]
    fn a_lone_signer_holds_all_three_namespaces() {
        let rendered = render_seed(&[entry(ALICE, Role::Human)]).unwrap();
        assert_eq!(
            rendered,
            "alice@example.com namespaces=\"spine-signoff@v1,spine-review@v1,spine-seal@v1\" \
             ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla\n"
        );
        let keyring = Keyring::parse(rendered.as_bytes());
        assert_eq!(keyring.mode, Mode::Solo);
        assert_eq!(keyring.findings, Vec::new());
    }

    /// The DERIVED half of the same rule: once a pipeline key exists it holds
    /// the seal, and the human does not. Still solo — MF §4.5 counts signoff
    /// fingerprints, and the seal-only line adds none.
    #[test]
    fn a_lone_signer_beside_a_pipeline_key_does_not_hold_the_seal() {
        let rendered =
            render_seed(&[entry(ALICE, Role::Human), entry(CI, Role::Pipeline)]).unwrap();
        assert!(
            rendered.contains("alice@example.com namespaces=\"spine-signoff@v1,spine-review@v1\" ")
        );
        assert!(rendered.contains("ci@example.com namespaces=\"spine-seal@v1\" "));
        let keyring = Keyring::parse(rendered.as_bytes());
        assert_eq!(keyring.mode, Mode::Solo);
        assert_eq!(keyring.findings, Vec::new());
    }

    /// PB §6.7: "G13 refuses a team-mode keyring with no `spine-seal@v1`
    /// principal". Two humans and no pipeline key is that keyring, and the
    /// writer refuses to be the thing that creates it.
    #[test]
    fn two_humans_and_no_pipeline_key_is_refused_as_keyring_no_seal() {
        let error = render_seed(&[entry(ALICE, Role::Human), entry(BOB, Role::Human)]).unwrap_err();
        let SeedError::Keyring(findings) = error else {
            panic!("expected the reader's own findings");
        };
        assert_eq!(
            findings.iter().map(|f| f.lint).collect::<Vec<_>>(),
            vec![Lint::KeyringNoSeal]
        );
    }

    /// PB §11: "`--identity` names the principal, defaulting to the key's
    /// comment."
    #[test]
    fn the_principal_is_identity_then_the_keys_comment() {
        let key = read_public_key(ALICE).unwrap();
        assert_eq!(key.comment.as_deref(), Some("alice@example.com"));
        assert_eq!(
            enrol(key.clone(), None, Role::Human).unwrap().principal,
            "alice@example.com"
        );
        assert_eq!(
            enrol(key, Some("alice+yubikey@example.com"), Role::Human)
                .unwrap()
                .principal,
            "alice+yubikey@example.com"
        );
    }

    /// A commentless key with no `--identity` has no principal, and PB §11's
    /// discovery rule is a refusal rather than a guess.
    #[test]
    fn a_commentless_key_with_no_identity_is_refused() {
        let key = read_public_key(
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla",
        )
        .unwrap();
        assert_eq!(key.comment, None);
        assert_eq!(
            enrol(key, None, Role::Human).unwrap_err(),
            SeedError::NoPrincipal
        );
    }

    /// The key's own comment is not written to the line. MF §8.7's entries end
    /// at the blob, and the 411 bytes depend on it.
    #[test]
    fn the_keys_comment_is_not_written_to_the_line() {
        let key = read_public_key(ALICE).unwrap();
        let rendered = render_seed(&[enrol(key.clone(), None, Role::Human).unwrap()]).unwrap();
        // The comment and the principal are the same bytes here, and appear
        // once: as the principal, at the head of the line.
        assert_eq!(rendered.matches("alice@example.com").count(), 1);
        assert!(rendered.starts_with("alice@example.com "));
        assert!(
            rendered.ends_with(&format!("{}\n", key.keyblob)),
            "the line ends at the key blob"
        );
    }

    /// MF §4.2's keytype list, and why `ssh-rsa` is not on it: "OpenSSH >= 8.2
    /// is a stated requirement (PB §11) and SHA-1 RSA signatures are the one
    /// thing that release deprecated."
    #[test]
    fn a_keytype_outside_mf_4_2s_list_is_refused() {
        assert_eq!(
            read_public_key("ssh-rsa AAAAB3NzaC1yc2E= alice@example.com").unwrap_err(),
            SeedError::KeytypeUnknown("ssh-rsa".into())
        );
        assert_eq!(
            read_public_key("ssh-dss AAAAB3NzaC1kc3M= alice@example.com").unwrap_err(),
            SeedError::KeytypeUnknown("ssh-dss".into())
        );
    }

    /// MF §4.4's `keyring-key-not-base64`, second limb: "a key blob that is not
    /// base64, **or that does not decode to a key of the declared type**". The
    /// blob below is a real ed25519 key under an RSA declaration.
    #[test]
    fn a_blob_of_another_keytype_is_refused_under_the_declared_one() {
        let error = read_public_key(
            "rsa-sha2-256 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla a@b",
        )
        .unwrap_err();
        let SeedError::KeyNotBase64(detail) = error else {
            panic!("expected keyring-key-not-base64");
        };
        assert!(detail.contains("ssh-ed25519"), "{detail}");
    }

    /// First limb of the same row. MF §4.2's `keyblob` admits only
    /// `ALPHA / DIGIT / "+" / "/" / "="`.
    #[test]
    fn a_blob_that_is_not_base64_is_refused() {
        let error = read_public_key("ssh-ed25519 not!base64 a@b").unwrap_err();
        assert!(matches!(error, SeedError::KeyNotBase64(_)), "{error:?}");
    }

    /// MF §4.2's `principal` production. A comma would be
    /// `keyring-multi-principal`; a `#` would turn the entry into a comment and
    /// vanish rather than fail; whitespace shifts every later field by one.
    #[test]
    fn a_principal_outside_mf_4_2s_production_is_refused() {
        let key = read_public_key(ALICE).unwrap();
        for bad in ["alice,bob", "alice#1", "alice smith", ""] {
            assert!(
                matches!(
                    enrol(key.clone(), Some(bad), Role::Human),
                    Err(SeedError::PrincipalMalformed(_))
                ),
                "{bad:?} was accepted"
            );
        }
    }

    /// PB §6.7: "in team mode it strips the seal namespace from every human
    /// line". The input is the keyring a landing that entered team mode leaves
    /// behind — alice still holds the seal she held when she was solo.
    #[test]
    fn append_pipeline_key_strips_the_seal_from_human_lines_in_team_mode() {
        let before = concat!(
            "alice@example.com namespaces=\"spine-signoff@v1,spine-review@v1,spine-seal@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla\n",
            "bob@example.com namespaces=\"spine-signoff@v1,spine-review@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINZJBgqcpDmx19xO9D29xeFtCCUMyfe/ti+lY7c+rvim\n",
        );
        // The state this call exists to repair: team mode, alice seal-mixed.
        let stale = Keyring::parse(before.as_bytes());
        assert_eq!(stale.mode, Mode::Team);
        assert_eq!(
            stale.findings.iter().map(|f| f.lint).collect::<Vec<_>>(),
            vec![Lint::KeyringSealMixed]
        );

        let after = append_pipeline_key(before.as_bytes(), &entry(CI, Role::Pipeline)).unwrap();
        assert_eq!(
            after, MF_8_7,
            "the repair lands on MF §8.7's published file"
        );
        assert_eq!(Keyring::parse(after.as_bytes()).findings, Vec::new());
    }

    /// MF §4.5: "In **solo** mode the rule is inverted by definition: the one
    /// principal holds all three namespaces … so `keyring-seal-mixed` is
    /// evaluated only when `mode = \"team\"`." Stripping here would leave a
    /// solo repository unable to seal any landing but this one.
    #[test]
    fn append_pipeline_key_leaves_a_solo_humans_seal_namespace_alone() {
        let before = render_seed(&[entry(ALICE, Role::Human)]).unwrap();
        let after = append_pipeline_key(before.as_bytes(), &entry(CI, Role::Pipeline)).unwrap();
        assert!(after.starts_with(&before), "the human line is untouched");
        assert!(after.contains("spine-signoff@v1,spine-review@v1,spine-seal@v1"));
        assert!(after.ends_with(
            "ci@example.com namespaces=\"spine-seal@v1\" ssh-ed25519 \
             AAAAC3NzaC1lZDI1NTE5AAAAICE3UkO6VDz+9ag4xQScwsfdP8PHJhLh+QWkIwzUjWze\n"
        ));
        assert_eq!(Keyring::parse(after.as_bytes()).findings, Vec::new());
    }

    /// MF §4.1: the keyring has no canonical byte form "precisely so that
    /// re-indenting is not a gate failure", so an append must not re-render.
    /// Comments, blank lines, tab and multi-space alignment and a trailing
    /// comment-text all survive byte for byte, and only the one spliced
    /// `namespaces=` value moves.
    #[test]
    fn append_pipeline_key_preserves_every_byte_it_did_not_have_to_change() {
        let before = concat!(
            "# .spine/allowed_signers — roles are namespaces\n",
            "\n",
            "alice@example.com\tnamespaces=\"spine-signoff@v1,spine-seal@v1,spine-review@v1\"  ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla alice's laptop\n",
            "bob@example.com   namespaces=\"spine-signoff@v1,spine-review@v1\"                 ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINZJBgqcpDmx19xO9D29xeFtCCUMyfe/ti+lY7c+rvim\n",
        );
        let after = append_pipeline_key(before.as_bytes(), &entry(CI, Role::Pipeline)).unwrap();
        let before_lines: Vec<&str> = before.lines().collect();
        let after_lines: Vec<&str> = after.lines().collect();

        assert_eq!(before_lines[0], after_lines[0], "the comment survives");
        assert_eq!(before_lines[1], after_lines[1], "the blank line survives");
        assert_eq!(
            after_lines[2],
            "alice@example.com\tnamespaces=\"spine-signoff@v1,spine-review@v1\"  ssh-ed25519 \
             AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla alice's laptop",
            "the tab, the double space and the trailing comment-text all survive; \
             only spine-seal@v1 left the option value"
        );
        assert_eq!(before_lines[3], after_lines[3], "bob's alignment survives");
        assert_eq!(after_lines.len(), 5);
        assert_eq!(Keyring::parse(after.as_bytes()).findings, Vec::new());
    }

    /// MF §4.2: "a final line without a terminator is accepted (OpenSSH accepts
    /// it) and is not an error" — so a human's last line may have no LF, and
    /// appending to it directly would fuse two entries into one.
    #[test]
    fn append_pipeline_key_terminates_an_unterminated_last_line() {
        let before = render_seed(&[entry(ALICE, Role::Human)]).unwrap();
        let unterminated = before.trim_end_matches('\n');
        let after =
            append_pipeline_key(unterminated.as_bytes(), &entry(CI, Role::Pipeline)).unwrap();
        assert_eq!(after.lines().count(), 2);
        assert_eq!(Keyring::parse(after.as_bytes()).findings, Vec::new());
    }

    /// The two findings `--pipeline-key` repairs are tolerated on the way in;
    /// nothing else is. A CR is `keyring-cr` and no append can fix it, so it is
    /// reported rather than carried forward into a file that lints worse.
    #[test]
    fn append_pipeline_key_refuses_a_keyring_it_cannot_repair() {
        let before = "alice@example.com namespaces=\"spine-signoff@v1,spine-review@v1,spine-seal@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla\r\n";
        let error = append_pipeline_key(before.as_bytes(), &entry(CI, Role::Pipeline)).unwrap_err();
        let SeedError::Keyring(findings) = error else {
            panic!("expected the reader's own findings");
        };
        assert!(findings.iter().any(|f| f.lint == Lint::KeyringCr));
    }

    /// MF §4.5, forced by DM §5.2: two keys under one principal is
    /// `keyring-duplicate-principal` — "an unrepresentable graph, and G10 diffs
    /// node ids before every landing". The remedy is a second principal, "which
    /// costs one line and is what `--signer-key` already produces".
    #[test]
    fn one_principal_with_two_keys_is_refused() {
        let alice = enrol(read_public_key(ALICE).unwrap(), None, Role::Human).unwrap();
        let twin = enrol(
            read_public_key(BOB).unwrap(),
            Some("alice@example.com"),
            Role::Human,
        )
        .unwrap();
        let error = render_seed(&[alice, twin, entry(CI, Role::Pipeline)]).unwrap_err();
        let SeedError::Keyring(findings) = error else {
            panic!("expected the reader's own findings");
        };
        assert!(
            findings
                .iter()
                .any(|f| f.lint == Lint::KeyringDuplicatePrincipal)
        );
    }

    /// MF §4.5: "one key (by fingerprint) under two principals" is
    /// `keyring-key-two-principals`, because `reviewer != signer` compares
    /// fingerprints and "one key wearing two names would satisfy it under one
    /// name and fail under the other".
    #[test]
    fn one_key_under_two_principals_is_refused() {
        let alice = enrol(read_public_key(ALICE).unwrap(), None, Role::Human).unwrap();
        let alias = enrol(
            read_public_key(ALICE).unwrap(),
            Some("alice2@example.com"),
            Role::Human,
        )
        .unwrap();
        let error = render_seed(&[alice, alias, entry(CI, Role::Pipeline)]).unwrap_err();
        let SeedError::Keyring(findings) = error else {
            panic!("expected the reader's own findings");
        };
        assert!(
            findings
                .iter()
                .any(|f| f.lint == Lint::KeyringKeyTwoPrincipals)
        );
    }

    /// MF §4.4's `keyring-cr`, from the writer's side: ".gitattributes pins
    /// `eol=lf` on `.spine/**` (ID §2.5); a CR forks the blob G16 compares."
    /// Nothing this module emits carries one.
    #[test]
    fn the_seed_carries_no_cr_and_ends_in_one_lf() {
        let rendered = render_seed(&published()).unwrap();
        assert!(!rendered.contains('\r'));
        assert!(rendered.ends_with('\n'));
        assert!(!rendered.ends_with("\n\n"));
    }

    /// MF §4.7: the keyring "is not versioned: there is no `Keyring: v<n>`
    /// line, and `templates.keyring` names the seed's template, never the
    /// file's content". It also carries no policy — `C-A1` lives in the
    /// constitution — and no header of any kind.
    #[test]
    fn the_seed_carries_no_version_line_and_no_policy() {
        let rendered = render_seed(&published()).unwrap();
        assert!(!rendered.contains("Keyring:"));
        assert!(!rendered.contains("C-A1"));
        assert_eq!(
            rendered.lines().count(),
            3,
            "three entry lines and nothing else"
        );
        assert_eq!(TEMPLATE, "keyring@1", "MF §8.3's files[] record");
    }
}
