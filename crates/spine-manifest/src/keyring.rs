//! `.spine/allowed_signers` — the keyring, its lint, and the values G13 reads.
//!
//! Two properties shape everything here.
//!
//! **The keyring has no canonical byte form** (MF §4.1) — the opposite of the
//! manifest's rule, and for the opposite reason: it is `user-owned`, humans
//! edit it under a protected PR, and "requiring canonical bytes would make
//! re-indenting a gate failure".
//!
//! **The grammar is a lint, not the parser verification depends on** (MF §4.1).
//! Signature verification is `ssh-keygen -Y verify` and "OpenSSH decides"
//! (MF §12). What this module produces is a set of findings and the derived
//! values — `mode`, the fingerprints, the roles — that G13 and G16 read.
//!
//! MF §4.2's `entry` production is too tight to implement literally: §4.4 needs
//! a distinct status for a line carrying `cert-authority`, `valid-after=`,
//! `namespaces=""`, a typo'd namespace or `ssh-rsa`, and none of those matches
//! `entry`. So this **field-splits permissively and then classifies**, and
//! reserves `keyring-line-malformed` for a line that cannot be split at all.

use core::fmt;

/// MF §4.3, and the domain is closed. An unknown token is
/// `keyring-namespace-unknown` and is **never ignored**: "an ignored token is a
/// role nobody can audit and a typo (`spine-signof@v1`) silently removes a
/// signer's authority while leaving the line looking correct."
pub const NAMESPACES: [&str; 3] = ["spine-review@v1", "spine-seal@v1", "spine-signoff@v1"];

/// MF §4.2's keytype list. **`ssh-rsa` is deliberately absent**: OpenSSH >= 8.2
/// is a stated requirement (PB §11) and SHA-1 RSA signatures are the one thing
/// that release deprecated.
pub const KEYTYPES: [&str; 8] = [
    "ecdsa-sha2-nistp256",
    "ecdsa-sha2-nistp384",
    "ecdsa-sha2-nistp521",
    "rsa-sha2-256",
    "rsa-sha2-512",
    "sk-ecdsa-sha2-nistp256@openssh.com",
    "sk-ssh-ed25519@openssh.com",
    "ssh-ed25519",
];

/// MF §4.4's closed list. No other condition makes a keyring malformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lint {
    KeyringMissing,
    KeyringEmpty,
    KeyringLineMalformed,
    KeyringCr,
    KeyringMultiPrincipal,
    KeyringNoNamespaces,
    KeyringOptionUnknown,
    KeyringValidityOption,
    KeyringCertAuthority,
    KeyringNamespaceUnknown,
    KeyringNamespaceEmpty,
    KeyringKeytypeUnknown,
    KeyringKeyNotBase64,
    KeyringDuplicateLine,
    KeyringDuplicatePrincipal,
    KeyringKeyTwoPrincipals,
    KeyringSealMixed,
    KeyringNoSeal,
}

impl Lint {
    pub fn token(self) -> &'static str {
        match self {
            Lint::KeyringMissing => "keyring-missing",
            Lint::KeyringEmpty => "keyring-empty",
            Lint::KeyringLineMalformed => "keyring-line-malformed",
            Lint::KeyringCr => "keyring-cr",
            Lint::KeyringMultiPrincipal => "keyring-multi-principal",
            Lint::KeyringNoNamespaces => "keyring-no-namespaces",
            Lint::KeyringOptionUnknown => "keyring-option-unknown",
            Lint::KeyringValidityOption => "keyring-validity-option",
            Lint::KeyringCertAuthority => "keyring-cert-authority",
            Lint::KeyringNamespaceUnknown => "keyring-namespace-unknown",
            Lint::KeyringNamespaceEmpty => "keyring-namespace-empty",
            Lint::KeyringKeytypeUnknown => "keyring-keytype-unknown",
            Lint::KeyringKeyNotBase64 => "keyring-key-not-base64",
            Lint::KeyringDuplicateLine => "keyring-duplicate-line",
            Lint::KeyringDuplicatePrincipal => "keyring-duplicate-principal",
            Lint::KeyringKeyTwoPrincipals => "keyring-key-two-principals",
            Lint::KeyringSealMixed => "keyring-seal-mixed",
            Lint::KeyringNoSeal => "keyring-no-seal",
        }
    }
}

