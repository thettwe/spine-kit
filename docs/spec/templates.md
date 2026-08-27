# The three intent templates

**Artifact:** the bytes `spine new` writes to `intents/<ID>.md`, in each of the three template variants PB §3.5 ships — `intent`, `intent-change`, `intent-bug` — together with each variant's section table, its version stamp, and what `spine new --reopen` does to it when a `resign` floor moves.
**Home in the playbook:** PB §3.1 (the template, `intent@2`), PB §3.5 (two lanes, three templates), PB §6.7 (template versioning, the `resign` floor, and the rule that the binary keeps a parser for every template version ever shipped), PB §4.3 (`--reopen`), PB §6.3 (G4, G12). Vocabulary from PB §11, which wins over prose here as it wins there.
**References:** `PB §n` cites `PLAYBOOK.md`; `ID §n` cites `docs/spec/intent-doc.md`; a bare `§n` cites this document. `esc` is `gate-report.md` §2.3's; `tok` is its §6.2's; the runner tokens are `import-resolver.md` §11.1's.
**Spec version:** 1 · **Template versions specified:** `intent@2`, `intent-change@2`, `intent-bug@2`, with `<variant>@1` and the legacy bare `Template: v<n>` spelling as compatibility targets (§9.2, ID §3.2) · **Covers:** PLAYBOOK.md v0.19 · **Status:** normative for v1. It is subordinate to `intent-doc.md`, which owns the grammar all three variants share.

---

## 1. What this artifact is, and what rests on it

A pre-implementation audit found: *"The Change and Bug template bodies are described in prose and given nowhere, though both ship in v1, the indexer must parse every template generation forever, and `--reopen` must stub each new mandatory section."* Three separate obligations are hiding in that sentence, and each one fails differently if it is left to prose.

**The bodies are shipped code, not documentation.** `spine new --change` and `spine new --bug` write bytes into someone's repository. Those bytes are the first thing a human sees, the thing an interview agent fills, and — once signed — bytes sealed verbatim into a landing commit forever (PB §5.5). Two binaries that scaffold different bytes are two binaries whose users write differently-shaped intents, and since the section table *is* the parse contract (ID §4.8), a scaffold that names a section the parser does not know produces a document nobody can sign.

**The section tables are a gate input.** `intents/<ID>.md` is the sole source of the `declares` edges G2 and G7 evaluate (ID §1), and G7 evaluates *another branch's* document with *my* binary. A Change intent whose Invariants section my binary does not know is a Change intent whose touchpoints my binary never reaches — because ID §4.9 refuses an unknown section outright, so the whole parse fails and the branch contributes no lease (ID §7.4). One missing table row silently removes a lease from the registry.

