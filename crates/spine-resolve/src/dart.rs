//! The Dart resolver — IR §6.
//!
//! Two things make Dart different from the other three. It has **no dynamic
//! import** — "`deferred as` is a lazy *load* of a statically named URI and is
//! an ordinary edge" — and it has **no type-only import**: "Every import is a
//! runtime import; §3.6 says none is recognized." So every recognized site here
//! either names a file in the tree or is `external`, and the only refusals are
//! §6.7's six.

use crate::lang::{Lang, LangUnclassifiable, Unresolvable};
use crate::lex::{self, Token, TokenKind};
use crate::site::{Disposition, ImportSite};
use crate::tree::{self, Tree};
use crate::yaml::{self, Yaml};

/// One package of `RC(dart, tree)` — IR §6.3: "`RC` is a set of packages, each
/// `(rootDir, name, pathDeps)`."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// "A `pubspec.yaml` at directory `d` declares a package rooted at `d`."
    pub root_dir: String,
    pub name: String,
    /// "`<pkg> → normalize(d + "/" + p)`", for each `<pkg>: { path: <p> }`
    /// entry of `dependencies:` and `dev_dependencies:`.
    pub path_deps: Vec<(String, String)>,
}

/// `RC(dart, tree)` — "Extracted from **every** `pubspec.yaml` in the tree."
///
/// Equality is structural, which is what IR §3.3 Rule 2 compares: "adding a
/// dependency to `pubspec.yaml` … changes nothing" unless it is a `path:` one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rc {
    /// Sorted by `root_dir` bytes, so the value is a function of the tree and
    /// not of the order the caller walked it (IR §15 rule 5).
    pub packages: Vec<Package>,
}

impl Rc {
    /// IR §6.3 step 5: "The importing file's package is the one whose `rootDir`
    /// is the **longest** prefix of the file's path. A Dart file under no
    /// package root → its `package:` self-references are `external` and its
    /// relative imports still resolve."
    pub fn package_of(&self, path: &str) -> Option<&Package> {
        self.packages
            .iter()
            .filter(|p| {
                p.root_dir.is_empty() || path.starts_with(&format!("{}/", p.root_dir))
            })
            .max_by_key(|p| p.root_dir.len())
    }
}

/// IR §6.3's extraction, with its four refusals.
pub fn rc(tree: &dyn Tree) -> Result<Rc, LangUnclassifiable> {
    let mut packages: Vec<Package> = Vec::new();
    let manifests: Vec<String> = tree
        .entries()
        .iter()
        .filter(|e| e.path == "pubspec.yaml" || e.path.ends_with("/pubspec.yaml"))
        .map(|e| e.path.clone())
        .collect();

    for manifest in manifests {
        let dir = tree::dirname(&manifest).to_string();
        let bytes = tree
            .read(&manifest)
            .ok_or(LangUnclassifiable::PubspecNotDeclarative)?;
        let text =
            core::str::from_utf8(bytes).map_err(|_| LangUnclassifiable::PubspecNotDeclarative)?;
        let doc = yaml::parse(text).ok_or(LangUnclassifiable::PubspecNotDeclarative)?;

        // 1. "Its `name:` must be a plain scalar matching `^[a-z_][a-z0-9_]*$`,
        //    else unclassifiable, reason `pubspec-name-malformed`."
        let name = doc
            .get("name")
            .and_then(Yaml::as_scalar)
            .ok_or(LangUnclassifiable::PubspecNameMalformed)?;
        if !valid_package_name(name) {
            return Err(LangUnclassifiable::PubspecNameMalformed);
        }

        // 3. "`pathDeps` is built from `dependencies:` and `dev_dependencies:`
        //    … provided the result stays inside the repository. A `path:`
        //    escaping the root, or a `git:`/`hosted:` dependency, contributes
        //    nothing and **is not an error**."
        let mut path_deps: Vec<(String, String)> = Vec::new();
        for section in ["dependencies", "dev_dependencies"] {
            let Some(Yaml::Map(entries)) = doc.get(section) else {
                continue;
            };
            for (pkg, spec) in entries {
                let Some(relative) = spec.get("path").and_then(Yaml::as_scalar) else {
                    continue;
                };
                let Some(resolved) = tree::normalize(&tree::join(&dir, relative)) else {
                    continue;
                };
                if !path_deps.iter().any(|(k, _)| k == pkg) {
                    path_deps.push((pkg.clone(), resolved));
                }
            }
        }

        packages.push(Package {
            root_dir: dir,
            name: name.to_string(),
            path_deps,
        });
    }

    // 4. "Two packages with the same `name` in one repository → unclassifiable,
    //    reason `duplicate-package-name`."
    let mut names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    names.sort_unstable();
    if names.windows(2).any(|w| w[0] == w[1]) {
        return Err(LangUnclassifiable::DuplicatePackageName);
    }

    packages.sort_by(|a, b| a.root_dir.cmp(&b.root_dir));
    Ok(Rc { packages })
}

