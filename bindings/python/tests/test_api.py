from __future__ import annotations

import asyncio
import gc
import threading
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


def test_render_png_delegates_to_the_native_handle() -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4]).title("demo")

    with patch.object(
        type(plot._native_plot), "render_png_bytes", return_value=b"native-png"
    ) as render:
        assert plot.render_png() == b"native-png"

    render.assert_called_once()


def test_render_svg_delegates_to_the_native_handle() -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4]).title("demo")

    with patch.object(
        type(plot._native_plot), "render_svg", return_value="<?xml native-svg"
    ) as render:
        assert plot.render_svg() == "<?xml native-svg"

    render.assert_called_once()


def test_observable_render_updates_native_plot() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().line([0.0, 1.0, 2.0], source)

    first_png = plot.render_png()
    source.replace([3.0, 2.0, 1.0])
    second_png = plot.render_png()

    assert first_png != second_png


def test_snapshot_carries_schema_version_through_copies_and_replay() -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4]).title("versioned")

    assert plot.to_snapshot()["schemaVersion"] == 1
    assert plot.clone().to_snapshot()["schemaVersion"] == 1
    assert copy(plot).to_snapshot()["schemaVersion"] == 1
    assert deepcopy(plot).to_snapshot()["schemaVersion"] == 1


def test_replay_tolerates_and_reemits_schema_version() -> None:
    snapshot = ruviz.plot().line([0, 1, 2], [0, 1, 4]).to_snapshot()

    replayed = ruviz.Plot._replay_snapshot(snapshot)

    assert replayed.to_snapshot() == snapshot


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


def test_widget_refresh_stays_synchronous_without_an_event_loop() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().line([0.0, 1.0, 2.0], source)
    widget = plot.widget()

    with patch.object(type(widget), "refresh") as refresh:
        for index in range(5):
            source.set_at(0, float(index))

        assert refresh.call_count == 5

    source.set_at(0, 9.0)

    assert widget.snapshot["series"][0]["y"]["values"] == [9.0, 2.0, 3.0]


def test_widget_refresh_is_coalesced_under_a_running_event_loop() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().line([0.0, 1.0, 2.0], source)
    widget = plot.widget()

    async def burst() -> int:
        with patch.object(type(widget), "refresh") as refresh:
            for index in range(20):
                source.set_at(0, float(index))
            assert refresh.call_count == 0

            await asyncio.sleep(0)
            return int(refresh.call_count)

    calls = asyncio.run(burst())

    assert calls == 1
    assert widget.snapshot["series"][0]["y"]["values"] == [1.0, 2.0, 3.0]

    asyncio.run(_refresh_once(source))

    assert widget.snapshot["series"][0]["y"]["values"] == [7.0, 2.0, 3.0]


async def _refresh_once(source: ruviz.ObservableSeries) -> None:
    source.set_at(0, 7.0)
    await asyncio.sleep(0)


def test_widget_esm_uses_generated_bundle() -> None:
    expected_path = Path(ruviz.__file__).with_name("widget.js")
    assert expected_path.is_file()
    assert str(ruviz.RuvizWidget._esm) == expected_path.read_text(encoding="utf-8")


MATRIX = [[1.0, 2.0], [3.0, 4.0]]

NON_1D_CASES = [
    ("line x", lambda values: ruviz.plot().line(values, [0.0, 1.0])),
    ("line y", lambda values: ruviz.plot().line([0.0, 1.0], values)),
    ("scatter x", lambda values: ruviz.plot().scatter(values, [0.0, 1.0])),
    ("bar values", lambda values: ruviz.plot().bar(["a", "b"], values)),
    ("histogram x", lambda values: ruviz.plot().histogram(values)),
    ("boxplot x", lambda values: ruviz.plot().boxplot(values)),
    ("error_bars x", lambda values: ruviz.plot().error_bars(values, [0.0, 1.0], [0.1, 0.1])),
    (
        "error_bars_xy y_errors",
        lambda values: ruviz.plot().error_bars_xy([0.0, 1.0], [0.0, 1.0], [0.1, 0.1], values),
    ),
    ("kde x", lambda values: ruviz.plot().kde(values)),
    ("ecdf x", lambda values: ruviz.plot().ecdf(values)),
    ("contour z", lambda values: ruviz.plot().contour([0.0, 1.0], [0.0, 1.0], values)),
    ("pie values", lambda values: ruviz.plot().pie(values)),
    ("violin x", lambda values: ruviz.plot().violin(values)),
    ("polar_line r", lambda values: ruviz.plot().polar_line(values, [0.0, 1.0])),
    (
        "radar series values",
        lambda values: ruviz.plot().radar(["a", "b"], [{"name": "s", "values": values}]),
    ),
    ("observable values", lambda values: ruviz.observable(values)),
]

