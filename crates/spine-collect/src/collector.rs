//! RF §7.1's ten steps as an algorithm, §7.2's reduction, §7.3's fold.
//!
//! Everything that touches a process, a filesystem or a runner is behind a
//! trait ([`Host`], [`RunnerAdapter`], [`Release`]). What is left is the part
//! the corpus argues about: the *order*, which is a security property, and the
//! *reduction*, which is the file's determinism.
//!
//! **The order is the property.** RF §7.1 step 7: "every invocation of either
//! kind is a `B` invocation and all of them precede every `T` execution, which
//! is what keeps rule 3's property intact … **Multi-runner sharpens this rather
//! than relaxing it**: interleaving — collect on `B` with pytest, run pytest on
//! `T`, then collect on `B` with vitest — would let code the candidate ran
//! under the first runner reach the second runner's collection of the floor,
//! which is exactly the attack rule 3 forbids. Every `B` collection precedes
//! every `T` execution, without exception."
//!
//! [`collect`] therefore drains *all* `B` work in one phase before it touches
//! `T`, and the shape of the loops is the enforcement. A per-runner loop that
//! did enumerate-then-run would be shorter, would pass every test about record
//! contents, and would be the one bug this whole section exists to prevent.

use crate::file::ResultFile;
use crate::header::{Profile, Provenance};
use crate::outcome::{BaseOutcome, Outcome};
use crate::record::{BaseRecord, ResultRecord, RunnerToken, Status};
use core::fmt;
use spine_canon::ObjectFormat;
use spine_manifest::{Isolation, Manifest};
use std::collections::BTreeMap;

/// Whether this is the `--ci` collector or the solo one.
///
/// RF §7.4 makes two header values a function of this and of nothing observed:
/// "The solo path (§5.4) runs the same code, and two header values are settled
/// before any observation is made: **`keys_visible=true`** … and
/// `profile=none`, because **outside `--ci` the collector attempts no boundary
/// at all**."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Ci,
    Solo,
}

/// RF §7.1 step 1's refusals, plus step 5's. "Class R": the job fails and **no
/// file is written**.
///
/// The distinction from a [`Status`] is the whole of RF §7.3's opening: "The
/// collector always writes a file once `T` is known and policy has been read."
/// Everything before that point refuses; everything after it is carried by the
/// `end` record. `ci.md` §5.2 reads the split off the filesystem — exit 2 is
/// "Refused. Nothing ran and **no result file exists**", exit 1 is a file that
/// exists beside a non-zero collector.
///
/// **DERIVED, in part.** The corpus fixes the *behaviour* of each of these
/// ("fail the job, write nothing") and a verdict line for the `uid` case, but
/// fixes no status token for any of them — none reaches a wire, a report or a
/// seal, because a refused run produces no report at all. Two of the five reuse
/// MF §3.11's spellings for the same condition, since a token already exists
/// there and inventing a second name for one fault helps nobody; the other
/// three are this implementation's, for stderr only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// RF §7.1 step 2, PB §7.4 rule 2: "Verify its own bytes against the pinned
    /// artifact list. Mismatch: fail the job, write nothing."
    ToolBytesMismatch,
    /// RF §7.1 step 3: "A language in `params.langs` the running release
    /// supports no adapter for: fail the job, write nothing — the same shape as
    /// failing its own hash check, and for the same reason, since a collector
    /// that cannot run a declared language cannot produce the floor a landing
    /// will be judged against."
    LanguageWithoutAdapter(String),
    /// RF §7.1 disposition 1: "`params.isolation: "uid"` … The collector
    /// **refuses**: it fails the job and writes nothing, at **step 1**."
    ///
    /// The refusal is *never* a downgrade, and RF §7.1 says why at length: "A
    /// build limitation is not a repository's isolation failure; no landing in
    /// that repository could ever clear it; and `none` would spend a
    /// permanently sealed field (PB §11) on a defect the repository can neither
    /// see nor fix, dressing a refusal as a green run that merely cannot
    /// auto-merge."
    ///
    /// Token borrowed from MF §3.11, where check 12b raises
    /// `isolation-unsupported` for the same manifest value.
    IsolationUnsupported,
    /// RF §7.1 *The deadline*: "It is present and not a positive integer: the
    /// collector fails the job and writes nothing (step 1's shape)."
    ///
    /// `0` is refused with the rest: RF §13 R24 reads it as "no deadline",
    /// which PB §6.7 forbids. Token borrowed from MF §3.11.
    TimeoutOutOfRange,
    /// RF §7.1 step 5: "A conflict yields no `T` and therefore no file; the
    /// collector fails the job and writes nothing. The trusted stage detects
    /// `needs-rebase` independently at step 1 of §5.4 and does not need a file
    /// to do it."
    MergeConflict,
}

