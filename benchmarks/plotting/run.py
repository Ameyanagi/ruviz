from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

from common import (
    MANIFEST_PATH,
    ROOT,
    host_environment,
    load_manifest,
)
from python_runner import run_python_benchmarks
from report import generate_markdown_report, write_consolidated_csv


def run_command(args: list[str]) -> None:
    subprocess.run(args, cwd=ROOT, check=True)


def load_payload(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def validate_hashes(runtime_payloads: list[dict[str, Any]]) -> None:
    expected: dict[tuple[str, str, str], str] = {}
    for payload in runtime_payloads:
        for result in payload["results"]:
            if result["implementation"] != "ruviz":
                continue
            key = (result["scenarioId"], result["sizeLabel"], result["boundary"])
            expected_hash = expected.setdefault(key, result["datasetHash"])
            if expected_hash != result["datasetHash"]:
                raise RuntimeError(
                    f"dataset hash mismatch for {key}: {expected_hash} != {result['datasetHash']}"
                )


def main() -> None:
    parser = argparse.ArgumentParser(description="Run the cross-runtime plotting benchmark suite.")
    parser.add_argument("--mode", choices=["full", "smoke"], default="full")
    parser.add_argument(
        "--manifest",
        type=Path,
        default=MANIFEST_PATH,
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=None,
    )
    parser.add_argument(
        "--docs-output",
        type=Path,
        default=None,
    )
    parser.add_argument(
        "--reuse-existing",
        action="store_true",
        help="Reuse existing runtime JSON outputs in the output directory when present.",
    )
    args = parser.parse_args()

    manifest_path = args.manifest.resolve()
    if args.output_dir is None:
        default_output_dir = (
            ROOT / "benchmarks" / "plotting" / "results" / "reference"
            if args.mode == "full"
            else ROOT / "benchmarks" / "plotting" / "results" / "smoke"
        )
        output_dir = default_output_dir.resolve()
    else:
        output_dir = args.output_dir.resolve()

    if args.docs_output is None:
        default_docs_output = (
            ROOT / "docs" / "benchmarks" / "large-dataset-plotting.md"
            if args.mode == "full"
            else output_dir / "report.md"
        )
        docs_output = default_docs_output.resolve()
    else:
        docs_output = args.docs_output.resolve()

    environment = host_environment()
    output_dir.mkdir(parents=True, exist_ok=True)

    rust_output = output_dir / "rust.json"
    python_output = output_dir / "python.json"
    wasm_output = output_dir / "wasm.json"

    if not (args.reuse_existing and rust_output.exists()):
        run_command(
            [
                "cargo",
                "run",
                "--release",
                "--features",
                "serde",
                "--example",
                "plotting_benchmark_runner",
                "--",
                "--manifest",
                str(manifest_path),
                "--mode",
                args.mode,
                "--output",
                str(rust_output),
            ]
        )

    if args.reuse_existing and python_output.exists():
        python_payload = load_payload(python_output)
    else:
        python_payload = run_python_benchmarks(manifest_path, args.mode)
        python_output.write_text(json.dumps(python_payload, indent=2), encoding="utf-8")

    if not (args.reuse_existing and wasm_output.exists()):
        run_command(
            [
                "bun",
                "benchmarks/plotting/wasm_runner.mjs",
                "--manifest",
                str(manifest_path),
                "--mode",
                args.mode,
                "--output",
                str(wasm_output),
            ]
        )

    runtime_payloads = [
        load_payload(rust_output),
        load_payload(python_output),
        load_payload(wasm_output),
    ]
    validate_hashes(runtime_payloads)

    environment["manifest"] = load_manifest(manifest_path)
    environment["runtimes"] = {
        payload["runtime"]: payload["environment"] for payload in runtime_payloads
    }
    environment_path = output_dir / "environment.json"
    environment_path.write_text(json.dumps(environment, indent=2), encoding="utf-8")

    csv_path = output_dir / "results.csv"
    write_consolidated_csv(csv_path, runtime_payloads)

    docs_markdown = generate_markdown_report(
        environment=environment,
        runtime_payloads=runtime_payloads,
        raw_link_base="../../benchmarks/plotting/results/reference"
        if output_dir.name == "reference"
        else ".",
        report_title="Large Dataset Plotting Benchmarks",
    )
    output_report = output_dir / "report.md"
    output_report_markdown = generate_markdown_report(
        environment=environment,
        runtime_payloads=runtime_payloads,
        raw_link_base=".",
        report_title="Large Dataset Plotting Benchmarks",
    )

    output_report.write_text(output_report_markdown, encoding="utf-8")

    if docs_output != output_report:
        docs_output.parent.mkdir(parents=True, exist_ok=True)
        docs_output.write_text(docs_markdown, encoding="utf-8")

    print(f"Wrote benchmark artifacts to {output_dir}")
    print(f"Wrote benchmark report to {docs_output}")


if __name__ == "__main__":
    main()
