//! `lang(path)`, the four `params.langs` tokens, and the closed refusal
//! vocabularies of IR §3.8's three-level unclassifiable ladder.
//!
//! IR §3.1: "Total, byte-exact on the final path component, lowercase only.
//! `.PY` is not Python; a repository that ships uppercase extensions gets
//! `none` … because the alternative is a case-folding rule that differs between
//! an approving macOS laptop and a Linux CI container."

use core::fmt;

/// The four v1 languages. IR §3.1: "**The four `params.langs` tokens are
/// ratified as exactly `python`, `ts`, `dart` and `swift`.** … they are
/// permanent: `params.langs` is a floor-relevant manifest field (PB §6.3, G16),
/// so a token cannot be corrected later without a floor-protected landing."
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Lang {
    Python,
    /// `ts` covers JavaScript. IR §3.1: "PB §6.7 counts 'TypeScript/JavaScript'
    /// as one language and there is one resolver for both."
    Ts,
    Dart,
    Swift,
}

/// `kotlin` is a `params.langs` value that no v1 release emits.
///
/// IR §11.1: "**reserved** — §18 OPEN-1 dropped the language; its analysis is
/// Appendix A, so a later release finds the string free. No v1 release emits
/// it, and a `params.langs` containing it is refused by `result-file.md` §7.1
/// step 3 as a language with no adapter."
///
/// [`Lang::from_token`] must therefore answer `None` for it, and does.
pub const RESERVED_LANG_TOKENS: [&str; 1] = ["kotlin"];

