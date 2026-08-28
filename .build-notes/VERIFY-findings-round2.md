# Adversarial verification — 2026-08-28 workflow (six units)

Two verifiers (lifecycle, report) died on connection errors and are absent.

Cleaned up; the tree is as I found it (144 tests, clippy clean, confirmed by re-run).

---

## Verification of `crates/spine-intent/` against the corpus

### 1. Fabricated values — none. Every published figure reproduces.

I recomputed all four vectors independently (`python3` + `hashlib`, byte counts, char counts, LF counts) and **also** diffed the held bytes against the fenced blocks in `docs/spec/intent-doc.md` and `docs/spec/templates.md`:

| Vector | bytes | sha1 blob | sha256 blob | sha256sum | spec-fence diff |
|---|---|---|---|---|---|
| `id-9.1-INT-042.md` | 1258 / 1249 chars / 26 LF | `1b9e7580…54be` ✓ | `1e594dc7…f39a` ✓ | `b9306483…1d99` ✓ | **byte-identical** |
| `id-9.4-INT-001.md` | 415 | `59deb402…a218` ✓ | `bbab2c9f…de69` ✓ | `66802409…acd0` ✓ | **byte-identical** |
| `tm-10.2-INT-043.md` | 1502 / 1490 / 32 | `89f6a976…8096` ✓ | `dc2cb930…9bf0` ✓ | `2c505283…dab2a`✓ | **byte-identical** |
| `tm-10.3-BUG-051.md` | 1096 / 1086 / 24 | `21328869…40b3` ✓ | `5f59718d…3879` ✓ | `d7d25fe6…9293` ✓ | **byte-identical** |

Non-ASCII censuses reproduce (`·`×3, `–`×1, `—`×2, exactly six for §9.1). ID §9.6's *published exhaustive* claim reproduces at full scale against the crate's own `overlap` and matcher: **2926 generated / 2926 accepted / 399 paths / 0 violations** — the spec's numbers exactly.

I also wrote an independent Python implementation of ID §6.1–§6.3 from the spec text and differential-tested it against `Pattern::parse` / `matches_str` over 445 patterns × 738 paths (including bracket, POSIX-class, globstar and dot-segment corners): **0 parse-status differences, 0 match differences.**

The `spine-template` dev-dep introduces no cycle (`spine-template` → canon + manifest only); `spine-gates` is the only reverse consumer.

### every-landing
**None.** Nothing I could construct produces a wrong G2/G7 verdict, a wrong digest, a wrong node id, or a silently-widened declaration.

### serious

**S1 — An indented touchpoint label line is accepted; §5.4 and §15 item 17 say it must be refused. `src/sections.rs:parse_label_line`.**

```
## Touchpoints
  Expected to change: src/          ← parses, yields expected=["src/"]
Must NOT change:
```
and the tab form `\tMust NOT change:` likewise parses. ID §5.4 is explicit: *"Every non-empty line must be a **label line**. Prose, bullets, AC lines and **continuations** are all `unknown-touchpoint-line`."* A line whose first byte is `0x20`/`0x09` **is** a continuation by §4.10's class test, and §15 item 17 restates it (*"and no other non-empty line"*). `parse_label_line` instead applies §5.4's `trim_matches([' ','\t'])` clause to the bytes before the colon, which swallows the indent and admits the line.

The corpus is genuinely self-contradictory here (under the strict `label-line := label ":" …` production, *both* strips are dead text), so a derivation is required — but the crate took the **fail-open** side of it, against the stated posture that a guarantee failing loudly can ship and one failing silently cannot. It is also the opposite of what the builder's own derived-notes claim: *"EXCEPT in a touchpoints body, where §5.4's explicit enumeration ('Prose, bullets, AC lines and continuations are all unknown-touchpoint-line') is total and overrides."* The code does not implement that.

Consequence is exactly ID §1's failure mode: a strict conforming parser refuses this document (no lease, exit 4) and this one accepts it (lease granted, `declares` edges emitted) — two binaries, two G7 verdicts, identical git objects. Note `  - Expected to change: x` and `  AC-1: x` are correctly refused; only an exactly-spelled indented label slips through, which is precisely the shape an author produces by accident.

### minor

**M2 — `empty-item` is unreachable and is the only status with zero test coverage.** A `Bullet(text)` with empty text requires the line `- `, which canon rule 9 refuses as `trailing-whitespace` at exit 2 first (verified: `- \n` → `8:trailing-whitespace`). The parallel case in `ac.rs` carries an explicit note (*"`AC-1: ` cannot occur — §2.1 rule 9 forbids the trailing space"*); `sections.rs::parse_bullets` carries none, so a reader cannot tell the branch is dead by design rather than untested. A scan of all 57 statuses shows `Status::EmptyItem` is the single one never named in any test.

**M3 — An out-of-range numeral is reported as `bad-id-padding`, which is not a padding failure. `src/header.rs:id_refusal`.** `# INT-9007199254740992: t` → `1:bad-id-padding`. The numeral is canonically padded; what failed is ID §3.1's `1 … 9007199254740991` bound. `bad-id` is the correct catch-all under the crate's own stated split. Same for any numeral that overflows `u64`. §3.1 names no mapping, so this is unspecified input — but the token reaches `unattested` counts permanently.

**M4 — `Spelling` is not exposed on `Parsed`, so ID §8.1's "the variant read from the header (§3.2), **not derived**" cannot be implemented by a `--sign` driver for a legacy document.** `check_signoff` takes `resign_floor: u32` pre-selected by the caller, and the only variant the caller can see is `parsed.variant`, which for a bare `Template: v<n>` value is the §3.3 **derived** variant. Latent only because no legacy document exists (no release has shipped), but the API offers no way to honour the clause and no note saying so.

**M5 — `canon::tests::the_document_bound_is_65536_bytes` never tests 65536.** It builds `ok` at exactly `MAX_DOCUMENT`, asserts only `ok.len() == MAX_DOCUMENT`, and never passes it to `check`; the comment "65536 bytes exactly is legal" is unverified. The real assertions are at 65530 (ok) and 65537 (refused), so the `≤` vs `<` boundary is untested. (The code is correct — `d.len() > MAX_DOCUMENT` — but the test does not say so.)

**M6 — `sections::tests::the_label_is_ascii_case_insensitive_and_the_typo_is_loud` contains three spelling-independent statements inside its spelling loop.** `let t = body(&["Expected to change: a"]).parse_touchpoints(); assert!(t.is_err());` builds and asserts the same value on all four iterations and exercises none of the four spellings. The `parse_label_line` assertions below it are real, so the test is not vacuous overall — but a quarter of its body is.

**M7 — Coverage gap on TM §4.5's legacy table: 4 of 8 rows untested.** `tm_4_5s_legacy_table_derives_the_variant_from_the_probe` covers rows 1, 2, 6, 7, 8. I probed the other three by hand and all match the published outcome, so this is coverage, not a bug: `v2` + Change sections with Invariants deleted → `4:unknown-section` (at `## Current behavior`) ✓; `v2` + Goal **and** Invariants → `4:unknown-section` (at `## Goal`) ✓; `v2` + Current only with Invariants → `missing-section` (`target behavior`) ✓. The qualified table's `intent-change@2` + Invariants deleted → `missing-section` ✓ is likewise untested.

