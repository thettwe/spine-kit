//! The scalar grammars `manifest.md` §3 fixes, each with the status token its
//! violation raises.
//!
//! These are separate from the schema because several are shared — a repository
//! path is validated identically in `paths` values, in `files[].path`, and in
//! the region form — and because a grammar with its refusal beside it is the
//! unit a test can pin.

use crate::status::{Refusal, Status};
use spine_canon::ObjectFormat;

/// MF §3.1: `^[A-Za-z0-9._-]+$`, 1..=64 bytes. Identity-encoded, **not** `esc`
/// — the grammar admits no byte `esc` would touch, so the two agree and the
/// distinction only matters to someone writing an encoder.
pub fn check_repo(s: &str) -> Result<(), Refusal> {
    let ok = !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(Refusal::new(Status::RepoOutOfGrammar, "repo"))
    }
}

/// MF §3.2: CI §5.5's `[0-9A-Za-z._+-]+` bounded to 1..=64 bytes, and **never**
/// the four bytes `none`.
///
/// `none` is excluded because `Spine-Upgrade` uses it as the sentinel for "no
/// manifest" in `from=`, `to=` and `manifest=` (MF §3.2, §6.4). A version that
/// could spell the sentinel would make an uninstall indistinguishable from an
/// upgrade to a release someone named `none`.
pub fn check_cli_version(s: &str) -> Result<(), Refusal> {
    let ok = !s.is_empty()
        && s.len() <= 64
        && s != "none"
        && s.bytes().all(|b| {
            b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'+' || b == b'-'
        });
    if ok {
        Ok(())
    } else {
        Err(Refusal::new(Status::CliVersionOutOfGrammar, "cli.version"))
    }
}

/// MF §3.2: `"sha256:"` followed by exactly 64 lowercase hex digits.
pub fn check_dist_hash(s: &str) -> Result<(), Refusal> {
    if spine_canon::digest::parse_sha256_prefixed(s).is_some() {
        Ok(())
    } else {
        Err(Refusal::new(Status::DistHashMalformed, "cli.dist_hash"))
    }
}

/// MF §3.5: a git blob id, lowercase hex, at the full length `object_format`
/// implies. **Never abbreviated** — an abbreviated id compares unequal to
/// `git ls-tree`'s output and would fail G16 for a reason nobody could see.
pub fn check_blob(s: &str, format: ObjectFormat, where_: &str) -> Result<(), Refusal> {
    let ok = s.len() == format.hex_len()
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if ok {
        Ok(())
    } else {
        Err(Refusal::new(Status::BlobMalformed, where_))
    }
}

/// MF §3.4's path rules, verbatim: 1..=4096 bytes, no leading `/`, no `//`, no
/// `.` or `..` segment, no trailing `/`, no `0x00`.
///
/// The input is the **`esc`-encoded** form, which is what the manifest stores;
/// the rules are applied to the decoded bytes, because `esc` can spell a `/`
/// only as itself and a `0x00` only as `\x00`, so checking the encoded form
/// would miss the second.
pub fn check_repo_path(esc_encoded: &str, where_: &str) -> Result<(), Refusal> {
    let bad = |w: &str| Refusal::new(Status::PathsValueMalformed, w.to_string());

    let bytes = spine_canon::unesc(esc_encoded).map_err(|_| bad(where_))?;
    if bytes.is_empty() || bytes.len() > 4096 {
        return Err(bad(where_));
    }
    if bytes.contains(&0x00) {
        return Err(bad(where_));
    }
    if bytes.first() == Some(&b'/') || bytes.last() == Some(&b'/') {
        return Err(bad(where_));
    }
    for segment in bytes.split(|&b| b == b'/') {
        // An empty segment is the `//` case and, at the ends, is already caught
        // above; catching it here too makes the rule one loop instead of three.
        if segment.is_empty() || segment == b"." || segment == b".." {
            return Err(bad(where_));
        }
    }
    Ok(())
}

/// MF §3.7: a region key matches `^[a-z][a-z0-9_-]{0,63}$`.
///
/// The key is **not** a template name and is never a `templates` index — the
/// record's own `template` member is (MF §3.7; build plan R6). All three v1
/// regions use the key `spine`.
pub fn check_region_key(s: &str) -> Result<(), Refusal> {
    let mut bytes = s.bytes();
    let ok = match bytes.next() {
        Some(first) if first.is_ascii_lowercase() => {
            s.len() <= 64
                && bytes.all(|b| {
                    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-'
                })
        }
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(Refusal::new(
            Status::RegionNameOutOfGrammar,
            format!("region key {s:?}"),
        ))
    }
}

