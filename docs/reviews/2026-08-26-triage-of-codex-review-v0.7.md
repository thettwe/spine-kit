# Triage of the 2026-08-26 Codex adversarial review (v0.7)

Method: 10 independent verifiers (a refuter lens and a builder lens per finding) read PLAYBOOK.md v0.7 against each Codex finding, then one synthesizer broke ties. Line numbers are v0.7 line numbers.

# Triage: Codex adversarial review of PLAYBOOK.md v0.7

## 1. Summary table

| Finding | Codex sev | Refuter | Builder | Consensus | One-line reason |
|---|---|---|---|---|---|
| F1 Forged test results authorize code | critical | CONFIRMED (0.85) | CONFIRMED (0.85) | **Real** (disclosed, but understated) | L803 admits the hole; its G6 escape is false for a selective forger, and L8/L88/L829-830 still sell "safe auto-merge" unqualified. |
| F2 Mutable expected-path oracle | high | PARTIALLY (0.75) | CONFIRMED (0.85) | **Real, medium** | L296 states the carve-out and names B as owner; but L18 says LLM noticing is discipline, and L830/L960/L965 over-claim closure. |
| F3 G10 after the CAS | high | PARTIALLY (0.72) | CONFIRMED (0.85) | **Real, medium** | L616 literally "blocks nothing"; ledger stays reconstructible by construction, but landings continue under a divergent indexer with unrecomputable `report=`. |
| F4 Manifest schema upgrade deadlock | high | REFUTED (0.75) | PARTIALLY (0.75) | **Partly real, low** | L678/L699 already give the frozen-field bridge; only L713's last sentence contradicts it and §11 never states the invariant. |
| F5 `Spine-Intent` required on upgrade events | medium | PARTIALLY (0.8) | CONFIRMED (0.85) | **Real, low** | L919 cell is literally wrong for upgrade *events* (and quick/reseal review events); landing side already consistent via L920/L926/L933. |

## 2. Per-finding detail

### F1 — self-reported test results (L798, L803)
- True: the trusted stage executes nothing from the repo (L739, L798) and ingests any result file whose `tree=` matches (L798); G1 (L612) is satisfied by a forged file. L803 concedes "nothing closes it". Both verifiers reproduce it end-to-end with a prompt-injected Agent A, the §7 first-named adversary (L727).
- Both agree the real defect is textual: (i) L803's "G6 flags as weak" only holds for an always-green stub — G6 runs the SUT, so it must run in the untrusted job and share the forgeable channel; (ii) L8, L88, L829-830 claim "safe auto-merge"/"drift blocks the merge" without the §7.4 qualifier. No disagreement on substance; both also agree Codex's "cryptographically attest outcomes" is unbuildable for in-process runners and "disable auto-merge" is already the default (L129, L795, L858).
- v0.8 change: (a) rewrite L803 — drop sandbox/tree-id/G6 from the forgery bar-raisers, state that G1's "passed" is exactly as strong as runner isolation, and that G6 shares the channel; (b) have trunk's `.spine/ci.sh` (already run from `origin/<trunk>`, L795) collect the B id set before checkout of `T`, run the runner as a child with no write path to the result location, and write the result file itself — a flag on `spine check --ci`, no new command; (c) qualify L88 and L829-830 with a pointer to the §7.4 residual; (d) strike "the countermeasure" at L300/L622. Optionally gate `C-M4 = on` behind a collector-produced header the way L795 gates it behind the key probe. Severity stays critical-as-disclosure-defect, not as new hole.

### F2 — oracle inside `expected` (L296)
- True: L296 excludes every import resolving into `expected`, explicitly says an in-`expected` oracle is "Agent B's job to notice"; G8 (L298) compares only frozen/harness blobs; G12 (L304) makes the missing helper red, which is the *good* signature; C-T3 (L125) sees no pytest import. Mechanically nothing fires. L830 ("re-fixtures … G8 rejects a changed byte"), L960 and the L965 residual list do not disclose it.
- Disagreement: Refuter says "disclosed carve-out, medium"; Builder says "confirmed, L18 makes B-noticing discipline". Builder is right on L18 but Refuter is right that Codex's fix (classify SUT entry points) is unbuildable under L210/L678 and that the v0.6 F2 as literally worded is closed. Net: real, medium, a strength-family gap mislabelled as closed by G8.
- v0.8 change: (a) add "oracle/golden/helper inside `expected`" to L965 and fix L830/L960 wording; (b) inside the closure walk `--approve` already does: a test-root import resolving into `expected` to a module with no inbound non-test import in the approval tree is test-only code — freeze it as a leaf or trip the existing closure tripwire → `approval-review` with `reason=` (L477, L922). Zero new gate/command; catches the blatant helper/golden-in-src variants; residual (oracle also imported by SUT) stated. Optional: `Spine-Unfrozen: <path>` derived trailer so reviewers see what can move.

