"""Public Python API for ruviz.

The Python package exposes a fluent :class:`Plot` builder for static export,
static notebook display, explicit Jupyter widgets, and native interactive
display outside notebooks.
"""

from __future__ import annotations

import asyncio
import operator
import sys
import weakref
from collections.abc import Callable, Mapping, Sequence
from copy import deepcopy
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Any, TypeAlias, cast

import numpy as np
import numpy.typing as npt

from . import _native, _typing
from ._typing import (
    ArrayLike,
    DataSource,
    LabelsLike,
    LegendPositionName,
    LineStyleName,
    MarkerName,
    MatrixLike,
    Plot3DSnapshot,
    PlotSnapshot,
    RadarSeriesDict,
    ScaleName,
    Theme,
)

if TYPE_CHECKING:
    from ._widget import RuvizWidget
else:
    # ``widget()`` imports ``_widget`` on demand so the optional widget extra
    # stays optional. The name still has to exist here for
    # :func:`typing.get_type_hints`; ``_widget`` replaces it with the real class
    # as soon as it loads.
    RuvizWidget = Any

#: Internal storage for every numeric vector: an owned C-contiguous float64 array.
_F64Array: TypeAlias = "npt.NDArray[np.float64]"


def _is_notebook() -> bool:
    try:
        # Imported from its defining module: the IPython package re-export is
        # not visible to type checkers.
        from IPython.core.getipython import get_ipython
    except ImportError:
        return False

    shell = get_ipython()
    return shell is not None and shell.__class__.__name__ == "ZMQInteractiveShell"


def _is_dataframe(value: Any) -> bool:
    return _is_pandas_dataframe(value) or _is_polars_dataframe(value)


def _is_series(value: Any) -> bool:
    return _is_pandas_series(value) or _is_polars_series(value)


def _is_pandas_dataframe(value: Any) -> bool:
    # A value can only be a pandas type once pandas is imported, so checking
    # ``sys.modules`` first keeps a plain list from paying the import.
    if "pandas" not in sys.modules:
        return False

    try:
        import pandas as pd
    except ImportError:
        return False

    return isinstance(value, pd.DataFrame)


def _is_pandas_series(value: Any) -> bool:
    if "pandas" not in sys.modules:
        return False

    try:
        import pandas as pd
    except ImportError:
        return False

    return isinstance(value, pd.Series)


def _is_polars_dataframe(value: Any) -> bool:
    if "polars" not in sys.modules:
        return False

    try:
        import polars as pl
    except ImportError:
        return False

    return isinstance(value, pl.DataFrame)


def _is_polars_series(value: Any) -> bool:
    if "polars" not in sys.modules:
        return False

    try:
        import polars as pl
    except ImportError:
        return False

    return isinstance(value, pl.Series)


def _column_values(data: Any, column: Any) -> Any:
    """Look one named column up in a ``data=`` source.

    Anything indexable by column name works: a DataFrame, any
    :class:`~collections.abc.Mapping`, or any other object implementing
    ``__getitem__`` — which is what the :data:`~ruviz.DataSource` alias promises.
    """
    if data is None or not isinstance(column, str):
        return column

    if _is_series(data):
        raise TypeError(
            "data= expects a DataFrame or dict; pass a Series directly as the value instead"
        )
    if not (_is_dataframe(data) or isinstance(data, Mapping) or hasattr(data, "__getitem__")):
        raise TypeError(f"unsupported data source for column lookup: {type(data)!r}")

    try:
        return data[column]
    except KeyError as err:
        raise KeyError(f"column {column!r} is not in the data= source") from err
    except TypeError as err:
        raise TypeError(f"unsupported data source for column lookup: {type(data)!r}") from err


# Plot kinds that forward an ObservableSeries to the native renderer.
_OBSERVABLE_KINDS = "line, scatter, bar, histogram, boxplot, error_bars, error_bars_xy"


def _reject_observable(values: Any, kind: str) -> Any:
    if isinstance(values, ObservableSeries):
        raise TypeError(
            f"{kind} does not support ObservableSeries; pass static values "
            f"(observables are supported by {_OBSERVABLE_KINDS})"
        )
    return values


def _to_numeric_1d(values: Any, name: str = "numeric input") -> _F64Array:
    """Normalize a numeric vector into an owned C-contiguous float64 array.

    The array is what reaches the native handle, which copies it with a single
    ``memcpy``; ``np.array`` always copies, so stored state never aliases a
    caller-owned buffer.
    """
    if isinstance(values, ObservableSeries):
        values = values.values()
    if isinstance(values, str):
        raise TypeError(f"{name} is a string; pass data= to look up columns by name")
    if _is_dataframe(values):
        raise TypeError(
            f"{name} must be a 1D numeric array; select a column or pass data=<DataFrame>"
        )
    if _is_series(values):
        values = values.to_list()

    array = np.array(values, dtype=np.float64, order="C")
    if array.ndim != 1:
        raise TypeError(f"{name} must be a 1D numeric array")
    return array


def _to_static_numeric_1d(values: Any, kind: str, name: str) -> _F64Array:
    """Normalize a numeric vector for plot kinds that cannot track observables."""
    return _to_numeric_1d(_reject_observable(values, kind), f"{kind} {name}")


def _to_numeric_2d(values: Any, name: str) -> _F64Array:
    """Normalize a regular 3D grid while preserving its row/column shape."""
    if _is_dataframe(values) or _is_series(values):
        values = values.to_numpy()

    array = np.array(values, dtype=np.float64, order="C")
    if array.ndim != 2:
        raise TypeError(f"{name} must be a 2D numeric array")
    return array


def _materialize(value: Any) -> Any:
    """Deep-copy stored state into the snapshot's plain JSON-friendly types.

    Numeric state is held as float64 arrays; snapshots stay plain ``list``s of
    Python floats, so this runs once per snapshot rebuild and is then cached.
    """
    if isinstance(value, np.ndarray):
        return value.tolist()
    if isinstance(value, dict):
        return {key: _materialize(item) for key, item in value.items()}
    if isinstance(value, list):
        return [_materialize(item) for item in value]
    return value


def _copy_materialized(value: Any) -> Any:
    """Copy a materialized snapshot without :func:`deepcopy`'s per-element cost.

    A materialized snapshot only holds plain dicts, lists, and scalars, and every
    list in it is homogeneous, so a numeric or label vector is copied by one
    ``list()`` call instead of element by element.
    """
    if isinstance(value, dict):
        return {key: _copy_materialized(item) for key, item in value.items()}
    if isinstance(value, list):
        if value and isinstance(value[0], (dict, list)):
            return [_copy_materialized(item) for item in value]
        return list(value)
    return value


def _to_string_list(values: Any, name: str = "label input") -> list[str]:
    _reject_observable(values, name)
    if isinstance(values, str):
        raise TypeError(f"{name} is a string; pass data= to look up columns by name")
    if _is_dataframe(values):
        raise TypeError(f"{name} must be 1D; select a column or pass data=<DataFrame>")
    if _is_series(values):
        values = values.to_list()
    return [str(value) for value in values]


def _normalize_observable_math_input(value: Any) -> Any:
    if isinstance(value, ObservableSeries):
        return value
    if _is_series(value):
        value = value.to_list()

    array = np.array(value, dtype=np.float64, order="C")
    if array.ndim == 0:
        return float(array.item())
    if array.ndim == 1:
        return array

    raise TypeError(
        "ObservableSeries math only supports real scalars, 1D numeric arrays, and other observables"
    )


#: Messages for the whole-number plot dimensions, shared with the native handle.
_SIZE_PX_MESSAGE = "plot dimensions must be integers greater than zero"
_DPI_MESSAGE = "plot dpi must be an integer greater than zero"
_SIZE_PX_3D_MESSAGE = "3D plot dimensions must be integers greater than zero"
_DPI_3D_MESSAGE = "3D plot dpi must be an integer greater than zero"

#: Snapshot layout version carried by :meth:`Plot.to_snapshot` and
#: :meth:`Plot3D.to_snapshot`; consumers must ignore fields they do not know.
_SNAPSHOT_SCHEMA_VERSION = 1


def _heatmap_matrix(series: dict[str, Any]) -> list[list[float]]:
    cols = int(series["cols"])
    values = series["values"]
    return [values[start : start + cols] for start in range(0, len(values), cols)]


#: matplotlib shorthands accepted by ``marker=``, mapped onto the core names.
_MARKER_ALIASES = {
    "o": "circle",
    "s": "square",
    "^": "triangle",
    "v": "triangle-down",
    "d": "diamond",
    "+": "plus",
    "x": "cross",
    "*": "star",
}

#: matplotlib shorthands accepted by ``linestyle=``, mapped onto the core names.
_LINESTYLE_ALIASES = {"-": "solid", "--": "dashed", ":": "dotted", "-.": "dash-dot"}


