# The Spine-Kit Playbook
### Drift-gated, intent-first development for AI-assisted teams — lightweight, organised, self-enforcing

**Version:** 0.6 · **Status:** design frozen — build reference · **Owner:** _assign before adoption_

> This playbook is the reference design for **spine-kit** — a spec-kit-style toolkit (CLI command: `spine`) that automates the workflow described here. Sections marked ⚙ describe behavior spine-kit will eventually enforce mechanically; until then, teams apply them by convention.

---

## 1. Why this exists

Full spec-driven development front-loads too much writing: heavyweight specs are slow to produce, expensive to keep in sync, and — worst of all — a stale spec actively misleads coding agents, which execute an outdated plan confidently and never flag that anything is wrong. Pure vibe coding fails the opposite way: agents drift from intent, quality slips through, and context evaporates between sessions.

This workflow takes a third position. It is built on one governing rule:

> **Every artifact must map to a specific failure mode, and every rule must be enforced by a machine, not by discipline.**

Discipline is exactly what erodes when a team moves fast with capable models. So this playbook keeps the process thin, makes almost everything disposable or executable, and leaves exactly **one mandatory human gate** in the entire pipeline.

The system has three layers with three different lifespans:

| Layer | Artifact | Lifespan | Solves |
|---|---|---|---|
| Organised | Constitution + ADRs | Permanent, rarely edited | Architectural drift, context loss |
| Lightweight | Intent doc (one page) | Disposable — deleted at merge | Feature-level drift, slow specs |
| Harness | Tests generated from acceptance criteria + CI tripwires | Executable, runs forever | Regressions, agent compliance |
| *(Derived)* | Traceability graph — an index computed from the three layers above | Regenerated on demand, never edited | Drift detection, archaeology, session resume |

```
                human intent (~2 min, rough)
                          │
                          ▼
              ┌───────────────────────┐    ┌──────────────────────────────┐
              │ interview agent       │◀───│ LAYER 1 · ORGANISED          │
              │ (≤7 questions)        │    │ CONSTITUTION.md (versioned)  │
              └───────────┬───────────┘    │ /adr/ (append-only)          │
                          ▼                └───────────────┬──────────────┘
              ┌───────────────────────┐        context, loaded into
              │ LAYER 2 · LIGHTWEIGHT │◀──────  every agent session
              │ intents/INT-042.md    │
              │ Goal · Non-goals ·    │
              │ ACs · Touchpoints     │
              └───────────┬───────────┘
                          ▼
              ══ HUMAN GATE: sign-off ══   ← the only mandatory human moment
                          │
                          ▼
              ┌──────────────────────────────────────────────────┐
              │ LAYER 3 · HARNESS (executable)                   │
              │   Agent A: ACs → failing tests                   │
              │        ↑ bounce-back on weak tests               │
              │   Agent B: adversarial cross-check (isolated)    │
              │   Agent A: implement until green (tests frozen)  │
              │   CI: gate families ×4 · tripwires               │
              └───────────┬──────────────────────────────────────┘
                          │   clean → auto-merge · tripped → human review
                          ▼
              merge: intent doc → PR description · file deleted
                          │
   ╔══════════════════════╧═══════════════════════════════════════╗
   ║ THE SPINE — derived traceability graph  (`spine index`)      ║
   ║ intent ─ AC ─ test ─ code_unit ─ changeset ─ ADR ─ constn    ║
   ║ SQLite · gitignored · rebuilt on demand · queried by         ║
   ║ `spine check` (all gate checks) · `spine context` (resume)   ║
   ╚══════════════════════════════════════════════════════════════╝
```

The double-lined box at the bottom is deliberate: every layer above it feeds the spine, nothing in the spine is written by hand, and the two commands on it are the only way anyone reads it. (§5.1 shows the same system as a sequential pipeline; this is the structural view. Two diagrams is the budget — resist adding more.)

Only Layer 1 is permanent prose. Layer 2 is deliberately thrown away. Layer 3 is code. If you find yourself adding a fourth prose artifact, you are rebuilding heavyweight SDD with extra steps — stop. The traceability graph (§6) is not a fourth artifact: nobody writes it, nobody maintains it, and it is deleted and rebuilt at will. It is an index over the first three layers, which is exactly why it is allowed to exist.

### 1.1 The name, and where spine-kit sits in the market

**Why "spine".** The `-kit` suffix deliberately places the toolkit in the spec-kit lineage — the name invites the comparison, because spine-kit wins it on the axes that matter (staleness, drift, verification). "Spine" names the end-to-end connective object of the system rather than any single feature, and the anatomy maps directly onto the architecture:

| Anatomy | Spine-kit |
|---|---|
| Vertebrae | Individual intents — small, uniform units stacked over time |
| Spinal cord | The traceability graph — the signal channel running end to end |
| Reflex arc | Tripwires and gates — they fire *without conscious thought*; no brain (human review) engages unless something unusual fires |
| Posture | Anti-drift — the whole point of having a spine |

