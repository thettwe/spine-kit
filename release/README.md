# Release inputs

## `release.json` — absent, and that is the shipped state

`ci.md` §3.4 defines the release manifest: a versioned file frozen into the
binary when the release is built, carrying the two things that cannot be known
until a release is cut — the distribution root and the three GitHub Action
commit pins.

**This directory has no `release.json`, so every ordinary build of this repo is
a _development build_.** CI §3.4: a development build "renders no CI definition,
writes no `.spine/manifest.json`, creates no path, and reports `REFUSE` for every
row of the plan … It does not fall back on a default host, a tag in place of a
commit, an empty string, or a rendered file with the token left in."

That is the correct behaviour and not a gap. Both values are the owner's and are
still open — `ci.md` §18 **OPEN-1** (the host `dist_base` names) and **OPEN-7**
(the three commits) — and §3.4 says why they are left open rather than invented:
"this document prints tokens rather than a hostname somebody would later have to
un-invent".

## `release.synthetic.json` — for exercising the paths a refusal cannot reach

Every value in it is deliberately **unusable**:

| Member | Value | Why it cannot be mistaken for real |
|---|---|---|
| `version` | `0.0.0-synthetic` | says so |
| `dist_base` | `https://dist.invalid/spine-synthetic` | `.invalid` is reserved by RFC 2606 and resolves nowhere, ever |
| `actions.*.commit` | `000…001`, `000…002`, `000…003` | 40 hex, so the schema accepts them; no such commit exists in any repository |

It is compiled in **only** under the `synthetic-release` cargo feature, which is
off by default:

```sh
cargo build                              # development build: init refuses every row
cargo build --features synthetic-release # init runs its apply path
```

A feature and not an environment variable, deliberately. CI §3.4 makes the
release manifest a build input read once and frozen: "Nothing at run time
re-reads it from disk, **so a repository cannot supply one and a candidate
cannot forge one**." A runtime override would hand a candidate the one input the
trusted-execution argument rests on.

**A binary built with this feature must never be published.** Its CI renders
point at a host that does not exist and pin actions that do not exist; the
artifact-list fetch fails closed, which is the safe direction, but a repository
initialised by it carries workflows nobody can run.
