//! The `constitution@1` render — the bytes `spine init` seeds.
//!
//! CN §6.2 prints the twelve-rule block and calls it "the canonical bytes, and
//! this is the only place they are fixed". It is the only place the *rules* are
//! fixed; it is **not** the whole file. CN §3.1 and §9.1 each require one more
//! line above it, and both are mandatory with their own refusals:
//!
//! - §3.1: "Line 1 is the title line. It must exist … `spine init` writes
//!   `# Constitution — <repo>`." Absent: `missing-title`.
//! - §3.1, §9.1: "Line 2 is the header line. It must exist (`missing-header`)."
//!
//! So the render is the preamble §3.1 states, then §6.2's block. Rendering
//! §6.2's block alone yields a file whose line 1 is `# The non-negotiables` and
//! whose line 2 is blank — `missing-header` on the seed of every repository.
//! `.build-notes/FINDINGS-constitution-seed.md` records that, and the two
//! values §6.2 leaves without a source.

use crate::harness;

/// CN §9.1 field 1: `v` + a decimal integer, no leading zeros.
///
/// The seed is `v1` and the corpus never says so: no document in the set
/// contains the bytes `Version: v1`. It is the only value consistent with §9.3
/// ("the version must change when the file changes") on a file that has not
/// changed yet, and with §9.2 ("it is not a clock").
pub const SEED_VERSION: u32 = 1;

/// CN §6.2's twelve-rule block, byte for byte, with `C-T1` and `C-T2` left as
/// the two spans §6.4 fills.
///
/// Everything here is fixed in every repository on every platform. The two
/// `<per §6.4>` markers are the only variance, and CN §6.2 says so: "the `C-T1`
/// and `C-T2` values are §6.4's function of `params.langs`; every other byte is
/// fixed."
const RULES_BLOCK: &str = include_str!("../tests/vectors/cn-6.2-block.txt");

/// What `spine init` needs in order to render the seed.
#[derive(Debug, Clone)]
pub struct Seed<'a> {
    /// The manifest's `repo` — the basename of the git toplevel (owner ruling,
    /// 2026-08-27), substituted into §3.1's title line.
    pub repo: &'a str,
    /// The principal of the signing identity, taken **verbatim with no `@`
    /// prefix added**, which is TM §6.1 substitution 2's rule for the intent
    /// scaffold's `Owner:`. The corpus states no rule for the constitution's,
    /// and this is its only precedent; CN §12.1's `@alice` is a human's own
    /// later edit, and §9.1 makes the field "read by no gate".
    pub owner: &'a str,
    /// `params.langs`, in any order — CN §6.4's render order is fixed and
    /// independent of it.
    pub langs: &'a [&'a str],
}

