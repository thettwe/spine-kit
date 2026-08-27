//! The Swift resolver — IR §7.
//!
//! Swift is the language PB §6.7's rule nearly removed. IR §7.3 states the
//! decision in one sentence: "**a guarantee that fails loudly can ship with its
//! limits stated, one that fails silently cannot.**" Kotlin was dropped because
//! "an oracle in a `.java` file inside a mixed Kotlin/Java module is invisible
//! to a Kotlin resolver and *nothing reports the miss*"; a `.m`, a `.mm` or a
//! bridging header inside a Swift target "is the same failure and not a smaller
//! one". So Swift stays in v1 and the hole is made **loud**:
//! [`LangUnclassifiable::MixedObjcTarget`].
//!
//! Two more things are Swift's alone. Its compilation unit is the **module**,
//! so §7.4's `imports(f)` includes every other source file of `f`'s own target
//! with no import statement to find them by. And it has no string specifiers,
//! no relative imports and no dynamic import, so §7.8's site-level list has one
//! entry.

use crate::ids::SwiftTarget;
use crate::lang::{self, Lang, LangUnclassifiable, Unresolvable};
use crate::lex::{self, Token, TokenKind};
use crate::site::{Disposition, ImportSite};
use crate::tree::{self, EntryKind, Tree};

/// IR §7.3's target-call callees, the closed set of rule 2.
const TARGET_CALLEES: [&str; 7] = [
    "target",
    "testTarget",
    "executableTarget",
    "macro",
    "systemLibrary",
    "binaryTarget",
    "plugin",
];

/// The argument labels IR §7.3 test 2 observes. "**Presence alone triggers,
/// whatever the value.** That is why this test needs no widening of the literal
/// subset above: an argument rule 3 would refuse to read is never read here
/// either, only observed."
const OBJC_LABELS: [&str; 3] = ["publicHeadersPath", "cSettings", "cxxSettings"];

/// IR §7.3 test 2's string clause.
const IMPORT_OBJC_HEADER: &str = "-import-objc-header";

/// One target of `RC(swift, tree)` — IR §7.3: "each target is `(name, kind,
/// sourceDirs, sources, exclude, dependencies)`."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub name: String,
    /// The callee, less its leading `.` — `target`, `testTarget`, …
    pub kind: String,
    /// The resolved source directory, repo-relative.
    pub source_dir: String,
    pub sources: Vec<String>,
    pub exclude: Vec<String>,
    pub dependencies: Vec<String>,
    /// `{ p ∈ F(t) : lang(p) = Swift }` — "§3.1's `.swift`, byte-exact and
    /// lowercase only", and "what §7.4 draws edges over".
    pub source_files: Vec<String>,
    /// Some entry beneath this target is a symlink or a submodule, which
    /// IR §2.12 rule 2 forbids following.
    ///
    /// DERIVED: §7.8 lists `symlink-or-submodule` as Swift's whole site-level
    /// list but never says where a Swift site meets one — Swift resolves module
    /// names, not paths. The only place a Swift edge can reach an entry git
    /// marked `120000` or `160000` is a target's own file set, so the flag is
    /// recorded here and the refusal is raised at the site that resolves to
    /// this target.
    pub unfollowable: bool,
}

/// `RC(swift, tree)` — "Extracted from every `Package.swift` in the tree. `RC`
/// is a set of packages, each `(rootDir, [target])`."
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rc {
    /// Every target of every package, sorted by name — the names are unique by
    /// rule 5, so this is a canonical order.
    pub targets: Vec<Target>,
}

impl Rc {
    pub fn target(&self, name: &str) -> Option<&Target> {
        self.targets.iter().find(|t| t.name == name)
    }

    /// The target a source file belongs to. Rule 5's `overlapping-targets`
    /// refusal is what makes this single-valued.
    pub fn target_of(&self, path: &str) -> Option<&Target> {
        self.targets
            .iter()
            .find(|t| t.source_files.iter().any(|p| p == path))
    }

    /// The shape [`crate::ids::swift_id_to_path`] needs.
    pub fn id_targets(&self) -> Vec<SwiftTarget> {
        self.targets
            .iter()
            .map(|t| SwiftTarget {
                name: t.name.clone(),
                sources: t.source_files.clone(),
            })
            .collect()
    }
}

