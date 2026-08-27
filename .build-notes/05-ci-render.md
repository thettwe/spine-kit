# 05 · CI render: the four templates, the release manifest, `.spine/ci.sh`, the artifact list

Concern: what `spine init` renders for CI, where the rendered constants come from, the bytes of
`.spine/ci.sh`, the release artifact list and platform table, the registry allowlist, the two-job
contract, and the three provider renderings (GitHub collect + land, GitLab, generic).

Citation convention used throughout: `(CI §x.y)` = `docs/spec/ci.md`; `(PB §x)` = `PLAYBOOK.md` v0.19;
`(MF §x)` = `docs/spec/manifest.md`; `(RF §x)` = `docs/spec/result-file.md`;
`(GR §x)` = `docs/spec/gate-report.md`; `(README)` = `docs/spec/README.md`.

**Precedence rule applied here (CI §1):** *"Where this document and PB §11 disagree, §11 wins and the
disagreement is a defect in one of them — reported in §15, never resolved silently."*

---

## Sources read

| File | Lines | What |
|---|---|---|
| `docs/spec/ci.md` | 1–140 | header, §1 Scope, §2 Position, §3.1 paths/manifest rows, §3.2 two GitHub files, §3.3 rendered constants, §3.4 release manifest (schema, consumption order, byte scan), §4 environment variables |
| `docs/spec/ci.md` | 137–300 | §5 `.spine/ci.sh`, §5.1 invocation contract, §5.2 exit codes, §5.3 script (first half) |
| `docs/spec/ci.md` | 300–520 | §5.3 script (second half) + published digests + verification record, §5.4 order of operations, §5.5 opening |
| `docs/spec/ci.md` | 520–660 | §5.5 artifact list location/bytes/platform table, §5.6 registry allowlist, §6.1–§6.6 two-job contract, handoff, ref→`--land` mapping, gate-report publication, `--land` exit codes, §7.1 |
| `docs/spec/ci.md` | 660–930 | §7.2 `spine-collect.yml` in full, §7.3 `spine-land.yml` in full, §7.4 environment, §7.5 merge queue |
| `docs/spec/ci.md` | 930–1225 | §8.1–§8.4 GitLab (three files in full + discovery), §9.1–§9.4 generic contract (Job A, Job B), §10.1 |
| `docs/spec/ci.md` | 1240–1345 | §10.2, §10.3, §11 configurations (a)/(b), §12 worked example, §13 determinism and conformance |
| `docs/spec/ci.md` | 1346–1476 | §14 R1–R28, §15 D1–D25, §16, §17 out of scope, §18 OPEN-1…OPEN-7 |
| `PLAYBOOK.md` | 848–885 | §7.4 rules 0–5 (verbatim) |
| `PLAYBOOK.md` | 1012–1036 | §11 Files and refs, wire aggregation, subject lines, CLI, git requirements |
| `PLAYBOOK.md` | 717–745, 795–799, 839–841 | §6.7 manifest example + owner classes, §7.1 least-privilege table, §7.3 floor globs |
| `docs/spec/manifest.md` | 50, 94–107, 132–147, 215–245, 1130–1151, 1265–1275, 1481–1495 | member-name grammar, `json_one` dependency, `cli` + artifact list, twelve `templates` keys, §8.2 published artifact list + `dist_hash`, §8.6 A/B manifests, C10/C11 closures |
| `docs/spec/gate-report.md` | 188–225 | §4.4–§4.4.2 note ref, annotated object, note content, write commands, when, failure, republication, concurrency |
| `docs/spec/README.md` | 1–88 | per-spec status, six settled owner decisions, published-digest table (the `ci.md` §5.3 row) |

---

## Data model

### DM-1 · `release/release.json` — the release manifest (CI §3.4)

Location: **`release/release.json`, at the root of spine-kit's own source tree** (CI §3.4). It is a
**build input**: read once when the binary is built, frozen into it, never consulted again. It is not
written into an adopting repository; no `files[]` record names it; no owner class applies; it is on no
floor; no gate reads it; nothing re-reads it from disk at run time (CI §3.4).

Format: **UTF-8 JSON, read by an ordinary parser. No canonical form is required and none is defined**
(CI §3.4) — nothing digests this file.

