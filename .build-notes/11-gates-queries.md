# Sheet 11 — Gates as queries: all sixteen gates, their inputs, verdicts and wires

**Scope.** What `spine check` must eventually compute: the sixteen gate ids, what each queries, what each reads, the verdict vocabulary each may write, the wire each raises (token, `class`, `kind`), who may override it, and which document owns its algorithm. Plus the two orderings PB §11 fixes (numeric for `gates[]`/`Spine-Gates`, byte-order for `wires`), the protected floor, and the six trusted-execution rules 0–5.

**Citation convention.** `PB` = `PLAYBOOK.md`; `GR` = `docs/spec/gate-report.md`; `RF` = `docs/spec/result-file.md`; `MF` = `docs/spec/manifest.md`; `CN` = `constitution.md`; `ID` = `intent-doc.md`; `IR` = `import-resolver.md`; `CI` = `ci.md`; `DM` = `dump.md`; `EV` = `envelope-vectors.md`.

**Precedence rule this sheet obeys** (`docs/spec/README.md` line 9): "Where prose here and the playbook's §11 disagree, §11 still wins — report it as a defect in one of them." Otherwise the spec resolves PB's ambiguity.

---

## Sources read

| File | Section | Lines | Read |
|---|---|---|---|
| `PLAYBOOK.md` | §2.1 the twelve scaffolded rules (block) | 118–166 | full |
| `PLAYBOOK.md` | §5.2 Tripwires (tiered auto-merge) | 367–386 | full |
| `PLAYBOOK.md` | §5.3 Roles summary (untrusted/trusted stage rows) | 387–399 | full |
| `PLAYBOOK.md` | §6.2 Derivation sources (G13 wire row, graph schema) | 596–638 | full |
| `PLAYBOOK.md` | §6.3 Gates as queries — the whole G1..G16 table + closing paragraph | 640–685 | full |
| `PLAYBOOK.md` | §7.3 The protected floor | 835–846 | full |
| `PLAYBOOK.md` | §7.4 Trusted execution, rules 0–5 + the three closing paragraphs | 848–880 | full |
| `PLAYBOOK.md` | §7.6 Break-glass, not backdoors | 892–894 | full |
| `PLAYBOOK.md` | §11 Vocabulary (trailers, files/refs, wire aggregation, signerless overlay, break-glass overlay, landings that run no suite, subject lines, states, Gates, CLI, git requirements) | 983–1039 | full |
| `docs/spec/README.md` | index, status, settled decisions, published digests | 1–88 | full |
| `docs/spec/gate-report.md` | §4.3 exit codes | 160–181 | full |
| `docs/spec/gate-report.md` | §5.6–§5.6.2 gates array, status domain, which gates run | 435–525 | full |
| `docs/spec/gate-report.md` | §5.7 `floor_hits` | 526–533 | full |
| `docs/spec/gate-report.md` | §5.8 `automerge` | 534–575 | full |
| `docs/spec/gate-report.md` | §5.9–§5.10 `evidence`, `run` | 576–625 | full |
| `docs/spec/gate-report.md` | §6 Wires — §6.1 array, §6.2 token, §6.3 the wire each gate raises | 628–715 | full |
| `docs/spec/gate-report.md` | §7 determinism rules | 718–734 | full |
| `docs/spec/gate-report.md` | §10 owner decisions, §11 out of scope | 1133–1167 | full |
| `docs/spec/result-file.md` | §8 Ingestion (§8.1–§8.7) | 461–579 | full |
| `docs/spec/result-file.md` | §14 OPEN | 811–831 | full |
| `docs/spec/manifest.md` | §4.8 G13 (§4.8.1–§4.8.7 head) | 422–570 | full |
| `docs/spec/manifest.md` | §5 G14 (§5.1–§5.11) | 618–834 | full |
| `docs/spec/manifest.md` | §6.1–§6.3 G16 shape and checks; §6.10 verdict | 838–904, 1038–1045 | full |
| `docs/spec/manifest.md` | §13 OPEN | 1504–1513 | full |

---

## Data model

### 1. Gate identity

| Field | Type | Domain | Default | Required |
|---|---|---|---|---|
| gate id | string | `"G1"` … `"G16"` (GR §5.6, §6.1) | — | yes |
| family | enum (public vocabulary) | `Integrity` (G1, G5, G8, G9, G10), `Drift` (G2, G7), `Freshness` (G3, G4, G11), `Strength` (G6, G12), `Authority` (G13, G14, G15, G16) — PB §11 *Gates* | — | yes |
| — | — | "**five families** are the public vocabulary … G-numbers are internal check IDs" (PB §6.3) | | |

### 2. `gates[]` entry (GR §5.6)

| Member | Type | Domain | Default | Required |
|---|---|---|---|---|
| `gate` | string | `"G1"` … `"G16"` | — | yes |
| `status` | string | `"pass"` \| `"override"` \| `"fail"` | — | yes |

Sealed vocabulary is narrower: a `Spine-Gates` entry is `pass` or `override` only (PB §11). `fail` is GR's third value "for evaluations that do not seal" (GR §5.6.1).

### 3. `wires[]` entry (GR §6.1)

| Member | Type | Domain | Presence | Required |
|---|---|---|---|---|
| `gate` | string | `"G1"` … `"G16"` | always | yes |
| `path` | string | `esc`-encoded path; for `G13` a commit oid (lowercase hex at `object_format` length) | present iff the wire names a path, or a commit for G13 | conditional |
| `class` | string | `"tripwire"` \| `"protected"` | always | yes |
| `kind` | string | `"finding"` \| `"advisory"` \| `"warn"` | always | yes |

Uniqueness key: `(gate, path)`, the pathless case a distinct key (GR §6.1).

### 4. Wire token (the string a human signs over) — GR §6.2

* pathless: `G<n>`
* path-bearing: `G<n>` + `:` + `tok(path)`

### 5. `automerge` record (GR §5.8)

| Member | Type | Domain | Required |
|---|---|---|---|
| `requested` | boolean | `policy.rules.c_m4 == "on"` | yes |
| `preconditions` | array of 5 | `{"id": 0..4, "status": "met" \| "unmet" \| "exempt"}`, `id` ascending | yes |
| `effective` | boolean | `requested` AND every precondition `"met"` or `"exempt"` | yes |

### 6. `evidence` (GR §5.9) — present iff a result file was ingested

`result_sha256` (string `"sha256:<hex>"`), `collector.version` (string), `collector.dist_hash` (string `"sha256:<hex>"`), `keys_visible` (boolean), `ids` (integer; an id is a `(runner, id)` pair).

### 7. `floor_hits` (GR §5.7)

Array of `esc`-encoded paths, deduplicated, sorted ascending by encoded bytes.

### 8. Constitution rules the gates read (PB §2.1, lines 121–161, verbatim ids/keys/defaults)

| Rule | Key = shipped value | `enforced_by` |
|---|---|---|
| C-A1 | `mode = team` | `spine:G13` |
| C-A2 | `protected = adr/` | `spine:G14` |
| C-A3 | `threat.candidate = hostile` | `spine:G11` |
| C-M1 | `merge.strategy = merge` | `spine:G9` |
| C-M2 | `merge.reverify = full` | `spine:G11` |
| C-M3 | `merge.reverify_limit = 3` | `spine:G11` |
| C-M4 | `merge.auto = off` | `spine:G11` |
| C-Q1 | `quick.paths = docs/` | `spine:G2` |
| C-Q2 | `quick.max_lines = 400` | `spine:G2` |
| C-T1 | `test.roots = <per params.langs>` | `spine:G8` |
| C-T2 | `test.support = <per params.langs>` | `spine:G8` |
| C-T3 | `test.framework_isolation = on` | `spine:G8` |

### 9. Landing shapes (the four the gate set is indexed by — GR §5.6.2)

`gated land` · `tombstone` · `quick / lifecycle land` (upgrade, rollback, uninstall, re-init — all ride the quick lane, PB §11) · `reseal`.

---

## Algorithm

### A. The sixteen gates — the master table

Columns: what it queries · what it reads · verdict vocabulary · wire (token / `class` / `kind`) · overridable by · owning document.

