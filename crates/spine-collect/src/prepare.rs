//! RF §7.1 steps 1-5 — everything the collector does **before** it may write a
//! file, and every refusal it has.
//!
//! RF §7.3 draws the line this module stops at: *"The collector always writes a
//! file once `T` is known and policy has been read."* Before that line every
//! fault is a refusal that writes nothing; after it, every fault is a status in
//! a file. So the five steps live apart from [`crate::collector::collect`],
//! which starts at step 6 and can only produce a file.
//!
//! **Every input here comes from `origin/<trunk>`, and that is the point.**
//! PB §7.4 rule 1: "the trusted stage **and the collector** read
//! `.spine/manifest.json` … from `origin/<trunk>` … never from the checkout
//! under test." A candidate that could move its own timeout, its own language
//! set or its own isolation request would be choosing the terms it is judged
//! on.

use spine_canon::ObjectFormat;
use spine_manifest::Manifest;

use crate::collector::{Mode, Policy, Refusal, Release, Run, invocation_set};
use crate::record::RunnerToken;

/// The git reads steps 1 and 5 need, and nothing else.
///
/// A trait rather than a concrete repository so that the five steps are
/// testable without one: every refusal below is reachable from a handful of
/// strings, and a test that needs a real repository to reach a refusal is a
/// test that will not be written for all of them.
pub trait Git {
    /// `git rev-parse <rev>^{commit}` — the full oid, or `None` where the ref
    /// does not resolve.
    fn rev_parse(&self, rev: &str) -> Option<String>;

    /// The blob at `<rev>:<path>`, or `None` where the path is not in that
    /// tree.
    fn blob_at(&self, rev: &str, path: &str) -> Option<Vec<u8>>;

    /// `git merge-tree --write-tree <base> <head>`: the written tree's oid, or
    /// `None` for a conflict.
    ///
    /// RF §7.1 step 5 needs only the two outcomes. The conflict *report* is the
    /// trusted stage's business at step 1 of PB §5.4, which "detects
    /// `needs-rebase` independently … and does not need a file to do it".
    fn merge_tree(&self, base: &str, head: &str) -> Option<String>;

    /// First-parent commit messages from `rev`, newest first — the walk a
    /// reseal's `base=` is found on. Empty where `rev` does not resolve.
    fn first_parent_messages(&self, rev: &str) -> Vec<(String, String)>;
}

/// PB §5.5's reseal, which reads policy from somewhere other than trunk's tip.
///
/// > "a **reseal** is a quick-lane landing with `Spine-Event: reseal`, parent =
/// > the orphan tip `O`, tree identical to `O`'s, seal `base=` **the last valid
/// > landing below the range** and `head=O`"
///
/// RF §4.2's `base` row: "`origin/<trunk>` tip at the moment the collector read
/// policy — **for a reseal, the seal's `base=`, from which every policy read for
/// a reseal is taken**", and RF §8.6 adds "**`params.langs` and `params.timeout`
/// included**".
///
/// **Why it cannot be skipped.** On `refs/heads/quick/reseal-<O>`, trunk's tip
/// *is* the orphan — G9 refuses every landing above an orphan until the reseal
/// lands. A collector that resolved `origin/<trunk>` would seal `base=<O>`,
/// read `params.langs`, `params.timeout` and the pin from `O`'s manifest, and
/// verify its own bytes against `O`'s pin. The trusted stage fixed
/// `(T, B) = (tree(O), the seal's base=)`, so RF §8.3 step 1 answers
/// `base-moved` — and this is the one landing shape that can never clear it:
/// "its tree must equal `O`'s, so there is no candidate to fix, and a reseal is
/// not promotable by `spine new --from`", while trunk cannot move while the
/// orphan stands. The repository is bricked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject<'a> {
    /// Every other shape: policy is `origin/<trunk>`'s (PB §7.4 rule 1).
    Trunk,
    /// A reseal of the orphan tip named by the branch, `quick/reseal-<O>`.
    Reseal { orphan: &'a str },
}

/// CI §6.4's router matches `quick/reseal-*` **before** `quick/*`, and names
/// the orphan in the ref: "PB §5.5 puts a reseal's review commits on
/// `refs/heads/quick/reseal-<O>`".
pub fn subject_of(head_ref: &str) -> Subject<'_> {
    let name = head_ref.strip_prefix("refs/heads/").unwrap_or(head_ref);
    match name.strip_prefix("quick/reseal-") {
        Some(orphan) if !orphan.is_empty() => Subject::Reseal { orphan },
        _ => Subject::Trunk,
    }
}

