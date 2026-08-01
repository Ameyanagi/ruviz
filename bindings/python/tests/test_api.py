from __future__ import annotations

import gc
from functools import lru_cache
import weakref
from copy import copy, deepcopy
from pathlib import Path
from unittest.mock import patch

import numpy as np
import pytest
import ruviz

PNG_HEADER = b"\x89PNG\r\n\x1a\n"


@lru_cache(maxsize=1)
def _large_xy() -> tuple[np.ndarray, np.ndarray]:
    x = np.linspace(0.0, 10.0, num=100_000, dtype=float)
    y = np.sin(x) + 0.2 * np.cos(x * 3.0)
    return x, y


@lru_cache(maxsize=1)
def _large_error_bar_xy() -> tuple[np.ndarray, np.ndarray]:
    x = np.linspace(0.0, 10.0, num=25_000, dtype=float)
    y = np.sin(x) + 0.2 * np.cos(x * 3.0)
    return x, y


@lru_cache(maxsize=1)
def _large_scalars() -> np.ndarray:
    x = np.linspace(0.0, 20.0, num=100_000, dtype=float)
    return np.sin(x * 0.8) + 0.35 * np.cos(x * 1.7)


@lru_cache(maxsize=1)
def _large_heatmap() -> np.ndarray:
    y = np.linspace(-1.0, 1.0, num=320, dtype=float)
    x = np.linspace(-1.0, 1.0, num=320, dtype=float)
    grid_x, grid_y = np.meshgrid(x, y)
    ridge = np.exp(-((grid_x - 0.25) ** 2 + (grid_y + 0.1) ** 2) * 9.0)
    waves = 0.35 * np.sin(grid_x * 8.0) * np.cos(grid_y * 6.0)
    return ridge + waves


@lru_cache(maxsize=1)
def _large_contour() -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    x = np.linspace(-2.0, 2.0, num=320, dtype=float)
    y = np.linspace(-2.0, 2.0, num=320, dtype=float)
    grid_x, grid_y = np.meshgrid(x, y)
    z = (grid_x**2 - grid_y**2) + 0.25 * np.sin(grid_x * 3.0) * np.cos(grid_y * 2.0)
    return x, y, z.reshape(-1)


@lru_cache(maxsize=1)
def _large_categories() -> list[str]:
    return [f"c{index}" for index in range(20_000)]


@lru_cache(maxsize=1)
def _large_bar_values() -> np.ndarray:
    x = np.linspace(0.0, 15.0, num=20_000, dtype=float)
    return 1.0 + 0.45 * np.sin(x) + 0.1 * np.cos(x * 4.0)


def _base_large_plot() -> ruviz.Plot:
    return ruviz.plot().size_px(320, 200).ticks(False)


def _build_large_line_plot() -> ruviz.Plot:
    x, y = _large_xy()
    return _base_large_plot().line(x, y)


def _build_large_scatter_plot() -> ruviz.Plot:
    x, y = _large_xy()
    return _base_large_plot().scatter(x, y)


def _build_large_bar_plot() -> ruviz.Plot:
    return _base_large_plot().bar(_large_categories(), _large_bar_values())


def _build_large_histogram_plot() -> ruviz.Plot:
    return _base_large_plot().histogram(_large_scalars())


def _build_large_boxplot_plot() -> ruviz.Plot:
    return _base_large_plot().boxplot(_large_scalars())


def _build_large_heatmap_plot() -> ruviz.Plot:
    return _base_large_plot().heatmap(_large_heatmap())


def _build_large_error_bars_plot() -> ruviz.Plot:
    x, y = _large_error_bar_xy()
    y_errors = 0.03 + 0.01 * np.abs(np.sin(x * 0.7))
    return _base_large_plot().error_bars(x, y, y_errors)


def _build_large_error_bars_xy_plot() -> ruviz.Plot:
    x, y = _large_error_bar_xy()
    x_errors = 0.02 + 0.008 * np.abs(np.cos(x * 0.9))
    y_errors = 0.03 + 0.01 * np.abs(np.sin(x * 0.7))
    return _base_large_plot().error_bars_xy(x, y, x_errors, y_errors)


