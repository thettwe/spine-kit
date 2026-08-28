//! GR §8's published test vectors, built from the typed schema.
//!
//! The order is the corpus's own and is not negotiable: "Reproducing them is
//! mechanical and ordered: (1) canonicalize evaluation 1's value — the §8.2
//! value with `authority.reviews: []` and `gates[G2].status: "fail"` — and take
//! its SHA-256 and its length; (2) that digest is **already substituted** into
//! bob's `report=` in the §8.2 value below, so canonicalize §8.2 as printed and
//! take its SHA-256 and its length. §8.3's minimal vector is unaffected and
//! stands as published; build against it first."
//!
//! The landing is PB §5.5's canonical envelope and EV vector A: `INT-042`, team
//! mode, merge strategy, `C-A3: hostile`, `C-M4: on`, `profile=container`, one
//! reopen, one `class=tripwire` review by bob over a `G2` containment finding,
//! with the universal rule-5 `G11` advisory present because precondition 0
//! fails under `hostile`.
//!
//! **Every fabricated value here is copied from GR §8, never invented.** GR §8
//! enumerates which of its own values are fabricated-but-well-formed and which
//! are "computed, and adopted from their owners rather than invented"; this
//! file transcribes both classes and computes nothing of its own except the two
//! digests under test.

use spine_canon::ObjectFormat;
use spine_report::{
    Authority, AutoMerge, Automerge, Collector, Event, Evidence, Fingerprint, Gate, GateResult,
    GateStatus, IntentId, LandingShape, Lane, Mode, Namespace, Objects, Oid, Policy,
    PreconditionStatus, REPORT_VERSION, Report, Reverify, RuleMode, Rules, Run, SealProfile,
    Sha256Digest, Statement, Strategy, Subject, Threat, Tool, Wire, WireClass, WireKind, WireSet,
    spine_gates_value,
};

/// GR §8.1's published digest for evaluation 1. It is substituted into bob's
/// `report=` inside §8.2, so the two vectors are ordered and this constant is
/// the join.
const EVAL_1_REPORT: &str =
    "sha256:e2bd8cb5da473701bf0dd8e9aa0c6a28cc1280e33ba06758165c5089ae3f5b47";

const EVAL_2_REPORT: &str =
    "sha256:a47c13281b0b4a13085e523cd3c09e4f7d1caf0c80419de5fa3a869f7aec309e";

/// GR §8.2's `authority.reviews[0].fingerprint` — bob's key, published in
/// EV §8.1 and reproduced by `ssh-keygen -lf` (GR §8.2.1 withdrawal 4).
const BOB_FP: &str = "SHA256:V2dasTIGWUnhlaxa7vr3Qmqpe/qYBoxi+C7Cs6yxxEs";
/// alice's key, on the sign-off, the approve and the reopen.
const ALICE_FP: &str = "SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM";

/// GR §8.2's `tool.dist_hash` and `evidence.collector.dist_hash` — one value,
/// "computed with `shasum -a 256` over that document's 529-byte artifact list"
/// (`manifest.md` §8.2). Not EV §8's `41d0e9b7…` placeholder: EV §15 records
/// that divergence and the build plan's C26 rules for this one.
const DIST_HASH: &str = "sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db";

fn sha1(hex: &str) -> Oid {
    Oid::parse(hex, ObjectFormat::Sha1).expect("a published sha1 oid")
}

fn fp(s: &str) -> Fingerprint {
    Fingerprint::parse(s).expect("a published fingerprint")
}

fn stmt(line: &str, fingerprint: &str, namespace: Namespace) -> Statement {
    Statement {
        line: line.as_bytes().to_vec(),
        fingerprint: fp(fingerprint),
        namespace,
    }
}

