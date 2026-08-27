//! The release artifact list and the platform table (CI §5.5).
//!
//! PB §6.7 defines `dist_hash` as "the SHA-256 of the release's *artifact list*
//! — a file the release publishes naming every platform artifact and the wheel
//! with its own SHA-256" and fixes neither its location nor its bytes. CI §5.5
//! fixes both, and `manifest.md` §3.2 **adopts CI §5.5 as normative for the
//! manifest** rather than restating it. This module is that adoption.
//!
//! The layout is content-addressed on the list's own digest:
//!
//! ```text
//! <SPINE_DIST_BASE>/<H>/artifacts.txt      the list
//! <SPINE_DIST_BASE>/<H>/<artifact-name>    every artifact the list names
//! ```
//!
//! "Keying the directory on the list's own digest is what lets `ci.sh` fetch
//! the list before it knows the version, and it makes the pin sufficient: one
//! 64-hex string locates and authenticates everything."

use core::fmt;

/// CI §5.5's platform table, `uname -s` / `uname -m` to target token.
///
/// **v1 ships no Windows target** and says so rather than half-supporting one:
/// `.tar.gz` is the only container and a Git Bash job would need a `.zip` path,
/// an `.exe` suffix and a different `uname` match (CI §18 OPEN-4).
pub fn target_for(uname_s: &str, uname_m: &str) -> Option<&'static str> {
    match (uname_s, uname_m) {
        ("Linux", "x86_64" | "amd64") => Some("x86_64-unknown-linux-musl"),
        ("Linux", "aarch64" | "arm64") => Some("aarch64-unknown-linux-musl"),
        ("Darwin", "arm64") => Some("aarch64-apple-darwin"),
        ("Darwin", "x86_64") => Some("x86_64-apple-darwin"),
        _ => None,
    }
}

/// The target token of the platform this binary was built for.
///
/// PB §6.7: "each binary embeds the list's hash and verifies its own bytes
/// against the list's entry for its platform at start-up".
pub const fn host_target() -> Option<&'static str> {
    // `musl` rather than `gnu` is the shipped Linux artifact: CI §5.5's tokens
    // are what the list carries, and the release builds statically.
    match (cfg!(target_os = "linux"), cfg!(target_os = "macos")) {
        (true, _) => {
            if cfg!(target_arch = "x86_64") {
                Some("x86_64-unknown-linux-musl")
            } else if cfg!(target_arch = "aarch64") {
                Some("aarch64-unknown-linux-musl")
            } else {
                None
            }
        }
        (_, true) => {
            if cfg!(target_arch = "aarch64") {
                Some("aarch64-apple-darwin")
            } else if cfg!(target_arch = "x86_64") {
                Some("x86_64-apple-darwin")
            } else {
                None
            }
        }
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactEntry {
    /// 64 lowercase hex.
    pub sha256: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactListError {
    NotUtf8,
    /// A line that is not `<64 lowercase hex>` + two spaces + `<name>`.
    MalformedLine(usize),
    /// CI §5.5: "Lines sorted ascending by the bytes of the artifact name. Two
    /// builds of one release produce one list, byte for byte, or `dist_hash` is
    /// not a pin."
    Unsorted(usize),
    /// A CR, a BOM, a blank line, a comment, a header, or a missing final LF.
    Framing(&'static str),
    /// An artifact name outside `spine-<version>-<target>.tar.gz` or
    /// `spine-<version>-py3-none-any.whl`.
    BadName(String),
    /// "a release whose own list is ambiguous is not a pin"
    DuplicateTarget(String),
}

impl fmt::Display for ArtifactListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactListError::NotUtf8 => write!(f, "the artifact list is not UTF-8"),
            ArtifactListError::MalformedLine(n) => {
                write!(f, "line {n} is not sha256sum format (two spaces)")
            }
            ArtifactListError::Unsorted(n) => write!(f, "line {n} is out of order"),
            ArtifactListError::Framing(why) => write!(f, "framing: {why}"),
            ArtifactListError::BadName(name) => write!(f, "artifact name {name:?}"),
            ArtifactListError::DuplicateTarget(t) => {
                write!(f, "two artifacts for target {t}")
            }
        }
    }
}

