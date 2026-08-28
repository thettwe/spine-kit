# Adversarial verification — 2026-08-28, the two crates round 2 never reached

Both verifiers died on connection errors during the round-2 workflow, so
`spine-init`'s lifecycle and `spine-report` had never been reviewed. Both were
re-run to completion. **Neither found a fabricated value.**

- `spine-report`: GR §8.2's 4053 canonical bytes and `report=sha256:a47c1328…`
  reproduce **byte-identically**, verified three ways (a from-scratch JCS
  canonicalization of the spec's printed JSON; the crate's own
  `canonical_bytes()`; a byte diff of the two). §8.1's 3476 bytes — published
  nowhere — reproduce from §8.2 plus the two-member delta, and the crate's
  derivation of them is honest: one builder, a transcribed constant for the
  digest, so a wrong canonicalization fails loudly rather than laundering
  itself into §8.2.
- `spine-init`: every published digest re-derived independently rather than read
  back out of the code — MF §8.3's manifest (1763/1762/`cb4cd490…`), §8.6's `A`
  (1696/`24f11f00…`) and `M_T` (1710/`74806e98…`/gap 14), §8.1's region blob
  `ccf916b1…`, §8.2's artifact-list hash `sha256:6f49644f…`. All reproduce.

---

# `spine-init` — the lifecycle

## EVERY-LANDING

**E1 — a region template bump makes the upgrade unrunnable, with no exit.**
`plan.rs:310,319` locate the *existing* block with the **new** template
version, so a bump makes `region::find` return `VersionMismatch` for a region
byte-identical to what spine wrote. The row refuses; one refusing row stops the
whole upgrade (PB §6.7 step 3); and all three documented exits are closed
because `resolve` keys off `SpineOwnedDiverged`/`MarkersRemoved` only. If the
refusal is bypassed, `apply.rs:135` runs the same version-pinned find and
**appends a second block**, which is `region-markers-malformed` at G16 check 9
on every landing afterwards. `rollback::restore_region` does this correctly and
says why in a comment; `plan`/`apply` never got the same treatment.

**E2 — PB §6.7 step 1's clean-tree precondition is never enforced.**
`git.rs:117` `is_clean` and `:124` `dirty_paths` have no caller outside their
own unit test. The plan compares HEAD blobs, correctly, so a working-tree edit
is invisible to it — step 1 is the only thing covering the working tree.
Uncommitted work is silently overwritten by an upgrade, with no refusal and no
mention in the plan. `--abort`'s totality claim rests on the same precondition.

## SERIOUS

**S1 — `rollback::execute` carries out a plan that refuses.** `rollback.rs:513`
iterates every row and acts regardless; `RollbackPlan::refuses()` exists and
nothing consults it. `apply::apply` front-loads exactly this check.
**S2 — a region template bump defeats the rollback's `--force` refusal.**
`tree_state` locates with the *ancestor's* version, both reads become `None`,
`None != None` is false, and a human's committed edit inside the block is
discarded with no refusal.
**S3 — re-running `spine init` after a crash refuses.** `staging::classify` and
`Interrupted` have no caller in the workspace; `apply` unconditionally calls
`Staging::create`, which refuses when a run is pending. The diagnostic tells the
operator to do the thing they just did, and `--abort` is unimplemented.
**S4 — `apply` deletes a managed region's host file.** `plan.rs:230` gives a
region record `Action::Delete` and `apply.rs:194` removes the *file*. MF §3.7:
a region is "a block inside a file spine does not own". `uninstall.rs` gets this
right.
**S5 — the uninstall is not all-or-nothing.** `compute` returns only
`UserOwnedRegion`; `execute` raises `UnknownRegionTemplate` mid-walk with the
manifest removed last, leaving a half-uninstalled repository.
**S6 — the uninstall leaves a duplicated block's markers behind.**
`block_range` returns after one pair, so a copy-pasted block survives →
`uninstall-region-remains` at G16, outright.
**S7 — a hand-edited region is deleted without being named.** `current_blob`
uses the version-pinned find, so a hand-edited `@99` reads `Missing` rather than
`Modified` and `deleted_but_modified()` counts zero — the loudness rule failing
precisely on the case it exists for.
**S8 — the manifest is written by `fs::write`, not by atomic rename.**
`apply.rs:209`. A torn write is a malformed manifest at `B`, which refuses every
run before any gate; combined with S3 and the unimplemented `--abort` it bricks
the repository. The fix shape is already present and unused.
**S9 — "the graph cache is deleted" is executed by nothing.** PB §6.7 step 6.
`uninstall::execute` knows how; the upgrade path does not do it.
**S10 (scope) — none of this is reachable from the binary yet.** `--abort`,
`--rollback`, `--uninstall` print "not yet implemented"; `resolve::resolve` has
no caller, so `--merge`/`--adopt`/`--force` are parsed and ignored. These are
library defects that fire the moment the CLI is wired.

## MINOR

M1 `rollback::compute` reads `B` as the literal `"HEAD"` while `locate` takes a
ref. M2 the rollback's `--force` validation checks membership in `P`, not that
the row refused, and its error text describes the check it does not perform.
M3 `resolve` returns `NotRefused` for rows that did refuse with another reason.
M4 nothing implements MF §6.7 step 6's user-owned check. M5 `monotone_union`'s
shape selector would emit `[]` for a zero-value key (unreachable today).
M6 `UpgradeLine::parse` reports `UnknownKey` for a known field out of order.

---

# `spine-report`

