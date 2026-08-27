# 06 — The result file: encoding, header line, record kinds, ordering, outcome vocabulary

Concern owner sheet. Everything below is extracted from `docs/spec/result-file.md` (**RF**) unless another file is named.
Citations are `(RF §x.y)`. Requirements are marked **[R-NN]** and typed **MUST / MUST NOT / REFUSE / SHOULD**.
`REFUSE` is used only where the corpus defines a refusal with a named status token, a named finding label, or an exit-status rule.

**A note on scope.** This sheet owns the *grammar* — bytes, header, records, ordering, vocabulary, identity. It does **not** own: the collector's step ordering and isolation mechanism (RF §7.1), the ingestion order and preconditions (RF §8, summarised here only where the grammar's error labels live), per-language runner tokens/grammars (`import-resolver.md`), or the gate report (`gate-report.md`). See *Cross-references*.

---

## Sources read

| File | Section | Lines |
|---|---|---|
| `/Users/thettwe/Works/spine-kit/docs/spec/result-file.md` | Front-matter + amendments (spec version 3, five amendment paragraphs) | 1–13 |
| " | §1 Scope (five governing constraints) | 15–29 |
| " | §2 Position in the pipeline (the writer/reader table) | 31–44 |
| " | **§3 Path and naming** (assigned, full) | 46–60 |
| " | **§4 File format — §4.1 encoding/framing, §4.2 header line, §4.3 canonical JSON, §4.4 record kinds, §4.5 ordering** (assigned, full) | 62–169 |
| " | **§5 The outcome vocabulary** (assigned, full) | 171–199 |
| " | **§6 Runners, test ids and the roll-up — §6.1–§6.7** (assigned, full) | 201–296 |
| " | §7.2 Reduction, §7.3 status contributions + the fold, §7.4 outside `--ci` (read for the `end.status` domain, which §4.4 delegates) | 420–459 |
| " | §8.1–§8.6 ingestion (read for the malformed/missing labels the grammar's errors map to, and for the one consumer of `base.out`) | 461–571 |
| " | **§10 Worked example** (assigned, full, verbatim below) | 614–718 |
| " | §11 Conformance checklist, §12 Out of scope, §13 R1–R35, §14 OPEN-1…OPEN-9 + the three §11 amendments | 719–833 |
| `/Users/thettwe/Works/spine-kit/PLAYBOOK.md` | §11 *Files and refs* — the `results/<T>.jsonl` clause (header grammar as PB fixes it) | 1012 |
| " | §11 `Spine-Test` payload row | 1001 |
| " | §11 `Spine-Seal` row (`profile=` seal domain incl. `n/a`) | 1010 |
| " | §4.3 approval example (`Spine-Test: vitest …`), §6.3 G1 *Skipping is modifying* | 324–325, 338 |
| `/Users/thettwe/Works/spine-kit/docs/spec/README.md` | `result-file.md` row (status v3, OPEN set) | 15, 49, 79, 85, 88 |
| `/Users/thettwe/Works/spine-kit/docs/spec/gate-report.md` | `evidence.result_sha256` member; the fabricated-identity enumeration for vector A | 580–616, 741, 812–887 |
| `/Users/thettwe/Works/spine-kit/docs/spec/manifest.md` | published `dist_hash` `6f49644f…` over the 529-byte artifact list | 102, 1145–1158 |
| `/Users/thettwe/Works/spine-kit/docs/spec/envelope-vectors.md` | the disclosed `tool=` divergence table | 1020–1030 |

---

## Data model

### 0. The file as a whole

| Property | Value | Cite |
|---|---|---|
| Path | `.spine/cache/results/<T>.jsonl` | RF §3 |
| `<T>` | tree oid of the synthetic merge the untrusted job computed itself, `git merge-tree --write-tree origin/<trunk> H`; **"lowercase hex, full length, never abbreviated"** — 40 chars under `object_format: sha1`, 64 under `sha256` | RF §3 |
| Extension | **"exactly `.jsonl`"**; stem carries "no prefix, suffix, branch name or intent id" | RF §3 |
| Cardinality | **One file per `T`, covering every runner.** No per-runner file, no per-runner directory, no per-language suffix | RF §3 |
| Encoding | UTF-8, no BOM | RF §4.1 |
| Line 1 | the header line — **"It is not JSON."** | RF §4.1 |
| Lines 2..n | one canonical JSON object each, JSONL | RF §4.1–§4.3 |
| Last line | the single `end` record | RF §4.5 |
| Size caps | **"There is no size cap and no line-length cap. Readers stream."** | RF §4.1 |
| Signed? | No — never a signature, MAC or envelope over the file | RF §2, §12 |
| Published? | Never. Only the gate report's `evidence.result_sha256` over these bytes reaches `refs/notes/spine` | RF §2, §12; `gate-report.md` §5.9 |
| Spec version | 3 (document metadata; **not written into the file**, not compared by any gate) | RF front-matter, §13 closing note |

### 1. Header line — six fields, fixed order

Verbatim grammar (RF §4.2):

```
tree=<oid> base=<sha> tool=<version>+sha256:<hex64> keys_visible=<bool> profile=<profile> ids=<n>
```

**"Fixed by §11. Six fields, `key=value`, separated by exactly one `U+0020`, in this order"** (RF §4.2).

| # | Key | Type | Domain / grammar | Default | Required | Produced from |
|---|---|---|---|---|---|---|
| 1 | `tree` | string | lowercase hex object id, length per `object_format` (40 or 64) | none | yes | the `T` the untrusted job computed itself |
| 2 | `base` | string | lowercase hex object id, length per `object_format` | none | yes | `origin/<trunk>` tip at the moment the collector read policy — **for a reseal, the seal's `base=`** |
| 3 | `tool` | string | `<version>` `+` `sha256:` `<64 lowercase hex>` | none | yes | the collector's **own** embedded version and artifact-list hash |
| 4 | `keys_visible` | bool literal | `true` \| `false` | none | yes | the key-material predicate (below), over the collector's own environment **and every runner's** |
| 5 | `profile` | enum | `container` \| `uid` \| `none` | none | yes | the boundary the collector **achieved** — created *and* tested |
| 6 | `ids` | integer literal | **"non-negative decimal, no sign, no leading zero except `0` itself"** | none | yes | the number of `base` records that follow |

- `<version>` is trunk's `cli.version` **string verbatim**: "non-empty, every character in `U+0021`–`U+007E` (printable ASCII, the space already excluded by the field rule above). **No version is unrepresentable.**" (RF §4.2)
- `profile=n/a` is a **seal** value only; a *header* carrying it is malformed (RF §4.2, §13 R15). PB §11's `Spine-Seal` row admits `container|uid|none|n/a`; the header row admits three.
- `uid` **is never written by a v1 collector** — v1 ships no mechanism for it and refuses the request instead (RF §4.2, §7.1, §11 item 16).
- **`params.timeout` is not a header field** (RF §4.2). No timestamp, duration, or wall-clock-derived ordinal exists anywhere in the file (RF §1, §12, §11 item 13).

### 2. `base` record — one per `(runner, id)` pair collected on `B`

Verbatim shape (RF §4.4):

```json
{"id":"<runner-native id>","out":"<outcome on B>","path":"<repo-relative path>","runner":"<runner token>","t":"base"}
```

| Field | Type | Domain | Required | Notes |
|---|---|---|---|---|
| `id` | string | non-empty; runner-native, **"including any parametrization suffix"** | yes | as collected on a checkout of `B` |
| `out` | string | the eight §5 values **plus `absent`** | yes | `absent` is legal here **and on no other kind** |
| `path` | string | repo-relative, `/`-separated, **"byte for byte as git stores it in `B`'s tree"**; **empty string** where no tree entry matches | yes | may be `""` |
| `runner` | string | `[a-z][a-z0-9_-]{0,31}`, non-empty | yes | collector constant |
| `t` | string | literal `"base"` | yes | discriminator |

- **No `fn` on a `base` record**: "the `B` floor matches on full ids within a runner, never by roll-up" (RF §4.4, §6.5).
- **Key order on the wire is `id, out, path, runner, t`** — ASCII ascending (RF §4.3 rule 2).

