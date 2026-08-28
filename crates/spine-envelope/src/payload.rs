//! PB §11's *Trailers* table, one type per payload grammar.
//!
//! "PB §11's *Trailers* table is the grammar and it wins" (EV §2.5). Three
//! things it shows but does not say, and all three are enforced here: field
//! order is normative, exactly one `U+0020` between fields, and `reason=`
//! values are JSON string literals.
//!
//! **Emission is by position, parsing is by key.** Every type below therefore
//! carries a `render` that walks PB §11's printed order, and a `parse` that
//! refuses a line whose fields arrive in any other one — "without that, two
//! implementations produce different bytes over identical facts and every
//! digest and every signature diverges" (EV §2.5).

use crate::quote::{quote_path, unquote_path};
use crate::refusal::{EnvelopeError, Refusal};
use crate::trailer::{
    Key, as_str, counter, oid, render_fields, sha256_field, show, split_fields, take,
};
use core::fmt;

// ---------------------------------------------------------------------------
// Closed token sets
// ---------------------------------------------------------------------------

macro_rules! closed_set {
    ($(#[$m:meta])* $name:ident { $($variant:ident => $token:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub fn token(self) -> &'static str {
                match self { $($name::$variant => $token),+ }
            }
            pub fn parse(bytes: &[u8]) -> Option<Self> {
                $(if bytes == $token.as_bytes() { return Some($name::$variant); })+
                None
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.token())
            }
        }
    };
}

closed_set! {
    /// PB §11's `Spine-Event` row: "`signoff · approve · review · reopen ·
    /// withdraw · upgrade · land · reseal`".
    Event {
        Signoff => "signoff",
        Approve => "approve",
        Review => "review",
        Reopen => "reopen",
        Withdraw => "withdraw",
        Upgrade => "upgrade",
        Land => "land",
        Reseal => "reseal",
    }
}

closed_set! {
    /// PB §11: `gated | quick`.
    Lane { Gated => "gated", Quick => "quick" }
}

closed_set! {
    /// PB §11: `merge | squash`. "*Rebase* landings are refused for both lanes"
    /// (PB §5.5), so there is no third variant to refuse later.
    Strategy { Merge => "merge", Squash => "squash" }
}

closed_set! {
    /// PB §11's seal row: `mode=solo|team|recovery`.
    Mode { Solo => "solo", Team => "team", Recovery => "recovery" }
}

closed_set! {
    /// PB §11's seal row: `threat=hostile|trusted`.
    Threat { Hostile => "hostile", Trusted => "trusted" }
}

closed_set! {
    /// PB §11's seal row: `profile=container|uid|none|n/a`. `n/a` is the
    /// tombstone's — the one landing shape that runs no suite (PB §11).
    Profile { Container => "container", Uid => "uid", None => "none", NotApplicable => "n/a" }
}

closed_set! {
    /// PB §11's review row: `class=tripwire|protected|break-glass`.
    ReviewClass {
        Tripwire => "tripwire",
        Protected => "protected",
        BreakGlass => "break-glass",
    }
}

closed_set! {
    /// The **sealed** status vocabulary, and only it.
    ///
    /// GR §5.6.1: "PB §11 fixes the sealed vocabulary: a `Spine-Gates` entry is
    /// `pass` or `override`. This spec adds exactly one value, `fail`, for
    /// evaluations that do not seal." A `Spine-Gates` line exists only on a
    /// landing, and a landing sealed, so `fail` has no spelling here and a line
    /// carrying one is `envelope-malformed`.
    GateStatus { Pass => "pass", Override => "override" }
}

fn token<T>(value: &[u8], what: &str, parse: impl Fn(&[u8]) -> Option<T>) -> Result<T, EnvelopeError> {
    parse(value).ok_or_else(|| {
        EnvelopeError::malformed(format!("{what} is outside its closed set: {}", show(value)))
    })
}

// ---------------------------------------------------------------------------
// `reason=` and the other JSON string literals
// ---------------------------------------------------------------------------

/// EV §2.5 rule 3: "`reason=` values are JSON string literals (PB §7.2, PB §11):
/// a `"` delimited run with JSON's escaping, so a reason containing a quote, a
/// backslash, a newline or any non-ASCII character is representable and the
/// line stays one line."
fn parse_json_string(value: &[u8], what: &str) -> Result<String, EnvelopeError> {
    if value.first() != Some(&b'"') {
        return Err(EnvelopeError::malformed(format!(
            "{what} is not a JSON string literal: {}",
            show(value)
        )));
    }
    match spine_canon::parse(value) {
        Ok(spine_canon::Value::Str(s)) => Ok(s),
        Ok(_) => Err(EnvelopeError::malformed(format!("{what} is not a string"))),
        Err(e) => Err(EnvelopeError::malformed(format!("{what}: {e}"))),
    }
}

/// DERIVED: the corpus fixes the *shape* ("JSON string literals") but names no
/// escaping profile for emission. Rendering goes through RFC 8785's escape set
/// — `spine_canon`'s canonicalizer — because that is the only string escaping
/// the corpus fixes anywhere, and because it escapes `0x0A` and `0x0D`, which
/// is what keeps the line one line (EV §2.5).
fn render_json_string(s: &str) -> Vec<u8> {
    spine_canon::canonicalize(&spine_canon::Value::Str(s.to_owned()))
}

// ---------------------------------------------------------------------------
// Wire tokens
// ---------------------------------------------------------------------------

