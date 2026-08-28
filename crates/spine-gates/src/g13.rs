//! **G13 — Authority · Signers.** `docs/spec/manifest.md` §4.8, whole.
//!
//! Thirteen ordered checks. "the order matters in one way only: checks 1 and 2
//! are a prefix that halts on an outright failure, because a keyring that does
//! not lint is not a set anything verifies against, and a line whose signature
//! did not verify is not a line whose fields may be read. From check 3 onward
//! every check runs and findings accumulate, for §6.1's reason — a reviewer
//! signing a protected review needs the whole list, not the first item."
//!
//! **G13's wires are `class=protected`, always** (GR §6.3) and they name a
//! commit, not a path (GR §6.1). **Exactly one check is coverable**: check 2
//! over a commit whose signed line claims none of the five roles a landing
//! rests on. MF §4.8.4 says why there is one and only one — PB §6.2's "a branch
//! stays append-only, and a bogus commit cannot brick it".

use crate::gate::Gate;
#[cfg(test)]
use crate::review::Binding;
use crate::review::{Mode, ReviewClass, Reviews, signerless_review_count_holds};
use crate::status::G13Status;
use crate::verdict::{Finding, Verdict, decide};
use crate::wire::{Wire, WireClass, WireKind};
use spine_manifest::Keyring;
use std::collections::BTreeSet;

/// MF §4.8.2's two clocks. "they are not two readings of one check but two
/// different governing keyrings."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Situation {
    /// `spine check`, `--sign`, `--approve`, `--review`, and `--land` before a
    /// seal exists. `K` is the keyring at trunk's **current** tip.
    InFlight,
    /// `spine index`'s first-parent walk, `--authority`, G9's ledger walk. `K`
    /// is the keyring at the **seal's `base=`**.
    Landed,
}

/// The trailer a signed line claims, read "from the line's own name, on the
/// commit, whatever the verification did with it" (MF §4.8.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trailer {
    Signoff,
    Reopen,
    /// `run=` present ⇒ the pipeline's approval; absent ⇒ a human's. MF §4.8.3
    /// makes the two namespaces exclusive, "which is what makes PB §6.3's 'a
    /// review or approval without `run=` signed by a `spine-seal@v1` key' a
    /// refusal rather than a preference".
    Approve {
        has_run: bool,
    },
    Review,
    Upgrade,
    Withdraw,
    Seal,
    /// "anything hand-made that merely looks like a trailer".
    Other(String),
}

impl Trailer {
    /// MF §4.8.4's split: "Five trailers name statements a landing rests on,
    /// and a failing one of those is **outright**."
    ///
    /// "**The split is deliberately not over `A`.** GR §5.5 records no
    /// unverified statement, so a line whose signature failed supplies no
    /// member of `A` by construction; splitting over `A` would route every
    /// *forged binding sign-off* to the coverable branch, where a protected
    /// review discharges it, and G13 would seal a landing over a signature
    /// nobody made."
    pub fn claims_role(&self) -> bool {
        matches!(
            self,
            Trailer::Signoff
                | Trailer::Approve { .. }
                | Trailer::Review
                | Trailer::Upgrade
                | Trailer::Withdraw
        )
    }

    /// MF §4.8.3's table: "for each trailer, the namespace its signature must
    /// verify under, and it is the whole of PB §6.2's *whose role disagrees
    /// with its namespace*."
    ///
    /// `Spine-Withdraw` has two admissible namespaces and "check 8 decides
    /// which, by key". `Spine-Seal` depends on the landing's mode: PB §7.5's
    /// recovery form verifies under `spine-review@v1`.
    pub fn required_namespaces(&self, recovery_seal: bool) -> &'static [&'static str] {
        match self {
            Trailer::Signoff | Trailer::Reopen | Trailer::Upgrade => &["spine-signoff@v1"],
            Trailer::Withdraw => &["spine-signoff@v1", "spine-review@v1"],
            Trailer::Approve { has_run: true } => &["spine-seal@v1"],
            Trailer::Approve { has_run: false } => &["spine-review@v1"],
            Trailer::Review => &["spine-review@v1"],
            Trailer::Seal if recovery_seal => &["spine-review@v1"],
            Trailer::Seal => &["spine-seal@v1"],
            // A line claiming no known role has no required namespace.
            //
            // The empty slice is the value, and it is NOT the whole rule:
            // `[].contains(_)` is false for every namespace, so a hand-made
            // `Spine-Foo` line whose signature **verifies** used to fall
            // through to `statement-namespace` and raise a class=protected
            // `G13:<oid>` wire — promoting the landing to protected-review on
            // a trailer MF §4.8.3's table has no row for. §4.8.4 describes the
            // coverable branch as being for a *failing* line, "noise a human
            // may accept". Check 2 now skips an unknown trailer that verified,
            // rather than relying on this slice to say so.
            Trailer::Other(_) => &[],
        }
    }
}

/// What OpenSSH said. "Whether a signature is well-formed [is] OpenSSH's
/// (§4.1). A malformed `-Sig` line simply fails verification and takes check
/// 2's status; G13 parses no SSHSIG" (MF §4.8.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verification {
    /// `ssh-keygen -Y verify` succeeded under a namespace the trailer admits.
    Ok {
        namespace: String,
        fingerprint: String,
    },
    /// "the bytes and the signature disagree."
    SignatureFailed,
    /// "the key holds a role the trailer does not admit."
    NamespaceWrong { namespace: String },
}

