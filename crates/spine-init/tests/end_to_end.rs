//! `spine init` against real git repositories.
//!
//! The plan's rules are unit-tested against an in-memory tree; these exercise
//! the thing the unit tests cannot — that the plan, the renders, the atomic
//! apply and the manifest agree with each other and with git.
//!
//! Two of these caught real defects the moment they first ran, and both are
//! worth naming because neither is visible from a unit test:
//!
//! - **`init` was not idempotent.** It never read the manifest already in the
//!   tree, so every path a previous run wrote read as `foreign` — present in
//!   HEAD, claimed by no record — and `spine-owned` + foreign is a refusal. A
//!   clean re-run refused four of its own five paths.
//! - **A region's blob is the region's, not the host file's.** The manifest
//!   records `ccf916b1…` for `AGENTS.md#spine` while the host file hashes to
//!   something else entirely, and only a run that writes both can tell.

use std::path::{Path, PathBuf};
use std::process::Command;

fn spine() -> PathBuf {
    // `cargo test` puts integration binaries next to the crate's own build
    // output, so the CLI is one directory up.
    let mut path = std::env::current_exe().expect("test binary");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join("spine")
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Option<Self> {
        let dir = std::env::temp_dir().join(format!("spine-e2e-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok()?;
        git(&dir, &["init", "-q", "-b", "main", "."])?;
        git(&dir, &["config", "user.email", "t@example.invalid"])?;
        git(&dir, &["config", "user.name", "Test"])?;
        // A python marker so `--langs` detection succeeds (PB §11's table), and
        // two files that already have hand-written content, so the region
        // writes have something to preserve.
        std::fs::write(dir.join("pyproject.toml"), "[project]\n").ok()?;
        std::fs::write(dir.join("AGENTS.md"), "# Agent notes\n\nHand-written.\n").ok()?;
        std::fs::write(dir.join(".gitignore"), "node_modules/\n").ok()?;
        git(&dir, &["add", "-A"])?;
        git(&dir, &["commit", "-q", "-m", "seed"])?;
        Some(Scratch(dir))
    }

    fn init(&self, extra: &[&str]) -> (i32, String) {
        let mut args = vec!["init", "--ci", "generic", "--identity", "alice@example.com"];
        args.extend_from_slice(extra);
        let out = Command::new(spine())
            .current_dir(&self.0)
            .args(&args)
            .output()
            .expect("spine runs");
        let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&out.stderr));
        (out.status.code().unwrap_or(-1), text)
    }

    fn commit(&self, message: &str) {
        git(&self.0, &["add", "-A"]).unwrap();
        git(&self.0, &["commit", "-q", "-m", message]).unwrap();
    }

    fn dirty(&self) -> usize {
        let out = Command::new("git")
            .current_dir(&self.0)
            .args(["status", "--porcelain"])
            .output()
            .expect("git");
        String::from_utf8_lossy(&out.stdout).lines().count()
    }

    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.0.join(path)).unwrap_or_default()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(dir: &Path, args: &[&str]) -> Option<()> {
    Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|_| ())
}

/// Every test here needs the CLI, and it only exists once the workspace is
/// built. Skipping rather than failing keeps `cargo test -p spine-init` honest
/// when run alone.
fn available() -> bool {
    spine().exists()
}

/// Which build of the CLI is on disk, **observed rather than assumed**.
///
/// The test crate's own cargo features say nothing about how `spine` was
/// built — `cargo test --workspace` and
/// `cargo build -p spine-cli --features synthetic-release` write the same
/// binary path — so a `#[cfg(feature = ...)]` here asserts a fact it cannot
/// see. The first version of this file did exactly that and failed whenever the
/// two disagreed, which is the wrong failure: the test was wrong, not the tool.
///
/// So the kind is read off the tool's own behaviour. CI §3.4 makes a
/// development build report `no-release-manifest` for every row, and that
/// diagnostic is the observable difference.
#[derive(Debug, PartialEq, Eq)]
enum BuildKind {
    Development,
    Release,
}

fn build_kind(scratch: &Scratch) -> BuildKind {
    let (_, text) = scratch.init(&["--dry-run"]);
    if text.contains("no-release-manifest") {
        BuildKind::Development
    } else {
        BuildKind::Release
    }
}

