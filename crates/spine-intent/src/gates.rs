//! ID §7 — the two gate predicates that are pure functions of the parse.
//!
//! ID §7 fixes them "so two implementations compute the same verdict"; what
//! enters `Δ`, how the exempt set `X` is computed, which branches are in
//! flight and how a wire is ordered are PB §5.2, PB §5.4, PB §6.3 and
//! `gate-report.md`'s, and are deliberately out of scope here (ID §14).
//!
//! **This is the module ID §1 is about.** "Another branch's leases are
//! evaluated by my binary. … when `spine check --land INT-042` runs, it fetches
//! every other in-flight intent branch and parses **their** documents to compute
//! G7. A landing is therefore refused or permitted on the strength of my
//! binary's reading of a document someone else wrote and someone else signed."

use crate::parse::Parsed;
use spine_canon::tok;
use spine_resolve::Pattern;

/// G2's two findings over one diff.
///
/// ID §7.1: `forbidden_hits` "→ **hard fail in every mode**, including
/// warn-before-block"; `outside` "→ a containment finding, `warn` under
/// calibration and `finding` otherwise".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct G2 {
    /// `{ p ∈ Δ \ X : ∃ f ∈ F . match(f, p) }`, in the order `Δ` supplied.
    pub forbidden_hits: Vec<Vec<u8>>,
    /// `{ p ∈ Δ \ X : ¬∃ e ∈ E . match(e, p) } \ forbidden_hits`.
    pub outside: Vec<Vec<u8>>,
}

impl G2 {
    /// The wire token for one path: `G2:<tok(p)>` (ID §7.1).
    ///
    /// `tok` — not `esc` — is the encoding, and the difference is load-bearing:
    /// `tok` additionally escapes `,`, space and `"`, which is what keeps a
    /// path out of the `wires=` separator a reviewer signs. **The order of the
    /// resulting array is `gate-report.md` §6.1's and is not this document's**
    /// (ID §7.1: "The wire order is `gate-report.md` §6.1's and is not
    /// restated"), so these are produced unsorted, in `Δ`'s order.
    pub fn wire(path: &[u8]) -> String {
        format!("G2:{}", tok(path))
    }

    pub fn is_clean(&self) -> bool {
        self.forbidden_hits.is_empty() && self.outside.is_empty()
    }
}

/// ID §7.1's G2 — Containment.
///
/// ```text
/// forbidden_hits := { p ∈ Δ \ X : ∃ f ∈ F . match(f, p) }
/// outside        := { p ∈ Δ \ X : ¬∃ e ∈ E . match(e, p) } \ forbidden_hits
/// ```
///
/// **Forbidden is evaluated first and dominates.** ID §11.8: "`expected: src/`,
/// `forbidden: src/auth/` is the natural way to write 'this subtree except
/// that' and is only coherent under this precedence." A path in both is
/// reported once, as a forbidden hit — "because two wires for one path over one
/// gate would collapse anyway under `gate-report.md` §6.1's uniqueness rule,
/// and the collapsed entry must be the finding, not the containment miss."
pub fn g2(parsed: &Parsed, delta: &[&[u8]], exempt: &[&[u8]]) -> G2 {
    let mut out = G2::default();
    for path in delta {
        if exempt.contains(path) {
            continue;
        }
        if parsed.forbidden.iter().any(|f| f.matches(path)) {
            out.forbidden_hits.push(path.to_vec());
        } else if !parsed.expected.iter().any(|e| e.matches(path)) {
            out.outside.push(path.to_vec());
        }
    }
    out
}

/// ID §7.2's G7 hard clause, for one other in-flight intent `J`.
///
/// ```text
/// hard(J) := ∃ p ∈ Δ . ( ∃ f ∈ J.forbidden . match(f, p) )  ∨  ( p ∈ J.frozen )
/// ```
///
/// `j_frozen` are "**concrete paths, not patterns**, and the test is byte
/// equality — a frozen path is a `(blob, path)` pair naming a file that exists,
/// and applying glob semantics to it would silently widen a freeze into a
/// subtree."
pub fn g7_hard(delta: &[&[u8]], j_forbidden: &[Pattern], j_frozen: &[&[u8]]) -> bool {
    delta.iter().any(|p| {
        j_forbidden.iter().any(|f| f.matches(p)) || j_frozen.contains(p)
    })
}

