from __future__ import annotations

import ruviz

from _shared import ExampleMeta, base_plot, save_example

META = ExampleMeta(
    slug="notebook-observable",
    title="Notebook observables",
    summary="Observable series driving an explicit widget view in Jupyter.",
    section="Interactive workflows",
    gallery=False,
)


def build_plot():
    source = ruviz.observable([0.2, 0.9, 0.5, 1.3, 0.8])
    return base_plot("Observable Notebook Plot").line([0, 1, 2, 3, 4], source)


def build_widget():
    source = ruviz.observable([0.2, 0.9, 0.5, 1.3, 0.8])
    plot = base_plot("Observable Notebook Plot").line([0, 1, 2, 3, 4], source)
    return plot.widget(), source


if __name__ == "__main__":
    widget, source = build_widget()
    source.replace([0.3, 1.1, 0.7, 1.0, 0.6])
    save_example(META, build_plot())