impl fmt::Display for Lint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// A lint finding, with the 1-based line it was found on where there is one.
///
/// The line number is not decoration: DM's `signer` node provenance is
/// `git:<sha>:.spine/allowed_signers:<line>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub lint: Lint,
    pub line_no: Option<usize>,
    pub detail: String,
}

/// MF §4.5, PB §11. Computed **from the key count, never from `C-A1`** — a
/// `C-A1` that disagrees is a warning, not a finding, and not an input to any
/// check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Exactly one distinct fingerprint under `spine-signoff@v1`. That
    /// principal then holds all three namespaces (PB §11).
    Solo,
    Team,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Solo => "solo",
            Mode::Team => "team",
        }
    }
}

/// One parsed entry line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub line_no: usize,
    pub principal: String,
    /// Ascending by bytes — the order DM §7.2 exports as a `signer` node's
    /// `roles` attr.
    pub namespaces: Vec<String>,
    pub keytype: String,
    pub keyblob: String,
    /// `"SHA256:"` + unpadded base64, exactly what `ssh-keygen -lf` prints.
    /// **This** is what `reviewer != signer` compares, never the principal
    /// (PB §7.2, GR §5.5).
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keyring {
    pub entries: Vec<Entry>,
    pub findings: Vec<Finding>,
    pub mode: Mode,
}

impl Keyring {
    /// The file is absent from the tree. "There is no authority without it."
    pub fn missing() -> Self {
        Keyring {
            entries: Vec::new(),
            findings: vec![Finding {
                lint: Lint::KeyringMissing,
                line_no: None,
                detail: ".spine/allowed_signers is absent".into(),
            }],
            // Immaterial: check 1 halts before mode is read. `Team` is the
            // fail-closed spelling, since it is the mode that binds
            // `reviewer != signer`.
            mode: Mode::Team,
        }
    }

    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    /// Every distinct fingerprint holding the given namespace.
    pub fn fingerprints_under(&self, namespace: &str) -> Vec<&str> {
        let mut out: Vec<&str> = self
            .entries
            .iter()
            .filter(|e| e.namespaces.iter().any(|n| n == namespace))
            .map(|e| e.fingerprint.as_str())
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    pub fn by_fingerprint(&self, fingerprint: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.fingerprint == fingerprint)
    }

    /// Parse and lint. Never fails: a malformed keyring is a keyring with
    /// findings, because both G13 and G16 need the findings rather than an
    /// error, and they differ only in the *kind* they raise them at.
    pub fn parse(bytes: &[u8]) -> Self {
        let mut findings = Vec::new();

        // MF §4.4: any 0x0D. ".gitattributes pins eol=lf on .spine/** (ID
        // §2.5); a CR forks the blob G16 compares."
        if bytes.contains(&b'\r') {
            findings.push(Finding {
                lint: Lint::KeyringCr,
                line_no: None,
                detail: "the file contains a CR".into(),
            });
        }

        let mut entries: Vec<Entry> = Vec::new();
        // MF §4.2: "a final line without a terminator is accepted (OpenSSH
        // accepts it) and is not an error", so splitting on LF and dropping a
        // trailing empty piece is the whole rule.
        let text = String::from_utf8_lossy(bytes);
        let mut lines: Vec<&str> = text.split('\n').collect();
        if lines.last() == Some(&"") {
            lines.pop();
        }

        for (index, raw) in lines.iter().enumerate() {
            let line_no = index + 1;
            let trimmed = raw.trim_matches(|c| c == ' ' || c == '\t');
            // blank | comment
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            match parse_entry(trimmed, line_no) {
                Ok((entry, mut entry_findings)) => {
                    findings.append(&mut entry_findings);
                    entries.push(entry);
                }
                Err(mut entry_findings) => findings.append(&mut entry_findings),
            }
        }

        if entries.is_empty() && !findings.iter().any(|f| f.lint == Lint::KeyringEmpty) {
            // "no entry lines" — a file of comments is empty for this purpose.
            findings.push(Finding {
                lint: Lint::KeyringEmpty,
                line_no: None,
                detail: "no entry lines".into(),
            });
        }

        // MF §4.5: mode is the count of distinct signoff **fingerprints**.
        let mut signoff: Vec<&str> = entries
            .iter()
            .filter(|e| e.namespaces.iter().any(|n| n == "spine-signoff@v1"))
            .map(|e| e.fingerprint.as_str())
            .collect();
        signoff.sort_unstable();
        signoff.dedup();
        let mode = if signoff.len() == 1 {
            Mode::Solo
        } else {
            Mode::Team
        };

        cross_entry_lints(&entries, mode, &mut findings);

        findings.sort_by_key(|f| (f.lint, f.line_no));
        Keyring {
            entries,
            findings,
            mode,
        }
    }
}