/// ID §7.3's `litprefix`.
///
/// ```text
/// litprefix(P) := P   if P contains none of * ? [
///                 the longest prefix of P that ends in "/" and lies wholly
///                 before the first occurrence of * ? [   otherwise
/// ```
///
/// "Truncating to the last `/` is load-bearing, not tidying. Without it,
/// `litprefix("ab*")` would be `ab`, `litprefix("abc/")` is `abc/`, neither is
/// a segment-prefix of the other — and both patterns match `abc/d`."
pub fn litprefix(pattern: &str) -> &str {
    match pattern.find(['*', '?', '[']) {
        None => pattern,
        Some(meta) => {
            let head = &pattern[..meta];
            match head.rfind('/') {
                Some(slash) => &head[..slash + 1],
                None => "",
            }
        }
    }
}

/// ID §7.3's `segprefix(a, b)` — "`a` is a segment-aligned prefix of `b`".
///
/// ```text
/// segprefix(a, b) := a = ""                -- empty is a prefix of everything
///                  ∨ a = b
///                  ∨ (a ends with "/" ∧ b starts with a)
///                  ∨ b starts with a ++ "/"
/// ```
pub fn segprefix(a: &str, b: &str) -> bool {
    a.is_empty()
        || a == b
        || (a.ends_with('/') && b.starts_with(a))
        || b.starts_with(&format!("{a}/"))
}

/// ID §7.3's `overlap` — a **sound over-approximation** of pattern
/// intersection, "which is the right shape for an advisory signal: it never
/// misses a real overlap, and its false positives cost a notification."
///
/// ID §7.3's soundness argument, and its check: "**Verified exhaustively** over
/// 2 926 legal patterns × 399 paths: for every path, every pair of patterns
/// matching it satisfies `overlap` — 0 violations."
pub fn overlap(p: &str, q: &str) -> bool {
    let (lp, lq) = (litprefix(p), litprefix(q));
    segprefix(lp, lq) || segprefix(lq, lp)
}