def _style_text(name: str, aliases: Mapping[str, str] | None = None) -> Callable[[Any], str]:
    """Accept a string; the native layer maps it onto the matching core enum.

    ``aliases`` resolves the matplotlib shorthands here, so a snapshot only ever
    stores the canonical name.
    """
    shorthands: Mapping[str, str] = aliases or {}

    def normalize(value: Any) -> str:
        if not isinstance(value, str):
            raise TypeError(f"{name} must be a string")
        normalized = value.strip().lower().replace("_", "-")
        return shorthands.get(normalized, normalized)

    return normalize


def _style_label(value: Any) -> str:
    if not isinstance(value, str):
        raise TypeError("label must be a string")
    return value


def _style_color(value: Any) -> str:
    if not isinstance(value, str):
        raise TypeError("color must be a string such as '#2563eb' or 'red'")
    return value.strip().lower()


def _style_alpha(value: Any) -> float:
    alpha = float(value)
    if not 0.0 <= alpha <= 1.0:
        raise ValueError("alpha must be between 0.0 and 1.0")
    return alpha


def _style_flag(name: str) -> Callable[[Any], bool]:
    """Accept a plain ``bool``; ``1``/``"yes"`` are caller mistakes, not flags."""

    def normalize(value: Any) -> bool:
        if not isinstance(value, bool):
            raise TypeError(f"{name} must be a bool")
        return value

    return normalize


def _style_positive(name: str) -> Callable[[Any], float]:
    def normalize(value: Any) -> float:
        number = float(value)
        if not np.isfinite(number) or number <= 0.0:
            raise ValueError(f"{name} must be a finite positive number")
        return number

    return normalize


def _exact_int(value: Any, message: str) -> int:
    """Convert an integer option exactly, so a fraction is rejected, not truncated.

    ``operator.index`` accepts Python and NumPy integers and nothing else;
    booleans are rejected on top of that because ``True`` is not a count.
    """
    if isinstance(value, bool):
        raise ValueError(message)
    try:
        return operator.index(value)
    except TypeError:
        raise ValueError(message) from None


def _style_count(name: str, minimum: int) -> Callable[[Any], int]:
    message = f"{name} must be an integer >= {minimum}"

    def normalize(value: Any) -> int:
        count = _exact_int(value, message)
        if count < minimum:
            raise ValueError(message)
        return count

    return normalize


#: Snapshot style key -> validator. Keys are camelCase to match the snapshot's
#: existing ``sizePx``/``xLabel`` spelling; :data:`_STYLE_KEYWORDS` maps them
#: back to the Python keyword arguments.
_STYLE_OPTIONS: dict[str, Callable[[Any], Any]] = {
    "label": _style_label,
    "color": _style_color,
    "alpha": _style_alpha,
    "width": _style_positive("width"),
    "linestyle": _style_text("linestyle", _LINESTYLE_ALIASES),
    "marker": _style_text("marker", _MARKER_ALIASES),
    "markerSize": _style_positive("marker_size"),
    "bins": _style_count("bins", 1),
    "density": _style_flag("density"),
    "bandwidth": _style_positive("bandwidth"),
    "levels": _style_count("levels", 2),
}

#: Snapshot style key -> Python keyword, for the keys whose spellings differ.
_STYLE_KEYWORDS = {"markerSize": "marker_size"}


@dataclass(frozen=True)
class _SeriesKind:
    """How one snapshot series maps onto the ``Plot``/native method of the same name.

    ``args`` are snapshot keys in positional call order and ``sources`` names the
    subset holding ``{"kind", "values"}`` numeric sources that may be backed by an
    observable. ``native_args``/``public_args`` override the derived arguments for
    the kinds whose stored shape differs from the method signature. ``style`` lists
    the :data:`_STYLE_OPTIONS` keys the core builder for this kind actually honors.
    """

    args: tuple[str, ...] = ()
    sources: frozenset[str] = frozenset()
    native_args: Callable[[dict[str, Any]], list[Any]] | None = None
    public_args: Callable[[dict[str, Any]], list[Any]] | None = None
    style: frozenset[str] = frozenset()


def _pie_args(series: dict[str, Any]) -> list[Any]:
    return [series["values"], series.get("labels")]


_COMMON_STYLE = frozenset({"label", "color", "alpha"})
_LINE_STYLE = _COMMON_STYLE | {"width"}

_SERIES_KINDS: dict[str, _SeriesKind] = {
    "line": _SeriesKind(
        ("x", "y"),
        frozenset({"x", "y"}),
        style=_LINE_STYLE | {"linestyle", "marker", "markerSize"},
    ),
    "scatter": _SeriesKind(
        ("x", "y"),
        frozenset({"x", "y"}),
        style=_COMMON_STYLE | {"marker", "markerSize"},
    ),
    "bar": _SeriesKind(("categories", "values"), frozenset({"values"}), style=_COMMON_STYLE),
    "histogram": _SeriesKind(
        ("data",),
        frozenset({"data"}),
        style=_COMMON_STYLE | {"bins", "density"},
    ),
    "boxplot": _SeriesKind(
        ("data",),
        frozenset({"data"}),
        style=_LINE_STYLE | {"linestyle"},
    ),
    "heatmap": _SeriesKind(
        native_args=lambda series: [
            series["values"],
            int(series["rows"]),
            int(series["cols"]),
        ],
        public_args=lambda series: [_heatmap_matrix(series)],
    ),
    "error-bars": _SeriesKind(
        ("x", "y", "yErrors"),
        frozenset({"x", "y", "yErrors"}),
        style=_LINE_STYLE,
    ),
    "error-bars-xy": _SeriesKind(
        ("x", "y", "xErrors", "yErrors"),
        frozenset({"x", "y", "xErrors", "yErrors"}),
        style=_LINE_STYLE,
    ),
    "kde": _SeriesKind(("data",), style=_LINE_STYLE | {"bandwidth"}),
    "ecdf": _SeriesKind(("data",), style=_LINE_STYLE),
    "contour": _SeriesKind(("x", "y", "z"), style=frozenset({"alpha", "width", "levels"})),
    "pie": _SeriesKind(native_args=_pie_args, public_args=_pie_args),
    "radar": _SeriesKind(
        ("labels", "series"),
        native_args=lambda series: [
            series["labels"],
            [(item.get("name"), item["values"]) for item in series["series"]],
        ],
    ),
    "violin": _SeriesKind(("data",), style=_LINE_STYLE),
    "polar-line": _SeriesKind(("r", "theta"), style=_LINE_STYLE),
}


def _series_kind(series: dict[str, Any]) -> tuple[_SeriesKind, str]:
    """Return the series spec and the method name shared by ``Plot`` and the handle."""
    kind = series["kind"]
    spec = _SERIES_KINDS.get(kind)
    if spec is None:
        raise ValueError(f"unsupported plot snapshot kind: {kind}")
    return spec, kind.replace("-", "_")


def _styled_series(kind: str, fields: dict[str, Any], options: dict[str, Any]) -> dict[str, Any]:
    """Build one series snapshot, validating and attaching the style the caller set."""
    supported = _SERIES_KINDS[kind].style
    style: dict[str, Any] = {}
    for key, value in options.items():
        if value is None:
            continue
        if key not in supported:
            accepted = ", ".join(sorted(_STYLE_KEYWORDS.get(name, name) for name in supported))
            raise ValueError(
                f"{kind} does not support {_STYLE_KEYWORDS.get(key, key)}=; "
                f"accepted: {accepted or 'none'}"
            )
        normalized = _STYLE_OPTIONS[key](value)
        if normalized is False:
            # Flag options are off by default, so `False` is the absence of
            # styling: validate it, then keep it out of the snapshot.
            continue
        style[key] = normalized

    series = {"kind": kind, **fields}
    if style:
        series["style"] = style
    return series


def _freeze_observable_sources(state: dict[str, Any]) -> dict[str, Any]:
    """Mark every numeric source in a copied state static.

    A cloned plot holds values, not live observables, so its snapshot must not
    keep claiming ``kind: "observable"``.
    """
    for series in state["series"]:
        spec, _ = _series_kind(series)
        for key in spec.sources:
            source = series[key]
            if source["kind"] == "observable":
                source["kind"] = "static"
    return state


def _style_keywords(series: dict[str, Any]) -> dict[str, Any]:
    """Turn a stored style back into the keyword arguments that produced it."""
    style: dict[str, Any] = series.get("style", {})
    return {_STYLE_KEYWORDS.get(key, key): value for key, value in style.items()}


#: Snapshot key -> (method name shared by ``Plot`` and the native handle, whether
#: the stored value is an argument list). Applied before the series so replayed
#: plots and rebuilt native handles configure the axes identically.
_PLOT_SETTINGS: tuple[tuple[str, str, bool], ...] = (
    ("sizePx", "size_px", True),
    ("dpi", "dpi", False),
    ("theme", "theme", False),
    ("ticks", "ticks", False),
    ("title", "title", False),
    ("xLabel", "xlabel", False),
    ("yLabel", "ylabel", False),
    ("legend", "legend", False),
    ("grid", "grid", False),
    ("xLim", "xlim", True),
    ("yLim", "ylim", True),
    ("xScale", "xscale", True),
    ("yScale", "yscale", True),
)