/// The five findings that need more than one line to decide.
fn cross_entry_lints(entries: &[Entry], mode: Mode, findings: &mut Vec<Finding>) {
    for (i, a) in entries.iter().enumerate() {
        for b in &entries[i + 1..] {
            if a.principal == b.principal {
                if a.fingerprint == b.fingerprint {
                    findings.push(Finding {
                        lint: Lint::KeyringDuplicateLine,
                        line_no: Some(b.line_no),
                        detail: format!("{} repeats line {}", b.principal, a.line_no),
                    });
                } else {
                    // MF §4.5, forced by DM §5.2: "a `signer` node's id is
                    // `signer:` + esc(principal), so two keys under
                    // alice@example.com are two signer nodes with one id …
                    // an unrepresentable graph, and G10 diffs node ids before
                    // every landing." Remedy: a second principal.
                    findings.push(Finding {
                        lint: Lint::KeyringDuplicatePrincipal,
                        line_no: Some(b.line_no),
                        detail: format!(
                            "{} already has a different key on line {}",
                            b.principal, a.line_no
                        ),
                    });
                }
            } else if a.fingerprint == b.fingerprint {
                // "The hazard is `reviewer != signer`, which compares
                // fingerprints — one key wearing two names would satisfy it
                // under one name and fail under the other."
                findings.push(Finding {
                    lint: Lint::KeyringKeyTwoPrincipals,
                    line_no: Some(b.line_no),
                    detail: format!("{} and {} share a key", a.principal, b.principal),
                });
            }
        }
    }

    // MF §4.5 R20: both of these are evaluated **only in team mode**. "In solo
    // mode the rule is inverted by definition: the one principal holds all
    // three namespaces."
    if mode == Mode::Team {
        let mut has_seal = false;
        for entry in entries {
            let holds_seal = entry.namespaces.iter().any(|n| n == "spine-seal@v1");
            if holds_seal {
                has_seal = true;
                // Refused in either direction: a human key also under
                // spine-seal@v1, and the seal key holding anything else.
                if entry.namespaces.len() > 1 {
                    findings.push(Finding {
                        lint: Lint::KeyringSealMixed,
                        line_no: Some(entry.line_no),
                        detail: format!(
                            "{} holds spine-seal@v1 and {}",
                            entry.principal,
                            entry
                                .namespaces
                                .iter()
                                .filter(|n| *n != "spine-seal@v1")
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    });
                }
            }
        }
        if !has_seal {
            // PB §6.7: "G13 refuses a team-mode keyring with no
            // `spine-seal@v1` principal" — it would have nobody who can seal,
            // and every landing would be a recovery landing.
            findings.push(Finding {
                lint: Lint::KeyringNoSeal,
                line_no: None,
                detail: "team mode with no spine-seal@v1 principal".into(),
            });
        }
    }
}

type EntryResult = Result<(Entry, Vec<Finding>), Vec<Finding>>;

/// Field-split permissively, then classify — the implementation note MF §4.2
/// leaves implicit and §4.4's status list forces.
fn parse_entry(line: &str, line_no: usize) -> EntryResult {
    let at = |lint: Lint, detail: String| Finding {
        lint,
        line_no: Some(line_no),
        detail,
    };
    let malformed = |why: &str| {
        vec![Finding {
            lint: Lint::KeyringLineMalformed,
            line_no: Some(line_no),
            detail: why.to_string(),
        }]
    };

    let mut findings = Vec::new();

    // An `allowed_signers` line is: principals WS options WS keytype WS blob
    // [WS comment]. The options field is the one that can contain a quoted
    // list with spaces in it, so split it out by quote-awareness rather than
    // by whitespace alone.
    let (principals, rest) =
        split_ws_once(line).ok_or_else(|| malformed("no fields after the principal"))?;

    // The option field is **optional** in OpenSSH's format, and a line that
    // omits it is precisely `keyring-no-namespaces` — §4.4's reason being that
    // "a line without it matches every namespace, so one key would hold all
    // three roles by omission". So the absent-options case must reach the
    // classifier rather than dying as `keyring-line-malformed`.
    //
    // OpenSSH disambiguates by trying to parse the field as a key and falling
    // back to options. This does the cheap equivalent: a key type is a token
    // from a known family, an option is anything else. The heuristic is wider
    // than §4.2's list on purpose — `ssh-rsa` must reach
    // `keyring-keytype-unknown` (R14) rather than be read as an option.
    let (options, rest) = match split_ws_once(rest) {
        Some((first, _)) if looks_like_keytype(first) => ("", rest),
        _ => split_options(rest).ok_or_else(|| malformed("unterminated option list"))?,
    };

    let (keytype, rest) = split_ws_once(rest).ok_or_else(|| malformed("no key type"))?;
    let (keyblob, _comment) = match split_ws_once(rest) {
        Some((blob, comment)) => (blob, Some(comment)),
        // MF §4.2: trailing comment-text is accepted and ignored.
        None => (rest, None),
    };
    if keyblob.is_empty() {
        return Err(malformed("no key blob"));
    }

    // MF §4.2 R12: one entry, one principal.
    if principals.contains(',') {
        findings.push(at(
            Lint::KeyringMultiPrincipal,
            format!("{principals} names more than one principal"),
        ));
    }
    let principal = principals
        .split(',')
        .next()
        .unwrap_or(principals)
        .to_string();
    if principal.is_empty() || principal.contains('#') {
        return Err(malformed("principal is empty or contains '#'"));
    }

    // Options. `namespaces=` is the only one accepted; the other three OpenSSH
    // defines each get their own status (MF §4.2, §4.4).
    let mut namespaces: Vec<String> = Vec::new();
    let mut saw_namespaces = false;
    for option in split_option_list(options) {
        let (name, value) = match option.split_once('=') {
            Some((n, v)) => (n, Some(v.trim_matches('"'))),
            None => (option, None),
        };
        match name {
            "namespaces" => {
                saw_namespaces = true;
                let value = value.unwrap_or("");
                if value.is_empty() {
                    findings.push(at(Lint::KeyringNamespaceEmpty, "namespaces=\"\"".into()));
                    continue;
                }
                for token in value.split(',') {
                    if !NAMESPACES.contains(&token) {
                        findings.push(at(
                            Lint::KeyringNamespaceUnknown,
                            format!("{token:?} is not one of the three"),
                        ));
                        continue;
                    }
                    if !namespaces.iter().any(|n| n == token) {
                        namespaces.push(token.to_string());
                    }
                }
            }
            "cert-authority" => findings.push(at(
                Lint::KeyringCertAuthority,
                "the cert-authority option".into(),
            )),
            "valid-after" | "valid-before" => findings.push(at(
                Lint::KeyringValidityOption,
                format!("the {name}= option; the chain is the clock (MF §4.6)"),
            )),
            other if !other.is_empty() => findings.push(at(
                Lint::KeyringOptionUnknown,
                format!("{other:?} is not namespaces="),
            )),
            _ => {}
        }
    }
    if !saw_namespaces {
        // "a line without it matches **every** namespace, so one key would
        // hold all three roles by omission."
        findings.push(at(
            Lint::KeyringNoNamespaces,
            "no namespaces= option".into(),
        ));
    }
    // MF §4.6, DM §7.2: ascending by bytes.
    namespaces.sort();

    if !KEYTYPES.contains(&keytype) {
        findings.push(at(
            Lint::KeyringKeytypeUnknown,
            format!("{keytype:?} is not in MF §4.2's list"),
        ));
    }

    // R17, both limbs: base64, **and** decoding to a key of the declared type.
    let fingerprint = match decode_key(keyblob, keytype) {
        Ok(fp) => fp,
        Err(why) => {
            findings.push(at(Lint::KeyringKeyNotBase64, why));
            String::new()
        }
    };

    Ok((
        Entry {
            line_no,
            principal,
            namespaces,
            keytype: keytype.to_string(),
            keyblob: keyblob.to_string(),
            fingerprint,
        },
        findings,
    ))
}

/// `ssh-keygen -lf` over `<keytype> <keyblob>`: SHA-256 of the decoded blob,
/// then unpadded base64 (MF §4.2). Also checks the blob's own embedded key type
/// against the declared one — an SSH public key blob begins with a
/// length-prefixed copy of its type, so the second limb of R17 costs one read.
fn decode_key(keyblob: &str, declared: &str) -> Result<String, String> {
    let raw = base64_decode(keyblob).ok_or_else(|| "not base64".to_string())?;
    if raw.len() < 4 {
        return Err("blob is too short to carry a key type".into());
    }
    let name_len = u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize;
    let embedded = raw
        .get(4..4 + name_len)
        .and_then(|b| core::str::from_utf8(b).ok())
        .ok_or_else(|| "blob does not carry a key type".to_string())?;

    // The RSA SHA-2 signature algorithms are spelled `rsa-sha2-256` in a
    // keyring line, while the *key* blob they name is an `ssh-rsa` key: the
    // difference is the signature algorithm, not the key. Everything else
    // names itself.
    let expected: &str = match declared {
        "rsa-sha2-256" | "rsa-sha2-512" => "ssh-rsa",
        other => other,
    };
    if embedded != expected {
        return Err(format!(
            "blob is a {embedded:?} key but the line declares {declared:?}"
        ));
    }

    let digest = spine_canon::digest::sha256_raw(&raw);
    Ok(format!("SHA256:{}", base64_encode_unpadded(&digest)))
}

fn split_ws_once(s: &str) -> Option<(&str, &str)> {
    let index = s.find([' ', '\t'])?;
    let rest = s[index..].trim_start_matches([' ', '\t']);
    Some((&s[..index], rest))
}

/// Split the option field off, honouring the quoted `namespaces="a,b"` value.
fn split_options(s: &str) -> Option<(&str, &str)> {
    let mut in_quotes = false;
    for (index, ch) in s.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ' ' | '\t' if !in_quotes => {
                let rest = s[index..].trim_start_matches([' ', '\t']);
                return Some((&s[..index], rest));
            }
            _ => {}
        }
    }
    if in_quotes { None } else { Some((s, "")) }
}

