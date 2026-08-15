"""Public type aliases and snapshot shapes for the ruviz Python API.

Nothing here imports pandas, polars, or IPython: dataframe support is described
structurally so the optional dependencies stay optional for type checkers too.
"""

from __future__ import annotations

from collections.abc import Iterator, Mapping, Sequence
from typing import (
    TYPE_CHECKING,
    Any,
    ForwardRef,
    Literal,
    Protocol,
    TypeAlias,
    TypedDict,
    Union,
)

if TYPE_CHECKING:
    from ._api import ObservableSeries


class NumericVector(Protocol):
    """Structural stand-in for NumPy arrays and pandas/polars Series."""

    def __len__(self) -> int: ...

    def __iter__(self) -> Iterator[Any]: ...


class NumericMatrix(Protocol):
    """Structural stand-in for 2D arrays: requires an array ``shape``.

    Plain 1D sequences (``list[float]``) do not qualify. A 1D ndarray or Series
    still matches structurally — dimensionality is not expressible in static
    types — and is rejected at runtime instead.
    """

    @property
    def shape(self) -> tuple[int, ...]: ...

    def __iter__(self) -> Iterator[Any]: ...


class ColumnSource(Protocol):
    """Structural stand-in for objects indexed by column name, such as DataFrames.

    ``key`` is position-only, which is how ``dict``, ``MappingProxyType``, and
    the dataframe libraries all declare it.
    """

    def __getitem__(self, key: str, /) -> Any: ...


if TYPE_CHECKING:
    #: A numeric vector: a sequence, a NumPy array, a pandas/polars Series, or an
    #: :class:`~ruviz.ObservableSeries`.
    ArrayLike: TypeAlias = Sequence[float] | NumericVector | ObservableSeries
else:
    # The aliases are real runtime objects so ``typing.get_type_hints`` works on
    # the annotated public methods. ``ObservableSeries`` lives in ``_api``, which
    # imports this module, so it is named through a forward reference bound to
    # *this* module: ``_api`` injects the class here once it exists, and the
    # reference then resolves no matter which module asks for the hints.
    # ``Union[...]`` rather than ``|``: only it accepts a ``ForwardRef`` operand
    # on every supported Python version.
    ArrayLike = Union[
        Sequence[float], NumericVector, ForwardRef("ObservableSeries", module=__name__)
    ]

#: A rectangular numeric matrix, such as a nested sequence or a 2D NumPy array.
MatrixLike: TypeAlias = Sequence[Sequence[float]] | NumericMatrix

#: A vector of labels; values are stringified when they are not already strings.
LabelsLike: TypeAlias = Sequence[object] | NumericVector

#: A ``data=`` source whose columns are looked up by name.
DataSource: TypeAlias = Mapping[str, Any] | ColumnSource | None

#: Built-in theme names accepted by :meth:`Plot.theme`.
Theme: TypeAlias = Literal[
    "light",
    "dark",
    "seaborn",
    "publication",
    "minimal",
    "presentation",
]

#: Theme names accepted by :meth:`Plot3D.theme`, which only ships the two.
Theme3D: TypeAlias = Literal["light", "dark"]

#: Line style names accepted by ``linestyle=``; the last four are the matplotlib
#: shorthands, and a snapshot always stores the canonical name they resolve to.
LineStyleName: TypeAlias = Literal[
    "solid",
    "dashed",
    "dotted",
    "dash-dot",
    "dash-dot-dot",
    "-",
    "--",
    ":",
    "-.",
]

#: Marker names accepted by ``marker=``; the last eight are the matplotlib
#: shorthands, and a snapshot always stores the canonical name they resolve to.
MarkerName: TypeAlias = Literal[
    "circle",
    "square",
    "triangle",
    "triangle-down",
    "diamond",
    "plus",
    "cross",
    "star",
    "circle-open",
    "square-open",
    "triangle-open",
    "diamond-open",
    "o",
    "s",
    "^",
    "v",
    "D",
    "+",
    "x",
    "*",
]

#: Legend placements accepted by :meth:`ruviz.Plot.legend`; ``"best"`` auto-places.
LegendPositionName: TypeAlias = Literal[
    "best",
    "upper_right",
    "upper_left",
    "lower_left",
    "lower_right",
    "right",
    "center_left",
    "center_right",
    "lower_center",
    "upper_center",
    "center",
    "outside_right",
    "outside_left",
    "outside_upper",
    "outside_lower",
]

#: Axis scales accepted by :meth:`ruviz.Plot.xscale` and :meth:`ruviz.Plot.yscale`.
ScaleName: TypeAlias = Literal["linear", "log", "symlog"]

#: Snapshot ``kind`` discriminators for 2D series.
SeriesKindName: TypeAlias = Literal[
    "line",
    "scatter",
    "bar",
    "histogram",
    "boxplot",
    "heatmap",
    "error-bars",
    "error-bars-xy",
    "kde",
    "ecdf",
    "contour",
    "pie",
    "radar",
    "violin",
    "polar-line",
]

#: Snapshot ``kind`` discriminators for 3D series.
Series3DKindName: TypeAlias = Literal["scatter3d", "line3d", "surface", "wireframe"]


class NumericSourceDict(TypedDict):
    """One numeric column of a snapshot series, static or observable-backed."""

    kind: Literal["static", "observable"]
    values: list[float]


#: A snapshot numeric field: tracked kinds store a source dict, the rest a plain list.
NumericField: TypeAlias = NumericSourceDict | list[float]


