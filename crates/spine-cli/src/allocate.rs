//! PB §5.4's intent-id allocation, and the round-trip it refuses without.
//!
//! > "Intent ids: `spine new` takes max+1 over live `refs/heads/intent/*`,
//! > `refs/remotes/*/intent/*` and every `Spine-Intent` id sealed on trunk —
//! > landings and tombstones — refuses an id already in the ledger, and
//! > renumbers if its push loses. Because landing deletes the ref, a stale
//! > clone can recreate `intent/INT-042` after it landed: `spine new` fetches
//! > `refs/heads/<trunk>` and `refs/heads/intent/*` immediately before
//! > allocating and refuses to allocate without that round-trip."
//!
//! The three sources are three different kinds of evidence and all three are
//! needed: a live local branch, a live branch someone else pushed, and an id
//! that no longer has a branch at all because landing deleted it. Dropping the
//! third is the failure the paragraph is written about.

use crate::argv::Variant;

/// The git reads allocation performs, as a trait so the rule is testable
/// without a repository — every refusal below is reachable from a handful of
/// strings, and a rule this consequential should not have a test suite bounded
/// by how many repositories are convenient to build.
pub trait Refs {
    /// `git for-each-ref --format=%(refname) refs/heads/intent/ refs/remotes/`
    /// — every ref name, unfiltered. The caller filters, because the two
    /// patterns PB §5.4 names are this module's rule and not git's.
    fn ref_names(&self) -> Vec<String>;

    /// Every first-parent commit message on trunk, tip-first.
    fn trunk_messages(&self) -> Vec<String>;

    /// `git fetch` of trunk and the intent refs. `false` where it could not be
    /// performed — the network is down, the remote is gone, the credentials
    /// are wrong.
    fn fetch(&self) -> bool;

    /// Whether the repository has a remote to fetch from at all.
    fn has_remote(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocateError {
    /// "refuses to allocate without that round-trip".
    FetchFailed,
    /// PB §5.4 bounds nothing here, and neither does ID §2. This is the
    /// implementation's own limit, at the point where the *next* id would not
    /// fit in a `u64` — reported rather than wrapped.
    Exhausted,
}

impl core::fmt::Display for AllocateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AllocateError::FetchFailed => f.write_str(
                "could not fetch trunk and refs/heads/intent/*, and allocating without that \
                 round-trip can recreate an id that already landed (PB §5.4)",
            ),
            AllocateError::Exhausted => f.write_str("the id space is exhausted"),
        }
    }
}

impl core::error::Error for AllocateError {}

/// Every id the ledger holds, from all three sources.
///
/// **One number space across both prefixes.** DERIVED: PB §5.4 says "max+1
/// over" a set of *ids*, and taking a maximum requires reading the number out
/// of each; nothing says the two prefixes count separately. Sharing the space
/// is also the only reading that cannot produce `INT-7` and `BUG-7` in one
/// repository, which would be two documents at two paths whose node ids differ
/// by three bytes — and `dump.md` §5.2 builds every node id from the id.
/// Per-prefix numbering buys nothing and costs that.
pub fn ledger(refs: &dyn Refs) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();

    // Sources 1 and 2: live branches, local and remote-tracking. PB §5.4 names
    // `refs/heads/intent/*` and `refs/remotes/*/intent/*`, so a ref under
    // `refs/remotes/origin/main` is not one and neither is `refs/tags/intent/x`.
    for name in refs.ref_names() {
        if let Some(id) = intent_ref_id(&name) {
            ids.push(id.to_string());
        }
    }

    // Source 3: "every `Spine-Intent` id sealed on trunk — landings **and
    // tombstones**". Landing deletes the branch, so this is the only source
    // that remembers an id whose work is done, and it is the one a stale clone
    // does not have.
    for message in refs.trunk_messages() {
        if let Some(id) = spine_intent_payload(&message) {
            ids.push(id);
        }
    }

    ids.sort_unstable();
    ids.dedup();
    ids
}

/// The id of `refs/heads/intent/<ID>` or `refs/remotes/<remote>/intent/<ID>`.
fn intent_ref_id(refname: &str) -> Option<&str> {
    if let Some(id) = refname.strip_prefix("refs/heads/intent/") {
        return valid_id(id);
    }
    let rest = refname.strip_prefix("refs/remotes/")?;
    // `<remote>/intent/<ID>` — the remote name is one segment, and an id has
    // no `/`, so anything deeper is not an intent branch.
    let (_remote, rest) = rest.split_once('/')?;
    valid_id(rest.strip_prefix("intent/")?)
}

