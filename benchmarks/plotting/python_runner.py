from __future__ import annotations

import argparse
import io
import json
import platform
import time
from pathlib import Path
from typing import Any, Callable

import matplotlib

matplotlib.use("Agg")

import numpy as np
import ruviz
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from ruviz import _native

from common import ROOT, build_dataset, load_manifest, scenario_runs, summarize_iterations

PYTHON_CASE_BUDGET_SECONDS = 60.0


def measure(
    *,
    warmup_iterations: int,
    measured_iterations: int,
    fn: Callable[[], bytes],
    adaptive_budget_seconds: float | None = None,
) -> tuple[list[float], int, int, int]:
    effective_warmup = warmup_iterations
    effective_measured = measured_iterations
    iterations_ms: list[float] = []
    last_length = 0

    if adaptive_budget_seconds is not None and measured_iterations > 0:
        probe_start = time.perf_counter_ns()
        probe_payload = fn()
        probe_ms = (time.perf_counter_ns() - probe_start) / 1_000_000.0
        probe_seconds = max(probe_ms / 1000.0, 1e-9)
        max_total_iterations = max(1, int(adaptive_budget_seconds / probe_seconds))
        if max_total_iterations < warmup_iterations + measured_iterations:
            effective_warmup = min(warmup_iterations, max(0, max_total_iterations - 1))
            effective_measured = max(1, min(measured_iterations, max_total_iterations - effective_warmup))

        last_length = len(probe_payload)
        if effective_warmup > 0:
            remaining_warmups = effective_warmup - 1
        else:
            iterations_ms.append(probe_ms)
            remaining_warmups = 0
    else:
        remaining_warmups = warmup_iterations

    for _ in range(remaining_warmups):
        fn()

    remaining_measured = max(0, effective_measured - len(iterations_ms))
    for _ in range(remaining_measured):
        start = time.perf_counter_ns()
        payload = fn()
        elapsed_ms = (time.perf_counter_ns() - start) / 1_000_000.0
        iterations_ms.append(elapsed_ms)
        last_length = len(payload)
    return iterations_ms, last_length, effective_warmup, effective_measured


def build_ruviz_plot(run: dict[str, Any], dataset: dict[str, Any]) -> ruviz.Plot:
    canvas = run["canvas"]
    plot = ruviz.plot().size_px(canvas["width"], canvas["height"]).theme("light")
    if run["plotKind"] == "line":
        return plot.line(dataset["x"], dataset["y"])
    if run["plotKind"] == "scatter":
        return plot.scatter(dataset["x"], dataset["y"])
    if run["plotKind"] == "histogram":
        return plot.histogram(dataset["values"])
    if run["plotKind"] == "heatmap":
        return plot.heatmap(dataset["matrix"])
    raise ValueError(f"unsupported plot kind: {run['plotKind']}")


def build_matplotlib_canvas(run: dict[str, Any], dataset: dict[str, Any]) -> FigureCanvasAgg:
    canvas = run["canvas"]
    figure = Figure(
        figsize=(canvas["width"] / canvas["dpi"], canvas["height"] / canvas["dpi"]),
        dpi=canvas["dpi"],
        facecolor="white",
    )
    agg = FigureCanvasAgg(figure)
    axis = figure.subplots()
    axis.set_facecolor("white")
    if run["plotKind"] == "line":
        axis.plot(dataset["x"], dataset["y"], linewidth=1.0)
    elif run["plotKind"] == "scatter":
        axis.scatter(dataset["x"], dataset["y"], s=4)
    elif run["plotKind"] == "histogram":
        axis.hist(dataset["values"], bins="auto")
    elif run["plotKind"] == "heatmap":
        axis.imshow(dataset["matrix"], origin="lower", aspect="auto", interpolation="nearest")
    else:
        raise ValueError(f"unsupported plot kind: {run['plotKind']}")
    return agg


def canvas_png_bytes(canvas: FigureCanvasAgg) -> bytes:
    buffer = io.BytesIO()
    canvas.print_png(buffer)
    return buffer.getvalue()


def fresh_matplotlib_png(run: dict[str, Any], dataset: dict[str, Any]) -> bytes:
    canvas = build_matplotlib_canvas(run, dataset)
    try:
        return canvas_png_bytes(canvas)
    finally:
        canvas.figure.clear()


