from __future__ import annotations

from pathlib import Path

from _shared import ExampleMeta, base_plot, save_example, wave_series

META = ExampleMeta(
    slug="notebook-export",
    title="Notebook export flow",
    summary="Render a widget in Jupyter and save a static image alongside it.",
    section="Interactive workflows",
    gallery=False,
)


def build_plot():
    x, y = wave_series()
    return (
        base_plot("Notebook Export")
        .xlabel("x")
        .ylabel("signal")
        .line(x, y)
    )


def export_static(path: str | Path = "notebook-export.png") -> Path:
    return build_plot().save(path)


if __name__ == "__main__":
    save_example(META, build_plot())
