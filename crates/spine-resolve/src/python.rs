//! The Python resolver — IR §4.
//!
//! IR §4.2: "`RC(python, ·)` is **empty**. Python's resolution roots are fixed
//! by rule (§4.3) and read no file. `pyproject.toml` is *not* consulted: its
//! `[tool.setuptools] package-dir`, `[tool.poetry] packages` and `[tool.hatch.build]
//! sources` keys would each be a different spelling of the same fact, they are
//! candidate-controlled, and reading them would put a TOML parser between two
//! implementations that must agree to the byte."
//!
//! So Python is the one language with no `RC`, no `lang-unclassifiable` state,
//! and no branch-controlled configuration in its resolution at all.

use crate::lang::{Lang, Unresolvable};
use crate::lex::{self, Token, TokenKind};
use crate::site::{Disposition, ImportSite};
use crate::tree::{self, Tree};

/// IR §4.3's roots, "in this order, evaluated against the tree being resolved
/// against":
///
/// 1. `""` — the repository root.
/// 2. `src/` — if and only if a tree entry `src` exists and is a directory.
///
/// "That is the whole list. It covers the flat layout and the src layout, which
/// is every layout a repository gated by spine can have, because pytest must be
/// able to import the package from the repository root without an installed
/// distribution."
pub fn roots(tree: &dyn Tree) -> Vec<&'static str> {
    if tree.is_dir("src") {
        vec!["", "src"]
    } else {
        vec![""]
    }
}

/// IR §4.3's dynamic-import token sequences, each a `word`/`.` run.
///
/// "The argument is not inspected even when it is a simple literal: deciding
/// that `importlib` is the standard library and not a local shim is a
/// name-binding question (§1)." Case P10: "**must not** resolve the literal."
const DYNAMIC_SEQUENCES: [&[&str]; 5] = [
    &["__import__"],
    &["importlib", ".", "import_module"],
    &["importlib", ".", "__import__"],
    &["importlib", ".", "util", ".", "spec_from_file_location"],
    &["imp", ".", "load_source"],
];

/// Every import site in `src`, the file being at repository path `path`.
///
/// Sites come back sorted by offset, which makes the output a function of the
/// file's bytes and of the tree alone (IR §15 rule 5).
pub fn sites(source: &str, path: &str, tree: &dyn Tree) -> Vec<ImportSite> {
    let tokens = lex::without_comments(lex::lex(source, Lang::Python));
    let mut out = Vec::new();
    out.extend(dynamic_sites(source, &tokens));
    for statement in statements(&tokens) {
        out.extend(statement_sites(source, path, tree, statement));
    }
    out.sort_by_key(|site| site.offset);
    out
}

/// IR §4.3's dynamic constructs, scanned over the whole token stream because
/// they are not anchored on a statement: "A file containing any of the token
/// sequences … has, **at each occurrence**, an import site with disposition
/// `unresolvable`."
fn dynamic_sites(source: &str, tokens: &[Token]) -> Vec<ImportSite> {
    let mut out = Vec::new();
    for start in 0..tokens.len() {
        for sequence in DYNAMIC_SEQUENCES {
            if sequence.len() > tokens.len() - start {
                continue;
            }
            let matched = sequence.iter().enumerate().all(|(k, want)| {
                let token = &tokens[start + k];
                if *want == "." {
                    token.is_punct(source, b'.')
                } else {
                    token.is_word(source, want)
                }
            });
            if matched {
                out.push(ImportSite {
                    offset: tokens[start].start,
                    disposition: Disposition::Unresolvable(Unresolvable::DynamicImport),
                });
                break;
            }
        }
    }
    out
}

/// Split the stream into logical statements. IR §4.1: "a logical line ends at a
/// newline that is not inside `(`/`[`/`{` and is not preceded by a backslash
/// continuation. `;` separates statements inside a logical line."
///
/// The lexer already emits `newline` only at bracket depth zero and only where
/// no continuation precedes, so the split here is on `newline` and on `;`.
fn statements(tokens: &[Token]) -> Vec<&[Token]> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (i, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Newline {
            if start < i {
                out.push(&tokens[start..i]);
            }
            start = i + 1;
        }
    }
    if start < tokens.len() {
        out.push(&tokens[start..]);
    }
    out
}

