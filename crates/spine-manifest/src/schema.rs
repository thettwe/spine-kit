//! The manifest's framing, canonical form, domains and typed views.
//!
//! **The parsed `Value` is the source of truth, not a struct.** PB §6.7 makes
//! forward compatibility the whole point of the artifact — "every binary parses
//! [the frozen fields] for every `manifest_version` it will ever meet and
//! treats the rest as opaque — that is what lets an old binary judge a new
//! manifest". A struct with named fields drops what it has no field for, and a
//! dropped member is a manifest an old binary rewrites into a different
//! document. Holding the `Value` makes preservation the default and losing a
//! member impossible rather than merely discouraged.

use crate::grammar;
use crate::status::{Refusal, Result, Status};
use spine_canon::{ObjectFormat, Value, canonicalize};

/// MF §2.2's resource bounds. Exceeding one is `manifest-too-large`.
pub const MAX_FILE_BYTES: usize = 1024 * 1024;
pub const MAX_ARRAY_ELEMENTS: usize = 4096;
pub const MAX_STRING_BYTES: usize = 8192;
pub const MAX_OBJECT_MEMBERS: usize = 256;
/// MF §2.2: depth <= 6, counting the root object as 1. v1 reaches 3.
pub const MAX_DEPTH: usize = 6;

/// MF §3.4: a `paths` key is never `trunk` or `dist_hash` — those names belong
/// to `params` and `cli`, and a `paths` entry spelled with one would read as a
/// floor entry named after a field.
pub const RESERVED_PATHS_KEYS: [&str; 2] = ["trunk", "dist_hash"];

/// The twelve templates the v1 release ships (MF §3.6, PB §6.7).
///
/// One key per template **the release ships**, whether or not this repository
/// holds a rendered instance — so a `--ci github` repository still carries
/// `ci-gitlab`, and a provider migration adds no key.
pub const V1_TEMPLATES: [&str; 12] = [
    "agents-block",
    "ci-generic",
    "ci-github-collect",
    "ci-github-land",
    "ci-gitlab",
    "constitution",
    "gitattributes",
    "gitignore",
    "intent",
    "intent-bug",
    "intent-change",
    "keyring",
];

/// MF §3.6, TM §7.2: `resign` is intent-only.
pub const RESIGN_KEYS: [&str; 3] = ["intent", "intent-bug", "intent-change"];

/// MF §3.3. Note `kotlin` is absent and **not reserved at the manifest level**:
/// a manifest carrying it is `langs-unknown`, not a reserved-token refusal.
pub const V1_LANGS: [&str; 4] = ["dart", "python", "swift", "ts"];

pub const CI_PROVIDERS: [&str; 3] = ["generic", "github", "gitlab"];

/// MF §3.5, PB §6.7. "The set never changes at any `manifest_version`."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    SpineOwned,
    UserOwned,
    UserModified,
}

impl Owner {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "spine-owned" => Some(Owner::SpineOwned),
            "user-owned" => Some(Owner::UserOwned),
            "user-modified" => Some(Owner::UserModified),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Owner::SpineOwned => "spine-owned",
            Owner::UserOwned => "user-owned",
            Owner::UserModified => "user-modified",
        }
    }
}

/// PB §7.4 rule 3's three profiles. `params.isolation` is the **request**;
/// the collector's header field is the **finding** (RF §7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Isolation {
    Container,
    /// v1 ships no mechanism for this. A manifest at `T` requesting it fails
    /// G16 **outright** with `isolation-unsupported` (MF §6.2 check 12b), and
    /// the collector refuses at step 1 rather than downgrading (RF §7.1).
    Uid,
    None,
}

impl Isolation {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "container" => Some(Isolation::Container),
            "uid" => Some(Isolation::Uid),
            "none" => Some(Isolation::None),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Isolation::Container => "container",
            Isolation::Uid => "uid",
            Isolation::None => "none",
        }
    }
}

/// One `files[]` record, read out of the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecord {
    /// The `esc`-encoded path, including any `#<region key>` suffix.
    pub path: String,
    /// The path with the region suffix stripped, `esc`-encoded.
    pub file_path: String,
    /// `Some` when `path` names a managed region.
    pub region: Option<String>,
    pub owner: Owner,
    pub blob: String,
    /// `<name>@<version>`, split. Absent on a record with no template.
    pub template: Option<(String, u64)>,
    /// Present **iff** `owner == UserModified` (MF §3.5).
    pub base: Option<String>,
}

