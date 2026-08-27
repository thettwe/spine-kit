# Build decisions — v1

Settled. Propagate them; do not re-litigate them. Each names what it closes and
where the consequence lands.

## Owner rulings, 2026-08-27

**1 · The two §7.1 isolation defects are fixed in the code *and* in the spec.**
`result-file.md` §7.1 amended, recorded as **§13 R36**, indexed in
`docs/spec/README.md`. M1's root takes a second, empty lower layer on a `tmpfs`;
P4(a) is written over the reachable interface set (`IFF_UP`, plus the address
set) rather than a device count, enumerated normatively over netlink. §12's
"the particular system call … is out of scope" is narrowed by exactly one
clause: free over *creation*, not over *measurement*. No byte of the file
grammar moved and every §10 vector stands. Evidence in
[`FINDINGS-isolation.md`](FINDINGS-isolation.md).

**2 · `repo` is the basename of the toplevel.**
`basename $(git rev-parse --show-toplevel)`, refused with `repo-out-of-grammar`
when it fails MF §3.1's `^[A-Za-z0-9._-]+$` or the 64-byte bound. **No `--repo`
flag**, so PB §11's CLI signature is unchanged. Consequence, and it is loud
rather than silent: `repo` is a frozen field and DM §5.2 builds every node id
from it, so renaming the checkout directory before a re-init makes the computed
value disagree with the manifest's, and G16 refuses the landing.

**3 · `--strategy` is dropped from v1.**
It had no conforming write target: `params` has exactly five members and all are
frozen (MF §3.3, PB §11's frozen twelve), and the constitution is `user-owned`
and never rewritten after the seed (PB §6.7). Merge strategy stays at `C-M1`,
which a human edits under the protected review the constitution already takes.
**PB §11's `spine init` signature loses one token**; nothing else moves, because
the flag could write nothing today.

**4 · Uninstall removes every `spine-owned` path in `M_B`, modified or not.**
MF §6.8's outright check wins over PB §6.7's "removes **clean** `spine-owned`
paths". Each deleted-but-modified path is named loudly in the output. It is the
only implementation whose uninstall can land, and the human's bytes stay
reachable through git history. **PB §6.7's "clean" is the sentence that gets
amended** — not yet done, tracked below.

## Plan defaults taken as recommended

From `00-BUILD-PLAN.md` §1, where the fail-closed reading is unambiguous. Each
is a decision the implementation makes and records, not an owner ruling.

| # | Item | Taken |
|---|---|---|
| B2 | `--ci` has no default | **Refuse when omitted**, exactly as `--langs` refuses. Detecting from the tree is wrong: a stale `.gitlab-ci.yml` would silently pick `gitlab` and permanently retire auto-merge precondition 2 (CI §8.1). |
| B3 | `--trunk` has no default | The branch `git symbolic-ref --short HEAD` names; **refuse on a detached HEAD**. On `--ci github`, additionally refuse when it is not the provider's default branch (CI §7.1). |
| B6 | `<run>` staging grammar | **At most one staging directory exists at a time**; `<run>` is a 32-hex random nonce — not a clock (MF §7 rule 1 bars one), covered by no digest, gitignored. A second `init` that finds one treats it as the interrupted case and never creates a second. |
| B7 | the plan's `delete` token | Delete a `files[]` path **iff** its `owner` is `spine-owned` **and** the new render set does not name it. Every `user-owned` and `user-modified` path stays and is reported. `create`/`update`/`skip` are display only. |
| B8 | `supersedes` / `superseded_by` direction | `supersedes`: superseding → superseded. `superseded_by`: superseded → superseding. **Emit both.** The only reading under which both names mean what they say and PB §6.6's "archaeology queries return the current truth first" is answerable. |
| B9 | `changeset.tool_version` split | Split at the **last** `+sha256:` (RF §13 R14; unambiguous because `<dist_hash>` is exactly 64 lowercase hex), take the left half, carry the digest in no attr. DM §12.2's vector is the only evidence and must match. |
| B10 | shipped-floor `protects` edges | DM's own option (c): **emit none**; G14 reads the release constant `F0` directly. Option (b) would add a `release` node kind, move `PRAGMA user_version` to 8 and change PB §6.2 — the owner's, and not needed. |
| B11 | `dist_base` + the three Action pins | **Values only, still open** (CI §18 OPEN-1, OPEN-7). Does not block: a development build's refusal of every plan row with `no-release-manifest` *is* the specified behaviour (CI §3.4) and is testable. Tests supply a fixture release manifest. |

## Implementation decisions

- **Language: Rust**, workspace at the repo root. Matches the target triples
  CI §5.5 froze (`x86_64-unknown-linux-musl`, `aarch64-apple-darwin`), gives the
  self-hash-verify at start-up one file to hash, and gives M1 direct
  `clone(2)`/`unshare(2)`/`pivot_root(2)`. The `py3-none-any` wheel is a
  downloader shim.
- **No serde.** GR §2.2 needs duplicate-member *refusal*, integer-only numbers
  and a depth bound over untrusted input; a permissive library gets all three
  wrong silently. `spine-canon` carries a hand-written parser and JCS writer.
- **`sha2` is the one crypto dependency.** SHA-1 is hand-written (git blob ids
  in a `sha1` repository only, never a security primitive — PB §11's hash
  policy makes every security digest SHA-256) and checked against the FIPS
  180-1 vectors.
- **M1 tests run in a Linux container**; the probe decision logic sits behind an
  injectable seam so `profile=none` on Darwin is a tested path, not an untested
  fallback.

## Amendments owed to `PLAYBOOK.md`

Not yet made. Each is one sentence and each follows from a ruling above.

1. §11's `spine init` signature: drop `--strategy` (ruling 3).
2. §6.7's uninstall sentence: "removes **clean** `spine-owned` paths" → removes
   every `spine-owned` path in the base manifest, naming the modified ones
   (ruling 4).