impl Lang {
    /// The `params.langs` token. These bytes are sealed into landings forever.
    pub const fn token(self) -> &'static str {
        match self {
            Lang::Python => "python",
            Lang::Ts => "ts",
            Lang::Dart => "dart",
            Lang::Swift => "swift",
        }
    }

    /// The closed token set. A reserved token (`kotlin`) answers `None` — it is
    /// not a language this release has an adapter for, and admitting it here
    /// would spend a token a later release needs.
    pub fn from_token(token: &str) -> Option<Self> {
        match token {
            "python" => Some(Lang::Python),
            "ts" => Some(Lang::Ts),
            "dart" => Some(Lang::Dart),
            "swift" => Some(Lang::Swift),
            _ => None,
        }
    }

    /// Every language, in the order CN §6.4 renders `C-T1`/`C-T2` in — which is
    /// *not* the manifest's byte order (`dart, python, swift, ts`).
    pub const ALL: [Lang; 4] = [Lang::Python, Lang::Ts, Lang::Dart, Lang::Swift];
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// The final path component — the bytes after the last `/`, or the whole path
/// when it holds none.
pub fn basename(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

/// IR §3.1's table. **Byte-exact on the final component, lowercase only.**
///
/// Anything not in the table is `none` — `.kt`, `.kts`, `.java`, `.m`, `.go`,
/// `.pyi`, `.vue`, `.svelte` — and IR §12.4.3 states the residual plainly: "the
/// same files that can hold an oracle the closure cannot see can hold a
/// framework import the grep cannot see."
pub fn lang(path: &str) -> Option<Lang> {
    let name = basename(path);
    // The rule is "final component **ends with**", byte-exact, and nothing
    // more: no stem is required, no case is folded, no dot is counted. A file
    // whose whole name is `.py` therefore is Python, which is the literal
    // reading and the only one two implementations reach without a second rule.
    const TABLE: [(&str, Lang); 11] = [
        (".py", Lang::Python),
        (".ts", Lang::Ts),
        (".tsx", Lang::Ts),
        (".mts", Lang::Ts),
        (".cts", Lang::Ts),
        (".js", Lang::Ts),
        (".jsx", Lang::Ts),
        (".mjs", Lang::Ts),
        (".cjs", Lang::Ts),
        (".dart", Lang::Dart),
        (".swift", Lang::Swift),
    ];
    // `.d.ts`, `.d.mts` and `.d.cts` need no rows: they end in `.ts`, `.mts`
    // and `.cts` and are TypeScript by extension (IR §3.1). What separates them
    // is `is_declaration`, which makes their sites `type_only`, not their lang.
    TABLE
        .iter()
        .find(|(ext, _)| name.ends_with(ext))
        .map(|(_, l)| *l)
}

/// IR §3.1: "`.d.ts`, `.d.mts`, `.d.cts` are type-only by construction. … every
/// import site in them is `type_only` and they are never a resolution target
/// for a value import (§5.2 skips them in candidate expansion). A declaration
/// file contains no runtime code, so nothing in it can weaken an oracle."
pub fn is_declaration(path: &str) -> bool {
    let name = basename(path);
    [".d.ts", ".d.mts", ".d.cts"]
        .iter()
        .any(|ext| name.ends_with(ext))
}

/// IR §7.3 test 1's list, verbatim and in its published order:
///
/// > `.m  .mm  .h  .hh  .hpp  .hxx  .pch  .c  .cc  .cpp  .cxx  .modulemap`
///
/// "The list is C-family rather than Objective-C alone, and the reason token is
/// still `mixed-objc-target`, because Objective-C's interop surface is C's: a
/// `.c` behind a `.h` is the same invisible oracle."
pub const C_FAMILY_EXTENSIONS: [&str; 12] = [
    ".m", ".mm", ".h", ".hh", ".hpp", ".hxx", ".pch", ".c", ".cc", ".cpp", ".cxx", ".modulemap",
];

/// IR §7.3 test 1, "matched byte-exactly and lowercase only, exactly as §3.1's
/// table is matched".
///
/// **No rule here reads a filename stem** — that is why every spelling of a
/// bridging header, `<Target>-Bridging-Header.h` included, is caught by `.h`
/// alone and needs no clause of its own (IR §7.3, case S14).
pub fn is_c_family(path: &str) -> bool {
    let name = basename(path);
    C_FAMILY_EXTENSIONS.iter().any(|ext| name.ends_with(ext))
}

/// IR §3.8's **language** level: "every file of that language contributes no
/// edges and is never added by an import edge; `lang-unclassifiable`; clause 3
/// and clause 4 still apply."
///
/// The variants are the closed per-language lists of §4.7, §5.7, §6.7 and §7.8
/// plus §3.3 Rule 2's shared reason. `Display` is the reason token those
/// sections fix, because it reaches a `spine stats` counter and a human's
/// `reason=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LangUnclassifiable {
    /// IR §3.3 Rule 2, shared by every language that has an `RC` at all: "if
    /// `RC(lang, A) ≠ RC(lang, B)`, the language is unclassifiable for this
    /// approval."
    RcChangedOnBranch,
    // --- TypeScript, IR §5.7 ---
    TsconfigUnparseable,
    TsconfigExtendsExternal,
    TsconfigExtendsCycle,
    BaseurlEscapesRoot,
    PathsMalformed,
    // --- Dart, IR §6.7 ---
    PubspecNameMalformed,
    PubspecNotDeclarative,
    DuplicatePackageName,
    // --- Swift, IR §7.8 ---
    ManifestNotLiteral,
    DuplicateTargetName,
    XcodeProjectUnsupported,
    TargetDirMissing,
    OverlappingTargets,
    /// IR §7.3. The rule that removed Kotlin, applied to Swift rather than a
    /// second judgement being made about the same shape: "a guarantee that
    /// fails loudly can ship with its limits stated, one that fails silently
    /// cannot."
    MixedObjcTarget,
    /// "a Swift file exists and no `Package.swift` does" (IR §7.8).
    NoPackageManifest,
}

impl LangUnclassifiable {
    /// The closed list each language section publishes. Python has none:
    /// IR §4.2, "Python therefore never raises `lang-unclassifiable`."
    pub fn is_reachable_for(self, lang: Lang) -> bool {
        use LangUnclassifiable as R;
        match lang {
            Lang::Python => false,
            Lang::Ts => matches!(
                self,
                R::TsconfigUnparseable
                    | R::TsconfigExtendsExternal
                    | R::TsconfigExtendsCycle
                    | R::BaseurlEscapesRoot
                    | R::PathsMalformed
                    | R::RcChangedOnBranch
            ),
            Lang::Dart => matches!(
                self,
                R::PubspecNameMalformed
                    | R::PubspecNotDeclarative
                    | R::DuplicatePackageName
                    | R::RcChangedOnBranch
            ),
            Lang::Swift => matches!(
                self,
                R::ManifestNotLiteral
                    | R::DuplicateTargetName
                    | R::XcodeProjectUnsupported
                    | R::TargetDirMissing
                    | R::OverlappingTargets
                    | R::MixedObjcTarget
                    | R::NoPackageManifest
                    | R::RcChangedOnBranch
            ),
        }
    }
}