impl Refusal {
    pub fn token(&self) -> &'static str {
        match self {
            Refusal::ToolBytesMismatch => "tool-bytes-mismatch",
            Refusal::LanguageWithoutAdapter(_) => "lang-without-adapter",
            Refusal::IsolationUnsupported => "isolation-unsupported",
            Refusal::TimeoutOutOfRange => "timeout-out-of-range",
            Refusal::MergeConflict => "merge-conflict",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::LanguageWithoutAdapter(lang) => {
                write!(f, "{}: {lang}", self.token())
            }
            _ => f.write_str(self.token()),
        }
    }
}

impl core::error::Error for Refusal {}

/// RF §7.1 step 1's six policy reads, "from `origin/<trunk>`, never from the
/// checkout".
///
/// PB §7.4 rule 1 is the rule this type exists to make structural: "the trusted
/// stage **and the collector** read `.spine/manifest.json` … from
/// `origin/<trunk>` … never from the checkout under test." There is no
/// constructor that takes a candidate's manifest, because a candidate's
/// manifest is not an input to any of these values — RF §9 puts every one of
/// them in the *cannot influence* column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    pub cli_version: String,
    pub dist_hash: String,
    pub isolation: Isolation,
    pub langs: Vec<String>,
    /// Seconds. RF §7.1: "a **strictly positive integer number of seconds** …
    /// defaulting to `1800` when the key is absent".
    pub timeout_secs: u64,
    pub object_format: ObjectFormat,
}

/// RF §7.1's default deadline: "`params.timeout` absent means `1800`."
pub const DEFAULT_TIMEOUT_SECS: u64 = 1800;

impl Policy {
    /// Read step 1 out of **trunk's** manifest.
    ///
    /// The two refusals that live here are step 1's, and both are checked
    /// before anything is spawned and before `T` exists.
    pub fn read(trunk: &Manifest, mode: Mode) -> Result<Self, Refusal> {
        let isolation = trunk.isolation();
        // RF §7.1 disposition 1, and RF §7.4's exemption from it: outside
        // `--ci` the solo collector "attempts nothing, it **refuses nothing** —
        // a manifest declaring `uid` costs a solo developer no run, and
        // disposition 1 of §7.1 is a `--ci` rule — and it writes `none`."
        if isolation == Isolation::Uid && mode == Mode::Ci {
            return Err(Refusal::IsolationUnsupported);
        }
        Ok(Policy {
            cli_version: trunk.cli_version().to_owned(),
            dist_hash: trunk.cli_dist_hash().to_owned(),
            isolation,
            langs: trunk.langs().iter().map(|s| (*s).to_string()).collect(),
            timeout_secs: timeout(trunk)?,
            object_format: trunk.object_format(),
        })
    }

    /// The `tool=` token this collector would write **if** its own version and
    /// artifact-list hash were trunk's.
    ///
    /// It exists for the trusted stage's §8.3 step 2, which "constructs the
    /// expected token … from trunk's manifest and compares it to the header's
    /// `tool=` **as bytes over the whole token**". A collector must never call
    /// it for its *own* header: RF §4.2, "The collector writes what it **is**,
    /// never what trunk pins. Copying the manifest's value would assert
    /// nothing."
    ///
    /// **The corpus's recipe for this token is wrong by one prefix, and the
    /// vector is right.** RF §8.3 step 2 spells the construction "`<cli.version>`
    /// `+sha256:` `<cli.dist_hash>`", but MF §3 stores `cli.dist_hash` in the
    /// `sha256:<hex>` form PB §11's hash policy fixes for a non-git artifact —
    /// MF §8.3's published manifest carries
    /// `"dist_hash":"sha256:6f49644f…744db"` — so the recipe read literally
    /// yields `1.4.0+sha256:sha256:6f49…`, which is not RF §4.2's grammar and
    /// is not RF §10's `tool=1.4.0+sha256:6f49644f…744db`. The vector wins: the
    /// separator is written once, and a stored prefix is not repeated.
    pub fn expected_tool_token(&self) -> String {
        let hex = self
            .dist_hash
            .strip_prefix("sha256:")
            .unwrap_or(&self.dist_hash);
        format!("{}+sha256:{}", self.cli_version, hex)
    }
}

/// RF §7.1 *The deadline*, and the one place `Manifest::timeout`'s convenience
/// default is not enough.
///
/// `Manifest::timeout()` folds "absent" and "present but not an integer" into
/// the same `1800`, which is right for MF's readers and wrong here: RF §7.1
/// makes the first the default and the second a refusal. So the raw member is
/// read rather than the accessor.
fn timeout(trunk: &Manifest) -> Result<u64, Refusal> {
    let Some(member) = trunk.value().get("params").and_then(|p| p.get("timeout")) else {
        return Ok(DEFAULT_TIMEOUT_SECS);
    };
    match member.as_u64() {
        Some(n) => deadline_from_secs(n),
        // Present and not an integer at all.
        None => Err(Refusal::TimeoutOutOfRange),
    }
}