impl core::error::Error for ArtifactListError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactList {
    pub entries: Vec<ArtifactEntry>,
}

impl ArtifactList {
    /// Parse CI §5.5's bytes, enforcing every rule that makes the list a pin.
    pub fn parse(bytes: &[u8]) -> Result<Self, ArtifactListError> {
        if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return Err(ArtifactListError::Framing("a BOM"));
        }
        if bytes.contains(&b'\r') {
            return Err(ArtifactListError::Framing("a CR"));
        }
        let text = core::str::from_utf8(bytes).map_err(|_| ArtifactListError::NotUtf8)?;
        // "every line terminated including the last"
        let Some(body) = text.strip_suffix('\n') else {
            return Err(ArtifactListError::Framing("no final LF"));
        };

        let mut entries: Vec<ArtifactEntry> = Vec::new();
        for (index, line) in body.split('\n').enumerate() {
            let line_no = index + 1;
            // "no blank lines, no comments, no header"
            if line.is_empty() {
                return Err(ArtifactListError::Framing("a blank line"));
            }
            // Exactly `<64 hex>` + two spaces + name. `sha256sum`'s own format.
            let Some((digest, name)) = line.split_once("  ") else {
                return Err(ArtifactListError::MalformedLine(line_no));
            };
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || name.is_empty()
                || name.starts_with(' ')
            {
                return Err(ArtifactListError::MalformedLine(line_no));
            }
            if let Some(previous) = entries.last()
                && previous.name.as_str() >= name
            {
                return Err(ArtifactListError::Unsorted(line_no));
            }
            entries.push(ArtifactEntry {
                sha256: digest.to_string(),
                name: name.to_string(),
            });
        }

        if entries.is_empty() {
            return Err(ArtifactListError::Framing("no entries"));
        }

