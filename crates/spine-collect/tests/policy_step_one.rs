//! RF §7.1 step 1, over a real trunk manifest.
//!
//! "Read policy from `origin/<trunk>`, never from the checkout: `cli.version`,
//! `cli.dist_hash`, `params.isolation`, **`params.langs`**, **`params.timeout`**,
//! `object_format` (§7.4 rule 1). `params.isolation` absent means `none`
//! (§6.7); `params.timeout` absent means `1800`."
//!
//! The manifest is `manifest.md` §8.3's published bytes, joined from that
//! document's own fenced block ("Line-broken here for reading; **the file is
//! one line plus one LF**"). Its `params` are `isolation: "container"`,
//! `langs: ["python"]`, `timeout: 1800`, which is the shape every case below
//! mutates one member of.

use spine_canon::ObjectFormat;
use spine_collect::collector::{DEFAULT_TIMEOUT_SECS, Mode, Policy, Refusal};
use spine_manifest::{Isolation, Manifest};

const VECTOR: &[u8] = include_bytes!("vectors/mf-8.3-manifest.json");

fn trunk(bytes: &[u8]) -> Manifest {
    Manifest::parse(bytes, Some(ObjectFormat::Sha1)).expect("MF §8.3 is a conforming manifest")
}

/// Mutate one member of the published bytes, keeping canonical form: every
/// replacement below preserves member order and produces a legal value, so the
/// manifest is still a manifest and only the policy read moves.
fn mutated(from: &str, to: &str) -> Vec<u8> {
    let text = core::str::from_utf8(VECTOR).expect("UTF-8");
    assert!(text.contains(from), "fixture no longer carries `{from}`");
    text.replace(from, to).into_bytes()
}

#[test]
fn step_one_reads_the_six_values_from_trunk() {
    let policy = Policy::read(&trunk(VECTOR), Mode::Ci).expect("conforming policy");
    assert_eq!(policy.cli_version, "1.4.0");
    // MF §3 stores it in PB §11's `sha256:<hex>` form for a non-git artifact,
    // which is one prefix more than RF §8.3 step 2's construction recipe
    // assumes. `Policy` keeps the manifest's bytes and
    // `expected_tool_token` reconciles the two.
    assert_eq!(
        policy.dist_hash,
        "sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db"
    );
    assert_eq!(policy.isolation, Isolation::Container);
    assert_eq!(policy.langs, vec!["python".to_string()]);
    assert_eq!(policy.timeout_secs, 1800);
    assert_eq!(policy.object_format, ObjectFormat::Sha1);

    // RF §4.2, §8.3 step 2: the trusted stage "constructs the expected token —
    // `<cli.version>` `+sha256:` `<cli.dist_hash>` — from trunk's manifest and
    // compares it to the header's `tool=` **as bytes over the whole token**".
    assert_eq!(
        policy.expected_tool_token(),
        "1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db"
    );
}