/// The anchor of IR §4.1: "an import site begins at a `word` token `import` or
/// `from` that is the **first token of a logical line or the first token after
/// a `;`**. Both are keywords, so no further disambiguation is needed."
///
/// The suite of a compound statement written on one line, if this clause is
/// one.
///
/// Returns the tokens after the header's `:` — the `:` at bracket depth 0 that
/// follows one of Python's compound keywords. Bracket depth is what keeps a
/// dict literal, a slice, an annotation and a `lambda` from reading as a
/// header; the keyword check is what keeps an annotated assignment
/// (`x: int = 1`) from doing the same.
fn compound_suite<'a>(source: &str, clause: &'a [Token]) -> Option<&'a [Token]> {
    const COMPOUND: [&str; 11] = [
        "if", "elif", "else", "for", "while", "try", "except", "finally", "with", "def", "class",
    ];
    let first = clause.first()?;
    let is_compound = COMPOUND.iter().any(|k| first.is_word(source, k))
        // `async for` / `async with` / `async def`.
        || (first.is_word(source, "async")
            && clause
                .get(1)
                .is_some_and(|t| ["for", "with", "def"].iter().any(|k| t.is_word(source, k))));
    if !is_compound {
        return None;
    }

    let mut depth = 0i32;
    for (i, token) in clause.iter().enumerate() {
        if token.is_punct(source, b'(') || token.is_punct(source, b'[') || token.is_punct(source, b'{') {
            depth += 1;
        } else if token.is_punct(source, b')') || token.is_punct(source, b']') || token.is_punct(source, b'}') {
            depth -= 1;
        } else if depth == 0 && token.is_punct(source, b':') {
            let suite = &clause[i + 1..];
            // A header with nothing after its `:` is the ordinary multi-line
            // form; its suite is on the following lines and is a statement in
            // its own right, already scanned.
            return (!suite.is_empty()).then_some(suite);
        }
    }
    None
}

/// **A compound statement's suite is scanned too, and §4.1's anchor does not
/// say so.** In `try: import oracle` / `if True: import oracle` the `import`
/// is neither the first token of its logical line nor the first after a `;`,
/// so the anchor as written draws no site and the module is never frozen.
///
/// This was left as a reported defect and is now closed in §3.7's direction,
/// because §3.7 settles it and does so by name:
///
/// > Every one of the four languages has at least one construct whose active
/// > branch is chosen by something outside the tree — Swift's `#if`, Dart's
/// > `import … if (…)`, TypeScript's environment-dependent `exports`
/// > conditions, **Python's `try: import … except ImportError:`**.
/// >
/// > **The rule is the union.** Every branch of every conditional construct
/// > contributes its import sites, and all of them are resolved. … dropping a
/// > branch is how an [oracle hides].
///
/// So §4.1's anchor is a lexical rule written for the ordinary case that does
/// not reach the compound form, while §3.7 states the requirement the closure
/// must meet. Between a section that names the construct and a section that
/// merely fails to reach it, the one that names it governs — and the direction
/// matters: the anchor's reading leaves an oracle a one-line hiding place in
/// the freeze closure, which is the failure that cost Kotlin its place in v1.
///
/// **This moves `closure_digest` for a repository containing one**, and that
/// is filed in `.build-notes/OPEN-questions.md`: §4.1's anchor needs the
/// matching clause so two implementations reading it alone agree with two
/// reading §3.7.
fn statement_sites(
    source: &str,
    path: &str,
    tree: &dyn Tree,
    statement: &[Token],
) -> Vec<ImportSite> {
    let mut out = Vec::new();
    for clause in statement.split(|t| t.is_punct(source, b';')) {
        let Some(first) = clause.first() else {
            continue;
        };
        // A compound statement header — `if …:`, `try:`, `except X:`, `for …:`
        // — whose suite is on the same line. Everything after the header's `:`
        // is a statement list in its own right, so it is scanned as one.
        if let Some(suite) = compound_suite(source, clause) {
            out.extend(statement_sites(source, path, tree, suite));
            continue;
        }
        if first.is_word(source, "import") {
            out.extend(parse_plain_import(source, tree, clause));
        } else if first.is_word(source, "from") {
            out.extend(parse_from_import(source, path, tree, clause));
        }
    }
    out
}