/// Bob's `Spine-Review` line, byte-for-byte from GR §8.1, "with the wire tokens
/// in the byte order of §6.2".
fn bob_review_line() -> String {
    format!(
        "Spine-Review: INT-042 class=tripwire \
         head=77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9 \
         tree=3e91c7a2d0f46b58e19d73c0a5b284fd61e0c9a7 \
         base=7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51 \
         intent=dfb4079e22de55ec377468b9b697fdf86085ea37 \
         report={EVAL_1_REPORT} \
         wires=G11,G2:src/shared/util.ts \
         reason=\"shared helper touched outside touchpoints; read the diff and the outcomes\" \
         reviewer=bob@example.com"
    )
}

/// GR §8.2's value, parameterized by the two members GR §8.1 says the two
/// evaluations differ in and nothing else: "`gates[G2].status` becomes
/// `"override"`, and the review enters `authority.reviews`. Nothing else moves,
/// because a review commit is empty and `Hc`, `T` and `merge_base` are
/// unchanged."
fn landing(reviews: Vec<Statement>, g2: GateStatus) -> Report {
    let mut gates: Vec<GateResult> = Gate::running_on(LandingShape::GatedLand)
        .into_iter()
        .map(|g| GateResult::new(g, GateStatus::Pass))
        .collect();
    for entry in &mut gates {
        if entry.gate == Gate::G2 {
            entry.status = g2;
        }
    }

    Report {
        subject: Subject {
            lane: Lane::Gated,
            event: Event::Land,
            intent: Some(IntentId::parse("INT-042").unwrap()),
            strategy: Strategy::Merge,
        },
        objects: Objects {
            base: sha1("7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51"),
            head: sha1("77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9"),
            ref_name: b"refs/heads/intent/INT-042".to_vec(),
            merge_base: sha1("6a41d0c93bf7e25184ad0c76b3e91f52d7c40e8b"),
            tree: sha1("3e91c7a2d0f46b58e19d73c0a5b284fd61e0c9a7"),
            intent_blob: Some(sha1("dfb4079e22de55ec377468b9b697fdf86085ea37")),
        },
        tool: Tool {
            version: "1.4.0".to_owned(),
            dist_hash: Sha256Digest::parse(DIST_HASH).unwrap(),
        },
        git_version: "2.45".to_owned(),
        object_format: ObjectFormat::Sha1,
        mode: Mode::Team,
        profile: SealProfile::Container,
        policy: Policy {
            manifest: sha1("8c14a70b3d9e52f6081ac47b39d0e2f5617ab8c0"),
            keyring: sha1("0aa71c9e4d38b60f27ec5a1943d0b8e762fa4c15"),
            constitution: sha1("e9a2f0714c8b53d609af2e75b1c840d3629ea7f0"),
            ci_sh: sha1("51d9c0827a4e6b13f05d92ac7e380b4617fc25da"),
            // GR §5.4: "every entry of `C-A2` plus every value of every
            // `paths.*` key", `esc`-encoded, deduplicated and sorted ascending
            // by encoded bytes. Printed here in GR §8.2's own order, which is
            // that sorted order — the flattening that produces it is pinned
            // separately by `floor_extensions_flatten_from_c_a2_and_paths`.
            floor_extensions: vec![
                b"AGENTS.md".to_vec(),
                b"CLAUDE.md".to_vec(),
                b"CONSTITUTION.md".to_vec(),
                b"adr/".to_vec(),
                b"db/migrations/".to_vec(),
            ],
            rules: Rules {
                c_a1: RuleMode::Team,
                c_a2: vec![b"adr/".to_vec(), b"db/migrations/".to_vec()],
                c_a3: Threat::Hostile,
                c_m1: Strategy::Merge,
                c_m2: Reverify::Full,
                c_m3: 3,
                c_m4: AutoMerge::On,
                c_q1: vec![b"docs/".to_vec(), b"src/**".to_vec()],
                c_q2: 400,
                c_t1: vec![b"tests/".to_vec(), b"src/**/__tests__/".to_vec()],
                // CN §6.4's render for `params.langs: ["python", "ts"]` —
                // fifteen patterns in the constitution's own order, which
                // GR §5.4.1 forbids reordering.
                c_t2: [
                    "tests/support/**",
                    "**/conftest.py",
                    "pytest.ini",
                    "pyproject.toml",
                    "tox.ini",
                    "setup.cfg",
                    "package.json",
                    "tsconfig.json",
                    "jsconfig.json",
                    "vite.config.*",
                    "vitest.config.*",
                    "vitest.workspace.*",
                    "vitest.setup.*",
                    "jest.config.*",
                    "jest.setup.*",
                ]
                .iter()
                .map(|s| s.as_bytes().to_vec())
                .collect(),
                c_t3: true,
            },
        },
        authority: Authority {
            signoff: Some(stmt(
                "Spine-Signoff: INT-042 blob=dfb4079e22de55ec377468b9b697fdf86085ea37 \
                 template=intent@2 constitution=v3 reopens=1 signer=alice@example.com",
                ALICE_FP,
                Namespace::Signoff,
            )),
            approve: Some(stmt(
                "Spine-Approve: INT-042 intent=dfb4079e22de55ec377468b9b697fdf86085ea37 \
                 base=5f8e2a10cd47b6390e5a2c8db14f70369ba2e0d7 rounds=1 total_rounds=3 \
                 reopens=1 red=5/5 \
                 freeze=sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2 \
                 signer=alice@example.com",
                ALICE_FP,
                Namespace::Review,
            )),
            reopens: vec![stmt(
                "Spine-Reopen: INT-042 \
                 voids=sha256:4d1e0b7c9a2f83d6540e7b1c8a95f2036d4e8b71ca03f95e2b6d178c04a3e9f5 \
                 reopens=1 reason=\"AC-3 was not testable as written\" signer=alice@example.com",
                ALICE_FP,
                Namespace::Signoff,
            )],
            reviews,
            upgrade: None,
            withdraw: None,
        },
        gates,
        // GR §8's landing is `params.ci: github`, "load-bearing rather than
        // incidental": precondition 2 reads `"met"`, which after the amendment
        // of 2026-08-26 requires all three conjuncts, and GitHub is the one
        // shipped arrangement that supplies the third.
        wires: WireSet::from_raised([
            Wire::at(
                Gate::G2,
                "src/shared/util.ts",
                WireClass::Tripwire,
                WireKind::Finding,
            ),
            Wire::bare(Gate::G11, WireClass::Tripwire, WireKind::Advisory),
        ])
        .unwrap(),
        floor_hits: Vec::new(),
        automerge: Automerge {
            preconditions: [
                // `C-A3: hostile`, so precondition 0 is unmet — "the universal
                // rule-5 `G11` advisory wire present because precondition 0
                // fails under `hostile`".
                PreconditionStatus::Unmet,
                PreconditionStatus::Met,
                PreconditionStatus::Met,
                PreconditionStatus::Met,
                PreconditionStatus::Met,
            ],
        },
        evidence: Some(Evidence {
            result_sha256: Sha256Digest::parse(
                "sha256:0b93f4ac5182d67e0a4c31fb9d20e857643ca0b1f9e78d5236ca04b81e7d3f96",
            )
            .unwrap(),
            collector: Collector {
                version: "1.4.0".to_owned(),
                dist_hash: Sha256Digest::parse(DIST_HASH).unwrap(),
            },
            keys_visible: false,
            ids: 412,
        }),
        run: Run { reverifications: 0 },
    }
}