impl fmt::Display for LangUnclassifiable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LangUnclassifiable as R;
        f.write_str(match self {
            R::RcChangedOnBranch => "rc-changed-on-branch",
            R::TsconfigUnparseable => "tsconfig-unparseable",
            R::TsconfigExtendsExternal => "tsconfig-extends-external",
            R::TsconfigExtendsCycle => "tsconfig-extends-cycle",
            R::BaseurlEscapesRoot => "baseurl-escapes-root",
            R::PathsMalformed => "paths-malformed",
            R::PubspecNameMalformed => "pubspec-name-malformed",
            R::PubspecNotDeclarative => "pubspec-not-declarative",
            R::DuplicatePackageName => "duplicate-package-name",
            R::ManifestNotLiteral => "manifest-not-literal",
            R::DuplicateTargetName => "duplicate-target-name",
            R::XcodeProjectUnsupported => "xcode-project-unsupported",
            R::TargetDirMissing => "target-dir-missing",
            R::OverlappingTargets => "overlapping-targets",
            R::MixedObjcTarget => "mixed-objc-target",
            R::NoPackageManifest => "no-package-manifest",
        })
    }
}

impl core::error::Error for LangUnclassifiable {}

/// IR §3.8's **site** level: "one import site whose target cannot be
/// determined … disposition `unresolvable`; no edge; `unresolvable-import` if
/// in an `H`-true file."
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Unresolvable {
    /// Python §4.3, TypeScript §5.2. IR §5.2: "'Dynamic' is read as *the
    /// specifier is not statically determined*, not as *the syntax is named
    /// dynamic import*", so a literal `import('./x')` is **not** this.
    DynamicImport,
    /// A relative specifier whose lexical normalization escapes the root.
    RelativeEscapesRoot,
    /// Python §4.3: "a repository with both `a/b.py` and `a/b/__init__.py` is
    /// broken, and guessing which one the interpreter picks would be reading
    /// `sys.path` semantics the resolver has refused."
    AmbiguousModule,
    /// TypeScript §5.2 step 2 — "an absolute filesystem path is
    /// environment-dependent".
    AbsoluteSpecifier,
    /// TypeScript §5.2 step 3 — a `package.json` `imports` subpath.
    SubpathImports,
    /// TypeScript §5.2 step 5, Dart §6.2.
    NoCandidate,
    /// TypeScript §5.2 step 4: an alias matched and no substitution resolved.
    /// Case T15 pins that this is **not** `external`.
    AliasDeadEnd,
    /// Dart §6.2 — `file:`, `http:`, `asset:`.
    UnsupportedScheme,
    /// Dart §6.2 — `part of a.b.c` with zero or several `library a.b.c;`.
    AmbiguousLibraryName,
    /// §3.4 rule 5: a specifier that is not a *simple* literal.
    NonSimpleLiteral,
    /// IR §2.12 rule 2, cases C15/C16: mode `120000` or `160000`. **Must not**
    /// follow the link or descend into the submodule.
    SymlinkOrSubmodule,
}

