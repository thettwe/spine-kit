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
