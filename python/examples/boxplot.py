from __future__ import annotations

from _shared import ExampleMeta, base_plot, sample_distribution, save_example

META = ExampleMeta(
    slug="boxplot",
    title="Boxplot",
    summary="Quartiles and outliers summarized as a boxplot.",
    section="Statistical plots",
)


def build_plot():
    return base_plot("Boxplot").ylabel("value").boxplot(sample_distribution())


if __name__ == "__main__":
    save_example(META, build_plot())
