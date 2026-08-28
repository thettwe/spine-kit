//! The `wires` array, the wire token, and the containment check.
//!
//! **Two encodings of one path meet in this file, and they are not the same
//! bytes.** GR §6.1: a wire's `path` member is `esc(path)`; its *sort key* is
//! `tok(path)`. "The two differ on `,`, ` ` and `"` (§6.2), and sorting the
//! array on one key while the line is written under the other produces a
//! `wires=` whose order does not match the array's over the same findings."
//! Every [`Wire`] therefore holds the **raw path bytes** and derives both, so
//! neither can be reached for by mistake.
//!
//! **The order is not numeric.** PB §11's `Spine-Review` row fixes it —
//! "ascending by unsigned byte value over the whole token, so `G11` precedes
//! `G2`" — and GR §5.6 says why the two orders differ: `gates[]` is keyed by a
//! gate and nothing else, so a numeric key is total, while a wire token carries
//! an optional path and is therefore a *string*. GR §9.19 records this
//! document choosing the numeric order and being wrong, and GR §8.2.1 records
//! the trap that hid it: "Re-sorting `wires` … Both are permutations …
//! *every length check in this document passes under both orders and only the
//! digests separate them*."

use std::collections::{BTreeMap, BTreeSet};

use spine_canon::{esc, tok};

use crate::gate::Gate;
use crate::vocab::{WireClass, WireKind};

/// One entry of the `wires` array (GR §6.1).
///
/// `path` is "present iff the wire names a path or, for `G13`, a commit". It
/// holds **raw bytes**, not an encoding: a repository path is a byte string
/// that need not be UTF-8 (GR §2.3), and the two encodings this type derives
/// disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wire {
    pub gate: Gate,
    /// Raw path bytes, or for `G13` the commit oid's ASCII hex — "for which
    /// `esc` — and `tok` — is the identity".
    pub path: Option<Vec<u8>>,
    pub class: WireClass,
    pub kind: WireKind,
}

impl Wire {
    /// A pathless wire: `G3`, `G4`, `G11`, `G12`, G1's five findings that name
    /// no path, G2's diff-size sub-check, G16 where none is implicated.
    pub fn bare(gate: Gate, class: WireClass, kind: WireKind) -> Self {
        Wire {
            gate,
            path: None,
            class,
            kind,
        }
    }

    /// A path-bearing wire. Takes bytes, because that is what a path is.
    pub fn at(gate: Gate, path: impl Into<Vec<u8>>, class: WireClass, kind: WireKind) -> Self {
        Wire {
            gate,
            path: Some(path.into()),
            class,
            kind,
        }
    }

    /// GR §6.2's **wire token**: `G<n>` when `path` is absent, `G<n>` + `:` +
    /// `tok(path)` otherwise.
    ///
    /// This is the sort key, the byte string a reviewer signs inside `wires=`,
    /// and the unit of GR §6.2's containment check. It is **not** what the
    /// `path` member serializes to — see [`Wire::path_member`].
    pub fn token(&self) -> String {
        match &self.path {
            None => self.gate.token().to_owned(),
            Some(p) => format!("{}:{}", self.gate.token(), tok(p)),
        }
    }

    /// GR §6.1's `path` member: `esc`-encoded, **not** `tok`-encoded.
    ///
    /// The difference is invisible until a repository holds a path with a
    /// comma, a space or a quote in it, at which point an implementation that
    /// wrote `tok` here produces a report no conforming implementation
    /// reproduces — and every published byte count still matches, because
    /// GR §8.2's vector path `src/shared/util.ts` contains none of the three.
    pub fn path_member(&self) -> Option<String> {
        self.path.as_deref().map(esc)
    }
}