NON_TRACKING_OBSERVABLE_CASES = [
    ("kde", lambda source: ruviz.plot().kde(source)),
    ("ecdf", lambda source: ruviz.plot().ecdf(source)),
    ("contour", lambda source: ruviz.plot().contour([0.0, 1.0], [0.0, 1.0], source)),
    ("pie", lambda source: ruviz.plot().pie(source)),
    ("violin", lambda source: ruviz.plot().violin(source)),
    ("polar_line", lambda source: ruviz.plot().polar_line(source, [0.0, 1.0, 2.0, 3.0])),
    ("heatmap", lambda source: ruviz.plot().heatmap(source)),
    (
        "radar",
        lambda source: ruviz.plot().radar(["a", "b", "c", "d"], [{"name": "s", "values": source}]),
    ),
]


@pytest.mark.parametrize(("name", "builder"), NON_1D_CASES, ids=[name for name, _ in NON_1D_CASES])
def test_numeric_inputs_reject_non_1d_values(name: str, builder: object) -> None:
    with pytest.raises(TypeError, match=f"{name} must be a 1D numeric array"):
        builder(MATRIX)

    with pytest.raises(TypeError, match=f"{name} must be a 1D numeric array"):
        builder(np.asarray(MATRIX))


def test_heatmap_still_accepts_a_2d_matrix() -> None:
    snapshot = ruviz.plot().heatmap(MATRIX).to_snapshot()["series"][0]

    assert snapshot["rows"] == 2
    assert snapshot["cols"] == 2
    assert snapshot["values"] == [1.0, 2.0, 3.0, 4.0]


def test_heatmap_accepts_a_data_keyword() -> None:
    snapshot = ruviz.plot().heatmap("grid", data={"grid": MATRIX}).to_snapshot()["series"][0]

    assert snapshot["values"] == [1.0, 2.0, 3.0, 4.0]


@pytest.mark.parametrize(
    ("kind", "builder"),
    NON_TRACKING_OBSERVABLE_CASES,
    ids=[kind for kind, _ in NON_TRACKING_OBSERVABLE_CASES],
)
def test_non_tracking_kinds_reject_observables(kind: str, builder: object) -> None:
    source = ruviz.observable([1.0, 2.0, 3.0, 4.0])

    with pytest.raises(TypeError, match=f"{kind} does not support ObservableSeries"):
        builder(source)


def test_tracking_kinds_still_accept_observables() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().line([0.0, 1.0, 2.0], source)

    assert plot.to_snapshot()["series"][0]["y"]["kind"] == "observable"


@pytest.mark.parametrize("extension", ["png", "svg", "pdf", "PNG"])
def test_save_accepts_supported_extensions(extension: str, tmp_path: Path) -> None:
    plot = ruviz.plot().size_px(200, 150).line([0, 1, 2], [0, 1, 4])

    output = plot.save(tmp_path / f"out.{extension}")

    assert output.is_file()


def test_save_rejects_unknown_extension(tmp_path: Path) -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4])

    with pytest.raises(ValueError, match=r"unsupported save extension '\.jpg'"):
        plot.save(tmp_path / "out.jpg")


def test_save_rejects_path_without_extension(tmp_path: Path) -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4])

    with pytest.raises(ValueError, match="has no extension"):
        plot.save(tmp_path / "out")


@pytest.mark.parametrize(("width", "height"), [(0, 100), (100, 0), (-1, 100), (100, -1)])
def test_size_px_rejects_non_positive_dimensions(width: int, height: int) -> None:
    with pytest.raises(ValueError, match="greater than zero"):
        ruviz.plot().size_px(width, height)


def test_theme_normalizes_case_and_rejects_unknown_themes() -> None:
    assert ruviz.plot().theme("Dark").to_snapshot()["theme"] == "dark"
    assert ruviz.plot().theme("LIGHT").to_snapshot()["theme"] == "light"

    with pytest.raises(ValueError, match="unsupported theme: solarized"):
        ruviz.plot().theme("solarized")