/// A `files[].path`: an `esc`-encoded repository path, optionally followed by
/// `#` and a region key (MF §3.5, §3.7).
///
/// Splitting on `#` is why `path-hash-ambiguous` exists: a repository path may
/// legitimately contain a `#`, so a path with more than one is not decidable
/// between "a file called `a#b`" and "region `b` of file `a`".
pub fn split_region(path: &str) -> Result<(&str, Option<&str>), Refusal> {
    match path.split('#').count() {
        1 => Ok((path, None)),
        2 => {
            let (file, key) = path.split_once('#').expect("exactly one '#'");
            Ok((file, Some(key)))
        }
        _ => Err(Refusal::new(
            Status::PathHashAmbiguous,
            format!("files[].path {path:?}"),
        )),
    }
}

/// MF §3.6: `<template name>@<integer >= 1>`, the name matching
/// `^[a-z][a-z0-9-]{0,63}$`.
///
/// Note the name grammar admits `-` and **not** `_`, unlike a region key: the
/// twelve template names are `ci-github-collect`, `intent-change` and so on.
pub fn parse_template_ref(s: &str) -> Result<(&str, u64), Refusal> {
    let bad = || Refusal::new(Status::TemplateMalformed, format!("template {s:?}"));

    let (name, version) = s.rsplit_once('@').ok_or_else(bad)?;

    let mut bytes = name.bytes();
    let name_ok = match bytes.next() {
        Some(first) if first.is_ascii_lowercase() => {
            name.len() <= 64
                && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        }
        _ => false,
    };
    if !name_ok {
        return Err(bad());
    }

    // No leading zero, no sign, no empty run — the manifest's integers are
    // spelled one way everywhere (MF §2.2).
    if version.is_empty()
        || !version.bytes().all(|b| b.is_ascii_digit())
        || (version.len() > 1 && version.starts_with('0'))
    {
        return Err(bad());
    }
    let n: u64 = version.parse().map_err(|_| bad())?;
    if n < 1 {
        return Err(bad());
    }
    Ok((name, n))
}

