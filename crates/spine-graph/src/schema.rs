//! The traceability graph's schema: nine node kinds, fifteen edge kinds, their
//! attributes, the node-id grammar, and the provenance citation.
//!
//! PB §6.1's iron rule governs everything here: *"Every graph in spine-kit is a
//! **cache**: gitignored, deleted at will, deterministically rebuilt from the
//! repo by one command."* Nothing in this module can be authored — it is a
//! vocabulary for what an indexer derived, and the one thing it enforces is
//! PB §6.1's corollary, the provenance law: *"every node and edge must cite its
//! source. An edge that cannot say where it came from does not exist."* Hence
//! [`Src`] is not an `Option` anywhere.
//!
//! The domains are closed on purpose. DM §3.2: *"The schema is closed: forward
//! compatibility is bought with a version bump, not with tolerance … a tolerant
//! reader and a strict one produce different bytes over the same document, and
//! the whole artifact is compared by bytes."*

use crate::status::{Refusal, Result, Status};
use spine_canon::{ObjectFormat, Value, esc, sha256_hex};

/// PB §6.2's `PRAGMA user_version` — the *store's* schema version, recorded in
/// the dump header (DM §3.1).
///
/// DM §3.3 keeps this separate from [`crate::dump::DUMP_VERSION`] because they
/// move independently: "a store-schema change that adds an excluded attr
/// changes `schema_version` and not `dump_version`."
pub const SCHEMA_VERSION: u64 = 7;

// ---------------------------------------------------------------------------
// Node kinds
// ---------------------------------------------------------------------------

/// DM §5.1's closed set of nine, which is PB §6.2's.
///
/// Declaration order is the wire tokens' byte order, which is also the first
/// component of the node sort key (DM §6.2). The key is built from the tokens
/// themselves rather than from the discriminant, so this coincidence is a
/// convenience and never load-bearing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeKind {
    Ac,
    Adr,
    Approval,
    Changeset,
    CodeUnit,
    Constitution,
    Intent,
    Signer,
    Test,
}

