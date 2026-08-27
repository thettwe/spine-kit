# Sheet 13 — Id grammars, the four resolvers, the runner adapters, and the pragma join

Scope: everything an implementer needs to (a) decide `lang(path)`, (b) lex and resolve imports in
Python / TypeScript-JavaScript / Dart / Swift, (c) extract each language's resolution configuration
`RC`, (d) compute the freeze closure's **lexical seed set**, (e) drive the four runner adapters —
their tokens, id grammars, `id → fn`, `id → path`, outcome mappings, `B` enumeration and `B`
outcome commands — and (f) compute the `@verifies` pragma join, the `AC<n>` naming sugar, and the
`C-T3` predicate. The glob dialect these all match against is included in full.

Citation convention used throughout: `(IR §x.y)` = `docs/spec/import-resolver.md`,
`(ID §x.y)` = `docs/spec/intent-doc.md`, `(RF §x.y)` = `docs/spec/result-file.md`,
`(CN §x.y)` = `docs/spec/constitution.md`, `(PB §x)` = `PLAYBOOK.md`,
`(DM §x)` = `docs/spec/dump.md`, `(README)` = `docs/spec/README.md`.

**Precedence rule applied here:** where PB and a spec disagree, PB §11 (Vocabulary) wins; otherwise
the spec is normative and resolves PB's ambiguity (README, "Where prose here and the playbook's §11
disagree, §11 still wins"). Every disagreement found is in §Contradictions.

---

## Sources read

| File | Lines | Sections |
|---|---|---|
| `/Users/thettwe/Works/spine-kit/docs/spec/import-resolver.md` | 68–100 | §2.1.1 the seed set `S`, §2.2 two trees |
| " | 109–160 | §2.4 pattern dialect, §2.4.1 why a pointer, §2.4.2 the vector |
| " | 264–297 | §2.11 findings (tripwires/counters), §2.12 determinism rules |
| " | 297–416 | §3 resolver contract: §3.1 `lang`, §3.2 dispositions, §3.3 `RC`, §3.4 lexing, §3.5 re-exports, §3.6 type-only, §3.7 union rule, §3.8 unclassifiable ladder |
| " | 416–506 | §4 Python (§4.1–§4.7) |
| " | 506–612 | §5 TypeScript/JavaScript (§5.1–§5.7) |
| " | 612–692 | §6 Dart (§6.1–§6.7) |
| " | 692–812 | §7 Swift (§7.1–§7.8), incl. §7.3 `RC(swift, tree)` and `mixed-objc-target`; §8 Kotlin withdrawn |
| " | 812–851 | §9 judging the provisional decision, §10 per-language guarantees |
| " | 851–1078 | §11 runner adapters: §11.1 the four tokens + reserved, §11.2 pytest, §11.3 vitest, §11.4 dart-test, §11.5 swift-test, §11.6 shared rules |
| " | 1078–1190 | §11.7 how §11.2–§11.5 were ratified (reproduced toolchain output) |
| " | 1190–1320 | §12 the lexical reads: §12.1 pragma, §12.2 file-granular join, §12.3 naming sugar, §12.4/§12.4.1/§12.4.2/§12.4.3 `C-T3` |
| " | 1320–1571 | §13 worked examples 13.1–13.4 with published `closure_digest` values |
| " | 1571–1790 | §14 conformance cases (C, T, P, T-ts, D, S, R, J), §15 determinism rules collected |
| " | 1825–1960 | §16.7–§16.12 resolved ambiguities, §17 defects D1–D12, §18 OPEN-1…OPEN-13, §19 out of scope |
| `/Users/thettwe/Works/spine-kit/docs/spec/intent-doc.md` | 469–605 | §6 touchpoint patterns: §6.1 byte grammar, §6.2 glob dialect, §6.3 `match(P,p)`, §6.4 what is matched, §6.5 case sensitivity |
| `/Users/thettwe/Works/spine-kit/docs/spec/result-file.md` | 115–200 | §4.4 record kinds (`base`/`result`/`end`, runner token grammar), §4.5 ordering, §5 outcome vocabulary |
| " | 201–300 | §6.1–§6.7 runners, ids, roll-up, the six adapter obligations |
| `/Users/thettwe/Works/spine-kit/docs/spec/constitution.md` | 535–553 | §6.4 `C-T1`/`C-T2` as functions of `params.langs` |
| `/Users/thettwe/Works/spine-kit/PLAYBOOK.md` | 311–346 | §4.3 test immutability, the closure clauses, G8, `C-T3` |
| " | 569–690 | §6.2 graph schema and derivation table, §6.3 gates as queries (G1/G5/G8 rows) |
| " | 983–1040 | §11 Vocabulary (`Spine-Test`, `Spine-Frozen`, trailers, CLI) |
| `/Users/thettwe/Works/spine-kit/docs/spec/README.md` | 1–88 | status table, settled owner decisions, published digests, known gaps |

---

## Data model

### D-1 `lang` (IR §3.1)

Total function on a repository path; decided **byte-exactly on the final path component, lowercase
only**. `.PY` is not Python.

| Final component ends with | `lang` value | `params.langs` token |
|---|---|---|
| `.py` | Python | `python` |
| `.ts` `.tsx` `.mts` `.cts` `.js` `.jsx` `.mjs` `.cjs` | TypeScript/JavaScript | `ts` |
| `.dart` | Dart | `dart` |
| `.swift` | Swift | `swift` |
| anything else | `none` | — |

Domain of `params.langs`: exactly `{python, ts, dart, swift}` (IR §3.1; CN §6.4; RF §6.4).
Default: none — `spine init` refuses when it detects none (PB §11 CLI).
Overrides: `.d.ts` / `.d.mts` / `.d.cts` are TypeScript by extension but **every import site in them
is `type_only`** and they are never a value-import resolution target (IR §3.1).

### D-2 Import site (IR §3.2)

| Field | Type | Domain | Required |
|---|---|---|---|
| `path` | repo path | any tree entry | yes |
| `offset` | integer | byte offset of the first token of the site | yes (site identity is `(path, offset)`) |
| `disposition` | enum | `repo(m…)` \| `external` \| `type_only` \| `unresolvable` | yes |
| `targets` | set of repo paths | non-empty only when `disposition = repo` | conditional |
| `reason` | token | per-language closed list (D-9) | only when `unresolvable` |
| `specifier` | bytes as written, or `<dynamic>` | reported in the `unresolvable-import` finding | conditional |

`repo(m)` yields an edge and no finding; `external`, `type_only` yield no edge and no finding;
`unresolvable` yields no edge and the finding `unresolvable-import` when `H(path)` is true
(IR §3.2, §2.11). **A site may yield several `repo` targets** — Python ancestor `__init__.py`s, a
Dart conditional import, a Swift `import M` naming every source file of `M`; that is never a reason
to call the site `unresolvable` (IR §3.2).

### D-3 Resolution configuration `RC` (IR §3.3)

| Language | `RC` shape | Default when nothing to read |
|---|---|---|
| `python` | **empty** — Python never raises `lang-unclassifiable` (IR §4.2) | n/a |
| `ts` | pair `(baseUrl, paths)`, `paths` a list of `(pattern, [substitution, …])` **in the file's own key order** (IR §5.3) | `(none, [])` — legal, not unclassifiable |
| `dart` | set of packages, each `(rootDir, name, pathDeps)` (IR §6.3) | empty set |
| `swift` | set of packages, each `(rootDir, [target])`; each target `(name, kind, sourceDirs, sources, exclude, dependencies)` (IR §7.3) | empty set |

Equality is **structural on the extracted value**, not on the file's blob (IR §3.3).

### D-4 Seed set `S` and closure inputs (IR §2.1.1)