def _build_large_kde_plot() -> ruviz.Plot:
    return _base_large_plot().kde(_large_scalars())


def _build_large_ecdf_plot() -> ruviz.Plot:
    return _base_large_plot().ecdf(_large_scalars())


def _build_large_contour_plot() -> ruviz.Plot:
    x, y, z = _large_contour()
    return _base_large_plot().contour(x, y, z)


def _build_large_violin_plot() -> ruviz.Plot:
    return _base_large_plot().violin(_large_scalars())


def _build_large_polar_line_plot() -> ruviz.Plot:
    theta = np.linspace(0.0, np.pi * 20.0, num=100_000, dtype=float)
    r = 1.0 + 0.25 * np.sin(theta * 2.0) + 0.1 * np.cos(theta * 7.0)
    return _base_large_plot().polar_line(r, theta)


LARGE_RASTER_CASES = [
    ("line", _build_large_line_plot),
    ("scatter", _build_large_scatter_plot),
    ("bar", _build_large_bar_plot),
    ("histogram", _build_large_histogram_plot),
    ("boxplot", _build_large_boxplot_plot),
    ("heatmap", _build_large_heatmap_plot),
    ("error-bars", _build_large_error_bars_plot),
    ("error-bars-xy", _build_large_error_bars_xy_plot),
    ("kde", _build_large_kde_plot),
    ("ecdf", _build_large_ecdf_plot),
    ("contour", _build_large_contour_plot),
    ("violin", _build_large_violin_plot),
    ("polar-line", _build_large_polar_line_plot),
]

LARGE_VECTOR_CASES = [
    ("line", _build_large_line_plot),
    ("histogram", _build_large_histogram_plot),
    ("heatmap", _build_large_heatmap_plot),
]

LARGE_WIDGET_CASES = [
    ("line", _build_large_line_plot),
    ("histogram", _build_large_histogram_plot),
    ("heatmap", _build_large_heatmap_plot),
]


def _svg_has_graphics_markup(svg: str) -> bool:
    return any(token in svg for token in ("<path", "<polyline", "<rect", "<circle", "<image"))


def test_render_svg_smoke() -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4]).title("demo")

    svg = plot.render_svg()

    assert svg.startswith("<?xml")
    assert "<svg" in svg


def test_repr_png_smoke() -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4]).title("demo")

    png = plot._repr_png_()

    assert png.startswith(b"\x89PNG\r\n\x1a\n")


def test_empty_plot_render_svg_succeeds() -> None:
    plot = ruviz.plot().title("Empty Plot").xlabel("X").ylabel("Y")

    svg = plot.render_svg()

    assert svg.startswith("<?xml")
    assert "Empty Plot" in svg


def test_empty_plot_render_png_succeeds() -> None:
    plot = ruviz.plot().title("Empty Plot")

    png = plot.render_png()

    assert png.startswith(b"\x89PNG\r\n\x1a\n")


def test_empty_plot_repr_png_succeeds() -> None:
    plot = ruviz.plot().title("Empty Plot")

    png = plot._repr_png_()

    assert png.startswith(PNG_HEADER)


def test_render_png_uses_native_handle_not_snapshot_json() -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4]).title("demo")

    with patch.object(plot, "_snapshot_json", side_effect=AssertionError("snapshot path should not run")):
        png = plot.render_png()

    assert png.startswith(PNG_HEADER)


def test_render_svg_uses_native_handle_not_snapshot_json() -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4]).title("demo")

    with patch.object(plot, "_snapshot_json", side_effect=AssertionError("snapshot path should not run")):
        svg = plot.render_svg()

    assert svg.startswith("<?xml")


def test_observable_render_updates_native_plot_without_snapshot_roundtrip() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().line([0.0, 1.0, 2.0], source)

    first_png = plot.render_png()
    source.replace([3.0, 2.0, 1.0])

    with patch.object(plot, "_snapshot_json", side_effect=AssertionError("snapshot path should not run")):
        second_png = plot.render_png()

    assert first_png != second_png