/// CI §3.4: a development build "renders no CI definition, writes no
/// `.spine/manifest.json`, creates no path, and reports `REFUSE` for every row
/// of the plan … It does not fall back on a default host, a tag in place of a
/// commit, an empty string, or a rendered file with the token left in."
#[test]
fn a_development_build_refuses_every_row_and_writes_nothing() {
    let Some(scratch) = Scratch::new("dev-build") else {
        return;
    };
    if !available() || build_kind(&scratch) != BuildKind::Development {
        return;
    }
    let (code, text) = scratch.init(&[]);
    assert_eq!(code, 2, "{text}");
    assert!(text.contains("no-release-manifest"), "{text}");
    assert_eq!(scratch.dirty(), 0, "a refusal leaves the tree as it was");
    assert!(!scratch.0.join(".spine").exists());
}

mod with_a_release {

    use super::*;

    /// A first `init` writes its render set and the manifest, and the manifest
    /// is written **last** (PB §6.7 step 4).
    #[test]
    fn a_first_init_writes_the_render_set_and_the_manifest() {
        let Some(scratch) = Scratch::new("first-init") else {
            return;
        };
        if !available() || build_kind(&scratch) != BuildKind::Release {
            return;
        }
        let (code, text) = scratch.init(&[]);
        assert_eq!(code, 0, "{text}");

        // The three managed regions went into files spine does not own, and the
        // hand-written content above them survived.
        let agents = scratch.read("AGENTS.md");
        assert!(agents.starts_with("# Agent notes\n\nHand-written.\n"));
        assert!(agents.contains("<!-- spine:begin agents-block@2 -->"));
        assert!(scratch.read(".gitignore").starts_with("node_modules/\n"));
        assert!(scratch.read(".gitignore").contains(".spine/cache/"));

        // The manifest is canonical: one line plus exactly one LF (MF §2.4).
        let manifest = std::fs::read(scratch.0.join(".spine/manifest.json")).unwrap();
        assert!(manifest.ends_with(b"\n"));
        assert_eq!(manifest.iter().filter(|b| **b == b'\n').count(), 1);
        let parsed =
            spine_manifest::Manifest::parse(&manifest, Some(spine_canon::ObjectFormat::Sha1))
                .expect("init writes a conforming manifest");
        assert_eq!(parsed.repo(), "spine-e2e-first-init");
        assert_eq!(parsed.trunk(), "main");
        assert_eq!(parsed.langs(), vec!["python"]);

        // Staging is deleted — step 4's last clause.
        assert!(
            std::fs::read_dir(scratch.0.join(".spine/cache/staging"))
                .map(|mut d| d.next().is_none())
                .unwrap_or(true),
            "no staging run survives a completed apply"
        );
    }

    /// MF §3.5: a region's `blob` is `git hash-object` over the **region's**
    /// bytes with no filters, never the host file's. The two differ, and only a
    /// run that writes both can tell them apart — which is why this is an
    /// end-to-end test and not a unit test.
    #[test]
    fn a_region_record_carries_the_regions_blob_not_its_hosts() {
        let Some(scratch) = Scratch::new("region-blob") else {
            return;
        };
        if !available() || build_kind(&scratch) != BuildKind::Release {
            return;
        }
        assert_eq!(scratch.init(&[]).0, 0);

        let manifest = std::fs::read(scratch.0.join(".spine/manifest.json")).unwrap();
        let parsed =
            spine_manifest::Manifest::parse(&manifest, Some(spine_canon::ObjectFormat::Sha1))
                .unwrap();
        let record = parsed
            .files()
            .into_iter()
            .find(|f| f.path == "AGENTS.md#spine")
            .expect("the region is recorded");

        // MF §8.1's published region blob, reached through a real init.
        assert_eq!(record.blob, "ccf916b1f5a2813b9156128dff6f3bc4036c8b2d");

        let host_blob = spine_canon::git_blob_id(
            &std::fs::read(scratch.0.join("AGENTS.md")).unwrap(),
            spine_canon::ObjectFormat::Sha1,
        );
        assert_ne!(
            record.blob, host_blob,
            "the host file's blob is a different value, and recording it would \
             fail G16 on every landing"
        );
    }