def test_pandas_series_are_direct_inputs() -> None:
    pd = pytest.importorskip("pandas")
    frame = pd.DataFrame({"time": [0.0, 1.0, 2.0], "value": [1.0, 2.0, 3.0]})

    plot = ruviz.plot().line(frame["time"], frame["value"])

    assert plot.to_snapshot()["series"][0]["x"]["values"] == [0.0, 1.0, 2.0]


def test_polars_series_are_direct_inputs() -> None:
    pl = pytest.importorskip("polars")
    frame = pl.DataFrame({"time": [0.0, 1.0, 2.0], "value": [1.0, 2.0, 3.0]})

    plot = ruviz.plot().line(frame["time"], frame["value"])

    assert plot.to_snapshot()["series"][0]["y"]["values"] == [1.0, 2.0, 3.0]


def test_dataframe_column_lookup_works_for_pandas_polars_and_dicts() -> None:
    pd = pytest.importorskip("pandas")
    pl = pytest.importorskip("polars")
    columns = {"time": [0.0, 1.0, 2.0], "value": [1.0, 2.0, 3.0]}

    for data in (pd.DataFrame(columns), pl.DataFrame(columns), columns):
        plot = ruviz.plot().line("time", "value", data=data)
        assert plot.to_snapshot()["series"][0]["y"]["values"] == [1.0, 2.0, 3.0]


def test_dataframe_as_a_direct_numeric_input_is_rejected() -> None:
    pd = pytest.importorskip("pandas")
    frame = pd.DataFrame({"time": [0.0, 1.0, 2.0], "value": [1.0, 2.0, 3.0]})

    with pytest.raises(TypeError, match="select a column or pass data="):
        ruviz.plot().line(frame, frame)


def test_series_passed_as_data_is_rejected() -> None:
    pd = pytest.importorskip("pandas")
    frame = pd.DataFrame({"time": [0.0, 1.0, 2.0], "value": [1.0, 2.0, 3.0]})

    with pytest.raises(TypeError, match="data= expects a DataFrame or dict"):
        ruviz.plot().line("time", "value", data=frame["time"])


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


STYLE_X = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0]
STYLE_Y = [0.5, 1.5, 1.0, 2.5, 2.0, 3.0]
STYLE_SAMPLES = [-2.3, -1.9, -1.1, -0.4, 0.2, 0.8, 1.0, 1.4, 1.7, 2.1, 2.5, 2.9]
STYLE_ERRORS = [0.2, 0.3, 0.1, 0.25, 0.15, 0.2]
STYLE_GRID = [-1.0, 0.0, 1.0]
STYLE_Z = [0.1, 0.2, 0.3, 0.2, 0.6, 0.2, 0.3, 0.2, 0.1]

#: One builder per styled plot kind, so the style cases below stay derived from
#: the single ``_SERIES_KINDS`` definition rather than restating it.
STYLED_BUILDERS = {
    "line": lambda plot, **style: plot.line(STYLE_X, STYLE_Y, **style),
    "scatter": lambda plot, **style: plot.scatter(STYLE_X, STYLE_Y, **style),
    "bar": lambda plot, **style: plot.bar(["a", "b", "c"], [1.0, 2.0, 3.0], **style),
    "histogram": lambda plot, **style: plot.histogram(STYLE_SAMPLES, **style),
    "boxplot": lambda plot, **style: plot.boxplot(STYLE_SAMPLES, **style),
    "error-bars": lambda plot, **style: plot.error_bars(STYLE_X, STYLE_Y, STYLE_ERRORS, **style),
    "error-bars-xy": lambda plot, **style: plot.error_bars_xy(
        STYLE_X, STYLE_Y, STYLE_ERRORS, STYLE_ERRORS, **style
    ),
    "kde": lambda plot, **style: plot.kde(STYLE_SAMPLES, **style),
    "ecdf": lambda plot, **style: plot.ecdf(STYLE_SAMPLES, **style),
    "contour": lambda plot, **style: plot.contour(STYLE_GRID, STYLE_GRID, STYLE_Z, **style),
    "violin": lambda plot, **style: plot.violin(STYLE_SAMPLES, **style),
    "polar-line": lambda plot, **style: plot.polar_line(
        [1.0, 2.0, 1.5, 2.5], [0.0, 1.0, 2.0, 3.0], **style
    ),
}