/// The parsed payload of the two lines checks 4, 5, 11 and 12 read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Payload {
    #[default]
    None,
    Approve(ApprovePayload),
    /// `voids=` naming a `freeze=`, or `voids=none`.
    Reopen {
        voids: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApprovePayload {
    /// `intent=` — the intent blob this approval binds.
    pub intent: String,
    /// `freeze=` — the SHA-256 over the sorted `Spine-Frozen`/`Spine-Test`
    /// lines.
    pub freeze: String,
    /// `red=k/n`'s `k`.
    pub red_k: u64,
    pub held: bool,
    pub reason: Option<String>,
    pub rounds: u64,
    pub total_rounds: u64,
}

/// One event commit of `E`.
///
/// `E := the branch's event commits, ancestor-first along
/// git rev-list --reverse --first-parent B..H, extended past Hc to H`
/// (MF §4.8.2). The order is the caller's to establish and this module relies
/// on it for checks 4, 5, 11 and 12.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventCommit {
    /// "lowercase hex at the length `object_format` implies, for which both
    /// `esc` and `tok` are the identity" (MF §4.8.1).
    pub oid: String,
    pub trailer: Trailer,
    /// "the principal is the line's own `signer=` or `reviewer=` value."
    pub principal: String,
    /// The trailer line's exact bytes, terminator excluded (PB §7.2) — check
    /// 3's datum.
    pub line: String,
    pub verification: Verification,
    pub payload: Payload,
}

/// A verified statement in GR §5.5's `authority` object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub fingerprint: String,
    pub namespace: String,
    /// "The trailer line exactly as it appears in the commit message,
    /// excluding the line terminator" (GR §5.5), which is `authority`'s third
    /// member and was missing here.
    ///
    /// It is also this statement's **identity in `E`**: check 3 refuses two
    /// byte-identical signed lines, so a line resolves to at most one commit,
    /// and resolving by `freeze=` — which two approvals may legitimately share
    /// — mis-picks whichever came first.
    pub line: String,
}

impl Statement {
    /// The statement's position in `E`, by the line it is.
    fn position_in(&self, events: &[EventCommit]) -> Option<usize> {
        events.iter().position(|c| c.line == self.line)
    }
}

/// GR §5.5's `authority` object — "the bound statements" (MF §4.8.2).
///
/// A void statement is **absent** from it: MF §4.8.2, "A signed line whose
/// principal holds no key in `K` is §4.8.2's **void** — a transition PB §6's
/// table consumes, never a finding."
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Authority {
    pub signoff: Option<Statement>,
    pub approve: Option<(Statement, ApprovePayload)>,
    pub reopens: Vec<Statement>,
    pub upgrade: Option<Statement>,
    pub withdraw: Option<Statement>,
}

impl Authority {
    /// GR §5.5's signer key: "`A.signoff.fingerprint` when present, else
    /// `A.upgrade.fingerprint` when present, else none" (MF §4.8.4 check 9).
    pub fn signer_key(&self) -> Option<&str> {
        self.signoff
            .as_ref()
            .or(self.upgrade.as_ref())
            .map(|s| s.fingerprint.as_str())
    }
}

/// MF §4.8.4.1's three limbs, as answers a caller supplies.
///
/// "The **delta** is over entries, not lines: two keyrings are compared by
/// their `(principal, fingerprint)` sets under §4.2's parse, so re-indenting
/// the file is not a delta at all, and editing a line in place is one removal
/// plus one addition."
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chain {
    /// `diff(B, Hc)` touches `.spine/allowed_signers`. Its truth *is* the
    /// trigger; the limbs below are evaluated only under it.
    pub keyring_touched: bool,
    /// Limb 1: every `class=protected` reviewer's fingerprint is a principal in
    /// **the parent's** keyring.
    pub every_reviewer_in_parent_keyring: bool,
    /// Limb 2: the delta only removes entries.
    pub delta_is_removal_only: bool,
    /// Limb 2: the one protected review is from a remaining key that is not a
    /// removed entry's key.
    pub remover_is_not_removed: bool,
    /// Limb 3, **landed only**: the seal is by a `spine-seal@v1` key in the
    /// parent's keyring. "The seal limb cannot run in flight: the seal signs
    /// `envelope=`, which covers `report=`, which is the digest of the report
    /// this gate's own verdict sits inside."
    pub seal_key_in_parent_keyring: bool,
}

/// The in-flight-only checks 11–13.
///
/// MF §4.8.4: "All three read event commits the landing does not copy … So
/// these three are refusals `spine check` and `spine check --approve` make in
/// flight; **they produce no wire in any landing report**, and an
/// implementation evaluating them at landing would be reading fields that are
/// not there."
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AtApprove {
    /// Whether this evaluation is `--approve`'s (checks 12 and 13).
    pub approving: bool,
    /// Check 13: "`reason=` is present when the closure tripwire fired." The
    /// tripwire is computed by `--approve` from the freeze closure (PB §4.3,
    /// IR §2.5); a landing does not recompute it (MF §4.8.4 "On check 6").
    pub closure_tripwire_fired: bool,
}

/// Everything G13 reads. "Nothing else. No wall clock, no environment, no prior
/// run, no side file (§7)."
#[derive(Debug, Clone)]
pub struct G13Input<'a> {
    /// `K` — at `B` in flight, at the seal's `base=` when landed.
    pub keyring: &'a Keyring,
    /// `mode` — §4.5's key count over `K`, **never `C-A1`** (MF §4.8.5).
    pub mode: Mode,
    pub events: &'a [EventCommit],
    pub authority: &'a Authority,
    pub situation: Situation,
    /// The intent blob under evaluation — check 4's `intent=` comparison.
    pub intent_blob: &'a str,
    pub chain: Chain,
    pub at_approve: AtApprove,
    /// Whether the landing's seal is the `mode=recovery` form (PB §7.5), which
    /// moves `Spine-Seal`'s required namespace.
    pub recovery_seal: bool,
}

fn wire(oid: &str) -> Wire {
    // MF §4.8.1: `path` carries the oid, "for which both `esc` and `tok` are the
    // identity — so the wire token is `G13:` + that oid, and it is the one
    // non-path value v1 puts in that member".
    Wire::at(
        Gate::G13,
        oid.as_bytes().to_vec(),
        WireClass::Protected,
        WireKind::Finding,
    )
}

