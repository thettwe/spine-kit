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

## 4 · `gate-report.md` §6.1's G7 hard-lease counter counts two leases as none

**Where.** GR §6.1: the `spine stats` counter for *"landings whose only
`class: "protected"` entry is a `G7` hard lease"*, made mechanical as *"it
holds iff some entry has `gate == "G7"` and `class == "protected"`, and no
other entry has `class == "protected"`."*

**The ambiguity.** A landing with **two** protected `G7` entries — one intent
leasing two paths. Read literally, the second entry is an "other entry" with
`class == "protected"`, so the predicate is false and the landing counts as
zero. Read against the sentence the predicate serves — *"what the counter buys
is that one intent quietly taxing every other landing becomes visible"* — two
leases from one intent are exactly the phenomenon, and the landing should
count.

**What the implementation does.** The literal reading: exactly one protected
entry, and it is a `G7`. `crates/spine-gates/src/wire.rs`,
`only_protected_is_a_g7_hard_lease`, with both cases tested.

**Why it is safe to leave open.** The counter is derived at read time, is in no
digest, is read by no gate, and `spine stats` reads the graph rather than
reports. Getting it wrong costs a number in a report to a human, and changing
it later changes no stored bytes.

**The fix is one word** — *"and no other entry has `class == "protected"`"*
becomes *"and no entry with `gate != "G7"` has `class == "protected"`"*, if the
counter is meant to count the two-lease landing.

## 5 · `spine new --from <branch>` — how the quick branch's work reaches the intent branch

**Where.** PB §11: *"`--from <branch>` promotes an escalated quick-lane
branch."* PB §5.2 names the trigger — a quick-lane change that trips a Drift or
Strength wire, touches a harness path, adds a `@verifies` pragma, or intersects
an in-flight lease is *"escalated — needs an intent (`spine new --from
<branch>`)"*.

**What is fixed.** The result is *"a fresh `intent/<ID>` branch"* (PB §11), and
PB §5.4 constrains where it starts: *"`spine new` branches only from trunk — a
stacked intent would misattribute members after the first lands."*

**What is not.** How the quick branch's commits get onto it. Cherry-picked one
by one, squashed into a single commit, merged, or left in the worktree for the
human to commit — the corpus names none of these. Each produces a different
`M(L) = git rev-list B..L`, which is the changeset `dump.md` §8.2 derives and
G10 reconstructs, so the choice is ledger content and not a convenience.

**What the implementation does.** Refuses, naming the flag. The plain creation
form is built; `--from` is not, because guessing here would invent the
membership of a changeset.

**The fix is one clause** in PB §11 or §5.2 saying which of the four it is.
`spine stats` *"counts promotions separately"*, which suggests the promoted
commits stay identifiable — an argument for cherry-pick or merge over squash,
but not a statement of one.

## 6 · `result-file.md` §7.1 contradicts itself on P4(a)'s source

**Where.** RF §7.1's fresh-namespace paragraph, parenthetically:

> "(These flags must be read from a `sysfs` mounted **inside** the namespace or
> over netlink; an inherited `sysfs` reports the job's own loopback, up, which
> is the trap the paragraph below is about.)"

And the paragraph it points at, which is explicitly normative:

> "**P4(a) is read over netlink and never over `sysfs`, and this is normative
> rather than an implementation note.** … The per-namespace `procfs` net files
> (`/proc/net/dev`, `/proc/net/if_inet6`) have the same property and are a
> conforming source; `sysfs` is not."

RF §12 repeats the normative form: "P4(a)'s enumeration is normatively over
**netlink** and never `sysfs`".

**The consequence.** A builder who reads the parenthetical mounts a fresh
`sysfs` inside the namespace and believes it conforming. It is not.

**A narrower tension inside the same rule.** P4's table row fixes netlink
*specifically* — "over netlink (`RTM_GETLINK`, `RTM_GETADDR`)" — while the
normative paragraph admits `procfs` as an alternative source. Two conforming
implementations could read different files.

**What the implementation does.** Netlink, both dumps
(`crates/spine-isolate/src/linux.rs`, `measure_egress`), which satisfies both
readings of the normative paragraph and the table row.

**The fix is to delete "a `sysfs` mounted inside the namespace or" from the
parenthetical**, and to say whether the table row's netlink or the paragraph's
netlink-or-procfs is the rule.

## 7 · `import-resolver.md` §11.6's items are numbered 1, 2, 3, 4, 5, 4, 5, 6

**Where.** IR §11.6's list restarts its numbering partway through, so there are
two items numbered 4 and two numbered 5.

**The consequence.** Every cross-reference by number is ambiguous.
`crates/spine-resolve/src/ids.rs:96` already has to write "rule 4 (the second
one so numbered)" to be unambiguous.

**The fix is to renumber**, and until then every reference should quote the
sentence rather than the number.

## 8 · `import-resolver.md` §11.6 claims a terminal session-end event every adapter names, and names two of four

**Where.** IR §11.6's last item: "**Every adapter names its terminal session-end
event**", followed by "`dart-test`'s is the `done` event, `swift-test`'s is the
final `Test Suite 'All tests' <verb> at …` line."

**What is missing.** Neither §11.2 (pytest) nor §11.3 (vitest) names one.

**The consequence.** RF §7.3 defines `complete` in terms of the terminal
session-end event, so for two of the four shipped languages the status
`complete` has no definition — and `complete` is the row that separates a run
that finished from one that died part-way.

**The fix is two sentences**, one in §11.2 and one in §11.3.

## 9 · How does the *collector* find a reseal's `base=`, holding no keyring?

**Where.** PB §5.5: a reseal's seal `base=` is *"the last valid landing below
the range"*. RF §4.2's `base` row: *"for a reseal, the seal's `base=`, from
which every policy read for a reseal is taken"*, and RF §8.6 adds *"`params.langs`
and `params.timeout` included"*.

**The gap.** "Valid landing" is G9's predicate — a verifying seal, a
recomputing `envelope=`, a fenced `blob=` that hashes — and the collector holds
none of the machinery. PB §7.4 rule 3 gives it git objects and policy and
nothing else: it "executes no repository code", holds no keyring, and does no
envelope verification. The corpus fixes what `base=` **is** and never says how
the one process that must write it is to find it.

**What the implementation does.** `crates/spine-collect/src/prepare.rs`,
`reseal_base`: the newest first-parent commit below the orphan carrying a
`Spine-Seal` **trailer** — the syntactic half of G9's predicate. Marked DERIVED
at the function.

**Why it is safe to leave open.** Where the syntactic answer differs from G9's
— an orphan range containing a commit with a forged seal line — the collector
writes a `base=` the trusted stage disagrees with, and RF §8.3 step 1 answers
`base-moved`. That is the retryable outcome, not a wrong file ingested, and it
is the direction to be wrong in.

**Two candidate fixes**, both one clause. Either say the collector takes the
syntactic reading and that a disagreement is `base-moved` by design; or have
`.spine/ci.sh` pass the trusted stage's `B` to the collector, which the
invocation contract (CI §5.1, digest-pinned) does not currently do — and which
would also close the `T`-computed-twice skew this document's neighbours note.