| Field | Type | Source |
|---|---|---|
| `A` | tree | the approval tree (branch HEAD at `--approve`; the approval commit's tree in `--ci`) |
| `B` | tree | the commit named by the approve line's `base=` |
| `AC` | set of AC numbers | the signed intent blob, per ID §5.3 (decimal `1 … 6`, no leading zeros) |
| `E` | pattern list | the intent's `expected` touchpoints |
| `C-T1`, `C-T2` | pattern lists | trunk's constitution at `base=` |
| `S` | set of paths in `A` | computed by ALG-E |

### D-5 Pragma occurrence (IR §12.1)

| Field | Type | Notes |
|---|---|---|
| `path` | repo path | the file whose comment carries it |
| `line` | 1-based integer | IR §3.4 rule 2 |
| `intent_id` | `("INT" \| "BUG") "-" numeral` | ID §3.1 domain exactly |
| `ac_digits` | digit run, **captured as written** | `<digit>+`, wider than ID §5.3's `1 … 6` on purpose |

### D-6 Runner adapter (IR §11.1, RF §6.3)

| Field | Type | Domain / value |
|---|---|---|
| `runner` | token | `[a-z][a-z0-9_-]{0,31}` (RF §4.4); one of `pytest`, `vitest`, `dart-test`, `swift-test` |
| `invocation_T` | argv | fixed per adapter (D-7) |
| `enumeration_B` | argv | fixed per adapter (D-7) |
| `outcome_B` | argv | fixed per adapter (D-7) |
| `id → fn` | total fn | output is a **prefix** of input; identity on an unparametrized id |
| `id → path` | total fn | repo-relative `/`-separated; **the empty string** where no tree entry matches |
| outcome mapping | total fn | onto RF §5's eight values, `unknown` the home for anything unmapped |
| terminal event | per adapter | `done` (dart-test), `Test Suite 'All tests' <verb> at …` (swift-test) |

The token is a **constant of the adapter, embedded in the pinned release** — never read from a
stream, a manifest, `params.langs` or the environment (IR §11.6 rule 1; RF §4.4).

### D-7 The four `runner` tokens — ratified (IR §11.1, verbatim table)

| `params.langs` token | `runner` token | Invocation on `T` | `B` enumeration | `B` outcomes | `B` invocations |
|---|---|---|---|---|---|
| `python` | `pytest` | `pytest` | `pytest --collect-only` (§11.2) | `pytest`, the `T` invocation run against the checkout of `B` (§11.2) | **two** |
| `ts` | `vitest` | `vitest run` | `vitest run` (§11.3) | the same run (§11.3) | one |
| `dart` | `dart-test` | `dart test --reporter=json --no-retry` | the same command (§11.4) | the same run (§11.4) | one |
| `swift` | `swift-test` | `swift test --disable-swift-testing` | `swift test list --disable-swift-testing` (§11.5) | `swift test --disable-swift-testing`, the `T` invocation run against the checkout of `B` (§11.5) | **two** |

### D-8 Reserved tokens (IR §11.1)

| Token | Kind | Status | Why |
|---|---|---|---|
| `kotlin` | `params.langs` value | **reserved** | language dropped; refused by RF §7.1 step 3 |
| `gradle` | `runner` | **reserved** | the adapter Kotlin would have used (Appendix A §A.5) |
| `jest` | `runner` | **reserved** | OPEN-4: a second TypeScript adapter is a later release |
| `swift-testing` | `runner` | **not reserved; reservation recommended** | OPEN-8; owner's call |
| `junit`, `kotest` | `runner` | **contested** | RF §6.4 reserves both; IR reserves neither; MF §3.3 says `kotlin` is not reserved at all. OPEN-12 |

**"The hard set is `kotlin`, `gradle`, `jest`."** Those three are emitted by nothing in v1 and no
other adapter may take them (IR §11.1, §20 item 26).

### D-9 Closed unclassifiable / unresolvable reasons, per language

| Language | Language level (`lang-unclassifiable`) | Site level (`unresolvable`) |
|---|---|---|
| Python (IR §4.7) | **none** | `dynamic-import`; relative import escaping root (`relative-escapes-root`); `ambiguous-module`; `symlink-or-submodule`; (file level) `file-not-utf8` |
| TS/JS (IR §5.7) | `tsconfig-unparseable`, `tsconfig-extends-external`, `tsconfig-extends-cycle`, `baseurl-escapes-root`, `paths-malformed`, `rc-changed-on-branch` | `dynamic-import`, `absolute-specifier`, `subpath-imports`, `relative-escapes-root`, `no-candidate`, `alias-dead-end`, `symlink-or-submodule` |
| Dart (IR §6.7) | `pubspec-name-malformed`, `pubspec-not-declarative`, `duplicate-package-name`, `rc-changed-on-branch` | `unsupported-scheme`, `relative-escapes-root`, `no-candidate`, `ambiguous-library-name`, `non-simple-literal`, `symlink-or-submodule` |
| Swift (IR §7.8) | `manifest-not-literal`, `duplicate-target-name`, `xcode-project-unsupported`, `target-dir-missing`, `overlapping-targets`, `mixed-objc-target`, `no-package-manifest`, `rc-changed-on-branch` | `symlink-or-submodule` — "That is the whole list" |

### D-10 Resolver findings (IR §2.11) — kind and effect

| Finding | Kind | Effect |
|---|---|---|
| `unresolvable-import` | tripwire | routes to `approval-review`; `--approve` refuses without a human `reason=` |
| `unresolvable-import-outside-harness` | counter | reported, blocks nothing |
| `lang-unclassifiable` | tripwire | once per **language**, not per file; carries language token + reason |
| `lang-unclassifiable-outside-harness` | counter | every file of that language excluded from the closure |
| `closure-tripwire` | tripwire | carries the sorted list of excluded branch-created paths |
| `closure-too-large` | tripwire | `\|closure\| > 200` |
| `expected-hits-harness` | tripwire | some `expected` entry matches any `C-T1`/`C-T2` pattern |
| `seed-outside-test-roots` | **refusal** | `--approve` refuses outright; carries the sorted list of such paths |
| `no-seed` | tripwire | `S` empty |
| `file-not-utf8` | tripwire if `H`, else counter | file contributes no edges either way |

`spine stats` counters: `closure_size`, `closure_tripwires`, `closure_size_tripwires`,
`unresolvable_imports`, `dynamic_imports`, `unclassifiable_languages`, `excluded_branch_created`,
`frozen_leaves_in_expected`, `seedless_approvals` (IR §2.11).

### D-11 Collector-level refusals introduced by the adapters (IR §11.5, §11.6)

| Finding | Raised by | Effect |
|---|---|---|
| `duplicate-test-id` | any adapter (IR §11.6 rule 2) | collector **fails the job and writes nothing**; carries runner token and id |
| `id-separator-in-path` | pytest, dart-test (`::`), vitest (` > `) (IR §11.6 rule 3) | collector fails the job and writes nothing |
| `ambiguous-test-class` | swift-test (IR §11.5) | collector fails the job and writes nothing |
| `swift-testing-unsupported` | swift-test (IR §11.5) | collector fails the job and writes nothing |

---

## Algorithm

### ALG-A — `lang(path)`

**R1 (MUST)** Compute `lang(path)` by byte-exact, lowercase-only suffix match on the **final path
component**, per D-1 (IR §3.1).
**R2 (MUST NOT)** Case-fold the extension. "`.PY` is not Python; a repository that ships uppercase
extensions gets `none`" (IR §3.1).
**R3 (MUST)** Treat every import site in a `.d.ts` / `.d.mts` / `.d.cts` file as `type_only`, and
skip such files in TS candidate expansion (IR §3.1, §5.2).
**R4 (MUST)** A path whose `lang` is not in `langs` contributes **no edges**, in either tree; it may
still be frozen by clause 3 or clause 4 (IR §3.1).
**R5 (REFUSE)** A `params.langs` naming `kotlin` is refused by RF §7.1 step 3 as a language the
release has no adapter for (IR §3.1, §8).

### ALG-B — Lexical preliminaries shared by all four (IR §3.4)

**R6 (MUST)** Decode every file as UTF-8. A file that is not valid UTF-8 **MUST NOT** be lexed: it
contributes no edges and raises `file-not-utf8` (IR §3.4 rule 1, §2.11).
**R7 (MUST NOT)** Honour any encoding declaration — "not PEP 263's coding cookie, not a BOM, not an
XML declaration" (IR §3.4 rule 1).
**R8 (MUST)** Skip a leading UTF-8 BOM (`EF BB BF`); it is not part of the first token (IR §3.4 r1).
**R9 (MUST)** Treat LF, CRLF and CR each as terminating a line; finding line numbers are 1-based and
count terminators (IR §3.4 rule 2).
**R10 (MUST)** Produce exactly these token kinds: `word` (a maximal run of `[A-Za-z0-9_$]`; `.` is
**never** part of a word — it is punctuation), `string`, `punct` (any other single byte), `comment`
(discarded before matching), `newline` (produced for Python only, discarded elsewhere) (IR §3.4 r3).
**R11 (MUST)** Scan comments for §12.1 pragmas **before** discarding them (IR §3.4 rule 4).
**R12 (MUST)** Treat a string literal as **simple** iff it is a single literal token, contains no
interpolation, and contains **no backslash**. A specifier that is not a simple literal is
`unresolvable` — "including adjacent-literal concatenation (`'pack' 'age:x'` in Dart and Python),
template literals with no substitution (`` `./x` `` in TypeScript), and any literal containing an
escape" (IR §3.4 rule 5).
**R13 (MUST)** Recognize import sites at **any nesting depth**: "An import inside a function, a
class, a `try`, an `if`, a `#if` branch or a lazily-loaded block is an import site. There is no
'top-level only' rule anywhere in this document" (IR §3.4 rule 7).

### ALG-C — Re-exports, type-only, conditionals

**R14 (MUST)** Treat every re-export as an import site with the same dispositions and classification
(IR §3.5; PB §4.3 "re-exports count as imports"). Forms per language:
- Python: none distinct (`from .a import b` is already an import; `__all__` is never read).
- TS/JS: `export * from 's'`, `export * as ns from 's'`, `export { a } from 's'`,
  `export { default as d } from 's'`, `export { a as default } from 's'`.
- Dart: `export 'uri';` with `show`/`hide`. (`part 'uri';` is stronger and is also an edge.)
- Swift: `@_exported import M` — resolves exactly as `import M`.

**R15 (MUST NOT)** Walk a `FROZEN_LEAF` re-exporting module; the modules it republishes are **not**
reached through it (IR §3.5).
**R16 (MUST)** Recognize the closed TypeScript type-only set and only it (IR §3.6):
`import type X from 's'`, `import type { A } from 's'`, `import type * as ns from 's'`,
`export type { A } from 's'`, `export type * from 's'`; `import { type A, type B } from 's'` is
`type_only` **only if every** named specifier carries the inline `type` modifier —
`import { type A, b } from 's'` is a normal import site; `/// <reference path="…" />` and
`/// <reference types="…" />`; and any site whose resolved target is a `.d.ts`/`.d.mts`/`.d.cts`.
**R17 (MUST)** Treat a Python `import` under `if TYPE_CHECKING:` as an **ordinary import site**
(IR §3.6, §16.5). Python, Dart and Swift have no type-only form.
**R18 (MUST)** Apply the **union rule** to every conditional construct: "Every branch of every
conditional construct contributes its import sites, and all of them are resolved. No configuration,
flag set, target platform, build variant, Swift compilation condition or Dart environment
declaration is ever read" (IR §3.7).

### ALG-D — `RC` extraction and the base-tree rule (IR §3.3)

**R19 (MUST)** Compute the closure with `RC(lang, B)` — the configuration comes from the **base
tree** (IR §3.3 Rule 1). PB §4.3's reason governs: *"It is read from the base tree, which the branch
cannot edit."*
**R20 (MUST)** Where `RC(lang, A) ≠ RC(lang, B)` (structural equality on the extracted value), raise
`lang-unclassifiable` with reason `rc-changed-on-branch`; `--approve` refuses without a human
`reason=`, and `spine stats` counts it (IR §3.3 Rule 2). The closure is still computed with
`RC(lang, B)` (IR §14 C18).
**R21 (MUST)** Resolve every existence test, specifier lookup and candidate expansion against **`A`**;
answer the two `base=` questions wholly in **`B`** (IR §2.2).
**R22 (MUST)** Under `lang-unclassifiable`, **every** file of that language contributes no edges and
is never added by an import edge; clause 3 and clause 4 still apply (IR §3.8).

#### ALG-D1 — `RC(python, ·)` (IR §4.2)

**R23 (MUST)** Treat `RC(python, ·)` as **empty**. `pyproject.toml` is **not** consulted for
`package-dir` / `packages` / `sources` (IR §4.2). Python therefore never raises
`lang-unclassifiable`.

#### ALG-D2 — `RC(ts, tree)` (IR §5.3)

Extracted from the repository-root `tsconfig.json`, or `jsconfig.json` if no `tsconfig.json` exists
at the root. Ordered steps:

**R24 (MUST)** Parse the file as **JSON with comments and trailing commas** (the dialect `tsc`
accepts). A file that does not parse in that dialect → unclassifiable, reason `tsconfig-unparseable`.
**R25 (MUST)** Follow `extends` **only** for a value that is a simple string beginning `./` or `../`,
resolved against the extending file's directory, with the extension `.json` appended if absent; child
keys override parent keys. An `extends` naming a bare specifier, an absolute path, or an array →
`tsconfig-extends-external`. A cycle → `tsconfig-extends-cycle`.
**R26 (MUST)** `compilerOptions.baseUrl`, if present, must be a simple string, resolved relative to
the file that declares it and must stay inside the repository, else `baseurl-escapes-root`.
**R27 (MUST)** `compilerOptions.paths` must be an object whose every value is an array of strings,
each containing **at most one `*`**, and whose every key contains at most one `*`, else
`paths-malformed`.
**R28 (MUST NOT)** Read any other key. "`include`, `exclude`, `files`, `references` and
`moduleResolution` are ignored" (IR §5.3 step 5).
**R29 (MUST)** With no root `tsconfig.json` and no `jsconfig.json`, `RC` is `(none, [])` — legal, not
unclassifiable.

#### ALG-D3 — `RC(dart, tree)` (IR §6.3)

**R30 (MUST)** Extract from **every** `pubspec.yaml` in the tree; a `pubspec.yaml` at directory `d`
declares a package rooted at `d`. Its `name:` must be a plain scalar matching `^[a-z_][a-z0-9_]*$`,
else `pubspec-name-malformed`.
**R31 (MUST)** Read the YAML as the **declarative subset**: block mappings, block sequences and plain
or single/double-quoted scalars only. "Anchors (`&`), aliases (`*`), merge keys (`<<`), tags (`!`),
multi-document streams (`---` more than once) and flow mappings that nest more than one level →
unclassifiable, reason `pubspec-not-declarative`."
**R32 (MUST)** Build `pathDeps` from `dependencies:` and `dev_dependencies:`: each entry of the form
`<pkg>: { path: <p> }` contributes `<pkg> → normalize(d + "/" + p)`, **provided the result stays
inside the repository**. A `path:` escaping the root, or a `git:`/`hosted:` dependency, contributes
nothing and **is not an error**.
**R33 (MUST)** Two packages with the same `name` in one repository → `duplicate-package-name`.
**R34 (MUST)** The importing file's package is the one whose `rootDir` is the **longest** prefix of
the file's path. A Dart file under no package root: its `package:` self-references are `external` and
its relative imports still resolve.

#### ALG-D4 — `RC(swift, tree)` (IR §7.3) — the literal manifest subset

Extracted from every `Package.swift`. `RC` is a set of packages, each `(rootDir, [target])`; each
target is `(name, kind, sourceDirs, sources, exclude, dependencies)`.

**R35 (MUST)** Require exactly one top-level expression statement whose callee is `Package`, assigned
to `let package`. "Any other top-level statement other than the `// swift-tools-version:` comment,
`import PackageDescription`, and that one `let` → unclassifiable, reason `manifest-not-literal`."
**R36 (MUST)** Require the `targets:` argument to be an **array literal** of call expressions whose
callees are `.target`, `.testTarget`, `.executableTarget`, `.macro`, `.systemLibrary`,
`.binaryTarget` or `.plugin`.
**R37 (MUST)** Require every `name:`, `path:`, `sources:` and `exclude:` argument to be a simple
string literal or an array literal of simple string literals. "Any identifier reference, string
interpolation, `+`, ternary, `#if`, `for`, `map`, or function call in those positions →
unclassifiable, reason `manifest-not-literal`."
**R38 (MUST)** Read `dependencies:` only for its simple string literals and `.target(name: "X")` /
`.byName(name: "X")` forms; anything else contributes no dependency and **is not an error**.
**R39 (MUST)** Two targets with the same `name` anywhere in the repository → `duplicate-target-name`.
**R40 (MUST)** A repository containing a `.xcodeproj` or `.xcworkspace` directory and no
`Package.swift` → `xcode-project-unsupported`.
**R41 (MUST)** Resolve a target's source directory: if `path:` is given, `rootDir + "/" + path`;
otherwise the **first existing** directory in this order —
- `.testTarget`: `Tests/<name>`, `Sources/<name>`, `Source/<name>`, `src/<name>`, `srcs/<name>`;
- every other kind: `Sources/<name>`, `Source/<name>`, `src/<name>`, `srcs/<name>`, `Tests/<name>`.

None existing → `target-dir-missing`.
**R42 (MUST)** Compute the **file set** `F(t)`: if `sources:` is given, then for each entry — the
entry itself when it names a blob, and every blob recursively beneath it when it names a directory;
otherwise every blob recursively beneath the source directory, at every depth. Then remove every path
equal to, or beneath, any `exclude:` entry. `F(t)` is the whole set of remaining blobs and **is
filtered by no extension**.
**R43 (MUST)** The target's **source files** are `{ p ∈ F(t) : lang(p) = Swift }` — `.swift`,
byte-exact and lowercase only. A path in the source files of two targets → `overlapping-targets`.

#### ALG-D5 — Swift's mixed-target refusal `mixed-objc-target` (IR §7.3)

**R44 (REFUSE)** `RC(swift, tree)` is **unclassifiable, reason `mixed-objc-target`**, if any target
`t` of any package in the tree satisfies **Test 1 or Test 2**. Both are decided by path and by
argument label; **no file is opened for its content and no manifest value is evaluated**.

**R45 (MUST) Test 1 — a C-family entry in the file set.** Some `p ∈ F(t)` whose final path component
ends in one of the following, matched byte-exactly and lowercase only:

```
.m  .mm  .h  .hh  .hpp  .hxx  .pch  .c  .cc  .cpp  .cxx  .modulemap
```

**R46 (MUST NOT)** Read a filename stem. "A bridging header needs no clause of its own — every
spelling of one, `<Target>-Bridging-Header.h` included, ends in `.h`, and no rule here reads a
filename stem."
**R47 (MUST)** Apply Test 1 to `F(t)` **post-`exclude:`** — "a `.m` under an `exclude:` entry does
**not** trigger."
**R48 (MUST) Test 2 — a manifest construct.** `t`'s call carries any of the argument labels
`publicHeadersPath:`, `cSettings:` or `cxxSettings:`; **or** it contains a simple string literal equal
to `-import-objc-header`; **or** `t`'s callee is `.systemLibrary`. "**Presence alone triggers,
whatever the value.**"
**R49 (MUST)** Run **both tests in both trees**. "an entry in **either** tree raises
`mixed-objc-target`" — the branch is where an oracle arrives.
**R50 (MUST)** Decide `mixed-objc-target` **during extraction and therefore before §3.3 Rule 2's
comparison**: where a branch introduces the first Objective-C entry, "the reason is
`mixed-objc-target` and never `rc-changed-on-branch`" (IR §7.3 Precedence; IR §14 S18).
**R51 (MUST NOT)** Narrow the refusal to targets that also contain Swift. A pure C-family target that
a Swift file `import`s by name is the silent miss itself (IR §7.3, §18 OPEN-2; §14 S17).

### ALG-E — The freeze closure's lexical seed rule (IR §2.1.1)

**R52 (MUST)** Compute `S` exactly as:

> `S` = every path `p` in `A` such that `match(P, p)` holds for at least one pattern `P` in `C-T1`
> (§2.4), and `p`'s bytes in `A` carry at least one §12.1 pragma occurrence whose intent id is this
> intent's and whose acceptance-criterion number is in `AC`.

**R53 (MUST NOT)** Read anything a collection produces when computing `S`. "Nothing a collection
produces is read." A `verified_by`-edge-derived seed is unimplementable at both `--approve` (writes
no result file) and `--ci` (holds no collection over `A`) (IR §2.1.1).
**R54 (MUST NOT)** Seed from the §12.3 `AC<n>` naming sugar. "A test file whose only tie to its
criteria is §12.3's `AC<n>` naming sugar is **not** a seed" (IR §2.1.1, §16.12; conformance C27).
**R55 (MUST)** Treat a pragma naming another intent, or naming an AC number **not in `AC`**, as an
occurrence that seeds nothing — `@verifies INT-041/AC-1` and `@verifies INT-042/AC-9` are both
recognized occurrences (so G5 can report them) and neither is a seed (IR §2.1.1).
**R56 (MUST)** Note that every member of `S` is `FROZEN_WALK` by construction — `H` holds for it
because `C-T1` is a conjunct of the definition (IR §2.1.1).
**R57 (REFUSE)** A path in `A` carrying a §12.1 pragma naming an AC in `AC` that **no `C-T1` pattern
matches** — including one matched only by `C-T2` — is the finding `seed-outside-test-roots`, and
`--approve` **refuses outright** (not a tripwire). It carries the sorted list of such paths
(IR §2.1.1, §2.11; conformance C10).
**R58 (MUST)** An empty `S` is the `no-seed` **tripwire**: the closure is empty, `Spine-Frozen` names
nothing, G8's containment check is `∅ ⊆ ∅`. `--approve` refuses without a `reason=`; `spine stats`
counts `seedless_approvals` (IR §2.1.1, §2.11, §16.12).
**R59 (MUST NOT)** Fall back to "every file under `C-T1`" when no file carries a pragma
(IR §14 C28).
**R60 (MUST)** Seed a file whose pragma exists even if no runner would collect it: "still a seed.
Collection is not read anywhere in §2" (IR §14 C29).

### ALG-F — Python (IR §4)

