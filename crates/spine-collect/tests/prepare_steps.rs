//! RF §7.1 steps 1-5, in order, and every refusal each one has.
//!
//! The trunk manifest is `manifest.md` §8.3's published bytes — `isolation:
//! "container"`, `langs: ["python"]`, `timeout: 1800` — the same fixture step
//! 1's own test reads, mutated one member at a time.
//!
//! **Every case here refuses before a file exists.** RF §7.3 draws that line:
//! "The collector always writes a file once `T` is known and policy has been
//! read." Everything below is on the other side of it, which is why the
//! outcomes are refusals rather than statuses.

use spine_collect::collector::{Mode, Refusal, Release};
use spine_collect::keys::{KeyMaterial, Probe as KeyProbe};
use spine_collect::prepare::{
    Collector, Git, MANIFEST_PATH, PrepareError, Refs, SelfBytes, SelfIdentity, Subject, prepare,
    subject_of,
};
use spine_collect::record::RunnerToken;

/// A step-4 probe that found the operator's ssh-agent — RF §4.2's solo answer.
fn keys_reachable() -> KeyProbe {
    KeyProbe {
        reachable: vec![KeyMaterial::SshAgent],
    }
}

const VECTOR: &[u8] = include_bytes!("vectors/mf-8.3-manifest.json");
const TRUNK: &str = "origin/main";
const BASE: &str = "5f8e2a10cd47b6390e5a2c8db14f70369ba2e0d7";
const HEAD_REF: &str = "HEAD";
const HEAD: &str = "77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9";
const TREE: &str = "3e91c7a2d0f46b58e19d73c0a5b284fd61e0c9a7";

/// A repository reduced to the three reads steps 1 and 5 perform.
struct Fake {
    trunk_resolves: bool,
    head_resolves: bool,
    manifest: Option<Vec<u8>>,
    merge_conflicts: bool,
    /// `(sha, message)` first-parent, newest first — the walk a reseal's
    /// `base=` is found on.
    messages: Vec<(String, String)>,
    /// A manifest served **only** at the reseal's base, so a policy read that
    /// went to trunk instead finds nothing.
    manifest_at_reseal_base: Option<Vec<u8>>,
}

impl Default for Fake {
    fn default() -> Self {
        Fake {
            trunk_resolves: true,
            head_resolves: true,
            manifest: Some(VECTOR.to_vec()),
            merge_conflicts: false,
            messages: Vec::new(),
            manifest_at_reseal_base: None,
        }
    }
}

impl Git for Fake {
    fn rev_parse(&self, rev: &str) -> Option<String> {
        match rev {
            TRUNK => self.trunk_resolves.then(|| BASE.to_string()),
            HEAD_REF => self.head_resolves.then(|| HEAD.to_string()),
            // A reseal branch resolves to its orphan tip: PB §5.5 puts the
            // review commits on `quick/reseal-<O>`, and they change no tree.
            r if r.starts_with("quick/reseal-") => Some(ORPHAN.to_string()),
            // Anything already an oid resolves to itself.
            r if r.len() == 40 && r.bytes().all(|b| b.is_ascii_hexdigit()) => Some(r.to_string()),
            _ => None,
        }
    }

    fn blob_at(&self, rev: &str, path: &str) -> Option<Vec<u8>> {
        if path != MANIFEST_PATH {
            return None;
        }
        // Reading policy from anything but the commit policy is *supposed* to
        // come from is the failure PB §7.4 rule 1 exists to prevent, so the
        // fake serves it at exactly one commit and nothing else.
        if let Some(bytes) = &self.manifest_at_reseal_base
            && rev == LAST_LANDING
        {
            return Some(bytes.clone());
        }
        (rev == BASE).then(|| self.manifest.clone()).flatten()
    }

    fn first_parent_messages(&self, _rev: &str) -> Vec<(String, String)> {
        self.messages.clone()
    }

    fn merge_tree(&self, base: &str, head: &str) -> Option<String> {
        // Step 5 merges onto whichever commit policy came from — trunk's tip
        // ordinarily, and a reseal's `base=` on `quick/reseal-<O>`, where
        // RF §8.6 makes `merge-tree(base, O) = tree(O)` because `base=` is an
        // ancestor of `O`.
        assert!(
            base == BASE || base == LAST_LANDING,
            "step 5 merges onto the commit policy came from, got {base}"
        );
        assert!(
            head == HEAD || head == ORPHAN,
            "step 5 merges the candidate's head, got {head}"
        );
        (!self.merge_conflicts).then(|| TREE.to_string())
    }
}

/// A release with a python adapter and nothing else.
struct Shipped;