/// The deadline bound, on its own so something can call it.
///
/// "strictly positive": RF §13 R24 refuses `0` because it would spell "no
/// deadline", and PB §6.7 admits no such value.
///
/// This is `pub` for a reason worth stating. `Manifest::parse` refuses every
/// out-of-range spelling first, so this branch is unreachable through the
/// dependency's public constructor — and until 2026-08-28 the test that claimed
/// to cover it compared a `&'static str` to itself instead. A check nothing can
/// call is a check nothing verifies, and RF §7.1 makes "a collector enforcing
/// no deadline" non-conformant, so the check stays and is now reachable.
pub fn deadline_from_secs(secs: u64) -> Result<u64, Refusal> {
    // MF §3.3's domain, which this must not contradict: `1 <= t <= 86400`.
    if (1..=86_400).contains(&secs) {
        Ok(secs)
    } else {
        Err(Refusal::TimeoutOutOfRange)
    }
}

/// What the pinned release knows about languages — `import-resolver.md`'s, not
/// this crate's.
///
/// RF §6.2: the invocation set is "a **total function of trunk's `params.langs`
/// and the pinned release**, and of nothing else". RF §6.4 forbids inferring
/// the mapping from this document's examples: "This document fixes no
/// per-language id grammar and no `runner` token, and an implementer must not
/// infer either from the examples here."
///
/// **Unimplemented here on purpose.** The adapter set is another crate's.
pub trait Release {
    /// The adapters this release assigns to a language, or `None` where it
    /// supports the language with no adapter at all — RF §7.1 step 3's refusal.
    fn adapters_for(&self, lang: &str) -> Option<Vec<RunnerToken>>;
}

/// RF §7.1 step 3: "Compute the invocation set from `params.langs` (§6.2)."
///
/// The result carries every adapter of every declared language, in the order
/// the languages are declared, deduplicated. RF §6.2: "The collector invokes
/// **every** member of that set, in full, with no selection argument. A
/// collector that skips one narrows the floor exactly as a `-k` would, and is
/// non-conformant for the same reason."
///
/// Order here is *not* a wire fact: RF §4.5's sort on `runner` bytes removes
/// invocation order from the file, so this vector's order is free to be the
/// manifest's and can never reach a byte of output.
pub fn invocation_set(policy: &Policy, release: &dyn Release) -> Result<Vec<RunnerToken>, Refusal> {
    let mut set: Vec<RunnerToken> = Vec::new();
    for lang in &policy.langs {
        let adapters = release
            .adapters_for(lang)
            .ok_or_else(|| Refusal::LanguageWithoutAdapter(lang.clone()))?;
        for token in adapters {
            if !set.contains(&token) {
                set.push(token);
            }
        }
    }
    Ok(set)
}

/// Which checkout a phase runs against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checkout {
    /// RF §7.1 step 7: trunk's own tree.
    Base,
    /// RF §7.1 step 8: "Check out `T`, detached." Never `H` — PB §7.4 rule 3.
    Candidate,
}

/// The isolation boundary of RF §7.1 step 6, and the host-side phases the
/// collector drives through it.
///
/// **`spine-isolate` owns every implementation of this.** RF §7.1's *The
/// isolation boundary* is that crate's specification in full: M1's five
/// namespaces, the two-lower-layer overlay, the identity source, the two
/// network dispositions, the restore phase, and the probe's four tests P1-P4.
/// None of it is here, and the one thing this crate needs from it is a
/// [`Profile`] that a test licensed.
///
/// The trait exposes no way to *request* a profile, and that is deliberate:
/// RF §7.1 says "`profile=` is a **finding**: the collector writes it only
/// where a test it performed, and could have failed, licensed it", and "The
/// collector never upgrades, and never substitutes." An implementation that
/// returns [`Profile::Container`] without having passed P1-P4 is non-conformant
/// (RF §11 item 16), and no signature here can prevent that — which is why the
/// tests live behind the boundary rather than in front of it.
pub trait Host {
    /// The boundary step 6 achieved, not the one step 1 requested.
    ///
    /// RF §7.1's disposition 2 collapses every host-prerequisite absence, every
    /// creation failure and every failed probe test into `none`: "There is no
    /// third outcome and no partial one: three tests out of four is `none`."
    fn profile(&self) -> Profile;

    /// RF §7.1 steps 7 and 8.
    ///
    /// **DERIVED: the corpus is silent on a failed checkout.** RF §7.3's status
    /// table has no row for it and RF §7.1's refusals are all at steps 1-5, so
    /// this signature cannot report one. See the crate's report.
    fn checkout(&mut self, which: Checkout);

