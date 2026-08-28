//! `manifest.md` §8.6 — a rollback restoration, reproduced.
//!
//! §8.6 does not print `A` or `M_T`. It prints §8.3's manifest, a table of
//! deltas, and the two digests, and it says exactly how the digests were
//! obtained: "Both were computed by applying the delta table above to §8.3's
//! printed bytes and re-serializing by JCS". This test does that, with the
//! monotone union of `rollback::monotone_union` supplying `M_T.paths` — so what
//! is checked is not that the two byte counts were copied down correctly but
//! that **this crate's union produces the manifest §8.6 published**.
//!
//! `tests/vectors/mf-8.3-manifest.json` is §8.3's printed manifest, byte for
//! byte; every digest below is recomputed from it here.

use spine_canon::{ObjectFormat, Value};
use spine_init::rollback;
use spine_manifest::Manifest;

const MF_8_3: &[u8] = include_bytes!("vectors/mf-8.3-manifest.json");

fn base() -> Manifest {
    Manifest::parse(MF_8_3, Some(ObjectFormat::Sha1)).expect("§8.3 is a conforming manifest")
}

/// Replace one member of an object, keeping every other byte.
fn with_member(value: &Value, name: &str, new: Value) -> Value {
    let Value::Obj(members) = value else {
        panic!("not an object")
    };
    Value::Obj(
        members
            .iter()
            .map(|(k, v)| {
                if k == name {
                    (k.clone(), new.clone())
                } else {
                    (k.clone(), v.clone())
                }
            })
            .collect(),
    )
}

fn without_member(value: &Value, name: &str) -> Value {
    let Value::Obj(members) = value else {
        panic!("not an object")
    };
    Value::Obj(members.iter().filter(|(k, _)| k != name).cloned().collect())
}

/// `A` — the manifest at `<sha>`, built by applying §8.6's delta table to
/// §8.3's bytes.
///
/// Every value in the table is §8.6's own, including the one stand-in it names:
/// "`A.cli.dist_hash` is the one stand-in … its digest is fixed as the SHA-256
/// of the 21 ASCII bytes `spine-1.3.0-artifacts`, no trailing newline" — which
/// is computed here rather than copied, because a published digest that is not
/// in the value space of anything is the worst failure available.
fn ancestor() -> Manifest {
    let dist_hash = spine_canon::sha256_prefixed(b"spine-1.3.0-artifacts");
    assert_eq!(
        dist_hash, "sha256:1bcc0dea652db94e6e3ca7c79455cd3e89292f7ffa14c85aa21d620a14579ea7",
        "§8.6's printed stand-in, recomputed from the 21 bytes it names"
    );
    assert_eq!(b"spine-1.3.0-artifacts".len(), 21);

    let mut value = base().value().clone();

    value = with_member(
        &value,
        "cli",
        Value::obj([
            ("dist_hash", Value::Str(dist_hash)),
            ("version", Value::Str("1.3.0".into())),
        ]),
    );

    // "`templates.ci-generic` · `.ci-github-collect` · `.ci-github-land` ·
    // `.ci-gitlab` | `4` | `3`"
    let Value::Obj(templates) = value.get("templates").expect("templates").clone() else {
        panic!("templates is an object")
    };
    let templates = Value::Obj(
        templates
            .into_iter()
            .map(|(k, v)| {
                if k.starts_with("ci-") {
                    (k, Value::Int(3))
                } else {
                    (k, v)
                }
            })
            .collect(),
    );
    value = with_member(&value, "templates", templates);

    // The three `files[]` deltas, including the class change §8.6 explains:
    // "`spine-land.yml` is `spine-owned` at `<sha>` and `user-modified` at `B`
    // because the hand-tune happened *during* the 1.3.0 → 1.4.0 upgrade".
    let Value::Arr(files) = value.get("files").expect("files").clone() else {
        panic!("files is an array")
    };
    let files = Value::Arr(
        files
            .into_iter()
            .map(|record| match record.get("path").and_then(Value::as_str) {
                Some(".github/workflows/spine-collect.yml") => {
                    let r = with_member(
                        &record,
                        "blob",
                        Value::Str("081136631faa5fca86793d3b940b5bd83952c55a".into()),
                    );
                    with_member(&r, "template", Value::Str("ci-github-collect@3".into()))
                }
                Some(".github/workflows/spine-land.yml") => {
                    let r = without_member(&record, "base");
                    let r = with_member(
                        &r,
                        "blob",
                        Value::Str("1e27a99f6888d22c1dcc129d8ef9915ea7d0fb4f".into()),
                    );
                    let r = with_member(&r, "owner", Value::Str("spine-owned".into()));
                    with_member(&r, "template", Value::Str("ci-github-land@3".into()))
                }
                Some(".spine/ci.sh") => {
                    let r = with_member(
                        &record,
                        "blob",
                        Value::Str("d61e31f1a8d0130fb53241f89296ea89c2288677".into()),
                    );
                    with_member(&r, "template", Value::Str("ci-generic@3".into()))
                }
                _ => record,
            })
            .collect(),
    );
    value = with_member(&value, "files", files);

    // "`paths.agent_context` | `["AGENTS.md","CLAUDE.md"]` | `"AGENTS.md"`"
    let paths = with_member(
        value.get("paths").expect("paths"),
        "agent_context",
        Value::Str("AGENTS.md".into()),
    );
    value = with_member(&value, "paths", paths);

    Manifest::from_value(value, Some(ObjectFormat::Sha1)).expect("A is conforming")
}