/// Why a set of raised wires could not be collapsed into a `wires` array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireSetError {
    /// GR §6.1: "The collapse can never merge an advisory into a finding. The
    /// `kind` precedence exists for repeated findings of one gate over one
    /// path, not to reconcile two different claims: in v1 the only
    /// advisory-bearing gate is `G11`, and `G11` raises no findings, so every
    /// collapse is between two entries of the same kind. […] A later version
    /// that gives some gate both a finding and an advisory must say how they
    /// are told apart before it may rely on this rule; it is a
    /// `report_version` question, not an implementation's."
    ///
    /// So this is refused rather than resolved. Applying the `kind` precedence
    /// here is exactly the defect PB v0.18 was written to close (PB §12): the
    /// collapse promoted an advisory to `finding`, and a signature over
    /// `wires=G1` meant either "a human accepted that auto-merge is
    /// unavailable" or "a human accepted a failing test" with nothing in the
    /// record to say which.
    CrossKindCollapse {
        token: String,
        first: WireKind,
        second: WireKind,
    },
}

impl core::fmt::Display for WireSetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WireSetError::CrossKindCollapse {
                token,
                first,
                second,
            } => write!(
                f,
                "wire {token} raised as both {first} and {second}; \
                 v1 has no rule for telling them apart"
            ),
        }
    }
}

impl core::error::Error for WireSetError {}

/// The `wires` array: collapsed by GR §6.1's uniqueness rule, ordered by
/// GR §6.1's byte order.
///
/// Both invariants are established at construction and there is no way to
/// mutate one afterwards, because both are inside `report=` and both are
/// invisible to every published byte count.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WireSet(Vec<Wire>);

impl WireSet {
    /// Collapse and order the wires an evaluation raised.
    ///
    /// GR §6.1's collapse: "If one evaluation would produce the same key twice,
    /// the entries collapse and the surviving `class` is `protected` if either
    /// was … the surviving `kind` is the strongest of `finding` > `advisory` >
    /// `warn`." The `kind` half is *refused* rather than applied where the two
    /// differ — see [`WireSetError::CrossKindCollapse`].
    pub fn from_raised(raised: impl IntoIterator<Item = Wire>) -> Result<Self, WireSetError> {
        // Keyed by `(gate, path)`, which is also the insertion-independent
        // identity: a `BTreeMap` gives a deterministic collapse whatever order
        // the gates ran in, which matters because a gate's evaluation order is
        // not fixed by any document and the digest is.
        let mut collapsed: BTreeMap<(u8, Option<Vec<u8>>), Wire> = BTreeMap::new();
        for wire in raised {
            let key = (wire.gate.number(), wire.path.clone());
            match collapsed.get_mut(&key) {
                None => {
                    collapsed.insert(key, wire);
                }
                Some(existing) => {
                    if existing.kind != wire.kind {
                        return Err(WireSetError::CrossKindCollapse {
                            token: wire.token(),
                            first: existing.kind,
                            second: wire.kind,
                        });
                    }
                    existing.class = existing.class.dominant(wire.class);
                    existing.kind = existing.kind.strongest(wire.kind);
                }
            }
        }

        let mut wires: Vec<Wire> = collapsed.into_values().collect();
        sort_by_token(&mut wires);
        Ok(WireSet(wires))
    }

    /// The entries, in the array's own order.
    pub fn as_slice(&self) -> &[Wire] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// The wire tokens, in the array's order — which is the line's order,
    /// because "one key … governs both the array and the line, so the line is
    /// the array's tokens joined by `,` and nothing has to be re-sorted to
    /// write it" (GR §6.2).
    pub fn tokens(&self) -> Vec<String> {
        self.0.iter().map(Wire::token).collect()
    }

    /// The value of a `Spine-Review`'s `wires=` field for this set:
    /// comma-separated, in the array's order (GR §6.2, PB §11).
    ///
    /// A **numeric** sort — `G2:src/shared/util.ts,G11` — is non-conforming:
    /// "the sorted line is signed, so a numeric implementation produces
    /// byte-different `Spine-Review` lines and its containment check fails
    /// against a conforming implementation's report over identical facts."
    pub fn wires_line(&self) -> String {
        self.tokens().join(",")
    }

