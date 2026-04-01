from __future__ import annotations

from _shared import ExampleMeta, base_plot, error_bar_series, save_example

META = ExampleMeta(
    slug="error-bars",
    title="Vertical error bars",
    summary="A line-like series with y-direction uncertainty.",
    section="Statistical plots",
)


def build_plot():
    x, y, errors = error_bar_series()
    return (
        base_plot("Vertical Error Bars")
        .xlabel("trial")
        .ylabel("measurement")
        .error_bars(x, y, errors)
    )


if __name__ == "__main__":
    save_example(META, build_plot())