/// PB §5.5's "the last valid landing below the range", as far as a collector
/// can see it.
///
/// **DERIVED, and the derivation is an approximation the corpus does not
/// bound.** "Valid landing" is G9's predicate — a verifying seal, a recomputing
/// `envelope=`, a fenced `blob=` that hashes — and the collector holds no
/// keyring and no envelope verifier: PB §7.4 rule 3 gives it git objects and
/// policy and nothing else. So this takes the newest first-parent commit
/// carrying a `Spine-Seal` **trailer**, which is the syntactic half of the
/// predicate.
///
/// Where the two differ — an orphan carrying a forged seal line — the collector
/// writes a `base=` the trusted stage disagrees with and RF §8.3 step 1 answers
/// `base-moved`, which is the retryable outcome rather than a wrong file. That
/// is the right direction to be wrong in, and it is recorded in
/// `.build-notes/OPEN-questions.md`: the corpus fixes what `base=` **is** and
/// never says how the collector, which cannot verify, is to find it.
pub fn reseal_base(git: &dyn Git, orphan: &str) -> Option<String> {
    git.first_parent_messages(orphan)
        .into_iter()
        // The orphan itself is by definition "neither a landing nor the trust
        // root", so the walk starts below it.
        .skip(1)
        .find(|(_, message)| message.lines().any(|line| line.starts_with("Spine-Seal: ")))
        .map(|(sha, _)| sha)
}

/// Where the collector's own bytes stand against the pinned artifact list.
///
/// RF §7.1 step 2 is a *verification the collector performs on itself*, and it
/// cannot be performed here: the artifact list is the release's and the bytes
/// are the running binary's. The caller performs it and reports the verdict,
/// which keeps step 2's refusal in step order rather than wherever the check
/// happens to live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfBytes {
    /// The running binary is in the pinned list, at its own hash.
    Verified,
    /// "Mismatch: fail the job, write nothing."
    Mismatch,
}

/// What the collector **is** — never what trunk pins.
///
/// RF §4.2: "The collector writes what it **is**, never what trunk pins.
/// Copying the manifest's value would assert nothing." So this is the running
/// release's own pair, and [`Policy::expected_tool_token`] — trunk's — is a
/// different value that only the trusted stage may compute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfIdentity {
    pub version: String,
    /// With or without the `sha256:` prefix; the token is written with exactly
    /// one.
    pub dist_hash: String,
}

impl SelfIdentity {
    /// RF §4.2's `tool=<version>+sha256:<hex>`.
    fn tool_token(&self) -> String {
        let hex = self
            .dist_hash
            .strip_prefix("sha256:")
            .unwrap_or(&self.dist_hash);
        format!("{}+sha256:{}", self.version, hex)
    }
}

/// Steps 1-5 done, and the run they authorize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    pub policy: Policy,
    /// Step 3's set, in language-declaration order (RF §6.2).
    pub invocation: Vec<RunnerToken>,
    pub run: Run,
}

/// The refusals that are not [`Refusal`]'s because they are not RF's.
///
/// RF §7.1 assumes a repository with a trunk ref and a readable manifest on it;
/// where there is not one there is no policy to read, so there is no step 1 and
/// no file. These are named separately rather than folded into `Refusal` so
/// that a run refused for want of a repository is not reported as a run refused
/// by a rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepareError {
    /// `origin/<trunk>` does not resolve.
    TrunkUnresolvable(String),
    /// `H` does not resolve.
    HeadUnresolvable(String),
    /// No `.spine/manifest.json` where policy is read from.
    NoManifestOnTrunk,
    /// `quick/reseal-<O>` names an `<O>` this repository does not have.
    OrphanUnresolvable(String),
    /// PB §5.5's "the last valid landing below the range" is not there: the
    /// first-parent walk below the orphan carries no `Spine-Seal` at all, so
    /// there is nothing for a reseal's `base=` to name.
    NoLandingBelowTheOrphan,
    /// Trunk's manifest does not parse. The token is MF §3.11's.
    TrunkManifestMalformed(String),
    /// One of RF §7.1's own five.
    Refused(Refusal),
}

impl core::fmt::Display for PrepareError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PrepareError::TrunkUnresolvable(r) => write!(f, "{r} does not resolve"),
            PrepareError::HeadUnresolvable(r) => write!(f, "{r} does not resolve"),
            PrepareError::NoManifestOnTrunk => f.write_str(
                "no .spine/manifest.json where policy is read from: trunk's tip, or a reseal's base=",
            ),
            PrepareError::OrphanUnresolvable(o) => {
                write!(f, "quick/reseal-{o} names an orphan this repository does not have")
            }
            PrepareError::NoLandingBelowTheOrphan => f.write_str(
                "no Spine-Seal below the orphan: a reseal's base= is the last valid landing                  below the range, and there is none",
            ),
            PrepareError::TrunkManifestMalformed(t) => write!(f, "trunk's manifest: {t}"),
            PrepareError::Refused(r) => write!(f, "{r}"),
        }
    }
}

impl core::error::Error for PrepareError {}