/// GR §8.1: "Evaluation 1 ends in `landing-review`: `G2` has a finding nobody
/// has accepted, so `gates[G2].status` is `"fail"` and `authority.reviews` is
/// empty."
fn evaluation_1() -> Report {
    landing(Vec::new(), GateStatus::Fail)
}

/// GR §8.2: the sealed report. "The two reports differ in exactly two members."
fn evaluation_2() -> Report {
    landing(
        vec![stmt(&bob_review_line(), BOB_FP, Namespace::Review)],
        GateStatus::Override,
    )
}

/// GR §8.3, the minimal canonicalizer vector.
///
/// Already green in `spine_canon` — asserted here because GR §8.2.1 says "Debug
/// against §8.3 first", and a regression in the canonicalizer would otherwise
/// reach this crate as an unexplained digest failure.
#[test]
fn gr_8_3_the_minimal_canonicalizer_vector_still_reproduces() {
    use spine_canon::Value;
    let value = Value::obj([
        ("b", Value::arr([Value::Int(1), Value::Int(2)])),
        ("a", Value::str("x\\y")),
        ("Z", Value::Bool(true)),
        (
            "_c",
            Value::obj([("n", Value::Int(0)), ("m", Value::str("q\"r"))]),
        ),
    ]);
    let canonical = spine_canon::canonicalize(&value);
    assert_eq!(
        String::from_utf8(canonical.clone()).unwrap(),
        r#"{"Z":true,"_c":{"m":"q\"r","n":0},"a":"x\\y","b":[1,2]}"#
    );
    assert_eq!(
        Sha256Digest::of(&canonical).as_str(),
        "sha256:a594772ccb6408158b6e76b170d5488c2454ba576e09ae379e24d743e21921f0"
    );
}