STYLE_VALUES = {
    "label": "Series",
    "color": "#2563eb",
    "alpha": 0.35,
    "width": 4.0,
    "linestyle": "dashed",
    "marker": "square",
    "marker_size": 12.0,
    "bins": 7,
    "bandwidth": 0.9,
    "levels": 9,
}

STYLE_CASES = [
    (kind, ruviz._api._STYLE_KEYWORDS.get(key, key))
    for kind, builder in STYLED_BUILDERS.items()
    for key in sorted(ruviz._api._SERIES_KINDS[kind].style)
]


def _styled_base() -> ruviz.Plot:
    return ruviz.plot().size_px(320, 200).legend()


@pytest.mark.parametrize(("kind", "keyword"), STYLE_CASES, ids=[f"{k}-{o}" for k, o in STYLE_CASES])
def test_every_series_style_keyword_changes_the_render(kind: str, keyword: str) -> None:
    build = STYLED_BUILDERS[kind]

    plain = build(_styled_base()).render_png()
    styled = build(_styled_base(), **{keyword: STYLE_VALUES[keyword]}).render_png()

    assert styled.startswith(PNG_HEADER)
    assert styled != plain


@pytest.mark.parametrize(("kind", "keyword"), STYLE_CASES, ids=[f"{k}-{o}" for k, o in STYLE_CASES])
def test_series_style_round_trips_through_snapshot_copies(kind: str, keyword: str) -> None:
    plot = STYLED_BUILDERS[kind](_styled_base(), **{keyword: STYLE_VALUES[keyword]})

    snapshot = plot.to_snapshot()

    assert len(snapshot["series"][0]["style"]) == 1
    assert plot.clone().to_snapshot() == snapshot
    assert copy(plot).to_snapshot() == snapshot
    assert deepcopy(plot).to_snapshot() == snapshot
    assert ruviz.Plot._replay_snapshot(snapshot).to_snapshot() == snapshot


@pytest.mark.parametrize("kind", sorted(STYLED_BUILDERS), ids=sorted(STYLED_BUILDERS))
def test_unstyled_series_stay_style_free(kind: str) -> None:
    plot = STYLED_BUILDERS[kind](_styled_base())

    series = plot.to_snapshot()["series"][0]

    assert "style" not in series
    assert plot.render_png() == STYLED_BUILDERS[kind](_styled_base()).render_png()


def test_series_style_accepts_named_colors_and_underscore_enum_names() -> None:
    plot = ruviz.plot().line(STYLE_X, STYLE_Y, color="Red", marker="triangle_down")

    assert plot.to_snapshot()["series"][0]["style"] == {
        "color": "red",
        "marker": "triangle-down",
    }
    assert plot.render_png().startswith(PNG_HEADER)


@pytest.mark.parametrize(
    ("style", "message"),
    [
        ({"color": "not-a-color"}, "unsupported color"),
        ({"color": 12}, "color must be a string"),
        ({"marker": "blob"}, "unsupported marker"),
        ({"linestyle": "wavy"}, "unsupported linestyle"),
        ({"alpha": 1.5}, "alpha must be between"),
        ({"width": 0.0}, "width must be a finite positive number"),
        ({"marker_size": -1.0}, "marker_size must be a finite positive number"),
        ({"label": 7}, "label must be a string"),
    ],
)
def test_invalid_series_style_values_are_rejected_at_the_call(
    style: dict[str, object], message: str
) -> None:
    with pytest.raises((ValueError, TypeError), match=message):
        ruviz.plot().line(STYLE_X, STYLE_Y, **style)


def test_unsupported_style_keyword_is_rejected_per_kind() -> None:
    with pytest.raises(TypeError, match="unexpected keyword argument 'linestyle'"):
        ruviz.plot().bar(["a"], [1.0], linestyle="dashed")


