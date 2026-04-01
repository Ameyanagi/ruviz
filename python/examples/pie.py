from __future__ import annotations

from _shared import ExampleMeta, base_plot, save_example

META = ExampleMeta(
    slug="pie",
    title="Pie chart",
    summary="A simple composition view with labels.",
    section="Categorical plots",
)


def build_plot():
    labels = ["Exports", "Widgets", "WASM", "Docs"]
    values = [30.0, 26.0, 24.0, 20.0]
    return base_plot("Feature Mix").pie(values, labels)


if __name__ == "__main__":
    save_example(META, build_plot())
