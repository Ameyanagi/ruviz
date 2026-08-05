from __future__ import annotations

from _shared import ExampleMeta, base_plot, sample_distribution, save_example

META = ExampleMeta(
    slug="kde",
    title="Kernel density estimate",
    summary="A smoothed density curve for a numeric sample with an explicit bandwidth.",
    section="Statistical plots",
)


def build_plot():
    return (
        base_plot("Kernel Density Estimate")
        .xlabel("value")
        .kde(sample_distribution(), bandwidth=0.35, color="#7c3aed", width=2.0)
    )


if __name__ == "__main__":
    save_example(META, build_plot())