| # | Gate | Query, in words (PB §6.3) | Reads | Verdicts | Wire token · class · kind | Overridable by | Algorithm owned by |
|---|---|---|---|---|---|---|---|
| G1 | Integrity — Coverage | Every AC of a `tests-approved`+ intent has ≥1 `verified_by` edge with a collected id, and every frozen test id and every id trunk collected on `B` is reported *passed* in a result file trunk's collector wrote and labelled with `T`, save the two carve-outs | ingested result file (`base` + `result` records, `end.status`), `Spine-Test` lines of the binding approval, graph `has_ac`/`verified_by`, the `B` id set | pass / override / fail | `G1:`+`tok(path)` per-id; bare `G1` for the five pathless findings · `protected` · `finding` always | break-glass (PB §7.6); on a reseal, the reseal's own `class=protected` review naming the token | RF §8.5, §8.7 (evaluation, tokens); GR §5.6.1, §6.3 (status, class) |
| G2 | Drift — Containment | `modifies` of the synthetic merge ⊆ declared `expected` touchpoints; any `forbidden` hit is a hard fail | intent touchpoints (both polarities), `git diff --numstat --no-renames` over `merge-base..Hc`, package-manifest paths per language, `C-Q1`, `C-Q2`, floor + spine-owned path sets | pass / override / fail | `G2:`+`tok(path)` per drift path, per `forbidden` hit, per changed package manifest; **bare `G2`** for the diff-size sub-check · `tripwire` · `warn` under calibration, `finding` otherwise; a `forbidden` hit is `finding` in every mode | break-glass; warn-mode does not block | PB §6.3 (semantics), ID §6.3 (`spine_match`), IR (manifest paths), GR §6.3 (token/class) |
| G3 | Freshness — Staleness | An in-flight intent older than ~14 days (committer dates) is flagged | committer date of `objects.base`; the 1 209 600-second constant of the pinned release | pass / override / fail | bare `G3` · `tripwire` · `warn` under calibration, `finding` otherwise | break-glass; warn-mode | PB §6.3; GR §9.8, §6.3 |
| G4 | Freshness — Currency | An in-flight intent `built_under` a constitution bump flagged `resign`, or stamped with a template version below the manifest's `resign` floor, trips a wire | `built_under` edge, constitution version, manifest `templates`/`resign` maps (indexed **by variant**, `intent@2` style) | pass / override / fail | bare `G4` · `tripwire` · `finding` | break-glass | PB §6.3, §6.7; MF §3.6; GR §6.3 |
| G5 | Integrity — Orphans | A `verified_by` edge to a nonexistent AC; a pragma outside every frozen blob; a pragma first appearing after its intent's approval (`attributed: false`) | graph `verified_by` edges + `attributed`, frozen blob set, pragma sites | pass / override / fail | `G5:`+`tok(path)` — the blob the pragma sits in; **one wire per path** (two pragmas in one blob collapse) · `tripwire` · `finding` | **not** break-glass-bypassable (PB §7.6: "never G5"); a tripwire review naming the token | PB §6.3; IR §12 (pragma grammar + join); GR §6.3 |
| G6 | Strength — Mutation | Mutate the implementation; if AC tests stay green they are too weak | — (*roadmap 5, not v1*) | — | — (no `gates` entry, no `wires` entry in any version-1 report) | on PB §7.6's list, but in v1 there is no entry to mark | PB §6.3; GR §5.6.2, §6.3 |
| G7 | Drift — Interference | **soft:** `expected ∩ expected` → surfaced to both owners. **hard:** the integrated diff ∩ another intent's `forbidden` or frozen set. **ground-moved:** trunk moved since the binding approval's `base=` ∩ touchpoints → diagnostic; ∩ forbidden → wire | declared sets of other in-flight intents over a fresh fetch of `refs/heads/intent/*`; the integrated diff; the lease registry | pass / override / fail | soft: `G7:`+`tok(path)` · `tripwire` · `warn`/`finding`. hard: `G7:`+`tok(path)` · **`protected`** · `finding` in every mode | break-glass (both clauses are on the list); the hard clause never warns | PB §6.3, §5.4; GR §6.3 |
| G8 | Integrity — Freeze | For each frozen `(blob, path)`: `T`'s blob equals it, or equals trunk's; plus `C-T3` tree grep; plus the landed-id clause; plus `--ci` closure ⊆ `Spine-Frozen`; plus intent blob equals the signed blob | approval tree blobs, trunk blobs, `T` blobs, `Spine-Frozen`/`Spine-Test`, result file `base` records, recomputed freeze closure (IR) | pass / override / fail | `G8:`+`tok(path)` **always**. class **per clause**: `tripwire` for harness-moved; `protected` for branch-edited-before-approval, the landed-id clause, and `C-T3` · `finding`, never `warn` | break-glass; on a reseal the reseal's own protected review | PB §6.3, §4.3; RF §8.5 clause 2 (allocation); IR (closure); GR §6.3 |
| G9 | Integrity — Ledger | First-parent walk of trunk: every commit is the root or a valid landing (envelope parses, fenced bytes hash to `blob=`, seal's `base=` is the first parent, every `-Sig` verifies against the keyring at `base=`, `Spine-Gates` entries are `pass` or `override` and none is G10, reviews cover, the tree rule, exactly one `land` per intent id, subject recomputes) | trunk first-parent history, envelopes, keyring at each `base=`, `merge-tree`, `git=` | pass / override / fail (raises **no wire**) | — (failures are refusals and index states) | **never** (PB §7.6: "never G5, G9, G10, G11") | PB §6.3, §5.5; EV (envelope grammar); GR §5.6.1, §9.14 |
| G10 | Integrity — Reconstruction | Push `L` into scratch clone `S`, clone `--no-local --no-hardlinks file://S`, index both sides, canonical `--dump` both sides, diff must be empty | git objects only; both sides pinned to the runner's trust root; `GIT_CONFIG_GLOBAL=/dev/null`, no network, default refs only | — (never in `gates`, never in `wires`) | — | **never**; failure **refuses the push** | PB §6.3; DM (dump format) |
| G11 | Freshness — Base currency | A gate record is consumable only while trunk == `base` and the branch == `head`; the ref update is the check. Also owns rule 5's auto-merge availability wire | trunk tip, branch tip, `C-M2`, `C-M3`, `C-M4`, `C-A3`, rule-5 preconditions 0–4 | pass / override / fail (own check failure ends the run before a report seals) | bare `G11` · `tripwire` · **`advisory`, always** | never bypassable by break-glass (PB §7.6); discharged by a `class=tripwire` landing-review with mandatory `reason=` | PB §7.4 rule 5, §5.2; GR §5.8, §6.3; RF §8.4 |
| G12 | Strength — Red at approval | `red=k/n` recorded at `--approve`, measured with the intent's `expected` paths restored to base; `k = 0` is a wire **at approval** | approval-time base-restored tree; at landing only the copied approve line's `red=` well-formedness | pass / override / fail | bare `G12` · `tripwire` · `finding`. Raised by `--approve`, **never by `--land`**: no v1 landing report carries a `G12` wire and `gates[].G12` reads `pass` | break-glass | PB §6.3; GR §6.3 |
| G13 | Authority — Signers | Every human signature verifies under its namespace against the governing keyring; ~a dozen distinct refusals (mode, reviewer ≠ signer, seal-namespace exclusivity, duplicate event lines, voided approvals, reopen `voids=`, withdraw key, signerless cardinality, chain rule) | keyring `K` (trunk tip in flight / seal's `base=` landed), event commits `E`, the bound statements `A`, `mode`, `B`, `Hc`, intent blob | pass / override / fail | `G13:` + commit oid (lowercase hex at `object_format` length; `esc` and `tok` are the identity) · **`protected`** · `finding`. **Exactly one check is coverable** | **never** break-glass ("never Authority"); the one coverable finding takes a protected review naming `G13:<oid>` | **MF §4.8** (13 ordered checks, statuses, verdict) |
| G14 | Authority — Floor | The `merge-base..head` diff — renames, deletions, mode changes, symlinks (`120000`), submodule pointers (`160000`) included, paths casefolded — ∩ (shipped floor ∪ `C-A2` ∪ manifest `paths.*` at `B`) = ∅, or a `Spine-Review class=protected` verifies | `mb..Hc` raw diff, `F0` (release constant), `effective(C-A2)` at `B`, `E(M_B)`, `paths(T)` for the collision clause | pass / override / fail | `G14:`+`tok(path)`, exactly one per `floor_hits` entry and **no other `G14` entry** · **`protected`** · `finding` | **never** break-glass; a `class=protected` review naming every token. `paths-shrank`, `c-a2-shrank`, `c-a2-bracket-case` are **outright** | **MF §5** |
| G15 | Authority — Tool | The running binary's platform artifact **is listed in** trunk's pinned `dist_hash` artifact list — a membership test, never a comparison | manifest `cli.version` + `cli.dist_hash` at trunk, the running binary, the ingested header's collector `tool=`, the seal | pass / override / fail (raises **no wire**) | — | **never**, "in any mode, by anyone" | PB §6.3; MF §3.2 (the list); RF §8.3 step 2 |
| G16 | Authority — Scaffold | Every spine-owned path's blob equals its manifest blob or the path is `user-modified`; the manifest blob changes only under a signed `Spine-Upgrade`; floor-relevant fields never shrink; no `valid-before=`/`valid-after=`; no staging residue; the `to=none`/`from=none` clauses; on a rollback, the restoration rule | `M_T`, `M_B`, `K_T`, `K_B`, `T`, the copied `Spine-Upgrade`, the constitution, the `from-manifest=` ancestor | pass / override / fail | `G16:`+`tok(path)` where a path is implicated, **bare `G16`** where none is · **`protected`** · `finding` | **never** break-glass; coverable checks take a protected review naming the token | **MF §6** (17 ordered checks + rollback rule + verdict) |

### B. Which gates run — normative table (GR §5.6.2)

| Gate | gated land | tombstone | quick / lifecycle land | reseal |
|---|---|---|---|---|
| G1 Coverage | ✓ | — | ✓ | ✓ |
| G2 Containment | ✓ | — | ✓ | ✓ |
| G3 Staleness | ✓ | — | — | — |
| G4 Currency | ✓ | — | — | — |
| G5 Orphans | ✓ | — | ✓ | ✓ |
| G6 Mutation | never in a version-1 report | never | never | never |
| G7 Interference | ✓ | — | ✓ | ✓ |
| G8 Freeze | ✓ | — | ✓ | ✓ |
| G9 Ledger | ✓ | ✓ | ✓ | ✓ |
| G10 Reconstruction | never in the report | never | never | never |
| G11 Base currency | ✓ | — | ✓ | ✓ |
| G12 Red at approval | ✓ | — | — | — |
| G13 Signers | ✓ | ✓ | ✓ | ✓ |
| G14 Floor | ✓ | ✓ | ✓ | ✓ |
| G15 Tool | ✓ | ✓ | ✓ | ✓ |
| G16 Scaffold | ✓ | — | ✓ | ✓ |

**R1 (MUST).** A gate runs iff every input its PB §6.3 check reads exists for this landing (GR §5.6.2). Implement the table above literally; it is the resolution of PB's under-specification (GR §9.12).

**R2 (MUST).** A tombstone's `Spine-Gates` lists exactly G9, G13, G14, G15 (PB §5.4 step 2, PB §11; GR §5.6.2).

**R3 (MUST).** A **reseal runs the suite**: G1 evaluates every clause, `evidence` is present, and `profile` is whatever the ingested header reports — "never `n/a`" (PB §7.4 rule 5; GR §5.6.2; RF §8.6).

**R4 (MUST NOT).** No version-1 report may contain a `gates` entry or a `wires` entry for **G6** (GR §5.6.2, §6.3).

**R5 (MUST NOT).** G10's result is never in `gates` and never in `Spine-Gates` — "it runs after the seal, and its own result cannot be inside the message `L`'s seal covers" (GR §5.6.1, PB §11).

**R6 (MUST).** `gates[].G9` records the **pre-build** ledger walk (PB §5.4 step 3; on a tombstone, the binding walk of step 2), not the walk over `L` at step 5 (GR §5.6.1, §9.14).

### C. Computing the wire set

**R7 (MUST).** "**The complete wire set is computed before any lane routes it**, and it is computed the same way for every landing that runs gates — gated, quick and lifecycle alike. Lane decides the ceremony; it never decides which wires exist." (PB §11, verbatim.)

**R8 (MUST).** Wires accumulate; `(gate, path)` is the uniqueness key with the pathless case a distinct key. On a duplicate key the entries collapse: surviving `class` is `"protected"` if either was; surviving `kind` is the strongest of `finding` > `advisory` > `warn` (GR §6.1).

**R9 (MUST).** The collapse can never merge an advisory into a finding in v1: the only advisory-bearing gate is G11 and G11 raises no findings (GR §6.1).

**R10 (MUST).** `warn` wires "enter the report's wire set and `wires=`" and do not block on their own; only G2, G3 and G7's **soft** clause may produce `warn`. "a `forbidden` hit, and G7's hard lease over another intent's forbidden or frozen set, block in every mode" (PB §6.3 closing paragraph, PB §11 *Gates*, GR §6.1).

**R11 (MUST).** Warn-before-block applies to **G2, G3 and G7's soft clause only** (PB §11). Every other gate blocks from day one (PB §6.3).

**R12 (MUST).** The rule-5 `G11` wire is raised on **every landing rule 5 applies to** — every landing but a tombstone. Under the shipped defaults (`C-A3: hostile`, `C-M4: off`) it is present in every wire set of every landing that tests anything, in every lane (PB §11 *Wire aggregation*; GR §5.8).

**R13 (MUST).** `C-M4 == off` and "a precondition is unmet" are two reasons that produce **one** `(G11, pathless)` entry. "An implementation that emits two `G11` entries produces a wire array — and therefore a `wires=` line and an `envelope=` — that no conforming implementation reproduces" (GR §5.8; RF §8.4: "**One wire, however many conjuncts failed**").

**R14 (MUST NOT / REFUSE).** The rule-5 advisory MUST NOT be spelled `G1`. PB §11: "It is never spelled `G1`: a `G1` wire is a finding that named tests did not pass, and the two must never share a token a reviewer signs over." A `G1` wire in a version-1 report is **always** a `finding`; a `G11` wire is **always** an `advisory` (GR §6.1, §5.8, §9.18).

**R15 (MUST).** The `reason=` distinguishing the two G11 reasons lives in the review line only; it is **not** a report member (GR §5.8, §9.13).

### D. Aggregation and review state (PB §11, verbatim rules)

**R16 (MUST).** "wires accumulate, **`protected` dominates `tripwire`, and a landing has exactly one review state.** A `protected` wire anywhere in the set makes the landing `protected-review` … There is no first-match rule and no combined state." (PB §11.)

**R17 (MUST).** The discharging review's signed `wires=` "must cover the complete set, not merely the wires of its own class. A gate report whose wire set is not wholly covered by the review's `wires=` is not consumable" (PB §11). Containment is over the **union** of the discharging reviews' `wires=`, byte-for-byte over wire tokens, and includes `warn` and `advisory` entries (GR §6.2).

**R18 (MUST).** A review's `wires=` may name tokens absent from the report (a review signed against a larger earlier set), subject to PB §5.4's retention rules (GR §6.2).

**R19 (MUST).** **Signerless overlay, evaluated after aggregation.** A landing with no signer — every quick-lane landing, every reseal — carries **at least two distinct `class=protected` reviews in team mode**, whatever class the wire set produced, and **one** in solo mode. "a floor and never an exact count" (PB §11).

**R20 (MUST).** **Break-glass is an overlay, not a class in the aggregation ordering.** It records which gates a human chose to bypass; it never relaxes who must sign. A landing that hits the floor *and* needs a G1/G8 override takes its `class=protected` review with team-mode reviewer separation intact **and** a separately signed `class=break-glass` review — "two reviews, one state, no contradiction" (PB §11).

**R21 (MUST).** A **reseal** cannot reach break-glass; its `class=protected` review (two in team mode, one in solo) is what admits its G1 and G8 findings, and there is no break-glass review to sign (PB §11, PB §5.5, PB §7.6).

**R22 (MUST NOT).** G14 MUST NOT be treated as bypassable by break-glass: "the floor's authorization is a property of the landing, not of the emergency" (PB §11; MF §5.10).

**R23 (MUST).** A `class=tripwire` `G11` wire alone does **not** make the landing `protected-review`, "though any genuine protected wire in the same set still does" (PB §11).

### E. Verdict assignment — the status domain (GR §5.6.1)

**R24 (MUST).** `pass` — the gate ran and produced no *finding*, and no break-glass review names it. "A gate may read `pass` while having raised wires" (GR §5.6.1). PB §11 states the same for the sealed line: "a gate that ran and passed its own check reads `=pass` even when it raised a wire — the rule-5 `G11` precondition wire is not a finding about G11."

**R25 (MUST).** `override` — the gate ran and either (a) produced at least one finding and **every** finding is covered by a signed review whose class admits that wire, or (b) it is named in the `wires=` of a `class=break-glass` review, among the eight gates PB §7.6 permits — G1, G2, G3, G4, G6, G7, G8, G12 (GR §5.6.1).

**R26 (MUST).** `fail` — the gate ran, produced at least one finding, and at least one is uncovered (GR §5.6.1).

**R27 (MUST / REFUSE).** "**A report containing any `fail` is a non-landing report.** … A run that would seal a report containing a `fail` refuses: status **`report-not-landable`**. A landing whose `Spine-Gates` copies a `fail` is malformed and G9 indexes it `unattested`" (GR §5.6.1, verbatim).

**R28 (MUST).** **Outright findings.** An outright finding is one limb (a) never reaches: no review class admits it, so the gate reads `fail` whatever any `Spine-Review` names. Limb (b) still applies where PB §7.6 lists the gate, and only there (GR §5.6.1).

The v1 outright set, verbatim in substance from GR §5.6.1:

| Gate | Outright findings | On PB §7.6's bypass list? |
|---|---|---|
| G1 | **every** finding — `result-missing`, `result-malformed`, a frozen id not passed, a landed id gone from `T`'s collection, a landed id that collected and did not pass | **yes** — a `class=break-glass` review naming `G1` reads `override` |
| G8 | a `C-T1`/`C-T2`/runner-config or approval-frozen path whose blob in `T` differs from **both** the approval tree and trunk; the intent blob differing from the signed blob | **yes** |
| G13 | **every finding but one** (list in §Error cases) | **no** — Authority is never on the list |
| G14 | `paths-shrank`, `c-a2-shrank`, `c-a2-bracket-case` | **no** — `fail` is terminal |
| G16 | checks 1–8, 10, 11, 12b, 16, 17, and every clause of the rollback restoration rule | **no** — `fail` is terminal |
| G1, G8 **on a `Spine-Event: reseal` landing** | **none** — both rows are suspended; every G1/G8 finding is admitted by the reseal's own `class=protected` review naming the token, and the gate reads `override` | n/a — break-glass is unavailable to a reseal |

**R29 (MUST).** **Outright is a coverage rule, never a containment rule.** A landing carrying an outright wire that reaches a review state still needs that wire **named** in the review's `wires=` to be consumable, and naming it still does not make the gate `override` (GR §5.6.1).

**R30 (MUST).** A break-glass bypass reads `=override` whether or not the gate produced a finding. G9's check reads override → named, never named → override (GR §5.6.1; PB §7.6: "The bypassed gates are likewise marked `=override`").

**R31 (MUST).** G13's, G14's and G16's outright findings stay outright on **every** landing shape, a reseal included. The reseal suspension row is exactly G1 and G8, "because those are the two gates PB §5.5 names" (GR §5.6.1; MF §4.8.6).

### F. G1's two carve-outs and the G1/G8 allocation (RF §8.5 clause 2 — the algorithm)

**R32 (MUST).** Evaluate clause 0 first: `end.status` is `complete`. Otherwise **G1 fails**; no pair counts as passed and none counts as collected. Clauses 1–3 are still evaluated and reported, **but the G8 allocation of clause 2 is not made** — a partial run's clause-2 entries are G1's alone and no `G8:<path>` entry accompanies them (RF §8.5 clause 0, §8.5 per-clause tokens).

**R33 (MUST).** Clause 2 allocation, in order:
1. Let `b` be a `base` record for which no `result` record has `r.runner == b.runner`, `r.id == b.id` and `r.out == "passed"`.
2. Classify: **went away** (no `result` record with that pair) or **did not pass** (a `result` record exists with `r.out ≠ "passed"`).
3. **Carve-out first:** where `b.out` is `"xfail"` **or** `"skipped"` **and** the shape is *did not pass*, `b` yields **no finding at all** — not G1's, not G8's — no wire, nothing for a review to cover, and `G1=pass` with `G8=pass` on `b`'s account (RF §8.5 clause 2; PB §6.3 G1 limb (ii) and the matching G8 clause).
4. Otherwise both shapes are a **G8** finding `G8:<b.path>`, discharged where a verifying `class=protected` review's `wires=` contains that token.
5. **G1 fails on `b` as well**, save — for the *went-away* shape only — where that same review names `G8:<b.path>`.

**R34 (MUST).** Three boundaries on the carve-out, each of which decides a different `wires` array (GR §5.6.1; RF §8.5):
* decided on `b.out` **alone**, never on the `T` outcome — `xfail`, `failed`, `error`, `skipped`, `xpass`, `unknown` on `T` all leave it carved out;
* it does **not** reach the *went-away* shape (a vanished `xfail`/`skipped` id is a harness change and stays G8's, review and all);
* it does **not** reach a frozen `Spine-Test` entry (clause 1), only a `B`-floor id.

**R35 (MUST).** "**The carve-out is unconditional on every landing shape, reseal included**"; the three *boundary* cases read `override` on a reseal (GR §5.6.1).

**R36 (MUST).** An id-loss a `class=protected` `G8:<path>` review names is G8's finding, never G1's: the landing records **`G1=pass`, `G8=override`** (GR §5.6.1, quoting PB §6.3 G1). The `G8:<path>` review exemption reaches the **went-away** shape only; an id that collected on `T` and did not pass is a finding of **both** gates, and such a landing reads `G8=override` with G1's finding uncovered — not landable until a `class=break-glass` review bypasses G1 (GR §5.6.1).

**R37 (MUST).** "The same id" means the same `(runner, id)` pair; a landed id goes away when the runner that collected it stops running (dropped from `params.langs`, removed from `.spine/ci.sh`, deselected) and that is the same G8 clause with the same remedy (GR §5.6.1).

**R38 (MUST).** G1's per-id token is `G1:` + `tok(path)`, the pair's `result` record `path` where the file carries one, its `base` record `path` where it does not (RF §8.5). The **bare `G1`** is a closed list of five: `result-missing`; `result-malformed`; clause 0's own finding (`end.status ≠ complete`); clause 3's uncovered AC; clause 1 where `P(R, F)` is empty (RF §8.5).

**R39 (MUST NOT).** `G1:` with nothing after the colon is **never written**; an empty `path` is no path and takes the bare `G1` (RF §8.5, §13 R19).

**R40 (MUST).** One entry per path, never per pair: two failing ids from one file, and a parametrized function's several failing ids, collapse to a single `G1:<path>` entry (RF §8.5; GR §6.1).

**R41 (MUST).** Clause 3 (AC coverage) matches on `(runner, fn)`, not on `id`: some `result` record has `r.runner == R` and `r.fn == F`. "**Outcome is irrelevant; collection is the test**." Matching on `id` fails every parametrized AC test; matching on a bare `fn` across runners lets one runner's collection satisfy an AC verified in another (RF §8.5 clause 3).

### G. Auto-merge — PB §7.4 rule 5 as an algorithm

**R42 (MUST).** `C-M4: merge.auto = on` is a **request**, not a capability. Whether a run may act on it is computed per run, from these five preconditions, "each read from trunk or produced by this run, never asserted by the branch asking to merge" (PB §7.4 rule 5):

| id | Precondition | Recomputable / Attested |
|---|---|---|
| 0 | `C-A3: threat.candidate` is `trusted`. Under `hostile` — the default — `C-M4` can **never** evaluate `on`, whatever the rest say | R |
| 1 | trunk's manifest declares `params.isolation` as `container` — never `none` — **and** the ingested header's `profile=` equals it | R (both sides recorded) |
| 2 | **three conjuncts:** the header's `keys_visible=` is `false`; **and** the collector's `tool=` is the base's pin; **and** the run established that the ingested file came from a job whose definition was taken from trunk | A |
| 3 | reconstruction was proved before this push — structurally `"met"` in v1, there being no deferred mode | R |
| 4 | this run performs step 6's compare-and-swap itself and the object it pushes is the object that becomes trunk's tip | A |

**R43 (MUST).** `effective` = `requested` AND every precondition `"met"` or `"exempt"`. Any precondition missing ⇒ the run evaluates `C-M4` as `off` whatever the constitution says, and raises a **`G11`** wire naming which precondition failed, with `reason=` mandatory and required to name it (PB §7.4 rule 5; GR §5.8).

**R44 (MUST).** `"exempt"` is used **only** where the design grants exemption, and the single grant is the **tombstone**: all five `"exempt"`, `profile: "n/a"`. A tombstone under `C-M4: on` therefore records `effective: true`, and that is not a bug (GR §5.8).

**R45 (MUST).** "**A reseal is exempt from nothing.**" It records all five as computed; under the shipped `C-A3: hostile`, precondition 0 is `"unmet"` (GR §5.8; PB §7.4 rule 5; RF §8.6).

**R46 (MUST NOT).** Do not widen the precondition status domain. `"unverifiable"`, or a split of `"unmet"` by cause, is **rejected**: it would put a new token inside a digest-bearing member and force a `report_version` bump for a distinction no gate reads (GR §5.8).

**R47 (MUST).** `keys_visible=true` is a **legal** header value producing a legal report: it does not refuse ingestion; `preconditions[2].status` is `"unmet"`, `evidence.keys_visible` is `true`, and the rule-5 wire is raised (GR §5.8, §9.22; RF §8.4).

**R48 (MUST).** The class of the rule-5 review is **`class=tripwire`**, "**Deliberately not `class=protected`**": `C-A3: hostile` names the *coding agent* as the adversary, while reviewer ≠ signer is a control against a malicious human insider. In solo mode that human is the author (PB §7.4 rule 5).

### H. Result-file ingestion order — what G1 and G15 read (RF §8.3, normative order)

**R49 (MUST).** Perform in this order, stopping at the first failure:
1. `tree=` or `base=` ≠ the `(T, B)` this run fixed → **`base-moved`**. The run ends and re-queues on the new tip; `C-M3` bounds re-verifications **within the run**.
2. `tool=` ≠ trunk's `cli.version` + `cli.dist_hash` → **G15 failure, never a retry**. The expected token is constructed as `<cli.version>` `+sha256:` `<cli.dist_hash>` and compared **as bytes over the whole token**; no parse of the collector's token is required or performed.
3. A `runner` value the repository does not declare → **`result-malformed`**.

**R50 (MUST NOT).** There is no step 4, and provenance is not one. Trunk-defined origin evidence decides no label, produces no retry and yields no finding; it is evaluated once, at RF §8.4, as the third conjunct of precondition 2 (RF §8.3).

**R51 (MUST).** A result file whose trunk-defined origin cannot be demonstrated **is ingested**: `evidence` present with all five members from the header, `profile` = **the header's own value**, `preconditions[2].status` = `"unmet"`, `effective` = `false`, `gates[]` carrying whatever the gates found, **`G1` is `"pass"` on a green suite**, and the rule-5 `G11` wire present with `reason=` naming precondition 2 (GR §5.9; RF §8.1).

**R52 (MUST).** When no file was ingested (`result-missing` / `result-malformed`): `evidence` **absent**; `profile` is **`"none"`** (never `"n/a"`, which is the tombstone's alone); `preconditions[1]` and `[2]` are `"unmet"`; `gates[].G1` is `"fail"`, or `"override"` where a `class=break-glass` review names it, and that review's `wires=` carries a **bare `G1`** (GR §5.9; RF §8.4).

**R53 (MUST).** On collector deadline expiry (`params.timeout`, default 1800s, per runner invocation): the collector kills the process group and writes `status=runner-timeout`; that file **is** ingested, `evidence` **is** present, and `status ≠ complete` means no id counts as passed, so every frozen id is a G1 finding and `gates[].G1` reads `fail` (or `override`) (GR §5.9; RF §7.3).

### I. Quick-lane terminality and break-glass reach

**R54 (MUST).** Break-glass is available **only from `tests-approved` onward** — never before an approval exists — and the transition table admits it from `tests-approved`, `landing-review`, `protected-review` and `base-moved†` only (PB §7.6; RF §8.7).

**R55 (MUST).** **In the quick lane a `result-missing`, `result-malformed` or G1 finding is terminal**: the exits are a conforming run, or promotion to the gated lane via `spine new --from <branch>` (RF §8.7).

**R56 (MUST).** A **reseal** is the one quick-lane shape this does not reach: it "is never escalated and never refused by a wire", and every G1/G8 finding there — `result-missing` and `result-malformed` included — is admitted by its own `class=protected` review naming the token, sealed `=override` and counted as a freeze override (PB §5.5, §7.6; RF §8.7; GR §5.6.1).

**R57 (MUST).** A review whose wire set carries a **pathless** wire never survives a base move; a per-id `G1:<path>` review survives a base move that touches neither the floor nor those paths (RF §8.7; PB §5.4 table).

### J. The protected floor (PB §7.3) — G14's subject

**R58 (MUST).** The floor "can never auto-merge, whatever any intent declares, and always take[s] a protected review (G14)". The list, verbatim in items:
* `.spine/**` — manifest, keyring, `ci.sh`, and the optional `restore.sh` the collector reads from trunk;
* the constitution and every agent-context file the manifest lists — "anything loaded into an agent session is instruction surface";
* CI definitions: `.github/workflows/**`, `.github/actions/**`, `.gitlab-ci.yml`, `.circleci/**`, `.buildkite/**`, `Jenkinsfile*`, and the remainder, "enumerated closed in `docs/spec/manifest.md` and shipped inside the pinned release";
* `CODEOWNERS`, wherever it lives;
* files that make git execute or fetch code: `.gitattributes`, `.gitmodules`, `.githooks/**`, `.husky/**`, `.pre-commit-config.yaml`;
* "any diff entry that adds or changes a **symlink** (mode `120000`) or a **submodule pointer** (mode `160000`) — the two ways to reach a protected path without naming it".

**R59 (MUST).** Agent-context, hook and attribute names match at **any depth** and **case-insensitively**; paths are casefolded before comparison, "and a diff entry whose casefolded path equals an existing path's is itself a floor hit: two spellings of one file are a collision, not a new file" (PB §7.3).

**R60 (MUST).** "The floor ships *inside* the pinned spine release, so a repository cannot shrink it; `C-A2` can only extend it, and every `paths.*` entry in the manifest is a floor entry and is monotone the same way — a landing whose tree drops an entry present at the base fails G14 outright, review or no review" (PB §7.3).

**R61 (MUST).** Matching runs over the **full `merge-base..head` diff including renames and deletions** — "renaming `ci.yml` to `ci.yml.bak` is a touch" (PB §7.3).

**R62 (MUST).** `params.trunk` is a rendering hint; the trusted stage protects the branch it is configured for out-of-band (PB §7.3).

**R63 (MUST).** **Declared touchpoints are never consulted by G14.** "an intent that declares `.github/workflows/` as expected has declared nothing" (PB §5.2, §7.3, §6.3 G14).

**R64 (SHOULD).** Where the host offers CODEOWNERS or branch protection, `spine init` emits matching entries as a supplement; "the guarantee does not depend on them" (PB §7.3).

**R65 (MUST).** G14's diff set `D` is `git -c core.quotePath=false diff --raw -z --no-renames <mb> <Hc>`, `mb = git merge-base B Hc`; `D` is the set of triples `(src_mode, dst_mode, path)` with `path` the raw bytes. For a tombstone `D := ∅` (MF §5.3, §5.1).

**R66 (MUST).** `F := F0 ∪ effective(C-A2) at B ∪ E(M_B)`; `F0` and `C-A2` are **patterns** matched by `pmatch`, `E(M_B)` are **literal paths** matched by `lmatch` — "a union of predicates, not a merged dialect" (MF §5.4, §5.6).

**R67 (MUST).** `F` is built from `B` alone: a candidate that adds a `C-A2` entry or a `paths` key protects its own new paths from the *next* landing, never this one (MF §5.4; PB §7.4 rule 1).

### K. Trusted execution — the six rules, 0 through 5 (PB §7.4)

The section opens: "Gate results are worth exactly what produced them. **Six rules**" — numbered 0..5, so "the five trusted-execution rules" of common speech is rules 1–5 plus rule 0.

**R68 (Rule 0 — MUST).** "**The trusted stage's own definition is policy.**" The trusted job runs from trunk's workflow file, never the candidate's — **and so does the untrusted job** (GitHub: `pull_request_target`, or a `workflow_run` dispatcher on trunk, with `permissions: contents: read` and no secrets; GitLab: an MR pipeline whose config is `include:`d with `ref: <trunk>`; `--ci generic`: a definition outside the repository).

**R69 (Rule 0 — MUST).** A result file from a job that cannot demonstrate a trunk-defined origin **is still ingested** — "it simply fails auto-merge precondition 2, so that landing takes a human review and never auto-merges". The strict reading ("never ingestible at all") is **withdrawn**.

**R70 (Rule 0 — MUST NOT / REFUSE).** `merge_group` is refused as a trigger for **both** jobs: "never `merge_group`, which executes the merge group's own workflow file on a `gh-readonly-queue/*` ref that fails a trunk-only deployment rule; **never `merge_group` for the untrusted job either**". A provider that cannot arrange a trunk-defined untrusted job plus a trusted job performing the CAS "is configuration (b) of §5.4 whatever it is called, and (b) cannot auto-merge".

**R71 (Rule 0 — MUST).** The untrusted job is the only job that runs on `intent/*`, `quick/*` and `spine/upgrade-*` pushes, runs with `permissions: contents: read`, and receives no other secret. `.spine/ci.sh` is executed from `git show origin/<trunk>:.spine/ci.sh`, **never from the checkout**. The probe is the untrusted job itself: it fails the run if the pipeline-key variable (`SPINE_PIPELINE_KEY`) is visible to it, and the collector writes that assertion and its own `tool=` into the result-file header.

**R72 (Rule 0 — MUST).** "Every such test is per run and remembers nothing between runs: a repo whose results do not come from trunk's collector never auto-merges, and never latches itself open by having once produced a good header."

**R73 (Rule 1 — MUST).** "**Policy is read from trunk.**" The trusted stage **and the collector** read `.spine/manifest.json`, `.spine/allowed_signers` and the constitution's scaffolded rules from `origin/<trunk>`, never from the checkout under test. "A candidate may change policy; that change is a floor hit reviewed under the *old* policy, and governs only later landings."

**R74 (Rule 2 — MUST / REFUSE).** "**Gates run from a pinned, hash-verified release.**" The manifest's `cli.version` + `cli.dist_hash` pin the binary; the trusted stage installs that exact release, verifies the hash, and **refuses to run anything else — including a spine built from the repository**. `.spine/ci.sh`, read from trunk, installs and hash-verifies the collector the same way before anything else runs in the untrusted job; a mismatch fails the run, and no result file exists to ingest.

**R75 (Rule 3 — MUST).** "**The graph is rebuilt from git objects, every run.**" `spine index --fresh` is implied by `spine check --ci`; no SQLite file is fetched, cached or trusted from anywhere, and the trusted stage restores no cache at all. It executes no repository code. The untrusted stage computes `T := git merge-tree --write-tree origin/<trunk> H` itself and tests a **detached checkout of `T` — never `H`** — under the collector.

**R76 (Rule 3 — MUST).** The collector is `spine check --ci --collect`: the pinned release, invoked by trunk's `.spine/ci.sh`, **holding no key and signing nothing**. It collects the id set on a checkout of `B` *before* `T` exists — "so no candidate can make a landed test uncollectable" — then spawns the runner as a child, reads its machine-readable stream over a pipe, and, after reaping the whole process group, writes the result file itself.

**R77 (Rule 3 — MUST).** Isolation profiles: `profile=container` (runner inside a container the collector created; result directory outside it and unmounted; stream crosses on a pipe the collector holds), `profile=uid` (**reserved, and refused in v1**), `profile=none` (one uid, one process tree, no boundary at all). "The collector measures what it actually got and names it in the header; trunk's `params.isolation` says what the repo claims to provide, and the trusted stage requires the two to agree."

**R78 (Rule 3 — REFUSE).** "**`uid` ships no mechanism in v1 and is a refusal**, not a downgrade: a manifest declaring it fails the job with no result file." Every *other* way of not getting the boundary — a platform without namespaces, a creation that failed, a test that failed — records `none`, runs the suite unisolated, and is priced by precondition 1. "The collector never substitutes one mechanism for another and never upgrades."

**R79 (Rule 3 — MUST).** `container` may be written only after a probe boundary passed **four** tests it ran and could have failed: containment of the result directory; an identity the *host* confirms; separation from the collector's own process and root; and **no egress** (loopback-only interface, its outbound connection attempt having to fail).

**R80 (Rule 3 — MUST).** **Dependency restore is a phase of its own**, run once per checkout before the first runner invocation against it and outside every one of them, from trunk's `.spine/restore.sh` and never from the checkout.

**R81 (Rule 3 — MUST).** The trusted stage ingests a result file only if its `tree=` equals the `T` it computed — a mismatch is `base-moved`; a header whose collector `tool=` ≠ the base's pin is a **G15 failure, never a retry**. A `base-moved` exit ends the run; the snippet re-queues the whole two-job run on the new `T`, and the gate report records how many re-verifications this run performed.

**R82 (Rule 3 — MUST).** The trusted job checks out with full history plus an explicit fetch of `refs/heads/intent/*`, "or the lease registry is empty in CI".

**R83 (Rule 4 — MUST).** "**Every landing is attested.**" `spine check --land` produces a canonical-JSON gate report — intent blob, base, head, tree, tool version and hash, git version, the ids of the policy files it read, mode, per-gate results, floor hits, the verified sign-off, approval and review lines, `self_approved` — and seals its SHA-256 into the envelope (`report=`).

**R84 (Rule 4 — MUST).** "**The trusted stage publishes the full report to `refs/notes/spine`, and that is not optional.**" It changes nothing about authority: notes are not fetched by default, no gate reads one, and **a note is never a source**.

**R85 (Rule 5).** Auto-merge preconditions — see §G above (R42–R48).

**R86 (MUST).** Two CI jobs, one command: the untrusted job builds and tests; the trusted job runs `spine check --ci [--land [<id>]]`. "`--ci` is a mode (self-hash check first, skew hard-fails, **Authority never warns**), `--land` the terminal stage" (PB §7.4 closing).

**R87 (MUST).** On plain git the guarantee is **detection**: "G9 derives every trunk commit without a valid seal as an orphan or `unattested`, reports it on every run, counts it forever, and refuses to land on top of it until a human reseals." Two supplements are non-optional: non-fast-forward pushes denied on trunk *and* on `refs/heads/intent/*`, with deletion of intent branches restricted to the pipeline principal (PB §7.4 closing, PB §11 *Git requirements*).

**R88 (Stated residual — MUST record, MUST NOT claim otherwise).** "**G1's `passed` is therefore exactly as strong as the isolation between a candidate's code and its own runner, and nothing in this design establishes that property.**" G6 is not the answer — it runs in the same untrusted stage through the same collector. "**Spine-kit's auto-merge therefore guarantees provenance and blast radius, never correctness**" (PB §7.4).

### L. Tripwire list — the green-pipeline conditions (PB §5.2)

**R89 (MUST).** A green pipeline lands **only when all of the following hold**, evaluated on the synthetic merge of the branch onto trunk's *current* tip, never on the branch alone:
1. the integrated diff stays inside declared "expected to change" touchpoints;
2. nothing in "must NOT change" was touched;
3. *(see Contradiction C1: PB §5.2 still lists "No changes to schema, auth, or public API surface"; PB §6.3 withdraws that wire)*;
4. no new dependencies introduced;
5. diff size under `C-Q2`. "Spine-owned and floor paths are exempt from this wire, the dependency wire and quick-lane containment — they are renders of a pinned release, verified by blob";
6. every AC ID has a matching test; every frozen test id, and every test trunk already had, is reported *passed* on this synthetic merge; lint, types and coverage gates pass;
7. nothing in the diff intersects another in-flight intent's **forbidden** set or **frozen** paths (the hard lease);
8. **no protected-floor hit**; a floor hit routes to a *protected* review — reviewer ≠ intent signer where the team has two;
9. `C-M4: merge.auto = on` **and** every precondition of §7.4 rule 5 holds — "Either missing → a `class=tripwire` wire to `landing-review`: `G11` (`C-M4`) where the constitution says off, and `G11` naming the precondition where the run computed it off — one gate, two reasons, distinguished by `reason=`. Both are read and signed by a human with a mandatory reason; neither requires a second one, because neither is a floor change."

**R90 (MUST).** "**Auto-merge is not a button.** It is the compare-and-swap of §5.4: every check above runs on the synthetic merge tree, and the result lands only if trunk's tip has not moved since. A green branch is not a fact about trunk." (PB §5.2.)

**R91 (v1 behaviour).** In v1, touchpoint checks are **path-prefix matching**; graph containment is a post-v1 upgrade (PB §5.2). *(Resolved by ID §6.3: the matcher is segment-boundary, never byte-prefix — see Contradiction C4.)*

---

## Byte-level fixities

**F1 — the wire order (the source, PB §11 `Spine-Review` row, verbatim).**
> `wires=<G<n>[:path],…>` — "ascending by unsigned byte value over the whole token, so `G11` precedes `G2`; a set with no order is a signature two runs spell differently"

Consequences fixed by GR §6.1: `G1` precedes `G11`; within one gate the pathless entry precedes every `:`-suffixed one, its token being a proper prefix of theirs. **The sort key is the token's bytes**, i.e. `tok(path)` and not `esc(path)`.

**F2 — `gates[]` is the *other* order.** "`gates[]` sorts by gate number ascending — an array rather than an object because gate order is numeric and JCS would sort `g1, g10, g11, …, g2` by name. **`wires[]` does not** … The two orders differ deliberately and an implementation that applies one to the other produces a different `report=` over identical findings." (GR §5.6.)

**F3 — a **numeric** sort of `wires=` is non-conforming.** `G2:src/shared/util.ts,G11` is wrong; `G11,G2:src/shared/util.ts` is right (GR §6.2, §9.19).

**F4 — `tok(s)`.** `esc(s)` with three bytes moved out of the printable row into the `\xHH` row: `,` (`0x2C`) → `\x2c`, `U+0020` (`0x20`) → `\x20`, `"` (`0x22`) → `\x22`. Every other byte follows GR §2.3 unchanged. "**`tok` is one pass over the bytes of `s`**, not `esc` composed with a second escaping step: a second pass would re-escape the `\` that the first pass emitted and turn `,` into `\\x2c`." `=` is deliberately **not** escaped — "Three escapes, not four" (GR §6.2).

**F5 — `Spine-Gates` rendering.** "`Spine-Gates` is a rendering of this array, in the same order, as `G<n>=<status>`, space-separated." Example payload shape: `G1=pass … G16=pass` (GR §5.6.1; PB §11).

**F6 — `Spine-Gates` value domain.** "every gate that ran, never G10 (it runs after the seal); a gate that ran and passed its own check reads `=pass` even when it raised a wire — the rule-5 `G11` precondition wire is not a finding about G11 — while `=override` marks a gate whose own *finding* a signed review accepted, or which break-glass bypassed; a tombstone lists the four that ran" (PB §11, verbatim).

**F7 — the wire aggregation paragraph, verbatim key sentence.** "It is never spelled `G1`: a `G1` wire is a finding that named tests did not pass, and the two must never share a token a reviewer signs over." (PB §11.)

**F8 — G14's casefold `cf`.** ASCII only, over raw path bytes, length-preserving, total:
```
cf(s)[i] = s[i] + 0x20   if 0x41 ≤ s[i] ≤ 0x5A
         = s[i]          otherwise
```
No Unicode table, no locale, no normalization (MF §5.2). `cf` is applied to both sides of every comparison and **to no stored value**.

**F9 — G14's shipped floor `F0`, the closed list of seventeen patterns** (MF §5.5), in ID §6.2's dialect:
`.spine/` · `.github/workflows/` · `.github/actions/` · `.circleci/` · `.buildkite/` · `Jenkinsfile*` · `**/.gitlab-ci.yml` · `**/AGENTS.md` · `**/CLAUDE.md` · `**/.claude/` · `**/.cursor/` · `**/CODEOWNERS` · `**/.gitattributes` · `**/.gitmodules` · `**/.githooks/` · `**/.husky/` · `**/.pre-commit-config.yaml`.
Depth rules: a floor entry named by a **file or directory name** matches at any depth (`**/`); one named by a **provider directory prefix** stays root-anchored (`.github/`, `.circleci/`, `.buildkite/`, `Jenkinsfile*`). Symlinks and submodules are not in the list because they are not paths — that is the mode clause.

**F10 — G14's matchers.**
```
pmatch(P, p) := match( cf(P), cf(p) )        P from F0 or C-A2   -- ID §6.3's match
lmatch(v, p) := cf(p) = cf(v)  ∨  cf(p) begins with cf(v) ++ "/"  v from E(M_B)
```
(MF §5.6.)

**F11 — G14's mode clause.** `modehit(src_mode, dst_mode) := src_mode ∈ {120000, 160000} ∨ dst_mode ∈ {120000, 160000}` — **both sides**, so a symlink deleted or replaced by a regular file is a hit too (MF §5.8).

**F12 — G14's collision clause.** `collides(d) := ∃ x ∈ paths(T) : x ≠ d ∧ cf(x) = cf(d)`, where `paths(T)` is `git ls-tree -r -z --name-only <T>`, raw bytes; "an existing path" is resolved to **`T`** (MF §5.7). The hit is recorded against the **diff entry**, never the file it collided with (GR §5.7).

**F13 — G14's verdict expression** (MF §5.10):
```
hits := { d : (sm, dm, d) ∈ D ∧ ( modehit(sm, dm)
                                ∨ ∃P ∈ F0 ∪ effective(C-A2) : pmatch(P, d)
                                ∨ ∃v ∈ E(M_B)               : lmatch(v, d)
                                ∨ collides(d) ) }
```
`floor_hits := esc(d)` for every `d ∈ hits`, deduplicated, **sorted ascending by the `esc`-encoded bytes**. Wires: one `{gate: "G14", path: esc(d), class: "protected", kind: "finding"}` per hit and **no other `G14` entry**; token is `G14:` + `tok(d)`.

**F14 — G2's diff-size measurement, verbatim.** "**Diff size** is `git diff --numstat --no-renames` over `merge-base..Hc`, additions plus deletions summed, binaries refused rather than counted, floor and spine-owned paths exempt — a count two implementations compute differently is a wire that fires on one and not the other." (PB §6.3 G2.)

**F15 — object ids and digests in any gate output.** Object ids are lowercase hex at the full length `object_format` implies (40 or 64), never abbreviated, never uppercase, never prefixed. Non-git digests are `"sha256:"` + 64 lowercase hex (GR §7 rules 9–10; PB §11 hash policy).

**F16 — no wall clock in any gate result.** "No member holds a time, a duration, a date or anything derived from one. G3's staleness comparison is made against the committer date of `objects.base`, not against 'now'" (GR §7 rule 1, §9.8). The staleness window is the constant **1 209 600 seconds** of the pinned release (GR §10 OPEN-3).

**F17 — arrays.** "Every array whose semantics is 'the set of X' is emitted even when empty; `[]` is a value, not an absence." `null` never appears (GR §7 rules 5–6).

**F18 — G10's canonical dump comparison.** "nodes sorted by kind,id, edges by from,to,kind, `src` included; provisional (in-flight) elements, † states (dumped as `tests-approved`), volatile test results and worktree-only files excluded; the diff must be empty" (PB §6.3 G10).

**F19 — G13's wire path member.** For `G13`, `path` carries the offending event commit's oid, "lowercase hex at the length `object_format` implies, for which `esc` — and `tok` — is the identity"; token is `G13:<oid>` (GR §6.1, §6.3; MF §4.8.1).

---

## Error cases

### Gate-level statuses and refusals

| Condition | Behaviour | Status token / exit code | Owner |
|---|---|---|---|
| Any gate reads `fail` in a report a run would seal | run refuses; nothing is sealed | **`report-not-landable`** | GR §5.6.1; MF §4.8.6, §6.10 |
| A landing whose `Spine-Gates` copies a `fail` | malformed; G9 indexes it | **`unattested`** | GR §5.6.1 |
| G10 dump diff non-empty | **refuses the push**, ends the run, no retry; `L` never becomes a git object | **`reconstruction-failed`** | PB §6.3 G10, PB §11 *States* |
| Result-file `tree=`/`base=` ≠ the run's `(T, B)` | run ends and re-queues on the new tip | **`base-moved`** | RF §8.3 step 1; PB §6.3 G11 |
| Re-verifications inside one run exceed `C-M3` | the run ends | **`starved`** | PB §6.3 G11, PB §11 *States* |
| Collector `tool=` ≠ base's pin | **G15 failure, never a retry**, never overridable | G15 `fail` | RF §8.3 step 2; PB §11 |
| Running binary not listed in trunk's pinned `dist_hash` artifact list | refuse locally, fail in `--ci` | G15 `fail` | PB §6.3 G15 |
| Trunk tip is not a valid landing | blocks `--land` until resealed; the one admitted form is `--land --reseal` with `head=` that tip | (G9 refusal) | PB §6.3 G9, §5.5 |
| A landing whose ledger walk fails | indexed, reported and counted | **`unattested`** | PB §6.3 G9 |
| A trunk commit pushed around the pipeline | entered outside the transition table | **`orphan`** | PB §5.5, PB §11 |
| No result file at the fixed path | G1 finding (a state is **not** entered) | **`result-missing`** (bare `G1` wire) | RF §8.2, §8.7 |
| File found and §4 grammar or §8.3 step 3 rejects it | G1 finding | **`result-malformed`** (bare `G1` wire) | RF §8.2, §8.7 |
| `runner` token not declared by `params.langs` | ingestion rejects the file | **`result-malformed`** | RF §8.3 step 3 |
| `end.status ≠ complete` (e.g. deadline) | file **is** ingested; no pair counts as passed or collected; clause 2's G8 allocation is not made | collector writes **`status=runner-timeout`**; bare `G1` wire | RF §7.3, §8.5 clause 0 |
| Manifest declares `params.isolation = "uid"` | the collect job fails and writes **no** result file; G16 refuses the manifest outright at the landing that installs it | G16 **`isolation-unsupported`** (outright) | PB §7.4 rule 3, §11 CLI; MF §6.2 check 12b |
| Trunk-defined origin cannot be demonstrated | file is **ingested**; sole consequence is precondition 2 | `preconditions[2].status: "unmet"`, `G11` advisory wire | RF §8.1, §8.4; GR §5.9, §9.25 |
| `keys_visible=true` in the header | legal; does not refuse ingestion | `preconditions[2].status: "unmet"` | GR §5.8, §9.22 |
| Quick-lane landing with `result-missing`/`result-malformed`/any G1 finding | **terminal** — no override of any class is reachable | exits: conforming run, or `spine new --from <branch>` (**`escalated`**) | RF §8.7 |
| Break-glass attempted before `tests-approved` | unavailable | (refusal) | PB §7.6 |
| Break-glass naming G5, G9, G10, G11 or any Authority gate | not permitted | (refusal) | PB §7.6, PB §11 |

### G13's status tokens (MF §4.8.4) — every one **outright** except check 2's non-role case

| # | Check | Kind | Status token(s) |
|---|---|---|---|
| 1 | `K` present and passes §4.4's lint at §4.5's key count | outright (halts) | the `keyring-*` tokens (incl. `keyring-duplicate-principal`, `keyring-seal-mixed`) |
| 2 | every event commit in `E` carries a signed line verifying under the namespace its trailer requires | **outright** if the trailer is `Spine-Signoff`, `Spine-Approve`, `Spine-Review`, `Spine-Upgrade`, `Spine-Withdraw`; **coverable** otherwise | `statement-unverified`, `statement-namespace` |
| 3 | no two commits in `E` carry byte-identical signed lines | outright | `event-line-duplicate` |
| 4 | `A.approve` is the newest verifying approve, later than the last reopen, `intent=` matches, `freeze=` unvoided | outright | `approval-voided` |
| 5 | every `Spine-Reopen` carries `voids=` naming the approval binding immediately before it; `voids=none` exactly when none preceded | outright | `reopen-voids-mismatch` |
| 6 | `reason=` present whenever `red=` reads `0/n` or `held=false` | outright | `approve-reason-missing` |
| 7 | team mode: no `class=protected`/`class=break-glass` review has `self_approved: true` | outright | `self-approved-protected` |
| 8 | `A.withdraw` verifies under `spine-signoff@v1` by the sign-off's fingerprint, or `spine-review@v1` by a different one | outright | `withdraw-key` |
| 9 | signerless overlay cardinality | outright | `signerless-review-count` |
| 10 | chain rule when `diff(B, Hc)` touches `.spine/allowed_signers` | outright | `chain-review-not-in-parent`, `chain-remover-removed`, `chain-seal-not-in-parent` |
| 11 | *(in-flight only)* `total_rounds=` arithmetic | outright | `total-rounds-mismatch` |
| 12 | *(in-flight only, at `--approve`)* no redundant later approval | outright | `approval-redundant` |
| 13 | *(in-flight only, at `--approve`)* `reason=` present when the closure tripwire fired | outright | `approve-reason-missing` |

Namespace table G13 enforces (MF §4.8.3): `Spine-Signoff`, `Spine-Reopen`, `Spine-Upgrade` → `spine-signoff@v1`; `Spine-Withdraw` → `spine-signoff@v1` **or** `spine-review@v1` (check 8 decides by key); `Spine-Approve` **with** `run=` → `spine-seal@v1` and only that; `Spine-Approve` **without** `run=` → `spine-review@v1` and only that; `Spine-Review` any class → `spine-review@v1`; `Spine-Seal` on `mode=solo`/`team` → `spine-seal@v1`; `Spine-Seal` on `mode=recovery` → `spine-review@v1`.

**Voiding is a transition, not a finding**: a statement whose fingerprint is absent from `K` is **void** in flight, absent from `authority`, and check 2 skips it (MF §4.8.2, §4.8.4 note).

### G14's outright statuses (MF §5.9)

| Condition | Behaviour | Status token |
|---|---|---|
| `E(M_B) ⊄ E(M_T)` (manifest `paths.*` shrank), except under `Spine-Upgrade: to=none` | outright `fail`, review or no review | `paths-shrank` |
| `C-A2` pattern set at `T` ⊉ at `B` (by bytes) | outright `fail` | `c-a2-shrank` |
| A `C-A2` pattern with an ASCII uppercase letter inside a bracket expression | outright `fail` (for `F0` this is a release-build assertion) | `c-a2-bracket-case` |

### G16's statuses (MF §6.2, ordered; checks 1–8 halt on first failure)

| # | Kind | Status token(s) |
|---|---|---|
| 1 | outright | `manifest-missing` / `manifest-not-removed` |
| 2 | outright | §3.11's malformed list |
| 3 | outright | `manifest-noncanonical` |
| 4 | outright | `frozen-member-missing`, `frozen-member-type`, `owner-unknown`, `manifest-unknown-member-value` |
| 5 | outright | `member-name-out-of-grammar`, `reserved-member-name` |
| 6 | outright | §3.11's list |
| 7 | outright | `manifest-noncanonical`, `files-duplicate-path`, `files-base-misplaced`, `template-version-mismatch` |
| 8 | outright | `object-format-mismatch`, `blob-malformed` |
| 9 | **coverable** | `scaffold-blob-mismatch`, `scaffold-path-missing`, `region-markers-missing`, `region-markers-malformed`, `region-version-mismatch` |
| 10 | outright | `manifest-changed-without-upgrade`, `upgrade-without-manifest-change`, `upgrade-manifest-mismatch`, `upgrade-version-mismatch`, `forced-disagrees` |
| 11 | outright | `resign-floor-above-current` |
| 11b | **coverable** (skipped under `from=none`) | `resign-lowered` |
| 12 | **coverable** (skipped under `from=none`) | `langs-shrank` |
| 12b | outright | `isolation-unsupported` |
| 13 | **coverable** | the `keyring-*` tokens (over `K_T`) |
| 14 | **coverable** | `staging-residue` |
| 15 | **coverable** | `constitution-*` |
| 16 | outright | `uninstall-*` |
| 17 | outright | `reinit-*` |

### `spine check --verify` exit codes (GR §4.3) — order normative

| Exit | Status | Meaning |
|---|---|---|
| 0 | `verified` | Recomputed report's digest equals the seal's `report=` |
| 1 | `report-mismatch` | The candidate was the sealed report and the recomputation disagrees |
| 1 | `candidate-mismatch` | The candidate's own bytes do not hash to the seal's `report=` |
| 2 | `report-unavailable` | No candidate report: no `--report`, and no note in this clone |
| 3 | `wrong-release` / `wrong-git` / `report-version-unknown` | Preconditions for recomputation not met |
| 4 | `not-recomputable` | `objects.head` unreachable and the evaluation needed it — a `land` under squash strategy |

Order: (1) seal's `tool=` and `git=` → exit 3, before any candidate is read; (2) resolve a candidate → exit 2; (3) sha256 over the candidate's exact bytes → exit 1 `candidate-mismatch`; (4) parse → exit 3 `report-version-unknown`; (5) recomputability → exit 4; (6) rebuild and compare → exit 0 or exit 1 `report-mismatch`.

---

## Worked examples / test vectors

**V1 — the canonical wire line (four independent occurrences; every literal `wires=` in the corpus).**
> `wires=G11,G2:src/shared/util.ts`

Found in PB §5.5's canonical envelope, EV §8.3, GR §8.1 and GR §8.2 (GR §5.6, §6.2). It is the byte-order sort, not the numeric one; a numeric implementation writes `G2:src/shared/util.ts,G11` and fails containment against a conforming report over identical facts (GR §6.2).

**V2 — the flagship landing.** GR §8: `INT-042`, team mode, merge strategy, `C-A3: hostile`, `C-M4: on`, `profile=container`, one reopen, one `class=tripwire` review by `bob` over a `G2` containment finding, "with the universal rule-5 `G11` advisory wire present because precondition 0 fails under `hostile`" (GR §8).

**V3 — published digests bearing on gate output** (`docs/spec/README.md` digest table):

| Where | Published value |
|---|---|
| GR §8.1 evaluation 1 | **3476 bytes, `sha256:e2bd8cb5da473701bf0dd8e9aa0c6a28cc1280e33ba06758165c5089ae3f5b47`** |
| GR §8.2 evaluation 2 | **4053 bytes, `sha256:a47c13281b0b4a13085e523cd3c09e4f7d1caf0c80419de5fa3a869f7aec309e`** (its inner `report=` carries §8.1's digest, so §8.1 must be recomputed first — the one ordering dependency in the set) |
| GR §8.3 minimal canonicalizer vector | **`sha256:a594772c…`** — "Untouched since publication. Build against it first." |
| MF §8.3 manifest blob | **1762/1763 bytes, `cb4cd49034bbe25f76573c40d6711b2c33f9136f`** |
| CI §5.3 `.spine/ci.sh` | **319 lines, `git hash-object` `131f13fb0312162579605999d3f9f4e90098c74c`, `sha256:d6bcf50c…`** |
| DM §12.3 dump vector (what G10 diffs) | **62 lines, 14054 bytes, `sha256:3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da`** |

**V4 — the byte-count invariant that localises a wire-order bug.** "re-sorting `wires` is a permutation, so **both byte counts are unchanged and only the digests moved** — an implementation that matches every published length and neither digest has a numeric wire comparator, not a broken canonicalizer" (`docs/spec/README.md` known-gaps, closed entry 2; GR §9.19, EV §14 D3).

**V5 — the G14 worked runs.** MF §8.4 computes a G14 run (including the case-collision case `src/billing/Tax.py` beside `src/billing/tax.py` — "a floor hit and nothing else in the design would notice"); MF §8.5 a G16 run over the same landing; MF §8.6 a rollback restoration.

**V6 — the drift gate as literal SQL** (PB §6.3, verbatim):
```sql
-- G2: files modified by the landing that fall outside declared touchpoints
-- (paths frozen by this intent's binding approval are excluded: they are G8's)
SELECT m.to_id FROM edges m
JOIN edges i ON i.from_id = m.from_id AND i.kind = 'implements'
             AND json_extract(i.attrs,'$.role') = 'landing'
WHERE m.kind = 'modifies' AND i.to_id = 'myrepo/INT-042'
  AND NOT EXISTS (SELECT 1 FROM edges d
    WHERE d.from_id = 'myrepo/INT-042' AND d.kind = 'declares'
    AND json_extract(d.attrs,'$.polarity') = 'expected'
    AND spine_match(d.to_id, m.to_id));
-- any row → tripwire fires
```
With the note that `spine_match` "is the touchpoint matcher, not equality … Its semantics — segment-boundary, never byte-prefix, so `src/bill` does not match `src/billing/x.ts` — are fixed in `docs/spec/intent-doc.md`."

---

## Cross-references it depends on

| What this sheet does not own | Owner |
|---|---|
| The result file's grammar, header, outcome vocabulary, ingestion order, G1's clause-by-clause evaluation and token allocation | `result-file.md` (sheet: result file / collector interface) |
| The canonical gate report — canonicalization, schema, `esc`, `tok`, exit codes, publication to `refs/notes/spine` | `gate-report.md` (sheet: gate report) |
| G13, G14, G16 algorithms; the manifest and keyring schemas; `F0`; `cf` | `manifest.md` (sheet: manifest / keyring / Authority gates) |
| Constitution rule grammar, `effective(C-A2)`, `C-T1`/`C-T2` renders per language, defaults table | `constitution.md` |
| Touchpoint matching (`spine_match`), the glob dialect `match`, the intent template parse | `intent-doc.md` |
| Per-language resolvers, the freeze closure G8 recomputes, package-manifest paths for the new-dependency wire, the `@verifies` pragma grammar and the **source-symbol → runner-id join** G1's coverage clause and G5's orphan clause both assume (`import-resolver.md` §12.1–§12.3) | `import-resolver.md` |
| The envelope grammar, trailer line bytes, `envelope=` byte range, `-Sig` payloads | `envelope-vectors.md` |
| `spine index --dump` — the format G10 diffs | `dump.md` |
| The three CI definitions, `.spine/ci.sh`, the per-provider trunk-defined-origin test (CI §14 R11 for GitHub, §10.3 scoring) | `ci.md` |
| Templates and their versions (G4's `resign` map input) | `templates.md` |

---

## OPEN items

Undecided owner questions bearing on gates. **No value is invented here.**

1. **RF OPEN-5 — G6's reporting channel.** "Mutation results are per-mutant, not per-id, and this spec deliberately does not overload `result` records to carry them. Four languages make the question larger rather than smaller: a mutation channel would owe a per-language mutant identity as well as a per-language test identity." (RF §14 OPEN-5.) *Blocks any G6 implementation; G6 is roadmap 5 and must produce no `gates`/`wires` entry in v1.*
2. **`params.ci` monotonicity — filed three times and decided nowhere** (`ci.md` OPEN-3, RF OPEN-7, MF OPEN-1). Moving `github → gitlab` "moves precondition 2 from reachable to permanently unmet for that repository … nothing makes the guarantee it retires legible in the diff, in a wire, or in the seal." Options: leave it to the protected review, or make **G16** fail such a landing with a wire naming the lost row (MF §13 OPEN-1 recommends (b)). *Would add a G16 status token.* (`docs/spec/README.md` known gaps; RF §14 OPEN-7; MF §13 OPEN-1.)
3. **CN OPEN-9 / MF §4.8.5 — `C-A1`'s declared value versus the key count.** Today a mismatch is "a warning on every report" and **not a wire**. CN §15 D15 recommends option (c): the *maximum* governs and a mismatch becomes a **G13 finding**. If adopted, exactly three things move: MF §4.5's `mode`, a new outright check with status **`mode-declaration-mismatch`**, and GR §5.6.1's G13 row. *Owner's.* (MF §4.8.5.)
4. **MF OPEN-3 — does `C-A2` keep bracket expressions at all?** §5.6 refuses only an uppercase letter inside one; the wider option refuses brackets in `C-A2` outright. Recommendation: keep the narrow refusal. *Changes G14's `c-a2-bracket-case` reach.* (MF §13 OPEN-3.)
5. **MF OPEN-4 — should an unknown `templates` key be a G16 finding?** Options: (a) silent, as now; (b) a `G16` warn that never blocks; (c) a coverable finding. Recommendation (a). *Note (b) would require Authority to raise a `warn` kind, which GR §6.1/§6.3 currently forbid.* (MF §13 OPEN-4.)
6. **MF OPEN-2 — canonical form for `.spine/allowed_signers`.** Option (b) would have **G16 warn (never fail)** on a non-canonical line — again a `warn` kind for an Authority gate. *Owner's.* (MF §13 OPEN-2.)
7. **Whether G12 gains a landing-time clause** — "Whether G12 gains a landing-time clause is PB §6's to settle; this row records the v1 behaviour and nothing more" (GR §6.3 G12 row).
8. **G3's staleness window as a constitution rule — decided *no*** (GR §10 OPEN-3), recorded here because the constant `1 209 600` seconds has no lever: "A team that wants a different window still has no lever, and that is the answer rather than the cost of it." *Not open; listed so an implementer does not add `C-F1`.*

---

## Contradictions found

**C1 — PB §5.2 still lists a wire PB §6.3 withdrew.** PB §5.2 bullet 3 requires "No changes to schema, auth, or public API surface." PB §6.3's G2 row says: "The **schema/auth/public-API** wire of earlier versions is **withdrawn**: no document ever gave it a predicate, three declined it as another's, and a wire nobody can compute is a wire that never fires — better deleted than left as a promise." GR §11 confirms the withdrawal. **Resolution:** the wire does not exist; PB §5.2's bullet is stale prose. GR §6.3's G2 row states it raises nothing. *File against PB §5.2.*

**C2 — PB §6.3 says the diff-size sub-check is recorded as `G2:<path>`; GR §6.3 says it takes the bare `G2`.** PB §6.3 G2: "the diff-size and new-dependency wires of §5.2 are G2 sub-checks, recorded as `G2:<path>`." GR §6.3 G2: "**Bare `G2`** for the diff-size sub-check … **a repository-wide count that names no path**, so it takes the bare id under PB §11's 'gates without a path use the bare id'". **Resolution: GR wins via PB §11** — §11's *gates without a path use the bare id* is the governing sentence, and GR adds the retention argument ("a pathless wire never survives a base move"). An implementation writing `G2:<path>` for the diff-size count produces a different `wires` array, `report=` and `envelope=`. *File against PB §6.3's G2 row.*

**C3 — the signerless review count: "at least two" (PB §11) versus "two" (MF §4.8.4 check 9).** PB §11: "carries **at least two distinct `class=protected` reviews in team mode** … **a floor and never an exact count**, since a third reviewer signing a contentious reseal is diligence and must not be the thing that refuses the landing". MF §4.8.4 check 9: "`A.reviews` holds **two** `class=protected` reviews with distinct fingerprints in team mode, and **one** in solo mode", status `signerless-review-count`, outright. Read as equality, MF refuses the three-reviewer reseal PB §11 protects. **Resolution: PB §11 wins** — implement check 9 as `≥ 2` in team mode and `≥ 1` in solo mode. *File against MF §4.8.4 check 9.*

**C4 — PB §5.2's "path-prefix matching" versus ID's segment-boundary matcher.** PB §5.2: "In v1, touchpoint checks are path-prefix matching." PB §6.3's own SQL note: "`spine_match` is the touchpoint matcher, not equality … segment-boundary, **never byte-prefix**, so `src/bill` does not match `src/billing/x.ts` — [its semantics] are fixed in `docs/spec/intent-doc.md`." **Resolution:** ID §6.3's segment-boundary matcher is normative; a byte-prefix implementation matches `src/billing/x.ts` against a touchpoint `src/bill` and produces a smaller wire set than a conforming one. *File against PB §5.2.*

**C5 — MF §5.9 claims GR §5.6.1 has no outright category; GR §5.6.1 has one.** MF §5.9: "§11 C2 asks GR §5.6.1 to admit the category, which its current `override` rule does not." GR §5.6.1 as of the current corpus defines *outright* at length, tabulates the v1 outright set, and names G14's three tokens in it. **Resolution:** the request is discharged; MF §5.9's sentence is stale citation surface (exactly the round-4 defect class `docs/spec/README.md` line 39 describes). Same shape for MF §6.1's "PB assigns G16 no class anywhere, which GR §6 records as a gap" — GR §6.3 now assigns `protected` and cites MF §11 C1 as the request it answered.

**C6 — `C-Q2` is `quick.max_lines` yet PB §5.2 applies the diff-size tripwire under it in every lane.** PB §2.1: `C-Q2: quick.max_lines = 400`, `enforced_by: spine:G2`. PB §5.2 bullet 5: "Diff size under `C-Q2` (400 changed lines is a sane start)" — stated as a condition of *every* green pipeline. PB §6.3's G2 row scopes the `C-Q2` line count to the **quick lane** ("quick lane: ⊆ `C-Q1` ∪ floor ∪ spine-owned paths, and under `C-Q2` lines") while stating the diff-size sub-check generally. **Unresolved by the corpus:** whether the gated lane's diff-size sub-check has any threshold, and if so which. GR §6.3 fixes only the *measurement* and the *token*, not the bound. *File against PB §5.2/§6.3; an implementer must not invent a gated-lane bound.*

**C7 — PB §7.3's floor list ends in an ellipsis that MF closes differently in two places.** PB §7.3 gives `.spine/**` and `.gitlab-ci.yml` (root-anchored in prose, inside the CI bullet) and ends the depth clause with "…". MF §5.5 renders entry 1 as `.spine/` (dialect: trailing `/` = contents-only) and entry 7 as `**/.gitlab-ci.yml` (any depth), and files PB's ellipsis as a defect (MF §10 D5). **Resolution:** MF §5.5's seventeen-pattern closed list is the release constant; PB §7.3's prose is not implementable as written. Note the two deliberate deviations from a literal reading of PB: `.gitlab-ci.yml` gains `**/`, and `Jenkinsfile*` stays root-anchored and is "the weakest entry in the list".