impl FileRecord {
    /// [`path`] decoded to raw bytes — including the `#<region key>` suffix,
    /// which is why this is not a repository path and must not be handed to
    /// anything that opens a file. It is the right value for a wire, whose
    /// contract is raw bytes.
    ///
    /// [`path`]: FileRecord::path
    pub fn path_raw(&self) -> Vec<u8> {
        spine_canon::unesc(&self.path).expect("validated")
    }

    /// [`file_path`] decoded to raw bytes: the path on disk.
    ///
    /// [`file_path`]: FileRecord::file_path
    pub fn file_path_raw(&self) -> Vec<u8> {
        spine_canon::unesc(&self.file_path).expect("validated")
    }
}

/// A parsed, validated `.spine/manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    value: Value,
}

impl Manifest {
    /// Parse from file bytes, validating framing, canonicality and every
    /// domain — MF §2.4 and the read path of the build sheet's algorithm B.
    ///
    /// Errors carry the **first** status in §3.11's document order and stop
    /// there, "because a manifest that does not parse cannot be checked
    /// further" (MF §3.11).
    pub fn parse(bytes: &[u8], repo_format: Option<ObjectFormat>) -> Result<Self> {
        // 1. Framing (MF §2.4). Checked before JSON, because a CR or a missing
        //    final LF is a byte fault and JSON would happily accept both.
        if bytes.len() > MAX_FILE_BYTES {
            return Err(Refusal::new(Status::ManifestTooLarge, "file"));
        }
        if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return Err(Refusal::new(Status::ManifestNotJson, "byte-order mark"));
        }
        if bytes.contains(&b'\r') {
            return Err(Refusal::new(Status::ManifestNotJson, "CR"));
        }
        let Some(body) = bytes.strip_suffix(b"\n") else {
            return Err(Refusal::new(
                Status::ManifestNoncanonical,
                "no final LF (MF §2.4)",
            ));
        };
        if body.contains(&b'\n') {
            return Err(Refusal::new(
                Status::ManifestNoncanonical,
                "an LF other than the final one",
            ));
        }

        // 2. JSON, under the profile.
        let value = spine_canon::parse(body).map_err(|e| {
            use spine_canon::parse::ParseErrorKind as K;
            match e.kind {
                K::DuplicateMember(_) => {
                    Refusal::new(Status::ManifestDuplicateMember, e.to_string())
                }
                K::TooDeep => Refusal::new(Status::ManifestTooLarge, e.to_string()),
                _ => Refusal::new(Status::ManifestNotJson, e.to_string()),
            }
        })?;

        // 3. Canonicality is a gate condition, not a convention (MF §2.4).
        //    Re-serializing and comparing is the only check that catches a
        //    hand-reordered member, which is exactly what G16 exists to see.
        if canonicalize(&value) != body {
            return Err(Refusal::new(
                Status::ManifestNoncanonical,
                "not RFC 8785 canonical",
            ));
        }