/// IR §7.3's extraction and its seven refusals.
///
/// **Both tests run in both trees** (§7.3): "`RC`'s tuple is manifest-derived,
/// so a branch that drops `Sources/Billing/Oracle.m` into an existing target
/// changes no extracted value and §3.3 Rule 2's comparison does not see it.
/// Test 1 therefore runs against `A` as well as against `B`, and an entry in
/// **either** tree raises `mixed-objc-target`. This is … the half of the rule
/// that matters: the branch is where an oracle arrives." (Case S18.) A caller
/// therefore runs this over `A` and over `B` and refuses if either does.
pub fn rc(tree: &dyn Tree) -> Result<Rc, LangUnclassifiable> {
    let manifests: Vec<String> = tree
        .entries()
        .iter()
        .filter(|e| e.path == "Package.swift" || e.path.ends_with("/Package.swift"))
        .map(|e| e.path.clone())
        .collect();

    if manifests.is_empty() {
        // 6. "A repository containing a `.xcodeproj` or `.xcworkspace` directory
        //    and no `Package.swift` → unclassifiable, reason
        //    `xcode-project-unsupported`."
        if tree.entries().iter().any(|e| {
            e.path.split('/').any(|segment| {
                segment.ends_with(".xcodeproj") || segment.ends_with(".xcworkspace")
            })
        }) {
            return Err(LangUnclassifiable::XcodeProjectUnsupported);
        }
        // §7.8: `no-package-manifest` — "a Swift file exists and no
        // `Package.swift` does".
        if tree
            .entries()
            .iter()
            .any(|e| lang::lang(&e.path) == Some(Lang::Swift))
        {
            return Err(LangUnclassifiable::NoPackageManifest);
        }
        return Ok(Rc::default());
    }

    let mut targets: Vec<Target> = Vec::new();
    for manifest in &manifests {
        let root_dir = tree::dirname(manifest).to_string();
        let bytes = tree
            .read(manifest)
            .ok_or(LangUnclassifiable::ManifestNotLiteral)?;
        let source =
            core::str::from_utf8(bytes).map_err(|_| LangUnclassifiable::ManifestNotLiteral)?;
        targets.extend(parse_manifest(source, &root_dir, tree)?);
    }

    // 5. "Two targets with the same `name` anywhere in the repository →
    //    unclassifiable, reason `duplicate-target-name`."
    let mut names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
    names.sort_unstable();
    if names.windows(2).any(|w| w[0] == w[1]) {
        return Err(LangUnclassifiable::DuplicateTargetName);
    }

    // "A path in the source files of two targets → unclassifiable, reason
    // `overlapping-targets`." (Case S10.)
    let mut owned: Vec<&str> = targets
        .iter()
        .flat_map(|t| t.source_files.iter().map(String::as_str))
        .collect();
    owned.sort_unstable();
    if owned.windows(2).any(|w| w[0] == w[1]) {
        return Err(LangUnclassifiable::OverlappingTargets);
    }

    targets.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Rc { targets })
}

/// IR §7.3's rules 1 to 4, over one `Package.swift`.
fn parse_manifest(
    source: &str,
    root_dir: &str,
    tree: &dyn Tree,
) -> Result<Vec<Target>, LangUnclassifiable> {
    let tokens = lex::without_comments(lex::lex(source, Lang::Swift));

    // 1. "The file contains exactly one top-level expression statement whose
    //    callee is `Package` — the initializer — assigned to `let package`. Any
    //    other top-level statement other than the `// swift-tools-version:`
    //    comment, `import PackageDescription`, and that one `let` →
    //    unclassifiable, reason `manifest-not-literal`."
    //
    // The tools-version line is a comment and the lexer has already discarded
    // it, so the check is over the words that remain at depth zero.
    let mut depth = 0i32;
    let mut initializers = 0usize;
    for (i, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Punct {
            match source.as_bytes()[token.start] {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                _ => {}
            }
            continue;
        }
        if depth != 0 || token.kind != TokenKind::Word {
            continue;
        }
        match token.text(source) {
            "import" | "PackageDescription" | "let" | "package" => {}
            "Package" => {
                let is_initializer = i >= 3
                    && tokens[i - 3].is_word(source, "let")
                    && tokens[i - 2].is_word(source, "package")
                    && tokens[i - 1].is_punct(source, b'=')
                    && tokens.get(i + 1).is_some_and(|t| t.is_punct(source, b'('));
                if !is_initializer {
                    return Err(LangUnclassifiable::ManifestNotLiteral);
                }
                initializers += 1;
            }
            _ => return Err(LangUnclassifiable::ManifestNotLiteral),
        }
    }
    if initializers != 1 {
        return Err(LangUnclassifiable::ManifestNotLiteral);
    }

    // 2. "Inside the initializer, the `targets:` argument is an **array
    //    literal** of call expressions whose callees are …". Case S7:
    //    `targets: buildTargets()` is `manifest-not-literal`.
    let Some(label) = (0..tokens.len()).find(|k| {
        tokens[*k].is_word(source, "targets")
            && tokens.get(k + 1).is_some_and(|t| t.is_punct(source, b':'))
    }) else {
        // A manifest with no `targets:` declares no target; that is legal and
        // simply contributes nothing.
        return Ok(Vec::new());
    };
    let open = label + 2;
    if !tokens.get(open).is_some_and(|t| t.is_punct(source, b'[')) {
        return Err(LangUnclassifiable::ManifestNotLiteral);
    }
    let close = matching(source, &tokens, open).ok_or(LangUnclassifiable::ManifestNotLiteral)?;

    let mut out = Vec::new();
    for element in split_top_level(source, &tokens[open + 1..close]) {
        if element.is_empty() {
            continue;
        }
        out.push(parse_target(source, element, root_dir, tree)?);
    }
    Ok(out)
}

