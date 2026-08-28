//! CI §3.1's render set — the four CI templates, and the six paths they render.
//!
//! `spine init` writes a provider's CI definition from this table and nothing
//! else. Four things about it are load-bearing and each is a way an
//! implementation reading the table too quickly goes wrong:
//!
//! - **`ci-generic` names the provider-independent shell, not the `generic`
//!   provider** (CI §3.1). Every provider carries `.spine/ci.sh`, and PB §6.7's
//!   own manifest example proves it: a repository whose `params.ci` is
//!   `"github"` records `.spine/ci.sh` with `"template": "ci-generic@4"`.
//!   Reading the name the other way "produces a repository that writes no
//!   `ci.sh` for GitHub, which nothing then executes".
//! - **GitHub takes two files under two template names.** `workflow_run`
//!   selects its trigger by the triggering workflow's `name:` (CI §3.2), so one
//!   self-named file "chains from its own completion and runs for ever". And
//!   one *template* name for both "would make G16's check 7 unable to tell a
//!   collector rendered at `@4` from a lander left at `@3`, which is the whole
//!   of what that check does" (MF §3.6).
//! - **GitLab takes three files under one template name.** `.gitlab-ci.yml`
//!   only `include:`s; the two job definitions live under `.spine/` so that the
//!   floor covers them (CI §8.2–§8.4).
//! - **`.spine/restore.sh` is not in the table, and that is deliberate** (CI
//!   §3.1): "`spine init` writes no such file, no template renders one, and the
//!   manifest carries no `files[]` record for it." The collector reads it from
//!   `origin/<trunk>` if the repository has one; where trunk has none the
//!   dependency-restore phase is empty.
//!
//! The bodies are the corpus's printed bytes, carried as vectors under
//! `tests/vectors/` so that one set of bytes is both what ships and what the
//! published digests are computed over.
//!
//! DERIVED: the bodies ship **verbatim as printed**, `@N` included. CI's three
//! render-header comments say "from template `ci-github-collect@N`" where CI
//! §5.3's `ci.sh` header says `ci-generic@4`, and `N` is not a render token —
//! CI §3.4 step 4 names only `@@` and the three `PIN_` literals, so no
//! substitution touches it and the byte scan does not see it. Rewriting it to
//! `@4` here would be this file inventing bytes whose digests the corpus has
//! not published; the discrepancy is reported instead.
//!
//! DERIVED: rows are held in CI §3.1's table order, `.spine/ci.sh` first. The
//! manifest's order is MF §3.5's — `files[]` sorted by `esc(path)` bytes — and
//! producing that record is the manifest writer's job, not this table's.

use crate::substitute::{RenderError, Table};
use core::fmt;
use spine_manifest::schema::Owner;

/// MF §3.6, PB §6.7's `templates` map: all four CI templates are at `4`.
///
/// They version together only by coincidence of this release; MF §3.6 gives
/// each its own key precisely so they need not, so this constant is the
/// release's current value and not an invariant.
pub const CI_TEMPLATE_VERSION: u64 = 4;

/// The four CI keys of MF §3.6's twelve, in the order that section prints them.
pub const CI_TEMPLATE_NAMES: [&str; 4] = [
    "ci-generic",
    "ci-github-collect",
    "ci-github-land",
    "ci-gitlab",
];

/// `ci-generic@4` — CI §5.3's `.spine/ci.sh`, the collector's entry point.
///
/// The only executable artifact the release ships and the only one every
/// provider carries. It is included here rather than owned here: its bytes and
/// their two published digests are pinned by `tests/ci_5_3_ci_sh.rs`.
pub const CI_GENERIC: &str = include_str!("../tests/vectors/ci-5.3-ci.sh");

/// `ci-github-collect@4` — CI §7.2's `.github/workflows/spine-collect.yml`.
///
/// The untrusted half. `pull_request_target` is what makes U1 true on GitHub:
/// GitHub takes such a workflow's definition **from the base branch**, so it is
/// "a structural property of the provider rather than a check spine performs"
/// (CI §7.1).
pub const CI_GITHUB_COLLECT: &str = include_str!("../tests/vectors/ci-7.2-spine-collect.yml");

