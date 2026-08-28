//! The TypeScript / JavaScript resolver — IR §5.
//!
//! One resolver for both languages: IR §3.1, "`ts` covers JavaScript; PB §6.7
//! counts 'TypeScript/JavaScript' as one language and there is one resolver for
//! both."
//!
//! This is the language whose pattern dialect defect IR §2.4.1 records, and the
//! one whose candidate list is longest. IR §5.2 states why the list needs no
//! ambiguity rule: "The list is exhaustive and ordered, so no ambiguity rule is
//! needed: a directory containing both `x.ts` and `x.js` resolves to `x.ts`,
//! which is what TypeScript does, and the resolver does not need to know that
//! to be deterministic."

use crate::jsonc::{self, Json};
use crate::lang::{self, Lang, LangUnclassifiable, Unresolvable};
use crate::lex::{self, Token, TokenKind};
use crate::site::{Disposition, ImportSite};
use crate::tree::{self, Tree};

/// `RC(ts, tree)` — IR §5.3: "`RC` is the pair `(baseUrl, paths)` where `paths`
/// is a list of `(pattern, [substitution, …])` in the file's own key order."
///
/// Equality is **structural**, which is what IR §3.3 Rule 2 compares: "adding a
/// dependency to `pubspec.yaml` or a script to `package.json` changes nothing;
/// adding a target, a project, a source-set override or a path alias raises
/// `lang-unclassifiable` with reason `rc-changed-on-branch`."
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rc {
    /// Repo-relative directory, already resolved against the file that declared
    /// it. `None` where no `baseUrl` was declared.
    pub base_url: Option<String>,
    /// `(pattern, [substitution, …])`, in the declaring file's key order.
    pub paths: Vec<(String, Vec<String>)>,
    /// The directory substitutions are resolved against: "relative to `baseUrl`
    /// (or, absent `baseUrl`, to the directory of the `tsconfig.json` that
    /// declared `paths`)".
    pub paths_base: String,
}

/// IR §5.3: "Extracted from the repository-root `tsconfig.json`, or
/// `jsconfig.json` if no `tsconfig.json` exists at the root."
///
/// "If no `tsconfig.json` and no `jsconfig.json` exists at the repository root,
/// `RC` is `(none, [])` — **legal, not unclassifiable**. A repository with no
/// alias table simply has no bare specifier that resolves inside it."
pub fn rc(tree: &dyn Tree) -> Result<Rc, LangUnclassifiable> {
    let root = if tree.is_file("tsconfig.json") {
        "tsconfig.json"
    } else if tree.is_file("jsconfig.json") {
        "jsconfig.json"
    } else {
        return Ok(Rc::default());
    };

    // The `extends` chain, base-most first, each with the directory of the file
    // that carries it — `baseUrl` is "resolved relative to the file that
    // declares it", so the directory travels with the value.
    let mut chain: Vec<(String, Json)> = Vec::new();
    let mut visited: Vec<String> = Vec::new();
    let mut current = root.to_string();
    loop {
        if visited.contains(&current) {
            // "A cycle → unclassifiable, reason `tsconfig-extends-cycle`."
            return Err(LangUnclassifiable::TsconfigExtendsCycle);
        }
        visited.push(current.clone());
        let bytes = tree
            .read(&current)
            .ok_or(LangUnclassifiable::TsconfigUnparseable)?;
        let text =
            core::str::from_utf8(bytes).map_err(|_| LangUnclassifiable::TsconfigUnparseable)?;
        let json = jsonc::parse(text).ok_or(LangUnclassifiable::TsconfigUnparseable)?;
        let dir = tree::dirname(&current).to_string();
        let next = match json.get("extends") {
            None => None,
            Some(Json::Str(spec)) => Some(resolve_extends(tree, &dir, spec)?),
            // "An `extends` naming a bare specifier, an absolute path, **or an
            // array** → `RC` unclassifiable, reason `tsconfig-extends-external`."
            // Any other shape is refused with it: DERIVED, because §5.3 names
            // only the three that occur and a fourth must not be silently
            // ignored.
            Some(_) => return Err(LangUnclassifiable::TsconfigExtendsExternal),
        };
        chain.push((dir, json));
        match next {
            Some(path) => current = path,
            None => break,
        }
    }
    // "Child keys override parent keys" — so fold base-most first.
    chain.reverse();

    let mut rc = Rc::default();
    let mut paths_dir = String::new();
    for (dir, json) in &chain {
        let Some(options) = json.get("compilerOptions") else {
            continue;
        };
        // The merge is per `compilerOptions` key rather than wholesale: a child
        // that sets only `strict` must not erase the parent's `paths`. DERIVED
        // — §5.3 says "child keys override parent keys" and does not fix the
        // granularity; per-key is `tsc`'s and is the only granularity under
        // which an `extends` chain is useful at all.
        if let Some(value) = options.get("baseUrl") {
            // "must be a simple string"
            let raw = value
                .as_str()
                .ok_or(LangUnclassifiable::BaseurlEscapesRoot)?;
            let joined = tree::join(dir, raw);
            let resolved =
                tree::normalize(&joined).ok_or(LangUnclassifiable::BaseurlEscapesRoot)?;
            rc.base_url = Some(resolved);
        }
        if let Some(value) = options.get("paths") {
            rc.paths = extract_paths(value)?;
            paths_dir = dir.clone();
        }
    }
    rc.paths_base = rc.base_url.clone().unwrap_or(paths_dir);
    Ok(rc)
}