/// MF §4.8.7, executed.
pub fn evaluate(input: &G13Input<'_>, reviews: &Reviews) -> Verdict<G13Status> {
    let mut findings: Vec<Finding<G13Status>> = Vec::new();

    // ---- 1 — the governing keyring; halts -----------------------------
    if !input.keyring.is_clean() {
        for finding in &input.keyring.findings {
            findings.push(Finding::outright(G13Status::Keyring(finding.lint)));
        }
        return decide(Gate::G13, findings, reviews);
    }

    // ---- 2 — every event commit's signature; halts --------------------
    let principals: BTreeSet<&str> = input
        .keyring
        .entries
        .iter()
        .map(|e| e.principal.as_str())
        .collect();
    let mut halted = false;
    for commit in input.events {
        // "A void statement is not read here at all." Deciding it "takes no
        // SSHSIG parse: the principal is the line's own `signer=` or
        // `reviewer=` value, and §4.4's `keyring-duplicate-principal` makes one
        // principal one key. Without this, rotating a signer's key mid-flight
        // would turn an append-only branch's own sign-off into an outright
        // refusal — the brick PB §6.2 rules out in terms."
        if !principals.contains(commit.principal.as_str()) {
            continue;
        }
        let status = match &commit.verification {
            Verification::Ok { namespace, .. } => {
                // An unknown trailer that VERIFIED is not this check's
                // business. MF §4.8.3's table gives it no required namespace,
                // and a gate cannot find a line off a table it is not on — so
                // `admitted.is_empty()` means "no rule to break" and not
                // "breaks every rule", which is what `[].contains(_)` said.
                let admitted = commit.trailer.required_namespaces(input.recovery_seal);
                if admitted.is_empty() || admitted.contains(&namespace.as_str()) {
                    continue;
                }
                G13Status::StatementNamespace
            }
            Verification::NamespaceWrong { .. } => G13Status::StatementNamespace,
            Verification::SignatureFailed => G13Status::StatementUnverified,
        };
        if commit.trailer.claims_role() {
            findings.push(Finding::outright_with_wire(status, wire(&commit.oid)));
            halted = true;
        } else {
            // The one coverable G13 finding (MF §4.8.4, GR §5.6.1, §6.3).
            findings.push(Finding::coverable(status, wire(&commit.oid)));
        }
    }
    if halted {
        return decide(Gate::G13, findings, reviews);
    }

    // ---- 3..10 — accumulate -------------------------------------------
    // 3 — "no two commits in `E` carry byte-identical signed lines." Outright,
    // "because two siblings rest a **total order** on it" (GR §5.5.1, DM §5.2).
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for commit in input.events {
        if !seen.insert(commit.line.as_str()) {
            findings.push(Finding::outright(G13Status::EventLineDuplicate));
        }
    }

    // 4 — the binding approval (PB §4.3, MF §4.8.4).
    if let Some((statement, approve)) = &input.authority.approve
        && !is_binding(statement, approve, input.events, input.intent_blob)
    {
        findings.push(Finding::outright(G13Status::ApprovalVoided));
    }

    // 5 — "every `Spine-Reopen` in `E` carries `voids=` naming the `freeze=` of
    // the approval binding immediately before it, and `voids=none` exactly when
    // no approval preceded it."
    let mut freeze_before: Option<&str> = None;
    for commit in input.events {
        match &commit.payload {
            Payload::Approve(a) => freeze_before = Some(a.freeze.as_str()),
            Payload::Reopen { voids } => {
                if voids.as_deref() != freeze_before {
                    findings.push(Finding::outright(G13Status::ReopenVoidsMismatch));
                }
                // A reopen voids the approval before it, so the next reopen
                // with none of its own reads `voids=none`.
                //
                // DERIVED, and the two clauses of check 5 pull apart here.
                // Two consecutive reopens with no approval between them: the
                // first clause asks for "the `freeze=` of the approval
                // **binding** immediately before it", and after the first
                // reopen no approval is binding — so the second reopen names
                // nothing, which is `voids=none`. The second clause reads
                // "`voids=none` exactly when no approval preceded it", and one
                // did precede it.
                //
                // The first clause governs, because the alternative is for the
                // second reopen to name a `freeze=` the first already voided —
                // a signed claim to void something already void, and no reading
                // of "binding" makes that true. The second clause is the base
                // case of the first, not an independent test.
                freeze_before = None;
            }
            Payload::None => {}
        }
    }

    // 6 — "`A.approve`'s `reason=` is present whenever its `red=` reads `0/n`
    // or it carries `held=false`." The third limb is check 13.
    if let Some((_, approve)) = &input.authority.approve
        && needs_reason(approve)
        && approve.reason.is_none()
    {
        findings.push(Finding::outright(G13Status::ApproveReasonMissing));
    }

    // 7 — team mode, no self-approved protected or break-glass review.
    if input.mode == Mode::Team {
        for review in reviews.all() {
            if matches!(
                review.class,
                ReviewClass::Protected | ReviewClass::BreakGlass
            ) && review.self_approved
            {
                findings.push(Finding::outright(G13Status::SelfApprovedProtected));
            }
        }
    }

    // 8 — `withdraw_key_ok` (MF §4.8.7).
    if let Some(withdraw) = &input.authority.withdraw
        && !withdraw_key_ok(withdraw, input.authority.signoff.as_ref())
    {
        findings.push(Finding::outright(G13Status::WithdrawKey));
    }

    // 9 — the signerless overlay.
    if input.authority.signer_key().is_none() && !signerless_review_count_holds(input.mode, reviews)
    {
        findings.push(Finding::outright(G13Status::SignerlessReviewCount));
    }

    // 10 — the chain rule (MF §4.8.4.1).
    if input.chain.keyring_touched {
        if !input.chain.every_reviewer_in_parent_keyring {
            findings.push(Finding::outright(G13Status::ChainReviewNotInParent));
        }
        // "a delta that only **removes** entries takes one protected review
        // from a remaining key that is **not** a removed entry's key" —
        // "a departed or compromised key is never asked to co-sign its own
        // revocation".
        if input.chain.delta_is_removal_only && !input.chain.remover_is_not_removed {
            findings.push(Finding::outright(G13Status::ChainRemoverRemoved));
        }
        if input.situation == Situation::Landed && !input.chain.seal_key_in_parent_keyring {
            findings.push(Finding::outright(G13Status::ChainSealNotInParent));
        }
    }

    // ---- 11..13 — in flight only ---------------------------------------
    if input.situation == Situation::InFlight {
        // 11 — `total_rounds=` equals its own `rounds=` plus the `rounds=` of
        // every earlier verifying `Spine-Approve` in `E`.
        if let Some((statement, approve)) = &input.authority.approve {
            // "the `rounds=` of every **earlier verifying** `Spine-Approve`
            // in `E`" (MF §4.8.4 check 11). Both adjectives are load-bearing
            // and the `freeze != freeze` filter enforced neither.
            //
            // **Earlier**: `E` is ancestor-first (GR §5.5.1), so "earlier" is
            // "before this one in `E`" — the old filter summed LATER approvals
            // too.
            //
            // **Verifying**: a void approval is one check 2 correctly skipped,
            // and counting its `rounds=` produced a spurious
            // `total-rounds-mismatch`. That is precisely the brick MF §4.8.2
            // says voiding exists to prevent: "rotating a signer's key
            // mid-flight would turn an append-only branch's own sign-off into
            // an outright refusal". A signer whose key rotated out of `K`
            // leaves exactly such an approval behind.
            //
            // **This one**: by the statement's own line, not by its `freeze=`.
            // Two approvals sharing a freeze are legitimate — re-approving an
            // unchanged tree produces exactly that — and the freeze match took
            // the first of them, summing the wrong prefix of `E`.
            let this_one = statement.position_in(input.events);
            let earlier: u64 = input
                .events
                .iter()
                .take(this_one.unwrap_or(input.events.len()))
                .filter(|c| matches!(c.verification, Verification::Ok { .. }))
                .filter(|c| principals.contains(c.principal.as_str()))
                .filter_map(|c| match &c.payload {
                    Payload::Approve(a) => Some(a.rounds),
                    _ => None,
                })
                .sum();
            if approve.total_rounds != approve.rounds + earlier {
                findings.push(Finding::outright(G13Status::TotalRoundsMismatch));
            }
        }
        if input.at_approve.approving {
            // 12 — "the branch carries no verifying `Spine-Approve` later than
            // the last `Spine-Reopen` with the same `intent=`, unless that
            // approval's fingerprint has since left `K`."
            if redundant_approval(input.events, input.intent_blob, &principals) {
                findings.push(Finding::outright(G13Status::ApprovalRedundant));
            }
            // 13 — the third `reason=` limb, "evaluated where the closure is in
            // hand".
            if input.at_approve.closure_tripwire_fired
                && input
                    .authority
                    .approve
                    .as_ref()
                    .is_none_or(|(_, a)| a.reason.is_none())
            {
                findings.push(Finding::outright(G13Status::ApproveReasonMissing));
            }
        }
    }

    decide(Gate::G13, findings, reviews)
}