The "Spine" context-assembler concept in the Spec Growth Engine paper (arXiv 2606.27045) is acknowledged lineage: spine-kit aims to be a first shipping implementation of that family of ideas. (The term "Intent-Driven Development" is deliberately *not* used as branding — it is already claimed by several unrelated methodologies.)

**Unique selling proposition.** Spine-kit is the only toolkit where specs cannot go stale — because intent docs are deleted at merge, traceability is derived rather than authored, and drift blocks the merge instead of waiting to be noticed. Its second headline is the adversarial test cross-check (§4.2): validated by research (AdverTest, UAgent, SWE-ABS, TDAD), shipped by no competitor.

**Adopted from the field:** AGENTS.md as the interop substrate for agent context (never invent a new context-file format); delta-scoped change specs (OpenSpec) as the model for brownfield change-intents; a genuine quick lane, answering the market's loudest complaint ("a quick bug fix gets the same ceremony as a new feature"); EARS-style phrasing for acceptance criteria plus property-based testing (Kiro); and a warn-before-block calibration mode for the drift gate.

**Deliberately refused:** multi-persona agent theater; IDE lock-in; credit-metered pricing; per-feature document suites (the cautionary tale: a 444-line generated contract for a module a quarter that size); spec-as-source code generation; cloud-only architecture. Spine-kit stays CLI-first, local-first, MIT-licensed, with a local SQLite graph and no mandatory API key.

---

## 2. Layer 1 — The Constitution and ADRs (Organised)

### 2.1 The Constitution

One file per repository (`CONSTITUTION.md`, or folded into `CLAUDE.md` / `AGENTS.md` so agents load it automatically). It contains the durable, rarely-changing truths of the codebase: the stack, the architectural shape, naming conventions, testing rules, and the non-negotiables.

Rules that keep it alive:

- **Hard cap: ~150 lines.** Past that, agents skim it and humans stop reading it. Brevity is what makes it authoritative.
- **Written in week one on greenfield projects.** This is non-optional. Without it, your conventions get defined by whichever agent ran first, and every developer's agent invents a slightly different codebase. Week-one constitution-writing is the single highest-leverage meeting of a greenfield project — and `spine init` ⚙ facilitates it: the interview agent's first job is interviewing the *team* to draft the constitution.
- **Changes only via pull request.** The PR discussion is your governance. No verbal amendments.
- **It is versioned** (v1, v2, v3…) and every intent doc records which version it was built under, so mid-flight rule changes never become an argument three weeks later.
- **It has a named owner.** Unowned constitutions rot in about a month. The owner's job is not to write it alone — it is to keep it honest and small.
- **Rules carry IDs, and rules grow teeth. ⚙** Every rule is numbered (`C-1`, `C-2`…) and optionally carries an `enforced_by:` field pointing at a real check — a lint rule, a dependency constraint, a grep probe, or an LLM-judge for the genuinely fuzzy ones. `spine check --constitution` reports each rule as **enforced** or **aspirational**, and the enforced ratio is a health metric: a constitution that is mostly aspirational is a wish list, not a constitution.

```markdown
C-4: No module may import from `auth/` except through `auth/api.ts`.
  enforced_by: depcruise:no-auth-internals
C-7: Prefer composition over inheritance.
  enforced_by: (aspirational)
```

### 2.2 Architecture Decision Records (ADRs)

An append-only folder (`/adr/`), one short file per decision: what we decided, why, and what we rejected. One paragraph each is fine.

ADRs are the team-scale answer to "losing context between sessions." The operating principle:

> **Anything an agent needs to resume work must live in the repo, not in someone's chat history.**

Prose session-handoff notes do not scale past two people. ADRs plus the constitution plus the in-flight intent doc are the entire resumable state of the project.

**ADR template:**

```markdown
# ADR-007: <decision in one line>
Date: 2026-08-25 · Status: accepted

**Decision.** We will <X>.
**Why.** <1–3 sentences of context and reasoning.>
**Rejected.** <Alternatives considered and the one-line reason each lost.>
```

---

## 3. Layer 2 — The Intent Doc (Lightweight)

One page. Fifteen minutes. Disposable. This is the artifact your team touches daily, so every field maps to a failure mode — anything that doesn't is bureaucracy and was cut.

### 3.1 The template

