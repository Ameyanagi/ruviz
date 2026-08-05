from __future__ import annotations

from _shared import ExampleMeta, base_plot, save_example, wave_series

META = ExampleMeta(
    slug="line",
    title="Line plot",
    summary="A fluent line plot with a styled, labelled series and a legend.",
    section="Basic plots",
)


def build_plot():
    x, y = wave_series()
    return (
        base_plot("Line Plot")
        .xlabel("x")
        .ylabel("signal")
        .line(x, y, label="signal", color="#2563eb", width=2.0)
        .legend("upper_right")
    )


if __name__ == "__main__":
    save_example(META, build_plot())
