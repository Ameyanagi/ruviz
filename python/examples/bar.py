from __future__ import annotations

from _shared import ExampleMeta, base_plot, categorical_series, save_example

META = ExampleMeta(
    slug="bar",
    title="Bar chart",
    summary="Categorical metrics rendered as a bar chart.",
    section="Basic plots",
)


def build_plot():
    categories, values = categorical_series()
    return base_plot("Runtime Coverage").ylabel("score").bar(categories, values)


if __name__ == "__main__":
    save_example(META, build_plot())