/// Comma-split an option list at the top level, ignoring commas inside quotes.
fn split_option_list(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut in_quotes = false;
    let mut start = 0;
    for (index, ch) in s.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                out.push(&s[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out.retain(|piece| !piece.is_empty());
    out
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut accumulator: u32 = 0;
    let mut bits = 0u32;
    for byte in s.bytes() {
        if byte == b'=' {
            break;
        }
        let value = TABLE.iter().position(|&c| c == byte)? as u32;
        accumulator = (accumulator << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((accumulator >> bits) as u8);
        }
    }
    Some(out)
}

fn base64_encode_unpadded(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        let count = chunk.len() + 1;
        for i in 0..count {
            let index = ((triple >> (18 - 6 * i)) & 0x3F) as usize;
            out.push(TABLE[index] as char);
        }
    }
    out
}

/// A parse heuristic, not a normative rule: does this token sit in the position
/// of a key type rather than an option list?
///
/// Deliberately wider than [`KEYTYPES`]. A line reading `alice ssh-rsa AAAA…`
/// has no options and an out-of-list key type, and MF §4.4 wants
/// `keyring-keytype-unknown` for it — which it can only get if the field split
/// recognises `ssh-rsa` as sitting where a key type sits.
fn looks_like_keytype(token: &str) -> bool {
    KEYTYPES.contains(&token)
        || ["ssh-", "ecdsa-", "sk-", "rsa-", "webauthn-"]
            .iter()
            .any(|prefix| token.starts_with(prefix))
}
