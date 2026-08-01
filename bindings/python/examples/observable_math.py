from __future__ import annotations

import numpy as np
import ruviz

from _shared import ExampleMeta, base_plot, save_example

META = ExampleMeta(
    slug="observable-math",
    title="Observable math",
    summary="Compose live derived observables with arithmetic and NumPy ufuncs.",
    section="Interactive workflows",
)


def build_sources():
    x = np.linspace(0.0, 6.0, 160)
    amplitude = ruviz.observable(0.8 + 0.15 * np.sin(x * 0.7))
    phase = ruviz.observable(np.linspace(0.0, 1.2, x.size))
    signal = np.sin((phase * 2.0) + x) * amplitude
    return x.tolist(), amplitude, phase, signal


def build_plot():
    x, amplitude, _, signal = build_sources()
    amplitude_line = [value * 0.9 for value in amplitude.snapshot_values()]
    return (
        base_plot("Observable Math")
        .xlabel("x")
        .ylabel("value")
        .line(x, signal)
        .line(x, amplitude_line)
    )


def build_widget():
    x, amplitude, phase, signal = build_sources()
    plot = (
        base_plot("Observable Math Widget")
        .size_px(640, 360)
        .xlabel("x")
        .ylabel("value")
        .line(x, signal)
    )
    return plot.widget(), amplitude, phase


if __name__ == "__main__":
    save_example(META, build_plot())
