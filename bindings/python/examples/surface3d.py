from __future__ import annotations

import numpy as np

import ruviz
from _shared import ExampleMeta, save_example

META = ExampleMeta(
    slug="surface3d",
    title="3D surface",
    summary="An opaque regular-grid surface using the static Python 3D alpha.",
    section="3D plots",
    gallery=False,
)


def build_plot() -> ruviz.Plot3D:
    x = np.linspace(-3.0, 3.0, 36)
    y = np.linspace(-3.0, 3.0, 28)
    grid_x, grid_y = np.meshgrid(x, y)
    radius = np.hypot(grid_x, grid_y)
    z = np.sin(radius * 2.2) / (1.0 + radius)

    return (
        ruviz.surface(x, y, z)
        .size_px(760, 480)
        .title("Damped radial wave")
        .xlabel("x")
        .ylabel("y")
        .zlabel("amplitude")
        .azimuth_deg(42.0)
        .elevation_deg(27.0)
    )


if __name__ == "__main__":
    save_example(META, build_plot())