/// GR §6.2's wire token: "`G<n>` when `path` is absent; `G<n>` + `:` +
/// `tok(path)` otherwise."
///
/// `tok` is `spine_canon`'s and is never re-derived: it is `esc` with `,`, ` `
/// and `"` moved into the `\xHH` row, "one pass over the bytes of `s`, not
/// `esc` composed with a second escaping step" (GR §6.2).
pub fn wire_token(gate: u32, path: Option<&[u8]>) -> String {
    match path {
        Some(p) => format!("G{gate}:{}", spine_canon::tok(p)),
        None => format!("G{gate}"),
    }
}

/// The comparator PB §11 fixes in the `Spine-Review` row itself: "ascending by
/// unsigned byte value over the whole token, so `G11` precedes `G2`; a set with
/// no order is a signature two runs spell differently."
///
/// **A numeric sort is non-conforming.** GR §6.2: it "produces byte-different
/// `Spine-Review` lines and its containment check fails against a conforming
/// implementation's report over identical facts" — and because re-sorting is a
/// permutation, every published byte count survives it and only the digests
/// separate the two (EV §14 D3).
pub fn cmp_wires(a: &str, b: &str) -> core::cmp::Ordering {
    a.as_bytes().cmp(b.as_bytes())
}

/// Parse a `wires=` value, checking the order PB §11 signs.
pub fn parse_wires(value: &[u8]) -> Result<Vec<String>, EnvelopeError> {
    if value.is_empty() {
        return Err(EnvelopeError::malformed("wires= is empty"));
    }
    let mut out = Vec::new();
    for part in value.split(|&b| b == b',') {
        let s = as_str(part, "a wire token")?;
        if s.is_empty() {
            return Err(EnvelopeError::malformed(
                "wires= has a leading, trailing or doubled comma",
            ));
        }
        out.push(s.to_owned());
    }
    for pair in out.windows(2) {
        if cmp_wires(&pair[0], &pair[1]) != core::cmp::Ordering::Less {
            return Err(EnvelopeError::malformed(format!(
                "wires= is not ascending by unsigned byte value: {} then {}",
                pair[0], pair[1]
            )));
        }
    }
    Ok(out)
}

/// Sort into PB §11's order and render. The line is the array's tokens joined
/// by `,` — "nothing has to be re-sorted to write it" (GR §6.2), which holds
/// only because the array was built under the same key.
pub fn render_wires(tokens: &[String]) -> String {
    let mut sorted = tokens.to_vec();
    sorted.sort_by(|a, b| cmp_wires(a, b));
    sorted.join(",")
}

// ---------------------------------------------------------------------------
// `forced=`
// ---------------------------------------------------------------------------

/// MF §6.4: "`tok(path)` [ `,` `tok(path)` ]\* — **the empty list is the empty
/// value**". Not `none`: "`none` would be indistinguishable from `tok("none")`,
/// which is a legal path. A leading, trailing or doubled comma is malformed."
pub fn parse_forced(value: &[u8]) -> Result<Vec<Vec<u8>>, EnvelopeError> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for part in value.split(|&b| b == b',') {
        let s = as_str(part, "a forced= entry")?;
        if s.is_empty() {
            return Err(EnvelopeError::malformed(
                "forced= has a leading, trailing or doubled comma",
            ));
        }
        out.push(
            spine_canon::unesc(s)
                .map_err(|e| EnvelopeError::malformed(format!("forced= entry: {e}")))?,
        );
    }
    Ok(out)
}

pub fn render_forced(paths: &[Vec<u8>]) -> String {
    paths
        .iter()
        .map(|p| spine_canon::tok(p))
        .collect::<Vec<_>>()
        .join(",")
}

// ---------------------------------------------------------------------------
// The signed statements
// ---------------------------------------------------------------------------

/// PB §11: `INT-042 blob=<oid> template=<variant>@<n> constitution=v3
/// reopens=n [lease_override="…"] signer=<p>`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signoff {
    pub id: String,
    pub blob: String,
    pub template: String,
    pub constitution: String,
    pub reopens: u64,
    pub lease_override: Option<String>,
    pub signer: String,
}

const SIGNOFF_KEYS: &[Key] = &[
    Key::req("blob"),
    Key::req("template"),
    Key::req("constitution"),
    Key::req("reopens"),
    Key::opt("lease_override"),
    Key::req("signer"),
];

impl Signoff {
    pub fn parse(payload: &[u8]) -> Result<Self, EnvelopeError> {
        let (pos, v) = take(payload, 1, SIGNOFF_KEYS)?;
        Ok(Signoff {
            id: as_str(pos[0], "the intent id")?.to_owned(),
            blob: oid(v[0].unwrap(), "blob")?,
            // The owner's decision of 2026-08-26: "`Template:` names the
            // variant and the version", so a payload reads `template=intent@2`
            // and never the bare `v2` of envelope-vectors version 1 (EV, amended).
            template: as_str(v[1].unwrap(), "template")?.to_owned(),
            constitution: as_str(v[2].unwrap(), "constitution")?.to_owned(),
            reopens: counter(v[3].unwrap(), "reopens")?,
            lease_override: v[4]
                .map(|x| parse_json_string(x, "lease_override"))
                .transpose()?,
            signer: as_str(v[5].unwrap(), "signer")?.to_owned(),
        })
    }