    /// RF §7.1 *The restore phase*: "After each checkout and **before the first
    /// runner invocation against it**, the collector runs one restore phase for
    /// that checkout: at step 7 for `B`, at step 8 for `T`. **Two per run,
    /// never one per runner**, whatever the invocation set holds."
    ///
    /// It returns nothing because there is nothing to return: "**It is not a
    /// runner.** It is in no invocation set (§6.2), contributes no `base`
    /// record, no `result` record, no id and no `status` contribution (§7.3),
    /// and **nothing reads its exit code**." A `Result` here would be a value
    /// some future caller folds into `end.status`, which RF §11 item 16a makes
    /// non-conformance.
    fn restore(&mut self, which: Checkout, timeout_secs: u64);

    /// RF §7.1 step 9: "Reap every process group." RF §9 turns it into an
    /// ordering guarantee — records are "All serialized by the collector after
    /// reaping every process group."
    fn reap_all(&mut self);

    /// RF §7.1 step 8: "spawn it as a child **under the runner disposition of
    /// the boundary** and read its stream over the pipe, enforcing the deadline
    /// below."
    ///
    /// **An adapter cannot spawn for itself**, which is why this is on `Host`
    /// and not on `RunnerAdapter`. The boundary is the host's — the mount, PID,
    /// IPC, network and user namespaces, the mapped identity, the masked result
    /// directory — and a runner started outside it is a runner RF §7.1's whole
    /// step 6 did not contain. The adapter chooses the argv (`spine-resolve`'s
    /// ratified table) and reads the bytes; the host decides what the process
    /// runs inside.
    ///
    /// **The stream is the transport's, not the runner's stdout.** RF §6.6:
    /// "it is read over a pipe the collector holds, it is not supplied by the
    /// candidate's environment". `env` is what the adapter needs on the child
    /// to reach that pipe — a plugin module name and the descriptor to write
    /// to — and it may not carry key material: `keys::Probe`'s step-4 reading
    /// is over the collector's own environment, and is honest about "every
    /// runner invocation" only because no invocation adds any.
    fn spawn(&mut self, spec: &Spawn<'_>) -> Spawned;
}

/// One id in a runner's enumeration of `B`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseId {
    pub id: String,
    /// RF §6.3 obligation 3: "a **total `id → path`** function producing the
    /// repo-relative, `/`-separated path the id was collected from, mapped onto
    /// a tree entry and emitted as the tree's bytes (§4.4). The empty string
    /// where no tree entry matches."
    pub path: String,
}

/// The result of a runner's `B` **enumeration** — the invocation that decides
/// the floor's membership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseEnumeration {
    Collected(Vec<BaseId>),
    /// RF §7.3: "The **enumeration** of the id set on the checkout of `B`
    /// failed, or its deadline expired during that enumeration."
    Failed,
}

/// The result of a runner's `B` **outcome** run — RF §6.3 obligation 6.
///
/// For `vitest` and `dart-test` the enumeration and the outcome run are one
/// invocation and this is what that invocation already reported; for `pytest`
/// and `swift-test` it is a second invocation against `B`
/// (`import-resolver.md` §11.1). Either way it is a `B` invocation and RF §7.1
/// puts all of them before every `T` execution.
///
/// There is no failure variant, and that is RF §7.3's asymmetry made
/// structural: "**A failed `B` outcome run is not all-or-nothing, and is not a
/// status at all.** … a failure of the second — it did not start, it died, it
/// was killed at the deadline, its stream was unparsable — leaves the `base`
/// section whole and gives every id it did not report a terminal outcome for
/// `out: "absent"`. No status contribution is made for it and `end.status` does
/// not move." A failed outcome run is therefore reported by *reporting less*,
/// and `absent` is what the collector fills in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaseOutcomeRun {
    /// Terminal outcomes the run reported, by runner-native id. An id the run
    /// never reached simply is not here.
    pub reported: Vec<(String, Outcome)>,
}

/// One item a runner reported on `T`, before reduction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultItem {
    pub id: String,
    /// RF §6.3 obligation 2's `id → fn`, computed by the adapter. RF §6.5:
    /// "**`fn` is computed by the collector, not by the trusted stage.**"
    pub function: String,
    pub path: String,
    pub out: Outcome,
}

/// A runner's `T` run: what it reported, and which row of RF §7.3 it landed on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRun {
    /// **In the runner's own report order.** Reduction is the collector's, not
    /// the adapter's: RF §7.2's "the last terminal outcome that runner reported
    /// for that id wins" is a rule about this sequence, and an adapter that
    /// pre-reduced would decide it privately.
    pub items: Vec<ResultItem>,
    /// RF §7.3's row, "evaluated **top to bottom, first match wins**".
    ///
    /// Never [`Status::BaseCollectFailed`]: that row is about the `B`
    /// enumeration, and [`collect`] assigns it from [`BaseEnumeration::Failed`]
    /// rather than reading it here.
    pub contribution: Status,
}