```markdown
# INT-042: <short imperative title>
Owner: @name · Template: v1 · Status: draft | tests-approved | merged | reverted | superseded
Ticket: <link> · Constitution version: v3

## Goal (2–3 sentences)
What the user/system can do after this ships, and why it matters.
Written as outcome, not implementation.

## Non-goals (mandatory, minimum 2)
- Things an eager agent would plausibly do that we do NOT want.
- Adjacent features explicitly deferred.

## Acceptance criteria (maximum 6 — more means split the task)
AC-1: Given <state>, when <action>, then <observable result>
AC-2: ...
Each AC must be verifiable by a test. If you cannot imagine
the test, rewrite the AC.

## Touchpoints (expected blast radius)
Expected to change: src/billing/, api/invoices.ts
Must NOT change: auth/, shared/schema/

## Open questions (optional — must be empty before implementation)
- Anything unresolved. The agent must ask, not assume.
```

### 3.2 Why each field earns its place

**Goal** is outcome-phrased so review discussions stay about intent, not implementation taste.

**Non-goals are mandatory (minimum two)** because drift is rarely an agent ignoring the goal — it is the agent *over-serving* it: adding caching nobody asked for, "improving" adjacent code, handling corner cases that don't exist. Naming what not to do is far cheaper than describing everything to do. This is the highest-leverage sixty seconds in the document.

**Acceptance criteria are capped at six**, and the cap is a scoping tool disguised as formatting. When someone hits AC-7, the template itself says "split the task" — nobody has to police scope in review. Each AC has an ID because IDs flow downstream: tests are named after them (`test_AC1_...`), CI verifies every AC has a matching test, and "did the agent follow the spec" becomes a mechanical check instead of a vibe.

**Touchpoints — especially "must NOT change" — is the tripwire.** When a diff touches `auth/` and the doc said it wouldn't, that is no longer a judgment call; a CI bot flags it in seconds. Cheapest drift detection available. Touchpoints also do double duty as auto-merge policy (see §5.2).

**Constitution version** pins which rules the feature was built under.

**Open questions must be empty before implementation begins.** This converts "the agent assumed" into "the agent asked."

### 3.3 Rules that keep it lightweight

- **The 15-minute rule.** If the intent doc takes longer than fifteen minutes to produce, the task is too big. **Split the task; never grow the doc.** Spec-driven workflows fail when people spec epic-sized work; they succeed at small-feature scope. Scope is the variable — the template never expands.
- **The disposal rule.** At merge, the intent doc becomes the pull-request description and the file is deleted from the working tree. Git history preserves it forever for archaeology; the repo stays free of stale specs that could mislead a future agent. Permanence without staleness.

### 3.4 How it gets written: the agent interview

The human does not write the intent doc from scratch. The human gives a rough, high-level intention (two minutes, verbal or a couple of sentences), and an agent interviews them, then produces the filled template.

Interview protocol for the agent:

1. **Hard cap: 7 questions.** An unconstrained agent will interrogate for twenty minutes.
2. **At least 2 questions must hunt for non-goals** — "Should I also handle X?" Every "no" becomes a non-goal line for free. Agent interviews are the best non-goal extractors that exist.
3. **At least 1 question must stress-test AC verifiability** — "How would we observe that this worked?" If the human can't answer, the AC gets rewritten during the interview, not discovered broken later.
4. The agent asks about blast radius to fill Touchpoints, including what must not change.
5. Anything still ambiguous lands in Open questions rather than being silently assumed.

The human then reads the finished one-pager and signs it off. **This sign-off is the single mandatory human gate in the whole pipeline** — the highest-stakes three minutes in the process. Everything downstream is machine-enforced, which is precisely why the interview must force testable ACs and explicit non-goals here.

Two future-proofing details baked into the artifact. ⚙ The sign-off is a **signed git commit (or signed trailer), not a status flip** — with auto-merge in play, "who approved this intent" must be tamper-evident, and bolting attribution on after an incident is how compliance retrofits are born. And the `Template: v1` header exists because merged intents live in git history forever and the indexer must parse every generation of them; version-tag the template on day one or every future template change silently breaks historical parsing.

### 3.5 Two lanes, three templates ⚙

Not all work deserves the pipeline — forcing full ceremony onto small tasks is the most common way SDD tooling dies in practice, because developers skip the process exactly where AI drifts most. Spine-kit has exactly **two lanes**:

- **Quick lane** — no intent doc. Available only when the change is small (below the diff threshold), stays inside safe touchpoints, and touches no schema/auth/public API. The constitution and CI gates still apply; the tripwires *are* the lane boundary — a quick-lane change that trips one is automatically escalated to the gated lane. The router can be mechanical: predicted blast radius from the code graph decides the lane, not vibes.
- **Gated lane** — the full pipeline of §5.1, with **three template variants** for the intent doc:
  - **Feature** — the standard template of §3.1.
  - **Change (brownfield)** — "Goal" is replaced by **Current behavior → Target behavior**, and a mandatory **Invariants** section lists what must remain true. Deltas against existing behavior fit modification work better than greenfield-style goals.
  - **Bug** — a `BUG-` intent where the reproduction *is* AC-1: the test must fail before the fix and pass after. Bugs are the natural home of the test-first flow.