**The versioning is what makes history readable.** PB §6.7: *"the binary keeps a **parser and a renderer** for every template and envelope version ever shipped — a parser so history always parses, and a renderer because a binary one version ahead stamps the manifest's version and must therefore write that version's sections."* There are three independent version counters and three independent `resign` floors (PB §6.7's manifest), a `Template: <variant>@<n>` header that names one of the three by name — decision 4 of PB v0.19, closing ID §12 D2 — and a `--reopen` that must edit a signed document deterministically enough that its new blob id is a function of the old one (PB §4.3). §7 and §8 make each of those mechanical, and §9.1 makes the unit of compatibility the `(variant, version)` pair the header now spells out.

**What this document is a function of.** Everything here is a function of the variant, the template version, and four named substitutions (§6.1). Not of the clock — the intent document carries no date, which is why `spine new` needs no clock and PB §7.5's *"one clock, and it is the chain"* costs this artifact nothing. Not of the environment, the locale, the platform, or the running binary's own newest template version beyond the render/refuse decision of §7.5.

---

## 2. What this document does not define

`intent-doc.md` owns the grammar all three variants share, and this document adopts it unchanged. Restating it here would create a second copy to drift, so it is not restated. Specifically, and normatively, this document does **not** define:

| Concern | Owner |
|---|---|
| Canonical form: encoding, line endings, the `-----` and `Spine-` refusals, the size bounds | ID §2 |
| The intent id's grammar and padding | ID §3.1 |
| The `Template:` header's syntax — `<variant>@<n>`, the closed variant set, the legacy bare spelling — and what a reader does with an unknown version | ID §3.2 |
| Variant selection from the header, the legacy derivation, and the id-prefix agreement rule | ID §3.3 |
| The line model, the title line, the header line, `Supersedes:`, the preamble | ID §4.1–§4.5 |
| How a section is located, its key, what terminates it | ID §4.6–§4.7 |
| Unknown / duplicate / missing / misordered section behaviour | ID §4.9 |
| Body line classes: bullet, ac, continuation, prose | ID §4.10 |
| The body grammars `prose`, `bullet`, `ac`, `touchpoints`, `free` | ID §5.1–§5.5 |
| The touchpoint pattern dialect and `match` | ID §6 |
| G2 and G7's predicates | ID §7 |
| The failure order and exit codes | ID §8.2 |

ID §3.3 and ID §14 bound this document to *"two rows of §4.8's table and nothing else"*, and forbid it from defining *"a body grammar, a header field, a pattern dialect or a matching rule of its own"*. §4 respects that bound exactly: every section named below carries a body grammar that already exists in ID §5, and no header field is added.

**One extension is taken, and it is named here rather than buried.** ID §5.6 fixes the parse result's members for variant `intent`. The Change variant has two sections and one count that variant does not, so §4.4 adds exactly three members, all variant-conditional, all computed by grammars ID §5 already defines. No new grammar accompanies them. §12.9 records the choice.

---

## 3. The variant set, selection, and the disjointness invariant

### 3.1 Three variants, one lane

PB §3.5 gives the taxonomy in one line: *"Two lanes to route between, three templates within one of them."* The three are the gated lane's, the quick lane has no intent document at all, and the caps apply identically everywhere — one page, six ACs, fifteen minutes.

| Variant | Manifest key | Id prefix | PB §3.5's description |
|---|---|---|---|
| Feature | `intent` | `INT` | *"the standard template of §3.1"* |
| Change (brownfield) | `intent-change` | `INT` | *"'Goal' is replaced by Current behavior → Target behavior, and a mandatory Invariants section"* |
| Bug | `intent-bug` | `BUG` | *"a `BUG-` intent where the reproduction is AC-1"* |

The manifest keys are PB §6.7's, and they are the names used throughout: `templates.intent`, `templates.intent-change`, `templates.intent-bug`, and the same three keys under `resign`.

### 3.2 Selection is ID §3.3's, and this document still owes it an invariant

After decision 4 of PB v0.19, ID §3.3 **reads** the variant out of the document's header rather than inferring it:

```
variant(d) := the variant token of d's `Template:` value          -- ID §3.2
```

so the three manifest keys, the three `resign` floors, the three parsers and the three section tables below are all indexed by a token the document itself carries. G4 indexes `resign[variant(d)]`, the indexer picks a parser by name, and neither has to guess — which is what §1 said the versioning had to buy and what a bare `v2` could not.

ID §3.3 adds one cross-check this document depends on: **the id prefix and the variant token must agree**, `BUG` with `intent-bug` and `INT` with the other two, or `variant-prefix-mismatch`, exit 4. §3.3 below is the reason that rule exists, and §4.5's table is where it shows up.

**The legacy derivation survives, for legacy documents alone.** A document whose `Template:` value is the bare `v<n>` spelling (ID §3.2, `n ∈ {1, 2}`) carries no variant token, so ID §3.3 derives one:

```
variant_legacy(d) :=
  "intent-bug"     if the id's prefix is "BUG"
  "intent-change"  else if d contains a line whose section key (ID §4.7) is "invariants"
  "intent"         otherwise
```

That rule is total only because the tables below satisfy one invariant, which is stated here because this document is where it can be broken:

> **Disjointness invariant.** The section key `invariants` appears in exactly one variant's table — `intent-change` — and is **mandatory** there. No future version of `intent` or `intent-bug` may add it, at any presence, and no future version of `intent-change` may make it optional.

Breaking it in either direction breaks the derivation. If `intent` gained an optional `invariants`, a legacy Feature document that used it would derive to `intent-change` and be refused for missing `current behavior`; if `intent-change` made it optional, a legacy Change document omitting it would derive to `intent` and be refused for an unknown `current behavior`. Both are loud, which is ID §13 OPEN-1(a)'s stated cost — and it is precisely that cost the owner declined to pay for every document forever, which is why (b) won and why the derivation now covers a set of documents that is empty today and bounded at version 2 (ID §11.9).

**The invariant is kept rather than retired, and cheaply.** It binds one path, and that path is closed. Keeping it costs one row of every future section table being disallowed; retiring it would mean a version bump could make the legacy derivation ambiguous for a generation of documents nobody can re-stamp, since a landed intent's bytes are sealed. A rule that constrains only the future and protects an unmigratable past is the right kind of rule to keep.

The prefix test runs first in the derivation, so a legacy `BUG-` document is never a Change document whatever headings it carries. §4.5 tabulates every mis-templating outcome for both the qualified and the legacy path.

### 3.3 `--bug` forces the prefix, and that is now checked as well as required

`spine new --bug` allocates an id with prefix `BUG` and no other prefix, and stamps `Template: intent-bug@<n>`. This is normative, and the reason is mechanical rather than cosmetic: §4.3 makes `intent-bug`'s section table **identical** to `intent`'s, so the two facts a Bug intent is made of — the prefix and the variant — are the only things that distinguish it from a Feature intent, and PB §4.3's outright refusal of a green reproduction keys off the first.

**Before decision 4 this had no detector.** A `--bug` document carrying an `INT-` id derived to variant `intent`, parsed cleanly, and silently lost the one thing the Bug variant exists to buy. The two spellings of one fact could disagree and nothing anywhere would say so. Now they are checked against each other: ID §3.3's agreement rule makes `INT-051` with `Template: intent-bug@2` a `variant-prefix-mismatch`, exit 4, at ID §8.2 step 4, and `BUG-051` with `Template: intent@2` the same. The flag, the prefix and the header are three spellings of one fact, and any two of them disagreeing is a refusal rather than a silent downgrade. §13 D11 asks PB §11's CLI grammar to say that `--bug` allocates the prefix; the check no longer depends on PB saying it.

`--change` does not change the prefix. A Change intent is an `INT-` intent stamped `intent-change@<n>`; nothing downstream distinguishes the prefixes except the Bug clause of PB §4.3 (ID §3.1: *"Nothing else in this document distinguishes the prefixes"*).

**One id space, one counter.** PB §5.4 allocates `max+1` over live refs and sealed ids; ID §3.1 requires that *"the padding rule makes id and integer a bijection"*. A bijection between ids and integers exists only if the numeral is allocated from **one counter shared by both prefixes**, so `INT-042` and `BUG-042` never both exist. PB never says so, which is §13 D12.

## 4. The three section tables

### 4.1 Variant `intent`, template version 2 — reproduced, not defined

Defined by ID §4.8, which governs. Reproduced here so §4.3's identity claim and §4.5's mis-templating table can be checked against one page. Where this reproduction and ID §4.8 disagree, ID §4.8 wins and the disagreement is a defect in this document.

| Ordinal | Key | Presence | Body grammar |
|---|---|---|---|
| 1 | `goal` | mandatory | **prose** (ID §5.1) |
| 2 | `non-goals` | mandatory | **bullet** (ID §5.2) |
| 3 | `acceptance criteria` | mandatory | **ac** (ID §5.3) |
| 4 | `touchpoints` | mandatory | **touchpoints** (ID §5.4) |
| 5 | `open questions` | optional | **free** (ID §5.5) |

### 4.2 Variant `intent-change`, template version 2 — normative

Closed, ordered, and complete.

| Ordinal | Key | Presence | Body grammar | Scaffolded heading (§6.4) |
|---|---|---|---|---|
| 1 | `current behavior` | mandatory | **prose** (ID §5.1) | `## Current behavior (2–3 sentences)` |
| 2 | `target behavior` | mandatory | **prose** (ID §5.1) | `## Target behavior (2–3 sentences)` |
| 3 | `non-goals` | mandatory | **bullet** (ID §5.2) | `## Non-goals (mandatory, minimum 2)` |
| 4 | `invariants` | mandatory | **bullet** (ID §5.2) | `## Invariants (mandatory, minimum 1 — what must remain true)` |
| 5 | `acceptance criteria` | mandatory | **ac** (ID §5.3) | `## Acceptance criteria (maximum 6 — more means split the task)` |
| 6 | `touchpoints` | mandatory | **touchpoints** (ID §5.4) | `## Touchpoints (expected blast radius)` |
| 7 | `open questions` | optional | **free** (ID §5.5) | `## Open questions (optional — must be empty before implementation)` |

`goal` is **not** a key of this table. PB §3.5 says Goal is *replaced*, and a Change document carrying `## Goal` is `unknown-section` (ID §4.9), refused rather than tolerated.

**Shape bounds for `invariants`**, in ID §2.3's table's form:

| Rule | Value | Status |
|---|---|---|
| Minimum items | 1 | `invariants-too-few` |
| Maximum items | 256 | `too-many-invariants` |
| Item text | non-empty after `- ` | `empty-item` (ID §4.10) |

Minimum **1**, not 2. PB §3.2's minimum-two argument is specific to non-goals and is about an agent *over-serving* a goal, which is a failure with many plausible shapes; an invariant is a single positive claim about what the delta may not break, and one is a real claim. §12.4 records the resolution.

**Two spellings that are not accepted, and one that is.** The key is `behavior`, matching PB §3.5's own bytes. `## Current behaviour` has key `current behaviour`, is not in the table, and is `unknown-section`. One spelling per section is ID §4.9's rule (*"a tolerated `## Touchpoint` beside the mandatory `## Touchpoints` is two declarations of blast radius"*), and it applies here for the same reason. A conforming CLI's refusal message names the accepted spelling.

**Ordinal order, defended.** The delta pair occupies the positions Goal vacated, so the document still opens by saying what the work is. `non-goals` then `invariants` puts the two constraint sections together — what we will not do, then what must not break — and leaves `acceptance criteria`, `touchpoints` and `open questions` in the same relative order as the other two variants, which is what lets a reviewer read three variants without relearning the shape and what gives §8.3's stub insertion a stable target. §12.3 records why `invariants` is not adjacent to `touchpoints` instead.

### 4.3 Variant `intent-bug`, template version 2 — normative

**The section table is identical to `intent`'s** — same keys, same ordinals, same presence, same body grammars:

| Ordinal | Key | Presence | Body grammar | Scaffolded heading (§6.4) |
|---|---|---|---|---|
| 1 | `goal` | mandatory | **prose** (ID §5.1) | `## Goal (2–3 sentences — the defect, and what correct behavior looks like)` |
| 2 | `non-goals` | mandatory | **bullet** (ID §5.2) | `## Non-goals (mandatory, minimum 2)` |
| 3 | `acceptance criteria` | mandatory | **ac** (ID §5.3) | `## Acceptance criteria (AC-1 is the reproduction — maximum 6)` |
| 4 | `touchpoints` | mandatory | **touchpoints** (ID §5.4) | `## Touchpoints (expected blast radius)` |
| 5 | `open questions` | optional | **free** (ID §5.5) | `## Open questions (optional — must be empty before implementation)` |

Three things differ from `intent`, and they are the whole of the variant:

1. **The id prefix is `BUG`** (§3.3), which is what selects the variant and what PB §4.3's refusal keys off.
2. **AC-1 is the reproduction**, normatively (§5.3).
3. **Two heading parentheticals differ.** They are the only place the reproduction rule reaches the author, and ID §4.7 discards them, so they cost nothing mechanically.

PB §3.5 names exactly one difference for the Bug variant — the reproduction AC — and inventing sections for it (a `## Symptom`, a `## Expected behavior`) would add mandatory sections no playbook line asks for, would need their own `resign` bump to arrive, and would make the two most similar variants the two hardest to convert between. §12.2 records the resolution and §14 OPEN-2 puts it to the owner, because a shipped template is permanent.

### 4.4 The parse-result members the variants add

ID §5.6 fixes the members for variant `intent`. Three members are added, conditioned on the variant. Presence is *"absent means this concept does not apply"* — `gate-report.md` §7 rule 6, unchanged.

| Member | Type | Presence | Value |
|---|---|---|---|
| `goal_present` | boolean | always (ID §5.6) | `true` iff the variant's table has a `goal` section — so `true` for `intent` and `intent-bug`, **`false` for `intent-change`** |
| `current_behavior_present` | boolean | iff `variant = "intent-change"` | always `true` when the parse succeeded |
| `target_behavior_present` | boolean | iff `variant = "intent-change"` | always `true` when the parse succeeded |
| `invariant_count` | integer | iff `variant = "intent-change"` | 1 … 256 |

`goal_present` is the member ID §5.6 introduced for exactly this: *"the member exists so the shape is total across variants where Goal is replaced."* Its value in a Change document is `false`, and that is the only reading under which the sentence means anything.

**The three added members carry no text and reach no gate.** Like non-goals (PB §6.2: *"Non-goals are not nodes. They are prose constraints with no mechanically derivable edges"*), the mechanical content of Current behavior, Target behavior and Invariants is presence and a count. No node kind holds them, no edge is derived from them, `dump.md` §7.2 gives them no attrs, and G10 does not compare them. §5.2 says plainly why that is honest rather than weak.

### 4.5 What a mis-templated document does — total, and loud

Every row is a consequence of ID §3.3's selection rule and the three tables, with ID §8.2's order deciding which status fires. There is no row in which a document parses as the wrong variant and proceeds.

**Qualified header** — the variant is read, so a mis-templated document is one whose *sections* disagree with the variant it declares, or whose *prefix* does:

| Document | `variant()` | Outcome |
|---|---|---|
| `INT-`, `intent@2`, `## Goal`, no `## Invariants` | `intent` | parses |
| `INT-`, `intent-change@2`, Current + Target + Invariants | `intent-change` | parses |
| `INT-`, `intent-change@2`, `## Invariants` deleted | `intent-change` | `missing-section` (`invariants`) |
| `INT-`, `intent-change@2`, `## Goal` present | `intent-change` | `unknown-section` at `goal` (ID §8.2 step 7 checks unknown before missing) |
| `INT-`, `intent@2`, `## Invariants` present | `intent` | `unknown-section` at `invariants` |
| `INT-`, `intent-change@2`, `## Current behaviour` (British) | `intent-change` | `unknown-section` at `current behaviour` |
| `INT-`, `intent-bug@2` | — | `variant-prefix-mismatch` (ID §3.3), exit 4, before any section is read |
| `BUG-`, `intent@2` or `intent-change@2` | — | `variant-prefix-mismatch`, exit 4 |
| `BUG-`, `intent-bug@2`, feature sections | `intent-bug` | parses; AC-1 **is** the reproduction whether the author meant it or not (§5.3) |
| `BUG-`, `intent-bug@2`, `## Invariants` present | `intent-bug` | `unknown-section` at `invariants` |
| any, `Template: chore@2` | — | `template-variant-unknown` (ID §3.2), exit 4 |
| Sections present but in the wrong order for the declared variant | as declared | `section-order` |

**Legacy bare header** — the variant is derived, so the old failure shapes survive on this path and only on it (ID §3.2, `n ∈ {1, 2}`):

| Document | `variant_legacy()` | Outcome |
|---|---|---|
| `INT-`, `v2`, `## Goal`, no `## Invariants` | `intent` | parses |
| `INT-`, `v2`, Current + Target + Invariants | `intent-change` | parses |
| `INT-`, `v2`, Change sections, `## Invariants` deleted | `intent` | `unknown-section` at `current behavior` — refused, but for the wrong reason, which is the cost that decided ID §13 OPEN-1 |
| `INT-`, `v2`, `## Goal` **and** `## Invariants` | `intent-change` | `unknown-section` at `goal` |
| `INT-`, `v2`, Current only, with `## Invariants` | `intent-change` | `missing-section` (`target behavior`) |
| `BUG-`, `v2`, feature sections | `intent-bug` | parses; AC-1 is the reproduction |
| `INT-` id carrying a bug's content, `v2` | `intent` | parses as a Feature; PB §4.3's outright refusal never applies — the failure with no detector, and the reason §3.3 exists |
| any, bare `v3` or higher | — | `bad-template` (ID §3.2), exit 4 |

The second table is a table about documents that do not exist: no release has shipped, so nothing carries a bare value (ID §11.9). It is published because a parser must implement both paths and because the contrast is the argument for decision 4 in one page — the same eight documents, refused for the right reason above and for the wrong reason, or not at all, below.

**A branch whose document lands in any refusing row of either table contributes no lease and does not fail my landing** (ID §7.4). The cost of a mis-templated document is borne entirely by its own branch.

## 5. What differs between the three, and why

### 5.1 Change is delta-scoped, and that is the whole point

PB §1.1 credits the shape: *"delta-scoped change specs (OpenSpec) as the model for brownfield change-intents"*, and PB §3.5 gives the reason in one clause — *"Deltas against existing behavior fit modification work better than greenfield-style goals."*

The failure a Goal-shaped brownfield intent produces is specific. *"What the user/system can do after this ships"* (PB §3.1) describes a destination and says nothing about the origin, so an agent handed a Goal on existing code has no written statement of what it is starting from and no written boundary on how much of it may be replaced. The eager-agent failure PB §3.2 names for non-goals — *"the agent over-serving it: adding caching nobody asked for, 'improving' adjacent code"* — is at its worst here, because on brownfield code every line is adjacent to something.

**Current behavior pins the left edge of the delta.** It is the statement the tests are red against: Agent A writes failing tests from the ACs, and on a change intent the ACs describe the *difference*, so a document that never said what today does gives A nothing to differ from. It is also what Agent B reads under context isolation (PB §4.2: B gets *"only the intent doc, the tests, and … the interface slice"*), and B's job — *"write an implementation that passes every one of these tests while violating the intent doc"* — is materially easier to do honestly when the doc says what the code did before.

**Target behavior is the destination, phrased as behaviour rather than as a change.** The two together are what makes a Change intent reviewable in the fifteen minutes PB §3.3 budgets: the reviewer reads two paragraphs and a diff between them, rather than one paragraph and an assumption.

### 5.2 Invariants — what it is, what it is not, and what it costs

PB §3.5: *"a mandatory **Invariants** section lists what must remain true."*

**It is the behavioural analogue of `Must NOT change:`.** Touchpoints bound the delta in *paths*; Invariants bounds it in *behaviour*. The pair is deliberate and neither substitutes for the other: a change confined to `src/webhooks/` can still stop being idempotent, and a change that preserves idempotence can still have reached into `auth/`.

**It is not mechanically checked, and pretending otherwise would be worse than saying so.** By PB §6.2's own defence of the schema — *"Non-goals are not nodes. They are prose constraints with no mechanically derivable edges — 'violated a non-goal' cannot be auto-detected. By this playbook's own governing rule, what cannot be machine-checked stays in the doc for humans and Agent B"* — an invariant is in exactly the same position. Its mechanical content is `invariant_count`, an integer between 1 and 256. Nothing gates on it.

Two consumers make the section pay for itself anyway, and both are the ones PB assigns prose to:

- **Agent B.** B's packet is the intent doc and the tests. An invariant is a written, adversarially attackable claim: B's framing (*"write an implementation that passes every one of these tests while violating the intent doc"*) is satisfied by an implementation that breaks a listed invariant while staying green, and that is a counterexample A must answer within the two-round budget.
- **The human at the sign-off gate.** PB §3.4 calls it *"the highest-stakes three minutes in the process"*. An invariant is a sentence a human can refuse in that window.

**A section that costs a mandatory heading and buys one integer is worth defending, and the defence is that the Change variant is where drift is cheapest.** The intent exists because code already works; the risk is not that the feature is wrong but that something else stops working. Naming that something is PB §3.2's non-goal argument applied to the axis brownfield work actually fails on.

### 5.3 Bug — AC-1 is the reproduction, and G12 refuses to let it be green

PB §3.5: *"a `BUG-` intent where the reproduction *is* AC-1: the test must fail before the fix and pass after — and G12 (§4.3) refuses the approval outright if it doesn't."* PB §4.3: *"For `BUG-` intents the reproduction AC must be red or the approval is refused outright."* PB §6.3 G12: *"a `BUG-` reproduction AC that **was** green is refused outright at approval, with no `reason=` and no break-glass."*

Three statements of one rule, and none of them says which AC the reproduction is in a way a gate can compute. This section makes it computable, using only what already exists.

**Definition.** For a document whose variant is `intent-bug`, the **reproduction AC** is the AC numbered 1. Nothing marks it; its position is its identity. ID §5.3 already makes AC numbering contiguous from 1 and in document order, and already refuses a document without an AC-1, so `AC-1` exists in every parsable Bug document and needs no new grammar, no header field, no marker syntax and no node kind. The join to tests is the one that already exists: the collected ids with a `verified_by` edge to the node `<repo>/<ID>/AC-1` (`dump.md` §5.2, PB §6.2).

**The predicate.** G12 measures red on the tree PB §4.3 specifies — the approval tree *"with every path under the intent's `expected` touchpoints restored to its `base=` blob"* — where *"an id that errors, fails to import or is not collected counts as red — red means not passed."* On that tree:

```
reproduction_red := variant = "intent-bug"
                 ∧ R ≠ ∅
                 ∧ ∀ i ∈ R . i is red on the restored tree

R := { collected ids i : i has a verified_by edge to <repo>/<ID>/AC-1 }
```

`--approve` is **refused outright** when `variant = "intent-bug"` and `reproduction_red` is false.

Two clauses of that predicate are resolutions, not restatements, and §12.5 records both:

- **Every id, not any.** A reproduction that already passes without the fix is not a reproduction. If AC-1 carries three collected ids and one is green on the restored tree, that one asserts something today's code already does, and accepting the approval would let a genuine reproduction and a decoration sign together. Requiring all of them costs nothing an author cannot fix by moving the decoration to AC-2, and it is the only reading under which the sentence *"the test must fail before the fix"* is true of the ACs the document actually names.
- **`R` non-empty is already guaranteed elsewhere and is restated for totality.** PB §6's approve guards require *"every AC covered by a collected id"*, so a Bug intent whose AC-1 has no collected id is refused before G12 runs. The conjunct is present so the predicate is total when read alone: a vacuous ∀ must not read as red.

**Refused outright means no signature clears it.** This is the one place in the design where a human reason is not an exit, and the contrast with G12's other clause is deliberate:

| G12 clause | Applies to | Exit |
|---|---|---|
| `red = 0/n` — every frozen id green at approval | every gated intent | a **tripwire**: `--approve` is refused *unless a human signs it with a reason* (PB §4.3, PB §11's `reason=` mandatory on `red=0/n`) |
| reproduction AC green | `intent-bug` only | **outright refusal**. No `reason=`, no `--force`, no warn mode, and break-glass cannot reach it — PB §11 permits bypassing `G1, G2, G3, G4, G6, G7, G8, G12` *at a landing*, and this refusal happens at `--approve`, before an approval exists for break-glass to be *"available only from `tests-approved` onward"* (PB §7.6) |

The remedy is not an override. It is either a real reproduction — write the test that fails today — or the honest admission that this is not a bug, in which case the work is a Feature or a Change and the id should have been `INT-`.

**Choosing `BUG-` is choosing the stricter approval.** Because the clause keys off the variant, and the variant off the prefix, `spine new --bug` is a decision with teeth rather than a label. That is the right shape: it is the author, at the one moment they know what they are doing, buying a check on their own later self. It is also why §3.3 forbids `--bug` from producing an `INT-` id.

### 5.4 What does *not* differ, and why the caps hold everywhere

PB §3.5 closes on it: *"One page, six ACs, fifteen minutes: the caps apply identically everywhere."* Concretely, and normatively, across all three variants:

- the document bound (65 536 bytes), the line bound (4 096), the title bound (72), the AC maximum (6) and minimum (1), the non-goal minimum (2) and maximum (256), the touchpoint counts and the pattern bound — all ID §2.3's and ID §5's, unchanged;
- the header line: the same four fields in the same order, with `Ticket` the only optional one (ID §4.3). **No variant adds a header field**, which is ID §14's constraint and is what keeps `Template: <variant>@<n>` the only version stamp — one field carrying both facts, which is why decision 4 was a respelling rather than a fourth field (ID §13 OPEN-1(c));
- the touchpoint dialect and `match` (ID §6);
- the canonical-form rules (ID §2.1), including the `-----` and `Spine-` line refusals;
- `open questions`, optional and last in every table, with the same emptiness rule and the same normative constraint on the scaffold (ID §5.5: the scaffolded body must be empty).

The Change variant is one page with seven sections rather than five. That is the only budget line any variant moves, and it moves because two of its sections replace one.

---

## 6. The scaffold — the bytes `spine new` emits

### 6.1 What `spine new` writes, and the four substitutions

`spine new [--change|--bug]` allocates an id, creates `refs/heads/intent/<ID>` from trunk, and writes `intents/<ID>.md`. **The bytes it writes are the variant's scaffold at the version the manifest names** (§7.1), with exactly four spans substituted and nothing else:

| # | Span | Value | Refusal if unavailable |
|---|---|---|---|
| 1 | the id in the title line | the allocated id (ID §3.1), equal to the path's and the branch's | — |
| 2 | the `Owner:` value | the principal of the signing identity — `--identity` if given, else the principal of the key `spine init --signer-key` enrolled for this operator (PB §11) — **verbatim, with no `@` prefix added** | `bad-owner-principal` if it is empty, exceeds 128 bytes, contains `" · "`, or has leading or trailing space or tab (ID §4.3) |
| 3 | the `Template:` value | `<variant>@<n>`, the variant being the one being created and `<n>` = `templates.<variant>` from `.spine/manifest.json`, read from trunk (§7.1). The variant token is a literal of the scaffold, not a substitution: only `<n>`'s digits vary. | `unrenderable-template-version` (§7.5) |
| 4 | the `Constitution:` value's `<n>` | the version of the constitution at the manifest's `paths.constitution`, per `constitution.md` | `no-constitution-version` |

Nothing else varies. Every other byte of a scaffold is fixed by §6.4 and is identical in every repository on every platform.

**No `@` is prefixed to the principal.** PB §3.1's `Owner: @name` is a human convention — a tracker or forge handle — and `spine new` has no source for one; every identity in the design is a keyring principal (PB §7.2's `alice@example.com`). Prefixing would produce `@alice@example.com`. The field is *"a hint for humans"* with no authority (PB §3.1: *"`signed_by` in the graph is the truth"*), a human may rewrite it to `@alice` freely, and both parse. §12.7 records it and §13 D13 files the playbook's disagreement with itself.

**`Ticket` is omitted from every scaffold.** It is the one optional header field, an empty value is impossible (ID §4.3), and a scaffolded `Ticket: <link>` would be a placeholder value sealed into a landing forever if the author forgot it. The cost is that an author adding a ticket must put it at order 3, between `Template` and `Constitution`, or take `header-field-order`; a conforming CLI's refusal message names the position. §12.8 records it.

**No `Supersedes:` line is scaffolded.** It is optional, it is line 3 when present (ID §4.4), and it is authored.

### 6.2 The layout rule, stated once

A scaffold is produced by this rule and nothing else. Every byte below is `0x0A`-terminated; there is no `0x0D` anywhere and exactly one trailing `0x0A` (ID §2.1 rules 6 and 8).

1. the title line;
2. the header line;
3. one empty line;
4. then, for each section of the variant's table in ordinal order: the heading line, then the section's scaffolded body lines, then one empty line — **except after the last section, where no empty line is emitted**.

**A scaffolded body is empty, except for structural lines that carry no content.** Exactly one section has such lines: `touchpoints`, whose grammar requires both label lines and admits no other non-empty line (ID §5.4). Its scaffolded body is the two bare label lines:

```
Expected to change:
Must NOT change:
```

Every other section's scaffolded body is empty. In particular `open questions` is scaffolded empty — heading, no body — which is ID §5.5's normative constraint on this document: *"A scaffold that seeds a prose line here makes every freshly created intent unsignable."*

The two label lines are included because they are the section's *grammar* rather than its content, and their absence produces the wrong error. With them, an author who has not yet filled in patterns gets `no-expected-touchpoint`, which names what to add; without them they get `missing-touchpoint-line`, which names what is missing and requires them to know two exact strings. §12.6 records the rule and its one-section scope, so a future template that adds a structurally-lined section applies the same rule rather than inventing a second one.

### 6.3 A scaffold does not parse, and that is the design

Run ID §8.2's order over any scaffold of §6.4 and the answer is the same for all three: canonical form passes, the title passes, the header passes, the template version is known, variant selection succeeds, the preamble passes, every section heading is present, in order, known and unique — and then step 8 reaches the first mandatory body, which is empty.

| Variant | First refusal | Section | Exit |
|---|---|---|---|
| `intent` | `empty-section` | `goal` | 4 |
| `intent-change` | `empty-section` | `current behavior` | 4 |
| `intent-bug` | `empty-section` | `goal` | 4 |

This is deliberate, and it is the resolution of the document's central ambiguity (§12.1). A scaffold is a **form**, not a promise. Three consequences follow and all three are wanted:

- **A placeholder can never be signed.** The alternative — seeding bodies with content that satisfies the minima, as PB §3.1's block does — produces a document that parses, so `AC-1: Given <state>, when <action>, then <observable result>` becomes a signable acceptance criterion that promises nothing and that G1's coverage clause is satisfied by any test named `AC1`. PB §9's first open risk is *"is the interview agent producing genuinely testable ACs, or plausible-sounding ones?"*; a parsing scaffold ships the plausible-sounding one in the box.
- **A scaffolded touchpoint can never become a lease.** PB §3.1's block seeds `Expected to change: src/billing/, api/invoices.ts` and `Must NOT change: auth/, shared/schema/`. A signed lease is binding on every other landing in the repository (PB §5.4), and `Must NOT change: auth/` declared by accident is a `class=protected` G7 wire on everyone else's work. The scaffold declares nothing.
- **The state machine is unchanged and needs no new row.** PB §6's `draft` state is where a scaffold lives, and `spine new` is its guard. A `draft` document contributes no lease and yields no `intent` node — which is ID §7.4's rule for any document that does not parse, applied to the one case that is normal rather than hostile. Nothing is bricked: the branch is append-only, the human or the interview agent fills the bodies, and the first document that parses is the first one that can be signed.

**The same rule governs a reopen stub** (§8.3), which is why there is one rule and not two: an empty stub is a heading and no body, and a document containing one does not parse, which is exactly what forces the human to fill it before re-signing.

### 6.4 The three scaffolds, byte for byte

Each block below is the scaffold with §6.1's four substitutions applied to a concrete instance, so it can be hashed. The bytes between the fence markers are the document, each line terminated by one `0x0A`, with no other byte after the last.

The instances: `INT-042` / `INT-043` owned by `alice@example.com`, `BUG-051` owned by `bob@example.com`; `templates.<variant> = 2` and the constitution at `v3` in all three. Every `Template:` value is the qualified form of ID §3.2; `spine new` never emits the legacy bare spelling at any version.

#### 6.4.1 `intent@2`

```markdown
# INT-042: <short imperative title>
Owner: alice@example.com · Template: intent@2 · Constitution: v3

## Goal (2–3 sentences)

## Non-goals (mandatory, minimum 2)

## Acceptance criteria (maximum 6 — more means split the task)

## Touchpoints (expected blast radius)
Expected to change:
Must NOT change:

## Open questions (optional — must be empty before implementation)
```

| Quantity | Value |
|---|---|
| Byte length | `380` |
| Characters / lines | `372` / `14` |
| Blob id, `object_format = sha1` | `e627ec183de2a71b0e5aaed0b6227c1e8437ccde` |
| Blob id, `object_format = sha256` | `a4dae5b325b3661b7892cbb9d8b9c846fdda4c27ac97690d8503fe80bae35647` |
| `sha256sum` over the file's bytes | `eea04ff59b608f016a8f6ae7d24bdae0dcfe77615d99e9858c31af72d5603071` |

#### 6.4.2 `intent-change@2`

```markdown
# INT-043: <short imperative title>
Owner: alice@example.com · Template: intent-change@2 · Constitution: v3

## Current behavior (2–3 sentences)

## Target behavior (2–3 sentences)

## Non-goals (mandatory, minimum 2)

## Invariants (mandatory, minimum 1 — what must remain true)

## Acceptance criteria (maximum 6 — more means split the task)

## Touchpoints (expected blast radius)
Expected to change:
Must NOT change:

## Open questions (optional — must be empty before implementation)
```

| Quantity | Value |
|---|---|
| Byte length | `501` |
| Characters / lines | `489` / `18` |
| Blob id, `object_format = sha1` | `091549257b229b6a3eb7ae5d44e4e9937a7d941a` |
| Blob id, `object_format = sha256` | `fd0059feb982fce1c8c90a2aebf62d61f243c56a0af660aabf51c14edb6e4257` |
| `sha256sum` over the file's bytes | `e130a6ca264383a8083ede79d81228b9fd6b5194ca8299e07c68349c6d74bffb` |

#### 6.4.3 `intent-bug@2`

```markdown
# BUG-051: <the defect in one line>
Owner: bob@example.com · Template: intent-bug@2 · Constitution: v3

## Goal (2–3 sentences — the defect, and what correct behavior looks like)

## Non-goals (mandatory, minimum 2)

## Acceptance criteria (AC-1 is the reproduction — maximum 6)

## Touchpoints (expected blast radius)
Expected to change:
Must NOT change:

## Open questions (optional — must be empty before implementation)
```

| Quantity | Value |
|---|---|
| Byte length | `434` |
| Characters / lines | `424` / `14` |
| Blob id, `object_format = sha1` | `5eb75dcc51602ecb01d9d428d2ed0eebb2d1a86c` |
| Blob id, `object_format = sha256` | `62331b46c4b2602c8f24955e330e19c08e58a3f49ba757cf3961a75d1d0a665d` |
| `sha256sum` over the file's bytes | `868e04bfe7bd6fca19bc835a4b57a8e6423bb108d607a48ed350f52b62b5d54b` |

#### 6.4.4 The non-ASCII characters, enumerated

A scaffold is not ASCII, and an implementer who transcribes an em dash as a hyphen produces a different blob. Every non-ASCII character in every scaffold above is one of three, and their counts are the check:

| Character | Code point | UTF-8 | `intent@2` | `intent-change@2` | `intent-bug@2` | Where |
|---|---|---|---|---|---|---|
| `·` | U+00B7 MIDDLE DOT | `c2 b7` | 2 | 2 | 2 | the header line's two field separators |
| `–` | U+2013 EN DASH | `e2 80 93` | 1 | 2 | 1 | `(2–3 sentences)` |
| `—` | U+2014 EM DASH | `e2 80 94` | 2 | 3 | 3 | the `— …` clause in the AC, Open questions, Invariants and Bug-Goal parentheticals |

The `Template:` value's variant token, its `@` and its digits are all ASCII, so decision 4 changed each scaffold's length and every digest below it without changing one row of this table. The middle dot is ID §4.3's field separator, the three bytes `0x20 0xC2 0xB7 0x20`, and it is grammar rather than typography. The dashes are inside heading parentheticals, which ID §4.7 discards when computing a key — so a transcription error there changes the blob and changes nothing about the parse, which is the worst kind of divergence and the reason this table exists.

**The blob ids and the `sha256sum` rows.** Per PB §11's hash policy the identity of an intent document is its git object id; the `sha256sum` row is not a spine digest, appears in no trailer, and is published only so a reader can check their bytes without a git repository and then check their git against the row above it. All ids were produced with git 2.50.1 by `git hash-object --path intents/<name>` in repositories carrying ID §2.5's two-line `.gitattributes`.

### 6.5 What a scaffold is not

`spine new` runs the interview (PB §3.4, PB §9's roadmap step 1) and the interview agent fills the scaffold in place. The **filled** document is what a human reads and signs; §10 publishes one per variant. This document fixes the scaffold's bytes because that is what two implementations must agree on; what the interview agent writes into it is a prompt in the binary (PB §6.7: *"prompt tuning is a toolkit release, not a repo edit"*) and is out of scope (§15).

---

## 7. Versioning: `templates`, `resign`, and what a bump may do

### 7.1 Stamping — one number, from the manifest, never from the binary

PB §3.4: *"`spine new` stamps the version from the install manifest (§6.7), never from the binary, so one developer's newer binary cannot fork the team's template."* PB §6.7's skew table repeats it for the `newer` row: *"everything works; `spine new` stamps the *manifest's* template version."*

Mechanically:

```
stamp(variant) := variant ++ "@" ++ manifest.templates[variant]
```

read from `.spine/manifest.json` **at trunk**, per PB §7.4 rule 1 (*"Policy is read from trunk … never from the checkout under test"*). The whole value is written into the `Template:` header and the same version's scaffold is rendered (§7.5). A manifest with no `templates` entry for the variant is malformed and `spine new` refuses. **`spine new` never emits the legacy bare spelling**, at any version, on any path — not for a fresh intent, not for a reopen (§8.2), not for `--from`; the qualified form is the only thing a conforming binary writes, which is what keeps the legacy path (ID §3.2) a read-only compatibility target rather than a live second spelling.

The three counters are independent. `templates.intent = 3`, `templates.intent-change = 2`, `templates.intent-bug = 2` is a legal manifest, and it means a Feature intent created today is stamped `intent@3` and a Bug intent `intent-bug@2`. Because the header now carries the key as well as the number, two documents created the same day no longer look alike: the reader can see which counter each was stamped from.

### 7.2 The two maps, and the invariant between them

PB §6.7's manifest carries both:

```json
"templates": { "intent": 2, "intent-change": 2, "intent-bug": 2, "constitution": 1,
               "ci-github-collect": 4, "ci-github-land": 4, "ci-gitlab": 4, "ci-generic": 4,
               "agents-block": 2, "gitignore": 1, "gitattributes": 1, "keyring": 1 },
"resign":    { "intent": 2, "intent-change": 2, "intent-bug": 2 }
```

| Map | Domain | Meaning |
|---|---|---|
| `templates` | all **twelve** templates the pinned release ships — `agents-block · ci-generic · ci-github-collect · ci-github-land · ci-gitlab · constitution · gitattributes · gitignore · intent · intent-bug · intent-change · keyring` (`manifest.md` §3.6, PB §6.7), whether or not this repository holds a rendered instance of each | the version `spine new` / `spine init` renders and stamps |
| `resign` | the three intent variants **only** | the floor below which an intent may not be signed |

`resign` is intent-only. A `resign` entry for `ci-github-collect` or `keyring` has no meaning — nothing signs a CI workflow's template version — and a manifest carrying one is malformed: `manifest.md` §3.6 makes a key outside the three `resign-key-unknown` (§13 D15). **The map above was an eight-key map naming a single `ci-github` until PB §6.7 replaced it with these twelve**; nothing in this document reads a `templates` key, so no digest, scaffold or conformance case here moves with it (`manifest.md` §11 C11).

**The invariant, which nothing currently checks:**

> For every variant `v`: `1 ≤ resign[v] ≤ templates[v]`.

A manifest violating it bricks the repository for that variant by an ordinary-looking edit. With `resign.intent = 3` and `templates.intent = 2`, `spine new` stamps `intent@2`, ID §8.1's Layer 2 precondition — `template` ≥ `resign[variant]`, the variant now **read from the header** rather than derived — fails, and `--sign` refuses with `template-below-resign-floor` — **every intent the repository can create is unsignable, and nothing on the way in says so.** A conforming implementation refuses such a manifest with `resign-floor-above-current` at every command that reads it.

**This is now a landing gate, and it is specified elsewhere.** Decision 3 of PB v0.19 gives G14 and G16 their own document: `manifest.md` §3.6 states both rules of this section normatively, and §6.2's check 11 makes the inversion an **outright** G16 failure (`resign-floor-above-current`) while check 11b makes a lowered floor a **coverable protected finding** (`resign-lowered`). §13 D7 and §13 D8 are answered there, and §14 OPEN-4 is closed by that answer.

**Monotonicity.** `resign[v]` never decreases across an upgrade landing. Lowering it re-admits documents `--sign` has already refused and silently clears a live G4 wire on an in-flight intent, which is a policy reversal with no signed record of the reversal beyond the upgrade line itself. `manifest.md` §6.2 check 11b enforces it as a coverable finding rather than an outright refusal, because a rollback legitimately lowers `resign` when it restores an older manifest and carries a protected review by construction — which is the distinction the bare monotonicity rule was missing. G16 makes `params.langs` monotone for a structurally identical reason (PB §6.3 G16: *"floor-relevant manifest fields never shrink, and `params.langs` is one of them"*). §13 D8 records what PB itself still owes.

### 7.3 `resign` is a signing policy, never a parsing rule

This is the distinction most likely to be implemented wrongly, so it is stated flatly.

| Reads `resign` | Does not read `resign` |
|---|---|
| `spine new --sign` — Layer 2 precondition, `template-below-resign-floor`, exit 5 (ID §8.1), the variant read from the header | the parser: a document below the floor **parses normally**, by its own `(variant, version)` parser |
| G4 — an in-flight intent stamped below the floor trips a `landing-review` wire (PB §6.3) | `spine check --approve`, `--land` step 2, and every gate but G4 |
| `spine new --reopen`, when it is taken for the resign reason (§8) | `spine index` — in flight or from a sealed envelope |
| | G9, which audits a landing and never consults a floor |

So a landed intent stamped `Template: v1` — the legacy bare form, which §9.2 makes version 1's only spelling — stays readable, indexable and re-verifiable forever after `resign.intent` has moved to 5. That is the whole content of PB §6.7's promise that *"history always parses"*: a floor governs what may be signed **now**, and history was signed then. Decision 4 sharpens the promise rather than weakening it, because *which* floor applies is now a fact the document states instead of one a probe guesses.

**G4's wire is pathless, and that has a consequence nobody has written down.** PB §6.3 G4 raises the bare wire `G4` — a gate id with no path (PB §11: *"gates without a path use the bare id"*). PB §5.4's review-retention rule requires that *"every signed wire names a path"* for a review to survive a base move, because a pathless wire cannot be bounded by a path test. So **a `landing-review` discharging a G4 wire is void on any base movement and must be re-signed.** Under the shipped defaults this is invisible — the rule-5 `G11` advisory is pathless and present on every landing anyway (PB §11) — but a repository that reaches `C-A3: trusted` and `C-M4: on` will meet it as the only pathless wire left, and the review it voids is the one saying *"I accept that this intent was written under an older template."* §13 D9 asks PB §6.3's G4 row to say so.

### 7.4 What a template version bump may and may not do

A version, once shipped, is **immutable**: its section table, its body grammars, its scaffold bytes and its parse-result shape never change. A correction to a shipped version is a new version. This is what makes *"a parser for every template version ever shipped"* a promise a binary can keep rather than a moving target.

PB §6.7: *"Template bumps are additive by policy. A bump that adds a mandatory section is flagged `resign` in the release notes and the manifest's `resign` floor."* Made mechanical:

| A bump from `v(n)` to `v(n+1)` of one variant | Permitted | Notes |
|---|---|---|
| add an **optional** section at any ordinal | yes | not a `resign` bump |
| add a **mandatory** section at any ordinal | yes | **is** a `resign` bump: the release notes flag it and the upgrade raises `resign[v]` to `n+1` |
| change a section's heading parenthetical | yes | ID §4.7 discards it; the scaffold's bytes change, nothing else does |
| relax a bound (raise a maximum, lower a minimum) | yes | not a `resign` bump |
| remove a section | **no** | not additive; the exit is `--uninstall` and a new manifest lineage (PB §6.7) |
| rename a section key | **no** | a rename is a removal and an addition |
| reorder existing sections relative to one another | **no** | a new section may be inserted at any ordinal, but the relative order of the sections already present never changes — §8.3's stub insertion depends on it |
| change an existing section's body grammar | **no** | it would make a valid `v(n)` document invalid under a parser a reader might pick |
| add or remove a header field | **no** | ID §14 reserves the header to `intent-doc.md`; a header change is a change to all three variants at once and is a grammar change (below) |
| respell an existing header field's value | **no** | it changes the bytes of every document at that version, which the immutability rule above forbids — see the note below on why decision 4 is not a counterexample |
| break §3.2's disjointness invariant | **no** | it breaks variant selection for every document in the repository |

**Decision 4 is not a counterexample to immutability, and the reason is dated.** Respelling `Template: v2` as `Template: <variant>@2` changes the bytes of every version-2 scaffold — §6.4's three digests moved — while leaving the number at 2. That is legal exactly once and exactly now: **no release has shipped**, so no `v2` document exists anywhere for the respelling to invalidate, and version 2 is being *defined* with the qualified header rather than edited after the fact (ID §3.2, ID §11.9). The moment the first release ships, the same edit becomes a bump under the row above, and the legacy bare spelling — bounded at version 2, never emitted, never extended — is the artifact of the decision having been taken at the last moment it was free. A reader who wants the general rule should read the table, not this paragraph.

**A change to the shared grammar bumps all three variants at once.** ID §2, §4.1–§4.7, §4.10, §5.1–§5.5 and §6 are common to the three tables, so a change to any of them changes what `intent@n`, `intent-change@n` and `intent-bug@n` mean. Each variant's counter increments by one — the three numbers stay independent and need not become equal — and every variant's `resign` floor moves only if the change made a section mandatory. Nothing in PB says this, and a release that bumped one variant for a shared-grammar change would ship two parsers claiming to implement one grammar (§13 D10).

### 7.5 The binary keeps a **renderer** for every version, not only a parser

PB §6.7 promises a parser. It needs to promise a renderer, and the gap fails in an ordinary configuration.

Take a repository whose manifest says `templates.intent = 2` and a developer whose binary ships `intent@3`. PB §6.7's skew table puts them in the `newer` row: *"everything works; `spine new` stamps the manifest's template version."* If the binary holds only its newest renderer, `spine new` writes `Template: intent@2` above `intent@3`'s sections. The `intent@2` parser then meets `## Rollback` — or whatever `intent@3` added — and refuses `unknown-section`. **`spine new` would have produced a document `spine new --sign` cannot sign, on every repository one template version behind its developers' binaries.** This is §13 D1, and it is the most consequential defect in this pass.

The rule:

> A binary holds, for every `(variant, version)` pair it or any earlier release of the same lineage has stamped, **both** a parser and a renderer. `spine new` renders and stamps the same version. A binary asked to render a version it does not hold refuses with `unrenderable-template-version` and does not fall back to a nearby version.

The refusal is reachable only from a stale-clone or cross-repository read, the same narrow window ID §3.2 identifies for the parser, and G15's pin closes it inside a healthy repository: the trusted stage runs the pinned release, and the manifest that names the version and the release that must render it move together in one upgrade landing.

---

## 8. `spine new --reopen` and the `resign` floor

### 8.1 Two reopens, one command

PB §4.3: *"the only way to change a frozen byte is `spine new --reopen INT-042 --reason '…'`: the commit that changes the intent blob carries a signed `Spine-Reopen` line naming the freeze digest it voids, and returns the intent to `awaiting-sign-off`."* And: *"A reopen must change the blob — a no-op reopen is refused; when the reopen exists to satisfy a `resign` floor (§6.7), it rewrites the header to the floor version and inserts each new mandatory section as an empty stub, so it always changes the blob."*

Two paths, distinguished by one test:

```
resign_path := parse(d).template < manifest.resign[variant(d)]
```

| Path | Who edits the document | What guarantees the blob changes |
|---|---|---|
| ordinary — the tests are wrong (PB §4.3), the ground moved (PB §5.4) | the human, before running `--reopen` | nothing automatic: `--reopen` compares the blob at head against the blob the binding sign-off names and refuses `no-op-reopen` if they are equal |
| **resign** — `resign_path` holds | **`spine new --reopen` itself**, per §8.2 | the header rewrite, always (§8.4) |

The two are not exclusive: a human may also have edited the document. `--reopen` applies §8.2's rewrite whenever `resign_path` holds, over whatever bytes are at head, and the no-op test is evaluated after the rewrite.

`--reopen` never *lowers* a version. If `parse(d).template ≥ manifest.resign[variant(d)]`, the header is untouched, no stub is inserted, and the ordinary path applies. A reopen is not an upgrade command.

### 8.2 What the resign path rewrites, in order

Let `v_old := parse(d).template` and `v_new := manifest.templates[variant(d)]`.

1. **The header's `Template:` value becomes `<variant>@<v_new>`.** The variant token is written unchanged — **a reopen is never a variant conversion**, and `variant(d)` is read from the header before the rewrite and written back into it after — so on a qualified document only the value's digits change. On a **legacy** document (ID §3.2) the whole value is replaced: `v2` becomes `<variant_legacy(d)>@<v_new>`, which is the one place a legacy spelling is ever rewritten, and it is a one-way conversion. The field's position, its name, the surrounding `" · "` separators and every other field are untouched in both cases.
2. **Every section mandatory in `variant@v_new` and absent from the document is inserted as an empty stub**, in ascending ordinal order, at the position §8.3 fixes.
3. Nothing else. No section is removed, renamed, reordered or re-parenthesised; no body is touched; the title, `Owner`, `Ticket`, `Constitution` and `Supersedes` lines are untouched.

**Stamp `templates[variant]`, not `resign[variant]`.** The variant is the document's own, read from its header; the number is the manifest's. PB §4.3 says *"rewrites the header to the floor version"*. This document stamps the current version instead, for two reasons. It matches `spine new` (§7.1), so two documents created on the same day carry the same version rather than differing by whether one was reopened; and stamping the floor leaves the document one bump behind, so the *next* floor movement re-trips G4 on a document that was just re-signed. The two choices differ only in which number is written, never in which sections are required: PB §6.7 makes every mandatory-section addition a flagged `resign` bump, so `templates[v]` and `resign[v]` always have the same mandatory set whenever `resign[v] ≤ templates[v]` (§7.2's invariant). §13 D6 files the wording.

### 8.3 Where a stub goes, exactly, and what one is

**A stub is one heading line and no body** — the same rule §6.2 gives a scaffold, applied to one section. Its heading line is `variant@v_new`'s scaffolded heading for that key, byte for byte, parenthetical included. Structural body lines are emitted for a stubbed section under §6.2's rule if the new section's grammar requires them, which no section in v2 of any variant does but `touchpoints` would.

**Insertion position.** Let `k` be the new section's ordinal in `variant@v_new`'s table, and let `N` be the first section *present in the document* whose ordinal in that table exceeds `k`.

- If `N` exists, insert immediately before `N`'s heading line, as the two lines `<heading>` and one empty line.
- If `N` does not exist — the new section sorts after every section present — append at end of document, as one empty line followed by `<heading>`.

Both forms preserve canonical form: the first leaves the file's final `0x0A` where it was, the second makes the new heading the last line with no trailing blank line (ID §2.1 rule 8). Several stubs are inserted in ascending ordinal order, each against the document as the previous insertion left it.

This is the position ID §4.9 says fixed section order exists to provide: *"PB §4.3's reopen rule … needs a defined insertion position for the stub. Fixed order gives it one: at the new section's ordinal."* PB §4.3 states the requirement and gives neither position nor bytes (§13 D5).

### 8.4 Why the blob always changes — and the case PB's sentence misses

PB §4.3 attributes the guarantee to the pair (*"rewrites the header … and inserts each new mandatory section as an empty stub, so it always changes the blob"*). The stubs are not what guarantees it. **The header rewrite alone does**, and it has to, because a `resign` bump need not add a section: a bump may be flagged `resign` because it changed what an existing section must contain, and then step 2 inserts nothing.

```
resign_path ⟹ v_old < resign[variant] ≤ templates[variant] = v_new  ⟹  v_old ≠ v_new
```

and `Template: <variant>@<v_old>` and `Template: <variant>@<v_new>` are different byte strings, because the variant token is written identically in both and ID §3.2 forbids leading zeros, so the decimal spelling of an integer is unique. So the document's bytes differ, so its blob id differs, so the reopen is never a no-op and never refused as one. On a legacy document the guarantee is stronger still and does not even need the inequality: `v<v_old>` and `<variant>@<v_new>` differ in their first byte. §13 D5 asks PB §4.3 to say which limb carries the guarantee.

### 8.5 What the reopened document does next

Nothing else changes. PB §4.3's sequence runs unaltered: *"the human re-signs the doc (the one gate, reused — not a second one), A regenerates or edits tests, B attacks them with a fresh two-round budget and its own prior counterexamples (§4.2), and a new approval freezes the new closure."*

Two properties of the rewrite are worth stating because they are what make it safe:

- **The reopened document does not parse** while a stub is unfilled — `empty-section`, or the stubbed section's own minimum status (§6.3's rule, one section at a time). So `--sign` cannot be run until a human has written the new section's content. A reopen that silently produced a signable document would let a `resign` floor be satisfied by a command rather than by a person, which is the opposite of what a floor named *resign* is for.
- **The stub is inserted, never filled.** `spine new --reopen` writes no prose. There is no default content for a section that exists because the template decided the author must say something new.

### 8.6 Worked reopen

`INT-043` is §10.2's filled Change document, stamped `Template: intent-change@2`. The team upgrades: `intent-change@3` adds one mandatory section, key `rollback`, at ordinal 7 — after `touchpoints` (6) and before `open questions`, which becomes ordinal 8 — with body grammar **prose** and scaffolded heading `## Rollback (mandatory — how this change is undone)`. The upgrade landing sets `templates["intent-change"] = 3` and `resign["intent-change"] = 3`.

G4 fires on the in-flight `INT-043`: `parse(d).template = 2 < resign["intent-change"] = 3`. The human runs `spine new --reopen INT-043 --reason "intent-change@3 requires a rollback statement"` — and the reason string and the header value are now the same token, which they were not when the header read `v2`.

`--reopen` performs §8.2:

1. `Template: intent-change@2` → `Template: intent-change@3` on the header line — the variant token unchanged, only the digit;
2. `rollback` is mandatory in `intent-change@3` and absent; its ordinal is 7; the first present section with a higher ordinal is `open questions` (8); so the two lines `## Rollback (mandatory — how this change is undone)` and one empty line are inserted immediately before `## Open questions (optional — must be empty before implementation)`.

The tail of the resulting document:

```markdown
## Touchpoints (expected blast radius)
Expected to change: src/webhooks/, api/deliveries.ts
Must NOT change: auth/, src/webhooks/signing.ts

## Rollback (mandatory — how this change is undone)

## Open questions (optional — must be empty before implementation)
```

| Quantity | Before (`f_change`, §10.2) | After the reopen |
|---|---|---|
| Byte length | `1502` | `1557` |
| Characters / lines | `1490` / `32` | `1543` / `34` |
| Blob id, `object_format = sha1` | `89f6a976879cd598f2341d6d873b2c4eac808096` | `e92d825a37bfb5310ee13c27ff98d314ec514d10` |
| Blob id, `object_format = sha256` | `dc2cb930a5efb00f1884f5089314adf600e7c95363f7b730d18f7e6044009bf0` | `19980f046ed2948848e9a58dd9469feaa229af6cdb65433d221c5c134c7a21fe` |
| `sha256sum` over the file's bytes | `2c50528306b06c256bd5b5a7011f577c552e118e1d1bb9a311aed173422dab2a` | `b06c5d4d771b5c6113f5ff27f718355fa37124c7b06cb46a043a95f919ea5c8f` |
| Parses? | yes | **no** — `empty-section` at `rollback`, exit 4 |

The blob changed, so the reopen is not a no-op; the document no longer parses, so it cannot be signed until the human writes the rollback statement; and the `Spine-Reopen` line the commit carries names the freeze digest it voids, which is `envelope-vectors.md`'s and PB §11's business and not this document's.

**+55 bytes**, of which none is the `2`→`3` in the header (a one-digit substitution, not a growth) and all 55 are the inserted heading, its `0x0A`, and the empty line. Two implementations that insert the stub in different positions, or that add a second blank line, produce different blob ids for the same reopen — which is why the position is normative and why this vector is published.

---

## 9. Compatibility: parsing an old generation

### 9.1 The unit of compatibility is `(variant, version)`

PB §6.7 says *"a parser and a renderer for every template and envelope version ever shipped"*, and the *version* in it is one notch too coarse — the renderer half is not at issue here. There are three counters, so `intent@2` and `intent-change@2` are two different parsers that happen to share a number, and `intent@3` may exist while `intent-change` is still at 2. After decision 4 the header spells the pair out, which is exactly what makes the rule below implementable rather than aspirational: the lookup key is a substring of the document.

> A binary holds a parser and a renderer (§7.5) for every `(variant, version)` pair any release of the lineage has stamped. Parser selection is: **read** the pair from `Template: <variant>@<n>` (ID §3.2); for a legacy bare value, read the version and derive the variant (ID §3.3, §3.2 here). A pair the binary does not hold is `template-version-unknown`, exit 3 — never a partial parse, never a guess, never a fall back to the newest version held, and never another variant's parser for the same number (ID §3.2).

Variant selection runs *before* parser selection and is itself version-independent: on the qualified path it reads one token of the header; on the legacy path it inspects only the id prefix and the presence of an `## Invariants` heading, and all three are stable across every version §7.4 permits.

### 9.2 Version 1

ID §3.2 defines template version 1 for the grammar it specifies: *"version 2, plus one additional permitted header field `Status`, whose value is any non-empty free-text run and is parsed and discarded. No other difference."* ID §11.9 notes this creates the compatibility target rather than reconstructing one, because *"no v1 document exists in any repository — no release has shipped."*

Extended uniformly, with no special case:

> For every variant `v`, `v@1` is `v@2` plus a permitted `Status` header field at order 5, parsed and discarded. `spine new` never stamps version 1 for any variant, and `spine init` never writes a manifest with `templates[v] = 1`.

**Version 1's only spelling is the legacy bare one.** ID §3.2 bounds the bare `Template: v<n>` form at `n ∈ {1, 2}` and makes `Status` reachable only through it, so `v@1` is read as *"variant `v`, version 1"* and is written `Template: v1` — never `Template: intent@1`. Version 2 has both spellings, the qualified one being the only one anything emits; every version from 3 on has exactly one. That is the shape decision 4 leaves behind, and it is bounded rather than open-ended by construction.

The uniform rule is chosen over a variant-specific one because the alternative — v1 existing for `intent` only, on the reasoning that PB §3.1 is the Feature template — creates a special case in the parser-selection table to describe zero documents. Both are free; one is one line. ID §12 D9 recommends deleting the v1 promise altogether, which this document supports and which would delete this section with it.

### 9.3 What a landed generation costs

An intent stamped `Template: v1` and landed years ago is read from the fenced bytes of its landing commit (PB §6.2: *"historical: the fenced intent bytes in the landing commit's envelope (§5.5), parsed by the `Template:` version's parser"*), by `<variant>@1`'s parser — the variant derived, the version read — against that pair's section table. Nothing about the current manifest is consulted — not `templates`, not `resign` (§7.3), not the constitution version. Two consequences:

- **`--verify` and G10 stay reproducible.** ID §3.4 makes the parse a function of the document's bytes and the id from its path alone; §7.5's renderer rule adds nothing to that function, and the version-immutability rule of §7.4 is what keeps it true across releases. A landing indexed by today's binary and by a binary three releases newer yields the same nodes and edges, which is what G10 compares byte for byte.
- **A landed document that does not parse has one behaviour, already specified.** ID §8.3: the indexer refuses the document and G9 records that landing `unattested`, *"reported and counted forever"*. There is no route for one — `--sign` refuses a non-parsing document, `--land` step 2 re-parses, break-glass cannot reach G9 — so the case arises only from a hand-built envelope or an imported history.

---

## 10. Worked documents, one per variant

Each is a **filled** document — what an interview leaves behind, and what a human signs — in canonical form. As in §6.4, the bytes are those between the fence markers, each line terminated by one `0x0A`, with no other byte after the last.

### 10.1 Variant `intent` — `intent-doc.md` §9.1, verified

The Feature vector is not duplicated. `intent-doc.md` §9.1 publishes a complete filled `intent@2` document and §9.2 publishes its identity; both were reproduced byte for byte while writing this document, as a cross-check that the two specifications describe one artifact.

| Quantity | `intent-doc.md` §9.2 publishes | Reproduced here |
|---|---|---|
| Byte length | `1258` | `1258` ✓ |
| Characters / lines | `1249` / `26` | `1249` / `26` ✓ |
| Blob id, `object_format = sha1` | `1b9e758012b85f788e3b3f16f6e81383bfdc54be` | ✓ |
| Blob id, `object_format = sha256` | `1e594dc7885e7902d7e3125fc80394c53ef57aa716cf62119df0cea7be3cf39a` | ✓ |
| `sha256sum` over the file's bytes | `b93064833e0e0fbf05ed39237dcab9dce1ed407b9a19373cc69749504a3b1d99` | ✓ |

Its parse is `intent-doc.md` §9.3's, with one member this document adds: `goal_present: true` (§4.4), and `current_behavior_present`, `target_behavior_present` and `invariant_count` absent, the variant not being `intent-change`.

### 10.2 Variant `intent-change`

```markdown
# INT-043: Retry failed webhook deliveries
Owner: alice@example.com · Template: intent-change@2 · Constitution: v3

## Current behavior (2–3 sentences)
A delivery that returns a non-2xx status is logged once and dropped. The
subscriber is told nothing, and support re-sends by hand from a runbook.

## Target behavior (2–3 sentences)
A failed delivery is retried on a bounded schedule and then parked, and the
subscriber can see which state a delivery is in. The runbook is deleted.

## Non-goals (mandatory, minimum 2)
- Retrying deliveries that failed before this ships.
- A replay UI for parked deliveries. Parking is enough for now.

## Invariants (mandatory, minimum 1 — what must remain true)
- A subscriber that has already returned 2xx is never sent that delivery again.
- Delivery order within one subscription is unchanged.

## Acceptance criteria (maximum 6 — more means split the task)
AC-1: Given a delivery that returns 500, when the schedule runs, then it is
  retried at most five times and is then parked.
AC-2: Given a delivery that returns 200 on its second attempt, when the schedule
  runs again, then no third attempt is made.
AC-3: Given a parked delivery, when the subscription is rendered, then its state
  reads parked and carries the last response code.

## Touchpoints (expected blast radius)
Expected to change: src/webhooks/, api/deliveries.ts
Must NOT change: auth/, src/webhooks/signing.ts

## Open questions (optional — must be empty before implementation)
```

| Quantity | Value |
|---|---|
| Byte length | `1502` |
| Characters / lines | `1490` / `32` |
| Blob id, `object_format = sha1` | `89f6a976879cd598f2341d6d873b2c4eac808096` |
| Blob id, `object_format = sha256` | `dc2cb930a5efb00f1884f5089314adf600e7c95363f7b730d18f7e6044009bf0` |
| `sha256sum` over the file's bytes | `2c50528306b06c256bd5b5a7011f577c552e118e1d1bb9a311aed173422dab2a` |

Non-ASCII: `·` ×2, `–` ×2, `—` ×3 — all in the header line and the heading parentheticals, none in a body.

Its parse, in `intent-doc.md` §9.3's illustrative JSON rendering:

```json
{
  "id": "INT-043",
  "variant": "intent-change",
  "template": 2,
  "title": "Retry failed webhook deliveries",
  "owner": "alice@example.com",
  "constitution": 3,
  "goal_present": false,
  "current_behavior_present": true,
  "target_behavior_present": true,
  "non_goal_count": 2,
  "invariant_count": 2,
  "acs": [1, 2, 3],
  "expected": ["src/webhooks/", "api/deliveries.ts"],
  "forbidden": ["auth/", "src/webhooks/signing.ts"],
  "open_questions_empty": true
}
```

`ticket` and `supersedes` are absent. The graph elements, with `repo = myrepo`: nodes `myrepo/INT-043` (`intent`), `myrepo/INT-043/AC-1…3` (`ac`), `myrepo/code:src/webhooks/`, `myrepo/code:api/deliveries.ts`, `myrepo/code:auth/`, `myrepo/code:src/webhooks/signing.ts` (`code_unit`), `myrepo/constitution:v3`; edges `has_ac` ×3, `declares` ×4 (two `expected`, two `forbidden`), `built_under` ×1. **No node or edge derives from Current behavior, Target behavior or Invariants** (§4.4). Provenance for the `declares` edges, in flight, is `intents/INT-043.md:29` for the expected pair and `intents/INT-043.md:30` for the forbidden pair — the label line, not the pattern (ID §6.6).

This document exercises ID §7.1's precedence deliberately: `src/webhooks/` is expected and `src/webhooks/signing.ts` is forbidden, which is the *"this subtree, except that"* case. It is not a `polarity-conflict` — ID §5.4 refuses only a **byte-identical** pattern in both polarities — and under ID §7.1 a change to `src/webhooks/signing.ts` is reported once, as a hard forbidden hit, and not also as a containment miss.

### 10.3 Variant `intent-bug`

```markdown
# BUG-051: Zero-rated lines are taxed at the default rate
Owner: bob@example.com · Template: intent-bug@2 · Constitution: v3

## Goal (2–3 sentences — the defect, and what correct behavior looks like)
An invoice line marked zero-rated is charged the customer's default rate, so
every invoice carrying an exempt line overstates its tax total. A zero-rated
line must contribute nothing to that total.

## Non-goals (mandatory, minimum 2)
- Reissuing invoices already sent with the wrong total.
- Reworking the rate table. The lookup is correct; the branch that skips it is
  not taken.

## Acceptance criteria (AC-1 is the reproduction — maximum 6)
AC-1: Given an invoice with one zero-rated line and one standard line, when it
  is rendered, then the tax total covers the standard line only.
AC-2: Given an invoice whose lines are all zero-rated, when it is rendered, then
  the tax total reads 0.00.

## Touchpoints (expected blast radius)
Expected to change: src/billing/tax.ts
Must NOT change: auth/, shared/schema/

## Open questions (optional — must be empty before implementation)
```

| Quantity | Value |
|---|---|
| Byte length | `1096` |
| Characters / lines | `1086` / `24` |
| Blob id, `object_format = sha1` | `213288695f3037c75b94229a7ee21ae5f4c940b3` |
| Blob id, `object_format = sha256` | `5f59718dbd881dee8ac93e4472236ca0d0a1a2b1738614561139517910643879` |
| `sha256sum` over the file's bytes | `d7d25fe63465ae63ce41789fbf21cc3aa3ab3dcf01b883b5aed6ad56c5319293` |

Non-ASCII: `·` ×2, `–` ×1, `—` ×3.

Its parse:

```json
{
  "id": "BUG-051",
  "variant": "intent-bug",
  "template": 2,
  "title": "Zero-rated lines are taxed at the default rate",
  "owner": "bob@example.com",
  "constitution": 3,
  "goal_present": true,
  "non_goal_count": 2,
  "acs": [1, 2],
  "expected": ["src/billing/tax.ts"],
  "forbidden": ["auth/", "shared/schema/"],
  "open_questions_empty": true
}
```

**What G12 does with it.** `variant = "intent-bug"`, so the reproduction AC is AC-1. Suppose `--approve` collects, in a `vitest` repository, the runner-qualified ids (PB §11, `import-resolver.md` §11.1):

```
Spine-Test: vitest tests/billing/tax.test.ts > zero-rated > AC1 exempt line is untaxed
Spine-Test: vitest tests/billing/tax.test.ts > zero-rated > AC2 all-exempt invoice reads 0.00
```

both with `@verifies` pragmas naming `BUG-051/AC-1` and `BUG-051/AC-2` respectively. `R` is the first id alone. On the approval tree with `src/billing/tax.ts` restored to its `base=` blob, that id must be red — not passed, per PB §4.3's *"an id that errors, fails to import or is not collected counts as red"*. If it is, `red=2/2` is recorded and `--approve` proceeds. If it passes, `--approve` is **refused outright**: no `reason=` clears it, and PB §7.6's break-glass is not reachable, an approval not yet existing.

Had this document been created as `INT-051` **before decision 4**, everything above would be identical except that `variant` would be `intent`, `R` would never be computed, and a green AC-1 would be a `red=1/2` — no tripwire at all, since `k ≠ 0`. That is §3.3's argument in one paragraph, and it is now also §3.3's fix: an `INT-051` carrying `Template: intent-bug@2` is `variant-prefix-mismatch` and never parses, and an `INT-051` carrying `Template: intent@2` is a Feature intent that says so in its own header rather than one that fell through a probe.

---

## 11. Determinism rules, collected

1. **A scaffold is a function of six inputs**: the variant, the version, and §6.1's four substitutions — one of which, the `Template:` value, is itself the first two (§7.1). Nothing else. Two binaries given the same six produce byte-identical bytes.
2. **No clock.** The intent document carries no date, no duration and no version of anything that changes with time. `Constitution: v<n>` names a repository fact derived from git. PB §7.5's *"one clock, and it is the chain"* costs this artifact nothing.
3. **No environment.** No locale, no platform, no hostname, no user, no `$EDITOR`, no terminal width. The em dashes are the same bytes on every platform because they are fixed by §6.4.4, not by a formatter.
4. **No state the design forbids.** No side file records which template a repository last rendered; `templates[variant]` in the manifest at trunk is the only source (§7.1), and the manifest is a source, not a graph (PB §6.1).
5. **Version immutability.** A shipped `(variant, version)` pair's table, grammars, scaffold bytes and parse shape never change (§7.4). A correction is a new version.
6. **Additive bumps only**, with the closed permission table of §7.4. Relative order of existing sections is preserved, so §8.3's insertion position is stable.
7. **One insertion position** for a stub (§8.3), so a reopen's output blob is a function of its input blob and the two version numbers.
8. **The header rewrite carries the blob-change guarantee** (§8.4), so the guarantee holds for a `resign` bump that adds no section.
9. **`resign` is read at signing, never at parsing** (§7.3). A landed document parses forever under its own version.
10. **Selection before parsing, and selection is a read.** The variant comes from the header's own bytes (ID §3.2); only a legacy bare value falls back to the derivation, which reads the id prefix and the presence of an `## Invariants` heading (§3.2) — all three inputs stable across every permitted bump. A conforming binary never emits the legacy spelling, so the fallback is a read path only (§7.1).
11. **`esc` and `tok` are the identity on everything this document produces.** No scaffolded touchpoint pattern exists (§6.2), and every wire this document's rules raise is pathless — `G4` (§7.3) and `G12` — so no path token arises here. Where a document's own patterns reach a node id or a wire, ID §6.1 already establishes that `esc` and `tok` are the identity on every legal pattern.
12. **Two renderers agree iff they emit the same bytes**; two parsers agree iff they produce ID §5.6's value extended by §4.4's three members.

---

## 12. Resolved ambiguities

Each entry states what the playbook says, what this document chose, and why.

### 12.1 Whether `spine new` emits a document that parses

**Playbook:** PB §3.1 prints a template block with guidance text in every body, two example non-goals, two example ACs, two example touchpoint lines and one example open question. PB §9's roadmap says `spine new` *"runs the interview (§3.4) on a fresh `intent/<ID>` branch and emits the filled template, stamped with the manifest's template version"*. Nothing says whether the bytes written before the interview constitute a valid document.
**Chosen:** the scaffold is a form. Every mandatory body is empty except `touchpoints`' two structural label lines, and a scaffold therefore **does not parse** — `empty-section` at the first mandatory section, exit 4 (§6.3).
**Why:** the alternative makes placeholder content signable. `AC-1: Given <state>, when <action>, then <observable result>` parses, satisfies ID §5.3's minimum, and satisfies G1's coverage clause against any test named `AC1`; `Expected to change: src/billing/` parses and becomes a lease binding on every other landing in the repository the moment it is signed (PB §5.4). PB §9's first open risk is that the interview produces *"plausible-sounding"* ACs; a parsing scaffold ships one in the box. The cost is that a `draft` intent has no `intent` node and contributes no lease, which is ID §7.4's behaviour for any non-parsing document applied to the one case that is normal rather than hostile, and nothing in PB §6's table needs changing for it. §14 OPEN-1 puts it to the owner.

### 12.2 Whether the Bug variant has its own sections

**Playbook:** PB §3.5 gives the Bug variant exactly one difference — *"a `BUG-` intent where the reproduction is AC-1"* — and PB §6.7 gives it its own template counter and `resign` floor as though it were structurally distinct.
**Chosen:** the section table is identical to `intent`'s (§4.3). The variant differs in the id prefix, in two heading parentheticals, and in AC-1's normative meaning.
**Why:** inventing `## Symptom` and `## Expected behavior` would add mandatory sections no playbook line asks for, would need a `resign` bump to arrive if added later, and would make the two most similar variants the hardest to convert between — a bug that turns out to be a missing feature is a common and cheap re-labelling today. The separate counter and floor are still justified: they let the Bug template's guidance evolve on its own schedule, which is the only thing that actually differs. §14 OPEN-2 puts it to the owner, because a shipped template is permanent.

### 12.3 Whether "Current behavior → Target behavior" is one section or two

**Playbook:** PB §3.5: *"'Goal' is replaced by **Current behavior → Target behavior**"*. The arrow is prose and could denote one heading or a pair.
**Chosen:** two sections, at ordinals 1 and 2, both mandatory, both body grammar `prose` (§4.2).
**Why:** one heading would put U+2192 in a section key, and ID §4.7 warns that a key containing a non-ASCII byte is a key most authors will retype wrongly (`->` is not `→`) into an `unknown-section`. Two sections also give each half its own ordinal, which §8.3's stub insertion needs and which a single arrow-joined heading cannot provide; and they let a reviewer see the delta as a diff between two paragraphs, which is the OpenSpec shape PB §1.1 credits. §13 D4 files the ambiguity.

### 12.4 Invariants' minimum count

**Playbook:** PB §3.5 says the section *"lists what must remain true"* and gives no bound. PB §3.1 gives non-goals a minimum of 2.
**Chosen:** minimum 1, maximum 256 (§4.2).
**Why:** PB §3.2's minimum-two argument is specific to non-goals and rests on a failure with many plausible shapes — *"the agent over-serving it"* — where one example is not evidence that the author thought about the space. An invariant is a single positive claim about what the delta may not break; one is a real claim, and demanding a second would produce a padded second. The maximum matches non-goals because it exists for the same reason: ID §2.3's bounds are normative parse limits, because another branch's document is parsed during my landing.

### 12.5 Which AC is the reproduction, and how many of its tests must be red

**Playbook:** PB §3.5 says *"the reproduction *is* AC-1"*; PB §4.3 and PB §6.3 G12 both say *"the reproduction AC"* without saying how a gate identifies it or how many of its ids must be red.
**Chosen:** AC-1 by position, with no marker syntax; and **every** collected id verifying AC-1 must be red on the restored tree, with `R` non-empty (§5.3).
**Why:** position needs no new grammar, no header field and no node kind, and ID §5.3 already guarantees contiguous numbering from 1, so AC-1 exists in every parsable Bug document. *Every* rather than *any*, because a reproduction that already passes without the fix is not a reproduction, and accepting a mixed set would let a decoration sign alongside a genuine one — the remedy, moving the decoration to AC-2, costs an author one line. The non-emptiness conjunct is restated from PB §6's approve guards so the predicate is total read alone: a vacuous ∀ must not read as red.

### 12.6 What a scaffolded body contains

**Playbook:** PB §3.1's block fills every body with guidance. ID §5.5 imposes the opposite for one section: *"the scaffolded body of this section must be empty — no guidance line, no placeholder bullet. Guidance goes in the heading's parenthetical."*
**Chosen:** ID §5.5's rule generalised to every section, with one carve-out: **structural lines that carry no content are scaffolded**, and `touchpoints` is the only section in any v2 variant that has any (§6.2).
**Why:** generalising is what makes §12.1's decision coherent — a scaffold with guidance in one body and none in another is two rules. The carve-out earns itself by producing a better refusal: `no-expected-touchpoint` names what to add, `missing-touchpoint-line` names what is missing and requires the author to know two exact strings. Scoping the carve-out to structure rather than to a named section is what keeps it one rule when a future template adds a structurally-lined section.

### 12.7 What `spine new` writes into `Owner:`

**Playbook:** PB §3.1 shows `Owner: @name`; PB §7.2's principals are `alice@example.com`; PB §3.1 also says *"`Owner:` is a hint for humans; `signed_by` in the graph is the truth."*
**Chosen:** the signing identity's principal, verbatim, with no `@` prefixed (§6.1).
**Why:** `spine new` has no source for a forge handle, and prefixing a principal yields `@alice@example.com`. The field has no authority, both forms parse under ID §4.3, and a human who prefers `@alice` rewrites one line. The alternative — asking for a handle at `spine new` time — adds a prompt to the command PB §9 wants to take two minutes, for a field nothing reads. §13 D13 files the playbook's disagreement with itself.

### 12.8 Whether `Ticket:` is scaffolded

**Playbook:** PB §3.1's template line includes `Ticket: <link>`; ID §4.3 makes `Ticket` the one optional header field and notes *"A field with nothing to say is omitted."*
**Chosen:** omitted from every scaffold (§6.1).
**Why:** a scaffolded `Ticket: <link>` is a placeholder value that survives into a signed, sealed landing if the author forgets it, and unlike an unfilled body it does not stop the document parsing, so nothing catches it. The cost is one refusal — `header-field-order`, if the author appends the field after `Constitution` — which is loud, one-time, and fixable by a message that names the position.

### 12.9 Extending ID §5.6's parse result

**Playbook:** silent; PB §6.2's `intent` attrs hold no per-variant field.
**Chosen:** exactly three added members, all variant-conditional, all computed by grammars ID §5 already defines: `current_behavior_present`, `target_behavior_present`, `invariant_count`; plus the reading that ID §5.6's always-present `goal_present` is `false` for `intent-change` (§4.4).
**Why:** ID §5.6 fixes the members *for variant `intent`*, and a Change document has two sections and a count that variant does not; the shape has to be able to say so, and `goal_present` exists in ID §5.6 for precisely this reason (*"the member exists so the shape is total across variants where Goal is replaced"*). ID §14's prohibition is on new *grammars*, header fields, dialects and matching rules, and none of the three is any of those. None reaches a node, an edge, a gate or the dump, so no digest changes *for this extension*. (Decision 4 changed digests, but through the header's bytes, not through these three members — §12.11.)

### 12.10 Which template `--reopen` stamps

**Playbook:** PB §4.3 says the resign reopen *"rewrites the header to the floor version"*; PB §3.4 and PB §6.7 say `spine new` stamps `templates[variant]`.
**Chosen:** `templates[variant]`, the same number `spine new` stamps (§8.2).
**Why:** stamping the floor leaves a just-re-signed document one bump behind, so the next floor movement re-trips G4 on it, and two documents created the same day carry different versions depending on whether one was reopened. The two choices never differ in which sections are required, because PB §6.7 makes every mandatory-section addition a flagged `resign` bump, so `templates[v]` and `resign[v]` carry the same mandatory set under §7.2's invariant. §13 D6 files the wording.

### 12.11 Whether respelling `Template:` at version 2 is a version bump

**Playbook:** decision 4 of PB v0.19 replaces `Template: v2` with `Template: intent@2`, `intent-change@2`, `intent-bug@2`, and leaves the three numbers at 2. §7.4 of this document makes a shipped version's scaffold bytes immutable, and a respelling changes them.
**Chosen:** not a bump. Version 2 is *defined* with the qualified header; the bare form becomes a legacy read-only spelling bounded at version 2 (ID §3.2, §9.2); §6.4's three digests are recomputed rather than carried forward.
**Why:** the immutability rule protects documents that exist, and **none do** — no release has shipped, so no `v2` document is anywhere to be invalidated, which is the same argument ID §11.9 uses to *create* version 1 rather than reconstruct it. Bumping instead would have cost a `resign` movement on three variants to record a change that adds no section and refuses no document, and would have shipped a version 2 nobody ever wrote. The cost of the choice is honest and is written down: the bare spelling is a compatibility target forever, §4.5 publishes a second mis-templating table for it, and §7.4 gains a row saying the same edit is a bump the moment the first release ships.

---

## 13. Defects found in PLAYBOOK.md v0.19

Reported here rather than repaired, per `docs/spec/README.md`: where this document and PB §11 disagree, §11 wins and the disagreement is a defect in one of them. None of these is in §11. **Citations are section anchors plus a verbatim quote, never line numbers** — a line number rots as the playbook grows, a section and a quote do not. Every entry is marked **OPEN** or **CLOSED** against `PLAYBOOK.md` as it now stands.

**D1 · CLOSED · The binary was promised a parser for every template version and no renderer, and the gap broke `spine new` on any repository one version behind** (PB §6.7's *Templates and the `Template: <variant>@<n>` header* paragraph and its skew table's `newer` row). **As filed**, PB §6.7 read *"the binary keeps a parser for every template and envelope version ever shipped, so history always parses"*, and the skew table's `newer` row read *"one-line 'upgrade pending: run `spine init`'; everything works; `spine new` stamps the *manifest's* template version"*. A binary holding only its newest renderer therefore writes `Template: intent@2` above `intent@3`'s sections. The `intent@2` parser meets a section `intent@3` added, refuses `unknown-section` (ID §4.9), and the document `spine new` just created cannot be signed, approved, landed or indexed. Because a developer's binary routinely runs ahead of a repository's pin — that is the entire purpose of the `newer` row — **this would have failed `spine new` on every lagging repository, silently, with the refusal appearing one command later at `--sign`.** The fix asked for four words. **Taken:** PB §6.7 now reads *"the binary keeps a **parser and a renderer** for every template and envelope version ever shipped — a parser so history always parses, and a renderer because a binary one version ahead stamps the manifest's version and must therefore write that version's sections"*, and the skew table's `newer` row ends *"**and renders that version's body**"*. Decision 4 does not touch this: naming the variant in the header tells a reader which parser to want, and says nothing about whether the binary holds the matching renderer. §7.5.

**D2 · OPEN · PB §3.1's template block is not a document the grammar accepts** (PB §3.1's fenced template block, the one implementers transcribe). It reads `Each AC must be verifiable by a test. If you cannot imagine` and `the test, rewrite the AC.` — two **prose** lines inside `## Acceptance criteria`, whose body grammar admits only `ac` and `continuation` lines (ID §5.3). They are `stray-text`, exit 4. Its `Supersedes: INT-017                        (optional)` header line is `bad-supersedes` (ID §12 D7) and fires earlier in ID §8.2's order, so the block's *first* refusal is the `Supersedes:` line and its second the first guidance line. An implementer who ships PB §3.1 as `spine new`'s output ships a command whose output cannot be parsed, and the parser names a line the template itself printed. **Fix:** move the two guidance sentences into the heading's parenthetical, where ID §4.7 discards them, and correct the `Supersedes:` line per ID §12 D7.

