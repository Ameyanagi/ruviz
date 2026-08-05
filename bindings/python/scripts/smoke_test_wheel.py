"""Smoke test an installed ruviz distribution before it is published.

Run this with the interpreter of a fresh virtualenv that has the built wheel or
sdist installed, from any working directory: it checks the published artifact,
not the source tree.
"""

from __future__ import annotations

import argparse
import importlib.resources
import importlib.util
import sys
from pathlib import Path
from typing import Any


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--label", default="smoke", help="Name used in the rendered plot title")
    parser.add_argument(
        "--no-native-show",
        action="store_true",
        help="Assert plot.show() refuses to open a native window (published Linux wheels)",
    )
    args = parser.parse_args()

    import ruviz

    module_path = Path(ruviz.__file__).resolve()
    assert "site-packages" in module_path.parts, f"imported the source tree: {module_path}"

    assert isinstance(ruviz.__version__, str) and ruviz.__version__, "missing __version__"

    plot = ruviz.plot().line([0.0, 1.0, 2.0], [0.0, 1.0, 0.5]).title(f"{args.label} smoke")
    png = plot.render_png()
    assert isinstance(png, (bytes, bytearray)), f"render_png returned {type(png)!r}"
    assert png.startswith(b"\x89PNG"), "render_png did not return PNG bytes"

    py_typed = importlib.resources.files("ruviz").joinpath("py.typed")
    assert py_typed.is_file(), "the distribution does not ship ruviz/py.typed"

    _check_widget_extra(ruviz)

    if args.no_native_show:
        _check_native_show_is_unavailable(plot)

    print(f"ruviz {ruviz.__version__} smoke test passed ({args.label})")
    return 0


def _check_widget_extra(ruviz: Any) -> None:
    """Widgets work with the extra installed and fail clearly without it."""
    if importlib.util.find_spec("anywidget") is None:
        try:
            widget_type = ruviz.RuvizWidget
        except ImportError as exc:
            assert "ruviz[widget]" in str(exc), f"unhelpful widget error: {exc}"
            return
        raise AssertionError(f"{widget_type} should fail without the ruviz[widget] extra")

    widget = ruviz.plot().line([0.0, 1.0], [0.0, 1.0]).widget()
    assert widget.snapshot["series"], "widget snapshot is empty"


def _check_native_show_is_unavailable(plot: Any) -> None:
    try:
        plot.show()
    except RuntimeError as exc:
        assert "install ruviz from source on Linux" in str(exc), f"unexpected show() error: {exc}"
    else:
        raise AssertionError("expected plot.show() to raise on the published Linux wheel build")


if __name__ == "__main__":
    sys.exit(main())