    pub fn render(&self) -> Vec<u8> {
        let mut keyed: Vec<(&str, Vec<u8>)> = vec![
            ("blob", self.blob.clone().into_bytes()),
            ("template", self.template.clone().into_bytes()),
            ("constitution", self.constitution.clone().into_bytes()),
            ("reopens", self.reopens.to_string().into_bytes()),
        ];
        if let Some(r) = &self.lease_override {
            keyed.push(("lease_override", render_json_string(r)));
        }
        keyed.push(("signer", self.signer.clone().into_bytes()));
        render_fields(&[self.id.as_bytes()], &keyed)
    }
}

/// PB §11: `INT-042 intent=<oid> base=<sha> rounds=0..2 total_rounds=n
/// reopens=n red=k/n freeze=sha256:<hex> [run=sha256:<hex>] [held=false]
/// [reason="…"] signer=<p>`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approve {
    pub id: String,
    pub intent: String,
    pub base: String,
    pub rounds: u64,
    pub total_rounds: u64,
    pub reopens: u64,
    /// PB §11's `red=k/n`; EV §7 rule 9 makes "both halves" plain decimal.
    pub red: (u64, u64),
    pub freeze: String,
    /// PB §11: "`run=` present ⇒ verifies under `spine-seal@v1` only; absent ⇒
    /// `spine-review@v1` only" — which is why this field decides a namespace
    /// rather than merely recording one (see [`crate::verify`]).
    pub run: Option<String>,
    /// PB §11: "`held=false` marks B still breaking at the cap".
    pub held: Option<bool>,
    pub reason: Option<String>,
    pub signer: String,
}

const APPROVE_KEYS: &[Key] = &[
    Key::req("intent"),
    Key::req("base"),
    Key::req("rounds"),
    Key::req("total_rounds"),
    Key::req("reopens"),
    Key::req("red"),
    Key::req("freeze"),
    Key::opt("run"),
    Key::opt("held"),
    Key::opt("reason"),
    Key::req("signer"),
];

impl Approve {
    pub fn parse(payload: &[u8]) -> Result<Self, EnvelopeError> {
        let (pos, v) = take(payload, 1, APPROVE_KEYS)?;
        let red = v[5].unwrap();
        let slash = red
            .iter()
            .position(|&b| b == b'/')
            .ok_or_else(|| EnvelopeError::malformed("red= is not k/n"))?;
        Ok(Approve {
            id: as_str(pos[0], "the intent id")?.to_owned(),
            intent: oid(v[0].unwrap(), "intent")?,
            base: oid(v[1].unwrap(), "base")?,
            rounds: counter(v[2].unwrap(), "rounds")?,
            total_rounds: counter(v[3].unwrap(), "total_rounds")?,
            reopens: counter(v[4].unwrap(), "reopens")?,
            red: (
                counter(&red[..slash], "red's numerator")?,
                counter(&red[slash + 1..], "red's denominator")?,
            ),
            freeze: sha256_field(v[6].unwrap(), "freeze")?,
            run: v[7].map(|x| sha256_field(x, "run")).transpose()?,
            held: v[8]
                .map(|x| match x {
                    b"false" => Ok(false),
                    b"true" => Ok(true),
                    other => Err(EnvelopeError::malformed(format!(
                        "held= is not a boolean: {}",
                        show(other)
                    ))),
                })
                .transpose()?,
            reason: v[9].map(|x| parse_json_string(x, "reason")).transpose()?,
            signer: as_str(v[10].unwrap(), "signer")?.to_owned(),
        })
    }

    pub fn render(&self) -> Vec<u8> {
        let mut keyed: Vec<(&str, Vec<u8>)> = vec![
            ("intent", self.intent.clone().into_bytes()),
            ("base", self.base.clone().into_bytes()),
            ("rounds", self.rounds.to_string().into_bytes()),
            ("total_rounds", self.total_rounds.to_string().into_bytes()),
            ("reopens", self.reopens.to_string().into_bytes()),
            (
                "red",
                format!("{}/{}", self.red.0, self.red.1).into_bytes(),
            ),
            ("freeze", self.freeze.clone().into_bytes()),
        ];
        if let Some(r) = &self.run {
            keyed.push(("run", r.clone().into_bytes()));
        }
        if let Some(h) = self.held {
            keyed.push(("held", h.to_string().into_bytes()));
        }
        if let Some(r) = &self.reason {
            keyed.push(("reason", render_json_string(r)));
        }
        keyed.push(("signer", self.signer.clone().into_bytes()));
        render_fields(&[self.id.as_bytes()], &keyed)
    }
}

/// PB §11's `voids=`: "names the binding approval's freeze, `none` only when no
/// approval exists; G13 refuses otherwise."
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Voids {
    None,
    Freeze(String),
}

/// PB §11: `INT-042 voids=sha256:<freeze digest>|none reopens=n reason="…"
/// signer=<p>`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reopen {
    pub id: String,
    pub voids: Voids,
    pub reopens: u64,
    pub reason: String,
    pub signer: String,
}

const REOPEN_KEYS: &[Key] = &[
    Key::req("voids"),
    Key::req("reopens"),
    Key::req("reason"),
    Key::req("signer"),
];

impl Reopen {
    pub fn parse(payload: &[u8]) -> Result<Self, EnvelopeError> {
        let (pos, v) = take(payload, 1, REOPEN_KEYS)?;
        Ok(Reopen {
            id: as_str(pos[0], "the intent id")?.to_owned(),
            voids: match v[0].unwrap() {
                b"none" => Voids::None,
                other => Voids::Freeze(sha256_field(other, "voids")?),
            },
            reopens: counter(v[1].unwrap(), "reopens")?,
            reason: parse_json_string(v[2].unwrap(), "reason")?,
            signer: as_str(v[3].unwrap(), "signer")?.to_owned(),
        })
    }

