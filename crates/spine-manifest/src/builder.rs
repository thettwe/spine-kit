//! Building a manifest from typed inputs — the write path.
//!
//! [`crate::schema::Manifest::parse`] is the read path and holds its parsed
//! `Value` so unknown members survive. This is the other direction: `spine
//! init` has params, a render set and a template map, and needs the canonical
//! bytes.
//!
//! The builder emits members in *insertion* order and lets
//! [`spine_canon::canonicalize`] sort them. That is deliberate: a builder that
//! emitted them pre-sorted would encode JCS's ordering rule in a second place,
//! and two places that must agree about an ordering are two places that can
//! drift. There is exactly one implementation of the sort, in `spine-canon`.

use crate::schema::{Manifest, Owner};
use crate::status::Result;
use spine_canon::{ObjectFormat, Value};

/// A `files[]` record to be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// A repository path, or `<path>#<region key>`.
    pub path: String,
    pub owner: Owner,
    pub blob: String,
    pub template: Option<String>,
    /// Present **iff** `owner == UserModified` (MF §3.5) — the builder refuses
    /// the other combinations rather than emitting a manifest that will not
    /// parse.
    pub base: Option<String>,
}

/// Everything `init` decides, in one place.
#[derive(Debug, Clone)]
pub struct Builder {
    pub repo: String,
    pub cli_version: String,
    pub cli_dist_hash: String,
    pub object_format: ObjectFormat,
    pub schema: u64,
    pub envelope: u64,
    pub manifest_version: u64,
    pub trunk: String,
    pub ci: String,
    /// Written only when it is not the default. MF §3.3 makes absent mean
    /// `none`, and PB §6.7 calls that fail-closed, so writing `"none"`
    /// explicitly is legal but writing nothing is what a `none` repository
    /// should serialize to — one state, one spelling.
    pub isolation: Option<String>,
    pub langs: Vec<String>,
    /// Likewise: absent means `1800` (MF §3.3).
    pub timeout: Option<u64>,
    /// The open map of floor entries. A key with exactly one entry serializes
    /// as a **string**; two or more as a **sorted array** (MF §3.4).
    pub paths: Vec<(String, Vec<String>)>,
    pub templates: Vec<(String, u64)>,
    pub resign: Vec<(String, u64)>,
    pub files: Vec<FileEntry>,
}

