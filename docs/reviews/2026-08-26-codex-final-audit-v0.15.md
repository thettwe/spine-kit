# Codex Final Audit of PLAYBOOK.md (v0.15) — model gpt-5.6-sol — 2026-08-26

Seventh review, run as a pre-implementation audit: *can a competent engineer who has never seen this document implement v1 from it without inventing a rule, resolving a contradiction, or guessing at a security boundary?* Public-toolkit framing given as an explicit constraint.

Verdict: **not ready** — 2 high, 2 medium. Two of the four were created by the v0.15 owner decisions themselves. All four answered in v0.16.

Verdict: needs-attention

No-ship: v0.15 still permits a default hostile-agent review to survive onto a tree nobody reviewed, contradicts its signerless quick-lane authorization rule, requires retry state it never stores, and has not actually removed OKF from the build reference.

Findings:
- [high] Default landing reviews can authorize a different final tree (PLAYBOOK.md:493)
  A review signs an exact tree and report, but after a lost CAS this row retains it when the head and wire set are unchanged and the base delta avoids floor/wire paths. The universal hostile-agent wire is bare `G1`, so it names no paths; almost any non-floor base movement can therefore retain the old review while producing a new synthetic tree and gate report. This defeats the stated control that a human reads every landing under `C-A3: hostile`: code can land after integration changes without anyone reviewing the final tree or its reported outcomes.
  Recommendation: Invalidate every rule-5 `G1` tripwire review on any base movement and require a new signature over the recomputed tree and report. More generally, specify that a retained review is consumable only if its signed `tree=` and `report=` equal the final gate record.
- [high] Quick-lane routing bypasses the signerless-landing authorization rule (PLAYBOOK.md:505-506)
  The default quick-lane transition requires one `class=tripwire` review and reaches `checked`, but ordinary quick landings have no signer. G13 at line 629 requires a signerless team-mode landing to carry two distinct `class=protected` reviews, and the normative trailer rule at line 949 also says two reviews are required. Wire aggregation at line 960 says the default `G1` tripwire does not create a protected state, leaving mutually exclusive implementations: accept one self-signable tripwire as these rows direct, or enforce the signerless two-review floor. Choosing the former opens the exact authorization downgrade the owner decision says must remain closed for signerless landings. Stale active prose at lines 515 and 824 further preserves the superseded protected-default rule.
  Recommendation: Make signerlessness an explicit Authority overlay evaluated after wire aggregation: every signerless team-mode landing must carry two distinct protected reviews regardless of wire class. Update the quick transition, G13, review grammar, and all stale default-threat prose to state the same rule. Alternatively, define and sign an authoritative quick-lane author statement so the landing is no longer signerless.
- [medium] C-M3 depends on retry history the design never persists (PLAYBOOK.md:385)
  The CAS-lost run is discarded and re-queued, while `C-M3` must count attempts across runs and make `starved` terminal until a human review resets it. No trailer, ref, result-header field, or other Git-native artifact records an unsuccessful attempt; gate reports are per-run, and optional notes are explicitly non-authoritative. A fresh process therefore cannot distinguish attempt one from attempt four without provider metadata or undeclared side state, violating the design's own rule against keeping such state. Implementers must either invent storage, reset the counter on every run, or deadlock inconsistently.
  Recommendation: Either delete C-M3/starved and allow unbounded externally managed re-queues, or define a Git-native retry/reset event whose identity, signature, transition rules, and reconstruction semantics are normative. Do not rely on CI run history or cache state.
- [medium] OKF remains described as shipped behavior after removal from v1 (PLAYBOOK.md:458)
  This active architecture section still says Spine ships “two graphs, one rendering and one table,” cites nonexistent §6.8, and says the rendering is counted in §10, while §10 says the rendering was removed and the budget returned to two. Section 12 then states that Spine emits the bundle and specifies `--okf`, and the residual-risk section claims `git check-ignore` enforcement and a manifest pin, but neither the normative CLI grammar nor the manifest schema contains them. A fresh implementer cannot determine whether v1 must build an exporter, destination guard, and pin or ignore them as proposal-only behavior.
  Recommendation: Remove all present-tense OKF behavior, `--okf`, manifest-pin, enforcement, and §6.8 references from PLAYBOOK.md. Retain only a short non-normative pointer stating that exported renderings are outside v1 and specified exclusively in `docs/proposals/okf.md`.

Next steps:
- Require final-tree signatures for default hostile-agent landing reviews.
- Resolve signerless quick-lane authorization consistently across §6, §7, and §11.
- Delete C-M3 or give retries a reconstructible Git-native record.
- Complete the OKF extraction and re-run a full restatement consistency pass.