@pytest.mark.parametrize(
    ("keyword", "value", "message"),
    [
        ("bins", 0, "bins must be an integer >= 1"),
        ("bandwidth", 0.0, "bandwidth must be a finite positive number"),
        ("levels", 1, "levels must be an integer >= 2"),
    ],
)
def test_invalid_series_config_values_are_rejected(
    keyword: str, value: object, message: str
) -> None:
    builders = {
        "bins": lambda **style: ruviz.plot().histogram(STYLE_SAMPLES, **style),
        "bandwidth": lambda **style: ruviz.plot().kde(STYLE_SAMPLES, **style),
        "levels": lambda **style: ruviz.plot().contour(STYLE_GRID, STYLE_GRID, STYLE_Z, **style),
    }

    with pytest.raises(ValueError, match=message):
        builders[keyword](**{keyword: value})


PLOT_SETTING_CASES = [
    ("dpi", lambda plot: plot.dpi(200), "dpi", 200),
    ("legend", lambda plot: plot.legend("upper_left"), "legend", "upper_left"),
    ("grid", lambda plot: plot.grid(False), "grid", False),
    ("xlim", lambda plot: plot.xlim(0.0, 2000.0), "xLim", [0.0, 2000.0]),
    ("ylim", lambda plot: plot.ylim(-5.0, 10.0), "yLim", [-5.0, 10.0]),
    ("xscale", lambda plot: plot.xscale("log"), "xScale", ["log"]),
    ("yscale", lambda plot: plot.yscale("symlog", 2.0), "yScale", ["symlog", 2.0]),
]


def _plot_setting_base() -> ruviz.Plot:
    return (
        ruviz.plot()
        .size_px(320, 200)
        .line([1.0, 10.0, 100.0, 1000.0], [1.0, 2.0, 3.0, 4.0], label="L")
    )


@pytest.mark.parametrize(
    ("name", "apply", "key", "stored"),
    PLOT_SETTING_CASES,
    ids=[case[0] for case in PLOT_SETTING_CASES],
)
def test_plot_level_setting_renders_and_round_trips(
    name: str, apply: object, key: str, stored: object
) -> None:
    plot = apply(_plot_setting_base())

    snapshot = plot.to_snapshot()
    assert snapshot[key] == stored
    assert plot.render_png() != _plot_setting_base().render_png()
    assert plot.clone().to_snapshot() == snapshot
    assert deepcopy(plot).to_snapshot() == snapshot
    assert ruviz.Plot._replay_snapshot(snapshot).to_snapshot() == snapshot


def test_plot_level_settings_default_to_absent() -> None:
    snapshot = _plot_setting_base().to_snapshot()

    assert not {"dpi", "legend", "grid", "xLim", "yLim", "xScale", "yScale"} & set(snapshot)


@pytest.mark.parametrize(
    ("apply", "message"),
    [
        (lambda plot: plot.dpi(0), "plot dpi must be greater than zero"),
        (lambda plot: plot.dpi(0.5), "plot dpi must be greater than zero"),
        (lambda plot: plot.legend("nowhere"), "unsupported legend position"),
        (lambda plot: plot.xscale("logarithmic"), "unsupported axis scale"),
        (lambda plot: plot.xscale("symlog", 0.0), "linthresh must be a finite positive number"),
        (lambda plot: plot.xscale("log", 2.0), "linthresh only applies to the symlog scale"),
        (lambda plot: plot.xlim(1.0, 1.0), "x limits must be finite and strictly ascending"),
        (
            lambda plot: plot.ylim(float("inf"), 1.0),
            "y limits must be finite and strictly ascending",
        ),
    ],
)
def test_invalid_plot_level_settings_are_rejected(apply: object, message: str) -> None:
    with pytest.raises(ValueError, match=message):
        apply(ruviz.plot())


def test_styling_and_axis_settings_compose_in_one_snapshot() -> None:
    plot = (
        ruviz.plot()
        .size_px(320, 200)
        .line(STYLE_X, STYLE_Y, label="Revenue", color="#2563eb", width=2.0)
        .scatter(STYLE_X, STYLE_Y, label="Samples", marker="diamond", marker_size=8.0)
        .legend("upper_right")
        .grid(True)
        .ylim(0.0, 4.0)
    )

    snapshot = plot.to_snapshot()

    assert snapshot["series"][0]["style"] == {"label": "Revenue", "color": "#2563eb", "width": 2.0}
    assert snapshot["series"][1]["style"] == {
        "label": "Samples",
        "marker": "diamond",
        "markerSize": 8.0,
    }
    assert snapshot["legend"] == "upper_right"
    assert plot.render_png().startswith(PNG_HEADER)
    assert ruviz.Plot._replay_snapshot(snapshot).to_snapshot() == snapshot


