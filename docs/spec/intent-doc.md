# The intent document

**Artifact:** the one-page file `intents/<ID>.md`, the bytes a human signs at PB §3.4's gate, the bytes sealed verbatim into every landing's envelope, and the sole source of the `declares` edges that G2 and G7 evaluate.
**Home in the playbook:** PB §3.1 (the template, `intent@2`), PB §3.2 (why each field), PB §3.3 (the canonical-form rule), PB §3.5 (the three variants); the gates that read the parse are PB §6.3's G2 and G7 and PB §6.2's `declares` row. Vocabulary from PB §11, which wins over prose here as it wins there.
**References:** `PB §n` cites `PLAYBOOK.md`; a bare `§n` cites this document. The two numbering schemes collide — PB §3.3 is the canonical-form rule, §3.3 is variant selection — so every citation says which. `esc` is `gate-report.md` §2.3's; `tok` is its §6.2's; the `code_unit` node id is `dump.md` §5.2's.
**Spec version:** 1 · **Template version specified:** `intent@2`, with the legacy bare `Template: v1` / `Template: v2` spellings as compatibility targets (§3.2, §11.9) · **Covers:** PLAYBOOK.md v0.19 · **Status:** normative for v1, alongside `docs/spec/templates.md`, which owes the Change and Bug section tables (§3.3, §14) and must adopt §4's grammar unchanged.

---

## 1. What this artifact is, and what rests on it

A pre-implementation audit found that the intent document *"has no parse grammar"* while two gates are pure functions of its parse. That is not a documentation gap. It is a correctness gap, and the shape of it is unusual enough to state plainly:

**Another branch's leases are evaluated by my binary.** PB §5.4 derives the lease registry from `refs/heads/intent/*` — *"which a default clone fetches — no service, no side file"* — so when `spine check --land INT-042` runs, it fetches every other in-flight intent branch and parses **their** documents to compute G7. A landing is therefore refused or permitted on the strength of my binary's reading of a document someone else wrote and someone else signed. Two parsers that disagree about where a section ends, about whether `- ` continues a list, or about whether `src/bill` covers `src/billing/x.ts` do not merely render the document differently: they produce different gate verdicts over identical git objects, one binary rejects the other's landings, and PB §1.1's headline — an offline clone that re-verifies — is false.

Four consumers parse this document, and **all four must agree byte for byte**:

| Consumer | Reads | Needs |
|---|---|---|
| `spine new --sign` | the blob at branch head | canonical form, the whole parse, the sign-off preconditions (§8) |
| `spine check --approve` | the blob at the approval tree | the AC set, for *"every AC covered by a collected id"* (PB §6, approve guards) |
| `spine check --land` | the blob in `T`, and every other in-flight branch's blob | the touchpoint sets, for G2 and G7 |
| `spine index` | in flight, the branch blob; landed, the fenced envelope bytes (PB §6.2) | the whole parse, for `intent` and `ac` nodes and `has_ac`, `declares`, `built_under` edges |

The document is a **git object**, so its identity is a git object id, per PB §11's hash policy. Nothing here introduces a `sha256:` digest; §9 publishes one only as an independent check on the worked example's bytes.

**One clock, and it is the chain.** No field of this document is a date, a duration or a version of anything that changes with time. `Constitution: v3` names a constitution version, which is a repository fact derived from git; PB §6.3's G3 measures staleness from committer dates, which are not read here. Nothing in the parse depends on when it runs.

---

## 2. Canonical form

PB §3.3 states the rule in one sentence: *"`spine new` writes the file in canonical form (UTF-8, LF, trailing whitespace stripped, exactly one trailing newline, no Unicode normalisation); `--sign` refuses anything else, refuses any line beginning `-----` or `Spine-` (they would collide with the envelope's own syntax), and hashes the *index* blob (`git hash-object --path`), never worktree bytes — so `core.autocrlf` cannot fork the identity."* This section makes each clause mechanical and adds the two the sentence needs to be total.

### 2.1 The byte rules

A byte string `d` is in **canonical form** iff every rule below holds. Each rule names the status a checker reports; §8.2 fixes the order in which they are checked, so the reported status is determined for a document that breaks several.

| # | Rule | Status on failure |
|---|---|---|
| 1 | `d` is non-empty. | `empty-document` |
| 2 | `len(d) ≤ 65536`. | `document-too-large` |
| 3 | `d` is well-formed UTF-8 (RFC 3629): no overlong forms, no surrogate code points `U+D800…U+DFFF`, no value above `U+10FFFF`. | `not-utf8` |
| 4 | `d` contains no `U+FEFF`, at any position, byte-order mark or not. | `bom` |
| 5 | `d` contains no `0x00`. | `nul-byte` |
| 6 | `d` contains no `0x0D`. Not "no CRLF" — no CR at all, lone or paired. | `cr-byte` |
| 7 | `d` contains no other C0 control and no `0x7F`: every byte below `0x20` is `0x09` or `0x0A`, and `0x7F` never appears. | `control-byte` |
| 8 | `d` ends with `0x0A`, and `d` is not `0x0A` alone and does not end `0x0A 0x0A`. Exactly one trailing newline; no blank line at end of file. | `no-final-newline` / `trailing-blank-line` |
| 9 | No line ends with `0x20` or `0x09`. | `trailing-whitespace` |
| 10 | No line exceeds 4096 bytes. | `line-too-long` |
| 11 | No line begins with the five bytes `-----`. | `fence-collision` |
| 12 | No line's first six bytes, ASCII-lowercased, are `spine-`. | `trailer-collision` |

A **line** is a maximal run of bytes containing no `0x0A`; rule 8 makes the last line the one before the file's final `0x0A`, and there is no line after it. `d` has `k` lines where `k` is the count of `0x0A` in `d`.

**Tabs are permitted inside a line** and are how a continuation line is marked (§4.10). They are not permitted at end of line, by rule 9.

**No normalisation, in either direction.** A checker performs no Unicode normalisation and rejects nothing for being un-normalised. An NFC document and its NFD counterpart are two different documents with two different blob ids and two different signatures. This is `gate-report.md` §2.3's rule for the same reason: a report computed on macOS and one computed in a Linux container must agree, and a normalising step is a place they can differ.

**No casefolding of content.** The parse casefolds exactly three things and nothing else: a section heading's key (§4.7), a touchpoint label (§5.4), and the `Spine-` test of rule 12. Every other byte is compared as written.

### 2.2 The two line refusals, exactly

**`-----` (rule 11).** Case is not a question — the byte is `0x2D`. Five is the count, not "five or more": a line of six hyphens begins with five, so it is refused too. Refused because PB §5.5's envelope delimits the fenced intent with `-----BEGIN SPINE-INTENT blob=… bytes=…-----` and `-----END SPINE-INTENT-----`, and because `ssh-keygen -Y` armour is `-----BEGIN SSH SIGNATURE-----`. A document may not contain a line that a fence scanner could mistake for a boundary, whatever that scanner's exact rule turns out to be — `envelope-vectors.md` fixes it, and this refusal is what makes its choice not matter. The cost is that a Markdown horizontal rule written as five or more hyphens, and a setext `Heading\n-----`, are refused; the remedy is `***` or an ATX heading.

**`Spine-` (rule 12), and it is ASCII case-insensitive.** PB §3.3 writes `Spine-` in the document's own capitalisation. The check is case-insensitive because `git interpret-trailers` matches trailer tokens case-insensitively, so a line reading `SPINE-SEAL: x` or `spine-approve: y` inside the fenced block is a line some reader will treat as a trailer of the landing commit. The refusal covers all 64 spellings. Bytes after the sixth are not examined: `Spine-anything`, and `Spine-` alone, are refused. `Spinel: x` is not (the sixth byte is `l`, not `-`).

Neither refusal has an escape. A document that must discuss the envelope writes `Spine‑Seal` with a non-hyphen, or indents the line, or does not.

### 2.3 Resource bounds, and why they are here rather than in a runtime

Rules 2 and 10, and the item counts of §5, are **normative parse limits**, not implementation advice. The reason is §1's: a document another person wrote, on a branch anyone with push access created, is parsed by my binary during my landing. A 2 GiB `intents/INT-999.md` on a pushed branch must cost my landing a bounded amount of work and then contribute no lease, rather than exhausting the trusted stage.

| Bound | Value | Status |
|---|---|---|
| Document | 65536 bytes | `document-too-large` |
| Line | 4096 bytes | `line-too-long` |
| Sections | fixed by the variant's table (§4.8) — at most 7 at version 2, across the three variants (`templates.md` §4) | `unknown-section`, `duplicate-section` |
| Acceptance criteria | 6 (PB §3.1) | `too-many-acs` |
| Non-goal items | 256 | `too-many-non-goals` |
| Touchpoint patterns, per polarity | 256 | `too-many-touchpoints` |
| Touchpoint pattern | 255 bytes | `pattern-too-long` |

65536 is four times PB §5.5's 16 KiB envelope cap, so it never fires on a document that could land, and it bounds the lease evaluation at 64 KiB per in-flight branch. The parse is single-pass over lines with no backtracking except inside one bracket expression and one segment match (§6.2), both bounded by the 255-byte pattern limit, so the whole parse is linear in the document's length.

### 2.4 Identity: the index blob, hashed with `--path`

The intent's identity is a **git blob id**. Three questions the playbook leaves open, answered:

**Which bytes.** The blob's, never the worktree's. Concretely: the object named by `intents/<ID>.md` in the tree of the branch head at the moment of signing — `git rev-parse HEAD:intents/<ID>.md` — and the canonical-form rules of §2.1 are checked over `git cat-file blob <oid>`, not over the file on disk. This is what PB §3.3 means by *"hashes the index blob … never worktree bytes"*, and it is what makes `blob=` on the `Spine-Signoff` line and the fenced bytes in the envelope the same bytes by construction.

**What `--path` is for.** `git hash-object --path intents/<ID>.md` hashes bytes *as if* they were at that path, applying the `.gitattributes` in effect there. It is the form to use when hashing bytes that are not already at that path — from `--stdin`, from a temporary file, or when reconstructing the blob from an envelope's fenced region. Verified: with `intents/** text eol=lf` in effect, a CRLF-bearing copy of §9.1's document hashes to `1b9e758012b85f788e3b3f16f6e81383bfdc54be`, the canonical LF blob, whereas the same bytes hashed at an unattributed path give `605f55e173f787712e1e5eab34912fcd841a549c`. The attribute, not the checkout, fixes the identity.

**What `--sign` additionally requires.** The worktree file at `intents/<ID>.md`, if it exists, must hash — via `git hash-object --path intents/<ID>.md` — to the head blob's id. A dirty intent path is refused (`worktree-dirty`); signing bytes a human is not looking at is the failure this closes.

The three ids of §9.2 are published for both object formats, because a repository's format is a repository fact (`object_format` in the manifest, PB §6.7) and neither is the default everywhere.

### 2.5 The `.gitattributes` entry the whole rule rests on — and it is wrong in the playbook

**PB §3.3 now writes two lines, and this document's requirement is met by them.** It reads: *"`spine init` writes **two** lines to `.gitattributes` — `.spine/** text eol=lf` and `intents/** text eol=lf`. One line naming two patterns is not gitattributes syntax: the second pattern parses as an attribute name, git rejects the line whole (`intents/** is not a valid attribute name`), and *neither* pattern gets `text eol=lf` — verified on git 2.50.1."* Until v0.19 it prescribed the single line `.spine/** intents/** text eol=lf`, which git discards whole so that **both** patterns silently lose `text` and `eol=lf`; §12 D1 keeps the measurement, including the two blob ids the two forms produce.

The entry this document depends on is those two lines:

```
.spine/** text eol=lf
intents/** text eol=lf
```

With those, a CRLF worktree copy of an intent commits as the canonical blob. Without them, it commits as the CRLF blob, `--sign` refuses it under rule 6, and the developer whose editor writes CRLF cannot sign an intent at all.

---

## 3. Identity and versioning

### 3.1 The intent id

An intent id is a **prefix**, a hyphen, and a **numeral**:

```
id        := prefix "-" numeral
prefix    := "INT" | "BUG"
numeral   := a decimal integer 1 … 9007199254740991, written in ASCII digits,
             left-padded with "0" to a minimum width of 3, and padded no further
```

So `INT-001`, `INT-042`, `BUG-051`, `INT-1042` are ids; `INT-42` (under-padded), `INT-0042` (over-padded), `INT-000` (zero), `INT-+42`, `int-042` (case) and `TASK-042` are not (`bad-id`, `bad-id-padding`).

The padding rule makes id and integer a bijection, which three mechanisms need and none states: `spine new` allocates `max+1` over live refs and sealed ids (PB §5.4), G9 requires *"exactly one `Spine-Event: land` per intent id"*, and G7's *"the lower intent id holds the lease"* is a numeric comparison. Two spellings of one number would break all three.

**Three places carry the id and all three must agree**: the file path `intents/<ID>.md`, the branch `refs/heads/intent/<ID>` (PB §11), and the title line (§4.2). A parse is given the id from the path; a title naming another id is `id-path-mismatch`. The branch check is `spine new --sign`'s (§8.1); the indexer parsing a landed envelope has no branch and skips it.

`BUG-` selects the Bug variant (§3.3) and, at approval, PB §4.3's rule that the reproduction AC must be red or `--approve` is refused outright. Nothing else in this document distinguishes the prefixes.

### 3.2 `Template:` — the variant and the version, and which parser runs

`Template: <variant>@<n>` names **both** the variant and the version, and the pair names the parser. PB §3.4 is explicit about why the field exists — *"sealed intents live in git history forever and the indexer must parse every generation of them"* — and about what it must carry: it names *"the **variant as well as the version** (`intent@2`, `intent-change@2`, `intent-bug@2`), because G4 must index the `resign` map by variant and the indexer must pick a parser by name, and neither is decidable from a bare `v2`"*. `spine new` stamps it **from the install manifest, never from the binary**, so one developer's newer binary cannot fork the team's template.