Two lanes to route between, three templates within one of them — that is the entire taxonomy. One page, six ACs, fifteen minutes: the caps apply identically everywhere.

---

## 4. Layer 3 — The Harness (executable, not prose)

The harness is not markdown. It is tests, type checks, linters, and CI gates — because drift a document catches depends on someone re-reading the document, while drift a test catches is caught on every single run.

### 4.1 The AC → test flow

1. **Agent A** reads the signed-off intent doc and writes **failing tests**, one or more per acceptance criterion, named after the AC IDs (`test_AC1_invoice_totals_include_tax`).
2. **Agent B cross-checks adversarially** (see §4.2). If B finds a hole, the tests bounce back to A. Loop until B fails to break them.
3. **Agent A implements until green.** The tests are the contract; the implementation is whatever satisfies it within the constitution's rules.
4. CI verifies mechanical compliance: every AC ID has at least one matching test, lint passes, types pass, coverage holds on changed lines.

Humans review tests only when a tripwire fires (§5.2) — and when they do, they review *tests*, not implementations. Reviewing tests is roughly an order of magnitude cheaper and is the moment a human confirms the agent understood intent, before any implementation exists to be attached to.

### 4.2 The adversarial cross-check

The known trap of agent-checks-agent is **correlated failure**: two similar models with similar priors can share the exact same misreading of an ambiguous AC, and green tests then prove only that the code matches the tests — not that the tests match intent. Two mitigations are mandatory:

- **Context isolation.** Agent B receives *only* the intent doc and the tests. Not the implementation, not Agent A's conversation, not the ticket thread. B must independently derive what the tests should assert. Isolation is what breaks the shared-context correlation — it matters more than using a different model vendor (though a different model family is a cheap additional hedge).
- **Adversarial framing.** B's task is not "review these tests." It is: **"Write an implementation that passes every one of these tests while violating the intent doc."** If B succeeds, the tests are too weak and go back to A with B's counterexample attached. Adversarial framing catches weak assertions that a friendly review waves through.
- **Bounded loop.** The A↔B exchange is capped at **two rounds**. If B can still break the tests after two hardening passes, the ambiguity is in the ACs, not the tests — the intent routes to a human instead of burning tokens on an unwinnable ping-pong. An anti-runaway harness must not itself contain a runaway loop.

### 4.3 Test immutability during implementation

Once tests are approved (by B's failure to break them, or by a human when a tripwire fired), **Agent A may not modify them while implementing.** If implementation reveals the tests are wrong, that is an intent problem: reopen the intent doc, re-sign, regenerate. This prevents the quiet failure where an agent "fixes" a failing test instead of the code.

---

## 5. The pipeline end to end

### 5.1 Flow

```
human intent (≈2 min, rough)
  → agent interviews human (≤7 questions) → intent doc
  → HUMAN GATE: sign off the doc          ← the one mandatory human moment
  → Agent A: failing tests from the ACs
  → Agent B: adversarial cross-check (context-isolated)
  → Agent A: implement until green (tests frozen)
  → tripwires clean?  → auto-merge
     tripwire fired?  → human reviews (tests first, then diff)
  → intent doc becomes the PR description; file deleted from repo
```

### 5.2 Tripwires (tiered auto-merge)

A green pipeline auto-merges **only when all of the following hold**:

- The diff stays inside the intent doc's declared "expected to change" touchpoints.
- Nothing in "must NOT change" was touched.
- No changes to schema, auth, or public API surface.
- No new dependencies introduced.
- Diff size under the team's threshold (pick a number; 400 changed lines is a sane start).
- Every AC ID has a matching passing test; lint, types, and coverage gates pass.

Any tripwire fires → a human reviews before merge. The checks are mechanical, so they cost nothing when work is normal — the touchpoints field of the intent doc is literally the merge policy.

⚙ In v1, touchpoint checks are path-prefix matching. Once spine-kit ships a code graph (§6.1), they upgrade to **graph containment**: "did the diff stay inside the declared dependency subgraph?" — which catches indirect blast radius (a change that ripples into `auth/` through a shared helper) that path matching misses.

### 5.3 Roles summary

| Actor | Responsibility |
|---|---|
| Human (feature owner) | Rough intent; answer interview; **sign off intent doc**; review only when tripwires fire |
| Interview agent | ≤7 questions; extract non-goals; force testable ACs; fill template |
| Agent A | Failing tests from ACs; implementation until green; tests frozen during impl |
| Agent B | Context-isolated adversarial attack on the tests |
| CI | AC↔test mapping, lint/types/coverage, tripwire evaluation, auto-merge |
| Constitution owner | Keep constitution honest, small, and versioned |

### 5.4 Multiple intents in flight ⚙

A larger team runs many intents concurrently, and a pipeline designed for one intent at a time will fail in exactly one place: two intents claiming the same code. Spine-kit treats declared touchpoints as **soft leases**:

- **At sign-off**, gate **G7 — Interference** queries the graph: which other in-flight intents declare overlapping `expected` touchpoints? A collision doesn't forbid the work — it surfaces the overlap to both owners *before* implementation starts, when coordination costs minutes instead of a rewrite. (Overlap with another intent's `forbidden` set is a hard flag.)
- **At every merge**, the graph flags each in-flight intent whose declared touchpoints intersect the merged diff: *"your ground moved — re-verify."* The flagged intent's tests re-run against the new base; failures route back through the normal bounce-back path rather than being discovered at its own merge.
- **Frozen tests meet moved ground** the honest way: if a re-verify failure reveals the intent itself is now wrong (not just the code), that is an intent problem — reopen, re-sign, regenerate, exactly as §4.3 prescribes.