/// One adapter of the invocation set. `import-resolver.md` specifies each; RF
/// §6.3 lists what every one of them owes.
///
/// **Unimplemented here on purpose** — the per-language runners, their tokens,
/// their id grammars and their outcome mappings are another crate's, and RF
/// §6.4 says so in terms.
pub trait RunnerAdapter {
    /// RF §6.3 obligation 1: "A stable `runner` **token** … It is permanent:
    /// `Spine-Test` lines carrying it are sealed into landings forever."
    ///
    /// RF §4.4: "**The token is a constant of the collector's adapter, embedded
    /// in the pinned release.** It is never read from the runner's stream, from
    /// the repository's configuration, from `params.langs`, or from the
    /// environment."
    fn token(&self) -> &RunnerToken;

    /// A `B` invocation: enumerate the floor. RF §7.1 step 7.
    fn enumerate_base(&mut self, host: &mut dyn Host, timeout_secs: u64) -> BaseEnumeration;

    /// A `B` invocation: each enumerated id's own outcome on `B`. RF §4.4.
    fn base_outcomes(&mut self, host: &mut dyn Host, timeout_secs: u64) -> BaseOutcomeRun;

    /// The `T` run. RF §7.1 step 8.
    fn run_candidate(&mut self, host: &mut dyn Host, timeout_secs: u64) -> CandidateRun;
}

/// One runner invocation, as the adapter asks for it.
#[derive(Debug, Clone)]
pub struct Spawn<'a> {
    /// `import-resolver.md`'s ratified argv, from `spine-resolve`. IR §11.1:
    /// "**No adapter runs a command this section has not already ratified.**"
    pub argv: &'a [&'a str],
    /// Which checkout it runs against — the host is already standing on it
    /// (RF §7.1 steps 7 and 8), and this is carried so a host can refuse a
    /// mismatch rather than run the wrong tree.
    pub checkout: Checkout,
    /// Environment the child needs to reach the transport's pipe. Added, never
    /// removed: RF §7.1 makes the child's environment "the collector's own".
    pub env: &'a [(&'a str, String)],
    /// `params.timeout`, from trunk.
    pub timeout_secs: u64,
}

/// What a spawn produced. The three failures are RF §7.3's rows, named so an
/// adapter maps rather than invents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Spawned {
    /// The transport's bytes, and whether any member of the process group was
    /// terminated by a signal.
    ///
    /// RF §7.3 makes the second a conjunct of `complete`: "`complete` requires
    /// **both** that the adapter parsed that runner's terminal session-end
    /// event **and** that no member of its process group was terminated by a
    /// signal". The exit code is deliberately absent — "The runner's *exit
    /// code* is never the discriminator — a red suite exits non-zero on every
    /// runner that ships, so an exit-code test would make `complete`
    /// unreachable for exactly the runs G1 exists to judge."
    Stream { bytes: Vec<u8>, signalled: bool },
    /// "The runner could not be started at all."
    SpawnFailed,
    /// "The collector's deadline expired … and it killed that process group."
    TimedOut,
}

/// The values steps 2, 4 and 5 produced, which [`collect`] writes into the
/// header rather than computing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// Step 5: `T := git merge-tree --write-tree origin/<trunk> H`, computed by
    /// the untrusted job itself (PB §7.4 rule 3).
    pub tree: String,
    /// `origin/<trunk>` tip at the moment policy was read (RF §4.2).
    pub base: String,
    /// Step 2's own version and artifact-list hash — **the collector's**, never
    /// trunk's (RF §4.2).
    pub tool: String,
    /// Step 4's key-visibility probe. Ignored under [`Mode::Solo`], where
    /// RF §7.4 settles it as `true` before any observation is made.
    pub keys_visible: bool,
}