/// Render the seeded `CONSTITUTION.md`.
///
/// Every line is `0x0A`-terminated, there is no `0x0D` anywhere, and there is
/// exactly one trailing `0x0A` (CN §2.2).
pub fn render(seed: &Seed<'_>) -> String {
    let c_t1 = harness::join(&harness::c_t1_patterns(seed.langs));
    let c_t2 = harness::join(&harness::c_t2_patterns(seed.langs));

    let mut out = String::with_capacity(RULES_BLOCK.len() + 256);
    // §3.1's title line, verbatim: "`spine init` writes `# Constitution —
    // <repo>`". The dash is U+2014, as CN §12.1's line 1 carries it.
    out.push_str("# Constitution \u{2014} ");
    out.push_str(seed.repo);
    out.push('\n');
    // §9.1's header: fields in the table's order, separated by " · " (U+00B7
    // with a space either side). `Resign` is optional and absent means false,
    // so the seed omits it.
    out.push_str(&format!(
        "Version: v{} \u{00B7} Owner: {}\n",
        SEED_VERSION, seed.owner
    ));
    out.push('\n');

    out.push_str(
        &RULES_BLOCK
            .replace("C-T1: test.roots = <per §6.4>", &format!("C-T1: test.roots = {c_t1}"))
            .replace(
                "C-T2: test.support = <per §6.4>",
                &format!("C-T2: test.support = {c_t2}"),
            ),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed<'a>(langs: &'a [&'a str]) -> Seed<'a> {
        Seed {
            repo: "myrepo",
            owner: "alice@example.com",
            langs,
        }
    }

    /// CN §3.1 and §9.1 both name a mandatory line the §6.2 block does not
    /// carry. Without them the seed takes `missing-title` or `missing-header`
    /// on the first landing of every repository.
    #[test]
    fn the_preamble_is_present_and_in_cn_3_1s_positions() {
        let rendered = render(&seed(&["python"]));
        let lines: Vec<&str> = rendered.lines().collect();
        assert_eq!(lines[0], "# Constitution — myrepo");
        assert_eq!(lines[1], "Version: v1 · Owner: alice@example.com");
        assert_eq!(lines[2], "", "§6.2's block begins after one blank line");
        assert_eq!(lines[3], "# The non-negotiables");
    }

    /// §9.1's field order is the table's: Version, Owner, then the optional
    /// Resign. The separator is `" · "` — U+00B7 with a space either side —
    /// which is also `intent-doc.md` §4.3's, adopted "so that the two
    /// hand-adjacent artifacts do not have two header syntaxes".
    #[test]
    fn the_header_uses_id_4_3s_separator_and_omits_the_optional_field() {
        let rendered = render(&seed(&["python"]));
        let header = rendered.lines().nth(1).unwrap();
        assert!(header.contains(" \u{00B7} "));
        assert!(
            !header.contains("Resign"),
            "absent means false (§9.1), so the seed omits it"
        );
        assert!(
            header.starts_with("Version: "),
            "fields appear in the table's order"
        );
    }

    /// TM §6.1 substitution 2, applied here for want of a rule of its own:
    /// the principal goes in **verbatim, with no `@` prefix added**. Prefixing
    /// would produce `@alice@example.com`.
    #[test]
    fn the_owner_principal_is_verbatim() {
        let rendered = render(&seed(&["python"]));
        assert!(rendered.contains("Owner: alice@example.com"));
        assert!(!rendered.contains("@alice@example.com"));
    }

    /// CN §6.2: "the `C-T1` and `C-T2` values are §6.4's function of
    /// `params.langs`; every other byte is fixed."
    #[test]
    fn only_c_t1_and_c_t2_vary_with_langs() {
        let one = render(&seed(&["python"]));
        let all = render(&seed(&["python", "ts", "dart", "swift"]));

        assert!(one.contains("C-T1: test.roots = tests/\n"));
        assert!(one.contains(
            "C-T2: test.support = tests/support/**, **/conftest.py, pytest.ini, \
             pyproject.toml, tox.ini, setup.cfg\n"
        ));
        assert!(all.contains("C-T1: test.roots = tests/, src/**/__tests__/, test/, Tests/\n"));

        // No marker survives either render.
        for rendered in [&one, &all] {
            assert!(!rendered.contains("<per §6.4>"));
            assert!(!rendered.contains("<repo>"));
        }

        // And nothing else moved: strip the two rule lines and the rest is
        // byte-identical.
        let strip = |s: &str| {
            s.lines()
                .filter(|l| !l.starts_with("C-T1:") && !l.starts_with("C-T2:"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(strip(&one), strip(&all));
    }

    /// CN §2.2's byte rules.
    #[test]
    fn byte_rules_hold() {
        let rendered = render(&seed(&["python", "ts"]));
        assert!(!rendered.contains('\r'), "no 0x0D anywhere");
        assert!(rendered.ends_with('\n'));
        assert!(!rendered.ends_with("\n\n"), "exactly one trailing 0x0A");
    }

    /// All twelve scaffolded rules, each with its `enforced_by:` line
    /// (CN §6.2). A missing rule takes the fail-closed default (§7), which is
    /// silent — so the count is worth asserting.
    #[test]
    fn all_twelve_scaffolded_rules_are_present_with_their_gates() {
        let rendered = render(&seed(&["python"]));
        for (id, gate) in [
            ("C-A1", "spine:G13"),
            ("C-A2", "spine:G14"),
            ("C-A3", "spine:G11"),
            ("C-M1", "spine:G9"),
            ("C-M2", "spine:G11"),
            ("C-M3", "spine:G11"),
            ("C-M4", "spine:G11"),
            ("C-Q1", "spine:G2"),
            ("C-Q2", "spine:G2"),
            ("C-T1", "spine:G8"),
            ("C-T2", "spine:G8"),
            ("C-T3", "spine:G8"),
        ] {
            let line = rendered
                .lines()
                .position(|l| l.starts_with(&format!("{id}: ")))
                .unwrap_or_else(|| panic!("{id} is missing from the seed"));
            assert_eq!(
                rendered.lines().nth(line + 1).unwrap().trim(),
                format!("enforced_by: {gate}"),
                "{id}'s enforced_by"
            );
        }
        assert_eq!(
            rendered.lines().filter(|l| l.starts_with("C-")).count(),
            12,
            "twelve scaffolded rules and no more"
        );
    }

    /// The two narrowed defaults CN §6.2 argues for, because "these are the
    /// values a repository is stuck with": `C-A2` is monotone (§6.5) so a
    /// seeded pattern can never be removed, and `C-Q1` is the entire boundary
    /// of the lane that lands with no intent doc and no frozen test.
    #[test]
    fn the_two_narrowed_defaults_are_the_narrow_ones() {
        let rendered = render(&seed(&["python"]));
        assert!(rendered.contains("C-A2: protected = adr/\n"));
        assert!(!rendered.contains("db/migrations/"));
        assert!(rendered.contains("C-Q1: quick.paths = docs/\n"));
        assert!(!rendered.contains("src/**"));
    }
}