**R61 (MUST)** Lex Python comments as `#` to end of line; there is no block comment; a `#` inside a
string literal is not a comment (IR §4.1).
**R62 (MUST)** Lex Python string literals `'…'`, `"…"`, `'''…'''`, `"""…"""`, each optionally
prefixed by **any case-insensitive combination** of `r`, `b`, `f`, `u`, `rb`, `br`. "A literal is
**simple** (§3.4 rule 5) only when its prefix is empty or `r`/`u`, it is not an f-string, and it
contains no backslash" (IR §4.1).
**R63 (MUST)** Determine logical lines: "a logical line ends at a newline that is not inside
`(`/`[`/`{` and is not preceded by a backslash continuation. `;` separates statements inside a
logical line" (IR §4.1).
**R64 (MUST)** Anchor an import site at a `word` token `import` or `from` that is **the first token
of a logical line or the first token after a `;`** (IR §4.1).
**R65 (MUST)** Use exactly two resolution roots, in this order, evaluated against the tree being
resolved against: (1) `""` — the repository root; (2) `src/` — **if and only if** a tree entry `src`
exists and is a directory. "That is the whole list" (IR §4.3).
**R66 (MUST)** For a dotted name `n₁.n₂.….n_k`, form, for each root `r` in order, the candidates
`r + n₁/…/n_k + ".py"` and `r + n₁/…/n_k + "/__init__.py"`. "The first root for which at least one
candidate exists wins" (IR §4.3).
**R67 (REFUSE/site)** If **both** candidates exist under the winning root, the site is `unresolvable`,
reason `ambiguous-module` (IR §4.3; conformance P4).
**R68 (MUST)** Add ancestor packages to the edge: "the site's targets are the resolved module
**plus**, under the same winning root, every existing `n₁/…/n_j/__init__.py` for `1 ≤ j < k`. A
missing intermediate `__init__.py` is a namespace package (PEP 420) and is simply not a target — it
is not an error" (IR §4.3; conformance P3 — "**must not** be only the leaf").
**R69 (MUST)** Implement the forms table exactly (IR §4.3):

| Form | Targets |
|---|---|
| `import a.b.c` / `import a.b.c as d` | dotted resolution of `a.b.c` |
| `import a.b, c.d` | dotted resolution of each, as **separate sites** |
| `from a.b import c` | first, dotted resolution of `a.b.c`; if that yields nothing, dotted resolution of `a.b` alone. If neither resolves, `external` |
| `from a.b import c, d` | the union of the `from` rule applied to each name; **one site** |
| `from a.b import *` | dotted resolution of `a.b` |
| `from . import c` | package-relative, level 1 |
| `from .a.b import c` | package-relative, level 1, then dotted `a.b.c` / `a.b` |
| `from ..a import b` | package-relative, level 2 |
| `from a import (b, c)` (parenthesized, possibly multi-line) | as `from a import b, c`; one site |

**R70 (MUST)** Package-relative resolution: "Let `d` be the directory containing the importing file.
For level `L`, the base directory is `d` with `L − 1` components removed." This is the same for
`p/q/mod.py` and `p/q/__init__.py`. If the base directory would escape the repository root, the site
is `unresolvable`, reason `relative-escapes-root`. Otherwise resolve the remaining dotted name against
that directory as a single root, "and add the existing `__init__.py` of the base directory and of
every intermediate directory as targets" (IR §4.3).
**R71 (MUST)** Treat each occurrence of any of these token sequences as an import site with
disposition `unresolvable`, reason `dynamic-import`: `__import__`, `importlib . import_module`,
`importlib . __import__`, `importlib . util . spec_from_file_location`, `imp . load_source`.
**R72 (MUST NOT)** Inspect the argument "even when it is a simple literal" (IR §4.3; conformance P10).
**R73 (MUST)** `AncestorConfig(python)` clause-3 basenames: `__init__.py`, `conftest.py`,
`pytest.ini`, `pyproject.toml`, `tox.ini`, `setup.cfg` (IR §4.4).
**R74 (MUST)** Scaffolded `C-T2` for `python ∈ langs` (IR §4.5), `C-T1` default `tests/`:

```
tests/support/**
**/conftest.py
pytest.ini
pyproject.toml
tox.ini
setup.cfg
```

**R75 (MUST)** Python snapshot patterns (clause 4): final component matching `*.ambr`,
`*.approved.txt`, `*.golden`, `*.snap`; or any path with a directory component named `__snapshots__`
or `snapshots` (IR §4.6).

### ALG-G — TypeScript / JavaScript (IR §5)

**R76 (MUST)** Lex comments `//` to end of line and `/* … */` (**not nested**). A `///`-prefixed line
comment whose remainder matches `<reference …/>` is a triple-slash directive and a `type_only` import
site; it is otherwise a comment (IR §5.1).
**R77 (MUST)** Lex string literals `'…'`, `"…"`, and template literals `` `…` ``; "A template literal
containing `${` is not simple." Regex literals are lexed as `punct` runs and never as strings: "a `/`
that follows a `word`, `)`, `]` or a numeric literal is division, otherwise it opens a regex"
(IR §5.1; conformance TS T22).
**R78 (MUST)** Anchor: a `word` token `import` **not immediately preceded by** a `.` token is an
import site — dynamic if the next token is `(`, `import.meta` (**not** a site) if the next token is
`.`, a declaration otherwise. A `word` token `export` not preceded by `.` begins a re-export site
**iff** a `from` word token followed by a simple string literal occurs before the next `;` or `}` at
the same bracket depth. A `word` token `require` not preceded by `.` and immediately followed by `(`
is a CommonJS import site (IR §5.1).
**R79 (MUST)** Classify a specifier `s` for importing file `f` by these ordered steps (IR §5.2):
1. `s` begins `./`, `../`, or is exactly `.` or `..` → **relative**. Base path `Bp` = the lexical
   normalization of `dirname(f) + "/" + s`, collapsing `.` and `..` **textually**. Escapes the
   repository root → `unresolvable` (`relative-escapes-root`).
2. `s` begins `/` → `unresolvable` (`absolute-specifier`).
3. `s` begins `#` → `unresolvable` (`subpath-imports`).
4. otherwise **bare**: consult the alias table. If some alias matches, each substituted candidate base
   path goes to candidate expansion **in the table's order** and the **first** that resolves wins; if
   an alias matched and none resolves → `unresolvable` (`alias-dead-end`). If no alias matches →
   `external`.

**R80 (MUST)** Candidate expansion for base path `Bp`, **first match wins over the whole ordered
list** (IR §5.2 step 5):
1. `Bp` itself, if it is an existing file entry.
2. The TypeScript output-extension rewrite when `Bp` ends in a JavaScript extension: `.js` → `.ts`,
   `.tsx`; `.mjs` → `.mts`; `.cjs` → `.cts`.
3. `Bp + ext` for `ext` in this exact order: `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs`,
   `.cjs`, `.json`.
4. If `Bp` is an existing **directory** entry: `Bp + "/index" + ext` for `ext` in the same order.
5. otherwise → `unresolvable` (`no-candidate`).

**R81 (MUST)** **Skip** a `.d.ts`/`.d.mts`/`.d.cts` candidate in steps 1–4 rather than matching it;
if every candidate is a declaration file, the site is `type_only` (IR §5.2).
**R82 (MUST NOT)** Read `package.json` for directory resolution — no `main`, `module`, `exports` or
`types`; a directory resolves by `index` alone (IR §5.2; conformance TS T4).
**R83 (MUST)** Implement the forms table (IR §5.2): `import d from 's'` / `{a}` / `* as n` /
`d, {a}` = value import; `import 's'` = side-effect import, **a real edge**; `import x = require('s')`
= value import; the re-export forms of R14; `import('s')` with a simple literal = value import;
`import(expr)` = `unresolvable` reason `dynamic-import`; `require('s')` simple literal = value import;
`require(expr)` = `unresolvable` reason `dynamic-import`; type-only forms per R16.
**R84 (MUST NOT)** Treat `require.resolve('s')` as an import site — "it returns a path and executes
nothing" (IR §5.2; conformance TS T12).
**R85 (MUST)** Resolve a **literal** `import('./x')` rather than tripwiring it: "'Dynamic' is read as
*the specifier is not statically determined*" (IR §5.2, §16.8).
**R86 (MUST)** Match aliases thus: "a key `k` matches if either `k` has no `*` and `k == s`, or `k` is
`p*q` and `s` begins with `p` and ends with `q` and `|s| ≥ |p| + |q|`; the capture is the middle.
Where several keys match, the one with the **longest literal prefix before its `*`** wins; ties are
impossible." Each substitution has its `*` replaced by the capture and is resolved relative to
`baseUrl` (or, absent `baseUrl`, to the directory of the `tsconfig.json` that declared `paths`)
(IR §5.3).
**R87 (MUST)** `AncestorConfig(ts)` clause-3 basenames (IR §5.4): `package.json`, `tsconfig.json`,
`jsconfig.json`, `vitest.config.ts`, `vitest.config.mts`, `vitest.config.js`, `vitest.config.mjs`,
`vitest.workspace.ts`, `vitest.workspace.js`, `vitest.setup.ts`, `vitest.setup.js`, `jest.config.ts`,
`jest.config.js`, `jest.config.mjs`, `jest.config.cjs`, `jest.setup.ts`, `jest.setup.js`.
**R88 (MUST)** Walk config files that are themselves TypeScript — they satisfy `H`, so
`vitest.config.ts`'s own `import './vitest.setup.ts'` pulls the setup file into the closure (IR §5.4).
**R89 (MUST)** Scaffolded `C-T2` for `ts` (IR §5.5), `C-T1` default `tests/`, `src/**/__tests__/`:

```
tests/support/**
package.json
tsconfig.json
jsconfig.json
vite.config.*
vitest.config.*
vitest.workspace.*
vitest.setup.*
jest.config.*
jest.setup.*
```

`vite.config.*` is on the list **because §12.4.2 makes it a `C-T3` hook basename, and the two lists
have to agree**; omitting it left every Vite repository with a permanent `class=protected`
`G8:vite.config.ts` finding on every landing (IR §5.5; conformance T8a).
**R90 (MUST)** TS snapshot patterns (clause 4): final component matching `*.snap`; or any path with a
directory component named `__snapshots__` (IR §5.6).

### ALG-H — Dart (IR §6)

**R91 (MUST)** Lex comments `//` to end of line and `/* … */` **nested** — "Dart's block comments
nest, and a lexer that does not nest them mis-lexes a commented-out block containing `*/`" (IR §6.1;
conformance D11).
**R92 (MUST)** Lex string literals `'…'`, `"…"`, `'''…'''`, `"""…"""`, each optionally prefixed `r`.
"A literal containing `$` followed by `{` or an identifier character is interpolated and not simple;
a raw (`r`) literal with no interpolation and no backslash is simple" (IR §6.1).
**R93 (MUST)** Anchor on shape: "a `word` token `import`, `export` or `part` **immediately followed by
a `string` token**, or the sequence `part` `of`" (IR §6.1).
**R94 (MUST)** Resolve a Dart URI by scheme (IR §6.2):

| URI shape | Resolution |
|---|---|
| `dart:…` | `external` |
| `package:<name>/<rest>` where `<name>` is `RC.selfName` | `lib/<rest>`, relative to the package root directory |
| `package:<name>/<rest>` where `<name>` is a key of `RC.pathDeps` | `<RC.pathDeps[name]>/lib/<rest>` |
| `package:<name>/…` otherwise | `external` |
| no scheme (a relative URI) | lexically normalized against the importing file's directory; escaping the repository root → `unresolvable` |
| any other scheme (`file:`, `http:`, `asset:`) | `unresolvable`, reason `unsupported-scheme` |

**R95 (MUST NOT)** Append extensions or do index resolution in Dart. "Dart requires the `.dart`
extension in every URI, so there is no candidate expansion, no index resolution and no extension
list. A resolved path that is not an existing file entry → `unresolvable`, reason `no-candidate`"
(IR §6.2; conformance D10).
**R96 (MUST)** Implement the forms: `import 'uri';` with optional `as`, `show`, `hide`, `deferred as`
= value import; `export 'uri';` with `show`/`hide` = re-export; `import 'a' if (c) 'b' if (d) 'e';` =
**one site, all URIs**; `export 'a' if (c) 'b';` = one site, all URIs; `part 'uri';` = value import
and the part file **is walked**; `part of 'uri';` = value import naming the parent library file
(IR §6.2).
**R97 (MUST)** Resolve `part of <dotted name>` "through a library-name index built over the tree being
resolved against: every Dart file whose directives contain `library <dotted name>;`. Exactly one match
→ that file. Zero or more than one → `unresolvable`, reason `ambiguous-library-name`" (IR §6.2).
**R98 (MUST)** Treat `deferred as` as an ordinary edge — Dart has no dynamic import; Dart has no
type-only import (IR §6.2).
**R99 (MUST)** `AncestorConfig(dart)` clause-3 basenames: `pubspec.yaml`, `dart_test.yaml`,
`build.yaml`. `analysis_options.yaml` is **deliberately absent** (IR §6.4).
**R100 (MUST)** Scaffolded `C-T2` for `dart` (IR §6.5), `C-T1` default **`test/`** (singular):

```
test/support/**
pubspec.yaml
dart_test.yaml
build.yaml
```

**R101 (MUST)** Dart snapshot patterns (clause 4): final component matching `*.golden`,
`*.approved.txt`; or any path with a directory component named `goldens` or `__snapshots__`
(IR §6.6).

### ALG-I — Swift (IR §7)

**R102 (MUST)** Lex comments `//` to end of line and `/* … */` (**nested**) (IR §7.1).
**R103 (MUST)** Lex string literals `"…"`, `"""…"""`, and extended delimiters `#"…"#`, `##"…"##`;
interpolation is `\(` (or `#\(` at matching delimiter depth) (IR §7.1).
**R104 (MUST)** Anchor: "a `word` token `import` not immediately preceded by `.`, optionally preceded
by the attribute tokens `@testable` or `@_exported`" (IR §7.1).
**R105 (MUST)** Map the import forms to modules (IR §7.2): `import Foo` → `Foo`; `import Foo.Bar` →
`Foo` (first component); `import struct Foo.Baz` (and `class`, `enum`, `protocol`, `typealias`,
`func`, `let`, `var`) → `Foo`; `@testable import Foo` → `Foo`; `@_exported import Foo` → `Foo`, a
re-export; `#if`/`#elseif`/`#else` around any of the above → **every branch**.
**R106 (MUST)** Treat `@testable import` identically to `import` for resolution. "It changes the
*visibility* … and it changes nothing about which module is imported or which files that module
contains" (IR §7.2; conformance S1).
**R107 (MUST)** Compute a Swift file's imports as (IR §7.4):

> `imports(f) =` every source file of `M` other than `f`, **plus** for each `import N` site in `f`,
> every source file of the target named `N` if one exists (and `external` if none does).

**R108 (MUST)** Keep classification **per file**, not per module: a branch-created
`Sources/Billing/Oracle.swift` inside an existing target is reached, found absent from `B`, is
`EXCLUDED` by row 3, and **raises the closure tripwire** (IR §7.4; conformance S6).
**R109 (MUST)** `AncestorConfig(swift)` clause-3 basenames: `Package.swift`, `Package.resolved`
(IR §7.5). `Package.resolved` "pins the exact revision of every dependency … It is not read for
resolution; it is frozen."
**R110 (MUST)** Scaffolded `C-T2` for `swift` (IR §7.6), `C-T1` default `Tests/`:

```
Tests/Support/**
Package.swift
Package.resolved
```

**R111 (MUST)** Swift snapshot patterns (clause 4): final component matching `*.snap`,
`*.approved.txt`, `*.golden`; or any path with a directory component named `__Snapshots__` or
`Snapshots` (IR §7.7).

### ALG-J — The glob dialect and `match` (ID §6.1–§6.3, adopted unaltered by IR §2.4)

**R112 (MUST)** Use **one** dialect for `C-T1`, `C-T2`, `C-Q1`, `C-A2` and touchpoints: ID §6.1 (byte
grammar + refusal list), §6.2 (glob dialect), §6.3 (`match(P, p)`). "This document defines no pattern
syntax and no matching rule of its own" (IR §2.4).
**R113 (MUST)** A pattern is **1 … 255 bytes, each in `0x21 … 0x7E`**, excluding `0x2C` `,`,
`0x22` `"`, and `0x5C` `\`. Space `0x20` is excluded by the range; bytes above `0x7E` are excluded by
the range — **a pattern is ASCII**. Failing bytes are `pattern-illegal-byte` (ID §6.1).
**R114 (REFUSE)** The hard refusal list (ID §6.1): empty → `pattern-empty`; longer than 255 bytes →
`pattern-too-long`; begins `!` → `bad-negation`; begins `/` → `leading-slash`; contains `//` →
`empty-segment`; has a segment `.` or `..` → `dot-segment`; a segment contains `**` but is not exactly
`**` → `bad-globstar`; a malformed bracket → `bad-bracket`.
**R115 (MUST NOT)** Provide any escape mechanism — "`\` is not an allowed byte" (ID §6.1).
**R116 (MUST)** Split a pattern into **segments** on `/`; a trailing `/` yields a final empty segment
which §6.3 removes before splitting (ID §6.2).
**R117 (MUST)** Within a segment: `?` matches exactly one byte, **never `/`**; `*` matches zero or
more bytes, **none of them `/`** — "`*` does not cross a separator"; `[ … ]` matches one byte from the
set; any other byte matches itself exactly (ID §6.2).
**R118 (MUST)** A whole segment equal to `**` matches **zero or more complete segments**, and `**` may
appear **only** as a whole segment. `a/**/b` matches `a/b`; `**/x` matches `x`; `a/**` matches `a`
(uniform, unlike gitignore) (ID §6.2).
**R119 (MUST)** Implement bracket expressions per this grammar (ID §6.2, verbatim):

```
bracket := "[" [ "!" ] [ "]" ] member* "]"
member  := byte | byte "-" byte
```

- A leading `!` negates. **`^` does NOT negate**; it is an ordinary member byte.
- A `]` immediately after `[` or `[!` is a literal member. `[]]` is the set `{ ] }`.
- A range `a-b` requires `a ≤ b` as byte values; `[z-a]` is `bad-bracket`.
- An unterminated `[` is `bad-bracket`. **It is not treated as a literal `[`.**
- `/` inside a bracket is `bad-bracket`.
- POSIX classes, collating symbols and equivalence classes are refused: a bracket whose first member
  byte (after an optional `!`) is `:`, `.` or `=`, or which contains the two-byte sequence `[:`, `[.`
  or `[=`, is `bad-bracket`.
- A bracket never matches `/`.

**R120 (MUST)** Validate brackets **over the whole pattern before splitting into segments**, so
`[a/b]` is `bad-bracket` rather than two malformed segments (ID §6.2).
**R121 (MUST NOT)** Perform brace expansion. `{` and `}` are ordinary bytes (ID §6.2).
**R122 (MUST)** Implement `match` exactly as (ID §6.3, verbatim):

```
go(i, j) :=
  if i = |ps|            : j = |ss|
  else if ps[i] = "**"   : ∃ k ∈ [j, |ss|] : go(i+1, k)
  else if j = |ss|       : false
  else                   : segmatch(ps[i], ss[j]) ∧ go(i+1, j+1)

gmatch(P, s) := go(0, 0)
```

```
match(P, p) :=
  if P ends with "/" :
     let Q := P without its trailing "/"
     ∃ a split p = q ++ "/" ++ r, with r non-empty, such that gmatch(Q, q)
  else :
     gmatch(P, p)
     ∨ ∃ a split p = q ++ "/" ++ r, with r non-empty, such that gmatch(P, q)
```

**R123 (MUST)** Read `p` as "a repository path as git produces it: a byte string, `/`-separated, no
leading `/`, no `.` or `..` component, no trailing `/`" (ID §6.3).
**R124 (MUST NOT)** Casefold, normalize (Unicode), or rewrite separators when matching. "against a
repository path exactly as git stores it, byte-wise, with no case folding and no normalization"
(IR §2.4; ID §6.4, §6.5).
**R125 (MUST)** Contribute **both** paths of a rename to the matched path set, the deleted path of a
deletion, and the path of a mode change, symlink or submodule entry (ID §6.4).
**R126 (MUST NOT)** Implement version 1 of IR §2.4 — its rule 2 (trailing `/` as a raw byte prefix)
and rule 4 (six invalid bytes) are named for deletion: "Any implementation still carrying version 1's
rules 1–5 is non-conforming" (IR §2.4.1).

### ALG-K — The `@verifies` pragma (IR §12.1)

**R127 (MUST)** Recognize a pragma occurrence **inside a `comment` token** with this grammar
(verbatim):

```
@verifies <SP>+ <intent-id> "/" "AC-" <digit>+
```

where `<SP>` is `U+0020` or `U+0009`.
**R128 (MUST)** Use ID §3.1's intent-id domain and **no other**: `("INT" | "BUG") "-" numeral`, the
numeral a decimal integer **left-padded with `0` to a minimum width of 3 and padded no further**. "So
`INT-042`, `BUG-051` and `INT-1042` are ids and `INT-42`, `INT-0042`, `INT-000` and `int-042` are
not" (IR §12.1; conformance C26).
**R129 (MUST)** Scan over the comment's **decoded bytes**; a comment may carry several occurrences,
separated by any bytes (IR §12.1).
**R130 (MUST)** Require `@verifies` to be preceded by a byte **outside `[A-Za-z0-9_@]`** or to be at
the comment's start — "so `x@verifies` is not one" (IR §12.1; conformance J3).
**R131 (MUST)** Capture the AC number **as written** with `<digit>+` — deliberately wider than
ID §5.3's `1 … 6` — so that a pragma naming `AC-9` is *recognized* in order to be reported by G5
(IR §12.1).
**R132 (MUST)** Compare the captured digit run **canonically** against ID §5.3's spelling — a decimal
`1 … 6` with no leading zeros — for `AC` membership (§2.1.1) and G5's orphan test. "so `AC-9`, `AC-01`
and `AC-007` are occurrences that name no acceptance criterion, seed nothing, and are G5 findings"
(IR §12.1).
**R133 (MUST)** Scan the four languages' own comment forms: `#` for Python; `//` and `/* */` for the
other three (**nested** for Dart and Swift); Python has no block comment (IR §12.1).
**R134 (MUST NOT)** Scan docstrings. "Docstrings are **not** comments and are not scanned — a
`@verifies` in a Python docstring does not count, because a docstring is a string literal and the
resolver's lexer classifies it as one" (IR §12.1; conformance J2, C30).
**R135 (MUST NOT)** Use a second id domain such as version 2's `^(INT|BUG)-[0-9]+$` — it admits both
`INT-42` and `INT-042` and forks the closure (IR §12.1).

### ALG-L — The join, file-granular (IR §12.2)

**R136 (MUST)** Attribute a pragma occurrence in file `P` to **every collected test id whose
`id → path` equals `P`**, for every runner in the invocation set (IR §12.2).
**R137 (MUST NOT)** Perform a declaration-level parse to narrow the attribution. "It requires no
declaration-level parse — which the resolver deliberately cannot do (§1) — and it is total" (IR §12.2).
**R138 (MUST)** Accept the consequence: "a pragma attributes to every test in its file, not to the
test it sits above" (IR §12.2; conformance J4 — a pragma in a file from which the runner collected
three ids yields **three** `verified_by` edges).
**R139 (MUST)** Set `attributed` per PB §6.2 unchanged: "true iff the pragma's line is in a blob the
binding approval froze, or — before approval — the file is on the intent's own branch and under
`C-T1`" (IR §12.2; PB §7.1 Pragma provenance).

### ALG-M — The `AC<n>` naming sugar (IR §12.3)

**R140 (MUST)** Use exactly one pattern for all runners: "the byte sequence `AC` followed by one or
more digits, preceded by a byte outside `[A-Za-z0-9]` or at the start of the field, and followed by a
byte outside `[0-9]` or at the end of the field. The capture is the digit run" (IR §12.3).
**R141 (MUST)** Take the intent to be "the branch's single gated intent" (PB §4.3, one gated intent
per branch) (IR §12.3).
**R142 (MUST)** Use these per-runner **fields** and no others (IR §12.3, verbatim table):

| Runner | Field |
|---|---|
| `pytest` | the final `::`-separated component of `fn`, with the parametrization suffix already removed |
| `vitest` | the final ` > `-separated component of `id` |
| `dart-test` | the bytes of `id` after the first `::` — the test's fully qualified name, group prefixes included |
| `swift-test` | the bytes of `id` after the `/` — the method name — **with a leading `test` removed if present** |

For `swift-test`: "`testAC1TotalsIncludeTax` gives the field `AC1TotalsIncludeTax`, in which `AC1` is
at the field's start; `test_AC1_totals` gives `_AC1_totals`, which yields the same edge."
**R143 (MUST)** Yield **several edges** for several `AC<n>` matches in one field (IR §12.3).
**R144 (MUST)** Report a match whose AC number has no corresponding AC in the intent as **G5's
finding**, exactly as a typo'd pragma is (IR §12.3).
**R145 (MUST)** Take the **union** where a file carries both a pragma and a matching name — "no rule
prefers one" (IR §12.3).
**R146 (MUST NOT)** Fork the pattern per runner. Only the **field** varies; the pattern does not. So
`testAC1AndAC2Totals` under `swift-test` gives **AC-1 and not AC-2** (the second `AC` is preceded by
`d`), while `test_AC1_and_AC2` under pytest gives both (IR §12.3; conformance J5, J6, J7).
**R147 (MUST)** Accept `dart-test`'s coarseness: the field is the whole qualified name, so
`group('AC3 rounding'){ test('half even') }` gives the field `AC3 rounding half even` and yields AC-3
for **every test in the group** (IR §12.3).

### ALG-N — `C-T3`, the predicate G8 greps (IR §12.4)

**R148 (MUST)** Evaluate `C-T3` over the **synthetic merge `T`**; read `C-T1`, `C-T2` and `C-T3`'s own
value from trunk (PB §7.4 rule 1). `C-T3`'s v1 domain is the single token `on` (CN §6.1), so the grep
runs on every landing that runs G8 (IR §12.4).
**R149 (MUST)** Implement the predicate exactly (IR §12.4, verbatim):

> `ct3(p)` is **true** — a hit — iff `H(p)` is false (§2.3) and either (a) `lang(p) ∈ langs` and `p`'s
> bytes carry a framework import site by §12.4.1, or (b) `p`'s final path component is in §12.4.2's
> hook-basename set for some language in `langs`, or (c) `lang(p) ∈ langs` and `p`'s tokens carry a
> hook token sequence by §12.4.2.

**R150 (MUST)** Test against **`H` = `C-T1` ∪ `C-T2`**, never `C-T1` alone (IR §12.4; PB §2.1, §4.3,
§6.3 G8 all now read "outside the harness (`C-T1` ∪ `C-T2`)"; §17 D12 CLOSED).
**R151 (MUST)** Emit **one finding per hit path**, whatever the number of sites in it — the wire set
is per path (IR §12.4; conformance T14).
**R152 (MUST NOT)** Count a type-only import as a hit (IR §12.4, §3.6; conformance T4).
**R153 (MUST)** Ignore disposition — "the test is on the specifier's bytes rather than on where it
resolves"; a vendored runner resolving `repo(m)` is a hit exactly as much (IR §12.4).
**R154 (MUST NOT)** Report a hit for a file the resolver cannot read: `lang(p) = none` is not scanned,
and a non-UTF-8 file raises `file-not-utf8` and yields no `C-T3` finding (IR §12.4).
**R155 (MUST)** Use exactly this closed framework-specifier set, tested **on the specifier as
written**, after §3.4's lexing (IR §12.4.1, verbatim table):

| Language | Tested on | Framework set |
|---|---|---|
| Python | the dotted name of §4.3's forms, reduced to its **longest matching dotted prefix** | `pytest`, `_pytest`, `unittest` — **less the single exemption `unittest.mock`** |
| TypeScript/JavaScript | a **bare** specifier (§5.2 step 4) reduced to its package name: the bytes before the first `/` for an unscoped name, the bytes before the second `/` for one beginning `@`, and the whole specifier for one beginning `node:` | `vitest`, `jest`, `chai`, `expect`, any package under the scope `@vitest/` or `@jest/`, and `node:test` |
| Dart | a `package:` URI reduced to its `<name>` (§6.2) | `test`, `test_api`, `test_core`, `matcher` |
| Swift | the module named by §7.2's forms | `XCTest`, `Testing` — **plus any `@testable import`, whatever module it names** |

**R156 (MUST)** Apply Python's exemption **by full dotted prefix**: `import unittest.mock`,
`from unittest.mock import patch` and `from unittest import mock` are **not** hits, while
`import unittest`, `from unittest import TestCase` and `from unittest.case import TestCase` **are**.
`doctest` is deliberately **out** (IR §12.4.1; conformance T5, T6).
**R157 (MUST)** Match `node:test` as the **whole specifier**; a bare `test` in TypeScript is **not** a
hit (IR §12.4.1).
**R158 (MUST)** Treat any `@testable import` as a hit whatever module follows it (IR §12.4.1;
conformance T11).
**R159 (MUST)** Use exactly this closed hook set (IR §12.4.2, verbatim table):

| Language | Hook basenames — loaded with no import | Hook token sequences (§3.4 tokens, in order, `comment` discarded) |
|---|---|---|
| Python | `conftest.py` | `def` followed by a `word` beginning `pytest_`; `async` `def` followed by a `word` beginning `pytest_`; the `word` `pytest_plugins` followed by the `punct` `=` |
| TypeScript/JavaScript | `vitest.config.` / `vitest.workspace.` / `vite.config.` / `jest.config.` each followed by one of `ts`, `mts`, `cts`, `js`, `mjs`, `cjs`; and `jest.config.json` | none |
| Dart | `dart_test.yaml` | none |
| Swift | none | none |

**R160 (MUST)** Decide a basename hit **from the path and never from the content**, "at any depth"
wherever `H` is false (IR §12.4.2; conformance T8 — "**must not** be read for content first").
**R161 (MUST)** Treat a `def pytest_…` at any nesting depth as a hit — "Nesting is irrelevant here as
it is everywhere else in this document (§3.4 rule 7)" (IR §12.4.2; conformance T9).
**R162 (SHOULD, stated residual)** Accept the three misses §12.4.3 names: a dynamic reach
(`importlib.import_module("pytest")`, `require(expr)`) is **not** a hit (it is an `unresolvable` site
and, outside the harness, the counter `unresolvable-import-outside-harness`); a `lang: none` file is
not scanned at all; a monkeypatch needing no framework import is untouched (IR §12.4.3; conformance
T12, T13).

