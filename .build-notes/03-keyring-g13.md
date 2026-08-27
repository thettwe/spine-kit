# `.spine/allowed_signers` — line grammar, three namespaces, and G13 (Signers) in full

Concern sheet 03. Written so a Rust implementation of the keyring parser, the keyring lint and gate **G13 — Signers** can be built from this sheet alone.

Citation convention: `(MF §x.y)` = `docs/spec/manifest.md`, `(PB §x.y)` = `PLAYBOOK.md`, `(GR …)` = `docs/spec/gate-report.md`, `(DM …)` = `docs/spec/dump.md`, `(EV …)` = `docs/spec/envelope-vectors.md`, `(CN …)` = `docs/spec/constitution.md`, `(README)` = `docs/spec/README.md`.

Precedence rule in force (stated by MF §1 itself): *"Where this document and PB §11 disagree, §11 wins and the disagreement is a defect in one of them — reported in §10, never resolved silently."* Everywhere else the spec resolves PB's ambiguity and is normative.

---

## Sources read

| File | Lines | Sections, read in full |
|---|---|---|
| `docs/spec/manifest.md` | 1–35 | §1 *What these artifacts are, and what rests on them* (the three governing facts, the four design constraints, in/out of scope) |
| `docs/spec/manifest.md` | 186–242 | §3.5 `files[]` and the three ownership classes; §3.6 `templates`/`resign` (the twelve template keys, incl. `keyring`) |
| `docs/spec/manifest.md` | 316–330 | §4.1 *The format, and why it is not spine's* |
| `docs/spec/manifest.md` | 324–350 | §4.2 *The line grammar* (verbatim ABNF-ish block + four bullets + the fingerprint paragraph) |
| `docs/spec/manifest.md` | 351–360 | §4.3 *The three namespaces* (the role table, closed domain) |
| `docs/spec/manifest.md` | 361–385 | §4.4 *What makes a keyring malformed — the closed list* (18-row table) |
| `docs/spec/manifest.md` | 386–408 | §4.5 *The rules G13 relies on, and where each is evaluated* (mode, one-key-two-principals, two-keys-one-principal, seal-mixed, no-seal, which gate reports which) |
| `docs/spec/manifest.md` | 409–421 | §4.6 *What is derived from it*; §4.7 *What the keyring is not* |
| `docs/spec/manifest.md` | 422–442 | §4.8 / §4.8.1 *The shape of the gate* |
| `docs/spec/manifest.md` | 443–469 | §4.8.2 *The two evaluation situations, and the inputs each fixes* |
| `docs/spec/manifest.md` | 470–487 | §4.8.3 *The required namespace of every signed line* |
| `docs/spec/manifest.md` | 488–531 | §4.8.4 *The checks, in order* (13 rows + the seven "On check n" paragraphs) |
| `docs/spec/manifest.md` | 532–545 | §4.8.4.1 *The chain rule, as a check* |
| `docs/spec/manifest.md` | 546–562 | §4.8.5 *What G13 does not check*; §4.8.6 *The verdict* |
| `docs/spec/manifest.md` | 563–617 | §4.8.7 *G13, in one place* (the pseudocode block, verbatim) |
| `docs/spec/manifest.md` | 838–905 | §6.1 *The shape of G16*; §6.2 *The checks, in order* (check 13 = keyring lint over `K_T`) |
| `docs/spec/manifest.md` | 1010–1040 | §6.8 `to=none`; §6.9 `from=none` (both carry keyring clauses) |
| `docs/spec/manifest.md` | 1048–1067 | §7 *Determinism rules, collected* |
| `docs/spec/manifest.md` | 1083–1102, 1153–1200, 1240–1262 | §8.1 blobs, §8.3 manifest bytes (the `.spine/allowed_signers` record), §8.5 G16 run (check 13) |
| `docs/spec/manifest.md` | 1320–1366 | §8.7 *The keyring, and reproducing everything* (published keyring, fingerprints, the walked lint) |
| `docs/spec/manifest.md` | 1367–1426 | §9 R6, R7, R24, R25, R26, R27, R28 |
| `docs/spec/manifest.md` | 1427–1460 | §10 D11 (two keys, one principal) and the surrounding defect list |
| `docs/spec/manifest.md` | 1461–1503 | §11 C4, C12, C13; §12 out of scope |
| `docs/spec/manifest.md` | 1504–1514 | §13 OPEN-2 (canonical form for the keyring) |
| `PLAYBOOK.md` | 805–834 | §7.2 *Authority: keys, roles, and what a signature binds* (in full) |
| `PLAYBOOK.md` | 835–847 | §7.3 *The protected floor* (`.spine/**` is floor) |
| `PLAYBOOK.md` | 882–891 | §7.5 *Trust root, rotation, revocation* (in full: bootstrap, chain rule, two clocks, recovery landing, rotation) |
| `PLAYBOOK.md` | 892–897 | §7.6 *Break-glass, not backdoors* (the bypass list) |
| `PLAYBOOK.md` | 679 | §6.3 the G13 — Signers row (verbatim, one sentence) |
| `PLAYBOOK.md` | 513–550 | §6 transition table rows citing G13 (sign-off, reopen, key-removal ×2, approval, tripwire/protected review, break-glass, the two quick-lane rows) |
| `PLAYBOOK.md` | 416, 468 | §5.4 step 2 (the tombstone's four gates), §5.5's `Spine-Gates` line |
| `PLAYBOOK.md` | 983–1039 | §11 *Vocabulary* — Roles and namespaces, Signed statement, the trailer table, Files and refs, Wire aggregation, the signerless overlay, Break-glass overlay, Gates, CLI (`init` flags), Git requirements |
| `PLAYBOOK.md` | 747 | §6.7 `--pipeline-key` / team-mode strip / `keyring-no-seal` |
| `docs/spec/gate-report.md` | 392–434 | §5.5 `authority` + §5.5.1 ordering |
| `docs/spec/gate-report.md` | 450–496 | §5.6.1 the status domain and the outright table (G13 row) |
| `docs/spec/gate-report.md` | 160–181 | §4.3 `--verify` exit codes |
| `docs/spec/gate-report.md` | 630–717 | §6.1 wires, §6.2 the token, §6.3 the per-gate wire table (G13 row) |
| `docs/spec/envelope-vectors.md` | 468–485 | §8.1 *The keyring and the keys* (the byte-identical twin of MF §8.7) |
| `docs/spec/dump.md` | 222, 387–390, 596, 627–629, 684 | `signer` node id, its four attrs, the three published signer nodes |
| `docs/spec/constitution.md` | 1307, 1388, 1414, 1425 | §14.11 half-resolution, §15 D15, §16 OPEN-9, §17 out-of-scope |
| `docs/spec/README.md` | 15–88 | the index rows for `manifest.md`, the published-digest table, the cross-doc open list |

---

## Data model

### 1. The file

| Field | Type | Domain | Default | Required |
|---|---|---|---|---|
| path | fixed | `.spine/allowed_signers` | — | yes — its absence is `keyring-missing` (MF §4.4) |
| ownership class | fixed | `user-owned` (MF §3.5, PB §6.7) | — | yes |
| `files[]` template | string | `keyring@1` in v1 (MF §3.6, §8.3) | — | yes |
| canonical byte form | **none** | — | — | **no** (MF §4.1) |
| encoding | bytes | lines terminated `0x0A`; final terminator optional | — | — |
| max size | not fixed anywhere in the corpus | — | — | — |

**R1 (MUST NOT).** The keyring **MUST NOT** be required to have a canonical byte form. MF §4.1, verbatim: *"**The keyring has no canonical byte form.** It is `user-owned` (PB §6.7): humans edit it under a protected PR, `spine init --pipeline-key` appends to it, and requiring canonical bytes would make re-indenting a gate failure."* Contrast the manifest, which MUST be canonical (MF §2.4, §6.2 check 3).

**R2 (MUST).** The grammar of §4.2 **MUST** be implemented as a *lint*, not as the parser verification depends on. MF §4.1, verbatim: *"the grammar below is a lint, not a parser spine's verification depends on."* Verification is `ssh-keygen -Y verify` and *"OpenSSH decides"* (MF §4.1, MF §12 *"Signature verification. OpenSSH's."*).

**R3 (MUST).** Both the keyring and the manifest **MUST** be read from trunk, never from the candidate (MF §1, PB §7.4 rule 1) — *"the keyring a signature verifies against is trunk's, never the branch's, so a branch cannot enrol the key that authorizes it (§4.8.2)"*.

**R4 (fact).** `.spine/**` is on the protected floor (PB §7.3), so any landing touching the keyring already raises a `class=protected` `G14:<path>` wire before G13 or G16 speaks (MF §1).

### 2. The entry (one parsed line)

| Field | Type | Domain | Default | Required |
|---|---|---|---|---|
| `line_no` | integer ≥ 1 | 1-based index in the file | — | yes (DM provenance `git:<sha>:.spine/allowed_signers:<line>`, DM §12 vector) |
| `principal` | byte string | `1*( %x21-7E except "," and "#" and WS )` | — | yes, exactly one (R8) |
| `namespaces` | set of tokens | subset of `{spine-signoff@v1, spine-review@v1, spine-seal@v1}`, non-empty | — | yes (`keyring-no-namespaces`, `keyring-namespace-empty`) |
| `keytype` | token | the eight of R6 | — | yes |
| `keyblob` | base64 | `1*( ALPHA / DIGIT / "+" / "/" / "=" )`, decoding to a key of the declared type | — | yes |
| `comment_text` | byte string | any, after `WS+` following the blob | absent | no — *"accepted and ignored"* (MF §4.2) |
| `fingerprint` | string | `"SHA256:"` + unpadded base64 | — | derived (R11) |

### 3. Derived per-keyring values

| Name | Type | Definition | Citation |
|---|---|---|---|
| `mode` | `"solo"` \| `"team"` | `"solo"` iff exactly one distinct **fingerprint** is listed under `spine-signoff@v1`; `"team"` otherwise | MF §4.5, PB §11 |
| `principals(K)` | set | the principals of all entries | MF §4.8.7 |
| `roles(principal)` | array of strings | the entry's namespaces, **ascending by bytes** | MF §4.6, DM §7.2 |
| `valid_from` | git oid | the trunk commit at which this `(principal, key)` **first appears** | MF §4.6 |
| `valid_to` | git oid, absent if still present | the trunk commit at which it **stops appearing** | MF §4.6, DM §7.2 |
| delta between two keyrings | set difference over `(principal, fingerprint)` pairs | MF §4.8.4.1 | |

**R5 (MUST).** `valid_from` and `valid_to` **MUST** be commits, not times. MF §4.6, verbatim: *"**Both are commits, not times** — the chain is the clock."* PB §7.5: *"the chain, not timestamps, is the authority"*, *"One clock, no timestamps."*

**R6 (MUST).** A line edited in place (same principal, new key) **MUST** be treated as a removal plus an addition — the old fingerprint gets a `valid_to`, the new one a `valid_from` (MF §4.6). `keyring-duplicate-principal` guarantees the two never coexist (MF §4.6).

**R7 (MUST NOT).** The keyring **MUST NOT** be treated as a source of identity beyond the file. MF §4.7, verbatim: *"no `~/.ssh/allowed_signers`, no `gpg.ssh.allowedSignersFile` from git config, no `cert-authority`, no CA, no external directory. It is not versioned: there is no `Keyring: v<n>` line, and `templates.keyring` names the seed's template, never the file's content. It carries no policy: `C-A1`, `C-A2` and the rest live in the constitution (PB §7.2), and a keyring is key material."*

### 4. G13's inputs (and nothing else)

MF §4.8.2, **verbatim**:

```
K       := .spine/allowed_signers at B (in-flight) or at the seal's base= (landed)
mode    := §4.5's key count over K            # "solo" iff exactly one spine-signoff@v1 fingerprint
E       := the branch's event commits, ancestor-first along
           git rev-list --reverse --first-parent B..H, extended past Hc to H   (GR §5.5.1)
A       := the bound statements — GR §5.5's authority object:
           signoff, approve, reopens[], reviews[], upgrade, withdraw
oidlen  := 40 if object_format = "sha1" else 64                                (§3.1)
```

followed by, verbatim: *"Nothing else. No wall clock, no environment, no prior run, no side file (§7)."*

`A`'s statement shape (GR §5.5): each statement is `{ line: string (esc-encoded, terminator excluded, **not normalized**), fingerprint: "SHA256:"+unpadded base64, namespace: one of the three }`. A **review** adds one computed member `self_approved: boolean`, `true` iff the review's `fingerprint` equals the landing's signer key. **The landing's signer key** is `authority.signoff.fingerprint` when present, else `authority.upgrade.fingerprint` when present, else **none** (GR §5.5).

**R8 (MUST).** G13 **MUST** supply the fingerprints from which `self_approved` is computed, and that fingerprint **MUST** be the one this gate's verification recorded, never the principal (MF §4.8.1, GR §5.5, PB §7.2 *"reviewer ≠ signer compares fingerprints"*).

---

## Algorithm

### Part A — parse and lint the keyring

**R9 (MUST).** Split the file into lines terminated by `0x0A`. A final line without a terminator **MUST** be accepted and is **not** an error (MF §4.2: *"a final line without a terminator is accepted (OpenSSH accepts it) and is not an error"*).

**R10 (REFUSE).** Any `0x0D` anywhere in the file is `keyring-cr` (MF §4.2, §4.4). Reason recorded: *"`.gitattributes` pins `eol=lf` on `.spine/**` (ID §2.5); a CR forks the blob G16 compares."*

**R11 (MUST).** Classify each line against the grammar, **verbatim** (MF §4.2):

```
line        := blank | comment | entry
blank       := WS*
comment     := WS* "#" any*
entry       := WS* principals WS+ options WS+ keytype WS+ keyblob [ WS+ comment-text ]
principals  := principal [ "," principal ]*
principal   := 1*( %x21-7E except "," and "#" and WS )
options     := "namespaces=" DQUOTE namespace [ "," namespace ]* DQUOTE
namespace   := "spine-signoff@v1" | "spine-review@v1" | "spine-seal@v1"
keytype     := "ssh-ed25519" | "ecdsa-sha2-nistp256" | "ecdsa-sha2-nistp384"
             | "ecdsa-sha2-nistp521" | "sk-ssh-ed25519@openssh.com"
             | "sk-ecdsa-sha2-nistp256@openssh.com" | "rsa-sha2-256" | "rsa-sha2-512"
keyblob     := 1*( ALPHA / DIGIT / "+" / "/" / "=" )
WS          := %x20 / %x09
```

**Implementation note (mandatory, derived).** The strict grammar above admits *only* conforming lines, yet §4.4 requires distinct statuses for lines that violate it in specific ways (`keyring-no-namespaces`, `keyring-option-unknown`, `keyring-validity-option`, `keyring-cert-authority`, `keyring-namespace-unknown`, `keyring-namespace-empty`, `keyring-keytype-unknown`, `keyring-key-not-base64`, `keyring-multi-principal`). A conforming implementation therefore **MUST** parse permissively — field-split into `principals`, an OpenSSH option list, `keytype`, `keyblob`, `comment-text` — and then classify, reserving `keyring-line-malformed` for a line that cannot even be split into those fields. This is stated here because MF §4.2/§4.4 leave it implicit; see *Contradictions* C1.

**R12 (REFUSE).** One entry, one principal. MF §4.2, verbatim: *"**One entry, one principal.** `principals` admits OpenSSH's comma list; the lint refuses a line with more than one (`keyring-multi-principal`)."* Rationale given: *"A comma list makes one key reach several identities on one line, which is the same hazard as §4.5's 'one key under two principals' wearing different syntax, and no spine workflow writes one."*

**R13 (REFUSE).** `namespaces=` is the only option accepted. MF §4.2: *"OpenSSH also defines `cert-authority`, `valid-after=` and `valid-before=`. All three are refused (§4.4)."* Statuses: `keyring-cert-authority`, `keyring-validity-option` (both `valid-after=` and `valid-before=`), `keyring-option-unknown` (anything else).

**R14 (MUST).** The key is the two fields `ssh-keygen -lf` reads: `<keytype> <keyblob>` (MF §4.2). **`ssh-rsa` (SHA-1) is not in the keytype list** — MF §4.2: *"OpenSSH ≥ 8.2 is a stated requirement (PB §11) and SHA-1 RSA signatures are the one thing that release deprecated."* An out-of-list keytype is `keyring-keytype-unknown`.

**R15 (MUST).** A trailing comment-text after the key blob **MUST** be accepted and ignored — MF §4.2: *"it is where `ssh-keygen` puts a key's own comment, and humans put names there."*

**R16 (MUST).** Compute the fingerprint. MF §4.2, verbatim: *"**The fingerprint** of an entry is `ssh-keygen -lf` over `<keytype> <keyblob>`: `\"SHA256:\"` plus unpadded base64. That is what `reviewer ≠ signer` compares (PB §7.2, GR §5.5), never the principal."*

**R17 (REFUSE).** A key blob that is not base64, **or that does not decode to a key of the declared type**, is `keyring-key-not-base64` (MF §4.4). Note the second limb: the check is a decode-and-typecheck, not a charset test.

**R18 (MUST).** Apply the closed malformed list of MF §4.4 in full (see *Error cases* below). MF §4.4 is **closed**: no other condition makes a keyring malformed.

**R19 (MUST).** Compute `mode` **from the key count, never from `C-A1`**. MF §4.5, verbatim:

```
mode := "solo"  if |{ fingerprint : entry lists spine-signoff@v1 }| = 1
        "team"  otherwise
```

MF §4.5: *"A `C-A1` disagreeing with that is a warning, not a finding, and not an input to any check."* PB §11 (which wins): *"Solo mode = exactly one signoff key (`C-A1`), whose principal then holds all three namespaces."*

**R20 (MUST).** `keyring-seal-mixed` **MUST** be evaluated only when `mode = "team"`. MF §4.5, verbatim: *"In **solo** mode the rule is inverted by definition: the one principal holds all three namespaces (PB §11, *Roles and namespaces*, *"Solo mode = exactly one signoff key"*), so `keyring-seal-mixed` is evaluated only when `mode = \"team\"`."* Same scoping for `keyring-no-seal` (its §4.4 row reads *"in **team** mode, no principal holding `spine-seal@v1`"*).

**R21 (REFUSE).** In team mode the seal principal holds the seal namespace **and nothing else — in either direction** (PB §6.3 G13, MF §4.5): a human key also under `spine-seal@v1` is refused, and the seal key holding any other namespace is refused. Status `keyring-seal-mixed`.

**R22 (REFUSE).** One key (by fingerprint) under two principals: `keyring-key-two-principals`. MF §4.5: *"The hazard is `reviewer ≠ signer`, which compares fingerprints — one key wearing two names would satisfy it under one name and fail under the other, and which one a verifier saw would depend on the order `ssh-keygen` matched."* PB §7.2 and PB §6.3 both state the refusal.

**R23 (REFUSE).** Two keys under one principal: `keyring-duplicate-principal`. This is **new in MF** (PB does not say it) and is forced by DM §5.2: *"A `signer` node's id is `signer:` + `esc(principal)`, so two keys under `alice@example.com` are two signer nodes with one id, with different `fingerprint` attrs — an unrepresentable graph, and G10 diffs node ids before every landing."* Remedy: enrol a second principal (`alice+yubikey@example.com`) — *"which costs one line and is what `--signer-key` already produces"* (MF §4.5, §9 R7, §10 D11, §11 C4).

**R24 (REFUSE).** Team mode requires a seal principal: `keyring-no-seal`. PB §6.7, verbatim: *"G13 refuses a team-mode keyring with no `spine-seal@v1` principal"*. MF §4.5: *"A team-mode keyring with none has nobody who can seal, and every landing would be a recovery landing."*

**R25 (MUST).** Which gate reports which (MF §4.5, the *Which gate reports which* paragraph):
- **G16** raises §4.4's lint findings as `class=protected` `G16` wires over **`K_T`** (the keyring in the candidate tree `T`) — MF §6.2 check 13, kind **coverable**.
- **G13** raises them **outright** over **`K_B`** (the keyring at the base `B`) — MF §4.8.4 check 1.
- The overlap is deliberate: *"a malformed keyring at `B` means no signature verifies, so G13 fails first and G16's finding is redundant. … The asymmetry in *kind* is deliberate and follows from the commit each reads: the keyring at `B` is already trunk's, so a review cannot make it verify, while the keyring in `T` is what the landing is proposing and is exactly what a protected review is for."*

### Part B — the three namespaces

MF §4.3 / PB §7.2 — the domain is **closed**:

| Role | Namespace | Signs | Held by |
|---|---|---|---|
| signer | `spine-signoff@v1` | sign-off, reopen, withdraw, toolkit upgrade events | humans |
| reviewer | `spine-review@v1` | reviews (tripwire, protected, break-glass); approvals in v1; the seal of a recovery landing | humans |
| pipeline | `spine-seal@v1` | the seal; approvals carrying `run=` | the trusted stage; in solo mode, the human's own key |

PB §7.2's *Held by* column adds detail MF §4.3 compresses: the pipeline key is *"the trusted stage — a CI secret no laptop holds; in solo mode, the human's own key"*, and it signs approvals carrying `run=` *"once B runs in the trusted stage"*.

**R26 (REFUSE).** An unknown namespace token is `keyring-namespace-unknown` — **never ignored**. MF §4.3, verbatim: *"not ignored, because an ignored token is a role nobody can audit and a typo (`spine-signof@v1`) silently removes a signer's authority while leaving the line looking correct."*

**R27 (REFUSE).** `namespaces=""` is `keyring-namespace-empty` — *"a key with no role"* (MF §4.4).

**R28 (REFUSE).** An entry with no `namespaces=` option at all is `keyring-no-namespaces`. MF §4.4's reason, verbatim: *"a line without it matches **every** namespace, so one key would hold all three roles by omission"*.

**R29 (MUST).** `roles` as exported to DM's `signer` node is the entry's namespaces **ascending by bytes** (MF §4.6, DM §7.2) — e.g. `["spine-review@v1","spine-signoff@v1"]` (DM §12 vector, line 627).

### Part C — G13: shape

**R30 (MUST).** G13 runs on **all four landing shapes** — gated, quick/lifecycle, reseal and tombstone (MF §4.8.1, GR §5.6.2's table, which marks G13 ✓✓✓✓). A tombstone's four gates are **G9, G13, G14, G15** (PB §5.4 step 2, PB §11, PB §5.5's `Spine-Gates`). MF §4.8.1: *"a landing nobody may sign is not a landing, whatever it does to the tree."*

**R31 (MUST).** G13 has **no** not-run row. MF §4.8.1: GR §5 marks both `authority` and `self_approved` *"always present"*, so *"a shape on which G13 did not run would owe a required member computed from fingerprints nothing produced."*

**R32 (MUST).** Checks are **ordered**. Checks **1 and 2 are a halting prefix**: *"checks 1 and 2 are a prefix that halts on an outright failure, because a keyring that does not lint is not a set anything verifies against, and a line whose signature did not verify is not a line whose fields may be read. From check 3 onward every check runs and findings accumulate"* (MF §4.8.1) — the accumulation reason being *"a reviewer signing a protected review needs the whole list, not the first item"* (MF §6.1, adopted by §4.8.1).

**R33 (MUST).** Two finding kinds only (MF §4.8.1, mirroring §6.1):
- **outright** — *"G13 reads `fail` whatever any review names. The landing does not seal, and a recovery-sealed one also indexes `unattested` (PB §7.5)."*
- **coverable** — *"a `class=protected` wire, dischargeable by a protected review whose `wires=` contains the token."* **G13 has exactly one coverable check** (check 2, restricted branch).

**R34 (MUST).** G13's wires are `class=protected`, **always** (MF §4.8.1, GR §6.3): *"Authority never warns, is never on PB §7.6's bypass list, and judges the machinery that judges the landing"*.

**R35 (MUST).** G13's wire **names a commit, not a path**. MF §4.8.1, verbatim: *"`path` carries the offending event commit's object id, lowercase hex at the length `object_format` implies, for which both `esc` and `tok` are the identity — so the wire token is `G13:` + that oid, and it is the one non-path value v1 puts in that member. One wire per commit, deduplicated under GR §6.1's `(gate, path)` rule and sorted with the rest of the array by GR §6.1's ordering."*

**R36 (MUST NOT).** Break-glass **cannot** bypass G13. MF §4.8.1: *"PB §7.6's list is G1, G2, G3, G4, G6, G7, G8 and G12; Authority is never in it, and PB §6's break-glass row states it twice — *'never Authority'*. A `class=break-glass` review is itself a statement G13 verifies, and check 7 holds over it."*

### Part D — the two evaluation situations

MF §4.8.2's table, reproduced:

| | **in-flight** | **landed** |
|---|---|---|
| Raised by | `spine check`, `--sign`, `--approve`, `--review`, and `--land` before a seal exists | `spine index`'s first-parent walk, `spine check --authority`, G9's ledger walk |
| Governing keyring `K` | `.spine/allowed_signers` at trunk's **current** tip | `.spine/allowed_signers` at the **seal's `base=`** — `L`'s first parent, or for a reseal the last valid landing below its range |
| A statement whose fingerprint is absent from `K` | **void**, not a finding | history stays valid; `spine check --authority` lists the landing |
| Can see the seal | no — the seal signs `envelope=`, which covers the report this gate feeds | yes |

**R37 (MUST).** `spine check --land` evaluates in the **in-flight** situation, and its `B` is the trunk tip the landing is landing onto — *"the value that becomes the seal's `base=`. The two keyrings are therefore the same bytes for a landing that lands; they diverge only afterwards, when the keyring moves."* (MF §4.8.2)

**R38 (MUST).** **Voiding is a transition, not a finding.** MF §4.8.2, verbatim: *"PB §6's three key-removal rows — a signer's key removed returns the intent to `awaiting-sign-off`; an approver's or a reviewer's key removed voids the approval or the review — all name G13 as the gate, and not one of them is a wire. G13 supplies the predicate (*this statement's fingerprint is not in `K`*) and PB §6's transition table consumes it. A void statement is **absent** from GR §5.5's `authority` object … Two clocks is what makes that safe: revoking a key un-approves in-flight work and leaves landed work alone."*

**R39 (MUST).** Check 2 **MUST** skip a void statement, in **both** situations, and deciding voidness **MUST NOT** require an SSHSIG parse. MF §4.8.4: *"the principal is the line's own `signer=` or `reviewer=` value, and §4.4's `keyring-duplicate-principal` makes one principal one key. Without this, rotating a signer's key mid-flight would turn an append-only branch's own sign-off into an outright refusal — the brick PB §6.2 rules out in terms."*

### Part E — the required namespace of every signed line

MF §4.8.3's table — *"the whole of PB §6.2's 'whose role disagrees with its namespace'"*:

| Signed line | Required namespace |
|---|---|
| `Spine-Signoff` | `spine-signoff@v1` |
| `Spine-Reopen` | `spine-signoff@v1` |
| `Spine-Upgrade` | `spine-signoff@v1` |
| `Spine-Withdraw` | `spine-signoff@v1` **or** `spine-review@v1` — check 8 decides which, by key |
| `Spine-Approve` carrying `run=` | `spine-seal@v1`, and only that (PB §11) |
| `Spine-Approve` not carrying `run=` | `spine-review@v1`, and only that (PB §11) |
| `Spine-Review`, any `class=` | `spine-review@v1` |
| `Spine-Seal` on a `mode=solo` or `mode=team` landing | `spine-seal@v1` |
| `Spine-Seal` on a `mode=recovery` landing | `spine-review@v1` (PB §7.5) |

**R40 (MUST).** The signature is over the trailer line's **exact bytes, terminator excluded** (PB §7.2), produced by `ssh-keygen -Y sign -n <namespace>`; verification is `ssh-keygen -Y verify -f K -I <principal> -n <namespace> -s <sig>` and **OpenSSH decides** (MF §4.8.3, §4.1). *"The principal is the line's own `signer=` or `reviewer=` value."*

**R41 (MUST).** In **team** mode the keyring already guarantees the `spine-seal@v1` and human sets are disjoint (`keyring-seal-mixed`); in **solo** mode one key holds all three namespaces, *"and this table is the only thing separating the roles, which is why it is stated per trailer rather than per key"* (MF §4.8.3).

**R42 (REFUSE).** `Spine-Approve`'s two rows are exclusive in both directions — which is what makes PB §6.3's *"a review or approval without `run=` signed by a `spine-seal@v1` key"* **a refusal rather than a preference** (MF §4.8.3). Status: `statement-namespace`.

### Part F — the checks, in order (MF §4.8.4)

| # | Check | Kind | Status |
|---|---|---|---|
| 1 | `K` is present and passes §4.4's lint, its five mode-dependent clauses evaluated at §4.5's key count | outright | the `keyring-*` tokens of §4.4 |
| 2 | every event commit in `E` carries a signed line that verifies against `K` under §4.8.3's namespace for its trailer | **outright** if the line's trailer is `Spine-Signoff`, `Spine-Approve`, `Spine-Review`, `Spine-Upgrade` or `Spine-Withdraw`; **coverable** otherwise | `statement-unverified`, `statement-namespace` |
| 3 | no two commits in `E` carry byte-identical signed lines | outright | `event-line-duplicate` |
| 4 | `A.approve`, when present, is the newest verifying `Spine-Approve` in `E`; is later in `E` than the last `Spine-Reopen`; carries an `intent=` equal to the intent blob under evaluation; and carries a `freeze=` no `Spine-Reopen` in `E` names | outright | `approval-voided` |
| 5 | every `Spine-Reopen` in `E` carries `voids=` naming the `freeze=` of the approval binding immediately before it, and `voids=none` exactly when no approval preceded it | outright | `reopen-voids-mismatch` |
| 6 | `A.approve`'s `reason=` is present whenever its `red=` reads `0/n` or it carries `held=false` | outright | `approve-reason-missing` |
| 7 | in **team** mode, no `class=protected` and no `class=break-glass` review in `A.reviews` has `self_approved: true` | outright | `self-approved-protected` |
| 8 | `A.withdraw`, when present, verifies under `spine-signoff@v1` by the fingerprint on `A.signoff`, or under `spine-review@v1` by a fingerprint ≠ it | outright | `withdraw-key` |
| 9 | the signerless overlay: when the landing has no signer key, `A.reviews` holds **two** `class=protected` reviews with distinct fingerprints in team mode, and **one** in solo mode | outright | `signerless-review-count` |
| 10 | the chain rule, when `diff(B, Hc)` touches `.spine/allowed_signers` (§4.8.4.1) | outright | `chain-review-not-in-parent`, `chain-remover-removed`, `chain-seal-not-in-parent` |
| 11 | *(in-flight only)* `A.approve`'s `total_rounds=` equals its own `rounds=` plus the `rounds=` of every earlier verifying `Spine-Approve` in `E` | outright | `total-rounds-mismatch` |
| 12 | *(in-flight only, at `--approve`)* the branch carries no verifying `Spine-Approve` later than the last `Spine-Reopen` with the same `intent=`, unless that approval's fingerprint has since left `K` | outright | `approval-redundant` |
| 13 | *(in-flight only, at `--approve`)* `reason=` is present when the closure tripwire fired | outright | `approve-reason-missing` |

Normative notes attached to the checks:

**R43 (check 1).** Both readings are wanted — G13 at `B`, G16 at `T`: *"a landing may be refused by G13 for the keyring it is landing *onto* and by G16 for the keyring it is landing. … a keyring that verifies today and is malformed for the next landing must not pass."* (MF §4.8.4)

**R44 (check 2 — the split, MUST).** The split is by **the role the trailer claims** — *"read from the line's own name, on the commit, whatever the verification did with it."* Five trailers are **outright**: `Spine-Signoff`, `Spine-Approve`, `Spine-Review`, `Spine-Upgrade`, `Spine-Withdraw`. *"Any other line on the branch — `Spine-Reopen`, and anything hand-made that merely looks like a trailer — is noise a human may accept. **Coverable.**"* (MF §4.8.4, §9 R24)

**R45 (check 2 — MUST NOT).** The split **MUST NOT** be made over `A`. MF §4.8.4, verbatim: *"GR §5.5 records no unverified statement, so a line whose signature failed supplies no member of `A` by construction; splitting over `A` would route every *forged binding sign-off* to the coverable branch, where a protected review discharges it, and G13 would seal a landing over a signature nobody made."*

**R46 (check 2 — the two statuses).** *"`statement-unverified` and `statement-namespace` are two statuses for one wire because they are two different repairs: the first says the bytes and the signature disagree, the second says the key holds a role the trailer does not admit — a `spine-review@v1` key signing a `Spine-Signoff`, or the case PB §6.3 names outright, a review or a `run=`-less approval signed by a `spine-seal@v1` key."* (MF §4.8.4)

**R47 (check 2 — the exclusion is unconditional).** *"The exclusion from state derivation is not something the review grants — it is a fact the gate records either way, and DM's graph never sees the commit."* (MF §4.8.4)

**R48 (check 3 — MUST be outright).** *"two siblings rest a **total order** on it: GR §5.5.1 orders `reopens` and `reviews` ancestor-first and calls the order total *'because G13 refuses that'*, and DM §5.2 keys an approval node on the signed line's hash and cites this refusal for uniqueness."* (MF §4.8.4)

**R49 (check 4 — scope).** Check 4 is *"one check on **the approval bound to this evaluation**, not a check over every approve line the branch carries"* — because *"a reopened intent keeps its voided approvals on an append-only branch for ever, and reading the refusal over all of them would make a reopen a permanent refusal — the exact opposite of PB §4.3's 'Reopens are never refused. They are never silent.'"* (MF §4.8.4)

**R50 (check 6 — the third limb lives in check 13).** PB §11 makes `reason=` mandatory on three conditions — *"`red=0/n`, `held=false`, or a closure tripwire"*. The two line-readable limbs are checked on every evaluation; *"the tripwire limb is check 13, evaluated where the closure is in hand, and a landing checks what the line carries."* (MF §4.8.4, §9 R25). A landing **MUST NOT** recompute the closure for this purpose.

**R51 (check 7 — solo is exempt).** *"In **solo** mode both are self-signed by definition, recorded in `self_approved` and counted by `spine stats`, never refused (PB §7.2's table)."* (MF §4.8.4)

**R52 (check 9 — the solo limb is load-bearing).** *"solo mode has exactly one signoff key by definition, so requiring two would make a quick landing, a reseal and a keyring change unlandable in every solo repository — the contradiction PB §12 records v0.15 closing."* (MF §4.8.4). Every reseal, every quick-lane landing copying no `Spine-Upgrade`, and every orphaned tombstone is signerless (GR §5.5, PB §11). PB §11's overlay wording is *"at least two"* in team mode, *"a floor and never an exact count"* — see *Contradictions* C4.

**R53 (checks 11–13 — MUST NOT produce a wire).** *"All three read event commits the landing does not copy. … So these three are refusals `spine check` and `spine check --approve` make in flight; they produce no wire in any landing report, and an implementation evaluating them at landing would be reading fields that are not there."* (MF §4.8.4, §9 R26). Check 12's exception is PB §4.3's, verbatim: *"unless that approval's key has since left the keyring"*.

### Part G — the chain rule (MF §4.8.4.1)

Governs **every landing whose `diff(B, Hc)` touches `.spine/allowed_signers`**. Three limbs, evaluated in two different situations:

| Limb | PB §7.5 (verbatim) | Situation | Status |
|---|---|---|---|
| the landing carries a `class=protected` review by a principal in **the parent's** keyring `K` | *"carry a protected review by a principal in the parent's keyring (≠ signer in team mode, two reviewers when there is no signer)"* | in-flight **and** landed | `chain-review-not-in-parent` |
| a delta that only **removes** entries takes one protected review from a remaining key that is **not** a removed entry's key; a delta that adds or edits an entry takes the full rule | *"a departed or compromised key is never asked to co-sign its own revocation"* | in-flight **and** landed | `chain-remover-removed` |
| the landing is sealed by a `spine-seal@v1` key in **the parent's** keyring | *"must be sealed by a pipeline key in the parent's keyring"* | **landed only** | `chain-seal-not-in-parent` |

**R54 (MUST).** The seal limb **MUST NOT** run in flight — *"the seal signs `envelope=`, which covers `report=`, which is the digest of the report this gate's own verdict sits inside."* It is evaluated on `spine index`'s first-parent walk — PB §7.5 verbatim: *"`spine index` walks trunk first-parent from the tip to the root"* — *"the same walk that derives §4.6's `valid_from`/`valid_to`, and its failure makes a landing `unattested` rather than refusing one."* (MF §4.8.4.1, §9 R27)

**R55 (MUST).** The **delta is over entries, not lines**: *"two keyrings are compared by their `(principal, fingerprint)` sets under §4.2's parse, so re-indenting the file is not a delta at all, and editing a line in place is one removal plus one addition."* (MF §4.8.4.1)

**R56 (MUST).** The reviewer limbs are **additional** to checks 7 and 9, never a substitute: *"check 7 says a team-mode protected review is not self-approved, check 9 says a signerless landing carries two, and this limb says every one of those reviewers was in the **parent's** keyring rather than the one the landing installs. Without it a landing could enrol a key and use it to authorize its own enrolment."* (MF §4.8.4.1)

**R57 (recovery form — MUST).** MF §4.8.4.1, verbatim: *"a `mode=recovery` seal verifies under `spine-review@v1` by one of two distinct protected reviewers in `K`, never the same key as the other reviewer and, when the landing has a signer, never that signer (PB §7.5); G9 and G15 admit that form only for the landings PB §7.5 enumerates."*

### Part H — what G13 does not check (MF §4.8.5)

**R58 (MUST NOT).** G13 **MUST NOT** check `C-A1`'s declared value. *"§4.5 fixes mode as the key count, and PB §11 wins. A `C-A1` disagreeing with the count is 'a warning on every report' (PB §6.3) and it is **not a wire**: Authority raises no `warn` kind (GR §6.1, GR §6.3), and a wire would put a constitution typo inside `wires=`, `report=` and `envelope=`, moving three digests over a value no check reads. It is a diagnostic `spine check` prints beside the report, in the same class as G5's per-pragma diagnostic, which GR §11 also keeps out of the report."* (§9 R28)

**R59 (MUST NOT).** G13 **MUST NOT** check the keyring in `T` (that is G16 check 13); **MUST NOT** check whether a signature is well-formed (*"OpenSSH's … G13 parses no SSHSIG"*); **MUST NOT** check the envelope's grammar or line order (EV's — *"G13 reads lines the envelope parser has already produced, and records the bytes it finds without normalizing them"*); **MUST NOT** own the trust root, rotation and revocation (PB §7.5); **MUST NOT** decide whether a landing *should* have a signer (PB §6's transition table).

### Part I — the verdict (MF §4.8.6)

**R60 (MUST).** Verdict function, verbatim:
- **`pass`** — no finding.
- **`override`** — *"every coverable finding's token appears in the union of the `wires=` of the `class=protected` reviews discharging this landing — each verifying under `spine-review@v1` against `K`, each carrying `head=Hc` and a `tree=` equal to the tree under evaluation, each by a reviewer eligible under §4.5 and checks 7 and 9 — and no outright finding fired."*
- **`fail`** — *"any outright finding, or any uncovered coverable finding."*

**R61 (REFUSE).** *"A `fail` makes the report a non-landing report (GR §5.6.1); the run refuses with `report-not-landable` and nothing is sealed."* (MF §4.8.6) GR §5.6.1 adds: *"A landing whose `Spine-Gates` copies a `fail` is malformed and G9 indexes it `unattested`."*

**R62 (MUST).** *"**G13's outright findings stay outright on every landing shape, a reseal included:** GR §5.6.1's reseal row suspends the G1 and G8 rows and no others, PB §5.5 naming those two gates and no other, and a reseal's own protected reviews are themselves statements checks 2, 7 and 9 read."* (MF §4.8.6, GR §5.6.1)

**R63 (MUST).** Outright is a **coverage** rule, never a **containment** rule (GR §5.6.1): a landing carrying an outright wire that reaches a review state still needs that wire **named** in the review's `wires=` to be consumable, and naming it still does not make the gate `override`.

### Part J — G13, in one place (MF §4.8.7, verbatim)

```
G13(K, mode, E, A, situation, B, Hc, intent_blob):
  wires := []; outright := []
  # 1 — the governing keyring (§4.4, §4.5); halts
  if K absent or lint(K, mode) ≠ []:            return FAIL_OUTRIGHT(lint(K, mode))
  # 2 — every event commit's signature, under the namespace its trailer requires (§4.8.3); halts
  claims_role := { Spine-Signoff, Spine-Approve, Spine-Review, Spine-Upgrade, Spine-Withdraw }
  for c in E:
      if principal(line(c)) ∉ principals(K):  continue   # §4.8.2 — void, a transition, not a finding
      st := verify(K, line(c), principal(line(c)), ns(line(c)))
      if st ≠ ok:
          if trailer(line(c)) ∈ claims_role:
                         outright += (st, oid(c))      # statement-unverified | statement-namespace
          else:          wires    += {G13, oid(c), protected, finding}
  if outright ≠ []:                             return FAIL_OUTRIGHT(outright)
  # 3..10 — accumulate
  if ∃ c ≠ c' ∈ E : line(c) = line(c'):         outright += event-line-duplicate
  if A.approve and not binding(A.approve, E, intent_blob):
                                                outright += approval-voided
  for r in reopens(E):
      if r.voids ≠ freeze_of_binding_before(r, E):
                                                outright += reopen-voids-mismatch
  if A.approve and needs_reason(A.approve) and A.approve.reason absent:
                                                outright += approve-reason-missing
  if mode = "team":
      for v in A.reviews where v.class ∈ {protected, break-glass} and v.self_approved:
                                                outright += self-approved-protected
  if A.withdraw and not withdraw_key_ok(A.withdraw, A.signoff):
                                                outright += withdraw-key
  if signer_key(A) = none:                                                  # GR §5.5
      n := |{ v.fingerprint : v ∈ A.reviews, v.class = "protected" }|
      if n ≠ (2 if mode = "team" else 1):       outright += signerless-review-count
  if keyring_touched(B, Hc):                    outright += chain(K, A, situation)   # §4.8.4.1
  if situation = in_flight:                     outright += in_flight_only(E, A)     # 11..13
  wires := sort(wires, key=token)                                                    # GR §6.1
  if outright ≠ []:                             return FAIL,     wires, outright
  if wires = []:                                return PASS,     [],    []
  if covered(wires, protected_reviews(A)):      return OVERRIDE, wires, []
  return FAIL, wires, []

needs_reason(a) := a.red has k = 0  ∨  a.held = false          # the third limb is check 13
binding(a, E, blob) := a = newest verifying Spine-Approve in E
                     ∧ a is later in E than the last Spine-Reopen
                     ∧ a.intent = blob
                     ∧ ¬∃ r ∈ reopens(E) : r.voids = a.freeze
withdraw_key_ok(w, s) := (ns(w) = "spine-signoff@v1" ∧ s present ∧ fp(w) = fp(s))
                       ∨ (ns(w) = "spine-review@v1"  ∧ (s absent ∨ fp(w) ≠ fp(s)))
```

**R64 (MUST).** `withdraw_key_ok`'s `s absent` limb is the **orphaned tombstone** (GR §5.5, PB §11): *"the sign-off key has left `K`, so the sign-off is omitted from `A`, the withdraw line carries `orphaned=<principal>`, and there is no fingerprint for the reviewer to differ from. Such a landing is signerless, so check 9 requires the reviewers the overlay demands."* (MF §4.8.7)

### Part K — what solo vs team mode changes (collected)

| Aspect | solo (exactly one `spine-signoff@v1` fingerprint) | team (two or more) |
|---|---|---|
| how mode is decided | key count over `K`, never `C-A1` (MF §4.5, PB §11) | same |
| `keyring-seal-mixed` | **not evaluated** — the one principal holds all three namespaces (MF §4.5) | evaluated, in both directions (PB §6.3, MF §4.5) |
| `keyring-no-seal` | **not evaluated** (§4.4 row is team-scoped) | evaluated — refused (PB §6.7) |
| check 7 (`self-approved-protected`) | **not evaluated**: protected and break-glass reviews are self-signed by definition, recorded in `self_approved`, counted by `spine stats`, *"never refused"* (PB §7.2's table, MF §4.8.4) | evaluated: *"reviewer ≠ signer; refused otherwise"* (PB §7.2) |
| check 9 (signerless) | **one** `class=protected` review (MF §4.8.4, PB §11) | **two** with distinct fingerprints (MF §4.8.4); PB §11 says *"at least two"* |
| chain-rule reviewer limb | one protected review by a principal in the parent's keyring | additionally `≠ signer`; two reviewers when there is no signer (PB §7.5) |
| the seal | `mode=solo` on the `Spine-Seal` line; the pipeline namespace may be the human's own key (PB §7.2) | `mode=team`; the pipeline key is a CI secret no laptop holds (PB §7.2) |
| `spine init --rotate-trust-root` | permitted (solo only) | **refused** — *"a team recovers through a recovery landing"* (PB §7.5) — note this one refusal reads the **declared** `C-A1`, see C3 |
| tripwire review | self allowed, recorded `self_approved` | self allowed — *"the signer knows the intent best, and the wire is a quality wire"* (PB §7.2) |

### Part L — `--pipeline-key`, `--signer-key`, and what team mode strips

**R65 (MUST).** `spine init --pipeline-key <pub>` **appends the seal line to the keyring** — PB §6.7, verbatim: *"except `--pipeline-key`, which appends the seal line to the keyring: that landing is a keyring change under the chain rule (§7.5), and in team mode it strips the seal namespace from every human line; G13 refuses a team-mode keyring with no `spine-seal@v1` principal — so a repo that starts solo and offline can grow a remote and a pipeline without a second bootstrap"*.

**R66 (MUST).** The stripping is a property of **the landing that enters team mode**, not of `--pipeline-key` alone. PB §7.2, verbatim: *"the landing that enters team mode strips the seal namespace from every human line, and any later human seal is `unattested` — except the recovery form of §7.5."*

**R67 (MUST).** `spine init --signer-key <pub>` *"enrols a human signing key in the keyring under `spine-signoff@v1` and `spine-review@v1`"* (PB §11 CLI). Omitted, `init` takes the single key in `ssh-add -L`, else the single `~/.ssh/*.pub`, and **refuses with instructions when neither is unambiguous** *"rather than guessing which key a repository's authority will rest on"*. `--identity <principal>` names the principal, **defaulting to the key's comment**. *"A first `init` with no signing key cannot produce a trust root and says so."* (PB §11)

**R68 (MUST).** Each additional person is enrolled by *"a keyring change under the chain rule"* (PB §9 adoption notes, line 931), not by a further `--signer-key`.

**R69 (MUST NOT).** `--pipeline-key` **MUST NOT** be assumed to write a canonical line shape: MF §13 OPEN-2 records emitting one as an *unadopted* option (b). Today the file is lint-only.

### Part M — trust root, rotation, revocation, recovery (PB §7.5)

**R70 (MUST).** **Bootstrap.** *"The trust root is the commit `spine.trustRoot` pins — at first init, the commit introducing `.spine/allowed_signers`, which `spine init` signs with a key inside it. Its SHA is pinned out-of-band, like the release hash: the rendered CI snippet reads it from a provider variable (`SPINE_TRUST_ROOT`), never a tracked file, and `spine check --ci` refuses to run without one — trust-on-first-use is a laptop convenience (`spine index` prints the root and its fingerprints once and stores it in `git config spine.trustRoot`, the only per-clone spine setting), never a CI mode. `spine init` prints the root SHA and the variable to set as its last line; changing a stored pin takes an explicit `spine init --trust-root <sha>`."*

**R71 (MUST).** **Retirement and revocation are both deletion of the line, in one protected PR.** (PB §7.5)

**R72 (MUST).** **This is the one landing a member of a two-signer team makes alone**; `spine stats` counts it (PB §7.5).

**R73 (MUST).** **Two verification clocks** (PB §7.5, verbatim): *"In-flight signatures are verified against the *current* keyring: revoke a key and every intent it signed drops to `awaiting-sign-off`, every approval or review it signed is void, on the next `spine check` — to be redone by someone else. Landed signatures are verified against the keyring *as of the seal's base*: history does not become invalid when people leave — but `spine check --authority` lists every landing signed by a since-revoked key, which is exactly the list a compromise post-mortem needs."*

**R74 (MUST).** **Recovery landing** (PB §7.5, condensed but with the operative clauses verbatim):
- Trigger: *"With no usable pipeline key — lost, or a pin that cannot be installed"*.
- Seal: *"a landing may be sealed under `spine-review@v1` by one of two distinct protected reviewers from the parent's set (when the landing has a signer, that signer may be one of the two but never the sealing one); its seal carries `mode=recovery`"*.
- Diff confinement: *"the landing's `diff(B, L)` is confined to `.spine/allowed_signers` and the constitution's `C-A1` line — or, for a rollback, uninstall or re-init, to the manifest and the `spine-owned` and `user-modified` paths the two manifests list — never a `user-owned` one; anything else makes the seal `unattested`."*
- Admission: *"G9 and G15 accept that form only for a landing whose keyring delta removes or replaces every `spine-seal@v1` principal, a **rollback** landing … an **uninstall** (`to=none`), or a re-init landing (`from=none`)"*.
- *"Two humans are always at least one human plus a pipeline — the path is honest, not hidden."*
- Residual, stated: *"Compromise of the CI secret holding the pipeline key equals compromise of landing for non-floor code: OIDC-scoped short-lived keys and hardware-backed keys are recommended, not enforced."*

**R75 (REFUSE).** **Rotation.** *"`spine init --rotate-trust-root` is refused when `C-A1` is `team` — a team recovers through a recovery landing. A solo developer whose only key is gone lands a rotation root carrying `Spine-Trust-Root-Prev: <sha>`, re-pinned out-of-band; the indexer continues the walk below it against the old chain, marking the boundary in every affected signer's `valid_to`. Only a trust-root commit lands directly (§6.7)."* (PB §7.5)

### Part N — the keyring in G16 and in the two lifecycle shapes

**R76 (MUST).** G16 check 13: *"`K_T` passes §4.4's lint, including the mode-dependent clauses evaluated under §4.5's key count"* — kind **coverable**, statuses the `keyring-*` tokens (MF §6.2). G16 runs on every landing **except a tombstone** (MF §6.1).

**R77 (REFUSE).** Uninstall (`to=none`, MF §6.8): *"`.spine/allowed_signers` and the constitution in `T` are byte-identical to `B`'s"* → `uninstall-keyring-changed` / `uninstall-constitution-changed`, **outright**. *"The keyring clause is not redundant with the `user-owned` clause: it is what makes a later re-init's `since=` check meaningful (PB §6.3 G16), and it is stated separately because the re-init check compares against exactly this file."*

**R78 (REFUSE).** Re-init (`from=none`, MF §6.9): *"`.spine/allowed_signers` in `T` is byte-identical to the keyring at `since=`"* → `reinit-keyring-differs`, **outright**. *"The re-init is not the place to change the keyring, because its own seal and reviews verify against the keyring at `since=`."*

**R79 (MUST).** A rollback restoration **MUST NOT** touch the keyring: MF §8.6 — *"Step 6 fails outright if `CONSTITUTION.md` or the keyring appears in `diff(tree(B), T)` at all"*, because both are `user-owned` in both manifests (MF §3.5: a `user-owned` path's appearance in a rollback's diff is an **outright** failure).

---

## Byte-level fixities

**F1 — line termination (MF §4.2, verbatim).** *"The file is a sequence of lines, each terminated by `0x0A`; a final line without a terminator is accepted (OpenSSH accepts it) and is not an error. `0x0D` anywhere is `keyring-cr`."*

**F2 — the grammar.** Reproduced verbatim in R11 above. Note `WS := %x20 / %x09` and `WS+` between fields, so the aligned, multi-space spelling PB §7.2 prints is conforming:

```
# .spine/allowed_signers — roles are namespaces; ssh-keygen enforces them
alice@example.com  namespaces="spine-signoff@v1,spine-review@v1"  ssh-ed25519 AAAA…
bob@example.com    namespaces="spine-signoff@v1,spine-review@v1"  ssh-ed25519 AAAA…
ci@example.com     namespaces="spine-seal@v1"                     ssh-ed25519 AAAA…
```

**F3 — the three namespace tokens, exact bytes.** `spine-signoff@v1` · `spine-review@v1` · `spine-seal@v1` (PB §11 *Roles and namespaces*, MF §4.3). Closed domain. The `namespaces=` option value is a `DQUOTE`-delimited, comma-separated list with no spaces in the grammar.

**F4 — the fingerprint (MF §4.2, verbatim).** *"`ssh-keygen -lf` over `<keytype> <keyblob>`: `\"SHA256:\"` plus unpadded base64."* Reproduce with, verbatim (MF §8.7): `ssh-keygen -lf <(printf '%s %s\n' ssh-ed25519 AAAA…)`

**F5 — sign and verify command forms (PB §11, MF §4.8.3, verbatim).**
- Sign: `ssh-keygen -Y sign -n <namespace>` over *"the exact bytes of that line"*, terminator excluded.
- Verify: `ssh-keygen -Y verify -f .spine/allowed_signers -I <principal> -n <namespace> -s <sig>`
- PB §11's by-hand form: `ssh-keygen -Y verify -f .spine/allowed_signers -I alice@example.com -n spine-signoff@v1 -s <sig> < <line>`

**F6 — the signed-statement shape (PB §7.2, verbatim).** *"One trailer line ending in `signer=<principal>` (reviews: `reviewer=`), plus `<Name>-Sig: <SSHSIG, armor stripped to one line>` produced by `ssh-keygen -Y sign -n <namespace>` over the exact bytes of that line; `reason=` values are JSON string literals."*

**F7 — a reader normalizes nothing (GR §5.5, verbatim).** *"`<name>` `:` one space `<payload>` is a writer's constraint, not a reader's rewrite. … A reader records what it finds — two spaces, no space, anything — because the signature is over the line's exact bytes, so a line that is not what it should be fails G13's verification, and that refusal is the check. A report that silently reshaped the line would hash bytes nobody signed."*

**F8 — the wire token (MF §4.8.1, GR §6.3, verbatim).** `G13:` + the offending event commit's object id, *"lowercase hex at the length `object_format` implies, for which both `esc` and `tok` are the identity"*. `oidlen := 40 if object_format = "sha1" else 64`. One wire per commit, deduplicated under GR §6.1's `(gate, path)` rule.

**F9 — wire ordering (PB §11, `Spine-Review` row, verbatim).** *"ascending by unsigned byte value over the whole token, so `G11` precedes `G2`; a set with no order is a signature two runs spell differently"*. MF §4.8.7 sorts `wires` by token; MF §7 rule 5 defers array ordering for wires to GR §6.1.

**F10 — `roles` ordering (MF §4.6).** *"the entry's namespaces, ascending by bytes."*

**F11 — `E`'s ordering (MF §4.8.2, GR §5.5.1, verbatim).** *"the branch's event commits, ancestor-first along `git rev-list --reverse --first-parent B..H`, extended past `Hc` to `H`"*. GR §5.5.1 adds: *"Event commits are created on the branch tip, so they are always on that path; a `Spine-*` line on a commit that is not on it is not an event commit of this branch and is not recorded."*

**F12 — DM provenance for a signer node (DM §12 vector, verbatim).** `"src":"git:0a1b2c3d4e5f60718293a4b5c6d7e8f901234567:.spine/allowed_signers:1"` — the trailing integer is the **1-based line number** of the entry in the keyring.

**F13 — determinism (MF §7, verbatim rule 1).** *"**No wall clock.** No member of the manifest is a time; the keyring's validity is the first-parent chain; `valid-after=`/`valid-before=` are refused. G13's *two clocks* are two commits — trunk's tip and the seal's `base=` — and neither is a time (§4.8.2)."* Rule 3: *"G13, G14 and G16 read git objects reachable from `B`, `Hc`, `H`, `T`, `<sha>` and `since=`, plus constants inside the pinned release. No note, no side file, no cache, no prior run. G13's `total_rounds=` check counts `rounds=` over event commits **on the branch**, never over a memory of previous runs."*

**F14 — the keyring's `files[]` record (MF §8.3, verbatim excerpt).**

```
{"blob":"6d4db08390092d7d5d96476eddca6355815bc49f","owner":"user-owned","path":".spine/allowed_signers","template":"keyring@1"}
```

**F15 — git requirements bearing on the keyring (PB §11).** *"OpenSSH ≥ 8.2 (`ssh-keygen -Y`) and an SSH signing key per human signer"*. v1 supports **SSH signatures only**; OpenPGP is v1.1 (PB §7.2). *"The versioned namespace suffix is the cheapest place to version a signature payload format."*

---

## Error cases

### Keyring lint — the closed list (MF §4.4)

Kind column: **outright** when raised by G13 over `K_B` (check 1); **coverable** (`class=protected` `G16` wire) when raised by G16 over `K_T` (check 13). Same token either way (MF §4.5).

| Status token | Condition (verbatim where quoted) | Mode-scoped? | Behaviour |
|---|---|---|---|
| `keyring-missing` | *"the file is absent from the tree"* — *"there is no authority without it"* | no | G13 check 1 halts → FAIL_OUTRIGHT |
| `keyring-empty` | *"no entry lines"* | no | idem |
| `keyring-line-malformed` | *"a line matches neither `blank`, `comment` nor `entry`"* | no | idem |
| `keyring-cr` | *"any `0x0D`"* | no | idem |
| `keyring-multi-principal` | *"an entry naming more than one principal"* | no | idem |
| `keyring-no-namespaces` | *"an entry with no `namespaces=` option"* | no | idem |
| `keyring-option-unknown` | *"any option other than `namespaces=`"* | no | idem |
| `keyring-validity-option` | *"`valid-after=` or `valid-before=`"* | no | idem |
| `keyring-cert-authority` | *"the `cert-authority` option"* | no | idem |
| `keyring-namespace-unknown` | *"a namespace outside the three"* | no | idem |
| `keyring-namespace-empty` | *"`namespaces=\"\"`"* | no | idem |
| `keyring-keytype-unknown` | *"a keytype outside §4.2's list"* | no | idem |
| `keyring-key-not-base64` | *"a key blob that is not base64, or that does not decode to a key of the declared type"* | no | idem |
| `keyring-duplicate-line` | *"two entries with the same `(principal, key)`"* | no | idem |
| `keyring-duplicate-principal` | *"two entries with the same principal and different keys"* | no | idem |
| `keyring-key-two-principals` | *"one key (by fingerprint) under two principals"* | no | idem |
| `keyring-seal-mixed` | *"in **team** mode, a key holding `spine-seal@v1` and any other namespace"* | **team only** | idem |
| `keyring-no-seal` | *"in **team** mode, no principal holding `spine-seal@v1`"* | **team only** | idem |

MF §4.4's closing note, verbatim: *"`keyring-missing` … `keyring-key-not-base64` are pure lints of the file. The last five read the file plus one constitution value (`C-A1`), and are shared with G13 (§4.5)."* (See C2 — the `C-A1` half of that sentence is contradicted by §4.5 and §4.8.5.)

### G13 findings

| Check | Status token | Kind | Behaviour |
|---|---|---|---|
| 1 | any `keyring-*` of §4.4 | outright | **halts** — `return FAIL_OUTRIGHT(lint(K, mode))`; gate reads `fail`; run refuses `report-not-landable` |
| 2 (five roles) | `statement-unverified` | outright | halts after the loop — `return FAIL_OUTRIGHT(outright)` |
| 2 (five roles) | `statement-namespace` | outright | idem |
| 2 (any other trailer) | — (no status; a wire) | **coverable** | `G13:<oid>`, `class=protected`, `kind=finding`; commit excluded from state derivation **either way**; discharged by a protected review naming the token → gate reads `override` |
| 3 | `event-line-duplicate` | outright | accumulate → `fail` |
| 4 | `approval-voided` | outright | accumulate → `fail` |
| 5 | `reopen-voids-mismatch` | outright | accumulate → `fail` |
| 6 | `approve-reason-missing` | outright | accumulate → `fail` |
| 7 | `self-approved-protected` | outright (team only) | accumulate → `fail` |
| 8 | `withdraw-key` | outright | accumulate → `fail` |
| 9 | `signerless-review-count` | outright | accumulate → `fail` |
| 10 | `chain-review-not-in-parent` | outright | in-flight **and** landed |
| 10 | `chain-remover-removed` | outright | in-flight **and** landed |
| 10 | `chain-seal-not-in-parent` | outright | **landed only**; failure makes the landing `unattested`, it does not refuse a run |
| 11 | `total-rounds-mismatch` | outright, **in-flight only** | refusal by `spine check`; **no wire in any landing report** |
| 12 | `approval-redundant` | outright, **in-flight only, at `--approve`** | idem |
| 13 | `approve-reason-missing` | outright, **in-flight only, at `--approve`** | idem |
| — | *void statement* (principal ∉ `principals(K)`) | **not a finding** | a **transition** PB §6's table consumes; statement absent from `A` |
| — | `C-A1` count mismatch | **not a finding, not a wire, not a `gates[]` status** | a **diagnostic** `spine check` prints beside the report (MF §4.8.5, §9 R28) |

### Gate status / run outcome

| Condition | `Spine-Gates` entry | Run behaviour |
|---|---|---|
| no finding | `G13=pass` | landing may seal |
| only coverable findings, all named in the union of the discharging `class=protected` reviews' `wires=` | `G13=override` | landing may seal |
| any outright finding, or any uncovered coverable finding | `G13=fail` | report is a **non-landing report**; run refuses with status **`report-not-landable`**; nothing is sealed (MF §4.8.6, GR §5.6.1) |
| a landing whose `Spine-Gates` copies a `fail` | — | malformed; **G9 indexes it `unattested`** (GR §5.6.1) |
| a recovery-sealed landing carrying an outright finding | — | *"also indexes `unattested`"* (MF §4.8.1, PB §7.5) |
| break-glass review naming G13 | — | **inert** — G13 is not on PB §7.6's list `{G1,G2,G3,G4,G6,G7,G8,G12}` (MF §4.8.1, GR §5.6.1 *"**no** — Authority is never on PB §7.6's list"*) |

**Exit codes.** No exit code is defined for G13 itself anywhere in the corpus; the refusal is carried by the status token `report-not-landable`. The only exit codes in the neighbourhood are `spine check --verify`'s (GR §4.3): `0 verified` · `1 report-mismatch` / `candidate-mismatch` · `2 report-unavailable` · `3 wrong-release` / `wrong-git` / `report-version-unknown` · `4 not-recomputable`. These are failures *of the copy or of the record, never of the landing* (GR §4.3) and are not G13's.

---

## Worked examples / test vectors

### The published keyring (MF §8.7 — byte-identical to EV §8.1)

```
alice@example.com namespaces="spine-signoff@v1,spine-review@v1" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMb068SxqsLNkSdlCVXeIPOcHOPCh/TemT4tv9iJnqla
bob@example.com namespaces="spine-signoff@v1,spine-review@v1" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINZJBgqcpDmx19xO9D29xeFtCCUMyfe/ti+lY7c+rvim
ci@example.com namespaces="spine-seal@v1" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICE3UkO6VDz+9ag4xQScwsfdP8PHJhLh+QWkIwzUjWze
```

**411 bytes, three entries, blob `6d4db08390092d7d5d96476eddca6355815bc49f`** (MF §8.7, MF §8.1). *"These are EV §8.1's three keys, byte for byte … no private key is published and none is needed to verify."*

| Principal | Fingerprint (`ssh-keygen -lf`) | Namespaces |
|---|---|---|
| `alice@example.com` | `SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM` | signoff, review |
| `bob@example.com` | `SHA256:V2dasTIGWUnhlaxa7vr3Qmqpe/qYBoxi+C7Cs6yxxEs` | signoff, review |
| `ci@example.com` | `SHA256:eQ0ZoC+rlhZstRuvhLXwJlwqLAreBcmnyFedpvPmTVY` | seal |

**The lint, walked (MF §8.7, verbatim).** *"three entry lines, no blanks, no comments, no CR; one principal each; `namespaces=` present on all three and the only option; every namespace in the domain; every keytype `ssh-ed25519`; three distinct fingerprints under three distinct principals, so neither `keyring-key-two-principals` nor `keyring-duplicate-principal`; two distinct signoff keys, so `mode = team`; `ci@example.com` holds `spine-seal@v1` and nothing else and no human holds it, so no `keyring-seal-mixed`; a seal principal exists, so no `keyring-no-seal`. Clean."*

*"`mode = team` is computed from the key count and not from `C-A1` (§4.5). CN §12.1's constitution declares `C-A1: team`, so the count and the declaration agree and no warning is raised."* (MF §8.7)

EV §8.1 states the consequence for the vector: *"Two distinct signoff keys, so `C-A1` is **team** and `reviewer ≠ signer` binds (PB §7.2): alice signs, bob reviews."*

### G16 check 13 over the same repository (MF §8.5)

> `13 | pass — §8.7's keyring lints clean under `mode = team``

Landing verdict there: `G16 = pass`, no wires; the landing is still `protected-review` from G14's six floor hits.

### G13 across the corpus's vectors

README and MF §11 C13 both record: *"G13 raises no wire in any vector in the corpus, every one of which reads `G13=pass`."* EV §8.4's seal and PB §5.5's envelope both print `G13=pass` inside `Spine-Gates`.

### The derived `signer` nodes (DM §12 vector, verbatim)

```
{"attrs":{"fingerprint":"SHA256:dDNTLP8TJNB4MJxBYyoReNyBoxiCqhv9TqEUICFm3BM","roles":["spine-review@v1","spine-signoff@v1"],"valid_from":"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567"},"id":"myrepo/signer:alice@example.com","kind":"signer","src":"git:0a1b2c3d4e5f60718293a4b5c6d7e8f901234567:.spine/allowed_signers:1","t":"node"}
{"attrs":{"fingerprint":"SHA256:V2dasTIGWUnhlaxa7vr3Qmqpe/qYBoxi+C7Cs6yxxEs","roles":["spine-review@v1","spine-signoff@v1"],"valid_from":"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567"},"id":"myrepo/signer:bob@example.com","kind":"signer","src":"git:0a1b2c3d4e5f60718293a4b5c6d7e8f901234567:.spine/allowed_signers:2","t":"node"}
{"attrs":{"fingerprint":"SHA256:eQ0ZoC+rlhZstRuvhLXwJlwqLAreBcmnyFedpvPmTVY","roles":["spine-seal@v1"],"valid_from":"0a1b2c3d4e5f60718293a4b5c6d7e8f901234567"},"id":"myrepo/signer:ci@example.com","kind":"signer","src":"git:0a1b2c3d4e5f60718293a4b5c6d7e8f901234567:.spine/allowed_signers:3","t":"node"}
```

Note `signer.valid_to` is **absent, not null**, on all three (DM §12 note 6). Dump vector digest: **62 lines, 14054 bytes, `sha256:3321e7bd4b5113d5b2a987535e262bc8b12963266555216504b5c946716812da`** (README published-digest table; MF §11 C12 records the 2026-08-27 adoption of `ci@example.com` in place of an invented fourth key set under `pipeline@ci.example.com`).

### Regeneration hazard (MF §8.7 — record it in the test harness)

*"**The three sites a keyring regeneration moves, listed so the next one misses none.** **§8.3** — the `files[]` record for `.spine/allowed_signers` carries the keyring blob, so the manifest's own blob id and both SHA-256 rows move (the byte counts do not: a blob id is fixed-width). **§8.5** — check 10's second limb quotes §8.3's manifest blob id, so it must be requoted; it printed a stale `5e83bbb…` until 2026-08-27, which matched nothing in the corpus. **§8.6** — every digest taken over a manifest containing the keyring. §8.7's own 411-byte count and blob id are the input, not an output."*

Manifest blob for that repository: **1762/1763 bytes, `cb4cd49034bbe25f76573c40d6711b2c33f9136f`** (README).

### Reproduction commands (MF §8.7, verbatim)

```sh
git hash-object <file>                       # every blob above
shasum -a 256 artifacts.txt                  # cli.dist_hash
ssh-keygen -lf <(printf '%s %s\n' ssh-ed25519 AAAA…)   # every fingerprint
python3 -c 'import json,sys;                 # canonical manifest bytes
  d=json.load(open(sys.argv[1]));
  sys.stdout.buffer.write(json.dumps(d,sort_keys=True,separators=(",",":"),
                                     ensure_ascii=False).encode())' m.json
```

---

## Cross-references it depends on (which other sheet owns what)

| Owned elsewhere | Owner | What G13/the keyring consumes from it |
|---|---|---|
| The `authority` object — `signoff`, `approve`, `reopens[]`, `reviews[]`, `upgrade`, `withdraw`; the statement shape (`line`/`fingerprint`/`namespace`); `self_approved`; **which reviews bind**; the ancestor-first ordering of `reopens`/`reviews` | **GR §5.5, §5.5.1** | the whole of `A`, G13's second input |
| `pass` / `override` / `fail` domain, the outright table, `report-not-landable`, `unattested` indexing, the reseal suspension being G1+G8 only | **GR §5.6.1** | the verdict's consequences |
| Which gates run per landing shape (G13 = ✓ on all four) | **GR §5.6.2** | R30 |
| The `wires` array members, `(gate, path)` dedup, wire ordering, `esc`, `tok`, and the `G13:<oid>` token | **GR §6.1, §6.2, §6.3** | F8, F9, R35 |
| `--verify` exit codes | **GR §4.3** | the *Error cases* footnote |
| The envelope grammar and line order; the `-Sig` line; the published signed lines and their verification | **EV** (and MF §4.8.5: *"The envelope's grammar, or the order of its lines. EV's."*) | the bytes check 2 verifies |
| The `signer` node id, attrs (`roles`, `fingerprint`, `valid_from`, `valid_to`) and provenance `src` | **DM §5.2, §7.2, §12** | MF §4.6's parse contract, R23's motivation |
| `C-A1`'s declared value, its domain, and `team` as its fail-closed default | **CN §6.1, §7.2** | *not* an input to any G13 check (R58); only the diagnostic |
| The manifest schema, canonical bytes, frozen fields, ownership classes, `templates`/`resign` — and **G14** and **G16** | **MF §2, §3, §5, §6** (concern sheets 02 / 04) | `.spine/allowed_signers`'s `files[]` record; G16 check 13; §6.8/§6.9 keyring clauses; the rollback restoration rule |
| The shipped floor `F0`, `cf`, the diff entry set, `floor_hits` | **MF §5** (G14 sheet) | why every keyring landing is `protected-review` before G13 speaks |
| PB §6's transition table (what a void statement *does*) | **PB §6** | R38 — G13 supplies the predicate only |
| PB §4.3's freeze closure and the closure tripwire | **PB §4.3 / IR §2.5** | check 13's condition; check 6 explicitly does **not** recompute it |
| `spine init`'s plan, `--merge`, `--adopt`, `--force`, `--abort`, staging | **PB §6.7** (MF §12 out of scope) | `--signer-key` / `--pipeline-key` behaviour only |
| G15 (tool pin) | **PB §6.3 + MF §3.2** (MF §12 declines the gate) | co-gate on a tombstone and on the recovery form |

---

## OPEN items

These are undecided owner questions. **No value is invented here.**

1. **MF §13 OPEN-2 — should `.spine/allowed_signers` have a canonical form after all?** *"§4.1 says no, because it is `user-owned` and OpenSSH is the reader. The cost is that a keyring change's diff is not canonical, so two protected reviewers may be reading a whitespace change and a key change in one hunk with nothing distinguishing them, and `spine stats` cannot count 'lines changed' without a parse. Options: (a) no canonical form, lint only — the status quo of §4; (b) `init --pipeline-key` and `--signer-key` emit a canonical line shape and G16 warns (never fails) when a line is not in it; (c) require it, making the keyring effectively machine-written. **Recommendation: (b)** … Owner-level because (c) would change PB §6.7's ownership class."* **Build against (a).**

2. **CN §16 OPEN-9 / CN §15 D15 — `C-A1` versus the keyring count.** Three options, verbatim: *"(a) the declaration governs and a mismatch is a warning, as PB §6.3's G13 row says — under which `C-A1: solo` in a five-key repository self-approves every protected landing; (b) the count governs and `C-A1` is documentation, as PB §7.2 and PB §11 say; (c) the **maximum** governs — `team` if either says two or more — and a mismatch is a G13 finding rather than a warning. **Recommendation: (c).**"* MF §4.8.5 implements **(b)** and names the exact three edits (c) would cost, verbatim: *"`manifest.md` §4.5's `mode` becomes `\"team\"` unless *both* the count and `C-A1` read solo; a new **outright** check joins §4.8.4 with status `mode-declaration-mismatch`; and GR §5.6.1's G13 row gains it. Nothing else in §4.8 changes, because every check that reads `mode` reads it through §4.5."* **Build against (b), but route every read of `mode` through one function so (c) is a three-line change.** Status token `mode-declaration-mismatch` is reserved but **not** in v1's vocabulary.

3. **MF §10 D11 — OPEN against the playbook.** *"Two keys under one principal are representable in the keyring and not in the graph (PB §7.2 against `dump.md` §5.2's `signer` node id). … **Fix:** refuse it in G16's keyring lint (§4.5), or key `dump.md`'s signer node on the fingerprint and republish its vector."* MF §4.5 takes the first horn (`keyring-duplicate-principal`); MF §11 C4 records it *"as a choice `dump.md` may reverse."*

4. **MF §13 OPEN-1 (`params.ci` monotonicity)** and **OPEN-4 (unknown `templates` key)** touch the `keyring` template row only incidentally — `templates.keyring` exists in every manifest whether or not a record names it — and are G16/manifest questions, not keyring ones. README also files `params.ci` as *"one owner question"* filed in three specs.

5. **No maximum file size, no maximum entry count, no maximum principal length is fixed anywhere** for `.spine/allowed_signers`. (The manifest is bounded at 1 MiB, MF §6.2 note on check 3; the keyring is not.) Not marked OPEN by any document — recorded here as a genuine gap an implementer must decide defensively without claiming spec authority. Related: TM §7.1's `Owner:` principal is capped at **128 bytes**, but that cap is TM's and is not stated of the keyring's `principal`.

---

## Contradictions found

**C1 · MF §4.2's grammar vs MF §4.4's closed status list — the grammar is too tight for its own error vocabulary.** §4.2's `entry` production makes `options` mandatory and admits *only* `namespaces="<one of three>[,…]"`, and `keytype` admits only eight tokens. So a line carrying `cert-authority`, `valid-after=`, `namespaces=""`, `namespaces="spine-signof@v1"`, `ssh-rsa`, or no options at all does not match `entry` and — read literally — is `keyring-line-malformed`. But §4.4 assigns each of those a *distinct* status (`keyring-cert-authority`, `keyring-validity-option`, `keyring-namespace-empty`, `keyring-namespace-unknown`, `keyring-keytype-unknown`, `keyring-no-namespaces`). The statuses win (they are the closed list, and §4.3 argues explicitly that a typo'd namespace must be `keyring-namespace-unknown` and *"not ignored"*), so the lint must field-split permissively and classify. Reported because two implementations reading §4.2 literally and §4.4 literally emit different tokens for the same file, and the token is what a reviewer's `wires=` names when G16 raises it.

**C2 · MF §4.4's closing note vs MF §4.5 and §4.8.5 — does the lint read `C-A1`?** §4.4, verbatim: *"The last five read the file plus one constitution value (`C-A1`), and are shared with G13 (§4.5)."* §4.5, verbatim: *"A `C-A1` disagreeing with that is a warning, not a finding, **and not an input to any check**."* §4.8.5 repeats it and calls the mismatch a diagnostic. **§4.5 governs** (it is the section §4.4 itself cites, and PB §11's count reading wins under MF §1). The lint's mode-dependent clauses read the **key count**, never `C-A1`. Additionally, §4.8.4 check 1 and §6.2 check 13 both say *"five mode-dependent clauses"*, but only **two** of the last five (`keyring-seal-mixed`, `keyring-no-seal`) are mode-scoped; `keyring-duplicate-line`, `keyring-duplicate-principal` and `keyring-key-two-principals` hold in both modes. Implement two mode-scoped clauses, not five.

**C3 · PB §7.5's rotation refusal reads the declared `C-A1`, against MF §4.5's count.** PB §7.5, verbatim: *"`spine init --rotate-trust-root` is refused when `C-A1` is `team`"* — the declared constitution value. Everywhere else in the authority path mode is the key count (PB §11, MF §4.5). Not reconciled by any document. Note it is a **tool** refusal, not a gate, so it does not sit inside a digest; but a repository whose count says team and whose `C-A1` says solo could rotate its trust root under the literal reading. This is CN §16 OPEN-9's blast radius reaching one more site than OPEN-9 enumerates.

**C4 · "two" vs "at least two" for the signerless overlay.** MF §4.8.4 check 9, verbatim: *"`A.reviews` holds **two** `class=protected` reviews with distinct fingerprints in team mode"*, and MF §4.8.7 encodes it as `if n ≠ (2 if mode = "team" else 1)` — an **exact** count. PB §11's overlay, verbatim: *"carries **at least two** distinct `class=protected` reviews in team mode … **a floor and never an exact count, since a third reviewer signing a contentious reseal is diligence and must not be the thing that refuses the landing**"*, and PB §6.3's G13 row also says *"at least two"*. **PB §11 wins under MF §1**, so the implementation MUST use `n < (2 if team else 1)` and MF §4.8.7's `≠` is a defect. Under MF's literal spelling a third diligent reviewer refuses the landing — precisely the outcome PB §11 rules out in terms. Not filed in MF §10.

**C5 · MF §4.3's role table drops two qualifiers PB §7.2 carries.** PB §7.2's *Signs* column for the pipeline role reads *"the seal; approvals carrying `run=` **once B runs in the trusted stage**"* and its *Held by* column reads *"the trusted stage — **a CI secret no laptop holds**; in solo mode, the human's own key"*; MF §4.3 compresses both. Not a mechanical disagreement (MF §4.8.3's `run=` rule is exact either way), but the trusted-stage precondition on `run=` approvals lives only in PB §7.2 and PB §11's `Spine-Approve` row.

**C6 · MF §4.4 gives `keyring-empty` for *"no entry lines"* while §4.4's own `keyring-missing` covers absence — and neither says what a file of only comments and blanks is.** A file consisting solely of `comment` and `blank` lines has no entry lines, so it is `keyring-empty` by the row's text. That reading is consistent, but it is worth pinning in tests: a keyring commented out is `keyring-empty`, not `keyring-line-malformed`.

**C7 · The two `approve-reason-missing` sites share one token across two evaluation situations.** MF §4.8.4 assigns `approve-reason-missing` to **check 6** (a landing check, outright) and to **check 13** (in-flight only, at `--approve`). Since check 13 *"produce[s] no wire in any landing report"* (§9 R26) while check 6 does, a consumer reading only the token cannot tell which limb fired. MF §9 R25 defends the split deliberately, so this is a naming collision rather than a rule conflict — but an implementation SHOULD carry the check number alongside the token internally.

**C8 · No contradiction, recorded to forestall one: MF §4.8.1's *"exactly one coverable check"* and GR §6.3's *"One finding of G13's carries it, and G13 raises no other wire"* agree**, and GR §5.6.1's G13 row (*"every finding but one"*) agrees with both. MF §11 C13 records GR adopting MF §4.8's split as **CLOSED**, with *"no published digest moves: G13 raises no wire in any vector in the corpus."* The three documents are consistent on G13's wire surface.