**M8 — `looks_like_a_variant_token` is an invented shape grammar deciding a permanently-recorded token.** ID §3.2's two clauses are literally contradictory and the crate's `[a-z-]+` (no leading/trailing hyphen) test does classify every corpus example correctly (`Intent@2`/`INTENT-CHANGE@2` → `bad-template`; `chore@2` → `template-variant-unknown`). But a case-based reading — the distinguishing feature in the two examples the corpus actually gives — splits differently on unnamed input: `intent_change@2` is `bad-template` here and `template-variant-unknown` there. Same class and exit 4 either way, so only the token diverges. Worth a `DERIVED` note naming the rejected alternative, which the doc-comment does not.

**M9 — `d == b"\n"` is reported as `trailing-blank-line`.** ID §2.1 rule 8 states three clauses and names two statuses (`no-final-newline` / `trailing-blank-line`) without assigning which clause takes which; a one-byte document ending in its only LF is equally readable as either. Derived, unremarked in the code.

### Confirmed-correct where I expected trouble

- **The three cross-crate traps are all handled.** `tok` (not `esc`) for `G2:` wires, with a test pinning `tok(b"a b") = a\x20b` and `≠ esc`; `esc` for the `code_unit` node id, with a test proving both are the identity on legal patterns; wire **order** is correctly *not* produced here (ID §7.1 defers it to gate-report §6.1), so the byte-order/numeric-order permutation trap is out of scope by construction and `ac::labels_in_byte_order` documents it anyway. The trailing-LF rule is pinned in both directions: `tm_6_2_the_scaffolded_open_questions_body_is_empty` asserts a suffix ending in exactly one LF (a two-LF render fails that `ends_with`).
- **The §9.3 provenance off-by-one is a spec defect, not a code defect, and the crate is right.** In §9.1's own bytes line 22 is `## Touchpoints (expected blast radius)` and the label lines are 23/24; §9.3 publishes `:22`/`:23`. TM §10.2 publishes `:29`/`:30` and is correct against its bytes, fixing the convention as 1-based. `id_9_3s_published_provenance_line_numbers_are_off_by_one` names it and asserts (23, 24) — I confirmed both files' line numbering independently.
- ID §8.2's ten-step order, including step 4 before step 5 and step 7's *unknown → duplicate → missing → order* sub-order, holds under every two-fault document I built. `truncated` raised at step 2 rather than step 6 is observationally sound and reports the status §4.5 names.
- G2's forbidden-dominates precedence, the exempt set, byte-equality on `J.frozen`, dedup-keeping-first, byte-identical-only `polarity-conflict`, the as-written touchpoint bound, and `template_attr()` reconstruction all match the spec and the vectors.

### Verdict

**The crate is sound.** No digest, byte count, node id, gate predicate or ordering rule is wrong, and nothing I found can change a landing's verdict on a well-formed document. The one substantive defect is **S1**, a fail-open reading of a contradictory clause in ID §5.4 that lets a document parse here which a strict conforming parser refuses — a conformance divergence, not a miscomputation, and the one finding I would fix before this ships alongside a second implementation.

---

# Adversarial verification — `crates/spine-envelope/`

`cargo test -p spine-envelope`: **118 passed, 0 failed** (70 + 14 + 10 + 24 + 0 doc). `cargo clippy -p spine-envelope --all-targets`: clean, and there is not a single `#[allow]` in `src/`.

## 1. Fabricated values — none. Everything reproduces.

I recomputed every published quantity from `docs/spec/envelope-vectors.md`'s own fenced blocks with `hashlib`, `git hash-object` and `ssh-keygen`, then diffed the crate's fixtures against those bytes. All nine of the builder's claimed values are real:

| Claim | Recomputed |
|---|---|
| vector C sort, join 382, `sha256:bbf3ba10…f45d4` | ✅ my independent byte-sort of `c-authored` equals `c-sorted` exactly; 382; digest matches |
| vector A `freeze=` — 7 lines, join 573, `sha256:3a8fc309…44c2` | ✅ |
| EV §8.2's published **wrong** trailing-LF value `sha256:8262e6d9…19e9` | ✅ reproduces as the trailing-LF join |
| vector A `envelope=` — 15 lines, join 2379, `sha256:e1652897…682f` | ✅ |
| EV §8.3's published wrong value `sha256:a0c024c2…0ac5` | ✅ |
| vector A message 43 lines / 4031 bytes; cap 4031 | ✅ |
| intent blob 765 bytes / 762 chars / `dfb4079e…ea37` | ✅ `git hash-object` agrees |
| vector B — 7 lines, 859, `sha256:97648524…e44d`; 11 lines, 1636 | ✅ |
| vector D — 22 lines, 2954, `sha256:9895816b…fcc4`; 50 lines, 4606; capped 4032 (574 excluded) | ✅ (manifest = 567 bytes + 7 terminators) |
| three key fingerprints | ✅ `ssh-keygen -lf` reproduces all three |
| all nine signatures | ✅ `ssh-keygen -Y verify` — 5 on A, 1 on the approval commit, 2 on B, 1 on D |

Every fixture under `tests/vectors/` is byte-identical to the spec block it comes from; `d-message.txt`, which the spec does not print whole, is byte-identical to the derivation EV §11 states. `armor()` really is byte-identical to `ssh-keygen -Y sign`'s output — I generated a throwaway key, signed, and compared (70/70/70/34 wrapping, exact match).

The one *spec* text defect the crate flagged is genuine: EV §8.3 point 1's "The five `-Sig` lines are inside" — vector A's above-seal block carries **four**. The crate's comment in `tests/vectors.rs` is right, and no digest moves.

## `serious`

**S1 — `verify_line` writes the keyring to a fully predictable temp path, and follows symlinks.** `src/verify.rs:287` builds `$TMPDIR/spine-envelope-<pid>-<counter>` with `create_dir_all`, which returns `Ok` on a directory that already exists and sets no mode; there is no `O_EXCL` and no random component (the counter starts at 0). I demonstrated the consequence in-process: pre-create that directory, place `allowed_signers` as a symlink, call `verify_line`, and the spine-supplied keyring bytes land on the symlink target. On Linux CI — where `TMPDIR` is usually unset and `/tmp` is shared, which is exactly where PB §7.4's trusted stage runs — an attacker who pre-creates the directory *owns* it and can `rename` their own file over `allowed_signers` between our `fs::write` and `ssh-keygen`'s open, i.e. verify a forged statement against a keyring of their choosing. This is the one code path in the crate that decides authority, and PB §7.2 rests the whole trust model on it. Fix: `mkdtemp`-style `O_EXCL` creation with mode 0700, or pass the keyring via `/dev/fd`.

**S2 — a signed statement with no `-Sig` line is accepted.** `src/message.rs:436` checks adjacency only from the `-Sig` side ("this `-Sig` follows what it signs"); nothing checks that a statement which takes a signature *has* one. EV §2.4's table pairs them (`Spine-Signoff` + `-Sig`, one cardinality cell) and §2.7 defines a signed statement as the line **plus** its `-Sig`. Demonstrated on vector A: delete `Spine-Signoff-Sig`, reseal with the recomputed digest, and the result parses, `check_envelope_digest()` passes, `check_subject()` passes — a gated landing whose human sign-off carries no signature, and nothing in the crate says so. The asymmetry is the tell: the seal pair *is* enforced (by the exactly-one rule at `message.rs:401`), the other six are not. Same hole for `Spine-Approve`, `Spine-Review`, `Spine-Reopen`, `Spine-Withdraw`, `Spine-Upgrade` (I confirmed Review and Upgrade directly).