/// GR §8.1's published vector: **3476 bytes**,
/// `sha256:e2bd8cb5da473701bf0dd8e9aa0c6a28cc1280e33ba06758165c5089ae3f5b47`.
///
/// It must be reproduced first because §8.2 carries it inside bob's `report=`.
#[test]
fn gr_8_1_evaluation_one() {
    let bytes = evaluation_1().canonical_bytes();
    assert_eq!(bytes.len(), 3476, "canonical length");
    assert_eq!(Sha256Digest::of(&bytes).as_str(), EVAL_1_REPORT);
}

/// GR §8.2's published vector: **4053 bytes**,
/// `sha256:a47c13281b0b4a13085e523cd3c09e4f7d1caf0c80419de5fa3a869f7aec309e`.
#[test]
fn gr_8_2_the_sealed_report() {
    let bytes = evaluation_2().canonical_bytes();
    assert_eq!(bytes.len(), 4053, "canonical length");
    assert_eq!(Sha256Digest::of(&bytes).as_str(), EVAL_2_REPORT);
}

/// GR §8.2's `first 96 canonical bytes`, which GR §9.17 confirms is 96 and not
/// 84. "A canonicalizer that does not reproduce those 96 bytes is already wrong
/// before the digest is compared."
#[test]
fn gr_8_2_the_first_ninety_six_canonical_bytes() {
    let bytes = evaluation_2().canonical_bytes();
    let prefix = r#"{"authority":{"approve":{"fingerprint":"SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM","lin"#;
    assert_eq!(prefix.len(), 96);
    assert_eq!(&bytes[..96], prefix.as_bytes());
}

/// GR §8.1: "The two reports differ in exactly two members: `gates[G2].status`
/// becomes `"override"`, and the review enters `authority.reviews`."
///
/// Checked structurally rather than by eye: every other field is `==`, so a
/// third difference creeping into the fixture would fail here rather than
/// silently move a digest.
#[test]
fn the_two_evaluations_differ_in_exactly_two_members() {
    let mut e1 = evaluation_1();
    // Apply exactly the two-member delta GR §8.1 names.
    for g in &mut e1.gates {
        if g.gate == Gate::G2 {
            g.status = GateStatus::Override;
        }
    }
    e1.authority.reviews = vec![stmt(&bob_review_line(), BOB_FP, Namespace::Review)];
    assert_eq!(e1, evaluation_2());
}

