from __future__ import annotations

import csv
from pathlib import Path
from typing import Any


FEATURE_ORDER = [
    "baseline_cpu",
    "default",
    "parallel_only",
    "parallel_simd",
    "performance_alias",
    "gpu_only",
]


def format_ms(value: float) -> str:
    return f"{value:.2f} ms"


def format_speedup(value: float) -> str:
    if value <= 0.0:
        return "-"
    return f"{value:.2f}x"


def _table(headers: list[str], rows: list[list[str]]) -> str:
    lines = [
        "| " + " | ".join(headers) + " |",
        "| " + " | ".join(["---"] * len(headers)) + " |",
    ]
    for row in rows:
        lines.append("| " + " | ".join(row) + " |")
    return "\n".join(lines)


def _flatten_results(runtime_payloads: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for payload in runtime_payloads:
        environment = payload["environment"]
        feature_label = environment["featureLabel"]
        cargo_features = ",".join(environment.get("cargoFeatures", []))
        for result in payload["results"]:
            row = dict(result)
            row["featureLabel"] = feature_label
            row["cargoFeatures"] = cargo_features
            rows.append(row)
    return rows


def write_feature_csv(path: Path, runtime_payloads: list[dict[str, Any]]) -> None:
    rows = _flatten_results(runtime_payloads)
    fieldnames = [
        "featureLabel",
        "cargoFeatures",
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
        "actualBackend",
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
                    "featureLabel": row["featureLabel"],
                    "cargoFeatures": row["cargoFeatures"],
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
                    "actualBackend": row.get("actualBackend", ""),
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


def _index_results(runtime_payloads: list[dict[str, Any]]) -> dict[tuple[str, str, str, str], dict[str, Any]]:
    index: dict[tuple[str, str, str, str], dict[str, Any]] = {}
    for payload in runtime_payloads:
        feature_label = payload["environment"]["featureLabel"]
        for result in payload["results"]:
            key = (
                feature_label,
                result["scenarioId"],
                result["sizeLabel"],
                result["boundary"],
            )
            index[key] = result
    return index


def _unique_cases(runtime_payloads: list[dict[str, Any]]) -> list[tuple[str, str]]:
    return sorted(
        {
            (result["scenarioId"], result["sizeLabel"])
            for payload in runtime_payloads
            for result in payload["results"]
            if result["implementation"] == "ruviz"
        }
    )


def _format_feature_cell(
    baseline: dict[str, Any] | None,
    current: dict[str, Any] | None,
    *,
    include_backend: bool,
) -> str:
    if current is None:
        return "-"
    median = current["summary"]["medianMs"]
    text = format_ms(median)
    if baseline is not None and current is not baseline:
        speedup = baseline["summary"]["medianMs"] / median if median > 0 else 0.0
        text = f"{text} ({format_speedup(speedup)})"
    if include_backend and current.get("actualBackend"):
        text = f"{text} [{current['actualBackend']}]"
    return text


def generate_feature_report(
    *,
    environment: dict[str, Any],
    runtime_payloads: list[dict[str, Any]],
    raw_link_base: str,
    report_title: str,
) -> str:
    result_index = _index_results(runtime_payloads)
    cases = _unique_cases(runtime_payloads)
    manifest = environment["manifest"]
    defaults = manifest["defaults"]
    feature_matrix = environment["featureMatrix"]

    sections: list[str] = [
        f"# {report_title}",
        "",
        "This page is generated from the committed Rust feature-impact plotting benchmark reference run.",
        "",
        "## Methodology",
        "",
        "- Scope: Rust-only feature study for the core `ruviz` crate",
        "- Output target: in-memory PNG bytes only",
        "- Dataset generation is excluded from all measured timings",
        "- File I/O is excluded from all measured timings",
        "- Boundaries:",
        "  - `render_only`: reuse a built plot object and call `render_png_bytes()`",
        "  - `public_api_render`: rebuild through the public API, then call `render_png_bytes()`",
        "  - `save_only`: reuse a built plot object and measure the same backend-selection path used by `save()`, but write PNG to memory instead of disk",
        "  - `public_api_save`: rebuild through the public API, then measure the same in-memory `save()` backend path",
        "- Feature builds benchmarked:",
        "  - `baseline_cpu`: `--no-default-features`",
        "  - `default`: default crate features",
        "  - `parallel_only`: `--no-default-features --features parallel`",
        "  - `parallel_simd`: `--no-default-features --features parallel,simd`",
        "  - `performance_alias`: `--no-default-features --features performance`",
        "  - `gpu_only`: `--no-default-features --features gpu`",
        "- Every benchmark build also enables `serde` for JSON output only",
        "- `gpu_only` requests `.gpu(true)` for the save-path boundaries only",
        "- Save-path tables include the actual backend used in brackets, so CPU fallbacks are visible",
        f"- Full-run warmup / measured iterations: `{defaults['warmupIterations']}` / `{defaults['measuredIterations']}`",
        "",
        "## Scenario Matrix",
        "",
        _table(
            ["Scenario", "Dataset", "Sizes", "Canvas"],
            [
                [
                    scenario["id"],
                    scenario["datasetKind"],
                    ", ".join(size["label"] for size in scenario["sizes"]),
                    f"{scenario['canvas']['width']}x{scenario['canvas']['height']} @ {scenario['canvas']['dpi']} DPI",
                ]
                for scenario in manifest["scenarios"]
            ],
        ),
        "",
        "## Feature Builds",
        "",
        _table(
            ["Label", "Cargo features", "GPU requested for save path"],
            [
                [
                    entry["label"],
                    ", ".join(entry["cargoFeatures"]) if entry["cargoFeatures"] else "(none)",
                    "yes" if entry["requestGpu"] else "no",
                ]
                for entry in feature_matrix
            ],
        ),
        "",
        "## Environment",
        "",
        f"- Captured at: `{environment['capturedAt']}`",
        f"- Git commit: `{environment['gitCommit']}`",
        f"- Git branch: `{environment['gitBranch']}`",
        f"- Git worktree dirty: `{'yes' if environment.get('gitDirty') else 'no'}`",
        f"- Host OS: `{environment['os']}`",
        f"- Host machine: `{environment['machine']}`",
        f"- Host processor: `{environment['processor']}`",
        f"- CPU count: `{environment['cpuCount']}`",
        f"- Python: `{environment['pythonVersion']}`",
        f"- Rust: `{environment['rustVersion']}`",
        "",
        "Raw artifacts:",
        f"- [environment.json]({raw_link_base}/environment.json)",
        f"- [results.csv]({raw_link_base}/results.csv)",
    ]

    for entry in feature_matrix:
        sections.append(f"- [{entry['label']}.json]({raw_link_base}/{entry['label']}.json)")
    sections.append("")

    for boundary in ("render_only", "public_api_render", "save_only", "public_api_save"):
        include_backend = "save" in boundary
        rows: list[list[str]] = []
        for scenario_id, size_label in cases:
            baseline = result_index.get(("baseline_cpu", scenario_id, size_label, boundary))
            if baseline is None:
                continue
            row = [scenario_id, size_label]
            for feature_label in FEATURE_ORDER:
                current = result_index.get((feature_label, scenario_id, size_label, boundary))
                row.append(
                    _format_feature_cell(
                        baseline,
                        current,
                        include_backend=include_backend,
                    )
                )
            rows.append(row)

        sections.extend(
            [
                f"## Rust feature impact (`{boundary}`)",
                "",
                _table(
                    [
                        "Plot",
                        "Size",
                        "baseline_cpu",
                        "default",
                        "parallel_only",
                        "parallel_simd",
                        "performance_alias",
                        "gpu_only",
                    ],
                    rows,
                ),
                "",
            ]
        )

    alias_rows: list[list[str]] = []
    for scenario_id, size_label in cases:
        for boundary in ("render_only", "public_api_render", "save_only", "public_api_save"):
            parallel_row = result_index.get(("parallel_simd", scenario_id, size_label, boundary))
            alias_row = result_index.get(("performance_alias", scenario_id, size_label, boundary))
            if parallel_row is None or alias_row is None:
                continue
            ratio = (
                alias_row["summary"]["medianMs"] / parallel_row["summary"]["medianMs"]
                if parallel_row["summary"]["medianMs"] > 0
                else 0.0
            )
            alias_rows.append(
                [
                    scenario_id,
                    size_label,
                    boundary,
                    format_ms(parallel_row["summary"]["medianMs"]),
                    format_ms(alias_row["summary"]["medianMs"]),
                    format_speedup(ratio),
                ]
            )

    sections.extend(
        [
            "## `parallel_simd` vs `performance_alias`",
            "",
            "These rows should remain near parity because `performance` currently aliases `parallel + simd`.",
            "",
            _table(
                [
                    "Plot",
                    "Size",
                    "Boundary",
                    "parallel_simd",
                    "performance_alias",
                    "Alias ratio",
                ],
                alias_rows,
            ),
            "",
        ]
    )

    return "\n".join(sections).rstrip() + "\n"
