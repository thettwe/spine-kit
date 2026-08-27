# The intent envelope, and its three digests

**Artifact:** the byte string that is a landing commit's message — subject, fenced intent, trailer block — and the three digests sealed inside it: `envelope=`, `freeze=` and `report=`.
**Home in the playbook:** PB §5.5 (the landing commit and its envelope), PB §4.3 (the approval record and `freeze=`), PB §7.4 rule 4 (`report=`), PB §11 (every trailer's payload grammar, which wins over prose here as it wins there).
**References:** `PB §n` cites `PLAYBOOK.md`; a bare `§n` cites this document; `GR §n`, `RF §n` and `DU §n` cite `gate-report.md`, `result-file.md` and `dump.md`. The numbering schemes collide — PB §5.5 is the envelope, §5.5 is nothing — so every citation says which.
**Spec version:** 2 · **Envelope version:** 1 (`Spine-Envelope: 1`) · **Covers:** PLAYBOOK.md v0.19 · **Status:** normative for v1.
**Normative dependencies.** `gate-report.md` §2 (what `report=` digests), §2.3 (the `esc` encoding), §6.2 (the wire token `tok`, adopted verbatim), §5.5.1 (the order of copied `Spine-Reopen` and `Spine-Review` lines, adopted verbatim), §5.6.2 (which gates have `Spine-Gates` entries). `result-file.md` §4.4 (the `runner` token's lexical form). `import-resolver.md` §11.1 (the four ratified `runner` tokens — Kotlin and the `gradle` adapter are out of v1) and §11.2–§11.3 (the `pytest` and `vitest` id grammars the vectors below use). `dump.md` §5.2.1 (the byte range of a signed trailer line — this document fixes what that range is signed over, as DU §5.2.1 asks it to). `intent-doc.md` and `constitution.md` fix inputs this document only frames; §17 says what that costs.

**Amended 2026-08-26**, to four owner decisions of that date. **(a) The landing subject is derived, not written, and G9 recomputes and checks it** (§2.1, §3.2 question 3, §6.3, §12, §13.10, §16 OPEN-1, §17 item 10). It stays **outside `envelope=`**: no digest definition changes and no signature moves on that account. The residual the owner accepted is stated wherever it is relevant — the quick lane's summary is free text, and every toolkit lifecycle landing rides the quick lane. **(b) `Template:` names the variant and the version**, so an intent doc's header reads `Template: intent@2` and a `Spine-Signoff` payload reads `template=intent@2`, replacing the bare `v2` of version 1. **(c) Kotlin is dropped from v1** (§4.4): four languages, four ratified `runner` tokens. **(d) The 16 KiB cap does not apply to a reseal envelope** (§2.9, §12, §14 D11, §16 OPEN-3, §18 item 24), because a reseal is the one shape that can neither be split nor break-glassed, so a capped reseal over a long orphan range would leave trunk permanently unlandable. No vector below moves: the capped quantity is defined exactly as before, and none of the vectors is a reseal, so every one of them is still measured against the cap.

**Decision (b) changed bytes this document digests, and every affected value below was recomputed rather than adjusted.** The intent blob is new (`bytes=` 759 → 765, a new `blob=`), and with it the `Spine-Signoff`, `Spine-Approve` and `Spine-Review` lines that carry it, and therefore `envelope=` for vectors A and D. `freeze=` is unchanged and was re-verified, because it covers only `Spine-Frozen` and `Spine-Test` lines, which decision (b) does not touch; vector C is unchanged for the same reason and was re-verified byte for byte.

**The throwaway keyring of §8.1 was regenerated, and this is the one change nothing in the owner's decisions forced.** Version 1 published the three public keys and no private key, so the four signed lines whose bytes decision (b) moved could not be re-signed under the old keys, and a vector whose `-Sig` line does not verify is worse than no vector. Three fresh throwaway ed25519 keys were generated, §8.1 and §9 publish them with their fingerprints, and **all nine signatures below were produced over exactly the line printed above each and verified with `ssh-keygen -Y verify`**. Every `-Sig` line moved with the keys, and with them vector B's `envelope=` — which decision (b) does not otherwise touch. Vector C carries no signature and no template field, and is byte-identical to version 1's. One consequence is outside this document and is not reconciled here: `manifest.md` §8.7 adopts §8.1's keyring verbatim, so its three key lines, its three fingerprints, its `.spine/allowed_signers` blob id and every manifest digest computed over that blob id are now stale and must be recomputed against the keys published below.

---

## 1. What this artifact is, and what rests on it

The envelope is the record. PB §5.5: *"The disposal rule deletes the file. It must not delete the truth. The record is a git object: the landing commit."* Everything the ledger later says about a landing — who signed what, over which base, against which tests, under which policy, with which gates green — is read out of that one byte string, by `git cat-file commit`, on a clone with nothing but git objects and OpenSSH.

Three digests bind it, and each answers a different question:

| Digest | Where it lives | Covers | What its failure means |
|---|---|---|---|
| `envelope=` | `Spine-Seal` | every `Spine-*` line above the seal | the trailer block is not the one the pipeline sealed |
| `freeze=` | `Spine-Approve` | the sorted `Spine-Frozen` and `Spine-Test` lines | the approved test set is not the one that was frozen |
| `report=` | `Spine-Review` and `Spine-Seal` | the canonical gate report (`gate-report.md`) | the judgement is not the one that was signed |

**The finding this document closes.** A pre-implementation audit of the spec set recorded: *"no test vector for any of the three digests, and the one block implementers transcribe contains an ellipsis."* Both halves are fatal in the same way. Two implementations that compute `envelope=` differently cannot verify each other's landings, which is the property PB §1.1 sells; and PB §5.5's canonical envelope — the block every implementer will copy — leaves open whether `-Sig` lines are inside the digest, whether the fenced intent is, whether the subject is, whether a trailing LF exists, and what happens to a provider's own trailers. Nothing in PB §11's payload grammar answers any of the five.

This document answers all five, fixes the join, fixes the sort, fixes the encodings, and publishes four vectors whose digests were computed over exactly the bytes printed.

**What is *not* at stake.** The digests are functions of the message bytes alone. A landing commit's object id additionally depends on its tree, its parents, and its author and committer identity lines — including the two dates git requires. **No spine digest covers a date and no gate reads one** (PB §7.5: *one clock, no timestamps*; GR §4.4.3 says the same of the note commit that publishes the report). §6.3 states exactly what the commit object carries that nothing hashes.

---

## 2. The envelope, as bytes

### 2.1 The message's regions

A landing commit's message is a byte string with three regions, in this order:

```
<subject line>
<blank line>
[<fenced intent block>]
[<blank line>]
<trailer block>
```

1. **Subject** — one line, **derived and never written** (PB §5.5). `spine check --land` computes it from the envelope, and **G9 recomputes it on the first-parent walk and refuses `subject-mismatch`** on a difference. It is outside `envelope=` and outside every signature, so the rule costs no digest and moves none. §13.10 gives the derivation for each of the five landing shapes; in brief: a **gated** landing and a **tombstone** take the fenced block's first line with the leading `# ` removed; a **quick-lane** and a **toolkit lifecycle** landing take `quick: ` followed by a one-line free-text summary; a **reseal** takes `reseal: ` followed by the full object id of the orphan tip `O`, which is the seal's `head=`.
2. **Blank line** — exactly one, exactly `0x0A` on its own.
3. **Fenced intent block** — present on a gated landing and on a tombstone, absent otherwise (PB §5.5). §2.6.
4. **Blank line** — exactly one, present iff the fenced block is.
5. **Trailer block** — one `Spine-*` line per line, no blank lines inside it, ending with `Spine-Seal-Sig`.

The message ends with exactly one `0x0A` after the `Spine-Seal-Sig` line. There is no trailing blank line, no CR anywhere, and no BOM.

**Anything after the `Spine-Seal-Sig` line** is a provider's own appended trailers (§2.8). They are outside every digest and outside every signature; a `Spine-*` line among them is a refusal, not a decoration (§2.8).

### 2.2 What a line is

A **line** is a maximal run of bytes containing no `0x0A`, delimited by `0x0A` or by the start of the message. **The terminating `0x0A` is not part of the line.**

Lines are taken **raw**. Nothing is trimmed, unfolded, case-folded, Unicode-normalized, or re-encoded, at any point, for any purpose — not to hash, not to sign, not to verify, not to parse. A trailing `0x0D` is part of the line and is hashed as such: an implementation that strips CR before hashing would accept a CRLF body the seal does not cover, which is exactly the failure PB §5.4 warns about (*"never the web editor — CRLF hashes wrong"*). Detection, never repair.

`spine check --land` refuses to build a message containing `0x0D` or `0x00` anywhere, and G9 refuses a landing whose message contains either. Both refusals are `envelope-malformed`.

### 2.3 What a `Spine-*` line is, and why selection and validation are separate acts

A **`Spine-*` line** is a line whose first six bytes are `S`, `p`, `i`, `n`, `e`, `-` (`0x53 0x70 0x69 0x6E 0x65 0x2D`), case-sensitive, followed by at least one further byte.

**Selection is purely lexical and total.** Deciding whether a line is in the digest never requires parsing it, knowing its name, or judging it well-formed. That is deliberate: a digest that could only be computed over a *valid* envelope would be uncomputable for the malformed envelope a verifier most needs to diagnose. Every reader can always compute `envelope=`.

**Validation is a separate act, and it fails closed.** A `Spine-*` line is **well-formed** iff:

- its name matches `Spine-[A-Za-z][A-Za-z0-9-]*` and is one of the twenty-six names in §2.4's closed set;
- the name is followed by exactly `:` `U+0020` (one colon, one space);
- the remainder — the **payload** — is non-empty and contains no `0x0D` and no `0x00`.

A `Spine-*` line that is not well-formed makes the envelope malformed. It is still inside the digest, because selection ignores validity. The two rules compose the safe way round: `Spine-Review:x` is hashed *and* refused, where a validity-gated selection would have hashed neither and an unwary indexer might still have read it.

**Case matters.** `spine-seal: …` and `SPINE-SEAL: …` are not `Spine-*` lines: they are ordinary message text, outside the digest, and G9 refuses a landing containing either (`envelope-malformed`) rather than leaving a near-miss spelling that a sloppy reader might honour.

**Nothing else in the message can be mistaken for one.** PB §3.3's canonical-form rule makes `spine new --sign` refuse an intent doc containing any line beginning `-----` or `Spine-`. That refusal is what makes `^Spine-` a sound selector over the whole message rather than only over the trailer block: no fenced byte can ever match it. The rule is load-bearing for `envelope=` and this document depends on it.

### 2.4 The trailer block: a closed name set in a fixed order

**The name set is closed.** These twenty-six names, and no others:

`Spine-Envelope` · `Spine-Event` · `Spine-Lane` · `Spine-Intent` · `Spine-Signoff` · `Spine-Signoff-Sig` · `Spine-Upgrade` · `Spine-Upgrade-Sig` · `Spine-Reopen` · `Spine-Reopen-Sig` · `Spine-Withdraw` · `Spine-Withdraw-Sig` · `Spine-Approve` · `Spine-Approve-Sig` · `Spine-Approval` · `Spine-Frozen` · `Spine-Test` · `Spine-Review` · `Spine-Review-Sig` · `Spine-Gates` · `Spine-Strategy` · `Spine-Supersedes` · `Spine-Reverts` · `Spine-Trust-Root-Prev` · `Spine-Seal` · `Spine-Seal-Sig`

(Twenty-six names. Seven of them are `-Sig` lines — `Spine-Signoff-Sig`, `Spine-Upgrade-Sig`, `Spine-Reopen-Sig`, `Spine-Withdraw-Sig`, `Spine-Approve-Sig`, `Spine-Review-Sig`, `Spine-Seal-Sig` — each signing the line above it, so **nineteen** are statements. The count of *lines* on any one landing is smaller: no landing carries every name.) An unknown `Spine-*` name is `envelope-malformed`, never ignored: there is no forward-compatibility relaxation, for the reason `result-file.md` §4.3 gives — `Spine-Envelope` already versions the format, so a reader that meets a name it does not know is reading a version it does not implement and must say so.

**The order is fixed.** PB §5.5's example fixes most of it and this document fixes the rest. Rank ascending; a trailer absent from a landing consumes no rank; a `-Sig` line immediately follows the line it signs, on the next line, with nothing between.

| Rank | Trailer | Cardinality | Present on |
|---|---|---|---|
| 1 | `Spine-Envelope` | 1 | every landing |
| 2 | `Spine-Event` | 1 | every landing |
| 3 | `Spine-Lane` | 1 | every landing |
| 4 | `Spine-Intent` | 0–1 | gated landing, tombstone |
| 5 | `Spine-Signoff` + `-Sig` | 0–1 | gated landing; tombstone where one exists (PB §5.5) |
| 6 | `Spine-Upgrade` + `-Sig` | 0–1 | toolkit lifecycle landing (PB §6.7) |
| 7 | `Spine-Reopen` + `-Sig` | 0–n | gated landing, one per reopen |
| 8 | `Spine-Withdraw` + `-Sig` | 0–1 | tombstone |
| 9 | `Spine-Approve` + `-Sig` | 0–1 | gated landing |
| 10 | `Spine-Approval` | 0–1 | gated landing |
| 11 | `Spine-Frozen` | 0–n | gated landing, squash strategy only |
| 12 | `Spine-Test` | 0–n | gated landing, squash strategy only |
| 13 | `Spine-Review` + `-Sig` | 0–n | any landing that took a review |
| 14 | `Spine-Gates` | 1 | every landing |
| 15 | `Spine-Strategy` | 1 | every landing |
| 16 | `Spine-Supersedes` | 0–1 | gated landing, from the intent's header |
| 17 | `Spine-Reverts` | 0–1 | any landing |
| 18 | `Spine-Trust-Root-Prev` | 0–1 | a rotation root (PB §7.5), which is not a landing — §17 |
| 19 | `Spine-Seal` | 1 | every landing |
| 20 | `Spine-Seal-Sig` | 1 | every landing |

**Within a repeatable rank:**

- `Spine-Reopen` and `Spine-Review` are emitted **ancestor-first along the branch's first-parent path**, which is `gate-report.md` §5.5.1's order, adopted verbatim so that the envelope's order and the report's array order are one order and not two. GR §5.5.1 proves it total: G13 refuses two event commits carrying byte-identical signed lines, outright (`manifest.md` §4.8.4 check 3). For reopens this is also ascending `reopens=`.
- `Spine-Frozen` and `Spine-Test` are emitted in the **`freeze=` sort order of §4.2**, so the digest recomputes from the copied lines by `sort`-free concatenation and a reader can check the order as well as the value.

**Exactly one `Spine-Seal`, exactly one `Spine-Seal-Sig`, and the `-Sig` immediately after the seal.** A second `Spine-Seal` line anywhere is `envelope-malformed`. This closes the append-a-second-seal shape before any digest is consulted.

**No blank line inside the trailer block**, and no non-`Spine-*` line inside it (§2.8).

### 2.5 The payload grammar

PB §11's *Trailers* table is the grammar and it wins. Three things it shows but does not say, fixed here:

1. **Field order is normative.** Fields appear in exactly the order PB §11's payload column gives, with optional fields in their stated positions when present and absent otherwise. A different order is `envelope-malformed`. Parsing is by key, but *emission* is by position — without that, two implementations produce different bytes over identical facts and every digest and every signature diverges.
2. **One `U+0020` between fields**, none before the first, none after the last. The payload has no leading and no trailing space.
3. **`reason=` values are JSON string literals** (PB §7.2, PB §11): a `"` delimited run with JSON's escaping, so a reason containing a quote, a backslash, a newline or any non-ASCII character is representable and the line stays one line. This is the *only* quoting a payload field uses, apart from the two path encodings below.

**Two different path encodings live in one envelope, and both are required.** This is not a redundancy and an implementation must not unify them:

| Where a path appears | Encoding | `tests/fixtures/café.json` becomes |
|---|---|---|
| `Spine-Frozen: <oid> <path>` | `git ls-tree` C-style quoting (§4.3) | `"tests/fixtures/caf\303\251.json"` |
| a wire token in `Spine-Review`'s `wires=` | `tok`, i.e. `gate-report.md` §6.2's one-pass `esc` variant | `G8:tests/fixtures/caf\xc3\xa9.json` |

`Spine-Frozen` uses git's quoting because PB §4.3 and PB §11 say it does (*"`<oid> <path>` (`git ls-tree` quoting)"*), and a frozen path must be comparable byte-for-byte with what `git ls-tree` prints during a hand audit. Wire tokens use `tok` because `gate-report.md` §6.2 fixes them there and instructs this document to adopt that function verbatim, which it does. §13.9 records the asymmetry and why it is not repaired here.

### 2.6 The fenced intent block

```
-----BEGIN SPINE-INTENT blob=<oid> bytes=<n>-----
<exactly n bytes>
-----END SPINE-INTENT-----
```

- The BEGIN line is exactly that text: five hyphens, `BEGIN SPINE-INTENT`, one space, `blob=` and the full-length lowercase object id, one space, `bytes=` and a plain decimal count with no sign and no leading zero, five hyphens. It is terminated by one `0x0A`, which is **not** part of the `n` bytes.
- The next `n` bytes are the intent blob's bytes, verbatim. `git hash-object` over exactly those bytes reproduces `blob=` (PB §5.5).
- The END line follows the `n`-th byte **immediately**, with no inserted separator. PB §3.3's canonical form guarantees the intent ends with exactly one `0x0A`, so byte `n` is that `0x0A` and the END line begins a line. A blob whose last byte is not `0x0A` cannot be fenced; `--sign` already refuses it.
- A parser reads exactly `n` bytes. It never searches for the END delimiter, so an intent that somehow contained the END line as text could not truncate the block. (`--sign` refuses a line beginning `-----`, so it cannot, but the parser does not rely on that.)

`bytes=` counts **bytes, not characters**. Vector A's intent is 765 bytes and 762 characters; the three-character difference is three `·` (U+00B7, two bytes each) in the header line.

**The fenced block is outside `envelope=`** (§3.2, question 2). It is bound by `blob=` on the sign-off line, which is itself inside `envelope=` and inside a signature.

### 2.7 The signed statement, and what a `-Sig` covers

PB §7.2 and PB §11: one trailer line ending in `signer=<principal>` (reviews: `reviewer=`), plus `<Name>-Sig: <SSHSIG, armor stripped to one line>` produced by `ssh-keygen -Y sign -n <namespace>` over **that line's exact bytes**.

**The signed byte range**, fixed here because `dump.md` §5.2.1 asks this document to fix it and hashes the same range: from the **first byte of the trailer name** — the `S` of `Spine-` — through the **last byte before the terminating `0x0A`**, with the `0x0A` **excluded**. The trailer name and the `: ` are inside the signature. Signing only the payload after `: ` is non-conforming: it would let a `Spine-Approve` payload be replayed as a `Spine-Signoff`.

**The `-Sig` payload** is the SSHSIG blob's base64 with the PEM armor removed: the `-----BEGIN SSH SIGNATURE-----` line, the `-----END SSH SIGNATURE-----` line and every `0x0A` inside are deleted, leaving one unbroken base64 run. Re-armoring for `ssh-keygen -Y verify` is the exact inverse: the BEGIN line, the base64 wrapped at **70 characters per line**, the END line, each terminated by `0x0A`. This round-trip is byte-identical to what `ssh-keygen -Y sign` writes (§8.6 verifies it).

**Ed25519 signatures are deterministic** (RFC 8032), so re-signing an identical line with an identical key reproduces an identical `-Sig` payload. That is a property of the vectors below, not a requirement on implementations.

**Verification** is PB §11's, by hand, with no spine binary:

```
ssh-keygen -Y verify -f .spine/allowed_signers -I <principal> -n <namespace> -s <sigfile> < <linefile>
```

where `<linefile>` holds exactly the line bytes with no terminator. §8.6 gives the extraction commands.

### 2.8 Foreign trailers

PB §5.5: *"Foreign trailers a provider appends after the seal are outside the digest and ignored."*

- A **non-`Spine-*`** line appearing **after** the `Spine-Seal-Sig` line — `Co-authored-by:`, `See merge request …`, a provider's sign-off — is outside `envelope=`, outside every signature, and permitted. This is the case PB §5.5 describes and it is the only permitted one.
- A **non-`Spine-*`** line **inside** the trailer block, between rank 1 and `Spine-Seal-Sig`, is `envelope-malformed`. Providers append at the end; an interleaved foreign line is either tampering or a non-conforming builder, and refusing is the fail-closed direction.
- A **`Spine-*`** line **anywhere below** the `Spine-Seal` line other than the single `Spine-Seal-Sig` is `envelope-malformed`. PB §5.5's word "ignored" is wrong for this case and §14 D7 files it: an *ignored* trailer is one an indexer that greps the whole message will read, unsigned and unsealed, and PB §11 already makes the seal-plus-`-Sig` pair the last `Spine-*` lines. The refusal is what makes that definition enforceable.

### 2.9 The 16 KiB cap

PB §5.5, as amended 2026-08-26: *"The fenced intent plus the signed lines are capped at 16 KiB, projected at `--approve` and checked at `--land`; exceeding it is a refusal (`envelope-too-large`), never a truncation — split the intent. Switching strategy is not an exit and is no longer offered as one … So the cap does not apply to a `Spine-Event: reseal` envelope … `Spine-Frozen`/`Spine-Test` lines are outside the cap."*

**The capped quantity**, fixed here: the byte length of the whole commit message, **excluding every `Spine-Frozen` and `Spine-Test` line together with its terminating `0x0A`**, compared against **16384**. Greater than 16384 is a refusal — **on every landing shape except a reseal**, which is not measured at all (below).

This is a superset of the literal phrase "the fenced intent plus the signed lines" — it also counts the subject, the blank lines, and the unsigned `Spine-Envelope`/`Spine-Event`/`Spine-Lane`/`Spine-Approval`/`Spine-Gates`/`Spine-Strategy` lines, a few hundred bytes in all. A superset is the safe direction: it can only refuse earlier, never later, and it is the quantity that actually bounds the object. It is also computable without parsing anything but line prefixes.

Vector A's capped quantity is 4031 of 16384; vector D's, the same landing under squash, is 4032 — one byte more, because `squash` is one byte longer than `merge`, and the 567 bytes of frozen manifest, with their seven terminators, do not count.

**A reseal envelope is not capped.** The comparison above is performed for every landing shape **except** one whose `Spine-Event` is `reseal`: for that shape no cap is evaluated, no projection is made at `--approve`, and `envelope-too-large` (§12) is never raised. PB §5.5 fixes this, and it closes §14 D11 and §16 OPEN-3 as option (a). The shape is read from `Spine-Event`, which is mandatory on every landing, sits above the seal and is inside `envelope=` — the same sealed input §13.10 selects a subject derivation by — so the exemption needs no parse of any payload and cannot be claimed by a landing that is not a reseal.

The reason is that a reseal is the one shape with **no exit from the cap at all**. Its protected review's `wires=` grows with the orphan range it covers (PB §5.5 folds every wire in the range into one report and one review); the range is a fact about what was pushed around the pipeline, not a document an author can divide; and break-glass is unreachable, because PB §7.6 offers it only from `tests-approved` onward and a reseal is not an intent and never enters that state. In solo mode the signerless overlay gives a reseal exactly one review, so there is not even a second review to spread the wire set across. A capped reseal over a long range would therefore be a refusal that no legal act clears, while G9 refuses to land on top of an unresealed orphan: trunk permanently unlandable. An over-long commit message on the one shape nobody reads line by line is the smaller harm, and it is not unbounded in practice — the range it describes is bounded by however far the pipeline was bypassed before anyone noticed.

**Every other shape has exactly one exit: split the intent.** Switching `C-M1: merge.strategy` is not an exit and PB §5.5 no longer offers it as one — the capped quantity is the fenced intent plus the signed lines, and this section's own vectors measure what strategy moves: 4031 under `merge`, 4032 under `squash`, one byte, the difference in length between the two words. An implementation that suggests re-landing under the other strategy to get under 16384 is suggesting a no-op.

---

## 3. `envelope=`

### 3.1 The definition

> `envelope=sha256:<hex>`, where `<hex>` is 64 lowercase hex digits of the SHA-256 of the byte string formed by taking, **in message order**, every `Spine-*` line (§2.3) that appears **above the `Spine-Seal` line**, and joining them with a single `0x0A` between consecutive lines — **no separator before the first, and none after the last**.

Equivalently: `sha256("\n".join(lines))` where `lines` is that sequence.

The digest of an empty sequence is the SHA-256 of the empty string, `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. No conforming landing produces one — ranks 1, 2, 3, 14 and 15 are mandatory on every landing — but the function is total and an implementation must not special-case it.

### 3.2 The five questions the playbook left open

**1 · Are `-Sig` lines inside the digest? Yes.**
PB §11 says *"every `Spine-*` line above, in order, LF-joined"*, and `Spine-Signoff-Sig`, `Spine-Reopen-Sig`, `Spine-Approve-Sig`, `Spine-Review-Sig` and `Spine-Upgrade-Sig` are all `Spine-*` lines. §11 wins; there is no exception clause and none is invented. It is also the stronger reading: with the `-Sig` lines inside, `envelope=` binds *which key's signature* accompanied each statement, not merely that some verifying signature did. `Spine-Seal-Sig` is the only `-Sig` line excluded, because it is below the seal.

**2 · Is the fenced intent block inside the digest? No.**
No line of it begins `Spine-` — the BEGIN and END delimiters begin `-----`, and PB §3.3 forbids the body a line that starts `Spine-`. The block is bound instead by `blob=` on the `Spine-Signoff` line (and by `Spine-Approve`'s `intent=`, and by `Spine-Withdraw`'s `blob=` on a tombstone), each of which *is* inside the digest and inside a human signature. Editing one fenced byte breaks `blob=`; editing the sign-off line to match breaks its signature and `envelope=` both.

**3 · Is the subject line inside the digest? No.**
It does not begin `Spine-`, and the owner settled on 2026-08-26 that it stays out. Folding it in would have meant prepending it to the joined lines — a change to every implementation's digest function, available only before the first landing existed anywhere. What replaced the binding is a **derivation**: the subject is a pure function of the envelope (§13.10), `spine check --land` emits no other, and **G9 recomputes it and refuses `subject-mismatch`**. So the case PB §5.5's *"everything is trusted because it hashes"* left open — a provider rebuilding the commit from a PR body under PB §5.4 configuration (b) and setting whatever subject it likes while every signature still verifies — is now caught by a gate rather than by nothing, without a byte of this digest moving. §14 D10 and §16 OPEN-1 record how it was closed. The residual the owner accepted, stated here because this is where a reader will look for it: **the quick lane's summary is free text, and every toolkit lifecycle landing rides the quick lane**.

**4 · Is there a trailing LF? No.**
"LF-joined" is separator semantics: `n` lines yield `n − 1` separators. Three reasons beyond the words. (a) It matches the house: `gate-report.md` §2.1 hashes the report with *"no trailing newline, no BOM, no framing"*, and `freeze=` uses the same join (§4.1); the one place spine terminates every line including the last is `dump.md` §2.2, and that is a *stream framing*, which a join is not. (b) It makes the one-line self-test hold — the envelope digest of a single-line trailer block equals the SHA-256 of that line, which is the first thing anyone checks by hand. (c) The join is injective over well-formed line sets either way, since a line cannot contain `0x0A`, so neither reading is safer and the words decide.

The wrong value is published beside the right one in every vector below, precisely so an implementation that got this wrong recognises its own output.

**5 · What happens to foreign trailers? §2.8.**
Below the seal and not `Spine-*`: outside the digest, permitted. Below the seal and `Spine-*`: refused. Inside the trailer block and not `Spine-*`: refused.

**And a sixth the playbook's phrasing invites: above *what*?**
PB §11 spells the field `envelope=sha256:<hex over every Spine-* line above, in order, LF-joined>` inside a row headed `Spine-Seal + -Sig`, which admits the reading "above the `Spine-Seal-Sig` line". That reading is **impossible**: it would put the `Spine-Seal` line — which contains `envelope=` — inside its own digest. PB §5.5's prose is the unambiguous one (*"SHA-256 over every `Spine-*` line above **it**"*, of the seal), and this document uses it. §14 D8 recommends §11 say "above the `Spine-Seal` line".

### 3.3 Reproducing it from a commit

```sh
git cat-file commit <L> \
  | awk '/^Spine-Seal: /{exit} /^Spine-/{if(n++)printf "\n"; printf "%s", $0}' \
  | sha256sum
```

`git cat-file commit`, never `git log` (PB §5.5: *"the indexer reads messages with `git cat-file commit`, never `git log`, so no cleanup rule ever touches the fenced bytes"*). The `awk` program emits the selected lines joined by `0x0A` with no trailing `0x0A`, which is the definition transcribed. It is safe over the whole `cat-file` output because commit headers never begin `Spine-` and PB §3.3 forbids a fenced line that does.

On macOS, `shasum -a 256` for `sha256sum`. §8.6 runs both this command and the two below against real commit objects.

### 3.4 Edge cases, each with a defined behaviour

| Case | Behaviour |
|---|---|
| No `Spine-Seal` line | `envelope-malformed`. The commit is not a landing; G9 indexes it `orphan` (PB §5.5). |
| Two `Spine-Seal` lines | `envelope-malformed`. Not "the first one wins". |
| `Spine-Seal-Sig` missing or not immediately after the seal | `envelope-malformed`. |
| A `Spine-*` line below the seal, other than its `-Sig` | `envelope-malformed` (§2.8). |
| `Spine-Envelope: 2` | `envelope-version-unknown`. Refuse; do not attempt the version-1 digest. |
| A malformed `Spine-*` line above the seal | Hashed (selection is lexical), and `envelope-malformed`. |
| Message contains `0x0D` | Hashed as-is; refused by `--land` and by G9. |
| Zero `Spine-*` lines above the seal | Digest of the empty string. Cannot occur on a conforming landing. |
| Provider rebuilt the commit (PB §5.4 configuration (b)) with a different trailer set | The recomputed digest differs from the seal's `envelope=`; G9 indexes the landing `unattested`. This is the case the digest exists for. |

---

## 4. `freeze=`

### 4.1 The definition

PB §4.3: *"`freeze=` a SHA-256 over the sorted `Spine-Frozen` and `Spine-Test` lines — a non-git digest, used to name the approval elsewhere."* PB §6.3 G9 recomputes it *"over that commit's sorted `Spine-Frozen`/`Spine-Test` lines"*, and under squash *"`freeze=` recomputes from the copied lines"*.

> `freeze=sha256:<hex>`, where `<hex>` is 64 lowercase hex digits of the SHA-256 of the byte string formed by taking **every `Spine-Frozen` and every `Spine-Test` line** of the commit in question, **each line entire — trailer name, `: `, and payload — and excluding its terminating `0x0A`**, sorting them ascending by §4.2's comparison, and joining them with a single `0x0A` between consecutive lines, **with no trailing `0x0A`**.

The join is `envelope=`'s join, for the same reasons (§3.2, question 4). PB §4.3 fixes no join at all; this document fixes one and uses it in both places rather than two.

**The whole line is hashed, not the payload.** Three reasons. (a) "the sorted lines" says lines. (b) It makes the two kinds segregate under one comparison with no special rule (§4.2). (c) It removes any need to unquote a path before hashing, so the digest cannot diverge on a quoting disagreement — the quoting is hashed, not the decoded bytes.

**The commit in question** is:
- the **approval commit**, for the `freeze=` that `spine check --approve` seals into its own `Spine-Approve` line, and for G9's audit of a landing under **merge** strategy, where the approval commit is reachable via `Spine-Approval` (PB §6.3 G9: *"`Spine-Approval ∈ M(L)` under merge and the SHA-256 over that commit's sorted `Spine-Frozen`/`Spine-Test` lines equal to the copied approve line's `freeze=`"*);
- the **landing commit** itself, under **squash**, where PB §5.5 copies the manifest into the envelope because the approval commit becomes unreachable.

Both recompute the same value from the same lines. Vector D demonstrates it: the digest computed from the approval commit in §8.2 and the digest computed from the squash envelope in §11 are the same 64 digits.

### 4.2 The sort

**Ascending by unsigned byte value, over the entire line, `memcmp` order, shorter-is-smaller on a prefix tie.** Formally: compare byte `i` of each line as an integer in `[0, 255]` for increasing `i`; the first difference decides; if one line is a proper prefix of the other, the shorter sorts first.

- **Not** locale collation. `LC_ALL=C sort` in a shell; a language's default string comparison only if it is a byte comparison (Python `bytes`, Rust `&[u8]`, Go `string`, Java `byte[]` — **not** Java `String.compareTo` on decoded UTF-16, and not JavaScript's default `Array.prototype.sort`).
- **Not** the `esc` order that `dump.md` §6.4 uses. `dump.md` sorts `esc`-encoded bytes because its paths are `esc`-encoded in the artifact it sorts; here the artifact is the raw trailer line, and the sort is over exactly the bytes that are hashed. Sorting one thing and hashing another is how a spec grows a second place to disagree.
- **Duplicate lines are impossible.** Two identical `Spine-Frozen` lines would name one path twice; two identical `Spine-Test` lines would name one `(runner, id)` pair twice. `--approve` refuses either (`freeze-duplicate`). The order is therefore total.

**How the two kinds interleave: they do not.** `Spine-Frozen: ` and `Spine-Test: ` share the prefix `Spine-`, and `F` (`0x46`) precedes `T` (`0x54`), so under a whole-line comparison every `Spine-Frozen` line precedes every `Spine-Test` line. This is a consequence of §4.1's rule, not a separate rule, and an implementation must not encode it as one — a future trailer name between them would then be misplaced.

**Two consequences worth stating, because they surprise:**

1. **The frozen manifest is ordered by blob id, not by path.** A `Spine-Frozen` line is `Spine-Frozen: <oid> <path>`, so the first varying bytes are the object id's. Vector A's five frozen files appear in an order matching neither their path order nor their tree order. This is deterministic, which is what the digest needs; it is not a display order, and a review packet is free to sort by path.
2. **`Spine-Test` lines order by runner, then by id, bytewise.** The `runner` token is space-free and colon-free (`result-file.md` §4.4), so `Spine-Test: pytest …` precedes `Spine-Test: vitest …` for every id, and within one runner the ids sort bytewise — which is **not** numerically. Vector C pins both: `… AC10 …` precedes `… AC2 …`.

### 4.3 Path quoting in `Spine-Frozen`

PB §4.3 and PB §11: `<oid> <path>`, with `git ls-tree` quoting.

The path is the repository-relative, `/`-separated path **exactly as git stores it in the tree**, rendered by git's C-style quoting. The rendering is normative and **must not depend on `core.quotePath`**: spine always emits the quoted form, equivalent to `core.quotePath=true`, because a digest that varied with a local git config would be no digest at all.

**The rule, stated so an implementation need not shell out to git:**

The path is quoted — wrapped in `"` … `"` with escapes — **iff** it contains at least one byte in `0x00–0x1F`, `0x7F–0xFF`, `"` (`0x22`) or `\` (`0x5C`). Otherwise it is emitted literally, unwrapped. Inside a quoted path:

| Byte | Emits |
|---|---|
| `0x07` | `\a` |
| `0x08` | `\b` |
| `0x09` | `\t` |
| `0x0A` | `\n` |
| `0x0B` | `\v` |
| `0x0C` | `\f` |
| `0x0D` | `\r` |
| `0x22` | `\"` |
| `0x5C` | `\\` |
| any other byte in `0x00–0x1F` or `0x7F–0xFF` | `\` + exactly three octal digits, zero-padded, lowercase-irrelevant (`\001`, `\177`, `\303`) |
| any other byte in `0x20–0x7E` | itself |

**A space does not trigger quoting.** `tests/fixtures/tax rates.json` is emitted literally, spaces and all, and the payload is `<oid>` `U+0020` `tests/fixtures/tax rates.json`. Parsing is still exact: the payload splits at its **first** space, and everything after it is the path field.

**Deciding whether the path field is quoted is exact**: it is quoted iff its first byte is `"`. A real path beginning with `"` contains `"` and is therefore always quoted, so the test can never misfire. A path field that begins with `"` and is not a valid C-quoted string — unterminated, a bad escape, a trailing byte after the closing quote — is `envelope-malformed`.

**Nothing is normalized.** No NFC, no NFD, no case folding, no separator rewriting — the same rule `gate-report.md` §2.3 states and for the same reason: a path frozen on macOS and audited in a Linux container must be the same bytes.

### 4.4 `Spine-Test` payloads

PB §11: `<runner> <runner-native function id>`, without parametrization suffix. `result-file.md` §4.4 fixes the runner token as `[a-z][a-z0-9_-]{0,31}` — no uppercase, no space, no colon — so the split at the **first** space is exact even though a function id may itself contain spaces (vitest's `>`-joined names do). `import-resolver.md` §11.1 ratifies the four tokens v1 ships — `pytest`, `vitest`, `dart-test`, `swift-test` — one per language, for the four languages of PB §6.7 v0.19: Python, TypeScript/JavaScript, Dart, Swift. (`gradle` is reserved and unusable: Kotlin was dropped, IR §11.1.) The vectors below use `vitest` (whose `fn == id`, IR §11.3) and `pytest` (whose suffix rule is IR §11.2).

**Ids are runner-native and are never rewritten** (`result-file.md` §6.1). No escaping is applied and none is available. Two constraints follow, and the second is a defect in PB §11 (§14 D9):

- an id containing `0x0A` or `0x0D` cannot be represented in a trailer at all;
- an id containing `0x00` likewise.

`spine check --approve` refuses to freeze such an id (`test-id-unrepresentable`) rather than mangling it. A result file may carry one — `result-file.md`'s JSON strings can encode `\n` — so the refusal has to be here, at the boundary where the id becomes a line.

An id containing bytes above `0x7F` is representable and is emitted verbatim as UTF-8 (or as whatever bytes the runner produced): the line is then not pure ASCII, which §4.2's byte sort handles without comment.

### 4.5 Where `freeze=` is read

| Reader | Reads | Against |
|---|---|---|
| `spine check --approve` | computes it from the lines it is about to write | seals it into its own `Spine-Approve` line |
| `Spine-Reopen`'s `voids=` | names a `freeze=` value | the binding approval's, to void it (PB §4.3) |
| G9, merge strategy | recomputes from the approval commit reached via `Spine-Approval` | the copied `Spine-Approve` line's `freeze=` |
| G9, squash strategy | recomputes from the envelope's own copied lines | the same |
| `dump.md` §7.2 | records it as the `approval.freeze` attr | joins `voids=` against it |

`freeze=` is a **non-git digest** and is written `sha256:<hex>` under PB §11's hash policy. It is never abbreviated and never bare hex.

### 4.6 Reproducing it from a commit

From an approval commit (merge strategy) or from a squash landing — the same command, because the lines are the same lines:

```sh
git cat-file commit <A-or-L> \
  | grep -E '^Spine-(Frozen|Test): ' \
  | LC_ALL=C sort \
  | awk '{if(n++)printf "\n"; printf "%s", $0}' \
  | sha256sum
```

The `LC_ALL=C sort` is belt and braces: §2.4 already requires the lines be emitted in this order, so a conforming envelope is unchanged by it. A verifier that wants to check the *order* as well as the value drops the `sort` and compares both results.

---

## 5. `report=`

`report=sha256:<hex>` on `Spine-Review` and on `Spine-Seal` is the SHA-256 of the canonical bytes of the gate report. **`gate-report.md` owns it entirely and this document does not restate it** — not the canonicalization (GR §2, RFC 8785 JCS under GR §2.2's profile with GR §2.3's `esc`), not the schema (GR §5), not what a note publishes (GR §4.4), not what `--verify` can rebuild (GR §4).

Four things belong here because they are envelope facts:

1. **Spelling.** `sha256:` plus 64 lowercase hex digits, per PB §11's hash policy. Never bare hex, never abbreviated, never uppercase.
2. **The two occurrences name different reports.** A `Spine-Review`'s `report=` names **evaluation 1** — the non-landing report, containing the `fail` the reviewer read and accepted. The `Spine-Seal`'s `report=` names the **sealing** evaluation, in which that gate reads `override` and the review has entered `authority.reviews`. GR §9.3 settles this; the vectors carry two distinct digests to make it visible.
3. **There is no circularity, in either direction.** The review names an earlier report; the seal names a later one. And `envelope=` covers the `Spine-Review` lines that carry `report=`, so a report that contained `envelope=` would be circular — which is why GR §7 rule 11 forbids it. GR §7 rule 11's stated *reason* is imprecise and §15 corrects it.
4. **A landing's report is published to `refs/notes/spine`, on every landing, and no gate reads it.** PB §7.4 rule 4 made publication non-optional in v0.19; GR §4.4 fixes the ref, the object and the bytes. A note is never a source (PB §7.4 rule 4, GR §7 rule 3), so nothing in this document reads one: `envelope=` and `freeze=` recompute from the commit alone, and a clone with no notes fetched verifies both.

---

## 6. Building the landing commit

### 6.1 `git commit-tree`, exactly

PB §5.4 step 4: *"build the landing commit `L` with `git commit-tree` (never `git commit` or `git merge`, whose message cleanup rewrites bytes)"*.

```sh
# merge strategy
git commit-tree "$TREE_L" -p "$B" -p "$H" -F envelope.txt

# squash strategy
git commit-tree "$TREE_L" -p "$B" -F envelope.txt
```

- `TREE_L` is a tree object the run wrote itself: `merge-tree(B, H)` with `intents/<ID>.md` removed (gated), or `merge-tree(B, H)` unchanged (quick, lifecycle), or `B`'s tree (tombstone, reseal). It is **not** any tree `git merge` would produce, which is the second and independent reason `git merge` is refused.
- `B` is the first parent, always (PB §5.5).
- `-F <file>`, or `-F -` from stdin. Not `-m`: repeated `-m` arguments are joined with a blank line between them, which is a message transformation.
- `git commit-tree` performs **no cleanup**. Verified: a message containing `#`-prefixed lines, runs of blank lines, and trailing spaces is stored byte-identically, and a message with no final `0x0A` is stored with none. §2.1 requires exactly one final `0x0A`, so supply it.
- Author and committer identity are the pipeline principal; the two dates are whatever git supplies. §6.3.

### 6.2 Why `git commit` and `git merge` are refused

Not a style preference — three demonstrated rewrites, any one of which destroys the record.

**1 · Comment stripping deletes the intent's own title.** `git commit`'s `--cleanup=default` is `strip` whenever the message is edited, and `strip` removes every line beginning with `core.commentChar` (`#` by default). The intent doc's first line is `# INT-042: Invoice totals include tax`, and its section headings are `## Goal`, `## Non-goals`, `## Acceptance criteria`, `## Touchpoints`. Running `git commit --cleanup=strip -F` over **vector A's exact message** turns the fenced block from 765 bytes into 668 and its blob id from `dfb4079e22de55ec377468b9b697fdf86085ea37` into `a3fc7bf4d8ee6d07a524b9ef6ee6ba01f89f151d`. `blob=` no longer reproduces; the sign-off's signature covers a blob that is no longer there; G9 refuses the landing. **This alone would fail every gated landing** in any repository whose `commit.cleanup` is `strip` or whose message path goes through an editor.

**2 · Whitespace cleanup collapses blank runs and strips trailing spaces.** `--cleanup=whitespace` — the default when the message comes from `-F` or `-m` — removes trailing whitespace from every line and collapses consecutive blank lines into one. Demonstrated: a fenced payload of `a\n\n\nb   \n` (9 bytes) is stored as `a\n\nb\n` (5 bytes). PB §3.3's canonical form strips trailing whitespace, so limb two of this is mostly defanged for the intent body — but a blank-line run inside a Markdown intent is legal canonical form and is silently eaten, and no canonical-form rule protects a `Spine-Test` id that ends in a space.

**3 · Hooks run.** `git commit` fires `prepare-commit-msg` and `commit-msg`, either of which may rewrite the message arbitrarily and both of which are repository code — which PB §7.4 rule 3 forbids the trusted stage to execute at all. `git commit-tree` fires neither.

**And `git merge` has a fourth, decisive problem**: it generates its own subject, applies the same cleanup, and cannot produce a tree that is the merge result *minus a file*. The landing's tree is `merge-tree(B, H)` with `intents/<ID>.md` deleted; `git merge` has no way to write it.

The same reasoning runs in the read direction. PB §5.5: the indexer reads with `git cat-file commit`, never `git log`, whose `--pretty` machinery re-wraps, re-indents and can strip. `git show`, `git log -1 --format=%B` and every porcelain rendering are out for the same reason.

### 6.3 What the commit object carries that no digest covers

| Field | Covered by | Read by |
|---|---|---|
| tree | the seal's `tree=` | G9 (PB §11: *"`tree=` names `L`'s tree, so G9 checks it from `L` alone"*) |
| first parent | the seal's `base=` | G9 |
| second parent (merge) | the seal's `head=` (as `Hc`) and `Spine-Approval`'s membership | G9 |
| author identity | nothing | nothing |
| committer identity | nothing | nothing |
| **author date, committer date** | **nothing** | **nothing** |
| subject line | nothing — it is derived, not hashed (§13.10) | **G9**, which recomputes it and refuses `subject-mismatch` |
| foreign trailers below `Spine-Seal-Sig` | nothing | nothing |

**The dates are the only clock in the object and the design reads none of them.** PB §7.5: *"One clock, no timestamps"* — the chain is the clock. Two runs of the same landing on the same objects produce different commit ids because the dates differ; this is not a determinism failure, because nothing anywhere compares two landing commit ids for equality. What must be reproducible is the *message*, and it is: §7 rule 1.

**A deterministic date would be a false determinism** and must not be adopted as a fix: two landings of different work would still differ in tree and parents, so pinning the date buys no equality anyone needs while inserting a fabricated fact into a git object.

---

## 7. Determinism rules, collected

Normative, and repeated here so an implementer can check against one list.

1. **The message is a total function of the objects and the policy.** Given the same intent blob, the same event commits on the branch, the same gate report, the same strategy and the same seal fields, two conforming implementations emit byte-identical messages. Everything that could vary — trailer order (§2.4), repeat order within a rank (§2.4), field order within a payload (§2.5), sort order of the frozen manifest (§4.2), path quoting (§4.3), the subject line (§13.10) — is fixed by this document. The subject is inside this rule even though it is inside no digest: it is derived, and a derived line that two implementations spell differently is a landing each refuses.
2. **No wall clock.** No trailer payload holds a time, a duration, a date, or anything derived from one. `params.timeout` is policy read from trunk and never appears in an envelope. The commit's own date fields are outside every digest and are read by nothing (§6.3).
3. **No environment.** No hostname, no runner id, no user, no locale, no path outside the repository, no process id. `git=<major.minor>` on the seal is a capability record, not an environment probe, and PB §11 caps it at major.minor for exactly that reason.
4. **No state the design forbids.** No side file, no note read as a source, no persisted graph. Both digests this document defines are computed from the commit message alone; `report=` is computed by `gate-report.md`'s rules from git objects alone.
5. **No normalization, ever.** Not of paths, not of principals, not of ids, not of line endings, not of Unicode. §2.2, §4.3.
6. **No local git config may change a byte.** `core.quotePath` is overridden (§4.3); `core.commentChar`, `commit.cleanup`, `core.autocrlf` and every hook are avoided by using `commit-tree` (§6.1) and `cat-file` (§3.3). PB §3.3 already writes two `.gitattributes` lines — `.spine/** text eol=lf` and `intents/** text eol=lf`, the single-line form having been corrected in v0.19 because git discards it whole (`intent-doc.md` §2.5, §12 D1) — and hashes the index blob, so `core.autocrlf` cannot fork the intent's identity either.
7. **Object ids** are lowercase hex at the full length the repository's `object_format` implies — 40 or 64 digits. Never abbreviated, never uppercase. PB §5.5's `9f2c…` is display, not a value.
8. **Non-git digests** are `sha256:` + 64 lowercase hex (PB §11 hash policy). `freeze=`, `report=`, `envelope=`, `dist_hash`, and `Spine-Reopen`'s `voids=`.
9. **Counters** are plain decimal, no sign, no leading zero: `bytes=`, `reopens=`, `rounds=`, `total_rounds=`, and both halves of `red=k/n`.
10. **The joins have no trailing separator.** §3.1, §4.1.
11. **No self-reference.** `envelope=` covers no line containing `envelope=`; `freeze=` covers no line containing `freeze=`; the report contains neither (GR §7 rule 11, as corrected in §15).
12. **`Spine-Gates` is a rendering of the report's `gates` array**, in that array's order, `G<n>=<status>`, single spaces, no trailing space (GR §5.6.1). That array's order is `gates[]`'s, which **GR §5.6 fixes as ascending by the integer after `G`** — so `G9` precedes `G11` precedes `G12`. It is not restated here, and it is deliberately **not** the wire order: `wires[]` and `wires=` sort by unsigned byte value over the whole token (PB §11, GR §5.6), which is why the same landing prints `G9 G11 G12` in `Spine-Gates` and `G11` before `G2` in `wires=`. A lexical `Spine-Gates` order — `G1 G10 G11 G12 G13 G14 G15 G16 G2 G3 …` — is non-conforming and changes `envelope=`. G10 never has an entry (PB §11); G6 never has one in a version-1 report (GR §5.6.2). A gated landing therefore lists **fourteen** gates and a quick-lane landing **eleven**; §13.7 tabulates them.

---

## 8. Vector A — a gated, merge-strategy landing

This is the landing PB §5.5 describes and the one `gate-report.md` §8 reports on: `INT-042`, team mode, merge strategy, `C-A3: hostile`, `C-M4: on`, `profile=container`, one reopen, one `class=tripwire` review by bob over a `G2` containment finding, with the universal rule-5 `G11` advisory wire present because precondition 0 fails under `hostile`.

**What is computed and what is fabricated, enumerated by PB §11's two hash classes.** PB §11 splits identities into *git object ids* — for intent blobs, frozen files, trees and commits — and *SHA-256 over non-git artifacts*, for `dist_hash`, the gate report, `freeze=`, `envelope=` and `B`'s transcript. Both classes carry computed and fabricated values here, so listing "object ids" alone would not cover it.

**Computed over exactly the bytes printed**, and reproduced by the commands in §8.6 against real git objects: the **intent blob id** and its `bytes=` (§8.2); **`freeze=`** (§8.2, over the seven manifest lines); **`envelope=`** (§8.3, over the fifteen lines above the seal); the three **key fingerprints** of §8.1, which `ssh-keygen -lf` reproduces from the published keys; and **every `-Sig` line**, which `ssh-keygen -Y verify` checks against those keys.

**Fabricated but well-formed git object ids:** `head=`, `base=`, `tree=` on the seal and on the review, the `Spine-Approval` commit id, and every `Spine-Frozen` blob id. **Fabricated but well-formed SHA-256 values:** both **`report=`** digests, the `voids=` on the `Spine-Reopen` line, and the **`dist_hash` inside the seal's `tool=`**. This vector tests the envelope's digest functions, not a repository.

**Two of the fabricated SHA-256 values are owed to another document, and cannot be replaced here without regenerating this vector's signatures.** `gate-report.md` §8 computes both `report=` digests for this same landing — §8.1's evaluation-1 digest is what a `Spine-Review` carries and §8.2's is what a `Spine-Seal` carries — and `manifest.md` §8.2 computes release 1.4.0's `dist_hash`. All three sit inside signed lines whose private keys are not published, so adopting them means regenerating §8.1's keyring and re-signing, exactly as §14 D3 records being done twice already. **§15 states the three values to adopt at the next regeneration and is the live filing.** Everything GR §8 owed *this* document it has taken: as of 2026-08-27 GR §8 prints this section's computed intent blob and `freeze=`, so the two documents no longer disagree about any value either of them computed.

### 8.1 The keyring and the keys

`.spine/allowed_signers` at the seal's `base=`, in git's own `allowed_signers` format (PB §7.2). These are throwaway test keys published so the vector can be verified end to end; no private key is published and none is needed to verify.

```
alice@example.com namespaces="spine-signoff@v1,spine-review@v1" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla
bob@example.com namespaces="spine-signoff@v1,spine-review@v1" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINZJBgqcpDmx19xO9D29xeFtCCUMyfe/ti+lY7c+rvim
ci@example.com namespaces="spine-seal@v1" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICE3UkO6VDz+9ag4xQScwsfdP8PHJhLh+QWkIwzUjWze
```

| Principal | Fingerprint (`ssh-keygen -lf`) |
|---|---|
| `alice@example.com` | `SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM` |
| `bob@example.com` | `SHA256:V2dasTIGWUnhlaxa7vr3Qmqpe/qYBoxi+C7Cs6yxxEs` |
| `ci@example.com` | `SHA256:eQ0ZoC+rlhZstRuvhLXwJlwqLAreBcmnyFedpvPmTVY` |

Two distinct signoff keys, so `C-A1` is **team** and `reviewer ≠ signer` binds (PB §7.2): alice signs, bob reviews.

### 8.2 The intent blob, the approval commit, and `freeze=`

The intent doc, in PB §3.3 canonical form — UTF-8, LF, no trailing whitespace, exactly one trailing newline, no line beginning `-----` or `Spine-`:

```
# INT-042: Invoice totals include tax
Owner: @alice · Template: intent@2 · Ticket: https://example.invalid/t/4471 · Constitution: v3

## Goal
Invoice totals shown to a customer include tax, so the amount on the invoice
equals the amount charged. Today the total omits tax and support reconciles
the difference by hand.

## Non-goals
- Multi-currency invoices.
- Recomputing invoices already issued.

## Acceptance criteria
AC-1: Given a line item in a taxed jurisdiction, when the invoice total is
computed, then the total includes that line's tax.
AC-2: Given a zero-rated line item, when the invoice total is computed, then
no tax is added for that line.

## Touchpoints
Expected to change: src/billing/, api/invoices.ts
Must NOT change: auth/, shared/schema/
```

```
bytes:      765   (762 characters; the three · are U+00B7, two bytes each)
lines:      21
blob:       dfb4079e22de55ec377468b9b697fdf86085ea37
```

**Verified**, by `git hash-object` over exactly those bytes.

The **approval commit**'s message. Its subject is illustrative — nothing reads it and §16 OPEN-2 asks the owner to fix a form. Its trailer block is normative: the `Spine-Approve` line here is the line copied verbatim into the envelope, and the seven manifest lines are the input to `freeze=`.

```
INT-042: approve

Spine-Event: approve
Spine-Intent: INT-042
Spine-Approve: INT-042 intent=dfb4079e22de55ec377468b9b697fdf86085ea37 base=5f8e2a10cd47b6390e5a2c8db14f70369ba2e0d7 rounds=1 total_rounds=3 reopens=1 red=5/5 freeze=sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2 signer=alice@example.com
Spine-Approve-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgxvTrxLGqws2RJ2UJVd4g85wc48KH9N6ZPi2/2ImeqVoAAAAPc3BpbmUtcmV2aWV3QHYxAAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAAQDpCQ3jAQFLU+gzT862njx6wJW7bUwJZpO+VlQplHR/4iiWpY5w717oDGruM60CTRI5cQQx+8Xo/Ufp40bawMgM=
Spine-Frozen: 0c3a7f18e2b56d94a0c7f3e18b52d6a4907c1e3f "tests/fixtures/caf\303\251.json"
Spine-Frozen: 1e9f4b7d0c3a6e589b2d4f7a1c0e3b6d8f2a5c94 vitest.config.ts
Spine-Frozen: 58d2e7c1a9b4f60d3e8c2a5b7f091d4c6e8a0b23 tests/fixtures/invoices.json
Spine-Frozen: a41b3f0c5d2e6b8a9074f1c3e5d7b90a2c4e6f81 tests/billing/invoice.test.ts
Spine-Frozen: c07e5a2b8d1f3c6e90a4b7d2f5c8e0a3b6d9f142 tests/setup.ts
Spine-Test: vitest tests/billing/invoice.test.ts > invoice totals > AC1 includes tax
Spine-Test: vitest tests/billing/invoice.test.ts > invoice totals > AC2 zero-rated lines
```

`freeze=`, over those seven lines:

```
lines:      7
join bytes: 573
freeze:     sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2

wrong value, published so a mis-implementation recognises itself:
  with a trailing LF: sha256:8262e6d9f2a911d564bd706c0c8adb33a271fc6a467fc1dd75a5c91f3009e19c
```

**Verified.** Five things in it are worth checking against:

1. **The order is by blob id, not by path.** `café.json` (`0c3a…`) first, `setup.ts` (`c07e…`) last; the path order would be `tests/billing/invoice.test.ts`, `tests/fixtures/café.json`, `tests/fixtures/invoices.json`, `tests/setup.ts`, `vitest.config.ts`.
2. **The quoted path is quoted because of its bytes, not its position.** `tests/fixtures/café.json` holds `0xC3 0xA9`, so it is wrapped and the two bytes become `\303\251` (§4.3). The surrounding `"` are part of the hashed line.
3. **All five `Spine-Frozen` lines precede both `Spine-Test` lines** — a consequence of hashing whole lines, not a rule (§4.2).
4. **`vitest` is the runner token** and the split is at the first space; the function ids themselves contain spaces, which is why `result-file.md` §4.4 forbids a space in the token.
5. **573, not 580.** Seven lines joined by six separators. The 580-byte reading is the trailing-LF error.

### 8.3 The trailer block above the seal, and `envelope=`

Fifteen lines, in the ranks of §2.4:

```
Spine-Envelope: 1
Spine-Event: land
Spine-Lane: gated
Spine-Intent: INT-042
Spine-Signoff: INT-042 blob=dfb4079e22de55ec377468b9b697fdf86085ea37 template=intent@2 constitution=v3 reopens=1 signer=alice@example.com
Spine-Signoff-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgxvTrxLGqws2RJ2UJVd4g85wc48KH9N6ZPi2/2ImeqVoAAAAQc3BpbmUtc2lnbm9mZkB2MQAAAAAAAAAGc2hhNTEyAAAAUwAAAAtzc2gtZWQyNTUxOQAAAEAclyUyeZn33w6sK7Kfb2JIRczgkOT0AECOeM0JWFmVN0JG3z9L7hgHWcwqw5my7Er5rT993xMklo8CvFJ3VzMN
Spine-Reopen: INT-042 voids=sha256:4d1e0b7c9a2f83d6540e7b1c8a95f2036d4e8b71ca03f95e2b6d178c04a3e9f5 reopens=1 reason="AC-3 was not testable as written" signer=alice@example.com
Spine-Reopen-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgxvTrxLGqws2RJ2UJVd4g85wc48KH9N6ZPi2/2ImeqVoAAAAQc3BpbmUtc2lnbm9mZkB2MQAAAAAAAAAGc2hhNTEyAAAAUwAAAAtzc2gtZWQyNTUxOQAAAEDHgHMNfvexBt2aO+YGlq1AxNp0OzWVDrHPTk4KsIZuO909pFNhW6q8mNlA8Lp5crA1fTIsxlcEf/2TfYzSwssD
Spine-Approve: INT-042 intent=dfb4079e22de55ec377468b9b697fdf86085ea37 base=5f8e2a10cd47b6390e5a2c8db14f70369ba2e0d7 rounds=1 total_rounds=3 reopens=1 red=5/5 freeze=sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2 signer=alice@example.com
Spine-Approve-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgxvTrxLGqws2RJ2UJVd4g85wc48KH9N6ZPi2/2ImeqVoAAAAPc3BpbmUtcmV2aWV3QHYxAAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAAQDpCQ3jAQFLU+gzT862njx6wJW7bUwJZpO+VlQplHR/4iiWpY5w717oDGruM60CTRI5cQQx+8Xo/Ufp40bawMgM=
Spine-Approval: 5c9e2b71a04df836e15c9a2b7d0f43e618ca50d9
Spine-Review: INT-042 class=tripwire head=77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9 tree=3e91c7a2d0f46b58e19d73c0a5b284fd61e0c9a7 base=7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51 intent=dfb4079e22de55ec377468b9b697fdf86085ea37 report=sha256:b2f4c60e1a97d385c0b64e2f79a1d08c3e5b7f92a4160d8ce73b295f0a4d6e18 wires=G11,G2:src/shared/util.ts reason="shared helper touched outside touchpoints; read the diff and the outcomes" reviewer=bob@example.com
Spine-Review-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAg1kkGCpykObHX3E70Pb3F4W0IJQzJ97+2L6Vjtz6u+KYAAAAPc3BpbmUtcmV2aWV3QHYxAAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAAQMp+m8/m1OtyVV86kxzsWC+vbr9Cw3O0yvmTjqjkcliRdfxVglsu9II/6KjaZnpcNBYQF92QWNy5j6vszSwVtQ4=
Spine-Gates: G1=pass G2=override G3=pass G4=pass G5=pass G7=pass G8=pass G9=pass G11=pass G12=pass G13=pass G14=pass G15=pass G16=pass
Spine-Strategy: merge
```

```
lines above the seal: 15
join bytes:           2379
envelope:             sha256:e1652897b251b001fe7e03e343d40bbdc7fb9b112ef920c8b53987916b14682f

wrong value, published so a mis-implementation recognises itself:
  with a trailing LF: sha256:a0c024c23ffad492901a72ee2fa48537793b64d0a74506b33dfccd070f5e0ac5
```

**Verified**, over exactly those fifteen lines joined by fourteen `0x0A`. Seven things it pins:

1. **The five `-Sig` lines are inside.** Remove them and the digest changes; §3.2 question 1.
2. **The fenced intent block is outside.** It contributes no line beginning `Spine-`; §3.2 question 2.
3. **The subject is outside**; §3.2 question 3.
4. **`Spine-Seal` and `Spine-Seal-Sig` are outside**; §3.2 question 6.
5. **`wires=G11,G2:src/shared/util.ts`** — PB §11's order, ascending by unsigned byte value over the whole token, so `G11` precedes `G2` (GR §5.6, §6.1, §6.2). This is the order PB §5.5's example prints, and it is the one signed here. A **numeric** order — `G2:src/shared/util.ts,G11` — yields a byte-different `Spine-Review` line, a different `Spine-Review-Sig`, and a different `envelope=`, while leaving every byte count in this section unchanged (§14 D3).
6. **Fourteen gates, no G6, no G10**, ascending numerically so `G9 G11 G12` and not `G11 G12 G13 … G9` (§7 rule 12, §13.7).
7. **`Spine-Frozen`/`Spine-Test` are absent** — this is merge strategy, and PB §11 confines them to squash. §11 below is the same landing with them present, and its digest is different.

### 8.4 The seal

```
Spine-Seal: INT-042 base=7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51 head=77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9 tree=8b47d0e6c2a915f37e04b8d1c6a2f905e37b1d48 report=sha256:e70a3c92d1b845f6027e9ab3c5d10f684a2b7e93c60d5f81a34b0e29d7c6f105 tool=1.4.0+sha256:41d0e9b7c2a6538f10bd47e29c05a3f6b81d24e70c9a5b3f68d1027ae4c95b3d git=2.45 mode=team threat=hostile profile=container envelope=sha256:e1652897b251b001fe7e03e343d40bbdc7fb9b112ef920c8b53987916b14682f signer=ci@example.com
Spine-Seal-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgITdSQ7pUPP71qDjFBJzCx90/w8cmEuH5BaQjDNSNbN4AAAANc3BpbmUtc2VhbEB2MQAAAAAAAAAGc2hhNTEyAAAAUwAAAAtzc2gtZWQyNTUxOQAAAECTtFvHU1geKXCGNqRVUb5Ta85Fjn1RdWSKyfBe+B+CeWhwxk6Ip6uuuQpNAL8KCwJYjXveIoflI5uq1pE14AAN
```

**The seal's `tree=` is not the review's `tree=`, and both are correct.** The review names `T` — `3e91c7a2…`, the synthetic merge every gate evaluated, with `intents/INT-042.md` still in it. The seal names `L`'s tree — `8b47d0e6…`, that same tree with the intent file deleted (PB §11: *"`tree=` names `L`'s tree, so G9 checks it from `L` alone"*; PB §6.3 G9 compares a review's `tree=` against `merge-tree(review.base, L^2)`). Under merge strategy they differ by exactly one deleted file. PB §5.5's example elides both as `tree=…` and hides it; §14 D6.

**The two `report=` values differ**, and must: `b2f4c60e…` on bob's review is evaluation 1, in which `G2` reads `fail` and `authority.reviews` is empty; `e70a3c92…` on the seal is the sealing evaluation, in which `G2` reads `override` and bob's review is in the array (GR §8.1, GR §9.3). An implementation that puts one digest in both places has collapsed two evaluations into one.

### 8.5 The complete message

Forty-three lines, 4031 bytes, ending with exactly one `0x0A`.

```
INT-042: Invoice totals include tax

-----BEGIN SPINE-INTENT blob=dfb4079e22de55ec377468b9b697fdf86085ea37 bytes=765-----
# INT-042: Invoice totals include tax
Owner: @alice · Template: intent@2 · Ticket: https://example.invalid/t/4471 · Constitution: v3

## Goal
Invoice totals shown to a customer include tax, so the amount on the invoice
equals the amount charged. Today the total omits tax and support reconciles
the difference by hand.

## Non-goals
- Multi-currency invoices.
- Recomputing invoices already issued.

## Acceptance criteria
AC-1: Given a line item in a taxed jurisdiction, when the invoice total is
computed, then the total includes that line's tax.
AC-2: Given a zero-rated line item, when the invoice total is computed, then
no tax is added for that line.

## Touchpoints
Expected to change: src/billing/, api/invoices.ts
Must NOT change: auth/, shared/schema/
-----END SPINE-INTENT-----

Spine-Envelope: 1
Spine-Event: land
Spine-Lane: gated
Spine-Intent: INT-042
Spine-Signoff: INT-042 blob=dfb4079e22de55ec377468b9b697fdf86085ea37 template=intent@2 constitution=v3 reopens=1 signer=alice@example.com
Spine-Signoff-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgxvTrxLGqws2RJ2UJVd4g85wc48KH9N6ZPi2/2ImeqVoAAAAQc3BpbmUtc2lnbm9mZkB2MQAAAAAAAAAGc2hhNTEyAAAAUwAAAAtzc2gtZWQyNTUxOQAAAEAclyUyeZn33w6sK7Kfb2JIRczgkOT0AECOeM0JWFmVN0JG3z9L7hgHWcwqw5my7Er5rT993xMklo8CvFJ3VzMN
Spine-Reopen: INT-042 voids=sha256:4d1e0b7c9a2f83d6540e7b1c8a95f2036d4e8b71ca03f95e2b6d178c04a3e9f5 reopens=1 reason="AC-3 was not testable as written" signer=alice@example.com
Spine-Reopen-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgxvTrxLGqws2RJ2UJVd4g85wc48KH9N6ZPi2/2ImeqVoAAAAQc3BpbmUtc2lnbm9mZkB2MQAAAAAAAAAGc2hhNTEyAAAAUwAAAAtzc2gtZWQyNTUxOQAAAEDHgHMNfvexBt2aO+YGlq1AxNp0OzWVDrHPTk4KsIZuO909pFNhW6q8mNlA8Lp5crA1fTIsxlcEf/2TfYzSwssD
Spine-Approve: INT-042 intent=dfb4079e22de55ec377468b9b697fdf86085ea37 base=5f8e2a10cd47b6390e5a2c8db14f70369ba2e0d7 rounds=1 total_rounds=3 reopens=1 red=5/5 freeze=sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2 signer=alice@example.com
Spine-Approve-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgxvTrxLGqws2RJ2UJVd4g85wc48KH9N6ZPi2/2ImeqVoAAAAPc3BpbmUtcmV2aWV3QHYxAAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAAQDpCQ3jAQFLU+gzT862njx6wJW7bUwJZpO+VlQplHR/4iiWpY5w717oDGruM60CTRI5cQQx+8Xo/Ufp40bawMgM=
Spine-Approval: 5c9e2b71a04df836e15c9a2b7d0f43e618ca50d9
Spine-Review: INT-042 class=tripwire head=77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9 tree=3e91c7a2d0f46b58e19d73c0a5b284fd61e0c9a7 base=7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51 intent=dfb4079e22de55ec377468b9b697fdf86085ea37 report=sha256:b2f4c60e1a97d385c0b64e2f79a1d08c3e5b7f92a4160d8ce73b295f0a4d6e18 wires=G11,G2:src/shared/util.ts reason="shared helper touched outside touchpoints; read the diff and the outcomes" reviewer=bob@example.com
Spine-Review-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAg1kkGCpykObHX3E70Pb3F4W0IJQzJ97+2L6Vjtz6u+KYAAAAPc3BpbmUtcmV2aWV3QHYxAAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAAQMp+m8/m1OtyVV86kxzsWC+vbr9Cw3O0yvmTjqjkcliRdfxVglsu9II/6KjaZnpcNBYQF92QWNy5j6vszSwVtQ4=
Spine-Gates: G1=pass G2=override G3=pass G4=pass G5=pass G7=pass G8=pass G9=pass G11=pass G12=pass G13=pass G14=pass G15=pass G16=pass
Spine-Strategy: merge
Spine-Seal: INT-042 base=7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51 head=77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9 tree=8b47d0e6c2a915f37e04b8d1c6a2f905e37b1d48 report=sha256:e70a3c92d1b845f6027e9ab3c5d10f684a2b7e93c60d5f81a34b0e29d7c6f105 tool=1.4.0+sha256:41d0e9b7c2a6538f10bd47e29c05a3f6b81d24e70c9a5b3f68d1027ae4c95b3d git=2.45 mode=team threat=hostile profile=container envelope=sha256:e1652897b251b001fe7e03e343d40bbdc7fb9b112ef920c8b53987916b14682f signer=ci@example.com
Spine-Seal-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgITdSQ7pUPP71qDjFBJzCx90/w8cmEuH5BaQjDNSNbN4AAAANc3BpbmUtc2VhbEB2MQAAAAAAAAAGc2hhNTEyAAAAUwAAAAtzc2gtZWQyNTUxOQAAAECTtFvHU1geKXCGNqRVUb5Ta85Fjn1RdWSKyfBe+B+CeWhwxk6Ip6uuuQpNAL8KCwJYjXveIoflI5uq1pE14AAN
```

```
message lines:    43
message bytes:    4031
capped quantity:  4031 of 16384  (no Spine-Frozen/Spine-Test lines to exclude)
```

**Nothing above is elided.** Every byte of the envelope is printed: the block is the commit message, byte for byte, as `git cat-file commit` returns it after its header lines. Pasting it into a file and running §8.6's commands reproduces every digest in this section. That is the point — the audit's finding was that *"the one block implementers transcribe contains an ellipsis"*, and a transcribable block is the remedy.

### 8.6 Reproducing every value from the commit

With `L` the landing commit and `A` the approval commit, in a repository holding both:

```sh
# envelope=
git cat-file commit "$L" \
  | awk '/^Spine-Seal: /{exit} /^Spine-/{if(n++)printf "\n"; printf "%s", $0}' \
  | sha256sum
# → e1652897b251b001fe7e03e343d40bbdc7fb9b112ef920c8b53987916b14682f

# freeze=  (from the approval commit; identical from a squash envelope, §11)
git cat-file commit "$A" \
  | grep -E '^Spine-(Frozen|Test): ' | LC_ALL=C sort \
  | awk '{if(n++)printf "\n"; printf "%s", $0}' \
  | sha256sum
# → 3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2

# blob= and bytes= of the fenced block
git cat-file commit "$L" \
  | sed -n '/^-----BEGIN SPINE-INTENT /,/^-----END SPINE-INTENT-----$/p' | sed '1d;$d' \
  | git hash-object --stdin
# → dfb4079e22de55ec377468b9b697fdf86085ea37
git cat-file commit "$L" \
  | sed -n '/^-----BEGIN SPINE-INTENT /,/^-----END SPINE-INTENT-----$/p' | sed '1d;$d' | wc -c
# → 765

# any signature, by hand, with no spine binary
verify() {   # $1 trailer name, $2 principal, $3 namespace
  git cat-file commit "$L" | grep "^$1: " | tr -d '\n' > /tmp/line
  { echo "-----BEGIN SSH SIGNATURE-----"
    git cat-file commit "$L" | sed -n "s/^$1-Sig: //p" | fold -w 70
    echo "-----END SSH SIGNATURE-----"; } > /tmp/sig
  ssh-keygen -Y verify -f .spine/allowed_signers -I "$2" -n "$3" -s /tmp/sig < /tmp/line
}
verify Spine-Signoff alice@example.com spine-signoff@v1
verify Spine-Reopen  alice@example.com spine-signoff@v1
verify Spine-Approve alice@example.com spine-review@v1
verify Spine-Review  bob@example.com   spine-review@v1
verify Spine-Seal    ci@example.com    spine-seal@v1
```

All five verify. `sed -n '/^-----BEGIN/,/^-----END/p'` is a convenience for a well-formed envelope; a conforming parser reads `bytes=` and takes exactly that many bytes (§2.6). `sha256sum` is `shasum -a 256` on macOS.

---

## 9. Vector B — a quick-lane landing

The minimal landing: solo mode, squash strategy, no intent, no fenced block, no sign-off, no approval, no frozen manifest. Solo mode is the minimum because PB §11's signerless overlay gives a quick-lane landing **two** `class=protected` reviews in team mode and **one** in solo.

**PB §5.5's own description of this envelope is wrong, and this vector is the correction.** PB §5.5 lists *"subject `quick: <summary>`, `Spine-Envelope`, `Spine-Event: land`, `Spine-Lane: quick`, gates, strategy, seal"* with a review only for the lifecycle case. §11's signerless overlay makes a review mandatory on **every** quick-lane landing, and the shipped defaults put a `G11` advisory wire in every wire set, which some review's `wires=` must cover. An envelope built from PB §5.5's list has neither. §14 D1.

The solo keyring — one signoff key, whose principal then holds all three namespaces (PB §11):

```
alice@example.com namespaces="spine-signoff@v1,spine-review@v1,spine-seal@v1" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla
```

The seven lines above the seal:

```
Spine-Envelope: 1
Spine-Event: land
Spine-Lane: quick
Spine-Review: quick class=protected head=e04b19a7c35d820f6e19b47a0c58d3f27e6a1b90 tree=9d17e0c4a52b836f10e7c94d2a6b58f03c1e7d46 base=2c6a91d0f4b783e5a19c07d2f6b4830e59c1a72d report=sha256:5c31e08a9b7d4f26013ac85be2947f0d6a1b3c85f70e29d4a6b81c035e7f9d24 wires=G11 reason="no signer on a quick-lane landing; auto-merge unavailable, precondition 0 unmet under threat.candidate=hostile" reviewer=alice@example.com
Spine-Review-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgxvTrxLGqws2RJ2UJVd4g85wc48KH9N6ZPi2/2ImeqVoAAAAPc3BpbmUtcmV2aWV3QHYxAAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAAQAK72bU9o5uYhml4hRhKRPA2uoogDgnd2DKIpe8NM+t+n3ldT9ImmLpeEbaI+WtQYMI8vUxgFC+Zn0TuqciwEQg=
Spine-Gates: G1=pass G2=pass G5=pass G7=pass G8=pass G9=pass G11=pass G13=pass G14=pass G15=pass G16=pass
Spine-Strategy: squash
```

```
lines above the seal: 7
join bytes:           859
envelope:             sha256:9764852ed4bd33a9eb42ca0674b88195f03eeac20df829b6e845175a449be44d
```

**Verified.** The complete message, eleven lines, 1636 bytes:

```
quick: bump tar to 6.2.2 for CVE-2026-1188

Spine-Envelope: 1
Spine-Event: land
Spine-Lane: quick
Spine-Review: quick class=protected head=e04b19a7c35d820f6e19b47a0c58d3f27e6a1b90 tree=9d17e0c4a52b836f10e7c94d2a6b58f03c1e7d46 base=2c6a91d0f4b783e5a19c07d2f6b4830e59c1a72d report=sha256:5c31e08a9b7d4f26013ac85be2947f0d6a1b3c85f70e29d4a6b81c035e7f9d24 wires=G11 reason="no signer on a quick-lane landing; auto-merge unavailable, precondition 0 unmet under threat.candidate=hostile" reviewer=alice@example.com
Spine-Review-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgxvTrxLGqws2RJ2UJVd4g85wc48KH9N6ZPi2/2ImeqVoAAAAPc3BpbmUtcmV2aWV3QHYxAAAAAAAAAAZzaGE1MTIAAABTAAAAC3NzaC1lZDI1NTE5AAAAQAK72bU9o5uYhml4hRhKRPA2uoogDgnd2DKIpe8NM+t+n3ldT9ImmLpeEbaI+WtQYMI8vUxgFC+Zn0TuqciwEQg=
Spine-Gates: G1=pass G2=pass G5=pass G7=pass G8=pass G9=pass G11=pass G13=pass G14=pass G15=pass G16=pass
Spine-Strategy: squash
Spine-Seal: quick base=2c6a91d0f4b783e5a19c07d2f6b4830e59c1a72d head=e04b19a7c35d820f6e19b47a0c58d3f27e6a1b90 tree=9d17e0c4a52b836f10e7c94d2a6b58f03c1e7d46 report=sha256:0d84b71fe2a35c609d18a47b2e5c930f6b18d47a02c39e51f8a6b0d43c72e915 tool=1.4.0+sha256:41d0e9b7c2a6538f10bd47e29c05a3f6b81d24e70c9a5b3f68d1027ae4c95b3d git=2.45 mode=solo threat=hostile profile=none envelope=sha256:9764852ed4bd33a9eb42ca0674b88195f03eeac20df829b6e845175a449be44d signer=alice@example.com
Spine-Seal-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgxvTrxLGqws2RJ2UJVd4g85wc48KH9N6ZPi2/2ImeqVoAAAANc3BpbmUtc2VhbEB2MQAAAAAAAAAGc2hhNTEyAAAAUwAAAAtzc2gtZWQyNTUxOQAAAEAQjNpu4BY7x7D5Ijiry6227+20foMZwkiaSzV9eC6W0yeCmB6tainJciyThFt8dq3YUXLg2P22n1gEsS9h6fAE
```

Both signatures verify against the solo keyring. Six things this vector pins that vector A does not:

1. **The first field of `Spine-Review` and of `Spine-Seal` is `quick`**, not an intent id (PB §11), and `intent=` is absent from the review.
2. **The seal's `tree=` equals the review's `tree=`** here, unlike vector A: a quick-lane landing deletes no intent file, so `L`'s tree *is* `T`. Both readings of "tree" coincide, which is exactly why the distinction is easy to miss in vector A.
3. **Eleven gates.** G3, G4 and G12 read an in-flight intent or an approval and a quick landing has neither (GR §5.6.2); G6 and G10 never appear.
4. **`class=protected`, not `tripwire`,** although the only wire is a `tripwire`-class `G11` advisory. PB §11's signerless overlay is evaluated after aggregation and only ever raises: it sets the class and the cardinality of the reviews, whatever the wire set produced.
5. **`profile=none` and `mode=solo`** are the honest solo-laptop values (PB §5.4): rule 5's preconditions 1 and 2 fail by construction, which is why `wires=G11` is present and why the landing takes a human's signature.
6. **`self_approved` is not a trailer field.** A signerless landing has no signer to be self, so GR §5.5's `self_approved` is `false` in the report and nothing appears in the envelope.

---

## 10. Vector C — the `freeze=` sort

Debug your comparator against this before attempting §8.2. It exercises every tie-break the sort has, both path encodings' boundary cases, and nothing else. It is a fragment, not a commit.

Authored, in an arbitrary order:

```
Spine-Test: vitest web/x.test.ts > totals > AC2 zero-rated
Spine-Frozen: 7f3aa0c19b48d6250e3f7a1c85b09d24e6f31a70 tests/z.py
Spine-Test: pytest tests/test_tax.py::test_AC1_totals
Spine-Frozen: 0a12f7d3e5b96c08a41d7e2f39c05b6a8d14e037 "tests/caf\303\251.py"
Spine-Frozen: 7f3aa0c19b48d6250e3f7a1c85b09d24e6f31a70 tests/a b.py
Spine-Test: vitest web/x.test.ts > totals > AC10 rounding
```

Sorted:

```
Spine-Frozen: 0a12f7d3e5b96c08a41d7e2f39c05b6a8d14e037 "tests/caf\303\251.py"
Spine-Frozen: 7f3aa0c19b48d6250e3f7a1c85b09d24e6f31a70 tests/a b.py
Spine-Frozen: 7f3aa0c19b48d6250e3f7a1c85b09d24e6f31a70 tests/z.py
Spine-Test: pytest tests/test_tax.py::test_AC1_totals
Spine-Test: vitest web/x.test.ts > totals > AC10 rounding
Spine-Test: vitest web/x.test.ts > totals > AC2 zero-rated
```

```
lines:      6
join bytes: 382
digest:     sha256:bbf3ba10080d190a1ba224483f4ad760083efa861d073f7a0d5f16df92bf45d4
```

**Verified**, over the sorted block only, six lines joined by five `0x0A`, no trailing `0x0A`. What it pins:

- **Frozen before Test**, from the whole-line comparison alone — `F` < `T` (§4.2);
- **oid-major order within `Spine-Frozen`**: `0a12…` before `7f3a…`, and the quoted `café.py` first *because of its oid*, not because `"` sorts low;
- **the path breaks the tie when two blobs are identical**: `7f3a…` appears twice — the same content at two paths — and `tests/a b.py` precedes `tests/z.py`;
- **a space in a path does not trigger quoting** and does not break the parse: `tests/a b.py` is literal, and the payload splits at its first space (§4.3);
- **runner-major order within `Spine-Test`**: `pytest` before `vitest`, so a multi-runner repository's manifest is grouped by runner without a grouping rule;
- **byte order, not numeric order**, within one runner: `AC10` precedes `AC2`. An implementation that sorts ids "naturally" produces a different digest over identical facts. (`dump.md` §12.4 pins the same trap for the same reason.)

---

## 11. Vector D — the same landing under squash

Repository policy differs in one line — `C-M1: merge.strategy = squash` — and PB §5.5 therefore copies the frozen manifest into the envelope. Everything else is vector A.

The envelope's above-seal block is §8.3's fifteen lines with two changes:

1. the seven manifest lines of §8.2 inserted at ranks 11 and 12, immediately after `Spine-Approval` and before `Spine-Review`, in the same `freeze=` sort order they carry on the approval commit;
2. `Spine-Strategy: merge` becomes `Spine-Strategy: squash`.

```
lines above the seal: 22
join bytes:           2954
envelope:             sha256:9895816bcbc90400ac90cec50bbb6eec516e26712097c4eef1877ea739bfcc4b
```

The seal, resealed over the new digest:

```
Spine-Seal: INT-042 base=7b0d1f4a9c2e6b8d05f3a71c4e9b2d6f8a0c3e51 head=77aa3c19e5b48f0d2617ca93b4e5f8d70a1c62b9 tree=8b47d0e6c2a915f37e04b8d1c6a2f905e37b1d48 report=sha256:e70a3c92d1b845f6027e9ab3c5d10f684a2b7e93c60d5f81a34b0e29d7c6f105 tool=1.4.0+sha256:41d0e9b7c2a6538f10bd47e29c05a3f6b81d24e70c9a5b3f68d1027ae4c95b3d git=2.45 mode=team threat=hostile profile=container envelope=sha256:9895816bcbc90400ac90cec50bbb6eec516e26712097c4eef1877ea739bfcc4b signer=ci@example.com
Spine-Seal-Sig: U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgITdSQ7pUPP71qDjFBJzCx90/w8cmEuH5BaQjDNSNbN4AAAANc3BpbmUtc2VhbEB2MQAAAAAAAAAGc2hhNTEyAAAAUwAAAAtzc2gtZWQyNTUxOQAAAEBauIwAWLPIexxySK5bLiRfcDMnNFvq6L8jQ8Q5iC8KzjXXK9TxzdB5XtyACEJ5ZFGEhUdYvpEpTL4DvI6K8tIH
```

```
message lines:    50
message bytes:    4606
capped quantity:  4032 of 16384
```

**Verified**, including the seal's signature. Four things it pins:

1. **`Spine-Frozen` and `Spine-Test` lines are inside `envelope=`.** They begin `Spine-` and sit above the seal, so §3.1 admits them without exception; the digest is not vector A's. There is no "manifest lines are outside the digest" rule, and an implementation that invented one to mirror the 16 KiB carve-out would produce vector A's digest for vector D's envelope.
2. **They are outside the 16 KiB cap.** 4606 message bytes, 4032 capped — the 567-byte manifest plus its seven line terminators, 574 bytes in all, is excluded (§2.9). Inside the digest, outside the cap: two different subsets of one trailer block, and both are correct.
3. **`freeze=` recomputes from the envelope alone.** Running §8.6's freeze command against *this* commit — not the approval commit — yields `3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2`, the same value the copied `Spine-Approve` line carries. That is PB §6.3 G9's squash freeze audit, executed.
4. **`Spine-Approval` is still present** even though the approval commit is unreachable from `L`. §13.6.

The report digests are held at vector A's values so that the vector isolates exactly what changed in the digest's input; a real squash landing's reports differ from a real merge landing's.

---

## 12. Refusals

| Code | Raised by | Meaning |
|---|---|---|
| `envelope-too-large` | `--approve` (projection), `--land` (check) | the capped quantity of §2.9 exceeds 16384. Never a truncation. **Never raised for a landing whose `Spine-Event` is `reseal`**, which §2.9 exempts from the cap; the only exit for every other shape is to split the intent. |
| `envelope-malformed` | `--land`, G9 | any structural violation of §2: unknown or malformed `Spine-*` name, wrong field order, missing or duplicated seal, `Spine-*` below the seal, non-`Spine-*` inside the trailer block, blank line inside it, `0x0D` or `0x00` anywhere, a bad fence, a bad C-quoted path. |
| `envelope-version-unknown` | any reader | `Spine-Envelope` is not `1`. Refuse before computing anything. |
| `fence-mismatch` | `--land`, G9 | the fenced bytes do not hash to `blob=`, or their count is not `bytes=`. |
| `subject-mismatch` | `--land`, G9 | the subject is not the line §13.10 derives for the landing's shape. On a quick-lane or lifecycle landing, only the `quick: ` prefix and a non-empty remainder are checked. |
| `digest-mismatch` | G9, `--verify` | a recomputed `envelope=`, `freeze=` or `report=` differs from the sealed value. |
| `freeze-duplicate` | `--approve` | two identical `Spine-Frozen` or `Spine-Test` lines (§4.2). |
| `test-id-unrepresentable` | `--approve` | a `Spine-Test` function id contains `0x0A`, `0x0D` or `0x00` (§4.4). |

At `--land`, every one of these is a refusal before anything is sealed. On an already-landed commit, every one makes G9 index the landing `unattested` — reported and counted forever, never silently repaired (PB §6.3 G9). A landing that fails any of them is not fixed by editing the message: that would rewrite the commit, which the non-fast-forward rule on trunk denies. The repair is a reseal (PB §5.5).

---

## 13. Resolved ambiguities

Each entry states what the playbook says, what this document chose, and why.

### 13.1 Whether `-Sig` lines are inside `envelope=`

**Playbook:** PB §11 says *"every `Spine-* `line above, in order, LF-joined"*; PB §5.5 says *"every `Spine-*` line above it"*. Neither mentions `-Sig` lines, and PB §5.5's example annotates them as commentary (*"← human sign-off, copied verbatim"*), which reads like a hint that they are decoration.
**Chosen:** inside, all of them except `Spine-Seal-Sig`, which is below the seal.
**Why:** they are `Spine-*` lines and §11 has no exception clause; and it is the stronger binding — with them inside, `envelope=` fixes which signature accompanied each statement, not merely that a verifying one did. Excluding them would also require a rule about *which* `Spine-*` lines are excluded, which is a second thing to disagree about.

### 13.2 Whether the join carries a trailing LF

**Playbook:** "LF-joined" (PB §11); nothing at all for `freeze=` (PB §4.3).
**Chosen:** no trailing LF, for both digests. `n` lines, `n − 1` separators.
**Why:** §3.2 question 4. Briefly: it is what *joined* means; it matches `gate-report.md` §2.1's "no trailing newline, no BOM, no framing"; and it makes the single-line self-test hold. The wrong value is published beside the right one in §8.2 and §8.3 so a mis-implementation is diagnosable in one comparison rather than by bisecting a hash function.

### 13.3 Whether the fenced block and the subject are inside `envelope=`

**Playbook:** PB §5.5 prints them above the trailers and says the digest covers "every `Spine-*` line", without saying what makes a line one.
**Chosen:** both outside. Selection is the lexical predicate `^Spine-` (§2.3), which neither region can satisfy — the fence delimiters begin `-----`, and PB §3.3 forbids the intent body a line beginning `Spine-` or `-----`.
**Why:** the fenced block is already bound, by `blob=` inside a signed line inside the digest; a second binding buys nothing. The subject is bound by no digest either, and the owner settled on 2026-08-26 that it stays that way: it is **derived** and G9 recomputes it, which closes the gap without moving a byte of `envelope=` — §13.10, §14 D10, §16 OPEN-1.

### 13.4 What `freeze=` is computed over

**Playbook:** PB §4.3, *"a SHA-256 over the sorted `Spine-Frozen` and `Spine-Test` lines"*. No collation, no byte range, no join, no statement about how the two kinds interleave.
**Chosen:** whole lines including the trailer name, unsigned-byte ascending, LF-joined with no trailing LF (§4.1, §4.2).
**Why:** hashing whole lines is what "the lines" says; it makes the two kinds segregate with no interleave rule; and it means no implementation has to unquote a path before hashing, so a quoting disagreement cannot become a digest disagreement. The alternative — sort by a parsed key such as `(path, oid)` — was rejected because it requires unquoting before sorting, which puts the C-quoting rule on the critical path of the digest twice instead of once.

### 13.5 The frozen manifest is ordered by blob id, and that is intended

**Playbook:** silent.
**Chosen:** a consequence of §4.1, accepted rather than patched.
**Why:** `Spine-Frozen: <oid> <path>` puts the oid first, so a whole-line sort is oid-major. It reads oddly in a 200-file closure and it is the price of a rule that needs no parse. A review packet (PB §6.5) is free to display the closure by path; the *digest input* is this order, and the emission order matches it so that a verifier can check the order as cheaply as the value.

### 13.6 Whether `Spine-Approval` appears under squash

**Playbook:** PB §11 lists it under "gated landing" with the parenthetical *"(∈ `M(L)` under merge)"*; PB §5.5 says the approval commit becomes unreachable under squash.
**Chosen:** present under both strategies. The membership check applies only under merge, which is what the parenthetical restricts.
**Why:** the sha remains useful audit data — a clone that still holds the branch can reach the commit — and dropping it would make the trailer set differ by strategy for no gain, which is one more branch in every reader. Nothing reads it under squash, and G9's squash branch audits the freeze from the copied lines instead (PB §6.3 G9).

### 13.7 Which gates have `Spine-Gates` entries

**Playbook:** PB §11 says "every gate that ran, never G10"; PB §5.5's example writes `G1=pass G2=override G3=pass … G16=pass`, whose ellipsis implies a contiguous run; PB §5.4 step 2 enumerates only the tombstone's four.
**Chosen:** `gate-report.md` §5.6.2's table, rendered by GR §5.6.1 in the report's array order — ascending by gate number.

| Landing | `Spine-Gates` entries |
|---|---|
| gated | `G1 G2 G3 G4 G5 G7 G8 G9 G11 G12 G13 G14 G15 G16` (14) |
| quick / lifecycle | `G1 G2 G5 G7 G8 G9 G11 G13 G14 G15 G16` (11) |
| reseal | `G1 G2 G5 G7 G8 G9 G11 G13 G14 G15 G16` (11) |
| tombstone | `G9 G13 G14 G15` (4) |

**Why:** G10 is excluded by PB §11 (it runs after the seal); G6 is excluded from every version-1 report by GR §5.6.2 (nothing in the playbook says where a repository configures it, so "iff configured" would make two implementations disagree about the length of the line — and therefore about `envelope=`). G3, G4 and G12 read an in-flight intent or an approval, which a subjectless landing has neither of. §14 D5 files the ellipsis.

### 13.8 What a signature covers

**Playbook:** PB §7.2 and PB §11, *"over that line's exact bytes"*.
**Chosen:** from the first byte of the trailer name through the last byte before the terminating `0x0A`, terminator excluded — the same byte range `dump.md` §5.2.1 hashes for an `approval` node id, which that document defined independently *"so that it does not move when `envelope-vectors.md` fixes the signed payload"*. It does not move.
**Why:** the trailer name must be inside, or a `Spine-Approve` payload could be replayed as a `Spine-Signoff` with the same signature. §8.6 verifies all five signatures under this reading against real commit objects.

### 13.9 Two path encodings in one envelope

**Playbook:** PB §4.3 and PB §11 mandate `git ls-tree` quoting for `Spine-Frozen`; `gate-report.md` §6.2 mandates `tok` for a wire token and instructs this document to adopt it verbatim.
**Chosen:** both, unchanged, side by side (§2.5). `tests/fixtures/café.json` is `"tests/fixtures/caf\303\251.json"` in a `Spine-Frozen` line and `tests/fixtures/caf\xc3\xa9.json` inside a `G8:` wire token, in the same commit message.
**Why:** each is normative in its own home and this document has no authority to unify them. The asymmetry is a genuine hazard — an implementation that reuses one encoder for both produces lines no conforming implementation reproduces — so it is named here rather than left to be discovered. Unifying them is possible in a later version and would be a change to PB §11 and to GR §6.2, not to this document.

### 13.10 The subject line, how it is derived, and what checks it

**Playbook:** PB §5.5, as amended 2026-08-26: *"The subject is derived, not written. A landing's first line is a pure function of its envelope … and G9 recomputes it and refuses a landing whose subject it did not produce. It stays outside `envelope=`, so no digest changes and no existing signature moves."* The owner's decision closes §16 OPEN-1 and §14 D10. PB §11's *Subject lines* paragraph now names the derivation for all five shapes — gated landing and tombstone: the fenced intent's first line with its leading `# ` removed; quick lane and every toolkit lifecycle landing: `quick: ` and a one-line free-text summary, of which only the prefix and a non-empty remainder are checked; reseal: `reseal: ` and the full object id of the orphan tip `O`, which is the seal's `head=`. **The table below is those five forms and nothing else**; where a word of it could be read against PB §11, §11 wins. What this section adds, and what PB does not say, is *which sealed bytes G9 recomputes each form from* and what it checks when a form is not derivable.

**Chosen — the derivation, per landing shape.** The shape is read from `Spine-Event` and `Spine-Lane`, which are mandatory on every landing, sit above the seal, and are inside `envelope=` — so the input a verifier selects the derivation by is itself sealed.

| Shape | Subject | What G9 recomputes it from |
|---|---|---|
| gated (`Spine-Lane: gated`) | the fenced block's first line with the leading `# ` removed | the fenced block, whose bytes `blob=` binds on a signed line inside `envelope=` |
| tombstone (`Spine-Event: withdraw`) | the same | its own fenced block, bound by `Spine-Withdraw`'s `blob=` |
| quick (`Spine-Lane: quick`, no `Spine-Upgrade`) | `quick: ` + a one-line summary | **the prefix only** — the summary is free text |
| toolkit lifecycle (`Spine-Lane: quick` with `Spine-Upgrade` present) | `quick: ` + a one-line summary | the same: a lifecycle landing rides the quick lane |
| reseal (`Spine-Event: reseal`) | `reseal: ` + the object id of the orphan tip `O`, in full | the seal's `head=`, which PB §5.5 makes `O` on a reseal |

**What G9 checks, exactly.** For the three derivable shapes it recomputes the byte string and compares it to the subject byte for byte; a difference is `subject-mismatch` (§12), which at `--land` is a refusal before anything is sealed and on an already-landed commit indexes the landing `unattested` like every other code in that table. For the two quick-lane shapes it checks only that the line begins with the seven bytes `quick: `, that at least one further byte follows, and — as §2.2 already requires of every line — that it carries no `0x0D`. Nothing else about a quick-lane subject is decidable, and G9 does not pretend otherwise.

**Two things this fixes that PB's sentence leaves open.**

1. **The gated form is a derivation, not a template.** PB spells it `<id>: <the intent's title>`, which would have an implementation parse the intent's first line into an id and a title and rejoin them. Taking the whole first line minus its `# ` yields the same string for any conforming intent doc — PB §3.3 fixes the `# <ID>: <title>` form — and needs no parse, so two implementations cannot disagree about where the id ends.
2. **The reseal form carries the full object id, not an abbreviation** — and PB now agrees. Version 1 of this section fixed the full id against a PB §5.5 that spelled it `<short-sha of O>`: no document fixes an abbreviation length, `core.abbrev` is a per-clone setting, and git's own default varies with repository size, so an abbreviation would make a *derived* subject a function of the reader's configuration and break §7 rule 1 on the one shape whose subject is fully computable. §7 rule 7's *never abbreviated* governs every other object id here and governs this one. PB §5.5 now delegates the tombstone and reseal forms to PB §11, and PB §11 reads *"`reseal: ` and the full object id of the orphan tip `O`, which is the seal's `head=`"* — byte-identical to the table above. §14 D12(a) is closed; D12(b) is not.

**Why a derivation and not a digest.** Folding the subject into `envelope=` binds it exactly and costs every implementation's digest function; the owner declined that and took the gate rule, which buys most of the guarantee for a check `--land` and G9 already perform. The price is stated rather than hidden: **the quick lane's summary is free text, and PB §11 routes every toolkit lifecycle landing through the quick lane**, so an upgrade, a rollback or an uninstall can land under the subject `quick: update deps` with every signature verifying and every gate green. Nothing the record *attests* is falsified by that — the sealed `Spine-Upgrade` line inside `envelope=` says what actually happened, and `spine stats` counts it — but the first line of `git log` can mislead, and a reader who trusts subjects over trailers should know that this is the one place the design lets them down.

### 13.11 The event commits' own messages

**Playbook:** PB §4.3 says the approval is *"a signed, empty commit on the intent branch"* and shows its trailers; no subject, no message shape, for any event commit.
**Chosen:** this document fixes the **trailer block** of an approval commit — because `freeze=` and PB §6.3 G9 read it — as: `Spine-Event`, `Spine-Intent`, `Spine-Approve` + `-Sig`, then the manifest in `freeze=` order. It does **not** fix any event commit's subject; §8.2 prints one and labels it illustrative.
**Why:** nothing reads a subject, no digest covers one, and inventing a form here would be inventing where the owner should choose. §16 OPEN-2.

### 13.12 "Ignored" foreign trailers

**Playbook:** PB §5.5, *"Foreign trailers a provider appends after the seal are outside the digest and ignored."*
**Chosen:** *outside the digest* for a non-`Spine-*` line after `Spine-Seal-Sig`; **refused** for any `Spine-*` line below the seal, and for any non-`Spine-*` line inside the trailer block (§2.8).
**Why:** "ignored" is safe only if every reader ignores it, and an indexer that greps the whole message for `Spine-Review` does not. PB §11 already makes the seal-plus-`-Sig` the last `Spine-*` lines; refusing is how that definition is enforced rather than merely stated. §14 D7.

---

## 14. Defects found in PLAYBOOK.md v0.19

Reported here rather than repaired, per `docs/spec/README.md`: where this document and PB §11 disagree, §11 wins and the disagreement is a defect in one of them. **Citations are section anchors plus a verbatim quote, never line numbers** — a line number rots as the playbook grows, a section and a quote do not. Every entry is marked **OPEN**, **CLOSED** or **WITHDRAWN** against `PLAYBOOK.md` as it now stands.

**D1 · CLOSED · §5.5's quick-lane envelope omitted the review that §11 makes mandatory, and as written never landed** (PB §5.5, *"Every trunk commit is sealed"*). **As filed**, PB §5.5 read *"A quick-lane change lands with a minimal envelope (subject `quick: <summary>`, `Spine-Envelope`, `Spine-Event: land`, `Spine-Lane: quick`, gates, strategy, seal — plus, **on a toolkit lifecycle landing**, the copied `Spine-Upgrade` + `-Sig` and its protected `Spine-Review` + `-Sig` …)"* — a review only for the lifecycle case. §11's signerless overlay is unconditional: *"A landing with no signer — **every quick-lane landing**, every reseal — carries **at least two** distinct `class=protected` reviews in team mode … in solo mode … it carries one."* It fails twice over: the signerless overlay is unsatisfied, and under the shipped defaults the universal rule-5 `G11` advisory wire is in every wire set (§11, *Wire aggregation*) with no review's `wires=` to contain it. An implementation that built §5.5's list produced an envelope every landing rejects. The recommendation was that §5.5's parenthetical name the review for every quick-lane landing, not only the lifecycle one. **Taken:** PB §5.5's parenthetical now reads *"gates, strategy, seal, **and the copied `Spine-Review` + `-Sig` that every quick landing carries** — one in solo mode, two from distinct keys in team mode, because a quick landing has no signer and §11's signerless overlay applies to it whatever its wires say"*. §9 is that shape, and needs no change.

**D2 · CLOSED — fixed in PB v0.19; withdrawn.** As filed against v0.18: the annotation on `Spine-Review-Sig` read *"← `G1` is in `wires=` because this seal says `threat=hostile`"* while the line above it already read `wires=G11,…`, in the one block implementers transcribe. **PB §5.5's annotation now reads `G11`**, agreeing with the line it annotates, with §11's categorical *"It is never spelled `G1`"*, and with §7.4 rule 5. Nothing remains to correct.

**D3 · WITHDRAWN — the playbook was right and this filing was wrong** (PB §5.5's envelope block, *"`wires=G11,G2:src/shared/util.ts`"*). This defect claimed §5.5's `wires=G11,G2:src/shared/util.ts` was mis-sorted, on the strength of `gate-report.md` §6.2's numeric order and GR §9.19. Both have since been withdrawn: **§11 fixes the order in the `Spine-Review` row itself** — *"ascending by unsigned byte value over the whole token, so `G11` precedes `G2`"* — so `wires=G11,G2:src/shared/util.ts` was always the conforming spelling and GR §6.2 was the non-conforming document. Nothing in the playbook needs correcting here. **This vector was recomputed when the order was adopted**: §8.3's review line, its `Spine-Review-Sig`, §8.3's and §11's `envelope=`, and both seals' signatures all moved, and §8.1's keyring was regenerated because the signed review line moved and no private key is published. Every byte count in §8 and §11 is unchanged — the re-sort is a permutation — which is precisely why the divergence survived two reviews.

**D4 · CLOSED — fixed in PB v0.19; withdrawn.** As filed: §5.5's example printed `Spine-Frozen` and `Spine-Test` annotated *"← squash strategy only"* inside a block ending `Spine-Strategy: merge`, so the one block implementers transcribe was internally inconsistent and the annotation read as a footnote rather than an exclusion. **PB §5.5 now takes the second option**: the merge block carries neither line, and the two are printed separately below it, in their rank between `Spine-Approval` and `Spine-Review`, annotated *"squash only; absent from the merge block above"*. §8.3's fifteen-line block here is the merge shape and matches; §11's vector is the squash shape and carries both.

**D5 · CLOSED — fixed in PB v0.19; withdrawn.** As filed: `Spine-Gates: G1=pass G2=override G3=pass … G16=pass` read as a contiguous run G1…G16, while §11 excludes G10, `gate-report.md` §5.6.2 excludes G6 from every version-1 report, and G3, G4 and G12 are absent from every subjectless landing. **PB §5.5 now prints the fourteen** — `G1=pass G2=override G3=pass G4=pass G5=pass G7=pass G8=pass G9=pass G11=pass G12=pass G13=pass G14=pass G15=pass G16=pass` — byte-identical to §8.3's line here and to `gate-report.md` §8.2's rendering, and adds the sentence that a quick landing lists eleven. **This was the ellipsis the pre-implementation audit named, and it is closed.**

**D6 · OPEN · The seal's `tree=` and a review's `tree=` name different objects and §5.5 elides both identically** (PB §5.5's envelope block still prints `tree=…` on both the `Spine-Review` and the `Spine-Seal` line; PB §11's `Spine-Seal` row). §11: the seal's `tree=` names `L`'s tree. §6.3 G9: a review's `tree=` equals `merge-tree(review.base, L^2)`, which is `T`. Under merge strategy they differ by the deleted intent file; both are printed as `tree=…`. GR §9.2 resolved it for the report; the envelope example still hides it, and it is where an implementer copies from. Recommended: spell both in §5.5's example.

**D7 · OPEN · "Ignored" is the wrong disposition for a `Spine-*` line below the seal** (PB §5.5, *"Foreign trailers a provider appends after the seal are outside the digest and ignored."*). *"Foreign trailers a provider appends after the seal are outside the digest and ignored."* Outside the digest is right. *Ignored* is a licence for an indexer that greps the whole message to read an unsigned, unsealed `Spine-Review` or `Spine-Gates` line appended below the seal — which §11's own definition of the seal as the last `Spine-*` line already forbids, but which nothing in the text refuses. Recommended: "…are outside the digest and ignored; a `Spine-*` line below the seal other than its own `-Sig` makes the envelope malformed."

**D8 · OPEN · §11's `envelope=` gloss admits a self-referential reading** (PB §11's `Spine-Seal` + `-Sig` row). The field is spelled *"`envelope=sha256:<hex over every Spine-* line above, in order, LF-joined>`"* in a row headed `Spine-Seal + -Sig`. Read as "above the `-Sig`", the `Spine-Seal` line — which carries `envelope=` — is inside its own digest, and the value is uncomputable. §5.5's prose ("above **it**") is the unambiguous one. Recommended: §11 say "above the `Spine-Seal` line".

**D9 · OPEN · §11's `Spine-Test` payload admits ids no trailer can carry** (PB §11's `Spine-Test` row, *"`<runner> <runner-native function id>` without parametrization suffix"*). The payload is `<runner> <runner-native function id>` with no escaping and no character restriction, while `result-file.md` §4.4 lets an id be any non-empty string and its JSON framing can carry `0x0A`. A trailer line cannot. The gap is silent: a repository whose runner emits such an id gets a corrupt approval commit rather than a refusal. Recommended: §11's row add "the function id contains no LF, CR or NUL; `--approve` refuses one that does" — §4.4 and §12's `test-id-unrepresentable` here.

**D10 · CLOSED by the owner, 2026-08-26 · Nothing bound the subject line** (PB §5.4 configuration (b); PB §5.5, now *"**The subject is derived, not written.**"*). §5.5's own rule is *"Nothing is trusted because it says so; everything is trusted because it hashes"*, and the subject hashed to nothing: outside `envelope=` (which selects `Spine-*` lines), outside every `-Sig`, and unchecked by any gate. Under PB §5.4 configuration (b) the provider builds the commit from a PR body and could set any subject it liked while every signature still verified. **The owner took the derivation and not the digest**: §5.5 now makes the subject a pure function of the envelope and gives G9 the duty of recomputing it and refusing a landing whose subject it did not produce — no change to `envelope=`, no signature moved. §13.10 is the derivation for all five shapes, §12 carries `subject-mismatch`, and §14 D12 records the two loose ends the new sentence still has. What remains is an accepted residual and not a defect: the quick lane's summary is free text and every lifecycle landing rides the quick lane.

**D11 · CLOSED by the owner, 2026-08-26 · The 16 KiB cap could deadlock a reseal** (PB §5.5, *"Every trunk commit is sealed"*). Version 1 filed it: §5.5 capped the envelope and offered two escapes, *"split the intent or use merge strategy"*, and neither reached a reseal. A reseal folds **every wire in the orphan range** into one report and its review's `wires=` (§5.5), the review line was inside the cap, and in **solo** mode §11's signerless overlay gives the reseal exactly **one** review — no second review to spread the wire set across, as team mode's union rule allows. A long orphan range produced a review line over 16 KiB, `--land --reseal` refused `envelope-too-large`, and G9 *"refuses to land on top of an orphan until it is resealed"*: trunk permanently blocked, with break-glass unreachable because §7.6 offers it only from `tests-approved` onward and a reseal never enters that state. **The owner took §16 OPEN-3 option (a): the cap does not apply to a `Spine-Event: reseal` envelope.** §2.9 carries the exemption, §12's `envelope-too-large` row excludes the shape, and §18 item 24 states it as an invariant. The second half of the old escape clause went with it: switching merge strategy was never an exit — this document's own vectors move the capped quantity by one byte between `merge` and `squash` — and PB §5.5 now offers splitting the intent as the sole exit for every shape that is still capped.

**D12 · (a) CLOSED, (b) OPEN · Two loose ends in §5.5's new subject rule** (PB §5.5, *"**The subject is derived, not written.**"*). **(a) — fixed 2026-08-26, no longer a defect.** The reseal subject was spelled `<short-sha of O>`, and no document in the set fixes an abbreviation length: `core.abbrev` is a per-clone setting and git's default varies with repository size, so two conforming implementations derived different subjects for the same reseal and each refused the other's landing, on the one shape whose subject is otherwise fully computable. PB §5.5 now names no abbreviation at all — it delegates the tombstone and reseal forms to PB §11, whose *Subject lines* paragraph reads *"`reseal: ` and the full object id of the orphan tip `O`, which is the seal's `head=`"*, which is §13.10's row. Nothing remains to repair here. **(b)** The residual's own illustration — *"an uninstall can land under the subject `chore: update deps`"* — is a subject the same section's quick-lane form (`quick: <summary>`) forbids, so no conforming landing can produce it and the example undercuts the rule it illustrates. Recommended: `quick: update deps`. Neither is gate-visible today, because nothing but this document derives a subject; both become gate-visible the moment G9 does.

---

## 15. Reconciliation with `gate-report.md`

**The one correction — adopted 2026-08-27, and kept here as the record.** `gate-report.md` §7 rule 11 used to read: *"The report never contains its own digest, and never contains `envelope=` — **the seal line that carries `report=` is inside the envelope digest**, and a report containing either would be circular."* The emphasised clause was false under the definition this document fixes and under PB §5.5's prose: the `Spine-Seal` line is *below* the seal boundary and is not inside `envelope=`. **The rule was right and its stated reason was wrong.** The real circularity runs through the reviews: `Spine-Review` lines carry `report=`, they are above the seal, they *are* inside `envelope=` — so a report containing `envelope=` would depend on a digest that depends on a line that depends on the report. GR §7 rule 11 now carries that wording.

**GR §8 has taken both values this document computes. Closed.** GR §8's example is this document's vector A, and the two of its values that are computed here were adopted on 2026-08-27:

| GR §8 printed | Now prints |
|---|---|
| intent blob `9f2c8d4b1a63e05f7c29d84b6e130af52c7b9d84` | `dfb4079e22de55ec377468b9b697fdf86085ea37` (§8.2, computed over published bytes) |
| `freeze=sha256:1c7f0a3d9b62e4581f0d73ac5b28e9146fd0372b8ea5c19d604fb837a2e51c0d` | `sha256:3a8fc3095a793cdf9dfdbe1310aeff0a1dd87888aaaaf3f01edf19275fb544c2` (§8.2, computed) |

Both are fixed-width, so GR §8's byte counts did not move on their account and only its digests did. GR §8 also took §8.1's three fingerprints, which `ssh-keygen -lf` reproduces from the keys published there, so it no longer carries a fabricated one. Both documents spell `template=intent@2` under the owner's decision of 2026-08-26. **On every value either document computes, the two now agree.**

---

**What this document still owes vector A, and why it cannot be paid without regenerating the keys.** Three fabricated values in §8 belong to a document that computes them:

| §8 prints | Owner's computed value | Where it sits |
|---|---|---|
| `report=sha256:b2f4c60e1a97d385c0b64e2f79a1d08c3e5b7f92a4160d8ce73b295f0a4d6e18` (§8.3, the review line) | `sha256:e2bd8cb5da473701bf0dd8e9aa0c6a28cc1280e33ba06758165c5089ae3f5b47` (`gate-report.md` §8.1, evaluation 1) | inside `Spine-Review`, hence inside `Spine-Review-Sig` **and** inside `envelope=` |
| `report=sha256:e70a3c92d1b845f6027e9ab3c5d10f684a2b7e93c60d5f81a34b0e29d7c6f105` (§8.4, the seal) | `sha256:a47c13281b0b4a13085e523cd3c09e4f7d1caf0c80419de5fa3a869f7aec309e` (`gate-report.md` §8.2, evaluation 2) | inside `Spine-Seal`, hence inside `Spine-Seal-Sig` |
| `tool=1.4.0+sha256:41d0e9b7c2a6538f10bd47e29c05a3f6b81d24e70c9a5b3f68d1027ae4c95b3d` — the same token on **all five** seals in §8–§11 | `sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db` (`manifest.md` §8.2, computed with `shasum -a 256` over that document's 529-byte artifact list) | inside `Spine-Seal`, hence inside `Spine-Seal-Sig` |

All three are 64 lowercase hex, so **no byte count in §8 moves when they are adopted** — `bytes=`, the 573-byte `freeze=` join and the 2379-byte `envelope=` join all stand, and §16 OPEN-3's 4031/4032 stand with them. What does move: the review line's bytes, therefore `Spine-Review-Sig`, therefore `envelope=`, therefore the seal line, therefore `Spine-Seal-Sig`; and the seal line independently, for the other two. **No private key is published here** (§8.1 says so in terms), so this is not an edit — it is a regeneration of §8.1's keyring and a re-signing of all five signatures, the same operation §14 D3 records being performed twice, cascading into `manifest.md` §8.3 and §8.7 exactly as it did then. **Until that regeneration happens the three values above are the divergence, and they are stated here rather than left to be rediscovered.** A reader verifying vector A against `gate-report.md` §8 should expect these three and no others; every other value the two documents share is byte-identical.

**Do not "fix" this by editing the three fields in place.** Editing them without re-signing produces a vector whose `ssh-keygen -Y verify` commands in §8.6 fail, which is strictly worse than a disclosed divergence: it converts a stated inconsistency into a broken published test.

---

## 16. OPEN — the owner's calls

**OPEN-1 · Closed by the owner, 2026-08-26: the subject is derived, and G9 checks it.** Version 1 put the question as **(a)** adopt §13.10's derivation and accept an unbound quick-lane subject, **(b)** fold the subject into `envelope=` by prepending it to the joined lines, or **(c)** add a `Spine-Subject` trailer carrying a digest of it — and recommended (a) now, (b) if it could be decided before the first implementation shipped. **The owner took (a).** The subject stays outside `envelope=`, so no digest definition changes and no existing signature moves, and PB §5.5 gives G9 the duty of recomputing it and refusing a landing whose subject it did not produce. §13.10 is the derivation for all five landing shapes, §12 carries the refusal code `subject-mismatch`, and §14 D12 files the two loose ends the playbook's own sentence still leaves. The residual the decision accepts, stated wherever it is relevant: **the quick lane's summary is free text, and every toolkit lifecycle landing rides the quick lane** — so the subject of an upgrade, a rollback or an uninstall says whatever its operator typed, while everything the landing actually attests sits in the sealed `Spine-Upgrade` line inside `envelope=`.

**OPEN-2 · The event commits' message shape.** Nothing reads the subject of a sign-off, approval, review, reopen, withdrawal or upgrade commit, and no digest covers one — `freeze=` reads only the manifest lines, G13 only the signed lines. Fixing a form (`<ID>: <event>`, say) removes one more way two implementations differ where nothing forces them to agree; leaving it free costs nothing measurable. Recommendation: fix `<ID>: <event>` for intent-branch events and `<event>` for the rest, in `intent-doc.md` or a CLI spec, not here. Owner-level only because it is somebody's to own and it is not this document's.

**OPEN-3 · Closed by the owner, 2026-08-26: the 16 KiB cap does not apply to a reseal.** Version 1 put the question as **(a)** exempt `Spine-Event: reseal` from the cap — one clause, admitting an unbounded commit message on exactly the landing nobody reviews line by line; **(b)** let a solo reseal carry two or more `class=protected` reviews whose **union** covers the wire set, as team mode already allows, meaning the one solo key signs twice; or **(c)** cap the orphan range one reseal may cover, resealing a long range in several steps, at the cost of a new bound to pick — and recommended (c) with (b) as the escape. **The owner took (a).** PB §5.5 carries it; §2.9 fixes what the exemption is measured by (`Spine-Event`, a sealed input), §12 excludes the shape from `envelope-too-large`, §18 item 24 states it, and §14 D11 is closed. Nothing about the capped quantity itself changed, so no vector in this document moves on that account: A and D remain 4031 and 4032 of 16384, and both are gated landings the cap still governs. (The later adoption of PB §11's wire order changed A's and D's `envelope=` and their signatures but not one byte count, so these two figures stand as published.) The residual the decision accepts, stated: a reseal's message has no size bound in this specification, and the only thing bounding it in practice is how far the pipeline was bypassed before the orphan was noticed.

---

## 17. Out of scope

Deliberately not specified here, and where it belongs instead:

- **The gate report** — its canonicalization, schema, digest, publication to `refs/notes/spine`, and what `--verify` can rebuild: `gate-report.md`. This document fixes only how `report=` is spelled in a trailer and which evaluation each occurrence names (§5).
- **The wire token and the `wires` array order** — `gate-report.md` §6.1–§6.2, adopted verbatim (§2.5). **`esc`** — `gate-report.md` §2.3. Neither is restated and neither may be re-derived.
- **The intent doc's grammar** — its sections, its AC ids, its touchpoint syntax, and what makes it parse: `intent-doc.md`. This document fixes only its canonical *form* as PB §3.3 already states it, and how its bytes are fenced (§2.6).
- **The freeze closure** — which files are frozen, how imports resolve, what an `expected` touchpoint excludes: PB §4.3 and `import-resolver.md`. This document serializes a closure someone else computed and never asks how.
- **The runner tokens, the `id → fn` functions, and the source-symbol → runner-id join** — `import-resolver.md`, for all **four** v1 languages: Python, TypeScript/JavaScript, Dart and Swift (Kotlin was dropped by the owner on 2026-08-26; `kotlin` and the `gradle` runner token stay reserved). The join is `import-resolver.md` §12.1–§12.3 and is no longer outstanding. §4.4 constrains what a `Spine-Test` id may *contain*; what it *is* is that document's, and every `Spine-Test` line in §8 and §10 is only as reproducible as that document is.
- **The result file** — `result-file.md`. No envelope field is derived from one; `profile=` on the seal is copied from a header the trusted stage ingested, and this document does not define ingestion.
- **Gate semantics.** What G2 containment means, when a gate reads `override`, how G14 casefolds, what makes a landing `unattested`. `Spine-Gates` is a rendering (§7 rule 12); which gates *ran* is `gate-report.md` §5.6.2's.
- **The keyring's format, rotation, revocation, and the trust root** — PB §7.2 and §7.5. §8.1 publishes a keyring only as a vector input.
- **The commit's author and committer fields, and its dates** — nothing covers them and nothing reads them (§6.3). A spec that pinned them would be pinning a fabricated fact.
- **The three CI definitions and `.spine/ci.sh`** — `ci.md`, including the required publish of the gate report.
- **Provider renderings.** A PR body, a merge-queue message, a `gh pr edit --body-file` payload: PB §5.4 configuration (b) describes them, and they are renderings of an envelope this document defines. The envelope is the commit message; a rendering is not the record.
- **`spine stats`, `spine context`, `spine review`** and every other reader. They read the graph, which `dump.md` serializes and PB §6.2 derives.

---

## 18. Conformance checklist

An implementation conforms iff all of the following hold. Every item is mechanically checkable against a produced landing commit.

**Structure**

1. The message has the regions of §2.1 in that order, with exactly one blank line between them, no trailing blank line, and exactly one final `0x0A`.
2. The message contains no `0x0D` and no `0x00`.
3. Every `Spine-*` line is well-formed per §2.3: a name from §2.4's closed set, `:`, one space, a non-empty payload.
4. The trailer block contains no blank line and no non-`Spine-*` line; any non-`Spine-*` line appears only after `Spine-Seal-Sig`.
5. There is exactly one `Spine-Seal` and exactly one `Spine-Seal-Sig`, in that order, adjacent, and no `Spine-*` line follows them.
6. Trailer ranks are non-decreasing per §2.4; each `-Sig` immediately follows the line it signs.
7. Repeated `Spine-Reopen` and `Spine-Review` lines are in `gate-report.md` §5.5.1's ancestor-first order; repeated `Spine-Frozen` and `Spine-Test` lines are in §4.2's sort order.
8. Every payload's fields are in PB §11's order, single-spaced, with no leading or trailing space.

**The fence**

9. Present iff the landing is gated or a tombstone; `git hash-object` over exactly `bytes=` bytes after the BEGIN line's terminator reproduces `blob=`; the END line begins immediately after them.
10. The subject is the line §13.10 derives for the landing's shape — gated and tombstone: the fenced block's first line with `# ` removed; reseal: `reseal: ` followed by the seal's `head=` in full; quick and lifecycle: `quick: ` followed by a non-empty free-text summary. A difference is `subject-mismatch`. Nothing hashes the subject; G9 recomputes it.

**Digests**

11. `envelope=` equals §3.1's digest of the `Spine-*` lines above the `Spine-Seal` line — `-Sig` lines included, fence and subject excluded, joined by `0x0A` with **no trailing** `0x0A`.
12. `freeze=` on the copied `Spine-Approve` line equals §4.1's digest, recomputed from the approval commit under merge and from the envelope's own copied lines under squash.
13. `report=` is `sha256:` plus 64 lowercase hex; the review's and the seal's values are the digests of two different evaluations (§5).
14. Every oid is lowercase hex at the repository's full `object_format` length; every non-git digest is `sha256:` + 64 lowercase hex; every counter is plain decimal with no leading zero.

**Signatures**

15. Each `-Sig` payload is a single unbroken base64 run with no armor and no `0x0A`.
16. Re-armoring at 70 characters per line and running `ssh-keygen -Y verify` against the keyring **at the seal's `base=`** succeeds for every signed statement, under the namespace PB §7.2 assigns its role.
17. The signed byte range is the whole line, trailer name included, terminator excluded (§13.8, `dump.md` §5.2.1).

**Encodings**

18. A `Spine-Frozen` path is C-quoted exactly per §4.3 whenever it contains a byte in `0x00–0x1F`, `0x7F–0xFF`, `"` or `\`, and literal otherwise — independent of `core.quotePath`.
19. A wire token inside `wires=` is `gate-report.md` §6.2's `tok`, which is not §4.3's quoting (§13.9).
20. No path, principal, id or reason is normalized, casefolded or separator-rewritten anywhere.

**Construction**

21. The commit is built with `git commit-tree`, never `git commit` or `git merge` (§6.1, §6.2); the message is read back with `git cat-file commit`, never `git log`.
22. The first parent is the seal's `base=`; the tree is the seal's `tree=`.
23. No trailer payload holds a time, a duration or a date.

**Size**

24. On every landing shape except a reseal, the capped quantity of §2.9 — the message length minus every `Spine-Frozen` and `Spine-Test` line and its terminator — is at most 16384, and exceeding it is `envelope-too-large`, never a truncation. A landing whose `Spine-Event` is `reseal` is not measured against the cap and can never raise that code (§2.9).

**Vectors**

25. Vector C (§10) reproduces, then vector A's `freeze=` (§8.2), then vector A's `envelope=` (§8.3), then vector B (§9) and vector D (§11). Debug in that order: C isolates the sort, A's freeze adds the quoting, A's envelope adds the selection and the join, and B and D add the lane and strategy variations.
