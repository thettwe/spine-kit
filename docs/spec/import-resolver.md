# The per-language import resolvers

**Artifact:** the four static resolvers behind the freeze closure — the function that turns *the approval tree* into *the set of repository paths `Spine-Frozen` must name*, seeds included (§2.1.1), recomputed by G8 in `--ci` on every landing — together with the four runner adapters that name the tests it freezes (§11) and the two other lexical reads G1, G5 and G8 take from the same files (§12).
**Home in the playbook:** PB §4.3 ("What is frozen: the closure, not the file list"), PB §6.3's G8 row, PB §2.1's `C-T1`/`C-T2`/`C-T3`, and `params.langs` in PB §6.7. Vocabulary from PB §11, which wins over prose here as it wins there.
**References:** `PB §n` cites `PLAYBOOK.md`; a bare `§n` cites this document. The numbering schemes collide — PB §4.3 is the freeze rules, §4.3 is Python's specifier resolution — so every citation says which.
**Spec version:** 4 · **Covers:** PLAYBOOK.md v0.19 · **Status:** normative for v1. It is the second normative precondition `gate-report.md` §5.4.2 declares, and the pointer `dump.md` §16 declares. It depends in turn on `docs/spec/intent-doc.md` for the one path-pattern dialect — §2.4 adopts §6.1–§6.3 there and defines none of its own — and on `docs/spec/constitution.md` for how a `C-T1`/`C-T2` line splits into a list of patterns.

**What version 4 changed.** One repair from the third cross-document review, and it is the one that made a PLAYBOOK rule computable rather than merely stated.

- **Every adapter now produces the `B` *outcome*, not only the `B` id set** (§11.1, §11.2–§11.5, §11.6 rules 4 and 5). PB §6.3's G1 row exempts, in limb (ii), *"an id in the `B` floor — **never for a frozen id** — where that id's own collected outcome on `B` was already `xfail` or `skipped` and it still collects on `T`"*, and PB §6.3's G8 row carves out the same case in its own words — *"an id whose own collected outcome on `B` was already `xfail` or `skipped` and which still collects on `T`"* — but a `base` record carried `{id, path, runner, t}` and no outcome, and **two of the four adapters produced no `B` outcome at all**: `pytest --collect-only` stops before the first `call`, and `swift test list` prints specifiers. The exemption was therefore uncomputable in a Python or a Swift repository, which is precisely where `xfail` exists. `result-file.md` §4.4 adds `out` to the `base` record and §6.3 adds obligation 6; this document says, per adapter, what produces it. **`vitest` and `dart-test` pay nothing** — they already ran the whole suite on `B` and discarded the outcomes. **`pytest` and `swift-test` pay one more full run of the suite against `B` on every landing**, because neither runner's expected-failure polarity is decidable without running the test. §11.1 states that cost in the same table as the commands.

**What version 3 changed.** Three repairs from the second cross-document review. Each of the first two failed every gated landing or left a gate with no predicate at all:

- **The freeze-closure seed `S` is lexical** (§2.1, §2.1.1). Version 2 derived it from `verified_by` edges; §12.2 makes such an edge require a *collected* test id, `--approve` writes no result file (PB §4.3), and G8's `--ci` recomputation holds no collection over the approval tree — so version 2's `S` had no obtainable input at either end, and the omitted-seed hole PB §4.3 closed reopened. `S` is now **computed** from the inputs both computations hold: every path in `A` under a `C-T1` root whose bytes carry a §12.1 pragma naming an acceptance criterion of this intent. §12.3's naming sugar does **not** seed, and §16.12 says why.
- **`C-T3` has a predicate** (§12.4). PB §2.1's rule — now *"no test-framework import or runner hook defined outside the harness (`C-T1` ∪ `C-T2`)"*, and *"outside test roots"* until §17 D12 was taken — had no framework set and no hook form in any of the four languages, while PB §7.4 rests part of its isolation argument on the grep. §12.4 closes both sets per language, in the lexical shape §12.1 already uses, evaluates them over `H` in all three documents, and §19 no longer declines the clause.
- **The `B` collection command is fixed for `pytest` and `vitest`** (§11.1–§11.3), against output reproduced on a real toolchain (§11.7), closing §18 OPEN-11. `vitest list` is refused with the vector that refuses it: it omits every skipped test, which is a floor smaller than `B`'s real one.

Three smaller repairs ride with them: §11.1's reserved tokens are one table rather than a paragraph that three sibling documents contradict; §12.1 cites `intent-doc.md` §3.1 for the intent id instead of a rival grammar that admits `INT-42`; and §17 D11 records the one five-language claim that survives in the playbook.

**What version 2 changed.** Two owner decisions of 2026-08-26, executed here rather than filed, plus the gap they exposed:

- **v1 ships four languages — Python, TypeScript/JavaScript, Dart and Swift. Kotlin is dropped**, and the `gradle` adapter with it. Version 1's Kotlin analysis is preserved unaltered as **Appendix A**, which also states plainly why it was dropped, so that a later release adding Kotlin does not redo the work.
- **The path-pattern dialect is `intent-doc.md` §6.1–§6.3.** Version 1's §2.4 defined a rival dialect. `constitution.md` §14.15 adjudicated between the two and directed that this document's §2.4 "must be corrected"; §2.4, §16.11 and §17 D4 execute that. Under version 1's §2.4 the shipped `C-T1` value `src/**/__tests__/` matched nothing, so a TypeScript repository using the scaffolded default had an empty harness predicate, an empty closure, and a G8 that rejected every approval.
- **`dart-test` and `swift-test` are ratified** (§11.4, §11.5). Version 1 §11.4 declined all three remaining adapters, which left `Spine-Test` unwritable and no Dart or Swift repository able to land at all. They are ratified against reporter output reproduced on a real toolchain; §11.7 records the toolchains, the commands and the observed bytes, and names the one fact taken from source rather than reproduced.

---

## 1. What this artifact is, and what rests on it

PB §4.3 says the thing that makes this the highest-stakes document in the directory, and it is worth quoting exactly:

> In `--ci`, G8 recomputes the closure over the approval commit's tree with the pinned release and fails if any file it computes is missing from `Spine-Frozen` — an approval signed by a newer or older binary cannot under-freeze.

So the closure is not computed once and recorded. It is computed at `--approve` by one binary and **recomputed at every landing** by another, over the same tree, and the two must agree. A resolver that differs from another on a single edge case does not produce a different opinion; it produces a `G8` failure whose only exits are a signed reopen or a counted freeze override (PB §6.3, G8 "never runs in warn-before-block mode"). **It rejects an approval that was valid.**

Three constraints follow, and every rule in this document is downstream of them.

- **Totality.** Every construct in every file the walk reaches has a defined disposition. There is no "the resolver does its best."
- **Determinism.** The closure is a pure function of the two git trees named in §2.1 and the pinned release. Two runs of one release over the same inputs produce the same set, on any host, in any order, with any working tree.
- **Environment independence.** Nothing is read from an installed interpreter, an SDK, a package manager, a lockfile's resolved contents, a `node_modules` directory, a `.build` directory, an environment variable, or the host platform. PB §4.3: *"Resolution is static and environment-independent: repo-local imports resolve from the tree alone."* Where a language's real semantics depend on a build configuration, this document does not read the configuration — it over-approximates (§3.7), because the failure mode of under-approximating is an unfrozen oracle and the failure mode of over-approximating is a file the branch may not edit.

**What this document is not.** It is not a compiler, a type checker, or a general dependency analyser. It answers one question per import site — *which repository path, if any, does this specifier name* — and three per file: *which language is this* (§3.1), *does it carry a pragma, and which criteria does the pragma name* (§12.1, which §2.1.1's seed rule and G5 both read), and *does it reach a test framework or define a runner hook from outside the harness* (§12.4, which is `C-T3`'s predicate and nothing more). All four are lexical, all four are answered from tree bytes alone, and everything else is out of scope (§19).

---

## 2. The closure, as an algorithm

PB §4.3 states the closure in four numbered clauses and three classification rules spread over one paragraph. This section restates it as an algorithm with no prose left in it. Where the restatement makes a choice, §16 records what the playbook said and why the choice was made.

### 2.1 The closed input set

The closure is a function of exactly these, and of nothing else:

| Input | Where from |
|---|---|
| `A` — the **approval tree** | the tree of the approval commit (PB §4.3: `--approve` "freezes the branch HEAD's tree"; G8 recomputes "over the approval commit's tree") |
| `B` — the **base tree** | the tree of the commit named by the approve line's `base=` (PB §4.3, PB §11) |
| `E` — the intent's **expected** touchpoints | the signed intent blob's "Expected to change" list (PB §3.1) |
| `AC` — the intent's **acceptance-criterion numbers**, and with them its **id** | both from the signed intent blob: `intent-doc.md` §5.3's Acceptance criteria section gives the set `{1 … k}`, contiguous from 1, `1 ≤ k ≤ 6`, and §3.1 gives the id its title line carries — the id the path `intents/<ID>.md` must agree with, and the one §2.1.1 matches a pragma against |
| `C-T1`, `C-T2` | the constitution at `base` (PB §7.4 rule 1: policy is read from trunk, and `base` is trunk's tip at approval) |
| `langs` | `params.langs` in `.spine/manifest.json` at `base` (PB §6.7, PB §7.4 rule 1) |
| the pinned release | `cli.version` + `cli.dist_hash` at `base` (PB §7.4 rule 2) |

Not inputs: the working tree, `HEAD`, any ref other than the two commits above, any note, any cache, the clock, the host, the locale, the filesystem's case sensitivity or normalization, and the order in which git enumerates a tree. **Nor is any of these**: a result file, a runner, a collected test id, a `verified_by` edge, or any other product of executing repository code. That second exclusion is load-bearing rather than tidy, and §2.1.1 is why.

The **seed set `S`** is not in the table because it is not supplied. It is computed from the rows above (§2.1.1), which is what makes the closure a function of two trees and a signed blob and nothing else.

### 2.1.1 The seed set `S`, and why it is lexical

PB §4.3, in the sentence this section implements:

> (1) the seed set — every file under a `C-T1` test root, in the approval tree, carrying a pragma naming an acceptance criterion of this intent. The seed is **lexical, not collected**

So:

> `S` = every path `p` in `A` such that `match(P, p)` holds for at least one pattern `P` in `C-T1` (§2.4), and `p`'s bytes in `A` carry at least one §12.1 pragma occurrence whose intent id is this intent's and whose acceptance-criterion number is in `AC`.

Five things follow, and each is a rule rather than a gloss.

- **Nothing a collection produces is read.** Version 2 of this document defined `S` as *"every test file with a `verified_by` edge to this intent"*, and that definition cannot be evaluated anywhere it is needed. §12.2 makes a pragma's edge land on *"every **collected** test id whose `id → path` equals `P`"*, so a `verified_by` edge presupposes a runner collection; PB §4.3 says plainly that *"`--approve` writes no result file"*; and G8's `--ci` recomputation holds `A`, `B` and the pinned release and no collection over either. A seed set with no obtainable input is not a stricter closure, it is an unimplementable one — and the specific hole PB §4.3 names would reopen, because *"taking it from the approval's own `Spine-Test` lines would let an approval that omits a seed omit that seed's whole subtree and still pass the recomputation in `--ci`, which is the case that check exists to catch"*.
- **Both computations hold every input.** `--approve` has `A` (the branch HEAD's tree it freezes), the signed blob — hence `AC` and `E` — and `C-T1` from `base`. `--ci` has `A` from the approval commit, `AC` and `E` from the blob the approve line's `intent=` names, and `C-T1` from the commit its `base=` names. Neither needs a ref, a note, a cache or a process. That symmetry is exactly what PB §4.3's *"an approval signed by a newer or older binary cannot under-freeze"* requires, and it is the property version 2's definition did not have.
- **The pragma decides, not the file's name.** A test file whose only tie to its criteria is §12.3's `AC<n>` naming sugar is **not** a seed. PB §4.3 says *"carrying a pragma"*, and the sugar is not carried by a file — it is a pattern over a runner-native id's field, which again presupposes a collection. §16.12 records the choice, what it costs, and the two things that bound the cost.
- **A pragma this intent does not own seeds nothing.** `AC` is the membership test, so `@verifies INT-041/AC-1` in this intent's tree, and `@verifies INT-042/AC-9` where the intent has three criteria, are both occurrences (§12.1 recognizes them, which is what makes G5 able to report them) and neither is a seed. The second is G5's orphan finding, and PB §6's transition table makes *"G5 clean"* an `--approve` guard in its own right, so that approval is refused before the closure matters.
- **The seed's own class is fixed by construction.** `H` holds for every member of `S` — `C-T1` is a conjunct of the definition — so row 1 of §2.5 applies and every seed is `FROZEN_WALK`. §2.6's first property is now true by definition rather than by argument.

**Which files carry a seeding pragma and are not seeds.** A path in `A` carrying a §12.1 pragma whose intent id is this intent's and whose AC number is in `AC`, and which **no `C-T1` pattern matches**, is the finding `seed-outside-test-roots` (§2.11) and `--approve` refuses outright. That covers two shapes and refuses both: a file outside the harness entirely, where the pragma is in the wrong file and PB §7.1's provenance rule would index it `attributed: false` and fail G5 at landing anyway; and a file matched only by `C-T2` — a root `vitest.config.ts`, a `**/conftest.py` outside `tests/` — where either the pragma or `C-T1` is wrong. The remedy is the one PB §7.3 already prescribes: move the pragma, which is a branch edit, or widen `C-T1`, which is a floor-protected landing.

**An empty `S`.** The closure is then empty and `Spine-Frozen` names nothing. That is the `no-seed` tripwire (§2.11) — a human signs, with a `reason=`, that this intent's tests name their criteria by no pragma at all. §16.12 argues it.

### 2.2 Two trees, and which question each answers

The playbook uses `A` and `B` for different jobs and never says so in one place. It matters, so it is fixed here:

- **`A` answers "does this path exist, and what does this file import."** Every resolution — every specifier-to-path lookup, every existence test, every candidate expansion — is performed against `A`. The closure names paths in `A`, because `Spine-Frozen` pairs a blob with a path and the blob is `A`'s.
- **`B` answers "was this already here, and did non-test code already use it."** The two `base=` questions of PB §4.3's clause 2 — *existed at `base=`*, and *was imported there by a non-test file* — are evaluated wholly in `B` (§2.9).
- **`B` also answers "what is the resolution configuration."** §3.3. This is not in the playbook; it is added, and §16.4 says why.

PB §4.3's own reason for the second bullet is the one that governs: *"It is read from the base tree, which the branch cannot edit."*

### 2.3 The harness predicate `H` and the expected predicate `E`

For a repository path `p`:

- `H(p)` is true iff `p` matches any pattern in `C-T1` ∪ `C-T2` (§2.4). `C-T2` is where the per-runner configuration patterns live; §4–§7 fix, per language, exactly which patterns `spine init` renders into it. PB §4.3 and PB §6.3 write "`C-T1`/`C-T2`/runner-config" as three sets; PB §2.1's `C-T2` text says the per-runner config *is* `C-T2`. They are the same set, and the three-way phrasing is filed as a defect (§17, D3).
- `E(p)` is true iff `p` matches any entry of the intent's `expected` touchpoints (§2.4).

`H` and `E` are independent; both may hold. PB §4.3: *"Runner-config patterns match at any depth, including inside `expected`."*

### 2.4 Path patterns: the matching dialect

`C-T1`, `C-T2`, `C-Q1`, `C-A2` and the intent's touchpoint entries are all path patterns, and **one dialect governs all of them: `intent-doc.md` §6.1 (the byte grammar and the refusal list), §6.2 (the glob dialect) and §6.3 (`match(P, p)`), adopted here by reference and unaltered.** This document defines no pattern syntax and no matching rule of its own, and where a reader wants to know what a pattern means, `intent-doc.md` §6.1–§6.3 is the answer and this section is a pointer to it.

Concretely, adopted and not restated: patterns are 1…255 bytes drawn from `0x21…0x7E` less `,`, `"` and `\`; `*` does not cross `/` and `**` does, only as a whole segment, matching zero or more segments; `[ … ]` is a bracket expression and `{`/`}` are ordinary bytes; a leading `!` and a leading `/` are refused; and `match(P, p)` is **segment-boundary** matching — a pattern matches the whole path, or a prefix of the path ending exactly at a `/`, and a pattern ending in `/` gives up the first clause. The refusal names (`bad-globstar`, `bad-bracket`, `pattern-illegal-byte`, …) are `intent-doc.md` §6.1's; which command refuses is `constitution.md`'s for a constitution line and `intent-doc.md` §8's for a touchpoint line.

Everything in this document that says "matches" — `H` and `E` (§2.3), `expected-hits-harness` (§2.11), clause 4's `C-T1` test (§2.8), and the scaffolded `C-T2` lists of §4.5, §5.5, §6.5 and §7.6 — means `match` as defined there, against a repository path exactly as git stores it, byte-wise, with no case folding and no normalization.

#### 2.4.1 Why this is a pointer, and what it cost to make it one

Version 1 of this document defined a rival dialect here, on the ground that no document had yet said what a path pattern meant. `intent-doc.md` §6 then said it, and the two disagreed observably in two places (`constitution.md` §14.15's table): version 1's rule 3 anchored a pattern with no trailing `/` at **both ends**, where §6.3 also matches at a segment boundary; and version 1's rule 4 made `[`, `]`, `{`, `}`, `!` and `\` **invalid in any position**, where §6.1–§6.2 admit brackets, treat braces as ordinary bytes, and refuse only a *leading* `!`.

`constitution.md` §14.15 adjudicated: **`intent-doc.md` §6.1–§6.3**, for three reasons of which the first is decisive — G2's quick-lane clause is *"⊆ `C-Q1` ∪ floor ∪ spine-owned paths"*, so a constitution list and a touchpoint list are compared against one diff by one gate, and one semantics is not a preference there. It also recorded that version 1's §2.4 "must be corrected". **This section is that correction.** Any implementation still carrying version 1's rules 1–5 is non-conforming, and the two rules to delete by name are its rule 2 (trailing `/` as a raw byte prefix) and its rule 4 (the six invalid bytes).

**`constitution.md` §14.15's closing sentence — *"Where they agree, nothing turns on it"* — is false and is withdrawn here.** It is repeated nowhere in this document, and §14.15 should strike it. The value that falsifies it is shipped, is scaffolded by `spine init`, and is in PB §2.1 and in §5.5 below:

> `C-T1: test roots: tests/, src/**/__tests__/`

Under version 1's rule 2 a trailing-`/` pattern matched a path iff the pattern's own bytes were a byte prefix of it. The bytes `src/**/__tests__/` are a prefix of no repository path — a real path has a directory name where the pattern has `**` — so **the pattern matched nothing.** For a TypeScript repository whose tests live under `src/**/__tests__/` and not under `tests/`, `H` was false for every file, so no file could be a seed, every `@verifies` pragma in the repository raised `seed-outside-test-roots`, and `--approve` refused outright — and a repository that carried no pragma at all got the other end of the same failure: an empty closure, and a G8 containment check comparing an empty set against an approval that froze real files. The failure is silent in the sense that matters: nothing in the dialect says a pattern that matches nothing is suspicious.

#### 2.4.2 The vector, computed

Produced by an implementation of `intent-doc.md` §6.1–§6.3, and consistent with the row published at `intent-doc.md` §9.5 for the same pattern.

| Pattern | Path | `match` | Under version 1 §2.4 |
|---|---|---|---|
| `src/**/__tests__/` | `src/billing/__tests__/x.test.ts` | **yes** | **no** — the defect |
| `src/**/__tests__/` | `src/billing/__tests__/nested/y.test.ts` | **yes** | no |
| `src/**/__tests__/` | `src/__tests__/z.test.ts` | **yes** — `**` matches zero segments | no |
| `src/**/__tests__/` | `src/billing/__tests__` | no — a trailing `/` never matches the directory's own path | no |
| `src/**/__tests__/` | `src/billing/x.test.ts` | no | no |
| `tests/` | `tests/a/b.py` | **yes** | yes |
| `tests/` | `tests` | no | no |
| `tests/` | `testsuite/x.py` | no | no |
| `**/conftest.py` | `conftest.py` | **yes** | yes |
| `**/conftest.py` | `tests/billing/conftest.py` | **yes** | yes |
| `pytest.ini` | `pytest.ini` | **yes** | yes |
| `pytest.ini` | `tools/pytest.ini` | no — a pattern with no `/` is root-anchored | no |
| `tests/support/**` | `tests/support` | **yes** — `**` matches zero segments | yes |
| `tests/support/**` | `tests/support/factories.py` | **yes** | yes |
| `vitest.config.*` | `vitest.config.ts` | **yes** | yes |
| `vitest.config.*` | `packages/a/vitest.config.ts` | no — `*` does not cross `/` | no |
| `Tests/Support/**` | `Tests/Support/Fixtures.swift` | **yes** | yes |
| `test/support/**` | `test/support/index.dart` | **yes** | yes |
| `src/bill` | `src/billing/x.ts` | no — the segment-boundary clause | no |

**The four published closures of §13 are unaffected.** Every `C-T1` and `C-T2` value those examples use (`tests/`, `test/`, `Tests/`, `src/**/__tests__/`, the six §4.5 patterns, the nine §5.5 patterns, the four §6.5 patterns and the three §7.6 patterns) gives the same answer on every path in every one of those four trees under both dialects — the last column above is the check — because none of those trees contains a path under a `src/**/__tests__/` directory. The four `closure_digest` values in §13 were recomputed under this dialect and are unchanged.

### 2.5 The classification function

For a repository path `m` that the walk has reached by an import edge, `class(m)` is exactly one of `FROZEN_WALK`, `FROZEN_LEAF`, `EXCLUDED`. The table is total over the four combinations of `H` and `E` plus the two `B`-questions:

| `H(m)` | `E(m)` | `m` in `B` | non-test importer of `m` in `B` (§2.9) | `class(m)` | PB §4.3 sentence |
|:--:|:--:|:--:|:--:|---|---|
| yes | any | any | any | `FROZEN_WALK` | "Everything else in the walk is frozen" + "the walk prunes at an excluded import" (so a non-excluded member is walked) |
| no | yes | yes | yes | `EXCLUDED` | "it resolves into a module that existed at the approval's `base=` and was imported there by a non-test file" |
| no | yes | no | — | `EXCLUDED` | "or into a module that did not exist at `base=` at all — the stub the red tests import" |
| no | yes | yes | no | `FROZEN_LEAF` | "the module existed at `base=` and no non-test file imported it there" |
| no | no | any | any | `FROZEN_LEAF` | "an import that resolves outside both expected and the harness is frozen as a leaf, because A had no business touching it" |

`FROZEN_WALK` — the path joins the closure and its own imports are walked.
`FROZEN_LEAF` — the path joins the closure and its imports are **not** walked.
`EXCLUDED` — the path does not join the closure and its imports are not walked. PB §4.3: *"The walk **prunes at an excluded import** — what code under test imports is code under test."*

**The unit of classification is a file path, never a module.** PB §4.3 says "module" throughout, and for Python, TypeScript and Dart a module is a file, so nothing turns on it. For Swift a module is a target, so the word is ambiguous exactly where it is dangerous: a branch-created *file* inside an existing *module* would be "at `base=`" on the module reading and would silently escape the closure tripwire. `Spine-Frozen` names files, so classification names files. §16.6 records the choice; §7 keeps module granularity where it belongs — in the *edge* set, not in the classification.

**Row 3 raises the closure tripwire.** PB §4.3: `--approve` "raises a **closure tripwire** listing every one of them a frozen test imports". The finding is `closure-tripwire`, it carries the sorted list of the excluded branch-created paths, and `--approve` refuses without a human `reason=` (PB §11, `Spine-Approve`; PB §6.3 G13 refuses its absence). That limb of G13 is evaluated **at `--approve` only**, where the closure is in hand — `manifest.md` §4.8.4 check 13 and §9 R25 — so a landing never recomputes the closure to decide whether `reason=` was owed. Rows 2 and 4 raise nothing.

### 2.6 The walk

```
frozen  := ∅                       # set of repository paths
seen    := ∅
queue   := S                       # clause 1 seeds, each FROZEN_WALK by construction

while queue is not empty:
    f := remove any element of queue          # order is immaterial; see §2.12 rule 3
    if f ∈ seen: continue
    seen := seen ∪ {f}
    frozen := frozen ∪ {f}
    if class(f) = FROZEN_LEAF: continue       # leaves are added, never walked
    for each import site i in imports(f, A):  # §3.2
        case disposition(i) of
            external, type_only:  nothing
            unresolvable:         record a finding (§2.11); no edge
            repo(m):
                if class(m) = EXCLUDED:  record m if row 3 of §2.5; no edge
                else: queue := queue ∪ {m}

frozen := frozen ∪ clause3(S)      # §2.7
frozen := frozen ∪ clause4()       # §2.8
```

Three properties, each load-bearing:

- **Seeds are never excluded.** `S` is defined by a `C-T1` match (§2.1.1), so `H` holds for every seed and row 1 of §2.5 applies to it — by construction, not by argument. The hazard the old wording caught has moved with the definition rather than disappeared: a file carrying a pragma for this intent that no `C-T1` pattern matches is not a seed but a `seed-outside-test-roots` refusal, because the constitution's `C-T1` does not cover the repository's own test layout, and the remedy is a constitution change (a floor-protected landing, PB §7.3) or a pragma in the right file.
- **`seen` is keyed by path, so the walk terminates** on any tree, including one with import cycles. Cycles are legal in Python and TypeScript and are neither an error nor a finding here.
- **A path reached both as `FROZEN_WALK` and as `FROZEN_LEAF` cannot happen**, because `class` is a function of the path alone and not of how it was reached. This is why `class` reads no state from the walk.

### 2.7 Clause 3 — configuration on the ancestor chain

PB §4.3 clause 3: *"runner configuration and package `__init__.py` files on the path from repo root to each test — a root setup file can deselect every test below it without touching one."*

`clause3(S)` = for each seed `s ∈ S`, for each directory `d` that is the repository root or a proper ancestor directory of `s` **or the directory containing `s`**, for each name `n` in `AncestorConfig(lang(s))` (§4–§7), the path `d + "/" + n` if it exists in `A`.

Fixed points:

- The chain includes the seed's own directory and the repository root, and every directory between.
- Membership is by **path and basename only, never by file content.** A file is runner configuration because of where it is and what it is called, not because it contains a `[tool.pytest.ini_options]` table. A content test would make the closure depend on a TOML/JSON/YAML parser and would let a branch change the closure by adding a section. §16.2 records the choice; §18, OPEN-5 records its cost.
- Clause 3 files enter the closure as `FROZEN_WALK` when `H` holds for them, which it does whenever `C-T2` carries their pattern — and §4–§7 render exactly those patterns. A clause-3 file that is *not* matched by `C-T2` (a repository that edited the scaffolded rule) is added as `FROZEN_LEAF`, because `class` says so.
- `__init__.py` is Python's entry in `AncestorConfig`; it is not a general clause. PB's own rationale — a root setup file that deselects — is `conftest.py`'s, not `__init__.py`'s, and `conftest.py` appears nowhere in the playbook (§17, D1).

### 2.8 Clause 4 — snapshots and goldens

PB §4.3 clause 4: *"snapshot and golden files under test roots — an expectation written by the implementation is not a test of intent, so it exists before approval or the test does not."*

`clause4()` = every path in `A` that satisfies `H` by a `C-T1` pattern **and** whose final path component matches one of the snapshot patterns of §4–§7, or which lies under a directory whose name is in that language's snapshot-directory set.

They are added as `FROZEN_LEAF`: a snapshot is data, is not resolved for imports, and is not a source file in any of the four languages.

Clause 4 is **not** restricted to snapshots the seeds reach. It is every snapshot under a test root, because the playbook's rationale is about the file existing before approval, and a snapshot only some other test reaches is exactly as much an expectation.

### 2.9 The base-tree importer predicate

`nonTestImporter(m)` — used by rows 2 and 4 of §2.5 — is true iff there exists a path `q` in `B` such that:

1. `H(q)` is **false** (evaluated with the same `C-T1`/`C-T2` from `base`, §2.1) — this is what "non-test file" means; and
2. `lang(q)` = `lang(m)` and `lang(q) ∈ langs`; and
3. some import site in `imports(q, B)` has disposition `repo(m)` — resolved **against `B`**, not against `A`.

This is a reverse-import query over the whole base tree, and it is the single most expensive operation in the design. The playbook never says so and a reader implementing PB §4.3 literally — as a forward walk from the tests — cannot evaluate it at all (§17, D2).

Four rules make it total:

- **No cross-language edges.** The import graph is the disjoint union of four per-language graphs. A Python file never imports a `.ts` file. A path `m` whose language differs from every candidate importer's has no importer, whatever the two files say.
- **A file whose language is not in `langs` contributes no edges**, in `B` or in `A`. It can still be frozen by clause 3 or clause 4, and it can still be a *target* of an edge only if a same-language file resolves to it, which by definition it cannot be. The consequence is a real hole and it is named in §10: an oracle written in a language absent from `langs` is invisible to the closure.
- **An unresolvable specifier in a base-tree file yields no edge** and no finding. Findings are an approval-time concept about the branch's own harness; the base tree is trunk's, already landed, and re-litigating it at every approval would make every approval tripwire on somebody else's code.
- **`m` not in `B`** makes the predicate vacuously false, but row 3 of §2.5 has already fired, so the predicate is never consulted.

### 2.10 Output, size, and what this document does not order

The closure is a **set** of repository paths. It has no order.

`Spine-Frozen` lines carry `<oid> <path>` with `git ls-tree` quoting (PB §11) and the approve line's `freeze=` is *"a SHA-256 over the sorted `Spine-Frozen` and `Spine-Test` lines"* (PB §4.3). **That sentence does not fix a digest**: it does not say whether "line" means the rendered trailer or the payload, whether the sort key is the whole line (which begins with an object id, making the order effectively oid order) or the path, whether the collation is byte order, and whether the two groups are interleaved or concatenated. Two conforming implementations therefore compute different `freeze=` values over an identical approval, G9's freeze recomputation fails, and every landing is `unattested`. It is filed as D5 in §17 and **this document publishes no `freeze=` vector**, because a `freeze=` computed under a guess would be exactly the "probably right" test vector the directory exists to refuse.

G8's own recomputation is unaffected: PB §4.3 and PB §6.3 make it set containment — *"the closure recomputed by the pinned release ⊆ `Spine-Frozen`"* — which needs no order. An approval may freeze **more** than the closure computes (a human who moved a module under `C-T1` after a tripwire, and re-approved, does exactly that); it may never freeze less.

**Size.** `|closure|` is the count of distinct repository paths. PB §4.3 makes *"a closure over 200 files"* an approval tripwire: the finding `closure-too-large` fires when `|closure| > 200`, strictly. §18, OPEN-6 records why this threshold interacts badly with Swift.

**The closure digest, a conformance aid.** So that an implementer can check a closure without a `freeze=` rule existing, this document defines a digest that is **not** a protocol artifact, is written into no trailer, is read by no gate, and is used only by §14's conformance cases:

> `closure_digest` = SHA-256 over the RFC 8785 JCS serialization (`gate-report.md` §2.1, under §2.2's profile) of the JSON **array** of the closure's paths, each `esc`-encoded (`gate-report.md` §2.3), sorted ascending by encoded bytes.

Under §2.2's profile that array serializes as `["a","b",…]` with no whitespace. The four vectors in §13 are published with theirs, computed.

### 2.11 Findings: tripwires and counters

Every abnormality the resolver can produce is in this closed list. A **tripwire** routes the intent to `approval-review` and `--approve` refuses without a human `reason=` (PB §4.3, PB §6.3). A **counter** is reported and counted by `spine stats` and blocks nothing.

| Finding | Kind | Fires when |
|---|---|---|
| `unresolvable-import` | tripwire | a site with disposition `unresolvable` occurs in a file `f` with `H(f)` true. PB §4.3: *"an unresolvable or dynamic import inside test roots"*. Carries `(path, line, specifier-as-written or `<dynamic>`)`. |
| `unresolvable-import-outside-harness` | counter | the same, in a file with `H(f)` false that the walk reached. No edge, no block. |
| `lang-unclassifiable` | tripwire | a language's resolution configuration is unclassifiable (§3.3) **and** some seed or some `H`-true file in the walk is in that language. Carries the language token and the reason from that language's closed list (§4–§7). Fires **once per language**, not once per file. |
| `lang-unclassifiable-outside-harness` | counter | the same, when no `H`-true file is in that language. Every file of that language is excluded from the closure. |
| `closure-tripwire` | tripwire | row 3 of §2.5 fired at least once. Carries the sorted list of excluded branch-created paths. |
| `closure-too-large` | tripwire | `|closure| > 200`. |
| `expected-hits-harness` | tripwire | some entry of `expected` matches any `C-T1`/`C-T2` pattern. PB §4.3: *"frozen paths are exempt from G2, so declaring them is a request to change the harness mid-flight"*. Owned here because §2.4 owns the matching. |
| `seed-outside-test-roots` | refusal | a path in `A` carries a §12.1 pragma naming an AC in `AC` and **no `C-T1` pattern matches it** (§2.1.1) — including a path matched only by `C-T2`. Carries the sorted list of such paths. Not a tripwire: `--approve` refuses outright, because the constitution does not describe the repository. |
| `no-seed` | tripwire | `S` is empty (§2.1.1). The closure is then empty, `Spine-Frozen` names nothing, and G8's containment check is `∅ ⊆ ∅`. §16.12. |
| `file-not-utf8` | tripwire / counter | a file the walk must lex is not valid UTF-8 (§3.4). Tripwire if `H` holds, counter otherwise; the file contributes no edges either way. |

`spine stats` counters, named so two implementations report the same things: `closure_size`, `closure_tripwires`, `closure_size_tripwires`, `unresolvable_imports`, `dynamic_imports`, `unclassifiable_languages`, `excluded_branch_created`, `frozen_leaves_in_expected`, `seedless_approvals`. `frozen_leaves_in_expected` is PB §4.3's own request — *"`spine stats` counts them, and a rate that does not fall means the harness is entangled with the code under test."*

**This closed list is the resolver's, not `spine stats`' whole set**, and the distinction matters after 2026-08-26. The owner settled that an unbounded `forbidden` set stays legal and that `spine stats` gains a counter for **landings whose only protected wire is a G7 hard lease** (PB §5.4). That counter is a landing-level predicate over a gate report's wire set (`gate-report.md` §6.1) and touches nothing here: no resolver finding produces a `G7` wire, the resolver never reads a lease, and every counter above is per-approval rather than per-landing. It is named here only so that a reader who takes §2.11 as the complete inventory of `spine stats` does not conclude the new counter is missing from a list it was never in.

### 2.12 Determinism rules

1. **Two trees, no third.** Every existence test, every read and every pattern match names `A` or `B` explicitly. A rule that says only "the tree" is a defect in this document.
2. **No filesystem.** The resolver reads git tree entries, not a checkout. It never calls `stat`, never follows a symlink, never asks whether a path is executable, and never observes case-insensitivity. A tree entry with mode `120000` (symlink) is **not** a resolution target: a specifier that names one is `unresolvable`. A tree entry with mode `160000` (submodule) is not a directory and not a file: a specifier that names or descends through one is `unresolvable`.
3. **Order-free.** The walk's queue order does not affect the result: `class` is a pure function of the path, `seen` is keyed by path, and the output is a set. An implementation may walk depth-first, breadth-first, or in parallel.
4. **No I/O outside the object store.** No network, no package registry, no `node_modules`, no `.dart_tool`, no `.build`, no interpreter, no `sys.path`, no `PYTHONPATH`, no `NODE_PATH`.
5. **No time.** Nothing in the closure derives from a clock. (PB's one-clock rule; `dump.md` §10 makes the same commitment.)
6. **First-match-wins is always ordered explicitly.** Wherever a candidate list exists (§5.2's extension order, §4.3's root order), the order is written down and exhaustive, so resolution is single-valued by construction rather than by tie-breaking.
7. **Duplicate detection is by full path.** Two tree entries cannot share a path, so no deduplication rule is needed beyond set semantics.

---

## 3. The resolver contract

### 3.1 `lang(path)`

Total, byte-exact on the final path component, lowercase only. `.PY` is not Python; a repository that ships uppercase extensions gets `none`, and `spine stats` counts nothing about it, because the alternative is a case-folding rule that differs between an approving macOS laptop and a Linux CI container.

| Final component ends with | `lang` | `params.langs` token |
|---|---|---|
| `.py` | Python | `python` |
| `.ts` `.tsx` `.mts` `.cts` `.js` `.jsx` `.mjs` `.cjs` | TypeScript/JavaScript | `ts` |
| `.dart` | Dart | `dart` |
| `.swift` | Swift | `swift` |
| anything else | `none` | — |

**The four `params.langs` tokens are ratified as exactly `python`, `ts`, `dart` and `swift`.** They are the values PB §6.7's manifest example uses for the two it names, and they are permanent: `params.langs` is a floor-relevant manifest field (PB §6.3, G16), so a token cannot be corrected later without a floor-protected landing. `ts` covers JavaScript; PB §6.7 counts "TypeScript/JavaScript" as one language and there is one resolver for both. **`kotlin` is reserved and unusable** (§11.1): a `params.langs` naming it is refused by `result-file.md` §7.1 step 3 as a language the release supports no adapter for, and Appendix A says why it was dropped.

Two overrides, each stated because each is a divergence class:

- **`.d.ts`, `.d.mts`, `.d.cts` are type-only by construction.** They are TypeScript by extension, they are lexed like any other TypeScript file, but every import site in them is `type_only` and they are never a resolution target for a value import (§5.2 skips them in candidate expansion). A declaration file contains no runtime code, so nothing in it can weaken an oracle.
- **A path whose `lang` is not in `langs` contributes no edges**, in either tree (§2.9). It may still be frozen by clause 3 or clause 4.

### 3.2 The import site and its disposition

An **import site** is one syntactic occurrence, in one file, of one of the forms §4–§7 enumerate for that file's language. Sites are identified by `(path, byte offset of the first token)`, which makes them countable and reportable and gives findings a stable location.

`disposition(i)` is exactly one of:

| Disposition | Meaning | Edge | Finding |
|---|---|---|---|
| `repo(m)` | resolves to exactly one path `m` present in the tree being resolved against | yes | none |
| `external` | resolves outside the repository: a package, a stdlib or SDK module, a framework, generated code that is not in the tree | no | none |
| `type_only` | a recognized type-only form (§3.6) | no | none |
| `unresolvable` | recognized as an import site, but the target cannot be determined | no | `unresolvable-import` (§2.11) |

**One site yields at most one disposition, but a site may yield several `repo` targets.** Three forms genuinely name more than one file: a Python dotted import executes every ancestor package `__init__.py` as well as the module (§4.3); a Dart conditional import names one URI per branch (§6.2); a Swift `import M` names every source file of `M` (§7.4). Where that happens, the site's disposition is `repo({m₁ … mₖ})` and each `mᵢ` is classified independently. It is never a reason to call the site `unresolvable`.

**`external` is the safe default for a bare name that matches nothing.** This is worth stating because the instinct runs the other way. An oracle must live in the repository to be an oracle; if it lives in the repository it is a tree entry; if it is a tree entry the language's resolution rule finds it. A bare name that resolves to no tree entry therefore cannot be hiding an oracle — it is a dependency, an SDK module, or generated code that does not exist in the tree. Calling it `unresolvable` instead would make every Swift `import Foundation` and `import XCTest` a tripwire, which would mean every Swift approval routes to a human, which would mean the tripwire carries no information. §16.7 records this.

**`unresolvable` is reserved for the cases where the target *is* in the repository and the resolver cannot say which:** a specifier that is not a simple string literal, a relative specifier that names nothing, a path that escapes the repository root, a symlink or submodule entry, a `part of <library-name>` with zero or several candidates, and each language's closed list in §4–§7.

### 3.3 Resolution configuration `RC`, and the base-tree rule

Three of the four languages cannot resolve a specifier without reading something out of the repository: TypeScript needs `tsconfig.json`'s `paths`, Dart needs `pubspec.yaml`'s `name`, and Swift needs `Package.swift`'s targets. Call the extracted value the language's **resolution configuration**, `RC(lang, tree)`. §5.3, §6.3 and §7.3 define the extraction, each as a *declarative subset* with a closed refusal list. Python's `RC` is empty (§4.2).

Two rules govern it, and neither is in the playbook.

**Rule 1 — the closure is computed with `RC(lang, B)`.** The configuration comes from the base tree, exactly as PB §4.3's clause-2 test does, and for the same stated reason: *"It is read from the base tree, which the branch cannot edit."* Without this, a candidate reshapes `Package.swift` so that its oracle falls outside every target the resolver knows about, and the closure it computes at `--approve` is the same closure G8 recomputes in CI — both wrong, and agreeing.

**Rule 2 — if `RC(lang, A) ≠ RC(lang, B)`, the language is unclassifiable for this approval.** The comparison is on the *extracted configuration*, not on the file's blob, so adding a dependency to `pubspec.yaml` or a script to `package.json` changes nothing; adding a target, a project, a source-set override or a path alias raises `lang-unclassifiable` with reason `rc-changed-on-branch`, `--approve` refuses without a human `reason=`, and `spine stats` counts it. This is not a refusal of the work — it is the human confirming that a change to how the closure is computed is intended, in the one place where the branch would otherwise be grading its own homework.

`RC` is a value, and "differs" means differs as a value: §5.3, §6.3 and §7.3 each give `RC`'s shape, so equality is structural and does not depend on how a manifest was formatted.

**Rule 2 has one consequence worth stating plainly:** the intent that *introduces* a language's harness will always trip it, because `RC(lang, B)` is empty and `RC(lang, A)` is not. That is correct. The first approval that gates a new language is exactly the one a human should read.

### 3.4 Lexical preliminaries, shared by all four

The resolver does not parse. It **lexes**, and matches token patterns. That is a deliberate level: a full grammar for four languages is not writable in one document and would not be implemented identically twice, while the lexical rules below are small, closed, and sufficient — because in all four languages the import forms are anchored on a reserved word and terminated by a string literal or a dotted name.

1. **Decoding.** A file is decoded as UTF-8. A file that is not valid UTF-8 is not lexed: it contributes no edges and raises `file-not-utf8` (§2.11). No encoding declaration is honoured — not PEP 263's coding cookie, not a BOM, not an XML declaration. A leading UTF-8 BOM (`EF BB BF`) is skipped and is not part of the first token.
2. **Line terminators.** LF, CRLF and CR each terminate a line. Line numbers in findings are 1-based and count terminators.
3. **Token kinds.** `word` (a maximal run of `[A-Za-z0-9_$]`, plus `.` never — `.` is punctuation), `string`, `punct` (any other single byte), `comment` (discarded before matching), `newline` (produced for Python, where a line break is syntactically significant; discarded elsewhere).
4. **Comments** are per language (§4.1, §5.1, §6.1, §7.1) and are discarded **after** the pragma scan of §12, which reads them.
5. **String literals.** Per language. A literal is **simple** iff it is a single literal token, contains no interpolation, and contains no backslash. A specifier that is not a simple literal is `unresolvable` — including adjacent-literal concatenation (`'pack' 'age:x'` in Dart and Python), template literals with no substitution (`` `./x` `` in TypeScript), and any literal containing an escape. This is over-strict by design: a specifier with an escape in it is either exotic or evasive, and the cost of refusing it is one tripwire.
6. **`import`, `export`, `from`, `part` and `package` are reserved words in all four languages** in the positions this document uses them, so recognition needs no statement-boundary tracking beyond the per-language anchors of §4–§7. Each language's section states its anchor explicitly.
7. **Nesting is irrelevant.** An import inside a function, a class, a `try`, an `if`, a `#if` branch or a lazily-loaded block is an import site. There is no "top-level only" rule anywhere in this document: a conditionally-executed import is a real dependency, and a rule that ignored one would be a rule an oracle could hide behind.

### 3.5 Re-exports

PB §4.3: *"re-exports count as imports"*. A **re-export** is any form that both names another module and republishes some or all of its bindings under the current module's name. It is an import site like any other, with the same dispositions and the same classification. Per language:

| Language | Re-export forms | Note |
|---|---|---|
| Python | none distinct | `from .a import b` is already an import; a re-export is that statement plus `__all__`, which the resolver never reads |
| TypeScript/JavaScript | `export * from 's'`, `export * as ns from 's'`, `export { a } from 's'`, `export { default as d } from 's'`, `export { a as default } from 's'` | `export type { … } from 's'` and `export { type A } from 's'` are `type_only` (§3.6) |
| Dart | `export 'uri';`, with `show`/`hide` | `part 'uri';` is stronger than a re-export and is also an edge (§6.2) |
| Swift | `@_exported import M` | resolves exactly as `import M` |

A `FROZEN_LEAF` re-exporting module is **not** walked, so the modules it republishes are not reached through it. That is the leaf rule doing its job, and §13.2 shows it happening.

### 3.6 Type-only imports, and why they do not count

PB §4.3: *"type-only imports do not count"*.

**The reason, stated once.** A type-only import is erased before the test runs. Nothing it names is present in the running process, so changing the imported module cannot change what any assertion observes. Freezing it would achieve nothing against an oracle and would cost a great deal: the declaration files and type modules a test imports are precisely the surface an implementation is *supposed* to change, so every signature change during implementation would become a G8 failure with no exit but a reopen.

**Only TypeScript has one.** The forms are closed:

- `import type X from 's'`, `import type { A } from 's'`, `import type * as ns from 's'` — the whole site is `type_only`.
- `export type { A } from 's'`, `export type * from 's'` — `type_only`.
- `import { type A, type B } from 's'` — `type_only` **only if every** named specifier carries the inline `type` modifier. `import { type A, b } from 's'` has a value binding and is a normal import site.
- `/// <reference path="…" />` and `/// <reference types="…" />` — `type_only`. (Recognized as a comment form, §5.1.)
- A site whose resolved target is a `.d.ts`/`.d.mts`/`.d.cts` file — `type_only`, by §3.1.

**Python, Dart and Swift have none, and Python's near-miss is the one to be explicit about.** An `import` nested under `if TYPE_CHECKING:` is erased in effect, and it is tempting to recognize it. This document does not, for a reason that is about determinism and not about taste: recognizing it requires deciding that `TYPE_CHECKING` is `typing.TYPE_CHECKING` and not a module-level `TYPE_CHECKING = True`, which is a name-binding question the resolver refuses to answer (§1). So **a Python import under `if TYPE_CHECKING:` is an ordinary import site.** The cost is over-freezing a module a test only type-references; that module is usually inside `expected` and therefore excluded anyway. §16.5 records it, and §17 D6 asks the playbook to say "where the language has one" rather than stating a general rule that is true of one language in four.

### 3.7 Conditional constructs: the union rule

Every one of the four languages has at least one construct whose active branch is chosen by something outside the tree — Swift's `#if`, Dart's `import … if (…)`, TypeScript's environment-dependent `exports` conditions, Python's `try: import … except ImportError:`.

**The rule is the union.** Every branch of every conditional construct contributes its import sites, and all of them are resolved. No configuration, flag set, target platform, build variant, Swift compilation condition or Dart environment declaration is ever read.

This is not a convenience. It is forced by two of the three constraints in §1 at once. A compilation configuration is not in the tree — `os(macOS)`, `arch(arm64)`, `dart.library.io` and `debug` are properties of the machine performing the build — so resolving under one would make the closure differ between the laptop that approved and the container that recomputes, which is precisely the disagreement that rejects an approval. And the union is the *unique* environment-independent over-approximation: any other answer either reads the environment or drops branches, and dropping a branch is how an oracle hides.

The cost is over-freezing: a file imported only under `#if os(Windows)` is frozen and the branch may not edit it. That is the safe direction, and it is small.

### 3.8 The unclassifiable ladder

Three different things get called "unclassifiable" and they are not the same, so they are named separately and used consistently:

| Level | Trigger | Effect |
|---|---|---|
| **site** | one import site whose target cannot be determined | disposition `unresolvable`; no edge; `unresolvable-import` if in an `H`-true file |
| **file** | a file that cannot be lexed (not UTF-8) | no edges from that file; `file-not-utf8` |
| **language** | `RC(lang, ·)` outside the declarative subset, or `RC(lang, A) ≠ RC(lang, B)` | **every** file of that language contributes no edges and is never added by an import edge; `lang-unclassifiable`; clause 3 and clause 4 still apply |

PB §4.3's sentence — *"a module whose imports cannot be resolved statically is unclassifiable and stays excluded, counted by `spine stats`"* — is read as the **site** level, because "stays excluded" is only meaningful about a path the resolver can name, and a site whose target is unknown has no path to exclude. §16.3 records the reading and §17 D7 files the sentence's collision with the tripwire that covers the same event.

---

## 4. Python

### 4.1 Lexing

- **Comments:** `#` to end of line. There is no block comment. A `#` inside a string literal is not a comment.
- **String literals:** `'…'`, `"…"`, `'''…'''`, `"""…"""`, each optionally prefixed by any case-insensitive combination of `r`, `b`, `f`, `u`, `rb`, `br`. A literal is **simple** (§3.4 rule 5) only when its prefix is empty or `r`/`u`, it is not an f-string, and it contains no backslash.
- **Logical lines:** a logical line ends at a newline that is not inside `(`/`[`/`{` and is not preceded by a backslash continuation. `;` separates statements inside a logical line.
- **Anchor:** an import site begins at a `word` token `import` or `from` that is the first token of a logical line or the first token after a `;`. Both are keywords, so no further disambiguation is needed.

### 4.2 `RC(python, ·)`

**Empty.** Python's resolution roots are fixed by rule (§4.3) and read no file. `pyproject.toml` is *not* consulted: its `[tool.setuptools] package-dir`, `[tool.poetry] packages` and `[tool.hatch.build] sources` keys would each be a different spelling of the same fact, they are candidate-controlled, and reading them would put a TOML parser between two implementations that must agree to the byte. The residual is named in §10.

Python therefore never raises `lang-unclassifiable`.

### 4.3 Specifier resolution

**Roots**, in this order, evaluated against the tree being resolved against:

1. `""` — the repository root.
2. `src/` — if and only if a tree entry `src` exists and is a directory.

That is the whole list. It covers the flat layout and the src layout, which is every layout a repository gated by spine can have, because pytest must be able to import the package from the repository root without an installed distribution (`spine init` pins the runner's roots to `C-T1`, `result-file.md` §6.7).

**A dotted name `n₁.n₂.….n_k`** resolves as follows. For each root `r` in order, form the candidates:

- `r + n₁/…/n_k + ".py"`
- `r + n₁/…/n_k + "/__init__.py"`

The first root for which at least one candidate exists wins. If **both** candidates exist under the winning root, the site is `unresolvable` (reason `ambiguous-module`): a repository with both `a/b.py` and `a/b/__init__.py` is broken, and guessing which one the interpreter picks would be reading `sys.path` semantics the resolver has refused.

**Ancestor packages are part of the edge.** Importing `a.b.c` executes `a/__init__.py` and `a/b/__init__.py` before `a/b/c.py`. So the site's targets are the resolved module **plus**, under the same winning root, every existing `n₁/…/n_j/__init__.py` for `1 ≤ j < k`. A missing intermediate `__init__.py` is a namespace package (PEP 420) and is simply not a target — it is not an error.

**Forms:**

| Form | Targets |
|---|---|
| `import a.b.c` / `import a.b.c as d` | dotted resolution of `a.b.c` |
| `import a.b, c.d` | dotted resolution of each, as separate sites |
| `from a.b import c` | first, dotted resolution of `a.b.c` (`c` may be a submodule); if that yields nothing, dotted resolution of `a.b` alone (`c` is an attribute). If neither resolves, `external` |
| `from a.b import c, d` | the union of the `from` rule applied to each name; one site |
| `from a.b import *` | dotted resolution of `a.b` |
| `from . import c` | package-relative, level 1 |
| `from .a.b import c` | package-relative, level 1, then dotted `a.b.c` / `a.b` |
| `from ..a import b` | package-relative, level 2 |
| `from a import (b, c)` (parenthesized, possibly multi-line) | as `from a import b, c`; the logical-line rule of §4.1 makes it one site |

**Package-relative resolution.** Let `d` be the directory containing the importing file. For level `L`, the base directory is `d` with `L − 1` components removed. Note that this is the same for `p/q/mod.py` and `p/q/__init__.py`: Python gives both the package `p.q`, whose directory is `p/q`. If the base directory would escape the repository root, the site is `unresolvable` (reason `relative-escapes-root`). Otherwise resolve the remaining dotted name against that directory as a single root, and add the existing `__init__.py` of the base directory and of every intermediate directory as targets.

**Dynamic imports.** A file containing any of the token sequences `__import__`, `importlib . import_module`, `importlib . __import__`, `importlib . util . spec_from_file_location`, or `imp . load_source` has, at each occurrence, an import site with disposition `unresolvable` (reason `dynamic-import`). The argument is not inspected even when it is a simple literal: deciding that `importlib` is the standard library and not a local shim is a name-binding question (§1).

### 4.4 `AncestorConfig(python)` — clause 3 basenames

`__init__.py`, `conftest.py`, `pytest.ini`, `pyproject.toml`, `tox.ini`, `setup.cfg`.

`conftest.py` is the one that carries the hazard PB §4.3's clause 3 describes, and it is the one PB does not name (§17, D1). pytest auto-loads every `conftest.py` from the rootdir down to each test file, without any import statement, and a `pytest_collection_modifyitems` hook in one can deselect every test below it.

### 4.5 Scaffolded `C-T2` patterns

`spine init` renders these into `C-T2` when `python ∈ langs`:

```
tests/support/**
**/conftest.py
pytest.ini
pyproject.toml
tox.ini
setup.cfg
```

`C-T1`'s Python default is `tests/`.

### 4.6 Snapshot patterns (clause 4)

Final component matching `*.ambr` (syrupy), `*.approved.txt`, `*.golden`, `*.snap`; or any path with a directory component named `__snapshots__` or `snapshots`.

### 4.7 The closed unclassifiable list

Python has no language-level unclassifiable state. Site-level `unresolvable` arises in exactly these cases and no others:

1. a dynamic-import construct (§4.3);
2. a relative import whose level escapes the repository root;
3. a dotted name resolving to both `x.py` and `x/__init__.py` under the winning root;
4. a resolved candidate whose tree entry is a symlink or submodule (§2.12 rule 2);
5. the file is not valid UTF-8 (file level).

Everything else is `repo(…)` or `external`.

---

## 5. TypeScript / JavaScript

### 5.1 Lexing

- **Comments:** `//` to end of line, `/* … */` (not nested). A `///`-prefixed line comment whose remainder matches `<reference …/>` is a triple-slash directive and is a `type_only` import site (§3.6); it is otherwise a comment.
- **String literals:** `'…'`, `"…"`, and template literals `` `…` ``. A template literal containing `${` is not simple. Regular-expression literals are lexed as `punct` runs and never as strings; a `/` that follows a `word`, `)`, `]` or a numeric literal is division, otherwise it opens a regex. (This rule exists only so that a `//` inside a regex is not read as a comment; no import form can occur inside one.)
- **Anchor:** `import` and `export` are reserved words in module code. A `word` token `import` **not immediately preceded by** a `.` token is an import site — a *dynamic* one if the next token is `(`, `import.meta` (not a site) if the next token is `.`, and a declaration otherwise. A `word` token `export` not preceded by `.` begins a re-export site iff a `from` word token followed by a simple string literal occurs before the next `;` or `}` at the same bracket depth. A `word` token `require` not preceded by `.` and immediately followed by `(` is a CommonJS import site.

### 5.2 Specifier resolution

Given the importing file `f` and a specifier `s`:

1. `s` begins `./`, `../`, or is exactly `.` or `..` → **relative**. Base path `Bp` = the lexical normalization of `dirname(f) + "/" + s`, collapsing `.` and `..` textually. If it escapes the repository root → `unresolvable`.
2. `s` begins `/` → `unresolvable` (an absolute filesystem path is environment-dependent).
3. `s` begins `#` → `unresolvable` (a `package.json` `imports` subpath; v1 reads no `exports`/`imports` map, §18 OPEN-7).
4. otherwise → **bare**. Consult the alias table (§5.3). If some alias matches, each of its substituted candidate base paths goes to step 5 in the table's order and the **first** that resolves wins; if an alias matched and none resolves → `unresolvable` (reason `alias-dead-end`). If no alias matches → `external`.

**Step 5 — candidate expansion for a base path `Bp`,** first match wins over the whole ordered list:

1. `Bp` itself, if it is an existing file entry.
2. **The TypeScript output-extension rewrite**, when `Bp` ends in a JavaScript extension: `.js` → `.ts`, `.tsx`; `.mjs` → `.mts`; `.cjs` → `.cts`. (`import './x.js'` in a TypeScript file names `x.ts`.)
3. `Bp + ext` for `ext` in this exact order: `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs`, `.cjs`, `.json`.
4. If `Bp` is an existing directory entry: `Bp + "/index" + ext` for `ext` in the same order as step 3.
5. otherwise → `unresolvable` (reason `no-candidate`).

A candidate that is a `.d.ts`/`.d.mts`/`.d.cts` file is **skipped** in steps 1–4 rather than matched: a declaration file is never the target of a value import (§3.1). If every candidate is a declaration file, the site is `type_only`.

The list is exhaustive and ordered, so no ambiguity rule is needed: a directory containing both `x.ts` and `x.js` resolves to `x.ts`, which is what TypeScript does, and the resolver does not need to know that to be deterministic.

**Directory resolution reads no `package.json`.** A directory containing a `package.json` with a `main`, `module`, `exports` or `types` field resolves by `index` alone. Workspace packages inside a monorepo are reached through the alias table or are `external`. §18, OPEN-7.

**Forms:**

| Form | Site |
|---|---|
| `import d from 's'`, `import { a } from 's'`, `import * as n from 's'`, `import d, { a } from 's'` | value import |
| `import 's'` | side-effect import — a real edge; a setup file is imported this way |
| `import x = require('s')` | value import (TypeScript import-equals) |
| `export * from 's'`, `export * as n from 's'`, `export { a } from 's'`, `export { default as d } from 's'` | re-export (§3.5) |
| `import('s')` where `s` is a simple literal | value import |
| `import(expr)` | `unresolvable`, reason `dynamic-import` |
| `require('s')` where `s` is a simple literal | value import |
| `require(expr)` | `unresolvable`, reason `dynamic-import` |
| `import type …`, `export type …`, all-inline-`type` named specifiers, `/// <reference …>` | `type_only` (§3.6) |

`require.resolve('s')` is **not** an import site: it returns a path and executes nothing.

**A literal `import('./x')` resolves rather than tripwiring**, even though TypeScript calls it a "dynamic import" and PB §4.3's tripwire says "unresolvable or dynamic". "Dynamic" is read as *the specifier is not statically determined*, not as *the syntax is named dynamic import* — a literal one is exactly as determined as a static one, and tripwiring idiomatic lazy loading would put a human in front of every approval that splits a bundle. §16.8.

### 5.3 `RC(ts, tree)`

Extracted from the repository-root `tsconfig.json`, or `jsconfig.json` if no `tsconfig.json` exists at the root. `RC` is the pair `(baseUrl, paths)` where `paths` is a list of `(pattern, [substitution, …])` in the file's own key order.

Extraction:

1. The file is parsed as **JSON with comments and trailing commas** (the dialect `tsc` accepts). A file that does not parse in that dialect → `RC` unclassifiable, reason `tsconfig-unparseable`.
2. `extends` is followed only for a value that is a simple string beginning `./` or `../`, resolved against the extending file's directory, with the extension `.json` appended if absent. Child keys override parent keys. An `extends` naming a bare specifier, an absolute path, or an array → `RC` unclassifiable, reason `tsconfig-extends-external`. A cycle → unclassifiable, reason `tsconfig-extends-cycle`.
3. `compilerOptions.baseUrl`, if present, must be a simple string; it is resolved relative to the file that declares it and must stay inside the repository, else unclassifiable, reason `baseurl-escapes-root`.
4. `compilerOptions.paths` must be an object whose every value is an array of strings, each containing at most one `*`, and whose every key contains at most one `*`, else unclassifiable, reason `paths-malformed`.
5. No other key is read. `include`, `exclude`, `files`, `references` and `moduleResolution` are ignored: v1 has one alias table for the repository, not one per project (§18, OPEN-7).

**Alias matching.** For a bare specifier `s`, a key `k` matches if either `k` has no `*` and `k == s`, or `k` is `p*q` and `s` begins with `p` and ends with `q` and `|s| ≥ |p| + |q|`; the capture is the middle. Where several keys match, the one with the **longest literal prefix before its `*`** wins; ties are impossible because two distinct keys cannot have equal literal prefixes and equal suffixes. Each substitution has its `*` replaced by the capture and is resolved relative to `baseUrl` (or, absent `baseUrl`, to the directory of the `tsconfig.json` that declared `paths`).

If no `tsconfig.json` and no `jsconfig.json` exists at the repository root, `RC` is `(none, [])` — legal, not unclassifiable. A repository with no alias table simply has no bare specifier that resolves inside it.

### 5.4 `AncestorConfig(ts)` — clause 3 basenames

`package.json`, `tsconfig.json`, `jsconfig.json`, `vitest.config.ts`, `vitest.config.mts`, `vitest.config.js`, `vitest.config.mjs`, `vitest.workspace.ts`, `vitest.workspace.js`, `vitest.setup.ts`, `vitest.setup.js`, `jest.config.ts`, `jest.config.js`, `jest.config.mjs`, `jest.config.cjs`, `jest.setup.ts`, `jest.setup.js`.

`package.json` is on the list for two reasons at once: it can carry a `"vitest"` or `"jest"` key that reconfigures collection, and its `"type"` field decides whether every `.js` file in the tree is ESM or CJS. §18, OPEN-5 records what freezing it costs.

Config files that are themselves TypeScript are walked (they satisfy `H`), so `vitest.config.ts`'s own `import './vitest.setup.ts'` pulls the setup file into the closure even where the basename list would not have. §13.2 shows this.

### 5.5 Scaffolded `C-T2` patterns

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

`C-T1`'s default is `tests/`, `src/**/__tests__/`.

**`vite.config.*` is on this list because §12.4.2 makes it a `C-T3` hook basename**, and the two lists have to agree. Vitest loads a root `vite.config.ts` as its configuration when no `vitest.config.*` sits beside it, which is the ordinary Vite-plus-Vitest layout; §12.4.2 therefore lists `vite.config.` among the TypeScript hook basenames, and a basename is a hit *wherever `H` is false*. Omitting it from the scaffolded `C-T2` left every Vite repository carrying a permanent `class=protected` `G8:vite.config.ts` finding on **every** landing, over a file `spine init` itself expects to be there — the same shape §17 D12 closes for the predicate's domain, one list further along. Its presence here also puts the file in `H`, so it is walked into the freeze closure and is read-only from the branch after approval, which is the treatment the runner's own configuration should have had from the start.

**It does not displace `vitest.config.*`.** Both are on the list, a repository may hold either or both, and neither is required to exist; a pattern that matches nothing costs nothing (§2.4).

### 5.6 Snapshot patterns (clause 4)

Final component matching `*.snap`; or any path with a directory component named `__snapshots__`.

### 5.7 The closed unclassifiable list

Language level (`lang-unclassifiable`, reason as named): `tsconfig-unparseable`, `tsconfig-extends-external`, `tsconfig-extends-cycle`, `baseurl-escapes-root`, `paths-malformed`, `rc-changed-on-branch` (§3.3).

Site level (`unresolvable`): `dynamic-import` (non-literal `import(…)` or `require(…)`), `absolute-specifier`, `subpath-imports` (`#…`), `relative-escapes-root`, `no-candidate`, `alias-dead-end`, `symlink-or-submodule`.

---

## 6. Dart

### 6.1 Lexing

- **Comments:** `//` to end of line, `/* … */` (**nested**, unlike C — Dart's block comments nest, and a lexer that does not nest them mis-lexes a commented-out block containing `*/`).
- **String literals:** `'…'`, `"…"`, `'''…'''`, `"""…"""`, each optionally prefixed `r`. A literal containing `$` followed by `{` or an identifier character is interpolated and not simple; a raw (`r`) literal with no interpolation and no backslash is simple.
- **Anchor:** the pattern itself is the anchor. `import`, `export` and `part` are built-in identifiers whose directive forms are unambiguous by shape: a `word` token `import`, `export` or `part` **immediately followed by a `string` token**, or the sequence `part` `of`. This needs no statement-boundary tracking.

### 6.2 Specifier resolution

A Dart URI is resolved by scheme:

| URI shape | Resolution |
|---|---|
| `dart:…` | `external` |
| `package:<name>/<rest>` where `<name>` is `RC.selfName` | `lib/<rest>`, relative to the package root directory |
| `package:<name>/<rest>` where `<name>` is a key of `RC.pathDeps` | `<RC.pathDeps[name]>/lib/<rest>` |
| `package:<name>/…` otherwise | `external` |
| no scheme (a relative URI) | lexically normalized against the importing file's directory; escaping the repository root → `unresolvable` |
| any other scheme (`file:`, `http:`, `asset:`) | `unresolvable`, reason `unsupported-scheme` |

Dart requires the `.dart` extension in every URI, so there is no candidate expansion, no index resolution and no extension list. A resolved path that is not an existing file entry → `unresolvable`, reason `no-candidate`.

**Forms:**

| Form | Site |
|---|---|
| `import 'uri';` with optional `as`, `show`, `hide`, `deferred as` | value import |
| `export 'uri';` with optional `show`, `hide` | re-export (§3.5) |
| `import 'a' if (c) 'b' if (d) 'e';` | **one site, all URIs** (§3.7's union rule) |
| `export 'a' if (c) 'b';` | one site, all URIs |
| `part 'uri';` | value import — stronger than one: the part file *is* the library, so it is walked |
| `part of 'uri';` | value import naming the parent library file |
| `part of a.b.c;` (legacy library-name form) | see below |

**`part of <dotted name>`** is resolved through a library-name index built over the tree being resolved against: every Dart file whose directives contain `library <dotted name>;`. Exactly one match → that file. Zero or more than one → `unresolvable`, reason `ambiguous-library-name`.

**Dart has no dynamic import.** `deferred as` is a lazy *load* of a statically named URI and is an ordinary edge.

**Dart has no type-only import.** Every import is a runtime import; §3.6 says none is recognized.

### 6.3 `RC(dart, tree)`

Extracted from every `pubspec.yaml` in the tree. `RC` is a set of packages, each `(rootDir, name, pathDeps)`.

1. A `pubspec.yaml` at directory `d` declares a package rooted at `d`. Its `name:` must be a plain scalar matching `^[a-z_][a-z0-9_]*$`, else unclassifiable, reason `pubspec-name-malformed`.
2. The YAML is read as the **declarative subset**: block mappings, block sequences and plain or single/double-quoted scalars only. Anchors (`&`), aliases (`*`), merge keys (`<<`), tags (`!`), multi-document streams (`---` more than once) and flow mappings that nest more than one level → unclassifiable, reason `pubspec-not-declarative`.
3. `pathDeps` is built from `dependencies:` and `dev_dependencies:`: each entry of the form `<pkg>: { path: <p> }` contributes `<pkg> → normalize(d + "/" + p)`, provided the result stays inside the repository. A `path:` escaping the root, or a `git:`/`hosted:` dependency, contributes nothing and is not an error.
4. Two packages with the same `name` in one repository → unclassifiable, reason `duplicate-package-name`.
5. The importing file's package is the one whose `rootDir` is the **longest** prefix of the file's path. A Dart file under no package root → its `package:` self-references are `external` and its relative imports still resolve.

### 6.4 `AncestorConfig(dart)` — clause 3 basenames

`pubspec.yaml`, `dart_test.yaml`, `build.yaml`.

`analysis_options.yaml` is deliberately absent: it configures the analyzer, not the test runner, and cannot deselect a test.

### 6.5 Scaffolded `C-T2` patterns

```
test/support/**
pubspec.yaml
dart_test.yaml
build.yaml
```

`C-T1`'s default is `test/` (Dart's convention is singular, and `dart test` will not collect from `tests/` without configuration).

### 6.6 Snapshot patterns (clause 4)

Final component matching `*.golden`, `*.approved.txt`; or any path with a directory component named `goldens` or `__snapshots__`.

### 6.7 The closed unclassifiable list

Language level: `pubspec-name-malformed`, `pubspec-not-declarative`, `duplicate-package-name`, `rc-changed-on-branch`.

Site level: `unsupported-scheme`, `relative-escapes-root`, `no-candidate`, `ambiguous-library-name`, `non-simple-literal`, `symlink-or-submodule`.

---

## 7. Swift

Swift is the first of the two languages the brief flags, and the first thing to say is that one of the three flagged difficulties dissolves on inspection.

### 7.1 Lexing

- **Comments:** `//` to end of line, `/* … */` (**nested**).
- **String literals:** `"…"`, `"""…"""`, and extended delimiters `#"…"#`, `##"…"##`. Interpolation is `\(` (or `#\(` at matching delimiter depth). No import specifier is a string in Swift, so simplicity does not arise; the rules exist only to lex correctly.
- **Anchor:** a `word` token `import` not immediately preceded by `.`, optionally preceded by the attribute tokens `@testable` or `@_exported`.

### 7.2 The import forms, and `@testable`

| Form | Module named |
|---|---|
| `import Foo` | `Foo` |
| `import Foo.Bar` | `Foo` — a submodule path; the first component is the module |
| `import struct Foo.Baz` (and `class`, `enum`, `protocol`, `typealias`, `func`, `let`, `var`) | `Foo` |
| `@testable import Foo` | `Foo` |
| `@_exported import Foo` | `Foo` — a re-export (§3.5) |
| `#if …` / `#elseif` / `#else` around any of the above | every branch (§3.7) |

**`@testable import` needs no special handling at all.** It changes the *visibility* of what the imported module exports — `internal` becomes visible to the importer — and it changes nothing about which module is imported or which files that module contains. It resolves exactly as `import`. The brief lists it among the constructs that make "what does this file import" depend on build configuration; it does not. What it does depend on is that the target was built for testing, which is a property of the build, not of the import — and the resolver never asks.

### 7.3 `RC(swift, tree)`

Extracted from every `Package.swift` in the tree. `RC` is a set of packages, each `(rootDir, [target])`, and each target is `(name, kind, sourceDirs, sources, exclude, dependencies)`.

`Package.swift` is Swift source code, so it is read as a **literal manifest subset** and refused outside it. The subset:

1. The file contains exactly one top-level expression statement whose callee is `Package` — the initializer — assigned to `let package`. Any other top-level statement other than the `// swift-tools-version:` comment, `import PackageDescription`, and that one `let` → unclassifiable, reason `manifest-not-literal`.
2. Inside the initializer, the `targets:` argument is an **array literal** of call expressions whose callees are `.target`, `.testTarget`, `.executableTarget`, `.macro`, `.systemLibrary`, `.binaryTarget` or `.plugin`.
3. Every `name:`, `path:`, `sources:` and `exclude:` argument is a simple string literal or an array literal of simple string literals. Any identifier reference, string interpolation, `+`, ternary, `#if`, `for`, `map`, or function call in those positions → unclassifiable, reason `manifest-not-literal`.
4. `dependencies:` is read only for its simple string literals and `.target(name: "X")` / `.byName(name: "X")` forms; anything else contributes no dependency and is **not** an error, because target dependencies do not affect which files belong to a target.
5. Two targets with the same `name` anywhere in the repository → unclassifiable, reason `duplicate-target-name`.
6. A repository containing a `.xcodeproj` or `.xcworkspace` directory and no `Package.swift` → unclassifiable, reason `xcode-project-unsupported`.

**A target's source directory.** If `path:` is given, the directory is `rootDir + "/" + path`. Otherwise the first existing directory in this order:

- for `.testTarget`: `Tests/<name>`, `Sources/<name>`, `Source/<name>`, `src/<name>`, `srcs/<name>`;
- for every other kind: `Sources/<name>`, `Source/<name>`, `src/<name>`, `srcs/<name>`, `Tests/<name>`.

None existing → unclassifiable, reason `target-dir-missing`.

**A target's file set, and its source files.** The two are defined separately because the refusal below reads the first and the walk (§7.4) reads the second.

The **file set** `F(t)` is: if `sources:` is given, then for each entry — the entry itself when it names a blob in the tree, and every blob recursively beneath it when it names a directory; otherwise every blob recursively beneath the source directory, at every depth. Then remove from `F(t)` every path equal to, or beneath, any `exclude:` entry. `F(t)` is the **whole** set of remaining blobs and is filtered by no extension.

The target's **source files** are `{ p ∈ F(t) : lang(p) = Swift }` — §3.1's `.swift`, byte-exact and lowercase only — and they are what §7.4 draws edges over. A path in the source files of two targets → unclassifiable, reason `overlapping-targets`.

**A mixed target fails closed — `mixed-objc-target`.** PB §6.7 removed Kotlin because an oracle in a `.java` file inside a mixed Kotlin/Java module is invisible to a Kotlin resolver and *nothing reports the miss*: **a guarantee that fails loudly can ship with its limits stated, one that fails silently cannot.** A `.m`, a `.mm` or a bridging header inside a Swift target is the same failure and not a smaller one — Objective-C is `lang: none` (§3.1), it is the target of no edge, and a Swift file reaches it through a header with no `import` line for the resolver to find. So the rule that removed Kotlin is applied here rather than a second judgement being made about the same shape: Swift stays in v1 and the hole is made **loud**.

`RC(swift, tree)` is **unclassifiable, reason `mixed-objc-target`**, if any target `t` of any package in the tree satisfies either test below. Both are decided by path and by argument label; no file is opened for its content and no manifest value is evaluated.

**Test 1 — a C-family entry in the file set.** Some `p ∈ F(t)` whose final path component ends in one of the following, matched byte-exactly and lowercase only, exactly as §3.1's table is matched:

```
.m  .mm  .h  .hh  .hpp  .hxx  .pch  .c  .cc  .cpp  .cxx  .modulemap
```

The list is C-family rather than Objective-C alone, and the reason token is still `mixed-objc-target`, because Objective-C's interop surface is C's: a `.c` behind a `.h` is the same invisible oracle, and a list that stopped at `.m`/`.mm` would leave the identical silent hole this rule exists to close. **A bridging header needs no clause of its own** — every spelling of one, `<Target>-Bridging-Header.h` included, ends in `.h`, and no rule here reads a filename stem. `F(t)` is post-`exclude:`, so a `.m` under an `exclude:` entry does **not** trigger: it is not compiled into the target and no Swift file can reach it.

**Test 2 — a manifest construct.** `t`'s call carries any of the argument labels `publicHeadersPath:`, `cSettings:` or `cxxSettings:`; or it contains a simple string literal (§3.4 rule 5) equal to `-import-objc-header`; or `t`'s callee is `.systemLibrary`. **Presence alone triggers, whatever the value.** That is why this test needs no widening of the literal subset above: an argument rule 3 would refuse to read is never read here either, only observed.

**Both tests run in both trees.** `RC`'s tuple is manifest-derived, so a branch that drops `Sources/Billing/Oracle.m` into an existing target changes no extracted value and §3.3 Rule 2's comparison does not see it. Test 1 therefore runs against `A` as well as against `B`, and an entry in **either** tree raises `mixed-objc-target`. This is the one part of `RC(swift, ·)` that reads `A` for its own sake rather than to compare, and it is the half of the rule that matters: the branch is where an oracle arrives.

**Precedence.** `mixed-objc-target` is decided during extraction and therefore before §3.3 Rule 2's comparison. Where a branch introduces the first Objective-C entry, the reason is `mixed-objc-target` and never `rc-changed-on-branch`, so the finding names the hole rather than the diff.

**What it costs, at true size.** The refusal is repository-wide, like `duplicate-target-name` and `xcode-project-unsupported`, and it is not narrowed to targets that also contain Swift — a pure Objective-C target is the arrangement §10 once cited as SwiftPM's mitigating convention, and it is not one: `import CBits` where `CBits` holds no `.swift` file yields **zero** edges and no finding, which is the silent miss exactly. So a repository whose Swift package holds any C-family file becomes ungatable for Swift — not silently under-frozen, ungatable, with one `lang-unclassifiable` tripwire naming the target. The exits are the C-family sources leaving the repository, or a release that adds an Objective-C lexer (§10). Refusing is the direction PB §6.7's rule requires; §18 OPEN-2 records the decision.

### 7.4 What a Swift file imports

Swift's compilation unit is the **module**, and a file sees every other file of its own module with no import statement. So for a Swift file `f` in target `M`:

> `imports(f) =` every source file of `M` other than `f`, **plus** for each `import N` site in `f`, every source file of the target named `N` if one exists (and `external` if none does).

Both halves matter.

- Without the first, an oracle in a sibling file of the test target is invisible: the test uses it with no import line to find.
- The second is why `external` is the right default for an unknown module name (§3.2): `Foundation`, `XCTest`, `Combine` and every SDK framework land there, and an oracle cannot, because an oracle is a file in the tree and a file in the tree is in a target.

**Classification stays per file** (§2.5): the implicit same-module edge means a branch-created `Sources/Billing/Oracle.swift` inside an existing target is reached, is found absent from `B`, is `EXCLUDED` by row 3, and **raises the closure tripwire**. Under a module-granular reading of "existed at `base=`" it would have passed silently, which is why §2.5 fixes the unit.

**A consequence to state honestly:** because every file of an imported target enters the walk, and because two files of one module implicitly import each other, a target with two or more files at `base` has every one of its files `EXCLUDED` by row 2 when it is inside `expected` — which is right, it is the code under test — and `FROZEN_LEAF` by row 5 when it is outside. A Swift closure is therefore module-shaped and considerably larger than the equivalent Python or TypeScript one. §18, OPEN-6.

### 7.5 `AncestorConfig(swift)` — clause 3 basenames

`Package.swift`, `Package.resolved`.

`Package.resolved` is on the list because it pins the exact revision of every dependency: changing it changes what the tests link against without touching a test file. It is not read for resolution; it is frozen.

### 7.6 Scaffolded `C-T2` patterns

```
Tests/Support/**
Package.swift
Package.resolved
```

`C-T1`'s default is `Tests/`.

### 7.7 Snapshot patterns (clause 4)

Final component matching `*.snap`, `*.approved.txt`, `*.golden`; or any path with a directory component named `__Snapshots__` or `Snapshots`.

### 7.8 The closed unclassifiable list

Language level: `manifest-not-literal`, `duplicate-target-name`, `xcode-project-unsupported`, `target-dir-missing`, `overlapping-targets`, `mixed-objc-target` (§7.3), `no-package-manifest` (a Swift file exists and no `Package.swift` does), `rc-changed-on-branch`.

Site level: `symlink-or-submodule`. That is the whole list — Swift has no string specifiers, no relative imports and no dynamic import, so a Swift `import` either names a target or is `external`.

---

## 8. Kotlin — withdrawn

**This section number is retained and deliberately empty.** Version 1 specified a Kotlin resolver here. The owner dropped Kotlin from v1 on 2026-08-26 (§18, OPEN-1) and the whole of version 1's §8 is preserved, unaltered but for section numbering, as **Appendix A**, together with its `§10` row, its worked example, its conformance cases and the `gradle` adapter's status.

The number is not reused and the sections below are not renumbered, because `constitution.md`, `dump.md`, `gate-report.md` and `result-file.md` all cite sections of this document by number and a renumbering would silently repoint every one of them. `.kt` and `.kts` are `lang: none` (§3.1); a `params.langs` naming `kotlin` is refused by `result-file.md` §7.1 step 3.

---

## 9. Judging the owner's provisional decision

> *"Resolve under the test configuration read from trunk, since freezing what the tests actually import when run is the point; genuinely unresolvable modules fall to the playbook's existing rule — excluded, and counted by `spine stats`."*

The decision is **right about the goal and wrong about the mechanism**, in three separable ways. The replacement is §3.3 and §3.7, and this section says plainly what was rejected and why, because the brief asks for a judgment rather than a compliance.

**1. "The test configuration" is not in any tree, so it cannot be read from one.** For Swift the phrase covers two very different things. One is *structure* — which files belong to which target — and that genuinely is in the tree, in `Package.swift`. The other is *configuration* — the active `#if` flags, the target platform, the build variant — and that is a property of the machine performing the build. `#if os(Linux)`, `#if canImport(UIKit)`, `if (dart.library.io)` and `debug` versus `release` are all answered by the builder, not by the repository. Reading them would make the closure computed on an approving laptop differ from the closure G8 recomputes in a Linux container — which is not a disagreement about policy but the exact failure this document exists to prevent. **Replaced by §3.7's union rule:** every branch of every conditional contributes, no configuration is read, and the closure over-freezes rather than mis-freezing.

**2. "From trunk" names the wrong tree twice.** The closure is computed over the *approval tree*, not trunk, so resolution has to happen against `A` (§2.2). But the *structure* that decides how resolution works must not come from `A`, because the branch writes `A` — a candidate that can edit `Package.swift` can move its oracle outside every target the resolver knows about, and the resolver at `--approve` and the resolver in CI will agree, both wrong. **Replaced by §3.3:** structure is `RC(lang, B)`, read from the base tree the branch cannot edit — the same reasoning PB §4.3 already applies to clause 2 — and a branch that changes `RC` raises `lang-unclassifiable` with reason `rc-changed-on-branch` and takes a human signature.

**3. `@testable import` is not a difficulty.** It changes visibility, not module membership, and resolves identically to `import` (§7.2). Listing it among the hard cases suggests the hard case was believed to be *which symbols* a file sees; the hard case is actually *which files*, and for Swift the answer is that a file sees its whole module with no import at all — which the brief does not mention and which is the thing that would have broken the closure. §7.4 handles it with an implicit same-module edge.

**What survives of the decision, and it is the important half:** the fall-back is right. A genuinely unresolvable module is excluded and counted, exactly as PB §4.3 says, and this document changes only the granularity — one `lang-unclassifiable` finding per language rather than one `unresolvable-import` per file, so a Swift repository outside the subset produces one tripwire a human can act on instead of four hundred that mean nothing (§2.11).

---

## 10. What each language can guarantee, stated at true size

PB §7.4's own habit — *"state the guarantee at its true size"* — applied here. The guarantee at stake is: **every file a frozen test transitively imports is either frozen or is code under test that already existed on trunk and was already used by trunk's own non-test code.**

| Language | Meets the bar | Under what restriction | Residual that does not close |
|---|---|---|---|
| **Python** | yes | none | `pyproject.toml`'s `package-dir` / `packages` remapping is not read (§4.2): a repository that relocates its package roots resolves against the two roots of §4.3 and its absolute imports become `external`, silently under-freezing. Detectable in review, not by the resolver. C extensions (`.so`, `.pyd`) are not in the tree and are `external`. |
| **TypeScript/JavaScript** | yes | one repository-root `tsconfig.json`; no `exports`/`imports` maps; index-only directory resolution | A monorepo whose workspace packages resolve through `package.json` `exports` rather than `tsconfig` `paths` sees those specifiers as `external` (§18, OPEN-7). Non-JS single-file components — `.vue`, `.svelte`, `.astro` — are `lang: none`, contribute no edges, and can hold an oracle. |
| **Dart** | yes | declarative `pubspec.yaml` | Generated parts (`*.g.dart`, `*.freezed.dart`) that are committed resolve normally; ones that are not committed are `unresolvable` under `no-candidate` and tripwire inside test roots. That is loud, which is correct, but it means a repository that gitignores its generated parts tripwires on every approval. |
| **Swift** | **yes, within SwiftPM** | a literal `Package.swift`; no Xcode project; **no target anywhere in the repository carrying a C-family entry or construct** (§7.3) | **Nothing silent.** A target containing `.m`/`.mm`/`.h` files, a bridging header, or a C-family manifest construct could hold an oracle the resolver cannot see — Objective-C is not one of the four languages, `lang` returns `none`, and no edge exists — so that shape is now `lang-unclassifiable`, reason `mixed-objc-target`, and is refused rather than under-frozen (§7.3). What is left is a capability limit, stated: such a repository cannot be gated for Swift until a release adds an Objective-C lexer. The other restriction is not as narrow as it looks — `swift test` requires SwiftPM, so a repository spine can gate is a SwiftPM repository by construction, and an Xcode-only project simply has no supported runner. |

Two things follow that the owner asked to hear plainly.

**Swift does meet the bar, and it now meets it the way Kotlin was required to.** The configuration it needs is the configuration its runner needs. What it does not *close* is Objective-C interop, which is nameable, narrow, and could be closed later by adding an ObjC lexer — the resolution rules are the same (module-granular, `#import "…"` and a bridging header) and nothing in this document's structure would change. What it no longer does is fail **silently** there: §7.3's `mixed-objc-target` refuses the shape, so the limit is reported at approval time instead of being discovered by an oracle passing through it. §18 OPEN-2 records the decision.

**Kotlin does not appear in this table, and version 1's row for it is Appendix A.** Its restriction was the conventional Gradle subset, which fails *loudly* — `layout-overridden` is one lexical test. Its residual was mixed Kotlin/Java modules, which fail *silently*: a `.java` file in a Kotlin source set is `lang: none`, contributes no edges, is the target of none, and Kotlin's same-package visibility means a test reaches a Java helper with no import line at all. A silent hole in the freeze closure is the failure mode PB §4.3 exists to close, and mixed Kotlin/Java modules are ordinary rather than exotic. Version 1 recommended detect-and-refuse; the owner dropped the language instead (2026-08-26), which closes the hole rather than reporting it.

**One rule, applied twice — this is the same decision as Swift's, not a different one.** The rule is PB §6.7's: *a guarantee that fails loudly can ship with its limits stated, one that fails silently cannot.* Kotlin and Swift both had a silent hole of identical shape, and the rule admits two conforming outcomes — remove the language, or make the hole loud. Kotlin's hole was **ordinary** (a mixed Kotlin/Java module is the common Gradle shape) and making it loud would have refused most Gradle repositories, so removal was the outcome that left a usable language set. Swift's is **not** ordinary (SwiftPM keeps a package's Swift and C-family sources in separate targets by convention, and neither shape is now silent), so detect-and-refuse leaves Swift gatable for the repositories that do not mix, and refuses the ones that do. The two decisions differ in which outcome the rule selects, never in the rule: **neither language ships a silent hole.** §18 OPEN-1 and OPEN-2 are the two halves.

Appendix A keeps the whole analysis so that a release adding Kotlin starts from it.

---

## 11. Runner adapters

`result-file.md` §6.3 assigns six obligations per runner to "the same specification" as the resolvers, and §6.4 says explicitly that the tokens must be *"ratified explicitly rather than inherited from a worked example."* This section discharges all five obligations for all four v1 runners. Version 1 discharged two and declined three; the consequence was that `Spine-Test` could not be written for a Dart or a Swift repository, G1's frozen-id roll-up and `B`-floor comparison were uncomputable there, and **no Dart or Swift repository could land**. Nothing about a runner adapter is optional for the same reason nothing about a resolver is: a `runner` token and an id grammar are sealed into landings forever (`result-file.md` §6.3 obligation 1), and two implementations that disagree on one reject each other's landings rather than merely differing.

The two adapters ratified here were written against reporter output **reproduced on a real toolchain**, not against a recollection of one. §11.7 records the toolchain versions, the exact commands, the observed lines, and the single fact that was taken from published source rather than reproduced.

### 11.1 The four `runner` tokens — ratified

| `params.langs` token | `runner` token | Invocation on `T` | `B` enumeration — what the floor's membership is taken from | `B` outcomes — what each `base` record's `out` is taken from | `B` invocations |
|---|---|---|---|---|---|
| `python` | `pytest` | `pytest` | `pytest --collect-only` (§11.2) | `pytest`, the `T` invocation run against the checkout of `B` (§11.2) | **two** |
| `ts` | `vitest` | `vitest run` | `vitest run` (§11.3) | the same run (§11.3) | one |
| `dart` | `dart-test` | `dart test --reporter=json --no-retry` | the same command (§11.4) | the same run (§11.4) | one |
| `swift` | `swift-test` | `swift test --disable-swift-testing` | `swift test list --disable-swift-testing` (§11.5) | `swift test --disable-swift-testing`, the `T` invocation run against the checkout of `B` (§11.5) | **two** |

**No adapter runs a command this section has not already ratified.** The `B` outcome run is the adapter's own `T` invocation, byte for byte, executed against a checkout of `B` — so its stream, its id composition, its outcome mapping, its terminal event and its refusals are §11.2–§11.5's, unchanged and already reproduced (§11.7). Nothing new had to be observed to fix it, which is why this section fixes it rather than filing it.

**What it costs, said plainly.** `vitest` and `dart-test` already obtain the floor by running the whole suite on `B` and discarding the outcomes, so for them `out` costs **nothing**: the outcomes were on the stream and were being thrown away. `pytest` and `swift-test` obtain the floor from a cheap listing that reports no outcome at all, so for them `out` costs **one more full run of the repository's suite against `B`, on every landing** — a Python or Swift repository pays two full suite runs per landing where it paid one. That is the price of PB §6.3's `xfail`/`skipped` exemption and there is no cheaper way to it: neither runner's expected-failure marker is decidable without running the test, and neither runner's cheap listing reports a skip either. pytest evaluates `xfail` at call time and admits `pytest.xfail()` and `add_marker` at run time, and Swift's `XCTExpectFailure` is a call inside the test body — so no listing, no static read of the tree and no collection-phase report can answer the question the exemption asks. The alternative is not a cheaper computation; it is withdrawing the exemption, which restores a deadlock with no override in the quick lane (`result-file.md` §5, §8.7).

**One `B` invocation or two, the ordering rule is the same.** Every `B` invocation of every runner — enumeration and outcome run alike — precedes every `T` execution (`result-file.md` §7.1 step 7, §7.4 rule 3). Interleaving is forbidden for the reason it was already forbidden: a process the candidate's tree spawned must never be alive while any runner is reading trunk's floor.

**A `B` outcome run that fails is not a `base-collect-failed`.** The floor's membership comes from the enumeration and is already whole; a `B` outcome run that will not start, dies, is killed at `params.timeout` or emits an unparsable stream leaves every id it did not report a terminal outcome for at `out: "absent"`, contributes no status, and moves no byte of the `end` record (`result-file.md` §4.4, §7.3). The asymmetry is the fail-closed direction: a truncated **enumeration** shrinks the floor, while a truncated **outcome run** can only withhold the carve-out's exemption. For `vitest` and `dart-test`, where the two are one invocation, only the enumeration rule can fire and `absent` is unreachable on a collection that succeeded.

**Every adapter's `B` collection is fixed here — both halves of it**, and three of the four differ from the `T` invocation in a way no reader could infer. Version 1 fixed none; version 2 fixed `dart-test`'s and `swift-test`'s and filed the other two as OPEN-11, which is a divergence risk of its own — `result-file.md` §7.1 step 7 requires a collection on `B` for **every** runner and says nothing about how. Version 3 closed the enumeration: §11.2 and §11.3 fix `pytest`'s and `vitest`'s against output reproduced on a real toolchain (§11.7). Version 4 closes the other half. `result-file.md` §4.4 now puts an `out` member on every `base` record and §6.3 obligation 6 requires an adapter to supply it, because PB §6.3's G1 and G8 rows carve out an id *whose collected outcome on `B` was already `xfail` or `skipped`* and no adapter produced a `B` outcome for two of the four runners to read.

**The rule the four share, stated once so that four adapters cannot drift apart:** the `B` floor is the set of ids the runner **collected and selected** on the checkout of `B` — every id it enumerated, less any it reported as *deselected*, and **irrespective of outcome**. The outcome is recorded beside the id, on the `base` record's `out` member (`result-file.md` §4.4), and it decides nothing about *membership*: an id trunk reported `failed`, `skipped` or `xfail` is in the floor exactly as an id it reported `passed` is. `out` has one reader, `result-file.md` §8.5 clause 2's `xfail`/`skipped` carve-out, and that carve-out is about which gate raises a finding, never about which ids the floor contains. A command that reports fewer ids than the runner collects writes a floor **smaller than `B`'s real one**, which §11.4 identifies as the one truncation that weakens the gate rather than tightening it; §11.3 refuses `vitest list` on exactly that ground, with the vector. A `base` floor that contains an id trunk itself reports as `skipped` is a consequence of "collected and selected" that all four adapters share, and it is a separate question from what `result-file.md` §8.5 clause 2 then does with that id: `skipped` is the carve-out's second exempt value, so such an id in the *did not pass* shape raises no G1 and no G8 finding (§18, OPEN-13, closed). Membership and allocation are decided by different rules and no adapter here departs from either unilaterally.

These are the values `result-file.md` §4.4's token grammar carries, the values `Spine-Test` lines are sealed with, and the values `test:<runner>:<id>` node ids use (PB §6.2). They are permanent. The mapping is 1:1 in v1 — one adapter per language — so `result-file.md` §6.2's invocation set is exactly the image of trunk's `params.langs` under this table. Every invocation runs at the **repository root** with no selection argument of any kind (`result-file.md` §7.2).

**Reserved, and not usable in v1 — one table.** Reserving costs nothing and prevents the one mistake that cannot be undone: a later release finding its natural token already spent. The table is here rather than in prose because three documents currently give three answers about it, and an implementer needs to know which strings a v1 release may never emit.

| Token | Kind | Status | Why |
|---|---|---|---|
| `kotlin` | `params.langs` value | **reserved** | §18 OPEN-1 dropped the language; its analysis is Appendix A, so a later release finds the string free. No v1 release emits it, and a `params.langs` containing it is refused by `result-file.md` §7.1 step 3 as a language with no adapter |
| `gradle` | `runner` | **reserved** | the adapter Kotlin would have used (Appendix A §A.5) |
| `jest` | `runner` | **reserved** | §18 OPEN-4: a second TypeScript adapter is a later release, and Jest is at least as common as vitest |
| `swift-testing` | `runner` | **not reserved; reservation recommended** | §18 OPEN-8 recommends reserving it now and it is the owner's call, not this document's. §11.5 detects swift-testing and fails the job rather than ignoring it |
| `junit`, `kotest` | `runner` | **contested** | `result-file.md` §6.4 reserves both; version 1 and version 2 of this document reserved neither; and `manifest.md` §3.3 says `kotlin` is *not* reserved at all, on the ground that a later release adding the language does so as a release. §18, OPEN-12 |

**The hard set is `kotlin`, `gradle`, `jest`.** Those three are emitted by nothing in v1 and no other adapter may take them (§20, item 26). The remaining rows are not this document's to settle, and no v1 release emits any of them either — so the disagreement costs nothing today and would cost a permanent token later, which is why OPEN-12 asks for one word rather than for an argument.

### 11.2 `pytest` — ratified

`result-file.md` §6.7 gives the rule as illustrative; it is ratified here without change.

- **`id → fn`:** split the nodeid on `::`. In the final component, the parametrization suffix begins at the **first** `[` and runs to the end, and exists only if the component's last byte is `]`. `fn` is the nodeid with that suffix removed. Python identifiers cannot contain `[`, so the split is exact and invertible, and `fn` is a prefix of `id` as `result-file.md` §6.3 obligation 2 requires.
- **`id → path`:** the component before the first `::`, as repo-relative POSIX bytes; the empty string where no tree entry matches.
- **Outcome mapping:** `result-file.md` §6.7's table, unchanged.

**`B` enumeration: `pytest --collect-only`**, run at the repository root on the checkout of `B`, through the same transport the `T` invocation uses (`result-file.md` §6.6). `--collect-only` is **not** a selection argument in the sense `result-file.md` §7.2 forbids: it narrows no test set and skips nothing, it runs the collection phase and stops before the first `call`. This is what the floor's membership is taken from, and it stays the authority for membership for the reason below.

**`B` outcomes: `pytest`** — this adapter's own `T` invocation, unchanged and with no selection argument, run at the repository root on the same checkout of `B`, through the same transport. Every `base` record's `out` is that run's outcome for that id under the mapping above; an id the run reported no terminal outcome for takes `out: "absent"` (`result-file.md` §4.4). **This is a second full run of the repository's suite on every landing**, and §11.1 states the cost rather than burying it here.

**Why a second invocation and not one.** `--collect-only` reports no outcome at all — it stops before the first `call`, which is where pytest decides `passed` from `xfail` — so it cannot supply `out`. Nor can any cheaper read: `pytest.mark.xfail` carries a `condition=` evaluated at run time, `pytest.xfail()` may be raised inside the test body, and `request.node.add_marker` may attach the marker during setup, so the expected-failure polarity is not a property of the tree and no lexical or collection-phase test decides it. The carve-out's other value is no cheaper: `--collect-only` collects a `@pytest.mark.skip` item and reports no outcome for it, a `skipif` condition is evaluated at setup, and `pytest.skip()` may be raised inside the test body. Running the suite is the only answer the runner offers.

**And why the enumeration is not simply taken from that run.** A full run can die part-way — a segfault in a C extension, a killed process group — and report fewer ids than it collected, which is a floor smaller than `B`'s real one and the one truncation §11.4 names as weakening the gate. `--collect-only` cannot: it either enumerates the whole set or interrupts and is `base-collect-failed` (below). Keeping membership on the enumeration and outcomes on the run means a broken outcome run costs an exemption and never a floor entry. It also costs nothing in agreement, because the two sets are the same set — which is reproduced rather than assumed, below.

**The id set it yields is identical to a full run's, and that is reproduced rather than assumed** (§11.7). Three cases decide it, and they are the three OPEN-11 named.

- A **`@pytest.mark.skip` or `skipif` item is collected under both.** The marker is evaluated at `call` time, long after collection, so the item is in the set either way.
- A **module-level `pytest.skip(allow_module_level=True)` yields zero items under both.** It raises during the module's import, which *is* collection, so the module contributes nothing to either set. `result-file.md` §6.6 already fixes what that means on `T` — *"collects nothing and errors nothing, so its ids are simply absent"* — and on `B` it means the same thing: those ids are not in the floor, under either command.
- A **`pytest_collection_modifyitems` deselection removes the same items under both.** Collection hooks run under `--collect-only`, and `pytest_deselected` fires there exactly as it does in a run.

So the divergence OPEN-11 feared is not reachable for pytest, and the cheap command is also the correct one **for membership** — which is why `pytest`'s `B` *enumeration* is not a full suite run where `dart-test`'s must be (§11.4). The reason the two differ is a fact about the runners, not a preference: pytest has a collection phase it can be asked to stop after, and `dart test` has no list-only mode at all. It is also why the three cases above still matter after the `B` outcome run was added: they are what makes the enumeration and the outcome run agree on the id set, so that `out: "absent"` is reachable only when the outcome run actually failed to reach an id and never merely because the two commands see different tests.

**Deselected ids are not in the floor.** They are the one thing the collection reports that the floor drops. An id trunk's own configuration excluded never runs on `B` and never runs on `T`, and `result-file.md` §8.5 clause 2 would then demand a `passed` `result` record for it on every landing, with no exit but a `class=protected` `G8:<path>` review naming it — a permanent block bought by nothing. Deselection is one of the four signals `result-file.md` §6.6 makes every transport preserve, so this is a rule a conforming collector can follow rather than a hope.

**Completeness.** pytest reports its own collected-and-selected count — `4 tests collected`, or `3/4 tests collected (1 deselected)` where a hook deselected (§11.7). The adapter compares it with the number of ids it extracted: fewer or more is `base-collect-failed` on `B` and `runner-failed` on `T`. This is `dart-test`'s root-`group` `testCount` check (§11.4) in pytest's own vocabulary, and it exists for the same reason — it is what tells a truncated collection from a genuinely short one.

**A collection error during the `B` enumeration is `base-collect-failed`.** (A collection error during the `B` *outcome* run is not: it is an `error` record's worth of information about `B`, the floor is already enumerated, and every id the run did not reach takes `out: "absent"` — §11.1.) pytest **interrupts** collection at the first error — `!!! Interrupted: 1 error during collection !!!`, exit 2, under `--collect-only` and under a full run alike (§11.7) — so every id it had not yet reached is missing from the set, and a floor truncated by an import error is a floor smaller than `B`'s real one. `result-file.md` §7.3's all-or-nothing rule then applies: no `base` and no `result` records from any runner. **Exit status is not the signal and must not be used as one**: `pytest --collect-only` exits `5` over a tree with no tests at all, which is a legitimate trunk before the first intent lands.

### 11.3 `vitest` — ratified, and the interpolation problem closed

- **`id → fn`:** `fn == id`, always.
- **`id → path`:** the substring before the first ` > `, as repo-relative POSIX bytes.
- **Outcome mapping:** as `result-file.md` §5's vocabulary, with vitest's `todo` mapping to `skipped` and its `passed`/`failed`/`skipped` mapping directly; anything else `unknown`.

`result-file.md` §6.4 names the hard case: *"interpolating parametrization — Jest and vitest `test.each` … leaves no unparametrized base name, so obligation 2's prefix property must be established for it or the runner declared unsupported."* **It is closed by making `fn` the identity.** vitest has no parametrization suffix: a `test.each` case is a test whose name is its own, and the runner reports it as one id like any other. Setting `fn == id` satisfies the prefix property trivially and has exactly one consequence, which is benign: each generated case is a separate `Spine-Test` entry rather than one rolled-up entry. Removing a case afterwards makes a frozen id absent, which is not a pass, which is a G1 failure — correct. Adding one is a harness edit, which G8 blocks — also correct.

**The id is composed, not reported.** vitest's own machine-readable run output names a file by an **absolute** path and names a test by a space-joined `fullName`, so the id of this section is assembled: the file's path made repo-relative, then `" > "`, then the enclosing suite titles outermost-first joined by `" > "`, then `" > "`, then the test's own title. That is the spelling `vitest list` prints (§11.7), the spelling PB §6.2's `test:<runner>:<id>` node id carries, and the spelling `result-file.md` §8.5 prints in its own worked pair. A reported file path that is not under the repository root makes the contribution `stream-invalid` (`result-file.md` §7.3) — §11.4's rule for `suite.path`, unchanged.

**Collection on `B`: the same invocation as `T` — `vitest run` — on the checkout of `B`.** One invocation serves both halves: the ids it reports are the floor's membership, and each id's outcome under the mapping above is that `base` record's `out` (`result-file.md` §4.4). The cost is one extra full suite run per landing, exactly as it is for `dart-test` (§11.4) and for the same kind of reason — and `out` adds **nothing** to it, since those outcomes were on the stream already and version 3 simply discarded them.

**`out: "absent"` is unreachable here on a collection that succeeded.** Every id in the set came from a report that carried a status, so every `base` record has a real outcome; a file that failed to load is `base-collect-failed` and writes no records at all (below). This adapter never produces `xfail` — vitest's mapping has no expected-failure value (above) — so a `vitest` `base` record reaches `result-file.md` §8.5 clause 2's carve-out only through its **other** exempt value, `skipped`, which this adapter does produce and which `vitest run` is chosen over `vitest list` precisely to keep in the floor (above). Never reaching the `xfail` limb is correct rather than a gap: a runner with no notion of expected failure has no test trunk declares expected-to-fail, so there is nothing on that limb to release.

**`vitest list` is refused, and the vector is the reason.** vitest ships a list-only mode whose stdout is one id per line in precisely this adapter's `<path> > <suites…> > <name>` spelling — as close to `swift test list` as a reader could want, and the obvious choice. It **omits every skipped test.** Reproduced (§11.7): a tree of five tests, two of which are behind `it.skip` and `describe.skip`, lists **three**; `vitest run` reports all five, the two missing ones with `status: "skipped"`. A floor built from it is therefore smaller than `B`'s real one by exactly the set trunk chose to skip, which is the truncation §11.4 names as the one that weakens the gate; and it would make `vitest` the only v1 adapter whose floor means something different from the other three's, which is the failure PB §12 states when it says a rule that means something different in two languages is a rule that rejects its own approvals. No flag restores them — the mode offers `--hideSkippedTests` and nothing that would un-hide them.

**A file that fails to load is `base-collect-failed` on `B`.** vitest reports such a file as a failed suite carrying an error message and **zero** test entries (§11.7), so its ids are simply missing from the set — the same shape as `dart-test`'s load pseudo-test (§11.4) and the same answer: if any file reports a load or transform error while collecting on `B`, this runner's contribution is `base-collect-failed`, and by `result-file.md` §7.3's all-or-nothing rule no `base` and no `result` records are written from any runner.

**vitest has no deselection**, so `pytest`'s deselection rule does not arise here: `deselected` is never produced by this adapter, and "collected and selected" (§11.1) collapses to "collected".

### 11.4 `dart-test` — ratified

**The runner emits no id.** `package:test`'s JSON reporter reports a *suite* (a path) and a *test* (a name, already qualified by its enclosing groups) as separate fields of separate events, and there is no single string the runner calls the test's identity. The adapter therefore **composes** one, and the composition is fixed here rather than left to an implementer — that is the whole of the divergence risk for this runner.

**Invocation and stream.** `dart test --reporter=json --no-retry`, run at the repository root, with the runner's stdout and stderr **merged into one pipe** the collector holds (`result-file.md` §6.6). A repository with no `pubspec.yaml` at its root cannot be run by this adapter: the contribution is `spawn-failed` (`result-file.md` §7.3), the one row whose meaning is "no runner configuration in the tree under test".

- `--no-retry` is **mandatory**. `package:test` honours a `retry:` in test metadata or in `dart_test.yaml`, and a retried test is reported as a fresh `testStart` under the same name, which composes to the same `id` and would make the file's `(runner, id)` pair non-unique — malformed under `result-file.md` §4.4. It is not a selection argument in the sense `result-file.md` §7.2 forbids: it narrows no test set and skips nothing, it removes a repetition.
- No `--timeout`, no `--concurrency`/`-j`, no `-p`/`-c`, no `--test-randomize-ordering-seed`, no `-n`/`-N`/`-t`/`-x`, no path or directory argument. The per-test timeout is the repository's, in `dart_test.yaml`, which clause 3 freezes; the job deadline is `params.timeout`, which the collector enforces.

**Events read.** A line of the merged stream is an **event** iff it parses as a JSON object with a string `type` member; every other line is discarded (build diagnostics, and a test's own raw writes to `stdout`). Only four event types are read — `start`, `suite`, `testStart`, `testDone` — plus `group` for the completeness check below, and `done` as the terminal event. `allSuites`, `print`, `error` and `debug` are read for nothing: `testDone.result` already distinguishes a failure from an error, and `testDone.skipped` already carries a skip.

- The `start` event's `protocolVersion` must begin `0.1.`; otherwise this runner's contribution is `stream-invalid` (`result-file.md` §7.3). Observed: `0.1.1`.
- **Terminal event:** `done`. `result-file.md` §7.3's `complete` requires it to have been parsed *and* the process group to have exited of its own accord.

**The id.** For a `testDone` event, let `e` be the `testStart` event whose `test.id` equals its `testID`, and `S` the `suite` event whose `suite.id` equals `e.test.suiteID`. A `testDone` whose `testID` was introduced by no prior `testStart`, or a `testStart` whose `suiteID` names no prior `suite`, is **discarded** — this is what makes a forged event that invents an identity inert. Let `sp` be `S.suite.path`; the reporter documents it as absolute or relative to the package root, so an absolute `sp` under the repository root is made repo-relative and any other absolute `sp`, or a null one, makes the contribution `stream-invalid`. Then:

```
id := sp ++ "::" ++ e.test.name
fn := id
```

- **`id → path`** is the bytes before the **first** `::`, mapped onto a tree entry (of `B` for a `base` record, of `T` for a `result` record) and emitted as the tree's bytes; the empty string where no entry matches. §11.6 rule 3 governs a repository path that itself contains `::`.
- **`fn == id`** for the same reason as vitest (§11.3): Dart's parametrization idiom is a `for` loop or a generator building the name by interpolation, which leaves no unparametrized base name to roll up to. The prefix property of `result-file.md` §6.3 obligation 2 holds trivially, each generated case is its own `Spine-Test` entry, and the consequences are §11.3's, unchanged.
- `test.name` is already qualified by every enclosing group, joined with a single `U+0020` — the reporter documents it as *"the name of the test, including prefixes from any containing groups"*, and it is reproduced in §11.7. The adapter does not re-join anything.

**Which `testDone` events yield a record.** Total and ordered; first match wins.

| # | Condition | `base` | `result` |
|:--:|---|---|---|
| 1 | `test.groupIDs` is empty — the suite's *load* pseudo-test — and `hidden` is `true` | no | no |
| 2 | `test.groupIDs` is empty and `hidden` is `false` — the suite **failed to load** | no, and on `B` see below | yes, one record, `out = error` |
| 3 | `test.name` is `(setUpAll)` or `(tearDownAll)`, or ends with `U+0020` followed by one of those — a **scaffold** test | **no** | yes |
| 4 | `hidden` is `true` | no | no |
| 5 | otherwise | yes | yes |

Rows 1, 2 and 3 are load-bearing.

- **The load pseudo-test.** `package:test` reports every suite's compilation as a test named `loading <path>` with an empty `groupIDs`; it is `hidden` when it succeeds. The empty `groupIDs` is the discriminator, not the name — every real test sits in at least the suite's implicit root group. When it *fails*, `hidden` is `false` and it is exactly `result-file.md` §6.6's *"collection error that yields no item id"*, recorded as one `error` record under the runner's own id for the failing collector.
- **A load failure during the `B` collection is a collection failure.** If any suite's load pseudo-test reports a non-`success` result while collecting on the checkout of `B`, this runner's contribution is **`base-collect-failed`** — which, by `result-file.md` §7.3's all-or-nothing rule, means no `base` and no `result` records from any runner. Anything weaker writes a floor that is smaller than `B`'s real one, and a shrunken floor is the one truncation that weakens the gate rather than tightening it.
- **Scaffold tests never enter the floor.** `package:test` creates `(setUpAll)` and `(tearDownAll)` as synthetic tests — the two literal names are its whole set — and reports them **only when they fail**. An id whose existence is conditioned on its own failure cannot be frozen: it enters the `B` floor when the hook is broken on trunk, and the moment the hook is fixed the id disappears, and absence is not a pass (`result-file.md` §5). That is a permanent, unfixable G1 block reachable by fixing a bug, so the `base` section excludes them by name. They are still written to the `result` section, where they are evidence and never credit.

**Completeness.** Each `suite` has exactly one root `group` (the one with `parentID` null), whose `testCount` is the number of real tests in that suite — scaffold and load pseudo-tests excluded. The adapter compares it, per suite, with the number of **row-5** records it emitted for that suite. Fewer means the suite was truncated: the contribution is `runner-failed` on `T` and `base-collect-failed` on `B`. More means the stream carried an item the runner did not declare: the same statuses. This is what lets the adapter tell a suite whose VM died half-way from a suite that was genuinely short, which nothing else in the protocol does.

**Outcome mapping**, evaluated top to bottom, first match wins:

| `testDone` observation | `out` |
|---|---|
| `skipped` is `true` | `skipped` |
| `result` is `"success"` | `passed` |
| `result` is `"failure"` | `failed` |
| `result` is `"error"` | `error` |
| any other `result` value | `unknown` |

`skipped` is tested **first**, and that order is not cosmetic: `package:test` reports a skipped test as `{"result":"success","skipped":true}`, so a mapping that read `result` first would credit every skipped test as a pass — the one direction `result-file.md` §5 may not get wrong. `xfail`, `xpass` and `deselected` are never produced: `package:test` has no expected-failure marker, and it has no deselection the collector can reach, since tag and name filters are selection arguments the collector never passes.

**Collection on `B` runs the suite, and that is stated at its true size.** `dart test` has no list-only mode — its selection flags are `-n`, `-N`, `-t` and `-x`, all of which `result-file.md` §7.2 forbids, and there is no dry run. The `B` id set is therefore obtained by running the same invocation against the checkout of `B`; one invocation serves both halves, the ids being the floor's membership and each id's outcome under the mapping above being that `base` record's `out` (`result-file.md` §4.4). `result-file.md` §7.4 rule 3 is satisfied unchanged — every `B` invocation still precedes every `T` execution — and the cost is one extra full suite run per landing, to which `out` adds nothing: the outcomes were on the stream already and version 3 discarded them. The completeness check above is what keeps that run honest.

**Rows 1–4 of the table above still write no `base` record**, so they carry no `out` either — a scaffold test in particular is excluded from the floor by name and nothing about `out` readmits it. `out: "absent"` is unreachable on a collection that succeeded, every row-5 record having come from a `testDone`. And this adapter never produces `xfail`: `package:test` has no expected-failure marker, so a `dart-test` `base` record reaches `result-file.md` §8.5 clause 2's carve-out only through its **other** exempt value, `skipped`, which row 5's mapping does produce and tests first. Never reaching the `xfail` limb is correct — a runner with no expected-failure concept has no test trunk declares expected-to-fail.

### 11.5 `swift-test` — ratified

**The runner has two surfaces, and the adapter reads the stable one for identity and the other only for outcomes.** SwiftPM prints test identities in its own **specifier format**, which is the format `--filter` consumes and is the same on every platform. XCTest prints outcomes through its `PrintObserver`, whose spelling of a test's name is **not** the same on every platform. Taking the id from SwiftPM and the outcome from XCTest is the whole design of this adapter.

**Scope.** `swift-test` covers **XCTest**. swift-testing (`@Test`) has no v1 adapter, and it is **detected rather than ignored** (below). A repository with no `Package.swift` at its root cannot be run by this adapter: the contribution is `spawn-failed`.

**`B` enumeration:** `swift test list --disable-swift-testing`, run at the repository root on the checkout of `B`. Its **stdout** carries one specifier per line and nothing else; SwiftPM writes build progress to stderr, which this command's reader discards. A non-zero exit, or a non-empty stdout line that is not a specifier, makes the contribution `base-collect-failed`. This is the authority for the floor's membership, and it stays so.

**`B` outcomes:** `swift test --disable-swift-testing` — this adapter's own `T` invocation, unchanged, run at the repository root on the same checkout of `B`, stdout and stderr merged into one pipe, `--parallel` never passed. The stream is read exactly as the `T` stream is read below: the same two `PrintObserver` spellings, the same `(class-path, method)` extraction, the same join against the `swift test list` output, the same multiplicity rule and the same outcome mapping. Each `base` record's `out` is the value that mapping yields for its id; an id with no terminal line in that run takes `out: "absent"` (`result-file.md` §4.4). **This is a second full run of the repository's suite on every landing**; §11.1 states the cost.

**Why a second invocation and not one.** `swift test list` prints specifiers and nothing else — no verb, no outcome, no `Expected failure` line — so it cannot supply `out`. And the datum the carve-out needs is `xfail` **or** `skipped`: the first comes for this adapter from an `Expected failure in <case identity>:` line that `XCTExpectFailure` prints from **inside the test body** (below), and the second from a `skipped` verb on the `Test Case` line, which `XCTSkip` is thrown from inside the body to produce. There is no listing, no build product and no static read of the tree from which either can be predicted. Running the suite is the only answer XCTest offers.

**And why the enumeration is not taken from that run.** A case whose process crashes emits no terminal line, so a run-derived id set is smaller than `swift test list`'s by exactly the cases that died — a shrunken floor bought by a crash. `swift test list` enumerates from the built test bundle and is unaffected. Membership therefore stays on the listing and outcomes on the run, and the two disagreeing in that direction is precisely what `out: "absent"` records. The `ambiguous-test-class` refusal is unchanged and is checked on the listing, once, before either run is joined.

**The id is the specifier line, byte for byte:**

```
id := <target> "." <class-path> "/" <method>
fn := id
```

`fn == id` because XCTest has no parametrization: a test is a method, and a method has no suffix to remove.

**swift-testing detection.** The adapter additionally runs `swift test list --enable-swift-testing --disable-xctest`. **Any non-empty stdout makes the collector fail the job and write nothing**, the same shape `result-file.md` §7.1 step 3 gives a language whose adapter the release does not have, and for the same reason: a v1 adapter that ran with `--disable-swift-testing` and said nothing would silently omit from the floor exactly the tests a repository migrating to swift-testing trusts most. The finding is `swift-testing-unsupported`. §18, OPEN-8 records when it goes away.

**Invocation on `T`:** `swift test --disable-swift-testing`, at the repository root, stdout and stderr merged into one pipe. `--parallel` is **never** passed. SwiftPM's default is already `--no-parallel`, and the reason it must stay that way is that `--parallel` runs several `xctest` processes onto one stream, after which a per-case line cannot be attributed to a process and the multiplicity rule below cannot tell a second process from a forgery.

**The stream.** XCTest's `PrintObserver` emits one line per case transition. Two spellings of the case identity exist and both are read:

| Toolchain | Terminal line |
|---|---|
| Darwin (Objective-C runtime) | `Test Case '-[<target>.<class-path> <method>]' <verb> (<t> seconds).` |
| swift-corelibs-xctest | `Test Case '<class-path>.<method>' <verb> (<t> seconds)` |

`<verb>` is one of `passed`, `failed`, `skipped`; the closed verb set is what keeps the `started` line out. The **case identity** the adapter extracts from either spelling is the pair `(class-path, method)`. Note that the corelibs spelling carries no target: `XCTestCase.name` is `"\(type(of: self)).\(name)"`.

**The terminal event** is the last line matching `Test Suite 'All tests' <verb> at <rest>` with `<verb>` in `{passed, failed}`. `result-file.md` §7.3's `complete` requires it.

**The join, and the one refusal it needs.** Each `(class-path, method)` from the stream is joined to the specifier id from `swift test list` that ends `.<class-path>/<method>`. If two ids in that list share a `(class-path, method)` under different targets, the join is not single-valued on a corelibs toolchain: the collector **fails the job and writes nothing**, finding `ambiguous-test-class`. It is one lexical test over the list output, it is loud, and the repository's remedy is to rename a class. A case identity in the stream that matches no id in the list is discarded.

**Outcome mapping**, evaluated top to bottom, first match wins:

| Observation for the case | `out` |
|---|---|
| more than one terminal line in the run | `unknown` |
| a line matching `<any>Expected failure in <case identity>:<any>` occurred in the run | `xfail` |
| `<verb>` is `passed` | `passed` |
| `<verb>` is `failed` | `failed` |
| `<verb>` is `skipped` | `skipped` |
| no terminal line at all | no `result` record — the id is *absent*, which is not a pass |

Three rows carry the weight.

- **The `Expected failure` row is not a nicety.** `XCTExpectFailure` makes XCTest print `Expected failure in <case>: …` and then report the case as **`passed`** (reproduced in §11.7). Without this row a test that fails behind an expected-failure marker is credited as a pass, which is precisely the value `result-file.md` §5 says is never a pass. `xpass` is never produced by this adapter: a strict `XCTExpectFailure` whose failure does not occur is reported by XCTest itself as `failed`, which is also not a pass, so nothing is lost by not distinguishing it. `XCTExpectFailure` does not exist on corelibs, so the row is unreachable there.
- **The multiplicity row is a defence, and the attack is reproduced.** XCTest's per-case lines go to the same stream a test's own `print` goes to, so a test can emit a byte-identical `Test Case '…' passed (0.001 seconds).` line naming *another* test — §11.7 shows it working. The rule makes the forged line collide with the real one and the outcome `unknown`, which is not a pass. A candidate can therefore downgrade an outcome and never upgrade one, and downgrading its own landing is something `result-file.md` §9 already grants it. The general position is unchanged and is not weakened by having a defence: `result-file.md` §7.2, *"G1 is exactly as strong as the runner's honesty and no stronger"*.
- **`error` and `deselected` are never produced.** XCTest reports a setup or teardown failure as a failure of the case, not as a separate kind, and it has no deselection the collector can reach. A build failure or a crashed process emits no terminal line for any case: every id is absent, and the status is `no-output` or `runner-failed` by `result-file.md` §7.3.

**`id → path`: a lexical declaration lookup, because the id carries no path.** XCTest identifies a test by class and method and never by file, so obligation 3 is discharged against the tree rather than against the id's own bytes. For an id and a tree `X`:

1. Let `M` be the **longest** target name in `RC(swift, X)` (§7.3) such that the id begins `M ++ "."`. No such target → the empty string.
2. Let `C` be the bytes between that `.` and the first `/`, and `c` the last `.`-separated component of `C` — the class's own name, where `C` is a nested class path.
3. Among the source files of `M` in `RC(swift, X)`, a file **declares** `c` iff its token stream (lexed by §7.1, with comments and string literals discarded) contains a `word` token `class` immediately followed by a `word` token equal to `c`. This reaches `final class C`, `public final class C`, `class C: XCTestCase` and `class C<T>` without a parser.
4. Exactly one such file → its path, emitted as the tree's bytes (`B` for a `base` record, `T` for a `result` record). Zero or several → **the empty string.**

The empty string is the fail-closed answer and it costs the id two things and only two: `result-file.md` §4.4's `G8:<path>` exemption, and §12.2's pragma join. A candidate that plants a decoy `class InvoiceTests` in a second file of the same target buys itself the empty string, which is strictly worse for it than doing nothing.

### 11.6 Rules shared by every adapter

1. **The `runner` token is a constant of the adapter** (`result-file.md` §4.4): never read from a stream, a manifest, `params.langs` or the environment.
2. **`(runner, id)` uniqueness is a collector precondition, not a hope.** `result-file.md` §4.4 makes a repeated `(runner, id)` pair *malformed*, so a conforming collector must never write one. If two distinct reported items compose to one id under any runner, the collector **fails the job and writes nothing** — finding `duplicate-test-id`, carrying the runner token and the id. This is reachable in an honest repository (two `test('x')` calls with the same name in one Dart suite compose to one id — reproduced in §11.7) and it is also where a forged stream event lands, so one rule serves the accident and the attack. It tightens `pytest` and `vitest` too, where nothing previously said what a collector does with a duplicate; it changes neither adapter's `id → fn` nor its `id → path`.
3. **A separator inside a path.** Where an adapter's `id → path` is "the bytes before the first `<sep>`", a repository path containing `<sep>` makes the function ambiguous: the collector **fails the job and writes nothing**, finding `id-separator-in-path`. `<sep>` is `::` for `pytest` and `dart-test` and ` > ` for `vitest`; `swift-test` has no such split and is unaffected. Refusing is the only fail-closed answer available — silently dropping the suite would narrow the `B` floor by exactly the tests a directory name chose, which is an attack, and guessing the split would make the same bytes name two tests.
4. **The `B` outcome is the adapter's own mapping, applied to `B`.** Obligation 6 of `result-file.md` §6.3 is discharged by running the adapter's *`T` invocation* against the checkout of `B` and passing its terminal reports through the **same** outcome mapping the adapter uses on `T` (obligation 4). No adapter defines a second mapping, a `B`-only value or a `B`-only refusal, and none may: `result-file.md` §8.5 clause 2 compares `b.out` against the two literals `xfail` and `skipped`, so a `B` mapping that differed from the `T` mapping would make the carve-out mean one thing on trunk and another on the branch. The one value the `T` side has no use for is `absent`, which is not produced by a mapping at all — it is what an id that the `B` outcome run reported no terminal outcome for gets (`result-file.md` §4.4), and it is written by the collector rather than by the adapter.
5. **`xfail` is producible by two of the four adapters, `skipped` by all four, and the carve-out reads both.** `pytest` produces `xfail` from the expected-failure marker (§11.2) and `swift-test` from an `Expected failure in` line (§11.5); `vitest` and `dart-test` have no expected-failure value in their mappings at all (§11.3, §11.4). `skipped` is in all four mappings. So `result-file.md` §8.5 clause 2's carve-out — one predicate over `xfail` **or** `skipped` — is reachable in **all four** suites, while its `xfail` limb alone is reachable only in a Python or a Swift one. That limb is not what makes obligation 6 cost a second invocation: `pytest` and `swift-test` are exactly the pair whose `B` enumeration reports no outcome, and neither `pytest --collect-only` nor `swift test list` reports a skip either, so both limbs need the same second run for the same two runners. An implementer who reads §11.1's cost table and wonders whether the cheap half could be skipped for the expensive runners has it backwards.
4. **`fn` is a prefix of `id`**, checked per record by `result-file.md` §4.4. Every v1 adapter satisfies it: `pytest` by construction, and `vitest`, `dart-test` and `swift-test` because `fn == id`.
5. **The six obligations of `result-file.md` §6.3 are discharged here for all four runners**, and nowhere else. Obligation 6 — a `B` outcome per collected id — is the newest and the only one with a per-landing price; §11.1 fixes what each adapter runs to satisfy it and what that costs. There is no `docs/spec/runner-adapters.md` and none is owed.
6. **Every adapter names its terminal session-end event**, because `result-file.md` §7.3's `complete` is defined in terms of one: `dart-test`'s is the `done` event, `swift-test`'s is the final `Test Suite 'All tests' <verb> at …` line.

### 11.7 How §11.2 – §11.5 were ratified

Written against runner output reproduced on these toolchains, on `arm64-apple-macosx`:

| | Version | Surface |
|---|---|---|
| Python | CPython 3.14.5 · pytest 9.1.1 | collection reports under `--collect-only` and under a full run |
| TypeScript | Node.js 26.0.0 · vitest 4.1.11 | `vitest list` stdout; `vitest run` machine-readable run output |
| Dart | Dart SDK 3.12.0 (stable) · `package:test` 1.31.2 | JSON reporter, `protocolVersion` `0.1.1` |
| Swift | Apple Swift 6.3.2 (`swiftlang-6.3.2.1.108`) · SwiftPM `swift test` | `swift test list` specifiers; XCTest `PrintObserver` lines |

**`pytest`, observed.** For `tests/test_invoice.py` holding a two-case `@pytest.mark.parametrize`, a `@pytest.mark.skip` function and a `TestRounding` class with one method, plus `tests/test_modskip.py` whose second line is `pytest.skip("module level", allow_module_level=True)` above one function:

```
$ pytest --collect-only -q                    # stdout
tests/test_invoice.py::test_AC1_totals_include_tax[zero-rate]
tests/test_invoice.py::test_AC1_totals_include_tax[std]
tests/test_invoice.py::test_AC2_zero_rated_lines
tests/test_invoice.py::TestRounding::test_half_even

4 tests collected in 0.00s

$ pytest -q -rA                               # stdout, filtered
PASSED tests/test_invoice.py::test_AC1_totals_include_tax[zero-rate]
PASSED tests/test_invoice.py::test_AC1_totals_include_tax[std]
PASSED tests/test_invoice.py::TestRounding::test_half_even
SKIPPED [1] tests/test_modskip.py:2: module level
SKIPPED [1] tests/test_invoice.py:7: decorator skip
3 passed, 2 skipped in 0.00s
```

The two commands agree on the id set, which is §11.2's whole claim: four ids under both, the decorator-skipped one among them, and **none** from `test_modskip.py` under either — a module-level skip raises during import, which is collection. Note that the full run additionally reports the module-level skip as a *file-and-line* skip carrying no item id, which is why it can never enter a floor keyed by id.

Also reproduced. Adding a root `conftest.py` whose `pytest_collection_modifyitems` moves `test_half_even` into `pytest_deselected` gives `3/4 tests collected (1 deselected)` under `--collect-only` and `2 passed, 2 skipped, 1 deselected` under a run — the same item deselected under both, so collection hooks demonstrably run under `--collect-only`. Adding a `tests/test_broken.py` whose first line imports a module that does not exist gives, under **both** commands, `ERROR tests/test_broken.py`, `!!! Interrupted: 1 error during collection !!!` and exit status **2** — collection stops at the first error, which is why §11.2 makes it `base-collect-failed` rather than a partial floor. Over an empty tree, `pytest --collect-only -q` prints `no tests collected` and exits **5**, which is why §11.2 forbids reading the exit status as the completeness signal.

**`vitest`, observed.** For `tests/invoice.test.ts` holding a `describe('invoice totals')` with one `it` and one `it.skip`, plus a two-case `test.each`, and `tests/skipped.test.ts` holding a `describe.skip` with one test:

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

**Three against five, and the two missing are exactly the two skipped.** That is the vector §11.3 refuses `vitest list` on. Note also that `vitest list`'s line format is byte-for-byte this adapter's id — the confirmation that §11.3's `id → path` split at the first ` > ` reads a real spelling of the runner's and not an invented one — while the run's own machine-readable form names the file by an absolute path and the test by a space-joined `fullName`, which is why §11.3 says the id is composed.

Also reproduced: a test file whose first line imports a module that does not exist appears in the run output as a **failed file entry with zero test entries** and a `Cannot find module …` message, while the other files report normally — so its ids are absent rather than reported, which is §11.3's `base-collect-failed` case. `vitest list --help` offers `--hideSkippedTests` and no inverse.

**`dart-test`, observed.** For `test/billing/invoice_test.dart` containing one top-level test, a `group('rounding')` with two tests of which one carries `skip:`, and a `for` loop generating two interpolated names, `dart test --reporter=json --no-retry` emitted a root `group` with `"testCount":5` and five `testDone` events, which compose to these five `result` records — printed in `result-file.md` §4.3 canonical form and §4.5 order:

```json
{"fn":"test/billing/invoice_test.dart::AC1 totals include tax","id":"test/billing/invoice_test.dart::AC1 totals include tax","out":"passed","path":"test/billing/invoice_test.dart","runner":"dart-test","t":"result"}
{"fn":"test/billing/invoice_test.dart::rate std applies","id":"test/billing/invoice_test.dart::rate std applies","out":"passed","path":"test/billing/invoice_test.dart","runner":"dart-test","t":"result"}
{"fn":"test/billing/invoice_test.dart::rate zero applies","id":"test/billing/invoice_test.dart::rate zero applies","out":"passed","path":"test/billing/invoice_test.dart","runner":"dart-test","t":"result"}
{"fn":"test/billing/invoice_test.dart::rounding AC2 banker rounding","id":"test/billing/invoice_test.dart::rounding AC2 banker rounding","out":"skipped","path":"test/billing/invoice_test.dart","runner":"dart-test","t":"result"}
{"fn":"test/billing/invoice_test.dart::rounding half even","id":"test/billing/invoice_test.dart::rounding half even","out":"passed","path":"test/billing/invoice_test.dart","runner":"dart-test","t":"result"}
```

Note `rounding half even`: the group prefix is already in `test.name`, joined with one space, and the adapter re-joins nothing. Note also that the skipped test's raw event was `{"result":"success","skipped":true,…}`, which is why §11.4's mapping tests `skipped` first.

Also reproduced: a nested `group('outer'){group('inner')}` yields `test.name` `outer inner deep AC2 case`; a failing `tearDownAll` appears as a test named `outer (tearDownAll)` with `hidden` `false` while a succeeding `setUpAll` appears with `hidden` `true`; a file that does not compile yields a `testDone` for `loading <path>` with `"result":"error"` and `hidden` `false`; two `test('dup name')` calls in one suite yield two events composing to one id (§11.6 rule 2); and a test calling `stdout.writeln` with a forged `testDone` object put that object on the stream verbatim, where §11.4's "no prior `testStart`" rule discards it. The two synthetic names are the closed set `(setUpAll)`, `(tearDownAll)`, confirmed in `package:test`'s declarer, and the `test.name` and `suite.path` semantics quoted in §11.4 are the JSON reporter protocol document's own words.

**`swift-test`, observed.** For a `BillingTests` test target with `InvoiceTests` (three methods, one of which throws `XCTSkip` and one of which sits behind `XCTExpectFailure`) and `RoundingTests` (two methods, one failing):

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

giving these five `result` records:

```json
{"fn":"BillingTests.InvoiceTests/testAC1TotalsIncludeTax","id":"BillingTests.InvoiceTests/testAC1TotalsIncludeTax","out":"passed","path":"Tests/BillingTests/InvoiceTests.swift","runner":"swift-test","t":"result"}
{"fn":"BillingTests.InvoiceTests/testAC2BankerRounding","id":"BillingTests.InvoiceTests/testAC2BankerRounding","out":"skipped","path":"Tests/BillingTests/InvoiceTests.swift","runner":"swift-test","t":"result"}
{"fn":"BillingTests.InvoiceTests/testKnownBad","id":"BillingTests.InvoiceTests/testKnownBad","out":"xfail","path":"Tests/BillingTests/InvoiceTests.swift","runner":"swift-test","t":"result"}
{"fn":"BillingTests.RoundingTests/testFails","id":"BillingTests.RoundingTests/testFails","out":"failed","path":"Tests/BillingTests/RoundingTests.swift","runner":"swift-test","t":"result"}
{"fn":"BillingTests.RoundingTests/testHalfEven","id":"BillingTests.RoundingTests/testHalfEven","out":"passed","path":"Tests/BillingTests/RoundingTests.swift","runner":"swift-test","t":"result"}
```

`testKnownBad` is the row that matters: XCTest printed `passed` for it, and the `Expected failure in` line is the only thing between that and G1 crediting a failing test.

Also reproduced: `swift test --xunit-output` produces a file only under `--parallel`, and that file records the skipped case and the expected-failure case as **plain passing `<testcase>` elements with no marker** — which is why the xUnit file is not this adapter's transport, and why no future adapter may make it one without first solving that. And the forgery: a test whose body is `print("Test Case '-[BillingTests.InvoiceTests testFailing]' passed (0.001 seconds).")` put that exact line on the stream, giving `testFailing` two terminal lines — one `failed` from XCTest and one forged `passed` — which §11.5's multiplicity row maps to `unknown`.

**The one fact not reproduced here.** The corelibs spelling of the `Test Case` line, and `XCTestCase.name` being `"\(type(of: self)).\(name)"` with no target, are taken from `swift-corelibs-xctest`'s published source (`Sources/XCTest/Private/PrintObserver.swift` and `Sources/XCTest/Public/XCTestCase.swift`), not from a Linux run on this host. Everything that depends on it is in §11.5's join and its `ambiguous-test-class` refusal. §18, OPEN-10 makes a Linux reproduction a release-blocking item rather than an assumption.

---

## 12. The lexical reads: the source-symbol → runner-id join, and `C-T3`

Two reads that are not the closure, and that live here for one reason: both are per-language lexical reads of the files the resolver already lexes, over tokens §3.4 already produces. §12.1–§12.3 are the join. §12.4 is `C-T3`'s predicate.

`docs/spec/README.md` listed the join as a known gap — *"How a `@verifies` pragma or a `test_AC1_*` name in a blob maps to a runner-native test id is assumed by G1's coverage clause, by `Spine-Test`, and by both specs — and no document defines it."* **This section is what closes it**, and the entry has been withdrawn there; `gate-report.md` §5.4.2 and §11, `result-file.md` §12, `intent-doc.md` §13, `templates.md` §16 and `dump.md` §16 all now point here rather than at a gap. PB §4.3 assigns `C-T3` here by name, and says why it may not be left to an implementer: *"What counts as a framework import or a hook is per language and closed — `docs/spec/import-resolver.md` lists both sets, alongside the pragma forms it already lexes from the same files — because §7.4 rests part of its isolation argument on this grep, and a security boundary whose predicate is unwritten is not one."*

§12.1 is also what §2.1.1's seed rule reads, so the pragma's grammar is load-bearing three times over: for the join, for G5's orphan clause, and for which files the freeze closure starts from.

### 12.1 The pragma — canonical, and identical across languages

PB §6.2: *"pragmas `@verifies INT-042/AC-1` in a comment (canonical, and identical across languages)"*.

A **pragma occurrence** is, inside a `comment` token (§3.4 rule 4, which is why comments are scanned before being discarded):

```
@verifies <SP>+ <intent-id> "/" "AC-" <digit>+
```

where `<SP>` is `U+0020` or `U+0009` and `<intent-id>` is an intent id **exactly as `intent-doc.md` §3.1 defines one**: `("INT" | "BUG") "-" numeral`, the numeral a decimal integer left-padded with `0` to a minimum width of 3 and padded no further. So `INT-042`, `BUG-051` and `INT-1042` are ids and `INT-42`, `INT-0042`, `INT-000` and `int-042` are not. The scan is over the comment's decoded bytes; a comment may carry several occurrences, separated by any bytes. `@verifies` must be preceded by a byte outside `[A-Za-z0-9_@]` or be at the comment's start, so `x@verifies` is not one.

**Version 2 wrote `^(INT|BUG)-[0-9]+$` here and that was a second id domain.** It admits both `INT-42` and `INT-042`, which `intent-doc.md` §3.1 refuses and whose bijection with the integer three mechanisms depend on. The divergence bites in this document rather than in that one: two implementations disagreeing on it disagree about whether `@verifies INT-42/AC-1` is an occurrence at all, hence about whether the file is a seed (§2.1.1), hence about the whole closure — and about whether G5 fires. One domain, and it is `intent-doc.md` §3.1's.

**The AC number is captured as written, and compared canonically.** `<digit>+` is deliberately wider than `intent-doc.md` §5.3's `1 … 6`: a pragma naming `AC-9` must be *recognized* in order to be reported, since PB §6.3's G5 fails loudly on *"a `verified_by` edge to a nonexistent AC (typo'd pragma)"* and a grammar that declined to recognize it would make the orphan invisible instead. Membership in `AC` (§2.1.1) and G5's orphan test then compare the captured digit run against §5.3's spelling — a decimal `1 … 6` with no leading zeros — so `AC-9`, `AC-01` and `AC-007` are occurrences that name no acceptance criterion, seed nothing, and are G5 findings. One number, one spelling, for the reason `intent-doc.md` §3.1 gives about ids.

The comment forms are the four languages' own: `#` for Python; `//` and `/* */` for the other three (nested for Dart and Swift, §6.1/§7.1); Python has no block comment. Docstrings are **not** comments and are not scanned — a `@verifies` in a Python docstring does not count, because a docstring is a string literal and the resolver's lexer classifies it as one.

### 12.2 The join is file-granular

A pragma occurrence in file `P` attributes to **every collected test id whose `id → path` equals `P`**, for every runner in the invocation set.

That is PB §6.2's own granularity, in its own words: *"A pragma counts only when a runner collected an id from **its file**"*. It requires no declaration-level parse — which the resolver deliberately cannot do (§1) — and it is total: the collector's `base` records carry `path` (`result-file.md` §4.4) and nothing else that could locate a line.

The consequence is that a pragma attributes to every test in its file, not to the test it sits above. That is coarser than a reader might expect and it is the right trade: G1's coverage clause asks whether an AC has *at least one* verifying collected id (PB §6.3), so coarseness can only make coverage easier to satisfy, never harder — and G5's orphan clause fails on a pragma naming a nonexistent AC, which is unaffected by granularity.

`attributed` follows PB §6.2 unchanged: true iff the pragma's line is in a blob the binding approval froze, or — before approval — the file is on the intent's own branch and under `C-T1`.

### 12.3 The naming sugar

PB §6.2: *"or a test name carrying `AC<n>` in its runner's conventional position (sugar, per-runner pattern in `docs/spec/`)"*.

The pattern is: the byte sequence `AC` followed by one or more digits, preceded by a byte outside `[A-Za-z0-9]` or at the start of the field, and followed by a byte outside `[0-9]` or at the end of the field. The capture is the digit run, and the intent is the branch's single gated intent (PB §4.3, "one gated intent per branch").

The **field** is per runner:

| Runner | Field |
|---|---|
| `pytest` | the final `::`-separated component of `fn`, with the parametrization suffix already removed |
| `vitest` | the final ` > `-separated component of `id` |
| `dart-test` | the bytes of `id` after the first `::` — the test's fully qualified name, group prefixes included |
| `swift-test` | the bytes of `id` after the `/` — the method name — **with a leading `test` removed if present**. XCTest discovers a method only if its name begins `test`, so removing that prefix is reading the runner's own convention: `testAC1TotalsIncludeTax` gives the field `AC1TotalsIncludeTax`, in which `AC1` is at the field's start; `test_AC1_totals` gives `_AC1_totals`, which yields the same edge. |

Several `AC<n>` matches in one field yield several edges. A match whose AC number has no corresponding AC in the intent is G5's finding, exactly as a typo'd pragma is.

The sugar is sugar: where a file carries both a pragma and a matching name, the edges are the union and no rule prefers one.

**Two consequences of keeping one pattern and varying only the field, stated rather than discovered.**

- **`swift-test` yields one edge where pytest yields two.** The pattern requires the byte before `AC` to be outside `[A-Za-z0-9]` or at the field's start, so a camelCase method naming two criteria — `testAC1AndAC2Totals` — gives AC-1 and **not** AC-2, because the second `AC` is preceded by `d`. `test_AC1_and_AC2` under pytest gives both, because `_` is outside the class. The pattern is deliberately not forked per runner: `§12.3` fixes the field per runner and the pattern for all of them, and a second spelling of "conventional position" is exactly the kind of per-runner divergence that would make two implementations derive different `verified_by` edges from one repository. The remedy where it matters is the pragma (§12.1), which is canonical, identical across languages, and always available.
- **`dart-test`'s field is the whole qualified name, so a group name counts.** `group('AC3 rounding'){ test('half even') }` gives the field `AC3 rounding half even` and yields AC-3 for every test in the group. That is coarser than a reader might expect and it is the same coarseness §12.2 already accepts for the file-granular pragma join, in the same direction: G1's coverage clause asks whether an AC has *at least one* verifying collected id, so extra edges can only make coverage easier to satisfy, never harder, and G5's orphan clause is unaffected because it fails on an AC number the intent does not have.

### 12.4 The `C-T3` predicate — framework specifiers and hook forms

PB §2.1's twelfth scaffolded rule, and PB §6.3's G8 row:

> `C-T3: no test-framework import or runner hook defined outside the harness (C-T1 u C-T2)`
>
> `C-T3` — no test-framework import or hook definition outside the harness (`C-T1` ∪ `C-T2`) — is a tree grep G8 runs, its finding a `G8:<path>` wire

This section is the two closed sets. It decides nothing about G8: the wire is `G8:` + `tok(path)`, its class is `protected`, it never runs in warn mode, and it is bypassable only by break-glass — all of which are PB §6.3's, `gate-report.md` §6.3's per-gate wire table and `constitution.md` §6.3's, and none of which changes here. What this section supplies is the predicate those rules evaluate.

**Where it runs, and over what.** G8 evaluates `C-T3` over the **synthetic merge `T`** — it is a property of the tree that would land, not of `A` or of `B`. `C-T1`, `C-T2` and `C-T3`'s own value are read from trunk like every other policy (PB §7.4 rule 1); `C-T3`'s v1 domain is the single token `on` (`constitution.md` §6.1), so the grep runs on every landing that runs G8. For a repository path `p` present in `T`:

> `ct3(p)` is **true** — a hit — iff `H(p)` is false (§2.3) and either (a) `lang(p) ∈ langs` and `p`'s bytes carry a framework import site by §12.4.1, or (b) `p`'s final path component is in §12.4.2's hook-basename set for some language in `langs`, or (c) `lang(p) ∈ langs` and `p`'s tokens carry a hook token sequence by §12.4.2.

One finding per hit path, whatever the number of sites in it — the wire set is per path (`gate-report.md` §6.1's `(gate, path)` uniqueness rule), exactly as it is for G5's per-pragma diagnostic.

Four rules make the predicate total.

- **The test is `H`, not `C-T1` alone**, which is the reading §16.10 already took for the unresolvable-import tripwire, before this clause had a predicate at all. **All three documents now write it that way**: PB §2.1's rule line and PB §6.3's G8 row read *"outside the harness (`C-T1` ∪ `C-T2`)"* and `constitution.md` §6.3's row reads the same, where earlier versions wrote *"outside test roots"* and *"outside `C-T1`"* — the rule's **name** is still *test roots*, because a name is not a predicate. Reading the rule over `C-T1` alone fails G8 on every repository this tool scaffolds, on its first landing. §5.5 renders `vitest.config.*` into **`C-T2`** and not into `C-T1`; a `vitest.config.ts` imports `defineConfig` from `vitest/config` because that is what the file is for; and it sits at the repository root, where `C-T1`'s `tests/`, `src/**/__tests__/` reaches nothing. `**/conftest.py` (§4.5) is the same shape and imports `pytest` by construction, and `dart_test.yaml` and `Package.swift` are `C-T2` for the same reason. The harness is `C-T1 ∪ C-T2` everywhere else in this document (§2.3) and it is `C-T1 ∪ C-T2` here. §17 D12 filed the wording and it is fixed in both documents; nothing in the predicate changed.
- **Type-only imports are not hits** (§3.6). A TypeScript `import type { Mock } from 'vitest'` is erased before anything runs, so it can monkeypatch nothing — §3.6's own reason, applied unchanged.
- **Disposition is irrelevant** (§3.2). A framework specifier is normally `external`, and the test is on the specifier's bytes rather than on where it resolves. A repository that vendors its runner into the tree, so that the same specifier resolves `repo(m)`, is a hit exactly as much: the file still reaches the framework.
- **A file the resolver cannot read is not a hit.** `lang(p) = none` (§3.1) is not scanned, and a file that is not valid UTF-8 raises `file-not-utf8` (§3.4 rule 1) and yields no `C-T3` finding. Both are residuals of the same shape §10 states for the closure, and §12.4.3 names them rather than leaving them to be discovered.

#### 12.4.1 Framework module specifiers, per language

The set is closed. Membership is tested on the specifier **as written**, after §3.4's lexing, at the granularity each language's own section already computes.

| Language | Tested on | Framework set |
|---|---|---|
| Python | the dotted name of §4.3's forms, reduced to its **longest matching dotted prefix** — `import pytest`, `import pytest.hookspec` and `from pytest import fixture` all reduce to `pytest` | `pytest`, `_pytest`, `unittest` — **less the single exemption `unittest.mock`** |
| TypeScript/JavaScript | a **bare** specifier (§5.2 step 4) reduced to its package name: the bytes before the first `/` for an unscoped name, the bytes before the second `/` for one beginning `@`, and the whole specifier for one beginning `node:` | `vitest`, `jest`, `chai`, `expect`, any package under the scope `@vitest/` or `@jest/`, and `node:test` |
| Dart | a `package:` URI reduced to its `<name>` (§6.2) | `test`, `test_api`, `test_core`, `matcher` |
| Swift | the module named by §7.2's forms | `XCTest`, `Testing` — **plus any `@testable import`, whatever module it names** |

**The set names the frameworks a v1 adapter drives, the internals those frameworks are built on, and the assertion libraries they route their `expect` through — and nothing else.** A framework with no v1 adapter cannot reach the runner that produces a `Spine-Test` id, so an import of `nose`, `mocha`, `jasmine` or `junit` outside the test roots is an unused dependency and not the hazard PB §4.3 names — *"an implementation that monkeypatches the assertion library"* means the library the landing's own G1 verdict is computed through. `jest` and `@jest/` are in the TypeScript set although §11.1 reserves `jest` rather than shipping it: the token is reserved precisely because the framework is real and common, and a repository that runs vitest under spine while importing jest's `expect` into production code is patching the same assertion engine by another door. Adding a language or an adapter adds rows here, and that is a release, not a repo setting (PB §6.7).

Three per-language notes, each of which an implementation would otherwise have to guess.

- **Python's one exemption is `unittest.mock`, by full dotted prefix.** It is the standard library's mocking module, ordinary production code imports it, and it can patch nothing about a runner it does not import. So `import unittest.mock`, `from unittest.mock import patch` and `from unittest import mock` are **not** hits, while `import unittest`, `from unittest import TestCase` and `from unittest.case import TestCase` are. `_pytest` is in the set because it is where pytest's assertion rewriting and its hook machinery actually live, and a file that imports it is reaching past the public surface. `doctest` is deliberately **out**: it is a standard-library module whose ordinary use — `doctest.testmod()` under a `__main__` guard — is not a test-framework reach, and pytest collects doctests only when a configuration file this document already freezes says so.
- **`node:test` is in the TypeScript set and the bare specifier `test` is not.** Node's built-in runner is reachable only as `node:test`; a bare `test` is an ordinary npm package name and matching it would fire on unrelated code. The `node:` prefix is what makes the built-in unambiguous, which is why the table tests the whole specifier for it.
- **Swift's `@testable` row is not about which module is imported.** §7.2 is right that `@testable import Foo` resolves exactly as `import Foo` — that is a statement about *resolution*. This is a statement about *what the file is*: `@testable` compiles only against a module built for testing, so its presence in a file the harness patterns do not cover says the file is part of the test build while claiming not to be. It is a hit whatever module follows it, `XCTest` or otherwise.

#### 12.4.2 Runner hook-definition forms, per language

A hook is a definition the runner discovers and executes **without any file importing it**. That is the whole of the second clause's hazard, and the reason it is separate from the first: an import is visible in the file that makes it and §12.4.1 catches it, while an auto-discovered hook is reachable from nowhere in the source and runs anyway. There are two kinds — a **basename** the runner loads by convention, and a **token sequence** the runner discovers by name.

| Language | Hook basenames — loaded with no import | Hook token sequences (§3.4 tokens, in order, `comment` discarded) |
|---|---|---|
| Python | `conftest.py` | `def` followed by a `word` beginning `pytest_`; `async` `def` followed by a `word` beginning `pytest_`; the `word` `pytest_plugins` followed by the `punct` `=` |
| TypeScript/JavaScript | `vitest.config.` / `vitest.workspace.` / `vite.config.` / `jest.config.` each followed by one of `ts`, `mts`, `cts`, `js`, `mjs`, `cjs`; and `jest.config.json` | none |
| Dart | `dart_test.yaml` | none |
| Swift | none | none |

Four notes, and the empty cells are the ones that need them most.

- **Python's is the only non-empty token set, and §17 D1 is why.** pytest auto-loads every `conftest.py` from the rootdir down with no import statement anywhere, discovers every hook in it by the `pytest_` name prefix, and a `pytest_collection_modifyitems` in one deselects every test below it. The prefix is pytest's own closed convention — every hook it calls is spelled `pytest_*` — so a module-level `def pytest_anything(` in a non-harness file is either a hook or a name chosen to look like one. `pytest_plugins` is on the list because it is the one *assignment* that loads a hook module by name rather than defining a hook in place. Nesting is irrelevant here as it is everywhere else in this document (§3.4 rule 7): a `def pytest_…` inside a class or a function is still a definition pytest's plugin manager can find once the module is loaded, and a rule that ignored one would be a rule to hide behind.
- **A basename is a hit wherever `H` is false, at any depth, read from the path and never from the content.** This is §2.7's rule for §2.7's reason: a content test would put a TOML/JSON/YAML parser between two implementations that must agree to the byte, and would let a branch change the answer by adding a section.
- **How much the basename column bites depends on `C-T2`, and the scaffolded values differ between languages.** `spine init` renders `**/conftest.py` into Python's `C-T2` (§4.5), which matches a `conftest.py` at any depth — so under the untouched scaffold **no** `conftest.py` is ever a `C-T3` hit, and the hazard is carried instead by `H`: the file is harness, it is read-only from the branch after approval, and a change to it before approval is a `class=protected` `G8:<path>` review (PB §4.3). That is the stronger treatment, not a gap. The column bites where the scaffolded pattern is **root-anchored**: §5.5 renders `vite.config.*` and `vitest.config.*` and §6.5 renders `dart_test.yaml`, and a pattern containing no `/` matches only at the repository root (§2.4.2). A monorepo's `packages/a/vitest.config.ts`, a `packages/a/vite.config.ts` or a nested `test/dart_test.yaml` is therefore outside `H`, is loaded by the runner with no import anywhere, and is exactly what this column is for. **A *root* `vite.config.ts` is not a hit**, because §5.5 renders `vite.config.*` into `C-T2` for exactly this reason: every basename in this column has a scaffolded `C-T2` pattern covering its root instance, and one that did not would fail G8 on the first landing of every repository laying the file out the ordinary way. It also bites in any repository that edited the scaffolded `C-T2` — the same case §2.7 already handles for clause 3.
- **Dart's and Swift's token sets are empty, and TypeScript's is empty for the opposite reason.** `package:test`'s `setUp`, `tearDown`, `setUpAll` and `tearDownAll` are ordinary functions a file must import from `package:test`, and XCTest's `setUp`/`tearDown`/`XCTestObservation` are members of types that require `import XCTest` — so §12.4.1 already catches every file that could define one, and SwiftPM auto-loads no test configuration file at all. Adding `setUp`-shaped word matches would buy nothing there and would fire on ordinary code, since `setUp(` is not a rare byte sequence. TypeScript's set is empty from the other side: vitest discovers hooks **only** through its configuration's `setupFiles` and `globalSetup`, and those are named by a config file the basename column already catches. In all three cases the empty cell is a result, not an omission.

#### 12.4.3 What the predicate does not catch

Stated at true size, which is PB §7.4's own habit and the reason that section is quotable.

- **A dynamic reach for the framework is not a hit.** `importlib.import_module("pytest")` and `require(expr)` are `unresolvable` sites (§4.3, §5.2) whose argument this document never evaluates, so the framework's name is not in the tested bytes. Inside the harness such a site is already `unresolvable-import`, a tripwire; outside it, it is the counter `unresolvable-import-outside-harness` and nothing more. Closing it would mean deciding what a string evaluates to, which §1 refuses in the same breath for the same reason.
- **A file whose `lang` is `none` is not scanned at all** — `.java`, `.m`, `.go`, `.vue`, `.svelte`. This is §10's residual in a second place rather than a new one: the same files that can hold an oracle the closure cannot see can hold a framework import the grep cannot see.
- **A monkeypatch that needs no framework import is untouched.** A `sys.modules` entry, a patched `builtins`, a `conftest.py` *inside* the test roots — which is harness, is frozen, and is G8's blob-equality clause's business rather than this one's. `C-T3` is one of the five bar-raisers PB §7.4 lists, and PB §7.4 is explicit that **none of them closes the property**: *"G1's `passed` is therefore exactly as strong as the isolation between a candidate's code and its own runner, and nothing in this design establishes that property."* What this section changes is that the bar-raiser now has a predicate two implementations compute identically, which is the difference between a boundary and a sentence about one.

---

## 13. Worked examples

Each example gives a base tree `B`, an approval tree `A`, the constitution's `C-T1`/`C-T2`, the intent's `expected`, the walk, the closure, and the `closure_digest` of §2.10 — **computed**, not asserted. The intent is `INT-042` in all four, with acceptance criteria `AC-1 … AC-3`, and each seed is **derived** by §2.1.1 from the `@verifies` pragma shown in that file's own bytes rather than supplied: a pragma is a comment, contributes no edge (§3.4 rule 4), and moves no published `closure_digest`. There are four, one per v1 language. Every path in these examples is ASCII with no backslash, so `esc` is the identity on all of them, and all four `closure_digest` values were recomputed under §2.4's dialect and are unchanged from version 1.

### 13.1 Python

`C-T1`: `tests/`. `C-T2`: §4.5's list. `expected`: `src/billing/`. Seed, derived by §2.1.1: `tests/billing/test_invoice.py` — the one path a `C-T1` pattern matches whose bytes carry a pragma naming an AC of `INT-042`.

`B`:

```
pyproject.toml
conftest.py
src/__init__.py
src/api.py                       from src.billing.invoice import total
src/billing/__init__.py
src/billing/invoice.py           from src.shared.money import Money
                                 from src.billing.rates import rate
src/billing/rates.py
src/billing/oracle.py            (no importer)
src/shared/__init__.py
src/shared/money.py
tests/__init__.py
tests/conftest.py
tests/billing/__init__.py
tests/billing/test_invoice.py
tests/support/__init__.py
tests/support/factories.py       from src.shared.money import Money
```

`A` = `B` plus `src/billing/stub.py`, and `tests/billing/test_invoice.py` reads:

```python
# @verifies INT-042/AC-1
from src.billing.invoice import total
from src.billing.oracle import expected_total
from src.billing.stub import newthing
from tests.support.factories import make_invoice
import json
```

The walk:

| Site | Targets | `H` | `E` | in `B` | non-test importer in `B` | class |
|---|---|:--:|:--:|:--:|:--:|---|
| seed | `tests/billing/test_invoice.py` | y | n | y | — | `FROZEN_WALK` |
| `src.billing.invoice` | `src/__init__.py` | n | n | y | y | `FROZEN_LEAF` |
| | `src/billing/__init__.py` | n | y | y | y (`src/api.py`) | `EXCLUDED` |
| | `src/billing/invoice.py` | n | y | y | y (`src/api.py`) | `EXCLUDED` — prune, so `rates.py` and `money.py` are never reached this way |
| `src.billing.oracle` | `src/billing/oracle.py` | n | y | y | **n** | `FROZEN_LEAF` |
| `src.billing.stub` | `src/billing/stub.py` | n | y | **n** | — | `EXCLUDED` + **closure tripwire** |
| `tests.support.factories` | `tests/__init__.py`, `tests/support/__init__.py`, `tests/support/factories.py` | y | n | y | — | `FROZEN_WALK` |
| ↳ `src.shared.money` | `src/__init__.py`, `src/shared/__init__.py`, `src/shared/money.py` | n | n | y | y | `FROZEN_LEAF` |
| `json` | — | — | — | — | — | `external` |

Clause 3 adds `pyproject.toml`, `conftest.py`, `tests/conftest.py`, `tests/__init__.py`, `tests/billing/__init__.py`. Clause 4 adds nothing.

Closure — 12 paths:

```
conftest.py
pyproject.toml
src/__init__.py
src/billing/oracle.py
src/shared/__init__.py
src/shared/money.py
tests/__init__.py
tests/billing/__init__.py
tests/billing/test_invoice.py
tests/conftest.py
tests/support/__init__.py
tests/support/factories.py
```

`closure_digest` = `sha256:c17cb077493566e549417309f2448343c60259b5621ae8282ca06427831b0ea6` (over 278 canonical bytes).

Three things this example is here to show. `src/billing/oracle.py` is the case PB §4.3 designed clause 2 for — inside `expected`, at `base`, never imported by trunk's own non-test code, so it freezes as a leaf rather than passing as code under test. `src/billing/stub.py` is the branch-created module the tripwire names rather than freezes. And `src/__init__.py` freezes even though nothing about it is a test, because it is outside `expected` and outside the harness: *"A had no business touching it."*

### 13.2 TypeScript

`C-T1`: `tests/`, `src/**/__tests__/`. `C-T2`: §5.5's list. `expected`: `src/billing/`. Seed, derived by §2.1.1: `tests/billing/invoice.test.ts`. Note that `tests/billing/helper.ts` is under `C-T1` too and carries no pragma, so it is not a seed — and is in the closure anyway, reached by an import from the one that is.

`B` = `A` in this example:

```
package.json
tsconfig.json            {"compilerOptions":{"baseUrl":".","paths":{"@shared/*":["src/shared/*"]}}}
vitest.config.ts         import './vitest.setup.ts'
vitest.setup.ts
src/api.ts               import { total } from './billing/invoice'
src/billing/index.ts     export * from './invoice'
src/billing/invoice.ts   import { Money } from '@shared/money'; import { rate } from './rates'
src/billing/rates.ts
src/shared/money.ts
tests/billing/invoice.test.ts
tests/billing/helper.ts
tests/billing/__snapshots__/invoice.test.ts.snap
tests/fixtures/invoices.json
tests/support/factories.ts        (no repo-local imports)
```

`tests/billing/invoice.test.ts`:

```ts
// @verifies INT-042/AC-1
import { total } from '../../src/billing';
import { total as t2 } from '../../src/billing/invoice';
import type { Money } from '@shared/money';
import { makeInvoice } from '../support/factories';
import invoices from '../fixtures/invoices.json';
const { helper } = await import('./helper.js');
```

The walk:

| Site | Target | class | why |
|---|---|---|---|
| `'../../src/billing'` | `src/billing/index.ts` (directory → index) | `FROZEN_LEAF` | inside `expected`, at `base`, and no non-test file imports the barrel — `src/api.ts` imports `./billing/invoice` directly. Prune: the `export * from './invoice'` re-export is **not** followed |
| `'../../src/billing/invoice'` | `src/billing/invoice.ts` | `EXCLUDED` | inside `expected`, at `base`, imported by `src/api.ts`. Prune: `rates.ts` and `money.ts` are not reached |
| `import type … '@shared/money'` | — | — | `type_only`: **`src/shared/money.ts` is not frozen** |
| `'../support/factories'` | `tests/support/factories.ts` | `FROZEN_WALK` | harness |
| `'../fixtures/invoices.json'` | `tests/fixtures/invoices.json` | `FROZEN_LEAF` | outside `expected` and outside the harness patterns |
| `await import('./helper.js')` | `tests/billing/helper.ts` | `FROZEN_WALK` | literal dynamic import; `.js` → `.ts` rewrite, §5.2 step 5.2 |

Clause 3 adds `package.json`, `tsconfig.json`, `vitest.config.ts`, `vitest.setup.ts`. `vitest.setup.ts` would also arrive by the walk, since `vitest.config.ts` satisfies `H` and is therefore `FROZEN_WALK`. Clause 4 adds `tests/billing/__snapshots__/invoice.test.ts.snap`.

Closure — 10 paths:

```
package.json
src/billing/index.ts
tests/billing/__snapshots__/invoice.test.ts.snap
tests/billing/helper.ts
tests/billing/invoice.test.ts
tests/fixtures/invoices.json
tests/support/factories.ts
tsconfig.json
vitest.config.ts
vitest.setup.ts
```

`closure_digest` = `sha256:da93556c4c3bdb8abfb29c75f3a03a5ae9d3396d96e99bb08dc4172be62070c8` (over 261 canonical bytes).

This is the example where `src/shared/money.ts` is absent and the Python example's equivalent is present: the difference is the `import type`, and it is the whole of §3.6 made visible. It also shows PB §4.3's own `Spine-Frozen: 58d2… tests/fixtures/invoices.json` line arriving by a real import edge rather than by a special rule for fixtures (§16.9).

### 13.3 Dart

`C-T1`: `test/`. `C-T2`: §6.5's list. `expected`: `lib/src/`. Seed, derived by §2.1.1: `test/billing/invoice_test.dart`. `pubspec.yaml` declares `name: billing`.

`B` = `A`:

```
pubspec.yaml
dart_test.yaml
bin/app.dart                      import 'package:billing/src/invoice.dart';
lib/src/invoice.dart              import 'package:billing/src/money.dart';
lib/src/money.dart
lib/src/oracle.dart               (no importer)
test/billing/invoice_test.dart
test/billing/conditional_io.dart
test/billing/conditional_stub.dart
test/support/index.dart           export 'factories.dart';
test/support/factories.dart       part 'factories_data.dart';
test/support/factories_data.dart  part of 'factories.dart';
```

`test/billing/invoice_test.dart`:

```dart
// @verifies INT-042/AC-1
import 'package:billing/src/invoice.dart';
import 'package:billing/src/oracle.dart';
import '../support/index.dart';
import 'dart:convert';
import 'conditional_stub.dart' if (dart.library.io) 'conditional_io.dart';
```

| Site | Target | class |
|---|---|---|
| `package:billing/src/invoice.dart` | `lib/src/invoice.dart` | `EXCLUDED` — inside `expected`, at `base`, imported by `bin/app.dart`; prune, so `money.dart` is never reached |
| `package:billing/src/oracle.dart` | `lib/src/oracle.dart` | `FROZEN_LEAF` — inside `expected`, at `base`, no non-test importer |
| `'../support/index.dart'` | `test/support/index.dart` | `FROZEN_WALK` |
| ↳ `export 'factories.dart'` | `test/support/factories.dart` | `FROZEN_WALK` — a re-export is an import (§3.5) |
| ↳ `part 'factories_data.dart'` | `test/support/factories_data.dart` | `FROZEN_WALK` |
| `dart:convert` | — | `external` |
| conditional import | **both** `test/billing/conditional_stub.dart` and `test/billing/conditional_io.dart` | `FROZEN_WALK` — §3.7's union rule |

Clause 3 adds `pubspec.yaml` and `dart_test.yaml`. Clause 4 adds nothing.

Closure — 9 paths:

```
dart_test.yaml
lib/src/oracle.dart
pubspec.yaml
test/billing/conditional_io.dart
test/billing/conditional_stub.dart
test/billing/invoice_test.dart
test/support/factories.dart
test/support/factories_data.dart
test/support/index.dart
```

`closure_digest` = `sha256:cd83d5c6267e9abd5a72878d9a103765ceb0342bc931ea3f0a07d7b418c06954` (over 251 canonical bytes).

### 13.4 Swift

`C-T1`: `Tests/`. `C-T2`: §7.6's list. `expected`: `Sources/Billing/`. Seed, derived by §2.1.1: `Tests/BillingTests/InvoiceTests.swift`, whose first line is `// @verifies INT-042/AC-1`. `Factories.swift` carries no pragma and is not a seed; the implicit same-module edge reaches it regardless (§7.4).

`Package.swift` (inside the literal subset): targets `.target(name: "Shared")`, `.target(name: "Billing", dependencies: ["Shared"])`, `.testTarget(name: "BillingTests", dependencies: ["Billing"])`.

`B` = `A`:

```
Package.swift
Sources/Billing/Invoice.swift
Sources/Billing/Rates.swift
Sources/Shared/Money.swift
Tests/BillingTests/InvoiceTests.swift    // @verifies INT-042/AC-1
                                         @testable import Billing; import XCTest
                                         #if os(Linux) import Glibc #endif
Tests/BillingTests/Factories.swift       import Shared
Tests/BillingTests/__Snapshots__/Invoice.snap
```

| Edge | Target | class |
|---|---|---|
| implicit same-module (`BillingTests`) | `Tests/BillingTests/Factories.swift` | `FROZEN_WALK` |
| `@testable import Billing` | `Sources/Billing/Invoice.swift` | `EXCLUDED` — inside `expected`, at `base`, and `Rates.swift` (a non-test file of the same module) implicitly imports it |
| | `Sources/Billing/Rates.swift` | `EXCLUDED`, symmetrically |
| `import XCTest`, `import Glibc` | — | `external` (both branches of the `#if` are taken, §3.7) |
| ↳ `Factories.swift`: `import Shared` | `Sources/Shared/Money.swift` | `FROZEN_LEAF` — outside `expected`, outside the harness |

Clause 3 adds `Package.swift` (no `Package.resolved` in this tree). Clause 4 adds `Tests/BillingTests/__Snapshots__/Invoice.snap`.

Closure — 5 paths:

```
Package.swift
Sources/Shared/Money.swift
Tests/BillingTests/Factories.swift
Tests/BillingTests/InvoiceTests.swift
Tests/BillingTests/__Snapshots__/Invoice.snap
```

`closure_digest` = `sha256:8a2d5fbc97efdaf17467daba9f2836caaca14da5424bc5aec7c55117f9d66eff` (over 171 canonical bytes).

`Factories.swift` is in the closure with no import line anywhere pointing at it: the implicit same-module edge of §7.4 is the only thing that reaches it, and without that rule an oracle in that file would be invisible. Had the branch added `Sources/Billing/Oracle.swift`, it would have been reached by `@testable import Billing`, found absent from `B`, and named by the closure tripwire — which is the case §2.5's per-file classification exists for.

---

## 14. Conformance test cases

An implementer runs these against their resolver and its adapters. Each states a tree fragment, an input, and the required answer. Cases C1–C30 are language-independent, the T cases are `C-T3`'s predicate (§12.4), the R cases are the runner adapters of §11, and the rest are per language. A case marked **must not** describes an answer a plausible implementation gives and that is wrong.

**Shared — the closure algorithm**

| # | Case | Required |
|---|---|---|
| C1 | A seed imports a module inside `expected`, present at `B`, imported at `B` only by another test file | `FROZEN_LEAF`, not `EXCLUDED`. "Non-test" is `H`-false, and a test file is not one |
| C2 | A seed imports a module inside `expected`, absent from `B` | `EXCLUDED`, and `closure-tripwire` fires naming it |
| C3 | A seed imports a module outside `expected` and outside the harness, which itself imports ten more | exactly the one module in the closure; the ten are **not** reached (leaf pruning) |
| C4 | A seed imports a module that is `EXCLUDED`, which imports a harness file | the harness file is **not** in the closure. The walk prunes at an excluded import |
| C5 | Two seeds reach the same module by different routes | one entry; the result is a set |
| C6 | An import cycle among three harness files | terminates; all three frozen once each |
| C7 | A module matching both a `C-T1` pattern and an `expected` entry | `FROZEN_WALK`. `H` wins over `E`; runner-config patterns match inside `expected` |
| C8 | 201 distinct paths in the closure | `closure-too-large` fires; at 200 it does not |
| C9 | An `expected` entry equal to a `C-T2` pattern | `expected-hits-harness` fires |
| C10 | A file carrying a pragma naming an AC of this intent that no `C-T1` pattern matches — whether outside the harness or matched only by `C-T2` | `seed-outside-test-roots`; `--approve` refuses outright, not a tripwire. It is not a seed and the closure does not start from it |
| C11 | A snapshot under a test root that no seed reaches | in the closure (clause 4 is not reachability-based) |
| C12 | A `conftest.py` two directories above the seed | in the closure (clause 3) |
| C13 | Walk order reversed | byte-identical closure |
| C14 | The same tree with a dirty working directory, or in a bare clone | byte-identical closure |
| C15 | A resolved candidate whose tree entry has mode `120000` | `unresolvable`, reason `symlink-or-submodule`; **must not** follow the link |
| C16 | A resolved candidate under a `160000` entry | `unresolvable`; **must not** descend into the submodule |
| C17 | A file that is not valid UTF-8, under a test root | `file-not-utf8` tripwire; no edges; **must not** fall back to latin-1 or to a coding declaration |
| C18 | `RC(lang, A) ≠ RC(lang, B)` | `lang-unclassifiable`, reason `rc-changed-on-branch`; the closure is computed with `RC(lang, B)` |
| C19 | A pattern `tests/` and a path `testsuite/x.py` | no match. Trailing-`/` is a directory prefix, not a string prefix |
| C20 | A pattern `**/conftest.py` and a path `conftest.py` at the root | match. `**` matches zero segments |
| C21 | The shipped `C-T1` value `src/**/__tests__/` and a path `src/billing/__tests__/x.test.ts` | **match** (§2.4.2). **Must not** be read as a raw byte prefix, under which it matches nothing and the closure is empty |
| C22 | A `C-T2` pattern `src/[abc]*.ts` | legal — a bracket expression (`intent-doc.md` §6.2). **Must not** be refused as an illegal byte |
| C23 | A `C-T2` pattern `src/**.ts` | refused, `bad-globstar`; the constitution does not parse and G16 finds it before any closure is computed |
| C24 | A file under `C-T1` carrying `@verifies INT-042/AC-1`, the intent being `INT-042` with three ACs | a seed |
| C25 | The same file carrying only `@verifies INT-042/AC-9` | **not** a seed, and G5's orphan finding. **Must not** be silently dropped: §12.1 recognizes the occurrence so that it can be reported |
| C26 | The same file carrying only `@verifies INT-41/AC-1` | not an occurrence at all — `intent-doc.md` §3.1's padding rule — so not a seed and not an orphan. **Must not** be admitted by a looser id grammar that accepts an unpadded numeral (version 2's) |
| C27 | A file under `C-T1` named `test_AC1_totals.py`, carrying no pragma | **not** a seed (§12.3 sugar seeds nothing, §16.12). It is still in the closure if another seed imports it, and its own bytes are still harness |
| C28 | A tree under `C-T1` in which no file carries a pragma for this intent | `S = ∅`, closure `= ∅`, the `no-seed` tripwire; **must not** fall back to "every file under `C-T1`" |
| C29 | A pragma in `A` whose file no runner would collect | still a seed. Collection is not read anywhere in §2 |
| C30 | The `@verifies` text inside a Python docstring, or inside a string literal in any language | not an occurrence (§12.1); not a seed |

**Shared — the `C-T3` predicate (§12.4)**

| # | Case | Required |
|---|---|---|
| T1 | `import pytest` in `src/billing/invoice.py` | a hit; wire `G8:src/billing/invoice.py` |
| T2 | `import pytest` in `tests/conftest.py`, `C-T1` being `tests/` | no hit — `H` is true |
| T3 | `from vitest import …`-equivalent: `import { vi } from 'vitest'` in the repository-root `vitest.config.ts`, which `C-T2` matches and `C-T1` does not | **no hit.** Evaluating the rule over `C-T1` alone here fails G8 on `spine init`'s own scaffold (§12.4, §17 D12) |
| T4 | `import type { Mock } from 'vitest'` in `src/api.ts` | no hit (§3.6) |
| T5 | `from unittest.mock import patch` in `src/api.py` | no hit — the one Python exemption |
| T6 | `from unittest import TestCase` in `src/api.py` | a hit |
| T7 | `import nose` in `src/api.py` | no hit — no v1 adapter drives it (§12.4.1) |
| T8 | `packages/a/vitest.config.ts`, `C-T2` carrying the scaffolded root-anchored `vitest.config.*` | a hit, by basename; **must not** be read for content first |
| T8a | a repository-root `vite.config.ts` under the untouched scaffolded `C-T2` of §5.5 | **no** hit — `vite.config.*` is on that list, so `H` is true. An implementation whose `C-T2` omits it raises `G8:vite.config.ts` on every landing, which is the defect §5.5's note names |
| T8b | `src/billing/conftest.py`, `C-T2` carrying the scaffolded `**/conftest.py` | **no** hit — `H` is true, and G8's blob clause governs the file instead. The same path in a repository that removed `**/conftest.py` from `C-T2` **is** a hit |
| T9 | `def pytest_collection_modifyitems(items):` in `src/plugins.py` | a hit |
| T10 | `func setUp()` in `Sources/Billing/Helper.swift` with no `import XCTest` | no hit — Swift's token set is empty and the import clause is the whole predicate there |
| T11 | `@testable import Billing` in `Sources/Billing/Debug.swift` | a hit, whatever module follows |
| T12 | `importlib.import_module("pytest")` in `src/api.py` | no hit (§12.4.3); it is an `unresolvable` site and a counter |
| T13 | A `.java` file importing `org.junit.Test` | no hit — `lang: none`, never scanned (§12.4.3) |
| T14 | Two framework imports in one non-harness file | **one** finding and one wire; the wire set is per path |

**Python**

| # | Case | Required |
|---|---|---|
| P1 | `from a.b import c` where both `a/b/c.py` and `a/b/__init__.py` exist | targets are both, plus `a/__init__.py` if present |
| P2 | `from a.b import c` where `a/b/c.py` does not exist but `a/b.py` does | target is `a/b.py`; `c` is an attribute |
| P3 | `import a.b.c` | targets `a/__init__.py`, `a/b/__init__.py`, `a/b/c.py` — every existing ancestor package, **must not** be only the leaf |
| P4 | `a/b.py` and `a/b/__init__.py` both exist, `import a.b` | `unresolvable`, reason `ambiguous-module` |
| P5 | `src/pkg/mod.py` exists, no top-level `pkg/`, `import pkg.mod` | resolves under root 2 (`src/`) |
| P6 | Both `pkg/mod.py` and `src/pkg/mod.py` exist, `import pkg.mod` | root 1 wins; `src/pkg/mod.py` is not a target |
| P7 | `from ... import x` in `a/b/c.py` | base directory `` (root); resolves. In `a/c.py` it escapes → `unresolvable` |
| P8 | An import nested inside a function body | an import site; **must not** be ignored |
| P9 | An import under `if TYPE_CHECKING:` | an ordinary import site (§3.6) |
| P10 | `importlib.import_module("a.b")` with a literal argument | `unresolvable`, reason `dynamic-import`; **must not** resolve the literal |
| P11 | `import os; import a.b` on one line | two sites |
| P12 | `from a import (\n  b,\n  c,\n)` | one site; the logical-line rule spans the parentheses |
| P13 | `# import a.b` | no site |
| P14 | `x = "import a.b"` | no site |
| P15 | A `.pyi` file next to a `.py` | `.pyi` is `lang: none`; never a target, never lexed |

**TypeScript/JavaScript**

| # | Case | Required |
|---|---|---|
| T1 | `import './x.js'` where `x.ts` exists and `x.js` does not | resolves to `x.ts` |
| T2 | `import './x'` where both `x.ts` and `x.js` exist | `x.ts` (extension order) |
| T3 | `import './dir'` where `dir/index.ts` exists | resolves to `dir/index.ts` |
| T4 | `import './dir'` where `dir/package.json` has `"main":"lib.js"` and no `index.*` exists | `unresolvable`, reason `no-candidate`; **must not** read `main` |
| T5 | `import type { A } from './x'` | `type_only`; `x.ts` is not frozen |
| T6 | `import { type A, b } from './x'` | an ordinary import site — `b` is a value |
| T7 | `export * from './x'` | an import site (re-export) |
| T8 | `export type * from './x'` | `type_only` |
| T9 | `await import('./x')` | an import site, resolved |
| T10 | `await import(name)` | `unresolvable`, reason `dynamic-import` |
| T11 | `require('./x')` in a `.cjs` file | an import site |
| T12 | `require.resolve('./x')` | **not** an import site |
| T13 | `import.meta.url` | **not** an import site |
| T14 | `@shared/money` with `paths: {"@shared/*": ["src/shared/*"]}` and `baseUrl: "."` | resolves to `src/shared/money.ts` |
| T15 | `@shared/nope` with the same alias and no such file | `unresolvable`, reason `alias-dead-end` — **not** `external` |
| T16 | `lodash` with no matching alias | `external` |
| T17 | `#internal/x` | `unresolvable`, reason `subpath-imports` |
| T18 | `import './x.json'` where `x.json` exists | an import site resolving to the JSON file |
| T19 | A specifier resolving only to `x.d.ts` | `type_only` |
| T20 | `` import `./x` `` (template literal, no substitution) | `unresolvable` — not a simple literal (§3.4 rule 5) |
| T21 | `extends: "@company/tsconfig"` in the root tsconfig | `lang-unclassifiable`, reason `tsconfig-extends-external` |
| T22 | A `//` inside a regex literal, followed on a later line by a real import | the import is found; the regex did not open a comment |

**Dart**

| # | Case | Required |
|---|---|---|
| D1 | `import 'package:self/x.dart'` where `pubspec.yaml` says `name: self` | resolves to `lib/x.dart` |
| D2 | `import 'package:other/x.dart'` with no `path:` dependency on `other` | `external` |
| D3 | `import 'package:other/x.dart'` with `other: {path: ../other}` inside the repo | resolves to `../other/lib/x.dart`, normalized |
| D4 | `import 'a' if (dart.library.io) 'b';` | **both** URIs are sites |
| D5 | `export 'x.dart' show Y;` | an import site |
| D6 | `part 'x.dart';` | an import site, and `x.dart` is walked |
| D7 | `part of 'x.dart';` | an import site naming `x.dart` |
| D8 | `part of my.lib;` with exactly one `library my.lib;` in the tree | resolves to that file |
| D9 | `part of my.lib;` with two such files | `unresolvable`, reason `ambiguous-library-name` |
| D10 | `import 'x';` (no `.dart`) where `x.dart` exists | `unresolvable`, reason `no-candidate` — Dart does not append extensions |
| D11 | A nested block comment `/* /* */ */` followed by an import | the import is found |
| D12 | `import 'dart:async';` | `external` |
| D13 | `pubspec.yaml` using a YAML anchor | `lang-unclassifiable`, reason `pubspec-not-declarative` |

**Swift**

| # | Case | Required |
|---|---|---|
| S1 | `@testable import M` | identical to `import M` |
| S2 | `import struct M.T` | module `M` |
| S3 | Two files in one target, neither importing the other | each is an edge target of the other (implicit same-module) |
| S4 | `#if os(Linux) import A #else import B #endif` | **both** `A` and `B` are sites |
| S5 | `import Foundation` with no target of that name | `external` |
| S6 | A branch-added file inside an existing target inside `expected` | `EXCLUDED` **and** `closure-tripwire` — **must not** be treated as "the module existed at base" |
| S7 | `Package.swift` with `targets: buildTargets()` | `lang-unclassifiable`, reason `manifest-not-literal` |
| S8 | `Package.swift` with `path: "Custom/Dir"` (a literal) | honoured |
| S9 | A target with `exclude: ["Legacy"]` | files under `Legacy` are not target sources |
| S10 | Two targets whose source globs overlap | `lang-unclassifiable`, reason `overlapping-targets` |
| S11 | A repository with `.xcodeproj` and no `Package.swift` | `lang-unclassifiable`, reason `xcode-project-unsupported` |
| S12 | A target whose file set contains `Sources/Billing/Legacy.m` | `lang-unclassifiable`, reason `mixed-objc-target` (§7.3). Every Swift file contributes no edges and the closure holds only its clause-3 and clause-4 members. **Must not** be "the `.m` is `lang: none` and contributes nothing" — that answer is the silent hole §7.3 closes |
| S13 | A target every entry of whose file set ends in `.swift`, in a repository whose other targets are the same | **no** `mixed-objc-target`; resolution proceeds by §7.4 and the closure is computed normally. The refusal fires on a C-family entry or construct and on nothing else |
| S14 | A target whose file set contains `Tests/BillingTests/BillingTests-Bridging-Header.h` | `mixed-objc-target`, by the `.h` extension alone. **Must not** require the stem to end in `-Bridging-Header`: no rule in §7.3 reads a filename stem |
| S15 | A target with `exclude: ["Legacy"]` whose only C-family entry is `Legacy/Old.m` | **no** `mixed-objc-target`. `F(t)` is post-`exclude:`, and a file compiled into no target is reachable from no Swift file |
| S16 | A `.systemLibrary` target; or any target whose call carries `publicHeadersPath: "include"`, whatever that directory holds | `mixed-objc-target`, by test 2. **Must not** require a header to exist on disk first — presence of the label is the test |
| S17 | A package whose only C-family target is pure Objective-C — no `.swift` entry anywhere in its file set — which a Swift target `import`s by name | `mixed-objc-target`. **Must not** be narrowed to targets that also contain Swift: `import CBits` over a target with no Swift source yields zero edges and no finding, which is the miss itself |
| S18 | `B` pure Swift; the branch adds `Sources/Billing/Oracle.m` to an existing target and edits no manifest | `mixed-objc-target`, from the test over `A`. **Must not** be `rc-changed-on-branch` — `RC(swift, A) = RC(swift, B)`, the manifest did not move — and **must not** pass because `RC` is read from `B` |
| S19 | `swiftSettings: [.unsafeFlags(["-import-objc-header", "Shim.h"])]` on a target whose file set holds no C-family entry | `mixed-objc-target`, by test 2's string-literal clause |

**The runner adapters (§11)**

| # | Case | Required |
|---|---|---|
| R1 | A `dart-test` suite `test/a_test.dart` with `group('g'){test('t')}` | one id, `test/a_test.dart::g t` — the group prefix comes from `test.name`; **must not** be re-joined by the adapter |
| R2 | A `dart-test` test reported `{"result":"success","skipped":true}` | `out` is `skipped`; **must not** be `passed` |
| R3 | A `dart-test` suite whose root `group` declares `"testCount":5` and which yields 4 records | `runner-failed` on `T`, `base-collect-failed` on `B` |
| R4 | A `dart-test` `testDone` for a `loading <path>` pseudo-test with `hidden` `true` | no record, in either section |
| R5 | The same with `hidden` `false`, during the `B` collection | `base-collect-failed`; and by `result-file.md` §7.3, no records from any runner |
| R6 | A `dart-test` test named `outer (tearDownAll)` | a `result` record; **no** `base` record |
| R7 | Two `test('x')` calls with the same name in one `dart-test` suite | `duplicate-test-id`; the collector fails the job and writes nothing |
| R8 | A `dart-test` stream line that is a JSON `testDone` whose `testID` had no `testStart` | discarded; **must not** yield a record |
| R9 | A repository path containing `::`, under `pytest` or `dart-test` | `id-separator-in-path`; the collector fails the job and writes nothing |
| R10 | A `swift-test` id `M.C/testX` where two files of target `M` both declare `class C` | `path` is the empty string; **must not** pick either |
| R11 | A `swift-test` case with an `Expected failure in …` line and a `passed` verb | `out` is `xfail`; **must not** be `passed` |
| R12 | A `swift-test` case with two terminal lines in one run | `out` is `unknown` |
| R13 | `swift test list --enable-swift-testing --disable-xctest` yields any line | `swift-testing-unsupported`; the collector fails the job and writes nothing |
| R13a | The `B` **enumeration** and the `B` **outcome run** for `swift` | `swift test list --disable-swift-testing` and `swift test --disable-swift-testing` respectively, both at the repository root on the checkout of `B` (§11.5) |
| R13b | A `swift-test` case in the `B` listing with no terminal line in the `B` outcome run | `base.out` is `absent`; the id **stays** in the floor. **Must not** be dropped from it, and **must not** be read as `xfail` |
| R13c | A `swift-test` case with an `Expected failure in …` line in the **`B`** run | `base.out` is `xfail`, by the same mapping the `T` stream uses (§11.6 rule 4) |
| R14 | Two `swift-test` ids sharing `(class-path, method)` under different targets | `ambiguous-test-class`; the collector fails the job and writes nothing |
| R15 | Any adapter's `fn` | a prefix of its `id`; equal to it for `vitest`, `dart-test` and `swift-test` |
| R16 | The `B` **enumeration** for `python` | `pytest --collect-only`, at the repository root, through the `T` transport |
| R16a | The `B` **outcome run** for `python` | `pytest` — the `T` invocation, against the checkout of `B`, no selection argument. **Must not** be omitted: without it every `base` record's `out` is unobtainable and PB §6.3 G1's second exemption cannot be evaluated (§11.1, §11.2) |
| R16b | A `python` `B` outcome run killed at `params.timeout` after reporting three of five ids | the two unreported ids take `out: "absent"`; the floor still holds five entries; `end.status` is **not** `base-collect-failed` and **not** `runner-timeout` (§11.1, `result-file.md` §7.3) |
| R16c | A `pytest` id trunk reports `xfail` on `B` and `xfail` on `T` | `base.out` is `xfail`, `result.out` is `xfail`, and `result-file.md` §8.5 clause 2 raises **no** finding — neither G1's nor G8's. **Must not** be a G1 or a G8 finding |
| R16d | The same id, reported `xfail` on `B` and **absent** from `T`'s collection | the ordinary went-away allocation: a `G8:<path>` finding, and G1 unless a `class=protected` review names that path. **Must not** be released by the carve-out |
| R16e | A `pytest` id trunk reports `passed` on `B` and `xfail` on `T` | G8 and G1 both. **Must not** be released by the carve-out — `b.out` is `passed` |
| R16f | A `pytest` id trunk reports `skipped` on `B`, still collected on `T` and reported `skipped`, `failed` or `error` there | `base.out` is `skipped`, the id **stays** in the floor, and `result-file.md` §8.5 clause 2 raises **no** finding — neither G1's nor G8's. **Must not** be a G1 or a G8 finding, and **must not** be dropped from the floor |
| R16g | The same id, reported `skipped` on `B` and **absent** from `T`'s collection | the ordinary went-away allocation: a `G8:<path>` finding, and G1 unless a `class=protected` review names that path. **Must not** be released by the carve-out |
| R17 | The `B` collection for `ts` | `vitest run`, the same command as `T`, serving both the enumeration and the outcomes. **Must not** be a list-only mode: it omits every skipped test and shrinks the floor (§11.3, §11.7) |
| R17a | Any `vitest` or `dart-test` `base` record | an `out` from that adapter's own mapping, never `xfail` — neither runner has an expected-failure value (§11.3, §11.4, §11.6 rule 5). `skipped` **is** producible by both, and is the carve-out's other exempt value, so the carve-out is reachable in a TypeScript and in a Dart suite through that value alone |
| R18 | A `pytest` `B` collection in which a hook deselects an item | that id is **not** in the floor |
| R19 | A `pytest` `B` collection interrupted by a collection error, or a `vitest` `B` collection with a file that fails to load | `base-collect-failed`; no `base` and no `result` records from any runner. **Must not** write the partial floor |
| R20 | A `pytest --collect-only` over a tree with no tests | exit `5`, and the floor is legitimately empty. **Must not** be read as a failure |

**The pragma and the sugar**

| # | Case | Required |
|---|---|---|
| J1 | `# @verifies INT-042/AC-1` in a Python comment | one pragma occurrence |
| J2 | `"""@verifies INT-042/AC-1"""` in a Python docstring | **no** occurrence — a docstring is a string, not a comment |
| J3 | `// x@verifies INT-042/AC-1` | no occurrence — `@verifies` must not be preceded by `[A-Za-z0-9_@]` |
| J4 | A pragma in a file from which the runner collected three ids | three `verified_by` edges (file granularity, §12.2) |
| J5 | `def test_AC1_and_AC2_totals` under pytest | two edges, AC-1 and AC-2 |
| J6 | `def test_AC12_totals` | one edge, AC-12 — **not** AC-1 |
| J7 | `def test_MAC1_x` | no edge — `AC` must not be preceded by `[A-Za-z0-9]` |

---

## 15. Determinism rules, collected

1. Every read names `A` or `B` (§2.1, §2.2). There is no third tree and no filesystem (§2.12 rules 1–2).
2. Resolution configuration comes from `B`; a difference between `A` and `B` is a tripwire, not a silent re-resolution (§3.3).
3. No configuration, platform, flag, variant or target is ever read; conditionals take the union (§3.7).
4. Every candidate list is ordered and exhaustive; first match wins (§2.12 rule 6).
5. Classification is a pure function of the path; the walk's order is immaterial and the output is a set (§2.5, §2.12 rule 3).
6. Membership of clause 3 and clause 4 is decided by path and basename, never by file content (§2.7, §2.8).
7. Byte-wise matching everywhere; no case folding, no Unicode normalization, no separator rewriting (§2.4 rule 1).
8. No value in the closure derives from a clock, a duration or a date.
9. Decoding is UTF-8 or nothing; no encoding declaration is honoured (§3.4 rule 1).
10. Two runs of one release over one pair of trees produce the same set on any host.

---

## 16. Resolved ambiguities

Each entry states what the playbook says, what this document chose, and why.

### 16.1 The closure's five cases were prose, not a function

**Playbook:** PB §4.3 gives clause 2 as three sentences — two exclusion cases, one leaf case — plus *"Everything else in the walk is frozen"* and *"an import that resolves outside both expected and the harness is frozen as a leaf."* It never says whether a harness file is walked or is a leaf.
**Chosen:** §2.5's five-row table, with `FROZEN_WALK` for harness files.
**Why:** *"The walk prunes at an excluded import"* is the only pruning rule stated, so a frozen non-leaf is walked; and it must be, or `vitest.config.ts`'s import of `vitest.setup.ts` never reaches the setup file and a root setup file — the exact hazard clause 3 names — escapes the closure.

### 16.2 Runner configuration by pattern or by content

**Playbook:** clause 3 says "runner configuration" and gives no test.
**Chosen:** basename and location only (§2.7). `pyproject.toml` is Python runner configuration wherever it sits on a seed's ancestor chain, whether or not it contains `[tool.pytest.ini_options]`.
**Why:** a content test puts a TOML, JSON and YAML parser between two implementations that must agree to the byte, and it lets a branch change the closure by adding or removing a section. The cost — §18, OPEN-5 — is that a dependency addition after approval is a G8 failure, which PB §5.2 already treats as a wire in its own right.

### 16.3 "A module whose imports cannot be resolved statically is unclassifiable"

**Playbook:** PB §4.3, one sentence, alongside a tripwire for *"an unresolvable or dynamic import inside test roots."*
**Chosen:** the site level (§3.8): an import site whose target cannot be determined yields no edge, and the *importing* file keeps its classification.
**Why:** "stays excluded" is meaningful only about a path the resolver can name, and a site whose target is unknown has no path. The alternative reading — the importing module becomes unclassifiable and drops out — would let a test file with one dynamic import drop its whole closure, which is the opposite of fail-closed. §17 D7 files the collision.

### 16.4 Which tree the resolution configuration comes from

**Playbook:** silent. It says the closure is computed over the approval tree and that clause 2's `base=` test reads the base tree, and says nothing about `tsconfig.json`, `Package.swift` or `settings.gradle`.
**Chosen:** `RC(lang, B)`, with `RC(lang, A) ≠ RC(lang, B)` a tripwire (§3.3).
**Why:** the branch writes `A`. A candidate that can restructure `Package.swift` can move its oracle outside every target the resolver knows, and the recomputation in CI reads the same restructured manifest and agrees. PB §4.3 already reasons this way about clause 2 — *"It is read from the base tree, which the branch cannot edit"* — and this is the same argument one file over.

### 16.5 Type-only, for languages that have no such thing

**Playbook:** *"type-only imports do not count"*, stated generally.
**Chosen:** only TypeScript has the form (§3.6); Python's `if TYPE_CHECKING:` is an ordinary import.
**Why:** recognizing `TYPE_CHECKING` requires deciding that the name is `typing`'s and not a local `TYPE_CHECKING = True`, which is a name-binding question the resolver refuses. Over-freezing a type-referenced module is cheap; a rule two implementations resolve differently is not. §17 D6 asks the playbook to scope the sentence.

### 16.6 "Module" as the unit of the `base=` test

**Playbook:** clause 2 says "a module that existed at the approval's `base=`".
**Chosen:** the unit is a file path (§2.5).
**Why:** for Swift a module is a target, so on a module reading a branch-created file inside an existing module "existed at base" and escapes the closure tripwire silently. `Spine-Frozen` names files; classification names files.

### 16.7 What a bare name that matches nothing is

**Playbook:** silent; it distinguishes only "repo-local imports".
**Chosen:** `external`, with `unresolvable` reserved for targets that are in the repository and cannot be identified (§3.2).
**Why:** an oracle must be a file in the tree, and a file in the tree is found by the language's resolution rule. The opposite default would make `import Foundation` and `import XCTest` tripwires, so every Swift approval would route to a human and the tripwire would carry no information.

### 16.8 Whether a literal `import('./x')` is "dynamic"

**Playbook:** the tripwire says *"an unresolvable or dynamic import inside test roots"*, and TypeScript calls `import()` a dynamic import.
**Chosen:** "dynamic" means *the specifier is not statically determined*; a literal `import('./x')` resolves (§5.2).
**Why:** it is exactly as determined as a static import, and refusing it would put a human in front of every approval in a repository that lazy-loads. `require(expr)` and `import(expr)` remain `unresolvable`.

### 16.9 How a fixture file reaches `Spine-Frozen`

**Playbook:** PB §4.3's example lists `Spine-Frozen: 58d2… tests/fixtures/invoices.json`, and no clause mentions fixtures.
**Chosen:** by an ordinary import edge where the language has one (TypeScript's JSON module import, §13.2), and otherwise not at all — a fixture read at runtime with `open()` is protected by G8's blanket `C-T1`/`C-T2` rule rather than by the closure.
**Why:** inventing a fifth clause for "data files a test might read" would require deciding what a test reads, which is a runtime question. G8 already protects everything under a test root from the branch's side after approval (PB §6.3), so nothing is lost.

### 16.10 What "test roots" means for the tripwire

**Playbook:** the tripwire is *"an unresolvable or dynamic import inside test roots"*, and `C-T1` is called "test roots" while `C-T2` is "test support".
**Chosen:** `H` — `C-T1` ∪ `C-T2` (§2.11).
**Why:** a dynamic import in `tests/support/factories.ts` is exactly as much of a hole as one in the test file, and the walk treats the two identically everywhere else. Reading the tripwire as `C-T1` only would leave the support tree unguarded.

**The same reading now governs `C-T3`** (§12.4), where the phrase appeared again in PB §2.1 and again — as `C-T1` by name — in `constitution.md` §6.3, and both now spell the union out. There the consequence of reading it narrowly is sharper than an unguarded support tree: `spine init` renders `vite.config.*`, `vitest.config.*` and `**/conftest.py` into `C-T2`, all of them import their framework by construction, and none is under any scaffolded `C-T1` pattern — so a repository would fail G8 on its first landing over the tool's own scaffold. §17 D12 asked the playbook and the constitution spec for the wording and both have taken it: PB §2.1, PB §4.3 and PB §6.3's G8 row read *"outside the harness (`C-T1` ∪ `C-T2`)"*, and `constitution.md` §6.3's row reads the same. §12.4 fixed the predicate either way, because a predicate that refuses the shipped default is not a candidate reading.

### 16.11 Two sibling documents defined two pattern dialects

**Playbook:** silent — PB §5.2 says only *"path-prefix matching"*, and PB §2.1 gives pattern-shaped values with no grammar (§17, D4).
**Chosen:** `intent-doc.md` §6.1–§6.3, adopted by reference (§2.4). Version 1 of this document defined a rival dialect in the same place.
**Why:** the choice is not this document's to make and was not made here — `constitution.md` §14.15 adjudicated it, on the ground that G2's quick-lane clause compares a constitution list and a touchpoint list against one diff in one set expression, so one semantics is forced. What §2.4 adds is the execution, which had not happened, and the reason it could not wait: version 1's rule 2 read a trailing-`/` pattern as a raw byte prefix, under which the shipped `C-T1` value `src/**/__tests__/` matched nothing at all — empty harness predicate, empty closure, and a G8 that rejected every approval in any repository using the scaffolded TypeScript default. §2.4.2 publishes the vector. `constitution.md` §14.15's *"where they agree, nothing turns on it"* is false for that value and is withdrawn.

---

### 16.12 The seed rule's two edges: the naming sugar, and an empty seed set

**PB said:** clause 1's seed is *"every file under a `C-T1` test root, in the approval tree, carrying a pragma naming an acceptance criterion of this intent"*, and it says in the same breath that the seed is *"lexical, not collected"* (PB §4.3). PB §6.2 gives a test **two** ways to name a criterion — the `@verifies` pragma and the `AC<n>` naming sugar — and PB §4.3 names only one of them.

**Chosen:** the pragma seeds; the sugar does not. Two reasons, of which the second is the one that leaves no choice.

- The sentence says *"carrying a pragma"*, and a document that made the sugar seed as well would be reading a rule that is not there into the one clause PB §4.3 spends a paragraph insisting is lexical.
- The sugar is not carried by a file at all. §12.3's pattern runs over a **runner-native test id's** field — the last `::` component of a pytest nodeid, the bytes of a `swift-test` specifier after the `/` — and a runner-native id exists only after a collection, the input §2.1 refuses. Reading `AC<n>` out of the source instead would mean matching it against a *declaration name*, which the resolver would have to parse for; §1 forbids the parse and §12.2 already declines it for the pragma join, which is why that join is file-granular.

**What that costs, and what bounds it.** It does not cost coverage: the sugar still yields `verified_by` edges at index time from collected ids (§12.3), so G1's coverage clause and PB §6's transition table and its *"every AC covered by a collected id"* approve guard are untouched. What it costs is a seed, and with it whatever that file alone imports. Two things bound the loss, and both are worth stating because they are what makes the choice safe rather than merely defensible. A sugar-only file under `C-T1` satisfies `H`, so PB §4.3's read-only-after-approval clause and PB §6.3's G8 clause over *"any `C-T1`/`C-T2`/runner-config path"* protect its own bytes whether the closure names it or not. And any *other* seed that imports it reaches it as `FROZEN_WALK` by row 1 of §2.5, because `H` holds for it there too. What is genuinely lost is the `FROZEN_LEAF` case — a fixture outside `C-T1` and `C-T2` that only a sugar-only test imports.

**An empty `S` is where the loss is total**, and it is a tripwire (`no-seed`, §2.11) rather than a refusal or a silence. A repository whose tests carry no pragma at all freezes nothing: `Spine-Frozen` is empty, G8's *"closure ⊆ `Spine-Frozen`"* is `∅ ⊆ ∅` and passes, and every `FROZEN_LEAF` clause 2 exists for is unprotected. Refusing outright was rejected — the repository is not malformed, the remedy is one comment line, and `seed-outside-test-roots` is already the refusal for the case where something *is* wrong. Staying silent was rejected because the guarantee §10 states is void and nothing else in the run would say so. The tripwire routes the intent to `approval-review`, `--approve` refuses without a `reason=`, and `spine stats` counts it as `seedless_approvals` so the rate is visible.

---

## 17. Defects found in PLAYBOOK.md v0.19

Reported rather than repaired, per `docs/spec/README.md`. Where this document and PB §11 disagree, §11 wins and the disagreement is a defect in one of them; none of these is in §11 except D5, which was in both. **Citations are section anchors plus a verbatim quote, never line numbers** — a line number rots as the playbook grows, a section and a quote do not. Every entry is marked **OPEN** or **CLOSED** against `PLAYBOOK.md` as it now stands.

**D1 · OPEN · Clause 3 names the wrong Python file for its own rationale** (PB §4.3, *"**What is frozen: the closure, not the file list.**"*, clause (3)). It reads: *"runner configuration and package `__init__.py` files on the path from repo root to each test — a root setup file can deselect every test below it without touching one."* `__init__.py` cannot deselect anything. The file that carries the hazard the sentence describes is `conftest.py`: pytest auto-loads every one from rootdir down with no import statement, and a `pytest_collection_modifyitems` hook in a root `conftest.py` deselects every test below it. `conftest.py` appears **nowhere in the playbook**. An implementer who freezes what the clause names and not what it means ships a resolver that misses the stated attack. Recommended: name `conftest.py` in the clause and let `docs/spec/import-resolver.md` §4.4 carry the rest.

**D2 · OPEN · Clause 2's `base=` test needs a whole-tree reverse-import query the playbook never mentions** (PB §4.3, clause (2)). PB §4.3 describes the closure as a walk from the tests, then makes classification depend on whether a module *"was imported there by a non-test file"* at `base=`. Deciding that requires resolving the imports of **every** non-test file in the base tree — a full-tree resolution pass, by far the most expensive operation in the design, and one a reader implementing the clause as written will not build. The playbook also gives it no behaviour when a base-tree file's own imports are unresolvable. §2.9 supplies both. Recommended: one clause saying the predicate is a reverse query over the base tree, with a pointer.

**D3 · OPEN, narrowed · "`C-T1`/`C-T2`/runner-config" is three sets in PB §4.3 and PB §6.3 and two in PB §2.1.** PB §4.3 and PB §6.3's G8 row consistently write the harness as *"any `C-T1`/`C-T2`/runner-config path"*, as though runner config were a third set beside them, while PB §2.1's `C-T3` comment writes it as two — *"No test-framework import or runner hook outside the harness (C-T1 ∪ C-T2)."* (**As filed**, the evidence for the two-set reading was PB §2.1's scaffold folding the per-runner configuration into `C-T2` itself — `C-T2: test support: tests/support/**, <per-runner config>`. The scaffold's normalisation replaced that with `C-T2: test.support = <per params.langs>` and moved the patterns to `constitution.md` §6.4, which narrows the evidence and does not close the defect: the three-set phrasing is unchanged in both places that carry it.) If it is `C-T2`, then `gate-report.md`'s `policy.rules.c_t2` records it and the digest covers it; if it is a third set living inside the binary, then two releases with different config lists compute different closures over an identical constitution and an identical manifest, and nothing in the report says so. Recommended: strike "runner-config" from both phrasings and let `C-T2` be the harness's second half, with `spine init` rendering the per-language patterns (§4.5, §5.5, §6.5, §7.6).

**D4 · OPEN, narrowed · No document says what a path pattern means, and §2.4 no longer invents one** (PB §5.2, *"In v1, touchpoint checks are path-prefix matching"*). `C-T1`, `C-T2`, `C-Q1`, `C-A2` and the intent's touchpoints are all patterns, and G2, G8, G14 and the whole freeze closure depend on matching them. PB says only *"path-prefix matching"* (PB §5.2), which leaves undefined whether `**` crosses `/`, whether `tests/` matches `testsuite/x`, whether `*` crosses a separator and whether matching casefolds — and each answer changes which files a landing freezes and which the floor protects. **The dialect is `intent-doc.md` §6.1–§6.3**, adjudicated by `constitution.md` §14.15 and adopted by §2.4 here; version 1 of this document defined a rival one in that slot and §2.4.1 records what deleting it cost. PB §6.3's G2 query now cites the dialect in a comment — it opens *"`spine_match` is the touchpoint matcher, not equality"* and delegates the matcher's semantics to `docs/spec/intent-doc.md` — which is one of the two places a reader looks and not the one PB §5.2 sends them to. Recommended, unchanged: PB §2.1 and PB §5.2 each cite `intent-doc.md` §6.1–§6.3 once, so that a reader of the playbook alone is not left to invent a third.

**D5 · CLOSED · `freeze=` was a signed digest with no defined ordering, and every landing depended on it** (PB §4.3's `Spine-Approve` field glosses). **As filed**, PB §4.3 read only *"`freeze=` a SHA-256 over the sorted `Spine-Frozen` and `Spine-Test` lines"*, and "sorted" fixed nothing: it did not say whether a line is the rendered trailer (`Spine-Frozen: <oid> <path>`) or the payload, whether the key is the whole line — which begins with an object id, making the order effectively oid order and therefore different for the same file set on two different trees — or the path, whether the collation is byte order or something else, and whether the two kinds are interleaved or concatenated. PB §6.3's G9 requires *"the SHA-256 over that commit's sorted `Spine-Frozen`/`Spine-Test` lines equal to the copied approve line's `freeze=`"*, so two conforming implementations disagreed on every landing and every landing indexed `unattested`. This was the class of defect that fails every landing. The recommended home was `envelope-vectors.md`, with PB pointing at it. **Taken:** PB §4.3 now continues *"Sorted how, and over which bytes, is not a detail: `docs/spec/envelope-vectors.md` fixes it (whole lines including the trailer name, ascending by unsigned byte value, LF-joined, no trailing LF) and publishes vectors, because two implementations that sort differently compute different digests over the same approval and each rejects the other's."* **This document still publishes no `freeze=` vector**, for the reason it never did — the digest is the approval record's (§2.10).

**D6 · OPEN · "Type-only imports do not count" is a general rule with one instance** (PB §4.3, *"re-exports count as imports, type-only imports do not"*). PB §4.3 states it once and unconditionally, for every v1 language — the sentence carries no language count and did not lose one when Kotlin went. Three of the four have no type-only import form at all, and Python's near-miss — an import under `if TYPE_CHECKING:` — is not syntactically one and cannot be recognized without resolving a name binding. Two implementers reading the sentence will make opposite calls about Python and compute different closures. Recommended: *"type-only imports, where the language has such a form, do not count"*, with the per-language answer in this document (§3.6).

**D7 · OPEN · Two sentences describe the same event with different subjects and different outcomes** (PB §4.3, both sentences inside the closure paragraph). It says *"a module whose imports cannot be resolved statically is unclassifiable and stays excluded, counted by `spine stats`"* — silent, counted — and in the next breath makes *"an unresolvable or dynamic import inside test roots"* an approval tripwire — loud, human. A test file with a dynamic import satisfies both. The first sentence is also not well-formed: a module the resolver cannot identify has no path to exclude. §16.3 and §3.8 resolve it by level. Recommended: the first sentence attaches to an import site, and the tripwire is the narrower rule that wins inside the harness.

**D8 · OPEN · `params.langs` is defined as the harness's languages, but the closure needs the code's** (PB §6.7, *"`params.langs` is the set of languages this repository's harness is written in"*). The closure walks *out of* the harness into the code under test, and clause 2's base-tree predicate resolves the imports of every non-test file. A repository whose harness is TypeScript and whose code under test is Python — an ordinary shape — would declare `langs: ["ts"]` on the sentence's plain reading, and every Python edge would silently vanish: no closure member, no `nonTestImporter`, so every Python module a test reaches would freeze as a leaf or be missed. Recommended: *"the set of languages this repository's harness and the code it tests are written in."*

**D9 · OPEN · A resolver change in a new release invalidates every in-flight approval, with no named remedy** (PB §4.3, *"In `--ci`, G8 recomputes the closure over the approval commit's tree with the pinned release"*). PB §4.3 makes G8 recompute the closure *"with the pinned release"* and fail if a computed file is missing from `Spine-Frozen`, and PB §6.7's upgrade lifecycle has nothing to say about it. A release that fixes a resolver bug computes a larger closure over the same tree, so every approval taken under the previous release now under-freezes and every open intent must be reopened. The only exits are a signed reopen or a counted freeze override (PB §6.3, G8), and `spine stats` has no counter that would make the event visible as anything but a spike in freeze overrides. Recommended: treat a resolver change as a `resign`-class bump (PB §6.7's mechanism already exists for exactly this — an announced, counted, rare bump that forces re-approval) and say so in the release-notes rule.

**D10 · OPEN · The closure tripwire's "mechanical remedy" is not mechanical in Swift** (PB §4.3). It reads: *"the tripwire has a mechanical remedy: move the module under a `C-T1`/`C-T2` path, where the harness rules already freeze it and G8 already guards it, and re-approve."* In Swift, moving a file into `Tests/` moves it into the test target, where the module under test cannot see it — the code stops compiling. The remedy is mechanical for Python, TypeScript and Dart, where a path is just a path. Recommended: say the remedy is per-language and that for Swift the alternatives are signing past the tripwire with a `reason=` (which the design already permits and counts) or making the oracle a test-target file that the code under test does not need. (Version 1 filed the same defect for Kotlin, which is no longer a v1 language; Appendix A keeps it.)

**D11 · CLOSED by PLAYBOOK.md v0.19, and the sweep it named is done.** As filed: PB defined `params.langs` over five languages and said each of the five *"has an id grammar … specified in `docs/spec/`"* — the first over-counting after the owner's decision of 2026-08-26, the second false when written (version 1 §11.4 fixed no grammar for three of the five) and true only of four thereafter. Left as it stood, an implementer reading PB alone would have built a Kotlin resolver this document does not define and refused a Dart repository it does. **PB v0.19 now ships four** — *"v1 ships Python, TypeScript/JavaScript, Dart and Swift — Kotlin was dropped"* — so the second sentence does not survive. **The first does, and version 2 of this document was wrong to say the playbook enumerates no language set anywhere else.** As filed, PB §12 still read *"Five languages is the widest v1 in any version of this document, and the cost is concentrated in one place — `docs/spec/import-resolver.md` owes five total, deterministic resolvers"*, two lines below the sentence that ships four — the last five-language claim in the playbook, naming this document and telling an implementer that a fifth resolver was owed. **That is taken too:** PB §12 now reads *"The cost of the remaining four is concentrated in one place — `docs/spec/import-resolver.md` owes four total, deterministic resolvers"*, and its only surviving `five` is historical — *"Five languages was briefly the set, and the cross-document review cut it to four"*. The sibling edits this defect asked for are also taken: `result-file.md` §6.4, `constitution.md` §3.5/§6.4/§14.3/§15 D17, `gate-report.md` §4/§5.4.2/§10, `dump.md` §12 and `docs/spec/README.md` all now say four, and README's *Settled by the owner* entry no longer lists Kotlin. The two stale five-token claims this entry filed **outside** this document are gone as well: `envelope-vectors.md` §4.4 now reads *"`import-resolver.md` §11.1 ratifies the four tokens v1 ships"* and `templates.md` §15 reads *"**The four runner tokens** `pytest`, `vitest`, `dart-test`, `swift-test`"*, both agreeing with §11.1, which ratifies **four** and reserves `gradle`. Nothing in this entry is outstanding.

**D12 · CLOSED — the wording is taken in all three documents · `C-T3`'s "outside test roots" named `C-T1`, and `C-T1` alone failed `spine init`'s own scaffold** (PB §2.1's `C-T3` comment and its `C-T1` line; PB §4.3; PB §6.3's G8 row). **As filed** (the quotations below are of the text before the fix): PB §2.1's `C-T3` line read *"no test-framework import or runner hook defined outside test roots"*, PB §4.3 repeated *"outside test roots"*, and PB §2.1 named `C-T1` *"test roots"* in as many words — so the plain reading of the rule was `C-T1` and nothing else. `constitution.md` §6.3 had taken that reading verbatim: *"G8 runs a tree grep for a test-framework import or runner-hook definition outside `C-T1`"*. Under it, a repository scaffolded by `spine init` fails G8 on its first landing. `vitest.config.ts` is rendered into **`C-T2`** and not into `C-T1` (§5.5); it imports `defineConfig` from `vitest/config` because that is what the file is; and it sits at the repository root, which `C-T1`'s `tests/`, `src/**/__tests__/` does not reach. `**/conftest.py` is the same shape and imports `pytest` by construction. The harness is `C-T1 ∪ C-T2` in every other rule that reads it (PB §4.3's read-only clause, PB §6.3's G8 row, §2.3 here), and §12.4 evaluates the predicate over `H` for that reason. **Recommended, and taken:** PB §2.1's rule line, PB §4.3 and PB §6.3's G8 row now read *"outside the harness (`C-T1` ∪ `C-T2`)"*, and `constitution.md` §6.3's row reads the same; the rule's name stays `test roots`, since a name is not a predicate. §12.4's predicate is unchanged — it always evaluated over `H` — so no conformance case, worked example or digest in this document moves.

---

## 18. OPEN — the owner's calls

**OPEN-1 · Closed by the owner, 2026-08-26: Kotlin is dropped from v1.** Version 1 put the question as *detect the Java hole and refuse, add Java as a sixth language, or drop Kotlin*, and recommended detect-and-refuse. The owner took the third: an oracle in a `.java` file inside a mixed Kotlin module is invisible to a Kotlin resolver, nothing reports the miss, and a guarantee that fails silently cannot ship — while detect-and-refuse turns the hole into a refusal that fires on the ordinary shape rather than the exotic one. `kotlin` and `gradle` are reserved tokens (§11.1) and the analysis is Appendix A, so a later release adding the language starts from it rather than from nothing. **Read this with OPEN-2, which is the same rule reaching the other outcome:** Swift's Objective-C hole is the identical shape and is made loud rather than removed, because the shape it refuses is the exotic one there and was the ordinary one here (§10). Neither language ships a silent hole.

**OPEN-2 · Closed 2026-08-27: detect and refuse. Swift stays in v1 and its Objective-C hole is loud.** As filed: a target containing `.m`/`.h` files or a bridging header can hold an oracle the resolver cannot see — Objective-C is `lang: none`, so no edge exists (§10) — which is the same *shape* as the Kotlin/Java hole that cost Kotlin its place, with the same three exits: detect and refuse, add the language, or drop Swift. Leaving it OPEN while shipping Swift in v1 was the corpus contradicting its own stated rule, since PB §6.7 removed Kotlin for exactly this failure class.

**Taken, as recommended: detect and refuse.** §7.3 makes `RC(swift, tree)` unclassifiable with reason `mixed-objc-target` whenever any target of any package in the tree carries a C-family entry in its file set or a C-family manifest construct, evaluated in **both** trees so that a branch-added `.m` is caught; §7.8 lists the reason; §10's row states the residual as a capability limit rather than a silent hole; §14 S12–S19 vector each clause. **One filed narrowing did not survive the specification and is recorded here rather than quietly widened:** the recommendation scoped the test to *a target's source directory contains a `.m`, `.mm` or `.h`*, and the excuse for the narrow scope was that "SwiftPM's own convention keeps ObjC in its own target". That convention is not a mitigation — a Swift file that writes `import CBits`, where `CBits` is a target with no `.swift` source, gets zero edges and no finding, which is the silent miss in its purest form — so the test is repository-wide, covers pure C-family targets, and covers the C-family extensions rather than the Objective-C ones alone (§7.3 says why the token is still `mixed-objc-target`).

**What it costs and who paid it:** a repository that mixes C-family sources into its Swift package becomes ungatable for Swift, which is the owner-level half — it makes a currently-gatable repository shape ungatable. That is the price the rule sets, and the alternative was the one PB §6.7 already refused.

**OPEN-3 · Whether `RC(lang, A) ≠ RC(lang, B)` should tripwire or refuse.** §3.3 makes it a tripwire a human can sign past with a `reason=`. The alternative is an outright refusal: the branch must land the structural change on trunk first, through its own gated intent, which is exactly the remedy PB §4.3 prescribes for buying an exclusion. Refusing is stricter and more consistent with the clause-2 reasoning it borrows; tripwiring is kinder to the intent that legitimately adds a test target. **Recommendation:** tripwire, as specified, and revisit if `spine stats` shows the reason being signed routinely. Owner-level because it changes how often a human is in the loop.

**OPEN-4 · Whether a second TypeScript adapter ships in v1.** §11.1 ratifies one runner per language. Jest is at least as common as vitest, and the `runner` token is permanent — a repository that runs both would need both adapters, and `Spine-Test` lines naming a runner outside the invocation set can never pass (`result-file.md` §6.2). **Recommendation:** ship `vitest` only in v1 and add `jest` in a later release; the token `jest` is reserved now so nobody uses it for anything else.

**OPEN-5 · Whether the package manifest belongs in the closure.** §2.7 puts `pyproject.toml` and `package.json` on the ancestor-config list because both can reconfigure collection. The consequence is that **adding a dependency after approval is a G8 failure**, whose only exits are a reopen or a counted freeze override. That is consistent with PB §5.2's new-dependency wire and it is a real friction. The alternative — freezing only a region of the file — is not available, because freezing is by blob id. **Recommendation:** keep it, and let `spine stats` measure how often a freeze override cites a manifest edit. Owner-level because it is a workflow cost, not a correctness question.

**OPEN-6 · The 200-file closure threshold does not fit Swift.** PB §4.3 tripwires *"a closure over 200 files"* on the reasoning that the harness is too entangled to freeze honestly. For Swift the closure is module-shaped — the whole test target, plus every file of every non-`expected` module a test imports (§7.4) — so it crosses 200 in a medium repository with no entanglement at all, and the tripwire would fire on every approval and stop meaning anything. Three ways out: a per-language threshold; count *modules* rather than files where a language's unit is a module; or keep one number and raise it. **Recommendation:** count files but exclude clause-3 and clause-4 members and `FROZEN_WALK` harness files from the count, leaving the threshold measuring what the sentence says it measures — how much non-harness code the tests have pulled in. Owner-level because it changes a published threshold.

**OPEN-7 · TypeScript monorepos: `exports` maps and per-project tsconfigs.** §5.2 reads no `package.json` `exports`/`imports` and §5.3 reads one repository-root `tsconfig.json`. A monorepo whose workspace packages resolve through `exports` sees those specifiers as `external`, so a helper in a sibling workspace package is outside the closure. Supporting `exports` means implementing the conditional-exports algorithm, which is condition-set dependent — i.e. environment dependent — and would breach §1. **Recommendation:** leave it, document it (§10), and revisit only if a real repository needs it; the safe workaround is a `tsconfig` `paths` alias, which resolves.

**OPEN-8 · Discharged: all four adapters are ratified (§11), and swift-testing is the residue.** Version 1 declined `dart-test`, `swift-test` and `gradle`, which left `params.langs` unable to name `dart` or `swift` in a conforming repository. §11.4 and §11.5 ratify the two that survive the language decision, against reproduced reporter output (§11.7), and no `docs/spec/runner-adapters.md` is owed. What remains open is narrower and is genuinely the owner's: **does swift-testing (`@Test`) get a v1 adapter?** Today §11.5 detects it and fails the job, which is loud and correct but makes a repository that has begun migrating ungatable. The alternatives are to write a second Swift adapter with its own permanent `runner` token (`swift-testing` is the obvious spelling and is not reserved yet) or to ship XCTest only and say so in the release notes. **Recommendation:** XCTest only in v1, reserve the token now.

**OPEN-9 · `dart-test` collects the `B` floor by running the suite, and one truncation is undetectable.** `dart test` has no list-only mode and every one of its selection flags is forbidden by `result-file.md` §7.2, so §11.4 obtains `B`'s id set by running the same invocation against the checkout of `B` and keeping only the ids. The completeness check — each suite's root `group` declares a `testCount`, which must equal the records emitted for that suite — catches a suite that failed to load and a suite whose process died part-way, and promotes both to `base-collect-failed`. What it does not catch is a suite whose *root group's own `testCount`* is smaller because a `for` loop over a list built at load time produced fewer cases on `B` than on `T`; that is a genuinely shorter suite and a shrunken floor at once, and the protocol offers nothing to tell them apart. It costs one extra full suite run per landing as well. Two ways out: ask upstream for a list-only reporter mode, or accept the cost and the residual and say so in the release notes. **Recommendation:** accept and say so; the residual requires the candidate to change trunk's own test count, which is a trunk edit and therefore already gated. Owner-level because it is a per-landing cost, not a correctness question.

**OPEN-10 · One fact in §11.5 is cited rather than reproduced, and it should not ship that way.** The corelibs (non-Darwin) spelling of XCTest's `Test Case '…' <verb>` line, and `XCTestCase.name` being `"\(type(of: self)).\(name)"` with no target component, come from `swift-corelibs-xctest`'s published source and not from a Linux run (§11.7). Everything that depends on it is §11.5's join and its `ambiguous-test-class` refusal — which exists *because* of that spelling. If the corelibs line differs from what the source implies, a Linux collector attributes outcomes to the wrong ids or to none. **Recommendation:** a Linux reproduction of §11.7's swift vector is a release-blocking checklist item, not a follow-up. It is an hour's work and it is the only unreproduced byte in this document.

**OPEN-11 · Discharged: the `B` collection is fixed for all four runners — enumeration in version 3, outcomes in version 4 (§11.1).** As filed: §11.1 named each runner's invocation on `T` and a collection command for `dart-test` and `swift-test` only, while `result-file.md` §7.1 step 7 requires a collection on `B` for every runner and does not say how — so two implementations could reasonably collect pytest's `B` ids with `--collect-only` and with a full run and get different sets. Version 3 fixes both against reproduced output (§11.7), exactly as §11.4 and §11.5 were, and the answers are not symmetrical. **pytest: `pytest --collect-only`** — reproduced to yield the identical id set to a full run, including under a decorator skip, a module-level skip and a collection-hook deselection, so the feared divergence is not reachable and the cheap command is the correct one. **vitest: `vitest run`, the same command as `T`** — because `vitest list`, which looked like the obvious answer, **omits every skipped test** and would write a floor smaller than `B`'s real one. What is left open is not the command but one consequence of "collected and selected": §18, OPEN-13.

**OPEN-12 · Which `runner` tokens are reserved, three documents, three answers.** `result-file.md` §6.4 reserves `gradle`, `junit` **and `kotest`** and reserves `kotlin` as a language token; §11.1 here reserves `kotlin`, `gradle` and `jest` and has never mentioned `junit` or `kotest`; `manifest.md` §3.3 says `"kotlin"` is *"not reserved either: a later release that solves the mixed-module problem adds it as a release, not as a repo setting."* Nothing in v1 emits any of the five, so the disagreement is inert today. It is not inert later: a `runner` token is sealed into a `Spine-Test` line for ever (`result-file.md` §6.3 obligation 1), and a reservation is a promise about a name, which is exactly the kind of thing that cannot be made retroactively. **Recommendation:** reserve all of `kotlin`, `gradle`, `jest`, `junit`, `kotest` and `swift-testing`, and let `manifest.md` §3.3 say only what it needs to — that `"kotlin"` is outside `params.langs`' domain — which is a different claim from *unreserved* and is the one its check needs. Owner-level because it is a permanence decision and one word in each of three documents, not an editorial one.

**OPEN-13 · Closed by the owner, 2026-08-27: an id trunk itself skips stays in the `B` floor and is carved out of both gates.** §11.1's shared rule makes the floor the ids the runner collected and selected, irrespective of outcome, and every adapter here honours it: `dart-test` emits a `base` record for a skipped test (§11.4, row 5), `swift test list` lists a method that will throw `XCTSkip`, `pytest --collect-only` collects a `@pytest.mark.skip` item, and `vitest run` reports an `it.skip` case. As filed, `result-file.md` §8.5 clause 2 then required a `passed` `result` record for every `base` record, so **a test skipped on trunk blocked every subsequent landing** until a `class=protected` `G8:<path>` review named its path — and again on the next landing, because trunk had not changed. Skipped tests on trunk are ordinary, which made this an every-landing availability failure rather than an edge case. **The answer is not the one recommended here.** Redefining the floor as *collected, selected and not skipped* would have moved a membership rule in four adapters and changed what `ids=` counts; the owner extended the carve-out that already existed instead. `result-file.md` §8.5 clause 2 now reads `b.out` against **two** literals, `xfail` and `skipped`, on the *did not pass* shape only, and PB §6.3's G1 and G8 rows both write the same predicate — so such an id raises no finding in either gate and needs no review, while the floor's membership, `ids=`, the record grammar and §4.5's sort are all untouched. The reasoning is the one that released `xfail`, word for word: an id that did not pass on trunk is not a guarantee this landing can retire. The went-away shape is unmoved — deleting a skipped test is a harness change and still takes its protected review (vectors R16f and R16g, §14). `result-file.md` §14 OPEN-8 and §13 R35 record the same decision from the other side.

**What the datum made possible, and the order in which it was taken.** The input OPEN-13 lacked exists since version 4: every `base` record carries its `B` outcome (§11.1, `result-file.md` §4.4), so `skipped` on `B` is one literal comparison away from being actionable and costs no further invocation in any adapter. Version 4 stopped there deliberately — the carve-out of `result-file.md` §8.5 clause 2 was written because PB §6.3 wrote it in both the G1 and the G8 row, `skipped` appeared in neither, and helping itself to the value because the datum had arrived would have widened the rule under cover of a repair. PB §6.3 now writes `xfail` **or** `skipped` in both rows, which is the order this corpus requires: §6.3 states the predicate and this document supplies the value. A reader who sees `out` on a `base` record should infer exactly those two exempt values and no third — `failed`, `error`, `xpass`, `unknown` and `absent` on `B` all leave an id's non-passing `result` record a G8 **and** a G1 finding, exactly as before.

---

## 19. Out of scope

Deliberately not specified here, and where it belongs instead:

- **`freeze=`, the `Spine-Frozen` line grammar, and their ordering.** §2.10 fixes the closure as a set and stops. `envelope-vectors.md` owes the rendering, the `git ls-tree` quoting rule and the sort, and §17 D5 says why no vector can be published until it does.
- **`C-T1`/`C-T2` list grammar** — how a constitution line splits into patterns, how whitespace is handled, what order they yield in: `constitution.md`, which adopts `intent-doc.md` §6.1–§6.3 as §2.4 does, so that one dialect serves both. `gate-report.md` §5.4.1 already records `c_t1` and `c_t2` as `esc`-encoded arrays in file order.
- **The touchpoint grammar and the pattern dialect itself** — how "Expected to change: src/billing/, api/invoices.ts" becomes a list of entries, and what each entry matches: `intent-doc.md` §6, which owns the dialect §2.4 adopts and publishes its own matching vectors at §9.5.
- **Nothing about the runner adapters is out of scope any more.** §11 discharges all six obligations of `result-file.md` §6.3 for all four v1 runners, and since version 3 the `B` collection command for all four as well (§11.1–§11.5, §18 OPEN-11 discharged). There is no `docs/spec/runner-adapters.md` and none is owed. swift-testing (`@Test`) has no v1 adapter and is detected rather than ignored (§11.5, §18 OPEN-8).
- **The result file** — `result-file.md`. This document never writes one, never reads one, and touches a test id only in §11 and §12.
- **The gate report** — `gate-report.md`. `policy.rules` records `c_t1`/`c_t2`; no report member holds a resolver's output, and the dependency runs through a `G8` status (`gate-report.md` §5.4.2).
- **The dump** — `dump.md`. `freezes` edges are derived from the approval's `Spine-Frozen` lines, never recomputed by the indexer, so a resolver disagreement is a G8 failure at approval and landing time and never a G10 failure.
- **G8's other clauses** — blob equality, harness-moved, the read-only-after-approval rule. This document supplies G8's *inputs* — the closure (§2) and, since version 3, `C-T3`'s predicate (§12.4) — and decides nothing about what G8 does with them: the wire's spelling and class, its overridability, whether it runs in warn mode and its place in `Spine-Gates` are PB §6.3's and `gate-report.md` §6.3's. **`C-T3`'s tree grep is no longer declined here.** Version 2's disclaimer covered it by name, which left the clause with no predicate in any document while PB §7.4 rested part of its isolation argument on it; PB §4.3 now assigns both sets to this document by name, and §12.4 is them.
- **G2, G7 and G14's use of the same patterns.** §2.4 adopts a dialect because the closure needs one; what those gates do with a match is theirs.
- **Any language outside the four.** PB §6.7 is explicit: *"A language absent from that list cannot be gated; adding one is a release, not a repo setting."* Java, Kotlin, Objective-C, Go, Rust, C# and single-file-component formats (`.vue`, `.svelte`) are `lang: none` and contribute nothing (§10 names what that costs, and Appendix A names what dropping Kotlin cost). Objective-C is the one whose *presence* is nonetheless detected: `lang("x.m")` is still `none` and it is still resolved by nothing, but a Swift target that holds it is refused outright rather than resolved as though it were not there (§7.3, `mixed-objc-target`).
- **Type checking, symbol resolution, call graphs, dead-code analysis, coverage.** The resolver answers one question per import site (§1). `exercises` edges come from coverage reports and are PB §6.2's, at v1.1.
- **Performance.** §2.9's base-tree pass is expensive; caching it is an implementation matter, provided the cache is derived, gitignored and rebuildable (PB §6.1's iron rule).
- **Diagnostics.** How a finding is presented, which file the tripwire lists first, what `spine review`'s packet shows: the CLI's output and PB §6.5.

---

## 20. Conformance checklist

A resolver conforms iff all of the following hold. Every item is mechanically checkable.

**Inputs**

1. The only inputs read are the seven of §2.1, and `S` is **derived** from them by §2.1.1 rather than supplied. No filesystem call, no network, no package manager, no interpreter, no environment variable, no clock — and no result file, runner, collected id or `verified_by` edge.
2. Every existence test and every resolution names `A` or `B` explicitly, and the two are never confused (§2.2).
3. Resolution configuration is extracted from `B`; `RC(lang, A)` is computed only to compare (§3.3) — with the single exception of item 3a.
3a. §7.3's two Objective-C tests are evaluated in **both** trees for their own sake, over the post-`exclude:` file set `F(t)` and the target call's argument labels; an entry or construct in either tree raises `lang-unclassifiable`, reason `mixed-objc-target`, repository-wide, and is decided before §3.3 Rule 2's comparison so that a branch-introduced entry is never reported as `rc-changed-on-branch`.

**Language and file**

4. `lang` is total, byte-exact and lowercase-only over the extension table of §3.1; `.d.ts` files take their override.
5. A file that is not valid UTF-8 contributes no edges and raises `file-not-utf8`. No encoding declaration, BOM or fallback encoding is honoured.
6. A file whose `lang ∉ langs` contributes no edges in either tree.

**Sites and dispositions**

7. Every recognized site has exactly one disposition from §3.2's four.
8. Every specifier that is not a simple string literal (§3.4 rule 5) is `unresolvable`, in every language that has string specifiers.
9. Every conditional construct contributes every branch (§3.7). No configuration, flag, platform or variant is read.
10. Re-export forms are import sites (§3.5); TypeScript's type-only forms are `type_only` and no other language has one (§3.6).
11. A resolved candidate whose tree entry is mode `120000` or under `160000` is `unresolvable`.
12. Each language's `unresolvable` reasons are drawn from its closed list (§4.7, §5.7, §6.7, §7.8) and no other reason is emitted.

**The closure**

13. `class` is evaluated by §2.5's five rows and is a function of the path alone.
14. The walk prunes at `EXCLUDED` and at `FROZEN_LEAF`, and only there.
15. Clause 3 uses the language's `AncestorConfig` basenames over the seed's full ancestor chain including the root and the seed's own directory; clause 4 covers every snapshot under a `C-T1` path whether or not a seed reaches it.
16. The output is a set of repository paths; the implementation imposes no order and none is required.
17. `closure_digest` over §13's four trees reproduces the four published values.

**Findings**

18. Every finding is from §2.11's closed list, with its stated kind.
19. `lang-unclassifiable` fires once per language, never once per file.
20. `closure-tripwire` fires iff row 3 of §2.5 fired, and carries the sorted list of excluded branch-created paths.
21. `--approve` refuses outright on `seed-outside-test-roots` and refuses without a human `reason=` on every tripwire.

**Determinism**

22. Two runs over one pair of trees produce the same set, in any walk order, on any host, with any working tree, in a bare clone or a checkout.
23. Reordering the entries of any tree, any `paths` map, or any `targets:` array changes nothing but `RC` equality where the order is part of the extracted value (§5.3's `paths` key order).
24. No value derives from a clock, a duration or a date.

**Patterns**

25. Every "matches" in this document is `intent-doc.md` §6.3's `match(P, p)`; no rival dialect is implemented, and §2.4.2's nineteen rows reproduce.

**Runner adapters**

26. Each of the four runners has the `runner` token of §11.1, and `kotlin`, `gradle` and `jest` are emitted by nothing.
27. `fn` is a prefix of `id` for every record, and equals `id` under `vitest`, `dart-test` and `swift-test`.
28. `dart-test` composes `id` as `<suite path>::<test.name>`, discards a `testDone` with no prior `testStart`, tests `skipped` before `result`, excludes `(setUpAll)`/`(tearDownAll)` from the `base` section, and enforces the root `group`'s `testCount` per suite.
29. `swift-test` takes ids from `swift test list --disable-swift-testing`, outcomes from the XCTest stream, maps an `Expected failure in` line to `xfail`, maps a second terminal line to `unknown`, and fails the job on `swift-testing-unsupported` or `ambiguous-test-class`. On `B` it runs both commands: the listing for the floor's membership, `swift test --disable-swift-testing` for each `base` record's `out`.
30. A duplicate composed id, and a repository path containing an adapter's `id → path` separator, each make the collector fail the job and write nothing (§11.6 rules 2 and 3).
31. `pytest`'s `B` **enumeration** is `pytest --collect-only` and `vitest`'s is `vitest run`; `vitest list` is used by nothing. Deselected ids are not in the floor, a collection error or a file that fails to load during a `B` **enumeration** is `base-collect-failed`, and no adapter reads a process exit status as its completeness signal.
31a. Every adapter supplies a `B` **outcome** for every id it puts in the floor (obligation 6): `vitest` and `dart-test` from the run that already enumerates it, `pytest` and `swift-test` from a second invocation of their own `T` command against the checkout of `B`. The mapping is the adapter's `T` mapping unchanged (§11.6 rule 4); an id the outcome run reported no terminal outcome for takes `out: "absent"`; a failed outcome run is not `base-collect-failed` and contributes no status; and every `B` invocation of either kind precedes every `T` execution.
31b. `xfail` is producible on `B` by `pytest` and `swift-test` alone and `skipped` by all four adapters, so `result-file.md` §8.5 clause 2's carve-out — one predicate over `xfail` **or** `skipped` — is reachable in all four suites, and its `xfail` limb in a Python or a Swift one and nowhere else (§11.6 rule 5).

**The seed set**

32. `S` is every path in `A` that a `C-T1` pattern matches and whose bytes carry a §12.1 pragma occurrence naming an AC in `AC` — and is computed from nothing else (§2.1.1).
33. §12.3's naming sugar seeds nothing, and neither does a pragma naming another intent or an AC number outside `AC`.
34. A path carrying such a pragma that no `C-T1` pattern matches — `C-T2`-only paths included — is `seed-outside-test-roots` and `--approve` refuses outright; an empty `S` is the `no-seed` tripwire.
35. `<intent-id>` in a pragma is `intent-doc.md` §3.1's, so `INT-42` and `INT-0042` are not occurrences; the AC digit run is captured as written and compared against `intent-doc.md` §5.3's spelling, so `AC-01` and `AC-9` are occurrences that name nothing.

**`C-T3`**

36. `ct3(p)` is evaluated over `T`, for every `p` with `H(p)` **false** — `C-T1 ∪ C-T2`, never `C-T1` alone — against §12.4.1's closed framework sets and §12.4.2's closed hook basenames and token sequences, and against no others.
37. A type-only framework import is not a hit; a `@testable import` in a non-harness Swift file is; `unittest.mock` is exempt and `unittest` is not; a hook basename is tested by path and never by content.

---

# Appendix A — Kotlin, and why v1 does not ship it

**Not normative.** Nothing in this appendix is part of the v1 contract. `kotlin` is not a legal `params.langs` value, `gradle` is not a legal `runner` token, `.kt` and `.kts` are `lang: none` (§3.1), and no conforming implementation reads a word of what follows. It is kept because the analysis was done, it is correct, and a release that adds Kotlin should start from it rather than from nothing.

**Why the language was dropped, stated once.** Version 1 rated Kotlin as meeting the bar *within the conventional Gradle subset*, with two qualifications that turned out to be very different in kind.

- **The Gradle subset fails loudly.** A `build.gradle(.kts)` containing any of `sourceSets`, `srcDir`, `srcDirs`, `productFlavors` and the rest is detected by one lexical scan, raises `lang-unclassifiable` with reason `layout-overridden`, and `--approve` refuses without a human `reason=`. A repository outside the subset knows it.
- **The Java hole fails silently, and it is the ordinary case.** A `.java` file in a resolved Kotlin source set is `lang: none`: it contributes no edges and is the target of none. Kotlin's same-package visibility means a test file reaches a Java helper **with no import line at all**, so an oracle written in Java is invisible to the closure — not excluded, not counted, not reported. Mixed Kotlin/Java modules are the normal shape of a JVM codebase, not an exotic one.

Version 1's §18 OPEN-1 offered three exits — detect and refuse (`mixed-jvm-module`), add Java as a sixth language, or drop Kotlin — and recommended the first. The owner chose the third on 2026-08-26. The reasoning is worth keeping: a guarantee whose failure mode is *nothing happens and nobody is told* cannot ship, and detect-and-refuse would have fired on the ordinary shape rather than the exotic one, so it buys a loud refusal for most of the repositories the language exists to serve. Dropping is the honest version of the same answer.

**Swift's Objective-C hole is the same rule reaching the other outcome, not a different judgement** (§18 OPEN-2, closed 2026-08-27). The rule admits two conforming answers — remove the language, or make the hole loud — and which one it selects turns on whether the refusal would fire on the ordinary shape or the exotic one. For Kotlin, mixed Kotlin/Java is the ordinary shape, so detect-and-refuse would have refused most of the repositories the language exists to serve and removal was the honest answer. For Swift, a package that compiles C-family sources into a gated target is the exotic shape, so `mixed-objc-target` refuses it and leaves the rest gatable. **Neither language ships a silent hole**, which is the invariant; the language set is the variable.

**What a later release owes.** The four sections below are version 1's text, unaltered but for section numbering. A release adding Kotlin needs, on top of them: a resolution for the Java hole (a sixth language, or a refusal on §7.3's `mixed-objc-target` model, which is the precedent OPEN-2 set on 2026-08-27); the `gradle` adapter to §11's standard (§A.5); the `closure-too-large` threshold question of §18 OPEN-6, which Kotlin's source-set-shaped closures raise as sharply as Swift's module-shaped ones; and PB §4.3's closure-tripwire remedy, which §17 D10 records as non-mechanical for Kotlin because moving a file into `src/test/` moves it into the test compilation.

## A.1 The Kotlin resolver, as version 1 specified it

### A.1.1 Lexing

- **Comments:** `//` to end of line, `/* … */` (**nested**).
- **String literals:** `"…"`, `"""…"""`. Interpolation is `$`. No import specifier is a string, so simplicity does not arise.
- **Anchor:** a `word` token `import` not immediately preceded by `.`; likewise `package`.

### A.1.2 The import forms

| Form | Names |
|---|---|
| `import a.b.C` | the dotted name `a.b.C` |
| `import a.b.C as D` | the dotted name `a.b.C` |
| `import a.b.*` | the package `a.b` |
| `package a.b` | not an import — it records the file's package, which §A.1.4 needs |

Kotlin has no re-export, no type-only import, no dynamic import and no string specifier. A `typealias` is a declaration, not an import.

### A.1.3 `RC(kotlin, tree)`

`RC` is a set of modules, each `(dir, sourceSets)` where `sourceSets` maps a source-set name to a set of directories, plus a set of inter-module project dependencies.

1. **Projects.** From `settings.gradle.kts` or `settings.gradle` at the repository root: every `include(` *simple string literal* `)` and `include ` *simple string literal*, one project each. A settings file containing any of the tokens `includeBuild`, `rootProject.children`, `projectDir`, `for`, `while`, `forEach`, `file(`, `File(`, `apply`, or a `include` argument that is not a simple string literal → unclassifiable, reason `settings-not-declarative`. No settings file, but a `build.gradle(.kts)` at the root → a single root project. Neither → unclassifiable, reason `no-gradle-build`.
2. **Project directories.** `:a:b` → `a/b`, relative to the repository root. The root project is the repository root iff a root `build.gradle(.kts)` exists.
3. **Layout override detection.** Each project's `build.gradle.kts` / `build.gradle` is scanned lexically. If it contains any of the tokens `sourceSets`, `srcDir`, `srcDirs`, `kotlin.srcDir`, `java.srcDirs`, `sourceSets.getByName`, `androidComponents`, `productFlavors`, `flavorDimensions` → unclassifiable, reason `layout-overridden`. This is deliberately blunt: a build script that touches source-set layout at all makes the convention mapping below a *silent* wrong answer, and a silent wrong answer is the one outcome this document may not produce.
4. **Source sets by convention.** For a project directory `d`, every directory `d/src/<S>` names source set `S`, whose directories are `d/src/<S>/kotlin` and `d/src/<S>/java` (Kotlin files under `java/` are ordinary in Gradle projects and are included). Files of source set `S` are every `.kt` file under those directories, excluding recognized Gradle build scripts (§3.1).
5. **Project dependencies.** Every `project(` *simple string literal* `)` occurrence in a project's build script contributes a dependency on the named project. A build script that is unclassifiable under rule 3 never reaches this rule.

### A.1.4 What a Kotlin file imports — the visible set, and why it is a union

Kotlin's hard cases are not the import syntax. They are that (a) files in the same package see each other with **no import at all**, across modules as well as within one, and (b) which files are on a test compilation's classpath depends on the source set graph, which for Kotlin Multiplatform is a lattice of `commonMain`, `jvmMain`, `nativeMain` and their test counterparts, resolved per target platform.

The **visible set** `V(f)` of a Kotlin file `f` in project `P`, source set `S`:

1. every file of `S` in `P`;
2. every file of **every non-test source set** of `P` — `main`, `commonMain`, `jvmMain`, `nativeMain`, `androidMain`, all of them, whether or not the platform they target is the one the tests run on;
3. for every project `Q` that `P` depends on, the same two sets computed in `Q`;
4. transitively over rule 3.

A source set is a **test** source set iff its name is `test` or ends with `Test` or `Tests` (`test`, `androidTest`, `jvmTest`, `commonTest`, `iosTest`); every other source set is non-test. This is also what makes `H` correct for Kotlin: `C-T1`'s scaffolded pattern (§A.1.6) matches the same set of directories.

**Rule 2 is the union rule of §3.7 applied to source sets, and it dissolves `expect`/`actual`.** An `expect` declaration in `commonMain` and its `actual` in `jvmMain` and `nativeMain` are three files declaring the same package; taking every non-test source set puts all three in `V(f)`, so all three are reached and classified, with no target platform read anywhere. Resolving under "the test configuration" would have required knowing which platform the tests run on, which is not in the tree.

Given `V(f)`, the edges out of `f` are:

- **implicit same-package:** every `g ∈ V(f)` whose `package` declaration equals `f`'s;
- **explicit:** for each `import` site naming a dotted name `n₁.….n_k`, every `g ∈ V(f)` whose `package` declaration equals `n₁.….n_k` (the wildcard and the `import a.b.*` case) **or** equals `n₁.….n_{k−1}` (the common case, where the last component is a class or a top-level function). No file matching either → `external`.

The two-candidate rule over-approximates: `import a.b.C` where a package `a.b.C` also exists pulls in both. That is the safe direction and it costs a handful of frozen files in the rare repositories that have a package and a class of the same dotted name.

A file with no `package` declaration is in the root package; it matches other root-package files in `V`.

**Generated code is `external`, and that is correct.** `import com.example.databinding.FooBinding` names a package no file in the tree declares, so it is `external` and raises nothing. Room, Dagger, KSP, kapt, protobuf and view binding all land there. An oracle cannot: it is a file in the tree, in a source set, declaring a package, and rule 2 puts it in `V`.

### A.1.5 `AncestorConfig(kotlin)` — clause 3 basenames

`build.gradle.kts`, `build.gradle`, `settings.gradle.kts`, `settings.gradle`, `gradle.properties`, `libs.versions.toml`.

`gradle.properties` is included because it can set `systemProp` values and JVM args that change what the test task collects. `libs.versions.toml` (Gradle's version catalog) pins every dependency version and is the Gradle analogue of a lockfile.

### A.1.6 Scaffolded `C-T2` patterns

```
**/src/testFixtures/**
**/build.gradle
**/build.gradle.kts
settings.gradle
settings.gradle.kts
gradle.properties
gradle/libs.versions.toml
```

`C-T1`'s default is `**/src/test/**`, `**/src/*Test/**`, `**/src/*Tests/**`.

### A.1.7 Snapshot patterns (clause 4)

Final component matching `*.snap`, `*.approved.txt`, `*.golden`; or any path with a directory component named `__snapshots__` or `snapshots`, or a path under `**/src/test/resources/**`.

### A.1.8 The closed unclassifiable list

Language level: `settings-not-declarative`, `no-gradle-build`, `layout-overridden`, `rc-changed-on-branch`.

Site level: `symlink-or-submodule`. As with Swift, a Kotlin `import` either names files in the visible set or is `external`.

## A.2 What Kotlin could guarantee — version 1 §10's row, verbatim

| Language | Meets the bar | Under what restriction | Residual that does not close |
|---|---|---|---|
| **Kotlin** | **yes, within the conventional Gradle subset** | declarative `settings.gradle`; no `sourceSets` customization anywhere; no Android variant source sets | **Mixed Kotlin/Java modules.** A `.java` file in a Kotlin source set is `lang: none`, contributes no edges, and is not a target of any edge — so an oracle written in Java is invisible to the closure, and same-package visibility means the test needs no import to reach it. This is the largest hole in the five and it is not narrow: Kotlin/Java mixed modules are ordinary. It is why the language was dropped (§18, OPEN-1, now closed). |

## A.3 The Kotlin worked example — version 1 §13.5, verbatim

*(closure and digest reproduced; the `closure_digest` below was recomputed and is unchanged)*

`C-T1`: `**/src/test/**`, `**/src/*Test/**`, `**/src/*Tests/**`. `C-T2`: §A.1.6's list. `expected`: `app/src/main/kotlin/com/example/billing/`. Seed: `app/src/test/kotlin/com/example/billing/InvoiceTest.kt`.

`settings.gradle.kts` is `include("app")`; `app/build.gradle.kts` contains no layout-override token.

`B` = `A`:

```
settings.gradle.kts
build.gradle.kts
app/build.gradle.kts
app/src/main/kotlin/com/example/billing/Invoice.kt      package com.example.billing
app/src/main/kotlin/com/example/billing/Rates.kt        package com.example.billing
app/src/main/kotlin/com/example/shared/Money.kt         package com.example.shared
app/src/test/kotlin/com/example/billing/InvoiceTest.kt  package com.example.billing
                                                        import com.example.shared.Money
                                                        import com.example.support.Fixtures
                                                        import org.junit.jupiter.api.Test
app/src/test/kotlin/com/example/billing/Factories.kt    package com.example.billing
app/src/test/kotlin/com/example/support/Fixtures.kt     package com.example.support
```

`V(InvoiceTest.kt)` = the `test` source set of `app` ∪ every non-test source set of `app` (here, `main`).

| Edge | Target | class |
|---|---|---|
| implicit same-package `com.example.billing` | `app/src/test/kotlin/com/example/billing/Factories.kt` | `FROZEN_WALK` |
| | `app/src/main/kotlin/com/example/billing/Invoice.kt` | `EXCLUDED` — inside `expected`, at `base`, and `Rates.kt` (non-test, same package) implicitly imports it |
| | `app/src/main/kotlin/com/example/billing/Rates.kt` | `EXCLUDED`, symmetrically |
| `import com.example.shared.Money` | `app/src/main/kotlin/com/example/shared/Money.kt` | `FROZEN_LEAF` |
| `import com.example.support.Fixtures` | `app/src/test/kotlin/com/example/support/Fixtures.kt` | `FROZEN_WALK` |
| `import org.junit.jupiter.api.Test` | — | `external` — no file in `V` declares `org.junit.jupiter.api` or `org.junit.jupiter.api.Test` |

Clause 3 adds `settings.gradle.kts`, `build.gradle.kts`, `app/build.gradle.kts`. Clause 4 adds nothing.

Closure — 7 paths:

```
app/build.gradle.kts
app/src/main/kotlin/com/example/shared/Money.kt
app/src/test/kotlin/com/example/billing/Factories.kt
app/src/test/kotlin/com/example/billing/InvoiceTest.kt
app/src/test/kotlin/com/example/support/Fixtures.kt
build.gradle.kts
settings.gradle.kts
```

`closure_digest` = `sha256:373236c6b64ad1a5ed2d44ba1d3e099de5f5cbd379fea5f766a2c4d1b93f7237` (over 281 canonical bytes).

`Factories.kt` is reached with no import line, by the same-package rule. If `app/build.gradle.kts` had contained a `sourceSets { … }` block, `RC` would be unclassifiable with reason `layout-overridden`, this closure would contain only the clause-3 files, and `--approve` would raise `lang-unclassifiable` and refuse without a human reason.

## A.4 The Kotlin conformance cases — version 1 §14, verbatim

**Kotlin**

| # | Case | Required |
|---|---|---|
| K1 | Two files in one source set with the same `package` and no imports | each is an edge target of the other |
| K2 | A test file and a main file with the same `package`, different source sets | an edge; the main file is classified normally |
| K3 | `import a.b.C` where a file declares `package a.b` | that file is a target |
| K4 | `import a.b.C` where a file declares `package a.b.C` | that file is also a target (both candidates, §A.1.4) |
| K5 | `import a.b.*` | every file declaring `package a.b` |
| K6 | `import org.junit.jupiter.api.Test` with no such package in the tree | `external` |
| K7 | An `expect` in `commonMain` and `actual`s in `jvmMain` and `nativeMain` | all three are in `V` for a `commonTest` file; no platform is read |
| K8 | `build.gradle.kts` containing `sourceSets {` | `lang-unclassifiable`, reason `layout-overridden` |
| K9 | `settings.gradle.kts` containing `include(*names)` | `lang-unclassifiable`, reason `settings-not-declarative` |
| K10 | A `.kt` file under `src/main/java/` | included in the `main` source set |
| K11 | `build.gradle.kts` itself | never walked for imports — version 1 §3.1's Gradle-build-script override, removed from §3.1 with the language |
| K12 | A `.java` file in a resolved source set | `lang: none`, no edges — the documented hole (§A.2), and the reason the language was dropped |

## A.5 The `gradle` adapter

Version 1 §11.4 declined to fix `gradle`'s `id → fn`, `id → path` and outcome mapping, on the ground that the JUnit Platform's `UniqueId` and its `test-template-invocation` segments are a permanent identifier that must not be written from memory. That refusal stands and is now moot: with Kotlin dropped there is no v1 language whose runner is Gradle. The token `gradle` is **reserved** (§11.1) so that a later release adding Kotlin — or Java — finds it free, and the work that release owes is exactly §11's, for one more runner: a token, a total `id → fn` whose output is a prefix of its input, a total `id → path`, an outcome mapping onto `result-file.md` §5's eight values, and a conforming transport, each ratified against the reporter's real output rather than its documentation.
