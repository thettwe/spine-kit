//! The gate report: GR §5's schema as a typed value, and GR §2's canonical
//! bytes as its only serialization.
//!
//! **What is stored and what is derived.** GR §5's table lists eighteen
//! top-level members; this type has fewer fields, because six members — the
//! two top-level ones below and four nested — are stated as functions of others,
//! and a field that can disagree with its own definition is a way to write a
//! wrong value into a signed artifact:
//!
//! | Member | Definition | Where it is computed |
//! |---|---|---|
//! | `threat` | "Equals `policy.rules.c_a3`" (§5) | [`Report::threat`] |
//! | `self_approved` | "`true` iff some entry of `authority.reviews` has `self_approved: true`" (§5) | [`Report::self_approved`] |
//! | `authority.reviews[].self_approved` | "`true` iff this review's `fingerprint` equals **the landing's signer key**" (§5.5) | [`Authority::review_is_self_approved`] |
//! | `automerge.requested` | "`policy.rules.c_m4 == \"on\"`" (§5.8) | [`Automerge::requested`] |
//! | `automerge.effective` | "`requested` **and** every precondition's `status` is `met` or `exempt`" (§5.8) | [`Automerge::effective`] |
//! | `policy.floor_source` | "`spine:<tool.version>:floor`" (§5.4) | [`Report::floor_source`] |
//!
//! GR §9.10 calls three of these "deliberate redundancies in the schema, and
//! each is checkable". They are checkable here because only one of the two
//! spellings exists in memory.
//!
//! **Byte-valued members hold bytes.** Every member that carries a repository
//! path, a pattern or a trailer line is a `Vec<u8>` and is `esc`-encoded on the
//! way out (GR §2.3, §7 rule 8). "Nothing is ever normalized. No NFC, no NFD,
//! no case folding, no separator rewriting."

use spine_canon::{ObjectFormat, Value, canonicalize, esc};

use crate::gate::{Gate, GateResult};
use crate::ids::{Fingerprint, IntentId, Oid, Sha256Digest};
use crate::vocab::{
    AutoMerge, Event, GateStatus, LandingShape, Lane, Mode, Namespace, PreconditionStatus,
    Reverify, RuleMode, SealProfile, Strategy, Threat, WireClass, WireKind,
};
use crate::wire::WireSet;

/// GR §3.1: "Every report carries `report_version`, an integer. This document
/// defines version `1`."
pub const REPORT_VERSION: u64 = 1;

/// GR §5.1 — which landing this is.
///
/// GR §5.1 is emphatic that this is **not** the landing commit's subject line:
/// that line is derived, sits outside `envelope=`, is covered by no digest, and
/// "is **not** a member of this report". A reader wanting the commit's first
/// line reads the commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject {
    pub lane: Lane,
    pub event: Event,
    /// "Present iff the landing has an intent id."
    pub intent: Option<IntentId>,
    pub strategy: Strategy,
}

impl Subject {
    /// GR §5.1: "The seal's first field is derived, not stored: `subject.intent`
    /// when present; else `"reseal"` when `subject.event == "reseal"`; else
    /// `"quick"`. Storing it would be a second spelling of the same fact."
    pub fn seal_first_field(&self) -> String {
        match (&self.intent, self.event) {
            (Some(id), _) => id.to_string(),
            (None, Event::Reseal) => "reseal".to_owned(),
            (None, _) => "quick".to_owned(),
        }
    }

    /// The GR §5.6.2 row this landing is judged by.
    pub fn shape(&self) -> LandingShape {
        LandingShape::of(self.lane, self.event)
    }

    fn to_value(&self) -> Value {
        let mut m: Vec<(String, Value)> = vec![
            ("lane".into(), Value::str(self.lane.token())),
            ("event".into(), Value::str(self.event.token())),
        ];
        if let Some(id) = &self.intent {
            m.push(("intent".into(), Value::str(id.as_str())));
        }
        m.push(("strategy".into(), Value::str(self.strategy.token())));
        Value::Obj(m)
    }
}