impl From<Refusal> for PrepareError {
    fn from(r: Refusal) -> Self {
        PrepareError::Refused(r)
    }
}

/// The manifest path, fixed by MF §3 and not configurable.
pub const MANIFEST_PATH: &str = ".spine/manifest.json";

/// The two refs steps 1 and 5 read. Named rather than positional because
/// `(trunk, head)` and `(head, trunk)` are both two strings and only one of
/// them reads policy from trunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refs<'a> {
    /// `origin/<trunk>`. PB §7.4 rule 1 fixes it as the only source of policy.
    pub trunk: &'a str,
    /// `H`, the candidate's content head.
    pub head: &'a str,
}

/// What the running collector knows about **itself** — every value RF §9 puts
/// in the *cannot influence* column, gathered so that none of them can be
/// confused with trunk's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collector<'a> {
    pub mode: Mode,
    /// Step 2's verdict, performed by the caller (see [`SelfBytes`]).
    pub self_bytes: SelfBytes,
    /// Step 4's probe, performed by the caller — [`crate::keys::probe`] or
    /// [`crate::keys::probe_this_process`], never a literal.
    ///
    /// RF §4.2 makes it a predicate over "the collector process **or** any
    /// process group it spawned", and the whole job gets one answer: "the field
    /// is not per-runner, and a collector that strips key material for one
    /// runner and not another writes `true`."
    pub keys: crate::keys::Probe,
    pub identity: &'a SelfIdentity,
}

/// RF §7.1 steps 1-5, **in that order**, refusing at the first that fails.
///
/// The order is not incidental and is tested. Step 2 precedes step 3 because a
/// collector whose own bytes are wrong should not be trusted to say which
/// languages it supports; step 5 is last because it is the only step that
/// writes an object, and every reason to refuse should already have fired.
///
/// [`Collector::keys_visible`] is step 4's, taken as an argument for the same
/// reason [`SelfBytes`] is: the probe is the host's, and RF §7.4 settles the
/// value before any observation under [`Mode::Solo`], so a collector that
/// measured it here would measure it in the one mode where the answer is
/// fixed.
pub fn prepare(
    git: &dyn Git,
    refs: Refs<'_>,
    me: &Collector<'_>,
    release: &dyn Release,
) -> Result<Prepared, PrepareError> {
    // ---- Step 1: policy, from `origin/<trunk>`, never from the checkout —
    // **except on a reseal**, where RF §4.2 and §8.6 send every policy read to
    // the seal's `base=` instead. See [`Subject`] for why skipping it bricks
    // the repository.
    let head = git
        .rev_parse(refs.head)
        .ok_or_else(|| PrepareError::HeadUnresolvable(refs.head.to_string()))?;
    let base = match subject_of(refs.head) {
        Subject::Trunk => git
            .rev_parse(refs.trunk)
            .ok_or_else(|| PrepareError::TrunkUnresolvable(refs.trunk.to_string()))?,
        Subject::Reseal { orphan } => {
            let orphan = git
                .rev_parse(orphan)
                .ok_or_else(|| PrepareError::OrphanUnresolvable(orphan.to_string()))?;
            reseal_base(git, &orphan).ok_or(PrepareError::NoLandingBelowTheOrphan)?
        }
    };

    let bytes = git
        .blob_at(&base, MANIFEST_PATH)
        .ok_or(PrepareError::NoManifestOnTrunk)?;
    // `None` for the repository format: trunk's manifest states its own
    // `object_format`, and there is nothing here to cross-check it against —
    // the collector reads policy, it does not lint the repository.
    let trunk = Manifest::parse(&bytes, None)
        .map_err(|r| PrepareError::TrunkManifestMalformed(r.status.token().to_string()))?;
    let policy = Policy::read(&trunk, me.mode)?;

    // ---- Step 2: its own bytes, against the pinned artifact list.
    if me.self_bytes == SelfBytes::Mismatch {
        return Err(Refusal::ToolBytesMismatch.into());
    }

    // ---- Step 3: the invocation set.
    let invocation = invocation_set(&policy, release)?;

    // ---- Step 4 is the caller's probe; step 5 writes the tree.
    let tree = git
        .merge_tree(&base, &head)
        .ok_or(Refusal::MergeConflict.into_prepare())?;

    Ok(Prepared {
        run: Run {
            tree,
            base,
            tool: me.identity.tool_token(),
            keys_visible: me.keys.keys_visible(),
        },
        invocation,
        policy,
    })
}

impl Refusal {
    fn into_prepare(self) -> PrepareError {
        PrepareError::Refused(self)
    }
}

/// The `object_format` step 1 read, for a caller that needs it before a
/// [`Prepared`] exists.
pub fn trunk_object_format(prepared: &Prepared) -> ObjectFormat {
    prepared.policy.object_format
}
