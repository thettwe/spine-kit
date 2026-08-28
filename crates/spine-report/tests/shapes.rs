//! The three landing shapes GR §8 does not publish a vector for, and the one
//! path encoding no published vector exercises.
//!
//! GR §8's landing is a gated `land` with an empty `floor_hits` and a path
//! containing none of `,`, ` ` or `"`. So every rule that only fires on a
//! tombstone, a reseal, or a path with a comma in it is unmeasured by the
//! digests — which is precisely the shape of defect GR §8.2.1 warns about, one
//! that "no published byte count localises".

use spine_canon::{ObjectFormat, canonicalize_to_string};
use spine_report::{
    Authority, AutoMerge, Automerge, Collector, Event, Evidence, Fingerprint, Gate, GateResult,
    GateStatus, IntentId, Invariant, LandingShape, Lane, Mode, Namespace, Objects, Oid, Policy,
    PreconditionStatus, Report, Reverify, RuleMode, Rules, Run, SealProfile, Sha256Digest,
    Statement, Strategy, Subject, Threat, Tool, Wire, WireClass, WireKind, WireSet, rule_five_wire,
    spine_gates_value,
};

const FP: &str = "SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM";
const DIST: &str = "sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db";

fn sha1(hex: &str) -> Oid {
    Oid::parse(hex, ObjectFormat::Sha1).unwrap()
}

fn digest(hex: &str) -> Sha256Digest {
    Sha256Digest::parse(hex).unwrap()
}

fn stmt(line: &str, ns: Namespace) -> Statement {
    Statement {
        line: line.as_bytes().to_vec(),
        fingerprint: Fingerprint::parse(FP).unwrap(),
        namespace: ns,
    }
}

fn rules(c_m4: AutoMerge) -> Rules {
    Rules {
        c_a1: RuleMode::Team,
        c_a2: vec![b"adr/".to_vec()],
        c_a3: Threat::Hostile,
        c_m1: Strategy::Merge,
        c_m2: Reverify::Full,
        c_m3: 3,
        c_m4,
        c_q1: vec![b"docs/".to_vec()],
        c_q2: 400,
        c_t1: vec![b"tests/".to_vec()],
        c_t2: vec![b"tests/support/**".to_vec()],
        c_t3: true,
    }
}

fn policy(c_m4: AutoMerge) -> Policy {
    Policy {
        manifest: sha1("8c14a70b3d9e52f6081ac47b39d0e2f5617ab8c0"),
        keyring: sha1("0aa71c9e4d38b60f27ec5a1943d0b8e762fa4c15"),
        constitution: sha1("e9a2f0714c8b53d609af2e75b1c840d3629ea7f0"),
        ci_sh: sha1("51d9c0827a4e6b13f05d92ac7e380b4617fc25da"),
        floor_extensions: vec![b"adr/".to_vec()],
        rules: rules(c_m4),
    }
}

fn objects(intent_blob: Option<Oid>) -> Objects {
    Objects {
        base: sha1("7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51"),
        head: sha1("77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9"),
        ref_name: b"refs/heads/quick/reseal-77aa3c19".to_vec(),
        merge_base: sha1("6a41d0c93bf7e25184ad0c76b3e91f52d7c40e8b"),
        tree: sha1("3e91c7a2d0f46b58e19d73c0a5b284fd61e0c9a7"),
        intent_blob,
    }
}

fn tool() -> Tool {
    Tool {
        version: "1.4.0".to_owned(),
        dist_hash: digest(DIST),
    }
}

fn all_pass(shape: LandingShape) -> Vec<GateResult> {
    Gate::running_on(shape)
        .into_iter()
        .map(|g| GateResult::new(g, GateStatus::Pass))
        .collect()
}

/// PB §5.4 step 2: "parent `B`, tree identical to `B`'s, and no test run,
/// skipping the step that computes `T`."
///
/// GR §5.8: "a **tombstone** is exempt from the rule entirely — all five
/// `exempt`, `profile: n/a`."
fn tombstone(c_m4: AutoMerge) -> Report {
    Report {
        subject: Subject {
            lane: Lane::Gated,
            event: Event::Withdraw,
            intent: Some(IntentId::parse("INT-042").unwrap()),
            strategy: Strategy::Merge,
        },
        objects: objects(Some(sha1("dfb4079e22de55ec377468b9b697fdf86085ea37"))),
        tool: tool(),
        git_version: "2.45".to_owned(),
        object_format: ObjectFormat::Sha1,
        mode: Mode::Team,
        profile: SealProfile::NotApplicable,
        policy: policy(c_m4),
        authority: Authority {
            // GR §5.2: on a tombstone `intent_blob` is the `Spine-Withdraw`
            // line's `blob=`, and an *orphaned* tombstone carries no sign-off.
            withdraw: Some(stmt(
                "Spine-Withdraw: INT-042 blob=dfb4079e22de55ec377468b9b697fdf86085ea37 \
                 signer=alice@example.com",
                Namespace::Signoff,
            )),
            ..Authority::default()
        },
        gates: all_pass(LandingShape::Tombstone),
        wires: WireSet::default(),
        floor_hits: Vec::new(),
        automerge: Automerge::EXEMPT,
        evidence: None,
        run: Run { reverifications: 0 },
    }
}

