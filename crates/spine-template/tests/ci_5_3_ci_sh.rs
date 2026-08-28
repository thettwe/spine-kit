//! `ci.md` §5.3 — `.spine/ci.sh`, the collector's entry point, reproduced.
//!
//! This is the largest artifact the release ships into a repository and the
//! only executable one. It is `spine-owned` with its blob recorded in the
//! manifest, so its bytes are shipped bytes: §5.3's two digests moved the last
//! time a line of it changed.
//!
//! The digests are over the **unsubstituted** template — §5.3: "Computed
//! digests for exactly these bytes, with `@@DIST_BASE@@` unsubstituted — a
//! rendered `ci.sh` carries the release's URL there and has a different id."

use spine_canon::ObjectFormat;
use spine_template::substitute;

const CI_SH: &[u8] = include_bytes!("vectors/ci-5.3-ci.sh");

#[test]
fn ci_5_3_line_count_and_both_digests() {
    assert_eq!(
        CI_SH.iter().filter(|b| **b == b'\n').count(),
        319,
        "§5.3 publishes 319 lines"
    );
    assert_eq!(
        spine_canon::git_blob_id(CI_SH, ObjectFormat::Sha1),
        "6946401d143a2f6b2504a9c267a9dc10567c9a83"
    );
    assert_eq!(
        spine_canon::sha256_hex(CI_SH),
        "6599ef42df8771c0f04ba54631f1f5a19de2860c73725be4ae06c9a0d1603a1d"
    );
}

/// `ci.sh` is a POSIX shell script read by whatever `/bin/sh` the runner image
/// has. §5.3 re-checked it under `sh`, `dash`, `bash` and `zsh`; this asserts
/// the byte-level properties that make that possible, and the shell check
/// itself lives in the repository's own CI rather than in a unit test.
#[test]
fn framing_is_what_a_posix_shell_needs() {
    assert!(CI_SH.starts_with(b"#!"), "a shebang");
    assert!(
        !CI_SH.contains(&b'\r'),
        "no CR — .gitattributes pins eol=lf"
    );
    assert!(CI_SH.ends_with(b"\n"), "a final newline");
    assert!(
        core::str::from_utf8(CI_SH).is_ok(),
        "UTF-8, so the substitution pass can walk it as characters"
    );
}

/// The template carries exactly one render token, and the byte scan must refuse
/// it *before* substitution — which is the property that makes the scan a
/// meaningful gate rather than a formality.
/// The scan must refuse the **unrendered** template and accept the **rendered**
/// one. Only the second half is new, and it is the half that was false: §5.3's
/// line 24 carried `'@@'` inside the comment describing the scan, so a rendered
/// ci.sh failed it too — and `.spine/ci.sh` is on every provider's row, so no
/// repository could be initialised at all (CI §15 D20).
#[test]
fn a_rendered_ci_sh_passes_the_scan_that_an_unrendered_one_fails() {
    let release = spine_template::ReleaseManifest::parse(
        br#"{"release_manifest_version":1,"version":"1.4.0",
             "dist_base":"https://dist.example.invalid/spine",
             "actions":{
               "checkout":{"repo":"actions/checkout","commit":"11bd71901bbe5b1630ceea73d27597364c9af683"},
               "upload_artifact":{"repo":"actions/upload-artifact","commit":"ea165f8d65b6e75b540449e92b4886f43607fa02"},
               "download_artifact":{"repo":"actions/download-artifact","commit":"fa0a91b85d4f404e444e00e005971372dc801d16"}}}"#,
    )
    .expect("a conforming release manifest");
    let table = spine_template::Table::build(&release, "main").expect("a legal trunk name");
    let body = core::str::from_utf8(CI_SH).unwrap();

    assert!(
        substitute::scan(body).is_err(),
        "an unrendered ci.sh must never pass the scan"
    );
    let rendered = table
        .render_checked(body)
        .expect("and a rendered one must pass it — otherwise nothing can be initialised");
    assert!(!rendered.contains("@@"));
    assert!(rendered.contains("https://dist.example.invalid/spine"));
}

#[test]
fn the_unrendered_template_carries_one_token_and_fails_the_scan() {
    let text = core::str::from_utf8(CI_SH).unwrap();
    assert_eq!(
        text.matches(substitute::DIST_BASE).count(),
        1,
        "one @@DIST_BASE@@, and ci-generic carries no trunk name since ci.sh \
         takes trunk as an argument (CI §3.4 step 4)"
    );
    for pin in substitute::PIN_TOKENS {
        assert_eq!(
            text.matches(pin).count(),
            0,
            "{pin} belongs to the GitHub workflow templates, not to ci.sh"
        );
    }
    assert!(
        substitute::scan(text).is_err(),
        "an unrendered ci.sh must never pass the scan"
    );
}

/// §5.4 item 1: the process-wide `umask 077` was narrowed to `umask 022` plus
/// an explicit `chmod 0700 "$WORK"`, because at 077 nothing the collector
/// writes is reachable to the mapped id M1 spawns runners under — so
/// `profile=container` was unlicensable on every host.
#[test]
fn the_umask_narrowing_is_in_these_bytes() {
    let text = core::str::from_utf8(CI_SH).unwrap();
    // Directives, not substrings: the script also *names* `umask 077` in the
    // comment that explains why it is not that, so a substring test would
    // assert the opposite of what it reads.
    let directives: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .collect();
    assert!(
        directives.contains(&"umask 022"),
        "the narrowed process-wide umask"
    );
    assert!(
        !directives.iter().any(|l| l.starts_with("umask 077")),
        "the mode that made profile=container unlicensable on every host"
    );
    assert!(
        directives
            .iter()
            .any(|l| l.starts_with("chmod 0700 \"$WORK\"")),
        "and the explicit tightening of $WORK that replaced it"
    );
    assert!(
        directives.iter().any(|l| l.starts_with("chmod 0755")),
        "with the install directory left traversable to the mapped id"
    );
}

/// §5.6: `repo.maven.apache.org` and `services.gradle.org` were removed on
/// 2026-08-27 with Kotlin. "no invocation set can reach a Gradle build and the
/// two entries granted the untrusted job egress nothing in v1 could use."
#[test]
fn the_registry_allowlist_carries_no_dead_kotlin_hosts() {
    let text = core::str::from_utf8(CI_SH).unwrap();
    assert!(text.contains("SPINE_ALLOWED_HOSTS"));
    for dead in ["repo.maven.apache.org", "services.gradle.org"] {
        assert!(!text.contains(dead), "{dead} was removed with Kotlin");
    }
    // The three v1 registries that have a host.
    for live in [
        "pypi.org",
        "files.pythonhosted.org",
        "registry.npmjs.org",
        "pub.dev",
    ] {
        assert!(text.contains(live), "{live} is a v1 registry host");
    }
}
