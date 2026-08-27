# Codex Adversarial Review of PLAYBOOK.md (v0.9) — model gpt-5.6-sol — 2026-08-26

Fourth review. Target: working tree (PLAYBOOK.md v0.9, ~1002 lines). The prompt asked Codex to judge, on its merits, v0.9's explicit refusal of its v0.8 recommendation to make auto-merge mechanically unavailable for in-process runners.

Verdict: needs-attention

No-ship: v0.9 relocates the central auto-merge trust failure and adds rollback and scheduled-reconstruction protocols whose claimed safety cannot be derived from the normative Git ledger.

Findings:
- [critical] Auto-merge still accepts outcomes the named adversary can forge (PLAYBOOK.md:810-821)
  Rule 5 attests the collector boundary, reconstruction mode, key visibility, and CAS, but none establishes runner honesty. The document explicitly concedes that candidate code shares the test process and can monkeypatch assertions or the reporter so the pinned collector records fabricated passes. Under the stated prompt-injected-agent threat model, `params.isolation=container|uid` therefore allows C-M4 to evaluate on while every substantive test outcome is false. Sealing `profile=` improves disclosure, not authorization; an ADR accepting the residual does not make automated landing safe. The argument that disabling this for in-process runners would delete the product is a product-viability argument, not a security closure.
  Recommendation: Make auto-merge mechanically unavailable whenever candidate code and the authoritative runner/reporter share a process. Define an actually isolated outcome-attestation profile, or require an explicit human review of code and independently observed test outcomes rather than permitting C-M4 on.
- [high] Scheduled reconstruction depends on failure state the ledger never records or recovers (PLAYBOOK.md:618-619)
  Scheduled mode says a failed proof prevents every later `proved=` from advancing, but G10 is forbidden from `Spine-Gates`, the seal records only `proved=<sha>`, and no normative trailer or event records `failed=<sha>`. A fresh clone therefore cannot determine which historical landing failed merely from the specified ledger, so G9 cannot enforce the rule without undeclared external or recomputed state. The absolute prohibition on advancing past a failed landing also defines no retry or superseding proof after an indexer fix, causing a permanent landing deadlock once the backlog reaches `<n>`.
  Recommendation: Either remove scheduled mode or add Git-native reconstruction proof/failure events naming the target landing, tool, canonical dump digest, and disposition. Define a signed superseding success after fixes; G9 must derive backlog and blocking state solely from those events.
- [high] G16 can certify an incomplete rollback as an exact restoration (PLAYBOOK.md:629-630)
  The rollback branch compares manifest records with the named ancestor but checks file content only for managed paths present in the rollback diff. A recovery author can leave a newer or otherwise incorrect managed file unchanged, install the ancestor's manifest record for that path, and pass the stated predicate because the path is absent from `diff(B,L)`. The normal scaffold invariant is in the `Otherwise` branch and therefore does not repair this. The resulting tree can execute managed CI or agent-context content inconsistent with the restored manifest and tool pin.
  Recommendation: For rollback, enumerate every managed path in the union of the base and ancestor manifests and require its blob in `T` to equal the ancestor-prescribed blob or deletion, regardless of whether Git reports it in the landing diff.
- [medium] Normative result-file grammar omits the profile required to authorize auto-merge (PLAYBOOK.md:951-953)
  Section 11 wins over prose and defines the collector result's first line without `profile=`, while G1 and rule 5 require that header field and compare it with trunk's `params.isolation`. A strict implementation of the normative grammar cannot evaluate precondition 1; a permissive implementation must invent an undocumented field at the security boundary.
  Recommendation: Add `profile=container|uid|none` to the normative result-file header schema and specify exact parsing, duplicate-field rejection, and how the pinned collector derives the value from successful boundary creation rather than configuration alone.

Next steps:
- Redesign C-M4 around outcome integrity rather than result-file custody.
- Make scheduled reconstruction state Git-native and retryable.
- Strengthen G16 to validate the complete restored managed-path set.
- Reconcile the collector header grammar in Section 11.