/// `import a.b.c`, `import a.b.c as d`, `import a.b, c.d`.
///
/// IR §4.3: "`import a.b, c.d` | dotted resolution of each, **as separate
/// sites**."
fn parse_plain_import(source: &str, tree: &dyn Tree, clause: &[Token]) -> Vec<ImportSite> {
    let mut out = Vec::new();
    let mut i = 1usize;
    let mut first_clause = true;
    loop {
        let (parts, next) = take_dotted(source, clause, i);
        if parts.is_empty() {
            break;
        }
        // IR §3.2 identifies a site by "the byte offset of the **first
        // token**". A statement naming several modules yields several sites,
        // which cannot all carry the keyword's offset, so the first takes the
        // keyword's and each later one takes its own name's. DERIVED: §3.2
        // fixes that offsets exist and are stable, not how a multi-name
        // statement apportions them.
        let offset = if first_clause {
            clause[0].start
        } else {
            clause[i].start
        };
        first_clause = false;
        i = next;
        // `as d` binds a name and names no further module.
        if clause.get(i).is_some_and(|t| t.is_word(source, "as")) {
            i += 2;
        }
        out.push(ImportSite {
            offset,
            disposition: dotted(tree, &roots(tree), &parts),
        });
        if clause.get(i).is_some_and(|t| t.is_punct(source, b',')) {
            i += 1;
            continue;
        }
        break;
    }
    out
}

/// The `from` forms, absolute and package-relative. IR §4.3: "`from a.b import
/// c, d` | the union of the `from` rule applied to each name; **one site**."
fn parse_from_import(
    source: &str,
    path: &str,
    tree: &dyn Tree,
    clause: &[Token],
) -> Vec<ImportSite> {
    let offset = clause[0].start;
    let mut i = 1usize;
    // Leading dots give the level. `...` lexes as three `.` punct tokens, so
    // counting tokens is counting dots (case P7).
    let mut level = 0usize;
    while clause.get(i).is_some_and(|t| t.is_punct(source, b'.')) {
        level += 1;
        i += 1;
    }
    // `import` is a reserved word (§3.4 rule 6), so it can never be a module
    // component; without this guard `from . import x` reads the keyword as the
    // module name and the whole relative form yields no site at all.
    let (module, next) = if clause.get(i).is_some_and(|t| t.is_word(source, "import")) {
        (Vec::new(), i)
    } else {
        take_dotted(source, clause, i)
    };
    i = next;
    if !clause.get(i).is_some_and(|t| t.is_word(source, "import")) {
        return Vec::new();
    }
    i += 1;

    // `from a.b import *` names the package and no submodule.
    let star = clause.get(i).is_some_and(|t| t.is_punct(source, b'*'));
    let names = if star {
        Vec::new()
    } else {
        take_import_names(source, clause, i)
    };

    let site = |disposition| {
        vec![ImportSite {
            offset,
            disposition,
        }]
    };

    // The roots the remaining dotted name is resolved against.
    let search_roots: Vec<String> = if level == 0 {
        roots(tree).into_iter().map(String::from).collect()
    } else {
        // IR §4.3: "Let `d` be the directory containing the importing file. For
        // level `L`, the base directory is `d` with `L − 1` components removed.
        // Note that this is the same for `p/q/mod.py` and `p/q/__init__.py`:
        // Python gives both the package `p.q`, whose directory is `p/q`."
        let dir = tree::dirname(path);
        let mut components: Vec<&str> = if dir.is_empty() {
            Vec::new()
        } else {
            dir.split('/').collect()
        };
        for _ in 0..level - 1 {
            // "If the base directory would escape the repository root, the site
            // is `unresolvable` (reason `relative-escapes-root`)." (Case P7.)
            if components.pop().is_none() {
                return site(Disposition::Unresolvable(Unresolvable::RelativeEscapesRoot));
            }
        }
        vec![components.join("/")]
    };
    let root_refs: Vec<&str> = search_roots.iter().map(String::as_str).collect();

    let mut targets: Vec<String> = Vec::new();

    // "add the existing `__init__.py` of the base directory … as targets" — the
    // relative form always executes its own package.
    if level > 0 {
        let base_init = tree::join(&search_roots[0], "__init__.py");
        match probe(tree, &base_init) {
            Err(reason) => return site(Disposition::Unresolvable(reason)),
            Ok(true) => targets.push(base_init),
            Ok(false) => {}
        }
    }

    // "`from a.b import c` | first, dotted resolution of `a.b.c` (`c` may be a
    // submodule); if that yields nothing, dotted resolution of `a.b` alone (`c`
    // is an attribute). If neither resolves, `external`."
    let candidates: Vec<Vec<String>> = if star || names.is_empty() {
        vec![module.clone()]
    } else {
        names
            .iter()
            .map(|name| {
                let mut parts = module.clone();
                parts.push(name.clone());
                parts
            })
            .collect()
    };

    for parts in candidates {
        match dotted(tree, &root_refs, &parts) {
            Disposition::Unresolvable(reason) => {
                return site(Disposition::Unresolvable(reason));
            }
            Disposition::Repo(found) => extend_unique(&mut targets, found),
            _ => {
                // The submodule reading yielded nothing; fall back to the
                // package itself, where there is one.
                if !module.is_empty() {
                    match dotted(tree, &root_refs, &module) {
                        Disposition::Unresolvable(reason) => {
                            return site(Disposition::Unresolvable(reason));
                        }
                        Disposition::Repo(found) => extend_unique(&mut targets, found),
                        _ => {}
                    }
                }
            }
        }
    }

    if targets.is_empty() {
        site(Disposition::External)
    } else {
        site(Disposition::Repo(targets))
    }
}

