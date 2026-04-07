"""Public Python API for ruviz.

The Python package exposes a fluent :class:`Plot` builder for static export,
static notebook display, explicit Jupyter widgets, and native interactive
display outside notebooks.
"""

from __future__ import annotations

import json
import weakref
from copy import deepcopy
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Any

import numpy as np

from . import _native

if TYPE_CHECKING:
    from ._widget import RuvizWidget


def _is_notebook() -> bool:
    try:
        from IPython import get_ipython
    except ImportError:
        return False

    shell = get_ipython()
    return shell is not None and shell.__class__.__name__ == "ZMQInteractiveShell"


def _is_pandas_dataframe(value: Any) -> bool:
    try:
        import pandas as pd
    except ImportError:
        return False

    return isinstance(value, (pd.DataFrame, pd.Series))


def _is_polars_dataframe(value: Any) -> bool:
    try:
        import polars as pl
    except ImportError:
        return False

    return isinstance(value, (pl.DataFrame, pl.Series))


def _column_values(data: Any, column: Any) -> Any:
    if data is None:
        return column

    if isinstance(column, str):
        if _is_pandas_dataframe(data):
            return data[column]
        if _is_polars_dataframe(data):
            return data[column]
        if isinstance(data, dict):
            return data[column]
        raise TypeError(f"unsupported data source for column lookup: {type(data)!r}")

    return column


def _to_numeric_list(values: Any) -> list[float]:
    if isinstance(values, ObservableSeries):
        return values.snapshot_values()
    if _is_pandas_dataframe(values) or _is_polars_dataframe(values):
        return _to_numeric_list(values.to_list())

    array = np.asarray(values, dtype=float)
    return array.astype(float).reshape(-1).tolist()


def _to_string_list(values: Any) -> list[str]:
    if _is_pandas_dataframe(values) or _is_polars_dataframe(values):
        values = values.to_list()
    return [str(value) for value in values]


def _normalize_observable_math_input(value: Any) -> Any:
    if isinstance(value, ObservableSeries):
        return value
    if _is_pandas_dataframe(value) or _is_polars_dataframe(value):
        value = value.to_list()

    array = np.asarray(value, dtype=float)
    if array.ndim == 0:
        return float(array.item())
    if array.ndim == 1:
        return array.astype(float).tolist()

    raise TypeError(
        "ObservableSeries math only supports real scalars, 1D numeric arrays, and other observables"
    )


@dataclass
class _ObservableDerivation:
    ufunc: np.ufunc
    inputs: tuple[Any, ...]
    bindings: list[tuple["ObservableSeries", int]]


