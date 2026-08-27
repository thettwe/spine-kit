# Codex Adversarial Review of PLAYBOOK.md (v0.11) — model gpt-5.6-sol — 2026-08-26

Fifth review. Target: working tree (PLAYBOOK.md v0.11, ~1031 lines). Asked to judge whether the v0.9 findings are closed, to attack the new §6.8 OKF projection hardest, and to test the reframed human-gate claim.

**Not raised, and therefore closed:** the auto-merge criticality that ran through the v0.7, v0.8 and v0.9 reviews. Codex's next-steps line confirms the rollback and result-header fixes close the cited v0.9 findings. The deletion of deferred reconstruction went unchallenged.

Verdict: needs-attention

No-ship: v0.11 makes the default workflow materially more human-heavy than its budget admits, leaves an ambiguous security-critical transition, and adds an OKF surface that can become stale knowledge while contradicting the document’s own graph rule.

Findings:
- [high] Default settings match two transition rows with different review authority (PLAYBOOK.md:487-488)
  The shipped defaults are `C-M4=off` and `C-A3=hostile`. Every landing therefore simultaneously matches the tripwire row for `C-M4` being off and the protected-review row for a failed threat-model precondition. The table declares itself exhaustive but defines neither precedence nor a combined state. A first-match implementation can choose `landing-review`, where self-review is allowed, instead of the intended protected review requiring reviewer separation. This recreates the authorization downgrade v0.11 claims its consistency pass fixed.
  Recommendation: Define a normative wire-aggregation rule: any protected-class wire dominates all tripwire wires, producing one `protected-review` state whose signed `wires=` covers the complete set. Add the combined default case explicitly to the transition table and §11.
- [high] The human-cost headline excludes a mandatory default landing review (PLAYBOOK.md:930-943)
  Under the default hostile threat model, precondition 0 always fails and every gated landing requires an explicit protected human review of code and test outcomes. Thus v1 requires at least sign-off, the `--approve` attestation, and a distinct landing judgement; in team mode the landing reviewer must differ from the signer. Quick-lane changes are worse: they have no signer, so the same default protected review requires two distinct reviewers. Calling reviews “conditional” is false when shipped defaults make the condition universal, and the claimed budget of one judgement plus one attestation materially understates adoption cost.
  Recommendation: Budget and headline the default workflow honestly: two judgement stages plus one attestation for gated intents, and two protected reviewers for quick changes in team mode. If that exceeds the intended budget, redesign the default lane or remove the one-gate claim.
- [high] The OKF export recreates the stale-knowledge failure the design exists to prevent (PLAYBOOK.md:727-744)
  The bundle is manually generated and expressly intended for another agent to read, but it carries no normative bundle-level source tip, dump digest, emitter version, or freshness check, and G10 deliberately does not verify it. After any landing, an existing bundle can remain unchanged and be consumed as current knowledge with no detectable warning. `git check-ignore` only constrains where the command writes; it cannot establish freshness or enforce that consumers do not treat the files as authoritative. A stale intent status, edge, or `declares` polarity can therefore misdirect an agent exactly like the stale specifications §1 identifies as the motivating failure.
  Recommendation: Do not ship the projection until it has an atomic bundle manifest binding the exact trunk SHA, canonical dump digest, schema/emitter version, and complete file digest set, plus a consumer-side freshness/conformance check. Otherwise remove OKF from the build reference.
- [medium] The OKF projection is a third graph under the playbook’s own rule (PLAYBOOK.md:935-943)
  Section 6.1 says without qualification that a rendering which is read is a graph. Section 6.8 defines OKF specifically so another agent can read it, while the complexity budget excludes it from the graph count because spine does not read it back. Those statements cannot all hold: either “read” secretly means “read by spine,” contrary to the stated rule, or OKF is a third externally consumed graph with its own schema, semantics, freshness and compatibility costs. The current annotation evades rather than satisfies the graph budget.
  Recommendation: Make the rule precise and budget accordingly. If external consumption counts, raise the graph budget and apply graph-level determinism, provenance and versioning requirements to OKF; otherwise redefine the rule explicitly as “read back by spine” and justify why stale external consumers are safe.

Next steps:
- Resolve protected-versus-tripwire precedence before implementation.
- Recalculate the human budget using default constitution values.
- Either harden OKF as a versioned, freshness-bound projection or remove it from v1.
- Retain the v0.11 rollback and result-header fixes; they close the cited v0.9 findings.