/// PB §7.4 rule 5: a reseal "does run the suite, does ingest a result file, and
/// seals the real `profile=` that file reports — never `n/a` … preconditions 1
/// and 2 are evaluated for it like any other landing that tests something."
fn reseal() -> Report {
    let preconditions = [
        // Under the shipped `C-A3: hostile`, precondition 0 is unmet.
        PreconditionStatus::Unmet,
        PreconditionStatus::Met,
        PreconditionStatus::Met,
        PreconditionStatus::Met,
        PreconditionStatus::Met,
    ];
    Report {
        subject: Subject {
            lane: Lane::Quick,
            event: Event::Reseal,
            intent: None,
            // GR §4.2: a reseal "records the repository's `C-M1` value" and
            // does not rest on it.
            strategy: Strategy::Squash,
        },
        objects: objects(None),
        tool: tool(),
        git_version: "2.45".to_owned(),
        object_format: ObjectFormat::Sha1,
        mode: Mode::Team,
        profile: SealProfile::Container,
        policy: policy(AutoMerge::On),
        authority: Authority {
            // PB §5.5: a reseal has no signer, so in team mode it takes "at
            // least two distinct `class=protected` reviews".
            reviews: vec![
                stmt("Spine-Review: reseal class=protected …", Namespace::Review),
                stmt("Spine-Review: reseal class=protected ..", Namespace::Review),
            ],
            ..Authority::default()
        },
        gates: all_pass(LandingShape::Reseal),
        wires: WireSet::from_raised(rule_five_wire(
            LandingShape::Reseal,
            AutoMerge::On,
            &preconditions,
        ))
        .unwrap(),
        floor_hits: Vec::new(),
        automerge: Automerge { preconditions },
        evidence: Some(Evidence {
            result_sha256: digest(
                "sha256:0b93f4ac5182d67e0a4c31fb9d20e857643ca0b1f9e78d5236ca04b81e7d3f96",
            ),
            collector: Collector {
                version: "1.4.0".to_owned(),
                dist_hash: digest(DIST),
            },
            keys_visible: false,
            ids: 412,
        }),
        run: Run { reverifications: 0 },
    }
}

/// PB §5.4 step 2 and PB §11: a tombstone's `Spine-Gates` lists G9, G13, G14,
/// G15 and nothing else.
#[test]
fn a_tombstone_is_four_gates_no_evidence_and_n_a() {
    let t = tombstone(AutoMerge::On);
    assert_eq!(t.validate(), Vec::new());
    assert_eq!(
        spine_gates_value(&t.gates),
        "G9=pass G13=pass G14=pass G15=pass"
    );
    assert_eq!(t.profile, SealProfile::NotApplicable);
    assert!(t.evidence.is_none());
    assert!(t.wires.is_empty());
}

