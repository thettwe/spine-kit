//! GR §5.3's parse of `git --version`.
//!
//! "**The parse is normative, because a mis-parse forks both the digest and
//! §3.3's `wrong-git` check.** Over `git --version`'s output: take the first
//! maximal run of ASCII digits, then the first maximal run of ASCII digits
//! following the next `.`; record the two joined by `.`."
//!
//! Two consumers, and they must agree: `git_version` is a digest-bearing member
//! of every report, and `--verify` compares the running binary's parse against
//! the seal's `git=` and exits 3 `wrong-git` on disagreement (GR §3.3). PB §7.4
//! rule 4 makes that "a requirement, not a warning, because `merge-tree` output
//! is a git version's contract."
//!
//! "Nothing else in this document reads a version out of a version string."

/// GR §5.3: "Output from which two such runs cannot be read is a refusal: no
/// report is produced, and `--verify` exits 3 `wrong-git`."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitVersionError {
    /// No run of ASCII digits anywhere in the output.
    NoMajorRun,
    /// A major run was found and no `.` follows it.
    NoSeparator,
    /// A `.` was found and no run of ASCII digits follows it.
    NoMinorRun,
}

impl core::fmt::Display for GitVersionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GitVersionError::NoMajorRun => f.write_str("no major version digits in git --version"),
            GitVersionError::NoSeparator => f.write_str("no '.' after the major version"),
            GitVersionError::NoMinorRun => f.write_str("no minor version digits after the '.'"),
        }
    }
}

impl core::error::Error for GitVersionError {}

/// `"<major>.<minor>"` — "Patch level, release-candidate suffixes and vendor
/// suffixes are discarded before recording."
///
/// The three worked cases GR §5.3 publishes:
/// `git version 2.39.5 (Apple Git-154)` → `"2.39"`;
/// `git version 2.45.1.windows.2` → `"2.45"`;
/// `git version 2.46.GIT` → `"2.46"`.
///
/// DERIVED: the spec says "the first maximal run of ASCII digits **following**
/// the next `.`", which is implemented literally — the search resumes after the
/// separator and takes the first digit run it meets, whether or not that run is
/// adjacent to the `.`. The stricter alternative (require adjacency) agrees on
/// all three published cases and differs only on output no shipped git
/// produces, such as `git version 2.x39`; the literal reading is taken because
/// the spec states an algorithm and this is what the algorithm says.
pub fn parse(output: &str) -> Result<String, GitVersionError> {
    let bytes = output.as_bytes();

    // "Take the first maximal run of ASCII digits."
    let major_start = bytes
        .iter()
        .position(u8::is_ascii_digit)
        .ok_or(GitVersionError::NoMajorRun)?;
    let major_end = major_start
        + bytes[major_start..]
            .iter()
            .position(|b| !b.is_ascii_digit())
            .unwrap_or(bytes.len() - major_start);

    // "…then the first maximal run of ASCII digits following the next `.`."
    // The next `.` is the next one after the major run, not the first `.` in
    // the string: `git version 2.45` has none before the run, but a vendor
    // banner could.
    let dot = major_end
        + bytes[major_end..]
            .iter()
            .position(|b| *b == b'.')
            .ok_or(GitVersionError::NoSeparator)?;
    let minor_start = dot
        + 1
        + bytes[dot + 1..]
            .iter()
            .position(u8::is_ascii_digit)
            .ok_or(GitVersionError::NoMinorRun)?;
    let minor_end = minor_start
        + bytes[minor_start..]
            .iter()
            .position(|b| !b.is_ascii_digit())
            .unwrap_or(bytes.len() - minor_start);

    Ok(format!(
        "{}.{}",
        &output[major_start..major_end],
        &output[minor_start..minor_end]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GR §5.3's three published cases, byte for byte.
    #[test]
    fn gr_5_3_worked_cases() {
        assert_eq!(parse("git version 2.39.5 (Apple Git-154)").unwrap(), "2.39");
        assert_eq!(parse("git version 2.45.1.windows.2").unwrap(), "2.45");
        assert_eq!(parse("git version 2.46.GIT").unwrap(), "2.46");
    }

    /// GR §8.2's vector carries `git_version: "2.45"`.
    #[test]
    fn the_vectors_git_version_parses_from_a_plain_banner() {
        assert_eq!(parse("git version 2.45.1").unwrap(), "2.45");
        assert_eq!(parse("git version 2.45").unwrap(), "2.45");
    }

    /// "Patch level, release-candidate suffixes and vendor suffixes are
    /// discarded" — a multi-digit minor is not truncated by that discarding.
    #[test]
    fn a_multi_digit_minor_survives_intact() {
        assert_eq!(parse("git version 2.100.0").unwrap(), "2.100");
        assert_eq!(parse("git version 10.2.0").unwrap(), "10.2");
    }

    /// GR §5.3: "Output from which two such runs cannot be read is a refusal."
    #[test]
    fn output_with_no_two_runs_is_a_refusal() {
        assert_eq!(parse("git version"), Err(GitVersionError::NoMajorRun));
        assert_eq!(parse("git version 2"), Err(GitVersionError::NoSeparator));
        assert_eq!(parse("git version 2.GIT"), Err(GitVersionError::NoMinorRun));
    }

    /// The "next `.`" is the one after the major run. A banner carrying a dot
    /// before the version must not consume it.
    #[test]
    fn the_separator_is_sought_after_the_major_run() {
        assert_eq!(parse("git.exe version 2.45.1").unwrap(), "2.45");
    }

    /// The version parsed here is the value that lands in `git_version` and is
    /// compared against the seal's `git=`; a real `git --version` on this
    /// machine must round-trip through it.
    #[test]
    fn the_local_git_banner_parses() {
        let out = std::process::Command::new("git").arg("--version").output();
        let Ok(out) = out else {
            return; // No git on this machine; the vectors above still stand.
        };
        let banner = String::from_utf8_lossy(&out.stdout);
        let parsed = parse(banner.trim()).expect("git --version must parse");
        let (major, minor) = parsed.split_once('.').unwrap();
        assert!(major.bytes().all(|b| b.is_ascii_digit()), "{parsed}");
        assert!(minor.bytes().all(|b| b.is_ascii_digit()), "{parsed}");
        assert!(
            banner.contains(&parsed),
            "{banner} does not contain {parsed}"
        );
    }
}