Coordination-aware gating is a differentiator: no current SDD toolkit models intent-to-intent interference at all.

---

## 6. Graph engineering ⚙ — the derived layer

This workflow already contains a latent graph. The ID discipline — `INT-042 → AC-1…6 → test_AC1_* → touchpoints → constitution v3 → ADR-007` — is nodes and edges. Spine-kit's job is not to ask anyone to draw a graph; it is to **extract the graph that already exists** in the artifacts. This section defines that extraction.

"Graph engineering" in current practice means three different graphs, and conflating them is the fastest way to bloat the toolkit. Spine-kit ships **two graphs and one table** — the third "graph" was deliberately demoted in v0.5:

| Graph | Authored by | Role in spine-kit |
|---|---|---|
| Traceability graph | Nobody — derived from IDs by `spine index` | Drift gates, coverage gates, archaeology, session resume |
| Code graph | Nobody — derived from AST/dependencies (tree-sitter) | Touchpoint proposal in the interview; graph-containment tripwires; scoped context for Agents A and B; the quick-lane router |

**The workflow "graph" is a transition table, not an engine.** What graph engineering actually teaches is that transitions must be explicit, reviewable, and permitted-only — it does not require an orchestration framework. Spine-kit encodes the pipeline as a dozen declarative rows (state × event → next state, plus the guard that enforces each), checked by the same code that runs the gates:

| State | Event | Next state | Enforced by |
|---|---|---|---|
| draft | interview complete | awaiting-sign-off | `spine new` |
| awaiting-sign-off | signed commit | signed | signature check |
| signed | A's tests written | tests-drafted | `spine index` refuses `verified_by` edges to unsigned intents |
| tests-drafted | B fails to break (≤2 rounds) | tests-approved | bounded A↔B loop (§4.2) |
| tests-drafted | B still breaking after 2 rounds | human review | loop cap |
| tests-approved | implementation green + tripwires clean | merged | `spine check` + auto-merge |
| tests-approved | tripwire fired | human review | tripwires (§5.2) |
| merged | revert detected in git | reverted | derived (§6.6) |

Anything not in the table cannot happen — implementation before sign-off simply has no row. If full orchestration of Agents A and B someday genuinely needs retries, budgets, and resumable runs, an engine can be adopted *behind* this same table; not before. **User-defined custom workflow DAGs are refused**: a user-authored workflow is an authored graph, and the iron rule below applies to workflows too.

### 6.1 The iron rule: derived, never authored

The moment a user has to create or maintain a graph, you have rebuilt SDD bureaucracy in graph clothing, with a worse editor. Every graph in spine-kit is a **cache**: gitignored, deleted at will, deterministically rebuilt from the repo by one command. A derived graph can never go stale, which is the same property that justified deleting intent docs at merge — permanence without staleness, now for structure instead of prose.

Corollary — **the provenance law**: every node and edge must cite its source (a `file:line` or a git object). An edge that cannot say where it came from does not exist. This is what makes the graph auditable, regenerable, and honest.

### 6.2 Traceability graph schema

Designed backwards from the six questions it must answer mechanically: Is every AC verified? Did the diff stay in bounds? Which intents ever touched this module? What is the resumable context for an in-flight intent? Which intent does a failing test trace to? Which in-flight work was built under an outdated constitution? Nothing that fails to serve one of those questions is in the schema.

```sql
CREATE TABLE nodes (
  id   TEXT PRIMARY KEY,  -- "INT-042" | "INT-042/AC-1" |
                          -- "test:billing/test_inv.py::test_AC1_totals" |
                          -- "code:src/billing/" | "cs:abc123f" |
                          -- "ADR-007" | "constitution:v3"
  kind TEXT NOT NULL,     -- intent | ac | test | code_unit |
                          -- changeset | adr | constitution
  attrs JSON,             -- intent: {status, owner, title}
                          -- test: {last_result, ci_run}
  src  TEXT NOT NULL      -- provenance: "intents/INT-042.md:14" | "git:abc123f"
);

CREATE TABLE edges (
  from_id TEXT NOT NULL,
  to_id   TEXT NOT NULL,
  kind    TEXT NOT NULL,  -- has_ac | verified_by | declares |
                          -- implements | modifies | built_under |
                          -- supersedes | superseded_by | exercises
  attrs   JSON,           -- declares: {"polarity":"expected"|"forbidden"}
  src     TEXT NOT NULL
);
```

