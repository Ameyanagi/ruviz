"""Notebook widget integration for ruviz plots."""

from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import anywidget
import traitlets

if TYPE_CHECKING:
    from ._api import Plot


class RuvizWidget(anywidget.AnyWidget):
    """AnyWidget wrapper that renders the WASM-backed notebook frontend."""

    _esm = Path(__file__).with_name("widget.js")
    snapshot = traitlets.Dict().tag(sync=True)

    def __init__(self, plot: "Plot") -> None:
        """Bind the widget to a :class:`ruviz.Plot` instance."""
        super().__init__()
        self._plot = plot
        self.refresh()

    def refresh(self) -> None:
        """Push the latest plot snapshot into the synced widget model."""
        self.snapshot = self._plot.to_snapshot()