impl Unresolvable {
    /// The closed per-language lists of §4.7, §5.7, §6.7 and §7.8. Swift's is
    /// one entry: "Swift has no string specifiers, no relative imports and no
    /// dynamic import, so a Swift `import` either names a target or is
    /// `external`" (IR §7.8).
    pub fn is_reachable_for(self, lang: Lang) -> bool {
        use Unresolvable as U;
        match lang {
            Lang::Python => matches!(
                self,
                U::DynamicImport
                    | U::RelativeEscapesRoot
                    | U::AmbiguousModule
                    | U::SymlinkOrSubmodule
            ),
            Lang::Ts => matches!(
                self,
                U::DynamicImport
                    | U::AbsoluteSpecifier
                    | U::SubpathImports
                    | U::RelativeEscapesRoot
                    | U::NoCandidate
                    | U::AliasDeadEnd
                    | U::SymlinkOrSubmodule
            ),
            Lang::Dart => matches!(
                self,
                U::UnsupportedScheme
                    | U::RelativeEscapesRoot
                    | U::NoCandidate
                    | U::AmbiguousLibraryName
                    | U::NonSimpleLiteral
                    | U::SymlinkOrSubmodule
            ),
            Lang::Swift => matches!(self, U::SymlinkOrSubmodule),
        }
    }
}

impl fmt::Display for Unresolvable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Unresolvable as U;
        f.write_str(match self {
            U::DynamicImport => "dynamic-import",
            U::RelativeEscapesRoot => "relative-escapes-root",
            U::AmbiguousModule => "ambiguous-module",
            U::AbsoluteSpecifier => "absolute-specifier",
            U::SubpathImports => "subpath-imports",
            U::NoCandidate => "no-candidate",
            U::AliasDeadEnd => "alias-dead-end",
            U::UnsupportedScheme => "unsupported-scheme",
            U::AmbiguousLibraryName => "ambiguous-library-name",
            U::NonSimpleLiteral => "non-simple-literal",
            U::SymlinkOrSubmodule => "symlink-or-submodule",
        })
    }
}

impl core::error::Error for Unresolvable {}

/// IR §3.8's **file** level, and the only member of it: "a file that cannot be
/// lexed (not UTF-8) … no edges from that file; `file-not-utf8`."
///
/// IR §3.4 rule 1: "No encoding declaration is honoured — not PEP 263's coding
/// cookie, not a BOM, not an XML declaration." Case C17 spells the failure this
/// forbids: "**must not** fall back to latin-1 or to a coding declaration."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileNotUtf8;

impl fmt::Display for FileNotUtf8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("file-not-utf8")
    }
}

impl core::error::Error for FileNotUtf8 {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ir_3_1_the_extension_table() {
        for (path, expected) in [
            ("a/b.py", Some(Lang::Python)),
            ("x.ts", Some(Lang::Ts)),
            ("x.tsx", Some(Lang::Ts)),
            ("x.mts", Some(Lang::Ts)),
            ("x.cts", Some(Lang::Ts)),
            ("x.js", Some(Lang::Ts)),
            ("x.jsx", Some(Lang::Ts)),
            ("x.mjs", Some(Lang::Ts)),
            ("x.cjs", Some(Lang::Ts)),
            ("x.dart", Some(Lang::Dart)),
            ("x.swift", Some(Lang::Swift)),
            ("x.d.ts", Some(Lang::Ts)),
            ("README.md", None),
            ("Makefile", None),
        ] {
            assert_eq!(lang(path), expected, "lang({path})");
        }
    }

    /// IR §3.1: "`.PY` is not Python; a repository that ships uppercase
    /// extensions gets `none`."
    #[test]
    fn an_uppercase_extension_is_lang_none_not_a_casefold() {
        assert_eq!(lang("a/B.PY"), None);
        assert_eq!(lang("a/B.Py"), None);
        assert_eq!(lang("Main.Swift"), None);
    }

    /// Case P15: "`.pyi` is `lang: none`; never a target, never lexed."
    /// Case T13 and IR §12.4.3: `.java`, `.m`, `.go`, `.vue`, `.svelte`.
    /// IR §8: "`.kt` and `.kts` are `lang: none`."
    #[test]
    fn the_named_none_extensions_are_none() {
        for path in [
            "a.pyi", "a.java", "a.m", "a.mm", "a.go", "a.vue", "a.svelte", "a.kt", "a.kts",
        ] {
            assert_eq!(lang(path), None, "lang({path})");
        }
    }

    /// A directory named `src.py` must not make everything under it Python:
    /// the table is matched on the **final path component** only.
    #[test]
    fn the_table_matches_the_final_component_and_not_the_path() {
        assert_eq!(lang("src.py/README"), None);
        assert_eq!(lang("a.ts/b.txt"), None);
        assert_eq!(lang("a.ts/b.ts"), Some(Lang::Ts));
    }