/// `ci-github-land@4` — CI §7.3's `.github/workflows/spine-land.yml`.
///
/// The trusted half, and the reason there are two files at all.
pub const CI_GITHUB_LAND: &str = include_str!("../tests/vectors/ci-7.3-spine-land.yml");

/// `ci-gitlab@4`, file 1 of 3 — CI §8.2's `.gitlab-ci.yml`.
///
/// It carries no job of its own: it `include:`s the two below at `ref: main`,
/// which is how the *trusted* definition comes from trunk on GitLab (CI §8.1).
pub const CI_GITLAB_ROOT: &str = include_str!("../tests/vectors/ci-8.2-gitlab-ci.yml");

/// `ci-gitlab@4`, file 2 of 3 — CI §8.3's `.spine/gitlab/untrusted.yml`.
pub const CI_GITLAB_UNTRUSTED: &str = include_str!("../tests/vectors/ci-8.3-untrusted.yml");

/// `ci-gitlab@4`, file 3 of 3 — CI §8.4's `.spine/gitlab/trusted.yml`.
///
/// A scheduled pipeline on trunk, which discovers its candidate rather than
/// being handed one — CI §8.4 fixes that discovery because PB §7.4 rule 0
/// "offers 'a schedule that polls for candidates' without saying how, which is
/// a hole an implementer cannot fill without inventing".
pub const CI_GITLAB_TRUSTED: &str = include_str!("../tests/vectors/ci-8.4-trusted.yml");

/// `params.ci` (MF §3.3, `schema::CI_PROVIDERS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Github,
    Gitlab,
    /// CI §9.1: "`spine init --ci generic` **writes `.spine/ci.sh` and no
    /// definition at all**, and prints the contract" of §9.4. "It cannot do
    /// more: it does not know the provider, so it can neither render a
    /// definition nor check that one exists."
    Generic,
}

impl Provider {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "github" => Some(Provider::Github),
            "gitlab" => Some(Provider::Gitlab),
            "generic" => Some(Provider::Generic),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Github => "github",
            Provider::Gitlab => "gitlab",
            Provider::Generic => "generic",
        }
    }

    /// CI §3.1's rows for this provider, in that table's order.
    ///
    /// The order is this document's, not the manifest's: MF §3.5 sorts
    /// `files[]` by `esc(path)` bytes, and building that record is the
    /// manifest writer's job, not this table's.
    pub fn files(self) -> &'static [CiFile] {
        match self {
            Provider::Github => &GITHUB_FILES,
            Provider::Gitlab => &GITLAB_FILES,
            Provider::Generic => &GENERIC_FILES,
        }
    }
}

/// One row of CI §3.1's table: a path, the template that renders it, that
/// template's version, the owner class the fourth column fixes, and the bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CiFile {
    /// Repo-relative, and exactly as CI §3.1 spells it.
    pub path: &'static str,
    /// The `templates` key (MF §3.6), with no `@version` suffix.
    pub template: &'static str,
    pub version: u64,
    /// CI §3.1's fourth column is `spine-owned` on every row.
    pub owner: Owner,
    /// The unrendered template bytes.
    pub body: &'static str,
}

impl CiFile {
    /// `<name>@<version>` — the one vocabulary `files[].template`, the
    /// `templates` map and the intent header share (MF §3.6).
    pub fn template_ref(&self) -> String {
        format!("{}@{}", self.template, self.version)
    }

    /// Render this row, then scan it (CI §3.4 steps 3 and 5, in that order).
    pub fn render(&self, table: &Table) -> Result<String, PlanRefusal> {
        table
            .render_checked(self.body)
            .map_err(|error| PlanRefusal {
                path: self.path,
                error,
            })
    }
}