```
template-value := variant "@" version
variant        := "intent" | "intent-change" | "intent-bug"
version        := a decimal integer 0 … 999, in ASCII digits, no leading zeros
                  except the single digit "0"
```

A value that is not `variant "@" version` is `bad-template`; a value whose variant token is outside the closed set is `template-variant-unknown`, exit 4 — refused, never carried opaque, for §10 rule 6's reason. The version's spelling is unique because leading zeros are forbidden, which §8.4 of `templates.md` depends on.

**The variant token is matched byte-exactly and case-sensitively.** `Intent@2` and `INTENT-CHANGE@2` are `bad-template`, not variants. The three tokens are simultaneously the manifest's `templates` and `resign` keys (§8.1, `manifest.md` §3.6), and a header that casefolded where a JSON member name does not would be two spellings of one map key. This document defines the pair `intent@2`.

**The legacy bare spelling, and how it maps to a variant.** A value of the form `v<n>` carries no variant token. It is the **legacy spelling**, it still parses, and it is accepted for `n ∈ {1, 2}` only — the two generations that predate decision 4 of PB v0.19. A bare `v3` or higher is `bad-template`: there is no generation it could name, and accepting it would create a second permanent spelling for every version yet to ship. `spine new` never emits the legacy form at any version (`templates.md` §7.1), and no repository holds a document carrying it, because no release has shipped; it is defined so that PB §3.1's promise about older generations has a referent rather than an implication.

A legacy value names no variant, so **the variant is derived** — by §3.3's pre-pass, which now exists for this path and for nothing else. The derivation is total, so a legacy document always selects exactly one variant; §3.3 states what it costs.

**Reading an unknown version.** A reader that does not hold the parser for a document's `(variant, version)` pair **refuses**: status `template-version-unknown`, exit 3. It never partially parses, never guesses a nearby version, never falls back to the newest it knows, and never substitutes another variant's parser for the same number — `intent@3` is not a parser for `intent-change@3`. A binary keeps a parser for every pair it has ever shipped — the same promise PB §6.7 makes for manifests and `gate-report.md` §3.2 makes for reports, refined from one counter to three by `templates.md` §9.1.

This is reachable in exactly one way inside a healthy repository, and G15 closes it: the running binary is pinned by trunk's manifest, `spine new` stamps the manifest's version for the variant it is creating, so every document in a repository carries a pair the pinned release knows. The unknown case is a stale clone, a hand-written document, or a cross-repository read — and in every one of them refusing is right.

**Template version 1**, which PB §3.1 promises still parses (*"their `Status:` field is ignored"*), is defined here as: version 2, plus one additional permitted header field `Status`, whose value is any non-empty free-text run and is parsed and discarded. No other difference, and `templates.md` §9.2 extends it uniformly to all three variants. Version 1 is reachable **only** through the legacy spelling `Template: v1`: a `Status` field beside a qualified value is `unknown-header-field`, because a qualified value is by construction a value stamped after decision 4 and no such generation ever carried `Status`.

**Why version 2 changed spelling without becoming version 3.** The variant token changes the bytes of every version-2 document, and `templates.md` §7.4 makes a shipped version's scaffold bytes immutable. Both hold, because **no release has shipped**: there is no `v2` document anywhere for the respelling to invalidate, so version 2 is being *defined* with the qualified header rather than edited after the fact — the same argument §11.9 uses to create version 1. After the first release the identical change would be a version bump, and `templates.md` §7.4's permission table says so. See §11.9 and §12 D2.

### 3.3 The three variants, how one is selected, and the legacy derivation

PB §3.5 gives three templates within the gated lane — Feature, Change (brownfield), Bug — and PB §6.7's manifest carries three independent version numbers (`templates.intent`, `templates.intent-change`, `templates.intent-bug`) and three independent `resign` floors, **keyed by exactly the three tokens `Template:` now carries**. So selection is a read, not an inference:

```
variant(d) := the variant token of d's `Template:` value          (§3.2)
```

G4 therefore indexes `resign[variant(d)]` directly and the indexer picks a parser by name, neither having to guess. §12 D2 records the defect this closes and §11.9 records the compatibility cost.

**One consistency rule, and it is mechanical.** The id's prefix and the header's variant token must agree:

| Id prefix | Permitted variant token |
|---|---|
| `BUG` | `intent-bug` |
| `INT` | `intent`, `intent-change` |

A disagreement is `variant-prefix-mismatch`, exit 4, checked at §8.2 step 4. This is what turns `templates.md` §3.3 — headed *"`--bug` forces the prefix, and that is now checked as well as required"* — into an enforced rule rather than a CLI convention. Under the derivation rule a Bug document carrying an `INT-` id parsed cleanly as a Feature and silently lost PB §4.3's outright refusal of a green reproduction, with no detector anywhere in the design (`templates.md` §13 D11). Now the two facts are checked against each other, and the failure is loud.

**The legacy derivation** applies to a document whose `Template:` value is the bare `v<n>` spelling (§3.2), and to nothing else:

```
variant_legacy(d) :=
  "intent-bug"     if the id's prefix is "BUG"
  "intent-change"  else if d contains a line whose section key (§4.7) is "invariants"
  "intent"         otherwise
```

The probe is a **pre-pass**: scan every line whose first three bytes are `## `, compute its key by §4.7, and test for `invariants`. It runs before the section table is chosen and reads nothing else. It is total — every legacy document selects exactly one variant — and it is stable under every edit that does not add or remove an `## Invariants` heading. Its known failure is why it is no longer the primary rule: a Change intent whose Invariants section is renamed derives to `intent`, meets an unknown `current behavior`, and is refused for the wrong reason. `templates.md` §3.2's disjointness invariant is what keeps the derivation total, and after decision 4 that invariant binds this path alone.

This document specifies the section table of variant **`intent`** in full (§4.8) and fixes for all three: the line model (§4.1), the preamble (§4.2–§4.5), how a section is located and terminated (§4.6–§4.7), the body line classes (§4.10), every field grammar the variants share (§5), and touchpoint matching (§6). `templates.md` owes only the two other rows of §4.8 — which section keys, in which order, mandatory or optional, with which body grammar — and **must not** add a body grammar, a header field, or a matching rule of its own.

### 3.4 What the parse is a function of

The parse result (§5.6) is a function of exactly two inputs: the document's bytes, and the id taken from its path. Not of the repository, not of the tree, not of the manifest, not of the clock, not of the local git version, not of the running binary's own template version beyond the accept/refuse decision of §3.2. This is what lets `--verify`, the indexer, the G10 clone and the lease evaluator all reach the same answer without agreeing on anything else.

---

## 4. The document grammar

### 4.1 The line model

The parse is **line-oriented and is not Markdown.** It runs one pass over the lines of §2.1's decoded string. It knows nothing about fenced code blocks, inline code spans, HTML blocks, link reference definitions, list nesting, or lazy continuation. A line's class is a function of its own leading bytes and of the section it is in — never of a state a fence opened.

That is a decision, and §11.1 defends it. Its one visible consequence: a line whose first three bytes are `## ` begins a section **wherever it appears**, including inside what an author intended as a code fence. The result is a loud refusal (`unknown-section`, or `duplicate-section`), never a silent difference between two parsers — which is the property this document exists to buy.

A document is: one **title line**, one **header line**, an optional **`Supersedes:` line**, zero or more blank lines, then zero or more **sections**.

```
document  := title-line header-line [supersedes-line] blank* section*
section   := heading-line body-line*
```

### 4.2 The title line

Line 1, and there is exactly one. A line whose first two bytes are `# ` may not appear anywhere else in the document (`duplicate-title`); a second such line inside a section body is refused rather than treated as body text, because two concatenated documents must not parse as one.

```
title-line := "# " id ": " title
title      := 1 … 72 bytes, containing no U+000A, with no leading or trailing
              U+0020 or U+0009
```

`id` is §3.1's and must equal the id from the path. The title's bytes are recorded verbatim as the `intent` node's `title` attr (`dump.md` §7.2) and are the subject line of the landing commit `L` after PB §5.5's `<ID>: ` prefix.

**The landing subject is derived from these bytes, not written beside them** — decision 6 of PB v0.19 — and **G9 recomputes it and checks it**. So the title is a gate input, not only a display string: a gated landing whose subject is not `<ID>: ` ++ these exact bytes fails G9. Two consequences reach this document. The 72-byte bound below is load-bearing at landing as well as at sign-off; and the subject stays **outside `envelope=`**, so nothing here changes a digest — deriving it cost no digest change, which is why it was decided at all. The residual is stated where it belongs rather than hidden: **the quick lane has no intent document and its summary is free text**, and PB §11 routes every toolkit lifecycle landing through the quick lane, so an uninstall can land under any subject at all with every signature intact. Nothing in this document reaches that lane — there is no title for G9 to recompute from — and §14 leaves it to PB §5.5.

**72 bytes** is a hard refusal (`title-too-long`), not advice. `INT-042: ` plus 72 is 81 columns, so a landing's subject stays one line in every tool that shows one, and the bound makes the envelope's 16 KiB projection computable at `--approve` from the parse alone. Raising it is a template version bump.

The title is not parsed further. `<short imperative title>` in PB §3.1 is guidance to the interview agent, not a grammar.

### 4.3 The header line

Line 2, and there is exactly one. It is a sequence of **fields** separated by the three bytes `0x20 0xC2·0xB7 0x20` — space, U+00B7 MIDDLE DOT, space:

```
header-line := field (" · " field)*
field       := name ": " value
```

`name` is drawn from a closed table and the fields appear in the table's order. A name outside the table is `unknown-header-field`; a repeat is `duplicate-header-field`; an out-of-order field is `header-field-order`; a field with no `": "` is `bad-header-field`.

| Order | Name | Presence | Value grammar | Consumed by |
|---|---|---|---|---|
| 1 | `Owner` | mandatory | 1 … 128 bytes, no U+000A, not containing `" · "`, no leading or trailing space or tab | `intent.owner` attr. **A hint, never authority** (PB §3.1): the truth is `signed_by`. A leading `@` is retained, not stripped. |
| 2 | `Template` | mandatory | §3.2's `<variant>@<n>`, or the legacy bare `v<n>` for `n ∈ {1, 2}` | selects the variant **and** the parser; §5.6's `variant` and `template` members, and the `intent.template` attr (`dump.md` §7.2), which is always the canonical `<variant>@<n>` however the header was spelled |
| 3 | `Ticket` | optional | as `Owner` | nothing. Recorded by no node, read by no gate. |
| 4 | `Constitution` | mandatory | `v` + a decimal integer `0 … 999`, no leading zeros except `0` | the `built_under` edge to `<repo>/constitution:v<n>` (`dump.md` §5.2); G4's currency check (PB §6.3) |

A value may not be empty: `Ticket: ` with nothing after it is impossible anyway, since §2.1 rule 9 forbids the trailing space, and `Ticket:` with no `": "` is `bad-header-field`. A field with nothing to say is omitted, and only `Ticket` may be.

**Template version 1 additionally permits `Status`** at order 5, value as `Owner`, parsed and discarded (§3.2). Version 1 is reachable only through the legacy bare spelling, so a `Status` field beside a qualified `Template:` value is `unknown-header-field`.

### 4.4 The `Supersedes:` line

Optional. If present it is line 3 — immediately after the header line, with no blank line between.

```
supersedes-line := "Supersedes: " id
```

Exactly one id, and nothing after it. PB §3.1's template shows `Supersedes: INT-017                        (optional)`; the parenthetical is template annotation and is **not** part of the value — a document carrying it is `bad-supersedes` (§12 D7). A second `Supersedes:` line is `bad-supersedes` too, because a `Supersedes:` line may only be line 3.

One id and not a list, because PB §11's `Spine-Supersedes` payload is `INT-017` singular and PB §6.2 derives the `supersedes` edge from it. An intent that supersedes two others is not representable at template version 2 in any variant.

The value's id need not exist, need not be landed, and is not checked against the ledger by the parse. PB §6.2 derives the edge; whether its target resolves is G5's business.

### 4.5 The preamble terminator

After the title line, the header line and the optional `Supersedes:` line, every line up to the first heading line must be empty. A non-empty line there is `stray-preamble`. The template's blank line before `## Goal` is therefore permitted but not required, and two blank lines are permitted.

A document with fewer than two lines is `truncated`.

### 4.6 How a section is located, and what terminates one

A **heading line** is a line whose first three bytes are exactly `## ` — two U+0023 and one U+0020. Nothing else is a heading:

- `###` and deeper are body text. A sub-heading inside a section is content, not a boundary.
- `##Goal`, with no space, is body text.
- `  ## Goal`, indented, is a continuation line (§4.10) or body text, never a heading.
- Setext underlining (`Goal` over `-----`) is refused by §2.1 rule 11 before it can be considered.

A section's **body** runs from the line after its heading to the line before the next heading line, or to the end of the document. A section is terminated by the next heading line and by nothing else — not by a blank line, not by indentation, not by the end of a list.

Level is fixed at two and is not negotiable: level 1 is the title and may not recur (§4.2); level 3 and deeper are body.

### 4.7 The section key

A heading line's **key** is computed by three steps, in order:

1. take the bytes after the leading `## `;
2. strip leading and trailing `0x20` and `0x09`;
3. take the bytes before the first `0x28` (`(`), strip trailing `0x20` and `0x09` from what remains, and ASCII-lowercase it — bytes `0x41…0x5A` mapped to `0x61…0x7A`, and no other byte changed.

So `## Acceptance criteria (maximum 6 — more means split the task)` has key `acceptance criteria`; `## Non-Goals` has key `non-goals`; `##  Goal  ` — impossible under rule 9, but were it possible — would have key `goal`.

