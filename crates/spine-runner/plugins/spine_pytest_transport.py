"""RF §6.6's transport for pytest — the four signals, over a pipe the collector holds.

This file ships **inside the pinned release** and is materialised by the
collector into a directory outside the tree under test. It is never read from
the candidate's checkout: RF §6.6 requires the stream to be "read over a pipe
the collector holds" and "not supplied by the candidate's environment", and a
plugin the candidate could edit would be the candidate writing its own results.

Loaded through ``PYTEST_PLUGINS`` rather than ``-p`` because IR §11.1 ratifies
the argv and says "**No adapter runs a command this section has not already
ratified.**" The ratified forms are ``pytest`` and ``pytest --collect-only``.

The four signals RF §6.6 makes mandatory, per item:

1. the runner-native id      -- ``nodeid``, verbatim
2. the per-phase outcome     -- one entry per ``setup``/``call``/``teardown``
3. the expected-failure polarity
4. **deselection**           -- which pytest reports outside the per-item
                                report, through ``pytest_deselected``, and
                                which a transport carrying only the first three
                                cannot distinguish from an absent id

Nothing here decides an outcome. RF §6.6: "Precedence is phases plus polarity,
never the transport's own outcome word" -- so the plugin reports what happened
in each phase and whether the marker was set, and the collector maps.
"""

import json
import os
import sys

_FD_VARIABLE = "SPINE_TRANSPORT_FD"


class _Channel:
    """The collector's pipe. One JSON object per line, LF-terminated."""

    def __init__(self, stream):
        self._stream = stream

    def write(self, record):
        self._stream.write(json.dumps(record, sort_keys=True, separators=(",", ":")))
        self._stream.write("\n")
        # Flushed per record: a runner killed at ``params.timeout`` should have
        # delivered everything it finished, and RF §7.3 distinguishes a stream
        # that "ended mid-record" from one that never started.
        self._stream.flush()

    def close(self):
        try:
            self._stream.close()
        except Exception:
            pass


def _open_channel():
    raw = os.environ.get(_FD_VARIABLE)
    if raw is None:
        return None
    try:
        return _Channel(os.fdopen(int(raw), "w", closefd=False))
    except (ValueError, OSError) as exc:
        # A channel that cannot be opened is not a test failure and must not
        # become one: the collector sees a stream it cannot parse and RF §7.3
        # has a row for that.
        print(f"spine: {_FD_VARIABLE}={raw!r} could not be opened: {exc}", file=sys.stderr)
        return None


class SpineTransport:
    def __init__(self, channel):
        self._channel = channel
        # nodeid -> {"phases": [...], "expected_failure": bool}
        self._items = {}
        self._order = []
        self._deselected = []
        self._selected = None
        self._collect_errors = []

    # -- collection ------------------------------------------------------

    def pytest_collection_modifyitems(self, items):
        """Every id pytest collected, before any hook has deselected.

        The **count** is not taken here. Verified against pytest 9.1: a
        ``conftest.py`` implementing this same hook may run after this plugin's,
        so ``len(items)`` is the *denominator* of
        ``3/4 tests collected (1 deselected)``. IR §11.1 defines the floor as
        "collected **and selected**", and IR §11.2's count is the numerator, so
        a collector comparing against the denominator raises
        ``base-collect-failed`` on every repository with a collection hook.
        """
        for item in items:
            self._remember(item)

    def pytest_collection_finish(self, session):
        """The selected count, after **every** ``modifyitems`` hook has run.

        This is the hook that gives IR §11.2's numerator, and the reason the
        count is not read one hook earlier.
        """
        self._selected = len(session.items)
        for item in session.items:
            self._remember(item)

    def pytest_deselected(self, items):
        """Signal 4. The one pytest reports outside the per-item report."""
        for item in items:
            self._deselected.append(item.nodeid)

    def pytest_collectreport(self, report):
        """RF §6.6: "A collection error that yields no item id is recorded as
        one ``error`` record whose ``id`` and ``fn`` are the runner's own id for
        the failing collector — for pytest, the file's nodeid"."""
        if report.failed:
            self._collect_errors.append(report.nodeid)

    def _remember(self, item):
        if item.nodeid in self._items:
            return
        self._order.append(item.nodeid)
        self._items[item.nodeid] = {
            "phases": [],
            # Signal 3, read from the item's own markers rather than from a
            # report word: a marker is what the author wrote, and RF §6.6 makes
            # polarity beat the runner's summary.
            "expected_failure": item.get_closest_marker("xfail") is not None,
        }

    # -- execution -------------------------------------------------------

    def pytest_runtest_logreport(self, report):
        """Signal 2, one entry per phase, in report order."""
        record = self._items.get(report.nodeid)
        if record is None:
            # An id that reached execution without collection is not a shape
            # pytest produces, but recording it is cheaper than losing it.
            self._order.append(report.nodeid)
            record = {"phases": [], "expected_failure": False}
            self._items[report.nodeid] = record
        # ``wasxfail`` is set by pytest when an expected failure was observed,
        # including on the strict-xpass path where ``outcome`` reads "failed".
        # RF §6.6's precedence rule is the collector's; this only makes sure the
        # polarity reaches it.
        if hasattr(report, "wasxfail"):
            record["expected_failure"] = True
        record["phases"].append({"phase": report.when, "outcome": report.outcome})

    # -- the terminal event ----------------------------------------------

    def pytest_sessionfinish(self):
        """RF §7.3 defines ``complete`` in terms of the terminal session-end
        event. pytest's is this hook: it runs after the last item and does not
        run when the process is killed, which is exactly the distinction the
        status turns on."""
        if self._channel is None:
            return
        deselected = set(self._deselected)
        for nodeid in self._order:
            if nodeid in deselected:
                continue
            record = self._items[nodeid]
            self._channel.write(
                {
                    "t": "item",
                    "id": nodeid,
                    "phases": record["phases"],
                    "expected_failure": record["expected_failure"],
                    "deselected": False,
                }
            )
        for nodeid in self._deselected:
            self._channel.write(
                {"t": "item", "id": nodeid, "phases": [], "expected_failure": False,
                 "deselected": True}
            )
        for nodeid in self._collect_errors:
            self._channel.write({"t": "collect-error", "id": nodeid})
        if self._selected is not None:
            # IR §11.2: "pytest reports its own collected-and-selected count".
            # The **selected** one -- the numerator of
            # ``3/4 tests collected (1 deselected)``.
            self._channel.write({"t": "count", "selected": self._selected})
        self._channel.write({"t": "end"})
        self._channel.close()


def pytest_configure(config):
    config.pluginmanager.register(SpineTransport(_open_channel()), "spine-transport")
