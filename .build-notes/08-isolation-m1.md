# 08 — The isolation boundary: M1, the probe P1–P4, the two dispositions, the restore phase

Scope of this sheet: everything a Rust implementer needs to build the collector's **step 6** (establish + test the boundary), the **restore phase**, the **two network dispositions**, and the derivation of the result-file header field **`profile=`**. It does not own the result file's grammar, ingestion order, gate allocation, or the manifest's own checks — see *Cross-references*.

**Authority rule applied throughout (RF §1):** where a spec and PB §11 (Vocabulary) disagree, §11 wins; otherwise the spec is normative and resolves PB's ambiguity. Both directions are exercised below and every disagreement found is in *Contradictions found*.

---

## Sources read

| File | Lines | Section |
|---|---|---|
| `/Users/thettwe/Works/spine-kit/docs/spec/result-file.md` | 1–30 | title block, spec version, the three amendment paragraphs (incl. the 2026-08-27 M1/network/P4 amendment) |
| " | 46–62 | **§3 Path and naming** — incl. *"Where the directory lives is the isolation profile"* |
| " | 73–115 | **§4.2 The header line** (field 5 `profile`), **§4.3 canonical JSON** |
| " | 298–420 | **§7.1 Order of operations** in full: steps 1–10; *The isolation boundary — step 6 in full*; the profile table; *M1 — the shipped mechanism*; *M1's root — an overlay…*; *M1's identity source…*; the five-prerequisite table; *M1's two network dispositions*; *The restore phase*; *What is now enforced…*; *The probe, and the four tests* + the P1–P4 table; *The verdict*; the two dispositions of failure; *The collector never upgrades…*; *What the boundary is not*; *The deadline*; *The collector passes no selection* |
| " | 426–460 | **§7.3** status fold (for `runner-timeout` / `base-collect-failed`), **§7.4 Outside `--ci`** |
| " | 502–519 | **§8.4 Preconditions of §7.4 rule 5** |
| " | 580–614 | **§9 What a candidate can and cannot influence** (the *Egress, and the restore script* row) |
| " | 614–660 | **§10 Worked example** (published header line, `profile=container`) |
| " | 719–768 | **§11 Conformance checklist** items 16 and 16a; **§12 Out of scope** (M1 syscall, images, `SPINE_ALLOWED_HOSTS`, restore-script contents) |
| " | 805–806 | **§13 R33, R34** |
| " | 811–833 | **§14 OPEN**, incl. OPEN-9 (closed) |
| `/Users/thettwe/Works/spine-kit/PLAYBOOK.md` | 782–805 | **§7.1 Least privilege per stage** (the table + Keys column + Injection defense) |
| " | 835–882 | **§7.3 The protected floor**, **§7.4 rules 0–5** (rule 3's three profile bullets and the *"Each of the three has a predicate"* paragraph; rule 5's preconditions 0–4) |
| " | 983–1040 | **§11 Vocabulary** — `Spine-Seal` grammar, *Files and refs*, *Landings that run no suite*, the `spine init` CLI line |
| " | 1117–1130 | §12 change log — *The bounded fix pass…*, *The final external review…*, *Three costs the owner confirmed* |
| `/Users/thettwe/Works/spine-kit/docs/spec/ci.md` | 139–168 | §5.1 invocation contract, **§5.2 exit codes** |
| " | 190–216, 436–455, 476–501 | §5.3 the script: `umask 022` block, the registry-allowlist block, the collect tail + published digests |
| " | 501–550 | **§5.4** items 1–9, **§5.5** platform table (Darwin target) |
| " | 550–562 | **§5.6 The registry allowlist** — enforced vs declared, in full |
| " | 566–585 | §6.1 the untrusted job (U1–U8) |
| " | 1445–1461 (grep) | §17 out of scope — container images deferred to RF |
| `/Users/thettwe/Works/spine-kit/docs/spec/manifest.md` | 128, 154, 163, 270–281, 308, 864, 875, 888, 1057 (grep) | `params.isolation` type/domain/default; **§6.2 check 12b**; status token `isolation-unsupported` |
| `/Users/thettwe/Works/spine-kit/docs/spec/gate-report.md` | 136, 273, 547, 560, 592–612, 1095–1097, 1121 (grep) | `profile` member domain, precondition 1 recomputability, the seal's `profile` when nothing/unattested was ingested |
| `/Users/thettwe/Works/spine-kit/docs/spec/README.md` | 1–88 | index: RF v3 status, OPEN-9 closed, published digests (`ci.md` §5.3 319 lines / `131f13fb…` / `sha256:d6bcf50c…`) |

---

## Data model

### D1 — Policy inputs (read at step 1, from `origin/<trunk>` only)

| Field | Type | Domain | Default | Required | Source |
|---|---|---|---|---|---|
| `params.isolation` | string | `"container"` \| `"uid"` \| `"none"` | **absent ⇒ `none`** | optional, **frozen** | trunk's `.spine/manifest.json` (MF §3.3 line 154; RF §7.1 step 1) |
| `params.timeout` | integer | **strictly positive** seconds | **absent ⇒ `1800`** | optional, frozen | trunk's manifest (RF §7.1 *The deadline*) |
| `params.langs` | array of string | v1: `python`, `ts`, `dart`, `swift` | — | required | trunk's manifest (RF §7.1 step 3) — determines the invocation set, not the boundary |
| `object_format` | string | `sha1` \| `sha256` | — | required | fixes `<T>`'s hex length, hence P1(c)'s path |
| `.spine/restore.sh` | blob (bytes) | arbitrary `sh` source | **absent ⇒ the restore phase is empty** | optional | `origin/<trunk>:.spine/restore.sh` (RF §7.1 *The restore phase*) |

**`params.isolation` is a *request*, never a capability** (RF §7.1; PB §2.1 line 142: *"A request, not a capability. Rule 5 decides per run (§7.4)."*).

### D2 — The finding: header field 5

| Field | Type | Domain | Notes |
|---|---|---|---|
| `profile` | string | `container` \| `uid` \| `none` | RF §4.2 field 5. **`n/a` is never a header value, and a header carrying it is malformed** (RF §4.2). `uid` is written by **no v1 collector** (RF §4.2, §7.1, §11 item 16). |