    pub fn render(&self) -> Vec<u8> {
        let voids = match &self.voids {
            Voids::None => b"none".to_vec(),
            Voids::Freeze(f) => f.clone().into_bytes(),
        };
        render_fields(
            &[self.id.as_bytes()],
            &[
                ("voids", voids),
                ("reopens", self.reopens.to_string().into_bytes()),
                ("reason", render_json_string(&self.reason)),
                ("signer", self.signer.clone().into_bytes()),
            ],
        )
    }
}

/// PB §11: `INT-042 blob=<oid> [orphaned=<principal>] reason="…" signer=<p>`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Withdraw {
    pub id: String,
    pub blob: String,
    /// Present when the sign-off could not be copied because its key is no
    /// longer in the keyring at `base=` (PB §5.5).
    pub orphaned: Option<String>,
    pub reason: String,
    pub signer: String,
}

const WITHDRAW_KEYS: &[Key] = &[
    Key::req("blob"),
    Key::opt("orphaned"),
    Key::req("reason"),
    Key::req("signer"),
];

impl Withdraw {
    pub fn parse(payload: &[u8]) -> Result<Self, EnvelopeError> {
        let (pos, v) = take(payload, 1, WITHDRAW_KEYS)?;
        Ok(Withdraw {
            id: as_str(pos[0], "the intent id")?.to_owned(),
            blob: oid(v[0].unwrap(), "blob")?,
            orphaned: v[1].map(|x| as_str(x, "orphaned")).transpose()?.map(str::to_owned),
            reason: parse_json_string(v[2].unwrap(), "reason")?,
            signer: as_str(v[3].unwrap(), "signer")?.to_owned(),
        })
    }

    pub fn render(&self) -> Vec<u8> {
        let mut keyed: Vec<(&str, Vec<u8>)> = vec![("blob", self.blob.clone().into_bytes())];
        if let Some(o) = &self.orphaned {
            keyed.push(("orphaned", o.clone().into_bytes()));
        }
        keyed.push(("reason", render_json_string(&self.reason)));
        keyed.push(("signer", self.signer.clone().into_bytes()));
        render_fields(&[self.id.as_bytes()], &keyed)
    }
}

/// PB §11: `from=<A> to=<B> manifest=<blob oid> forced=<paths>
/// [from-manifest=<sha>] [since=<sha>] signer=<p>`, with MF §6.4's
/// `<oid|none>` for `manifest` and its `forced=` grammar.
///
/// No positional field: a lifecycle event names no intent (PB §11's
/// `Spine-Intent` row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upgrade {
    /// MF §6.4: "a `cli.version` (§3.2), or `none` for a re-init".
    pub from: String,
    /// "a `cli.version`, or `none` for an uninstall".
    pub to: String,
    /// "the git blob id of `.spine/manifest.json` in `T`, or `none` when
    /// `to=none`".
    pub manifest: String,
    pub forced: Vec<Vec<u8>>,
    /// "mandatory on a rollback, absent otherwise".
    pub from_manifest: Option<String>,
    /// "mandatory on a re-init (`from=none`), absent otherwise".
    pub since: Option<String>,
    pub signer: String,
}

const UPGRADE_KEYS: &[Key] = &[
    Key::req("from"),
    Key::req("to"),
    Key::req("manifest"),
    Key::req("forced"),
    Key::opt("from-manifest"),
    Key::opt("since"),
    Key::req("signer"),
];

impl Upgrade {
    pub fn parse(payload: &[u8]) -> Result<Self, EnvelopeError> {
        let (_, v) = take(payload, 0, UPGRADE_KEYS)?;
        Ok(Upgrade {
            from: as_str(v[0].unwrap(), "from")?.to_owned(),
            to: as_str(v[1].unwrap(), "to")?.to_owned(),
            manifest: match v[2].unwrap() {
                b"none" => "none".to_owned(),
                other => oid(other, "manifest")?,
            },
            forced: parse_forced(v[3].unwrap())?,
            from_manifest: v[4].map(|x| oid(x, "from-manifest")).transpose()?,
            since: v[5].map(|x| oid(x, "since")).transpose()?,
            signer: as_str(v[6].unwrap(), "signer")?.to_owned(),
        })
    }

    pub fn render(&self) -> Vec<u8> {
        let mut keyed: Vec<(&str, Vec<u8>)> = vec![
            ("from", self.from.clone().into_bytes()),
            ("to", self.to.clone().into_bytes()),
            ("manifest", self.manifest.clone().into_bytes()),
            ("forced", render_forced(&self.forced).into_bytes()),
        ];
        if let Some(f) = &self.from_manifest {
            keyed.push(("from-manifest", f.clone().into_bytes()));
        }
        if let Some(s) = &self.since {
            keyed.push(("since", s.clone().into_bytes()));
        }
        keyed.push(("signer", self.signer.clone().into_bytes()));
        render_fields(&[], &keyed)
    }
}

