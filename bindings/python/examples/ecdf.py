from __future__ import annotations

from _shared import ExampleMeta, base_plot, sample_distribution, save_example

META = ExampleMeta(
    slug="ecdf",
    title="ECDF",
    summary="An empirical cumulative distribution plot for ranked samples.",
    section="Statistical plots",
)


def build_plot():
    return base_plot("ECDF").xlabel("value").ylabel("probability").ecdf(sample_distribution())


if __name__ == "__main__":
    save_example(META, build_plot())