/// MF §3.3: a branch name `git check-ref-format --branch` accepts, stored
/// `esc`-encoded.
///
/// The rules are git's, restated rather than shelled out to: `init` validates
/// this before a repository exists to ask, and the collector validates it from
/// trunk's manifest where invoking git per field would be a syscall per read.
pub fn check_branch_name(esc_encoded: &str) -> Result<(), Refusal> {
    let bad = || Refusal::new(Status::TrunkNotABranchName, "params.trunk");
    let bytes = spine_canon::unesc(esc_encoded).map_err(|_| bad())?;

    if bytes.is_empty() || bytes.len() > 255 {
        return Err(bad());
    }
    // git-check-ref-format(1), the rules that apply to a branch name.
    if bytes.first() == Some(&b'-') || bytes.first() == Some(&b'/') || bytes.last() == Some(&b'/') {
        return Err(bad());
    }
    if bytes.last() == Some(&b'.') || bytes.ends_with(b".lock") {
        return Err(bad());
    }
    if bytes == b"@" {
        return Err(bad());
    }
    for window in bytes.windows(2) {
        if matches!(window, b".." | b"@{" | b"//") {
            return Err(bad());
        }
    }
    for &b in &bytes {
        if b <= 0x20 || b == 0x7F || matches!(b, b'~' | b'^' | b':' | b'?' | b'*' | b'[' | b'\\') {
            return Err(bad());
        }
    }
    for segment in bytes.split(|&b| b == b'/') {
        if segment.is_empty() || segment.first() == Some(&b'.') {
            return Err(bad());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_grammar() {
        for good in ["myrepo", "a", "spine-kit", "a.b_c-1", &"x".repeat(64)] {
            assert!(check_repo(good).is_ok(), "{good:?} should be legal");
        }
        for bad in ["", &"x".repeat(65), "my repo", "my/repo", "café", "a#b"] {
            assert_eq!(
                check_repo(bad).unwrap_err().status,
                Status::RepoOutOfGrammar,
                "{bad:?} should be refused"
            );
        }
    }

    #[test]
    fn cli_version_excludes_the_upgrade_sentinel() {
        assert!(check_cli_version("1.4.0").is_ok());
        assert!(check_cli_version("1.4.0+build.7").is_ok());
        // MF §3.2: "`none` is excluded because `Spine-Upgrade` uses it as the
        // sentinel for 'no manifest'".
        assert_eq!(
            check_cli_version("none").unwrap_err().status,
            Status::CliVersionOutOfGrammar
        );
        // But a version that merely contains it is fine.
        assert!(check_cli_version("none.1").is_ok());
        assert_eq!(
            check_cli_version("1.4 0").unwrap_err().status,
            Status::CliVersionOutOfGrammar
        );
    }

    #[test]
    fn blob_ids_are_never_abbreviated() {
        let full = "cb4cd49034bbe25f76573c40d6711b2c33f9136f";
        assert!(check_blob(full, ObjectFormat::Sha1, "x").is_ok());
        assert_eq!(
            check_blob(&full[..7], ObjectFormat::Sha1, "x")
                .unwrap_err()
                .status,
            Status::BlobMalformed
        );
        // A sha1-length id in a sha256 repository is the wrong width.
        assert_eq!(
            check_blob(full, ObjectFormat::Sha256, "x").unwrap_err().status,
            Status::BlobMalformed
        );
        // Uppercase hex is not git's spelling.
        assert_eq!(
            check_blob(&full.to_uppercase(), ObjectFormat::Sha1, "x")
                .unwrap_err()
                .status,
            Status::BlobMalformed
        );
    }

    #[test]
    fn repo_paths_follow_mf_3_4() {
        for good in [
            "CONSTITUTION.md",
            ".spine/ci.sh",
            ".github/workflows/spine-collect.yml",
            "src/a=b.ts",
            r"caf\xc3\xa9.py",
        ] {
            assert!(check_repo_path(good, "x").is_ok(), "{good:?} should be legal");
        }
        for bad in [
            "",
            "/abs.md",
            "trailing/",
            "a//b",
            "./a",
            "a/../b",
            "a/./b",
            r"nul\x00byte",
        ] {
            assert_eq!(
                check_repo_path(bad, "x").unwrap_err().status,
                Status::PathsValueMalformed,
                "{bad:?} should be refused"
            );
        }
    }

    #[test]
    fn a_second_hash_is_ambiguous_rather_than_guessed() {
        assert_eq!(split_region("AGENTS.md").unwrap(), ("AGENTS.md", None));
        assert_eq!(
            split_region("AGENTS.md#spine").unwrap(),
            ("AGENTS.md", Some("spine"))
        );
        assert_eq!(
            split_region("a#b#c").unwrap_err().status,
            Status::PathHashAmbiguous
        );
    }

    #[test]
    fn template_refs_carry_a_name_and_a_version() {
        assert_eq!(parse_template_ref("intent@2").unwrap(), ("intent", 2));
        assert_eq!(
            parse_template_ref("ci-github-collect@4").unwrap(),
            ("ci-github-collect", 4)
        );
        // README decision 4 of 2026-08-26: never a bare `v2`.
        for bad in ["v2", "intent", "intent@", "intent@0", "intent@01", "intent@x", "Intent@2", "intent_bug@2"] {
            assert_eq!(
                parse_template_ref(bad).unwrap_err().status,
                Status::TemplateMalformed,
                "{bad:?} should be refused"
            );
        }
    }

    #[test]
    fn branch_names_follow_check_ref_format() {
        for good in ["main", "trunk", "release/2026", "a.b"] {
            assert!(check_branch_name(good).is_ok(), "{good:?} should be legal");
        }
        for bad in [
            "", "-lead", "/lead", "trail/", "trail.", "x.lock", "@", "a..b", "a@{b", "a//b",
            "a b", "a~b", "a^b", "a:b", "a?b", "a*b", "a[b", r"a\\b", "a/.hidden",
        ] {
            assert_eq!(
                check_branch_name(bad).unwrap_err().status,
                Status::TrunkNotABranchName,
                "{bad:?} should be refused"
            );
        }
    }
}
