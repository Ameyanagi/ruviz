"""Public Python API for ruviz.

The Python package exposes a fluent :class:`Plot` builder for static export,
Jupyter widgets, and native interactive display outside notebooks.
"""

from __future__ import annotations

import json
import weakref
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
        self._listeners: dict[int, weakref.ReferenceType[Any] | weakref.WeakMethod[Any]] = {}
        self._next_listener_token = 0

    def replace(self, values: Any) -> None:
        """Replace the entire series and notify attached widgets."""
        self._values = _to_numeric_list(values)
        self._notify()

    def set_at(self, index: int, value: float) -> None:
        """Update a single element in-place and notify attached widgets."""
        if index < 0 or index >= len(self._values):
            raise IndexError("observable index is out of bounds")
        self._values[index] = float(value)
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
        self._widgets: "weakref.WeakSet[Any]" = weakref.WeakSet()
        self._observables: list[ObservableSeries] = []
        self._observable_listener_tokens: dict[ObservableSeries, int] = {}
        self._observable_bindings: list[tuple[ObservableSeries, dict[str, Any]]] = []

    def clone(self) -> "Plot":
        """Return a deep copy of the plot state."""
        clone = Plot()
        clone._state = json.loads(json.dumps(self.to_snapshot()))
        return clone

    def size_px(self, width: int, height: int) -> "Plot":
        """Set the pixel size used for export and notebook rendering."""
        self._state["sizePx"] = [max(1, int(width)), max(1, int(height))]
        return self

    def theme(self, theme: str) -> "Plot":
        """Set the built-in light or dark theme."""
        if theme not in {"light", "dark"}:
            raise ValueError("theme must be 'light' or 'dark'")
        self._state["theme"] = theme
        return self

    def ticks(self, enabled: bool) -> "Plot":
        """Enable or disable axis ticks."""
        self._state["ticks"] = bool(enabled)
        return self

    def title(self, title: str) -> "Plot":
        """Set the plot title."""
        self._state["title"] = str(title)
        return self

    def xlabel(self, label: str) -> "Plot":
        """Set the x-axis label."""
        self._state["xLabel"] = str(label)
        return self

    def ylabel(self, label: str) -> "Plot":
        """Set the y-axis label."""
        self._state["yLabel"] = str(label)
        return self

    def line(self, x: Any, y: Any, *, data: Any = None) -> "Plot":
        """Add a line series from x/y arrays or dataframe columns."""
        x_values = self._numeric_source(_column_values(data, x))
        y_values = self._numeric_source(_column_values(data, y))
        self._ensure_equal_length("line", x_values, y_values)
        self._state["series"].append({"kind": "line", "x": x_values, "y": y_values})
        return self

    def scatter(self, x: Any, y: Any, *, data: Any = None) -> "Plot":
        """Add a scatter series from x/y arrays or dataframe columns."""
        x_values = self._numeric_source(_column_values(data, x))
        y_values = self._numeric_source(_column_values(data, y))
        self._ensure_equal_length("scatter", x_values, y_values)
        self._state["series"].append({"kind": "scatter", "x": x_values, "y": y_values})
        return self

    def bar(self, x: Any, y: Any, *, data: Any = None) -> "Plot":
        """Add a categorical bar series."""
        categories = _to_string_list(_column_values(data, x))
        values = self._reactive_numeric_source(_column_values(data, y))
        if len(categories) != len(values["values"]):
            raise ValueError("bar categories and values must have the same length")
        self._state["series"].append({"kind": "bar", "categories": categories, "values": values})
        return self

    def histogram(self, x: Any, *, data: Any = None) -> "Plot":
        """Add a histogram from one numeric sample vector."""
        self._state["series"].append(
            {"kind": "histogram", "data": self._reactive_numeric_source(_column_values(data, x))}
        )
        return self

    def boxplot(self, x: Any, *, data: Any = None) -> "Plot":
        """Add a boxplot from one numeric sample vector."""
        self._state["series"].append(
            {"kind": "boxplot", "data": self._reactive_numeric_source(_column_values(data, x))}
        )
        return self

    def heatmap(self, values: Any) -> "Plot":
        """Add a heatmap from a rectangular numeric matrix."""
        rows = [_to_numeric_list(row) for row in values]
        if not rows or not rows[0]:
            raise ValueError("heatmap input must be a non-empty 2D numeric matrix")
        cols = len(rows[0])
        if any(len(row) != cols for row in rows):
            raise ValueError("heatmap rows must all have the same length")
        self._state["series"].append(
            {"kind": "heatmap", "values": [value for row in rows for value in row], "rows": len(rows), "cols": cols}
        )
        return self

    def error_bars(self, x: Any, y: Any, y_errors: Any, *, data: Any = None) -> "Plot":
        """Add a series with vertical error bars."""
        x_values = self._reactive_numeric_source(_column_values(data, x))
        y_values = self._reactive_numeric_source(_column_values(data, y))
        error_values = self._reactive_numeric_source(_column_values(data, y_errors))
        self._ensure_equal_length("error-bars", x_values, y_values, error_values)
        self._state["series"].append(
            {"kind": "error-bars", "x": x_values, "y": y_values, "yErrors": error_values}
        )
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
        x_values = self._reactive_numeric_source(_column_values(data, x))
        y_values = self._reactive_numeric_source(_column_values(data, y))
        x_error_values = self._reactive_numeric_source(_column_values(data, x_errors))
        y_error_values = self._reactive_numeric_source(_column_values(data, y_errors))
        self._ensure_equal_length("error-bars-xy", x_values, y_values, x_error_values, y_error_values)
        self._state["series"].append(
            {
                "kind": "error-bars-xy",
                "x": x_values,
                "y": y_values,
                "xErrors": x_error_values,
                "yErrors": y_error_values,
            }
        )
        return self

    def kde(self, x: Any, *, data: Any = None) -> "Plot":
        """Add a kernel density estimate for a numeric sample vector."""
        self._state["series"].append({"kind": "kde", "data": _to_numeric_list(_column_values(data, x))})
        return self

    def ecdf(self, x: Any, *, data: Any = None) -> "Plot":
        """Add an empirical cumulative distribution plot."""
        self._state["series"].append({"kind": "ecdf", "data": _to_numeric_list(_column_values(data, x))})
        return self

    def contour(self, x: Any, y: Any, z: Any, *, data: Any = None) -> "Plot":
        """Add a contour plot from x/y axes and a flattened z grid."""
        x_values = _to_numeric_list(_column_values(data, x))
        y_values = _to_numeric_list(_column_values(data, y))
        z_values = _to_numeric_list(_column_values(data, z))
        if len(z_values) != len(x_values) * len(y_values):
            raise ValueError("contour z must contain x.length * y.length values")
        self._state["series"].append({"kind": "contour", "x": x_values, "y": y_values, "z": z_values})
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
        self._state["series"].append(series)
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
        self._state["series"].append({"kind": "radar", "labels": label_values, "series": normalized})
        return self

    def violin(self, x: Any, *, data: Any = None) -> "Plot":
        """Add a violin plot from one numeric sample vector."""
        self._state["series"].append({"kind": "violin", "data": _to_numeric_list(_column_values(data, x))})
        return self

    def polar_line(self, r: Any, theta: Any, *, data: Any = None) -> "Plot":
        """Add a polar line from radius and angle vectors."""
        r_values = _to_numeric_list(_column_values(data, r))
        theta_values = _to_numeric_list(_column_values(data, theta))
        if len(r_values) != len(theta_values):
            raise ValueError("polar r and theta must have the same length")
        self._state["series"].append({"kind": "polar-line", "r": r_values, "theta": theta_values})
        return self

    def render_png(self) -> bytes:
        """Render the current plot to PNG bytes."""
        return bytes(_native.render_png_bytes(self._snapshot_json()))

    def render_svg(self) -> str:
        """Render the current plot to an SVG document string."""
        return _native.render_svg(self._snapshot_json())

    def save(self, path: str | Path) -> Path:
        """Save the current plot to a PNG, SVG, or PDF file."""
        output = Path(path)
        _native.save(self._snapshot_json(), str(output))
        return output

    def widget(self) -> "RuvizWidget":
        """Create a synced Jupyter widget for this plot."""
        from ._widget import RuvizWidget

        widget = RuvizWidget(self)
        self._widgets.add(widget)
        return widget

    def show(self) -> Any:
        """Display the plot in Jupyter or open a native interactive window."""
        if _is_notebook():
            widget = self.widget()
            try:
                from IPython.display import display

                display(widget)
            except ImportError:
                pass
            return widget

        _native.show_native(self._snapshot_json())
        return None

    def to_snapshot(self) -> dict[str, Any]:
        """Serialize the current plot state to a JSON-friendly snapshot."""
        self._sync_observables()
        return json.loads(json.dumps(self._state))

    def _snapshot_json(self) -> str:
        return json.dumps(self.to_snapshot())

    def _reactive_numeric_source(self, value: Any) -> dict[str, Any]:
        if isinstance(value, ObservableSeries):
            snapshot = value._snapshot()
            self._track_observable(value, snapshot)
            return snapshot
        return {"kind": "static", "values": _to_numeric_list(value)}

    def _numeric_source(self, value: Any) -> dict[str, Any]:
        if isinstance(value, ObservableSeries):
            snapshot = value._snapshot()
            self._track_observable(value, snapshot)
            return snapshot
        return {"kind": "static", "values": _to_numeric_list(value)}

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
        self._sync_observables()
        for widget in list(self._widgets):
            widget.refresh()

    def _ensure_equal_length(self, name: str, *sources: dict[str, Any]) -> None:
        lengths = [len(source["values"]) for source in sources]
        if len(set(lengths)) != 1:
            raise ValueError(f"{name} inputs must have the same length")

    def _ipython_display_(self) -> None:
        self.show()


def plot() -> Plot:
    """Create a new fluent :class:`Plot` builder."""
    return Plot()


def observable(values: Any) -> ObservableSeries:
    """Create an :class:`ObservableSeries` from array-like numeric input."""
    return ObservableSeries(values)