impl Builder {
    /// Build and validate. Returns a [`Manifest`], so the only way to get one
    /// out of this module is to get one the parser would also accept.
    pub fn build(mut self) -> Result<Manifest> {
        // MF §3.3: `langs` is non-empty, deduplicated and sorted ascending by
        // bytes. Sorting here rather than refusing an unsorted input is right
        // for a *builder* — the caller has a set, not a serialization — while
        // the parser still refuses an unsorted manifest on disk, because there
        // the order is the artifact.
        self.langs.sort();
        self.langs.dedup();

        // MF §3.5: sorted ascending by the `esc`-encoded path bytes, no
        // duplicates. Same reasoning.
        self.files.sort_by(|a, b| a.path.cmp(&b.path));

        // Insertion order, not sorted order: `spine-canon` owns the sort.
        let root: Vec<(String, Value)> = vec![
            (
                "cli".into(),
                Value::obj([
                    ("version", Value::str(self.cli_version.clone())),
                    ("dist_hash", Value::str(self.cli_dist_hash.clone())),
                ]),
            ),
            ("envelope".into(), Value::Int(self.envelope)),
            ("files".into(), self.files_value()),
            ("manifest_version".into(), Value::Int(self.manifest_version)),
            (
                "object_format".into(),
                Value::str(self.object_format.as_str()),
            ),
            ("params".into(), self.params_value()),
            ("paths".into(), self.paths_value()),
            ("repo".into(), Value::str(self.repo.clone())),
            (
                "resign".into(),
                Value::Obj(
                    self.resign
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::Int(*v)))
                        .collect(),
                ),
            ),
            ("schema".into(), Value::Int(self.schema)),
            (
                "templates".into(),
                Value::Obj(
                    self.templates
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::Int(*v)))
                        .collect(),
                ),
            ),
        ];

        Manifest::from_value(Value::Obj(root), Some(self.object_format))
    }

    fn params_value(&self) -> Value {
        let mut members: Vec<(String, Value)> = vec![
            ("trunk".into(), Value::str(self.trunk.clone())),
            ("ci".into(), Value::str(self.ci.clone())),
            (
                "langs".into(),
                Value::Arr(self.langs.iter().map(Value::str).collect()),
            ),
        ];
        if let Some(isolation) = &self.isolation {
            members.push(("isolation".into(), Value::str(isolation.clone())));
        }
        if let Some(timeout) = self.timeout {
            members.push(("timeout".into(), Value::Int(timeout)));
        }
        Value::Obj(members)
    }

    /// MF §3.4's canonical shape, which has exactly two forms and no third: a
    /// key with one entry is a **string**, a key with two or more is a
    /// **sorted array**. A one-element array, an empty array, an unsorted array
    /// or a duplicated element is `manifest-noncanonical` — so the builder
    /// produces the one spelling rather than trusting its caller.
    fn paths_value(&self) -> Value {
        Value::Obj(
            self.paths
                .iter()
                .map(|(key, entries)| {
                    let mut entries = entries.clone();
                    entries.sort();
                    entries.dedup();
                    let value = if entries.len() == 1 {
                        Value::str(entries[0].clone())
                    } else {
                        Value::Arr(entries.iter().map(Value::str).collect())
                    };
                    (key.clone(), value)
                })
                .collect(),
        )
    }

    fn files_value(&self) -> Value {
        Value::Arr(
            self.files
                .iter()
                .map(|entry| {
                    let mut members: Vec<(String, Value)> = vec![
                        ("path".into(), Value::str(entry.path.clone())),
                        ("owner".into(), Value::str(entry.owner.as_str())),
                        ("blob".into(), Value::str(entry.blob.clone())),
                    ];
                    if let Some(template) = &entry.template {
                        members.push(("template".into(), Value::str(template.clone())));
                    }
                    if let Some(base) = &entry.base {
                        members.push(("base".into(), Value::str(base.clone())));
                    }
                    Value::Obj(members)
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `manifest.md` §8.3, rebuilt from typed inputs.
    ///
    /// This is a stronger test than parsing §8.3: it proves the *write* path
    /// produces the published bytes, which is what `spine init` actually does.
    /// Parsing only proves the reader agrees with a file somebody else wrote.
    fn mf_8_3_builder() -> Builder {
        let file = |path: &str, owner: Owner, blob: &str, template: &str| FileEntry {
            path: path.into(),
            owner,
            blob: blob.into(),
            template: Some(template.into()),
            base: None,
        };
        Builder {
            repo: "myrepo".into(),
            cli_version: "1.4.0".into(),
            cli_dist_hash:
                "sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db".into(),
            object_format: ObjectFormat::Sha1,
            schema: 7,
            envelope: 1,
            manifest_version: 1,
            trunk: "main".into(),
            ci: "github".into(),
            isolation: Some("container".into()),
            langs: vec!["python".into()],
            timeout: Some(1800),
            paths: vec![
                (
                    "agent_context".into(),
                    vec!["AGENTS.md".into(), "CLAUDE.md".into()],
                ),
                ("constitution".into(), vec!["CONSTITUTION.md".into()]),
            ],
            templates: vec![
                ("agents-block".into(), 2),
                ("ci-generic".into(), 4),
                ("ci-github-collect".into(), 4),
                ("ci-github-land".into(), 4),
                ("ci-gitlab".into(), 4),
                ("constitution".into(), 1),
                ("gitattributes".into(), 1),
                ("gitignore".into(), 1),
                ("intent".into(), 2),
                ("intent-bug".into(), 2),
                ("intent-change".into(), 2),
                ("keyring".into(), 1),
            ],
            resign: vec![
                ("intent".into(), 2),
                ("intent-bug".into(), 2),
                ("intent-change".into(), 2),
            ],
            files: vec![
                file(
                    ".gitattributes#spine",
                    Owner::SpineOwned,
                    "91b88cb441665850be9c99df862e715fbea11311",
                    "gitattributes@1",
                ),
                file(
                    ".github/workflows/spine-collect.yml",
                    Owner::SpineOwned,
                    "e7f192f88d1f9605fc5b316d4bfa2eb78523013a",
                    "ci-github-collect@4",
                ),
                FileEntry {
                    path: ".github/workflows/spine-land.yml".into(),
                    owner: Owner::UserModified,
                    blob: "e85fcdd455ece650d2c463ec5f7c52be802521c8".into(),
                    template: Some("ci-github-land@4".into()),
                    base: Some("4275e9df2ca6f096909f49fc8142fd87341abc07".into()),
                },
                file(
                    ".gitignore#spine",
                    Owner::SpineOwned,
                    "e7b7021f73cd490a36a99973cb26c09c974b930d",
                    "gitignore@1",
                ),
                file(
                    ".spine/allowed_signers",
                    Owner::UserOwned,
                    "6d4db08390092d7d5d96476eddca6355815bc49f",
                    "keyring@1",
                ),
                file(
                    ".spine/ci.sh",
                    Owner::SpineOwned,
                    "dc1893727069b1c188505544ecf4174d48a13bdb",
                    "ci-generic@4",
                ),
                file(
                    "AGENTS.md#spine",
                    Owner::SpineOwned,
                    "ccf916b1f5a2813b9156128dff6f3bc4036c8b2d",
                    "agents-block@2",
                ),
                file(
                    "CONSTITUTION.md",
                    Owner::UserOwned,
                    "22609629e86d75a7c4abb7208c3575c7a8c2ead3",
                    "constitution@1",
                ),
            ],
        }
    }

    #[test]
    fn the_write_path_reproduces_mf_8_3_byte_for_byte() {
        let manifest = mf_8_3_builder().build().expect("a conforming manifest");
        let bytes = manifest.to_bytes();

        assert_eq!(bytes.len(), 1763, "file bytes (JCS + one LF)");
        assert_eq!(
            manifest.blob_id(ObjectFormat::Sha1),
            "cb4cd49034bbe25f76573c40d6711b2c33f9136f",
            "the published blob — built, not parsed"
        );
        assert_eq!(
            spine_canon::sha256_hex(&bytes),
            "54fa96d16788a5f32b4efc06bf73774f2edcb45f6763a67b613c2216fcb7b327"
        );

        // And it is byte-identical to the vector the parser reads.
        let vector = include_bytes!("../tests/vectors/mf-8.3-manifest.json");
        assert_eq!(bytes, vector);
    }

    /// The builder emits members in insertion order and lets `spine-canon` sort
    /// them, so the output must not depend on the order the caller supplied.
    /// A builder that pre-sorted would encode JCS's rule in a second place.
    #[test]
    fn member_order_at_the_input_does_not_reach_the_output() {
        let straight = mf_8_3_builder().build().unwrap().to_bytes();

        let mut shuffled = mf_8_3_builder();
        shuffled.templates.reverse();
        shuffled.resign.reverse();
        shuffled.paths.reverse();
        shuffled.files.reverse();
        shuffled.langs.reverse();

        assert_eq!(
            shuffled.build().unwrap().to_bytes(),
            straight,
            "one set of facts has one serialization"
        );
    }

    /// MF §3.4: one entry is a string, two or more is a sorted array. There is
    /// no third form — a one-element array is `manifest-noncanonical`.
    #[test]
    fn a_paths_key_takes_the_one_canonical_shape_for_its_size() {
        let mut builder = mf_8_3_builder();
        builder.paths = vec![
            ("constitution".into(), vec!["CONSTITUTION.md".into()]),
            // MF §3.4's path rules forbid a trailing `/`: a `paths` value is a
            // repository **path**, not one of the constitution's patterns. The
            // pattern form (`adr/`) belongs to `C-A2`, whose type is
            // `pattern-list` (CN §5.5) and whose grammar is a different one.
            ("adr".into(), vec!["adr".into(), "docs/adr".into()]),
        ];
        let manifest = builder.build().unwrap();
        let text = String::from_utf8(manifest.to_bytes()).unwrap();
        assert!(text.contains(r#""constitution":"CONSTITUTION.md""#));
        assert!(text.contains(r#""adr":["adr","docs/adr"]"#));

        // And the entry set is the flattened values, so `adr` contributes two.
        assert_eq!(
            manifest.floor_entries(),
            vec!["CONSTITUTION.md", "adr", "docs/adr"]
        );

        // The pattern form is refused where it does not belong, which is what
        // keeps the two vocabularies apart.
        let mut pattern_shaped = mf_8_3_builder();
        pattern_shaped.paths = vec![("adr".into(), vec!["adr/".into(), "docs/adr/".into()])];
        assert_eq!(
            pattern_shaped.build().unwrap_err().status,
            crate::Status::PathsValueMalformed
        );
    }

    /// MF §3.3: absent means `none` for `isolation` and `1800` for `timeout`,
    /// and both defaults are fail-closed (MF §7 rule 6). A repository at the
    /// default serializes to the absent form — one state, one spelling.
    #[test]
    fn the_two_defaulted_params_are_omitted_rather_than_spelled_out() {
        let mut builder = mf_8_3_builder();
        builder.isolation = None;
        builder.timeout = None;
        let manifest = builder.build().unwrap();
        let text = String::from_utf8(manifest.to_bytes()).unwrap();

        assert!(!text.contains("isolation"));
        assert!(!text.contains("timeout"));
        // And the reader still sees the fail-closed values.
        assert_eq!(manifest.isolation(), crate::schema::Isolation::None);
        assert_eq!(manifest.timeout(), 1800);
    }

    /// The builder validates through the parser's own rules, so it cannot
    /// produce a manifest the parser would refuse.
    #[test]
    fn the_builder_cannot_emit_what_the_parser_refuses() {
        let mut bad_base = mf_8_3_builder();
        bad_base.files[0].base = Some("4275e9df2ca6f096909f49fc8142fd87341abc07".into());
        assert_eq!(
            bad_base.build().unwrap_err().status,
            crate::Status::FilesBaseMisplaced,
            "only a user-modified record carries a base"
        );

        let mut bad_repo = mf_8_3_builder();
        bad_repo.repo = "my repo".into();
        assert_eq!(
            bad_repo.build().unwrap_err().status,
            crate::Status::RepoOutOfGrammar
        );

        let mut bad_resign = mf_8_3_builder();
        bad_resign.resign.push(("constitution".into(), 1));
        assert_eq!(
            bad_resign.build().unwrap_err().status,
            crate::Status::ResignKeyUnknown,
            "resign is intent-only"
        );
    }
}