/// RF §7.1 steps 6-10, in order, over an established boundary.
///
/// Steps 1-5 are the caller's: they refuse rather than write, and RF §7.3's
/// "The collector always writes a file once `T` is known and policy has been
/// read" is the line this function starts after. Everything from here produces
/// a file.
pub fn collect(
    run: &Run,
    policy: &Policy,
    mode: Mode,
    host: &mut dyn Host,
    adapters: &mut [Box<dyn RunnerAdapter>],
) -> ResultFile {
    let timeout = policy.timeout_secs;

    // ---- Step 7: `B`. Every `B` invocation of every runner, and nothing else.
    //
    // RF §7.1: "Check out `B`, **run the restore phase for it**, and collect
    // the id set **and each id's outcome on `B`** … **before any process has
    // run against `T`'s content**."
    host.checkout(Checkout::Base);
    host.restore(Checkout::Base, timeout);

    let mut floors: Vec<BaseEnumeration> = Vec::with_capacity(adapters.len());
    for adapter in adapters.iter_mut() {
        floors.push(adapter.enumerate_base(host, timeout));
    }
    // A second pass, still on `B`, still before `T` exists as a checkout. Two
    // passes rather than one loop body because RF §7.1 counts the outcome run
    // as "a `B` invocation" and puts *all of them* ahead of `T`: fusing them
    // into the enumeration loop would still be correct, but fusing the `T` run
    // into either is the interleaving the spec names as the attack.
    let mut outcomes: Vec<BaseOutcomeRun> = Vec::with_capacity(adapters.len());
    for adapter in adapters.iter_mut() {
        outcomes.push(adapter.base_outcomes(host, timeout));
    }

    // ---- Step 8: `T`. No runner has been spawned against it until here.
    host.checkout(Checkout::Candidate);
    host.restore(Checkout::Candidate, timeout);

    let mut runs: Vec<CandidateRun> = Vec::with_capacity(adapters.len());
    for adapter in adapters.iter_mut() {
        runs.push(adapter.run_candidate(host, timeout));
    }

    // ---- Step 9.
    host.reap_all();

    // ---- Step 10: reduce, union, sort, fold, write.
    let mut base_records: Vec<BaseRecord> = Vec::new();
    let mut result_records: Vec<ResultRecord> = Vec::new();
    let mut contributions: Vec<Status> = Vec::with_capacity(adapters.len());

    for (index, adapter) in adapters.iter().enumerate() {
        let token = adapter.token().clone();

        // RF §7.3's rows are "evaluated **top to bottom, first match wins**",
        // and `base-collect-failed` is above every `T`-run row — so a runner
        // whose enumeration failed contributes that whatever its `T` run did.
        let contribution = match floors[index] {
            BaseEnumeration::Failed => Status::BaseCollectFailed,
            BaseEnumeration::Collected(_) => runs[index].contribution,
        };
        contributions.push(contribution);

        if let BaseEnumeration::Collected(ids) = &floors[index] {
            // RF §7.2: "`base` pairs are a set: duplicates at collection are
            // reduced to one record, per runner." Keyed by id inside this
            // runner, because the pair is the identity and the runner half is
            // constant here.
            let mut floor: BTreeMap<&str, &BaseId> = BTreeMap::new();
            for entry in ids {
                floor.entry(entry.id.as_str()).or_insert(entry);
            }
            let reported: BTreeMap<&str, Outcome> = outcomes[index]
                .reported
                .iter()
                .map(|(id, out)| (id.as_str(), *out))
                .collect();
            for (id, entry) in floor {
                base_records.push(BaseRecord {
                    runner: token.clone(),
                    id: entry.id.clone(),
                    // RF §4.4: `absent` "is the fail-closed value for every id
                    // the outcome run did not reach". Not `unknown`: that is a
                    // terminal report the adapter could not map, and this is no
                    // terminal report at all.
                    out: reported
                        .get(id)
                        .copied()
                        .map_or(BaseOutcome::Absent, BaseOutcome::Reported),
                    path: entry.path.clone(),
                });
            }
        }

        // RF §7.3's third column, enforced here rather than trusted from the
        // adapter: `spawn-failed`, `no-output` and `stream-invalid` contribute
        // "no `result` records", and for `stream-invalid` RF §7.2 spells out
        // that this covers records parsed either side of the bad one — "**That
        // runner** contributes **no** `result` records at all".
        if !contribution.keeps_result_records() {
            continue;
        }

        // RF §7.2: "One `result` record per distinct `(runner, id)` pair. When
        // a runner reports an id more than once — a rerun plugin, a repeated
        // phase — **the last terminal outcome that runner reported for that id
        // wins**. The collector transcribes; it does not adjudicate." Insertion
        // into a map keyed by id is exactly last-wins, and the map is per
        // runner because "Reduction never crosses runners: two runners
        // reporting one id string produce two records, and neither is the
        // other's rerun."
        let mut reduced: BTreeMap<&str, &ResultItem> = BTreeMap::new();
        for item in &runs[index].items {
            reduced.insert(item.id.as_str(), item);
        }
        for item in reduced.into_values() {
            result_records.push(ResultRecord {
                runner: token.clone(),
                id: item.id.clone(),
                function: item.function.clone(),
                out: item.out,
                path: item.path.clone(),
            });
        }
    }

    let status = fold(&contributions);

    // RF §7.3: "**Collection on `B` is all-or-nothing across runners.** If
    // *any* invoked runner's collection on `B` fails, the file's `status` is
    // `base-collect-failed`, `ids=0`, and **no `base` and no `result` records
    // are written at all**, from any runner. A partial `base` section is a
    // *shrunken floor*, which is the one truncation that weakens rather than
    // tightens the gate (§13, R13), and it would be indistinguishable from a
    // repository that genuinely has fewer landed tests."
    if status == Status::BaseCollectFailed {
        base_records.clear();
        result_records.clear();
    }

    ResultFile::new(
        Provenance {
            tree: run.tree.clone(),
            base: run.base.clone(),
            tool: run.tool.clone(),
            // RF §7.4: on the solo path `keys_visible` is `true` "out of §4.2's
            // own predicate, because the operator's own signing key is
            // reachable from the process tree that ran the tests", and
            // `profile` is `none` "because **outside `--ci` the collector
            // attempts no boundary at all**". Both are settled before any
            // observation is made, which is what makes PB §5.4's "preconditions
            // 1 and 2 fail by construction" a derivation rather than a claim.
            keys_visible: match mode {
                Mode::Ci => run.keys_visible,
                Mode::Solo => true,
            },
            profile: match mode {
                Mode::Ci => host.profile(),
                Mode::Solo => Profile::None,
            },
        },
        base_records,
        result_records,
        status,
    )
}