/// One `.target(…)` element of the array literal.
fn parse_target(
    source: &str,
    element: &[Token],
    root_dir: &str,
    tree: &dyn Tree,
) -> Result<Target, LangUnclassifiable> {
    // The callee, `. <word> (`.
    if !element[0].is_punct(source, b'.') || element.len() < 3 {
        return Err(LangUnclassifiable::ManifestNotLiteral);
    }
    let kind = element[1].text(source);
    if element[1].kind != TokenKind::Word || !TARGET_CALLEES.contains(&kind) {
        return Err(LangUnclassifiable::ManifestNotLiteral);
    }
    if !element[2].is_punct(source, b'(') {
        return Err(LangUnclassifiable::ManifestNotLiteral);
    }
    let close = matching(source, element, 2).ok_or(LangUnclassifiable::ManifestNotLiteral)?;
    let args = &element[3..close];

    // Test 2 first, over the raw call tokens. DERIVED ordering: §7.3 fixes that
    // `mixed-objc-target` precedes §3.3 Rule 2's comparison and is silent about
    // rule 3, and test 2 is decidable "by path and by argument label" over
    // bytes rule 3 would refuse to read. Naming the hole beats refusing to look
    // at a manifest that reaches Objective-C by a spelling this subset does not
    // parse.
    if kind == "systemLibrary" {
        return Err(LangUnclassifiable::MixedObjcTarget);
    }
    for (i, token) in args.iter().enumerate() {
        let is_objc_label = token.kind == TokenKind::Word
            && OBJC_LABELS.contains(&token.text(source))
            && args.get(i + 1).is_some_and(|t| t.is_punct(source, b':'));
        if is_objc_label {
            return Err(LangUnclassifiable::MixedObjcTarget);
        }
        if token.simple_literal(source) == Some(IMPORT_OBJC_HEADER) {
            return Err(LangUnclassifiable::MixedObjcTarget);
        }
    }

    let mut name: Option<String> = None;
    let mut path: Option<String> = None;
    let mut sources: Vec<String> = Vec::new();
    let mut exclude: Vec<String> = Vec::new();
    let mut dependencies: Vec<String> = Vec::new();

    for arg in split_top_level(source, args) {
        if arg.len() < 2 || arg[0].kind != TokenKind::Word || !arg[1].is_punct(source, b':') {
            continue;
        }
        let label = arg[0].text(source);
        let value = &arg[2..];
        match label {
            // 3. "Every `name:`, `path:`, `sources:` and `exclude:` argument is
            //    a simple string literal or an array literal of simple string
            //    literals. Any identifier reference, string interpolation, `+`,
            //    ternary, `#if`, `for`, `map`, or function call in those
            //    positions → unclassifiable, reason `manifest-not-literal`."
            "name" | "path" => {
                let literals = literal_strings(source, value)
                    .ok_or(LangUnclassifiable::ManifestNotLiteral)?;
                let [one] = literals.as_slice() else {
                    return Err(LangUnclassifiable::ManifestNotLiteral);
                };
                if label == "name" {
                    name = Some(one.clone());
                } else {
                    path = Some(one.clone());
                }
            }
            "sources" | "exclude" => {
                let literals = literal_strings(source, value)
                    .ok_or(LangUnclassifiable::ManifestNotLiteral)?;
                if label == "sources" {
                    sources = literals;
                } else {
                    exclude = literals;
                }
            }
            // 4. "`dependencies:` is read only for its simple string literals
            //    and `.target(name: "X")` / `.byName(name: "X")` forms; anything
            //    else contributes no dependency and **is not an error**, because
            //    target dependencies do not affect which files belong to a
            //    target."
            "dependencies" => {
                for (i, token) in value.iter().enumerate() {
                    let Some(literal) = token.simple_literal(source) else {
                        continue;
                    };
                    let named = i >= 2
                        && value[i - 1].is_punct(source, b':')
                        && value[i - 2].is_word(source, "name");
                    let bare = i == 0
                        || value[i - 1].is_punct(source, b'[')
                        || value[i - 1].is_punct(source, b',');
                    if named || bare {
                        dependencies.push(literal.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    let name = name.ok_or(LangUnclassifiable::ManifestNotLiteral)?;

    // "**A target's source directory.** If `path:` is given, the directory is
    // `rootDir + "/" + path`. Otherwise the first existing directory in this
    // order …"
    let source_dir = match &path {
        Some(p) => tree::join(root_dir, p),
        None => {
            let order: &[&str] = if kind == "testTarget" {
                &["Tests", "Sources", "Source", "src", "srcs"]
            } else {
                &["Sources", "Source", "src", "srcs", "Tests"]
            };
            order
                .iter()
                .map(|prefix| tree::join(root_dir, &format!("{prefix}/{name}")))
                .find(|candidate| tree.is_dir(candidate))
                // "None existing → unclassifiable, reason `target-dir-missing`."
                .ok_or(LangUnclassifiable::TargetDirMissing)?
        }
    };

    let (file_set, unfollowable) = file_set(tree, &source_dir, &sources, &exclude);

    // "**Test 1 — a C-family entry in the file set.**" `F(t)` is post-`exclude:`,
    // so "a `.m` under an `exclude:` entry does **not** trigger: it is not
    // compiled into the target and no Swift file can reach it." (Cases S14,
    // S15, S17.)
    if file_set.iter().any(|p| lang::is_c_family(p)) {
        return Err(LangUnclassifiable::MixedObjcTarget);
    }

    let source_files = file_set
        .into_iter()
        .filter(|p| lang::lang(p) == Some(Lang::Swift))
        .collect();

    Ok(Target {
        name,
        kind: kind.to_string(),
        source_dir,
        sources,
        exclude,
        dependencies,
        source_files,
        unfollowable,
    })
}

/// IR §7.3's `F(t)`: "if `sources:` is given, then for each entry — the entry
/// itself when it names a blob in the tree, and every blob recursively beneath
/// it when it names a directory; otherwise every blob recursively beneath the
/// source directory, **at every depth**. Then remove from `F(t)` every path
/// equal to, or beneath, any `exclude:` entry. `F(t)` is the **whole** set of
/// remaining blobs and is filtered by no extension."
fn file_set(
    tree: &dyn Tree,
    source_dir: &str,
    sources: &[String],
    exclude: &[String],
) -> (Vec<String>, bool) {
    let mut unfollowable = false;
    let mut collect = |root: &str, out: &mut Vec<String>| {
        for entry in tree.entries() {
            let under = entry.path == root || entry.path.starts_with(&format!("{root}/"));
            if !under {
                continue;
            }
            match entry.kind {
                EntryKind::File => {
                    if !out.contains(&entry.path) {
                        out.push(entry.path.clone());
                    }
                }
                // IR §2.12 rule 2: never followed, and never silently dropped.
                EntryKind::Symlink | EntryKind::Submodule => unfollowable = true,
            }
        }
    };

    let mut set: Vec<String> = Vec::new();
    if sources.is_empty() {
        collect(source_dir, &mut set);
    } else {
        for entry in sources {
            collect(&tree::join(source_dir, entry), &mut set);
        }
    }

    for excluded in exclude {
        let root = tree::join(source_dir, excluded);
        let prefix = format!("{root}/");
        set.retain(|p| *p != root && !p.starts_with(&prefix));
    }
    set.sort();
    (set, unfollowable)
}

/// The index of the token closing the bracket opened at `open`.
fn matching(source: &str, tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (k, token) in tokens.iter().enumerate().skip(open) {
        if token.kind != TokenKind::Punct {
            continue;
        }
        match source.as_bytes()[token.start] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}

/// Split on `,` at nesting depth zero.
fn split_top_level<'a>(source: &str, tokens: &'a [Token]) -> Vec<&'a [Token]> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (k, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Punct {
            continue;
        }
        match source.as_bytes()[token.start] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                out.push(&tokens[start..k]);
                start = k + 1;
            }
            _ => {}
        }
    }
    if start < tokens.len() {
        out.push(&tokens[start..]);
    }
    out
}

/// A simple string literal, or an array literal of simple string literals, and
/// nothing else. `None` is rule 3's `manifest-not-literal`.
fn literal_strings(source: &str, value: &[Token]) -> Option<Vec<String>> {
    if value.len() == 1 {
        return value.first()?.simple_literal(source).map(|s| vec![s.to_string()]);
    }
    if !value.first()?.is_punct(source, b'[') || !value.last()?.is_punct(source, b']') {
        return None;
    }
    let inner = &value[1..value.len() - 1];
    let mut out = Vec::new();
    for (i, token) in inner.iter().enumerate() {
        if i % 2 == 0 {
            out.push(token.simple_literal(source)?.to_string());
        } else if !token.is_punct(source, b',') {
            return None;
        }
    }
    Some(out)
}

/// Every explicit `import` site in a Swift file — IR §7.2's forms.
pub fn sites(source: &str, rc: &Rc) -> Vec<ImportSite> {
    let tokens = lex::without_comments(lex::lex(source, Lang::Swift));
    let mut out = Vec::new();
    for i in 0..tokens.len() {
        if !tokens[i].is_word(source, "import") {
            continue;
        }
        // IR §7.1's anchor: "a `word` token `import` not immediately preceded
        // by `.`". The optional `@testable` / `@_exported` attributes precede
        // it and change nothing about resolution — §7.2: "**`@testable import`
        // needs no special handling at all.** It changes the *visibility* of
        // what the imported module exports … and it changes nothing about which
        // module is imported or which files that module contains."
        if i > 0 && tokens[i - 1].is_punct(source, b'.') {
            continue;
        }
        // "`import struct Foo.Baz` (and `class`, `enum`, `protocol`,
        // `typealias`, `func`, `let`, `var`) | `Foo`."
        let mut k = i + 1;
        if tokens.get(k).is_some_and(|t| {
            matches!(
                t.text(source),
                "struct" | "class" | "enum" | "protocol" | "typealias" | "func" | "let" | "var"
            ) && t.kind == TokenKind::Word
        }) {
            k += 1;
        }
        let Some(module) = tokens.get(k) else {
            continue;
        };
        if module.kind != TokenKind::Word {
            continue;
        }
        // "`import Foo.Bar` | `Foo` — a submodule path; the first component is
        // the module."
        let name = module.text(source);
        out.push(ImportSite {
            offset: tokens[i].start,
            disposition: match rc.target(name) {
                // "for each `import N` site in `f`, every source file of the
                // target named `N` if one exists (and `external` if none does)".
                Some(target) if target.unfollowable => {
                    Disposition::Unresolvable(Unresolvable::SymlinkOrSubmodule)
                }
                Some(target) => Disposition::Repo(target.source_files.clone()),
                // IR §7.4: "the second is why `external` is the right default
                // for an unknown module name (§3.2): `Foundation`, `XCTest`,
                // `Combine` and every SDK framework land there, and an oracle
                // cannot, because an oracle is a file in the tree and a file in
                // the tree is in a target."
                None => Disposition::External,
            },
        });
    }
    out
}

/// IR §7.4's `imports(f)`, both halves:
///
/// > `imports(f) =` every source file of `M` other than `f`, **plus** for each
/// > `import N` site in `f`, every source file of the target named `N` if one
/// > exists (and `external` if none does).
///
/// "Without the first, an oracle in a sibling file of the test target is
/// invisible: the test uses it with no import line to find."
pub fn imports(source: &str, path: &str, rc: &Rc) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(own) = rc.target_of(path) {
        for sibling in &own.source_files {
            if sibling != path {
                out.push(sibling.clone());
            }
        }
    }
    for site in sites(source, rc) {
        for target in site.targets() {
            if target != path && !out.contains(target) {
                out.push(target.clone());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::MapTree;

    fn tree_of(files: &[(&str, &str)]) -> MapTree {
        MapTree::new(files.iter().map(|(p, c)| (p.to_string(), c.to_string())))
    }

    fn manifest(targets: &str) -> String {
        format!(
            "// swift-tools-version:5.9\nimport PackageDescription\n\nlet package = Package(\n    name: \"Demo\",\n    targets: [{targets}]\n)\n"
        )
    }

    /// Cases S1 and S2: `@testable` resolves exactly as `import`, and a
    /// submodule path names its first component.
    #[test]
    fn s1_and_s2_testable_and_a_decl_import_name_the_same_module() {
        let tree = tree_of(&[
            (
                "Package.swift",
                &manifest("\n        .target(name: \"M\"),\n        .testTarget(name: \"MTests\")\n    "),
            ),
            ("Sources/M/A.swift", ""),
            ("Sources/M/B.swift", ""),
            ("Tests/MTests/T.swift", ""),
        ]);
        let rc = rc(&tree).unwrap();
        let expected = Disposition::Repo(vec![
            "Sources/M/A.swift".into(),
            "Sources/M/B.swift".into(),
        ]);
        for source in [
            "import M\n",
            "@testable import M\n",
            "@_exported import M\n",
            "import M.Sub\n",
            "import struct M.Thing\n",
            "import class M.Thing\n",
        ] {
            let found = sites(source, &rc);
            assert_eq!(found.len(), 1, "{source}");
            assert_eq!(found[0].disposition, expected, "{source}");
        }
    }

    /// Case S3: "Two files in one target, neither importing the other | each is
    /// an edge target of the other (implicit same-module)."
    #[test]
    fn s3_two_files_of_one_target_import_each_other_implicitly() {
        let tree = tree_of(&[
            ("Package.swift", &manifest(".target(name: \"M\")")),
            ("Sources/M/A.swift", ""),
            ("Sources/M/B.swift", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(imports("", "Sources/M/A.swift", &rc), ["Sources/M/B.swift"]);
        assert_eq!(imports("", "Sources/M/B.swift", &rc), ["Sources/M/A.swift"]);
    }

    /// Case S4: "`#if os(Linux) import A #else import B #endif` | **both** `A`
    /// and `B` are sites." IR §3.7: "dropping a branch is how an oracle hides."
    #[test]
    fn s4_both_branches_of_a_compilation_condition_are_sites() {
        let tree = tree_of(&[
            (
                "Package.swift",
                &manifest(
                    "\n        .target(name: \"A\"),\n        .target(name: \"B\"),\n        .target(name: \"M\")\n    ",
                ),
            ),
            ("Sources/A/a.swift", ""),
            ("Sources/B/b.swift", ""),
            ("Sources/M/m.swift", ""),
        ]);
        let rc = rc(&tree).unwrap();
        let source = "#if os(Linux)\nimport A\n#else\nimport B\n#endif\n";
        let found = sites(source, &rc);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(
            found[0].disposition,
            Disposition::Repo(vec!["Sources/A/a.swift".into()])
        );
        assert_eq!(
            found[1].disposition,
            Disposition::Repo(vec!["Sources/B/b.swift".into()])
        );
    }

    /// Case S5: "`import Foundation` with no target of that name | `external`."
    #[test]
    fn s5_an_sdk_module_is_external_and_never_a_tripwire() {
        let tree = tree_of(&[
            ("Package.swift", &manifest(".target(name: \"M\")")),
            ("Sources/M/A.swift", ""),
        ]);
        let rc = rc(&tree).unwrap();
        for source in ["import Foundation\n", "import XCTest\n", "import Combine\n"] {
            let found = sites(source, &rc);
            assert_eq!(found[0].disposition, Disposition::External, "{source}");
        }
    }

    /// Case S7: "`Package.swift` with `targets: buildTargets()` |
    /// `lang-unclassifiable`, reason `manifest-not-literal`."
    #[test]
    fn s7_a_computed_targets_argument_is_manifest_not_literal() {
        let tree = tree_of(&[(
            "Package.swift",
            "import PackageDescription\nlet package = Package(name: \"D\", targets: buildTargets())\n",
        )]);
        assert_eq!(
            rc(&tree).unwrap_err(),
            LangUnclassifiable::ManifestNotLiteral
        );
    }

    /// Rule 1: any other top-level statement is `manifest-not-literal`.
    #[test]
    fn a_top_level_statement_outside_the_subset_is_manifest_not_literal() {
        for body in [
            "import PackageDescription\nlet suffix = \"X\"\nlet package = Package(name: \"D\", targets: [])\n",
            "import PackageDescription\nfunc f() {}\nlet package = Package(name: \"D\", targets: [])\n",
            "import PackageDescription\n",
        ] {
            let tree = tree_of(&[("Package.swift", body)]);
            assert_eq!(
                rc(&tree).unwrap_err(),
                LangUnclassifiable::ManifestNotLiteral,
                "{body}"
            );
        }
    }

    /// Rule 3: a non-literal in a `name:`, `path:`, `sources:` or `exclude:`
    /// position.
    #[test]
    fn a_non_literal_name_or_path_is_manifest_not_literal() {
        for element in [
            ".target(name: targetName)",
            ".target(name: \"M\" + suffix)",
            ".target(name: \"M\", path: computePath())",
            ".target(name: \"M\", sources: [fileList])",
        ] {
            let tree = tree_of(&[
                ("Package.swift", &manifest(element)),
                ("Sources/M/A.swift", ""),
            ]);
            assert_eq!(
                rc(&tree).unwrap_err(),
                LangUnclassifiable::ManifestNotLiteral,
                "{element}"
            );
        }
    }

    /// Case S8: "`Package.swift` with `path: \"Custom/Dir\"` (a literal) |
    /// honoured."
    #[test]
    fn s8_a_literal_path_is_honoured() {
        let tree = tree_of(&[
            (
                "Package.swift",
                &manifest(".target(name: \"M\", path: \"Custom/Dir\")"),
            ),
            ("Custom/Dir/A.swift", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(rc.target("M").unwrap().source_dir, "Custom/Dir");
        assert_eq!(rc.target("M").unwrap().source_files, ["Custom/Dir/A.swift"]);
    }

    /// Case S9: "A target with `exclude: [\"Legacy\"]` | files under `Legacy`
    /// are not target sources."
    #[test]
    fn s9_excluded_files_are_not_target_sources() {
        let tree = tree_of(&[
            (
                "Package.swift",
                &manifest(".target(name: \"M\", exclude: [\"Legacy\"])"),
            ),
            ("Sources/M/A.swift", ""),
            ("Sources/M/Legacy/Old.swift", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(rc.target("M").unwrap().source_files, ["Sources/M/A.swift"]);
    }

    /// Case S10: "Two targets whose source globs overlap |
    /// `lang-unclassifiable`, reason `overlapping-targets`."
    #[test]
    fn s10_two_targets_owning_one_file_is_overlapping_targets() {
        let tree = tree_of(&[
            (
                "Package.swift",
                &manifest(
                    "\n        .target(name: \"A\", path: \"Shared\"),\n        .target(name: \"B\", path: \"Shared\")\n    ",
                ),
            ),
            ("Shared/X.swift", ""),
        ]);
        assert_eq!(
            rc(&tree).unwrap_err(),
            LangUnclassifiable::OverlappingTargets
        );
    }

    /// Case S11: "A repository with `.xcodeproj` and no `Package.swift` |
    /// `lang-unclassifiable`, reason `xcode-project-unsupported`."
    #[test]
    fn s11_an_xcode_project_with_no_package_manifest_is_unsupported() {
        let tree = tree_of(&[
            ("App.xcodeproj/project.pbxproj", ""),
            ("Sources/A.swift", ""),
        ]);
        assert_eq!(
            rc(&tree).unwrap_err(),
            LangUnclassifiable::XcodeProjectUnsupported
        );
    }

    /// IR §7.8: `no-package-manifest` — "a Swift file exists and no
    /// `Package.swift` does".
    #[test]
    fn a_swift_file_with_no_package_manifest_is_its_own_reason() {
        let tree = tree_of(&[("Sources/A.swift", "")]);
        assert_eq!(
            rc(&tree).unwrap_err(),
            LangUnclassifiable::NoPackageManifest
        );
        // A repository with no Swift at all is simply empty of Swift packages.
        let none = tree_of(&[("README.md", "")]);
        assert_eq!(rc(&none).unwrap(), Rc::default());
    }

    /// Case S12: "A target whose file set contains `Sources/Billing/Legacy.m` |
    /// `lang-unclassifiable`, reason `mixed-objc-target`. … **Must not** be
    /// 'the `.m` is `lang: none` and contributes nothing' — that answer is the
    /// silent hole §7.3 closes."
    #[test]
    fn s12_a_c_family_entry_in_the_file_set_is_mixed_objc_target() {
        let tree = tree_of(&[
            ("Package.swift", &manifest(".target(name: \"Billing\")")),
            ("Sources/Billing/Invoice.swift", ""),
            ("Sources/Billing/Legacy.m", ""),
        ]);
        assert_eq!(
            rc(&tree).unwrap_err(),
            LangUnclassifiable::MixedObjcTarget
        );
    }

    /// Case S13: "A target every entry of whose file set ends in `.swift` … |
    /// **no** `mixed-objc-target`; resolution proceeds by §7.4 … The refusal
    /// fires on a C-family entry or construct and **on nothing else**."
    #[test]
    fn s13_a_pure_swift_package_is_not_refused() {
        let tree = tree_of(&[
            (
                "Package.swift",
                &manifest("\n        .target(name: \"A\"),\n        .target(name: \"B\")\n    "),
            ),
            ("Sources/A/x.swift", ""),
            ("Sources/A/Resources/data.json", ""),
            ("Sources/B/y.swift", ""),
        ]);
        let rc = rc(&tree).expect("pure Swift must not be refused");
        assert_eq!(rc.targets.len(), 2);
        // A non-Swift, non-C-family entry is in `F(t)` and is simply not a
        // source file: "`F(t)` is the **whole** set of remaining blobs and is
        // filtered by no extension."
        assert_eq!(rc.target("A").unwrap().source_files, ["Sources/A/x.swift"]);
    }

    /// Case S14: "A target whose file set contains
    /// `Tests/BillingTests/BillingTests-Bridging-Header.h` | `mixed-objc-target`,
    /// by the `.h` extension alone. **Must not** require the stem to end in
    /// `-Bridging-Header`: no rule in §7.3 reads a filename stem."
    #[test]
    fn s14_a_bridging_header_triggers_by_its_extension_and_not_by_its_stem() {
        let tree = tree_of(&[
            ("Package.swift", &manifest(".testTarget(name: \"BillingTests\")")),
            ("Tests/BillingTests/T.swift", ""),
            (
                "Tests/BillingTests/BillingTests-Bridging-Header.h",
                "",
            ),
        ]);
        assert_eq!(rc(&tree).unwrap_err(), LangUnclassifiable::MixedObjcTarget);
    }

    /// Case S15: "A target with `exclude: [\"Legacy\"]` whose only C-family
    /// entry is `Legacy/Old.m` | **no** `mixed-objc-target`. `F(t)` is
    /// post-`exclude:`, and a file compiled into no target is reachable from no
    /// Swift file."
    #[test]
    fn s15_a_c_family_file_under_an_exclude_does_not_trigger() {
        let tree = tree_of(&[
            (
                "Package.swift",
                &manifest(".target(name: \"M\", exclude: [\"Legacy\"])"),
            ),
            ("Sources/M/A.swift", ""),
            ("Sources/M/Legacy/Old.m", ""),
        ]);
        let rc = rc(&tree).expect("excluded, so not compiled in");
        assert_eq!(rc.target("M").unwrap().source_files, ["Sources/M/A.swift"]);
    }

    /// Case S16: "A `.systemLibrary` target; or any target whose call carries
    /// `publicHeadersPath: \"include\"`, whatever that directory holds |
    /// `mixed-objc-target`, by test 2. **Must not** require a header to exist
    /// on disk first — presence of the label is the test."
    #[test]
    fn s16_test_two_triggers_on_the_label_alone_with_no_header_on_disk() {
        for element in [
            ".systemLibrary(name: \"CBits\")",
            ".target(name: \"M\", publicHeadersPath: \"include\")",
            ".target(name: \"M\", cSettings: [])",
            ".target(name: \"M\", cxxSettings: [])",
        ] {
            let tree = tree_of(&[
                ("Package.swift", &manifest(element)),
                ("Sources/M/A.swift", ""),
                ("Sources/CBits/x.swift", ""),
            ]);
            assert_eq!(
                rc(&tree).unwrap_err(),
                LangUnclassifiable::MixedObjcTarget,
                "{element}"
            );
        }
    }

    /// Case S17: "A package whose only C-family target is pure Objective-C — no
    /// `.swift` entry anywhere in its file set — which a Swift target `import`s
    /// by name | `mixed-objc-target`. **Must not** be narrowed to targets that
    /// also contain Swift: `import CBits` over a target with no Swift source
    /// yields zero edges and no finding, which is the miss itself."
    #[test]
    fn s17_a_pure_objective_c_target_still_triggers() {
        let tree = tree_of(&[
            (
                "Package.swift",
                &manifest(
                    "\n        .target(name: \"CBits\"),\n        .target(name: \"M\")\n    ",
                ),
            ),
            ("Sources/CBits/impl.c", ""),
            ("Sources/M/A.swift", ""),
        ]);
        assert_eq!(rc(&tree).unwrap_err(), LangUnclassifiable::MixedObjcTarget);
    }

    /// Case S18: "`B` pure Swift; the branch adds `Sources/Billing/Oracle.m` to
    /// an existing target and edits no manifest | `mixed-objc-target`, from the
    /// test over `A`. **Must not** be `rc-changed-on-branch` … and **must not**
    /// pass because `RC` is read from `B`."
    #[test]
    fn s18_a_branch_added_objective_c_file_refuses_from_the_test_over_a() {
        let manifest_bytes = manifest(".target(name: \"Billing\")");
        let base = tree_of(&[
            ("Package.swift", manifest_bytes.as_str()),
            ("Sources/Billing/Invoice.swift", ""),
        ]);
        let approval = tree_of(&[
            ("Package.swift", manifest_bytes.as_str()),
            ("Sources/Billing/Invoice.swift", ""),
            ("Sources/Billing/Oracle.m", ""),
        ]);
        // `B` extracts cleanly, so a resolver that only read `B` would proceed.
        assert!(rc(&base).is_ok());
        // …and the manifest did not move, so §3.3 Rule 2 sees no difference.
        assert_eq!(
            rc(&base).unwrap(),
            Rc {
                targets: vec![Target {
                    name: "Billing".into(),
                    kind: "target".into(),
                    source_dir: "Sources/Billing".into(),
                    sources: Vec::new(),
                    exclude: Vec::new(),
                    dependencies: Vec::new(),
                    source_files: vec!["Sources/Billing/Invoice.swift".into()],
                    unfollowable: false,
                }]
            }
        );
        // The test over `A` is what fires, and it names the hole.
        assert_eq!(
            rc(&approval).unwrap_err(),
            LangUnclassifiable::MixedObjcTarget
        );
    }

    /// Case S19: "`swiftSettings: [.unsafeFlags([\"-import-objc-header\",
    /// \"Shim.h\"])]` on a target whose file set holds no C-family entry |
    /// `mixed-objc-target`, by test 2's string-literal clause."
    #[test]
    fn s19_an_import_objc_header_flag_triggers_however_deeply_it_is_nested() {
        let tree = tree_of(&[
            (
                "Package.swift",
                &manifest(
                    ".target(name: \"M\", swiftSettings: [.unsafeFlags([\"-import-objc-header\", \"Shim.h\"])])",
                ),
            ),
            ("Sources/M/A.swift", ""),
        ]);
        assert_eq!(rc(&tree).unwrap_err(), LangUnclassifiable::MixedObjcTarget);
    }

    /// IR §7.3's search order, and its one difference for a test target.
    #[test]
    fn a_test_target_looks_under_tests_first_and_every_other_kind_last() {
        let tree = tree_of(&[
            (
                "Package.swift",
                &manifest(
                    "\n        .testTarget(name: \"M\"),\n        .target(name: \"N\")\n    ",
                ),
            ),
            ("Tests/M/T.swift", ""),
            ("Sources/M/S.swift", ""),
            ("Tests/N/T.swift", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(rc.target("M").unwrap().source_dir, "Tests/M");
        // `N` has no `Sources/N`, `Source/N`, `src/N` or `srcs/N`, so the last
        // entry of the non-test order, `Tests/N`, wins.
        assert_eq!(rc.target("N").unwrap().source_dir, "Tests/N");
    }

    /// IR §7.3: "None existing → unclassifiable, reason `target-dir-missing`."
    #[test]
    fn a_target_with_no_source_directory_is_target_dir_missing() {
        let tree = tree_of(&[
            ("Package.swift", &manifest(".target(name: \"Missing\")")),
            ("Sources/Other/A.swift", ""),
        ]);
        assert_eq!(rc(&tree).unwrap_err(), LangUnclassifiable::TargetDirMissing);
    }

    /// IR §7.3 rule 5: "Two targets with the same `name` anywhere in the
    /// repository".
    #[test]
    fn a_duplicate_target_name_anywhere_in_the_repository_refuses() {
        let tree = tree_of(&[
            ("Package.swift", &manifest(".target(name: \"M\")")),
            ("Sources/M/A.swift", ""),
            ("sub/Package.swift", &manifest(".target(name: \"M\")")),
            ("sub/Sources/M/B.swift", ""),
        ]);
        assert_eq!(
            rc(&tree).unwrap_err(),
            LangUnclassifiable::DuplicateTargetName
        );
    }

    /// IR §7.3 rule 4: `dependencies:` reads its two literal forms and "anything
    /// else contributes no dependency and **is not an error**".
    #[test]
    fn dependencies_read_two_forms_and_ignore_the_rest_without_erroring() {
        let tree = tree_of(&[
            (
                "Package.swift",
                &manifest(
                    "\n        .target(name: \"M\"),\n        .testTarget(name: \"MTests\", dependencies: [\"M\", .target(name: \"M\"), .product(name: \"Other\", package: \"p\")])\n    ",
                ),
            ),
            ("Sources/M/A.swift", ""),
            ("Tests/MTests/T.swift", ""),
        ]);
        let rc = rc(&tree).expect("a `.product` is not an error");
        // `"M"` (bare) and `.target(name: "M")` are read; `.product(…)` is not,
        // and its `name:` literal is not a target dependency.
        assert_eq!(rc.target("MTests").unwrap().dependencies, ["M", "M", "Other"]);
    }

    /// IR §7.4's consequence, stated honestly: "a target with two or more files
    /// at `base` has every one of its files … reached", so a Swift closure "is
    /// module-shaped and considerably larger than the equivalent Python or
    /// TypeScript one."
    #[test]
    fn a_swift_import_reaches_every_source_file_of_the_named_target() {
        let tree = tree_of(&[
            (
                "Package.swift",
                &manifest(
                    "\n        .target(name: \"Billing\"),\n        .testTarget(name: \"BillingTests\")\n    ",
                ),
            ),
            ("Sources/Billing/Invoice.swift", ""),
            ("Sources/Billing/Tax.swift", ""),
            ("Sources/Billing/Rounding.swift", ""),
            ("Tests/BillingTests/T.swift", ""),
            ("Tests/BillingTests/Helper.swift", ""),
        ]);
        let rc = rc(&tree).unwrap();
        let reached = imports(
            "@testable import Billing\nimport XCTest\n",
            "Tests/BillingTests/T.swift",
            &rc,
        );
        assert_eq!(
            reached,
            [
                "Sources/Billing/Invoice.swift",
                "Sources/Billing/Rounding.swift",
                "Sources/Billing/Tax.swift",
                "Tests/BillingTests/Helper.swift",
            ]
        );
    }

    /// IR §11.5's `id → path` reads `RC(swift, X)`'s targets; this is the
    /// handshake between the two.
    #[test]
    fn the_rc_supplies_the_targets_the_swift_test_id_to_path_lookup_needs() {
        let tree = tree_of(&[
            ("Package.swift", &manifest(".testTarget(name: \"MTests\")")),
            (
                "Tests/MTests/InvoiceTests.swift",
                "import XCTest\nfinal class InvoiceTests: XCTestCase { func testTax() {} }\n",
            ),
            ("Tests/MTests/Helper.swift", "struct Helper {}\n"),
        ]);
        let rc = rc(&tree).unwrap();
        let read = |p: &str| {
            tree.read(p)
                .and_then(|b| core::str::from_utf8(b).ok())
                .map(str::to_string)
        };
        assert_eq!(
            crate::ids::swift_id_to_path("MTests.InvoiceTests/testTax", &rc.id_targets(), read),
            "Tests/MTests/InvoiceTests.swift"
        );
    }

    /// IR §2.12 rule 2, at the only place a Swift edge can meet one.
    #[test]
    fn a_symlink_inside_a_target_makes_an_import_of_it_unresolvable() {
        let tree = tree_of(&[
            ("Package.swift", &manifest(".target(name: \"M\")")),
            ("Sources/M/A.swift", ""),
        ])
        .with_special("Sources/M/link.swift", EntryKind::Symlink);
        let rc = rc(&tree).unwrap();
        assert!(rc.target("M").unwrap().unfollowable);
        let found = sites("import M\n", &rc);
        assert_eq!(
            found[0].disposition,
            Disposition::Unresolvable(Unresolvable::SymlinkOrSubmodule)
        );
    }
}