    /// GR §6.2's containment condition: "the report's wire set ⊆ the union of
    /// the `wires=` of the reviews that discharge the landing's review state".
    ///
    /// Set containment over wire tokens, byte-for-byte. "It includes `warn` and
    /// `advisory` wires — every entry of the array", so nothing is filtered
    /// here. And "a review's `wires=` may name tokens absent from the report",
    /// so this is containment and never equality.
    ///
    /// **Not for `class=break-glass`.** GR §6.2: "For `class=break-glass`,
    /// `wires=` lists *the gates bypassed* as bare ids, not the wire set, and
    /// is never used for containment."
    pub fn contained_in(&self, reviewed: &BTreeSet<String>) -> bool {
        self.0.iter().all(|w| reviewed.contains(&w.token()))
    }

    /// The tokens this set carries that no discharging review names — the
    /// containment check's counterexample, for a refusal message.
    pub fn uncovered(&self, reviewed: &BTreeSet<String>) -> Vec<String> {
        self.0
            .iter()
            .map(Wire::token)
            .filter(|t| !reviewed.contains(t))
            .collect()
    }

    /// GR §6.1's `spine stats` predicate, "deliberately **derived at read time
    /// and never stored**": a landing "whose only `class: protected` entry is a
    /// `G7` hard lease … holds iff some entry has `gate == "G7"` and
    /// `class == "protected"`, and no other entry has `class == "protected"`."
    ///
    /// It is here, and not a member, because "adding a boolean would be a
    /// second spelling of a fact the wire set already fixes, in a
    /// digest-bearing member, for a counter no gate reads."
    pub fn only_protected_wire_is_a_g7_lease(&self) -> bool {
        let mut protected = self.0.iter().filter(|w| w.class == WireClass::Protected);
        match protected.next() {
            Some(first) => first.gate == Gate::G7 && protected.next().is_none(),
            None => false,
        }
    }
}

