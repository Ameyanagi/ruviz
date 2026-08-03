from __future__ import annotations

import sys
from pathlib import Path
from unittest.mock import patch

import numpy as np
import pytest
import ruviz

if sys.version_info >= (3, 11):
    import tomllib
else:  # Python 3.10 has no stdlib tomllib
    import tomli as tomllib

PNG_HEADER = b"\x89PNG\r\n\x1a\n"


def _grid() -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    x = np.linspace(-1.0, 1.0, 4)
    y = np.linspace(-1.0, 1.0, 3)
    grid_x, grid_y = np.meshgrid(x, y)
    z = np.sin(grid_x * 2.0) * np.cos(grid_y * 2.0)
    return x, y, z


def test_python_build_enables_exact_3d_cargo_feature() -> None:
    cargo_toml = Path(__file__).resolve().parents[1] / "Cargo.toml"
    with cargo_toml.open("rb") as source:
        metadata = tomllib.load(source)

    assert "3d" in metadata["dependencies"]["ruviz"]["features"]


def test_all_four_3d_entry_points_are_public() -> None:
    x, y, z = _grid()

    cases = [
        (ruviz.scatter3d([0, 1], [0, 1], [0, 1]), "scatter3d"),
        (ruviz.line3d([0, 1], [0, 1], [0, 1]), "line3d"),
        (ruviz.surface(x, y, z), "surface"),
        (ruviz.wireframe(x, y, z), "wireframe"),
    ]

    for plot, kind in cases:
        assert isinstance(plot, ruviz.Plot3D)
        assert plot.to_snapshot()["series"][0]["kind"] == kind


def test_plot3d_combines_series_and_exports_png_svg_and_pdf(tmp_path: Path) -> None:
    x, y, z = _grid()
    plot = (
        ruviz.plot3d()
        .size_px(320, 240)
        .theme("light")
        .title("Python 3D alpha")
        .xlabel("x")
        .ylabel("y")
        .zlabel("z")
        .surface(x, y, z)
        .wireframe(x, y, z)
        .line3d([-1.0, 1.0], [-1.0, 1.0], [-0.5, 0.5])
        .scatter3d([-0.5, 0.5], [0.5, -0.5], [0.25, -0.25])
        .azimuth_deg(38.0)
        .elevation_deg(24.0)
    )

    png = plot.render_png()
    svg = plot.render_svg()
    assert png.startswith(PNG_HEADER)
    assert len(png) > 2_048
    assert svg.startswith("<?xml")
    assert "<svg" in svg
    assert "<image" in svg

    png_path = plot.save(tmp_path / "scene.png")
    svg_path = plot.save(tmp_path / "scene.svg")
    pdf_path = plot.save(tmp_path / "scene.pdf")
    assert png_path.read_bytes().startswith(PNG_HEADER)
    assert svg_path.read_text(encoding="utf-8").startswith("<?xml")
    assert pdf_path.read_bytes().startswith(b"%PDF")


def test_plot3d_render_delegates_to_the_native_handle() -> None:
    plot = ruviz.scatter3d([0.0, 1.0], [0.0, 1.0], [0.0, 1.0]).size_px(240, 180)

    with patch.object(
        type(plot._native_plot), "render_png_bytes", return_value=b"native-png"
    ) as render:
        assert plot.render_png() == b"native-png"

    render.assert_called_once()


def test_plot3d_reuses_the_cached_builder_across_renders() -> None:
    plot = ruviz.scatter3d([0.0, 1.0], [0.0, 1.0], [0.0, 1.0]).size_px(240, 180)

    first = plot.render_png()
    assert plot.render_png() == first

    assert plot.title("changed").render_png() != first


def test_surface_requires_y_rows_by_x_columns() -> None:
    with pytest.raises(
        ValueError,
        match=r"surface z shape must be \(len\(y\), len\(x\)\)",
    ):
        ruviz.surface([0.0, 1.0, 2.0], [0.0, 1.0], np.zeros((3, 2)))


def test_point_series_reject_matrix_coordinates() -> None:
    with pytest.raises(TypeError, match="scatter3d z must be a 1D numeric array"):
        ruviz.scatter3d([0.0, 1.0], [0.0, 1.0], [[0.0, 1.0]])


def test_empty_plot3d_reports_a_clear_error() -> None:
    with pytest.raises(ValueError, match="must contain at least one series"):
        ruviz.plot3d().render_png()


def test_plot3d_save_rejects_unknown_extension(tmp_path: Path) -> None:
    plot = ruviz.scatter3d([0.0, 1.0], [0.0, 1.0], [0.0, 1.0]).size_px(160, 120)

    with pytest.raises(ValueError, match=r"unsupported save extension '\.jpg'"):
        plot.save(tmp_path / "scene.jpg")


def test_plot3d_save_rejects_path_without_extension(tmp_path: Path) -> None:
    plot = ruviz.scatter3d([0.0, 1.0], [0.0, 1.0], [0.0, 1.0]).size_px(160, 120)

    with pytest.raises(ValueError, match="has no extension"):
        plot.save(tmp_path / "scene")


def test_plot3d_save_accepts_uppercase_extensions(tmp_path: Path) -> None:
    plot = ruviz.scatter3d([0.0, 1.0], [0.0, 1.0], [0.0, 1.0]).size_px(160, 120)

    assert plot.save(tmp_path / "scene.PNG").read_bytes().startswith(PNG_HEADER)


def test_plot3d_theme_normalizes_case_and_rejects_unknown_themes() -> None:
    assert ruviz.plot3d().theme("Dark").to_snapshot()["theme"] == "dark"

    with pytest.raises(ValueError, match="unsupported theme: solarized"):
        ruviz.plot3d().theme("solarized")


@pytest.mark.parametrize(("width", "height"), [(0, 100), (100, 0), (-1, 100)])
def test_plot3d_size_px_rejects_non_positive_dimensions(width: int, height: int) -> None:
    with pytest.raises(ValueError, match="greater than zero"):
        ruviz.plot3d().size_px(width, height)


def test_plot3d_rejects_observable_and_dataframe_inputs() -> None:
    pd = pytest.importorskip("pandas")
    frame = pd.DataFrame({"x": [0.0, 1.0], "y": [0.0, 1.0], "z": [0.0, 1.0]})

    plot = ruviz.plot3d().scatter3d("x", "y", "z", data=frame)
    assert plot.to_snapshot()["series"][0]["x"] == [0.0, 1.0]

    with pytest.raises(TypeError, match="select a column or pass data="):
        ruviz.scatter3d(frame, frame, frame)

    with pytest.raises(TypeError, match="data= expects a DataFrame or dict"):
        ruviz.plot3d().scatter3d("x", "y", "z", data=frame["x"])


def test_plot3d_snapshot_carries_schema_version() -> None:
    plot = ruviz.scatter3d([0, 1], [0, 1], [0, 1]).title("versioned")

    assert plot.to_snapshot()["schemaVersion"] == 1
