# Codex Final Review of the spine-kit corpus — model gpt-5.6-sol — 2026-08-27

Seventh Codex review, and the first of the **complete corpus**: PLAYBOOK.md v0.19 plus ten normative specifications (~14,100 lines). Asked the build question directly, with the security boundaries called out and the settled owner decisions marked off-limits.

Four highs, no criticals. The sharpest is finding 3: Swift ships with the same silent failure class that justified dropping Kotlin.

Verdict: needs-attention

Do not start implementation yet. The corpus still contains an every-landing failure for ordinary skipped tests, an explicitly unenforced security boundary, a silent Swift closure hole, and unresolved values that make `spine init` refuse its own CI render.

Findings:
- [high] A skipped test on trunk blocks every subsequent landing (docs/spec/import-resolver.md:1893-1895)
  The normative floor includes collected skipped tests, while G1/G8 require their candidate result to be `passed`. The specification explicitly confirms that an ordinary skipped test therefore raises G1 and G8 on every later landing until each landing receives a protected review. This is a shipped-configuration availability failure, not an edge-case ambiguity.
  Recommendation: Before implementation, exclude `skipped` outcomes from the B floor, or define a consistent skipped-on-B exemption in PLAYBOOK.md, result-file.md, import-resolver.md, and the gate-report vectors.
- [high] The advertised untrusted-stage egress boundary has no enforcement (docs/spec/result-file.md:351-366)
  The playbook promises registry-only network during restore and no network afterward, but M1 creates no network namespace or filter. The document states that runners can reach everything the job can reach and that no gate, precondition, header field, or isolation test detects this. Candidate code can therefore make arbitrary outbound connections while the result still records `profile=container`, violating the stated least-privilege boundary.
  Recommendation: Add an explicit dependency-restore phase outside runner execution, make M1 enforce loopback-only/no-egress networking afterward, and add a fourth probe that must pass before `profile=container` is emitted.
- [high] Swift ships with a silent Objective-C oracle hole (docs/spec/import-resolver.md:1869-1873)
  The resolver ignores `.m`, `.mm`, `.h`, and bridging-header dependencies in mixed Swift targets. An oracle or test helper in those files can change without entering the freeze closure, with no refusal or diagnostic. This is the same silent failure class used to justify dropping Kotlin, yet it remains OPEN for a language explicitly included in v1.
  Recommendation: Keep Swift in v1 but make `RC(swift, tree)` fail closed with `mixed-objc-target` whenever a resolved target contains Objective-C sources, headers, or a bridging header; add conformance vectors for each case.
- [high] `spine init` cannot produce a conforming CI render from this corpus (docs/spec/ci.md:64-68)
  The required distribution root and three GitHub Action commit pins remain symbolic. The same section requires that no `@@` or `PIN_` token survive rendering and requires `init` to refuse otherwise. An implementer must invent release inputs not supplied by the corpus, so first-run CI output cannot yet be byte-identical or usable.
  Recommendation: Freeze the HTTPS distribution root and exact 40-hex Action commits in a normative, versioned release manifest, and specify the canonical substitution input consumed by `spine init` before implementation begins.

Next steps:
- Resolve and vector the skipped-test floor semantics.
- Implement and test actual runner egress isolation.
- Add Swift mixed-Objective-C fail-closed detection.
- Publish normative release rendering inputs.
- Rerun cross-document and every-landing conformance review.