**F1** `WireSet::from_raised` refuses any differing kinds, making GR §6.1's
`finding > advisory > warn` precedence unreachable code. `warn` + `finding` on
one key is reachable in v1: a `forbidden` hit outside `expected` under
warn-before-block calibration is one key and two kinds, and the report cannot
be constructed at all.
**F2** `Spine-Gates` renders the caller's order; `report=` sorts. One value,
two renderings, and the one that reaches `envelope=` is the wrong one.
`validate()` sorts before comparing and so never checks order.
**F3** `automerge.preconditions[0]` is never checked against
`policy.rules.c_a3`, and the rule-5 wire check derives from the unverified
array, so the two errors cancel: a hostile-threat repository can serialize
`"effective":true` with an empty `wires` array and validate clean.
**F4** GR §5.9's fixed shape for a landing that ingested no result file is
unenforced — `evidence: None` beside `profile: "container"` validates clean,
claiming a boundary no header established.
**F5** `ExemptOffTombstone` implements "any exempt" where §5.8 requires all
five; a tombstone can serialize `effective: false` where §5.8 requires `true`.
**F6** §6.3's token-shape column is modeled (`TokenShape`) and never enforced.
Bare `G5`, `G3:src/a.ts` and `G13:NOT-AN-OID` all validate clean.
**F7** An empty `path` is accepted, where GR §6.3's G1 row says "an empty path
being no path". Serializes `"path":""` and renders `G1:`.
**F8** The reader silently normalizes two things §3.2's closed schema makes it
refuse: an unknown member inside a known version, and an out-of-order `gates`
array. Both round-trip to different bytes, which is `--verify`'s
`report-mismatch` against a sound landing.
**F9** §5.6.1's *outright* set is not modeled; `derive`'s `all_findings_covered`
boolean cannot express "no review class admits it", and `suspends_outright` has
nothing to suspend.
**F10** `git_version` is an unchecked `String` beside a correct, unused parse.
**F11** `authority.signoff` presence is unchecked, and it flips
`self_approved` — the fact PB §11's signerless overlay turns on.
**F12** Two wrong counts in a doc comment.

---

# RESOLUTION — 2026-08-28

## `spine-init`

**One root cause under E1, S2 and S7: `region::find` conflated two questions.**
MF §3.7 says "a region is located by its markers only", and `find` located *and*
checked the version in one call. Split into `region::locate` (markers only) and
`region::find` (locate plus the version check), with the doc on `find` saying
which callers want which. Every lifecycle caller — the plan, the apply, the
rollback's `tree_state`, the uninstall's `current_blob` — wants `locate`, and
`plan::compute` no longer takes a template-version table at all: it was only
ever used to ask the wrong question.

- **E1** fixed. A template bump is now an ordinary `update`, decided by the
  recorded blob like every other row, and `apply` replaces the block instead of
  appending a second one. A hand-edited block still refuses.
- **E2** fixed. `apply` takes the working tree's dirty paths and refuses on
  them, with PB §6.7's exception applied where staging is known — narrowly:
  only a *resumed* run's staged bytes, compared byte-for-byte, are exempt.
  **DERIVED**: untracked files are narrowed to paths the run would write. The
  precondition exists because "Spine cannot lose an edit it can see", and an
  untracked file at a path no render touches cannot be lost by the upgrade or
  by `--abort`, which checks out only the paths the manifests name. Refusing on
  every `??` would refuse on a build output and is not a safety property. A
  tracked modification anywhere still refuses. Verified end to end against the
  real binary: the refusal fires and the uncommitted work survives.
- **S1** fixed. `execute` front-loads the refusal, as `apply` does.
- **S2** fixed by `locate` — the `--force` refusal fires across a bump.
- **S3** fixed. `Staging::resume_or_create` adopts a pending run and says so,
  which is what makes PB §6.7's "fixed by re-running `spine init`" true. Two
  pending runs stays a refusal.
- **S4** fixed. A retired region is `Action::StripRegion`, not `Delete`: the
  block comes out and the host file — a human's, and a floor path — stays.
- **S5** fixed. Every reason `execute` could fail on a row is decided in
  `compute`, before a byte moves.
- **S6** fixed. `strip_region` loops until the host is marker-free, so a
  copy-pasted block does not survive into `uninstall-region-remains`.
- **S7** fixed by `locate` — a hand-edited region reads `Modified` and is named.
- **S8** fixed. The manifest goes through staging like every other render, so
  it moves by atomic rename and is parse-validated before it lands.
- **S9** fixed. PB §6.7 step 6's graph-cache delete runs.
- **S10** stands: the CLI still refuses `--abort`, `--rollback`, `--uninstall`
  and ignores `--merge`/`--adopt`/`--force`. These fixes are what makes wiring
  them safe.
- **M2** fixed: `--force` must override an actual refusal, not merely name a
  path in `P` — the check its own error text already described.
- M1, M3, M4, M5, M6 stand; none is reachable from v1's flows and each is
  recorded above.

## `spine-report`

All twelve worked. Seven new invariants (precondition 0 against `c_a3`,
precondition 1 against `profile`, §5.9's no-evidence shape, the gates order,
§6.3's token shape, the `git_version` form, a reseal's sign-off), the
all-five-exempt fix, §6.1's kind precedence made reachable, the empty path made
no path, the reader's two silent normalizations closed, and `GateStatus::derive`
given a four-valued `Findings` so outright cannot be confused with uncovered.

Workspace: 1343 tests, 0 failures, clippy clean, in both the debug build and
the release build with `synthetic-release`.