/// RF §7.3's fold, verbatim: "`end.status` is `complete` **iff every** invoked
/// runner contributed `complete`. Otherwise it is the **first row in this
/// table's order, after `complete`, contributed by any runner**."
///
/// [`Status`]'s `Ord` is the table's order, so this is a `min` over the
/// non-`complete` contributions. It reads no invocation order and no wall time,
/// which is what RF §4.5's determinism claim requires.
///
/// **An empty invocation set does not fold to `complete`.**
///
/// "iff every" is vacuously true over nothing, and taking that reading gave a
/// runner-less run a fully green, zero-evidence file: `status=complete`,
/// `ids=0`, no records, and — before the `floor_holds` fix beside it — a
/// satisfied floor. That is the vacuous quick-lane pass RF §7.3 spends a
/// paragraph closing, arrived at from the other end.
///
/// The argument for the vacuous reading was that MF §3.11's `langs-empty`
/// keeps an empty `params.langs` off trunk, so a conforming repository cannot
/// reach it. That is true and it is not enough: the check lives in another
/// crate, this function does not consult it, and "unreachable in a conforming
/// repository" is exactly the phrase that precedes a fail-open. Where the
/// corpus is silent the fail-closed reading was available, so it is taken:
/// a run that invoked no runner collected no evidence, and `base-collect-failed`
/// is the status for a run whose `B` collection produced nothing.
pub fn fold(contributions: &[Status]) -> Status {
    if contributions.is_empty() {
        return Status::BaseCollectFailed;
    }
    contributions
        .iter()
        .copied()
        .filter(|s| *s != Status::Complete)
        .min()
        .unwrap_or(Status::Complete)
}

/// RF §7.3: "The collector's exit status is non-zero for every `status` other
/// than `complete`, and for `keys_visible=true`, so the untrusted job fails as
/// §7.4 rule 0 requires. Writing the file anyway is deliberate: a failed job
/// that produced no file and a failed job that produced an honest one are
/// different things, and the trusted stage should be able to say which."
///
/// The `keys_visible` conjunct is why this is not `status.credits_outcomes()`:
/// a solo run can be green in every record and still exit non-zero, because its
/// operator's key was reachable from the process tree that ran the tests.
pub fn exit_is_zero(file: &ResultFile) -> bool {
    file.status.credits_outcomes() && !file.header.keys_visible
}