### F3 — G10 post-CAS (L616)
- True: L616 "after the CAS … blocks nothing … may schedule it"; L930 excludes it from `Spine-Gates`; L383 pushes before any clone/index of `L`. Nothing gates a later landing on a prior G10 failure, and `--verify` (L799) cannot recompute a report the runner and a clean clone disagree on. Not in L965's residual list.
- Disagreement: Refuter says the *ledger* is intact by construction (every G9 input is a git object, L615/L933) so "high" overstates; Builder says the L88 per-landing property is nonetheless unproven before push and the sub-case where fresh G9 rejects `L` forces a reseal scar (L446). Both are right; the honest severity is medium and the fix is cheap because `L` is fully built at step 4 (L382).
- v0.8 change: insert step 4b between L382 and L383 — set local trunk to `L` (already in L616), clone/index/dump-diff, and additionally self-G9 `L`; mismatch → refuse push, reset to `origin/<trunk>` (existing discard path, L383), count in `spine stats`. Keep "never G10 in Spine-Gates" (recording it would alter `L`). Drop Codex's redundant post-CAS rerun and persistent "block subsequent" flag (no side state, L389/L463). For repos that keep G10 scheduled, add "G10 after landing" to L965. Reword L516, L827, L959.

### F4 — manifest_version bump (L678, L699, L713)
- True only at the wording level: L713's "an upgrade the older binary refuses" contradicts L678 ("every binary parses [frozen fields] for every `manifest_version` it will ever meet") and L699/L711 (base's pin evaluates a `Spine-Upgrade` landing). §11 is silent on `manifest_version`, so the tie is unadjudicated.
- Disagreement: Refuter REFUTED (L713 is the local-skew case, mechanism exists, §12 L965 records the closure); Builder PARTIALLY (an implementer reading L713 literally deadlocks; uninstall→re-init at L717 is an unnamed escape). Refuter is right that Codex's premise — old binaries must understand new semantics — is answered by "treats the rest as opaque", and that the two-release bridge is strictly worse. Builder is right that a build-reference must not leave the sentence ambiguous. Low severity, prose fix.
- v0.8 change: (a) rewrite L713's last sentence: bump lands like any upgrade under the base's pin by frozen fields alone; afterwards a local binary that does not know the new version is "older" and refuses; (b) at L678 state the invariant — frozen fields' names/types/`owner` set never change; a release needing to change one is `--uninstall` + re-init (L717), not a bump; (c) one clause under §11 Files (L935) so the invariant has a normative home; (d) tighten L812 "rollback landing" to `to=` lower than `from=`. Reject Codex's forward-migration recovery rule (would let an upgrade land without the trusted stage).

### F5 — `Spine-Intent` on upgrade events (L919)
- True: L919 exempts only "quick and reseal landings"; the upgrade commit is an *event* (L918, L696) with no id (L929, L946), so §11 (which wins, L8) makes it malformed. Same cell also breaks review event commits on `quick/*`, `quick/reseal-<O>` (L446) and `spine/upgrade-*` — Codex's scope was too narrow.
- Disagreement: none on the core. Refuter correctly refutes Codex's three collateral claims: seal subject is forced to `quick` (L920, L933), review grammar already admits id-less landings (L926), derivation already keys on `Spine-Upgrade` (L573, L583), G9 (L615) demands `Spine-Intent` only on withdraw. Builder adds real adjacent gaps: no transition-table row for `spine/upgrade-*` (L498/L510), `approves` target for an upgrade approval node unstated (L542/L573).
- v0.8 change: one cell at L919: "every commit on `refs/heads/intent/*`, every gated landing and tombstone; absent on quick, reseal and upgrade events/landings and their review events (no id, L946)". Plus one phrase each: L933 "an upgrade landing seals as `quick`"; L573/L583 "upgrade approval `approves` `cs:<L>`"; L498 add `refs/heads/spine/upgrade-*`.

## 3. Closing verdict

Codex's no-ship is **not justified as stated** — none of the five is an undisclosed security hole, F4 is a wording ambiguity, F5 is a one-cell vocabulary bug, and Codex missed the existing mechanisms at L678/L699 (F4), L920/L926/L933 (F5), and the L296 disclosure (F2). But v0.7 should not be frozen as a "build reference" either, because three findings expose the same defect: the document claims closure where it only disclosed a residual, and in F1 and F2 the named countermeasure (G6, Agent B) is weaker than the text implies.

Gates for v0.8, in order:
1. **F1** — must change. Rewrite L803, qualify L8/L88/L829-830, strike G6-as-countermeasure at L300/L622, and add the controller-owned result channel (flag on `--ci`). This is the headline guarantee and the doc's own L18 standard.
2. **F3** — should change; it is the cheapest real fix (reorder one step, ~4 line edits) and turns an "after the fact" monitor into the per-landing property L88 promises.
3. **F2** — must at least be disclosed (L830, L960, L965); the leaf-freeze narrowing is worth adding but can be v0.8.x.
4. **F5, F4** — prose fixes, do them in the same pass; neither gates anything but F5 would block a literal implementer of §11.

All five fit §10's budget unchanged: one human gate, three artifacts, four commands, nothing newly authored.