from __future__ import annotations

from _shared import ExampleMeta, base_plot, sample_distribution, save_example

META = ExampleMeta(
    slug="histogram",
    title="Histogram",
    summary="A distribution view built from a deterministic sample with an explicit bin count.",
    section="Statistical plots",
)


def build_plot():
    return (
        base_plot("Histogram")
        .xlabel("value")
        .histogram(sample_distribution(), bins=24, color="#f97316", alpha=0.85)
    )


if __name__ == "__main__":
    save_example(META, build_plot())
