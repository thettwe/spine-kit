# Captured from real pytest

Not hand-written. Both files are the transport's own output from
`crates/spine-runner/plugins/spine_pytest_transport.py` under **pytest 9.1.1,
CPython 3.14.5**, over a scratch project carrying one test of every shape
RF §6.7's mapping has a row for: a pass, a failure, an `xfail`, an `xpass`, a
`@pytest.mark.skip`, two parametrizations, and one id deselected by a
`conftest.py` implementing `pytest_collection_modifyitems`.

- `pytest-9.1-collect-only.jsonl` — the `B` enumeration, `pytest --collect-only`
- `pytest-9.1-candidate.jsonl` — the `T` run, `pytest`

**Two defects were found by capturing these rather than writing them**, and both
are the traps `import-resolver.md` and `result-file.md` warn about:

1. `@pytest.mark.skip` produces `[setup: skipped, teardown: passed]` and **no
   `call` phase at all**. A mapping that read the skip off `call` answered
   `unknown` for every skipped test. RF §6.7's row is "skipped, no
   expected-failure marker" — unqualified by phase.
2. A `conftest.py`'s own `pytest_collection_modifyitems` may run **after** the
   plugin's, so `len(items)` there is the *denominator* of
   `3/4 tests collected (1 deselected)`. IR §11.2's count is the numerator, and
   comparing against the denominator raises `base-collect-failed` on every
   repository with a collection hook. The count is taken in
   `pytest_collection_finish`, which runs after every `modifyitems` hook.

Regenerate with the recipe in `crates/spine-runner/tests/pytest_vectors.rs`.