def test_clone_rebuilds_native_plot_from_mixed_series_shapes() -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4]).kde([1, 2, 2, 3]).title("clone")

    clone = plot.clone()

    assert clone.render_png().startswith(PNG_HEADER)
    assert clone.to_snapshot() == plot.to_snapshot()


@pytest.mark.parametrize(("name", "builder"), LARGE_RASTER_CASES, ids=[name for name, _ in LARGE_RASTER_CASES])
def test_large_plot_public_png_paths(name: str, builder: object, tmp_path: Path) -> None:
    plot = builder()

    png = plot.render_png()
    assert png.startswith(PNG_HEADER)
    assert len(png) > 2_048

    output = plot.save(tmp_path / f"{name}.png")
    saved = output.read_bytes()
    assert saved.startswith(PNG_HEADER)
    assert saved == png


@pytest.mark.parametrize(("name", "builder"), LARGE_VECTOR_CASES, ids=[name for name, _ in LARGE_VECTOR_CASES])
def test_large_plot_public_vector_paths(name: str, builder: object, tmp_path: Path) -> None:
    plot = builder()

    svg = plot.render_svg()
    assert svg.startswith("<?xml")
    assert "<svg" in svg
    assert _svg_has_graphics_markup(svg)

    svg_path = plot.save(tmp_path / f"{name}.svg")
    saved_svg = svg_path.read_text(encoding="utf-8")
    assert saved_svg == svg

    pdf_path = plot.save(tmp_path / f"{name}.pdf")
    assert pdf_path.is_file()
    assert pdf_path.stat().st_size > 1_024


@pytest.mark.parametrize(("name", "builder"), LARGE_WIDGET_CASES, ids=[name for name, _ in LARGE_WIDGET_CASES])
def test_large_plot_widget_snapshot_smoke(name: str, builder: object) -> None:
    plot = builder()

    widget = plot.widget()

    assert len(widget.snapshot["series"]) == 1
    assert widget.snapshot["series"][0]["kind"] == name


@pytest.mark.parametrize(("name", "builder"), LARGE_WIDGET_CASES, ids=[name for name, _ in LARGE_WIDGET_CASES])
def test_large_plot_show_uses_static_image_in_notebooks(name: str, builder: object) -> None:
    plot = builder()

    with (
        patch("ruviz._api._is_notebook", return_value=True),
        patch("IPython.display.display") as display,
    ):
        result = plot.show()

    assert result is None
    display.assert_called_once()
    image = display.call_args.args[0]
    assert image.data.startswith(PNG_HEADER)
    assert len(image.data) > 2_048


def test_clone_keeps_observable_series_static() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().line([0.0, 1.0, 2.0], source)

    clone = plot.clone()
    source.replace([3.0, 2.0, 1.0])

    assert clone.to_snapshot()["series"][0]["y"]["values"] == [1.0, 2.0, 3.0]


def test_plot_copy_is_independent() -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4]).title("base")

    clone = copy(plot)
    clone.xlabel("copy-x")

    assert plot.to_snapshot().get("xLabel") is None
    assert clone.to_snapshot()["xLabel"] == "copy-x"
    assert clone._state is not plot._state
    assert clone._native_plot is not plot._native_plot


def test_plot_deepcopy_preserves_independent_live_observables() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().line([0.0, 1.0, 2.0], source).scatter([0.0, 1.0, 2.0], source)

    clone = deepcopy(plot)

    assert len(clone._observables) == 1
    assert clone.to_snapshot() == plot.to_snapshot()

    source.replace([3.0, 2.0, 1.0])

    assert plot.to_snapshot()["series"][0]["y"]["values"] == [3.0, 2.0, 1.0]
    assert clone.to_snapshot()["series"][0]["y"]["values"] == [1.0, 2.0, 3.0]

    cloned_source = clone._observables[0]
    cloned_source.replace([9.0, 8.0, 7.0])

    clone_snapshot = clone.to_snapshot()
    assert clone_snapshot["series"][0]["y"]["values"] == [9.0, 8.0, 7.0]
    assert clone_snapshot["series"][1]["y"]["values"] == [9.0, 8.0, 7.0]
    assert plot.to_snapshot()["series"][0]["y"]["values"] == [3.0, 2.0, 1.0]


