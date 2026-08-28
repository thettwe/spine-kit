//! `Spine-Upgrade`, composed and parsed (MF §6.4).
//!
//! Every lifecycle path this crate implements ends in one of these lines, and
//! `init` — not `check` — is what writes it on the two forms PB §6.7 hands to
//! it: the rollback whose target release cannot be installed, and the uninstall
//! and re-init, "because the base has no pin and no workflow".
//!
//! PB §11 prints the grammar:
//!
//! > `from=<A> to=<B> manifest=<blob oid> forced=<paths> [from-manifest=<sha>]
//! > [since=<sha>] signer=<p>`
//!
//! and MF §6.4 fixes what PB left open: "Fields are space-separated
//! `key=value`, order as PB §11 prints it, each key exactly once."
//!
//! **`forced=`'s encoding is the trap.** MF §6.4, verbatim: "`forced=`'s
//! grammar is fixed here and was fixed nowhere. PB §11 writes `forced=<paths>`
//! — a list value inside a single-space-separated payload with no separator,
//! quoting or escaping … The resolution reuses machinery rather than adding
//! any: **`tok` from GR §6.2**, which already escapes exactly the three bytes
//! (`,`, space, `"`) that break this line". The manifest spells the same path
//! with `esc` (MF §2.3), so a record's `path` is **not** a `forced=` token and
//! copying it across would produce a line whose `-Sig` verifies over the wrong
//! bytes. This module owns that conversion and nothing else may do it.

use spine_canon::{esc, tok, unesc};
use spine_manifest::grammar;

/// A `from=`/`to=` endpoint.
///
/// MF §3.2 excludes `none` from `cli.version`'s grammar exactly so this
/// sentinel is unambiguous — an uninstall's `to=none` can never collide with a
/// release that happened to be called `none`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    Version(String),
    /// `to=none` — the uninstall (MF §6.8). `from=none` — the re-init (§6.9).
    None,
}

impl Endpoint {
    pub fn parse(s: &str) -> Result<Self, UpgradeError> {
        if s == "none" {
            return Ok(Endpoint::None);
        }
        grammar::check_cli_version(s)
            .map_err(|_| UpgradeError::VersionOutOfGrammar(s.to_string()))?;
        Ok(Endpoint::Version(s.to_string()))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Endpoint::None)
    }
}

impl core::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Endpoint::Version(v) => f.write_str(v),
            Endpoint::None => f.write_str("none"),
        }
    }
}

/// Why a `Spine-Upgrade` line is not one.
///
/// Typed and total: the line is signed and folded into `envelope=`, so a reader
/// that guessed at a malformed field would verify a digest over bytes nobody
/// wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpgradeError {
    /// The line does not begin `Spine-Upgrade: `.
    NotAnUpgradeLine,
    /// A field that is not `key=value`.
    FieldMalformed(String),
    /// "each key exactly once" (MF §6.4).
    DuplicateKey(&'static str),
    MissingKey(&'static str),
    UnknownKey(String),
    /// "order as PB §11 prints it" (MF §6.4).
    FieldOutOfOrder {
        found: String,
        expected: &'static str,
    },
    VersionOutOfGrammar(String),
    /// "A leading, trailing or doubled comma is malformed" (MF §6.4).
    ForcedListMalformed,
    /// A `forced=` token that is not `tok` output.
    ForcedTokenMalformed(String),
    /// MF §6.8: `to=none` requires `manifest=none`, and the pairing is checked
    /// in both directions — the gate's `upgrade-manifest-mismatch`.
    ManifestMismatch,
    /// MF §6.9: `since=` is "mandatory on a re-init (`from=none`), absent
    /// otherwise" — the gate's `reinit-since-missing`.
    SinceMisplaced,
    /// A `manifest=`, `from-manifest=` or `since=` value that is not hex.
    OidMalformed(String),
}

impl core::fmt::Display for UpgradeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UpgradeError::NotAnUpgradeLine => f.write_str("not a Spine-Upgrade line"),
            UpgradeError::FieldMalformed(s) => write!(f, "field {s:?} is not key=value"),
            UpgradeError::DuplicateKey(k) => write!(f, "{k}= appears twice"),
            UpgradeError::MissingKey(k) => write!(f, "{k}= is missing"),
            UpgradeError::UnknownKey(k) => write!(f, "{k}= is not a Spine-Upgrade field"),
            UpgradeError::FieldOutOfOrder { found, expected } => {
                write!(f, "{found}= where {expected}= was expected")
            }
            UpgradeError::VersionOutOfGrammar(v) => write!(f, "cli-version-out-of-grammar: {v:?}"),
            UpgradeError::ForcedListMalformed => {
                f.write_str("forced= has a leading, trailing or doubled comma")
            }
            UpgradeError::ForcedTokenMalformed(t) => write!(f, "forced= token {t:?} is not tok"),
            UpgradeError::ManifestMismatch => {
                f.write_str("upgrade-manifest-mismatch: to=none and manifest= must agree")
            }
            UpgradeError::SinceMisplaced => {
                f.write_str("since= is mandatory on from=none and absent otherwise")
            }
            UpgradeError::OidMalformed(s) => write!(f, "not a lowercase hex object id: {s:?}"),
        }
    }
}