/// PB §11: `INT-042|quick|reseal class=… head=<sha> tree=<oid> base=<sha>
/// [intent=<oid>] report=sha256:<hex> wires=… reason="…" reviewer=<p>`
///
/// "the first field is the seal's and `intent=` is present only when the
/// landing has one; for `class=break-glass`, `wires=` lists the gates
/// bypassed."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Review {
    pub subject: String,
    pub class: ReviewClass,
    /// PB §11: "`head=` is the content head `Hc` (§5.4)".
    pub head: String,
    /// GR §9.2 and EV §8.4: a review's `tree=` names `T`, the synthetic merge
    /// every gate evaluated — **not** the seal's `tree=`, which names `L`'s.
    pub tree: String,
    pub base: String,
    pub intent: Option<String>,
    /// EV §5: a review's `report=` names **evaluation 1**, the non-landing
    /// report containing the `fail` the reviewer read and accepted.
    pub report: String,
    pub wires: Vec<String>,
    pub reason: String,
    pub reviewer: String,
}

const REVIEW_KEYS: &[Key] = &[
    Key::req("class"),
    Key::req("head"),
    Key::req("tree"),
    Key::req("base"),
    Key::opt("intent"),
    Key::req("report"),
    Key::req("wires"),
    Key::req("reason"),
    Key::req("reviewer"),
];

impl Review {
    pub fn parse(payload: &[u8]) -> Result<Self, EnvelopeError> {
        let (pos, v) = take(payload, 1, REVIEW_KEYS)?;
        Ok(Review {
            subject: as_str(pos[0], "the review's first field")?.to_owned(),
            class: token(v[0].unwrap(), "class", ReviewClass::parse)?,
            head: oid(v[1].unwrap(), "head")?,
            tree: oid(v[2].unwrap(), "tree")?,
            base: oid(v[3].unwrap(), "base")?,
            intent: v[4].map(|x| oid(x, "intent")).transpose()?,
            report: sha256_field(v[5].unwrap(), "report")?,
            wires: parse_wires(v[6].unwrap())?,
            reason: parse_json_string(v[7].unwrap(), "reason")?,
            reviewer: as_str(v[8].unwrap(), "reviewer")?.to_owned(),
        })
    }

    pub fn render(&self) -> Vec<u8> {
        let mut keyed: Vec<(&str, Vec<u8>)> = vec![
            ("class", self.class.token().as_bytes().to_vec()),
            ("head", self.head.clone().into_bytes()),
            ("tree", self.tree.clone().into_bytes()),
            ("base", self.base.clone().into_bytes()),
        ];
        if let Some(i) = &self.intent {
            keyed.push(("intent", i.clone().into_bytes()));
        }
        keyed.push(("report", self.report.clone().into_bytes()));
        keyed.push(("wires", render_wires(&self.wires).into_bytes()));
        keyed.push(("reason", render_json_string(&self.reason)));
        keyed.push(("reviewer", self.reviewer.clone().into_bytes()));
        render_fields(&[self.subject.as_bytes()], &keyed)
    }
}

/// PB §11's `tool=<version>+sha256:<dist_hash>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    pub version: String,
    pub dist_hash: String,
}

impl Tool {
    pub fn parse(value: &[u8]) -> Result<Self, EnvelopeError> {
        let s = as_str(value, "tool")?;
        let plus = s
            .find('+')
            .ok_or_else(|| EnvelopeError::malformed(format!("tool= is not <version>+<hash>: {s}")))?;
        Ok(Tool {
            version: s[..plus].to_owned(),
            dist_hash: sha256_field(&s.as_bytes()[plus + 1..], "tool's dist_hash")?,
        })
    }

    pub fn render(&self) -> Vec<u8> {
        format!("{}+{}", self.version, self.dist_hash).into_bytes()
    }
}

/// PB §11's seal row: `INT-042|quick|reseal base=<sha> head=<sha> tree=<oid>
/// report=sha256:<hex> tool=<version>+sha256:<dist_hash> git=<major.minor>
/// mode=… threat=… profile=… envelope=sha256:<hex> signer=<p>`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seal {
    /// "the first field is the landing's identity: the intent id for a gated
    /// landing or tombstone, `quick` for a quick-lane landing **and for every
    /// toolkit lifecycle landing**, `reseal` for a reseal" (PB §11).
    pub subject: String,
    pub base: String,
    pub head: String,
    /// "`tree=` names `L`'s tree, so G9 checks it from `L` alone" (PB §11).
    pub tree: String,
    /// EV §5: the seal's `report=` names the **sealing** evaluation, in which
    /// the overridden gate reads `override` and the review has entered
    /// `authority.reviews`. "An implementation that puts one digest in both
    /// places has collapsed two evaluations into one" (EV §8.4).
    pub report: String,
    pub tool: Tool,
    /// PB §11 caps it at major.minor because it "is a capability record, not an
    /// environment probe" (EV §7 rule 3).
    pub git: String,
    pub mode: Mode,
    pub threat: Threat,
    pub profile: Profile,
    pub envelope: String,
    pub signer: String,
}

const SEAL_KEYS: &[Key] = &[
    Key::req("base"),
    Key::req("head"),
    Key::req("tree"),
    Key::req("report"),
    Key::req("tool"),
    Key::req("git"),
    Key::req("mode"),
    Key::req("threat"),
    Key::req("profile"),
    Key::req("envelope"),
    Key::req("signer"),
];