/// MF §4.8.7's `binding`. PB §4.3: "the newest `Spine-Approve` on the branch
/// whose `freeze=` no `Spine-Reopen` names and whose `intent=` equals the
/// current signed blob."
fn is_binding(
    statement: &Statement,
    approve: &ApprovePayload,
    events: &[EventCommit],
    intent_blob: &str,
) -> bool {
    let newest_approve = events.iter().rposition(|c| {
        matches!(&c.payload, Payload::Approve(_))
            && matches!(c.verification, Verification::Ok { .. })
    });
    // By position, not by `freeze=`: two approvals may carry the same freeze —
    // a re-approval of an unchanged tree is the ordinary way — and matching on
    // it made an older approval read as the newest one.
    let is_newest =
        statement.position_in(events).is_some() && statement.position_in(events) == newest_approve;
    let last_reopen = events
        .iter()
        .rposition(|c| matches!(c.payload, Payload::Reopen { .. }));
    let later_than_reopen = match (newest_approve, last_reopen) {
        (Some(a), Some(r)) => a > r,
        (Some(_), None) => true,
        _ => false,
    };
    let voided = events.iter().any(|c| match &c.payload {
        Payload::Reopen { voids } => voids.as_deref() == Some(approve.freeze.as_str()),
        _ => false,
    });
    is_newest && later_than_reopen && approve.intent == intent_blob && !voided
}

/// MF §4.8.7: `needs_reason(a) := a.red has k = 0 ∨ a.held = false`.
fn needs_reason(approve: &ApprovePayload) -> bool {
    approve.red_k == 0 || !approve.held
}

/// MF §4.8.7's `withdraw_key_ok`.
///
/// The `s absent` limb "is the **orphaned tombstone** (GR §5.5, PB §11): the
/// sign-off key has left `K`, so the sign-off is omitted from `A`, the withdraw
/// line carries `orphaned=<principal>`, and there is no fingerprint for the
/// reviewer to differ from."
fn withdraw_key_ok(withdraw: &Statement, signoff: Option<&Statement>) -> bool {
    match withdraw.namespace.as_str() {
        "spine-signoff@v1" => signoff.is_some_and(|s| s.fingerprint == withdraw.fingerprint),
        "spine-review@v1" => signoff.is_none_or(|s| s.fingerprint != withdraw.fingerprint),
        _ => false,
    }
}