impl core::error::Error for UpgradeError {}

/// The parsed line.
///
/// `forced` holds **`esc`-encoded** paths — the manifest's own spelling — so a
/// caller moves record paths in and out without ever meeting a second
/// encoding. `tok` exists only across the wire, in [`UpgradeLine::render`] and
/// [`UpgradeLine::parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeLine {
    pub from: Endpoint,
    pub to: Endpoint,
    /// The git blob id of `.spine/manifest.json` in `T`; `None` renders `none`,
    /// which MF §6.8 requires of an uninstall.
    pub manifest: Option<String>,
    /// `esc`-encoded paths this landing overwrote under `--force`.
    pub forced: Vec<String>,
    /// "a commit sha; **mandatory on a rollback**, absent otherwise" (MF §6.4).
    /// Its presence *is* the restoration rule's trigger (MF §6.7).
    pub from_manifest: Option<String>,
    /// "a commit sha; **mandatory on a re-init** (`from=none`)" (MF §6.4).
    pub since: Option<String>,
    pub signer: String,
}

/// The trailer name, with its separator — the bytes a `-Sig` covers alongside
/// the payload.
pub const PREFIX: &str = "Spine-Upgrade: ";

impl UpgradeLine {
    /// The whole trailer line, exactly as MF §8.6 prints one.
    ///
    /// No trailing LF: R3's rule inverts by artifact, and a trailer line is
    /// joined into `envelope=` by LF rather than terminated by one.
    pub fn render(&self) -> Result<String, UpgradeError> {
        self.check_pairings()?;
        let mut out = String::with_capacity(PREFIX.len() + 160);
        out.push_str(PREFIX);
        out.push_str("from=");
        out.push_str(&self.from.to_string());
        out.push_str(" to=");
        out.push_str(&self.to.to_string());
        out.push_str(" manifest=");
        out.push_str(self.manifest.as_deref().unwrap_or("none"));
        out.push_str(" forced=");
        out.push_str(&encode_forced(&self.forced)?);
        if let Some(sha) = &self.from_manifest {
            out.push_str(" from-manifest=");
            out.push_str(sha);
        }
        if let Some(sha) = &self.since {
            out.push_str(" since=");
            out.push_str(sha);
        }
        out.push_str(" signer=");
        out.push_str(&self.signer);
        Ok(out)
    }

    /// MF §6.8's `upgrade-manifest-mismatch`, and §6.9's `reinit-since-missing`,
    /// enforced on the **write** side so `init` cannot emit a line its own gate
    /// would refuse.
    fn check_pairings(&self) -> Result<(), UpgradeError> {
        if self.to.is_none() != self.manifest.is_none() {
            return Err(UpgradeError::ManifestMismatch);
        }
        if self.from.is_none() != self.since.is_some() {
            return Err(UpgradeError::SinceMisplaced);
        }
        Ok(())
    }

