"""Public Python API for ruviz.

The Python package exposes a fluent :class:`Plot` builder for static export,
static notebook display, explicit Jupyter widgets, and native interactive
display outside notebooks.
"""

from __future__ import annotations

import json
import weakref
from copy import deepcopy
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


class ObservableSeries:
    """Mutable numeric data source for notebook-driven updates."""

    def __init__(self, values: Any) -> None:
        """Create an observable numeric series from array-like values."""
        self._values = _to_numeric_list(values)
        self._native_observable = _native.NativeObservable1D(self._values)
        self._listeners: dict[int, weakref.ReferenceType[Any] | weakref.WeakMethod[Any]] = {}
        self._next_listener_token = 0

    def replace(self, values: Any) -> None:
        """Replace the entire series and notify attached widgets."""
        self._values = _to_numeric_list(values)
        self._native_observable.replace(self._values)
        self._notify()

    def set_at(self, index: int, value: float) -> None:
        """Update a single element in-place and notify attached widgets."""
        if index < 0 or index >= len(self._values):
            raise IndexError("observable index is out of bounds")
        self._values[index] = float(value)
        self._native_observable.set_at(index, float(value))
        self._notify()

    def values(self) -> np.ndarray:
        """Return the current values as a NumPy array."""
        return np.asarray(self._values, dtype=float)

    def snapshot_values(self) -> list[float]:
        """Return the current values as a plain Python list."""
        return list(self._values)

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

    def _rebuild_native_plot(self, snapshot: dict[str, Any]) -> None:
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
            kind = series["kind"]
            if kind == "line":
                native_plot.line(series["x"]["values"], series["y"]["values"])
            elif kind == "scatter":
                native_plot.scatter(series["x"]["values"], series["y"]["values"])
            elif kind == "bar":
                native_plot.bar(series["categories"], series["values"]["values"])
            elif kind == "histogram":
                native_plot.histogram(series["data"]["values"])
            elif kind == "boxplot":
                native_plot.boxplot(series["data"]["values"])
            elif kind == "heatmap":
                native_plot.heatmap(series["values"], int(series["rows"]), int(series["cols"]))
            elif kind == "error-bars":
                native_plot.error_bars(
                    series["x"]["values"], series["y"]["values"], series["yErrors"]["values"]
                )
            elif kind == "error-bars-xy":
                native_plot.error_bars_xy(
                    series["x"]["values"],
                    series["y"]["values"],
                    series["xErrors"]["values"],
                    series["yErrors"]["values"],
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

        self._native_plot = native_plot

    def clone(self) -> "Plot":
        """Return a deep copy of the plot state."""
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
        self._native_plot.line(native_x, native_y)
        if x_observable is not None:
            self._track_observable(x_observable, x_values)
        if y_observable is not None:
            self._track_observable(y_observable, y_values)
        self._state["series"].append({"kind": "line", "x": x_values, "y": y_values})
        self._invalidate_snapshot_cache()
        return self

    def scatter(self, x: Any, y: Any, *, data: Any = None) -> "Plot":
        """Add a scatter series from x/y arrays or dataframe columns."""
        x_values, native_x, x_observable = self._build_native_numeric_source(_column_values(data, x))
        y_values, native_y, y_observable = self._build_native_numeric_source(_column_values(data, y))
        self._ensure_equal_length("scatter", x_values, y_values)
        self._native_plot.scatter(native_x, native_y)
        if x_observable is not None:
            self._track_observable(x_observable, x_values)
        if y_observable is not None:
            self._track_observable(y_observable, y_values)
        self._state["series"].append({"kind": "scatter", "x": x_values, "y": y_values})
        self._invalidate_snapshot_cache()
        return self

    def bar(self, x: Any, y: Any, *, data: Any = None) -> "Plot":
        """Add a categorical bar series."""
        categories = _to_string_list(_column_values(data, x))
        values, native_values, observable = self._build_native_numeric_source(_column_values(data, y))
        if len(categories) != len(values["values"]):
            raise ValueError("bar categories and values must have the same length")
        self._native_plot.bar(categories, native_values)
        if observable is not None:
            self._track_observable(observable, values)
        self._state["series"].append({"kind": "bar", "categories": categories, "values": values})
        self._invalidate_snapshot_cache()
        return self

    def histogram(self, x: Any, *, data: Any = None) -> "Plot":
        """Add a histogram from one numeric sample vector."""
        series_data, native_data, observable = self._build_native_numeric_source(_column_values(data, x))
        self._native_plot.histogram(native_data)
        if observable is not None:
            self._track_observable(observable, series_data)
        self._state["series"].append({"kind": "histogram", "data": series_data})
        self._invalidate_snapshot_cache()
        return self

    def boxplot(self, x: Any, *, data: Any = None) -> "Plot":
        """Add a boxplot from one numeric sample vector."""
        series_data, native_data, observable = self._build_native_numeric_source(_column_values(data, x))
        self._native_plot.boxplot(native_data)
        if observable is not None:
            self._track_observable(observable, series_data)
        self._state["series"].append({"kind": "boxplot", "data": series_data})
        self._invalidate_snapshot_cache()
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
        self._native_plot.heatmap(values, len(rows), cols)
        self._state["series"].append({"kind": "heatmap", "values": values, "rows": len(rows), "cols": cols})
        self._invalidate_snapshot_cache()
        return self

    def error_bars(self, x: Any, y: Any, y_errors: Any, *, data: Any = None) -> "Plot":
        """Add a series with vertical error bars."""
        x_values, native_x, x_observable = self._build_native_numeric_source(_column_values(data, x))
        y_values, native_y, y_observable = self._build_native_numeric_source(_column_values(data, y))
        error_values, native_errors, error_observable = self._build_native_numeric_source(
            _column_values(data, y_errors)
        )
        self._ensure_equal_length("error-bars", x_values, y_values, error_values)
        self._native_plot.error_bars(native_x, native_y, native_errors)
        if x_observable is not None:
            self._track_observable(x_observable, x_values)
        if y_observable is not None:
            self._track_observable(y_observable, y_values)
        if error_observable is not None:
            self._track_observable(error_observable, error_values)
        self._state["series"].append(
            {"kind": "error-bars", "x": x_values, "y": y_values, "yErrors": error_values}
        )
        self._invalidate_snapshot_cache()
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
        self._native_plot.error_bars_xy(native_x, native_y, native_x_errors, native_y_errors)
        if x_observable is not None:
            self._track_observable(x_observable, x_values)
        if y_observable is not None:
            self._track_observable(y_observable, y_values)
        if x_error_observable is not None:
            self._track_observable(x_error_observable, x_error_values)
        if y_error_observable is not None:
            self._track_observable(y_error_observable, y_error_values)
        self._state["series"].append(
            {
                "kind": "error-bars-xy",
                "x": x_values,
                "y": y_values,
                "xErrors": x_error_values,
                "yErrors": y_error_values,
            }
        )
        self._invalidate_snapshot_cache()
        return self

    def kde(self, x: Any, *, data: Any = None) -> "Plot":
        """Add a kernel density estimate for a numeric sample vector."""
        values = _to_numeric_list(_column_values(data, x))
        self._native_plot.kde(values)
        self._state["series"].append({"kind": "kde", "data": values})
        self._invalidate_snapshot_cache()
        return self

    def ecdf(self, x: Any, *, data: Any = None) -> "Plot":
        """Add an empirical cumulative distribution plot."""
        values = _to_numeric_list(_column_values(data, x))
        self._native_plot.ecdf(values)
        self._state["series"].append({"kind": "ecdf", "data": values})
        self._invalidate_snapshot_cache()
        return self

    def contour(self, x: Any, y: Any, z: Any, *, data: Any = None) -> "Plot":
        """Add a contour plot from x/y axes and a flattened z grid."""
        x_values = _to_numeric_list(_column_values(data, x))
        y_values = _to_numeric_list(_column_values(data, y))
        z_values = _to_numeric_list(_column_values(data, z))
        if len(z_values) != len(x_values) * len(y_values):
            raise ValueError("contour z must contain x.length * y.length values")
        self._native_plot.contour(x_values, y_values, z_values)
        self._state["series"].append({"kind": "contour", "x": x_values, "y": y_values, "z": z_values})
        self._invalidate_snapshot_cache()
        return self

    def pie(self, values: Any, labels: Any = None, *, data: Any = None) -> "Plot":
        """Add a pie chart with optional labels."""
        numeric = _to_numeric_list(_column_values(data, values))
        label_values = None if labels is None else _to_string_list(_column_values(data, labels))
        if label_values is not None and len(label_values) != len(numeric):
            raise ValueError("pie values and labels must have the same length")
        self._native_plot.pie(numeric, label_values)
        series = {"kind": "pie", "values": numeric}
        if label_values is not None:
            series["labels"] = label_values
        self._state["series"].append(series)
        self._invalidate_snapshot_cache()
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
        self._native_plot.radar(
            label_values,
            [(item.get("name"), item["values"]) for item in normalized],
        )
        self._state["series"].append({"kind": "radar", "labels": label_values, "series": normalized})
        self._invalidate_snapshot_cache()
        return self

    def violin(self, x: Any, *, data: Any = None) -> "Plot":
        """Add a violin plot from one numeric sample vector."""
        values = _to_numeric_list(_column_values(data, x))
        self._native_plot.violin(values)
        self._state["series"].append({"kind": "violin", "data": values})
        self._invalidate_snapshot_cache()
        return self

    def polar_line(self, r: Any, theta: Any, *, data: Any = None) -> "Plot":
        """Add a polar line from radius and angle vectors."""
        r_values = _to_numeric_list(_column_values(data, r))
        theta_values = _to_numeric_list(_column_values(data, theta))
        if len(r_values) != len(theta_values):
            raise ValueError("polar r and theta must have the same length")
        self._native_plot.polar_line(r_values, theta_values)
        self._state["series"].append({"kind": "polar-line", "r": r_values, "theta": theta_values})
        self._invalidate_snapshot_cache()
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