The parenthetical is the template's own guidance and is **advisory in every respect**: it may be present, absent, or reworded, and the key is unchanged. Nothing reads it. That is what lets `spine new` scaffold `## Non-goals (mandatory, minimum 2)` while an author who deletes the hint still has a parsing document.

Three consequences worth naming. A key is ASCII by construction only if the heading's leading text is; a heading whose key contains a non-ASCII byte simply fails to match the closed table and is `unknown-section`. An empty key — a heading of `## ` alone, or `## (hint)` — is `unknown-section`. And a key is casefolded but never normalised (§2.1), so a heading spelled with a fullwidth letter is a different key.

### 4.8 The section table — `intent@2`

Closed, ordered, and complete. `templates.md` owes the equivalent table for `intent-change` and `intent-bug` and nothing else.

| Ordinal | Key | Presence | Body grammar | PB §3.1 heading as scaffolded |
|---|---|---|---|---|
| 1 | `goal` | mandatory | **prose** (§5.1) | `## Goal (2–3 sentences)` |
| 2 | `non-goals` | mandatory | **bullet** (§5.2) | `## Non-goals (mandatory, minimum 2)` |
| 3 | `acceptance criteria` | mandatory | **ac** (§5.3) | `## Acceptance criteria (maximum 6 — more means split the task)` |
| 4 | `touchpoints` | mandatory | **touchpoints** (§5.4) | `## Touchpoints (expected blast radius)` |
| 5 | `open questions` | optional | **free** (§5.5) | `## Open questions (optional — must be empty before implementation)` |

### 4.9 Unknown, duplicate, missing, misordered

| Condition | Behaviour | Status |
|---|---|---|
| A heading whose key is not in the variant's table | **Refuse.** Not ignored, not carried opaquely. | `unknown-section` |
| Two headings with the same key | **Refuse.** | `duplicate-section` |
| A mandatory key absent | **Refuse.** | `missing-section` |
| Sections present but not in ascending ordinal order | **Refuse.** | `section-order` |
| An optional key absent | Fine. Its parse-result member is absent (§5.6). | — |
| No sections at all | Caught by `missing-section`. | — |

**Why unknown sections are refused rather than ignored.** `gate-report.md` §3.2 states the general rule and it applies unchanged: *"forward compatibility is bought with a version bump, not with tolerance, because a tolerant reader and a strict one compute different digests over the same document."* Here the divergence is worse than a digest. A tolerant parser lets a document carry a section named `## Touchpoint` — singular, a typo — that declares the real blast radius while the mandatory `## Touchpoints` carries something narrower, and the two parsers disagree about what was declared. It also makes the intent document an open-ended instruction surface inside the fenced envelope bytes, which PB §7.3 treats as a protected category everywhere else.

**Why the order is enforced.** Two reasons, and the second is mechanical. Uniformity: a signed one-pager that reads the same way every time is cheaper to review, which is what PB §6.5 says review fatigue costs. And PB §4.3's reopen rule — a reopen against a `resign` floor *"rewrites the header to the floor version and inserts each new mandatory section as an empty stub"* — needs a defined insertion position for the stub. Fixed order gives it one: at the new section's ordinal.

### 4.10 Body line classes

Within a section body, a non-empty line is classified by its first bytes, in this order. A blank line is a separator and is classified as nothing.

| Class | Test | Notes |
|---|---|---|
| **bullet** | first two bytes are `- ` | text is the rest; empty text is `empty-item` |
| **ac** | first three bytes are `AC-` | must then match §5.3's grammar exactly, or `malformed-ac` |
| **continuation** | first byte is `0x20` or `0x09` | strip the leading run of spaces and tabs; the remainder joins the preceding item with one `0x20` |
| **prose** | anything else | |

Two traps are closed by refusal rather than by a rule an author has to know:

- A continuation whose stripped text begins `- ` is `indented-item`.
- A continuation whose stripped text begins `AC-` is `indented-ac`.

Without these, an author who indents `AC-2` under `AC-1` silently ships a document with one AC, the second AC has no node, G1's coverage clause is vacuous over it, and nothing anywhere says so. The whole point of the AC id is that it flows downstream (PB §3.2); an AC that vanishes at parse time is the failure mode with no detector. A continuation with no preceding item is `stray-continuation`.

Which classes a section admits is the section's business (§5). A class a section does not admit is `stray-text`.

---

## 5. The fields

### 5.1 Goal — body grammar `prose`

Every non-empty line must be **prose** or **continuation**; a bullet or an AC line is `stray-text`. At least one non-empty line is required (`empty-section`).

**Nothing else is checked, and nothing reads the text.** PB §3.1's *"(2–3 sentences)"* is advisory and is never counted: sentence segmentation is a natural-language judgement with no byte-exact specification, and a parser that counts and one that does not would refuse different documents. PB §3.2's *"outcome-phrased, not implementation"* is the same. Both are interview-agent guidance (PB §3.4) and Agent B's material.

PB §6.2's `intent` attrs are `{status, owner, title, template, blob, signer, reopen_count, late_reopen_count, landing, base}` — there is no `goal`. The Goal's mechanical content is therefore exactly one bit: it is present and non-empty. That is not a weakness of the specification; it is the honest reading of a schema that deliberately holds no prose.

### 5.2 Non-goals — body grammar `bullet`

Every non-empty line must be a **bullet** or a **continuation**; prose and AC lines are `stray-text`. Items are the bullets, in document order.

| Rule | Value | Status |
|---|---|---|
| Minimum items | 2 | `non-goals-too-few` |
| Maximum items | 256 | `too-many-non-goals` |
| Item text | non-empty after `- ` | `empty-item` |

**Which command enforces the minimum, and what happens on 1.** The count is part of the parse, so *every* consumer of §1's table enforces it identically: `spine new --sign` refuses to sign (exit 4, `non-goals-too-few`, and no event commit is written); `spine check --approve` and `--land` refuse before any gate runs; the indexer refuses the document, which for a landed envelope means G9 records that landing `unattested` (§8.3). A document with one non-goal has no valid path into the ledger. There is no override flag, no warn mode and no `--force`: PB §3.2 calls this *"the highest-leverage sixty seconds in the document"*, and a cap with an escape hatch is advice.

PB names no enforcer at all, which is §12 D3.

**The text is never read.** PB §6.2 is explicit: *"Non-goals are not nodes. They are prose constraints with no mechanically derivable edges."* The mechanical content of this section is one integer.

### 5.3 Acceptance criteria — body grammar `ac`

Every non-empty line must be an **ac** or a **continuation**; prose and bullets are `stray-text`.

```
ac-line := "AC-" number ": " text
number  := a decimal integer 1 … 6, no leading zeros
text    := non-empty, no U+000A
```

A line beginning `AC-` that does not match is `malformed-ac`. That clause is load-bearing: it is what stops `AC-3 the total is right` (no colon) from being silently reclassified as prose and dropped.

| Rule | Value | Status |
|---|---|---|
| Minimum items | 1 | `no-acceptance-criteria` |
| Maximum items | 6 (PB §3.1) | `too-many-acs` |
| Numbering | the numbers, in document order, are exactly `1, 2, …, k` | `ac-numbering` |

**Numbering is contiguous from 1 and in order.** Deleting AC-3 means renumbering. This is enforced because the id is the join key to everything downstream — `@verifies INT-042/AC-1` pragmas, `test_AC1_*` names, the `<repo>/INT-042/AC-1` node id (`dump.md` §5.2), G1's coverage clause and G5's orphan clause — and a document with `AC-1, AC-2, AC-7` either has a seventh AC that is not there or a numbering scheme nothing else in the system shares. It also makes the maximum mechanical: with contiguous numbering, `AC-7` cannot exist.

**Minimum 1** is not in the playbook and is resolved here (§11.4). A zero-AC intent makes `--approve`'s *"every AC covered by a collected id"* guard vacuous, makes G1's coverage clause vacuous, and asks a human to sign a document that promises nothing testable — which is the failure PB §3.4 designs the interview to prevent.

**EARS phrasing is advisory and is never checked.** PB §3.1 shows `Given <state>, when <action>, then <observable result>` and PB §1.1 credits *"EARS-style phrasing for acceptance criteria"* to Kiro. EARS is a family of five patterns, of which the template shows one; deciding whether an English sentence is in the family is a natural-language judgement, and a checker that enforced it would refuse documents another checker accepts. It stays where PB §3.4 puts it — the interview agent must *"stress-test AC verifiability"* — and where PB §4.2 puts it, as Agent B's material. §11.3 records the choice.

**The text is never read either.** PB §6.2 gives the `ac` kind no attrs, and `dump.md` §7.2 makes that explicit: *"a kind PB §6.2 does not give attrs for has none in the dump"*. An implementation may store the text; the graph does not carry it and G10 does not compare it. The mechanical content of this section is the set of AC ids and their count.

### 5.4 Touchpoints — body grammar `touchpoints`

Every non-empty line must be a **label line**. Prose, bullets, AC lines and continuations are all `unknown-touchpoint-line`. There is no prose in this section, which is what makes a mistyped label loud instead of silent.

```
label-line := label ":" [ " " pattern-list ]
label      := "Expected to change" | "Must NOT change"     -- ASCII case-insensitive
pattern-list := pattern-field ("," pattern-field)*
pattern-field := [space-or-tab*] pattern [space-or-tab*]
```

The label is matched by ASCII-lowercasing the bytes before the first `:` and stripping leading and trailing spaces and tabs, then comparing against the closed set `{expected to change, must not change}`. `Must not change`, `MUST NOT CHANGE` and `must not change` all parse; `Must NOT chnage` is `unknown-touchpoint-line`.

| Label | Polarity | Presence | Minimum entries |
|---|---|---|---|
| `Expected to change` | `expected` | mandatory, exactly once | 1 (`no-expected-touchpoint`) |
| `Must NOT change` | `forbidden` | mandatory, exactly once | 0 |

A missing label line is `missing-touchpoint-line`; a repeated one is `duplicate-touchpoint-line`. The line may not be continued: a long list is one line, bounded by §2.3's 4096 bytes.

**The empty forbidden set is written `Must NOT change:` with nothing after the colon** — the trailing space is already forbidden by §2.1 rule 9, so there is exactly one spelling. The label is still mandatory, because an absent line and an empty line are different claims and only one of them was made deliberately.

**Splitting.** Split the value on `,` (`0x2C`), then strip leading and trailing spaces and tabs from each field. A field that is empty after stripping — a trailing comma, a doubled comma, a list of one comma — is `empty-touchpoint`. Each surviving field must be a valid pattern by §6.1. This split is unambiguous because §6.1 forbids `,` and space inside a pattern.

**Duplicates and conflicts.** A pattern repeated within one polarity is permitted and deduplicated by byte equality, because the `declares` edge set is a set (§6.6). A pattern appearing **byte-identically in both polarities** is `polarity-conflict` and the document is refused: it declares a path both expected and forbidden, and every landing that touched it would be a hard G2 failure. Overlap that is not byte-identical — `expected: src/`, `forbidden: src/auth/` — is legal, meaningful and common; §7.1 fixes its precedence.

**Nothing here consults the tree.** The patterns are recorded as written. They are never expanded to the set of paths they currently match, which would make the parse a function of the tree, would make a landed intent's `declares` edges depend on when they were derived, and would break G10's byte equality between two indexings of the same objects at different tips. §11.6 records the choice, and §12 D4 records that PB §6.3's own G2 query assumes the opposite.

### 5.5 Open questions — body grammar `free`, and what "empty" means

The section is optional. Any non-empty line is permitted, of any class, and none of it is parsed.

**The section is *empty* iff its body contains no non-empty line.** Not "no bullets": a body of prose, or of a single line reading `None`, or `- (none)`, is **not** empty. This is the strictest available reading and it is the right one, because the condition it feeds is *"this converts 'the agent assumed' into 'the agent asked'"* (PB §3.2) — a section with words in it has words in it.

**Emptiness is a sign-off precondition, not a parse rule.** A document with open questions is a valid document; it is the normal state of an intent in `draft`. `spine new --sign` refuses it (§8.1, exit 5, `open-questions-nonempty`); `--approve`, `--land` and the indexer accept it, because a landed intent's Open questions section was empty at signing and is being read years later for archaeology. This is the one place where a rule about the document's *stage* is separated from rules about its *shape*, and §11.5 says why.

**One constraint on `templates.md`, and it is normative.** The scaffolded body of this section must be empty — no guidance line, no placeholder bullet. Guidance goes in the heading's parenthetical, where §4.7 discards it. A scaffold that seeds a prose line here makes every freshly created intent unsignable.

PB's transition table names `spine new` as the enforcer of this condition, which it cannot be; that is §12 D5.

### 5.6 What the parse produces

The parse result is a value with exactly these members. Two implementations agree iff they produce this value for every document. Absent means the member is not present, never `null` and never empty — `gate-report.md` §7 rule 6's rule, unchanged.

| Member | Type | Presence | Value |
|---|---|---|---|
| `id` | string, ASCII | always | §3.1, from the path, equal to the title's |
| `variant` | string | always | `intent` \| `intent-change` \| `intent-bug` — the header's variant token (§3.2), or `variant_legacy(d)` for a legacy bare value (§3.3) |
| `template` | integer | always | the `Template:` value's version `<n>`, in either spelling |
| `title` | string, bytes | always | the title line's text, verbatim |
| `owner` | string, bytes | always | the `Owner` field's value, verbatim, `@` retained |
| `ticket` | string, bytes | iff the field is present | verbatim |
| `constitution` | integer | always | the `Constitution:` value's `<n>` |
| `supersedes` | string, ASCII | iff the line is present | one id |
| `goal_present` | boolean | always | always `true` when the parse succeeded; the member exists so the shape is total across variants where Goal is replaced (§3.3) |
| `non_goal_count` | integer | always | 2 … 256 |
| `acs` | array of integers | always | `[1, 2, …, k]`, `1 ≤ k ≤ 6`; the ids, in order |
| `expected` | array of strings, ASCII | always | patterns as written, in document order, duplicates removed keeping the first occurrence; length ≥ 1 |
| `forbidden` | array of strings, ASCII | always | as `expected`; length ≥ 0 |
| `open_questions_empty` | boolean | always | `true` if the section is absent or its body has no non-empty line |