/// ID §7.3's soft lease: "Two intents interfere softly iff
/// `∃ e ∈ E_i, e' ∈ E_j . overlap(e, e')`."
///
/// ID §7.3 rejects evaluating it over a tree, and says why: doing so "misses
/// two intents that both declare a directory that does not exist yet — which is
/// the single most common way two agents collide on greenfield work."
pub fn g7_soft(a: &[Pattern], b: &[Pattern]) -> bool {
    a.iter()
        .any(|e| b.iter().any(|f| overlap(e.as_str(), f.as_str())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;
    use spine_resolve::pragma::IntentId;

    fn patterns(list: &[&str]) -> Vec<Pattern> {
        list.iter().map(|s| Pattern::parse(s).unwrap()).collect()
    }

    /// ID §9.6's published table, every row.
    #[test]
    fn id_9_6s_overlap_vectors_reproduce() {
        struct Row {
            p: &'static str,
            q: &'static str,
            lp: &'static str,
            lq: &'static str,
            overlaps: bool,
        }
        for row in [
            Row { p: "src/billing/", q: "src/billing/tax.ts", lp: "src/billing/", lq: "src/billing/tax.ts", overlaps: true },
            Row { p: "src/bill", q: "src/billing/", lp: "src/bill", lq: "src/billing/", overlaps: false },
            Row { p: "src/a/", q: "src/b/", lp: "src/a/", lq: "src/b/", overlaps: false },
            Row { p: "docs/", q: "src/", lp: "docs/", lq: "src/", overlaps: false },
            Row { p: "api/invoices.ts", q: "api/invoices.ts", lp: "api/invoices.ts", lq: "api/invoices.ts", overlaps: true },
            Row { p: "src/*/a.ts", q: "src/*/b.ts", lp: "src/", lq: "src/", overlaps: true },
            Row { p: "a*/x", q: "ab/x", lp: "", lq: "ab/x", overlaps: true },
            Row { p: "a*/x", q: "cd/x", lp: "", lq: "cd/x", overlaps: true },
            Row { p: "**/util.ts", q: "src/", lp: "", lq: "src/", overlaps: true },
        ] {
            assert_eq!(litprefix(row.p), row.lp, "litprefix({})", row.p);
            assert_eq!(litprefix(row.q), row.lq, "litprefix({})", row.q);
            assert_eq!(
                overlap(row.p, row.q),
                row.overlaps,
                "overlap({}, {})",
                row.p,
                row.q
            );
            // The relation is symmetric by construction; ID §7.3 writes it as a
            // disjunction of the two directions.
            assert_eq!(overlap(row.q, row.p), row.overlaps);
        }
    }

    /// ID §7.3: "The `a*/x` rows show why the truncation in `litprefix` is not
    /// optional: the first is a true overlap that a non-truncating definition
    /// would miss."
    #[test]
    fn the_truncation_in_litprefix_is_not_optional() {
        // Both patterns match `abc/d`.
        assert!(Pattern::parse("ab*").unwrap().matches_str("abc/d"));
        assert!(Pattern::parse("abc/").unwrap().matches_str("abc/d"));
        // Without truncation `litprefix("ab*")` would be `ab`, and neither
        // `ab` nor `abc/` is a segment-prefix of the other.
        assert!(!segprefix("ab", "abc/"));
        assert!(!segprefix("abc/", "ab"));
        // With it, the overlap is reported.
        assert_eq!(litprefix("ab*"), "");
        assert!(overlap("ab*", "abc/"));
    }

    /// ID §7.3's soundness claim, re-run rather than quoted: over the
    /// document's own alphabets, every pair of patterns that both match some
    /// path satisfies `overlap`.
    ///
    /// The generator is smaller than §7.3's 2 926 × 399 — depth 1–2 over a
    /// four-symbol alphabet — because the property is a universal one and this
    /// is a unit test; the published run is the exhaustive evidence.
    #[test]
    fn overlap_never_misses_a_real_overlap() {
        let segs = ["a", "ab", "x", "*", "**", "?", "a*", "[ab]"];
        let mut pats: Vec<String> = Vec::new();
        for a in segs {
            for trailing in ["", "/"] {
                pats.push(format!("{a}{trailing}"));
            }
            for b in segs {
                for trailing in ["", "/"] {
                    pats.push(format!("{a}/{b}{trailing}"));
                }
            }
        }
        let pats: Vec<Pattern> = pats.iter().filter_map(|s| Pattern::parse(s).ok()).collect();
        assert!(pats.len() > 100);

        let path_segs = ["a", "ab", "x", "b"];
        let mut paths: Vec<String> = Vec::new();
        for a in path_segs {
            paths.push(a.to_string());
            for b in path_segs {
                paths.push(format!("{a}/{b}"));
                for c in path_segs {
                    paths.push(format!("{a}/{b}/{c}"));
                }
            }
        }

        let mut violations = 0usize;
        for path in &paths {
            let hitters: Vec<&Pattern> = pats.iter().filter(|p| p.matches_str(path)).collect();
            for (i, p) in hitters.iter().enumerate() {
                for q in &hitters[i..] {
                    if !overlap(p.as_str(), q.as_str()) {
                        violations += 1;
                    }
                }
            }
        }
        assert_eq!(violations, 0, "overlap must never miss a real overlap");
    }

    // ---- ID §7.1 ----

    fn doc_with(expected: &str, forbidden: &str) -> Parsed {
        let d = format!(
            "# INT-042: t\nOwner: @a \u{00B7} Template: intent@2 \u{00B7} Constitution: v1\n\n\
             ## Goal\ng\n\n## Non-goals\n- a\n- b\n\n## Acceptance criteria\nAC-1: x\n\n\
             ## Touchpoints\nExpected to change: {expected}\nMust NOT change:{}\n",
            if forbidden.is_empty() {
                String::new()
            } else {
                format!(" {forbidden}")
            }
        );
        parse(d.as_bytes(), &IntentId::parse("INT-042").unwrap()).unwrap()
    }

    /// The precedence ID §11.8 resolves, on the case it calls "common and
    /// intended".
    #[test]
    fn forbidden_is_evaluated_first_and_a_path_in_both_is_reported_once() {
        let p = doc_with("src/", "src/auth/");
        let delta: Vec<&[u8]> = vec![b"src/auth/login.ts", b"src/billing/tax.ts", b"docs/readme.md"];
        let v = g2(&p, &delta, &[]);
        assert_eq!(v.forbidden_hits, vec![b"src/auth/login.ts".to_vec()]);
        assert_eq!(v.outside, vec![b"docs/readme.md".to_vec()]);
        // `src/auth/login.ts` matches `src/` too, and appears once.
        assert!(!v.outside.contains(&b"src/auth/login.ts".to_vec()));
    }

    #[test]
    fn the_exempt_set_removes_a_path_from_both_findings() {
        let p = doc_with("src/", "src/auth/");
        let delta: Vec<&[u8]> = vec![b"src/auth/login.ts", b"docs/readme.md"];
        let exempt: Vec<&[u8]> = vec![b"src/auth/login.ts", b"docs/readme.md"];
        assert!(g2(&p, &delta, &exempt).is_clean());
    }

    /// ID §6.3's headline row, reached through G2 rather than through the
    /// matcher: "an intent that declared one module would have licensed a
    /// differently-named sibling, and G2 would have passed a diff nobody
    /// declared."
    #[test]
    fn src_bill_does_not_license_src_billing() {
        let p = doc_with("src/bill", "");
        let delta: Vec<&[u8]> = vec![b"src/billing/x.ts"];
        assert_eq!(g2(&p, &delta, &[]).outside, vec![b"src/billing/x.ts".to_vec()]);
    }

    /// R2: the wire token is `tok`, not `esc`. They differ on exactly the bytes
    /// a `wires=` list would be split on.
    #[test]
    fn a_g2_wire_uses_tok_not_esc() {
        assert_eq!(G2::wire(b"src/billing/tax.ts"), "G2:src/billing/tax.ts");
        assert_eq!(G2::wire(b"a b"), "G2:a\\x20b");
        assert_eq!(G2::wire(b"a,b"), "G2:a\\x2cb");
        assert_ne!(G2::wire(b"a b"), format!("G2:{}", spine_canon::esc(b"a b")));
    }

    /// ID §6.1: "`esc` and `tok` are the identity on every legal pattern", so a
    /// pattern's own bytes are already its wire token.
    #[test]
    fn tok_is_the_identity_on_every_legal_pattern() {
        for source in ["src/billing/", "api/invoices.ts", "**", "src/[!abc]*.ts", "a*b*c"] {
            let p = Pattern::parse(source).unwrap();
            assert_eq!(tok(p.as_str().as_bytes()), source);
            assert_eq!(spine_canon::esc(p.as_str().as_bytes()), source);
        }
    }

    // ---- ID §7.2 ----

    #[test]
    fn a_frozen_path_is_matched_by_byte_equality_never_as_a_pattern() {
        let frozen: Vec<&[u8]> = vec![b"src/auth/login.ts"];
        let delta: Vec<&[u8]> = vec![b"src/auth/login.ts.bak"];
        // A glob reading of the frozen path would widen it to the sibling.
        assert!(!g7_hard(&delta, &[], &frozen));
        let exact: Vec<&[u8]> = vec![b"src/auth/login.ts"];
        assert!(g7_hard(&exact, &[], &frozen));
    }

    #[test]
    fn another_branchs_forbidden_patterns_are_matched_with_id_6_3s_match() {
        let delta: Vec<&[u8]> = vec![b"auth/session.ts"];
        assert!(g7_hard(&delta, &patterns(&["auth/"]), &[]));
        assert!(!g7_hard(&delta, &patterns(&["authz/"]), &[]));
        // OPEN-3's authorised case: `**` is legal and trips on everything.
        assert!(g7_hard(&delta, &patterns(&["**"]), &[]));
    }

    #[test]
    fn the_soft_clause_fires_on_two_greenfield_directories_that_do_not_exist() {
        assert!(g7_soft(
            &patterns(&["src/webhooks/"]),
            &patterns(&["src/webhooks/retry.ts"])
        ));
        assert!(!g7_soft(&patterns(&["src/a/"]), &patterns(&["src/b/"])));
    }
}
