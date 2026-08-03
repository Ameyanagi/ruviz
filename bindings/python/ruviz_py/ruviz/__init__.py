"""Python bindings and notebook widgets for ruviz."""

from importlib.metadata import PackageNotFoundError, version as _installed_version
from typing import TYPE_CHECKING, Any

from . import _native
from ._api import (
    ObservableSeries,
    Plot,
    Plot3D,
    line3d,
    observable,
    plot,
    plot3d,
    scatter3d,
    surface,
    wireframe,
)
from ._typing import (
    ArrayLike,
    ColumnSource,
    DataSource,
    LabelsLike,
    LegendPositionName,
    LineStyleName,
    MarkerName,
    MatrixLike,
    NumericVector,
    Plot3DSnapshot,
    PlotSnapshot,
    RadarSeriesDict,
    ScaleName,
    Series3DSnapshot,
    SeriesSnapshot,
    StyleDict,
    Theme,
)

if TYPE_CHECKING:
    # Imported lazily at runtime: notebook widgets live in the optional
    # `ruviz[widget]` extra, so importing ruviz must not require anywidget.
    from ._widget import RuvizWidget


def __getattr__(name: str) -> Any:
    """Resolve ``RuvizWidget`` on first access so the widget extra stays optional."""
    if name == "RuvizWidget":
        from ._widget import RuvizWidget

        return RuvizWidget
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


try:
    #: Installed distribution version; matches the compiled extension.
    __version__ = _installed_version("ruviz")
except PackageNotFoundError:  # pragma: no cover - source tree without metadata
    __version__ = _native.version()

__all__ = [
    "ArrayLike",
    "ColumnSource",
    "DataSource",
    "LabelsLike",
    "LegendPositionName",
    "LineStyleName",
    "MarkerName",
    "MatrixLike",
    "NumericVector",
    "ObservableSeries",
    "Plot",
    "Plot3D",
    "Plot3DSnapshot",
    "PlotSnapshot",
    "RadarSeriesDict",
    "RuvizWidget",
    "ScaleName",
    "Series3DSnapshot",
    "SeriesSnapshot",
    "StyleDict",
    "Theme",
    "__version__",
    "line3d",
    "observable",
    "plot",
    "plot3d",
    "scatter3d",
    "surface",
    "wireframe",
]
