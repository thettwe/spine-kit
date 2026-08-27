# The gate report

**Artifact:** the canonical-JSON record `spine check --land` produces on every evaluation, whose SHA-256 is sealed into every envelope as `report=` and named by every `Spine-Review`.
**Home in the playbook:** PB §7.4 rule 4. Vocabulary from PB §11, which wins over prose here as it wins there.
**References:** `PB §n` cites `PLAYBOOK.md`; a bare `§n` cites this document. The two numbering schemes collide — PB §5.2 is the tripwire list, §5.2 is `objects` — so every citation says which.
**Spec version:** 1 · **Report schema version:** 1 · **Covers:** PLAYBOOK.md v0.19 · **Status:** normative for v1, alongside `docs/spec/constitution.md` (§5.4.1, §11) and `docs/spec/import-resolver.md` (§5.4.2, §11) — the two external documents that can move a member's value without a line of this one changing.

**Amended 2026-08-26**, to the owner's six settled decisions of that date. Four touch this document. **(a) Provider evidence is narrowed** (§4, §5.8, §5.9, §9.22, §9.25): auto-merge precondition 2 gains a third conjunct — the run must have established that the ingested result file came from a job whose definition was taken from trunk — and a file that cannot demonstrate that origin is **ingested** rather than refused. **(b) Kotlin is dropped**: v1 ships **four** languages (§4, §5.4, §5.4.2, §11). **(c) `Template:` names the variant and the version** — `intent@2`, `intent-change@2`, `intent-bug@2` — so the `template=` field inside `authority.signoff.line` changes bytes (§5.3, §8.2, §8.2.1). **(d) The landing subject is derived and G9 checks it**, which is a different object from this report's `subject` member and moves no digest (§5.1, §9.14). The remaining two — an unbounded `forbidden` set stays legal with a new `spine stats` counter (§6.1), and G14/G16 getting their own document (`manifest.md`) — leave no member's value different. **A per-gate wire table has since been added at §6.3**, which adds no member and moves no digest — `class` was already required and two-valued; §6.3 fixes the value each gate writes, where PB §6.3 left seven of them unassigned. **The report schema does not move.** No member is added, removed or retyped; `automerge.preconditions[2].status` keeps its three-value domain and takes `"unmet"` for the new failure; `report_version` stays 1. Every digest in §8 stands as printed: §8.3 is untouched by all six, and **§8.1 and §8.2 are now published** — recomputed over the value as amended, with (c)'s `template=intent@2` inside `authority.signoff.line` already in the bytes they cover (§8.2.1).

---

## 1. What this artifact is, and what rests on it

`report=` appears in two signed places and never leaves either:

- on `Spine-Review`, where a human's signature says *I read this evaluation of this tree and accepted these wires* (PB §5.4, PB §11);
- on `Spine-Seal`, where the pipeline's signature says *these gates, over these objects, under this policy, produced this verdict* (PB §11).

It is the only artifact that records **why a landing was permitted**. The envelope records the inputs; the report records the judgement over them. `spine check --verify <landing-sha>` re-runs the pinned release over the same objects and compares digests (PB §7.4 rule 4) — so two implementations that canonicalize differently produce different digests over identical facts, and neither can verify the other's landings. That is the property PB §1.1 sells, and this document is what makes it hold.

The report is **not** a git object. It is a non-git artifact, so its digest is `sha256:<hex>` per PB §11's hash policy. Everything it *names* that is a git object is named by object id.

**It is, however, published as one.** As of PB v0.19 the trusted stage writes the report's canonical bytes to `refs/notes/spine` on every landing, and that is not optional (PB §7.4 rule 4). That publication is what makes `--verify` available to anyone holding a clone rather than to whoever still holds a CI run's artifacts; §4.4 fixes the ref, the annotated object, the exact bytes, and what a reader does when the note is missing or does not hash to the seal. **It changes nothing about authority.** A note is never a source, no gate reads one, and the ledger derives from commits alone — a missing or edited note is a lost audit, never an invalid landing (§4.4, §7 rule 3).

---

## 2. Canonicalization

### 2.1 The scheme, by name

The canonical form of a gate report is its **RFC 8785 JSON Canonicalization Scheme (JCS)** serialization, restricted by the value profile of §2.2.

**Why RFC 8785.** It is the only JSON canonicalization that is (a) an IETF-published specification rather than a convention, (b) implemented and cross-tested in every language spine-kit will ship a verifier in, and (c) defined as a *serialization* of a parsed value rather than a transformation of source text — so a verifier that rebuilds the report from git objects and a verifier that parses a stored copy produce the same bytes without agreeing on how the copy was pretty-printed. Alternatives were considered and refused: canonical CBOR (a second encoding to specify for an artifact humans must read during an incident), OLPC canonical JSON (unversioned, no number rules, no test vectors), and "sorted keys, two-space indent" (not a specification, and silently divergent on escaping and numbers).

**Digest.** `report=sha256:<hex>`, lowercase, 64 hex digits, over exactly the canonical bytes. No trailing newline, no BOM, no framing. A file holding a report contains exactly the canonical bytes and nothing else, so `sha256sum` over the file reproduces `report=`.

### 2.2 The value profile

JCS's hard corners are floating-point serialization and UTF-16 ordering of non-ASCII keys. A gate report never reaches them, because:

| Restriction | Rule |
|---|---|
| Member names | Match `^[a-z][a-z0-9_]*$`. ASCII only, so JCS's UTF-16 code-unit ordering reduces to byte ordering. |
| Numbers | Integers only, `0 ≤ n ≤ 2^53 − 1`. No sign, no leading zero, no fraction, no exponent, no `-0`. There is no floating-point value anywhere in a gate report. |
| Strings | ASCII only after the escape of §2.3: every character is in `U+0020…U+007E`. |
| Null | Never emitted. An absent value is an absent member (§7 rule 6). |
| Duplicate names | Invalid. A parser that meets one refuses the document. |
| Arrays | Order is fixed by this document per field; JCS preserves it. |
| Depth | Bounded by this document's schema; no recursion. |