Downstream (not this sheet's to write, recorded so the implementer does not conflate them):
- **Seal** `profile=` domain is **four** values: `container|uid|none|n/a` (PB §11 `Spine-Seal`).
- **Gate report** `profile` member domain is the same four (GR §5 member table, line 273) and is `R` = recomputable from the seal.

### D3 — The collector's own identity constants

| Name | Meaning | Where used |
|---|---|---|
| `U` | the collector's own **real uid** | P2's exclusion set; M1's identity source |
| `Ug` | the collector's own **real gid** | P2's exclusion set |

Verbatim (RF §7.1): *"`U` and `Ug` below are the collector's own real uid and gid."*

### D4 — M1's namespace set

Exactly five, all created for the child: **mount, PID, IPC, network, user** (RF §7.1 *M1 — the shipped mechanism*; RF §13 R33/R34). No other namespace is required or forbidden by the spec.

### D5 — The two dispositions of one boundary

| Disposition | Network namespace | Everything else | Who runs in it |
|---|---|---|---|
| **restore disposition** | **does not unshare**; keeps the job's own network namespace | identical: mount, PID, IPC and user namespace, root (read-only overlay), writable tree, masked result directory, mapped identity, pipes | **exactly one phase per checkout** — the restore phase |
| **runner disposition** | **fresh network namespace holding only loopback** (`lo`, brought up by the collector) | identical as above | **every runner invocation, without exception**, and **the probe boundary** |

Verbatim (RF §7.1 *M1's two network dispositions*): *"identical in mount, PID, IPC and user namespace, in root, in writable tree, in masked result directory, in mapped identity and in pipes, and differing in exactly one thing, the network namespace"*; and *"Exactly one phase per checkout runs in the first, everything else in the second, and which is which is fixed by the collector rather than chosen per runner."*

Environment of the restore disposition: *"`SPINE_ALLOWED_HOSTS`, `SPINE_REGISTRY_PROXY` and the client variables `ci.md` §5.6 sets are in its environment, for whatever the host puts in front of the socket to read"* (RF §7.1).

### D6 — M1's five host prerequisites (RF §7.1, verbatim table)

| # | Prerequisite | Absent when |
|---|---|---|
| 1 | the four filesystem-and-process namespaces — mount, PID, IPC, user — creatable by this collector; the **network** namespace is prerequisite 5 | not Linux (`ci.md` §5.5 ships a Darwin target); a kernel or seccomp policy that refuses `unshare`; a nested runner that already spent them |
| 2 | an identity source — a delegated `subuid`/`subgid` range with `newuidmap`/`newgidmap`, or a collector running as uid 0 | neither present, which is the bare-metal shared runner with no `uidmap` package |
| 3 | a read-only overlay over the job's root, mountable inside the namespace | unprivileged overlayfs unavailable and the collector is not root; a lower layer the kernel refuses to stack |
| 4 | a filesystem the mapped id can traverse — the checkouts of `B` and `T`, and the binary the probe re-execs | the collector inherits a `0700` umask. `ci.md` §5.3 narrows `.spine/ci.sh`'s process-wide `umask 077` to `umask 022` plus an explicit `chmod 0700 "$WORK"` and `0755` on the install directory and the binary, for exactly this reason (`ci.md` §5.4 item 1) |
| 5 | a **network namespace**, creatable by this collector, with a loopback device it can bring up | a kernel or seccomp policy that refuses `CLONE_NEWNET`; a runtime that hands the child a network namespace it may not replace; a child that holds no `CAP_NET_ADMIN` even inside its own user namespace, so `lo` cannot be brought up and the runner disposition would be a namespace with no usable interface at all |

### D7 — M1's identity source (RF §7.1, exactly two arrangements, one required)

1. **Delegated subordinate ids** — host grants the collector's uid a range in `/etc/subuid` and `/etc/subgid` and supplies `newuidmap`/`newgidmap` to write the child's `uid_map`/`gid_map` (equivalently the collector holds `CAP_SETUID`/`CAP_SETGID` on the host). The child runs as an id inside that range.
2. **A root collector that drops privilege** — where the job runs the collector as uid 0 (the ordinary case on a container-based CI runner, `ci.md` §7.2's untrusted job being itself already a container), `U` is 0, the collector maps the child to a **fixed unprivileged id**, and any non-zero id satisfies P2.

**Which of the two a host supplies never reaches the file** — *"Both are M1, both license `container`, neither is recorded, and the header carries no trace of the difference — so two collectors on two hosts of different arrangements write the same bytes."* (RF §7.1)

### D8 — The probe artifacts

| Object | Property | Citation |
|---|---|---|
| **canary** | a file created by the collector inside the result directory, opened `O_CREAT\|O_EXCL`, **under a name no other process can predict**; its bytes are held **in memory** by the collector | RF §7.1 probe step 1 |
| **probe boundary** | built from **exactly the runner disposition** M1 will use for a runner invocation, **differing only in that its writable tree is an empty directory of the collector's own making** | RF §7.1 probe step 2 |
| **probe process** | re-execs the hash-verified collector binary (prerequisite 4 names *"the binary the probe re-execs"*) | RF §7.1 prerequisite 4 |
| **residue** | **none.** *"No probe artifact survives step 6, whatever the outcome"* | RF §7.1 probe step 4 |

### D9 — The restore phase

| Field | Value |
|---|---|
| Bytes | `origin/<trunk>:.spine/restore.sh` — **never from a checkout** |
| Interpreter | `sh` over those bytes |
| Working directory | the root of the checkout it is running for |
| Disposition | **restore disposition** |
| Identity / root / result-dir masking | the same mapped identity, the same read-only overlay, the same masked result directory a runner gets |
| Count | **two per run** — one for `B` (inside step 7), one for `T` (inside step 8) — *"never one per runner, whatever the invocation set holds"* |
| Timing | after each checkout and **before the first runner invocation against it** |
| Deadline | `params.timeout`, like an invocation; on expiry kill its process group and reap it |
| Environment | *"the collector's own, unchanged"* |
| Exit code | **nothing reads it** |
| Contribution to the file | **none**: no `base` record, no `result` record, no id, no `status` contribution |
| Absent on trunk | the phase is **empty**, no process runs, one diagnostic to stderr; *not* a prerequisite failure, *not* a downgrade |
| `files[]` record / template | **none** — `spine init` does not write it, `manifest.md` §6.2 requires no record for it |
| Floor status | `.spine/**` is protected floor (PB §7.3, which names *"the optional `restore.sh` the collector reads from trunk"*) |

---

## Normative requirements (numbered)

Each is MUST / MUST NOT / REFUSE / SHOULD with its citation.

**Reading the request**

1. **MUST** read `params.isolation` from `origin/<trunk>`, never from the checkout, at step 1 of §7.1. (RF §7.1 step 1; PB §7.4 rule 1)
2. **MUST** treat an absent `params.isolation` as `none`. (RF §7.1 step 1; MF §3.3)
3. **MUST** treat `params.isolation` as a *request* and `profile=` as a *finding*: *"the collector writes it only where a test it performed, and could have failed, licensed it."* (RF §7.1)
4. **MUST NOT** write a finding stronger than the request. Verbatim: *"A finding is never stronger than the request; the collector never substitutes a mechanism for a profile the request did not name; and comparing the two is the trusted stage's job, never the collector's."* (RF §7.1)
5. **MUST NOT** compare `profile=` with `params.isolation` in the collector — that comparison belongs to the trusted stage (RF §4.2, §8.4).

**Disposition 1 — the `uid` refusal**

6. **REFUSE:** `params.isolation == "uid"` ⇒ **fail the job and write nothing, at step 1**, before `T` exists. (RF §7.1 *The verdict*, disposition 1; §11 item 16)
7. **MUST NOT** downgrade a `uid` request to `none`. Verbatim: *"It is never a downgrade to `none`."* Rationale recorded: *"`none` would spend a permanently sealed field (PB §11, `Spine-Seal`) on a defect the repository can neither see nor fix, dressing a refusal as a green run that merely cannot auto-merge."* (RF §7.1)
8. **MUST** ship **no** `uid` mechanism in v1, and therefore **MUST NOT** ever write the header value `uid`. (RF §4.2 field 5, §7.1 profile table, §11 item 16)
9. The refusal's observable consequence at the shell layer is `.spine/ci.sh`'s existing one and needs no change: no file at the expected path ⇒ **exit 2**, *"Refused. Nothing ran and no result file exists."* (RF §7.1 disposition 1; CI §5.2)

**Disposition 3 — `none` requested**

10. **MUST**, where `params.isolation == "none"`, attempt **no boundary at all** and write `profile=none`. (RF §7.1 *The verdict*)
11. **MUST NOT** build a boundary anyway and write `container` under a `none` request — *"a collector that builds one anyway and writes `container` is non-conformant"* (RF §7.1 *The collector never upgrades, and never substitutes*).

**M1 — the mechanism**

12. **MUST**, where `params.isolation == "container"`, use **M1 and only M1**: each runner spawned as a child in a new **mount, PID, IPC, network and user** namespace over an **overlay of the job's own root filesystem**. (RF §7.1 *M1 — the shipped mechanism*)
13. **MUST NOT** pull or name a container image. Verbatim: *"No image is pulled and none is named."* Consequences fixed by the spec: *"that is why `params` needs no image key and `.spine/ci.sh` passes the collector no isolation argument."* (RF §7.1; RF §12; CI §17)
14. **MUST** make the **only** writable non-scratch path the **tree under test** — the detached checkout of `T` on a `T` run, the collector's checkout of `B` on a `B` invocation — **together with one private temporary directory**; everything else in the child's view is read-only. (RF §7.1)
15. **MUST** make `.spine/cache/` **absent from the child's view**, by one of exactly two arrangements: (a) **masked** where the runtime can mask a subpath, or (b) the result directory lives **outside the mounted root** and the file is moved into place **after the process group is reaped**. (RF §3, §7.1)
16. **MUST NOT** treat a mounted, writable result directory as `container`: *"a mounted, writable result directory does not [satisfy `container`], whatever the configuration claims — which is what P1 measures."* (RF §7.1, §3)
17. **MUST** map the child's uid and gid to values that are **neither the collector's nor 0**, and that are visible as such **to the host**. (RF §7.1)
18. **MUST** hold the runner's **stdout and stderr as pipes on the host side**. Verbatim: *"No runner stream is ever a file inside the boundary: a stream the boundary can rewrite is not evidence."* (RF §7.1)
19. **MUST** give the runner's network namespace nothing but loopback: *"It is fresh, it is empty but for a `lo` device the collector brings up, and no interface, bridge, veth pair, connected socket or file descriptor from the job's own namespace is moved into it or passed across. A runner reaches `127.0.0.1` and `::1` and nothing else."* (RF §7.1)

**M1's root**

20. **MUST** mount an **overlay whose only layer is a lower layer** — the job's own root filesystem — i.e. an overlay with **no upper layer**, read-only by construction. (RF §7.1 *M1's root*)
21. **MUST** then **bind-mount the writable tree over the overlay afterwards**, mount the private temporary directory as a **`tmpfs`** the same way, and **`pivot_root`** the child into the result. (RF §7.1 — the exact sequence)
22. **MUST NOT** use a bare mount namespace over the job's own root as M1's root. Reason fixed by the spec: *"A mount namespace over the job's root *is* the job's root — `stat` on the child's `/` returns the collector's own `(device, inode)` pair, so P3's separation limb would fail on every host, for every configuration, forever. An overlay is a distinct filesystem with its own device, so the pair differs."* (RF §7.1)

**M1's identity source**

23. **MUST** obtain the child's identity from **one of exactly two** arrangements (D7): a delegated `subuid`/`subgid` range with `newuidmap`/`newgidmap` (or `CAP_SETUID`/`CAP_SETGID` on the host), **or** a root collector that maps the child to a fixed unprivileged id. (RF §7.1 *M1's identity source*)
24. **MUST NOT** rely on a bare unprivileged user namespace as the identity source. Reason, verbatim: *"an unprivileged user namespace with no delegated subordinate range cannot supply one: it maps exactly one host uid, `U` itself, so the file P2 `stat`s comes back owned by `U` and the test fails on every host, for every configuration, forever."* (RF §7.1)
25. **MUST NOT** record which of the two arrangements was used — it reaches no header field and no artifact. (RF §7.1)
26. Using either arrangement is **not** the forbidden cross-mechanism fallback: *"that rule is about `params.isolation`'s *named mechanism*, of which v1 ships exactly one, and M1 obtaining its id map two ways is inside M1."* (RF §7.1)

**The two dispositions**

27. **MUST** spawn **every runner invocation** — the `B` enumeration, the separate `B` outcome run where an adapter has one, and the `T` run — under the **runner disposition**, without exception. (RF §7.1; §11 item 16a)
28. **MUST** run **exactly one phase per checkout** in the restore disposition, and everything else in the runner disposition; which is which is **fixed by the collector, never chosen per runner**. (RF §7.1)
29. **MUST** build the **probe boundary** from the **runner disposition** (not the restore one), differing only in the writable tree. (RF §7.1 probe step 2; §11 item 16)
30. **MUST NOT** unshare the network namespace for the restore disposition — it keeps the job's own. (RF §7.1)

**The restore phase**

31. **MUST** run one restore phase per checkout, **after the checkout and before the first runner invocation against it** — at step 7 for `B`, at step 8 for `T`. **Two per run, never one per runner.** (RF §7.1 *The restore phase*; §11 item 16a; CI §5.6)
32. **MUST** read its bytes from `origin/<trunk>:.spine/restore.sh` and **MUST NOT** read them from any checkout, `T`'s included. (RF §7.1; §11 item 16a; RF §9 *Egress, and the restore script*)
33. **MUST** run it as **`sh` over those bytes, at the root of that checkout**, in the restore disposition, under the same mapped identity, the same read-only overlay and the same masked result directory a runner gets. (RF §7.1)
34. **MUST** bound it by `params.timeout`; on expiry **kill its process group and reap it**, and **proceed with the run**. (RF §7.1 *The restore phase*, *The deadline*; §11 item 10)
35. **MUST** give it **the collector's own environment, unchanged** — it moves no header field, and `keys_visible`'s first conjunct already covers it. (RF §7.1)
36. **MUST NOT** let it contribute anything to the file: no `base` record, no `result` record, no id, no `status` contribution. (RF §7.1; §11 item 16a)
37. **MUST NOT** read its exit code. A non-zero exit is *"a diagnostic on stderr and the run proceeds"*. (RF §7.1)
38. **MUST**, where `.spine/restore.sh` is absent on trunk, treat the phase as **empty** — no process runs — and write **one diagnostic to stderr saying so**. **MUST NOT** treat the absence as a prerequisite failure, a failure, or a downgrade. (RF §7.1; §11 item 16a)
39. **MUST** run the restore phase **irrespective of the profile** — under `params.isolation: "none"` and on the solo path alike: *"same bytes, same source, same place in the order, same deadline — simply with no boundary around it."* (RF §7.1 *The phase is not conditioned on the profile*)
40. On the solo path the bytes still come from `origin/<trunk>`; **a working copy with no such remote-tracking ref has no restore phase**, which is the empty case of R38. (RF §7.1)
41. **MUST NOT** filter the restore phase's egress by hostname inside the collector. `SPINE_ALLOWED_HOSTS` narrowing is *"a network policy, a proxy sidecar or an egress firewall the host puts in front of the socket"*. (RF §7.1 *What is now enforced*; RF §12; CI §5.6)

**The probe, step 6, in order**

42. **MUST** perform step 6 **before `B` or `T` is checked out and before any repository process has ever run**, and after step 5 has computed `T` (so `<T>` is known and P1(c) has a path). (RF §7.1 steps 5–7, probe preamble)
43. **MUST** create the result directory (§3) and write a **canary** into it, `O_CREAT|O_EXCL`, **under a name no other process can predict**, holding its bytes **in memory**. (RF §7.1 probe step 1)
44. **MUST** create a probe boundary from exactly the runner disposition, its writable tree being **an empty directory of the collector's own making**. (RF §7.1 probe step 2)
45. **MUST** run **P1, P2, P3 and P4** inside it. (RF §7.1 probe step 3)
46. **MUST** reap it, tear it down, and remove the canary, leaving **no probe artifact** whatever the outcome. (RF §7.1 probe step 4)
47. **MUST** establish and test **one configuration once** — `profile=` is **not per-runner and not per-invocation**. *"a collector that isolates one runner and not another has achieved the weaker profile."* (RF §7.1; §3)
48. **MUST** place step 6 **before** step 7 (the `B` checkout): *"trunk's own tests are code that runs in the job too, and a floor enumerated by an uncontained process is a floor the job's other processes had a write path to."* (RF §7.1)

**The four tests** (verbatim in *Byte-level fixities*)

49. **P1 — Containment. MUST** attempt, **by absolute path**, four things and require **all four to fail**, *and* the canary's bytes read back on the host side after the probe is reaped to be **unchanged**. **One success is a failed test.** (RF §7.1 P1)
50. **P2 — Identity. MUST** decide from **the host's view**: the probe's reported real+effective uid and gid all outside `{0, U, Ug}`, **and** the file it created in its writable tree owned, *as the host sees it*, by neither `U` nor 0. (RF §7.1 P2)
51. **P3 — Separation. MUST** require the collector's own pid **absent** from the probe's process table **and** the probe's root a **different `(device, inode)` pair** from the collector's root. (RF §7.1 P3)
52. **P4 — Egress. MUST** require **both** limbs: (a) the interface set is **exactly one device, loopback**, carrying no address other than `127.0.0.1/8` and `::1/128`; **and** (b) a **non-blocking** `connect(2)` to `192.0.2.1:443`, bounded at **one second**, **fails**. (RF §7.1 P4)
53. **P4 MUST** treat a **completed** connect as a failed test **and** a connect **still pending when the one-second bound expires** as a failed test: *"pending means a route existed."* (RF §7.1 P4)
54. **MUST NOT** drop either P4 limb: *"The two limbs check each other and neither alone would do."* (RF §7.1 P4)

**The verdict**

55. **MUST** write `profile=container` **iff** `params.isolation == "container"` **and P1 ∧ P2 ∧ P3 ∧ P4 all passed**. (RF §7.1 profile table + *The verdict*; §11 item 16)
56. **MUST** write `profile=none` in **every other case that reaches step 7**. *"There is no third outcome and no partial one: three tests out of four is `none`."* (RF §7.1)
57. **MUST NOT** run any runner inside a boundary whose test failed — *"runs no runner inside a boundary that did not pass its test"*; the collector tears down whatever it built and proceeds unisolated. (RF §7.1 disposition 2; §11 item 16)
58. **MUST** write a diagnostic to **stderr** naming **which** of M1's five prerequisites, or **which** of P1, P2, P3, P4, failed — and the diagnostic **MUST** distinguish *host could not build the boundary* from *boundary failed a test*, because *"a host that cannot build the boundary and a boundary that failed P1, P2, P3 or P4 differ to the human reading the `G11` wire, even though the header field they produce is the same."* (RF §7.1; CI §5.1: *"every diagnostic, and all of the collector's own output, goes to stderr"*)
59. **MUST NOT** make the disposition-2 downgrade silent. It is loud in three places at once: (i) the header carries `none` against a manifest saying `container`, failing auto-merge precondition 1 and raising the `class=tripwire` `G11` wire with a mandatory `reason=`; (ii) `profile=none` is **sealed into the landing and stays in the ledger forever**; (iii) the stderr diagnostic of R58. (RF §7.1; §8.4; PB §7.4 rule 5)
60. **MUST NOT** treat disposition 2 as a refusal. *"rule 5's answer to weak isolation is always that a human reads the landing, never that nothing happens."* (RF §7.1; §8.4)
61. **MUST NOT**, in a future multi-mechanism release, attempt any mechanism other than the one the request names: *"a fallback order across mechanisms would make the header a function of the host's inventory rather than of the request, and two collectors of the same build would then disagree on the same repository."* (RF §7.1)

**Outside `--ci` (the solo path)**

62. **MUST**, outside `--ci`, attempt **no boundary at all**, **refuse nothing** (a manifest declaring `uid` costs a solo developer no run — *"disposition 1 of §7.1 is a `--ci` rule"*), and write `profile=none`. (RF §7.4; §11 item 16)
63. Solo-path `keys_visible` is settled at **`true`** by §4.2's own predicate; with `profile=none` this makes rule 5's preconditions 1 and 2 **fail by construction**. (RF §7.4)

**Where the result file lives, per profile (RF §3)**

64. **Under `container`: MUST** put the result directory **outside the boundary's filesystem view, on the host side** (R15's two arrangements). (RF §3, §7.1 profile table)
65. **Under `uid`** the directory would be **mode `0700` under the collector's own uid, inside the collector's own working copy of `B` — never inside the detached checkout of `T`** — but no v1 collector reaches this row. (RF §3, §7.1 profile table)
66. **Under `none`:** anywhere in the job; **any process in it can write the file**, and the design says so rather than pretending. (RF §3, §7.1 profile table, §9)
67. **MUST** write the file exactly once, by temp-and-rename, after every process group is reaped: temp file in the same directory, `O_CREAT|O_EXCL`, unpredictable name, `fsync`, `rename()` over `<T>.jsonl`, **overwriting a pre-existing file without comment**. (RF §3)

