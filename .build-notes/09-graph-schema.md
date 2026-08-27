# Requirement sheet 09 — The traceability graph: node kinds, edge kinds, derivation, the iron rule

Scope: PB §6 (the derived layer) and the parts of `dump.md` that enumerate node and edge kinds. This sheet
owns **the graph schema, the derivation table, the iron rule, the store/cache rules, the dump projection's
node/edge enumeration and ordering, and session resume**. It does *not* own gate semantics (G1…G16), the
envelope grammar, the intent-doc parse, the keyring parse, the result file, or the gate report — those are
other sheets' (§ "Cross-references it depends on").

Citation convention used throughout, matching the corpus's own: **PB** = `PLAYBOOK.md` v0.19; **DM** =
`docs/spec/dump.md`; **ID** = `docs/spec/intent-doc.md`; **IR** = `docs/spec/import-resolver.md`;
**CN** = `docs/spec/constitution.md`; **MF** = `docs/spec/manifest.md`; **RF** = `docs/spec/result-file.md`;
**GR** = `docs/spec/gate-report.md`; **CI** = `docs/spec/ci.md`; **README** = `docs/spec/README.md`.

Resolution rule in force (README, "Where prose here and the playbook's §11 disagree, §11 still wins — report
it as a defect in one of them"): PB §11 wins over a spec; elsewhere the spec is normative and resolves PB.

---

## Sources read

| File | Lines | Section(s) |
|---|---|---|
| `/Users/thettwe/Works/spine-kit/PLAYBOOK.md` | 497–563 | §6 preamble: two graphs and one table; the workflow transition table (62 rows read in full) |
| " | 561–572 | §6.1 The iron rule: derived, never authored; the provenance law |
| " | 569–643 | §6.2 Traceability graph schema (SQL block, storage paragraph, derivation table, two defended positions) |
| " | 640–686 | §6.3 Gates as queries (read in full for G5/G9/G10/G15 graph clauses and the `spine_match` note) |
| " | 686–690 | §6.4 Session resume via graph query |
| " | 690–698 | §6.5 The dependability suite (`spine stats`, `spine review`, `spine eval`) |
| " | 698–708 | §6.6 Post-landing lifecycle (reverted · superseded · withdrawn) |
| " | 708–781 | §6.7 install lifecycle — read for the cache-deletion step, the manifest `schema` field and the skew rule |
| " | 848–860 | §7.4 rule 3 (the graph is rebuilt from git objects every run) |
| " | 882–897 | §7.5 chain rule (signer `valid_from`/`valid_to`) |
| " | 983–1039 | §11 Vocabulary: hash policy, trailers, Files and refs, States, Gates, CLI |
| `/Users/thettwe/Works/spine-kit/docs/spec/dump.md` | 1–914 | **entire document**, read in full |
| `/Users/thettwe/Works/spine-kit/docs/spec/intent-doc.md` | 604–620 | §6.6 the `code_unit` node and the `declares` edge (+ §13 D4/D6, lines 1011–1013, 1067–1071) |
| `/Users/thettwe/Works/spine-kit/docs/spec/import-resolver.md` | 1190–1250 | §12.1–§12.3 the pragma, the file-granular join, the naming sugar |
| `/Users/thettwe/Works/spine-kit/docs/spec/constitution.md` | 795–826 | §9.5–§9.6 the `constitution` node and the `protects` edges |
| `/Users/thettwe/Works/spine-kit/docs/spec/manifest.md` | 401–420, 123 | §4.5–§4.6 what the keyring owes the `signer` node; the manifest `schema` field |
| `/Users/thettwe/Works/spine-kit/docs/spec/result-file.md` | 42 | the result file's graph effect (`test.result_at` only) |
| `/Users/thettwe/Works/spine-kit/docs/spec/README.md` | 1–88 | index, published digests, six settled owner decisions |

---

## Data model

### 0. The three "graphs", and which one this sheet is

PB §6 preamble ships **two graphs and one table**:

| Graph | Authored by | Role |
|---|---|---|
| Traceability graph | Nobody — derived from IDs and git objects by `spine index` | Drift gates, coverage gates, archaeology, session resume |
| Code graph | Nobody — derived from AST/dependencies (tree-sitter) | Touchpoint proposal in the interview; graph-containment tripwires; scoped context for Agents A and B; the quick-lane router |

**R1 · MUST.** The workflow is a **transition table, not an engine**: "Spine-kit encodes the pipeline as
declarative rows (state × event → next state, plus the guard that enforces each), checked by the same code
that runs the gates" (PB §6). "Anything not in the table cannot happen" (PB §6).

**R2 · REFUSE.** "**User-defined custom workflow DAGs are refused**: a user-authored workflow is an authored
graph, and the iron rule below applies to workflows too." (PB §6). No status token is given; treat as a
refusal at parse/config time.

**R3 · MUST NOT.** The two graphs are not merged: "there are no function-level nodes: that is the code
graph's job; the two graphs join on `code_unit` paths rather than merging into one mega-graph" (PB §6.2).

**R4 · MUST.** † states (`checked†`, `base-moved†`) are runner-local: "they exist only while a gate record is
live, and collapse to `tests-approved` in any fresh clone" (PB §6); PB §11: "† = runner-local, collapses to
`tests-approved` in any clone."

### 1. The iron rule and the provenance law (PB §6.1)

**R5 · MUST.** "Every graph in spine-kit is a **cache**: gitignored, deleted at will, deterministically
rebuilt from the repo by one command." (PB §6.1)

**R6 · MUST NOT.** No user may create or maintain a graph: "The moment a user has to create or maintain a
graph, you have rebuilt SDD bureaucracy in graph clothing, with a worse editor." (PB §6.1)

**R7 · MUST.** The provenance law: "every node and edge must cite its source. An edge that cannot say where
it came from does not exist." (PB §6.1) — mechanised as the `src` column, `TEXT NOT NULL` on both tables
(PB §6.2).

**R8 · MUST.** The provenance grammar is fixed and closed (PB §6.1, verbatim):

```
<path>:<line> · git:<sha> · git:<sha>:msg:L<n> (a line inside a commit message) ·
git:<sha>:trailer:<Name> · git:<sha>:patch-id · git:<sha>:<path>:<line> (a line of a file at a commit) ·
spine:<version>:floor (the protected floor shipped in the release)
```

**R9 · MUST.** A *rendering* of the graph (PR body, report note, exported knowledge bundle) "must transport
each node's and each edge's `src` verbatim in the grammar above" (PB §6.1).

**R10 · MUST NOT.** "**A rendering that spine reads back is a graph**, and the law then applies to it in full
— which is why nothing in spine reads one." (PB §6.1) DM §1: "**Nothing in spine ever reads a dump.**"

**R11 · MUST.** A rendering other tools read "must be datable against the ledger it came from — and any such
rendering that ships is counted in §10" (PB §6.1). This is why the dump header carries `head` (DM §3.1).

**R12 · MUST.** The envelope (PB §5.5), the approval record (PB §4.3) and the install manifest (PB §6.7) are
**sources, not graphs**: "written by a tool, never by hand, and each the analogue of a commit to a diff. The
graph stays fully derived from them." (PB §6.1)

### 2. The store (PB §6.2)

Verbatim schema (PB §6.2):

```sql
PRAGMA user_version = 7;   -- graph schema; ≠ the binary's constant → delete and rebuild (§6.7)
CREATE TABLE meta (key TEXT PRIMARY KEY, val TEXT);
  -- 'cli_dist_hash' | 'manifest_blob' | 'built_at_trunk'
  -- a cache built by another binary, another manifest, or an older trunk tip is never queried — it is rebuilt

CREATE TABLE nodes (
  id   TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  attrs JSON,
  src  TEXT NOT NULL
);

CREATE TABLE edges (
  from_id TEXT NOT NULL,
  to_id   TEXT NOT NULL,
  kind    TEXT NOT NULL,
  attrs   JSON,
  src     TEXT NOT NULL
);
```

| Field | Table | Type | Domain | Required | Default |
|---|---|---|---|---|---|
| `user_version` | pragma | integer | `7` in v1 | yes | — (a mismatch ⇒ delete and rebuild) |
| `meta.key` | meta | TEXT PRIMARY KEY | `cli_dist_hash` \| `manifest_blob` \| `built_at_trunk` | yes | — |
| `meta.val` | meta | TEXT | opaque | yes | — |
| `nodes.id` | nodes | TEXT PRIMARY KEY | §"Node ids" below | yes | — |
| `nodes.kind` | nodes | TEXT NOT NULL | 9 values, closed | yes | — |
| `nodes.attrs` | nodes | JSON | per kind, below | in the dump: always present, `{}` when empty | `{}` |
| `nodes.src` | nodes | TEXT NOT NULL | provenance grammar (R8) | yes | — |
| `edges.from_id` | edges | TEXT NOT NULL | a node id | yes | — |
| `edges.to_id` | edges | TEXT NOT NULL | a node id | yes | — |
| `edges.kind` | edges | TEXT NOT NULL | 15 values, closed | yes | — |
| `edges.attrs` | edges | JSON | per kind, below | in the dump: always present, `{}` when empty | `{}` |
| `edges.src` | edges | TEXT NOT NULL | provenance grammar (R8) | yes | — |

**R13 · MUST.** "Storage is SQLite in a single gitignored file (`.spine/cache/graph.sqlite`)." (PB §6.2)
PB §11 *Files and refs* enumerates `.spine/cache/` as gitignored, holding "`graph.sqlite`, `staging/`,
`report.json` … and `results/<T>.jsonl`".

**R14 · MUST NOT.** "One canonical store; any rendering of it is a cache beside it, never a second store
(§6.1). No graph database in v1" (PB §6.2).

**R15 · MUST.** A cache row is **never queried** when its provenance differs: "a cache built by another
binary, another manifest, or an older trunk tip is never queried — it is rebuilt" (PB §6.2 `meta` comment).
PB §6.2 `PRAGMA` comment: schema version "≠ the binary's constant → delete and rebuild (§6.7)".

**R16 · SHOULD (performance, not a byte rule).** "Indexing is incremental: the cache is keyed by trunk tip,
and only commits above the last verified landing are re-walked." (PB §6.2) PB §9 roadmap 2 restates:
`spine index` rebuilds "from scratch whenever the cache's schema or builder hash differs".

**R17 · MUST.** On upgrade, "**The graph cache is deleted.** Schema migration is *nothing*: `spine index`
rebuilds under the new schema." (PB §6.7 step 6). PB §8 failure table: "The cache carries `PRAGMA
user_version`; migration is delete-and-rebuild."

**R18 · MUST.** `--dump` implies `--fresh` (DM §4.3, DM §10 rule 3). PB §7.4 rule 3: "**The graph is rebuilt
from git objects, every run.** `spine index --fresh` is implied by `spine check --ci`; no SQLite file is
fetched, cached or trusted from anywhere, and the trusted stage restores no cache at all." CI §7.4/T5:
"Restores **no cache**, and rebuilds the graph `--fresh`. Implied by `spine check --ci`."

**R19 · MUST.** The manifest's `schema` field records "the graph schema version the writing release used
(`PRAGMA user_version`, DM §3.3). `7` in v1. Read by nothing at landing time; the cache is deleted and
rebuilt on upgrade (PB §6.7 step 6)." (MF §3, line 123)

**R20 · MUST.** IDs are repo-scoped from day one: "`myrepo/INT-042`, not bare `INT-042` — the prefix comes
from the manifest's `repo`, while trailers carry the bare id so a fork or rename does not invalidate
history" (PB §6.2). DM §13.3 makes this total over **every** kind; PB §6.2's unprefixed examples are read as
abbreviations (and are filed as DM §14 D5).

### 3. Node kinds — the closed set of nine

DM §5.1 (closed, alphabetical as the dump spells them): `ac`, `adr`, `approval`, `changeset`, `code_unit`,
`constitution`, `intent`, `signer`, `test`. PB §6.2's list is the same nine: "intent | ac | test | code_unit
| changeset | approval | signer | adr | constitution".

**R21 · MUST.** Node ids are `<repo>` + `/` + a per-kind local id (DM §5.2):

| kind | local id | example (DM §5.2) |
|---|---|---|
| `intent` | the bare trailer id: `INT-<n>` or `BUG-<n>` | `myrepo/INT-042` |
| `ac` | `<intent local id>/AC-<n>` | `myrepo/INT-042/AC-1` |
| `test` | `test:` + `<runner>` + `:` + `<runner-native function id>` | `myrepo/test:pytest:tests/billing/test_invoice.py::test_AC1_totals` |
| `code_unit` | `code:` + `esc(path bytes)`; a trailing `/` means a directory | `myrepo/code:src/billing/tax.py` |
| `changeset` | `cs:` + the commit's **full** oid | `myrepo/cs:1b2c…6789` |
| `approval` | `approval:` + 64 lowercase hex (DM §5.2.1) | `myrepo/approval:c428…5597` |
| `signer` | `signer:` + `esc(principal bytes)` | `myrepo/signer:alice@example.com` |
| `adr` | the ADR's own id, as its heading spells it | `myrepo/ADR-007` |
| `constitution` | `constitution:v<n>` | `myrepo/constitution:v3` |

**R22 · REFUSE.** "**`<repo>` matches `^[A-Za-z0-9._-]+$`.** A manifest whose `repo` does not is refused
(`id-out-of-grammar`)." (DM §5.2) MF §3.1 now owns it "unchanged, adding a 1…64-byte length bound and the
refusal code `repo-out-of-grammar`".

**R23 · MUST.** "**A `test` id is qualified by its runner, and the pair is the identity** (PB §4.3, settled
2026-08-26). … two runners collecting the same function id are two nodes, and merging them would let one
runner's rename silently satisfy another's coverage. `<runner>` contains no `:`" (DM §5.2). The four v1
runner tokens are `pytest`, `vitest`, `dart-test`, `swift-test` (`gradle` reserved, emitted by nothing —
README "Known gaps"; IR §11.1).