    /// PB §6.7: "On an initialised repo, `init` is idempotent." It was not: it
    /// never read the manifest already in the tree, so every path a previous
    /// run wrote read as `foreign` and `spine-owned` + foreign is a refusal.
    #[test]
    fn a_re_run_on_an_initialised_repository_skips_everything() {
        let Some(scratch) = Scratch::new("idempotent") else {
            return;
        };
        if !available() || build_kind(&scratch) != BuildKind::Release {
            return;
        }
        assert_eq!(scratch.init(&[]).0, 0);
        scratch.commit("spine init");

        let (code, text) = scratch.init(&["--dry-run"]);
        assert_eq!(code, 0, "{text}");
        assert!(!text.contains("REFUSE"), "{text}");
        assert!(text.contains("0 refused"), "{text}");
        assert_eq!(
            text.matches("skip").count(),
            5,
            "every path skips on a clean re-run (generic renders five): {text}"
        );
    }

    /// PB §6.7 step 3, and the rule the whole lifecycle rests on: "One
    /// `spine-owned` path with HEAD blob ≠ manifest blob stops the whole
    /// upgrade — a partial upgrade is the interrupted case by another name."
    #[test]
    fn a_hand_edited_spine_owned_path_refuses_and_nothing_is_written() {
        let Some(scratch) = Scratch::new("hand-edit") else {
            return;
        };
        if !available() || build_kind(&scratch) != BuildKind::Release {
            return;
        }
        assert_eq!(scratch.init(&[]).0, 0);
        scratch.commit("spine init");

        let ci = scratch.0.join(".spine/ci.sh");
        let mut edited = std::fs::read_to_string(&ci).unwrap();
        edited.push_str("# a human edited this\n");
        std::fs::write(&ci, edited).unwrap();
        scratch.commit("hand edit");

        // The dry run refuses, and exits 2 — "0, or 2 if it would refuse".
        let (code, text) = scratch.init(&["--dry-run"]);
        assert_eq!(code, 2, "{text}");
        assert!(text.contains("REFUSE .spine/ci.sh"), "{text}");
        // And only that path: the other four are untouched and still skip.
        assert!(text.contains("1 refused"), "{text}");

        // A real run refuses too, and writes nothing at all.
        let before = scratch.dirty();
        let (code, _) = scratch.init(&[]);
        assert_eq!(code, 2);
        assert_eq!(scratch.dirty(), before, "a refused plan writes nothing");
    }

    /// PB §11: `--langs` "detects from the tree … and **refuses** when it finds
    /// none rather than guessing".
    #[test]
    fn a_repository_with_no_language_marker_refuses() {
        let Some(scratch) = Scratch::new("no-langs") else {
            return;
        };
        if !available() || build_kind(&scratch) != BuildKind::Release {
            return;
        }
        std::fs::remove_file(scratch.0.join("pyproject.toml")).unwrap();
        scratch.commit("drop the marker");

        let (code, text) = scratch.init(&[]);
        assert_eq!(code, 2, "{text}");
        assert!(text.contains("refusing rather than guessing"), "{text}");
        assert!(!scratch.0.join(".spine").exists());
    }

    /// The full GitHub render set: seven paths, every CI body substituted, and
    /// every recorded blob equal to what landed on disk.
    ///
    /// This is the test CI §15 D20 denied for as long as it stood — the
    /// comment describing §3.4's token scan spelled `@@`, so `.spine/ci.sh`
    /// failed the scan, and `.spine/ci.sh` is row 1 of every provider.
    #[test]
    fn the_github_render_set_lands_with_every_token_substituted() {
        let Some(scratch) = Scratch::new("github-render") else {
            return;
        };
        if !available() || build_kind(&scratch) != BuildKind::Release {
            return;
        }
        let out = Command::new(spine())
            .current_dir(&scratch.0)
            .args(["init", "--ci", "github", "--identity", "alice@example.com"])
            .output()
            .expect("spine runs");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );

        let ci_sh = scratch.read(".spine/ci.sh");
        assert!(
            !ci_sh.contains("@@"),
            "no token survives a render, in a comment or anywhere else"
        );
        assert!(ci_sh.contains("https://dist.invalid/spine-synthetic"));

