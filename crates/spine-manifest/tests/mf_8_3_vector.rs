//! `manifest.md` §8.3 — the published manifest, reproduced.
//!
//! Every digest asserted here is one the corpus publishes and this test
//! recomputes from the bytes in `tests/vectors/mf-8.3-manifest.json`, which are
//! the bytes §8.3 prints. Nothing is asserted that was not computed: MF §8's
//! own rule, and the reason `gate-report.md` §8.2 caught a fingerprint that was
//! not in the value space of any key.
//!
//! Note which `ci.sh` blob this vector carries. MF §8.1's **234-byte stand-in**
//! is what `.spine/ci.sh`'s record names here, *not* CI §5.3's real 319-line
//! render — two correct digests exist for one path, and using the real one
//! produces a manifest blob that is not `cb4cd490…`.

use spine_canon::ObjectFormat;
use spine_manifest::schema::{Isolation, Owner, V1_TEMPLATES};
use spine_manifest::{Manifest, Status};

const VECTOR: &[u8] = include_bytes!("vectors/mf-8.3-manifest.json");

fn parsed() -> Manifest {
    Manifest::parse(VECTOR, Some(ObjectFormat::Sha1)).expect("§8.3 is a conforming manifest")
}

#[test]
fn byte_counts_and_digests_are_the_published_ones() {
    let manifest = parsed();
    let file_bytes = manifest.to_bytes();

    // §8.3's table, every row.
    assert_eq!(file_bytes.len(), 1763, "file bytes (JCS + one LF)");
    assert_eq!(file_bytes.len() - 1, 1762, "canonical bytes (JCS, no LF)");
    assert_eq!(
        spine_canon::sha256_hex(&file_bytes[..file_bytes.len() - 1]),
        "b19e7a0142e93105b01c0fe54f6ba8824b21f5ffa757ec149bde8c56d981f0c3",
        "SHA-256 over the canonical bytes"
    );
    assert_eq!(
        spine_canon::sha256_hex(&file_bytes),
        "54fa96d16788a5f32b4efc06bf73774f2edcb45f6763a67b613c2216fcb7b327",
        "SHA-256 over the file bytes"
    );
    assert_eq!(
        manifest.blob_id(ObjectFormat::Sha1),
        "cb4cd49034bbe25f76573c40d6711b2c33f9136f",
        "git blob id, object_format: sha1"
    );
    assert_eq!(
        manifest.blob_id(ObjectFormat::Sha256),
        "65e47173762a4c67d6db74a671f0c24bb9b694f7b4acd959a9dee3bad649fb7f",
        "git blob id, object_format: sha256"
    );
}

/// Parsing and re-serializing must be the identity on the byte level. This is
/// the property that lets an old binary rewrite a new manifest without changing
/// it, and it is stronger than "we kept the fields we know about".
#[test]
fn round_trip_is_byte_identical() {
    assert_eq!(parsed().to_bytes(), VECTOR);
}

#[test]
fn the_typed_views_read_what_the_vector_says() {
    let m = parsed();
    assert_eq!(m.manifest_version(), 1);
    assert_eq!(m.repo(), "myrepo");
    assert_eq!(m.cli_version(), "1.4.0");
    assert_eq!(
        m.cli_dist_hash(),
        "sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db"
    );
    assert_eq!(m.object_format(), ObjectFormat::Sha1);
    assert_eq!(m.trunk(), "main");
    assert_eq!(m.ci(), "github");
    assert_eq!(m.isolation(), Isolation::Container);
    assert_eq!(m.timeout(), 1800);
    assert_eq!(m.langs(), vec!["python"]);
}

/// MF §3.4: the entry set is the **flattened value set**, so
/// `agent_context: ["AGENTS.md","CLAUDE.md"]` contributes two entries and the
/// key is not part of an entry's identity.
#[test]
fn the_floor_entry_set_is_flattened_over_values() {
    assert_eq!(
        parsed().floor_entries(),
        vec!["AGENTS.md", "CLAUDE.md", "CONSTITUTION.md"]
    );
}