**The header's spelling is not a member, and leaves no trace in the graph.** `variant` and `template` are the two facts; the `intent.template` attr `dump.md` §7.2 records is their canonical concatenation `<variant>@<n>`, reconstructed rather than copied, so a legacy `Template: v2` document and a `Template: intent@2` document with otherwise identical bytes — which cannot both exist, having different blob ids — would still yield the same attr. Nothing downstream can tell which spelling a document used, which is the property that lets the legacy form be retired without a graph migration.

Deliberately not members: the Goal's text, the non-goals' texts, the ACs' texts, the heading parentheticals, and the blank-line layout. Nothing in the graph holds them (PB §6.2, `dump.md` §7.2) and nothing in a gate reads them. An implementation may keep them for `spine review`'s packet; two implementations that differ in whether they do still agree on every gate verdict.

---

## 6. Touchpoint patterns and matching

This is the section the audit named. PB §5.2 says v1 touchpoint checks are *"path-prefix matching"*, and taken literally that is wrong in a way that silently widens a declaration: **`src/bill` would match `src/billing/x.ts`**, so an intent that declared one module would have licensed a differently-named sibling, and G2 would have passed a diff nobody declared. The rules below fix it with segment-boundary matching, and that exact pair is a published vector (§9.5).

### 6.1 The byte grammar of a pattern

A pattern is 1 … 255 bytes, each in `0x21 … 0x7E`, excluding three:

| Excluded | Why |
|---|---|
| `0x2C` `,` | the list separator here (§5.4) and the `wires=` separator a review signs (`gate-report.md` §6.2) |
| `0x22` `"` | `git ls-tree`'s quoting trigger, JSON's string delimiter, and a trailer `reason=`'s |
| `0x5C` `\` | `esc`'s escape byte (`gate-report.md` §2.3), and never a path separator in git |

`0x20` (space) is excluded by the range. Bytes above `0x7E` are excluded by the range, so **a pattern is ASCII**; §11.7 records that resolution and OPEN-2 asks the owner whether to lift it. Failing bytes are `pattern-illegal-byte`.

Further refusals, all hard:

| Condition | Status | Why |
|---|---|---|
| empty | `pattern-empty` | |
| longer than 255 bytes | `pattern-too-long` | §2.3 |
| begins `!` | `bad-negation` | negation makes the declared set order-dependent; G2 and G7 are set operations, and an ordered pattern list is a second semantics for `templates.md` and `constitution.md` to get wrong |
| begins `/` | `leading-slash` | every pattern is root-anchored already; gitignore's meaning for a leading slash is *anchoring*, so accepting it would teach a false lesson |
| contains `//` | `empty-segment` | |
| has a segment `.` or `..` | `dot-segment` | git paths have neither; accepting them invites a matcher to resolve them |
| a segment contains `**` but is not exactly `**` | `bad-globstar` | §6.2 |
| a malformed bracket | `bad-bracket` | §6.2 |

