"""Notebook widget integration for ruviz plots."""

from __future__ import annotations

from pathlib import Path

try:
    import anywidget
    import traitlets
except ModuleNotFoundError as exc:  # pragma: no cover - covered by a subprocess test
    raise ImportError(
        "ruviz notebook widgets need anywidget and traitlets. "
        'Install them with `pip install "ruviz[widget]"`.'
    ) from exc

from . import _api
from ._api import Plot


class RuvizWidget(anywidget.AnyWidget):
    """AnyWidget wrapper that renders the WASM-backed notebook frontend."""

    _esm = Path(__file__).with_name("widget.js")
    snapshot = traitlets.Dict().tag(sync=True)

    def __init__(self, plot: Plot) -> None:
        """Bind the widget to a :class:`ruviz.Plot` instance."""
        super().__init__()
        self._plot = plot
        self.refresh()

    def refresh(self) -> None:
        """Push the latest plot snapshot into the synced widget model."""
        self.snapshot = self._plot.to_snapshot()


#: ``Plot.widget`` is annotated with this class, which ``_api`` cannot import
#: eagerly without making the optional widget extra mandatory; publishing it
#: here keeps :func:`typing.get_type_hints` exact once the extra is in use.
_api.RuvizWidget = RuvizWidget
