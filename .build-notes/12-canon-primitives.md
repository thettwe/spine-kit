# Shared primitives — JCS canonical JSON, `esc`, `tok`, digests, git object handling

Build sheet 12. Everything an implementer needs to write the canonicalization / encoding / digest layer of
spine-kit in Rust **without reading the corpus**, with a citation on every requirement.

Citation convention (the corpus's own): `PB §n` = `PLAYBOOK.md`; `GR` = `docs/spec/gate-report.md`;
`EV` = `docs/spec/envelope-vectors.md`; `DM` = `docs/spec/dump.md`; `MF` = `docs/spec/manifest.md`;
`RF` = `docs/spec/result-file.md`; `CI` = `docs/spec/ci.md`; `CN` = `docs/spec/constitution.md`;
`IR` = `docs/spec/import-resolver.md`.

**Precedence rule for this whole sheet** (`docs/spec/README.md`, "Status"): *"Where prose here and the
playbook's §11 disagree, §11 still wins — report it as a defect in one of them."* GR restates it: PB §11
"wins over prose here as it wins there" (GR, front matter). Otherwise the spec resolves PB's ambiguity.

---

## Sources read (file + line ranges, read in full)

| File | Lines | What |
|---|---|---|
| `docs/spec/gate-report.md` | 1–10 | front matter: precedence, amendment list, "report schema does not move" |
| `docs/spec/gate-report.md` | 12–26 | §1 what the artifact is; "not a git object"; note publication |
| `docs/spec/gate-report.md` | 27–36 | §2.1 the scheme by name + **Digest** paragraph |
| `docs/spec/gate-report.md` | 37–54 | §2.2 the value profile (table + reduction + impl note) |
| `docs/spec/gate-report.md` | 55–84 | §2.3 `esc` (table, decoding, worked cases, no-normalization) |
| `docs/spec/gate-report.md` | 85–111 | §3.1–§3.2 `report_version`, unknown version/member |
| `docs/spec/gate-report.md` | 112–139 | §3.3, §4 recomputable vs attested |
| `docs/spec/gate-report.md` | 140–181 | §4.1–§4.3 candidate resolution, exit codes, normative order |
| `docs/spec/gate-report.md` | 182–258 | §4.4–§4.4.6 `refs/notes/spine` publication (ref, object, bytes, write path, ordering, concurrency, fetching) |
| `docs/spec/gate-report.md` | 259–298 | §5 top-level schema table (`object_format`, digest-bearing members) |
| `docs/spec/gate-report.md` | 299–329 | §5.2 `objects` (oid members) |
| `docs/spec/gate-report.md` | 318–329 | §5.3 `tool`, `git_version` and the **normative version parse** |
| `docs/spec/gate-report.md` | 330–392 | §5.4–§5.4.1 `policy` (blob oids, `floor_source`, `floor_extensions`, `rules`) |
| `docs/spec/gate-report.md` | 392–435 | §5.5–§5.5.1 statements: `line` bytes, fingerprint form, ordering |
| `docs/spec/gate-report.md` | 435–449 | §5.6 `gates[]` order vs `wires[]` order + the grep inventory |
| `docs/spec/gate-report.md` | 450–533 | §5.6.1–§5.7 status domain, outright table, `floor_hits` |
| `docs/spec/gate-report.md` | 531–627 | §5.8–§5.10 `automerge`, `evidence` (`result_sha256`), `run` |
| `docs/spec/gate-report.md` | 628–717 | §6.1–§6.2 the `wires` array, uniqueness, ordering, **`tok`** |
| `docs/spec/gate-report.md` | 683–717 | §6.3 per-gate wire tokens (`tok(path)` sites) |
| `docs/spec/gate-report.md` | 718–736 | §7 determinism rules 1–12 (rules 4,5,6,7,8,9,10,11 are this sheet's) |
| `docs/spec/gate-report.md` | 737–960 | §8–§8.3 worked example, published vectors, §8.2.1 arithmetic, **§8.3 minimal canonicalizer vector** |
| `docs/spec/gate-report.md` | 961–1000, 1079–1098 | §9.1–§9.6, §9.19–§9.20 resolved ambiguities |
| `docs/spec/gate-report.md` | 1133–1167 | §10 owner decisions, §11 out of scope |
| `PLAYBOOK.md` | 983–1039 | §11 Vocabulary: **Hash policy**, trailers, wire aggregation, subject lines, files/refs, git requirements |
| `docs/spec/dump.md` | 25–95, 219–222, 273, 321–334, 533, 861, 882 | §2.1–§2.5 JCS/JSONL profile, `esc` adoption, `tok` non-use, dump digest, byte order |
| `docs/spec/manifest.md` | 31, 36–111, 137–147, 1056–1061, 1133–1160 | §2.1–§2.5 manifest JCS profile & `esc` map, §3.2 `dist_hash`, §7 rules, §8.2 `dist_hash` vector |
| `docs/spec/envelope-vectors.md` | 7, 128–145, 207–216, 267–329, 1004–1041, 1048, 1098 | normative deps, two path encodings, `envelope=`, `freeze=`, `ls-tree` quoting, §15 reconciliation |
| `docs/spec/result-file.md` | 66–170, 424–438, 548–564 | §4.2–§4.5 its own canonical-JSON profile, ordering, `tok` use for `G1:` |
| `docs/spec/ci.md` | 485–500, 596, 624–646, 1318, 1337 | ci.sh digests, note publication wiring, exit code 5 |
| `docs/spec/README.md` | whole (88 lines) | status, six owner decisions, **published-digest index** |

**Verified in-session** (not merely read): GR §8.3, GR §8.1, GR §8.2 and MF §8.2 all reproduce
byte-for-byte and digit-for-digit under `json.dumps(v, sort_keys=True, separators=(',',':'),
ensure_ascii=False)` / `shasum -a 256`. See *Worked examples*.

---

## Data model

### 1. Canonical value (the JCS value space of a gate report) — GR §2.2

| Concept | Rust shape | Domain | Default | Required |
|---|---|---|---|---|
| member name | `&str` | `^[a-z][a-z0-9_]*$`, ASCII only (GR §2.2) | — | yes |
| number | `u64` | integer, `0 ≤ n ≤ 2^53 − 1`; no sign, no leading zero, no fraction, no exponent, no `-0` (GR §2.2, §7 rule 7) | — | — |
| string | `String` (ASCII) | every char in `U+0020…U+007E` **after** `esc` (GR §2.2) | — | — |
| boolean | `bool` | `true` / `false` | — | — |
| null | — | **never emitted** (GR §2.2, §7 rule 6) | — | never |
| array | `Vec<Value>` | order fixed per field by GR §5/§6; JCS preserves it; `[]` is a value, not an absence (GR §7 rule 5) | — | — |
| object | map | duplicate member names invalid; a parser meeting one **refuses the document** (GR §2.2) | — | — |
| depth | — | bounded by the schema; no recursion (GR §2.2) | — | — |

### 2. Digest / id scalar types (PB §11 hash policy; GR §7 rules 9–10)

| Type | Lexical form | Where it appears | Notes |
|---|---|---|---|
| `Oid` | lowercase hex, **full** length implied by `object_format`: 40 (`sha1`) or 64 (`sha256`) | `objects.base/head/merge_base/tree/intent_blob`, `policy.manifest/keyring/constitution/ci_sh`, a `G13` wire's `path`, `Spine-Approval`, `Spine-Frozen`'s `<oid>`, `manifest=`, `from-manifest=`, `files[].blob`, `Spine-Trust-Root-Prev` | "Never abbreviated, never uppercase, never prefixed. The playbook's `9f2c…` is display, not a value." (GR §7 rule 9) |
| `Sha256Digest` | `"sha256:"` + exactly 64 lowercase hex | `report=`, `envelope=`, `freeze=`, `dist_hash`, `voids=`, `run=`, `evidence.result_sha256`, `evidence.collector.dist_hash`, dump digest | "Never bare hex, never uppercase, never another algorithm." (GR §7 rule 10; MF §7 rule 10) |
| `ObjectFormat` | `"sha1"` \| `"sha256"` | `report.object_format`, dump header `object_format`, manifest `object_format` | "from the manifest at `base` (PB §6.7). Fixes oid length: 40 or 64 lowercase hex." (GR §5) |
| `Fingerprint` | `"SHA256:"` + unpadded base64 (`ssh-keygen -lf` form), 43 chars for a 32-byte digest | `authority.*.fingerprint` | not a member of the hash policy; it is an SSH key fingerprint (GR §5.5). A published one whose last base64 char's alphabet index is not ≡ 0 (mod 4) is not in the value space (README, "What writing these has already found") |
| `EscString` | ASCII, `U+0020…U+007E` | every path / pattern / principal / trailer line in a report or dump | output of `esc` |
| `WireToken` | `G<n>` or `G<n>:` + `tok(path)` | `wires[]` sort key; `Spine-Review`'s `wires=` | GR §6.2 |
| `GitVersion` | `"<major>.<minor>"` | `git_version`, seal's `git=` | GR §5.3 |

### 3. `esc` — GR §2.3

`esc: &[u8] -> String` (ASCII). Total. Inverse `unesc: &str -> Result<Vec<u8>>` total and unambiguous.

### 4. `tok` — GR §6.2

`tok: &[u8] -> String` (ASCII). **One pass** over the bytes, never `esc` composed with a second escaping pass.

### 5. Digest inventory — which artifact takes which hash (PB §11 + owners)

**Git object ids** (`<oid>`, in the repo's object format) — PB §11: *"Git object ids (`<oid>`, in the repo's
object format) for everything that is a git object: intent blob, frozen files, trees, commits."*

| Artifact | Member / field | Owner |
|---|---|---|
| intent blob | `objects.intent_blob`, `blob=` on `Spine-Signoff`/`Spine-Withdraw`, `intent=` on `Spine-Approve`/`Spine-Review` | GR §5.2; PB §11 |
| frozen files | `Spine-Frozen: <oid> <path>` | PB §4.3, PB §11; EV §4.3 |
| trees | `objects.tree` (= `T`), seal's `tree=` (= `L`'s tree), review's `tree=` | GR §5.2, §9.2; PB §11 |
| commits | `objects.base`, `objects.head` (`Hc`), `objects.merge_base`, `Spine-Approval`, `Spine-Trust-Root-Prev`, a `G13` wire's `path` | GR §5.2, §6.1 |
| policy blobs | `policy.manifest`, `policy.keyring`, `policy.constitution`, `policy.ci_sh` | GR §5.4 |
| manifest blob | `Spine-Upgrade`'s `manifest=`, `from-manifest=`, `files[].blob` | PB §11; MF §2.4 |
| `.spine/ci.sh` | its `files[]` blob id — published `131f13fb0312162579605999d3f9f4e90098c74c` | CI §5.3 |
| the published note's content | a **blob** created with `git hash-object -w --stdin` | GR §4.4.2 |

**SHA-256 (`sha256:<hex>`)** — PB §11: *"SHA-256 (`sha256:<hex>`) only for non-git artifacts: release
artifact list (`dist_hash`), gate report, freeze digest, envelope digest, B's transcript."*

| Artifact | Field | Bytes hashed | Owner |
|---|---|---|---|
| release artifact list | `cli.dist_hash`, seal's `tool=<version>+sha256:<dist_hash>`, `tool.dist_hash`, `evidence.collector.dist_hash` | the `sha256sum`-format artifact list, one artifact per target, sorted by artifact name | PB §6.7; MF §3.2, §8.2; CI §5.5 |
| gate report | `report=` on `Spine-Review` and `Spine-Seal`; also the note's content | exactly the canonical JCS bytes | GR §2.1, §4.4.1 |
| freeze digest | `freeze=` on `Spine-Approve`; `voids=` on `Spine-Reopen` names one | sorted `Spine-Frozen` + `Spine-Test` **whole lines**, LF-joined, no trailing LF | EV §4.1–§4.2 |
| envelope digest | `envelope=` on `Spine-Seal` | every `Spine-*` line above the `Spine-Seal` line, message order, LF-joined | EV §3.1 |
| B's transcript | `run=sha256:<hex>` on `Spine-Approve` (optional) | out of scope of every read spec | PB §11 |
| result file | `evidence.result_sha256` | "the result file's exact bytes as the collector wrote them" | GR §5.9 |
| dump | the dump digest | the whole byte stream **including the final LF** | DM §2.5 |

DM §2.5 states the classification rule for anything new: *"It is a non-git artifact, so PB §11's hash policy
makes it SHA-256 (`gate-report.md` §7 rule 10)."*

---

## Algorithm

Numbered normative requirements. **MUST** / **MUST NOT** / **REFUSE** (a defined refusal carrying an exit
code or status token) / **SHOULD**.

### A. Canonicalize (GR §2)

**R1 (MUST).** The canonical form of a gate report is its **RFC 8785 JSON Canonicalization Scheme (JCS)**
serialization, restricted by the value profile of GR §2.2 (GR §2.1).

**R2 (MUST).** Under that profile, JCS reduces to exactly this, verbatim (GR §2.2):
> sort each object's members by member-name bytes, ascending; emit with no whitespace; emit integers in plain
> decimal; emit strings with JSON's minimal escaping (`"` → `\"`, `\` → `\\`, nothing else can occur); output UTF-8.

**R3 (MUST NOT).** No whitespace, no indentation, no pretty form anywhere in the canonical bytes. GR §8.2:
*"Shown pretty-printed for reading. **The pretty form is not canonical.**"*

**R4 (MUST).** Key ordering is JCS's — ascending by member-name **bytes**. "Never insertion order, never a
hand-written order." (GR §7 rule 4). Because member names match `^[a-z][a-z0-9_ ]*$`-class ASCII, JCS's
UTF-16 code-unit ordering *reduces to* byte ordering (GR §2.2) — the implementation MUST NOT rely on that
reduction for member names outside the profile.

**R5 (MUST).** Numbers are integers in `[0, 2^53 − 1]`, plain decimal (GR §7 rule 7, §2.2). **MUST NOT**
emit a sign, a leading zero, a fraction, an exponent or `-0`. "There is no floating-point value anywhere in
a gate report." (GR §2.2)

**R6 (MUST NOT).** Emit `null`. "An absent value is an absent member." An optional member is present or
absent, and absence always means *this concept does not apply to this landing*, never "unknown" and never
"empty" (GR §2.2, §7 rule 6).

**R7 (MUST).** Every array whose semantics is "the set of X" is emitted even when empty; `[]` is a value, not
an absence (GR §7 rule 5). Array order is fixed per field by GR §5 and §6; JCS preserves it (GR §2.2).

**R8 (REFUSE).** Duplicate member names are invalid: "A parser that meets one refuses the document."
(GR §2.2). For the manifest the token is `manifest-duplicate-member` (MF §2.2).

**R9 (MUST).** Digest and framing, verbatim (GR §2.1):
> **Digest.** `report=sha256:<hex>`, lowercase, 64 hex digits, over exactly the canonical bytes. No trailing
> newline, no BOM, no framing. A file holding a report contains exactly the canonical bytes and nothing else,
> so `sha256sum` over the file reproduces `report=`.

**R10 (SHOULD, non-normative aid).** GR §2.2's implementation note: for this profile,
`json.dumps(obj, sort_keys=True, separators=(',',':'), ensure_ascii=False).encode('utf-8')` is byte-identical
to JCS. *"It is **not** JCS in general — floats and non-BMP member names diverge — which is exactly why the
profile exists."* A Rust implementation MUST therefore either implement RFC 8785 properly or enforce the
profile at the type level; a serializer that merely sorts keys is conforming **only** inside the profile.

**R11 (MUST NOT).** Normalize anything. GR §2.3: *"**Nothing is ever normalized.** No NFC, no NFD, no case
folding, no separator rewriting."* Where a gate itself casefolds (G14 before floor comparison, PB §7.3) "the
report records the path **as the diff produced it**, not the casefolded form."

### B. `esc` (GR §2.3)

**R12 (MUST).** Every value in a gate report that carries repository bytes or human bytes — *"paths, trailer
lines, patterns, principals"* — is encoded with `esc` and is thereafter pure ASCII (GR §2.3, §7 rule 8).

**R13 (MUST).** `esc(s)`, for a byte string `s`, emits for each byte `b` (table verbatim, GR §2.3):

| `b` | emits |
|---|---|
| `0x5C` (`\`) | the two characters `\` `\` |
| `0x20 … 0x7E`, other than `0x5C` | the character with that code point |
| anything else (`0x00–0x1F`, `0x7F–0xFF`) | the four characters `\` `x` and two **lowercase** hex digits of `b` |

**R14 (MUST).** *"The result is a character string over `U+0020…U+007E`, which the JSON layer then escapes
normally (`"` → `\"`, `\` → `\\`)."* — i.e. `esc` runs **before** JSON escaping, and the two layers compose
(GR §2.3). MF §2.3 restates: *"`esc` is applied **once**, to the raw bytes, before the JSON layer's own escaping."*

**R15 (MUST / REFUSE).** Decoding is total and unambiguous: *"`\` introduces either `\` (one literal
backslash) or `x` plus exactly two lowercase hex digits (one byte). Any other sequence after `\` is an
invalid report."* (GR §2.3) — uppercase hex digits after `\x` are therefore **not** accepted.

**R16 (MUST).** `esc` is the identity for values already constrained to ASCII — object ids, integers,
booleans, closed enumerations — so an implementation "may apply it uniformly" (DM §2.4; GR §6.1 says the same
for a `G13` wire's oid). MF §2.3 fixes the per-member map for the manifest: `files[].path`, every
`paths.*` value and `params.trunk` are `esc`-encoded; `repo`, `cli.version`, `cli.dist_hash`, every
`templates`/`resign` value, `files[].owner/template/blob/base`, `params.ci`, `params.isolation` and every
`params.langs` element are identity.

**R17 (MUST).** Worked cases, verbatim (GR §2.3) — an implementation MUST reproduce all five:

| Path bytes | `esc` | bytes in the canonical JSON |
|---|---|---|
| `src/shared/util.ts` | `src/shared/util.ts` | `"src/shared/util.ts"` |
| `a\b` | `a\\b` | `"a\\\\b"` |
| `caf` + `0xC3 0xA9` | `caf\xc3\xa9` | `"caf\\xc3\\xa9"` |
| `a"b` | `a"b` | `"a\"b"` |
| `a,b` | `a,b` | `"a,b"` (the comma is only escaped inside a *wire token*, §6.2) |

### C. `tok` (GR §6.2)

**R18 (MUST).** The **wire token** of a `wires[]` entry is `G<n>` when `path` is absent; `G<n>` + `:` +
`tok(path)` otherwise (GR §6.2).

**R19 (MUST).** Verbatim (GR §6.2):
> where `tok(s)` is `esc(s)` with three bytes moved out of the printable row of §2.3 into the `\xHH` row:
> `,` (`0x2C`) → `\x2c`, ` ` (`0x20`) → `\x20`, `"` (`0x22`) → `\x22`. Every other byte follows §2.3
> unchanged, so `tok` is `esc` for every path containing none of the three. `tok` is **one pass** over the
> bytes of `s`, not `esc` composed with a second escaping step: a second pass would re-escape the `\` that the
> first pass emitted and turn `,` into `\\x2c`.

**R20 (MUST NOT).** Escape `=`. Verbatim (GR §6.2): *"`=` is deliberately **not** escaped: a trailer field
splits on its first `=`, so `wires=G2:src/a=b.ts` parses as the field `wires` with the value
`G2:src/a=b.ts`. Three escapes, not four — the same reasoning that justifies the three forbids a fourth."*

**R21 (MUST).** `tok` is used, and `esc` is not, wherever a token is written into a signed line or used as a
sort key: `G1:` + `tok(path)` (RF §8.5, GR §6.3), `G2/G5/G7/G8/G14/G16:` + `tok(path)` (GR §6.3),
`forced=` in `Spine-Upgrade` is `tok(path)` comma-joined with the empty list as the empty value (MF §6.4, R13).

**R22 (MUST).** `tok` is **not** used inside a dump: *"The `tok` variant of `gate-report.md` §6.2 is not used
here … The one attr that carries wire tokens — `approval.wires` — carries them as `tok` produced them,
because those are the bytes the signed line contains"* (DM §2.4).

### D. Orderings (two different keys — do not cross them)

**R23 (MUST).** `gates[]` sorts **by gate number ascending** (GR §5.6): "an array rather than an object
because gate order is numeric and JCS would sort `g1, g10, g11, …, g2` by name." So `G9` precedes `G11`
precedes `G12` (EV §7 rule 12).

**R24 (MUST).** `wires[]` and a `Spine-Review`'s `wires=` sort **ascending by unsigned byte value over the
whole wire token** — PB §11's `Spine-Review` row is the source: *"ascending by unsigned byte value over the
whole token, so `G11` precedes `G2`; a set with no order is a signature two runs spell differently"*.
Consequences fixed by GR §6.1: `G11` precedes `G2` (`0x31 < 0x32` at the second byte), `G1` precedes `G11`,
and within one gate the pathless entry precedes every `:`-suffixed one because its token is a proper prefix
of theirs.

**R25 (MUST).** *"**The sort key is the token's bytes**, which for a path-bearing entry means `tok(path)` and
not `esc(path)`: the two differ on `,`, ` ` and `"` (§6.2), and sorting the array on one key while the line
is written under the other produces a `wires=` whose order does not match the array's over the same
findings."* (GR §6.1). One comparator writes both: "the line is the array's tokens joined by `,` and nothing
has to be re-sorted to write it" (GR §6.2).

**R26 (REFUSE-adjacent, non-conforming).** A **numeric** wire sort — `G2:src/shared/util.ts,G11` — is
**non-conforming** (GR §6.2). GR §9.19 withdrew the numeric reading by name. Diagnostic that MUST be known
(GR §8.2.1): re-sorting is a permutation, so *"every length check in this document passes under both orders
and only the digests separate them"* — an implementation matching all published lengths and no digest has a
wrong wire comparator, not a wrong canonicalizer.

**R27 (MUST).** `floor_hits` is `esc`-encoded paths, deduplicated, **sorted ascending by encoded bytes**
(GR §5.7; producer MF §5.10: `floor_hits := sort_unique([esc(d) for d in hits])`). Note the key here is
`esc` bytes, not `tok` bytes — the `G14` wire derived from the same hit sorts by `tok` (R25).

**R28 (MUST).** `policy.floor_extensions` is `esc`-encoded, deduplicated, sorted ascending by encoded bytes;
a list-valued `paths.*` key contributes one entry per element, flattened (GR §5.4).

**R29 (MUST).** `reopens` and `reviews` are ancestor-first: "the order the commits appear in
`git rev-list --reverse --first-parent <objects.base>..<objects.head>`, extended past `head` to the literal
ref tip `H` for review commits" (GR §5.5.1).

**R30 (MUST).** `automerge.preconditions` is five entries, **`id` ascending** 0..4 (GR §5.8).

**R31 (MUST).** `esc`-byte order is **not** raw-byte order, and the difference is load-bearing: `esc` maps
every byte above `0x7E` into a sequence beginning with `\` (`0x5C`), which sorts *below* every lowercase
letter, so `src/\xe9.py` sorts before `src/z.py` while the raw bytes sort the other way (DM §6.4).

**R32 (MUST).** The `freeze=` / trailer-line sort is a **third** key and MUST NOT be confused with the other
two: *"Ascending by unsigned byte value, over the entire line, `memcmp` order, shorter-is-smaller on a prefix
tie."* — not locale collation, and explicitly *"**Not** the `esc` order that `dump.md` §6.4 uses"* (EV §4.2).
`LC_ALL=C sort`; in Rust, `&[u8]` comparison.

### E. Digests over non-JSON artifacts

**R33 (MUST).** `envelope=` — verbatim (EV §3.1):
> `envelope=sha256:<hex>`, where `<hex>` is 64 lowercase hex digits of the SHA-256 of the byte string formed
> by taking, **in message order**, every `Spine-*` line (§2.3) that appears **above the `Spine-Seal` line**,
> and joining them with a single `0x0A` between consecutive lines — **no separator before the first, and none
> after the last**.

**R34 (MUST).** The function is total and MUST NOT be special-cased: the digest of an empty sequence is the
SHA-256 of the empty string, `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (EV §3.1).

**R35 (MUST).** `freeze=` — verbatim (EV §4.1):
> `freeze=sha256:<hex>`, where `<hex>` is 64 lowercase hex digits of the SHA-256 of the byte string formed by
> taking **every `Spine-Frozen` and every `Spine-Test` line** of the commit in question, **each line entire —
> trailer name, `: `, and payload — and excluding its terminating `0x0A`**, sorting them ascending by §4.2's
> comparison, and joining them with a single `0x0A` between consecutive lines, **with no trailing `0x0A`**.

The whole line is hashed, not the payload — *"it removes any need to unquote a path before hashing, so the
digest cannot diverge on a quoting disagreement — the quoting is hashed, not the decoded bytes"* (EV §4.1).
Interleaving is a *consequence*, not a rule: `F` (`0x46`) precedes `T` (`0x54`), so every `Spine-Frozen`
line precedes every `Spine-Test` line, and an implementation **MUST NOT** encode that as a separate rule
(EV §4.2).

**R36 (MUST).** `dist_hash` is `"sha256:"` + 64 lowercase hex over the release artifact list, whose format is
*"`sha256sum` bytes, one artifact per target, sorted by artifact name"* (MF §8.2, adopting CI §5.5).
`tool.version + "+" + tool.dist_hash` is exactly the seal's `tool=` field (GR §5.3).

**R37 (MUST).** The dump digest is `sha256:` + 64 lowercase hex "over exactly the byte stream of §2.2 —
including the final LF, excluding nothing" (DM §2.5), and **MUST NOT** be sealed, signed, made a trailer
field, or made a member of a gate report (DM §2.5).

### F. Git object handling

**R38 (MUST).** The gate report *"is **not** a git object. It is a non-git artifact, so its digest is
`sha256:<hex>` per PB §11's hash policy. Everything it *names* that is a git object is named by object id."*
(GR §1)

**R39 (MUST).** `object_format` (from the manifest at `base`) fixes oid length: 40 or 64 lowercase hex
(GR §5). Every oid is full-length, lowercase, unabbreviated, unprefixed (GR §7 rule 9).

**R40 (MUST).** `objects.tree` is `T := git merge-tree --write-tree B Hc` — the tree the gates evaluated —
**not** `L`'s tree; on a tombstone it is `B`'s tree (GR §5.2, §9.2). `objects.head` is the **content head**
`Hc`, never the literal ref tip (GR §5.2). `objects.merge_base` is `git merge-base <base> <head>` (GR §5.2).

**R41 (MUST).** The git version parse is normative *"because a mis-parse forks both the digest and §3.3's
`wrong-git` check"* — verbatim (GR §5.3):
> Over `git --version`'s output: take the first maximal run of ASCII digits, then the first maximal run of
> ASCII digits following the next `.`; record the two joined by `.`. `git version 2.39.5 (Apple Git-154)` →
> `"2.39"`; `git version 2.45.1.windows.2` → `"2.45"`; `git version 2.46.GIT` → `"2.46"`. Output from which
> two such runs cannot be read is a refusal: no report is produced, and `--verify` exits 3 `wrong-git`.

**R42 (MUST).** Publication target — verbatim table (GR §4.4.1): **Ref** `refs/notes/spine`, *"written in
full. The porcelain shorthand is `--ref=spine`; the ref name is normative and no other notes ref carries a
gate report."* **Annotated object** *"the **landing commit `L`** … Never the tree, never `Hc`, never
`objects.tree`, never an envelope blob. One landing, one note."* **Note content** *"exactly the canonical
bytes of §2 for the report that landing's seal names — the same bytes `report=` is a SHA-256 of. No trailing
newline, no BOM, no framing, no pretty-printing, no header, no signature, nothing appended."*

**R43 (MUST).** The consequence is the test (GR §4.4.1, verbatim):
```
git cat-file blob $(git notes --ref=spine list <L> | cut -d' ' -f1) | sha256sum
```
*"reproduces the hex of that landing's `report=`. A publisher whose note fails this has not published the
report, whatever it wrote."*

**R44 (MUST).** The write path, verbatim (GR §4.4.2):
```
blob=$(printf '%s' "$canonical" | git hash-object -w --stdin)
git notes --ref=spine add -C "$blob" <L>
git push origin refs/notes/spine
```
**MUST NOT** use `-m`, `-F` or the editor paths: *"`-m`, `-F` and the editor paths are **non-conforming**: git
terminates a note message with a newline, and a note carrying one trailing `0x0A` hashes to something that is
not `report=`. `-C <blob>` reuses the object's bytes verbatim, which is the only write path this document
admits."*

**R45 (MUST).** Ordering: after the CAS of PB §5.4 step 6 has made `L` trunk's tip, "and never before"
(GR §4.4.2; CI §6.5). **MUST NOT** retract the landing on a failed publish: a failed note push **fails the CI
job** and "changes nothing about `L`, its seal, or the ledger" (GR §4.4.2).

**R46 (MUST / REFUSE).** Republication is idempotent; overwriting is refused: re-publishing byte-identical
content is a no-op; publishing *different* content for a commit that already carries a note is **refused**.
`git notes ... add -f`, `append`, `edit` and `remove` "are never part of publication" (GR §4.4.2).

**R47 (MUST NOT).** Push `refs/notes/spine` with `--force`. A rejected non-fast-forward push is answered by
fetching the ref, re-applying this landing's note to the refreshed ref, and retrying — bounded (GR §4.4.2).

**R48 (MUST NOT).** Fetch or read a note during a gate run. *"Spine does not install that configuration, does
not fetch it implicitly during a gate run, and never fetches it during `spine check --land` or `--ci`"*
(GR §4.4.5). A clone that wants `--verify` fetches explicitly:
`git fetch origin '+refs/notes/spine:refs/notes/spine'` (GR §4.4.5).

**R49 (MUST NOT).** Read anything from a note as a fact: *"No gate reads a note … The ledger derives from
commits alone … `--verify` reads a note and believes nothing in it"* (GR §4.4.6). A note commit's
author/committer date is read by nothing (GR §4.4.3).

**R50 (MUST).** `Spine-Frozen` path quoting is `git ls-tree` C-style quoting and is a **fourth** encoding that
MUST NOT be unified with `esc` or `tok` (EV §2.5, §4.3, §13.9). The rendering "must not depend on
`core.quotePath`". Rule, verbatim (EV §4.3): the path is quoted — wrapped in `"` … `"` with escapes — **iff**
it contains at least one byte in `0x00–0x1F`, `0x7F–0xFF`, `"` (`0x22`) or `\` (`0x5C`); otherwise emitted
literally, unwrapped. Inside a quoted path:

| Byte | Emits |
|---|---|
| `0x07` | `\a` |
| `0x08` | `\b` |
| `0x09` | `\t` |
| `0x0A` | `\n` |
| `0x0B` | `\v` |
| `0x0C` | `\f` |
| `0x0D` | `\r` |
| `0x22` | `\"` |
| `0x5C` | `\\` |
| any other byte in `0x00–0x1F` or `0x7F–0xFF` | `\` + exactly three octal digits, zero-padded, lowercase-irrelevant (`\001`, `\177`, `\303`) |
| any other byte in `0x20–0x7E` | itself |

A space does **not** trigger quoting; the payload splits at its **first** space. "Deciding whether the path
field is quoted is exact: it is quoted iff its first byte is `"`." A field beginning with `"` that is not a
valid C-quoted string is `envelope-malformed` (EV §4.3).

**R51 (MUST).** The same path therefore has two spellings inside one commit message and both are required
(EV §2.5): `tests/fixtures/café.json` is `"tests/fixtures/caf\303\251.json"` in a `Spine-Frozen` line and
`tests/fixtures/caf\xc3\xa9.json` inside a `G8:` wire token.

### G. Determinism rules that bind the primitive layer (GR §7)

**R52 (MUST NOT).** Hold a time, a duration, a date, or anything derived from one, in any member (GR §7 rule
1; PB §7.5 "one clock and it is the chain"). `params.timeout` is a duration and is therefore not a member.

**R53 (MUST NOT).** Record environment: "No hostname, no runner id, no user, no path outside the repository,
no locale, no process id." (GR §7 rule 2)

**R54 (MUST NOT).** Read persisted state: no count of prior runs, no side file, no note read as a source, and
no persisted, fetched or restored graph (GR §7 rule 3).

**R55 (MUST NOT).** Self-reference: *"The report never contains its own digest, and never contains
`envelope=`"* — the circularity runs through the `Spine-Review` lines that carry `report=` and sit inside
`envelope=` (GR §7 rule 11, corrected 2026-08-27; EV §15).

**R56 (MUST NOT).** Apply a size cap to the report: "Only the digest enters the envelope, so PB §5.5's 16 KiB
envelope cap does not apply to the report." (GR §7 rule 12)

### H. Reader / verifier order (GR §3.2, §4.1, §4.3)

**R57 (REFUSE).** A reader that does not know a report's `report_version` **refuses**: status
`report-version-unknown`, exit 3. *"It never partially parses, never ignores unknown members, and never
guesses."* A reader meeting an unknown **member name** inside a known version refuses the same way
(GR §3.2). A binary keeps a parser *and a serializer* for every report version it has ever shipped.

**R58 (MUST).** `--verify` candidate resolution order: (1) `--report <path>` if given; (2) the
`refs/notes/spine` note on `<landing-sha>` if present in this clone; (3) otherwise status
`report-unavailable`, exit 2 (GR §4.1).

**R59 (MUST).** The check order is **normative** — verbatim (GR §4.3): *"The order is normative, because two
implementations that check the same things in a different order report different statuses for a clone that is
wrong in two ways at once"*:
1. the seal's `tool=` against the running binary, and its `git=` against the parsed `git --version` — exit 3, before any candidate is read;
2. resolve a candidate — exit 2 if there is none;
3. `sha256` over the candidate's exact bytes against the seal's `report=` — exit 1 `candidate-mismatch`, **before the candidate is parsed**;
4. parse it; an unknown `report_version` or an unknown member name — exit 3 `report-version-unknown`;
5. recomputability of the objects the evaluation needed — exit 4;
6. rebuild, copy the attested members in, canonicalize, compare — exit 0 or exit 1 `report-mismatch`.

**R60 (MUST).** On `candidate-mismatch`, `--verify` stops and prints both digests; it MUST NOT proceed
(GR §4.1).

### I. Sibling artifact profiles (same scheme, different profile — do not share one struct blindly)

**R61 (MUST).** **Dump** (DM §2.1–§2.5): JSON Lines, each record JCS under DM §2.3's profile with DM §2.4's
`esc`; framing: every line terminated by exactly one `0x0A` **including the last**, "No CR anywhere, no BOM,
no blank lines, no comments, no trailing blank line"; depth exactly two; arrays contain strings only; header
is line 1. Digest covers the final LF (DM §2.5). `esc` is `gate-report.md` §2.3's and is *not* restated:
*"a divergence between the two documents is a defect in `gate-report.md`, which owns it."* (DM §2.4)

**R62 (MUST).** **Manifest** (MF §2.1–§2.4): JCS under a profile that differs from GR §2.2 in exactly two
stated ways — member names match `^[a-z][a-z0-9_-]{0,63}$` (**`-` admitted**, "Wider than GR §2.2 by one
byte"), and **booleans are permitted** ("Not in GR §2.2's table"). Plus resource bounds: file ≤ 1 MiB, any
array ≤ 4096 elements, any string ≤ 8192 bytes after `esc`, ≤ 256 members in any object
(`manifest-too-large`). File bytes are `JCS(value) ++ 0x0A` — **exactly one trailing LF**, the opposite of the
report's rule, and MF §2.4 states why: *"the report is a digest input and never a tracked file, while the
manifest is a tracked file under `.gitattributes`'s `.spine/** text eol=lf`"*. The recorded blob id is the git
blob id of those bytes.

**R63 (MUST).** **Result file** (RF §4.3): a *different* canonical-JSON profile — "RFC 8785-compatible over
the value space this file uses", but with its own string rule, verbatim: *"Strings: `"` → `\"`, `\` → `\\`,
`U+0008` → `\b`, `U+0009` → `\t`, `U+000A` → `\n`, `U+000C` → `\f`, `U+000D` → `\r`; every other code point
below `U+0020` → `\u00xx` with **lowercase** hex; every other code point emitted literally as UTF-8. No other
escape is produced and none is accepted."* No `esc`, no numbers, no booleans, no null, no nested objects, no
arrays. Canonical form is required **on read** as well as on write; a non-canonical body line is malformed.
Ordering: header, `base` records sorted by bytes of `runner` then bytes of `id`, `result` records likewise,
then exactly one `end` record (RF §4.5).

**R64 (MUST).** Therefore an implementation MUST carry **three** JSON canonicalizers-or-one-parameterized-by-
profile (report/dump, manifest, result file) and **four** byte-encodings (`esc`, `tok`, `git ls-tree` quoting,
result-file JSON escaping), and MUST NOT unify any pair. EV §13.9: *"an implementation that reuses one encoder
for both produces lines no conforming implementation reproduces."*

---

## Byte-level fixities (verbatim)

1. **JCS reduction** — GR §2.2: *"sort each object's members by member-name bytes, ascending; emit with no
   whitespace; emit integers in plain decimal; emit strings with JSON's minimal escaping (`"` → `\"`,
   `\` → `\\`, nothing else can occur); output UTF-8."*
2. **Report framing** — GR §2.1: *"No trailing newline, no BOM, no framing."*
3. **Hash policy** — PB §11 (the whole paragraph): *"Git object ids (`<oid>`, in the repo's object format) for
   everything that is a git object: intent blob, frozen files, trees, commits. SHA-256 (`sha256:<hex>`) only
   for non-git artifacts: release artifact list (`dist_hash`), gate report, freeze digest, envelope digest,
   B's transcript."*
4. **Non-git digests** — GR §7 rule 10: *"`"sha256:"` + 64 lowercase hex (PB §11 hash policy). Never bare hex,
   never uppercase, never another algorithm."*
5. **Object ids** — GR §7 rule 9: *"lowercase hex at the full length `object_format` implies — 40 or 64
   digits. Never abbreviated, never uppercase, never prefixed. The playbook's `9f2c…` is display, not a
   value."*
6. **`esc` hex case** — GR §2.3: *"the four characters `\` `x` and two **lowercase** hex digits of `b`"*.
7. **`tok`'s three escapes** — GR §6.2: *"`,` (`0x2C`) → `\x2c`, ` ` (`0x20`) → `\x20`, `"` (`0x22`) →
   `\x22`"*, one pass, `=` never escaped.
8. **Wire order** — PB §11: *"ascending by unsigned byte value over the whole token, so `G11` precedes `G2`;
   a set with no order is a signature two runs spell differently"*.
9. **Gate order** — GR §5.6: `gates[]` "sorts by gate number ascending"; EV §7 rule 12: *"so `G9` precedes
   `G11` precedes `G12`"*.
10. **Trailer-line / freeze sort** — EV §4.2: *"Ascending by unsigned byte value, over the entire line,
    `memcmp` order, shorter-is-smaller on a prefix tie."*
11. **Envelope join** — EV §3.1: LF between consecutive lines, *"no separator before the first, and none after
    the last"*.
12. **Freeze join** — EV §4.1: whole lines, *"excluding its terminating `0x0A`"*, joined by single `0x0A`,
    *"with no trailing `0x0A`"*.
13. **Note content** — GR §4.4.1: *"No trailing newline, no BOM, no framing, no pretty-printing, no header, no
    signature, nothing appended."*
14. **Note write path** — GR §4.4.2: `git hash-object -w --stdin` then `git notes --ref=spine add -C "$blob"`;
    *"`-m`, `-F` and the editor paths are **non-conforming**"*.
15. **Manifest file bytes** — MF §2.4: `file bytes := JCS(value) ++ 0x0A`; *"Exactly one trailing `0x0A`, no
    other `0x0A` anywhere, no `0x0D` anywhere, no BOM."*
16. **Dump framing** — DM §2.2: *"each terminated by exactly one `0x0A` (LF). The final line is terminated too,
    so the stream ends with `0x0A`. No CR anywhere, no BOM, no blank lines, no comments, no trailing blank
    line."*
17. **`ls-tree` quoting** — EV §4.3 table (reproduced at R50), octal escapes exactly three digits.
18. **Result-file string escaping** — RF §4.3 clause 3 (reproduced at R63), `\u00xx` lowercase hex.
19. **Empty-sequence SHA-256** — EV §3.1: `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
20. **`git --version` parse** — GR §5.3 (reproduced at R41).

---

## Error cases

| Condition | Behaviour | Exit / status token / message |
|---|---|---|
| `report_version` unknown to the reader | refuse; never partial-parse (GR §3.2) | exit **3**, status `report-version-unknown` |
| unknown **member name** inside a known version | refuse identically (GR §3.2, §4.3 step 4) | exit **3**, `report-version-unknown` |
| duplicate member name in a report | the parser **refuses the document** (GR §2.2) | (no token fixed in GR — manifest's is `manifest-duplicate-member`, MF §2.2) |
| invalid `\` sequence in an `esc` string (not `\\`, not `\x` + two lowercase hex) | *"Any other sequence after `\` is an invalid report."* (GR §2.3) | refusal; no token fixed |
| running binary's platform artifact hash ≠ seal's `tool=` `dist_hash` | refuse, print the release to install (GR §3.3) | exit **3**, `wrong-release` |
| parsed `git --version` ≠ seal's `git=` | refuse (GR §3.3) — a requirement, not a warning | exit **3**, `wrong-git` |
| `git --version` output from which two digit runs cannot be read | *"no report is produced"* (GR §5.3) | exit **3**, `wrong-git` |
| no `--report` and no note on the landing in this clone | (GR §4.1, §4.4.4) | exit **2**, `report-unavailable` |
| candidate's bytes do not hash to the seal's `report=` | stop **before parsing**, print both digests (GR §4.1) | exit **1**, `candidate-mismatch` |
| candidate was the sealed report and recomputation disagrees | print the recomputed report (GR §4.3) | exit **1**, `report-mismatch` |
| `objects.head` unreachable and the evaluation needed it (a `land` under squash) | (GR §4.2, §4.3) | exit **4**, `not-recomputable` |
| recomputed digest equals the seal's `report=` | success | exit **0**, `verified` |
| a run would seal a report containing any `fail` | refuse (GR §5.6.1) | status `report-not-landable` (no exit code fixed) |
| note push fails | landing complete; **fail the CI job**, do not re-queue, retract nothing (GR §4.4.2; CI §6.5) | ci.sh exit **5**, `note-publish-failed` |
| a second, *different* report published for a commit that already carries a note | publication **refused**; a repository holding two distinct reports for one landing "has a finding for a human, not a merge to perform" (GR §4.4.2) | — |
| non-fast-forward push of `refs/notes/spine` | fetch, re-apply, retry, bounded; never `--force` (GR §4.4.2) | — |
| manifest parses but is not canonical | G16 check 3 (MF §2.4, §6.2) | `manifest-noncanonical` |
| manifest exceeds a resource bound | (MF §2.2) | `manifest-too-large` |
| manifest member name outside `^[a-z][a-z0-9_-]{0,63}$`, or `trunk`/`dist_hash` used outside `params.trunk`/`cli.dist_hash` | G16 check 5 (MF §6, §3.10) | `member-name-out-of-grammar`, `reserved-member-name` |
| `Spine-Frozen` path field begins with `"` and is not a valid C-quoted string (unterminated, bad escape, trailing byte after the closing quote) | (EV §4.3) | `envelope-malformed` |
| result-file body line parses as JSON but is not in RF §4.3 canonical form | malformed → G1 finding `result-malformed` (RF §4.3, §8.2) | `result-malformed` (a G1 finding, not a run-ender) |
| result-file id whose bytes are not valid UTF-8 | that runner contributes **no** `result` records; fold makes the file's `status` `stream-invalid` (RF §7.2, §7.3) | `stream-invalid` |
| `tree=`/`base=` in the ingested header ≠ this run's `T`/`B` | ends the run **before** a report exists (GR §5.9; RF §8.3 step 1) | `base-moved` |
| collector `tool=` ≠ base's pin | a **G15** failure, never a G1 finding, never overridable (GR §5.9) | — |

---

## Worked examples / test vectors

### V1 — GR §8.3, the minimal canonicalizer vector (build against this FIRST)

Verbatim from GR §8.3:

```
value:     {"b":[1,2],"a":"x\\y","Z":true,"_c":{"n":0,"m":"q\"r"}}
canonical: {"Z":true,"_c":{"m":"q\"r","n":0},"a":"x\\y","b":[1,2]}
digest:    sha256:a594772ccb6408158b6e76b170d5488c2454ba576e09ae379e24d743e21921f0
```

GR §8.3: *"Debug your canonicalizer against this before attempting §8.2. It exercises member ordering across
case and underscore, a nested object, an array of integers, a JSON-escaped quote and a JSON-escaped backslash
— and nothing else."* And: *"(The member names `Z` and `_c` are outside §2.2's `^[a-z][a-z0-9_]*$` and appear
in this vector only to pin ordering behaviour. A real gate report never uses them.)"*

**Reproduced in-session:** canonical length **55 bytes**; SHA-256 =
`a594772ccb6408158b6e76b170d5488c2454ba576e09ae379e24d743e21921f0`. Parsing the `value:` line and
canonicalizing yields the `canonical:` line character for character. (Length 55 is my measurement — GR does
not publish a length for §8.3.)

Ordering facts this vector pins: `Z` (`0x5A`) < `_c` (`0x5F`) < `a` (`0x61`) < `b` (`0x62`) — i.e. uppercase
before underscore before lowercase, which is byte order and **not** case-insensitive order.

### V2 — GR §8.1, evaluation 1 (the report a reviewer signs)

```
canonical length = 3476 bytes
report           = sha256:e2bd8cb5da473701bf0dd8e9aa0c6a28cc1280e33ba06758165c5089ae3f5b47
```
Derived from §8.2's printed value by exactly two members: `authority.reviews: []` and
`gates[G2].status: "fail"` (GR §8.1, §8.2.1). **Reproduced in-session: 3476 bytes, digest matches.**

### V3 — GR §8.2, evaluation 2 (the sealed report)

```
canonical length = 4053 bytes
report           = sha256:a47c13281b0b4a13085e523cd3c09e4f7d1caf0c80419de5fa3a869f7aec309e
first 96 canonical bytes:
{"authority":{"approve":{"fingerprint":"SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM","lin
```
**Reproduced in-session: 4053 bytes, digest matches, first 96 bytes match.**

Ordering dependency (GR §8, the numbered recipe): evaluation 1 must be canonicalized and digested **first**,
because its digest is substituted into bob's `report=` inside evaluation 2 — though GR §8.2 notes the printed
value *already* carries it, so §8.2 canonicalizes as printed.

The `Spine-Gates` rendering of §8.2's `gates` array (GR §8.2):
```
Spine-Gates: G1=pass G2=override G3=pass G4=pass G5=pass G7=pass G8=pass G9=pass G11=pass G12=pass G13=pass G14=pass G15=pass G16=pass
```

The signed wire line in the same landing (GR §8.1, §8.2; PB §5.5; EV §8.3), in PB §11's byte order:
`wires=G11,G2:src/shared/util.ts`.

Backing decision (c) (`template=intent@2` → `template=v2`) out of both values reproduces **3470** and **4047**
— six less than each (GR §8.2.1). Useful as a differential check while debugging.

### V4 — MF §8.2, `dist_hash` from a printed 529-byte artifact list

```
f0ed236cfc75bb491003477b77cbd09b76f80420f546b585e2a16ee85ad989ae  spine-1.4.0-aarch64-apple-darwin.tar.gz
ce946375b5e89e3e5546d7563ef8a539c5c62828125c851220edf74578dfb167  spine-1.4.0-aarch64-unknown-linux-musl.tar.gz
40627734cff1df388697c03a037273fb6693cfa5ba594e4cbf85db44ef626bbb  spine-1.4.0-py3-none-any.whl
2d90a2ef987219f1df0ac40b08fd853156b0500e3f31177a1bd701bc4f618977  spine-1.4.0-x86_64-apple-darwin.tar.gz
48f5f6e485b72cc4e848a488256435ffcb6025c0f401ae211136d8c34577c1ec  spine-1.4.0-x86_64-unknown-linux-musl.tar.gz
```
`529 bytes`, `sha256 = 6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db`, so
`cli.dist_hash = "sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db"` (MF §8.2).
**Reproduced in-session (529 bytes, digest matches).** Two `sha256sum`-format spaces between digest and name.

### V5 — the corpus's published-digest index (`docs/spec/README.md`)

| Where | Published value |
|---|---|
| `dump.md` §12.3 — the dump vector | 62 lines, 14054 bytes, `sha256:3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da` |
| `gate-report.md` §8.1 | 3476 bytes, `sha256:e2bd8cb5…5b47` (full above) |
| `gate-report.md` §8.2 | 4053 bytes, `sha256:a47c1328…309e` (full above) |
| `gate-report.md` §8.3 | `sha256:a594772c…` — *"Untouched since publication. Build against it first."* |
| `manifest.md` §8.3 — the manifest blob | 1762/1763 bytes, git blob id `cb4cd49034bbe25f76573c40d6711b2c33f9136f` |
| `ci.md` §5.3 — `.spine/ci.sh` | 319 lines, `git hash-object` = `131f13fb0312162579605999d3f9f4e90098c74c`, SHA-256 = `d6bcf50cf675614033aaef61df104aad253d30c4accc756719599ad5bd41060b` (CI §5.3; README truncates it to `sha256:d6bcf50c…`) |
| `envelope-vectors.md` §8 — vector A | `freeze=` **573 join bytes**, `envelope=` **2379 join bytes**, envelope size 4031/16384 (vector D: 4032) |

### V6 — the stated EV↔GR divergence (EV §15) — expect exactly three differences and no others

| EV §8 prints (fabricated) | Owner's computed value |
|---|---|
| `report=sha256:b2f4c60e1a97d385c0b64e2f79a1d08c3e5b7f92a4160d8ce73b295f0a4d6e18` (review line) | `sha256:e2bd8cb5da473701bf0dd8e9aa0c6a28cc1280e33ba06758165c5089ae3f5b47` (GR §8.1) |
| `report=sha256:e70a3c92d1b845f6027e9ab3c5d10f684a2b7e93c60d5f81a34b0e29d7c6f105` (seal) | `sha256:a47c13281b0b4a13085e523cd3c09e4f7d1caf0c80419de5fa3a869f7aec309e` (GR §8.2) |
| `tool=1.4.0+sha256:41d0e9b7c2a6538f10bd47e29c05a3f6b81d24e70c9a5b3f68d1027ae4c95b3d` (all five seals) | `sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db` (MF §8.2) |

All three are 64 lowercase hex, so **no byte count in EV §8 moves**; the reason they stand is that they sit
inside signed lines whose private keys EV does not publish (EV §15). *"Do not 'fix' this by editing the three
fields in place."*

### V7 — `esc` / ordering micro-vectors worth unit-testing (DM §6.4, GR §2.3)

- raw-byte order: `src/z.py` before `src/` + `0xE9` + `.py`; `esc`-byte order: `src/\xe9.py` **first**
  (`0x5C < 0x7A`) — DM §6.4.
- `AC-10` precedes `AC-2`; `G11` precedes `G2` — byte order, never numeric (DM §6.4; PB §11).
- `esc("a\\b") = "a\\\\b"`, JSON `"a\\\\b"`; `esc(b"caf\xc3\xa9") = "caf\\xc3\\xa9"` — GR §2.3.
- `tok` vs `esc` differ only on `,`, ` `, `"` — GR §6.2.

---

## Cross-references it depends on (which other sheet owns what)

| Topic | Owner |
|---|---|
| The report **schema** — every member, presence condition, R/A marking, `automerge`, `evidence`, `gates`, `wires` semantics | GR §5, §6.1, §6.3 → the gate-report schema sheet. This sheet owns only how those values are *encoded, ordered and digested*. |
| Gate semantics (what G2 containment means, the freeze closure, how G14 casefolds) | PB §6.3, PB §4.3, MF §5/§6, GR §11 |
| The envelope grammar: trailer syntax, field order, `reason=` JSON literals, the fenced intent block, the 16 KiB cap, `-Sig` payloads | EV §2, §3, §4 → the envelope sheet. EV adopts `esc` and `tok` from GR verbatim and may not re-derive them (EV §17). |
| The result file: header, records, ingestion order, outcome enum, its own canonical JSON | RF → the result-file sheet |
| `.spine/manifest.json` and `.spine/allowed_signers`, G13/G14/G16 algorithms, `dist_hash` provenance | MF → the manifest sheet |
| The constitution parser whose output lands in `policy.rules` (list splitting, whitespace, yield order) | CN — *"normative, not decorative"* (GR §5.4.1) |
| Per-language resolvers, `id → fn`, `id → path`, the pragma→id join | IR §7, §11, §12 — *"normative, not decorative"* (GR §5.4.2) |
| Dump format and G10's comparison | DM — reuses this sheet's `esc` and JCS, adds JSONL framing |
| Who pushes the note, how the push is retried, the two-job contract | CI §6.5, §14; CI must adopt GR §4.4.1–§4.4.2 verbatim (GR §11) |
| `spine stats` counters, `spine review` diagnostics, metrics | out of scope everywhere (GR §11) |

---

## OPEN items (undecided; do not invent)

Nothing in this concern area is OPEN. Recording the state explicitly so an implementer does not go looking:

1. **GR has 0 OPEN** (README status table; GR §10): OPEN-1 (a `report` pin in the manifest) — **decided: no
   pin**; OPEN-2 (must CI publish the report) — **decided: it must, on every landing**; OPEN-3 (staleness as a
   constitution rule) — **decided: no**. *"None is re-opened by implementation experience alone."* (GR §10)
2. **EV OPEN-1 and OPEN-3 are closed** by the owner 2026-08-26 (subject derived and G9-checked; the 16 KiB cap
   does not apply to a reseal). **EV OPEN-2 remains open** — the *event commits'* message shape (sign-off,
   approval, review, reopen, withdrawal, upgrade). It touches no digest: *"Nothing reads the subject of a
   sign-off … commit, and no digest covers one — `freeze=` reads only the manifest lines, G13 only the signed
   lines."* (EV §16). Recommendation there is `<ID>: <event>` / `<event>`, to be fixed in another document.
   **Do not invent a subject form for event commits in the primitive layer.**
3. **The EV§8 ↔ GR§8 three-value divergence (V6) is disclosed, not open** — it is a known, stated
   inconsistency awaiting a keyring regeneration, not an owner question (EV §15).
4. **A residual with no owner, adjacent to this sheet:** non-UTF-8 **paths** in a result file. RF §4.3 has no
   `esc` and emits "every other code point … literally as UTF-8"; RF §7.2 fixes the rule for non-UTF-8 **ids**
   (`stream-invalid`, unsupported in v1, §12) but I found no clause fixing non-UTF-8 `path` bytes. Do not
   invent one — the encoding for such a path in a *wire token* is fixed (`tok`), and its encoding in the
   *result file* is not.

---

## Contradictions found

**C1 — `wires[]` order: GR §6.2/§9.19 (historical) vs PB §11. Resolved, in PB's favour, and the corpus is now
consistent.** GR §9.19 is titled *"The order of `wires=` — withdrawn; PB §11 fixes it"* and states the
mistake in terms: *"the entry rested on the premise that 'PB §11 … fixes no order at all' … The premise was
false."* All published GR/EV digests were recomputed under the byte order (GR §9.19; EV §14 D3, WITHDRAWN).
**Implementation consequence:** any third-party artifact or older draft carrying `wires=G2:src/shared/util.ts,G11`
is non-conforming; the byte counts do not distinguish it, only the digests do (GR §8.2.1).

**C2 — GR §7 rule 11's *stated reason* was false until 2026-08-27.** Old wording: *"the seal line that carries
`report=` is inside the envelope digest"*. EV §15: *"The emphasised clause was false … the `Spine-Seal` line
is *below* the seal boundary and is not inside `envelope=`. **The rule was right and its stated reason was
wrong.**"* GR §7 rule 11 now carries EV's wording verbatim. No behaviour changes; a reader of an older copy
would derive the wrong model of `envelope=`.

**C3 — Value profiles that look shared and are not.** GR §2.2 vs MF §2.2: MF admits `-` in member names
(`^[a-z][a-z0-9_-]{0,63}$`) and admits booleans; GR does not. MF flags both as deliberate differences (*"both
stated as differences so nobody assumes the two profiles are the same table"*). GR §2.2 vs RF §4.3: RF escapes
`\b \t \n \f \r` and `\u00xx`, emits non-ASCII literally, and uses **no** `esc`; GR emits minimal escaping over
ASCII-after-`esc` only. **Not a defect — a hazard.** A single shared serializer is only correct if it is
parameterized by profile (R64).

**C4 — Trailing-newline rule inverts between artifacts.** Report: *"No trailing newline"* (GR §2.1);
note content: same (GR §4.4.1); manifest: *"`JCS(value) ++ 0x0A`, exactly one trailing `0x0A`"* (MF §2.4);
dump: every line terminated **including the last** and the digest covers it (DM §2.2, §2.5); `freeze=` and
`envelope=` joins: no trailing LF (EV §3.1, §4.1). MF §2.4 states the rationale for the divergence
(digest input vs tracked file). Reconciled, but it is the single easiest place to lose a digest.

**C5 — Two path encodings inside one commit message, by design.** `Spine-Frozen` uses `git ls-tree` C-quoting
(`"tests/fixtures/caf\303\251.json"`); a wire token uses `tok` (`G8:tests/fixtures/caf\xc3\xa9.json`)
(EV §2.5, §13.9). EV names it *"a genuine hazard"* and refuses to unify: *"each is normative in its own home
and this document has no authority to unify them."* Add `floor_hits`, which sorts and stores `esc(path)` while
its derived `G14` wire token is `tok(path)` (GR §5.7 vs GR §6.3, MF §5.10) — **three encodings of one path can
appear in one landing.**

**C6 — Sort keys that differ per artifact over the same paths.** `esc`-byte order (DM §6.4, GR §5.7
`floor_hits`, MF §7 rule 5) vs whole-line byte order (EV §4.2 `freeze=`) vs whole-token byte order (PB §11,
GR §6.1) vs numeric (`gates[]`, GR §5.6). EV §4.2 explicitly warns off DM's key: *"**Not** the `esc` order that
`dump.md` §6.4 uses … Sorting one thing and hashing another is how a spec grows a second place to disagree."*

**C7 — `gates[]` numeric order is stated in GR §5.6 and restated in EV §7 rule 12 as *"GR §5.6 fixes as
ascending by the integer after `G`"*.** Consistent, but note GR §5.6's own warning that a statement of the
*numeric* order applied to **wires** anywhere is a defect (*"A statement of the numeric order anywhere is a
defect: §9.19 withdrew that reading by name"*) — the grep inventory to maintain is
`grep -n "unsigned byte value" PLAYBOOK.md docs/spec/*.md` (GR §5.6). I ran it; the hits are PB:328, PB:1003,
GR:437/441/655/674/1083, EV:287/452/596/982, DM:334/384, IR:1886, and all of them agree.

**C8 — No contradiction, but a naming trap worth flagging:** GR §5.1's `subject` member (four enumerated
members of the *landing identity*) is a different object from the landing commit's **subject line**, which is
derived, outside `envelope=`, covered by no digest, and recorded in no report member (GR §5.1; PB §11
*Subject lines*; owner decision 6 of 2026-08-26).