### ALG-O — Runner adapters, rules shared by all four (IR §11.6)

**R163 (MUST)** Hold the `runner` token as an adapter constant — "never read from a stream, a
manifest, `params.langs` or the environment" (IR §11.6 rule 1).
**R164 (REFUSE)** If two distinct reported items compose to one id under any runner, the collector
**fails the job and writes nothing** — finding `duplicate-test-id`, carrying the runner token and the
id (IR §11.6 rule 2; conformance R7).
**R165 (REFUSE)** Where an adapter's `id → path` is "the bytes before the first `<sep>`", a repository
path containing `<sep>` makes the collector **fail the job and write nothing**, finding
`id-separator-in-path`. `<sep>` is `::` for `pytest` and `dart-test` and ` > ` for `vitest`;
`swift-test` has no such split and is unaffected (IR §11.6 rule 3; conformance R9).
**R166 (MUST)** Obtain the `B` outcome by running the adapter's **`T` invocation** against the
checkout of `B` and passing its terminal reports through the **same** outcome mapping used on `T`.
"No adapter defines a second mapping, a `B`-only value or a `B`-only refusal, and none may"
(IR §11.6 rule 4).
**R167 (MUST)** Note that `absent` "is not produced by a mapping at all — it is what an id that the
`B` outcome run reported no terminal outcome for gets … and it is written by the collector rather than
by the adapter" (IR §11.6 rule 4; RF §4.4).
**R168 (MUST)** Guarantee `fn` is a prefix of `id`; `fn == id` for `vitest`, `dart-test` and
`swift-test`, and by construction for `pytest` (IR §11.6; RF §4.4; conformance R15).
**R169 (MUST)** Name a terminal session-end event per adapter: `dart-test`'s is the `done` event,
`swift-test`'s is the final `Test Suite 'All tests' <verb> at …` line (IR §11.6 rule 6).
**R170 (MUST)** Take the `B` floor as "the set of ids the runner **collected and selected** on the
checkout of `B` — every id it enumerated, less any it reported as *deselected*, and **irrespective of
outcome**" (IR §11.1). An id trunk reported `failed`, `skipped` or `xfail` is in the floor exactly as
a `passed` one.
**R171 (MUST)** Run **every** `B` invocation of **every** runner — enumeration and outcome run alike —
**before every `T` execution**. "Interleaving is forbidden" (IR §11.1; RF §7.1 step 7, §7.4 rule 3).
**R172 (MUST NOT)** Treat a failed `B` **outcome** run as `base-collect-failed`. It "leaves every id it
did not report a terminal outcome for at `out: "absent"`, contributes no status, and moves no byte of
the `end` record" (IR §11.1; conformance R16b).
**R173 (MUST)** Run every invocation **at the repository root with no selection argument of any kind**
(IR §11.1; RF §7.2).
**R174 (MUST NOT)** Run any command §11 has not ratified. "**No adapter runs a command this section
has not already ratified.**" (IR §11.1).

### ALG-P — `pytest` (IR §11.2)

**R175 (MUST)** `id → fn`: "split the nodeid on `::`. In the final component, the parametrization
suffix begins at the **first** `[` and runs to the end, and exists only if the component's last byte
is `]`. `fn` is the nodeid with that suffix removed."
**R176 (MUST)** `id → path`: "the component before the first `::`, as repo-relative POSIX bytes; the
empty string where no tree entry matches."
**R177 (MUST)** Outcome mapping: RF §6.7's table, unchanged —

| Observation | `out` |
|---|---|
| all phases passed, no expected-failure marker | `passed` |
| all phases passed, expected-failure marker set | `xpass` |
| `call` failed, no expected-failure marker | `failed` |
| `call` failed or skipped, expected-failure marker set | `xfail` |
| skipped, no expected-failure marker | `skipped` |
| failure or exception in `setup`/`teardown`, or a collection error | `error` |
| collected, then excluded before running | `deselected` |
| any other terminal report | `unknown` |

**R178 (MUST)** `B` enumeration: `pytest --collect-only`, at the repository root, on the checkout of
`B`, through the same transport the `T` invocation uses. It is **not** a forbidden selection argument
(IR §11.2).
**R179 (MUST)** `B` outcomes: `pytest` — the adapter's own `T` invocation, unchanged, no selection
argument, at the repository root on the same checkout of `B`. "**This is a second full run of the
repository's suite on every landing**" (IR §11.2; conformance R16a).
**R180 (MUST)** Exclude **deselected** ids from the floor. "They are the one thing the collection
reports that the floor drops" (IR §11.2; conformance R18).
**R181 (MUST)** Run the completeness check: compare pytest's own collected-and-selected count —
`4 tests collected`, or `3/4 tests collected (1 deselected)` — with the number of ids extracted;
"fewer or more is `base-collect-failed` on `B` and `runner-failed` on `T`" (IR §11.2).
**R182 (REFUSE)** A collection error during the `B` **enumeration** is `base-collect-failed`. pytest
interrupts at the first error — `!!! Interrupted: 1 error during collection !!!`, exit 2, under
`--collect-only` and a full run alike — so RF §7.3's all-or-nothing rule applies: **no `base` and no
`result` records from any runner** (IR §11.2; conformance R19).
**R183 (MUST NOT)** Use exit status as the completeness signal. "**Exit status is not the signal and
must not be used as one**: `pytest --collect-only` exits `5` over a tree with no tests at all, which
is a legitimate trunk before the first intent lands" (IR §11.2; conformance R20).
**R184 (MUST)** Rely on the three reproduced agreement cases (IR §11.2): a `@pytest.mark.skip`/`skipif`
item is collected under both commands; a module-level `pytest.skip(allow_module_level=True)` yields
zero items under both; a `pytest_collection_modifyitems` deselection removes the same items under both.

### ALG-Q — `vitest` (IR §11.3)

**R185 (MUST)** `id → fn`: **`fn == id`, always**.
**R186 (MUST)** `id → path`: "the substring before the first ` > `, as repo-relative POSIX bytes."
**R187 (MUST)** Outcome mapping: "as `result-file.md` §5's vocabulary, with vitest's `todo` mapping to
`skipped` and its `passed`/`failed`/`skipped` mapping directly; anything else `unknown`."
**R188 (MUST)** **Compose** the id — it is not reported: "the file's path made repo-relative, then
`" > "`, then the enclosing suite titles outermost-first joined by `" > "`, then `" > "`, then the
test's own title" (IR §11.3).
**R189 (REFUSE)** "A reported file path that is not under the repository root makes the contribution
`stream-invalid`" (IR §11.3; RF §7.3).
**R190 (MUST)** `B` collection: **`vitest run`**, the same invocation as `T`, on the checkout of `B`;
one invocation serves both enumeration and outcomes (IR §11.3; conformance R17).
**R191 (MUST NOT)** Use `vitest list`. It **omits every skipped test** — reproduced: five tests, two
behind `it.skip`/`describe.skip`, list **three**; `vitest run` reports all five. "No flag restores
them — the mode offers `--hideSkippedTests` and nothing that would un-hide them" (IR §11.3).
**R192 (REFUSE)** "if any file reports a load or transform error while collecting on `B`, this
runner's contribution is `base-collect-failed`", and by RF §7.3 no `base` and no `result` records are
written from any runner (IR §11.3).
**R193 (MUST)** Note that this adapter never produces `xfail` and never produces `deselected`; `out:
"absent"` is unreachable on a collection that succeeded (IR §11.3; conformance R17a).

### ALG-R — `dart-test` (IR §11.4)

**R194 (MUST)** Invoke `dart test --reporter=json --no-retry` at the repository root, with stdout and
stderr **merged into one pipe** the collector holds.
**R195 (REFUSE)** "A repository with no `pubspec.yaml` at its root cannot be run by this adapter: the
contribution is `spawn-failed`."
**R196 (MUST)** Pass `--no-retry` — **mandatory**: a retried test is reported as a fresh `testStart`
under the same name, composing to the same `id`, which would make the file's `(runner, id)` pair
non-unique (IR §11.4).
**R197 (MUST NOT)** Pass `--timeout`, `--concurrency`/`-j`, `-p`/`-c`,
`--test-randomize-ordering-seed`, `-n`/`-N`/`-t`/`-x`, or any path or directory argument (IR §11.4).
**R198 (MUST)** Read events thus: "A line of the merged stream is an **event** iff it parses as a JSON
object with a string `type` member; every other line is discarded." Only `start`, `suite`,
`testStart`, `testDone` are read, plus `group` for the completeness check and `done` as the terminal
event. `allSuites`, `print`, `error` and `debug` are read for nothing (IR §11.4).
**R199 (REFUSE)** "The `start` event's `protocolVersion` must begin `0.1.`; otherwise this runner's
contribution is `stream-invalid`." Observed: `0.1.1`.
**R200 (MUST)** Compose the id: for a `testDone` event, let `e` be the `testStart` whose `test.id`
equals its `testID`, and `S` the `suite` whose `suite.id` equals `e.test.suiteID`. Let `sp` be
`S.suite.path`; an absolute `sp` under the repository root is made repo-relative. Then (verbatim):

```
id := sp ++ "::" ++ e.test.name
fn := id
```

**R201 (MUST)** **Discard** a `testDone` whose `testID` was introduced by no prior `testStart`, or a
`testStart` whose `suiteID` names no prior `suite` — "this is what makes a forged event that invents
an identity inert" (IR §11.4; conformance R8).
**R202 (REFUSE)** "any other absolute `sp`, or a null one, makes the contribution `stream-invalid`."
**R203 (MUST)** `id → path` is the bytes before the **first** `::`, mapped onto a tree entry (of `B`
for a `base` record, of `T` for a `result` record) and emitted as the tree's bytes; the empty string
where no entry matches (IR §11.4).
**R204 (MUST NOT)** Re-join group prefixes. "`test.name` is already qualified by every enclosing group,
joined with a single `U+0020` … The adapter does not re-join anything" (IR §11.4; conformance R1).
**R205 (MUST)** Apply this **total and ordered, first-match-wins** record table (IR §11.4, verbatim):

| # | Condition | `base` | `result` |
|:--:|---|---|---|
| 1 | `test.groupIDs` is empty — the suite's *load* pseudo-test — and `hidden` is `true` | no | no |
| 2 | `test.groupIDs` is empty and `hidden` is `false` — the suite **failed to load** | no, and on `B` see below | yes, one record, `out = error` |
| 3 | `test.name` is `(setUpAll)` or `(tearDownAll)`, or ends with `U+0020` followed by one of those — a **scaffold** test | **no** | yes |
| 4 | `hidden` is `true` | no | no |
| 5 | otherwise | yes | yes |

**R206 (MUST)** Use the empty `groupIDs` as the load-pseudo-test discriminator, **not the name** —
"every real test sits in at least the suite's implicit root group" (IR §11.4).
**R207 (REFUSE)** "If any suite's load pseudo-test reports a non-`success` result while collecting on
the checkout of `B`, this runner's contribution is **`base-collect-failed`**" — no `base` and no
`result` records from any runner (IR §11.4; conformance R5).
**R208 (MUST NOT)** Put a scaffold test in the floor. `(setUpAll)`/`(tearDownAll)` are "the two literal
names … its whole set" and are reported **only when they fail**; such an id "enters the `B` floor when
the hook is broken on trunk, and the moment the hook is fixed the id disappears" (IR §11.4;
conformance R6). They are still written to the `result` section.
**R209 (MUST)** Run the completeness check per suite: the root `group` (the one with `parentID` null)
declares `testCount`, "the number of real tests in that suite — scaffold and load pseudo-tests
excluded"; compare it with the number of **row-5** records emitted for that suite. Fewer **or** more →
`runner-failed` on `T`, `base-collect-failed` on `B` (IR §11.4; conformance R3).
**R210 (MUST)** Apply this outcome mapping **top to bottom, first match wins** (IR §11.4, verbatim):

| `testDone` observation | `out` |
|---|---|
| `skipped` is `true` | `skipped` |
| `result` is `"success"` | `passed` |
| `result` is `"failure"` | `failed` |
| `result` is `"error"` | `error` |
| any other `result` value | `unknown` |

**R211 (MUST)** Test `skipped` **first** — "`package:test` reports a skipped test as
`{"result":"success","skipped":true}`, so a mapping that read `result` first would credit every
skipped test as a pass" (IR §11.4; conformance R2).
**R212 (MUST)** Note this adapter never produces `xfail`, `xpass` or `deselected` (IR §11.4).

### ALG-S — `swift-test` (IR §11.5)

**R213 (MUST)** Take the **id from SwiftPM's specifier format** and the **outcome from XCTest's
`PrintObserver`**. "Taking the id from SwiftPM and the outcome from XCTest is the whole design of this
adapter."
**R214 (MUST)** Scope to **XCTest**. swift-testing (`@Test`) has no v1 adapter.
**R215 (REFUSE)** "A repository with no `Package.swift` at its root cannot be run by this adapter: the
contribution is `spawn-failed`."
**R216 (MUST)** `B` enumeration: `swift test list --disable-swift-testing`, at the repository root on
the checkout of `B`. "Its **stdout** carries one specifier per line and nothing else; SwiftPM writes
build progress to stderr, which this command's reader discards."
**R217 (REFUSE)** "A non-zero exit, or a non-empty stdout line that is not a specifier, makes the
contribution `base-collect-failed`."
**R218 (MUST)** `B` outcomes: `swift test --disable-swift-testing` — the `T` invocation unchanged, at
the repository root on the same checkout of `B`, stdout and stderr merged into one pipe, `--parallel`
never passed. "Each `base` record's `out` is the value that mapping yields for its id; an id with no
terminal line in that run takes `out: "absent"`" (IR §11.5; conformance R13a, R13b, R13c).
**R219 (MUST)** Compose the id from the specifier line, **byte for byte** (verbatim):

```
id := <target> "." <class-path> "/" <method>
fn := id
```

`fn == id` because XCTest has no parametrization.
**R220 (REFUSE)** swift-testing detection: additionally run
`swift test list --enable-swift-testing --disable-xctest`. "**Any non-empty stdout makes the collector
fail the job and write nothing**"; the finding is `swift-testing-unsupported` (IR §11.5;
conformance R13).
**R221 (MUST NOT)** Pass `--parallel` on `T` or `B`. "`--parallel` runs several `xctest` processes onto
one stream, after which a per-case line cannot be attributed to a process and the multiplicity rule
below cannot tell a second process from a forgery" (IR §11.5).
**R222 (MUST)** Read **both** `PrintObserver` spellings (IR §11.5, verbatim):

| Toolchain | Terminal line |
|---|---|
| Darwin (Objective-C runtime) | `Test Case '-[<target>.<class-path> <method>]' <verb> (<t> seconds).` |
| swift-corelibs-xctest | `Test Case '<class-path>.<method>' <verb> (<t> seconds)` |