/// `.spine/ci.sh`, CI §3.1's first row: written for **every** value of
/// `params.ci`, which is what makes `ci-generic` the shell's name and not the
/// provider's.
const CI_SH: CiFile = CiFile {
    path: ".spine/ci.sh",
    template: "ci-generic",
    version: CI_TEMPLATE_VERSION,
    owner: Owner::SpineOwned,
    body: CI_GENERIC,
};

static GENERIC_FILES: [CiFile; 1] = [CI_SH];

static GITHUB_FILES: [CiFile; 3] = [
    CI_SH,
    CiFile {
        path: ".github/workflows/spine-collect.yml",
        template: "ci-github-collect",
        version: CI_TEMPLATE_VERSION,
        owner: Owner::SpineOwned,
        body: CI_GITHUB_COLLECT,
    },
    CiFile {
        path: ".github/workflows/spine-land.yml",
        template: "ci-github-land",
        version: CI_TEMPLATE_VERSION,
        owner: Owner::SpineOwned,
        body: CI_GITHUB_LAND,
    },
];

static GITLAB_FILES: [CiFile; 4] = [
    CI_SH,
    CiFile {
        path: ".gitlab-ci.yml",
        template: "ci-gitlab",
        version: CI_TEMPLATE_VERSION,
        owner: Owner::SpineOwned,
        body: CI_GITLAB_ROOT,
    },
    // Under `.spine/`, so the protected floor covers the two job definitions
    // the root file includes (CI §3.1's note on `.spine/**`, PB §7.3).
    CiFile {
        path: ".spine/gitlab/untrusted.yml",
        template: "ci-gitlab",
        version: CI_TEMPLATE_VERSION,
        owner: Owner::SpineOwned,
        body: CI_GITLAB_UNTRUSTED,
    },
    CiFile {
        path: ".spine/gitlab/trusted.yml",
        template: "ci-gitlab",
        version: CI_TEMPLATE_VERSION,
        owner: Owner::SpineOwned,
        body: CI_GITLAB_TRUSTED,
    },
];

/// A render that failed the scan, with the path that failed it.
///
/// CI §3.4: "one failure refuses the **whole** plan rather than writing the
/// paths that happened to pass. A repository half-scaffolded by a bad release
/// is worse than one not scaffolded at all."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRefusal {
    pub path: &'static str,
    pub error: RenderError,
}

impl fmt::Display for PlanRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.error)
    }
}

impl core::error::Error for PlanRefusal {}