/// IR §6.3 step 1's grammar for `name:`.
fn valid_package_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    match bytes.next() {
        Some(first) if first.is_ascii_lowercase() || first == b'_' => {
            bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        }
        _ => false,
    }
}

/// Every import site in a Dart file.
pub fn sites(source: &str, path: &str, tree: &dyn Tree, rc: &Rc) -> Vec<ImportSite> {
    let tokens = lex::without_comments(lex::lex(source, Lang::Dart));
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        // IR §6.1's anchor: "a `word` token `import`, `export` or `part`
        // **immediately followed by a `string` token**, or the sequence `part`
        // `of`. This needs no statement-boundary tracking."
        let is_directive_word = tokens[i].is_word(source, "import")
            || tokens[i].is_word(source, "export")
            || tokens[i].is_word(source, "part");
        if !is_directive_word {
            i += 1;
            continue;
        }
        let offset = tokens[i].start;

        // `part of` — either a URI or the legacy dotted library name.
        if tokens[i].is_word(source, "part")
            && tokens.get(i + 1).is_some_and(|t| t.is_word(source, "of"))
        {
            let (site, next) = part_of(source, path, tree, rc, &tokens, i + 2, offset);
            if let Some(site) = site {
                out.push(site);
            }
            i = next;
            continue;
        }

        if !tokens
            .get(i + 1)
            .is_some_and(|t| matches!(t.kind, TokenKind::Str(_)))
        {
            i += 1;
            continue;
        }

        // IR §3.7's union rule: "`import 'a' if (c) 'b' if (d) 'e';` | **one
        // site, all URIs**." Every branch contributes, and no environment
        // declaration is ever read — "the union is the *unique*
        // environment-independent over-approximation".
        let (uris, next) = directive_uris(source, &tokens, i + 1);
        out.push(ImportSite {
            offset,
            disposition: union_of(&uris, source, path, tree, rc),
        });
        i = next;
    }
    out.sort_by_key(|site| site.offset);
    out
}

/// Collect the string literals a directive names, stopping at its `;`.
///
/// Only literals at parenthesis depth zero count: a condition may itself carry
/// one — `if (const String.fromEnvironment('x') == 'y') 'b'` — and taking that
/// as a URI would name a file the directive never mentioned.
fn directive_uris<'a>(
    source: &str,
    tokens: &'a [Token],
    start: usize,
) -> (Vec<&'a Token>, usize) {
    let mut uris = Vec::new();
    let mut depth = 0i32;
    let mut i = start;
    while i < tokens.len() {
        let token = &tokens[i];
        if token.kind == TokenKind::Punct {
            match source.as_bytes()[token.start] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                b';' if depth <= 0 => {
                    i += 1;
                    break;
                }
                _ => {}
            }
        } else if matches!(token.kind, TokenKind::Str(_)) && depth == 0 {
            uris.push(token);
        }
        i += 1;
    }
    (uris, i)
}