impl Seal {
    pub fn parse(payload: &[u8]) -> Result<Self, EnvelopeError> {
        let (pos, v) = take(payload, 1, SEAL_KEYS)?;
        Ok(Seal {
            subject: as_str(pos[0], "the seal's first field")?.to_owned(),
            base: oid(v[0].unwrap(), "base")?,
            head: oid(v[1].unwrap(), "head")?,
            tree: oid(v[2].unwrap(), "tree")?,
            report: sha256_field(v[3].unwrap(), "report")?,
            tool: Tool::parse(v[4].unwrap())?,
            git: as_str(v[5].unwrap(), "git")?.to_owned(),
            mode: token(v[6].unwrap(), "mode", Mode::parse)?,
            threat: token(v[7].unwrap(), "threat", Threat::parse)?,
            profile: token(v[8].unwrap(), "profile", Profile::parse)?,
            envelope: sha256_field(v[9].unwrap(), "envelope")?,
            signer: as_str(v[10].unwrap(), "signer")?.to_owned(),
        })
    }

    pub fn render(&self) -> Vec<u8> {
        render_fields(
            &[self.subject.as_bytes()],
            &[
                ("base", self.base.clone().into_bytes()),
                ("head", self.head.clone().into_bytes()),
                ("tree", self.tree.clone().into_bytes()),
                ("report", self.report.clone().into_bytes()),
                ("tool", self.tool.render()),
                ("git", self.git.clone().into_bytes()),
                ("mode", self.mode.token().as_bytes().to_vec()),
                ("threat", self.threat.token().as_bytes().to_vec()),
                ("profile", self.profile.token().as_bytes().to_vec()),
                ("envelope", self.envelope.clone().into_bytes()),
                ("signer", self.signer.clone().into_bytes()),
            ],
        )
    }
}

// ---------------------------------------------------------------------------
// The manifest lines
// ---------------------------------------------------------------------------

/// PB §11 and PB §4.3: `<oid> <path>` (`git ls-tree` quoting).
///
/// "the payload splits at its **first** space, and everything after it is the
/// path field" (EV §4.3) — because a space does not trigger quoting and a real
/// path may hold one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frozen {
    pub oid: String,
    /// The repository's own bytes, decoded. The *hashed* form is the quoted
    /// one; EV §4.1 hashes whole lines precisely so that nothing has to unquote
    /// before hashing.
    pub path: Vec<u8>,
}

impl Frozen {
    pub fn parse(payload: &[u8]) -> Result<Self, EnvelopeError> {
        let space = payload
            .iter()
            .position(|&b| b == b' ')
            .ok_or_else(|| EnvelopeError::malformed("Spine-Frozen payload has no space"))?;
        Ok(Frozen {
            oid: oid(&payload[..space], "a frozen blob id")?,
            path: unquote_path(&payload[space + 1..])?,
        })
    }

    pub fn render(&self) -> Vec<u8> {
        let mut out = self.oid.clone().into_bytes();
        out.push(b' ');
        out.extend_from_slice(&quote_path(&self.path));
        out
    }
}

/// PB §11: `<runner> <runner-native function id>` without parametrization
/// suffix.
///
/// "`result-file.md` §4.4 fixes the runner token as `[a-z][a-z0-9_-]{0,31}` —
/// no uppercase, no space, no colon — so the split at the **first** space is
/// exact even though a function id may itself contain spaces (vitest's
/// `>`-joined names do)" (EV §4.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Test {
    pub runner: String,
    /// "Ids are runner-native and are never rewritten. No escaping is applied
    /// and none is available" (EV §4.4).
    pub id: Vec<u8>,
}

impl Test {
    pub fn parse(payload: &[u8]) -> Result<Self, EnvelopeError> {
        let space = payload
            .iter()
            .position(|&b| b == b' ')
            .ok_or_else(|| EnvelopeError::malformed("Spine-Test payload has no space"))?;
        let runner = as_str(&payload[..space], "a runner token")?;
        // RF §4.4's lexical form, adopted by EV §4.4.
        let ok = (1..=32).contains(&runner.len())
            && runner.starts_with(|c: char| c.is_ascii_lowercase())
            && runner
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
        if !ok {
            return Err(EnvelopeError::malformed(format!(
                "runner token out of grammar: {runner}"
            )));
        }
        let id = payload[space + 1..].to_vec();
        if id.is_empty() {
            return Err(EnvelopeError::malformed("Spine-Test id is empty"));
        }
        Ok(Test {
            runner: runner.to_owned(),
            id,
        })
    }

    /// EV §4.4: an id holding `0x0A`, `0x0D` or `0x00` "cannot be represented
    /// in a trailer at all … `spine check --approve` refuses to freeze such an
    /// id (`test-id-unrepresentable`) rather than mangling it. A result file
    /// may carry one — `result-file.md`'s JSON strings can encode `\n` — so the
    /// refusal has to be here, at the boundary where the id becomes a line."
    pub fn check_representable(id: &[u8]) -> Result<(), EnvelopeError> {
        match id.iter().position(|&b| b == 0x0A || b == 0x0D || b == 0x00) {
            Some(i) => Err(EnvelopeError::new(
                Refusal::TestIdUnrepresentable,
                format!("id holds 0x{:02X} at byte {i}", id[i]),
            )),
            None => Ok(()),
        }
    }