impl Release for Shipped {
    fn adapters_for(&self, lang: &str) -> Option<Vec<RunnerToken>> {
        match lang {
            "python" => Some(vec![RunnerToken::new("pytest").expect("a legal token")]),
            _ => None,
        }
    }
}

fn me() -> SelfIdentity {
    SelfIdentity {
        version: "1.4.0".into(),
        dist_hash: "sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db".into(),
    }
}

fn refs() -> Refs<'static> {
    Refs {
        trunk: TRUNK,
        head: HEAD_REF,
    }
}

fn run(git: &Fake, self_bytes: SelfBytes) -> Result<spine_collect::Prepared, PrepareError> {
    let identity = me();
    prepare(
        git,
        refs(),
        &Collector {
            mode: Mode::Ci,
            self_bytes,
            keys: keys_reachable(),
            identity: &identity,
        },
        &Shipped,
    )
}

#[test]
fn the_five_steps_produce_the_run_the_header_is_written_from() {
    let prepared = run(&Fake::default(), SelfBytes::Verified).expect("a conforming preamble");

    // Step 1's six values, read from trunk.
    assert_eq!(prepared.policy.langs, ["python"]);
    assert_eq!(prepared.policy.timeout_secs, 1800);

    // Step 3's set.
    assert_eq!(
        prepared
            .invocation
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<_>>(),
        ["pytest"]
    );

    // Step 5's tree, and the base it merged onto.
    assert_eq!(prepared.run.tree, TREE);
    assert_eq!(prepared.run.base, BASE);

    // RF §4.2: "The collector writes what it **is**, never what trunk pins."
    // Here the two agree, so the assertion that matters is the *shape* — one
    // `sha256:` prefix, not the two a literal reading of RF §8.3 step 2 gives.
    assert_eq!(
        prepared.run.tool,
        "1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db"
    );
}

/// RF §4.2 again, in the case that distinguishes the two: a collector older
/// than the pin writes **its own** version, and the trusted stage's §8.3 step 2
/// is what notices.
#[test]
fn the_tool_token_is_the_collectors_own_and_never_trunks() {
    let older = SelfIdentity {
        version: "1.3.0".into(),
        dist_hash: "0000000000000000000000000000000000000000000000000000000000000000".into(),
    };
    let prepared = prepare(
        &Fake::default(),
        refs(),
        &Collector {
            mode: Mode::Ci,
            self_bytes: SelfBytes::Verified,
            keys: keys_reachable(),
            identity: &older,
        },
        &Shipped,
    )
    .expect("an old collector still runs; the trusted stage decides");
    assert!(prepared.run.tool.starts_with("1.3.0+sha256:"));
    assert_ne!(prepared.run.tool, prepared.policy.expected_tool_token());
}

/// Step 2: "Mismatch: fail the job, write nothing."
#[test]
fn a_collector_whose_own_bytes_do_not_verify_refuses() {
    assert_eq!(
        run(&Fake::default(), SelfBytes::Mismatch),
        Err(PrepareError::Refused(Refusal::ToolBytesMismatch))
    );
}