/// GR §5.8: "A tombstone under `C-M4: on` therefore records `effective: true`.
/// All five are exempt, so the conjunction reduces to `requested`. That reads
/// like a bug and is not one."
#[test]
fn a_tombstone_under_c_m4_on_serializes_effective_true() {
    let on = canonicalize_to_string(&tombstone(AutoMerge::On).to_value());
    assert!(on.contains(r#""automerge":{"effective":true,"preconditions":["#));
    assert!(on.contains(r#"{"id":0,"status":"exempt"}"#));
    assert!(on.contains(r#""requested":true"#));

    let off = canonicalize_to_string(&tombstone(AutoMerge::Off).to_value());
    assert!(off.contains(r#""automerge":{"effective":false"#));
}

/// PB §11: `"n/a"` is reserved "for a landing that runs no suite, which after
/// v0.18 is the tombstone alone". A reseal recording it is the error PB §7.4
/// rule 5 corrects by name.
#[test]
fn a_reseal_that_records_n_a_is_refused() {
    let mut r = reseal();
    assert_eq!(r.validate(), Vec::new());
    r.profile = SealProfile::NotApplicable;
    assert!(
        r.validate()
            .contains(&Invariant::ProfileNotApplicableOffTombstone)
    );
}

/// GR §5.8: "**A reseal is exempt from nothing.** … `exempt` is used only where
/// the design grants exemption, and the grant is PB §7.4 rule 5's own,
/// singular: a **tombstone**."
///
/// GR §9.17 records the request to record a reseal's precondition 0 `"exempt"`
/// being "rejected twice over": it "would make `automerge.effective` say a
/// reseal could have auto-merged in a repository whose threat model forbids
/// auto-merge to exist at all".
#[test]
fn a_reseal_may_not_record_an_exempt_precondition() {
    let mut r = reseal();
    r.automerge.preconditions[0] = PreconditionStatus::Exempt;
    assert!(r.validate().contains(&Invariant::ExemptOffTombstone));
}

/// PB §7.4 rule 5: "a reseal does run the suite, does ingest a result file".
/// Its gate set and its rule-5 wire are a quick landing's exactly.
#[test]
fn a_reseal_runs_the_suite_and_raises_the_rule_five_wire() {
    let r = reseal();
    assert_eq!(r.validate(), Vec::new());
    assert!(r.evidence.is_some());
    assert_eq!(r.profile, SealProfile::Container);
    assert_eq!(r.wires.wires_line(), "G11");
    assert_eq!(r.wires.as_slice()[0].kind, WireKind::Advisory);
    assert!(r.gates.iter().any(|g| g.gate == Gate::G1));
}

/// GR §5.6.2's tombstone row: G3, G4 and G12 "read an in-flight intent, an
/// approval, or both, and a subjectless landing has neither". A tombstone
/// carrying a `G12=pass` entry has a `Spine-Gates` line, and therefore an
/// `envelope=`, that no conforming implementation reproduces (GR §9.12).
#[test]
fn a_tombstone_carrying_a_gate_that_did_not_run_is_refused() {
    let mut t = tombstone(AutoMerge::On);
    t.gates.push(GateResult::new(Gate::G12, GateStatus::Pass));
    assert!(
        t.validate()
            .contains(&Invariant::GateSetDisagreesWithTheShape)
    );
}

/// R2's three encodings in one landing, over a path GR §8's vector does not
/// reach: `floor_hits` stores `esc(path)`, its derived G14 wire's `path` member
/// stores `esc(path)`, and that wire's **token** — the byte string a reviewer
/// signs and the array's sort key — is `tok(path)`.
///
/// GR §6.2: `tok` "is `esc` for every path containing none of the three", which
/// is why no published digest separates them.
#[test]
fn one_floor_hit_is_esc_in_the_member_and_tok_in_the_token() {
    // A comma, a space and a quote — exactly the three bytes GR §6.2 moves.
    let path = b"adr/0001 draft,\"final\".md".to_vec();
    let mut r = reseal();
    r.floor_hits = vec![path.clone()];
    let mut raised = vec![Wire::bare(
        Gate::G11,
        WireClass::Tripwire,
        WireKind::Advisory,
    )];
    raised.extend(Report::floor_wires(&r.floor_hits));
    r.wires = WireSet::from_raised(raised).unwrap();

    assert_eq!(r.validate(), Vec::new());

    let g14 = r
        .wires
        .as_slice()
        .iter()
        .find(|w| w.gate == Gate::G14)
        .unwrap();

    // The `path` member: `esc`, which leaves all three bytes alone.
    assert_eq!(g14.path_member().unwrap(), r#"adr/0001 draft,"final".md"#);
    // The token: `tok`, which moves all three into the `\xHH` row.
    assert_eq!(g14.token(), r"G14:adr/0001\x20draft\x2c\x22final\x22.md");

    // And the serialized `floor_hits` entry is the `esc` spelling, JSON-escaped
    // once more on the way into the string — GR §2.3's two layers.
    let json = canonicalize_to_string(&r.to_value());
    assert!(
        json.contains(r#""floor_hits":["adr/0001 draft,\"final\".md"]"#),
        "{json}"
    );
}

/// GR §5.7: "for each entry `p`, `wires` contains exactly one … and `wires`
/// contains no other `G14` entry." Both halves.
#[test]
fn a_g14_wire_without_a_floor_hit_is_refused_and_so_is_the_reverse() {
    let mut r = reseal();
    r.wires = WireSet::from_raised([
        Wire::bare(Gate::G11, WireClass::Tripwire, WireKind::Advisory),
        Wire::at(
            Gate::G14,
            "adr/1.md",
            WireClass::Protected,
            WireKind::Finding,
        ),
    ])
    .unwrap();
    assert!(
        r.validate()
            .contains(&Invariant::FloorHitsAndG14WiresDisagree),
        "a G14 wire with no floor hit"
    );

    let mut r = reseal();
    r.floor_hits = vec![b"adr/1.md".to_vec()];
    assert!(
        r.validate()
            .contains(&Invariant::FloorHitsAndG14WiresDisagree),
        "a floor hit with no G14 wire"
    );
}

/// GR §6.3: G14 is not on PB §7.6's bypass list and its wire is `protected` —
/// "a landing overriding the frozen-test floor is exactly the emergency PB §7.6
/// says needs a second human".
#[test]
fn a_tripwire_class_floor_wire_is_refused() {
    let mut r = reseal();
    r.floor_hits = vec![b"adr/1.md".to_vec()];
    r.wires = WireSet::from_raised([
        Wire::bare(Gate::G11, WireClass::Tripwire, WireKind::Advisory),
        Wire::at(
            Gate::G14,
            "adr/1.md",
            WireClass::Tripwire,
            WireKind::Finding,
        ),
    ])
    .unwrap();
    let violations = r.validate();
    assert!(violations.contains(&Invariant::FloorHitsAndG14WiresDisagree));
    assert!(violations.contains(&Invariant::WireClassDisagreesWith63 { gate: Gate::G14 }));
}

/// GR §6.3's G12 row: "no version-1 landing report carries a `G12` entry in
/// `wires`" — that wire is "raised by `--approve` and **never** by `--land`".
#[test]
fn a_g12_wire_never_appears_in_a_landing_report() {
    let mut r = reseal();
    r.wires = WireSet::from_raised([
        Wire::bare(Gate::G11, WireClass::Tripwire, WireKind::Advisory),
        Wire::bare(Gate::G12, WireClass::Tripwire, WireKind::Finding),
    ])
    .unwrap();
    assert!(r.validate().contains(&Invariant::G12WireInALandingReport));
}

/// GR §5.9 and §9.21: a landing that ingested no result file records
/// `evidence` absent, `profile: "none"` — never `"n/a"`, which "would claim no
/// suite was attempted" — and preconditions 1 and 2 `"unmet"`.
///
/// This is the one reseal shape GR §5.6.2 admits: "One whose file is absent or
/// malformed is not exempt from anything and is not refused either."
#[test]
fn a_landing_that_ingested_nothing_records_none_and_no_evidence() {
    let mut r = reseal();
    r.profile = SealProfile::None;
    r.evidence = None;
    r.automerge.preconditions[1] = PreconditionStatus::Unmet;
    r.automerge.preconditions[2] = PreconditionStatus::Unmet;
    // G1 reads `override` under the reseal's own `class=protected` review
    // naming the bare `G1` (GR §5.6.1's reseal row).
    for g in &mut r.gates {
        if g.gate == Gate::G1 {
            g.status = GateStatus::Override;
        }
    }
    r.wires = WireSet::from_raised([
        Wire::bare(Gate::G11, WireClass::Tripwire, WireKind::Advisory),
        Wire::bare(Gate::G1, WireClass::Protected, WireKind::Finding),
    ])
    .unwrap();

    assert_eq!(r.validate(), Vec::new());
    assert!(r.is_landable());
    // GR §5.6.1's containment rule still applies: the review must name the
    // token, and the bare `G1` is what it names.
    assert_eq!(r.wires.wires_line(), "G1,G11");
}

/// GR §5: `n/a` and `evidence` are the tombstone's pair, and neither survives
/// alone.
#[test]
fn evidence_on_a_tombstone_is_refused() {
    let mut t = tombstone(AutoMerge::On);
    t.evidence = reseal().evidence;
    assert!(
        t.validate()
            .contains(&Invariant::EvidenceOnALandingThatRanNoSuite)
    );
}

/// GR §5.5: `approve` is "gated landing only; never on a tombstone, quick,
/// lifecycle or reseal landing".
#[test]
fn an_approve_on_a_reseal_is_refused() {
    let mut r = reseal();
    r.authority.approve = Some(stmt("Spine-Approve: …", Namespace::Review));
    assert!(r.validate().contains(&Invariant::ApproveOnNonGatedLanding));
}

/// GR §5.5: a reseal is signerless — "there is nobody to be self" — so both of
/// its protected reviews read `self_approved: false` even though they are
/// signed by the same key that signs everything else in this fixture.
#[test]
fn every_review_on_a_signerless_landing_is_not_self_approved() {
    let r = reseal();
    assert!(r.authority.signer_key().is_none());
    assert!(!r.self_approved());
    let json = canonicalize_to_string(&r.to_value());
    assert!(json.contains(r#""self_approved":false"#));
    assert_eq!(json.matches(r#""self_approved":false"#).count(), 3);
}

/// GR §5.10: `reverifications` is "≥ 0, ≤ `policy.rules.c_m3`" — the one
/// cross-member numeric bound in the schema.
#[test]
fn reverifications_above_c_m3_is_refused() {
    let mut r = reseal();
    r.run.reverifications = r.policy.rules.c_m3;
    assert_eq!(r.validate(), Vec::new());
    r.run.reverifications += 1;
    assert!(r.validate().contains(&Invariant::ReverificationsAboveCM3));
}

/// GR §5: `mode` "equals `policy.rules.c_a1` except on a recovery-sealed
/// landing (PB §7.5), where it is `recovery`".
#[test]
fn mode_equals_c_a1_unless_it_is_recovery() {
    let mut r = reseal();
    r.mode = Mode::Solo;
    assert!(r.validate().contains(&Invariant::ModeDisagreesWithCA1));
    r.mode = Mode::Recovery;
    assert_eq!(r.validate(), Vec::new());
}

/// GR §5.2: `intent_blob` is "present iff `subject.intent` is present". A
/// tombstone has both; a reseal has neither.
#[test]
fn intent_and_intent_blob_are_present_together() {
    let mut t = tombstone(AutoMerge::On);
    t.objects.intent_blob = None;
    assert!(t.validate().contains(&Invariant::IntentBlobPresence));

    let mut r = reseal();
    r.objects.intent_blob = Some(sha1("dfb4079e22de55ec377468b9b697fdf86085ea37"));
    assert!(r.validate().contains(&Invariant::IntentBlobPresence));
}

/// GR §7 rule 9: "lowercase hex at the full length `object_format` implies —
/// 40 or 64 digits." A report declaring `sha256` while carrying sha1 ids is one
/// no repository produced.
#[test]
fn an_oid_of_the_wrong_width_for_the_declared_format_is_refused() {
    let mut r = reseal();
    r.object_format = ObjectFormat::Sha256;
    let violations = r.validate();
    assert!(violations.iter().any(|v| matches!(
        v,
        Invariant::OidWidth {
            member: "objects.base"
        }
    )));
}

/// GR §7 rule 5: "Every array whose semantics is *the set of X* is emitted even
/// when empty; `[]` is a value, not an absence." And rule 6: `null` never
/// appears.
#[test]
fn empty_arrays_are_emitted_and_null_never_is() {
    let json = canonicalize_to_string(&tombstone(AutoMerge::On).to_value());
    assert!(json.contains(r#""floor_hits":[]"#));
    assert!(json.contains(r#""wires":[]"#));
    assert!(json.contains(r#""reopens":[]"#));
    assert!(json.contains(r#""reviews":[]"#));
    assert!(!json.contains("null"), "{json}");
    // And the absent members are absent, not empty. Matched with the trailing
    // `":` so that `self_approved`, which is present and false, is not read as
    // an `approve` member.
    assert!(!json.contains(r#""evidence":"#));
    assert!(!json.contains(r#""approve":"#));
    assert!(!json.contains(r#""upgrade":"#));
    assert!(!json.contains(r#""signoff":"#));
    assert!(json.contains(r#""self_approved":false"#));
}

/// Every shape round-trips through the reader to the same bytes, which is the
/// property `--verify` rests on for landings GR §8 publishes no vector for.
#[test]
fn every_shape_round_trips_to_the_same_bytes() {
    for report in [
        tombstone(AutoMerge::On),
        tombstone(AutoMerge::Off),
        reseal(),
    ] {
        let bytes = report.canonical_bytes();
        let parsed = Report::from_canonical(&bytes).expect("parses");
        assert_eq!(parsed, report);
        assert_eq!(parsed.canonical_bytes(), bytes);
    }
}

/// GR §9.10's third checkable redundancy: "`policy.floor_extensions` restates
/// every entry of `policy.rules.c_a2` under a second ordering."
///
/// PB §7.3 makes `C-A2` a floor source, so a report whose `floor_extensions`
/// dropped one would let G14 miss a diff entry a human declared protected.
#[test]
fn floor_extensions_restates_every_c_a2_entry() {
    let mut r = reseal();
    assert_eq!(r.validate(), Vec::new());
    r.policy.rules.c_a2.push(b"db/migrations/".to_vec());
    assert!(
        r.validate()
            .contains(&Invariant::FloorExtensionsMissingCA2Entry)
    );
    r.policy.floor_extensions.push(b"db/migrations/".to_vec());
    assert_eq!(r.validate(), Vec::new());
}
