"""Python bindings and notebook widgets for ruviz."""

from importlib.metadata import PackageNotFoundError, version as _installed_version

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
from ._widget import RuvizWidget

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
