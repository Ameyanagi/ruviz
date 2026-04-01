from __future__ import annotations

from _shared import ExampleMeta, base_plot, save_example, scatter_series

META = ExampleMeta(
    slug="scatter",
    title="Scatter plot",
    summary="A scatter plot for irregular point clouds.",
    section="Basic plots",
)


def build_plot():
    x, y = scatter_series()
    return (
        base_plot("Scatter Plot")
        .xlabel("feature")
        .ylabel("response")
        .scatter(x, y)
    )


if __name__ == "__main__":
    save_example(META, build_plot())