/// One site, all URIs — the union rule applied to a directive's branches.
fn union_of(
    uris: &[&Token],
    source: &str,
    path: &str,
    tree: &dyn Tree,
    rc: &Rc,
) -> Disposition {
    let mut targets: Vec<String> = Vec::new();
    let mut external = false;
    for uri in uris {
        // IR §3.4 rule 5 and §6.7's site-level `non-simple-literal`.
        let Some(spec) = uri.simple_literal(source) else {
            return Disposition::Unresolvable(Unresolvable::NonSimpleLiteral);
        };
        match resolve(spec, path, tree, rc) {
            Disposition::Repo(found) => {
                for target in found {
                    if !targets.contains(&target) {
                        targets.push(target);
                    }
                }
            }
            Disposition::External => external = true,
            // A branch this resolver cannot read makes the whole site
            // unresolvable: the site has one disposition (IR §3.2), and the
            // fail-closed direction is to report the branch it could not read.
            other => return other,
        }
    }
    if targets.is_empty() {
        if external {
            Disposition::External
        } else {
            Disposition::Unresolvable(Unresolvable::NoCandidate)
        }
    } else {
        Disposition::Repo(targets)
    }
}

/// `part of 'uri';` and `part of a.b.c;`.
fn part_of(
    source: &str,
    path: &str,
    tree: &dyn Tree,
    rc: &Rc,
    tokens: &[Token],
    start: usize,
    offset: usize,
) -> (Option<ImportSite>, usize) {
    if tokens
        .get(start)
        .is_some_and(|t| matches!(t.kind, TokenKind::Str(_)))
    {
        let (uris, next) = directive_uris(source, tokens, start);
        return (
            Some(ImportSite {
                offset,
                disposition: union_of(&uris, source, path, tree, rc),
            }),
            next,
        );
    }
    // The legacy `part of a.b.c;` form. IR §6.2: "resolved through a
    // library-name index built over the tree being resolved against: every Dart
    // file whose directives contain `library <dotted name>;`. **Exactly one
    // match** → that file. Zero or more than one → `unresolvable`, reason
    // `ambiguous-library-name`."
    let mut name = String::new();
    let mut i = start;
    while let Some(token) = tokens.get(i) {
        match &token.kind {
            TokenKind::Word => {
                name.push_str(token.text(source));
                i += 1;
            }
            TokenKind::Punct if token.is_punct(source, b'.') => {
                name.push('.');
                i += 1;
            }
            _ => break,
        }
    }
    if tokens.get(i).is_some_and(|t| t.is_punct(source, b';')) {
        i += 1;
    }
    if name.is_empty() {
        return (None, i);
    }
    let matches = library_index(tree, &name);
    let disposition = match matches.as_slice() {
        [one] => Disposition::Repo(vec![one.clone()]),
        _ => Disposition::Unresolvable(Unresolvable::AmbiguousLibraryName),
    };
    (Some(ImportSite { offset, disposition }), i)
}

/// Every Dart file in the tree declaring `library <dotted name>;`.
fn library_index(tree: &dyn Tree, name: &str) -> Vec<String> {
    let mut found = Vec::new();
    for entry in tree.entries() {
        if crate::lang::lang(&entry.path) != Some(Lang::Dart) || !tree.is_file(&entry.path) {
            continue;
        }
        let Some(bytes) = tree.read(&entry.path) else {
            continue;
        };
        let Ok(text) = core::str::from_utf8(bytes) else {
            continue;
        };
        if declares_library(text, name) {
            found.push(entry.path.clone());
        }
    }
    found
}