    pub fn parse(line: &str) -> Result<Self, UpgradeError> {
        let payload = line
            .strip_prefix(PREFIX)
            .ok_or(UpgradeError::NotAnUpgradeLine)?;

        // "order as PB §11 prints it, each key exactly once" — so the parse is a
        // walk down the fixed order rather than a map lookup. A map would accept
        // a permuted line, and a permuted line is a different `envelope=`.
        let mut seen: Vec<&str> = Vec::new();
        let mut got: Vec<(String, String)> = Vec::new();
        for field in payload.split(' ') {
            let (key, value) = field
                .split_once('=')
                .ok_or_else(|| UpgradeError::FieldMalformed(field.to_string()))?;
            let known = match key {
                "from" => "from",
                "to" => "to",
                "manifest" => "manifest",
                "forced" => "forced",
                "from-manifest" => "from-manifest",
                "since" => "since",
                "signer" => "signer",
                other => return Err(UpgradeError::UnknownKey(other.to_string())),
            };
            if seen.contains(&known) {
                return Err(UpgradeError::DuplicateKey(known));
            }
            seen.push(known);
            got.push((key.to_string(), value.to_string()));
        }

        // The fixed order, with the two optional fields in their printed slots.
        const ORDER: [&str; 7] = [
            "from",
            "to",
            "manifest",
            "forced",
            "from-manifest",
            "since",
            "signer",
        ];
        const OPTIONAL: [&str; 2] = ["from-manifest", "since"];
        let mut expected = ORDER.iter();
        for (key, _) in &got {
            loop {
                let want = expected
                    .next()
                    .ok_or_else(|| UpgradeError::UnknownKey(key.clone()))?;
                if want == key {
                    break;
                }
                if !OPTIONAL.contains(want) {
                    return Err(UpgradeError::FieldOutOfOrder {
                        found: key.clone(),
                        expected: want,
                    });
                }
            }
        }

        let value =
            |name: &'static str| got.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone());
        let required = |name: &'static str| value(name).ok_or(UpgradeError::MissingKey(name));

        let manifest_field = required("manifest")?;
        let line = UpgradeLine {
            from: Endpoint::parse(&required("from")?)?,
            to: Endpoint::parse(&required("to")?)?,
            manifest: match manifest_field.as_str() {
                "none" => None,
                oid => Some(check_oid(oid)?),
            },
            forced: decode_forced(&required("forced")?)?,
            from_manifest: value("from-manifest").map(|s| check_oid(&s)).transpose()?,
            since: value("since").map(|s| check_oid(&s)).transpose()?,
            signer: required("signer")?,
        };
        line.check_pairings()?;
        Ok(line)
    }
}

fn check_oid(s: &str) -> Result<String, UpgradeError> {
    // Never abbreviated, always lowercase hex (MF §3.5). The length is left to
    // the caller's `object_format`, because a `Spine-Upgrade` line carries commit
    // shas as well as blob ids and only the repository knows which width it uses.
    if s.is_empty()
        || !s
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(UpgradeError::OidMalformed(s.to_string()));
    }
    Ok(s.to_string())
}

/// `esc`-encoded paths in, one `forced=` value out.
///
/// **The empty list is the empty value.** MF §6.4: "`none` would be
/// indistinguishable from `tok("none")`, which is a legal path."
///
/// DERIVED: the corpus fixes set equality for `forced=` (MF §6.4's
/// `derived_forced`) and no order. The order taken here is R1's wire
/// comparator — **ascending by unsigned byte value over the whole `tok`
/// token** — because `forced=` is a `tok`-encoded comma list on a signed line,
/// which is what `wires=` is, and one comparator for the two is one fewer place
/// to get `G11` before `G2` wrong. Sorting the `esc` spelling instead would be
/// R2's mistake in the same line.
pub fn encode_forced(esc_paths: &[String]) -> Result<String, UpgradeError> {
    let mut tokens: Vec<String> = Vec::with_capacity(esc_paths.len());
    for path in esc_paths {
        let bytes = unesc(path).map_err(|_| UpgradeError::ForcedTokenMalformed(path.clone()))?;
        tokens.push(tok(&bytes));
    }
    tokens.sort_unstable_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    tokens.dedup();
    Ok(tokens.join(","))
}