impl NodeKind {
    pub fn token(self) -> &'static str {
        match self {
            NodeKind::Ac => "ac",
            NodeKind::Adr => "adr",
            NodeKind::Approval => "approval",
            NodeKind::Changeset => "changeset",
            NodeKind::CodeUnit => "code_unit",
            NodeKind::Constitution => "constitution",
            NodeKind::Intent => "intent",
            NodeKind::Signer => "signer",
            NodeKind::Test => "test",
        }
    }

    pub const ALL: [NodeKind; 9] = [
        NodeKind::Ac,
        NodeKind::Adr,
        NodeKind::Approval,
        NodeKind::Changeset,
        NodeKind::CodeUnit,
        NodeKind::Constitution,
        NodeKind::Intent,
        NodeKind::Signer,
        NodeKind::Test,
    ];

    /// The attrs DM §7.2 gives this kind, and the type of each.
    ///
    /// `None` means the name is not in the kind's row, which DM §7.1 makes a
    /// refusal: *"An attr name … comes from the closed set of §7.2; anything
    /// else refuses the dump (`attrs-out-of-profile`, exit 3)."*
    ///
    /// Two absences are deliberate and are the two DM §8.4 excludes:
    /// `test.result_at` (volatile, and the kind's only attr, so every `test`
    /// node carries `{}`) and `verified_by.introduced_by` — see [`EdgeKind`].
    /// DM §7.2's **presence** column, the `always` rows only.
    ///
    /// Transcribed from the table rather than inferred from `attr_type`, which
    /// says nothing about presence: a kind's attr set and its *mandatory* attr
    /// set are different facts and §7.2 gives them in different columns.
    ///
    /// An `always` attr that is absent is a node the dump can hold and no
    /// reader can use. For `changeset.landing` in particular the absence was
    /// load-bearing: DM §7.2's member-changeset rule is written over the value
    /// being present and `false`, so a changeset omitting it escaped the rule
    /// entirely and could carry seal fields it has no seal for.
    ///
    /// `changeset` lists only `landing`, because §7.2's second row makes every
    /// other changeset attr conditional — "*(the rest)* … iff `landing` is
    /// `true`". `test` lists none: "`{}` always".
    pub fn always_present_attrs(self) -> &'static [&'static str] {
        match self {
            NodeKind::Intent => &[
                "status",
                "title",
                "template",
                "blob",
                "reopen_count",
                "late_reopen_count",
                "landing",
                "base",
            ],
            NodeKind::Changeset => &["landing"],
            NodeKind::Approval => &["event", "role", "principal", "verified"],
            NodeKind::Signer => &["roles", "fingerprint", "valid_from"],
            // "A kind PB §6.2 does not give attrs for has none in the dump":
            // `ac`, `adr`, `code_unit`, `constitution` — and `test`, whose only
            // attr §8.4 excludes.
            _ => &[],
        }
    }

    pub fn attr_type(self, name: &str) -> Option<AttrType> {
        use AttrType::{Bool, Int, Str, StrArr};
        match (self, name) {
            (NodeKind::Intent, "status") => Some(Str),
            (NodeKind::Intent, "owner") => Some(Str),
            (NodeKind::Intent, "title") => Some(Str),
            (NodeKind::Intent, "template") => Some(Str),
            (NodeKind::Intent, "blob") => Some(Str),
            (NodeKind::Intent, "signer") => Some(Str),
            (NodeKind::Intent, "reopen_count") => Some(Int),
            (NodeKind::Intent, "late_reopen_count") => Some(Int),
            (NodeKind::Intent, "landing") => Some(Str),
            (NodeKind::Intent, "base") => Some(Str),

            (NodeKind::Changeset, "landing") => Some(Bool),
            (NodeKind::Changeset, "lane") => Some(Str),
            (NodeKind::Changeset, "event") => Some(Str),
            (NodeKind::Changeset, "strategy") => Some(Str),
            (NodeKind::Changeset, "base") => Some(Str),
            (NodeKind::Changeset, "head") => Some(Str),
            (NodeKind::Changeset, "tree") => Some(Str),
            (NodeKind::Changeset, "seal_principal") => Some(Str),
            (NodeKind::Changeset, "seal_verified") => Some(Bool),
            (NodeKind::Changeset, "report_sha256") => Some(Str),
            (NodeKind::Changeset, "threat") => Some(Str),
            (NodeKind::Changeset, "profile") => Some(Str),
            (NodeKind::Changeset, "tool_version") => Some(Str),
            (NodeKind::Changeset, "git_version") => Some(Str),
            (NodeKind::Changeset, "mode") => Some(Str),
            (NodeKind::Changeset, "unattested") => Some(Bool),
            (NodeKind::Changeset, "resealed") => Some(Bool),

            (NodeKind::Approval, "event") => Some(Str),
            (NodeKind::Approval, "role") => Some(Str),
            (NodeKind::Approval, "principal") => Some(Str),
            (NodeKind::Approval, "verified") => Some(Bool),
            (NodeKind::Approval, "blob") => Some(Str),
            (NodeKind::Approval, "base") => Some(Str),
            (NodeKind::Approval, "head") => Some(Str),
            (NodeKind::Approval, "tree") => Some(Str),
            (NodeKind::Approval, "class") => Some(Str),
            (NodeKind::Approval, "rounds") => Some(Int),
            (NodeKind::Approval, "total_rounds") => Some(Int),
            (NodeKind::Approval, "reopens") => Some(Int),
            (NodeKind::Approval, "red") => Some(Str),
            (NodeKind::Approval, "freeze") => Some(Str),
            (NodeKind::Approval, "wires") => Some(StrArr),
            (NodeKind::Approval, "voided_by") => Some(Str),
            (NodeKind::Approval, "void_reason") => Some(Str),

            (NodeKind::Signer, "roles") => Some(StrArr),
            (NodeKind::Signer, "fingerprint") => Some(Str),
            (NodeKind::Signer, "valid_from") => Some(Str),
            (NodeKind::Signer, "valid_to") => Some(Str),

            // DM §7.2, §13.9: "A kind PB §6.2 does not give attrs for has none
            // in the dump." `ac`, `adr`, `code_unit` and `constitution` carry
            // `{}`; so does `test`, once §8.4 removes `result_at`.
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Edge kinds
// ---------------------------------------------------------------------------

/// DM §5.3's closed set of fifteen, which is PB §6.2's.
///
/// `Exercises` is a member of the domain and is emitted by nothing: DM §5.3
/// says it "is never emitted in v1", DM §8.3 says its source is a CI coverage
/// report and so it "stays excluded when it ships". The enum has fifteen
/// members and the emitter fourteen, which is exactly DM §17 item 9's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeKind {
    Approves,
    AttestedBy,
    BuiltUnder,
    Declares,
    Exercises,
    Freezes,
    HasAc,
    Implements,
    Modifies,
    Protects,
    Reverts,
    SignedBy,
    Supersedes,
    SupersededBy,
    VerifiedBy,
}

impl EdgeKind {
    pub fn token(self) -> &'static str {
        match self {
            EdgeKind::Approves => "approves",
            EdgeKind::AttestedBy => "attested_by",
            EdgeKind::BuiltUnder => "built_under",
            EdgeKind::Declares => "declares",
            EdgeKind::Exercises => "exercises",
            EdgeKind::Freezes => "freezes",
            EdgeKind::HasAc => "has_ac",
            EdgeKind::Implements => "implements",
            EdgeKind::Modifies => "modifies",
            EdgeKind::Protects => "protects",
            EdgeKind::Reverts => "reverts",
            EdgeKind::SignedBy => "signed_by",
            EdgeKind::Supersedes => "supersedes",
            EdgeKind::SupersededBy => "superseded_by",
            EdgeKind::VerifiedBy => "verified_by",
        }
    }

    pub const ALL: [EdgeKind; 15] = [
        EdgeKind::Approves,
        EdgeKind::AttestedBy,
        EdgeKind::BuiltUnder,
        EdgeKind::Declares,
        EdgeKind::Exercises,
        EdgeKind::Freezes,
        EdgeKind::HasAc,
        EdgeKind::Implements,
        EdgeKind::Modifies,
        EdgeKind::Protects,
        EdgeKind::Reverts,
        EdgeKind::SignedBy,
        EdgeKind::Supersedes,
        EdgeKind::SupersededBy,
        EdgeKind::VerifiedBy,
    ];

    /// The endpoint kinds DM §5.3's direction paragraph fixes, as
    /// `(from, to)`. `None` where the corpus does not fix them.
    ///
    /// DM §13.4 claims "§5.3's paragraph fixes all fifteen"; it names twelve.
    /// `supersedes`, `superseded_by` and `exercises` are unfixed there — the
    /// two supersession kinds are given a direction by this crate (see
    /// [`EdgeKind::Supersedes`] in `store`'s doc comment and the crate's
    /// derived-decision list), and `exercises` is never emitted, so its
    /// endpoints never arise.
    ///
    /// `Approves` has two legal `to` kinds — PB §6.2: `approves` "names the
    /// intent for every line carrying an id and the landing changeset `cs:<L>`
    /// for those that do not" — so it is reported as `None` rather than forced.
    pub fn endpoints(self) -> Option<(NodeKind, NodeKind)> {
        match self {
            // "`verified_by` runs **test → ac**: PB §6.3 G5 fails on 'a
            // `verified_by` edge to a nonexistent AC (typo'd pragma)', so the
            // AC is `to_id`." (DM §5.3) — however oddly the name then reads.
            EdgeKind::VerifiedBy => Some((NodeKind::Test, NodeKind::Ac)),
            EdgeKind::HasAc => Some((NodeKind::Intent, NodeKind::Ac)),
            EdgeKind::Declares => Some((NodeKind::Intent, NodeKind::CodeUnit)),
            EdgeKind::BuiltUnder => Some((NodeKind::Intent, NodeKind::Constitution)),
            EdgeKind::Implements => Some((NodeKind::Changeset, NodeKind::Intent)),
            EdgeKind::Modifies => Some((NodeKind::Changeset, NodeKind::CodeUnit)),
            EdgeKind::SignedBy => Some((NodeKind::Approval, NodeKind::Signer)),
            EdgeKind::AttestedBy => Some((NodeKind::Changeset, NodeKind::Signer)),
            EdgeKind::Protects => Some((NodeKind::Constitution, NodeKind::CodeUnit)),
            EdgeKind::Reverts => Some((NodeKind::Changeset, NodeKind::Changeset)),
            // `freezes` runs approval → code_unit (with `oid`) or approval →
            // test (DM §5.3), so only its `from` is fixed.
            EdgeKind::Freezes | EdgeKind::Approves => None,
            EdgeKind::Supersedes | EdgeKind::SupersededBy => {
                Some((NodeKind::Intent, NodeKind::Intent))
            }
            EdgeKind::Exercises => None,
        }
    }

    /// DM §7.2's edge table. The two exclusions are visible as absences:
    /// `verified_by.introduced_by` (DM §8.5 clause 1 — `git blame` "has no
    /// specified output contract", so including it "would make a routine `git`
    /// upgrade on the runner turn the next landing into
    /// `reconstruction-failed`") and every attr of the eight kinds PB §6.2
    /// gives none.
    pub fn attr_type(self, name: &str) -> Option<AttrType> {
        use AttrType::{Bool, Str};
        match (self, name) {
            (EdgeKind::Declares, "polarity") => Some(Str),
            (EdgeKind::Implements, "role") => Some(Str),
            (EdgeKind::Implements, "provisional") => Some(Bool),
            (EdgeKind::Implements, "verified") => Some(Bool),
            (EdgeKind::VerifiedBy, "attributed") => Some(Bool),
            (EdgeKind::Freezes, "oid") => Some(Str),
            (EdgeKind::Protects, "floor") => Some(Bool),
            (EdgeKind::Reverts, "partial") => Some(Bool),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// attrs
// ---------------------------------------------------------------------------

/// DM §7.1's value profile: *"An attr value is a **string**, a non-negative
/// **integer** in `[0, 2^53 − 1]`, a **boolean**, or an **array of strings**.
/// Never an object, never `null`, never a number outside that range."*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrType {
    Str,
    Int,
    Bool,
    StrArr,
}

/// DM §2.3: `0 ≤ n ≤ 2^53 − 1`. The bound is JSON's interoperable integer
/// range, not Rust's — a value above it round-trips through a double-precision
/// reader as a different number, and the two sides of G10 need not be Rust.
pub const MAX_INT: u64 = (1u64 << 53) - 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrValue {
    Str(String),
    Int(u64),
    Bool(bool),
    StrArr(Vec<String>),
}

impl AttrValue {
    fn type_of(&self) -> AttrType {
        match self {
            AttrValue::Str(_) => AttrType::Str,
            AttrValue::Int(_) => AttrType::Int,
            AttrValue::Bool(_) => AttrType::Bool,
            AttrValue::StrArr(_) => AttrType::StrArr,
        }
    }

    fn to_value(&self) -> Value {
        match self {
            AttrValue::Str(s) => Value::str(s.clone()),
            AttrValue::Int(n) => Value::Int(*n),
            AttrValue::Bool(b) => Value::Bool(*b),
            AttrValue::StrArr(items) => Value::arr(items.iter().map(|s| Value::str(s.clone()))),
        }
    }
}

/// A record's `attrs` object.
///
/// DM §7.1: *"**`attrs` is always present.** A kind with no attrs emits `{}`.
/// `{}` is a value; an omitted `attrs` member is not a legal record. This
/// removes the whole 'did you emit `{}` or omit it' divergence class, which is
/// precisely the class G10 punishes terminally."* So [`Attrs::default`] is a
/// legal, emittable value and never a missing one.
///
/// Members are held in insertion order; JCS sorts them at serialization time
/// (DM §10 rule 4: "Never insertion order, never a hand-written order").
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attrs {
    members: Vec<(String, AttrValue)>,
}

impl Attrs {
    pub fn new() -> Self {
        Attrs::default()
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&AttrValue> {
        self.members.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &AttrValue)> {
        self.members.iter().map(|(k, v)| (k.as_str(), v))
    }

    fn put(mut self, name: &str, value: AttrValue) -> Self {
        self.members.push((name.to_string(), value));
        self
    }

    /// An attr whose value is already ASCII by construction — a closed
    /// enumeration, an oid, a `sha256:` digest, a version string.
    ///
    /// DM §2.4: `esc` "does **not** apply to object ids, integers, booleans, or
    /// the closed enumerations of §7.2, which are already ASCII and for which
    /// `esc` is the identity". Kept separate from [`Attrs::bytes`] so a reader
    /// can see, at each call site, which side of that sentence a value is on.
    pub fn str(self, name: &str, value: impl Into<String>) -> Self {
        self.put(name, AttrValue::Str(value.into()))
    }

    /// An attr whose §7.2 row says *bytes* — a path, a principal, a title, a
    /// reason. `esc`-encoded here, per DM §2.4, and never normalized.
    pub fn bytes(self, name: &str, value: &[u8]) -> Self {
        self.put(name, AttrValue::Str(esc(value)))
    }

    pub fn int(self, name: &str, value: u64) -> Self {
        self.put(name, AttrValue::Int(value))
    }

    pub fn bool(self, name: &str, value: bool) -> Self {
        self.put(name, AttrValue::Bool(value))
    }

    /// A string array. DM §2.3: "Elements are strings only. Order is fixed per
    /// attr by §7.2" — and the two arrays in the schema fix it differently.
    /// `signer.roles` is "ascending by bytes"; `approval.wires` is *"in the
    /// line's order … Not re-sorted here: the signed line's order is the fact,
    /// and a dump that re-sorted it would hide a non-conforming review rather
    /// than reproduce it."* Neither is sorted by this method; the caller owns
    /// the order because only the caller knows which rule applies.
    pub fn arr(self, name: &str, items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.put(
            name,
            AttrValue::StrArr(items.into_iter().map(Into::into).collect()),
        )
    }

    /// The `attrs` object as a canonicalizable value.
    pub fn to_value(&self) -> Value {
        Value::Obj(
            self.members
                .iter()
                .map(|(k, v)| (k.clone(), v.to_value()))
                .collect(),
        )
    }

    /// DM §7.1 and §2.3, applied to one record's attrs.
    ///
    /// `lookup` is the kind's row from [`NodeKind::attr_type`] or
    /// [`EdgeKind::attr_type`]. Everything it does not know is refused, which
    /// is what makes `verified_by.introduced_by` and `test.result_at`
    /// unrepresentable rather than merely discouraged.
    pub(crate) fn check_profile(
        &self,
        lookup: impl Fn(&str) -> Option<AttrType>,
        where_: &str,
    ) -> Result<()> {
        let bad = |what: &str| {
            Err(Refusal::new(
                Status::AttrsOutOfProfile,
                format!("{where_}: {what}"),
            ))
        };
        let mut seen: Vec<&str> = Vec::new();
        for (name, value) in &self.members {
            // DM §2.3: member names "Match `^[a-z][a-z0-9_]*$`. ASCII only, so
            // JCS's UTF-16 ordering reduces to byte ordering." A name outside
            // it would make the two orders differ.
            if !is_member_name(name) {
                return bad(&format!("member name {name:?}"));
            }
            if seen.contains(&name.as_str()) {
                // DM §2.3, "Duplicate names | Invalid."
                return bad(&format!("duplicate member {name:?}"));
            }
            seen.push(name);
            match lookup(name) {
                None => return bad(&format!("unknown attr {name:?}")),
                Some(expected) if expected != value.type_of() => {
                    return bad(&format!("attr {name:?} has the wrong type"));
                }
                Some(_) => {}
            }
            match value {
                AttrValue::Int(n) if *n > MAX_INT => {
                    return bad(&format!("attr {name:?} exceeds 2^53-1"));
                }
                AttrValue::Str(s) if !is_ascii_printable(s) => {
                    return bad(&format!("attr {name:?} is not ASCII after esc"));
                }
                AttrValue::StrArr(items) => {
                    if let Some(bad_item) = items.iter().find(|s| !is_ascii_printable(s)) {
                        return bad(&format!("attr {name:?} element {bad_item:?} is not ASCII"));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// DM §2.3's member-name grammar, `^[a-z][a-z0-9_]*$`.
pub fn is_member_name(s: &str) -> bool {
    let mut bytes = s.bytes();
    match bytes.next() {
        Some(b) if b.is_ascii_lowercase() => {}
        _ => return false,
    }
    bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// DM §2.3, Strings row: *"ASCII only after `esc` (§2.4): every character is in
/// `U+0020 … U+007E`."* Every string in a dump has passed through `esc`, so a
/// violation means a value bypassed the encoder.
pub fn is_ascii_printable(s: &str) -> bool {
    s.bytes().all(|b| (0x20..=0x7E).contains(&b))
}

// ---------------------------------------------------------------------------
// Node ids
// ---------------------------------------------------------------------------

/// DM §5.2's node-id constructors.
///
/// *"Every node id is `<repo>` + `/` + a per-kind local id, where `<repo>` is
/// the manifest's `repo` (PB §6.2: 'the prefix comes from the manifest's
/// `repo`, while trailers carry the bare id')."*
///
/// PB §6.2's own examples — `code:src/billing/`, `cs:abc123f`, `approval:5c9e…`
/// — carry no prefix and contradict the same paragraph's repo-scoping rule.
/// DM §13.3 reads them as abbreviations and prefixes every kind; DM §14 D5
/// files the contradiction. These constructors follow DM.
pub mod id {
    use super::*;

    fn scoped(repo: &str, local: &str) -> String {
        format!("{repo}/{local}")
    }

    /// `<repo>/INT-<n>` or `<repo>/BUG-<n>` — "the bare trailer id".
    pub fn intent(repo: &str, local: &str) -> String {
        scoped(repo, local)
    }

    /// `<repo>/<intent local id>/AC-<n>`.
    pub fn ac(repo: &str, intent_local: &str, n: u64) -> String {
        scoped(repo, &format!("{intent_local}/AC-{n}"))
    }

    /// `<repo>/test:<runner>:<runner-native function id>`.
    ///
    /// The runner qualifies the identity: DM §5.2, *"two runners collecting the
    /// same function id are two nodes, and merging them would let one runner's
    /// rename silently satisfy another's coverage."* The native id is a byte
    /// string from a runner's output, so it goes through `esc`.
    pub fn test(repo: &str, runner: &str, native_id: &[u8]) -> String {
        scoped(repo, &format!("test:{runner}:{}", esc(native_id)))
    }

    /// `<repo>/code:<esc(path bytes)>`; a trailing `/` means a directory.
    ///
    /// DM §2.4: *"A `code_unit` id is built from the repo-relative,
    /// `/`-separated path exactly as git stores it in the tree entry"* — the
    /// tree's bytes, never the filesystem's, because "a macOS runner reports
    /// NFD where git stores NFC, and a path that names nothing is worse than no
    /// path at all."
    pub fn code_unit(repo: &str, path: &[u8]) -> String {
        scoped(repo, &format!("code:{}", esc(path)))
    }

    /// `<repo>/cs:<full oid>`. DM §5.2: *"Oids are full, lowercase, never
    /// abbreviated. PB's `cs:abc123f` is display"* — an abbreviation's length
    /// is a function of the repository's object count, so it is not a fact
    /// about the commit.
    pub fn changeset(repo: &str, oid: &str) -> String {
        scoped(repo, &format!("cs:{oid}"))
    }

    /// `<repo>/approval:<64 lowercase hex>` — DM §5.2.1.
    ///
    /// The hash is over *"the exact bytes of the signed trailer line as the
    /// commit message carries it — from the first byte of the trailer name …
    /// through the last byte before its terminating LF, with no LF included."*
    ///
    /// It is **not** the approve line's `freeze=` digest, which PB §6.2's
    /// `approval:5c9e…` example reads as: DM §13.6 rejects that because it "is
    /// total over only one of the six `event` values" — a sign-off, review,
    /// reopen, withdrawal and upgrade have no freeze digest. `freeze=` is
    /// carried as the `freeze` attr instead.
    ///
    /// Keying a node on the line's bytes is safe because G13 makes the bytes
    /// unique: *"an event line byte-identical to an earlier one on the branch …
    /// is refused"*, outright, with no review that discharges it.
    pub fn approval(repo: &str, trailer_line: &[u8]) -> String {
        scoped(repo, &format!("approval:{}", sha256_hex(trailer_line)))
    }

    /// `<repo>/signer:<esc(principal bytes)>`.
    ///
    /// One id per principal, which is why MF §4.5 refuses a keyring listing two
    /// keys under one principal (`keyring-duplicate-principal`): they would be
    /// two signer nodes with one id, an unrepresentable graph.
    pub fn signer(repo: &str, principal: &[u8]) -> String {
        scoped(repo, &format!("signer:{}", esc(principal)))
    }

    /// `<repo>/<the ADR's own id, as its heading spells it>`.
    pub fn adr(repo: &str, adr_id: &str) -> String {
        scoped(repo, adr_id)
    }

    /// `<repo>/constitution:v<n>`.
    pub fn constitution(repo: &str, version: u64) -> String {
        scoped(repo, &format!("constitution:v{version}"))
    }
}

/// `changeset.tool_version` from the seal's `tool=` field.
///
/// DM §7.2 wants "the release version, e.g. `1.4.0`", and PB §11's seal carries
/// `tool=` — which `gate-report.md` §5 defines as *"`tool.version` + `"+"` +
/// `tool.dist_hash`"*, so `1.4.0+sha256:9f2e…` (CI §5.5's example spells it
/// out). The dump carries the left half.
///
/// **DERIVED, and the split is at the *last* `+sha256:`, not the first `+`.**
/// MF §3.2 admits `+` inside `cli.version` itself — the grammar is
/// `[0-9A-Za-z._+-]+` — so a release named `1.4.0+build.7` is legal, and a
/// first-`+` split would silently record `1.4.0` for it. `dist_hash` is always
/// `sha256:` + 64 hex (MF §3.2), so the last `+sha256:` is unambiguous. A
/// `tool=` with no `+sha256:` is returned whole rather than refused: the one
/// landing form GR §5 lets escape the pin is a rollback or uninstall, and a
/// dump records what the seal says.
pub fn tool_version_from_seal(tool: &str) -> &str {
    match tool.rfind("+sha256:") {
        Some(at) => &tool[..at],
        None => tool,
    }
}

/// DM §5.2, applied to a produced id: does it match its kind's row, and does it
/// begin `<repo>/`? (DM §17 item 10.)
///
/// A failure is `id-out-of-grammar`, exit 3 (DM §4.4). Refusing here rather
/// than emitting is the same argument the whole exit-3 family rests on: a
/// malformed id in a dump is a terminal G10 failure whose diff names the
/// ledger, not the defect.
pub fn check_node_id(repo: &str, kind: NodeKind, id: &str, format: ObjectFormat) -> Result<()> {
    let bad = || Err(Refusal::new(Status::IdOutOfGrammar, id.to_string()));

    if !is_ascii_printable(id) {
        return bad();
    }
    let Some(local) = id
        .strip_prefix(repo)
        .and_then(|rest| rest.strip_prefix('/'))
    else {
        return bad();
    };
    if local.is_empty() {
        return bad();
    }

    let ok = match kind {
        // "the bare trailer id: `INT-<n>` or `BUG-<n>`" (DM §5.2). The numeric
        // part is `intent-doc.md`'s grammar, not this document's; only the
        // shape DM prints is checked here.
        NodeKind::Intent => is_intent_local(local),
        // "`<intent local id>/AC-<n>`" — so the local id splits at the last
        // `/` into an intent local id and `AC-<n>`.
        NodeKind::Ac => match local.rsplit_once('/') {
            Some((intent, ac)) => {
                is_intent_local(intent) && ac.strip_prefix("AC-").is_some_and(is_decimal)
            }
            None => false,
        },
        // "`test:` + `<runner>` + `:` + `<runner-native function id>`", where
        // "`<runner>` contains no `:` … so `test:` + runner + `:` delimits
        // without a parse" (DM §5.2).
        NodeKind::Test => match local.strip_prefix("test:") {
            Some(rest) => match rest.split_once(':') {
                Some((runner, native)) => !runner.is_empty() && !native.is_empty(),
                None => false,
            },
            None => false,
        },
        // "`code:` + `esc(path bytes)`". The path's own grammar is the
        // manifest's and the intent doc's; an empty one names nothing.
        NodeKind::CodeUnit => local.strip_prefix("code:").is_some_and(|p| !p.is_empty()),
        NodeKind::Changeset => local
            .strip_prefix("cs:")
            .is_some_and(|oid| is_oid(oid, format)),
        // 64 lowercase hex regardless of `object_format`: DM §10 rule 10 makes
        // it a non-git digest, and PB §11's hash policy fixes SHA-256 for those.
        NodeKind::Approval => local
            .strip_prefix("approval:")
            .is_some_and(|hex| hex.len() == 64 && is_lower_hex(hex)),
        NodeKind::Signer => local.strip_prefix("signer:").is_some_and(|p| !p.is_empty()),
        NodeKind::Constitution => local.strip_prefix("constitution:v").is_some_and(is_decimal),
        // DERIVED: DM §5.2 gives `adr` the local id "the ADR's own id, as its
        // heading spells it" and fixes no grammar for it — PB §2.2's heading is
        // `# ADR-007: <decision>`, but an ADR id is the ADR's, not this
        // format's. Only non-emptiness is checked, so a repository that spells
        // an ADR id some other way still dumps.
        NodeKind::Adr => true,
    };
    if ok { Ok(()) } else { bad() }
}

fn is_intent_local(s: &str) -> bool {
    matches!(s.strip_prefix("INT-").or_else(|| s.strip_prefix("BUG-")), Some(n) if is_decimal(n))
}

fn is_decimal(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn is_lower_hex(s: &str) -> bool {
    s.bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// DM §10 rule 9: *"Object ids are lowercase hex at `object_format`'s full
/// length — 40 or 64 digits. Never abbreviated, never uppercase, never
/// prefixed."*
pub fn is_oid(s: &str, format: ObjectFormat) -> bool {
    s.len() == format.hex_len() && is_lower_hex(s)
}

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

/// PB §6.1's provenance grammar, closed.
///
/// PB §6.1: *"every node and edge must cite its source. An edge that cannot say
/// where it came from does not exist."* DM §5.4 carries that citation "verbatim,
/// in PB §6.1's grammar and no other", and refuses the whole dump for anything
/// outside it — *"a dump that quietly drops it produces a G10 diff whose cause
/// is invisible."*
///
/// Two productions are representable and never emitted, and both variants exist
/// so the grammar is complete rather than convenient: [`Src::FileLine`], "a line
/// of a file in the working tree", which DM §8.7 excludes because a dump is a
/// function of trees and refs; and [`Src::ShippedFloor`], which DM §8.5 excludes
/// because the floor list lives inside the binary and "including it would make
/// the dump a function of the release, which §3.4 forbids".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Src {
    /// `<path>:<line>` — a line of a file in the working tree. **Never emitted
    /// by a dump** (DM §5.4, §8.7).
    FileLine { path: Vec<u8>, line: u64 },
    /// `git:<sha>` — a whole commit: `modifies` from that commit's diff, a
    /// member changeset.
    Commit { sha: String },
    /// `git:<sha>:msg:L<n>` — a line of the envelope's fenced intent bytes:
    /// intent, ac, `has_ac`, `declares`, `built_under`.
    MessageLine { sha: String, line: u64 },
    /// `git:<sha>:trailer:<Name>` — a signed line: approval nodes, `approves`,
    /// `signed_by`, `attested_by`, `freezes`, the landing changeset.
    ///
    /// DM §14 D2 (OPEN) records that this production cannot name the second of
    /// two identical trailers; the recommended `#<n>` suffix is deliberately
    /// **not** implemented, because it would change the dump's bytes and so
    /// require a `dump_version` bump.
    Trailer { sha: String, name: String },
    /// `git:<sha>:patch-id` — `reverts`.
    PatchId { sha: String },
    /// `git:<sha>:<path>:<line>` — a line of a file at a commit: test nodes,
    /// `verified_by`, signer nodes, `protects` from `C-A2`, adr and
    /// constitution nodes.
    FileLineAt {
        sha: String,
        path: Vec<u8>,
        line: u64,
    },
    /// `spine:<version>:floor` — the release's floor list. **Never emitted by a
    /// dump** (DM §5.4, §8.3).
    ShippedFloor { version: String },
}

impl Src {
    /// The citation's bytes. `<path>` is `esc(path bytes)`; `<line>` and `<n>`
    /// are decimal (DM §5.4).
    pub fn render(&self) -> String {
        match self {
            Src::FileLine { path, line } => format!("{}:{line}", esc(path)),
            Src::Commit { sha } => format!("git:{sha}"),
            Src::MessageLine { sha, line } => format!("git:{sha}:msg:L{line}"),
            Src::Trailer { sha, name } => format!("git:{sha}:trailer:{name}"),
            Src::PatchId { sha } => format!("git:{sha}:patch-id"),
            Src::FileLineAt { sha, path, line } => format!("git:{sha}:{}:{line}", esc(path)),
            Src::ShippedFloor { version } => format!("spine:{version}:floor"),
        }
    }

    /// DM §5.4 and DM §17 item 12, applied to one citation.
    ///
    /// *"`<sha>` is a full oid at `object_format`'s length. `<line>` and `<n>`
    /// are decimal integers ≥ 1 with no leading zero."*
    ///
    /// The two never-emitted productions refuse here. DM §17 item 12 makes
    /// their appearance a conformance failure but DM §4.4 fixes no token of
    /// their own; `provenance-invalid` is this implementation's choice, on the
    /// grounds that a citation the artifact may not carry is a citation outside
    /// the dump's grammar. **DERIVED.**
    pub fn check(&self, format: ObjectFormat) -> Result<()> {
        let bad = || Err(Refusal::new(Status::ProvenanceInvalid, self.render()));
        let sha_ok = |sha: &String| is_oid(sha, format);
        // "decimal integers ≥ 1": zero cites no line, and a `u64` cannot spell
        // a leading zero, so the rule reduces to this one comparison.
        let line_ok = |line: &u64| *line >= 1;
        let ok = match self {
            Src::FileLine { .. } | Src::ShippedFloor { .. } => false,
            Src::Commit { sha } | Src::PatchId { sha } => sha_ok(sha),
            Src::MessageLine { sha, line } => sha_ok(sha) && line_ok(line),
            Src::FileLineAt { sha, line, .. } => sha_ok(sha) && line_ok(line),
            // A trailer name is a header name in the commit message; the
            // envelope's grammar owns its shape (DM §16), so only emptiness is
            // refused here.
            Src::Trailer { sha, name } => sha_ok(sha) && !name.is_empty(),
        };
        if ok { Ok(()) } else { bad() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nine_node_kinds_are_dm_5_1s_closed_set() {
        let tokens: Vec<&str> = NodeKind::ALL.iter().map(|k| k.token()).collect();
        assert_eq!(
            tokens,
            [
                "ac",
                "adr",
                "approval",
                "changeset",
                "code_unit",
                "constitution",
                "intent",
                "signer",
                "test"
            ]
        );
    }

    #[test]
    fn the_fifteen_edge_kinds_are_dm_5_3s_closed_set() {
        let tokens: Vec<&str> = EdgeKind::ALL.iter().map(|k| k.token()).collect();
        assert_eq!(
            tokens,
            [
                "approves",
                "attested_by",
                "built_under",
                "declares",
                "exercises",
                "freezes",
                "has_ac",
                "implements",
                "modifies",
                "protects",
                "reverts",
                "signed_by",
                "supersedes",
                "superseded_by",
                "verified_by"
            ]
        );
    }

    #[test]
    fn four_node_kinds_and_test_have_no_attrs_at_all() {
        // DM §7.2/§13.9: silence in PB §6.2 means none. `test` joins them once
        // §8.4 excludes `result_at`, which is the kind's only attr.
        for kind in [
            NodeKind::Ac,
            NodeKind::Adr,
            NodeKind::CodeUnit,
            NodeKind::Constitution,
            NodeKind::Test,
        ] {
            assert!(kind.attr_type("result_at").is_none());
            assert!(kind.attr_type("title").is_none());
            assert!(kind.attr_type("text").is_none());
        }
    }

    #[test]
    fn verified_by_has_attributed_and_cannot_carry_introduced_by() {
        // DM §8.5 clause 1: `git blame` "has no specified output contract", so
        // a git upgrade on the runner would turn the next landing into
        // `reconstruction-failed` with a report naming the graph.
        assert_eq!(
            EdgeKind::VerifiedBy.attr_type("attributed"),
            Some(AttrType::Bool)
        );
        assert!(EdgeKind::VerifiedBy.attr_type("introduced_by").is_none());
    }

    #[test]
    fn verified_by_runs_test_to_ac_because_g5_fails_on_the_ac_side() {
        assert_eq!(
            EdgeKind::VerifiedBy.endpoints(),
            Some((NodeKind::Test, NodeKind::Ac))
        );
    }

    #[test]
    fn an_approval_id_is_the_sha256_of_the_signed_line_not_its_freeze_digest() {
        // DM §12.1's three published lines, hashed by §5.2.1's rule. The
        // approve line's own `freeze=` is `sha256:5c9e2a71…`, and the id it
        // produces is `b6352921…` — the two readings are visibly different,
        // which is DM §13.6's whole argument.
        let signoff = b"Spine-Signoff: INT-042 blob=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 template=intent@2 constitution=v3 reopens=0 signer=alice@example.com";
        assert_eq!(
            id::approval("myrepo", signoff),
            "myrepo/approval:2f5e600237ec3d9a1f35fbc7ee6cf0dfd4335433def9937b5eeb8202bd3d66f6"
        );

        let approve = b"Spine-Approve: INT-042 intent=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 base=0a1b2c3d4e5f60718293a4b5c6d7e8f901234567 rounds=1 total_rounds=1 reopens=0 red=5/5 freeze=sha256:5c9e2a71b0463df8951ce2a4708b3d61f0492c8ad735be106f4a2c9d80e37b45 signer=alice@example.com";
        assert_eq!(
            id::approval("myrepo", approve),
            "myrepo/approval:b6352921ea42d618930f32f04ad773c20157810d418d20d06758149f366b85a8"
        );

        let review = b"Spine-Review: INT-042 class=tripwire head=60718293a4b5c6d7e8f90123456789012ab3c4d5 tree=7b0dc1f4a2e58d3906bb4c7e21f5a8d90c3e64b7 base=0a1b2c3d4e5f60718293a4b5c6d7e8f901234567 intent=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 report=sha256:3c6f1a09b8d24e57af0132c9de6b48570e29a1cf83b6d045e71a29c4b0d83e16 wires=G11 reason=\"auto-merge unavailable: C-A3 hostile\" reviewer=bob@example.com";
        assert_eq!(
            id::approval("myrepo", review),
            "myrepo/approval:ae8a406391f7130ce62d0e86fa4dca92195812aae2743e15e60434af56244021"
        );
    }

    #[test]
    fn a_code_unit_id_carries_the_trees_bytes_through_esc() {
        // DM §12.1's non-UTF-8 path: `src/billing/caf` + 0xE9 + `.py`, a
        // Latin-1 `é` that git stores as one byte. `esc` makes it ASCII; the
        // JSON layer then doubles the backslash, which the dump vector shows.
        let mut path = b"src/billing/caf".to_vec();
        path.push(0xE9);
        path.extend_from_slice(b".py");
        assert_eq!(
            id::code_unit("myrepo", &path),
            "myrepo/code:src/billing/caf\\xe9.py"
        );
    }

    #[test]
    fn a_node_id_that_does_not_begin_with_the_repo_is_out_of_grammar() {
        // PB §6.2's own examples are unprefixed; DM §13.3 reads them as
        // abbreviations, so an unprefixed id is a defect, not a variant.
        let err =
            check_node_id("myrepo", NodeKind::Intent, "INT-042", ObjectFormat::Sha1).unwrap_err();
        assert_eq!(err.status.token(), "id-out-of-grammar");
    }

    #[test]
    fn an_abbreviated_changeset_oid_is_out_of_grammar() {
        // "PB's `cs:abc123f` is display" (DM §5.2). An abbreviation's length is
        // a function of the repository's object count.
        assert!(
            check_node_id(
                "myrepo",
                NodeKind::Changeset,
                "myrepo/cs:abc123f",
                ObjectFormat::Sha1
            )
            .is_err()
        );
        assert!(
            check_node_id(
                "myrepo",
                NodeKind::Changeset,
                "myrepo/cs:1b2c3d4e5f60718293a4b5c6d7e8f90123456789",
                ObjectFormat::Sha1
            )
            .is_ok()
        );
    }

    #[test]
    fn an_uppercase_oid_is_out_of_grammar_at_either_object_format() {
        assert!(!is_oid(
            "1B2C3D4E5F60718293A4B5C6D7E8F90123456789",
            ObjectFormat::Sha1
        ));
        assert!(!is_oid(
            "1b2c3d4e5f60718293a4b5c6d7e8f90123456789",
            ObjectFormat::Sha256
        ));
    }

    #[test]
    fn a_worktree_citation_is_refused_because_a_dump_is_a_function_of_trees() {
        // DM §8.7: "Running `--dump` in a bare repository, with a dirty working
        // tree, with a stale index, or with untracked files present produces
        // identical bytes."
        let src = Src::FileLine {
            path: b"src/a.py".to_vec(),
            line: 3,
        };
        assert_eq!(src.render(), "src/a.py:3");
        assert_eq!(
            src.check(ObjectFormat::Sha1).unwrap_err().status.token(),
            "provenance-invalid"
        );
    }

    #[test]
    fn a_shipped_floor_citation_is_refused_because_it_names_the_release() {
        // DM §8.5 clause 2 and §3.4: two releases at one `dump_version` must
        // agree byte for byte.
        let src = Src::ShippedFloor {
            version: "1.4.0".to_string(),
        };
        assert_eq!(src.render(), "spine:1.4.0:floor");
        assert!(src.check(ObjectFormat::Sha1).is_err());
    }

    #[test]
    fn a_message_line_citation_is_one_indexed() {
        let sha = "1b2c3d4e5f60718293a4b5c6d7e8f90123456789".to_string();
        let good = Src::MessageLine {
            sha: sha.clone(),
            line: 24,
        };
        assert_eq!(
            good.render(),
            "git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L24"
        );
        assert!(good.check(ObjectFormat::Sha1).is_ok());
        assert!(
            Src::MessageLine { sha, line: 0 }
                .check(ObjectFormat::Sha1)
                .is_err()
        );
    }

    #[test]
    fn the_tool_version_splits_at_the_last_plus_sha256_not_the_first_plus() {
        // CI §5.5's spelling of a seal's `tool=`.
        assert_eq!(
            tool_version_from_seal(
                "1.4.0+sha256:9f2e0000000000000000000000000000000000000000000000000000000000"
            ),
            "1.4.0"
        );
        // MF §3.2 admits `+` inside `cli.version`, so a first-`+` split loses
        // the build metadata and records a release that does not exist.
        assert_eq!(
            tool_version_from_seal(
                "1.4.0+build.7+sha256:9f2e0000000000000000000000000000000000000000000000000000000000"
            ),
            "1.4.0+build.7"
        );
        // A rollback or uninstall seal may name a release the base does not
        // pin (GR §5); the dump records what the seal says.
        assert_eq!(tool_version_from_seal("1.4.0"), "1.4.0");
    }

    #[test]
    fn an_unknown_attr_name_refuses_with_attrs_out_of_profile() {
        let attrs = Attrs::new().str("colour", "blue");
        let err = attrs
            .check_profile(|n| NodeKind::Intent.attr_type(n), "myrepo/INT-1")
            .unwrap_err();
        assert_eq!(err.status.token(), "attrs-out-of-profile");
    }

    #[test]
    fn an_integer_above_two_to_the_fifty_three_is_out_of_profile() {
        let attrs = Attrs::new().int("reopen_count", MAX_INT + 1);
        assert!(
            attrs
                .check_profile(|n| NodeKind::Intent.attr_type(n), "myrepo/INT-1")
                .is_err()
        );
        let attrs = Attrs::new().int("reopen_count", MAX_INT);
        assert!(
            attrs
                .check_profile(|n| NodeKind::Intent.attr_type(n), "myrepo/INT-1")
                .is_ok()
        );
    }

    #[test]
    fn an_attr_of_the_wrong_type_is_out_of_profile() {
        // `landing` is a boolean on a changeset and an oid on an intent; the
        // two are different facts, and a swapped one is a silent whole-dump
        // diff.
        let attrs = Attrs::new().str("landing", "true");
        assert!(
            attrs
                .check_profile(|n| NodeKind::Changeset.attr_type(n), "myrepo/cs:x")
                .is_err()
        );
    }
}
