from __future__ import annotations

import csv
from pathlib import Path
from typing import Any


def format_ms(value: float) -> str:
    return f"{value:.2f}"


def format_speedup(value: float) -> str:
    if 0.0 < value < 0.01:
        return "<0.01x"
    return f"{value:.2f}x"


def format_throughput(value: float) -> str:
    return f"{value / 1_000_000.0:.2f} M/s"


def flatten_results(runtime_payloads: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for payload in runtime_payloads:
        runtime = payload["runtime"]
        for result in payload["results"]:
            row = dict(result)
            row["runtime"] = runtime
            rows.append(row)
    return rows


def write_consolidated_csv(path: Path, runtime_payloads: list[dict[str, Any]]) -> None:
    rows = flatten_results(runtime_payloads)
    fieldnames = [
        "runtime",
        "implementation",
        "scenarioId",
        "plotKind",
        "sizeLabel",
        "boundary",
        "outputTarget",
        "elements",
        "width",
        "height",
        "dpi",
        "byteCount",
        "datasetHash",
        "warmupIterations",
        "measuredIterations",
        "meanMs",
        "medianMs",
        "p95Ms",
        "minMs",
        "maxMs",
        "stdevMs",
        "throughputElementsPerSec",
    ]
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames, lineterminator="\n")
        writer.writeheader()
        for row in rows:
            writer.writerow(
                {
                    "runtime": row["runtime"],
                    "implementation": row["implementation"],
                    "scenarioId": row["scenarioId"],
                    "plotKind": row["plotKind"],
                    "sizeLabel": row["sizeLabel"],
                    "boundary": row["boundary"],
                    "outputTarget": row["outputTarget"],
                    "elements": row["elements"],
                    "width": row["canvas"]["width"],
                    "height": row["canvas"]["height"],
                    "dpi": row["canvas"]["dpi"],
                    "byteCount": row["byteCount"],
                    "datasetHash": row["datasetHash"],
                    "warmupIterations": row["warmupIterations"],
                    "measuredIterations": row["measuredIterations"],
                    "meanMs": row["summary"]["meanMs"],
                    "medianMs": row["summary"]["medianMs"],
                    "p95Ms": row["summary"]["p95Ms"],
                    "minMs": row["summary"]["minMs"],
                    "maxMs": row["summary"]["maxMs"],
                    "stdevMs": row["summary"]["stdevMs"],
                    "throughputElementsPerSec": row["summary"]["throughputElementsPerSec"],
                }
            )


def _result_index(
    runtime_payloads: list[dict[str, Any]], *, implementation: str | None = None
) -> dict[tuple[str, str, str, str], dict[str, Any]]:
    index: dict[tuple[str, str, str, str], dict[str, Any]] = {}
    for payload in runtime_payloads:
        runtime = payload["runtime"]
        for result in payload["results"]:
            if implementation and result["implementation"] != implementation:
                continue
            key = (runtime, result["scenarioId"], result["sizeLabel"], result["boundary"])
            index[key] = result
    return index


def _unique_cases(runtime_payloads: list[dict[str, Any]]) -> list[tuple[str, str]]:
    cases = {
        (result["scenarioId"], result["sizeLabel"])
        for payload in runtime_payloads
        for result in payload["results"]
    }
    return sorted(cases)


def _table(headers: list[str], rows: list[list[str]]) -> str:
    lines = [
        "| " + " | ".join(headers) + " |",
        "| " + " | ".join(["---"] * len(headers)) + " |",
    ]
    for row in rows:
        lines.append("| " + " | ".join(row) + " |")
    return "\n".join(lines)