`<verb>` is one of `passed`, `failed`, `skipped`; "the closed verb set is what keeps the `started` line
out". The extracted **case identity** is the pair `(class-path, method)`; the corelibs spelling carries
no target (`XCTestCase.name` is `"\(type(of: self)).\(name)"`).
**R223 (MUST)** Terminal event: "the last line matching `Test Suite 'All tests' <verb> at <rest>` with
`<verb>` in `{passed, failed}`" (IR §11.5).
**R224 (MUST)** Join each `(class-path, method)` from the stream to the specifier id from
`swift test list` that **ends `.<class-path>/<method>`**. A case identity in the stream matching no id
in the list is **discarded** (IR §11.5).
**R225 (REFUSE)** "If two ids in that list share a `(class-path, method)` under different targets, the
join is not single-valued on a corelibs toolchain: the collector **fails the job and writes nothing**,
finding `ambiguous-test-class`." It is checked on the listing, once, before either run is joined
(IR §11.5; conformance R14).
**R226 (MUST)** Apply this outcome mapping **top to bottom, first match wins** (IR §11.5, verbatim):

| Observation for the case | `out` |
|---|---|
| more than one terminal line in the run | `unknown` |
| a line matching `<any>Expected failure in <case identity>:<any>` occurred in the run | `xfail` |
| `<verb>` is `passed` | `passed` |
| `<verb>` is `failed` | `failed` |
| `<verb>` is `skipped` | `skipped` |
| no terminal line at all | no `result` record — the id is *absent*, which is not a pass |

**R227 (MUST)** Keep the `Expected failure` row above `passed`: `XCTExpectFailure` makes XCTest print
`Expected failure in <case>: …` and then report the case as **`passed`** (IR §11.5; conformance R11).
**R228 (MUST)** Keep the multiplicity row first — a test can `print` a byte-identical
`Test Case '…' passed (0.001 seconds).` line naming another test; the rule collides the forged line
with the real one and maps to `unknown` (IR §11.5; conformance R12).
**R229 (MUST)** Note that `error`, `xpass` and `deselected` are never produced by this adapter
(IR §11.5).
**R230 (MUST)** Compute `id → path` by a **lexical declaration lookup** over a tree `X`, in these
ordered steps (IR §11.5, verbatim intent):
1. Let `M` be the **longest** target name in `RC(swift, X)` such that the id begins `M ++ "."`. No
   such target → the empty string.
2. Let `C` be the bytes between that `.` and the first `/`, and `c` the last `.`-separated component
   of `C`.
3. Among the source files of `M` in `RC(swift, X)`, a file **declares** `c` iff its token stream
   (lexed by §7.1, with comments and string literals discarded) contains a `word` token `class`
   immediately followed by a `word` token equal to `c`.
4. Exactly one such file → its path, emitted as the tree's bytes (`B` for a `base` record, `T` for a
   `result` record). **Zero or several → the empty string.**

**R231 (MUST NOT)** Pick one of several declaring files. "A candidate that plants a decoy
`class InvoiceTests` in a second file of the same target buys itself the empty string" (IR §11.5;
conformance R10).

---

## Byte-level fixities (verbatim)

1. **Pragma grammar** (IR §12.1): `@verifies <SP>+ <intent-id> "/" "AC-" <digit>+`, `<SP>` ∈
   {`U+0020`, `U+0009`}; `@verifies` preceded by a byte outside `[A-Za-z0-9_@]` or at the comment's
   start.
2. **Intent-id numeral** (ID §3.1 via IR §12.1): "a decimal integer left-padded with `0` to a minimum
   width of 3 and padded no further". `INT-042`, `BUG-051`, `INT-1042` are ids; `INT-42`, `INT-0042`,
   `INT-000`, `int-042` are not.
3. **Sugar pattern** (IR §12.3): `AC` + one or more digits, preceded by a byte outside `[A-Za-z0-9]`
   or at the field start, followed by a byte outside `[0-9]` or at the field end. Capture = the digit
   run.
4. **Sugar field separators** (IR §12.3): pytest `::` (last component of `fn`); vitest ` > ` (last
   component of `id`); dart-test — bytes after the **first** `::`; swift-test — bytes after `/`, with
   a leading `test` removed if present.
5. **`runner` token grammar** (RF §4.4): `[a-z][a-z0-9_-]{0,31}`. No uppercase, no `U+0020`, no
   `U+003A`.
6. **`Spine-Test` payload** (PB §11; RF §6.5): `<runner>` `U+0020` `<runner-native function id>`
   without parametrization suffix; **split at the first `U+0020`**.
7. **`test` node id** (PB §6.2; DM §5.2): `test:` + `<runner>` + `:` + `<runner-native function id>`,
   repo-scoped as `<repo>/test:<runner>:<fn>`.
8. **pytest id** (IR §11.2): nodeid split on `::`; parametrization suffix begins at the **first** `[`
   in the final component and runs to the end, and exists **only if the component's last byte is
   `]`**.
9. **vitest id** (IR §11.3): `<path> > <suites…> > <name>` — repo-relative path, then `" > "`, suite
   titles outermost-first joined by `" > "`, then `" > "`, then the test title. `id → path` = before
   the **first** ` > `.
10. **dart-test id** (IR §11.4): `id := sp ++ "::" ++ e.test.name`; `fn := id`; `id → path` = bytes
    before the **first** `::`. `test.name` already joins group prefixes with a single `U+0020`.
11. **swift-test id** (IR §11.5): `id := <target> "." <class-path> "/" <method>`; `fn := id`.
12. **dart-test protocol guard** (IR §11.4): `start.protocolVersion` **must begin `0.1.`**. Observed
    `0.1.1`.
13. **dart-test scaffold names** (IR §11.4): the closed set is exactly `(setUpAll)` and
    `(tearDownAll)`; a name **ends with `U+0020` followed by one of those** also matches row 3.
14. **XCTest terminal lines** (IR §11.5): Darwin
    `Test Case '-[<target>.<class-path> <method>]' <verb> (<t> seconds).`; corelibs
    `Test Case '<class-path>.<method>' <verb> (<t> seconds)`. Verb set `{passed, failed, skipped}`.
    Session terminal: `Test Suite 'All tests' <verb> at <rest>` with `<verb>` ∈ `{passed, failed}`.
    Expected-failure line pattern: `<any>Expected failure in <case identity>:<any>`.
15. **Swift C-family extension list** (IR §7.3), matched byte-exactly and lowercase only:
    `.m  .mm  .h  .hh  .hpp  .hxx  .pch  .c  .cc  .cpp  .cxx  .modulemap`
16. **Swift manifest trigger labels** (IR §7.3): `publicHeadersPath:`, `cSettings:`, `cxxSettings:`;
    the simple string literal `-import-objc-header`; the callee `.systemLibrary`.
17. **TS extension order** (IR §5.2 step 3/4), exact: `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`,
    `.mjs`, `.cjs`, `.json`.
18. **TS output-extension rewrite** (IR §5.2 step 2): `.js` → `.ts`, `.tsx`; `.mjs` → `.mts`;
    `.cjs` → `.cts`.
19. **Python roots** (IR §4.3), in order: `""`, then `src/` iff a tree entry `src` exists and is a
    directory.
20. **Dart package name regex** (IR §6.3): `^[a-z_][a-z0-9_]*$`.
21. **Pattern byte range** (ID §6.1): 1…255 bytes, each `0x21…0x7E`, excluding `,` (`0x2C`),
    `"` (`0x22`), `\` (`0x5C`).
22. **`match`** (ID §6.3): the two code blocks reproduced verbatim in R122.
23. **`esc`/`tok` on patterns** (ID §6.1): "**`esc` and `tok` are the identity on every legal
    pattern.**"
24. **Path bytes**: every `path` a record carries is "the repo-relative, `/`-separated path … **byte
    for byte as git stores it**", never the filesystem's — "a macOS runner reports NFD where git
    stores NFC" (RF §4.4). No tree entry matches → the empty string.
25. **`closure_digest`** (IR §2.10): "SHA-256 over the RFC 8785 JCS serialization … of the JSON
    **array** of the closure's paths, each `esc`-encoded, sorted ascending by encoded bytes."
26. **Result-file ordering** (RF §4.5): header; every `base` record sorted ascending by the **bytes**
    of `runner`, then by the **bytes** of `id`; every `result` record, same sort; the `end` record.

---

## Error cases

| # | Condition | Behaviour | Code / status token / finding |
|---|---|---|---|
| E1 | Site target undeterminable, in a file with `H` true | tripwire; `--approve` refuses without human `reason=` | `unresolvable-import` (carries `(path, line, specifier-as-written or `<dynamic>`)`) |
| E2 | Same, `H` false, reached by the walk | counter only | `unresolvable-import-outside-harness` |
| E3 | `RC` outside the declarative subset, or `RC(A) ≠ RC(B)`, with a seed or `H`-true file in that language | tripwire, **once per language** | `lang-unclassifiable` + reason token |
| E4 | Same, no `H`-true file in that language | counter; every file of that language excluded | `lang-unclassifiable-outside-harness` |
| E5 | File the walk must lex is not valid UTF-8 | tripwire if `H`, counter otherwise; no edges either way | `file-not-utf8` |
| E6 | Pragma naming an AC in `AC`, in a path no `C-T1` pattern matches (incl. `C-T2`-only) | **`--approve` refuses outright** — not a tripwire | `seed-outside-test-roots` (sorted path list) |
| E7 | `S = ∅` | tripwire; closure empty; `spine stats` `seedless_approvals` | `no-seed` |
| E8 | Row 3 of §2.5 fired ≥ once | tripwire; carries sorted excluded branch-created paths | `closure-tripwire` |
| E9 | `\|closure\| > 200` | tripwire | `closure-too-large` |
| E10 | An `expected` entry matches any `C-T1`/`C-T2` pattern | tripwire | `expected-hits-harness` |
| E11 | Python dotted name resolving to both `x.py` and `x/__init__.py` under the winning root | site `unresolvable` | reason `ambiguous-module` |
| E12 | Python relative import whose level escapes the root | site `unresolvable` | reason `relative-escapes-root` |
| E13 | Python dynamic-import token sequence | site `unresolvable`; argument never inspected | reason `dynamic-import` |
| E14 | TS specifier begins `/` | site `unresolvable` | reason `absolute-specifier` |
| E15 | TS specifier begins `#` | site `unresolvable` | reason `subpath-imports` |
| E16 | TS alias matched, no substitution resolves | site `unresolvable` — **not `external`** | reason `alias-dead-end` |
| E17 | TS base path exhausts candidate expansion | site `unresolvable` | reason `no-candidate` |
| E18 | Root tsconfig/jsconfig unparseable in the JSONC+trailing-comma dialect | language unclassifiable | `tsconfig-unparseable` |
| E19 | `extends` naming a bare specifier, absolute path, or array | language unclassifiable | `tsconfig-extends-external` |
| E20 | `extends` cycle | language unclassifiable | `tsconfig-extends-cycle` |
| E21 | `baseUrl` leaving the repository | language unclassifiable | `baseurl-escapes-root` |
| E22 | `paths` not object-of-array-of-string, or >1 `*` in a key or value | language unclassifiable | `paths-malformed` |
| E23 | Dart URI with an unsupported scheme (`file:`, `http:`, `asset:`) | site `unresolvable` | `unsupported-scheme` |
| E24 | Dart resolved path not an existing file entry | site `unresolvable` | `no-candidate` |
| E25 | Dart `part of <dotted>` with zero or several `library` declarations | site `unresolvable` | `ambiguous-library-name` |
| E26 | Dart specifier not a simple literal | site `unresolvable` | `non-simple-literal` |
| E27 | `pubspec.yaml` `name:` not `^[a-z_][a-z0-9_]*$` | language unclassifiable | `pubspec-name-malformed` |
| E28 | `pubspec.yaml` uses anchors/aliases/merge keys/tags/multi-doc/deep flow | language unclassifiable | `pubspec-not-declarative` |
| E29 | Two Dart packages with the same `name` | language unclassifiable | `duplicate-package-name` |
| E30 | `Package.swift` outside the literal subset | language unclassifiable | `manifest-not-literal` |
| E31 | Two Swift targets with the same `name` | language unclassifiable | `duplicate-target-name` |
| E32 | `.xcodeproj`/`.xcworkspace` and no `Package.swift` | language unclassifiable | `xcode-project-unsupported` |
| E33 | No source directory exists for a target | language unclassifiable | `target-dir-missing` |
| E34 | A path in the source files of two targets | language unclassifiable | `overlapping-targets` |
| E35 | Any target's `F(t)` holds a C-family entry, or its call carries a Test-2 construct, in **either** tree | language unclassifiable, **decided before the `RC(A) ≠ RC(B)` comparison** | `mixed-objc-target` |
| E36 | A Swift file exists and no `Package.swift` does | language unclassifiable | `no-package-manifest` |
| E37 | `RC(lang, A) ≠ RC(lang, B)` (and not E35) | language unclassifiable | `rc-changed-on-branch` |
| E38 | Resolved candidate is mode `120000` or under `160000` | site `unresolvable`; **must not** follow / descend | `symlink-or-submodule` |
| E39 | Two reported items compose to one id under any runner | collector **fails the job and writes nothing** | `duplicate-test-id` (runner + id) |
| E40 | Repository path contains the adapter's `<sep>` (`::` or ` > `) | collector fails the job and writes nothing | `id-separator-in-path` |
| E41 | Two swift-test ids share `(class-path, method)` under different targets | collector fails the job and writes nothing | `ambiguous-test-class` |
| E42 | `swift test list --enable-swift-testing --disable-xctest` yields any non-empty stdout | collector fails the job and writes nothing | `swift-testing-unsupported` |
| E43 | pytest `B` enumeration interrupted by a collection error | no `base` and no `result` records from **any** runner (RF §7.3 all-or-nothing) | `base-collect-failed` |
| E44 | pytest collected-count ≠ extracted-id count | on `T`: `runner-failed`; on `B`: `base-collect-failed` | as named |
| E45 | vitest file fails to load/transform during `B` collection | no records from any runner | `base-collect-failed` |
| E46 | vitest reports a file path not under the repository root | contribution invalid | `stream-invalid` |
| E47 | dart-test `start.protocolVersion` does not begin `0.1.` | contribution invalid | `stream-invalid` |
| E48 | dart-test `suite.path` absolute outside the repo root, or null | contribution invalid | `stream-invalid` |
| E49 | dart-test load pseudo-test non-`success` during `B` collection | no records from any runner | `base-collect-failed` |
| E50 | dart-test suite root `group.testCount` ≠ row-5 records emitted (fewer **or** more) | on `T`: `runner-failed`; on `B`: `base-collect-failed` | as named |
| E51 | No `pubspec.yaml` at repo root (dart) / no `Package.swift` at repo root (swift) | contribution | `spawn-failed` |
| E52 | swift `swift test list` non-zero exit, or a non-empty stdout line that is not a specifier | contribution | `base-collect-failed` |
| E53 | `B` **outcome** run dies / times out / unparsable | **not** `base-collect-failed`; unreached ids take `out: "absent"`; floor unchanged; `end.status` unchanged | `out: "absent"` on those `base` records |
| E54 | Pattern violates ID §6.1 | constitution/touchpoint line refused before any closure is computed | `pattern-illegal-byte`, `pattern-empty`, `pattern-too-long`, `bad-negation`, `leading-slash`, `empty-segment`, `dot-segment`, `bad-globstar`, `bad-bracket` |
| E55 | `pytest --collect-only` over a tree with no tests | exit **`5`**, floor legitimately empty — **must not** be read as a failure | n/a |
| E56 | pytest collection interrupt banner | `!!! Interrupted: 1 error during collection !!!`, exit **2**, under `--collect-only` and a full run alike | `base-collect-failed` on `B` |

