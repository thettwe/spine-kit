//! `release/release.json` — the release manifest, and what a build without one
//! may do (CI §3.4).
//!
//! It is a **build input**: read once when the binary is built, frozen into it,
//! and never consulted again. "It is not written into an adopting repository,
//! no `files[]` record names it, no owner class applies to it, it is on no
//! floor, and no gate reads it. Nothing at run time re-reads it from disk, so a
//! repository cannot supply one and a candidate cannot forge one."
//!
//! **An unknown member is a refusal, not opaque data** — the opposite of
//! `.spine/manifest.json`'s rule, and for the opposite reason: a repository
//! manifest must be judged by binaries older than itself, so forward
//! compatibility is worth an ignored key; this file is read only by the build
//! that freezes it, and "an ignored typo — `dist_bases`, `pins` — ships a
//! placeholder into every repository the release initialises."

use core::fmt;
use spine_canon::Value;

/// The three GitHub actions the release pins, and the repo each `commit` must
/// belong to.
///
/// The `repo` member exists so the build "checks the pin against the `uses:`
/// line that will carry it and refuses a manifest that pins the checkout commit
/// into the download step — a transposition no later check catches, because all
/// three are well-formed 40-hex strings."
pub const ACTIONS: [(&str, &str); 3] = [
    ("checkout", "actions/checkout"),
    ("download_artifact", "actions/download-artifact"),
    ("upload_artifact", "actions/upload-artifact"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseManifest {
    pub version: String,
    /// No trailing `/` — "`ci.sh` appends one (§5.3), and two spellings of one
    /// root would render two `ci.sh` blobs for one release."
    pub dist_base: String,
    /// Keyed by the names in [`ACTIONS`], each a 40-hex commit.
    pub actions: Vec<(String, String)>,
}

/// Why a build is a **development build**.
///
/// CI §3.4: "A build embedding a release manifest that satisfies the schema
/// above is a release build; anything else — no file, a file the schema
/// refuses, an unknown `release_manifest_version` — is a development build."
/// Every variant reports the one diagnostic `no-release-manifest`; the detail
/// is for the human reading stderr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoReleaseManifest {
    pub detail: String,
}

impl fmt::Display for NoReleaseManifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no-release-manifest: {}", self.detail)
    }
}

impl core::error::Error for NoReleaseManifest {}

fn refuse(detail: impl Into<String>) -> NoReleaseManifest {
    NoReleaseManifest {
        detail: detail.into(),
    }
}

impl ReleaseManifest {
    /// Parse and validate. CI §3.4 step 1: "Validate first … **before any plan
    /// is computed**. Failure is `no-release-manifest` and nothing is written."
    pub fn parse(bytes: &[u8]) -> Result<Self, NoReleaseManifest> {
        let value = spine_canon::parse(bytes)
            .map_err(|e| refuse(format!("release/release.json does not parse: {e}")))?;

        let Value::Obj(members) = &value else {
            return Err(refuse("release/release.json is not a JSON object"));
        };

        // Every member required, nothing else permitted.
        const KNOWN: [&str; 4] = [
            "actions",
            "dist_base",
            "release_manifest_version",
            "version",
        ];
        for (name, _) in members {
            if !KNOWN.contains(&name.as_str()) {
                return Err(refuse(format!("unknown member {name:?}")));
            }
        }
        for name in KNOWN {
            if value.get(name).is_none() {
                return Err(refuse(format!("missing member {name:?}")));
            }
        }

        // "A build that meets a value it does not know refuses rather than
        // guessing which members are present."
        match value
            .get("release_manifest_version")
            .and_then(Value::as_u64)
        {
            Some(1) => {}
            Some(other) => {
                return Err(refuse(format!(
                    "release_manifest_version {other} is unknown"
                )));
            }
            None => return Err(refuse("release_manifest_version is not an integer")),
        }

        let version = value
            .get("version")
            .and_then(Value::as_str)
            .ok_or_else(|| refuse("version is not a string"))?;
        // MF §3.2's grammar, which is also CI §5.5's `<version>` production.
        spine_manifest::grammar::check_cli_version(version)
            .map_err(|_| refuse(format!("version {version:?} is out of grammar")))?;

        let dist_base = value
            .get("dist_base")
            .and_then(Value::as_str)
            .ok_or_else(|| refuse("dist_base is not a string"))?;
        check_dist_base(dist_base)?;

        let Some(Value::Obj(actions)) = value.get("actions") else {
            return Err(refuse("actions is not an object"));
        };
        if actions.len() != ACTIONS.len() {
            return Err(refuse(format!(
                "actions has {} members, expected exactly {}",
                actions.len(),
                ACTIONS.len()
            )));
        }
        let mut resolved: Vec<(String, String)> = Vec::new();
        for (key, expected_repo) in ACTIONS {
            let Some(action) = value.get("actions").and_then(|a| a.get(key)) else {
                return Err(refuse(format!("actions.{key} is missing")));
            };
            let Value::Obj(action_members) = action else {
                return Err(refuse(format!("actions.{key} is not an object")));
            };
            if action_members.len() != 2 {
                return Err(refuse(format!(
                    "actions.{key} has {} members, expected exactly repo and commit",
                    action_members.len()
                )));
            }
            let repo = action
                .get("repo")
                .and_then(Value::as_str)
                .ok_or_else(|| refuse(format!("actions.{key}.repo is not a string")))?;
            if repo != expected_repo {
                // The transposition guard: all three commits are well-formed
                // 40-hex, so nothing downstream would notice a swap.
                return Err(refuse(format!(
                    "actions.{key}.repo is {repo:?}, expected {expected_repo:?}"
                )));
            }
            let commit = action
                .get("commit")
                .and_then(Value::as_str)
                .ok_or_else(|| refuse(format!("actions.{key}.commit is not a string")))?;
            if !is_full_commit(commit) {
                return Err(refuse(format!(
                    "actions.{key}.commit must be exactly 40 lowercase hex, never a tag \
                     and never an abbreviation"
                )));
            }
            resolved.push((key.to_string(), commit.to_string()));
        }

        Ok(ReleaseManifest {
            version: version.to_string(),
            dist_base: dist_base.to_string(),
            actions: resolved,
        })
    }