Under this profile, JCS reduces to: sort each object's members by member-name bytes, ascending; emit with no whitespace; emit integers in plain decimal; emit strings with JSON's minimal escaping (`"` → `\"`, `\` → `\\`, nothing else can occur); output UTF-8.

**Implementation note, not normative:** for this profile, Python's `json.dumps(obj, sort_keys=True, separators=(',',':'), ensure_ascii=False).encode('utf-8')` is byte-identical to JCS. It is *not* JCS in general — floats and non-BMP member names diverge — which is exactly why the profile exists.

### 2.3 Byte-valued data: the `esc` encoding

Repository paths are byte strings. Git does not require them to be UTF-8, and macOS filesystems disagree with Linux ones about normalization. Trailer lines carry `reason=` values that may be any UTF-8. JSON has no byte-string type. So every value in a gate report that carries repository bytes or human bytes — paths, trailer lines, patterns, principals — is encoded with `esc` and is thereafter pure ASCII.

`esc(s)`, for a byte string `s`, emits for each byte `b`:

| `b` | emits |
|---|---|
| `0x5C` (`\`) | the two characters `\` `\` |
| `0x20 … 0x7E`, other than `0x5C` | the character with that code point |
| anything else (`0x00–0x1F`, `0x7F–0xFF`) | the four characters `\` `x` and two **lowercase** hex digits of `b` |

The result is a character string over `U+0020…U+007E`, which the JSON layer then escapes normally (`"` → `\"`, `\` → `\\`).

Decoding is total and unambiguous: `\` introduces either `\` (one literal backslash) or `x` plus exactly two lowercase hex digits (one byte). Any other sequence after `\` is an invalid report.

Worked cases, showing both layers:

| Path bytes | `esc` | bytes in the canonical JSON |
|---|---|---|
| `src/shared/util.ts` | `src/shared/util.ts` | `"src/shared/util.ts"` |
| `a\b` | `a\\b` | `"a\\\\b"` |
| `caf` + `0xC3 0xA9` | `caf\xc3\xa9` | `"caf\\xc3\\xa9"` |
| `a"b` | `a"b` | `"a\"b"` |
| `a,b` | `a,b` | `"a,b"` (the comma is only escaped inside a *wire token*, §6.2) |

**Nothing is ever normalized.** No NFC, no NFD, no case folding, no separator rewriting. This matches PB §3.3's canonical-form rule for the intent doc ("no Unicode normalisation") and is the reason a report computed on macOS and one computed in a Linux container agree. Where a gate itself casefolds — G14 casefolds paths before floor comparison (PB §7.3) — the report records the path **as the diff produced it**, not the casefolded form.

---

## 3. Identity, versioning, and who decides

### 3.1 `report_version`

Every report carries `report_version`, an integer. This document defines version `1`.

The authority for which version a run emits is **the pinned release** — `cli.version` + `cli.dist_hash` in the manifest at `base` (PB §6.7, PB §7.4 rule 2). There is no `report` pin in `.spine/manifest.json` and this spec does not add one: G15 binds the running binary to the base's pin, so for every landing the trusted stage seals, the manifest at `base` already names the release that produced the report, and a `report` key would be a second spelling of a fact `cli` fixes.

**One landing form escapes that binding, and it is not an oversight.** On a rollback, uninstall or re-init whose `to=` release the base does not pin, `init` — not `check` — writes the envelope, and G15 accepts a seal whose `tool=` is that release while the base pins another (PB §6.7, PB §7.5). That landing's report is produced by the version its own `tool.version` names, and it is precisely the case — backing out a yanked release — where version skew is most likely. So the invariant is not "one version per repository at a time" but **one version per landing, named in the landing** — which is what `tool` records and what §3.2's keep-every-parser promise is for.

### 3.2 Reading an unknown version

A reader that does not know a report's `report_version` **refuses**: status `report-version-unknown`, exit 3. It never partially parses, never ignores unknown members, and never guesses. A binary keeps a parser *and a serializer* for every report version it has ever shipped, the same promise PB §6.7 makes for template and envelope versions.

A reader that meets an unknown **member name** inside a version it does know refuses the same way. The schema is closed: forward compatibility is bought with a version bump, not with tolerance, because a tolerant reader and a strict one compute different digests over the same document and the whole artifact is a digest.

### 3.3 Version skew and `--verify`

`spine check --verify <landing-sha>` requires the release the seal names. It refuses unless:

- the running binary's platform artifact hash equals the `dist_hash` in the seal's `tool=` — status `wrong-release`, exit 3, printing the release to install;
- `git --version`, parsed by the rule of §5.3, equals the seal's `git=` — status `wrong-git`, exit 3 (PB §7.4 rule 4 makes this a requirement, not a warning, because `merge-tree` output is a git version's contract).

This is what makes "recompute an old landing's report" tractable: you install the release that produced it. No binary is asked to reproduce another version's gate semantics.

---

## 4. Recomputation: what `--verify` can and cannot rebuild

A gate report is a function of three things: git objects reachable from the landing, the policy blobs at `base`, and **the collector's result file** (PB §7.4 rule 3), which is not a git object — it lives at `.spine/cache/results/<T>.jsonl`, gitignored and ephemeral. PB §1.1 already draws this line: "every hash, signature, base and gate query recomputes offline. The one input it cannot recompute is whether the candidate's own runner reported honestly."

So each member of the schema is marked **recomputable** or **attested**:

- **recomputable** — derivable from the landing commit `L`, the objects it reaches, the policy blobs at the seal's `base=`, and the seal's own fields, including by querying a graph rebuilt `--fresh` from exactly those objects (§7 rule 3). `--verify` rebuilds these and any difference is a finding.
- **attested** — derivable only from the result file, from refs that no longer exist, or from the run's own memory. `--verify` takes these verbatim from the candidate report it was given. Attested is not unpinned: the seal's `report=` covers them like every other byte, so altering one produces a mismatch.

The attested set is small and closed:

| Member | Why attested |
|---|---|
| `evidence` (whole object) | The result file's digest and header; not a git object. |
| `run.reverifications` | A counter the run holds in memory and nothing may store (PB §5.4, `C-M3`). |
| `automerge.preconditions[2].status` | The collector's key-visibility assertion, and whether *this run* established that the file it ingested came from a trunk-defined job (PB §7.4 rule 0; §5.8). The first is read from a header that is not a git object; the second from provider facts that exist only while the CI run does. The conjunct's *arrangement* half is recomputable — `params.ci` is inside `policy.manifest` at `base`, so a verifier can tell whether such evidence was **available** — but whether the run **obtained** it is not, and a member one clause can move is attested. |
| `automerge.preconditions[4].status` | Whether *this run* performed the CAS. |
| every `gates[]` entry whose `gate` is `G1` | Their checks read test outcomes. |
| every `wires[]` entry whose `gate` is `G1` | Same. |
| every `gates[]` entry whose `gate` is `G8` | One clause of G8's check reads test outcomes — on **`T` and on `B` both**, since PB §6.3's `xfail`/`skipped` carve-out is decided by the `out` member of the result file's `base` records (`docs/spec/result-file.md` §4.4) and no git object carries it. PB §6.3 G8 fails on "a landed id `T` no longer collects **or does not pass**", and PB §4.3 makes an id collected on `B` and absent from `T`'s collection a G8 failure. `docs/spec/result-file.md` §2 names G8 a reader of the file's `base` and `result` sections and §8.5 clause 2 allocates that finding between G8 and G1. G8's other clauses — the frozen-blob comparisons, the closure recomputation, the `C-T3` tree grep — are recomputable, but the member records one **status**, and a clause that can move it is enough to make the member attested. |
| every `wires[]` entry whose `gate` is `G8` | Same. A `G8:<path>` wire raised by the blob clauses would be rebuildable; one raised by the landed-id clause is not, and the two are not distinguishable in the record. |
| every `gates[]` entry whose `gate` is `G7` | The binding lease check reads the integrated diff against every other in-flight intent's forbidden and frozen sets, over a fresh fetch of `refs/heads/intent/*` at PB §5.4 step 3. Those refs are deleted when their intents land or withdraw, and the lease registry is derived from them alone (PB §5.4) — which is why PB §6.3 says of the proof itself, "G10 proves the ledger, not the lease registry". A year later there is nothing to rebuild the sets from. |
| every `wires[]` entry whose `gate` is `G7` | Same. |

Everything else — including `profile`, which the seal carries, and preconditions 0, 1 and 3, which the constitution, manifest and the absence of a deferred mode settle — is recomputable.

**Recomputable is relative to two documents this one does not own.** `policy.rules` is only as determined as `docs/spec/constitution.md` (§5.4.1), and G8's *recomputable* clauses — the freeze-closure recomputation PB §4.3 requires of every `--ci` run — are only as determined as `docs/spec/import-resolver.md` (§5.4.2). Each of the four v1 languages owes a total, deterministic `id → fn`, an `id → path`, and a static import resolver; G8 recomputes the closure over the approval commit's tree with the pinned release, so a resolver that differs from another's on one edge case does not merely disagree — it computes a different closure, fails or passes G8 differently over identical objects, and thereby rejects the other implementation's approvals and reports `report-mismatch` on its landings. That is the property, not a defect in it: the whole point of specifying the resolvers is that "recomputable" means *by anyone*, and a member whose value depends on an unspecified resolver is recomputable only by its author.

### 4.1 Where `--verify` gets a candidate report

`--verify` is a *comparison*, so it needs a candidate: `spine check --verify <landing-sha> [--report <path>]`. Resolution order:

1. `--report <path>`, if given;
2. the `refs/notes/spine` note on `<landing-sha>`, if present in this clone (§4.4);
3. otherwise: status `report-unavailable`, exit 2.

Path 2 is the ordinary path, not the fallback: PB §7.4 rule 4 requires the trusted stage to publish the note on every landing, so its absence in a clone that has fetched the ref means either that the fetch did not include it (§4.4.5) or that the pipeline failed to publish — a defect to file against the pipeline, never against the landing.

**The candidate's own bytes are checked against the seal first.** `sha256` over the candidate's exact bytes must equal the seal's `report=`. If it does not, `--verify` stops: status `candidate-mismatch`, exit 1, printing both digests. It does not proceed, because a candidate that is not the sealed report supplies attested members (§4) that no signature covers, and running the recomputation on them would produce a second mismatch whose message names the wrong culprit. The check is redundant with the final comparison — a candidate that fails it could never produce a matching recomputed digest — and it is required anyway, because it is the only check that can distinguish *this is not the report the seal names* from *this is the sealed report and it does not describe these objects*. The first is a stale, truncated or forged copy; the second is tampering or a defect in spine.

**This does not make a note a source.** PB §7.4 rule 4 requires the note and still forbids depending on it; PB §1.1 forbids provider metadata as truth. Nothing here believes the candidate: it is admitted only because a signed trailer already fixes its digest, every recomputable member is then thrown away and rebuilt, the attested members are copied in, and the result is hashed against `report=` in that *signed* trailer. A forged or stale candidate produces a mismatch, never a false accept. This is the same discipline configuration (b) applies to a PR body in PB §5.4 — read it back, hash it, refuse it if it disagrees.

### 4.2 What `--verify` cannot do

Exit 4 is a property of the objects, not of `subject.strategy`. `--verify` refuses as `not-recomputable` when the report's gate results were computed over a tree built from `objects.head` and `objects.head` is unreachable — in practice every `Spine-Event: land` under `C-M1: squash`, gated or quick, where `L` has one parent and the CAS deleted the only ref that held `H`. PB §7.4 rule 4 says this in one clause ("Recomputation needs `H` reachable, which is why the gated lane defaults to merge strategy"); the exit code is this document's. Under squash the audit degrades to the seal plus G9's freeze audit, exactly as PB §5.5 states, and `--verify` says so rather than pretending.

**A tombstone and a reseal are recomputable whatever `subject.strategy` records.** Both carry the repository's `C-M1` value (§5.1), and neither depends on it: a tombstone's `objects.tree` is `B`'s tree, built at PB §5.4 step 2 with no suite and no merge tree over `H`; a reseal's `Hc` is `O`, a first-parent commit of trunk, which no landing deletes. Reading exit 4 off `subject.strategy` alone would refuse every tombstone and every reseal in a squash repository for a fact neither rests on.

### 4.3 Exit codes

| Exit | Status | Meaning |
|---|---|---|
| 0 | `verified` | Recomputed report's digest equals the seal's `report=`. |
| 1 | `report-mismatch` | The candidate was the sealed report and the recomputation disagrees with it. The recomputed report is printed; the difference is a defect to file against spine, or evidence of tampering. |
| 1 | `candidate-mismatch` | The candidate's own bytes do not hash to the seal's `report=` (§4.1). It is not the report this seal names — a stale, truncated or forged copy — and nothing was recomputed from it. |
| 2 | `report-unavailable` | No candidate report: no `--report`, and no note on the landing in this clone (§4.4.4). |
| 3 | `wrong-release` / `wrong-git` / `report-version-unknown` | Preconditions for recomputation not met. |
| 4 | `not-recomputable` | `objects.head` is unreachable and the evaluation needed it — a `land` under squash strategy. |

**The order is normative**, because two implementations that check the same things in a different order report different statuses for a clone that is wrong in two ways at once:

1. the seal's `tool=` against the running binary, and its `git=` against the parsed `git --version` (§3.3) — exit 3, before any candidate is read, since neither depends on one;
2. resolve a candidate (§4.1) — exit 2 if there is none;
3. `sha256` over the candidate's exact bytes against the seal's `report=` — exit 1 `candidate-mismatch`, before the candidate is parsed, because bytes that are not the sealed report are not worth parsing;
4. parse it; an unknown `report_version` or an unknown member name — exit 3 `report-version-unknown` (§3.2);
5. recomputability of the objects the evaluation needed (§4.2) — exit 4;
6. rebuild, copy the attested members in, canonicalize, compare — exit 0 or exit 1 `report-mismatch`.

Both exit-1 statuses are failures of the *copy* or of the *record*, never of the landing. A landing is valid because its seal verifies against the keyring at its base; `--verify` re-derives the judgement that seal covers, and every outcome above except `verified` says only that the re-derivation could not be completed or did not agree.

### 4.4 Publication to `refs/notes/spine`

PB §7.4 rule 4 as of v0.19: **"The trusted stage publishes the full report to `refs/notes/spine`, and that is not optional."** §1.1 sells an offline clone that can re-verify; without the published report, `--verify` reaches only whoever still holds the CI run's artifacts, which is nobody a week later, and the claim would be true of the seal and of G9's freeze audit but not of the judgement. This section fixes what "publishes" means byte-for-byte, because a note whose bytes are not exactly the canonical form verifies nothing.

#### 4.4.1 The ref, the object, and the bytes

| | |
|---|---|
| **Ref** | `refs/notes/spine`, written in full. The porcelain shorthand is `--ref=spine`; the ref name is normative and no other notes ref carries a gate report. |
| **Annotated object** | the **landing commit `L`** — the commit that carries the `Spine-Seal` whose `report=` these bytes hash to, and the object `spine check --verify <landing-sha>` names. Never the tree, never `Hc`, never `objects.tree`, never an envelope blob. One landing, one note. |
| **Note content** | exactly the canonical bytes of §2 for the report that landing's seal names — the same bytes `report=` is a SHA-256 of. No trailing newline, no BOM, no framing, no pretty-printing, no header, no signature, nothing appended. |

**The consequence is the test:** for a landing `L`,

```
git cat-file blob $(git notes --ref=spine list <L> | cut -d' ' -f1) | sha256sum
```

reproduces the hex of that landing's `report=`. A publisher whose note fails this has not published the report, whatever it wrote.

**Which evaluation is published.** The one the seal names — evaluation 2 of §8.1, the report containing every review that still binds and no `fail`. The review-stage evaluations are **not** published and get no note: a reviewer's `report=` names a report that never sealed, `--verify` verifies a landing rather than a review, and a second note per landing would give the ref two objects a reader could confuse. A reviewer who wants to check what they signed recomputes it from the same objects (§8.1 shows the two differ in exactly two members).

#### 4.4.2 How it is written

The note is created from a blob holding the canonical bytes, never from a message:

```
blob=$(printf '%s' "$canonical" | git hash-object -w --stdin)
git notes --ref=spine add -C "$blob" <L>
git push origin refs/notes/spine
```

`-m`, `-F` and the editor paths are **non-conforming**: git terminates a note message with a newline, and a note carrying one trailing `0x0A` hashes to something that is not `report=`. `-C <blob>` reuses the object's bytes verbatim, which is the only write path this document admits.

**When.** After the CAS of PB §5.4 step 6 has made `L` trunk's tip, and never before — a note on a commit that lost the CAS annotates an object no ref reaches, and the next `git gc` on the server takes the commit and leaves a dangling note. Publication is therefore *outside* the landing's atomic step, which is why the next two rules exist.

**Failure to publish does not retract the landing.** The landing is complete when the CAS succeeds; the note is a separate push of a separate ref. A failed note push **fails the CI job** — so that a human sees it and retries — and changes nothing about `L`, its seal, or the ledger. Nothing in the design can un-land a commit that reached trunk (PB §5.4), and a rule that tried would make the audit trail's transport an input to the ledger's validity.

**Republication is idempotent; overwriting is refused.** Re-publishing byte-identical content for a commit that already carries it is a no-op and is the correct response to a retry. Publishing *different* content for a commit that already carries a note is refused: the seal fixes one report per landing, so a second one is either a defect or an attack. `git notes ... add -f`, `append`, `edit` and `remove` are never part of publication, and a repository that finds two distinct reports for one landing has a finding for a human, not a merge to perform.

**Concurrency.** `refs/notes/spine` is one ref and every landing pushes it, so concurrent landings race. A rejected non-fast-forward push is answered by fetching the ref, re-applying this landing's note to the refreshed ref, and retrying — bounded, and never with `--force`. Notes for distinct commits never conflict; two publishers writing the same commit are the idempotent case above.

#### 4.4.3 A note commit carries a clock, and nothing reads it

A note is stored in a commit, and every git commit carries author and committer dates. That does not breach §7 rule 1 or PB §7.5's "one clock and it is the chain": the timestamp is on the *notes* commit, not in the report, not in the digest, not in the envelope, and not in the ledger. No gate reads it, `--verify` does not read it, `spine index` does not walk `refs/notes/*`, and `spine stats` counts landings, not notes. A reader that derived anything at all from a note commit's date would be deriving it from an object anyone can rewrite without detection.

For the same reason the report does not record that it was published. Publication happens after the report is hashed into the envelope, so a `published: true` member would be a claim about the future — the same ordering trap §5.8 names for auto-merge precondition 3, and worse, because it would also be self-referential (§7 rule 11).

#### 4.4.4 When the note is missing

`--verify` exits 2, `report-unavailable`, and says which of the two causes applies where it can tell them apart: the clone has not fetched `refs/notes/spine` (§4.4.5), or the ref is present and carries no note for this commit.

**The landing is still valid.** PB §7.4 rule 4 is explicit that a missing or edited note is a lost convenience, never an invalid landing; the seal, the freeze audit and G9's ledger walk are unaffected, and a repository whose notes ref was deleted entirely still has every guarantee the chain gives. What it has lost is third-party recomputation of the judgement.

**And it may have lost it permanently.** A missing note is not always recoverable, and this is the reason publication is mandatory rather than best-effort: the recomputable members can be rebuilt by anyone from the landing's objects, but the **attested** ones (§4) — `evidence`, `run.reverifications`, preconditions 2 and 4, and the G1, G7 and G8 statuses — exist nowhere else once the CI run's artifacts expire. Without them no candidate can be assembled that hashes to `report=`, so a lost note is a landing whose judgement is permanently unverifiable by anyone. Re-running the pipeline does not help: a new run produces a new result file and a new report, which is a different digest that no seal names.

#### 4.4.5 Fetching

Notes are not fetched by default (PB §7.4 rule 4, PB §11). A clone that wants `--verify` fetches the ref explicitly:

```
git fetch origin '+refs/notes/spine:refs/notes/spine'
```

or configures it as an extra refspec. Spine does not install that configuration, does not fetch it implicitly during a gate run, and never fetches it during `spine check --land` or `--ci`: the trusted stage restores no cache and reads no note (§7 rule 3), and an implicit fetch during a gated run would put a mutable, unauthenticated ref inside a landing's dependency set for no gain.

#### 4.4.6 A note is never a source

Stated once, flatly, because the ref is now guaranteed to exist and a guaranteed artifact invites being read:

- **No gate reads a note.** Every gate of §5.6.2 reads commits, trees, blobs and the result file, and nothing else. There is no clause in PB §6.3 that consults `refs/notes/*`, and adding one would be a change to the ledger's authority, not an optimization.
- **The ledger derives from commits alone** (PB §5.4, PB §6.2). `spine index --fresh` walks commits; a note is not among the objects G10 proves the graph reconstructible from, so a graph that read one could not be proved at all (§7 rule 3).
- **`--verify` reads a note and believes nothing in it** (§4.1): it is admitted only after its bytes hash to a value a signature already covers, its recomputable members are then discarded and rebuilt, and only the attested residual PB §7.4 already concedes survives.
- **Nothing degrades when a note is absent, edited or forged.** Absent: exit 2. Edited or forged: `candidate-mismatch`, exit 1. In neither case does any landing's status, any gate's result, or any ledger state change, because none of them was ever a function of the note.

---

## 5. The schema

Top-level members. JCS sorts them by name; the table is in reading order. **R** = recomputable, **A** = attested.

| Member | Type | Presence | R/A | Value |
|---|---|---|---|---|
| `report_version` | integer | always | R | `1` |
| `subject` | object | always | R | §5.1 |
| `objects` | object | always | R | §5.2 |
| `tool` | object | always | R | §5.3 |
| `git_version` | string | always | R | §5.3 |
| `object_format` | string | always | R | `"sha1"` \| `"sha256"`, from the manifest at `base` (PB §6.7). Fixes oid length: 40 or 64 lowercase hex. |
| `mode` | string | always | R | `"solo"` \| `"team"` \| `"recovery"` (PB §11). Equals `policy.rules.c_a1` except on a recovery-sealed landing (PB §7.5), where it is `"recovery"`. |
| `threat` | string | always | R | `"hostile"` \| `"trusted"`. Equals `policy.rules.c_a3`. |
| `profile` | string | always | R | `"container"` \| `"uid"` \| `"none"` \| `"n/a"` (PB §7.4 rule 3, PB §11) — the seal's `profile=`, so recomputable from the seal. `"n/a"` iff the landing runs no suite, which PB §11 makes the tombstone and nothing else. A landing that ran gates and ingested no file records `"none"` and no `evidence` (§5.9). **A landing that ingested a file whose trunk-defined origin was not established records the header's own value** — `"container"`, `"uid"` or `"none"` as that header reports it, never a downgrade (`docs/spec/result-file.md` §8.4). The doubt is `automerge.preconditions[2].status`, not this member, and the two are read together (§5.8, §9.25). |
| `policy` | object | always | R | §5.4 |
| `authority` | object | always | R | §5.5 |
| `self_approved` | boolean | always | R | `true` iff some entry of `authority.reviews` has `self_approved: true`. |
| `gates` | array | always | mixed | §5.6 |
| `wires` | array | always (may be empty) | mixed | §6 |
| `floor_hits` | array of string | always (may be empty) | R | §5.7 |
| `automerge` | object | always | mixed | §5.8 |
| `evidence` | object | present iff a result file was ingested | A | §5.9 |
| `run` | object | always | A | §5.10 |

There is no timestamp, no duration, no hostname, no environment capture, and no counter of anything outside this run. PB §7.5's rule is the whole rule: **one clock and it is the chain.**

### 5.1 `subject` — which landing this is

| Member | Type | Presence | Value |
|---|---|---|---|
| `lane` | string | always | `"gated"` \| `"quick"` (PB §11 `Spine-Lane`). Toolkit lifecycle landings are `"quick"` (PB §6.7). |
| `event` | string | always | `"land"` \| `"withdraw"` \| `"reseal"` (PB §11 `Spine-Event`, landing values only). |
| `intent` | string | present iff the landing has an intent id | The bare id, e.g. `"INT-042"` or `"BUG-051"`. Matches `^(INT|BUG)-[0-9]+$`. Never the repo-scoped graph id (PB §6.2). |
| `strategy` | string | always | `"merge"` \| `"squash"` (`C-M1`, PB §11 `Spine-Strategy`). A reseal and a tombstone record the repository's `C-M1` value; their tree rule is their parent's either way (PB §5.5), and §4.2 does not read this member. |

**The seal's first field is derived, not stored:** `subject.intent` when present; else `"reseal"` when `subject.event == "reseal"`; else `"quick"`. Storing it would be a second spelling of the same fact.

**This member is not the landing commit's subject line, and the collision of names is worth pinning.** PB §5.5 as of v0.19 makes that line **derived, not written** — a pure function of the envelope, `<id>: <the intent's title>` for a gated landing, `quick: <summary>` for the quick lane, the tombstone and reseal forms of PB §11 — and **G9 recomputes it and refuses a landing whose subject it did not produce**. It stays **outside `envelope=`**, so it is covered by no digest, moves no signature, and is **not a member of this report**: `subject` here records *which landing this is*, in four enumerated members a verifier recomputes from the seal. A reader wanting the commit's first line reads the commit. The residual PB §5.5 names travels with it and is `lane`-shaped: the quick lane's summary is free text, and PB §6.7 routes **every toolkit lifecycle landing** through the quick lane, so a report reading `{"lane": "quick", "event": "land"}` may sit under **any summary after the mandatory `quick: ` prefix** — PB §11's *Subject lines* checks the seven-byte prefix and a non-empty remainder and fixes nothing beyond them — which is why nothing here derives anything from that line.

### 5.2 `objects` — what the record is bound to

A gate record is bound to `(head, base, tree)` and is void the instant either ref moves (PB §5.4).

| Member | Type | Presence | Value |
|---|---|---|---|
| `base` | oid | always | `B` — the trunk tip the run fixed. For a reseal, the last valid landing below the range (PB §5.5). |
| `head` | oid | always | The **content head** `Hc` (PB §5.4): `H`'s nearest ancestor that is not an empty `Spine-Event: review` commit. Never the literal ref tip. |
| `ref` | string | always | The ref the run names, `esc`-encoded: `refs/heads/intent/<ID>`, `refs/heads/quick/<name>`, `refs/heads/spine/upgrade-<version>`, `refs/heads/quick/reseal-<O>`. |
| `merge_base` | oid | always | `git merge-base <base> <head>` — the left end of the diff G14 matches over (PB §6.3). |
| `tree` | oid | always | `T := git merge-tree --write-tree B Hc` — **the tree the gates evaluated**, with the intent file still in it. On a tombstone, `B`'s tree (below). |
| `intent_blob` | oid | present iff `subject.intent` is present | The signed intent blob: the sign-off's `blob=` on a gated landing, the `Spine-Withdraw` line's `blob=` on a tombstone (PB §4.3, PB §5.5). |

**A tombstone has no merge tree and may have no sign-off.** PB §5.4 step 2 builds it with parent `B`, tree identical to `B`'s, and no test run, skipping the step that computes `T`. So `objects.tree` is `B`'s tree — which is also `L`'s tree — and `objects.intent_blob` is the `Spine-Withdraw` line's `blob=`. Where a sign-off is copied, PB §5.5 requires the fenced bytes to hash to both, so the two are equal; an **orphaned** tombstone, whose sign-off is omitted because its key has left the keyring at `base`, carries the withdraw line alone and `intent_blob` is that line's. Recording the sign-off's blob unconditionally would leave the member undefined for exactly the landing that has no sign-off.

**`objects.tree` is `T`, not `L`'s tree.** PB §11's `Spine-Seal` carries `tree=<L's tree>`, which under merge strategy is `T` with `intents/<ID>.md` deleted (PB §5.5) — a different oid. Under configuration (b) the two coincide: the candidate's last content commit already deletes the intent file, so the provider's squash tree equals the sealed `tree=` and equals `T` (PB §5.4). PB §5.4's review row and G9's review clause both compare against `merge-tree(review.base, L^2)`, which is `T` in every configuration. Recording `T` is the choice that makes the report a record of the evaluation; `L`'s tree is derivable from `T` and the intent path and is not stored. See §9.2.

Because review commits are empty, adding one changes `H` but not `Hc`, not `T` and not `merge_base` — which is why the review-stage and seal-stage reports of §8 differ in exactly two members.

### 5.3 `tool` and `git_version`

| Member | Type | Value |
|---|---|---|
| `tool.version` | string | The release version, e.g. `"1.4.0"`. `esc`-encoded; in practice ASCII. |
| `tool.dist_hash` | string | `"sha256:<64 lowercase hex>"` — the release's artifact-list digest (PB §6.7). |
| `git_version` | string | `"<major>.<minor>"`, e.g. `"2.45"`. Patch level, release-candidate suffixes and vendor suffixes are discarded before recording, by the parse below. |

**The parse is normative, because a mis-parse forks both the digest and §3.3's `wrong-git` check.** Over `git --version`'s output: take the first maximal run of ASCII digits, then the first maximal run of ASCII digits following the next `.`; record the two joined by `.`. `git version 2.39.5 (Apple Git-154)` → `"2.39"`; `git version 2.45.1.windows.2` → `"2.45"`; `git version 2.46.GIT` → `"2.46"`. Output from which two such runs cannot be read is a refusal: no report is produced, and `--verify` exits 3 `wrong-git`. Nothing else in this document reads a version out of a version string.

`tool.version + "+" + tool.dist_hash` is exactly the seal's `tool=` field (PB §11). `git_version` is exactly the seal's `git=`. Both must equal the manifest's pin at `base`, or G15 has already failed and no report exists.

### 5.4 `policy` — what governed this run

Policy is read from trunk, never from the candidate (PB §7.4 rule 1). Every blob id below is the blob **at `objects.base`**; for a reseal that is the seal's `base=`, never `O` (PB §5.5).

| Member | Type | Presence | Value |
|---|---|---|---|
| `manifest` | oid | always | `.spine/manifest.json` |
| `keyring` | oid | always | `.spine/allowed_signers` |
| `constitution` | oid | always | the manifest's `paths.constitution` |
| `ci_sh` | oid | always | `.spine/ci.sh` |
| `floor_source` | string | always | `"spine:<tool.version>:floor"` — the provenance token of PB §6.1 for the floor list shipped inside the pinned release. |
| `floor_extensions` | array of string | always (may be empty) | The floor's repository-side extensions: every entry of `C-A2` plus every value of every `paths.*` key in the manifest at `base` (PB §6.7, PB §7.3). `esc`-encoded, deduplicated, sorted ascending by encoded bytes. |
| `rules` | object | always | The twelve scaffolded constitution rules, §5.4.1. |

**A list-valued `paths.*` key contributes one entry per element.** PB §6.7 fixes the shape: "`paths` is an open map whose every key, present or future, names a repository path or a list of them, and every such value is a floor entry." `paths.agent_context: ["AGENTS.md", "CLAUDE.md"]` therefore yields two entries, never one stringified list. §8.2's `floor_extensions` shows both spellings resolved into one flat, sorted array.

The shipped floor list itself is not enumerated in the report. It is a constant of the pinned release, and `tool.dist_hash` pins it; enumerating it would put a hundred patterns in every landing's digest to prove a fact one hash already proves.

**No `params.*` value is a member, and `policy.manifest` is why.** The manifest at `base` carries `params.trunk`, `params.ci`, `params.isolation`, `params.langs` and `params.timeout` (PB §6.7); `policy.manifest` is that blob's oid, so every one of them is already pinned to the byte by a member the digest covers. Two are visible in the report anyway, and neither is a second spelling: `params.isolation` is compared against the ingested header inside auto-merge precondition 1, and what the report records is the *header's* answer, `profile` (§5.8); the `paths.*` keys are restated in `floor_extensions` because PB §7.3 makes them floor entries and PB §11's protected-review row reads floor hits out of the wire set. `params.langs` and `params.timeout` have no such consumer:

- **`params.langs`** — the set of languages this repository's harness is written in, and therefore which of the four v1 resolvers G8 uses to recompute the freeze closure (PB §4.3, PB §6.7). It bears on a gate's *result*, which the report records, never on the report's shape. Recording the set as well would put a value in the digest that `policy.manifest` already fixes, and give a reader two places to look when a closure recomputation disagrees.
- **`params.timeout`** — the bound on one runner invocation, in seconds, default 1800 when the key is absent, read from trunk like every other policy (PB §6.7, PB §7.4 rule 3). A collector that enforces no deadline is non-conformant. It is **also barred by §7 rule 1**: it is a duration, and no member of a gate report holds a duration. What a deadline produced is recorded where it belongs — a run the collector killed writes `status=runner-timeout` (`docs/spec/result-file.md` §7.3), no id counts as passed, and the report carries the resulting G1 finding and the `evidence` digest of the file that reports it (§5.9).
- **`params.ci`** — the provider arrangement, and since the amendment of 2026-08-26 it decides whether auto-merge precondition 2's third conjunct is **reachable at all**: `github` supplies trunk-defined origin evidence, `gitlab` with in-repository configuration and `generic` never do (`docs/spec/ci.md` §10.3). It is not a member for the same reason as the others — `policy.manifest` pins it — and recording it would tempt a reader to compute precondition 2 from it, which is wrong in the one direction that matters: an arrangement that *can* supply the evidence is not a run that *did*. What the report records is the run's answer, `preconditions[2].status`, which is attested (§4) precisely because the second half of that test is not in any git object.

#### 5.4.1 `policy.rules`

Keys are the rule ids of PB §2.1 lowercased with `-` → `_`. Values are what the constitution parser (`docs/spec/constitution.md`) yields, in the order it yields it — this spec does not re-parse the constitution and does not reorder its lists.

| Key | Rule | Type | Domain |
|---|---|---|---|
| `c_a1` | mode | string | `"solo"` \| `"team"` |
| `c_a2` | protected | array of string | `esc`-encoded paths, file order |
| `c_a3` | threat.candidate | string | `"hostile"` \| `"trusted"` |
| `c_m1` | merge.strategy | string | `"merge"` \| `"squash"` |
| `c_m2` | merge.reverify | string | `"full"` \| `"scoped"` |
| `c_m3` | merge.reverify_limit | integer | ≥ 0 |
| `c_m4` | merge.auto | string | `"on"` \| `"off"` |
| `c_q1` | quick.paths | array of string | `esc`-encoded, file order |
| `c_q2` | quick.max_lines | integer | ≥ 0 |
| `c_t1` | test roots | array of string | `esc`-encoded, file order |
| `c_t2` | test support | array of string | `esc`-encoded, file order |
| `c_t3` | no test-framework import or runner-hook definition outside the harness (`C-T1` ∪ `C-T2`) | boolean | `true` iff the rule is present and in force |

A scaffolded rule missing from the constitution at `base` fails G16's scaffold check before a report exists, so every key above is always present. A team's own `C-<n>` rules are **never** in the report: PB §2.1 makes them a health report with no gate id, so they cannot be named in a review's `wires=` and cannot bear on a landing.

**`c_t3` is `true` in every version-1 report, and that is the answer, not an oversight.** `C-T3` carries no value to parse — it is a prohibition G8 enforces as a tree grep (PB §2.1, PB §6.3) — and G16 requires all twelve scaffolded rules at `base`, so "present and in force" cannot be false while a report exists. It is recorded because `policy.rules` records the twelve, and a schema that dropped the one constant member would make every reader ask which one is missing and why. A constitution grammar that later admits an aspirational or negated `C-T3` changes what this boolean can say, and that is a `report_version` bump.

**Normative precondition.** `policy.rules` is only as determined as `docs/spec/constitution.md`. Two implementations can agree on every byte of this document and still disagree on `policy.rules` until that document fixes the list splitting, the whitespace handling and the yield order. This spec is normative for v1 **alongside** it, not before it; it is one of two external dependencies that can invalidate every vector in §8 without touching a line here (§5.4.2, §11).

#### 5.4.2 The second normative precondition: the per-language resolvers

The other is `docs/spec/import-resolver.md`. **v1 ships four languages — Python, TypeScript/JavaScript, Dart and Swift** (PB §6.7, as settled by the owner on 2026-08-26) — and each owes three total, deterministic functions: `id → fn` and `id → path` over runner-native test ids, and a static import resolver over the tree. A language absent from that list cannot be gated; adding one is a release, not a repository setting.

**Kotlin was the fifth and was dropped, and the reason is a `gates[]` reason.** In a mixed Kotlin/Java module an oracle in a `.java` file is invisible to a Kotlin resolver, so the freeze closure omits edges and **nothing reports the omission** — G8 writes `"pass"` over an approval it should have refused, and the status is indistinguishable from an honest one. Every other way a resolver can be wrong shows up as two implementations disagreeing, which this document's whole recomputation argument is built to surface (below); a resolver that is *silently incomplete* produces agreement between two equally blind implementations, and `--verify` confirms it. That is not a gap in the resolver spec this document can be normative alongside. **Swift's Objective-C hole is the same failure class and is closed the other way, by one rule rather than two judgements:** a target that compiles Objective-C or C-family sources hides an oracle from a Swift resolver identically, so `docs/spec/import-resolver.md` §7.3 refuses it — `lang-unclassifiable`, reason `mixed-objc-target` — and a `G8` status over such a repository is a reported refusal rather than a `"pass"` two blind implementations agree on. Removal and detect-and-refuse are the two outcomes the rule selects between; neither language ships a silently-incomplete closure. `kotlin` stays reserved as a `params.langs` value and `gradle`, `junit` and `kotest` as `runner` tokens (`docs/spec/result-file.md` §6.4); no report member names any of them, so nothing here changes shape.

Nothing in this schema names a language, and no member holds a resolver's output. The dependency runs through gate *status*:

- **G8** recomputes the freeze closure over the approval commit's tree on every `--ci` run and fails if any file it computes is missing from `Spine-Frozen` (PB §4.3). The closure is the transitive repo-local import walk, so `gates[]`'s `G8` entry is the resolver's verdict wearing a status. Two implementations whose resolvers differ on one edge case — a re-export, a conditional import, a path alias, a package `__init__`, a type-only import — compute different closures over the same tree, write different `G8` statuses, and produce different digests over identical objects. That is not a disagreement they can live with: it makes one reject the other's approvals and report `report-mismatch` on the other's landings.
- **G1**'s AC-coverage clause and **G5**'s orphan clause join a source symbol to a runner-native id (PB §6.3, §5.9 below). The join is specified in neither this document nor `docs/spec/result-file.md`; it is `docs/spec/import-resolver.md` §12's, and **that document now supplies it** — §12.1 fixes the pragma's grammar, §12.2 makes the join file-granular ("a pragma occurrence in file `P` attributes to every collected test id whose `id → path` equals `P`"), and §12.3 fixes the naming sugar per runner. `gates[]`'s `G1` and `G5` entries are determined up to that section, which is the same normative-alongside relationship §5.4.1 has with the constitution.

So the precondition is the same as §5.4.1's, one document over: this spec is normative **alongside** `import-resolver.md`, not before it, and a `G8` status is recomputable exactly to the extent that document fixes the walk. §11 records it as the second out-of-scope pointer that can invalidate this spec without touching it.

### 5.5 `authority` — the verified signed statements

A report exists only for a run whose PB §5.4 step-2 bindings verified; a mismatch voids the record and nothing else runs. So the report never records an unverified statement and carries no `verified` flag.

Every signed statement has the same shape:

| Member | Type | Value |
|---|---|---|
| `line` | string | The trailer line exactly as it appears in the commit message, excluding the line terminator, `esc`-encoded. A reader records the bytes it finds and normalizes nothing. |
| `fingerprint` | string | The SSH public key fingerprint the signature verified with, in `ssh-keygen -lf` form: `"SHA256:"` plus unpadded base64. This, not the principal, is what `reviewer ≠ signer` compares (PB §7.2). |
| `namespace` | string | The namespace the signature verified under: `"spine-signoff@v1"` \| `"spine-review@v1"` \| `"spine-seal@v1"` (PB §7.2). Roles are derived from this, never claimed. |

**`<name>` `:` one space `<payload>` is a writer's constraint, not a reader's rewrite.** Spine emits that shape (PB §7.2, PB §11) and so does every trailer it copies into an envelope. A reader records what it finds — two spaces, no space, anything — because the signature is over the line's exact bytes, so a line that is not what it should be fails G13's verification, and that refusal is the check. A report that silently reshaped the line would hash bytes nobody signed.

The `-Sig` line is not recorded: it is a git object in the envelope, and duplicating it into a hashed artifact buys nothing a verifier does not already have.

| Member | Type | Presence |
|---|---|---|
| `signoff` | statement | Present iff a `Spine-Signoff` binds this landing (gated landing; tombstone where one exists and its key is in the keyring at `base`, PB §5.5). |
| `approve` | statement | Present iff a binding `Spine-Approve` exists (gated landing only; never on a tombstone, quick, lifecycle or reseal landing). |
| `reopens` | array of statement | Every `Spine-Reopen` on the branch, ancestor-first (§5.5.1). Empty array when there are none. |
| `reviews` | array of review | Exactly the `Spine-Review` lines that bind **this** evaluation, ancestor-first (§5.5.1). Empty array when there are none. |
| `upgrade` | statement | Present iff the landing carries a copied `Spine-Upgrade` (PB §6.7). |
| `withdraw` | statement | Present iff `subject.event == "withdraw"`. |

**Which reviews bind.** A `Spine-Review` is a member of `reviews` iff its `head=` equals `objects.head` and its signature verifies under `spine-review@v1` against the keyring at `objects.base` — the two bindings PB §5.4 step 2 checks before anything else runs. A review a content push voided — its `head=` names a superseded `Hc` — is **absent**, not recorded with a flag and not recorded at all: PB §5.4's review row says a content push changes `H` and voids it, leaving `tree=` and `report=` as audit data, and a void statement inside a digest a seal covers would make the seal attest to a record the design has already discarded. Leaving membership to "present on the branch" is the difference between one implementation that includes a stale review and one that drops it — two digests over identical facts.

A **review** is a statement plus one computed member:

| Member | Type | Value |
|---|---|---|
| `self_approved` | boolean | `true` iff this review's `fingerprint` equals **the landing's signer key** (PB §7.2). |

**The landing's signer key** is `authority.signoff.fingerprint` when present, else `authority.upgrade.fingerprint` when present, else none. A landing with none is a **signerless** landing — every quick-lane landing that copies no `Spine-Upgrade`, every reseal, and an **orphaned tombstone**, whose sign-off is omitted because its key has left the keyring at `base` and whose withdraw line names it `orphaned=<principal>` (PB §5.5) — and every review on it has `self_approved: false`, because there is nobody to be self. A toolkit lifecycle landing rides the quick lane but copies a `Spine-Upgrade`, and PB §6.7 says plainly that this is what gives that landing a signer. Signerlessness is exactly why PB §11's signerless overlay demands two distinct protected reviewers in team mode instead: the separation has to come from the reviewers.

A review's `class=`, `wires=`, `head=`, `tree=`, `base=`, `intent=` and `report=` are **not** re-encoded as members. They are inside `line`, parsed by the envelope grammar, and a second spelling in a hashed artifact is a second thing to disagree about.

#### 5.5.1 Ordering of `reopens` and `reviews`

Ancestor-first along the branch's first-parent path: the order the commits appear in `git rev-list --reverse --first-parent <objects.base>..<objects.head>`, extended past `head` to the literal ref tip `H` for review commits, which are empty and therefore sit above `Hc`. Event commits are created on the branch tip, so they are always on that path; a `Spine-*` line on a commit that is not on it is not an event commit of this branch and is not recorded.

Two event commits cannot carry byte-identical signed lines — G13 refuses that (PB §6.3), **outright**, no review discharging it (`docs/spec/manifest.md` §4.8.4 check 3) — so the order is total.

### 5.6 `gates` — per-gate results

An **array**. `gates[]` sorts by gate number ascending — an array rather than an object because gate order is numeric and JCS would sort `g1, g10, g11, …, g2` by name. **`wires[]` does not**: PLAYBOOK §11 fixes the wire order as ascending by unsigned byte value over the whole token, so `G11` precedes `G2`, and §11 wins. The two orders differ deliberately and an implementation that applies one to the other produces a different `report=` over identical findings.

**This is the one place the corpus states the *reason* the two orders differ; it is not the only place either order is stated, and maintaining it means grepping.** `gates[]` is keyed by a gate and nothing else, so a numeric key is total and an implementer never has to think about it; a **wire token** carries an optional path after the gate id, so it is a *string* whose order must be defined over its whole length, and the only order every implementation already has for a string is its bytes. §6.1 and §6.2 adopt the byte order for the `wires[]` array and for a `Spine-Review`'s `wires=` line respectively, restating the rule without re-deriving it.

**The complete inventory of the wire order, as a grep list.** `grep -n "unsigned byte value" PLAYBOOK.md docs/spec/*.md` finds every one of them; an edit to any must be checked against all. **Normative:** `PLAYBOOK.md` §11's `Spine-Review` row (the source — "ascending by unsigned byte value over the whole token, so `G11` precedes `G2`"). **Restatements in this document:** §5.6 (here), §6.1, §6.2, §9.19. **Elsewhere:** `docs/spec/envelope-vectors.md` §4.2 (the trailer-line sort, a different key over whole lines — read it and do not confuse the two), §7 rule 12, §8.3 item 5 and §14 D3; `docs/spec/dump.md` §6.4 and §7.2; `docs/spec/manifest.md` §5.10 (G14's producer), §7 rule 5 and §8.4. **Every literal `wires=` in the corpus is `wires=G11,G2:src/shared/util.ts`** (PB §5.5, EV §8.3, GR §8.1 and §8.2), and every `gates[]` and `Spine-Gates` rendering is numeric. A statement of the *numeric* order anywhere is a defect: §9.19 withdrew that reading by name.

Each entry:

| Member | Type | Value |
|---|---|---|
| `gate` | string | `"G1"` … `"G16"` |
| `status` | string | `"pass"` \| `"override"` \| `"fail"` |

#### 5.6.1 The status domain

PB §11 fixes the sealed vocabulary: a `Spine-Gates` entry is `pass` or `override`, and "a gate that ran and passed its own check reads `=pass` even when it raised a wire". This spec adds exactly one value, `fail`, for evaluations that do not seal:

- **`pass`** — the gate ran and produced no *finding*, and no break-glass review names it. A gate may read `pass` while having raised wires (§6.1).
- **`override`** — the gate ran and either (a) produced at least one finding and every finding is covered by a signed review whose class admits that wire (PB §11 wire aggregation), or (b) is named in the `wires=` of a `class=break-glass` review, among the eight gates PB §7.6 permits it to bypass — G1, G2, G3, G4, G6, G7, G8, G12.
- **`fail`** — the gate ran, produced at least one finding, and at least one is uncovered.

**A finding may be *outright*, and then no review discharges it.** An outright finding is one that limb (a) never reaches: **no review class admits it**, so the gate reads `fail` whatever any `Spine-Review` names, and a review whose `wires=` carries its token changes nothing about the status. Limb (b) still applies where PB §7.6 lists the gate, and only there. The v1 outright set, named here so an implementer building from this document alone writes the same statuses:

| Gate | Outright findings | Fixed by | On PB §7.6's bypass list? |
|---|---|---|---|
| G1 | **every** finding — `result-missing`, `result-malformed`, a frozen id not passed, a landed id gone from `T`'s collection, a landed id that collected and did not pass. **An id trunk itself reported `xfail` or `skipped` on `B` that still collects on `T` is not a finding at all** (`docs/spec/result-file.md` §8.5 clause 2), so there is nothing for *outright* to be true of; see the carve-out paragraph below | PB §6's `tests-approved` row: *(blocked)*, "exits are reopen or a counted freeze override"; §5.9; `docs/spec/result-file.md` §8.7 | **yes** — a `class=break-glass` review naming `G1` reads `override` |
| G8 | a `C-T1`/`C-T2`/runner-config or approval-frozen path whose blob in `T` differs from **both** the approval tree and trunk; the intent blob differing from the signed blob | PB §6.3 G8 ("fails"); PB §6's same row | **yes** — same limb (b) |
| G13 | **every finding but one** — the keyring at `B` failing `docs/spec/manifest.md` §4.4's lint (its `keyring-*` tokens); a `Spine-Signoff`, `Spine-Approve`, `Spine-Review`, `Spine-Upgrade` or `Spine-Withdraw` line whose signature or namespace did not verify (`statement-unverified`, `statement-namespace`); `event-line-duplicate`; `approval-voided`; `reopen-voids-mismatch`; `approve-reason-missing`; `self-approved-protected`; `withdraw-key`; `signerless-review-count`; the three `chain-*` statuses; and, in flight only, `total-rounds-mismatch` and `approval-redundant`. **The exception is a `G13:<oid>` wire over an event commit whose signed line claims none of those five roles**, which is coverable — PB §6.2's "a bogus commit cannot brick it" | PB §6.3 G13 (every clause is a *refusal*); `docs/spec/manifest.md` §4.8.4, §4.8.6 | **no** — Authority is never on PB §7.6's list, which PB §6's break-glass row states as "never Authority" |
| G14 | `paths-shrank`, `c-a2-shrank`, `c-a2-bracket-case` | PB §6.3 G14 ("fails outright, review or no review"); `docs/spec/manifest.md` §5.9, §5.11 | **no** — `fail` is terminal |
| G16 | checks 1–8, 10, 11, 16, 17, and every clause of the rollback restoration rule | `docs/spec/manifest.md` §6.2, §6.7 | **no** — `fail` is terminal |
| G1, G8 **on a `Spine-Event: reseal` landing** | **none** — the G1 and G8 rows above are suspended for this landing shape: every G1 and G8 finding is admitted by the reseal's own `class=protected` review naming that finding's token, and the gate reads `override` | PB §5.5 ("a reseal is never escalated and never refused by a wire; a G1 or G8 finding inside the range is sealed `=override`"); `docs/spec/result-file.md` §8.6, §8.7 | **n/a** — break-glass is unavailable to a reseal (PB §5.5) and is not the route |

**The reseal row is normative, and without it a reseal can permanently block trunk.** PB §5.5 gives a reseal a `class=protected` review of its own — two of them in team mode, since a reseal has no signer — folds *"every wire and the floor … into the protected review's `wires=`"*, and states both halves of the discharge: a reseal *"is never escalated and never refused by a wire"*, and *"a G1 or G8 finding inside the range is sealed `=override` and counted as a freeze override, because the code is already on trunk and the only honest act is to say so."* So on this landing shape limb (a) of the status domain reaches a G1 or G8 finding: `class=protected` is a class that admits its wire. **The containment rule is unchanged** — the review must still *name* the token, exactly as it must for any other finding (above) — and so is the class: `protected`, never `tripwire`, is what admits it. The gate then reads `override` and `spine stats` counts a freeze override.

**Every exit the general rule leaves open is closed for a reseal, which is why the special case exists.** Break-glass is limb (b)'s only route to `override` for G1 and G8, and PB §5.5 says in as many words that *"break-glass is unavailable because a reseal is not an intent that reached `tests-approved`"*; `docs/spec/result-file.md` §8.7's quick-lane exits are a conforming run or promotion by `spine new --from <branch>`, and a reseal can take neither — its tree must equal `O`'s, so there is no candidate to fix, and a reseal is not promotable. Meanwhile G9 refuses every landing above an orphan until the reseal lands (PB §6.3 G9). Reading G1's and G8's rows over a reseal therefore produces a trunk nobody can ever land on again, from one hand commit. §9.6 records the choice.

**Two clauses read differently on a reseal, and both readings are fixed here.** G8's outright clause names a blob differing from *"**both** the approval tree and trunk"*; a reseal has no approval, so on this shape the clause reduces to *the blob at `O` differs from its blob at the seal's `base=`* — which PB §5.5 confirms by making a frozen path changed inside a resealed range *"a G8 failure for every intent that froze it"* — and that finding is admitted by this row like any other. G1's `result-missing` and `result-malformed` are admitted too: they are findings about G1 as a whole, PB §5.5's *"never refused by a wire"* is unqualified, and the review carries the bare `G1` (`docs/spec/result-file.md` §8.7). A **per-id** G1 finding inside the range — a landed id that did not pass on `O`, or one that went away — is admitted the same way, and the review names `G1:` + `tok(path)` (`docs/spec/result-file.md` §8.5, §6.3 below). That token is what makes this row operable at all: the containment rule requires the review to *name* the finding, so while a per-id G1 finding had no token there was nothing for a reseal's review to name, and an orphan range containing one failing landed test was a trunk nobody could land on. Such a landing seals `profile=none` with `evidence` absent (§5.9, `docs/spec/result-file.md` §8.4) rather than the ingested header a conforming reseal carries (§5.6.2).

**G13's, G14's and G16's outright findings are not in this row and stay outright on every shape, a reseal included.** For G13 the reason is direct: a reseal is discharged by `class=protected` reviews of its own, and those reviews are themselves statements G13 verifies — a reseal that suspended G13 would be a reseal authorized by signatures nobody checked. They cannot arise on one: every path they read — `.spine/**`, the CI definition, the constitution, agent context — is a path PB §5.5 already refuses the reseal over when its blob at `O` differs from its blob at `base=`, *"so a reseal never seals a policy change"*. The row is exactly G1 and G8 because those are the two gates PB §5.5 names.

**Outright is a coverage rule, never a containment rule, and the two must not be conflated.** §6.2's containment condition — the report's wire set ⊆ the union of the discharging reviews' `wires=` — is about what the human read, and it includes every entry of the array, outright findings among them. So a landing that carries an outright wire and reaches a review state at all needs that wire **named** in the review's `wires=` to be consumable, and naming it still does not make the gate `override`. Reading the two rules as one produces the exact failure PB §12 records being closed in v0.14: a review naming `G14:<path>` would discharge `paths-shrank`, which PB §6.3 forbids in the words `docs/spec/manifest.md` §5.9 quotes.

**An id-loss a `class=protected` `G8:<path>` review names is G8's finding, never G1's.** PB §6.3 G1 writes the exemption in those words — "save where a `class=protected` `G8:<path>` review names the path whose ids went away: a deletion the harness rules already routed to a human is G8's finding and never independently G1's" — and PB §4.3 makes an id collected on `B` and absent from `T`'s collection a G8 failure. The allocation is made when the outcomes are read, before any status is written: `docs/spec/result-file.md` §8.5 clause 2 assigns the *went-away* shape to G8 alone once such a review names the path, so the wire set carries one `G8:<path>` finding and **no G1 finding**, and the landing records **`G1=pass`, `G8=override`**. That is not an exception to the covering rule above — which still requires the covering wire to name the gate it excuses — but a rule about which gate the finding was ever attributed to. Read the other way, the covering wire would be `G8:<path>` against a G1 finding, no admitting review could exist, `G1` would read `fail`, and the one path PB §4.3 and PB §6.3 build for deleting a landed test could never land; PB §12 records that seam being closed once already, in v0.14.

**The `G8:<path>` review exemption reaches the went-away shape only.** An id that collected on `T` and did not pass is a finding of both gates (result-file §8.5 clause 2), and a `G8:<path>` review covers G8's alone: that landing reads `G8=override` with G1's finding still uncovered, and is not landable until a `class=break-glass` review bypasses G1 — no review of any other class admits a G1 finding, which is what makes every one of them outright (above).

**The `xfail`/`skipped` carve-out is the opposite shape, and it is not a review exemption at all.** PB §6.3's G1 and G8 rows both carve out an id **whose own collected outcome on `B` was already `xfail` or `skipped`** and which still collects on `T`. `docs/spec/result-file.md` §8.5 clause 2 applies it before the allocation, and it produces **no finding in either gate**: not an outright one, not a coverable one, and no `wires` entry. The landing records `G1=pass` and `G8=pass` on that id's account, and no review names anything, because §6.2's containment condition has nothing to contain. Like §9.20's rule this is an **attribution** rule and not a status rule — it decides which gate a finding was ever attributed to, and the answer here is neither — so the reading that would make it a covered `fail` is wrong for the same reason and with the same consequence: it would leave every repository carrying one long-standing `xfail`, or one ordinary skipped test, on trunk with an uncovered G1 finding on every landing, hence `report-not-landable`, hence no landing at all in a lane where break-glass is unreachable (`docs/spec/result-file.md` §8.7).

**Three boundaries on it, because each decides a different `wires` array.** It is decided on the `B` outcome alone and never on `T`'s, so an id trunk reports `xfail` or `skipped` that the branch turns into `failed`, `error`, `skipped`, `xfail`, `xpass` or `unknown` is still no finding. It does **not** reach an id that went away, which stays the ordinary allocation above with its `class=protected` `G8:<path>` review. And it does **not** reach a frozen `Spine-Test` entry, only a `B`-floor id — so a landing whose frozen id is not passed is a G1 finding and outright, whatever `B` said about it. An implementation that widens any of the three writes a smaller wire set than a conforming one, a different `report=` and a different `envelope=` over identical objects. **On a `Spine-Event: reseal` landing the three boundary cases read `override`, and the carve-out itself is unchanged.** The distinction matters because the two produce different `gates[]` entries. The boundaries name findings — a `B` outcome that was neither `xfail` nor `skipped`, an id that went away, a frozen `Spine-Test` entry that did not pass — and on a reseal the row of the table above admits each of them under the reseal's own `class=protected` review, so a landing that hits one reads `G1=override` or `G8=override`. **The carve-out is unconditional on every landing shape, reseal included**: where `b.out` is `"xfail"` or `"skipped"` and the shape is *did not pass*, there is no finding for any review to admit, `wires` carries no entry, and the landing reads `G1=pass` and `G8=pass` on that id's account (`docs/spec/result-file.md` §8.5 clause 2, which applies it before the allocation and without conditioning on the landing shape).

**"The same id" means the same `(runner, id)` pair.** A repository may run several runners and a test id is qualified by the one that collected it (PB §4.3, §5.9). A landed id therefore goes away not only when its test is deleted but when the runner that collected it stops running — dropped from `params.langs`, removed from `.spine/ci.sh`, or reconfigured to deselect it — and that is the same G8 clause with the same remedy: a `class=protected` `G8:<path>` review naming the path, or the landing does not land. This document fixes only how the resulting status is recorded; which pair a source symbol maps to is the join `import-resolver.md` §12 supplies (§5.4.2, §11).

**A break-glass bypass reads `=override` whether or not the gate produced a finding.** PB §7.6: "The bypassed gates are likewise marked `=override`." PB §6's transition row: "G1, G2, G3, G4, G6, G7, G8, G12 bypassed **and marked `=override`**." G9 validates the entry the same way in both limbs — "each `override` named in the `wires=` of a copied review whose class admits it (tripwire/protected: that wire; break-glass: the PB §7.6 list)". A break-glass review may name a gate that did not run for this landing; only gates that ran have entries (§5.6.2), and G9's check reads override → named, never named → override.

**A report containing any `fail` is a non-landing report.** It is the report a reviewer reads and binds with `report=` on their `Spine-Review`; it is never the report a seal names. A run that would seal a report containing a `fail` refuses: status `report-not-landable`. A landing whose `Spine-Gates` copies a `fail` is malformed and G9 indexes it `unattested`.

**`gates[].G9` is the pre-build ledger walk.** G9 runs twice — over trunk before the landing is built (PB §5.4 step 3; on a tombstone, where step 3 is skipped, the binding walk of step 2), and over `L` at step 5. The member records the first. The second cannot be a member for exactly the reason G10's result cannot: `L` exists by then, and its seal covers the message the report's digest is already inside (PB §5.4 step 5, PB §11). See §9.14.

**`Spine-Gates` is a rendering of this array**, in the same order, as `G<n>=<status>`, space-separated. G10's result is never in it (PB §11) and never in `gates` — G10 runs after `L` exists and its own result cannot be inside the message `L`'s seal covers (PB §5.4 step 5).

#### 5.6.2 Which gates run

PB §11 says `Spine-Gates` lists "every gate that ran". The general rule: **a gate runs iff every input its PB §6.3 check reads exists for this landing.** The playbook enumerates only the tombstone case; the rest is resolved here (§9.12).

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

The tombstone row is normative in the playbook: `Spine-Gates` lists only G9, G13, G14, G15 (PB §5.4 step 2, PB §11). G3, G4 and G12 read an in-flight intent, an approval, or both, and a subjectless landing has neither.

**A reseal runs the suite.** PB §7.4 rule 5 states it and names the earlier reading as the defect it corrects: "§5.5 seals a G1 or G8 finding inside a resealed range as `=override`, which requires G1 to have *run*, so a reseal does run the suite, does ingest a result file, and seals the real `profile=` that file reports — never `n/a`." PB §11's *Landings that run no suite* says the same and names only the tombstone. So a reseal's G1 evaluates every clause, its `evidence` is present, and its `profile` is whatever the ingested header reports; the labels that file carries are `docs/spec/result-file.md` §8.6's (`base=` the seal's `base=`, `tree=` `tree(O)`, since `Hc = O`). **That is the conforming reseal.** One whose file is absent or malformed is not exempt from anything and is not refused either: `result-missing` or `result-malformed` is a G1 finding, §5.6.1's reseal row admits it under the reseal's `class=protected` review, and that landing seals `profile=none` with `evidence` absent (§5.9, `docs/spec/result-file.md` §8.4) — the one reseal shape in which the two members read that way.

**G6 never appears in a version-1 report.** PB §6.3 as of v0.19 marks it *roadmap 5, not v1* and PB §9's roadmap ships it at step 5; nothing in the playbook says where a repository configures it, and "iff configured" over a configuration that does not exist is two implementations disagreeing about the length of `gates[]` — and therefore about `Spine-Gates`, and therefore about `envelope=`. G6 arrives with the mechanism that configures it, under a `report_version` bump. Break-glass may name G6 in its `wires=` (PB §7.6); in v1 there is no entry for it to mark.

### 5.7 `floor_hits`

Array of `esc`-encoded paths, deduplicated, sorted ascending by encoded bytes: every entry of the `merge_base..head` diff — renames, deletions, mode changes, symlinks (`120000`), submodule pointers (`160000`) included — that intersects the shipped floor, `C-A2`, or the manifest's `paths.*` (PB §6.3 G14, PB §7.3). Paths are recorded as the diff produced them; G14's casefolding is a comparison, not a rewriting.

**A casefold collision records the diff entry, not the file it collides with.** PB §7.3 makes "a diff entry whose casefolded path equals an existing path's" a floor hit — two spellings of one file are a collision, not a new file. The entry in `floor_hits` is that **diff entry's** path, as the diff produced it; the existing path it collided with is not in the diff and is never recorded.

**Invariant:** for each entry `p`, `wires` contains exactly one `{gate: "G14", path: p, class: "protected", kind: "finding"}`, and `wires` contains no other `G14` entry. `floor_hits` is the authoritative list; the `G14` wires are derived from it. Both are recorded because PB §7.4 rule 4 names floor hits as a report field and PB §11's protected-review row reads the floor hits out of the wire set as `G14:<path>`.

### 5.8 `automerge` — PB §7.4 rule 5, made a record

| Member | Type | R/A | Value |
|---|---|---|---|
| `requested` | boolean | R | `policy.rules.c_m4 == "on"` |
| `preconditions` | array | mixed | Five entries, `id` ascending |
| `effective` | boolean | mixed | `requested` **and** every precondition's `status` is `"met"` or `"exempt"` |

Each precondition entry: `{"id": <0..4>, "status": "met" | "unmet" | "exempt"}`.

| id | Precondition (PB §7.4 rule 5) | R/A |
|---|---|---|
| 0 | `C-A3: threat.candidate == "trusted"` | R |
| 1 | manifest `params.isolation` = `container`, the one boundary v1 ships a mechanism for, **and** the ingested header's `profile=` equals it (`docs/spec/result-file.md` §7.1, §8.4) | R (both sides are recorded: the manifest at `base`, and `profile`, which the seal carries) |
| 2 | `"met"` iff **all three**: the ingested header's `keys_visible=` is `false`; the collector's `tool=` is the base's pin; **and this run established that the ingested file came from a job whose definition was taken from trunk** (PB §7.4 rule 0, as amended 2026-08-26; `docs/spec/result-file.md` §8.1, §8.4). `"unmet"` otherwise | A |
| 3 | reconstruction proved before this push | R — structurally `"met"` in v1: there is no deferred mode (PB §6.3 G10) |
| 4 | this run performs the CAS itself and pushes the object that becomes trunk's tip | A |

**Precondition 3 records a proof that has not run yet.** G10 runs at PB §5.4 step 5, after the report is hashed into the envelope at step 4. `"met"` is therefore a statement about the pipeline's shape — there is no mode in which reconstruction is deferred — not an observation of this run. It is harmless because a G10 failure discards the landing and the report with it, so no report bearing a false `"met"` ever reaches trunk; it is stated because an implementer who notices the ordering will otherwise "fix" it by moving the hash after step 5, which breaks the envelope.

**Precondition 2 fails on a `true` assertion; it does not refuse the run.** `keys_visible=true` is a legal header value and produces a legal report: `docs/spec/result-file.md` §8.4 makes precondition 2 hold only if `keys_visible=false`, and its failure does not refuse ingestion — "the design's answer to weak isolation is always that a human reads the landing, never that nothing happens." PB §7.4 rule 5's "it *carries* rule 0's key-visibility assertion" is not a test of presence: a header carrying `true` carries the assertion and still fails the precondition. So `automerge.preconditions[2].status` is `"unmet"` there, `evidence.keys_visible` is `true`, and the rule-5 wire below is raised.

**Precondition 2's third conjunct fails the same way, and it is the only place the narrowed provider rule is felt** (owner, 2026-08-26; §9.25). PB §7.4 rule 0 requires the untrusted job to run from trunk's own definition and, as amended, says a result file from a job that *cannot demonstrate* that origin is **ingested** — it fails this precondition and nothing else. So a landing on `--ci gitlab` with in-repository configuration, or on `--ci generic`, produces an entirely ordinary report: `evidence` present with every member read from the ingested header, `gates[]` carrying whatever the gates found, `preconditions[2].status: "unmet"`, and the rule-5 `G11` wire. What counts as such evidence is `docs/spec/ci.md`'s (§10.3 scores each arrangement; §14 R11 fixes GitHub's test) and this document does not restate it — a provider test written here would be a second, unsealed copy of that one.

**The status domain admits the value, and is not widened.** `"unmet"` is the honest answer for all three conjuncts: `"exempt"` means *the design granted an exemption* and rule 5 grants none here, and `"met"` is plainly false. A fourth value — `"unverifiable"`, or a split of `"unmet"` by cause — was considered and **rejected**: it would put a new token inside a digest-bearing member, force a `report_version` bump for a distinction no gate reads, and break §3.2's reader rule for every existing reader of a version-1 report. It would also be the wrong shape, because the three conjuncts are not exclusive — a `--ci generic` run on a laptop-built collector fails all three at once — and a status that names a cause has to choose one. **The cause lives in the review's `reason=`**, mandatory on the `G11` wire (PB §5.2, PB §7.4 rule 5), which is prose inside the review's `line` and is not re-encoded as a member (§9.13). One precondition, one three-part test, one `"unmet"`, one wire.

`"exempt"` is used only where the design grants exemption, and the grant is PB §7.4 rule 5's own, singular: a **tombstone** is exempt from the rule entirely — all five `"exempt"`, `profile: "n/a"`. **A reseal is exempt from nothing.** PB §7.4 rule 5 corrects the earlier reading by name — a reseal "does run the suite, does ingest a result file, and seals the real `profile=` that file reports — never `n/a` … preconditions 1 and 2 are evaluated for it like any other landing that tests something" — so a reseal records all five as it computes them; under the shipped `C-A3: hostile`, precondition 0 is `"unmet"`.

**A tombstone under `C-M4: on` therefore records `effective: true`.** All five are exempt, so the conjunction reduces to `requested`. That reads like a bug and is not one: a tombstone changes no tree, runs no suite and produces no wire of its own, and PB §7.4 rule 5 exempts it by name — there is nothing for a human to read.

**The wires this produces** are PB §6.3's business, and as of PB v0.18 they are **one gate with two reasons**:

- `requested == false` — the constitution says `off` — raises a bare **`G11`** wire (PB §5.2).
- any precondition `"unmet"` raises a bare **`G11`** wire naming the failed precondition in the review's mandatory `reason=` (PB §5.2, PB §7.4 rule 5). PB §5.2 states both in one clause: "`G11` (`C-M4`) where the constitution says off, and `G11` naming the precondition where the run computed it off — one gate, two reasons, distinguished by `reason=`."
- **Both conditions can hold at once, and the wire set carries one entry.** Under the shipped defaults (`C-A3: hostile`, `C-M4: off`) both do. The two are raised independently — PB §5.2's "either missing" — but they are the same key `(G11, pathless)`, so §6.1's uniqueness rule collapses them into a single `{gate: "G11", class: "tripwire", kind: "advisory"}` entry and a single `G11` token in a review's `wires=`. What distinguishes them is the review's `reason=`, which is not a member of this report. An implementation that emits two `G11` entries produces a wire array — and therefore a `wires=` line and an `envelope=` — that no conforming implementation reproduces.

**The rule-5 `G11` wire attaches to every landing rule 5 applies to**, which PB §7.4 rule 5 makes every landing but a tombstone. PB §11 pins both ends: precondition 0 "fails on every run that tests anything, so the `G11` precondition wire is present in every such set, in every lane", while a tombstone "runs no gates that can produce a rule-5 `G11` wire" and is exempt from the rule entirely. A reseal raises it like any other landing that tests something; the protected review a reseal also takes is PB §5.5's, and rule 5's tripwire never substitutes for it.

Either reason produces a `class=tripwire`, `kind: "advisory"` wire, and neither is a finding about G11 — G11's own check is base currency, whose failure ends the run rather than raising a wire (PB §6.3 G11).

**Never `G1`.** PB v0.18 moved this wire off `G1` precisely because a bare `G1` advisory and a genuine `G1` finding were byte-identical tokens: "a reviewer signing `wires=G1` on a green landing and a reviewer signing `wires=G1` over a failing test would sign byte-identical wire sets meaning opposite things, and G9's ledger audit could not tell them apart" (PB §12). PB §11's wire aggregation is normative: "It is never spelled `G1`: a `G1` wire is a finding that named tests did not pass, and the two must never share a token a reviewer signs over." **A `G1` wire in a version-1 report is always a finding** (§6.1).

### 5.9 `evidence` — the collector's attested facts

Present iff a result file was ingested. Every member is attested.

**"Ingested" is the test, and the narrowed provider rule does not change it.** A well-formed file whose trunk-defined origin the run could not establish **is ingested** (`docs/spec/result-file.md` §8.1), so `evidence` is **present** and every member below is read from its header exactly as from any other. The doubt about who wrote that header is recorded once, in `automerge.preconditions[2].status` (§5.8), and nowhere else: no member of `evidence` is suppressed, blanked, downgraded or annotated on account of it. This is deliberate and it is the reverse of §5.9's *no file* case below — there the members have no source and must be absent; here they have a source whose authority is in question, and a digest that records what was actually read is more useful to a post-mortem than one that records nothing. `result_sha256` in particular pins the exact bytes a forger would have had to write, which is the only thing that makes a later investigation possible at all.

| Member | Type | Value |
|---|---|---|
| `result_sha256` | string | `"sha256:<hex>"` over the result file's exact bytes as the collector wrote them. The file's format and its outcome vocabulary are `docs/spec/result-file.md`'s; this spec names it only by digest. |
| `collector.version` | string | The collector's `tool=` version from the header. |
| `collector.dist_hash` | string | `"sha256:<hex>"` — the collector's artifact-list digest from the header. |
| `keys_visible` | boolean | The header's `keys_visible=` assertion (PB §7.4 rule 0). **`true` is representable**: it does not refuse ingestion and does not fail the run — it fails auto-merge precondition 2, and `automerge.preconditions[2].status` is `"unmet"` (§5.8, `docs/spec/result-file.md` §8.4). |
| `ids` | integer | The header's `ids=` — the size of the id set collected on `B`. **An id is a `(runner, id)` pair.** PB §4.3: a repository may run several runners (`params.langs`), a `Spine-Test` id is qualified by the runner that collected it, and the pair is the identity. So one runner-native string collected by two runners is two members of the set and counts twice, and a set of the same cardinality under a different runner configuration is a different set. The count is a fact about the collection, not about the repository's tests. |

The header's `tree=` and `base=` are not recorded: the trusted stage ingests a file only when both equal the `T` and `B` this run fixed, and refuses otherwise as `base-moved` (PB §7.4 rule 3, `docs/spec/result-file.md` §8.3 step 1), so recording them would be a second spelling of `objects.tree` and `objects.base`.

**When no file was ingested.** `result-missing` and `result-malformed` are G1 findings, not run-ending states: "the run proceeds to a gate report and a review state like any other failing gate" (`docs/spec/result-file.md` §8.2), and both are in the break-glass bypass list (§8.7 there, PB §7.6), so a landing can seal over them — and a seal carries `report=` and `profile=`.

**The two findings, as that spec defines them and this one uses them.** `result-missing` is *no file at the path §8.1 fixes*; `result-malformed` is *a file was found and §4's grammar or §8.3 step 3's runner-token check rejected it*. Together they are exhaustive over the ways ingestion fails, and **neither is ever raised for want of trunk-defined origin evidence** — that file is present and well-formed, so it is ingested and its consequence is precondition 2's (`docs/spec/result-file.md` §8.1, §8.2; §9.25 below). Two neighbouring outcomes are neither finding and must not be recorded as one: a `tree=`/`base=` mismatch is `base-moved`, which ends the run before a report exists (`docs/spec/result-file.md` §8.3 step 1); a `tool=` mismatch is a **G15** failure, never a G1 finding and never overridable (PB §11).

That report's shape is fixed:

- `evidence` is **absent**. Every member of it is read from a header that does not exist, and §7 rule 6 makes an inapplicable member absent rather than null or empty.
- `profile` is **`"none"`**. `docs/spec/result-file.md` §8.4 fixes it: "a seal must never claim a boundary no header established, and PB §11's seal grammar admits no fifth value." `"n/a"` is not available — PB §11 reserves it for a landing that runs no suite, which is the tombstone alone — so `profile: "none"` with `evidence` absent is the signature of this landing, and `profile: "none"` with `evidence` present is a collector that reported an unisolated run.
- `automerge.preconditions[1].status` and `[2].status` are **`"unmet"`**. Precondition 1 needs the ingested header's `profile=` to equal the manifest's `params.isolation`, and precondition 2 needs `keys_visible=false` from a pinned collector; with no header, neither holds and no exemption is granted. The rule-5 `G11` wire is therefore in the set, as it is on any landing rule 5 applies to (§5.8).
- `gates[]` carries `G1` with `"fail"`, or `"override"` where a `class=break-glass` review names it. The break-glass review's `wires=` carries a **bare `G1`** — `result-missing` and `result-malformed` are findings about G1 as a whole and name no path (`docs/spec/result-file.md` §8.7) — and for `class=break-glass`, `wires=` lists bypassed gate ids rather than wire tokens (§6.2).

**When a file was ingested but its origin was not established.** This is the case the amendment of 2026-08-26 created, and it is emphatically **not** the case above. The shape is fixed and is ordinary in every member but one:

- `evidence` is **present**, with all five members read from the ingested header (§5.9 preamble).
- `profile` is **the header's own value** (§5, member table), not `"none"` and not `"n/a"`.
- `automerge.preconditions[2].status` is **`"unmet"`**, by the third conjunct (§5.8). `[1].status` is computed normally and may well be `"met"` — a header reporting `container` against a manifest declaring `container` satisfies precondition 1 whoever wrote the header, which is exactly why the design never let precondition 1 stand alone (PB §7.4 rule 5).
- `automerge.effective` is **`false`**, since a precondition is unmet.
- `gates[]` carries whatever the gates found. **`G1` is `"pass"` on a green suite**: the origin question is about auto-merge, not about whether the tests passed, and no gate reads it.
- The rule-5 `G11` wire is in the set, `class=tripwire`, `kind: "advisory"`, its `reason=` naming precondition 2.

So the landing is representable at both ends — a report with a digest, and a seal carrying `report=` and a legal `profile=` — which is the property the narrowing had to preserve and the strict reading destroyed: under *never ingestible* there was no file, hence `result-missing`, hence a G1 finding with no override in the quick lane (`docs/spec/result-file.md` §8.7), hence no landing to represent (§9.25).

**When the collector's deadline expired.** `params.timeout` bounds one runner invocation (default 1800 seconds, read from trunk, §5.4); on expiry the collector kills the process group and writes `status=runner-timeout` with the `base` records and whatever `result` records it parsed before the kill (`docs/spec/result-file.md` §7.3). That file **is** ingested and `evidence` **is** present — it is an honest file with a complete header — but `status ≠ complete` means no id counts as passed whatever any record says, so every frozen id is a G1 finding and `gates[]`'s `G1` reads `fail`, or `override` under a review that admits it. The report records none of this directly: no deadline, no elapsed time, no record count (§7 rule 1). `result_sha256` pins the file that says it, and the G1 status says what it cost. A collector that enforces no deadline is non-conformant, so "the run simply never ended" is not a state a conforming pipeline can present to this schema.

The result file's per-id outcomes are not recorded — neither the `T` outcomes nor the `B` outcomes its `base` records now carry (`docs/spec/result-file.md` §4.4) — and neither are the ids themselves, nor the runners that qualify them. The report is decision-bearing, not diagnostic: `result_sha256` pins every byte the collector wrote, and diagnostics belong to `spine review` (PB §6.5), which is out of scope (§11).

### 5.10 `run`

| Member | Type | Value |
|---|---|---|
| `reverifications` | integer | How many re-verifications this run has performed, ≥ 0, ≤ `policy.rules.c_m3` (PB §7.4 rule 3). |

This counter lives in the run's memory and dies with it. Nothing in this design may remember that a previous run happened (PB §5.4, PB §12), and this member records only *this* run's count. A fresh run after a lost CAS starts at 0.

---

## 6. Wires

### 6.1 The `wires` array

Each entry:

| Member | Type | Presence | Value |
|---|---|---|---|
| `gate` | string | always | `"G1"` … `"G16"` |
| `path` | string | present iff the wire names a path or, for `G13`, a commit | `esc`-encoded path |
| `class` | string | always | `"tripwire"` \| `"protected"`. **Which value each gate raises is §6.3**, and it is not free: it decides the landing's review state through PB §11's aggregation and is inside `report=` and `envelope=`. |
| `kind` | string | always | `"finding"` \| `"advisory"` \| `"warn"` |

**`G13` names a commit, not a path.** PB §6.2 raises an event commit whose signature fails, or whose role disagrees with its namespace, as "a G13 wire naming the sha" — excluded from state derivation so a bogus commit cannot brick an append-only branch. For `G13`, `path` carries that commit's oid, lowercase hex at the length `object_format` implies, for which `esc` is the identity; the wire token is `G13:<oid>` (§6.2). It rides in `path` because the wire set has one shape and a review's `wires=` must be able to name it; nothing else in v1 puts a non-path there. **Which G13 finding raises it is `docs/spec/manifest.md` §4.8.4 check 2** — an event commit whose signed line claims none of the five roles a landing rests on (`Spine-Signoff`, `Spine-Approve`, `Spine-Review`, `Spine-Upgrade`, `Spine-Withdraw`) — and it is the only G13 finding that is not outright (§5.6.1).

`kind` distinguishes three things PB §6.3 and PB §11 keep separate but never name together:

- **`finding`** — the gate's own check was not satisfied. Drives `gates[].status` toward `override` or `fail`; routes to a review state.
- **`advisory`** — the gate raised a wire that is not a finding about itself. **The only advisory wire in v1 is `G11`**, carrying rule 5's two reasons — a failed precondition, and a constitution that says `C-M4: off` — which collapse into one entry (§5.8). PB §11 names this case exactly: "a gate that ran and passed its own check reads `=pass` even when it raised a wire". Routes to a review state; does not affect gate status.

**One consequence, stated because two gates' tokens once collided.** In a version-1 report a **`G1` wire is always a `finding`** — PB §11: "a `G1` wire is a finding that named tests did not pass" — and a **`G11` wire is always an `advisory`**, G11's own check being base currency, whose failure ends the run before a report is sealed (PB §6.3 G11). PB §11's `Spine-Gates` row once carried a pre-v0.18 parenthetical spelling that wire `G1`; **it now reads "the rule-5 `G11` precondition wire is not a finding about G11"**, which agrees with PB §11's own wire-aggregation paragraph, with PB §7.4 rule 5 and with PB §12. Nothing in §11 disagrees with anything else in §11 any more, and this document's reading (§9.18) is now the only one available rather than the chosen one.
- **`warn`** — a Drift finding under warn-before-block calibration (PB §6.3, PB §9). Enters the wire set and any review's `wires=` (PB §6.3 says so explicitly), does **not** route, does **not** affect gate status. Only G2, G3 and G7's *soft* clause can produce it; a `forbidden` hit and a hard lease over another intent's forbidden or frozen set are `finding` in every mode (PB §11).

**Uniqueness.** `(gate, path)` — with the pathless case treated as a distinct key — appears at most once. If one evaluation would produce the same key twice, the entries collapse and the surviving `class` is `"protected"` if either was, per PB §11's "`protected` dominates `tripwire`"; the surviving `kind` is the strongest of `finding` > `advisory` > `warn`.

**The collapse can never merge an advisory into a finding.** The `kind` precedence exists for repeated findings of one gate over one path, not to reconcile two different claims: in v1 the only advisory-bearing gate is `G11`, and `G11` raises no findings, so every collapse is between two entries of the same kind. This is what PB v0.18 bought by moving the rule-5 wire off `G1` (PB §12): while both were spelled `G1` and both pathless, a failing frozen test and a green landing under the shipped defaults produced the same key, the collapse promoted the advisory's kind to `finding`, and a signature over `wires=G1` meant either "a human accepted that auto-merge is unavailable" or "a human accepted a failing test" with nothing in the record to say which. A later version that gives some gate both a finding and an advisory must say how they are told apart before it may rely on this rule; it is a `report_version` question, not an implementation's.

**Ordering.** Ascending by unsigned byte value over the whole **wire token** of §6.2 — PB §11's order, adopted from §5.6 and not restated here. So `G11` precedes `G2` (`0x31` < `0x32` at the second byte), `G1` precedes `G11`, and within one gate the pathless entry precedes every `:`-suffixed one because its token is a proper prefix of theirs. **The sort key is the token's bytes**, which for a path-bearing entry means `tok(path)` and not `esc(path)`: the two differ on `,`, ` ` and `"` (§6.2), and sorting the array on one key while the line is written under the other produces a `wires=` whose order does not match the array's over the same findings.

**Aggregation** is PB §11's and is not restated here: `protected` anywhere in the set makes the landing `protected-review`; a landing has exactly one review state; the signerless overlay is evaluated after aggregation and sets the cardinality and class of the reviews. The report records the set; the state is derived from it by the transition table of PB §6.

**One shape of this array is now counted, and the counter is not a member.** The owner settled on 2026-08-26 that an unbounded `forbidden` set stays legal — both polarities take the same patterns and a human signed the declaration — and that `spine stats` gains a counter for **landings whose only `class: "protected"` entry is a `G7` hard lease** (PB §5.4). That predicate is a function of this array and nothing else: it holds iff some entry has `gate == "G7"` and `class == "protected"`, and no other entry has `class == "protected"`. It is deliberately **derived at read time and never stored** — `spine stats` reads the graph, not reports (§11), and adding a boolean would be a second spelling of a fact the wire set already fixes, in a digest-bearing member, for a counter no gate reads. What the counter buys is that one intent quietly taxing every other landing becomes visible rather than mysterious.

### 6.2 The wire token, and a review's `wires=`

A `Spine-Review` carries `wires=<G<n>[:path],…>`, comma-separated and sorted (PB §11). The **wire token** of an entry is:

- `G<n>` when `path` is absent;
- `G<n>` + `:` + `tok(path)` otherwise,

where `tok(s)` is `esc(s)` with three bytes moved out of the printable row of §2.3 into the `\xHH` row: `,` (`0x2C`) → `\x2c`, ` ` (`0x20`) → `\x20`, `"` (`0x22`) → `\x22`. Every other byte follows §2.3 unchanged, so `tok` is `esc` for every path containing none of the three. `tok` is **one pass over the bytes of `s`**, not `esc` composed with a second escaping step: a second pass would re-escape the `\` that the first pass emitted and turn `,` into `\\x2c`.

`=` is deliberately **not** escaped: a trailer field splits on its first `=`, so `wires=G2:src/a=b.ts` parses as the field `wires` with the value `G2:src/a=b.ts`. Three escapes, not four — the same reasoning that justifies the three forbids a fourth.

Those three escapes exist because the trailer is a space-delimited line of `key=value` fields whose `wires=` value is comma-separated, and a repository may contain a path with a comma, a space or a quote in it. Without them a single such path would make a signed review unparseable, and the containment check PB §11 requires would be undecidable. **`docs/spec/envelope-vectors.md` must adopt this token function verbatim**; it is defined here because the wire set is computed here.

**PB §11's "wires sorted" is the `wires` array order of §6.1**, which is PB §11's own: ascending by unsigned byte value over the whole token, so `G11` precedes `G2`. One key — the token's bytes — governs both the array and the line, so the line is the array's tokens joined by `,` and nothing has to be re-sorted to write it. A **numeric** sort — `G2:src/shared/util.ts,G11` — is **non-conforming**: the sorted line is signed, so a numeric implementation produces byte-different `Spine-Review` lines and its containment check fails against a conforming implementation's report over identical facts.

PB §5.5's canonical envelope reads `wires=G11,G2:src/shared/util.ts`, which is exactly this order, and §8's example below uses it. `docs/spec/envelope-vectors.md` vectors A and D carry the same line.

**Containment.** The transition table's condition is *the report's wire set ⊆ the union of the `wires=` of the reviews that discharge the landing's review state* (PB §6, PB §11). PB §6's protected row says "the union of their `wires=`" for exactly this reason: a signerless landing carries two `class=protected` reviews from distinct keys in team mode (PB §11's signerless overlay), and neither need individually cover the set. A `landing-review` discharged by one review is the same rule with one term. The comparison is set containment over wire tokens, byte-for-byte. It includes `warn` and `advisory` wires — every entry of the array. A review's `wires=` may name tokens absent from the report: a review signed against a larger earlier set is retained when the set shrinks, subject to PB §5.4's retention rules.

**Break-glass is the one exception.** For `class=break-glass`, `wires=` lists *the gates bypassed* as bare ids (PB §11), not the wire set, and is never used for containment — it is read by §5.6.1 and by G9 instead. A break-glass review sits alongside the review that discharges the state; it never replaces it (PB §11).

### 6.3 The wire each gate raises

`class` is required, two-valued and **digest-bearing**, and it decides the landing's review state through PB §11's aggregation — which decides who must sign. A gate whose class is unassigned is a gate two implementations route differently, producing a different `wires` array, a different `report=` and a different `envelope=` over identical facts. PB §6.3 assigns a class for G5, both of G7's clauses, G8, G11, G14 and G16; the remaining gates are fixed **here**, and the assignments are not free choices:

- **Authority** (G13, G14, G15, G16) never warns, is never in PB §7.6's bypass list, and judges the machinery that judges the landing — the case PB §7.6 says "need[s] a second human". Its wires are `protected`. `docs/spec/manifest.md` §6.1 makes the same argument for G16 and asks this section to carry it.
- **Drift** (G2, G7-soft), **Freshness** (G3, G4, G11) and **Strength** (G12) route to `landing-review` under PB §6's transition table — "wire tripped on `T` (Drift / Freshness / Strength…) **and no protected-class wire present** → landing-review". Their wires are `tripwire`. G7's *hard* clause is the one Drift wire PB §6.3 makes `protected`, and PB §6's floor row routes it accordingly.
- **Integrity** (G1, G5, G8, G9) splits by clause and is stated per row.

A gate that raises no wire in v1 is listed with an em dash rather than omitted, because "no row" and "no wire" are two different things to an implementer reading a table for the value to write.

| Gate | Wire token | `class` | `kind` | Raised when, and what fixes it |
|---|---|---|---|---|
| G1 Coverage | `G1:` + `tok(path)` for a **per-id** finding — the pair's `result` record `path` where the file carries one, its `base` record `path` where it does not (`docs/spec/result-file.md` §8.5), mirroring `G8:<path>` over the same bytes. **Bare `G1`** for the five findings that name no path: `result-missing`, `result-malformed`, an `end.status` fold that is not `complete`, an AC with no collected `verified_by` edge, and a frozen `Spine-Test` entry that collected nothing — and for any per-id finding whose `path` is the empty string, an empty path being no path (§8.5, §8.7 there). One entry per path under §6.1's `(gate, path)` key | `protected` | `finding`, always (§6.1) | Every G1 finding is **outright** (§5.6.1) **on every landing shape but a reseal**: PB §6's `tests-approved` row gives it no route into a review state, and the only discharge is a `class=break-glass` review naming `G1` — save on a `Spine-Event: reseal` landing, where §5.6.1's reseal row has the reseal's own `class=protected` review naming the token admit it instead (PB §5.5), break-glass being unavailable there. `protected` is the class because break-glass "never relaxes who must sign" (PB §11) and the companion review that discharges the state must carry team mode's reviewer separation — a landing overriding the frozen-test floor is exactly the emergency PB §7.6 says needs a second human. **One case is not a finding and therefore raises no wire here at all**: a `B`-floor id whose own collected outcome on `B` was already `xfail` or `skipped` and which still collects on `T` is carved out of G1 *and* G8 by PB §6.3's two rows and `docs/spec/result-file.md` §8.5 clause 2, so it contributes no `wires` entry and needs no review of any class — a set carrying a `G1` on its account is a set no conforming implementation reproduces |
| G2 Containment | `G2:` + `tok(path)`, one entry per diff path outside the declared `expected` set, per `forbidden` hit, and per changed package manifest (the new-dependency sub-check, whose per-language paths `docs/spec/import-resolver.md` lists). **Bare `G2`** for the diff-size sub-check | `tripwire` | `warn` under warn-before-block calibration, `finding` otherwise; a `forbidden` hit is `finding` in **every** mode (PB §11, §6.1) | PB §6.3 G2. The diff-size sub-check is `git diff --numstat --no-renames` over `merge-base..Hc`, additions plus deletions summed, binaries refused, floor and spine-owned paths exempt — **a repository-wide count that names no path**, so it takes the bare id under PB §11's "gates without a path use the bare id", and PB §6.3's `G2:<path>` governs the sub-check that implicates one. That is also the right retention behaviour: a base move changes `merge-base` and therefore the count, and a pathless wire never survives a base move (PB §6). The **schema/auth/public-API** wire is withdrawn (PB §6.3) and raises nothing |
| G3 Staleness | bare `G3` | `tripwire` | `warn` under calibration, `finding` otherwise | PB §6.3 G3. Staleness is a fact about the in-flight intent's committer dates, not about a path, so there is nothing to put after the colon |
| G4 Currency | bare `G4` | `tripwire` | `finding` | PB §6.3 G4 states both: "trips a wire: `landing-review` with `G4` — proceed by tripwire review, or a human reopens" |
| G5 Orphans | `G5:` + `tok(path)`, the path of the blob the offending pragma sits in | `tripwire` | `finding` | PB §6.3 G5, as of v0.19: "One wire per offending pragma, token `G5:<path>`, `class=tripwire`". **Two offending pragmas in one blob collapse to one entry** under §6.1's `(gate, path)` uniqueness rule — the wire set is per path, the diagnostic is per pragma, and the diagnostic is not in the report (§11) |
| G6 Mutation | — | — | — | Roadmap 5, not v1 (PB §6.3). No `gates` entry and no `wires` entry in any version-1 report (§5.6.2). Break-glass may name `G6` in its `wires=`; in v1 there is no entry for it to mark |
| G7 Interference — **soft** clause | `G7:` + `tok(path)` | `tripwire` | `warn` under calibration, `finding` otherwise | PB §6.3 G7: `expected ∩ expected`, surfaced to both owners. "The soft clause's wire is `G7:<path>` with `class=tripwire`" |
| G7 Interference — **hard** clause | `G7:` + `tok(path)` | `protected` | `finding` in **every** mode (PB §11) | PB §6.3 G7: the integrated diff ∩ another intent's `forbidden` or frozen set, over a fresh fetch at PB §5.4 step 3 — "the class is what separates them". The **ground-moved** clause is anchored on the **binding approval's `base=`** (the only anchor that exists): its `∩ forbidden` half is a hard-clause wire and takes this row; its `∩ touchpoints` half is a `spine check` diagnostic and is **not a landing wire at all** |
| G8 Freeze | `G8:` + `tok(path)`, always | **per clause**: `tripwire` for the harness-moved clause; `protected` for the branch-edited-before-approval clause, the landed-id clause, and `C-T3` | `finding`, never `warn` (PB §6) | PB §6.3 G8 and PB §4.3 assign the first two directly — harness moved → "rerun + landing-review" (PB §6's row: "landing-review, wire `G8:<path>`"), branch-edited → "is a `class=protected` wire `G8:<path>`", landed id gone or failing → "unless that review names its path", which PB §6.3 G1 and §5.6.1 both spell `class=protected` — **less the `xfail`/`skipped` carve-out**, which PB §6.3's G8 row states in the same words as its G1 row: an id trunk itself reported `xfail` or `skipped` on `B` that still collects on `T` is no G8 finding either and raises no `G8:<path>` entry (`docs/spec/result-file.md` §8.5 clause 2), while a *vanished* `xfail` or `skipped` id is a harness change and does raise one. **`C-T3` is assigned `protected` here**: it is the tree grep PB §7.4 rests the isolation argument on, and a boundary the branch moved is not a finding its own author may sign away. The clause where `T`'s blob differs from **both** the approval tree and trunk is **outright** (§5.6.1) — except on a `Spine-Event: reseal` landing, which has no approval tree, reads the clause as *differs from its blob at the seal's `base=`*, and admits the finding under its own `class=protected` review (§5.6.1's reseal row, PB §5.5) |
| G9 Ledger | — | — | — | G9 raises no wire. Its failures are refusals and index states, not review material: a trunk tip that is not a valid landing blocks `--land` until resealed, and a failing landing indexes `unattested` (PB §6.3 G9). Never bypassable (PB §7.6) |
| G10 Reconstruction | — | — | — | Never in `gates` and never in `wires` (§5.6.2): it runs after the seal, and its failure refuses the push |
| G11 Base currency | bare `G11` | `tripwire` | `advisory`, always (§6.1) | PB §7.4 rule 5 and PB §5.2. Rule 5's two reasons — a failed precondition and a constitution reading `C-M4: off` — collapse into **one** entry distinguished by the review's `reason=` (§5.8). PB §5.5 fixes the class: "`class=tripwire`, not protected — no floor path is touched" |
| G12 Red at approval | bare `G12` | `tripwire` | `finding` | PB §6.3 G12: "`class=tripwire`, token `G12`, raised by `--approve` and **never** by `--land`". A landing's only G12 check is that the copied approve line's `red=` is present and well-formed, and a malformed one is an envelope G9's parse refuses before any report seals — so **no version-1 landing report carries a `G12` entry in `wires`**, and `gates[].G12` reads `pass`. Whether G12 gains a landing-time clause is PB §6's to settle; this row records the v1 behaviour and nothing more |
| G13 Signers | `G13:` + the commit oid, lowercase hex at the length `object_format` implies, for which `esc` — and `tok` — is the identity (§6.1). **One finding of G13's carries it**, and G13 raises no other wire | `protected` | `finding` | PB §6.2 raises an event commit whose signature fails, or whose role disagrees with its namespace, as "a G13 wire naming the sha". Authority, so `protected`: a signature that did not verify is the one finding an author may never sign off on their own landing. **`docs/spec/manifest.md` §4.8 is the gate's algorithm** — thirteen ordered checks, each with its kind and a closed status token, and the verdict. Of those, **only check 2 over a commit whose signed line claims none of the five roles a landing rests on — `Spine-Signoff`, `Spine-Approve`, `Spine-Review`, `Spine-Upgrade`, `Spine-Withdraw` — is *coverable*** and raises this token; every other check is **outright** (§5.6.1), because PB §6.3 spells each as a refusal and no review class admits it. The coverable case exists so that PB §6.2's "a bogus commit cannot brick it" is true of an append-only branch: a protected review naming `G13:<oid>` discharges it, and the commit stays excluded from state derivation either way |
| G14 Floor | `G14:` + `tok(path)`, one per `floor_hits` entry and no other `G14` entry (§5.7) | `protected` | `finding` | PB §6.3 G14, PB §7.3, `docs/spec/constitution.md` §14.15, `docs/spec/manifest.md` §5.6. `paths-shrank`, `c-a2-shrank` and `c-a2-bracket-case` are **outright** (§5.6.1) and G14 is not on PB §7.6's list, so those are terminal |
| G15 Tool | — | — | — | G15 raises no wire. An unlisted `dist_hash` refuses locally and fails in `--ci`; it is a membership test whose failure ends the run (PB §6.3 G15, `docs/spec/result-file.md` §8.7). Never bypassable, in any mode, by anyone |
| G16 Scaffold | `G16:` + `tok(path)` where a path is implicated, **bare `G16`** where none is | `protected` | `finding` | `docs/spec/manifest.md` §6.1–§6.2, which asks this section to carry the class (§11 C1 there). Its outright checks are listed in §5.6.1; the coverable ones are dischargeable by a protected review naming the token |

**What this table does not decide.** It fixes the `class` and, where the corpus fixes one, the token; it decides no gate's semantics (§11, *Gate semantics*). **Every row is closed.** The last residual — a per-id `G1` finding with no token — was fixed at its owner rather than invented here: `docs/spec/result-file.md` §8.5 assigns `G1:` + `tok(path)` over the record that spec already fixed, leaves the bare form to the five findings that name no path, and this row adopts it whole. Two things turned on it. An implementation raising `G1:<tok(path)>` for a landed id that did not pass and one raising a bare `G1` wrote different `wires` arrays, hence a different `report=` and a different `envelope=` over identical objects — the §9.19 shape of defect, which no published byte count localises, since re-spelling a token changes the line. And on a `Spine-Event: reseal` landing, where break-glass is unreachable and the only discharge is the reseal's own `class=protected` review *naming the finding's token* (§5.6.1's reseal row), a finding with no token was a finding no review could admit — so an orphan range containing one failing landed test yielded a trunk nobody could land on, which is the deadlock that row exists to prevent.

**PB §6.3's rows adopt this table.** Where a playbook row states a class, this table repeats it and adds nothing; where it states none, this is the assignment, and PB §11 — which wins over prose there as here — fixes no competing one.

---

## 7. Determinism rules, collected

Everything below is normative and repeated here so an implementer can check against one list.

1. **No wall clock.** No member holds a time, a duration, a date or anything derived from one. G3's staleness comparison is made against the committer date of `objects.base`, not against "now" (§9.8); `params.timeout` is a duration and is therefore not a member, though the manifest blob `policy.manifest` names pins it (§5.4); and the committer date on the notes commit that publishes the report is outside the report, outside the digest and read by nothing (§4.4.3). PB §7.5: one clock, no timestamps.
2. **No environment.** No hostname, no runner id, no user, no path outside the repository, no locale, no process id.
3. **No state the design forbids.** No count of prior runs, no side file, no note read as a source, and no **persisted, fetched or restored** graph. PB §7.4 rule 3 is the rule: `spine index --fresh` is implied by `spine check --ci`, no SQLite file is fetched, cached or trusted from anywhere, and the trusted stage restores no cache at all. A gate result derived from that per-run rebuild is recomputable, because the rebuild is a deterministic function of the same git objects — which is what G10 proves on a clean clone before every landing (PB §6.3 G10). This is not a licence to read a stored graph, and it is not a prohibition on querying one: PB §6.3 makes every gate a query, and G1's AC-coverage and G5's orphan clauses are graph queries. They are recomputable for that reason and no other. **Publishing the report to `refs/notes/spine` is required (§4.4) and reading a note is still forbidden**, and the two are not in tension: publication is an output of a landing that has already happened, while reading would make a mutable, unauthenticated ref an input to one that has not. A gated run fetches no notes ref and consults none; `--verify` — which is not a gate — reads one only after its bytes hash to a value a signature already covers (§4.1, §4.4.6).
4. **Key ordering** is JCS's: ascending by member-name bytes. Never insertion order, never a hand-written order.
5. **Array ordering** is fixed per field by §5 and §6. Every array whose semantics is "the set of X" is emitted even when empty; `[]` is a value, not an absence.
6. **Absent versus null.** `null` never appears. An optional member is present or absent, and §5 states the presence condition for every one. Absence always means "this concept does not apply to this landing", never "unknown" and never "empty".
7. **Numbers** are integers in `[0, 2^53 − 1]`, plain decimal.
8. **Paths, patterns, principals, trailer lines** are `esc`-encoded (§2.3) and are never normalized, casefolded or separator-rewritten.
9. **Object ids** are lowercase hex at the full length `object_format` implies — 40 or 64 digits. Never abbreviated, never uppercase, never prefixed. The playbook's `9f2c…` is display, not a value.
10. **Non-git digests** are `"sha256:"` + 64 lowercase hex (PB §11 hash policy). Never bare hex, never uppercase, never another algorithm.
11. **No self-reference.** The report never contains its own digest, and never contains `envelope=` — the `Spine-Review` lines that carry `report=` are inside the envelope digest, and a report containing `envelope=` would be circular through them. (The rule was right and its stated reason was wrong until 2026-08-27: the earlier wording blamed the *seal* line, which sits **below** the seal boundary and is not inside `envelope=` at all. `docs/spec/envelope-vectors.md` §15 filed the correction and it is adopted here verbatim.)
12. **No size cap.** Only the digest enters the envelope, so PB §5.5's 16 KiB envelope cap does not apply to the report. It does apply to the review that must cover the report's wire set: `Spine-Review` lines are inside the cap, and a reseal folds every wire in an orphan range into one report and one review's `wires=` (PB §5.5). Reports are bounded in practice by `floor_hits` and `wires`, both bounded by the diff. **The reseal is exempt, and that exemption is now the playbook's.** PB §5.5 as of v0.19 does not apply the 16 KiB cap to a `Spine-Event: reseal` envelope: a reseal's review line grows with the orphan range it folds, nothing about it can be split, break-glass is unavailable to it, and switching strategy moves the capped quantity by one byte — so the cap has no exit there and would brick trunk. `docs/spec/envelope-vectors.md` §2.9 fixes the capped quantity and the exemption, and §12 never raises `envelope-too-large` for that shape. For every other landing shape the cap stands and its **only** exit is splitting the intent — never truncation, and no longer a change of merge strategy.

---

## 8. Worked example, with published test vectors

The landing is the canonical envelope of PB §5.5: `INT-042`, team mode, merge strategy, `C-A3: hostile`, `C-M4: on`, `profile=container`, one reopen, one `class=tripwire` review by `bob` over a `G2` containment finding, with the universal rule-5 `G11` advisory wire present because precondition 0 fails under `hostile`.

**What in this vector is fabricated, enumerated by PB §11's two hash classes.** The vector tests a canonicalizer, not a repository, so most identities in it are invented; PB §11 splits identities into *git object ids* and *SHA-256 over non-git artifacts*, and both classes carry fabricated values here. **Fabricated but well-formed git object ids:** `objects.base`, `objects.head`, `objects.merge_base`, `objects.tree`, `policy.ci_sh`, `policy.constitution`, `policy.keyring`, `policy.manifest`, and the `head=`, `tree=`, `base=` and `Spine-Approve`'s `base=` inside `authority.*.line`. **Fabricated but well-formed SHA-256 values:** `evidence.result_sha256` and the `voids=` inside `authority.reopens[0].line`. **Computed, and adopted from their owners rather than invented:** `objects.intent_blob` and every `blob=`/`intent=` in `authority.*.line` (`docs/spec/envelope-vectors.md` §8.2, computed over that document's published intent bytes); the `freeze=` inside `authority.approve.line` (EV §8.2, computed over its seven manifest lines); `tool.dist_hash` and `evidence.collector.dist_hash` (`docs/spec/manifest.md` §8.2, computed with `shasum -a 256` over that document's 529-byte artifact list); the three `fingerprint` members (EV §8.1's published keys, reproduced by `ssh-keygen -lf`); `policy.rules.c_t2` and `policy.rules.c_t1` (`docs/spec/constitution.md` §6.4's render for `params.langs: ["python", "ts"]`); and both `report=` digests, which are §8.1's and §8.2's own published values. **Not an identity at all:** `git_version`, `tool.version`, `evidence.ids`.

**The example repository is `params.ci: github`, and that is load-bearing rather than incidental.** `preconditions[2].status` reads `"met"` below, which after the amendment of 2026-08-26 requires all three conjuncts, and GitHub is the one shipped arrangement that supplies the third (§5.8, `docs/spec/ci.md` §10.3). Stating it keeps the vector reproducible: an implementer testing against a `gitlab` or `generic` repository must expect `"unmet"` there and a different digest, and would otherwise read a mismatch as a canonicalizer bug. **The amendment moves no byte of this vector** — the value printed in §8.2 is unchanged by it, and the digests below cover the v0.18 wire respelling, decision (c)'s `template=intent@2`, PB §11's wire order, the real fingerprints, and the four values adopted from their owners on 2026-08-27 (§8.2.1).

**This landing is `docs/spec/envelope-vectors.md` vector A, and the two documents are reconciled in this direction.** EV §8 computes the intent blob, `freeze=`, `envelope=` and every signature; this section computes both `report=` digests. Each has taken what the other computes, with **one bounded exception in EV's direction**: EV §8's `report=` values and the `dist_hash` inside its seals' `tool=` sit inside signed lines whose private keys EV does not publish, so EV cannot adopt §8.1's and §8.2's digests or `manifest.md` §8.2's `dist_hash` without regenerating its keyring and re-signing all five signatures. **EV §15 tabulates those three values and states the divergence**; until that regeneration, an implementer verifying both vectors should expect exactly those three differences and no others. This section's values are the computed ones and are not to be adjusted toward EV's fabricated placeholders.

> **Both digests below are published and are normative test vectors.** They were recomputed over the JSON exactly as printed here — the rule-5 advisory wire spelled `G11` (PB v0.18), `template=intent@2` inside `authority.signoff.line` (decision (c) of 2026-08-26), the `wires` array and bob's `wires=` in PB §11's byte order (§5.6, §6.1), and the intent blob, `freeze=`, `dist_hash` and `c_t2` adopted from their owning documents on 2026-08-27 — and §8.2.1 records how, and what the earlier disagreements were. Reproducing them is mechanical and ordered: (1) canonicalize evaluation 1's value — the §8.2 value with `authority.reviews: []` and `gates[G2].status: "fail"` — and take its SHA-256 and its length; (2) that digest is **already substituted** into bob's `report=` in the §8.2 value below, so canonicalize §8.2 as printed and take its SHA-256 and its length. §8.3's minimal vector is unaffected and stands as published; build against it first.

### 8.1 Two reports, one landing

Evaluation 1 ends in `landing-review`: `G2` has a finding nobody has accepted, so `gates[G2].status` is `"fail"` and `authority.reviews` is empty. Its digest is what bob signs:

```
canonical length = 3476 bytes
report           = sha256:e2bd8cb5da473701bf0dd8e9aa0c6a28cc1280e33ba06758165c5089ae3f5b47
```

Bob's review line, byte-for-byte, carrying that digest, with the wire tokens in the byte order of §6.2:

```
Spine-Review: INT-042 class=tripwire head=77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9 tree=3e91c7a2d0f46b58e19d73c0a5b284fd61e0c9a7 base=7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51 intent=dfb4079e22de55ec377468b9b697fdf86085ea37 report=sha256:e2bd8cb5da473701bf0dd8e9aa0c6a28cc1280e33ba06758165c5089ae3f5b47 wires=G11,G2:src/shared/util.ts reason="shared helper touched outside touchpoints; read the diff and the outcomes" reviewer=bob@example.com
```

Evaluation 2 ingests the same result file — the collector writes it at `.spine/cache/results/<T>.jsonl` and `T` has not moved (PB §11) — recomputes the same wire set, finds it covered, and seals. The two reports differ in exactly two members: `gates[G2].status` becomes `"override"`, and the review enters `authority.reviews`. Nothing else moves, because a review commit is empty and `Hc`, `T` and `merge_base` are unchanged (§5.2), and bob's `head=` still equals `objects.head`, which is what keeps the review in the array (§5.5). There is no circularity: the review names evaluation 1's digest; the seal names evaluation 2's.

### 8.2 The sealed report

Shown pretty-printed for reading. **The pretty form is not canonical.** JCS-serialize this value per §2 and the result must be exactly the length and digest below. That is the test. Bob's `report=` already carries §8.1's digest, so nothing has to be substituted first.

```json
{
  "authority": {
    "approve": {
      "fingerprint": "SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM",
      "line": "Spine-Approve: INT-042 intent=dfb4079e22de55ec377468b9b697fdf86085ea37 base=5f8e2a10cd47b6390e5a2c8db14f70369ba2e0d7 rounds=1 total_rounds=3 reopens=1 red=5/5 freeze=sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2 signer=alice@example.com",
      "namespace": "spine-review@v1"
    },
    "reopens": [
      {
        "fingerprint": "SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM",
        "line": "Spine-Reopen: INT-042 voids=sha256:4d1e0b7c9a2f83d6540e7b1c8a95f2036d4e8b71ca03f95e2b6d178c04a3e9f5 reopens=1 reason=\"AC-3 was not testable as written\" signer=alice@example.com",
        "namespace": "spine-signoff@v1"
      }
    ],
    "reviews": [
      {
        "fingerprint": "SHA256:V2dasTIGWUnhlaxa7vr3Qmqpe/qYBoxi+C7Cs6yxxEs",
        "line": "Spine-Review: INT-042 class=tripwire head=77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9 tree=3e91c7a2d0f46b58e19d73c0a5b284fd61e0c9a7 base=7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51 intent=dfb4079e22de55ec377468b9b697fdf86085ea37 report=sha256:e2bd8cb5da473701bf0dd8e9aa0c6a28cc1280e33ba06758165c5089ae3f5b47 wires=G11,G2:src/shared/util.ts reason=\"shared helper touched outside touchpoints; read the diff and the outcomes\" reviewer=bob@example.com",
        "namespace": "spine-review@v1",
        "self_approved": false
      }
    ],
    "signoff": {
      "fingerprint": "SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM",
      "line": "Spine-Signoff: INT-042 blob=dfb4079e22de55ec377468b9b697fdf86085ea37 template=intent@2 constitution=v3 reopens=1 signer=alice@example.com",
      "namespace": "spine-signoff@v1"
    }
  },
  "automerge": {
    "effective": false,
    "preconditions": [
      { "id": 0, "status": "unmet" },
      { "id": 1, "status": "met" },
      { "id": 2, "status": "met" },
      { "id": 3, "status": "met" },
      { "id": 4, "status": "met" }
    ],
    "requested": true
  },
  "evidence": {
    "collector": {
      "dist_hash": "sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db",
      "version": "1.4.0"
    },
    "ids": 412,
    "keys_visible": false,
    "result_sha256": "sha256:0b93f4ac5182d67e0a4c31fb9d20e857643ca0b1f9e78d5236ca04b81e7d3f96"
  },
  "floor_hits": [],
  "gates": [
    { "gate": "G1", "status": "pass" },
    { "gate": "G2", "status": "override" },
    { "gate": "G3", "status": "pass" },
    { "gate": "G4", "status": "pass" },
    { "gate": "G5", "status": "pass" },
    { "gate": "G7", "status": "pass" },
    { "gate": "G8", "status": "pass" },
    { "gate": "G9", "status": "pass" },
    { "gate": "G11", "status": "pass" },
    { "gate": "G12", "status": "pass" },
    { "gate": "G13", "status": "pass" },
    { "gate": "G14", "status": "pass" },
    { "gate": "G15", "status": "pass" },
    { "gate": "G16", "status": "pass" }
  ],
  "git_version": "2.45",
  "mode": "team",
  "object_format": "sha1",
  "objects": {
    "base": "7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51",
    "head": "77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9",
    "intent_blob": "dfb4079e22de55ec377468b9b697fdf86085ea37",
    "merge_base": "6a41d0c93bf7e25184ad0c76b3e91f52d7c40e8b",
    "ref": "refs/heads/intent/INT-042",
    "tree": "3e91c7a2d0f46b58e19d73c0a5b284fd61e0c9a7"
  },
  "policy": {
    "ci_sh": "51d9c0827a4e6b13f05d92ac7e380b4617fc25da",
    "constitution": "e9a2f0714c8b53d609af2e75b1c840d3629ea7f0",
    "floor_extensions": [
      "AGENTS.md",
      "CLAUDE.md",
      "CONSTITUTION.md",
      "adr/",
      "db/migrations/"
    ],
    "floor_source": "spine:1.4.0:floor",
    "keyring": "0aa71c9e4d38b60f27ec5a1943d0b8e762fa4c15",
    "manifest": "8c14a70b3d9e52f6081ac47b39d0e2f5617ab8c0",
    "rules": {
      "c_a1": "team",
      "c_a2": ["adr/", "db/migrations/"],
      "c_a3": "hostile",
      "c_m1": "merge",
      "c_m2": "full",
      "c_m3": 3,
      "c_m4": "on",
      "c_q1": ["docs/", "src/**"],
      "c_q2": 400,
      "c_t1": ["tests/", "src/**/__tests__/"],
      "c_t2": ["tests/support/**", "**/conftest.py", "pytest.ini", "pyproject.toml", "tox.ini", "setup.cfg", "package.json", "tsconfig.json", "jsconfig.json", "vite.config.*", "vitest.config.*", "vitest.workspace.*", "vitest.setup.*", "jest.config.*", "jest.setup.*"],
      "c_t3": true
    }
  },
  "profile": "container",
  "report_version": 1,
  "run": { "reverifications": 0 },
  "self_approved": false,
  "subject": {
    "event": "land",
    "intent": "INT-042",
    "lane": "gated",
    "strategy": "merge"
  },
  "threat": "hostile",
  "tool": {
    "dist_hash": "sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db",
    "version": "1.4.0"
  },
  "wires": [
    { "class": "tripwire", "gate": "G11", "kind": "advisory" },
    { "class": "tripwire", "gate": "G2", "kind": "finding", "path": "src/shared/util.ts" }
  ]
}
```

**Published vector:**

```
canonical length = 4053 bytes
report           = sha256:a47c13281b0b4a13085e523cd3c09e4f7d1caf0c80419de5fa3a869f7aec309e
first 96 canonical bytes:
{"authority":{"approve":{"fingerprint":"SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM","lin
```

The prefix is unaffected by both the v0.18 respelling and decision (c): `authority.approve` sorts first and its line changes under neither, so a canonicalizer that does not reproduce those 96 bytes is already wrong before the digest is compared.

Both digests were computed, never placeholders: reproducible by JCS alone from the value each was computed over, with no repository, no keys and no git. An implementation whose canonicalizer reproduces it is correct on every construct this schema uses — nested objects, arrays of objects, an empty array, integers, booleans, absent optional members, and JSON-escaped quotes inside `esc`-encoded lines. `floor_extensions` also shows §5.4's list rule resolved: `paths.constitution` contributes one entry and `paths.agent_context`'s two elements contribute one each, flattened into one sorted array.

The `Spine-Gates` rendering of this report's `gates` array is:

```
Spine-Gates: G1=pass G2=override G3=pass G4=pass G5=pass G7=pass G8=pass G9=pass G11=pass G12=pass G13=pass G14=pass G15=pass G16=pass
```

G6 is absent because no version-1 report carries a G6 entry (§5.6.2); G10 is absent because it is never in `Spine-Gates` (PB §11).

**`preconditions[2].status: "met"` in this example presupposes all three conjuncts**, which is a fact about the repository it is drawn from rather than a default: `keys_visible=false`, a collector `tool=` equal to the base's pin, **and** a trunk-defined origin the run established. That last is available only on the arrangements `docs/spec/ci.md` §10.3 scores as supplying it — `--ci github`'s `workflow_run` of trunk's own collect workflow is the shipped one. Re-point this example at `--ci gitlab` with in-repository configuration, or at `--ci generic`, and precondition 2 reads `"unmet"` with everything else in the report unchanged (§5.8, §5.9, §9.25).

**The bytes JCS produces from this value are also what §4.4 publishes.** The note on this landing's commit holds them exactly — no newline, no framing, no pretty form — so `git cat-file blob $(git notes --ref=spine list <L> | cut -d' ' -f1) | sha256sum` on any clone that has fetched the ref reproduces `sha256:a47c13281b0b4a13085e523cd3c09e4f7d1caf0c80419de5fa3a869f7aec309e`. Evaluation 1's report, which bob's `report=` names, is never published: it did not seal (§4.4.1).

### 8.2.1 How the two digests were recovered, and what the 56-byte disagreement was

**This section used to say the values were unrecoverable. They were not, and it is now closed.**

The pair published above has been withdrawn and recomputed five times, and the list is kept because each withdrawal is a different lesson. **(1)** PB v0.18 respelled the rule-5 advisory wire from `G1` to `G11`, changing the `wires` array and the `wires=` inside bob's signed line. **(2)** Decision (c) of 2026-08-26 respelled `template=v2` as `template=intent@2` inside `authority.signoff.line` — six bytes longer, and `signoff` sorts last within `authority`, so only the tail of the canonical form moves. **(3)** The adoption of PB §11's byte order (§5.6) re-sorted the `wires` array and bob's `wires=`; that one moved no length at all (below). **(4)** On 2026-08-27 the three `fingerprint` members, fabricated until then, were replaced by the real keys `docs/spec/envelope-vectors.md` §8.1 publishes and `ssh-keygen -lf` reproduces; all three are 43 unpadded base64 characters, so that moved the digests and no length. **(5)** In the same pass, four values that another document *computes* were adopted in place of this document's invented ones — EV §8.2's intent blob and `freeze=`, `docs/spec/manifest.md` §8.2's `dist_hash`, and `docs/spec/constitution.md` §6.4's `c_t2` render for `params.langs: ["python", "ts"]`. The first three are fixed-width and moved no length; `c_t2` grew from five patterns to fifteen and moved both.

The 56-byte disagreement recorded below happened between (2) and (3), and it is the reason this section exists. Recomputation was attempted twice and **the two attempts disagreed by 56 bytes over the same object**, one making evaluation 2 3891 bytes and the other 3835. This document concluded that the disagreement meant one of two things: an ambiguity §2 or §5 had not closed, or one wrong canonicalizer.

**It was one wrong canonicalizer, and §2 has no residual ambiguity here.** The arithmetic settles it without an owner decision:

| Value | Canonical length | Digest |
|---|---|---|
| Evaluation 1 (§8.1) — `authority.reviews: []`, `gates[G2].status: "fail"` | **3476** | `sha256:e2bd8cb5da473701bf0dd8e9aa0c6a28cc1280e33ba06758165c5089ae3f5b47` |
| Evaluation 2 (§8.2) — as printed, bob's `report=` carrying evaluation 1's digest | **4053** | `sha256:a47c13281b0b4a13085e523cd3c09e4f7d1caf0c80419de5fa3a869f7aec309e` |

The first attempt's 3891 is exactly **six less than the 3897 that printing published for evaluation 2**: it is the same value with `template=v2` in the sign-off line, so that attempt had every member right and was stale only about decision (c), which had not been taken when it ran. The second attempt's 3835 is reproducible from no value this document defines, at any point in its history. (3897 was the length before withdrawal (5); the same subtraction against the current printing is 4053 − 6 = 4047, below.) There was never a second reading of §5 in which "which members belong in a report" differed by one or two members — the delta the disagreement was attributed to does not exist, and the earlier text saying it might was the wrong conclusion drawn from two numbers.

**What was checked before publishing.** §8.3's minimal vector reproduces character for character on the canonical string and digit for digit on the digest. §8.2's published `first 96 canonical bytes` reproduce from §8.2's value. Evaluation 1 is derived from §8.2's printed JSON by the two-member delta §8.1 names and nothing else; its digest is then substituted into bob's `report=` — the one ordering dependency, and the reason the recipe above is numbered. Backing decision (c) out of both current values reproduces **3470** and **4047** — six less than each, exactly as it reproduced 3314 and 3891 against the printing the 56-byte disagreement was measured on, which is what makes the first attempt's figure diagnosable rather than merely wrong.

**The third withdrawal moved no length, and that is the trap it sets.** Re-sorting `wires` from numeric to byte order swaps two array entries and rewrites bob's `wires=` from `G2:src/shared/util.ts,G11` to `G11,G2:src/shared/util.ts`. Both are permutations: evaluation 1 stays **3476** bytes and evaluation 2 stays **4053**, exactly as they are under the numeric order, so *every length check in this document passes under both orders and only the digests separate them*. An implementation that sorts numerically will match all three published lengths, the `first 96 canonical bytes`, and §8.3, and still reproduce neither digest. If that is what you are seeing, the canonicalizer is right and the wire comparator is wrong — read §5.6 and §6.1, not §2.

**Debug against §8.3 first.** It exercises the scheme — member ordering across case and underscore, a nested object, an array of integers, a JSON-escaped quote and a JSON-escaped backslash. A canonicalizer that reproduces §8.3 and not §8.1 or §8.2 has the scheme right and the *schema* wrong, and §5 is where to look: the members, their presence conditions, and the array orders of §5.6, §5.7 and §6.1.

---

### 8.3 A minimal canonicalizer vector

Debug your canonicalizer against this before attempting §8.2. It exercises member ordering across case and underscore, a nested object, an array of integers, a JSON-escaped quote and a JSON-escaped backslash — and nothing else.

```
value:     {"b":[1,2],"a":"x\\y","Z":true,"_c":{"n":0,"m":"q\"r"}}
canonical: {"Z":true,"_c":{"m":"q\"r","n":0},"a":"x\\y","b":[1,2]}
digest:    sha256:a594772ccb6408158b6e76b170d5488c2454ba576e09ae379e24d743e21921f0
```

(The member names `Z` and `_c` are outside §2.2's `^[a-z][a-z0-9_]*$` and appear in this vector only to pin ordering behaviour. A real gate report never uses them.)

---

## 9. Resolved ambiguities

Each entry states what the playbook says, what this document chose, and why.

### 9.1 The canonicalization was never named

**Playbook:** PB §7.4 rule 4 says "canonical-JSON gate report" and claims cross-machine recomputation. PB §11 fixes only the digest algorithm.
**Chosen:** RFC 8785 JCS, restricted by the value profile of §2.2 and the `esc` encoding of §2.3.
**Why:** without a named scheme `--verify` cannot work across implementations at all, which is the finding this document exists to close. JCS is the only published standard for the job; the profile removes its two ambiguous corners (floating-point, non-ASCII ordering) rather than trusting every implementer to navigate them, and `esc` removes a third the playbook never raised — that git paths are bytes and JSON strings are not.

### 9.2 "tree" is ambiguous between `T` and `L`'s tree

**Playbook:** PB §7.4 rule 4 lists "tree" among the report's fields. PB §11 says the seal's `tree=` names `L`'s tree; PB §5.4 and G9 compare a review's `tree=` against `merge-tree(review.base, L^2)`, which is `T`. Under merge strategy these differ by one deleted file; under configuration (b) they coincide, the candidate having already deleted it.
**Chosen:** `objects.tree` is `T` — the tree every gate evaluated, with `intents/<ID>.md` still present where the candidate still had it. `L`'s tree is not a member. On a tombstone, which has no merge tree, `objects.tree` is `B`'s tree (§5.2).
**Why:** the report records an evaluation, and nothing was evaluated against `L`'s tree. `L`'s tree is derivable from `T` and the intent path, and the seal carries it independently, so storing it would be a second spelling of a derived fact — exactly the class of redundancy that makes two implementations disagree.

### 9.3 One report or two: `Spine-Review`'s `report=` versus `Spine-Seal`'s

**Playbook:** both trailers carry `report=sha256:…` (PB §11) and PB §5.4's review row binds "the gate report", without saying whether they are the same value.
**Chosen:** one schema, one report per **evaluation**. A review names the digest of the evaluation the reviewer read — which does not contain that review and typically contains a `fail`. A seal names the digest of the evaluation that landed — which contains every review that still binds and contains no `fail`. A report with any `fail` can never be sealed (§5.6.1).
**Why:** the alternative — one report per landing — is circular, since a review's signed line contains a digest that would have to cover that line. The chosen reading also gives the review→seal progression a mechanical signature: the reviewer's acceptance is exactly the transition of one gate from `fail` to `override`, visible in the two digests. §8.1 demonstrates it.

### 9.4 Recomputation versus the result file

**Playbook:** PB §7.4 rule 4 says the report is recomputable and `--verify` "re-runs the pinned release over the same objects". PB §7.4 rule 3 makes the collector's result file a non-git input that G1 cannot do without. PB §1.1 concedes the residual in the abstract but the mechanism was never stated.
**Chosen:** every member is marked recomputable or attested (§4). `--verify` requires a candidate report, rebuilds every recomputable member, copies the attested ones verbatim, and compares the digest against the seal.
**Why:** it makes `--verify` a total, sound operation with a small, closed and named residual, instead of a claim that quietly fails on G1. A forged candidate cannot produce a match, because everything a forger would want to change is rebuilt from git, and the attested members are still pinned by the seal's digest. The residual it leaves is the one PB §7.4 already names at length and refuses to paper over: a candidate's runner can lie about its own results, and no digest closes that.

### 9.5 `refs/notes/spine` is "never a source" but `--verify` needs a copy

**Playbook:** PB v0.19 §7.4 rule 4 *requires* the full report on `refs/notes/spine` — "and that is not optional" — while keeping every restriction v0.18's convenience carried: "notes are not fetched by default, no gate reads one, and a note is **never a source** … a missing or edited note is a lost convenience, never an invalid landing." PB §1.1 refuses git notes as a source of truth. So the ref went from optional-and-unread to required-and-unread, and "required" is the half that invites being read.
**Chosen:** a note (or a `--report` file) is an **input to be hashed, never a fact to be believed**. `--verify` reads it, discards every recomputable member, and refutes or confirms it against the seal. Publication is specified in §4.4; the prohibition on reading is restated in §4.4.6 and §7 rule 3.
**Why:** "source" means "believed on its say-so", and that is orthogonal to whether the artifact is guaranteed to exist. This use is the same discipline configuration (b) applies to a PR body in PB §5.4 — re-read through the API, hashed against `blob=`, refused on disagreement. Nothing depends on a note being present: absent one, `--verify` exits 2 and every gate, every seal and the whole ledger are unmoved. What v0.19 changed is who can perform the recomputation, not what the recomputation trusts.

### 9.6 The boundary between `pass` and `override`

**Playbook:** PB §11 says a `Spine-Gates` entry is `pass` or `override`, that a gate "that ran and passed its own check reads `=pass` even when it raised a wire", and that `=override` marks a gate "whose own *finding* a signed review accepted, or which break-glass bypassed". Several PB §6.3 gates write the review into their own check ("fails unless that review names its path"), so the two readings collide; and PB §7.6 and PB §6's transition row both mark a bypassed gate `=override` without asking whether it had a finding.
**Chosen:** a wire is either a **finding** of its gate or an **advisory** raised by it (§6.1). `override` iff the gate produced at least one finding and every finding is covered by an admitting review — **or** the gate is named in a `class=break-glass` review's `wires=`, finding or no finding; `pass` otherwise. Concretely: a floor hit with a protected review reads `G14=override`, not `pass`; the rule-5 `G11` precondition wire leaves `G11=pass`; a break-glass review naming `G3` reads `G3=override` on a landing where G3 was silent.
**Why:** the finding-versus-advisory rule is the only one that reproduces the playbook's own canonical envelope (PB §5.5, `G2=override` with the containment wire in `wires=`, and `G11=pass` beside a `G11` wire in the same set) and PB §5.5's reseal clause ("a G1 or G8 finding inside the range is sealed `=override`") with exactly one special case, which is in §5.6.1's table rather than only in this rationale: on a `Spine-Event: reseal` landing a G1 or G8 finding is not outright, because the reseal's own `class=protected` review admits it. That case is not optional and is not a softening of the outright rule — break-glass, the only other route to `override` for those two gates, is unavailable to a reseal (PB §5.5), and G9 refuses every landing above an orphan until the reseal lands, so the alternative is a trunk nobody can land on. Earlier drafts of this section claimed the rule reproduced PB §5.5's clause *without* a special case; §5.6.1's own outright table falsified that, and the row is the correction. The break-glass limb is not a second rule but the playbook's own two normative statements, which win over any derivation from findings; G9 validates both limbs identically, by reading each `override` back out of a copied review whose class admits it. The alternative — a review branch inside the check — makes `override` unreachable for G14 and turns PB §11's `=override` into a break-glass-only value, which PB §5.5 contradicts.

### 9.7 Warn mode has no serialization under a closed `pass|override` domain

**Playbook:** PB §6.3 says a warn-mode Drift finding "still enters the report's wire set and `wires=` — it merely does not block on its own"; PB §11 closes the `Spine-Gates` domain to `pass` and `override`.
**Chosen:** warn mode converts a gate's finding into a `kind: "warn"` wire. The wire is in the report's set and must be covered by the reviews' `wires=`; it does not route to a review state and does not affect the gate's status, so the gate reads `pass`. `forbidden` hits and G7 hard leases stay `finding` in every mode (PB §11).
**Why:** a third `Spine-Gates` value would contradict PB §11, which wins. Reading warn findings as `override` would require a review that warn mode exists to avoid. `pass` plus a recorded, covered wire keeps the finding visible, countable by `spine stats`, and free of ceremony — which is what calibration mode is for.

### 9.8 G3 measures staleness against a wall clock

**Playbook:** PB §6.3 flags "an in-flight intent older than ~14 days (committer dates — forgeable, acceptable for a warning)". PB §7.5 forbids timestamps as authority: "One clock, no timestamps."
**Chosen:** at landing, G3 compares the sign-off event commit's committer date to the committer date of `objects.base`, and the threshold is exactly **1 209 600 seconds** (14 days), a constant of the pinned release. The report holds no date and no G3 measurement — only a `G3` gate status and, when the threshold is crossed, a `kind: "warn"` wire.
**Why:** measuring against "now" would make the report non-recomputable — the same objects would yield a different digest tomorrow — and `--verify` would fail on every landing older than the threshold. Anchoring to `base`'s committer date keeps the comparison inside the chain, where PB §7.5 says the authority lives, and keeps it as forgeable as the playbook already accepts for a warning. The `~` is removed because two implementations cannot both round it.

### 9.9 A wire token cannot survive a path containing a comma or a space

**Playbook:** PB §11 gives `wires=<G<n>[:path],…>` with no escaping rule, and the trailer is a space-delimited `key=value` line.
**Chosen:** the wire token of §6.2 — `esc` with `,`, ` ` and `"` moved into the `\xHH` row, in one pass; `=` deliberately left alone.
**Why:** without it, one path with a comma makes a signed review unparseable and the PB §11 containment check undecidable, silently, and only in repositories unlucky enough to have such a path. The escape is the identity for every ordinary path, so nothing readable changes. Defining it as a second pass over `esc`'s output would double every backslash and produce a different token for the same path in the two implementations that read the definition differently.

### 9.10 `self_approved` is listed as a report field but defined per review

**Playbook:** PB §7.4 rule 4 names `self_approved` among the report's fields. PB §7.2 defines it as a property of a `Spine-Review` whose key equals the landing's signer key. PB §6.5 counts "self-approved *protected* reviews".
**Chosen:** both — a per-review boolean (what G13 enforces) and a top-level boolean equal to their disjunction (what PB §7.4 rule 4 names), with the invariant stated in §5.
**Why:** the two consumers ask different questions, and deriving one from the other at read time would put the derivation in every reader instead of in the record. `spine stats`' own counter is the per-review flag read together with `class=` out of `line`: the top-level boolean is a disjunction over every class and cannot express "self-approved protected review", so it is not what PB §6.5 counts. This is one of three deliberate redundancies in the schema, and each is checkable: `policy.floor_extensions` restates every entry of `policy.rules.c_a2` under a second ordering, and `policy.floor_source` is a function of `tool.version`.

### 9.11 `head` is ambiguous between the literal ref and the content head

**Playbook:** PB §5.4 defines `Hc` and says "Reviews and the seal name `Hc` in `head=`; wherever this section, the PB §6 rows or G9 compare a review's `head=` with `H`, read `Hc`". A gate record is bound to "(head, base, tree)" without disambiguation.
**Chosen:** `objects.head` is `Hc`.
**Why:** it matches what the seal and every review record, and it is what makes the two evaluations of §8.1 differ in two members instead of three. The literal `H` is guarded by the CAS of PB §5.4 step 6, is not a gate input, and is not recorded.

### 9.12 Which gates "ran"

**Playbook:** PB §11 says `Spine-Gates` lists "every gate that ran" and enumerates only the tombstone case (four gates).
**Chosen:** a gate runs iff every input its PB §6.3 check reads exists for this landing; the resulting table is §5.6.2. G6 is not in it for v1, having no configuration source anywhere in the playbook.
**Why:** without it, one implementation emits `G12=pass` for a quick-lane landing that has no approval and another omits it, and the two `Spine-Gates` lines — and therefore the two `envelope=` digests — differ over an identical landing. "Iff configured" is the same defect wearing a condition nobody can evaluate.

### 9.13 `C-M4` off and a failed precondition are two reasons for one wire

**Playbook:** PB §5.2 puts both on one gate in one clause — "`G11` (`C-M4`) where the constitution says off, and `G11` naming the precondition where the run computed it off — one gate, two reasons, distinguished by `reason=`" — and PB §5.5's canonical envelope carries a single `G11`, which under that example's `C-M4: on` is rule 5's.
**Chosen:** both conditions are evaluated and recorded independently in `automerge` (§5.8); both raise a `class=tripwire`, `kind: "advisory"` `G11` wire; and because both are pathless they are **one** entry in `wires` and one `G11` token in a review's `wires=`.
**Why:** the record and the wire set answer different questions, and only the record can hold both answers. An implementation that gates the precondition wire behind `requested == true` gets the shipped defaults wrong forever — under them `requested` is false *and* precondition 0 fails — and one that emits two pathless `G11` entries produces a `wires` array, a `wires=` line and an `envelope=` no conforming implementation reproduces. The distinction stays exactly where PB §5.2 puts it: in the review's mandatory `reason=`, which lives inside the review's `line` and is not re-encoded as a member, for the reason §5.5 gives of every other field of a review.

### 9.14 Which invocation of G9 the report records

**Playbook:** G9 runs over trunk before the landing is built (PB §5.4 step 3; on a tombstone, the binding walk of step 2) and again over `L` at step 5, which "always runs here and always refuses the push". PB §11 excludes G10 from `Spine-Gates` because "it runs after the seal" and says nothing about G9's second run.
**Chosen:** `gates[].G9` records the pre-build walk. The step-5 walk over `L` is a push guard and is not a member.
**Why:** the step-5 result cannot be inside the message it would have to be inside — the report is hashed into the envelope at step 4, and `L`'s seal covers that message — which is precisely PB §11's stated reason for excluding G10. Leaving it unstated lets one implementation record a walk the other cannot have performed yet, over a landing that by then either exists or has been discarded.

**The subject check G9 gained in PB v0.19 is a step-5 check and is likewise not a member.** PB §5.5 makes a landing's first line derived — a pure function of the envelope — and has G9 recompute it and refuse a landing whose subject it did not produce. That check reads `L`'s message, which does not exist at step 3, so it belongs to the same push-guard invocation as the ledger walk over `L` and is recorded here no more than that walk is. It is also **outside `envelope=`** by construction, so it moves no digest in §8, no signature, and no member of this schema: a `gates[].G9` entry reading `"pass"` is the pre-build walk's verdict and says nothing about the subject either way. The residual PB §5.5 names — the quick lane's summary is free text, and every toolkit lifecycle landing rides the quick lane — is therefore invisible to this document by design, and §5.1 says so where the name `subject` could otherwise mislead.

### 9.15 A G7 finding cannot be rebuilt once the leases are gone

**Playbook:** PB §6.3 makes G7's landing check binding over "the integrated diff over a fresh fetch of `refs/heads/intent/*`", and PB §5.4 derives the lease registry from those refs alone. Landing or withdrawal deletes them. PB §6.3 states the consequence for the proof itself: "G10 proves the ledger, not the lease registry."
**Chosen:** every `gates[]` and `wires[]` entry whose `gate` is `G7` is **attested**, not recomputable (§4).
**Why:** marking it recomputable makes `--verify` report `report-mismatch` on every landing that ever took a lease wire — turning a soundness check into a false-positive generator on exactly the landings that mattered most. The alternative, recording the in-flight ids and their declared forbidden and frozen sets as a `policy.leases` member, would buy recomputability at the price of putting another repository's declarations inside this landing's digest; the playbook already concedes the smaller thing, so this document records the concession rather than inventing the member.

### 9.16 The rule-5 `G11` wire on a landing that runs no suite

**Playbook:** PB §7.4 rule 5 raises a `G11` wire whenever a precondition fails and exempts a **tombstone** from the rule entirely; v0.18 withdraws the reseal's exemption in the same paragraph — a reseal "does run the suite, does ingest a result file, and seals the real `profile=` … preconditions 1 and 2 are evaluated for it like any other landing that tests something" — while leaving it the protected review PB §5.5 gives it in its own right. PB §11 says a tombstone "runs no gates that can produce a rule-5 `G11` wire", that a reseal "is not one of them", and that under the shipped defaults precondition 0 fails "on every run that tests anything".
**Chosen:** the wire is raised iff rule 5 applies to the landing, which is every landing but a tombstone. A tombstone records all five preconditions `"exempt"` and raises none. A reseal records all five as it computes them and raises the wire like anything else that tests.
**Why:** the wire exists so that a human reads *this landing's diff and its test outcomes* (PB §7.4 rule 5), and after v0.18 the only landing with no outcomes to read is the tombstone. Conditioning on rule 5's own applicability rather than on `profile` also survives the one landing that tests something and ingests nothing: a `result-missing` or `result-malformed` bypass seals `profile: "none"` with no `evidence` (§5.9), and its preconditions 1 and 2 are unmet, so it is exactly a landing whose auto-merge unavailability a human should have to sign for.

### 9.17 Raised by the review and not applied

- **That a reseal be recorded `"exempt"` for precondition 0** as well as 1 and 2. Rejected twice over. `"exempt"` in this schema means *the design granted an exemption*, not *the wire was suppressed*, and under `C-A3: hostile` precondition 0 is genuinely unmet for a reseal; recording it `"exempt"` would make `automerge.effective` say a reseal could have auto-merged in a repository whose threat model forbids auto-merge to exist at all. As of PB v0.18 the premise is gone as well: rule 5 grants a reseal no exemption from any precondition, so 1 and 2 are computed for it too (§5.8). The defect behind the request — the rule-5 wire on a reseal — was never a report defect: the wire is `G11`, it is raised, and the reseal's protected review is PB §5.5's own.
- **That the example line labelled `first 96 canonical bytes:` prints 84 and must be corrected.** Rejected on the facts: it prints 96. The line is `{"authority":{"approve":{"fingerprint":"SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM","lin`, which is 96 bytes — 97 in the file, counting the LF that terminates the fenced line and is not part of the prefix. It was left byte-for-byte as published, and it is the one part of §8.2's vector v0.18 does not disturb.

### 9.18 The advisory wire's gate id, and what a bare `G1` means

**Closed. PB §11 no longer disagrees with itself, and this entry is kept as a record of how the tie was broken while it did.**

**Playbook, as of v0.19:** the `Spine-Gates` row reads "a gate that ran and passed its own check reads `=pass` even when it raised a wire — the rule-5 `G11` precondition wire is not a finding about G11". PB §11's wire-aggregation paragraph agrees: "It is never spelled `G1`: a `G1` wire is a finding that named tests did not pass, and the two must never share a token a reviewer signs over." PB §7.4 rule 5 spells the wire `G11` and says why — "**G11 and not G1** … a wire is a claim about its own gate" — and PB §12's v0.18 note records the respelling as the change that version exists for. Four statements, one token.

**What the row said until v0.19,** and why this entry existed: the parenthetical read "the rule-5 `G1` precondition wire is not a finding about G1", which is the pre-v0.18 spelling.
**Chosen:** the advisory is `G11`. A `G1` wire in a version-1 report is always `kind: "finding"`; a `G11` wire is always `kind: "advisory"` (§6.1, §5.8).
**And what the bare form now means, which is narrower than it was.** A bare `G1` is no longer *any* G1 finding. `docs/spec/result-file.md` §8.5 gives a per-id finding the token `G1:` + `tok(path)` and leaves the bare form to the five that name no path — `result-missing`, `result-malformed`, an `end.status` fold that is not `complete`, an AC with no collected `verified_by` edge, and a frozen entry that collected nothing — plus any per-id finding whose `path` is the empty string. §6.3's G1 row adopts that assignment; this document invents no token across the boundary, and the entry that recorded the question as one it declined is withdrawn. A reviewer signing a bare `G1` is therefore signing *the file, the fold, or the coverage clause*, never *this test failed*, and the two are as distinguishable in the ledger as `G1` and `G11` became in v0.18.
**Why the tie was broken this way, while there was a tie:** §11 wins over prose, but §11 then disagreed with itself, so the tie went to specificity and to direction — the wire-aggregation paragraph and rule 5 state the token as a rule, the `Spine-Gates` row stated it as an example inside a clause about something else, that a gate which passed its own check reads `=pass`. That clause's general half is what this document relies on and was never in doubt. Reading it the other way would have reinstated exactly the defect PB §12 says v0.18 was written to close: two reviewers signing byte-identical `wires=G1` over a green landing and over a failing frozen test, indistinguishable to G9's ledger audit forever. **The playbook adopted the fix**, so the reading and the row now say the same thing and nothing here rests on a tie-break.

### 9.19 The order of `wires=` — withdrawn; PB §11 fixes it

**This entry chose the numeric order and was wrong. It is withdrawn, and nothing in this document rests on it any more.**

**Playbook:** PB §11 fixes the order in the `Spine-Review` row itself — *"ascending by unsigned byte value over the whole token, so `G11` precedes `G2`; a set with no order is a signature two runs spell differently"* — and PB §5.5's canonical envelope prints `wires=G11,G2:src/shared/util.ts`, which is that order.
**Chosen:** PB §11's byte order, for the `wires[]` array (§6.1) and for the `wires=` line (§6.2) alike. §8's vectors are computed under it.
**Why it was wrong, stated so the mistake is not repeated:** the entry rested on the premise that "PB §11 … fixes no order at all", and inferred that a rule stated here could therefore govern. The premise was false — §11 states the order in the same row as the field — and the inference inverted the corpus's own precedence rule, which §5.6 and the *Gate semantics* clause of §11 both give to the playbook. Two locations in this document then disagreed with each other for a version (§5.6 had been corrected, §6.1 and §6.2 had not), which is the worse failure: an implementer reading §6.2 wrote a numeric comparator, an implementer reading §5.6 wrote a byte comparator, and because re-sorting changes no byte count neither could tell from any length this document publishes. Every `report=` and `envelope=` computed under the numeric order — §8.1, §8.2, and `docs/spec/envelope-vectors.md` vectors A and D — was recomputed when the order was adopted.

### 9.20 An id-loss a `G8:<path>` review names

**Playbook:** PB §6.3 G1 exempts "the path whose ids went away" where a `class=protected` `G8:<path>` review names it — "G8's finding and never independently G1's" — and PB §4.3 makes an id collected on `B` and absent from `T`'s collection a G8 failure. PB §6.3 G8 owns the clause outright: "a landed id `T` no longer collects or does not pass fails unless that review names its path."
**Chosen:** the allocation happens when outcomes are read, not when statuses are written: the went-away shape under such a review is a `G8:<path>` finding and no G1 finding at all, so the landing records `G1=pass`, `G8=override` (§5.6.1). That exemption does not reach an id that collected and did not pass, which is a finding of both — **except under the separate `xfail`/`skipped` carve-out**, which is its mirror image: PB §6.3's G1 and G8 rows both release an id whose own collected outcome on `B` was already `xfail` or `skipped` and which still collects on `T`, and release it from **both** gates with no review at all, so the landing records `G1=pass`, `G8=pass` and the wire set is empty on its account. The two rules apply to disjoint shapes — one to *went away* and only under a review, the other to *did not pass* and never under one — and neither is a special case of the other.
**Why:** §5.6.1's `override` requires a covering wire that names the gate it excuses, and it must — that is what makes `G9`'s audit of an `override` decidable. Applying it to a G1 finding covered by a `G8:<path>` review would leave every reviewed deletion of a landed test with an uncovered G1 finding, hence `report-not-landable`, hence no path at all for the operation PB §4.3 and PB §6.3 exist to build. PB §12 records this seam closing once already, in v0.14 — "G1 would have killed the reviewed-deletion path G8 builds" — and it reopens the moment the allocation is treated as a status rule rather than an attribution rule. The `xfail`/`skipped` carve-out is the same argument at a different point on the same axis and must be implemented the same way, as an attribution rule evaluated before any status is written: treated as a *covered* finding it would need a covering wire, no review class admits a G1 finding, and every repository carrying one long-standing `xfail`, or one ordinary skipped test, on trunk would be permanently unlandable through the quick lane — the deadlock PB §6.3's exemption exists to remove. The cost is that G8's entries join the attested set (§4): one clause of its check reads test outcomes, the member records one status, and a status a non-git input can move is not recomputable.

### 9.21 The report of a landing that ingested no result file

**Playbook:** PB §7.6 puts G1 in the break-glass bypass list and PB §11 admits exactly four values for `profile=`. PB §11 reserves `n/a` for a landing that runs no suite, which after v0.18 is the tombstone alone.
**Chosen:** `evidence` absent, `profile: "none"`, preconditions 1 and 2 `"unmet"`, `G1` `fail` or — under a `class=break-glass` review naming it — `override` (§5.9).
**Why:** `docs/spec/result-file.md` §8.2 makes `result-missing` and `result-malformed` G1 findings rather than states, §8.7 makes them break-glassable, and a bypass that lands produces a seal carrying `report=` and `profile=` — so the report needs a shape or the landing is unrepresentable. Each value is forced rather than chosen: `n/a` would claim no suite was attempted, a fifth value would break PB §11's closed domain, and `"none"` is what the companion spec fixes for exactly this case — "a seal must never claim a boundary no header established". `evidence` cannot be present because every member of it is read from a header that does not exist, which is why its presence condition is now "a result file was ingested" rather than `profile ≠ "n/a"`: the two coincided only while nothing could ingest nothing and still land.

### 9.22 `keys_visible=true` is representable

**Playbook:** PB §7.4 rule 0 has the collector assert key visibility and rule 5's precondition 2 reads that "it carries rule 0's key-visibility assertion, from a collector whose `tool=` is the base's pin" — a phrasing that reads as a presence test.
**Chosen:** precondition 2 is `"met"` iff `keys_visible=false` **and** the collector's `tool=` is the base's pin **and** this run established a trunk-defined origin for the ingested file (the third conjunct, added by the owner's decision of 2026-08-26; §5.8, §9.25). `evidence.keys_visible` may be `true` and the report is legal (§5.8, §5.9). This entry was written before that decision and named two conjuncts; the third is stated here so that §9.22 and §5.8 cannot be read against each other.
**Why:** `docs/spec/result-file.md` §8.4 settles it — "precondition 2 holds iff `keys_visible=false` and §8.3 step 2 passed. `keys_visible=true` fails it" — and neither failure refuses ingestion, because the design's answer to weak isolation is a human reading the landing, never nothing happening. The presence reading is also internally impossible here: `automerge.preconditions[2].status` is a member with a three-value domain and is marked attested, which is only meaningful if a report can exist with the assertion false. A `true` assertion is a fact about the run that the ledger should keep, and suppressing it by refusing the run would delete the evidence instead of routing it.

### 9.23 "Publishes the full report" fixes no bytes, no object and no reader behaviour

**Playbook:** PB v0.19 §7.4 rule 4 makes publication mandatory — "The trusted stage publishes the full report to `refs/notes/spine`, and that is not optional" — and PB §11 lists the ref among the things not in the repo, "required on every landing, fetched by nobody automatically, and never a source". Neither says which object the note annotates, what exactly it contains, when it is written relative to the CAS, what a second note for one landing means, or what a reader does when it is missing or wrong.
**Chosen:** §4.4. The note annotates the landing commit `L`; it holds exactly the canonical bytes of §2 for the report that landing's seal names, so `sha256sum` over the note blob reproduces `report=`; it is written from a blob with `-C`, never from a message, because git terminates a message with a newline; it is written after the CAS, its failure fails the job and never the landing, re-publication of identical bytes is a no-op and of different bytes is refused. A reader with no note exits 2 and the landing stays valid; a reader with a note whose bytes do not hash to the seal exits 1 `candidate-mismatch` and recomputes nothing from it (§4.1, §4.3).
**Why:** "publish the report" is a requirement two implementations can both satisfy while producing notes that hash differently — one appends a newline, one pretty-prints, one annotates the tree, one publishes the review-stage evaluation — and every one of those makes `--verify` fail for a third party over a landing that is perfectly sound. The digest is the artifact; a publication rule that does not reproduce the digest publishes nothing. The reader rules are stated because a mandatory artifact is one people start to rely on: fixing that its absence is exit 2 and its corruption is exit 1, and that neither touches the landing's validity, is what stops the note becoming a source by habit (§4.4.6).

### 9.24 A test id is a pair, and this document counts pairs

**Playbook:** PB §4.3 as of v0.19: "`Spine-Test` ids are collected *function* ids without parametrization suffixes, qualified by the runner that collected them, since a repository may run several (`params.langs`); the pair is the identity."
**Chosen:** wherever this document says *id* it means the `(runner, id)` pair — `evidence.ids` counts pairs (§5.9), and G8's went-away clause is about a pair going away, which a dropped or reconfigured runner can cause as surely as a deleted test (§5.6.1). No id and no runner name is a member.
**Why:** the count is inside the digest, so an implementation that de-duplicated across runners — treating one runner-native string collected by two runners as one id — would produce a different `evidence.ids` over an identical collection, and therefore a different `report=`. Recording the ids themselves would be diagnostics (§11) and would put the join this document does not own inside every landing's digest; recording the count of pairs records exactly the fact the header already asserts and the seal already covers.

### 9.25 A result file whose trunk-defined origin cannot be demonstrated

**Playbook:** PB §7.4 rule 0 required the untrusted job to run from trunk's own definition and said a result file from a job that was not "is never ingestible" — naming no test the trusted stage can perform. `docs/spec/ci.md` §14 R14 refused that reading and narrowed auto-merge precondition 2 instead. The owner settled it on 2026-08-26 in favour of the narrowing, and PB §7.4 rule 0 now reads that such a file "is still **ingested** — it simply fails auto-merge precondition 2".

**Chosen:** the file is ingested; `evidence` is present; `profile` is the header's own value; **`automerge.preconditions[2].status` is `"unmet"`** by a third conjunct; `automerge.effective` is `false`; `gates[]` is whatever the gates found, `G1` included; the rule-5 `G11` wire is raised with the cause in its `reason=` (§5 `profile`, §5.8, §5.9). **No schema change**: no member added, none retyped, `report_version` unchanged, every §8 digest standing.

**Why:** three things had to be true at once and only this shape makes them so. **(1) The landing has to be representable.** Under the strict reading a GitLab-in-repository or `--ci generic` run refused its file, which is `result-missing`, which is a G1 finding that the quick lane cannot override at all (`docs/spec/result-file.md` §8.7) — so two of the three shipped providers could land nothing, and there was no report to specify because there was no landing. **(2) The doubt has to be recorded, not laundered.** Where the evidence is absent the whole file may be the candidate's (`docs/spec/ci.md` §10.2), so a report that recorded only `G1: "pass"` would assert more than the run established. Precondition 2 is where PB §7.4 rule 0 had already put exactly this claim — "a run whose ingested header lacks either … fails precondition 2 of rule 5" — so the narrowing adds no new mechanism and reuses the one signed artifact already dedicated to auto-merge availability. **(3) It has to be a narrowing.** Nothing that failed precondition 2 before passes now; a repository on GitHub sees no change; a repository on GitLab or `generic` sees `"unmet"` on every run forever, which is what `spine init`'s refusal of `merge.auto = on` already told it (`docs/spec/ci.md` §8.1, §9.3).

**Rejected, and why each would have cost more than it bought.** *A fourth precondition status* (`"unverifiable"`, or `"unmet"` split by cause): a new token in a digest-bearing member, a `report_version` bump, §3.2 refusals from every existing reader, and a false shape — the three conjuncts can fail together, so a cause-bearing status has to pick one. The cause belongs in the review's `reason=`, where §9.13 already puts the analogous distinction. *A second `G11` wire, or a distinct wire token*: the wire is keyed `(G11, pathless)` and §6.1's uniqueness rule collapses duplicates; two entries would produce a `wires` array, a `wires=` line and an `envelope=` no conforming implementation reproduces, which is the collision PB v0.18 existed to close (§9.18). *Downgrading `profile` to `"none"`*: it would write a false fact about a boundary in order to express a true doubt about provenance, and the doubt already has a member. *Recording `params.ci`, or a `provider_verified` boolean*: `policy.manifest` pins `params.ci` already (§5.4), and the arrangement being *capable* of the evidence is not the run having *obtained* it — the second is what precondition 2 records and the first would invite a reader to compute it wrongly. *Suppressing `evidence`*: its members have a source here, unlike the no-file case (§5.9), and `result_sha256` pins the exact bytes any forgery would consist of, which is the one thing that makes a post-mortem possible.

**A playbook defect the amendment left behind — now closed, and the record kept.** As filed, `PLAYBOOK.md` §7.4 rule 0 carried the narrowed rule while precondition 2's own text one paragraph below did not, and still read *"it carries rule 0's key-visibility assertion, from a collector whose `tool=` is the base's pin"* — **two conjuncts where the decision requires three**. That was the worst-placed wrong value in the corpus: an implementer following the precondition's own line wrote a two-conjunct test, and every `--ci gitlab` or `--ci generic` repository then recorded `preconditions[2].status: "met"` and `automerge.effective: true` on a result file its own candidate could have written — a wrong value in a **signed, digest-covered** member, reachable by following the playbook rather than by misreading it. **PLAYBOOK.md v0.19 has taken the transcription**: precondition 2 now reads *"it carries rule 0's key-visibility assertion, **and** its collector's `tool=` is the base's pin, **and** the run demonstrates a trunk-defined origin for the job that produced it (rule 0). Three conjuncts, not two"*. This document's §5.8 and §9.22, `docs/spec/result-file.md` §8.4 and `docs/spec/ci.md` §14 R14 all now agree with it, and no document in this directory states the two-conjunct test. The entry is kept rather than deleted because a reader meeting a pre-v0.19 clone needs to know which reading is the live one.

**One residual, named rather than closed.** `params.ci` is inside `.spine/**` and floor-protected, so moving a repository from `github` to `gitlab` takes a `class=protected` review — but the review's subject is a one-word manifest edit, and nothing in this report makes visible that the edit retired precondition 2's reachability for good. Filed as `docs/spec/ci.md` OPEN-3 and `docs/spec/result-file.md` OPEN-7; it is a PB §6.7 and G16 question, not a schema one.

---

## 10. Owner decisions, settled 2026-08-26

This document had three open owner calls. All three are decided; the labels are kept so anything citing `OPEN-n` lands on its resolution. **None is re-opened by implementation experience alone** — each was settled on the design, and the argument for reversing one has to be a new argument.

**OPEN-1 · A `report` pin in the manifest — decided: no pin.** The schema version stays in the report and the pinned release stays its authority (§3.1). The alternative — a `report: 1` key beside `envelope: 1` and `schema: 7` in `.spine/manifest.json` — was never rejected on cost: it would cost no `manifest_version` bump, since `report` would not be a frozen field and PB §6.7 has every binary parse the frozen fields and carry the rest opaque. It is rejected on **redundancy**, which is the more expensive fault. `cli.version` + `cli.dist_hash` already name the release that produces reports, G15 binds the running binary to them, and a reader that meets an unknown `report_version` refuses (§3.2) rather than needing a pin to have warned it. A second spelling of a fact `cli` already fixes is a second thing two implementations can disagree about, and the disagreement surfaces exactly where it is worst: on the one landing form where the two can legitimately differ — a rollback or re-init that `init` seals under a release the base does not pin (§3.1) — a `report` pin at `base` and the `tool` in the seal would name different versions, and nothing in the design says which wins. No pin means there is nothing to reconcile.

**OPEN-2 · Whether CI must publish the report — decided: it must, on every landing.** PB §7.4 rule 4 no longer makes `refs/notes/spine` optional, and the earlier recommendation here (a default-on convenience with an off switch, documented in `SECURITY.md`) is **withdrawn**. It was wrong in the way an opt-out usually is: what a repository turned off was not its own convenience but every third party's ability to recompute the judgement, and the loss is unrecoverable, because the attested members of a report exist nowhere but the report once the CI artifacts expire (§4.4.4). A flag would have made the shipped promise of §1.1 — an offline clone that can re-verify — a per-repository setting that no clone holder can detect until they need it. The normative content moves to **§4.4**, which fixes the ref, the annotated object, the exact bytes, the write path, the ordering against the CAS, republication, concurrency, fetching, and what a reader does when the note is missing or does not hash to the seal. Authority is unchanged and §4.4.6 says so in one place: no gate reads a note, the ledger derives from commits alone, and a note is never a source.

**OPEN-3 · Whether staleness becomes a constitution rule — decided: no.** 1 209 600 seconds stays a constant of the pinned release (§9.8), and there is no thirteenth scaffolded rule. A team that wants a different window still has no lever, and that is the answer rather than the cost of it: PB §10's budget counts a thirteenth rule, `C-F1: staleness.days` would need its own gate id and its own `enforced_by`, and the playbook's own standard for turning a threshold into a knob is evidence from `spine stats` that teams need it — which cannot exist before v1 ships. The report is unaffected either way: it holds no date, no window and no G3 measurement, only a `G3` status and, when the threshold is crossed, a `kind: "warn"` wire (§9.8). If the rule is ever added, it appears in `policy.rules` as `c_f1` and that is a `report_version` bump.

**Four decisions of PB v0.19 are folded in rather than listed here**, because each lands on a member or a rule rather than on this document's shape: the v1 language set and its resolvers (§4, §5.4.2), runner-qualified test ids (§5.6.1, §5.9, §9.24), `params.timeout` (§5.4, §5.9), and the required note (§4.4). What they leave open is not this spec's to close: the source-symbol → runner-id join that G1's coverage clause and G5's orphan clause both assume is specified in `import-resolver.md` §12, per runner and file-granular; this document adopts it and fixes nothing about it (§5.4.2, §11).

**The owner's six decisions of 2026-08-26 are folded in the same way**, and not one moves the schema. **(1) The v1 language set is four — Python, TypeScript/JavaScript, Dart, Swift; Kotlin is dropped** (§4, §5.4, §5.4.2, §11): no member names a language, so what changes is which resolvers a `G8` status can be recomputed by, and the reason Kotlin went is that its failure mode was a silently-incomplete closure that two blind implementations would agree on and `--verify` would confirm (§5.4.2). **(2) Provider evidence is narrowed** (§5 `profile`, §5.8, §5.9, §9.22, §9.25): auto-merge precondition 2 gains a third conjunct and a file of undemonstrable origin is ingested rather than refused, so `automerge.preconditions[2].status` takes `"unmet"` in one more circumstance and nothing else in the schema moves. **(3) G14 and G16 have their own document** — `docs/spec/manifest.md` — which this document's `policy` and `G14`/`G16` statuses now delegate to, alongside `constitution.md` and `import-resolver.md`, as a third external spec that can move a member's *value* without a line of this one changing. **(4) `Template:` names the variant and the version** (`intent@2`, `intent-change@2`, `intent-bug@2`): the bytes inside `authority.signoff.line` change, this document copies that line verbatim and normalizes nothing (§5.3), so the member's type, presence and rule are untouched and only the example in §8.2 moves. **(5) An unbounded `forbidden` set stays legal**, with a `spine stats` counter for landings whose only protected wire is a G7 hard lease — a predicate over `wires` computed at read time and stored in no member (§6.1). **(6) The landing subject is derived and G9 checks it**: a different object from this document's `subject` member, outside `envelope=`, covered by no digest and recorded nowhere here (§5.1).

---

## 11. Out of scope

Deliberately not specified here, and where it belongs instead:

- **The envelope's grammar** — trailer syntax, line folding, the fenced intent block, `envelope=`'s byte range, and what the signed payload of a `-Sig` line is: `docs/spec/envelope-vectors.md`. This spec records a trailer line's bytes; it does not define them. That document must adopt §6.2's `tok` verbatim.
- **The result file** — its records, its header layout, its ingestion order, and its outcome enum (`docs/spec/result-file.md` §5): `docs/spec/result-file.md`. This spec names the file by SHA-256 and nothing else, and **does not restate the enum**: it is closed there, this document does not own it, and a citing spec that re-enumerates a closed set teaches a reader the list it happened to copy. PB §6.3 G1's "not skipped, xfail, deselected, or absent" is a phrase about which outcomes are not a pass, not a statement of the vocabulary's size.
- **The constitution's grammar** — how `C-Q1`'s pattern list is split, how `enforced_by` parses, how a team rule is written: `docs/spec/constitution.md`. `policy.rules` records that parser's output, and **that dependency is normative, not decorative** (§5.4.1): two implementations that agree on every byte of this document still disagree on `policy.rules` until that document exists, and every vector in §8 is only as reproducible as it is. It is one of two out-of-scope pointers that can invalidate this spec without touching it; the other is next.
- **The per-language resolvers** — `id → fn`, `id → path`, and the static import walk, for each of the four v1 languages (Python, TypeScript/JavaScript, Dart, Swift): `docs/spec/import-resolver.md`. `gates[]`'s `G8` entry is that walk's verdict recorded as a status, and **that dependency is normative, not decorative** (§5.4.2): G8 recomputes the freeze closure in CI, so two resolvers differing on one edge case write different statuses over identical objects, reject each other's approvals, and report `report-mismatch` on each other's landings. The **source-symbol → runner-native-id join** that G1's AC-coverage clause and G5's orphan clause both assume belongs there and **is written there**: `import-resolver.md` §12.1–§12.3. A `G1` or `G5` status is determined up to that section, on the same normative-alongside terms as the closure walk above.
- **The intent doc's grammar** and touchpoint matching: `docs/spec/intent-doc.md`.
- **`spine index --dump`**, the format G10 diffs: `docs/spec/dump.md`. The graph is a per-run rebuild, never a fetched or persisted input (§7 rule 3); a gate result that comes from querying it is recomputable because the rebuild is a deterministic function of the same git objects (PB §6.1, PB §7.4 rule 3).
- **The floor list's contents and format**, and the three CI definitions: the release and `docs/spec/ci.md`. That document owns the *wiring* of §4.4 — which job pushes `refs/notes/spine`, what `spine init` writes into each provider's definition, and how the push is retried — and must adopt §4.4.1 and §4.4.2 verbatim, because a note that is not byte-exact publishes nothing.
- **Gate semantics.** What G2 containment *means*, how the freeze closure is computed, how G14 casefolds — all PB §6.3 and PB §4.3, and for the three Authority gates that read `.spine/**`, `docs/spec/manifest.md`: **G13 §4.8**, G14 §5, G16 §6, each with its own ordered check list, status vocabulary and verdict. This spec fixes how a gate's *result* is recorded, never what the gate decides. PB §6.3 as of v0.19 closes what was named here as a gap: the **schema/auth/public-API** wire is withdrawn, **diff size** is `git diff --numstat --no-renames` over `merge-base..Hc` with additions plus deletions summed, binaries refused and floor and spine-owned paths exempt, and the **new-dependency** wire is a change to a package manifest whose per-language paths `docs/spec/import-resolver.md` lists. `gates[]`'s `G2` entry is recomputable to that extent and no further; §6.3 records the tokens those sub-checks raise, which is this spec's business, while what they mean stays PB §6.3's.
- **The trusted stage's own workflow definition**, which PB §7.4 rule 0 makes policy. It is not a member. Rule 4's field list does not name it; its path is provider-dependent and, for `--ci generic` or a dispatcher living outside the repository, there is no blob to name — so the member would be absent exactly where rule 0 matters most, and an optional policy blob is worse than none. What spine does own and every provider executes, `.spine/ci.sh`, is recorded as `policy.ci_sh`; the workflow that invoked it is pinned by the base commit and audited by rule 0's own per-run probe, which latches nothing.
- **How a run establishes that its result file came from a trunk-defined job**, per provider: `docs/spec/ci.md` — §10.3 scores each shipped arrangement and §14 R11 fixes GitHub's three-clause test. This spec fixes only what the answer *does*: it is the third conjunct of auto-merge precondition 2, it makes that status `"unmet"` when absent, and it changes no other member (§5.8, §9.25). A provider test restated here would be a second, unsealed copy of that document's, going stale the first time a provider renamed an event — and the two copies would then write different `preconditions[2]` values over identical runs, which is the one failure this whole spec exists to prevent.
- **Diagnostics.** Which test id vanished, which import could not be resolved, which line of the diff drifted: `spine review`'s packet (PB §6.5) and the CLI's own output. The report is decision-bearing; every byte of diagnostic prose is a byte two implementations can differ on.
- **Metrics.** Cycle time, token cost, bounce-back counts, wire fire rates: `spine stats`, which reads the graph, not reports (PB §6.5).
- **Storage, retention and transport** beyond §4.1's resolution order and §4.4's publication rules. How long a host keeps a notes ref, whether it is mirrored, and what a fork inherits are the repository's business; what the note must contain if it exists at all is not.
- **The graph's representation of the report** — `changeset.report_sha256` in PB §6.2 — which is derived from the seal, not from a report.
- **Any second digest, second format, or exported rendering** of the report. A rendering that ships is counted by PB §10's graph budget, and this artifact ships none.