/// MF §3.6: one key per template the release ships, "whether or not this
/// repository holds a rendered instance" — so `ci-gitlab` is present although
/// `params.ci` is `github`.
#[test]
fn all_twelve_template_keys_are_present_including_the_other_providers() {
    let m = parsed();
    for name in V1_TEMPLATES {
        assert!(
            m.template_version(name).is_some(),
            "templates.{name} is missing"
        );
    }
    assert_eq!(m.template_version("ci-gitlab"), Some(4));
    assert_eq!(m.ci(), "github");
}

#[test]
fn the_eight_file_records_carry_their_classes_and_regions() {
    let files = parsed().files();
    assert_eq!(files.len(), 8);

    let land = files
        .iter()
        .find(|f| f.path == ".github/workflows/spine-land.yml")
        .expect("the hand-tuned workflow");
    assert_eq!(land.owner, Owner::UserModified);
    assert_eq!(
        land.base.as_deref(),
        Some("4275e9df2ca6f096909f49fc8142fd87341abc07"),
        "a user-modified record carries the pristine render it diverged from"
    );
    assert_eq!(land.template, Some(("ci-github-land".to_string(), 4)));

    let agents = files
        .iter()
        .find(|f| f.path == "AGENTS.md#spine")
        .expect("the managed region");
    assert_eq!(agents.file_path, "AGENTS.md");
    assert_eq!(agents.region.as_deref(), Some("spine"));
    assert_eq!(agents.owner, Owner::SpineOwned);
    assert!(
        agents.base.is_none(),
        "only user-modified records carry a base"
    );

    // The three region records all use the key `spine`, which is never a
    // `templates` index — the record's own `template` member is.
    let regions: Vec<&str> = files.iter().filter_map(|f| f.region.as_deref()).collect();
    assert_eq!(regions, vec!["spine", "spine", "spine"]);
}

/// The records are sorted by `esc`-encoded path bytes, which is why
/// `.gitattributes#spine` precedes `.github/...` — `a` (0x61) before `h`
/// (0x68). A sort that treated `#` as a separator would order them the other
/// way and produce a different blob.
#[test]
fn records_are_sorted_by_the_whole_esc_path() {
    let files = parsed().files();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            ".gitattributes#spine",
            ".github/workflows/spine-collect.yml",
            ".github/workflows/spine-land.yml",
            ".gitignore#spine",
            ".spine/allowed_signers",
            ".spine/ci.sh",
            "AGENTS.md#spine",
            "CONSTITUTION.md",
        ]
    );
    let mut sorted = paths.clone();
    sorted.sort_unstable();
    assert_eq!(paths, sorted);
}

// ---- the refusals, exercised against the same vector -------------------

fn mutate(from: &str, to: &str) -> Status {
    let text = String::from_utf8(VECTOR.to_vec()).unwrap();
    assert!(text.contains(from), "mutation source {from:?} not present");
    let mutated = text.replacen(from, to, 1);
    Manifest::parse(mutated.as_bytes(), Some(ObjectFormat::Sha1))
        .expect_err("the mutation should be refused")
        .status
}