        // Names, and "exactly one artifact per target".
        let mut seen_targets: Vec<String> = Vec::new();
        for entry in &entries {
            match classify(&entry.name) {
                Some(Artifact::Wheel { .. }) => {}
                Some(Artifact::Platform { target, .. }) => {
                    if seen_targets.iter().any(|t| t == target) {
                        return Err(ArtifactListError::DuplicateTarget(target.to_string()));
                    }
                    seen_targets.push(target.to_string());
                }
                None => return Err(ArtifactListError::BadName(entry.name.clone())),
            }
        }
        Ok(ArtifactList { entries })
    }

    /// `sha256:` + the SHA-256 of exactly these bytes — the value the manifest
    /// carries as `cli.dist_hash`.
    pub fn dist_hash(bytes: &[u8]) -> String {
        spine_canon::sha256_prefixed(bytes)
    }

    /// The entry for a target, or `None`. CI §5.5: `ci.sh` "refuses a list with
    /// none (the release does not build for this runner)".
    pub fn for_target(&self, target: &str) -> Option<&ArtifactEntry> {
        self.entries.iter().find(|e| {
            matches!(classify(&e.name), Some(Artifact::Platform { target: t, .. }) if t == target)
        })
    }

    /// CI §5.3 R4: "the version is derived from the hash-verified artifact list"
    /// rather than read from the manifest beside the hash — "a version string
    /// that could be read independently of the digest is a string that could
    /// disagree with it."
    pub fn version(&self) -> Option<&str> {
        self.entries.iter().find_map(|e| match classify(&e.name) {
            Some(Artifact::Platform { version, .. } | Artifact::Wheel { version }) => Some(version),
            None => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Artifact<'a> {
    Platform { version: &'a str, target: &'a str },
    Wheel { version: &'a str },
}

/// CI §5.5: `spine-<version>-<target>.tar.gz` for platform artifacts and
/// `spine-<version>-py3-none-any.whl` for the wheel. `<version>` is
/// `[0-9A-Za-z._+-]+`.
///
/// `<version>` admits `-`, so the split is anchored on the **known target
/// tokens** at the tail rather than on the last `-`: a version like `1.4.0-rc1`
/// would otherwise take a target token's leading segment with it.
fn classify(name: &str) -> Option<Artifact<'_>> {
    const TARGETS: [&str; 4] = [
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-musl",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
    ];
    let rest = name.strip_prefix("spine-")?;

    if let Some(version) = rest.strip_suffix("-py3-none-any.whl") {
        return version_ok(version).then_some(Artifact::Wheel { version });
    }
    let stem = rest.strip_suffix(".tar.gz")?;
    for target in TARGETS {
        if let Some(version) = stem.strip_suffix(target)
            && let Some(version) = version.strip_suffix('-')
            && version_ok(version)
        {
            return Some(Artifact::Platform { version, target });
        }
    }
    None
}

fn version_ok(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= 64
        && version != "none"
        && version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'+' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `manifest.md` §8.2 — the published list, 529 bytes, whose SHA-256 is the
    /// vector manifest's `cli.dist_hash`.
    const MF_8_2: &[u8] = include_bytes!("../tests/vectors/mf-8.2-artifacts.txt");

    #[test]
    fn mf_8_2_artifact_list_and_dist_hash() {
        assert_eq!(MF_8_2.len(), 529);
        assert_eq!(
            ArtifactList::dist_hash(MF_8_2),
            "sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db"
        );

        let list = ArtifactList::parse(MF_8_2).expect("§8.2 is a conforming list");
        assert_eq!(list.entries.len(), 5, "four targets and the wheel");
        assert_eq!(list.version(), Some("1.4.0"));

        // One artifact per target, and every target present.
        for target in [
            "x86_64-unknown-linux-musl",
            "aarch64-unknown-linux-musl",
            "aarch64-apple-darwin",
            "x86_64-apple-darwin",
        ] {
            assert!(list.for_target(target).is_some(), "{target} is missing");
        }
        assert_eq!(
            list.for_target("aarch64-apple-darwin").unwrap().sha256,
            "f0ed236cfc75bb491003477b77cbd09b76f80420f546b585e2a16ee85ad989ae"
        );
        assert!(list.for_target("x86_64-pc-windows-msvc").is_none());
    }

    /// CI §5.5's platform table, every row including the refusal.
    #[test]
    fn the_platform_table() {
        assert_eq!(
            target_for("Linux", "x86_64"),
            Some("x86_64-unknown-linux-musl")
        );
        assert_eq!(
            target_for("Linux", "amd64"),
            Some("x86_64-unknown-linux-musl"),
            "amd64 is the same row as x86_64"
        );
        assert_eq!(
            target_for("Linux", "aarch64"),
            Some("aarch64-unknown-linux-musl")
        );
        assert_eq!(
            target_for("Linux", "arm64"),
            Some("aarch64-unknown-linux-musl")
        );
        assert_eq!(target_for("Darwin", "arm64"), Some("aarch64-apple-darwin"));
        assert_eq!(target_for("Darwin", "x86_64"), Some("x86_64-apple-darwin"));

        // "anything else | refused, exit 2". v1 ships no Windows CI target.
        assert_eq!(target_for("MINGW64_NT-10.0", "x86_64"), None);
        assert_eq!(target_for("FreeBSD", "amd64"), None);
        assert_eq!(target_for("Darwin", "ppc"), None);
    }

    /// "Lines sorted ascending by the bytes of the artifact name. Two builds of
    /// one release produce one list, byte for byte, or `dist_hash` is not a
    /// pin." A permutation leaves the entry set identical and the digest
    /// different, which is exactly the failure a sort check exists to prevent.
    #[test]
    fn an_unsorted_list_is_refused_even_though_it_holds_the_same_entries() {
        let text = String::from_utf8(MF_8_2.to_vec()).unwrap();
        let mut lines: Vec<&str> = text.trim_end().split('\n').collect();
        lines.swap(0, 1);
        let permuted = lines.join("\n") + "\n";

        assert_eq!(permuted.len(), MF_8_2.len(), "same bytes, different order");
        assert_ne!(
            ArtifactList::dist_hash(permuted.as_bytes()),
            ArtifactList::dist_hash(MF_8_2),
            "which is why the order has to be checked and not assumed"
        );
        assert!(matches!(
            ArtifactList::parse(permuted.as_bytes()),
            Err(ArtifactListError::Unsorted(2))
        ));
    }

    /// "Exactly one artifact per target. `ci.sh` refuses a list with none …
    /// and refuses a list with two (a release whose own list is ambiguous is
    /// not a pin)."
    #[test]
    fn two_artifacts_for_one_target_is_a_refusal() {
        let doubled = concat!(
            "aa00000000000000000000000000000000000000000000000000000000000000  spine-1.4.0-aarch64-apple-darwin.tar.gz\n",
            "bb00000000000000000000000000000000000000000000000000000000000000  spine-1.5.0-aarch64-apple-darwin.tar.gz\n",
        );
        assert!(matches!(
            ArtifactList::parse(doubled.as_bytes()),
            Err(ArtifactListError::DuplicateTarget(_))
        ));
    }

    #[test]
    fn framing_is_sha256sum_format_and_nothing_else() {
        let good = "aa00000000000000000000000000000000000000000000000000000000000000  spine-1.4.0-aarch64-apple-darwin.tar.gz\n";
        assert!(ArtifactList::parse(good.as_bytes()).is_ok());

        // One space instead of two — the format `sha256sum -c` reads is two.
        let one_space = good.replacen("  ", " ", 1);
        assert!(matches!(
            ArtifactList::parse(one_space.as_bytes()),
            Err(ArtifactListError::MalformedLine(1))
        ));

        for (bytes, why) in [
            (good.trim_end().as_bytes().to_vec(), "no final LF"),
            (format!("\n{good}").into_bytes(), "a blank line"),
            (good.replace('\n', "\r\n").into_bytes(), "a CR"),
            (
                {
                    let mut v = vec![0xEF, 0xBB, 0xBF];
                    v.extend_from_slice(good.as_bytes());
                    v
                },
                "a BOM",
            ),
        ] {
            assert!(
                matches!(ArtifactList::parse(&bytes), Err(ArtifactListError::Framing(_))),
                "expected a framing refusal for {why}"
            );
        }
    }

    /// `<version>` admits `-`, so anchoring the split on the last `-` would
    /// mis-parse a pre-release. The tail is matched against the known targets.
    #[test]
    fn a_version_containing_a_dash_still_parses() {
        let list = "aa00000000000000000000000000000000000000000000000000000000000000  spine-1.4.0-rc.1-aarch64-apple-darwin.tar.gz\n";
        let parsed = ArtifactList::parse(list.as_bytes()).unwrap();
        assert_eq!(parsed.version(), Some("1.4.0-rc.1"));
    }

    #[test]
    fn names_outside_the_two_shapes_are_refused() {
        for name in [
            "spine-1.4.0-x86_64-pc-windows-msvc.tar.gz",
            "spine-1.4.0-aarch64-apple-darwin.zip",
            "spine-1.4.0.tar.gz",
            "notspine-1.4.0-aarch64-apple-darwin.tar.gz",
            "spine-none-aarch64-apple-darwin.tar.gz",
        ] {
            let line = format!(
                "aa00000000000000000000000000000000000000000000000000000000000000  {name}\n"
            );
            assert!(
                matches!(
                    ArtifactList::parse(line.as_bytes()),
                    Err(ArtifactListError::BadName(_))
                ),
                "{name:?} should be refused"
            );
        }
    }

    #[test]
    fn the_host_target_is_one_of_the_four_or_none() {
        // On this machine it is a real one; on an unshipped platform, None —
        // which is what makes G15 a membership test rather than a comparison.
        if let Some(target) = host_target() {
            assert!(
                [
                    "x86_64-unknown-linux-musl",
                    "aarch64-unknown-linux-musl",
                    "aarch64-apple-darwin",
                    "x86_64-apple-darwin",
                ]
                .contains(&target)
            );
        }
    }
}