        let manifest = Manifest { value };
        manifest.validate(repo_format)?;
        Ok(manifest)
    }

    /// The canonical file bytes: `JCS(value) ++ 0x0A` (MF §2.4).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = canonicalize(&self.value);
        out.push(b'\n');
        out
    }

    /// The manifest's own blob id — `Spine-Upgrade`'s `manifest=`, and
    /// GR §5.4's `policy.manifest` (MF §2.4).
    pub fn blob_id(&self, format: ObjectFormat) -> String {
        spine_canon::git_blob_id(&self.to_bytes(), format)
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Build from an already-canonical value, validating it. Used by the
    /// renderer; `parse` is used for anything read off disk.
    pub fn from_value(value: Value, repo_format: Option<ObjectFormat>) -> Result<Self> {
        let manifest = Manifest { value };
        manifest.validate(repo_format)?;
        Ok(manifest)
    }

    // ---- typed views -----------------------------------------------------

    pub fn manifest_version(&self) -> u64 {
        self.value
            .get("manifest_version")
            .and_then(Value::as_u64)
            .expect("validated")
    }

    pub fn repo(&self) -> &str {
        self.value
            .get("repo")
            .and_then(Value::as_str)
            .expect("validated")
    }

    pub fn cli_version(&self) -> &str {
        self.value
            .get("cli")
            .and_then(|c| c.get("version"))
            .and_then(Value::as_str)
            .expect("validated")
    }

    pub fn cli_dist_hash(&self) -> &str {
        self.value
            .get("cli")
            .and_then(|c| c.get("dist_hash"))
            .and_then(Value::as_str)
            .expect("validated")
    }

    pub fn object_format(&self) -> ObjectFormat {
        self.value
            .get("object_format")
            .and_then(Value::as_str)
            .and_then(ObjectFormat::parse)
            .expect("validated")
    }

    pub fn trunk(&self) -> &str {
        self.params_str("trunk").expect("validated")
    }

    pub fn ci(&self) -> &str {
        self.params_str("ci").expect("validated")
    }

    /// MF §3.3, PB §6.7: **absent means `none`**, "so a manifest written before
    /// the field existed fails the auto-merge precondition rather than passing
    /// it by silence". Fail-closed, per MF §7 rule 6.
    pub fn isolation(&self) -> Isolation {
        self.params_str("isolation")
            .and_then(Isolation::parse)
            .unwrap_or(Isolation::None)
    }

    /// MF §3.3: absent means `1800`. Also fail-closed — a collector enforcing
    /// no deadline is non-conformant (PB §6.7).
    pub fn timeout(&self) -> u64 {
        self.value
            .get("params")
            .and_then(|p| p.get("timeout"))
            .and_then(Value::as_u64)
            .unwrap_or(1800)
    }

    pub fn langs(&self) -> Vec<&str> {
        self.value
            .get("params")
            .and_then(|p| p.get("langs"))
            .and_then(Value::as_arr)
            .map(|items| items.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    fn params_str(&self, name: &str) -> Option<&str> {
        self.value
            .get("params")
            .and_then(|p| p.get(name))
            .and_then(Value::as_str)
    }

    /// MF §3.4's entry set `E(M)`: the **flattened value set**, deduplicated,
    /// in the `esc`-encoded spelling the manifest stores (MF §2.3).
    ///
    /// "An entry is a value, not a key and not a list." The key is not part of
    /// an entry's identity, so moving `AGENTS.md` between keys drops no entry
    /// and shrinks no floor.
    ///
    /// **These are not path bytes.** `caf\xc3\xa9/CONSTITUTION.md` comes back
    /// as twenty-three ASCII characters, and comparing them against the raw
    /// bytes of a diff — which is what G14's literal floor does — silently
    /// matches nothing. Sort order is over the encoded bytes, which is what
    /// GR §5.7 wants for `floor_hits`; use [`floor_entries_raw`] wherever the
    /// entries meet path bytes.
    ///
    /// [`floor_entries_raw`]: Manifest::floor_entries_raw
    pub fn floor_entries(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        let Some(Value::Obj(members)) = self.value.get("paths") else {
            return out;
        };
        for (_key, value) in members {
            match value {
                Value::Str(s) => out.push(s),
                Value::Arr(items) => out.extend(items.iter().filter_map(Value::as_str)),
                _ => {}
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// `paths` as key → value set, in the key order the object stores (JCS
    /// sorts them, so ascending by key). Each value is the `esc` spelling.
    ///
    /// The **shape** — a string for a singleton, a sorted duplicate-free array
    /// of two or more (MF §3.4) — is settled at parse time, so a caller
    /// reading this back never has to re-check it and a caller *building* a
    /// `paths` object has to produce it.
    pub fn paths_by_key(&self) -> Vec<(&str, Vec<&str>)> {
        let Some(Value::Obj(members)) = self.value.get("paths") else {
            return Vec::new();
        };
        members
            .iter()
            .map(|(key, value)| {
                let values = match value {
                    Value::Str(s) => vec![s.as_str()],
                    Value::Arr(items) => items.iter().filter_map(Value::as_str).collect(),
                    _ => Vec::new(),
                };
                (key.as_str(), values)
            })
            .collect()
    }

    /// [`floor_entries`] decoded: `E(M)` as **raw path bytes**, the form every
    /// comparison against a tree or a diff needs.
    ///
    /// Order and deduplication are the encoded ones — `unesc` is injective, so
    /// decoding cannot merge two entries, and G14 reads this as a set.
    ///
    /// Infallible on a parsed manifest: `check_repo_path` decodes every
    /// `paths.*` value during validation, so a `Manifest` that exists at all
    /// has entries that decode.
    ///
    /// [`floor_entries`]: Manifest::floor_entries
    pub fn floor_entries_raw(&self) -> Vec<Vec<u8>> {
        self.floor_entries()
            .into_iter()
            .map(|e| spine_canon::unesc(e).expect("validated"))
            .collect()
    }

    pub fn files(&self) -> Vec<FileRecord> {
        let Some(Value::Arr(items)) = self.value.get("files") else {
            return Vec::new();
        };
        items
            .iter()
            .map(|record| {
                let path = record
                    .get("path")
                    .and_then(Value::as_str)
                    .expect("validated");
                let (file_path, region) = grammar::split_region(path);
                FileRecord {
                    path: path.to_string(),
                    file_path: file_path.to_string(),
                    region: region.map(str::to_string),
                    owner: record
                        .get("owner")
                        .and_then(Value::as_str)
                        .and_then(Owner::parse)
                        .expect("validated"),
                    blob: record
                        .get("blob")
                        .and_then(Value::as_str)
                        .expect("validated")
                        .to_string(),
                    template: record.get("template").and_then(Value::as_str).map(|t| {
                        let (name, v) = grammar::parse_template_ref(t).expect("validated");
                        (name.to_string(), v)
                    }),
                    base: record
                        .get("base")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                }
            })
            .collect()
    }

    pub fn template_version(&self, name: &str) -> Option<u64> {
        self.value
            .get("templates")
            .and_then(|t| t.get(name))
            .and_then(Value::as_u64)
    }

    pub fn resign_version(&self, name: &str) -> Option<u64> {
        self.value
            .get("resign")
            .and_then(|t| t.get(name))
            .and_then(Value::as_u64)
    }

    // ---- validation ------------------------------------------------------

    fn validate(&self, repo_format: Option<ObjectFormat>) -> Result<()> {
        let root = match &self.value {
            Value::Obj(members) => members,
            other => {
                return Err(Refusal::new(
                    Status::FrozenMemberType,
                    format!("root is a {}", other.kind()),
                ));
            }
        };

        check_profile(&self.value, 1)?;

        // §3.11's order: the frozen twelve are checked for presence and type
        // before any value grammar, because a missing frozen field is what
        // makes an old binary unable to judge a new manifest at all.
        for (name, required) in [
            ("manifest_version", true),
            ("cli", true),
            ("schema", true),
            ("envelope", true),
            ("object_format", true),
            ("params", true),
            ("paths", true),
            ("files", true),
            ("repo", true),
            ("templates", true),
            ("resign", true),
        ] {
            if required && self.value.get(name).is_none() {
                return Err(Refusal::new(Status::FrozenMemberMissing, name));
            }
        }

        typed(&self.value, "manifest_version", is_int)?;
        typed(&self.value, "schema", is_int)?;
        typed(&self.value, "envelope", is_int)?;
        typed(&self.value, "repo", is_str)?;
        typed(&self.value, "object_format", is_str)?;
        typed(&self.value, "cli", is_obj)?;
        typed(&self.value, "params", is_obj)?;
        typed(&self.value, "paths", is_obj)?;
        typed(&self.value, "templates", is_obj)?;
        typed(&self.value, "resign", is_obj)?;
        typed(&self.value, "files", is_arr)?;

        if self.manifest_version() < 1 {
            return Err(Refusal::new(Status::FrozenMemberType, "manifest_version"));
        }

        grammar::check_repo(self.repo())?;

        // cli
        let cli = self.value.get("cli").expect("present");
        typed(cli, "version", is_str)
            .map_err(|_| Refusal::new(Status::CliVersionOutOfGrammar, "cli.version"))?;
        typed(cli, "dist_hash", is_str)
            .map_err(|_| Refusal::new(Status::DistHashMalformed, "cli.dist_hash"))?;
        grammar::check_cli_version(self.cli_version())?;
        grammar::check_dist_hash(self.cli_dist_hash())?;

        // object_format, and G16 check 8's cross-check against the repository.
        let format = self
            .value
            .get("object_format")
            .and_then(Value::as_str)
            .and_then(ObjectFormat::parse)
            .ok_or_else(|| Refusal::new(Status::ObjectFormatUnknown, "object_format"))?;
        if let Some(actual) = repo_format
            && actual != format
        {
            return Err(Refusal::new(
                Status::ObjectFormatMismatch,
                format!(
                    "manifest says {}, repository is {}",
                    format.as_str(),
                    actual.as_str()
                ),
            ));
        }

        self.validate_params()?;
        self.validate_paths()?;
        self.validate_files(format)?;
        self.validate_templates_and_resign()?;

        let _ = root;
        Ok(())
    }

    fn validate_params(&self) -> Result<()> {
        let params = self.value.get("params").expect("present");

        typed(params, "trunk", is_str)
            .map_err(|_| Refusal::new(Status::TrunkNotABranchName, "params.trunk"))?;
        grammar::check_branch_name(self.trunk())?;

        if let Some(raw) = params.get("isolation") {
            let s = raw
                .as_str()
                .ok_or_else(|| Refusal::new(Status::IsolationUnknown, "params.isolation"))?;
            if Isolation::parse(s).is_none() {
                return Err(Refusal::new(Status::IsolationUnknown, "params.isolation"));
            }
        }

        let ci = params
            .get("ci")
            .and_then(Value::as_str)
            .ok_or_else(|| Refusal::new(Status::CiUnknown, "params.ci"))?;
        if !CI_PROVIDERS.contains(&ci) {
            return Err(Refusal::new(Status::CiUnknown, "params.ci"));
        }

        let langs = params
            .get("langs")
            .and_then(Value::as_arr)
            .ok_or_else(|| Refusal::new(Status::LangsUnknown, "params.langs"))?;
        if langs.is_empty() {
            return Err(Refusal::new(Status::LangsEmpty, "params.langs"));
        }
        let mut previous: Option<&str> = None;
        for item in langs {
            let s = item
                .as_str()
                .ok_or_else(|| Refusal::new(Status::LangsUnknown, "params.langs"))?;
            if !V1_LANGS.contains(&s) {
                return Err(Refusal::new(
                    Status::LangsUnknown,
                    format!("params.langs {s:?}"),
                ));
            }
            // Sorted ascending by bytes, deduplicated (MF §3.3). Not merely a
            // tidiness rule: two spellings of one language set are two
            // manifests, and G16 compares blobs.
            if let Some(prev) = previous
                && prev >= s
            {
                return Err(Refusal::new(
                    Status::ManifestNoncanonical,
                    "params.langs is unsorted or has a duplicate",
                ));
            }
            previous = Some(s);
        }

        if let Some(raw) = params.get("timeout") {
            let t = raw
                .as_u64()
                .ok_or_else(|| Refusal::new(Status::TimeoutOutOfRange, "params.timeout"))?;
            if !(1..=86_400).contains(&t) {
                return Err(Refusal::new(Status::TimeoutOutOfRange, "params.timeout"));
            }
        }
        Ok(())
    }

    fn validate_paths(&self) -> Result<()> {
        let Some(Value::Obj(members)) = self.value.get("paths") else {
            return Err(Refusal::new(Status::FrozenMemberType, "paths"));
        };
        for (key, value) in members {
            if RESERVED_PATHS_KEYS.contains(&key.as_str()) {
                return Err(Refusal::new(
                    Status::ReservedMemberName,
                    format!("paths.{key}"),
                ));
            }
            match value {
                Value::Str(s) => grammar::check_repo_path(s, &format!("paths.{key}"))?,
                Value::Arr(items) => {
                    // MF §3.4: two or more, sorted by `esc` bytes, no
                    // duplicates. A one-element array or an empty one is
                    // `manifest-noncanonical`, because the canonical shape of a
                    // single entry is a string.
                    if items.len() < 2 {
                        return Err(Refusal::new(
                            Status::ManifestNoncanonical,
                            format!("paths.{key} is an array of {}", items.len()),
                        ));
                    }
                    let mut previous: Option<&str> = None;
                    for item in items {
                        let s = item.as_str().ok_or_else(|| {
                            Refusal::new(Status::PathsValueMalformed, format!("paths.{key}"))
                        })?;
                        grammar::check_repo_path(s, &format!("paths.{key}"))?;
                        if let Some(prev) = previous
                            && prev >= s
                        {
                            return Err(Refusal::new(
                                Status::ManifestNoncanonical,
                                format!("paths.{key} is unsorted or has a duplicate"),
                            ));
                        }
                        previous = Some(s);
                    }
                }
                _ => {
                    return Err(Refusal::new(
                        Status::PathsValueMalformed,
                        format!("paths.{key} is a {}", value.kind()),
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_files(&self, format: ObjectFormat) -> Result<()> {
        let Some(Value::Arr(items)) = self.value.get("files") else {
            return Err(Refusal::new(Status::FrozenMemberType, "files"));
        };
        let mut previous: Option<&str> = None;
        for record in items {
            if !matches!(record, Value::Obj(_)) {
                return Err(Refusal::new(Status::FrozenMemberType, "files[]"));
            }
            let path = record
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| Refusal::new(Status::FrozenMemberMissing, "files[].path"))?;

            let (file_path, region) = grammar::split_region(path);
            grammar::check_repo_path(file_path, &format!("files[].path {path:?}"))?;
            if let Some(key) = region {
                grammar::check_region_key(key)?;
            }

            // Sorted ascending by the `esc`-encoded path bytes, no duplicates
            // (MF §3.5). Equality is `files-duplicate-path`; out-of-order is
            // `manifest-noncanonical` — two different tokens, and §3.11's order
            // puts the noncanonical one first.
            if let Some(prev) = previous {
                if prev == path {
                    return Err(Refusal::new(
                        Status::FilesDuplicatePath,
                        format!("files[].path {path:?}"),
                    ));
                }
                if prev > path {
                    return Err(Refusal::new(
                        Status::ManifestNoncanonical,
                        "files is unsorted",
                    ));
                }
            }
            previous = Some(path);

            let owner = record
                .get("owner")
                .and_then(Value::as_str)
                .and_then(Owner::parse)
                .ok_or_else(|| {
                    Refusal::new(Status::OwnerUnknown, format!("files[].owner for {path:?}"))
                })?;

            let blob = record.get("blob").and_then(Value::as_str).ok_or_else(|| {
                Refusal::new(Status::BlobMalformed, format!("files[].blob for {path:?}"))
            })?;
            grammar::check_blob(blob, format, &format!("files[].blob for {path:?}"))?;

            if let Some(template) = record.get("template") {
                let s = template.as_str().ok_or_else(|| {
                    Refusal::new(
                        Status::TemplateMalformed,
                        format!("files[].template for {path:?}"),
                    )
                })?;
                let (name, version) = grammar::parse_template_ref(s)?;
                // MF §3.6 / G16 check 7: the key must exist and its value must
                // equal the version after `@`.
                match self.template_version(name) {
                    Some(current) if current == version => {}
                    Some(_) => {
                        return Err(Refusal::new(
                            Status::TemplateVersionMismatch,
                            format!("files[].template {s:?}"),
                        ));
                    }
                    None => {
                        return Err(Refusal::new(
                            Status::TemplateVersionMismatch,
                            format!("no templates key for {name:?}"),
                        ));
                    }
                }
            }

            // MF §3.5: `base` is present **iff** the class is `user-modified`.
            match (owner, record.get("base")) {
                (Owner::UserModified, Some(base)) => {
                    let s = base.as_str().ok_or_else(|| {
                        Refusal::new(Status::BlobMalformed, format!("files[].base for {path:?}"))
                    })?;
                    grammar::check_blob(s, format, &format!("files[].base for {path:?}"))?;
                }
                (Owner::UserModified, None) => {
                    return Err(Refusal::new(
                        Status::FilesBaseMisplaced,
                        format!("user-modified {path:?} has no base"),
                    ));
                }
                (_, Some(_)) => {
                    return Err(Refusal::new(
                        Status::FilesBaseMisplaced,
                        format!("{} {path:?} carries a base", owner.as_str()),
                    ));
                }
                (_, None) => {}
            }
        }
        Ok(())
    }

    fn validate_templates_and_resign(&self) -> Result<()> {
        let Some(Value::Obj(templates)) = self.value.get("templates") else {
            return Err(Refusal::new(Status::FrozenMemberType, "templates"));
        };
        for (name, version) in templates {
            let _ = grammar::parse_template_ref(&format!("{name}@1"))?;
            let v = version.as_u64().ok_or_else(|| {
                Refusal::new(Status::TemplateMalformed, format!("templates.{name}"))
            })?;
            if v < 1 {
                return Err(Refusal::new(
                    Status::TemplateMalformed,
                    format!("templates.{name}"),
                ));
            }
        }

        let Some(Value::Obj(resign)) = self.value.get("resign") else {
            return Err(Refusal::new(Status::FrozenMemberType, "resign"));
        };
        for (name, floor) in resign {
            // MF §3.6, TM §7.2: intent-only.
            if !RESIGN_KEYS.contains(&name.as_str()) {
                return Err(Refusal::new(
                    Status::ResignKeyUnknown,
                    format!("resign.{name}"),
                ));
            }
            let floor = floor.as_u64().ok_or_else(|| {
                Refusal::new(Status::ResignFloorAboveCurrent, format!("resign.{name}"))
            })?;
            let current = self.template_version(name).ok_or_else(|| {
                Refusal::new(Status::ResignKeyUnknown, format!("no templates.{name}"))
            })?;
            // MF §3.6, G16 check 11: `1 <= resign[v] <= templates[v]`.
            // An inversion is outright; a *decrease across a landing* is the
            // separate, coverable finding and is not decidable from one file.
            if floor < 1 || floor > current {
                return Err(Refusal::new(
                    Status::ResignFloorAboveCurrent,
                    format!("resign.{name} = {floor}, templates.{name} = {current}"),
                ));
            }
        }
        Ok(())
    }
}

fn is_int(v: &Value) -> bool {
    matches!(v, Value::Int(_))
}
fn is_str(v: &Value) -> bool {
    matches!(v, Value::Str(_))
}
fn is_obj(v: &Value) -> bool {
    matches!(v, Value::Obj(_))
}
fn is_arr(v: &Value) -> bool {
    matches!(v, Value::Arr(_))
}

fn typed(parent: &Value, name: &str, ok: fn(&Value) -> bool) -> Result<()> {
    match parent.get(name) {
        Some(v) if ok(v) => Ok(()),
        Some(v) => Err(Refusal::new(
            Status::FrozenMemberType,
            format!("{name} is a {}", v.kind()),
        )),
        None => Err(Refusal::new(Status::FrozenMemberMissing, name)),
    }
}

/// MF §2.2's value profile and resource bounds.
fn check_profile(value: &Value, depth: usize) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(Refusal::new(Status::ManifestTooLarge, "nesting"));
    }
    match value {
        // "Null | Never emitted, never accepted."
        Value::Null => Err(Refusal::new(Status::ManifestUnknownMemberValue, "null")),
        Value::Str(s) => {
            if s.len() > MAX_STRING_BYTES {
                return Err(Refusal::new(Status::ManifestTooLarge, "string"));
            }
            // "Strings | ASCII only after `esc`: every character is in
            // U+0020..U+007E."
            if !s.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
                return Err(Refusal::new(
                    Status::ManifestUnknownMemberValue,
                    "string is not ASCII after esc",
                ));
            }
            Ok(())
        }
        Value::Arr(items) => {
            if items.len() > MAX_ARRAY_ELEMENTS {
                return Err(Refusal::new(Status::ManifestTooLarge, "array"));
            }
            items.iter().try_for_each(|v| check_profile(v, depth + 1))
        }
        Value::Obj(members) => {
            if members.len() > MAX_OBJECT_MEMBERS {
                return Err(Refusal::new(Status::ManifestTooLarge, "object"));
            }
            for (name, v) in members {
                check_member_name(name)?;
                check_profile(v, depth + 1)?;
            }
            Ok(())
        }
        Value::Bool(_) | Value::Int(_) => Ok(()),
    }
}

/// MF §2.2: `^[a-z][a-z0-9_-]{0,63}$` — **wider than GR §2.2 by one byte**,
/// because `templates` and `resign` are keyed by template names that carry `-`.
fn check_member_name(name: &str) -> Result<()> {
    let mut bytes = name.bytes();
    let ok = match bytes.next() {
        Some(first) if first.is_ascii_lowercase() => {
            name.len() <= 64
                && bytes
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
        }
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(Refusal::new(
            Status::MemberNameOutOfGrammar,
            format!("member {name:?}"),
        ))
    }
}