/// IR §5.3 step 2: "`extends` is followed only for a value that is a simple
/// string beginning `./` or `../`, resolved against the extending file's
/// directory, with the extension `.json` appended if absent."
fn resolve_extends(tree: &dyn Tree, dir: &str, spec: &str) -> Result<String, LangUnclassifiable> {
    if !(spec.starts_with("./") || spec.starts_with("../")) {
        return Err(LangUnclassifiable::TsconfigExtendsExternal);
    }
    let with_ext = if spec.ends_with(".json") {
        spec.to_string()
    } else {
        format!("{spec}.json")
    };
    let joined = tree::join(dir, &with_ext);
    let path = tree::normalize(&joined).ok_or(LangUnclassifiable::TsconfigExtendsExternal)?;
    if !tree.is_file(&path) {
        // A relative `extends` whose target is not in the tree names a file the
        // tree cannot show, which is the same situation as a bare specifier.
        // DERIVED: §5.3 fixes the three refusable *forms* and is silent on a
        // well-formed one that resolves to nothing.
        return Err(LangUnclassifiable::TsconfigExtendsExternal);
    }
    Ok(path)
}

/// IR §5.3 step 4: "`compilerOptions.paths` must be an object whose every value
/// is an array of strings, each containing **at most one** `*`, and whose every
/// key contains at most one `*`, else unclassifiable, reason `paths-malformed`."
fn extract_paths(value: &Json) -> Result<Vec<(String, Vec<String>)>, LangUnclassifiable> {
    let Json::Obj(members) = value else {
        return Err(LangUnclassifiable::PathsMalformed);
    };
    let mut out = Vec::new();
    for (key, entry) in members {
        if stars(key) > 1 {
            return Err(LangUnclassifiable::PathsMalformed);
        }
        let Json::Arr(items) = entry else {
            return Err(LangUnclassifiable::PathsMalformed);
        };
        let mut substitutions = Vec::new();
        for item in items {
            let s = item.as_str().ok_or(LangUnclassifiable::PathsMalformed)?;
            if stars(s) > 1 {
                return Err(LangUnclassifiable::PathsMalformed);
            }
            substitutions.push(s.to_string());
        }
        out.push((key.clone(), substitutions));
    }
    Ok(out)
}

fn stars(s: &str) -> usize {
    s.bytes().filter(|b| *b == b'*').count()
}

/// IR §5.2's candidate extension list, "in this exact order".
const EXTENSIONS: [&str; 9] = [
    ".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs", ".json",
];

/// Every import site in a TypeScript or JavaScript file.
pub fn sites(source: &str, path: &str, tree: &dyn Tree, rc: &Rc) -> Vec<ImportSite> {
    // IR §3.1: "`.d.ts`, `.d.mts`, `.d.cts` are type-only by construction. They
    // are TypeScript by extension, they are lexed like any other TypeScript
    // file, but **every import site in them is `type_only`**."
    let declaration_file = lang::is_declaration(path);

    let all = lex::lex(source, Lang::Ts);
    let mut out = Vec::new();

    // IR §5.1: "A `///`-prefixed line comment whose remainder matches
    // `<reference …/>` is a triple-slash directive and is a `type_only` import
    // site (§3.6); it is otherwise a comment." Scanned before the discard.
    for token in &all {
        if token.kind == TokenKind::Comment
            && let Some(rest) = token.text(source).strip_prefix("///")
        {
            let trimmed = rest.trim();
            if trimmed.starts_with("<reference") && trimmed.ends_with("/>") {
                out.push(ImportSite {
                    offset: token.start,
                    disposition: Disposition::TypeOnly,
                });
            }
        }
    }

    let tokens = lex::without_comments(all);
    for i in 0..tokens.len() {
        // "A `word` token `import` **not immediately preceded by** a `.` token"
        // — and the same guard for `export` and `require`, which is what keeps
        // `obj.import`, `a.export` and `require.resolve` out.
        if i > 0 && tokens[i - 1].is_punct(source, b'.') {
            continue;
        }
        let site = if tokens[i].is_word(source, "import") {
            import_site(source, path, tree, rc, &tokens, i)
        } else if tokens[i].is_word(source, "export") {
            export_site(source, path, tree, rc, &tokens, i)
        } else if tokens[i].is_word(source, "require") {
            require_site(source, path, tree, rc, &tokens, i)
        } else {
            None
        };
        if let Some(mut site) = site {
            if declaration_file {
                site.disposition = Disposition::TypeOnly;
            }
            out.push(site);
        }
    }
    out.sort_by_key(|site| site.offset);
    out
}

/// The end of the construct an anchor at `i` belongs to: IR §5.1's own bound,
/// "before the next `;` or `}` at the same bracket depth".
fn construct_end(source: &str, tokens: &[Token], i: usize) -> usize {
    let mut depth = 0i32;
    for (k, token) in tokens.iter().enumerate().skip(i + 1) {
        if token.kind != TokenKind::Punct {
            continue;
        }
        let byte = source.as_bytes()[token.start];
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' => depth -= 1,
            b'}' if depth > 1 => depth -= 1,
            // The `}` that closes the construct's own outermost brace.
            //
            // Depth alone cannot decide here, and reading it as "keep going"
            // was a fail-open: `export enum E { A, B }` has no trailing `;`, so
            // the anchor ran past its own end and terminated on the NEXT
            // statement's `;` — stealing that statement's specifier and
            // resolving it **without** its modifiers. `export enum E { A, B }`
            // followed by `import type { X } from './a';` produced a second,
            // spurious site that froze `a.ts`, which IR §3.6 says a type-only
            // import must never do.
            //
            // What separates the two shapes is the token after the brace:
            // `export { X } from './a';` continues, because `from` is where its
            // specifier lives; `export enum E { … }` and `export function f()
            // {}` end there, because a braced declaration takes no `;`.
            b'}' => {
                depth -= 1;
                let continues = tokens.get(k + 1).is_some_and(|next| {
                    next.kind == TokenKind::Word && &source[next.start..next.end] == "from"
                });
                if !continues {
                    return k;
                }
            }
            b';' if depth <= 0 => return k,
            _ => {}
        }
    }
    tokens.len()
}