class ObservableSeries:
    """Mutable numeric data source for notebook-driven updates."""

    __array_priority__ = 1000

    def __init__(self, values: Any) -> None:
        """Create an observable numeric series from array-like values."""
        self._initialize(_to_numeric_list(values))

    def _initialize(self, values: list[float]) -> None:
        self._values = list(values)
        self._native_observable = _native.NativeObservable1D(self._values)
        self._listeners: dict[int, weakref.ReferenceType[Any] | weakref.WeakMethod[Any]] = {}
        self._next_listener_token = 0
        self._derivation: _ObservableDerivation | None = None

    @classmethod
    def _from_values(cls, values: list[float]) -> "ObservableSeries":
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
            token = value._attach(observable._refresh_from_derivation)
            bindings.append((value, token))
            weakref.finalize(observable, value._detach, token)

        observable._derivation = _ObservableDerivation(ufunc=ufunc, inputs=normalized_inputs, bindings=bindings)
        return observable

    @staticmethod
    def _input_length(value: Any) -> int | None:
        if isinstance(value, ObservableSeries):
            return len(value._values)
        if isinstance(value, list):
            return len(value)
        return None

    @staticmethod
    def _materialize_input(value: Any) -> float | np.ndarray:
        if isinstance(value, ObservableSeries):
            return np.asarray(value._values, dtype=float)
        if isinstance(value, list):
            return np.asarray(value, dtype=float)
        return float(value)

    @classmethod
    def _evaluate_ufunc(cls, ufunc: np.ufunc, inputs: tuple[Any, ...]) -> list[float]:
        lengths = {length for value in inputs if (length := cls._input_length(value)) is not None}
        if len(lengths) > 1:
            raise ValueError("observable math operands must have the same length")

        try:
            result = ufunc(*[cls._materialize_input(value) for value in inputs])
        except ValueError as err:
            raise ValueError("observable math operands must have the same length") from err
        except TypeError as err:
            raise TypeError("unsupported observable math operation") from err

        array = np.asarray(result, dtype=float)
        if array.ndim != 1:
            raise TypeError("observable math must produce a 1D numeric result")
        return array.astype(float).tolist()

    def _detach_derivation(self) -> None:
        if self._derivation is None:
            return

        for source, token in self._derivation.bindings:
            source._detach(token)
        self._derivation = None

    def _refresh_from_derivation(self) -> None:
        if self._derivation is None:
            return

        self._values = self._evaluate_ufunc(self._derivation.ufunc, self._derivation.inputs)
        self._native_observable.replace(self._values)
        self._notify()

    def _ensure_detached(self) -> None:
        if self._derivation is not None:
            self._detach_derivation()

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

    def replace(self, values: Any) -> None:
        """Replace the entire series and notify attached widgets."""
        next_values = _to_numeric_list(values)
        self._ensure_detached()
        self._values = next_values
        self._native_observable.replace(self._values)
        self._notify()

    def set_at(self, index: int, value: float) -> None:
        """Update a single element in-place and notify attached widgets."""
        if index < 0 or index >= len(self._values):
            raise IndexError("observable index is out of bounds")
        normalized_value = float(value)
        self._ensure_detached()
        self._values[index] = normalized_value
        self._native_observable.set_at(index, normalized_value)
        self._notify()

    def values(self) -> np.ndarray:
        """Return the current values as a NumPy array."""
        return np.asarray(self._values, dtype=float)

    def snapshot_values(self) -> list[float]:
        """Return the current values as a plain Python list."""
        return list(self._values)

    def __array__(self, dtype: Any = None) -> np.ndarray:
        array = np.asarray(self._values, dtype=float)
        if dtype is not None:
            array = array.astype(dtype)
        return array

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

    def __add__(self, other: Any) -> "ObservableSeries":
        return type(self)._from_ufunc(np.add, self, other)

    def __radd__(self, other: Any) -> "ObservableSeries":
        return type(self)._from_ufunc(np.add, other, self)

    def __sub__(self, other: Any) -> "ObservableSeries":
        return type(self)._from_ufunc(np.subtract, self, other)

    def __rsub__(self, other: Any) -> "ObservableSeries":
        return type(self)._from_ufunc(np.subtract, other, self)

    def __mul__(self, other: Any) -> "ObservableSeries":
        return type(self)._from_ufunc(np.multiply, self, other)

    def __rmul__(self, other: Any) -> "ObservableSeries":
        return type(self)._from_ufunc(np.multiply, other, self)

    def __truediv__(self, other: Any) -> "ObservableSeries":
        return type(self)._from_ufunc(np.true_divide, self, other)

    def __rtruediv__(self, other: Any) -> "ObservableSeries":
        return type(self)._from_ufunc(np.true_divide, other, self)

    def __floordiv__(self, other: Any) -> "ObservableSeries":
        return type(self)._from_ufunc(np.floor_divide, self, other)

    def __rfloordiv__(self, other: Any) -> "ObservableSeries":
        return type(self)._from_ufunc(np.floor_divide, other, self)

    def __pow__(self, other: Any) -> "ObservableSeries":
        return type(self)._from_ufunc(np.power, self, other)

    def __rpow__(self, other: Any) -> "ObservableSeries":
        return type(self)._from_ufunc(np.power, other, self)

    def _snapshot(self) -> dict[str, Any]:
        return {"kind": "observable", "values": self.snapshot_values()}

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