def test_observable_copy_and_deepcopy_are_independent() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])

    shallow = copy(source)
    deep = deepcopy(source)

    shallow.set_at(0, 10.0)
    deep.replace([7.0, 8.0, 9.0])

    assert source.snapshot_values() == [1.0, 2.0, 3.0]
    assert shallow.snapshot_values() == [10.0, 2.0, 3.0]
    assert deep.snapshot_values() == [7.0, 8.0, 9.0]
    assert shallow._native_observable is not source._native_observable
    assert deep._native_observable is not source._native_observable


def test_observable_math_stays_live_for_scalars_pairs_and_ufuncs() -> None:
    left = ruviz.observable([1.0, 2.0, 3.0])
    right = ruviz.observable([0.5, 1.5, 2.5])

    result = np.sin((left * 2.0) + right)
    np.testing.assert_allclose(result.snapshot_values(), np.sin(np.asarray([2.5, 5.5, 8.5])))

    left.replace([2.0, 4.0, 6.0])

    np.testing.assert_allclose(result.snapshot_values(), np.sin(np.asarray([4.5, 9.5, 14.5])))


def test_observable_math_detaches_on_write() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    derived = source * 2.0

    derived.set_at(0, 99.0)
    source.replace([4.0, 5.0, 6.0])

    assert derived.snapshot_values() == [99.0, 4.0, 6.0]


def test_observable_numpy_bridge_supports_snapshot_and_shape_validation() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])

    np.testing.assert_allclose(np.asarray(source), [1.0, 2.0, 3.0])
    np.testing.assert_allclose(np.add(source, 1.0).snapshot_values(), [2.0, 3.0, 4.0])

    with pytest.raises(ValueError, match="same length"):
        _ = source + [1.0]

    with pytest.raises(TypeError, match="keyword arguments"):
        np.add(source, 1.0, out=np.empty(3))


def test_show_uses_static_image_in_notebooks() -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4]).title("demo")

    with (
        patch("ruviz._api._is_notebook", return_value=True),
        patch("IPython.display.display") as display,
    ):
        result = plot.show()

    assert result is None
    display.assert_called_once()
    image = display.call_args.args[0]
    assert image.data.startswith(b"\x89PNG\r\n\x1a\n")
    assert len(plot._widgets) == 0


def test_empty_plot_show_uses_static_image_in_notebooks() -> None:
    plot = ruviz.plot().title("Empty Plot")

    with (
        patch("ruviz._api._is_notebook", return_value=True),
        patch("IPython.display.display") as display,
    ):
        result = plot.show()

    assert result is None
    display.assert_called_once()
    image = display.call_args.args[0]
    assert image.data.startswith(b"\x89PNG\r\n\x1a\n")


def test_show_uses_native_window_outside_notebooks() -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4]).title("demo")

    with (
        patch("ruviz._api._is_notebook", return_value=False),
        patch.object(type(plot._native_plot), "show_native") as show_native,
    ):
        result = plot.show()

    assert result is None
    show_native.assert_called_once()


def test_observable_updates_widget_snapshot() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().line([0.0, 1.0, 2.0], source)
    widget = plot.widget()

    source.replace([4.0, 5.0, 6.0])

    assert widget.snapshot["series"][0]["y"]["values"] == [4.0, 5.0, 6.0]


def test_widget_esm_uses_generated_bundle() -> None:
    expected_path = Path(ruviz.__file__).with_name("widget.js")
    assert expected_path.is_file()
    assert str(ruviz.RuvizWidget._esm) == expected_path.read_text(encoding="utf-8")


def test_observable_detaches_discarded_plot_listeners() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])

    def build_plot() -> weakref.ReferenceType[ruviz.Plot]:
        plot = ruviz.plot().line([0.0, 1.0, 2.0], source)
        return weakref.ref(plot)

    plot_ref = build_plot()

    gc.collect()

    assert plot_ref() is None
    assert source._listeners == {}

    source.replace([4.0, 5.0, 6.0])

    assert source._listeners == {}