@dataclass
class _ObservableDerivation:
    ufunc: np.ufunc
    inputs: tuple[Any, ...]
    #: One ``(source, token)`` pair per distinct observable operand, so the
    #: derived series can unregister itself from each source.
    bindings: list[tuple["ObservableSeries", int]]


class ObservableSeries:
    """Mutable numeric data source for notebook-driven updates."""

    __array_priority__ = 1000

    def __init__(self, values: ArrayLike) -> None:
        """Create an observable numeric series from array-like values."""
        self._initialize(_to_numeric_1d(values, "observable values"))

    def _initialize(self, values: _F64Array) -> None:
        self._values = values
        self._native_observable = _native.NativeObservable1D(self._values)
        self._listeners: dict[int, weakref.ReferenceType[Any] | weakref.WeakMethod[Any]] = {}
        self._next_listener_token = 0
        self._derivation: _ObservableDerivation | None = None
        self._resize_guards: dict[int, weakref.WeakMethod[Any]] = {}
        self._derived_children: dict[int, weakref.ref["ObservableSeries"]] = {}

    @classmethod
    def _from_values(cls, values: _F64Array) -> "ObservableSeries":
        observable = cls.__new__(cls)
        observable._initialize(values)
        return observable

    @classmethod
    def _from_ufunc(cls, ufunc: np.ufunc, *inputs: Any) -> "ObservableSeries":
        normalized_inputs = tuple(_normalize_observable_math_input(value) for value in inputs)
        if not any(isinstance(value, ObservableSeries) for value in normalized_inputs):
            return cls(cls._evaluate_ufunc(ufunc, normalized_inputs))

        observable = cls._from_values(cls._evaluate_ufunc(ufunc, normalized_inputs))
        bindings: list[tuple[ObservableSeries, int]] = []
        attached_sources: set[int] = set()
        for value in normalized_inputs:
            if not isinstance(value, ObservableSeries):
                continue
            source_id = id(value)
            if source_id in attached_sources:
                continue
            attached_sources.add(source_id)
            token = value._register_derived(observable)
            bindings.append((value, token))
            weakref.finalize(observable, value._unregister_derived, token)

        observable._derivation = _ObservableDerivation(ufunc=ufunc, inputs=normalized_inputs, bindings=bindings)
        return observable

    @staticmethod
    def _input_length(value: Any) -> int | None:
        if isinstance(value, ObservableSeries):
            return len(value._values)
        if isinstance(value, np.ndarray):
            return len(value)
        return None

    @staticmethod
    def _materialize_input(value: Any) -> float | _F64Array:
        if isinstance(value, ObservableSeries):
            return value._values
        if isinstance(value, np.ndarray):
            return value
        return float(value)

    @classmethod
    def _evaluate_ufunc(cls, ufunc: np.ufunc, inputs: tuple[Any, ...]) -> _F64Array:
        lengths = {length for value in inputs if (length := cls._input_length(value)) is not None}
        if len(lengths) > 1:
            raise ValueError("observable math operands must have the same length")

        try:
            result = ufunc(*[cls._materialize_input(value) for value in inputs])
        except ValueError as err:
            raise ValueError("observable math operands must have the same length") from err
        except TypeError as err:
            raise TypeError("unsupported observable math operation") from err

        array = np.array(result, dtype=np.float64, order="C")
        if array.ndim != 1:
            raise TypeError("observable math must produce a 1D numeric result")
        return array

    def _detach_derivation(self) -> None:
        if self._derivation is None:
            return

        for source, token in self._derivation.bindings:
            source._unregister_derived(token)
        self._derivation = None

    def _recompute(self) -> None:
        """Re-evaluate this derived series from its (already updated) sources."""
        if self._derivation is None:
            return

        self._values = self._evaluate_ufunc(self._derivation.ufunc, self._derivation.inputs)
        self._native_observable.replace(self._values)

    def _ensure_detached(self) -> None:
        if self._derivation is not None:
            self._detach_derivation()

    def _attach_resize_guard(self, guard: Any) -> int:
        token = self._next_listener_token
        self._next_listener_token += 1
        self._resize_guards[token] = weakref.WeakMethod(guard)
        return token

    def _detach_resize_guard(self, token: int) -> None:
        self._resize_guards.pop(token, None)

    def _register_derived(self, child: "ObservableSeries") -> int:
        token = self._next_listener_token
        self._next_listener_token += 1
        self._derived_children[token] = weakref.ref(child)
        return token

    def _unregister_derived(self, token: int) -> None:
        self._derived_children.pop(token, None)

    def _check_resize(self, new_length: int, prospective: dict[int, int]) -> None:
        """Let bound plots veto a length change before it is applied."""
        if new_length == len(self._values):
            return
        for token, guard_ref in list(self._resize_guards.items()):
            guard = guard_ref()
            if guard is None:
                self._resize_guards.pop(token, None)
                continue
            guard(self, new_length, prospective)

    def _live_derived_children(self) -> list["ObservableSeries"]:
        """Return the derived observables still tracking this one, pruning dead refs."""
        children: list[ObservableSeries] = []
        for token, child_ref in list(self._derived_children.items()):
            child = child_ref()
            if child is None:
                self._derived_children.pop(token, None)
                continue
            if child._derivation is not None:
                children.append(child)
        return children

    def _derivation_plan(self, new_length: int) -> tuple[list["ObservableSeries"], dict[int, int]]:
        """Plan an update: every reachable observable, in dependency order.

        Returns the affected observables topologically sorted (this one first)
        together with the length each will hold afterwards. Sizing a derived
        observable only once *all* of its affected operands are known is what
        keeps a diamond-shaped graph — two derived siblings feeding one child —
        from reporting an operand mismatch that the completed update would not
        have. Derivation graphs are acyclic, so the sort always completes.
        """
        affected: dict[int, ObservableSeries] = {id(self): self}
        children: dict[int, list[ObservableSeries]] = {}
        frontier: list[ObservableSeries] = [self]
        while frontier:
            node = frontier.pop()
            children[id(node)] = node._live_derived_children()
            for child in children[id(node)]:
                if id(child) in affected:
                    continue
                affected[id(child)] = child
                frontier.append(child)

        # How many affected operands each derived node is still waiting for.
        pending = {
            key: len(
                {
                    id(value)
                    for value in cast(_ObservableDerivation, node._derivation).inputs
                    if isinstance(value, ObservableSeries) and id(value) in affected
                }
            )
            for key, node in affected.items()
            if node is not self
        }

        order: list[ObservableSeries] = [self]
        prospective = {id(self): new_length}
        queue: list[ObservableSeries] = [self]
        while queue:
            node = queue.pop()
            for child in children[id(node)]:
                pending[id(child)] -= 1
                if pending[id(child)] > 0:
                    continue
                prospective[id(child)] = self._planned_length(child, prospective)
                order.append(child)
                queue.append(child)

        return order, prospective

    @staticmethod
    def _planned_length(child: "ObservableSeries", prospective: dict[int, int]) -> int:
        """Return the length ``child`` takes once its planned operands are applied."""
        derivation = cast(_ObservableDerivation, child._derivation)
        lengths = {
            prospective.get(id(value), len(value._values))
            if isinstance(value, ObservableSeries)
            else len(value)
            for value in derivation.inputs
            if isinstance(value, (ObservableSeries, np.ndarray))
        }
        if len(lengths) > 1:
            raise ValueError("observable math operands must have the same length")
        return lengths.pop()

    def _prevalidate_resize(
        self, order: list["ObservableSeries"], prospective: dict[int, int]
    ) -> None:
        """Let every affected plot veto the planned lengths before anything mutates.

        Guards run only once the whole plan exists, so a series input that
        resizes alongside its siblings is judged at its planned length rather
        than the one it still has.
        """
        for observable in order:
            observable._check_resize(prospective[id(observable)], prospective)

    def _propagate(self, order: list["ObservableSeries"]) -> None:
        """Recompute derived observables in dependency order, then notify listeners.

        Recomputing before notifying keeps every operand of a derived series at
        its post-update length, which a listener-driven cascade cannot guarantee.
        """
        for observable in order[1:]:
            observable._recompute()
        for observable in order:
            observable._notify()

    def __copy__(self) -> "ObservableSeries":
        return self.__deepcopy__({})

    def __deepcopy__(self, memo: dict[int, Any]) -> "ObservableSeries":
        existing = memo.get(id(self))
        if existing is not None:
            return existing

        if self._derivation is None:
            clone = type(self)(self._values)
            memo[id(self)] = clone
            return clone

        copied_inputs = tuple(deepcopy(value, memo) for value in self._derivation.inputs)
        clone = type(self)._from_ufunc(self._derivation.ufunc, *copied_inputs)
        memo[id(self)] = clone
        return clone

    def replace(self, values: ArrayLike) -> None:
        """Replace the entire series and notify attached widgets.

        Raises ValueError before any state changes when the new length would
        break a bound plot series whose inputs must stay equal-length (line,
        scatter, bar, error bars) — including series bound to observables
        derived from this one. Observables derived from this one resize with it.

        On a derived observable this permanently detaches it from its sources:
        it keeps the values you set and stops tracking later source updates.
        """
        next_values = _to_numeric_1d(values, "observable values")
        order, prospective = self._derivation_plan(len(next_values))
        self._prevalidate_resize(order, prospective)
        self._ensure_detached()
        self._values = next_values
        self._native_observable.replace(self._values)
        self._propagate(order)

    def set_at(self, index: int, value: float) -> None:
        """Update a single element in-place and notify attached widgets.

        On a derived observable this permanently detaches it from its sources:
        it keeps the value you set and stops tracking later source updates.
        """
        if index < 0 or index >= len(self._values):
            raise IndexError("observable index is out of bounds")
        normalized_value = float(value)
        order, _ = self._derivation_plan(len(self._values))
        self._ensure_detached()
        self._values[index] = normalized_value
        self._native_observable.set_at(index, normalized_value)
        self._propagate(order)

    def values(self) -> npt.NDArray[np.float64]:
        """Return the current values as a NumPy array."""
        return self._values.copy()

    def snapshot_values(self) -> list[float]:
        """Return the current values as a plain Python list."""
        return cast("list[float]", self._values.tolist())

    def __len__(self) -> int:
        """Return the number of values currently in the series."""
        return len(self._values)

    def __getitem__(self, index: int) -> float:
        """Return one value by index; negative indices count from the end.

        Slices are intentionally not supported: use :meth:`values` for a NumPy
        array of the whole series.
        """
        if not isinstance(index, (int, np.integer)) or isinstance(index, bool):
            raise TypeError("ObservableSeries indices must be integers; slicing is not supported")
        return float(self._values[index])

    def __array__(self, dtype: Any = None, copy: bool | None = None) -> npt.NDArray[Any]:
        """Expose the series to NumPy, honoring the NumPy 2 ``copy`` contract.

        ``copy=False`` means *never copy*: it returns a read-only view of the
        stored float64 buffer, or raises when a dtype conversion would force a
        copy. ``copy=None`` (the default) and ``copy=True`` both copy, so the
        caller can never write through to the series.
        """
        if copy is False:
            if dtype is not None and np.dtype(dtype) != self._values.dtype:
                raise ValueError(
                    "cannot return an ObservableSeries view with a different dtype "
                    "without copying"
                )
            view = self._values.view()
            view.flags.writeable = False
            return view

        return self._values.astype(np.float64 if dtype is None else dtype)

    def __array_ufunc__(self, ufunc: np.ufunc, method: str, *inputs: Any, **kwargs: Any) -> Any:
        if method != "__call__":
            raise TypeError("ObservableSeries only supports direct elementwise NumPy ufunc calls")
        if kwargs:
            raise TypeError("ObservableSeries ufunc calls do not support keyword arguments")
        if ufunc.nout != 1:
            raise TypeError("ObservableSeries only supports single-output NumPy ufuncs")

        normalized_inputs = tuple(_normalize_observable_math_input(value) for value in inputs)
        return type(self)._from_ufunc(ufunc, *normalized_inputs)

    def __neg__(self) -> "ObservableSeries":
        return type(self)._from_ufunc(np.negative, self)

    def __pos__(self) -> "ObservableSeries":
        return type(self)._from_ufunc(np.positive, self)

    def __abs__(self) -> "ObservableSeries":
        return type(self)._from_ufunc(np.absolute, self)

    def __add__(self, other: float | ArrayLike) -> "ObservableSeries":
        return type(self)._from_ufunc(np.add, self, other)

    def __radd__(self, other: float | ArrayLike) -> "ObservableSeries":
        return type(self)._from_ufunc(np.add, other, self)

    def __sub__(self, other: float | ArrayLike) -> "ObservableSeries":
        return type(self)._from_ufunc(np.subtract, self, other)

    def __rsub__(self, other: float | ArrayLike) -> "ObservableSeries":
        return type(self)._from_ufunc(np.subtract, other, self)

    def __mul__(self, other: float | ArrayLike) -> "ObservableSeries":
        return type(self)._from_ufunc(np.multiply, self, other)

    def __rmul__(self, other: float | ArrayLike) -> "ObservableSeries":
        return type(self)._from_ufunc(np.multiply, other, self)

    def __truediv__(self, other: float | ArrayLike) -> "ObservableSeries":
        return type(self)._from_ufunc(np.true_divide, self, other)

    def __rtruediv__(self, other: float | ArrayLike) -> "ObservableSeries":
        return type(self)._from_ufunc(np.true_divide, other, self)

    def __floordiv__(self, other: float | ArrayLike) -> "ObservableSeries":
        return type(self)._from_ufunc(np.floor_divide, self, other)

    def __rfloordiv__(self, other: float | ArrayLike) -> "ObservableSeries":
        return type(self)._from_ufunc(np.floor_divide, other, self)

    def __pow__(self, other: float | ArrayLike) -> "ObservableSeries":
        return type(self)._from_ufunc(np.power, self, other)

    def __rpow__(self, other: float | ArrayLike) -> "ObservableSeries":
        return type(self)._from_ufunc(np.power, other, self)

    def _snapshot(self) -> dict[str, Any]:
        return {"kind": "observable", "values": self._values}

    def _attach(self, listener: Any) -> int:
        token = self._next_listener_token
        self._next_listener_token += 1
        if hasattr(listener, "__self__") and getattr(listener, "__self__", None) is not None:
            listener_ref = weakref.WeakMethod(listener)
        else:
            listener_ref = weakref.ref(listener)
        self._listeners[token] = listener_ref
        return token

    def _detach(self, token: int) -> None:
        self._listeners.pop(token, None)

    def _notify(self) -> None:
        for token, listener_ref in list(self._listeners.items()):
            listener = listener_ref()
            if listener is None:
                self._listeners.pop(token, None)
                continue
            listener()


