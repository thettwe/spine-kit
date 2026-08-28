//! The `wires` array: its entries, its uniqueness rule, and its order.
//!
//! Two orders exist in one report and they are not the same order (GR §5.6):
//! `gates[]` sorts by gate **number**, `wires[]` "ascending by unsigned byte
//! value over the whole token, so `G11` precedes `G2`". Re-sorting `wires` is a
//! permutation, so a byte count is unchanged by getting it wrong and only the
//! digests move — `docs/spec/README.md`'s closed known-gap 2: "an implementation
//! that matches every published length and neither digest has a numeric wire
//! comparator, not a broken canonicalizer."
//!
//! The sort key is **`tok(path)`, not `esc(path)`** (GR §6.1): "the two differ
//! on `,`, ` ` and `\"` … and sorting the array on one key while the line is
//! written under the other produces a `wires=` whose order does not match the
//! array's over the same findings."

use crate::gate::Gate;
use core::fmt;
use spine_canon::{esc, tok};

/// GR §6.1. Two values, and neither is free: `class` "decides the landing's
/// review state through PB §11's aggregation and is inside `report=` and
/// `envelope=`."
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WireClass {
    Tripwire,
    /// PB §11: "`protected` dominates `tripwire`".
    Protected,
}

impl WireClass {
    pub fn as_str(self) -> &'static str {
        match self {
            WireClass::Tripwire => "tripwire",
            WireClass::Protected => "protected",
        }
    }
}

impl fmt::Display for WireClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// GR §6.1's three kinds. The `Ord` derived here is the collapse precedence —
/// "the strongest of `finding` > `advisory` > `warn`" — so the variants are
/// declared weakest-first and `max` is the surviving kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WireKind {
    /// "a Drift finding under warn-before-block calibration … does **not**
    /// route, does **not** affect gate status."
    Warn,
    /// "the gate raised a wire that is not a finding about itself. **The only
    /// advisory wire in v1 is `G11`**."
    Advisory,
    /// "the gate's own check was not satisfied."
    Finding,
}

impl WireKind {
    pub fn as_str(self) -> &'static str {
        match self {
            WireKind::Warn => "warn",
            WireKind::Advisory => "advisory",
            WireKind::Finding => "finding",
        }
    }
}

impl fmt::Display for WireKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One entry of the `wires` array.
///
/// `path` holds **raw bytes**, never an encoding: the report member is
/// `esc(path)` and the wire token is `tok(path)`, and R2's whole point is that
/// one landing can carry both spellings of one path — `floor_hits` stores
/// `esc`, the derived `G14` token is `tok`, and `Spine-Frozen` C-quotes it.
/// Storing the encoded form would fix one of the three and lose the others.
///
/// For `G13` the bytes are a commit oid, "lowercase hex at the length
/// `object_format` implies, for which both `esc` and `tok` are the identity"
/// (MF §4.8.1) — so no special case is needed here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wire {
    pub gate: Gate,
    pub path: Option<Vec<u8>>,
    pub class: WireClass,
    pub kind: WireKind,
}

impl Wire {
    /// A wire that names no path. GR §6.1 keys it distinctly from every
    /// path-bearing entry of the same gate.
    pub fn pathless(gate: Gate, class: WireClass, kind: WireKind) -> Self {
        Wire {
            gate,
            path: None,
            class,
            kind,
        }
    }

    /// A wire that names a path.
    ///
    /// **An empty path is no path**, and this is where that is enforced rather
    /// than at each caller. RF §8.5: "`G1:` with nothing after the colon is
    /// **never written**: it is not the bare id and it names nothing, so it
    /// could be neither cited by a reviewer nor distinguished from a typo."
    /// RF §4.4 emits the empty string where no tree entry matches the runner's
    /// reported path, so this case is reachable from real collector output.
    pub fn at(gate: Gate, path: impl Into<Vec<u8>>, class: WireClass, kind: WireKind) -> Self {
        let path = path.into();
        if path.is_empty() {
            return Wire::pathless(gate, class, kind);
        }
        Wire {
            gate,
            path: Some(path),
            class,
            kind,
        }
    }

    /// GR §6.2: "`G<n>` when `path` is absent; `G<n>` + `:` + `tok(path)`
    /// otherwise."
    ///
    /// This is the string a reviewer signs over inside `wires=`, and it is also
    /// this array's sort key.
    pub fn token(&self) -> String {
        match &self.path {
            None => self.gate.id().to_string(),
            Some(p) => format!("{}:{}", self.gate.id(), tok(p)),
        }
    }

    /// The `path` member of the report entry — `esc`, not `tok` (GR §6.1).
    pub fn esc_path(&self) -> Option<String> {
        self.path.as_deref().map(esc)
    }

    /// GR §6.1's uniqueness key: "`(gate, path)` — with the pathless case
    /// treated as a distinct key — appears at most once."
    fn key(&self) -> (u8, Option<&[u8]>) {
        (self.gate.number(), self.path.as_deref())
    }
}

