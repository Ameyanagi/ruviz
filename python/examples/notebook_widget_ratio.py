from __future__ import annotations

import numpy as np
import ruviz

from _shared import ExampleMeta, save_example

META = ExampleMeta(
    slug="notebook-widget-ratio",
    title="Notebook widget aspect ratio",
    summary="Notebook widgets follow the plot aspect ratio configured by `size_px(width, height)`.",
    section="Interactive workflows",
    gallery=False,
)


def build_plot():
    x = np.linspace(0.0, 8.0, 220)
    y = np.sin(x) * 0.7 + np.cos(x * 2.4) * 0.25
    return (
        ruviz.plot()
        .size_px(640, 360)
        .theme("light")
        .ticks(True)
        .title("16:9 Notebook Widget")
        .xlabel("time")
        .ylabel("signal")
        .line(x, y)
    )


def build_widget():
    return build_plot().widget()


if __name__ == "__main__":
    save_example(META, build_plot())
