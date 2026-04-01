from __future__ import annotations

from _shared import ExampleMeta, base_plot, sample_distribution, save_example

META = ExampleMeta(
    slug="kde",
    title="Kernel density estimate",
    summary="A smoothed density curve for a numeric sample.",
    section="Statistical plots",
)


def build_plot():
    return base_plot("Kernel Density Estimate").xlabel("value").kde(sample_distribution())


if __name__ == "__main__":
    save_example(META, build_plot())
