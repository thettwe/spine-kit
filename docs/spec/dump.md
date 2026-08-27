# The `spine index --dump` format

**Artifact:** the canonical byte stream `spine index --dump` writes to stdout — a total, ordered serialization of the traceability graph's ledger-derived projection. Two of them, byte-compared, are G10.
**Home in the playbook:** PB §6.3 G10 (the comparison), PB §6.1 (the iron rule and the provenance law), PB §6.2 (the schema and its derivation table), PB §5.4 step 5 (where G10 runs).
**References:** `PB §n` cites `PLAYBOOK.md`; a bare `§n` cites this document. The two numbering schemes collide — PB §6.2 is the graph schema, §6.2 is the node sort key — so every citation says which.
**Spec version:** 1 · **Dump format version:** 1 · **Graph schema version:** 7 (`PRAGMA user_version`) · **Covers:** PLAYBOOK.md v0.19 · **Status:** normative for v1.
**Normative dependencies:** `gate-report.md` §2.3 (the `esc` encoding, adopted here verbatim) and §6.1–§6.2 (wire entries and their order, for the one attr that carries them). `import-resolver.md` and `intent-doc.md` fix inputs to the *derivation* this document serializes; §16 says what that costs.

---

## 1. What this artifact is, and what rests on it

G10 clones the repository, indexes both sides, dumps both, and **refuses the push on a non-empty diff** (PB §5.4 step 5, PB §6.3 G10). That refusal is terminal: the run ends `reconstruction-failed`, the candidate `L` is discarded and never becomes a git object, the run is not re-queued, no `C-M3` retry is consumed, and break-glass cannot bypass it — G10 is not in PB §7.6's list and could not be, since `L` already exists and its seal covers its own message.

So the dump is the only artifact in spine-kit whose *format* can fail a landing. Every other artifact is checked against a signature or a digest; this one is checked against another copy of itself. A difference of one byte — a key emitted in insertion order, an integer with a leading zero, a path spelled NFD on one side and NFC on the other, a `git blame` heuristic that changed between git releases — is indistinguishable from a corrupted ledger and produces the same terminal refusal. PB §6.3 fixes the sort order in one clause and the serialization not at all. This document fixes both.

**The dump is a projection of the graph, not the graph.** The store (`.spine/cache/graph.sqlite`, PB §6.2) holds in-flight intents, provisional changesets, volatile test results and the shipped floor, because `spine check`, `spine context` and the drift gates need them. The dump holds only what a fresh clone of trunk can rederive. PB §6.3 says why in one sentence: *"G10 proves the ledger, not the lease registry."*

**Nothing in spine ever reads a dump.** G10 compares two byte strings; it never parses one into nodes and edges, and no fact anywhere in spine is derived from a dump's content. That is what keeps PB §6.1's rule intact — *"a rendering that spine reads back is a graph, and the law then applies to it in full — which is why nothing in spine reads one"* — while still letting G10 use it. A byte comparator is not a reader. The dump nonetheless obeys the provenance law in full: every record carries its `src` verbatim in PB §6.1's grammar (§5.4), because PB §6.1 requires that of *every* rendering, read back or not.

**A dump is never a git object, never a note, never fetched.** It is written to stdout and consumed in the same process tree that produced it. PB §7.4 rule 4's published gate report rides `refs/notes/spine`; the dump rides nothing. The G10 clone is made with `--no-local --no-hardlinks`, `GIT_CONFIG_GLOBAL=/dev/null`, no network and default refs only, so there is no channel by which one side's dump could reach the other.

---

## 2. Serialization

### 2.1 The scheme, by name

A dump is **JSON Lines**: a sequence of records, one per line, each record serialized in its **RFC 8785 JSON Canonicalization Scheme (JCS)** form, restricted by the value profile of §2.3 and the `esc` encoding of §2.4.

**Why JCS, and why the same JCS.** `gate-report.md` §2.1 already settled canonical JSON for spine-kit and gave the reasons — an IETF-published specification, cross-tested implementations in every language spine will ship a verifier in, and a definition in terms of a parsed value rather than of source text. Those reasons hold here unchanged, and a second scheme would be a second thing to get wrong: an implementer who has built a JCS serializer for the gate report reuses it byte for byte. Alternatives were refused for the same reasons `gate-report.md` refused them, plus one specific to this artifact: canonical CBOR would make a G10 failure undiagnosable, and the first thing anyone does with a failed reconstruction is `diff` the two dumps by eye.

**Why JSON Lines rather than one JSON document.** Three reasons, in order of weight. (1) A dump is unbounded — it grows with the ledger, not with one landing — and a single document forces both sides to hold the whole value in memory to canonicalize it, where a line-oriented stream can be produced by an external sort. (2) G10's failure is terminal and its only artifact is the run's own report, so the diff has to be readable: a line diff of two sorted JSONL streams names the record that differs; a character offset into a 40 MB JSON document does not. (3) `result-file.md` §4.1 already frames the collector's output as JSONL, so the house has one framing, not two.

The record grammar is closed: exactly three record kinds, distinguished by the `t` member — `"header"`, `"node"`, `"edge"`. No other value of `t` is legal, and an unknown member name inside a known record kind is not tolerated (§3.2).

### 2.2 Framing

1. The byte stream is a sequence of **lines**, each terminated by exactly one `0x0A` (LF). The final line is terminated too, so the stream ends with `0x0A`. No CR anywhere, no BOM, no blank lines, no comments, no trailing blank line.
2. Line 1 is the **header** record (§3.1). It is present in every dump, including an empty one (§9).
3. Lines 2 … *m* are the **node** records, in the order of §6.2.
4. Lines *m*+1 … *n* are the **edge** records, in the order of §6.3.
5. Nothing follows. A dump has no footer, no count and no digest of itself (§10 rule 11).

`spine index --dump` writes exactly these bytes to **stdout** and nothing else to stdout. Diagnostics, warnings and progress go to stderr, which is not part of the artifact. A dump redirected to a file is byte-identical to the stream, so `sha256sum` over the file reproduces the digest of §2.5.

### 2.3 The value profile

JCS's hard corners are floating-point serialization and UTF-16 code-unit ordering of non-ASCII member names. A dump never reaches them:

| Restriction | Rule |
|---|---|
| Member names | Match `^[a-z][a-z0-9_]*$`. ASCII only, so JCS's UTF-16 ordering reduces to byte ordering. The complete set is fixed by §5: `attrs`, `dump_version`, `from`, `head`, `id`, `kind`, `object_format`, `repo`, `schema_version`, `src`, `t`, `to`, `trunk`, `trust_root`, plus the attr names of §7.2. |
| Numbers | Integers only, `0 ≤ n ≤ 2^53 − 1`. No sign, no leading zero, no fraction, no exponent, no `-0`. There is no floating-point value anywhere in a dump. |
| Strings | ASCII only after `esc` (§2.4): every character is in `U+0020 … U+007E`. |
| Booleans | `true` and `false` are values; they appear only where §7.2 names them. |
| Null | Never emitted. An absent value is an absent member (§10 rule 6). |
| Duplicate names | Invalid. |
| Arrays | Elements are strings only. Order is fixed per attr by §7.2. |
| Depth | Exactly two: a record object, whose `attrs` member is an object of scalars and string arrays. **No attr value is an object.** |