**D3 · OPEN · PB §3.1's block seeds `## Open questions` with a bullet, which makes every freshly created intent unsignable** (PB §3.1's fenced template block, *"`- Anything unresolved. The agent must ask, not assume.`"*). ID §5.5 makes emptiness a sign-off precondition and states the constraint on this document normatively: *"The scaffolded body of this section must be empty — no guidance line, no placeholder bullet … A scaffold that seeds a prose line here makes every freshly created intent unsignable."* A user's first act after `spine new` would be deleting a line the template told them to keep, with `open-questions-nonempty` (exit 5) as the only explanation. **Fix:** delete that bullet; the `## Open questions` heading's own parenthetical already carries the guidance.

**D4 · OPEN · PB §3.5's Change template is unresolvable between one section and two** (PB §3.5's *Change (brownfield)* bullet, which reads: `"Goal"` is replaced by **Current behavior → Target behavior**, and a mandatory **Invariants** section lists what must remain true.). One heading `## Current behavior → Target behavior` and two headings `## Current behavior` / `## Target behavior` are both readings. They differ in the section count, in whether a section key contains U+2192, in how many ordinals exist for PB §4.3's stub insertion to target, and in what an author must retype. Nothing else in the playbook disambiguates, and §6.7's manifest carries only a version number. §4.2 resolves it as two; §12.3 gives the reasoning. **Fix:** PB §3.5 names the two headings, or cites `docs/spec/templates.md` for the table.