Note on exit codes: the only exit statuses the corpus fixes are pytest's **`5`** (no tests collected;
not a failure) and **`2`** (collection interrupted), and swift's **non-zero exit** on
`swift test list` (IR §11.2, §11.5, §11.7). "Exit status is not the signal and must not be used as
one" for completeness (IR §11.2).

---

## Worked examples / test vectors

### V-1 Published closure digests (IR §13, computed not asserted)

| Example | Closure | `closure_digest` | Canonical bytes |
|---|---|---|---|
| §13.1 Python | 12 paths | `sha256:c17cb077493566e549417309f2448343c60259b5621ae8282ca06427831b0ea6` | 278 |
| §13.2 TypeScript | 10 paths | `sha256:da93556c4c3bdb8abfb29c75f3a03a5ae9d3396d96e99bb08dc4172be62070c8` | 261 |
| §13.3 Dart | 9 paths | `sha256:cd83d5c6267e9abd5a72878d9a103765ceb0342bc931ea3f0a07d7b418c06954` | 251 |
| §13.4 Swift | 5 paths | `sha256:8a2d5fbc97efdaf17467daba9f2836caaca14da5424bc5aec7c55117f9d66eff` | 171 |
| Appendix A Kotlin (historical, not v1) | — | `sha256:373236c6b64ad1a5ed2d44ba1d3e099de5f5cbd379fea5f766a2c4d1b93f7237` | 281 |

Every seed in all four is **derived** by §2.1.1 from a `@verifies INT-042/AC-1` pragma in that file's
own bytes; the intent is `INT-042` with `AC-1 … AC-3`. "Every path in these examples is ASCII with no
backslash, so `esc` is the identity on all of them."

### V-2 pytest, reproduced (IR §11.7; CPython 3.14.5 · pytest 9.1.1)

```
$ pytest --collect-only -q                    # stdout
tests/test_invoice.py::test_AC1_totals_include_tax[zero-rate]
tests/test_invoice.py::test_AC1_totals_include_tax[std]
tests/test_invoice.py::test_AC2_zero_rated_lines
tests/test_invoice.py::TestRounding::test_half_even

4 tests collected in 0.00s
```

Deselection under `--collect-only` prints `3/4 tests collected (1 deselected)`; a broken import gives
`ERROR tests/test_broken.py`, `!!! Interrupted: 1 error during collection !!!`, exit **2**; an empty
tree gives `no tests collected` and exit **5**.

`id → fn` vector (RF §6.7, ratified unchanged by IR §11.2):

```
id     = tests/billing/test_invoice.py::test_AC1_totals_include_tax[zero-rate]
  fn   = tests/billing/test_invoice.py::test_AC1_totals_include_tax
  path = tests/billing/test_invoice.py

id     = tests/core/test_util.py::TestRounding::test_half_even
  fn   = tests/core/test_util.py::TestRounding::test_half_even     (unparametrized: fn == id)
  path = tests/core/test_util.py
```

### V-3 vitest, reproduced (IR §11.7; Node 26.0.0 · vitest 4.1.11)

```
$ vitest list                                  # stdout
tests/invoice.test.ts > invoice totals > AC1 includes tax
tests/invoice.test.ts > rate zero-rate
tests/invoice.test.ts > rate std
```

```
$ vitest run --reporter=json                   # ids and statuses, extracted
passed  | tests/invoice.test.ts > invoice totals > AC1 includes tax
skipped | tests/invoice.test.ts > invoice totals > AC2 zero-rated lines
passed  | tests/invoice.test.ts > rate zero-rate
passed  | tests/invoice.test.ts > rate std
skipped | tests/skipped.test.ts > whole suite > never runs
```

**Three against five, and the two missing are exactly the two skipped** — the vector on which
`vitest list` is refused.

### V-4 dart-test, reproduced (IR §11.7; Dart 3.12.0 · `package:test` 1.31.2, `protocolVersion` `0.1.1`)

Root `group` declared `"testCount":5`; five `testDone` events composed to these five `result` records
(RF §4.3 canonical form, §4.5 order):

```json
{"fn":"test/billing/invoice_test.dart::AC1 totals include tax","id":"test/billing/invoice_test.dart::AC1 totals include tax","out":"passed","path":"test/billing/invoice_test.dart","runner":"dart-test","t":"result"}
{"fn":"test/billing/invoice_test.dart::rate std applies","id":"test/billing/invoice_test.dart::rate std applies","out":"passed","path":"test/billing/invoice_test.dart","runner":"dart-test","t":"result"}
{"fn":"test/billing/invoice_test.dart::rate zero applies","id":"test/billing/invoice_test.dart::rate zero applies","out":"passed","path":"test/billing/invoice_test.dart","runner":"dart-test","t":"result"}
{"fn":"test/billing/invoice_test.dart::rounding AC2 banker rounding","id":"test/billing/invoice_test.dart::rounding AC2 banker rounding","out":"skipped","path":"test/billing/invoice_test.dart","runner":"dart-test","t":"result"}
{"fn":"test/billing/invoice_test.dart::rounding half even","id":"test/billing/invoice_test.dart::rounding half even","out":"passed","path":"test/billing/invoice_test.dart","runner":"dart-test","t":"result"}
```

Also reproduced: nested groups give `test.name` `outer inner deep AC2 case`; a failing `tearDownAll`
appears as `outer (tearDownAll)` with `hidden` `false`; a succeeding `setUpAll` has `hidden` `true`; a
non-compiling file yields a `testDone` for `loading <path>` with `"result":"error"` and `hidden`
`false`; two `test('dup name')` calls compose to one id; a forged `testDone` written by a test to
stdout is discarded by the "no prior `testStart`" rule.

### V-5 swift-test, reproduced (IR §11.7; Apple Swift 6.3.2, arm64-apple-macosx)

```
$ swift test list --disable-swift-testing        # stdout
BillingTests.InvoiceTests/testAC1TotalsIncludeTax
BillingTests.InvoiceTests/testAC2BankerRounding
BillingTests.InvoiceTests/testKnownBad
BillingTests.RoundingTests/testFails
BillingTests.RoundingTests/testHalfEven

$ swift test list --enable-swift-testing --disable-xctest   # stdout: empty, exit 0

$ swift test --disable-swift-testing             # merged stream, filtered
Test Suite 'All tests' started at 2026-08-26 20:03:31.283.
Test Case '-[BillingTests.InvoiceTests testAC1TotalsIncludeTax]' passed (0.000 seconds).
Test Case '-[BillingTests.InvoiceTests testAC2BankerRounding]' skipped (0.000 seconds).
InvoiceTests.swift:10: Expected failure in -[BillingTests.InvoiceTests testKnownBad]: XCTAssertEqual failed: ("2") is not equal to ("3")Reason: (known bad)
Test Case '-[BillingTests.InvoiceTests testKnownBad]' passed (0.026 seconds).
Test Case '-[BillingTests.RoundingTests testFails]' failed (0.000 seconds).
Test Case '-[BillingTests.RoundingTests testHalfEven]' passed (0.000 seconds).
Test Suite 'All tests' failed at 2026-08-26 20:03:31.316.
```

giving:

```json
{"fn":"BillingTests.InvoiceTests/testAC1TotalsIncludeTax","id":"BillingTests.InvoiceTests/testAC1TotalsIncludeTax","out":"passed","path":"Tests/BillingTests/InvoiceTests.swift","runner":"swift-test","t":"result"}
{"fn":"BillingTests.InvoiceTests/testAC2BankerRounding","id":"BillingTests.InvoiceTests/testAC2BankerRounding","out":"skipped","path":"Tests/BillingTests/InvoiceTests.swift","runner":"swift-test","t":"result"}
{"fn":"BillingTests.InvoiceTests/testKnownBad","id":"BillingTests.InvoiceTests/testKnownBad","out":"xfail","path":"Tests/BillingTests/InvoiceTests.swift","runner":"swift-test","t":"result"}
{"fn":"BillingTests.RoundingTests/testFails","id":"BillingTests.RoundingTests/testFails","out":"failed","path":"Tests/BillingTests/RoundingTests.swift","runner":"swift-test","t":"result"}
{"fn":"BillingTests.RoundingTests/testHalfEven","id":"BillingTests.RoundingTests/testHalfEven","out":"passed","path":"Tests/BillingTests/RoundingTests.swift","runner":"swift-test","t":"result"}
```

Also reproduced: `swift test --xunit-output` produces a file only under `--parallel`, and that file
records the skipped and expected-failure cases as **plain passing `<testcase>` elements with no
marker** — so the xUnit file is not this adapter's transport. And the forgery: a test printing
`Test Case '-[BillingTests.InvoiceTests testFailing]' passed (0.001 seconds).` gives `testFailing`
two terminal lines → `unknown`.

### V-6 Pattern-match vectors (IR §2.4.2, produced by an ID §6.1–§6.3 implementation)

| Pattern | Path | `match` |
|---|---|---|
| `src/**/__tests__/` | `src/billing/__tests__/x.test.ts` | **yes** |
| `src/**/__tests__/` | `src/billing/__tests__/nested/y.test.ts` | **yes** |
| `src/**/__tests__/` | `src/__tests__/z.test.ts` | **yes** — `**` matches zero segments |
| `src/**/__tests__/` | `src/billing/__tests__` | no — a trailing `/` never matches the directory's own path |
| `src/**/__tests__/` | `src/billing/x.test.ts` | no |
| `tests/` | `tests/a/b.py` | **yes** |
| `tests/` | `tests` | no |
| `tests/` | `testsuite/x.py` | no |
| `**/conftest.py` | `conftest.py` | **yes** |
| `**/conftest.py` | `tests/billing/conftest.py` | **yes** |
| `pytest.ini` | `pytest.ini` | **yes** |
| `pytest.ini` | `tools/pytest.ini` | no — a pattern with no `/` is root-anchored |
| `tests/support/**` | `tests/support` | **yes** — `**` matches zero segments |
| `tests/support/**` | `tests/support/factories.py` | **yes** |
| `vitest.config.*` | `vitest.config.ts` | **yes** |
| `vitest.config.*` | `packages/a/vitest.config.ts` | no — `*` does not cross `/` |
| `Tests/Support/**` | `Tests/Support/Fixtures.swift` | **yes** |
| `test/support/**` | `test/support/index.dart` | **yes** |
| `src/bill` | `src/billing/x.ts` | no — the segment-boundary clause |

Directory-vs-file table (ID §6.3):

| Pattern | Matches `src/billing` itself | Matches `src/billing/x.ts` | Matches `src/billingx/y.ts` |
|---|---|---|---|
| `src/billing/` | no | yes | no |
| `src/billing` | yes | yes | no |

### V-7 Pragma and sugar vectors (IR §14 "The pragma and the sugar")

| # | Case | Required |
|---|---|---|
| J1 | `# @verifies INT-042/AC-1` in a Python comment | one pragma occurrence |
| J2 | `"""@verifies INT-042/AC-1"""` in a Python docstring | **no** occurrence |
| J3 | `// x@verifies INT-042/AC-1` | no occurrence |
| J4 | A pragma in a file from which the runner collected three ids | **three** `verified_by` edges |
| J5 | `def test_AC1_and_AC2_totals` under pytest | two edges, AC-1 and AC-2 |
| J6 | `def test_AC12_totals` | one edge, **AC-12** — not AC-1 |
| J7 | `def test_MAC1_x` | no edge |
| C24 | file under `C-T1` carrying `@verifies INT-042/AC-1`, intent `INT-042` with three ACs | a seed |
| C25 | same file carrying only `@verifies INT-042/AC-9` | **not** a seed, and G5's orphan finding |
| C26 | same file carrying only `@verifies INT-41/AC-1` | not an occurrence at all — not a seed and not an orphan |
| C27 | file under `C-T1` named `test_AC1_totals.py`, no pragma | **not** a seed |
| C28 | no file under `C-T1` carries a pragma for this intent | `S = ∅`, closure `= ∅`, `no-seed` |
| C29 | pragma in `A` whose file no runner would collect | still a seed |
| C30 | `@verifies` inside a Python docstring or any string literal | not an occurrence |

### V-8 `C-T3` predicate vectors (IR §14 T-cases)

T1 `import pytest` in `src/billing/invoice.py` → hit, wire `G8:src/billing/invoice.py`.
T2 same in `tests/conftest.py` with `C-T1` = `tests/` → no hit.
T3 `import { vi } from 'vitest'` in the root `vitest.config.ts` matched by `C-T2` → **no hit**.
T4 `import type { Mock } from 'vitest'` in `src/api.ts` → no hit.
T5 `from unittest.mock import patch` in `src/api.py` → no hit. T6 `from unittest import TestCase` → hit.
T7 `import nose` → no hit. T8 `packages/a/vitest.config.ts` under root-anchored `vitest.config.*` → hit
by basename. T8a root `vite.config.ts` under the scaffolded `C-T2` → **no** hit. T8b
`src/billing/conftest.py` under scaffolded `**/conftest.py` → **no** hit (but a hit in a repository
that removed the pattern). T9 `def pytest_collection_modifyitems(items):` in `src/plugins.py` → hit.
T10 `func setUp()` in `Sources/Billing/Helper.swift` with no `import XCTest` → no hit. T11
`@testable import Billing` in `Sources/Billing/Debug.swift` → hit. T12
`importlib.import_module("pytest")` → no hit. T13 a `.java` file importing `org.junit.Test` → no hit.
T14 two framework imports in one non-harness file → **one** finding and one wire.

### V-9 Adapter conformance vectors (IR §14 R-cases)