**Boundary scope — what it is not**

68. **MUST NOT** conflate the boundary with `keys_visible`: *"`keys_visible` is a predicate over the collector's own environment *and* every runner invocation's (§4.2), so hiding key material from a contained runner does not make it `false` where the collector itself could reach it, and the two header fields are independent."* (RF §7.1)
69. **MUST** state the residual honestly: P1–P4 *"measure the boundary between the collector and the runner — its filesystem, its identity, its process table and its route off the host — which is the only thing `container` has ever claimed"*; PB §7.4: *"they isolate the collector from the runner, and nothing more."* (RF §7.1; PB §7.4)

**Deadline interaction (owned by the deadline sheet; restated because the restore phase is inside it)**

70. **MUST** enforce `params.timeout` on **every runner invocation** *and* on **each of the two restore phases**; a collector that enforces no deadline is **non-conformant** whatever the manifest says. Worst-case wall time = `params.timeout` × (number of invocations **+ 2**). (RF §7.1 *The deadline*; §11 item 10)
71. **REFUSE:** `params.timeout` present and not a strictly positive integer ⇒ **fail the job, write nothing** (step 1's shape). (RF §7.1 *The deadline*)

**Conformance negatives (RF §11 items 16 and 16a — each is a MUST NOT)**

72. **MUST NOT** write `container` without having run the four tests. (RF §11 item 16)
73. **MUST NOT** answer a `uid` request with a file rather than a refusal. (RF §11 item 16)
74. **MUST NOT** substitute one mechanism for another. (RF §11 item 16)
75. **MUST NOT** run any runner inside a boundary whose test failed. (RF §11 item 16)
76. **MUST NOT** spawn a runner with egress. (RF §11 item 16a)
77. **MUST NOT** take the restore script from a checkout. (RF §11 item 16a)
78. **MUST NOT** let the restore phase contribute to the file. (RF §11 item 16a)
79. **MUST NOT** treat a missing restore script as a prerequisite failure. (RF §11 item 16a)

**Host-side prerequisites the *shell* must satisfy (CI, restated because M1 depends on them)**

80. `.spine/ci.sh` **MUST** set `umask 022` (not `077`), `chmod 0700 "$WORK"`, and `0755` on `$INSTALL_DIR` and `$BIN` — because *"at 077 every checkout it writes, and every file under `$INSTALL_DIR`, is unreachable to that id and M1 fails a prerequisite rather than a test."* (CI §5.3, §5.4 item 1)
81. `.spine/ci.sh` **MUST NOT** pass the collector any isolation argument, and `params` **MUST NOT** carry an image key. (RF §7.1 first bullet)
82. **SHOULD** note the platform reach: M1 needs kernel namespaces and therefore **exists on Linux only** (prerequisite 1); `ci.md` §5.5's platform table also ships a Darwin target, where M1 **cannot be created at all** — *"that is not a refusal but disposition 2"*. (RF §7.1)

**Manifest-side gating of `uid` (not this sheet's to implement; recorded so the residual is understood)**

83. `spine init` accepts only `--isolation container|none` (PB §11 CLI line) and **MF §6.2 check 12b** fails a landing where `params.isolation` at `T` is `"uid"`, **outright**, status token **`isolation-unsupported`**. Disposition 1 is therefore *"the residual for a manifest that reached trunk before either existed, or around them; it is not a path a conforming toolchain opens."* (RF §7.1; MF §6.2 check 12b, §6.2 note *On check 12b*; PB §7.4 rule 3)

---

## Algorithm

The collector's step 6 and the two restore phases, in the order §7.1 fixes. Steps 1–10 are §7.1's own numbering; 6.x and 7.x/8.x are this sheet's expansion.

1. Read policy from `origin/<trunk>`: `cli.version`, `cli.dist_hash`, `params.isolation`, `params.langs`, `params.timeout`, `object_format`.
   - `params.isolation` absent ⇒ `none`; `params.timeout` absent ⇒ `1800`.
   - **If `params.isolation == "uid"` ⇒ REFUSE HERE: fail the job, write nothing** (R6). This is *before* `T` exists.
   - If `params.timeout` is present and not a strictly positive integer ⇒ fail the job, write nothing (R71).
2. Verify own bytes against the pinned artifact list. Mismatch ⇒ fail the job, write nothing.
3. Compute the invocation set from `params.langs`. A declared language with no adapter ⇒ fail the job, write nothing.
4. Probe key visibility (§4.2) and hold the boolean. (Independent of the boundary — R68.)
5. Compute `T := git merge-tree --write-tree origin/<trunk> H`. Conflict ⇒ no `T`, no file, fail the job.
   - **`T` is now known**, which is what gives P1 limb (c) a concrete absolute path.
6. **Establish the isolation boundary, test it, record what the test licensed.**
   - **6.0** If `params.isolation == "none"` ⇒ **attempt nothing**, set `profile := none`, go to step 7. (R10)
   - **6.1** Create the result directory `.spine/cache/results/` per §3. Create the **canary** inside it, `O_CREAT|O_EXCL`, unpredictable name; **hold its bytes in memory**. (R43)
   - **6.2** Check M1's five host prerequisites (D6). Any absent ⇒ **disposition 2**: tear down whatever was built, `profile := none`, stderr diagnostic naming **which prerequisite** failed, go to step 7. (R57, R58)
   - **6.3** Build M1's boundary in the **runner disposition**:
     1. unshare **user** namespace and install the id map from the identity source of D7 (delegated range via `newuidmap`/`newgidmap`, or root→fixed unprivileged id);
     2. unshare **mount**, **PID**, **IPC**;
     3. unshare **network**; bring up `lo`; move **nothing** across (no interface, bridge, veth, connected socket or fd);
     4. mount the **overlay** with a **lower layer only** = the job's own root filesystem;
     5. **bind-mount the writable tree** over the overlay (for the probe: an empty directory of the collector's own making);
     6. mount the private temporary directory as **`tmpfs`** the same way;
     7. **`pivot_root`** the child into the result;
     8. arrange `.spine/cache/` absence — mask the subpath, or keep the result directory outside the mounted root;
     9. hold stdout/stderr as **pipes on the host side**.
     - Creation failure at any sub-step ⇒ **disposition 2** exactly as 6.2.
   - **6.4** Run **P1, P2, P3, P4** inside the probe boundary (details below). All four must pass.
   - **6.5** Reap the probe, tear down the probe boundary, remove the canary. **No probe artifact survives.** (R46)
   - **6.6** Verdict (verbatim block in *Byte-level fixities*): all four passed ⇒ `profile := container`; otherwise ⇒ **disposition 2**: `profile := none`, stderr diagnostic naming **which of P1–P4** failed, run the suite unisolated. (R55–R58)
7. Check out `B`.
   - **7.a** **Run the restore phase for `B`**: read `origin/<trunk>:.spine/restore.sh`; if absent ⇒ empty phase + one stderr diagnostic. Otherwise `sh` those bytes at the root of the `B` checkout, in the **restore disposition**, bounded by `params.timeout`; on expiry kill the process group and reap; ignore the exit code. (R31–R38)
   - **7.b** For **every** runner in the invocation set, spawn the `B` **enumeration** (and, where the adapter has one, the separate `B` **outcome run**) under the **runner disposition** of the boundary of step 6 — **all of them, before any process has run against `T`'s content**.
8. Check out `T`, detached.
   - **8.a** **Run the restore phase for `T`** — the same trunk bytes, at the root of the `T` checkout, restore disposition, same deadline, exit code unread. This is *"the last phase of the run that holds the job's own network"*.
   - **8.b** For each runner, spawn it as a child under the **runner disposition** and read its stream over the pipe, enforcing the deadline.
9. Reap every process group.
10. Reduce, union, sort, fold statuses, write the file by temp-and-rename — the file carries the `profile=` decided at 6.6 and nothing about how it was decided.

### The four tests, expanded

**P1 — Containment.** By **absolute path**, the probe attempts, and each must fail:
- (a) read the canary;
- (b) write to the canary;
- (c) create a file at `.spine/cache/results/<T>.jsonl`;
- (d) remove the result directory
— *"at whichever path the collector is actually using it, since §3 admits two arrangements for `container`."*
Then, on the **host side, after the probe is reaped**, the collector re-reads the canary and requires its bytes **unchanged**.
Pass = all four attempts failed **and** the canary is byte-identical. **One success is a failed test.**

**P2 — Identity.** The probe reports its **real and effective uid and gid** and **creates a file in its writable tree**. The collector then **`stat`s that file from the host side**.
Pass = every reported id lies outside `{0, U, Ug}` **and** the created file's owner **as the host sees it** is neither `U` nor 0.
Note the asymmetry the spec insists on: *"The host's view decides: a uid the probe reports is a claim, and an identity mapping that does not reach the host is exactly the forgery a test trusting the report would miss."*

**P3 — Separation.** The probe **enumerates its own process table** and **`stat`s its root directory**.
Pass = the collector's own pid is **absent** from that table **and** the probe's root is a **different `(device, inode)` pair** from the collector's root.
This is the limb that forces the overlay (R20–R22): a bare mount namespace over the job's root yields the collector's own `(device, inode)` and would fail forever.

**P4 — Egress.** Two limbs, both required:
- **(a)** the probe **enumerates its own network interfaces and their addresses**; pass = the interface set is **exactly one device, loopback**, carrying no address other than `127.0.0.1/8` and `::1/128`;
- **(b)** the probe opens a TCP socket and attempts a **non-blocking `connect(2)`** to **`192.0.2.1:443`** (RFC 5737 TEST-NET-1), the collector bounding the attempt at **one second**; pass = the connect **fails**. A **completed** connect fails the test; a connect **still pending at the bound** also fails the test.

Why both: *"(a) is the evidence and needs no packet, but a collector could read it from the wrong namespace; (b) is answered by the kernel the child is actually in, but on a host with no default route it would pass without a namespace, and (a) catches that."*

---

## Byte-level fixities

**The verdict block** — RF §7.1, verbatim, fenced as printed:

```
step 1:  params.isolation = "uid"        ->  refuse: fail the job, write nothing   (disposition 1)
step 6:  params.isolation = "container"  ->  "container"  if P1, P2, P3 and P4 all passed
                                             "none"       otherwise                (disposition 2)
         params.isolation = "none"       ->  "none", and no boundary is attempted
```

**The profile table** — RF §7.1, verbatim:

| `profile=` | v1 mechanism | Where the result directory must be | What licenses writing it |
|---|---|---|---|
| `container` | **M1** — the only mechanism v1 ships | outside the boundary's filesystem view, on the host side (§3) | **P1 ∧ P2 ∧ P3 ∧ P4**, all four passed at step 6 |
| `uid` | **none in v1** — the request is a **refusal** (disposition 1 below), never a downgrade | mode `0700` under the collector's own uid (§3) | nothing in v1 can license it, so no v1 collector ever writes it |
| `none` | **no boundary is attempted** | anywhere in the job; any process in it can write the file (§3, §9) | nothing to test — `none` asserts the *absence* of a boundary, and an absence needs no evidence |

**The header field's position and separators** — RF §4.2, verbatim; `profile` is field **5 of 6**, `key=value`, separated by **exactly one `U+0020`**, in this order:

```
tree=<oid> base=<sha> tool=<version>+sha256:<hex64> keys_visible=<bool> profile=<profile> ids=<n>
```

Field-order violations, a repeated key, a missing key, an unknown key, an empty value, a value containing `U+0020`, or a value outside its grammar all **reject the file** (RF §4.2). `profile=n/a` in a **header** is **malformed** (RF §4.2).

**The published header line of the §10 worked example** (`params.isolation: container`, `params.timeout: 1800`, `object_format: sha1`, `cli.version 1.4.0`), verbatim:

```
tree=3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28 base=7b0d4a1f2c3e5d6a8b9c0d1e2f3a4b5c6d7e8f90 tool=1.4.0+sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db keys_visible=false profile=container ids=7
```

**The result path**, RF §3:

```
.spine/cache/results/<T>.jsonl
```
`<T>` **lowercase hex, full length, never abbreviated** — 40 characters under `object_format: sha1`, 64 under `sha256`. Extension exactly `.jsonl`. Stem carries no prefix, suffix, branch name or intent id, and equals the header's `tree=` **byte for byte**.

**The restore script's address**, RF §7.1 / §11 item 16a / CI §5.6, verbatim:

```
origin/<trunk>:.spine/restore.sh
```

**P4's fixed target and bound**, RF §7.1, verbatim: *"attempts a **non-blocking** `connect(2)` to `192.0.2.1:443` — RFC 5737 TEST-NET-1, an address that is unroutable on the public internet by definition, so no packet the limb emits can ever reach a third party — and the collector bounds the attempt at **one second**."*

**P4 limb (a)'s address set**, verbatim: *"the interface set is **exactly one device, loopback**, carrying no address other than `127.0.0.1/8` and `::1/128`"*.

**P2's exclusion set**, verbatim: *"The reported ids all lie outside `{0, U, Ug}`, **and** the created file's owner as *the host* sees it is neither `U` nor 0."*

**Canary creation flags**, verbatim: *"writes a **canary** into it, `O_CREAT|O_EXCL`, under a name no other process can predict, holding its bytes in memory"*.

**Result-file publication**, RF §3, verbatim: *"writes to a temporary file in the same directory opened `O_CREAT|O_EXCL` under a name no other process can predict, `fsync`s it, and `rename()`s it over `<T>.jsonl`, replacing any file already there."*

**Mount sequence for M1's root**, RF §7.1, verbatim: *"the writable tree is bind-mounted over the overlay afterwards, the private temporary directory is a `tmpfs` mounted the same way, and the child is `pivot_root`ed into the result."*

**`.spine/ci.sh` shell fixities that M1 depends on** (CI §5.3, verbatim from the printed script):

```sh
umask 022
```
```sh
SPINE_ALLOWED_HOSTS='pypi.org files.pythonhosted.org registry.npmjs.org pub.dev'
export SPINE_ALLOWED_HOSTS
```
```sh
[ -f "$TOP/$RESULT" ] || die 2 "the collector wrote no result file at $RESULT"
printf 'result=%s\n' "$RESULT"
[ "$COLLECTOR_RC" -eq 0 ] || exit 1
exit 0
```
`die` prints, verbatim: `printf 'spine/ci.sh: %s\n' "$*" >&2` — so the disposition-1 refusal surfaces at the shell as, on **stderr**:

```
spine/ci.sh: the collector wrote no result file at .spine/cache/results/<T>.jsonl
```

with **exit 2**. Published digests for exactly those `ci.sh` bytes (with `@@DIST_BASE@@` unsubstituted): **319 lines**, `git hash-object` **`131f13fb0312162579605999d3f9f4e90098c74c`**, SHA-256 **`d6bcf50cf675614033aaef61df104aad253d30c4accc756719599ad5bd41060b`** (CI §5.3; README digest table).

**Diagnostic channel**, CI §5.1, verbatim: *"Every diagnostic, and all of the collector's own output, goes to stderr."* stdout on `collect` carries exactly one line: `result=<repo-relative path>`.

**Canonical JSON note** (relevant only in that the restore phase and the probe write **nothing** into the body): body lines are canonical JSON per RF §4.3 — no whitespace outside strings, members ordered by key ascending over UTF-16 code units, lowercase `\u00xx` escapes, string values only.

---

## Error cases

| # | Condition | Behaviour | Exit code / status token / message |
|---|---|---|---|
| E1 | `params.isolation == "uid"` under `--ci` | **REFUSE at step 1**: fail the job, write **no** result file. Never a downgrade. (RF §7.1 disposition 1) | `ci.sh` finds no file ⇒ **exit 2**, stderr `spine/ci.sh: the collector wrote no result file at .spine/cache/results/<T>.jsonl` (CI §5.2, §5.3) |
| E2 | `params.isolation == "uid"` **outside** `--ci` | **No refusal.** Solo collector attempts nothing and writes `profile=none`. (RF §7.4) | header `profile=none` |
| E3 | Any of M1's five host prerequisites absent (Darwin runner; kernel/seccomp refusing `unshare`; no delegated `subuid` range and not root; no overlay; filesystem the mapped id cannot traverse; no network namespace or no loopback) | **Disposition 2**, *not* a refusal, *not* a failed test: tear down, `profile=none`, run the suite unisolated, run proceeds (RF §7.1) | header `profile=none`; **stderr diagnostic naming which prerequisite**; downstream: precondition 1 `unmet`, `class=tripwire` **`G11`** wire with mandatory `reason=` |
| E4 | Boundary creation failed mid-way | Same as E3 — disposition 2 (RF §7.1 disposition 2) | `profile=none` + stderr diagnostic |
| E5 | Any of P1, P2, P3, P4 failed (including 3-of-4) | **Disposition 2**: tear down, **run no runner inside it**, `profile=none`, proceed (RF §7.1) | header `profile=none`; **stderr diagnostic naming which of P1–P4** failed; `G11` tripwire downstream |
| E6 | P1 limb succeeded (any of read/write/create/remove) **or** canary bytes changed | P1 **fails** ⇒ E5 | as E5 |
| E7 | P4 (b) connect **completes** | P4 **fails** ⇒ E5 (RF §7.1: *"A connect that completes is a failed test"*) | as E5 |
| E8 | P4 (b) connect **still pending** at the one-second bound | P4 **fails** ⇒ E5 (*"pending means a route existed"*) | as E5 |
| E9 | P4 (a) interface set is anything other than exactly loopback with only `127.0.0.1/8` and `::1/128` | P4 **fails** ⇒ E5 | as E5 |
| E10 | Header `profile=` ≠ trunk's `params.isolation`, **or** `params.isolation` is not `container` | **Auto-merge precondition 1 unmet.** *Not* a refusal, *not* an ingestion failure. (RF §8.4; PB §7.4 rule 5) | `C-M4` evaluates `off`; **one** `class=tripwire` **`G11`** wire, `reason=` mandatory; GR `automerge.preconditions[1].status = "unmet"` |
| E11 | `.spine/restore.sh` absent on trunk | Phase is **empty**, no process runs; **one stderr diagnostic**; not a prerequisite failure, not a failure, not a downgrade (RF §7.1; §11 item 16a) | run proceeds; runners still loopback-only |
| E12 | Restore phase exits non-zero | **Nothing reads the exit code.** Diagnostic on stderr; run proceeds. *"a restore that failed reappears as the suite that fails without it"* (RF §7.1) | no `status` token; `end.status` unmoved |
| E13 | Restore phase exceeds `params.timeout` | Kill its process group, reap it, **contribute no `status`**, run proceeds (RF §7.1 *The deadline*, *The restore phase*) | no `status` token |
| E14 | `params.timeout` present and not a strictly positive integer | Fail the job, write nothing (step 1's shape) (RF §7.1) | `ci.sh` ⇒ **exit 2** |
| E15 | Deadline expires during a `B` **enumeration** | That enumeration failed ⇒ fold yields `base-collect-failed`, `ids=0`, **no `base`/`result` records at all from any runner** (RF §7.1, §7.3) | `status` token **`base-collect-failed`** |
| E16 | Deadline expires during the separate `B` **outcome run** | Not a status at all; every id not reached takes `out: "absent"`; `end.status` unaffected (RF §7.1, §7.3) | no status token |
| E17 | Deadline expires on the `T` run | Kill that process group; that runner contributes `runner-timeout` (RF §7.3) | `status` token **`runner-timeout`** |
| E18 | Collector writes `container` without having run P1–P4 / answers `uid` with a file / substitutes a mechanism / runs a runner in a failed boundary | **Non-conformant implementation** (RF §11 item 16) | — |
| E19 | Collector spawns a runner with egress / takes the restore script from a checkout / lets the restore phase contribute to the file / treats a missing restore script as a prerequisite failure | **Non-conformant implementation** (RF §11 item 16a) | — |
| E20 | `params.isolation == "uid"` in a **landing's** manifest at `T` | MF §6.2 **check 12b** fails the landing **outright** | status token **`isolation-unsupported`** |
| E21 | `spine init --isolation uid` | Refused — the CLI grammar admits only `container|none` (PB §11) | — |
| E22 | A landing ran gates but ingested no file (`result-missing` / `result-malformed`, bypassed under §8.7) | Seals **`profile=none`**, `evidence` absent; preconditions 1 and 2 `"unmet"` (RF §8.4; GR §5.9) | seal `profile=none` |
| E23 | A file was ingested but its trunk-defined origin could not be established | Seal carries **the header's own `profile=` unaltered** — `container`, `uid` or `none`. **`n/a` refused. `none` refused** as a substitute. Precondition 2 `"unmet"` carries the doubt. (RF §8.4) | seal `profile=<header value>`; GR `automerge.preconditions[2].status = "unmet"` |
| E24 | A tombstone (changes no tree, runs no suite) | Exempt from rule 5 entirely | seal **`profile=n/a`** (PB §7.4 rule 5, PB §11) |

---

## Worked examples / test vectors

**V1 — the §10 happy path (published).** `billingsvc`, `params.ci: github`, `params.langs: ["python","ts"]`, **`params.isolation: container`**, `params.timeout: 1800`, `object_format: sha1`, `cli.version 1.4.0`. Result file `.spine/cache/results/3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28.jsonl`, **20 lines**, header line verbatim in *Byte-level fixities*. Reading, verbatim from RF §10:

> `profile=container` equals `params.isolation` → precondition 1 holds. `keys_visible=false`, a matching `tool=`, **and trunk-defined origin evidence** — the trusted job ran on a `workflow_run` of `.github/workflows/spine-collect.yml` in this repository (`ci.md` §14 R11) — → all three conjuncts hold, so precondition 2 holds (§8.4). Precondition 0 fails anyway, because `C-A3` is `hostile`…

> The landing seals `profile=container threat=hostile`, so the ledger records forever how strong the evidence was.

Corresponding seal fragment (PB §5.5, line 471, verbatim):

```
Spine-Seal: INT-042 base=7b0d… head=77aa… tree=… report=sha256:… tool=1.4.0+sha256:… git=2.45 mode=team threat=hostile profile=container envelope=sha256:… signer=ci@example.com
```

**V2 — disposition 2 by prerequisite (Darwin).** `params.isolation: container` on a `Darwin` runner (`ci.md` §5.5 ships `aarch64-apple-darwin` / `x86_64-apple-darwin`). Prerequisite 1 absent ⇒ tear down, `profile=none`, stderr names prerequisite 1, suite runs unisolated, file is written. Trusted stage: precondition 1 unmet (`container` vs `none`) ⇒ `C-M4` off + one `class=tripwire` `G11` wire with mandatory `reason=`; seal carries `profile=none` forever. (RF §7.1)

**V3 — disposition 2 by test.** `params.isolation: container`, all five prerequisites present, boundary built, but P4 (b)'s connect to `192.0.2.1:443` is still pending at one second. ⇒ P4 fails ⇒ `profile=none`, stderr names **P4**, **no runner is spawned inside that boundary**, run proceeds. Same downstream as V2, and the stderr diagnostic is what tells the human this was a *failed test* rather than a *missing prerequisite*.

**V4 — disposition 1.** `params.isolation: uid` under `--ci`. Refusal at step 1: no `T` is even computed for the purpose, no file exists. `ci.sh` exits **2** with `spine/ci.sh: the collector wrote no result file at .spine/cache/results/<T>.jsonl`. Note the residual: MF check 12b would normally have kept `uid` off trunk (`isolation-unsupported`), and `spine init` refuses to write it.

**V5 — solo.** `--collect` outside `--ci`, manifest says `container` (or `uid`). No boundary attempted, nothing refused, `keys_visible=true`, `profile=none`. Restore phase **still runs** — same trunk bytes, same order, same deadline, no boundary around it — unless there is no `origin/<trunk>` remote-tracking ref, in which case the phase is empty. Preconditions 1 and 2 fail by construction. (RF §7.4, §7.1)

**V6 — restore phase absent.** Trunk has no `.spine/restore.sh`. Both restore phases are empty; one stderr diagnostic each says so; every runner still runs loopback-only; `profile=container` is still reachable. (RF §7.1; §11 item 16a; CI §5.6)

**Published digests touching this concern** (README digest table): `ci.md` §5.3 `.spine/ci.sh` — **319 lines**, `git hash-object` `131f13fb0312162579605999d3f9f4e90098c74c`, `sha256:d6bcf50c…`; *"the process-wide `umask 077` was narrowed to `umask 022` plus an explicit `chmod 0700 "$WORK"` and `0755` on `$INSTALL_DIR` and `$BIN`, because at 077 nothing the collector writes is reachable to the mapped id `result-file.md` §7.1's M1 spawns runners under, so `profile=container` was unlicensable on every host (+12 lines)."*

---

## Cross-references it depends on

| Owned elsewhere | What it owns | Where |
|---|---|---|
| **Result-file grammar sheet** | The six header fields, canonical JSON, record kinds (`base`/`result`/`end`), `out`, §4.5 sort, `ids=` | RF §4.1–§4.5 |
| **Status/fold sheet** | The `status` vocabulary and the fold — `complete`, `base-collect-failed`, `spawn-failed`, `no-output`, `stream-invalid`, `runner-failed`, `runner-timeout`; the all-or-nothing `B` rule | RF §7.3 |
| **Deadline sheet** | `params.timeout`'s full semantics, per-invocation budget, worst-case arithmetic. This sheet restates only that the **two restore phases** are inside it | RF §7.1 *The deadline*; §11 item 10 |
| **Ingestion sheet** | §8's ordered checks, `base-moved`, `result-missing`, `result-malformed`, G15, the undeclared-runner check, §8.5's clause 2 carve-out | RF §8.1–§8.7 |
| **Auto-merge / preconditions sheet** | Preconditions 0–4 in full; `G11` wire keying `(G11, pathless)`; one wire however many conjuncts failed | RF §8.4; PB §7.4 rule 5; GR §5.8, §9.22 |
| **Gate-report sheet** | The `profile` member (four values, `R`), `evidence` object, `automerge.preconditions[n].status` | GR §5 member table, §5.8, §5.9 |
| **Manifest sheet** | `params.isolation` type/domain/default/frozen-ness; §6.2 **check 12b** (`isolation-unsupported`); `params.langs` monotonicity (check 12, `langs-shrank`) | MF §3.3, §3.8, §6.2 |
| **CI sheet** | `.spine/ci.sh` bytes and digests, exit codes 0/1/2, the umask narrowing, `SPINE_ALLOWED_HOSTS` / `SPINE_REGISTRY_PROXY` and the client variables, the platform table, U1–U8 | CI §5.1–§5.6, §6.1 |
| **Import-resolver sheet** | `import-resolver.md` §11.1 — which adapters need a **separate `B` outcome run** (hence how many invocations the runner disposition is entered for) | IR §11.1 |
| **Seal / envelope sheet** | `Spine-Seal`'s `profile=` field and its four-value domain | PB §11; EV |

---

## OPEN items

Isolation-specific OPENs are **closed**; the ones below are recorded because a reader of this concern will meet them.

1. **RF OPEN-9 — CLOSED (2026-08-27).** *"Filed as `SPINE_ALLOWED_HOSTS` has a declarer and no enforcer; raised again as finding 2 of the Codex final corpus review, 2026-08-27."* Closed by M1 gaining a network namespace, loopback-only runners, the restore phase, and P4. *"What remains open here is nothing; what remains **declared rather than enforced** is named in §12 and is the host's socket filter, not a gap in this document."* (RF §14 OPEN-9; §13 R34)
2. **Declared-not-enforced, permanently and by design (not an OPEN, but the live residual):** the *which hosts* half of `SPINE_ALLOWED_HOSTS`. M1 enforces *when* (loopback-only runners, P4-tested); narrowing the restore phase to the declared host list *"is still a network policy, a proxy sidecar or an egress firewall the host supplies"*. **No gate, precondition or header field reads egress; what reads it is P4, and its whole output is `profile=`.** (RF §12; CI §5.6)
3. **Out of scope, deliberately — do not invent:** *"The particular system call, helper or runtime that creates M1's namespaces. `unshare(2)` used directly, a rootless OCI runtime, and a sandbox helper are all conforming if the probe of §7.1 passes P1-P4 under them, and none is named here."* An implementer may pick freely; the choice must not become part of the file's meaning. (RF §12)
4. **Out of scope:** *"What a restore script contains, per language."* No v1 template writes one, no `files[]` record names one. (RF §12)
5. **Out of scope:** container images, registries and image policy — *"the answer is that the question does not arise."* (RF §12; CI §17)
6. **RF OPEN-7 — genuinely OPEN, adjacent.** *"is `params.ci` monotone in the guarantee it names?"* It does not touch the boundary, but it moves precondition 2 permanently, so a reader of `profile=` in the seal must read `automerge.preconditions[2].status` beside it. Filed three times — `ci.md` OPEN-3, `result-file.md` OPEN-7, `manifest.md` OPEN-1 — and *"`manifest.md` is the one that owns G16 and would carry the fix."* (RF §14 OPEN-7; README *Known gaps*)
7. **RF OPEN-5 — genuinely OPEN, adjacent.** G6's reporting channel. Does not touch the boundary; noted because mutation runs would also be spawned under the runner disposition if it ever ships. (RF §14 OPEN-5)
8. **`ci.md` OPEN-4 — no Windows CI target in v1.** M1 is Linux-only anyway (prerequisite 1); Darwin is disposition 2. (CI §5.5)
9. **The owner-confirmed cost, settled not open:** *"The container prerequisite stack stays at five — user namespaces, an identity source, an overlay root, a traversable filesystem, a network namespace — so `profile=container` asserts a real boundary or is not claimed, and every absence is a stated disposition rather than a silent downgrade. Hosted GitHub and GitLab runners meet all five; on anything more restricted spine-kit runs with a human on every landing."* (PB §12, *Three costs the owner confirmed*)

---

## Contradictions found

**C1 — PB §12 change log says "four host prerequisites"; PB §12 (same section, later) and RF §7.1 say five.**
PB line 1117: *"The boundary is now a read-only overlay pivoted into, with identity from a delegated subordinate range or a root collector dropping privilege, and **four** host prerequisites whose absence is a stated disposition rather than an assumption."*
PB line ~1129: *"**The container prerequisite stack stays at five** — user namespaces, an identity source, an overlay root, a traversable filesystem, a network namespace."*
RF §7.1 prints a **five**-row prerequisite table and says *"M1 requires, all five"* and *"any of M1's five host prerequisites absent"*.
**Resolution:** **five.** The "four" sentence is a stale count from the pre-2026-08-27 fix pass, before the network namespace became prerequisite 5. It is prose in PB's change log, not §11 Vocabulary, so RF §7.1 is normative (RF §1). Implement five.

**C2 — PB §7.4 rule 3's `container` bullet says "a container the collector created"; RF says namespaces over the job's own filesystem, no container runtime and no image.**
PB: *"`profile=container` — the runner ran inside a container the collector created; the result directory is outside it and unmounted, and the stream crosses on a pipe the collector holds."*
RF §7.1: *"No image is pulled and none is named. The boundary is made out of the filesystem the job already has"*; RF §12 makes the runtime choice explicitly unspecified.
**Resolution:** not a genuine conflict of requirement — PB's *"container"* is the profile **name**, and PB's own following paragraph defers the mechanism to RF (*"`docs/spec/result-file.md` §7.1 owns it: `container` is namespaces over the job's own filesystem with no image pulled and none named"*). But an implementer reading only PB §7.4 rule 3's bullet list would reach for an OCI runtime. **Follow RF §7.1 + RF §12.** Flagged because it is exactly the shape of misreading RF §13 R33 was written against.

**C3 — RF §7.1 quotes `ci.md` §5.2's exit-2 row as a message; `ci.sh` emits a different string.**
RF §7.1 disposition 1: *"no file at the expected path is `die 2`, exit 2, *refused: nothing ran and no result file exists* (`ci.md` §5.2)."*
`ci.md` §5.2's table cell reads *"Refused. Nothing ran and **no result file exists**."* — a **meaning**, not a message. The **actual** emitted bytes are `die 2 "the collector wrote no result file at $RESULT"` rendered by `printf 'spine/ci.sh: %s\n' "$*" >&2` (CI §5.3).
**Resolution:** the exit code **2** is normative and agreed; the italicised phrase in RF is a gloss of the exit-code table, **not** a string to emit. Emit `ci.sh`'s own bytes. Report as a citation defect in RF §7.1.

**C4 — RF §7.1 prerequisite 4 attributes the umask fix to `ci.md` §5.4 item 1; the number of directories differs in the two renderings.**
RF §7.1 prerequisite 4: *"`umask 022` plus an explicit `chmod 0700 "$WORK"` and `0755` on the install directory and the binary"*.
CI §5.4 item 1: *"`$WORK` … is `chmod 0700` explicitly, and `$INSTALL_DIR` and the verified binary are `0755`."*
**Resolution:** identical content, differently worded. **No conflict**; recorded so an implementer does not go looking for a third mode.

**C5 — RF §3 assigns `uid` a result-directory location; RF §7.1 says no v1 collector ever reaches it.**
RF §3: *"Under `profile=uid` it is mode `0700` under the collector's own uid, inside the collector's own working copy of `B` — never inside the detached checkout of `T`."*
RF §7.1 / §4.2 / §11 item 16: `uid` is written by no v1 collector; the request is a refusal.
**Resolution:** both stand — §3 describes the reserved shape for a future release; **v1 implements the refusal and never the directory rule**. Not a defect; recorded because §3 read alone implies a code path that must not exist in v1.

**C6 — PB §7.1's stage table grants the untrusted stage a "sandbox" that strips key material, while RF §7.1 says the boundary is *not* the key-visibility control.**
PB §7.1: *"the sandbox strips `SSH_AUTH_SOCK`, `GPG_AGENT_INFO`, `~/.ssh`, `~/.gnupg`"*.
RF §7.1 *What the boundary is not*: *"It is not the key-visibility control: `keys_visible` is a predicate over the collector's own environment *and* every runner invocation's (§4.2), so hiding key material from a contained runner does not make it `false` where the collector itself could reach it, and the two header fields are independent."*
**Resolution:** `keys_visible` and `profile` are **independent header fields** (RF §4.2, §7.1). M1 stripping key material from a *runner* does **not** make `keys_visible=false` if the *collector* could reach it. PB's "sandbox strips…" describes the agent stages, and RF's is the collector's predicate. Implement RF's: one predicate over the collector's environment **and** every runner invocation's, `true` if either could reach key material.

**C7 — PB §7.1's stage table (pre-v0.19 wording) promised an allow-listed registry proxy "verified against the lockfile's hashes"; the shipped rule is narrower.**
Recorded by RF §13 R34 and RF §7.1 as a **withdrawn** reading: *"(Until 2026-08-27 it granted "an allow-listed registry proxy during dependency restore, verified against the lockfile's hashes, then none", which promised a filter no v1 component applied; it is narrowed to what this section enforces.)"*
PB v0.19's live text now says spine enforces *"the **then none**, and says so rather than claiming the rest"*, with hash-verification attributed to *"the package manager's, over a lockfile `C-T2` freezes"*.
**Resolution:** live text and RF agree. Flagged only so that an implementer meeting a **pre-v0.19 clone** does not build a hostname filter or a lockfile hash checker into the collector — RF §12 forbids both by name.

---

### One-line summary for the implementer

`profile=` is a **finding, never a request, and never upgraded silently**: v1 ships exactly one mechanism (M1 — mount/PID/IPC/network/user namespaces, lower-only overlay + bind-mounted writable tree + tmpfs scratch + `pivot_root`, identity from a delegated `subuid` range or a privilege-dropping root collector), it is licensed only by **P1 ∧ P2 ∧ P3 ∧ P4** run against a probe built from the **runner disposition**, `uid` is a **step-1 refusal that writes nothing**, everything else is **`profile=none` with a loud stderr diagnostic and a `G11` tripwire**, and the **only** phase with a route off the host is the twice-per-run restore phase reading `origin/<trunk>:.spine/restore.sh`, which contributes nothing to the file and whose exit code nobody reads.