class StyleDict(TypedDict, total=False):
    """Per-series styling as stored in a snapshot.

    Keys are camelCase to match the snapshot spelling; the matching Python
    keyword for ``markerSize`` is ``marker_size``.
    """

    label: str
    color: str
    alpha: float
    width: float
    linestyle: LineStyleName
    marker: MarkerName
    markerSize: float
    bins: int
    density: bool
    bandwidth: float
    levels: int


class _RadarSeries(TypedDict):
    values: ArrayLike


class RadarSeriesDict(_RadarSeries, total=False):
    """One named radar series; ``name`` is optional and may be ``None``."""

    name: str | None


class _SeriesSnapshot(TypedDict):
    kind: SeriesKindName


class SeriesSnapshot(_SeriesSnapshot, total=False):
    """One serialized 2D series; only the keys its ``kind`` uses are present."""

    style: StyleDict
    x: NumericField
    y: NumericField
    z: list[float]
    r: list[float]
    theta: list[float]
    data: NumericField
    values: NumericField
    categories: list[str]
    labels: list[str]
    series: list[RadarSeriesDict]
    xErrors: NumericField
    yErrors: NumericField
    rows: int
    cols: int


class _PlotSnapshot(TypedDict):
    schemaVersion: int
    series: list[SeriesSnapshot]


class ReferenceLineStyleSnapshot(TypedDict, total=False):
    """Styling for a ``vline``/``hline``; unset fields keep the 1pt dashed gray default."""

    color: str
    width: float
    linestyle: LineStyleName


class TextAnnotationStyleSnapshot(TypedDict, total=False):
    """Styling for a text annotation; the default is 10pt black."""

    color: str
    fontSize: float


class _VLineAnnotationSnapshot(TypedDict):
    kind: Literal["vline"]
    x: float


class VLineAnnotationSnapshot(_VLineAnnotationSnapshot, total=False):
    style: ReferenceLineStyleSnapshot


class _HLineAnnotationSnapshot(TypedDict):
    kind: Literal["hline"]
    y: float


class HLineAnnotationSnapshot(_HLineAnnotationSnapshot, total=False):
    style: ReferenceLineStyleSnapshot


class _TextAnnotationSnapshot(TypedDict):
    kind: Literal["text"]
    x: float
    y: float
    text: str


class TextAnnotationSnapshot(_TextAnnotationSnapshot, total=False):
    style: TextAnnotationStyleSnapshot


#: One plot-level annotation; the list preserves call order, and the literal
#: ``kind`` fields let a type checker narrow the union.
AnnotationSnapshot: TypeAlias = (
    VLineAnnotationSnapshot | HLineAnnotationSnapshot | TextAnnotationSnapshot
)


class PlotSnapshot(_PlotSnapshot, total=False):
    """JSON-friendly snapshot of a :class:`ruviz.Plot`.

    Consumers must ignore keys they do not know; ``schemaVersion`` tracks the
    layout, and every plot-level setting is absent until it is set.
    """

    sizePx: list[int]
    #: Figure size in inches. Spelled to match the JS runtime, which renders
    #: these snapshots for the notebook widget — a divergent key would be
    #: silently ignored there rather than raising.
    sizeIn: list[float]
    dpi: int
    maxResolution: list[int]
    fontSize: float
    titleSize: float
    fontFamily: str
    scaleTypography: float
    lineWidthPt: float
    margin: float
    tightLayoutPad: float
    scientificNotation: bool
    fast: bool
    theme: Theme
    ticks: bool
    title: str
    xLabel: str
    yLabel: str
    legend: LegendPositionName
    grid: bool
    xLim: list[float]
    yLim: list[float]
    xScale: list[str | float]
    yScale: list[str | float]
    #: Plot-level annotations — reference lines and text labels — in call order.
    annotations: list[AnnotationSnapshot]


class _Series3DSnapshot(TypedDict):
    kind: Series3DKindName


class Series3DSnapshot(_Series3DSnapshot, total=False):
    """One serialized 3D series; ``z`` is a grid for surface and wireframe."""

    x: list[float]
    y: list[float]
    z: list[float] | list[list[float]]


class _Plot3DSnapshot(TypedDict):
    schemaVersion: int
    series: list[Series3DSnapshot]


class Plot3DSnapshot(_Plot3DSnapshot, total=False):
    """JSON-friendly snapshot of a :class:`ruviz.Plot3D`."""

    sizePx: list[int]
    dpi: int
    theme: Theme3D
    title: str
    xLabel: str
    yLabel: str
    zLabel: str
    xLim: list[float]
    yLim: list[float]
    zLim: list[float]
    azimuthDeg: float
    elevationDeg: float
    projection: Literal["orthographic", "perspective"]
    perspectiveDeg: float


__all__ = [
    "ArrayLike",
    "ColumnSource",
    "DataSource",
    "LabelsLike",
    "LegendPositionName",
    "LineStyleName",
    "MarkerName",
    "MatrixLike",
    "NumericField",
    "NumericMatrix",
    "NumericSourceDict",
    "NumericVector",
    "Plot3DSnapshot",
    "PlotSnapshot",
    "RadarSeriesDict",
    "ScaleName",
    "Series3DKindName",
    "Series3DSnapshot",
    "SeriesKindName",
    "SeriesSnapshot",
    "StyleDict",
    "Theme",
    "Theme3D",
]
