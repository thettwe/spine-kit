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
    /// No `.spine/manifest.json` on trunk — the repository is not initialised,
    /// or the collector was pointed at the wrong ref.
    NoManifestOnTrunk,
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
                "no .spine/manifest.json on trunk: policy is read from there and nowhere else",
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
    /// Step 4's probe, performed by the caller.
    pub keys_visible: bool,
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
    // ---- Step 1: policy, from `origin/<trunk>`, never from the checkout.
    let base = git
        .rev_parse(refs.trunk)
        .ok_or_else(|| PrepareError::TrunkUnresolvable(refs.trunk.to_string()))?;
    let head = git
        .rev_parse(refs.head)
        .ok_or_else(|| PrepareError::HeadUnresolvable(refs.head.to_string()))?;

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
            keys_visible: me.keys_visible,
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
