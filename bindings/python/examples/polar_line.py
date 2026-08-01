from __future__ import annotations

from _shared import ExampleMeta, base_plot, polar_series, save_example

META = ExampleMeta(
    slug="polar-line",
    title="Polar line",
    summary="A polar line rendered from radius and angle vectors.",
    section="Specialized plots",
)


def build_plot():
    radius, theta = polar_series()
    return base_plot("Polar Line").polar_line(radius, theta)


if __name__ == "__main__":
    save_example(META, build_plot())
