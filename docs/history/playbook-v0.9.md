# The Spine-Kit Playbook
### Drift-gated, intent-first development for AI-assisted teams — lightweight, organised, self-enforcing

**Version:** 0.9 · **Status:** design frozen — build reference · **Owner:** _assign before adoption_

> This playbook is the reference design for **spine-kit** — a spec-kit-style toolkit (CLI command: `spine`) that automates the workflow described here. Sections marked ⚙ describe behavior spine-kit will eventually enforce mechanically; until then, teams apply them by convention.
>
> **v0.9** answers a third adversarial design review (`docs/reviews/2026-08-26-codex-adversarial-review-v0.8.md`), each version answering the review of the one before. v0.7 made the three central guarantees — rebuildable provenance, frozen tests, safe auto-merge — mechanically enforceable; v0.8 made the text stop claiming more than the mechanism delivered; v0.9 closes what the reviews kept finding underneath both — that a *mode* could always be chosen in which a guarantee did not hold. Auto-merge is now a capability a run computes from evidence it carries, never a preference a team sets (§7.4 rule 5); deferred reconstruction is bounded, recorded and blocking (`C-M5`, G9); a rollback must be an exact restoration of one named ancestor, not a manifest that resembles it (§7.5, G16); and a module inside `expected` that no static rule can classify is decided by the human who signs the approval, never silently frozen or silently excused (§4.3). §12 maps every finding to where it is closed — and lists what is not. §11 is the vocabulary every section shares: trailer names, key roles, states, gates, CLI flags. Where prose and §11 disagree, §11 wins.

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
| Lightweight | Intent doc (one page) | Disposable — file deleted at landing; its signed bytes are sealed into the landing commit | Feature-level drift, slow specs |
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
                   (signature over the intent's exact bytes)
                          ▼
              ┌──────────────────────────────────────────────────┐
              │ LAYER 3 · HARNESS (executable)                   │
              │   Agent A: ACs → failing tests                   │
              │        ↑ bounce-back on weak tests               │
              │   Agent B: adversarial cross-check (isolated)    │
              │   approval record: tests frozen by blob id       │
              │   Agent A: implement until green (tests frozen)  │
              │   CI: gate families ×5 · tripwires · attested    │
              └───────────┬──────────────────────────────────────┘
                          │   clean → land by compare-and-swap · tripped → human review
                          ▼
              land: signed intent sealed into the landing commit · file deleted
                          │
   ╔══════════════════════╧═══════════════════════════════════════╗
   ║ THE SPINE — derived traceability graph  (`spine index`)      ║
   ║ intent ─ AC ─ approval ─ test ─ code_unit ─ changeset ─ ADR ║
   ║ SQLite · gitignored · rebuilt on demand · queried by         ║
   ║ `spine check` (all gate checks) · `spine context` (resume)   ║
   ╚══════════════════════════════════════════════════════════════╝
```

The double-lined box at the bottom is deliberate: every layer above it feeds the spine, nothing in the spine is written by hand, and it is read only through `spine` commands — never by hand. (§5.1 shows the same system as a sequential pipeline; this is the structural view. Two diagrams is the budget — resist adding more.)

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

**Unique selling proposition.** Spine-kit is the only toolkit where specs cannot go stale — because the intent file is deleted at landing (its signed, frozen bytes live on as a git object in the landing commit, §5.5), traceability is derived rather than authored, and drift blocks the merge instead of waiting to be noticed. Its second headline is the adversarial test cross-check (§4.2): validated by research (AdverTest, UAgent, SWE-ABS, TDAD), shipped by no competitor. Its third: every landing is a hash-bound, signed record that an offline clone with nothing but git objects can re-verify (§5.4–5.5, §7) — auto-merge you can verify without a hosting provider, and proved reconstructible before the push rather than after (G10). The record attests what the pipeline checked: every hash, signature, base and gate query recomputes offline. The one input it cannot recompute is whether the candidate's own runner reported honestly — bounded in §7.4, not papered over.

**Adopted from the field:** AGENTS.md as the interop substrate for agent context (never invent a new context-file format); delta-scoped change specs (OpenSpec) as the model for brownfield change-intents; a genuine quick lane, answering the market's loudest complaint ("a quick bug fix gets the same ceremony as a new feature"); EARS-style phrasing for acceptance criteria plus property-based testing (Kiro); a warn-before-block calibration mode for the drift gate; scaffolding assets embedded in the binary so `init` works offline, and hash-tracked install manifests that refuse to overwrite modified files (both spec-kit — spine-kit adds pristine-blob rollback, §6.7).

**Deliberately refused:** multi-persona agent theater; IDE lock-in; credit-metered pricing; per-feature document suites (the cautionary tale: a 444-line generated contract for a module a quarter that size); spec-as-source code generation; cloud-only architecture; a per-agent command-directory matrix (spine-kit writes one `AGENTS.md` block and nothing agent-specific; the CI snippet is the only provider-specific rendering it performs); template and command override points (the template never expands, §3.3, so there is nothing to override); git notes, PR descriptions, or any provider metadata as a source of truth. Spine-kit stays CLI-first, local-first, MIT-licensed, with a local SQLite graph and no mandatory API key.

---

## 2. Layer 1 — The Constitution and ADRs (Organised)

### 2.1 The Constitution

One file per repository (`CONSTITUTION.md`, or folded into `CLAUDE.md` / `AGENTS.md` so agents load it automatically). It contains the durable, rarely-changing truths of the codebase: the stack, the architectural shape, naming conventions, testing rules, and the non-negotiables.

Rules that keep it alive:

- **Hard cap: ~150 lines.** Past that, agents skim it and humans stop reading it. Brevity is what makes it authoritative.
- **Written in week one on greenfield projects.** This is non-optional. Without it, your conventions get defined by whichever agent ran first, and every developer's agent invents a slightly different codebase. Week-one constitution-writing is the single highest-leverage meeting of a greenfield project — and `spine init` ⚙ facilitates it: the interview agent's first job is interviewing the *team* to draft the constitution.
- **Changes only via pull request.** The PR discussion is your governance. No verbal amendments. ⚙ The constitution is on the protected floor (§7.3): a change to it never auto-merges, whatever an intent declares.
- **It is versioned** (v1, v2, v3…) and every intent doc records which version it was built under, so mid-flight rule changes never become an argument three weeks later. ⚙ A version bump that changes what an in-flight intent must satisfy carries a `resign: true` flag in its header; only flagged bumps trip G4 (§6.3) — a typo fix does not reopen every intent in flight.
- **It has a named owner.** Unowned constitutions rot in about a month. The owner's job is not to write it alone — it is to keep it honest and small.
- **Rules carry IDs, and rules grow teeth. ⚙** Every rule is numbered (`C-1`, `C-2`…) and optionally carries an `enforced_by:` field pointing at a real check — a lint rule, a dependency constraint, a grep probe, or an LLM-judge for the genuinely fuzzy ones. `spine check --constitution` reports each rule as **enforced** or **aspirational** (a failing `enforced_by` check on `T` is a wire; for `C-T3` it is `class=protected`, never warn mode), and the enforced ratio is a health metric: a constitution that is mostly aspirational is a wish list, not a constitution.

```markdown
C-4: No module may import from `auth/` except through `auth/api.ts`.
  enforced_by: depcruise:no-auth-internals
C-7: Prefer composition over inheritance.
  enforced_by: (aspirational)
```

**Twelve rules ship with teeth. ⚙** `spine init` scaffolds them in lettered families so they never collide with the team's own `C-<n>`; each is machine-enforced, and together they cost about twenty of the 150 lines — the price of making §4.3, §5.4 and §7 enforceable rather than aspirational:

```markdown
C-A1: mode: team                          # solo ⇔ exactly one signoff key (§7.2)      enforced_by: spine:G13
C-A2: protected: adr/, db/migrations/     # extends the floor of §7.3; never shrinks it  enforced_by: spine:G14
C-T1: test roots: tests/, src/**/__tests__/                                            enforced_by: spine:G8
C-T2: test support: tests/support/**, **/conftest.py, pytest.ini, pyproject.toml, jest.config.*   enforced_by: spine:G8
C-T3: no test-framework imports or `pytest_*` hook definitions outside test roots      enforced_by: grep
C-M1: merge.strategy = merge              # merge | squash — rebase landings are refused (§5.5)   enforced_by: spine:G9
C-M2: merge.reverify = full               # scoped only once the code graph proves disjointness  enforced_by: spine:G11
C-M3: merge.retries = 3                   # base-moved runs before a human is asked (§5.4)        enforced_by: spine:G11
C-M4: merge.auto = off                    # `on` is a request; §7.4 rule 5 decides per run        enforced_by: spine:G11
C-M5: merge.reconstruct = inline          # inline | scheduled:<n landings> — scheduled ⇒ no auto-merge enforced_by: spine:G10
C-Q1: quick.paths = docs/, src/**         # touchpoints the quick lane may touch (§3.5)           enforced_by: spine:G2
C-Q2: quick.max_lines = 400               # the diff-size wire (§5.2)                             enforced_by: spine:G2
```

The keys themselves are *not* in the constitution — they live in `.spine/allowed_signers` (§7.2), git's own keyring format, so the 150-line cap survives a ten-person team.

### 2.2 Architecture Decision Records (ADRs)

An append-only folder (`adr/`), one short file per decision: what we decided, why, and what we rejected. One paragraph each is fine.

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

### 3.1 The template (v2)

```markdown
# INT-042: <short imperative title>
Owner: @name · Template: v2 · Ticket: <link> · Constitution: v3
Supersedes: INT-017                        (optional)

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

**There is no `Status:` line — v2 removed it.** Status is derived from git (§6): the signed sign-off makes an intent *signed*, the approval record makes it *tests-approved*, the landing commit makes it *merged*. A status a human can hand-edit is a status nobody can trust — and, decisively, the sign-off signs the file's exact bytes, so a mutable header would break its own signature. Template v1 files still parse; their `Status:` field is ignored. `Owner:` is a hint for humans; `signed_by` in the graph is the truth.

### 3.2 Why each field earns its place

**Goal** is outcome-phrased so review discussions stay about intent, not implementation taste.

**Non-goals are mandatory (minimum two)** because drift is rarely an agent ignoring the goal — it is the agent *over-serving* it: adding caching nobody asked for, "improving" adjacent code, handling corner cases that don't exist. Naming what not to do is far cheaper than describing everything to do. This is the highest-leverage sixty seconds in the document.

**Acceptance criteria are capped at six**, and the cap is a scoping tool disguised as formatting. When someone hits AC-7, the template itself says "split the task" — nobody has to police scope in review. Each AC has an ID because IDs flow downstream: tests are named after them (`test_AC1_...`), CI verifies every AC has a matching test, and "did the agent follow the spec" becomes a mechanical check instead of a vibe.

**Touchpoints — especially "must NOT change" — is the tripwire.** When a diff touches `auth/` and the doc said it wouldn't, that is no longer a judgment call; a CI bot flags it in seconds. Cheapest drift detection available. Touchpoints also do double duty as auto-merge policy (§5.2) and as the lease other intents coordinate around (§5.4). What touchpoints can *not* do is admit the machinery that evaluates them: the protected floor (§7.3) sits above every declaration.

**Constitution version** pins which rules the feature was built under.

**Open questions must be empty before implementation begins.** This converts "the agent assumed" into "the agent asked."

### 3.3 Rules that keep it lightweight

- **The 15-minute rule.** If the intent doc takes longer than fifteen minutes to produce, the task is too big. **Split the task; never grow the doc.** Spec-driven workflows fail when people spec epic-sized work; they succeed at small-feature scope. Scope is the variable — the template never expands.
- **The disposal rule.** The intent file exists only on its own branch (`intent/INT-042`) and is deleted by the commit that lands the work. Its frozen, signed bytes are sealed into that landing commit's message together with the trailers that record base, head, approved tests and gate results — the **intent envelope** (§5.5). The repo stays free of stale specs that could mislead a future agent; the record of what was promised, by whom, against which base, is a git object that every clone carries and nobody can edit without breaking a hash. Permanence without staleness — and without a hosting provider. A PR description is a rendering; the indexer never reads one.
- **The canonical-form rule. ⚙** `spine new` writes the file in canonical form (UTF-8, LF, trailing whitespace stripped, exactly one trailing newline, no Unicode normalisation); `--sign` refuses anything else, refuses any line beginning `-----` or `Spine-` (they would collide with the envelope's own syntax), and hashes the *index* blob (`git hash-object --path`), never worktree bytes — so `core.autocrlf` cannot fork the identity. `spine init` writes `.spine/** intents/** text eol=lf` to `.gitattributes`. The point is mechanical: the bytes in the envelope equal the bytes in the blob, so the intent's identity — its git blob id — is recomputable from the envelope alone.

### 3.4 How it gets written: the agent interview

The human does not write the intent doc from scratch. The human gives a rough, high-level intention (two minutes, verbal or a couple of sentences), and an agent interviews them, then produces the filled template.

Interview protocol for the agent:

1. **Hard cap: 7 questions.** An unconstrained agent will interrogate for twenty minutes.
2. **At least 2 questions must hunt for non-goals** — "Should I also handle X?" Every "no" becomes a non-goal line for free. Agent interviews are the best non-goal extractors that exist.
3. **At least 1 question must stress-test AC verifiability** — "How would we observe that this worked?" If the human can't answer, the AC gets rewritten during the interview, not discovered broken later.
4. The agent asks about blast radius to fill Touchpoints, including what must not change.
5. Anything still ambiguous lands in Open questions rather than being silently assumed.

The human then reads the finished one-pager and signs it off. **This sign-off is the single mandatory human gate in the whole pipeline** — the highest-stakes three minutes in the process. Everything downstream is machine-enforced, which is precisely why the interview must force testable ACs and explicit non-goals here.

**What sign-off is, exactly. ⚙** `spine new --sign INT-042` — TTY only, refused inside an agent session (`SPINE_AGENT=1`) — writes an empty *event commit* on the intent branch carrying `Spine-Event: signoff`, `Spine-Intent: INT-042`, and two signed trailers:

```
Spine-Signoff: INT-042 blob=9f2c… template=v2 constitution=v3 reopens=0 signer=alice@example.com
Spine-Signoff-Sig: <SSH signature over that exact line · namespace spine-signoff@v1>
```

The signature is over the **content** — the intent's blob id — not over a commit. A commit signature dies at squash and says nothing about *which* text was approved; a detached signature bound to bytes survives squash, rebase and branch deletion, and is copied verbatim into the landing commit (§5.5), which is what makes "who approved exactly this wording" answerable from an offline clone years later. `reopens=` is the count of signed reopens on the branch at signing, so a sign-off cannot be replayed after a reopen. Any edit after sign-off changes the blob: the intent derives back to `awaiting-sign-off`, and unless that edit is a signed **reopen** (§4.3) the Integrity family fails — §9's "quiet reopen" risk is closed by construction, not by vigilance. `--sign` refuses a `Template:` version below the manifest's `resign` floor (§6.7). Signing your own intent is authorship, not approval, and it is the normal path for everyone, solo or team; approval of the *code* is the gates' job, and the human moments after this one are conditional (§7.2). The key must be listed in `.spine/allowed_signers` (§7.2) — a signature proves identity; the keyring is what grants authority.

And the `Template: v2` header exists because sealed intents live in git history forever and the indexer must parse every generation of them; `spine new` stamps the version from the install manifest (§6.7), never from the binary, so one developer's newer binary cannot fork the team's template.

### 3.5 Two lanes, three templates ⚙

Not all work deserves the pipeline — forcing full ceremony onto small tasks is the most common way SDD tooling dies in practice, because developers skip the process exactly where AI drifts most. Spine-kit has exactly **two lanes**:

- **Quick lane** — no intent doc. A candidate is a branch under `quick/*`, landed by `spine check --land --quick <branch>`. It qualifies only when the diff is under `C-Q2` lines, stays inside `C-Q1` paths, and touches no schema/auth/public API. The constitution and CI gates still apply; the wires *are* the lane boundary — a quick-lane change that trips a Drift or Strength wire is escalated to the gated lane (`spine new --from <branch>`; G12 measures red with `expected` restored to base, so the promoted code is invisible to it: the only defense against tests written to fit existing code on this path is B's isolation (§4.2), and `spine stats` counts promotions separately). Three more boundaries: a diff that touches **test roots or harness configuration** (`C-T1`/`C-T2`) is escalated, because the harness is what every other gate rests on; one that intersects **another in-flight intent's lease** (§5.4) is escalated; and one that adds a `@verifies` pragma is escalated, because attributing a test to an intent is by definition gated-lane work. A floor hit (§7.3) does not change lane — it adds a protected review in place, which is how toolkit upgrades (§6.7) and reseals (§5.5) land. Warn mode never applies to the router. The router can be mechanical: predicted blast radius from the code graph decides the lane, not vibes. Quick-lane changes land through the same landing step as everything else, with a minimal envelope — so every commit on trunk belongs to a sealed changeset.
- **Gated lane** — the full pipeline of §5.1, with **three template variants** for the intent doc:
  - **Feature** — the standard template of §3.1.
  - **Change (brownfield)** — "Goal" is replaced by **Current behavior → Target behavior**, and a mandatory **Invariants** section lists what must remain true. Deltas against existing behavior fit modification work better than greenfield-style goals.
  - **Bug** — a `BUG-` intent where the reproduction *is* AC-1: the test must fail before the fix and pass after — and G12 (§4.3) refuses the approval outright if it doesn't. Bugs are the natural home of the test-first flow.

Two lanes to route between, three templates within one of them — that is the entire taxonomy. One page, six ACs, fifteen minutes: the caps apply identically everywhere.

---

## 4. Layer 3 — The Harness (executable, not prose)

The harness is not markdown. It is tests, type checks, linters, and CI gates — because drift a document catches depends on someone re-reading the document, while drift a test catches is caught on every single run.

### 4.1 The AC → test flow

1. **Agent A** reads the signed-off intent doc and writes **failing tests**, one or more per acceptance criterion, named after the AC IDs (`test_AC1_invoice_totals_include_tax`).
2. **Agent B cross-checks adversarially** (see §4.2). If B finds a hole, the tests bounce back to A. Loop until B fails to break them.
3. **The approval record freezes the tests** (§4.3): a signed commit naming every approved test file by blob id, the fixtures and runner configuration they depend on, and the count of tests that were red at approval.
4. **Agent A implements until green.** The tests are the contract; the implementation is whatever satisfies it within the constitution's rules. A frozen byte that changes is a gate failure, not a judgment call.
5. CI verifies mechanical compliance on the **synthetic merge** of the branch onto trunk (§5.4), never on the branch alone: every AC ID has at least one matching test, every frozen test id and every test trunk already had is reported *passed*, lint passes, types pass, coverage holds on changed lines.

Humans review tests only when a tripwire fires (§5.2) — and when they do, they review *tests*, not implementations. Reviewing tests is roughly an order of magnitude cheaper and is the moment a human confirms the agent understood intent, before any implementation exists to be attached to.

### 4.2 The adversarial cross-check

The known trap of agent-checks-agent is **correlated failure**: two similar models with similar priors can share the exact same misreading of an ambiguous AC, and green tests then prove only that the code matches the tests — not that the tests match intent. Three mitigations are mandatory:

- **Context isolation.** Agent B receives *only* the intent doc, the tests, and — on brownfield code, where a test imports modules B has never seen — the code graph's *interface slice* of the expected touchpoints and the frozen closure: signatures, types, fixture names; no bodies, no diff, no conversation (`spine context <id> --role B`, roadmap 3–4). Not the implementation, not Agent A's conversation, not the ticket thread. B must independently derive what the tests should assert. Isolation is what breaks the shared-context correlation — it matters more than using a different model vendor (though a different model family is a cheap additional hedge).
- **Adversarial framing.** B's task is not "review these tests." It is: **"Write an implementation that passes every one of these tests while violating the intent doc."** If B succeeds, the tests are too weak and go back to A with B's counterexample attached. Adversarial framing catches weak assertions that a friendly review waves through.
- **Bounded loop.** The A↔B exchange is capped at **two rounds**. If B can still break the tests after two hardening passes, the ambiguity is in the ACs, not the tests — the intent routes to a human instead of burning tokens on an unwinnable ping-pong. An anti-runaway harness must not itself contain a runaway loop. A reopen (§4.3) grants a fresh budget, but B's packet for a reopened intent includes B's own prior counterexamples — intent history, not code — so resubmitting the same tests after a reopen does not reset the argument, and `total_rounds=` on the approval keeps the cumulative count visible.

### 4.3 Test immutability is a checkpoint, not a rule ⚙

"Agent A may not modify approved tests" is discipline unless something records *which bytes* were approved and refuses anything else. Spine-kit records the approval in git and refuses in CI.

**The approval record.** The transition to `tests-approved` is a signed, empty commit on the intent branch (`spine check --approve INT-042`, which refuses a dirty worktree and freezes the branch HEAD's tree). Alongside `Spine-Event: approve` and `Spine-Intent`, its trailers are the contract:

```
Spine-Approve: INT-042 intent=9f2c… base=7b0d… rounds=1 total_rounds=1 reopens=0 red=5/5 freeze=sha256:… signer=alice@example.com
Spine-Approve-Sig: <SSH signature over the Spine-Approve line · namespace spine-review@v1>
Spine-Frozen: a41b… tests/billing/test_invoice.py
Spine-Frozen: c07e… tests/conftest.py
Spine-Frozen: 58d2… tests/fixtures/invoices.json
Spine-Frozen: 1e9f… pyproject.toml
Spine-Test: tests/billing/test_invoice.py::test_AC1_totals_include_tax
Spine-Test: tests/billing/test_invoice.py::test_AC2_zero_rated_lines
```

`intent=` is the signed intent blob; `base=` the trunk tip at approval (audit data — the keyring that verifies a landed approval is the seal's, §7.2); `rounds=` the A↔B bounce-backs consumed this time and `total_rounds=` across reopens (G13 checks it against the earlier approve lines while they are reachable on the branch); `reopens=` how many times this intent has been reopened; `red=k/n` how many frozen tests failed at approval (below); `freeze=` a SHA-256 over the sorted `Spine-Frozen` and `Spine-Test` lines — a non-git digest, used to name the approval elsewhere. `Spine-Frozen` lines are `<blob id> <path>` with `git ls-tree` quoting: a path says where a test lives; a blob id says what it asserts. `Spine-Test` ids are collected *function* ids without parametrization suffixes; G1 requires every collected parametrization of each to pass. Why a commit, not a tag or a note: a commit is an ancestor of every implementation commit after it, travels with a plain fetch of the branch, and cannot be removed without rewriting the branch — which the branch's non-fast-forward rule refuses (§7.4). Why empty: the state *is* the commit; nothing in the tree changes, so there is no status line to flip.

**Who signs it.** In v1 — before A and B run in the trusted stage (roadmap step 5) — the developer who ran the B loop signs under `spine-review@v1`, attesting *B's verdict*, not code quality; self-signing an approval is therefore permitted in every mode. When a tripwire routed the tests to a human (loop cap, green-at-approval, closure tripwire — state `approval-review`), the human reviewer signs with a reason. Once orchestration ships, B's run is attested by the pipeline key: the approve line gains a `run=` digest of B's transcript blob and verifies under `spine-seal@v1` — a line without `run=` verifies under `spine-review@v1` only, never both. Roles are derived from which namespace the signature verifies under (§7.2), never claimed in the trailer.

**What is frozen: the closure, not the file list.** A frozen test that imports an unfrozen fixture is frozen in name only — A weakens the fixture and the test passes. `--approve` computes the closure and freezes all of it: (1) every test file with a `verified_by` edge to the intent; (2) their transitive repo-local imports — an import that resolves inside the intent's `expected` touchpoints, outside every `C-T1`/`C-T2`/runner-config pattern, *and* into a module that existed at the approval's `base=` and was imported there by a non-test file, is the code under test and is excluded; everything else in the walk is frozen, and an import that resolves outside both expected and the harness is frozen as a leaf, because A had no business touching it; (3) runner configuration and package `__init__.py` files on the path from repo root to each test — a root `conftest.py` can deselect every test below it without touching one; (4) snapshot and golden files under test roots — an expectation written by the implementation is not a test of intent, so it exists before approval or the test does not. Runner-config patterns match at any depth, including inside `expected`. Clause (2)'s `base=` test is what stops an oracle hiding at the address of code under test: a module that already existed on trunk and that trunk's own non-test code never imported is test-only, and freezes as a leaf. It is read from the base tree, which the branch cannot edit — buying an exclusion means landing the importing line on trunk first, through a full gated intent, which is visible and reviewed. A module that does not exist at `base=` cannot be classified this way at all: it may be the stub the red tests import, or an oracle A wrote to compute their expected values, and nothing static separates the two before an implementation exists. So it is neither silently frozen nor silently excused — it is a **closure tripwire**: `--approve` lists every branch-created module inside `expected` that a frozen test imports, and the human signing the approval names each one code-under-test or test-only in `reason=` (mandatory, and G13 refuses its absence, §11). Test-only ones join the frozen closure; the rest are excluded and counted. In v1 a human already signs every approval (below), so the cost is a sentence, not a gate; when the code graph lands (roadmap 4) its call edges can propose the answer, and the human still signs it. Resolution is static and environment-independent: repo-local imports resolve from the tree alone, by the pinned release's resolver for `params.lang`, never from an installed environment; re-exports count as imports, type-only imports do not; a module whose imports cannot be resolved statically is unclassifiable and stays excluded, counted by `spine stats`. Two residuals follow, stated rather than closed: an oracle A creates on the branch, and one the implementation also imports. Noticing them is Agent B's job (§4.2); G6 is the signal that assertions stopped biting. Approval tripwires route to a human instead: an unresolvable or dynamic import inside test roots; a closure over 200 files (the harness is too entangled to freeze honestly — a threshold `spine stats` should turn into evidence); an `expected` entry matching any `C-T1`/`C-T2`/runner-config pattern (frozen paths are exempt from G2, so declaring them is a request to change the harness mid-flight); an AC whose only `verified_by` edges are pragmas in files no runner collected; non-deterministic test ids across two collection runs. In `--ci`, G8 recomputes the closure over the approval commit's tree with the pinned release and fails if any file it computes is missing from `Spine-Frozen` — an approval signed by a newer or older binary cannot under-freeze.

**The freeze gate (G8).** On every `spine check`, for each frozen `(blob, path)`: the blob in the synthetic merge `T` must equal the approved blob — or equal trunk's current blob at that path, meaning the change arrived from trunk and not from this branch (that is *harness moved*, a wire of its own, below). Anything else — modified, deleted, renamed — fails. **After approval the harness is read-only from the branch's side:** any path matching a `C-T1`/`C-T2`/runner-config pattern — or frozen by an approval that a landing on trunk copied or names (`Spine-Approval`, `Spine-Frozen`), which is what keeps an in-`expected` leaf protected after its own intent lands — whose blob in `T` differs from both the approval tree and trunk's tip — added, modified, deleted or renamed, frozen by this approval or not — fails G8, because a test from a *landed* intent is exactly as much a contract as this intent's own. Before approval the harness is not read-only, but it is reviewed: a harness path present at `B` whose blob in `T` differs from `B`'s is a `class=protected` wire `G8:<path>` at landing — the only way a landed test or shared fixture changes — and a landed id `T` no longer collects, or does not pass, is a G8 failure unless that review names its path. So does any `@verifies` pragma or `test_AC*` name for this intent that first appears after approval (it indexes as unattributed and fails G5, §7.1), and an intent blob that differs from the signed blob. **One gated intent per branch**, enforced by `spine new` (branches only from trunk) and by the landing step (refuses a delta that adds or removes ≠ 1 intent file). G8 never runs in warn-before-block mode: blob equality has no threshold to calibrate and no false positives, and it protects the thing that makes every other gate meaningful. Its only exits are a signed reopen or a break-glass review (§7.6), which `spine stats` counts separately as a *freeze override* — the override that should always end in an ADR.

**Harness moved.** A frozen path whose blob in `T` equals trunk's but not the approved one was changed by a landing on trunk. The frozen ids rerun on `T` — and a human confirms (state `landing-review`, wire `G8:<path>`), unless every first-parent landing between the approval's `base=` and `B` that changed the path carries a `Spine-Review class=protected` whose wires include `G7:<path>` or `G8:<path>`: a mover that was already reviewed as a harness change (§5.4) does not need reviewing twice. One accepted residual: a shared fixture legitimately weakened by a protected-reviewed landing is accepted on every intent that froze it after its ids rerun — the hard lease on the mover's side is best-effort (it sees the refs it fetched); the tripwire on the frozen side is the guarantee, and G6 the signal (§6.3). An in-`expected` leaf also joins the frozen set G7's hard lease reads, so a landing that touches one takes a `class=protected` `G7:<path>` review; `spine stats` counts them, and a rate that does not fall means the harness is entangled with the code under test.

**Skipping is modifying.** A skipped test is a passing test to a naive gate. G1 therefore checks results *by identity*: every `Spine-Test` id — and every test id trunk collected on `B` — must appear in the report for `T` as *passed*: not skipped, not xfail, not deselected, not absent. Whether the skip came from a decorator, a `-k`, an environment variable, a collection hook in a new `conftest.py`, or a `pytest.skip()` raised at import time inside the implementation, the id is missing from the passed set and G1 names the id that vanished. An id collected on `B` but absent from `T`'s collection is a G8 failure unless the protected `G8:<path>` review names its path. A landed intent's tests are the floor for every later landing. `C-T3` (no test-framework imports or hook definitions outside test roots) is the cheap answer to an implementation that monkeypatches the assertion library.

**Red at approval (G12).** `--approve` runs the frozen ids against the approval tree *with every path under the intent's `expected` touchpoints restored to its `base=` blob* — red without this intent's code, so implementation commits that precede a late reopen cannot game it — and records `red=k/n`: ids are collected on the approval tree, and on the restored tree an id that errors, fails to import or is not collected counts as red — red means *not passed*. `k = 0` — tests green before any implementation exists — proves nothing about the ACs and is the signature of tests written to fit code that already exists. It is a tripwire: the approval is refused unless a human signs it with a reason. For `BUG-` intents the reproduction AC must be red or the approval is refused outright.

**Reopen is a transition, not an edit.** If implementation reveals the tests are wrong, that is an intent problem, and the only way to change a frozen byte is `spine new --reopen INT-042 --reason "…"`: the commit that changes the intent blob carries a signed `Spine-Reopen` line naming the freeze digest it voids, and returns the intent to `awaiting-sign-off`. A reopen must change the blob — a no-op reopen is refused; when the reopen exists to satisfy a `resign` floor (§6.7), it rewrites the header to the floor version and inserts each new mandatory section as an empty stub, so it always changes the blob. From there the whole sequence reruns: the human re-signs the doc (the one gate, reused — not a second one), A regenerates or edits tests, B attacks them with a fresh two-round budget and its own prior counterexamples (§4.2), and a new approval freezes the new closure. **The binding approval is the newest `Spine-Approve` on the branch whose `freeze=` no `Spine-Reopen` names and whose `intent=` equals the current signed blob**; older approvals are void, a sign-off counts only if it is later on the branch than the last reopen, and `--approve` refuses — and G13 excludes — a `Spine-Approve` on a branch already carrying a verifying one later than the last `Spine-Reopen` with the same `intent=`, unless that approval's key has since left the keyring (the key-removed row of the §6 table): from `tests-approved` the only road to a new freeze is a reopen, and G13 refuses an event commit whose signed line is byte-identical to an earlier one. Implementation commits already on the branch stay; they are simply unverified until the new approval exists, and G12 notices if the new tests were written to match them. "Ground moved and the intent is wrong" (§5.4) is the same human reopen. Every reopen is a number in `spine stats`: reopens per intent, and *late* reopens — those with implementation commits between the voided approval and the reopen — which is §9's "quietly reopening to weaken ACs" turned from a worry into a metric. Reopens are never refused. They are never silent.

---

## 5. The pipeline end to end

### 5.1 Flow

```
human intent (≈2 min, rough)
  → agent interviews human (≤7 questions) → intent doc on branch intent/INT-042
  → HUMAN GATE: `spine new --sign` — signature over the doc's exact bytes   ← the one mandatory human moment
  → Agent A: failing tests from the ACs
  → Agent B: adversarial cross-check (context-isolated)
       B still breaking after 2 rounds → approval-review: a human signs the approval, or reopens
  → `spine check --approve`: signed approval record freezes tests + closure, records red count
  → Agent A: implement until green (G8 rejects any frozen byte that changes)
  → `spine check --land`: gates on the synthetic merge onto trunk's current tip
       clean → landing commit sealed, reconstruction proved on a clean clone, then pushed by CAS
       wire tripped  → landing-review: a human signs over the exact tree, then land
       floor hit     → protected-review: someone other than the signer (team), then land
       tests wrong   → signed reopen → back to sign-off; A, B, approval rerun
  → landing commit carries the signed intent + trailers (§5.5); intent file deleted; branch deleted
```

### 5.2 Tripwires (tiered auto-merge)

A green pipeline lands **only when all of the following hold** — evaluated on the synthetic merge of the branch onto trunk's *current* tip (§5.4), never on the branch alone:

- The integrated diff stays inside the intent doc's declared "expected to change" touchpoints.
- Nothing in "must NOT change" was touched.
- No changes to schema, auth, or public API surface.
- No new dependencies introduced.
- Diff size under `C-Q2` (400 changed lines is a sane start). Spine-owned and floor paths are exempt from this wire, the dependency wire and quick-lane containment — they are renders of a pinned release, verified by blob.
- Every AC ID has a matching test; every frozen test id, and every test trunk already had, is reported *passed* on this synthetic merge; lint, types, and coverage gates pass.
- Nothing in the diff intersects another in-flight intent's **forbidden** set or **frozen** paths (the hard lease, §5.4).
- **No protected-floor hit (§7.3).** CI definitions, `.spine/`, the constitution, agent-context files, `CODEOWNERS`, git hook and attribute files, symlinks, submodule pointers. This is the one line touchpoints cannot override: an intent that declares `.github/workflows/` as expected has declared nothing. A floor hit routes to a *protected* review — reviewer ≠ intent signer where the team has two.
- `C-M4: merge.auto = on`. While it is `off` — the calibration phase of §9 — every landing takes a tripwire review, and the wire it trips is `G11` (`C-M4`).

Any wire fires → a human reviews before landing. The checks are mechanical, so they cost nothing when work is normal — the touchpoints field of the intent doc is literally the merge policy.

⚙ **Auto-merge is not a button.** It is the compare-and-swap of §5.4: every check above runs on the synthetic merge tree, and the result lands only if trunk's tip has not moved since. A green branch is not a fact about trunk.

⚙ In v1, touchpoint checks are path-prefix matching. Once spine-kit ships a code graph (§6), they upgrade to **graph containment**: "did the diff stay inside the declared dependency subgraph?" — which catches indirect blast radius (a change that ripples into `auth/` through a shared helper) that path matching misses.

### 5.3 Roles summary

| Actor | Responsibility |
|---|---|
| Human (feature owner, *signer* key) | Rough intent; answer interview; **sign off the intent doc** (`spine new --sign`); sign reopens and withdrawals; sign toolkit upgrades; review only when a wire fires |
| Human (*reviewer* key) | Sign `Spine-Review` over the exact tree when a wire fires or the floor is hit; for protected and break-glass reviews in team mode, must not be the intent's signer; sign approvals in v1 (attesting B's verdict); seal recovery landings (§7.5) |
| Interview agent | ≤7 questions; extract non-goals; force testable ACs; fill template |
| Agent A | Failing tests from ACs; implementation until green; tests frozen during impl; holds no key |
| Agent B | Context-isolated adversarial attack on the tests; holds no key |
| Untrusted CI stage | Compute the synthetic merge, build and test it in a sandbox — no secrets, no key, network only to an allow-listed registry proxy during dependency restore; hand over the result files trunk's collector wrote, labelled with the tree they tested (§7.4) |
| Trusted CI stage (*pipeline* key) | Run from trunk's own workflow definition, read policy from trunk, run the pinned spine release, rebuild the graph fresh, evaluate all five families on the synthetic merge, seal and land the envelope by compare-and-swap — executes no repository code (§7.4) |
| Constitution owner | Keep constitution honest, small, and versioned |

### 5.4 Multiple intents in flight ⚙ — the serialized merge protocol

A pipeline designed for one intent fails in exactly one place: two intents claiming the same ground. Checking leases at sign-off and re-verifying *after* a merge leaves the window between them open — two intents sign off against the same tip, test against the same stale base, and both reach auto-merge before either learns the ground moved. Closing that window needs no queue service. **Git's ref update is already a compare-and-swap, and trunk is already a serialized log.** The protocol: check the exact tree you intend to land, then land it only if nobody moved the tip.

**Four records, four lifetimes.** Every identity below is a git object id or a digest recorded inside a signed trailer.

| Record | Produced by · when | Bound to | Survives a base move? |
|---|---|---|---|
| Sign-off (`Spine-Signoff`) | signer key · `spine new --sign` | the intent **blob id** | **Always.** Intent is about outcome, not a snapshot of code. Voided only by a reopen (new blob) or key revocation; a `resign` floor trips G4 (§6.3) but does not void it. |
| Approval (`Spine-Approve`) | reviewer key (v1) / pipeline key (step 5) · `spine check --approve` | intent blob + every **frozen blob**; `base=` for audit | **Iff every frozen path's blob in the synthetic merge equals the approved blob, or equals trunk's** (harness moved: rerun and reviewed per §4.3). A frozen blob changed *by the branch* is a G8 failure (blocked, §6 table), not a void. B never saw the base (§4.2 isolation), so its verdict is tests-vs-intent, not tests-vs-code. |
| Review (`Spine-Review`) | reviewer key · `spine check --review` | the **head**, the exact **tree**, the base, the intent blob, the gate report, and the wires accepted | Iff `H` is unchanged, the new synthetic merge is conflict-free, its wire set ⊆ the wires signed, and the base's movement (`git diff --name-only B_old B_new`) touched neither the floor nor any path named in the signed wires. A content push changes `H` and voids it; `tree=` and `report=` then become audit data. |
| Gate record | trusted stage · `spine check --land` | **(head, base, tree)** | **Never.** Void the instant trunk ≠ base or the branch ≠ head. |

**The landing step, exactly.** `spine check --land INT-042`:

1. `B := origin/<trunk>` tip (the local trunk only when there is no remote), `H := refs/heads/intent/INT-042`. If the remote no longer has the branch, stop: the intent has landed or been withdrawn — G9 shows which. `T := git merge-tree --write-tree B H` (git ≥ 2.38; in-memory, no worktree). `H` is the literal ref; the **content head** `Hc` is its nearest ancestor that is not an empty `Spine-Event: review` commit (review commits change no tree, so `merge-tree(B, H) = merge-tree(B, Hc)`). Reviews and the seal name `Hc` in `head=`; wherever this section, the §6 rows or G9 compare a review's `head=` with `H`, read `Hc`; the CAS of step 6 guards the literal `H`. Conflict → `needs-rebase`: merge trunk *into* the intent branch (never rebase a branch carrying event commits — it rewrites the SHAs the approval descends from) and start over.
2. Verify bindings against `T`: the intent blob equals the signed blob; no `Spine-Event: land` for this id is already sealed on trunk; the branch carries a verifying, non-voided approval (§4.3) — a gated branch without one is refused — unless its newest event commit is a verifying `Spine-Withdraw`, in which case `--land` skips steps 2–3, builds the tombstone (parent `B`, tree = `B`'s, no test run; `Spine-Gates` lists only G9, G13, G14, G15) and goes to step 5 — a tombstone carries a full envelope, so it is proved reconstructible like any other landing; every `Spine-Frozen` path in `T` has the approved blob (or trunk's — see the table); any review's `head=` equals `Hc`; signatures verify against trunk's keyring (§7.2). Mismatch → the corresponding record is void; nothing else runs.
3. Fetch `refs/heads/intent/*` fresh and run **every** gate — including the hard lease over the integrated diff — over the result file trunk's collector wrote for `T` (§7.4 rule 3: the trusted stage runs no tests; it ingests one file, whose header must carry `tree=<T>` and a collector pinned by the base). Results labelled with any other tree are not results. The result is the gate record for `(H, B, T)`.
4. Wire or floor hit → the matching review state, bound to `H` and `T`. Clean → build the landing commit `L` with `git commit-tree` (never `git commit` or `git merge`, whose message cleanup rewrites bytes) from `T` with `intents/INT-042.md` deleted in that same tree, parents `(B, H)` (merge strategy) or `(B)` (squash), message = the envelope of §5.5, sealed by the pipeline key.
5. **Prove it reconstructs, before anything is irreversible.** Clone the repo into a scratch dir `S` and push the post-CAS ref set into it — `git push S L:refs/heads/<trunk> :refs/heads/intent/INT-042` — so `S` sees exactly what trunk will see, and the intent is the landing's own source on both sides. The runner's own refs never move: no worktree, no concurrent run and no interrupted run can observe an unpushed `L`, and the guards of step 6 stay intact on the remote-less path, where they are the only transaction there is. **G9 over `L`** — the envelope parses, hashes, and passes the ledger — always runs here and always refuses the push. **G10** (§6.3) runs here too under `C-M5: merge.reconstruct = inline`: index `S`, clean-clone it, canonical dump on both sides, and refuse the push on a non-empty diff; under `scheduled:<window>` it runs later and the seal records which. A refusal discards the landing, ends the run **without re-queueing and without consuming a `C-M3` retry**, and reports the intent `reconstruction-failed` — a deterministic failure re-runs identically, so it is an indexer defect to file against spine, never contention. G10's own result never enters `Spine-Gates`: `L` exists by then and its seal covers its message (§11).
6. **Compare-and-swap.** `git push --atomic --force-with-lease=refs/heads/<trunk>:B --force-with-lease=refs/heads/intent/INT-042:H origin L:refs/heads/<trunk> :refs/heads/intent/INT-042`. Without a remote: a `git update-ref --stdin` transaction with the same two guards. Either ref moved → the push is refused → `base-moved` — the run ends and re-queues on the new tip; `C-M3` counts runs per `(intent, head)`, resets on a human review, and on exhaustion reports the intent `starved` rather than asking a human for nothing. A landing whose push is refused is discarded: reset the local trunk to `origin/<trunk>` and run again; never pull over a sealed landing. `--land` consults `git worktree list`, detaches any worktree holding the intent branch, and deletes the local branch only after the remote CAS succeeds.

Two runners racing for the same tip: exactly one CAS wins; the loser's record is garbage by construction, not by policy. There is no time-of-check/time-of-use gap: the tree is content-addressed and the ref update *is* the check. A seal is valid only for the base it names (G11). Optimistic CAS has no progress guarantee: a trunk that moves faster than one re-verification per landing starves every landing — when `spine stats` shows re-verify counts near `C-M3`, a provider queue as runner (configuration (a), below) is the answer, not a bigger `C-M3`.

**Cheapest re-verification.** `D := git diff --name-only B_old B_new`. Drift gates are pure queries over the new diff — always recomputed, always cheap. Tests rerun **in full** on the new `T` by default (`C-M2: merge.reverify = full`). `scoped` — rerun only this intent's frozen ids — is permitted only when the code graph proves `D` is disjoint from the transitive closure of the intent's touchpoints and tests; path-prefix matching cannot see transitive reach, so v1 is full and `scoped` arrives with the code graph (roadmap 4). The cost is O(queue length) per landing; `spine stats` reports re-verify count per landed intent so the number is evidence.

**Leases.** Declared touchpoints are leases; the registry is **derived from `refs/heads/intent/*`**, which a default clone fetches — no service, no side file — and a lease is derived only from the blob named in a verifying `Spine-Signoff` on that branch: an unsigned or revoked branch contributes no lease (or any pushed branch could declare `Must NOT change: **` and halt every landing). Leases are held from `signed` onward. The lease is *advisory at sign-off* — G7 evaluates the declared sets over the refs fetched at that moment — and *binding at landing*, where step 3 fetches the refs fresh after fixing `B` and evaluates the landing intent's integrated diff against every other in-flight intent's forbidden and frozen sets. A lease pushed after that fetch is caught by the ground-moved wire on the other intent's side, not by the CAS, which guards only trunk and the landing branch.

- **Soft lease** — `expected ∩ expected ≠ ∅`: both proceed; both are told at sign-off and at every `spine check`; whoever lands second is base-moved and re-verifies; a textual conflict stops in `merge-tree`, never in a working tree.
- **Hard lease** — the diff touches another in-flight intent's forbidden set, or a path *frozen* by its approval (shared fixtures and runner config are hard leases): refused at sign-off if seen (the lower intent id holds the lease — ids are allocated by ref creation on the remote); at landing it is a wire reviewed as `class=protected` — reviewer ≠ signer in team mode — because it changes ground another intent's guarantee rests on. Ways out: wait for the earlier intent to land or withdraw; the earlier owner narrows its set via reopen; or `spine new --sign <id> --override-lease "<reason>"`, recorded as `lease_override=` on the sign-off line — the lease still trips at landing.
- **Quick lane** — a diff intersecting any in-flight lease is escalated to the gated lane.

Intent ids: `spine new` takes max+1 over live `refs/heads/intent/*`, `refs/remotes/*/intent/*` and every `Spine-Intent` id sealed on trunk — landings and tombstones — refuses an id already in the ledger, and renumbers if its push loses. Because landing deletes the ref, a stale clone can recreate `intent/INT-042` after it landed: `spine new` fetches `refs/heads/<trunk>` and `refs/heads/intent/*` immediately before allocating and refuses to allocate without that round-trip; `spine check --pre-receive` refuses creating `refs/heads/intent/<id>` for an id already sealed on trunk; without the hook, `--land` step 2 refuses a duplicate id before anything is sealed, and its exit is `--withdraw` — G9 accepts a tombstone for an id that already has a landing, because a tombstone is not a landing. `spine new` branches only from trunk — a stacked intent would misattribute members after the first lands.

**Ground moved.** On every `spine check`, for each in-flight intent: paths changed on trunk since its sign-off (or its approval base, once approved) ∩ touchpoints → reported; ∩ forbidden → wire, and a human decides reopen-or-proceed. Frozen tests meet moved ground the honest way: if re-verification shows the intent itself is now wrong, reopen, re-sign, regenerate (§4.3).

**Provider queues are supplements.** Two configurations are supported. **(a)** The trusted stage pushes the landing commit itself, with the pipeline principal on the branch-protection bypass list for required checks (the non-fast-forward rule has no bypass list) — the full guarantee; GitHub merge queue and GitLab merge trains (not merged-results pipelines, which check but do not serialize) may be the runner here and are the answer to starvation; the queue's required check is the status the trusted `workflow_run` job posts on the merge-group head. **(b)** Serial only: squash strategy, `spine check --land --print` run in the trusted `workflow_run` job (rule 0 — a PR-triggered job holds no key and prints nothing sealed) and posted as the required check on the PR against trunk's current tip, the printed envelope posted as the PR body with `gh pr edit --body-file` (never the web editor — CRLF hashes wrong), no queue; the required check re-reads the body through the provider API and fails before the merge if `git hash-object` over its fenced block ≠ `blob=`, the seal's `head=` ≠ the PR head's content head `Hc`, or its `base=` ≠ trunk's tip. `--print` emits a sealed envelope only for a run that would have landed; `--dry-run` never signs. What a provider can lose: the pipeline signature (its own key signs instead) and, in squash mode, `H`. What it cannot do: weaken the guarantee silently — anything else (a queue that reorders bases, a merge commit with a generic message, an edited body) lands `unattested` and fails G9 loudly; repair is a reseal (§5.5), and a rising reseal count is the signal to move to (a). **(b) is not an auto-merge configuration.** Spine performs no CAS there, so G10 proves the `L` that `--print` built and not the commit the provider ultimately creates, and the body re-read is a check before a merge, never an atomic guard on the tip. Precondition 4 of §7.4 rule 5 therefore fails on every landing in (b): each one takes a review, and the object that actually reached trunk is verified by the next run's G9 and G10 — loudly `unattested` when it differs. (b) is the on-ramp for teams whose provider owns the merge button; (a) is the configuration the guarantees are written for.

**Solo developers** run the same protocol: one key holds all three roles (§7.2), `--land` runs locally, TTY-gated like every other signing act (§7.1), and the CAS targets `origin/<trunk>` whenever a remote exists.

### 5.5 Landing: the intent envelope ⚙

The disposal rule deletes the file. It must not delete the truth. The record is a git object: the **landing commit**.

**Definition.** Every change reaches trunk through exactly one landing commit `L` per intent (or per quick-lane change, withdrawal, or reseal), whose message is the **envelope**:

```
INT-042: Invoice totals include tax

-----BEGIN SPINE-INTENT blob=9f2c… bytes=1472-----
# INT-042: Invoice totals include tax
Owner: @alice · Template: v2 · Ticket: … · Constitution: v3
…the intent doc, byte-for-byte as signed…
-----END SPINE-INTENT-----

Spine-Envelope: 1
Spine-Event: land
Spine-Lane: gated
Spine-Intent: INT-042
Spine-Signoff: INT-042 blob=9f2c… template=v2 constitution=v3 reopens=1 signer=alice@example.com
Spine-Signoff-Sig: …                       ← human sign-off, copied verbatim
Spine-Reopen: INT-042 voids=sha256:… reopens=1 reason="…" signer=alice@example.com
Spine-Reopen-Sig: …                        ← every reopen, copied verbatim
Spine-Approve: INT-042 intent=9f2c… base=7b0d… rounds=1 total_rounds=3 reopens=1 red=5/5 freeze=sha256:… signer=alice@example.com
Spine-Approve-Sig: …                       ← approval, copied verbatim
Spine-Approval: 5c9e…                      ← the approval commit (reachable via L^2 under merge strategy)
Spine-Frozen: a41b… tests/billing/test_invoice.py     ← squash strategy only: the frozen manifest, copied
Spine-Test: tests/billing/test_invoice.py::test_AC1_totals_include_tax
Spine-Review: INT-042 class=tripwire head=77aa… tree=… base=7b0d… intent=9f2c… report=sha256:… wires=G2:src/shared/util.ts reason="…" reviewer=bob@example.com
Spine-Review-Sig: …                        ← only when a wire tripped
Spine-Gates: G1=pass G2=override G3=pass … G16=pass
Spine-Strategy: merge
Spine-Supersedes: INT-017                  ← optional, from the intent's header
Spine-Seal: INT-042 base=7b0d… head=77aa… tree=… report=sha256:… tool=1.4.0+sha256:… git=2.45 mode=team envelope=sha256:… signer=ci@example.com
Spine-Seal-Sig: …                          ← pipeline signature; the last Spine-* line
```

Rules, each tied to a failure:

- **Frozen bytes.** The fence names the blob and the byte count; the parser reads exactly `n` bytes, and `git hash-object` over them reproduces `blob=`. The human's signature is over the sign-off line that names that blob. Edit the file after sign-off and the blob no longer matches. Edit the commit message after landing and the seal's `envelope=` digest — SHA-256 over every `Spine-*` line above it, in order — no longer matches. Foreign trailers a provider appends after the seal are outside the digest and ignored. Nothing is trusted because it says so; everything is trusted because it hashes.
- **Signatures.** All three are SSH (`ssh-keygen -Y`), verified against `.spine/allowed_signers` **as it existed at the seal's `base=`** — a landing can never admit its own signer. Because the seal's payload never contains the commit SHA, a provider-made landing commit that copies the lines verbatim still verifies. Signed commits on the branch are welcome but are not the record; under squash they are unreachable and garbage-collected within weeks.
- **Two shapes, one switch.** `C-M1: merge.strategy = merge | squash`. *merge*: `L` is the `--no-ff` merge commit with parents `(B, H)` — the default, because `H` stays reachable and the gate report is recomputable offline. *squash*: `L` has parent `(B)`; the frozen manifest lines are copied into the envelope because the approval commit becomes unreachable, and the audit degrades to the pipeline seal plus the freeze audit of G9. *Rebase* landings are refused for both lanes: they land intermediate trees no gate saw and drop the empty event commits. In every shape `L` deletes `intents/INT-042.md`, `L`'s tree equals `merge-tree(B, H)` minus that file, and `B` is `L`'s first parent. The fenced intent plus the signed lines are capped at 16 KiB, projected at `--approve` and checked at `--land`; exceeding it is a refusal (`envelope-too-large`), never a truncation — split the intent or use merge strategy. `Spine-Frozen`/`Spine-Test` lines are outside the cap; their size is the closure's, bounded by the 200-file tripwire.
- **Written and read as bytes.** `L` is created with `git commit-tree`; the indexer reads messages with `git cat-file commit`, never `git log`, so no cleanup rule ever touches the fenced bytes.
- **Every trunk commit is sealed.** A quick-lane change lands with a minimal envelope (subject `quick: <summary>`, `Spine-Envelope`, `Spine-Event: land`, `Spine-Lane: quick`, gates, strategy, seal — plus, on a toolkit lifecycle landing, the copied `Spine-Upgrade` + `-Sig` and its protected `Spine-Review` + `-Sig` (§6.7) — no fenced block, no sign-off, no approval). A withdrawal (`spine new --withdraw`, then `--land`) lands a **tombstone**: parent `B`, tree identical to `B`'s, `Spine-Event: withdraw`, the fenced intent whose bytes hash to the `Spine-Withdraw` line's `blob=` — and to the sign-off's when one exists — the sign-off if one existed *and its key is in the keyring at `base=`* (otherwise omitted, and the withdraw line names it `orphaned=<principal>`), the signed `Spine-Withdraw` line, the seal — so abandonment is countable from trunk alone, and a tombstone retires the id. A first-parent trunk commit that is neither a landing nor the trust root (§7.5) is an **orphan**: a push around the pipeline. G9 refuses to land on top of one until it is resealed: a **reseal** is a quick-lane landing with `Spine-Event: reseal`, parent = the orphan tip `O`, tree identical to `O`'s, seal `base=` the last valid landing below the range and `head=O`; every wire and the floor are evaluated over `diff(base, O)` and folded into the protected review's `wires=` — a reseal is never escalated and never refused by a wire; a G1 or G8 finding inside the range is sealed `=override` and counted as a freeze override, because the code is already on trunk and the only honest act is to say so. Every policy read for a reseal — keyring, manifest, pin, workflow, mode, the chain rule, §7.4 rules 0–2 — is from `base=`, never from `O`: a range in which any `.spine/**`, CI-definition, constitution or agent-context path at `O` differs from its blob at `base=` is refused until a further hand commit restores those blobs, so a reseal never seals a policy change. Its reviews are event commits on `refs/heads/quick/reseal-<O>`, branched from `O` (so `Hc = O`), landed by `--land --reseal`, which the CAS deletes. It always takes a protected review — two distinct `class=protected` reviews in team mode, since there is no signer (§7.2). Resealed commits index as `unattested` members of the reseal changeset, counted forever by `spine stats`; a reseal cannot cover a *valid* reseal's range — an `unattested` reseal is an orphan like any other and the next reseal covers it as one; and a frozen path changed inside a resealed range never satisfies G8's harness-moved clause — it is a G8 failure for every intent that froze it. Solo developers meet the reseal the first time they `git commit` on trunk; that is the point.

**How the indexer reads it.** A `Spine-Seal` trailer marks a landing `L`; its members are `M(L) = git rev-list B..L`, `B` being the seal's `base=` (for a reseal, the resealed range) — merge strategy: `L` plus every branch commit not already on trunk (merges *from* trunk into the branch are excluded automatically); squash: `{L}`. Every `c ∈ M(L)` gets `cs:<c> → implements → INT-042` with provenance `git:<L>:trailer:Spine-Intent`; membership comes from the landing range, never from a trailer on a branch commit, because a branch commit can claim anything. The integrated delta `git diff --name-only B L` is what G2 gates on. Everything else the envelope yields — nodes, edges, provenance, the structural checks — is the derivation table of §6.2 and the G9 row of §6.3, stated once there.

---

## 6. Graph engineering ⚙ — the derived layer

This workflow already contains a latent graph. The ID discipline — `INT-042 → AC-1…6 → test_AC1_* → touchpoints → constitution v3 → ADR-007` — is nodes and edges. Spine-kit's job is not to ask anyone to draw a graph; it is to **extract the graph that already exists** in the artifacts. This section defines that extraction.

"Graph engineering" in current practice means three different graphs, and conflating them is the fastest way to bloat the toolkit. Spine-kit ships **two graphs and one table** — the third "graph" was deliberately demoted in v0.5:

| Graph | Authored by | Role in spine-kit |
|---|---|---|
| Traceability graph | Nobody — derived from IDs and git objects by `spine index` | Drift gates, coverage gates, archaeology, session resume |
| Code graph | Nobody — derived from AST/dependencies (tree-sitter) | Touchpoint proposal in the interview; graph-containment tripwires; scoped context for Agents A and B; the quick-lane router |

**The workflow "graph" is a transition table, not an engine.** What graph engineering actually teaches is that transitions must be explicit, reviewable, and permitted-only — it does not require an orchestration framework. Spine-kit encodes the pipeline as declarative rows (state × event → next state, plus the guard that enforces each), checked by the same code that runs the gates. States marked † are runner-local: they exist only while a gate record is live, and collapse to `tests-approved` in any fresh clone, which is correct — a gate record that is not being consumed right now is worth nothing (§5.4). Every human-caused transition after `draft` is a signed git object, so the table is checkable offline.

| State | Event | Next state | Enforced by |
|---|---|---|---|
| draft | interview complete; Open questions empty | awaiting-sign-off | `spine new` |
| awaiting-sign-off | `spine new --sign`: `Spine-Signoff` + `-Sig` verify under `spine-signoff@v1`; `blob=` equals the intent blob at head; `reopens=` equals the branch's reopen count | signed | G13 |
| awaiting-sign-off | G7 hard collision with a lower-id in-flight lease (as fetched) | awaiting-sign-off — refused: wait, coordinate, or `--override-lease` | G7, advisory at sign-off |
| signed · tests-drafted · tests-approved | intent blob changes without a signed `Spine-Reopen` | awaiting-sign-off **and** Integrity fails | G8 + G9 |
| any in-flight | `spine new --reopen` (signed `Spine-Reopen`, voids the freeze digest it names; must change the blob) | awaiting-sign-off; `reopens`+1; A↔B budget resets | G13 verifies it; G9 records it |
| any in-flight | signer's key removed from trunk's keyring | awaiting-sign-off | G13, in-flight clock (§7.5) |
| tests-approved · landing-review · protected-review | approver's or reviewer's key removed from trunk's keyring | tests-drafted (approval void) / the review state with the review void | G13 |
| tests-approved (reported on every `spine check` for earlier states; `--land` is refused there per the `refused — no binding approval` row) | G4: `built_under` a constitution bump flagged `resign`, or `Template:` below the manifest's `resign` floor | landing-review, wire `G4` — proceed by tripwire review, or a human reopens | G4 (§6.7) |
| signed | A's tests written | tests-drafted | `spine index` refuses `verified_by` edges to unsigned intents |
| tests-drafted | B fails to break within 2 rounds; `spine check --approve` | tests-approved | approve guards: every AC covered by a collected id, G5 clean, closure resolves, G12 red |
| tests-drafted | B still breaking after 2 rounds · G12 green · closure tripwire | approval-review | loop cap; `--approve` refuses without a human reason |
| approval-review | human signs the approval with a reason | tests-approved | G13 |
| signed · tests-drafted · approval-review | `spine check --land` with no `Spine-Withdraw` on the branch | refused — no binding approval | landing step 2 |
| tests-approved | a harness-pattern path's blob in `T` differs from both the approval tree and trunk · frozen test id not passed · intent blob ≠ signed blob | *(blocked)* | G8 + G1 — never warn mode; exits are reopen or a counted freeze override |
| tests-approved | frozen path's blob in `T` equals trunk's tip but not the approved blob | landing-review, wire `G8:<path>` — frozen ids rerun **and** a human confirms, unless every mover carried a protected review with wire `G7:<path>` or `G8:<path>` | G8 harness-moved clause |
| tests-approved | `merge-tree(B, H)` conflicts | needs-rebase | `git merge-tree` |
| needs-rebase | trunk merged into the branch; frozen blobs intact | tests-approved | G8 |
| tests-approved | all gates + suite clean on the synthetic merge `T` | checked † | `spine check --land` |
| tests-approved | wire tripped on `T` (Drift / Freshness / Strength, or `C-M4` off) | landing-review | tripwires (§5.2) |
| tests-approved | floor hit on `merge-base..head` — regardless of touchpoints — or G7 hard lease on `T` | protected-review | G14 · G7 |
| landing-review | `Spine-Review class=tripwire` verifies with `head=Hc` over the current tree (self allowed) and the report's wire set ⊆ its `wires=` | checked † | G13 |
| protected-review | `Spine-Review class=protected` verifies with `head=Hc`; reviewer ≠ signer in team mode; two reviewers when the landing has no signer; the report's wire set — floor hits as `G14:<path>` — ⊆ the union of their `wires=` | checked † | G13 + G14 |
| landing-review · protected-review | `H ≠ review.head`, or `merge-tree(review.base, H) ≠ review.tree` — the branch changed | same state, review void | reviews bind head and tree |
| landing-review · protected-review | base moved; `H == review.head`; new merge conflict-free; new wire set ⊆ wires signed; `D ∩ (floor ∪ C-A2 ∪ paths in the signed wires) = ∅` | same state, review retained | G11 recomputes the report |
| checked † | CAS wins: trunk == base, branch == head | merged | `push --atomic --force-with-lease` / `update-ref` |
| checked † | CAS lost, or any landing on trunk | base-moved † | G11 |
| checked † | G9 over `L`, or G10 under `C-M5: inline`, fails on the built `L` | reconstruction-failed — discarded, reported, never re-queued: a deterministic failure re-runs identically | G10 |
| base-moved † | runs < `C-M3` | tests-approved — re-verify from step 1 on the new tip | G11 |
| base-moved † | moved ground ∩ forbidden ≠ ∅ | landing-review | G7 |
| base-moved † | runs exhausted | starved — reported; re-queued | G11 |
| tests-approved · landing-review · protected-review · base-moved † | `spine check --break-glass`: `Spine-Review class=break-glass` (≠ signer in team mode) | checked † — G1, G2, G3, G4, G6, G7, G8, G12 bypassed and marked `=override` | G13 — never before approval, never G5, never Authority, never G9–G11 |
| any in-flight | `spine new --withdraw` (signed) then `--land` tombstone | withdrawn | G9 accepts tombstones (tree == parent's) |
| quick-candidate (`refs/heads/quick/*`; toolkit lifecycle branches `refs/heads/spine/upgrade-*` enter here too, §6.7) | clean on `T` | checked † | `spine check --land --quick <branch>`; `--land` with no id for a lifecycle landing |
| quick-candidate | Drift or Strength wire (size and dependency included), a harness path, a pragma added, or a lease on `refs/heads/quick/*` — on a lifecycle branch a lease is a `class=protected` wire in lane, never an escalation (§6.7) | escalated — needs an intent (`spine new --from <branch>`) | router (§3.5) |
| quick-candidate | floor hit | protected-review, in lane | G14 |
| quick-candidate | `C-M4` off | landing-review, in lane — `Spine-Review class=tripwire` with `head=Hc` → checked † | G11 + G13 |
| merged | envelope fails G9 on re-index (edited message, unknown key, base ≠ first parent, tree ≠ merge-tree, missing sign-off or approval) | unattested — reported and counted forever | G9 |
| orphan · unattested | reseal landing over the range under a protected review (two reviewers in team mode) | resealed — members stay `unattested`, counted | G9 |
| merged | full revert of the landing on trunk (patch-id over `L`'s paths) | reverted | derived (§6.6) |
| reverted | that revert itself fully reverted | merged | derived (§6.6) |
| merged | a later landing carries `Spine-Supersedes` naming this intent | superseded | derived from the envelope |
| keyring vN | `.spine/allowed_signers` change lands with a parent-set seal and a parent-set protected review — or, with no pipeline key, as a recovery landing (§7.5) | keyring vN+1 | chain rule (§7.5) |
| keyring vN | change lacking the above, or trunk history rewritten below the trust root | chain broken — Authority hard-fails every check | chain rule (§7.5) |

Anything not in the table cannot happen — implementation before sign-off simply has no row; "edit a frozen test" is not a transition but a G8 failure; a merge whose base is not trunk's tip has no row; a merge without a seal has no row. If full orchestration of Agents A and B someday genuinely needs retries, budgets, and resumable runs, an engine can be adopted *behind* this same table; not before. **User-defined custom workflow DAGs are refused**: a user-authored workflow is an authored graph, and the iron rule below applies to workflows too.

### 6.1 The iron rule: derived, never authored

The moment a user has to create or maintain a graph, you have rebuilt SDD bureaucracy in graph clothing, with a worse editor. Every graph in spine-kit is a **cache**: gitignored, deleted at will, deterministically rebuilt from the repo by one command. A derived graph can never go stale, which is the same property that justified deleting intent docs at landing — permanence without staleness, now for structure instead of prose.

Corollary — **the provenance law**: every node and edge must cite its source. An edge that cannot say where it came from does not exist. The provenance grammar is fixed: `<path>:<line>` · `git:<sha>` · `git:<sha>:msg:L<n>` (a line inside a commit message) · `git:<sha>:trailer:<Name>` · `git:<sha>:patch-id` · `git:<sha>:<path>:<line>` (a line of a file at a commit) · `spine:<version>:floor` (the protected floor shipped in the release). This is what makes the graph auditable, regenerable, and honest — and what G10 proves on a clean clone before every landing is pushed.

The envelope (§5.5), the approval record (§4.3) and the install manifest (§6.7) are *sources*, not graphs: written by a tool, never by hand, and each the analogue of a commit to a diff. The graph stays fully derived from them.

### 6.2 Traceability graph schema

Designed backwards from the questions it must answer mechanically: Is every AC verified? Did the diff stay in bounds? Which intents ever touched this module? What is the resumable context for an in-flight intent? Which intent does a failing test trace to? Which in-flight work was built under an outdated constitution? *Who approved exactly this, against which base, and does it still hash?* Nothing that fails to serve one of those questions is in the schema.

```sql
PRAGMA user_version = 7;   -- graph schema; ≠ the binary's constant → delete and rebuild (§6.7)
CREATE TABLE meta (key TEXT PRIMARY KEY, val TEXT);
  -- 'cli_dist_hash' | 'manifest_blob' | 'built_at_trunk'
  -- a cache built by another binary, another manifest, or an older trunk tip is never queried — it is rebuilt

CREATE TABLE nodes (
  id   TEXT PRIMARY KEY,  -- "myrepo/INT-042" | "myrepo/INT-042/AC-1" |
                          -- "test:billing/test_inv.py::test_AC1_totals" |
                          -- "code:src/billing/" | "cs:abc123f" | "approval:5c9e…" |
                          -- "signer:alice@example.com" | "ADR-007" | "constitution:v3"
  kind TEXT NOT NULL,     -- intent | ac | test | code_unit | changeset |
                          -- approval | signer | adr | constitution
  attrs JSON,             -- intent:    {status (derived), owner, title, template, blob,
                          --             signer, reopen_count, late_reopen_count, landing, base}
                          -- changeset: {landing: bool, lane, event, strategy, base, head, tree,
                          --             seal_principal, seal_verified, report_sha256, recon,
                          --             tool_version, git_version, mode, unattested, resealed}
                          -- approval:  {event: signoff|approve|review|reopen|withdraw|upgrade,
                          --             role, principal, verified, blob, base, head, tree, class,
                          --             rounds, total_rounds, reopens, red, freeze, wires,
                          --             voided_by, void_reason}
                          -- signer:    {roles[], fingerprint, valid_from, valid_to}
                          -- test:      {result_at: {tree, base, passed}}  -- volatile; excluded from G10
  src  TEXT NOT NULL      -- provenance, per the grammar of §6.1
);

CREATE TABLE edges (
  from_id TEXT NOT NULL,
  to_id   TEXT NOT NULL,
  kind    TEXT NOT NULL,  -- has_ac | verified_by | declares | implements | modifies | built_under |
                          -- approves | freezes | signed_by | attested_by | reverts |
                          -- supersedes | superseded_by | protects | exercises
  attrs   JSON,           -- declares:    {"polarity":"expected"|"forbidden"}
                          -- implements:  {"role":"landing"|"member","provisional":bool,"verified":bool}
                          -- verified_by: {"attributed":bool,"introduced_by":"cs:…"}
                          -- freezes→code_unit: {"oid":"a41b…"}   freezes→test: {}
                          -- reverts:     {"partial":bool}        protects: {"floor":bool}
  src     TEXT NOT NULL
);
```

Storage is SQLite in a single gitignored file (`.spine/cache/graph.sqlite`). No graph database in v1; the day SQLite genuinely cannot answer a needed query is the day to revisit — not before. **IDs are repo-scoped from day one** (`myrepo/INT-042`, not bare `INT-042` — the prefix comes from the manifest's `repo`, while trailers carry the bare id so a fork or rename does not invalidate history): it costs one line in the ID scheme now, and it makes multi-repo federation a namespace merge later instead of a rewrite of every graph, pragma, and envelope. When the multi-repo day comes, federate SQLite files — do not reach for a distributed graph database. Indexing is incremental: the cache is keyed by trunk tip, and only commits above the last verified landing are re-walked.

**Derivation sources** (this table is the indexer's spec):

| Graph element | Derived from |
|---|---|
| `intent`, `ac` nodes; `has_ac`, `declares` (polarity), `built_under`; intent `template` | in flight: `intents/<ID>.md` on `refs/heads/intent/*`; historical: the fenced intent bytes in the landing commit's envelope (§5.5), parsed by the `Template:` version's parser. Never a PR description |
| `approval` nodes; `approves`, `signed_by` | `Spine-Signoff`, `Spine-Approve`, `Spine-Review`, `Spine-Reopen`, `Spine-Withdraw`, `Spine-Upgrade` lines with verifying `-Sig` — on event commits while in flight, copied into the envelope once landed; `approves` names the intent for every line carrying an id and the landing changeset `cs:<L>` for those that do not (`Spine-Upgrade`, and any review of a quick, reseal or lifecycle landing) — emitted only once the landing is indexed; in flight an id-less line's approval node carries no `approves` edge, there being no `L` yet — verified against `.spine/allowed_signers` at trunk's tip (in flight) or at the seal's `base=` (landed). An event commit whose signature fails, or whose role disagrees with its namespace, is excluded from state derivation and raised as a G13 wire naming the sha — a branch stays append-only, and a bogus commit cannot brick it |
| `freezes` | `Spine-Frozen` (→ `code_unit`, with the blob) and `Spine-Test` (→ `test`) lines of the binding approval (§4.3) |
| `signer` nodes | `.spine/allowed_signers` at every trunk first-parent commit from the trust root, with `valid_from`/`valid_to` from the chain walk (§7.5) |
| `changeset` (landing + members); `implements`; `attested_by` | `M(L) = git rev-list B..L` for every trunk commit carrying `Spine-Seal`; the seal's fields become the landing changeset's attrs; in flight: `merge-base..branch`, provisional |
| `modifies` | `git diff --name-only B L` — the integrated delta G2 gates on; per-member diffs for archaeology |
| `test` nodes; `verified_by` (`attributed`) | pragmas `# @verifies INT-042/AC-1` (canonical) or `test_AC1_*` names (sugar) **in blobs some approval froze**: in flight, the branch's test files — `attributed` iff the line is in a blob frozen by the binding approval, or (before approval) the file is on the intent's own branch and under `C-T1`; landed, parsed from `<L>:<path>` for every `Spine-Test` path of the landing — the frozen blob, reachable through `L`'s tree forever, provenance `git:<L>:<path>:<line>`, attributed by construction and never lost to later edits or deletions. `git blame` yields `introduced_by` for archaeology, never a gate input |
| intent `status` | derived, never read from the file: the transition table, evaluated over event commits and landings |
| `reverts`; status `reverted` | a landing `R` later than `L` on first-parent, with a non-empty diff, whose `git diff R^ R -- <L's paths> \| git patch-id --stable` equals `git diff L L^ \| git patch-id --stable` — restricted to `L`'s paths, so the `BUG-` reproduction test `R` also lands does not disqualify it; missing hunks inside `L`'s paths → `{partial: true}` and a warning; only `Spine-Event: land` commits participate; `Spine-Reverts:` and git's "This reverts commit" line are hints |
| `supersedes` / `superseded_by` | ADR and constitution headers; the `Spine-Supersedes` trailer, copied from the intent's `Supersedes:` header |
| `protects` | the floor list inside the pinned release (`spine:<version>:floor`) + constitution `C-A2` |
| changeset `event`, `Spine-Upgrade` attrs | landing envelopes carrying `Spine-Event: land \| withdraw \| reseal` and, for toolkit lifecycle landings, the copied `Spine-Upgrade` line (§6.7) — so "which toolkit version was this intent built and landed under" is an archaeology query |
| `exercises` (optional, v1.1) | CI coverage reports |

Two schema positions, defended:

- **The pragma is canonical; the naming convention is sugar.** `test_AC1_totals` is a friendly default, but the comment pragma survives test renames, works identically across languages, and makes `verified_by` greppable without parsing any test framework. A pragma counts only when a runner collected an id from its file — an AC "covered" by a pragma in a file no runner executes is not covered.
- **Non-goals are not nodes.** They are prose constraints with no mechanically derivable edges — "violated a non-goal" cannot be auto-detected. By this playbook's own governing rule, what cannot be machine-checked stays in the doc for humans and Agent B. (Same reason there are no function-level nodes: that is the code graph's job; the two graphs join on `code_unit` paths rather than merging into one mega-graph.)

### 6.3 Gates as queries

With this schema, every gate becomes a query. The drift gate is literally set containment:

```sql
-- G2: files modified by the landing that fall outside declared touchpoints
-- (paths frozen by this intent's binding approval are excluded: they are G8's)
SELECT m.to_id FROM edges m
JOIN edges i ON i.from_id = m.from_id AND i.kind = 'implements'
             AND json_extract(i.attrs,'$.role') = 'landing'
WHERE m.kind = 'modifies' AND i.to_id = 'myrepo/INT-042'
  AND m.to_id NOT IN (SELECT to_id FROM edges
    WHERE from_id = 'myrepo/INT-042' AND kind = 'declares'
    AND json_extract(attrs,'$.polarity') = 'expected');
-- any row → tripwire fires
```

The full gate suite — **five families** are the public vocabulary (§10 argues the fifth: the four existing families judge *what changed*; none judged *who may cause a landing*, and hiding that under Integrity would bury a security boundary in a quality label); G-numbers are internal check IDs. *Warn* marks the gates that participate in warn-before-block calibration (§9); every other gate blocks from day one.

| Family | Check | Query, in words | Warn |
|---|---|---|---|
| Integrity | G1 — Coverage | Every AC of a `tests-approved`+ intent has ≥1 `verified_by` edge with a collected id, **and every frozen test id, and every id trunk collected on `B`, is reported *passed* in a result file trunk's collector wrote and labelled with `T` (§7.4)** — not skipped, xfail, deselected, or absent. A pass against any other tree is not a pass. A landed intent's tests are the floor for every later landing. G1 also reports the *strength* of its own evidence — the header's `profile=`, sealed as `profile=` — and raises a wire naming any failed auto-merge precondition (§7.4 rule 5) | no |
| Integrity | G5 — Orphans | A `verified_by` edge to a nonexistent AC (typo'd pragma) fails loudly; so does a pragma outside every frozen blob, or one that first appears after its intent's approval — `attributed: false` | no |
| Integrity | G8 — Freeze | For each frozen `(blob, path)`: `T`'s blob equals it, or equals trunk's (harness moved → rerun + landing-review); any `C-T1`/`C-T2`/runner-config path, or one frozen by an approval trunk's own landings carry, whose blob in `T` differs from both the approval tree and trunk fails; one present at `B` whose blob in `T` differs from `B`'s — edited, deleted or renamed by the branch before approval — is a `class=protected` wire `G8:<path>`, and a landed id `T` no longer collects or does not pass fails unless that review names its path: harness changes never auto-merge; intent blob equals the signed blob; in `--ci` the closure recomputed by the pinned release ⊆ `Spine-Frozen` (§4.3) | no |
| Integrity | G9 — Ledger | First-parent walk of trunk: every commit is the root or a valid landing — envelope parses; fenced bytes hash to `blob=`; the seal's `base=` is the first parent — for a reseal, the last valid landing below its range, with `head=` its first parent and `base..head` its members; every `-Sig` verifies against the keyring at `base=`; a gated `Spine-Event: land` envelope carries a verifying `Spine-Signoff` for its blob and a verifying `Spine-Approve` naming that blob whose `freeze=` no copied `Spine-Reopen` voids (each copied `Spine-Reopen` commit an ancestor of `H` under merge), with `Spine-Approval ∈ M(L)` under merge and the SHA-256 over that commit's sorted `Spine-Frozen`/`Spine-Test` lines equal to the copied approve line's `freeze=` (a `withdraw` envelope instead carries `Spine-Intent`, a verifying `Spine-Withdraw` whose `blob=` the fenced bytes hash to, the sign-off iff one is copied, and no approval); every `Spine-Gates` entry is `pass` or `override`, each `override` named in the `wires=` of a copied review whose class admits it (tripwire/protected: that wire; break-glass: the §7.6 list), and no entry is G10; under `recon=scheduled:<n>` the seal's `proved=` names a first-parent landing with at most `<n>` landings above it, and a landing whose reconstruction failed is never named by a later `proved=`; a copied `Spine-Review` carries `head=` equal to the seal's — `L^2` or an ancestor of it reached only through empty review commits — and, under merge, `merge-tree(review.base, L^2)` equals its `tree=`; a landing whose `diff B L` hits the floor carries a protected review (or the two-reviewer / recovery form) by a key eligible at `base=`; exactly one `Spine-Event: land` per intent **id** — a reverted id is retired like a tombstone's, and the fix lands as a new intent naming it in `Supersedes:`; a tombstone is not a landing but retires the id; tombstones and reseals have their parent's tree. Tree rule: under `merge`, `L`'s tree equals `merge-tree(B, H)` minus the intent file (if the indexer's git differs from the seal's `git=` and the trees differ, record `tree: unverifiable(git-version)` — reported, not `unattested`); under `squash`, `H` is by design unreachable — the tree rule is never consulted even when `H` happens to be reachable, so a source-side index and the G10 clone derive the same thing — G9 records `tree: unverifiable(squash)` and instead audits the freeze from the envelope: every copied `Spine-Frozen` blob equals `L`'s or `B`'s blob at that path — or `Spine-Gates` records `G8=override` and a copied `class=break-glass` review with the seal's `head=` names `G8` in its `wires=` — no `intents/` path remains, and `freeze=` recomputes from the copied lines. In-flight event commits and blob edits without a signed reopen are checked the same way. A failing landing indexes `unattested` — reported and counted; a trunk tip that is not a valid landing blocks `--land` until resealed (§5.5) — `--land --reseal`, whose `head=` is that tip, is the one form it admits. In `--ci` the walk checks each landing in full down to and including the nearest one whose seal verifies, whose `envelope=` and fenced `blob=` recompute, and whose `Spine-Gates` records `G9=pass`, then stops; the full walk to the root is `--reconstruct`'s and `--authority`'s job. Pre-adoption history below the root, and the range between an uninstall and the next init (§6.7), are exempt | no |
| Integrity | G10 — Reconstruction | Runs **before** the CAS, on the candidate landing `L` built at step 4 of §5.4, and never by moving the runner's own refs: `L` is pushed into a scratch clone `S` as `refs/heads/<trunk>` with the intent ref deleted, so `S` holds the post-CAS ref set both sides index; then `git clone --no-local --no-hardlinks file://S` into a temp dir with `GIT_CONFIG_GLOBAL=/dev/null` and no network (default refs only — no notes, no custom refs, no provider metadata); the runner's pinned trust root is copied into the clone's `spine.trustRoot` (TOFU is for humans, never for G10); `spine index` there; canonical `--dump` on both sides — nodes sorted by kind,id, edges by from,to,kind, `src` included; provisional (in-flight) elements, † states (dumped as `tests-approved`), volatile test results and worktree-only files excluded; the diff must be empty. G10 proves the ledger, not the lease registry. A failure **refuses the push**, ends the run as `reconstruction-failed` without a retry. The discarded `L` never becomes a git object, so the run's own report is the only record — one more reason the failure is terminal rather than quiet. It is still an indexer defect to file against spine, not a ledger defect — the envelope G9 accepted is valid — but a landing a clean clone cannot reproduce does not reach trunk. `C-M5` governs when it runs. `inline` — the default — is before every push, and is precondition 3 of auto-merge (§7.4 rule 5). `scheduled:<n>` is the degraded mode for repos too large for that, and `<n>` counts **landings, never days** — the chain is this design's only clock (§7.5). It forces `C-M4` off, and works by amortisation rather than by a separate job: every landing run proves the oldest landing not yet proved, and seals `recon=scheduled:<n> proved=<sha>` naming the newest one that has been. The backlog is therefore self-limiting — each landing adds its own and retires an older one — and G9 refuses any landing with more than `<n>` unproved first-parent landings above `proved=`. A proof that fails reports `reconstruction-failed` against the landing it names, and no later `proved=` may advance past it: the backlog then grows until it hits `<n>` and the repo stops landing, which is the point. Spine-kit's own release suite is always `inline` | no |
| Drift | G2 — Containment | `modifies` of the synthetic merge ⊆ declared `expected` touchpoints; any `forbidden` hit is a hard fail; paths frozen by this intent's approval are G8's, not G2's; quick lane: ⊆ `C-Q1` ∪ floor ∪ spine-owned paths, and under `C-Q2` lines — a floor path is G14's, reviewed in lane, never escalated; the diff-size, new-dependency and schema/auth/public-API wires of §5.2 are G2 sub-checks, recorded as `G2:<path>` | yes |
| Drift | G7 — Interference | At sign-off (advisory, declared sets over the refs fetched) and at landing (binding, the integrated diff over a fresh fetch of `refs/heads/intent/*`): `expected ∩ expected` → soft, surfaced to both owners; the diff ∩ another intent's forbidden or frozen set → hard, a `class=protected` wire at landing. On every `spine check`: ground moved on trunk since sign-off/approval ∩ touchpoints → reported; ∩ forbidden → wire. Quick lane: diff ∩ any lease → escalate (§5.4) | yes |
| Freshness | G3 — Staleness | An in-flight intent older than ~14 days (committer dates — forgeable, acceptable for a warning) is flagged — anti-staleness *inside* the working window, not just after landing | yes |
| Freshness | G4 — Currency | An in-flight intent `built_under` a constitution bump flagged `resign`, or stamped with a template version below the manifest's `resign` floor (§6.7), trips a wire: `landing-review` with `G4` — proceed by tripwire review, or a human reopens. The pipeline never writes to a branch | no |
| Freshness | G11 — Base currency | A gate record is consumable only while trunk == `base` and the branch == `head`; the ref update is the check. A lost CAS → `base-moved`; re-verification depth per `C-M2`; runs per `C-M3`, then `starved`; `C-M4` evaluates `on` only when all four preconditions of §7.4 rule 5 hold in this run, and `off` ⇒ a landing-review wire on every landing. Sign-off survives; approval survives iff frozen blobs are byte-identical (or trunk's); a review survives per the §5.4 table; gate results never survive | no |
| Strength | G6 — Mutation (optional) | Mutate the implementation; if the AC tests stay green, they are too weak — the deterministic twin of Agent B's adversarial check, giving correlated-failure protection from two uncorrelated directions. Recommended wherever harness changes auto-merge, and the signal for a weakened oracle (§4.3) — but a *strength signal*, never an integrity control: G6 runs in the untrusted stage and reports through the same collector as every other test, so it measures weak tests, never a dishonest runner (§7.4) | n/a |
| Strength | G12 — Red at approval | `red=k/n` recorded at `--approve`, measured with the intent's `expected` paths restored to base; `k = 0` is a wire (a human signs with a reason); a `BUG-` reproduction AC that is green is refused outright | no |
| Authority | G13 — Signers | Every human signature verifies under its namespace against trunk's keyring — the *current* set for in-flight work (revocation reopens or voids), the set at the seal's `base=` for landed work (history stays valid; since-revoked signers are listed by `spine check --authority`); `C-A1` mode equals the count of distinct **keys** under `spine-signoff@v1` (a count mismatch is a warning on every report); reviewer ≠ signer compares fingerprints; a keyring listing one key under two principals, or — in team mode — a key listed under `spine-seal@v1` together with any other namespace (in either direction: the seal principal holds the seal namespace and nothing else), is refused, as is a review or approval without `run=` signed by a `spine-seal@v1` key; protected and break-glass reviews are not self-approved in team mode; a `Spine-Withdraw` under `spine-signoff@v1` is by the sign-off's key, any other under `spine-review@v1` by a reviewer ≠ that key; a landing with no signer carries two protected reviews from distinct keys; an event line byte-identical to an earlier one on the branch, or an approval whose `freeze=` a reopen voids, is refused; keyring changes obey the chain rule (§7.5) | never |
| Authority | G14 — Floor | The `merge-base..head` diff — renames, deletions, mode changes, symlinks (`120000`), submodule pointers (`160000`) included, paths casefolded — ∩ (shipped floor ∪ `C-A2`) = ∅, **or** a `Spine-Review class=protected` verifies with `head=Hc` over the current tree by an eligible reviewer. A landing whose `T` drops an entry from the manifest's `paths.*` present at `B` fails outright, review or no review — except a landing carrying `Spine-Upgrade: to=none`, which needs only the protected review. Declared touchpoints are not consulted | never |
| Authority | G15 — Tool | The running binary's platform artifact is listed in trunk's pinned `dist_hash` artifact list (older → refuse; newer → warn locally, fail in `--ci`; a landing carrying `Spine-Upgrade` is evaluated by the base's pin, never the candidate's); policy files were read from trunk, not the candidate; the graph was built `--fresh`; the seal verifies under `spine-seal@v1` (or the recovery form, §7.5) and its `tool=` equals the pin (or, on a solo or `mode=recovery` seal of a rollback, uninstall or re-init landing — a manifest restored from the `from-manifest=` ancestor under G16's restoration rule, `to=none`, or `from=none` — that line's `to=`, and for `to=none` the running binary, there being no version to equal); a manifest naming another trunk fails | never |
| Authority | G16 — Scaffold | On a rollback, the **restoration rule**: `from-manifest=<sha>` is a first-parent ancestor of `B`, every frozen field and `files[]` record in `T`'s manifest equals that ancestor's but for a monotone `paths.*` union, and every managed path in the landing's diff equals its blob at `<sha>` or the deletion that manifest prescribes — a recovery-sealed landing that fails it is `unattested` (§7.5). Otherwise: every spine-owned path's blob equals its manifest blob or the path is `user-modified`; the manifest blob changes only on a landing carrying a copied, signed `Spine-Upgrade` whose `forced=` agrees with the blobs; floor-relevant manifest fields never shrink; the keyring contains no `valid-before=`/`valid-after=`; no staging residue (§6.7); for `to=none`, only that every spine-owned path and managed region listed at `B` is absent or marker-free in `T` | never |

In warn mode a Drift finding still enters the report's wire set and `wires=` — it merely does not block on its own; a `forbidden` hit, and G7's hard lease over another intent's forbidden or frozen set, block in every mode. G5 encodes a principle worth stating: in a derived graph, **dangling edges are the linter**. Traditional traceability systems rot because broken links fail silently; under the provenance law, a broken link is a build failure with a `file:line` to fix. G9 and G10 extend it to the ledger: a landing that does not hash is a loud `unattested`, and a graph that cannot be rebuilt from bare git objects is a failed release.

### 6.4 Session resume via graph query

The resumable-state principle of §2.2 upgrades from convention to query: a resuming agent runs `spine context INT-042` and receives the intent doc, its ACs and their current test results, the frozen manifest and reopen history, declared touchpoints and any active lease collisions, the constitution version it was built under, and any ADRs touching the same code units — assembled from the graph, scoped to the task, with zero reliance on anyone's chat history. Agent B still receives only its intent doc, its tests and the interface slice of §4.2: `spine context` scopes by role, and in-flight intents on other branches are never part of an agent's packet.

### 6.5 The dependability suite ⚙

The field's frontier question has shifted from "can agents code?" to "can we depend on agent work?" — and dependability is measured, not asserted. Three commands, all reading data the graph already collects:

- **`spine stats`** — cycle time per intent, **token cost per intent** (the A↔B loop spends real money, and "what does a feature cost" is a month-one question), A↔B bounce-back counts, wire fire rates by gate, quick-lane escalation rate, and every counter this playbook defines (reopens and late reopens, red-at-approval ratio, freeze and scaffold overrides, re-verify count, starvations, withdrawals, unattested, resealed and recovery landings, self-approved protected reviews, reviewer diversity — the only statistical answer to one human holding two keys). This is what turns the playbook's thresholds (`C-Q2`, 14-day staleness, 6-AC cap, 200-file closure) from guesses into evidence, and what tells you when warn-before-block mode has earned the right to block. Output is text; someone else can chart it — no dashboard UI in scope.
- **`spine review <id>`** — when a wire fires, the reviewer receives an assembled packet: the intent doc, the tests grouped by AC with their frozen blobs, the diff of the synthetic merge, and exactly which wire tripped and why. Review fatigue is the documented killer of gated workflows; a good packet is the antidote, and it keeps the review anchored on tests-versus-intent rather than line-by-line code reading. (`spine check --review` ships the minimal version in v1; the rich packet lands at roadmap step 6.)
- **`spine eval`** — a golden-set harness for the interview agent: replay past intents from their envelopes, score AC testability and non-goal coverage against how those intents actually played out. The riskiest assumption in the whole system (does the interview produce genuinely testable ACs?) graduates from a thing we spot-audit to a thing we regression-test.

### 6.6 Post-landing lifecycle

Intents do not end at "merged" — production disagrees. Three further states close the loop, all derived from git objects:

- **reverted** — derived, never declared, by the patch-id rule of §6.2: a later landing whose diff over `L`'s paths reverses `L`'s. A revert lands through the pipeline like any other change — usually a `BUG-` intent whose AC-1 is the reproduction — so it is both `cs:R implements BUG-051` and `cs:R reverts cs:L`; the extra reproduction test does not disqualify it because matching is restricted to `L`'s paths. A partial reversal is a warning, not a status flip. Tombstones and reseals never revert and are never reverted; a revert is never matched against a landing that follows it. A revert is the loudest possible input to the learnings loop: it should almost always produce an ADR.
- **superseded** — a later intent whose `Supersedes:` header names this one lands with a `Spine-Supersedes` trailer; the indexer emits `superseded_by`, so archaeology queries return the current truth first and the history behind it.
- **withdrawn** — a tombstone landing (§5.5) records an abandoned intent on trunk: no code, the signed doc, the reason. `spine stats` counts it; archaeology can find it; the id is retired. A plain withdrawal is signed by the key on the branch's `Spine-Signoff` (any signer when there is none); an orphaned branch — its signer's key gone from the keyring — is withdrawn with `--withdraw --protected`, signed under `spine-review@v1` by a reviewer ≠ the original signer (§11).

A revert is detected, never declared; a supersession is sealed, never asserted; a withdrawal is landed, never deleted.

### 6.7 The install lifecycle ⚙ — upgrading without losing anything

`spine init` writes files into someone else's repository: a CI workflow, a managed block in `AGENTS.md`, a keyring, a `.gitignore` entry. Every toolkit that does this without a lifecycle dies the same way — a repo running gates from three versions ago, a refresh that clobbers a hand-tuned CI file, a graph schema the old binary cannot read, and nothing that pins *which* spine binary CI must run. The fix is the one package managers settled on: a lockfile, hashes, and a refusal to overwrite what you did not write.

**The manifest.** `spine init` writes `.spine/manifest.json` and commits it. It is machine-written, never hand-edited (G16 enforces this) — a lockfile, not a document, and not a fourth prose artifact: it records a *decision* (which toolkit this repo agreed to run), which is the one thing a derived graph cannot reconstruct.

```json
{ "manifest_version": 1,
  "repo": "myrepo",
  "cli":       { "version": "1.4.0", "dist_hash": "sha256:9f2e…" },
  "schema": 7, "envelope": 1, "object_format": "sha1",
  "templates": { "intent": 2, "intent-change": 2, "intent-bug": 2, "constitution": 1,
                 "ci-github": 4, "ci-generic": 4, "agents-block": 2, "keyring": 1 },
  "resign":    { "intent": 2, "intent-change": 2, "intent-bug": 2 },
  "params":    { "ci": "github", "lang": "python", "trunk": "main", "isolation": "container" },
  "paths":     { "constitution": "CONSTITUTION.md", "agent_context": ["AGENTS.md", "CLAUDE.md"] },
  "files": [
    { "path": ".github/workflows/spine.yml", "owner": "user-modified", "template": "ci-github@4",    "base": "3b1c…", "blob": "77e0…" },
    { "path": ".spine/ci.sh",                "owner": "spine-owned",   "template": "ci-generic@4",   "blob": "51d9…" },
    { "path": ".spine/allowed_signers",      "owner": "user-owned",    "template": "keyring@1",      "blob": "0aa7…" },
    { "path": "AGENTS.md#spine",             "owner": "spine-owned",   "template": "agents-block@2", "blob": "c41a…" },
    { "path": "CONSTITUTION.md",             "owner": "user-owned",    "template": "constitution@1", "blob": "e9a2…" }
  ] }
```

`blob` is the git blob id of what spine wrote (`git hash-object --path`, so `.gitattributes` and CRLF churn are not drift); `base`, on a `user-modified` entry, is the pristine render the human diverged from (updated on every `--merge`). Using git's own hash is the point: the pristine content stays reachable forever through the upgrade commit, which is what makes three-way merge and rollback work on an offline clone holding nothing but git objects — spec-kit hashes raw bytes and keeps no pristine copy, so it cannot. `path#marker` names a managed region — a `<!-- spine:begin agents-block@2 --> … <!-- spine:end -->` block inside a file spine does not own; a region is located by its markers only, and `init` never re-creates a region whose recorded content still appears in the file without markers (it refuses with "markers removed"; the exits are restoring them or `--adopt AGENTS.md#spine`, after which spine stops writing it and G16 stops checking it). `params.isolation` records the boundary the repo's runners actually provide (§7.4 rule 3) — read from trunk, so a candidate cannot claim its own, and **absent means `none`**, so a manifest written before the field existed fails the auto-merge precondition rather than passing it by silence. `dist_hash` is the SHA-256 of the release's *artifact list* — a file the release publishes naming every platform artifact and the wheel with its own SHA-256; each binary embeds the list's hash and verifies its own bytes against the list's entry for its platform at start-up, so one pin covers every platform and a binary not in the list is not the release. `manifest_version`, `cli`, `params.trunk`, `paths` and `files[]{path, owner, blob}` are the manifest's **frozen fields**: every binary parses them for every `manifest_version` it will ever meet and treats the rest as opaque — that is what lets an old binary judge a new manifest (below). Their names, their types and the `owner` set never change, and neither does what a `paths` key means: `paths` is an open map whose every key, present or future, names a repository path or a list of them, and every such value is a floor entry — so a binary preserves keys it does not know and evaluates them as floor. That is what makes "every `manifest_version` it will ever meet" a promise a binary shipped today can keep. A release that must break one of those invariants is not a bump but `--uninstall` and re-init, the one path that starts a new manifest lineage. Templates and agent prompts are embedded in the binary and never written to the repo: there is nothing to customise, which is what "the template never expands" (§3.3) means mechanically, and prompt tuning is a toolkit release, not a repo edit (anything loaded into an agent session is instruction surface, §7.3).

**Three ownership classes, one rule each.**

| Class | Who writes it | On upgrade | Examples |
|---|---|---|---|
| `spine-owned` | spine, every version | Rewritten **only if** the HEAD blob equals the manifest blob. Any other blob is a human edit, and the upgrade refuses | CI workflow, `.spine/ci.sh`, `AGENTS.md#spine`, `.gitignore#spine`, `.gitattributes#spine` |
| `user-owned` | spine once (seed), humans after | Never touched again — by upgrade, by `--force`, or by rollback. `--status` reports "still identical to seed" as a health warning (a permanent false positive for a solo keyring, and it says so) | `CONSTITUTION.md`, `.spine/allowed_signers`, `adr/` |
| `user-modified` | spine once, then adopted (`--adopt <path>`, or a successful `--merge`) | Never rewritten silently; upgrade reports "template moved"; the recorded `base` blob lets `--merge` offer a three-way merge | a hand-tuned CI workflow |

Class is declared; *modified* is never declared — it is detected by hash. Spine cannot lose an edit it can see, and it sees every edit because it knows exactly what it wrote.

**Upgrade is re-running `spine init`.** There is no upgrade command. On an initialised repo, `init` is idempotent: it renders every template the binary ships using the manifest's `params` (flags given on a re-run — `--ci`, `--strategy`, `--trunk`, `--pipeline-key` — update `params` and are an upgrade like any other — except `--pipeline-key`, which appends the seal line to the keyring: that landing is a keyring change under the chain rule (§7.5), and in team mode it strips the seal namespace from every human line; G13 refuses a team-mode keyring with no `spine-seal@v1` principal — so a repo that starts solo and offline can grow a remote and a pipeline without a second bootstrap), compares blob ids, and emits a per-path plan — `create · update · delete · skip · REFUSE`. Then:

1. **Preconditions.** Working tree clean, except paths whose blob equals a render of a pending run (the interrupted case, below). Binary not older than the manifest. A branch `spine/upgrade-<version>` is created from trunk: upgrades land through `spine check --land` like everything else — quick lane, under a protected review (§7.3), self-signed and recorded in solo mode. Only the very first `init` commit, the trust root (§7.5), lands directly.
2. **`--dry-run`** prints the plan and a unified diff; writes nothing; exits 0, or 2 if it would refuse. CI may run it to announce pending upgrades.
3. **Refusal is the default.** One `spine-owned` path with HEAD blob ≠ manifest blob stops the whole upgrade — a partial upgrade is the interrupted case by another name. Resolution is explicit: `--merge` runs `git merge-file` (base = manifest blob, ours = HEAD, theirs = new render); a clean merge lands and reclassifies the path `user-modified`; a conflict refuses (conflict markers never touch the tree). `--adopt <path>` reclassifies without merging — spec-kit preserves such files with a warning; spine refuses until you say which class they are. `--force <path>` overwrites — recorded on the upgrade line and counted by `spine stats`, the same loud-override rule as break-glass.
4. **Atomic apply.** Everything is rendered into gitignored `.spine/cache/staging/<run>/` — with the renders of the binary that started the run recorded in `staging/<run>/manifest.json` before any rename — and parse-validated (YAML, JSON) before a single tree file changes; each file then moves into place by atomic rename; the manifest is written **last**; staging is deleted. The manifest therefore always describes the last *completed* upgrade.
5. **One signed event, one landing.** The upgrade commit on the branch carries `Spine-Event: upgrade` and a signed `Spine-Upgrade: from=<A> to=<B> manifest=<blob> forced=<paths> signer=<p>` line (rollback and uninstall are upgrades with `to=<A>` / `to=none`; a rollback also carries `from-manifest=<sha>`, the ancestor it restores); the landing copies it into the envelope — findable and auditable with no hosting provider in the loop, readable under squash, and giving the landing a signer for the reviewer ≠ signer rule. `forced=` is a hint; the indexer derives it from blobs, and a disagreeing line fails G16.
6. **The graph cache is deleted.** Schema migration is *nothing*: `spine index` rebuilds under the new schema. This is the iron rule paying rent — a toolkit whose graph were authored would need a migration framework here; ours needs `rm`.

**Who evaluates an upgrade.** The *base's* pinned binary — the old one — like any other floor change (§7.4). Three rules make that possible: (1) the frozen manifest fields above; (2) for a landing carrying `Spine-Upgrade`, G16 reads the manifest *in `T`* for the blob comparison and requires `from=` to equal the base's pin and `to=` to equal `cli.version` in `T`, while G15 still binds the running binary to the base's pin; (3) diff-size and dependency wires never apply to spine-owned paths (§5.2) — they are renders of a pinned release, verified by blob — so an upgrade's only wire is the floor, and it never leaves the quick lane.

**Interrupted upgrade.** Crash anywhere and one of three states remains, each detected by hash, each fixed by re-running `spine init`: staging exists and the tree is untouched (continue); some files renamed but the manifest is old (their blobs equal the recorded renders, so the re-run recognises its own work and continues); manifest new but uncommitted (commit). A re-run by a different binary reports "interrupted by <version>: run that version, or `--abort`". `spine init --abort` discards instead: `git checkout` every manifest path, delete created paths, delete staging. Because the tree was clean before, abort is total.

**Rollback = revert the upgrade — by path, not by trailer.** `spine init --rollback [<sha>]` locates the upgrade landing `U` (default: the first-parent commit that last touched the manifest), reads the *old* manifest from `U^`, restores every `spine-owned` and `user-modified` path listed in either manifest to its `U^` blob (`git checkout U^ -- <path>`, or `git rm` for paths `U` created) — never a `user-owned` path: the keyring and constitution change only through their own protected PRs, and a toolkit rollback is not a governance rollback — writes `U^`'s manifest with `paths.*` replaced by the union of `U^`'s and `U`'s entries (the floor never shrinks, not even on rollback), and lands it with `Spine-Upgrade: from=<B> to=<A>`. Path-based restore survives squash landings and rewritten messages; it needs only `U` and `U^`. A path whose HEAD blob ≠ its `U` blob was modified after the upgrade and is refused unless `--force`. `--rollback`, `--uninstall` and `--status` are exempt from the version gate, so an *older* binary can always back out a yanked release or leave — and a rollback lands one of two ways. With `<B>` installable it is an ordinary upgrade landing, evaluated and sealed by the trusted stage under `<B>` (`tool=<B>`). Otherwise — `<B>` uninstallable, or `<B>` is the release being backed out — `init`, not `check`, writes the envelope on `spine/upgrade-<A>`; the reviews are signed there with `<A>`'s `--review` (the local skew check reads the checkout's manifest, which now pins `<A>`), and a second `init --rollback` collects them and seals — the solo key in solo mode, `mode=recovery` (§7.5) in team mode; its seal names `tool=<A>` where the base pins `<B>`, and G15 accepts that only on a rollback `Spine-Upgrade` landing that passes G16's restoration rule against its `from-manifest=` ancestor, whose `to=` equals the seal's tool, and whose seal is solo or `mode=recovery`. When the pinned release cannot be installed at all, the trusted stage is by definition absent and the rollback is a recovery landing (§7.5). This is genuinely beyond spec-kit, whose scaffold upgrade explicitly does not tear down on failure and whose CLI upgrade prints a manual re-pin hint; say so in the docs, do not claim parity.

**Version skew.** Every `spine` invocation compares itself to the manifest before doing anything:

| Binary vs manifest | Local | `spine check --ci` |
|---|---|---|
| equal | ok | ok |
| newer | one-line "upgrade pending: run `spine init`"; everything works; `spine new` stamps the *manifest's* template version | **fail** (G15) — CI runs the pinned hash or nothing |
| older | **refuse** every command except `init --status`, `init --rollback` and `init --uninstall` | **fail** (except a landing carrying `Spine-Upgrade`, evaluated by the base's pin) |

The CI snippet spine writes installs the binary *from the manifest pin* before running `spine check --ci`, and `check` re-verifies its own hash against the manifest before evaluating a single gate. This is §7.4's trusted-execution requirement made concrete — the specific gap the v0.6 review's F3 and F5 share: spec-kit records the version that wrote each manifest, but no hash, and nothing installs from or verifies against it. There is no `spine self upgrade`: local install is a release binary or `uv tool install`; skew detection replaces self-management in v1, and a `self`-style flag is explicitly out of scope. A `manifest_version` bump lands like any other upgrade: the base's pinned binary evaluates it through the frozen fields alone, which is what those fields are for. The skew table governs what happens *after* it lands, never the landing itself — a local binary that does not know the new version is `older` from then on, and backs the bump out with `init --rollback` if the release is yanked.

**Templates and the `Template: vN` header.** The manifest records which version `spine new` stamps; the binary keeps a parser for every template and envelope version ever shipped, so history always parses. Template bumps are additive by policy. A bump that adds a mandatory section is flagged `resign` in the release notes and the manifest's `resign` floor; G4 trips a wire for in-flight intents below it (§6.3), `--sign` refuses the old version, and `spine new --reopen` rewrites the header and stubs the new sections (§4.3). `resign` bumps are rare, announced, and counted by `spine stats` so the maintainers feel the cost.

**Uninstall.** `spine init --uninstall` removes clean `spine-owned` paths and managed regions, leaves `user-owned` and `user-modified` files in place (reported), removes the manifest and cache, and lands with `Spine-Upgrade: to=none`. Landed intents stay in git as envelopes; a later `spine init` + `spine index` reads them all back, and G9 treats the first-parent range between the uninstall landing and the re-init landing (which names it with `from=none since=<sha>`) as pre-adoption history — exempt, bounded by two envelopes. `since=` must name a landing carrying `to=none`, or the re-init is refused and nothing is exempt. A re-init is a keyring landing under the chain rule with the uninstall landing as its parent: its seal and reviews verify against the keyring at `since=`, and a keyring at the re-init that differs from the keyring at `since=` is refused — gap edits are re-landed as a protected PR afterwards. It is evaluated and sealed by the binary its `to=` names, the way a rollback is (`init` writes the envelope; solo key, or `mode=recovery` in team mode), because the base has no pin and no workflow. Leaving costs what arriving cost — the disposal rule's guarantee, applied to the toolkit itself.

**`spine init --status`** prints the table humans want: binary vs manifest versions, cache schema, mode, chain status, which human keys are not hardware-backed, and per path: owner · template@version · `clean | modified | missing | foreign` · planned action. `spine check` runs the same comparisons as G15 and G16.

Git requirements: §11. `spine init` probes the remote with a throwaway ref — a stale `--force-with-lease` must be rejected, or auto-merge stays off. Object-format migration (SHA-1 → SHA-256) invalidates every recorded blob id; the manifest records `object_format` so a future indexer can rehash, but v1 does not support the migration and says so rather than failing silently.

---

## 7. Threat model, authority, and trusted execution ⚙

Spine-kit auto-merges machine-written code. From an attacker's perspective that is the product: get code past the gates and it reaches trunk with no human. Everything above hardens the pipeline against *accidents*; this section hardens it against *adversaries* — a drifting or prompt-injected agent first, a stolen key second, a malicious insider last. A `SECURITY.md` derived from this section ships with v1.

**A signature proves identity; authority is a policy question** — and policy is answered by the merge target, never by the branch asking to be merged.

### 7.1 Least privilege per stage

| Stage | May read | May execute | Network | Keys |
|---|---|---|---|---|
| Interview agent | repo, ADRs, constitution | nothing | none | none |
| Agent A (tests + implementation) | scoped code-graph context | sandboxed build/test only | none | none |
| Agent B (adversarial) | intent doc + tests + interface slice (§4.2 isolation is a security control, not just a correlated-failure one) | sandboxed test runs only | none | none |
| Untrusted CI stage | the synthetic merge `T` it computes itself | the candidate's build and tests, sandboxed — spawned by trunk's collector, which owns the result file (§7.4 rule 3) | an allow-listed registry proxy during dependency restore, verified against the lockfile's hashes, then none (the allowlist lives in `.spine/ci.sh`, a floor path, §7.3) | none |
| Trusted CI stage | git objects; policy from trunk | the pinned spine release, gate queries — nothing from the repo | push to trunk only | pipeline key |
| Human | everything | anything | anything | signer / reviewer key |

The Keys column is the boundary. Agents never hold key material and never see a signing agent: the sandbox strips `SSH_AUTH_SOCK`, `GPG_AGENT_INFO`, `~/.ssh`, `~/.gnupg`; any invocation that produces a `-Sig` line with a key that is not the `--ci` pipeline secret — `--sign`, `--reopen`, `--withdraw`, `--approve`, `--review`, `--break-glass`, and `--land` outside `--ci` — is TTY-only and refuses under `SPINE_AGENT=1`; in solo mode `--land` prompts for the key touch and prints `mode=solo` before signing. Outside `--ci`, `--land` seals under `spine-seal@v1` when the operator's key holds that namespace (solo); otherwise it seals `mode=recovery` under `spine-review@v1` iff the landing is recovery-eligible (§7.5), and refuses otherwise. Say the residual plainly: an agent running in a terminal whose agent socket holds an unlocked signing key can satisfy the TTY check with a pty, and on Windows the agent pipe is reachable by every process of the user and `ssh-add -c` does not exist; TTY detection falls back to ConPTY probing where `isatty` is false under Git Bash or mintty. Nothing in spine closes that; a hardware key or `ssh-add -c` (confirm on use) does. `spine init --status` reports every human key that is not hardware-backed.

**Injection defense.** Repository content — code, comments, README files, dependency docs — is *data, never instructions*, in every agent prompt. An instruction embedded in a source comment ("ignore previous constraints and…") is content the agent reasons *about*, not a directive it follows. This rule is written into every shipped agent prompt and tested in `spine eval`.

**Pragma provenance.** A `@verifies INT-042/AC-1` pragma is trusted iff it sits inside a blob some approval froze (§6.2): in flight, the binding approval's frozen files (or, before approval, a file on the intent's own branch under `C-T1`); landed, the frozen blobs reachable through `L`'s tree forever. Anything else — a direct push, a drive-by commit, an unrelated branch, a pragma added after approval — indexes as `attributed: false` and fails the Integrity family (G5). Corollary: adding a `@verifies` pragma is by definition gated-lane work, so the quick-lane router escalates any diff that introduces one.

### 7.2 Authority: keys, roles, and what a signature binds

Three roles, no more (§10 budgets them), expressed as SSH signature **namespaces** in `.spine/allowed_signers` — git's own `allowed_signers` format, so `ssh-keygen -Y verify` enforces role membership with zero spine code, on an offline clone with only git objects and OpenSSH:

```
# .spine/allowed_signers — roles are namespaces; ssh-keygen enforces them
alice@example.com  namespaces="spine-signoff@v1,spine-review@v1"  ssh-ed25519 AAAA…
bob@example.com    namespaces="spine-signoff@v1,spine-review@v1"  ssh-ed25519 AAAA…
ci@example.com     namespaces="spine-seal@v1"                     ssh-ed25519 AAAA…
```

| Role | Namespace | Signs | Held by |
|---|---|---|---|
| signer | `spine-signoff@v1` | sign-off, reopen, withdraw, toolkit upgrade events | humans |
| reviewer | `spine-review@v1` | reviews (tripwire, protected, break-glass); approvals in v1; the seal of a recovery landing (§7.5) | humans |
| pipeline | `spine-seal@v1` | the seal; approvals carrying `run=` once B runs in the trusted stage | the trusted stage — a CI secret no laptop holds; in solo mode, the human's own key |

The constitution carries the rules (`C-A1`, `C-A2`), the keyring carries the keys, and neither is a fourth prose artifact: the keyring is key material in git's own format, generated by `init`, edited only by protected PR — like CI YAML or `.gitignore`, which the budget has never counted. v1 supports SSH signatures only (one verifier, the format git already uses for `gpg.ssh.allowedSignersFile`); OpenPGP is v1.1. The versioned namespace suffix is the cheapest place to version a signature payload format.

**Every signed statement has one shape.** One trailer line ending in `signer=<principal>` (reviews: `reviewer=`), plus `<Name>-Sig: <SSHSIG, armor stripped to one line>` produced by `ssh-keygen -Y sign -n <namespace>` over the exact bytes of that line; `reason=` values are JSON string literals. Event lines reference *content* (`blob=`, `tree=`, `freeze=`) and `head=`, never other commits' SHAs, so merging trunk into a branch never invalidates them. Verification: in-flight statements against the keyring at trunk's current tip; landed statements against the keyring at the **seal's** `base=` — `L`'s first parent, or for a reseal the last valid landing below its range; an approve line's own `base=` is audit data. Two clocks, deliberately (§7.5). Mode counts distinct **keys** under `spine-signoff@v1`; reviewer ≠ signer compares fingerprints; G13 refuses a keyring listing one key under two principals.

**Self-approval, defined.** Signing your own intent is authorship, not approval — the interview output is yours, and it is the normal path for everyone. Signing your own *approval* attests B's verdict, not code quality, and is permitted in every mode (until B runs in the trusted stage and the pipeline signs it). *Self-approval* means a `Spine-Review` whose key equals the landing's signer key — the key on its `Spine-Signoff` or, for an upgrade, on its `Spine-Upgrade` line. The rule:

| Mode (`C-A1`, verified against the signoff-key count) | tripwire review | protected review · break-glass |
|---|---|---|
| **solo** — exactly one human signer | self, recorded `self_approved` | self, recorded `self_approved`, counted in `spine stats` |
| **team** — two or more human signers | self allowed: the signer knows the intent best, and the wire is a quality wire | **reviewer ≠ signer**; refused otherwise |

A landing with no signer — a reseal, a bare keyring change — has no key to differ from: in team mode its protected review requires **two** `Spine-Review class=protected` lines from distinct keys in the parent's keyring. The rule binds to the intent's signer, not to who produced the diff: a pair that splits authorship from sign-off can put one person on both sides of a floor change — a residual, not a guarantee; `spine stats` reviewer diversity is the statistical answer, and a two-hands-per-floor-change rule is deliberately not imposed (§10). Solo mode is not a weaker posture against the adversary that matters: the agent holds no key, so every protected landing still needs a human hand on a key over the exact tree. What solo mode cannot give is protection against the solo human — nothing can — and the seal carries `mode=solo` so nobody mistakes the one for the other. In team mode G13 refuses a keyring in which a human key is also listed under `spine-seal@v1`: the landing that enters team mode strips the seal namespace from every human line, and any later human seal is `unattested` — except the recovery form of §7.5. One human enrolling two keys is indistinguishable from a two-person team ("sybil solo"): the mitigations are social and statistical only — keys enter via reviewed PRs, and `spine stats` reports reviewer diversity. When the last other signer's line is deleted, the same protected landing may carry the `C-A1: solo` edit; until it does, the count mismatch is a warning on every report.

### 7.3 The protected floor

Touchpoints are the merge policy for code. They are *not* policy for the machinery that evaluates the policy. The following can never auto-merge, whatever any intent declares, and always take a protected review (G14):

- `.spine/**` — manifest, keyring, `ci.sh`
- the constitution and every agent-context file the manifest lists — anything loaded into an agent session is instruction surface
- CI definitions: `.github/workflows/**`, `.github/actions/**`, `.gitlab-ci.yml`, `.circleci/**`, `.buildkite/**`, `Jenkinsfile*`, and the rest of the floor list pinned in the release — and because a provider executes a candidate's workflow before any review, the pipeline key must be unreachable from it (§7.4, rule 0)
- `CODEOWNERS`, wherever it lives
- files that make git execute or fetch code: `.gitattributes`, `.gitmodules`, `.githooks/**`, `.husky/**`, `.pre-commit-config.yaml`
- any diff entry that adds or changes a **symlink** (mode `120000`) or a **submodule pointer** (mode `160000`) — the two ways to reach a protected path without naming it

Agent-context, hook and attribute names match at **any depth** (`**/AGENTS.md`, `**/CLAUDE.md`, `**/.claude/**`, `**/.cursor/**`, `**/.gitattributes`, …) and **case-insensitively** — paths are casefolded before comparison, and a diff entry whose casefolded path equals an existing path's is itself a floor hit: two spellings of one file are a collision, not a new file. The floor ships *inside* the pinned spine release, so a repository cannot shrink it; `C-A2` can only extend it, and every `paths.*` entry in the manifest is a floor entry and is monotone the same way — a landing whose tree drops an entry present at the base fails G14 outright, review or no review; `params.trunk` is a rendering hint, and the trusted stage protects the branch it is configured for out-of-band. Matching runs over the full `merge-base..head` diff including renames and deletions — renaming `ci.yml` to `ci.yml.bak` is a touch. Lists go stale as providers multiply; between releases an unknown provider's config directory is unprotected unless `C-A2` names it. Where the host offers CODEOWNERS or branch protection, `spine init` emits matching entries as a supplement; the guarantee does not depend on them.

### 7.4 Trusted execution: policy from the base, code from the head

Gate results are worth exactly what produced them. Six rules:

0. **The trusted stage's own definition is policy.** The trusted job runs from trunk's workflow file, never the candidate's — **and so does the untrusted job**, because a push-triggered job runs the candidate's definition and could simply never call the collector: the untrusted job is triggered from trunk's definition (GitHub: `pull_request_target`, or a `workflow_run` dispatcher on trunk, with `permissions: contents: read` and no secrets; GitLab: an MR pipeline whose config is `include:`d with `ref: <trunk>`; `--ci generic`: a definition outside the repository), and a result file from a job that was not is never ingestible. The CI snippet `spine init` writes triggers `spine check --ci --land` only from a trunk-scoped event (`workflow_run` of trunk's own untrusted workflow — never `merge_group`, which executes the merge group's own workflow file on a `gh-readonly-queue/*` ref that fails a trunk-only deployment rule; under a merge queue the untrusted job runs on `merge_group` and the trusted job is chained from it by `workflow_run`); the pipeline key lives in a provider environment whose deployment-branch rule is the trunk only (GitLab: a protected variable, `intent/*` and `quick/*` unprotected, and the trusted job is a pipeline on `ref=<trunk>` — a schedule that polls for candidates or a trigger scoped to that ref — so `.gitlab-ci.yml` comes from trunk by construction while MR pipelines are the untrusted job; `--ci generic`: the trusted job's definition lives outside the repository, pinned to trunk — a `Jenkinsfile` read from a candidate is the candidate's, and `init` refuses `merge.auto = on` for it); the untrusted job is the only job that runs on `intent/*`, `quick/*` and `spine/upgrade-*` pushes, runs with `permissions: contents: read` (its ambient `GITHUB_TOKEN` is a secret in all but name) and receives no other secret; the bypass principal of configuration (a) is a deploy key or app installation only the trusted job holds, never the Actions token both jobs share; `.spine/ci.sh` is executed from `git show origin/<trunk>:.spine/ci.sh`, never from the checkout. The probe is the untrusted job itself: it fails the run if the pipeline-key variable is visible to it, and the collector (rule 3) writes that assertion and its own `tool=` into the result-file header. A run whose ingested header lacks either — the assertion, or a collector pinned by the base — fails the first of the auto-merge preconditions below. Every such test is per run and remembers nothing between runs: a repo whose results do not come from trunk's collector never auto-merges, and never latches itself open by having once produced a good header. The bypass principal of configuration (a) bypasses required checks only — the non-fast-forward rule has no bypass list.
1. **Policy is read from trunk.** The trusted stage **and the collector** read `.spine/manifest.json`, `.spine/allowed_signers` and the constitution's scaffolded rules from `origin/<trunk>` (`git show origin/main:.spine/manifest.json`), never from the checkout under test. A candidate may change policy; that change is a floor hit reviewed under the *old* policy, and governs only later landings.
2. **Gates run from a pinned, hash-verified release.** The manifest's `cli.version` + `cli.dist_hash` pin the binary; the trusted stage installs that exact release, verifies the hash, and refuses to run anything else — including a spine built from the repository. `.spine/ci.sh`, read from trunk, installs and hash-verifies the collector the same way before anything else runs in the untrusted job; a mismatch fails the run, and no result file exists to ingest. This holds in spine-kit's own repo: dogfooding runs the *released* spine; building from the branch is a test, not a gate.
3. **The graph is rebuilt from git objects, every run.** `spine index --fresh` is implied by `spine check --ci`; no SQLite file is fetched, cached or trusted from anywhere, and the trusted stage restores no cache at all. It executes no repository code: the untrusted stage computes `T := git merge-tree --write-tree origin/<trunk> H` itself (`H` is the ref the run names: `intent/<ID>`, `quick/<name>` or `spine/upgrade-<version>`), tests a detached checkout of `T` — never `H` — under **the collector**. The collector is `spine check --ci --collect`: the pinned release, invoked by trunk's `.spine/ci.sh` (rule 0), holding no key and signing nothing. It collects the id set on a checkout of `B` *before* `T` exists — so no candidate can make a landed test uncollectable — then spawns the runner as a child, reads its machine-readable stream over a pipe, and — after reaping the whole process group — writes the result file itself, in a format carrying runner-native test ids. Where it writes it is the **isolation profile**. The collector measures what it actually got and names it in the header; trunk's `params.isolation` says what the repo claims to provide, and the trusted stage requires the two to agree — a header alone proves nothing, since a job with no boundary can write any header it likes:

   - `profile=container` — the runner ran inside a container the collector created; the result directory is outside it and unmounted, and the stream crosses on a pipe the collector holds.
   - `profile=uid` — the runner was dropped to a second uid; the result directory is mode 0700 under the collector's own.
   - `profile=none` — one uid, one process tree, no boundary at all.

   The uid or the container is the boundary, never the path: a directory mode means nothing against processes sharing one uid, and the threat set is every process in the job — dependency restore and build as much as the runner. Under `profile=none` the collector owns nothing and the design says so rather than pretending: the result file is advisory, and `C-M4` cannot evaluate `on` (below). Its header carries `tree=<T> base=<B>`, the id set collected on `B`, rule 0's key-visibility assertion, `profile=`, and the collector's own `tool=`. The trusted stage ingests a result file only if its `tree=` equals the `T` it computed — a mismatch is `base-moved` — and a header whose collector `tool=` ≠ the base's pin is a G15 failure, never a retry. A `base-moved` exit ends the run; the snippet re-queues the whole two-job run on the new `T`, and `C-M3` counts runs per `(intent, head)` in the gate report. The trusted job checks out with full history plus an explicit fetch of `refs/heads/intent/*`, or the lease registry is empty in CI.
4. **Every landing is attested.** `spine check --land` produces a canonical-JSON **gate report** — intent blob, base, head, tree, tool version and hash, git version, the ids of the policy files it read, mode, per-gate results, floor hits, the verified sign-off, approval and review lines, `self_approved` — and seals its SHA-256 into the envelope (`report=`). The report is *recomputable*: `spine check --verify <landing-sha>` re-runs the pinned release over the same objects (it requires the seal's git major.minor) and compares digests. Recomputation needs `H` reachable, which is why the gated lane defaults to merge strategy; under squash the audit is the seal plus G9's freeze audit. The full report may also be written to `refs/notes/spine` as a convenience — notes are not fetched by default and nothing depends on them; they are never a source.

5. **Auto-merge is a capability, not a preference.** `C-M4: merge.auto = on` is a request; whether a run may act on it is computed, per run, from four preconditions — each read from trunk or produced by this run, never asserted by the branch asking to merge:

   1. trunk's manifest declares `params.isolation` as `container` or `uid` — never `none` — and the ingested header's `profile=` equals it. The manifest is the authority and the header only has to agree, because a header is the one thing a job with no isolation can always forge: under `profile=none` nothing stops a second process writing a file that *claims* `container`. `params.isolation` is on the protected floor like the rest of `.spine/**` (§7.3), so raising it is a reviewed change to trunk, never a candidate's self-declaration;
   2. it carries rule 0's key-visibility assertion, from a collector whose `tool=` is the base's pin;
   3. `C-M5: merge.reconstruct = inline`, so reconstruction was proved before this push (§5.4 step 5);
   4. this run is performing the compare-and-swap itself — `--land` in the trusted stage, never `--land --print`, which hands an envelope to a provider that will create its own commit (configuration (b) of §5.4). The run knows this about itself; it is the one precondition that needs no evidence from elsewhere.

   Any precondition missing and the run evaluates `C-M4` as `off` whatever the constitution says, and raises a **`G1` wire** naming which one — so the mandatory review is pointed at the test outcomes and the reason they are unattested, not at a generic tripwire. The profile is sealed into the landing (`profile=` on `Spine-Seal`), so the ledger says, for every landing forever, how strong the evidence behind its green suite was. This is one rule with three former patches folded into it: an unattested channel, a deferred reconstruction and a provider-made commit are the same failure — a landing whose central claim was never established before it became irreversible.

Two CI jobs, one command: the untrusted job builds and tests; the trusted job runs `spine check --ci [--land [<id>]]` — `--ci` is a mode (self-hash check first, skew hard-fails, Authority never warns), `--land` the terminal stage. On plain git — an offline clone with only objects, OpenSSH, and no spine — every link is checkable by hand (§11). What plain git cannot do is *prevent* a bare `git push` to trunk. So the guarantee on plain git is detection: G9 derives every trunk commit without a valid seal as an orphan or `unattested`, reports it on every run, counts it forever, and refuses to land on top of it until a human reseals. Prevention is a supplement — branch protection on hosts, or the pre-receive hook `spine init --hooks` emits (`spine check --pre-receive`) on self-hosted git. Two supplements are non-optional: non-fast-forward pushes denied on trunk *and* on `refs/heads/intent/*`, with deletion of intent branches restricted to the pipeline principal — because a history rewrite below the trust root defeats every ledger and is detectable only as pin disagreement across machines, and a rewritten intent branch could erase a signed reopen.

**What a green gate proves, exactly — stated, not hidden.** Under `container` or `uid`, a candidate cannot forge a result *file*, hide a landed test by making it uncollectable, or present one tree's results as another's. What no profile touches is the honesty of the collector's own descendants. Frozen tests must import the code under test, so on the `T` run the candidate's code and the runner share a process, and code that monkeypatches the assertion library or the reporter can lie on the stream the collector faithfully records. **G1's `passed` is therefore exactly as strong as the isolation between a candidate's code and its own runner, and nothing in this design establishes that property.** The bar-raisers are real and none closes it: the read-only harness (G8), `C-T3`, the sandbox, the no-new-dependency wire, ingestion by tree id. **G6 is not the answer** — it runs in the same untrusted stage through the same collector, so a runner that lies about the suite lies about the mutants too. The v0.8 review's remedy — keep auto-merge mechanically unavailable for in-process runners — is **refused, and here is why**: every runner a real repository uses is in-process (`pytest`, `jest`, `go test`), so that rule does not qualify the headline, it deletes the product for everyone while leaving the same residual for anyone who ships around it. The design does the two things that are actually available instead: it refuses `on` unless the *channel* is attested, and it records the profile in the seal so no landing can later claim strength it never had. Turning `C-M4` on is therefore a decision to accept a named residual — a constitution change, so a protected-floor PR (§7.3), and one that should carry an ADR saying who accepted it and why.

### 7.5 Trust root, rotation, revocation

The keyring governs itself, so it needs a beginning and a chain.

- **Bootstrap.** The trust root is the commit `spine.trustRoot` pins — at first init, the commit introducing `.spine/allowed_signers`, which `spine init` signs with a key inside it. Its SHA is pinned out-of-band, like the release hash: the rendered CI snippet reads it from a provider variable (`SPINE_TRUST_ROOT`), never a tracked file, and `spine check --ci` refuses to run without one — trust-on-first-use is a laptop convenience (`spine index` prints the root and its fingerprints once and stores it in `git config spine.trustRoot`, the only per-clone spine setting), never a CI mode. `spine init` prints the root SHA and the variable to set as its last line; changing a stored pin takes an explicit `spine init --trust-root <sha>`.
- **Chain rule.** `spine index` walks trunk first-parent from the tip to the root. Every landing that changes `.spine/allowed_signers` must be sealed by a pipeline key in the *parent's* keyring and carry a protected review by a principal in the parent's keyring (≠ signer in team mode, two reviewers when there is no signer). Signer nodes carry `valid_from`/`valid_to` commit ranges derived from this walk; the chain, not timestamps, is the authority — `valid-after=`/`valid-before=` options are refused by G16's keyring lint. Retirement and revocation are both deletion of the line, in one protected PR. A delta that only *removes* lines needs one protected review from a remaining key that is not a removed line's key — a departed or compromised key is never asked to co-sign its own revocation; a delta that adds or edits a line takes the full rule. This is the one landing a member of a two-signer team makes alone; `spine stats` counts it. One clock, no timestamps.
- **Two verification clocks.** In-flight signatures are verified against the *current* keyring: revoke a key and every intent it signed drops to `awaiting-sign-off`, every approval or review it signed is void, on the next `spine check` — to be redone by someone else. Landed signatures are verified against the keyring *as of the seal's base*: history does not become invalid when people leave — but `spine check --authority` lists every landing signed by a since-revoked key, which is exactly the list a compromise post-mortem needs.
- **Recovery landing.** With no usable pipeline key — lost, or a pin that cannot be installed — a landing may be sealed under `spine-review@v1` by one of two distinct protected reviewers from the parent's set (when the landing has a signer, that signer may be one of the two but never the sealing one); its seal carries `mode=recovery`, and the landing's `diff(B, L)` is confined to `.spine/allowed_signers` and the constitution's `C-A1` line — or, for a rollback, uninstall or re-init, to the manifest and the paths the two manifests list; anything else makes the seal `unattested`. G9 and G15 accept that form only for a landing whose keyring delta removes or replaces every `spine-seal@v1` principal, a **rollback** landing — one that is a *deterministic restoration of one named ancestor*, not a manifest that merely resembles it: the `Spine-Upgrade` line carries `from-manifest=<sha>`, a first-parent ancestor of `B`, and every frozen field and every `files[]` record of the manifest in `T` equals that ancestor's except `paths.*`, which may only be the monotone union of the two (the floor never shrinks, §7.3); every managed path in `diff(B, L)` equals its blob at `<sha>` or is the deletion that ancestor's manifest prescribes. `--rollback` satisfies this by construction; a forward or malicious lifecycle change cannot, whatever its `to=` says, and no gate has to order two version strings — an **uninstall** (`to=none`), or a re-init landing (`from=none`); `spine stats` counts it. Two humans are always at least one human plus a pipeline — the path is honest, not hidden. Compromise of the CI secret holding the pipeline key equals compromise of landing for non-floor code: OIDC-scoped short-lived keys and hardware-backed keys are recommended, not enforced.
- **Rotation.** `spine init --rotate-trust-root` is refused when `C-A1` is `team` — a team recovers through a recovery landing. A solo developer whose only key is gone lands a rotation root carrying `Spine-Trust-Root-Prev: <sha>`, re-pinned out-of-band; the indexer continues the walk below it against the old chain, marking the boundary in every affected signer's `valid_to`. Only a trust-root commit lands directly (§6.7).

### 7.6 Break-glass, not backdoors

Emergencies are real: a 2 a.m. hotfix blocked by a false-positive gate must have a path forward, or the team disables the gate permanently the next morning — the documented death of every gated workflow. `spine check --break-glass "<reason>"` is a `Spine-Review` of class `break-glass`, available only from `tests-approved` onward — never before an approval exists. It bypasses G2, G3, G4, G6, G7, G12 and — of Integrity — G8 and G1 only, recorded as a *freeze override*; never G5, G9, G10, G11, and never Authority: the signature, the floor, and the reviewer ≠ signer rule in team mode still hold, because the emergencies that need a second human are precisely the ones that touch the machinery. The bypassed gates are likewise marked `=override` in the envelope's `Spine-Gates` line — in a git object, not a side file. A break-glass review binds `head` and `tree` like any review: a moved base re-runs step 1 and, if the wire set grows, the break-glass is re-signed. The compare-and-swap is not a gate anyone can override. It records the review as an `approval` node (`class=break-glass`) in the graph, surfaces in `spine stats` (freeze overrides broken out), and auto-opens a retro item that should usually become an ADR. A gate you can never override gets turned off; a gate you can override *loudly* survives its first incident.

---

## 8. Failure modes this playbook is designed against

| Failure mode | Countermeasure |
|---|---|
| Agent drifts from intent | Mandatory non-goals; touchpoint tripwires; AC-named tests |
| Specs take too long | One-page template; 15-minute rule; split-don't-grow; agent-led interview |
| Stale specs mislead agents, or the record of what was approved lives in provider metadata | Intent file deleted at landing; its signed bytes sealed into the landing commit (§5.5): hash-bound text, detached human signature, pipeline seal; G9 audits every landing, and G10 rebuilds the graph from an offline clone *before* that landing is pushed |
| Losing context between sessions | Constitution + ADRs + in-flight intent doc = full resumable state, all in-repo |
| Regressions slip through | ACs compile to tests; gates run on the synthetic merge; trunk's own tests are a pass criterion for every landing, their ids fixed by trunk's collector before the candidate's tree exists — as strong as the candidate's runner is honest, and no stronger (§7.4) |
| Agent weakens, skips, replaces, or re-fixtures an approved test mid-implementation | Signed approval record freezes the test closure by blob id; the harness is read-only from the branch after approval; G8 rejects a changed byte, G1 checks results by identity, G12 refuses green-at-approval; the only exit is a signed, counted reopen. Not closed, and listed as such (§12): an oracle the implementation also imports (§4.3), and a runner that lies about its own results (§7.4) |
| Agents rubber-stamp each other | Context isolation + adversarial framing for the cross-check |
| Auto-merge ships something risky | Tiered tripwires route risky diffs to humans; `C-M4` keeps every landing reviewed until the wires have earned trust |
| An intent modifies the machinery that judges it, or the machinery runs from the candidate | Protected floor (§7.3): CI, `.spine/`, constitution, agent context, hooks, symlinks, submodules — never auto-merged; the trusted job runs trunk's definition with a key a candidate cannot reach (§7.4 rule 0); pinned release, policy from trunk, fresh graph, attested landing |
| A signature proves identity, not authority | Roles as SSH namespaces in `.spine/allowed_signers`, read from trunk, verified by G13 with `ssh-keygen -Y verify` |
| An approval outlives the thing it approved | Sign-off, approval and reviews sign blob / head / tree ids; any edit reopens, and an unsigned edit fails Integrity |
| One human rubber-stamps their own risky change | Self-approval defined; refused for protected and break-glass in team mode; two reviewers for landings with no signer; flagged and counted in solo mode |
| The keyring edits itself | Trust root + chain rule; revocation reopens in-flight work, never rewrites history (§7.5) |
| Concurrent intents collide on the same code, or a tree no gate saw reaches trunk | Leases derived from signed intents on `refs/heads/intent/*`, advisory at sign-off and binding at landing; every record bound to a git object with a stated lifetime; gates on the synthetic merge; `push --atomic --force-with-lease` as the check; G9 re-derives every landing from an offline clone (§5.4) |
| Someone lands code around the pipeline | Every trunk commit is a sealed landing or the trust root; G9 flags orphans, refuses to land on top of them, and a reseal takes a protected review |
| Conventions diverge on greenfield | Week-one constitution, loaded by every agent session |
| Process bloats back into SDD | The three-layer rule: one permanent prose file, everything else disposable or executable |
| Traceability rots via silent broken links | Provenance law + dangling-edge linting (G5, G9): a broken link is a loud build failure |
| Graphs become bureaucracy | Iron rule: derived, never authored — every graph is a gitignored cache rebuilt by one command |
| Constitution rules stay aspirational prose | Rule IDs + `enforced_by:` checks; `spine check --constitution` reports the enforced/aspirational ratio |
| Toolkit upgrade clobbers hand edits, repos silently run stale gates, or CI evaluates gates with an unpinned binary | Install manifest with blob hashes and ownership classes; refuse-by-default upgrade with `--merge`/`--adopt`/loud `--force`; one signed landing per upgrade evaluated by the base's pin; path-based rollback; G15 skew gate; the pin lives on the floor (§6.7) |
| Graph schema change strands old caches | The cache carries `PRAGMA user_version`; migration is delete-and-rebuild |
| Reviewers burn out when tripwires fire | `spine review` packets anchor review on tests-versus-intent, not line-by-line code |
| Concept count creeps up version by version | The complexity budget (§10): human-side limits audited at every version bump, busting a number requires an open argument in the PR |

---

## 9. Adoption notes

**Week one (greenfield):** write the constitution as a team before meaningful code exists; assign its owner; enrol every signer's key; set the trust-root variable in CI; set up the two-job CI skeleton (§7.4) with `C-M4: merge.auto = off`.

**The dogfooding rule:** spine-kit's first intents are spine-kit's own. The toolkit's repository runs its own pipeline from the first commit, making it its own first case study — every gate false-positive and interview weakness is felt by its builders before any adopter. Dogfooding runs the *released* binary, never one built from the branch under test (§7.4).

**First two sprints:** run with `C-M4` off — every landing takes a tripwire review — while you calibrate the Drift wires and learn where the adversarial cross-check catches real problems. Turning auto-merge on is not a maturity milestone the wires unlock: it needs the four preconditions of §7.4 rule 5 to hold in every run — an attested collector profile, inline reconstruction, a trusted-stage CAS — and, past those, a deliberate acceptance of the one residual no gate closes, a candidate's own runner lying about its results. That acceptance is a constitution change, so a protected-floor PR, and it should carry the ADR that names who accepted it. A team whose threat model includes a hostile or prompt-injected agent leaves it off, and loses nothing but the last human click. *Auto-merge off does not mean Authority off* — sign-offs, the floor, the freeze and the seal run from the first commit; they are what you are calibrating the wires against. Solo developers will meet the reseal (§5.5) the first time they commit on trunk by hand; do it once, deliberately, and stop.

**Known open risks to revisit after a month of use:**

- Is the interview agent producing genuinely testable ACs, or plausible-sounding ones? Spot-audit five intent docs against their tests; watch the red-at-approval ratio.
- Has the constitution stayed under 150 lines? If it grew, something that belongs in ADRs or tooling leaked in.
- Are people splitting tasks when they hit the 15-minute wall, or padding the doc? Watch cycle-time per intent doc.
- Reopens are signed events, never edits, so "is anyone quietly reopening intents to weaken ACs" is a `spine stats` number: watch reopens per intent and especially *late* reopens. Two late reopens on one intent is a smell; a falling red-at-approval ratio is the same smell measured differently.
- Are spine-owned files being `--force`d or adopted routinely? A team that has adopted the CI workflow has forked the toolkit and will not receive upgrades there; that is allowed, but it should be a decision (an ADR), not a drift.
- Are re-verify counts approaching `C-M3`? That is starvation, and the answer is a provider queue as runner (§5.4), not a bigger number.

**Spine-kit tooling roadmap (build order, each step useful on its own). v1 ships exactly four commands — `spine init`, `spine new`, `spine index`, `spine check` — and nothing else (four, not three: the §10 budget was amended in v0.6, argued openly as the budget requires — `init` runs once per repo, is the entire on-ramp, and hiding bootstrap inside `new`'s first run would be implicit magic). Every behaviour in this playbook is a flag on those four (§11); signing, approving, reviewing, landing and upgrading are stages of one state machine, and splitting them across commands would split the machine across entry points:**

0. `spine init` — bootstraps the repo: constitution scaffold with the twelve rules of §2.1, the keyring, the manifest, the signed trust-root commit, `.gitignore#spine` and `.gitattributes` entries, the runner's `testpaths`/`roots` pinned to `C-T1`, the two-job CI snippet, `AGENTS.md#spine`, CODEOWNERS entries and a pre-receive hook as supplements, the remote probe; then runs the **constitution interview**: the interview agent's first job is interviewing the *team*, turning the week-one constitution meeting from a blank page into a facilitated 30 minutes. Re-running it is the upgrade path (§6.7). The full lifecycle ships in v1, not later — the first template bump arrives before the first adopter finishes calibrating, and a toolkit that cannot upgrade itself safely gets pinned forever or ripped out.
1. `spine new` — runs the interview (§3.4) on a fresh `intent/<ID>` branch and emits the filled template, stamped with the manifest's template version; variants (`--change`, `--bug`) per §3.5; `--from <branch>` promotes an escalated quick-lane branch. `--sign <id>` performs the one human gate (§3.4); `--reopen` and `--withdraw` re-enter it. Reads/writes AGENTS.md as the agent-context substrate (never a proprietary format).
2. `spine index` — builds the traceability graph (§6.2) from in-flight branches and every sealed envelope reachable from trunk, rebuilding from scratch whenever the cache's schema or builder hash differs. `spine check` runs the five gate families — Drift and staleness in **warn-before-block mode** so the drift gate earns trust before it enforces; everything else blocks from day one — and owns the transitions: approve, review, land, reconstruct, verify (§11). G7, G8, G9, G11 and G13–G16 ship here, not later: auto-merge without them is the race and the forgery the v0.6 review described.
3. `spine context <id>` — session resume via graph query (§6.4), including Agent B's interface slice.
4. Code graph via tree-sitter — touchpoint proposal in the interview, graph-containment tripwires (§5.2 ⚙), the mechanical quick-lane router (§3.5), and `merge.reverify = scoped` once disjointness can be proved (§5.4).
5. End-to-end A/B orchestration against the transition table (§6) — the bounded adversarial loop run automatically in the trusted stage, approvals signed by the pipeline key with a `run=` digest of B's transcript; adopt a workflow engine behind the table only if retries/budgets/resumability genuinely demand one. Optional G6 mutation gate for high-assurance teams. Batched landings (one synthetic merge for several queued intents) belong here too, next to `scoped`.
6. Dependability suite — `spine stats`, the rich `spine review` packet, `spine eval` (§6.5), and constitution enforcement reporting (§2.1); this is the release where thresholds graduate from defaults to evidence, and where `--force`, `--adopt`, reopen and unattested counts tell you whether a team has quietly forked the toolkit or its process.

**Upgrading:** run `spine init` on a branch, read the `--dry-run` diff, land it as the protected-path change it is, then `spine index`. Do the first upgrade *before* enabling auto-merge — a skewed binary is the most boring possible way to lose trust in a gate.

---

## 10. The complexity budget

The DNA — Lightweight, Organised, Harness — rendered as numbers this playbook must pass at every version bump. Any future addition that busts a number must argue for changing the budget openly, in the PR that proposes it:

| Budget | Limit | Currently |
|---|---|---|
| Mandatory human gates | 1 | 1 (intent sign-off; reviews are conditional — a wire or a floor hit) |
| Human-authored artifact types | 3 | 3 (constitution ≤150 lines · ADR ≤1 paragraph · intent ≤1 page) |
| Committed machine-format config files (never prose) | 2 (v0.7) | 2 (`.spine/manifest.json` — machine-written only · `.spine/allowed_signers` — edited only via protected PR); `.spine/ci.sh` is CI YAML-equivalent, uncounted like `.github/workflows/spine.yml` |
| Pages per intent | 1 | 1 |
| Lanes | 2 | 2 (quick, gated) |
| Graphs | 2 | 2 (traceability, code) + 1 transition table (~40 rows, all machine-side) |
| Gate families | 5 (v0.7) | 5 (Integrity, Drift, Freshness, Strength, Authority) — the four v0.6 families judged what changed; none judged who may cause a landing, and folding that into Integrity would hide a security boundary under a quality label |
| Authority roles | 3 (v0.7) | 3 (signer, reviewer, pipeline) — the moment someone proposes `approver-level-2`, this row catches it |
| Diagrams in this playbook | 2 | 2 (§1 structural, §5.1 sequential) |
| v1 CLI commands | 4 *(amended from 3 in v0.6 — argued openly, per this table's own rule, to admit `spine init`)* | 4 (`init`, `new`, `index`, `check`) — signing, approving, reviewing, landing and upgrading are flags, not commands |
| A↔B adversarial rounds | 2 | 2, then human (the budget resets per reopen; `total_rounds=` is reported, not capped, because a reopen must never be refusable) |
| Gate overrides | 0 silent | break-glass only — always recorded in a git object, always retro'd, never before approval, never self-approved in team mode (§7.6) |

Complexity is allowed to grow only on the machine side — gates, queries, derived state, signed records. The moment it grows on the human side, this table catches it. "Are we over-complexifying?" stops being a vibe question someone must remember to ask. v0.7's human-side additions, in full: one key enrolment per person at onboarding; one hand-on-key action per sign-off (already required in v0.6, now concrete); one per conditional review; one per reopen or withdrawal; one per toolkit upgrade. Everything else is machine-side. Deliberately *not* added, and why: a two-hands-per-floor-change rule (doubles the human hands per protected change with no argument offered), mandatory hardware keys (a human-side cost; the residual is stated instead), and a lock ref to serialise landings (a queue service in miniature). v0.8 added nothing to the human side. v0.9 adds exactly two things, argued here as this table requires. One: a sentence in the `reason=` of an approval whose frozen tests import a branch-created module inside `expected` — the case no static rule can decide (§4.3) — and in v1 a human signs every approval already, so it is a sentence, not a gate. Two: turning `C-M4` on now takes a constitution PR and should carry an ADR, which is a deliberate cost, because accepting a residual ought to have a signature against it. Neither adds a mandatory gate; row one stays 1.

---

## 11. Vocabulary — the names every section shares

**Hash policy.** Git object ids (`<oid>`, in the repo's object format) for everything that is a git object: intent blob, frozen files, trees, commits. SHA-256 (`sha256:<hex>`) only for non-git artifacts: release artifact list (`dist_hash`), gate report, freeze digest, envelope digest, B's transcript.

**Roles and namespaces.** signer `spine-signoff@v1` · reviewer `spine-review@v1` · pipeline `spine-seal@v1`. Keyring `.spine/allowed_signers`. Solo mode = exactly one signoff key (`C-A1`), whose principal then holds all three namespaces.

**Signed statement.** One trailer line ending in `signer=<principal>` / `reviewer=<principal>`, plus `<Name>-Sig: <SSHSIG, one line>` over that line's exact bytes; `reason=` values are JSON string literals. Verify by hand: `ssh-keygen -Y verify -f .spine/allowed_signers -I alice@example.com -n spine-signoff@v1 -s <sig> < <line>`.

**Trailers.**

| Trailer | Where | Payload |
|---|---|---|
| `Spine-Event` | every event and landing commit | `signoff · approve · review · reopen · withdraw · upgrade · land · reseal` — `upgrade` is the signed event on `spine/upgrade-<v>` (rollback and uninstall are upgrades with `to=<A>` / `to=none`); lifecycle landings are `land` plus the copied `Spine-Upgrade`; tombstones are `withdraw` |
| `Spine-Intent` | every **event** commit on `refs/heads/intent/*`; every gated landing and every tombstone | `INT-042` or `BUG-051` (bare; graph id is `<repo>/INT-042`) — never required on implementation commits, whose membership comes from the landing range (§5.5); quick, reseal and toolkit lifecycle events, their landings and the reviews that accept them have no intent id, and take their identity from the seal's first field |
| `Spine-Envelope` / `Spine-Lane` / `Spine-Strategy` | landing | `1` · `gated \| quick` · `merge \| squash` |
| `Spine-Signoff` + `-Sig` | sign-off commit; copied into the gated landing and, where one exists, into a tombstone (§5.5) | `INT-042 blob=<oid> template=v2 constitution=v3 reopens=n [lease_override="…"] signer=<p>` |
| `Spine-Approve` + `-Sig` | approval commit; copied into the gated landing | `INT-042 intent=<oid> base=<sha> rounds=0..2 total_rounds=n reopens=n red=k/n freeze=sha256:<hex> [run=sha256:<hex>] [held=false] [reason="…"] signer=<p>` — `held=false` marks B still breaking at the cap; `reason=` is mandatory, and G13 refuses its absence, on `red=0/n`, `held=false`, or a closure tripwire — `run=` present ⇒ verifies under `spine-seal@v1` only; absent ⇒ `spine-review@v1` only |
| `Spine-Frozen` (repeated) | approval commit; gated landing in squash strategy | `<oid> <path>` (`git ls-tree` quoting) — the closure: tests, fixtures, snapshots, runner config |
| `Spine-Test` (repeated) | approval commit; gated landing in squash strategy | `<runner-native function id>` without parametrization suffix |
| `Spine-Approval` | gated landing | `<approval commit sha>` (∈ `M(L)` under merge) |
| `Spine-Review` + `-Sig` (repeatable) | review commit; copied into the landing — two required for a landing with no signer in team mode | `INT-042\|quick\|reseal class=tripwire\|protected\|break-glass head=<sha> tree=<oid> base=<sha> [intent=<oid>] report=sha256:<hex> wires=<(G<n>\|C-<rule>)[:path],…> reason="…" reviewer=<p>` — wires sorted; gates without a path use the bare id; `head=` is the content head `Hc` (§5.4); the first field is the seal's and `intent=` is present only when the landing has one; for `class=break-glass`, `wires=` lists the gates bypassed |
| `Spine-Reopen` + `-Sig` (repeatable) | the commit that changes the intent blob; copied into the landing | `INT-042 voids=sha256:<freeze digest>\|none reopens=n reason="…" signer=<p>` — `voids=` names the binding approval's freeze, `none` only when no approval exists; G13 refuses otherwise |
| `Spine-Withdraw` + `-Sig` | withdraw commit; copied into the tombstone | `INT-042 blob=<oid> [orphaned=<principal>] reason="…" signer=<p>` (`--protected`: signed under `spine-review@v1` by a reviewer ≠ the original signer, for an orphaned branch) |
| `Spine-Upgrade` + `-Sig` | upgrade event commit; copied into the landing | `from=<A> to=<B> manifest=<blob oid> forced=<paths> [from-manifest=<sha>] [since=<sha>] signer=<p>` — `from-manifest=` names the exact ancestor a rollback restores and is mandatory on one (§7.5) |
| `Spine-Gates` | landing | `G1=pass … G16=pass`, plus `C-<rule>=pass\|override` for constitution rules carrying `enforced_by` (§2.1) — every gate that ran, never G10 (it runs after the seal); entries read `=override` where a signed review accepted the wire or break-glass bypassed the gate; a tombstone lists the four that ran |
| `Spine-Supersedes` / `Spine-Reverts` | landing (optional) | `INT-017` · `<sha>` (hints; edges come from the header and from patch-id equality) |
| `Spine-Trust-Root-Prev` | a rotation root (solo only) | `<sha>` of the previous root |
| `Spine-Seal` + `-Sig` | landing; the last `Spine-*` line | `INT-042\|quick\|reseal base=<sha> head=<sha> tree=<oid> report=sha256:<hex> tool=<version>+sha256:<dist_hash> git=<major.minor> mode=solo\|team\|recovery profile=container\|uid\|none recon=inline\|scheduled:<n> [proved=<sha>] envelope=sha256:<hex over every Spine-* line above, in order, LF-joined> signer=<p>` — the first field is the landing's identity: the intent id for a gated landing or tombstone, `quick` for a quick-lane landing **and for every toolkit lifecycle landing** (upgrade, rollback, uninstall, re-init — they ride the quick lane, §6.7), `reseal` for a reseal; `head=` is the content head `Hc` (§5.4); `tree=` names `L`'s tree, so G9 checks it from `L` alone |

**Files and refs.** `intents/<ID>.md` — only on `refs/heads/intent/<ID>`, deleted by the landing, never on trunk. Toolkit upgrades ride `refs/heads/spine/upgrade-<version>`; quick-lane candidates are `refs/heads/quick/*`. `.spine/manifest.json` (lockfile; its frozen fields never change name, type, `owner` set, or the meaning of a `paths` key, so any binary can judge any manifest, §6.7) · `.spine/allowed_signers` (keyring) · `.spine/ci.sh` (every provider — the collector's entry point, executed from trunk, §7.4 rule 0) · `.spine/cache/` (gitignored: `graph.sqlite`, `staging/`, and `results/<T>.jsonl` — the collector's output and the untrusted job's only artifact: first line `tree=<oid> base=<sha> tool=<version>+sha256:<dist_hash> keys_visible=false ids=<n>`, then the id set collected on `B`, then one record per runner-native id). Managed regions `AGENTS.md#spine`, `.gitignore#spine`, `.gitattributes#spine`. CI: `.spine/ci.sh` always, plus `.github/workflows/spine.yml` | a `.gitlab-ci.yml` snippet | a definition outside the repository. Not in the repo: the pre-receive hook, `refs/notes/spine` (optional report copy, never a source), git config `spine.trustRoot`, the CI variable `SPINE_TRUST_ROOT`. No `.spine/prompts/`, no `prepare-commit-msg` hook.

**States.** `draft → awaiting-sign-off → signed → tests-drafted → tests-approved → checked† → merged`; side states `approval-review` (from tests-drafted), `landing-review` (from tests-approved, base-moved† or quick-candidate) and `protected-review` (from tests-approved or quick-candidate), `needs-rebase`, `base-moved†`, `starved`, `reconstruction-failed` (§5.4 step 5), `escalated` (quick-candidate needing an intent); quick lane `quick-candidate → checked† → merged`; exit `withdrawn`; post-landing `reverted`, `superseded`, `orphan`, `unattested`, `resealed`. `orphan` is entered outside the transition table, by a push around the pipeline (§5.5); keyring `vN → vN+1` and `chain broken` (§7.5) are keyring states, not intent states. † = runner-local, collapses to `tests-approved` in any clone. *In flight* = `awaiting-sign-off` … `checked†`; leases are held from `signed` onward. A reopen from any in-flight state returns to `awaiting-sign-off`.

**Gates.** Integrity G1 Coverage · G5 Orphans · G8 Freeze · G9 Ledger · G10 Reconstruction — Drift G2 Containment · G7 Interference — Freshness G3 Staleness · G4 Currency · G11 Base currency — Strength G6 Mutation · G12 Red at approval — Authority G13 Signers · G14 Floor · G15 Tool · G16 Scaffold. Warn-before-block applies to G2, G3 and G7's *soft* clause only: a `forbidden` hit, and a hard lease over another intent's forbidden or frozen set, block in every mode. Break-glass may bypass G1, G2, G3, G4, G6, G7, G8, G12 only.

**CLI (four commands).**
- `spine init [--ci github|gitlab|generic] [--strategy merge|squash] [--trunk <name>] [--pipeline-key <pub>] [--hooks] [--trust-root <sha>] [--rotate-trust-root] [--dry-run] [--status] [--merge] [--adopt <path|file#region>] [--force <path>] [--abort] [--rollback [<sha>]] [--uninstall]` — first run = bootstrap + trust root; later runs = upgrade on `spine/upgrade-<version>`.
- `spine new [--change|--bug] [--from <quick-branch>]` · `spine new --sign <id> [--override-lease "<reason>"]` · `spine new --reopen <id> --reason "…"` · `spine new --withdraw <id> --reason "…" [--protected]`
- `spine index [--fresh] [--dump]`
- Roadmap 3+, not v1: `spine context <id> [--role A|B]` · `spine stats` · `spine review <id>` · `spine eval`
- `spine check [--ci [--collect]] [--constitution] [--authority] [--approve <id>] [--review [<id> | --quick <branch> | --reseal]] [--land [<id> | --quick <branch> | --reseal] [--print] [--dry-run]] [--reconstruct] [--verify <sha>] [--break-glass "<reason>"] [--pre-receive]` — `<id>` omitted for upgrade landings; `--collect` is the untrusted job's half, running the candidate's tests and writing the result file (§7.4 rule 3): it holds no key, signs nothing, and reads its pin and policy from `origin/<trunk>` rather than the checkout, so the skew table (§6.7) never fires on a candidate's manifest. `--collect`, `--approve` (which collects ids and runs the frozen ids for G12) and `--constitution` (which runs the repo's own `enforced_by` probes) are the flags that execute repository code; none of them runs in the trusted stage.
- Environment: `SPINE_AGENT=1` marks an agent session; every signing flag (`--sign`, `--reopen`, `--withdraw`, `--approve`, `--review`, `--break-glass`, and `--land` outside `--ci`) refuses under it or without a TTY (§7.1).

**Git requirements.** git ≥ 2.38 (`merge-tree --write-tree`), OpenSSH ≥ 8.2 (`ssh-keygen -Y`), a remote honouring `--force-with-lease` and `--atomic`, full history plus `refs/heads/intent/*` fetched in the trusted CI job, non-fast-forward pushes denied on trunk and on `refs/heads/intent/*` with intent-branch deletion restricted to the pipeline principal.

---

## 12. Changes in v0.9

v0.9 answers an adversarial design review of v0.8 (`docs/reviews/2026-08-26-codex-adversarial-review-v0.8.md`; verdict: needs-attention / no-ship). Its findings share one shape, and v0.9 answers that shape rather than patching five symptoms: **a guarantee that any supported mode can switch off is not a guarantee.** Auto-merge could be enabled on a channel nothing attested; reconstruction could be deferred indefinitely; a rollback could be claimed by a manifest that merely resembled an ancestor; a provider could create the commit nobody verified. Each finding, and where it is closed:

| Finding | Severity | What was wrong | Closed by |
|---|---|---|---|
| `C-M4` could be enabled although candidate code can forge every test outcome | critical | The collector protects the result file, not the runner's stream; a team following the documented enablement path turned auto-merge on and got a tripwire review that was handed the same forged report | Auto-merge is a **capability computed per run**, not a setting (§7.4 rule 5): the collector measures and names its isolation profile (`container`, `uid`, `none`), and `C-M4` evaluates `on` only under an attested profile, rule 0's key-visibility assertion, `C-M5: inline`, and a trusted-stage CAS. Any one missing and the run is `off` with a **`G1` wire naming which** — so the mandatory review is aimed at the test outcomes and why they are unattested. `profile=` is sealed into every landing, so the ledger records the strength of each landing's evidence forever. §9 no longer presents auto-merge as a milestone the wires unlock: it is an acceptance of a named residual, a constitution PR, and an ADR |
| The base-anchored closure still left greenfield intents an unfrozen oracle | high | A module created on the branch inside `expected` was excluded by construction, so A could write an oracle there, import it only from the frozen tests, and rewrite it during implementation | Nothing static separates a stub from an oracle before an implementation exists, so v0.9 stops pretending either way: a branch-created module inside `expected` that a frozen test imports is a **closure tripwire** (§4.3). `--approve` lists them and the signing human names each code-under-test or test-only in `reason=`, which §11 already makes mandatory and G13 already refuses to omit. Test-only ones join the frozen closure. In v1 a human signs every approval, so the cost is a sentence |
| The structural rollback predicate permitted arbitrary recovery upgrades | high | Matching an ancestor's `cli` object constrained one field and nothing else, so two reviewers could craft a manifest with an ancestral `cli` and arbitrary everything else, then move protected paths to match — reopening the very bypass v0.8 claimed to close | A rollback is now a **deterministic restoration of one named ancestor**: `Spine-Upgrade` carries `from-manifest=<sha>`, and G16's restoration rule requires every frozen field and every `files[]` record in `T`'s manifest to equal that ancestor's but for a monotone `paths.*` union, and every managed path in the diff to equal its blob there or the deletion that manifest prescribes (§7.5, G15, G16). `--rollback` satisfies it by construction; a forward or malicious lifecycle change cannot, whatever its `to=` claims — and no gate orders two version strings |
| Scheduled G10 made the pre-push reconstruction guarantee unenforceable | high | `recon=scheduled` recorded no window, deadline, result or disposition; unproved landings could accumulate indefinitely behind valid seals | `scheduled:<n>` is now a **degraded mode with teeth**, and `<n>` counts landings rather than days, because the chain is this design's only clock. It forces `C-M4` off and needs no separate job: every landing proves the oldest one not yet proved and seals `proved=<sha>`, so the backlog is self-limiting, and G9 refuses any landing with more than `<n>` unproved landings above `proved=`. A failed proof is a `reconstruction-failed` report that no later `proved=` may pass, so reconstruction is either current or the repo stops landing until it is |
| Provider-made merges validated a synthetic commit, not the object reaching trunk | high | Configuration (b) performs no CAS; G10 proved the `L` that `--print` built while the provider created a different commit, and the body re-read is a pre-merge check, never an atomic guard on the tip | **(b) is stated as not an auto-merge configuration** (§5.4): precondition 4 fails on every landing there, so each takes a review, and the object that actually reached trunk is verified by the next run's G9 and G10 — loudly `unattested` when it differs. (b) is the on-ramp for teams whose provider owns the merge button; (a) is what the guarantees are written for |

One recommendation is **refused, with its reason in the text** (§7.4): keeping auto-merge mechanically unavailable for in-process runners. Every runner a real repository uses is in-process, so that rule does not qualify the headline — it deletes the product for everyone while leaving the same residual for anyone who ships around it. v0.9 does the two things actually available instead: refuse `on` unless the channel is attested, and record the profile in the seal so no landing can later claim strength it never had.

**The lineage.** v0.7 answered the v0.6 review, which found all three guarantees unenforceable: the intent envelope and `implements = M(L)` replaced PR descriptions (§5.5, §6); the signed approval record froze the test closure by blob id (§4.3); roles became SSH namespaces with a protected floor and a trusted job running trunk's definition (§7); the serialized merge protocol made `push --atomic --force-with-lease` the compare-and-swap (§5.4); and the manifest gave scaffolding a lifecycle (§6.7). v0.8 answered the v0.7 review: trunk's collector replaced self-written result files, G10 moved ahead of the CAS, the freeze closure was anchored at the approval's `base=`, the `manifest_version` deadlock was resolved by making the frozen fields' invariants explicit, and §11's trailer grammar stopped rejecting the lifecycle commits §6.7 requires. Each version was also verified internally before freezing — one skeptic per change plus consistency, voice and fresh-implementer critics — and each time that pass rewrote closures rather than confirming them: v0.8's own collector was found to have moved the forgery rather than closed it, and its G10 would have made every remote-less landing unlandable for ever.

**Residual risks, stated rather than hidden** — in the order they should worry a team about to turn `C-M4` on:

- **A candidate's runner can lie about its own results** (§7.4). An attested profile closes forged files, relabelled trees and hidden ids; nothing closes a runner sharing a process with the code it tests, and nothing in this design ever will. This is why `C-M4 = on` is an acceptance with an ADR against it rather than a setting.
- **A human can classify a branch-created module wrongly** (§4.3). The tripwire guarantees the question is asked and signed for, not that it is answered correctly; `spine stats` counts the exclusions, and a rising count is the signal.
- **An oracle the implementation also imports** can still be weakened without changing a frozen blob (§4.3) — Agent B's job, G6's signal, nobody's gate.
- **A laptop or Windows agent with a live signing agent** can satisfy the TTY check (§7.1); a hardware key or `ssh-add -c` closes it, and `spine init --status` names every key that is not hardware-backed.
- **Sybil solo, and signer ≠ diff-author** (§7.2): one human holding two keys is a statistical finding — reviewer diversity in `spine stats` — never a mechanical one.
- **Trunk rewritten below the trust root** (§7.4) defeats every ledger and is visible only as pin disagreement across machines; denying non-fast-forward pushes is the non-optional supplement.
- **Weaken-via-trunk of a shared fixture** (§4.3): a fixture legitimately weakened by a protected-reviewed landing is accepted by every intent that froze it, once those ids rerun.
- **Provider configuration (b)** (§5.4) verifies the landed object one run late, and cannot auto-merge at all.
- **`recon=scheduled`** (§6.3) proves reconstruction over a window rather than at every push, and cannot auto-merge; the window is the size of the exposure, and G9 makes it finite.
- **Optimistic CAS has no progress guarantee** (§5.4): a fast-moving trunk starves landings, and the answer is a provider queue as runner, never a bigger `C-M3`.

---

*This playbook is itself governed by its own rules: keep it short, change it by PR, and delete anything that a machine could enforce instead.*