/// IR §4.3's dotted resolution.
///
/// "For each root `r` in order, form the candidates `r + n₁/…/n_k + ".py"` and
/// `r + n₁/…/n_k + "/__init__.py"`. **The first root for which at least one
/// candidate exists wins.** If **both** candidates exist under the winning
/// root, the site is `unresolvable` (reason `ambiguous-module`)."
///
/// "**Ancestor packages are part of the edge.** Importing `a.b.c` executes
/// `a/__init__.py` and `a/b/__init__.py` before `a/b/c.py`. … A missing
/// intermediate `__init__.py` is a namespace package (PEP 420) and is simply
/// not a target — it is not an error."
fn dotted(tree: &dyn Tree, roots: &[&str], parts: &[String]) -> Disposition {
    if parts.is_empty() {
        return Disposition::External;
    }
    let joined = parts.join("/");
    for root in roots {
        let base = tree::join(root, &joined);
        let module = format!("{base}.py");
        let package = format!("{base}/__init__.py");
        let has_module = match probe(tree, &module) {
            Err(reason) => return Disposition::Unresolvable(reason),
            Ok(found) => found,
        };
        let has_package = match probe(tree, &package) {
            Err(reason) => return Disposition::Unresolvable(reason),
            Ok(found) => found,
        };
        if has_module && has_package {
            // "a repository with both `a/b.py` and `a/b/__init__.py` is broken,
            // and guessing which one the interpreter picks would be reading
            // `sys.path` semantics the resolver has refused."
            return Disposition::Unresolvable(Unresolvable::AmbiguousModule);
        }
        if !has_module && !has_package {
            continue;
        }
        // Ancestors first, in execution order — `a/__init__.py`, then
        // `a/b/__init__.py`, then the module itself.
        let mut targets = Vec::new();
        for j in 1..parts.len() {
            let ancestor = format!("{}/__init__.py", tree::join(root, &parts[..j].join("/")));
            match probe(tree, &ancestor) {
                Err(reason) => return Disposition::Unresolvable(reason),
                Ok(true) => targets.push(ancestor),
                Ok(false) => {}
            }
        }
        targets.push(if has_module { module } else { package });
        return Disposition::Repo(targets);
    }
    // IR §3.2: a name that matches no tree entry "cannot be hiding an oracle —
    // it is a dependency, an SDK module, or generated code that does not exist
    // in the tree."
    Disposition::External
}

/// Does this candidate exist as a file, and may the resolver follow it?
///
/// IR §4.7 item 4: "a resolved candidate whose tree entry is a symlink or
/// submodule (§2.12 rule 2)" is site-level `unresolvable`.
fn probe(tree: &dyn Tree, path: &str) -> Result<bool, Unresolvable> {
    if let Some(reason) = tree.refuses_to_follow(path) {
        return Err(reason);
    }
    Ok(tree.is_file(path))
}

fn extend_unique(targets: &mut Vec<String>, found: Vec<String>) {
    for path in found {
        if !targets.contains(&path) {
            targets.push(path);
        }
    }
}

