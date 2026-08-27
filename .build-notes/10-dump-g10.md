# `spine index --dump` and G10 — normative requirement sheet

Concern: the canonical dump byte stream, its total order, its exclusion set, its determinism rules, and gate G10 (Reconstruction) which byte-compares two of them.

Citation convention used throughout, matching the corpus's own: **`PB §n`** = `PLAYBOOK.md`; **`DM §n`** = `docs/spec/dump.md`; **`GR §n`** = `docs/spec/gate-report.md`; **`RF`** = `result-file.md`; **`MF`** = `manifest.md`; **`IR`** = `import-resolver.md`; **`EV`** = `envelope-vectors.md`. DM's own §-numbers and PB's collide (PB §6.2 is the SQL schema, DM §6.2 is the node sort key), so every citation below says which document.

**Precedence rule applied (from `docs/spec/README.md`, "Where prose here and the playbook's §11 disagree, §11 still wins — report it as a defect in one of them"):** PB §11 wins over DM; elsewhere DM is normative and resolves PB. Disagreements are recorded in *Contradictions found*.

---

## Sources read

| File | Lines | What |
|---|---|---|
| `/Users/thettwe/Works/spine-kit/docs/spec/dump.md` | **1–914 (entire file, every line)** | §1 what the artifact is; §2 serialization (2.1 scheme, 2.2 framing, 2.3 value profile, 2.4 `esc`, 2.5 digest); §3 identity/versioning (3.1 header, 3.2 unknown version, 3.3 two version facts, 3.4 no release in dump); §4 closed input set (4.1 one ref, 4.2 trunk resolution, 4.3 never-inputs, 4.4 exit codes); §5 records (5.1 node, 5.2 id grammar, 5.2.1 approval id, 5.3 edge, 5.4 `src`, 5.5 multiply-derived); §6 total order (6.1 sections, 6.2 node key, 6.3 edge key, 6.4 comparison, 6.5 totality); §7 attrs (7.1 profile, 7.2 per kind, 7.2.1 `tree`/`git_version`, 7.3 presence + status domain); §8 exclusion set (8.1 generating rule, 8.2 by node kind, 8.3 by edge kind, 8.4 by attr, 8.5 three added exclusions, 8.6 † states, 8.7 worktree); §9 empty dump; §10 twelve+one determinism rules; §11 G10's comparison; §12 vectors (12.1 repo, 12.2 the 62-line dump, 12.3 digest, 12.4 ordering vector, 12.5 empty-dump vector); §13 resolved ambiguities (13.1–13.11); §14 playbook defects D1–D6; §15 OPEN-1/2/3; §16 out of scope; §17 conformance checklist items 1–24 |
| `/Users/thettwe/Works/spine-kit/PLAYBOOK.md` | 419 (§5.4 step 5), 475 (§5.5 `Spine-Gates` count), 530–545 (§5.5 state table incl. the `checked†` → `reconstruction-failed` row at line 540), 560–640 (§6.1 iron rule + provenance law, §6.2 schema + derivation table), 671 (§6.3 **G10 — Reconstruction** row, in full), 690–698 (§6.5), 888–897 (§7.6 break-glass), 904 (§8 failure-mode table), 983–1040 (**§11 Vocabulary**, in full), 1085 (§12 changes — the G10/D1 fix), 1115 | |
| `/Users/thettwe/Works/spine-kit/docs/spec/gate-report.md` | 55–80 (§2.3 `esc`, quoted verbatim below), 630–700 (§6.1–§6.2 wire order), 947–960 (§8.3 minimal canonicalizer vector) | normative dependency |
| `/Users/thettwe/Works/spine-kit/docs/spec/README.md` | 1–90 (status table, six settled owner decisions, published-digest table incl. the DM §12.3 row) | |

**Vectors independently re-verified in this session** (see *Worked examples*): all three DM digests and all three `approval` local ids reproduce exactly.

---

## Data model

### Record kinds (closed set)

Exactly three, discriminated by member `t`: `"header"`, `"node"`, `"edge"`. *"No other value of `t` is legal, and an unknown member name inside a known record kind is not tolerated"* (DM §2.1).

### Header record — one, always line 1 (DM §3.1)

Verbatim template:

```json
{"dump_version":1,"head":"<oid>","object_format":"sha1","repo":"<esc>","schema_version":7,"t":"header","trunk":"<esc>","trust_root":"<oid>"}
```

| Member | Type | Domain | Default | Required |
|---|---|---|---|---|
| `dump_version` | integer | `1` (this document defines version 1) | — | always (DM §3.1) |
| `object_format` | string | `"sha1"` \| `"sha256"` | `sha1` when `extensions.objectFormat` absent | always. **The indexed repository's own** format, not the manifest's (DM §3.1, §13.11). Fixes every oid at 40 or 64 lowercase hex |
| `repo` | string (bytes → `esc`) | `^[A-Za-z0-9._-]+$`, 1…64 bytes | — | always. The manifest's `repo`; prefix of every node id (DM §3.1, §5.2; bound owned by MF §3.1) |
| `schema_version` | integer | `7` | — | always. PB §6.2's `PRAGMA user_version = 7` (DM §3.1, §3.3) |
| `t` | string | `"header"` | — | always |
| `trunk` | string (bytes → `esc`) | the resolved trunk **branch name**, *not* a full refname | — | always (DM §3.1, §4.2) |
| `head` | string | full oid at `object_format` length | — | **iff `refs/heads/<trunk>` resolves.** Absent ⇒ the dump is empty (DM §3.1, §9) |
| `trust_root` | string | full oid | — | **iff `spine.trustRoot` is configured** (DM §3.1) |

Members `cli.version`, `cli.dist_hash`, or any other name for the producing binary are **MUST NOT** (DM §3.4).

### Node record (DM §5.1)

```json
{"attrs":{…},"id":"<esc>","kind":"<kind>","src":"<esc>","t":"node"}
```