fn valid_id(id: &str) -> Option<&str> {
    let rest = id
        .strip_prefix("INT-")
        .or_else(|| id.strip_prefix("BUG-"))?;
    (!rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())).then_some(id)
}

/// The `Spine-Intent` payload of one commit message, if it carries the trailer.
///
/// Matched as a whole line with the exact `Spine-Intent: ` prefix (PB §7.2's
/// trailer form). A line mentioning the word elsewhere — in prose, in a fenced
/// intent document — is not a trailer, and reading one as a trailer would let
/// a document's own text move the allocator.
fn spine_intent_payload(message: &str) -> Option<String> {
    message
        .lines()
        .filter_map(|line| line.strip_prefix("Spine-Intent: "))
        .find_map(|payload| valid_id(payload.trim()).map(str::to_string))
}

/// The number in an id, for the maximum. `None` where it does not fit.
fn number(id: &str) -> Option<u64> {
    let rest = id
        .strip_prefix("INT-")
        .or_else(|| id.strip_prefix("BUG-"))?;
    rest.parse().ok()
}

/// PB §5.4's `max+1`, after the round-trip it requires.
pub fn allocate(refs: &dyn Refs, variant: Variant) -> Result<String, AllocateError> {
    // "fetches … **immediately before allocating** and refuses to allocate
    // without that round-trip." A repository with no remote has nothing to be
    // stale against, and refusing there would leave a solo developer — whom
    // PB §5.4 says "run the same protocol" — with no legal invocation.
    if refs.has_remote() && !refs.fetch() {
        return Err(AllocateError::FetchFailed);
    }

    let highest = ledger(refs).iter().filter_map(|id| number(id)).max();
    let next = match highest {
        None => 1,
        Some(n) => n.checked_add(1).ok_or(AllocateError::Exhausted)?,
    };
    Ok(format!("{}{}", prefix(variant), numeral(next)))
}

/// ID §2's **canonical spelling**, which is the only one `IntentId::parse`
/// admits: at least three digits, zero-padded below `100`, and never padded
/// beyond three.
///
/// `INT-1` is **not an id**. It parses nowhere, so an allocator that emitted
/// one produced a branch, a document path and a `Spine-Signoff` payload that
/// every reader downstream refuses — which is what running `--sign` on a fresh
/// intent found.
fn numeral(n: u64) -> String {
    format!("{n:03}")
}