/// RF §7.1: "`params.timeout` absent means `1800`" — and RF §7.1 *The deadline*
/// adds why the absence cannot mean *no deadline*: "the field's absence selects
/// the default, never the absence of the control."
#[test]
fn an_absent_timeout_selects_the_default_and_never_the_absence_of_the_control() {
    let bytes = mutated(r#""timeout":1800,"#, "");
    let policy = Policy::read(&trunk(&bytes), Mode::Ci).expect("absent timeout is legal");
    assert_eq!(policy.timeout_secs, DEFAULT_TIMEOUT_SECS);
    assert_eq!(policy.timeout_secs, 1800);
}

/// RF §7.1 *The deadline*: "It is present and not a positive integer: the
/// collector fails the job and writes nothing (step 1's shape)." RF §13 R24
/// refuses `0` in particular, because it "would spell 'no deadline', which §6.7
/// forbids".
///
/// The refusal is checked twice on this path and the test says so rather than
/// pretending otherwise: MF §3.11's own `timeout-out-of-range` already refuses
/// the manifest, so a conforming trunk can never hand the collector a `0`. RF
/// §7.1 nonetheless makes it the collector's check as well, because the
/// collector's obligation is to the *value it read*, not to whichever release
/// gated the manifest that carries it.
#[test]
fn a_zero_timeout_refuses_the_job_rather_than_disabling_the_deadline() {
    let bytes = mutated(r#""timeout":1800"#, r#""timeout":0"#);
    let refused = Manifest::parse(&bytes, Some(ObjectFormat::Sha1))
        .expect_err("MF §3.11 refuses a zero timeout before the collector sees it");
    assert_eq!(refused.status.token(), "timeout-out-of-range");

    // And the collector's own check, reached directly.
    //
    // The second half of this test used to build a `Policy` struct literal with
    // `timeout_secs: 0`, assert that field equalled 0, and compare a
    // `&'static str` to itself — asserting coverage it did not have, over a
    // branch nothing in the suite executed. `deadline_from_secs` exists so the
    // branch is reachable, and these are the assertions the old ones claimed.
    assert_eq!(
        spine_collect::collector::deadline_from_secs(0),
        Err(Refusal::TimeoutOutOfRange),
        "a zero deadline is a refusal, never an absent deadline"
    );
    assert!(spine_collect::collector::deadline_from_secs(1).is_ok());
    assert!(spine_collect::collector::deadline_from_secs(86_400).is_ok());
    assert_eq!(
        spine_collect::collector::deadline_from_secs(86_401),
        Err(Refusal::TimeoutOutOfRange),
        "and MF §3.3's upper bound is the collector's too"
    );
}

/// RF §7.1 disposition 1: "`params.isolation: "uid"` … The collector
/// **refuses**: it fails the job and writes nothing, at **step 1**, where the
/// value is read and before `T` exists … **It is never a downgrade to `none`.**"
#[test]
fn a_uid_isolation_request_refuses_under_ci_and_is_never_downgraded_to_none() {
    let bytes = mutated(r#""isolation":"container""#, r#""isolation":"uid""#);
    assert_eq!(
        Policy::read(&trunk(&bytes), Mode::Ci),
        Err(Refusal::IsolationUnsupported)
    );
}

/// RF §7.4: outside `--ci` the solo collector "attempts nothing, it **refuses
/// nothing** — a manifest declaring `uid` costs a solo developer no run, and
/// disposition 1 of §7.1 is a `--ci` rule — and it writes `none`."
#[test]
fn the_solo_path_refuses_nothing_for_a_uid_manifest() {
    let bytes = mutated(r#""isolation":"container""#, r#""isolation":"uid""#);
    let policy = Policy::read(&trunk(&bytes), Mode::Solo).expect("solo refuses nothing");
    // The *request* is still `uid` — the collector does not rewrite policy. It
    // simply attempts no boundary, and `collect` writes `profile=none`.
    assert_eq!(policy.isolation, Isolation::Uid);
}

/// RF §7.4: "`params.langs` and `params.timeout` are read from trunk on the
/// solo path too, exactly as in `--ci` (§7.4 rule 1). A solo developer's laptop
/// does not choose its own invocation set or its own deadline."
#[test]
fn the_solo_path_reads_langs_and_timeout_from_trunk_like_ci_does() {
    let ci = Policy::read(&trunk(VECTOR), Mode::Ci).expect("conforming");
    let solo = Policy::read(&trunk(VECTOR), Mode::Solo).expect("conforming");
    assert_eq!(ci.langs, solo.langs);
    assert_eq!(ci.timeout_secs, solo.timeout_secs);
}

/// RF §7.1 step 1: "`params.isolation` absent means `none` (§6.7)." MF's own
/// reading is the same and is fail-closed — a manifest written before the field
/// existed fails auto-merge precondition 1 rather than passing it by silence.
#[test]
fn an_absent_isolation_means_none() {
    let bytes = mutated(r#""isolation":"container","#, "");
    let policy = Policy::read(&trunk(&bytes), Mode::Ci).expect("absent isolation is legal");
    assert_eq!(policy.isolation, Isolation::None);
}