**S3 — case near-miss `Spine-` lines are never refused.** EV §2.3 is explicit: "`spine-seal: …` and `SPINE-SEAL: …` … G9 refuses a landing containing either (`envelope-malformed`) rather than leaving a near-miss spelling that a sloppy reader might honour." `is_spine_line` correctly excludes them from the digest, but no check ever refuses them. Confirmed: appending `spine-seal: forged base=nowhere envelope=sha256:0` or `SPINE-REVIEW: forged reviewer=mallory` below `Spine-Seal-Sig` is **ACCEPTED** and lands in `foreign_trailers()` (`message.rs:124`); a subject of `spine-seal: x` is also accepted. This is precisely the fail-open the spec names its rationale for.

**S4 — the fence's `blob=` is never bound to the sign-off's.** PB §6.3 G9: "a gated `Spine-Event: land` envelope carries a verifying `Spine-Signoff` **for its blob** and a verifying `Spine-Approve` **naming that blob**", and for a tombstone "a verifying `Spine-Withdraw` whose `blob=` the fenced bytes hash to". The fence self-check (`parse_fence`) proves the body hashes to the fence header's own `blob=` and stops there. I tampered vector A's `Spine-Signoff` to `blob=0000…0000` and `Envelope::parse` **ACCEPTED** it. This check needs no git object — both values are in the message — so it is not covered by the builder's "cross-object checks need a repository" exclusion, and it is not on the not-implemented list. I checked the rest of the workspace: `spine-gates` has no G9 module, and `spine-graph/src/derive.rs:1549` says of its own envelope reader "It **validates nothing**". Nobody performs it. The same applies to `Spine-Approve`'s `intent=`.

**S5 — `envelope_digest` silently applies "first one wins" to a two-seal message.** EV §3.4 spells this row out and names the wrong answer: "Two `Spine-Seal` lines → `envelope-malformed`. **Not 'the first one wins'.**" `digest::envelope_digest` (`src/digest.rs:104`, re-exported at the crate root) refuses the *no-seal* row of the same table — its doc comment defends that as a structural judgement — but returns a digest for the two-seal one, from `above_seal`'s `break` at the first seal. Confirmed: appending a second `Spine-Seal:` to vector A returns vector A's digest; inserting one *above* the real seal returns a different digest, both `Ok`. `Envelope::parse` does refuse, so a caller going through the type is safe; a caller using the exported function is not, and the crate refuses one row and not its neighbour for no stated reason.

## `minor`

**M1 — panic on a hostile fence header.** `src/message.rs:619`, `message.get(*pos..*pos + bytes)`: `bytes=18446744073709551615` panics with `attempt to add with overflow` in any overflow-checked build (which is what `cargo test` runs), instead of `fence-mismatch`. Release wraps to a reversed range and returns the refusal by luck, but `panic = "abort"` is set in the workspace profile, so a checked build aborts on a commit message an attacker fully controls. Use `checked_add`.

**M2 — a trailing blank line is accepted.** EV §2.1 ("There is no trailing blank line") and §18 item 1. `message.rs:76` checks only that the last byte is `0x0A`; a blank line after `Spine-Seal-Sig` is swallowed as an empty "foreign trailer". Confirmed ACCEPTED.

**M3 — a UTF-8 BOM is accepted.** EV §2.1 says "no BOM"; nothing checks. It ends up inside the subject.

**M4 — payload values PB §11 does not admit.** `held=true` parses and `Approve::render` will emit it (`payload.rs:369`), though PB §11 spells only `[held=false]`; `rounds=99` parses though PB §11 spells `rounds=0..2`; `git=hello` parses though PB §11 spells `git=<major.minor>`. All three are lines no conforming emitter writes, and the crate's own posture elsewhere ("emission by position, parsing by key, refuse anything else") argues for refusing them.

