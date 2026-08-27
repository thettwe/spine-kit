# The collector: order of operations, reduction, crash/timeout handling, and the solo path

Requirement sheet for a Rust implementation of `spine check --ci --collect` (and `spine check --collect`),
covering **only** the collector's own control flow. The isolation boundary's internals (M1, its five host
prerequisites, the probe and P1–P4, the two network dispositions) and the restore phase's internals are
**owned by another sheet**; this sheet records only where they sit in the order of operations, what they
hand back, and what the deadline does to them.

Citation convention: `RF` = `docs/spec/result-file.md`, `PB` = `PLAYBOOK.md`, `CI` = `docs/spec/ci.md`,
`IR` = `docs/spec/import-resolver.md`, `MF` = `docs/spec/manifest.md`, `GR` = `docs/spec/gate-report.md`.

---

## Sources read

| File | Section | Lines |
|---|---|---|
| `docs/spec/result-file.md` | §1 Scope (five governing constraints) | 15–29 |
| `docs/spec/result-file.md` | §2 Position in the pipeline (the whole table) | 31–45 |
| `docs/spec/result-file.md` | §3 Path and naming (temp-and-rename, one-file rule) | 46–61 |
| `docs/spec/result-file.md` | §4.1 Encoding and framing | 64–72 |
| `docs/spec/result-file.md` | §4.2 The header line (incl. key-visibility predicate) | 73–102 |
| `docs/spec/result-file.md` | §4.3 Canonical JSON | 103–114 |
| `docs/spec/result-file.md` | §4.4 Record kinds (`base`, `result`, `end`; `runner` token) | 115–159 |
| `docs/spec/result-file.md` | §4.5 Ordering + determinism claim | 160–170 |
| `docs/spec/result-file.md` | §5 The outcome vocabulary | 171–200 |
| `docs/spec/result-file.md` | §6.1–§6.7 (invocation set, adapter obligations, transport, pytest) | 201–296 |
| `docs/spec/result-file.md` | **§7.1 Order of operations, steps 1–10** | 300–310 |
| `docs/spec/result-file.md` | §7.1 *The deadline* + *The collector passes no selection* | 412–418 |
| `docs/spec/result-file.md` | **§7.2 Reduction** | 420–424 |
| `docs/spec/result-file.md` | **§7.3 When a runner crashes, times out, or produces nothing** | 426–454 |
| `docs/spec/result-file.md` | **§7.4 Outside `--ci`** | 455–460 |
| `docs/spec/result-file.md` | §8.4 Preconditions of §7.4 rule 5 (cross-ref only) | 502–518 |
| `docs/spec/result-file.md` | **§9 What a candidate can and cannot influence** | 580–612 |
| `docs/spec/result-file.md` | §10 Worked example + the timed-out variant + the `generic` table | 614–718 |
| `docs/spec/result-file.md` | §11 Conformance checklist (items 9, 10, 11, 13, 14, 16, 16a) | 719–742 |
| `docs/spec/result-file.md` | §12 Out of scope, §13 R6/R12/R13/R23/R24/R28/R34/R35, §14 OPEN-1…OPEN-9 | 743–833 |
| `PLAYBOOK.md` | §7.1 Least privilege per stage (untrusted-stage row) | 788–804 |
| `PLAYBOOK.md` | **§7.4 rules 0, 1, 2, 3** (and rule 5 read for context) | 848–881 |
| `PLAYBOOK.md` | §6.7 `params.langs` / `params.timeout` / `params.isolation` prose | 735 |
| `PLAYBOOK.md` | §11 Files and refs — the result-file grammar sentence | 1012 |
| `PLAYBOOK.md` | §12 v0.19 change notes (the four owner decisions; the price paragraph) | 1081, 1103, 1109 |
| `docs/spec/ci.md` | §5.1 invocation contract, §5.2 exit codes, §5.4 step 8–9, §5.6 restore | 150–170, 505–515, 558 |
| `docs/spec/README.md` | status row for `result-file.md`, the six settled owner decisions, digest table | whole file (88 lines) |

Sections deliberately **not** transcribed here (another sheet owns them): RF §7.1 *The isolation boundary*,
*M1*, *M1's root*, *M1's identity source*, the host-prerequisite table, *M1's two network dispositions*,
*The restore phase*, *The probe, and the four tests*, *The verdict*, and the two dispositions (RF lines
311–411). RF §8 (ingestion) is another sheet's. RF §8.5 (evaluation/allocation) is another sheet's.

---

## Data model

### Policy inputs — all read from `origin/<trunk>`, none from the checkout

| Field | Source | Type | Domain | Default | Required? |
|---|---|---|---|---|---|
| `cli.version` | `origin/<trunk>:.spine/manifest.json` | string | non-empty; every char in `U+0021`–`U+007E` (RF §4.2) | none | yes |
| `cli.dist_hash` | same | string | `sha256:` + 64 lowercase hex (RF §4.2, MF §8.2) | none | yes |
| `params.isolation` | same | enum | `container` \| `uid` \| `none` | `none` when absent (RF §7.1 step 1, §6.7; PB §6.7) | no |
| `params.langs` | same | array of string | subset of `python`, `ts`, `dart`, `swift` (RF §6.4) | none | yes |
| `params.timeout` | same | integer | **strictly positive** integer seconds (RF §7.1 *The deadline*) | `1800` when absent | no |
| `object_format` | same | enum | `sha1` \| `sha256` — fixes oid hex length 40/64 (RF §3, §4.2) | none | yes |
| `params.trunk` | same | string | the trunk branch name; `origin/<trunk>` is built from it | none | yes |
| `params.ci` | same | enum | `github` \| `gitlab` \| `generic` — **read by the trusted stage, not by the collector**; changes no byte the collector writes (RF §10, *The same run, on `--ci generic`*) | — | n/a to collector |

### Run-local state the collector holds

| Name | Type | Domain | Notes |
|---|---|---|---|
| `H` | git ref | `intent/<ID>` \| `quick/<name>` \| `spine/upgrade-<version>` (PB §7.4 rule 3) | learned as `git symbolic-ref HEAD`; a detached HEAD is a refusal (CI §5.1) |
| `B` | commit oid | `origin/<trunk>` tip at the moment policy was read (RF §4.2 row 2) | for a **reseal**, `B` is the seal's `base=` and every policy read is taken from it (RF §4.2, §13 R22) |
| `T` | tree oid | `git merge-tree --write-tree origin/<trunk> H` (RF §7.1 step 5) | lowercase hex, full length, never abbreviated |
| `keys_visible` | bool | `true` \| `false` | probed at step 4, held (RF §7.1 step 4, §4.2) |
| `profile_achieved` | enum | `container` \| `uid` \| `none` | a **finding**, not the request; `uid` is written by no v1 collector (RF §4.2, §7.1) |
| `invocation_set` | set of adapter | total function of `params.langs` × pinned release (RF §6.2) | each member carries a constant `runner` token |
| `deadline` | seconds | `params.timeout` | applied **per invocation** and per restore phase |

### Per-adapter obligations the collector consumes (RF §6.3; grammars are IR's)

1. A stable `runner` **token**, grammar `[a-z][a-z0-9_-]{0,31}` (RF §4.4), a **constant of the pinned
   release**, never read from the stream, the tree, `params.langs`, or the environment.
2. A total, deterministic `id → fn`, output a **prefix** of its input, identity on an unparametrized id.
3. A total `id → path`, repo-relative, `/`-separated, **the tree's bytes**; empty string where no tree
   entry matches.
4. A total mapping from the runner's terminal reports onto the eight `out` values, with `unknown` the
   defined home for anything unmapped.
5. A conforming transport (RF §6.6) preserving **four** signals per item: runner-native id, per-phase
   outcome, expected-failure polarity, **and deselection**.
6. A **`B` outcome per collected id** — that id's own outcome on the checkout of `B`, or `absent`.

Whether an adapter's `B` enumeration and `B` outcome run are **one invocation or two** is IR §11.1's:
`vitest` and `dart-test` are one; `pytest` and `swift-test` are two (RF §4.4, §6.3 item 6; PB §12 line 1103).

### Records the collector emits (shape summary only — the file-format sheet owns the grammar)

| Kind | Keys (canonical, ascending) | Cardinality |
|---|---|---|
| `base` | `id`, `out`, `path`, `runner`, `t` | one per distinct `(runner, id)` pair collected on `B` |
| `result` | `fn`, `id`, `out`, `path`, `runner`, `t` | one per distinct `(runner, id)` pair a runner reported on `T` |
| `end` | `status`, `t` | **exactly one**, the last line |