**D5 · OPEN · PB §4.3's reopen stub has no position, no bytes, and an unstated guarantee** (PB §4.3, *"**Reopen is a transition, not an edit.**"*: *"when the reopen exists to satisfy a `resign` floor (§6.7), it rewrites the header to the floor version and inserts each new mandatory section as an empty stub, so it always changes the blob."*). Three gaps. **Position**: nothing says where a stub goes; two implementations inserting the same section at different points produce different blob ids for the same reopen, and the blob id is the identity a signature binds. **Bytes**: nothing says whether a stub is a heading alone, a heading plus a blank line, or a heading plus placeholder text — and whether the result parses, which decides whether the human must fill it before re-signing. **Guarantee**: the sentence attributes *"so it always changes the blob"* to the pair, but a `resign` bump need not add a section — a bump may be flagged `resign` because it changed what an existing section must contain — and then the stubs are empty and the guarantee rests on the header rewrite alone, which the sentence does not say. §8.3 and §8.4 close all three. **Fix:** PB §4.3 cites `templates.md` §8 and states that the header rewrite is what carries the guarantee.

**D6 · OPEN · `--reopen` is told to stamp the floor version while `spine new` stamps the current one** (PB §4.3, *"rewrites the header to the floor version"*, against PB §3.4 and PB §6.7, which have `spine new` stamp `templates[variant]` — PB §6.7: *"The manifest records which version `spine new` stamps"*). With `templates.intent = 3` and `resign.intent = 2`, a reopened intent is stamped `intent@2` and a fresh one `intent@3` on the same day, and the reopened one re-trips G4 the next time the floor moves — a document that was just re-signed for currency, immediately stale. §8.2 stamps `templates[variant]`. **Fix:** one word in PB §4.3's reopen paragraph.

