"""Downstream-consumer typing check for the public ``ruviz`` API.

This module is never imported or executed: pytest does not collect it (the name
does not match ``test_*``), and CI checks it with ``uv run pyright``. It is the
regression test for the inline types, so it exercises the public surface the way
a typed downstream package would.

Negative cases carry ``# pyright: ignore[<rule>]`` comments. With
``reportUnnecessaryTypeIgnoreComment = "error"`` in ``[tool.pyright]``, a case
that stops being an error fails the check, so the ignores double as assertions
that the annotation still rejects the bad call.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np

import ruviz
from ruviz import (
    ArrayLike,
    DataSource,
    LineStyleName,
    ObservableSeries,
    Plot,
    Plot3D,
    Plot3DSnapshot,
    PlotSnapshot,
    RadarSeriesDict,
    RuvizWidget,
    SeriesSnapshot,
    StyleDict,
    Theme,
)


def fluent_chain_returns_a_plot() -> Plot:
    """A styling chain keeps returning ``Plot``, and rendering returns bytes."""
    x = np.linspace(0.0, 10.0, 64)
    y = np.sin(x)
    chart = (
        ruviz.plot()
        .size_px(800, 600)
        .theme("dark")
        .title("Signal")
        .xlabel("t")
        .ylabel("amplitude")
        .grid(True)
        .ticks(True)
        .xlim(0.0, 10.0)
        .ylim(-1.5, 1.5)
        .xscale("symlog", 0.5)
        .yscale("linear")
        .legend("upper_right")
        .dpi(150)
        .line(x, y, label="sin", color="#2563eb", width=2.0, linestyle="dashed", marker="circle")
        .scatter([0.0, 1.0], [1.0, 0.0], marker="triangle-down", marker_size=8.0, alpha=0.5)
        # The matplotlib shorthands are aliases for the canonical names.
        .line(x, y, linestyle="--", marker="o")
        .scatter(x, y, marker="D")
    )
    png: bytes = chart.render_png()
    svg: str = chart.render_svg()
    assert png and svg
    return chart


def every_series_kind_accepts_sequences_and_arrays() -> Plot:
    """Lists, NumPy arrays, and ``data=`` column names are all array inputs."""
    frame: dict[str, list[float]] = {"x": [0.0, 1.0, 2.0], "y": [1.0, 2.0, 3.0]}
    samples = np.random.default_rng(0).normal(size=128)
    return (
        ruviz.plot()
        .line("x", "y", data=frame)
        .bar(["a", "b"], np.array([1.0, 2.0]), color="red")
        .histogram(samples, bins=12)
        .boxplot(samples, linestyle="dotted")
        .violin(samples.tolist())
        .kde(samples, bandwidth=0.4)
        .ecdf(samples)
        .heatmap([[0.0, 1.0], [1.0, 0.0]])
        .error_bars([0.0, 1.0], [1.0, 2.0], [0.1, 0.2], width=1.5)
        .error_bars_xy([0.0, 1.0], [1.0, 2.0], [0.1, 0.2], [0.1, 0.2])
        .contour([0.0, 1.0], [0.0, 1.0], [0.0, 1.0, 1.0, 0.0], levels=4)
        .pie([1.0, 2.0], ["a", "b"])
        .polar_line([1.0, 2.0], [0.0, 3.14])
    )


def radar_takes_typed_series() -> Plot:
    """Radar series are ``RadarSeriesDict`` mappings with optional names."""
    series: list[RadarSeriesDict] = [
        {"name": "python", "values": [4.5, 4.7, 4.8]},
        {"values": np.array([4.2, 4.1, 4.0])},
    ]
    return ruviz.plot().radar(["api", "docs", "export"], series)


def observables_are_array_inputs() -> ObservableSeries:
    """Observables are accepted wherever an array is, and support NumPy math."""
    source = ruviz.observable([0.0, 1.0, 2.0])
    doubled: ObservableSeries = source * 2.0
    # NumPy types its ufuncs as returning arrays, so the ``__array_ufunc__``
    # result needs the ignore even though it is an ObservableSeries at runtime.
    shifted: ObservableSeries = np.add(doubled, source)  # pyright: ignore[reportAssignmentType]
    ruviz.plot().line(source, shifted).scatter(source, doubled)
    source.replace(np.array([3.0, 4.0, 5.0]))
    source.set_at(0, 9.0)
    values: np.ndarray = source.values()
    snapshot: list[float] = source.snapshot_values()
    first: float = source[0]
    assert values.size == len(snapshot) == len(source) and first == first
    return source


def snapshots_are_typed_mappings(chart: Plot) -> tuple[int, list[SeriesSnapshot]]:
    """``to_snapshot`` returns a ``PlotSnapshot`` with known keys."""
    snapshot: PlotSnapshot = chart.to_snapshot()
    version: int = snapshot["schemaVersion"]
    series: list[SeriesSnapshot] = snapshot["series"]
    title: str = snapshot.get("title", "")
    theme: Theme = snapshot.get("theme", "light")
    style: StyleDict = series[0].get("style", {})
    label: str = style.get("label", "")
    assert title == title and theme in {"light", "dark"} and label == label
    return version, series


def plot3d_surface_takes_a_matrix() -> Plot3DSnapshot:
    """3D grid builders take a matrix ``z`` while point builders take vectors."""
    grid = np.zeros((4, 3))
    axis_x = [0.0, 1.0, 2.0]
    axis_y = [0.0, 1.0, 2.0, 3.0]
    surface: Plot3D = ruviz.surface(axis_x, axis_y, grid).title("surface").theme("light")
    ruviz.wireframe(axis_x, axis_y, grid.tolist())
    ruviz.line3d([0.0, 1.0], [0.0, 1.0], [0.0, 1.0])
    scatter = (
        ruviz.plot3d()
        .scatter3d([0.0, 1.0], [0.0, 1.0], [0.0, 1.0])
        .size_px(640, 480)
        .dpi(120)
        .xlabel("x")
        .ylabel("y")
        .zlabel("z")
        .xlim(0.0, 1.0)
        .ylim(0.0, 1.0)
        .zlim(0.0, 1.0)
        .azimuth_deg(45.0)
        .elevation_deg(20.0)
        .perspective_deg()
        .orthographic()
    )
    assert scatter.render_png()
    return surface.to_snapshot()


def exports_and_widgets(chart: Plot) -> RuvizWidget:
    """``save`` returns the written path and ``widget`` a synced widget."""
    written: Path = chart.save("chart.png")
    assert written.suffix == ".png"
    chart.show()
    return chart.widget()


def helpers_stay_generic(values: ArrayLike, data: DataSource, style: LineStyleName) -> Plot:
    """The exported aliases are usable in downstream signatures."""
    return ruviz.plot().line(values, values, data=data, linestyle=style)


def rejected_calls() -> None:
    """Each ignored line must stay an error; see the module docstring."""
    chart = ruviz.plot()
    chart.theme("solarized")  # pyright: ignore[reportArgumentType]
    chart.legend("top_left")  # pyright: ignore[reportArgumentType]
    chart.xscale("logarithmic")  # pyright: ignore[reportArgumentType]
    chart.line([0.0], [1.0], linestyle="wavy")  # pyright: ignore[reportArgumentType]
    chart.line([0.0], [1.0], marker="blob")  # pyright: ignore[reportArgumentType]
    chart.scatter([0.0], [1.0], marker_size="big")  # pyright: ignore[reportArgumentType]
    chart.line([0.0], [1.0], data=[1, 2])  # pyright: ignore[reportArgumentType]
    chart.histogram([0.0], bins="many")  # pyright: ignore[reportArgumentType]
    chart.radar(["a"], [{"name": "x"}])  # pyright: ignore[reportArgumentType]
    chart.size_px(800)  # pyright: ignore[reportCallIssue]
    chart.line([0.0])  # pyright: ignore[reportCallIssue]
    chart.to_snapshot()["nope"]  # pyright: ignore[reportGeneralTypeIssues]
    ruviz.plot3d().surface([0.0], [0.0], [0.0], extra=1)  # pyright: ignore[reportCallIssue]