def test_version_matches_the_compiled_extension() -> None:
    assert ruviz.__version__ == ruviz._native.version()


def test_package_ships_inline_typing_markers() -> None:
    package = Path(ruviz.__file__).parent

    assert (package / "py.typed").is_file()
    assert (package / "_native.pyi").is_file()


def test_observable_resize_bound_to_line_raises_and_leaves_state_intact() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().line([0.0, 1.0, 2.0], source)

    with pytest.raises(ValueError, match="cannot resize observable to 4 values"):
        source.replace([1.0, 2.0, 3.0, 4.0])

    assert source.snapshot_values() == [1.0, 2.0, 3.0]
    assert plot.to_snapshot()["series"][0]["y"]["values"] == [1.0, 2.0, 3.0]
    assert plot.render_png().startswith(PNG_HEADER)


def test_observable_resize_bound_to_bar_categories_raises() -> None:
    source = ruviz.observable([1.0, 2.0])
    plot = ruviz.plot().bar(["a", "b"], source)

    with pytest.raises(ValueError, match="bar categories have length 2"):
        source.replace([1.0, 2.0, 3.0])

    assert plot.render_png().startswith(PNG_HEADER)


def test_observable_resize_bound_to_error_bars_raises() -> None:
    errors = ruviz.observable([0.1, 0.2, 0.3])
    plot = ruviz.plot().error_bars([0.0, 1.0, 2.0], [1.0, 2.0, 3.0], errors)

    with pytest.raises(ValueError, match="input 'x' has length 3"):
        errors.replace([0.1, 0.2])

    assert plot.render_png().startswith(PNG_HEADER)


def test_observable_resize_bound_to_histogram_is_allowed() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().histogram(source)

    source.replace([1.0, 2.0, 3.0, 4.0, 5.0])

    assert plot.to_snapshot()["series"][0]["data"]["values"] == [1.0, 2.0, 3.0, 4.0, 5.0]
    assert plot.render_png().startswith(PNG_HEADER)


def test_same_observable_as_x_and_y_can_resize_together() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().line(source, source)

    source.replace([1.0, 2.0, 3.0, 4.0])

    snapshot = plot.to_snapshot()["series"][0]
    assert snapshot["x"]["values"] == [1.0, 2.0, 3.0, 4.0]
    assert snapshot["y"]["values"] == [1.0, 2.0, 3.0, 4.0]
    assert plot.render_png().startswith(PNG_HEADER)


def test_derived_observable_resize_is_guarded_and_atomic() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    derived = source * 2.0
    plot = ruviz.plot().line([0.0, 1.0, 2.0], derived)

    with pytest.raises(ValueError, match="cannot resize observable to 4 values"):
        source.replace([1.0, 2.0, 3.0, 4.0])

    assert source.snapshot_values() == [1.0, 2.0, 3.0]
    assert derived.snapshot_values() == [2.0, 4.0, 6.0]
    assert plot.render_png().startswith(PNG_HEADER)


def test_derived_chain_resize_is_guarded_and_atomic() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    halfway = source * 2.0
    derived = halfway + 1.0
    plot = ruviz.plot().line([0.0, 1.0, 2.0], derived)

    with pytest.raises(ValueError, match="cannot resize observable to 2 values"):
        source.replace([1.0, 2.0])

    assert source.snapshot_values() == [1.0, 2.0, 3.0]
    assert halfway.snapshot_values() == [2.0, 4.0, 6.0]
    assert derived.snapshot_values() == [3.0, 5.0, 7.0]
    assert plot.render_png().startswith(PNG_HEADER)


def test_resize_with_mismatched_derivation_operands_is_atomic() -> None:
    left = ruviz.observable([1.0, 2.0, 3.0])
    right = ruviz.observable([1.0, 2.0, 3.0])
    combined = left + right

    with pytest.raises(ValueError, match="observable math operands must have the same length"):
        left.replace([1.0, 2.0])

    assert left.snapshot_values() == [1.0, 2.0, 3.0]
    assert combined.snapshot_values() == [2.0, 4.0, 6.0]

    left.replace([4.0, 5.0, 6.0])
    assert combined.snapshot_values() == [5.0, 7.0, 9.0]