/// GR §8.1: bob's line, "byte-for-byte, carrying that digest, with the wire
/// tokens in the byte order of §6.2".
///
/// The `wires=` half is rebuilt from the report's own array rather than
/// transcribed, so the line and the array cannot disagree — which is the defect
/// GR §9.19 records, where "an implementer reading §6.2 wrote a numeric
/// comparator, an implementer reading §5.6 wrote a byte comparator, and because
/// re-sorting changes no byte count neither could tell from any length this
/// document publishes."
#[test]
fn gr_8_1_bobs_review_line_carries_the_arrays_own_order() {
    let report = evaluation_2();
    assert_eq!(report.wires.wires_line(), "G11,G2:src/shared/util.ts");
    assert!(bob_review_line().contains(&format!("wires={}", report.wires.wires_line())));
    assert!(bob_review_line().contains(&format!("report={EVAL_1_REPORT}")));
}

/// GR §8.2.1's trap, made into a test: "Re-sorting `wires` from numeric to byte
/// order swaps two array entries and rewrites bob's `wires=` … Both are
/// permutations: evaluation 1 stays **3476** bytes and evaluation 2 stays
/// **4053**, exactly as they are under the numeric order, so *every length
/// check in this document passes under both orders and only the digests
/// separate them*."
///
/// Built here by re-serializing the report with the array reversed, which is
/// exactly what a numeric comparator would have produced over these two wires.
#[test]
fn the_numeric_wire_order_matches_every_published_length_and_no_digest() {
    use spine_canon::{Value, canonicalize};
    let report = evaluation_2();
    let mut value = report.to_value();
    let Value::Obj(members) = &mut value else {
        unreachable!()
    };
    for (name, v) in members.iter_mut() {
        if name == "wires"
            && let Value::Arr(items) = v
        {
            items.reverse();
        }
    }
    let numeric = canonicalize(&value);
    assert_eq!(
        numeric.len(),
        4053,
        "the numeric order matches the published length"
    );
    assert_ne!(
        Sha256Digest::of(&numeric).as_str(),
        EVAL_2_REPORT,
        "and reproduces no published digest"
    );
}

/// GR §8.2's published `Spine-Gates` rendering. "G6 is absent because no
/// version-1 report carries a G6 entry (§5.6.2); G10 is absent because it is
/// never in `Spine-Gates` (PB §11)."
#[test]
fn gr_8_2_spine_gates_rendering() {
    let report = evaluation_2();
    assert_eq!(
        format!("Spine-Gates: {}", spine_gates_value(&report.gates)),
        "Spine-Gates: G1=pass G2=override G3=pass G4=pass G5=pass G7=pass G8=pass \
         G9=pass G11=pass G12=pass G13=pass G14=pass G15=pass G16=pass"
    );
}

/// GR §5.6.1: "A report containing any `fail` is a non-landing report … it is
/// never the report a seal names." §8.1 is exactly that report, and §8.2 is the
/// one that sealed.
#[test]
fn evaluation_one_is_not_landable_and_evaluation_two_is() {
    assert!(!evaluation_1().is_landable());
    assert!(evaluation_2().is_landable());
}

/// Both vectors satisfy every cross-member invariant. A published value that
/// tripped one would mean the invariant was misread, not that the vector was
/// wrong.
#[test]
fn both_published_vectors_validate() {
    assert_eq!(evaluation_1().validate(), Vec::new());
    assert_eq!(evaluation_2().validate(), Vec::new());
}

/// GR §8.2: "`floor_extensions` also shows §5.4's list rule resolved:
/// `paths.constitution` contributes one entry and `paths.agent_context`'s two
/// elements contribute one each, flattened into one sorted array."
///
/// The flattening arrives in the manifest's own order and overlaps `C-A2`; the
/// serializer owns the sort and the dedup, so a caller that hands it the raw
/// union reproduces the published bytes without doing either.
#[test]
fn floor_extensions_flatten_from_c_a2_and_paths() {
    let mut report = evaluation_2();
    report.policy.floor_extensions = vec![
        // C-A2's two entries, in the constitution's order.
        b"adr/".to_vec(),
        b"db/migrations/".to_vec(),
        // paths.constitution.
        b"CONSTITUTION.md".to_vec(),
        // paths.agent_context: ["AGENTS.md", "CLAUDE.md"] — "two entries, never
        // one stringified list" (GR §5.4).
        b"AGENTS.md".to_vec(),
        b"CLAUDE.md".to_vec(),
        // A duplicate, which C-A2 and a paths.* key can legitimately produce.
        b"adr/".to_vec(),
    ];
    let bytes = report.canonical_bytes();
    assert_eq!(bytes.len(), 4053);
    assert_eq!(Sha256Digest::of(&bytes).as_str(), EVAL_2_REPORT);
}

