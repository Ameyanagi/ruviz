from __future__ import annotations

from _shared import ExampleMeta, base_plot, save_example, wave_series

META = ExampleMeta(
    slug="line",
    title="Line plot",
    summary="A basic fluent line plot built with chained Python methods.",
    section="Basic plots",
)


def build_plot():
    x, y = wave_series()
    return (
        base_plot("Line Plot")
        .xlabel("x")
        .ylabel("signal")
        .line(x, y)
    )


if __name__ == "__main__":
    save_example(META, build_plot())
