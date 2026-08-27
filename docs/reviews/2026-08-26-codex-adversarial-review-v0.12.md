# Codex Adversarial Review of PLAYBOOK.md (v0.12) — model gpt-5.6-sol — 2026-08-26

Sixth review. Target: working tree (PLAYBOOK.md v0.12, ~1039 lines). Asked to check the four v0.11 closures, attack §6.8 on its second pass, and test the wire-aggregation rule against every landing shape.

No criticals. Codex also reported that the canonical OKF spec has moved to **v0.2** (breaking: `timestamp` removed, citations moved to a `sources` frontmatter family), which v0.13 adopts.

Verdict: needs-attention

No-ship: v0.12's central wire fix is not applied to every landing type, break-glass makes mixed protected failures unimplementable, and the new OKF freshness mechanism is both unauthenticated and non-conformant.

Findings:
- [high] Quick-lane landings still downgrade the default protected wire (PLAYBOOK.md:502-505)
  The protected-precondition row applies only from `tests-approved`, while the quick-lane rows route `C-M4` being off directly to `landing-review`. Under the shipped `C-A3: hostile` default, quick candidates should also produce the protected G1 precondition wire, but no quick-candidate transition does so. Section 11 can aggregate only wires that were produced; it cannot repair the missing transition. An ordinary quick change can therefore use a self-review instead of the two protected reviewers required for a signerless landing in team mode. The withdrawal row similarly reaches `withdrawn` without defining whether the supposedly universal default landing review applies to tombstones.
  Recommendation: Compute the complete wire set before lane-specific routing. Add an explicit quick-candidate row for failed §7.4 rule 5 preconditions that enters `protected-review`, and state whether tombstones run or are exempt from C-M4/G1. Test the default hostile case for gated, quick, tombstone, reseal, and lifecycle landings.
- [high] Break-glass precedence conflicts with the non-bypassable floor gate (PLAYBOOK.md:980)
  Section 11 says break-glass dominates protected wires and permits exactly one review state. However, G14 accepts only a `class=protected` review, and break-glass explicitly cannot bypass G14. A landing that both touches the floor and needs a G1/G8 break-glass override therefore has no valid representation: one break-glass review cannot satisfy G14, while retaining an additional protected review contradicts the single highest-class review rule. Implementers must either deadlock emergency floor landings or silently weaken the floor authorization requirement.
  Recommendation: Do not place break-glass in the wire-class precedence hierarchy. Define it as an override overlay: protected authorization and reviewer cardinality remain mandatory for protected wires, while a separately signed break-glass statement records only the bypassed gates. Alternatively, explicitly make break-glass satisfy G14 with identical or stronger reviewer-separation and signerless two-review rules.
- [high] The OKF manifest cannot prove freshness or detect deliberate edits (PLAYBOOK.md:744-746)
  The root freshness fields and `MANIFEST.md` are all mutable files inside the same unsigned bundle. A stale or modified bundle can update `spine_source_tip`, `spine_dump_sha256`, file contents, and their listed blob IDs together; comparison with the current trunk tip then succeeds. The document itself concedes that the bundle has no signature, hash, or attestation, which directly defeats the preceding claim that edited bundles are detectable and that each bundle proves its age. This protects only against accidental partial writes, not a misleading copied or rewritten knowledge bundle.
  Recommendation: Bind one canonical bundle-manifest digest to an authoritative signed Git object or detached signature, and specify a consumer-side verifier that checks the signature, source tip, dump digest, emitter identity, and every file digest. Otherwise narrow the claim to atomic-write and accidental-corruption detection and forbid describing the bundle as self-proving.
- [high] The prescribed root index is not a conformant OKF index (PLAYBOOK.md:742-744)
  The design requires four custom keys in the root `index.md` frontmatter, including `spine_okf_version`, but the current canonical OKF v0.2 specification permits root-index frontmatter only for the standard `okf_version` key; other index files have no frontmatter. The playbook also still describes OKF as v0.1. A conforming consumer or validator can reject or ignore the freshness fields, so the newest mechanism may fail precisely at the interoperability boundary it exists to serve. The adjacent uppercase `MANIFEST.md` is an ordinary concept document under OKF's case-sensitive reserved names and therefore also needs typed frontmatter, which the design does not specify. ([github.com](https://github.com/GoogleCloudPlatform/open-knowledge-format/blob/main/SPEC.md))
  Recommendation: Target and name an exact OKF version. Put only the standard `okf_version` key in root-index frontmatter, move Spine binding metadata into a conformant typed concept or explicitly versioned extension artifact, and define `MANIFEST.md` with the required `type` frontmatter or use a non-Markdown manifest format.

Next steps:
- Unify wire computation across every landing lane and event type.
- Redesign break-glass as an overlay rather than a dominating review class.
- Replace the unsigned OKF self-claims with a verifiable binding.
- Make the projection conform to a pinned canonical OKF version.

