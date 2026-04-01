from __future__ import annotations

from _shared import ExampleMeta, base_plot, heatmap_values, save_example

META = ExampleMeta(
    slug="heatmap",
    title="Heatmap",
    summary="A rectangular numeric matrix rendered as a heatmap.",
    section="Matrix plots",
)


def build_plot():
    return base_plot("Heatmap", theme="dark").heatmap(heatmap_values())


if __name__ == "__main__":
    save_example(META, build_plot())
