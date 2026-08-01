from __future__ import annotations

from _shared import ExampleMeta, base_plot, sample_distribution, save_example

META = ExampleMeta(
    slug="violin",
    title="Violin plot",
    summary="A violin plot for density and spread in one view.",
    section="Statistical plots",
)


def build_plot():
    return base_plot("Violin Plot").ylabel("value").violin(sample_distribution())


if __name__ == "__main__":
    save_example(META, build_plot())