#[test]
fn canonicality_is_a_gate_condition_not_a_convention() {
    // Two members transposed: still valid JSON, still every field present, and
    // refused — because G16 compares blobs and a hand-reordered manifest is a
    // different blob for the same facts (MF §2.4).
    let text = String::from_utf8(VECTOR.to_vec()).unwrap();
    let swapped = text.replacen(
        r#"{"cli":{"dist_hash":"#,
        r#"{"repo":"myrepo","cli":{"dist_hash":"#,
        1,
    );
    // `repo` now appears twice — which is the *first* fault in document order.
    assert_eq!(
        Manifest::parse(swapped.as_bytes(), Some(ObjectFormat::Sha1))
            .unwrap_err()
            .status,
        Status::ManifestDuplicateMember
    );

    // A genuinely non-canonical but duplicate-free document: whitespace.
    let spaced = text.replacen(r#""envelope":1"#, r#""envelope": 1"#, 1);
    assert_eq!(
        Manifest::parse(spaced.as_bytes(), Some(ObjectFormat::Sha1))
            .unwrap_err()
            .status,
        Status::ManifestNoncanonical
    );
}

#[test]
fn framing_faults_are_refused_before_json() {
    let mut no_lf = VECTOR.to_vec();
    no_lf.pop();
    assert_eq!(
        Manifest::parse(&no_lf, Some(ObjectFormat::Sha1))
            .unwrap_err()
            .status,
        Status::ManifestNoncanonical,
        "MF §2.4 requires exactly one trailing LF"
    );

    let mut crlf = VECTOR.to_vec();
    crlf.insert(crlf.len() - 1, b'\r');
    assert_eq!(
        Manifest::parse(&crlf, Some(ObjectFormat::Sha1))
            .unwrap_err()
            .status,
        Status::ManifestNotJson,
        "no 0x0D anywhere"
    );

    let mut bom = vec![0xEF, 0xBB, 0xBF];
    bom.extend_from_slice(VECTOR);
    assert_eq!(
        Manifest::parse(&bom, Some(ObjectFormat::Sha1))
            .unwrap_err()
            .status,
        Status::ManifestNotJson,
        "no BOM"
    );
}

#[test]
fn scalar_domains_are_enforced() {
    assert_eq!(
        mutate(r#""repo":"myrepo""#, r#""repo":"my repo""#),
        Status::RepoOutOfGrammar
    );
    assert_eq!(
        mutate(r#""version":"1.4.0""#, r#""version":"none""#),
        Status::CliVersionOutOfGrammar
    );
    assert_eq!(
        mutate(r#""object_format":"sha1""#, r#""object_format":"md5""#),
        Status::ObjectFormatUnknown
    );
    assert_eq!(
        mutate(r#""ci":"github""#, r#""ci":"jenkins""#),
        Status::CiUnknown
    );
    assert_eq!(
        mutate(r#""isolation":"container""#, r#""isolation":"vm""#),
        Status::IsolationUnknown
    );
    assert_eq!(
        mutate(r#""timeout":1800"#, r#""timeout":0"#),
        Status::TimeoutOutOfRange
    );
    assert_eq!(
        mutate(r#""timeout":1800"#, r#""timeout":86401"#),
        Status::TimeoutOutOfRange
    );
    // `kotlin` was dropped by the owner and is not reserved at this level.
    assert_eq!(
        mutate(r#""langs":["python"]"#, r#""langs":["kotlin"]"#),
        Status::LangsUnknown
    );
    assert_eq!(
        mutate(r#""langs":["python"]"#, r#""langs":[]"#),
        Status::LangsEmpty
    );
}

/// MF §3.5: `base` is present **iff** the class is `user-modified`. Both
/// directions are `files-base-misplaced`.
#[test]
fn base_is_bound_to_the_user_modified_class_in_both_directions() {
    assert_eq!(
        mutate(
            r#"{"base":"4275e9df2ca6f096909f49fc8142fd87341abc07","blob":"e85fcdd455ece650d2c463ec5f7c52be802521c8","owner":"user-modified""#,
            r#"{"base":"4275e9df2ca6f096909f49fc8142fd87341abc07","blob":"e85fcdd455ece650d2c463ec5f7c52be802521c8","owner":"spine-owned""#,
        ),
        Status::FilesBaseMisplaced,
        "a spine-owned record must not carry a base"
    );
}

/// MF §3.2: an abbreviated blob id compares unequal to `git ls-tree`'s output,
/// so it is refused at the door rather than failing G16 mysteriously later.
#[test]
fn blob_ids_are_never_abbreviated() {
    assert_eq!(
        mutate(
            r#""blob":"91b88cb441665850be9c99df862e715fbea11311""#,
            r#""blob":"91b88cb""#,
        ),
        Status::BlobMalformed
    );
}

/// MF §3.6 / G16 check 7: a record's `@<n>` must equal the `templates` key's
/// value. This is what tells a collector rendered at `@4` from a lander left at
/// `@3`, and why the two GitHub workflows are two template names.
#[test]
fn a_record_at_the_wrong_template_version_is_refused() {
    assert_eq!(
        mutate(
            r#""template":"ci-github-land@4""#,
            r#""template":"ci-github-land@3""#
        ),
        Status::TemplateVersionMismatch
    );
    assert_eq!(
        mutate(r#""template":"agents-block@2""#, r#""template":"v2""#),
        Status::TemplateMalformed,
        "README decision 4: never a bare v2"
    );
}

/// MF §3.6: `resign` is intent-only, and `1 <= resign[v] <= templates[v]`.
#[test]
fn resign_is_intent_only_and_never_above_the_current_version() {
    assert_eq!(
        mutate(
            r#""resign":{"intent":2"#,
            r#""resign":{"constitution":1,"intent":2"#
        ),
        Status::ResignKeyUnknown
    );
    assert_eq!(
        mutate(r#""resign":{"intent":2"#, r#""resign":{"intent":3"#),
        Status::ResignFloorAboveCurrent
    );
}

/// MF §3.4: a singleton is a string and two-or-more is a sorted array. A
/// one-element array is `manifest-noncanonical` — one set of entries has one
/// spelling, because G16 compares blobs.
#[test]
fn paths_values_have_exactly_one_canonical_shape() {
    assert_eq!(
        mutate(
            r#""constitution":"CONSTITUTION.md""#,
            r#""constitution":["CONSTITUTION.md"]"#,
        ),
        Status::ManifestNoncanonical
    );
    assert_eq!(
        mutate(
            r#""agent_context":["AGENTS.md","CLAUDE.md"]"#,
            r#""agent_context":["CLAUDE.md","AGENTS.md"]"#,
        ),
        Status::ManifestNoncanonical,
        "an unsorted array"
    );
    assert_eq!(
        mutate(
            r#""agent_context":["AGENTS.md","CLAUDE.md"]"#,
            r#""agent_context":["AGENTS.md","AGENTS.md"]"#,
        ),
        Status::ManifestNoncanonical,
        "a duplicated element"
    );
    assert_eq!(
        mutate(
            r#""agent_context":["AGENTS.md","CLAUDE.md"]"#,
            r#""agent_context":["/abs.md","AGENTS.md"]"#,
        ),
        Status::PathsValueMalformed
    );
}

/// G16 check 8: the manifest's `object_format` is cross-checked against the
/// repository's own. Disagreement is its own token, not `object-format-unknown`.
#[test]
fn object_format_is_cross_checked_against_the_repository() {
    assert_eq!(
        Manifest::parse(VECTOR, Some(ObjectFormat::Sha256))
            .unwrap_err()
            .status,
        Status::ObjectFormatMismatch
    );
    // With no repository to ask — the collector reading trunk's manifest
    // out of a bare object store — the cross-check is simply not made.
    assert!(Manifest::parse(VECTOR, None).is_ok());
}

/// PB §6.7's forward-compatibility promise, which is the reason this crate
/// holds a `Value` rather than a struct: an unknown member is preserved, and a
/// binary that rewrites the manifest must not drop it.
#[test]
fn unknown_members_survive_a_round_trip() {
    let text = String::from_utf8(VECTOR.to_vec()).unwrap();
    // Inserted in canonical position: `future_field` sorts after `files` and
    // before `manifest_version`.
    let extended = text.replacen(
        r#","manifest_version":1"#,
        r#","future_field":{"a":1},"manifest_version":1"#,
        1,
    );
    let m = Manifest::parse(extended.as_bytes(), Some(ObjectFormat::Sha1))
        .expect("an unknown member is opaque data, not a refusal");
    assert_eq!(
        m.to_bytes(),
        extended.as_bytes(),
        "the unknown member must come back byte-identical"
    );
    assert_eq!(m.repo(), "myrepo", "and the known fields still read");
}