**M5 — two tests do not test what their names claim.** `tests/signatures.rs::the_armor_round_trip_is_byte_identical_to_what_ssh_keygen_writes` never compares against `ssh-keygen` output; its body asserts `l.len() <= 70`, which holds for any wrap width. (The claim is true — I verified it against a real signature — and `verify.rs`'s unit test does pin 70 exactly; the integration test just overclaims.) `tests/refusals.rs::a_reseal_carries_no_fenced_intent_and_no_signoff` asserts the absence of two things its own fixture never added — true for any implementation.

**M6 — `fingerprint_of` defaults to `""` silently** (`verify.rs`). PB §7.2's "reviewer ≠ signer compares fingerprints" then compares empty strings. Both known comparisons fail *closed* under that degradation, so it is not exploitable, but a security-relevant field defaulting rather than erroring is the wrong shape for this corpus.

**M7 — unused dependencies.** `spine-manifest` and `spine-report` are declared in `Cargo.toml`; no `spine_manifest`/`spine_report` reference exists anywhere in the crate.

## Checked and clean

- **The three cross-crate traps are all handled correctly.** `cmp_wires` is `a.as_bytes().cmp(b.as_bytes())` and `parse_wires` **refuses** a numerically-ordered `wires=` rather than re-sorting; the byte counts that a permutation would preserve are pinned alongside the digests. `quote.rs` is a standalone C-quoter that calls nothing in `spine_canon::esc`, and `a_c_quoted_path_is_never_the_wire_encoding` asserts both encodings of `café.json` in one test — I verified `quote_path` against git's real escape table byte by byte, including the `\a`/`\v` names, three zero-padded octal digits, and a 0–255 round trip. Both joins carry no trailing LF and both wrong values are *recomputed*, not transcribed.
- Cardinality per name rather than per rank (the bug the builder says vector A caught) is right, and rank 5 sharing is the reason.
- The seal boundary is the 12-byte `Spine-Seal: ` prefix; `Spine-Seal-Sig` falls below it with no second rule.
- The reseal cap exemption is read from `Spine-Event` and is genuinely unmeasured; the non-reseal path refuses at >16384.
- Tombstone and lifecycle shapes both parse and derive correctly (I built one of each; neither is covered by a test — worth adding, but they work).
- `derive_subject` takes the whole first line minus `# `, per EV §13.10's "derivation, not a template", and the reseal form carries the full oid.
- `expected_namespace`'s three conditional selectors (`run=`, `mode=recovery`, `orphaned=`) match PB §11 and §7.5, and the `orphaned=` derivation is the fail-closed direction as documented.

## Verdict

**The crate is sound as a digest engine and unsound as a fail-closed structural validator.** Every value it publishes is real, every encoding is right, every vector reproduces from the bytes, and the three traps that would silently produce a wrong digest are all avoided. But `Envelope::parse` is sold as EV §18 items 3–9 and it lets through three shapes the spec refuses by name — an unsigned authorising statement (S2), a near-miss `Spine-` spelling (S3), and a fence whose blob the sign-off does not name (S4) — plus a two-seal digest the spec forbids in as many words (S5). S1 is independent of the spec and is the worst of them: the authority check writes its trust root to a guessable path. I would not ship it past S1, S2 and S3.

---

Confirmed. Writing up.

**VERDICT: the crate is NOT sound.** 153 tests pass and clippy is clean (both builder claims verified), and the two headline vectors reproduce — but there is one every-landing defect, six serious ones, and the corpus's own published flagship report cannot be produced through the crate's only status function.

---

# What reproduces (checked, not taken on trust)

- **MF §8.4's G14 run** — reproduces exactly: seven diff entries, six hits one clause each, `src/billing/invoice.py` clean, the six tokens in byte order, override iff one protected review names all six. `/Users/thettwe/Works/spine-kit/crates/spine-gates/src/g14.rs:534`.
- **F0** — I diffed `F0_PATTERNS` against MF §5.5's table programmatically: **byte-identical, 17 for 17**, including `**/.claude/` (MF) rather than PB §7.3's `**/.claude/**`.
- **The only literal digest in the crate**, `sha256:980d4cb66bc03353cdb93d9149ead2ec7aae73c8e1ab6ade536eb8628acd0753` (g1.rs:305, g16.rs:567), recomputed: `python3 hashlib.sha256(b"spine-gates-test-artifacts")` → `980d4cb6…0753`. Matches; the "26 ASCII bytes" count is also right.
- **1 209 600** = 14·24·60·60, and `g3_is_stale` uses strict `>`, which is what "exactly" requires.
- **`wires=G11,G2:src/shared/util.ts`** is produced by the comparator, not asserted, and `G11` sorts *between* `G1` and `G1:a.py` — the byte-order trap is handled correctly, `tok` is a single pass, and `WireSet::ordered` keys on `tok` while `floor_hits` sorts on `esc`.
- **Gate sets per shape**: 14 / 4 / 11 / 11 — matches PB §11's "fourteen … eleven" and GR §5.6.2 including the tombstone's exact `{G9,G13,G14,G15}`.
- The trailing-LF trap has no surface in this crate (no artifact is serialized here).

---

# EVERY-LANDING

### 1. G15 never consults the running binary. Any version passes, on every landing shape.

`/Users/thettwe/Works/spine-kit/crates/spine-gates/src/g15.rs:106`

```rust
(Ok(list), Some(target)) => {
    if list.for_target(target).is_none() {
        findings.push(Finding::outright(G15Status::ArtifactNotListed));
    }
}
```

`for_target` asks only *"does the pinned list contain some artifact for my platform?"* — true for every release, for every binary. `G15Input` has **no field describing the running binary** except its target triple: no version, no self-digest. `ArtifactList::version()` and `ArtifactEntry.sha256` both exist in `spine-manifest` and neither is read.

PB §6.3 G15: *"The **running binary's platform artifact** is listed in trunk's pinned `dist_hash` artifact list."* PB's skew table (PLAYBOOK.md:767) is explicit: `newer | … | **fail** (G15) — CI runs the pinned hash or nothing`. CI §5.5: *"The binary independently verifies its own bytes against the same list at start-up."* A 1.3.0 or 1.5.0 laptop binary against a 1.4.0 pin fetches the 1.4.0 list, hashes it to the pin, finds `spine-1.4.0-aarch64-apple-darwin.tar.gz`, and reads `G15=pass`. G15 is on no bypass list precisely because nothing else catches this.

**The test that claims to cover it proves the opposite** (g15.rs:196, `a_newer_binary_is_not_a_greater_binary_it_is_an_unlisted_one`): its input is byte-identical to `a_listed_platform_artifact_passes_and_raises_no_wire`, it asserts `Pass`, and its own comment concedes *"the version is never consulted."* The module docstring meanwhile asserts *"the 1.5.0 artifact is simply absent from the 1.4.0 list"* — which the code never checks. That is a test whose name claims a rule its body inverts.

Also missing and **not on the builder's not-implemented list**: PB §6.3 G15's seal clause (*"the seal verifies under `spine-seal@v1` … and its `tool=` equals the pin (or, on a solo or `mode=recovery` seal of a rollback, uninstall or re-init landing … that line's `to=`)"*) and PB §6.7's `--rollback`/`--uninstall`/`--status` exemption from the version gate. `G15Input` cannot express either.

---

# SERIOUS

### 2. `decide` implements only the *protected* discharge, so `G2=override` — the corpus's own flagship report — is unreachable.

`/Users/thettwe/Works/spine-kit/crates/spine-gates/src/verdict.rs:155`

```rust
} else if findings.iter().all(|f| {
    f.wire.as_ref().is_some_and(|w| reviews.protected_names(&w.token()))
}) { GateStatus::Override }
```

GR §5.6.1 limb (a): *"covered by a signed review **whose class admits that wire**"*, and PB §6's transition table discharges `landing-review` with a `class=tripwire` review. GR §8.2's published sealed report reads `{"gate": "G2", "status": "override"}` over bob's `class=tripwire` review. Passed to `decide`, that landing reads `fail` → `report-not-landable`.

This is not hypothetical dead code: `g1.rs`'s `Coverage` doc tells callers *"A caller assembles them with `crate::verdict::decide`"* for G8's six other clauses — and G8's harness-moved clause is `class=tripwire` (GR §6.3, `G8Status::class()` returns `Tripwire` for it). `decide` is re-exported at the crate root as the general rule and its docstring quotes the general rule. The tripwire limb has no test anywhere; `landing.rs:36` builds a `ReviewClass::Tripwire` review and checks only `contain`, never a status.

### 3. G8's clause-2 wire takes the *result* record's path; RF fixes it as `G8:<b.path>`.

`/Users/thettwe/Works/spine-kit/crates/spine-gates/src/g1.rs:198`

```rust
let path = result_path.unwrap_or(b.path.as_str());
let g8_token = Wire::at(Gate::G8, path, ...).token();
...
wire: Some(Wire::at(Gate::G8, path, G8Status::LandedId.class(), WireKind::Finding)),
```

RF §8.5 clause 2 writes `G8:<b.path>` three times and RF §13 R19 a fourth: *"Otherwise — that is, for the went away shape whatever `b.out` says, **and for the did not pass shape** where `b.out` is neither `"xfail"` nor `"skipped"` — both are a **G8** finding `G8:<b.path>`"*; *"Both gates fail: G8 on `G8:<b.path>`, G1 on the pair."* The `result`-record path is RF's rule for **G1's** token only (the "Per clause" list is introduced by *"A per-id finding takes `G1:` + `tok(path)`"*).

RF explicitly legalizes the disagreement: *"Where the two records for one pair disagree on `path`, that is not an error and neither record is rejected."* So on a did-not-pass id whose result path ≠ base path, this crate writes `G8:new/t.py` where a conforming implementation writes `G8:old/t.py` — a different `wires` array, `report=` and `envelope=` over identical objects, and a protected review naming the conforming token discharges nothing.

**The crate constructs exactly that case and does not check it.** `the_path_comes_from_the_result_record_where_one_exists_and_the_base_record_otherwise` (g1.rs:692) builds `base path=old/t.py` / `result path=new/t.py` and asserts only `out.g1.wires.tokens()`. `a_per_id_finding_takes_g1_plus_tok_of_the_path` (g1.rs:352) does the same with `t.py` vs `t b.py`. No test in the crate asserts a G8 token on the did-not-pass shape.

### 4. G16 gives wires to some outright findings and not others — a short `wires` array.

`/Users/thettwe/Works/spine-kit/crates/spine-gates/src/g16.rs`: checks 1, 2–8, 10 (two of four limbs), 12b and rollback steps 5 use `outright_with_wire`; **20 other outright statuses use `Finding::outright` with `wire: None`** — `upgrade-version-mismatch`, `forced-disagrees`, all five `constitution-*`, all five `uninstall-*`, all three `reinit-*`, and six `restore-*`.

GR §6.3's G16 row assigns the gate a token unconditionally: *"`G16:` + `tok(path)` where a path is implicated, **bare `G16`** where none is."* GR §5.6.1: *"**Outright is a coverage rule, never a containment rule** … [containment] includes every entry of the array, **outright findings among them**. So a landing that carries an outright wire and reaches a review state at all needs that wire **named**."* G14 is the one gate the corpus exempts (*"one per `floor_hits` entry and no other `G14` entry"*), and `g14.rs` argues that position at length — while `g16.rs` silently takes the other one.

`verdict.rs:78`'s doc justifies `wire: None` with *"every outright G16 check 1–8 failure, for instance, refuses the run before a wire could be read by anyone"* — which is (a) contradicted by GR §5.6.1 (*"It is the report a reviewer reads and binds with `report=`"*) and (b) contradicted by `g16.rs` itself, where checks 1–8 **do** attach `at(".spine/manifest.json")`.

### 5. Path-encoding boundary: esc-encoded corpus values are fed into raw-byte comparisons and into `Wire::at`.

`Wire`'s contract (wire.rs:88) is emphatic: *"`path` holds **raw bytes**, never an encoding."* But:

- `G14Input.e_m_b: Vec<Vec<u8>>` is documented as *"`E(M_B)` — MF §3.4's flattened `paths.*` value set at `B`"*. MF §2.3: *"every value of every `paths.*` key"* is **`esc`-encoded**. `spine_manifest::Manifest::floor_entries()` returns exactly those esc strings. Nothing in `g14.rs` mentions `unesc`, and `literal_match` compares them byte-for-byte against **raw** diff bytes. A `paths.constitution` of `caf\xc3\xa9/CONSTITUTION.md` reaches `lmatch` as 12 esc characters and never matches the 10 raw bytes — **the literal floor silently misses that path**. Fail-open on the floor.
- `ScaffoldObservation.path: String` is documented as *"keyed by the record's own `path`"* — i.e. `files[].path`, esc-encoded (MF §2.3, §3.5). `at(&observation.path)` then yields `G16:` + `tok(esc(p))`, doubly escaped for any `\` or non-ASCII byte. Same for `Rollback.paths_not_restored` / `paths_not_deleted` (drawn from `files(A) ∪ files(M_B)`).
- Both are `String`, so a non-UTF-8 repository path — which DM §2.4 makes first-class — is unrepresentable at all.

The crate does it correctly exactly once: `parse_forced` calls `unesc`. The asymmetry is the tell.

### 6. `Review` cannot express `head=`, `tree=` or `base=`, so every `override` is granted without the binding.

`review.rs:44` — `Review { class, fingerprint, self_approved, wires }`. MF §5.10 and §4.8.6 both require *"each verifying under `spine-review@v1` against the keyring at `B`, **each carrying `head=Hc` and a `tree=` equal to the tree under evaluation**, each by a reviewer eligible under §4.5"*, and PB §6 has two whole rows on it (*"`H ≠ review.head`, or `merge-tree(review.base, H) ≠ review.tree` — the branch changed → same state, **review void**"*, plus the base-move retention rule and its *"the set carries no pathless wire"* clause). None of this is representable, none is checked, and none is on the builder's not-implemented list. A stale review discharges a current finding.

### 7. G13 check 2 raises a spurious wire for any *verifying* unknown trailer, and the comment says it cannot.

`g13.rs:96` `Trailer::Other(_) => &[]`, with the comment *"check 2 reaches it only through `Verification::SignatureFailed`."* False — `g13.rs:303`:

```rust
Verification::Ok { namespace, .. } => {
    let admitted = commit.trailer.required_namespaces(input.recovery_seal);
    if admitted.contains(&namespace.as_str()) { continue; }
    G13Status::StatementNamespace
}
```

`[].contains(_)` is always false, so a hand-made `Spine-Foo` line whose signature **verifies** produces `statement-namespace` + a `class=protected` `G13:<oid>` wire, promoting the landing to `protected-review`. MF §4.8.3's table has no row for such a trailer and §4.8.4 describes the coverable branch as being for a *failing* line (*"noise a human may accept"*). The only `Trailer::Other` test (g13.rs:602) uses `Verification::SignatureFailed`; the `Ok` path is untested.

### 8. G13 check 11 sums the wrong approvals — and re-arms the key-rotation brick.

`g13.rs:418`:

```rust
Payload::Approve(a) if a.freeze != approve.freeze => Some(a.rounds),
```

MF §4.8.4 check 11: *"`total_rounds=` equals its own `rounds=` plus the `rounds=` of every **earlier verifying** `Spine-Approve` in `E`."* This filter enforces neither conjunct: it sums approvals **later** in `E`, and it sums **void and non-verifying** ones. A signer whose key rotated out of `K` leaves a void `Spine-Approve` that check 2 correctly skips — and check 11 then counts its `rounds=`, producing a spurious `total-rounds-mismatch`. That is precisely the brick MF §4.8.2 says voiding exists to prevent (*"rotating a signer's key mid-flight would turn an append-only branch's own sign-off into an outright refusal"*). In-flight only, so no digest moves, but it refuses runs.

---

# MINOR

9. **`g14.rs:340` `Err(FloorPatternError::Dialect(_)) => {}`** silently drops a `C-A2` pattern the ID dialect refuses. The comment's fail-closed claim rests on CN owning the parse — but CN §7.1/§7.2 make a malformed `C-A2` value take the **default `["**"]`**, i.e. everything becomes a floor hit. "Drop that one pattern" is the opposite direction. Unreachable for conforming input; the fail direction is still inverted.
10. **`diff_size_fires(_, None) == false`** (C6). Declared, but PB §5.2 (*"Diff size under `C-Q2`"*, stated as a condition of every green pipeline) and PB §6.3's G2 row arguably do fix the bound as `C-Q2` on every lane. As written the sub-check silently never fires when the caller omits a bound — a guarantee failing silently.
11. **`has_uppercase_in_bracket`** reports `c-a2-bracket-case` for an unterminated `[` containing an uppercase letter; ID §6.2 makes that `bad-bracket`, CN's refusal. A G14 status token for a finding G14 does not own. (Partially declared.)
12. **G13 check 5** resets `freeze_before = None` after a reopen, so two consecutive reopens require `voids=none` on the second. Defensible reading of *"the approval **binding** immediately before it"*, but it contradicts *"`voids=none` exactly when no approval preceded it"* and is undeclared.
13. **`is_binding` and check 11 identify approvals by `freeze=` equality**, not by position in `E`. Two approvals sharing a `freeze=` mis-resolve; MF's predicate is positional.
14. **`monotone_union` is dead** — no evaluator calls it (`paths_are_the_monotone_union` is a caller-supplied bool), and it implements only §6.7.1's per-key value union, not the key union or §3.4's canonical shape its own docstring quotes.
15. **`only_protected_is_a_g7_hard_lease`** returns `true` for two protected G7 entries; GR §6.1's literal *"no **other** entry has `class == "protected"`"* says false. Read-time counter, no digest.
16. **`G8Status::class()`** defaults to `protected` for the three clauses GR §6.3 does not name (`differs-from-both`, `intent-blob-differs`, `closure-not-contained`). Fail-closed and probably right, but undeclared.
17. **`g16.rs:337`** `input.manifest_blob_in_t.unwrap_or_default()` — a `Spine-Upgrade` with an empty `manifest=` passes check 10's agreement when the caller supplies `None`.
18. **`tripwires.rs:113`** `a + d` can wrap in release builds.

---

# Bottom line

The wire layer — the part the corpus warns about hardest — is the strongest thing here: byte-order comparator, `tok`/`esc` separation, collapse precedence, the `(gate, path)` key and the empty-path rule are all correct and genuinely tested, and MF §8.4 reproduces. The failures are at the crate's **edges**: what a gate is allowed to read about the running binary (#1), what encoding a caller's path is in (#5), what a review binds (#6), and which review class discharges what (#2). Four of the seven top findings are places where a `String` or a missing field quietly decides a digest.

Not shippable as is. #1 and #2 are the two that must be fixed before any report assembler is built on top of this, because both are silent: G15 says `pass` where the corpus says `fail`, and `decide` says `fail` where the corpus's own worked example says `override`.

---

I verified every published digest independently, read the specs, and read the crate. Here is what I found.

---

## FABRICATED VALUES: none

Every number the builder reported reproduces. Recomputed from the spec's own bytes:

| Vector | Reported | I got |
|---|---|---|
| DM §12.2 dump | 62 lines / 14054 B / `sha256:3321e7bd4b51…12da` | 62 / 14054 / `3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da` ✓ |
| DM §12.4 fragment | 1063 B / `a849ec34…` | 1063 / `a849ec349ef8f20ec1f40423ae6a7d3358745f4c9027545f55cf74ef9b72a139` ✓ |
| DM §12.5 empty dump | 105 B / `2a3fcea5…` | 105 / `2a3fcea5debff8e17af36eef1a4acfa1fea9f86a0d6c942cf2993cce3e80f290` ✓ |
| EV §8.3 `envelope=` | LF-join, no trailing LF | 15 lines / 2379 B / `e1652897b251b001fe7e03e343d40bbdc7fb9b112ef920c8b53987916b14682f` ✓ ; trailing-LF variant = `a0c024c2…` ✓ |
| DM §12.1 approval ids | `2f5e6002…` / `b6352921…` / `ae8a4063…` | all three reproduce byte-for-byte ✓ |
| Signer fingerprints | `dDNTLP8T…` / `V2dasTIG…` / `eQ0ZoC+r…` | `ssh-keygen -lf` over EV §8.1's keys reproduces all three ✓ |

`tests/vectors/*.jsonl` are byte-identical to the spec fences. `cargo test -p spine-graph`: 98 passed (61+12+7+8+4+6). `cargo clippy -p spine-graph --all-targets`: clean.

---

## every-landing

**1. `intent.late_reopen_count` is structurally always 0, and `approval.voided_by`/`void_reason` are unreachable.**
`crates/spine-graph/src/derive.rs:806-816` counts copied `Spine-Reopen` lines whose `voids=` equals the copied `Spine-Approve`'s `freeze=`. PB §11 (line 342) defines the binding approval as *"the newest `Spine-Approve` on the branch whose `freeze=` **no** `Spine-Reopen` names"*, and PB §5.5's envelope block copies exactly that one approve plus every reopen. So the predicate is false by construction on every conforming landing: `late` is 0 forever, and the same predicate at `derive.rs:1010-1027` means `voided_by`/`void_reason` are never emitted and `carrier_of` is dead code.
The DERIVED note claims *"it is the only rule available: EV §2.4 emits every reopen above the approve line so position cannot separate them."* That is about **envelope** position; PB §11 states the rule in terms of **commit** position — *"late reopens — those with implementation commits between the voided approval and the reopen"* — and the crate already walks `M(L)`, already resolves the approval commit from `Spine-Approval`, and already has `carrier_of` to locate a reopen's commit. The rule the corpus gives is computable and is not the one implemented. PB §9 sells this counter as the countermeasure for *"quietly reopening to weaken ACs"*; it is dead.

**2. `DUMP_VERSION = 1` is claimed while the projection is knowingly not DM §12.2's.**
`derive.rs` emits no `verified_by` edge (`grep VerifiedBy crates/spine-graph/src/derive.rs` → nothing; the only occurrence is the hand-built fixture at `tests/myrepo/mod.rs:348`), and cites `test` nodes as `git:<sha>:trailer:Spine-Test` instead of DM §5.4/§12.2's `git:<L>:<path>:<line>`. DM §8.3 lists `verified_by` as **yes**. DM §3.4 is a requirement, not a nicety: *"two releases carrying the same `dump_version` and `schema_version` **must** produce identical bytes over identical objects … a release that changes the projection **must** bump `dump_version`, even for a change it believes is a bug fix. A silent projection change is a fleet-wide `reconstruction-failed` on the first landing after a rolling upgrade."* This binary indexes DM §12.1's repository into bytes that are not DM §12.2's, and stamps them `dump_version: 1`. The omission is disclosed; shipping it under version 1 is the defect.
Consequence: `tests/dm_12_2_dump.rs` — the flagship vector test — runs against a graph containing `verified_by` edges and `test` citations that `derive.rs` cannot produce. It proves the serializer, not the crate.

---

## serious

**3. `changeset.seal_verified` accepts any namespace — fail-open on the one attr the builder calls fail-closed.**
`derive.rs:557-566` calls `namespace_that_verifies` and takes `.is_some()`. `verify.rs:71-79` tries all three of `NAMESPACES`. So a seal signed under `spine-signoff@v1` or `spine-review@v1` yields `seal_verified: true`, `unattested: false`. PB §7.2: *"In team mode G13 refuses a keyring in which a human key is also listed under `spine-seal@v1` … and any later human seal is `unattested` — except the recovery form of §7.5."* PB §11 G15: *"the seal verifies under `spine-seal@v1` (or the recovery form, §7.5)."* The crate applies no namespace constraint and no `mode=recovery` condition. The approval path does capture the namespace (for `role`); the seal path throws it away.

**4. Signer re-key is invisible; the retired fingerprint is reported as current.**
`derive.rs:333-336` matches an existing life by `l.principal == entry.principal` alone. MF §4.6: *"`valid_from` := the trunk commit at which this **`(principal, key)`** first appears … A line edited in place (same principal, new key) is a removal and an addition: the old fingerprint gets a `valid_to`, the new one a `valid_from`."* The doc comment directly above the code quotes that sentence. After a rotation the node keeps the old `valid_from`, `derive.rs:373-379` recomputes the fingerprint *at that old commit*, and no `valid_to` is set — the dump asserts a superseded key is still live. There is no test for `valid_to` anywhere (`grep valid_to crates/spine-graph/tests/` → one doc comment).

**5. `effective_c_a2` is not `effective(C-A2)`.** `derive.rs:1451-1468`:
- **No CN §2.3 preprocessing.** CN §2.3 exists precisely so *"a Windows checkout commits a CRLF constitution and a strict reader would make that repository unable to evaluate a single gate."* The code splits on `\n` and `trim_ascii` strips only `0x20`/`0x09`, so a CRLF constitution yields pattern bytes `infra/\r` → node id `myrepo/code:infra/\x0d` and a `protects` edge to a path that does not exist.
- **No key check.** CN §7.2 makes `rule-key-mismatch` take the fail-closed default `["**"]`; the code splits at the first `=` without ever checking the key is `protected`, so `C-A2: mode = team` emits `code:team`.
- **No duplicate handling.** CN §7.2: duplicated ⇒ `["**"]`. The code unions both lines' patterns.
- **No pattern validation.** CN §5.5: *"a malformed member pattern makes the **whole list** malformed rather than dropping the member"* ⇒ `["**"]`. The code emits every comma field verbatim.
Only the *absent* case was disclosed as DERIVED. The other three under-report the protected floor — the wrong direction.

**6. Oid- and digest-valued attrs on edges are never validated, and the node check names an attr that does not exist.**
`dump.rs:352-361` — `OID_VALUED_ATTRS` contains `"freezes_oid"`. No attr anywhere is called that: the real name is `oid` (`schema.rs:307`, `derive.rs:1144`). And `OID_VALUED_ATTRS` is only consulted from `check_node_attr_domains`, which runs on nodes; `check_edge_attr_domains` (`dump.rs:479-517`) has no oid check at all. So `freezes.oid` — which DM §7.2 types as `oid` and DM §17 item 5 covers — is unchecked, and `parse_frozen` (`derive.rs:1477-1480`) accepts any UTF-8 token before the first space. The crate's own comment states the stakes: *"An abbreviated or uppercase oid compares unequal to every id git produces, so it is a value that can never match anything and can never be noticed either."* Same gap for DM §10 rule 10's non-git digests: `changeset.report_sha256` and `approval.freeze` (`sha256:` + 64 lowercase hex) are accepted as any ASCII string — `tests/derive_shapes.rs` uses `report=sha256:00` throughout and nothing objects. `tests/dm_17_conformance.rs`'s item-5 checker also walks node attrs only, so the test has the same blind spot.

**7. DM §7.2's presence column is enforced for nodes and not for edges.**
`NodeKind::always_present_attrs` exists (`schema.rs:102-122`); there is no `EdgeKind` equivalent, and `check_edge_attr_domains`'s guards are written `if let Some(AttrValue::Bool(true))` — absence passes. So `implements` without `role`/`provisional`/`verified`, `protects` without `floor`, `declares` without `polarity`, `reverts` without `partial`, `verified_by` without `attributed` all serialize cleanly, and DM §17 items 16/17 are satisfiable by omission. Likewise `changeset`'s *"iff `landing` is `true`"* block (`lane`, `event`, `strategy`, …) is unenforced: only the `landing: false` ⇒ exactly-one-attr direction is checked (`dump.rs:426-433`).

**8. `reverts` is keyed by intent id, and pathspecs are not escaped.** `derive.rs:661-666` records `(intent_id, R)` and drops any reverted landing with no `Spine-Intent` — so a quick-lane or toolkit-lifecycle landing can never be recorded as reverted, though PB §6.2's rule is about landings, not intents, and DM §5.3 makes `reverts` changeset→changeset. `derive_reverts` then re-finds the reverted landing by `first` match on intent id, so a re-landed intent points the edge at the wrong changeset. Separately, `git.rs:280-288` pushes `String::from_utf8_lossy(path)` into `git diff -- <paths>` with no `--literal-pathspecs` and no `:(literal)` prefix, while its own comment says the path *"is dropped from the restriction"* — it is not dropped, it is mangled. A repo path containing `*`, `?`, `[` or a leading `:` is then a glob or pathspec magic: the first two widen the restriction, the last makes `git diff` fail and aborts the whole derivation. The claim *"it can only ever miss a revert, never invent one"* is not established by this code.

**9. DM §10 rule 12's pin is incomplete, and the indexing subprocesses read global config.**
`PINNED` (`git.rs:44-51`) fixes `diff.algorithm`, `diff.renames`, `core.quotePath` only. `patch_id` runs a full `git diff`, whose output — and therefore `git patch-id --stable` — still moves with repository config: `diff.context`, `diff.noprefix`, `diff.mnemonicPrefix`, `diff.srcPrefix`/`dstPrefix`, and `.gitattributes`-selected `textconv`/external diff drivers (no `--no-textconv`, no `--no-ext-diff`, no explicit `-U3`). DM §10 rule 12 is explicit: *"every other output-affecting option fixed by the release … A repository that sets `diff.algorithm` must not thereby change its own dump."* Also, `g10.rs:150-153` neutralises `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` on the **clone** command only; `git::Repo::run_bytes`/`run_stdin` set no env, so every indexing invocation on both sides reads the user's global and system config.

**10. `verify.rs` writes the keyring to a predictable world-writable path.** `Scratch::new` (`verify.rs:157-168`) builds `$TMPDIR/spine-graph-verify-<pid>-<n>` and calls `create_dir_all`, which **succeeds on an existing directory**; `write` then follows symlinks. A local attacker who pre-creates that directory (pid and counter are both predictable) can substitute `allowed_signers` and turn `approval.verified` / `seal_verified` from `false` into `true`. `mkdir` with `O_EXCL` semantics or a randomised name is the fix.

**11. First-match on intent id loses status.** `statuses()` (`derive.rs:1288-1330`) pushes one row per landing carrying an intent id and then mutates via `find`, and `emit_landing` reads via `find`. Two landings for one id (a land followed by a `withdraw` tombstone, or a reland) leave the tombstone's `withdrawn` unreachable, and the two intent nodes — same id, different `attrs` — collapse in `Graph::add_node` by `canonical(attrs) ‖ NUL ‖ src` byte order, i.e. by whichever `status` string sorts lower. Deterministic, but the winner is chosen by JCS bytes rather than by PB §6.6.

---

## minor

- `Envelope::read` (`derive.rs:1617-1633`) only collects `Spine-*` lines that contain a `:`. EV §2.3 defines a `Spine-*` line by its **first six bytes** and calls selection *"purely lexical and total"*. A colon-less `Spine-*` line is therefore excluded from `digest_above_seal` — fail-closed (the landing reads `unattested`), but it is a divergence from EV §2.3.
- `changeset.lane`/`event`/`strategy` are emitted as `""` when the trailer is absent (`derive.rs:701-704`, `seal_field`), conflating absent with empty against DM §7.3 (*"Absence means this concept does not apply … never 'unknown' and never 'empty'"*). None of `lane`, `event`, `strategy`, `threat`, `profile`, `mode` has a domain check.
- `git.rs:249` documents `git diff --name-only -z --no-renames` but the flag is not in the argv (the config pin covers it; the comment is wrong).
- `derive.rs:1427-1436` `constitution_version` uses `?` inside the field loop, so a header field lacking `": "` *before* `Version` deletes the constitution node and dangles every `built_under` edge. CN §9.1 makes that `bad-header-field`, a finding, not an unreadable version. Version is field 1, so it is only reachable via `header-field-order`.
- `blob_at`/`config` swallow all `GitError` with `.ok()?`, so "git unavailable" is indistinguishable from "path absent".

**Tests that cannot fail / coverage holes**
- `derive_repository.rs:654` `assert_eq!(attr(n, "late_reopen_count"), "0")` on a fixture with **zero** reopens — true for any implementation, and it is the only assertion on finding 1.
- `derive_shapes.rs` last line: `assert!(trunk.t0.len() == 40 || trunk.t0.len() == 64)` — vacuous.
- `a_tombstone_retires_its_id_with_status_withdrawn_and_its_parents_tree` never asserts anything about the tree.
- `an_orphan_on_trunk_is_no_changeset_at_all` ends with `assert!(!graph.nodes().is_empty())`.
- **Zero coverage** for: any `Spine-Reopen` line, `Spine-Upgrade`, `signer.valid_to` / key rotation, `changeset.resealed: true` (false in every test), `dump-version-skew`, `unverifiable(git-version)`, a squash whose approval commit is gone (`derive.rs:1108-1113`), and `approval.voided_by`.
- `tests/dm_17_conformance.rs`'s empty-dump test asserts `line_count == 1` only; DM §12.5's 105 bytes / `2a3fcea5…` are asserted in a `src/dump.rs` unit test, not against the serializer integration path.
- Every `derive_repository`/`derive_shapes` test returns silently when `git` or `ssh-keygen` is missing — 19 of the 98 pass vacuously on such a machine, including the G10 test.

---

## Verdict

**The serializer is sound; the derivation is not.** Everything DM §2–§6 and §17 covers — JCS profile, `esc`, framing, the terminating LF, the node/edge keys, the `esc`-before-raw ordering trap, the §5.5 collapse, JCS-vs-line sort — is implemented correctly and is genuinely pinned by reproduced vectors. `esc` and `tok` are separate encoders; the trailing-LF rule is right for this artifact; the wire comparator is byte order over the whole token. None of the three cross-crate traps is hit.

`derive.rs` is where it breaks. Two of its computed attrs are structurally dead (`late_reopen_count`, `voided_by`), one is fail-open on the security question it exists to answer (`seal_verified`), one silently reports a retired key as live (`signer.fingerprint`/`valid_to`), one under-reports the protected floor four different ways (`effective_c_a2`), and the value-domain checks that guard the artifact stop at node attrs — the edge side is guarded by a constant naming an attr that does not exist. Findings 1, 3, 4 and 5 are all in the same direction: a guarantee that fails silently rather than loudly. And the crate ships `DUMP_VERSION = 1` over a projection it knows is not DM §12.2's, which DM §3.4 makes a version bump, not a TODO.

I would not land this as conforming to `dump_version: 1`.
---

# RESOLUTION — 2026-08-28

Every finding above was worked. What changed, by number.

## Every-landing and serious

1. **G15 never consults the running binary.** Fixed: the gate takes the running
   version and dist hash as input and compares them.
2. **`G2=override` unreachable.** Fixed in `decide`: a tripwire wire is
   discharged by a protected review as well as a tripwire one
   (`Review::admits`), which is what makes GR §8.2's own report reproduce.
3. **G8 clause 2's wire.** Fixed: `G8:<b.path>`, the base record's path.
4. **G16 wires.** Fixed: all twenty outright findings carry a wire — ten
   path-bearing (constitution, keyring, manifest), the rest bare. Guarded by
   `no_g16_finding_is_raised_without_a_wire`, a source scan, because live
   evaluation cannot reach all twenty statuses at once.
5. **Path-encoding boundary.** Fixed in three places and made hard to
   re-introduce:
   - `Manifest::floor_entries_raw()` and `FileRecord::path_raw()` /
     `file_path_raw()` return decoded bytes; `floor_entries()`'s doc now says
     what it is. `G14Input.e_m_b`'s type already wanted `Vec<Vec<u8>>`, so the
     correct accessor now typechecks and the encoded one does not.
   - `ScaffoldObservation.path`, `Rollback.paths_not_restored` /
     `paths_not_deleted` and `staging_residue` are raw bytes, which also makes
     a non-UTF-8 path representable.
   - `a_floor_entry_is_matched_by_its_bytes_and_not_by_its_esc_spelling` pins
     both directions of the trap.
6. **`Review` could not express the binding.** Fixed: `Binding` is a required
   argument of `Review::new`, `names()` requires `Binding::Current`, and an
   unbound review discharges nothing.
7. **G13 check 2's spurious wire.** Fixed: a verifying unknown trailer raises
   nothing.
8. **G13 check 11.** Fixed: sums only *earlier verifying* approvals whose
   principal is in the keyring — the key-rotation brick MF §4.8.2's voiding
   exists to prevent.

## Minor

9. **Dialect-refused `C-A2` pattern.** Fixed, and the direction inverted: CN
   §7.2 makes a malformed member make the *whole list* malformed, so the list
   takes its default `["**"]` and everything is floor. Dropping the one pattern
   shrank the floor by exactly the entry nobody could parse.
10. **`diff_size_fires(_, None)`.** Fixed by typing it: `DiffSizeBound::Quick(n)`
    or `GatedUnbounded`. The corpus still fixes no gated-lane bound (C6), but a
    quick-lane caller can no longer omit one by accident.
11. **`has_uppercase_in_bracket` on an unterminated `[`.** Fixed: an
    unterminated bracket is ID §6.2's `bad-bracket`, left to the dialect.
12. **G13 check 5 after two reopens.** Kept, now declared and tested: the
    "binding" clause governs and the second reopen names nothing.
13. **Approvals identified by `freeze=`.** Fixed: `Statement` gained `line` —
    GR §5.5's third member, which was missing — and both `is_binding` and check
    11 resolve by position in `E`. Check 3 refuses byte-identical lines, which
    is what makes the line a key.
14. **`monotone_union` dead.** Fixed by making step 4 decide rather than
    assert: `Rollback.ancestor` carries `A` and
    `paths_are_the_monotone_union(M_T, A, M_B)` computes the key union and the
    per-key value union. The canonical shape needs no check — `Manifest::parse`
    already refuses a one-element array, an unsorted one and a duplicate.
15. **`only_protected_is_a_g7_hard_lease`.** Changed to GR §6.1's literal
    reading: exactly one protected entry, and it is a `G7`. Two G7 leases do
    not hold it. Recorded in OPEN-questions — the counter arguably wants them.
16. **`G8Status::class()`'s default.** Kept, now declared: `protected` is the
    wider class, so an unnamed clause guessing it can only over-review.
17. **`manifest_blob_in_t.unwrap_or_default()`.** Fixed: absent is a mismatch,
    not a blank an empty `manifest=` can match.
18. **`a + d` wrapping.** Fixed: saturating.

Workspace: 1300 tests, 0 failures, clippy clean.
