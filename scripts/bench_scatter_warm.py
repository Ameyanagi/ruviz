#!/usr/bin/env python3
"""Warm-state arm of scripts/bench_scatter_vs.py, measured in-process.

One warmup render pays every process-cold cost (imports stay excluded, font
caches, sprite caches, JITs, whatever each library amortizes), then each size
builds a fresh chart and times chart-to-PNG. Reported value is the minimum of
three runs.
"""

from __future__ import annotations

import sys
import time

import numpy as np

WIDTH, HEIGHT, DPI, SEED = 900, 420, 100, 20260713


def make_data(n: int):
    rng = np.random.default_rng(SEED)
    x = rng.standard_normal(n, dtype=np.float32)
    y = rng.standard_normal(n, dtype=np.float32)
    y *= np.float32(1.2)
    y += x
    y *= np.float32(0.5)
    return x, y


def main() -> None:
    library = sys.argv[1]
    sizes = [int(v) for v in sys.argv[2].split(",")]

    if library == "xy":
        from xy import Engine, scatter, scatter_chart

        def render(x, y):
            fig = scatter_chart(scatter(x=x, y=y), width=WIDTH, height=HEIGHT).figure()
            return fig.to_png(width=WIDTH, height=HEIGHT, scale=1, engine=Engine.default)
    elif library in ("ruviz", "ruviz-fast"):
        import ruviz

        def render(x, y):
            plot = ruviz.plot()
            if library == "ruviz-fast":
                plot = plot.fast()
            return plot.scatter(x, y).size_px(WIDTH, HEIGHT).render_png()
    elif library == "matplotlib":
        import io

        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt

        def render(x, y):
            fig, ax = plt.subplots(figsize=(WIDTH / DPI, HEIGHT / DPI), dpi=DPI)
            ax.scatter(x, y)
            out = io.BytesIO()
            fig.savefig(out, format="png", dpi=DPI)
            plt.close(fig)
            return out.getvalue()
    else:
        raise SystemExit(f"unknown library {library}")

    warm_x, warm_y = make_data(1_000)
    render(warm_x, warm_y)

    for n in sizes:
        x, y = make_data(n)
        times = []
        for _ in range(3):
            t0 = time.perf_counter()
            png = render(x, y)
            times.append(time.perf_counter() - t0)
        assert len(png) > 1000
        print(f"{library}\t{n}\t{1e3 * min(times):.1f}", flush=True)


if __name__ == "__main__":
    main()