def benchmark_ruviz(
    run: dict[str, Any],
    dataset: dict[str, Any],
    *,
    adaptive_budget_seconds: float | None,
) -> list[dict[str, Any]]:
    prepared_plot = build_ruviz_plot(run, dataset)
    snapshot_json = json.dumps(prepared_plot.to_snapshot())

    render_only_ms, render_only_bytes, render_only_warmup, render_only_measured = measure(
        warmup_iterations=run["warmupIterations"],
        measured_iterations=run["measuredIterations"],
        fn=lambda: bytes(_native.render_png_bytes(snapshot_json)),
        adaptive_budget_seconds=adaptive_budget_seconds,
    )

    public_ms, public_bytes, public_warmup, public_measured = measure(
        warmup_iterations=run["warmupIterations"],
        measured_iterations=run["measuredIterations"],
        fn=lambda: build_ruviz_plot(run, dataset).render_png(),
        adaptive_budget_seconds=adaptive_budget_seconds,
    )

    return [
        {
            "implementation": "ruviz",
            "scenarioId": run["scenarioId"],
            "plotKind": run["plotKind"],
            "sizeLabel": run["size"]["label"],
            "boundary": "render_only",
            "outputTarget": "png_bytes",
            "elements": run["elements"],
            "canvas": run["canvas"],
            "datasetHash": dataset["hash"],
            "warmupIterations": render_only_warmup,
            "measuredIterations": render_only_measured,
            "byteCount": render_only_bytes,
            "iterationsMs": render_only_ms,
            "summary": summarize_iterations(render_only_ms, run["elements"]),
        },
        {
            "implementation": "ruviz",
            "scenarioId": run["scenarioId"],
            "plotKind": run["plotKind"],
            "sizeLabel": run["size"]["label"],
            "boundary": "public_api_render",
            "outputTarget": "png_bytes",
            "elements": run["elements"],
            "canvas": run["canvas"],
            "datasetHash": dataset["hash"],
            "warmupIterations": public_warmup,
            "measuredIterations": public_measured,
            "byteCount": public_bytes,
            "iterationsMs": public_ms,
            "summary": summarize_iterations(public_ms, run["elements"]),
        },
    ]


def benchmark_matplotlib(
    run: dict[str, Any],
    dataset: dict[str, Any],
    *,
    adaptive_budget_seconds: float | None,
) -> list[dict[str, Any]]:
    prepared_canvas = build_matplotlib_canvas(run, dataset)

    render_only_ms, render_only_bytes, render_only_warmup, render_only_measured = measure(
        warmup_iterations=run["warmupIterations"],
        measured_iterations=run["measuredIterations"],
        fn=lambda: canvas_png_bytes(prepared_canvas),
        adaptive_budget_seconds=adaptive_budget_seconds,
    )

    public_ms, public_bytes, public_warmup, public_measured = measure(
        warmup_iterations=run["warmupIterations"],
        measured_iterations=run["measuredIterations"],
        fn=lambda: fresh_matplotlib_png(run, dataset),
        adaptive_budget_seconds=adaptive_budget_seconds,
    )

    return [
        {
            "implementation": "matplotlib",
            "scenarioId": run["scenarioId"],
            "plotKind": run["plotKind"],
            "sizeLabel": run["size"]["label"],
            "boundary": "render_only",
            "outputTarget": "png_bytes",
            "elements": run["elements"],
            "canvas": run["canvas"],
            "datasetHash": dataset["hash"],
            "warmupIterations": render_only_warmup,
            "measuredIterations": render_only_measured,
            "byteCount": render_only_bytes,
            "iterationsMs": render_only_ms,
            "summary": summarize_iterations(render_only_ms, run["elements"]),
        },
        {
            "implementation": "matplotlib",
            "scenarioId": run["scenarioId"],
            "plotKind": run["plotKind"],
            "sizeLabel": run["size"]["label"],
            "boundary": "public_api_render",
            "outputTarget": "png_bytes",
            "elements": run["elements"],
            "canvas": run["canvas"],
            "datasetHash": dataset["hash"],
            "warmupIterations": public_warmup,
            "measuredIterations": public_measured,
            "byteCount": public_bytes,
            "iterationsMs": public_ms,
            "summary": summarize_iterations(public_ms, run["elements"]),
        },
    ]


def run_python_benchmarks(manifest_path: Path, mode: str) -> dict[str, Any]:
    manifest = load_manifest(manifest_path)
    results: list[dict[str, Any]] = []
    adaptive_budget_seconds = PYTHON_CASE_BUDGET_SECONDS if mode == "full" else None

    for run in scenario_runs(manifest, mode):
        dataset = build_dataset(run)
        results.extend(
            benchmark_ruviz(run, dataset, adaptive_budget_seconds=adaptive_budget_seconds)
        )
        results.extend(
            benchmark_matplotlib(run, dataset, adaptive_budget_seconds=adaptive_budget_seconds)
        )

    return {
        "schemaVersion": 1,
        "runtime": "python",
        "environment": {
            "pythonVersion": platform.python_version(),
            "numpyVersion": np.__version__,
            "matplotlibVersion": matplotlib.__version__,
            "ruvizVersion": getattr(ruviz, "__version__", "workspace"),
            "matplotlibBackend": matplotlib.get_backend(),
        },
        "results": results,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Run Python plotting benchmarks.")
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--mode", choices=["full", "smoke"], default="full")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    payload = run_python_benchmarks(args.manifest.resolve(), args.mode)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(payload, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
