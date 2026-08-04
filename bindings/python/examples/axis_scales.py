from __future__ import annotations

from _shared import ExampleMeta, base_plot, decay_series, save_example

META = ExampleMeta(
    slug="axis-scales",
    title="Axis limits and scales",
    summary="Explicit axis limits, a logarithmic y-axis, and a grid behind two labelled series.",
    section="Basic plots",
)


def build_plot():
    x, fast, slow = decay_series()
    return (
        base_plot("Decay Rates")
        .xlabel("time")
        .ylabel("intensity")
        .line(x, fast, label="fast decay", color="#2563eb", width=2.0)
        .line(x, slow, label="slow decay", color="orange", linestyle="dashed")
        .xlim(0.0, 12.0)
        .yscale("log")
        .grid(True)
        .legend("upper_right")
    )


if __name__ == "__main__":
    save_example(META, build_plot())