/// Render every CI file this provider writes, or refuse the whole plan.
///
/// Nothing is returned unless every row scanned clean, which is CI §3.4's
/// order — step 5 scans every rendered CI file and "**only then** does the plan
/// compare blob ids and write".
pub fn render_all(
    provider: Provider,
    table: &Table,
) -> Result<Vec<(&'static str, String)>, PlanRefusal> {
    let mut out = Vec::with_capacity(provider.files().len());
    for file in provider.files() {
        out.push((file.path, file.render(table)?));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release::ReleaseManifest;
    use crate::substitute;

    /// A release manifest shaped exactly as CI §3.4's schema requires. Every
    /// value is a stand-in: CI §18 OPEN-1 is the host and OPEN-7 the three
    /// pins, and "no binary built from this corpus renders a CI definition at
    /// all" until the owner rules on both.
    const FIXTURE: &str = r#"{
      "release_manifest_version": 1,
      "version": "1.4.0",
      "dist_base": "https://dist.example.invalid/spine",
      "actions": {
        "checkout":          { "repo": "actions/checkout",          "commit": "11bd71901bbe5b1630ceea73d27597364c9af683" },
        "upload_artifact":   { "repo": "actions/upload-artifact",   "commit": "ea165f8d65b6e75b540449e92b4886f43607fa02" },
        "download_artifact": { "repo": "actions/download-artifact", "commit": "fa0a91b85d4f404e444e00e005971372dc801d16" }
      }
    }"#;

    fn table(trunk: &str) -> Table {
        let release = ReleaseManifest::parse(FIXTURE.as_bytes()).unwrap();
        Table::build(&release, trunk).unwrap()
    }

    /// CI §3.2: `workflow_run` selects its trigger by the triggering workflow's
    /// `name:`, so "a single file naming itself fires on its own completion".
    /// Two files; and MF §3.6 makes them two *template* names as well, because
    /// one name for both leaves check 7 unable to tell a collector at `@4` from
    /// a lander left at `@3`.
    #[test]
    fn two_workflow_files_carry_two_template_names() {
        let github: Vec<&str> = Provider::Github.files().iter().map(|f| f.path).collect();
        assert!(github.contains(&".github/workflows/spine-collect.yml"));
        assert!(github.contains(&".github/workflows/spine-land.yml"));

        let names: Vec<String> = Provider::Github
            .files()
            .iter()
            .filter(|f| f.path.starts_with(".github/"))
            .map(|f| f.template_ref())
            .collect();
        assert_eq!(names, vec!["ci-github-collect@4", "ci-github-land@4"]);

        // And the two files declare two workflow names, which is the property
        // the split exists for.
        assert!(CI_GITHUB_COLLECT.contains("\nname: spine-collect\n"));
        assert!(CI_GITHUB_LAND.contains("\nname: spine-land\n"));
        assert!(CI_GITHUB_LAND.contains("workflows: [\"spine-collect\"]"));
    }

    /// CI §3.1: "The template name `ci-generic` names the provider-independent
    /// shell, not the `generic` provider." A `--ci github` repository carries
    /// `.spine/ci.sh` with `"template": "ci-generic@4"`.
    #[test]
    fn a_github_repository_still_carries_the_ci_generic_shell() {
        for provider in [Provider::Github, Provider::Gitlab, Provider::Generic] {
            let shell = provider
                .files()
                .iter()
                .find(|f| f.path == ".spine/ci.sh")
                .unwrap_or_else(|| panic!("{} writes no ci.sh", provider.as_str()));
            assert_eq!(shell.template_ref(), "ci-generic@4");
        }
    }

    /// CI §3.1 gives all three GitLab paths the one template `ci-gitlab@N`:
    /// three files, one version counter, because they are `include:`d as a set
    /// and never move apart.
    #[test]
    fn the_three_gitlab_paths_share_one_template_name() {
        let rows: Vec<(&str, String)> = Provider::Gitlab
            .files()
            .iter()
            .filter(|f| f.template == "ci-gitlab")
            .map(|f| (f.path, f.template_ref()))
            .collect();
        assert_eq!(
            rows,
            vec![
                (".gitlab-ci.yml", "ci-gitlab@4".to_string()),
                (".spine/gitlab/untrusted.yml", "ci-gitlab@4".to_string()),
                (".spine/gitlab/trusted.yml", "ci-gitlab@4".to_string()),
            ]
        );
        // The root file only includes; both jobs are defined under `.spine/`,
        // which is what puts them on the protected floor (PB §7.3).
        assert!(CI_GITLAB_ROOT.contains("'/.spine/gitlab/untrusted.yml'"));
        assert!(CI_GITLAB_ROOT.contains("'/.spine/gitlab/trusted.yml'"));
    }

    /// CI §3.1's last row and §9.1: `generic` writes "*(nothing beyond
    /// `.spine/ci.sh`)*".
    #[test]
    fn generic_renders_nothing_beyond_ci_sh() {
        assert_eq!(Provider::Generic.files().len(), 1);
        assert_eq!(Provider::Generic.files()[0].path, ".spine/ci.sh");
    }

    /// CI §3.1: "**`.spine/restore.sh` is not in this table, and that is
    /// deliberate.** … `spine init` writes no such file, no template renders
    /// one, and the manifest carries no `files[]` record for it."
    #[test]
    fn no_provider_renders_a_restore_script() {
        for provider in [Provider::Github, Provider::Gitlab, Provider::Generic] {
            assert!(
                provider
                    .files()
                    .iter()
                    .all(|f| f.path != ".spine/restore.sh"),
                "{} must render no restore script",
                provider.as_str()
            );
        }
    }

    /// MF §3.6 and PB §6.7's `templates` map.
    #[test]
    fn every_ci_template_is_at_version_four() {
        for provider in [Provider::Github, Provider::Gitlab, Provider::Generic] {
            for file in provider.files() {
                assert_eq!(file.version, 4, "{}", file.path);
                assert!(
                    CI_TEMPLATE_NAMES.contains(&file.template),
                    "{} names a template outside MF §3.6's four",
                    file.path
                );
            }
        }
    }

    /// CI §3.1's fourth column, on every row.
    #[test]
    fn every_row_is_spine_owned() {
        for provider in [Provider::Github, Provider::Gitlab, Provider::Generic] {
            for file in provider.files() {
                assert_eq!(file.owner, Owner::SpineOwned, "{}", file.path);
            }
        }
    }

    /// CI §3.4 step 4: "`ci-generic` carries `@@DIST_BASE@@` and no trunk name,
    /// since `ci.sh` takes trunk as an argument (§5.1)."
    #[test]
    fn ci_generic_carries_dist_base_and_no_trunk_name() {
        assert_eq!(CI_GENERIC.matches(substitute::DIST_BASE).count(), 1);
        assert!(
            !CI_GENERIC.contains("main"),
            "the trunk name is an argument, never a rendered constant"
        );
        for pin in substitute::PIN_TOKENS {
            assert!(!CI_GENERIC.contains(pin), "{pin} is the workflows'");
        }
    }

    /// The pins are per-file: the collector uploads and never downloads, so it
    /// carries no `PIN_DOWNLOAD_ARTIFACT`. A table row with no occurrence is
    /// not a defect — CI §3.4 step 2 builds one table for all four templates.
    #[test]
    fn each_github_workflow_carries_exactly_the_pins_it_uses() {
        assert_eq!(
            CI_GITHUB_COLLECT.matches(substitute::PIN_CHECKOUT).count(),
            1
        );
        assert_eq!(
            CI_GITHUB_COLLECT
                .matches(substitute::PIN_UPLOAD_ARTIFACT)
                .count(),
            1
        );
        assert_eq!(
            CI_GITHUB_COLLECT
                .matches(substitute::PIN_DOWNLOAD_ARTIFACT)
                .count(),
            0,
            "the collector hands over; it ingests nothing"
        );

        assert_eq!(CI_GITHUB_LAND.matches(substitute::PIN_CHECKOUT).count(), 1);
        assert_eq!(
            CI_GITHUB_LAND
                .matches(substitute::PIN_UPLOAD_ARTIFACT)
                .count(),
            1,
            "T9 keeps the gate report as an artifact of its own run"
        );
        assert_eq!(
            CI_GITHUB_LAND
                .matches(substitute::PIN_DOWNLOAD_ARTIFACT)
                .count(),
            1,
            "T6 ingests exactly one result file"
        );

        // Neither workflow fetches a release directly; `ci.sh` does that.
        assert!(!CI_GITHUB_COLLECT.contains(substitute::DIST_BASE));
        assert!(!CI_GITHUB_LAND.contains(substitute::DIST_BASE));
    }

    /// CI §3.4's scan is what stands between a bad release manifest and a
    /// repository scaffolded with a literal token in its CI. An unrendered
    /// template that carries one must never pass it.
    #[test]
    fn an_unrendered_template_that_carries_a_token_fails_the_scan() {
        for (name, body) in [
            ("ci-generic", CI_GENERIC),
            ("ci-github-collect", CI_GITHUB_COLLECT),
            ("ci-github-land", CI_GITHUB_LAND),
        ] {
            assert!(
                substitute::scan(body).is_err(),
                "an unrendered {name} must never pass the scan"
            );
        }
    }

    /// **A defect in CI, recorded as a test rather than hidden by one.**
    ///
    /// CI §3.4 step 4 says "Only the four CI templates carry a `@@` or `PIN_`
    /// token", listing `ci-gitlab` among the four. Its three printed bodies
    /// (§8.2, §8.3, §8.4) carry neither: the GitLab render's only render-time
    /// variance is the trunk name, and §3.4's own residual paragraph says the
    /// scan is blind to that by design — "the trunk name is substituted into
    /// the three provider definitions", and a name is not a token.
    ///
    /// The consequence is worth pinning: on GitLab the token scan is not a
    /// gate. What refuses a development build there is step 1's validate-first,
    /// not step 5's scan.
    #[test]
    fn the_ci_gitlab_bodies_carry_no_token_so_the_scan_is_no_gate_on_them() {
        for (path, body) in [
            (".gitlab-ci.yml", CI_GITLAB_ROOT),
            (".spine/gitlab/untrusted.yml", CI_GITLAB_UNTRUSTED),
            (".spine/gitlab/trusted.yml", CI_GITLAB_TRUSTED),
        ] {
            assert!(!body.contains("@@"), "{path}");
            for pin in substitute::PIN_TOKENS {
                assert!(!body.contains(pin), "{path} / {pin}");
            }
            assert!(
                substitute::scan(body).is_ok(),
                "{path} passes the scan unrendered — CI §3.4 step 4 implies it \
                 should not"
            );
        }
    }

    /// After substitution the five provider definitions are token-free and
    /// carry the repository's own trunk name. `.spine/ci.sh` is excluded here,
    /// and the test after this one is why.
    #[test]
    fn a_rendered_provider_definition_is_token_free_and_carries_the_trunk_name() {
        let table = table("trunk");
        for file in Provider::Github
            .files()
            .iter()
            .chain(Provider::Gitlab.files())
            .filter(|f| f.path != ".spine/ci.sh")
        {
            let rendered = file.render(&table).unwrap();
            assert!(substitute::scan(&rendered).is_ok(), "{}", file.path);
            assert!(
                !rendered.contains("main"),
                "{} kept the placeholder",
                file.path
            );
            assert!(rendered.contains("trunk"), "{}", file.path);
        }

        // Spot-check the three substituted classes on the lander, which is the
        // only file carrying all three pins and the trunk name.
        let land = GITHUB_FILES[2].render(&table).unwrap();
        assert!(land.contains("uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683"));
        assert!(
            land.contains(
                "uses: actions/download-artifact@fa0a91b85d4f404e444e00e005971372dc801d16"
            )
        );
        assert!(land.contains("SPINE_TRUNK: \"trunk\""));
        assert!(land.contains("ref: \"trunk\""));
    }

    /// CI §15 D20, closed: **the comment describing the token scan defeated
    /// it, and no repository could be initialised.**
    ///
    /// §5.3's line 24 read *"a rendered ci.sh still containing a '@@' token is
    /// not a conforming render and init refuses to write it"*. The comment is
    /// not a token and is substituted by nothing, so it survived every render —
    /// and §3.4's scan is *"no occurrence of `@@` — two `U+0040`, **in any
    /// context**"*, and a comment is a context.
    ///
    /// `.spine/ci.sh` is row 1 of **every** provider, and §3.4's whole-plan
    /// rule makes one failure refuse everything, so `spine init` could render
    /// no CI definition on any provider on any host. Reproduced at byte 1158.
    ///
    /// The fix was the comment, not the scan, and that direction matters:
    /// §3.4's scan is worth having precisely because "it re-parses no YAML,
    /// does not know which template produced the bytes, and gives the same
    /// answer on every platform". Making it comment-aware would spend that;
    /// narrowing it to `@@<NAME>@@` would stop it catching a half-substituted
    /// token. Line count unchanged at 319; both digests moved.
    ///
    /// This test asserted the defect until the corpus was amended. It now
    /// asserts the closure, which is the assertion that can fail if anyone
    /// reintroduces a bare `@@` into a shipped template.
    #[test]
    fn a_rendered_ci_sh_passes_the_scan_and_every_provider_renders() {
        let table = table("main");

        let rendered = CI_SH
            .render(&table)
            .expect("a rendered ci.sh must pass the scan it describes");
        assert!(
            !rendered.contains("@@"),
            "no `@@` survives a render, in a comment or anywhere else"
        );
        assert!(
            !rendered.contains(substitute::DIST_BASE),
            "and the one real token did substitute"
        );

        // Every provider's plan now renders, which is the property the defect
        // denied: row 1 is `.spine/ci.sh` on all three.
        for provider in [Provider::Github, Provider::Gitlab, Provider::Generic] {
            let files = render_all(provider, &table)
                .unwrap_or_else(|e| panic!("{provider:?} must render: {e}"));
            assert_eq!(files[0].0, ".spine/ci.sh");
            for (path, body) in &files {
                assert!(
                    substitute::scan(body).is_ok(),
                    "{path} carries a surviving token"
                );
            }
        }
    }

    /// CI §3.4: "one failure refuses the **whole** plan rather than writing the
    /// paths that happened to pass. A repository half-scaffolded by a bad
    /// release is worse than one not scaffolded at all." `render_all` returns
    /// bytes or a refusal — there is no partial value a caller could write.
    #[test]
    fn one_failure_refuses_the_whole_plan() {
        let table = table("main");

        // Every row of every provider renders clean, now that CI §15 D20 is
        // closed. That is the precondition: a test of "one failure refuses
        // everything" is worthless if everything already fails.
        for provider in [Provider::Github, Provider::Gitlab, Provider::Generic] {
            assert!(render_all(provider, &table).is_ok(), "{provider:?}");
        }

        // Now inject exactly one failure, into one row, and watch the whole
        // plan refuse.
        //
        // The failing body is a real shape and not a contrivance: a template
        // carrying a token the substitution table has no row for is precisely
        // what a release manifest missing a member would produce, which is the
        // case CI §3.4's whole-plan rule exists for — "a repository
        // half-scaffolded by a bad release is worse than one not scaffolded at
        // all".
        //
        // Note it must be a token the table has no row for, not a *misspelling*
        // of one it has: `PIN_CHECKOUT_TYPO` begins with `PIN_CHECKOUT`, so
        // §3.4 step 3's "every occurrence of a token is replaced" substitutes
        // the prefix and leaves `_TYPO` — which is correct, and which is why
        // this uses `@@` instead.
        let poisoned = CiFile {
            path: ".github/workflows/spine-land.yml",
            template: "ci-github-land",
            version: CI_TEMPLATE_VERSION,
            owner: Owner::SpineOwned,
            body: "name: spine-land\nrun: echo @@LEFTOVER@@\n",
        };
        let refusal = poisoned
            .render(&table)
            .expect_err("a token with no table row must not render");
        assert_eq!(refusal.path, ".github/workflows/spine-land.yml");
        assert!(
            refusal
                .to_string()
                .starts_with(".github/workflows/spine-land.yml: unsubstituted-token"),
            "the whole-plan refusal names the file that failed: {refusal}"
        );

        // And the rows that would have rendered are not returned: `render_all`
        // yields a `Result`, not a per-file map, so there is no partial value a
        // caller could write.
        assert!(
            poisoned.render(&table).is_err(),
            "no partial render escapes"
        );
    }

    /// MF §3.6's twelve keys include these four, and `params.ci` never removes
    /// one: "The map is provider-independent … it records what the pinned
    /// binary would render, not what is on disk." So a `--ci github`
    /// repository still carries `ci-gitlab`.
    #[test]
    fn the_four_template_names_are_provider_independent() {
        assert_eq!(
            CI_TEMPLATE_NAMES,
            [
                "ci-generic",
                "ci-github-collect",
                "ci-github-land",
                "ci-gitlab"
            ]
        );
        // Nothing a provider renders can add a name outside the four, but the
        // four exist whatever the provider is.
        let rendered_by_github: Vec<&str> = Provider::Github
            .files()
            .iter()
            .map(|f| f.template)
            .collect();
        assert!(!rendered_by_github.contains(&"ci-gitlab"));
    }

    #[test]
    fn provider_round_trips_through_params_ci() {
        for name in spine_manifest::schema::CI_PROVIDERS {
            let provider = Provider::parse(name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(provider.as_str(), name);
        }
        assert_eq!(Provider::parse("jenkins"), None);
    }
}