/// A `word` token `import` that is not preceded by `.`. IR §5.1: "a *dynamic*
/// one if the next token is `(`, `import.meta` (**not a site**) if the next
/// token is `.`, and a declaration otherwise."
fn import_site(
    source: &str,
    path: &str,
    tree: &dyn Tree,
    rc: &Rc,
    tokens: &[Token],
    i: usize,
) -> Option<ImportSite> {
    let offset = tokens[i].start;
    let next = tokens.get(i + 1)?;

    // Case T13: `import.meta.url` is not an import site.
    if next.is_punct(source, b'.') {
        return None;
    }

    if next.is_punct(source, b'(') {
        // Case T9: "`await import('./x')` | an import site, resolved."
        // Case T10: "`await import(name)` | `unresolvable`, reason
        // `dynamic-import`."
        return Some(ImportSite {
            offset,
            disposition: match tokens.get(i + 2).and_then(|t| t.simple_literal(source)) {
                Some(spec) if tokens.get(i + 3).is_some_and(|t| t.is_punct(source, b')')) => {
                    resolve(spec, path, tree, rc)
                }
                _ => Disposition::Unresolvable(Unresolvable::DynamicImport),
            },
        });
    }

    let end = construct_end(source, tokens, i);

    // IR §3.6's type-only forms. `import type from 's'` binds a default named
    // `type` and is an ordinary import, so the token after `type` decides.
    if next.is_word(source, "type") && !tokens.get(i + 2).is_some_and(|t| t.is_word(source, "from"))
    {
        return Some(ImportSite {
            offset,
            disposition: Disposition::TypeOnly,
        });
    }

    // "`import 's'` | side-effect import — a real edge; a setup file is
    // imported this way."
    if matches!(next.kind, TokenKind::Str(_)) {
        return Some(ImportSite {
            offset,
            disposition: specifier_disposition(source, next, path, tree, rc),
        });
    }

    // Case T6: "`import { type A, b } from './x'` | an ordinary import site —
    // `b` is a value." IR §3.6: type-only "**only if every** named specifier
    // carries the inline `type` modifier".
    if next.is_punct(source, b'{') && all_named_specifiers_are_type_only(source, tokens, i + 1, end)
    {
        return Some(ImportSite {
            offset,
            disposition: Disposition::TypeOnly,
        });
    }

    // "`import x = require('s')` | value import (TypeScript import-equals)".
    // The `require` inside it is handled here rather than by `require_site`,
    // which would otherwise report the same bytes twice.
    if let Some(site) = from_specifier(source, path, tree, rc, tokens, i, end, offset) {
        return Some(site);
    }
    None
}

/// A re-export. IR §5.1: "A `word` token `export` not preceded by `.` begins a
/// re-export site **iff** a `from` word token followed by a simple string
/// literal occurs before the next `;` or `}` at the same bracket depth."
fn export_site(
    source: &str,
    path: &str,
    tree: &dyn Tree,
    rc: &Rc,
    tokens: &[Token],
    i: usize,
) -> Option<ImportSite> {
    let offset = tokens[i].start;
    let end = construct_end(source, tokens, i);
    // The `from` clause is what makes it a site at all, so find it first: an
    // `export type Foo = Bar` names no module and is no site.
    let site = from_specifier(source, path, tree, rc, tokens, i, end, offset)?;

    // IR §3.5: "`export type { … } from 's'` and `export { type A } from 's'`
    // are `type_only` (§3.6)." Case T8: `export type * from './x'`.
    let type_only = tokens.get(i + 1).is_some_and(|t| t.is_word(source, "type"))
        || (tokens.get(i + 1).is_some_and(|t| t.is_punct(source, b'{'))
            && all_named_specifiers_are_type_only(source, tokens, i + 1, end));
    if type_only {
        return Some(ImportSite {
            offset,
            disposition: Disposition::TypeOnly,
        });
    }
    Some(site)
}

/// IR §5.1: "A `word` token `require` not preceded by `.` and **immediately
/// followed by** `(` is a CommonJS import site."
///
/// Case T12: "`require.resolve('./x')` | **not** an import site" — the `.`
/// after `require` is what excludes it, and "it returns a path and executes
/// nothing" (IR §5.2).
fn require_site(
    source: &str,
    path: &str,
    tree: &dyn Tree,
    rc: &Rc,
    tokens: &[Token],
    i: usize,
) -> Option<ImportSite> {
    if !tokens.get(i + 1)?.is_punct(source, b'(') {
        return None;
    }
    // An `import x = require('s')` was already reported at its `import`; a
    // second site here would double-count one occurrence.
    if i >= 2 && tokens[i - 1].is_punct(source, b'=') && tokens[i - 2].kind == TokenKind::Word {
        let looks_like_import_equals = i >= 3 && tokens[i - 3].is_word(source, "import");
        if looks_like_import_equals {
            return None;
        }
    }
    Some(ImportSite {
        offset: tokens[i].start,
        disposition: match tokens.get(i + 2).and_then(|t| t.simple_literal(source)) {
            Some(spec) if tokens.get(i + 3).is_some_and(|t| t.is_punct(source, b')')) => {
                resolve(spec, path, tree, rc)
            }
            _ => Disposition::Unresolvable(Unresolvable::DynamicImport),
        },
    })
}

/// Find the specifier a declaration names — after `from`, or inside a
/// `require(…)` of an import-equals — and resolve it.
#[allow(clippy::too_many_arguments)]
fn from_specifier(
    source: &str,
    path: &str,
    tree: &dyn Tree,
    rc: &Rc,
    tokens: &[Token],
    i: usize,
    end: usize,
    offset: usize,
) -> Option<ImportSite> {
    for k in i + 1..end.min(tokens.len()) {
        let is_from = tokens[k].is_word(source, "from");
        let is_require = tokens[k].is_word(source, "require")
            && tokens.get(k + 1).is_some_and(|t| t.is_punct(source, b'('));
        if !is_from && !is_require {
            continue;
        }
        let literal_at = if is_from { k + 1 } else { k + 2 };
        let literal = tokens.get(literal_at)?;
        if !matches!(literal.kind, TokenKind::Str(_)) {
            continue;
        }
        return Some(ImportSite {
            offset,
            disposition: specifier_disposition(source, literal, path, tree, rc),
        });
    }
    None
}