#: ``_typing.ArrayLike`` names this class through a forward reference bound to
#: the ``_typing`` module; publishing it there is what lets
#: :func:`typing.get_type_hints` resolve the alias from any calling module.
_typing.ObservableSeries = ObservableSeries


class Plot:
    """Fluent plot builder for static and interactive ruviz rendering."""

    def __init__(self) -> None:
        self._state: dict[str, Any] = {"schemaVersion": _SNAPSHOT_SCHEMA_VERSION, "series": []}
        self._native_plot = _native.NativePlotHandle()
        self._widgets: "weakref.WeakSet[Any]" = weakref.WeakSet()
        self._observables: list[ObservableSeries] = []
        self._observable_listener_tokens: dict[ObservableSeries, int] = {}
        self._observable_bindings: list[tuple[ObservableSeries, dict[str, Any], str]] = []
        self._snapshot_cache: dict[str, Any] | None = None
        self._snapshot_dirty = True
        self._refresh_scheduled = False

    def _invalidate_snapshot_cache(self) -> None:
        self._snapshot_dirty = True
        self._snapshot_cache = None

    def _build_native_numeric_source(
        self, value: Any, name: str = "numeric input"
    ) -> tuple[dict[str, Any], _F64Array | Any, ObservableSeries | None]:
        if isinstance(value, ObservableSeries):
            snapshot = value._snapshot()
            return snapshot, value._native_observable, value

        values = _to_numeric_1d(value, name)
        return {"kind": "static", "values": values}, values, None

    @staticmethod
    def _apply_native_series(
        native_plot: Any,
        series: dict[str, Any],
        *,
        native_sources: dict[str, Any] | None = None,
    ) -> None:
        native_sources = native_sources or {}
        spec, method = _series_kind(series)
        if spec.native_args is not None:
            args = spec.native_args(series)
        else:
            args = [
                native_sources.get(key, series[key]["values"])
                if key in spec.sources
                else series[key]
                for key in spec.args
            ]
        if spec.style:
            args.append(series.get("style", {}))
        getattr(native_plot, method)(*args)

    def _append_series_snapshot(self, series: dict[str, Any]) -> None:
        self._state["series"].append(series)
        self._invalidate_snapshot_cache()

    def _rebuild_native_plot(self, snapshot: dict[str, Any]) -> None:
        """Rebuild the native handle from a static snapshot copy.

        Any observable-backed series in the original plot are rebuilt from the
        snapshot's current numeric values, so the rebuilt native plot is a
        static copy and does not retain live observable links.
        """
        native_plot = _native.NativePlotHandle()
        self._apply_snapshot_metadata(native_plot, snapshot)

        for series in snapshot["series"]:
            self._apply_native_series(native_plot, series)

        self._native_plot = native_plot

    @staticmethod
    def _apply_snapshot_metadata(target: Any, snapshot: dict[str, Any]) -> None:
        """Replay plot-level settings onto a ``Plot`` or a native handle."""
        for key, method, unpack in _PLOT_SETTINGS:
            value = snapshot.get(key)
            if value is None:
                continue
            setter = getattr(target, method)
            setter(*value) if unpack else setter(value)

    @staticmethod
    def _resolve_numeric_source(
        source: dict[str, Any],
        observable_lookup: dict[int, ObservableSeries],
    ) -> Any:
        if source["kind"] == "observable":
            return observable_lookup.get(id(source), source["values"])
        return source["values"]

    @classmethod
    def _replay_snapshot(
        cls,
        snapshot: dict[str, Any],
        observable_lookup: dict[int, ObservableSeries] | None = None,
    ) -> "Plot":
        observable_lookup = observable_lookup or {}
        plot = cls()
        cls._apply_snapshot_metadata(plot, snapshot)

        for series in snapshot["series"]:
            spec, method = _series_kind(series)
            if spec.public_args is not None:
                args = spec.public_args(series)
            else:
                args = [
                    cls._resolve_numeric_source(series[key], observable_lookup)
                    if key in spec.sources
                    else series[key]
                    for key in spec.args
                ]
            getattr(plot, method)(*args, **_style_keywords(series))

        return plot

    def __copy__(self) -> "Plot":
        return self.__deepcopy__({})

    def __deepcopy__(self, memo: dict[int, Any]) -> "Plot":
        existing = memo.get(id(self))
        if existing is not None:
            return existing

        observable_lookup = {
            id(series[key]): deepcopy(observable, memo)
            for observable, series, key in self._observable_bindings
        }
        clone = type(self)._replay_snapshot(self._state, observable_lookup)
        memo[id(self)] = clone
        return clone

    def clone(self) -> "Plot":
        """Return a static snapshot copy of the current plot.

        Observable-backed series are copied by value, so the clone renders the
        same current data but does not stay linked to later observable updates.
        """
        self._sync_observables()
        clone = Plot()
        clone._state = _freeze_observable_sources(deepcopy(self._state))
        clone._rebuild_native_plot(clone._state)
        return clone

    def size_px(self, width: int, height: int) -> "Plot":
        """Set the pixel size used for export and notebook rendering."""
        normalized_width = _exact_int(width, _SIZE_PX_MESSAGE)
        normalized_height = _exact_int(height, _SIZE_PX_MESSAGE)
        if normalized_width <= 0 or normalized_height <= 0:
            raise ValueError(_SIZE_PX_MESSAGE)
        self._native_plot.size_px(normalized_width, normalized_height)
        self._state["sizePx"] = [normalized_width, normalized_height]
        self._invalidate_snapshot_cache()
        return self

    def dpi(self, dpi: int) -> "Plot":
        """Set output dots per inch, scaling the exported pixels from ``size_px``."""
        normalized = _exact_int(dpi, _DPI_MESSAGE)
        if normalized < 1:
            raise ValueError(_DPI_MESSAGE)
        self._native_plot.dpi(normalized)
        self._state["dpi"] = normalized
        self._invalidate_snapshot_cache()
        return self

    def theme(self, theme: Theme) -> "Plot":
        """Set the built-in ``light`` or ``dark`` theme (case-insensitive)."""
        normalized = str(theme).lower()
        if normalized not in {"light", "dark"}:
            raise ValueError(f"unsupported theme: {theme}")
        self._native_plot.theme(normalized)
        self._state["theme"] = normalized
        self._invalidate_snapshot_cache()
        return self

    def ticks(self, enabled: bool) -> "Plot":
        """Enable or disable axis ticks."""
        normalized = bool(enabled)
        self._native_plot.ticks(normalized)
        self._state["ticks"] = normalized
        self._invalidate_snapshot_cache()
        return self

    def title(self, title: str) -> "Plot":
        """Set the plot title."""
        normalized = str(title)
        self._native_plot.title(normalized)
        self._state["title"] = normalized
        self._invalidate_snapshot_cache()
        return self

    def xlabel(self, label: str) -> "Plot":
        """Set the x-axis label."""
        normalized = str(label)
        self._native_plot.xlabel(normalized)
        self._state["xLabel"] = normalized
        self._invalidate_snapshot_cache()
        return self

    def ylabel(self, label: str) -> "Plot":
        """Set the y-axis label."""
        normalized = str(label)
        self._native_plot.ylabel(normalized)
        self._state["yLabel"] = normalized
        self._invalidate_snapshot_cache()
        return self

    def legend(self, position: LegendPositionName = "best") -> "Plot":
        """Show the legend at ``position``.

        Accepts ``"best"`` plus the core legend positions as lowercase names,
        such as ``"upper_right"``, ``"center"``, or ``"outside_right"``.
        """
        normalized = str(position).strip().lower().replace("-", "_").replace(" ", "_")
        self._native_plot.legend(normalized)
        self._state["legend"] = normalized
        self._invalidate_snapshot_cache()
        return self

    def grid(self, enabled: bool = True) -> "Plot":
        """Show or hide the axis grid."""
        normalized = bool(enabled)
        self._native_plot.grid(normalized)
        self._state["grid"] = normalized
        self._invalidate_snapshot_cache()
        return self

    def _set_limit(self, axis: str, minimum: float, maximum: float) -> "Plot":
        lower = float(minimum)
        upper = float(maximum)
        if not np.isfinite(lower) or not np.isfinite(upper) or lower == upper:
            raise ValueError(f"{axis} limits must be finite and different")
        getattr(self._native_plot, f"{axis}lim")(lower, upper)
        self._state[f"{axis}Lim"] = [lower, upper]
        self._invalidate_snapshot_cache()
        return self

    def xlim(self, minimum: float, maximum: float) -> "Plot":
        """Set finite, unequal x-axis limits; inverted bounds render a descending axis."""
        return self._set_limit("x", minimum, maximum)

    def ylim(self, minimum: float, maximum: float) -> "Plot":
        """Set finite, unequal y-axis limits; inverted bounds render a descending axis."""
        return self._set_limit("y", minimum, maximum)

    def _set_scale(self, axis: str, scale: ScaleName, linthresh: float | None) -> "Plot":
        normalized = str(scale).strip().lower()
        args: list[Any] = [normalized]
        if normalized == "symlog":
            threshold = 1.0 if linthresh is None else float(linthresh)
            if not np.isfinite(threshold) or threshold <= 0.0:
                raise ValueError("symlog linthresh must be a finite positive number")
            args.append(threshold)
        elif linthresh is not None:
            raise ValueError("linthresh only applies to the symlog scale")
        getattr(self._native_plot, f"{axis}scale")(*args)
        self._state[f"{axis}Scale"] = args
        self._invalidate_snapshot_cache()
        return self

    def xscale(self, scale: ScaleName, linthresh: float | None = None) -> "Plot":
        """Set the x-axis scale to ``linear``, ``log``, or ``symlog``."""
        return self._set_scale("x", scale, linthresh)

    def yscale(self, scale: ScaleName, linthresh: float | None = None) -> "Plot":
        """Set the y-axis scale to ``linear``, ``log``, or ``symlog``."""
        return self._set_scale("y", scale, linthresh)

    def line(
        self,
        x: ArrayLike | str,
        y: ArrayLike | str,
        *,
        data: DataSource = None,
        label: str | None = None,
        color: str | None = None,
        width: float | None = None,
        alpha: float | None = None,
        linestyle: LineStyleName | None = None,
        marker: MarkerName | None = None,
        marker_size: float | None = None,
    ) -> "Plot":
        """Add a line series from x/y arrays or dataframe columns.

        ``color`` takes a hex string (``"#2563eb"``) or a named color,
        ``linestyle`` one of solid/dashed/dotted/dash-dot/dash-dot-dot, and
        ``marker`` one of circle/square/triangle/triangle-down/diamond/plus/
        cross/star/circle-open/square-open/triangle-open/diamond-open. The
        matplotlib shorthands (``"o"``, ``"^"``, ``"--"``, ``":"``, ...) work
        wherever a marker or line style name does.
        """
        x_values, native_x, x_observable = self._build_native_numeric_source(
            _column_values(data, x), "line x"
        )
        y_values, native_y, y_observable = self._build_native_numeric_source(
            _column_values(data, y), "line y"
        )
        self._ensure_equal_length("line", x_values, y_values)
        series = _styled_series(
            "line",
            {"x": x_values, "y": y_values},
            {
                "label": label,
                "color": color,
                "width": width,
                "alpha": alpha,
                "linestyle": linestyle,
                "marker": marker,
                "markerSize": marker_size,
            },
        )
        self._apply_native_series(self._native_plot, series, native_sources={"x": native_x, "y": native_y})
        if x_observable is not None:
            self._track_observable(x_observable, series, "x")
        if y_observable is not None:
            self._track_observable(y_observable, series, "y")
        self._append_series_snapshot(series)
        return self

    def scatter(
        self,
        x: ArrayLike | str,
        y: ArrayLike | str,
        *,
        data: DataSource = None,
        label: str | None = None,
        color: str | None = None,
        alpha: float | None = None,
        marker: MarkerName | None = None,
        marker_size: float | None = None,
    ) -> "Plot":
        """Add a scatter series from x/y arrays or dataframe columns."""
        x_values, native_x, x_observable = self._build_native_numeric_source(
            _column_values(data, x), "scatter x"
        )
        y_values, native_y, y_observable = self._build_native_numeric_source(
            _column_values(data, y), "scatter y"
        )
        self._ensure_equal_length("scatter", x_values, y_values)
        series = _styled_series(
            "scatter",
            {"x": x_values, "y": y_values},
            {
                "label": label,
                "color": color,
                "alpha": alpha,
                "marker": marker,
                "markerSize": marker_size,
            },
        )
        self._apply_native_series(self._native_plot, series, native_sources={"x": native_x, "y": native_y})
        if x_observable is not None:
            self._track_observable(x_observable, series, "x")
        if y_observable is not None:
            self._track_observable(y_observable, series, "y")
        self._append_series_snapshot(series)
        return self

    def bar(
        self,
        x: LabelsLike | str,
        y: ArrayLike | str,
        *,
        data: DataSource = None,
        label: str | None = None,
        color: str | None = None,
        alpha: float | None = None,
    ) -> "Plot":
        """Add a categorical bar series."""
        categories = _to_string_list(_column_values(data, x), "bar categories")
        values, native_values, observable = self._build_native_numeric_source(
            _column_values(data, y), "bar values"
        )
        if len(categories) != len(values["values"]):
            raise ValueError("bar categories and values must have the same length")
        series = _styled_series(
            "bar",
            {"categories": categories, "values": values},
            {"label": label, "color": color, "alpha": alpha},
        )
        self._apply_native_series(self._native_plot, series, native_sources={"values": native_values})
        if observable is not None:
            self._track_observable(observable, series, "values")
        self._append_series_snapshot(series)
        return self

    def histogram(
        self,
        x: ArrayLike | str,
        *,
        data: DataSource = None,
        bins: int | None = None,
        density: bool = False,
        label: str | None = None,
        color: str | None = None,
        alpha: float | None = None,
    ) -> "Plot":
        """Add a histogram from one numeric sample vector.

        ``density=True`` normalizes the bars to a probability density, which is
        what a :meth:`kde` overlay is drawn on; without it the KDE curve sits
        flat at zero against a counts axis.
        """
        series_data, native_data, observable = self._build_native_numeric_source(
            _column_values(data, x), "histogram x"
        )
        series = _styled_series(
            "histogram",
            {"data": series_data},
            {
                "bins": bins,
                "density": density,
                "label": label,
                "color": color,
                "alpha": alpha,
            },
        )
        self._apply_native_series(self._native_plot, series, native_sources={"data": native_data})
        if observable is not None:
            self._track_observable(observable, series, "data")
        self._append_series_snapshot(series)
        return self

    def boxplot(
        self,
        x: ArrayLike | str,
        *,
        data: DataSource = None,
        label: str | None = None,
        color: str | None = None,
        alpha: float | None = None,
        width: float | None = None,
        linestyle: LineStyleName | None = None,
    ) -> "Plot":
        """Add a boxplot from one numeric sample vector."""
        series_data, native_data, observable = self._build_native_numeric_source(
            _column_values(data, x), "boxplot x"
        )
        series = _styled_series(
            "boxplot",
            {"data": series_data},
            {
                "label": label,
                "color": color,
                "alpha": alpha,
                "width": width,
                "linestyle": linestyle,
            },
        )
        self._apply_native_series(self._native_plot, series, native_sources={"data": native_data})
        if observable is not None:
            self._track_observable(observable, series, "data")
        self._append_series_snapshot(series)
        return self

    def heatmap(self, values: MatrixLike | str, *, data: DataSource = None) -> "Plot":
        """Add a heatmap from a rectangular 2D numeric matrix.

        With ``data=``, ``values`` may name a matrix column/key to look up.
        """
        matrix = _reject_observable(_column_values(data, values), "heatmap")
        if _is_dataframe(matrix):
            raise TypeError("heatmap values must be a 2D numeric matrix; pass DataFrame.to_numpy()")
        rows = [_to_static_numeric_1d(row, "heatmap", "row") for row in matrix]
        if not rows or len(rows[0]) == 0:
            raise ValueError("heatmap input must be a non-empty 2D numeric matrix")
        cols = len(rows[0])
        if any(len(row) != cols for row in rows):
            raise ValueError("heatmap rows must all have the same length")
        flattened = np.concatenate(rows)
        series = {"kind": "heatmap", "values": flattened, "rows": len(rows), "cols": cols}
        self._apply_native_series(self._native_plot, series)
        self._append_series_snapshot(series)
        return self

    def error_bars(
        self,
        x: ArrayLike | str,
        y: ArrayLike | str,
        y_errors: ArrayLike | str,
        *,
        data: DataSource = None,
        label: str | None = None,
        color: str | None = None,
        alpha: float | None = None,
        width: float | None = None,
    ) -> "Plot":
        """Add a series with vertical error bars."""
        x_values, native_x, x_observable = self._build_native_numeric_source(
            _column_values(data, x), "error_bars x"
        )
        y_values, native_y, y_observable = self._build_native_numeric_source(
            _column_values(data, y), "error_bars y"
        )
        error_values, native_errors, error_observable = self._build_native_numeric_source(
            _column_values(data, y_errors), "error_bars y_errors"
        )
        self._ensure_equal_length("error-bars", x_values, y_values, error_values)
        series = _styled_series(
            "error-bars",
            {"x": x_values, "y": y_values, "yErrors": error_values},
            {"label": label, "color": color, "alpha": alpha, "width": width},
        )
        self._apply_native_series(
            self._native_plot,
            series,
            native_sources={"x": native_x, "y": native_y, "yErrors": native_errors},
        )
        if x_observable is not None:
            self._track_observable(x_observable, series, "x")
        if y_observable is not None:
            self._track_observable(y_observable, series, "y")
        if error_observable is not None:
            self._track_observable(error_observable, series, "yErrors")
        self._append_series_snapshot(series)
        return self

    def error_bars_xy(
        self,
        x: ArrayLike | str,
        y: ArrayLike | str,
        x_errors: ArrayLike | str,
        y_errors: ArrayLike | str,
        *,
        data: DataSource = None,
        label: str | None = None,
        color: str | None = None,
        alpha: float | None = None,
        width: float | None = None,
    ) -> "Plot":
        """Add a series with both horizontal and vertical error bars."""
        x_values, native_x, x_observable = self._build_native_numeric_source(
            _column_values(data, x), "error_bars_xy x"
        )
        y_values, native_y, y_observable = self._build_native_numeric_source(
            _column_values(data, y), "error_bars_xy y"
        )
        x_error_values, native_x_errors, x_error_observable = self._build_native_numeric_source(
            _column_values(data, x_errors), "error_bars_xy x_errors"
        )
        y_error_values, native_y_errors, y_error_observable = self._build_native_numeric_source(
            _column_values(data, y_errors), "error_bars_xy y_errors"
        )
        self._ensure_equal_length("error-bars-xy", x_values, y_values, x_error_values, y_error_values)
        series = _styled_series(
            "error-bars-xy",
            {
                "x": x_values,
                "y": y_values,
                "xErrors": x_error_values,
                "yErrors": y_error_values,
            },
            {"label": label, "color": color, "alpha": alpha, "width": width},
        )
        self._apply_native_series(
            self._native_plot,
            series,
            native_sources={
                "x": native_x,
                "y": native_y,
                "xErrors": native_x_errors,
                "yErrors": native_y_errors,
            },
        )
        if x_observable is not None:
            self._track_observable(x_observable, series, "x")
        if y_observable is not None:
            self._track_observable(y_observable, series, "y")
        if x_error_observable is not None:
            self._track_observable(x_error_observable, series, "xErrors")
        if y_error_observable is not None:
            self._track_observable(y_error_observable, series, "yErrors")
        self._append_series_snapshot(series)
        return self

    def kde(
        self,
        x: ArrayLike | str,
        *,
        data: DataSource = None,
        bandwidth: float | None = None,
        label: str | None = None,
        color: str | None = None,
        alpha: float | None = None,
        width: float | None = None,
    ) -> "Plot":
        """Add a kernel density estimate for a numeric sample vector."""
        values = _to_static_numeric_1d(_column_values(data, x), "kde", "x")
        series = _styled_series(
            "kde",
            {"data": values},
            {
                "bandwidth": bandwidth,
                "label": label,
                "color": color,
                "alpha": alpha,
                "width": width,
            },
        )
        self._apply_native_series(self._native_plot, series)
        self._append_series_snapshot(series)
        return self

    def ecdf(
        self,
        x: ArrayLike | str,
        *,
        data: DataSource = None,
        label: str | None = None,
        color: str | None = None,
        alpha: float | None = None,
        width: float | None = None,
    ) -> "Plot":
        """Add an empirical cumulative distribution plot."""
        values = _to_static_numeric_1d(_column_values(data, x), "ecdf", "x")
        series = _styled_series(
            "ecdf",
            {"data": values},
            {"label": label, "color": color, "alpha": alpha, "width": width},
        )
        self._apply_native_series(self._native_plot, series)
        self._append_series_snapshot(series)
        return self

    def contour(
        self,
        x: ArrayLike | str,
        y: ArrayLike | str,
        z: ArrayLike | str,
        *,
        data: DataSource = None,
        levels: int | None = None,
        alpha: float | None = None,
        width: float | None = None,
    ) -> "Plot":
        """Add a contour plot from x/y axes and a flattened z grid.

        Contour lines take their colors from the colormap, so ``label`` and
        ``color`` are not offered here.
        """
        x_values = _to_static_numeric_1d(_column_values(data, x), "contour", "x")
        y_values = _to_static_numeric_1d(_column_values(data, y), "contour", "y")
        z_values = _to_static_numeric_1d(_column_values(data, z), "contour", "z")
        if len(z_values) != len(x_values) * len(y_values):
            raise ValueError("contour z must contain x.length * y.length values")
        series = _styled_series(
            "contour",
            {"x": x_values, "y": y_values, "z": z_values},
            {"levels": levels, "alpha": alpha, "width": width},
        )
        self._apply_native_series(self._native_plot, series)
        self._append_series_snapshot(series)
        return self

    def pie(
        self,
        values: ArrayLike | str,
        labels: LabelsLike | str | None = None,
        *,
        data: DataSource = None,
    ) -> "Plot":
        """Add a pie chart with optional labels."""
        numeric = _to_static_numeric_1d(_column_values(data, values), "pie", "values")
        label_values = (
            None if labels is None else _to_string_list(_column_values(data, labels), "pie labels")
        )
        if label_values is not None and len(label_values) != len(numeric):
            raise ValueError("pie values and labels must have the same length")
        series = {"kind": "pie", "values": numeric}
        if label_values is not None:
            series["labels"] = label_values
        self._apply_native_series(self._native_plot, series)
        self._append_series_snapshot(series)
        return self

    def radar(self, labels: LabelsLike, series: Sequence[RadarSeriesDict]) -> "Plot":
        """Add a radar chart from axis labels and named series.

        A series ``name`` is shown only once the plot asks for a legend, so pair
        named series with :meth:`legend`.
        """
        label_values = _to_string_list(labels, "radar labels")
        normalized = []
        for item in series:
            values = _to_static_numeric_1d(item["values"], "radar", "series values")
            if len(values) != len(label_values):
                raise ValueError("each radar series must match the labels length")
            normalized.append({"name": item.get("name"), "values": values})
        plot_series = {"kind": "radar", "labels": label_values, "series": normalized}
        self._apply_native_series(self._native_plot, plot_series)
        self._append_series_snapshot(plot_series)
        return self

    def violin(
        self,
        x: ArrayLike | str,
        *,
        data: DataSource = None,
        label: str | None = None,
        color: str | None = None,
        alpha: float | None = None,
        width: float | None = None,
    ) -> "Plot":
        """Add a violin plot from one numeric sample vector."""
        values = _to_static_numeric_1d(_column_values(data, x), "violin", "x")
        series = _styled_series(
            "violin",
            {"data": values},
            {"label": label, "color": color, "alpha": alpha, "width": width},
        )
        self._apply_native_series(self._native_plot, series)
        self._append_series_snapshot(series)
        return self

    def polar_line(
        self,
        r: ArrayLike | str,
        theta: ArrayLike | str,
        *,
        data: DataSource = None,
        label: str | None = None,
        color: str | None = None,
        alpha: float | None = None,
        width: float | None = None,
    ) -> "Plot":
        """Add a polar line from radius and angle vectors."""
        r_values = _to_static_numeric_1d(_column_values(data, r), "polar_line", "r")
        theta_values = _to_static_numeric_1d(_column_values(data, theta), "polar_line", "theta")
        if len(r_values) != len(theta_values):
            raise ValueError("polar r and theta must have the same length")
        series = _styled_series(
            "polar-line",
            {"r": r_values, "theta": theta_values},
            {"label": label, "color": color, "alpha": alpha, "width": width},
        )
        self._apply_native_series(self._native_plot, series)
        self._append_series_snapshot(series)
        return self

    def render_png(self) -> bytes:
        """Render the current plot to PNG bytes."""
        return bytes(self._native_plot.render_png_bytes())

    def _render_png_uncached(self) -> bytes:
        """Render PNG bytes without reusing the prepared frame cache."""
        return bytes(self._native_plot.render_png_bytes_uncached())

    def render_svg(self) -> str:
        """Render the current plot to an SVG document string."""
        return self._native_plot.render_svg()

    def save(self, path: str | Path) -> Path:
        """Save the current plot to a PNG, SVG, or PDF file."""
        output = Path(path)
        self._native_plot.save(str(output))
        return output

    def widget(self) -> "RuvizWidget":
        """Create an explicit synced Jupyter widget for this plot.

        Requires the optional widget extra (``pip install "ruviz[widget]"``);
        without it this raises :class:`ImportError`.
        """
        from ._widget import RuvizWidget

        widget = RuvizWidget(self)
        self._widgets.add(widget)
        return widget

    def _notebook_image(self) -> Any:
        from IPython.display import Image

        return Image(data=self.render_png(), format="png")

    def show(self) -> None:
        """Display a static image in Jupyter or open a native interactive window when available."""
        if _is_notebook():
            from IPython.display import display

            image = self._notebook_image()
            display(image)
            return None

        self._native_plot.show_native()
        return None

    def to_snapshot(self) -> PlotSnapshot:
        """Serialize the current plot state to a JSON-friendly snapshot."""
        self._sync_observables()
        if self._snapshot_dirty or self._snapshot_cache is None:
            self._snapshot_cache = _materialize(self._state)
            self._snapshot_dirty = False
        return cast(PlotSnapshot, _copy_materialized(self._snapshot_cache))

    def _track_observable(self, observable: ObservableSeries, series: dict[str, Any], key: str) -> None:
        self._observable_bindings.append((observable, series, key))
        if observable in self._observables:
            return
        self._observables.append(observable)
        token = observable._attach(self._notify_widgets)
        self._observable_listener_tokens[observable] = token
        weakref.finalize(self, observable._detach, token)
        guard_token = observable._attach_resize_guard(self._guard_observable_resize)
        weakref.finalize(self, observable._detach_resize_guard, guard_token)

    def _guard_observable_resize(
        self,
        observable: ObservableSeries,
        new_length: int,
        prospective: dict[int, int],
    ) -> None:
        """Reject an observable length change that would break a bound series.

        ``prospective`` maps observable ids onto their planned lengths, so a
        sibling input that resizes alongside ``observable`` is compared at the
        length it is about to take. Sources are visited in sorted order to keep
        the reported sibling deterministic.
        """
        source_owner = {id(series[key]): bound for bound, series, key in self._observable_bindings}
        for bound, series, key in self._observable_bindings:
            if bound is not observable:
                continue
            kind = series["kind"]
            if kind == "bar" and len(series["categories"]) != new_length:
                raise ValueError(
                    f"cannot resize observable to {new_length} values: bar categories "
                    f"have length {len(series['categories'])}"
                )
            for other_key in sorted(_SERIES_KINDS[kind].sources):
                if other_key == key:
                    continue
                sibling = series[other_key]
                owner = source_owner.get(id(sibling))
                if owner is observable:
                    continue
                sibling_length = (
                    len(sibling["values"])
                    if owner is None
                    else prospective.get(id(owner), len(owner._values))
                )
                if sibling_length != new_length:
                    raise ValueError(
                        f"cannot resize observable to {new_length} values: {kind} series "
                        f"input '{other_key}' has length {sibling_length}"
                    )

    def _sync_observables(self) -> None:
        for observable, series, key in self._observable_bindings:
            series[key]["values"] = observable._values

    def _notify_widgets(self) -> None:
        """Refresh attached widgets, coalescing bursts under a running event loop."""
        self._invalidate_snapshot_cache()
        if not self._widgets:
            return

        try:
            loop = asyncio.get_running_loop()
        except RuntimeError:
            self._refresh_widgets()
            return

        if self._refresh_scheduled:
            return
        self._refresh_scheduled = True
        loop.call_soon(self._flush_widget_refresh)

    def _flush_widget_refresh(self) -> None:
        self._refresh_scheduled = False
        self._refresh_widgets()

    def _refresh_widgets(self) -> None:
        for widget in list(self._widgets):
            widget.refresh()

    def _ensure_equal_length(self, name: str, *sources: dict[str, Any]) -> None:
        lengths = [len(source["values"]) for source in sources]
        if len(set(lengths)) != 1:
            raise ValueError(f"{name} inputs must have the same length")

    def _repr_png_(self) -> bytes:
        """Return PNG bytes for notebook rich display."""
        return self.render_png()