class Plot:
    """Fluent plot builder for static and interactive ruviz rendering."""

    def __init__(self) -> None:
        self._state: dict[str, Any] = {"series": []}
        self._native_plot = _native.NativePlotHandle()
        self._widgets: "weakref.WeakSet[Any]" = weakref.WeakSet()
        self._observables: list[ObservableSeries] = []
        self._observable_listener_tokens: dict[ObservableSeries, int] = {}
        self._observable_bindings: list[tuple[ObservableSeries, dict[str, Any]]] = []
        self._snapshot_cache: dict[str, Any] | None = None
        self._snapshot_dirty = True

    def _invalidate_snapshot_cache(self) -> None:
        self._snapshot_dirty = True
        self._snapshot_cache = None

    def _build_native_numeric_source(
        self, value: Any
    ) -> tuple[dict[str, Any], list[float] | Any, ObservableSeries | None]:
        if isinstance(value, ObservableSeries):
            snapshot = value._snapshot()
            return snapshot, value._native_observable, value

        values = _to_numeric_list(value)
        return {"kind": "static", "values": values}, values, None

    @staticmethod
    def _apply_native_series(
        native_plot: Any,
        series: dict[str, Any],
        *,
        native_sources: dict[str, Any] | None = None,
    ) -> None:
        native_sources = native_sources or {}
        kind = series["kind"]
        if kind == "line":
            native_plot.line(
                native_sources.get("x", series["x"]["values"]),
                native_sources.get("y", series["y"]["values"]),
            )
        elif kind == "scatter":
            native_plot.scatter(
                native_sources.get("x", series["x"]["values"]),
                native_sources.get("y", series["y"]["values"]),
            )
        elif kind == "bar":
            native_plot.bar(
                series["categories"],
                native_sources.get("values", series["values"]["values"]),
            )
        elif kind == "histogram":
            native_plot.histogram(native_sources.get("data", series["data"]["values"]))
        elif kind == "boxplot":
            native_plot.boxplot(native_sources.get("data", series["data"]["values"]))
        elif kind == "heatmap":
            native_plot.heatmap(series["values"], int(series["rows"]), int(series["cols"]))
        elif kind == "error-bars":
            native_plot.error_bars(
                native_sources.get("x", series["x"]["values"]),
                native_sources.get("y", series["y"]["values"]),
                native_sources.get("yErrors", series["yErrors"]["values"]),
            )
        elif kind == "error-bars-xy":
            native_plot.error_bars_xy(
                native_sources.get("x", series["x"]["values"]),
                native_sources.get("y", series["y"]["values"]),
                native_sources.get("xErrors", series["xErrors"]["values"]),
                native_sources.get("yErrors", series["yErrors"]["values"]),
            )
        elif kind == "kde":
            native_plot.kde(series["data"])
        elif kind == "ecdf":
            native_plot.ecdf(series["data"])
        elif kind == "contour":
            native_plot.contour(series["x"], series["y"], series["z"])
        elif kind == "pie":
            native_plot.pie(series["values"], series.get("labels"))
        elif kind == "radar":
            native_plot.radar(
                series["labels"],
                [(item.get("name"), item["values"]) for item in series["series"]],
            )
        elif kind == "violin":
            native_plot.violin(series["data"])
        elif kind == "polar-line":
            native_plot.polar_line(series["r"], series["theta"])
        else:
            raise ValueError(f"unsupported plot snapshot kind: {kind}")

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

        size = snapshot.get("sizePx")
        if size is not None:
            native_plot.size_px(int(size[0]), int(size[1]))
        theme = snapshot.get("theme")
        if theme is not None:
            native_plot.theme(str(theme))
        ticks = snapshot.get("ticks")
        if ticks is not None:
            native_plot.ticks(bool(ticks))
        title = snapshot.get("title")
        if title is not None:
            native_plot.title(str(title))
        x_label = snapshot.get("xLabel")
        if x_label is not None:
            native_plot.xlabel(str(x_label))
        y_label = snapshot.get("yLabel")
        if y_label is not None:
            native_plot.ylabel(str(y_label))

        for series in snapshot["series"]:
            self._apply_native_series(native_plot, series)

        self._native_plot = native_plot

    @staticmethod
    def _apply_snapshot_metadata(plot: "Plot", snapshot: dict[str, Any]) -> None:
        size = snapshot.get("sizePx")
        if size is not None:
            plot.size_px(int(size[0]), int(size[1]))
        theme = snapshot.get("theme")
        if theme is not None:
            plot.theme(str(theme))
        ticks = snapshot.get("ticks")
        if ticks is not None:
            plot.ticks(bool(ticks))
        title = snapshot.get("title")
        if title is not None:
            plot.title(str(title))
        x_label = snapshot.get("xLabel")
        if x_label is not None:
            plot.xlabel(str(x_label))
        y_label = snapshot.get("yLabel")
        if y_label is not None:
            plot.ylabel(str(y_label))

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
            kind = series["kind"]
            if kind == "line":
                plot.line(
                    cls._resolve_numeric_source(series["x"], observable_lookup),
                    cls._resolve_numeric_source(series["y"], observable_lookup),
                )
            elif kind == "scatter":
                plot.scatter(
                    cls._resolve_numeric_source(series["x"], observable_lookup),
                    cls._resolve_numeric_source(series["y"], observable_lookup),
                )
            elif kind == "bar":
                plot.bar(series["categories"], cls._resolve_numeric_source(series["values"], observable_lookup))
            elif kind == "histogram":
                plot.histogram(cls._resolve_numeric_source(series["data"], observable_lookup))
            elif kind == "boxplot":
                plot.boxplot(cls._resolve_numeric_source(series["data"], observable_lookup))
            elif kind == "heatmap":
                cols = int(series["cols"])
                rows = [
                    series["values"][row_start : row_start + cols]
                    for row_start in range(0, len(series["values"]), cols)
                ]
                plot.heatmap(rows)
            elif kind == "error-bars":
                plot.error_bars(
                    cls._resolve_numeric_source(series["x"], observable_lookup),
                    cls._resolve_numeric_source(series["y"], observable_lookup),
                    cls._resolve_numeric_source(series["yErrors"], observable_lookup),
                )
            elif kind == "error-bars-xy":
                plot.error_bars_xy(
                    cls._resolve_numeric_source(series["x"], observable_lookup),
                    cls._resolve_numeric_source(series["y"], observable_lookup),
                    cls._resolve_numeric_source(series["xErrors"], observable_lookup),
                    cls._resolve_numeric_source(series["yErrors"], observable_lookup),
                )
            elif kind == "kde":
                plot.kde(series["data"])
            elif kind == "ecdf":
                plot.ecdf(series["data"])
            elif kind == "contour":
                plot.contour(series["x"], series["y"], series["z"])
            elif kind == "pie":
                plot.pie(series["values"], series.get("labels"))
            elif kind == "radar":
                plot.radar(series["labels"], series["series"])
            elif kind == "violin":
                plot.violin(series["data"])
            elif kind == "polar-line":
                plot.polar_line(series["r"], series["theta"])
            else:
                raise ValueError(f"unsupported plot snapshot kind: {kind}")

        return plot

    def __copy__(self) -> "Plot":
        return self.__deepcopy__({})

    def __deepcopy__(self, memo: dict[int, Any]) -> "Plot":
        existing = memo.get(id(self))
        if existing is not None:
            return existing

        observable_lookup = {
            id(snapshot): deepcopy(observable, memo)
            for observable, snapshot in self._observable_bindings
        }
        clone = type(self)._replay_snapshot(self._state, observable_lookup)
        memo[id(self)] = clone
        return clone

    def clone(self) -> "Plot":
        """Return a static snapshot copy of the current plot.

        Observable-backed series are copied by value, so the clone renders the
        same current data but does not stay linked to later observable updates.
        """
        clone = Plot()
        clone._state = self.to_snapshot()
        clone._rebuild_native_plot(clone._state)
        clone._snapshot_cache = deepcopy(clone._state)
        clone._snapshot_dirty = False
        return clone

    def size_px(self, width: int, height: int) -> "Plot":
        """Set the pixel size used for export and notebook rendering."""
        normalized_width = max(1, int(width))
        normalized_height = max(1, int(height))
        self._native_plot.size_px(normalized_width, normalized_height)
        self._state["sizePx"] = [normalized_width, normalized_height]
        self._invalidate_snapshot_cache()
        return self

    def theme(self, theme: str) -> "Plot":
        """Set the built-in light or dark theme."""
        if theme not in {"light", "dark"}:
            raise ValueError("theme must be 'light' or 'dark'")
        self._native_plot.theme(theme)
        self._state["theme"] = theme
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

    def line(self, x: Any, y: Any, *, data: Any = None) -> "Plot":
        """Add a line series from x/y arrays or dataframe columns."""
        x_values, native_x, x_observable = self._build_native_numeric_source(_column_values(data, x))
        y_values, native_y, y_observable = self._build_native_numeric_source(_column_values(data, y))
        self._ensure_equal_length("line", x_values, y_values)
        series = {"kind": "line", "x": x_values, "y": y_values}
        self._apply_native_series(self._native_plot, series, native_sources={"x": native_x, "y": native_y})
        if x_observable is not None:
            self._track_observable(x_observable, x_values)
        if y_observable is not None:
            self._track_observable(y_observable, y_values)
        self._append_series_snapshot(series)
        return self

    def scatter(self, x: Any, y: Any, *, data: Any = None) -> "Plot":
        """Add a scatter series from x/y arrays or dataframe columns."""
        x_values, native_x, x_observable = self._build_native_numeric_source(_column_values(data, x))
        y_values, native_y, y_observable = self._build_native_numeric_source(_column_values(data, y))
        self._ensure_equal_length("scatter", x_values, y_values)
        series = {"kind": "scatter", "x": x_values, "y": y_values}
        self._apply_native_series(self._native_plot, series, native_sources={"x": native_x, "y": native_y})
        if x_observable is not None:
            self._track_observable(x_observable, x_values)
        if y_observable is not None:
            self._track_observable(y_observable, y_values)
        self._append_series_snapshot(series)
        return self

    def bar(self, x: Any, y: Any, *, data: Any = None) -> "Plot":
        """Add a categorical bar series."""
        categories = _to_string_list(_column_values(data, x))
        values, native_values, observable = self._build_native_numeric_source(_column_values(data, y))
        if len(categories) != len(values["values"]):
            raise ValueError("bar categories and values must have the same length")
        series = {"kind": "bar", "categories": categories, "values": values}
        self._apply_native_series(self._native_plot, series, native_sources={"values": native_values})
        if observable is not None:
            self._track_observable(observable, values)
        self._append_series_snapshot(series)
        return self

    def histogram(self, x: Any, *, data: Any = None) -> "Plot":
        """Add a histogram from one numeric sample vector."""
        series_data, native_data, observable = self._build_native_numeric_source(_column_values(data, x))
        series = {"kind": "histogram", "data": series_data}
        self._apply_native_series(self._native_plot, series, native_sources={"data": native_data})
        if observable is not None:
            self._track_observable(observable, series_data)
        self._append_series_snapshot(series)
        return self

    def boxplot(self, x: Any, *, data: Any = None) -> "Plot":
        """Add a boxplot from one numeric sample vector."""
        series_data, native_data, observable = self._build_native_numeric_source(_column_values(data, x))
        series = {"kind": "boxplot", "data": series_data}
        self._apply_native_series(self._native_plot, series, native_sources={"data": native_data})
        if observable is not None:
            self._track_observable(observable, series_data)
        self._append_series_snapshot(series)
        return self

    def heatmap(self, values: Any) -> "Plot":
        """Add a heatmap from a rectangular numeric matrix."""
        rows = [_to_numeric_list(row) for row in values]
        if not rows or not rows[0]:
            raise ValueError("heatmap input must be a non-empty 2D numeric matrix")
        cols = len(rows[0])
        if any(len(row) != cols for row in rows):
            raise ValueError("heatmap rows must all have the same length")
        values = [value for row in rows for value in row]
        series = {"kind": "heatmap", "values": values, "rows": len(rows), "cols": cols}
        self._apply_native_series(self._native_plot, series)
        self._append_series_snapshot(series)
        return self

    def error_bars(self, x: Any, y: Any, y_errors: Any, *, data: Any = None) -> "Plot":
        """Add a series with vertical error bars."""
        x_values, native_x, x_observable = self._build_native_numeric_source(_column_values(data, x))
        y_values, native_y, y_observable = self._build_native_numeric_source(_column_values(data, y))
        error_values, native_errors, error_observable = self._build_native_numeric_source(
            _column_values(data, y_errors)
        )
        self._ensure_equal_length("error-bars", x_values, y_values, error_values)
        series = {"kind": "error-bars", "x": x_values, "y": y_values, "yErrors": error_values}
        self._apply_native_series(
            self._native_plot,
            series,
            native_sources={"x": native_x, "y": native_y, "yErrors": native_errors},
        )
        if x_observable is not None:
            self._track_observable(x_observable, x_values)
        if y_observable is not None:
            self._track_observable(y_observable, y_values)
        if error_observable is not None:
            self._track_observable(error_observable, error_values)
        self._append_series_snapshot(series)
        return self

    def error_bars_xy(
        self,
        x: Any,
        y: Any,
        x_errors: Any,
        y_errors: Any,
        *,
        data: Any = None,
    ) -> "Plot":
        """Add a series with both horizontal and vertical error bars."""
        x_values, native_x, x_observable = self._build_native_numeric_source(_column_values(data, x))
        y_values, native_y, y_observable = self._build_native_numeric_source(_column_values(data, y))
        x_error_values, native_x_errors, x_error_observable = self._build_native_numeric_source(
            _column_values(data, x_errors)
        )
        y_error_values, native_y_errors, y_error_observable = self._build_native_numeric_source(
            _column_values(data, y_errors)
        )
        self._ensure_equal_length("error-bars-xy", x_values, y_values, x_error_values, y_error_values)
        series = {
            "kind": "error-bars-xy",
            "x": x_values,
            "y": y_values,
            "xErrors": x_error_values,
            "yErrors": y_error_values,
        }
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
            self._track_observable(x_observable, x_values)
        if y_observable is not None:
            self._track_observable(y_observable, y_values)
        if x_error_observable is not None:
            self._track_observable(x_error_observable, x_error_values)
        if y_error_observable is not None:
            self._track_observable(y_error_observable, y_error_values)
        self._append_series_snapshot(series)
        return self

    def kde(self, x: Any, *, data: Any = None) -> "Plot":
        """Add a kernel density estimate for a numeric sample vector."""
        values = _to_numeric_list(_column_values(data, x))
        series = {"kind": "kde", "data": values}
        self._apply_native_series(self._native_plot, series)
        self._append_series_snapshot(series)
        return self

    def ecdf(self, x: Any, *, data: Any = None) -> "Plot":
        """Add an empirical cumulative distribution plot."""
        values = _to_numeric_list(_column_values(data, x))
        series = {"kind": "ecdf", "data": values}
        self._apply_native_series(self._native_plot, series)
        self._append_series_snapshot(series)
        return self

    def contour(self, x: Any, y: Any, z: Any, *, data: Any = None) -> "Plot":
        """Add a contour plot from x/y axes and a flattened z grid."""
        x_values = _to_numeric_list(_column_values(data, x))
        y_values = _to_numeric_list(_column_values(data, y))
        z_values = _to_numeric_list(_column_values(data, z))
        if len(z_values) != len(x_values) * len(y_values):
            raise ValueError("contour z must contain x.length * y.length values")
        series = {"kind": "contour", "x": x_values, "y": y_values, "z": z_values}
        self._apply_native_series(self._native_plot, series)
        self._append_series_snapshot(series)
        return self

    def pie(self, values: Any, labels: Any = None, *, data: Any = None) -> "Plot":
        """Add a pie chart with optional labels."""
        numeric = _to_numeric_list(_column_values(data, values))
        label_values = None if labels is None else _to_string_list(_column_values(data, labels))
        if label_values is not None and len(label_values) != len(numeric):
            raise ValueError("pie values and labels must have the same length")
        series = {"kind": "pie", "values": numeric}
        if label_values is not None:
            series["labels"] = label_values
        self._apply_native_series(self._native_plot, series)
        self._append_series_snapshot(series)
        return self

    def radar(self, labels: Any, series: list[dict[str, Any]]) -> "Plot":
        """Add a radar chart from axis labels and named series."""
        label_values = _to_string_list(labels)
        normalized = []
        for item in series:
            values = _to_numeric_list(item["values"])
            if len(values) != len(label_values):
                raise ValueError("each radar series must match the labels length")
            normalized.append({"name": item.get("name"), "values": values})
        plot_series = {"kind": "radar", "labels": label_values, "series": normalized}
        self._apply_native_series(self._native_plot, plot_series)
        self._append_series_snapshot(plot_series)
        return self

    def violin(self, x: Any, *, data: Any = None) -> "Plot":
        """Add a violin plot from one numeric sample vector."""
        values = _to_numeric_list(_column_values(data, x))
        series = {"kind": "violin", "data": values}
        self._apply_native_series(self._native_plot, series)
        self._append_series_snapshot(series)
        return self

    def polar_line(self, r: Any, theta: Any, *, data: Any = None) -> "Plot":
        """Add a polar line from radius and angle vectors."""
        r_values = _to_numeric_list(_column_values(data, r))
        theta_values = _to_numeric_list(_column_values(data, theta))
        if len(r_values) != len(theta_values):
            raise ValueError("polar r and theta must have the same length")
        series = {"kind": "polar-line", "r": r_values, "theta": theta_values}
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
        """Create an explicit synced Jupyter widget for this plot."""
        from ._widget import RuvizWidget

        widget = RuvizWidget(self)
        self._widgets.add(widget)
        return widget

    def _notebook_image(self) -> Any:
        from IPython.display import Image

        return Image(data=self.render_png(), format="png")

    def show(self) -> Any:
        """Display a static image in Jupyter or open a native interactive window."""
        if _is_notebook():
            from IPython.display import display

            image = self._notebook_image()
            display(image)
            return None

        self._native_plot.show_native()
        return None

    def to_snapshot(self) -> dict[str, Any]:
        """Serialize the current plot state to a JSON-friendly snapshot."""
        self._sync_observables()
        if self._snapshot_dirty or self._snapshot_cache is None:
            self._snapshot_cache = deepcopy(self._state)
            self._snapshot_dirty = False
        return deepcopy(self._snapshot_cache)

    def _snapshot_json(self) -> str:
        return json.dumps(self.to_snapshot())

    def _track_observable(self, observable: ObservableSeries, snapshot: dict[str, Any]) -> None:
        self._observable_bindings.append((observable, snapshot))
        if observable in self._observables:
            return
        self._observables.append(observable)
        token = observable._attach(self._notify_widgets)
        self._observable_listener_tokens[observable] = token
        weakref.finalize(self, observable._detach, token)

    def _sync_observables(self) -> None:
        for observable, snapshot in self._observable_bindings:
            snapshot["values"] = observable.snapshot_values()

    def _notify_widgets(self) -> None:
        self._invalidate_snapshot_cache()
        for widget in list(self._widgets):
            widget.refresh()

    def _ensure_equal_length(self, name: str, *sources: dict[str, Any]) -> None:
        lengths = [len(source["values"]) for source in sources]
        if len(set(lengths)) != 1:
            raise ValueError(f"{name} inputs must have the same length")

    def _repr_png_(self) -> bytes:
        """Return PNG bytes for notebook rich display."""
        return self.render_png()


def plot() -> Plot:
    """Create a new fluent :class:`Plot` builder."""
    return Plot()


def observable(values: Any) -> ObservableSeries:
    """Create an :class:`ObservableSeries` from array-like numeric input."""
    return ObservableSeries(values)
