# Questions for the owner, raised while building

Filed rather than decided. Each names what is at stake and what the
implementation does in the meantime.

## 1 · Does the Dart declarative subset admit YAML's own quote escapes?

**Where.** `import-resolver.md` §6.3 step 2: the pubspec is read as "block
mappings, block sequences and plain or single/double-quoted scalars only", and
the excluded list is anchors, aliases, merge keys, tags, multi-document streams
and deeply nested flow mappings. A **doubled single quote** — `'it''s useful'`,
YAML's only escape inside a single-quoted scalar — is on neither list.

**What the implementation does.** Refuses it, with a stated reason: "refusing is
cheaper than deciding which of YAML's two escape dialects applies."

**Why it is worth a ruling.** The consequence is out of proportion to the
construct. A refused pubspec is `pubspec-not-declarative`, which is
`lang-unclassifiable`, and under §3.8's language level that makes **every Dart
file in the repository contribute no edges** — so one apostrophe in a
`description:` blocks every Dart landing in that repository, permanently, until
a human notices and rewrites the line.

That is the same shape and the same blast radius as the `#`-inside-quotes bug
fixed on 2026-08-28, which was a defect. This one is a documented choice, which
is why it is a question rather than a fix.

**Options.** (a) Admit `''` and `\"` — YAML's rules, two lines, and the subset
stops refusing valid pubspecs. (b) Keep the refusal and add the construct to
§6.3 step 2's excluded list, so it is refused *by name* rather than by
implication, and an implementer knows it is intended. (c) Keep it and downgrade
the consequence, which is the largest change and probably wrong: §3.8's language
level exists so a resolver that cannot read a package does not silently
under-freeze it.

Recommended: **(a)**. The refusal buys nothing a reader needs, and the failure
is silent to whoever writes the pubspec.

## 2 · §4.1's Python anchor cannot reach a construct §3.7 names by name

**Where.** IR §4.1's anchor: "an import site begins at a `word` token `import`
or `from` that is the **first token of a logical line or the first token after a
`;`**." In `try: import oracle` the `import` is after a `:`, so the anchor does
not reach it.

**But §3.7 names that exact construct**, in its opening sentence, as one of the
four languages' conditional constructs — "**Python's `try: import … except
ImportError:`**" — and requires that "every branch of every conditional
construct contributes its import sites, and all of them are resolved", because
"dropping a branch is how an [oracle hides]".

**What the implementation does.** Follows §3.7: a compound statement's suite is
scanned when it is written on the same line as its header. Between a section
that names the construct and a section that merely fails to reach it, the one
that names it governs — and the direction matters, because the anchor's reading
leaves an oracle a one-line hiding place in the freeze closure, which is exactly
the failure that cost Kotlin its place in v1.

**Why it still needs a ruling.** This **moves `closure_digest`** for any
repository containing a compound one-liner import, and `closure_digest` is a
value two implementations must agree on — G8 recomputes the closure in CI, so a
disagreement rejects an approval rather than merely differing. An implementer
reading §4.1 alone builds the narrower closure and rejects this one's approvals.

**The fix is one clause in §4.1**, so the two sections agree: the anchor gains
"or the first token after a `:` that opens a compound statement's suite on the
same logical line". Nothing else in §4.1 moves.

## 3 · `result-file.md` §8.3 step 2 constructs a `tool=` that cannot be right

**Where.** RF §8.3 step 2 builds the expected tool token literally as
`<cli.version>` + `+sha256:` + `<cli.dist_hash>`.

**Why it cannot be read literally.** `manifest.md` §3 stores `dist_hash`
*already prefixed* — MF §8.3's published manifest carries
`"dist_hash":"sha256:6f49644f…744db"`. Read literally the recipe yields
`1.4.0+sha256:sha256:6f49…`, which is outside RF §4.2's own grammar for the
field and is not RF §10's published `tool=`.

**What the implementation does.** Strips a `sha256:` prefix before re-adding the
separator, and pins RF §10's vector as the arbiter. Documented at
`crates/spine-collect/src/collector.rs`.

**The fix is one word in §8.3 step 2** — either the recipe drops its literal
`sha256:`, or it says the prefix is stripped from `cli.dist_hash` first.