class Plot3D:
    """Static fluent builder for the opt-in opaque 3D alpha.

    3D inputs are snapshotted when a series is added. The initial Python API
    intentionally exposes deterministic CPU PNG/SVG/PDF export; interactive
    orbit widgets and transparent surfaces remain outside the alpha contract.
    """

    def __init__(self) -> None:
        self._state: dict[str, Any] = {"schemaVersion": _SNAPSHOT_SCHEMA_VERSION, "series": []}
        self._native_plot = _native.NativePlot3DHandle()

    def _add_points(
        self,
        kind: str,
        x: ArrayLike | str,
        y: ArrayLike | str,
        z: ArrayLike | str,
        data: DataSource,
    ) -> "Plot3D":
        x_values = _to_static_numeric_1d(_column_values(data, x), kind, "x")
        y_values = _to_static_numeric_1d(_column_values(data, y), kind, "y")
        z_values = _to_static_numeric_1d(_column_values(data, z), kind, "z")
        if len({len(x_values), len(y_values), len(z_values)}) != 1:
            raise ValueError(f"{kind} x, y, and z inputs must have the same length")
        getattr(self._native_plot, kind)(x_values, y_values, z_values)
        self._state["series"].append({"kind": kind, "x": x_values, "y": y_values, "z": z_values})
        return self

    def _add_grid(
        self,
        kind: str,
        x: ArrayLike | str,
        y: ArrayLike | str,
        z: MatrixLike | str,
        data: DataSource,
    ) -> "Plot3D":
        x_values = _to_static_numeric_1d(_column_values(data, x), kind, "x")
        y_values = _to_static_numeric_1d(_column_values(data, y), kind, "y")
        z_values = _to_numeric_2d(_reject_observable(_column_values(data, z), kind), f"{kind} z")
        shape = (int(z_values.shape[0]), int(z_values.shape[1]))
        expected = (len(y_values), len(x_values))
        if shape != expected:
            raise ValueError(
                f"{kind} z shape must be (len(y), len(x)); expected {expected}, got {shape}"
            )
        getattr(self._native_plot, kind)(x_values, y_values, z_values)
        self._state["series"].append({"kind": kind, "x": x_values, "y": y_values, "z": z_values})
        return self

    def scatter3d(
        self,
        x: ArrayLike | str,
        y: ArrayLike | str,
        z: ArrayLike | str,
        *,
        data: DataSource = None,
    ) -> "Plot3D":
        """Add an opaque 3D scatter series from equal-length coordinate vectors."""
        return self._add_points("scatter3d", x, y, z, data)

    def line3d(
        self,
        x: ArrayLike | str,
        y: ArrayLike | str,
        z: ArrayLike | str,
        *,
        data: DataSource = None,
    ) -> "Plot3D":
        """Add a 3D polyline from equal-length coordinate vectors."""
        return self._add_points("line3d", x, y, z, data)

    def surface(
        self,
        x: ArrayLike | str,
        y: ArrayLike | str,
        z: MatrixLike | str,
        *,
        data: DataSource = None,
    ) -> "Plot3D":
        """Add a regular-grid surface where ``z.shape == (len(y), len(x))``."""
        return self._add_grid("surface", x, y, z, data)

    def wireframe(
        self,
        x: ArrayLike | str,
        y: ArrayLike | str,
        z: MatrixLike | str,
        *,
        data: DataSource = None,
    ) -> "Plot3D":
        """Add a regular-grid wireframe where ``z.shape == (len(y), len(x))``."""
        return self._add_grid("wireframe", x, y, z, data)

    def size_px(self, width: int, height: int) -> "Plot3D":
        """Set the exported image dimensions in pixels."""
        normalized_width = _exact_int(width, _SIZE_PX_3D_MESSAGE)
        normalized_height = _exact_int(height, _SIZE_PX_3D_MESSAGE)
        if normalized_width <= 0 or normalized_height <= 0:
            raise ValueError(_SIZE_PX_3D_MESSAGE)
        self._native_plot.size_px(normalized_width, normalized_height)
        self._state["sizePx"] = [normalized_width, normalized_height]
        return self

    def dpi(self, dpi: int) -> "Plot3D":
        """Set output dots per inch while preserving ``size_px`` dimensions."""
        normalized = _exact_int(dpi, _DPI_3D_MESSAGE)
        if normalized <= 0:
            raise ValueError(_DPI_3D_MESSAGE)
        self._native_plot.dpi(normalized)
        self._state["dpi"] = normalized
        return self

    def theme(self, theme: Theme) -> "Plot3D":
        """Use the ``light`` or ``dark`` theme."""
        normalized = str(theme).lower()
        if normalized not in {"light", "dark"}:
            raise ValueError(f"unsupported theme: {theme}")
        self._native_plot.theme(normalized)
        self._state["theme"] = normalized
        return self

    def title(self, title: str) -> "Plot3D":
        """Set the plot title."""
        self._native_plot.title(str(title))
        self._state["title"] = str(title)
        return self

    def xlabel(self, label: str) -> "Plot3D":
        """Set the x-axis label."""
        self._native_plot.xlabel(str(label))
        self._state["xLabel"] = str(label)
        return self

    def ylabel(self, label: str) -> "Plot3D":
        """Set the y-axis label."""
        self._native_plot.ylabel(str(label))
        self._state["yLabel"] = str(label)
        return self

    def zlabel(self, label: str) -> "Plot3D":
        """Set the z-axis label."""
        self._native_plot.zlabel(str(label))
        self._state["zLabel"] = str(label)
        return self

    def _set_limit(self, axis: str, minimum: float, maximum: float) -> "Plot3D":
        lower = float(minimum)
        upper = float(maximum)
        if not np.isfinite(lower) or not np.isfinite(upper) or lower >= upper:
            raise ValueError(f"{axis} limits must be finite and strictly ascending")
        getattr(self._native_plot, f"{axis}lim")(lower, upper)
        self._state[f"{axis}Lim"] = [lower, upper]
        return self

    def xlim(self, minimum: float, maximum: float) -> "Plot3D":
        """Set finite ascending x-axis limits."""
        return self._set_limit("x", minimum, maximum)

    def ylim(self, minimum: float, maximum: float) -> "Plot3D":
        """Set finite ascending y-axis limits."""
        return self._set_limit("y", minimum, maximum)

    def zlim(self, minimum: float, maximum: float) -> "Plot3D":
        """Set finite ascending z-axis limits."""
        return self._set_limit("z", minimum, maximum)

    def azimuth_deg(self, degrees: float) -> "Plot3D":
        """Set camera azimuth in degrees."""
        self._native_plot.azimuth_deg(float(degrees))
        self._state["azimuthDeg"] = float(degrees)
        return self

    def elevation_deg(self, degrees: float) -> "Plot3D":
        """Set camera elevation in degrees."""
        self._native_plot.elevation_deg(float(degrees))
        self._state["elevationDeg"] = float(degrees)
        return self

    def perspective_deg(self, vertical_fov_deg: float = 45.0) -> "Plot3D":
        """Use perspective projection with a vertical field of view in degrees."""
        self._native_plot.perspective_deg(float(vertical_fov_deg))
        self._state["projection"] = "perspective"
        self._state["perspectiveDeg"] = float(vertical_fov_deg)
        return self

    def orthographic(self) -> "Plot3D":
        """Use the default scientific orthographic projection."""
        self._native_plot.orthographic()
        self._state["projection"] = "orthographic"
        self._state.pop("perspectiveDeg", None)
        return self

    def render_png(self) -> bytes:
        """Render the 3D plot to deterministic CPU PNG bytes."""
        return bytes(self._native_plot.render_png_bytes())

    def render_svg(self) -> str:
        """Render hybrid SVG with a depth-tested raster scene and vector labels."""
        return self._native_plot.render_svg()

    def save(self, path: str | Path) -> Path:
        """Save the 3D plot as PNG, hybrid SVG, or hybrid PDF."""
        output = Path(path)
        self._native_plot.save(str(output))
        return output

    def to_snapshot(self) -> Plot3DSnapshot:
        """Return a JSON-friendly static copy of the 3D plot state."""
        return cast(Plot3DSnapshot, _materialize(self._state))

    def _repr_png_(self) -> bytes:
        """Return PNG bytes for notebook rich display."""
        return self.render_png()


