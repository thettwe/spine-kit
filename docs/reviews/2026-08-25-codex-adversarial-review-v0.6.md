# Codex Adversarial Review of spine-kit-playbook.md (v0.6) — model gpt-5.6-sol — 2026-08-25

Verdict: needs-attention

No-ship: the design's central claims—rebuildable provenance, frozen tests, and safe auto-merge—are not enforceable with the specified artifacts and state model.

## F1 [critical] Deleting intents destroys the only deterministic source of truth (§6.2 derivation table, lines 363-372)
The playbook deletes the intent and claims the indexer can reconstruct it from PR descriptions "in git log." PR descriptions are hosting-provider metadata, not guaranteed Git objects; they can be edited, unavailable offline, or omitted from merge commits. The derivation table also never defines how an `implements` edge is extracted. A fresh clone therefore cannot reliably rebuild intent, approval, changeset, revert, or pragma provenance, invalidating the graph's core guarantee.
Recommendation: Persist an immutable, machine-readable intent envelope in Git—such as a signed merge-commit trailer or content-addressed archive—and explicitly define `implements` derivation. Make provider metadata supplemental, then test reconstruction from an offline clone containing only Git objects.

## F2 [high] Test immutability has no enforceable checkpoint (§6.2 schema, lines 337-358)
Tests are declared frozen after approval, but the schema stores only their current location and latest result. It records no approved test-tree hash, blob IDs, approval commit, or transition event. Agent A can weaken or replace a test during implementation, after which indexing observes only the modified version and still reports a passing `verified_by` edge.
Recommendation: At `tests-approved`, persist a signed approval record binding the intent revision, every test blob hash, base commit, and approving actor. Reject implementation commits that alter those blobs unless the intent is formally reopened and the complete approval sequence reruns.

## F3 [high] Auto-merge lacks an authorization and gate-integrity boundary (§7, lines 433-452)
A signed commit proves identity, not authority. The design does not define who may sign an intent, whether authors can approve their own work, or which protected paths always require independent review. An intent can declare CI workflows, gate configuration, agent prompts, the constitution, or spine metadata as expected touchpoints and modify the mechanism evaluating its own change before auto-merge.
Recommendation: Define repository-level authorization policy and separation of duties. Hard-protect CI workflows, gate/configuration files, agent instructions, constitution files, and approval policy with CODEOWNERS-equivalent review; execute gates from immutable trusted tooling; bind approval to an exact intent hash; and forbid self-approval where auto-merge is enabled.

## F4 [high] Concurrent intents have a merge-time race (§5.4, lines 289-297)
Soft leases are checked at sign-off and after another change merges, but neither check reserves state atomically or binds approval to a base SHA. Two overlapping intents can pass sign-off concurrently, both test against stale bases, and enter auto-merge before either post-merge re-verification affects the other. Rerunning tests after a merge is too late if the second merge is already queued or approved.
Recommendation: Require a serialized merge queue. Bind sign-off, test approval, and gate results to exact head/base SHAs; atomically reserve normalized touchpoints; invalidate approvals whenever the base changes; and rerun all checks on the synthetic merge result immediately before merging.

## F5 [medium] Installed scaffolding has no safe upgrade or rollback model (§9 roadmap, lines 493-501)
`spine init` writes CI and agent-context files, but the roadmap defines no install manifest, file ownership, local-modification detection, schema migration, atomic upgrade, downgrade, or rollback behavior. Template versioning covers historical intents only. Once shipped prompts, checks, or graph schemas evolve, repositories can silently retain incompatible versions or lose user edits during refresh. The referenced spec-kit explicitly uses manifests, hashes, modified-file blocking, and rollback for this lifecycle.
Recommendation: Include lifecycle management in the v1 architecture: record CLI/schema/template versions and hashes in an install manifest; provide status, dry-run upgrade, migration, rollback, and uninstall paths; preserve locally modified files by default; and test interrupted upgrades and CLI/project version skew.

## Next steps (Codex)
- Replace PR-description reconstruction with an immutable Git-native intent envelope.
- Extend the state model to bind approvals and gate results to hashes and SHAs.
- Specify authorization, protected paths, and trusted execution before auto-merge.
- Design a serialized merge queue and atomic interference handling.
- Add manifest-aware installation, migration, and rollback requirements to v1.