### 3. `result` record — one per `(runner, id)` pair a runner reported on `T`

Verbatim shape (RF §4.4):

```json
{"fn":"<function id>","id":"<runner-native id>","out":"<outcome>","path":"<repo-relative path>","runner":"<runner token>","t":"result"}
```

| Field | Type | Domain | Required | Notes |
|---|---|---|---|---|
| `fn` | string | non-empty, **and a prefix of `id`**; equal to `id` when the id is not parametrized | yes | computed by the collector |
| `id` | string | non-empty, runner-native, full incl. parametrization suffix | yes | |
| `out` | string | exactly one of the **eight** §5 values — **`absent` is an unknown value here, hence malformed** | yes | |
| `path` | string | repo-relative, resolved against **`T`'s** tree, emitted as `T`'s bytes; empty string if no tree entry | yes | may differ from the same pair's `base.path`; **neither record is rejected** |
| `runner` | string | `[a-z][a-z0-9_-]{0,31}` | yes | must equal the matched `base` record's `runner` — "definitional rather than a check" |
| `t` | string | literal `"result"` | yes | |

- Wire key order: `fn, id, out, path, runner, t`.

### 4. `end` record — exactly one, the last line

```json
{"status":"<status>","t":"end"}
```

`status` domain (closed; from RF §7.3's table, in **table order**, which is also the fold order):

| # | `status` | Contributed when | That runner contributes |
|---|---|---|---|
| 1 | `complete` | The runner terminated of its own accord and its stream was parsed to a terminal event. | Its full `base` and `result` records. |
| 2 | `base-collect-failed` | The **enumeration** of the id set on the checkout of `B` failed, or its deadline expired during that enumeration. A failure of the separate `B` **outcome** run is *not* this row. | **Nothing, from any runner** (all-or-nothing). |
| 3 | `spawn-failed` | The runner could not be started at all. | Its `base` records; no `result` records. |
| 4 | `no-output` | The runner started and terminated but emitted no parsable stream event. | Its `base` records; no `result` records. |
| 5 | `stream-invalid` | Its stream contained an event the adapter cannot parse, or an id that is not valid UTF-8. | Its `base` records; no `result` records. |
| 6 | `runner-failed` | The runner terminated abnormally, or its stream ended mid-record. | Its `base` records; every `result` record parsed before the break. |
| 7 | `runner-timeout` | The collector's deadline expired on the `T` run and it killed that process group. | Its `base` records; every `result` record parsed before the kill. |

- Rows are evaluated **"top to bottom, first match wins"** per runner (RF §7.3).
- `complete` additionally requires **both** that the adapter parsed that runner's terminal session-end event **and** that no member of its process group was terminated by a signal (RF §7.3, §13 R23).
- **Exit codes and signals are never recorded and never the discriminator** (RF §4.4, §7.3, §12).
- **There is no per-runner status record** and no per-runner section (RF §4.4, §12, §13 R28).

### 5. The `runner` token

- Grammar, verbatim: **"`[a-z][a-z0-9_-]{0,31}` — one lowercase ASCII letter followed by up to 31 of lowercase letters, digits, `-` and `_`. A value outside it is malformed."** (RF §4.4)
- **No uppercase** — "so byte order and case-insensitive order coincide and two spellings of one runner cannot both exist."
- **No `U+0020`** — because `Spine-Test`'s payload is `<runner>` `U+0020` `<function id>` and function ids may contain spaces, so the split must be at the **first** space.
- **No `U+003A` (`:`)** — because a `test` node id is `test:<runner>:<id>` (PB §6.2) and only a colon-free token makes that split exact.
- **"The token is a constant of the collector's adapter, embedded in the pinned release. It is never read from the runner's stream, from the repository's configuration, from `params.langs`, or from the environment."** (RF §4.4)
- The **set** of tokens is `import-resolver.md`'s, not this file's (RF §6, §6.4, §12).
- Reserved and assignable to nothing else: `kotlin` (language), and `gradle`, `junit`, `kotest` (runner tokens) (RF §6.4, §13 R27, §14 OPEN-2).

### 6. The outcome enum (`out`) — closed, eight values, runner-independent

| `out` | Meaning (verbatim) | G1 |
|---|---|---|
| `passed` | "The runner ran the id and reported it passed, with no expected-failure marker in play." | **pass** |
| `failed` | "The id ran and its assertion phase failed." | not a pass |
| `error` | "The id could not run, or failed outside its assertion phase: a collection error, a setup or teardown error, an import failure." | not a pass |
| `skipped` | "The id was collected and not run, by a marker, a runtime skip, or an environment condition." | not a pass |
| `xfail` | "The id was declared an expected failure and did not pass." | not a pass |
| `xpass` | "The id was declared an expected failure and passed." | not a pass |
| `deselected` | "The id was collected and excluded before running — a selection expression, a collection hook." | not a pass |
| `unknown` | "The runner reported a terminal outcome the collector's adapter does not map." | not a pass |

Plus the **ninth, `base`-only** value:

| Value | Meaning | Where legal |
|---|---|---|
| `absent` | **"the `B` outcome run reported no terminal outcome for the pair"** — "It is not `unknown`, and the two must not be merged: `unknown` is a terminal report the adapter could not map, `absent` is no terminal report at all" | `base` records only; on a `result` record it is an unknown value, hence **malformed** |

---

## Algorithm

### A. Writing the file (collector side)

**[R-01] MUST** — The collector **MUST** create `.spine/cache/results/` itself, write to a temporary file in the same directory opened `O_CREAT|O_EXCL` "under a name no other process can predict", `fsync` it, and `rename()` it over `<T>.jsonl`, replacing any file already there (RF §3).

**[R-02] MUST NOT** — A pre-existing file at the final path **MUST** be "overwritten without comment". The collector **MUST NOT** refuse on a path that already exists: "nothing in this design may remember that a previous attempt happened" (RF §3, §13 R18, §12).

**[R-03] MUST** — The file **MUST** be written **once, after every runner has been reaped**. "There is no append, no per-runner flush and no partial publish." The union is atomic with the rename (RF §3).

**[R-04] MUST** — The filename stem **MUST** equal the header's `tree=` value **byte for byte**. "A file whose stem and header disagree is malformed" (RF §3).

**[R-05] MUST** — Emit, in this order and no other (RF §4.5):
1. Header line.
2. Every `base` record, sorted ascending by the **bytes** of `runner`, then by the **bytes** of `id`.
3. Every `result` record, sorted ascending by the bytes of `runner`, then by the bytes of `id`.
4. The `end` record.

**[R-06] MUST** — `ids=` **MUST** equal the number of `base` records that follow — under several runners the cardinality of the set of **`(runner, id)` pairs**, not of bare id strings (RF §4.2, §13 R2). Worked instance: "a repository whose pytest and vitest suites each collect an id spelled identically contributes **two** `base` records and **two** to `ids=`."

**[R-07] MUST** — `tool=` **MUST** be what the collector **is**, never what trunk pins: "Copying the manifest's value would assert nothing." Equality with trunk's manifest is the trusted stage's check (RF §4.2, §8.3 step 2).

**[R-08] MUST** — `tool=` **MUST** be spelled exactly as the seal's `tool=` (PB §11 `Spine-Seal`), "so the two compare by byte equality and an auditor can read them side by side" (RF §4.2).

**[R-09] MUST** — `keys_visible=false` **MUST** assert that "**no signing key material of any kind** was reachable from the collector process or from any process group it spawned — *every* runner invocation included: no variable named `SPINE_PIPELINE_KEY` … nor any provider-specific pipeline-key name that `ci.md` fixes, **and** no signing agent or private key — `SSH_AUTH_SOCK`, `GPG_AGENT_INFO`, a readable `~/.ssh` or `~/.gnupg`". One assertion covers the whole job; **it is not per-runner**, and "a collector that strips key material for one runner and not another writes `true`" (RF §4.2, §13 R11).

**[R-10] MUST** — Outside `--ci` (solo path) the collector **MUST** write `keys_visible=true` and `profile=none`, settled before any observation (RF §7.4, §13 R11). It attempts no boundary and **refuses nothing**: "a manifest declaring `uid` costs a solo developer no run" (RF §7.4).

**[R-11] MUST** — `profile=` **MUST** name the boundary the collector achieved by creating it **and testing it**, never what configuration claims (RF §4.2, §7.4 rule 3). `container` is licensed only by P1–P4 passing (RF §7.1, §11 item 16); `uid` is a **REFUSE** at step 1 with no file written; `none` in every other case.

**[R-12] MUST** — One `result` record per distinct `(runner, id)` pair; where a runner reports an id more than once, **"the last terminal outcome that runner reported for that id wins"**. "Reduction never crosses runners" (RF §7.2, §13 R6).

**[R-13] MUST** — `base` pairs are a **set**: duplicates at collection reduce to one record, per runner (RF §7.2).

**[R-14] MUST** — Precedence when deriving `out`: **"Precedence is phases plus polarity, never the transport's own outcome word."** "A strict expected-failure that passes is `xpass` here even where the runner itself reports it as `failed`." Normative for **every** adapter (RF §6.6).

**[R-15] MUST** — "**A collection error that yields no item id** is recorded as one `error` record whose `id` and `fn` are the runner's own id for the failing collector — for pytest, the file's nodeid — and whose `path` is that file, under that runner's token." Ids from the `B` set inside that file are then **absent** (RF §6.6).

**[R-16] MUST NOT** — A **whole-module skip** that is neither an item report nor a collection error "collects nothing and errors nothing, so its ids are simply absent… the collector records nothing it did not observe." The collector **MUST NOT** synthesise a record for it (RF §6.6).

**[R-17] REFUSE (status token)** — An id whose bytes are not valid UTF-8 is not representable: **that runner contributes no `result` records at all** and its status contribution is `stream-invalid`; other runners contribute theirs, and the fold makes the file's `status` `stream-invalid` (RF §7.2, §12).

**[R-18] MUST** — `end.status` **MUST** be the fold: **"`complete` iff every invoked runner contributed `complete`. Otherwise it is the first row in this table's order, after `complete`, contributed by any runner."** The fold is over the table's fixed order, "not over invocation order or wall time" (RF §7.3, §13 R28).

**[R-19] REFUSE (status token) / MUST NOT** — **Collection on `B` is all-or-nothing across runners.** "If *any* invoked runner's collection on `B` fails, the file's `status` is `base-collect-failed`, `ids=0`, and **no `base` and no `result` records are written at all**, from any runner." Here `ids=0` means *no `base` records follow* (RF §7.3, §13 R13).

**[R-20] MUST NOT** — A failed `B` **outcome** run is **not** a status and **not** all-or-nothing: the `base` section stays whole and every id the outcome run did not report gets `out: "absent"`; `end.status` does not move (RF §7.3, §4.4).

**[R-21] MUST (exit status)** — "The collector's exit status is non-zero for every `status` other than `complete`, and for `keys_visible=true`" — but the file **MUST** still be written: "a failed job that produced no file and a failed job that produced an honest one are different things" (RF §7.3).

**[R-22] MUST** — The collector **MUST** always write a file once `T` is known and policy has been read, and **"The header is always complete and honest."** (RF §7.3)

### B. The invocation set (which runners write into one file)

**[R-23] MUST** — The invocation set is **"a total function of trunk's `params.langs` and the pinned release, and of nothing else"** (RF §6.2). `params.langs` is read from `origin/<trunk>` — "never from the checkout, never from the candidate."

**[R-24] MUST** — "The collector invokes **every** member of that set, in full, **with no selection argument**. A collector that skips one narrows the floor exactly as a `-k` would, and is non-conformant for the same reason." (RF §6.2)

**[R-25] MUST** — `params.langs` and `params.timeout` are read from trunk **on the solo path too**: "A solo developer's laptop does not choose its own invocation set or its own deadline" (RF §7.4).

Three fail-closed consequences (RF §6.2), each normative:
- A frozen `Spine-Test` naming a runner **outside** the invocation set can never pass — `P(R, F)` empty ⇒ absence ⇒ not a pass ⇒ G1 fails. **"There is no silent skip."**
- An AC whose only `verified_by` edges name such a runner **fails §8.5 clause 3**.
- **"Adding a language takes effect on the landing after the one that adds it."**

### C. Runner-qualification and the two matches

**[R-26] MUST** — **Identity is the pair `(runner, id)`.** Every rule that used to say *id* says *pair*: uniqueness, the `B` floor, the roll-up, duplicate detection, and every gate that reads them (RF §1, §6.1).

**[R-27] MUST NOT** — Nothing in this file or in any gate that reads it "compares a bare id across runners" (RF §6.1), and `fn` "is never compared across `runner` values" (RF §4.4, §6.5).

**[R-28] MUST** — **The roll-up (frozen ids), within their runner** (RF §6.5, verbatim):
> Let `P(R, F) = { r ∈ result records : r.runner == R and r.fn == F }`, compared by exact string equality on both members. `(R, F)` passes iff `P(R, F)` is non-empty and every member has `out == "passed"`. `P(R, F)` empty means the frozen entry is absent, which is not a pass.

**[R-29] MUST** — **The `B` floor does not roll up** (RF §6.5, verbatim):
> Section 2 holds full ids as collected, each with its runner. A `base` record `b` passes iff some `result` record has `r.runner == b.runner` **and** `r.id == b.id` and `r.out == "passed"`, by exact string equality on both.

**[R-30] MUST** — **Parsing a `Spine-Test` line into `(R, F)`** (RF §6.5, verbatim): "The payload is `<runner>` `U+0020` `<function id>`: split at the **first** `U+0020`; the token before it is `R`, and every byte after it is `F`, spaces included."

**[R-31] REFUSE (G13 finding)** — "A payload with no `U+0020`, or whose first token is outside §4.4's grammar, is a malformed approval line — **G13's finding, not G1's**." (RF §6.5, §13 R29)

**[R-32] MUST** — **`fn` is computed by the collector, not by the trusted stage.** The trusted stage "is deliberately **runner**-unaware: it never parses a runner-native id, and it treats `fn` as an opaque string grouped by equality **within a `runner` value**" (RF §6.5).

**[R-33] MUST** — An AC's `verified_by` edge resolves through **`(runner, fn)`** — a `test` node id is `test:<runner>:<id>` and `Spine-Test`'s ids are *function* ids: "a parametrized function id never appears as a `result` record's `id`, only as its `fn`" (RF §8.5 clause 3, §11 item 7). Matching on `id` would fail every parametrized AC test — **the bug README line 49 records as fixed here**.

### D. What every runner adapter owes (six obligations, RF §6.3)

**[R-34] MUST** — 1. A stable `runner` **token** matching §4.4's grammar. **It is permanent**: `Spine-Test` lines carrying it are sealed into landings forever, "so the token cannot be corrected by a later release without invalidating approvals that name it."

**[R-35] MUST** — 2. A **total, deterministic `id → fn`** function "whose output is a prefix of its input, and which is the identity on an unparametrized id. Totality matters: every id the runner can report must have an `fn`, including ids the adapter's author did not anticipate."

**[R-36] MUST** — 3. A **total `id → path`** producing the repo-relative, `/`-separated path, mapped onto a tree entry and emitted as the tree's bytes; **the empty string where no tree entry matches**.

**[R-37] MUST** — 4. A **total mapping** from the runner's terminal reports onto the **eight** values of §5, "with `unknown` as the defined home for anything unmapped."

**[R-38] MUST** — 5. A **conforming transport** (§6.6).

**[R-39] MUST** — 6. A **`B` outcome per collected id**: for every id the adapter puts in the floor, that id's own outcome on the checkout of `B`, mapped by obligation 4, "or `absent` where the `B` outcome run reported no terminal outcome for it." One consumer only — §8.5 clause 2's carve-out. Cost: for two of the four v1 runners it is a **second invocation of the runner against `B`** (`import-resolver.md` §11.1).

**[R-40] MUST** — **The transport preserves, per item, four signals** (RF §6.6): "the runner-native id, the per-phase outcome, the expected-failure polarity, **and deselection**." "A transport that loses any of the four is not conforming, for any runner." Deselection is mandatory because "runners commonly report it outside the per-item report (pytest through `pytest_deselected`), so a transport carrying only the first three cannot distinguish a `deselected` id from an absent one."

**[R-41] MUST** — The transport "is read over a pipe the collector holds, it is not supplied by the candidate's environment" (RF §6.6).

### E. The pass rule and the `base.out` consumer

**[R-42] MUST** — **The mapping, stated once** (RF §5, verbatim):
> G1 counts a pair `(R, i)` as passed **iff** the body contains exactly one `result` record with `runner == R` and `id == i` whose `out` is `passed`, **and** the `end` record's `status` is `complete`. Every other value, and absence, is not a pass. `passed` is the only value that is ever a pass, in any lane, in any mode, for any gate, under any runner.

**[R-43] MUST** — **`xpass` is not a pass** (RF §5, §13 R4).

**[R-44] MUST** — **Absence is not a value — in the `result` section.** "A pair with no `result` record is *absent*, and absence is not a pass." On a `base` record absence **is** written, as `absent` (RF §5, §4.4).

**[R-45] MUST** — `deselected` and absent are distinguished on purpose: "neither is a pass, but a `deselected` record is a *collected* id and satisfies the AC-coverage clause of §8.5 where an absent one does not" (RF §5).

**[R-46] MUST** — **`status ≠ complete` ⇒ no pair counts as passed**, whatever any `result` record says and whichever runner produced it; and no pair counts as *collected* either, so no clause below clause 0 can be satisfied. "Records are still written because they are evidence for the human who will read the wire; **they are never credit**." (RF §7.3, §8.5 clause 0, §13 R12)

**[R-47] MUST NOT** — **No partial crediting.** "one hung runner fails the landing, including another runner's genuinely green suite. That is deliberate." (RF §7.3, §13 R28)

**[R-48] MUST** — **The one and only consumer of `base.out`** is §8.5 clause 2's carve-out (RF §4.4, §5, §8.5, PB §11): where `b.out` is `"xfail"` **or** `"skipped"` **and** the shape is *did not pass* (some `result` record exists for the pair with `out ≠ "passed"`), `b` yields **no finding at all — not G1's, not G8's**.

**[R-49] MUST NOT** — **`out` on a `base` record is never a pass and never evidence.** "No clause of §8.5 reads it as evidence that anything passed… The single question it answers is *was this id already `xfail` or `skipped` on trunk*, and every other value — `absent` included — answers it identically." (RF §4.4)

**[R-50] MUST** — The carve-out is decided **on `b.out` alone and never on the `T` outcome** (RF §8.5). It **does not reach the *went away* shape** and **does not reach clause 1** (frozen `Spine-Test` entries) (RF §8.5, §11 item 8a).

**[R-51] MUST** — `absent` **MUST NOT** be treated as exempting: only the two literals `xfail` and `skipped` exempt (RF §8.5, §13 R35).

---

## Byte-level fixities

All verbatim.

### F1. Encoding and framing (RF §4.1)

- "UTF-8, no BOM."
- "Lines are terminated by a single LF (`U+000A`). Every line, including the last, is terminated. A CR (`U+000D`) anywhere outside a JSON string escape makes the file malformed — the same rule that keeps envelopes hashing (§5.5)."
- "No blank lines, no comment lines, no leading or trailing whitespace on any line, no bytes after the final LF."
- "Line 1 is the **header line** (§4.2). It is not JSON."
- "Every line after line 1 is one JSON text in the canonical form of §4.3, in the order fixed by §4.5. The extension is `.jsonl` and the body is JSONL: exactly one JSON value per line, no framing other than LF."
- "There is no size cap and no line-length cap. Readers stream."

### F2. Header separator and order (RF §4.2)

- "Six fields, `key=value`, separated by exactly one `U+0020`, in this order".
- "The field order is fixed. A header whose keys appear in any other order is malformed."
- "**A repeated key rejects the file** (§11). So does a missing key, an unknown key, an empty value, a value containing `U+0020`, and any value outside its grammar."
- Why the header cannot grow for multi-runner: "one header line cannot name several runners without a repeated key, and *a repeated key rejects the file*."
- `tool=` parse, where one is wanted: "the token splits at its **last** occurrence of the literal `+sha256:`, which is unambiguous because `<dist_hash>` is exactly 64 lowercase hex." No parse is required for the check itself (RF §13 R14, §8.3 step 2).

### F3. Canonical JSON — the restricted profile (RF §4.3, verbatim, all five rules)

> Every body line is a JSON **object** serialized in this restricted canonical form, which is RFC 8785-compatible over the value space this file uses:
>
> 1. No whitespace anywhere outside string literals.
> 2. Members ordered by key, ascending, over the key's UTF-16 code units. Every key defined here is ASCII, so this is byte order.
> 3. Strings: `"` → `\"`, `\` → `\\`, `U+0008` → `\b`, `U+0009` → `\t`, `U+000A` → `\n`, `U+000C` → `\f`, `U+000D` → `\r`; every other code point below `U+0020` → `\u00xx` with **lowercase** hex; every other code point emitted literally as UTF-8. No other escape is produced and none is accepted.
> 4. No numbers, no `true`/`false`/`null`, no nested objects, no arrays. v1 record kinds contain string values only. (A future kind needing a number uses a non-negative integer with no sign, no leading zero, no fraction and no exponent.)
> 5. Duplicate keys within an object are malformed.

> **Canonical form is required on read, not only on write.** A body line that parses as JSON but is not in canonical form is malformed.

Resulting fixed key orders: `base` → `id, out, path, runner, t`. `result` → `fn, id, out, path, runner, t`. `end` → `status, t`.

### F4. Ordering (RF §4.5, verbatim)

> 1. Header line.
> 2. Every `base` record, sorted ascending by the **bytes** of `runner`, then by the **bytes** of `id`.
> 3. Every `result` record, sorted ascending by the bytes of `runner`, then by the bytes of `id`.
> 4. The `end` record.
>
> A `base` record after a `result` record, a record after `end`, a missing `end`, a second `end`, or a section out of sort order: malformed.

Rationale, normative in effect: "Byte-order sorting removes the runner's report order — which is not deterministic and would otherwise be the file's only clock — and sorting on `runner` first removes the *invocation* order of the runners themselves… **The file therefore does not record, and cannot be made to record, which runner ran first.**"

### F5. Determinism claim (RF §4.5, verbatim)

> For a fixed `(B, T)`, a fixed collector build, a fixed invocation set, and runners that behave identically, a file whose `end.status` is `complete` is fully determined byte for byte.

> **The claim is conditioned on `complete`**: a run the deadline kills is determined only up to where the kill fell, because how many records a hung runner emitted before it was killed is a fact about wall time.

Since v0.19 "*runners that behave identically* now carries the `B` outcome run too". "Two conforming implementations produce identical files."

### F6. Uniqueness and duplicate rules (RF §4.4)

- `base` section: "**The pair `(runner, id)` is unique across the section.** A repeated pair is malformed. The same `id` under two different `runner` values is **not** a duplicate and is not malformed — that is the case the pair exists for."
- `result` section: same — "one `id` under two runners is two records and no duplicate."

### F7. Forward compatibility — none (RF §4.4)

> **Unknown `t` values, unknown keys, missing keys and unknown `out`/`status`/`runner` values are all malformed** — there is no forward-compatibility relaxation, because §7.4 rule 3 already refuses a header whose `tool=` is not the base's pin, so writer and reader are the same build by construction.

`out` is a member of **both** `base` and `result`, "and its admissible set differs by kind: `absent` is a legal `out` on a `base` record and an unknown value — hence malformed — on a `result` one."

### F8. Path bytes (RF §4.4)

> `path` — the repo-relative, `/`-separated path of the file the id was collected from, **byte for byte as git stores it in `B`'s tree**. The collector maps the runner's reported path onto a tree entry and emits the tree's bytes, never the filesystem's: a macOS runner reports NFD where git stores NFC, and the `G8:<path>` a finding cites has to be the tree's spelling or it names nothing. No tree entry matches: the empty string.

### F9. Wire-token spellings that derive from this file (RF §8.5, §11 item 8b)

- Per-id G1 finding: **`G1:` + `tok(path)`**, `tok` being `gate-report.md` §6.2's token encoding — "`esc` with `,` (`0x2C`), `U+0020` and `"` (`0x22`) moved into the `\xHH` row, in one pass over the bytes".
- Path source, by record presence: "the pair's `result` record `path` where the file carries one, its `base` record `path` where it does not."
- **Bare `G1`** — closed list of five: `result-missing`; `result-malformed`; clause 0's own finding (`end.status ≠ complete`); clause 3's uncovered AC; clause 1 where `P(R, F)` is empty. Plus any per-id finding whose `path` is the empty string.
- "`G1:` with nothing after the colon is never written."
- One entry per path, never per pair — `gate-report.md` §6.1 keys `wires` on `(gate, path)`.

---

## Error cases

### E1. Grammar / framing errors — all yield `result-malformed`

Ingestion label (RF §8.2): "Absent, unreadable, or violating §4 in any particular: the finding is `result-missing` or `result-malformed`. **These are G1 findings, not states**… **There is no partial ingestion** — a malformed file yields no outcomes at all, never 'read what parsed'."

| Condition | Behaviour | Label / token | Cite |
|---|---|---|---|
| BOM present, or non-UTF-8 bytes | malformed | `result-malformed`, bare `G1` | RF §4.1, §8.2 |
| A CR (`U+000D`) outside a JSON string escape | malformed | `result-malformed` | RF §4.1 |
| Missing final LF, bytes after final LF, blank line, comment line, leading/trailing whitespace on a line | malformed | `result-malformed` | RF §4.1 |
| Header key repeated | **rejects the file** | `result-malformed` | RF §4.2, PB §11 |
| Header key missing / unknown / out of order | malformed | `result-malformed` | RF §4.2 |
| Header value empty, or containing `U+0020`, or outside its grammar | malformed | `result-malformed` | RF §4.2 |
| `profile=n/a` in a **header** | malformed | `result-malformed` | RF §4.2, §13 R15 |
| Header `keys_visible` key **missing** | malformed (≠ a precondition-2 failure) | `result-malformed` | RF §13 R10 |
| `ids=` disagrees with the number of `base` records | malformed | `result-malformed` | RF §4.2 |
| `ids=` with a sign, or a leading zero (other than `0` itself) | malformed | `result-malformed` | RF §4.2 |
| Filename stem ≠ header `tree=` | malformed | `result-malformed` | RF §3 |
| Body line parses as JSON but is not canonical (§4.3) | malformed | `result-malformed` | RF §4.3 |
| Duplicate keys within a body object | malformed | `result-malformed` | RF §4.3 rule 5 |
| Unknown `t`, unknown key, missing key | malformed | `result-malformed` | RF §4.4 |
| Unknown `out` value; `absent` on a `result` record | malformed | `result-malformed` | RF §4.4 |
| Unknown `status` value | malformed | `result-malformed` | RF §4.4, §7.3 |
| `runner` outside `[a-z][a-z0-9_-]{0,31}` | malformed | `result-malformed` | RF §4.4 |
| `result.fn` not a prefix of `result.id` | malformed | `result-malformed` | RF §4.4 |
| Repeated `(runner, id)` pair within a section | malformed | `result-malformed` | RF §4.4 |
| Same `id` under two different `runner` values | **NOT malformed** — legal, two records | — | RF §4.4, §7.2, §10 |
| `base` record after a `result` record | malformed | `result-malformed` | RF §4.5 |
| Any record after `end`; missing `end`; a second `end` | malformed | `result-malformed` | RF §4.5 |
| Either section out of sort order | malformed | `result-malformed` | RF §4.5 |
| A `runner` value the repository does not declare (`params.langs`) | **`result-malformed`**, checked at §8.3 **step 3**, after labels and `tool=` | `result-malformed` | RF §8.3, §13 R30 |
| A declared runner that contributed **no** records | **not checked** — absence is deliberately not checked | — | RF §8.3, §13 R30, §14 OPEN-6 |
| No file at `.spine/cache/results/<T>.jsonl` | G1 finding | `result-missing`, bare `G1` | RF §8.2 |
| Artifact holds ≠ exactly one regular file at exactly that path; extra entries, directories, symlinks, absolute paths, `..` components | not-a-result | `result-missing` | RF §8.1 |
| File present, well-formed, **origin unestablished** | **ingested**, evaluated in full | never `result-missing`/`result-malformed`; **auto-merge precondition 2 unmet** only | RF §8.1, §8.4, §11 item 15, §13 R31 |

### E2. Label / policy errors during ingestion (order is normative, stop at first failure)

| Step | Condition | Behaviour | Token | Cite |
|---|---|---|---|---|
| 8.3.1 | `tree=` **or** `base=` ≠ the `(T, B)` this run fixed | run ends and re-queues on the new tip; `C-M3` bounds re-verifications within the run | **`base-moved`** | RF §8.3, §13 R8 |
| 8.3.2 | `tool=` ≠ trunk's `cli.version` + `cli.dist_hash`, compared **as bytes over the whole token** | **G15 failure, never a retry** | `G15` | RF §8.3, §13 R7/R14 |
| 8.3.3 | undeclared `runner` token | malformed | `result-malformed` | RF §8.3, §13 R30 |
| 8.4 | `params.isolation ≠ container`, or header `profile=` ≠ it | **precondition 1 unmet**; `C-M4` evaluates `off`; one `class=tripwire` `G11` wire, `reason=` mandatory | `G11` (pathless) | RF §8.4, §13 R9 |
| 8.4 | `keys_visible=true`, **or** step-2 failed, **or** no trunk-defined origin evidence | **precondition 2 unmet** (three conjuncts) — **one wire however many conjuncts failed** | `G11` (pathless) | RF §8.4, §13 R10/R31 |

### E3. Runner-fate errors — the `end.status` domain

Each row of RF §7.3 (reproduced in *Data model §4*) is an error case with an exact status token. Governing rules:

| Condition | Behaviour | Status token | Cite |
|---|---|---|---|
| Every runner completed | outcomes count | `complete` | RF §7.3 |
| Any runner's **`B` enumeration** failed or timed out | **whole body suppressed**, `ids=0`, no `base` and no `result` from any runner | `base-collect-failed` | RF §7.3, §13 R13 |
| A runner could not start at all | its `base` records only | `spawn-failed` | RF §7.3 |
| Runner started, terminated, emitted no parsable stream event | its `base` records only | `no-output` | RF §7.3 |
| Unparsable stream event, or a non-UTF-8 id | its `base` records only | `stream-invalid` | RF §7.2, §7.3 |
| Abnormal termination, or stream ended mid-record | its `base` records + every `result` parsed before the break | `runner-failed` | RF §7.3 |
| Deadline expired on the `T` run; process group killed | its `base` records + every `result` parsed before the kill | `runner-timeout` | RF §7.3, §13 R24 |
| A runner emitted a terminal event **and then** a process-group member died by signal | `complete` does **not** apply | `runner-failed` | RF §7.3, §13 R23 |
| A separate **`B` outcome** run failed/was killed/was unparsable | no status contribution; `end.status` does not move; unreported ids get `out: "absent"` | *(none)* | RF §7.3, §4.4 |
| `params.timeout` present but non-positive or non-integer | **REFUSE — fails the job before anything is written** | *(no file)* | RF §13 R24 |
| `params.isolation: uid` requested under `--ci` | **REFUSE at §7.1 step 1, no file written** | *(no file)* | RF §7.1, §11 item 16 |
| Any `status` other than `complete`, or `keys_visible=true` | collector **exit status non-zero**; file still written | — | RF §7.3 |

### E4. Evaluation-time findings that read this file

| Condition | Behaviour | Token | Cite |
|---|---|---|---|
| `end.status ≠ complete` | **G1 fails**; clauses 1–3 still evaluated and reported; **the G8 allocation of clause 2 is not made** | bare `G1` (+ per-path clause entries) | RF §8.5 clause 0 |
| Frozen `(R, F)` with `P(R, F) = ∅` | G1 finding | **bare `G1`** | RF §8.5 clause 1 |
| Member of `P(R, F)` with `out ≠ passed` | one entry per member's `result` record `path` | `G1:` + `tok(path)` | RF §8.5 |
| `base` pair *did not pass*, `b.out ∉ {xfail, skipped}` | G8 **and** G1 | `G8:<b.path>` + `G1:tok(result.path)` | RF §8.5 clause 2 |
| `base` pair *did not pass*, `b.out ∈ {xfail, skipped}` | **no finding in either gate**, no wire, `G1=pass` and `G8=pass` on `b`'s account | — | RF §8.5 clause 2, §13 R16/R35 |
| `base` pair *went away* (whatever `b.out`) | G8 `G8:<b.path>`; G1 too **unless** a `class=protected` review's `wires=` names `G8:<b.path>` | `G8:<b.path>`, `G1:tok(b.path)` | RF §8.5, §13 R19 |
| `b.path` is the empty string | "names no path, can satisfy no exemption, and is a G1 finding" | **bare `G1`** | RF §4.4, §8.5, §13 R19 |
| An AC with no collected `verified_by` edge | G1 finding | **bare `G1`** | RF §8.5 clause 3 |
| Two runners collecting from one file | one `class=protected` `G8:<path>` review discharges **both** runners' ids at that path | `G8:<path>` (no runner qualifier) | RF §8.5, §10 |

---

## Worked examples / test vectors

### V1. The approval's two frozen lines (RF §10, verbatim)

```
Spine-Test: pytest tests/billing/test_invoice.py::test_AC1_totals_include_tax
Spine-Test: vitest tests/billing/invoice.test.ts > invoice totals > AC2 zero-rated lines
```

Parsed at the **first** space to `("pytest", "tests/billing/test_invoice.py::test_AC1_totals_include_tax")` and `("vitest", "tests/billing/invoice.test.ts > invoice totals > AC2 zero-rated lines")` — "the second function id contains spaces of its own, which is why the split is at the first one and not the last (§6.5)."

### V2. The 20-line `complete` file (RF §10, verbatim)

Setting: repo `billingsvc`, `params.ci: github`, `params.langs: ["python", "ts"]`, `params.isolation: container`, `params.timeout: 1800`, `object_format: sha1`, pinned `cli.version 1.4.0`. Invocation set = `pytest` + `vitest`. Trunk collected seven pairs (four pytest, three vitest).

Path: `.spine/cache/results/3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28.jsonl`

```
tree=3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28 base=7b0d4a1f2c3e5d6a8b9c0d1e2f3a4b5c6d7e8f90 tool=1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db keys_visible=false profile=container ids=7
{"id":"tests/billing/test_discounts.py::test_percentage_discount","out":"passed","path":"tests/billing/test_discounts.py","runner":"pytest","t":"base"}
{"id":"tests/billing/test_invoice.py::test_invoice_renders","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"base"}
{"id":"tests/billing/test_invoice.py::test_line_items_sum[one]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"base"}
{"id":"tests/billing/test_invoice.py::test_line_items_sum[two]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"base"}
{"id":"tests/billing/invoice.test.ts > invoice totals > renders","out":"passed","path":"tests/billing/invoice.test.ts","runner":"vitest","t":"base"}
{"id":"tests/core/util.test.ts > rounding > half-even","out":"passed","path":"tests/core/util.test.ts","runner":"vitest","t":"base"}
{"id":"tests/core/util.test.ts > rounding > half-up","out":"passed","path":"tests/core/util.test.ts","runner":"vitest","t":"base"}
{"fn":"tests/billing/test_discounts.py::test_percentage_discount","id":"tests/billing/test_discounts.py::test_percentage_discount","out":"passed","path":"tests/billing/test_discounts.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/test_invoice.py::test_AC1_totals_include_tax","id":"tests/billing/test_invoice.py::test_AC1_totals_include_tax[reduced-rate]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/test_invoice.py::test_AC1_totals_include_tax","id":"tests/billing/test_invoice.py::test_AC1_totals_include_tax[standard-rate]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/test_invoice.py::test_AC1_totals_include_tax","id":"tests/billing/test_invoice.py::test_AC1_totals_include_tax[zero-rate]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/test_invoice.py::test_invoice_renders","id":"tests/billing/test_invoice.py::test_invoice_renders","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/test_invoice.py::test_line_items_sum","id":"tests/billing/test_invoice.py::test_line_items_sum[one]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/test_invoice.py::test_line_items_sum","id":"tests/billing/test_invoice.py::test_line_items_sum[two]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/invoice.test.ts > invoice totals > AC2 zero-rated lines","id":"tests/billing/invoice.test.ts > invoice totals > AC2 zero-rated lines","out":"passed","path":"tests/billing/invoice.test.ts","runner":"vitest","t":"result"}
{"fn":"tests/billing/invoice.test.ts > invoice totals > renders","id":"tests/billing/invoice.test.ts > invoice totals > renders","out":"passed","path":"tests/billing/invoice.test.ts","runner":"vitest","t":"result"}
{"fn":"tests/core/util.test.ts > rounding > half-even","id":"tests/core/util.test.ts > rounding > half-even","out":"passed","path":"tests/core/util.test.ts","runner":"vitest","t":"result"}
{"fn":"tests/core/util.test.ts > rounding > half-up","id":"tests/core/util.test.ts > rounding > half-up","out":"passed","path":"tests/core/util.test.ts","runner":"vitest","t":"result"}
{"status":"complete","t":"end"}
```

Checks this vector exercises (RF §10):
- Sort: "every `pytest` record precedes every `vitest` one, because `p` < `v`. Within each runner the sort is on `id` bytes, which puts `…::test_AC1_…[reduced-rate]` before `[standard-rate]` before `[zero-rate]`, and `…half-even` before `…half-up`."
- `ids=7` equals the seven `base` records, four pytest and three vitest.
- Frozen `("pytest", …test_AC1_totals_include_tax)` → `P(R, F)` = the three parametrized records, all `passed` → pass.
- Frozen `("vitest", …> AC2 zero-rated lines)` → one record, `passed` → pass.
- The branch's own two tests are **absent from section 2** — "they are not trunk's, and they enter the floor only when this intent lands."
- `verified_by` targets are `test:pytest:tests/billing/test_invoice.py::test_AC1_totals_include_tax` and `test:vitest:tests/billing/invoice.test.ts > invoice totals > AC2 zero-rated lines`; the first "appears nowhere as a record's `id`; it resolves through `(runner, fn)`."
- `profile=container` = `params.isolation` → precondition 1 holds; `keys_visible=false` + matching `tool=` + origin evidence → precondition 2 holds; **precondition 0 fails anyway** (`C-A3: hostile`), so `C-M4` evaluates `off`, one `class=tripwire` `G11` wire, and **`Spine-Gates` records `G11=pass`**.
- Seal: `profile=container threat=hostile`.

**Published-digest status of this vector.** RF §10 publishes **no digest over these bytes** — the front-matter states "no vector in §10 publishes a digest", and `gate-report.md` §8's `evidence.result_sha256` (`sha256:0b93f4ac5182d67e0a4c31fb9d20e857643ca0b1f9e78d5236ca04b81e7d3f96`) is enumerated as **fabricated but well-formed**, not computed over this file (`gate-report.md` line 741). **Do not treat that value as a digest of V2.** The one *computed* value inside V2's header is the `tool=` `dist_hash` `sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db`, adopted from `manifest.md` §8.2 (`shasum -a 256` over that document's 529-byte artifact list).

### V3. The `xfail`-on-trunk case, and the four edges (RF §10, verbatim table)

If trunk's `tests/billing/test_discounts.py::test_percentage_discount` is `@pytest.mark.xfail`: its `base` record reads `out: "xfail"` and its `result` record reads `out: "xfail"`. Under §8.5 clause 2 → "**no finding in either gate**: `Spine-Gates` still records `G1=pass` and `G8=pass`, `wires` is unchanged". `ids=7` unchanged; "the two `out` members that moved are the only difference."

| What trunk reported on `B` | What `T` reported | Finding |
|---|---|---|
| `xfail` | `xfail`, `failed`, `error`, `skipped`, `xpass` or `unknown` | **none**, in either gate — the carve-out, decided on `b.out` alone |
| `xfail` | nothing at all (the id went away) | G8 `G8:tests/billing/test_discounts.py`, and G1 unless a `class=protected` review names that path — the carve-out does not reach this shape |
| `skipped` | `skipped`, `failed`, `error`, `xfail`, `xpass` or `unknown` | **none**, in either gate — the same carve-out, over its second value, decided on `b.out` alone |
| `passed` | `xfail` | G8 and G1 both, as before: an id that *was* passing on trunk and is now expected-to-fail is a regression this landing introduced, which is the whole distinction |

### V4. The two-runner id collision (RF §10)

"Suppose the repository were migrating from jest to vitest and both collected `tests/core/util.test.ts > rounding > half-even`. That is two `base` records and two `result` records, `ids=` counts two, and neither section is malformed — the pair is the identity, and a duplicate is a repeated *pair*."

### V5. The timed-out file — 18 lines, `status=runner-timeout` (RF §10, verbatim)

pytest completed; vitest hung after its first test and was killed at `params.timeout`. Both `B` collections had already succeeded, so the `base` section is whole.

```
tree=3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28 base=7b0d4a1f2c3e5d6a8b9c0d1e2f3a4b5c6d7e8f90 tool=1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db keys_visible=false profile=container ids=7
{"id":"tests/billing/test_discounts.py::test_percentage_discount","out":"passed","path":"tests/billing/test_discounts.py","runner":"pytest","t":"base"}
{"id":"tests/billing/test_invoice.py::test_invoice_renders","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"base"}
{"id":"tests/billing/test_invoice.py::test_line_items_sum[one]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"base"}
{"id":"tests/billing/test_invoice.py::test_line_items_sum[two]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"base"}
{"id":"tests/billing/invoice.test.ts > invoice totals > renders","out":"passed","path":"tests/billing/invoice.test.ts","runner":"vitest","t":"base"}
{"id":"tests/core/util.test.ts > rounding > half-even","out":"passed","path":"tests/core/util.test.ts","runner":"vitest","t":"base"}
{"id":"tests/core/util.test.ts > rounding > half-up","out":"passed","path":"tests/core/util.test.ts","runner":"vitest","t":"base"}
{"fn":"tests/billing/test_discounts.py::test_percentage_discount","id":"tests/billing/test_discounts.py::test_percentage_discount","out":"passed","path":"tests/billing/test_discounts.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/test_invoice.py::test_AC1_totals_include_tax","id":"tests/billing/test_invoice.py::test_AC1_totals_include_tax[reduced-rate]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/test_invoice.py::test_AC1_totals_include_tax","id":"tests/billing/test_invoice.py::test_AC1_totals_include_tax[standard-rate]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/test_invoice.py::test_AC1_totals_include_tax","id":"tests/billing/test_invoice.py::test_AC1_totals_include_tax[zero-rate]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/test_invoice.py::test_invoice_renders","id":"tests/billing/test_invoice.py::test_invoice_renders","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/test_invoice.py::test_line_items_sum","id":"tests/billing/test_invoice.py::test_line_items_sum[one]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/test_invoice.py::test_line_items_sum","id":"tests/billing/test_invoice.py::test_line_items_sum[two]","out":"passed","path":"tests/billing/test_invoice.py","runner":"pytest","t":"result"}
{"fn":"tests/billing/invoice.test.ts > invoice totals > AC2 zero-rated lines","id":"tests/billing/invoice.test.ts > invoice totals > AC2 zero-rated lines","out":"passed","path":"tests/billing/invoice.test.ts","runner":"vitest","t":"result"}
{"status":"runner-timeout","t":"end"}
```

"pytest contributed `complete`; vitest contributed `runner-timeout`; the fold is the first non-`complete` row any runner contributed, so `end.status` is `runner-timeout`. **`status ≠ complete`, so clause 0 fails and pytest's seven `passed` records credit nothing**… G1 fails and names every frozen entry and every `base` pair, in both runners. **No G8 allocation is made** — a killed run says nothing about whether those ids still collect."

### V6. The `--ci generic` variant — same bytes, one bit of evaluation moves (RF §10)

"Take the twenty-line `complete` file above and change one manifest field: `params.ci: generic`. **Nothing about the file moves — same bytes, same digest, same stem** — because the collector's output is a function of `(B, T)`, its build and its invocation set, and none of those is the provider."

| | `--ci github` | `--ci generic` |
|---|---|---|
| Ingested? | yes | **yes** |
| `result-missing` / `result-malformed`? | no | **no** |
| §8.3 steps 1–3 | pass | pass, on the same bytes |
| G1 clauses 0–3 | all satisfied | all satisfied, identically |
| `Spine-Gates` G1 | `pass` | `pass` |
| Precondition 1 | met | met |
| Precondition 2 | **met** | **unmet** — third conjunct |
| `G11` wire | one, `class=tripwire` | one, `class=tripwire`, `reason=` naming precondition 2 as well |
| Seal | `profile=container threat=hostile` | identical |
| Auto-merge | unavailable (precondition 0) | unavailable, and unavailable on every future run in this repository |

### V7. pytest id → fn → path (RF §6.7, **illustrative only**; normative in `import-resolver.md`)

```
runner = pytest
id     = tests/billing/test_invoice.py::test_AC1_totals_include_tax[zero-rate]
  fn   = tests/billing/test_invoice.py::test_AC1_totals_include_tax
  path = tests/billing/test_invoice.py

runner = pytest
id     = tests/core/test_util.py::TestRounding::test_half_even
  fn   = tests/core/test_util.py::TestRounding::test_half_even     (unparametrized: fn == id)
  path = tests/core/test_util.py
```

Rule: "For a nodeid, split on `::`. In the final component, the parametrization suffix begins at the **first** `[` and runs to the end, and exists only if the component's last character is `]`."

Illustrative outcome mapping (RF §6.7):

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

**[R-52] MUST NOT** — "This document fixes no per-language id grammar and no `runner` token, and an implementer **must not** infer either from the examples here." Where §6.7 disagrees with `import-resolver.md`, **that document wins** (RF §6.4).

---

## Cross-references it depends on

| What | Owner | Why this sheet needs it |
|---|---|---|
| The **set** of `runner` tokens; `id → fn`; `id → path`; per-runner outcome mappings; the `B`-outcome source and its cost per adapter (§11.1–§11.6) | `import-resolver.md` | RF §4.4, §6.3, §6.4, §12 delegate all four to it in one place "so that two implementations cannot disagree on an edge case and reject each other's approvals". The `base.out` values for all four adapters come from `import-resolver.md` §11.1. |
| The source-symbol → runner-id join (`@verifies` pragma grammar, the file-granular join over `path`, `test_AC<n>` sugar) | `import-resolver.md` §12.1–§12.3 | RF §12 explicitly hands it over; G1's coverage clause assumes it. |
| `tok()` token encoding, `wires` keying on `(gate, path)`, `evidence.result_sha256`, `automerge.preconditions[2].status` domain (`met`/`unmet`/`exempt`) | `gate-report.md` §5.8, §5.9, §6.1, §6.2, §6.3 | RF §8.5's wire tokens and §8.4's precondition record are written into that document's objects. |
| `cli.version` + `cli.dist_hash` (the expected `tool=` token), `params.langs` monotonicity (check 12, `langs-shrank`), check 12b keeping `isolation: uid` out of trunk | `manifest.md` §3.3, §6.2, §8.2 | RF §8.3 step 2 constructs the expected `tool=` from it; RF §14 OPEN-6 defers to check 12. |
| Trunk-defined origin evidence, per provider (GitHub's three-clause `workflow_run` test; the scoring of the shipped arrangements) | `ci.md` §10.2, §10.3, §14 R11 | RF §8.1 fixes only *what the evidence is*; the provider test is deliberately not here (RF §12). |
| The sort `freeze=` is taken over, now ordering on the runner token first (because it orders on the line's bytes) | `envelope-vectors.md` | RF §12 hands it that vector. |
| `Spine-Test` payload row, `Spine-Seal` `profile=` domain, `Spine-Gates` semantics, G1/G8 rows | `PLAYBOOK.md` §11, §6.3 | Rule 1: PB §11 wins on vocabulary. |
| The collector's step order, the isolation mechanism M1, P1–P4, the restore phase | RF §7.1 (a *different* sheet's concern) | This sheet only records the `profile=` domain the mechanism licenses. |
| Ingestion order, preconditions, overridability (§8.7) | RF §8 (a *different* sheet's concern) | Recorded here only as the destination of grammar errors. |

---

## OPEN items

Undecided owner questions. **Do not invent values.**

- **OPEN-5 — G6's reporting channel.** "§6.3 says mutation results come 'through the same collector as every other test'. Mutation results are per-mutant, not per-id, and this spec **deliberately does not overload `result` records** to carry them. Four languages make the question larger rather than smaller: a mutation channel would owe a per-language mutant identity as well as a per-language test identity." (RF §14 OPEN-5, §12) — **Consequence for this sheet: v1 record kinds stay at exactly three.**
- **OPEN-7 — is `params.ci` monotone in the guarantee it names?** Precondition 2's third conjunct is a function of the provider arrangement, so `params.ci: github → gitlab` moves precondition 2 "from reachable to permanently unmet for that repository". Candidate answers: leave it to the protected review, or make G16 fail such a landing unless it also writes `C-M4: off`. Filed three times — `ci.md` OPEN-3, RF OPEN-7, `manifest.md` OPEN-1 — and `manifest.md` owns the fix. **Owner's, and a §6.7/§11 change either way.** (RF §14 OPEN-7)
- **Coverage data / a second channel (v1.1).** "`exercises` edges are optional and v1.1… **There is no second channel in v1**: §11 makes this file the untrusted job's only artifact… v1.1 must either add a record kind here or raise §11's only-artifact rule — **the owner's decision, not this spec's**." (RF §12)
- **Interpolating parametrization** — `test.each` and equivalents "leave no unparametrized base name, so obligation 2's prefix property must be established for it **or the runner declared unsupported**." RF supplies the test (`fn` is a prefix of `id`) and "refuses to guess the grammar that satisfies it"; the resolution is `import-resolver.md`'s (RF §6.4, §14 OPEN-2).

**Closed, recorded so an implementer does not reopen them:** OPEN-1 (deadline: `params.timeout`, seconds, from trunk, absent ⇒ `1800`, per runner invocation, expiry ⇒ `runner-timeout`), OPEN-2 (four v1 languages; Kotlin dropped; tokens delegated), OPEN-3 (multi-runner: pair identity, one file, header unchanged), OPEN-4 (a reseal runs the suite and is exempt from nothing), OPEN-6 (answered by `manifest.md` §6.2 check 12, `langs-shrank`, **coverable** — "a reader who implements from §14 alone must take check 12's Kind column, not this paragraph's recommendation"), OPEN-8 (`skipped` joins `xfail` in the one carve-out), OPEN-9 (M1 network namespace + P4).

---

## Contradictions found

1. **`keys_visible` domain — PB §11 vs RF §4.2.** PB §11 (line 1012) spells the header literally as `keys_visible=false` while spelling `profile=` as an alternation. RF §4.2 gives the field the domain `true | false` and RF §13 R10 concedes this **widens** §11's grammar: "Admitting `true` **widens** §11's grammar rather than resolving an ambiguity in it, and is reported as a §11 defect." *Rule 1 says PB §11 wins on vocabulary — but the corpus itself has already chosen the wider reading, and the wider reading is required for §5.4's solo path to be reachable (`keys_visible=true` there, §7.4, §13 R11).* **Implement the two-value domain; flag it as an unlanded §11 defect.**
2. **Header vs seal `profile=` domain.** PB §11's `Spine-Seal` row admits `container|uid|none|n/a`; PB §11's result-file clause and RF §4.2 admit three. RF §13 R15 states the difference is deliberate: "`n/a` is a seal value only; a header carrying it is malformed." **Not a defect — but a reader of one row alone will get it wrong.**
3. **`Spine-Test` payload row — reported and since made.** As filed, PB §11's row read `<runner-native function id>` with no runner, while PB §4.3's own example spelled `Spine-Test: vitest …`; RF §13 R29 said "**§11's row must be amended** … the amendment is required, not optional, and is not made here." **PB v0.19 line 1001 now reads** `<runner> <runner-native function id>`, so the contradiction is closed in this clone. **A pre-v0.19 clone still carries it.**
4. **PB §7.4 rule 0 "never ingestible" vs RF §8.1.** As filed, rule 0 said a file from a job that cannot demonstrate trunk-defined origin "is **never ingestible**", and `ci.md` R14 said the opposite. Settled by the owner 2026-08-26 (RF §13 R31): **ingest, and add a third conjunct to auto-merge precondition 2**. The strict reading "would fail **every** landing on two of the three shipped providers". PB is amended to match. **RF §11 item 15 makes refusing such a file explicit non-conformance.**
5. **PB §7.4 rule 5's precondition 2 — two conjuncts vs three.** As filed, rule 5's own text still read two conjuncts one paragraph below rule 0's amended text, so "an implementer reading rule 5 in isolation wrote a two-conjunct test, and every GitLab-in-repository and `--ci generic` repository then reached `automerge.effective: true`". **PB v0.19 has taken the transcription: three conjuncts, not two** (RF §14, second amendment note). Implement three.
6. **PB §11 CLI nesting vs §5.4 solo path.** §11's CLI line spells `spine check [--ci [--collect]]` while §5.4 and §12 require `--collect` outside `--ci`. RF §13 R21 reads it as `spine check [--ci] [--collect]` and reports it as a **§11 defect**: "as written the solo path has no legal invocation and §5.4's whole *Solo developers* paragraph is unreachable."
7. **RF §14 OPEN-6 recommendation vs `manifest.md` §6.2 check 12.** RF "recommended an **outright** failure, with removal available only through `--uninstall` and re-init"; `manifest.md` check 12 makes `langs-shrank` **coverable** by a protected review. **The weaker answer is the live one** — spec-vs-spec, and RF concedes the point in terms.
8. **`tool=` token divergence across published vectors.** RF §4.2 requires the header's `tool=` and the seal's `tool=` to "compare by byte equality". RF §10's header and `manifest.md` §8.2 both carry `1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db`; `envelope-vectors.md`'s five seals in §8–§11 carry `1.4.0+sha256:41d0e9b7c2a6538f10bd47e29c05a3f6b81d24e70c9a5b3f68d1027ae4c95b3d`. EV discloses this as one of exactly three known divergences pending a keyring regeneration and warns: "**Do not 'fix' this by editing the three fields in place.**" **A conformance test comparing RF §10's header against an EV seal will fail on this value and only this value.**
9. **RF §6.7 vs `import-resolver.md`.** RF §6.7's pytest walkthrough is "**illustrative only**… where it disagrees with `import-resolver.md`, that document wins" (RF §6.4). Likewise the tokens `pytest` and `vitest` used throughout RF's examples: "**Fixing a permanent identifier by example is not good enough**, and `import-resolver.md` must ratify both tokens explicitly rather than inherit them from a worked example."
10. **RF §13 closing note vs the front-matter's spec version.** The §13 closing paragraph says "This document moves from spec version 1 to 2 because the record shape changed: a `runner` key is now mandatory"; the front-matter and README both say **spec version 3** (the third grammar change being `base.out`). The closing note was not re-worded when v3 landed. Harmless — the version "is not written into the file, not compared by any gate" — but a reader diffing the two will notice.
11. **Grammar-stability claim vs the `base.out` amendment.** The front-matter says of the `out` amendment: "**This is the one amendment that changes the file's grammar**: a `base` record gains a required member, so a file written before it is not a conforming file after it" — while the same paragraph asserts "no vector in §10 publishes a digest — the `base` lines there gain `"out":"passed"` and the file is still twenty lines." Both are true; the note exists because two other amendment paragraphs each claim "no byte of the file's grammar moves". **Only the `out` amendment breaks backward compatibility.**
