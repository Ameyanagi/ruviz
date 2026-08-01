from __future__ import annotations

from _shared import ExampleMeta, base_plot, radar_inputs, save_example

META = ExampleMeta(
    slug="radar",
    title="Radar chart",
    summary="Multi-axis comparison for runtime capabilities.",
    section="Categorical plots",
)


def build_plot():
    labels, series = radar_inputs()
    return base_plot("Runtime Radar").radar(labels, series)


if __name__ == "__main__":
    save_example(META, build_plot())