        let collect = scratch.read(".github/workflows/spine-collect.yml");
        assert!(!collect.contains("PIN_"), "every action pin substituted");
        assert!(collect.contains("uses: actions/checkout@0000"));

        // Every recorded blob equals what landed. A mismatch here is a G16
        // failure on the repository's very first landing.
        let manifest = std::fs::read(scratch.0.join(".spine/manifest.json")).unwrap();
        let parsed =
            spine_manifest::Manifest::parse(&manifest, Some(spine_canon::ObjectFormat::Sha1))
                .expect("conforming");
        assert_eq!(parsed.files().len(), 7, "five plus the two workflows");
        for record in parsed.files() {
            if record.region.is_some() {
                continue; // covered by the region test above
            }
            let on_disk = std::fs::read(scratch.0.join(&record.file_path)).unwrap();
            assert_eq!(
                record.blob,
                spine_canon::git_blob_id(&on_disk, spine_canon::ObjectFormat::Sha1),
                "{} records a blob that is not what landed",
                record.path
            );
        }
    }

    /// The keyring seed, through real `ssh-keygen` keys.
    ///
    /// PB §11 and MF §4.5 are unconditional: in solo mode "the one principal
    /// holds all three namespaces". A pipeline key does not change the mode —
    /// mode is the distinct signoff fingerprint count — so it does not change
    /// what the lone human holds. The opposite reading left a solo repository
    /// unable to land ever again if its CI secret was lost, since PB §7.5's
    /// recovery landing wants two distinct protected reviewers.
    #[test]
    fn the_keyring_seed_gives_a_solo_signer_all_three_namespaces() {
        let Some(scratch) = Scratch::new("keyring") else {
            return;
        };
        if !available() || build_kind(&scratch) != BuildKind::Release {
            return;
        }
        for (name, comment) in [("alice", "alice@example.com"), ("ci", "ci@example.com")] {
            let ok = Command::new("ssh-keygen")
                .current_dir(&scratch.0)
                .args(["-q", "-t", "ed25519", "-N", "", "-C", comment, "-f", name])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !ok {
                return; // no ssh-keygen on this host
            }
        }

        let run = |args: &[&str]| {
            Command::new(spine())
                .current_dir(&scratch.0)
                .args(args)
                .output()
                .expect("spine runs")
        };

        // Solo, with the pipeline key in the same invocation.
        let out = run(&[
            "init",
            "--ci",
            "generic",
            "--signer-key",
            "alice.pub",
            "--pipeline-key",
            "ci.pub",
        ]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );

        let keyring = scratch.read(".spine/allowed_signers");
        assert!(
            keyring.contains(
                "alice@example.com namespaces=\"spine-signoff@v1,spine-review@v1,spine-seal@v1\""
            ),
            "the one principal holds all three:\n{keyring}"
        );
        assert!(keyring.contains("ci@example.com namespaces=\"spine-seal@v1\""));

        // And it lints clean under the reader the gates use.
        let parsed = spine_manifest::Keyring::parse(keyring.as_bytes());
        assert!(parsed.is_clean(), "{:?}", parsed.findings);
        assert_eq!(parsed.mode, spine_manifest::Mode::Solo);
    }

    /// PB §11: "A first `init` with no signing key cannot produce a trust root
    /// and says so." A pipeline key alone would seed exactly that — and it
    /// lints clean, so the refusal cannot come from the lint.
    #[test]
    fn a_pipeline_key_without_a_signer_refuses() {
        let Some(scratch) = Scratch::new("keyring-no-signer") else {
            return;
        };
        if !available() || build_kind(&scratch) != BuildKind::Release {
            return;
        }
        let ok = Command::new("ssh-keygen")
            .current_dir(&scratch.0)
            .args([
                "-q",
                "-t",
                "ed25519",
                "-N",
                "",
                "-C",
                "ci@example.com",
                "-f",
                "ci",
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            return;
        }

        let out = Command::new(spine())
            .current_dir(&scratch.0)
            .args(["init", "--ci", "generic", "--pipeline-key", "ci.pub"])
            .output()
            .expect("spine runs");
        assert_eq!(out.status.code(), Some(2));
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("cannot produce a trust root"),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!scratch.0.join(".spine").exists());
    }
}
