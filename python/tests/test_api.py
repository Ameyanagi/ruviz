from __future__ import annotations

import gc
import weakref
from pathlib import Path
from unittest.mock import patch

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
