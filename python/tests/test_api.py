from __future__ import annotations

import gc
import weakref
from copy import copy, deepcopy
from pathlib import Path
from unittest.mock import patch

import numpy as np
import pytest
import ruviz


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

    assert png.startswith(b"\x89PNG\r\n\x1a\n")


def test_render_png_uses_native_handle_not_snapshot_json() -> None:
    plot = ruviz.plot().line([0, 1, 2], [0, 1, 4]).title("demo")

    with patch.object(plot, "_snapshot_json", side_effect=AssertionError("snapshot path should not run")):
        png = plot.render_png()

    assert png.startswith(b"\x89PNG\r\n\x1a\n")


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

    assert clone.render_png().startswith(b"\x89PNG\r\n\x1a\n")
    assert clone.to_snapshot() == plot.to_snapshot()


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
