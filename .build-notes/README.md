# Build notes

Working artifacts for the v1 implementation. **Not normative.** `PLAYBOOK.md`
and `docs/spec/` are the design; these are notes about building from it, and
where the two disagree the corpus wins.

Read in this order:

| File | What it is |
|---|---|
| [`00-BUILD-PLAN.md`](00-BUILD-PLAN.md) | **Start here.** Build order, the vector attack order, the eleven items that block code, the contradictions with adjudication, and "where this gets implemented wrong" |
| [`DECISIONS.md`](DECISIONS.md) | Owner rulings and the plan defaults taken, each with what it closes |
| [`FINDINGS-isolation.md`](FINDINGS-isolation.md) | The two `result-file.md` §7.1 defects, with the measurements — closed by §13 R36 |
| [`FINDINGS-constitution-seed.md`](FINDINGS-constitution-seed.md) | The `constitution@1` seed defect — closed by CN §15 D18, values left as §16 OPEN-10 and OPEN-11 |
| `01`–`13` | Requirement sheets, one per concern, extracted from the corpus: ~1,271 numbered requirements, each with its citation |

The sheets are a **navigation aid, not an authority**. Every requirement carries
its `(SPEC §x.y)` citation precisely so a reader goes back to the document
rather than trusting the sheet — several sheets already record places they were
wrong, and one of them (the region split) was wrong in a way that reached code
before the corpus corrected it.
