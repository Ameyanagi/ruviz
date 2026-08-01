from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import numpy as np
import pandas as pd

import ruviz

EXAMPLES_DIR = Path(__file__).resolve().parent
OUTPUT_DIR = EXAMPLES_DIR / "output"


@dataclass(frozen=True)
class ExampleMeta:
    slug: str
    title: str
    summary: str
    section: str
    gallery: bool = True


def wave_series(points: int = 120, extent: float = 6.0) -> tuple[list[float], list[float]]:
    x = np.linspace(0.0, extent, points)
    y = np.sin(x) + 0.35 * np.cos(x * 2.2)
    return x.tolist(), y.tolist()


def scatter_series(points: int = 48) -> tuple[list[float], list[float]]:
    x = np.linspace(-2.5, 2.5, points)
    y = 0.4 * x**3 - 0.8 * x + np.sin(x * 3.0) * 0.35
    return x.tolist(), y.tolist()


def categorical_series() -> tuple[list[str], list[float]]:
    categories = ["CPU", "SVG", "GPU", "WASM", "Jupyter"]
    values = [3.8, 2.6, 4.4, 4.9, 4.1]
    return categories, values


def sample_distribution() -> list[float]:
    left = np.linspace(-2.6, -0.3, 72) + np.sin(np.linspace(0.0, 5.0, 72)) * 0.14
    middle = np.linspace(-0.2, 1.1, 72) + np.cos(np.linspace(0.0, 8.0, 72)) * 0.09
    right = np.linspace(1.0, 2.9, 72) + np.sin(np.linspace(0.0, 6.5, 72)) * 0.12
    return np.concatenate([left, middle, right]).tolist()


def heatmap_values(rows: int = 7, cols: int = 7) -> list[list[float]]:
    y = np.linspace(-1.2, 1.2, rows)
    x = np.linspace(-1.2, 1.2, cols)
    grid = np.outer(np.sin(y * 2.0), np.cos(x * 2.6)) + np.outer(y, x) * 0.4
    return grid.tolist()


def error_bar_series() -> tuple[list[float], list[float], list[float]]:
    x = np.linspace(0.0, 5.0, 7)
    y = 1.2 + np.sin(x) * 0.9
    errors = 0.12 + np.abs(np.cos(x)) * 0.18
    return x.tolist(), y.tolist(), errors.tolist()


def error_bar_xy_series() -> tuple[list[float], list[float], list[float], list[float]]:
    x = np.linspace(0.8, 5.6, 6)
    y = 1.0 + np.cos(x * 0.8) * 0.75
    x_errors = (0.08 + np.linspace(0.02, 0.14, 6)).tolist()
    y_errors = (0.15 + np.abs(np.sin(x)) * 0.12).tolist()
    return x.tolist(), y.tolist(), x_errors, y_errors


def contour_grid() -> tuple[list[float], list[float], list[float]]:
    x = np.linspace(-2.5, 2.5, 24)
    y = np.linspace(-2.5, 2.5, 24)
    values: list[float] = []
    for y_value in y:
        for x_value in x:
            radius = np.hypot(x_value, y_value)
            values.append(np.sin(x_value * 1.8) * np.cos(y_value * 1.5) - radius * 0.08)
    return x.tolist(), y.tolist(), values


def radar_inputs() -> tuple[list[str], list[dict[str, object]]]:
    labels = ["API", "Docs", "Export", "Interactive", "Scale"]
    series = [
        {"name": "Python", "values": [4.5, 4.7, 4.8, 4.3, 4.0]},
        {"name": "Web", "values": [4.2, 4.1, 4.0, 4.8, 4.6]},
    ]
    return labels, series


def polar_series(points: int = 120) -> tuple[list[float], list[float]]:
    theta = np.linspace(0.0, np.pi * 4.0, points)
    radius = 0.35 + theta * 0.09 + np.sin(theta * 3.0) * 0.08
    return radius.tolist(), theta.tolist()


def sample_dataframe() -> pd.DataFrame:
    x, y = wave_series()
    baseline = np.linspace(min(y), max(y), len(y))
    return pd.DataFrame({"time": x, "value": y, "baseline": baseline})


def base_plot(title: str, *, theme: str = "light") -> ruviz.Plot:
    return ruviz.plot().size_px(760, 420).theme(theme).ticks(True).title(title)


def save_example(meta: ExampleMeta, plot: ruviz.Plot) -> Path:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    output = OUTPUT_DIR / f"{meta.slug}.png"
    plot.save(output)
    return output