/// GR §5.2 — "A gate record is bound to `(head, base, tree)` and is void the
/// instant either ref moves (PB §5.4)."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Objects {
    /// `B` — the trunk tip the run fixed. For a reseal, the last valid landing
    /// below the range (PB §5.5).
    pub base: Oid,
    /// **The content head `Hc`**, never the literal ref tip (GR §9.11).
    /// "`H`'s nearest ancestor that is not an empty `Spine-Event: review`
    /// commit."
    pub head: Oid,
    /// The ref the run names, as bytes. `esc`-encoded on the way out.
    pub ref_name: Vec<u8>,
    pub merge_base: Oid,
    /// `T := git merge-tree --write-tree B Hc` — **the tree the gates
    /// evaluated**, with the intent file still in it. Not `L`'s tree, which the
    /// seal carries independently (GR §9.2). On a tombstone, `B`'s tree.
    pub tree: Oid,
    /// "Present iff `subject.intent` is present." On a tombstone this is the
    /// `Spine-Withdraw` line's `blob=`, not the sign-off's — "recording the
    /// sign-off's blob unconditionally would leave the member undefined for
    /// exactly the landing that has no sign-off" (GR §5.2).
    pub intent_blob: Option<Oid>,
}

impl Objects {
    fn to_value(&self) -> Value {
        let mut m: Vec<(String, Value)> = vec![
            ("base".into(), Value::str(self.base.as_str())),
            ("head".into(), Value::str(self.head.as_str())),
            ("ref".into(), Value::str(esc(&self.ref_name))),
            ("merge_base".into(), Value::str(self.merge_base.as_str())),
            ("tree".into(), Value::str(self.tree.as_str())),
        ];
        if let Some(blob) = &self.intent_blob {
            m.push(("intent_blob".into(), Value::str(blob.as_str())));
        }
        Value::Obj(m)
    }
}

/// GR §5.3 — `tool`. "`tool.version + "+" + tool.dist_hash` is exactly the
/// seal's `tool=` field (PB §11)."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// The release version, e.g. `"1.4.0"`. `esc`-encoded; in practice ASCII.
    pub version: String,
    /// The release's artifact-list digest (PB §6.7).
    pub dist_hash: Sha256Digest,
}

impl Tool {
    /// The seal's `tool=` field.
    pub fn seal_field(&self) -> String {
        format!("{}+{}", self.version, self.dist_hash)
    }

    fn to_value(&self) -> Value {
        Value::obj([
            ("version", Value::str(esc(self.version.as_bytes()))),
            ("dist_hash", Value::str(self.dist_hash.as_str())),
        ])
    }
}

/// GR §5.4.1 — the twelve scaffolded constitution rules.
///
/// "Values are what the constitution parser (`docs/spec/constitution.md`)
/// yields, **in the order it yields it** — this spec does not re-parse the
/// constitution and does not reorder its lists." So the four list-valued rules
/// are **file order**, not sorted: GR §8.2's published `c_t2` is fifteen
/// patterns in the constitution's own order and sorting them changes the
/// digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    pub c_a1: RuleMode,
    /// `protected` — file order, never sorted.
    pub c_a2: Vec<Vec<u8>>,
    pub c_a3: Threat,
    pub c_m1: Strategy,
    pub c_m2: Reverify,
    pub c_m3: u64,
    pub c_m4: AutoMerge,
    /// `quick.paths` — file order.
    pub c_q1: Vec<Vec<u8>>,
    pub c_q2: u64,
    /// test roots — file order.
    pub c_t1: Vec<Vec<u8>>,
    /// test support — file order.
    pub c_t2: Vec<Vec<u8>>,
    /// GR §5.4.1: "`c_t3` is `true` in every version-1 report, and that is the
    /// answer, not an oversight." `C-T3` carries no value to parse and G16
    /// requires all twelve rules at `base`, so "present and in force" cannot be
    /// false while a report exists. The field is `bool` and not `()` because a
    /// constitution grammar that later admits an aspirational or negated `C-T3`
    /// "changes what this boolean can say, and that is a `report_version` bump".
    pub c_t3: bool,
}

impl Rules {
    fn to_value(&self) -> Value {
        Value::obj([
            ("c_a1", Value::str(self.c_a1.token())),
            ("c_a2", esc_list_in_file_order(&self.c_a2)),
            ("c_a3", Value::str(self.c_a3.token())),
            ("c_m1", Value::str(self.c_m1.token())),
            ("c_m2", Value::str(self.c_m2.token())),
            ("c_m3", Value::Int(self.c_m3)),
            ("c_m4", Value::str(self.c_m4.token())),
            ("c_q1", esc_list_in_file_order(&self.c_q1)),
            ("c_q2", Value::Int(self.c_q2)),
            ("c_t1", esc_list_in_file_order(&self.c_t1)),
            ("c_t2", esc_list_in_file_order(&self.c_t2)),
            ("c_t3", Value::Bool(self.c_t3)),
        ])
    }
}

