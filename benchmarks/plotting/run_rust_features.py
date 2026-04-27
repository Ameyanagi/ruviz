from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

from common import MANIFEST_PATH, ROOT, host_environment, load_manifest
from rust_feature_report import generate_feature_report, write_feature_csv


FEATURE_MATRIX = [
    {
        "label": "baseline_cpu",
        "cargoFeatures": ["serde"],
        "cargoArgs": ["--no-default-features", "--features", "serde"],
        "requestGpu": False,
    },
    {
        "label": "default",
        "cargoFeatures": ["ndarray", "parallel", "serde"],
        "cargoArgs": ["--features", "serde"],
        "requestGpu": False,
    },
    {
        "label": "parallel_only",
        "cargoFeatures": ["parallel", "serde"],
        "cargoArgs": ["--no-default-features", "--features", "serde,parallel"],
        "requestGpu": False,
    },
    {
        "label": "parallel_simd",
        "cargoFeatures": ["parallel", "simd", "serde"],
        "cargoArgs": ["--no-default-features", "--features", "serde,parallel,simd"],
        "requestGpu": False,
    },
    {
        "label": "performance_alias",
        "cargoFeatures": ["performance", "serde"],
        "cargoArgs": ["--no-default-features", "--features", "serde,performance"],
        "requestGpu": False,
    },
    {
        "label": "gpu_only",
        "cargoFeatures": ["gpu", "serde"],
        "cargoArgs": ["--no-default-features", "--features", "serde,gpu"],
        "requestGpu": True,
    },
]


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
    parser = argparse.ArgumentParser(
        description="Run the Rust feature-impact plotting benchmark suite."
    )
    parser.add_argument("--mode", choices=["full", "smoke"], default="full")
    parser.add_argument("--manifest", type=Path, default=MANIFEST_PATH)
    parser.add_argument("--output-dir", type=Path, default=None)
    parser.add_argument("--docs-output", type=Path, default=None)
    parser.add_argument(
        "--reuse-existing",
        action="store_true",
        help="Reuse existing per-feature JSON payloads when present.",
    )
    args = parser.parse_args()

    manifest_path = args.manifest.resolve()
    if args.output_dir is None:
        default_output_dir = (
            ROOT / "benchmarks" / "plotting" / "results" / "rust-features" / "reference"
            if args.mode == "full"
            else ROOT / "benchmarks" / "plotting" / "results" / "rust-features" / "smoke"
        )
        output_dir = default_output_dir.resolve()
    else:
        output_dir = args.output_dir.resolve()

    if args.docs_output is None:
        default_docs_output = (
            ROOT / "docs" / "benchmarks" / "rust-feature-impact.md"
            if args.mode == "full"
            else output_dir / "report.md"
        )
        docs_output = default_docs_output.resolve()
    else:
        docs_output = args.docs_output.resolve()

    environment = host_environment()
    output_dir.mkdir(parents=True, exist_ok=True)

    runtime_payloads: list[dict[str, Any]] = []
    for entry in FEATURE_MATRIX:
        output_path = output_dir / f"{entry['label']}.json"
        if not (args.reuse_existing and output_path.exists()):
            command = [
                "cargo",
                "run",
                "--release",
                *entry["cargoArgs"],
                "--example",
                "plotting_benchmark_runner",
                "--",
                "--manifest",
                str(manifest_path),
                "--mode",
                args.mode,
                "--output",
                str(output_path),
                "--feature-label",
                entry["label"],
                "--cargo-features",
                ",".join(entry["cargoFeatures"]),
                "--include-save-path",
                "--skip-plotters",
            ]
            if entry["requestGpu"]:
                command.append("--request-gpu")
            run_command(command)
        runtime_payloads.append(load_payload(output_path))

    validate_hashes(runtime_payloads)

    environment["manifest"] = load_manifest(manifest_path)
    environment["featureMatrix"] = FEATURE_MATRIX
    environment["runtimes"] = {
        payload["environment"]["featureLabel"]: payload["environment"]
        for payload in runtime_payloads
    }

    environment_path = output_dir / "environment.json"
    environment_path.write_text(json.dumps(environment, indent=2), encoding="utf-8")

    csv_path = output_dir / "results.csv"
    write_feature_csv(csv_path, runtime_payloads)

    using_default_reference_outputs = (
        args.mode == "full" and args.output_dir is None and args.docs_output is None
    )
    docs_markdown = generate_feature_report(
        environment=environment,
        runtime_payloads=runtime_payloads,
        raw_link_base="../../benchmarks/plotting/results/rust-features/reference"
        if using_default_reference_outputs
        else ".",
        report_title="Rust Feature Impact Plotting Benchmarks",
    )
    output_report = output_dir / "report.md"
    output_report_markdown = generate_feature_report(
        environment=environment,
        runtime_payloads=runtime_payloads,
        raw_link_base=".",
        report_title="Rust Feature Impact Plotting Benchmarks",
    )
    output_report.write_text(output_report_markdown, encoding="utf-8")

    if docs_output != output_report:
        docs_output.parent.mkdir(parents=True, exist_ok=True)
        docs_output.write_text(docs_markdown, encoding="utf-8")

    print(f"Wrote Rust feature benchmark artifacts to {output_dir}")
    print(f"Wrote Rust feature benchmark report to {docs_output}")


if __name__ == "__main__":
    main()
