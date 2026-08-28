//! `C-T1` and `C-T2` — the two constitution values that are functions of
//! `params.langs` (CN §6.4).
//!
//! Everything else in the `constitution@1` block is fixed bytes. These two are
//! the union of the per-language lists `import-resolver.md` §4.5, §5.5, §6.5
//! and §7.6 publish, and the union is what `spine init` renders.
//!
//! **`spine init` renders this exactly once.** The constitution is `user-owned`
//! (PB §6.7), so spine never rewrites it — not on an upgrade, not on a re-init.
//! Adding a language to `params.langs` later therefore leaves `C-T2` without
//! that runner's configuration patterns, and nothing in the design detects it
//! (CN §15 D3, §16 OPEN-4). That is a stated residual, not something this
//! module can fix.

/// The **fixed** language order CN §6.4 renders in — "deterministic given
/// `params.langs` and independent of the order the manifest happens to list it
/// in". The manifest stores `langs` sorted by bytes, which is a *different*
/// order (`dart, python, swift, ts`), so rendering in manifest order would
/// produce a different pattern list for the same repository.
pub const LANG_ORDER: [&str; 4] = ["python", "ts", "dart", "swift"];

/// CN §6.4's table, `C-T1` column.
fn test_roots(lang: &str) -> &'static [&'static str] {
    match lang {
        "python" => &["tests/"],
        "ts" => &["tests/", "src/**/__tests__/"],
        "dart" => &["test/"],
        "swift" => &["Tests/"],
        // `kotlin` has no row and there is no fifth token: a manifest carrying
        // one is `langs-unknown` and never reaches this rendering (CN §6.4,
        // MF §3.3). The token stays reserved rather than reusable.
        _ => &[],
    }
}

/// CN §6.4's table, `C-T2` column.
fn test_support(lang: &str) -> &'static [&'static str] {
    match lang {
        "python" => &[
            "tests/support/**",
            "**/conftest.py",
            "pytest.ini",
            "pyproject.toml",
            "tox.ini",
            "setup.cfg",
        ],
        "ts" => &[
            "tests/support/**",
            "package.json",
            "tsconfig.json",
            "jsconfig.json",
            // `vite.config.*` joined the row after IR §12.4.2 made
            // `vite.config.` a `C-T3` hook basename: without it a root
            // `vite.config.ts` outside `C-T2` was a class=protected
            // `G8:vite.config.ts` finding on every landing of every Vite
            // repository. It is what moved the union from 21/316 to 22/331.
            "vite.config.*",
            "vitest.config.*",
            "vitest.workspace.*",
            "vitest.setup.*",
            "jest.config.*",
            "jest.setup.*",
        ],
        "dart" => &[
            "test/support/**",
            "pubspec.yaml",
            "dart_test.yaml",
            "build.yaml",
        ],
        "swift" => &["Tests/Support/**", "Package.swift", "Package.resolved"],
        _ => &[],
    }
}

/// CN §6.4's render order: the fixed language order restricted to
/// `params.langs`, each language's list in table order, "with a byte-identical
/// pattern omitted after its first occurrence".
///
/// The dedup is why the four-language `C-T2` union is 22 and not 23:
/// `tests/support/**` is in both the `python` and the `ts` row.
fn union<'a>(langs: &[&str], table: fn(&str) -> &'a [&'a str]) -> Vec<&'a str> {
    let mut out: Vec<&str> = Vec::new();
    for lang in LANG_ORDER {
        if langs.contains(&lang) {
            for pattern in table(lang) {
                if !out.contains(pattern) {
                    out.push(pattern);
                }
            }
        }
    }
    out
}

pub fn c_t1_patterns(langs: &[&str]) -> Vec<&'static str> {
    union(langs, test_roots)
}

pub fn c_t2_patterns(langs: &[&str]) -> Vec<&'static str> {
    union(langs, test_support)
}

/// A `pattern-list` value as the constitution line spells it: patterns joined
/// by `", "` (CN §5.5).
pub fn join(patterns: &[&str]) -> String {
    patterns.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CN §6.4: "The full union is therefore **22 patterns, 331 bytes**."
    #[test]
    fn cn_6_4_four_language_union() {
        let all = ["python", "ts", "dart", "swift"];
        let c_t2 = c_t2_patterns(&all);
        assert_eq!(c_t2.len(), 22, "22 patterns");
        assert_eq!(join(&c_t2).len(), 331, "331 bytes joined by \", \"");

        // The dedup is load-bearing: 6 + 10 + 4 + 3 = 23 raw.
        let raw: usize = all.iter().map(|l| test_support(l).len()).sum();
        assert_eq!(raw, 23);
        assert_eq!(c_t2.iter().filter(|p| **p == "tests/support/**").count(), 1);
    }

    /// GR §8.1 / CN §6.4: the `c_t2` render for `["python","ts"]` is
    /// "15 patterns (6 + 10 − 1)". This is the value that moved GR §8.1's
    /// published length on its fifth recomputation.
    #[test]
    fn gr_8_1_c_t2_for_python_and_ts() {
        let c_t2 = c_t2_patterns(&["python", "ts"]);
        assert_eq!(c_t2.len(), 15);
        assert_eq!(
            join(&c_t2),
            "tests/support/**, **/conftest.py, pytest.ini, pyproject.toml, tox.ini, \
             setup.cfg, package.json, tsconfig.json, jsconfig.json, vite.config.*, \
             vitest.config.*, vitest.workspace.*, vitest.setup.*, jest.config.*, jest.setup.*"
        );
    }

    /// "Deterministic given `params.langs` and independent of the order the
    /// manifest happens to list it in." The manifest sorts `langs` by bytes,
    /// which is a different order, so this is not a theoretical concern.
    #[test]
    fn render_order_is_the_fixed_one_not_the_manifests() {
        let manifest_order = ["dart", "python", "swift", "ts"]; // sorted by bytes
        let fixed_order = ["python", "ts", "dart", "swift"];
        assert_ne!(manifest_order, fixed_order);
        assert_eq!(
            c_t2_patterns(&manifest_order),
            c_t2_patterns(&fixed_order),
            "the same set of languages must render one list"
        );
        assert_eq!(
            c_t1_patterns(&manifest_order),
            vec!["tests/", "src/**/__tests__/", "test/", "Tests/"],
            "and that list is in the fixed order, not the manifest's"
        );
    }

    #[test]
    fn single_language_repositories() {
        // MF §8.3's vector is a `["python"]` repository.
        assert_eq!(c_t1_patterns(&["python"]), vec!["tests/"]);
        assert_eq!(c_t2_patterns(&["python"]).len(), 6);
        assert_eq!(c_t1_patterns(&["swift"]), vec!["Tests/"]);
        assert_eq!(c_t2_patterns(&["dart"]).len(), 4);
    }

    /// `kotlin` has no row. It cannot reach here — MF §3.3 makes it
    /// `langs-unknown` — but rendering nothing rather than panicking keeps the
    /// refusal in the one place that owns it.
    #[test]
    fn a_reserved_token_contributes_nothing() {
        assert!(c_t1_patterns(&["kotlin"]).is_empty());
        assert_eq!(
            c_t2_patterns(&["python", "kotlin"]),
            c_t2_patterns(&["python"])
        );
    }
}
