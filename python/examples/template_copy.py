from __future__ import annotations

from copy import deepcopy

from _shared import ExampleMeta, base_plot, save_example, wave_series

META = ExampleMeta(
    slug="template-copy",
    title="Deepcopy plot template",
    summary="Fork a reusable plot template with `deepcopy(plot)` before adding variant-specific series.",
    section="Integration",
)


def build_template():
    x, y = wave_series()
    return (
        base_plot("Deepcopy Template")
        .xlabel("time")
        .ylabel("signal")
        .line(x, y)
    )


def build_plot():
    x, y = wave_series()
    template = build_template()
    variant = deepcopy(template).title("Deepcopy Template Copy")
    shifted = [value * 0.65 + 0.35 for value in y]
    return variant.line(x, shifted)


if __name__ == "__main__":
    save_example(META, build_plot())
