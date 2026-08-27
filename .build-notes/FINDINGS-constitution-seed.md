# The `constitution@1` seed, as specified, does not parse

`constitution.md` §6.2 prints the twelve-rule block under the heading *"The
block `spine init` writes"*, and says of it:

> **These are the canonical bytes, and this is the only place they are fixed.**

Rendered as printed, the resulting `CONSTITUTION.md` has:

```
line 1: # The non-negotiables
line 2: (empty)
```

CN §3.1 requires both of those lines to be something else, and gives each its
own refusal:

> **Line 1 is the title line.** It must exist and must contain at least one byte
> after §2.3's preprocessing (`missing-title`). … `spine init` writes
> `# Constitution — <repo>`.
>
> **Line 2 is the header line** (§9.1). It must exist (`missing-header`).

So the seed takes **`missing-header`** on the first landing of every repository.
That is not a soft failure: the constitution is read by G4, by the indexer (the
`constitution` node, its `built_under` edges and its `protects` edges) and by
`spine check --constitution`, from the first landing onward. CN §3.1 even pins
the line number as a downstream dependency — *"`dump.md` §12.2 publishes the
`constitution` node's provenance as `git:<sha>:CONSTITUTION.md:2`, and it is
line 2 because this document says the header is line 2"*.

This is a documentation-scope defect rather than a design one — §3.1 and §9.1
each state the missing line correctly, and §6.2's block is plainly the *rules*
and not the *file*. But §6.2's sentence claims to be the only place the bytes
are fixed, and an implementer who takes it at its word ships a seed that
refuses. Three things are needed and only two exist.

---

## 1 · `<repo>` is named as a substitution and has no site

§6.2's own prose:

> `<repo>` is the manifest's `repo`; the `C-T1` and `C-T2` values are §6.4's
> function of `params.langs`; every other byte is fixed.

The printed block contains no `<repo>`. `grep` over the whole corpus finds the
token only in §3.1's title line, in §9.5's `built_under` example, and in §9.6's
node id — never in §6.2's block. So the sentence describes a three-span render
of which the block carries two.

**Resolution taken:** the render is §3.1's title line, then §9.1's header line,
then one blank line, then §6.2's block. `<repo>` is substituted into the title
line, which is what §3.1 says `spine init` writes.

---

## 2 · The seed's `Version:` value is stated nowhere

§9.1 makes `Version` mandatory, grammar `v` + a decimal integer `1 … 999`, no
leading zeros. §9.3 requires the version to change when the file changes. §9.2
insists it is not a clock.

No document in the corpus contains the bytes `Version: v1`. CN §12.1's worked
file is at `v3` — a lived-in constitution with four numbered rules a team has
added, and `C-A2` widened to `infra/` — so it is a **parse** vector, not a
render vector, and it fixes nothing about the seed.

**Resolution taken:** `v1`. It is the only value consistent with §9.3 on a file
that has not changed yet, and the only one consistent with §9.2's insistence
that the number is not a date.

---

## 3 · The seed's `Owner:` value has no source

§9.1 makes `Owner` mandatory, 1 … 128 bytes, and says it is *"reported by
`--constitution`; read by no gate"*. Nothing says where `spine init` gets it.

The corpus has exactly one precedent for the same field on the adjacent
artifact. `templates.md` §6.1, substitution 2, for the intent scaffold's
`Owner:`:

> the principal of the signing identity — `--identity` if given, else the
> principal of the key `spine init --signer-key` enrolled for this operator
> (PB §11) — **verbatim, with no `@` prefix added**

and its reasoning, which transfers unchanged:

> PB §3.1's `Owner: @name` is a human convention — a tracker or forge handle —
> and `spine new` has no source for one; every identity in the design is a
> keyring principal. Prefixing would produce `@alice@example.com`.

**Resolution taken:** the same rule, and the same refusal
(`bad-owner-principal`, ID §4.3's grammar). Note CN §12.1's file reads
`Owner: @alice` — that is a human's own later edit of a field no gate reads, not
a counter-example.

---

## A fourth thing, cosmetic and worth one line

§6.2's block heads the rules with `# The non-negotiables`; CN §12.1's worked
file spells the same heading `## The non-negotiables`. Under §3.2 both are
class-5 **comment** lines — first non-whitespace byte is `#` — and comments are
"ignored completely", so the parse is identical and no digest moves. It is an
inconsistency between the seed and the example, not a defect.

---

## Recommended amendment

§6.2 gains the preamble above its block, or a sentence saying the block is
preceded by §3.1's title line and §9.1's header and naming the two values. The
second is smaller and keeps one owner per rule: §3.1 already owns line 1 and
§9.1 already owns line 2, so §6.2 should stop claiming to fix the whole file and
should name `v1` and the principal source instead.

Unlike the two §7.1 defects, this one is **not** terminal in the field: the
constitution is `user-owned`, so a human fixing the header once is the end of
it. But it is terminal for a *fresh* repository, which is every repository on
its first landing, and `spine init` is the command that can brick a repo.

## What the implementation does today

`crates/spine-template/src/constitution.rs` renders the three-part file, with
the two values above, and every choice carries the citation it rests on. Twelve
tests, including one that asserts the preamble is present and in §3.1's
positions, so a later "simplification" back to §6.2's literal block fails.