| Member | Type | Domain | Required | Default |
|---|---|---|---|---|
| `release_manifest_version` | integer | `1` | yes | none — absence is a refusal |
| `version` | string | `^[0-9A-Za-z._+-]{1,64}$`, **never the four bytes `none`** (MF §3.2; also CI §5.5's `<version>` production) | yes | none |
| `dist_base` | string | scheme `https://`; **no userinfo, no query, no fragment; no trailing `/`** | yes | none |
| `actions` | object | **exactly three members**: `checkout`, `upload_artifact`, `download_artifact` | yes | none |
| `actions.<k>.commit` | string | **exactly 40 lowercase hex digits — a full commit id, never a tag and never an abbreviation** | yes (each) | none |
| `actions.<k>.repo` | string | `actions/checkout`, `actions/upload-artifact`, `actions/download-artifact`, **one per key and fixed** | yes (each) | none |

- **Every member required, nothing else permitted. An unknown member is a refusal, not opaque data**
  (CI §3.4) — the opposite of `.spine/manifest.json`'s forward-compatible rule (PB §6.7, MF §3.9).
- `version` is the string `init` writes as `cli.version`, and the one the artifact names in CI §5.5's
  list carry (CI §3.4, MF §3.2).
- **Not members, and why** (CI §3.4): `dist_hash` (substituted into no template; written into
  `.spine/manifest.json` as `cli.dist_hash`; it is the digest of the §5.5 artifact list, fixed only once
  every artifact is built); the twelve template names/versions; the shipped floor `F0` (MF §5.5); the
  twelve constitution rules (CN §6.2); `SPINE_ALLOWED_HOSTS` (CI §5.6). Those are **source constants**.

### DM-2 · The CI substitution table (CI §3.3, §3.4 step 2)

Exactly these rows and no others:

| Token (literal bytes in the template) | Substituted with |
|---|---|
| `@@DIST_BASE@@` | `dist_base` — the release's distribution root, an `https://` URL (CI §5.5) |
| `PIN_CHECKOUT` | `actions.checkout.commit` (full 40-hex commit id) |
| `PIN_UPLOAD_ARTIFACT` | `actions.upload_artifact.commit` |
| `PIN_DOWNLOAD_ARTIFACT` | `actions.download_artifact.commit` |
| `main` in every CI §7/§8/§9 example | trunk's name, from `params.trunk` |

What the other eight templates render from — `params.langs` into `C-T1`/`C-T2` (CN §6.4), an intent's
fields (TM), the keyring's principal — **is theirs and is untouched here** (CI §3.4 step 2).

### DM-3 · Paths, templates and owner classes `init` writes (CI §3.1)

| `params.ci` | Path written | Template | Owner |
|---|---|---|---|
| every value | `.spine/ci.sh` | `ci-generic@N` | `spine-owned` |
| `github` | `.github/workflows/spine-collect.yml` | `ci-github-collect@N` | `spine-owned` |
| `github` | `.github/workflows/spine-land.yml` | `ci-github-land@N` | `spine-owned` |
| `gitlab` | `.gitlab-ci.yml` | `ci-gitlab@N` | `spine-owned` |
| `gitlab` | `.spine/gitlab/untrusted.yml` | `ci-gitlab@N` | `spine-owned` |
| `gitlab` | `.spine/gitlab/trusted.yml` | `ci-gitlab@N` | `spine-owned` |
| `generic` | *(nothing beyond `.spine/ci.sh`)* | — | — |

- **Three GitLab paths share one template name** `ci-gitlab@N`; G16 check 7 joins three `files[]`
  records to that one `templates` key (CI §3.1, MF §3.6).
- `.spine/restore.sh` is **not** in this table: `spine init` writes no such file, no template renders
  one, the manifest carries no `files[]` record for it (MF §6.2 requires none), it is optional, and where
  trunk has no such file the restore phase is empty (CI §3.1, §5.6).
- **`ci-generic` names the provider-independent shell, not the `generic` provider** (CI §3.1; PB §6.7's
  own example gives `.spine/ci.sh` the template `ci-generic@4` in a `params.ci: "github"` repository).
- `templates` in the manifest carries **twelve** keys — *"agents-block · ci-generic · ci-github-collect ·
  ci-github-land · ci-gitlab · constitution · gitattributes · gitignore · intent · intent-bug ·
  intent-change · keyring"* — one per template the pinned release ships, whether or not this repository
  holds a rendered instance (CI §3.1, MF §3.6, PB §6.7).
- Owner class over time: `.spine/ci.sh` and the GitLab job files are `spine-owned`; **the provider's
  workflow file starts `spine-owned` and becomes `user-modified` on `--adopt` or a successful `--merge`**
  (CI §2).
- Floor: **every path in §3.1 is on the protected floor** — `.spine/**` and the CI-definition globs of
  PB §7.3. Changing any of them takes a `class=protected` review by G14 and is evaluated **under the old
  policy** (CI §2, PB §7.4 rule 1).

### DM-4 · The release artifact list (CI §5.5)

Location, content-addressed, for a pinned `cli.dist_hash` of `sha256:<H>`:

```
<SPINE_DIST_BASE>/<H>/artifacts.txt      the list
<SPINE_DIST_BASE>/<H>/<artifact-name>    every artifact the list names
```

| Property | Value (verbatim from CI §5.5) |
|---|---|
| Encoding | *"UTF-8, LF-terminated, every line terminated including the last, no CR anywhere, no BOM, no blank lines, no comments, no header."* |
| Line form | *"Each line is `<64 lowercase hex>` `U+0020` `U+0020` `<artifact name>`. Two spaces, the format `sha256sum` writes and `sha256sum -c` reads."* |
| Artifact names | *"`spine-<version>-<target>.tar.gz` for platform artifacts and `spine-<version>-py3-none-any.whl` for the wheel. `<version>` is `[0-9A-Za-z._+-]+`."* |
| Ordering | *"**Lines sorted ascending by the bytes of the artifact name.** Two builds of one release produce one list, byte for byte, or `dist_hash` is not a pin."* |
| Digest | *"`dist_hash` is the SHA-256 of exactly these bytes."* |

`dist_hash` is written into `.spine/manifest.json` as `cli.dist_hash` = `"sha256:"` + 64 lowercase hex
(MF §3.2). `cli.version` = the release manifest's `version` (MF §3.2, CI §3.4).

**Exactly one artifact per target** (CI §5.5): `ci.sh` refuses a list with none (*"the release does not
build for this runner"*) and refuses a list with two (*"a release whose own list is ambiguous is not a
pin"*). *"The binary independently verifies its own bytes against the same list at start-up (PB §6.7), so
the check is made twice by two different parties."* G15's whole test is membership in this list, never a
comparison (MF §3.2, PB §6.3).

### DM-5 · Platform table (CI §5.5)

| `uname -s` | `uname -m` | target token |
|---|---|---|
| `Linux` | `x86_64`, `amd64` | `x86_64-unknown-linux-musl` |
| `Linux` | `aarch64`, `arm64` | `aarch64-unknown-linux-musl` |
| `Darwin` | `arm64` | `aarch64-apple-darwin` |
| `Darwin` | `x86_64` | `x86_64-apple-darwin` |
| anything else | | **refused, exit 2** |

**v1 ships no Windows CI target** (CI §5.5, §18 OPEN-4): `.tar.gz` is the only container,
`gzip -dc | tar -xf -` the only unpack.

### DM-6 · Normative environment variables (CI §4)

| Name | Kind | Job | Meaning |
|---|---|---|---|
| `SPINE_TRUST_ROOT` | **variable, not secret** | **both** | The trust-root commit sha (PB §7.5). `spine check --ci` refuses to run without one, so the untrusted job needs it too. **Never read from a tracked file.** |
| `SPINE_PIPELINE_KEY` | **secret** | trusted only | The OpenSSH **private key** of the `spine-seal@v1` principal, **as key material, not a path**. Read from the environment and never written to disk. |
| `SPINE_PUSH_TOKEN` | **secret** | trusted only | The credential the trusted job pushes trunk with — the bypass principal of configuration (a). **Never the provider's ambient job token.** |
| `SPINE_PUSH_KEY` | **secret** | trusted only | The SSH alternative to `SPINE_PUSH_TOKEN`. **Exactly one of the two is set; both set, or neither, is a refusal.** |
| `SPINE_DIST_BASE` | variable, not secret | both, **optional** | Overrides the distribution root the release manifest froze into `ci.sh`. *"it changes no hash that is checked."* |
| `SPINE_INSTALL_DIR` | convenience, not policy | both, optional | Where a verified binary is unpacked. |
| `SPINE_REGISTRY_PROXY` | convenience, not policy | untrusted, optional | Registry proxy root; must be `https://` (CI §5.6). |

**The seal principal is not a variable** (CI §4): the trusted stage reads `.spine/allowed_signers` at the
seal's `base=`, takes the principals listed under `spine-seal@v1`, and selects the one whose public key
corresponds to the private key in `SPINE_PIPELINE_KEY`. **Zero matches or several: refuse, exit 3.**

Exported by `ci.sh` itself (CI §5.3, §5.6): `LC_ALL=C`, `PATH` (sanitised), `SPINE_ALLOWED_HOSTS`, and —
only when `SPINE_REGISTRY_PROXY` is set — `PIP_INDEX_URL`, `NPM_CONFIG_REGISTRY`, `PUB_HOSTED_URL`.

### DM-7 · `ci.sh` internal values (CI §5.3)

| Name | Derivation |
|---|---|
| `MODE` | `$1`; domain `install` \| `collect` |
| `TRUNK` | `$2`; must pass `git check-ref-format --branch` |
| `CANDIDATE` | `$3`; `collect` only; must pass `git check-ref-format --branch` |
| `NL` | `"$(printf '\n_')"` with the trailing `_` stripped |
| `IFS` | `"$(printf ' \t\n_')"` with the trailing `_` stripped |
| `SPINE_DIST_BASE_DEFAULT` | render-time constant `'@@DIST_BASE@@'` |
| `DIST_BASE` | `${SPINE_DIST_BASE:-$SPINE_DIST_BASE_DEFAULT}`, then **exactly one trailing `/` appended if absent** |
| `MANIFEST_TRUNK` | `json_one trunk` over `origin/$TRUNK:.spine/manifest.json` |
| `DIST_HASH` | `json_one dist_hash`, `sha256:` prefix stripped, 64 lowercase hex enforced |
| `TARGET` | platform table DM-5 |
| `WORK` | `mktemp -d "${TMPDIR:-/tmp}/spine-ci.XXXXXX"`, then `chmod 0700` |
| `INSTALL_DIR` | `$SPINE_INSTALL_DIR` if set, else `mktemp -d "${TMPDIR:-/tmp}/spine-bin.XXXXXX"`; `chmod 0755` |
| `OWN_INSTALL_DIR` | set to `$INSTALL_DIR` **only when `MODE=collect` and `SPINE_INSTALL_DIR` was unset** (so `install`'s output survives the trap) |
| `BIN` | `$INSTALL_DIR/spine`, `chmod 0755` |
| `ART_SHA` / `ART_NAME` | the one matching line of `artifacts.txt` |
| `T` | `git merge-tree --write-tree refs/remotes/origin/$TRUNK refs/heads/$CANDIDATE` |
| `TOP` | `git rev-parse --show-toplevel` |
| `RESULT` | `.spine/cache/results/$T.jsonl` |
| `COLLECTOR_RC` | exit status of `spine check --ci --collect` — **captured, not propagated** |

### DM-8 · Exit codes

`.spine/ci.sh` — **three codes and no more** (CI §5.2):

| Exit | Meaning | The definition's obligation |
|---|---|---|
| 0 | `install` succeeded, or the collector ran and exited 0. | Continue. |
| 1 | The collector ran, exited non-zero, and **a result file exists**. | Hand the file over anyway, then fail the job. |
| 2 | Refused. Nothing ran and **no result file exists**. | Fail the job; there is nothing to hand over. |

`spine check --ci --land` (CI §6.6) — *"No definition may infer a landing's outcome from anything else."*

| Exit | Status token | Meaning | Obligation |
|---|---|---|---|
| 0 | `landed` | CAS succeeded and the note was published. | Succeed. |
| 1 | `blocked` | Gates produced a wire or a floor hit; a gate report exists. | Fail the job. **Do not re-queue.** |
| 2 | `base-moved` | Trunk or the branch moved; the record is void (PB §6.3 G11). | Re-queue the two-job run on the new tip, or leave it to the next scheduled run. |
| 3 | `refused` | A precondition of running at all: bad candidate ref, no trust root, no usable seal key, `needs-rebase`, version skew (G15). | Fail the job. **Never retry.** |
| 4 | `reconstruction-failed` | G9 or G10 refused the push at PB §5.4 step 5. **No retry and no `C-M3` consumption.** | Fail the job. **Never re-queue.** |
| 5 | `note-publish-failed` | The landing is complete; `refs/notes/spine` was not updated. | Fail the job so a human sees it. **Do not re-queue.** |

### DM-9 · Candidate ref → `--land` mapping (CI §6.4) — *"Normative, and the order is normative"*

| Candidate ref | Invocation |
|---|---|
| `quick/reseal-<O>` | `spine check --ci --land --reseal` |
| `intent/<ID>` | `spine check --ci --land <ID>` |
| `quick/<name>` | `spine check --ci --land --quick quick/<name>` |
| `spine/upgrade-<version>` | `spine check --ci --land` |
| anything else | **refuse, exit 3** |

---

## Normative requirements

MUST / MUST NOT / REFUSE (a defined refusal with an exit code or status token) / SHOULD.

### Render time — `spine init`

- **R1 · MUST.** Parse and validate the embedded release manifest against DM-1 **before any plan is
  computed** (CI §3.4 step 1).
- **R2 · REFUSE (`no-release-manifest`).** A build embedding a release manifest that satisfies DM-1 is a
  **release build**; *"anything else — no file, a file the schema refuses, an unknown
  `release_manifest_version` — is a **development build**"*. A development build *"renders no CI
  definition, writes no `.spine/manifest.json`, creates no path, and reports `REFUSE` for every row of the
  plan (PB §6.7's `create · update · delete · skip · REFUSE`) with the diagnostic `no-release-manifest`"*
  (CI §3.4).
- **R3 · MUST NOT.** A development build *"does **not** fall back on a default host, a tag in place of a
  commit, an empty string, or a rendered file with the token left in"* (CI §3.4).
- **R4 · MUST.** Build the CI substitution table as **exactly DM-2's rows and no others** (CI §3.4 step 2).
- **R5 · MUST.** *"Substitute literally, once, and never recursively. Every occurrence of a token is
  replaced by the value's bytes, and no substituted value is ever rescanned for tokens. The render is a
  function of the table, not of the order the table is walked."* (CI §3.4 step 3).
- **R6 · MUST.** Only the four CI templates carry a `@@` or `PIN_` token — `ci-generic`,
  `ci-github-collect`, `ci-github-land`, `ci-gitlab`. **`ci-generic` carries `@@DIST_BASE@@` and no trunk
  name**, since `ci.sh` takes trunk as an argument. *"No other template the release ships contains either
  token, and none is scanned."* (CI §3.4 step 4).
- **R7 · MUST.** Scan **every rendered CI file** for surviving tokens after every substitution, **before
  any path is written** (CI §3.4 steps 5–6, §13 item 1).
- **R8 · REFUSE (`unsubstituted-token`).** The scan is over the rendered bytes and requires: *"no
  occurrence of `@@` — two `U+0040`, in any context; and no occurrence of `PIN_CHECKOUT`,
  `PIN_UPLOAD_ARTIFACT` or `PIN_DOWNLOAD_ARTIFACT`. Any occurrence is `unsubstituted-token`: the whole
  plan is `REFUSE` and nothing is written."* (CI §3.4).
- **R9 · MUST.** The scan *"is a byte scan over the rendered bytes and reads nothing else — it re-parses
  no YAML, does not know which template produced the bytes, and gives the same answer on every platform"*
  (CI §3.4). MUST NOT be implemented as a YAML-aware or template-aware check.
- **R10 · MUST.** *"the scan precedes every write, and one failure refuses the **whole** plan rather than
  writing the paths that happened to pass"* (CI §3.4). Only then does the plan compare blob ids and write
  (step 6).
- **R11 · REFUSE (`trunk-name-collides-with-token`).** `init` refuses a `--trunk` name containing `@@` or
  equal to one of the three `PIN_` literals **at `--trunk`, where it is given**; a repository whose
  manifest already carries one *"meets the same refusal at the scan instead, which is the fail-closed
  direction and leaves its tree untouched"* (CI §3.4).
- **R12 · MUST NOT.** Do not scan before the trunk substitution: *"an order-dependent test is one two
  implementations can disagree about while both believing they conform"* (CI §3.4). `params.trunk`'s
  grammar is unchanged — any name `git check-ref-format --branch` accepts (MF §3.3).
- **R13 · REFUSE.** `spine init --ci github` refuses when `params.trunk` ≠ the repository's default
  branch (CI §7.1, §14 R13, §15 D6).
- **R14 · REFUSE.** `spine init --ci gitlab` and `spine init --ci generic` **refuse to write
  `merge.auto = on`** (CI §8.1, §9.3, §13 item 14; PB §7.4 rule 0). The refusal is *"a refusal to *write*,
  not a gate"* — a human may still edit `C-M4` by hand, and precondition 2 remains `"unmet"` regardless.
- **R15 · MUST.** `spine init --ci generic` writes `.spine/ci.sh` **and no definition at all**, and
  **prints** the CI §9.4 contract (CI §9.1).
- **R16 · SHOULD.** `init --ci github` prints the PR obligation: *"on GitHub, a candidate branch needs an
  open pull request against trunk as its event source"* — `spine new` does not open it (CI §7.1, §14 R23).

### `.spine/ci.sh` — invocation contract

- **R17 · MUST.** The invocation is exactly (CI §5.1):
  `git show "origin/<trunk>:.spine/ci.sh" >"$TMP/ci.sh"`, then `sh "$TMP/ci.sh" install <trunk>` (trusted)
  or `sh "$TMP/ci.sh" collect <trunk> <candidate-ref>` (untrusted).
- **R18 · MUST.** The bytes come from `git show origin/<trunk>:.spine/ci.sh` and go to a file **outside
  the repository working tree** — *"`$TMP` is the job's private scratch directory, not the workspace"* —
  which is then run with `sh` (CI §5.1, §13 item 2).
- **R19 · MUST NOT.** *"It is never piped into `sh -s`."* Piping is **non-conforming** (CI §5.1, §14 R2).
- **R20 · MUST.** Both modes take the trunk name as a **positional argument** from the CI definition
  (out-of-band configuration); trunk's own `params.trunk` is compared against it **as a misconfiguration
  guard, never as the source** (CI §5.1, §14 R3).
- **R21 · REFUSE (exit 2).** `collect` requires HEAD to be on the candidate branch; a detached HEAD is a
  refusal. *"`H` is `git symbolic-ref HEAD`"* (CI §5.1, §14 R7). `ci.sh` MUST NOT mutate the workspace to
  fix this.
- **R22 · MUST.** *"stdout carries exactly one line on success and nothing else."* `install` prints the
  absolute path of the verified binary; `collect` prints `result=<repo-relative path>`. *"Every
  diagnostic, and all of the collector's own output, goes to stderr."* (CI §5.1, §13 item 3).
- **R23 · MUST.** Exit codes are exactly DM-8's three, and no fourth (CI §5.2).

### `.spine/ci.sh` — behaviour, in order (CI §5.4)

- **R24 · MUST.** Fix `IFS` and `LC_ALL=C`, set **`umask 022`** (not `077`), and drop every relative or
  empty `PATH` entry (CI §5.3, §5.4 item 1).
- **R25 · REFUSE (exit 2).** If the sanitised `PATH` contains no absolute directory:
  `PATH contains no absolute directory`.
- **R26 · MUST.** Apply the restrictive mode where it is owed: **`chmod 0700 "$WORK"`** explicitly, and
  **`0755` on `$INSTALL_DIR` and on `$BIN`**. Reason (RF §7.1 M1): the collector inherits this umask and
  the mapped id that runs every runner and the boundary probe *"is by construction neither the collector's
  uid nor 0"* — at 077 *"`profile=container` could not be licensed on any host"* (CI §5.3, §5.4 item 1).
- **R27 · REFUSE (exit 2), `collect` only, before any download.** Refuse if **any** of
  `SPINE_PIPELINE_KEY`, `SPINE_PUSH_KEY`, `SPINE_PUSH_TOKEN` is **set — set at all, empty value
  included** (CI §5.3, §5.4 item 2, §6.1).
- **R28 · MUST NOT.** `ci.sh` **does not strip** those variables: *"Stripping would launder a
  misconfigured pipeline into a passing assertion, and rule 0 asks the job to fail, not to cope."*
  (CI §6.1, §14 R6).
- **R29 · REFUSE (exit 2), `collect` only.** Refuse when `SPINE_TRUST_ROOT` is unset:
  `SPINE_TRUST_ROOT is unset; spine check --ci refuses to run without it` (CI §5.3).
- **R30 · REFUSE (exit 2).** git preflight: `git` present; `git --version` parsed to major.minor;
  **git ≥ 2.38 required** (`merge-tree --write-tree`, PB §11); inside a git repository;
  `git check-ref-format --branch` over both ref arguments (CI §5.3, §5.4 item 3).
- **R31 · MUST.** Read policy from the base: `git show "origin/<trunk>:.spine/manifest.json"`, then
  **two members and no others** — `params.trunk` (must equal the argument) and `cli.dist_hash` (must be
  `sha256:` + exactly 64 lowercase hex) (CI §5.4 item 4).
- **R32 · MUST.** `ci.sh` **never parses the manifest as JSON**. `json_one` *"splits on JSON structure
  characters and accepts only a line that is exactly `"key": "value"`; absence and multiplicity are both
  refusals, never a guess."* (CI §5.4). MF §3.10 makes `trunk` and `dist_hash` **reserved member names**
  so the extractor cannot see two matches (`reserved-member-name`).
- **R33 · MUST NOT.** `ci.sh` needs no `cli.version`: *"the version is derived from the hash-verified
  artifact list rather than read from the manifest beside the hash: a version string that could be read
  independently of the digest is a string that could disagree with it."* (CI §5.4, §14 R4).
- **R34 · REFUSE (exit 2).** `DIST_BASE` (from `SPINE_DIST_BASE`, else the baked default) MUST match
  `https://*`; exactly one trailing `/` is appended if absent.
- **R35 · MUST.** Resolve `TARGET` from `uname -s`/`uname -m` by DM-5; **anything else refuses, exit 2**.
- **R36 · MUST.** Fetch `${DIST_BASE}${DIST_HASH}/artifacts.txt`; **its SHA-256 MUST equal the pinned
  `DIST_HASH`** or refuse (exit 2).
- **R37 · MUST.** Select the artifact line with the anchored matcher of CI §5.3:
  `^\([0-9a-f]\{64\}\)  \(spine-[0-9A-Za-z._+-]*-$TARGET\.tar\.gz\)$` — **two literal spaces**. Zero
  matches and more-than-one match are **both refusals, exit 2** (CI §5.5).
- **R38 · MUST.** Fetch `${DIST_BASE}${DIST_HASH}/${ART_NAME}`; **its SHA-256 MUST equal the listed
  `ART_SHA`** or refuse (exit 2).
- **R39 · MUST.** Unpack with `gzip -dc "$WORK/$ART_NAME" | (cd "$INSTALL_DIR" && tar -xf -)`; the
  archive MUST contain a `spine` binary **at its root**; `chmod 0755 "$BIN"` (CI §5.3).
- **R40 · MUST.** `install` mode stops here, prints `$BIN` on stdout, and exits 0 (CI §5.4 item 6).
- **R41 · MUST.** `collect` only: export the registry allowlist and, when `SPINE_REGISTRY_PROXY` is set
  and is `https://`, the three client variables (CI §5.4 item 7, §5.6). A non-`https://`
  `SPINE_REGISTRY_PROXY` is a refusal (exit 2).
- **R42 · MUST.** `collect` only: verify `refs/remotes/origin/<trunk>` and `refs/heads/<candidate>` are
  fetched, and HEAD is on `<candidate>`; then compute
  `T := git merge-tree --write-tree refs/remotes/origin/<trunk> refs/heads/<candidate>` (CI §5.3, §5.4
  item 8).
- **R43 · REFUSE (exit 2, `needs-rebase`).** A non-zero `merge-tree` exit prints its stderr and refuses
  with `merge-tree reports a conflict: needs-rebase` — *"exit 2 with no file, which is RF §7.1 step 5's
  'the collector fails the job and writes nothing'"* (CI §5.4 item 8).
- **R44 · MUST.** Run the collector from the repository top level, **stdin from `/dev/null` and stdout
  redirected to stderr**: `(cd "$TOP" && exec "$BIN" check --ci --collect) </dev/null >&2`. Its exit
  status is **captured, not propagated** (CI §5.4 item 9).
- **R45 · MUST.** After the collector: if `$TOP/.spine/cache/results/<T>.jsonl` does not exist, **refuse,
  exit 2**. Otherwise print `result=<path>` and exit **1** if the collector's status was non-zero, else
  **0** (CI §5.3, §5.2). *"the file's existence decides between exit 1 and exit 2"* (CI §5.4 item 9).
- **R46 · MUST.** Clean up on exit: `rm -rf "$WORK"` and, when `ci.sh` created it for `collect`,
  `rm -rf "$OWN_INSTALL_DIR"`; `INT TERM HUP` clean up and **exit 2** (CI §5.3).

### The registry allowlist (CI §5.6)

- **R47 · MUST.** Export `SPINE_ALLOWED_HOSTS` with exactly these four hosts, in this order:
  `pypi.org files.pythonhosted.org registry.npmjs.org pub.dev` — *"one host set per v1 language that has
  a registry"*: `pypi.org` + `files.pythonhosted.org` (Python), `registry.npmjs.org` (TypeScript/
  JavaScript), `pub.dev` (Dart) (CI §5.3, §5.6).
- **R48 · MUST NOT.** Do not add `repo.maven.apache.org` or `services.gradle.org`: **removed 2026-08-27**
  when Kotlin was dropped; `kotlin` is not in `params.langs`' domain (MF §3.3) and `gradle` is a reserved
  runner token no adapter emits (IR §11.1) (CI §5.6, README settled decision 1).
- **R49 · MUST.** SwiftPM gets **no environment knob and no registry host**: *"it fetches from whatever
  git remotes the manifest names, and its mirrors live in the repository's own build configuration, which
  `C-T2` freezes and G8 guards"* (CI §5.6).
- **R50 · MUST.** When `SPINE_REGISTRY_PROXY` is set, derive exactly:
  `PIP_INDEX_URL="${SPINE_REGISTRY_PROXY%/}/pypi/simple"`,
  `NPM_CONFIG_REGISTRY="${SPINE_REGISTRY_PROXY%/}/npm/"`,
  `PUB_HOSTED_URL="${SPINE_REGISTRY_PROXY%/}/pub"` (CI §5.3).
- **R51 · MUST.** Read the split honestly: *"`ci.sh` declares and configures. The boundary enforces."*
  A POSIX shell script cannot filter a socket; `SPINE_ALLOWED_HOSTS` is *"exported for a container network
  policy, a proxy sidecar or an egress firewall to read"* (CI §5.6, §14 R21).
- **R52 · MUST NOT.** The **distribution host is not on the registry allowlist and must not be**: the
  installer's fetch happens before the restore phase exists and is authenticated by a digest read from
  trunk (CI §5.6, §15 D19).
- **R53 · MUST NOT.** `ci.sh` neither runs the dependency-restore phase nor knows the path; the restore
  phase is the **collector's**, run once per checkout (for `B` and for `T`) before the first runner
  invocation against that checkout, from `origin/<trunk>:.spine/restore.sh` (CI §5.6, RF §7.1).

### The two-job contract (CI §6.1–§6.3)

Untrusted job — U1…U8, verbatim obligations (CI §6.1):

- **R54 · MUST (U1).** *"Its definition comes from trunk, not from the candidate."* (PB §7.4 rule 0)
- **R55 · MUST (U2).** *"It is the **only** job that runs on `intent/*`, `quick/*` and
  `spine/upgrade-*`."*
- **R56 · MUST (U3).** *"`permissions: contents: read`, or the provider's equivalent, and **no
  secret**."*
- **R57 · MUST (U4).** *"It executes `.spine/ci.sh` **read from `origin/<trunk>`**, and nothing else from
  the repository before the collector."*
- **R58 · MUST (U5).** *"It **fails the run** if a pipeline-key variable is visible to it."*
- **R59 · MUST (U6).** *"It hands over the result file — **its only artifact** — preserving the path of
  §6.3, even when the collector exited non-zero."*
- **R60 · MUST (U7).** *"It computes `T` itself, collects the `B` id set before any process runs against
  `T`'s content, and enforces `params.timeout`. All of this is the collector's, invoked by U4."*
- **R61 · MUST NOT (U8).** *"This job therefore adds **no** restore, install or setup step of its own …
  a restore step here would execute candidate-authored lifecycle scripts before rule 0's key-visibility
  probe had run."*
- **R62 · MUST.** Two parties assert the key-visibility probe and they assert different things:
  `ci.sh` **refuses** (R27/R28); **the collector measures and records** `keys_visible=` — RF §4.2's
  predicate over signing key material of any kind, *"the three variables above, plus `SSH_AUTH_SOCK`,
  `GPG_AGENT_INFO`, a readable `~/.ssh` or `~/.gnupg`"*, over the collector's own environment and every
  runner invocation's (CI §6.1).
- **R63 · MUST.** *"The provider's ambient token is not signing key material and does not set
  `keys_visible=true`. It cannot produce a `Spine-Seal`. What bounds it is U3."* (CI §6.1)

Trusted job — T1…T9, verbatim obligations (CI §6.2):

- **R64 · MUST (T1).** *"Triggered **only** from a trunk-scoped event, whose definition the provider takes
  from trunk. Never from a push, never from a pull-request event, never from `merge_group`."*
- **R65 · MUST (T2).** *"The pipeline key lives in an environment whose **deployment-branch rule is the
  trunk only**."*
- **R66 · MUST (T3).** *"Checks out **full history plus an explicit fetch of `refs/heads/intent/*`**, or
  the lease registry is empty and G7 is vacuous. It also fetches `quick/*` and `spine/upgrade-*`."*
- **R67 · MUST (T4).** *"Installs the pinned release through `.spine/ci.sh install`, read from
  `origin/<trunk>`, and runs **no repository code**: not the candidate's, not trunk's, not a build."*
- **R68 · MUST (T5).** *"Restores **no cache**, and rebuilds the graph `--fresh`."*
- **R69 · MUST (T6).** *"Ingests exactly one result file, materialized at the path of §6.3, from the
  untrusted run's artifact and from nowhere else."*
- **R70 · MUST (T7).** *"Pushes trunk with a credential the untrusted job does not hold. The bypass
  principal bypasses required checks only; the non-fast-forward rule has no bypass list."*
- **R71 · MUST (T8).** *"**Publishes the gate report to `refs/notes/spine` after the CAS**, on every
  landing."*
- **R72 · MUST (T9).** *"Keeps the canonical gate-report bytes as an artifact of its own run,
  `if: always()`."*

Handoff (CI §6.3):

- **R73 · MUST.** The collector writes `<repo-root>/.spine/cache/results/<T>.jsonl` — *"repo-relative,
  exactly that path, one file however many runners ran (RF §3)"*. `<repo-root>` is
  `git rev-parse --show-toplevel` **of the repository `ci.sh` was invoked in**, whatever detached
  checkouts the collector makes elsewhere (§14 R8).
- **R74 · MUST.** *"The untrusted job's **only** artifact carries that one file and nothing else. No log,
  no report, no second copy, no directory of runs."*
- **R75 · MUST.** The trusted job *"**materializes it at exactly `.spine/cache/results/<T>.jsonl`
  relative to its own workspace**, then checks that the materialized tree holds exactly one entry, that
  the entry is a regular file and not a symlink, and that its name ends `.jsonl`."* The transport
  container's internal naming is the provider's business (§14 R9).
- **R76 · MUST NOT.** The definition MUST NOT check identity: *"Identity is spine's, not the
  definition's … **The YAML checks shape; `spine check` checks identity.** A definition that tried to
  check identity would be a second, unsealed implementation of RF §8.3."*

### Landing, routing and note publication

- **R77 · MUST.** Route by DM-9, **`quick/reseal-*` tested before `quick/*`** (CI §6.4, §13 item 8,
  §14 R15).
- **R78 · MUST.** *"`spine check --ci --land` publishes it, not the definition."* The CI definition
  uploads a copy and does nothing else (CI §6.5).
- **R79 · MUST.** Order inside `--land`: *"step 4 build `L` → step 5 G9 and G10 on `L` in a scratch clone
  → step 6 the compare-and-swap → **then** `git hash-object -w` the canonical bytes and
  `git notes --ref=spine add -C <blob> <L>` and `git push origin refs/notes/spine`."* **After the CAS, and
  never before** (CI §6.5, GR §4.4.2).
- **R80 · MUST NOT.** No CI definition composes, edits or passes in a commit message: the landing subject
  is derived and **G9 recomputes it and refuses a landing whose subject it did not produce** (CI §6.5,
  PB §11 *Subject lines*, README settled decision 6).
- **R81 · MUST.** `--land --ci` **always** writes the canonical bytes to `.spine/cache/report.json`,
  overwritten per run, inside the gitignored cache; the definition uploads it `if: always()` (CI §6.5,
  PB §11 CLI; §14 R17).
- **R82 · MUST (exit 5, `note-publish-failed`).** *"A failed note push fails the job and retracts
  nothing."* The landing is complete; nothing can un-land a commit that reached trunk (CI §6.5).
- **R83 · MUST.** Note-push concurrency: a rejected non-fast-forward push is answered by fetching the
  ref, re-applying this landing's note, and retrying — **bounded, never with `--force`**; the retry is
  spine's, not the definition's (CI §6.5, GR §4.4.2).
- **R84 · MUST.** One trusted run at a time: GitHub `concurrency: spine-land` with
  `cancel-in-progress: false`; GitLab `resource_group: spine-land` with `interruptible: false`. Untrusted
  runs are unbounded and `cancel-in-progress: true` **per candidate** (CI §7.2, §7.3, §8.3, §8.4, §14 R27).

### Templating safety and provider facts

- **R85 · MUST NOT.** *"No candidate-controlled value is interpolated into a `run:` script."* Every
  `${{ }}` carrying a branch name, a repository name or a step output **crosses into shell as an `env:`
  binding** (CI §7.2, §13 item 11, §14 R20). This is *"the one place a CI definition can hand a candidate
  the trusted stage"*.
- **R86 · MUST.** The trusted GitHub job's `if:` carries **all three** clauses; two of the three are the
  guarantee, not defence in depth: `workflow_run.event == 'pull_request_target'`,
  `workflow_run.path == '.github/workflows/spine-collect.yml'`,
  `workflow_run.head_repository.full_name == github.repository` (CI §7.3, §14 R11).
- **R87 · MUST.** GitHub permissions: collect = `contents: read`; land = `actions: write` **and**
  `contents: read` — *"`read` is enough to download the artifact; the re-queue … needs `write`"* (CI §7.2,
  §7.3).
- **R88 · MUST.** The push credential is `SPINE_PUSH_TOKEN` and **never `github.token`**; its base64 is
  registered with `::add-mask::` before being written anywhere, and injected via
  `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0`/`GIT_CONFIG_VALUE_0` **with no file on disk** (git ≥ 2.31)
  (CI §7.3).
- **R89 · MUST.** GitHub checkout for the trusted job uses `persist-credentials: false`; the untrusted
  job **strips every credential from `.git/config` after fetching trunk** and fails if one survives
  (CI §7.2, §7.3).
- **R90 · MUST.** Stage the GitHub handoff into `$RUNNER_TEMP/spine-handoff` (a **dot-free** directory)
  rather than uploading from `.spine/cache/results` — `actions/upload-artifact` roots at the least common
  ancestor and some versions exclude dot-prefixed segments (CI §7.2).
- **R91 · MUST.** `fetch-depth: 0` **and** the explicit `origin/<trunk>` fetch are both required —
  *"`actions/checkout` fetches the named ref; `origin/<trunk>` is a separate fetch, and `ci.sh` refuses
  without it"* (CI §7.2).
- **R92 · MUST.** The upload step is `if: always() && steps.<id>.outputs.rc != ''` and the job's failure
  is deferred to a later step, so `ci.sh` exit 1 still hands the file over: *"A red suite must reach G1 as
  evidence, not vanish as a failed job."* (CI §7.2, §14 R22).
- **R93 · MUST NOT.** No merge queue in the shipped definitions: *"A merge queue creates the trunk commit
  itself"*, which is configuration (b) by PB §7.4 rule 5 precondition 4 *"whatever it is called"*
  (CI §7.5, §14 R12).
- **R94 · MUST.** GitLab discovery for the scheduled trusted job is CI §8.4's four deterministic steps:
  fetch the three ref globs; **sort the candidate ref names ascending by bytes**; for each in turn ask the
  API for a pipeline whose `sha` is that ref's tip and whose `source` is `merge_request_event`, and for a
  `spine-collect` job in it with an artifact; the first that has one is this run's candidate; download and
  land (CI §8.4, §14 R25). The sort *"is an ordering, not a priority"*.
- **R95 · MUST.** On GitLab, `base-moved` needs **no** re-queue step: the next scheduled pipeline
  rediscovers the candidate against the new tip (CI §8.4).
- **R96 · MUST.** Under `--ci generic` the operator's `publish_artifact` / `fetch_artifact_into`
  placeholders MUST *"move **one file, unmodified, preserving the relative path**"*, and `$REPO_URL` MUST
  be *"a credential the two jobs do not share"* (CI §9.4).
- **R97 · MUST.** Precondition 2 has **three** conjuncts: `keys_visible=false`; the collector's `tool=` is
  the base's pin; **and** this run established that the ingested file came from a trunk-defined untrusted
  job. On GitLab-in-repository and on `generic` the third **never** holds, so precondition 2 is
  `"unmet"`, the rule-5 `G11` wire is raised, and every landing takes a human reading (CI §8.1, §9.3,
  §13 item 14; PB §7.4 rule 5; README settled decision 2).
- **R98 · MUST NOT.** Nothing anywhere records a time, compares two, or derives a decision from one; a
  scheduled trigger is an **event source, not a clock** (CI §1, §8.4, §13 item 13, §14 R28).

---

## Algorithm

### A · `spine init` — rendering the CI artifacts (CI §3.4)

1. Validate the embedded release manifest against DM-1. Failure ⇒ `no-release-manifest`, **nothing is
   written**, every plan row `REFUSE` (R2).
2. Build the substitution table: exactly DM-2's five rows (R4).
3. Substitute literally, once, never recursively, into the four CI templates only (R5, R6).
4. Byte-scan every rendered CI file for `@@` and the three `PIN_` literals (R7–R9).
5. Any hit ⇒ `unsubstituted-token`, **the whole plan is REFUSE**, nothing written (R8, R10).
6. Only then compute the plan (`create · update · delete · skip · REFUSE`), compare blob ids, and write
   (CI §3.4 step 6, PB §6.7).
7. Record in `.spine/manifest.json`: `cli.version` = release manifest `version`; `cli.dist_hash` =
   `sha256:` + the artifact list digest; one `files[]` record per written path with its `template` at
   `name@version`; the twelve `templates` keys (MF §3.2, §3.6; PB §6.7).

### B · `.spine/ci.sh` — common prologue (both modes)

1. `set -eu`; compute `NL`; set `IFS` to space/tab/newline; `LC_ALL=C`, exported; `umask 022`.
2. Sanitise `PATH` to absolute entries only; export; refuse if empty (R24, R25).
3. Bind `MODE=$1`, `TRUNK=$2`, `CANDIDATE=$3`; refuse on `$# < 2`, on an unknown mode, and on `collect`
   with `$# ≠ 3`.
4. `collect` only: key-visibility probe (R27) and `SPINE_TRUST_ROOT` presence (R29) — **before anything
   is downloaded and long before any repository code is executed**.
5. git preflight: presence, version ≥ 2.38, inside a repository, both ref names well-formed (R30).
6. `WORK=$(mktemp -d "${TMPDIR:-/tmp}/spine-ci.XXXXXX")`; `chmod 0700 "$WORK"`; install `cleanup` on
   `EXIT` and `INT TERM HUP` (the signal trap exits **2**).
7. Policy from the base: `git show "origin/$TRUNK:.spine/manifest.json"`; `json_one trunk` must equal
   `$TRUNK`; `json_one dist_hash` must be `sha256:` + 64 lowercase hex (R31).
8. `DIST_BASE` resolution and `https://` check; ensure one trailing `/` (R34).
9. `uname -s`/`uname -m` → `TARGET` (R35).
10. `INSTALL_DIR` (from `SPINE_INSTALL_DIR` or `mktemp -d`), `mkdir -p`, `chmod 0755`; `BIN=$INSTALL_DIR/spine`.
11. Fetch and hash-verify `artifacts.txt` against `DIST_HASH` (R36).
12. Select exactly one line for `TARGET`; zero or two is a refusal (R37).
13. Fetch and hash-verify the artifact against its listed digest (R38).
14. `gzip -dc | tar -xf -` into `INSTALL_DIR`; require `spine` at the archive root; `chmod 0755 "$BIN"` (R39).

### C · `install` mode

15. Print `$BIN` on stdout; `exit 0` (R40). (Everything after step 14 is `collect`-only.)

### D · `collect` mode

15. Export `SPINE_ALLOWED_HOSTS`; if `SPINE_REGISTRY_PROXY` is set and `https://`, export the three
    client variables (R41, R47, R50).
16. Verify `refs/remotes/origin/$TRUNK` and `refs/heads/$CANDIDATE` are fetched; verify
    `git symbolic-ref --quiet --short HEAD` equals `$CANDIDATE` (R21, R42).
17. `T := git merge-tree --write-tree refs/remotes/origin/$TRUNK refs/heads/$CANDIDATE`; a non-zero exit
    prints stderr and refuses `needs-rebase` (R43). Truncate `T` at the first `NL`; refuse a non-hex or
    empty result.
18. `TOP := git rev-parse --show-toplevel`; `RESULT := .spine/cache/results/$T.jsonl`.
19. Run `(cd "$TOP" && exec "$BIN" check --ci --collect) </dev/null >&2`; capture `COLLECTOR_RC` (R44).
20. If `$TOP/$RESULT` is absent ⇒ refuse, exit 2 (R45).
21. Print `result=$RESULT`; `exit 1` when `COLLECTOR_RC ≠ 0`, else `exit 0` (R45).

### E · The untrusted job (any provider)

1. Guard: head repository is this repository; the ref matches `intent/*`, `quick/*` or
   `spine/upgrade-*`; `SPINE_TRUST_ROOT` is set. Otherwise fail the job before anything else runs.
2. Check out the candidate ref with full history; fetch `origin/<trunk>` explicitly; strip credentials
   from the checkout (R89, R91).
3. `git show "origin/<trunk>:.spine/ci.sh"` into a private scratch dir (mode 0700) **outside the
   workspace**; run it with `sh` (R17–R19).
4. Capture `rc`. `rc == 2` ⇒ fail now, nothing to hand over. Otherwise parse the single `result=` line.
5. Stage the named file into a dot-free handoff directory (GitHub) or rely on path-preserving artifacts
   (GitLab); upload as **one file** (R74, R90).
6. Upload `if: always()`; **then** fail the job if `rc != 0` (R92).

### F · The trusted job (any provider)

1. Guard the trigger with provider facts a candidate cannot forge (GitHub: the three `if:` clauses;
   GitLab: `$CI_PIPELINE_SOURCE == "schedule" && $CI_COMMIT_REF_NAME == "<trunk>"`) (R86, R64).
2. Refuse a non-candidate ref; refuse when `SPINE_TRUST_ROOT`, `SPINE_PIPELINE_KEY` or
   `SPINE_PUSH_TOKEN` is unset (GitLab: exit 3).
3. Check out trunk, full history, `persist-credentials: false`; fetch trunk plus `intent/*`, `quick/*`,
   `spine/upgrade-*` (R66).
4. Authenticate git without writing a credential to disk (R88).
5. `git show "origin/<trunk>:.spine/ci.sh"`; `SPINE_INSTALL_DIR=<scratch>/bin sh ci.sh install <trunk>`;
   read the single stdout line as the binary path (R67).
6. Materialize the artifact at `.spine/cache/results/`; **shape check**: exactly one entry, a regular
   file, not a symlink, name ends `.jsonl` (R75).
7. Route the candidate ref to a `--land` invocation by DM-9, **`quick/reseal-*` first** (R77).
8. Run `"$BIN" check --ci "$@" </dev/null`; capture `rc`.
9. Upload `.spine/cache/report.json` `if: always()` (R72, R81).
10. `rc == 2` ⇒ re-queue the two-job run (GitHub: `gh api --method POST …/actions/runs/<id>/rerun`;
    GitLab: nothing — the next schedule rediscovers) (R95).
11. Fail the job when `rc != 0`; react per DM-8's obligation column (never retry on 1, 3, 4, 5).

### G · Gate-report publication, inside `--land` (CI §6.5, GR §4.4.2)

1. Build `L` (step 4) → G9/G10 in a scratch clone (step 5) → compare-and-swap (step 6).
2. `blob=$(printf '%s' "$canonical" | git hash-object -w --stdin)`
3. `git notes --ref=spine add -C "$blob" <L>` — `-m`, `-F` and editor paths are **non-conforming**
   (GR §4.4.2).
4. `git push origin refs/notes/spine`; on non-fast-forward, fetch, re-apply, retry — bounded, never
   `--force` (R83).
5. Failure ⇒ exit 5, job fails, landing stands (R82).

---

## Byte-level fixities

### F-1 · `.spine/ci.sh`, reproduced byte for byte (CI §5.3)

*"Reproduced byte for byte. Tabs are tabs; every line ends `LF`; there is exactly one trailing `LF`."*

```sh
#!/bin/sh
# .spine/ci.sh — spine-kit's CI entry point.  Rendered by `spine init` from template
# ci-generic@4; owner class `spine-owned` (PB 6.7).  Do not edit: `spine init` refuses
# to upgrade a copy whose blob differs from the manifest's, and `.spine/**` is on the
# protected floor (PB 7.3), so a change here takes a protected review.
#
# Invocation — always from trunk, never from the checkout (PB 7.4 rule 0):
#     git show "origin/<trunk>:.spine/ci.sh" >"$TMP/ci.sh"
#     sh "$TMP/ci.sh" install <trunk>                    # trusted job
#     sh "$TMP/ci.sh" collect <trunk> <candidate-ref>    # untrusted job
#
# stdout carries exactly one line on success and nothing else:
#     install -> the absolute path of the hash-verified spine binary
#     collect -> result=<repo-relative path of the result file>
# Every diagnostic goes to stderr.
#
# Exit: 0 the collector ran and exited 0 (or `install` succeeded)
#       1 the collector ran, exited non-zero, and a result file exists
#       2 refused: nothing ran and no result file exists

set -eu

# Render-time constants.  `spine init` substitutes them; a rendered ci.sh still
# containing a '@@' token is not a conforming render and init refuses to write it.
SPINE_DIST_BASE_DEFAULT='@@DIST_BASE@@'

NL="$(printf '\n_')"; NL="${NL%_}"
IFS="$(printf ' \t\n_')"; IFS="${IFS%_}"
LC_ALL=C
export LC_ALL

# Not `umask 077`.  RF 7.1's M1 spawns every runner, and its boundary probe,
# under an id that is by construction neither the collector's uid nor 0, and the
# collector inherits this umask: at 077 every checkout it writes, and every file
# under $INSTALL_DIR, is unreachable to that id and M1 fails a prerequisite
# rather than a test.  The restrictive mode is applied to $WORK instead, which is
# the only directory here that ever holds unverified bytes.
umask 022

die() {
	_die_rc=$1
	shift
	printf 'spine/ci.sh: %s\n' "$*" >&2
	exit "$_die_rc"
}

have() { command -v "$1" >/dev/null 2>&1; }

# ---------------------------------------------------------------- PATH hygiene
# Drop every relative or empty PATH entry: a candidate that commits ./git or
# ./curl must not be able to interpose on anything below.
sanitize_path() {
	_sp_out=''
	_sp_rest="${PATH-}"
	while [ -n "$_sp_rest" ]; do
		case "$_sp_rest" in
		*:*)
			_sp_head="${_sp_rest%%:*}"
			_sp_rest="${_sp_rest#*:}"
			;;
		*)
			_sp_head="$_sp_rest"
			_sp_rest=''
			;;
		esac
		case "$_sp_head" in
		/*)
			if [ -z "$_sp_out" ]; then
				_sp_out="$_sp_head"
			else
				_sp_out="$_sp_out:$_sp_head"
			fi
			;;
		esac
	done
	printf '%s' "$_sp_out"
}
PATH="$(sanitize_path)"
export PATH
[ -n "$PATH" ] || die 2 'PATH contains no absolute directory'

# -------------------------------------------------------------------- helpers
sha256_of() {
	if have sha256sum; then
		sha256sum "$1" </dev/null | cut -d' ' -f1
	elif have shasum; then
		shasum -a 256 "$1" </dev/null | cut -d' ' -f1
	elif have openssl; then
		openssl dgst -sha256 -r "$1" </dev/null | cut -d' ' -f1
	else
		die 2 'no SHA-256 utility: need sha256sum, shasum or openssl'
	fi
}

fetch_to() {
	if have curl; then
		curl -fsS --proto '=https' --tlsv1.2 --retry 3 --max-time 300 \
			-o "$2" "$1" </dev/null
	elif have wget; then
		wget -q --https-only -O "$2" "$1" </dev/null
	else
		die 2 'no HTTPS client: need curl or wget'
	fi
}

# json_one <key> <file> — print the single JSON string value of <key>, refusing
# absence and ambiguity.  This is not a JSON parser: it splits on JSON structure
# characters and accepts only a line that is exactly `"key": "value"`, so a
# member of that name anywhere else in the document is a refusal, not a guess.
json_one() {
	_jo_v="$(tr ',{}[]' '\n\n\n\n\n' <"$2" |
		sed -n 's/^[	 ]*"'"$1"'"[	 ]*:[	 ]*"\([^"]*\)"[	 ]*$/\1/p')"
	case "$_jo_v" in
	'') die 2 "manifest: no \"$1\" member" ;;
	*"$NL"*) die 2 "manifest: \"$1\" occurs more than once" ;;
	esac
	printf '%s' "$_jo_v"
}

# ------------------------------------------------------------------ arguments
MODE="${1-}"
TRUNK="${2-}"
CANDIDATE="${3-}"
[ $# -ge 2 ] || die 2 'usage: ci.sh install <trunk> | ci.sh collect <trunk> <candidate-ref>'
case "$MODE" in
install | collect) : ;;
*) die 2 "unknown mode: $MODE" ;;
esac
[ "$MODE" = install ] || [ $# -eq 3 ] || die 2 'collect needs a candidate ref'

# ------------------------------------------------- rule 0: key-visibility probe
# The untrusted job asserts, by refusing to run, that no spine credential is
# reachable from it.  This is checked before anything is downloaded and long
# before any repository code is executed.
if [ "$MODE" = collect ]; then
	for _v in SPINE_PIPELINE_KEY SPINE_PUSH_KEY SPINE_PUSH_TOKEN; do
		eval "_seen=\${$_v+set}"
		if [ "${_seen-}" = set ]; then
			die 2 "rule 0: $_v is visible to the untrusted job"
		fi
	done
	unset _v _seen
	[ -n "${SPINE_TRUST_ROOT-}" ] ||
		die 2 'SPINE_TRUST_ROOT is unset; spine check --ci refuses to run without it'
fi

# ------------------------------------------------------------ git preflight
have git || die 2 'git not found'
_gv="$(git --version </dev/null)"
_gv="${_gv#git version }"
_gvmaj="${_gv%%.*}"
_gvrest="${_gv#*.}"
_gvmin="${_gvrest%%.*}"
case "$_gvmaj$_gvmin" in
'' | *[!0-9]*) die 2 "cannot parse git version: $_gv" ;;
esac
if [ "$_gvmaj" -lt 2 ] || { [ "$_gvmaj" -eq 2 ] && [ "$_gvmin" -lt 38 ]; }; then
	die 2 "git >= 2.38 required (merge-tree --write-tree); found $_gv"
fi
git rev-parse --git-dir >/dev/null 2>&1 || die 2 'not inside a git repository'
git check-ref-format --branch "$TRUNK" >/dev/null 2>&1 ||
	die 2 "not a branch name: $TRUNK"
if [ "$MODE" = collect ]; then
	git check-ref-format --branch "$CANDIDATE" >/dev/null 2>&1 ||
		die 2 "not a branch name: $CANDIDATE"
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/spine-ci.XXXXXX")" || die 2 'mktemp failed'
chmod 0700 "$WORK" || die 2 "cannot restrict $WORK"
OWN_INSTALL_DIR=''
cleanup() {
	[ -z "$WORK" ] || rm -rf "$WORK"
	[ -z "$OWN_INSTALL_DIR" ] || rm -rf "$OWN_INSTALL_DIR"
}
trap cleanup EXIT
trap 'cleanup; trap - EXIT; exit 2' INT TERM HUP

# ------------------------------------------------- policy, read from the base
git show "origin/$TRUNK:.spine/manifest.json" >"$WORK/manifest.json" 2>/dev/null ||
	die 2 "cannot read origin/$TRUNK:.spine/manifest.json"
MANIFEST_TRUNK="$(json_one trunk "$WORK/manifest.json")"
[ "$MANIFEST_TRUNK" = "$TRUNK" ] ||
	die 2 "manifest names trunk \"$MANIFEST_TRUNK\", invoked with \"$TRUNK\""
DIST_HASH="$(json_one dist_hash "$WORK/manifest.json")"
case "$DIST_HASH" in
sha256:*) DIST_HASH="${DIST_HASH#sha256:}" ;;
*) die 2 'manifest: cli.dist_hash is not sha256:<hex>' ;;
esac
case "$DIST_HASH" in
*[!0-9a-f]* | '') die 2 'manifest: cli.dist_hash is not lowercase hex' ;;
esac
[ "${#DIST_HASH}" -eq 64 ] || die 2 'manifest: cli.dist_hash is not 64 hex digits'

# ------------------------------------------------- install, and verify by hash
DIST_BASE="${SPINE_DIST_BASE:-$SPINE_DIST_BASE_DEFAULT}"
case "$DIST_BASE" in
https://*) : ;;
*) die 2 'SPINE_DIST_BASE must be an https:// URL' ;;
esac
case "$DIST_BASE" in
*[!/]) DIST_BASE="$DIST_BASE/" ;;
esac

_os="$(uname -s </dev/null)"
_arch="$(uname -m </dev/null)"
case "$_os" in
Linux)
	case "$_arch" in
	x86_64 | amd64) TARGET='x86_64-unknown-linux-musl' ;;
	aarch64 | arm64) TARGET='aarch64-unknown-linux-musl' ;;
	*) die 2 "unsupported architecture: $_os/$_arch" ;;
	esac
	;;
Darwin)
	case "$_arch" in
	arm64) TARGET='aarch64-apple-darwin' ;;
	x86_64) TARGET='x86_64-apple-darwin' ;;
	*) die 2 "unsupported architecture: $_os/$_arch" ;;
	esac
	;;
*) die 2 "unsupported platform: $_os (v1 ships no Windows CI target)" ;;
esac

INSTALL_DIR="${SPINE_INSTALL_DIR-}"
if [ -z "$INSTALL_DIR" ]; then
	INSTALL_DIR="$(mktemp -d "${TMPDIR:-/tmp}/spine-bin.XXXXXX")" || die 2 'mktemp failed'
	if [ "$MODE" = collect ]; then OWN_INSTALL_DIR="$INSTALL_DIR"; fi
fi
mkdir -p "$INSTALL_DIR"
# mktemp -d creates 0700 whatever the umask, and a contained runner's mapped id
# has to reach the hash-verified binary the probe re-execs (RF 7.1).  It is a
# release artifact, not a secret; the directory stays writable only by us.
chmod 0755 "$INSTALL_DIR" || die 2 "cannot make $INSTALL_DIR traversable"
BIN="$INSTALL_DIR/spine"

fetch_to "${DIST_BASE}${DIST_HASH}/artifacts.txt" "$WORK/artifacts.txt" ||
	die 2 'cannot fetch the release artifact list'
_got="$(sha256_of "$WORK/artifacts.txt")"
[ "$_got" = "$DIST_HASH" ] ||
	die 2 "artifact list hash $_got does not equal the pinned $DIST_HASH"

_line="$(sed -n "s/^\\([0-9a-f]\\{64\\}\\)  \\(spine-[0-9A-Za-z._+-]*-$TARGET\\.tar\\.gz\\)\$/\\1 \\2/p" \
	"$WORK/artifacts.txt")"
case "$_line" in
'') die 2 "the pinned release publishes no artifact for $TARGET" ;;
*"$NL"*) die 2 "the pinned release publishes more than one artifact for $TARGET" ;;
esac
ART_SHA="${_line%% *}"
ART_NAME="${_line#* }"

fetch_to "${DIST_BASE}${DIST_HASH}/${ART_NAME}" "$WORK/$ART_NAME" ||
	die 2 "cannot fetch $ART_NAME"
_got="$(sha256_of "$WORK/$ART_NAME")"
[ "$_got" = "$ART_SHA" ] ||
	die 2 "$ART_NAME hash $_got does not equal the listed $ART_SHA"

have gzip || die 2 'gzip not found'
have tar || die 2 'tar not found'
gzip -dc "$WORK/$ART_NAME" | (cd "$INSTALL_DIR" && tar -xf -) ||
	die 2 "cannot unpack $ART_NAME"
[ -f "$BIN" ] || die 2 "$ART_NAME contains no spine binary at its root"
chmod 0755 "$BIN"

if [ "$MODE" = install ]; then
	printf '%s\n' "$BIN"
	exit 0
fi

# ------------------------------------------------- registry allowlist (PB 7.1)
# Dependency restore is the untrusted job's only network access.  This file
# *declares* the allowlist and configures the clients that honour one; the
# isolation boundary is what *enforces* it.  SwiftPM has no single environment
# knob: its mirrors live in the repository's own build configuration, which
# C-T2 freezes and G8 guards.
SPINE_ALLOWED_HOSTS='pypi.org files.pythonhosted.org registry.npmjs.org pub.dev'
export SPINE_ALLOWED_HOSTS
if [ -n "${SPINE_REGISTRY_PROXY-}" ]; then
	case "$SPINE_REGISTRY_PROXY" in
	https://*) : ;;
	*) die 2 'SPINE_REGISTRY_PROXY must be an https:// URL' ;;
	esac
	PIP_INDEX_URL="${SPINE_REGISTRY_PROXY%/}/pypi/simple"
	NPM_CONFIG_REGISTRY="${SPINE_REGISTRY_PROXY%/}/npm/"
	PUB_HOSTED_URL="${SPINE_REGISTRY_PROXY%/}/pub"
	export PIP_INDEX_URL NPM_CONFIG_REGISTRY PUB_HOSTED_URL
fi

# --------------------------------------------------------- the synthetic merge
git rev-parse --verify -q "refs/remotes/origin/$TRUNK" >/dev/null ||
	die 2 "refs/remotes/origin/$TRUNK is not fetched"
git rev-parse --verify -q "refs/heads/$CANDIDATE" >/dev/null ||
	die 2 "refs/heads/$CANDIDATE is not fetched"
_head="$(git symbolic-ref --quiet --short HEAD || printf '')"
[ "$_head" = "$CANDIDATE" ] ||
	die 2 "HEAD is not on $CANDIDATE (the collector reads H from HEAD)"

if ! T="$(git merge-tree --write-tree "refs/remotes/origin/$TRUNK" \
	"refs/heads/$CANDIDATE" 2>"$WORK/mt.err")"; then
	cat "$WORK/mt.err" >&2
	die 2 'merge-tree reports a conflict: needs-rebase'
fi
case "$T" in
*"$NL"*) T="${T%%"$NL"*}" ;;
esac
case "$T" in
'' | *[!0-9a-f]*) die 2 'merge-tree produced no tree object id' ;;
esac

# ----------------------------------------------------------------- the collect
TOP="$(git rev-parse --show-toplevel)"
RESULT=".spine/cache/results/$T.jsonl"
set +e
(cd "$TOP" && exec "$BIN" check --ci --collect) </dev/null >&2
COLLECTOR_RC=$?
set -e
[ -f "$TOP/$RESULT" ] || die 2 "the collector wrote no result file at $RESULT"
printf 'result=%s\n' "$RESULT"
[ "$COLLECTOR_RC" -eq 0 ] || exit 1
exit 0
```

**Computed digests for exactly these bytes**, with `@@DIST_BASE@@` **unsubstituted** — *"a rendered
`ci.sh` carries the release's URL there and has a different id"* (CI §5.3):

| | |
|---|---|
| `git hash-object` (sha1) | `131f13fb0312162579605999d3f9f4e90098c74c` |
| SHA-256 of the file | `d6bcf50cf675614033aaef61df104aad253d30c4accc756719599ad5bd41060b` |
| Lines | 319 |

*"The blob id is the value a `files[]` record for `.spine/ci.sh` would carry under `object_format: sha1`
for this unsubstituted rendering."* (README's digest table carries the same three values and records both
2026-08-27 moves: the `SPINE_ALLOWED_HOSTS` narrowing at an unchanged 307 lines, then the umask narrowing
to 319.)

Sub-fixities inside the script an implementer must not paraphrase:

| Thing | Exact bytes |
|---|---|
| Diagnostic prefix | `printf 'spine/ci.sh: %s\n' "$*" >&2` — every diagnostic to **stderr** |
| stdout, `install` | the absolute path of `$BIN`, `printf '%s\n'`, one line |
| stdout, `collect` | `printf 'result=%s\n' "$RESULT"`, one line |
| `NL` | `NL="$(printf '\n_')"; NL="${NL%_}"` |
| `IFS` | `IFS="$(printf ' \t\n_')"; IFS="${IFS%_}"` |
| `json_one` split | `tr ',{}[]' '\n\n\n\n\n' <"$2"` then `sed -n 's/^[<TAB> ]*"'"$1"'"[<TAB> ]*:[<TAB> ]*"\([^"]*\)"[<TAB> ]*$/\1/p'` (the character class is **tab then space**) |
| curl | `curl -fsS --proto '=https' --tlsv1.2 --retry 3 --max-time 300 -o "$2" "$1" </dev/null` |
| wget | `wget -q --https-only -O "$2" "$1" </dev/null` |
| sha256 helpers, in order | `sha256sum "$1" </dev/null \| cut -d' ' -f1` → `shasum -a 256 …` → `openssl dgst -sha256 -r …` |
| artifact-line matcher | `sed -n "s/^\\([0-9a-f]\\{64\\}\\)  \\(spine-[0-9A-Za-z._+-]*-$TARGET\\.tar\\.gz\\)\$/\\1 \\2/p"` — **two spaces** between the digest and the name |
| allowlist | `SPINE_ALLOWED_HOSTS='pypi.org files.pythonhosted.org registry.npmjs.org pub.dev'` |
| result path | `RESULT=".spine/cache/results/$T.jsonl"` |
| modes | `umask 022` · `chmod 0700 "$WORK"` · `chmod 0755 "$INSTALL_DIR"` · `chmod 0755 "$BIN"` |

### F-2 · The release artifact list

Layout (CI §5.5):

```
<SPINE_DIST_BASE>/<H>/artifacts.txt      the list
<SPINE_DIST_BASE>/<H>/<artifact-name>    every artifact the list names
```

Line format, verbatim: *"Each line is `<64 lowercase hex>` `U+0020` `U+0020` `<artifact name>`."*
Sorting, verbatim: *"**Lines sorted ascending by the bytes of the artifact name.**"*
Termination, verbatim: *"every line terminated including the last, no CR anywhere, no BOM, no blank
lines, no comments, no header."*

### F-3 · `.github/workflows/spine-collect.yml` (CI §7.2)

```yaml
# Rendered by `spine init --ci github` from template ci-github-collect@N. spine-owned:
# do not edit. `.github/workflows/**` is on the protected floor (PB 7.3).
name: spine-collect

on:
  pull_request_target:
    types: [opened, synchronize, reopened]
    branches: ["main"]

permissions:
  contents: read

concurrency:
  group: spine-collect-${{ github.event.pull_request.head.label }}
  cancel-in-progress: true

defaults:
  run:
    shell: bash --noprofile --norc -euo pipefail {0}

jobs:
  collect:
    runs-on: ubuntu-latest
    timeout-minutes: 360
    env:
      SPINE_TRUNK: "main"
      SPINE_CANDIDATE: ${{ github.event.pull_request.head.ref }}
      SPINE_HEAD_REPO: ${{ github.event.pull_request.head.repo.full_name }}
      SPINE_THIS_REPO: ${{ github.repository }}
      SPINE_TRUST_ROOT: ${{ vars.SPINE_TRUST_ROOT }}
    steps:
      - name: Refuse anything that is not this repository's own candidate branch
        run: |
          if [ "$SPINE_HEAD_REPO" != "$SPINE_THIS_REPO" ]; then
            echo "spine: refusing a head outside $SPINE_THIS_REPO" >&2; exit 1
          fi
          case "$SPINE_CANDIDATE" in
            intent/*|quick/*|spine/upgrade-*) ;;
            *) echo "spine: not a candidate ref: $SPINE_CANDIDATE" >&2; exit 1 ;;
          esac
          if [ -z "$SPINE_TRUST_ROOT" ]; then
            echo "spine: vars.SPINE_TRUST_ROOT is unset" >&2; exit 1
          fi

      - uses: actions/checkout@PIN_CHECKOUT
        with:
          ref: ${{ github.event.pull_request.head.ref }}
          fetch-depth: 0

      - name: Fetch trunk, then drop every credential from the checkout
        run: |
          git fetch --no-tags --prune origin \
            "+refs/heads/$SPINE_TRUNK:refs/remotes/origin/$SPINE_TRUNK"
          git config --local --name-only --get-regexp '\.extraheader$' \
            | while read -r k; do git config --local --unset-all "$k"; done || true
          git config --local --unset-all core.sshCommand || true
          if git config --local --name-only --get-regexp '\.extraheader$'; then
            echo "spine: a credential survived in .git/config" >&2; exit 1
          fi

      - name: Collect
        id: collect
        run: |
          d="$RUNNER_TEMP/spine"; mkdir -p "$d"; chmod 700 "$d"
          git show "origin/$SPINE_TRUNK:.spine/ci.sh" >"$d/ci.sh"
          rc=0
          sh "$d/ci.sh" collect "$SPINE_TRUNK" "$SPINE_CANDIDATE" >"$d/out" || rc=$?
          echo "rc=$rc" >>"$GITHUB_OUTPUT"
          if [ "$rc" -eq 2 ]; then
            echo "spine: ci.sh refused; no result file" >&2; exit 1
          fi
          line="$(cat "$d/out")"
          case "$line" in
            result=*) ;;
            *) echo "spine: ci.sh printed no result line" >&2; exit 1 ;;
          esac
          h="$RUNNER_TEMP/spine-handoff"; mkdir -p "$h"; chmod 700 "$h"
          cp -- "${line#result=}" "$h/"

      - uses: actions/upload-artifact@PIN_UPLOAD_ARTIFACT
        if: always() && steps.collect.outputs.rc != ''
        with:
          name: spine-result
          path: ${{ runner.temp }}/spine-handoff
          if-no-files-found: error
          retention-days: 1

      - name: Report the collector's own verdict
        env:
          RC: ${{ steps.collect.outputs.rc }}
        run: |
          if [ "$RC" != "0" ]; then
            echo "spine: the collector exited $RC; the result file was handed over" >&2
            exit 1
          fi
```

### F-4 · `.github/workflows/spine-land.yml` (CI §7.3)

```yaml
# Rendered by `spine init --ci github` from template ci-github-land@N. spine-owned.
name: spine-land

on:
  workflow_run:
    workflows: ["spine-collect"]
    types: [completed]

permissions:
  actions: write
  contents: read

concurrency:
  group: spine-land
  cancel-in-progress: false

defaults:
  run:
    shell: bash --noprofile --norc -euo pipefail {0}

jobs:
  land:
    # Rule 0, as three provider facts a candidate cannot forge.
    if: >-
      github.event.workflow_run.event == 'pull_request_target' &&
      github.event.workflow_run.path == '.github/workflows/spine-collect.yml' &&
      github.event.workflow_run.head_repository.full_name == github.repository
    runs-on: ubuntu-latest
    environment: spine-trusted
    timeout-minutes: 120
    env:
      SPINE_TRUNK: "main"
      SPINE_CANDIDATE: ${{ github.event.workflow_run.head_branch }}
      SPINE_RUN_ID: ${{ github.event.workflow_run.id }}
      SPINE_TRUST_ROOT: ${{ vars.SPINE_TRUST_ROOT }}
      SPINE_PIPELINE_KEY: ${{ secrets.SPINE_PIPELINE_KEY }}
      SPINE_PUSH_TOKEN: ${{ secrets.SPINE_PUSH_TOKEN }}
    steps:
      - name: Refuse anything that is not a candidate ref
        run: |
          case "$SPINE_CANDIDATE" in
            intent/*|quick/*|spine/upgrade-*) ;;
            *) echo "spine: not a candidate ref: $SPINE_CANDIDATE" >&2; exit 1 ;;
          esac
          for v in SPINE_TRUST_ROOT SPINE_PIPELINE_KEY SPINE_PUSH_TOKEN; do
            eval "s=\${$v:-}"
            if [ -z "$s" ]; then echo "spine: $v is unset" >&2; exit 1; fi
          done

      - uses: actions/checkout@PIN_CHECKOUT
        with:
          ref: "main"
          fetch-depth: 0
          persist-credentials: false

      - name: Authenticate git without writing a credential to disk
        run: |
          b64="$(printf 'x-access-token:%s' "$SPINE_PUSH_TOKEN" | base64 | tr -d '\n')"
          echo "::add-mask::$b64"
          {
            echo "GIT_CONFIG_COUNT=1"
            echo "GIT_CONFIG_KEY_0=http.${{ github.server_url }}/.extraheader"
            echo "GIT_CONFIG_VALUE_0=AUTHORIZATION: basic $b64"
          } >>"$GITHUB_ENV"

      - name: Fetch trunk and every in-flight candidate ref
        run: |
          git fetch --no-tags --prune origin \
            "+refs/heads/$SPINE_TRUNK:refs/remotes/origin/$SPINE_TRUNK" \
            "+refs/heads/intent/*:refs/heads/intent/*" \
            "+refs/heads/quick/*:refs/heads/quick/*" \
            "+refs/heads/spine/upgrade-*:refs/heads/spine/upgrade-*"

      - name: Install the pinned release
        id: spine
        run: |
          d="$RUNNER_TEMP/spine"; mkdir -p "$d"; chmod 700 "$d"
          git show "origin/$SPINE_TRUNK:.spine/ci.sh" >"$d/ci.sh"
          SPINE_INSTALL_DIR="$d/bin" sh "$d/ci.sh" install "$SPINE_TRUNK" >"$d/path"
          echo "bin=$(cat "$d/path")" >>"$GITHUB_OUTPUT"

      - uses: actions/download-artifact@PIN_DOWNLOAD_ARTIFACT
        with:
          name: spine-result
          path: .spine/cache/results
          run-id: ${{ github.event.workflow_run.id }}
          github-token: ${{ github.token }}

      - name: Check the shape of the handoff
        run: |
          n="$(find .spine/cache/results -mindepth 1 | wc -l | tr -d ' ')"
          if [ "$n" != "1" ]; then
            echo "spine: the artifact holds $n entries, expected 1" >&2; exit 1
          fi
          f="$(find .spine/cache/results -mindepth 1)"
          if [ ! -f "$f" ] || [ -L "$f" ]; then
            echo "spine: the artifact entry is not a regular file" >&2; exit 1
          fi
          case "${f##*/}" in
            *.jsonl) ;;
            *) echo "spine: the artifact entry is not a .jsonl file" >&2; exit 1 ;;
          esac

      - name: Land
        id: land
        env:
          SPINE_BIN: ${{ steps.spine.outputs.bin }}
        run: |
          case "$SPINE_CANDIDATE" in
            quick/reseal-*)  set -- --land --reseal ;;
            intent/*)        set -- --land "${SPINE_CANDIDATE#intent/}" ;;
            quick/*)         set -- --land --quick "$SPINE_CANDIDATE" ;;
            spine/upgrade-*) set -- --land ;;
            *) echo "spine: unroutable candidate" >&2; exit 3 ;;
          esac
          rc=0
          "$SPINE_BIN" check --ci "$@" </dev/null || rc=$?
          echo "rc=$rc" >>"$GITHUB_OUTPUT"

      - uses: actions/upload-artifact@PIN_UPLOAD_ARTIFACT
        if: always() && steps.land.outputs.rc != ''
        with:
          name: spine-report
          path: .spine/cache/report.json
          if-no-files-found: warn
          retention-days: 90

      - name: Re-queue on base-moved
        if: steps.land.outputs.rc == '2'
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          gh api --method POST \
            "repos/${GITHUB_REPOSITORY}/actions/runs/${SPINE_RUN_ID}/rerun"

      - name: Report the landing's verdict
        env:
          RC: ${{ steps.land.outputs.rc }}
        run: |
          if [ "$RC" != "0" ]; then
            echo "spine: check --ci --land exited $RC (see docs/spec/ci.md 6.6)" >&2
            exit 1
          fi
```

### F-5 · `.gitlab-ci.yml` (CI §8.2)

```yaml
# Rendered by `spine init --ci gitlab` from template ci-gitlab@N. spine-owned.
# Requires GitLab >= 15.0 for variable expansion in `include:project`.
include:
  - project: '$CI_PROJECT_PATH'
    ref: 'main'
    file:
      - '/.spine/gitlab/untrusted.yml'
      - '/.spine/gitlab/trusted.yml'

stages:
  - spine
```

### F-6 · `.spine/gitlab/untrusted.yml` (CI §8.3)

```yaml
spine-collect:
  stage: spine
  interruptible: true
  rules:
    - if: '$CI_PIPELINE_SOURCE == "merge_request_event"
           && $CI_MERGE_REQUEST_TARGET_BRANCH_NAME == "main"
           && $CI_COMMIT_REF_NAME =~ /^(intent\/|quick\/|spine\/upgrade-)/'
    - when: never
  variables:
    GIT_STRATEGY: clone
    GIT_DEPTH: 0
    SPINE_TRUNK: "main"
  script:
    - |
      set -eu
      git fetch --no-tags --prune origin \
        "+refs/heads/$SPINE_TRUNK:refs/remotes/origin/$SPINE_TRUNK" \
        "+refs/heads/$CI_COMMIT_REF_NAME:refs/heads/$CI_COMMIT_REF_NAME"
      git checkout -q "$CI_COMMIT_REF_NAME"
      d="$(mktemp -d)"
      git show "origin/$SPINE_TRUNK:.spine/ci.sh" >"$d/ci.sh"
      rc=0
      sh "$d/ci.sh" collect "$SPINE_TRUNK" "$CI_COMMIT_REF_NAME" >"$d/out" || rc=$?
      if [ "$rc" -eq 2 ]; then echo "spine: ci.sh refused" >&2; exit 1; fi
      line="$(cat "$d/out")"
      case "$line" in result=*) ;; *) echo "spine: no result line" >&2; exit 1 ;; esac
      echo "spine: handing over ${line#result=}" >&2
      exit "$rc"
  artifacts:
    when: always
    expire_in: 1 day
    paths:
      - ".spine/cache/results/"
```

*"GitLab preserves the path for free. Artifacts are zipped relative to the project directory, so the
entry is literally `.spine/cache/results/<T>.jsonl`, and gitignored files are collected like any other.
No staging copy is needed and none is made."* (CI §8.3)

### F-7 · `.spine/gitlab/trusted.yml` (CI §8.4)

```yaml
spine-land:
  stage: spine
  interruptible: false
  resource_group: spine-land
  rules:
    - if: '$CI_PIPELINE_SOURCE == "schedule" && $CI_COMMIT_REF_NAME == "main"'
    - when: never
  variables:
    GIT_STRATEGY: clone
    GIT_DEPTH: 0
    SPINE_TRUNK: "main"
  script:
    - |
      set -eu
      command -v jq >/dev/null || { echo "spine: jq is required" >&2; exit 3; }
      for v in SPINE_TRUST_ROOT SPINE_PIPELINE_KEY SPINE_PUSH_TOKEN; do
        eval "s=\${$v:-}"
        [ -n "$s" ] || { echo "spine: $v is unset (protected variable?)" >&2; exit 3; }
      done

      api="$CI_API_V4_URL/projects/$CI_PROJECT_ID"
      hdr="PRIVATE-TOKEN: $SPINE_PUSH_TOKEN"

      git remote set-url origin \
        "$(printf '%s' "$CI_REPOSITORY_URL" | sed "s#://[^@]*@#://spine:$SPINE_PUSH_TOKEN@#")"
      git fetch --no-tags --prune origin \
        "+refs/heads/$SPINE_TRUNK:refs/remotes/origin/$SPINE_TRUNK" \
        "+refs/heads/intent/*:refs/heads/intent/*" \
        "+refs/heads/quick/*:refs/heads/quick/*" \
        "+refs/heads/spine/upgrade-*:refs/heads/spine/upgrade-*"

      cand=''; jobid=''
      for ref in $(git for-each-ref --format='%(refname:short)' \
                     refs/heads/intent refs/heads/quick refs/heads/spine \
                   | LC_ALL=C sort); do
        sha="$(git rev-parse "refs/heads/$ref")"
        pid="$(curl -fsS -H "$hdr" \
               "$api/pipelines?sha=$sha&source=merge_request_event&per_page=1" \
               | jq -r '.[0].id // empty')"
        [ -n "$pid" ] || continue
        jid="$(curl -fsS -H "$hdr" "$api/pipelines/$pid/jobs?per_page=100" \
               | jq -r '[.[] | select(.name=="spine-collect")][0].id // empty')"
        [ -n "$jid" ] || continue
        cand="$ref"; jobid="$jid"; break
      done
      [ -n "$cand" ] || { echo "spine: no candidate with a result artifact" >&2; exit 0; }

      mkdir -p .spine/cache
      curl -fsS -H "$hdr" -o "$CI_PROJECT_DIR/art.zip" \
        "$api/jobs/$jobid/artifacts"
      rm -rf .spine/cache/results
      unzip -qq -o "$CI_PROJECT_DIR/art.zip" '.spine/cache/results/*' -d .
      rm -f "$CI_PROJECT_DIR/art.zip"
      n="$(find .spine/cache/results -mindepth 1 | wc -l | tr -d ' ')"
      [ "$n" = "1" ] || { echo "spine: artifact holds $n entries" >&2; exit 1; }
      f="$(find .spine/cache/results -mindepth 1)"
      { [ -f "$f" ] && [ ! -L "$f" ]; } || { echo "spine: not a regular file" >&2; exit 1; }
      case "${f##*/}" in *.jsonl) ;; *) echo "spine: not .jsonl" >&2; exit 1 ;; esac

      d="$(mktemp -d)"
      git show "origin/$SPINE_TRUNK:.spine/ci.sh" >"$d/ci.sh"
      bin="$(SPINE_INSTALL_DIR="$d/bin" sh "$d/ci.sh" install "$SPINE_TRUNK")"

      case "$cand" in
        quick/reseal-*)  set -- --land --reseal ;;
        intent/*)        set -- --land "${cand#intent/}" ;;
        quick/*)         set -- --land --quick "$cand" ;;
        spine/upgrade-*) set -- --land ;;
        *) echo "spine: unroutable candidate $cand" >&2; exit 3 ;;
      esac
      rc=0
      "$bin" check --ci "$@" </dev/null || rc=$?
      echo "spine: check --ci --land exited $rc (docs/spec/ci.md 6.6)" >&2
      [ "$rc" -eq 0 ] || exit 1
  artifacts:
    when: always
    expire_in: 90 days
    paths:
      - ".spine/cache/report.json"
```

### F-8 · The `generic` contract, printed by `init --ci generic` (CI §9.4)

**Job A — untrusted.** *"Runs on `intent/*`, `quick/*`, `spine/upgrade-*`. Definition stored outside the
repository, pinned to trunk. No secret of any kind; no `SPINE_PIPELINE_KEY`, `SPINE_PUSH_KEY` or
`SPINE_PUSH_TOKEN` in its environment; read-only repository credentials."*

```sh
#!/bin/sh
# Job A — the untrusted half.  Runs nothing from the checkout before ci.sh.
set -eu
TRUNK=main
CANDIDATE="$1"          # intent/INT-042 | quick/<name> | spine/upgrade-<v>
export SPINE_TRUST_ROOT   # a variable, not a secret

case "$CANDIDATE" in
	intent/* | quick/* | spine/upgrade-*) : ;;
	*) echo "not a candidate ref: $CANDIDATE" >&2; exit 1 ;;
esac

git init -q .
git remote add origin "$REPO_URL"
git fetch -q --no-tags --prune origin \
	"+refs/heads/$TRUNK:refs/remotes/origin/$TRUNK" \
	"+refs/heads/$CANDIDATE:refs/heads/$CANDIDATE"
git checkout -q "$CANDIDATE"

d="$(mktemp -d)"
git show "origin/$TRUNK:.spine/ci.sh" >"$d/ci.sh"
rc=0
sh "$d/ci.sh" collect "$TRUNK" "$CANDIDATE" >"$d/out" || rc=$?
[ "$rc" -ne 2 ] || { echo "ci.sh refused" >&2; exit 1; }

line="$(cat "$d/out")"
case "$line" in result=*) : ;; *) echo "no result line" >&2; exit 1 ;; esac

# Hand over exactly one file, at exactly this repo-relative path.
publish_artifact "${line#result=}"     # provider-specific; one file, no extras
exit "$rc"
```

**Job B — trusted.** *"Runs only on trunk, from a definition outside the repository, with the environment
restricted to trunk."*

```sh
#!/bin/sh
# Job B — the trusted half.  Executes no repository code.
set -eu
TRUNK=main
CANDIDATE="$1"
export SPINE_TRUST_ROOT SPINE_PIPELINE_KEY SPINE_PUSH_TOKEN

git init -q .
git remote add origin "$REPO_URL"
git fetch -q --no-tags --prune origin \
	"+refs/heads/$TRUNK:refs/remotes/origin/$TRUNK" \
	"+refs/heads/intent/*:refs/heads/intent/*" \
	"+refs/heads/quick/*:refs/heads/quick/*" \
	"+refs/heads/spine/upgrade-*:refs/heads/spine/upgrade-*"
git checkout -q -B "$TRUNK" "refs/remotes/origin/$TRUNK"

mkdir -p .spine/cache/results
fetch_artifact_into .spine/cache/results      # provider-specific
n="$(find .spine/cache/results -mindepth 1 | wc -l | tr -d ' ')"
[ "$n" = 1 ] || { echo "artifact holds $n entries" >&2; exit 1; }
f="$(find .spine/cache/results -mindepth 1)"
{ [ -f "$f" ] && [ ! -L "$f" ]; } || { echo "not a regular file" >&2; exit 1; }

d="$(mktemp -d)"
git show "origin/$TRUNK:.spine/ci.sh" >"$d/ci.sh"
bin="$(SPINE_INSTALL_DIR="$d/bin" sh "$d/ci.sh" install "$TRUNK")"

case "$CANDIDATE" in
	quick/reseal-*)  set -- --land --reseal ;;
	intent/*)        set -- --land "${CANDIDATE#intent/}" ;;
	quick/*)         set -- --land --quick "$CANDIDATE" ;;
	spine/upgrade-*) set -- --land ;;
	*) echo "unroutable candidate: $CANDIDATE" >&2; exit 3 ;;
esac
rc=0
"$bin" check --ci "$@" </dev/null || rc=$?
publish_artifact .spine/cache/report.json     # keep the report; see 6.5
[ "$rc" -eq 0 ] || exit 1
```

### F-9 · The GitHub environment (CI §7.4)

`spine-trusted`, with **Deployment branches and tags → Selected branches → `<trunk>` only**. *"The
pipeline key, the push credential and nothing else are environment secrets there."* `SPINE_TRUST_ROOT` is
a repository **variable**, not a secret.

---

## Error cases

### E-1 · `spine init` (render time)

| Condition | Behaviour | Code / token |
|---|---|---|
| No embedded release manifest, or one the DM-1 schema refuses, or an unknown `release_manifest_version` | Development build: renders no CI definition, writes no manifest, creates no path; **every plan row `REFUSE`** | diagnostic `no-release-manifest` (CI §3.4) |
| A rendered CI file still contains `@@` (two `U+0040`, any context) | **Whole plan `REFUSE`, nothing written** | `unsubstituted-token` (CI §3.4) |
| A rendered CI file still contains `PIN_CHECKOUT`, `PIN_UPLOAD_ARTIFACT` or `PIN_DOWNLOAD_ARTIFACT` | **Whole plan `REFUSE`, nothing written** | `unsubstituted-token` (CI §3.4) |
| `--trunk` names a branch containing `@@` or equal to a `PIN_` literal | Refuse at `--trunk` | `trunk-name-collides-with-token` (CI §3.4) |
| Existing manifest carries such a trunk name | Refusal fires at the scan instead; tree untouched | `unsubstituted-token` (CI §3.4) |
| `--ci github` and `params.trunk` ≠ default branch | `init` refuses | (CI §7.1, §14 R13) |
| `--ci gitlab` or `--ci generic` with `merge.auto = on` | `init` refuses to **write** it | (CI §8.1, §9.3) |
| A development build meets an already-initialised repository | No new rule: **G15 disposes of it** — its platform artifact is in no release's list | G15 (CI §3.4, PB §6.3) |

### E-2 · `.spine/ci.sh` — every refusal, with its exact message (all exit **2**)

| Condition | Message (verbatim) |
|---|---|
| Sanitised `PATH` empty | `PATH contains no absolute directory` |
| No SHA-256 utility | `no SHA-256 utility: need sha256sum, shasum or openssl` |
| No HTTPS client | `no HTTPS client: need curl or wget` |
| `json_one` finds no member | `manifest: no "<key>" member` |
| `json_one` finds several | `manifest: "<key>" occurs more than once` |
| Fewer than two arguments | `usage: ci.sh install <trunk> \| ci.sh collect <trunk> <candidate-ref>` |
| Unknown mode | `unknown mode: <MODE>` |
| `collect` with ≠ 3 arguments | `collect needs a candidate ref` |
| `collect` sees a credential variable set | `rule 0: <VAR> is visible to the untrusted job` (for `SPINE_PIPELINE_KEY`, `SPINE_PUSH_KEY`, `SPINE_PUSH_TOKEN`, in that probe order) |
| `collect` without a trust root | `SPINE_TRUST_ROOT is unset; spine check --ci refuses to run without it` |
| `git` absent | `git not found` |
| Unparseable git version | `cannot parse git version: <_gv>` |
| git < 2.38 | `git >= 2.38 required (merge-tree --write-tree); found <_gv>` |
| Not inside a repository | `not inside a git repository` |
| Bad trunk / candidate name | `not a branch name: <name>` |
| `mktemp -d` fails | `mktemp failed` |
| `chmod 0700 $WORK` fails | `cannot restrict <WORK>` |
| Trunk manifest unreadable | `cannot read origin/<TRUNK>:.spine/manifest.json` |
| Manifest trunk ≠ argument | `manifest names trunk "<MANIFEST_TRUNK>", invoked with "<TRUNK>"` |
| `dist_hash` not `sha256:<hex>` | `manifest: cli.dist_hash is not sha256:<hex>` |
| `dist_hash` not lowercase hex | `manifest: cli.dist_hash is not lowercase hex` |
| `dist_hash` not 64 digits | `manifest: cli.dist_hash is not 64 hex digits` |
| `DIST_BASE` not `https://` | `SPINE_DIST_BASE must be an https:// URL` |
| Unsupported architecture | `unsupported architecture: <os>/<arch>` |
| Unsupported OS | `unsupported platform: <os> (v1 ships no Windows CI target)` |
| `chmod 0755 $INSTALL_DIR` fails | `cannot make <INSTALL_DIR> traversable` |
| Artifact list unfetchable | `cannot fetch the release artifact list` |
| List digest ≠ pin | `artifact list hash <got> does not equal the pinned <DIST_HASH>` |
| No artifact for this target | `the pinned release publishes no artifact for <TARGET>` |
| Two artifacts for this target | `the pinned release publishes more than one artifact for <TARGET>` |
| Artifact unfetchable | `cannot fetch <ART_NAME>` |
| Artifact digest ≠ listed | `<ART_NAME> hash <got> does not equal the listed <ART_SHA>` |
| `gzip` / `tar` absent | `gzip not found` · `tar not found` |
| Unpack fails | `cannot unpack <ART_NAME>` |
| No `spine` at archive root | `<ART_NAME> contains no spine binary at its root` |
| `SPINE_REGISTRY_PROXY` not `https://` | `SPINE_REGISTRY_PROXY must be an https:// URL` |
| `origin/<trunk>` not fetched | `refs/remotes/origin/<TRUNK> is not fetched` |
| Candidate not fetched | `refs/heads/<CANDIDATE> is not fetched` |
| HEAD not on the candidate (incl. detached) | `HEAD is not on <CANDIDATE> (the collector reads H from HEAD)` |
| `merge-tree` conflict | stderr of merge-tree, then `merge-tree reports a conflict: needs-rebase` |
| `merge-tree` gave no tree id | `merge-tree produced no tree object id` |
| Collector wrote no file | `the collector wrote no result file at <RESULT>` |
| `INT`/`TERM`/`HUP` | cleanup, then **exit 2** |

Non-refusal exits: collector ran and failed **with** a file ⇒ print `result=…` then **exit 1**; success ⇒
print `result=…` then **exit 0**; `install` ⇒ print the path, **exit 0**.

### E-3 · The rendered definitions

| Condition | Behaviour | Code / message |
|---|---|---|
| GitHub collect: head repo ≠ this repo | fail the job before checkout | `spine: refusing a head outside $SPINE_THIS_REPO`, exit 1 |
| GitHub collect: ref is not `intent/*`, `quick/*`, `spine/upgrade-*` | fail | `spine: not a candidate ref: $SPINE_CANDIDATE`, exit 1 |
| GitHub collect: `vars.SPINE_TRUST_ROOT` unset | fail | `spine: vars.SPINE_TRUST_ROOT is unset`, exit 1 |
| GitHub collect: a credential survived in `.git/config` | fail | `spine: a credential survived in .git/config`, exit 1 |
| GitHub collect: `ci.sh` exit 2 | fail, upload nothing to hand over | `spine: ci.sh refused; no result file`, exit 1 |
| GitHub collect: stdout has no `result=` line | fail | `spine: ci.sh printed no result line`, exit 1 |
| GitHub collect: `rc != 0` after upload | fail | `spine: the collector exited $RC; the result file was handed over`, exit 1 |
| GitHub land: `if:` clauses not all true | job is skipped entirely | (CI §7.3) |
| GitHub land: ref not a candidate ref | fail | `spine: not a candidate ref: $SPINE_CANDIDATE`, exit 1 |
| GitHub land: `SPINE_TRUST_ROOT`/`SPINE_PIPELINE_KEY`/`SPINE_PUSH_TOKEN` unset | fail | `spine: $v is unset`, exit 1 |
| Handoff holds ≠ 1 entry | fail | `spine: the artifact holds $n entries, expected 1`, exit 1 |
| Handoff entry not a regular file / is a symlink | fail | `spine: the artifact entry is not a regular file`, exit 1 |
| Handoff entry not `*.jsonl` | fail | `spine: the artifact entry is not a .jsonl file`, exit 1 |
| GitHub land: unroutable candidate | fail | `spine: unroutable candidate`, **exit 3** |
| GitHub land: `rc != 0` | fail | `spine: check --ci --land exited $RC (see docs/spec/ci.md 6.6)`, exit 1 |
| GitLab trusted: `jq` missing | fail | `spine: jq is required`, **exit 3** |
| GitLab trusted: a required variable unset | fail | `spine: $v is unset (protected variable?)`, **exit 3** |
| GitLab trusted: no candidate has a result artifact | **succeed and do nothing** | `spine: no candidate with a result artifact`, **exit 0** |
| GitLab trusted: artifact shape wrong | fail | `spine: artifact holds $n entries` · `spine: not a regular file` · `spine: not .jsonl`, exit 1 |
| GitLab trusted: unroutable candidate | fail | `spine: unroutable candidate $cand`, **exit 3** |
| generic Job A: not a candidate ref | fail | `not a candidate ref: $CANDIDATE`, exit 1 |
| generic Job A: `ci.sh` refused | fail | `ci.sh refused`, exit 1 |
| generic Job A: no result line | fail | `no result line`, exit 1 |
| generic Job B: artifact shape wrong | fail | `artifact holds $n entries` · `not a regular file`, exit 1 |
| generic Job B: unroutable candidate | fail | `unroutable candidate: $CANDIDATE`, **exit 3** |

### E-4 · Landing outcomes

DM-8's second table is the whole vocabulary: `landed` 0 · `blocked` 1 · `base-moved` 2 · `refused` 3 ·
`reconstruction-failed` 4 · `note-publish-failed` 5 (CI §6.6). Trusted-stage seal-key selection: **zero
matches or several under `spine-seal@v1` ⇒ refuse, exit 3** (CI §4).

---

## Worked examples / test vectors

### V-1 · The artifact list and its `dist_hash` (MF §8.2, adopting CI §5.5)

```
f0ed236cfc75bb491003477b77cbd09b76f80420f546b585e2a16ee85ad989ae  spine-1.4.0-aarch64-apple-darwin.tar.gz
ce946375b5e89e3e5546d7563ef8a539c5c62828125c851220edf74578dfb167  spine-1.4.0-aarch64-unknown-linux-musl.tar.gz
40627734cff1df388697c03a037273fb6693cfa5ba594e4cbf85db44ef626bbb  spine-1.4.0-py3-none-any.whl
2d90a2ef987219f1df0ac40b08fd853156b0500e3f31177a1bd701bc4f618977  spine-1.4.0-x86_64-apple-darwin.tar.gz
48f5f6e485b72cc4e848a488256435ffcb6025c0f401ae211136d8c34577c1ec  spine-1.4.0-x86_64-unknown-linux-musl.tar.gz
```

**529 bytes.** `sha256` = `6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db`, so

```
cli.dist_hash = "sha256:6f49644fdd3009155fe32ab46b9da846b6645f52a15eb3aa44234c02b1c744db"
```

*"and the list lives at `<SPINE_DIST_BASE>/6f49644f…744db/artifacts.txt`."* (MF §8.2)
Note the ordering: the `.whl` sorts between `aarch64-unknown-linux-musl` and `x86_64-apple-darwin`,
because the sort is over **artifact-name bytes**, not over target or kind. Command: `shasum -a 256
artifacts.txt` (MF §8.7).

### V-2 · `.spine/ci.sh` digests (CI §5.3, README digest table)

319 lines · `git hash-object` `131f13fb0312162579605999d3f9f4e90098c74c` ·
`sha256:d6bcf50cf675614033aaef61df104aad253d30c4accc756719599ad5bd41060b`, over the **unsubstituted**
bytes of F-1.

**Verified by execution, not by reading** (CI §5.3) — the conformance suite an implementation should
reproduce:

- syntax-checked under `sh`, `dash`, `bash` and `zsh`;
- a clean `collect` run printed one `result=` line; **exit 1** on a failing collector;
- a conflicting merge **exit 2** with `needs-rebase`;
- `SPINE_PIPELINE_KEY=1` **exit 2 at the probe, before anything was fetched**;
- a HEAD not on the candidate **exit 2**; an unfetched trunk **exit 2**;
- `json_one` exercised against a manifest containing an adversarial `files[]` path spelled
  `weird, {trunk}: "x"` and **returned `main`**;
- after the umask narrowing: `install` and `collect` both run end to end against a stubbed distribution
  served from a local tree; **`$WORK` is `drwx------` while the run holds it**; the install directory and
  the binary come out **`drwxr-xr-x`** and **`-rwxr-xr-x`**.

### V-3 · The end-to-end GitHub landing (CI §12)

Repository: `params.ci: "github"`, `params.langs: ["python","ts"]`, `params.isolation: "container"`,
`params.timeout: 1800`, `params.trunk: "main"`, `object_format: sha1`, pinned `cli.version 1.4.0`,
`C-A1: team`, `C-A3: hostile`, `C-M4: off`, `C-M1: merge`.

Untrusted run, verbatim invocation:

```
git show "origin/main:.spine/ci.sh" >"$RUNNER_TEMP/spine/ci.sh"
sh "$RUNNER_TEMP/spine/ci.sh" collect main intent/INT-042
```

`ci.sh` sanitises `PATH`, finds no `SPINE_*` credential set, reads `SPINE_TRUST_ROOT`, checks git 2.45 ≥
2.38, reads `origin/main:.spine/manifest.json`, confirms `params.trunk == "main"`, takes `dist_hash`
`9f2e…`, fetches `<base>/9f2e…/artifacts.txt`, finds its SHA-256 equal to `9f2e…`, selects the one
`x86_64-unknown-linux-musl` line, fetches and verifies that artifact, unpacks it, exports the allowlist,
computes

```
T = 3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28
```

runs `spine check --ci --collect` (twenty-line file of RF §10, `end.status: complete`, exit 0), and
prints exactly

```
result=.spine/cache/results/3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28.jsonl
```

The job copies that file to `$RUNNER_TEMP/spine-handoff/` and uploads it as `spine-result` — **one file,
no extras**.

Trusted run: `spine-land` fires on `workflow_run`; the three `if:` clauses pass; the `spine-trusted`
environment releases the two secrets because the job runs on `main`; the artifact lands at exactly
`.spine/cache/results/3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28.jsonl`; the shape check finds one regular
`.jsonl` file; `spine check --ci --land INT-042` runs. `tool=` equals `1.4.0+sha256:9f2e…`. Precondition 1
holds; **precondition 2 holds — including the third conjunct, which "the three `if:` clauses are"**;
precondition 0 fails (`C-A3: hostile`), so `C-M4` evaluates `off` and **one bare `class=tripwire` G11
wire** is raised. After the review, step 6 pushes `L` and deletes `refs/heads/intent/INT-042` atomically
with two `--force-with-lease` guards using `SPINE_PUSH_TOKEN`; **then** the note is published; `--land`
exits 0; `.spine/cache/report.json` is uploaded as `spine-report`.

Variation (CI §12): had another intent landed between the collector's run and step 1, `T` would differ,
ingestion step 1 returns `base-moved`, `--land` exits 2, the re-queue step re-runs `spine-collect` for run
`$SPINE_RUN_ID`, *"and the earlier file is never consulted again by this run or any other."*

### V-4 · Manifest rows for the rendered CI files (MF §8.3/§8.6, PB §6.7)

`.spine/ci.sh` → `"template": "ci-generic@4"`, `spine-owned`, blob `dc189372…` (rendered) — the `@3`
render of the same path carries blob `d61e31f1a8d0130fb53241f89296ea89c2288677`.
`.github/workflows/spine-collect.yml` → `ci-github-collect@4`, blob `e7f192f8…` (`@3`:
`081136631faa5fca86793d3b940b5bd83952c55a`).
`.github/workflows/spine-land.yml` → `ci-github-land@4`, `user-modified` with
`base 4275e9df2ca6f096909f49fc8142fd87341abc07` (180 bytes, the pristine 1.4.0 render) and tree blob
`e85fcdd4…`. `templates` carries all twelve keys at `4` for the four CI templates.

### V-5 · Conformance checklist (CI §13) — the acceptance test for this sheet

1. `.spine/ci.sh` byte-identical to F-1 with the render tokens substituted; **no `@@` or `PIN_` token
   survives a render**; the scan runs before any path is written; a build with no release manifest
   renders nothing and refuses the plan whole.
2. Executed from `git show "origin/<trunk>:.spine/ci.sh"`, to a file outside the working tree, **never
   piped into `sh -s`**.
3. stdout exactly one line on success; exit codes exactly the three of §5.2.
4. Artifact list is the sha256sum-format bytes of §5.5, sorted by artifact name; `dist_hash` is its
   SHA-256; platform table is §5.5's; **zero or two artifacts for a target is a refusal**.
5. `collect` refuses when any of the three credential variables is set, and without `SPINE_TRUST_ROOT`;
   **it never strips them**.
6. U1–U7 and T1–T9 hold.
7. One result file; materialized at `.spine/cache/results/<T>.jsonl`; shape checked.
8. Ref mapping is §6.4's with `quick/reseal-*` first.
9. Report published to `refs/notes/spine` after the CAS; a failed publish fails the job and retracts
   nothing.
10. `--land` exit codes and the definition's reaction are §6.6's.
11. **No candidate-controlled string is interpolated into a shell script by the CI templating language.**
12. The trusted job restores no cache, runs no repository code, pushes with a credential the untrusted
    job does not hold.
13. Nothing records a time, compares two, or derives a decision from one.
14. `init --ci gitlab` and `init --ci generic` refuse `merge.auto = on`; precondition 2 is `"unmet"` on
    every run under both.

**Determinism claim (CI §13):** *"For a fixed release, a fixed trunk tip and a fixed candidate tip,
`.spine/ci.sh` is a deterministic function of `(trunk manifest, platform, network availability)` up to the
collector it invokes, whose own determinism is RF §4.5's and is conditioned on
`end.status == complete`."*

---

## Cross-references it depends on

| Owned elsewhere | Where | What this sheet assumes |
|---|---|---|
| Result-file format, header fields, `keys_visible=`, `profile=`, ingestion order | RF §3, §4.2, §7.1, §8.1, §8.3, §8.4 | The collector writes one file at `.spine/cache/results/<T>.jsonl`; M1's namespaces incl. the **network namespace**; **P4** licenses `profile=container`; the **restore phase** is the collector's, from `origin/<trunk>:.spine/restore.sh`; precondition 2's three conjuncts |
| Gate-report canonical bytes, note ref/object/content, write commands, republication, concurrency | GR §2, §4, §4.4–§4.4.4, §5.8 | This sheet fixes only *where in the job* publication happens and what failure costs |
| `.spine/manifest.json` grammar, `cli.version` / `cli.dist_hash` domains, twelve `templates` keys, reserved member names, G13/G14/G16 | MF §3.2, §3.3, §3.6, §3.10, §5.5, §6.2 | `json_one`'s two members exist and are unambiguous; check 7 joins `files[].template` to `templates` at the same version; `reserved-member-name` protects the extractor |
| Constitution rules `C-T1`/`C-T2`, `C-A3`, `C-M1`, `C-M4`, `C-M3` | CN §6.2, §6.4 | Rendered from `params.langs` by **their** template, not by the CI substitution table |
| Intent templates and `Template: <variant>@<n>` | TM, ID | Untouched by the CI render; no `@@`/`PIN_` token, not scanned |
| Runner tokens and per-language resolvers | IR §11.1, §12 | No CI definition names a runner or a language; `gradle` is reserved and emitted by nothing |
| Floor globs and owner classes | PB §7.3, §6.7 | Every path in DM-3 is floor; `spine-owned` upgrade refuses a changed blob |

Sheets that must own what this one only cites: the result-file sheet (collector behaviour, isolation,
restore phase, precondition 2), the gate-report sheet (report bytes and note), the manifest sheet
(schema, G14/G16, template map).

---

## OPEN items

Owner questions, verbatim in substance, never invented (CI §18):

1. **OPEN-1 · The distribution root — the value, and only the value.** The layout
   `<base>/<dist_hash>/artifacts.txt`, the list's bytes, and where the root is carried
   (`release/release.json`'s `dist_base`, `https://`, no userinfo/query/fragment, no trailing `/`) are
   fixed; **the host is the owner's**. *"Until this is chosen no release manifest can be frozen and no
   binary renders a CI definition"* (CI §18 OPEN-1, §3.4).
2. **OPEN-2 · GitHub's untrusted trigger: `pull_request_target`, or a push dispatcher chained by
   `workflow_run`?** The second rests on whether a `workflow_run`-triggered workflow can itself trigger a
   second `workflow_run` workflow, *"which this document could not verify"* (CI §18 OPEN-2, §7.1).
3. **OPEN-3 · Whether `params.ci` is floor-relevant in G16's monotone sense.** Also filed as RF OPEN-7 and
   MF OPEN-1; *"the three are one question and must be decided once"*, and MF owns G16's check list
   (CI §18 OPEN-3; README *Known gaps*).
4. **OPEN-4 · A Windows CI target.** Refused in v1; supporting it needs a `.zip` container, an `.exe`
   suffix, a `uname` match for MSYS/MinGW/Cygwin, and an answer to PB §7.1's Windows agent-pipe residual
   (CI §18 OPEN-4, §5.5).
5. **OPEN-5 · A republish path for a failed note push.** Three ways out: a
   `--publish-note <landing-sha> --report <path>` flag; `--land` republishing idempotently; or GR §4.4.2's
   two commands over the T9 artifact, *"which is where v1 stands"* (CI §18 OPEN-5, §6.5, §15 D18).
6. **OPEN-6 · Whether GitLab-with-an-external-control-project deserves its own `params.ci` value.**
   Candidates named: a `gitlab-external` value, or a `params.ci_verified` boolean read by precondition 2's
   third conjunct. *"the only place in this document where the fail-closed answer is knowingly stricter
   than the arrangement warrants"* (CI §18 OPEN-6, §8.1).
7. **OPEN-7 · Which action versions the release pins — the values, and only the values.** The
   requirement — full 40-hex commit ids, never tags, one per `actions.<k>.commit` beside its `repo` — **is
   normative here**; the three commits are the owner's (CI §18 OPEN-7, §3.3, §3.4).

**Consequence for the build order:** with OPEN-1 and OPEN-7 unchosen, *"no binary built from this corpus
renders a CI definition at all"* (CI §3.4). An implementation can be complete and correct while `init`
refuses every plan with `no-release-manifest`; that is the specified behaviour, not a gap.

---

## Contradictions found

### C-1 · Live tensions this sheet must not paper over

| # | A says | B says | Status / disposition |
|---|---|---|---|
| 1 | PB §11 *Files and refs* names **two** CI variables, `SPINE_TRUST_ROOT` and `SPINE_PIPELINE_KEY` | CI §4 requires **five** — adding `SPINE_PUSH_TOKEN`, `SPINE_PUSH_KEY`, `SPINE_DIST_BASE` — and `ci.sh`'s probe needs the literal strings | **OPEN**, CI §15 D15 / §14 R19. PB §11 wins on vocabulary, so the amendment is *reported, not made*; an implementation still needs the three names because `ci.sh` refuses on them |
| 2 | PB §6.7's manifest example gives `.spine/ci.sh` the template `ci-generic@4` in a `params.ci: "github"` repository | Read as "the template for `--ci generic`", a GitHub repository writes no `ci.sh` and nothing executes the collector | **OPEN**, CI §15 D16. Resolution adopted here: `ci-generic` names the **provider-independent shell** |
| 3 | PB §7.3 calls `params.trunk` *"a rendering hint"* | CI §7.1 requires `params.trunk` = the provider's default branch on GitHub and makes `init` refuse otherwise | **OPEN**, CI §15 D6 / §14 R13 |
| 4 | PB §6.7 defines `dist_hash` as the SHA-256 of *"the release's artifact list"* and fixes **no** location, name or format | CI §5.5 fixes all three (content-addressed `<base>/<H>/artifacts.txt`, sha256sum bytes, sorted) | **OPEN**, CI §15 D7. MF §3.2 **adopts CI §5.5 as normative for the manifest** |
| 5 | PB §7.1 confines the untrusted stage's network to the dependency-restore phase | `ci.sh` must reach `SPINE_DIST_BASE` **before** that phase exists, to fetch the release it verifies | **OPEN**, CI §15 D19 / D11. Resolution: the distribution host is **not** on the registry allowlist and must not be |
| 6 | PB §11's CLI gives `--collect` no argument that could name a ref | PB §7.4 rule 3 says *"`H` is the ref the run names"* | **OPEN**, CI §15 D9 / §14 R7. Resolution: `H` = `git symbolic-ref HEAD`, detached HEAD refuses |
| 7 | PB §7.4 rule 1 requires policy from `origin/<trunk>` | Nothing says where trunk's **name** comes from before any policy is read | **OPEN**, CI §15 D14 / §14 R3. Resolution: positional argument, cross-checked against `params.trunk` |
| 8 | PB §7.5: *"`spine check --ci` refuses to run without"* a trust root | PB §7.1 grants the untrusted stage no variables | **OPEN**, CI §15 D13 / §14 R26. Resolution: **both** jobs, a variable and never a secret |
| 9 | PB §5.5 puts reseal reviews on `refs/heads/quick/reseal-<O>`; PB §11's CLI offers `--land --quick` and `--land --reseal` | Nothing orders the two matches | **OPEN**, CI §15 D22 / §14 R15. Resolution: `quick/reseal-*` **tested first** |
| 10 | PB §7.4 rule 3: *"the snippet re-queues the whole two-job run on the new `T`"* | On `--ci generic` spine writes no snippet; on GitLab the re-queue is the next schedule | **OPEN**, CI §15 D23 |
| 11 | PB §9 tells week one to *"set up the two-job CI skeleton"* and its roadmap step 0 says `init` writes *"the two-job CI snippet"* | On `generic`, `init` writes **no definition and cannot** | **OPEN**, CI §15 D12 / §9.1 |
| 12 | PB §7.4 rule 0 speaks of jobs that run on candidate-ref **pushes** | The only trunk-defined GitHub trigger for a branch is a pull-request event | **OPEN**, CI §15 D21 / §14 R23. Cost: a PR per candidate |
| 13 | PB §7.4 rule 4 makes publication non-optional | `--land` refuses an id already sealed on trunk, so a failed note push has **no remedy** and v1 has no command for one | **OPEN**, CI §15 D18, §18 OPEN-5 |
| 14 | PB §7.1's untrusted row: the runner is *"spawned by trunk's collector"* | The untrusted job's own definition must fetch, check out, invoke — and on GitHub stage and upload | **OPEN**, CI §15 D19 (first D19 entry) |
| 15 | PB §7.4 rule 0 states the requirement in GitHub's vocabulary (`permissions: contents: read`) | GitLab has no such key; no normative target exists for "no other secret" there | **OPEN**, CI §15 D20 |
| 16 | PB §5.4 step 5 clones the repository twice per landing in the trusted job | No CI-facing sentence tells an operator to size the job for it | **OPEN**, CI §15 D24 |
| 17 | PB §7.4 rule 0 offers GitLab *"a schedule that polls for candidates"* | Nothing defines discovery, ordering, or two ready candidates | **OPEN**, CI §15 D25; CI §8.4 fixes it |

### C-2 · Closed by PLAYBOOK.md v0.19 — kept so an implementer does not re-open them

| # | Was | Now |
|---|---|---|
| 18 | PB §11 named one GitHub workflow, `.github/workflows/spine.yml` | Two files, two names, two templates; `workflow_run` selects by `name:` so one file chains from its own completion (CI §3.2, §15 D2, PB §11) |
| 19 | `templates` listed `"ci-github": 4, "ci-generic": 4` — no `ci-gitlab` | **Twelve** keys, `ci-gitlab` among them, GitHub split into `ci-github-collect` / `ci-github-land` (CI §15 D1, MF §3.6) |
| 20 | *"a result file from a job that was not [trunk-defined] is never ingestible"* | Such a file **is ingested** and fails auto-merge precondition 2, which has **three** conjuncts (CI §15 D3, PB §7.4 rules 0 and 5, README decision 2) |
| 21 | PB §5.4 offered merge queues as configuration (a) while rule 5 precondition 4 defined them as (b) | PB §5.4 now says *"The queue serializes; it never creates the trunk commit"*; the shipped definitions use **no** queue (CI §15 D4, §7.5) |
| 22 | Rule 0 put the **untrusted** job on `merge_group` | *"**never `merge_group` for the untrusted job either**"* (CI §15 D5) |
| 23 | PB §11's `.spine/cache/` list had no slot for the gate report | `report.json` enumerated, and `--land --ci` **always** writes it (CI §15 D8, PB §11 CLI) |
| 24 | PB §7.1 promised a network allowlist a POSIX script cannot enforce | Narrowed: the *when* half is enforced (network namespace + **P4**); the *which hosts* half is declared (CI §15 D10, §5.6, RF §7.1) |

### C-3 · Two render hazards this corpus leaves for the implementer (not contradictions between documents, but places two implementations will diverge)

- **`@N` in three of the four printed templates.** CI §7.2, §7.3 and §8.2 print header comments reading
  `from template ci-github-collect@N`, `ci-github-land@N`, `ci-gitlab@N`, while CI §5.3's `ci.sh` prints a
  concrete `ci-generic@4`. CI §3.4 step 2 says the substitution table is *"exactly §3.3's rows and no
  others"*, and no row substitutes `N`. The shipped bytes must carry the concrete template version (the
  `files[].template` value, MF §3.6), so an implementation that copies the YAML verbatim ships a literal
  `@N` — and a template-version bump changes the file's bytes, hence its blob, hence the manifest record.
  The byte scan does **not** catch it: `@N` is neither `@@` nor a `PIN_` literal.
- **Two digests for one path.** CI §5.3's `131f13fb…` is over the **unsubstituted** `ci.sh` (with
  `@@DIST_BASE@@` intact); MF §8.3's `files[]` record for the same path carries `dc189372…`, the
  **rendered** blob. Both are correct; an implementation must not compare one to the other.