There is **no escape mechanism**, because `\` is not an allowed byte. A path whose name literally contains `*`, `?` or `[` cannot be declared as a touchpoint; declare an ancestor directory instead. This is deliberate: an escape syntax is the single most divergent corner of every glob dialect, and refusing to have one removes the corner.

**`esc` and `tok` are the identity on every legal pattern.** No legal pattern contains a byte `esc` escapes (`\`, or anything outside `0x20…0x7E`) or a byte `tok` additionally escapes (`,`, space, `"`). So a pattern's bytes, its `code_unit` node id suffix, and its `G2:<path>` wire token are the same bytes. Nothing here needs a second encoding.

### 6.2 The glob dialect

A pattern's **segments** are its bytes split on `/`; a trailing `/` yields a final empty segment which §6.3 removes before splitting. Within a segment:

| Construct | Matches |
|---|---|
| `?` | exactly one byte, and it is never `/` — a segment holds no `/` |
| `*` | zero or more bytes, none of them `/`. **`*` does not cross a separator.** |
| `[ … ]` | one byte from the set |
| any other byte | itself, exactly |

A whole segment equal to `**` matches **zero or more complete segments**, and `**` may appear only as a whole segment.

**`**` crosses separators; `*` does not.** That is the one question every glob dialect answers differently, and it is answered here for both.

**`**` matching zero segments is uniform.** `a/**/b` matches `a/b`; `**/x` matches `x`; and, following the same rule with no special case, `a/**` matches `a`. gitignore requires at least one segment for a trailing `a/**`; this document does not, because a uniform rule has no corner to get wrong and the only difference is whether a *file* named `a` is matched — which §6.3's segment-boundary clause already decides for the metacharacter-free case.

**A `**` that is not a whole segment is refused, not reinterpreted.** `src/**.ts` and `a**b` are `bad-globstar`. Bash's globstar, minimatch and git's pathspec all disagree about what those mean; refusing removes the disagreement instead of picking a winner. Multiple single `*` in one segment are fine: `a*b*c` is legal.

**Bracket expressions.**

```
bracket := "[" [ "!" ] [ "]" ] member* "]"
member  := byte | byte "-" byte
```

- A leading `!` negates. `^` does **not** negate; it is an ordinary member byte. One spelling, not two.
- A `]` immediately after `[` or `[!` is a literal member. `[]]` is the set `{ ] }`.
- A range `a-b` requires `a ≤ b` as byte values; `[z-a]` is `bad-bracket`.
- An unterminated `[` is `bad-bracket`. It is **not** treated as a literal `[`.
- `/` inside a bracket is `bad-bracket`.
- POSIX classes, collating symbols and equivalence classes are refused: a bracket whose first member byte (after an optional `!`) is `:`, `.` or `=`, or which contains the two-byte sequence `[:`, `[.` or `[=`, is `bad-bracket`. Their meaning is locale-dependent, and a locale is exactly the kind of environment input this design keeps out of a verdict.
- A bracket never matches `/`, which follows from segments containing none.

**No brace expansion.** `{` and `}` are ordinary bytes. `{a,b}` cannot arise anyway — the comma is the list separator.

Brackets are validated over the whole pattern **before** it is split into segments, so `[a/b]` is refused as `bad-bracket` rather than silently becoming two malformed segments.

### 6.3 `match(P, p)` — segment-boundary matching

Let `p` be a repository path as git produces it: a byte string, `/`-separated, no leading `/`, no `.` or `..` component, no trailing `/`.

Define `gmatch(P, s)`, whole-string glob matching, on the segment lists `ps = split(P, "/")` and `ss = split(s, "/")`:

```
go(i, j) :=
  if i = |ps|            : j = |ss|
  else if ps[i] = "**"   : ∃ k ∈ [j, |ss|] : go(i+1, k)
  else if j = |ss|       : false
  else                   : segmatch(ps[i], ss[j]) ∧ go(i+1, j+1)

gmatch(P, s) := go(0, 0)
```

Then:

```
match(P, p) :=
  if P ends with "/" :
     let Q := P without its trailing "/"
     ∃ a split p = q ++ "/" ++ r, with r non-empty, such that gmatch(Q, q)
  else :
     gmatch(P, p)
     ∨ ∃ a split p = q ++ "/" ++ r, with r non-empty, such that gmatch(P, q)
```

In words. **A pattern matches a path when it matches the whole path, or when it matches a prefix of the path that ends exactly at a `/`.** A pattern that ends in `/` is a *directory* pattern and gives up the first clause: it matches things *under* the named directory and never the directory's own path.

That single quantifier is the fix. `src/bill` does not match `src/billing/x.ts`, because the only prefixes of that path ending at a `/` are `src` and `src/billing`, and `src/bill` is neither. `src/billing` does match it, at the boundary. Prefix matching on raw bytes, which PB §5.2 prescribes, has no such quantifier and matches both.

**Directory versus file, distinguished.** The trailing `/` is the distinction and it is the only one — no stat, no tree lookup, no extension heuristic.

| Pattern | Matches the path `src/billing` itself | Matches `src/billing/x.ts` | Matches `src/billingx/y.ts` |
|---|---|---|---|
| `src/billing/` | no | yes | no |
| `src/billing` | yes | yes | no |

A pattern without a trailing `/` covers both a file and, if it names a directory, everything under it — which is what an author means by writing `api/invoices.ts`, and also by writing `src/billing`. A pattern with a trailing `/` is the way to say *"the contents, not the entry"*, which matters for a rename that turns a file into a directory.

**Vacuous patterns are legal.** `api/invoices.ts/` matches nothing unless a directory of that name exists. The parse cannot know, and does not guess.

**`**` alone is legal, and the defence is not the grammar. This is settled.** `Must NOT change: **` matches everything. PB §5.4 raises exactly this — *"or any pushed branch could declare `Must NOT change: **` and halt every landing"* — and answers it by deriving a lease only from a branch carrying a verifying `Spine-Signoff`. That closes the anonymous case and not the authorised one. Decision 5 of PB v0.19 leaves the authorised one open by design: **an unbounded `forbidden` set stays legal**, because a human signs it and because both polarities take the same patterns from the same dialect, so bounding one of them would fork the pattern language between `expected` and `forbidden` and between this document and `constitution.md` (§6.7). What is added instead is visibility — `spine stats` counts landings whose only protected wire is a G7 hard lease (§7.2) — so one intent quietly taxing every other landing is a number rather than a mystery. §12 D8 records the residual and §13 OPEN-3 records the decision.

### 6.4 What a pattern is matched against

The path set is supplied by the gate, not computed here. For completeness, because two implementations must feed the same set:

- **Renames contribute both paths.** PB §7.3 makes the same rule for the floor: *"matching runs over the full `merge-base..head` diff including renames and deletions — renaming `ci.yml` to `ci.yml.bak` is a touch."* A rename out of a declared area and a rename into a forbidden one are both hits.
- **Deletions contribute the deleted path.** A mode change contributes its path. A symlink or submodule entry contributes its path, and separately hits the floor (PB §7.3), which touchpoints cannot override.
- **No path is normalised, casefolded or decomposed** before matching. It is compared as the diff produced it — `gate-report.md` §2.3's rule for the same reason.

### 6.5 Case sensitivity, and why it differs from G14

**Touchpoint matching is byte-exact and case-sensitive.** `src/Billing/x.ts` does not match `src/billing/`.

G14 does the opposite: PB §7.3 casefolds paths before floor comparison, and *"a diff entry whose casefolded path equals an existing path's is itself a floor hit"*. The two rules are deliberately different and the difference is not an inconsistency:

- The floor is a **security boundary** defending against a second spelling of a protected file. Casefolding is the defence, and a false positive there costs a protected review.
- Touchpoints are a **declaration compared against git's own bytes**. Git's index is case-sensitive; a case-insensitive touchpoint match would produce one answer on a case-sensitive filesystem and another on a case-insensitive one for the same objects, which is the class of divergence this whole directory exists to remove.

And the case where the difference could bite is already closed: a repository containing two paths differing only in case is a G14 floor hit before G2 is consulted.

### 6.6 The `code_unit` node and the `declares` edge

Each distinct pattern, in either polarity, becomes exactly one `code_unit` node:

```
node id := <repo> "/" "code:" esc(pattern bytes)
```

which is `dump.md` §5.2's grammar, under which *"a trailing `/` means a directory"* — the same distinction §6.3 gives it. `esc` is the identity here (§6.1).

Each pattern yields one `declares` edge, intent → code_unit, with `attrs {"polarity": "expected"}` or `{"polarity": "forbidden"}` (PB §6.2). Provenance is PB §6.1's grammar: `<path>:<line>` for an in-flight document — `intents/INT-042.md:22` — and `git:<L>:msg:L<n>` for a landed one, `n` the 1-based line of the landing commit message on which the touchpoint line falls inside the fenced block. `dump.md` §5.4 owns the choice between them; this document owes only the line number, which is the touchpoint **label line's**, not the individual pattern's, since several patterns share one line.

A pattern appearing in both polarities is impossible (`polarity-conflict`, §5.4), so no `code_unit` carries two `declares` edges from one intent, and the edge set is a set under `(from, to, kind)`.

### 6.7 The relationship to the constitution's pattern lists

`C-Q1: quick.paths = docs/`, `C-T1: test.roots = tests/, src/**/__tests__/` and `C-T2` (PB §2.1) are pattern lists in the same positions: G2 evaluates the quick lane against `C-Q1`, and G8 evaluates harness membership against `C-T1`/`C-T2`. PB never says they share a language with touchpoints, which is §12 D6.

**They do, and this document is the definition.** `constitution.md` must adopt §6.1–§6.3 verbatim, including the byte grammar, the refusals, and `match`. A constitution whose `C-Q1` uses a different dialect would make one diff both inside and outside the quick lane depending on which matcher ran, and G2's quick-lane clause — *"⊆ `C-Q1` ∪ floor ∪ spine-owned paths"* — mixes a constitution list with a floor list in one set operation, which needs one semantics.

The **floor** is the exception, and it is stated so nobody unifies it by accident: PB §7.3's floor entries match at any depth and casefold (`**/AGENTS.md`, `**/.claude/**`). Its list ships inside the release, not in a repository, and G14 evaluates it under its own rule. Nothing here changes that.

---

## 7. The gates that are pure functions of this parse

Two gates read nothing but §5.6's parse result and a path set. Their semantics are PB §6.3's; what follows fixes the predicates so two implementations compute the same verdict.

Notation: `E` and `F` are the parse result's `expected` and `forbidden`; `Δ` is the path set of §6.4; `X` is the **exempt set** the gate supplies.

### 7.1 G2 — Containment

`X` for G2 is, per PB §5.2 and PB §6.3: the paths frozen by this intent's binding approval (*"paths frozen by this intent's approval are G8's, not G2's"*), plus spine-owned and floor paths (*"they are renders of a pinned release, verified by blob"*). Computing `X` is not this document's business; consuming it is.

```
forbidden_hits := { p ∈ Δ \ X : ∃ f ∈ F . match(f, p) }
outside        := { p ∈ Δ \ X : ¬∃ e ∈ E . match(e, p) } \ forbidden_hits
```

- `forbidden_hits` non-empty → **hard fail in every mode**, including warn-before-block (PB §11: *"a `forbidden` hit … blocks in every mode"*). One wire `G2:<tok(p)>` per path.
- `outside` non-empty → a containment finding, `warn` under calibration and `finding` otherwise (`gate-report.md` §6.1). One wire per path.

**Forbidden is evaluated first and dominates.** A path matching both an `expected` and a `forbidden` pattern is reported once, as a forbidden hit, and is not also reported as outside. PB §6.3 states the two clauses as if independent, which gives the same answer only if forbidden is evaluated first; §11.8 records the resolution. The practical case is common and intended: `expected: src/`, `forbidden: src/auth/` means *"this subtree, except that"*, and it works because forbidden wins.

The wire order is `gate-report.md` §6.1's and is not restated.

**PB §6.3's illustrative SQL cannot implement this** — it compares node ids with `NOT IN`, which is byte equality between `code:src/billing/tax.py` and `code:src/billing/`. §12 D4.

### 7.2 G7 — Interference, the hard clause

For each other in-flight intent `J` (PB §5.4 fixes which branches qualify; a lease derives only from the blob named in a verifying `Spine-Signoff`):

```
hard(J) := ∃ p ∈ Δ . ( ∃ f ∈ J.forbidden . match(f, p) )  ∨  ( p ∈ J.frozen )
```

- `J.forbidden` is `J`'s parse result, matched by §6.3. **This is the clause that makes §1 true**: `J`'s document, `J`'s patterns, my binary, my landing's verdict.
- `J.frozen` is the set of paths on `J`'s binding approval's `Spine-Frozen` lines. Those are **concrete paths, not patterns**, and the test is byte equality — a frozen path is a `(blob, path)` pair naming a file that exists, and applying glob semantics to it would silently widen a freeze into a subtree.

A hit is a `class=protected` wire at landing (PB §5.4), and PB §11's aggregation makes the landing `protected-review`.

**The counter decision 5 adds.** `J.forbidden` may be as broad as `**` (§6.1), and no grammar bounds it. `spine stats` therefore counts **landings whose only protected wire is a G7 hard lease** — landings where every other gate was clean and the sole reason a second human read the diff was another branch's `Must NOT change:`. The count is a function of the gate report's `wires` array and each wire's `class` (`gate-report.md` §6.1), so it is derivable from records already sealed and needs no new field anywhere. It is the whole of the mechanical answer to §12 D8: the exposure stays expressible, and it stops being invisible.

### 7.3 G7 — Interference, the soft clause, and `overlap`

PB §5.4's soft lease is `expected ∩ expected ≠ ∅` between two intents. Both sides are **pattern sets**, and intersecting two glob languages exactly is a decision procedure nobody should have to reimplement. This document defines a **sound over-approximation**, which is the right shape for an advisory signal: it never misses a real overlap, and its false positives cost a notification.

```
litprefix(P) := P                                     if P contains none of * ? [
                the longest prefix of P that ends in "/" and lies wholly before
                the first occurrence of * ? [          otherwise (empty if none)

segprefix(a, b) := a = ""                             -- empty is a prefix of everything
                 ∨ a = b
                 ∨ (a ends with "/" ∧ b starts with a)
                 ∨ b starts with a ++ "/"

overlap(P, Q) := segprefix(litprefix(P), litprefix(Q))
               ∨ segprefix(litprefix(Q), litprefix(P))
```

Two intents interfere softly iff `∃ e ∈ E_i, e' ∈ E_j . overlap(e, e')`.

**Soundness, in two lines.** If a path `p` matches both `P` and `Q`, then `litprefix(P)` and `litprefix(Q)` are each a segment-aligned prefix of `p` — for the metacharacter-free branch by §6.3's two clauses, and for the other branch because the bytes before the first metacharacter are matched literally and the truncation to a `/` makes what remains segment-aligned. Two segment-aligned prefixes of one string are comparable, and comparable segment-aligned prefixes satisfy `segprefix` in one direction. Hence no true overlap is missed. **Verified exhaustively** over 2 926 legal patterns × 399 paths: for every path, every pair of patterns matching it satisfies `overlap` — 0 violations (§9.6).

Truncating to the last `/` is load-bearing, not tidying. Without it, `litprefix("ab*")` would be `ab`, `litprefix("abc/")` is `abc/`, neither is a segment-prefix of the other — and both patterns match `abc/d`. With it, `litprefix("ab*")` is empty and the overlap is reported.

**Why not evaluate the soft clause over a tree.** Intersecting the two path sets a named tree realises is also total, but it misses two intents that both declare a directory that does not exist yet — which is the single most common way two agents collide on greenfield work. The syntactic rule catches it, needs no tree, and makes the soft finding a function of the two documents alone.

### 7.4 Another branch's document, parsed by my binary

Three cases, all decided:

**`J`'s document does not parse.** `J` contributes **no lease**, and the condition is reported as a diagnostic, not as a wire on my landing. Rationale: PB §5.4 already has this shape — *"an unsigned or revoked branch contributes no lease"* — and the failure is `J`'s to fix. Contributing no lease only weakens `J`'s own protection, so no attacker gains by it, and `J`'s own `--land` refuses the same document before it can land.

**`J`'s `Template:` version is unknown to my binary.** Same outcome: no lease, reported. Conservative in the only safe direction — a parser that guessed would compute a lease from a grammar it does not know. G15 makes this unreachable within one repository (§3.2), so it is a cross-repository or stale-clone case.

**`J`'s document exceeds a bound of §2.3.** No lease, reported, and my landing has spent at most 64 KiB of parsing on it.

In all three, my landing proceeds. A branch cannot deny service by pushing a document my binary cannot read.

---

## 8. Enforcement: which command refuses what

### 8.1 Two layers

**Layer 1 — the parse.** §2 (canonical form and bounds), §4 (grammar), §5's body grammars and shape bounds. Its verdict is the same for every consumer in §1's table. A document that fails Layer 1 has no parse result, so it has no ACs, no touchpoints and no `declares` edges.

**Layer 2 — the sign-off preconditions.** Checked only by `spine new --sign`, over a successful Layer 1 parse:

| Precondition | Status | Source |
|---|---|---|
| the branch is `refs/heads/intent/<id>` | `wrong-branch` | PB §11 |
| the worktree at `intents/<id>.md` is clean against the head blob | `worktree-dirty` | §2.4 |
| `open_questions_empty` | `open-questions-nonempty` | PB §3.1, PB §3.2 |
| `template` ≥ `resign[variant]` in the manifest at trunk — the variant read from the header (§3.2), not derived | `template-below-resign-floor` | PB §3.4, `manifest.md` §3.6 |

Everything else is Layer 1. In particular the non-goal minimum, the AC maximum, the AC numbering, the expected-touchpoint minimum and the polarity conflict are **Layer 1**, enforced identically by `--sign`, `--approve`, `--land` and the indexer. §11.5 defends the split: Layer 2 is about the document's *stage*, Layer 1 about its *shape*, and only shape can be checked years later against a sealed envelope.

### 8.2 Order and exit codes

| Exit | Status class | Members |
|---|---|---|
| 0 | `parsed` | — |
| 2 | `not-canonical` | every status of §2.1 and §2.3's document and line bounds |
| 3 | `template-version-unknown` | §3.2 |
| 4 | `malformed` | every status of §4, §5's grammars, §5's shape bounds, §6.1's pattern refusals |
| 5 | `signoff-refused` | §8.1's Layer 2 statuses (`spine new --sign` only) |

**The order is normative**, because two implementations checking the same things in a different order report different statuses for a document that is wrong in several ways at once:

1. canonical form and the document bound, §2.1's rules in their table order, then per line in line order — exit 2;
2. the title line, its id, and the id against the path — exit 4;
3. the header line and `Template:`'s syntax, including the variant token against §3.2's closed set (`bad-template`, `template-variant-unknown`) — exit 4;
4. variant selection — a read of the header, or `variant_legacy(d)` for a legacy value (§3.3) — then the id prefix against the selected variant (`variant-prefix-mismatch`) — exit 4;
5. the `(variant, version)` pair against the parsers this binary holds — exit 3;
6. `Supersedes:` and the preamble — exit 4;
7. section headings: keys, unknown, duplicate, missing, order — exit 4;
8. each section's body, in ordinal order, each body in line order — exit 4;
9. shape bounds, in section ordinal order — exit 4;
10. Layer 2, in §8.1's table order — exit 5.

Within a step, the first failure in document line order wins. A document breaking rules in two steps reports the earlier step's status.

**Why step 4 precedes step 5.** Variant selection and the prefix check are functions of the document alone; the parser lookup is a function of the *reader*. Ordering the reader-independent failure first means two binaries holding different parser sets still report the same status for a document that is wrong in both ways, and a `variant-prefix-mismatch` — the defect that used to have no detector at all (§3.3) — is never masked by a version one of them happens not to hold. It also makes step 5 well-posed: a `(variant, version)` pair cannot be looked up before the variant is known.

### 8.3 A landed document that does not parse

There is no route for one, and the failure mode if there were is already in the design.

`--sign` refuses a non-parsing document, so no `Spine-Signoff` names its blob. `--land` step 2 verifies the intent blob equals the signed blob and re-parses for G2, so it refuses. Break-glass cannot help: PB §11 permits bypassing `G1, G2, G3, G4, G6, G7, G8, G12` only, and never G9. The remaining route is a push around the pipeline, which makes the commit an **orphan** (PB §5.5) and is covered by a reseal.

If one nevertheless exists — a hand-built envelope, an imported history — the indexer refuses the document and G9 records that landing `unattested`: *"reported and counted forever"* (PB §6.3). The ledger walk continues, no clone is bricked, and the count is visible in `spine stats`. That is the correct behaviour and it needs no new mechanism.

---

## 9. Worked example

### 9.1 The document

`intents/INT-042.md`, complete and in canonical form. The fence below is this specification's; the document is the 26 lines between the fence markers, each terminated by one `0x0A`, with no other byte after the last.

```markdown
# INT-042: Invoice totals include tax
Owner: @alice · Template: intent@2 · Ticket: https://tracker.example.com/T-1187 · Constitution: v3

## Goal (2–3 sentences)
Invoices show a tax-inclusive total, so finance stops reconciling two numbers by
hand. The total is computed from the line items the invoice already lists, and no
invoice that has already been issued changes retroactively.

## Non-goals (mandatory, minimum 2)
- Multi-jurisdiction tax rules. One rate, from the customer's billing country.
- Recalculating invoices that were already issued.
- A tax report or an export of one. Reporting is its own intent.

## Acceptance criteria (maximum 6 — more means split the task)
AC-1: Given an invoice with taxable lines, when it is rendered, then the total
  includes tax at the customer's rate.
AC-2: Given an invoice whose lines are all zero-rated, when it is rendered, then
  the tax line reads 0.00 and the total equals the subtotal.
AC-3: Given an invoice issued before this ships, when it is re-rendered, then its
  stored total is unchanged.

## Touchpoints (expected blast radius)
Expected to change: src/billing/, api/invoices.ts
Must NOT change: auth/, shared/schema/

## Open questions (optional — must be empty before implementation)
```

**Exact bytes.** 1258 bytes, 1249 characters, 26 lines. Every byte is ASCII except six characters, listed so the document is reproducible without copy-paste ambiguity:

| Character | Code point | UTF-8 | Count | Where |
|---|---|---|---|---|
| `·` | U+00B7 MIDDLE DOT | `c2 b7` | 3 | the header line's field separators |
| `–` | U+2013 EN DASH | `e2 80 93` | 1 | `(2–3 sentences)` |
| `—` | U+2014 EM DASH | `e2 80 94` | 2 | `(maximum 6 — more …)`, `(optional — must be empty …)` |

The document exercises: an optional `Ticket` field, an absent `Supersedes:` line, three heading parentheticals containing non-ASCII, a three-line prose Goal, three non-goals, three ACs each with one continuation line, both touchpoint labels with two patterns each, a directory pattern and a file pattern in one list, and an Open questions section that is present and empty with the heading as the document's last line.

### 9.2 Its identity

Computed, not asserted. Produced with git 2.50.1 in a repository whose `.gitattributes` carries §2.5's two-line form, by `git hash-object --path intents/INT-042.md`, and confirmed against `git rev-parse HEAD:intents/INT-042.md` after a commit.

| Quantity | Value |
|---|---|
| Byte length | `1258` |
| Blob id, `object_format = sha1` | `1b9e758012b85f788e3b3f16f6e81383bfdc54be` |
| Blob id, `object_format = sha256` | `1e594dc7885e7902d7e3125fc80394c53ef57aa716cf62119df0cea7be3cf39a` |
| `sha256sum` over the file's bytes | `b93064833e0e0fbf05ed39237dcab9dce1ed407b9a19373cc69749504a3b1d99` |

The last row is not a spine digest and appears in no trailer. It is published so a reader who reproduces the bytes can check them **without** a git repository, and then check their git installation against the row above it. Per PB §11's hash policy, the intent's identity is the git object id; `sha256:` is for non-git artifacts and an intent document is a git object.

The envelope's fence for this document reads `-----BEGIN SPINE-INTENT blob=1b9e7580… bytes=1258-----`, and the 1258 bytes include the single trailing `0x0A`. `envelope-vectors.md` owns the fence's own syntax; this document owes only that the fenced bytes are these bytes, unaltered.

### 9.3 Its parse

```json
{
  "id": "INT-042",
  "variant": "intent",
  "template": 2,
  "title": "Invoice totals include tax",
  "owner": "@alice",
  "ticket": "https://tracker.example.com/T-1187",
  "constitution": 3,
  "goal_present": true,
  "non_goal_count": 3,
  "acs": [1, 2, 3],
  "expected": ["src/billing/", "api/invoices.ts"],
  "forbidden": ["auth/", "shared/schema/"],
  "open_questions_empty": true
}
```

`variant` and `template` are read from the header rather than derived — `intent@2` splits into exactly those two members (§3.2) — and the id's `INT` prefix agrees with `intent`, so §3.3's consistency rule passes. `supersedes` is absent, the line not being present. The rendering above is illustrative JSON for readability; the parse result is a value, not a serialization, and this document defines no wire format for it.

The graph elements it yields, per PB §6.2 and `dump.md` §5.2, with `repo = myrepo`:

- nodes `myrepo/INT-042` (kind `intent`), `myrepo/INT-042/AC-1`, `/AC-2`, `/AC-3` (kind `ac`), `myrepo/code:src/billing/`, `myrepo/code:api/invoices.ts`, `myrepo/code:auth/`, `myrepo/code:shared/schema/` (kind `code_unit`), `myrepo/constitution:v3`;
- edges `has_ac` ×3; `declares` ×4, two with `{"polarity":"expected"}` and two with `{"polarity":"forbidden"}`; `built_under` ×1.

Provenance for the four `declares` edges, in flight, is `intents/INT-042.md:22` for the two expected and `intents/INT-042.md:23` for the two forbidden — the label line, not the pattern (§6.6).

### 9.4 A minimal document

The smallest thing this grammar accepts, as a second vector. It exercises an omitted `Ticket`, an absent Open questions section, exactly two non-goals, exactly one AC with no continuation, headings with no parenthetical, and an empty forbidden list written as a bare label line.

```markdown
# INT-001: Add a health endpoint
Owner: @alice · Template: intent@2 · Constitution: v1

## Goal
The service answers a liveness probe without touching the database.

## Non-goals
- Readiness, which needs the database.
- Metrics of any kind.

## Acceptance criteria
AC-1: Given the process is running, when GET /healthz is called, then it answers 200.

## Touchpoints
Expected to change: src/http/
Must NOT change:
```

| Quantity | Value |
|---|---|
| Byte length | `415` |
| Blob id, `object_format = sha1` | `59deb4027988c87c4423ced5a4eb74550b74a218` |
| Blob id, `object_format = sha256` | `bbab2c9ff6a30140eaa90faf910cedf473f2a0b0662497d2509447024eccde69` |
| `sha256sum` over the file's bytes | `66802409b97a1d0bff2d5aa43e19284f016d2a90089a7c91e781a27cdf45acd0` |

Its parse: `forbidden` is `[]`, `ticket` and `supersedes` absent, `non_goal_count` 2, `acs` `[1]`, `open_questions_empty` `true` (the section is absent). Its last line is `Must NOT change:` and its last byte is one `0x0A`.

### 9.5 Matching vectors

Produced by an implementation of §6.1–§6.3. The first row is the one the audit named.

| Pattern | Path | `match` |
|---|---|---|
| `src/bill` | `src/billing/x.ts` | no |
| `src/bill` | `src/bill` | **yes** |
| `src/billing` | `src/billing/x.ts` | **yes** |
| `src/billing` | `src/billing` | **yes** |
| `src/billing` | `src/billingx/y.ts` | no |
| `src/billing/` | `src/billing/x.ts` | **yes** |
| `src/billing/` | `src/billing/a/b.ts` | **yes** |
| `src/billing/` | `src/billing` | no |
| `src/billing/` | `src/billingx/y.ts` | no |
| `api/invoices.ts` | `api/invoices.ts` | **yes** |
| `api/invoices.ts` | `api/invoices.tsx` | no |
| `api/invoices.ts` | `api/invoices.ts/x` | **yes** |
| `src/*` | `src/a.ts` | **yes** |
| `src/*` | `src/a/b.ts` | **yes** |
| `src/*` | `src` | no |
| `src/**` | `src/a/b.ts` | **yes** |
| `src/**` | `src` | **yes** |
| `**` | `anything/at/all` | **yes** |
| `**/util.ts` | `util.ts` | **yes** |
| `**/util.ts` | `src/shared/util.ts` | **yes** |
| `**/util.ts` | `src/shared/xutil.ts` | no |
| `a/**/b` | `a/b` | **yes** |
| `a/**/b` | `a/x/y/b` | **yes** |
| `a/**/b` | `a/x/y/bc` | no |
| `src/**/__tests__/` | `src/a/__tests__/t.ts` | **yes** |
| `src/?.ts` | `src/a.ts` | **yes** |
| `src/?.ts` | `src/ab.ts` | no |
| `src/[abc]*.ts` | `src/b1.ts` | **yes** |
| `src/[abc]*.ts` | `src/d1.ts` | no |
| `src/[!abc]*.ts` | `src/d1.ts` | **yes** |
| `auth/` | `auth` | no |
| `auth/` | `authz/x.ts` | no |

Refusal vectors, all `bad-pattern` at exit 4 with the sub-status named:

| Pattern | Status |
|---|---|
| `src/**.ts` | `bad-globstar` |
| `a**b` | `bad-globstar` |
| `!src/` | `bad-negation` |
| `/src/` | `leading-slash` |
| `src//a` | `empty-segment` |
| `src/./a`, `src/../a` | `dot-segment` |
| `src/[abc` | `bad-bracket` |
| `x[:alpha:]y`, `x[[:alpha:]]y` | `bad-bracket` |
| `a,b`, `a"b`, `a\b`, `a b`, `é/x` | `pattern-illegal-byte` |
| *(empty)* | `pattern-empty` |

Accepted, to pin the boundary: `a*b*c`, `sr*c/**`, `**`, `src/*/x`, `src/[!abc]*.ts`, `src/[]]x`, `docs/`, `a/**/b`.

### 9.6 Overlap vectors

`litprefix` and `overlap` of §7.3.

| P | Q | `litprefix(P)` | `litprefix(Q)` | soft lease |
|---|---|---|---|---|
| `src/billing/` | `src/billing/tax.ts` | `src/billing/` | `src/billing/tax.ts` | **overlap** |
| `src/bill` | `src/billing/` | `src/bill` | `src/billing/` | disjoint |
| `src/a/` | `src/b/` | `src/a/` | `src/b/` | disjoint |
| `docs/` | `src/` | `docs/` | `src/` | disjoint |
| `api/invoices.ts` | `api/invoices.ts` | `api/invoices.ts` | `api/invoices.ts` | **overlap** |
| `src/*/a.ts` | `src/*/b.ts` | `src/` | `src/` | **overlap** (over-approximate) |
| `a*/x` | `ab/x` | *(empty)* | `ab/x` | **overlap** |
| `a*/x` | `cd/x` | *(empty)* | `cd/x` | **overlap** (over-approximate) |
| `**/util.ts` | `src/` | *(empty)* | `src/` | **overlap** (over-approximate) |

The three over-approximate rows are the price of soundness without a pattern-intersection procedure, and they cost a notification each. The `a*/x` rows show why the truncation in `litprefix` is not optional: the first is a true overlap that a non-truncating definition would miss.

**Soundness check, run.** Over all 2 926 patterns generated from segment alphabet `{a, b, ab, abc, x, *, **, ?, a*, [ab], a?}` at depths 1–3, with and without a trailing `/`, filtered to those §6.1 accepts, against all 399 paths over `{a, b, ab, abc, x, ax, ba}` at depths 1–3: for every path, every pair of patterns that both match it satisfies `overlap`. **0 violations.**

---

## 10. Determinism rules, collected

1. **The parse is a function of two inputs**: the document's bytes and the id from its path (§3.4). Not the tree, not the manifest, not the environment, not the local git version. In particular the **variant is read, not inferred**, from the header's own bytes (§3.2); the legacy derivation of §3.3 runs only for the bare `v<n>` spelling, and it too reads nothing but the document.
2. **No clock.** No member of the parse result is a time, a duration or a date, and no rule consults one.
3. **No normalisation, no casefolding**, except the three casefolds §2.1 enumerates.
4. **No tree lookup.** A pattern is never expanded to the paths it currently matches (§5.4); a directory is distinguished from a file by a trailing `/` and never by a stat (§6.3).
5. **No Markdown.** The line model of §4.1 has no state a fence or a list can open.
6. **Closed sets refuse.** An unknown section, an unknown header field, an unknown template **variant**, an unknown template version, an unknown touchpoint label: refuse, never ignore, never carry opaque (§4.9, §3.2).
7. **One failure order.** §8.2 fixes it, so a document wrong in several ways has one status.
8. **Bounded work.** §2.3's limits are normative and make the parse linear in document length, because another branch's document is parsed during my landing (§1, §7.4).
9. **`esc` and `tok` are the identity on every legal pattern** (§6.1), so a pattern's bytes, its node id suffix and its wire token coincide.
10. **Two parsers agree iff they produce §5.6's value.** Everything else — retained texts, diagnostics, layout — is free.

---

## 11. Resolved ambiguities

Each entry states what the playbook says, what this document chose, and why.

### 11.1 There is no parse grammar at all

**Playbook:** PB §3.1 shows a filled template in a `markdown` fence; PB §3.3 constrains the bytes; PB §6.2 says historical intents are *"parsed by the `Template:` version's parser"*. No section is located, no field typed, no terminator given.
**Chosen:** a line-oriented, non-Markdown grammar (§4): level-2 ATX headings at column 0 locate sections, a heading's key is the text before the first `(`, casefolded (§4.7), a section runs to the next heading, the table is closed and ordered, and body line classes are fixed by leading bytes (§4.10).
**Why:** the alternative is "parse the Markdown", which means picking a CommonMark implementation and inheriting every corner of it — lazy continuation, setext headings, link reference definitions, four-space code blocks, tab expansion. Two implementations in four languages will not agree on those, and PB §1.1's offline re-verification requires that they do. The cost is that a `## ` line inside an intended code fence starts a section; the benefit is that the failure is a refusal rather than a silent difference in what was declared.

### 11.2 Section order, and whether an unknown section is fatal

**Playbook:** the template shows a fixed order and never says whether it is required; nothing says what an extra section does.
**Chosen:** order is enforced (`section-order`), unknown sections are refused (`unknown-section`).
**Why:** order gives PB §4.3's `resign` reopen — *"inserts each new mandatory section as an empty stub"* — a defined insertion position, which it otherwise lacks. Refusal follows `gate-report.md` §3.2's rule for the same reason it gives, plus one specific to this artifact: a tolerated `## Touchpoint` beside the mandatory `## Touchpoints` is two declarations of blast radius, and a tolerant parser and a strict one disagree about which is the lease.

### 11.3 Whether EARS phrasing is checked

**Playbook:** PB §3.1 shows `Given … when … then …`; PB §1.1 lists *"EARS-style phrasing for acceptance criteria"* as adopted from Kiro; PB §3.4 makes AC verifiability the interview's job.
**Chosen:** advisory. Never checked, and the AC's text is not a member of the parse result.
**Why:** EARS is five patterns, of which the template shows one, and deciding whether an English sentence belongs to the family is a judgement with no byte-exact specification. A checker that enforced it would refuse documents another accepts. The playbook already assigns the check to a human process (the interview) and to an adversarial one (Agent B); mechanising it would be inventing a gate the playbook does not have. This is consistent with PB §6.2's schema, which holds no AC text.

### 11.4 The AC cap has no floor, and no gate enforces either bound

**Playbook:** PB §3.1 says *"maximum 6 — more means split the task"*; PB §3.2 says the cap means *"nobody has to police scope in review"*. No minimum. No gate in PB §6.3 and no row in PB §6's table counts ACs.
**Chosen:** 1 … 6, contiguous from 1, in order, Layer 1, refused by every consumer (§5.3).
**Why:** *"nobody has to police scope"* is a claim about a machine check, so there must be one, and the parse is the only place all four consumers share. The floor of 1 is added because a zero-AC intent makes `--approve`'s coverage guard and G1's coverage clause both vacuous and asks a human to sign a document that promises nothing testable. Contiguity is enforced because the id is the join key to pragmas, test names and node ids, and it also makes the cap mechanical: with contiguity, `AC-7` cannot exist.

### 11.5 "Open questions must be empty" — when, and what counts

**Playbook:** PB §3.1 marks the section *"optional — must be empty before implementation"*; PB §3.2 says it *"converts 'the agent assumed' into 'the agent asked'"*; PB §6's table makes `spine new` the guard on `draft → awaiting-sign-off`.
**Chosen:** empty means **no non-empty line of any kind** — prose counts, `- (none)` counts. Emptiness is a **Layer 2** sign-off precondition checked by `spine new --sign` alone (§8.1); the parse accepts a non-empty section, and so do `--approve`, `--land` and the indexer.
**Why:** the strict reading of "empty" is the only one that serves the stated purpose — a section with words in it has words in it. The layering is forced by two facts pulling opposite ways: a document with open questions is the *normal* state of a draft, so it must parse; and a landed envelope must be readable for archaeology forever, so a stage condition cannot be a parse condition. Splitting stage from shape resolves both, and §12 D5 files the playbook's mis-assignment of the guard.

### 11.6 Touchpoints are patterns, not path sets

**Playbook:** PB §6.2's `code_unit` example is `code:src/billing/` — a pattern; PB §6.3's G2 query compares `modifies` node ids against `declares` node ids with `NOT IN` — which needs path sets.
**Chosen:** a pattern becomes exactly one `code_unit` node, written as declared, never expanded (§5.4, §6.6). G2 is a match predicate over `match` (§7.1), not a set membership test.
**Why:** expansion makes the parse a function of the tree, so a landed intent's `declares` edges would differ between an index taken at landing and one taken later, and G10's byte equality between two clones would depend on which tip each was indexed at. It also loses information: `src/billing/` and the six files under it today are different claims. PB §6.3's query is a defect under this reading and §12 D4 files it.

### 11.7 Non-ASCII touchpoint patterns

**Playbook:** silent. Git paths are byte strings and may be any bytes but `/` and NUL.
**Chosen:** a pattern is ASCII, `0x21…0x7E` minus `,`, `"`, `\` (§6.1). A non-ASCII pattern is `pattern-illegal-byte`.
**Why:** a declared non-ASCII pattern is typed by a human on one platform and compared against bytes git produced on another, and macOS and Linux disagree about NFC/NFD for exactly those bytes. The declaration would silently fail to match, which is the worst available outcome for a containment gate — a forbidden path that quietly matches nothing. Refusing is loud, and the workaround is an ASCII ancestor directory. Note that this restricts only *declarations*: paths in the diff may be any bytes, are matched as bytes, and are carried through `esc` wherever a report or a node id needs them. OPEN-2 asks whether the owner wants the restriction lifted.

### 11.8 A path matching both polarities

**Playbook:** PB §6.3 G2 states two clauses — containment in `expected`, and *"any `forbidden` hit is a hard fail"* — as though independent.
**Chosen:** forbidden is evaluated first and dominates; a path in both is reported once, as a forbidden hit (§7.1). A byte-identical pattern in both lists is refused outright at parse (`polarity-conflict`).
**Why:** `expected: src/`, `forbidden: src/auth/` is the natural way to write *"this subtree except that"* and is only coherent under this precedence. Reported once, because two wires for one path over one gate would collapse anyway under `gate-report.md` §6.1's uniqueness rule, and the collapsed entry must be the finding, not the containment miss.

### 11.9 Template v1's content, and the legacy bare `Template: v<n>` spelling

**Playbook:** PB §3.1 says *"Template v1 files still parse; their `Status:` field is ignored"* and gives no v1 template. PB §3.4, after decision 4, stamps `Template: <variant>@<n>` and says nothing about what happens to a document carrying the bare form the playbook itself printed through v0.18.
**Chosen:** v1 is v2 plus a permitted `Status` header field, parsed and discarded; and the bare `v<n>` spelling is a **legacy value**, accepted at `n ∈ {1, 2}` only, its variant supplied by §3.3's derivation (§3.2).
**Why:** both promises need referents and neither has one. It is safe to create them rather than reconstruct them, because **no document at version 1 and no document carrying the bare spelling exists in any repository** — no release has shipped, so no `spine new` has ever stamped either. Bounding the legacy form at `n ≤ 2` is what stops it becoming permanent: every version from 3 on has exactly one spelling, so a reader never has to decide which of two forms a future document meant, and the derivation rule — whose failure mode is a Change intent silently becoming a Feature intent (§3.3) — is confined to a set of documents that is empty today and can only stay empty. If the owner intended v1 to differ further, the difference must be written down before the first release, after which it is history. §12 D2 and §12 D9 file the two promises.

### 11.10 The `Supersedes:` line's `(optional)`

**Playbook:** PB §3.1's template block reads `Supersedes: INT-017                        (optional)`.
**Chosen:** the value is one id and nothing else; the parenthetical is annotation and a document carrying it is `bad-supersedes` (§4.4).
**Why:** it is annotation in the same block implementers transcribe, and PB §12 records this exact defect class biting v0.14 twice. Filed as §12 D7 so the block is corrected rather than the grammar loosened.

### 11.11 Whether the title has a bound

**Playbook:** `# INT-042: <short imperative title>`; "short" is not a number.
**Chosen:** 1 … 72 bytes, hard (`title-too-long`, §4.2).
**Why:** the title is the landing commit's subject after PB §5.5's `<ID>: ` prefix, so 72 keeps it inside 81 columns; and a bound is what makes PB §5.5's 16 KiB envelope projection computable at `--approve` from the parse alone rather than by rendering. It is a refusal rather than advice because an unbounded field in a signed artifact is an unbounded field — and, since decision 6 makes G9 recompute the subject from these bytes (§4.2), an unbounded title would be an unbounded gate input as well.

### 11.12 `Spine-` case sensitivity

**Playbook:** PB §3.3 refuses *"any line beginning `-----` or `Spine-`"*, in the document's own capitalisation.
**Chosen:** the `Spine-` test is ASCII case-insensitive; the `-----` test is exact (§2.2).
**Why:** `git interpret-trailers` matches trailer tokens case-insensitively, so `spine-seal: x` inside the fenced block is a line some reader treats as a trailer of `L`. The refusal exists to keep the document out of the envelope's syntax, and half a refusal does not.

---

## 12. Defects found in PLAYBOOK.md v0.19

Reported here rather than repaired, per `docs/spec/README.md`: where this document and PB §11 disagree, §11 wins and the disagreement is a defect in one of them. None of these is in §11. **Citations are section anchors plus a verbatim quote, never line numbers** — a line number rots as the playbook grows, a section and a quote do not. Every entry is marked **OPEN** or **CLOSED** against `PLAYBOOK.md` as it now stands.

**D1 · CLOSED by PLAYBOOK.md v0.19 — PB §3.3 now writes two lines. The measurement is kept because it is the only place the cost is quantified.** As filed against the single-line form: *"`spine init` writes `.spine/** intents/** text eol=lf` to `.gitattributes`. The point is mechanical: the bytes in the envelope equal the bytes in the blob, so the intent's identity — its git blob id — is recomputable from the envelope alone."* A `.gitattributes` line is one pattern followed by attributes. Git reads the pattern `.spine/**` and the attribute names `intents/**`, `text`, `eol=lf`; `intents/**` is not a valid attribute name, and git **discards the whole line**. Measured with git 2.50.1: `git check-attr text intents/INT-042.md` reports `unspecified` and prints `intents/** is not a valid attribute name: .gitattributes:1` on every attribute lookup in the repository; `.spine/**` loses `text eol=lf` too. With a CRLF worktree copy of §9.1's document, `git add` under the malformed line stores blob `ca273d5ddadfd15b071898bd3ef8439778342306`; under the corrected two-line form it stores `d16fee86b4f3a8c614b054ff9de9680ad78b1bf3`, the canonical blob. So the sentence *"so `core.autocrlf` cannot fork the identity"* was false as written: on a checkout whose editor wrote CRLF, the intent blob forked, §2.1 rule 6 refused it, and that developer could not sign an intent at all — while `.spine/allowed_signers` and `.spine/ci.sh`, whose blobs G16 compares against the manifest, forked the same way. **The fix asked for was two lines, one pattern each (§2.5), and PB §3.3 adopted it**, adding the git-2.50.1 rejection message and the observation that *neither* pattern survives the malformed line. `constitution.md` §15 D7 is the residue and is a different path: `paths.constitution` is covered by neither line.

**D2 · CLOSED by the owner, decision 4 of PB v0.19 · `Template:` names the variant.** The defect was that `Template: v<n>` could not name which of three templates it was. PB §6.7's manifest carries three independent versions — `"templates": { "intent": 2, "intent-change": 2, "intent-bug": 2, … }` — and three independent `resign` floors; two documents both reading `Template: v2` could be a Feature and a Change with different mandatory sections, G4 compared *"a template version below the manifest's `resign` floor"* with no way to choose which of the three floors applied, and the indexer's *"parsed by the `Template:` version's parser"* did not say which parser. The recommendation here was (b) — `Template: intent-change@2` — and it is what the owner took: PB §3.1's template block now stamps *"`Owner: @name · Template: intent@2 · Ticket: <link> · Constitution: v3`"*, PB §3.4 states the reason (*"it names the **variant as well as the version** (`intent@2`, `intent-change@2`, `intent-bug@2`), because G4 must index the `resign` map by variant"*), and `Spine-Signoff`'s payload is `template=<variant>@<n>`. §3.2 gives the grammar, §3.3 makes selection a read and adds the `variant-prefix-mismatch` check the old derivation had no way to perform, and §11.9 bounds the legacy bare spelling at version 2 so it can never become permanent. The one residual is the digest one: the header's bytes are inside every intent document, so every blob id, `freeze=`, `envelope=` and example digest covering them changed. §9.2, §9.4 and §15 are recomputed here. **The sibling pass this entry once owed is done**: `envelope-vectors.md` publishes no `template=v2` line, and `gate-report.md`'s two remaining occurrences are in §8's withdrawal record, describing a superseded computation rather than publishing a line.

**D3 · OPEN · Nothing enforces the non-goal minimum or the AC maximum** (PB §3.1's template block, *"`## Non-goals (mandatory, minimum 2)`"* and *"`## Acceptance criteria (maximum 6 — more means split the task)`"*; PB §3.2, *"**Non-goals are mandatory (minimum two)**"* and *"**Acceptance criteria are capped at six**"*). No gate in PB §6.3 counts either, no row of PB §6's transition table guards either, and PB §4.3's approval record — *"`spine check --approve INT-042`, which refuses a dirty worktree and freezes the branch HEAD's tree"* — names neither. By the playbook's own governing rule — *"every rule must be enforced by a machine, not by discipline"* — these are currently discipline. §5.2 and §5.3 assign them to the parse, which is the only place all four consumers share. **Recommended:** PB §3.3 names `spine new --sign` as the enforcer, or PB §6 gains a row.

**D4 · CLOSED · PB §6.3's G2 query failed every landing that changes a file inside a declared directory** (PB §6.3's G2 SQL block). **As filed**, the query read:

```sql
AND m.to_id NOT IN (SELECT to_id FROM edges
  WHERE from_id = 'myrepo/INT-042' AND kind = 'declares'
  AND json_extract(attrs,'$.polarity') = 'expected');
```

`modifies` targets are concrete paths (PB §6.2's derivation table: *"`git diff --name-only B L` — the integrated delta G2 gates on"*), so `code:src/billing/tax.py`; `declares` targets are the doc's touchpoints, so `code:src/billing/` (PB §6.2's own node-id example). `NOT IN` is byte equality, and those two strings are never equal — so the query returned every modified file, every tripwire fired, and no landing with a directory touchpoint was clean. The only reading that saved the query was expanding `declares` to concrete paths at index time, which §11.6 rejects because it makes the graph a function of the tree and breaks G10's byte equality. The fix asked that the query be a match predicate over PB §5.2's matcher rather than a set-membership test. **Taken:** PB §6.3's block now reads `AND NOT EXISTS (SELECT 1 FROM edges d … AND spine_match(d.to_id, m.to_id))`, and its own comment opens *"`spine_match` is the touchpoint matcher, not equality"*, gives this defect's argument as the reason, and delegates the semantics — segment-boundary, never byte-prefix — to this document. The `forbidden` clause and the frozen/spine-owned/floor exemptions this defect also asked for are carried by PB §6.3's G2 row rather than by the illustrative query.

**D5 · OPEN · The transition table makes `spine new` the guard on a condition it cannot observe** (PB §6's transition table: *"`| draft | interview complete; Open questions empty | awaiting-sign-off | `spine new` |`"*). `spine new` *creates* the document from the template, before a human has written a word of it; the Open questions section it scaffolds is the only one it will ever see. The only command that can observe the condition is `spine new --sign`, and PB §3.4's description of `--sign` does not mention it. §8.1 assigns it to `--sign`. **Fix:** one word in the Enforced-by column.

**D6 · OPEN, narrowed · The touchpoint pattern language is undefined and is shared with the constitution without saying so** (PB §5.2, *"In v1, touchpoint checks are path-prefix matching"*, which gives no dialect). PB §2.1's scaffold now writes `C-Q1: quick.paths = docs/` and `C-T1: test.roots = <per params.langs>` — the `**` globs this defect quoted against v0.19 moved into `constitution.md` §6.4's per-language table with the scaffold's normalisation, which narrows the evidence and does not close the defect: those values are still constitution patterns, still a different notation from a touchpoint, and still evaluated by the same G2 (PB §6.3's G2 row: *"quick lane: ⊆ `C-Q1` ∪ floor ∪ spine-owned paths"*) against the same diff, in one set operation that also mixes in the floor's own casefolding any-depth matcher (PB §7.3). Three notations, one set expression, no shared definition. §6 defines one dialect for touchpoints and `constitution.md` must adopt it verbatim (§6.7); the floor stays separate on purpose and PB should say that it does. **Fix:** PB §5.2 cites a dialect; PB §7.3 states that the floor's matcher is deliberately not it.

**D7 · OPEN · The `Supersedes:` template line carries an annotation inside the transcribed block** (PB §3.1's template block: *"`Supersedes: INT-017                        (optional)`"*). A reader implementing from the block accepts the parenthetical as part of the value, or writes it into a scaffold. PB §12 records this exact class biting v0.14 in the envelope block and in §10's closing paragraph. **Fix:** move the annotation to a comment beside the block, as PB §3.1 does for the heading hints, which are inside parentheses the parser is told about.

**D8 · CLOSED by the owner, decision 5 of PB v0.19 · The `Must NOT change: **` defence closed half the case it names** (PB §5.4's *Leases* paragraph: *"a lease is derived only from the blob named in a verifying `Spine-Signoff` on that branch: an unsigned or revoked branch contributes no lease (or any pushed branch could declare `Must NOT change: **` and halt every landing)"*). The signature closes the anonymous case. It does not close the authorised one: any signer may create `intent/INT-999`, sign a document declaring `Must NOT change: **`, and thereafter every landing in the repository takes a `class=protected` G7 review — reviewer ≠ signer in team mode. The only remedy the playbook names, `spine new --sign --override-lease`, is a flag on the *other* intent's sign-off, and PB §5.4's hard-lease bullet says *"the lease still trips at landing"*, so it does not help. The exposure is bounded by ceremony rather than by refusal, and nothing counted it. **Closed on the recommended terms:** the residual is stated where the parenthetical is, and `spine stats` gains a counter for landings whose only protected wire is a G7 hard lease (§7.2). The grammar is **not** narrowed — an unbounded `forbidden` set stays legal (§6.1, §13 OPEN-3) — so the fix here is a sentence in PB §5.4 and a counter, not a refusal. **Fix:** PB §5.4's parenthetical says that the authorised case is bounded by the signature and by the counter, not by the grammar; PB §11's `spine stats` list gains the counter.

**D9 · OPEN · A compatibility promise with no referent — now two of them** (PB §3.1, *"**There is no `Status:` line — v2 removed it.**"*: *"Template v1 files still parse; their `Status:` field is ignored."*). Decision 4 adds a second: every `Template: v2` document the playbook printed through v0.18 is now a legacy spelling, and PB does not say whether one parses. No v1 template body exists anywhere in the playbook, and PB §6.2's derivation table requires *"the `Template:` version's parser"*. §11.9 defines v1 as v2-plus-`Status`, which is safe only because no release has shipped and therefore no v1 document exists. **Fix:** either write v1 down before the first release, or delete the promise and let `spine new` start at v2 — the second is cheaper and loses nothing. Either way PB §3.1 should say in one clause what a bare `Template: v<n>` means now that the header carries a variant; §3.2 and §11.9 answer it as *legacy, `n ≤ 2`, variant derived*, which is the reading a reader of PB alone cannot reach.

**D10 · OPEN · PB §3.1's `Owner:` and PB §6.2's `owner` attr disagree about whether the field may be absent** (PB §3.1's template block, *"`Owner: @name · Template: intent@2 · Ticket: <link> · Constitution: v3`"*, with no optional marker, against PB §3.1's prose, *"`Owner:` is a hint for humans; `signed_by` in the graph is the truth"*); `dump.md` §7.2 records `owner` as present *"iff the doc has an `Owner:` field"*, which reads the playbook as permitting absence. §4.3 makes it mandatory and `dump.md`'s conditional then never fires. **Fix:** one word in PB §3.1's template block or in PB §3.2 saying which. Low cost, and it is the kind of disagreement that produces two different graphs over one document.

---

## 13. OPEN — the owner's calls

**OPEN-1 · Closed by the owner, 2026-08-26: `Template:` names the variant.** The three ways out were (a) leave §3.3's derivation rule, (b) `Template: intent-change@2`, (c) a fourth header field `Variant:`. The owner took **(b)**, this document's recommendation, and PB v0.19 §3.4 gives the reason in the mechanism rather than in taste: *"G4 must index the `resign` map by variant and the indexer must pick a parser by name, and neither is decidable from a bare `v2`"*. §3.2 is the grammar, §3.3 is selection plus the prefix-agreement check, §11.9 bounds the legacy spelling. What (a) would have cost is now on the record as the reason (a) lost: a Change intent whose Invariants section is renamed derives to `intent` and is refused for the wrong reason, and a `--bug` document under an `INT-` id had no detector at all. Because it was decidable now or never — every sealed intent carries the header for ever — it was decided now, and this section is closed rather than standing.

**OPEN-2 · Whether a touchpoint pattern may be non-ASCII.** §11.7 restricts patterns to `0x21…0x7E` minus three bytes, so a repository with non-ASCII paths cannot name one directly; the workaround is an ASCII ancestor. Lifting it means deciding what a declared non-ASCII pattern is compared against when the diff's bytes are NFD and the author's editor wrote NFC — and every answer except "compare bytes, and accept that it will silently fail to match" costs a normalisation step that §2.1 forbids everywhere else. **Recommendation: keep the restriction, and make the refusal message name the workaround.** Owner-level because it is a capability limit visible to users, not a serialization choice.

**OPEN-3 · Closed by the owner, 2026-08-26: an unbounded `forbidden` set stays legal, and `spine stats` counts the exposure.** §12 D8's residual. The options were (a) leave `**` and `*` legal at the root and rely on review plus a counter, (b) refuse a `forbidden` pattern whose `litprefix` is empty, (c) cap the count of in-flight intents whose forbidden sets a landing must clear. This document recommended (b); the owner took **(a)**, with (a)'s counter, and the reasoning is worth recording because it overrides the recommendation on a point the recommendation under-weighed: **a human signs the declaration**, so the breadth is an authored, attributable claim rather than an anonymous one, and **both polarities take the same patterns from the same dialect**, so (b) would have forked the pattern language — `expected` accepting `**` and `forbidden` refusing it, with `constitution.md` adopting §6.1–§6.3 verbatim (§6.7) and having to adopt the fork with it. §6.1 states the decision where the `**` note is, §7.2 defines the counter, and the shape of the answer is the playbook's own elsewhere: an exposure that is signed and counted rather than refused.

**OPEN-4 · Whether the AC cap and the non-goal minimum admit a signed override.** §5.2 and §5.3 make both hard refusals with no escape, on the reasoning that a cap with an escape hatch is advice. The playbook's own pattern elsewhere is the opposite: a closure tripwire, a `red=0/n` approval and a lease collision are all *signed, counted* overrides rather than refusals, and `spine stats` turns each into evidence. An `--override-scope "<reason>"` on `--sign`, recorded on the sign-off line and counted, would match that pattern. **Recommendation: keep them hard.** The overrides the playbook admits elsewhere all concern facts about *code* that a human can weigh; these concern the shape of a document the same human is writing, and the remedy — split the task — is the outcome the cap exists to produce. Owner-level because it is PB §3.2's own argument being taken at full strength.

---

## 14. Out of scope

Deliberately not specified here, and where it belongs instead:

- **The Change and Bug template bodies** — which sections, in which order, mandatory or optional, with which body grammar: `docs/spec/templates.md`. It owes two rows of §4.8's table and nothing else; it **must not** define a body grammar, a header field, a pattern dialect or a matching rule of its own, and §4.8's shape plus §5's field grammars govern all three variants. This document owns the grammar all three share, which is what makes §3.3's variant selection safe.
- **The envelope's grammar** — the fence's syntax, the byte range `bytes=` counts, trailer folding, what a `-Sig` covers, and the 16 KiB cap's exact measurement: `docs/spec/envelope-vectors.md`. §9.2 gives the fenced bytes and their blob id so that document has a vector to build on; it does not define the fence.
- **The constitution's grammar**, including how `C-Q1`'s and `C-T1`'s pattern lists are split and how `enforced_by` parses: `docs/spec/constitution.md`. **That document must adopt §6.1–§6.3 verbatim** (§6.7), and the dependency is normative, not decorative: G2's quick-lane clause evaluates a constitution list and an intent list in one set expression, so a second dialect makes one diff both inside and outside the lane.
- **The manifest's grammar**, including `repo` (which `dump.md` §5.2 constrains to `^[A-Za-z0-9._-]+$`), the `templates` and `resign` maps §3.2 and §8.1 read — both keyed by the three variant tokens `Template:` carries — and `params.langs`, whose v1 values are `python`, `ts`, `dart` and `swift`: `docs/spec/manifest.md`, §3.6 for the two maps and §6.2 for the G16 checks that hold them.
- **The floor list's contents and its matcher.** PB §7.3's any-depth, casefolding rules are the release's, deliberately not §6's, and §6.5 and §6.7 say why they should stay apart.
- **`esc`** — `gate-report.md` §2.3 owns it. **The wire token `tok` and the `wires` array's order** — `gate-report.md` §6.1–§6.2 own them; §7.1 produces wires and does not order them.
- **The `code_unit` node id's own grammar and the `src` provenance productions** — `dump.md` §5.2 and §5.4. §6.6 supplies the pattern bytes and the line number those productions need.
- **Gate semantics beyond the two predicates.** What enters `Δ`, how the exempt set `X` is computed, which branches are in flight, what a wire's `class` is, how warn mode is selected, how the review state is derived: PB §5.2, PB §5.4, PB §6.3 and PB §11. §7 fixes the predicates that read the parse and nothing downstream of them.
- **The pragma `@verifies INT-042/AC-1` and the `test_AC<n>` naming sugar**, and the **source-symbol → runner-native-id join** they both assume: `docs/spec/import-resolver.md` **§12**, where all three are now written — §12.1 the pragma's grammar, §12.2 the file-granular join, §12.3 the naming sugar per runner. §12.1 adopts **this document's §3.1 intent-id domain** by name, and version 2 of that document's own wider `^(INT|BUG)-[0-9]+$` is withdrawn as a second id domain; so the two documents share one spelling of `INT-042`. This document fixes the AC id that the pragma's right-hand side must name; it does not fix how a pragma is found in a blob, which is language-specific.
- **The interview agent** — PB §3.4's seven questions, its non-goal extraction, its EARS coaching, and `spine eval`'s golden set. The document this specification parses is that agent's output; how it is produced is a prompt in the binary, and PB §6.7 makes prompts a release, not a repo edit.
- **`spine new`'s scaffolding** — which bytes it writes for a fresh intent, how it allocates an id, how it fetches before allocating. One normative constraint reaches it from here (§5.5: the scaffolded Open questions body must be empty); the rest is `templates.md`'s and the CLI's.
- **Rendering.** How a reviewer's packet displays an intent, how `spine context` scopes it, what a PR body shows. PB §6.1's provenance law binds renderings; nothing in spine reads one, and this document defines no output format for the parse result (§5.6).
- **Storage and transport.** The document lives at `intents/<ID>.md` on one branch and as fenced bytes in one commit message. It is never a note, never fetched from a provider, never cached.

---

## 15. Conformance checklist

A parser conforms iff all of the following hold. Every item is mechanically checkable.

**Canonical form**

1. A document containing `0x0D`, `0x00`, `U+FEFF`, any C0 control other than `0x09`/`0x0A`, or `0x7F` is refused with the §2.1 status, exit 2.
2. A document not ending in exactly one `0x0A`, or with a line ending in space or tab, is refused, exit 2.
3. A line beginning with five `-` is refused; a line whose first six bytes lowercase to `spine-` is refused; both at exit 2.
4. A document over 65 536 bytes, or a line over 4 096 bytes, is refused, exit 2.
5. No input is Unicode-normalised, and no document is refused for being un-normalised.
6. Casefolding is applied to exactly three things: a section key, a touchpoint label, and the `Spine-` test.

**Grammar**

7. Line 1 is the only line beginning `# `; the id in it equals the id from the path; the title is 1–72 bytes.
8. Line 2 is the header line; its fields are from the closed table, in table order, without repeats; `Owner`, `Template` and `Constitution` are present. `Template:`'s value is `<variant>@<n>`, its variant token matched byte-exactly against `intent` / `intent-change` / `intent-bug`, or the legacy bare `v<n>` at `n ∈ {1, 2}`; a bare `v3` or higher, and a variant token outside the three, are refused (`bad-template`, `template-variant-unknown`), exit 4. The variant is read from that token and derived only for a legacy value (§3.3), and the id's prefix agrees with it — `BUG` with `intent-bug`, `INT` with the other two — or `variant-prefix-mismatch`, exit 4.
9. A `Supersedes:` line, if present, is line 3 and carries exactly one id and nothing else.
10. A heading is a line whose first three bytes are `## `; `###` and `##Goal` are not headings; a section ends at the next heading or at end of document.
11. A heading's key is the text after `## `, trimmed, truncated at the first `(`, trimmed again, ASCII-lowercased.
12. Unknown, duplicate, missing and misordered sections are all refused, exit 4. None is ignored.
13. A `(variant, version)` pair the binary holds no parser for is refused with `template-version-unknown`, exit 3, before any section is examined, and after the step-4 checks of §8.2 — never by substituting another variant's parser for the same number.

**Fields**

14. Goal has at least one non-empty line and no bullet or AC line; its text is not in the parse result.
15. Non-goals has 2 … 256 bullets and no prose; a bullet's text is not in the parse result.
16. Acceptance criteria has 1 … 6 AC lines numbered exactly `1 … k` in order; a line beginning `AC-` that does not match the grammar is refused; an indented `AC-` or `- ` is refused; AC text is not in the parse result.
17. Touchpoints carries exactly one of each label, ASCII-case-insensitively matched, and no other non-empty line; `Expected to change` has ≥ 1 pattern; a pattern in both polarities is refused.
18. Open questions is empty iff its body has no non-empty line; a non-empty section parses and is refused only by `spine new --sign`, exit 5.

**Patterns and matching**

19. Every pattern is 1–255 bytes from `0x21…0x7E` minus `,`, `"`, `\`; every refusal of §6.1's table fires with its own status.
20. `*` never crosses `/`; `**` crosses and appears only as a whole segment; a `**` that is not a whole segment is refused, not reinterpreted.
21. Matching is byte-exact and case-sensitive; no path or pattern is normalised or casefolded.
22. `match` reproduces every row of §9.5, `src/bill` × `src/billing/x.ts` = no included.
23. A trailing `/` makes a pattern match strictly under the directory and never the directory's own path.
24. `overlap` reproduces every row of §9.6 and is sound: for any path, any two patterns matching it overlap.

**Determinism and identity**

25. Two runs over the same bytes and the same path id produce the same parse result.
26. The parse consults no tree, no manifest, no environment variable, no locale and no clock.
27. §9.1's bytes — 1258 of them — hash to `1b9e758012b85f788e3b3f16f6e81383bfdc54be` (sha1) and `1e594dc7885e7902d7e3125fc80394c53ef57aa716cf62119df0cea7be3cf39a` (sha256), and parse to §9.3's value.
28. §9.4's bytes — 415 of them — hash to `59deb4027988c87c4423ced5a4eb74550b74a218` (sha1) and `bbab2c9ff6a30140eaa90faf910cedf473f2a0b0662497d2509447024eccde69` (sha256), and parse with `forbidden` empty.
29. A document that is wrong in several ways reports the status of the earliest step of §8.2 that fails, and within a step the first failure in line order.
30. A document another branch offers that fails any of the above contributes no lease and does not fail the landing that read it (§7.4).
31. A `forbidden` pattern of any breadth, `**` included, parses (§6.1); `spine stats` counts landings whose only protected wire is a G7 hard lease (§7.2).