/// RF §6.5's `Spine-Test` split: "The payload is `<runner>` `U+0020`
/// `<function id>`: split at the **first** `U+0020`; the token before it is
/// `R`, and every byte after it is `F`, spaces included."
///
/// "The split is exact because §4.4's grammar excludes `U+0020` from a token,
/// and it is necessary because runners such as vitest produce function ids that
/// contain spaces." A payload with no space, or whose first token is out of
/// grammar, is "a malformed approval line — G13's finding, not G1's", so this
/// returns `None` rather than a [`Malformed`].
pub fn parse_spine_test_payload(payload: &str) -> Option<(RunnerToken, &str)> {
    let (token, function) = payload.split_once(' ')?;
    let runner = RunnerToken::new(token)?;
    if function.is_empty() {
        return None;
    }
    Some((runner, function))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::RunnerToken;

    fn token(s: &str) -> RunnerToken {
        RunnerToken::new(s).expect("token in grammar")
    }

    /// RF §7.3's fold over the table's fixed order.
    #[test]
    fn the_fold_is_complete_only_when_every_runner_completed() {
        assert_eq!(
            fold(&[Status::Complete, Status::Complete]),
            Status::Complete
        );
        assert_eq!(
            fold(&[Status::Complete, Status::RunnerTimeout]),
            Status::RunnerTimeout
        );
        // "the **first row in this table's order, after `complete`**" —
        // `spawn-failed` is above `runner-timeout`, whichever runner ran first.
        assert_eq!(
            fold(&[Status::RunnerTimeout, Status::SpawnFailed]),
            Status::SpawnFailed
        );
        assert_eq!(
            fold(&[Status::SpawnFailed, Status::RunnerTimeout]),
            Status::SpawnFailed
        );
        // `base-collect-failed` is first after `complete` and wins over
        // everything.
        assert_eq!(
            fold(&[
                Status::RunnerFailed,
                Status::BaseCollectFailed,
                Status::NoOutput
            ]),
            Status::BaseCollectFailed
        );
    }

    /// RF §7.3: "the fold is over the table's fixed order and not over
    /// invocation order or wall time, so it is deterministic and independent of
    /// which runner ran first — which §4.5's determinism claim requires."
    #[test]
    fn the_fold_does_not_read_invocation_order() {
        let forward = [Status::NoOutput, Status::StreamInvalid, Status::Complete];
        let mut reversed = forward;
        reversed.reverse();
        assert_eq!(fold(&forward), fold(&reversed));
    }

    /// RF §7.3: exit status is "non-zero for every `status` other than
    /// `complete`, **and** for `keys_visible=true`".
    #[test]
    fn a_green_solo_run_still_exits_non_zero_because_keys_were_visible() {
        let solo = Provenance {
            tree: "3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28".into(),
            base: "7b0d4a1f2c3e5d6a8b9c0d1e2f3a4b5c6d7e8f90".into(),
            tool: "1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db"
                .into(),
            keys_visible: true,
            profile: Profile::None,
        };
        let green = ResultFile::new(solo.clone(), Vec::new(), Vec::new(), Status::Complete);
        assert!(!exit_is_zero(&green));

        let ci = ResultFile::new(
            Provenance {
                keys_visible: false,
                profile: Profile::Container,
                ..solo
            },
            Vec::new(),
            Vec::new(),
            Status::Complete,
        );
        assert!(exit_is_zero(&ci));
    }

    /// RF §10: `Spine-Test: vitest tests/billing/invoice.test.ts > invoice
    /// totals > AC2 zero-rated lines` splits at its first space, "which is why
    /// the split is at the first one and not the last (§6.5)".
    #[test]
    fn a_spine_test_payload_splits_at_the_first_space_and_keeps_the_rest() {
        let (runner, function) = parse_spine_test_payload(
            "vitest tests/billing/invoice.test.ts > invoice totals > AC2 zero-rated lines",
        )
        .expect("conforming payload");
        assert_eq!(runner.as_str(), "vitest");
        assert_eq!(
            function,
            "tests/billing/invoice.test.ts > invoice totals > AC2 zero-rated lines"
        );

        assert!(parse_spine_test_payload("pytest").is_none());
        assert!(parse_spine_test_payload("Pytest a::b").is_none());
    }

    /// RF §6.2: the invocation set is a total function of `params.langs` and
    /// the release. A language the release has no adapter for is step 3's
    /// refusal, not a narrowed set.
    #[test]
    fn a_declared_language_with_no_adapter_refuses_the_job() {
        struct TwoLanguages;
        impl Release for TwoLanguages {
            fn adapters_for(&self, lang: &str) -> Option<Vec<RunnerToken>> {
                match lang {
                    "python" => Some(vec![token("pytest")]),
                    "ts" => Some(vec![token("vitest")]),
                    _ => None,
                }
            }
        }
        let mut policy = Policy {
            cli_version: "1.4.0".into(),
            dist_hash: "6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db".into(),
            isolation: Isolation::Container,
            langs: vec!["python".into(), "ts".into()],
            timeout_secs: 1800,
            object_format: ObjectFormat::Sha1,
        };
        assert_eq!(
            invocation_set(&policy, &TwoLanguages),
            Ok(vec![token("pytest"), token("vitest")])
        );

        policy.langs.push("dart".into());
        assert_eq!(
            invocation_set(&policy, &TwoLanguages),
            Err(Refusal::LanguageWithoutAdapter("dart".into()))
        );
    }

    /// RF §4.2: "`tool=` is spelled exactly as the seal's `tool=`", and §8.3
    /// step 2 constructs it from trunk's manifest to compare bytes.
    #[test]
    fn the_expected_tool_token_is_version_plus_sha256_dist_hash() {
        let policy = Policy {
            cli_version: "1.4.0".into(),
            dist_hash: "6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db".into(),
            isolation: Isolation::Container,
            langs: vec!["python".into()],
            timeout_secs: 1800,
            object_format: ObjectFormat::Sha1,
        };
        let expected =
            "1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db";
        assert_eq!(policy.expected_tool_token(), expected);

        // And from the form a manifest actually stores (MF §3: `sha256:<hex>`),
        // which is the one RF §8.3 step 2's recipe double-prefixes. RF §10's
        // `tool=` is the arbiter and it carries the separator once.
        let stored = Policy {
            dist_hash: format!("sha256:{}", policy.dist_hash),
            ..policy
        };
        assert_eq!(stored.expected_tool_token(), expected);
    }
}