/// GR §5.4 — "what governed this run". "Policy is read from trunk, never from
/// the candidate (PB §7.4 rule 1). Every blob id below is the blob **at
/// `objects.base`**."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    pub manifest: Oid,
    pub keyring: Oid,
    pub constitution: Oid,
    pub ci_sh: Oid,
    /// "Every entry of `C-A2` plus every value of every `paths.*` key in the
    /// manifest at `base`." Held as raw bytes; deduplicated and sorted
    /// ascending by *encoded* bytes on the way out.
    ///
    /// "A list-valued `paths.*` key contributes one entry per element … never
    /// one stringified list."
    pub floor_extensions: Vec<Vec<u8>>,
    pub rules: Rules,
}

impl Policy {
    /// GR §5.4's `floor_source` is a function of `tool.version`, so it is not a
    /// field — see [`Report::floor_source`]. This takes the value the report
    /// derives.
    fn to_value(&self, floor_source: &str) -> Value {
        Value::obj([
            ("manifest", Value::str(self.manifest.as_str())),
            ("keyring", Value::str(self.keyring.as_str())),
            ("constitution", Value::str(self.constitution.as_str())),
            ("ci_sh", Value::str(self.ci_sh.as_str())),
            ("floor_source", Value::str(floor_source)),
            (
                "floor_extensions",
                esc_list_sorted_unique(&self.floor_extensions),
            ),
            ("rules", self.rules.to_value()),
        ])
    }
}

/// GR §5.5 — one verified signed statement.
///
/// "A report exists only for a run whose PB §5.4 step-2 bindings verified … So
/// the report never records an unverified statement and carries no `verified`
/// flag."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// "The trailer line exactly as it appears in the commit message, excluding
    /// the line terminator, `esc`-encoded. **A reader records the bytes it
    /// finds and normalizes nothing.**"
    ///
    /// GR §5.5: `<name> : ` one space `<payload>` "is a writer's constraint, not
    /// a reader's rewrite … A report that silently reshaped the line would hash
    /// bytes nobody signed."
    pub line: Vec<u8>,
    pub fingerprint: Fingerprint,
    pub namespace: Namespace,
}

impl Statement {
    fn to_value(&self, self_approved: Option<bool>) -> Value {
        let mut m: Vec<(String, Value)> = vec![
            ("line".into(), Value::str(esc(&self.line))),
            ("fingerprint".into(), Value::str(self.fingerprint.as_str())),
            ("namespace".into(), Value::str(self.namespace.token())),
        ];
        if let Some(sa) = self_approved {
            m.push(("self_approved".into(), Value::Bool(sa)));
        }
        Value::Obj(m)
    }
}

/// GR §5.5 — the verified signed statements.
///
/// "A `Spine-Review` is a member of `reviews` iff its `head=` equals
/// `objects.head` and its signature verifies … A review a content push voided
/// is **absent**, not recorded with a flag and not recorded at all."
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Authority {
    pub signoff: Option<Statement>,
    /// "Gated landing only; never on a tombstone, quick, lifecycle or reseal
    /// landing."
    pub approve: Option<Statement>,
    /// "Every `Spine-Reopen` on the branch, ancestor-first (§5.5.1)."
    pub reopens: Vec<Statement>,
    /// "Exactly the `Spine-Review` lines that bind **this** evaluation,
    /// ancestor-first." The per-review `self_approved` is derived, not stored.
    pub reviews: Vec<Statement>,
    pub upgrade: Option<Statement>,
    /// "Present iff `subject.event == "withdraw"`."
    pub withdraw: Option<Statement>,
}

impl Authority {
    /// GR §5.5: "**The landing's signer key** is `authority.signoff.fingerprint`
    /// when present, else `authority.upgrade.fingerprint` when present, else
    /// none."
    ///
    /// A landing with none is **signerless** — "every quick-lane landing that
    /// copies no `Spine-Upgrade`, every reseal, and an **orphaned tombstone**
    /// … and every review on it has `self_approved: false`, because there is
    /// nobody to be self."
    pub fn signer_key(&self) -> Option<&Fingerprint> {
        self.signoff
            .as_ref()
            .or(self.upgrade.as_ref())
            .map(|s| &s.fingerprint)
    }

    /// GR §5.5: "`true` iff this review's `fingerprint` equals the landing's
    /// signer key (PB §7.2)."
    pub fn review_is_self_approved(&self, review: &Statement) -> bool {
        self.signer_key() == Some(&review.fingerprint)
    }

    /// GR §5's top-level `self_approved`: the disjunction over `reviews`.
    ///
    /// GR §9.10 keeps both spellings deliberately — "the two consumers ask
    /// different questions" — while noting that the top-level boolean "cannot
    /// express *self-approved protected review*, so it is not what PB §6.5
    /// counts."
    pub fn any_review_self_approved(&self) -> bool {
        self.reviews.iter().any(|r| self.review_is_self_approved(r))
    }

