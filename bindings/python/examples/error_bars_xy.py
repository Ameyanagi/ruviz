from __future__ import annotations

from _shared import ExampleMeta, base_plot, error_bar_xy_series, save_example

META = ExampleMeta(
    slug="error-bars-xy",
    title="Horizontal and vertical error bars",
    summary="A point series with uncertainty in both axes.",
    section="Statistical plots",
)


def build_plot():
    x, y, x_errors, y_errors = error_bar_xy_series()
    return (
        base_plot("XY Error Bars")
        .xlabel("throughput")
        .ylabel("latency")
        .error_bars_xy(x, y, x_errors, y_errors)
    )


if __name__ == "__main__":
    save_example(META, build_plot())