/// The inverse: one `forced=` value in, `esc`-encoded paths out.
pub fn decode_forced(value: &str) -> Result<Vec<String>, UpgradeError> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    // "A leading, trailing or doubled comma is malformed" (MF §6.4).
    if value.starts_with(',') || value.ends_with(',') || value.contains(",,") {
        return Err(UpgradeError::ForcedListMalformed);
    }
    value
        .split(',')
        .map(|token| {
            let bytes =
                unesc(token).map_err(|_| UpgradeError::ForcedTokenMalformed(token.to_string()))?;
            // `tok` and `esc` differ only in which bytes they escape, so a token
            // that decodes must re-encode to itself under `tok` or it was never
            // `tok` output — a bare space or comma inside a token, say.
            if tok(&bytes) != token {
                return Err(UpgradeError::ForcedTokenMalformed(token.to_string()));
            }
            Ok(esc(&bytes))
        })
        .collect()
}

/// The `since=` a re-init must name (MF §6.9).
///
/// > `since=<sha>` is present and names a first-parent ancestor of `B` that is
/// > a **valid landing** carrying `Spine-Upgrade: to=none`
///
/// This finds the **candidate**: the newest first-parent commit whose message
/// carries a `Spine-Upgrade` line with `to=none`. It cannot decide "valid
/// landing" — that is G9's predicate, and it needs an envelope parser, a seal
/// and a keyring. PB §6.7 is emphatic about the cost of getting the pairing
/// wrong: "`since=` must name a landing carrying `to=none`, or the re-init is
/// refused and nothing is exempt" — and MF §6.9 adds that a re-init failing
/// either check "does not merely fail G16: the range stays un-exempt and every
/// commit in it indexes `unattested`".
///
/// `log` is `(sha, message)` newest first — [`crate::git::Repo::first_parent_log`].
pub fn find_uninstall_landing(log: &[(String, String)]) -> Option<String> {
    log.iter()
        .find(|(_, message)| {
            message.lines().any(|line| {
                UpgradeLine::parse(line.trim_end()).is_ok_and(|parsed| parsed.to.is_none())
            })
        })
        .map(|(sha, _)| sha.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MF §8.6's published line, over the vector's own values.
    #[test]
    fn mf_8_6_the_published_rollback_line_renders() {
        let line = UpgradeLine {
            from: Endpoint::parse("1.4.0").unwrap(),
            to: Endpoint::parse("1.3.0").unwrap(),
            manifest: Some("74806e98701b50e958074dbaad0d7509d84751a3".into()),
            forced: Vec::new(),
            from_manifest: Some("0123456789abcdef0123456789abcdef01234567".into()),
            since: None,
            signer: "alice@example.com".into(),
        };
        assert_eq!(
            line.render().unwrap(),
            "Spine-Upgrade: from=1.4.0 to=1.3.0 \
             manifest=74806e98701b50e958074dbaad0d7509d84751a3 forced= \
             from-manifest=0123456789abcdef0123456789abcdef01234567 signer=alice@example.com"
        );
        assert_eq!(UpgradeLine::parse(&line.render().unwrap()).unwrap(), line);
    }

    /// MF §6.4: "The empty list is the **empty value** … and not a sentinel:
    /// `none` would be indistinguishable from `tok("none")`, which is a legal
    /// path."
    #[test]
    fn an_empty_forced_list_is_the_empty_value_and_never_none() {
        assert_eq!(encode_forced(&[]).unwrap(), "");
        assert_eq!(decode_forced("").unwrap(), Vec::<String>::new());
        // `none` decodes as the one-element list naming the path `none`, which
        // is exactly why the empty value had to be empty.
        assert_eq!(decode_forced("none").unwrap(), vec!["none".to_string()]);
    }

    /// R2, in one line: the manifest spells a path with `esc`, the wire with
    /// `tok`, and the two differ on the bytes each escapes. A record's `path`
    /// copied straight into `forced=` is a signature over the wrong bytes.
    #[test]
    fn a_forced_token_is_tok_of_the_path_and_never_esc_of_it() {
        // A path with a space and a comma: `tok` escapes both, `esc` neither.
        let raw = b"a dir/with,comma.yml";
        let esc_path = esc(raw);
        assert_eq!(
            esc_path, "a dir/with,comma.yml",
            "esc leaves both bytes bare"
        );
        let encoded = encode_forced(std::slice::from_ref(&esc_path)).unwrap();
        assert_eq!(encoded, "a\\x20dir/with\\x2ccomma.yml");
        assert_ne!(encoded, esc_path, "esc in a forced= slot is the R2 defect");
        assert_eq!(decode_forced(&encoded).unwrap(), vec![esc_path]);
    }

    /// The comma is `tok`-escaped, so it can never split a token — which is the
    /// whole reason MF §6.4 chose `tok` over inventing a separator.
    #[test]
    fn a_comma_in_a_path_does_not_split_the_list() {
        let one = esc(b"a,b");
        let two = esc(b"c");
        let encoded = encode_forced(&[one.clone(), two.clone()]).unwrap();
        assert_eq!(encoded, "a\\x2cb,c");
        assert_eq!(decode_forced(&encoded).unwrap(), vec![one, two]);
    }

    /// R1's comparator: ascending by unsigned byte value over the whole token,
    /// so `G11` precedes `G2` — and `.spine/z` precedes `AGENTS.md` because `.`
    /// is 0x2E and `A` is 0x41.
    #[test]
    fn forced_sorts_by_unsigned_byte_value_over_the_whole_token() {
        let encoded = encode_forced(&[
            "AGENTS.md".to_string(),
            ".spine/z".to_string(),
            "G2".to_string(),
            "G11".to_string(),
        ])
        .unwrap();
        assert_eq!(encoded, ".spine/z,AGENTS.md,G11,G2");
    }

    #[test]
    fn a_leading_trailing_or_doubled_comma_is_malformed() {
        for value in [",a", "a,", "a,,b"] {
            assert_eq!(
                decode_forced(value),
                Err(UpgradeError::ForcedListMalformed),
                "{value:?}"
            );
        }
    }

    /// MF §6.8: an uninstall's `manifest=` is `none`, and the pairing holds in
    /// both directions so neither half can be forgotten.
    #[test]
    fn to_none_and_manifest_none_stand_or_fall_together() {
        let mut line = UpgradeLine {
            from: Endpoint::parse("1.4.0").unwrap(),
            to: Endpoint::None,
            manifest: None,
            forced: Vec::new(),
            from_manifest: None,
            since: None,
            signer: "alice@example.com".into(),
        };
        assert_eq!(
            line.render().unwrap(),
            "Spine-Upgrade: from=1.4.0 to=none manifest=none forced= signer=alice@example.com"
        );

        line.manifest = Some("cb4cd49034bbe25f76573c40d6711b2c33f9136f".into());
        assert_eq!(line.render(), Err(UpgradeError::ManifestMismatch));

        line.to = Endpoint::parse("1.3.0").unwrap();
        line.manifest = None;
        assert_eq!(line.render(), Err(UpgradeError::ManifestMismatch));
    }

    /// MF §6.9: `since=` is mandatory on a re-init and absent otherwise.
    #[test]
    fn since_is_mandatory_on_from_none_and_forbidden_elsewhere() {
        let reinit = UpgradeLine {
            from: Endpoint::None,
            to: Endpoint::parse("1.4.0").unwrap(),
            manifest: Some("cb4cd49034bbe25f76573c40d6711b2c33f9136f".into()),
            forced: Vec::new(),
            from_manifest: None,
            since: Some("0123456789abcdef0123456789abcdef01234567".into()),
            signer: "alice@example.com".into(),
        };
        let rendered = reinit.render().unwrap();
        assert!(rendered.contains(" since=0123456789abcdef0123456789abcdef01234567 signer="));
        assert_eq!(UpgradeLine::parse(&rendered).unwrap(), reinit);

        let mut missing = reinit.clone();
        missing.since = None;
        assert_eq!(missing.render(), Err(UpgradeError::SinceMisplaced));

        let mut stray = reinit.clone();
        stray.from = Endpoint::parse("1.3.0").unwrap();
        assert_eq!(stray.render(), Err(UpgradeError::SinceMisplaced));
    }

    /// "each key exactly once", and "order as PB §11 prints it". A permuted
    /// line is a different `envelope=`, so it is refused rather than reordered.
    #[test]
    fn a_permuted_or_repeated_field_is_refused() {
        assert_eq!(
            UpgradeLine::parse(
                "Spine-Upgrade: to=1.3.0 from=1.4.0 manifest=none forced= signer=a@b"
            ),
            Err(UpgradeError::FieldOutOfOrder {
                found: "to".into(),
                expected: "from",
            })
        );
        assert_eq!(
            UpgradeLine::parse(
                "Spine-Upgrade: from=1.4.0 from=1.4.0 to=1.3.0 manifest=none forced= signer=a@b"
            ),
            Err(UpgradeError::DuplicateKey("from"))
        );
        assert_eq!(
            UpgradeLine::parse("from=1.4.0 to=1.3.0 manifest=none forced= signer=a@b"),
            Err(UpgradeError::NotAnUpgradeLine)
        );
    }

    /// MF §6.9: the `since=` of a re-init names the uninstall landing, found by
    /// its `to=none` line and not by its subject — MF §6.8 is explicit that "an
    /// uninstall can therefore land under the subject `chore: update deps` with
    /// every signature intact", so "what a reader must not do is treat the
    /// subject as evidence of what a lifecycle landing did".
    #[test]
    fn the_uninstall_landing_is_found_by_its_line_and_never_by_its_subject() {
        let uninstall = (
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            "chore: update deps\n\nSpine-Event: land\n\
             Spine-Upgrade: from=1.4.0 to=none manifest=none forced= signer=alice@example.com\n"
                .to_string(),
        );
        let ordinary = (
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
            "quick: uninstall spine\n\nSpine-Event: land\n".to_string(),
        );
        let upgrade = (
            "cccccccccccccccccccccccccccccccccccccccc".to_string(),
            "quick: upgrade\n\n\
             Spine-Upgrade: from=1.3.0 to=1.4.0 manifest=\
             cb4cd49034bbe25f76573c40d6711b2c33f9136f forced= signer=alice@example.com\n"
                .to_string(),
        );

        // Newest first: the misleading subject is not the uninstall, and the
        // upgrade line that is not `to=none` is not either.
        assert_eq!(
            find_uninstall_landing(&[ordinary.clone(), upgrade.clone(), uninstall.clone()]),
            Some(uninstall.0.clone())
        );
        assert_eq!(find_uninstall_landing(&[ordinary, upgrade]), None);
    }

    /// MF §3.2 excludes `none` from `cli.version` so the sentinel is
    /// unambiguous; a version that is out of grammar is refused rather than
    /// carried.
    #[test]
    fn a_version_out_of_grammar_is_refused() {
        assert_eq!(
            Endpoint::parse("1.4.0 "),
            Err(UpgradeError::VersionOutOfGrammar("1.4.0 ".into()))
        );
        assert_eq!(Endpoint::parse("none"), Ok(Endpoint::None));
    }
}
