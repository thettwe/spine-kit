# Codex Adversarial Review of PLAYBOOK.md (v0.7) — model gpt-5.6-sol — 2026-08-26

Target: working tree (PLAYBOOK.md v0.7, 969 lines). Focus given to the reviewer: re-check the v0.6 findings F1–F5 against §12's closure map; attack the new mechanisms (§4.3, §5.4, §5.5, §6, §6.7, §7); check internal consistency against §11.

Verdict: needs-attention

No-ship: v0.7 still cannot guarantee safe auto-merge, frozen tests, or reconstruction before irreversible landing, and its manifest upgrade protocol deadlocks on schema evolution.

## F1 [critical] Candidate-controlled test reports can authorize arbitrary code (PLAYBOOK.md:798-803)
The trusted stage validates only that an untrusted result file names the expected tree; it never executes or independently observes the tests. The playbook explicitly admits that candidate code can monkeypatch the runner or forge the result file. An adversarial branch can therefore report every frozen and trunk test ID as passed, satisfying G1 while executing no meaningful tests. G6 cannot repair this if its results traverse the same attacker-controlled reporting channel. This defeats safe auto-merge for any non-floor code.
Recommendation: Disable auto-merge until a trusted controller launches the hermetic test process and cryptographically attests the tree, trusted runner/collector identity, collected IDs, and outcomes. Candidate code must not be able to construct the authoritative result artifact.

## F2 [high] Mutable expected-path oracles bypass the blob freeze (PLAYBOOK.md:294-300)
The closure deliberately excludes repo-local imports under expected touchpoints as code under test and explicitly leaves an oracle, golden, or snapshot inside expected unfrozen. A test can delegate expected-value calculation or assertion control to such a mutable helper; implementation then changes that helper while every Spine-Frozen blob remains identical. Agent B receives only interface information for expected code, and the v1 approval is a developer's self-attestation, so no mechanical gate reliably detects this escape. F2 is therefore not genuinely closed.
Recommendation: Forbid approved tests from deriving expected values or assertion decisions from mutable expected-path code, or freeze all repo-local test dependencies except narrowly classified SUT entry points. Route every exception to protected review and require independently executed mutation testing.

## F3 [high] Offline reconstruction is checked only after the irreversible CAS (PLAYBOOK.md:615-616)
G10 runs after trunk and the intent ref have already been atomically updated. Its failure is classified as an indexer defect that does not invalidate the landing or block anything. Consequently, the first envelope/indexer incompatibility can enter trunk before the claimed offline-reconstruction property is proven, leaving a shipped history that a clean clone cannot currently reconstruct. F1's central guarantee is monitored after the fact rather than enforced.
Recommendation: Construct the proposed landing commit locally, expose it through a temporary synthetic trunk ref, perform the clean offline clone and canonical dump comparison before the CAS, and refuse the push on mismatch. Retain the post-CAS run only as redundant verification and block subsequent landings after any failure.

## F4 [high] Manifest schema upgrades have no executable transition path (PLAYBOOK.md:678-713)
Ordinary upgrades must be evaluated by the base's old pinned binary, while the design claims that every binary can parse every future manifest version and later states that an old binary refuses a manifest_version bump. A binary cannot anticipate arbitrary future schema semantics, and the specified recovery exceptions do not include forward manifest-schema migration. The first manifest-version bump therefore cannot land through either the normal upgrade path or the defined recovery path, leaving F5 incomplete.
Recommendation: Define an immutable bootstrap manifest envelope and a mandatory two-release bridge: release N must understand and validate manifest N+1 before any landing switches versions. Specify compatibility invariants, unknown-field handling, and an explicit recovery rule for forward schema migration.

## F5 [medium] Normative trailer vocabulary makes upgrade events malformed (PLAYBOOK.md:918-929)
Section 11 wins over prose and requires Spine-Intent on every event except quick and reseal landings. However, the upgrade event defined in §6.7 has Spine-Event: upgrade and Spine-Upgrade but no intent ID. An implementation must either reject every upgrade event under the normative grammar or silently invent an undocumented exemption, making the lifecycle ledger and G9 parsing inconsistent.
Recommendation: Add upgrade events to the explicit Spine-Intent exemptions and define their canonical event identity and landing seal subject in §11; update G9 and the derivation table to use that exact grammar.

## Next steps (Codex)
- Replace self-reported test authorization with trusted execution attestation.
- Close the mutable-oracle hole in the frozen closure definition.
- Move G10 reconstruction proof before the landing CAS.
- Specify a buildable bridge protocol for manifest-version changes.
- Resolve the upgrade-event trailer contradiction in §11.
