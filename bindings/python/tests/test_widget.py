"""Notebook widget tests: the optional extra and the synced traitlet."""

from __future__ import annotations

import subprocess
import sys
from typing import Any

import ruviz
# Compare against the constant, not a literal: pinning the number here
# makes every legitimate schema bump look like a regression.
from ruviz._api import _SNAPSHOT_SCHEMA_VERSION


# Runs in a subprocess so the blocker cannot disturb the rest of the suite: it
# hides anywidget/traitlets from the import system the way a base install
# without `ruviz[widget]` looks.
_WITHOUT_ANYWIDGET = """
import sys


class _HideWidgetDeps:
    blocked = {"anywidget", "traitlets"}

    def find_spec(self, fullname, path=None, target=None):
        if fullname.split(".")[0] in self.blocked:
            raise ModuleNotFoundError(f"No module named {fullname!r}", name=fullname)
        return None


sys.meta_path.insert(0, _HideWidgetDeps())

import ruviz

assert ruviz.__version__
plot = ruviz.plot().line([0.0, 1.0], [0.0, 1.0]).title("no widget extra")
assert plot.render_png()
assert plot.to_snapshot()["series"]

for describe, call in (
    ("ruviz.RuvizWidget", lambda: ruviz.RuvizWidget),
    ("plot.widget()", plot.widget),
):
    try:
        call()
    except ImportError as exc:
        assert "ruviz[widget]" in str(exc), f"{describe}: {exc}"
    else:
        raise AssertionError(f"{describe} should fail without anywidget")
"""


def test_core_import_works_without_the_widget_extra() -> None:
    result = subprocess.run(
        [sys.executable, "-c", _WITHOUT_ANYWIDGET],
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0, result.stderr


def test_widget_snapshot_traitlet_syncs_observable_updates() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().line([0.0, 1.0, 2.0], source, color="#ff0000", width=2.0)
    widget = plot.widget()

    assert isinstance(widget, ruviz.RuvizWidget)
    assert widget.trait_metadata("snapshot", "sync") is True

    synced: list[dict[str, Any]] = []
    widget.observe(lambda change: synced.append(change["new"]), names="snapshot")

    source.replace([4.0, 5.0, 6.0])

    assert len(synced) == 1
    pushed = synced[-1]
    assert pushed == widget.snapshot
    assert pushed["schemaVersion"] == _SNAPSHOT_SCHEMA_VERSION
    assert pushed["series"][0]["y"]["values"] == [4.0, 5.0, 6.0]
    assert pushed["series"][0]["style"]["color"] == "#ff0000"


def test_widget_snapshot_preserves_horizontal_bar_orientation() -> None:
    widget = ruviz.plot().bar(["a", "b"], [1.0, 2.0], orientation="horizontal").widget()

    assert widget.snapshot["series"][0]["style"]["orientation"] == "horizontal"
