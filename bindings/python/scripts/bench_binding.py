"""Rough timings for the NumPy -> native -> render path.

Not a benchmark suite and not collected by pytest: a small single-machine
comparison used to sanity-check the binding's copy path and GIL handling.
Each stage is timed on a freshly built plot and the fastest round is reported.

    uv run python scripts/bench_binding.py [points] [rounds]
"""

from __future__ import annotations

import sys
from collections import defaultdict
from collections.abc import Callable
from time import perf_counter
from typing import Any

import numpy as np

import ruviz

STAGES = ("add line series", "render_png (first)", "render_png (cached)", "to_snapshot")


def _timed(call: Callable[[], Any]) -> float:
    start = perf_counter()
    call()
    return perf_counter() - start


def main(points: int, rounds: int) -> None:
    x = np.linspace(0.0, 100.0, num=points, dtype=np.float64)
    y = np.sin(x) + 0.2 * np.cos(x * 3.0)
    timings: defaultdict[str, list[float]] = defaultdict(list)

    for _ in range(rounds):
        plot = ruviz.plot().size_px(800, 600).ticks(False)
        timings["add line series"].append(_timed(lambda: plot.line(x, y)))
        timings["render_png (first)"].append(_timed(plot.render_png))
        timings["render_png (cached)"].append(_timed(plot.render_png))
        timings["to_snapshot"].append(_timed(plot.to_snapshot))

    print(f"points={points:,} rounds={rounds} (best of {rounds})")
    for stage in STAGES:
        print(f"{stage:<22} {min(timings[stage]) * 1000:9.1f} ms")


if __name__ == "__main__":
    main(
        int(sys.argv[1]) if len(sys.argv) > 1 else 1_000_000,
        int(sys.argv[2]) if len(sys.argv) > 2 else 5,
    )