Storage is SQLite in a single gitignored file. No graph database in v1; the day SQLite genuinely cannot answer a needed query is the day to revisit — not before. **IDs are repo-scoped from day one** (`myrepo/INT-042`, not bare `INT-042`): it costs one line in the ID scheme now, and it makes multi-repo federation a namespace merge later instead of a rewrite of every graph, pragma, and PR description. When the multi-repo day comes, federate SQLite files — do not reach for a distributed graph database.

**Derivation sources** (this table is the indexer's spec):

| Graph element | Derived from |
|---|---|
| `intent`, `ac` nodes; `has_ac`, `built_under` | `/intents/*.md` (in-flight) + PR descriptions in git log (historical — the disposal rule puts merged intents in git, and the indexer reads them from there) |
| `test` nodes; `verified_by` | test files, via naming convention `test_AC1_*` or explicit pragma `# @verifies INT-042/AC-1` |
| `declares` (with polarity) | the Touchpoints section of the intent doc |
| `changeset`; `modifies` | git diffs |
| `exercises` (optional, v1.1) | CI coverage reports |
| `supersedes` | ADR and constitution headers |

Two schema positions, defended:

- **The pragma is canonical; the naming convention is sugar.** `test_AC1_totals` is a friendly default, but the comment pragma survives test renames, works identically across languages, and makes `verified_by` greppable without parsing any test framework.
- **Non-goals are not nodes.** They are prose constraints with no mechanically derivable edges — "violated a non-goal" cannot be auto-detected. By this playbook's own governing rule, what cannot be machine-checked stays in the doc for humans and Agent B. (Same reason there are no function-level nodes: that is the code graph's job; the two graphs join on `code_unit` paths rather than merging into one mega-graph.)

### 6.3 Gates as queries

With this schema, every gate in §5.2 becomes a one-liner. The drift gate is literally set containment:

```sql
-- G2: files modified by the PR that fall outside declared touchpoints
SELECT m.to_id FROM edges m
JOIN edges i ON i.from_id = m.from_id AND i.kind = 'implements'
WHERE m.kind = 'modifies' AND i.to_id = 'INT-042'
  AND m.to_id NOT IN (SELECT to_id FROM edges
    WHERE from_id = 'INT-042' AND kind = 'declares'
    AND json_extract(attrs,'$.polarity') = 'expected');
-- any row → tripwire fires
```

The full gate suite — four **families** are the public vocabulary; G-numbers are internal check IDs:

| Family | Check | Query, in words |
|---|---|---|
| Integrity | G1 — Coverage | Every AC of a `tests-approved`+ intent has ≥1 `verified_by` edge whose test passed in the latest CI run |
| Integrity | G5 — Orphans | A `verified_by` edge pointing at a nonexistent AC (typo'd pragma) fails loudly instead of silently verifying nothing |
| Drift | G2 — Containment | `modifies` ⊆ declared `expected` touchpoints; any `forbidden` touchpoint hit is a hard fail |
| Drift | G7 — Interference ⚙ | At sign-off: any other in-flight intent declaring overlapping touchpoints is surfaced to both owners; on every merge: in-flight intents whose touchpoints intersect the merged diff are flagged to re-verify (§5.4) |
| Freshness | G3 — Staleness | An in-flight intent node older than ~14 days is flagged — anti-staleness *inside* the working window, not just after merge |
| Freshness | G4 — Constitution currency | Any in-flight intent `built_under` a superseded constitution version is routed back for re-sign-off |
| Strength | G6 — Mutation (optional) ⚙ | Mutate the implementation; if the AC tests stay green, they are too weak — the deterministic twin of Agent B's adversarial check, giving correlated-failure protection from two uncorrelated directions |

G5 encodes a principle worth stating: in a derived graph, **dangling edges are the linter**. Traditional traceability systems rot because broken links fail silently; under the provenance law, a broken link is a build failure with a `file:line` to fix.

### 6.4 Session resume via graph query

The resumable-state principle of §2.2 upgrades from convention to query: a resuming agent runs `spine context INT-042` and receives the intent doc, its ACs and their current test results, declared touchpoints, the constitution version it was built under, and any ADRs touching the same code units — assembled from the graph, scoped to the task, with zero reliance on anyone's chat history.

---

### 6.5 The dependability suite ⚙

The field's frontier question has shifted from "can agents code?" to "can we depend on agent work?" — and dependability is measured, not asserted. Three commands, all reading data the graph already collects:

- **`spine stats`** — cycle time per intent, **token cost per intent** (the A↔B loop spends real money, and "what does a feature cost" is a month-one question), A↔B bounce-back counts, tripwire fire rates by gate, quick-lane escalation rate, break-glass override counts, reopen counts. This is what turns the playbook's thresholds (400 changed lines, 14-day staleness, 6-AC cap) from guesses into evidence, and what tells you when warn-before-block mode has earned the right to block. Output is text; someone else can chart it — no dashboard UI in scope.
- **`spine review <id>`** — when a tripwire fires, the reviewer receives an assembled packet: the intent doc, the tests grouped by AC, the diff, and exactly which wire tripped and why. Review fatigue is the documented killer of gated workflows; a good packet is the antidote, and it keeps the review anchored on tests-versus-intent rather than line-by-line code reading.
- **`spine eval`** — a golden-set harness for the interview agent: replay past intents, score AC testability and non-goal coverage against how those intents actually played out. The riskiest assumption in the whole system (does the interview produce genuinely testable ACs?) graduates from a thing we spot-audit to a thing we regression-test.

### 6.6 Post-merge lifecycle

Intents do not end at "merged" — production disagrees. Two further states close the loop:

- **reverted** — the revert commit links back through the graph (`changeset → implements → intent`), the intent is marked, and a revert is the loudest possible input to the learnings loop: it should almost always produce an ADR.
- **superseded** — a later intent that replaces this one carries a `superseded_by` edge, so archaeology queries return the current truth first and the history behind it.

Both are derived from git like everything else: a revert is detected, never declared.

---

## 7. Threat model and agent sandbox ⚙

Spine-kit auto-merges machine-written code. From an attacker's perspective that is the product: get code past the tripwires and it reaches main with no human. Everything above hardens the pipeline against *accidents*; this section hardens it against *adversaries*. A `SECURITY.md` derived from this section ships with v1.

**Least privilege per pipeline stage.** Each stage gets only the capabilities its job requires:

| Stage | May read | May execute | Network |
|---|---|---|---|
| Interview agent | repo, ADRs, constitution | nothing | none |
| Agent A (tests + implementation) | scoped code-graph context | sandboxed build/test only | none |
| Agent B (adversarial) | intent doc + tests only (§4.2 isolation — a security control, not just a correlated-failure one) | sandboxed test runs only | none |
| CI gates | the graph | gate queries | none |

**Injection defense.** Repository content — code, comments, README files, dependency docs — is *data, never instructions*, in every agent prompt. An instruction embedded in a source comment ("ignore previous constraints and…") is content the agent reasons *about*, not a directive it follows. This rule is written into every shipped agent prompt and tested in `spine eval`.

**Pragma provenance.** A `@verifies` pragma is trusted only when introduced by a changeset that `implements` a signed intent. A pragma appearing from anywhere else — a drive-by commit, an unrelated branch — indexes as *unattributed* and fails the Integrity family. Test attribution gets provenance like everything else in the spine.

**Auto-merge hardening.** Reread the tripwires (§5.2) as security controls, because they are: no new dependencies, no schema/auth/public-API changes, and diff-size caps are supply-chain defenses first and quality gates second. The audit chain for any auto-merged change is complete by construction: signed intent → attributed tests → gate results → merge, every link a graph edge with provenance.

**Break-glass, not backdoors.** Emergencies are real: a 2 a.m. hotfix blocked by a false-positive gate must have a path forward, or the team disables the gate permanently the next morning — the documented death of every gated workflow. `spine check --break-glass "<reason>"` merges anyway, records an override node in the graph (reason, signer, timestamp), surfaces in `spine stats`, and auto-opens a retro item that should usually become an ADR. A gate you can never override gets turned off; a gate you can override *loudly* survives its first incident.

---

## 8. Failure modes this playbook is designed against

| Failure mode | Countermeasure |
|---|---|
| Agent drifts from intent | Mandatory non-goals; touchpoint tripwires; AC-named tests |
| Specs take too long | One-page template; 15-minute rule; split-don't-grow; agent-led interview |
| Stale specs mislead agents | Intent docs deleted at merge; only constitution is permanent, and it's capped and owned |
| Losing context between sessions | Constitution + ADRs + in-flight intent doc = full resumable state, all in-repo |
| Regressions slip through | ACs compile to tests; tests frozen during implementation; coverage on changed lines |
| Agents rubber-stamp each other | Context isolation + adversarial framing for the cross-check |
| Auto-merge ships something risky | Tiered tripwires route risky diffs to humans |
| Conventions diverge on greenfield | Week-one constitution, loaded by every agent session |
| Process bloats back into SDD | The three-layer rule: one permanent prose file, everything else disposable or executable |
| Traceability rots via silent broken links | Provenance law + dangling-edge linting (G5): a broken link is a loud build failure |
| Graphs become bureaucracy | Iron rule: derived, never authored — every graph is a gitignored cache rebuilt by one command |
| Concurrent intents collide on the same code | Touchpoints as soft leases; G7 interference check at sign-off; "ground moved" re-verify flags on every merge |
| Constitution rules stay aspirational prose | Rule IDs + `enforced_by:` checks; `spine check --constitution` reports the enforced/aspirational ratio |
| Reviewers burn out when tripwires fire | `spine review` packets anchor review on tests-versus-intent, not line-by-line code |
| Concept count creeps up version by version | The complexity budget (§10): human-side limits audited at every version bump, busting a number requires an open argument in the PR |

---

## 9. Adoption notes

**Week one (greenfield):** write the constitution as a team before meaningful code exists; assign its owner; set up the CI skeleton (AC↔test mapping check, tripwire evaluation) even if it starts permissive.

**The dogfooding rule:** spine-kit's first intents are spine-kit's own. The toolkit's repository runs its own pipeline from the first commit, making it its own first case study — every gate false-positive and interview weakness is felt by its builders before any adopter.

**First two sprints:** run with auto-merge *disabled* — every merge gets a human glance — while you calibrate tripwire thresholds and learn where the adversarial cross-check catches real problems. Turn on tiered auto-merge only once the tripwires have earned trust.

**Known open risks to revisit after a month of use:**

- Is the interview agent producing genuinely testable ACs, or plausible-sounding ones? Spot-audit five intent docs against their tests.
- Has the constitution stayed under 150 lines? If it grew, something that belongs in ADRs or tooling leaked in.
- Are people splitting tasks when they hit the 15-minute wall, or padding the doc? Watch cycle-time per intent doc.
- Is anyone quietly reopening intent docs to weaken ACs mid-flight? Reopens should be visible events, not silent edits.

**Spine-kit tooling roadmap (build order, each step useful on its own). v1 ships exactly four commands — `spine init`, `spine new`, `spine index`, `spine check` — and nothing else (four, not three: the §10 budget was amended in v0.6, argued openly as the budget requires — `init` runs once per repo, is the entire on-ramp, and hiding bootstrap inside `new`'s first run would be implicit magic):**

0. `spine init` — bootstraps the repo (constitution scaffold, `.gitignore` entry, CI snippet, AGENTS.md wiring) and runs the **constitution interview**: the interview agent's first job is interviewing the *team*, turning the week-one constitution meeting from a blank page into a facilitated 30 minutes.
1. `spine new` — runs the interview (§3.4) and emits the filled template; template variants (`--change`, `--bug`) per §3.5. Reads/writes AGENTS.md as the agent-context substrate (never a proprietary format).
2. `spine index` — builds the traceability graph (§6.2) from the repo; `spine check` runs the gate families in CI, starting in **warn-before-block mode** so the drift gate earns trust before it enforces.
3. `spine context <id>` — session resume via graph query (§6.4).
4. Code graph via tree-sitter — touchpoint proposal in the interview, graph-containment tripwires (§5.2 ⚙), and the mechanical quick-lane router (§3.5).
5. End-to-end A/B orchestration against the transition table (§6) — the bounded adversarial loop run automatically; adopt a workflow engine behind the table only if retries/budgets/resumability genuinely demand one. Optional G6 mutation gate for high-assurance teams.
6. Dependability suite — `spine stats`, `spine review`, `spine eval` (§6.5), plus G7 interference (§5.4) and constitution enforcement reporting (§2.1); this is the release where thresholds graduate from defaults to evidence.

---

## 10. The complexity budget

The DNA — Lightweight, Organised, Harness — rendered as numbers this playbook must pass at every version bump. Any future addition that busts a number must argue for changing the budget openly, in the PR that proposes it:

| Budget | Limit | Currently |
|---|---|---|
| Mandatory human gates | 1 | 1 (intent sign-off) |
| Human-authored artifact types | 3 | 3 (constitution ≤150 lines · ADR ≤1 paragraph · intent ≤1 page) |
| Pages per intent | 1 | 1 |
| Lanes | 2 | 2 (quick, gated) |
| Graphs | 2 | 2 (traceability, code) + 1 transition table |
| Diagrams in this playbook | 2 | 2 (§1 structural, §5.1 sequential) |
| v1 CLI commands | 4 *(amended from 3 in v0.6 — argued openly, per this table's own rule, to admit `spine init`)* | 4 (`init`, `new`, `index`, `check`) |
| A↔B adversarial rounds | 2 | 2, then human |
| Gate overrides | 0 silent | break-glass only — always recorded, always retro'd (§7) |

Complexity is allowed to grow only on the machine side — gates, queries, derived state. The moment it grows on the human side, this table catches it. "Are we over-complexifying?" stops being a vibe question someone must remember to ask.

---

*This playbook is itself governed by its own rules: keep it short, change it by PR, and delete anything that a machine could enforce instead.*