`out` domain on `result`: the eight of RF §5 — `passed`, `failed`, `error`, `skipped`, `xfail`, `xpass`,
`deselected`, `unknown`. On `base`: those eight **plus** `absent`. `absent` on a `result` record is
**malformed** (RF §4.4).

`status` domain (RF §7.3, closed set, and this is also the fold's table order):
`complete`, `base-collect-failed`, `spawn-failed`, `no-output`, `stream-invalid`, `runner-failed`,
`runner-timeout`.

---

## Algorithm

### A. RF §7.1 order of operations — steps 1–10, **verbatim**

> 1. Read policy from `origin/<trunk>`, never from the checkout: `cli.version`, `cli.dist_hash`, `params.isolation`, **`params.langs`**, **`params.timeout`**, `object_format` (§7.4 rule 1). `params.isolation` absent means `none` (§6.7); `params.timeout` absent means `1800`.
> 2. Verify its own bytes against the pinned artifact list (§7.4 rule 2). Mismatch: fail the job, write nothing.
> 3. Compute the invocation set from `params.langs` (§6.2). A language in `params.langs` the running release supports no adapter for: fail the job, write nothing — the same shape as failing its own hash check, and for the same reason, since a collector that cannot run a declared language cannot produce the floor a landing will be judged against.
> 4. Probe key visibility (§4.2) and hold the boolean.
> 5. Compute `T := git merge-tree --write-tree origin/<trunk> H`. A conflict yields no `T` and therefore no file; the collector fails the job and writes nothing. The trusted stage detects `needs-rebase` independently at step 1 of §5.4 and does not need a file to do it.
> 6. Establish the isolation boundary the request of step 1 names, **test it**, and record the boundary the test licensed — *The isolation boundary*, below, in full. One boundary configuration serves every runner (§3). The collector never silently upgrades the recorded profile; where the boundary cannot be had, the achieved profile is `none`, and precondition 1 of §7.4 rule 5 fails on that fact alone.
> 7. Check out `B`, **run the restore phase for it** (*The restore phase*, below), and collect the id set **and each id's outcome on `B`** (§4.4) **for every runner in the invocation set** — all of them, each spawned under the boundary of step 6, and **before any process has run against `T`'s content** (§7.4 rule 3). […] Every `B` collection precedes every `T` execution, without exception.
> 8. Check out `T`, detached. **Run the restore phase for it** (below) — the last phase of the run that holds the job's own network. Then, for each runner in the invocation set, spawn it as a child under the runner disposition of the boundary and read its stream over the pipe, enforcing the deadline below. Invocation order and concurrency are an implementation choice and cannot affect the file's bytes (§4.5).
> 9. Reap every process group.
> 10. Reduce each runner's stream to records, take the union, sort, fold the statuses (§7.3), and write the file by the temp-and-rename of §3.

(RF §7.1, lines 300–310. Step 7's elided middle is quoted in full under requirement **C12** below.)

### B. Numbered requirements

**Step 1 — policy**

- **C1. MUST** read `cli.version`, `cli.dist_hash`, `params.isolation`, `params.langs`, `params.timeout`
  and `object_format` from `origin/<trunk>`, **never from the checkout** (RF §7.1 step 1; PB §7.4 rule 1:
  *"the trusted stage **and the collector** read `.spine/manifest.json`, `.spine/allowed_signers` and the
  constitution's scaffolded rules from `origin/<trunk>` (`git show origin/main:.spine/manifest.json`),
  never from the checkout under test."*).
- **C2. MUST** treat `params.isolation` absent as `none` and `params.timeout` absent as `1800`
  (RF §7.1 step 1).
- **C3. REFUSE** — `params.timeout` present and not a strictly positive integer number of seconds:
  *"It is present and not a positive integer: the collector fails the job and writes nothing (step 1's
  shape)."* (RF §7.1 *The deadline*). `0` is not permitted, because *"`0` would spell 'no deadline', which
  §6.7 forbids"* (RF §13 R24). Exit status non-zero, **no file written**.
- **C4. REFUSE** — `params.isolation: "uid"` under `--ci`: disposition 1, *"the collector **refuses**: it
  fails the job and writes nothing"* (RF §7.1 line 403; the verdict block spells it
  `step 1: params.isolation = "uid" -> refuse: fail the job, write nothing (disposition 1)`).
  **MUST NOT** downgrade it to `none`. Outside `--ci` this refusal does **not** apply (**C41**).
- **C5. MUST NOT** consult any candidate-supplied value for any of these fields — the deadline included
  (RF §9, *cannot influence* row *The deadline*: *"`params.timeout` is read from trunk (§7.1), is on the
  protected floor with the rest of `.spine/**` (§7.3), and no candidate-supplied value is consulted"*).

**Step 2 — self-verification**

- **C6. MUST** verify its own bytes against the pinned artifact list (RF §7.1 step 2; PB §7.4 rule 2:
  *"each binary embeds the list's hash and verifies its own bytes against the list's entry for its platform
  at start-up"* — PB §6.7). **REFUSE** on mismatch: fail the job, write nothing.

**Step 3 — invocation set**

- **C7. MUST** compute the invocation set as a **total function of trunk's `params.langs` and the pinned
  release, and of nothing else** (RF §6.2).
- **C8. REFUSE** — a language in `params.langs` for which the running release supports no adapter: fail the
  job, write nothing (RF §7.1 step 3).
- **C9. MUST** invoke **every** member of the invocation set, in full (RF §6.2, §7.1, §11 item 9).
  *"A collector that skips one narrows the floor exactly as a `-k` would, and is non-conformant for the
  same reason."* (RF §6.2)
- **C10. MUST NOT** pass any selection argument, on `B` or on `T`: *"On `B` and on `T` alike it passes no
  selection argument of any kind — no `-k`, no `-m`, no node ids, no paths — and it invokes every member
  of the invocation set."* (RF §7.1). Selection comes **only** from the runner configuration in the tree
  under test (RF §7.1; RF §9).

**Step 4 — the key-visibility probe**

- **C11. MUST** probe key visibility at step 4 and hold the boolean for the header (RF §7.1 step 4).
  The predicate, verbatim (RF §4.2):
  > `keys_visible=false` asserts that **no signing key material of any kind** was reachable from the
  > collector process or from any process group it spawned — *every* runner invocation included: no
  > variable named `SPINE_PIPELINE_KEY` (§11, Files and refs) nor any provider-specific pipeline-key name
  > that `ci.md` fixes, **and** no signing agent or private key — `SSH_AUTH_SOCK`, `GPG_AGENT_INFO`, a
  > readable `~/.ssh` or `~/.gnupg`, the set §7.1 names when it says what a sandbox strips.
  - It is **one assertion over the whole job**, not per runner: *"the field is not per-runner, and a
    collector that strips key material for one runner and not another writes `true`."* (RF §4.2)
  - `true` is the honest negation and **MUST** be written rather than the field omitted (RF §4.2).
  - The **restore phase** adds no environment the predicate does not already cover: *"Its environment is
    the collector's own, unchanged, which is what §4.2's `keys_visible` predicate already covers by its
    first conjunct"* (RF §7.1, *The restore phase*).
  - The boundary is **not** the key-visibility control (RF §7.1, *What the boundary is not*).

**Step 5 — computing `T`**

- **C12. MUST** compute `T := git merge-tree --write-tree origin/<trunk> H` **itself**, in the untrusted
  stage (RF §7.1 step 5; PB §7.4 rule 3: *"the untrusted stage computes `T := git merge-tree --write-tree
  origin/<trunk> H` itself (`H` is the ref the run names: `intent/<ID>`, `quick/<name>` or
  `spine/upgrade-<version>`), tests a detached checkout of `T` — never `H`"*).
- **C13. REFUSE** — a merge conflict: *"A conflict yields no `T` and therefore no file; the collector fails
  the job and writes nothing."* (RF §7.1 step 5). CI §5.4 step 8 maps this to `.spine/ci.sh` **exit 2**:
  *"a non-zero exit is `needs-rebase` and exit 2 with no file, which is RF §7.1 step 5's 'the collector
  fails the job and writes nothing'."* The trusted stage detects `needs-rebase` independently at step 1 of
  PB §5.4 and does not need a file to do it (RF §7.1 step 5).
- **C14.** `T` **exists as a tree object from step 5** and that is what names the file; what must not yet
  exist is a **checkout** of it or **any process spawned under it** (RF §7.1 step 7). This is the reading
  that reconciles RF's step order with PB §7.4 rule 3's *"collects the id set on a checkout of `B` before
  `T` exists"* — see **Contradictions**, item 1.

**Step 6 — boundary (another sheet owns the mechanism)**

- **C15. MUST** establish the boundary the step-1 request names, **test it**, and record the boundary the
  test licensed (RF §7.1 step 6). One boundary configuration serves **every** runner (RF §3, §7.1 step 6).
- **C16. MUST NOT** silently upgrade the recorded profile; **MUST NOT** substitute one mechanism for
  another (RF §7.1 step 6, *The collector never upgrades, and never substitutes*).
- **C17.** Where the boundary cannot be had, the **achieved profile is `none`** (disposition 2, not a
  refusal), the collector *"tears down whatever it built"*, and precondition 1 of PB §7.4 rule 5 fails on
  that fact alone (RF §7.1 step 6, line 349, line 404).
- **C18. MUST NOT** run any runner inside a boundary whose test failed (RF §11 item 16).
- **C19.** No probe artifact survives step 6, whatever the outcome (RF §7.1, *The probe*, clause 4).

**Step 7 — `B`: the floor, and rule 3's ordering property**

- **C20. MUST** check out `B`, run the restore phase for it, and collect **both** the id set **and each
  id's outcome on `B`** — `out` on every `base` record — **for every runner in the invocation set**
  (RF §7.1 step 7, §4.4, §6.3 item 6).
- **C21. MUST** spawn every `B` invocation under the boundary of step 6 (RF §7.1 step 7).
- **C22. MUST** complete **every** `B` invocation of **every** runner before **any** `T` execution. Verbatim
  (RF §7.1 step 7):
  > The two are one invocation for some adapters and two for others (`import-resolver.md` §11.1); every
  > invocation of either kind is a `B` invocation and all of them precede every `T` execution, which is
  > what keeps rule 3's property intact — the outcome run reads trunk's code on trunk's tree with no
  > candidate process in the picture, so `out` is as far outside the candidate's reach as the id set is
  > (§9). […] That is the property rule 3 buys: no candidate can make a landed test uncollectable.
  > **Multi-runner sharpens this rather than relaxing it**: interleaving — collect on `B` with pytest, run
  > pytest on `T`, then collect on `B` with vitest — would let code the candidate ran under the first
  > runner reach the second runner's collection of the floor, which is exactly the attack rule 3 forbids.
  > Every `B` collection precedes every `T` execution, without exception.
- **C23. MUST NOT** interleave `B` collection with `T` execution across runners (same quote; RF §11 item 9).
- **C24.** `out` on a `base` record has **exactly one consumer** — RF §8.5 clause 2's `xfail`/`skipped`
  carve-out — and *"is never a pass and never evidence"* (RF §4.4).
- **C25.** `absent` is written on a `base` record when *"the `B` outcome run reported no terminal outcome
  for the pair"*; it is **not** `unknown`, and the two **MUST NOT** be merged (RF §4.4).

**Step 8 — `T`: the run under test**

- **C26. MUST** check out `T` **detached** — never `H` (RF §7.1 step 8; PB §7.4 rule 3).
- **C27. MUST** run the restore phase for `T` before the first runner invocation against it; it is *"the
  last phase of the run that holds the job's own network"* (RF §7.1 step 8).
- **C28. MUST** spawn each runner as a **child** under the **runner disposition** of the boundary and read
  its stream **over the pipe** the collector holds on the host side (RF §7.1 step 8; RF §7.1 M1 bullet:
  *"No runner stream is ever a file inside the boundary: a stream the boundary can rewrite is not
  evidence."*).
- **C29.** Invocation order and concurrency are an implementation choice and **MUST NOT** affect the file's
  bytes (RF §7.1 step 8, §4.5). The sort of RF §4.5 on `runner` bytes then `id` bytes is what removes both
  clocks: *"The file therefore does not record, and cannot be made to record, which runner ran first."*

**Step 9 — reaping**

- **C30. MUST** reap **every** process group before serializing (RF §7.1 step 9; RF §9 *cannot influence*
  row: *"All serialized by the collector after reaping every process group."*).

**Step 10 — reduce, union, sort, fold, write**

- **C31. MUST** reduce each runner's stream to records, take the **union** across runners, sort per RF §4.5,
  fold the statuses per RF §7.3, and write the file by the **temp-and-rename** of RF §3 (RF §7.1 step 10).
- **C32.** The file is written **once, after every runner has been reaped**. *"There is no append, no
  per-runner flush and no partial publish."* (RF §3)

### C. The deadline — `params.timeout` (RF §7.1 *The deadline*, §11 item 10, §13 R24)

- **C33. MUST** enforce a deadline. *"**A collector that enforces no deadline is non-conformant**, whatever
  the manifest says: the field's absence selects the default, never the absence of the control."*
- **C34.** The deadline **bounds one runner invocation**, and *"the `B` enumeration, the `B` outcome run and
  the `T` run are each an invocation."*
- **C35.** It **also bounds each of the two restore phases**, *"which are not invocations and are two per
  run whatever the invocation set holds"*.
- **C36.** Worst-case wall time, verbatim: *"so the worst-case wall time is `params.timeout` times the
  number of invocations **plus two**, and the invocations are two or three per runner depending on the
  adapter (`import-resolver.md` §11.1), not one per runner. A repository declaring three languages may
  therefore take up to nine or ten times `params.timeout` in the worst case."*
- **C37. MUST NOT** add a whole-job deadline: *"This document does not add a whole-job deadline, because
  §6.7 fixes the field's meaning as a single invocation and a second budget would be a second policy nobody
  declared."*
- **C38.** On expiry: *"the collector kills the whole process group of that invocation and reaps it; the
  run's `status` contribution is `runner-timeout` (§7.3), and **no wall-clock quantity enters the file**."*
- **C39. Three different expiry sites, three different behaviours** (RF §7.1 *The deadline*, second
  paragraph — verbatim):
  > Expiry during the **enumeration** of `B` kills it, the enumeration failed, and the fold of §7.3 yields
  > `base-collect-failed` — first in table order, and the honest one, because `status` describes the body
  > and a file with no `base` records is `base-collect-failed`'s body and not `runner-timeout`'s. Expiry
  > during the separate **`B` outcome run**, where an adapter has one, is not that: the floor is already
  > enumerated, so the run is killed, every id it had not reached takes `out: "absent"`, and the file's
  > `status` is unaffected. That asymmetry is deliberate and is the fail-closed direction — an enumeration
  > that stops early shrinks the floor, while an outcome run that stops early can only withhold exemptions
  > (§4.4, §8.5 clause 2).

  | Expiry site | Kill | `status` contribution | Body effect |
  |---|---|---|---|
  | `B` **enumeration** | kill the process group, reap | `base-collect-failed` | whole body suppressed, `ids=0` (all runners) |
  | `B` **outcome run** (adapters with a second `B` invocation) | kill the process group, reap | **none — not a status at all** | every unreached id gets `out: "absent"`; `base` section whole |
  | `T` run | kill the process group, reap | `runner-timeout` | that runner's `base` records + every `result` parsed before the kill |
  | **restore phase** (either) | kill and reap | **none** — *"contributes no `status` like any other restore outcome, and the run proceeds"* | nothing |

- **C40.** An old collector that does not know the key defaults to `1800` and *"therefore still enforces
  *a* deadline. That is the right direction: the deadline is a liveness control, never an integrity one."*

### D. RF §7.2 Reduction — verbatim

> - One `result` record per distinct `(runner, id)` pair. When a runner reports an id more than once — a
>   rerun plugin, a repeated phase — **the last terminal outcome that runner reported for that id wins**.
>   The collector transcribes; it does not adjudicate. […] The direction is fail-open and deliberate: where
>   the repo's *own* frozen configuration reruns, a `failed` followed by a `passed` is a pass, because the
>   repo chose that configuration under `C-T2` (§13, R6). Reduction never crosses runners: two runners
>   reporting one id string produce two records, and neither is the other's rerun.
> - `base` pairs are a set: duplicates at collection are reduced to one record, per runner.
> - An id whose bytes are not valid UTF-8 is not representable. **That runner** contributes **no** `result`
>   records at all and its status contribution is `stream-invalid`; other runners contribute theirs, and
>   the fold of §7.3 makes the file's `status` `stream-invalid`, so nothing credits either way. Non-UTF-8
>   test ids are unsupported in v1 (§12).

Derived requirements:

- **C41. MUST** key reduction on `(runner, id)`, never on a bare id (RF §7.2, §13 R6, §11 item 5).
- **C42. MUST** take the **last terminal outcome that runner reported** for a repeated pair — fail-open.
- **C43. MUST NOT** adjudicate: *"The collector transcribes; it does not adjudicate."*
- **C44. MUST** reduce duplicate `base` pairs to one record, per runner.
- **C45.** Non-UTF-8 id ⇒ that runner emits **no** `result` records at all and contributes `stream-invalid`;
  other runners still contribute theirs. **MUST NOT** drop only the offending record.

### E. RF §7.3 — crash, timeout, no output

**Framing rule, verbatim:**

> The collector always writes a file once `T` is known and policy has been read. The header is always
> complete and honest. The `end` record says what happened.

- **C46. MUST** write a file once `T` is known and policy has been read — even on a failing run
  (RF §7.3). Correspondingly, **no file at all** exists only for the five step-1..step-5 refusals
  (**C3, C4, C6, C8, C13**).

**Status contribution — verbatim table** (RF §7.3; evaluated **top to bottom, first match wins**, per
runner):

| `status` | Contributed when | That runner contributes |
|---|---|---|
| `complete` | The runner terminated of its own accord and its stream was parsed to a terminal event. | Its full `base` and `result` records. |
| `base-collect-failed` | The **enumeration** of the id set on the checkout of `B` failed, or its deadline expired during that enumeration. A failure of the separate `B` **outcome** run, where an adapter has one, is *not* this row (§4.4, §7.1). | **Nothing, from any runner** — see the all-or-nothing rule below. |
| `spawn-failed` | The runner could not be started at all — no runner configuration in the tree under test included. | Its `base` records; no `result` records. |
| `no-output` | The runner started and terminated but emitted no parsable stream event. | Its `base` records; no `result` records. |
| `stream-invalid` | Its stream contained an event the adapter cannot parse, or an id that is not valid UTF-8. | Its `base` records; no `result` records. |
| `runner-failed` | The runner terminated abnormally, or its stream ended mid-record. | Its `base` records; every `result` record parsed before the break. |
| `runner-timeout` | The collector's deadline expired on the `T` run and it killed that process group. | Its `base` records; every `result` record parsed before the kill. |

- **C47. MUST** evaluate the rows **top to bottom, first match wins, per runner** (RF §7.3, §13 R23).
- **C48.** `complete` requires **both** conjuncts, verbatim: *"`complete` requires **both** that the adapter
  parsed that runner's terminal session-end event **and** that no member of its process group was
  terminated by a signal; a member that dies abnormally after the terminal event yields `runner-failed`."*
- **C49. MUST NOT** use the runner's **exit code** as the discriminator, verbatim: *"The runner's *exit
  code* is never the discriminator — a red suite exits non-zero on every runner that ships, so an
  exit-code test would make `complete` unreachable for exactly the runs G1 exists to judge, and it would
  readmit the platform-divergent value §4.4 keeps out of the file (§13, R23)."* **Rejected explicitly**
  (R23): *"requiring every process-group member to exit with status 0."*

**The fold, verbatim:**

> `end.status` is `complete` **iff every** invoked runner contributed `complete`. Otherwise it is the
> **first row in this table's order, after `complete`, contributed by any runner**. The fold is over the
> table's fixed order and not over invocation order or wall time, so it is deterministic and independent
> of which runner ran first — which §4.5's determinism claim requires.

- **C50. MUST** fold over the **table's fixed order**, never over invocation order or wall time (RF §7.3,
  §11 item 11, §13 R28).
- **C51. MUST** emit **exactly one** `end` record; **MUST NOT** emit a per-runner status record
  (RF §4.4, §7.3, §12, §13 R28).

**All-or-nothing on `B`, verbatim:**

> **Collection on `B` is all-or-nothing across runners.** If *any* invoked runner's collection on `B`
> fails, the file's `status` is `base-collect-failed`, `ids=0`, and **no `base` and no `result` records are
> written at all**, from any runner. A partial `base` section is a *shrunken floor*, which is the one
> truncation that weakens rather than tightens the gate (§13, R13), and it would be indistinguishable from
> a repository that genuinely has fewer landed tests. `ids=0` here means *no `base` records follow* — the
> cardinality of `B`'s pair set is unknown and `status` carries that truth.

- **C52. MUST** suppress the **whole body** — no `base`, no `result`, from **any** runner — and write
  `ids=0` when any invoked runner's `B` **enumeration** fails (RF §7.3, §11 item 11, §13 R13).

**Failed `B` outcome run — verbatim:**

> **A failed `B` outcome run is not all-or-nothing, and is not a status at all.** Where an adapter takes
> the floor's membership from one `B` invocation and each id's `out` from a second
> (`import-resolver.md` §11.1), a failure of the second — it did not start, it died, it was killed at the
> deadline, its stream was unparsable — leaves the `base` section whole and gives every id it did not
> report a terminal outcome for `out: "absent"`. No status contribution is made for it and `end.status`
> does not move.

- **C53. MUST NOT** contribute a status for a failed `B` outcome run; **MUST** leave the `base` section
  whole and set `out: "absent"` on every id it did not report a terminal outcome for (RF §7.3, §11 item 10).

**The two safety rules — verbatim:**

> - **`status ≠ complete` ⇒ no pair counts as passed**, whatever any `result` record says, and whichever
>   runner produced it. Records are still written because they are evidence for the human who will read
>   the wire; they are never credit. […] It is also what makes the fold safe: **one hung runner fails the
>   landing, including another runner's genuinely green suite.** That is deliberate.
> - The collector's exit status is non-zero for every `status` other than `complete`, and for
>   `keys_visible=true`, so the untrusted job fails as §7.4 rule 0 requires. Writing the file anyway is
>   deliberate: a failed job that produced no file and a failed job that produced an honest one are
>   different things, and the trusted stage should be able to say which.

- **C54. MUST** still write the `result` records that were parsed, as evidence — they are never credit
  (RF §7.3, §13 R12).
- **C55. MUST** exit **non-zero** for every `status` other than `complete`, **and** for
  `keys_visible=true` (RF §7.3, §13 R10).
- **C56.** *"With several runners, `end.status` names *what* went wrong and not *which runner* it went
  wrong in. That is an accepted loss."* (RF §7.3) **MUST NOT** add a per-runner status record to close it
  (RF §13 R28, explicitly rejected, twice: a fourth record kind, and partial crediting).

### F. RF §7.4 — outside `--ci` (the solo path)

Verbatim, both paragraphs:

> The solo path (§5.4) runs the same code, and two header values are settled before any observation is
> made: **`keys_visible=true`**, out of §4.2's own predicate, because the operator's own signing key is
> reachable from the process tree that ran the tests; and `profile=none`, because **outside `--ci` the
> collector attempts no boundary at all**. A laptop is one uid and one process tree, the operator's key is
> reachable from it either way, and a boundary between a collector and a runner that both answer to the
> same person establishes nothing the header could honestly report. So `params.isolation` is not a request
> the solo collector acts on: it attempts nothing, it **refuses nothing** — a manifest declaring `uid`
> costs a solo developer no run, and disposition 1 of §7.1 is a `--ci` rule — and it writes `none`.
> Preconditions 1 and 2 of §7.4 rule 5 therefore fail by construction, which is exactly what §5.4 requires
> and costs a solo developer nothing they had.
>
> `params.langs` and `params.timeout` are read from trunk on the solo path too, exactly as in `--ci`
> (§7.4 rule 1). A solo developer's laptop does not choose its own invocation set or its own deadline, and
> a `--collect` run on a working copy whose manifest differs from trunk's uses trunk's.

- **C57.** Outside `--ci` the collector runs **the same code** (RF §7.4).
- **C58. MUST** write `keys_visible=true` outside `--ci`, settled before any observation (RF §7.4).
  It is a **derivation** from the one predicate, not a special case (RF §13 R11).
- **C59. MUST** write `profile=none` outside `--ci` and **MUST NOT** attempt any boundary — step 6 is
  skipped entirely (RF §7.4, §7.1 *Outside `--ci` none of this runs*, §11 item 16).
- **C60. MUST NOT** refuse anything on account of `params.isolation` outside `--ci`, `uid` included:
  *"it attempts nothing, it **refuses nothing**"*; *"disposition 1 of §7.1 is a `--ci` rule"* (RF §7.4).
- **C61. MUST** still read `params.langs` and `params.timeout` from trunk outside `--ci`, *"exactly as in
  `--ci`"*, and **MUST** prefer trunk's over a differing working-copy manifest (RF §7.4).
- **C62.** Outside `--ci` the restore phase **still runs**: *"The phase is not conditioned on the profile.
  Under `params.isolation: "none"`, and on the solo path where no boundary is attempted at all (§7.4), the
  restore phase still runs"* (RF §7.1, *The phase is not conditioned on the profile*).
- **C63.** Outside `--ci` there is **no artifact**: *"Locally (§5.4, solo) there is no artifact: `--land`
  reads the path directly out of the working copy."* (RF §3; RF §13 R18)
- **C64.** A pre-existing file at the final path is **overwritten without comment** — and the solo path is
  the reason the rule exists (RF §3): *"on the solo path the file lives in a working copy and survives the
  process, so a second `--collect` for one `T` would be refused because an earlier attempt was made"*, and
  nothing may remember that a previous attempt happened.
- **C65.** CLI shape: read PB §11's `spine check [--ci [--collect]]` as **`spine check [--ci] [--collect]`**
  (RF §13 R21) — *"§11 wins, so as written the solo path has no legal invocation and §5.4's whole
  *Solo developers* paragraph is unreachable. This is the only reading under which both survive"*.

### G. RF §9 — what a candidate can and cannot influence

**Conditioning clause, verbatim:** *"**The *cannot* table is conditioned on trunk-defined origin evidence
(§8.1), and says so in each row it depends on.** Every entry in it derives from *the collector wrote this
file*, and that premise is what the evidence establishes."*

**Cannot influence** (RF §9, table 1 — reasons condensed, rows verbatim in name):

| Cannot influence | Why (collector-relevant part) |
|---|---|
| That the file exists, and who authored it | The collector writes it, holding the pipe and the directory; under `container` or `uid` no runner has a write path to it. **Conditioned on origin evidence** |
| Under `container` or `uid`, and with origin evidence: **any of the six header fields** | Every one produced by the collector from its own observation or from trunk. **Under `none` the collector owns nothing** |
| **Which runners run** | The invocation set is a function of trunk's `params.langs` and the pinned release (§6.2). *"A candidate cannot add a runner, drop one, or reorder them into relevance — the sort of §4.5 removes order from the bytes"* |
| **The `runner` token on every record** | *"a constant embedded in the pinned release's adapter, never read from the runner's stream, the tree's configuration or the environment"* |
| **The deadline** | `params.timeout` read from trunk, on the protected floor, *"no candidate-supplied value is consulted"* |
| **Egress, and the restore script** | Loopback-only runners, P4 tested before any repository process ran; restore script read from `origin/<trunk>`. *"Under `none` no boundary is attempted and this row says nothing"* |
| Which tree the results are about | Trusted stage ingests only a file labelled with the `T` it computed itself; anything else is `base-moved` |
| Which release wrote it | `tool=` is the collector's own embedded constants |
| **Membership of the `B` pair set** | *"Every runner's collection happens on a checkout of `B` before any process has run against `T`'s content, and with no selection argument (§7.1 step 7). A landed test cannot be made uncollectable by any runner"* |
| **Every `base` record's `out`** | *"the `B` outcome run is a `B` invocation, so it precedes every `T` execution by §7.4 rule 3 and no candidate process exists when it runs (§7.1 step 7). A candidate therefore cannot put an id **into** the exempt set […] Taking an id **out** of the set is available to it and worth nothing: it withholds an exemption from itself."* **Conditioned on origin evidence** |
| The framing, ordering, canonical form, or the `end` record | *"All serialized by the collector after reaping every process group"* |
| Whether the file is ingested, and what G1 concludes | Trusted stage, pinned release, policy from trunk |
| Whether a human reads the landing | `C-A3: hostile` ⇒ precondition 0 fails on every run |

**Can influence** (RF §9, table 2 — collector-relevant rows):

| Can influence | Bound |
|---|---|
| Every `out` value, and every id, `fn` and `path` string in the **`result` section** | *"a runner that monkeypatches the reporter or the assertion library lies on the stream the collector faithfully records. **This is the residual §7.4 names and does not close.**"* Now **per runner** — *"a candidate that can lie in one runner lies about that runner's ids only, and the pair keying is what confines it"* |
| Which of several reported outcomes for one pair is transcribed, where trunk's own frozen config reruns | §7.2's last-terminal-outcome-wins is **fail-open**. *"The candidate cannot introduce the rerun […] but it can trigger it (§13, R6)"* |
| Whether a runner crashes, hangs, or emits nothing | **Fail-closed**: `status ≠ complete` credits nothing. *"**One runner is enough**: the fold means hanging the cheapest runner in the set fails the landing as surely as hanging them all — a denial of service against one's own landing, never a pass"* |
| Whether the deadline expires, by making a suite slow | **Fail-closed, and bounded**: *"the expiry is `runner-timeout`, which credits nothing. The only thing a candidate buys by hanging is a failed landing and, in the quick lane, an unoverridable one — a reseal excepted (§8.7)"* |
| Which ids are collected on `T`, in any runner | Adding tests permitted; removing/renaming/skipping a frozen or landed id is a G8 or G1 failure by identity. *"Moving a test from one runner to another changes its pair"* |
| **The whole file, where the run has no trunk-defined origin evidence** | *"every header field is then a forgery that §8.3 passes by construction"*. Bounded by precondition 2 (`unmet` on every run in such a repository) |
| **The whole file, under `profile=none`** | *"The collector owns nothing; any process in the job can write it."* Precondition 1 fails |

- **C66.** *"**G6 is not a repair.** It runs in the same untrusted stage and reports through the same
  collector, so a runner that lies about the suite lies about the mutants (§6.3 G6). It is a strength
  signal, never an integrity control."* (RF §9)

### H. PB §7.4 rules 0–3 — the collector-facing obligations

**Rule 0 (the trusted stage's own definition is policy).** Collector-facing clauses, verbatim:

- *"`.spine/ci.sh` is executed from `git show origin/<trunk>:.spine/ci.sh`, never from the checkout."*
- *"The probe is the untrusted job itself: it fails the run if the pipeline-key variable is visible to it,
  and the collector (rule 3) writes that assertion and its own `tool=` into the result-file header."*
- *"A run whose ingested header lacks either — the assertion, or a collector pinned by the base — fails
  precondition 2 of rule 5 below."*
- *"the untrusted job is the only job that runs on `intent/*`, `quick/*` and `spine/upgrade-*` pushes, runs
  with `permissions: contents: read` […] and receives no other secret"*.
- *"a result file from a job that cannot demonstrate a trunk-defined origin is still **ingested** — it
  simply fails auto-merge precondition 2"* (as amended 2026-08-26; RF §13 R31).
- **C67.** The collector **MUST NOT** hold a key and **MUST NOT** sign anything (PB §7.4 rule 3; RF §2:
  *"It holds no key and signs nothing."*).

**Rule 1 (policy is read from trunk).** — see **C1**. *"A candidate may change policy; that change is a
floor hit reviewed under the *old* policy, and governs only later landings."* Consequence for the
collector: **C68.** adding a language to `params.langs` **takes effect on the landing after the one that
adds it** (RF §6.2; the same behaviour `params.isolation` already has).

**Rule 2 (gates run from a pinned, hash-verified release).** — see **C6**. Verbatim: *"`.spine/ci.sh`, read
from trunk, installs and hash-verifies the collector the same way before anything else runs in the
untrusted job; a mismatch fails the run, and no result file exists to ingest."*

**Rule 3 (the graph is rebuilt from git objects, every run).** Collector-facing text, verbatim:

> the untrusted stage computes `T := git merge-tree --write-tree origin/<trunk> H` itself (`H` is the ref
> the run names: `intent/<ID>`, `quick/<name>` or `spine/upgrade-<version>`), tests a detached checkout of
> `T` — never `H` — under **the collector**. The collector is `spine check --ci --collect`: the pinned
> release, invoked by trunk's `.spine/ci.sh` (rule 0), holding no key and signing nothing. It collects the
> id set on a checkout of `B` *before* `T` exists — so no candidate can make a landed test uncollectable —
> then spawns the runner as a child, reads its machine-readable stream over a pipe, and — after reaping the
> whole process group — writes the result file itself, in a format carrying runner-native test ids. Where
> it writes it is the **isolation profile**. The collector measures what it actually got and names it in
> the header; trunk's `params.isolation` says what the repo claims to provide, and the trusted stage
> requires the two to agree — a header alone proves nothing, since a job with no boundary can write any
> header it likes

- **C69. MUST** test a detached checkout of `T`, **never `H`** (PB §7.4 rule 3; RF §7.1 step 8).
- **C70.** *"The uid or the container is the boundary, never the path: a directory mode means nothing
  against processes sharing one uid, and the threat set is every process in the job — dependency restore
  and build as much as the runner."* (PB §7.4 rule 3)
- **C71.** `spine index --fresh` is implied by `spine check --ci`; *"no SQLite file is fetched, cached or
  trusted from anywhere"* (PB §7.4 rule 3) — the collector caches nothing across runs (RF §1, *No state*).

---

## Byte-level fixities

Full ownership of the file grammar belongs to the file-format sheet; these are the fixities the
**collector's order of operations** must honour when it writes at step 10.

**Path** (RF §3):

```
.spine/cache/results/<T>.jsonl
```

- `<T>` is *"lowercase hex, full length, never abbreviated — 40 characters under `object_format: sha1`,
  64 under `sha256`"*. Extension is **exactly `.jsonl`**. *"The stem carries no prefix, suffix, branch name
  or intent id."*
- *"The filename stem equals the header's `tree=` value byte for byte."*
- **One file per `T`, covering every runner.** No per-runner file, no per-runner directory, no per-language
  suffix.

**Write protocol** (RF §3, verbatim):

> The collector creates the directory itself, writes to a temporary file in the same directory opened
> `O_CREAT|O_EXCL` under a name no other process can predict, `fsync`s it, and `rename()`s it over
> `<T>.jsonl`, replacing any file already there. **A pre-existing file at the final path is overwritten
> without comment** […] The rename is also what `O_EXCL` on the final path never bought: a reader sees
> either no file or a complete one.

**Header line** (RF §4.2; six fields, `key=value`, separated by exactly one `U+0020`, order fixed):

```
tree=<oid> base=<sha> tool=<version>+sha256:<hex64> keys_visible=<bool> profile=<profile> ids=<n>
```

- `tool=` is *"the collector's **own** embedded version and artifact-list hash"*. **C72. MUST NOT** copy
  trunk's value: *"The collector writes what it **is**, never what trunk pins. Copying the manifest's value
  would assert nothing."*
- `base=` is *"`origin/<trunk>` tip at the moment the collector read policy — for a reseal, the seal's
  `base=`"*.
- `ids=` counts the **`base` records that follow**, i.e. `(runner, id)` **pairs** (RF §4.2, §13 R2).
  **C73.** `ids=0` under `base-collect-failed` means *no `base` records follow* and the cardinality is
  unknown.
- **C74. MUST NOT** write `params.timeout` into the header: *"**`params.timeout` is not a header field.**
  […] Its expiry is recorded exactly once, as `status=runner-timeout` (§7.3)."*
- **C75. MUST NOT** write `n/a` as a header `profile=` value — it is a **seal** value only; a header
  carrying it is malformed (RF §4.2, §13 R15).

**Framing** (RF §4.1): UTF-8, no BOM; every line — *including the last* — terminated by a single LF
(`U+000A`); a CR (`U+000D`) anywhere outside a JSON string escape makes the file malformed; no blank lines,
no comment lines, no leading or trailing whitespace on any line, no bytes after the final LF; line 1 is the
header and **is not JSON**.

**Ordering** (RF §4.5, verbatim):

> 1. Header line.
> 2. Every `base` record, sorted ascending by the **bytes** of `runner`, then by the **bytes** of `id`.
> 3. Every `result` record, sorted ascending by the bytes of `runner`, then by the bytes of `id`.
> 4. The `end` record.

**C76.** Sorting on `runner` **first** is what removes the invocation order of the runners themselves —
*"the second clock multi-runner would otherwise introduce"*.

**Canonical JSON** (RF §4.3): no whitespace outside string literals; members ordered by key ascending over
UTF-16 code units (ASCII ⇒ byte order); the escape set is exactly
`"`→`\"`, `\`→`\\`, `U+0008`→`\b`, `U+0009`→`\t`, `U+000A`→`\n`, `U+000C`→`\f`, `U+000D`→`\r`, every other
code point below `U+0020` → `\u00xx` with **lowercase** hex, every other code point emitted literally as
UTF-8; no numbers, booleans, nulls, nested objects or arrays; duplicate keys malformed. **Canonical form is
required on read, not only on write.**

**Determinism claim** (RF §4.5, verbatim, and it bounds what the collector may vary):

> For a fixed `(B, T)`, a fixed collector build, a fixed invocation set, and runners that behave
> identically, a file whose `end.status` is `complete` is fully determined byte for byte. […] Two
> conforming implementations produce identical files. **The claim is conditioned on `complete`**: a run the
> deadline kills is determined only up to where the kill fell, because how many records a hung runner
> emitted before it was killed is a fact about wall time.

**No clock** (RF §1, §12, §11 item 13): **C77. MUST NOT** write any timestamp, duration, elapsed time, or
ordinal derived from wall time. *"The deadline of §7.1 is enforced with a clock and *records* nothing from
it beyond the fact that it expired."*

**No runner exit codes or signals in the file** (RF §4.4, §12): **C78. MUST NOT** record them.

---

## Error cases

Two disjoint classes. **Class R (refusals)** happen at steps 1–5, produce **no file**, and fail the job.
**Class S (status)** happen at steps 6–10, always produce a file, and are carried by `end.status`.

### Class R — refusal: fail the job, write nothing

| # | Condition | Behaviour | Exit / token | Citation |
|---|---|---|---|---|
| R-a | Collector's own bytes ≠ pinned artifact list | fail the job, **write nothing** | collector non-zero; `.spine/ci.sh` **exit 2** (no file) | RF §7.1 step 2; PB §7.4 rule 2; CI §5.2 |
| R-b | A language in `params.langs` has no adapter in the running release | fail the job, **write nothing** — *"the same shape as failing its own hash check"* | collector non-zero; ci.sh exit 2 | RF §7.1 step 3 |
| R-c | `params.isolation: "uid"` **under `--ci`** | **REFUSE** (disposition 1): fail the job, write nothing | verdict line: `step 1: params.isolation = "uid" -> refuse: fail the job, write nothing (disposition 1)` | RF §7.1 step 6 verdict, line 403; §11 item 16 |
| R-d | `params.timeout` present, not a strictly positive integer | fail the job, write nothing (step 1's shape) | collector non-zero; ci.sh exit 2 | RF §7.1 *The deadline*; §13 R24 |
| R-e | `git merge-tree --write-tree` conflict — no `T` | fail the job, write nothing | ci.sh **exit 2**; the trusted stage independently detects **`needs-rebase`** | RF §7.1 step 5; CI §5.4 step 8 |
| R-f | HEAD is detached / not on the candidate branch (`collect`) | refusal | ci.sh **exit 2** | CI §5.1, §14 R7 |

**Not a refusal (disposition 2):** any absent M1 host prerequisite, a failed boundary creation, or a failed
P1–P4 ⇒ **record `profile=none`, run the suite unisolated, continue** (RF §7.1 line 349, line 404; PB §7.4
rule 3). **A missing `.spine/restore.sh` on trunk is likewise not a prerequisite failure** — the phase is
empty, one diagnostic goes to stderr, and the run proceeds (RF §7.1 *The restore phase*; §11 item 16a).

### Class S — a file is written; `end.status` carries the failure

| # | Condition | `status` contribution | Body from that runner | Exit |
|---|---|---|---|---|
| S-a | Runner terminated of its own accord, stream parsed to a terminal event, **no** process-group member signalled | `complete` | full `base` + `result` | 0 (iff every runner) |
| S-b | `B` **enumeration** failed, or deadline expired during it | `base-collect-failed` | **nothing, from any runner**; `ids=0` | non-zero |
| S-c | Runner could not be started at all (incl. no runner configuration in the tree under test) | `spawn-failed` | its `base`; no `result` | non-zero |
| S-d | Runner started, terminated, emitted **no parsable stream event** | `no-output` | its `base`; no `result` | non-zero |
| S-e | Stream held an event the adapter cannot parse, **or** an id that is not valid UTF-8 | `stream-invalid` | its `base`; no `result` | non-zero |
| S-f | Runner terminated abnormally, or its stream ended mid-record; **or** a process-group member died abnormally after the terminal event | `runner-failed` | its `base`; every `result` parsed before the break | non-zero |
| S-g | Deadline expired on the **`T` run**; collector killed that process group | `runner-timeout` | its `base`; every `result` parsed before the kill | non-zero |
| S-h | `B` **outcome run** failed (did not start, died, killed at the deadline, unparsable stream) | **none — no status contribution** | `base` section whole; every unreached id `out: "absent"` | unaffected |
| S-i | Restore phase failed or its deadline expired | **none** | nothing; *"A non-zero exit is a diagnostic on stderr and the run proceeds"* | unaffected |
| S-j | `keys_visible=true` (solo path, or a key leaked into a `--ci` job) | — | header carries `true`; body unaffected | **non-zero** |

**Fold:** `end.status` = `complete` **iff every** invoked runner contributed `complete`; otherwise the
**first row in the table's order after `complete` contributed by any runner** (RF §7.3).

**Credit rule:** `status ≠ complete` ⇒ **no pair counts as passed**, whatever any `result` record says
(RF §5, §7.3, §13 R12).

### `.spine/ci.sh` exit codes (CI §5.2) — the wrapper the collector runs under

| Exit | Meaning | The definition's obligation |
|---|---|---|
| 0 | `install` succeeded, or the collector ran and exited 0. | Continue. |
| 1 | The collector ran, exited non-zero, and **a result file exists**. | Hand the file over anyway, then fail the job. |
| 2 | Refused. Nothing ran and **no result file exists**. | Fail the job; there is nothing to hand over. |

*"Three codes and no more […] The split that matters is 1 against 2 — *a file exists* against *no file
exists* — because it is what tells the definition whether to upload an artifact."* (CI §5.2)
The collector's own exit status is **captured, not propagated**: *"the file's existence decides between
exit 1 and exit 2"* (CI §5.4 step 9). CI §6.1: *"The upload is `if: always()` […] A red suite must reach
G1 as evidence, not vanish as a failed job."*

Collector stdio contract (CI §5.1): run *"from the repository top level with stdin from `/dev/null` and
stdout redirected to stderr"*; `collect` prints exactly `result=<repo-relative path>` on ci.sh's stdout and
nothing else.

---

## Worked examples / test vectors

### Vector 1 — the complete file (RF §10)

Setup, verbatim: repo `billingsvc`, `params.ci: github`, `params.langs: ["python", "ts"]`,
`params.isolation: container`, `params.timeout: 1800`, `object_format: sha1`, pinned `cli.version 1.4.0`.
Invocation set: `pytest` and `vitest`. Trunk collected **seven pairs** — four pytest, three vitest.

Path: `.spine/cache/results/3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28.jsonl`, complete, **20 lines**.

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

Sort checks the vector fixes (RF §10): *"every `pytest` record precedes every `vitest` one, because
`p` < `v`. Within each runner the sort is on `id` bytes, which puts `…::test_AC1_…[reduced-rate]` before
`[standard-rate]` before `[zero-rate]`, and `…half-even` before `…half-up`."*

Note for the collector: the branch's own two tests are **absent from section 2** — *"they are not trunk's,
and they enter the floor only when this intent lands."*

### Vector 2 — the same run, one runner timed out (RF §10)

pytest completed; vitest hung after its first test and the collector killed that process group at
`params.timeout`. **Both `B` collections had already succeeded, so the `base` section is whole.** The file
is byte-identical to Vector 1 except: the last four `result` lines of Vector 1 (vitest's `> renders`,
`half-even`, `half-up`) are absent — vitest contributes only the one `result` it reported before the kill —
and the `end` record reads:

```
{"status":"runner-timeout","t":"end"}
```

Full body, verbatim from RF §10 (17 lines + header + end = 19 lines), header identical to Vector 1, then
the seven `base` records of Vector 1 unchanged, then:

```
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

Reading, verbatim: *"The file is well-formed and ingestible. pytest contributed `complete`; vitest
contributed `runner-timeout`; the fold is the first non-`complete` row any runner contributed, so
`end.status` is `runner-timeout`. **`status ≠ complete`, so clause 0 fails and pytest's seven `passed`
records credit nothing** […] G1 fails and names every frozen entry and every `base` pair, in both runners.
No G8 allocation is made — a killed run says nothing about whether those ids still collect."*

The four exits from this state, verbatim: *"a fixed candidate, a raised `params.timeout` landed on trunk
under a protected review (`.spine/**` is floor, §7.3), a signed reopen, or a break-glass review recorded as
`G1=override` and counted as a freeze override — and in the quick lane, where break-glass does not reach,
only the first two (§8.7)."*

### Vector 3 — provider invariance (RF §10, *The same run, on `--ci generic`*)

Verbatim: *"Take the twenty-line `complete` file above and change one manifest field: `params.ci: generic`.
**Nothing about the file moves — same bytes, same digest, same stem** — because the collector's output is a
function of `(B, T)`, its build and its invocation set, and none of those is the provider (§4.5)."*
The only thing that moves is auto-merge precondition 2's third conjunct — trusted-stage business, not the
collector's.

### Vector 4 — the `xfail`-on-trunk case (RF §10)

Same file; trunk's `tests/billing/test_discounts.py::test_percentage_discount` carries
`@pytest.mark.xfail`. *"Its `base` record reads `out: "xfail"` and its `result` record reads `out: "xfail"`
as well. […] Every other byte of the file above is the same, `ids=7` is the same, and the two `out` members
that moved are the only difference."*

### Vector 5 — id collision across runners (RF §10)

A repository migrating from jest to vitest where both collect
`tests/core/util.test.ts > rounding > half-even`: *"That is two `base` records and two `result` records,
`ids=` counts two, and neither section is malformed — the pair is the identity, and a duplicate is a
repeated *pair*."*

### Published digests this sheet touches

| Value | Where fixed | Note |
|---|---|---|
| `sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db` | MF §8.2 — 529-byte artifact list, `shasum -a 256` | the corpus's one `dist_hash` for release `1.4.0`; appears in RF §10's `tool=` token, MF §8.2/§8.4, GR §8.1/§8.2 |
| `tool=1.4.0+sha256:6f49644fdd…744db` | RF §10 header line | the token the collector writes from **its own** embedded constants (RF §4.2) |
| `.spine/ci.sh` — 319 lines, `git hash-object` `131f13fb0312162579605999d3f9f4e90098c74c`, `sha256:d6bcf50c…` | CI §5.3, README digest table | last moved 2026-08-27: `umask 077` → `umask 022` + explicit `chmod 0700 "$WORK"`, `0755` on `$INSTALL_DIR`/`$BIN`, *"because at 077 nothing the collector writes is reachable to the mapped id `result-file.md` §7.1's M1 spawns runners under, so `profile=container` was unlicensable on every host"* |

**Known divergence, already recorded by the corpus:** `envelope-vectors.md` §8's seals carry
`tool=1.4.0+sha256:41d0e9b7c2a6538f10bd47e29c05a3f6b81d24e70c9a5b3f68d1027ae4c95b3d` — a **fabricated
placeholder** inside a signed line whose private key is not published; EV §15 and the README digest table
both say so. Do not build against it. Build against MF §8.2's value.

---

## Cross-references it depends on

| Owns | What this sheet defers to it |
|---|---|
| **File-format sheet** (RF §3, §4.1–§4.5, §5) | The full header grammar and its malformed-value rules, canonical JSON, record-kind key sets, the `runner` token grammar, the outcome enum's semantics, the sort. This sheet fixes only what the collector must do at step 10. |
| **Isolation-boundary sheet** (RF §7.1 *The isolation boundary* … *The verdict*) | M1's construction, the five host prerequisites, the overlay root, the identity source, the two network dispositions, the probe, P1–P4, dispositions 1 and 2. This sheet records only: step 6's position, that the profile is a **finding** not a request, that failure is `none` not a refusal (except `uid` under `--ci`), and that every runner invocation runs under it. |
| **Restore-phase sheet** (RF §7.1 *The restore phase* … *not conditioned on the profile*; CI §5.6) | Where the bytes come from (`origin/<trunk>:.spine/restore.sh`), the disposition it runs in, its environment, that it is `sh` at the checkout root, that a missing script is an empty phase with one stderr diagnostic. This sheet records only: it runs once per checkout, before the first invocation against it, it is bounded by `params.timeout`, and it contributes **no** record, id, `status` or exit-code read. |
| **Ingestion sheet** (RF §8.1–§8.4, §8.6, §8.7) | Provenance of the bytes, the artifact rules, labelling order, `base-moved`, G15, the undeclared-runner check at §8.3 step 3, auto-merge preconditions 1 and 2, `result-missing`/`result-malformed`, overridability. |
| **Evaluation sheet** (RF §8.5) | G1/G8 clause 0–3, the allocation, the `xfail`/`skipped` carve-out's predicate, wire tokens (`G1:` + `tok(path)`, bare `G1`). |
| **`import-resolver.md`** (§6, §11.1, §11.2–§11.5, §12) | The four `runner` tokens, per-language id grammars, `id → fn`, `id → path`, outcome mappings, **and which adapters need a second `B` invocation** (`pytest`, `swift-test`) versus one (`vitest`, `dart-test`). RF §6.4: *"where it disagrees with `import-resolver.md`, that document wins."* |
| **`manifest.md`** (§3.2, §3.3, §6.2 checks 12/12b) | Where `cli.version` and `cli.dist_hash` come from; `params.langs` monotonicity (`langs-shrank`, coverable); G16's refusal of `params.isolation: "uid"` in trunk. |
| **`ci.md`** (§5.1, §5.2, §5.3, §5.4, §5.6, §10.3, §14 R11) | `.spine/ci.sh`'s bytes, its three exit codes, its invocation contract, the artifact contract, `SPINE_ALLOWED_HOSTS`, and how trunk-defined origin evidence is obtained per provider. |
| **`gate-report.md`** (§5.8, §5.9) | `evidence.result_sha256` over these bytes, `automerge.preconditions[]`, the `G11` wire's shape. |

---

## OPEN items

Undecided owner questions. **Do not invent values for these.**

1. **RF §14 OPEN-5 — G6's reporting channel.** *"§6.3 says mutation results come 'through the same
   collector as every other test'. Mutation results are per-mutant, not per-id, and this spec deliberately
   does not overload `result` records to carry them. Four languages make the question larger rather than
   smaller: a mutation channel would owe a per-language mutant identity as well as a per-language test
   identity."* — **Directly a collector question**: v1's collector has no channel for G6.
2. **RF §14 OPEN-7 — is `params.ci` monotone in the guarantee it names?** Filed three times
   (`ci.md` OPEN-3, RF OPEN-7, `manifest.md` OPEN-1); `manifest.md` owns G16 and would carry the fix.
   *"Owner's, and a §6.7/§11 change either way."* Not a collector-behaviour question — the collector's
   bytes do not move with `params.ci` (Vector 3) — but it bounds what the run is worth.
3. **RF §14 OPEN-6 — `params.langs` removal.** *Answered* by `manifest.md` §6.2 **check 12**
   (`langs-shrank`, **coverable** by a protected review), which is **one notch weaker** than RF's own
   recommendation of an outright failure. RF says explicitly: *"a reader who implements from §14 alone
   must take check 12's Kind column, not this paragraph's recommendation."* The value to implement is
   `manifest.md`'s, and it is that sheet's, not this one's.
4. **`ci.md` OPEN-1 (the host) and OPEN-7 (the three commits)** are *"**values only** — the mechanism
   around them is normative"* (README status table). The collector's installer path depends on them.

**Closed, and named here so nobody reopens them:**
OPEN-1 (the deadline — settled: `params.timeout`, seconds, trunk-read, absent ⇒ `1800`, per invocation,
expiry ⇒ `runner-timeout`, no deadline ⇒ non-conformant);
OPEN-2 (four languages, Kotlin dropped, `kotlin`/`gradle`/`junit`/`kotest` **reserved**);
OPEN-3 (multi-runner ships; identity is `(runner, id)`; one file; header unchanged);
OPEN-4 (a reseal runs the suite and is exempt from nothing);
OPEN-8 (closed 2026-08-27 — `skipped` joins `xfail` in one carve-out over two values);
OPEN-9 (closed 2026-08-27 — network namespace, loopback-only runners, restore phase, P4).

---

## Contradictions found

1. **PB §7.4 rule 3 vs RF §7.1 steps 5/7 — when `T` "exists".**
   PB: *"It collects the id set on a checkout of `B` **before `T` exists**"* (PB §7.4 rule 3, line 855).
   RF: step 5 computes `T` **before** step 7's `B` collection, and reconciles explicitly —
   *"`T` exists as a tree object from step 5, which is what names the file; what must not yet exist is a
   checkout of it, or any process spawned under it."* (RF §7.1 step 7).
   **Resolution:** the spec resolves PB's ambiguity; implement RF's step order. PB's *"before `T` exists"*
   means *before a checkout of `T` or any process under it*.

2. **PB §7.4 rule 3 speaks of "the runner", singular, throughout.**
   *"then spawns **the runner** as a child, reads its machine-readable stream over a pipe"* (PB §7.4 rule 3).
   RF §6.2, §7.1 steps 7–8 and §13 R26 require **every** member of the invocation set. PB §12's own v0.19
   note (line 1081, decision 4) says a repository may run several runners. **Resolution:** multi-runner;
   PB rule 3's singular is pre-v0.19 prose that decision 4 supersedes. Report as a PB defect.

3. **PB §12 says the probe passes "three tests"; PB §7.4 rule 3 and RF §7.1 say four.**
   PB line 1109: *"`profile=container` stopped being an outcome with no algorithm: it is now a boundary the
   collector builds and **three tests** it must pass."*
   PB §7.4 rule 3 (line 861): *"the collector may write `container` only after a probe boundary has passed
   **four tests** it ran and could have failed — containment of the result directory, an identity the
   *host* confirms, separation from the collector's own process and root, and **no egress**."*
   RF §7.1 and §11 item 16: **P1, P2, P3 and P4**, all four.
   **Resolution:** four. PB §12's "three" is a stale count left behind by the 2026-08-27 addition of P4
   (RF §13 R34, OPEN-9). Report as a PB defect — a citation-surface leak of exactly the kind README's
   review-4 row describes.

4. **PB §11 spells `keys_visible=false` as a literal; RF §4.2 spells it `true | false`.**
   PB §11 (line 1012): *"first line `tree=<oid> base=<sha> tool=<version>+sha256:<dist_hash>
   **keys_visible=false** profile=container\|uid\|none ids=<n>`"*.
   RF §4.2 row 4 gives the grammar as `true` \| `false`, and RF §7.4 **requires** `true` on the solo path.
   RF §13 R10 records this as the resolution: *"The collector writes `keys_visible=true` and exits
   non-zero; an ingested `true` fails precondition 2 and raises the `G11` wire."*
   **Resolution note:** RF §1 says *"Where this document and §11 disagree, §11 wins"*, and a literal reading
   of PB §11 would make `true` unwritable and RF §7.4's solo path unimplementable. R10 is the live reading;
   implement `true | false`. Flag as an unclosed PB §11 defect.

5. **PB §11's body description omits the `end` record.**
   PB §11 (line 1012): *"then the id set collected on `B` — **each id with its own outcome there** […] —
   then one record per runner-native id"*. It names no terminator.
   RF §4.1/§4.5 require exactly one `end` record as the last line, and RF §13 R1 gives the reason:
   *"A terminator is required because truncation of the …"*; RF §4.5 makes a missing `end` malformed.
   RF §14's third amendment note says the `each id with its own outcome` half **has been made**; the `end`
   half is still absent from §11. **Resolution:** RF §4.5 governs; PB §11 is under-specified rather than
   contradictory, but an implementer reading PB §11 alone writes an unterminated file.

6. **`envelope-vectors.md` §8's `tool=` disagrees with the corpus's one `dist_hash`.**
   EV §8: `tool=1.4.0+sha256:41d0e9b7c2a6538f10bd47e29c05a3f6b81d24e70c9a5b3f68d1027ae4c95b3d`.
   MF §8.2 / GR §8 / RF §10: `…+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db`.
   **Already recorded** by EV §15 and the README digest table as a fabricated placeholder inside signed
   lines whose keys are unpublished; *"All three are 64 hex, so no byte count in §8 moves when they are."*
   Not a new defect — listed so a collector implementer building `tool=` does not pick the wrong constant.

7. **PB §7.1's untrusted-stage Network cell vs RF's `profile=none` path.**
   PB §7.1 states flatly: *"**none for anything the candidate runs**: every runner invocation is spawned in
   a network namespace holding only loopback."* RF §7.1 disposition 2 and §7.4 make `profile=none` an
   ordinary outcome — *"records `none`, **runs the suite unisolated**"* (PB §7.4 rule 3), i.e. with the
   job's network. **Resolution:** RF is the mechanism spec and resolves it: PB §7.1's cell describes what
   `container` enforces; under `none` (a failed or unattempted boundary, or the solo path) no namespace
   exists and no egress control holds. RF §9's *Egress* row says so explicitly: *"Under `none` no boundary
   is attempted and this row says nothing."* PB §7.1's unqualified *"every runner invocation"* is a
   defect of scope.