/// Check 12, PB §4.3 "verbatim in its exception too — *unless that approval's
/// key has since left the keyring*, the key-removed row of PB §6's table, which
/// is the one route from `tests-approved` back to a new freeze without a
/// reopen."
fn redundant_approval(
    events: &[EventCommit],
    intent_blob: &str,
    principals: &BTreeSet<&str>,
) -> bool {
    let after = events
        .iter()
        .rposition(|c| matches!(c.payload, Payload::Reopen { .. }))
        .map_or(0, |i| i + 1);
    events[after..].iter().any(|c| match &c.payload {
        Payload::Approve(a) => {
            matches!(c.verification, Verification::Ok { .. })
                && a.intent == intent_blob
                && principals.contains(c.principal.as_str())
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::Review;
    use crate::verdict::GateStatus;
    use spine_manifest::keyring::Lint;

    const OID: &str = "de841d39b7a84111dfbcc11ddc7a75aa9886b218";

    /// MF §8.7's shape: a team keyring with three principals across the three
    /// namespaces. Only the fields G13 reads are exercised here; the parse and
    /// the lint are `spine-manifest`'s, tested against MF §8.7 there.
    fn keyring(lines: &str) -> Keyring {
        Keyring::parse(lines.as_bytes())
    }

    fn team_keyring() -> Keyring {
        keyring(concat!(
            "alice@example.com namespaces=\"spine-signoff@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n",
            // MF §4.5: mode is the count of distinct `spine-signoff@v1`
            // fingerprints. Two of them is what makes this keyring `team`, and
            // checks 7 and 9 both read that and nothing else.
            "carol@example.com namespaces=\"spine-signoff@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD\n",
            "bob@example.com namespaces=\"spine-review@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB\n",
            "spine-pipeline namespaces=\"spine-seal@v1\" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC\n",
        ))
    }

    fn commit(trailer: Trailer, principal: &str, verification: Verification) -> EventCommit {
        EventCommit {
            oid: OID.into(),
            trailer,
            principal: principal.into(),
            line: format!("line-{principal}"),
            verification,
            payload: Payload::None,
        }
    }

    fn ok(ns: &str) -> Verification {
        Verification::Ok {
            namespace: ns.into(),
            fingerprint: format!("SHA256:{ns}"),
        }
    }

    fn input<'a>(
        keyring: &'a Keyring,
        events: &'a [EventCommit],
        authority: &'a Authority,
    ) -> G13Input<'a> {
        G13Input {
            keyring,
            mode: keyring.mode,
            events,
            authority,
            situation: Situation::InFlight,
            intent_blob: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            chain: Chain::default(),
            at_approve: AtApprove::default(),
            recovery_seal: false,
        }
    }

    /// MF §4.8.4 check 1: outright, and it **halts** — "a keyring that does not
    /// lint is not a set anything verifies against."
    #[test]
    fn a_keyring_that_does_not_lint_halts_before_any_signature_is_read() {
        let broken = Keyring::missing();
        let events = [commit(
            Trailer::Signoff,
            "alice",
            Verification::SignatureFailed,
        )];
        let authority = Authority::default();
        let verdict = evaluate(&input(&broken, &events, &authority), &Reviews::default());
        assert_eq!(verdict.status, GateStatus::Fail);
        assert_eq!(
            verdict.statuses(),
            [&G13Status::Keyring(Lint::KeyringMissing)]
        );
    }

    /// MF §4.8.4 check 2 and GR §6.3: "the only G13 finding that is not
    /// outright" — a line claiming none of the five roles.
    #[test]
    fn a_bogus_line_claiming_no_role_is_the_one_coverable_finding() {
        let k = team_keyring();
        let events = [commit(
            Trailer::Other("Spine-Nonsense".into()),
            "alice@example.com",
            Verification::SignatureFailed,
        )];
        let authority = Authority {
            signoff: Some(Statement {
                fingerprint: "SHA256:alice".into(),
                namespace: "spine-signoff@v1".into(),
                line: "line-alice".into(),
            }),
            ..Default::default()
        };
        let uncovered = evaluate(&input(&k, &events, &authority), &Reviews::default());
        assert_eq!(uncovered.status, GateStatus::Fail);
        assert_eq!(uncovered.wires.tokens(), [format!("G13:{OID}")]);

        // PB §6.2: "a branch stays append-only, and a bogus commit cannot brick
        // it." A protected review naming the oid discharges it.
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:bob", Binding::Current)
                .naming(vec![format!("G13:{OID}")]),
        ]);
        let covered = evaluate(&input(&k, &events, &authority), &reviews);
        assert_eq!(covered.status, GateStatus::Override);
    }

    /// Check 5's two clauses, where they pull apart: two consecutive reopens
    /// with no approval between them. The second names nothing, because after
    /// the first nothing is binding.
    #[test]
    fn a_second_consecutive_reopen_voids_none() {
        let k = team_keyring();
        let approve = ApprovePayload {
            intent: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            freeze: "f1".into(),
            red_k: 3,
            held: true,
            reason: None,
            rounds: 1,
            total_rounds: 1,
        };
        let mut a = commit(
            Trailer::Approve { has_run: false },
            "bob@example.com",
            ok("spine-review@v1"),
        );
        a.payload = Payload::Approve(approve);
        let reopen = |voids: Option<&str>| {
            let mut c = commit(Trailer::Reopen, "alice@example.com", ok("spine-signoff@v1"));
            c.payload = Payload::Reopen {
                voids: voids.map(str::to_string),
            };
            c
        };

        let good = [a.clone(), reopen(Some("f1")), reopen(None)];
        let authority = Authority::default();
        assert!(
            !evaluate(&input(&k, &good, &authority), &Reviews::default())
                .statuses()
                .contains(&&G13Status::ReopenVoidsMismatch)
        );

        // Naming the freeze the first reopen already voided is the reading
        // this rejects.
        let bad = [a, reopen(Some("f1")), reopen(Some("f1"))];
        assert!(
            evaluate(&input(&k, &bad, &authority), &Reviews::default())
                .statuses()
                .contains(&&G13Status::ReopenVoidsMismatch)
        );
    }

    /// Two approvals may carry the same `freeze=` — re-approving an unchanged
    /// tree is the ordinary way to produce one — and `freeze` equality then
    /// resolves the binding approval to whichever came first. Check 11 summed
    /// the wrong prefix of `E` for it, and check 4 called the older one the
    /// newest.
    #[test]
    fn two_approvals_sharing_a_freeze_resolve_by_position_not_by_freeze() {
        let k = team_keyring();
        let shared = |rounds: u64, total: u64| ApprovePayload {
            intent: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            freeze: "f1".into(),
            red_k: 3,
            held: true,
            reason: None,
            rounds,
            total_rounds: total,
        };
        let first = shared(2, 2);
        let second = shared(3, 5);

        let mut e1 = commit(
            Trailer::Approve { has_run: false },
            "bob@example.com",
            ok("spine-review@v1"),
        );
        e1.line = "line-approve-1".into();
        e1.payload = Payload::Approve(first);
        let mut e2 = commit(
            Trailer::Approve { has_run: false },
            "bob@example.com",
            ok("spine-review@v1"),
        );
        e2.line = "line-approve-2".into();
        e2.payload = Payload::Approve(second.clone());
        let events = [e1, e2];

        let authority = Authority {
            signoff: Some(Statement {
                fingerprint: "SHA256:alice".into(),
                namespace: "spine-signoff@v1".into(),
                line: "line-alice@example.com".into(),
            }),
            approve: Some((
                Statement {
                    fingerprint: "SHA256:bob".into(),
                    namespace: "spine-review@v1".into(),
                    line: "line-approve-2".into(),
                },
                second,
            )),
            ..Default::default()
        };
        let verdict = evaluate(&input(&k, &events, &authority), &Reviews::default());
        // `total_rounds=5` = its own 3 plus the earlier 2. Resolving by
        // `freeze` picked the *first* approval as "this one", summed nothing
        // before it, and raised `total-rounds-mismatch`.
        assert!(
            !verdict
                .statuses()
                .contains(&&G13Status::TotalRoundsMismatch),
            "statuses: {:?}",
            verdict.statuses()
        );
        // And the second is the binding one, so check 4 stays quiet.
        assert!(!verdict.statuses().contains(&&G13Status::ApprovalVoided));
    }

    /// MF §4.8.4: "**The split is deliberately not over `A`.** … splitting over
    /// `A` would route every *forged binding sign-off* to the coverable
    /// branch."
    #[test]
    fn a_forged_sign_off_is_outright_however_it_is_reviewed() {
        let k = team_keyring();
        let events = [commit(
            Trailer::Signoff,
            "alice@example.com",
            Verification::SignatureFailed,
        )];
        let authority = Authority::default();
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:bob", Binding::Current)
                .naming(vec![format!("G13:{OID}")]),
            Review::new(ReviewClass::Protected, "SHA256:c", Binding::Current)
                .naming(vec![format!("G13:{OID}")]),
        ]);
        let verdict = evaluate(&input(&k, &events, &authority), &reviews);
        assert_eq!(verdict.status, GateStatus::Fail);
        assert_eq!(verdict.statuses(), [&G13Status::StatementUnverified]);
    }

    /// MF §4.8.2: "**A void statement is not read here at all.** … Without
    /// this, rotating a signer's key mid-flight would turn an append-only
    /// branch's own sign-off into an outright refusal."
    #[test]
    fn a_statement_whose_principal_left_the_keyring_is_void_not_a_finding() {
        let k = team_keyring();
        let events = [commit(
            Trailer::Signoff,
            "departed@example.com",
            Verification::SignatureFailed,
        )];
        let authority = Authority::default();
        let mut i = input(&k, &events, &authority);
        // Signerless, and the overlay is check 9's, not check 2's — supply the
        // reviews it wants so this test isolates the void rule.
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a", Binding::Current),
            Review::new(ReviewClass::Protected, "SHA256:b", Binding::Current),
        ]);
        i.mode = Mode::Team;
        assert_eq!(evaluate(&i, &reviews).status, GateStatus::Pass);
    }

    /// MF §4.8.3: `Spine-Approve` **with** `run=` is `spine-seal@v1` and only
    /// that; **without** it, `spine-review@v1` and only that. PB §6.3 names the
    /// second as a refusal: "a review or approval without `run=` signed by a
    /// `spine-seal@v1` key".
    #[test]
    fn a_run_less_approval_signed_by_a_seal_key_is_a_namespace_refusal() {
        let k = team_keyring();
        let events = [commit(
            Trailer::Approve { has_run: false },
            "spine-pipeline",
            ok("spine-seal@v1"),
        )];
        let authority = Authority::default();
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a", Binding::Current),
            Review::new(ReviewClass::Protected, "SHA256:b", Binding::Current),
        ]);
        let verdict = evaluate(&input(&k, &events, &authority), &reviews);
        assert_eq!(verdict.statuses(), [&G13Status::StatementNamespace]);
        assert_eq!(verdict.status, GateStatus::Fail);
    }

    #[test]
    fn the_namespace_table_is_mf_4_8_3() {
        assert_eq!(
            Trailer::Signoff.required_namespaces(false),
            ["spine-signoff@v1"]
        );
        assert_eq!(
            Trailer::Reopen.required_namespaces(false),
            ["spine-signoff@v1"]
        );
        assert_eq!(
            Trailer::Upgrade.required_namespaces(false),
            ["spine-signoff@v1"]
        );
        assert_eq!(
            Trailer::Withdraw.required_namespaces(false),
            ["spine-signoff@v1", "spine-review@v1"]
        );
        assert_eq!(
            Trailer::Approve { has_run: true }.required_namespaces(false),
            ["spine-seal@v1"]
        );
        assert_eq!(
            Trailer::Approve { has_run: false }.required_namespaces(false),
            ["spine-review@v1"]
        );
        assert_eq!(
            Trailer::Review.required_namespaces(false),
            ["spine-review@v1"]
        );
        assert_eq!(Trailer::Seal.required_namespaces(false), ["spine-seal@v1"]);
        // PB §7.5's recovery form.
        assert_eq!(Trailer::Seal.required_namespaces(true), ["spine-review@v1"]);
    }

    /// MF §4.8.4 check 3, and why it is outright: "two siblings rest a **total
    /// order** on it."
    #[test]
    fn two_byte_identical_signed_lines_are_refused() {
        let k = team_keyring();
        let mut a = commit(Trailer::Review, "bob@example.com", ok("spine-review@v1"));
        a.line = "Spine-Review: class=tripwire ...".into();
        let b = a.clone();
        let events = [a, b];
        let authority = Authority {
            signoff: Some(Statement {
                fingerprint: "SHA256:alice".into(),
                namespace: "spine-signoff@v1".into(),
                line: "line-alice".into(),
            }),
            ..Default::default()
        };
        let verdict = evaluate(&input(&k, &events, &authority), &Reviews::default());
        assert_eq!(verdict.statuses(), [&G13Status::EventLineDuplicate]);
    }

    /// MF §4.8.4 check 6: "`reason=` is present whenever its `red=` reads `0/n`
    /// or it carries `held=false`."
    #[test]
    fn an_approval_with_red_zero_and_no_reason_is_refused() {
        let k = team_keyring();
        let approve = ApprovePayload {
            intent: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            freeze: "f1".into(),
            red_k: 0,
            held: true,
            reason: None,
            rounds: 1,
            total_rounds: 1,
        };
        let mut event = commit(
            Trailer::Approve { has_run: false },
            "bob@example.com",
            ok("spine-review@v1"),
        );
        event.payload = Payload::Approve(approve.clone());
        let events = [event];
        let authority = Authority {
            signoff: Some(Statement {
                fingerprint: "SHA256:alice".into(),
                namespace: "spine-signoff@v1".into(),
                line: "line-alice".into(),
            }),
            approve: Some((
                Statement {
                    fingerprint: "SHA256:bob".into(),
                    namespace: "spine-review@v1".into(),
                    // `commit()` builds `line-<principal>`; the statement is
                    // the same line, which is how it resolves in `E`.
                    line: "line-bob@example.com".into(),
                },
                approve,
            )),
            ..Default::default()
        };
        let verdict = evaluate(&input(&k, &events, &authority), &Reviews::default());
        assert!(
            verdict
                .statuses()
                .contains(&&G13Status::ApproveReasonMissing)
        );
    }

    /// MF §4.8.4 check 5: "`voids=none` exactly when no approval preceded it."
    #[test]
    fn a_reopen_must_void_the_approval_immediately_before_it() {
        let k = team_keyring();
        let mut approve = commit(
            Trailer::Approve { has_run: false },
            "bob@example.com",
            ok("spine-review@v1"),
        );
        approve.line = "approve-1".into();
        approve.payload = Payload::Approve(ApprovePayload {
            freeze: "f1".into(),
            ..Default::default()
        });
        let mut reopen = commit(Trailer::Reopen, "alice@example.com", ok("spine-signoff@v1"));
        reopen.line = "reopen-1".into();
        reopen.payload = Payload::Reopen {
            voids: Some("f2".into()),
        };
        let events = [approve, reopen];
        let authority = Authority {
            signoff: Some(Statement {
                fingerprint: "SHA256:alice".into(),
                namespace: "spine-signoff@v1".into(),
                line: "line-alice".into(),
            }),
            ..Default::default()
        };
        let verdict = evaluate(&input(&k, &events, &authority), &Reviews::default());
        assert!(
            verdict
                .statuses()
                .contains(&&G13Status::ReopenVoidsMismatch)
        );
    }

    /// MF §4.8.4 check 7 and PB §7.2's table: a team-mode protected review is
    /// "reviewer ≠ signer; refused otherwise".
    #[test]
    fn a_self_approved_protected_review_is_refused_in_team_mode() {
        let k = team_keyring();
        let events: [EventCommit; 0] = [];
        let authority = Authority {
            signoff: Some(Statement {
                fingerprint: "SHA256:alice".into(),
                namespace: "spine-signoff@v1".into(),
                line: "line-alice".into(),
            }),
            ..Default::default()
        };
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:alice", Binding::Current)
                .self_approved(true),
        ]);
        let verdict = evaluate(&input(&k, &events, &authority), &reviews);
        assert!(
            verdict
                .statuses()
                .contains(&&G13Status::SelfApprovedProtected)
        );
    }

    /// MF §4.8.7's `withdraw_key_ok`, both limbs, and the orphaned-tombstone
    /// case.
    #[test]
    fn the_withdraw_key_rule_admits_the_signoffs_key_or_a_different_reviewers() {
        let signoff = Statement {
            fingerprint: "SHA256:alice".into(),
            namespace: "spine-signoff@v1".into(),
            line: "line-alice".into(),
        };
        let by_signoff = Statement {
            fingerprint: "SHA256:alice".into(),
            namespace: "spine-signoff@v1".into(),
            line: "line-alice".into(),
        };
        assert!(withdraw_key_ok(&by_signoff, Some(&signoff)));
        let by_other_signoff = Statement {
            fingerprint: "SHA256:carol".into(),
            namespace: "spine-signoff@v1".into(),
            line: "line-carol".into(),
        };
        assert!(!withdraw_key_ok(&by_other_signoff, Some(&signoff)));
        let by_reviewer = Statement {
            fingerprint: "SHA256:bob".into(),
            namespace: "spine-review@v1".into(),
            line: "line-bob".into(),
        };
        assert!(withdraw_key_ok(&by_reviewer, Some(&signoff)));
        let self_review = Statement {
            fingerprint: "SHA256:alice".into(),
            namespace: "spine-review@v1".into(),
            line: "line-alice".into(),
        };
        assert!(!withdraw_key_ok(&self_review, Some(&signoff)));
        // The orphaned tombstone: no sign-off to differ from.
        assert!(withdraw_key_ok(&by_reviewer, None));
    }

    /// MF §4.8.4 check 9 and PB §11's overlay: "Every reseal, every quick-lane
    /// landing copying no `Spine-Upgrade`, and every orphaned tombstone is
    /// signerless."
    #[test]
    fn a_signerless_team_landing_needs_two_distinct_protected_reviewers() {
        let k = team_keyring();
        let events: [EventCommit; 0] = [];
        let authority = Authority::default();
        let one = Reviews::new(vec![Review::new(
            ReviewClass::Protected,
            "SHA256:a",
            Binding::Current,
        )]);
        let verdict = evaluate(&input(&k, &events, &authority), &one);
        assert!(
            verdict
                .statuses()
                .contains(&&G13Status::SignerlessReviewCount)
        );
        let two = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a", Binding::Current),
            Review::new(ReviewClass::Protected, "SHA256:b", Binding::Current),
        ]);
        assert_eq!(
            evaluate(&input(&k, &events, &authority), &two).status,
            GateStatus::Pass
        );
    }

    /// MF §4.8.4.1: "The seal limb cannot run in flight."
    #[test]
    fn the_chain_seal_limb_runs_only_when_landed() {
        let k = team_keyring();
        let events: [EventCommit; 0] = [];
        let authority = Authority {
            signoff: Some(Statement {
                fingerprint: "SHA256:alice".into(),
                namespace: "spine-signoff@v1".into(),
                line: "line-alice".into(),
            }),
            ..Default::default()
        };
        let chain = Chain {
            keyring_touched: true,
            every_reviewer_in_parent_keyring: true,
            delta_is_removal_only: false,
            remover_is_not_removed: false,
            seal_key_in_parent_keyring: false,
        };
        let mut i = input(&k, &events, &authority);
        i.chain = chain.clone();
        assert_eq!(evaluate(&i, &Reviews::default()).status, GateStatus::Pass);
        i.situation = Situation::Landed;
        assert!(
            evaluate(&i, &Reviews::default())
                .statuses()
                .contains(&&G13Status::ChainSealNotInParent)
        );
    }

    /// MF §4.8.4.1: "a departed or compromised key is never asked to co-sign
    /// its own revocation."
    #[test]
    fn a_removed_key_may_not_sign_its_own_removal() {
        let k = team_keyring();
        let events: [EventCommit; 0] = [];
        let authority = Authority {
            signoff: Some(Statement {
                fingerprint: "SHA256:alice".into(),
                namespace: "spine-signoff@v1".into(),
                line: "line-alice".into(),
            }),
            ..Default::default()
        };
        let mut i = input(&k, &events, &authority);
        i.chain = Chain {
            keyring_touched: true,
            every_reviewer_in_parent_keyring: true,
            delta_is_removal_only: true,
            remover_is_not_removed: false,
            seal_key_in_parent_keyring: true,
        };
        assert!(
            evaluate(&i, &Reviews::default())
                .statuses()
                .contains(&&G13Status::ChainRemoverRemoved)
        );
    }

    /// MF §4.8.4: checks 11–13 "produce no wire in any landing report, and an
    /// implementation evaluating them at landing would be reading fields that
    /// are not there."
    #[test]
    fn the_in_flight_only_checks_do_not_run_when_landed() {
        let k = team_keyring();
        let approve = ApprovePayload {
            intent: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            freeze: "f1".into(),
            red_k: 3,
            held: true,
            reason: None,
            rounds: 2,
            total_rounds: 99,
        };
        let mut event = commit(
            Trailer::Approve { has_run: false },
            "bob@example.com",
            ok("spine-review@v1"),
        );
        event.payload = Payload::Approve(approve.clone());
        let events = [event];
        let authority = Authority {
            signoff: Some(Statement {
                fingerprint: "SHA256:alice".into(),
                namespace: "spine-signoff@v1".into(),
                line: "line-alice".into(),
            }),
            approve: Some((
                Statement {
                    fingerprint: "SHA256:bob".into(),
                    namespace: "spine-review@v1".into(),
                    // `commit()` builds `line-<principal>`; the statement is
                    // the same line, which is how it resolves in `E`.
                    line: "line-bob@example.com".into(),
                },
                approve,
            )),
            ..Default::default()
        };
        let mut i = input(&k, &events, &authority);
        assert!(
            evaluate(&i, &Reviews::default())
                .statuses()
                .contains(&&G13Status::TotalRoundsMismatch)
        );
        i.situation = Situation::Landed;
        assert!(
            !evaluate(&i, &Reviews::default())
                .statuses()
                .contains(&&G13Status::TotalRoundsMismatch)
        );
    }

    /// MF §4.8.6: "**G13's outright findings stay outright on every landing
    /// shape, a reseal included**" — there is no `LandingShape` input at all,
    /// and this test is what says so.
    #[test]
    fn g13_has_no_reseal_suspension() {
        let k = team_keyring();
        let events = [commit(
            Trailer::Review,
            "bob@example.com",
            Verification::SignatureFailed,
        )];
        let authority = Authority::default();
        let reviews = Reviews::new(vec![
            Review::new(ReviewClass::Protected, "SHA256:a", Binding::Current)
                .naming(vec![format!("G13:{OID}")]),
            Review::new(ReviewClass::Protected, "SHA256:b", Binding::Current)
                .naming(vec![format!("G13:{OID}")]),
        ]);
        assert_eq!(
            evaluate(&input(&k, &events, &authority), &reviews).status,
            GateStatus::Fail
        );
    }
    /// An unknown trailer that **verified** is not check 2's business.
    ///
    /// `required_namespaces` returns an empty slice for `Trailer::Other`, and
    /// `[].contains(_)` is false for every namespace — so a hand-made
    /// `Spine-Foo` line whose signature verifies produced `statement-namespace`
    /// and a `class=protected` `G13:<oid>` wire, promoting the landing to
    /// protected-review over a trailer MF §4.8.3's table has no row for. §4.8.4
    /// describes the coverable branch as being for a *failing* line, "noise a
    /// human may accept".
    ///
    /// The sibling test above covers the failing line; the `Ok` path was
    /// untested, which is how the empty slice came to mean "breaks every rule"
    /// instead of "no rule to break".
    #[test]
    fn an_unknown_trailer_that_verifies_raises_nothing() {
        let k = team_keyring();
        let authority = Authority {
            signoff: Some(Statement {
                fingerprint: "SHA256:alice".into(),
                namespace: "spine-signoff@v1".into(),
                line: "line-alice".into(),
            }),
            ..Default::default()
        };

        let verifying = [commit(
            Trailer::Other("Spine-Foo".into()),
            "alice@example.com",
            Verification::Ok {
                namespace: "spine-review@v1".into(),
                fingerprint: "SHA256:alice".into(),
            },
        )];
        let verdict = evaluate(&input(&k, &verifying, &authority), &Reviews::default());
        assert_eq!(
            verdict.status,
            GateStatus::Pass,
            "an unknown trailer that verified is off the table, not against it: {:?}",
            verdict.statuses()
        );
        assert!(verdict.wires.is_empty(), "and it promotes no landing");
    }
}