/// §8.6's two published digests, over the manifests this crate builds.
#[test]
fn the_published_ancestor_and_rollback_manifests_reproduce() {
    let ancestor = ancestor();
    let base = base();

    let a_bytes = ancestor.to_bytes();
    assert_eq!(a_bytes.len() - 1, 1696, "A — canonical bytes");
    assert_eq!(
        ancestor.blob_id(ObjectFormat::Sha1),
        "24f11f00752bfb7bea259b4205315e7597692aca",
        "A — git blob (sha1)"
    );

    // The rollback's manifest: `A` whole, with `paths` replaced by the monotone
    // union. Nothing else is copied across, which is MF §6.7 step 3's stronger
    // reading — `eq(M_T with paths removed, A with paths removed)`.
    let rolled = rollback::rollback_manifest(&ancestor, &base, ObjectFormat::Sha1)
        .expect("M_T is conforming");
    let t_bytes = rolled.to_bytes();
    assert_eq!(t_bytes.len() - 1, 1710, "M_T — canonical bytes");
    assert_eq!(
        rolled.blob_id(ObjectFormat::Sha1),
        "74806e98701b50e958074dbaad0d7509d84751a3",
        "M_T — git blob (sha1)"
    );

    // "the 14-byte gap between them is `["AGENTS.md","CLAUDE.md"]` against
    // `"AGENTS.md"` and nothing else."
    assert_eq!(t_bytes.len() - a_bytes.len(), 14);
}

/// §8.6, computed: "`agent_context` gains `CLAUDE.md` from `B` — *the floor
/// never shrinks, not even backwards* — and the two-element result is written
/// as a sorted array while `constitution` stays a string, per §3.4."
#[test]
fn the_union_is_per_key_and_written_in_canonical_shape() {
    let union = rollback::monotone_union(&ancestor(), &base());
    assert_eq!(
        union,
        Value::obj([
            (
                "agent_context",
                Value::Arr(vec![
                    Value::Str("AGENTS.md".into()),
                    Value::Str("CLAUDE.md".into()),
                ])
            ),
            ("constitution", Value::Str("CONSTITUTION.md".into())),
        ]),
        "a singleton is a string, two or more a sorted array (MF §3.4)"
    );
}

/// MF §6.7.2's `P`, computed over §8.6's two manifests. §8.6 prints it:
///
/// ```text
/// P = { .gitattributes#spine, .github/workflows/spine-collect.yml,
///       .github/workflows/spine-land.yml, .gitignore#spine, .spine/ci.sh,
///       AGENTS.md#spine }
/// ```
///
/// "— from **both** manifests, and `.spine/allowed_signers` and
/// `CONSTITUTION.md` are excluded because both manifests call them
/// `user-owned`."
#[test]
fn mf_8_6_path_set_p_is_the_published_six() {
    assert_eq!(
        rollback::path_set(&ancestor(), &base()),
        vec![
            ".gitattributes#spine",
            ".github/workflows/spine-collect.yml",
            ".github/workflows/spine-land.yml",
            ".gitignore#spine",
            ".spine/ci.sh",
            "AGENTS.md#spine",
        ]
    );
}

/// "A path listed `spine-owned` in one and `user-modified` in the other is in
/// `P` once" — `spine-land.yml` is exactly that path, and it appears once.
#[test]
fn a_path_that_changed_class_is_in_p_once() {
    let p = rollback::path_set(&ancestor(), &base());
    assert_eq!(
        p.iter()
            .filter(|path| *path == ".github/workflows/spine-land.yml")
            .count(),
        1
    );
}

/// MF §6.7 step 3's stronger reading, negatively: a "rollback" that restored
/// every frozen field and every `files[]` record while quietly lowering
/// `resign` is not `eq(M_T with paths removed, A with paths removed)`, and this
/// crate cannot produce one — `rollback_manifest` takes `A` whole.
#[test]
fn the_rollback_manifest_is_a_whole_and_not_a_field_by_field_copy() {
    let ancestor = ancestor();
    let rolled =
        rollback::rollback_manifest(&ancestor, &base(), ObjectFormat::Sha1).expect("conforming");

    // Every member but `paths` is byte-identical to `A`'s.
    let strip = |m: &Manifest| without_member(m.value(), "paths");
    assert_eq!(
        spine_canon::canonicalize_to_string(&strip(&rolled)),
        spine_canon::canonicalize_to_string(&strip(&ancestor)),
        "eq(M_T with paths removed, A with paths removed)"
    );
    // Including the ones a field-by-field copy would have missed.
    assert_eq!(
        rolled.resign_version("intent"),
        ancestor.resign_version("intent")
    );
    assert_eq!(rolled.repo(), ancestor.repo());
    assert_eq!(rolled.cli_version(), "1.3.0");
}