def plot() -> Plot:
    """Create a new fluent :class:`Plot` builder."""
    return Plot()


def plot3d() -> Plot3D:
    """Create an empty :class:`Plot3D` alpha builder."""
    return Plot3D()


def scatter3d(
    x: ArrayLike | str,
    y: ArrayLike | str,
    z: ArrayLike | str,
    *,
    data: DataSource = None,
) -> Plot3D:
    """Create a 3D scatter plot."""
    return Plot3D().scatter3d(x, y, z, data=data)


def line3d(
    x: ArrayLike | str,
    y: ArrayLike | str,
    z: ArrayLike | str,
    *,
    data: DataSource = None,
) -> Plot3D:
    """Create a 3D line plot."""
    return Plot3D().line3d(x, y, z, data=data)


def surface(
    x: ArrayLike | str,
    y: ArrayLike | str,
    z: MatrixLike | str,
    *,
    data: DataSource = None,
) -> Plot3D:
    """Create a regular-grid 3D surface."""
    return Plot3D().surface(x, y, z, data=data)


def wireframe(
    x: ArrayLike | str,
    y: ArrayLike | str,
    z: MatrixLike | str,
    *,
    data: DataSource = None,
) -> Plot3D:
    """Create a regular-grid 3D wireframe."""
    return Plot3D().wireframe(x, y, z, data=data)


def observable(values: ArrayLike) -> ObservableSeries:
    """Create an :class:`ObservableSeries` from array-like numeric input."""
    return ObservableSeries(values)