/// `library <dotted name>;` in the token stream, comments and strings already
/// discarded by the lexer's own classification.
fn declares_library(source: &str, name: &str) -> bool {
    let tokens = lex::without_comments(lex::lex(source, Lang::Dart));
    let mut i = 0usize;
    while i < tokens.len() {
        if !tokens[i].is_word(source, "library") {
            i += 1;
            continue;
        }
        let mut declared = String::new();
        let mut k = i + 1;
        while let Some(token) = tokens.get(k) {
            match &token.kind {
                TokenKind::Word => {
                    declared.push_str(token.text(source));
                    k += 1;
                }
                TokenKind::Punct if token.is_punct(source, b'.') => {
                    declared.push('.');
                    k += 1;
                }
                _ => break,
            }
        }
        if declared == name && tokens.get(k).is_some_and(|t| t.is_punct(source, b';')) {
            return true;
        }
        i = k.max(i + 1);
    }
    false
}

/// IR §6.2's resolution table, by scheme.
///
/// "Dart requires the `.dart` extension in every URI, so there is **no
/// candidate expansion, no index resolution and no extension list**. A resolved
/// path that is not an existing file entry → `unresolvable`, reason
/// `no-candidate`." (Case D10.)
pub fn resolve(spec: &str, path: &str, tree: &dyn Tree, rc: &Rc) -> Disposition {
    if let Some(rest) = spec.strip_prefix("dart:") {
        let _ = rest;
        return Disposition::External;
    }
    if let Some(rest) = spec.strip_prefix("package:") {
        let Some((name, tail)) = rest.split_once('/') else {
            return Disposition::Unresolvable(Unresolvable::NoCandidate);
        };
        // "`package:<name>/<rest>` where `<name>` is `RC.selfName` | `lib/<rest>`,
        // relative to the package root directory."
        let own = rc.package_of(path);
        let base = if own.is_some_and(|p| p.name == name) {
            Some(tree::join(&own.unwrap().root_dir, "lib"))
        } else {
            // "`package:<name>/<rest>` where `<name>` is a key of `RC.pathDeps`
            // | `<RC.pathDeps[name]>/lib/<rest>`."
            own.and_then(|p| p.path_deps.iter().find(|(k, _)| k == name))
                .map(|(_, dir)| tree::join(dir, "lib"))
        };
        let Some(base) = base else {
            // "`package:<name>/…` otherwise | `external`."
            return Disposition::External;
        };
        return exists(&tree::join(&base, tail), tree);
    }
    // "any other scheme (`file:`, `http:`, `asset:`) | `unresolvable`, reason
    // `unsupported-scheme`."
    if let Some(colon) = spec.find(':')
        && spec[..colon]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'-' || b == b'.')
        && !spec[..colon].is_empty()
    {
        return Disposition::Unresolvable(Unresolvable::UnsupportedScheme);
    }
    // "no scheme (a relative URI) | lexically normalized against the importing
    // file's directory; escaping the repository root → `unresolvable`."
    let Some(resolved) = tree::normalize(&tree::join(tree::dirname(path), spec)) else {
        return Disposition::Unresolvable(Unresolvable::RelativeEscapesRoot);
    };
    exists(&resolved, tree)
}