/// The accumulated wire set of one landing.
///
/// PB §11: "**The complete wire set is computed before any lane routes it**,
/// and it is computed the same way for every landing that runs gates — gated,
/// quick and lifecycle alike. Lane decides the ceremony; it never decides which
/// wires exist."
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WireSet {
    entries: Vec<Wire>,
}

impl WireSet {
    pub fn new() -> Self {
        WireSet {
            entries: Vec::new(),
        }
    }

    /// Accumulate one wire, collapsing on GR §6.1's key.
    ///
    /// "If one evaluation would produce the same key twice, the entries
    /// collapse and the surviving `class` is `\"protected\"` if either was, per
    /// PB §11's \"`protected` dominates `tripwire`\"; the surviving `kind` is
    /// the strongest of `finding` > `advisory` > `warn`."
    pub fn insert(&mut self, wire: Wire) {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.key() == wire.key()) {
            existing.class = existing.class.max(wire.class);
            existing.kind = existing.kind.max(wire.kind);
            return;
        }
        self.entries.push(wire);
    }

    pub fn extend(&mut self, wires: impl IntoIterator<Item = Wire>) {
        for wire in wires {
            self.insert(wire);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The array, in report order: ascending by unsigned byte value over the
    /// whole wire token (PB §11, GR §6.1).
    ///
    /// `str`'s `Ord` is byte-wise and every token is ASCII by construction
    /// (`esc` and `tok` emit `U+0020..=U+007E` only, GR §2.3), so comparing the
    /// `String`s **is** the unsigned-byte comparison the rule names.
    pub fn ordered(&self) -> Vec<Wire> {
        let mut out = self.entries.clone();
        out.sort_by_key(Wire::token);
        out
    }

    /// The tokens, in the same order.
    pub fn tokens(&self) -> Vec<String> {
        self.ordered().iter().map(Wire::token).collect()
    }

    /// The value of a `Spine-Review`'s `wires=` field over this set.
    ///
    /// "One key — the token's bytes — governs both the array and the line, so
    /// the line is the array's tokens joined by `,` and nothing has to be
    /// re-sorted to write it" (GR §6.2).
    pub fn wires_line(&self) -> String {
        self.tokens().join(",")
    }

    /// Every wire this gate raised, in token order.
    pub fn of(&self, gate: Gate) -> Vec<Wire> {
        self.ordered().into_iter().filter(|w| w.gate == gate).collect()
    }

    /// PB §11's aggregation: "`protected` dominates `tripwire`, and a landing
    /// has exactly one review state. A `protected` wire anywhere in the set
    /// makes the landing `protected-review` … There is no first-match rule and
    /// no combined state."
    pub fn has_protected(&self) -> bool {
        self.entries.iter().any(|w| w.class == WireClass::Protected)
    }

    /// GR §6.1's counter predicate, "derived at read time and never stored":
    /// it "holds iff some entry has `gate == \"G7\"` and `class ==
    /// \"protected\"`, and no other entry has `class == \"protected\"`."
    pub fn only_protected_is_a_g7_hard_lease(&self) -> bool {
        let mut saw_g7 = false;
        for wire in &self.entries {
            if wire.class != WireClass::Protected {
                continue;
            }
            if wire.gate == Gate::G7 {
                saw_g7 = true;
            } else {
                return false;
            }
        }
        saw_g7
    }
}

impl FromIterator<Wire> for WireSet {
    fn from_iter<I: IntoIterator<Item = Wire>>(iter: I) -> Self {
        let mut set = WireSet::new();
        set.extend(iter);
        set
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tripwire_finding(gate: Gate, path: Option<&str>) -> Wire {
        match path {
            None => Wire::pathless(gate, WireClass::Tripwire, WireKind::Finding),
            Some(p) => Wire::at(gate, p, WireClass::Tripwire, WireKind::Finding),
        }
    }

    /// GR §6.2, §9.19: "A **numeric** sort — `G2:src/shared/util.ts,G11` — is
    /// **non-conforming**." The canonical line appears four times in the corpus
    /// (PB §5.5, EV §8.3, GR §8.1, GR §8.2) and reads the other way round.
    #[test]
    fn a_numeric_wire_comparator_is_non_conforming() {
        let mut set = WireSet::new();
        set.insert(tripwire_finding(Gate::G2, Some("src/shared/util.ts")));
        set.insert(Wire::pathless(
            Gate::G11,
            WireClass::Tripwire,
            WireKind::Advisory,
        ));
        assert_eq!(set.wires_line(), "G11,G2:src/shared/util.ts");
    }

    /// GR §6.1, in full: "`G1` precedes `G11`, and within one gate the pathless
    /// entry precedes every `:`-suffixed one because its token is a proper
    /// prefix of theirs."
    ///
    /// It does **not** follow that `G1:a.py` precedes `G11`, and the byte order
    /// says otherwise: at the third byte `1` is `0x31` and `:` is `0x3A`, so
    /// `G11` sorts between `G1` and `G1:a.py`. Nothing in the corpus groups an
    /// array by gate, and an implementation that "tidied" this into
    /// `G1, G1:a.py, G11` would write a different `wires=` and a different
    /// `envelope=` over identical findings.
    #[test]
    fn g11_sorts_between_g1_and_g1_colon_a_py() {
        let mut set = WireSet::new();
        set.insert(tripwire_finding(Gate::G1, Some("a.py")));
        set.insert(Wire::pathless(
            Gate::G11,
            WireClass::Tripwire,
            WireKind::Advisory,
        ));
        set.insert(tripwire_finding(Gate::G1, None));
        assert_eq!(set.tokens(), ["G1", "G11", "G1:a.py"]);
        // The third byte is what decides it: `1` is 0x31, `:` is 0x3A.
        assert_eq!((b'1', b':'), (0x31, 0x3A));
    }

    /// R2. GR §6.1: "The sort key is the token's bytes, which for a
    /// path-bearing entry means `tok(path)` and not `esc(path)`."
    ///
    /// `esc` leaves a space at `0x20`; `tok` writes `\x20`, whose first byte is
    /// `\` (`0x5C`). The two keys order these two paths oppositely, so a set
    /// sorted on `esc` writes a different line over identical findings.
    #[test]
    fn the_sort_key_is_tok_not_esc() {
        let mut set = WireSet::new();
        set.insert(tripwire_finding(Gate::G2, Some("a b")));
        set.insert(tripwire_finding(Gate::G2, Some("a!")));
        // Under `tok`: "G2:a\\x20b" vs "G2:a!" -> `!` (0x21) < `\` (0x5C).
        assert_eq!(set.tokens(), ["G2:a!", "G2:a\\x20b"]);
        // Under `esc` the space (0x20) would have sorted first.
        assert!(esc(b"a b") < esc(b"a!"));
    }

    /// GR §6.1's collapse. Two offending pragmas in one blob are one `G5:<path>`
    /// entry (GR §6.3), and the surviving class is the stronger of the two.
    #[test]
    fn a_repeated_key_collapses_and_protected_dominates_tripwire() {
        let mut set = WireSet::new();
        set.insert(Wire::at(
            Gate::G8,
            "harness.py",
            WireClass::Tripwire,
            WireKind::Finding,
        ));
        set.insert(Wire::at(
            Gate::G8,
            "harness.py",
            WireClass::Protected,
            WireKind::Finding,
        ));
        assert_eq!(set.len(), 1);
        assert_eq!(set.ordered()[0].class, WireClass::Protected);
    }

    #[test]
    fn the_collapse_takes_the_strongest_kind() {
        assert!(WireKind::Finding > WireKind::Advisory);
        assert!(WireKind::Advisory > WireKind::Warn);
        let mut set = WireSet::new();
        set.insert(Wire::at(Gate::G2, "x.ts", WireClass::Tripwire, WireKind::Warn));
        set.insert(Wire::at(
            Gate::G2,
            "x.ts",
            WireClass::Tripwire,
            WireKind::Finding,
        ));
        assert_eq!(set.ordered()[0].kind, WireKind::Finding);
    }

    /// RF §8.5, §13 R19: "`G1:` with nothing after the colon is **never
    /// written**; an empty `path` is no path and takes the bare `G1`."
    #[test]
    fn an_empty_path_is_no_path_and_takes_the_bare_id() {
        let wire = Wire::at(Gate::G1, "", WireClass::Protected, WireKind::Finding);
        assert_eq!(wire.token(), "G1");
        assert!(wire.path.is_none());
    }

    /// GR §6.1: `path` is `esc`-encoded in the report and `tok`-encoded in the
    /// token. One wire, two spellings, and the crate stores neither.
    #[test]
    fn a_wire_carries_esc_in_the_report_and_tok_in_the_token() {
        let wire = Wire::at(
            Gate::G14,
            "docs/a b,c.md",
            WireClass::Protected,
            WireKind::Finding,
        );
        assert_eq!(wire.esc_path().unwrap(), "docs/a b,c.md");
        assert_eq!(wire.token(), "G14:docs/a\\x20b\\x2cc.md");
    }

    #[test]
    fn a_tripwire_only_set_is_not_protected_review() {
        let mut set = WireSet::new();
        set.insert(Wire::pathless(
            Gate::G11,
            WireClass::Tripwire,
            WireKind::Advisory,
        ));
        assert!(!set.has_protected());
        set.insert(Wire::at(
            Gate::G14,
            "CODEOWNERS",
            WireClass::Protected,
            WireKind::Finding,
        ));
        assert!(set.has_protected());
    }

    #[test]
    fn the_g7_hard_lease_counter_reads_the_array_and_nothing_else() {
        let mut set = WireSet::new();
        set.insert(Wire::at(
            Gate::G7,
            "src/a.ts",
            WireClass::Protected,
            WireKind::Finding,
        ));
        set.insert(Wire::pathless(
            Gate::G11,
            WireClass::Tripwire,
            WireKind::Advisory,
        ));
        assert!(set.only_protected_is_a_g7_hard_lease());
        set.insert(Wire::at(
            Gate::G14,
            "CODEOWNERS",
            WireClass::Protected,
            WireKind::Finding,
        ));
        assert!(!set.only_protected_is_a_g7_hard_lease());
    }
}
