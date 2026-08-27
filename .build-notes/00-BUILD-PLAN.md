# 00 — Build plan: reconciliation of the thirteen requirement sheets

Reconciles `.build-notes/01`…`13` against `PLAYBOOK.md` v0.19 and `docs/spec/*.md`.
Every adjudication below was checked against the corpus text, not against the sheets alone.

Citation convention (the corpus's own): `PB` = `PLAYBOOK.md`; `MF` = `manifest.md`; `CI` = `ci.md`;
`RF` = `result-file.md`; `GR` = `gate-report.md`; `CN` = `constitution.md`; `TM` = `templates.md`;
`ID` = `intent-doc.md`; `DM` = `dump.md`; `EV` = `envelope-vectors.md`; `IR` = `import-resolver.md`.

**Precedence rule in force throughout** (`docs/spec/README.md`): where prose in a spec and **PB §11
(Vocabulary)** disagree, **§11 wins** and the disagreement is a defect in one of them. Everywhere else
the spec is normative and resolves PB's ambiguity.

---

## 0. Where the tree already stands

| Crate | State | Evidence |
|---|---|---|
| `spine-canon` | **built** — `value` / `jcs` / `esc` / `digest` / `parse`, 25 tests green | reproduces GR §8.3's `sha256:a594772c…` (`jcs.rs:168`) |
| `spine-manifest` | empty stub | — |
| `spine-collect` | empty stub | — |
| `spine-isolate` | empty stub | — |
| `spine-graph` | empty stub | — |
| `spine-resolve` | empty stub | — |
| `spine-cli` | `main.rs`, 4 lines | — |

Step 1 of the build order below is therefore *extension* of an existing crate, not a green field.
`float_arithmetic = "deny"` and `mutable_key_type = "deny"` are already set in the workspace lints,
which is the right shape: the determinism rules of GR §7 / MF §7 / DM §10 are enforced by the type
system and the lint table wherever they can be.

---

## 1. What actually blocks code

The distinction that matters: an item is **blocking** only if a conforming implementation cannot be
written without an owner decision — not merely if it is undecided. An undecided item that has a
fail-closed reading, or that reduces to a constant the binary can carry, is a **typed hole**, not a
blocker.

### 1.1 Blocking — `spine init`

**B1 · Nothing states what `init` writes into `repo`.**
MF §3.1 makes `repo` required with grammar `^[A-Za-z0-9._-]+$`, 1…64 bytes, and refusal
`repo-out-of-grammar`. DM §5.2 builds **every node id** from it and G10 diffs those dumps byte for
byte before every landing. PB §11's `spine init` signature has no `--repo` flag and no document names
a source. *Recommended default:* the basename of `git rev-parse --show-toplevel`, refused with
`repo-out-of-grammar` and an instruction to pass an (as yet unspecified) override when it does not
match the grammar. **A wrong choice here moves every node id in every dump forever**, so it deserves
an explicit ruling rather than an inference.

**B2 · `--ci` has no default, and `params.ci` is required.**
PB §11 and CI §3.1 fix the domain `github|gitlab|generic`; MF §3.3 marks `params.ci` `always`
present. `--langs` has an explicit detect-or-refuse rule and `--ci` has neither. *Recommended
default:* refuse, exactly as `--langs` refuses — "the way it refuses an ambiguous signing key"
(PB §11). Detecting a provider from the tree is the tempting alternative and is wrong: a repository
carrying a stale `.gitlab-ci.yml` would silently pick `gitlab` and permanently retire auto-merge
precondition 2 (CI §8.1).

**B3 · `--trunk` has no default either.**
Same shape as B2; `params.trunk` is frozen and `always` present. *Recommended default:* the branch
`git symbolic-ref --short HEAD` names, refused on a detached HEAD. Note CI §7.1's constraint on top:
`init --ci github` refuses when `params.trunk` ≠ the provider's default branch.

**B4 · `--strategy` has no conforming target at all.**
PB §6.7 lists it among flags that "update `params`", but MF §3.3 defines `params` as exactly
`{trunk, isolation, ci, langs, timeout}` and MF §3.1 freezes all five — PB §11's frozen twelve is the
same list, so **§11 wins and there is no `params.strategy`**. Merge strategy is `C-M1:
merge.strategy` in the constitution (PB §5.5, CN §6.1). But CN §6.2 fixes the `constitution@1` block
byte-for-byte with **only** `<repo>` and the `C-T1`/`C-T2` values varying, and CN §6.4 / PB §6.7 make
the constitution `user-owned` and never rewritten after the seed. So `--strategy` can neither write
`params` (no member) nor write the constitution at bootstrap (a sixth varying span CN §6.2 does not
admit) nor write it on a re-run (spine never rewrites a `user-owned` file).
*Recommended default:* **OWNER DECISION REQUIRED.** The cheapest resolution is to accept
`--strategy` at bootstrap only, as a documented sixth substitution into the `C-M1` line, and refuse
it on a re-run; the alternative is to drop the flag from v1 and let the human edit `C-M1` under the
protected review the constitution already takes.

**B5 · Uninstall over a modified `spine-owned` path is unimplementable as written.**
PB §6.7: uninstall "removes **clean** `spine-owned` paths", implying a modified one survives.
MF §6.8, **outright**: "every `spine-owned` path listed in `M_B` is absent from `T`"
(`uninstall-path-remains`). A repository with one hand-edited `spine-owned` path therefore has an
uninstall the tool performs and the gate refuses, and no `--force` is documented for it.
*Recommended default:* delete **every** `spine-owned` path listed in `M_B` regardless of blob, and
report each deleted-but-modified path loudly by name — the only implementation whose uninstall can
land, and the human's bytes stay reachable in git history.

**B6 · `<run>` in `.spine/cache/staging/<run>/` has no grammar, and staging cardinality is unstated.**
Interrupted-state detection (PB §6.7's three states) turns on locating *the* staging run and reading
`staging/<run>/manifest.json`; with several coexisting runs, states 1 and 2 are not discriminable.
MF §7 rule 1 bars a wall clock from these artifacts. *Recommended default:* at most one staging
directory may exist; `<run>` is a 32-hex random nonce (not a clock, not a digest of anything a gate
reads, and staging is gitignored and covered by no digest); a second `init` that finds one treats it
as the interrupted case and never creates a second.

**B7 · The plan's `delete` token is a write decision and is unspecified.**
MF §12 explicitly declines the `create · update · delete · skip` rules and only the `REFUSE` triggers
are normative — but *which paths a provider change removes* is a write, not a display choice
(a `--ci github → gitlab` re-run retires `.github/workflows/spine-*.yml`). *Recommended default:*
delete a `files[]` record's path iff its `owner` is `spine-owned` and the new render set does not name
it; leave every `user-owned` and `user-modified` path in place and report it. `create`/`update`/`skip`
are display only and may be fixed by the implementation (see §2, non-blocking).

### 1.2 Blocking — the collector and its isolation boundary

Nothing. RF §14 OPEN-9 is **closed** (2026-08-27) and RF's own words are "What remains open here is
nothing." The two adjacent OPENs (`params.ci` monotonicity, G6's channel) move no byte the collector
writes — RF §10's *The same run, on `--ci generic`* proves the first by construction, and v1's
collector has no G6 channel by design.

The one thing to record rather than solve: `SPINE_ALLOWED_HOSTS`'s *which hosts* half is
**declared and not enforced, permanently and by design** (RF §12, CI §5.6). M1 enforces *when*
(loopback-only runners, P4-tested). **Do not build a hostname filter or a lockfile hash checker into
the collector** — RF §12 forbids both by name, and a pre-v0.19 clone of PB §7.1 will invite it.

### 1.3 Blocking — `spine index`

**B8 · `supersedes` and `superseded_by` have no stated direction.**
DM §13.4 claims §5.3 "fixes all fifteen"; §5.3's paragraph names twelve and is silent on
`supersedes`, `superseded_by` and `exercises`. `exercises` is never emitted in v1 so it costs
nothing; the other two are emitted by any repository with a `Spine-Supersedes` trailer, and a wrong
direction is a permanent G10 divergence between two implementations. PB §6.6's "the indexer emits
`superseded_by`" is the only hint. *Recommended default:* `supersedes`: superseding intent →
superseded intent; `superseded_by`: superseded intent → superseding intent; **emit both**, which is
the only reading under which both names mean what they say and PB §6.6's "archaeology queries return
the current truth first" is answerable.

**B9 · `changeset.tool_version`'s split rule is stated nowhere.**
PB §11's seal carries `tool=<version>+sha256:<dist_hash>`; DM §12.2's vector records
`"tool_version":"1.4.0"` — the version half only, the digest dropped. No document states the split.
*Recommended default:* split at the **last** occurrence of the literal `+sha256:` (RF §13 R14's rule,
unambiguous because `<dist_hash>` is exactly 64 lowercase hex), take the left half, and carry the
digest in no attr. The vector is the only evidence and an implementation must match it.

**B10 · The store's shipped-floor `protects` edges have no legal `from_id`** (DM §15 OPEN-1).
PB §6.2 derives `protects` partly from "the floor list inside the pinned release" and gives no node
kind that could be its `from_id`; the nine kinds are all repository facts. The **dump** is unaffected
(DM §8.5 excludes the edges either way), but the **store** needs a shape.
*Recommended default:* DM's own recommendation (c) — emit no shipped-floor `protects` edge at all
and let G14 read the release constant `F0` directly. Option (b) adds a `release` node kind and moves
`PRAGMA user_version` to 8, which changes PB §6.2 and is the owner's.

### 1.4 Blocking — the release, not the code

**B11 · `dist_base`'s host value and the three GitHub Action commit pins** (CI §18 OPEN-1, OPEN-7).
Values only; the mechanism around them is fully normative (40 lowercase hex, never a tag, `https://`
with no trailing `/`, the five-row substitution table, the `@@`/`PIN_` byte scan). Until they are
chosen **no release manifest can be frozen, so every build is a development build and `spine init`
refuses every plan row with `no-release-manifest`** — CI §3.4's heading is literally "A development
build refuses `spine init`."

This does **not** block writing `init`: the refusal *is* the specified behaviour and is testable. It
blocks (a) cutting a release, and (b) publishing any conforming render of `.spine/ci.sh` or the two
workflows — which is exactly why MF §8.1's workflow and `ci.sh` bytes are declared stand-ins.

---

## 2. Undecided but not blocking — typed holes and config values

| Item | Why it does not block | What to do |
|---|---|---|
| `init` exit codes (OPEN-D) — only `--dry-run`'s 0/2 is fixed | No gate reads them; nothing is digested | Fix a table in `spine-cli`, document it, keep 0/2 for `--dry-run` |
| `init` refusal message strings (OPEN-E) — only `"markers removed"` and `"interrupted by <version>: run that version, or --abort"` are fixed | Diagnostics, covered by no digest | Emit the two fixed strings verbatim; choose the rest |
| `create` / `update` / `skip` plan tokens (OPEN-C) | Display; the writes are decided by the ownership rules (PB §6.7 step 3), not the token | Derive: `create` where the path is absent from HEAD, `update` where a rewrite is permitted and the render differs, `skip` otherwise and for every `user-owned` record |
| Nothing writes `params.timeout` (OPEN-B) | Absent means `1800` and the collector enforces it; a collector enforcing no deadline is non-conformant whatever the manifest says (RF §7.1) | `init` never writes the member; it is set by hand under a protected review |
| The constitution interview's place (OPEN-G) | PB §9 roadmap step 0; v1's `init` seeds the constitution and can ship without the interview | Defer; do not let it into the atomic-apply order |
| `init` under `SPINE_AGENT=1` / no TTY (OPEN-H) | PB §7.1's general clause already covers it; PB §11's enumeration omits it | **Fail closed**: refuse whenever `init` will sign (the trust root, the `Spine-Upgrade` line). File as a PB §7.1/§11 defect |
| `C-A1` vs the key count (CN §16 OPEN-9) | MF §4.5 implements the key count and PB §11 agrees | Build against the count; route **every** read of `mode` through one function so option (c) is a three-line change |
| `params.ci` monotonicity (filed three times) | No G16 check exists in v1 | Implement no check; leave the token space free |
| `.spine/allowed_signers` canonical form (MF §13 OPEN-2) | The file is `user-owned` and lint-only today | Build against (a), the status quo |
| Unknown `templates` key (MF §13 OPEN-4) | Silent today; the release-set reading makes an unnamed key the ordinary case | Silent |
| `C-A2` bracket expressions (MF §13 OPEN-3) | G14's concern, not init's / index's | Narrow refusal (`c-a2-bracket-case`) |
| `spine check --reconstruct` — PB §11 lists the flag, nothing defines it | Outside the first three steps | Note it; route to whichever sheet owns the CLI surface |
| `tree: unverifiable(git-version)` (DM §15 OPEN-2) | Fully specified as it stands | Implement as specified; add the git version to `spine stats` later |
| No size / entry / principal-length bound on the keyring | Not filed as OPEN by any document; a genuine gap | Bound defensively in the parser, claim no spec authority for the number |
| Non-UTF-8 **paths** in a result file | RF §4.3 has no `esc`; RF §7.2 fixes only the non-UTF-8 **id** case | Invent no encoding. Record the residual |

---

## 3. Contradictions, with adjudication

Ordered by how much a wrong ruling costs.

**C1 · Signerless review count: "at least two" (PB §11) vs "two" (MF §4.8.4 check 9 / §4.8.7).**
PB §11: "carries **at least two** distinct `class=protected` reviews in team mode … **a floor and
never an exact count**, since a third reviewer signing a contentious reseal is diligence and must not
be the thing that refuses the landing." MF §4.8.4 check 9 says "holds **two**" and MF §4.8.7 encodes
`if n ≠ (2 if mode = "team" else 1)` — verified at `manifest.md:500` and `:596`. Read as equality, MF
refuses the three-reviewer reseal PB §11 protects. **PB §11 wins.** Implement `n < (2 if team else
1)`. MF §4.8.7's `≠` is a defect not filed in MF §10. *Certain.*

**C2 · G16's outright set: MF §6.2 includes check 12b, GR §5.6.1's table does not.**
GR §5.6.1's G16 row reads "checks 1–8, 10, 11, 16, 17, and every clause of the rollback restoration
rule" — 12b absent. MF §6.2 check 12b marks `isolation-unsupported` **outright** and argues at length
that it cannot be coverable ("no protected reviewer can make a mechanism exist: a dischargeable wire
would let two humans sign a repository into the brick"). GR's own *Fixed by* column cites
`manifest.md` §6.2 as the owner of G16's check list. **MF wins; 12b is outright.** GR §5.6.1's row is
stale. *Certain.*

**C3 · The rollback target: PB §6.7's heuristic vs MF §6.7 step 2's gate rule.**
PB: `U` is "the first-parent commit that last touched the manifest". MF: `U` is "the newest
first-parent **landing** at or below `B` whose envelope carries a copied `Spine-Upgrade`", located by
the ledger. MF resolves it in terms: "where they disagree, the gate wins and the tool refuses."
**Implement MF's rule in the gate and let the tool's default refuse when it disagrees.** *Certain.*

**C4 · What the restoration rule compares: PB §6.3/§7.5 vs MF §6.7 step 3.**
PB: "every frozen field and `files[]` record in `T`'s manifest equals that ancestor's but for
`paths.*`". MF: `eq(M_T minus paths, A minus paths)` — canonical-byte equality of the **whole**
manifest. MF §9 R14: the literal reading "would let a rollback silently lower `resign`, drop a
`templates` key, or rename `repo` — the last of which changes every node id in the graph."
**Implement MF's stronger rule**; it is what `--rollback` produces by construction. *Certain.*

**C5 · `.spine/ci.sh` has two correct published digests that must never be compared.**
CI §5.3 publishes the **unsubstituted** bytes: 319 lines, `git hash-object`
`131f13fb0312162579605999d3f9f4e90098c74c`, `sha256:d6bcf50cf675614033aaef61df104aad253d30c4accc756719599ad5bd41060b`.
MF §8.1 publishes a 234-byte **stand-in render**, blob `dc1893727069b1c188505544ecf4174d48a13bdb`,
which is what MF §8.3's `files[]` record carries. Both are right. **An implementer building MF §8.3's
manifest against CI §5.3's real `ci.sh` will not reproduce `cb4cd49034bbe25f76573c40d6711b2c33f9136f`.**
Use MF §8.1's stand-in when reproducing the manifest vector. *Certain.*

**C6 · PB §6.7's own manifest example is not what `init` writes.**
It marks both GitHub workflow rows `user-modified` with a `base`; CI §3.1 writes both `spine-owned`.
MF §10 D13: "The example depicts a post-`--merge` repository without saying so, and a reader
implementing from it writes the wrong class on first init." **CI §3.1 is normative for `init`.**
*Certain.* Consequence for testing: MF §8.3's published manifest is **not** an `init` output and its
blob id is not an `init` acceptance test.

**C7 · `init` is TTY-only by PB §7.1's general clause and absent from its own enumeration.**
PB §7.1: "any invocation that produces a `-Sig` line with a key that is not the `--ci` pipeline
secret … is TTY-only and refuses under `SPINE_AGENT=1`." `init` signs the trust root (PB §7.5) and
the `Spine-Upgrade` line (PB §6.7 step 5), yet the closed lists in PB §7.1 and PB §11 *Environment*
omit it. **Fail-closed: treat `init` as TTY-only and `SPINE_AGENT`-refusing whenever it will sign.**
*Likely* — the general clause plainly reaches it, but the omission is unfiled.

**C8 · `init --abort` is offered by a message and is not on the skew exemption list.**
PB §6.7's skew table exempts only `--status`, `--rollback`, `--uninstall`; the interrupted-upgrade
message offers "run that version, or `--abort`". In interrupted **state 3** the manifest already pins
the new version, so an older binary is refused — **including its `--abort`**. The offered exit is
unreachable in one of the three states. *Certain* as a reading; unfiled in PB. Recommended: add
`--abort` to the exemption list and file the defect.

**C9 · The frozen twelve exclude three fields the corpus depends on** (MF §10 D1, OPEN).
`repo` (DM §5.2 builds every node id), `templates` (TM §7.1 on every `spine new`), `resign` (G4's
floor) sit outside PB §11's list, which permits a binary to treat them as opaque. MF §3.8 implements
exactly twelve "because §11 wins" and files the defect rather than widening. **Do not implement a
thirteenth frozen field.** *Certain.*

**C10 · `params.langs` is defined wrongly in PB** (MF §10 D2, OPEN).
PB §6.7: "the languages this repository's harness is written in" — which makes every Python edge
vanish for a TypeScript harness over Python code. MF's fix: "this repository's harness **and the code
it tests**." `init`'s own detection probes for the *code*'s marker files (PB §11), which agrees with
MF and not with PB's sentence. **Implement MF's reading.** *Certain.*

**C11 · `keys_visible` domain: PB §11 spells the literal `false`, RF §4.2 gives `true | false`.**
RF §13 R10 concedes this "**widens** §11's grammar rather than resolving an ambiguity in it, and is
reported as a §11 defect". But RF §7.4 **requires** `true` on the solo path, so a literal reading of
§11 makes §5.4's whole *Solo developers* paragraph unimplementable. **Implement the two-value
domain**; flag it as an unlanded §11 defect. *Certain.*

**C12 · M1's host prerequisites: PB §12's change log says four, everything else says five.**
PB §12 line ~1117 says "four host prerequisites"; PB §12 line ~1129 says "**The container
prerequisite stack stays at five** — user namespaces, an identity source, an overlay root, a
traversable filesystem, a network namespace"; RF §7.1 prints a five-row table. **Five.** The "four"
is a stale count from before the network namespace became prerequisite 5, and it is change-log prose
rather than §11 Vocabulary, so RF §7.1 is normative. *Certain.*

**C13 · The probe's test count: PB §12 says three, PB §7.4 rule 3 and RF §7.1 say four.**
Same shape as C12 — P4 (egress) joined on 2026-08-27 (RF §13 R34). **Four: P1 ∧ P2 ∧ P3 ∧ P4.**
*Certain.*

**C14 · `profile=container` and container runtimes.**
PB §7.4 rule 3's bullet says "the runner ran inside a container the collector created", which invites
an OCI runtime; RF §7.1 says "No image is pulled and none is named", and RF §12 leaves the syscall,
helper or runtime **deliberately unspecified**. PB's own following paragraph defers the mechanism to
RF. **`container` is the profile *name*, not a runtime requirement.** *Certain.*

**C15 · PB §7.1's untrusted-stage Network cell is unconditional; `profile=none` is an ordinary
outcome.** PB §7.1: "none for anything the candidate runs: every runner invocation is spawned in a
network namespace holding only loopback." RF §7.1 disposition 2 and §7.4 make `profile=none` a normal
result that "runs the suite unisolated". RF §9's *Egress* row resolves it: "Under `none` no boundary
is attempted and this row says nothing." **PB §7.1's cell describes what `container` enforces.**
*Certain.*

**C16 · `--collect`'s nesting: closed in this clone.**
RF §13 R21 reports PB §11 as spelling `spine check [--ci [--collect]]`, under which the solo path has
no legal invocation. **PB v0.19 as it stands reads `spine check [--ci] [--collect] …` and states
`--collect` is "independent of `--ci`"** — verified in §11. The defect is closed here and live only
in a pre-v0.19 clone.

**C17 · `intent.status` domain: PB §11's States list vs DM §7.3.**
PB §11 lists `reverted`, `superseded`, `orphan`, `unattested`, `resealed` together as post-landing
states; PB §6.2's schema puts `unattested` and `resealed` in the **changeset** attrs and none of the
three in the intent's. DM §7.3 caps `intent.status` at `merged | withdrawn | reverted | superseded`.
PB §11 nominally wins — but §11's list is of *lifecycle states*, not an assignment of attrs to node
kinds, and PB §6.2 is unambiguous about where the two flags live. **Implement DM's four-value
domain.** *Likely* — DM files no defect for it, so it is an unfiled disagreement worth an owner
line.

**C18 · `object_format`'s source: GR/PB take it from the manifest, DM from the repository.**
GR §5 and PB §6.7 read it from the manifest at `base`; DM §3.1/§13.11 reads the indexed repository's
`extensions.objectFormat`, defaulting to `sha1`. Deliberate and documented — "they agree in every
conforming repository; where they disagree the repository is broken and the disagreement is G15's or
G16's finding". **Implement both readings at their own sites.** *Certain.*

**C19 · `cli.version`'s grammar: RF §8 R14 vs MF §3.2.**
RF says it is "unconstrained beyond the header's no-space rule"; MF §3.2 constrains it to
`^[0-9A-Za-z._+-]{1,64}$` and bars the four bytes `none` (the sentinel `Spine-Upgrade` needs).
MF §9 R17 resolves in MF's favour, adopting CI §5.5's grammar. **MF's.** *Certain.*

**C20 · The marker syntax and the region key.**
PB §6.7 gives the marker syntax once, in HTML, for three regions — two of which are files where an
HTML comment is not a comment. MF §3.7's two-syntax table is the implementation (`<!-- spine:begin
agents-block@<n> -->` for `AGENTS.md`, `# spine:begin gitignore@<n>` and `# spine:begin
gitattributes@<n>` for the other two). Separately, MF §3.7 records its own earlier defect (R21): the
`#` suffix is a **region key** and is **never** looked up in `templates`; the record's own `template`
member supplies the name check 9 indexes by. All three v1 regions are keyed `spine` while their
templates are `agents-block`, `gitignore`, `gitattributes`. **Indexing by the key asks for
`templates["spine"]` and leaves `region-version-mismatch` undecidable for every region v1 ships.**
*Certain.*

**C21 · `forced=` and `manifest=` have no grammar in PB** (MF §10 D9, D10, OPEN).
PB §11 writes `forced=<paths>` — a list inside a space-separated signed payload with no separator,
quoting or escaping rule — and `manifest=<blob oid>` with no value for an uninstall. MF §6.4 fixes
both: `tok(path)` comma-joined with **the empty list spelled as the empty value** (`forced=
signer=…`, never `none`, because `tok("none")` is a legal path), and `manifest=none` under `to=none`.
**MF's.** *Certain.*

**C22 · `resign[t] ≤ templates[t]`: asserted of G16 in PB §6.7, absent from PB §6.3's G16 row**
(MF §10 D8, OPEN), and the mirror case (lowering `resign`) is unaddressed anywhere in PB.
MF §6.2 checks 11 (outright, `resign-floor-above-current`) and 11b (coverable, `resign-lowered`,
skipped under `from=none`) are the implementation. Separately PB §6.7 says "For every template",
which reads as all twelve; MF §3.6 and TM §7.2 make `resign` **intent-only** with any other key
`resign-key-unknown`. **Three variants only.** *Certain.*

**C23 · `templates` was an eight-key map naming a single `ci-github`; it is now twelve.**
MF §10 D3 / CI §15 D1, both **CLOSED**. Twelve keys with the GitHub pair split into
`ci-github-collect` and `ci-github-land` is current, and `ci-generic` names the **provider-independent
shell** `.spine/ci.sh`, rendered for every `params.ci` value — not the `generic` provider (CI §15
D16). Reading it the other way produces a GitHub repository in which nothing executes the collector.
*Certain.*

**C24 · The two G2 rulings a wrong reading of PB would produce.**
PB §5.2 still lists "No changes to schema, auth, or public API surface" as a green-pipeline
condition; PB §6.3's G2 row says that wire is **withdrawn** ("a wire nobody can compute is a wire
that never fires"). And PB §6.3 says the diff-size sub-check is recorded as `G2:<path>` while GR §6.3
gives it the **bare `G2`** ("a repository-wide count that names no path") under PB §11's "gates
without a path use the bare id". **The wire does not exist; the diff-size sub-check is bare `G2`.**
*Certain.* (Both bear on `check`, not on the first three steps — recorded so the ruling is not
re-derived later.)

**C25 · Reserved runner/language tokens: three documents, three answers** (IR §18 OPEN-12).
RF §6.4 reserves `gradle`, `junit`, `kotest` and the language `kotlin`; IR §11.1 reserves `kotlin`,
`gradle`, `jest` and has never mentioned `junit` or `kotest`; MF §3.3 says `kotlin` is "not reserved
either". Inert in v1 — nothing emits any of them. **Emit none of `kotlin`, `gradle`, `jest`, `junit`,
`kotest`, `swift-testing`; IR §11.1 is the runner-token authority per RF §6.4.** *Certain* for v1;
the reservation set itself is the owner's.

**C26 · The `tool=` divergence in `envelope-vectors.md` — disclosed, not open.**
EV §8's five seals carry `tool=1.4.0+sha256:41d0e9b7…`; MF §8.2, GR §8 and RF §10 carry
`sha256:6f49644f…744db`. EV §15 and the README both record EV's as a **fabricated placeholder**
inside signed lines whose private keys are unpublished; all three are 64 hex so no byte count in EV
§8 moves. **Build against MF §8.2's value. Do not "fix" EV in place.** *Certain.*

---

## 4. Build order

Fourteen steps. Every step names the published vector or executable test that proves it, and no step
depends on a step below it.

### Step 1 · `spine-canon` — finish the primitive layer *(partly done)*

Extend the existing crate with the two encoders it still owes and the profile parameterization.
Four byte-encodings must exist and **must not be unified** (`12-canon-primitives.md` R64):
`esc`, `tok`, `git ls-tree` C-quoting (for `Spine-Frozen`), and the result-file JSON escape set. Three
JSON profiles: report/dump (GR §2.2), manifest (MF §2.2 — `-` in member names, booleans, resource
bounds), result file (RF §4.3 — no `esc`, `\u00xx` lowercase, canonical **on read**).

*Verified by:* GR §8.3's minimal canonicalizer vector `sha256:a594772c…` (**already green**);
GR §2.3's five `esc` worked cases byte-for-byte; `tok` differing from `esc` on exactly `,`, ` `, `"`
with `=` **unescaped**; MF §8.2's 529-byte artifact list hashing to
`sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db`; `git_blob_id` agreeing
with `git hash-object` on a scratch file at both object formats.

### Step 2 · `spine-manifest` — schema, canonical bytes, the twelve frozen fields

The value model, the JCS profile widening, `esc`'s per-member map, the `paths` **entry set**
`E(M) := { v : v is a string in some paths value }` (a value, never a key), the canonical `paths`
shape (string for a singleton, sorted array for ≥2), `files` sorted by `esc(path)` and path-unique,
the reserved names `trunk`/`dist_hash` at any depth, the closed malformed list of MF §3.11.

*Verified by:* MF §8.3 — 1762 canonical bytes / 1763 file bytes, git blob
`cb4cd49034bbe25f76573c40d6711b2c33f9136f`, SHA-256 `b19e7a01…` over the canonical bytes and
`54fa96d1…` over the file bytes; MF §2.5's `json_one` extraction returning `main` and the
`dist_hash` from those same bytes; a `paths` key named `trunk` refused as `reserved-member-name`.

### Step 3 · `spine-manifest` — the keyring parser, the lint, and `mode`

Parse **permissively then classify** (the §4.2 grammar is too tight for §4.4's own status
vocabulary — see risk R9 below). The eighteen `keyring-*` tokens. `mode` from the **key count**,
never `C-A1`, routed through one function. Exactly **two** mode-scoped clauses
(`keyring-seal-mixed`, `keyring-no-seal`), not the "five" MF §4.8.4 and §6.2 both say.

*Verified by:* MF §8.7 — 411 bytes, blob `6d4db08390092d7d5d96476eddca6355815bc49f`, three
`ssh-keygen -lf` fingerprints (`SHA256:dDNTLP8T…`, `SHA256:V2dasTIG…`, `SHA256:eQ0ZoC+r…`), lint clean
under `mode = team`; the walked lint of MF §8.7 reproduced clause by clause.

### Step 4 · Template and constitution rendering

The twelve template names at their v1 versions; the three intent scaffolds; the `constitution@1`
block; `C-T1`/`C-T2` rendered in the fixed language order `python, ts, dart, swift` restricted to
`params.langs`, each language's list in table order, a byte-identical pattern omitted after its first
occurrence. The three managed regions and their two marker syntaxes; a region's blob is
`git hash-object` over the region bytes **with no filters**.

*Verified by:* TM §6.4's three scaffolds — 380 / 501 / 434 bytes, sha1 `e627ec18…`, `09154925…`,
`5eb75dcc…`, each carrying exactly F3's non-ASCII counts; CN §12.2's constitution — 4724 bytes, sha1
`22609629e86d75a7c4abb7208c3575c7a8c2ead3`; GR §8.1's published `c_t2` for `["python","ts"]` —
15 patterns, `tests/support/**` deduplicated at its first (python) occurrence; the four-language union
at 22 patterns / 331 bytes; MF §8.1's three region blobs — 179 / 14 / 45 bytes, `ccf916b1…`,
`e7b7021f…`, `91b88cb4…`.

### Step 5 · The release manifest, the CI substitution, the byte scan

`release/release.json` as a **build input**, read once, frozen in, written into no repository, named
by no `files[]` record. Closed schema — an unknown member is a **refusal**, the opposite of
`.spine/manifest.json`'s rule. The five-row substitution table and no others. Substitute literally,
once, **never recursively**. Then the byte scan over rendered bytes for `@@` (two U+0040, any
context) and the three `PIN_` literals — **before any path is written**, one hit refusing the whole
plan.

*Verified by:* CI §5.3's `.spine/ci.sh` at **319 lines**, `git hash-object`
`131f13fb0312162579605999d3f9f4e90098c74c`, `sha256:d6bcf50c…`, over the **unsubstituted** bytes; a
build with no release manifest reporting `REFUSE` on every plan row with `no-release-manifest` and
creating no path; a `--trunk` containing `@@` refused as `trunk-name-collides-with-token`.
**No vector exists for a rendered CI file** — that is B11.

### Step 6 · `spine init` — plan, preconditions, atomic apply, refusals

The plan over blob ids; the three ownership classes; `--merge` / `--adopt` / `--force`; the atomic
apply in its exact order — render into `.spine/cache/staging/<run>/` → write
`staging/<run>/manifest.json` **before any rename** → parse-validate → atomic-rename each file →
write `.spine/manifest.json` **last** → delete staging; the three interrupted states, each detected
by hash; `--abort`; `--status`; language and signing-key discovery, each refusing rather than
guessing.

*Verified by:* an `init` into a scratch repository reproducing MF §8.1's `.gitattributes` (87 bytes,
`54b0a456…`), `.gitignore` (72, `9f0093f4…`), `AGENTS.md` (363, `1a05f30c…`), the three region blobs,
`CONSTITUTION.md` (4724, `22609629…`) and the keyring (411, `6d4db083…`); the manifest it writes
being canonical under step 2 and carrying **both** workflows `spine-owned` per CI §3.1.
**Not** an acceptance test: MF §8.3's blob id — §8.3 is a post-`--merge` repository and its
`ci.sh`/workflow bytes are stand-ins (C5, C6).

### Step 7 · Lifecycle: rollback, uninstall, re-init, and the G16 side of each

The restoration rule's six outright steps; the path set `P` from **both** manifests; the comparison
against the **tree blob at `<sha>`** and against **mode**, never the record's `blob`; the per-key
monotone union of `paths`; `eq` as canonical-byte equality; `from-manifest=`'s presence as the
trigger; §6.8's four uninstall checks; §6.9's two re-init checks and the single `manifest-missing`
exemption at `B`.

*Verified by:* MF §8.6 — `A` at 1696 canonical bytes / blob `24f11f00752bfb7bea259b4205315e7597692aca`
and `M_T` at 1710 / `74806e98701b50e958074dbaad0d7509d84751a3`, the 14-byte gap being
`["AGENTS.md","CLAUDE.md"]` against `"AGENTS.md"` and nothing else; `P`'s six members with
`.spine/allowed_signers` and `CONSTITUTION.md` excluded; the `Spine-Upgrade` line with the **empty**
`forced=`.

### Step 8 · `spine-collect` — the result-file grammar

Six header fields in fixed order separated by exactly one U+0020; the restricted canonical JSON,
required **on read**; three record kinds; `out`'s eight values plus the `base`-only ninth `absent`;
the sort on `runner` bytes then `id` bytes; exactly one `end` record, last; the `(runner, id)` pair as
the identity.

*Verified by:* RF §10's twenty-line file reproduced byte for byte — including `ids=7`, every `pytest`
record preceding every `vitest` one, and `…[reduced-rate]` before `[standard-rate]` before
`[zero-rate]`; RF §10's nineteen-line `runner-timeout` variant; `tool=1.4.0+sha256:6f49644f…744db`
from step 1. RF publishes **no digest** over these bytes — the file's printed bytes are the vector.

### Step 9 · `spine-isolate` — M1, the probe, the two dispositions *(linux-only)*

Five namespaces; a **lower-layer-only overlay** of the job's own root plus a bind-mounted writable
tree, a tmpfs scratch, and `pivot_root`; identity from a delegated `subuid`/`subgid` range or a
privilege-dropping root collector; `.spine/cache/` absent from the child's view; pipes on the host
side; two dispositions differing in exactly one thing, the network namespace.

*Verified by:* no published vector — the probe **is** the acceptance test, and its four negatives are
the real suite: a bare mount namespace over the job's root fails P3 (same `(device, inode)`); a bare
unprivileged user namespace fails P2 (the file comes back owned by `U`); a host with a default route
fails P4(b) unless the namespace is fresh; a mounted writable result directory fails P1. Plus the
`ci.sh` prerequisite: `umask 022` with explicit `chmod 0700 "$WORK"` and `0755` on `$INSTALL_DIR` and
`$BIN` — at 077 prerequisite 4 fails on every host and `container` is unlicensable.

### Step 10 · `spine-collect` — the order of operations

Steps 1–10 of RF §7.1 in order; the five step-1..5 refusals that write **no file**; everything after
step 5 always writing one; every `B` invocation of every runner before **any** `T` execution; the
deadline's three expiry sites with three different behaviours; the fold over the table's fixed order;
all-or-nothing on `B`; `status ≠ complete` crediting nothing; the solo path settling
`keys_visible=true` and `profile=none` before any observation.

*Verified by:* RF §10's two files, now produced by a real run rather than hand-built; a `uid` request
under `--ci` producing no file and `ci.sh` exit 2; a failed `B` **outcome** run leaving the `base`
section whole with `out: "absent"` and `end.status` unmoved; a hung runner producing
`runner-timeout` with the other runner's green records written as evidence and credited with nothing.

### Step 11 · `spine-graph` — the store and the derivation

`PRAGMA user_version = 7`; the `meta` table; the nine node kinds and fifteen edge kinds; PB §6.2's
derivation table as the indexer's spec; the `approval` local id as SHA-256 over the signed trailer
line's exact bytes.

*Verified by:* DM §12.1's three signed lines hashing to `2f5e600237ec3d9a…`, `b6352921ea42d618…`,
`ae8a406391f7130c…` — `printf '%s' "<line>" | shasum -a 256`, **no trailing LF**. Attack these three
before any derivation: they test the byte range and nothing else.

### Step 12 · `spine-graph` — the dump serializer and the two sort keys

JSONL framing with **every** line terminated including the last; `attrs` always present, `{}` when
empty; depth exactly two; the node key `kind ‖ NUL ‖ id ‖ NUL ‖ canonical(attrs) ‖ NUL ‖ src` and the
edge key `from ‖ NUL ‖ to ‖ NUL ‖ kind ‖ NUL ‖ canonical(attrs) ‖ NUL ‖ src`, both over **`esc`-encoded
bytes**; `approval.wires` never re-sorted, `signer.roles` sorted.

*Verified by:* DM §12.4's ordering vector — 11 lines, **1063 bytes**,
`sha256:a849ec349ef8f20ec1f40423ae6a7d3358745f4c9027545f55cf74ef9b72a139`; DM §12.5's empty dump —
**105 bytes**, `sha256:2a3fcea5debff8e17af36eef1a4acfa1fea9f86a0d6c942cf2993cce3e80f290`. DM's own
instruction is to debug the comparator against §12.4 **before** attempting §12.2.

### Step 13 · `spine index --dump` — the full projection

The exclusion set generated by one rule ("derived from git objects reachable from the trunk tip");
the three exclusions DM adds beyond PB's four adjectives; `implements.provisional` and
`protects.floor` `false` in every dumped record; every `test` node's `attrs` `{}`; the four exit
codes.

*Verified by:* DM §12.2 — **62 lines, 14054 bytes,
`sha256:3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da`**, with the composition
assertion 1 header + 28 nodes + 33 edges and the eight checkpoints (notably: `myrepo/INT-042` is the
twenty-third node; `caf\xe9.py` precedes `tax.py`; the `code_unit` `src` is the **minimum** over
citing edges; under one `from`, edges order by `to` **before** `kind`, so a `signed_by` sits between
two pairs of `freezes`).

### Step 14 · G10 — the comparison harness

Push `L` into scratch clone `S` as `refs/heads/<trunk>` with the intent ref deleted; clone
`--no-local --no-hardlinks file://S` with `GIT_CONFIG_GLOBAL=/dev/null`, no network, default refs
only; write the runner's pinned trust root into **both** sides' `spine.trustRoot`; index and dump
each; compare as byte strings.

*Verified by:* two indexings of one repository producing identical bytes (DM §11's "no false
positive"); the clone asymmetry exercised with a second intent branch open, which is the case that
forces the exclusion set; a `dump_version`/`schema_version` mismatch refusing with
`dump-version-skew` rather than comparing.

---

## 5. Vector attack order

Reproduce **before** writing the code that depends on it. Ordered so each vector's inputs are already
proven.

1. **GR §8.3** minimal canonicalizer — `sha256:a594772ccb6408158b6e76b170d5488c2454ba576e09ae379e24d743e21921f0`, 55 canonical bytes. *(green)*
2. **GR §2.3** the five `esc` worked cases, byte-exact, including `caf`+`0xC3 0xA9` → `caf\xc3\xa9` → `"caf\\xc3\\xa9"` in JSON.
3. **GR §6.2** `tok` vs `esc`: divergent on exactly `,` → `\x2c`, ` ` → `\x20`, `"` → `\x22`, one pass, `=` never escaped.
4. **MF §8.2** the 529-byte artifact list → `dist_hash sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db` (two spaces, sorted by artifact name, every line LF-terminated).
5. **TM §6.4** the three intent scaffolds — 380 / 501 / 434 bytes; sha1 `e627ec183de2a71b0e5aaed0b6227c1e8437ccde`, `091549257b229b6a3eb7ae5d44e4e9937a7d941a`, `5eb75dcc51602ecb01d9d428d2ed0eebb2d1a86c`.
6. **CN §12.2** the rendered constitution — 4724 bytes, 136 lines, sha1 `22609629e86d75a7c4abb7208c3575c7a8c2ead3`; header on line 2 and `C-A2` on line 96 (DM's two provenance strings depend on both).
7. **GR §8.1 / CN §6.4** the `c_t2` render for `["python","ts"]` — 15 patterns (6 + 10 − 1), and the four-language union at 22 patterns / 331 bytes joined by `, `.
8. **MF §8.1** the three managed-region blobs — 179 / 14 / 45 bytes; `ccf916b1f5a2813b9156128dff6f3bc4036c8b2d`, `e7b7021f73cd490a36a99973cb26c09c974b930d`, `91b88cb441665850be9c99df862e715fbea11311`.
9. **MF §8.7** the keyring — 411 bytes, blob `6d4db08390092d7d5d96476eddca6355815bc49f`, three fingerprints, lint clean under `mode = team` computed from the key count.
10. **MF §8.3** the manifest — 1762 canonical / 1763 file bytes, blob `cb4cd49034bbe25f76573c40d6711b2c33f9136f`, `sha256 b19e7a01…` / `54fa96d1…`. **Use MF §8.1's 234-byte `ci.sh` stand-in**, not CI §5.3's real render.
11. **MF §8.6** the rollback pair — `A` 1696 bytes / `24f11f00752bfb7bea259b4205315e7597692aca`; `M_T` 1710 bytes / `74806e98701b50e958074dbaad0d7509d84751a3`; `A.cli.dist_hash` is the SHA-256 of the 21 ASCII bytes `spine-1.3.0-artifacts` with no trailing newline.
12. **TM §8.6** the reopen — 1502 → 1557 bytes (+55), sha1 `89f6a976879cd598f2341d6d873b2c4eac808096` → `e92d825a37bfb5310ee13c27ff98d314ec514d10`, and the result **does not parse**.
13. **CI §5.3** `.spine/ci.sh` unsubstituted — 319 lines, `131f13fb0312162579605999d3f9f4e90098c74c`, `sha256:d6bcf50cf675614033aaef61df104aad253d30c4accc756719599ad5bd41060b`; plus its own execution suite (`sh`/`dash`/`bash`/`zsh`, `$WORK` `drwx------`, `$INSTALL_DIR` `drwxr-xr-x`, the `json_one` adversarial-path case returning `main`).
14. **DM §12.1** the three `approval` local ids — `2f5e600237ec3d9a1f35fbc7ee6cf0dfd4335433def9937b5eeb8202bd3d66f6`, `b6352921ea42d618930f32f04ad773c20157810d418d20d06758149f366b85a8`, `ae8a406391f7130ce62d0e86fa4dca92195812aae2743e15e60434af56244021`.
15. **DM §12.4 → §12.5 → §12.2** in that order — 1063 bytes / `a849ec34…`; 105 bytes / `2a3fcea5…`; 14054 bytes / `3321e7bd…`.

Deferred to `spine check`, listed so they are not lost: **GR §8.1** (3476 bytes,
`sha256:e2bd8cb5…5b47`) and **GR §8.2** (4053 bytes, `sha256:a47c1328…309e`) — §8.1 must be
recomputed first because bob's `report=` inside §8.2 carries it. **EV §8** vector A's `freeze=` (573
join bytes) and `envelope=` (2379 join bytes).

---

## 6. Where this gets implemented wrong

Ordered by how expensive the mistake is to find.

**R1 · The wire comparator, sorted numerically instead of by bytes.**
`wires[]` and a review's `wires=` sort ascending by unsigned byte value over the **whole token**, so
`G11` precedes `G2`; `gates[]` sorts by gate **number**. Re-sorting is a permutation, so **every
published byte count passes under both orders and only the digests separate them** (GR §8.2.1). An
implementation matching every length and no digest has a wrong wire comparator, not a wrong
canonicalizer. The sort key is `tok(path)`, **not** `esc(path)`.

**R2 · One serializer for three profiles, or one encoder for four encodings.**
Report/dump, manifest and result file are three profiles of one scheme, differing in member-name
grammar, booleans, resource bounds and the string-escape set. `esc`, `tok`, `git ls-tree` C-quoting
and the result-file escape set are four encodings of one path. EV §13.9: "an implementation that
reuses one encoder for both produces lines no conforming implementation reproduces." **Three
encodings of one path can appear in one landing** — `floor_hits` stores `esc(path)`, its derived
`G14` wire is `tok(path)`, and `Spine-Frozen` C-quotes it.

**R3 · The trailing-LF rule, which inverts by artifact.**
Report and its note: **none**. Manifest: `JCS(value) ++ 0x0A`, exactly one. Dump: every line
terminated **including the last**, and the digest covers it. `envelope=` and `freeze=` joins: no
trailing LF. This is the single easiest place to lose a digest.

**R4 · Sorting over raw path bytes instead of `esc`-encoded bytes.**
`src/\xe9.py` sorts **before** `src/z.py` under `esc` (`0x5C < 0x7A`) and after it under raw bytes.
And byte order is not numeric order: `AC-10` precedes `AC-2`. Both appear in DM §12.4, which is why
that vector is attacked before §12.2.

**R5 · A managed region's blob computed with `--path` filters.**
MF §3.5: the `--path` form **does not apply** to a region — `git hash-object` over the region bytes
**with no filters**, because those bytes are already in-blob bytes. Region bytes are everything
strictly between the markers: from the first byte after the begin marker's `0x0A` through the last
byte before the end marker's first byte.

**R6 · The region `@<n>` looked up by the region key.**
The key (`spine` for all three v1 regions) is **never** a `templates` index; the record's own
`template` member is. Indexing by the key asks for `templates["spine"]`, which no manifest contains,
and leaves `region-version-mismatch` undecidable for every region v1 ships (MF §3.7, R21).

**R7 · The `paths` value's canonical shape.**
Exactly one entry ⇒ a **string**; two or more ⇒ a **sorted array**. A one-element array, an empty
array, an unsorted array or a duplicated element is `manifest-noncanonical`. And an entry is a
**value**, never a key: moving `AGENTS.md` between keys drops no floor entry.

**R8 · `forced=`'s empty list written as `none`.**
The empty list is the **empty value** (`forced= signer=alice@example.com`). `none` is
indistinguishable from `tok("none")`, which is a legal path. A leading, trailing or doubled comma is
malformed. And the decoded set must equal `derived_forced` **exactly** — derived from blobs, not
trusted from the line.

**R9 · The keyring lint implemented from MF §4.2's grammar literally.**
The `entry` production is too tight for MF §4.4's own closed status list: a line carrying
`cert-authority`, `valid-after=`, `namespaces=""`, a typo'd namespace, `ssh-rsa`, or no options at
all does not match `entry` and reads as `keyring-line-malformed`, while §4.4 assigns each a distinct
token. **Field-split permissively, then classify**; reserve `keyring-line-malformed` for a line that
cannot be split into fields at all. The token is what a reviewer's `wires=` names when G16 raises it.

**R10 · The rollback's step 5, compared against the record's `blob`.**
The comparison is against **the blob in the tree at `<sha>`** — the only reading that works for a
`user-modified` path, whose tree blob at `<sha>` is the human's copy and whose recorded `blob` is the
render they diverged from. Step 5 also compares **mode**, and enumerates `P` **from the two
manifests, never from the diff**, so a path left wrongly untouched cannot pass by being absent from
`diff(B, L)`.

**R11 · Reproducing MF §8.3 against CI §5.3's real `ci.sh`.**
Two correct digests exist for one path (C5). Using the 319-line render in the manifest vector
produces a blob that is not `cb4cd490…` and an implementer will spend a day on the canonicalizer.

**R12 · The collector interleaving `B` and `T` per runner.**
Every `B` invocation of **every** runner — the enumeration and, where the adapter has one, the
separate `B` outcome run — precedes **every** `T` execution. RF §7.1 step 7 names the attack:
interleaving would let code the candidate ran under the first runner reach the second runner's
collection of the floor. `pytest` and `swift-test` each cost **two** `B` invocations per landing.

**R13 · The runner's exit code used as the `complete` discriminator.**
A red suite exits non-zero on every shipped runner, so an exit-code test makes `complete` unreachable
for exactly the runs G1 exists to judge. `complete` requires **both** that the terminal session-end
event was parsed **and** that no process-group member was terminated by a signal.

**R14 · A failed `B` outcome run treated as `base-collect-failed`.**
A failed `B` **enumeration** is all-or-nothing across runners: `ids=0`, no `base` and no `result`
records from anyone. A failed `B` **outcome** run is not a status at all: the `base` section stays
whole, every unreached id takes `out: "absent"`, and `end.status` does not move. The asymmetry is the
fail-closed direction — an enumeration that stops early shrinks the floor; an outcome run that stops
early can only withhold exemptions.

**R15 · M1 built the two ways that fail forever.**
A **bare mount namespace** over the job's own root makes the child's `/` return the collector's own
`(device, inode)` pair, so P3's separation limb fails on every host, for every configuration,
forever — hence the lower-layer-only **overlay**. A **bare unprivileged user namespace** maps exactly
one host uid, `U` itself, so the file P2 `stat`s comes back owned by `U` and P2 fails forever — hence
the two, and only two, identity arrangements. Both produce a permanently silent `profile=none`, which
is a downgrade nobody reads as a bug.

---

## 7. One-line summary

`spine-canon` is green against GR §8.3; the next byte that matters is MF §8.7's keyring and MF §8.3's
manifest, and the eleven decisions in §1 are what stand between the plan above and code that cannot
be written twice the same way.