fn exists(candidate: &str, tree: &dyn Tree) -> Disposition {
    if let Some(reason) = tree.refuses_to_follow(candidate) {
        return Disposition::Unresolvable(reason);
    }
    if tree.is_file(candidate) {
        Disposition::Repo(vec![candidate.to_string()])
    } else {
        Disposition::Unresolvable(Unresolvable::NoCandidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::MapTree;

    const SELF_PUBSPEC: &str = "name: self\nversion: 1.0.0\n";

    fn only(source: &str, path: &str, tree: &MapTree, rc: &Rc) -> Disposition {
        let found = sites(source, path, tree, rc);
        assert_eq!(found.len(), 1, "expected one site, got {found:?}");
        found.into_iter().next().unwrap().disposition
    }

    fn repo(path: &str) -> Disposition {
        Disposition::Repo(vec![path.to_string()])
    }

    /// Case D1: "`import 'package:self/x.dart'` where `pubspec.yaml` says
    /// `name: self` | resolves to `lib/x.dart`."
    #[test]
    fn d1_a_self_package_uri_resolves_under_lib() {
        let tree = MapTree::new([
            ("pubspec.yaml", SELF_PUBSPEC),
            ("lib/x.dart", ""),
            ("test/a_test.dart", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only(
                "import 'package:self/x.dart';\n",
                "test/a_test.dart",
                &tree,
                &rc
            ),
            repo("lib/x.dart")
        );
    }

    /// Case D2: "`import 'package:other/x.dart'` with no `path:` dependency on
    /// `other` | `external`."
    #[test]
    fn d2_a_package_uri_with_no_path_dependency_is_external() {
        let tree = MapTree::new([("pubspec.yaml", SELF_PUBSPEC), ("test/a_test.dart", "")]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only(
                "import 'package:other/x.dart';\n",
                "test/a_test.dart",
                &tree,
                &rc
            ),
            Disposition::External
        );
    }

    /// Case D3: "`import 'package:other/x.dart'` with `other: {path: ../other}`
    /// inside the repo | resolves to `../other/lib/x.dart`, normalized."
    #[test]
    fn d3_a_path_dependency_resolves_into_the_other_packages_lib() {
        let tree = MapTree::new([
            (
                "pkg/pubspec.yaml",
                "name: self\ndependencies:\n  other: {path: ../other}\n",
            ),
            ("other/pubspec.yaml", "name: other\n"),
            ("other/lib/x.dart", ""),
            ("pkg/test/a_test.dart", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only(
                "import 'package:other/x.dart';\n",
                "pkg/test/a_test.dart",
                &tree,
                &rc
            ),
            repo("other/lib/x.dart")
        );
    }

    /// Case D4: "`import 'a' if (dart.library.io) 'b';` | **both** URIs are
    /// sites." IR §3.7: "No configuration, flag set, target platform, build
    /// variant, Swift compilation condition or Dart environment declaration is
    /// ever read."
    #[test]
    fn d4_a_conditional_import_is_one_site_naming_every_branch() {
        let tree = MapTree::new([
            ("pubspec.yaml", SELF_PUBSPEC),
            ("lib/a.dart", ""),
            ("lib/b.dart", ""),
            ("lib/t.dart", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only(
                "import 'a.dart' if (dart.library.io) 'b.dart';\n",
                "lib/t.dart",
                &tree,
                &rc
            ),
            Disposition::Repo(vec!["lib/a.dart".into(), "lib/b.dart".into()])
        );
    }

    /// A string inside the condition is not a URI — only literals at
    /// parenthesis depth zero are branches.
    #[test]
    fn a_string_inside_a_conditional_condition_is_not_a_branch() {
        let tree = MapTree::new([
            ("pubspec.yaml", SELF_PUBSPEC),
            ("lib/a.dart", ""),
            ("lib/b.dart", ""),
            ("lib/t.dart", ""),
        ]);
        let rc = rc(&tree).unwrap();
        let source =
            "import 'a.dart' if (const String.fromEnvironment('k') == 'v') 'b.dart';\n";
        assert_eq!(
            only(source, "lib/t.dart", &tree, &rc),
            Disposition::Repo(vec!["lib/a.dart".into(), "lib/b.dart".into()])
        );
    }

    /// Cases D5 and D6: an `export` is an import site, and a `part` is walked.
    #[test]
    fn d5_and_d6_export_and_part_are_import_sites() {
        let tree = MapTree::new([
            ("pubspec.yaml", SELF_PUBSPEC),
            ("lib/x.dart", ""),
            ("lib/t.dart", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only("export 'x.dart' show Y;\n", "lib/t.dart", &tree, &rc),
            repo("lib/x.dart")
        );
        assert_eq!(
            only("part 'x.dart';\n", "lib/t.dart", &tree, &rc),
            repo("lib/x.dart")
        );
    }

    /// Case D7: "`part of 'x.dart';` | an import site naming `x.dart`."
    #[test]
    fn d7_part_of_a_uri_names_the_parent_library_file() {
        let tree = MapTree::new([
            ("pubspec.yaml", SELF_PUBSPEC),
            ("lib/x.dart", ""),
            ("lib/t.dart", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only("part of 'x.dart';\n", "lib/t.dart", &tree, &rc),
            repo("lib/x.dart")
        );
    }

    /// Cases D8 and D9: the legacy library-name form, and its refusal.
    #[test]
    fn d8_and_d9_the_library_name_index_must_be_single_valued() {
        let one = MapTree::new([
            ("pubspec.yaml", SELF_PUBSPEC),
            ("lib/x.dart", "library my.lib;\n"),
            ("lib/t.dart", ""),
        ]);
        let rc_one = rc(&one).unwrap();
        assert_eq!(
            only("part of my.lib;\n", "lib/t.dart", &one, &rc_one),
            repo("lib/x.dart")
        );

        let two = MapTree::new([
            ("pubspec.yaml", SELF_PUBSPEC),
            ("lib/x.dart", "library my.lib;\n"),
            ("lib/y.dart", "library my.lib;\n"),
            ("lib/t.dart", ""),
        ]);
        let rc_two = rc(&two).unwrap();
        assert_eq!(
            only("part of my.lib;\n", "lib/t.dart", &two, &rc_two),
            Disposition::Unresolvable(Unresolvable::AmbiguousLibraryName)
        );

        // Zero matches is the same refusal, by the same clause.
        let none = MapTree::new([("pubspec.yaml", SELF_PUBSPEC), ("lib/t.dart", "")]);
        let rc_none = rc(&none).unwrap();
        assert_eq!(
            only("part of my.lib;\n", "lib/t.dart", &none, &rc_none),
            Disposition::Unresolvable(Unresolvable::AmbiguousLibraryName)
        );
    }

    /// Case D10: "`import 'x';` (no `.dart`) where `x.dart` exists |
    /// `unresolvable`, reason `no-candidate` — **Dart does not append
    /// extensions**."
    #[test]
    fn d10_dart_appends_no_extension_and_a_missing_one_is_no_candidate() {
        let tree = MapTree::new([
            ("pubspec.yaml", SELF_PUBSPEC),
            ("lib/x.dart", ""),
            ("lib/t.dart", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only("import 'x';\n", "lib/t.dart", &tree, &rc),
            Disposition::Unresolvable(Unresolvable::NoCandidate)
        );
    }

    /// Case D11, end to end: a nested block comment does not hide the import
    /// after it.
    #[test]
    fn d11_a_nested_block_comment_does_not_hide_the_following_import() {
        let tree = MapTree::new([
            ("pubspec.yaml", SELF_PUBSPEC),
            ("lib/x.dart", ""),
            ("lib/t.dart", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only("/* /* */ */ import 'x.dart';\n", "lib/t.dart", &tree, &rc),
            repo("lib/x.dart")
        );
    }

    /// Case D12, and the other-scheme refusal beside it.
    #[test]
    fn d12_a_dart_scheme_is_external_and_another_scheme_is_refused() {
        let tree = MapTree::new([("pubspec.yaml", SELF_PUBSPEC), ("lib/t.dart", "")]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only("import 'dart:async';\n", "lib/t.dart", &tree, &rc),
            Disposition::External
        );
        for spec in ["file:///x.dart", "http://example.com/x.dart", "asset:a/x.dart"] {
            assert_eq!(
                only(&format!("import '{spec}';\n"), "lib/t.dart", &tree, &rc),
                Disposition::Unresolvable(Unresolvable::UnsupportedScheme),
                "{spec}"
            );
        }
    }

    /// Case D13: "`pubspec.yaml` using a YAML anchor | `lang-unclassifiable`,
    /// reason `pubspec-not-declarative`."
    #[test]
    fn d13_a_pubspec_outside_the_declarative_subset_is_unclassifiable() {
        let tree = MapTree::new([("pubspec.yaml", "name: self\nx: &a 1\ny: *a\n")]);
        assert_eq!(
            rc(&tree).unwrap_err(),
            LangUnclassifiable::PubspecNotDeclarative
        );
    }

    /// IR §6.3 steps 1 and 4.
    #[test]
    fn a_malformed_name_and_a_duplicate_name_are_each_their_own_reason() {
        let bad = MapTree::new([("pubspec.yaml", "name: Self\n")]);
        assert_eq!(
            rc(&bad).unwrap_err(),
            LangUnclassifiable::PubspecNameMalformed
        );

        let duplicate = MapTree::new([
            ("pubspec.yaml", "name: same\n"),
            ("sub/pubspec.yaml", "name: same\n"),
        ]);
        assert_eq!(
            rc(&duplicate).unwrap_err(),
            LangUnclassifiable::DuplicatePackageName
        );
    }

    /// IR §6.3 step 3: "A `path:` escaping the root, or a `git:`/`hosted:`
    /// dependency, contributes nothing and **is not an error**."
    #[test]
    fn an_escaping_or_hosted_dependency_contributes_nothing_and_is_not_an_error() {
        let tree = MapTree::new([(
            "pubspec.yaml",
            "name: self\ndependencies:\n  outside: {path: ../../elsewhere}\n  hosted_one: ^1.0.0\n  git_one:\n    git: https://example.com/x.git\n",
        )]);
        let rc = rc(&tree).expect("not an error");
        assert_eq!(rc.packages[0].path_deps, Vec::new());
    }

    /// IR §6.3 step 5: the longest `rootDir` prefix wins, and a file under no
    /// package root still resolves its relative imports.
    #[test]
    fn the_longest_package_root_prefix_owns_the_file() {
        let tree = MapTree::new([
            ("pubspec.yaml", "name: outer\n"),
            ("pkg/pubspec.yaml", "name: inner\n"),
            ("pkg/lib/x.dart", ""),
            ("pkg/lib/t.dart", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(rc.package_of("pkg/lib/t.dart").unwrap().name, "inner");
        assert_eq!(
            only(
                "import 'package:inner/x.dart';\n",
                "pkg/lib/t.dart",
                &tree,
                &rc
            ),
            repo("pkg/lib/x.dart")
        );
        // A relative import needs no package at all.
        assert_eq!(
            only("import 'x.dart';\n", "pkg/lib/t.dart", &tree, &rc),
            repo("pkg/lib/x.dart")
        );
    }

    /// IR §6.2: "**Dart has no dynamic import.** `deferred as` is a lazy *load*
    /// of a statically named URI and is an ordinary edge."
    #[test]
    fn a_deferred_import_is_an_ordinary_edge() {
        let tree = MapTree::new([
            ("pubspec.yaml", SELF_PUBSPEC),
            ("lib/x.dart", ""),
            ("lib/t.dart", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only("import 'x.dart' deferred as x;\n", "lib/t.dart", &tree, &rc),
            repo("lib/x.dart")
        );
    }

    /// IR §3.4 rule 5 and §6.7's site level: an interpolated specifier is
    /// `non-simple-literal`.
    #[test]
    fn an_interpolated_specifier_is_non_simple_literal() {
        let tree = MapTree::new([("pubspec.yaml", SELF_PUBSPEC), ("lib/t.dart", "")]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only("import 'package:$name/x.dart';\n", "lib/t.dart", &tree, &rc),
            Disposition::Unresolvable(Unresolvable::NonSimpleLiteral)
        );
    }

    /// IR §6.2: escaping the repository root is its own reason, never clamped.
    #[test]
    fn a_relative_uri_escaping_the_root_is_refused() {
        let tree = MapTree::new([("pubspec.yaml", SELF_PUBSPEC), ("lib/t.dart", "")]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only("import '../../x.dart';\n", "lib/t.dart", &tree, &rc),
            Disposition::Unresolvable(Unresolvable::RelativeEscapesRoot)
        );
    }
}