    fn to_value(&self) -> Value {
        let mut m: Vec<(String, Value)> = Vec::new();
        if let Some(s) = &self.approve {
            m.push(("approve".into(), s.to_value(None)));
        }
        m.push((
            "reopens".into(),
            Value::Arr(self.reopens.iter().map(|s| s.to_value(None)).collect()),
        ));
        m.push((
            "reviews".into(),
            Value::Arr(
                self.reviews
                    .iter()
                    .map(|r| r.to_value(Some(self.review_is_self_approved(r))))
                    .collect(),
            ),
        ));
        if let Some(s) = &self.signoff {
            m.push(("signoff".into(), s.to_value(None)));
        }
        if let Some(s) = &self.upgrade {
            m.push(("upgrade".into(), s.to_value(None)));
        }
        if let Some(s) = &self.withdraw {
            m.push(("withdraw".into(), s.to_value(None)));
        }
        Value::Obj(m)
    }
}

/// GR §5.8 — "PB §7.4 rule 5, made a record".
///
/// Only the five statuses are stored: `requested` is `c_m4 == on` and
/// `effective` is their conjunction with it, both derived below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Automerge {
    /// Five entries, `id` ascending — the array index **is** the `id`.
    ///
    /// | id | Precondition (PB §7.4 rule 5) |
    /// |---|---|
    /// | 0 | `C-A3: threat.candidate == "trusted"` |
    /// | 1 | manifest `params.isolation` = `container` **and** the ingested header's `profile=` equals it |
    /// | 2 | `keys_visible=false` **and** the collector's `tool=` is the base's pin **and** this run established a trunk-defined origin — **three conjuncts, not two** |
    /// | 3 | reconstruction proved before this push — structurally `"met"` in v1 |
    /// | 4 | this run performs the CAS itself |
    pub preconditions: [PreconditionStatus; 5],
}

impl Automerge {
    /// GR §5.8: `requested` is `policy.rules.c_m4 == "on"`.
    pub const fn requested(rules_c_m4: AutoMerge) -> bool {
        matches!(rules_c_m4, AutoMerge::On)
    }

    /// GR §5.8: "`requested` **and** every precondition's `status` is `met` or
    /// `exempt`."
    ///
    /// GR §5.8 notes the one shape that reads oddly and is not a bug: "A
    /// tombstone under `C-M4: on` therefore records `effective: true`. All five
    /// are exempt, so the conjunction reduces to `requested` … a tombstone
    /// changes no tree, runs no suite and produces no wire of its own."
    pub fn effective(&self, rules_c_m4: AutoMerge) -> bool {
        Automerge::requested(rules_c_m4) && self.preconditions.iter().all(|p| p.satisfied())
    }

    /// PB §7.4 rule 5's singular exemption: "a **tombstone** is exempt from the
    /// rule entirely — all five `exempt`, `profile: n/a`."
    pub const EXEMPT: Automerge = Automerge {
        preconditions: [PreconditionStatus::Exempt; 5],
    };