**R24 · MUST.** "**Oids are full, lowercase, never abbreviated.** PB's `cs:abc123f` is display." (DM §5.2)

**R25 · MUST.** The `approval` local id is "`approval:` + the SHA-256, as 64 lowercase hex digits, of **the
exact bytes of the signed trailer line as the commit message carries it** — from the first byte of the
trailer name (`Spine-Approve`, `Spine-Signoff`, `Spine-Review`, `Spine-Reopen`, `Spine-Withdraw`,
`Spine-Upgrade`) through the last byte before its terminating LF, with no LF included." (DM §5.2.1)

**R26 · MUST NOT.** The `approval` id is **not** the approve line's `freeze=` digest — that reading "is total
over only one of the six `event` values" and is rejected; `freeze=` is carried as the `freeze` attr
(DM §5.2.1, DM §13.6).

**R27 · MUST.** A `code_unit` id is built from "the repo-relative, `/`-separated path exactly as git stores
it in the tree entry" (DM §2.4). ID §6.6: `node id := <repo> "/" "code:" esc(pattern bytes)` — one node per
**distinct declared pattern**, "written as declared, never expanded" (ID §13, D4 resolution).

**R28 · MUST.** A `constitution` node exists once per distinct version observed on the first-parent walk;
"Id `<repo>/constitution:v<n>`, kind `constitution`, `attrs {}`, `src` `git:<sha>:<esc(path)>:2` — line 2,
the header (§3.1). `<sha>` is the landing that introduced the version." (CN §9.6)

**R29 · REFUSE (upstream).** Two keys under one principal are refused by G16's keyring lint, precisely
because "A `signer` node's id is `signer:` + `esc(principal)`, so two keys under `alice@example.com` are two
signer nodes with one id … an unrepresentable graph" (MF §4.5, §9 R7, `keyring-duplicate-principal`).

### 4. Node attrs — every field, type, domain, presence

PB §6.2 gives attrs for five node kinds only. **DM §7.2 (normative): "A kind PB §6.2 does not give attrs for
has none in the dump": `ac`, `adr`, `code_unit` and `constitution` nodes carry `{}`.**

#### 4.1 `intent` (PB §6.2 · DM §7.2)

| attr | type | domain | presence | notes |
|---|---|---|---|---|
| `status` | string | `merged` \| `withdrawn` \| `reverted` \| `superseded` | always (in a dump) | **derived, never read from the file** (PB §6.2 derivation table) |
| `owner` | string, bytes | free bytes, `esc`-encoded | iff the doc has an `Owner:` field | "a hint, never authority (PB §3.1)" |
| `title` | string, bytes | free bytes | always | the `# INT-042: <title>` heading's title, **read from the sealed intent inside the landing commit's message — never from that commit's subject line** (DM §7.2) |
| `template` | string | `<variant>@<n>`, e.g. `intent@2`, `intent-change@2`, `intent-bug@2` | always | the `Template:` value — variant *and* version (PB §3.4; README settled decision 4) |
| `blob` | oid | 40/64 lowercase hex | always | the signed intent blob |
| `signer` | string, bytes | principal | iff a `Spine-Signoff` is copied into the landing | |
| `reopen_count` | integer | ≥ 0 | always | copied `Spine-Reopen` lines |
| `late_reopen_count` | integer | ≥ 0 | always | of those, the ones after the binding approval |
| `landing` | oid | | always | `L` |
| `base` | oid | | always | the seal's `base=` |

**R30 · MUST.** `intent.status` is drawn from `merged`, `withdrawn`, `reverted`, `superseded` **and nothing
else** in a dump (DM §7.3). "In-flight statuses (`draft` … `checked†`) cannot appear at all … an
implementation that emits one is non-conforming."

**R31 · MUST.** `orphan`, `unattested` and `resealed` are **changeset** facts, not intent statuses: "a
landing can be `unattested` while its intent is plainly `merged`" (DM §7.3, §13.8).

#### 4.2 `changeset` (PB §6.2 · DM §7.2)

| attr | type | domain | presence |
|---|---|---|---|
| `landing` | boolean | `true` for a sealed trunk commit, `false` for a member | always |
| `lane` | string | `gated` \| `quick` (PB §11 `Spine-Lane`) | iff `landing` |
| `event` | string | `land` \| `withdraw` \| `reseal` (PB §6.2 derivation row; PB §11 `Spine-Event` for landings) | iff `landing` |
| `strategy` | string | `merge` \| `squash` (PB §11 `Spine-Strategy`) | iff `landing` |
| `base` | oid | the seal's `base=` | iff `landing` |
| `head` | oid | the seal's `head=` (the content head `Hc`) | iff `landing` |
| `tree` | oid **or sentinel** | `L`'s tree oid, or `unverifiable(squash)`, or `unverifiable(git-version)` (DM §7.2.1) | iff `landing` |
| `seal_principal` | string, bytes | the seal's `signer=` | iff `landing` |
| `seal_verified` | boolean | | iff `landing` |
| `report_sha256` | string | `sha256:<64 hex>` | iff `landing` |
| `threat` | string | `hostile` \| `trusted` (PB §11 seal) | iff `landing` |
| `profile` | string | `container` \| `uid` \| `none` \| `n/a` (PB §11 seal) | iff `landing` |
| `tool_version` | string | the release version, e.g. `1.4.0` (DM §12.2 vector) | iff `landing` |
| `git_version` | string | `<major.minor>`, e.g. `2.45` — **the seal's `git=`, never the indexing binary's own** (DM §7.2.1) | iff `landing` |
| `mode` | string | `solo` \| `team` \| `recovery` (PB §11 seal) | iff `landing` |
| `unattested` | boolean | | iff `landing` |
| `resealed` | boolean | | iff `landing` |

**R32 · MUST.** "A member changeset carries `{"landing":false}` and nothing else: it has no seal, and every
one of those fields is a seal field." (DM §7.2)

**R33 · MUST.** `changeset.tree` may legally be either sentinel. `unverifiable(git-version)` "is the single
place a dump is not a pure function of §4.1's inputs" and is recorded rather than suppressed (DM §7.2.1,
§13.7). The local git version MUST NOT be put in the header to repair it (DM §7.2.1).

#### 4.3 `approval` (PB §6.2 · DM §7.2)

| attr | type | domain | presence |
|---|---|---|---|
| `event` | string | `signoff` \| `approve` \| `review` \| `reopen` \| `withdraw` \| `upgrade` | always |
| `role` | string | `signer` \| `reviewer` \| `pipeline` — **the namespace the signature verified under**, never a claim in the trailer | always |
| `principal` | string, bytes | the `signer=` / `reviewer=` value | always |
| `verified` | boolean | the `-Sig` verified against the keyring at the seal's `base=` | always |
| `blob` | oid | | iff the line carries `blob=` or `intent=` |
| `base`, `head`, `tree` | oid | | iff the line carries them |
| `class` | string | `tripwire` \| `protected` \| `break-glass` | iff `event` is `review` |
| `rounds`, `total_rounds`, `reopens` | integer | ≥ 0 | iff the line carries them |
| `red` | string | `"k/n"` | iff `event` is `approve` |
| `freeze` | string | `sha256:<hex>` | iff `event` is `approve` |
| `wires` | array of strings | wire tokens **in the line's order** | iff `event` is `review` |
| `voided_by` | oid | the commit carrying the voiding `Spine-Reopen` | iff a copied `Spine-Reopen` voids this approval |
| `void_reason` | string, bytes | that line's `reason=` | iff `voided_by` is present |

**R34 · MUST.** `role` is the verifying namespace, not a claim: "A v1 approve line signed under
`spine-review@v1` is `reviewer`." (DM §7.2)

**R35 · MUST NOT.** `approval.wires` is **not re-sorted**: "the signed line's order is the fact, and a dump
that re-sorted it would hide a non-conforming review rather than reproduce it" (DM §7.2). The line's own
required order is PB §11's — "ascending by unsigned byte value over the whole token, so `G11` precedes `G2`".

#### 4.4 `signer` (PB §6.2 · DM §7.2 · MF §4.6)

| attr | type | domain | presence |
|---|---|---|---|
| `roles` | array of strings | subset of `spine-review@v1`, `spine-seal@v1`, `spine-signoff@v1`, **ascending by bytes** | always |
| `fingerprint` | string | "as `ssh-keygen -lf` produces it (`SHA256:<base64>`)" | always |
| `valid_from` | oid | the trunk commit at which the key first appears in `.spine/allowed_signers` | always |
| `valid_to` | oid | the trunk commit at which it stopped appearing | iff the key has been removed |