Under this profile JCS reduces to: sort each object's members by member-name bytes ascending; emit with no whitespace; emit integers in plain decimal; emit strings with JSON's minimal escaping (`"` → `\"`, `\` → `\\`, nothing else can occur); output UTF-8, which is here also ASCII.

**Implementation note, not normative.** For this profile, `json.dumps(obj, sort_keys=True, separators=(',',':'), ensure_ascii=False).encode('utf-8')` is byte-identical to JCS. It is not JCS in general — floats and non-BMP member names diverge — which is exactly why the profile exists. `gate-report.md` §8.3 publishes a verified minimal canonicalizer vector; **debug against that one before attempting §12**, since it is the same scheme and it is already reproduced.

### 2.4 Byte-valued data: `esc`

Repository paths are byte strings. Git does not require them to be UTF-8; macOS filesystems disagree with Linux ones about normalization; principals and ref names are bytes too. JSON has no byte-string type.

**Every value in a dump that carries repository bytes or human bytes is encoded with `esc` as defined in `gate-report.md` §2.3, and is thereafter pure ASCII.** That definition is normative and is not restated here; a divergence between the two documents is a defect in `gate-report.md`, which owns it. As a reminder only: `0x5C` becomes `\` `\`; `0x20`–`0x7E` other than `0x5C` pass through; every other byte becomes `\` `x` and two **lowercase** hex digits.

`esc` applies to: every node `id`, every `from` and `to`, every `src`, the header's `repo` and `trunk`, and every attr whose §7.2 row says *bytes*. It does **not** apply to object ids, integers, booleans, or the closed enumerations of §7.2, which are already ASCII and for which `esc` is the identity — applying it changes nothing, so an implementation may apply it uniformly.

**The `tok` variant of `gate-report.md` §6.2 is not used here.** `tok` exists because a `Spine-Review` trailer is a space-delimited line of `key=value` fields whose `wires=` value is comma-separated. A dump has no such framing: a JSON string carries a comma, a space and a quote without help. The one attr that carries wire tokens — `approval.wires` (§7.2) — carries them as `tok` produced them, because those are the bytes the signed line contains and the dump records what the ledger says, not a re-derivation of it.

**Nothing is ever normalized.** No NFC, no NFD, no case folding, no separator rewriting, no path cleanup. This matches PB §3.3's canonical-form rule for the intent doc and `result-file.md` §4.4's rule for collected paths, and it is the reason a dump taken on macOS and a dump taken in a Linux container agree. Where a gate itself casefolds — G14 casefolds before floor comparison (PB §7.3) — the dump records the path **as the tree spells it**, never the casefolded form.

**Paths are the tree's bytes, never the filesystem's.** A `code_unit` id is built from the repo-relative, `/`-separated path exactly as git stores it in the tree entry. `result-file.md` §4.4 states the same rule for the collector and gives the same reason: a macOS runner reports NFD where git stores NFC, and a path that names nothing is worse than no path at all.

### 2.5 The dump digest

The **dump digest** is `sha256:` + 64 lowercase hex digits over exactly the byte stream of §2.2 — including the final LF, excluding nothing. It is a non-git artifact, so PB §11's hash policy makes it SHA-256 (`gate-report.md` §7 rule 10).

The digest is a convenience for G10 and for humans. **It is never sealed, never signed, never a trailer field, and never a member of a gate report.** G10's comparison is defined over the bytes (§11); a digest comparison is an implementation of it and is permitted precisely because SHA-256 collision resistance makes the two equivalent.

---

## 3. Identity and versioning

### 3.1 The header record

Exactly one, always line 1.

```json
{"dump_version":1,"head":"<oid>","object_format":"sha1","repo":"<esc>","schema_version":7,"t":"header","trunk":"<esc>","trust_root":"<oid>"}
```

| Member | Type | Presence | Value |
|---|---|---|---|
| `dump_version` | integer | always | `1`. This document defines version 1. |
| `object_format` | string | always | `"sha1"` \| `"sha256"` — the **indexed repository's own** format (`extensions.objectFormat`; absent means `sha1`). Fixes every oid's length at 40 or 64 lowercase hex. |
| `repo` | string, bytes | always | The manifest's `repo`, `esc`-encoded — the prefix of every node id (PB §6.2). |
| `schema_version` | integer | always | `7` — PB §6.2's `PRAGMA user_version` (§3.3). |
| `t` | string | always | `"header"`. |
| `trunk` | string, bytes | always | The resolved trunk **branch name** (not a full refname), `esc`-encoded (§4.2). |
| `head` | string | iff `refs/heads/<trunk>` resolves | Its full oid. Absent means the derivation had no trunk to walk, and the dump is empty (§9). |
| `trust_root` | string | iff `spine.trustRoot` is configured | Its full oid. |

**Why `head` is in the artifact.** PB §6.1 requires that a rendering which ships be *"datable against the ledger it came from"*, and one oid does it. It also converts the most confusing possible G10 failure into the most legible one: if the two sides ever index different tips, the diff is line 1 with an obvious cause, instead of a thousand-line body diff with none.

**Why `trust_root` is in the artifact.** The chain walk of PB §7.5 decides which signer nodes exist, which `verified` and `seal_verified` attrs are `true`, and which landings index `unattested`. It is therefore an input to the dump, and it is not carried by the objects — it is a git config value naming an object. Recording it makes a trust-root mismatch a line-1 diff. **PB §6.3 G10 as written copies the pinned trust root into the *clean clone* only; the scratch clone `S` gets no `spine.trustRoot`, and git config is not copied by `git clone`.** With no trust root the chain walk has no root, so the two sides disagree — see §14, defect D1. This document makes that disagreement loud rather than silent; it does not fix it, because the fix is one clause of PB §6.3.

### 3.2 Reading an unknown version

There is no reader. Nothing in spine parses a dump (§1). The rule is therefore stated for the only two consumers that exist:

- **G10** compares dumps produced by one binary in one process tree. A version mismatch cannot arise; if an implementation ever observes one, it is a defect in that implementation and the run refuses with `dump-version-skew`, exit 3, rather than comparing.
- **A human or an external tool** that meets an unknown `dump_version`, or an unknown member name inside a version it knows, **refuses**. The schema is closed: forward compatibility is bought with a version bump, not with tolerance, for the same reason `gate-report.md` §3.2 gives — a tolerant reader and a strict one produce different bytes over the same document, and the whole artifact is compared by bytes.

A binary keeps a serializer for every `dump_version` it has ever shipped, the same promise PB §6.7 makes for template and envelope versions.

### 3.3 `dump_version` and `schema_version` are two facts

`schema_version` is PB §6.2's `PRAGMA user_version` — the *store's* schema. `dump_version` is this document's — the *projection's*. They move independently, and both are recorded because either one changing can change the bytes:

- a store-schema change that adds an excluded attr changes `schema_version` and not `dump_version`;
- a projection change — a new exclusion, a new tie-break, a renamed member — changes `dump_version` and not `schema_version`;
- a store-schema change that adds an *included* element changes both.

A dump whose `schema_version` differs from the running binary's constant is not comparable with one that matches: PB §6.2 already says such a cache *"is never queried — it is rebuilt"*. Since `--dump` implies `--fresh` (§4.3), a dump always reports the running binary's constant, and a difference in this member between two G10 sides means two binaries ran, which §10 rule 2 forbids.

### 3.4 Why the producing release is not in the dump

`cli.version` and `cli.dist_hash` are **not** header members, and neither is any other name for the binary that produced the dump.

The reason is a requirement, not an omission: two releases carrying the same `dump_version` and `schema_version` **must** produce identical bytes over identical objects. That is what this document requires of them. Recording the release would let a genuine divergence hide behind a version difference — "of course they differ, one was 1.4.0" — and would break the only cross-machine use a dump has, which is one person checking another's clone. PB §6.7's skew table already deletes and rebuilds a store built by another binary; the projection needs no second spelling of that rule.

The consequence is stated plainly: a release that changes the projection **must** bump `dump_version`, even for a change it believes is a bug fix. A silent projection change is a fleet-wide `reconstruction-failed` on the first landing after a rolling upgrade, and the report will name the graph, not the release.

---

## 4. The closed input set

### 4.1 One ref

> **A dump is a function of exactly four things: the trunk tip's oid, the git objects reachable from it, the trust root, and the pinned release. Nothing else may influence one byte.**

This is the whole of §8's exclusion set in one sentence, and every clause of PB §6.3's G10 phrase — *"provisional (in-flight) elements, † states (dumped as `tests-approved`), volatile test results and worktree-only files excluded"* — is a corollary of it (§8.1).

It is also what makes G10 mean anything. The scratch clone `S` and the clean clone of `S` differ in exactly the ways a clone differs from its source: `S` was cloned from the origin repository, so it holds `refs/remotes/origin/intent/*` for every intent still in flight; the clean clone was made from `S`'s *local* heads only, so it holds none of them. Any element derived from an intent branch is present on one side and absent on the other, and G10 fails on every landing made while any other intent is open. PB §6.3 anticipates this and says so: *"G10 proves the ledger, not the lease registry."*

### 4.2 Trunk resolution

`spine index` resolves the trunk branch name, in this order, and records the result in the header:

1. an explicit `--trunk <name>`, when the CLI offers one;
2. `params.trunk` in `.spine/manifest.json` in the tree of the commit `HEAD` resolves to;
3. `params.trunk` in the manifest of the newest first-parent ancestor of `HEAD` whose tree carries one — this is the case for the range between an `--uninstall` landing and the next `init` (PB §6.7), where the tip carries no manifest but the history is still a ledger;
4. none: the repository is not a spine repository, and `--dump` refuses (§4.4).

Steps 2 and 3 read a **tree**, never the working directory, so a bare repository resolves identically to a checked-out one. `head` is then whatever `refs/heads/<trunk>` resolves to in this repository, and is absent when that ref does not exist (§9).

Trunk resolution properly belongs to the indexer, not to a serialization format. It is fixed here because the header records its result and §4.1 makes it an input; if a later `cli.md` states it, that document wins and this section becomes a citation.

### 4.3 What is never an input

| Not an input | Why |
|---|---|
| Any ref but `refs/heads/<trunk>` | §4.1. Includes `refs/heads/intent/*`, `refs/remotes/*/*`, tags, `refs/notes/*` and every provider ref. The clean clone has a different set of them by construction. |
| `refs/notes/spine` | PB §7.4 rule 4: publication is required, reading is forbidden, and a note is never a source. The G10 clone fetches default refs only, so the note is not even present. |
| The working tree, the index, `git status` | §8.7. A bare repository dumps identically. |
| A persisted, fetched or restored store | PB §7.4 rule 3. `--dump` implies `--fresh`: the projection is computed from a graph built in this process from git objects alone. Whether the store file is also written is unspecified and cannot affect one byte. |
| The collector's result file | `result-file.md` §2: it populates `test.result_at` only, which §8.4 excludes. A result file can never affect reconstruction. |
| A coverage report | The v1.1 `exercises` edge (§8.3). |
| Any wall clock | §10 rule 1. Committer dates are objects and may be *read* — G3's staleness comparison reads one (`gate-report.md` §9.8) — but no date, duration or ordinal derived from one is a value in a dump, and `params.timeout` (PB §6.7) never appears: it bounds a runner invocation, and the runner's output is excluded. |
| The environment | §10 rule 2. No hostname, user, locale, temp path, process id, or path outside the repository. |
| Repository, user or system git config | Beyond `extensions.objectFormat` and `spine.trustRoot`, both of which the header records. Every git plumbing invocation the derivation makes runs with its diff algorithm and rename detection pinned by the release, never read from config (§10 rule 12). |

### 4.4 Exit codes

| Exit | Status | Meaning |
|---|---|---|
| 0 | `ok` | The dump is on stdout. |
| 2 | `not-installed` | Trunk resolution reached step 4 of §4.2. Nothing is written to stdout. |
| 3 | `provenance-invalid` | The derivation produced a `src` outside PB §6.1's grammar (§5.4). |
| 3 | `id-out-of-grammar` | The derivation produced a node id outside §5.2. |
| 3 | `attrs-out-of-profile` | An attr value outside §2.3 or §7.2 — a float, a `null`, a nested object, an unknown name. |
| 3 | `dump-version-skew` | G10 only: two dumps with different `dump_version` or `schema_version` were offered for comparison (§3.2). |

Exits 3 are internal-consistency refusals: the derivation produced something this format cannot represent, and emitting a partial dump would produce a spurious terminal G10 failure with a misleading diff. Refusing loudly, in the same process that built the graph, names the defect instead.

---

## 5. Records

### 5.1 Node records

```json
{"attrs":{…},"id":"<esc>","kind":"<kind>","src":"<esc>","t":"node"}
```

| Member | Type | Presence | Value |
|---|---|---|---|
| `attrs` | object | always | §7. `{}` when the kind has none; never absent. |
| `id` | string, bytes | always | §5.2. Unique across the whole node section — the store makes it `PRIMARY KEY` (PB §6.2). |
| `kind` | string | always | One of `ac`, `adr`, `approval`, `changeset`, `code_unit`, `constitution`, `intent`, `signer`, `test`. The set is closed and is PB §6.2's. |
| `src` | string, bytes | always | §5.4. |
| `t` | string | always | `"node"`. |

### 5.2 The node id grammar, by kind

Every node id is `<repo>` + `/` + a per-kind local id, where `<repo>` is the manifest's `repo` (PB §6.2: *"the prefix comes from the manifest's `repo`, while trailers carry the bare id"*).

| kind | local id | example |
|---|---|---|
| `intent` | the bare trailer id: `INT-<n>` or `BUG-<n>` | `myrepo/INT-042` |
| `ac` | `<intent local id>/AC-<n>` | `myrepo/INT-042/AC-1` |
| `test` | `test:` + `<runner>` + `:` + `<runner-native function id>` | `myrepo/test:pytest:tests/billing/test_invoice.py::test_AC1_totals` |
| `code_unit` | `code:` + `esc(path bytes)`; a trailing `/` means a directory | `myrepo/code:src/billing/tax.py` |
| `changeset` | `cs:` + the commit's **full** oid | `myrepo/cs:1b2c…6789` |
| `approval` | `approval:` + 64 lowercase hex (§5.2.1) | `myrepo/approval:c428…5597` |
| `signer` | `signer:` + `esc(principal bytes)` | `myrepo/signer:alice@example.com` |
| `adr` | the ADR's own id, as its heading spells it | `myrepo/ADR-007` |
| `constitution` | `constitution:v<n>` | `myrepo/constitution:v3` |

Three rules make this total:

- **`<repo>` matches `^[A-Za-z0-9._-]+$`.** A manifest whose `repo` does not is refused (`id-out-of-grammar`). Without it, `myrepo/INT-042/AC-1` is ambiguous. This constraint was written here while no manifest spec existed; **`manifest.md` §3.1 now owns it and adopts it unchanged**, adding a 1…64-byte length bound and the refusal code `repo-out-of-grammar` (`manifest.md` §14 R18 records the handover). Nothing here moves.
- **A `test` id is qualified by its runner, and the pair is the identity** (PB §4.3, settled 2026-08-26). A repository may run several runners (`params.langs`); two runners collecting the same function id are two nodes, and merging them would let one runner's rename silently satisfy another's coverage. `<runner>` contains no `:` — a constraint on the per-runner specs, not on this document — so `test:` + runner + `:` delimits without a parse. Nothing in spine parses the id; the constraint exists so that a human reading a diff can.
- **Oids are full, lowercase, never abbreviated.** PB's `cs:abc123f` is display. `gate-report.md` §7 rule 9 says the same of every oid in a report, and for the same reason: an abbreviation's length is a function of the repository's object count.

#### 5.2.1 The `approval` local id

`approval:` + the SHA-256, as 64 lowercase hex digits, of **the exact bytes of the signed trailer line as the commit message carries it** — from the first byte of the trailer name (`Spine-Approve`, `Spine-Signoff`, `Spine-Review`, `Spine-Reopen`, `Spine-Withdraw`, `Spine-Upgrade`) through the last byte before its terminating LF, with no LF included.

This is defined here, independently of what a `-Sig` line covers, so that it does not move when `envelope-vectors.md` fixes the signed payload. Uniqueness is G13's: *"an event line byte-identical to an earlier one on the branch … is refused"* (PB §6.3), so two distinct approvals cannot collide. That refusal is **outright** — no review discharges it — which is what makes it safe to key a node on: `manifest.md` §4.8.4 check 3, which cites this section as one of the two dependencies that force the reading.

PB §6.2's example, `approval:5c9e…`, is consistent with this and with a second reading — that the id is the approve line's `freeze=` digest, which PB §4.3 calls *"a non-git digest, used to name the approval elsewhere"*. That reading is rejected because it is total over only one of the six `event` values: a sign-off, a review, a reopen, a withdrawal and an upgrade have no freeze digest. `freeze=` is carried instead as the `freeze` attr (§7.2), which is what a `Spine-Reopen`'s `voids=` joins against.

### 5.3 Edge records

```json
{"attrs":{…},"from":"<esc>","kind":"<kind>","src":"<esc>","t":"edge","to":"<esc>"}
```

| Member | Type | Presence | Value |
|---|---|---|---|
| `attrs` | object | always | §7. `{}` when the kind has none. |
| `from` | string, bytes | always | A node id present in the node section. |
| `kind` | string | always | One of `approves`, `attested_by`, `built_under`, `declares`, `exercises`, `freezes`, `has_ac`, `implements`, `modifies`, `protects`, `reverts`, `signed_by`, `supersedes`, `superseded_by`, `verified_by`. The set is closed and is PB §6.2's; `exercises` is never emitted in v1 (§8.3). |
| `src` | string, bytes | always | §5.4. |
| `t` | string | always | `"edge"`. |
| `to` | string, bytes | always | A node id present in the node section. |

**No dangling edges.** Every `from` and every `to` names a node record in the same dump. PB §6.1 makes this the linter — *"in a derived graph, dangling edges are the linter"* — and G5 is the gate that enforces it over the store. A dump containing one is non-conforming, and the failure surfaces as `id-out-of-grammar` only if the id is malformed; a well-formed id naming no node is an indexer defect that this format cannot detect and G5 must.

**Direction, where PB leaves it implicit.** `verified_by` runs **test → ac**: PB §6.3 G5 fails on *"a `verified_by` edge to a nonexistent AC (typo'd pragma)"*, so the AC is `to_id`. `approves` runs approval → intent, or approval → `cs:<L>` for a line carrying no id (PB §6.2). `signed_by` runs approval → signer. `attested_by` runs the landing changeset → the seal's signer. `freezes` runs approval → code_unit (with `oid`) or approval → test. `protects` runs constitution → code_unit (§8.3). `implements` runs changeset → intent. `modifies` runs changeset → code_unit. `built_under` runs intent → constitution. `has_ac` runs intent → ac. `declares` runs intent → code_unit. `reverts` runs the reverting landing → the reverted landing.

### 5.4 `src` — the provenance law, made mechanical

PB §6.1: *"every node and edge must cite its source. An edge that cannot say where it came from does not exist."* The dump carries that citation verbatim, in PB §6.1's grammar and no other:

| Production | Shape | Used for |
|---|---|---|
| file line | `<path>:<line>` | a line of a file in the working tree — **never emitted by a dump** (§8.7); the production exists in PB §6.1 and is listed for completeness |
| commit | `git:<sha>` | a whole commit — `modifies` from that commit's diff, a member changeset |
| message line | `git:<sha>:msg:L<n>` | a line of the envelope's fenced intent bytes — intent, ac, `has_ac`, `declares`, `built_under` |
| trailer | `git:<sha>:trailer:<Name>` | a signed line — approval nodes, `approves`, `signed_by`, `attested_by`, `freezes`, the landing changeset |
| patch id | `git:<sha>:patch-id` | `reverts` |
| file line at a commit | `git:<sha>:<path>:<line>` | test nodes, `verified_by`, signer nodes, `protects` from `C-A2`, adr and constitution nodes |
| shipped floor | `spine:<version>:floor` | the release's floor list — **never emitted by a dump** (§8.3) |

`<sha>` is a full oid at `object_format`'s length. `<line>` and `<n>` are decimal integers ≥ 1 with no leading zero. `<path>` is `esc(path bytes)`. `<Name>` is a trailer name.

A derivation that would emit a `src` outside this grammar refuses the dump (`provenance-invalid`, exit 3). This is PB §6.1's law applied to itself: an element whose citation is unrepresentable does not exist, and a dump that quietly drops it produces a G10 diff whose cause is invisible.

Two productions in that grammar are defective and are reported rather than repaired: §14 D2 (a trailer citation cannot distinguish the second of two `Spine-Review` lines) and §14 D3 (the grammar is not unambiguously parseable). Neither affects a dump, because nothing parses a `src` — they affect any *other* tool that reads a rendering, which PB §6.1 explicitly contemplates.

### 5.5 Multiply-derived elements

The store gives a node one `src` and one row (PB §6.2: `id TEXT PRIMARY KEY`). Derivations produce several citations for the same element — a `code_unit` is named by every edge that touches it; a changeset is both a landing and a member of nothing.

> **When a derivation produces the same element from more than one citation, the dump emits one record whose `src` is the minimum, under §6.4's ordering, of those citations.**

Applied to nodes, this is total: `code_unit` nodes take the minimum `src` over the edges naming them; every other kind has one citation by construction. Applied to edges, records that are equal in `from`, `to`, `kind` and `attrs` collapse to one with the minimum `src`; records that differ in `attrs` are two edges and both are emitted (§6.3).

The rule fixes the *choice* deterministically given the citation set. It cannot fix the citation set itself, which PB §6.2's derivation table owns — two implementations that derive the same graph from different citations still differ. That residual is real and is named in §13.5 rather than hidden.

---

## 6. The total order

PB §6.3 fixes two clauses: *"nodes sorted by kind,id, edges by from,to,kind"*. This section keeps both as written and supplies the tie-breakers and the comparison PB left open. **No key below reorders anything PB fixed**; each is appended beneath it.

### 6.1 Sections

Nodes first, then edges. The header is line 1 by framing (§2.2), not by sort.

### 6.2 The node key

For a node record `r`, with `‖` denoting concatenation and `NUL` the byte `0x00`:

```
key_node(r) = r.kind ‖ NUL ‖ r.id ‖ NUL ‖ canonical(r.attrs) ‖ NUL ‖ r.src
```

The node section is every node record sorted ascending by `key_node`. `canonical(r.attrs)` is the JCS serialization of the `attrs` object, the same bytes the record carries.

`kind` before `id` is PB §6.3's order and is preserved even though `id` alone is unique — the two orders differ, and PB's is the one G10 was specified against.

### 6.3 The edge key

```
key_edge(r) = r.from ‖ NUL ‖ r.to ‖ NUL ‖ r.kind ‖ NUL ‖ canonical(r.attrs) ‖ NUL ‖ r.src
```

The edge section is every edge record sorted ascending by `key_edge`, after the collapse of §5.5.

`attrs` is a tie-breaker rather than part of the identity because PB §6.2 gives no uniqueness constraint on `edges` at all, and two edges alike in `from`, `to` and `kind` but differing in `attrs` are representable — a `declares` edge naming one path as both `expected` and `forbidden`, for instance, which is a malformed intent doc and not this format's to refuse. Ordering by `attrs` makes such a pair deterministic instead of arbitrary.

### 6.4 Comparison, and what "byte order" means after `esc`

Every component of both keys is a string over `U+0020 … U+007E` (§2.3, §2.4). Comparison is **ascending over those bytes**, unsigned, with the shorter string first when one is a prefix of the other. Because the alphabet is ASCII, byte order, code-point order and UTF-16 code-unit order coincide, so JCS's ordering rule and this one are the same rule.

**`NUL` makes the concatenation faithful.** No component can contain `0x00`: `esc` maps it to the four characters `\x00`, and a JCS-serialized `attrs` is JSON text. So comparing the concatenations is exactly comparing the components in order, and the classic separator hazard — `a/b` sorting against `a-b` — cannot arise. An implementation may compare field by field instead; the results are identical, and the concatenation is given because it is checkable.

**The order is over the `esc`-encoded bytes, not the raw path bytes, and the two differ.** `esc` moves every byte above `0x7E` into a sequence beginning with `\` (`0x5C`), which sorts *below* every lowercase letter. So for the two paths `src/z.py` and `src/` + `0xE9` + `.py`:

- raw bytes: `src/z.py` first, because `0x7A < 0xE9`;
- `esc` bytes: `src/\xe9.py` first, because `0x5C < 0x7A`.

**The encoded order governs**, for one reason that decides it: those are the bytes in the artifact, so the dump is sorted with respect to itself, and a reader can verify the order from the file without decoding it. §12.4 publishes this exact pair as a vector.

**Numeric-looking ids sort as bytes.** `AC-10` precedes `AC-2`; `G11` precedes `G2`. This is deliberate: a byte order over `esc` output is the one order every implementation already has, and nothing in a dump is signed, so no signature depends on it. PB §11 now fixes the same direction for a signed `wires=` line — *"ascending by unsigned byte value over the whole token, so `G11` precedes `G2`"* — so the two orders agree, and an implementation may share one comparator. They agree by rule and not by dependence: this document's order is defined over `esc` output for the reason above and would not move if PB §11's ever did. (`gate-report.md` §6.1 and §6.2 have since adopted the same byte order, so the divergence that stood here is closed and all three documents now sort wires identically.)

### 6.5 Totality, and why a dump is not `sort`-stable

`key_node` is total on node records: `id` alone is unique, so no tie survives. `key_edge` is total on edge records after §5.5's collapse: two records with equal keys are the same record.

A dump is **not** a fixpoint of `LC_ALL=C sort`, and no conformance check should assume it is. JCS orders a record's members alphabetically, so every node line begins `{"attrs":` and every edge line begins `{"attrs":` — the sort key is not a prefix of the line. Two consequences worth stating because an implementer will otherwise discover them the hard way: an empty `attrs` sorts *after* a non-empty one at the line level (`}` is `0x7D`, `"` is `0x22`), and a line sort would interleave the two sections. Sort by the key of §6.2 and §6.3, never by the line.

---

## 7. `attrs`

### 7.1 The value profile of an attr

An attr value is a **string**, a non-negative **integer** in `[0, 2^53 − 1]`, a **boolean**, or an **array of strings**. Never an object, never `null`, never a number outside that range. An attr name matches `^[a-z][a-z0-9_]*$` and comes from the closed set of §7.2; anything else refuses the dump (`attrs-out-of-profile`, exit 3).

**`attrs` is always present.** A kind with no attrs emits `{}`. `{}` is a value; an omitted `attrs` member is not a legal record. This removes the whole "did you emit `{}` or omit it" divergence class, which is precisely the class G10 punishes terminally.

**An attr that names a git object carries the oid; a reference to another node is an edge, never an attr.** `intent.landing` is `L`'s oid, not `myrepo/cs:<L>`. Joins in the graph are edges — that is what the schema is for — and an id inside an attr is a second spelling of an edge that will disagree with it.

### 7.2 Per kind

PB §6.2's schema gives attrs for five node kinds and four edge kinds and gives none for the rest. **A kind PB §6.2 does not give attrs for has none in the dump**: `ac`, `adr`, `code_unit` and `constitution` nodes carry `{}`, as do `approves`, `attested_by`, `built_under`, `has_ac`, `modifies`, `signed_by`, `supersedes` and `superseded_by` edges. An implementation that wants to store an AC's text may; the dump does not carry it, and G10 does not compare it.

**Nodes.**

| kind | attr | type | presence | value |
|---|---|---|---|---|
| `intent` | `status` | string | always | `merged` \| `withdrawn` \| `reverted` \| `superseded` (§7.3) |
| | `owner` | string, bytes | iff the doc has an `Owner:` field | the field's value, `esc`-encoded — a hint, never authority (PB §3.1) |
| | `title` | string, bytes | always | the `# INT-042: <title>` heading's title, read from the sealed intent inside the landing commit's message — **never from that commit's subject line**. PB §11 derives a gated landing's subject from the very line this attr is read out of — *"the fenced intent's first line with its leading `# ` removed"*, which for a conforming doc is `<id>: <title>` — and PB §5.5 has G9 recompute it and refuse a subject it did not produce, so reading the subject back would be a second spelling of this attr with the id glued on; and on a quick-lane landing, which every toolkit lifecycle landing is, the subject is free text and there is no intent node at all. |
| | `template` | string | always | the `Template:` value — the variant and the version, e.g. `intent@2` (PB §3.4) |
| | `blob` | oid | always | the signed intent blob |
| | `signer` | string, bytes | iff a `Spine-Signoff` is copied into the landing | the sign-off's principal |
| | `reopen_count` | integer | always | copied `Spine-Reopen` lines |
| | `late_reopen_count` | integer | always | of those, the ones after the binding approval |
| | `landing` | oid | always | `L` |
| | `base` | oid | always | the seal's `base=` |
| `changeset` | `landing` | boolean | always | `true` for a sealed trunk commit, `false` for a member |
| | *(the rest)* | | iff `landing` is `true` | `lane`, `event`, `strategy`, `base`, `head`, `tree`, `seal_principal`, `seal_verified`, `report_sha256`, `threat`, `profile`, `tool_version`, `git_version`, `mode`, `unattested`, `resealed` — all from the seal (§7.2.1). A member changeset carries `{"landing":false}` and nothing else: it has no seal, and every one of those fields is a seal field. |
| `approval` | `event` | string | always | `signoff` \| `approve` \| `review` \| `reopen` \| `withdraw` \| `upgrade` |
| | `role` | string | always | `signer` \| `reviewer` \| `pipeline` — **the namespace the signature verified under**, never a claim in the trailer (PB §4.3, PB §7.2). A v1 approve line signed under `spine-review@v1` is `reviewer`. |
| | `principal` | string, bytes | always | the `signer=` / `reviewer=` value |
| | `verified` | boolean | always | the `-Sig` verified against the keyring at the seal's `base=` |
| | `blob` | oid | iff the line carries `blob=` or `intent=` | |
| | `base`, `head`, `tree` | oid | iff the line carries them | |
| | `class` | string | iff `event` is `review` | `tripwire` \| `protected` \| `break-glass` |
| | `rounds`, `total_rounds`, `reopens` | integer | iff the line carries them | |
| | `red` | string | iff `event` is `approve` | `"k/n"` |
| | `freeze` | string | iff `event` is `approve` | `sha256:<hex>` |
| | `wires` | array of strings | iff `event` is `review` | the `wires=` tokens, **in the line's order**, which PB §11 fixes as ascending by unsigned byte value over the whole token (so `G11` precedes `G2`) — the same direction §6.4 sorts ids in. Not re-sorted here: the signed line's order is the fact, and a dump that re-sorted it would hide a non-conforming review rather than reproduce it. |
| | `voided_by` | oid | iff a copied `Spine-Reopen` voids this approval | the commit carrying that reopen |
| | `void_reason` | string, bytes | iff `voided_by` is present | that line's `reason=` |
| `signer` | `roles` | array of strings | always | the namespaces this key is listed under, ascending by bytes: a subset of `spine-review@v1`, `spine-seal@v1`, `spine-signoff@v1` |
| | `fingerprint` | string | always | as `ssh-keygen -lf` produces it (`SHA256:<base64>`) |
| | `valid_from` | oid | always | the trunk commit at which the key first appears in `.spine/allowed_signers` |
| | `valid_to` | oid | iff the key has been removed | the trunk commit at which it stopped appearing |
| `test` | — | | | `{}` always: `result_at` is the kind's only attr and §8.4 excludes it. |

**Edges.**

| kind | attr | type | presence | value |
|---|---|---|---|---|
| `declares` | `polarity` | string | always | `expected` \| `forbidden` |
| `implements` | `role` | string | always | `landing` \| `member` |
| | `provisional` | boolean | always | **`false` in every dumped record** (§8.2). A `true` is a conformance failure, and a cheap one to test for. |
| | `verified` | boolean | always | membership verified by G9's walk |
| `verified_by` | `attributed` | boolean | always | the pragma is in a blob the binding approval froze (PB §6.2) |
| `freezes` | `oid` | oid | iff `to` is a `code_unit` | the frozen blob. A `freezes` edge to a `test` carries `{}` — PB §6.2 says so. |
| `protects` | `floor` | boolean | always | **`false` in every dumped record** (§8.3): the shipped floor is excluded, so only `C-A2` entries remain. |
| `reverts` | `partial` | boolean | always | hunks missing inside `L`'s paths (PB §6.2) |

#### 7.2.1 `changeset.tree` and `changeset.git_version` — the one environment-dependent value

`git_version` is **the seal's `git=`**, never the indexing binary's own `git --version`. It is a fact about the landing, and reading the local git would put the environment in the artifact (§10 rule 2).

`tree` is `L`'s tree oid — normally the seal's `tree=`, which G9 verifies — but PB §6.3 G9 defines two sentinel values, and both are legal here:

- `unverifiable(squash)` — under `C-M1: squash`, `H` is unreachable by design and the tree rule is never consulted, *"so a source-side index and the G10 clone derive the same thing"*. Deterministic.
- `unverifiable(git-version)` — under `merge`, when the indexer's git differs from the seal's `git=` **and** the recomputed merge tree differs. This is a function of the indexing environment, not of the objects, and it is the single place a dump is not a pure function of §4.1's inputs.

The second is safe for G10 — both sides run one binary and one git on one host, so they agree — and unsafe for cross-machine comparison, where two people with different gits get different dumps of one repository. It is recorded rather than suppressed because suppressing it would replace a visible divergence with an invisible one, and because the sentinel is PB §6.3's own answer. It is **not** repaired by putting the local git version in the header: that would make every cross-machine dump differ, which is strictly worse. §13.7 records the choice; §15 OPEN-2 asks whether G9 should keep the sentinel at all.

### 7.3 Presence, and the status domain

`null` never appears; a member is present or absent, and every row above states the condition. Absence means *this concept does not apply to this element*, never *unknown* and never *empty* — the same rule as `gate-report.md` §7 rule 6.

`intent.status` is drawn from `merged`, `withdrawn`, `reverted`, `superseded` and nothing else. Three of PB §11's post-landing names are absent on purpose: `orphan`, `unattested` and `resealed` are properties of a **changeset** in PB §6.2's schema — its attrs list `unattested` and `resealed`, and the intent's does not — and a landing can be `unattested` while its intent is plainly `merged`. In-flight statuses (`draft` … `checked†`) cannot appear at all, because §8.2 excludes in-flight intents; an implementation that emits one is non-conforming (§17).

---

## 8. The exclusion set

### 8.1 The generating rule

PB §6.3 gives four adjectives: *"provisional (in-flight) elements, † states (dumped as `tests-approved`), volatile test results and worktree-only files excluded"*. That is not a specification — it does not say which node kinds and which edge kinds it reaches, and two implementers will draw the line in different places. This section enumerates it, and does so from one rule:

> **A graph element is in the dump if and only if it is derived from git objects reachable from the trunk tip. An element derived from anything else — an intent branch, the collector's result file, a coverage report, the binary's own floor list, or a heuristic over the objects rather than the objects — is excluded.**

Each of PB's four adjectives falls out: in-flight elements come from `refs/heads/intent/*` (§8.2); † states exist only while a gate record is live, and no gate record is a git object (§8.6); test results come from the result file (§8.4); worktree-only files come from no tree at all (§8.7). Three further exclusions fall out that PB did not name, and §8.5 argues each.

### 8.2 By node kind

| kind | in the dump | condition | excluded when |
|---|---|---|---|
| `intent` | **yes** | a first-parent trunk commit carrying `Spine-Seal` names it — a `Spine-Event: land` landing or a `withdraw` tombstone. Derived from the fenced intent bytes of that envelope, parsed by the parser for that `Template:` header's **variant and version** — `intent@2`, `intent-change@2`, `intent-bug@2` — since PB §3.4 makes the variant part of the header precisely because a parser is not decidable from a bare version (PB §6.2) | in flight. The branch is `refs/heads/intent/*`, which the clean clone does not have (§4.1) |
| `ac` | **yes** | its intent is included | its intent is not |
| `test` | **yes** | derived from a `Spine-Test` id or a pragma in a `Spine-Frozen` path of an included landing, parsed from `<L>:<path>` — the frozen blob, reachable through `L`'s tree forever (PB §6.2) | derived from a branch's test files before the landing |
| `code_unit` | **yes** | named as `from` or `to` by an included edge | named only by an excluded edge — in particular, a path that is on the shipped floor and nothing else (§8.3) |
| `changeset` | **yes** | a first-parent trunk commit carrying `Spine-Seal`, at or above the trust root; and every member of `M(L) = git rev-list B..L` for each such landing | in flight (`merge-base..branch`, which PB §6.2 calls provisional); below the trust root; inside an `--uninstall` → re-init range, which G9 exempts (PB §6.3 G9, PB §6.7) |
| `approval` | **yes** | its signed line is copied into an included landing's envelope | the line is on an in-flight event commit and has not landed. PB §6.2 already says an id-less line's `approves` edge is *"emitted only once the landing is indexed"*; this extends the same rule to the node |
| `signer` | **yes** | `.spine/allowed_signers` at every trunk first-parent commit from the trust root (PB §6.2). Purely trunk-derived, so always included | never |
| `adr` | **yes** | an ADR file present in `adr/` in the trunk tip's tree (PB §2.2 makes the folder append-only) | never, in practice; a deleted ADR is not a node |
| `constitution` | **yes** | every distinct version observed in the constitution's header on the first-parent walk from the trust root. Historical versions are nodes because landed intents carry `built_under` edges to them | never |

### 8.3 By edge kind

| kind | in the dump | note |
|---|---|---|
| `has_ac` | **yes** | |
| `declares` | **yes** | both polarities |
| `built_under` | **yes** | |
| `implements` | **yes** | `provisional` is `false` in every dumped record — a provisional edge is an in-flight changeset's, and §8.2 excluded the changeset |
| `modifies` | **yes** | the landing's `git diff --name-only B L`, and the per-member diffs PB §6.2 keeps for archaeology |
| `approves` | **yes** | |
| `signed_by` | **yes** | |
| `attested_by` | **yes** | |
| `freezes` | **yes** | to `code_unit` with `oid`, to `test` with `{}` |
| `verified_by` | **yes** | `attributed` kept, `introduced_by` excluded (§8.5) |
| `reverts` | **yes** | derived from `git patch-id --stable` over `L`'s paths (PB §6.2); see §10 rule 12 |
| `supersedes` | **yes** | |
| `superseded_by` | **yes** | |
| `protects` | **partly** | `C-A2` entries — derived from the constitution blob, a git object — are included, with `from` the `constitution:<v>` node and `floor: false`. **Shipped-floor entries are excluded** (§8.5) |
| `exercises` | **no** | PB §6.2 marks it optional and v1.1, and its source is a CI coverage report, which is not a git object. It is excluded now and stays excluded when it ships, under the rule of §8.1 — a coverage report has the same standing as a result file |

### 8.4 By attr

Two attrs are excluded from kinds that are otherwise included:

- **`test.result_at` `{tree, base, passed}`** — PB §6.2 marks it volatile and PB §6.3 G10 excludes it by name. `result-file.md` §2 states the consequence from the other side: the result file *"Populates the volatile `test.result_at {tree, base, passed}` attrs only (§6.2), on `test` nodes whose ids are `test:<runner>:<id>` (§6.2). G10 excludes those attrs from the canonical dump, so a result file can never affect reconstruction."* Since it is the kind's only attr, every `test` node in a dump carries `{}`.
- **`verified_by.introduced_by`** — §8.5.

### 8.5 Three exclusions this document adds, and why each is a nondeterminism

PB §6.3's four adjectives do not reach these. Each is a place where the dump would otherwise stop being a function of §4.1's inputs.

**1 · `verified_by.introduced_by`.** PB §6.2 derives it with `git blame` and says in the same clause that it is *"for archaeology, never a gate input"*. `git blame` has no specified output contract: its rename and copy detection are heuristics whose defaults and behaviour have changed across git releases, so the value is a function of the git binary rather than of the objects. Including it would make a routine `git` upgrade on the runner turn the next landing into `reconstruction-failed`, with a report naming the graph and no way to see that the cause was a package update. Pinning blame's flags was considered and refused: pinning flags does not pin a heuristic's implementation, and the value is by PB's own words not a gate input, so nothing is lost. The store may hold it; the dump does not carry it.

**2 · Shipped-floor `protects` edges.** PB §6.2 derives `protects` from *"the floor list inside the pinned release (`spine:<version>:floor`) + constitution `C-A2`"*. The `C-A2` half is a git object — the constitution blob — and stays. The shipped half is not in the repository at all; it is inside the binary, and its `src` production `spine:<version>:floor` says so. Including it would make the dump a function of the release, which §3.4 forbids for a reason: two releases at one `dump_version` must agree byte for byte. It would also require a node kind for the release, which PB §6.2 does not have — the shipped floor has no plausible `from_id` among the nine kinds, and attributing it to `constitution:<v>` would be false, since the shipped floor changes when the release changes and not when the constitution does. Excluding it removes the need to invent a node kind and removes a release dependency in one move. G10 loses nothing: both sides run the same binary, so comparing the shipped floor against itself proved nothing. §15 OPEN-1 asks whether the store should carry a `release` node at all.

**3 · `exercises`.** §8.3. Stated here because it is the same rule and an implementer adding coverage in v1.1 will look for the ruling.

Each of the three is a change to PB §6.3's G10 clause, not merely a reading of it. §14 D4 files that clause as a defect: it should cite this document rather than enumerate four adjectives.

### 8.6 † states

PB §6.3 says † states are *"dumped as `tests-approved`"*, and PB §6.2 says they *"collapse to `tests-approved` in any fresh clone"* because they exist only while a gate record is live and a gate record is not a git object.

Under §8.2 the clause is **vacuous**: `checked†` and `base-moved†` are in-flight states, in-flight intents are not dumped, and no landed intent has a † status. It is restated rather than dropped for two reasons. It is a conformance check with teeth — a dump containing a † status is non-conforming and the check is one string comparison — and it is the rule a later `dump_version` would need if it ever dumped in-flight elements, which is not impossible: the reason it cannot happen today is the clone asymmetry of §4.1, not a principle.

### 8.7 The worktree

> **A dump is a function of trees and refs. Running `--dump` in a bare repository, with a dirty working tree, with a stale index, or with untracked files present produces identical bytes.**

That is the whole content of PB §6.3's *"worktree-only files excluded"*, stated as a testable property instead of an adjective. Its immediate consequence is that PB §6.1's `<path>:<line>` provenance production — a line of a file in the working tree — is never emitted by a dump (§5.4); every file citation in a dump is the `git:<sha>:<path>:<line>` form, anchored to a commit.

---

## 9. The empty dump

A dump is **empty** when the derivation produces no node and no edge. It is one line — the header — terminated by LF, and nothing else:

```
{"dump_version":1,"object_format":"sha1","repo":"myrepo","schema_version":7,"t":"header","trunk":"main"}
```

105 bytes. `digest: sha256:2a3fcea5debff8e17af36eef1a4acfa1fea9f86a0d6c942cf2993cce3e80f290` (verified: §12.5).

There is exactly one **empty dump**, and there are three distinguishable states around it:

1. **No manifest is resolvable** (§4.2 step 4). Not an empty dump: `--dump` refuses with `not-installed`, exit 2, and writes nothing to stdout. A dump of nothing and a dump of a repository spine does not manage are different facts, and conflating them would let a mis-targeted G10 clone compare two "empty" dumps and pass.
2. **A manifest resolves but `refs/heads/<trunk>` does not** — a detached HEAD whose manifest names a branch this clone lacks, which is exactly what a mis-cloned G10 side looks like. `head` is absent, the walk has no tip, the derivation produces nothing, and the dump is the bytes above.
3. **Trunk resolves and is at or below the trust root**, with no sealed landing above it. `head` is present. Signer and constitution nodes are derived from the trust root commit itself (§8.2), so this case is empty only in a repository where `spine init` has not yet landed anything — which the first `init` normally fixes, since it lands.

An empty dump is legal, is not an error, and exits 0. G10 comparing two of them is a pass, and correctly so: two clones that both derive nothing from the same tip agree.

---

## 10. Determinism rules, collected

Normative, and repeated here so an implementer can check against one list.

1. **No wall clock.** No record holds a time, a duration, a date or anything derived from one. Committer dates may be read by a derivation; none is a value. `params.timeout` (PB §6.7, settled 2026-08-26) bounds a runner invocation and never appears — the runner's whole output is excluded (§8.4). PB §7.5: one clock, and it is the chain.
2. **No environment.** No hostname, runner id, user, locale, process id, temp path, or path outside the repository. One binary and one git produce both sides of a G10 comparison; §7.2.1 names the single value that is nonetheless environment-dependent and says why it stays.
3. **No state the design forbids.** No persisted, fetched or restored store; `--dump` implies `--fresh` (§4.3). No note read as a source, and no dump written anywhere a later run could find it.
4. **Key ordering** inside a record is JCS's: ascending by member-name bytes. Never insertion order, never a hand-written order.
5. **Record ordering** is §6's, over the keys of §6.2 and §6.3 — never over the serialized line (§6.5).
6. **Absent versus null.** `null` never appears. `attrs` is always present, `{}` when empty (§7.1). Absence means "does not apply".
7. **Numbers** are integers in `[0, 2^53 − 1]`, plain decimal, no leading zero.
8. **Paths, principals, ref names and trailer values** are `esc`-encoded (§2.4) and are never normalized, casefolded or separator-rewritten. Paths are the tree's bytes.
9. **Object ids** are lowercase hex at `object_format`'s full length — 40 or 64 digits. Never abbreviated, never uppercase, never prefixed.
10. **Non-git digests** are `sha256:` + 64 lowercase hex, except the `approval` local id, which is bare hex inside an id (§5.2.1) — PB §11's hash policy governs values, and an id is not a value it governs.
11. **No self-reference.** A dump never contains its own digest, its own length, its own record count, or the release that produced it (§3.4).
12. **Git plumbing is pinned by the release.** Every invocation the derivation makes — `diff`, `rev-list`, `merge-tree`, `patch-id`, `ls-tree` — runs with its diff algorithm, rename and copy detection, and every other output-affecting option fixed by the release and never read from repository, user or system config. `modifies` edges come from `git diff --name-only`, whose output depends on rename detection; `reverts` comes from `git patch-id --stable`, whose input depends on the diff algorithm. A repository that sets `diff.algorithm` must not thereby change its own dump. The exact option set is the release's and the indexer spec's (§16); the rule that it is pinned is this document's.
13. **One binary per comparison.** G10 indexes and dumps both sides with the same process's release. §3.3's `schema_version` and §3.1's header make a violation visible at line 1.

---

## 11. G10's comparison

PB §6.3 G10, made exact. On the candidate landing `L`, at PB §5.4 step 5, after G9 and before the CAS:

1. `L` is pushed into the scratch clone `S` as `refs/heads/<trunk>` with the intent ref deleted, so `S` holds the post-CAS ref set.
2. `S` is cloned with `--no-local --no-hardlinks file://S`, `GIT_CONFIG_GLOBAL=/dev/null`, no network, default refs only — no notes, no custom refs, no provider metadata.
3. The runner's pinned trust root is written to `spine.trustRoot` **in both repositories** (§3.1, and §14 D1: PB §6.3 names only the clone).
4. `spine index --fresh --dump` runs in each, producing byte streams `D_S` and `D_C`.
5. **The comparison is `D_S == D_C` as byte strings.** Equal digests (§2.5) are an equivalent implementation. Nothing parses either stream.
6. Unequal: the push is refused, `L` is discarded, the run ends `reconstruction-failed` without a retry and without consuming a `C-M3` re-verification, and the run's own report is the only record.

Two properties this format is responsible for, and an implementer should test both directly:

- **No false positive.** Two indexings of the same objects by the same release produce identical bytes. Everything in §10 exists to make that true.
- **A legible failure.** Because the streams are line-sorted JSONL, `diff` names the record. The header is line 1 in both, so an input-set disagreement — different tip, different trust root, different trunk name — surfaces before the body and can be read at a glance.

**G10's own result never enters `Spine-Gates`** (PB §5.4 step 5, PB §11): `L` exists by then and its seal covers its message. `gate-report.md` §5.6.2 records the same fact from the report's side — G10 never appears in a version-1 report.

---

## 12. Worked example and published vectors

Object ids are fabricated but well-formed; the vectors test a serializer, not a repository. All three digests below were computed over the exact bytes printed, by a serializer written from §2 alone, and are **verified**.

### 12.1 The repository

`myrepo`, `object_format: sha1`, `params.trunk: main`, `params.langs: [python]`, pinned release 1.4.0, team mode, `C-A3: hostile`, `C-M1: merge`.

**One repository, three documents.** This is the same `myrepo` that `manifest.md` §8 and `constitution.md` §12 describe, and its keyring is the one `manifest.md` §8.7 prints and `envelope-vectors.md` §8.1 publishes: **three principals, `alice@example.com`, `bob@example.com` and `ci@example.com`**, whose fingerprints are what `ssh-keygen -lf` produces from those published keys. The three `signer` nodes below carry exactly those fingerprints, and the seal is `ci@example.com`'s. Until 2026-08-27 this section printed a fourth key set of its own — three invented fingerprints under a principal `pipeline@ci.example.com` that no other document names — inside a digest it had verified; the digest was right about the bytes and the bytes were about a repository that did not exist. `manifest.md` §11 C12 filed it and it is closed here.

Trunk's first-parent history is two commits: the trust root `T0` = `0a1b…4567`, the bootstrap `init` commit, which carries the keyring, `CONSTITUTION.md` at v3 and `adr/ADR-007-tax-rounding.md`; and the landing `L` = `1b2c…6789` for `INT-042: Invoice totals include tax`, whose base is `T0`.

`M(L)` is five member commits: `M1` `2c3d…8901` (sign-off, empty), `M2` `3d4e…012a` (tests written), `M3` `4e5f…2ab3` (approval, empty), `M4` `5f60…b3c4` (implementation), `M5` `60718…c4d5` (review, empty). `Hc` is `M5`.

Three signed lines are copied into the envelope. Their exact bytes, which §5.2.1 hashes for the `approval` local ids:

```
Spine-Signoff: INT-042 blob=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 template=intent@2 constitution=v3 reopens=0 signer=alice@example.com
Spine-Approve: INT-042 intent=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 base=0a1b2c3d4e5f60718293a4b5c6d7e8f901234567 rounds=1 total_rounds=1 reopens=0 red=5/5 freeze=sha256:5c9e2a71b0463df8951ce2a4708b3d61f0492c8ad735be106f4a2c9d80e37b45 signer=alice@example.com
Spine-Review: INT-042 class=tripwire head=60718293a4b5c6d7e8f90123456789012ab3c4d5 tree=7b0dc1f4a2e58d3906bb4c7e21f5a8d90c3e64b7 base=0a1b2c3d4e5f60718293a4b5c6d7e8f901234567 intent=9f2c8a1d6b4e30f5c27a91ded4b0836f5e1c74a2 report=sha256:3c6f1a09b8d24e57af0132c9de6b48570e29a1cf83b6d045e71a29c4b0d83e16 wires=G11 reason="auto-merge unavailable: C-A3 hostile" reviewer=bob@example.com
```

giving, as SHA-256 over each line's bytes with no LF:

| line | `approval` local id |
|---|---|
| `Spine-Signoff` | `2f5e600237ec3d9a1f35fbc7ee6cf0dfd4335433def9937b5eeb8202bd3d66f6` |
| `Spine-Approve` | `b6352921ea42d618930f32f04ad773c20157810d418d20d06758149f366b85a8` |
| `Spine-Review` | `ae8a406391f7130ce62d0e86fa4dca92195812aae2743e15e60434af56244021` |

The review is the universal rule-5 advisory: `C-A3: hostile` fails precondition 0, so `C-M4` evaluates off and G11 raises its `class=tripwire` wire on every landing that tests anything (PB §11).

The landing's diff against `T0` touches three paths, one of which is **not valid UTF-8**: `src/billing/tax.py`, `tests/billing/test_invoice.py`, and `src/billing/caf` + `0xE9` + `.py` — a Latin-1 `é`, which git stores as one byte and which no amount of normalization can make into text. The approval froze `tests/billing/test_invoice.py` and `pytest.ini` and two function ids. `C-A2` extends the floor with `infra/`.

The example exercises: every node kind; **eleven** of the fifteen edge kinds (`reverts`, `supersedes` and `superseded_by` have no occasion in a two-commit history; `exercises` is excluded from every dump by §8; `protects` **is** present, but only its `C-A2` limb, shipped-floor `protects` records being excluded by §8.3 — so eleven kinds appear and four do not); a non-UTF-8 path through `esc`; the `msg:L<n>`, `trailer:<Name>`, `git:<sha>` and `git:<sha>:<path>:<line>` provenance productions; an absent optional attr (`signer.valid_to`); an array attr; and the `code_unit` minimum-`src` rule of §5.5.

### 12.2 The dump

62 lines, 14054 bytes.

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

### 12.3 The digest

```
lines:  62
bytes:  14054
digest: sha256:3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da
```

**Verified**, over exactly the 62 lines above, each terminated by one LF including the last.

Six things in it are worth checking against, because they are where two readings would first part company:

1. **Node order is `kind` then `id`**, so `myrepo/INT-042` (kind `intent`) is the twenty-third node, far below `myrepo/INT-042/AC-1` (kind `ac`), which is the first. PB §6.3's order, not an id order.
2. **`myrepo/code:src/billing/caf\xe9.py` precedes `myrepo/code:src/billing/tax.py`**, which raw-byte order would reverse (§6.4).
3. **The `code_unit` src is the minimum over citing edges** (§5.5): `tests/billing/test_invoice.py` is cited by `modifies` from `L` and from `M2` and by `freezes` from `M3`, and takes `git:1b2c…6789`, the least of the three.
4. **Member changesets carry `{"landing":false}` and nothing else.** They have no seal.
5. **The two `declares` edges to `code:src/billing/` and `code:api/invoices.ts` share a `src`** — one line of the touchpoints block names both paths — and are ordered by `to`, PB's second key.
6. **`signer.valid_to` is absent, not null**, on all three signers.
7. **The three fingerprints are real** and reproduce from `envelope-vectors.md` §8.1's published keys with `ssh-keygen -lf`; the seal principal is `ci@example.com`, matching the `spine-seal@v1` entry in `manifest.md` §8.7's 411-byte keyring. **The line count did not change and the byte count did**: 62 lines either way, 14081 bytes before the substitution and 14054 after, because `ci@example.com` is nine bytes shorter than `pipeline@ci.example.com` in three places and the three fingerprints differ in length from the invented ones.

### 12.4 The ordering vector

Debug your comparator against this before attempting §12.2. It exercises every tie-break level and the `esc`-versus-raw ordering trap, and nothing else. Records are shown first in an arbitrary authored order, then in canonical order.

Authored:

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

Canonical:

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

**Verified**, over the canonical block only (11 lines, each LF-terminated, no header — this is a fragment, not a dump). What it pins:

- `AC-10` before `AC-2` — byte order, not numeric (§6.4);
- `ac` < `changeset` < `code_unit` — `kind` is the first node key;
- `\xe9.py` before `z.py` — `esc` order, the reverse of raw-byte order;
- the two `declares` edges tie on `from`, `to` and `kind` and break on `attrs`: `expected` before `forbidden`;
- the two `modifies` edges tie on `from`, `to`, `kind` **and** `attrs` and break on `src`: `git:aa` before `git:bb`;
- edges under one `from` are ordered by `to` before `kind`, so both `has_ac` edges precede both `declares` edges — PB §6.3's `from,to,kind` exactly.

### 12.5 The empty-dump vector

```
{"dump_version":1,"object_format":"sha1","repo":"myrepo","schema_version":7,"t":"header","trunk":"main"}
```

```
bytes:  105
digest: sha256:2a3fcea5debff8e17af36eef1a4acfa1fea9f86a0d6c942cf2993cce3e80f290
```

**Verified.** One line, one LF, no `head`, no `trust_root` (§9 case 2).

---

## 13. Resolved ambiguities

Each entry states what the playbook says, what this document chose, and why.

### 13.1 The serialization was never named

**Playbook:** PB §6.3 G10 says "canonical `--dump` on both sides" and fixes the sort order in one clause. It names no encoding, no framing, no number format, no string escaping, and no rule for a path that is not UTF-8.
**Chosen:** JSON Lines, each line RFC 8785 JCS under the profile of §2.3, byte-valued data through `gate-report.md`'s `esc`, LF framing with a terminating LF.
**Why:** without a named scheme, two implementations produce different bytes over identical objects and G10 refuses every landing terminally. JCS is reused rather than re-chosen because `gate-report.md` §2.1 already argued it and because an implementer who has one JCS serializer should need only one. JSONL rather than one document because the artifact is unbounded, must be sortable externally, and must diff legibly — a terminal failure whose only record is the run's own report deserves a readable diff.

### 13.2 Which side of the exclusion each kind falls on

**Playbook:** four adjectives — provisional, † states, volatile test results, worktree-only files.
**Chosen:** the generating rule of §8.1 — derived from git objects reachable from the trunk tip, or excluded — enumerated by node kind in §8.2 and by edge kind in §8.3.
**Why:** an adjective is not a specification. "Provisional" does not say whether an in-flight intent's `ac` nodes go, whether its `approval` nodes go, or what happens to a `code_unit` that only an in-flight `declares` names. The generating rule answers all three at once and answers the next question too. It is also the only reading under which G10 can pass at all: the scratch clone holds `refs/remotes/origin/intent/*` and the clean clone holds nothing, so any element derived from an intent branch differs on every landing made while another intent is open.

### 13.3 Node ids are repo-scoped for every kind

**Playbook:** PB §6.2 states the rule — *"IDs are repo-scoped from day one (`myrepo/INT-042`, not bare `INT-042`)"* — and then illustrates it with `test:vitest:…`, `code:src/billing/`, `cs:abc123f`, `approval:5c9e…`, `signer:alice@example.com`, `ADR-007` and `constitution:v3`, none of which carries the prefix.
**Chosen:** every node id is `<repo>` + `/` + a per-kind local id (§5.2). The examples are read as abbreviations.
**Why:** the rule's stated purpose is that *"multi-repo federation [is] a namespace merge later instead of a rewrite"*, and a federation that merges two repositories' `code:src/index.ts` has merged two different files. The prefix goes first, in the position `myrepo/INT-042` already puts it, so the intent form is unchanged and every other kind gains the same prefix in the same place. PB §6.2's example list should be updated to match (§14 D5).

### 13.4 `verified_by`'s direction, and other implicit directions

**Playbook:** PB §6.2 lists edge kinds without stating `from` and `to` for any of them.
**Chosen:** §5.3's paragraph fixes all fifteen.
**Why:** `verified_by` is decided by PB §6.3 G5, which fails on *"a `verified_by` edge to a nonexistent AC"* — so the AC is `to_id` and the test is `from_id`, however oddly the name then reads. The rest follow from the derivation table: `approves` and `signed_by` are properties of an approval, `attested_by` and `implements` of a changeset, `freezes` of an approval, `modifies` of a changeset. Getting one backwards is a whole-dump diff, so all fifteen are written down.

### 13.5 One `src` for an element several derivations cite

**Playbook:** PB §6.2 makes `src` a single `TEXT NOT NULL` and PB §6.1 requires every element to cite a source. Neither says which source, when there are several.
**Chosen:** the minimum under §6.4's ordering (§5.5).
**Why:** a `code_unit` is named by every edge that touches it, and "the first one the walk happened to reach" is walk order, which is not a specification. The minimum is total, cheap and independent of traversal. The residual is named rather than hidden: two implementations that produce different *citation sets* still differ, and the citation set is PB §6.2's derivation table to fix, not this document's.

### 13.6 The `approval` node id

**Playbook:** PB §6.2's example is `approval:5c9e…`; PB §4.3 calls the approve line's `freeze=` *"a non-git digest, used to name the approval elsewhere"*.
**Chosen:** SHA-256 over the signed trailer line's exact bytes (§5.2.1); `freeze=` is carried as the `freeze` attr.
**Why:** the freeze-digest reading is total over one of the six `event` values. A sign-off, a review, a reopen, a withdrawal and an upgrade have no freeze digest and would need a second id scheme inside one kind. Line-hash is uniform and its uniqueness is already a gate's: G13 refuses an event line byte-identical to an earlier one, outright (`manifest.md` §4.8.4 check 3).

### 13.7 `changeset.tree` may be a sentinel, and one sentinel reads the environment

**Playbook:** PB §6.3 G9 defines `tree: unverifiable(squash)` and `tree: unverifiable(git-version)`.
**Chosen:** both are legal values of the attr; the second is the one place a dump is not a pure function of §4.1's inputs (§7.2.1).
**Why:** suppressing it would not remove the divergence, only its explanation — the two sides would differ in a boolean somewhere with no clue why. Recording the local git version in the header was refused because it would make every cross-machine dump of one repository differ, which is worse. G10 is unaffected: one binary, one git, one host. §15 OPEN-2 asks whether the sentinel should exist at all.

### 13.8 `intent.status` in a dump, and where `unattested` lives

**Playbook:** PB §11's States paragraph lists `reverted`, `superseded`, `orphan`, `unattested` and `resealed` together as post-landing states. PB §6.2's schema puts `unattested` and `resealed` in the **changeset**'s attrs and neither in the intent's.
**Chosen:** `intent.status` ∈ {`merged`, `withdrawn`, `reverted`, `superseded`}; `unattested`, `resealed` and orphanhood are changeset facts (§7.3).
**Why:** the schema is decisive where the prose is loose, and the two are not the same fact: a landing can be `unattested` while its intent is plainly merged, and a resealed range's members stay `unattested` (PB §6.3 G9) while their intents do not change status. Putting the flag on the changeset keeps one fact in one place.

### 13.9 Kinds PB §6.2 gives no attrs

**Playbook:** the schema comments give attrs for `intent`, `changeset`, `approval`, `signer` and `test` nodes and for `declares`, `implements`, `verified_by`, `freezes`, `reverts` and `protects` edges. The rest are silent.
**Chosen:** silence means none. `ac`, `adr`, `code_unit` and `constitution` nodes carry `{}` (§7.2).
**Why:** the alternative is every implementation inventing an attr set — AC text, ADR title, constitution owner — and no two agreeing. A dump that carries no AC text is a dump G10 can compare; the store may hold whatever a query needs.

### 13.10 `attrs` is emitted when empty

**Playbook:** silent.
**Chosen:** always present; `{}` when empty (§7.1).
**Why:** the alternative is a divergence class with no diagnostic — one implementation omits, one emits, every landing fails — for a saving of nine bytes per record. `gate-report.md` §7 rule 5 makes the same call for empty arrays.

### 13.11 `object_format` comes from the repository, not the manifest

**Playbook:** `gate-report.md` §5 takes it from the manifest at `base`; PB §6.7 makes it a manifest field.
**Chosen:** the indexed repository's `extensions.objectFormat`, defaulting to `sha1` (§3.1).
**Why:** every oid in a dump is an object *in that repository*, and the dump should describe what it read. The two agree in every conforming repository; where they disagree, the repository is broken and the disagreement is G15's or G16's finding, not something the dump should paper over by relabelling its own oids.

---

## 14. Defects found in PLAYBOOK.md v0.19

Reported here rather than repaired, per `docs/spec/README.md`: where this document and PB §11 disagree, §11 wins and the disagreement is a defect in one of them. None of these is in §11. **Citations are section anchors plus a verbatim quote, never line numbers** — a line number rots as the playbook grows, a section and a quote do not. Every entry is marked **OPEN** or **CLOSED** against `PLAYBOOK.md` as it now stands.

**D1 · CLOSED · G10 set the trust root on one side of the comparison** (PB §6.3's G10 — Reconstruction row). **As filed**, that row said *"the runner's pinned trust root is copied into the clone's `spine.trustRoot`"*. `git clone` does not copy git config, and the scratch clone `S` is itself a clone, so `S` had no trust root unless one was written into it. The chain walk of PB §7.5 then had no root on the `S` side: signer nodes, `verified`, `seal_verified` and `unattested` differed, and every landing failed G10 for a reason that has nothing to do with the ledger. The fix was one clause — the trust root written into **both** repositories. **Taken:** the row now reads *"the runner's pinned trust root is written into **both** sides' `spine.trustRoot` — `S`'s as well as the clone's, since `S` is itself a fresh clone and carries no local config, and a side without a pin would trust on first use or refuse, either way diverging from the other on every landing (TOFU is for humans, never for G10)"*. §3.1 still makes the symptom legible (a line-1 diff) if it ever recurs.

**D2 · OPEN · A trailer citation cannot name the second of two identical trailers** (PB §6.1's provenance grammar, *"`git:<sha>:trailer:<Name>`"*). PB §6.1's grammar has `git:<sha>:trailer:<Name>`. A signerless landing carries two `Spine-Review` lines (PB §11's signerless overlay); a squash landing copies many `Spine-Frozen` and `Spine-Test` lines. Every element derived from any of them cites the same string, so the citation does not identify the line it came from — which is exactly what PB §6.1 says a citation is for. Recommended: `git:<sha>:trailer:<Name>#<n>`, `n` the 1-based occurrence. Latent for dumps, because two records that differ only in which line they came from also differ in `to` (§6.3); active for any tool that reads a rendering, which PB §6.1 explicitly contemplates.

**D3 · OPEN · The provenance grammar is not unambiguously parseable** (PB §6.1's provenance grammar). `git:<sha>:<path>:<line>` collides with `git:<sha>:msg:L<n>`, `git:<sha>:trailer:<Name>` and `git:<sha>:patch-id` when a path is called `msg`, `trailer` or `patch-id`; and `<path>:<line>` collides with the whole `git:` family when a path begins `git:`. A last-colon rule plus an oid-length test disambiguates in practice, but it is a heuristic and PB §6.1 offers none. Nothing in spine parses a `src` (§1), so this is latent — but PB §6.1 blesses renderings that other tools read, and those tools will parse it.

**D4 · OPEN · G10's exclusion clause is four adjectives** (PB §6.3's G10 — Reconstruction row). *"provisional (in-flight) elements, † states (dumped as `tests-approved`), volatile test results and worktree-only files excluded"* does not reach `verified_by.introduced_by` (a `git blame` heuristic), shipped-floor `protects` edges (derived from the binary, not the repository), or `exercises` (derived from a coverage report). Each is a nondeterminism or a release dependency in an artifact whose whole purpose is byte equality (§8.5). Recommended: the clause cites this document instead of enumerating.

**D5 · OPEN · PB §6.2's node-id examples contradict its own repo-scoping rule** (PB §6.2's schema comment, whose `id` row lists `"code:src/billing/" | "cs:abc123f" | "approval:5c9e…"`). §13.3. Recommended: prefix the examples, or say in one clause that only `intent` and `ac` ids are scoped — but the federation argument in the same paragraph forecloses the second.

**D6 · OPEN · The schema has no edge kind that answers PB §6.4's ADR query** (PB §6.4, *"any ADRs touching the same code units"*). PB §6.4 promises a resuming agent *"any ADRs touching the same code units"*. PB §6.2 gives `adr` nodes and connects them only through `supersedes` / `superseded_by`; there is no edge from an ADR to a `code_unit`, and none is derivable — an ADR is prose. As it stands an `adr` node is isolated (as in §12.2), the query cannot be answered, and the promise is unbacked. This costs the dump nothing; it costs `spine context` a feature.

---

## 15. OPEN — the owner's calls

**OPEN-1 · Whether the shipped floor belongs in the graph, and under which node.** §8.5 excludes shipped-floor `protects` edges from the *dump*, which resolves the format's problem and leaves the store's open: PB §6.2 derives `protects` partly from *"the floor list inside the pinned release"* and gives no node kind that could be its `from_id`. The nine kinds are all repository facts; a release is not one. Three ways out. (a) Leave it as §8.5 does: the store may hold the edges under whatever `from_id` an implementation likes, since nothing compares them — cheap, and it means two implementations' stores differ in a way no gate notices, which is the kind of thing this directory exists to stop. (b) Add a `release` node kind, id `<repo>/release:<version>`, `PRAGMA user_version` 8 — honest, and it makes G14's floor auditable as a graph query, at the cost of a schema bump and a node whose content is not in the repository. (c) Drop the edges entirely and let G14 read the floor list directly, which it already does. Recommendation: (c), then (b) if a query ever needs it. This is owner-level because (b) changes PB §6.2.

**OPEN-2 · Whether `tree: unverifiable(git-version)` should exist.** It is the only value in the dump that reads the local environment (§7.2.1), and it exists to record that the indexer could not reproduce a merge tree because its git differs from the seal's. The alternatives are to make the version mismatch a hard finding (G9 fails and the landing is `unattested`, which is loud but punishes an upgrade), or to make the tree check conditional on the git version matching and record nothing when it does not (silent, and the audit quietly weakens). Recommendation: keep it, and add the git version to `spine stats` so a fleet-wide skew is visible before it is confusing. Owner-level because it is a G9 semantics question, not a serialization one.

**OPEN-3 · Whether `--dump` should have a mode that includes in-flight elements.** §8.2 excludes them because G10 cannot compare them (§4.1). But `spine context` and `spine review` will want a rendering of exactly that, and PB §6.1 permits a rendering other tools read provided it is datable and counted in PB §10's budget. If such a mode ships it is a **second artifact**, not a flag on this one: it needs its own version, its own exclusion set, and — critically — it must not be what G10 diffs. Recommendation: not in v1; if it comes, name it something other than `--dump` so no one can wire the wrong one into G10. Owner-level because it is a PB §10 budget line.

---

## 16. Out of scope

Deliberately not specified here, and where it belongs instead:

- **The derivation itself.** Which commits are walked, how the envelope is parsed, how touchpoint strings become `code_unit` ids, how the constitution's version is read. PB §6.2's derivation table is the indexer's spec; this document serializes its output and fixes only the determinism rules that bear on the bytes (§10).
- **The pragma parse and the naming sugar** — and this is the one out-of-scope pointer that can invalidate this document without touching it. PB §6.2 derives `test` nodes and `verified_by` edges from *"pragmas `@verifies INT-042/AC-1` in a comment … or a test name carrying `AC<n>` in its runner's conventional position (sugar, per-runner pattern in `docs/spec/`)"*. A comment is language-specific and the sugar is runner-specific, and **`import-resolver.md` §12 now fixes both** for all four v1 languages — Python, TypeScript/JavaScript, Dart, Swift (settled 2026-08-26; **Kotlin was dropped**, `.kt`/`.kts` are `lang: none`): §12.1 the pragma grammar, §12.2 the file-granular join, §12.3 the per-runner naming sugar. The dependency is unchanged in kind — two implementations reading §12 differently still produce different `test` nodes and different `verified_by` edges over the same objects, and every vector in §12 here is only as reproducible as that section is — but it is a dependency on written text now, not on an unwritten one. This is the same relationship `gate-report.md` §11 declares with `constitution.md`, and it is normative, not decorative.
- **The freeze closure.** G8 recomputes it in `--ci` with the pinned release's resolver for each of `params.langs`, and a resolver differing on one edge case rejects another's approvals (PB §4.3). It does **not** affect a dump: `freezes` edges are derived from the `Spine-Frozen` and `Spine-Test` lines the approval carries, not recomputed by the indexer. A resolver disagreement is a G8 failure at approval time, not a G10 failure at landing time.
- **The manifest's grammar**, including the `repo` key this document constrains to `^[A-Za-z0-9._-]+$` (§5.2) and the `params.langs` array: **`manifest.md`**, written after this document and now owning both — §3.1 for `repo`, §3.3 for `params.langs`' domain and its monotonicity. §5.2's interim constraint was adopted there unchanged.
- **The envelope's grammar** — trailer syntax, line folding, the fenced block's byte range, and what a `-Sig` covers: `envelope-vectors.md`. §5.2.1 hashes a trailer line and defines the byte range it hashes so that it does not move when that document lands.
- **`esc`** — `gate-report.md` §2.3 owns it (§2.4). **The wire token `tok` and the `wires` array order** — `gate-report.md` §6.1–§6.2 own them; §7.2's `approval.wires` records what the signed line carries and does not re-derive it.
- **The result file** — `result-file.md`. The dump excludes everything derived from it (§8.4).
- **The gate report** — `gate-report.md`. `changeset.report_sha256` is derived from the seal, not from a report; the two artifacts never read each other.
- **The floor list's contents** and the three CI definitions: the release and `ci.md`.
- **Gate semantics.** What G2 containment means, how G14 casefolds, when a landing is `unattested`. This document fixes how a derived fact is *recorded*, never what a gate decides.
- **Diagnostics.** Which record differs and why, how a G10 failure is presented: the CLI's output and PB §6.5's review packet. Stdout carries the artifact and nothing else (§2.2).
- **`spine stats`, `spine context`, `spine review`** and every other reader of the graph. They read the store, which holds more than the dump does, and none of them reads a dump.
- **Storage, retention and transport.** A dump is written to stdout and consumed in the process tree that produced it. It is never a git object, never a note, never fetched, never cached (§1).
- **Any second dump format, second digest, or exported rendering.** A rendering that ships is counted by PB §10's graph budget; this artifact ships none, and OPEN-3 is what asks before it does.

---

## 17. Conformance checklist

A serializer conforms iff all of the following hold. Every item is mechanically checkable against a produced dump.

**Framing and encoding**

1. Line 1 is a `"header"` record; there is exactly one.
2. Every line is LF-terminated, including the last; no CR, no BOM, no blank line.
3. Every line is valid JCS over §2.3's profile: members sorted by name bytes, no whitespace, integers plain and in `[0, 2^53 − 1]`, no float, no `null`, no duplicate name, depth exactly two.
4. Every byte of every string is in `U+0020 … U+007E` after `esc`; no path, principal or ref name is normalized or casefolded.
5. Every oid is lowercase hex at `object_format`'s full length.
6. `sha256sum` over the stream equals the digest the run reports.

**Records**

7. Every record's `t` is `header`, `node` or `edge`; no other member name appears anywhere.
8. Every node has `attrs`, `id`, `kind`, `src`, `t`; every edge has `attrs`, `from`, `kind`, `src`, `t`, `to`. `attrs` is present even when `{}`.
9. Every `kind` is in §5.1's or §5.3's closed set. No `exercises` edge appears.
10. Every node `id` matches its kind's row in §5.2 and begins `<repo>/`.
11. Every `from` and every `to` names a node record in the same dump.
12. Every `src` is in one of PB §6.1's productions (§5.4). No `src` uses the bare `<path>:<line>` form, and none uses `spine:<version>:floor`.

**Order**

13. The node section precedes the edge section, and each is sorted ascending by §6.2's and §6.3's key. Verified by re-sorting the records by key and comparing, **not** by sorting the lines.
14. `id` is unique across the node section; `(from, to, kind, attrs, src)` is unique across the edge section.

**Exclusions**

15. No `intent` node carries a † status, and every `intent.status` is one of the four in §7.3.
16. Every `implements` edge has `provisional: false`.
17. Every `protects` edge has `floor: false`.
18. No `test` node carries `result_at`; every `test` node's `attrs` is `{}`.
19. No `verified_by` edge carries `introduced_by`.
20. No record's `src` names a commit not reachable from `head`, and no `changeset` node names a commit below the trust root.

**Determinism**

21. Two runs over the same repository, same release and same trust root produce identical bytes.
22. A run in a bare clone of the repository produces bytes identical to a run in a checked-out one, and a dirty working tree changes nothing.
23. No member holds a time, a duration or a date.
24. The dump changes when and only when the trunk tip, the objects it reaches, or the trust root changes.