| Member | Type | Presence | Value |
|---|---|---|---|
| `attrs` | object | always | `{}` when the kind has none; **never absent** (DM §5.1, §7.1) |
| `id` | string, bytes | always | DM §5.2 grammar. Unique across the whole node section (PB §6.2 `id TEXT PRIMARY KEY`) |
| `kind` | string | always | closed set: `ac`, `adr`, `approval`, `changeset`, `code_unit`, `constitution`, `intent`, `signer`, `test` (DM §5.1; = PB §6.2's nine) |
| `src` | string, bytes | always | DM §5.4 grammar |
| `t` | string | always | `"node"` |

### Node id grammar (DM §5.2)

Every node id is `<repo>` + `/` + a per-kind local id.

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

### Edge record (DM §5.3)

```json
{"attrs":{…},"from":"<esc>","kind":"<kind>","src":"<esc>","t":"edge","to":"<esc>"}
```

`kind` closed set (fifteen, = PB §6.2's): `approves`, `attested_by`, `built_under`, `declares`, `exercises`, `freezes`, `has_ac`, `implements`, `modifies`, `protects`, `reverts`, `signed_by`, `supersedes`, `superseded_by`, `verified_by`. **`exercises` is never emitted in v1** (DM §5.3, §8.3).

### Edge directions — PB leaves all fifteen implicit; DM §5.3 fixes them

| kind | from → to |
|---|---|
| `verified_by` | **test → ac** (PB §6.3 G5 fails on *"a `verified_by` edge to a nonexistent AC (typo'd pragma)"*, so the AC is `to_id`) |
| `approves` | approval → intent, **or** approval → `cs:<L>` for a line carrying no id (PB §6.2) |
| `signed_by` | approval → signer |
| `attested_by` | the landing changeset → the seal's signer |
| `freezes` | approval → code_unit (with `oid`) **or** approval → test |
| `protects` | constitution → code_unit |
| `implements` | changeset → intent |
| `modifies` | changeset → code_unit |
| `built_under` | intent → constitution |
| `has_ac` | intent → ac |
| `declares` | intent → code_unit |
| `reverts` | the reverting landing → the reverted landing |
| `supersedes` / `superseded_by` | (per PB §6.2 derivation: ADR/constitution headers and the `Spine-Supersedes` trailer) |
| `exercises` | never emitted (v1) |

### `src` — the provenance grammar (DM §5.4, PB §6.1)

| Production | Shape | Used for |
|---|---|---|
| file line | `<path>:<line>` | **never emitted by a dump** (DM §8.7); exists in PB §6.1, listed for completeness |
| commit | `git:<sha>` | a whole commit — `modifies` from that commit's diff, a member changeset |
| message line | `git:<sha>:msg:L<n>` | a line of the envelope's fenced intent bytes — intent, ac, `has_ac`, `declares`, `built_under` |
| trailer | `git:<sha>:trailer:<Name>` | a signed line — approval nodes, `approves`, `signed_by`, `attested_by`, `freezes`, the landing changeset |
| patch id | `git:<sha>:patch-id` | `reverts` |
| file line at a commit | `git:<sha>:<path>:<line>` | test nodes, `verified_by`, signer nodes, `protects` from `C-A2`, adr and constitution nodes |
| shipped floor | `spine:<version>:floor` | the release's floor list — **never emitted by a dump** (DM §8.3) |

*"`<sha>` is a full oid at `object_format`'s length. `<line>` and `<n>` are decimal integers ≥ 1 with no leading zero. `<path>` is `esc(path bytes)`. `<Name>` is a trailer name."* (DM §5.4)

### `attrs` value profile (DM §7.1)

An attr value is a **string**, a non-negative **integer** in `[0, 2^53 − 1]`, a **boolean**, or an **array of strings**. Never an object, never `null`, never a number outside that range. Attr name matches `^[a-z][a-z0-9_]*$`, from the closed set of DM §7.2.

### Node attrs, by kind (DM §7.2)

Kinds PB §6.2 gives no attrs for have **none**: `ac`, `adr`, `code_unit`, `constitution` nodes carry `{}` (DM §7.2, §13.9).

**`intent`**

| attr | type | presence | value |
|---|---|---|---|
| `status` | string | always | `merged` \| `withdrawn` \| `reverted` \| `superseded` — **and nothing else** (DM §7.3) |
| `owner` | string, bytes | iff the doc has an `Owner:` field | the field's value, `esc`-encoded — a hint, never authority (PB §3.1) |
| `title` | string, bytes | always | the `# INT-042: <title>` heading's title, read from the sealed intent inside the landing commit's message — **never from that commit's subject line** |
| `template` | string | always | the `Template:` value — variant *and* version, e.g. `intent@2` (PB §3.4) |
| `blob` | oid | always | the signed intent blob |
| `signer` | string, bytes | iff a `Spine-Signoff` is copied into the landing | the sign-off's principal |
| `reopen_count` | integer | always | copied `Spine-Reopen` lines |
| `late_reopen_count` | integer | always | of those, the ones after the binding approval |
| `landing` | oid | always | `L` |
| `base` | oid | always | the seal's `base=` |

**`changeset`**

| attr | type | presence | value |
|---|---|---|---|
| `landing` | boolean | always | `true` for a sealed trunk commit, `false` for a member |
| `lane`, `event`, `strategy`, `base`, `head`, `tree`, `seal_principal`, `seal_verified`, `report_sha256`, `threat`, `profile`, `tool_version`, `git_version`, `mode`, `unattested`, `resealed` | per DM §7.2 / PB §6.2 | **iff `landing` is `true`** | all from the seal. *"A member changeset carries `{"landing":false}` and nothing else: it has no seal, and every one of those fields is a seal field."* (DM §7.2) |

- `git_version` is **the seal's `git=`**, never the indexing binary's own `git --version` (DM §7.2.1).
- `tree` is `L`'s tree oid — normally the seal's `tree=` — **or** one of PB §6.3 G9's two sentinels, both legal: `unverifiable(squash)` (deterministic) and `unverifiable(git-version)` (**the single value in a dump that is not a pure function of DM §4.1's inputs**) (DM §7.2.1, §13.7).

**`approval`**

| attr | type | presence | value |
|---|---|---|---|
| `event` | string | always | `signoff` \| `approve` \| `review` \| `reopen` \| `withdraw` \| `upgrade` |
| `role` | string | always | `signer` \| `reviewer` \| `pipeline` — **the namespace the signature verified under**, never a claim in the trailer. *"A v1 approve line signed under `spine-review@v1` is `reviewer`."* (DM §7.2, PB §4.3/§7.2) |
| `principal` | string, bytes | always | the `signer=` / `reviewer=` value |
| `verified` | boolean | always | the `-Sig` verified against the keyring at the seal's `base=` |
| `blob` | oid | iff the line carries `blob=` or `intent=` | |
| `base`, `head`, `tree` | oid | iff the line carries them | |
| `class` | string | iff `event` is `review` | `tripwire` \| `protected` \| `break-glass` |
| `rounds`, `total_rounds`, `reopens` | integer | iff the line carries them | |
| `red` | string | iff `event` is `approve` | `"k/n"` |
| `freeze` | string | iff `event` is `approve` | `sha256:<hex>` |
| `wires` | array of strings | iff `event` is `review` | the `wires=` tokens **in the line's order**. Not re-sorted (see Byte-level fixities) |
| `voided_by` | oid | iff a copied `Spine-Reopen` voids this approval | the commit carrying that reopen |
| `void_reason` | string, bytes | iff `voided_by` is present | that line's `reason=` |

**`signer`**

| attr | type | presence | value |
|---|---|---|---|
| `roles` | array of strings | always | namespaces this key is listed under, **ascending by bytes**: a subset of `spine-review@v1`, `spine-seal@v1`, `spine-signoff@v1` |
| `fingerprint` | string | always | as `ssh-keygen -lf` produces it (`SHA256:<base64>`) |
| `valid_from` | oid | always | trunk commit at which the key first appears in `.spine/allowed_signers` |
| `valid_to` | oid | iff the key has been removed | trunk commit at which it stopped appearing |

**`test`** — `{}` always. *"`result_at` is the kind's only attr and §8.4 excludes it."* (DM §7.2)

### Edge attrs (DM §7.2)

Kinds with none (carry `{}`): `approves`, `attested_by`, `built_under`, `has_ac`, `modifies`, `signed_by`, `supersedes`, `superseded_by`.

| kind | attr | type | presence | value |
|---|---|---|---|---|
| `declares` | `polarity` | string | always | `expected` \| `forbidden` |
| `implements` | `role` | string | always | `landing` \| `member` |
| | `provisional` | boolean | always | **`false` in every dumped record.** *"A `true` is a conformance failure, and a cheap one to test for."* |
| | `verified` | boolean | always | membership verified by G9's walk |
| `verified_by` | `attributed` | boolean | always | the pragma is in a blob the binding approval froze |
| `freezes` | `oid` | oid | iff `to` is a `code_unit` | the frozen blob. **A `freezes` edge to a `test` carries `{}`** |
| `protects` | `floor` | boolean | always | **`false` in every dumped record** (shipped floor excluded) |
| `reverts` | `partial` | boolean | always | hunks missing inside `L`'s paths |

Excluded attrs: `test.result_at {tree, base, passed}` and `verified_by.introduced_by` (DM §8.4).

---

## Algorithm

Numbered, ordered. Requirement ids `R<n>` are the MUST/MUST NOT/REFUSE items an implementer must satisfy; there are **74**.

### A. Resolve inputs

1. **R1 (MUST)** Resolve the trunk branch name in exactly this order (DM §4.2): (1) an explicit `--trunk <name>`, when the CLI offers one; (2) `params.trunk` in `.spine/manifest.json` in the **tree** of the commit `HEAD` resolves to; (3) `params.trunk` in the manifest of the **newest first-parent ancestor of `HEAD` whose tree carries one** (the `--uninstall`-to-re-`init` range case, PB §6.7); (4) none.
2. **R2 (MUST)** Steps 2 and 3 read a **tree**, never the working directory, *"so a bare repository resolves identically to a checked-out one"* (DM §4.2).
3. **R3 (REFUSE)** Trunk resolution reaching step 4 ⇒ status `not-installed`, **exit 2**, nothing written to stdout (DM §4.2 step 4, §4.4).
4. **R4 (MUST)** `head` := whatever `refs/heads/<trunk>` resolves to in this repository; **absent** when that ref does not exist (DM §4.2, §3.1).
5. **R5 (MUST)** `--dump` implies `--fresh`: the projection is computed from a graph built **in this process from git objects alone**. No persisted, fetched or restored store. Whether the store file is also written is unspecified and *"cannot affect one byte"* (DM §4.3, §10 rule 3; PB §7.4 rule 3).
6. **R6 (MUST NOT)** Read as input: any ref but `refs/heads/<trunk>` (this **includes** `refs/heads/intent/*`, `refs/remotes/*/*`, tags, `refs/notes/*`, every provider ref); `refs/notes/spine`; the working tree, the index, `git status`; the collector's result file; a coverage report; any wall clock; the environment; repository/user/system git config beyond `extensions.objectFormat` and `spine.trustRoot` (DM §4.3).
7. **R7 (MUST)** *"A dump is a function of exactly four things: the trunk tip's oid, the git objects reachable from it, the trust root, and the pinned release. Nothing else may influence one byte."* (DM §4.1, verbatim)

### B. Derive the element set (apply the exclusion set)

8. **R8 (MUST)** Apply the generating rule verbatim (DM §8.1): *"A graph element is in the dump if and only if it is derived from git objects reachable from the trunk tip. An element derived from anything else — an intent branch, the collector's result file, a coverage report, the binary's own floor list, or a heuristic over the objects rather than the objects — is excluded."*
9. **R9 (MUST)** Node-kind inclusion conditions exactly as DM §8.2 (reproduced in *Exclusion set*, below).
10. **R10 (MUST)** Edge-kind inclusion exactly as DM §8.3, including **`protects` partly** and **`exercises` never**.
11. **R11 (MUST NOT)** Emit `test.result_at` or `verified_by.introduced_by` (DM §8.4).
12. **R12 (MUST NOT)** Emit any element derived from the working tree, the index, or an untracked file (DM §8.7).
13. **R13 (MUST NOT)** Emit an `intent` node whose `status` is a † state (`checked†`, `base-moved†`) or any in-flight status (`draft` … ) — vacuous under R9 but a conformance check with teeth (DM §8.6, §7.3, §17 item 15).

### C. Fix `src` per element

14. **R14 (MUST)** Every node and every edge carries `src` in PB §6.1's grammar, verbatim (DM §5.4; PB §6.1 *"every node and edge must cite its source. An edge that cannot say where it came from does not exist."*).
15. **R15 (REFUSE)** A derivation producing a `src` outside that grammar ⇒ `provenance-invalid`, **exit 3** (DM §4.4, §5.4).
16. **R16 (MUST)** *"When a derivation produces the same element from more than one citation, the dump emits one record whose `src` is the minimum, under §6.4's ordering, of those citations."* (DM §5.5, verbatim)
17. **R17 (MUST)** Applied to edges: records equal in `from`, `to`, `kind` **and** `attrs` collapse to one with the minimum `src`; records that differ in `attrs` are two edges and **both** are emitted (DM §5.5, §6.3).

### D. Validate

18. **R18 (REFUSE)** A node id outside DM §5.2's grammar ⇒ `id-out-of-grammar`, **exit 3** (DM §4.4). This includes a manifest whose `repo` does not match `^[A-Za-z0-9._-]+$` (DM §5.2; MF §3.1 additionally bounds it to 1…64 bytes and refuses with `repo-out-of-grammar`).
19. **R19 (REFUSE)** An attr value outside DM §2.3 or §7.2 — *"a float, a `null`, a nested object, an unknown name"* ⇒ `attrs-out-of-profile`, **exit 3** (DM §4.4, §7.1).
20. **R20 (MUST)** Every `from` and every `to` names a node record **in the same dump**. *"No dangling edges."* A well-formed id naming no node is an indexer defect this format cannot detect and **G5** must (DM §5.3; PB §6.1 *"in a derived graph, dangling edges are the linter"*).

### E. Serialize each record

21. **R21 (MUST)** Serialize each record as **RFC 8785 JCS**, restricted by DM §2.3's value profile and DM §2.4's `esc` (DM §2.1).
22. **R22 (MUST)** Under this profile JCS reduces to, verbatim (DM §2.3): *"sort each object's members by member-name bytes ascending; emit with no whitespace; emit integers in plain decimal; emit strings with JSON's minimal escaping (`"` → `\"`, `\` → `\\`, nothing else can occur); output UTF-8, which is here also ASCII."*
23. **R23 (MUST)** Depth is **exactly two**: a record object whose `attrs` member is an object of scalars and string arrays. **No attr value is an object** (DM §2.3).
24. **R24 (MUST NOT)** Emit `null` anywhere; emit a float anywhere; emit a duplicate member name; emit an array whose elements are not strings (DM §2.3, §7.3, §10 rule 6).
25. **R25 (MUST)** `attrs` is **always present**, `{}` when empty. *"`{}` is a value; an omitted `attrs` member is not a legal record."* (DM §7.1, §13.10)
26. **R26 (MUST)** Apply `esc` (GR §2.3) to: *"every node `id`, every `from` and `to`, every `src`, the header's `repo` and `trunk`, and every attr whose §7.2 row says *bytes*"* (DM §2.4).
27. **R27 (MUST NOT)** Apply the `tok` variant of GR §6.2 anywhere in a dump (DM §2.4).
28. **R28 (MUST NOT)** Normalize anything: *"No NFC, no NFD, no case folding, no separator rewriting, no path cleanup."* Where a gate itself casefolds (G14 before floor comparison, PB §7.3), *"the dump records the path **as the tree spells it**, never the casefolded form."* (DM §2.4)
29. **R29 (MUST)** *"A `code_unit` id is built from the repo-relative, `/`-separated path exactly as git stores it in the tree entry."* Paths are the tree's bytes, never the filesystem's (DM §2.4).
30. **R30 (MUST)** Every oid is lowercase hex at `object_format`'s **full** length — 40 or 64. *"Never abbreviated, never uppercase, never prefixed."* PB's `cs:abc123f` is display only (DM §5.2, §10 rule 9).

### F. Order

31. **R31 (MUST)** Header first (line 1, by framing, not by sort), then the **node section**, then the **edge section** (DM §2.2, §6.1).
32. **R32 (MUST)** Node key, verbatim (DM §6.2), with `‖` concatenation and `NUL` = byte `0x00`:
    `key_node(r) = r.kind ‖ NUL ‖ r.id ‖ NUL ‖ canonical(r.attrs) ‖ NUL ‖ r.src`
    `canonical(r.attrs)` is the JCS serialization of the `attrs` object — the same bytes the record carries. `kind` before `id` is PB §6.3's order and is preserved *"even though `id` alone is unique"*.
33. **R33 (MUST)** Edge key, verbatim (DM §6.3):
    `key_edge(r) = r.from ‖ NUL ‖ r.to ‖ NUL ‖ r.kind ‖ NUL ‖ canonical(r.attrs) ‖ NUL ‖ r.src`
    Sorted ascending **after** the collapse of DM §5.5.
34. **R34 (MUST)** Comparison is *"ascending over those bytes, unsigned, with the shorter string first when one is a prefix of the other"* (DM §6.4).
35. **R35 (MUST)** **The order is over the `esc`-encoded bytes, not the raw path bytes** (DM §6.4). Field-by-field comparison is permitted; results are identical.
36. **R36 (MUST NOT)** Sort by the serialized line. *"Sort by the key of §6.2 and §6.3, never by the line."* A dump is **not** a fixpoint of `LC_ALL=C sort` (DM §6.5, §10 rule 5, §17 item 13).

### G. Frame and emit

37. **R37 (MUST)** *"The byte stream is a sequence of **lines**, each terminated by exactly one `0x0A` (LF). The final line is terminated too, so the stream ends with `0x0A`. No CR anywhere, no BOM, no blank lines, no comments, no trailing blank line."* (DM §2.2 rule 1, verbatim)
38. **R38 (MUST)** Line 1 is the header record, present in **every** dump including an empty one (DM §2.2 rule 2, §9).
39. **R39 (MUST NOT)** Emit a footer, a count, or a digest of the dump itself. *"Nothing follows."* (DM §2.2 rule 5, §10 rule 11)
40. **R40 (MUST)** *"`spine index --dump` writes exactly these bytes to **stdout** and nothing else to stdout. Diagnostics, warnings and progress go to stderr, which is not part of the artifact."* (DM §2.2)
41. **R41 (MUST)** Exit **0** with status `ok` when the dump is on stdout (DM §4.4).

### H. G10 — the gate (DM §11, PB §6.3 G10 row, PB §5.4 step 5)

Runs **before** the CAS, on the candidate landing `L` built at PB §5.4 step 4, and **never by moving the runner's own refs**.

42. **R42 (MUST)** `L` is pushed into the scratch clone `S` as `refs/heads/<trunk>` with the intent ref deleted, so `S` holds the post-CAS ref set — PB §5.4 step 5 spells the command: `git push S L:refs/heads/<trunk> :refs/heads/intent/INT-042`.
43. **R43 (MUST)** `S` is cloned with `git clone --no-local --no-hardlinks file://S` into a temp dir, with `GIT_CONFIG_GLOBAL=/dev/null` and **no network**, **default refs only** — *"no notes, no custom refs, no provider metadata"* (PB §6.3 G10; DM §11 step 2).
44. **R44 (MUST)** *"the runner's pinned trust root is written into **both** sides' `spine.trustRoot` — `S`'s as well as the clone's, since `S` is itself a fresh clone and carries no local config, and a side without a pin would trust on first use or refuse, either way diverging from the other on every landing (TOFU is for humans, never for G10)"* (PB §6.3 G10 row, verbatim). *"the two sides are configured identically from the runner's pin and differ in nothing else"*.
45. **R45 (MUST)** `spine index --fresh --dump` runs in each, producing byte streams `D_S` and `D_C` (DM §11 step 4).
46. **R46 (MUST)** *"The comparison is `D_S == D_C` as byte strings."* Equal digests (DM §2.5) are an **equivalent implementation**, permitted *"precisely because SHA-256 collision resistance makes the two equivalent"*. **Nothing parses either stream** (DM §11 step 5, §2.5).
47. **R47 (REFUSE)** Unequal ⇒ *"the push is refused, `L` is discarded, the run ends `reconstruction-failed` without a retry and without consuming a `C-M3` re-verification, and the run's own report is the only record"* (DM §11 step 6; PB §5.4 step 5; PB §5.5 state table line 540: *"reconstruction-failed — discarded, reported, never re-queued: a deterministic failure re-runs identically"*).
48. **R48 (MUST)** G10 runs *"on every landing without exception"* (PB §5.4 step 5). There is **no deferred mode**: *"There is no deferred mode. v0.9 offered one for repos too large to clone and index per landing, and the v0.9 review showed why it could not work"* (PB §6.3 G10).
49. **R49 (MUST NOT)** Put G10's result into `Spine-Gates`. *"G10's own result never enters `Spine-Gates`: `L` exists by then and its seal covers its message (§11)."* (PB §5.4 step 5; PB §11 `Spine-Gates` row: *"every gate that ran, never G10 (it runs after the seal)"*; GR §5.6.2 — G10 never appears in a version-1 report; PB §5.5 line 475: `Spine-Gates` lists **fourteen** entries and G10 is not among them.)
50. **R50 (MUST NOT)** Allow break-glass to bypass G10. PB §7.6: *"never G5, G9, G10, G11, and never Authority"*; PB §11 Gates: *"Break-glass may bypass G1, G2, G3, G4, G6, G7, G8, G12 only."* DM §1: *"break-glass cannot bypass it — G10 is not in PB §7.6's list and could not be, since `L` already exists and its seal covers its own message."*
51. **R51 (MUST)** *"One binary per comparison.* G10 indexes and dumps both sides with the same process's release."* (DM §10 rule 13)
52. **R52 (REFUSE)** G10 only: two dumps with different `dump_version` or `schema_version` offered for comparison ⇒ `dump-version-skew`, **exit 3**, *"rather than comparing"* (DM §3.2, §4.4).
53. **R53 (MUST — testable properties of the format)** *"**No false positive.** Two indexings of the same objects by the same release produce identical bytes."* and *"**A legible failure.** Because the streams are line-sorted JSONL, `diff` names the record."* (DM §11)

### I. Versioning obligations

54. **R54 (MUST)** *"a release that changes the projection **must** bump `dump_version`, even for a change it believes is a bug fix."* A silent projection change is *"a fleet-wide `reconstruction-failed` on the first landing after a rolling upgrade, and the report will name the graph, not the release."* (DM §3.4)
55. **R55 (MUST)** *"two releases carrying the same `dump_version` and `schema_version` **must** produce identical bytes over identical objects."* (DM §3.4)
56. **R56 (MUST)** A binary keeps a serializer for **every** `dump_version` it has ever shipped — the same promise PB §6.7 makes for template and envelope versions (DM §3.2).
57. **R57 (REFUSE)** An external reader meeting an unknown `dump_version`, or an unknown member name inside a version it knows, **refuses**. *"The schema is closed: forward compatibility is bought with a version bump, not with tolerance"* (DM §3.2). Rationale kept verbatim from GR §3.2: *"a tolerant reader and a strict one produce different bytes over the same document, and the whole artifact is compared by bytes."*

### J. Determinism rules, collected (DM §10) — restated as requirements

58. **R58 (MUST NOT)** **No wall clock.** No record holds a time, a duration, a date or anything derived from one. Committer dates may be *read* by a derivation (G3's staleness comparison reads one); **none is a value**. `params.timeout` never appears (DM §10 rule 1, §4.3).
59. **R59 (MUST NOT)** **No environment.** No hostname, runner id, user, locale, process id, temp path, or path outside the repository (DM §10 rule 2) — except the single documented `unverifiable(git-version)` sentinel (DM §7.2.1).
60. **R60 (MUST NOT)** No persisted/fetched/restored store; no note read as a source; *"no dump written anywhere a later run could find it"* (DM §10 rule 3).
61. **R61 (MUST)** Key ordering inside a record is JCS's, ascending by member-name bytes — *"Never insertion order, never a hand-written order"* (DM §10 rule 4).
62. **R62 (MUST)** Numbers are integers in `[0, 2^53 − 1]`, plain decimal, no leading zero, no sign, no fraction, no exponent, no `-0` (DM §10 rule 7, §2.3).
63. **R63 (MUST)** Non-git digests are `sha256:` + 64 lowercase hex — *"except the `approval` local id, which is bare hex inside an id (§5.2.1) — PB §11's hash policy governs values, and an id is not a value it governs"* (DM §10 rule 10).
64. **R64 (MUST NOT)** Self-reference: *"A dump never contains its own digest, its own length, its own record count, or the release that produced it"* (DM §10 rule 11).
65. **R65 (MUST)** **Git plumbing is pinned by the release.** *"Every invocation the derivation makes — `diff`, `rev-list`, `merge-tree`, `patch-id`, `ls-tree` — runs with its diff algorithm, rename and copy detection, and every other output-affecting option fixed by the release and never read from repository, user or system config."* *"A repository that sets `diff.algorithm` must not thereby change its own dump."* (DM §10 rule 12, §4.3)

### K. The empty dump (DM §9)

66. **R66 (MUST)** A dump is **empty** when the derivation produces no node and no edge: **one line — the header — terminated by LF, and nothing else.** An empty dump *"is legal, is not an error, and exits 0"*. G10 comparing two of them is a **pass**.
67. **R67 (MUST)** Distinguish the three states around it (DM §9): (1) no manifest resolvable ⇒ **not** an empty dump, refuse `not-installed` exit 2, write nothing to stdout — *"A dump of nothing and a dump of a repository spine does not manage are different facts, and conflating them would let a mis-targeted G10 clone compare two 'empty' dumps and pass."*; (2) manifest resolves but `refs/heads/<trunk>` does not ⇒ `head` absent, empty dump; (3) trunk resolves at or below the trust root with no sealed landing above ⇒ `head` present, empty only where `spine init` has not yet landed anything.

### L. Conformance self-check (DM §17) — an implementer MUST satisfy all 24

68. **R68 (MUST)** Items 1–6 (framing/encoding), 7–12 (records), 13–14 (order), 15–20 (exclusions), 21–24 (determinism), reproduced in full in *Error cases* and *Exclusion set* below. Two are worth naming explicitly here:
    - item 13: *"The node section precedes the edge section, and each is sorted ascending by §6.2's and §6.3's key. Verified by re-sorting the records by key and comparing, **not** by sorting the lines."*
    - item 14: *"`id` is unique across the node section; `(from, to, kind, attrs, src)` is unique across the edge section."*
69. **R69 (MUST)** item 20: *"No record's `src` names a commit not reachable from `head`, and no `changeset` node names a commit below the trust root."*
70. **R70 (MUST)** item 22: *"A run in a bare clone of the repository produces bytes identical to a run in a checked-out one, and a dirty working tree changes nothing."*
71. **R71 (MUST)** item 24: *"The dump changes when and only when the trunk tip, the objects it reaches, or the trust root changes."*
72. **R72 (MUST)** item 12: *"No `src` uses the bare `<path>:<line>` form, and none uses `spine:<version>:floor`."*
73. **R73 (MUST)** item 9: *"No `exercises` edge appears."*
74. **R74 (MUST NOT)** Anything in spine may read a dump. *"Nothing in spine ever reads a dump… no fact anywhere in spine is derived from a dump's content."* A dump *"is never a git object, never a note, never fetched"* (DM §1, §16). PB §6.1's law is preserved exactly because *"A byte comparator is not a reader."*

---

## The exclusion set, complete, with the reason for each

### Generating rule (DM §8.1, verbatim)

> **A graph element is in the dump if and only if it is derived from git objects reachable from the trunk tip. An element derived from anything else — an intent branch, the collector's result file, a coverage report, the binary's own floor list, or a heuristic over the objects rather than the objects — is excluded.**

PB §6.3 G10's four adjectives are corollaries: *"provisional (in-flight) elements, † states (dumped as `tests-approved`), volatile test results and worktree-only files excluded"* (PB §6.3 G10, verbatim).

### By node kind (DM §8.2)

| kind | in dump | condition | excluded when | reason |
|---|---|---|---|---|
| `intent` | **yes** | a first-parent trunk commit carrying `Spine-Seal` names it — a `Spine-Event: land` landing or a `withdraw` tombstone; derived from the fenced intent bytes, parsed by the parser for that `Template:` header's **variant and version** (`intent@2`, `intent-change@2`, `intent-bug@2`) | in flight | the branch is `refs/heads/intent/*`, which the clean clone does not have (DM §4.1) |
| `ac` | **yes** | its intent is included | its intent is not | follows the intent |
| `test` | **yes** | derived from a `Spine-Test` id or a pragma in a `Spine-Frozen` path of an included landing, parsed from `<L>:<path>` — *"the frozen blob, reachable through `L`'s tree forever"* | derived from a branch's test files before the landing | branch objects are not reachable from the tip |
| `code_unit` | **yes** | named as `from` or `to` by an included edge | named only by an excluded edge — *"in particular, a path that is on the shipped floor and nothing else"* | the shipped floor is inside the binary |
| `changeset` | **yes** | a first-parent trunk commit carrying `Spine-Seal`, **at or above the trust root**; and every member of `M(L) = git rev-list B..L` for each such landing | in flight (`merge-base..branch`, which PB §6.2 calls provisional); **below the trust root**; inside an `--uninstall` → re-init range, which G9 exempts | not reachable / not in the chain |
| `approval` | **yes** | its signed line is copied into an included landing's envelope | the line is on an in-flight event commit and has not landed | PB §6.2 already says an id-less line's `approves` edge is *"emitted only once the landing is indexed"*; DM extends the same rule to the node |
| `signer` | **yes** | `.spine/allowed_signers` at every trunk first-parent commit from the trust root | **never** | purely trunk-derived |
| `adr` | **yes** | an ADR file present in `adr/` in the **trunk tip's tree** | never, in practice; *"a deleted ADR is not a node"* | tree-derived |
| `constitution` | **yes** | every distinct version observed in the constitution's header on the first-parent walk from the trust root; historical versions are nodes because landed intents carry `built_under` edges to them | **never** | object-derived |

### By edge kind (DM §8.3)

| kind | in dump | note / reason |
|---|---|---|
| `has_ac` | yes | |
| `declares` | yes | **both polarities** |
| `built_under` | yes | |
| `implements` | yes | `provisional` is `false` in every dumped record — *"a provisional edge is an in-flight changeset's, and §8.2 excluded the changeset"* |
| `modifies` | yes | the landing's `git diff --name-only B L`, and the per-member diffs PB §6.2 keeps for archaeology |
| `approves` | yes | |
| `signed_by` | yes | |
| `attested_by` | yes | |
| `freezes` | yes | to `code_unit` with `oid`, to `test` with `{}` |
| `verified_by` | yes | `attributed` kept, **`introduced_by` excluded** |
| `reverts` | yes | derived from `git patch-id --stable` over `L`'s paths; see DM §10 rule 12 (diff algorithm pinned) |
| `supersedes` | yes | |
| `superseded_by` | yes | |
| `protects` | **partly** | `C-A2` entries — derived from the constitution blob, a git object — are included, `from` the `constitution:<v>` node, `floor: false`. **Shipped-floor entries are excluded** |
| `exercises` | **no** | PB §6.2 marks it optional/v1.1; its source is a CI coverage report, not a git object. *"It is excluded now and stays excluded when it ships… a coverage report has the same standing as a result file"* |

### By attr (DM §8.4)

- **`test.result_at {tree, base, passed}`** — PB §6.2 marks it volatile and PB §6.3 G10 excludes it by name. RF §2, quoted verbatim in DM §8.4: *"Populates the volatile `test.result_at {tree, base, passed}` attrs only (§6.2), on `test` nodes whose ids are `test:<runner>:<id>` (§6.2). G10 excludes those attrs from the canonical dump, so a result file can never affect reconstruction."* Consequence: **every `test` node in a dump carries `{}`**.
- **`verified_by.introduced_by`** — see below.

### The three exclusions DM adds beyond PB's four adjectives (DM §8.5) — each is a nondeterminism

1. **`verified_by.introduced_by`.** PB §6.2 derives it with `git blame` and says in the same clause it is *"for archaeology, never a gate input"*. *"`git blame` has no specified output contract: its rename and copy detection are heuristics whose defaults and behaviour have changed across git releases, so the value is a function of the git binary rather than of the objects."* Including it *"would make a routine `git` upgrade on the runner turn the next landing into `reconstruction-failed`, with a report naming the graph and no way to see that the cause was a package update."* Pinning blame's flags was **considered and refused**: *"pinning flags does not pin a heuristic's implementation."* The store may hold it; the dump does not.
2. **Shipped-floor `protects` edges.** PB §6.2 derives `protects` from *"the floor list inside the pinned release (`spine:<version>:floor`) + constitution `C-A2`"*. The `C-A2` half is a git object and stays. The shipped half *"is not in the repository at all; it is inside the binary"*. Including it would make the dump a function of the release, which DM §3.4 forbids; and it would require a node kind for the release, which PB §6.2 does not have. *"G10 loses nothing: both sides run the same binary, so comparing the shipped floor against itself proved nothing."*
3. **`exercises`.** Same rule; stated so a v1.1 implementer adding coverage finds the ruling.

*"Each of the three is a change to PB §6.3's G10 clause, not merely a reading of it."* (DM §8.5 → DM §14 D4.)

### † states (DM §8.6)

PB §6.3 says † states are *"dumped as `tests-approved`"*; PB §6.2 says they *"collapse to `tests-approved` in any fresh clone"*; PB §11 States: *"† = runner-local, collapses to `tests-approved` in any clone."* **Under DM §8.2 the clause is vacuous** — † states are in-flight, in-flight intents are not dumped, and no landed intent has a † status. Kept as a conformance check (*"one string comparison"*) and as the rule a later `dump_version` would need.

### The worktree (DM §8.7, verbatim)

> **A dump is a function of trees and refs. Running `--dump` in a bare repository, with a dirty working tree, with a stale index, or with untracked files present produces identical bytes.**

Immediate consequence: PB §6.1's `<path>:<line>` production is **never emitted**; every file citation in a dump is `git:<sha>:<path>:<line>`.

---

## Byte-level fixities (verbatim)

**Framing** (DM §2.2 rule 1): *"The byte stream is a sequence of **lines**, each terminated by exactly one `0x0A` (LF). The final line is terminated too, so the stream ends with `0x0A`. No CR anywhere, no BOM, no blank lines, no comments, no trailing blank line."*

**Canonicalization, reduced** (DM §2.3): *"sort each object's members by member-name bytes ascending; emit with no whitespace; emit integers in plain decimal; emit strings with JSON's minimal escaping (`"` → `\"`, `\` → `\\`, nothing else can occur); output UTF-8, which is here also ASCII."*

**Value profile** (DM §2.3, verbatim rows):

| Restriction | Rule |
|---|---|
| Member names | *"Match `^[a-z][a-z0-9_]*$`. ASCII only, so JCS's UTF-16 ordering reduces to byte ordering."* Complete set fixed by DM §5: `attrs`, `dump_version`, `from`, `head`, `id`, `kind`, `object_format`, `repo`, `schema_version`, `src`, `t`, `to`, `trunk`, `trust_root`, plus the attr names of DM §7.2 |
| Numbers | *"Integers only, `0 ≤ n ≤ 2^53 − 1`. No sign, no leading zero, no fraction, no exponent, no `-0`. There is no floating-point value anywhere in a dump."* |
| Strings | *"ASCII only after `esc` (§2.4): every character is in `U+0020 … U+007E`."* |
| Booleans | *"`true` and `false` are values; they appear only where §7.2 names them."* |
| Null | *"Never emitted. An absent value is an absent member."* |
| Duplicate names | *"Invalid."* |
| Arrays | *"Elements are strings only. Order is fixed per attr by §7.2."* |
| Depth | *"Exactly two: a record object, whose `attrs` member is an object of scalars and string arrays. **No attr value is an object.**"* |

**Non-normative implementation note** (DM §2.3): *"`json.dumps(obj, sort_keys=True, separators=(',',':'), ensure_ascii=False).encode('utf-8')` is byte-identical to JCS"* for this profile. *"It is not JCS in general — floats and non-BMP member names diverge — which is exactly why the profile exists."* Debug against GR §8.3's published minimal canonicalizer vector **first**.

**`esc`** — owned by GR §2.3, adopted verbatim by DM §2.4; *"a divergence between the two documents is a defect in `gate-report.md`, which owns it."* GR §2.3's table, verbatim:

| `b` | emits |
|---|---|
| `0x5C` (`\`) | the two characters `\` `\` |
| `0x20 … 0x7E`, other than `0x5C` | the character with that code point |
| anything else (`0x00–0x1F`, `0x7F–0xFF`) | the four characters `\` `x` and two **lowercase** hex digits of `b` |

*"The result is a character string over `U+0020…U+007E`, which the JSON layer then escapes normally (`"` → `\"`, `\` → `\\`)."* Decoding is total: *"`\` introduces either `\` (one literal backslash) or `x` plus exactly two lowercase hex digits (one byte). Any other sequence after `\` is an invalid report."*

Two-layer worked case that matters for the dump vector (GR §2.3): path bytes `caf` + `0xC3 0xA9` → `esc` = `caf\xc3\xa9` → bytes in canonical JSON = `"caf\\xc3\\xa9"`. **In the dump vector the on-the-wire bytes for `é` (Latin-1 `0xE9`) are literally `caf\\xe9.py` — two backslashes** (DM §12.2, confirmed by byte inspection of the published block in this session).

**`tok` is not used** (DM §2.4): *"The `tok` variant of `gate-report.md` §6.2 is not used here… A dump has no such framing: a JSON string carries a comma, a space and a quote without help."*

**No normalization** (DM §2.4): *"Nothing is ever normalized. No NFC, no NFD, no case folding, no separator rewriting, no path cleanup."*

**Ordering, over `esc` bytes** (DM §6.4, verbatim): *"Every component of both keys is a string over `U+0020 … U+007E`… Comparison is **ascending over those bytes**, unsigned, with the shorter string first when one is a prefix of the other. Because the alphabet is ASCII, byte order, code-point order and UTF-16 code-unit order coincide."*

**Why `NUL` is the separator** (DM §6.4): *"No component can contain `0x00`: `esc` maps it to the four characters `\x00`, and a JCS-serialized `attrs` is JSON text. So comparing the concatenations is exactly comparing the components in order, and the classic separator hazard — `a/b` sorting against `a-b` — cannot arise."*

**The `esc`-versus-raw ordering trap** (DM §6.4, verbatim): for `src/z.py` and `src/` + `0xE9` + `.py` —
- *"raw bytes: `src/z.py` first, because `0x7A < 0xE9`;"*
- *"`esc` bytes: `src/\xe9.py` first, because `0x5C < 0x7A`."*

*"**The encoded order governs**, for one reason that decides it: those are the bytes in the artifact, so the dump is sorted with respect to itself, and a reader can verify the order from the file without decoding it."*

**Numeric-looking ids sort as bytes** (DM §6.4): *"`AC-10` precedes `AC-2`; `G11` precedes `G2`."* This agrees with PB §11's `Spine-Review` row — *"ascending by unsigned byte value over the whole token, so `G11` precedes `G2`; a set with no order is a signature two runs spell differently"* — *"They agree by rule and not by dependence."* GR §6.1–§6.2 have since adopted the same byte order.

**`approval.wires` is NOT re-sorted** (DM §7.2): *"the `wires=` tokens, **in the line's order**, which PB §11 fixes as ascending by unsigned byte value over the whole token (so `G11` precedes `G2`)… Not re-sorted here: the signed line's order is the fact, and a dump that re-sorted it would hide a non-conforming review rather than reproduce it."*

**`signer.roles` IS sorted** — *"ascending by bytes: a subset of `spine-review@v1`, `spine-seal@v1`, `spine-signoff@v1`"* (DM §7.2).

**Why a dump is not `sort`-stable** (DM §6.5, verbatim): *"JCS orders a record's members alphabetically, so every node line begins `{"attrs":` and every edge line begins `{"attrs":` — the sort key is not a prefix of the line… an empty `attrs` sorts *after* a non-empty one at the line level (`}` is `0x7D`, `"` is `0x22`), and a line sort would interleave the two sections."*

**The `approval` local id byte range** (DM §5.2.1, verbatim): *"`approval:` + the SHA-256, as 64 lowercase hex digits, of **the exact bytes of the signed trailer line as the commit message carries it** — from the first byte of the trailer name (`Spine-Approve`, `Spine-Signoff`, `Spine-Review`, `Spine-Reopen`, `Spine-Withdraw`, `Spine-Upgrade`) through the last byte before its terminating LF, with no LF included."*

**The dump digest** (DM §2.5): *"`sha256:` + 64 lowercase hex digits over exactly the byte stream of §2.2 — including the final LF, excluding nothing."* *"It is never sealed, never signed, never a trailer field, and never a member of a gate report."*

---

## Error cases

| Condition | Behaviour | Exit | Status token |
|---|---|---|---|
| The dump is on stdout | normal | **0** | `ok` (DM §4.4) |
| Trunk resolution reached step 4 of DM §4.2 (no `--trunk`, no manifest in `HEAD`'s tree, none in any first-parent ancestor's tree) | **REFUSE**; *"Nothing is written to stdout"* | **2** | `not-installed` (DM §4.4, §9 case 1) |
| The derivation produced a `src` outside PB §6.1's grammar | **REFUSE** | **3** | `provenance-invalid` (DM §4.4, §5.4) |
| The derivation produced a node id outside DM §5.2 (includes a manifest `repo` not matching `^[A-Za-z0-9._-]+$`) | **REFUSE** | **3** | `id-out-of-grammar` (DM §4.4, §5.2) |
| An attr value outside DM §2.3 or §7.2 — *"a float, a `null`, a nested object, an unknown name"* | **REFUSE** | **3** | `attrs-out-of-profile` (DM §4.4, §7.1) |
| **G10 only:** two dumps with different `dump_version` or `schema_version` offered for comparison | **REFUSE**, *"rather than comparing"* | **3** | `dump-version-skew` (DM §3.2, §4.4) |
| **G10:** `D_S ≠ D_C` | **REFUSE the push.** `L` discarded, never becomes a git object; run ends **without re-queueing and without consuming a `C-M3` retry**; the run's own report is the only record; **terminal**, never re-queued, **unbypassable by break-glass** | (run-level) | intent state **`reconstruction-failed`** (PB §5.4 step 5; PB §5.5 state table; PB §6.3 G10; DM §11 step 6) |
| A well-formed `from`/`to` naming no node record | *"an indexer defect that this format cannot detect and **G5** must"* — the dump is non-conforming | — | (G5's finding, not the dump's) (DM §5.3) |
| An external tool meets an unknown `dump_version` or an unknown member name | **REFUSE** (no tolerant reading) | — | (DM §3.2) |
| `--dump` in a bare repo / dirty tree / stale index / untracked files | **not an error** — identical bytes | 0 | (DM §8.7) |
| Derivation produces no node and no edge | **not an error** — the one-line empty dump | 0 | `ok` (DM §9) |
| An `implements` edge with `provisional: true`, a `protects` with `floor: true`, a `test` node with `result_at`, a `verified_by` with `introduced_by`, a † `intent.status`, an `exercises` edge | **non-conforming serializer** (DM §17 items 15–19, §7.2, §7.3) | — | — |

Rationale for the exit-3 family, verbatim (DM §4.4): *"Exits 3 are internal-consistency refusals: the derivation produced something this format cannot represent, and emitting a partial dump would produce a spurious terminal G10 failure with a misleading diff. Refusing loudly, in the same process that built the graph, names the defect instead."*

---

## Why G10 is terminal and unbypassable, and what it implies for the indexer

- **Terminal.** PB §6.3 G10: *"A failure **refuses the push**, ends the run as `reconstruction-failed` without a retry. The discarded `L` never becomes a git object, so the run's own report is the only record — one more reason the failure is terminal rather than quiet."* PB §5.5's state table (line 540): `checked †` + *"G9 or G10 fails on the built `L`"* → *"reconstruction-failed — discarded, reported, never re-queued: a deterministic failure re-runs identically"*.
- **No `C-M3` consumption.** PB §5.4 step 5: *"A refusal discards the landing, ends the run **without re-queueing and without consuming a `C-M3` retry**"*. (Contrast `base-moved†`, which *does* consume one.)
- **Unbypassable.** PB §7.6 lists what break-glass may bypass — *"G2, G3, G4, G6, G7, G12 and — of Integrity — G8 and G1 only, recorded as a *freeze override*; never G5, G9, G10, G11, and never Authority"*. PB §11 Gates repeats the whitelist. DM §1 adds the structural reason: *"G10 is not in PB §7.6's list and could not be, since `L` already exists and its seal covers its own message."*
- **Invisible in the ledger.** G10 never enters `Spine-Gates` (PB §11, PB §5.4 step 5, PB §5.5 line 475, GR §5.6.2). So a G10 failure leaves **no git artifact at all**.
- **What this implies for indexer correctness** (DM §1, verbatim): *"the dump is the only artifact in spine-kit whose *format* can fail a landing. Every other artifact is checked against a signature or a digest; this one is checked against another copy of itself. A difference of one byte — a key emitted in insertion order, an integer with a leading zero, a path spelled NFD on one side and NFC on the other, a `git blame` heuristic that changed between git releases — is indistinguishable from a corrupted ledger and produces the same terminal refusal."*
- **It is an indexer defect, not a ledger defect** (PB §6.3 G10): *"It is still an indexer defect to file against spine, not a ledger defect — the envelope G9 accepted is valid — but a landing a clean clone cannot reproduce does not reach trunk."*
- **What a difference *means*.** G10 proves that the projection of the ledger is reproducible from a clean clone. It does **not** prove the lease registry: *"G10 proves the ledger, not the lease registry"* (PB §6.3, DM §1, §4.1). The clone asymmetry is deliberate and is what forces the exclusion set: *"`S` was cloned from the origin repository, so it holds `refs/remotes/origin/intent/*` for every intent still in flight; the clean clone was made from `S`'s *local* heads only, so it holds none of them. Any element derived from an intent branch is present on one side and absent on the other, and G10 fails on every landing made while any other intent is open."* (DM §4.1)
- **The dump is a projection, not the graph** (DM §1): the store `.spine/cache/graph.sqlite` holds in-flight intents, provisional changesets, volatile test results and the shipped floor because `spine check`, `spine context` and the drift gates need them; *"The dump holds only what a fresh clone of trunk can rederive."*

---

## Worked examples / test vectors

All three vectors were **independently recomputed in this session** from the exact bytes `dump.md` prints; all three match, as do all three `approval` local ids.

### V1 — the full dump vector (DM §12.2, §12.3)

Repository (DM §12.1): `myrepo`, `object_format: sha1`, `params.trunk: main`, `params.langs: [python]`, pinned release 1.4.0, team mode, `C-A3: hostile`, `C-M1: merge`. Keyring is `manifest.md` §8.7's / `envelope-vectors.md` §8.1's **three principals** `alice@example.com`, `bob@example.com`, `ci@example.com`. Trust root `T0` = `0a1b…4567`; landing `L` = `1b2c…6789` for `INT-042: Invoice totals include tax`, base `T0`. `M(L)` = `M1 2c3d…8901`, `M2 3d4e…012a`, `M3 4e5f…2ab3`, `M4 5f60…b3c4`, `M5 60718…c4d5`; `Hc = M5`.

```
lines:  62
bytes:  14054
digest: sha256:3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da
```

**Reproduced in this session** — `sed -n '603,664p' docs/spec/dump.md | shasum -a 256` → `3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da`; `wc -l -c` → `62 14054`. This matches `docs/spec/README.md`'s published-digest row verbatim: *"62 lines, 14054 bytes, `sha256:3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da`"*.

**Composition, counted from the vector in this session** (an implementer can use these as assertions): 1 header + **28 node records** + **33 edge records** = 62 lines.

| node kind | count | edge kind | count |
|---|---|---|---|
| `ac` | 2 | `approves` | 3 |
| `adr` | 1 | `attested_by` | 1 |
| `approval` | 3 | `built_under` | 1 |
| `changeset` | 6 | `declares` | 4 |
| `code_unit` | 9 | `freezes` | 4 |
| `constitution` | 1 | `has_ac` | 2 |
| `intent` | 1 | `implements` | 6 |
| `signer` | 3 | `modifies` | 6 |
| `test` | 2 | `protects` | 1 |
| | | `signed_by` | 3 |
| | | `verified_by` | 2 |

*"The example exercises: every node kind; **eleven** of the fifteen edge kinds (`reverts`, `supersedes` and `superseded_by` have no occasion in a two-commit history; `exercises` is excluded from every dump by §8; `protects` **is** present, but only its `C-A2` limb…); a non-UTF-8 path through `esc`; the `msg:L<n>`, `trailer:<Name>`, `git:<sha>` and `git:<sha>:<path>:<line>` provenance productions; an absent optional attr (`signer.valid_to`); an array attr; and the `code_unit` minimum-`src` rule of §5.5."* (DM §12.1)

**Header line, verbatim (line 1 of the vector):**

```
{"dump_version":1,"head":"1b2c3d4e5f60718293a4b5c6d7e8f90123456789","object_format":"sha1","repo":"myrepo","schema_version":7,"t":"header","trunk":"main","trust_root":"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567"}
```

**The seven checkpoints DM §12.3 publishes** (the heading says "Six things"; seven are listed — see *Contradictions found*):

1. *"**Node order is `kind` then `id`**, so `myrepo/INT-042` (kind `intent`) is the twenty-third node, far below `myrepo/INT-042/AC-1` (kind `ac`), which is the first. PB §6.3's order, not an id order."* — **verified in this session**: `myrepo/INT-042` is file line 24 = node #23.
2. *"**`myrepo/code:src/billing/caf\xe9.py` precedes `myrepo/code:src/billing/tax.py`**, which raw-byte order would reverse (§6.4)."*
3. *"**The `code_unit` src is the minimum over citing edges** (§5.5): `tests/billing/test_invoice.py` is cited by `modifies` from `L` and from `M2` and by `freezes` from `M3`, and takes `git:1b2c…6789`, the least of the three."*
4. *"**Member changesets carry `{"landing":false}` and nothing else.** They have no seal."*
5. *"**The two `declares` edges to `code:src/billing/` and `code:api/invoices.ts` share a `src`** — one line of the touchpoints block names both paths — and are ordered by `to`, PB's second key."*
6. *"**`signer.valid_to` is absent, not null**, on all three signers."*
7. *"**The three fingerprints are real** and reproduce from `envelope-vectors.md` §8.1's published keys with `ssh-keygen -lf`; the seal principal is `ci@example.com`… **The line count did not change and the byte count did**: 62 lines either way, 14081 bytes before the substitution and 14054 after."*

**An eighth checkpoint an implementer will hit and DM does not spell out** (derived from the vector, ordering rule DM §6.3): under `from = myrepo/approval:b6352921…` the six edges are ordered by `to` **before** `kind`, so a `signed_by` edge sits *between* two pairs of `freezes` edges:
`…/INT-042` (approves) → `…/code:pytest.ini` (freezes) → `…/code:tests/billing/test_invoice.py` (freezes) → `…/signer:alice@example.com` (signed_by) → `…/test:pytest:…test_AC1_totals_include_tax` (freezes) → `…/test:pytest:…test_AC2_zero_rated` (freezes). Because `I`(0x49) < `c`(0x63) < `s`(0x73) < `t`(0x74).

**The three signed trailer lines whose bytes DM §5.2.1 hashes (DM §12.1, verbatim):**

```
Spine-Signoff: INT-042 blob=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 template=intent@2 constitution=v3 reopens=0 signer=alice@example.com
Spine-Approve: INT-042 intent=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 base=0a1b2c3d4e5f60718293a4b5c6d7e8f901234567 rounds=1 total_rounds=1 reopens=0 red=5/5 freeze=sha256:5c9e2a71b0463df8951ce2a4708b3d61f0492c8ad735be106f4a2c9d80e37b45 signer=alice@example.com
Spine-Review: INT-042 class=tripwire head=60718293a4b5c6d7e8f90123456789012ab3c4d5 tree=7b0dc1f4a2e58d3906bb4c7e21f5a8d90c3e64b7 base=0a1b2c3d4e5f60718293a4b5c6d7e8f901234567 intent=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 report=sha256:3c6f1a09b8d24e57af0132c9de6b48570e29a1cf83b6d045e71a29c4b0d83e16 wires=G11 reason="auto-merge unavailable: C-A3 hostile" reviewer=bob@example.com
```

| line | `approval` local id | reproduced here? |
|---|---|---|
| `Spine-Signoff` | `2f5e600237ec3d9a1f35fbc7ee6cf0dfd4335433def9937b5eeb8202bd3d66f6` | **yes** |
| `Spine-Approve` | `b6352921ea42d618930f32f04ad773c20157810d418d20d06758149f366b85a8` | **yes** |
| `Spine-Review` | `ae8a406391f7130ce62d0e86fa4dca92195812aae2743e15e60434af56244021` | **yes** |

(`printf '%s' "<line>" | shasum -a 256` — no trailing LF. This is the direct test of DM §5.2.1's byte range.)

### V2 — the ordering vector (DM §12.4)

*"Debug your comparator against this before attempting §12.2."* Canonical block, 11 lines, **no header — a fragment, not a dump**:

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

**Reproduced in this session** — 11 lines, 1063 bytes, `a849ec349ef8f20ec1f40423ae6a7d3358745f4c9027545f55cf74ef9b72a139`.

What it pins (DM §12.4, verbatim):
- *"`AC-10` before `AC-2` — byte order, not numeric (§6.4);"*
- *"`ac` < `changeset` < `code_unit` — `kind` is the first node key;"*
- *"`\xe9.py` before `z.py` — `esc` order, the reverse of raw-byte order;"*
- *"the two `declares` edges tie on `from`, `to` and `kind` and break on `attrs`: `expected` before `forbidden`;"*
- *"the two `modifies` edges tie on `from`, `to`, `kind` **and** `attrs` and break on `src`: `git:aa` before `git:bb`;"*
- *"edges under one `from` are ordered by `to` before `kind`, so both `has_ac` edges precede both `declares` edges — PB §6.3's `from,to,kind` exactly."*

Note that the two `modifies` records here differ only in `src`, i.e. they are **not** collapsed by DM §5.5 in this fragment — DM §5.5's collapse applies to the *derivation's* multiple citations of one element; the vector shows the tie-break level that exists after it.

### V3 — the empty-dump vector (DM §12.5, §9)

```
{"dump_version":1,"object_format":"sha1","repo":"myrepo","schema_version":7,"t":"header","trunk":"main"}
```

```
bytes:  105
digest: sha256:2a3fcea5debff8e17af36eef1a4acfa1fea9f86a0d6c942cf2993cce3e80f290
```

*"**Verified.** One line, one LF, no `head`, no `trust_root` (§9 case 2)."* — **reproduced in this session**: 1 line, 105 bytes, digest matches.

### V0 — build against this first (GR §8.3)

*"`gate-report.md` §8.3 publishes a verified minimal canonicalizer vector; **debug against that one before attempting §12**, since it is the same scheme and it is already reproduced."* (DM §2.3)

```
value:     {"b":[1,2],"a":"x\\y","Z":true,"_c":{"n":0,"m":"q\"r"}}
canonical: {"Z":true,"_c":{"m":"q\"r","n":0},"a":"x\\y","b":[1,2]}
digest:    sha256:a594772ccb6408158b6e76b170d5488c2454ba576e09ae379e24d743e21921f0
```

### How to reproduce V1 from scratch

1. Build a JCS serializer for DM §2.3's profile (validate against V0).
2. Build `esc` per GR §2.3 (validate against GR §2.3's four worked cases, including `caf` + `0xC3 0xA9` → `"caf\\xc3\\xa9"`).
3. Build the two comparators of DM §6.2/§6.3 (validate against V2 — 1063 bytes, `a849ec34…`).
4. Emit the header of DM §3.1 with `head` and `trust_root` present (validate the empty case against V3 — 105 bytes, `2a3fcea5…`).
5. Derive the 28 nodes and 33 edges of DM §12.1's repository; hash the three trailer lines of DM §12.1 for the `approval` local ids (validate against the three ids above).
6. Concatenate header ‖ sorted nodes ‖ sorted edges, each line + one LF including the last; `sha256sum` must give `3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da` at 14054 bytes.

---

## Cross-references it depends on (which other sheet owns what)

| What | Owner | Note |
|---|---|---|
| **`esc`** | **GR §2.3** | Adopted verbatim by DM §2.4. *"a divergence between the two documents is a defect in `gate-report.md`, which owns it."* |
| **JCS + value profile precedent, and the minimal canonicalizer vector** | **GR §2.1, §2.2, §8.3** | DM reuses the scheme deliberately: *"an implementer who has built a JCS serializer for the gate report reuses it byte for byte."* |
| **Wire token `tok`, and `wires=` order** | **GR §6.1–§6.2** | DM §7.2's `approval.wires` records what the signed line carries; DM does **not** re-derive or re-sort. PB §11 fixes the direction. |
| **G10 never appears in a version-1 report** | **GR §5.6.2** | Same fact from the report's side. |
| **The result file, and `test.result_at`** | **RF §2, §4.1, §4.4** | RF §4.1 frames the collector's output as JSONL (DM reuses the framing); RF §4.4 states the same "paths are the tree's bytes" rule; RF §2 states that G10 excludes `result_at` so *"a result file can never affect reconstruction."* |
| **`repo`'s grammar and length bound; `params.langs`** | **MF §3.1, §3.3** | MF §3.1 *"now owns it and adopts it unchanged"*, adding a 1…64-byte bound and refusal `repo-out-of-grammar` (MF §14 R18). DM §5.2's constraint does not move. |
| **G13's uniqueness of event lines (what makes the `approval` id safe)** | **MF §4.8.4 check 3** | Cites DM §5.2.1 as one of two dependencies forcing the reading. |
| **The envelope grammar, trailer syntax, what a `-Sig` covers, the published keyring** | **EV** (§8.1 keys, §8 vector A) | DM §5.2.1 defines its own byte range *"so that it does not move when `envelope-vectors.md` fixes the signed payload."* |
| **The `@verifies` pragma grammar, the file-granular join, per-runner naming sugar** | **IR §12 (§12.1, §12.2, §12.3)** | DM §16 names this as *"the one out-of-scope pointer that can invalidate this document without touching it"* — *"every vector in §12 here is only as reproducible as that section is."* Four v1 languages: Python, TS/JS, Dart, Swift; **Kotlin dropped**. |
| **`.spine/ci.sh`, the three CI definitions, publishing the report to `refs/notes/spine`** | **CI** | DM §4.3: the note is never an input. |
| **The derivation itself** (which commits are walked, envelope parsing, touchpoint→`code_unit`, constitution version reading) | **PB §6.2's derivation table** — *"this table is the indexer's spec"* | DM §16: DM serializes its output and fixes only the determinism rules that bear on the bytes. |
| **The freeze closure / G8** | PB §4.3, IR | DM §16: *"It does **not** affect a dump: `freezes` edges are derived from the `Spine-Frozen` and `Spine-Test` lines the approval carries, not recomputed by the indexer."* |
| **Gate semantics** (G2 containment, G14's casefolding, when a landing is `unattested`) | the respective gate specs | DM §16: *"This document fixes how a derived fact is *recorded*, never what a gate decides."* |
| **Diagnostics / how a G10 failure is presented** | the CLI and PB §6.5's review packet | DM §16: *"Stdout carries the artifact and nothing else."* |
| **Trunk resolution** | DM §4.2 **provisionally** | *"Trunk resolution properly belongs to the indexer, not to a serialization format… if a later `cli.md` states it, that document wins and this section becomes a citation."* |

---

## OPEN items (undecided; do not invent)

From **DM §15** — the owner's calls:

1. **OPEN-1 · Whether the shipped floor belongs in the graph, and under which node.** DM §8.5 excludes shipped-floor `protects` edges from the *dump*; the *store's* question is open. Three ways out, as written: *(a)* leave it — *"the store may hold the edges under whatever `from_id` an implementation likes, since nothing compares them — cheap, and it means two implementations' stores differ in a way no gate notices"*; *(b)* add a `release` node kind, id `<repo>/release:<version>`, `PRAGMA user_version` 8; *(c)* drop the edges entirely and let G14 read the floor list directly. **DM's recommendation: (c), then (b) if a query ever needs it. Owner-level because (b) changes PB §6.2.** (DM §15 OPEN-1)
2. **OPEN-2 · Whether `tree: unverifiable(git-version)` should exist.** *"It is the only value in the dump that reads the local environment (§7.2.1)."* Alternatives named: make the version mismatch a hard G9 finding (landing `unattested`), or make the tree check conditional and record nothing. **DM's recommendation: keep it, and add the git version to `spine stats`. Owner-level because it is a G9 semantics question, not a serialization one.** (DM §15 OPEN-2, §13.7)
3. **OPEN-3 · Whether `--dump` should have a mode that includes in-flight elements.** *"If such a mode ships it is a **second artifact**, not a flag on this one: it needs its own version, its own exclusion set, and — critically — it must not be what G10 diffs."* **DM's recommendation: not in v1; if it comes, name it something other than `--dump`. Owner-level because it is a PB §10 budget line.** (DM §15 OPEN-3)

From **DM §14** — playbook defects still OPEN (they are undecided owner questions about PB, not about DM's bytes):

4. **D2 · OPEN · A trailer citation cannot name the second of two identical trailers** (PB §6.1's grammar `git:<sha>:trailer:<Name>`). Recommended: `git:<sha>:trailer:<Name>#<n>`, `n` the 1-based occurrence. *"Latent for dumps… active for any tool that reads a rendering."* **Do not implement `#<n>` in v1 — it would change the dump bytes and require a `dump_version` bump.**
5. **D3 · OPEN · The provenance grammar is not unambiguously parseable.** `git:<sha>:<path>:<line>` collides with `git:<sha>:msg:L<n>`, `…:trailer:<Name>` and `…:patch-id` when a path is called `msg`, `trailer` or `patch-id`; and `<path>:<line>` collides with the whole `git:` family when a path begins `git:`. *"A last-colon rule plus an oid-length test disambiguates in practice, but it is a heuristic and PB §6.1 offers none."* Latent for dumps (nothing parses a `src`).
6. **D4 · OPEN · G10's exclusion clause is four adjectives.** Recommended: *"the clause cites this document instead of enumerating."*
7. **D5 · OPEN · PB §6.2's node-id examples contradict its own repo-scoping rule.** Recommended: prefix the examples.
8. **D6 · OPEN · The schema has no edge kind that answers PB §6.4's ADR query** (*"any ADRs touching the same code units"*). *"As it stands an `adr` node is isolated (as in §12.2), the query cannot be answered, and the promise is unbacked. This costs the dump nothing; it costs `spine context` a feature."*

**D1 is CLOSED** — PB §6.3's G10 row now writes the trust root into **both** sides (verified against `PLAYBOOK.md` line 671 in this session; PB §12 line 1085 records the fix).

---

## Contradictions found

1. **PB §6.3 G10's exclusion clause vs DM §8's enumerated set.** PB: *"provisional (in-flight) elements, † states (dumped as `tests-approved`), volatile test results and worktree-only files excluded"*. DM §8.5 adds three exclusions PB's four adjectives do not reach — `verified_by.introduced_by`, shipped-floor `protects`, `exercises` — and says so explicitly: *"Each of the three is a change to PB §6.3's G10 clause, not merely a reading of it."* PB §11 is silent, so **DM governs**; DM §14 D4 files the PB clause as a defect (OPEN). **Implementer: follow DM §8.**
2. **PB §6.2's node-id examples vs PB §6.2's own repo-scoping rule.** PB §6.2's `id` row lists `"code:src/billing/" | "cs:abc123f" | "approval:5c9e…"` — unprefixed — while the same paragraph says *"IDs are repo-scoped from day one (`myrepo/INT-042`, not bare `INT-042`)"*. DM §13.3 reads the examples as abbreviations and prefixes **every** kind. DM §14 D5 (OPEN). **Implementer: every id begins `<repo>/`.**
3. **PB §6.2's `approval:5c9e…` example vs DM §5.2.1's line-hash id.** PB §4.3 calls the approve line's `freeze=` *"a non-git digest, used to name the approval elsewhere"*, and `5c9e2a71…` is literally the `freeze=` digest in DM §12.1's approve line — so PB's example reads as the freeze-digest scheme. DM §5.2.1 / §13.6 **rejects** it: *"That reading is rejected because it is total over only one of the six `event` values."* PB §11 is silent. **Implementer: SHA-256 over the signed trailer line's exact bytes.**
4. **PB §11's States list vs DM §7.3's `intent.status` domain.** PB §11 States: *"post-landing `reverted`, `superseded`, `orphan`, `unattested`, `resealed`"*. DM §7.3 restricts `intent.status` to `merged | withdrawn | reverted | superseded` and puts `unattested`, `resealed` and orphanhood on the **changeset**, arguing from PB §6.2's schema (which does list `unattested` and `resealed` under `changeset` and not under `intent`). **Under the corpus's own precedence rule PB §11 wins over a spec — but PB §11's list is a lifecycle-state list, not an assignment of attrs to node kinds, and PB §6.2 is unambiguous about where the two flags live.** DM §13.8 states the resolution. **Flag for the owner:** either PB §11 should say these are changeset facts, or DM §7.3 must widen. Nothing in DM §14 files it, so it is an unfiled disagreement.
5. **`object_format`'s source: GR vs DM.** GR §5 takes it from the manifest at `base`; PB §6.7 makes it a manifest field. DM §3.1/§13.11 takes it from the **indexed repository's** `extensions.objectFormat`, defaulting to `sha1`. DM: *"The two agree in every conforming repository; where they disagree, the repository is broken and the disagreement is G15's or G16's finding."* **Deliberate, documented divergence — the two artifacts may name it differently in a broken repo.**
6. **Stale citation inside `dump.md` itself.** DM §3.1 says *"**PB §6.3 G10 as written copies the pinned trust root into the *clean clone* only; the scratch clone `S` gets no `spine.trustRoot`**"*, and DM §11 step 3 says *"§14 D1: PB §6.3 names only the clone"* — but DM §14 D1 is marked **CLOSED** and quotes the current PB text, which I verified at `PLAYBOOK.md` line 671: *"the runner's pinned trust root is written into **both** sides' `spine.trustRoot`"*. **DM §3.1 and §11 step 3 describe a PB that no longer exists.** The requirement (write to both) is not in doubt; the citation is stale.
7. **DM §12.3 says "Six things" and lists seven.** *"Six things in it are worth checking against"* is followed by items 1–7. Cosmetic, but an implementer counting checkpoints will notice. (Item 7 — the fingerprint/principal substitution — was appended by the 2026-08-27 keyring reconciliation that `docs/spec/README.md` records.)
8. **PB §11 CLI vs DM's flag set.** PB §11 gives `spine index [--fresh] [--dump]` and also `spine check … [--reconstruct]`. DM specifies `spine index --fresh --dump` (DM §11 step 4) and states `--dump` implies `--fresh` (DM §4.3), but says nothing about `spine check --reconstruct`. Not a contradiction, a **gap**: no document in the read set defines what `--reconstruct` does. Worth routing to whichever sheet owns the CLI surface.
9. **`exercises` is in PB §6.2's closed edge-kind set but never emitted.** DM §5.3 keeps it in the closed `kind` domain while DM §8.3 and §17 item 9 forbid it from appearing. Not a conflict, but the implementer must model the enum with fifteen members and the emitter with fourteen.

---

## Notes an implementer will otherwise get wrong

- The dump digest is a **convenience**, not part of the artifact: *"It is never sealed, never signed, never a trailer field, and never a member of a gate report"* (DM §2.5).
- A dump is written to stdout and consumed in the same process tree. *"never a git object, never a note, never fetched"* — *"there is no channel by which one side's dump could reach the other"* (DM §1).
- The two version numbers move independently and both are recorded (DM §3.3): a store-schema change adding an *excluded* attr moves `schema_version` only; a projection change moves `dump_version` only; a store-schema change adding an *included* element moves both.
- `intent.title` is read **from the sealed intent inside the landing commit's message, never from that commit's subject line** — the subject is *derived from* that same line (PB §11 Subject lines) and on a quick-lane landing (which every toolkit lifecycle landing is) it is free text with no intent node at all (DM §7.2).
- `approval.role` is *"the namespace the signature verified under, never a claim in the trailer"* — a v1 approve line signed under `spine-review@v1` is `reviewer`, which is exactly what DM §12.2's approve node shows (`"role":"reviewer"` on a `Spine-Approve`).
- **`freezes` edges have two different `src` shapes in one dump**: to a `code_unit` the citation is the member commit's `trailer:Spine-Frozen`, to a `test` it is `trailer:Spine-Test` — and the `signed_by` edge for the same approval cites the *landing's* `trailer:Spine-Approve`. Visible in V1.
- *"An attr that names a git object carries the oid; a reference to another node is an edge, never an attr. `intent.landing` is `L`'s oid, not `myrepo/cs:<L>`"* (DM §7.1).
