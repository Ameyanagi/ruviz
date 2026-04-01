from __future__ import annotations

from _shared import ExampleMeta, base_plot, contour_grid, save_example

META = ExampleMeta(
    slug="contour",
    title="Contour plot",
    summary="Contours computed from a flattened z-grid over x/y axes.",
    section="Matrix plots",
)


def build_plot():
    x, y, z = contour_grid()
    return base_plot("Contour Plot").contour(x, y, z)


if __name__ == "__main__":
    save_example(META, build_plot())