/// A specifier token's disposition, honouring §3.4 rule 5's simplicity rule.
///
/// **DERIVED, and reported.** Case T20 requires ``import `./x` `` — a template
/// literal with no substitution — to be `unresolvable`, but IR §5.7's closed
/// TypeScript site list carries no `non-simple-literal` reason (Dart's §6.7
/// does). The token used is `dynamic-import`, on §5.2's own reading of the
/// word: "'Dynamic' is read as *the specifier is not statically determined*."
fn specifier_disposition(
    source: &str,
    literal: &Token,
    path: &str,
    tree: &dyn Tree,
    rc: &Rc,
) -> Disposition {
    match literal.simple_literal(source) {
        Some(spec) => resolve(spec, path, tree, rc),
        None => Disposition::Unresolvable(Unresolvable::DynamicImport),
    }
}

/// IR §3.6: "`import { type A, type B } from 's'` — `type_only` **only if every**
/// named specifier carries the inline `type` modifier."
///
/// `{ type }` imports a binding *named* `type`, and `{ type as t }` renames it,
/// so a group is type-modified only when a second `word` follows and it is not
/// `as`.
fn all_named_specifiers_are_type_only(
    source: &str,
    tokens: &[Token],
    brace: usize,
    end: usize,
) -> bool {
    let close = (brace..end.min(tokens.len())).find(|k| tokens[*k].is_punct(source, b'}'));
    let Some(close) = close else {
        return false;
    };
    let inner = &tokens[brace + 1..close];
    if inner.is_empty() {
        // `import {} from 's'` has no value binding, but it is also not one of
        // §3.6's closed forms. It stays an ordinary site.
        return false;
    }
    inner
        .split(|t| t.is_punct(source, b','))
        .filter(|group| !group.is_empty())
        .all(|group| {
            group[0].is_word(source, "type")
                && group
                    .get(1)
                    .is_some_and(|t| t.kind == TokenKind::Word && !t.is_word(source, "as"))
        })
}

/// IR §5.2's four-way split on the specifier's first bytes.
pub fn resolve(spec: &str, path: &str, tree: &dyn Tree, rc: &Rc) -> Disposition {
    // 1. relative
    if spec.starts_with("./") || spec.starts_with("../") || spec == "." || spec == ".." {
        let joined = tree::join(tree::dirname(path), spec);
        let Some(base) = tree::normalize(&joined) else {
            return Disposition::Unresolvable(Unresolvable::RelativeEscapesRoot);
        };
        return expand(&base, tree);
    }
    // 2. "an absolute filesystem path is environment-dependent"
    if spec.starts_with('/') {
        return Disposition::Unresolvable(Unresolvable::AbsoluteSpecifier);
    }
    // 3. "a `package.json` `imports` subpath; v1 reads no `exports`/`imports`
    //    map, §18 OPEN-7"
    if spec.starts_with('#') {
        return Disposition::Unresolvable(Unresolvable::SubpathImports);
    }
    // 4. bare — consult the alias table.
    let Some(substitutions) = alias(rc, spec) else {
        // Case T16: "`lodash` with no matching alias | `external`."
        return Disposition::External;
    };
    for substitution in substitutions {
        let joined = tree::join(&rc.paths_base, &substitution);
        let Some(base) = tree::normalize(&joined) else {
            // A substitution escaping the root contributes no candidate; the
            // alias can still dead-end, which is the fail-closed answer.
            continue;
        };
        match expand(&base, tree) {
            Disposition::Unresolvable(Unresolvable::NoCandidate) => continue,
            other => return other,
        }
    }
    // Case T15: "`@shared/nope` with the same alias and no such file |
    // `unresolvable`, reason `alias-dead-end` — **not** `external`."
    Disposition::Unresolvable(Unresolvable::AliasDeadEnd)
}

/// IR §5.3's alias matching. "Where several keys match, the one with the
/// **longest literal prefix before its `*`** wins; ties are impossible because
/// two distinct keys cannot have equal literal prefixes and equal suffixes."
fn alias(rc: &Rc, spec: &str) -> Option<Vec<String>> {
    let mut best: Option<(usize, Vec<String>)> = None;
    for (key, substitutions) in &rc.paths {
        let (prefix, capture) = match key.split_once('*') {
            None => {
                if key == spec {
                    (key.len(), String::new())
                } else {
                    continue;
                }
            }
            Some((prefix, suffix)) => {
                if !(spec.starts_with(prefix)
                    && spec.ends_with(suffix)
                    && spec.len() >= prefix.len() + suffix.len())
                {
                    continue;
                }
                (
                    prefix.len(),
                    spec[prefix.len()..spec.len() - suffix.len()].to_string(),
                )
            }
        };
        let substituted: Vec<String> = substitutions
            .iter()
            .map(|s| s.replacen('*', &capture, 1))
            .collect();
        if best.as_ref().is_none_or(|(len, _)| prefix > *len) {
            best = Some((prefix, substituted));
        }
    }
    best.map(|(_, substituted)| substituted)
}