def test_replacing_a_derived_observable_lifts_its_source_constraint() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    derived = source * 2.0
    derived.replace([9.0, 9.0, 9.0])

    source.replace([1.0, 2.0])

    assert source.snapshot_values() == [1.0, 2.0]
    assert derived.snapshot_values() == [9.0, 9.0, 9.0]


def test_dropped_plot_no_longer_constrains_observable_resize() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    ruviz.plot().line([0.0, 1.0, 2.0], source)
    gc.collect()

    source.replace([1.0, 2.0, 3.0, 4.0])

    assert source.snapshot_values() == [1.0, 2.0, 3.0, 4.0]


def test_unbound_observable_can_resize_freely() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])

    source.replace([1.0, 2.0])

    assert source.snapshot_values() == [1.0, 2.0]


@pytest.mark.parametrize(
    ("method", "args", "style", "message"),
    [
        ("line", ([0.0, 1.0], [0.0, 1.0]), {"alpha": 1.5}, "alpha must be between 0.0 and 1.0"),
        ("line", ([0.0, 1.0], [0.0, 1.0]), {"width": 0.0}, "width must be a finite positive number"),
        (
            "scatter",
            ([0.0, 1.0], [0.0, 1.0]),
            {"markerSize": -1.0},
            "marker_size must be a finite positive number",
        ),
        ("histogram", ([0.0, 1.0, 2.0],), {"bins": 0}, "bins must be an integer >= 1"),
        (
            "kde",
            ([0.0, 1.0, 2.0],),
            {"bandwidth": 0.0},
            "bandwidth must be a finite positive number",
        ),
        (
            "contour",
            ([0.0, 1.0], [0.0, 1.0], [0.0, 1.0, 2.0, 3.0]),
            {"levels": 1},
            "levels must be an integer >= 2",
        ),
    ],
)
def test_native_handle_validates_numeric_style_ranges(method, args, style, message) -> None:
    handle = ruviz._native.NativePlotHandle()

    with pytest.raises(ValueError, match=message):
        getattr(handle, method)(*args, style)


def test_native_handle_rejects_zero_dpi() -> None:
    handle = ruviz._native.NativePlotHandle()

    with pytest.raises(ValueError, match="plot dpi must be greater than zero"):
        handle.dpi(0)


MARKER_ALIAS_CASES = sorted(ruviz._api._MARKER_ALIASES.items())
LINESTYLE_ALIAS_CASES = sorted(ruviz._api._LINESTYLE_ALIASES.items())


@pytest.mark.parametrize(
    ("shorthand", "canonical"),
    MARKER_ALIAS_CASES,
    ids=[canonical for _, canonical in MARKER_ALIAS_CASES],
)
def test_marker_shorthand_renders_as_its_canonical_name(shorthand: str, canonical: str) -> None:
    alias = _styled_base().line(STYLE_X, STYLE_Y, marker=shorthand)
    expected = _styled_base().line(STYLE_X, STYLE_Y, marker=canonical)

    assert alias.to_snapshot()["series"][0]["style"] == {"marker": canonical}
    assert alias.render_png() == expected.render_png()
    assert _styled_base().line(STYLE_X, STYLE_Y, marker=shorthand.upper()).render_png() == (
        expected.render_png()
    )


@pytest.mark.parametrize(
    ("shorthand", "canonical"),
    LINESTYLE_ALIAS_CASES,
    ids=[canonical for _, canonical in LINESTYLE_ALIAS_CASES],
)
def test_linestyle_shorthand_renders_as_its_canonical_name(shorthand: str, canonical: str) -> None:
    alias = _styled_base().line(STYLE_X, STYLE_Y, linestyle=shorthand)
    expected = _styled_base().line(STYLE_X, STYLE_Y, linestyle=canonical)

    assert alias.to_snapshot()["series"][0]["style"] == {"linestyle": canonical}
    assert alias.render_png() == expected.render_png()


def _source_kinds(snapshot: dict[str, object]) -> set[str]:
    return {
        source["kind"]
        for series in snapshot["series"]
        for source in series.values()
        if isinstance(source, dict) and "kind" in source
    }