/// GR §3.2's closed schema, exercised on the one document whose exact bytes are
/// published: parse §8.2 back and re-serialize to the same 4053 bytes.
///
/// This is the property `--verify` rests on — "a tolerant reader and a strict
/// one compute different digests over the same document".
#[test]
fn gr_8_2_round_trips_through_the_reader() {
    let bytes = evaluation_2().canonical_bytes();
    let parsed = Report::from_canonical(&bytes).expect("the sealed report parses");
    assert_eq!(parsed, evaluation_2());
    let reserialized = parsed.canonical_bytes();
    assert_eq!(reserialized, bytes);
    assert_eq!(Sha256Digest::of(&reserialized).as_str(), EVAL_2_REPORT);
}

/// GR §3.2: "A reader that meets an unknown **member name** inside a version it
/// does know refuses the same way" — `report-version-unknown`.
#[test]
fn an_unknown_member_is_report_version_unknown() {
    use spine_canon::Value;
    let mut value = evaluation_2().to_value();
    let Value::Obj(members) = &mut value else {
        unreachable!()
    };
    members.push(("published".to_owned(), Value::Bool(true)));
    let err = Report::from_value(&value).unwrap_err();
    assert_eq!(err.to_string(), "report-version-unknown");
}

/// The same rule one level down, where the reader was tolerant: GR §5.5 puts
/// `self_approved` on `reviews[]` and on nothing else, so on a sign-off it is
/// an unknown member.
///
/// Accepted and discarded, it round-tripped the report to different bytes —
/// which is `--verify` reporting `report-mismatch` against a sound landing.
#[test]
fn self_approved_on_a_sign_off_is_an_unknown_member() {
    use spine_canon::Value;
    let mut value = evaluation_2().to_value();
    let Value::Obj(members) = &mut value else {
        unreachable!()
    };
    for (name, v) in members.iter_mut() {
        if name == "authority"
            && let Value::Obj(authority) = v
        {
            for (member, statement) in authority.iter_mut() {
                if member == "signoff"
                    && let Value::Obj(fields) = statement
                {
                    fields.push(("self_approved".to_owned(), Value::Bool(false)));
                }
            }
        }
    }
    let err = Report::from_value(&value).unwrap_err();
    assert_eq!(err.to_string(), "report-version-unknown");
}

/// GR §5.6: `gates[]` "sorts by gate number ascending". The reader refuses
/// rather than sorting, because the serializer *would* sort: a tolerant read
/// followed by a write is a different digest over the same document.
#[test]
fn an_unsorted_gates_array_does_not_parse() {
    use spine_canon::Value;
    let mut value = evaluation_2().to_value();
    let Value::Obj(members) = &mut value else {
        unreachable!()
    };
    for (name, v) in members.iter_mut() {
        if name == "gates"
            && let Value::Arr(gates) = v
        {
            gates.reverse();
        }
    }
    assert!(Report::from_value(&value).is_err());
}

/// GR §3.2: "A reader that does not know a report's `report_version`
/// **refuses**."
#[test]
fn an_unknown_report_version_is_refused() {
    use spine_canon::Value;
    let mut value = evaluation_2().to_value();
    let Value::Obj(members) = &mut value else {
        unreachable!()
    };
    for (name, v) in members.iter_mut() {
        if name == "report_version" {
            *v = Value::Int(2);
        }
    }
    let err = Report::from_value(&value).unwrap_err();
    assert_eq!(err.to_string(), "report-version-unknown");
    assert_eq!(REPORT_VERSION, 1);
}