/// IR §5.2 step 5 — "candidate expansion for a base path `Bp`, **first match
/// wins over the whole ordered list**".
fn expand(base: &str, tree: &dyn Tree) -> Disposition {
    let mut candidates: Vec<String> = Vec::new();
    // 1. "`Bp` itself, if it is an existing file entry."
    candidates.push(base.to_string());
    // 2. "**The TypeScript output-extension rewrite**, when `Bp` ends in a
    //    JavaScript extension: `.js` → `.ts`, `.tsx`; `.mjs` → `.mts`; `.cjs` →
    //    `.cts`. (`import './x.js'` in a TypeScript file names `x.ts`.)"
    for (from, to) in [
        (".js", &[".ts", ".tsx"][..]),
        (".mjs", &[".mts"][..]),
        (".cjs", &[".cts"][..]),
    ] {
        if let Some(stem) = base.strip_suffix(from) {
            for ext in to {
                candidates.push(format!("{stem}{ext}"));
            }
        }
    }
    // 3. "`Bp + ext` for `ext` in this exact order".
    for ext in EXTENSIONS {
        candidates.push(format!("{base}{ext}"));
    }
    // 4. "If `Bp` is an existing directory entry: `Bp + "/index" + ext` for
    //    `ext` in the same order as step 3."
    if tree.is_dir(base) {
        for ext in EXTENSIONS {
            candidates.push(format!("{base}/index{ext}"));
        }
    }

    // "A candidate that is a `.d.ts`/`.d.mts`/`.d.cts` file is **skipped** in
    // steps 1–4 rather than matched … If every candidate is a declaration file,
    // the site is `type_only`."
    let mut saw_declaration = false;
    for candidate in candidates {
        if let Some(reason) = tree.refuses_to_follow(&candidate) {
            return Disposition::Unresolvable(reason);
        }
        if !tree.is_file(&candidate) {
            continue;
        }
        if lang::is_declaration(&candidate) {
            saw_declaration = true;
            continue;
        }
        return Disposition::Repo(vec![candidate]);
    }
    if saw_declaration {
        return Disposition::TypeOnly;
    }
    // 5. "otherwise → `unresolvable` (reason `no-candidate`)."
    Disposition::Unresolvable(Unresolvable::NoCandidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::MapTree;

    fn only(source: &str, path: &str, tree: &MapTree, rc: &Rc) -> Disposition {
        let found = sites(source, path, tree, rc);
        assert_eq!(found.len(), 1, "expected one site, got {found:?}");
        found.into_iter().next().unwrap().disposition
    }

    fn repo(path: &str) -> Disposition {
        Disposition::Repo(vec![path.to_string()])
    }

    /// Case T1: "`import './x.js'` where `x.ts` exists and `x.js` does not |
    /// resolves to `x.ts`."
    #[test]
    fn t1_the_output_extension_rewrite_names_the_source_file() {
        let tree = MapTree::new([("x.ts", ""), ("t.ts", "")]);
        assert_eq!(
            only("import './x.js';\n", "t.ts", &tree, &Rc::default()),
            repo("x.ts")
        );
    }

    /// Case T2: "`import './x'` where both `x.ts` and `x.js` exist | `x.ts`
    /// (extension order)."
    #[test]
    fn t2_the_extension_order_decides_and_needs_no_ambiguity_rule() {
        let tree = MapTree::new([("x.ts", ""), ("x.js", ""), ("t.ts", "")]);
        assert_eq!(
            only("import './x';\n", "t.ts", &tree, &Rc::default()),
            repo("x.ts")
        );
    }

    /// Case T3: "`import './dir'` where `dir/index.ts` exists | resolves to
    /// `dir/index.ts`."
    #[test]
    fn t3_a_directory_resolves_through_index() {
        let tree = MapTree::new([("dir/index.ts", ""), ("t.ts", "")]);
        assert_eq!(
            only("import './dir';\n", "t.ts", &tree, &Rc::default()),
            repo("dir/index.ts")
        );
    }

    /// Case T4: "`import './dir'` where `dir/package.json` has
    /// `\"main\":\"lib.js\"` and no `index.*` exists | `unresolvable`, reason
    /// `no-candidate`; **must not** read `main`."
    #[test]
    fn t4_directory_resolution_reads_no_package_json() {
        let tree = MapTree::new([
            ("dir/package.json", "{\"main\":\"lib.js\"}"),
            ("dir/lib.js", ""),
            ("t.ts", ""),
        ]);
        assert_eq!(
            only("import './dir';\n", "t.ts", &tree, &Rc::default()),
            Disposition::Unresolvable(Unresolvable::NoCandidate)
        );
    }

    /// Cases T5, T6 and T8 — IR §3.6's closed type-only forms.
    #[test]
    fn t5_t6_t8_the_type_only_forms_and_the_one_that_is_not() {
        let tree = MapTree::new([("x.ts", ""), ("t.ts", "")]);
        let rc = Rc::default();
        for source in [
            "import type { A } from './x';\n",
            "import type A from './x';\n",
            "import type * as ns from './x';\n",
            "import { type A, type B } from './x';\n",
            "export type { A } from './x';\n",
            "export type * from './x';\n",
            "export { type A } from './x';\n",
        ] {
            assert_eq!(
                only(source, "t.ts", &tree, &rc),
                Disposition::TypeOnly,
                "{source}"
            );
        }
        // T6: one value binding makes the whole site ordinary.
        assert_eq!(
            only("import { type A, b } from './x';\n", "t.ts", &tree, &rc),
            repo("x.ts")
        );
        // `import type from './x'` binds a default named `type`; it is a value
        // import, and the token after `type` is what distinguishes it.
        assert_eq!(
            only("import type from './x';\n", "t.ts", &tree, &rc),
            repo("x.ts")
        );
    }

    /// Case T7: "`export * from './x'` | an import site (re-export)." IR §3.5:
    /// "re-exports count as imports".
    #[test]
    fn t7_a_re_export_is_an_import_site() {
        let tree = MapTree::new([("x.ts", ""), ("t.ts", "")]);
        for source in [
            "export * from './x';\n",
            "export * as n from './x';\n",
            "export { a } from './x';\n",
            "export { default as d } from './x';\n",
        ] {
            assert_eq!(
                only(source, "t.ts", &tree, &Rc::default()),
                repo("x.ts"),
                "{source}"
            );
        }
        // An `export` with no `from` clause names no module and is no site.
        assert!(sites("export const a = 1;\n", "t.ts", &tree, &Rc::default()).is_empty());
        assert!(sites("export type Foo = Bar;\n", "t.ts", &tree, &Rc::default()).is_empty());
    }

    /// Cases T9 and T10, and IR §5.2's ruling: "**A literal `import('./x')`
    /// resolves rather than tripwiring** … tripwiring idiomatic lazy loading
    /// would put a human in front of every approval that splits a bundle."
    #[test]
    fn t9_and_t10_a_literal_dynamic_import_resolves_and_a_computed_one_does_not() {
        let tree = MapTree::new([("x.ts", ""), ("t.ts", "")]);
        let rc = Rc::default();
        assert_eq!(
            only("await import('./x');\n", "t.ts", &tree, &rc),
            repo("x.ts")
        );
        assert_eq!(
            only("await import(name);\n", "t.ts", &tree, &rc),
            Disposition::Unresolvable(Unresolvable::DynamicImport)
        );
    }

    /// Cases T11, T12 and T13.
    #[test]
    fn t11_t12_t13_require_resolve_and_import_meta() {
        let tree = MapTree::new([("x.ts", ""), ("t.cjs", "")]);
        let rc = Rc::default();
        assert_eq!(only("require('./x');\n", "t.cjs", &tree, &rc), repo("x.ts"));
        // T12: `require.resolve` "returns a path and executes nothing".
        assert!(sites("require.resolve('./x');\n", "t.cjs", &tree, &rc).is_empty());
        // T13: `import.meta.url` is not an import site.
        assert!(sites("const u = import.meta.url;\n", "t.cjs", &tree, &rc).is_empty());
    }

    /// "`import x = require('s')` | value import (TypeScript import-equals)" —
    /// **one** site, not two.
    #[test]
    fn an_import_equals_require_is_one_site_and_not_two() {
        let tree = MapTree::new([("x.ts", ""), ("t.ts", "")]);
        let found = sites(
            "import x = require('./x');\n",
            "t.ts",
            &tree,
            &Rc::default(),
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].disposition, repo("x.ts"));
    }

    /// Case T14: "`@shared/money` with `paths: {\"@shared/*\": [\"src/shared/*\"]}`
    /// and `baseUrl: \".\"` | resolves to `src/shared/money.ts`."
    #[test]
    fn t14_an_alias_substitutes_and_resolves_through_the_candidate_list() {
        let tree = MapTree::new([
            (
                "tsconfig.json",
                r#"{"compilerOptions":{"baseUrl":".","paths":{"@shared/*":["src/shared/*"]}}}"#,
            ),
            ("src/shared/money.ts", ""),
            ("t.ts", ""),
        ]);
        let rc = rc(&tree).expect("legal tsconfig");
        assert_eq!(rc.base_url.as_deref(), Some(""));
        assert_eq!(
            only("import { m } from '@shared/money';\n", "t.ts", &tree, &rc),
            repo("src/shared/money.ts")
        );
    }

    /// Cases T15 and T16: a matched alias that resolves nothing is
    /// `alias-dead-end`, and an unmatched bare name is `external`.
    #[test]
    fn t15_and_t16_a_dead_alias_is_not_external() {
        let tree = MapTree::new([
            (
                "tsconfig.json",
                r#"{"compilerOptions":{"baseUrl":".","paths":{"@shared/*":["src/shared/*"]}}}"#,
            ),
            ("src/shared/money.ts", ""),
            ("t.ts", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only("import '@shared/nope';\n", "t.ts", &tree, &rc),
            Disposition::Unresolvable(Unresolvable::AliasDeadEnd)
        );
        assert_eq!(
            only("import 'lodash';\n", "t.ts", &tree, &rc),
            Disposition::External
        );
    }

    /// Cases T17 and the absolute form of §5.2 step 2.
    #[test]
    fn t17_a_subpath_import_and_an_absolute_specifier_are_each_their_own_reason() {
        let tree = MapTree::new([("t.ts", "")]);
        let rc = Rc::default();
        assert_eq!(
            only("import '#internal/x';\n", "t.ts", &tree, &rc),
            Disposition::Unresolvable(Unresolvable::SubpathImports)
        );
        assert_eq!(
            only("import '/etc/x';\n", "t.ts", &tree, &rc),
            Disposition::Unresolvable(Unresolvable::AbsoluteSpecifier)
        );
    }

    /// Case T18: "`import './x.json'` where `x.json` exists | an import site
    /// resolving to the JSON file."
    #[test]
    fn t18_a_json_import_resolves() {
        let tree = MapTree::new([("x.json", "{}"), ("t.ts", "")]);
        assert_eq!(
            only(
                "import data from './x.json';\n",
                "t.ts",
                &tree,
                &Rc::default()
            ),
            repo("x.json")
        );
    }

    /// Case T19: "A specifier resolving only to `x.d.ts` | `type_only`."
    ///
    /// **DEFECT (IR §5.2), reported.** The candidate list of step 3 carries no
    /// declaration extension, so `import './types'` over a tree holding only
    /// `types.d.ts` is `no-candidate` and never reaches this row. T19 is
    /// therefore reachable only through step 1 or the step 2 rewrite, which is
    /// what this test uses.
    #[test]
    fn t19_a_specifier_resolving_only_to_a_declaration_file_is_type_only() {
        let tree = MapTree::new([("x.d.ts", ""), ("t.ts", "")]);
        let rc = Rc::default();
        assert_eq!(
            only("import type { A } from './x.d.ts';\n", "t.ts", &tree, &rc),
            Disposition::TypeOnly
        );
        assert_eq!(
            only("import { a } from './x.d.ts';\n", "t.ts", &tree, &rc),
            Disposition::TypeOnly
        );
        // The residual: the ordinary spelling is `no-candidate`.
        let tree = MapTree::new([("types.d.ts", ""), ("t.ts", "")]);
        assert_eq!(
            only("import { a } from './types';\n", "t.ts", &tree, &rc),
            Disposition::Unresolvable(Unresolvable::NoCandidate)
        );
    }

    /// Case T20: "`` import `./x` `` (template literal, no substitution) |
    /// `unresolvable` — not a simple literal (§3.4 rule 5)."
    #[test]
    fn t20_a_template_literal_specifier_is_never_simple() {
        let tree = MapTree::new([("x.ts", ""), ("t.ts", "")]);
        assert_eq!(
            only("import `./x`;\n", "t.ts", &tree, &Rc::default()),
            Disposition::Unresolvable(Unresolvable::DynamicImport)
        );
    }

    /// Case T21: "`extends: \"@company/tsconfig\"` in the root tsconfig |
    /// `lang-unclassifiable`, reason `tsconfig-extends-external`."
    #[test]
    fn t21_a_bare_or_absolute_or_array_extends_is_external() {
        for value in [
            "\"@company/tsconfig\"",
            "\"/abs/tsconfig.json\"",
            "[\"./a\"]",
        ] {
            let tree = MapTree::new([("tsconfig.json", format!("{{\"extends\":{value}}}"))]);
            assert_eq!(
                rc(&tree).unwrap_err(),
                LangUnclassifiable::TsconfigExtendsExternal,
                "{value}"
            );
        }
    }

    /// Case T22, end to end: the regex did not open a comment, so the import on
    /// the later line is found.
    #[test]
    fn t22_a_regex_does_not_swallow_the_import_that_follows_it() {
        let tree = MapTree::new([("x.ts", ""), ("t.ts", "")]);
        let source = "const re = /a\\/\\/b/;\nimport { x } from './x';\n";
        assert_eq!(only(source, "t.ts", &tree, &Rc::default()), repo("x.ts"));
    }

    /// IR §5.3 step 2: "`extends` is followed only for a value that is a simple
    /// string beginning `./` or `../` … with the extension `.json` appended if
    /// absent. **Child keys override parent keys.**"
    #[test]
    fn an_extends_chain_resolves_and_the_child_overrides_the_parent() {
        let tree = MapTree::new([
            (
                "tsconfig.json",
                r#"{"extends":"./config/base","compilerOptions":{"paths":{"@app/*":["src/app/*"]}}}"#,
            ),
            (
                "config/base.json",
                r#"{"compilerOptions":{"baseUrl":"..","paths":{"@old/*":["old/*"]}}}"#,
            ),
            ("src/app/x.ts", ""),
            ("t.ts", ""),
        ]);
        let rc = rc(&tree).expect("legal chain");
        // `baseUrl: ".."` in `config/base.json` resolves against that file's
        // own directory, giving the repository root.
        assert_eq!(rc.base_url.as_deref(), Some(""));
        // The child's `paths` replaced the parent's.
        assert_eq!(
            rc.paths,
            vec![("@app/*".to_string(), vec!["src/app/*".to_string()])]
        );
        assert_eq!(
            only("import '@app/x';\n", "t.ts", &tree, &rc),
            repo("src/app/x.ts")
        );
    }

    /// IR §5.3 step 2: "A cycle → unclassifiable, reason
    /// `tsconfig-extends-cycle`."
    #[test]
    fn an_extends_cycle_is_its_own_reason_and_never_a_hang() {
        let tree = MapTree::new([
            ("tsconfig.json", r#"{"extends":"./a.json"}"#),
            ("a.json", r#"{"extends":"./tsconfig.json"}"#),
        ]);
        assert_eq!(
            rc(&tree).unwrap_err(),
            LangUnclassifiable::TsconfigExtendsCycle
        );
    }

    /// IR §5.3 steps 3 and 4's refusals.
    #[test]
    fn a_bad_base_url_or_paths_shape_is_its_own_reason() {
        let escaping = MapTree::new([(
            "tsconfig.json",
            r#"{"compilerOptions":{"baseUrl":"../outside"}}"#,
        )]);
        assert_eq!(
            rc(&escaping).unwrap_err(),
            LangUnclassifiable::BaseurlEscapesRoot
        );

        for paths in [
            r#"{"@a/*":"src/a/*"}"#,     // value is not an array
            r#"{"@a/*":[1]}"#,           // array item is not a string
            r#"{"@a/*/*":["src/a/*"]}"#, // two stars in the key
            r#"{"@a/*":["src/*/*"]}"#,   // two stars in a substitution
        ] {
            let tree = MapTree::new([(
                "tsconfig.json",
                format!(r#"{{"compilerOptions":{{"paths":{paths}}}}}"#),
            )]);
            assert_eq!(
                rc(&tree).unwrap_err(),
                LangUnclassifiable::PathsMalformed,
                "{paths}"
            );
        }

        let broken = MapTree::new([("tsconfig.json", "{ not json")]);
        assert_eq!(
            rc(&broken).unwrap_err(),
            LangUnclassifiable::TsconfigUnparseable
        );
    }

    /// IR §5.3: "If no `tsconfig.json` and no `jsconfig.json` exists at the
    /// repository root, `RC` is `(none, [])` — **legal, not unclassifiable**."
    #[test]
    fn a_repository_with_no_config_has_a_legal_empty_rc() {
        let tree = MapTree::new([("src/a.ts", "")]);
        assert_eq!(rc(&tree).unwrap(), Rc::default());
        // …and `jsconfig.json` is consulted only when no `tsconfig.json` is
        // there.
        let both = MapTree::new([
            ("tsconfig.json", r#"{"compilerOptions":{"baseUrl":"src"}}"#),
            ("jsconfig.json", r#"{"compilerOptions":{"baseUrl":"js"}}"#),
        ]);
        assert_eq!(rc(&both).unwrap().base_url.as_deref(), Some("src"));
    }

    /// IR §5.3: "Where several keys match, the one with the **longest literal
    /// prefix before its `*`** wins."
    #[test]
    fn the_longest_literal_prefix_wins_among_matching_aliases() {
        let tree = MapTree::new([
            (
                "tsconfig.json",
                r#"{"compilerOptions":{"paths":{"@a/*":["wrong/*"],"@a/b/*":["right/*"]}}}"#,
            ),
            ("wrong/c.ts", ""),
            ("right/c.ts", ""),
            ("t.ts", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only("import '@a/b/c';\n", "t.ts", &tree, &rc),
            repo("right/c.ts")
        );
    }

    /// IR §5.2 step 4: "each of its substituted candidate base paths goes to
    /// step 5 **in the table's order** and the **first** that resolves wins".
    #[test]
    fn substitutions_are_tried_in_order_and_the_first_that_resolves_wins() {
        let tree = MapTree::new([
            (
                "tsconfig.json",
                r#"{"compilerOptions":{"paths":{"@x/*":["missing/*","found/*"]}}}"#,
            ),
            ("found/y.ts", ""),
            ("t.ts", ""),
        ]);
        let rc = rc(&tree).unwrap();
        assert_eq!(
            only("import '@x/y';\n", "t.ts", &tree, &rc),
            repo("found/y.ts")
        );
    }

    /// IR §5.1: a triple-slash reference directive "is a `type_only` import
    /// site (§3.6); it is otherwise a comment."
    #[test]
    fn a_triple_slash_reference_directive_is_a_type_only_site() {
        let tree = MapTree::new([("t.ts", "")]);
        let rc = Rc::default();
        assert_eq!(
            only("/// <reference types=\"node\" />\n", "t.ts", &tree, &rc),
            Disposition::TypeOnly
        );
        assert!(sites("/// an ordinary comment\n", "t.ts", &tree, &rc).is_empty());
    }

    /// IR §3.1: "every import site in them is `type_only`" — a declaration file
    /// contains no runtime code.
    #[test]
    fn every_site_in_a_declaration_file_is_type_only() {
        let tree = MapTree::new([("x.ts", ""), ("types/t.d.ts", "")]);
        assert_eq!(
            only(
                "import { a } from '../x';\n",
                "types/t.d.ts",
                &tree,
                &Rc::default()
            ),
            Disposition::TypeOnly
        );
    }

    /// IR §3.4 rule 7: "An import inside a function, a class, a `try`, an `if`
    /// … is an import site. There is no 'top-level only' rule anywhere in this
    /// document."
    #[test]
    fn a_nested_dynamic_import_is_a_site() {
        let tree = MapTree::new([("x.ts", ""), ("t.ts", "")]);
        let source = "function f() {\n  if (c) { return import('./x'); }\n}\n";
        assert_eq!(only(source, "t.ts", &tree, &Rc::default()), repo("x.ts"));
    }

    /// IR §5.2 step 1 and case C15's neighbour.
    #[test]
    fn a_relative_specifier_escaping_the_root_is_refused_and_never_clamped() {
        let tree = MapTree::new([("src/t.ts", "")]);
        assert_eq!(
            only(
                "import '../../etc/passwd';\n",
                "src/t.ts",
                &tree,
                &Rc::default()
            ),
            Disposition::Unresolvable(Unresolvable::RelativeEscapesRoot)
        );
    }

    /// "`import 's'` | side-effect import — a real edge; a setup file is
    /// imported this way." IR §5.4 depends on it: "`vitest.config.ts`'s own
    /// `import './vitest.setup.ts'` pulls the setup file into the closure."
    #[test]
    fn a_side_effect_import_is_a_real_edge() {
        let tree = MapTree::new([("vitest.setup.ts", ""), ("vitest.config.ts", "")]);
        assert_eq!(
            only(
                "import './vitest.setup.ts';\n",
                "vitest.config.ts",
                &tree,
                &Rc::default()
            ),
            repo("vitest.setup.ts")
        );
    }

    /// IR §5.1 bounds an export clause at "the next `;` or `}` at the same
    /// bracket depth", and IR §3.6 makes a type-only import contribute no edge
    /// and never freeze its target.
    ///
    /// A braced export declaration takes no trailing `;`, so an anchor that
    /// kept scanning past its own `}` terminated on the NEXT statement's `;` —
    /// producing a second site that resolved that statement's specifier
    /// **without** its `type` modifier, and froze a file §3.6 forbids freezing.
    #[test]
    fn a_braced_export_declaration_does_not_steal_the_next_statements_specifier() {
        let tree = MapTree::new([
            ("package.json", "{}"),
            ("src/a.ts", ""),
            ("src/main.ts", ""),
        ]);
        for source in [
            "export enum E { A, B }\nimport type { X } from './a';\n",
            "export function f() {}\nimport type { X } from './a';\n",
            "export interface I { a: 1 }\nimport type { X } from './a';\n",
        ] {
            let found = sites(source, "src/main.ts", &tree, &rc(&tree).unwrap());
            assert_eq!(
                found.len(),
                1,
                "one site — the type-only import — for {source:?}, got {found:?}"
            );
            assert_eq!(
                found[0].disposition,
                Disposition::TypeOnly,
                "and it keeps its modifier: {source:?}"
            );
        }
    }

    /// And a re-export still reaches its specifier, so the fix did not close
    /// the form it was distinguishing from.
    #[test]
    fn a_re_export_still_reads_the_specifier_after_its_closing_brace() {
        let tree = MapTree::new([
            ("package.json", "{}"),
            ("src/a.ts", ""),
            ("src/main.ts", ""),
        ]);
        let found = sites(
            "export { X } from './a';\n",
            "src/main.ts",
            &tree,
            &rc(&tree).unwrap(),
        );
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].disposition,
            Disposition::Repo(vec!["src/a.ts".to_string()])
        );
    }
}