def test_clone_marks_frozen_observable_sources_static() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().line([0.0, 1.0, 2.0], source).histogram(source)

    clone = plot.clone()

    assert _source_kinds(clone.to_snapshot()) == {"static"}
    assert "observable" in _source_kinds(plot.to_snapshot())
    assert clone.render_png().startswith(PNG_HEADER)


def test_render_png_works_from_a_worker_thread() -> None:
    plot = ruviz.plot().size_px(200, 150).line([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])
    rendered: list[bytes] = []

    thread = threading.Thread(target=lambda: rendered.append(plot.render_png()))
    thread.start()
    thread.join()

    assert rendered[0].startswith(PNG_HEADER)
    assert rendered[0] == plot.render_png()


def test_observable_set_at_works_from_a_worker_thread() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    plot = ruviz.plot().line([0.0, 1.0, 2.0], source)

    thread = threading.Thread(target=lambda: source.set_at(0, 9.0))
    thread.start()
    thread.join()

    assert source.snapshot_values() == [9.0, 2.0, 3.0]
    assert plot.to_snapshot()["series"][0]["y"]["values"] == [9.0, 2.0, 3.0]


def test_sibling_series_derived_from_one_source_resize_together() -> None:
    x = ruviz.observable([1.0, 2.0, 3.0])
    y = np.sin(x)
    plot = ruviz.plot().line(x, y)

    x.replace([1.0, 2.0, 3.0, 4.0])

    assert len(x) == len(y) == 4
    np.testing.assert_allclose(y.values(), np.sin(x.values()))
    series = plot.to_snapshot()["series"][0]
    assert series["x"]["values"] == [1.0, 2.0, 3.0, 4.0]
    np.testing.assert_allclose(series["y"]["values"], np.sin([1.0, 2.0, 3.0, 4.0]))
    assert plot.render_png().startswith(PNG_HEADER)


def test_diamond_derivation_graph_resizes_and_settles() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    doubled = source * 2.0
    shifted = source + 1.0
    combined = doubled + shifted
    plot = ruviz.plot().line(source, combined)

    source.replace([1.0, 2.0, 3.0, 4.0])

    assert [len(node) for node in (source, doubled, shifted, combined)] == [4, 4, 4, 4]
    assert doubled.snapshot_values() == [2.0, 4.0, 6.0, 8.0]
    assert shifted.snapshot_values() == [2.0, 3.0, 4.0, 5.0]
    assert combined.snapshot_values() == [4.0, 7.0, 10.0, 13.0]
    assert plot.to_snapshot()["series"][0]["y"]["values"] == [4.0, 7.0, 10.0, 13.0]
    assert plot.render_png().startswith(PNG_HEADER)


def test_uneven_derivation_paths_to_one_child_resize_and_settle() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])
    short = source * 2.0
    long_first = source + 1.0
    long_second = long_first * 3.0
    combined = short + long_second

    source.replace([1.0, 2.0, 3.0, 4.0])

    assert [len(node) for node in (short, long_first, long_second, combined)] == [4, 4, 4, 4]
    assert combined.snapshot_values() == [8.0, 13.0, 18.0, 23.0]


@pytest.mark.parametrize(
    ("build", "message"),
    [
        (lambda: ruviz.plot().line("time", "value"), "line x is a string"),
        (lambda: ruviz.plot().scatter([0.0, 1.0], "value"), "scatter y is a string"),
        (lambda: ruviz.plot().bar("category", [1.0, 2.0]), "bar categories is a string"),
        (lambda: ruviz.observable("values"), "observable values is a string"),
    ],
    ids=["line-x", "scatter-y", "bar-categories", "observable"],
)
def test_string_inputs_without_data_report_the_missing_lookup(build, message: str) -> None:
    with pytest.raises(TypeError, match=f"{message}; pass data="):
        build()


def test_bar_categories_reject_observables() -> None:
    source = ruviz.observable([1.0, 2.0])

    with pytest.raises(TypeError, match="bar categories does not support ObservableSeries"):
        ruviz.plot().bar(source, [1.0, 2.0])


def test_observable_supports_len_and_integer_indexing() -> None:
    source = ruviz.observable([1.0, 2.0, 3.0])

    assert len(source) == 3
    assert source[0] == 1.0
    assert source[-1] == 3.0

    with pytest.raises(IndexError):
        source[3]

    with pytest.raises(TypeError, match="indices must be integers"):
        source[0:2]