/// TM §3.3: "`--bug` forces the prefix, and that is now checked as well as
/// required" — ID §... makes a Bug document carrying an `INT-` id
/// `variant-prefix-mismatch`, so the prefix is not cosmetic.
pub fn prefix(variant: Variant) -> &'static str {
    match variant {
        Variant::Bug => "BUG-",
        Variant::Intent | Variant::Change => "INT-",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Fake {
        refs: Vec<String>,
        messages: Vec<String>,
        remote: bool,
        fetch_ok: bool,
    }

    impl Refs for Fake {
        fn ref_names(&self) -> Vec<String> {
            self.refs.clone()
        }
        fn trunk_messages(&self) -> Vec<String> {
            self.messages.clone()
        }
        fn fetch(&self) -> bool {
            self.fetch_ok
        }
        fn has_remote(&self) -> bool {
            self.remote
        }
    }

    /// The three sources, and that the third is the one that matters: landing
    /// deletes the branch, so an id whose work is done survives only in trunk's
    /// messages.
    #[test]
    fn the_ledger_is_all_three_sources() {
        let refs = Fake {
            refs: vec![
                "refs/heads/intent/INT-3".into(),
                "refs/remotes/origin/intent/BUG-5".into(),
            ],
            messages: vec!["feat: something\n\nSpine-Intent: INT-9\nSpine-Event: land\n".into()],
            ..Default::default()
        };
        assert_eq!(ledger(&refs), ["BUG-5", "INT-3", "INT-9"]);
        // And the next id clears all three, not just the live ones.
        assert_eq!(allocate(&refs, Variant::Intent).unwrap(), "INT-010");
    }

    /// PB §5.4 names two ref patterns and no others.
    #[test]
    fn only_the_two_patterns_pb_5_4_names_are_read() {
        let refs = Fake {
            refs: vec![
                "refs/heads/main".into(),
                "refs/heads/intent/INT-2".into(),
                "refs/heads/quick/typo".into(),
                "refs/remotes/origin/main".into(),
                "refs/remotes/origin/intent/INT-4".into(),
                "refs/tags/intent/INT-99".into(),
                "refs/heads/intent/not-an-id".into(),
                "refs/remotes/origin/team/intent/INT-77".into(),
            ],
            ..Default::default()
        };
        assert_eq!(ledger(&refs), ["INT-2", "INT-4"]);
    }

    /// One number space across both prefixes: `BUG-7` and `INT-7` cannot both
    /// exist, because `dump.md` §5.2 builds every node id from the id.
    #[test]
    fn the_two_prefixes_share_one_number_space() {
        let refs = Fake {
            refs: vec!["refs/heads/intent/BUG-7".into()],
            ..Default::default()
        };
        assert_eq!(allocate(&refs, Variant::Intent).unwrap(), "INT-008");
        assert_eq!(allocate(&refs, Variant::Bug).unwrap(), "BUG-008");
    }

    /// TM §3.3: "`--bug` forces the prefix". `--change` does not — it is a
    /// Feature variant with its own template and an `INT-` id.
    #[test]
    fn the_prefix_follows_the_variant() {
        let empty = Fake::default();
        assert_eq!(allocate(&empty, Variant::Intent).unwrap(), "INT-001");
        assert_eq!(allocate(&empty, Variant::Change).unwrap(), "INT-001");
        assert_eq!(allocate(&empty, Variant::Bug).unwrap(), "BUG-001");
    }

    /// "refuses to allocate without that round-trip" — but only where there is
    /// a remote to round-trip with. PB §5.4: a solo developer "runs the same
    /// protocol", and refusing on a repository with no remote would leave them
    /// no legal invocation.
    #[test]
    fn a_failed_fetch_refuses_and_a_missing_remote_does_not() {
        let offline = Fake {
            remote: true,
            fetch_ok: false,
            ..Default::default()
        };
        assert_eq!(
            allocate(&offline, Variant::Intent),
            Err(AllocateError::FetchFailed)
        );

        let local_only = Fake {
            remote: false,
            fetch_ok: false,
            ..Default::default()
        };
        assert!(allocate(&local_only, Variant::Intent).is_ok());
    }

    /// **The canonical spelling is the only one that parses.** ID §2 admits
    /// `INT-042` and refuses `INT-42` (under-padded) and `INT-0042` (over-
    /// padded), so an allocator that emitted `INT-1` produced a branch, a
    /// document path and a `Spine-Signoff` payload every reader downstream
    /// refuses. Found by running `--sign` on a freshly allocated intent.
    #[test]
    fn every_allocated_id_is_in_the_canonical_spelling() {
        for (highest, expected) in ["INT-001", "INT-002", "INT-003"].into_iter().enumerate() {
            let refs = Fake {
                refs: (1..=highest)
                    .map(|n| format!("refs/heads/intent/INT-{n:03}"))
                    .collect(),
                ..Default::default()
            };
            let id = allocate(&refs, Variant::Intent).unwrap();
            assert_eq!(id, expected);
            assert!(
                spine_resolve::pragma::IntentId::parse(&id).is_some(),
                "{id} is not a canonical intent id"
            );
        }

        // Above 999 the padding stops rather than growing: `INT-0042` is
        // over-padded and refused, so `INT-1000` is the spelling.
        let refs = Fake {
            refs: vec!["refs/heads/intent/INT-999".into()],
            ..Default::default()
        };
        let id = allocate(&refs, Variant::Intent).unwrap();
        assert_eq!(id, "INT-1000");
        assert!(spine_resolve::pragma::IntentId::parse(&id).is_some());
    }

    /// A trailer is a whole line with the exact prefix. A document's own prose
    /// — which is sealed into the landing's message inside a fenced block —
    /// must not move the allocator.
    #[test]
    fn only_a_real_trailer_line_contributes_an_id() {
        let refs = Fake {
            messages: vec![
                concat!(
                    "feat: thing\n\n",
                    "```\n",
                    "# INT-500 — a document quoting an id\n",
                    "See Spine-Intent: INT-600 in the docs.\n",
                    "```\n\n",
                    "Spine-Intent: INT-7\n"
                )
                .into(),
            ],
            ..Default::default()
        };
        assert_eq!(ledger(&refs), ["INT-7"]);
    }

    /// A tombstone is in the ledger as much as a landing: "landings **and**
    /// tombstones". Both carry `Spine-Intent`, so nothing here has to tell
    /// them apart — which is the point, since an id whose intent was withdrawn
    /// must not be handed out again.
    #[test]
    fn a_withdrawn_id_is_not_handed_out_again() {
        let refs = Fake {
            messages: vec![
                "chore: withdraw\n\nSpine-Intent: INT-4\nSpine-Event: withdraw\n".into(),
            ],
            ..Default::default()
        };
        assert_eq!(allocate(&refs, Variant::Intent).unwrap(), "INT-005");
    }
}