/// GR §4.4.1: "The bytes JCS produces from this value are also what §4.4
/// publishes. The note on this landing's commit holds them exactly — no
/// newline, no framing, no pretty form."
///
/// GR §4.4.2 makes `-m`, `-F` and the editor paths "**non-conforming**: git
/// terminates a note message with a newline, and a note carrying one trailing
/// `0x0A` hashes to something that is not `report=`." Computed here rather than
/// asserted: the digest of the bytes plus one LF is a different value.
#[test]
fn a_note_written_from_a_message_would_not_hash_to_report() {
    let bytes = evaluation_2().canonical_bytes();
    assert_eq!(Sha256Digest::of(&bytes).as_str(), EVAL_2_REPORT);
    let mut with_newline = bytes;
    with_newline.push(b'\n');
    assert_ne!(Sha256Digest::of(&with_newline).as_str(), EVAL_2_REPORT);
}

/// GR §4.3 end to end over the published vector: the sealed report verifies
/// against itself, and a candidate that is not it stops at step 3.
#[test]
fn gr_4_3_verifies_the_published_report_against_its_own_seal() {
    use spine_report::{Preconditions, VerifyStatus, verify};

    let sealed = Sha256Digest::parse(EVAL_2_REPORT).unwrap();
    let dist = Sha256Digest::parse(DIST_HASH).unwrap();
    let bytes = evaluation_2().canonical_bytes();
    let pre = Preconditions {
        seal_dist_hash: &dist,
        seal_git_version: "2.45",
        seal_report: &sealed,
        running_dist_hash: &dist,
        running_git_version: "2.45",
        head_reachable: true,
    };

    // Step 6 with a rebuild that reproduces the same objects.
    let out = verify(&pre, Some(&bytes), |r| r.clone());
    assert_eq!(out.status, VerifyStatus::Verified);
    assert_eq!(out.status.exit_code(), 0);

    // A rebuild that disagrees in one member: `report-mismatch`, exit 1, with
    // the recomputed bytes to print.
    let out = verify(&pre, Some(&bytes), |r| {
        let mut r = r.clone();
        r.run.reverifications = 1;
        r
    });
    assert!(matches!(out.status, VerifyStatus::ReportMismatch { .. }));
    assert_eq!(out.status.exit_code(), 1);
    assert!(out.recomputed.is_some());

    // Evaluation 1 is a real report and is not the one this seal names.
    let out = verify(&pre, Some(&evaluation_1().canonical_bytes()), |r| r.clone());
    assert!(matches!(out.status, VerifyStatus::CandidateMismatch { .. }));
    assert_eq!(out.status.exit_code(), 1);
}

/// GR §4.2: exit 4 is a property of the objects. This landing is
/// `strategy=merge`, but the predicate is `needs_head() && !head_reachable`,
/// and a `land` whose `Hc` the CAS deleted reaches it whatever the strategy
/// records.
#[test]
fn a_land_whose_head_is_unreachable_is_not_recomputable() {
    use spine_report::{Preconditions, VerifyStatus, verify};

    let sealed = Sha256Digest::parse(EVAL_2_REPORT).unwrap();
    let dist = Sha256Digest::parse(DIST_HASH).unwrap();
    let bytes = evaluation_2().canonical_bytes();
    let pre = Preconditions {
        seal_dist_hash: &dist,
        seal_git_version: "2.45",
        seal_report: &sealed,
        running_dist_hash: &dist,
        running_git_version: "2.45",
        head_reachable: false,
    };
    let out = verify(&pre, Some(&bytes), |_| {
        unreachable!("no rebuild is attempted")
    });
    assert_eq!(out.status, VerifyStatus::NotRecomputable);
    assert_eq!(out.status.exit_code(), 4);
}