    #[test]
    fn declaration_files_are_ts_and_are_flagged() {
        assert!(is_declaration("types/x.d.ts"));
        assert!(is_declaration("x.d.mts"));
        assert!(is_declaration("x.d.cts"));
        assert!(!is_declaration("x.ts"));
        // A declaration file is still `lang: ts` — the override is on the
        // sites, not on the classification (IR §3.1).
        assert_eq!(lang("types/x.d.ts"), Some(Lang::Ts));
    }

    /// IR §11.1 and case S12's neighbour: the reserved token answers `None`, so
    /// a `params.langs` naming it cannot be turned into a `Lang` by accident.
    #[test]
    fn kotlin_is_reserved_and_maps_to_no_language() {
        assert_eq!(Lang::from_token("kotlin"), None);
        assert_eq!(RESERVED_LANG_TOKENS, ["kotlin"]);
        for token in RESERVED_LANG_TOKENS {
            assert_eq!(Lang::from_token(token), None, "{token} must stay unspent");
        }
    }

    #[test]
    fn the_four_tokens_round_trip_and_nothing_else_does() {
        for l in Lang::ALL {
            assert_eq!(Lang::from_token(l.token()), Some(l));
        }
        for token in ["", "Python", "javascript", "js", "objc", "gradle"] {
            assert_eq!(Lang::from_token(token), None, "{token}");
        }
    }

    /// IR §7.3 test 1's list, and case S14: the `.h` alone decides, with no
    /// rule reading the `-Bridging-Header` stem.
    #[test]
    fn every_c_family_extension_hits_and_a_bridging_header_needs_no_clause() {
        for ext in C_FAMILY_EXTENSIONS {
            assert!(is_c_family(&format!("Sources/T/Legacy{ext}")), "{ext}");
        }
        assert!(is_c_family(
            "Tests/BillingTests/BillingTests-Bridging-Header.h"
        ));
        assert!(!is_c_family("Sources/T/Legacy.swift"));
        // Lowercase only, exactly as §3.1's table is matched.
        assert!(!is_c_family("Sources/T/Legacy.M"));
    }

    /// IR §4.2: "Python therefore never raises `lang-unclassifiable`."
    #[test]
    fn python_has_no_language_level_unclassifiable_state() {
        for reason in [
            LangUnclassifiable::RcChangedOnBranch,
            LangUnclassifiable::MixedObjcTarget,
            LangUnclassifiable::PathsMalformed,
        ] {
            assert!(!reason.is_reachable_for(Lang::Python), "{reason}");
        }
    }

    /// IR §7.8: "Site level: `symlink-or-submodule`. That is the whole list."
    #[test]
    fn swifts_only_site_level_refusal_is_symlink_or_submodule() {
        for u in [
            Unresolvable::DynamicImport,
            Unresolvable::NoCandidate,
            Unresolvable::RelativeEscapesRoot,
            Unresolvable::AliasDeadEnd,
            Unresolvable::NonSimpleLiteral,
        ] {
            assert!(!u.is_reachable_for(Lang::Swift), "{u}");
        }
        assert!(Unresolvable::SymlinkOrSubmodule.is_reachable_for(Lang::Swift));
    }

    /// The tokens reach a reviewer's signed `wires=` and a `spine stats`
    /// counter, so a wrong spelling is a wrong signature.
    #[test]
    fn every_refusal_token_is_spelled_as_the_spec_fixes_it() {
        assert_eq!(LangUnclassifiable::MixedObjcTarget.to_string(), "mixed-objc-target");
        assert_eq!(
            LangUnclassifiable::RcChangedOnBranch.to_string(),
            "rc-changed-on-branch"
        );
        assert_eq!(LangUnclassifiable::NoPackageManifest.to_string(), "no-package-manifest");
        assert_eq!(Unresolvable::DynamicImport.to_string(), "dynamic-import");
        assert_eq!(Unresolvable::AliasDeadEnd.to_string(), "alias-dead-end");
        assert_eq!(FileNotUtf8.to_string(), "file-not-utf8");
    }
}