    pub fn render(&self) -> Result<Vec<u8>, EnvelopeError> {
        Test::check_representable(&self.id)?;
        let mut out = self.runner.clone().into_bytes();
        out.push(b' ');
        out.extend_from_slice(&self.id);
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// `Spine-Gates`
// ---------------------------------------------------------------------------

/// PB §11: `G1=pass … G16=pass` — "every gate that ran, never G10".
///
/// The order is the report's `gates[]` order, which "GR §5.6 fixes as ascending
/// by the integer after `G`" (EV §7 rule 12) — **not** the wire order. "A
/// lexical `Spine-Gates` order — `G1 G10 G11 G12 G13 G14 G15 G16 G2 G3 …` — is
/// non-conforming and changes `envelope=`."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gates(pub Vec<(u32, GateStatus)>);

impl Gates {
    pub fn parse(payload: &[u8]) -> Result<Self, EnvelopeError> {
        let mut out: Vec<(u32, GateStatus)> = Vec::new();
        for field in split_fields(payload)? {
            let (k, v) = crate::trailer::split_kv(field).ok_or_else(|| {
                EnvelopeError::malformed(format!("Spine-Gates entry is not G<n>=<status>: {}", show(field)))
            })?;
            let n = as_str(k, "a gate id")?
                .strip_prefix('G')
                .ok_or_else(|| EnvelopeError::malformed(format!("gate id has no G: {}", show(k))))?;
            let n: u32 = counter(n.as_bytes(), "a gate number")? as u32;
            if !(1..=16).contains(&n) {
                return Err(EnvelopeError::malformed(format!("gate G{n} does not exist")));
            }
            // PB §11: "never G10 (it runs after the seal)". GR §5.6.2: G6 has no
            // entry in a version-1 report, because "iff configured" would make
            // two implementations disagree about the length of the line — and
            // therefore about `envelope=` (EV §13.7).
            if n == 10 || n == 6 {
                return Err(EnvelopeError::malformed(format!(
                    "G{n} has no Spine-Gates entry in a version-1 landing"
                )));
            }
            if let Some(&(prev, _)) = out.last()
                && prev >= n
            {
                return Err(EnvelopeError::malformed(format!(
                    "Spine-Gates is not ascending by gate number: G{prev} then G{n}"
                )));
            }
            out.push((n, token(v, "a gate status", GateStatus::parse)?));
        }
        Ok(Gates(out))
    }

    pub fn render(&self) -> Vec<u8> {
        let mut sorted = self.0.clone();
        sorted.sort_by_key(|&(n, _)| n);
        sorted
            .iter()
            .map(|(n, s)| format!("G{n}={s}"))
            .collect::<Vec<_>>()
            .join(" ")
            .into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VECTOR_A_SIGNOFF: &[u8] = b"INT-042 blob=dfb4079e22de55ec377468b9b697fdf86085ea37 template=intent@2 constitution=v3 reopens=1 signer=alice@example.com";
    const VECTOR_A_APPROVE: &[u8] = b"INT-042 intent=dfb4079e22de55ec377468b9b697fdf86085ea37 base=5f8e2a10cd47b6390e5a2c8db14f70369ba2e0d7 rounds=1 total_rounds=3 reopens=1 red=5/5 freeze=sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2 signer=alice@example.com";
    const VECTOR_A_GATES: &[u8] = b"G1=pass G2=override G3=pass G4=pass G5=pass G7=pass G8=pass G9=pass G11=pass G12=pass G13=pass G14=pass G15=pass G16=pass";

    #[test]
    fn vector_a_signoff_round_trips_byte_for_byte() {
        let s = Signoff::parse(VECTOR_A_SIGNOFF).unwrap();
        assert_eq!(s.template, "intent@2", "the owner's 2026-08-26 decision (b)");
        assert_eq!(s.reopens, 1);
        assert_eq!(s.render(), VECTOR_A_SIGNOFF.to_vec());
    }

    #[test]
    fn vector_a_approve_round_trips_byte_for_byte() {
        let a = Approve::parse(VECTOR_A_APPROVE).unwrap();
        assert_eq!(a.red, (5, 5));
        assert!(a.run.is_none(), "absent ⇒ verifies under spine-review@v1");
        assert_eq!(a.render(), VECTOR_A_APPROVE.to_vec());
    }

    #[test]
    fn vector_a_gates_lists_fourteen_ascending_by_number() {
        let g = Gates::parse(VECTOR_A_GATES).unwrap();
        assert_eq!(g.0.len(), 14, "EV §13.7: a gated landing lists fourteen");
        assert_eq!(g.0[1], (2, GateStatus::Override));
        assert!(g.0.iter().all(|&(n, _)| n != 6 && n != 10));
        assert_eq!(g.render(), VECTOR_A_GATES.to_vec());
    }

    #[test]
    fn a_lexical_spine_gates_order_is_non_conforming() {
        // EV §7 rule 12 names the wrong line explicitly.
        assert!(Gates::parse(b"G1=pass G11=pass G2=pass").is_err());
    }

    #[test]
    fn g10_and_g6_have_no_entry() {
        assert!(Gates::parse(b"G9=pass G10=pass").is_err());
        assert!(Gates::parse(b"G5=pass G6=pass").is_err());
    }

    #[test]
    fn a_sealed_gate_status_is_never_fail() {
        assert!(Gates::parse(b"G1=fail").is_err());
    }

    #[test]
    fn a_numeric_wire_comparator_is_non_conforming() {
        // PB §11, verbatim: "ascending by unsigned byte value over the whole
        // token, so `G11` precedes `G2`".
        let wires = vec!["G2:src/shared/util.ts".to_owned(), "G11".to_owned()];
        assert_eq!(render_wires(&wires), "G11,G2:src/shared/util.ts");
        assert!(
            parse_wires(b"G2:src/shared/util.ts,G11").is_err(),
            "the numeric order is refused, not silently re-sorted"
        );
        assert_eq!(
            parse_wires(b"G11,G2:src/shared/util.ts").unwrap(),
            vec!["G11", "G2:src/shared/util.ts"]
        );
    }

    #[test]
    fn a_pathless_token_precedes_every_suffixed_one_of_its_gate() {
        // GR §6.1: "its token is a proper prefix of theirs".
        assert_eq!(render_wires(&["G8:a".to_owned(), "G8".to_owned()]), "G8,G8:a");
    }

    #[test]
    fn a_wire_token_uses_tok_and_never_the_frozen_quoting() {
        // EV §13.9's asymmetry, in one assertion.
        let path = "tests/fixtures/café.json".as_bytes();
        assert_eq!(
            wire_token(8, Some(path)),
            "G8:tests/fixtures/caf\\xc3\\xa9.json"
        );
        assert_eq!(
            Frozen {
                oid: "0c3a7f18e2b56d94a0c7f3e18b52d6a4907c1e3f".to_owned(),
                path: path.to_vec(),
            }
            .render(),
            br#"0c3a7f18e2b56d94a0c7f3e18b52d6a4907c1e3f "tests/fixtures/caf\303\251.json""#.to_vec()
        );
    }

    #[test]
    fn a_frozen_path_splits_at_the_first_space() {
        // EV §4.3 and vector C's `tests/a b.py`.
        let f = Frozen::parse(b"7f3aa0c19b48d6250e3f7a1c85b09d24e6f31a70 tests/a b.py").unwrap();
        assert_eq!(f.path, b"tests/a b.py".to_vec());
        assert_eq!(
            f.render(),
            b"7f3aa0c19b48d6250e3f7a1c85b09d24e6f31a70 tests/a b.py".to_vec()
        );
    }

    #[test]
    fn a_test_id_splits_at_the_first_space_and_keeps_the_rest() {
        let t = Test::parse(b"vitest tests/billing/invoice.test.ts > invoice totals > AC1 includes tax")
            .unwrap();
        assert_eq!(t.runner, "vitest");
        assert_eq!(t.id, b"tests/billing/invoice.test.ts > invoice totals > AC1 includes tax".to_vec());
        assert_eq!(
            t.render().unwrap(),
            b"vitest tests/billing/invoice.test.ts > invoice totals > AC1 includes tax".to_vec()
        );
    }

    #[test]
    fn an_id_holding_lf_is_unrepresentable_rather_than_mangled() {
        let t = Test {
            runner: "pytest".to_owned(),
            id: b"a\nb".to_vec(),
        };
        assert_eq!(
            t.render().unwrap_err().refusal(),
            Refusal::TestIdUnrepresentable
        );
    }

    #[test]
    fn a_runner_token_has_no_uppercase_and_no_colon() {
        // RF §4.4, which is what makes the first-space split exact.
        assert!(Test::parse(b"Pytest a::b").is_err());
        assert!(Test::parse(b"py:test a::b").is_err());
        assert!(Test::parse(b"dart-test a::b").is_ok());
    }

    #[test]
    fn an_empty_forced_list_is_the_empty_value() {
        // MF §6.4: `none` is rejected because `tok("none")` is a legal path.
        assert_eq!(parse_forced(b"").unwrap(), Vec::<Vec<u8>>::new());
        assert_eq!(render_forced(&[]), "");
        assert_eq!(parse_forced(b"none").unwrap(), vec![b"none".to_vec()]);
        assert!(parse_forced(b"a,,b").is_err());
        assert!(parse_forced(b",a").is_err());
    }

    #[test]
    fn a_forced_path_with_a_comma_survives_tok() {
        let paths = vec![b"a,b.ts".to_vec(), b"c d.ts".to_vec()];
        let rendered = render_forced(&paths);
        assert_eq!(rendered, "a\\x2cb.ts,c\\x20d.ts");
        assert_eq!(parse_forced(rendered.as_bytes()).unwrap(), paths);
    }

    #[test]
    fn an_upgrade_line_round_trips() {
        // MF §6.6's rollback line shape, with a concrete `from-manifest=`.
        let line = b"from=1.4.0 to=1.3.0 manifest=74806e98701b50e958074dbaad0d7509d84751a3 forced= from-manifest=5f8e2a10cd47b6390e5a2c8db14f70369ba2e0d7 signer=alice@example.com";
        let u = Upgrade::parse(line).unwrap();
        assert_eq!(u.to, "1.3.0");
        assert!(u.forced.is_empty());
        assert_eq!(u.render(), line.to_vec());
    }

    #[test]
    fn a_reason_keeps_its_spaces_and_round_trips() {
        let line = br#"INT-042 voids=none reopens=1 reason="AC-3 was not testable as written" signer=alice@example.com"#;
        let r = Reopen::parse(line).unwrap();
        assert_eq!(r.reason, "AC-3 was not testable as written");
        assert_eq!(r.voids, Voids::None);
        assert_eq!(r.render(), line.to_vec());
    }

    #[test]
    fn every_closed_set_refuses_a_value_outside_it() {
        assert!(Event::parse(b"merge").is_none());
        assert!(Lane::parse(b"quiet").is_none());
        assert!(Strategy::parse(b"rebase").is_none(), "PB §5.5 refuses rebase");
        assert!(Profile::parse(b"n/a").is_some(), "the tombstone's profile");
        assert!(Mode::parse(b"recovery").is_some());
    }
}