/// Step 3: "A language in `params.langs` the running release supports no
/// adapter for: fail the job, write nothing."
///
/// The language is `dart`, not a made-up one: MF §3's `V1_LANGS` is closed, so
/// a manifest naming a language no *document* knows is `langs-unknown` at
/// parse time and never reaches step 3. Step 3's refusal is for the gap
/// between what the schema admits and what the running release ships an
/// adapter for — the case of a repository declaring `dart` to a release built
/// without the dart adapter.
#[test]
fn a_declared_language_with_no_adapter_refuses() {
    let text = core::str::from_utf8(VECTOR).unwrap();
    let mutated = text.replace(r#""langs":["python"]"#, r#""langs":["dart","python"]"#);
    assert_ne!(
        mutated.as_bytes(),
        VECTOR,
        "fixture no longer carries langs"
    );
    let git = Fake {
        manifest: Some(mutated.into_bytes()),
        ..Default::default()
    };
    assert_eq!(
        run(&git, SelfBytes::Verified),
        Err(PrepareError::Refused(Refusal::LanguageWithoutAdapter(
            "dart".into()
        )))
    );
}

/// Step 5: "A conflict yields no `T` and therefore no file."
#[test]
fn a_conflicting_merge_refuses_and_writes_nothing() {
    let git = Fake {
        merge_conflicts: true,
        ..Default::default()
    };
    assert_eq!(
        run(&git, SelfBytes::Verified),
        Err(PrepareError::Refused(Refusal::MergeConflict))
    );
}

/// **The order is the test.** A collector whose own bytes are wrong must not
/// reach step 3 and pronounce on which languages it supports, and nothing at
/// all may reach step 5, which is the only step that writes an object.
#[test]
fn the_steps_refuse_in_order() {
    let text = core::str::from_utf8(VECTOR).unwrap();
    let mutated = text.replace(r#""langs":["python"]"#, r#""langs":["dart"]"#);
    let git = Fake {
        manifest: Some(mutated.into_bytes()),
        merge_conflicts: true,
        ..Default::default()
    };

    // Steps 2, 3 and 5 all fail. Step 2 answers.
    assert_eq!(
        run(&git, SelfBytes::Mismatch),
        Err(PrepareError::Refused(Refusal::ToolBytesMismatch))
    );
    // With step 2 clean, step 3 answers — not step 5, whose fake would panic
    // if `merge_tree` were reached with the wrong arguments and would return a
    // conflict if it were reached at all.
    assert_eq!(
        run(&git, SelfBytes::Verified),
        Err(PrepareError::Refused(Refusal::LanguageWithoutAdapter(
            "dart".into()
        )))
    );
}

/// Not RF's refusals, and named apart from them: a repository with no trunk,
/// no head, or no manifest on trunk has no policy to read, so there is no step
/// 1 to refuse at.
#[test]
fn a_repository_without_policy_is_a_different_refusal() {
    assert_eq!(
        run(
            &Fake {
                trunk_resolves: false,
                ..Default::default()
            },
            SelfBytes::Verified
        ),
        Err(PrepareError::TrunkUnresolvable(TRUNK.into()))
    );
    assert_eq!(
        run(
            &Fake {
                head_resolves: false,
                ..Default::default()
            },
            SelfBytes::Verified
        ),
        Err(PrepareError::HeadUnresolvable(HEAD_REF.into()))
    );
    assert_eq!(
        run(
            &Fake {
                manifest: None,
                ..Default::default()
            },
            SelfBytes::Verified
        ),
        Err(PrepareError::NoManifestOnTrunk)
    );
}

/// RF §7.1 disposition 1, reached through the preamble rather than through
/// `Policy::read` directly: `uid` is a refusal under `--ci` and `none` outside
/// it, and the difference is the mode the caller passes.
#[test]
fn a_uid_request_refuses_under_ci_and_not_outside_it() {
    let text = core::str::from_utf8(VECTOR).unwrap();
    let mutated = text.replace(r#""isolation":"container""#, r#""isolation":"uid""#);
    assert_ne!(
        mutated.as_bytes(),
        VECTOR,
        "fixture no longer carries isolation"
    );
    let git = Fake {
        manifest: Some(mutated.into_bytes()),
        ..Default::default()
    };
    assert_eq!(
        run(&git, SelfBytes::Verified),
        Err(PrepareError::Refused(Refusal::IsolationUnsupported))
    );
    let identity = me();
    assert!(
        prepare(
            &git,
            refs(),
            &Collector {
                mode: Mode::Solo,
                self_bytes: SelfBytes::Verified,
                keys: keys_reachable(),
                identity: &identity,
            },
            &Shipped,
        )
        .is_ok(),
        "a manifest declaring uid costs a solo developer no run"
    );
}

// ---------------------------------------------------------------------------
// The reseal, where policy is not trunk's.
// ---------------------------------------------------------------------------

const ORPHAN: &str = "aa11bb22cc33dd44ee55ff6677889900aabbccdd";
const LAST_LANDING: &str = "9911772255338844bb00cc11dd22ee33ff445566";

/// CI §6.4's router names the orphan in the ref, and matches `quick/reseal-*`
/// **before** `quick/*`: "a router that matches `quick/*` first would land a
/// reseal as an ordinary quick-lane change".
#[test]
fn the_ref_says_whether_this_is_a_reseal() {
    assert_eq!(
        subject_of(&format!("refs/heads/quick/reseal-{ORPHAN}")),
        Subject::Reseal { orphan: ORPHAN }
    );
    // Unqualified, as `.spine/ci.sh` passes it.
    assert_eq!(
        subject_of(&format!("quick/reseal-{ORPHAN}")),
        Subject::Reseal { orphan: ORPHAN }
    );
    // Every other shape reads policy from trunk.
    for ordinary in [
        "refs/heads/main",
        "refs/heads/quick/typo",
        "refs/heads/intent/INT-42",
        // A branch merely *named* like one, with nothing after the dash, is
        // not a reseal: there is no orphan for `base=` to be found below.
        "refs/heads/quick/reseal-",
    ] {
        assert_eq!(subject_of(ordinary), Subject::Trunk, "{ordinary}");
    }
}

/// RF §4.2: "for a reseal, the seal's `base=`, from which **every** policy read
/// for a reseal is taken", and RF §8.6: "**`params.langs` and `params.timeout`
/// included**".
///
/// The failure this prevents is not a wrong field. On `quick/reseal-<O>`
/// trunk's tip *is* the orphan — G9 refuses every landing above one until the
/// reseal lands — so a collector reading `origin/<trunk>` seals `base=<O>`,
/// the trusted stage answers `base-moved`, and the shape can never clear it:
/// the tree must equal `O`'s so there is no candidate to fix, a reseal is not
/// promotable by `spine new --from`, and trunk cannot move while the orphan
/// stands. The repository is bricked.
#[test]
fn a_reseal_reads_policy_from_the_last_landing_below_the_orphan() {
    let head_ref = format!("quick/reseal-{ORPHAN}");
    // A manifest that exists **only** at the last landing, and whose
    // `params.timeout` differs from trunk's: RF §8.6 says a reseal reads
    // "`params.langs` and `params.timeout` included" from there, so a collector
    // that went to trunk finds no manifest at all.
    let text = core::str::from_utf8(VECTOR).unwrap();
    let at_base = text.replace(r#""timeout":1800"#, r#""timeout":900"#);
    assert_ne!(
        at_base.as_bytes(),
        VECTOR,
        "fixture no longer carries timeout"
    );
    let git = Fake {
        manifest: None,
        manifest_at_reseal_base: Some(at_base.into_bytes()),
        messages: vec![
            (ORPHAN.into(), "a push around the pipeline\n".into()),
            (
                LAST_LANDING.into(),
                format!("feat: the last real landing\n\nSpine-Seal: quick base={BASE} head=x\n"),
            ),
        ],
        ..Default::default()
    };

    let identity = me();
    let prepared = prepare(
        &git,
        Refs {
            trunk: TRUNK,
            head: &head_ref,
        },
        &Collector {
            mode: Mode::Ci,
            self_bytes: SelfBytes::Verified,
            keys: keys_reachable(),
            identity: &identity,
        },
        &Shipped,
    )
    .expect("a reseal prepares");

    assert_eq!(
        prepared.run.base, LAST_LANDING,
        "the last valid landing below the range, never trunk's tip"
    );
    assert_eq!(
        prepared.policy.timeout_secs, 900,
        "every policy read for a reseal is from base=, params.timeout included"
    );
}

/// The orphan itself is "neither a landing nor the trust root", so the walk for
/// `base=` starts *below* it — an orphan carrying its own `Spine-Seal` line
/// must not be mistaken for the landing below the range.
#[test]
fn the_orphan_is_not_its_own_base() {
    let head_ref = format!("quick/reseal-{ORPHAN}");
    let git = Fake {
        manifest: None,
        manifest_at_reseal_base: Some(VECTOR.to_vec()),
        messages: vec![
            (
                ORPHAN.into(),
                "pushed around the pipeline\n\nSpine-Seal: quick base=zz head=zz\n".into(),
            ),
            (
                LAST_LANDING.into(),
                format!("feat: real\n\nSpine-Seal: quick base={BASE} head=x\n"),
            ),
        ],
        ..Default::default()
    };
    let identity = me();
    let prepared = prepare(
        &git,
        Refs {
            trunk: TRUNK,
            head: &head_ref,
        },
        &Collector {
            mode: Mode::Ci,
            self_bytes: SelfBytes::Verified,
            keys: keys_reachable(),
            identity: &identity,
        },
        &Shipped,
    )
    .expect("a reseal prepares");
    assert_eq!(prepared.run.base, LAST_LANDING);
}

/// No `Spine-Seal` anywhere below the orphan: there is nothing for a reseal's
/// `base=` to name, and refusing beats writing a file the trusted stage will
/// answer `base-moved` to forever.
#[test]
fn a_reseal_with_no_landing_below_the_orphan_refuses() {
    let head_ref = format!("quick/reseal-{ORPHAN}");
    let git = Fake {
        messages: vec![
            (ORPHAN.into(), "orphan\n".into()),
            (
                "1111111111111111111111111111111111111111".into(),
                "seed\n".into(),
            ),
        ],
        ..Default::default()
    };
    let identity = me();
    assert_eq!(
        prepare(
            &git,
            Refs {
                trunk: TRUNK,
                head: &head_ref,
            },
            &Collector {
                mode: Mode::Ci,
                self_bytes: SelfBytes::Verified,
                keys: keys_reachable(),
                identity: &identity,
            },
            &Shipped,
        ),
        Err(PrepareError::NoLandingBelowTheOrphan)
    );
}
