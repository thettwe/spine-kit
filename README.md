# spine-kit

Drift-gated, intent-first development for AI-assisted teams — a spec-kit-style toolkit (CLI: `spine`) whose specs cannot go stale, whose tests are frozen by blob id, and whose auto-merges are signed, hash-bound records an offline clone can re-verify.

**Status:** design settled after seven adversarial reviews (`PLAYBOOK.md` §12). **The artifact grammars are written** — ten normative specs in [`docs/spec/`](docs/spec/), roughly 12,800 lines, indexed by [`docs/spec/README.md`](docs/spec/README.md), which carries the per-spec status, the open owner decisions and every published digest. Four cross-document reviews have been absorbed; the count of defects that would fail *every* landing has gone 9 → 4 → 7 → 4. Nothing ships yet.

- [`PLAYBOOK.md`](PLAYBOOK.md) — the reference design (v0.19). Start with §1, then §11 (vocabulary) and §12 (what changed, what is closed, and what is not).
- [`docs/reviews/`](docs/reviews/) — adversarial design reviews the playbook answers.
- [`docs/spec/`](docs/spec/) — normative artifact grammars; what v1 is built from.
- [`docs/proposals/`](docs/proposals/) — designs specified but deliberately outside v1.
- [`docs/history/`](docs/history/) — previous playbook versions.

The playbook is governed by its own rules: keep it short, change it by PR, delete anything a machine could enforce instead.