    pub fn commit(&self, action: &str) -> Option<&str> {
        self.actions
            .iter()
            .find(|(k, _)| k == action)
            .map(|(_, v)| v.as_str())
    }
}

/// "Scheme `https://`; no userinfo, no query, no fragment; **no trailing `/`**".
///
/// The `@` ban is load-bearing beyond URL hygiene: it is what guarantees a
/// substituted `dist_base` can never reintroduce the `@@` the §3.4 byte scan
/// then looks for.
fn check_dist_base(s: &str) -> Result<(), NoReleaseManifest> {
    let Some(rest) = s.strip_prefix("https://") else {
        return Err(refuse("dist_base must begin https://"));
    };
    if rest.is_empty() {
        return Err(refuse("dist_base has no host"));
    }
    if s.ends_with('/') {
        return Err(refuse(
            "dist_base must not end with '/' — ci.sh appends one",
        ));
    }
    for (byte, why) in [('@', "userinfo"), ('?', "a query"), ('#', "a fragment")] {
        if rest.contains(byte) {
            return Err(refuse(format!("dist_base must not carry {why}")));
        }
    }
    Ok(())
}

fn is_full_commit(s: &str) -> bool {
    s.len() == 40
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A conforming release manifest with **test values**. The host and the
    /// three commits are the owner's (CI §18 OPEN-1, OPEN-7) and are not in the
    /// corpus, so these are fixtures and are marked as such: they exist to
    /// exercise the schema, never to be published.
    pub(crate) const FIXTURE: &str = r#"{
      "release_manifest_version": 1,
      "version": "1.4.0",
      "dist_base": "https://dist.example.invalid/spine",
      "actions": {
        "checkout":          { "repo": "actions/checkout",          "commit": "11bd71901bbe5b1630ceea73d27597364c9af683" },
        "upload_artifact":   { "repo": "actions/upload-artifact",   "commit": "ea165f8d65b6e75b540449e92b4886f43607fa02" },
        "download_artifact": { "repo": "actions/download-artifact", "commit": "fa0a91b85d4f404e444e00e005971372dc801d16" }
      }
    }"#;

    #[test]
    fn a_conforming_release_manifest_parses() {
        let release = ReleaseManifest::parse(FIXTURE.as_bytes()).unwrap();
        assert_eq!(release.version, "1.4.0");
        assert_eq!(release.dist_base, "https://dist.example.invalid/spine");
        assert_eq!(
            release.commit("checkout"),
            Some("11bd71901bbe5b1630ceea73d27597364c9af683")
        );
        assert_eq!(release.actions.len(), 3);
    }

    /// "An unknown member is a refusal, not opaque data … an ignored typo —
    /// `dist_bases`, `pins` — ships a placeholder into every repository the
    /// release initialises."
    #[test]
    fn an_unknown_member_is_a_refusal() {
        let typo = FIXTURE.replace("\"dist_base\"", "\"dist_bases\"");
        let err = ReleaseManifest::parse(typo.as_bytes()).unwrap_err();
        assert!(err.detail.contains("dist_bases"), "{err}");
        // And the diagnostic is always the one token.
        assert!(err.to_string().starts_with("no-release-manifest: "));
    }

    #[test]
    fn an_unknown_schema_version_refuses_rather_than_guessing() {
        let future = FIXTURE.replace(
            "\"release_manifest_version\": 1",
            "\"release_manifest_version\": 2",
        );
        assert!(
            ReleaseManifest::parse(future.as_bytes())
                .unwrap_err()
                .detail
                .contains("unknown")
        );
    }

    /// The transposition guard: all three commits are well-formed 40-hex, so a
    /// swap is invisible to every later check.
    #[test]
    fn a_pin_under_the_wrong_repo_is_caught() {
        let transposed = FIXTURE.replace(
            r#""repo": "actions/download-artifact""#,
            r#""repo": "actions/checkout""#,
        );
        let err = ReleaseManifest::parse(transposed.as_bytes()).unwrap_err();
        assert!(err.detail.contains("download_artifact.repo"), "{err}");
    }

    /// "Exactly 40 lowercase hex digits: a full commit id, **never a tag and
    /// never an abbreviation**."
    #[test]
    fn a_tag_or_an_abbreviation_is_not_a_pin() {
        for bad in ["v4", "11bd719", "11BD71901BBE5B1630CEEA73D27597364C9AF683"] {
            let manifest = FIXTURE.replace("11bd71901bbe5b1630ceea73d27597364c9af683", bad);
            assert!(
                ReleaseManifest::parse(manifest.as_bytes())
                    .unwrap_err()
                    .detail
                    .contains("40 lowercase hex"),
                "{bad:?} should be refused"
            );
        }
    }

    #[test]
    fn dist_base_is_https_with_no_userinfo_query_fragment_or_trailing_slash() {
        for (base, why) in [
            ("http://dist.example.invalid", "not https"),
            ("https://dist.example.invalid/", "trailing slash"),
            ("https://user@dist.example.invalid", "userinfo"),
            ("https://dist.example.invalid?x=1", "query"),
            ("https://dist.example.invalid#f", "fragment"),
            ("https://", "no host"),
        ] {
            let manifest = FIXTURE.replace("https://dist.example.invalid/spine", base);
            assert!(
                ReleaseManifest::parse(manifest.as_bytes()).is_err(),
                "{base:?} should be refused ({why})"
            );
        }
    }

    /// The `@` ban is what makes the §3.4 byte scan sound: a substituted
    /// `dist_base` can never reintroduce an `@@`.
    #[test]
    fn a_valid_dist_base_can_never_contain_the_scanned_token() {
        let release = ReleaseManifest::parse(FIXTURE.as_bytes()).unwrap();
        assert!(!release.dist_base.contains("@@"));
        assert!(!release.dist_base.contains('@'));
    }

    #[test]
    fn version_takes_mf_3_2s_grammar_including_the_none_exclusion() {
        for bad in ["none", "1.4 0", ""] {
            let manifest =
                FIXTURE.replace("\"version\": \"1.4.0\"", &format!("\"version\": \"{bad}\""));
            assert!(
                ReleaseManifest::parse(manifest.as_bytes()).is_err(),
                "{bad:?} should be refused"
            );
        }
    }

    #[test]
    fn missing_members_and_wrong_shapes_are_refused() {
        assert!(ReleaseManifest::parse(b"{}").is_err());
        assert!(ReleaseManifest::parse(b"[]").is_err());
        assert!(ReleaseManifest::parse(b"not json").is_err());
        // An `actions` object with an extra member.
        let extra = FIXTURE.replace(
            r#""commit": "11bd71901bbe5b1630ceea73d27597364c9af683" }"#,
            r#""commit": "11bd71901bbe5b1630ceea73d27597364c9af683", "ref": "v4" }"#,
        );
        assert!(ReleaseManifest::parse(extra.as_bytes()).is_err());
    }
}
