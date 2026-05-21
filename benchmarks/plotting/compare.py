from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def load_payload(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def result_key(result: dict[str, Any]) -> tuple[str, str, str, str, str]:
    return (
        result["implementation"],
        result["scenarioId"],
        result["sizeLabel"],
        result["boundary"],
        result["outputTarget"],
    )


def index_results(payload: dict[str, Any]) -> dict[tuple[str, str, str, str, str], dict[str, Any]]:
    return {result_key(result): result for result in payload["results"]}


def median_ms(result: dict[str, Any]) -> float:
    return float(result["summary"]["medianMs"])


def format_delta(ratio: float) -> str:
    return f"{(ratio - 1.0) * 100.0:+.1f}%"


def format_key(key: tuple[str, str, str, str, str]) -> list[str]:
    return [key[0], key[1], key[2], key[3], key[4]]


def compare_results(
    baseline: dict[str, Any],
    candidate: dict[str, Any],
    *,
    regression_threshold: float,
) -> tuple[list[list[str]], list[list[str]], list[list[str]]]:
    baseline_index = index_results(baseline)
    candidate_index = index_results(candidate)
    baseline_keys = set(baseline_index)
    candidate_keys = set(candidate_index)
    rows: list[list[str]] = []
    regressions: list[list[str]] = []
    missing_candidate_rows = [
        format_key(key) for key in sorted(baseline_keys - candidate_keys)
    ]

    for key in sorted(baseline_keys & candidate_keys):
        baseline_result = baseline_index[key]
        candidate_result = candidate_index[key]
        baseline_median = median_ms(baseline_result)
        candidate_median = median_ms(candidate_result)
        ratio = candidate_median / baseline_median if baseline_median > 0.0 else 1.0
        row = [
            key[0],
            key[1],
            key[2],
            key[3],
            f"{baseline_median:.2f}",
            f"{candidate_median:.2f}",
            format_delta(ratio),
        ]
        rows.append(row)
        if ratio > 1.0 + regression_threshold:
            regressions.append(row)

    return rows, regressions, missing_candidate_rows


def markdown_table(headers: list[str], rows: list[list[str]]) -> str:
    lines = [
        "| " + " | ".join(headers) + " |",
        "| " + " | ".join(["---"] * len(headers)) + " |",
    ]
    lines.extend("| " + " | ".join(row) + " |" for row in rows)
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Compare two plotting benchmark JSON payloads."
    )
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument(
        "--regression-threshold",
        type=float,
        default=0.05,
        help="Allowed median slowdown as a fraction before failing. Default: 0.05.",
    )
    args = parser.parse_args()

    rows, regressions, missing_candidate_rows = compare_results(
        load_payload(args.baseline),
        load_payload(args.candidate),
        regression_threshold=args.regression_threshold,
    )
    headers = [
        "Implementation",
        "Scenario",
        "Size",
        "Boundary",
        "Baseline ms",
        "Candidate ms",
        "Delta",
    ]
    missing_headers = [
        "Implementation",
        "Scenario",
        "Size",
        "Boundary",
        "Output",
    ]
    report = "\n".join(
        [
            "# Plotting Benchmark Comparison",
            "",
            markdown_table(headers, rows),
            "",
            "## Regressions",
            "",
            "None."
            if not regressions
            else markdown_table(headers, regressions),
            "",
            "## Missing Candidate Rows",
            "",
            "None."
            if not missing_candidate_rows
            else markdown_table(missing_headers, missing_candidate_rows),
            "",
        ]
    )

    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(report, encoding="utf-8")
    else:
        print(report)

    if regressions or missing_candidate_rows:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