/// GR §6.1's ordering: "Ascending by unsigned byte value over the whole **wire
/// token** of §6.2."
///
/// Comparing `token()`'s bytes rather than `(gate.number(), path)` is the whole
/// rule. Byte order alone produces all three of the corpus's stated
/// consequences with no special cases: `G11` precedes `G2` (`0x31` < `0x32` at
/// the second byte), `G1` precedes `G11`, and "within one gate the pathless
/// entry precedes every `:`-suffixed one because its token is a proper prefix
/// of theirs".
fn sort_by_token(wires: &mut [Wire]) {
    wires.sort_by_key(|w| w.token().into_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tripwire_finding(gate: Gate, path: &str) -> Wire {
        Wire::at(gate, path, WireClass::Tripwire, WireKind::Finding)
    }

    /// PB §11's `Spine-Review` row, quoted in GR §5.6, §6.1, §6.2 and §9.19:
    /// "ascending by unsigned byte value over the whole token, so `G11`
    /// precedes `G2`".
    #[test]
    fn a_numeric_wire_comparator_is_non_conforming() {
        let set = WireSet::from_raised([
            tripwire_finding(Gate::G2, "src/shared/util.ts"),
            Wire::bare(Gate::G11, WireClass::Tripwire, WireKind::Advisory),
        ])
        .unwrap();
        // PB §5.5's canonical envelope, EV vectors A and D, and GR §8.1 and
        // §8.2 all carry exactly this line.
        assert_eq!(set.wires_line(), "G11,G2:src/shared/util.ts");
        // The numeric order, which every published byte count also satisfies.
        assert_ne!(set.wires_line(), "G2:src/shared/util.ts,G11");
    }

    /// GR §6.1: "`G1` precedes `G11`, and within one gate the pathless entry
    /// precedes every `:`-suffixed one because its token is a proper prefix of
    /// theirs."
    #[test]
    fn byte_order_puts_g1_before_g11_and_bare_before_suffixed() {
        let set = WireSet::from_raised([
            tripwire_finding(Gate::G2, "a"),
            Wire::at(Gate::G1, "z", WireClass::Protected, WireKind::Finding),
            Wire::bare(Gate::G11, WireClass::Tripwire, WireKind::Advisory),
            Wire::bare(Gate::G1, WireClass::Protected, WireKind::Finding),
        ])
        .unwrap();
        assert_eq!(set.tokens(), vec!["G1", "G11", "G1:z", "G2:a"]);
    }

    /// R2's crossing point, and GR §6.1's own sentence: "The sort key is the
    /// token's bytes, which for a path-bearing entry means `tok(path)` and not
    /// `esc(path)`."
    ///
    /// The two paths below are ordered one way by `esc` and the other by `tok`:
    /// `esc` leaves the comma as `,` (`0x2C`), which sorts *below* `Z`
    /// (`0x5A`); `tok` writes it `\x2c`, whose leading `\` (`0x5C`) sorts
    /// *above* `Z`. An implementation sorting on `esc` emits the opposite
    /// order, and no length moves — which is why only a digest catches it.
    #[test]
    fn the_sort_key_is_tok_and_not_esc() {
        let comma = b"a,b".to_vec();
        let zed = b"aZb".to_vec();
        assert!(esc(&comma) < esc(&zed), "esc orders the comma first");
        assert!(tok(&comma) > tok(&zed), "tok orders the comma second");

        let set = WireSet::from_raised([
            tripwire_finding(Gate::G2, "a,b"),
            tripwire_finding(Gate::G2, "aZb"),
        ])
        .unwrap();
        assert_eq!(set.tokens(), vec!["G2:aZb", "G2:a\\x2cb"]);
        // And the `path` members are the *other* encoding, in that same order.
        assert_eq!(
            set.as_slice()
                .iter()
                .map(|w| w.path_member().unwrap())
                .collect::<Vec<_>>(),
            vec!["aZb", "a,b"]
        );
    }

    /// GR §6.2: `tok` moves exactly three bytes, and `=` is "deliberately
    /// **not** escaped: a trailer field splits on its first `=`".
    #[test]
    fn a_wire_token_escapes_comma_space_and_quote_but_never_equals() {
        let w = tripwire_finding(Gate::G2, "src/a=b.ts");
        assert_eq!(w.token(), "G2:src/a=b.ts");
        let w = tripwire_finding(Gate::G2, "a b\"c,d");
        assert_eq!(w.token(), "G2:a\\x20b\\x22c\\x2cd");
        assert_eq!(w.path_member().unwrap(), "a b\"c,d");
    }

    /// GR §6.1's uniqueness rule and PB §11's "`protected` dominates
    /// `tripwire`".
    #[test]
    fn two_entries_on_one_key_collapse_and_protected_survives() {
        let set = WireSet::from_raised([
            tripwire_finding(Gate::G8, "tests/support/db.py"),
            Wire::at(
                Gate::G8,
                "tests/support/db.py",
                WireClass::Protected,
                WireKind::Finding,
            ),
        ])
        .unwrap();
        assert_eq!(set.len(), 1);
        assert_eq!(set.as_slice()[0].class, WireClass::Protected);
    }

    /// GR §5.8: under the shipped defaults `C-M4: off` *and* precondition 0
    /// fails, "the two are raised independently … but they are the same key
    /// `(G11, pathless)`, so §6.1's uniqueness rule collapses them into a
    /// single entry … An implementation that emits two `G11` entries produces
    /// a wire array — and therefore a `wires=` line and an `envelope=` — that
    /// no conforming implementation reproduces."
    #[test]
    fn rule_fives_two_reasons_collapse_into_one_g11_entry() {
        let advisory = Wire::bare(Gate::G11, WireClass::Tripwire, WireKind::Advisory);
        let set = WireSet::from_raised([advisory.clone(), advisory]).unwrap();
        assert_eq!(set.tokens(), vec!["G11"]);
    }

    /// GR §6.1: the pathless case is "a distinct key", so a bare `G2` and a
    /// `G2:<path>` never collapse into each other.
    #[test]
    fn a_pathless_wire_is_a_distinct_key_from_a_path_bearing_one() {
        let set = WireSet::from_raised([
            Wire::bare(Gate::G2, WireClass::Tripwire, WireKind::Finding),
            tripwire_finding(Gate::G2, "src/a.ts"),
        ])
        .unwrap();
        assert_eq!(set.len(), 2);
    }

    /// GR §6.1: "The collapse can never merge an advisory into a finding."
    /// v1 has no gate that raises both, so meeting one is a defect, not a case
    /// to resolve.
    #[test]
    fn a_cross_kind_collapse_is_refused_rather_than_resolved() {
        let err = WireSet::from_raised([
            Wire::bare(Gate::G11, WireClass::Tripwire, WireKind::Advisory),
            Wire::bare(Gate::G11, WireClass::Tripwire, WireKind::Finding),
        ])
        .unwrap_err();
        assert_eq!(
            err,
            WireSetError::CrossKindCollapse {
                token: "G11".to_owned(),
                first: WireKind::Advisory,
                second: WireKind::Finding,
            }
        );
    }

    /// GR §6.1: "in v1 the only advisory-bearing gate is `G11`, and `G11`
    /// raises no findings, so every collapse is between two entries of the same
    /// kind" — so the `kind` precedence is reachable only for repeats.
    #[test]
    fn a_same_kind_collapse_keeps_that_kind() {
        let set = WireSet::from_raised([
            Wire::at(Gate::G2, "x", WireClass::Tripwire, WireKind::Warn),
            Wire::at(Gate::G2, "x", WireClass::Tripwire, WireKind::Warn),
        ])
        .unwrap();
        assert_eq!(set.as_slice()[0].kind, WireKind::Warn);
    }

    /// GR §6.2: containment "includes `warn` and `advisory` wires — every entry
    /// of the array", and "a review's `wires=` may name tokens absent from the
    /// report".
    #[test]
    fn containment_covers_every_kind_and_admits_a_larger_review() {
        let set = WireSet::from_raised([
            Wire::bare(Gate::G11, WireClass::Tripwire, WireKind::Advisory),
            Wire::at(Gate::G3, [], WireClass::Tripwire, WireKind::Warn),
            tripwire_finding(Gate::G2, "src/shared/util.ts"),
        ])
        .unwrap();
        let signed: BTreeSet<String> = set.tokens().into_iter().chain(["G4".to_owned()]).collect();
        assert!(set.contained_in(&signed));

        let short: BTreeSet<String> = ["G11".to_owned()].into_iter().collect();
        assert!(!set.contained_in(&short));
        assert_eq!(set.uncovered(&short), vec!["G2:src/shared/util.ts", "G3:"]);
    }

    /// GR §6.1's `spine stats` predicate — a function of the array and nothing
    /// else.
    #[test]
    fn the_g7_lease_predicate_reads_the_array_alone() {
        let lease_only = WireSet::from_raised([
            Wire::at(Gate::G7, "a", WireClass::Protected, WireKind::Finding),
            Wire::bare(Gate::G11, WireClass::Tripwire, WireKind::Advisory),
        ])
        .unwrap();
        assert!(lease_only.only_protected_wire_is_a_g7_lease());

        let with_floor = WireSet::from_raised([
            Wire::at(Gate::G7, "a", WireClass::Protected, WireKind::Finding),
            Wire::at(
                Gate::G14,
                "adr/1.md",
                WireClass::Protected,
                WireKind::Finding,
            ),
        ])
        .unwrap();
        assert!(!with_floor.only_protected_wire_is_a_g7_lease());

        assert!(!WireSet::default().only_protected_wire_is_a_g7_lease());
    }

    /// GR §6.1: for `G13`, `path` "carries that commit's oid … for which `esc`
    /// is the identity; the wire token is `G13:<oid>`".
    #[test]
    fn a_g13_wire_names_a_commit_and_esc_is_the_identity_over_it() {
        let oid = "77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9";
        let w = Wire::at(Gate::G13, oid, WireClass::Protected, WireKind::Finding);
        assert_eq!(w.token(), format!("G13:{oid}"));
        assert_eq!(w.path_member().unwrap(), oid);
    }
}