**D7 · CLOSED in `manifest.md`; the PB edit is still OPEN · No invariant ties `resign[v]` to `templates[v]`, and a manifest violating it bricks the repository silently** (PB §6.7's manifest example gives `templates` and `resign` as two maps and nothing relates them; PB §3.4 refuses below the floor — *"`--sign` refuses a `Template:` version below the manifest's `resign` floor (§6.7)"*; PB §6.3's G16 row enumerates the frozen and monotone fields and `resign` is neither). A manifest with `resign.intent = 3` and `templates.intent = 2` makes `spine new` stamp `intent@2` and `--sign` refuse it — **every intent the repository can create is unsignable**, produced by an ordinary-looking manifest edit that G16 does not check, so the landing that introduces the inversion passes every gate. **CLOSED by decision 3 of PB v0.19**, which gives G14 and G16 their own document: `manifest.md` §3.6 states `1 ≤ resign[v] ≤ templates[v]` where the manifest is defined, and §6.2's check 11 makes an inversion an outright G16 failure (`resign-floor-above-current`). What remains is a PB edit, not a design gap. **Fix:** PB §6.7 states the inequality beside the two maps and PB §6.3's G16 row cites `manifest.md` §6.2 rather than enumerating checks that now live there. §7.2, §14 OPEN-4.

**D8 · CLOSED in `manifest.md`; the PB edit is still OPEN · `resign` is not monotone and is not floor-relevant** (PB §6.3's G16 — Scaffold row). Lowering `resign.intent` re-admits documents `--sign` has already refused and silently clears a live G4 wire on every in-flight intent below the old floor. That row makes `params.langs` monotone for a structurally identical reason — *"removing a language stops its landed tests being collected, which retires part of the G1 floor, so it takes the same protected review as any other floor change rather than passing as an ordinary edit"* — and says nothing about `resign`. The two are not equally severe (a lowered floor retires no landed guarantee) but they are the same shape: a manifest edit that shrinks a policy without a signature saying so. **CLOSED by decision 3**: `manifest.md` §6.2 check 11b makes a decrease a **coverable** protected finding (`resign-lowered`) rather than a monotone refusal, which is the answer this defect was reaching for and could not name — a rollback legitimately lowers `resign` when it restores an older manifest, and carries a protected review by construction. **Fix:** PB §6.3's G16 row cites `manifest.md` §6.2 checks 11 and 11b rather than leaving `resign` out of the monotone set with no explanation. §7.2.

**D9 · OPEN · G4's wire is pathless, and no row says what that costs** (PB §6.3's G4 — Currency row; PB §11's `Spine-Review` row; PB §5.4's signed-record table; PB §6's transition table, `landing-review` row). PB §6.3's G4 row raises the bare wire `G4`; PB §11's `Spine-Review` row confirms that *"gates without a path use the bare id"*. PB §5.4's signed-record table then makes a review carrying a pathless wire unable to survive a base move — *"a review carrying one never survives a base move … Re-sign, or the record is void"* — while PB §6's `landing-review` row states the rule only for *"the rule-5 `G11` wire"*. So a `landing-review` discharging a G4 currency wire is void on any base movement, and the reader has to derive that from two other sections. Under the shipped defaults it is invisible, because the pathless rule-5 `G11` advisory is on every landing anyway; a repository that reaches `C-A3: trusted` and `C-M4: on` meets it as the only pathless wire left. **Fix:** PB §6's `landing-review` row names *"any pathless wire"* rather than the rule-5 one specifically, and PB §6.3's G4 row says its wire is pathless. §7.3.

**D10 · OPEN · The three template counters are independent and the grammar is shared, and nothing says what a shared-grammar change does** (PB §6.7's manifest example, *"`"templates": { "intent": 2, "intent-change": 2, "intent-bug": 2, …`"*). PB §6.7 carries `intent`, `intent-change` and `intent-bug` as three independent numbers; `intent-doc.md` §4.1–§4.7, §4.10, §5.1–§5.5 and §6 define one line model, one header, one set of body line classes and one pattern dialect for all three. A change to any of them changes what all three versions mean, so a release that bumped one would ship two parsers claiming to implement one grammar and a document stamped `intent-bug@2` would parse under a grammar `intent@3` had already replaced. §7.4 requires all three to bump together, each by one. **Fix:** PB §6.7 states the rule where the three counters are introduced.

**D11 · OPEN, narrowed · PB §11's CLI grammar does not bind `--bug` to the `BUG-` prefix** (PB §11's CLI, *"`spine new [--change|--bug] [--from <quick-branch>]`"*). PB §3.5 makes the Bug variant *"a `BUG-` intent"* and PB §4.3 keys the outright refusal off *"`BUG-` intents"*, but nothing says the flag allocates that prefix. Since `intent-bug`'s section table is identical to `intent`'s (§4.3), a `--bug` document carrying an `INT-` id used to parse cleanly as a Feature and lose the outright refusal without a single error — the failure had no detector. **Narrowed by decision 4, not closed:** ID §3.3's agreement rule now refuses `INT-` beside `intent-bug@n` (`variant-prefix-mismatch`), so the two facts can no longer disagree silently. What is still unwritten is which prefix `--bug` *allocates*, and a `--bug` that allocated `INT-` and stamped `intent@2` would produce a consistent Feature intent, quietly. **Fix:** PB §11's CLI or PB §9's roadmap step 1 says `--bug` allocates a `BUG-` id and stamps `intent-bug`. §3.3.

**D12 · OPEN · Whether `--bug` allocates from the same id counter is never said** (PB §5.4, *"Intent ids: `spine new` takes max+1 over live `refs/heads/intent/*`, `refs/remotes/*/intent/*` and every `Spine-Intent` id sealed on trunk"*). It does not say whether the prefix participates. If two counters existed, `INT-042` and `BUG-042` could both exist, and `intent-doc.md` §3.1's bijection between ids and integers — which that `max+1`, PB §6.3's G9 row (*"exactly one `Spine-Event: land` per intent id"*) and PB §5.4's hard-lease rule (*"the lower intent id holds the lease"*) all rest on — would be false, leaving the lease comparison undefined between prefixes. §3.3 fixes one shared counter. **Fix:** one clause on PB §5.4's *Intent ids* paragraph.

**D13 · OPEN · `Owner:` has no stated source and its shown value is not a value the tool can produce** (PB §3.1's template block, *"`Owner: @name · Template: intent@2 · Ticket: <link> · Constitution: v3`"*, and PB §3.1's prose, *"`Owner:` is a hint for humans; `signed_by` in the graph is the truth"*). Every identity elsewhere in the design is a keyring principal (PB §7.2's `alice@example.com`), and `spine new` has no source for a forge handle — there is no `--owner` flag in PB §11's CLI and no handle in the manifest or keyring. An implementer must either invent a prompt, invent a mapping, or write the principal. §6.1 writes the principal. This is adjacent to `intent-doc.md` §12 D10, which files whether the field may be *absent*; this files where its *value* comes from. **Fix:** PB §3.1 shows `Owner: alice@example.com` or PB §3.2 says the field is scaffolded from the signing principal and freely rewritten.

**D14 · OPEN · `Constitution:` has no stated source either, and no defined behaviour when there is none** (PB §2.1, *"**It is versioned** (v1, v2, v3…) and every intent doc records which version it was built under"*). Nothing says where `spine new` reads the number from — a header line in `CONSTITUTION.md`, a manifest field, a git tag — or what happens on a repository whose constitution carries no version, which is every repository whose constitution a human wrote before adopting spine. The value feeds `built_under` (PB §6.2's derivation table) and G4 (PB §6.3's G4 — Currency row), so it is a gate input with no defined origin. **Fix:** PB §2.1 names the line the version is read from, and `docs/spec/constitution.md` gives its grammar; `spine new` refuses when it cannot be read.

**D15 · CLOSED in `manifest.md`; the PB edit is still OPEN · `resign` is intent-only and the manifest does not say so** (PB §6.7's manifest example). As filed, its `templates` map carried eight keys — three intent variants plus `constitution`, `ci-github`, `ci-generic`, `agents-block`, `keyring` — while its `resign` map carried three. (PB §6.7's map is now the twelve of `manifest.md` §3.6, with `ci-github` split into `ci-github-collect` and `ci-github-land`; the asymmetry this defect is about is unchanged and so is its fix.) Nothing says the second is a subset of the first by construction rather than by the example's choice, so a reader may add `"ci-github": 4` to `resign` and expect it to mean something. It cannot: nothing signs a CI workflow's template version, and there is no floor for `--sign` to refuse against. **CLOSED by decision 3**: `manifest.md` §3.6 makes `resign` intent-only and a key outside the three `resign-key-unknown`. **Fix:** one clause beside PB §6.7's `resign` map saying so, and a citation to `manifest.md` §3.6.

**D16 · OPEN · `spine new --from <branch>` does not say which template it uses** (PB §3.5's *Quick lane* bullet, *"a quick-lane change that trips a Drift or Strength wire is escalated to the gated lane (`spine new --from <branch>`"*; PB §9's roadmap step 1 repeats it). The promoted branch already carries a diff, so the work is brownfield by construction — the strongest case in the design for the Change variant — yet the flag composes with neither `--change` nor `--bug` in PB §11's CLI grammar, and nothing states a default. The choice decides which sections are mandatory, which `resign` floor applies, and whether Invariants is required, so it is not a detail. §14 OPEN-3. **Fix:** PB §11's CLI admits `--from` alongside `--change`/`--bug` and PB §3.5 names the default.

---

## 14. OPEN — the owner's calls

**OPEN-1 · Whether a scaffold parses.** §12.1 settles it as *no*: every mandatory body is empty, so a freshly created intent refuses with `empty-section` until a human or the interview agent fills it. The alternative is to seed bodies with placeholder content that satisfies ID §5's minima, so a `draft` intent parses, appears in the graph, and could in principle be signed as written. The trade is placed rather than hidden: a non-parsing scaffold cannot be signed by accident and declares no lease by accident, and costs a `draft` intent its node; a parsing scaffold gives `spine index` something to show for a draft and puts `AC-1: Given <state>, when <action>, then <observable result>` one keystroke from a signature. **Recommendation: keep it non-parsing.** Owner-level because it decides what every user sees on their first command and what PB §6's `draft` row means in the graph.

**OPEN-2 · Whether the Bug variant gets sections of its own.** §4.3 gives it `intent`'s table with two different parentheticals and a normative AC-1. The alternative is a `## Symptom` / `## Expected behavior` pair replacing Goal, mirroring what the Change variant does — which would make the Bug template genuinely delta-shaped and would give the reproduction AC a written antecedent, at the cost of two mandatory sections PB §3.5 does not ask for and a conversion cost when a bug turns out to be a missing feature. **Recommendation: keep the tables identical.** PB §3.5 names one difference for this variant and this document should not name three. Owner-level because a shipped template is permanent: `intent-bug@2` can be superseded but never edited, and every `BUG-` intent ever landed carries whichever shape ships.

**OPEN-3 · Which template `spine new --from <branch>` uses.** §13 D16 is the defect; this is the decision. (a) `intent`, matching the plain command — cheapest, and wrong in the common case: a promoted quick-lane branch has a diff, so the work is brownfield by definition and a Goal-shaped document has no place to record what the branch already did. (b) `intent-change` by default, with `--bug` still available — the promoted branch's existing diff becomes the Current behavior the author must write down, and Invariants becomes mandatory on exactly the work most likely to break something adjacent. (c) require the author to choose, refusing `--from` without a variant flag. **Recommendation: (b).** Owner-level because it makes a mandatory section appear on a path PB §3.5 designed as the *cheap* escalation, and that is a friction decision rather than a grammar one.

**OPEN-4 · Closed by the owner, 2026-08-26: both rules are G16 checks, and they are specified in `manifest.md`.** §7.2 stated two rules that no gate enforced: `resign[v] ≤ templates[v]`, and `resign[v]` never decreases. This document recommended adding both to G16, the inversion as an outright failure and the decrease as a floor-relevant shrink taking a protected review. Decision 3 of PB v0.19 gives G14 and G16 a document of their own, and `manifest.md` takes the recommendation with one refinement worth naming: §6.2's check 11 is the **outright** inversion failure (`resign-floor-above-current`), and check 11b is the decrease as a **coverable** protected finding (`resign-lowered`) rather than a refusal — because `--rollback` lowers `resign` legitimately when it restores an older manifest, and already carries a protected review by construction. §7.2 cites both. Nothing in this document is owed further; §13 D7 and D8 record what PB itself still has to say.

---

## 15. Out of scope

Deliberately not specified here, and where it belongs instead:

- **Everything in §2's table** — the shared grammar, the canonical form, the pattern dialect, the failure order, G2 and G7's predicates: `docs/spec/intent-doc.md`, which governs.
- **The interview agent.** PB §3.4's seven questions, its non-goal extraction, its AC-verifiability stress test, its variant-specific coaching — what it should ask about Current behavior, about Invariants, about a reproduction — and `spine eval`'s golden set. The scaffold is that agent's input and the filled document its output; the agent itself is a prompt embedded in the binary, and PB §6.7 makes prompt tuning a release rather than a repo edit.
- **Id allocation.** How `spine new` computes `max+1`, which refs it fetches first, how it renumbers when its push loses, and how `--pre-receive` refuses a sealed id: PB §5.4. §3.3 fixes only that the numeral comes from one counter shared by both prefixes.
- **The commit `spine new` writes**, the branch it creates, and the `Spine-Event`/`Spine-Reopen` trailers a reopen carries: PB §5.4, PB §11 and `docs/spec/envelope-vectors.md`. This document fixes the document's bytes; the freeze digest a `Spine-Reopen` voids is `envelope-vectors.md` §4's.
- **The manifest's grammar**, including the `templates` and `resign` maps' types, the frozen-field rules, and how an upgrade lands: `docs/spec/manifest.md` — §3.6 for the two maps and the invariant between them, §6.2 for the G16 checks that hold it — and PB §6.7. §7.2 cites both; it does not define the file, and OPEN-4 is closed there rather than here.
- **The constitution's version line**, which `Constitution: v<n>` records: `docs/spec/constitution.md`. §6.1 names it as a substitution source and gives its refusal.
- **G12's other clause and the whole of `--approve`.** How ids are collected, how the restored tree is built, what `red=k/n` counts, what a closure tripwire does: PB §4.3, `docs/spec/result-file.md`, `docs/spec/import-resolver.md`. §5.3 fixes only which AC is the reproduction and how many of its ids must be red.
- **The source-symbol → runner-native-id join** that §5.3's `R` assumes — how a `@verifies BUG-051/AC-1` pragma in a blob becomes a runner-qualified id: `docs/spec/import-resolver.md` §12, which now supplies it (§12.1 the pragma grammar, §12.2 the join, §12.3 the sugar); `docs/spec/README.md` recorded it as a tracked gap and has withdrawn the entry. §5.3 fixes the AC id the pragma's right-hand side names and nothing about how the pragma is found.
- **`esc` and `tok`** — `gate-report.md` §2.3 and §6.2. **The four runner tokens** `pytest`, `vitest`, `dart-test`, `swift-test` — `import-resolver.md` §11.1, one per v1 language after decision 1 dropped Kotlin and the `gradle` adapter with it (`gradle` is reserved and emitted by nothing); §10.3's example uses `vitest` and ratifies nothing.
- **The other nine scaffolded artifacts** the manifest's `templates` map names — `constitution`, `ci-generic`, `ci-github-collect`, `ci-github-land`, `ci-gitlab`, `agents-block`, `gitignore`, `gitattributes`, `keyring`. Their bytes belong to `docs/spec/constitution.md` and `docs/spec/ci.md`, and PB §11 for the three managed regions. §7.4's rules about what a bump may do are written for intent templates and are not claimed for them.
- **Rendering.** How a reviewer's packet or `spine context` displays an intent, and what a PR body shows. PB §6.1's provenance law binds renderings; nothing in spine reads one.

---

## 16. Conformance checklist

Every item is mechanically checkable. A conforming implementation satisfies all of them.

**The renderer**

1. `spine new` writes bytes byte-identical to §6.4's block for the variant, with only §6.1's four spans substituted; no fifth span varies.
2. The layout is §6.2's: title, header, one empty line, then heading + scaffolded body + one empty line per section in ordinal order, with no empty line after the last section.
3. Every scaffolded body is empty except `touchpoints`, which carries exactly the two bare label lines `Expected to change:` and `Must NOT change:`.
4. `Ticket` and `Supersedes:` are never scaffolded.
5. `Owner:` is the signing principal verbatim, with no `@` prefixed, refused as `bad-owner-principal` when it fails ID §4.3's value rules.
6. `Template:` is the qualified value `<variant>@<n>`, its variant the one being created and `<n>` = `templates[variant]` read from the manifest at trunk; the scaffold rendered is that same pair's, and the legacy bare spelling is never emitted on any path.
7. Rendering a `(variant, version)` pair the binary does not hold refuses with `unrenderable-template-version` and never falls back to another version.
8. §6.4's three scaffolds hash to `e627ec18…`, `09154925…` and `5eb75dcc…` (sha1) and to `a4dae5b3…`, `fd0059fe…` and `62331b46…` (sha256), at 380, 501 and 434 bytes.
9. Each scaffold contains exactly the non-ASCII characters and counts of §6.4.4, and no others.
10. `spine new --bug` allocates a `BUG-` id and stamps `intent-bug`; `spine new` and `spine new --change` allocate `INT-` and stamp `intent` / `intent-change`; all three draw the numeral from one shared counter.

**The parser**

11. Variant selection is ID §3.3's, evaluated before parser selection: the header's variant token on the qualified path, and only for a legacy bare value the derivation from the id prefix and the presence of a heading whose key is `invariants`. The id prefix and the selected variant then agree — `BUG` with `intent-bug`, `INT` with `intent` and `intent-change` — or the document is refused `variant-prefix-mismatch`, exit 4, before any section is read.
12. `intent-change@2`'s table is §4.2's — seven keys, that order, `invariants` mandatory with body grammar `bullet` and 1 … 256 items.
13. `intent-bug@2`'s table is byte-for-byte `intent@2`'s in keys, ordinals, presence and body grammars.
14. `goal` is absent from `intent-change`'s table and `invariants` from the other two; a document violating either is refused, never tolerated.
15. Every row of both of §4.5's mis-templating tables — the qualified one and the legacy one — produces the status that row names.
16. `invariants` with zero items is `invariants-too-few`; with 257 items, `too-many-invariants`; both exit 4.
17. The parse result is ID §5.6's, extended by §4.4's three variant-conditional members, with `goal_present` `false` for `intent-change`.
18. No variant adds a header field, a body grammar, a pattern dialect or a matching rule.
19. A `(variant, version)` pair the binary does not hold is `template-version-unknown`, exit 3, before any section is examined and after the prefix-agreement check; a variant token outside the three is `template-variant-unknown`, exit 4; a bare `v<n>` with `n ≥ 3` is `bad-template`, exit 4.
20. `v@1` is `v@2` plus a permitted `Status` header field at order 5, parsed and discarded, for every variant; version 1's only spelling is the legacy bare `Template: v1`.

**The scaffold's own parse**

21. Each of §6.4's three scaffolds refuses with `empty-section` at the section §6.3's table names, exit 4.
22. Filling every mandatory body of a scaffold, with no other edit, yields a document that parses.

**Versioning and `resign`**

23. `resign` is read by `--sign`, by G4 and by `--reopen`'s path test, and by nothing else; a document below the floor parses normally under its own version's parser.
24. A manifest with `resign[v] > templates[v]` is refused with `resign-floor-above-current` by every command that reads it, and fails G16 outright (`manifest.md` §6.2 check 11); a lowered `resign[v]` is a coverable protected finding, `resign-lowered` (check 11b).
25. `resign` is defined only for `intent`, `intent-change`, `intent-bug`; any other key is a malformed manifest.
26. A shipped `(variant, version)` pair's table, grammars, scaffold bytes and parse shape are never changed; every bump obeys §7.4's permission table; a shared-grammar change bumps all three variants.

**`--reopen`**

27. The resign path is taken iff `parse(d).template < resign[variant(d)]`, and it stamps `<variant(d)>@<templates[variant(d)]>` — the variant token carried through unchanged, never converted, and a legacy bare value rewritten to the qualified form.
28. It inserts every section mandatory in the new version and absent from the document, in ascending ordinal order, at §8.3's position, as a heading line with no body plus the structural lines the section's grammar requires.
29. It removes, renames, reorders and re-parenthesises nothing, and writes no prose into a stub.
30. It always changes the blob, and does so by the header rewrite alone when the bump added no section; a reopen whose result is byte-identical to its input is refused `no-op-reopen`.
31. §8.6's reopen of the 1502-byte `INT-043` produces exactly 1557 bytes, blob `e92d825a37bfb5310ee13c27ff98d314ec514d10` (sha1) and `19980f046ed2948848e9a58dd9469feaa229af6cdb65433d221c5c134c7a21fe` (sha256), and that result does not parse.

**The Bug clause**

32. For a document whose variant is `intent-bug`, the reproduction AC is AC-1, identified by position and by no marker.
33. `--approve` is refused outright when `variant = "intent-bug"` and any collected id verifying AC-1 is not red on the restored tree; no `reason=`, no flag and no break-glass clears it.
34. The same document under an `INT-` id gets no such refusal, and an implementation that applies the clause by content rather than by prefix is non-conforming.

**Determinism**

35. Two renderers given the same variant, version and four substitutions emit identical bytes.
36. Nothing in this document consults a clock, a locale, a platform, an environment variable or a stored file other than the manifest at trunk and the constitution it names.
37. §10.2's and §10.3's bytes hash to `89f6a976…` / `21328869…` (sha1) and `dc2cb930…` / `5f59718d…` (sha256), at 1502 and 1096 bytes, and parse to the values printed beside them.
38. §10.1's reproduction of `intent-doc.md` §9.1 hashes to `1b9e758012b85f788e3b3f16f6e81383bfdc54be` and `1e594dc7885e7902d7e3125fc80394c53ef57aa716cf62119df0cea7be3cf39a` at 1258 bytes, confirming that the two specifications describe one artifact.
