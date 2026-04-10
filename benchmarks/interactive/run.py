from __future__ import annotations

import argparse
import csv
import json
import platform
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
MANIFEST_PATH = ROOT / "benchmarks" / "interactive" / "scenarios.json"


def run_command(args: list[str]) -> None:
    subprocess.run(args, cwd=ROOT, check=True)


def load_payload(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def utc_timestamp() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def rust_version() -> str:
    return subprocess.check_output(["rustc", "-V"], cwd=ROOT, text=True).strip()


def bun_version() -> str:
    return subprocess.check_output(["bun", "--version"], cwd=ROOT, text=True).strip()


def git_commit() -> str:
    return subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()


def git_branch() -> str:
    return subprocess.check_output(
        ["git", "branch", "--show-current"], cwd=ROOT, text=True
    ).strip()


def git_dirty() -> bool:
    return bool(
        subprocess.check_output(["git", "status", "--short"], cwd=ROOT, text=True).strip()
    )


def host_environment(manifest: dict[str, Any]) -> dict[str, Any]:
    return {
        "capturedAt": utc_timestamp(),
        "os": platform.platform(),
        "machine": platform.machine(),
        "processor": platform.processor(),
        "cpuCount": __import__("os").cpu_count(),
        "rustVersion": rust_version(),
        "bunVersion": bun_version(),
        "gitCommit": git_commit(),
        "gitBranch": git_branch(),
        "gitDirty": git_dirty(),
        "manifest": manifest,
    }


def write_results_csv(path: Path, payloads: list[dict[str, Any]]) -> None:
    rows: list[dict[str, Any]] = []
    for payload in payloads:
        runtime = payload["runtime"]
        for result in payload["results"]:
            row = dict(result)
            row["runtime"] = runtime
            row["meanMs"] = result["summary"]["meanMs"]
            row["medianMs"] = result["summary"]["medianMs"]
            row["p95Ms"] = result["summary"]["p95Ms"]
            row["minMs"] = result["summary"]["minMs"]
            row["maxMs"] = result["summary"]["maxMs"]
            row["stdevMs"] = result["summary"]["stdevMs"]
            row["throughputElementsPerSec"] = result["summary"]["throughputElementsPerSec"]
            rows.append(row)

    fieldnames = [
        "runtime",
        "implementation",
        "scenarioId",
        "plotKind",
        "sizeLabel",
        "boundary",
        "outputTarget",
        "sessionMode",
        "elements",
        "datasetHash",
        "warmupIterations",
        "measuredIterations",
        "frameByteCount",
        "byteCount",
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
            writer.writerow({name: row.get(name) for name in fieldnames})


def main() -> None:
    parser = argparse.ArgumentParser(description="Run the interactive benchmark suite.")
    parser.add_argument("--mode", choices=["full", "smoke"], default="full")
    parser.add_argument("--manifest", type=Path, default=MANIFEST_PATH)
    parser.add_argument("--output-dir", type=Path, default=None)
    args = parser.parse_args()

    manifest_path = args.manifest.resolve()
    output_dir = (
        args.output_dir.resolve()
        if args.output_dir is not None
        else (
            ROOT / "benchmarks" / "interactive" / "results" / ("reference" if args.mode == "full" else "smoke")
        ).resolve()
    )
    output_dir.mkdir(parents=True, exist_ok=True)

    rust_output = output_dir / "rust.json"
    wasm_output = output_dir / "wasm.json"

    run_command(
        [
            "cargo",
            "run",
            "--release",
            "--features",
            "serde,animation",
            "--example",
            "interactive_benchmark_runner",
            "--",
            "--manifest",
            str(manifest_path),
            "--mode",
            args.mode,
            "--output",
            str(rust_output),
        ]
    )

    run_command(
        [
            "bun",
            "benchmarks/interactive/wasm_runner.mjs",
            "--manifest",
            str(manifest_path),
            "--mode",
            args.mode,
            "--output",
            str(wasm_output),
        ]
    )

    payloads = [load_payload(rust_output), load_payload(wasm_output)]
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    environment = host_environment(manifest)
    environment["runtimes"] = {
        payload["runtime"]: payload["environment"] for payload in payloads
    }
    (output_dir / "environment.json").write_text(
        json.dumps(environment, indent=2), encoding="utf-8"
    )
    write_results_csv(output_dir / "results.csv", payloads)

    print(f"Wrote interactive benchmark artifacts to {output_dir}")


if __name__ == "__main__":
    main()