R1 dart `group('g'){test('t')}` in `test/a_test.dart` → id `test/a_test.dart::g t`, not re-joined.
R2 `{"result":"success","skipped":true}` → `out` `skipped`, never `passed`.
R3 root `group` `"testCount":5` with 4 records → `runner-failed`/`base-collect-failed`.
R4 `loading <path>` with `hidden` true → no record either section. R5 same with `hidden` false during
`B` → `base-collect-failed`, no records from any runner. R6 `outer (tearDownAll)` → `result` record,
**no** `base` record. R7 duplicate dart test names → `duplicate-test-id`. R8 `testDone` with no prior
`testStart` → discarded. R9 repo path containing `::` under pytest or dart-test →
`id-separator-in-path`. R10 swift id `M.C/testX` with two files declaring `class C` → `path` is the
empty string. R11 `Expected failure` + `passed` verb → `xfail`. R12 two terminal lines → `unknown`.
R13 swift-testing listing non-empty → `swift-testing-unsupported`. R13a swift `B` enumeration and
outcome run are the two named commands. R13b listed swift case with no terminal line in the `B` run →
`base.out` `absent`, id **stays** in the floor. R13c `Expected failure` in the **`B`** run →
`base.out` `xfail`. R14 two swift ids sharing `(class-path, method)` → `ambiguous-test-class`.
R15 every adapter's `fn` is a prefix of its `id`. R16 python `B` enumeration is `pytest
--collect-only`. R16a python `B` outcome run is `pytest`, and **must not** be omitted. R16b python `B`
outcome run killed at `params.timeout` after 3 of 5 → the two unreported ids take `out: "absent"`,
floor still holds five, `end.status` is **not** `base-collect-failed` and **not** `runner-timeout`.
R16c/R16f: `xfail`-on-`B` and `skipped`-on-`B` ids that still collect on `T` raise **no** G1 and **no**
G8 finding. R16d/R16g: the same ids **absent** from `T` are the ordinary went-away allocation —
`G8:<path>` and G1 unless a `class=protected` review names the path. R16e `passed` on `B`, `xfail` on
`T` → G8 **and** G1. R17 `ts` `B` collection is `vitest run`, never a list-only mode. R17a any
`vitest`/`dart-test` `base` record carries an `out` from that adapter's mapping, **never** `xfail`.
R18 pytest deselected id → not in the floor. R19 interrupted pytest `B` collection, or a vitest `B`
file that fails to load → `base-collect-failed`, no partial floor. R20 `pytest --collect-only` over an
empty tree → exit `5`, floor legitimately empty.

### V-10 Swift `mixed-objc-target` vectors (IR §14 S12–S19)

S12 `Sources/Billing/Legacy.m` in a target's file set → `mixed-objc-target`; every Swift file
contributes no edges. S13 an all-`.swift` repository → **no** refusal. S14
`Tests/BillingTests/BillingTests-Bridging-Header.h` → refusal **by the `.h` extension alone**. S15
`exclude: ["Legacy"]` whose only C-family entry is `Legacy/Old.m` → **no** refusal. S16 `.systemLibrary`
or `publicHeadersPath: "include"` → refusal **by test 2**, without any header existing on disk. S17 a
pure Objective-C target a Swift target `import`s by name → refusal. S18 branch adds
`Sources/Billing/Oracle.m` with no manifest edit → `mixed-objc-target`, **not** `rc-changed-on-branch`,
and **must not** pass because `RC` is read from `B`. S19
`swiftSettings: [.unsafeFlags(["-import-objc-header", "Shim.h"])]` → refusal by test 2's string-literal
clause.

---

## Cross-references it depends on (which other sheet owns what)

| Concern | Owner | What this sheet consumes from it |
|---|---|---|
| The freeze-closure **walk**, `class(m)` table, clause 2's base-tree reverse-import query, clauses 3 and 4, `closure_digest` | IR §2.2–§2.10 (the freeze-closure sheet) | `H`, `E`, `FROZEN_WALK`/`FROZEN_LEAF`/`EXCLUDED`, the leaf-prune rule, `AncestorConfig` consumption |
| The result file's grammar, ordering, `end.status` vocabulary, ingestion and §8.5 clauses | RF §4, §5, §7, §8 | the eight `out` values, `absent` on `base` only, `(runner, id)` uniqueness, the `fn`-prefix check, §8.5 clause 2's `xfail`/`skipped` carve-out (the **only** consumer of `base.out`) |
| Collector order of operations, transport, `params.timeout`, `base-collect-failed` / `runner-failed` / `stream-invalid` / `spawn-failed` / `no-output` / `complete` | RF §6.6, §7.1–§7.4 | the status tokens this sheet's adapters raise; the "every `B` invocation precedes every `T` execution" ordering; the all-or-nothing rule |
| `Spine-Frozen` / `Spine-Test` line rendering, `git ls-tree` quoting, the `freeze=` sort | `envelope-vectors.md` (IR §19 explicitly declines it) | the `Spine-Test` payload split at the first `U+0020` |
| `esc`, `tok`, JCS canonicalization, wire tokens | `gate-report.md` §2.1–§2.3, §6.1–§6.2 | `closure_digest`'s encoding; `G8:` + `tok(path)`; `(gate, path)` wire uniqueness |
| Constitution line grammar; how a `C-T1`/`C-T2` line splits into patterns; the rendered scaffold | CN §2, §6.1–§6.4 | the per-language `C-T1`/`C-T2` values (must agree with IR §4.5/§5.5/§6.5/§7.6); `C-T3`'s v1 domain `on` |
| Intent parse, AC numbering, touchpoint list parse | ID §3.1, §5.3, §5.4, §8 | the intent-id domain the pragma reuses; `AC`'s spelling `1 … 6` |
| `test` and `ac` node ids, `verified_by`/`freezes` edge attrs, exclusion set | DM §5.2, §12 | the node id `test:<runner>:<fn>` the join must produce |
| G1/G5/G8 wire spelling, class, overridability, warn mode | PB §6.3, `gate-report.md` §6.3 | this sheet supplies G8's inputs only: the closure and `C-T3`'s predicate |
| `params.langs` domain, G16's floor-relevance check | `manifest.md` §3.3, §6.2 | the four-token domain; the `langs-unknown` refusal |

---

## OPEN items (undecided; do not invent)

1. **OPEN-3 (IR §18)** — whether `RC(lang, A) ≠ RC(lang, B)` should **tripwire or refuse**. §3.3
   specifies a tripwire signable with a `reason=`. Recommendation: tripwire, revisit if `spine stats`
   shows the reason signed routinely. Owner-level.
2. **OPEN-4 (IR §18)** — whether a **second TypeScript adapter (`jest`) ships in v1**. Recommendation:
   vitest only; `jest` reserved now.
3. **OPEN-5 (IR §18)** — whether the **package manifest belongs in the closure** (`pyproject.toml`,
   `package.json` on the ancestor-config list). Consequence: adding a dependency after approval is a
   G8 failure. Recommendation: keep it.
4. **OPEN-6 (IR §18)** — the **200-file closure threshold does not fit Swift** (the closure is
   module-shaped, IR §7.4). Recommendation: count files but exclude clause-3/clause-4 members and
   `FROZEN_WALK` harness files. Owner-level: it changes a published threshold.
5. **OPEN-7 (IR §18)** — **TypeScript monorepos**: `exports`/`imports` maps and per-project tsconfigs
   are not read (§5.2, §5.3). Recommendation: leave it; the workaround is a `tsconfig` `paths` alias.
6. **OPEN-8 (IR §18)** — **does swift-testing (`@Test`) get a v1 adapter?** Today §11.5 detects it and
   fails the job. Recommendation: XCTest only in v1, **reserve the token `swift-testing` now**. The
   token is currently *not* reserved.
7. **OPEN-9 (IR §18)** — `dart-test` collects the `B` floor by running the suite; **one truncation is
   undetectable** (a root group whose own `testCount` is smaller because a load-time `for` loop
   produced fewer cases on `B`). Recommendation: accept and say so in release notes.
8. **OPEN-10 (IR §18)** — the **corelibs spelling of XCTest's `Test Case` line, and
   `XCTestCase.name`, are cited from published source rather than reproduced on Linux.** Everything
   depending on it is §11.5's join and its `ambiguous-test-class` refusal. Recommendation: a Linux
   reproduction is a **release-blocking checklist item**. "It is the only unreproduced byte in this
   document."
9. **OPEN-12 (IR §18)** — **which `runner` tokens are reserved**: three documents give three answers
   (see Contradictions C-1). Recommendation: reserve all of `kotlin`, `gradle`, `jest`, `junit`,
   `kotest`, `swift-testing`. Owner-level; one word in each of three documents.
10. **ID §13 OPEN-2 (via ID §11.7)** — whether to **lift the ASCII restriction on patterns**
    (currently `0x21…0x7E` only). Undecided.
11. **CN §16 OPEN-4 (via CN §6.4)** — whether adding a language to `params.langs` after `spine init`,
    which leaves `C-T2` without that runner's configuration patterns, **should be a wire**. Today
    "Nothing in the design detects it"; `langs_unseeded` reports it.
12. **PB defects still OPEN that bear on this sheet** (IR §17): **D1** (clause 3 names `__init__.py`
    where it means `conftest.py`; `conftest.py` appears nowhere in PB), **D3** ("`C-T1`/`C-T2`/
    runner-config" is three sets in PB §4.3/§6.3 and two in PB §2.1), **D4** (PB §5.2 still says
    "path-prefix matching" and cites no dialect), **D6** ("type-only imports do not count" is stated
    unconditionally for all four languages), **D7** (two sentences describe the same event with
    different subjects), **D8** (`params.langs` defined as the *harness's* languages while the closure
    needs the code's), **D9** (a resolver change invalidates in-flight approvals with no named
    remedy), **D10** (the closure tripwire's "mechanical remedy" is not mechanical in Swift).

---

## Contradictions found

**C-1 · Reserved `runner`/language tokens — three documents, three answers (IR §18 OPEN-12).**
`result-file.md` §6.4 reserves `gradle`, `junit` **and `kotest`** as runner tokens and `kotlin` as a
language token. `import-resolver.md` §11.1 reserves `kotlin`, `gradle` and `jest`, and "has never
mentioned `junit` or `kotest`". `manifest.md` §3.3 says `"kotlin"` is *"not reserved either: a later
release that solves the mixed-module problem adds it as a release, not as a repo setting."* Inert in
v1 (nothing emits any of them); permanent later. **Resolution for an implementer:** emit none of
`kotlin`, `gradle`, `jest`, `junit`, `kotest`, `swift-testing`; treat the hard set `kotlin`, `gradle`,
`jest` as unavailable (IR §11.1 is the runner-token authority per RF §6.4).

**C-2 · The join's granularity is stated over *ids*; the graph node it feeds is keyed by *fn*.**
IR §12.2: "A pragma occurrence in file `P` attributes to **every collected test id** whose `id → path`
equals `P`". But PB §6.2's node id is `test:vitest:<…>` and DM §5.2 fixes the `test` node as
`test:` + `<runner>` + `:` + `<runner-native **function** id>`; RF §6.5 says "AC-1's and AC-2's
`verified_by` edges name *function* ids qualified by runner". For `vitest`, `dart-test` and
`swift-test` this is vacuous (`fn == id`), but for **pytest a parametrized file yields N ids and one
`fn`**. Nothing states which the edge lands on. **Reading an implementer should take:** the edge's
endpoint is the `test` node, whose id is `test:<runner>:<fn>` (DM §5.2 / RF §6.5 are the node-owning
specs); §12.2's "every collected test id" selects *which* tests are attributed, and the resulting edge
set is deduplicated onto their `fn`s. Report as a wording defect in IR §12.2.

**C-3 · CN §6.4 cites a section that no longer exists.** CN §6.4 opens: "`import-resolver.md` §4.5,
§5.5, §6.5, §7.6 **and §8.6** publish a `C-T1` default and a `C-T2` list per language." IR §8 is
"Kotlin — withdrawn … deliberately empty"; there is no §8.6. The values themselves agree with IR
§4.5/§5.5/§6.5/§7.6 exactly, so nothing computes differently — a stale citation, of the class README
calls "citations that outlived the text they cite".

**C-4 · PB §6.3's G2 row assigns this document a list it does not publish under that name.** PB §6.3:
"**New dependency** is a change to a package manifest, whose per-language paths
`docs/spec/import-resolver.md` lists." IR publishes `AncestorConfig(lang)` basename lists (§4.4, §5.4,
§6.4, §7.5) and scaffolded `C-T2` lists (§4.5, §5.5, §6.5, §7.6), but no list is labelled "package
manifest paths" and the two candidate lists differ (e.g. `AncestorConfig(ts)` carries
`vitest.setup.ts`, which is not a package manifest). An implementer computing G2's new-dependency wire
has no unambiguous input. **Reading:** the package manifests are `pyproject.toml`/`setup.cfg` (python),
`package.json` (ts), `pubspec.yaml` (dart), `Package.swift`/`Package.resolved` (swift) — the same set
`spine init`'s language detection uses (PB §11 CLI) — but this is inference, not a specified list.

**C-5 · PB §4.3's clause 3 names `__init__.py` for a hazard only `conftest.py` has** (IR §17 D1,
OPEN). PB: "runner configuration and package `__init__.py` files on the path from repo root to each
test — a root setup file can deselect every test below it without touching one." "`__init__.py` cannot
deselect anything." `conftest.py` appears **nowhere in the playbook**. IR §4.4 puts both on
`AncestorConfig(python)`, which is the resolving list.

**C-6 · PB §4.3's "type-only imports do not count" is unconditional; only one of four languages has
one** (IR §17 D6, OPEN). IR §3.6 fixes the per-language answer, including that a Python import under
`if TYPE_CHECKING:` **is** an ordinary import site. Two implementers reading PB alone make opposite
calls.

**C-7 · PB §4.3 describes the same event twice with different outcomes** (IR §17 D7, OPEN): "a module
whose imports cannot be resolved statically is unclassifiable and stays excluded, counted by
`spine stats`" (silent, counted) versus "an unresolvable or dynamic import inside test roots" as an
approval tripwire (loud, human). A test file with a dynamic import satisfies both. IR §3.8/§16.3
resolve it **by level**: the first sentence attaches to a **site**, and the tripwire is the narrower
rule that wins inside the harness.

**C-8 · `constitution.md` §14.15's closing sentence is withdrawn by IR §2.4.1 but still stands in
CN.** IR: "**`constitution.md` §14.15's closing sentence — *'Where they agree, nothing turns on it'* —
is false and is withdrawn here.** … §14.15 should strike it." The falsifying value is the shipped
`C-T1` `src/**/__tests__/`, which matched nothing under version 1's dialect. Until CN §14.15 is
edited, the corpus carries a withdrawn claim in the document that made it.

**C-9 · PB §5.2 still prescribes "path-prefix matching"** (IR §17 D4, OPEN; ID §6 opening). ID §6.3 is
explicit that a literal reading is wrong — "**`src/bill` would match `src/billing/x.ts`**" — and
`constitution.md` §14.15 adjudicated ID §6.1–§6.3 as the one dialect. PB §6.3's G2 comment now cites
ID, but PB §5.2 does not. **Resolution:** ID §6.1–§6.3 governs (adopted by IR §2.4); PB §5.2's phrase
is a defect.

**C-10 · IR §11.6's list is misnumbered.** The numbered list runs 1, 2, 3, 4, 5, **4, 5, 6** — two
items are numbered `4` ("The `B` outcome is the adapter's own mapping" and "`fn` is a prefix of
`id`") and two `5` ("`xfail` is producible by two of the four adapters" and "The six obligations …
are discharged here"). Citations elsewhere in the corpus to "§11.6 rule 4" and "§11.6 rule 5" are
therefore ambiguous by text alone; IR §11.1 and RF §4.4 disambiguate by quoting the rule they mean
(the `B`-mapping rule is the one cited as "rule 4"; the `xfail`/`skipped` reachability rule as
"rule 5").

**C-11 · IR §12.3's swift-test field trims a leading `test`; IR §11.5's `id → fn` does not.** These do
not conflict — `fn == id` for swift-test and the trim applies only to the **sugar field** — but an
implementer who reuses one function for both produces wrong edges. Stated here because §12.3's own
prose ("with a leading `test` removed if present") sits one table away from §11.5's "`fn := id`".