    fn to_value(self, rules_c_m4: AutoMerge) -> Value {
        Value::obj([
            ("requested", Value::Bool(Automerge::requested(rules_c_m4))),
            (
                "preconditions",
                Value::Arr(
                    self.preconditions
                        .iter()
                        .enumerate()
                        .map(|(id, status)| {
                            Value::obj([
                                ("id", Value::Int(id as u64)),
                                ("status", Value::str(status.token())),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("effective", Value::Bool(self.effective(rules_c_m4))),
        ])
    }
}

/// GR §5.9's `evidence.collector`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collector {
    pub version: String,
    pub dist_hash: Sha256Digest,
}

/// GR §5.9 — "the collector's attested facts". Present iff a result file was
/// ingested.
///
/// GR §5.9's preamble is the rule that keeps this member honest when provenance
/// is in doubt: "A well-formed file whose trunk-defined origin the run could
/// not establish **is ingested**, so `evidence` is **present** and every member
/// below is read from its header exactly as from any other … no member of
/// `evidence` is suppressed, blanked, downgraded or annotated on account of it.
/// `result_sha256` in particular pins the exact bytes a forger would have had
/// to write."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    /// Over the result file's exact bytes as the collector wrote them.
    pub result_sha256: Sha256Digest,
    pub collector: Collector,
    /// "**`true` is representable**: it does not refuse ingestion and does not
    /// fail the run — it fails auto-merge precondition 2."
    pub keys_visible: bool,
    /// "The header's `ids=` — the size of the id set collected on `B`. **An id
    /// is a `(runner, id)` pair**" (GR §9.24), so one runner-native string
    /// collected by two runners counts twice.
    pub ids: u64,
}

impl Evidence {
    fn to_value(&self) -> Value {
        Value::obj([
            ("result_sha256", Value::str(self.result_sha256.as_str())),
            (
                "collector",
                Value::obj([
                    (
                        "version",
                        Value::str(esc(self.collector.version.as_bytes())),
                    ),
                    ("dist_hash", Value::str(self.collector.dist_hash.as_str())),
                ]),
            ),
            ("keys_visible", Value::Bool(self.keys_visible)),
            ("ids", Value::Int(self.ids)),
        ])
    }
}

/// GR §5.10 — `run`. One member, and it is attested.
///
/// "This counter lives in the run's memory and dies with it. Nothing in this
/// design may remember that a previous run happened (PB §5.4, PB §12) … A fresh
/// run after a lost CAS starts at 0."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    /// "How many re-verifications this run has performed, ≥ 0, ≤
    /// `policy.rules.c_m3`."
    pub reverifications: u64,
}

/// A gate report.
///
/// The only serialization is [`Report::canonical_bytes`]. There is no pretty
/// form, no second digest and no exported rendering: GR §11 puts "any second
/// digest, second format, or exported rendering" out of scope, and "a rendering
/// that ships is counted by PB §10's graph budget".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub subject: Subject,
    pub objects: Objects,
    pub tool: Tool,
    /// `"<major>.<minor>"` — see [`crate::git_version::parse`], whose rule is
    /// normative "because a mis-parse forks both the digest and §3.3's
    /// `wrong-git` check".
    pub git_version: String,
    pub object_format: ObjectFormat,
    /// "Equals `policy.rules.c_a1` except on a recovery-sealed landing (PB
    /// §7.5), where it is `recovery`" — which is why this is stored and
    /// [`Report::threat`] is not.
    pub mode: Mode,
    pub profile: SealProfile,
    pub policy: Policy,
    pub authority: Authority,
    /// Ascending by gate **number** (GR §5.6). Membership is
    /// [`Gate::running_on`] for this landing's shape.
    pub gates: Vec<GateResult>,
    pub wires: WireSet,
    /// GR §5.7. Raw path bytes; `esc`-encoded, deduplicated and sorted
    /// ascending by encoded bytes on the way out. "Paths are recorded as the
    /// diff produced them; G14's casefolding is a comparison, not a rewriting."
    pub floor_hits: Vec<Vec<u8>>,
    pub automerge: Automerge,
    /// "Present iff a result file was ingested."
    pub evidence: Option<Evidence>,
    pub run: Run,
}

impl Report {
    /// GR §5: "Equals `policy.rules.c_a3`", with no exception.
    pub fn threat(&self) -> Threat {
        self.policy.rules.c_a3
    }

    /// GR §5: "`true` iff some entry of `authority.reviews` has
    /// `self_approved: true`."
    pub fn self_approved(&self) -> bool {
        self.authority.any_review_self_approved()
    }

    /// GR §5.4: "`spine:<tool.version>:floor` — the provenance token of PB §6.1
    /// for the floor list shipped inside the pinned release."
    ///
    /// GR §9.10 names it one of the schema's three checkable redundancies:
    /// "`policy.floor_source` is a function of `tool.version`."
    ///
    /// DERIVED: the corpus does not say whether the interpolated version is the
    /// raw or the `esc`-encoded one. `esc` is taken, because `floor_source` is
    /// a report string and every report string is ASCII after `esc` (GR §2.2),
    /// and because `tool.version` is itself `esc`-encoded (GR §5.3). The two
    /// readings coincide for every ASCII version, so no published byte moves.
    pub fn floor_source(&self) -> String {
        format!("spine:{}:floor", esc(self.tool.version.as_bytes()))
    }

    /// The GR §5.6.2 row this landing is judged by.
    pub fn shape(&self) -> LandingShape {
        self.subject.shape()
    }

    /// GR §5.6.1: "A report containing any `fail` is a non-landing report … A
    /// run that would seal a report containing a `fail` refuses: status
    /// `report-not-landable`."
    pub fn is_landable(&self) -> bool {
        !self.gates.iter().any(|g| g.status == GateStatus::Fail)
    }

    /// GR §4.2: `--verify` refuses as `not-recomputable` when "the report's gate
    /// results were computed over a tree built from `objects.head` and
    /// `objects.head` is unreachable".
    ///
    /// **This is not read off `subject.strategy`.** GR §4.2: "A tombstone and a
    /// reseal are recomputable whatever `subject.strategy` records … Reading
    /// exit 4 off `subject.strategy` alone would refuse every tombstone and
    /// every reseal in a squash repository for a fact neither rests on."
    /// A tombstone's tree is `B`'s, built with no merge tree over `H`; a
    /// reseal's `Hc` is `O`, a first-parent commit of trunk, which no landing
    /// deletes.
    pub fn needs_head(&self) -> bool {
        matches!(self.subject.event, Event::Land)
    }

    /// GR §5.7's derivation: "for each entry `p`, `wires` contains exactly one
    /// `{gate: "G14", path: p, class: "protected", kind: "finding"}` …
    /// `floor_hits` is the authoritative list; the `G14` wires are derived from
    /// it."
    ///
    /// Building the wires *from* the hits is what makes R2 unreachable: the two
    /// members encode the same bytes under `esc` and `tok` respectively, and a
    /// caller that assembled them separately could spell one path two ways.
    pub fn floor_wires(floor_hits: &[Vec<u8>]) -> Vec<crate::Wire> {
        let mut seen: Vec<&[u8]> = Vec::new();
        let mut out = Vec::new();
        for p in floor_hits {
            if seen.contains(&p.as_slice()) {
                continue;
            }
            seen.push(p);
            out.push(crate::Wire::at(
                Gate::G14,
                p.clone(),
                WireClass::Protected,
                WireKind::Finding,
            ));
        }
        out
    }

    /// GR §2's canonical form as a [`Value`]. The bytes are
    /// [`Report::canonical_bytes`]; this is the value they serialize from.
    ///
    /// Member insertion order is GR §5's reading order and is *not* the output
    /// order: RFC 8785 §3.2.3 sorts by member name, which is what
    /// `spine_canon::canonicalize` does.
    pub fn to_value(&self) -> Value {
        let mut m: Vec<(String, Value)> = vec![
            ("report_version".into(), Value::Int(REPORT_VERSION)),
            ("subject".into(), self.subject.to_value()),
            ("objects".into(), self.objects.to_value()),
            ("tool".into(), self.tool.to_value()),
            ("git_version".into(), Value::str(self.git_version.clone())),
            (
                "object_format".into(),
                Value::str(self.object_format.as_str()),
            ),
            ("mode".into(), Value::str(self.mode.token())),
            ("threat".into(), Value::str(self.threat().token())),
            ("profile".into(), Value::str(self.profile.token())),
            ("policy".into(), self.policy.to_value(&self.floor_source())),
            ("authority".into(), self.authority.to_value()),
            ("self_approved".into(), Value::Bool(self.self_approved())),
            ("gates".into(), gates_to_value(&self.gates)),
            ("wires".into(), wires_to_value(&self.wires)),
            (
                "floor_hits".into(),
                esc_list_sorted_unique(&self.floor_hits),
            ),
            (
                "automerge".into(),
                self.automerge.to_value(self.policy.rules.c_m4),
            ),
        ];
        // GR §7 rule 6: "`null` never appears. An optional member is present or
        // absent … Absence always means *this concept does not apply to this
        // landing*."
        if let Some(e) = &self.evidence {
            m.push(("evidence".into(), e.to_value()));
        }
        m.push((
            "run".into(),
            Value::obj([("reverifications", Value::Int(self.run.reverifications))]),
        ));
        Value::Obj(m)
    }

    /// GR §2.1: the RFC 8785 serialization of [`Report::to_value`]. "No
    /// trailing newline, no BOM, no framing. A file holding a report contains
    /// exactly the canonical bytes and nothing else, so `sha256sum` over the
    /// file reproduces `report=`."
    ///
    /// These are also the bytes GR §4.4.1 publishes: "the note … holds them
    /// exactly — no newline, no framing, no pretty form."
    pub fn canonical_bytes(&self) -> Vec<u8> {
        canonicalize(&self.to_value())
    }

    /// `report=sha256:<hex>` over exactly [`Report::canonical_bytes`].
    pub fn digest(&self) -> Sha256Digest {
        Sha256Digest::of(&self.canonical_bytes())
    }
}

/// GR §5.6: `gates[]` "sorts by gate number ascending".
fn gates_to_value(gates: &[GateResult]) -> Value {
    let mut sorted: Vec<GateResult> = gates.to_vec();
    sorted.sort_by_key(|g| g.gate.number());
    Value::Arr(
        sorted
            .into_iter()
            .map(|g| {
                Value::obj([
                    ("gate", Value::str(g.gate.token())),
                    ("status", Value::str(g.status.token())),
                ])
            })
            .collect(),
    )
}

/// GR §6.1's entry shape. The array's order is [`WireSet`]'s own, established
/// at construction and never re-derived here.
fn wires_to_value(wires: &WireSet) -> Value {
    Value::Arr(
        wires
            .as_slice()
            .iter()
            .map(|w| {
                let mut m: Vec<(String, Value)> = vec![("gate".into(), Value::str(w.gate.token()))];
                // `esc`, not `tok`. The token is the sort key and the signed
                // spelling; this member is the path (GR §6.1).
                if let Some(p) = w.path_member() {
                    m.push(("path".into(), Value::str(p)));
                }
                m.push(("class".into(), Value::str(w.class.token())));
                m.push(("kind".into(), Value::str(w.kind.token())));
                Value::Obj(m)
            })
            .collect(),
    )
}

/// A list the constitution parser yielded, "in the order it yields it"
/// (GR §5.4.1). Encoded, never reordered.
fn esc_list_in_file_order(items: &[Vec<u8>]) -> Value {
    Value::Arr(items.iter().map(|p| Value::str(esc(p))).collect())
}

/// GR §5.4 and §5.7: "`esc`-encoded, deduplicated, sorted ascending by encoded
/// bytes."
///
/// Sorting on the *encoded* bytes and not the raw ones is the rule as written,
/// and the two orders differ: `esc` maps every byte above `0x7E` to `\xhh`,
/// whose first byte is `0x5C`, so a raw-byte sort places such a path after
/// `~` while an encoded sort places it before `]`.
fn esc_list_sorted_unique(items: &[Vec<u8>]) -> Value {
    let mut encoded: Vec<String> = items.iter().map(|p| esc(p)).collect();
    encoded.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    encoded.dedup();
    Value::Arr(encoded.into_iter().map(Value::str).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::IntentId;

    #[test]
    fn the_seal_first_field_is_derived_from_intent_then_event() {
        let base = Subject {
            lane: Lane::Gated,
            event: Event::Land,
            intent: Some(IntentId::parse("INT-042").unwrap()),
            strategy: Strategy::Merge,
        };
        assert_eq!(base.seal_first_field(), "INT-042");

        let reseal = Subject {
            lane: Lane::Quick,
            event: Event::Reseal,
            intent: None,
            strategy: Strategy::Merge,
        };
        assert_eq!(reseal.seal_first_field(), "reseal");

        let quick = Subject {
            lane: Lane::Quick,
            event: Event::Land,
            intent: None,
            strategy: Strategy::Squash,
        };
        assert_eq!(quick.seal_first_field(), "quick");
    }

    /// GR §5.4: "sorted ascending by **encoded** bytes". A path whose bytes are
    /// non-ASCII sorts by its `\xhh` spelling, which begins `0x5C`.
    #[test]
    fn floor_lists_sort_on_the_encoded_bytes_not_the_raw_ones() {
        // `caf` + 0xC3 0xA9 — GR §2.3's own worked case, `caf\xc3\xa9`.
        let cafe = b"caf\xc3\xa9".to_vec();
        let tilde = b"caf~".to_vec();
        assert!(cafe > tilde, "raw byte order puts 0xC3 above 0x7E");
        assert!(
            esc(&cafe).as_bytes() < esc(&tilde).as_bytes(),
            "encoded order puts 0x5C below 0x7E"
        );

        let v = esc_list_sorted_unique(&[tilde, cafe]);
        assert_eq!(
            spine_canon::canonicalize_to_string(&v),
            r#"["caf\\xc3\\xa9","caf~"]"#
        );
    }

    /// GR §5.4: "deduplicated" — `C-A2`'s entries and the manifest's `paths.*`
    /// values overlap, and GR §8.2's `floor_extensions` shows `adr/` and
    /// `db/migrations/` appearing once each despite being in both.
    #[test]
    fn floor_extensions_deduplicate() {
        let v = esc_list_sorted_unique(&[b"adr/".to_vec(), b"adr/".to_vec()]);
        assert_eq!(spine_canon::canonicalize_to_string(&v), r#"["adr/"]"#);
    }

    /// GR §5.4.1: the constitution's lists are recorded "in the order it yields
    /// it — this spec does not re-parse the constitution and does not reorder
    /// its lists". GR §8.2's published `c_t2` is not sorted.
    #[test]
    fn constitution_lists_keep_file_order() {
        let v = esc_list_in_file_order(&[b"zebra".to_vec(), b"alpha".to_vec()]);
        assert_eq!(
            spine_canon::canonicalize_to_string(&v),
            r#"["zebra","alpha"]"#
        );
    }

    /// GR §5.5: the signer key is the sign-off's, else the upgrade's, else
    /// none — and "a landing with none is a **signerless** landing … every
    /// review on it has `self_approved: false`, because there is nobody to be
    /// self."
    #[test]
    fn a_signerless_landing_has_no_self_approved_review() {
        let fp = Fingerprint::parse("SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM").unwrap();
        let review = Statement {
            line: b"Spine-Review: quick ...".to_vec(),
            fingerprint: fp.clone(),
            namespace: Namespace::Review,
        };
        let signerless = Authority {
            reviews: vec![review.clone()],
            ..Authority::default()
        };
        assert!(signerless.signer_key().is_none());
        assert!(!signerless.any_review_self_approved());

        // PB §6.7: "a toolkit lifecycle landing rides the quick lane but copies
        // a `Spine-Upgrade`, and this is what gives that landing a signer."
        let with_upgrade = Authority {
            upgrade: Some(Statement {
                line: b"Spine-Upgrade: to=1.4.0".to_vec(),
                fingerprint: fp,
                namespace: Namespace::Signoff,
            }),
            reviews: vec![review],
            ..Authority::default()
        };
        assert!(with_upgrade.signer_key().is_some());
        assert!(with_upgrade.any_review_self_approved());
    }

    /// GR §5.5: the sign-off wins over the upgrade when both are present.
    #[test]
    fn the_signoff_key_takes_precedence_over_the_upgrade_key() {
        let a = Fingerprint::parse("SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM").unwrap();
        let b = Fingerprint::parse("SHA256:V2dasTIGWUnhlaxa7vr3Qmqpe/qYBoxi+C7Cs6yxxEs").unwrap();
        let stmt = |fp: Fingerprint| Statement {
            line: b"x".to_vec(),
            fingerprint: fp,
            namespace: Namespace::Signoff,
        };
        let auth = Authority {
            signoff: Some(stmt(a.clone())),
            upgrade: Some(stmt(b)),
            ..Authority::default()
        };
        assert_eq!(auth.signer_key(), Some(&a));
    }

    /// GR §5.8: "A tombstone under `C-M4: on` therefore records
    /// `effective: true`. All five are exempt, so the conjunction reduces to
    /// `requested`. That reads like a bug and is not one."
    #[test]
    fn a_tombstone_under_c_m4_on_is_effective() {
        assert!(Automerge::EXEMPT.effective(AutoMerge::On));
        assert!(!Automerge::EXEMPT.effective(AutoMerge::Off));
    }

    /// GR §5.8: "under the shipped `C-A3: hostile`, precondition 0 is
    /// `unmet`" — so `effective` is false however the rest reads.
    #[test]
    fn one_unmet_precondition_defeats_effective() {
        let am = Automerge {
            preconditions: [
                PreconditionStatus::Unmet,
                PreconditionStatus::Met,
                PreconditionStatus::Met,
                PreconditionStatus::Met,
                PreconditionStatus::Met,
            ],
        };
        assert!(Automerge::requested(AutoMerge::On));
        assert!(!am.effective(AutoMerge::On));
    }

    /// GR §5.7: "`floor_hits` is the authoritative list; the `G14` wires are
    /// derived from it", one per entry and no other `G14` entry.
    #[test]
    fn floor_wires_are_one_protected_finding_per_hit() {
        let hits = vec![b"adr/0001.md".to_vec(), b"db/migrations/1.sql".to_vec()];
        let wires = Report::floor_wires(&hits);
        assert_eq!(wires.len(), 2);
        for w in &wires {
            assert_eq!(w.gate, Gate::G14);
            assert_eq!(w.class, WireClass::Protected);
            assert_eq!(w.kind, WireKind::Finding);
        }
        assert_eq!(wires[0].token(), "G14:adr/0001.md");
    }

    /// GR §4.2: "A tombstone and a reseal are recomputable whatever
    /// `subject.strategy` records."
    #[test]
    fn recomputability_is_read_off_the_event_never_off_the_strategy() {
        let mk = |event| Subject {
            lane: Lane::Quick,
            event,
            intent: None,
            strategy: Strategy::Squash,
        };
        assert_eq!(mk(Event::Withdraw).shape(), LandingShape::Tombstone);
        assert_eq!(mk(Event::Reseal).shape(), LandingShape::Reseal);
    }
}