/// Read a dotted name — `word` (`.` `word`)* — starting at `i`. Returns the
/// components and the index one past the name.
fn take_dotted(source: &str, clause: &[Token], mut i: usize) -> (Vec<String>, usize) {
    let mut parts = Vec::new();
    while let Some(token) = clause.get(i) {
        if token.kind != TokenKind::Word {
            break;
        }
        parts.push(token.text(source).to_string());
        i += 1;
        match clause.get(i) {
            Some(dot) if dot.is_punct(source, b'.') => {
                // A trailing `.` with no name after it ends the run.
                if clause.get(i + 1).map(|t| &t.kind) == Some(&TokenKind::Word) {
                    i += 1;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    (parts, i)
}

/// The imported-name list of a `from … import …`, parenthesized or not.
///
/// IR §4.3: "`from a import (b, c)` (parenthesized, possibly multi-line) | as
/// `from a import b, c`; the logical-line rule of §4.1 makes it one site."
fn take_import_names(source: &str, clause: &[Token], mut i: usize) -> Vec<String> {
    if clause.get(i).is_some_and(|t| t.is_punct(source, b'(')) {
        i += 1;
    }
    let mut names = Vec::new();
    while let Some(token) = clause.get(i) {
        match &token.kind {
            TokenKind::Word => {
                names.push(token.text(source).to_string());
                i += 1;
                if clause.get(i).is_some_and(|t| t.is_word(source, "as")) {
                    i += 2;
                }
            }
            TokenKind::Punct => {
                // `,` continues the list; `)` and anything else ends it.
                if token.is_punct(source, b',') {
                    i += 1;
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{EntryKind, MapTree};

    fn only(source: &str, path: &str, tree: &MapTree) -> Disposition {
        let found = sites(source, path, tree);
        assert_eq!(found.len(), 1, "expected one site, got {found:?}");
        found.into_iter().next().unwrap().disposition
    }

    /// Case P1: "`from a.b import c` where both `a/b/c.py` and `a/b/__init__.py`
    /// exist | targets are both, plus `a/__init__.py` if present."
    #[test]
    fn p1_a_submodule_import_takes_the_module_and_every_ancestor_package() {
        let tree = MapTree::new([
            ("a/__init__.py", ""),
            ("a/b/__init__.py", ""),
            ("a/b/c.py", ""),
            ("t.py", ""),
        ]);
        assert_eq!(
            only("from a.b import c\n", "t.py", &tree),
            Disposition::Repo(vec![
                "a/__init__.py".into(),
                "a/b/__init__.py".into(),
                "a/b/c.py".into(),
            ])
        );
    }

    /// Case P2: "`from a.b import c` where `a/b/c.py` does not exist but `a/b.py`
    /// does | target is `a/b.py`; `c` is an attribute."
    #[test]
    fn p2_a_name_that_is_not_a_submodule_falls_back_to_the_package() {
        let tree = MapTree::new([("a/__init__.py", ""), ("a/b.py", ""), ("t.py", "")]);
        assert_eq!(
            only("from a.b import c\n", "t.py", &tree),
            Disposition::Repo(vec!["a/__init__.py".into(), "a/b.py".into()])
        );
    }

    /// Case P3: "`import a.b.c` | targets `a/__init__.py`, `a/b/__init__.py`,
    /// `a/b/c.py` — every existing ancestor package, **must not** be only the
    /// leaf."
    #[test]
    fn p3_every_existing_ancestor_package_is_a_target_and_not_only_the_leaf() {
        let tree = MapTree::new([
            ("a/__init__.py", ""),
            ("a/b/__init__.py", ""),
            ("a/b/c.py", ""),
            ("t.py", ""),
        ]);
        assert_eq!(
            only("import a.b.c\n", "t.py", &tree),
            Disposition::Repo(vec![
                "a/__init__.py".into(),
                "a/b/__init__.py".into(),
                "a/b/c.py".into(),
            ])
        );
    }

    /// IR §4.3: "A missing intermediate `__init__.py` is a namespace package
    /// (PEP 420) and is simply not a target — it is not an error."
    #[test]
    fn a_namespace_package_contributes_no_target_and_no_error() {
        let tree = MapTree::new([("a/b/c.py", ""), ("t.py", "")]);
        assert_eq!(
            only("import a.b.c\n", "t.py", &tree),
            Disposition::Repo(vec!["a/b/c.py".into()])
        );
    }

    /// Case P4: "`a/b.py` and `a/b/__init__.py` both exist, `import a.b` |
    /// `unresolvable`, reason `ambiguous-module`."
    #[test]
    fn p4_a_module_and_a_package_of_one_name_is_ambiguous_module() {
        let tree = MapTree::new([("a/b.py", ""), ("a/b/__init__.py", ""), ("t.py", "")]);
        assert_eq!(
            only("import a.b\n", "t.py", &tree),
            Disposition::Unresolvable(Unresolvable::AmbiguousModule)
        );
    }

    /// Case P5: "`src/pkg/mod.py` exists, no top-level `pkg/`, `import pkg.mod` |
    /// resolves under root 2 (`src/`)."
    #[test]
    fn p5_the_src_layout_resolves_under_the_second_root() {
        let tree = MapTree::new([("src/pkg/mod.py", ""), ("t.py", "")]);
        assert_eq!(roots(&tree), ["", "src"]);
        assert_eq!(
            only("import pkg.mod\n", "t.py", &tree),
            Disposition::Repo(vec!["src/pkg/mod.py".into()])
        );
    }

    /// IR §4.3: root 2 exists "if and only if a tree entry `src` exists and is a
    /// directory".
    #[test]
    fn without_a_src_directory_there_is_one_root() {
        let tree = MapTree::new([("pkg/mod.py", "")]);
        assert_eq!(roots(&tree), [""]);
    }

    /// Case P6: "Both `pkg/mod.py` and `src/pkg/mod.py` exist, `import pkg.mod` |
    /// root 1 wins; `src/pkg/mod.py` is not a target."
    #[test]
    fn p6_the_first_root_that_has_a_candidate_wins_outright() {
        let tree = MapTree::new([("pkg/mod.py", ""), ("src/pkg/mod.py", ""), ("t.py", "")]);
        assert_eq!(
            only("import pkg.mod\n", "t.py", &tree),
            Disposition::Repo(vec!["pkg/mod.py".into()])
        );
    }

    /// Case P7: "`from ... import x` in `a/b/c.py` | base directory `` (root);
    /// resolves. In `a/c.py` it escapes → `unresolvable`."
    #[test]
    fn p7_three_dots_reach_the_root_from_a_slash_b_and_escape_from_a() {
        let tree = MapTree::new([
            ("a/__init__.py", ""),
            ("a/b/__init__.py", ""),
            ("a/b/c.py", ""),
            ("a/c.py", ""),
            ("x.py", ""),
        ]);
        assert_eq!(
            only("from ... import x\n", "a/b/c.py", &tree),
            Disposition::Repo(vec!["x.py".into()])
        );
        assert_eq!(
            only("from ... import x\n", "a/c.py", &tree),
            Disposition::Unresolvable(Unresolvable::RelativeEscapesRoot)
        );
    }

    /// IR §4.3: the base directory "is the same for `p/q/mod.py` and
    /// `p/q/__init__.py`: Python gives both the package `p.q`."
    #[test]
    fn a_packages_own_init_resolves_relatives_from_its_directory_like_a_sibling() {
        let tree = MapTree::new([
            ("p/q/__init__.py", ""),
            ("p/q/mod.py", ""),
            ("p/q/helper.py", ""),
        ]);
        let expected = Disposition::Repo(vec!["p/q/__init__.py".into(), "p/q/helper.py".into()]);
        assert_eq!(only("from . import helper\n", "p/q/mod.py", &tree), expected);
        assert_eq!(
            only("from . import helper\n", "p/q/__init__.py", &tree),
            expected
        );
    }

    /// IR §4.3's relative forms: level 2, with the intermediate `__init__.py`
    /// files as targets.
    #[test]
    fn a_level_two_relative_import_resolves_against_the_parent_package() {
        let tree = MapTree::new([
            ("p/__init__.py", ""),
            ("p/a/__init__.py", ""),
            ("p/q/__init__.py", ""),
            ("p/q/mod.py", ""),
            ("p/a/b.py", ""),
        ]);
        assert_eq!(
            only("from ..a import b\n", "p/q/mod.py", &tree),
            Disposition::Repo(vec![
                "p/__init__.py".into(),
                "p/a/__init__.py".into(),
                "p/a/b.py".into(),
            ])
        );
    }

    /// Case P8: "An import nested inside a function body | an import site;
    /// **must not** be ignored."
    #[test]
    fn p8_an_import_inside_a_function_body_is_a_site() {
        let tree = MapTree::new([("a.py", ""), ("t.py", "")]);
        let source = "def f():\n    import a\n    return a\n";
        assert_eq!(
            only(source, "t.py", &tree),
            Disposition::Repo(vec!["a.py".into()])
        );
    }

    /// Case P9: "An import under `if TYPE_CHECKING:` | an ordinary import site
    /// (§3.6)." IR §3.6: recognizing it "requires deciding that `TYPE_CHECKING`
    /// is `typing.TYPE_CHECKING` and not a module-level `TYPE_CHECKING = True`,
    /// which is a name-binding question the resolver refuses to answer."
    #[test]
    fn p9_an_import_under_if_type_checking_is_an_ordinary_site() {
        let tree = MapTree::new([("a.py", ""), ("t.py", "")]);
        let source = "if TYPE_CHECKING:\n    import a\n";
        assert_eq!(
            only(source, "t.py", &tree),
            Disposition::Repo(vec!["a.py".into()])
        );
    }

    /// Case P10: "`importlib.import_module(\"a.b\")` with a literal argument |
    /// `unresolvable`, reason `dynamic-import`; **must not** resolve the
    /// literal."
    #[test]
    fn p10_every_dynamic_construct_is_unresolvable_and_its_argument_is_never_read() {
        let tree = MapTree::new([("a/b.py", ""), ("a/__init__.py", ""), ("t.py", "")]);
        for source in [
            "importlib.import_module(\"a.b\")\n",
            "__import__(\"a.b\")\n",
            "importlib.__import__(\"a.b\")\n",
            "importlib.util.spec_from_file_location(\"a.b\", p)\n",
            "imp.load_source(\"a.b\", p)\n",
        ] {
            let found = sites(source, "t.py", &tree);
            assert!(
                found
                    .iter()
                    .any(|s| s.unresolvable() == Some(Unresolvable::DynamicImport)),
                "{source}"
            );
            assert!(
                found.iter().all(|s| s.targets().is_empty()),
                "{source} must resolve no literal"
            );
        }
    }

    /// Case P11: "`import os; import a.b` on one line | two sites."
    #[test]
    fn p11_a_semicolon_separates_two_statements_and_yields_two_sites() {
        let tree = MapTree::new([("a/b.py", ""), ("t.py", "")]);
        let found = sites("import os; import a.b\n", "t.py", &tree);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].disposition, Disposition::External);
        assert_eq!(
            found[1].disposition,
            Disposition::Repo(vec!["a/b.py".into()])
        );
    }

    /// IR §4.3: "`import a.b, c.d` | dotted resolution of each, **as separate
    /// sites**" — and the two sites must be distinguishable, which is what §3.2
    /// makes offsets for.
    #[test]
    fn one_import_statement_naming_two_modules_yields_two_located_sites() {
        let tree = MapTree::new([("a/b.py", ""), ("c/d.py", ""), ("t.py", "")]);
        let found = sites("import a.b, c.d\n", "t.py", &tree);
        assert_eq!(found.len(), 2);
        assert_ne!(found[0].offset, found[1].offset);
        assert_eq!(
            found[0].disposition,
            Disposition::Repo(vec!["a/b.py".into()])
        );
        assert_eq!(
            found[1].disposition,
            Disposition::Repo(vec!["c/d.py".into()])
        );
    }

    /// Case P12: "`from a import (\\n  b,\\n  c,\\n)` | **one** site; the
    /// logical-line rule spans the parentheses."
    #[test]
    fn p12_a_parenthesized_multi_line_name_list_is_one_site() {
        let tree = MapTree::new([
            ("a/__init__.py", ""),
            ("a/b.py", ""),
            ("a/c.py", ""),
            ("t.py", ""),
        ]);
        assert_eq!(
            only("from a import (\n  b,\n  c,\n)\n", "t.py", &tree),
            Disposition::Repo(vec![
                "a/__init__.py".into(),
                "a/b.py".into(),
                "a/c.py".into(),
            ])
        );
    }

    /// Cases P13 and P14: a comment and a string carry no site.
    #[test]
    fn p13_and_p14_a_commented_or_quoted_import_is_no_site() {
        let tree = MapTree::new([("a/b.py", ""), ("t.py", "")]);
        assert!(sites("# import a.b\n", "t.py", &tree).is_empty());
        assert!(sites("x = \"import a.b\"\n", "t.py", &tree).is_empty());
    }

    /// Case P15: "A `.pyi` file next to a `.py` | `.pyi` is `lang: none`; never
    /// a target, never lexed." The resolver only ever forms `.py` candidates,
    /// so a `.pyi` cannot become one.
    #[test]
    fn p15_a_pyi_stub_is_never_a_candidate() {
        let tree = MapTree::new([("a.pyi", ""), ("t.py", "")]);
        assert_eq!(only("import a\n", "t.py", &tree), Disposition::External);
    }

    /// Cases C15 and C16, at the Python candidate.
    #[test]
    fn c15_and_c16_a_symlink_or_submodule_candidate_is_unresolvable_and_never_followed() {
        let tree = MapTree::new([("t.py", "")]).with_special("a.py", EntryKind::Symlink);
        assert_eq!(
            only("import a\n", "t.py", &tree),
            Disposition::Unresolvable(Unresolvable::SymlinkOrSubmodule)
        );

        let tree = MapTree::new([("t.py", "")]).with_special("vendor", EntryKind::Submodule);
        assert_eq!(
            only("import vendor.pkg\n", "t.py", &tree),
            Disposition::Unresolvable(Unresolvable::SymlinkOrSubmodule)
        );
    }

    /// IR §4.3: "`from a.b import *` | dotted resolution of `a.b`."
    #[test]
    fn a_star_import_names_the_package_and_no_submodule() {
        let tree = MapTree::new([
            ("a/__init__.py", ""),
            ("a/b/__init__.py", ""),
            ("a/b/c.py", ""),
            ("t.py", ""),
        ]);
        assert_eq!(
            only("from a.b import *\n", "t.py", &tree),
            Disposition::Repo(vec!["a/__init__.py".into(), "a/b/__init__.py".into()])
        );
    }

    /// IR §3.2: a bare name matching no tree entry is `external`, not a
    /// tripwire.
    #[test]
    fn a_name_that_matches_no_tree_entry_is_external() {
        let tree = MapTree::new([("t.py", "")]);
        assert_eq!(only("import os\n", "t.py", &tree), Disposition::External);
        assert_eq!(
            only("from pytest import fixture\n", "t.py", &tree),
            Disposition::External
        );
    }

    /// IR §4.7: "Python has no language-level unclassifiable state" and no `RC`
    /// — the roots are a function of the tree's shape alone.
    #[test]
    fn the_python_roots_read_no_file() {
        let tree = MapTree::new([
            ("pyproject.toml", "[tool.setuptools]\npackage-dir = {\"\" = \"lib\"}\n"),
            ("lib/pkg/mod.py", ""),
            ("t.py", ""),
        ]);
        // `lib/` is not a root, however `pyproject.toml` spells it.
        assert_eq!(roots(&tree), [""]);
        assert_eq!(only("import pkg.mod\n", "t.py", &tree), Disposition::External);
    }

    /// IR §3.7 names `try: import … except ImportError:` as a conditional
    /// construct whose every branch contributes, and gives the reason:
    /// "dropping a branch is how an [oracle hides]". §4.1's anchor does not
    /// reach the one-line form; §3.7 governs, and this is the test of it.
    #[test]
    fn a_compound_one_liner_draws_its_import_site() {
        let tree = MapTree::new([("oracle.py", ""), ("a.py", "")]);
        for source in [
            "try: import oracle\nexcept ImportError: pass\n",
            "if True: import oracle\n",
            "for x in y: import oracle\n",
            "while True: import oracle\n",
            "with open(f): import oracle\n",
            "else: import oracle\n",
            "async def f(): import oracle\n",
        ] {
            let found = sites(source, "a.py", &tree);
            assert_eq!(
                found.len(),
                1,
                "{source:?} must draw a site — an oracle must have nowhere to hide"
            );
            assert_eq!(
                found[0].disposition,
                Disposition::Repo(vec!["oracle.py".to_string()]),
                "{source:?}"
            );
        }
    }

    /// And the shapes a `:` appears in that are NOT compound headers still draw
    /// nothing, so the widening did not turn punctuation into an anchor.
    #[test]
    fn a_colon_that_does_not_open_a_suite_is_not_an_anchor() {
        let tree = MapTree::new([("oracle.py", ""), ("a.py", "")]);
        for source in [
            // An annotated assignment.
            "x: int = 1\n",
            // A dict literal whose value mentions the word.
            "d = {'import': 'oracle'}\n",
            // A slice.
            "s = xs[1:2]\n",
            // A lambda.
            "f = lambda x: x\n",
        ] {
            assert!(
                sites(source, "a.py", &tree).is_empty(),
                "{source:?} draws no site"
            );
        }
    }

    /// A header with nothing after its `:` is the ordinary multi-line form and
    /// is unaffected — its suite is already a statement of its own.
    #[test]
    fn a_multi_line_compound_statement_is_unchanged() {
        let tree = MapTree::new([("oracle.py", ""), ("a.py", "")]);
        let found = sites("try:\n    import oracle\nexcept ImportError:\n    pass\n", "a.py", &tree);
        assert_eq!(found.len(), 1);
    }
}
