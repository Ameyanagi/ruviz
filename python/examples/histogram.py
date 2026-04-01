from __future__ import annotations

from _shared import ExampleMeta, base_plot, sample_distribution, save_example

META = ExampleMeta(
    slug="histogram",
    title="Histogram",
    summary="A distribution view built from a deterministic sample.",
    section="Statistical plots",
)


def build_plot():
    return base_plot("Histogram").xlabel("value").histogram(sample_distribution())


if __name__ == "__main__":
    save_example(META, build_plot())
