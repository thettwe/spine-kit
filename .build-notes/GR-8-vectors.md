# The two gate-report vectors, and how §8.1 is reached

Verified 2026-08-28, before the report crate existed, because everything
downstream keys off these two digests.

## §8.2 — printed, and it reproduces as printed

The fenced block in §8.2 is pretty-printed JSON. JCS-serialize it per §2:

```
4053 bytes
sha256:a47c13281b0b4a13085e523cd3c09e4f7d1caf0c80419de5fa3a869f7aec309e
```

Both match the published values. §8.2 says so itself and it is worth quoting,
because it is the whole acceptance test for a canonicalizer:

> Shown pretty-printed for reading. **The pretty form is not canonical.**
> JCS-serialize this value per §2 and the result must be exactly the length and
> digest below. That is the test.

## §8.1 — NOT printed, and derivable

§8.1 publishes only a byte count and a digest:

```
canonical length = 3476 bytes
report           = sha256:e2bd8cb5da473701bf0dd8e9aa0c6a28cc1280e33ba06758165c5089ae3f5b47
```

Its bytes appear nowhere. What §8.1 gives instead is the difference:

> The two reports differ in exactly two members: `gates[G2].status` becomes
> `"override"`, and the review …

So evaluation 1 is evaluation 2 with those two members put back:

1. the `gates` entry whose `gate` is `"G2"` takes `status: "fail"`
2. `authority.reviews` is `[]`

That reproduces, first try:

```
3476 bytes
sha256:e2bd8cb5da473701bf0dd8e9aa0c6a28cc1280e33ba06758165c5089ae3f5b47
```

**Both changes are needed and the byte count alone would not have caught a
wrong one** — the two members' contributions do not happen to cancel, but a
test that checked only the length would still be weaker than one checking the
digest, which is the general rule GR §8.2.1 draws after a 56-byte disagreement
took three rounds to find.

## The ordering dependency, stated once

§8.2 carries §8.1's digest **inside it**, in bob's `Spine-Review` line's
`report=`. So §8.1 must be computed first and §8.2 must not be "fixed" to match
a recomputed §8.1 — the dependency runs one way. `docs/spec/README.md` records
this as "the one ordering dependency in the set".

## Reproducing it

```python
import hashlib, json, copy
# extract §8.2's ```json block from docs/spec/gate-report.md
ev2 = json.loads(block)
jcs = lambda v: json.dumps(v, sort_keys=True, separators=(",",":"),
                           ensure_ascii=False).encode()

ev1 = copy.deepcopy(ev2)
for g in ev1["gates"]:
    if g["gate"] == "G2":
        g["status"] = "fail"
ev1["authority"]["reviews"] = []

assert len(jcs(ev2)) == 4053
assert len(jcs(ev1)) == 3476
```