**R36 · MUST.** "**Both are commits, not times** — the chain is the clock." (MF §4.6; PB §7.5: "the chain,
not timestamps, is the authority"). "A line edited in place (same principal, new key) is a removal and an
addition: the old fingerprint gets a `valid_to`, the new one a `valid_from`." (MF §4.6)

#### 4.5 `test` (PB §6.2 · DM §7.2, §8.4)

| attr | type | domain | presence |
|---|---|---|---|
| `result_at` | object `{tree, base, passed}` | volatile | **store only** — excluded from every dump |

**R37 · MUST.** "`test`: `{result_at: {tree, base, passed}}` — volatile; excluded from G10" (PB §6.2). RF's
graph effect: the result file "Populates the volatile `test.result_at {tree, base, passed}` attrs only …
G10 excludes those attrs from the canonical dump, so a result file can never affect reconstruction." (RF §2)
Consequence: "every `test` node in a dump carries `{}`" (DM §8.4).

#### 4.6 `ac`, `adr`, `code_unit`, `constitution`

**R38 · MUST.** These four carry `{}` (DM §7.2, §13.9). "An implementation that wants to store an AC's text
may; the dump does not carry it, and G10 does not compare it." ID §5 restates: "PB §6.2 gives the `ac` kind
no attrs".

**R39 · MUST NOT.** "**Non-goals are not nodes.** They are prose constraints with no mechanically derivable
edges" (PB §6.2). ID: "The text is never read." (ID §5 on Non-goals)

**R40 · MUST NOT.** "An attr that names a git object carries the oid; a reference to another node is an edge,
never an attr. `intent.landing` is `L`'s oid, not `myrepo/cs:<L>`." (DM §7.1)

### 5. Edge kinds — the closed set of fifteen, with endpoints

PB §6.2: "has_ac | verified_by | declares | implements | modifies | built_under | approves | freezes |
signed_by | attested_by | reverts | supersedes | superseded_by | protects | exercises".
DM §5.3 lists the same fifteen alphabetically and closes the set.

**R41 · MUST.** Endpoint directions (DM §5.3, verbatim clause by clause):

| kind | from → to | attrs | source of the direction |
|---|---|---|---|
| `has_ac` | intent → ac | `{}` | DM §5.3 |
| `declares` | intent → code_unit | `{"polarity":"expected"\|"forbidden"}` | DM §5.3, ID §6.6 |
| `built_under` | intent → constitution | `{}` | DM §5.3 |
| `verified_by` | **test → ac** | `{"attributed":bool}` (+ store-only `introduced_by`) | DM §5.3/§13.4: G5 "fails on *a `verified_by` edge to a nonexistent AC*, so the AC is `to_id`" |
| `implements` | changeset → intent | `{"role":"landing"\|"member","provisional":bool,"verified":bool}` | DM §5.3 |
| `modifies` | changeset → code_unit | `{}` | DM §5.3 |
| `approves` | approval → intent, **or** approval → `cs:<L>` for a line carrying no id | `{}` | DM §5.3, PB §6.2 |
| `signed_by` | approval → signer | `{}` | DM §5.3 |
| `attested_by` | landing changeset → the seal's signer | `{}` | DM §5.3 |
| `freezes` | approval → code_unit (with `oid`) **or** approval → test (`{}`) | `{"oid":"<blob oid>"}` / `{}` | DM §5.3, PB §6.2 |
| `protects` | constitution → code_unit | `{"floor":bool}` | DM §5.3, CN §9.6 |
| `reverts` | the reverting landing → the reverted landing | `{"partial":bool}` | DM §5.3 |
| `supersedes` | **direction not fixed** — see "Contradictions found" | `{}` | PB §6.2 derivation row only |
| `superseded_by` | **direction not fixed** — see "Contradictions found" | `{}` | PB §6.6: "the indexer emits `superseded_by`" |
| `exercises` | code_unit ↔ test (unstated) | `{}` | PB §6.2 marks it "optional, v1.1" |

**R42 · MUST NOT.** "`exercises` is never emitted in v1" (DM §5.3) and stays excluded when it ships
(DM §8.3, §8.5 clause 3).

**R43 · MUST.** "**No dangling edges.** Every `from` and every `to` names a node record in the same dump."
(DM §5.3) PB §6.3 G5: "in a derived graph, **dangling edges are the linter**."

**R44 · MUST.** `implements.provisional` is `false` in every dumped record; `true` is a conformance failure
(DM §7.2, §8.3, conformance item 16).

**R45 · MUST.** `protects.floor` is `false` in every dumped record — the shipped-floor limb is excluded, so
only `C-A2` entries remain (DM §7.2, §8.3, §8.5 clause 2, conformance item 17).

**R46 · MUST.** A `freezes` edge to a `test` carries `{}`; one to a `code_unit` carries `{"oid": …}` — "PB
§6.2 says so" (DM §7.2).

**R47 · MUST.** Each `effective(C-A2)` entry yields exactly one `protects` edge, shaped (CN §9.6, verbatim):

```
from  <repo>/constitution:v<n>
to    <repo>/code: + esc(pattern bytes)          -- dump.md §5.2
kind  protects
attrs {"floor": false}                            -- dump.md §8.3
src   git:<sha>:<esc(path)>:<line of the C-A2 rule line>
```

"**Every pattern on the line shares the line's number**" (CN §9.6). "No other rule produces a node or an
edge. `C-Q1`, `C-T1` and `C-T2` are read by gates and are not `code_unit` nodes" (CN §9.6).

**R48 · MUST.** Each declared touchpoint pattern yields one `declares` edge, intent → code_unit, with
`{"polarity":"expected"}` or `{"polarity":"forbidden"}`; "the edge set is a set under `(from, to, kind)`"
and a pattern in both polarities is impossible (`polarity-conflict`, ID §5.4, §6.6).

---

## Algorithm

### A. Deriving the graph (`spine index`) — PB §6.2's derivation table is the indexer's spec

**R49 · MUST.** Derive each element from exactly the source PB §6.2's table names. Reproduced with its
mechanics:

1. **`intent`, `ac` nodes; `has_ac`, `declares` (polarity), `built_under`; intent `template`** — "in flight:
   `intents/<ID>.md` on `refs/heads/intent/*`; historical: the fenced intent bytes in the landing commit's
   envelope (§5.5), parsed by the `Template:` version's parser. **Never a PR description**." (PB §6.2)
   The parser is chosen by **variant and version** — `intent@2`, `intent-change@2`, `intent-bug@2` (DM §8.2;
   README settled decision 4). ID §1's table: the whole parse feeds these nodes and edges.
2. **`approval` nodes; `approves`, `signed_by`** — from `Spine-Signoff`, `Spine-Approve`, `Spine-Review`,
   `Spine-Reopen`, `Spine-Withdraw`, `Spine-Upgrade` lines with verifying `-Sig`, "on event commits while in
   flight, copied into the envelope once landed". `approves` "names the intent for every line carrying an id
   and the landing changeset `cs:<L>` for those that do not (`Spine-Upgrade`, and any review of a quick,
   reseal or lifecycle landing) — emitted only once the landing is indexed; in flight an id-less line's
   approval node carries no `approves` edge, there being no `L` yet". Verification is "against
   `.spine/allowed_signers` at trunk's tip (in flight) or at the seal's `base=` (landed)". (PB §6.2)
3. **`freezes`** — "`Spine-Frozen` (→ `code_unit`, with the blob) and `Spine-Test` (→ `test`) lines of the
   binding approval (§4.3)". (PB §6.2)
4. **`signer` nodes** — "`.spine/allowed_signers` at every trunk first-parent commit from the trust root,
   with `valid_from`/`valid_to` from the chain walk (§7.5)". (PB §6.2) The parse those attrs are functions
   of is MF §4.6's.
5. **`changeset` (landing + members); `implements`; `attested_by`** — "`M(L) = git rev-list B..L` for every
   trunk commit carrying `Spine-Seal`; the seal's fields become the landing changeset's attrs; in flight:
   `merge-base..branch`, provisional". (PB §6.2)
6. **`modifies`** — "`git diff --name-only B L` — the integrated delta G2 gates on; per-member diffs for
   archaeology". (PB §6.2)
7. **`test` nodes; `verified_by` (`attributed`)** — "pragmas `@verifies INT-042/AC-1` in a comment
   (canonical, and identical across languages) or a test name carrying `AC<n>` in its runner's conventional
   position (sugar, per-runner pattern in `docs/spec/`) **in blobs some approval froze**: in flight, the
   branch's test files — `attributed` iff the line is in a blob frozen by the binding approval, or (before
   approval) the file is on the intent's own branch and under `C-T1`; landed, parsed from `<L>:<path>` for
   every `Spine-Test` path of the landing — the frozen blob, reachable through `L`'s tree forever,
   provenance `git:<L>:<path>:<line>`, attributed by construction and never lost to later edits or
   deletions. `git blame` yields `introduced_by` for archaeology, never a gate input". (PB §6.2)
   The pragma grammar, the join and the sugar are IR §12.1–§12.3 (below, R57–R60).
8. **intent `status`** — "derived, never read from the file: the transition table, evaluated over event
   commits and landings". (PB §6.2)
9. **`reverts`; status `reverted`** — "a landing `R` later than `L` on first-parent, with a non-empty diff,
   whose `git diff R^ R -- <L's paths> | git patch-id --stable` equals `git diff L L^ | git patch-id
   --stable` — restricted to `L`'s paths, so the `BUG-` reproduction test `R` also lands does not disqualify
   it; missing hunks inside `L`'s paths → `{partial: true}` and a warning; only `Spine-Event: land` commits
   participate; `Spine-Reverts:` and git's "This reverts commit" line are hints". (PB §6.2)
10. **`supersedes` / `superseded_by`** — "ADR and constitution headers; the `Spine-Supersedes` trailer,
    copied from the intent's `Supersedes:` header". (PB §6.2)
11. **`protects`** — "the floor list inside the pinned release (`spine:<version>:floor`) + constitution
    `C-A2`". (PB §6.2) The shipped-floor limb never reaches a dump (R45).