def generate_markdown_report(
    *,
    environment: dict[str, Any],
    runtime_payloads: list[dict[str, Any]],
    raw_link_base: str,
    report_title: str,
) -> str:
    python_ruviz_index = _result_index(runtime_payloads, implementation="ruviz")
    python_matplotlib_index = _result_index(runtime_payloads, implementation="matplotlib")
    ruviz_index = _result_index(runtime_payloads, implementation="ruviz")
    plotters_index = _result_index(runtime_payloads, implementation="plotters")
    cases = _unique_cases(runtime_payloads)
    manifest = environment["manifest"]
    defaults = manifest["defaults"]
    scenario_rows = [
        [
            scenario["id"],
            scenario["datasetKind"],
            ", ".join(size["label"] for size in scenario["sizes"]),
            f"{scenario['canvas']['width']}x{scenario['canvas']['height']} @ {scenario['canvas']['dpi']} DPI",
        ]
        for scenario in manifest["scenarios"]
    ]

    sections: list[str] = [
        f"# {report_title}",
        "",
        "This page is generated from the committed large-dataset plotting benchmark reference run.",
        "",
        "## Methodology",
        "",
        "- Output target: in-memory PNG byte generation only",
        "- Dataset generation is excluded from all measured timings",
        "- File I/O is excluded from all measured timings",
        "- Boundaries:",
        "  - `render_only`: reuse prepared plot state and measure the uncached render/export call without plot reconstruction",
        "  - `public_api_render`: reuse the input data, rebuild through the normal public API, then render/export",
        "- Plot matrix: `line`, `scatter`, `histogram`, `heatmap`",
        "- Python comparison target: `matplotlib` with the `Agg` backend",
        "- Python `ruviz` benchmark runs use a release-built `maturin develop --release` extension",
        "- Rust comparison target: `plotters` on the `public_api_render` boundary only",
        "- `ruviz` PNG exports now use automatic raster fast paths for eligible cases:",
        "  - large monotonic solid lines without markers/error bars are reduced to a per-column envelope before stroking",
        "  - static histograms reuse prepared bins instead of re-binning on every render-only export",
        "  - nearest, non-annotated heatmaps rasterize directly to the output surface before PNG encoding",
        "- Python host-side `ruviz` rendering now uses a persistent native plot handle and prepared plot instead of rebuilding a Rust plot from JSON on every render",
        "- Python `render_only` timings bypass the prepared-frame image cache so they measure rasterization rather than cached PNG encoding",
        "- Notebook widgets still ship JSON-friendly snapshots to the browser, but that snapshot path is no longer the default host render/export path",
        "- Python full-mode runs cap very slow cases to a 60s per-case budget; the recorded warmup/measured counts reflect the effective counts used",
        "- Rust `plotters` histogram timings reuse pre-binned bars, and the `plotters` heatmap path rasterizes the shared matrix to the fixed output canvas before PNG encoding",
        "- wasm target: Chromium-only browser benchmark via Playwright",
        f"- Full-run warmup / measured iterations: `{defaults['warmupIterations']}` / `{defaults['measuredIterations']}`",
        "",
        "## Why It Got Faster",
        "",
        "The main change in this benchmark update is not a different benchmark harness. It is a different raster renderer path for large PNG exports.",
        "",
        "What changed in `ruviz`:",
        "",
        "- Large monotonic solid line series are reduced to a per-pixel-column envelope before stroking, so a `1M` point line on a `640px` canvas no longer pays to stroke every original segment.",
        "- Static histograms now cache computed `HistogramData`, so `render_only` exports reuse prepared bins instead of re-running histogram binning on every frame.",
        "- Nearest-neighbor, non-annotated heatmaps now render the final output surface directly and blit that image, instead of drawing one anti-aliased rectangle per source cell.",
        "- The parallel line backend now emits a single polyline draw instead of thousands of two-point draw calls.",
        "- Python host rendering now keeps a native Rust plot/prepared-plot handle alive across calls, so `render_png()`, `render_svg()`, `save()`, and `show()` no longer pay a Python JSON serialization + Rust JSON parse + plot reconstruction round-trip on every call.",
        "",
        "Why those changes matter:",
        "",
        "- The old line path scaled with source vertex count even when many samples collapsed onto the same output column.",
        "- The old histogram path repeated statistical preprocessing inside hot render loops.",
        "- The old heatmap path scaled with source cell count rather than output pixel count for raster exports.",
        "- The old Python binding path spent a large share of its time turning Python state into JSON and then rebuilding a fresh Rust `Plot` before rendering anything.",
        "",
        "The result is that current Rust PNG export timings mostly reflect output-resolution work for eligible raster cases, and current Python host-side timings reflect the renderer instead of the snapshot bridge much more closely than before.",
        "",
        "## Scenario Matrix",
        "",
        _table(["Scenario", "Dataset", "Sizes", "Canvas"], scenario_rows),
        "",
        "## Environment",
        "",
        f"- Captured at: `{environment['capturedAt']}`",
        f"- Git commit: `{environment['gitCommit']}`",
        f"- Git branch: `{environment['gitBranch']}`",
        f"- Host OS: `{environment['os']}`",
        f"- Host machine: `{environment['machine']}`",
        f"- Host processor: `{environment['processor']}`",
        f"- CPU count: `{environment['cpuCount']}`",
        f"- Python: `{environment['pythonVersion']}`",
        f"- Rust: `{environment['rustVersion']}`",
        f"- Bun: `{environment['bunVersion']}`",
        f"- Chromium: `{environment['runtimes']['wasm'].get('browserVersion', 'unknown')}`",
        "",
        "Raw artifacts:",
        f"- [environment.json]({raw_link_base}/environment.json)",
        f"- [results.csv]({raw_link_base}/results.csv)",
        f"- [python.json]({raw_link_base}/python.json)",
        f"- [rust.json]({raw_link_base}/rust.json)",
        f"- [wasm.json]({raw_link_base}/wasm.json)",
        "",
    ]

    for boundary in ("render_only", "public_api_render"):
        rows: list[list[str]] = []
        for scenario_id, size_label in cases:
            ruviz = python_ruviz_index[("python", scenario_id, size_label, boundary)]
            matplotlib = python_matplotlib_index[("python", scenario_id, size_label, boundary)]
            speedup = matplotlib["summary"]["medianMs"] / ruviz["summary"]["medianMs"]
            rows.append(
                [
                    scenario_id,
                    size_label,
                    format_ms(ruviz["summary"]["medianMs"]),
                    format_ms(matplotlib["summary"]["medianMs"]),
                    format_speedup(speedup),
                ]
            )

        sections.extend(
            [
                f"## Python: ruviz vs matplotlib (`{boundary}`)",
                "",
                _table(
                    ["Plot", "Size", "ruviz median", "matplotlib median", "Speedup"],
                    rows,
                ),
                "",
            ]
        )

    for boundary in ("render_only", "public_api_render"):
        rows = []
        for scenario_id, size_label in cases:
            python_row = ruviz_index[("python", scenario_id, size_label, boundary)]
            rust_row = ruviz_index[("rust", scenario_id, size_label, boundary)]
            wasm_row = ruviz_index[("wasm", scenario_id, size_label, boundary)]
            rows.append(
                [
                    scenario_id,
                    size_label,
                    format_ms(python_row["summary"]["medianMs"]),
                    format_ms(rust_row["summary"]["medianMs"]),
                    format_ms(wasm_row["summary"]["medianMs"]),
                    python_row["datasetHash"][:12],
                ]
            )

        sections.extend(
            [
                f"## ruviz cross-runtime medians (`{boundary}`)",
                "",
                _table(
                    ["Plot", "Size", "Python", "Rust", "Wasm", "Dataset hash"],
                    rows,
                ),
                "",
            ]
        )

    plotters_rows = []
    for scenario_id, size_label in cases:
        key = ("rust", scenario_id, size_label, "public_api_render")
        if key not in plotters_index:
            continue
        ruviz_row = ruviz_index[key]
        plotters_row = plotters_index[key]
        speedup = plotters_row["summary"]["medianMs"] / ruviz_row["summary"]["medianMs"]
        plotters_rows.append(
            [
                scenario_id,
                size_label,
                format_ms(ruviz_row["summary"]["medianMs"]),
                format_ms(plotters_row["summary"]["medianMs"]),
                format_speedup(speedup),
            ]
        )

    sections.extend(
        [
            "## Rust: ruviz vs plotters (`public_api_render`)",
            "",
            _table(
                ["Plot", "Size", "ruviz median", "plotters median", "Speedup"],
                plotters_rows,
            ),
            "",
        ]
    )

    throughput_rows = []
    for scenario_id, size_label in cases:
        rust_row = ruviz_index[("rust", scenario_id, size_label, "render_only")]
        throughput_rows.append(
            [
                scenario_id,
                size_label,
                format_throughput(rust_row["summary"]["throughputElementsPerSec"]),
            ]
        )

    sections.extend(
        [
            "## Rust render-only throughput",
            "",
            _table(["Plot", "Size", "Throughput"], throughput_rows),
            "",
            "## Notes",
            "",
            "- These numbers are a reference snapshot from one machine and should be treated as comparative, not universal.",
            "- Browser wasm timings include browser-side PNG generation, but not any disk writes or download flows.",
            "- Python `render_only` for `ruviz` uses the internal `_native.render_png_bytes(snapshot_json)` path, so JSON parsing is included there even though plot construction is excluded.",
            "- The remaining `plotters` gap on histogram and heatmap is partly semantic: `plotters` benchmarks pre-binned histogram bars and output-raster heatmap generation, while `ruviz` still includes its own plot-model setup and colorbar semantics on the public API path.",
        ]
    )

    return "\n".join(sections) + "\n"