12. **changeset `event`, `Spine-Upgrade` attrs** — "landing envelopes carrying `Spine-Event: land | withdraw
    | reseal` and, for toolkit lifecycle landings, the copied `Spine-Upgrade` line (§6.7)". (PB §6.2)
13. **`exercises` (optional, v1.1)** — "CI coverage reports". (PB §6.2) Never emitted in v1 (R42).

**R50 · REFUSE.** "`spine index` refuses `verified_by` edges to unsigned intents" (PB §6, transition table,
`signed → tests-drafted` row). No status token given.

**R51 · MUST.** An event commit whose signature fails or whose role disagrees with its namespace "is excluded
from state derivation and raised as a G13 wire naming the sha — a branch stays append-only, and a bogus
commit cannot brick it" (PB §6.2).

**R52 · MUST.** "The pragma is canonical; the naming convention is sugar. … A pragma counts only when a
runner collected an id from its file — an AC 'covered' by a pragma in a file no runner executes is not
covered." (PB §6.2)

**R53 · MUST.** `spine index` walks trunk first-parent from the tip to the root for the chain rule
(PB §7.5); the same walk derives `valid_from`/`valid_to` (MF §4.6) and evaluates the seal limb of the chain
rule (MF §4.8.4.1).

**R54 · MUST NOT.** `spine index` does not walk `refs/notes/*` (GR §2: "`spine index` does not walk
`refs/notes/*`"), and a note "is never a source" (PB §7.4 rule 4).

**R55 · MUST.** A `constitution` version reference resolves by string identity on the integer, with no
fallback: "the edge names `constitution:v<n>` whether or not such a node exists. If no landing on the
first-parent walk ever carried that version, the edge dangles and **G5 reports it**" (CN §9.5).

**R56 · MUST.** Provenance line numbers for a landed intent's `declares` edges are the **touchpoint label
line's**, not the individual pattern's, "since several patterns share one line" (ID §6.6). In flight the
production is `<path>:<line>` (`intents/INT-042.md:22`); landed it is `git:<L>:msg:L<n>` (ID §6.6; the
choice between them is DM §5.4's).

**R57 · MUST.** A **pragma occurrence** is, inside a `comment` token (IR §12.1, verbatim):

```
@verifies <SP>+ <intent-id> "/" "AC-" <digit>+
```

`<SP>` is `U+0020` or `U+0009`; `<intent-id>` is `("INT" | "BUG") "-" numeral`, the numeral left-padded with
`0` to a minimum width of 3 and no further — so `INT-042`, `BUG-051`, `INT-1042` are ids and `INT-42`,
`INT-0042`, `INT-000`, `int-042` are not. "`@verifies` must be preceded by a byte outside `[A-Za-z0-9_@]` or
be at the comment's start, so `x@verifies` is not one." Comment forms: "`#` for Python; `//` and `/* */` for
the other three (nested for Dart and Swift) … Docstrings are **not** comments and are not scanned."

**R58 · MUST.** "The AC number is captured as written, and compared canonically." `<digit>+` is wider than
`AC-1 … AC-6` so that a bad number is *recognized in order to be reported*; `AC-9`, `AC-01`, `AC-007` are
occurrences that name no acceptance criterion, seed nothing, and are G5 findings (IR §12.1).

**R59 · MUST.** The join is **file-granular**: "A pragma occurrence in file `P` attributes to **every
collected test id whose `id → path` equals `P`**, for every runner in the invocation set." (IR §12.2)
`attributed` follows PB §6.2 unchanged (IR §12.2).

**R60 · MUST.** Naming sugar: "the byte sequence `AC` followed by one or more digits, preceded by a byte
outside `[A-Za-z0-9]` or at the start of the field, and followed by a byte outside `[0-9]` or at the end of
the field. The capture is the digit run, and the intent is the branch's single gated intent." Field per
runner (IR §12.3): `pytest` — the final `::` component of `fn`, parametrization suffix removed; `vitest` —
the final ` > ` component of `id`; `dart-test` — the bytes of `id` after the first `::`; `swift-test` — the
bytes of `id` after the `/`, with a leading `test` removed if present. "Several `AC<n>` matches in one field
yield several edges"; "where a file carries both a pragma and a matching name, the edges are the union and
no rule prefers one."

### B. Multiply-derived elements

**R61 · MUST.** "**When a derivation produces the same element from more than one citation, the dump emits
one record whose `src` is the minimum, under §6.4's ordering, of those citations.**" (DM §5.5)
Applied to nodes: `code_unit` nodes take the minimum `src` over the edges naming them; every other kind has
one citation by construction. Applied to edges: "records that are equal in `from`, `to`, `kind` and `attrs`
collapse to one with the minimum `src`; records that differ in `attrs` are two edges and both are emitted."

### C. The dump projection (`spine index --dump`)

**R62 · MUST.** "**The dump is a projection of the graph, not the graph.** The store … holds in-flight
intents, provisional changesets, volatile test results and the shipped floor … The dump holds only what a
fresh clone of trunk can rederive." (DM §1)

**R63 · MUST.** The generating exclusion rule (DM §8.1, verbatim):

> **A graph element is in the dump if and only if it is derived from git objects reachable from the trunk
> tip. An element derived from anything else — an intent branch, the collector's result file, a coverage
> report, the binary's own floor list, or a heuristic over the objects rather than the objects — is
> excluded.**

**R64 · MUST.** Per-node-kind inclusion (DM §8.2):

| kind | included when | excluded when |
|---|---|---|
| `intent` | a first-parent trunk commit carrying `Spine-Seal` names it — a `Spine-Event: land` landing or a `withdraw` tombstone | in flight (branch is `refs/heads/intent/*`, absent from the clean clone) |
| `ac` | its intent is included | its intent is not |
| `test` | derived from a `Spine-Test` id or a pragma in a `Spine-Frozen` path of an included landing, parsed from `<L>:<path>` | derived from a branch's test files before the landing |
| `code_unit` | named as `from` or `to` by an included edge | named only by an excluded edge — in particular a path on the shipped floor and nothing else |
| `changeset` | a first-parent trunk commit carrying `Spine-Seal`, at or above the trust root; and every member of `M(L) = git rev-list B..L` | in flight (provisional); below the trust root; inside an `--uninstall` → re-init range, which G9 exempts |
| `approval` | its signed line is copied into an included landing's envelope | the line is on an in-flight event commit and has not landed |
| `signer` | `.spine/allowed_signers` at every trunk first-parent commit from the trust root — "purely trunk-derived, so always included" | never |
| `adr` | an ADR file present in `adr/` in the trunk tip's tree | "never, in practice; a deleted ADR is not a node" |
| `constitution` | every distinct version observed on the first-parent walk from the trust root | never |

**R65 · MUST.** Per-edge-kind inclusion (DM §8.3): `has_ac`, `declares` (both polarities), `built_under`,
`implements`, `modifies`, `approves`, `signed_by`, `attested_by`, `freezes`, `verified_by`, `reverts`,
`supersedes`, `superseded_by` — **yes**; `protects` — **partly** (`C-A2` limb only); `exercises` — **no**.

**R66 · MUST.** Two attrs are excluded from otherwise-included kinds (DM §8.4): `test.result_at` and
`verified_by.introduced_by`.

**R67 · MUST.** `verified_by.introduced_by` is excluded because "`git blame` has no specified output
contract: its rename and copy detection are heuristics whose defaults and behaviour have changed across git
releases, so the value is a function of the git binary rather than of the objects" (DM §8.5 clause 1).
"Pinning blame's flags was considered and refused."

**R68 · MUST.** Shipped-floor `protects` edges are excluded because including them "would make the dump a
function of the release, which §3.4 forbids" and "would also require a node kind for the release, which PB
§6.2 does not have" (DM §8.5 clause 2).

**R69 · MUST.** † states: PB §6.3 says they are "dumped as `tests-approved`". Under DM §8.2 the clause is
**vacuous** (in-flight intents are not dumped) but is retained as a conformance check: "a dump containing a
† status is non-conforming and the check is one string comparison" (DM §8.6).

**R70 · MUST.** The worktree property (DM §8.7, verbatim):

> **A dump is a function of trees and refs. Running `--dump` in a bare repository, with a dirty working
> tree, with a stale index, or with untracked files present produces identical bytes.**

Consequence: "PB §6.1's `<path>:<line>` provenance production … is never emitted by a dump; every file
citation in a dump is the `git:<sha>:<path>:<line>` form, anchored to a commit."

**R71 · MUST.** The closed input set (DM §4.1, verbatim):

> **A dump is a function of exactly four things: the trunk tip's oid, the git objects reachable from it, the
> trust root, and the pinned release. Nothing else may influence one byte.**

**R72 · MUST.** Trunk resolution order (DM §4.2): (1) explicit `--trunk <name>` when offered; (2)
`params.trunk` in `.spine/manifest.json` in the tree of the commit `HEAD` resolves to; (3) `params.trunk` in
the manifest of "the newest first-parent ancestor of `HEAD` whose tree carries one"; (4) none → refuse.
"Steps 2 and 3 read a **tree**, never the working directory, so a bare repository resolves identically to a
checked-out one."

**R73 · MUST NOT.** Never inputs (DM §4.3): any ref but `refs/heads/<trunk>` (including
`refs/heads/intent/*`, `refs/remotes/*/*`, tags, `refs/notes/*`, provider refs); `refs/notes/spine`; the
working tree, index or `git status`; a persisted/fetched/restored store; the collector's result file; a
coverage report; any wall clock; the environment; repository/user/system git config beyond
`extensions.objectFormat` and `spine.trustRoot`.

### D. Session resume (PB §6.4)

**R74 · MUST.** "a resuming agent runs `spine context INT-042` and receives the intent doc, its ACs and their
current test results, the frozen manifest and reopen history, declared touchpoints and any active lease
collisions, the constitution version it was built under, and any ADRs touching the same code units —
assembled from the graph, scoped to the task, with zero reliance on anyone's chat history." (PB §6.4)

The query decomposes to: `intent` node + `has_ac` → `ac` nodes + those ACs' `verified_by` edges and the
tests' `result_at` (store-only, R37) + the binding approval's `freezes` edges + `reopen_count`/
`late_reopen_count` + `declares` (both polarities) + G7 lease state + `built_under` → `constitution:v<n>` +
`adr` nodes "touching the same code units" — **which has no derivable edge; see DM §14 D6 in "Contradictions
found"**.

**R75 · MUST.** Role scoping: "Agent B still receives only its intent doc, its tests and the interface slice
of §4.2: `spine context` scopes by role, and in-flight intents on other branches are never part of an
agent's packet." (PB §6.4) CLI form: `spine context <id> [--role A|B]` (PB §11), roadmap 3+, not v1.

**R76 · MUST NOT.** `spine context`, `spine stats`, `spine review` read the **store**, never a dump: "They
read the store, which holds more than the dump does, and none of them reads a dump." (DM §16)

### E. The dependability suite (PB §6.5) — all three read data the graph already collects

**R77 · SHOULD (roadmap, not v1).** `spine stats` reports "cycle time per intent, **token cost per intent**
…, A↔B bounce-back counts, wire fire rates by gate, quick-lane escalation rate, and every counter this
playbook defines (reopens and late reopens, red-at-approval ratio, freeze and scaffold overrides, re-verify
count, starvations, withdrawals, unattested, resealed and recovery landings, self-approved protected
reviews, reviewer diversity …). … Output is text; someone else can chart it — no dashboard UI in scope."
(PB §6.5) Plus README settled decision 5: a counter for "landings whose only protected wire is a G7 hard
lease", "a predicate over a gate report's wire set, computed at read time and stored in no member".

**R78 · SHOULD.** `spine review <id>` assembles "the intent doc, the tests grouped by AC with their frozen
blobs, the diff of the synthetic merge, and exactly which wire tripped and why" — `spine check --review`
ships the minimal version in v1, the rich packet at roadmap step 6 (PB §6.5).

**R79 · SHOULD.** `spine eval` — "a golden-set harness for the interview agent: replay past intents from
their envelopes, score AC testability and non-goal coverage" (PB §6.5).

### F. Post-landing lifecycle (PB §6.6) — three further states, all derived from git objects

**R80 · MUST.** `reverted` — derived by the patch-id rule of PB §6.2 (step 9 above). "A partial reversal is a
warning, not a status flip. Tombstones and reseals never revert and are never reverted; a revert is never
matched against a landing that follows it." A revert is both `cs:R implements BUG-051` and `cs:R reverts
cs:L` (PB §6.6). The reverse transition exists: "reverted | that revert itself fully reverted | merged"
(PB §6, transition table).

**R81 · MUST.** `superseded` — "a later intent whose `Supersedes:` header names this one lands with a
`Spine-Supersedes` trailer; the indexer emits `superseded_by`, so archaeology queries return the current
truth first and the history behind it." (PB §6.6)

**R82 · MUST.** `withdrawn` — "a tombstone landing (§5.5) records an abandoned intent on trunk: no code, the
signed doc, the reason. … the id is retired." Plain withdrawal is signed by the branch's `Spine-Signoff`
key; an orphaned branch is withdrawn with `--withdraw --protected` under `spine-review@v1` by a reviewer ≠
the original signer (PB §6.6).

**R83 · MUST.** The governing sentence (PB §6.6, verbatim): "A revert is detected, never declared; a
supersession is sealed, never asserted; a withdrawal is landed, never deleted."

### G. G10 — the comparison this schema exists to survive

**R84 · MUST.** Procedure (DM §11, matching PB §6.3's G10 row):

1. `L` is pushed into the scratch clone `S` as `refs/heads/<trunk>` with the intent ref deleted, "so `S`
   holds the post-CAS ref set".
2. `S` is cloned with `--no-local --no-hardlinks file://S`, `GIT_CONFIG_GLOBAL=/dev/null`, no network,
   default refs only — "no notes, no custom refs, no provider metadata".
3. "The runner's pinned trust root is written to `spine.trustRoot` **in both repositories**."
4. `spine index --fresh --dump` runs in each, producing `D_S` and `D_C`.
5. "**The comparison is `D_S == D_C` as byte strings.** Equal digests (§2.5) are an equivalent
   implementation. Nothing parses either stream."
6. "Unequal: the push is refused, `L` is discarded, the run ends `reconstruction-failed` without a retry and
   without consuming a `C-M3` re-verification, and the run's own report is the only record."

**R85 · MUST.** G10 runs **before** the CAS, on the candidate landing `L` built at step 4 of PB §5.4, "and
never by moving the runner's own refs" (PB §6.3 G10).

**R86 · MUST NOT.** "**G10's own result never enters `Spine-Gates`**" (DM §11; PB §11: "every gate that ran,
never G10 (it runs after the seal)"). Break-glass cannot bypass G10 — it is not in PB §7.6's list (DM §1).

**R87 · MUST NOT.** "There is no deferred mode." (PB §6.3 G10)

**R88 · MUST.** G10 proves the ledger, not the lease registry (PB §6.3 G10; DM §1, §4.1) — which is why
every in-flight element is excluded (R63/R64).

---

## Byte-level fixities

All verbatim unless marked.

**F1 · Serialization.** "A dump is **JSON Lines**: a sequence of records, one per line, each record
serialized in its **RFC 8785 JSON Canonicalization Scheme (JCS)** form, restricted by the value profile of
§2.3 and the `esc` encoding of §2.4." (DM §2.1)

**F2 · Record grammar is closed.** "exactly three record kinds, distinguished by the `t` member —
`"header"`, `"node"`, `"edge"`. No other value of `t` is legal, and an unknown member name inside a known
record kind is not tolerated (§3.2)." (DM §2.1)

**F3 · Framing (DM §2.2, verbatim).**
1. "The byte stream is a sequence of **lines**, each terminated by exactly one `0x0A` (LF). The final line is
   terminated too, so the stream ends with `0x0A`. No CR anywhere, no BOM, no blank lines, no comments, no
   trailing blank line."
2. "Line 1 is the **header** record (§3.1). It is present in every dump, including an empty one (§9)."
3. "Lines 2 … *m* are the **node** records, in the order of §6.2."
4. "Lines *m*+1 … *n* are the **edge** records, in the order of §6.3."
5. "Nothing follows. A dump has no footer, no count and no digest of itself (§10 rule 11)."

**F4 · stdout only.** "`spine index --dump` writes exactly these bytes to **stdout** and nothing else to
stdout. Diagnostics, warnings and progress go to stderr, which is not part of the artifact." (DM §2.2)

**F5 · Value profile (DM §2.3, table verbatim in substance).**
- Member names match `^[a-z][a-z0-9_]*$`, ASCII only. "The complete set is fixed by §5: `attrs`,
  `dump_version`, `from`, `head`, `id`, `kind`, `object_format`, `repo`, `schema_version`, `src`, `t`, `to`,
  `trunk`, `trust_root`, plus the attr names of §7.2."
- Numbers: "Integers only, `0 ≤ n ≤ 2^53 − 1`. No sign, no leading zero, no fraction, no exponent, no `-0`.
  There is no floating-point value anywhere in a dump."
- Strings: "ASCII only after `esc` (§2.4): every character is in `U+0020 … U+007E`."
- Booleans appear only where §7.2 names them. Null: "Never emitted. An absent value is an absent member."
- Duplicate names: invalid. Arrays: "Elements are strings only. Order is fixed per attr by §7.2."
- Depth: "Exactly two: a record object, whose `attrs` member is an object of scalars and string arrays.
  **No attr value is an object.**"

**F6 · JCS reduced (DM §2.3, verbatim).** "sort each object's members by member-name bytes ascending; emit
with no whitespace; emit integers in plain decimal; emit strings with JSON's minimal escaping (`"` → `\"`,
`\` → `\\`, nothing else can occur); output UTF-8, which is here also ASCII."
Non-normative implementation note: `json.dumps(obj, sort_keys=True, separators=(',',':'),
ensure_ascii=False).encode('utf-8')` is byte-identical **for this profile only**.

**F7 · `esc` (DM §2.4).** "**Every value in a dump that carries repository bytes or human bytes is encoded
with `esc` as defined in `gate-report.md` §2.3, and is thereafter pure ASCII.**" Reminder of the definition:
"`0x5C` becomes `\` `\`; `0x20`–`0x7E` other than `0x5C` pass through; every other byte becomes `\` `x` and
two **lowercase** hex digits." Applies to: "every node `id`, every `from` and `to`, every `src`, the
header's `repo` and `trunk`, and every attr whose §7.2 row says *bytes*." Does not apply to object ids,
integers, booleans, or closed enumerations (where it is the identity).

**F8 · `tok` is not used here.** "The `tok` variant of `gate-report.md` §6.2 is not used here. … The one attr
that carries wire tokens — `approval.wires` (§7.2) — carries them as `tok` produced them, because those are
the bytes the signed line contains" (DM §2.4).

**F9 · Nothing is ever normalized.** "No NFC, no NFD, no case folding, no separator rewriting, no path
cleanup." "Where a gate itself casefolds — G14 casefolds before floor comparison (PB §7.3) — the dump records
the path **as the tree spells it**, never the casefolded form." "**Paths are the tree's bytes, never the
filesystem's.**" (DM §2.4)

**F10 · The dump digest.** "`sha256:` + 64 lowercase hex digits over exactly the byte stream of §2.2 —
including the final LF, excluding nothing." "It is **never sealed, never signed, never a trailer field, and
never a member of a gate report**." (DM §2.5)

**F11 · The header record (DM §3.1, verbatim shape).**

```json
{"dump_version":1,"head":"<oid>","object_format":"sha1","repo":"<esc>","schema_version":7,"t":"header","trunk":"<esc>","trust_root":"<oid>"}
```

| Member | Type | Presence | Value |
|---|---|---|---|
| `dump_version` | integer | always | `1` |
| `object_format` | string | always | `"sha1"` \| `"sha256"` — **the indexed repository's own** (`extensions.objectFormat`; absent means `sha1`); fixes every oid at 40 or 64 lowercase hex |
| `repo` | string, bytes | always | the manifest's `repo`, `esc`-encoded |
| `schema_version` | integer | always | `7` (PB §6.2's `PRAGMA user_version`) |
| `t` | string | always | `"header"` |
| `trunk` | string, bytes | always | the resolved trunk **branch name** (not a full refname), `esc`-encoded |
| `head` | string | iff `refs/heads/<trunk>` resolves | its full oid |
| `trust_root` | string | iff `spine.trustRoot` is configured | its full oid |

**F12 · Node record shape (DM §5.1).** `{"attrs":{…},"id":"<esc>","kind":"<kind>","src":"<esc>","t":"node"}`
— all five members always present; `attrs` is `{}` when the kind has none, "never absent".

**F13 · Edge record shape (DM §5.3).**
`{"attrs":{…},"from":"<esc>","kind":"<kind>","src":"<esc>","t":"edge","to":"<esc>"}` — all six always
present.

**F14 · `src` productions and their uses (DM §5.4, table verbatim).**

| Production | Shape | Used for |
|---|---|---|
| file line | `<path>:<line>` | working-tree line — **never emitted by a dump** |
| commit | `git:<sha>` | a whole commit — `modifies` from that commit's diff, a member changeset |
| message line | `git:<sha>:msg:L<n>` | a line of the envelope's fenced intent bytes — intent, ac, `has_ac`, `declares`, `built_under` |
| trailer | `git:<sha>:trailer:<Name>` | a signed line — approval nodes, `approves`, `signed_by`, `attested_by`, `freezes`, the landing changeset |
| patch id | `git:<sha>:patch-id` | `reverts` |
| file line at a commit | `git:<sha>:<path>:<line>` | test nodes, `verified_by`, signer nodes, `protects` from `C-A2`, adr and constitution nodes |
| shipped floor | `spine:<version>:floor` | the release's floor list — **never emitted by a dump** |

"`<sha>` is a full oid at `object_format`'s length. `<line>` and `<n>` are decimal integers ≥ 1 with no
leading zero. `<path>` is `esc(path bytes)`. `<Name>` is a trailer name." (DM §5.4)

**F15 · The node sort key (DM §6.2, verbatim).**

```
key_node(r) = r.kind ‖ NUL ‖ r.id ‖ NUL ‖ canonical(r.attrs) ‖ NUL ‖ r.src
```

"`kind` before `id` is PB §6.3's order and is preserved even though `id` alone is unique."

**F16 · The edge sort key (DM §6.3, verbatim).**

```
key_edge(r) = r.from ‖ NUL ‖ r.to ‖ NUL ‖ r.kind ‖ NUL ‖ canonical(r.attrs) ‖ NUL ‖ r.src
```

Sorted ascending "after the collapse of §5.5". `attrs` is a tie-breaker, not identity.

**F17 · Sections.** "Nodes first, then edges. The header is line 1 by framing (§2.2), not by sort."
(DM §6.1)

**F18 · Comparison (DM §6.4).** "Comparison is **ascending over those bytes**, unsigned, with the shorter
string first when one is a prefix of the other." "`NUL` makes the concatenation faithful" — no component can
contain `0x00` because `esc` maps it to `\x00`.

**F19 · The encoded order governs (DM §6.4, the trap).** For `src/z.py` versus `src/` + `0xE9` + `.py`:
"raw bytes: `src/z.py` first, because `0x7A < 0xE9`; `esc` bytes: `src/\xe9.py` first, because
`0x5C < 0x7A`. **The encoded order governs**".

**F20 · Numeric-looking ids sort as bytes.** "`AC-10` precedes `AC-2`; `G11` precedes `G2`." (DM §6.4) This
agrees with PB §11's signed `wires=` order by rule, not by dependence.

**F21 · A dump is not `sort`-stable.** "Sort by the key of §6.2 and §6.3, never by the line." An empty
`attrs` sorts *after* a non-empty one at the line level (`}` is `0x7D`, `"` is `0x22`), and a line sort
would interleave the sections (DM §6.5).

**F22 · Attr value profile.** "An attr value is a **string**, a non-negative **integer** in
`[0, 2^53 − 1]`, a **boolean**, or an **array of strings**. Never an object, never `null`, never a number
outside that range. An attr name matches `^[a-z][a-z0-9_]*$`". (DM §7.1)

**F23 · `attrs` is always present.** "A kind with no attrs emits `{}`. `{}` is a value; an omitted `attrs`
member is not a legal record." (DM §7.1)

**F24 · Absence vs null.** "`null` never appears; a member is present or absent … Absence means *this concept
does not apply to this element*, never *unknown* and never *empty*." (DM §7.3)

**F25 · Determinism rules, collected (DM §10, normative, condensed to their operative clauses).**
1. No wall clock. "Committer dates may be read by a derivation; none is a value."
2. No environment. "No hostname, runner id, user, locale, process id, temp path, or path outside the
   repository."
3. No state the design forbids. "`--dump` implies `--fresh`."
4. Key ordering inside a record is JCS's — ascending by member-name bytes, "Never insertion order".
5. Record ordering is §6's, "never over the serialized line".
6. Absent versus null; `attrs` always present, `{}` when empty.
7. Numbers are integers in `[0, 2^53 − 1]`, plain decimal, no leading zero.
8. Paths, principals, ref names and trailer values are `esc`-encoded and "never normalized, casefolded or
   separator-rewritten".
9. "Object ids are lowercase hex at `object_format`'s full length — 40 or 64 digits. Never abbreviated,
   never uppercase, never prefixed."
10. "Non-git digests are `sha256:` + 64 lowercase hex, except the `approval` local id, which is bare hex
    inside an id."
11. "No self-reference." No own digest, length, record count, or producing release.
12. "**Git plumbing is pinned by the release.** Every invocation the derivation makes — `diff`, `rev-list`,
    `merge-tree`, `patch-id`, `ls-tree` — runs with its diff algorithm, rename and copy detection, and every
    other output-affecting option fixed by the release and never read from repository, user or system
    config." "A repository that sets `diff.algorithm` must not thereby change its own dump."
13. "One binary per comparison."

**F26 · Versioning.** `dump_version` and `schema_version` are two facts and move independently (DM §3.3).
"a store-schema change that adds an excluded attr changes `schema_version` and not `dump_version`; a
projection change … changes `dump_version` and not `schema_version`; a store-schema change that adds an
*included* element changes both."

**F27 · The producing release is not in the dump.** "`cli.version` and `cli.dist_hash` are **not** header
members … two releases carrying the same `dump_version` and `schema_version` **must** produce identical
bytes over identical objects." "a release that changes the projection **must** bump `dump_version`, even for
a change it believes is a bug fix." (DM §3.4)

**F28 · Hash policy (PB §11).** "Git object ids (`<oid>`, in the repo's object format) for everything that is
a git object … SHA-256 (`sha256:<hex>`) only for non-git artifacts".

---

## Error cases

| # | Condition | Behaviour | Exit / status token / message |
|---|---|---|---|
| E1 | Trunk resolution reached step 4 of DM §4.2 (no manifest resolvable) | **REFUSE**; "Nothing is written to stdout" | exit **2**, status `not-installed` (DM §4.4) |
| E2 | Dump produced successfully | write to stdout | exit **0**, status `ok` (DM §4.4) |
| E3 | The derivation produced a `src` outside PB §6.1's grammar | **REFUSE** the dump | exit **3**, status `provenance-invalid` (DM §4.4, §5.4) |
| E4 | The derivation produced a node id outside DM §5.2 (includes a manifest `repo` outside `^[A-Za-z0-9._-]+$`) | **REFUSE** | exit **3**, status `id-out-of-grammar` (DM §4.4, §5.2) |
| E5 | An attr value outside DM §2.3/§7.2 — "a float, a `null`, a nested object, an unknown name" | **REFUSE** | exit **3**, status `attrs-out-of-profile` (DM §4.4, §7.1) |
| E6 | G10 offered two dumps with different `dump_version` or `schema_version` | **REFUSE** rather than compare | exit **3**, status `dump-version-skew` (DM §3.2, §4.4) |
| E7 | `D_S ≠ D_C` at G10 | push refused; `L` discarded and "never becomes a git object"; run ends **without a retry** and **without consuming a `C-M3` re-verification**; the run's own report is the only record | status `reconstruction-failed` (DM §11 step 6; PB §6, transition table row `checked† → reconstruction-failed`; PB §6.3 G10; PB §11 States) |
| E8 | A human or external tool meets an unknown `dump_version`, or an unknown member name inside a known version | **REFUSE**. "The schema is closed: forward compatibility is bought with a version bump, not with tolerance" | no code given (DM §3.2) |
| E9 | A `verified_by` edge to a nonexistent AC (typo'd pragma); a pragma outside every frozen blob; a pragma first appearing after its intent's approval (`attributed: false`) | G5 "fails loudly" | wire **`G5:<path>`**, `class=tripwire`, one wire per offending pragma; "the path is the blob the pragma sits in" (PB §6.3 G5) |
| E10 | A `built_under` edge naming a `constitution:v<n>` no landing ever carried | the edge dangles and **G5 reports it** | G5 finding; "No new gate, no new status" (CN §9.5) |
| E11 | An event commit whose signature fails, or whose role disagrees with its namespace | excluded from state derivation; raised as a wire naming the sha | **G13** wire (PB §6.2 derivation table) |
| E12 | A landing whose envelope fails G9 on re-index (edited message, unknown key, base ≠ first parent, tree ≠ merge-tree, missing sign-off or approval) | indexes `unattested` — "reported and counted forever" | status `unattested` (PB §6, transition table; PB §6.3 G9) |
| E13 | Indexer's git ≠ the seal's `git=` **and** the recomputed merge tree differs (strategy `merge`) | record the sentinel; "reported, not `unattested`" | `changeset.tree = unverifiable(git-version)` (PB §6.3 G9; DM §7.2.1) |
| E14 | Strategy `squash` | tree rule never consulted | `changeset.tree = unverifiable(squash)` (PB §6.3 G9; DM §7.2.1) |
| E15 | `reverts` matching finds missing hunks inside `L`'s paths | `{partial: true}` **and a warning**; not a status flip | edge attr `partial: true` (PB §6.2, §6.6) |
| E16 | An in-flight intent's blob changes without a signed `Spine-Reopen` | state returns to `awaiting-sign-off` **and** Integrity fails | G8 + G9 (PB §6 transition table) |
| E17 | Two keys under one principal in the keyring | **REFUSE** (the signer node would be unrepresentable) | `keyring-duplicate-principal` (MF §4.5, §9 R7) |
| E18 | A user tries to author a workflow DAG | **REFUSE** | no token given (PB §6) |
| E19 | Cache's `PRAGMA user_version` ≠ the binary's constant, or `meta` shows another binary / another manifest / an older trunk tip | never queried — **delete and rebuild** | not an error surface (PB §6.2, §6.7 step 6) |
| E20 | A dump containing a † intent status, an `implements` with `provisional: true`, a `protects` with `floor: true`, a `test` with `result_at`, a `verified_by` with `introduced_by`, or an `exercises` edge | **non-conforming** | DM §17 items 15–19, §8.6 |
| E21 | A well-formed edge id naming no node record | "an indexer defect that this format cannot detect and G5 must" | not detectable by the serializer (DM §5.3) |

---

## Worked examples / test vectors

All three vectors below are marked **verified** in DM §12 ("computed over the exact bytes printed, by a
serializer written from §2 alone").

### V0 · The example repository (DM §12.1)

`myrepo`, `object_format: sha1`, `params.trunk: main`, `params.langs: [python]`, pinned release 1.4.0, team
mode, `C-A3: hostile`, `C-M1: merge`. Keyring principals: `alice@example.com`, `bob@example.com`,
`ci@example.com` (MF §8.7 / EV §8.1). Trust root `T0` = `0a1b…4567`; landing `L` = `1b2c…6789`;
`M(L)` = five members `M1` `2c3d…8901`, `M2` `3d4e…012a`, `M3` `4e5f…2ab3`, `M4` `5f60…b3c4`,
`M5` `60718…c4d5`; `Hc` = `M5`.

The three signed lines whose exact bytes DM §5.2.1 hashes (DM §12.1, verbatim):

```
Spine-Signoff: INT-042 blob=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 template=intent@2 constitution=v3 reopens=0 signer=alice@example.com
Spine-Approve: INT-042 intent=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 base=0a1b2c3d4e5f60718293a4b5c6d7e8f901234567 rounds=1 total_rounds=1 reopens=0 red=5/5 freeze=sha256:5c9e2a71b0463df8951ce2a4708b3d61f0492c8ad735be106f4a2c9d80e37b45 signer=alice@example.com
Spine-Review: INT-042 class=tripwire head=60718293a4b5c6d7e8f90123456789012ab3c4d5 tree=7b0dc1f4a2e58d3906bb4c7e21f5a8d90c3e64b7 base=0a1b2c3d4e5f60718293a4b5c6d7e8f901234567 intent=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 report=sha256:3c6f1a09b8d24e57af0132c9de6b48570e29a1cf83b6d045e71a29c4b0d83e16 wires=G11 reason="auto-merge unavailable: C-A3 hostile" reviewer=bob@example.com
```

giving the `approval` local ids (SHA-256 over each line's bytes, no LF):

| line | `approval` local id |
|---|---|
| `Spine-Signoff` | `2f5e600237ec3d9a1f35fbc7ee6cf0dfd4335433def9937b5eeb8202bd3d66f6` |
| `Spine-Approve` | `b6352921ea42d618930f32f04ad773c20157810d418d20d06758149f366b85a8` |
| `Spine-Review` | `ae8a406391f7130ce62d0e86fa4dca92195812aae2743e15e60434af56244021` |

Coverage claimed by the vector (DM §12.1): "every node kind; **eleven** of the fifteen edge kinds (`reverts`,
`supersedes` and `superseded_by` have no occasion in a two-commit history; `exercises` is excluded from every
dump by §8; `protects` **is** present, but only its `C-A2` limb …); a non-UTF-8 path through `esc`; the
`msg:L<n>`, `trailer:<Name>`, `git:<sha>` and `git:<sha>:<path>:<line>` provenance productions; an absent
optional attr (`signer.valid_to`); an array attr; and the `code_unit` minimum-`src` rule of §5.5."

### V1 · The full dump (DM §12.2 — 62 lines, 14054 bytes)

```
lines:  62
bytes:  14054
digest: sha256:3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da
```

Reproduced verbatim from `docs/spec/dump.md` §12.2 (lines 603–664 of that file):

```
{"dump_version":1,"head":"1b2c3d4e5f60718293a4b5c6d7e8f90123456789","object_format":"sha1","repo":"myrepo","schema_version":7,"t":"header","trunk":"main","trust_root":"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567"}
{"attrs":{},"id":"myrepo/INT-042/AC-1","kind":"ac","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L24","t":"node"}
{"attrs":{},"id":"myrepo/INT-042/AC-2","kind":"ac","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L25","t":"node"}
{"attrs":{},"id":"myrepo/ADR-007","kind":"adr","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:adr/ADR-007-tax-rounding.md:1","t":"node"}
{"attrs":{"blob":"9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2","event":"signoff","principal":"alice@example.com","reopens":0,"role":"signer","verified":true},"id":"myrepo/approval:2f5e600237ec3d9a1f35fbc7ee6cf0dfd4335433def9937b5eeb8202bd3d66f6","kind":"approval","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Signoff","t":"node"}
{"attrs":{"base":"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567","blob":"9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2","class":"tripwire","event":"review","head":"60718293a4b5c6d7e8f90123456789012ab3c4d5","principal":"bob@example.com","role":"reviewer","tree":"7b0dc1f4a2e58d3906bb4c7e21f5a8d90c3e64b7","verified":true,"wires":["G11"]},"id":"myrepo/approval:ae8a406391f7130ce62d0e86fa4dca92195812aae2743e15e60434af56244021","kind":"approval","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Review","t":"node"}
{"attrs":{"base":"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567","blob":"9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2","event":"approve","freeze":"sha256:5c9e2a71b0463df8951ce2a4708b3d61f0492c8ad735be106f4a2c9d80e37b45","principal":"alice@example.com","red":"5/5","reopens":0,"role":"reviewer","rounds":1,"total_rounds":1,"verified":true},"id":"myrepo/approval:b6352921ea42d618930f32f04ad773c20157810d418d20d06758149f366b85a8","kind":"approval","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Approve","t":"node"}
{"attrs":{"base":"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567","event":"land","git_version":"2.45","head":"60718293a4b5c6d7e8f90123456789012ab3c4d5","landing":true,"lane":"gated","mode":"team","profile":"container","report_sha256":"sha256:3c6f1a09b8d24e57af0132c9de6b48570e29a1cf83b6d045e71a29c4b0d83e16","resealed":false,"seal_principal":"ci@example.com","seal_verified":true,"strategy":"merge","threat":"hostile","tool_version":"1.4.0","tree":"7b0dc1f4a2e58d3906bb4c7e21f5a8d90c3e64b7","unattested":false},"id":"myrepo/cs:1b2c3d4e5f60718293a4b5c6d7e8f90123456789","kind":"changeset","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Seal","t":"node"}
{"attrs":{"landing":false},"id":"myrepo/cs:2c3d4e5f60718293a4b5c6d7e8f9012345678901","kind":"changeset","src":"git:2c3d4e5f60718293a4b5c6d7e8f9012345678901","t":"node"}
{"attrs":{"landing":false},"id":"myrepo/cs:3d4e5f60718293a4b5c6d7e8f90123456789012a","kind":"changeset","src":"git:3d4e5f60718293a4b5c6d7e8f90123456789012a","t":"node"}
{"attrs":{"landing":false},"id":"myrepo/cs:4e5f60718293a4b5c6d7e8f90123456789012ab3","kind":"changeset","src":"git:4e5f60718293a4b5c6d7e8f90123456789012ab3","t":"node"}
{"attrs":{"landing":false},"id":"myrepo/cs:5f60718293a4b5c6d7e8f90123456789012ab3c4","kind":"changeset","src":"git:5f60718293a4b5c6d7e8f90123456789012ab3c4","t":"node"}
{"attrs":{"landing":false},"id":"myrepo/cs:60718293a4b5c6d7e8f90123456789012ab3c4d5","kind":"changeset","src":"git:60718293a4b5c6d7e8f90123456789012ab3c4d5","t":"node"}
{"attrs":{},"id":"myrepo/code:api/invoices.ts","kind":"code_unit","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L29","t":"node"}
{"attrs":{},"id":"myrepo/code:auth/","kind":"code_unit","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L30","t":"node"}
{"attrs":{},"id":"myrepo/code:infra/","kind":"code_unit","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:CONSTITUTION.md:96","t":"node"}
{"attrs":{},"id":"myrepo/code:pytest.ini","kind":"code_unit","src":"git:4e5f60718293a4b5c6d7e8f90123456789012ab3:trailer:Spine-Frozen","t":"node"}
{"attrs":{},"id":"myrepo/code:shared/schema/","kind":"code_unit","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L30","t":"node"}
{"attrs":{},"id":"myrepo/code:src/billing/","kind":"code_unit","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L29","t":"node"}
{"attrs":{},"id":"myrepo/code:src/billing/caf\\xe9.py","kind":"code_unit","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789","t":"node"}
{"attrs":{},"id":"myrepo/code:src/billing/tax.py","kind":"code_unit","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789","t":"node"}
{"attrs":{},"id":"myrepo/code:tests/billing/test_invoice.py","kind":"code_unit","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789","t":"node"}
{"attrs":{},"id":"myrepo/constitution:v3","kind":"constitution","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:CONSTITUTION.md:2","t":"node"}
{"attrs":{"base":"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567","blob":"9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2","landing":"1b2c3d4e5f60718293a4b5c6d7e8f90123456789","late_reopen_count":0,"owner":"@alice","reopen_count":0,"signer":"alice@example.com","status":"merged","template":"intent@2","title":"Invoice totals include tax"},"id":"myrepo/INT-042","kind":"intent","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L14","t":"node"}
{"attrs":{"fingerprint":"SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM","roles":["spine-review@v1","spine-signoff@v1"],"valid_from":"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567"},"id":"myrepo/signer:alice@example.com","kind":"signer","src":"git:0a1b2c3d4e5f60718293a4b5c6d7e8f901234567:.spine/allowed_signers:1","t":"node"}
{"attrs":{"fingerprint":"SHA256:V2dasTIGWUnhlaxa7vr3Qmqpe/qYBoxi+C7Cs6yxxEs","roles":["spine-review@v1","spine-signoff@v1"],"valid_from":"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567"},"id":"myrepo/signer:bob@example.com","kind":"signer","src":"git:0a1b2c3d4e5f60718293a4b5c6d7e8f901234567:.spine/allowed_signers:2","t":"node"}
{"attrs":{"fingerprint":"SHA256:eQ0ZoC+rlhZstRuvhLXwJlwqLAreBcmnyFedpvPmTVY","roles":["spine-seal@v1"],"valid_from":"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567"},"id":"myrepo/signer:ci@example.com","kind":"signer","src":"git:0a1b2c3d4e5f60718293a4b5c6d7e8f901234567:.spine/allowed_signers:3","t":"node"}
{"attrs":{},"id":"myrepo/test:pytest:tests/billing/test_invoice.py::test_AC1_totals_include_tax","kind":"test","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:tests/billing/test_invoice.py:7","t":"node"}
{"attrs":{},"id":"myrepo/test:pytest:tests/billing/test_invoice.py::test_AC2_zero_rated","kind":"test","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:tests/billing/test_invoice.py:19","t":"node"}
{"attrs":{},"from":"myrepo/INT-042","kind":"has_ac","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L24","t":"edge","to":"myrepo/INT-042/AC-1"}
{"attrs":{},"from":"myrepo/INT-042","kind":"has_ac","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L25","t":"edge","to":"myrepo/INT-042/AC-2"}
{"attrs":{"polarity":"expected"},"from":"myrepo/INT-042","kind":"declares","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L29","t":"edge","to":"myrepo/code:api/invoices.ts"}
{"attrs":{"polarity":"forbidden"},"from":"myrepo/INT-042","kind":"declares","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L30","t":"edge","to":"myrepo/code:auth/"}
{"attrs":{"polarity":"forbidden"},"from":"myrepo/INT-042","kind":"declares","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L30","t":"edge","to":"myrepo/code:shared/schema/"}
{"attrs":{"polarity":"expected"},"from":"myrepo/INT-042","kind":"declares","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L29","t":"edge","to":"myrepo/code:src/billing/"}
{"attrs":{},"from":"myrepo/INT-042","kind":"built_under","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:msg:L15","t":"edge","to":"myrepo/constitution:v3"}
{"attrs":{},"from":"myrepo/approval:2f5e600237ec3d9a1f35fbc7ee6cf0dfd4335433def9937b5eeb8202bd3d66f6","kind":"approves","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Signoff","t":"edge","to":"myrepo/INT-042"}
{"attrs":{},"from":"myrepo/approval:2f5e600237ec3d9a1f35fbc7ee6cf0dfd4335433def9937b5eeb8202bd3d66f6","kind":"signed_by","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Signoff","t":"edge","to":"myrepo/signer:alice@example.com"}
{"attrs":{},"from":"myrepo/approval:ae8a406391f7130ce62d0e86fa4dca92195812aae2743e15e60434af56244021","kind":"approves","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Review","t":"edge","to":"myrepo/INT-042"}
{"attrs":{},"from":"myrepo/approval:ae8a406391f7130ce62d0e86fa4dca92195812aae2743e15e60434af56244021","kind":"signed_by","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Review","t":"edge","to":"myrepo/signer:bob@example.com"}
{"attrs":{},"from":"myrepo/approval:b6352921ea42d618930f32f04ad773c20157810d418d20d06758149f366b85a8","kind":"approves","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Approve","t":"edge","to":"myrepo/INT-042"}
{"attrs":{"oid":"1e9f4c7a20d63b8859e04f1a7cd6b325908e4f71"},"from":"myrepo/approval:b6352921ea42d618930f32f04ad773c20157810d418d20d06758149f366b85a8","kind":"freezes","src":"git:4e5f60718293a4b5c6d7e8f90123456789012ab3:trailer:Spine-Frozen","t":"edge","to":"myrepo/code:pytest.ini"}
{"attrs":{"oid":"a41bd9c2e70f83615a4d2b8c09e7f1436d5028ba"},"from":"myrepo/approval:b6352921ea42d618930f32f04ad773c20157810d418d20d06758149f366b85a8","kind":"freezes","src":"git:4e5f60718293a4b5c6d7e8f90123456789012ab3:trailer:Spine-Frozen","t":"edge","to":"myrepo/code:tests/billing/test_invoice.py"}
{"attrs":{},"from":"myrepo/approval:b6352921ea42d618930f32f04ad773c20157810d418d20d06758149f366b85a8","kind":"signed_by","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Approve","t":"edge","to":"myrepo/signer:alice@example.com"}
{"attrs":{},"from":"myrepo/approval:b6352921ea42d618930f32f04ad773c20157810d418d20d06758149f366b85a8","kind":"freezes","src":"git:4e5f60718293a4b5c6d7e8f90123456789012ab3:trailer:Spine-Test","t":"edge","to":"myrepo/test:pytest:tests/billing/test_invoice.py::test_AC1_totals_include_tax"}
{"attrs":{},"from":"myrepo/approval:b6352921ea42d618930f32f04ad773c20157810d418d20d06758149f366b85a8","kind":"freezes","src":"git:4e5f60718293a4b5c6d7e8f90123456789012ab3:trailer:Spine-Test","t":"edge","to":"myrepo/test:pytest:tests/billing/test_invoice.py::test_AC2_zero_rated"}
{"attrs":{"floor":false},"from":"myrepo/constitution:v3","kind":"protects","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:CONSTITUTION.md:96","t":"edge","to":"myrepo/code:infra/"}
{"attrs":{"provisional":false,"role":"landing","verified":true},"from":"myrepo/cs:1b2c3d4e5f60718293a4b5c6d7e8f90123456789","kind":"implements","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Seal","t":"edge","to":"myrepo/INT-042"}
{"attrs":{},"from":"myrepo/cs:1b2c3d4e5f60718293a4b5c6d7e8f90123456789","kind":"modifies","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789","t":"edge","to":"myrepo/code:src/billing/caf\\xe9.py"}
{"attrs":{},"from":"myrepo/cs:1b2c3d4e5f60718293a4b5c6d7e8f90123456789","kind":"modifies","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789","t":"edge","to":"myrepo/code:src/billing/tax.py"}
{"attrs":{},"from":"myrepo/cs:1b2c3d4e5f60718293a4b5c6d7e8f90123456789","kind":"modifies","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789","t":"edge","to":"myrepo/code:tests/billing/test_invoice.py"}
{"attrs":{},"from":"myrepo/cs:1b2c3d4e5f60718293a4b5c6d7e8f90123456789","kind":"attested_by","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Seal","t":"edge","to":"myrepo/signer:ci@example.com"}
{"attrs":{"provisional":false,"role":"member","verified":true},"from":"myrepo/cs:2c3d4e5f60718293a4b5c6d7e8f9012345678901","kind":"implements","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Seal","t":"edge","to":"myrepo/INT-042"}
{"attrs":{"provisional":false,"role":"member","verified":true},"from":"myrepo/cs:3d4e5f60718293a4b5c6d7e8f90123456789012a","kind":"implements","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Seal","t":"edge","to":"myrepo/INT-042"}
{"attrs":{},"from":"myrepo/cs:3d4e5f60718293a4b5c6d7e8f90123456789012a","kind":"modifies","src":"git:3d4e5f60718293a4b5c6d7e8f90123456789012a","t":"edge","to":"myrepo/code:tests/billing/test_invoice.py"}
{"attrs":{"provisional":false,"role":"member","verified":true},"from":"myrepo/cs:4e5f60718293a4b5c6d7e8f90123456789012ab3","kind":"implements","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Seal","t":"edge","to":"myrepo/INT-042"}
{"attrs":{"provisional":false,"role":"member","verified":true},"from":"myrepo/cs:5f60718293a4b5c6d7e8f90123456789012ab3c4","kind":"implements","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Seal","t":"edge","to":"myrepo/INT-042"}
{"attrs":{},"from":"myrepo/cs:5f60718293a4b5c6d7e8f90123456789012ab3c4","kind":"modifies","src":"git:5f60718293a4b5c6d7e8f90123456789012ab3c4","t":"edge","to":"myrepo/code:src/billing/caf\\xe9.py"}
{"attrs":{},"from":"myrepo/cs:5f60718293a4b5c6d7e8f90123456789012ab3c4","kind":"modifies","src":"git:5f60718293a4b5c6d7e8f90123456789012ab3c4","t":"edge","to":"myrepo/code:src/billing/tax.py"}
{"attrs":{"provisional":false,"role":"member","verified":true},"from":"myrepo/cs:60718293a4b5c6d7e8f90123456789012ab3c4d5","kind":"implements","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:trailer:Spine-Seal","t":"edge","to":"myrepo/INT-042"}
{"attrs":{"attributed":true},"from":"myrepo/test:pytest:tests/billing/test_invoice.py::test_AC1_totals_include_tax","kind":"verified_by","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:tests/billing/test_invoice.py:6","t":"edge","to":"myrepo/INT-042/AC-1"}
{"attrs":{"attributed":true},"from":"myrepo/test:pytest:tests/billing/test_invoice.py::test_AC2_zero_rated","kind":"verified_by","src":"git:1b2c3d4e5f60718293a4b5c6d7e8f90123456789:tests/billing/test_invoice.py:18","t":"edge","to":"myrepo/INT-042/AC-2"}
```

Seven checkpoints the vector pins (DM §12.3):
1. "**Node order is `kind` then `id`**, so `myrepo/INT-042` (kind `intent`) is the twenty-third node, far
   below `myrepo/INT-042/AC-1` (kind `ac`), which is the first."
2. "**`myrepo/code:src/billing/caf\xe9.py` precedes `myrepo/code:src/billing/tax.py`**, which raw-byte order
   would reverse."
3. "**The `code_unit` src is the minimum over citing edges**: `tests/billing/test_invoice.py` is cited by
   `modifies` from `L` and from `M2` and by `freezes` from `M3`, and takes `git:1b2c…6789`, the least of the
   three."
4. "**Member changesets carry `{"landing":false}` and nothing else.**"
5. "**The two `declares` edges to `code:src/billing/` and `code:api/invoices.ts` share a `src`** — one line
   of the touchpoints block names both paths — and are ordered by `to`, PB's second key."
6. "**`signer.valid_to` is absent, not null**, on all three signers."
7. "**The three fingerprints are real** and reproduce from `envelope-vectors.md` §8.1's published keys with
   `ssh-keygen -lf`."

### V2 · The ordering vector (DM §12.4 — debug against this first)

Authored (arbitrary order):

```
{"attrs":{},"id":"r/code:src/z.py","kind":"code_unit","src":"git:aa","t":"node"}
{"attrs":{},"id":"r/code:src/\\xe9.py","kind":"code_unit","src":"git:aa","t":"node"}
{"attrs":{"landing":false},"id":"r/cs:bb","kind":"changeset","src":"git:bb","t":"node"}
{"attrs":{},"id":"r/INT-1/AC-2","kind":"ac","src":"git:cc:msg:L9","t":"node"}
{"attrs":{},"id":"r/INT-1/AC-10","kind":"ac","src":"git:cc:msg:L8","t":"node"}
{"attrs":{"polarity":"forbidden"},"from":"r/INT-1","kind":"declares","src":"git:cc:msg:L3","t":"edge","to":"r/code:src/z.py"}
{"attrs":{"polarity":"expected"},"from":"r/INT-1","kind":"declares","src":"git:cc:msg:L2","t":"edge","to":"r/code:src/z.py"}
{"attrs":{},"from":"r/INT-1","kind":"has_ac","src":"git:cc:msg:L8","t":"edge","to":"r/INT-1/AC-10"}
{"attrs":{},"from":"r/INT-1","kind":"has_ac","src":"git:cc:msg:L9","t":"edge","to":"r/INT-1/AC-2"}
{"attrs":{},"from":"r/cs:bb","kind":"modifies","src":"git:bb","t":"edge","to":"r/code:src/\\xe9.py"}
{"attrs":{},"from":"r/cs:bb","kind":"modifies","src":"git:aa","t":"edge","to":"r/code:src/\\xe9.py"}
```

Canonical (11 lines, each LF-terminated, no header — a fragment, not a dump):

```
{"attrs":{},"id":"r/INT-1/AC-10","kind":"ac","src":"git:cc:msg:L8","t":"node"}
{"attrs":{},"id":"r/INT-1/AC-2","kind":"ac","src":"git:cc:msg:L9","t":"node"}
{"attrs":{"landing":false},"id":"r/cs:bb","kind":"changeset","src":"git:bb","t":"node"}
{"attrs":{},"id":"r/code:src/\\xe9.py","kind":"code_unit","src":"git:aa","t":"node"}
{"attrs":{},"id":"r/code:src/z.py","kind":"code_unit","src":"git:aa","t":"node"}
{"attrs":{},"from":"r/INT-1","kind":"has_ac","src":"git:cc:msg:L8","t":"edge","to":"r/INT-1/AC-10"}
{"attrs":{},"from":"r/INT-1","kind":"has_ac","src":"git:cc:msg:L9","t":"edge","to":"r/INT-1/AC-2"}
{"attrs":{"polarity":"expected"},"from":"r/INT-1","kind":"declares","src":"git:cc:msg:L2","t":"edge","to":"r/code:src/z.py"}
{"attrs":{"polarity":"forbidden"},"from":"r/INT-1","kind":"declares","src":"git:cc:msg:L3","t":"edge","to":"r/code:src/z.py"}
{"attrs":{},"from":"r/cs:bb","kind":"modifies","src":"git:aa","t":"edge","to":"r/code:src/\\xe9.py"}
{"attrs":{},"from":"r/cs:bb","kind":"modifies","src":"git:bb","t":"edge","to":"r/code:src/\\xe9.py"}
```

```
bytes:  1063
digest: sha256:a849ec349ef8f20ec1f40423ae6a7d3358745f4c9027545f55cf74ef9b72a139
```

What it pins (DM §12.4): `AC-10` before `AC-2` (byte order, not numeric); `ac` < `changeset` < `code_unit`
(`kind` is the first node key); `\xe9.py` before `z.py` (`esc` order, the reverse of raw-byte order); two
`declares` edges tying on `from`/`to`/`kind` and breaking on `attrs` (`expected` before `forbidden`); two
`modifies` edges tying on `from`/`to`/`kind`/`attrs` and breaking on `src` (`git:aa` before `git:bb`); edges
under one `from` ordered by `to` before `kind`.

### V3 · The empty dump (DM §9, §12.5)

```
{"dump_version":1,"object_format":"sha1","repo":"myrepo","schema_version":7,"t":"header","trunk":"main"}
```

```
bytes:  105
digest: sha256:2a3fcea5debff8e17af36eef1a4acfa1fea9f86a0d6c942cf2993cce3e80f290
```

"One line, one LF, no `head`, no `trust_root` (§9 case 2)." Three distinguishable states around it (DM §9):
1. No manifest resolvable — **not** an empty dump: refuse `not-installed`, exit 2, nothing on stdout.
2. Manifest resolves but `refs/heads/<trunk>` does not — `head` absent, dump is the bytes above.
3. Trunk resolves and is at or below the trust root with no sealed landing above it — `head` **present**;
   empty only where `spine init` has not yet landed anything.
"An empty dump is legal, is not an error, and exits 0."

### V4 · Conformance checklist (DM §17) — 24 mechanically checkable items

Framing/encoding 1–6; Records 7–12; Order 13–14; Exclusions 15–20; Determinism 21–24. Reproduced in full
above as F1–F25, R43–R70 and E20. Two worth restating as tests:
- item 13: "Verified by re-sorting the records by key and comparing, **not** by sorting the lines."
- item 14: "`id` is unique across the node section; `(from, to, kind, attrs, src)` is unique across the edge
  section."
- item 20: "No record's `src` names a commit not reachable from `head`, and no `changeset` node names a
  commit below the trust root."

---

## Cross-references it depends on

| What | Owned by | Why this sheet needs it |
|---|---|---|
| `esc` (§2.3) and the wire token `tok` (§6.2) | **GR** `gate-report.md` | every byte-valued value in a node id, `src`, header or attr; `approval.wires` carries `tok` output verbatim (DM §2.4) |
| The canonical-JSON scheme and its minimal vector (§8.3) | **GR** | DM §2.1 reuses JCS; "**debug against that one before attempting §12**" (DM §2.3) |
| The intent-doc parse: id domain, AC numbering, touchpoint pattern dialect, `Template:` header | **ID** `intent-doc.md` §3.1, §5.3, §6.1, §6.6 | `intent`/`ac`/`code_unit` nodes, `has_ac`, `declares`, `built_under`, `intent.template` |
| The pragma grammar, the file-granular join, the naming sugar, `C-T3` | **IR** `import-resolver.md` §12.1–§12.4 | `test` nodes and `verified_by` edges; DM §16 calls this "the one out-of-scope pointer that can invalidate this document without touching it" |
| Runner tokens and `id → path` / `id → fn` | **IR** §11.1–§11.6 | the `test` node id's `<runner>` half and the join's path key |
| `effective(C-A2)`, the constitution header/version, `protects` shape | **CN** `constitution.md` §6.4, §6.5, §9.5, §9.6 | `constitution` nodes and the `C-A2` limb of `protects` |
| `repo` grammar (§3.1), `params.langs` (§3.3), manifest `schema` field, keyring parse (§4.2, §4.5, §4.6), G13/G14/G16 | **MF** `manifest.md` | the node-id prefix; `signer` node attrs; the keyring refusals that keep the signer node representable |
| `test.result_at` population and the `base` record's `out` member | **RF** `result-file.md` §2, §4.4, §8.5 | the one attr the store holds and the dump excludes |
| The envelope grammar, trailer syntax, what a `-Sig` covers, the fenced block's byte range | **EV** `envelope-vectors.md` | the byte range DM §5.2.1 hashes for the `approval` id; the fenced intent bytes every landed `intent` node parses |
| The three CI definitions; `--fresh`/no-cache in CI | **CI** `ci.md` §7.4/T5 | R18 |
| Gate semantics (G1…G16), wire aggregation, the signerless overlay | PB §6.3, PB §11 + GR/MF | this sheet records only how a derived fact is *recorded*, "never what a gate decides" (DM §16) |
| The floor list's contents | the release + `ci.md` | excluded from the dump (R68); OPEN-1 asks whether the store should carry it at all |

---

## OPEN items

Undecided owner questions. **Do not invent a value.**

1. **DM §15 OPEN-1 · Whether the shipped floor belongs in the graph, and under which node.** "PB §6.2 derives
   `protects` partly from *the floor list inside the pinned release* and gives no node kind that could be its
   `from_id`. The nine kinds are all repository facts; a release is not one." Three ways out: (a) leave it —
   the store may hold the edges under whatever `from_id` an implementation likes; (b) add a `release` node
   kind, id `<repo>/release:<version>`, `PRAGMA user_version` **8**; (c) drop the edges entirely and let G14
   read the floor list directly. "Recommendation: (c), then (b) if a query ever needs it. This is
   owner-level because (b) changes PB §6.2." — **blocks the store's `protects` shape; does not block the
   dump** (R45/R68 stand either way).
2. **DM §15 OPEN-2 · Whether `tree: unverifiable(git-version)` should exist.** "It is the only value in the
   dump that reads the local environment (§7.2.1)." Alternatives: make the version mismatch a hard G9
   finding (`unattested`), or make the tree check conditional and record nothing. "Recommendation: keep it,
   and add the git version to `spine stats`. Owner-level because it is a G9 semantics question."
3. **DM §15 OPEN-3 · Whether `--dump` should have a mode that includes in-flight elements.** "If such a mode
   ships it is a **second artifact**, not a flag on this one: it needs its own version, its own exclusion
   set, and — critically — it must not be what G10 diffs. Recommendation: not in v1; if it comes, name it
   something other than `--dump`." — bears directly on `spine context` / `spine review` renderings.
4. **DM §14 D2 · OPEN defect — a trailer citation cannot name the second of two identical trailers.** "A
   signerless landing carries two `Spine-Review` lines; a squash landing copies many `Spine-Frozen` and
   `Spine-Test` lines. Every element derived from any of them cites the same string." Recommended fix
   (**not adopted**): `git:<sha>:trailer:<Name>#<n>`, `n` the 1-based occurrence. Latent for dumps, active
   for any tool that reads a rendering.
5. **DM §14 D3 · OPEN defect — the provenance grammar is not unambiguously parseable.**
   "`git:<sha>:<path>:<line>` collides with `git:<sha>:msg:L<n>`, `git:<sha>:trailer:<Name>` and
   `git:<sha>:patch-id` when a path is called `msg`, `trailer` or `patch-id`; and `<path>:<line>` collides
   with the whole `git:` family when a path begins `git:`. A last-colon rule plus an oid-length test
   disambiguates in practice, but it is a heuristic and PB §6.1 offers none."
6. **DM §14 D4 · OPEN defect — G10's exclusion clause is four adjectives.** Recommended: "the clause cites
   this document instead of enumerating."
7. **DM §14 D5 · OPEN defect — PB §6.2's node-id examples contradict its own repo-scoping rule.**
   Recommended: prefix the examples. (Implementers follow DM §5.2/§13.3 — R20/R21 — meanwhile.)
8. **DM §14 D6 · OPEN defect — the schema has no edge kind that answers PB §6.4's ADR query.** "PB §6.4
   promises a resuming agent *any ADRs touching the same code units*. PB §6.2 gives `adr` nodes and connects
   them only through `supersedes` / `superseded_by`; there is no edge from an ADR to a `code_unit`, and none
   is derivable — an ADR is prose. As it stands an `adr` node is isolated (as in §12.2), the query cannot be
   answered, and the promise is unbacked." — **this is the one OPEN that blocks a feature in this sheet's
   own scope (R74).**
9. **Unfixed direction ·** the `from`/`to` of `supersedes`, `superseded_by` and `exercises` are stated
   nowhere, although DM §13.4 claims "§5.3's paragraph fixes all fifteen" (see Contradictions). No value may
   be invented; PB §6.6's "the indexer emits `superseded_by`" is the only hint.
10. **MF §14 D11 · OPEN** — "Two keys under one principal are representable in the keyring and not in the
    graph"; the fix (refuse in G16's keyring lint) is recorded as resolved in MF §9 R7 but the defect entry
    is still marked OPEN against PB §7.2.

---

## Contradictions found

1. **PB §6.2's node-id examples vs PB §6.2's own repo-scoping rule.** The `id` column lists
   `"code:src/billing/" | "cs:abc123f" | "approval:5c9e…" | "signer:alice@example.com" | "ADR-007" |
   "constitution:v3"` — none carrying the `myrepo/` prefix the same paragraph mandates. **Resolution:**
   DM §5.2/§13.3 — every node id is `<repo>/` + local id; the examples are abbreviations. Filed as DM §14 D5
   (OPEN).
2. **PB §6.2's `approval:5c9e…` vs DM §5.2.1's line hash.** PB §4.3 calls the approve line's `freeze=` "a
   non-git digest, used to name the approval elsewhere", and PB §6.2's example id matches the vector's
   `freeze=` prefix `5c9e…`. **Resolution:** DM §5.2.1/§13.6 rejects the freeze reading (it is total over
   only one of six `event` values); the id is SHA-256 over the trailer line's exact bytes, and `freeze=` is
   carried as an attr. In DM §12.2 the approve node's id is `b635…`, not `5c9e…`.
3. **PB §11 States vs PB §6.2 schema on where `unattested`/`resealed`/`orphan` live.** PB §11 lists
   "post-landing `reverted`, `superseded`, `orphan`, `unattested`, `resealed`" together as if they were one
   status vocabulary; PB §6.2's schema puts `unattested` and `resealed` in the **changeset** attrs and
   neither in the intent's. **Resolution:** DM §7.3/§13.8 — `intent.status` ∈ {`merged`, `withdrawn`,
   `reverted`, `superseded`}; the other three are changeset facts. (PB §11 nominally wins, but §11's list is
   of *states*, not of `intent.status` values, so the schema is decisive.)
4. **PB §6.3 G10's exclusion clause vs DM §8.5.** PB's four adjectives ("provisional (in-flight) elements,
   † states …, volatile test results and worktree-only files excluded") do not reach
   `verified_by.introduced_by`, shipped-floor `protects` edges, or `exercises`. DM §8.5 adds all three and
   calls them "a change to PB §6.3's G10 clause, not merely a reading of it" (DM §14 D4, OPEN). An
   implementer follows DM.
5. **PB §6.3 G10's trust-root clause (as filed) vs DM §3.1/§11.** DM §14 D1 recorded that G10 wrote the trust
   root into the clone only, so the scratch clone `S` had none and every landing failed G10. **CLOSED in
   PB v0.19**: the row now reads "written into **both** sides' `spine.trustRoot`". Both documents now agree;
   the header's `trust_root` member keeps a recurrence legible as a line-1 diff.
6. **PB §6.3's illustrative G2 SQL vs ID §6.6's `code_unit` model.** PB's query compares `modifies` node ids
   against `declares` node ids and PB's own note concedes "`spine_match` is the touchpoint matcher, not
   equality … `NOT IN` over node ids never matches and would fire on every landing." ID §13 D4 files the SQL
   as unimplementable: "PB §6.3's illustrative SQL cannot implement this — it compares node ids with
   `NOT IN`, which is byte equality between `code:src/billing/tax.py` and `code:src/billing/`."
   **Resolution:** a touchpoint pattern is one `code_unit` node, never expanded; G2 is a match predicate
   (ID §7.1), not set membership.
7. **DM §13.4 claims all fifteen edge directions are fixed; DM §5.3's paragraph fixes twelve.** The paragraph
   names `verified_by`, `approves`, `signed_by`, `attested_by`, `freezes`, `protects`, `implements`,
   `modifies`, `built_under`, `has_ac`, `declares`, `reverts` — and is silent on `supersedes`,
   `superseded_by` and `exercises`. Recorded as OPEN item 9 above.
8. **DM §7.2 says the landing changeset's attrs are "all from the seal"; PB §11 puts three of them in
   sibling trailers.** `lane` is `Spine-Lane`'s payload, `strategy` is `Spine-Strategy`'s, and `event` is
   `Spine-Event`'s (PB §11 trailer table; PB §6.2's derivation row for changeset `event` says exactly that:
   "landing envelopes carrying `Spine-Event: land | withdraw | reseal`"). The seal itself carries `base`,
   `head`, `tree`, `report`, `tool`, `git`, `mode`, `threat`, `profile`, `signer` and its first identity
   field. Read DM's "from the seal" as "from the envelope's sealed lines"; PB §11 wins on which trailer
   carries which field.
9. **`changeset.tool_version` vs the seal's `tool=`.** PB §11's seal carries
   `tool=<version>+sha256:<dist_hash>`; the DM §12.2 vector records `"tool_version":"1.4.0"` — the version
   half only, with the `dist_hash` dropped. Neither document states the split rule; the vector is the only
   evidence and is what an implementer must match.
10. **PB §6.4's ADR promise vs the schema.** See OPEN item 8 (DM §14 D6): the promise "any ADRs touching the
    same code units" has no edge and no derivation. Not repairable by an implementer without an owner
    decision.
11. **PB §6.2's `test` id example `test:vitest:billing/invoice.test.ts > AC1 totals` is unprefixed and PB
    §6.2's `cs:abc123f` is abbreviated**, against DM §5.2's "Oids are full, lowercase, never abbreviated"
    and the repo prefix. Same class as contradiction 1; DM wins ("PB's `cs:abc123f` is display").
