#!/usr/bin/env python3
"""Generate a truthful, reproducible JSON artifact from Criterion's 3D results.

The generator never promotes a local observation to a release-gate result.
Criterion measurements are preserved with their source files and empirical
percentiles; missing benchmark boundaries and hardware-sensitive gates remain
explicitly unmeasured/not-evaluated.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import sys
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_ID = "https://ruviz.dev/schemas/3d-performance-artifact-v1.json"
SCHEMA_VERSION = 1


PROFILE_DEFINITIONS = (
    {
        "id": "cpu.cold.scene_compile",
        "backend": "cpu",
        "phase": "cold",
        "boundary": "scene_compile",
        "required": True,
    },
    {
        "id": "cpu.cold.full_export",
        "backend": "cpu",
        "phase": "cold",
        "boundary": "full_export",
        "required": True,
    },
    {
        "id": "cpu.warm.unchanged_frame",
        "backend": "cpu",
        "phase": "warm",
        "boundary": "unchanged_frame",
        "required": True,
    },
    {
        "id": "cpu.update.camera",
        "backend": "cpu",
        "phase": "update",
        "boundary": "camera_only",
        "required": True,
    },
    {
        "id": "cpu.update.style",
        "backend": "cpu",
        "phase": "update",
        "boundary": "style_only",
        "required": True,
    },
    {
        "id": "cpu.update.data",
        "backend": "cpu",
        "phase": "update",
        "boundary": "data",
        "required": True,
    },
    {
        "id": "gpu.cold.adapter_pipeline",
        "backend": "gpu",
        "phase": "cold",
        "boundary": "adapter_pipeline",
        "required": True,
    },
    {
        "id": "gpu.cold.geometry_upload_first_frame",
        "backend": "gpu",
        "phase": "cold",
        "boundary": "geometry_upload_first_frame",
        "required": True,
    },
    {
        "id": "gpu.warm.unchanged_frame",
        "backend": "gpu",
        "phase": "warm",
        "boundary": "unchanged_frame",
        "required": True,
    },
    {
        "id": "gpu.update.camera",
        "backend": "gpu",
        "phase": "update",
        "boundary": "camera_only",
        "required": True,
    },
    {
        "id": "gpu.update.style",
        "backend": "gpu",
        "phase": "update",
        "boundary": "style_only",
        "required": True,
    },
    {
        "id": "gpu.update.data",
        "backend": "gpu",
        "phase": "update",
        "boundary": "data",
        "required": True,
    },
    {
        "id": "gpu.update.readback",
        "backend": "gpu",
        "phase": "update",
        "boundary": "readback",
        "required": True,
    },
)


GATE_DEFINITIONS = (
    {
        "id": "gpu.scatter-1m.camera-p95",
        "metric": "p95_ns",
        "operator": "<=",
        "threshold": 16_700_000,
        "requires": "fixed hardware, 1M scatter, 800x600, camera-only GPU frame",
    },
    {
        "id": "gpu.surface-512.camera-p95",
        "metric": "p95_ns",
        "operator": "<=",
        "threshold": 16_700_000,
        "requires": "fixed hardware, 512x512 surface, 800x600, camera-only GPU frame",
    },
    {
        "id": "gpu.scatter-10m.camera-p95",
        "metric": "p95_ns",
        "operator": "<=",
        "threshold": 33_300_000,
        "requires": "fixed hardware, 10M scatter, camera-only GPU frame",
    },
    {
        "id": "gpu.surface-1024.camera-p95",
        "metric": "p95_ns",
        "operator": "<=",
        "threshold": 33_300_000,
        "requires": "fixed hardware, 1024x1024 surface, camera-only GPU frame",
    },
    {
        "id": "cpu.scatter-100k.warm-median",
        "metric": "median_ns",
        "operator": "<=",
        "threshold": 33_000_000,
        "requires": "fixed hardware, 100K scatter, retained CPU warm frame",
    },
    {
        "id": "cpu.scatter-1m.warm-median",
        "metric": "median_ns",
        "operator": "<=",
        "threshold": 250_000_000,
        "requires": "fixed hardware, 1M scatter, retained CPU warm frame",
    },
    {
        "id": "cpu.surface-512.warm-median",
        "metric": "median_ns",
        "operator": "<=",
        "threshold": 300_000_000,
        "requires": "fixed hardware, 512x512 surface, retained CPU warm frame",
    },
    {
        "id": "cpu.surface-1024.static-median",
        "metric": "median_ns",
        "operator": "<=",
        "threshold": 1_200_000_000,
        "requires": "fixed hardware, 1024x1024 static CPU surface",
    },
    {
        "id": "warm-frame-stability",
        "metric": "p99_over_median",
        "operator": "<=",
        "threshold": 2.0,
        "requires": "fixed hardware and sufficient repeated warm-frame samples",
    },
    {
        "id": "long-orbit-host-growth",
        "metric": "host_growth_bytes",
        "operator": "<=",
        "threshold": 1_048_576,
        "requires": "allocation measurement across 10K orbit frames",
    },
    {
        "id": "glmakie-competitive-sanity",
        "metric": "ruviz_over_glmakie_warm_camera",
        "operator": "<=",
        "threshold": 1.5,
        "requires": "matched GLMakie run on the same fixed hardware",
    },
)


def command_output(args: list[str]) -> str | None:
    try:
        return subprocess.check_output(
            args, cwd=ROOT, text=True, stderr=subprocess.DEVNULL
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def git_metadata() -> dict[str, Any]:
    status = command_output(["git", "status", "--porcelain"])
    return {
        "commit": command_output(["git", "rev-parse", "HEAD"]),
        "branch": command_output(["git", "branch", "--show-current"]),
        "dirty": None if status is None else bool(status),
    }


def cpu_model() -> str | None:
    if sys.platform == "darwin":
        return command_output(["sysctl", "-n", "machdep.cpu.brand_string"])
    cpuinfo = Path("/proc/cpuinfo")
    if cpuinfo.exists():
        for line in cpuinfo.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.lower().startswith(("model name", "hardware")) and ":" in line:
                return line.split(":", 1)[1].strip()
    return platform.processor() or None


def environment_metadata() -> dict[str, Any]:
    return {
        "os": platform.system(),
        "os_release": platform.release(),
        "architecture": platform.machine(),
        "cpu": cpu_model(),
        "python": platform.python_version(),
        "rustc": command_output(["rustc", "--version", "--verbose"]),
        "cargo": command_output(["cargo", "--version"]),
    }


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as source:
        return json.load(source)


def percentile(values: list[float], fraction: float) -> float:
    """Return a linearly interpolated empirical percentile."""
    if not values:
        raise ValueError("cannot compute a percentile from no values")
    ordered = sorted(values)
    position = (len(ordered) - 1) * fraction
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def sample_summary(sample: dict[str, Any]) -> dict[str, Any]:
    iterations = sample.get("iters", [])
    aggregate_times = sample.get("times", [])
    if len(iterations) != len(aggregate_times) or not iterations:
        raise ValueError("Criterion sample must have matching non-empty iters/times")
    per_iteration = [
        float(total) / float(count)
        for count, total in zip(iterations, aggregate_times)
        if float(count) > 0
    ]
    if len(per_iteration) != len(iterations):
        raise ValueError("Criterion iteration counts must be positive")
    median = statistics.median(per_iteration)
    return {
        "unit": "ns",
        "sample_count": len(per_iteration),
        "sampling_mode": sample.get("sampling_mode"),
        "min_ns": min(per_iteration),
        "mean_ns": statistics.fmean(per_iteration),
        "median_ns": median,
        "p95_ns": percentile(per_iteration, 0.95),
        "p99_ns": percentile(per_iteration, 0.99),
        "max_ns": max(per_iteration),
        "p99_over_median": (
            percentile(per_iteration, 0.99) / median if median > 0 else None
        ),
        "per_iteration_ns": per_iteration,
    }


def parse_case_name(name: str) -> tuple[str, int, int | None, int | None] | None:
    scatter = re.fullmatch(r"scatter-(\d+)(?:-(\d+)x(\d+))?", name)
    if scatter:
        return (
            "scatter",
            int(scatter.group(1)),
            int(scatter.group(2)) if scatter.group(2) else None,
            int(scatter.group(3)) if scatter.group(3) else None,
        )
    surface = re.fullmatch(r"surface-(\d+)x(\d+)(?:-(\d+)x(\d+))?", name)
    if surface and surface.group(1) == surface.group(2):
        return (
            "surface",
            int(surface.group(1)),
            int(surface.group(3)) if surface.group(3) else None,
            int(surface.group(4)) if surface.group(4) else None,
        )
    return None


def classify_benchmark(metadata: dict[str, Any]) -> dict[str, Any] | None:
    group = metadata.get("group_id")
    function = metadata.get("function_id")
    value = metadata.get("value_str")

    if group in {"3d/cpu/scatter", "3d/cpu/surface"}:
        workload = group.rsplit("/", 1)[-1]
        try:
            size = int(value)
        except (TypeError, ValueError):
            return None
        if function == "compile":
            profile_id = "cpu.cold.scene_compile"
            viewport = None
        elif function == "render-640x480":
            profile_id = "cpu.cold.full_export"
            viewport = {"width": 640, "height": 480}
        else:
            return None
        return {
            "profile_id": profile_id,
            "workload": workload,
            "size": size,
            "viewport": viewport,
        }

    retained_groups = {
        "3d/cpu/retained-warm-frame": "cpu.warm.unchanged_frame",
        "3d/cpu/retained-camera-update": "cpu.update.camera",
        "3d/gpu/retained-warm-no-readback": "gpu.warm.unchanged_frame",
        "3d/gpu/retained-camera-no-readback": "gpu.update.camera",
        "3d/gpu/scene-upload-export": "gpu.cold.geometry_upload_first_frame",
    }
    profile_id = retained_groups.get(group)
    if profile_id is None or not isinstance(function, str):
        return None
    parsed = parse_case_name(function)
    if parsed is None:
        return None
    workload, size, width, height = parsed
    return {
        "profile_id": profile_id,
        "workload": workload,
        "size": size,
        "viewport": (
            {"width": width, "height": height}
            if width is not None and height is not None
            else None
        ),
    }


def dataset_key(workload: str, size: int) -> tuple[str, int]:
    return (workload, size)


def load_dataset_manifest(path: Path) -> tuple[dict[tuple[str, int], dict[str, Any]], dict[str, Any]]:
    manifest = read_json(path)
    datasets: dict[tuple[str, int], dict[str, Any]] = {}
    for row in manifest.get("datasets", []):
        kind = row.get("kind")
        size = row.get("elements") if kind == "scatter" else row.get("rows")
        if isinstance(kind, str) and isinstance(size, int):
            datasets[dataset_key(kind, size)] = row
    return datasets, {
        "path": str(path.resolve().relative_to(ROOT))
        if path.resolve().is_relative_to(ROOT)
        else str(path.resolve()),
        "sha256": sha256_file(path),
        "schema_version": manifest.get("schema_version"),
        "hash_algorithm": manifest.get("hash"),
    }


def relative_or_absolute(path: Path) -> str:
    resolved = path.resolve()
    try:
        return str(resolved.relative_to(ROOT))
    except ValueError:
        return str(resolved)


def collect_measurements(
    criterion_dir: Path,
    datasets: dict[tuple[str, int], dict[str, Any]],
    accepted_files: set[Path] | None = None,
) -> tuple[list[dict[str, Any]], list[dict[str, str]]]:
    measurements: list[dict[str, Any]] = []
    errors: list[dict[str, str]] = []
    for benchmark_path in sorted(criterion_dir.glob("**/new/benchmark.json")):
        if accepted_files is not None and benchmark_path.resolve() not in accepted_files:
            continue
        estimates_path = benchmark_path.with_name("estimates.json")
        sample_path = benchmark_path.with_name("sample.json")
        if not estimates_path.exists() or not sample_path.exists():
            continue
        try:
            benchmark = read_json(benchmark_path)
            classification = classify_benchmark(benchmark)
            if classification is None:
                continue
            estimates = read_json(estimates_path)
            samples = sample_summary(read_json(sample_path))
            dataset = datasets.get(
                dataset_key(classification["workload"], classification["size"])
            )
            throughput = benchmark.get("throughput")
            elements = (
                throughput.get("Elements")
                if isinstance(throughput, dict)
                else None
            )
            median_ns = samples["median_ns"]
            measurement_id = (
                f'{classification["profile_id"]}:'
                f'{classification["workload"]}-{classification["size"]}'
            )
            measurements.append(
                {
                    "id": measurement_id,
                    **classification,
                    "dataset": {
                        "id": dataset.get("id") if dataset else None,
                        "hash": dataset.get("hash") if dataset else None,
                        "identity_status": "verified" if dataset else "unverified",
                    },
                    "criterion": {
                        "full_id": benchmark.get("full_id"),
                        "throughput": throughput,
                        "mean_estimate_ns": estimates.get("mean", {}).get(
                            "point_estimate"
                        ),
                        "median_estimate_ns": estimates.get("median", {}).get(
                            "point_estimate"
                        ),
                        "mean_confidence_interval_ns": estimates.get("mean", {}).get(
                            "confidence_interval"
                        ),
                        "median_confidence_interval_ns": estimates.get(
                            "median", {}
                        ).get("confidence_interval"),
                    },
                    "samples": samples,
                    "derived": {
                        "frames_per_second_at_median": (
                            1_000_000_000.0 / median_ns if median_ns > 0 else None
                        ),
                        "elements_per_second_at_median": (
                            float(elements) * 1_000_000_000.0 / median_ns
                            if isinstance(elements, (int, float)) and median_ns > 0
                            else None
                        ),
                    },
                    "diagnostics": {
                        "status": "unmeasured",
                        "reason": (
                            "Criterion artifacts contain timing samples but no "
                            "RenderDiagnostics3D sidecar"
                        ),
                    },
                    "source": {
                        "benchmark": relative_or_absolute(benchmark_path),
                        "sample": relative_or_absolute(sample_path),
                        "estimates": relative_or_absolute(estimates_path),
                        "benchmark_sha256": sha256_file(benchmark_path),
                        "sample_sha256": sha256_file(sample_path),
                        "estimates_sha256": sha256_file(estimates_path),
                    },
                }
            )
        except (OSError, ValueError, TypeError, json.JSONDecodeError) as error:
            errors.append(
                {
                    "path": relative_or_absolute(benchmark_path),
                    "error": str(error),
                }
            )
    return measurements, errors


def build_profiles(measurements: list[dict[str, Any]]) -> list[dict[str, Any]]:
    by_profile: dict[str, list[str]] = {}
    for measurement in measurements:
        by_profile.setdefault(measurement["profile_id"], []).append(measurement["id"])
    profiles = []
    for definition in PROFILE_DEFINITIONS:
        evidence = sorted(by_profile.get(definition["id"], []))
        profile = dict(definition)
        profile["status"] = "measured" if evidence else "unmeasured"
        profile["measurement_ids"] = evidence
        if not evidence:
            profile["reason"] = (
                "No matching Criterion result was collected for this boundary"
            )
        if definition["id"] == "gpu.cold.adapter_pipeline":
            profile["reason"] = (
                "The current GPU export benchmark reuses the process device and "
                "pipelines, so it is not cold-adapter evidence"
            )
        profiles.append(profile)
    return profiles


def build_gates(measurements: list[dict[str, Any]]) -> list[dict[str, Any]]:
    measurement_ids = [measurement["id"] for measurement in measurements]
    return [
        {
            **definition,
            "status": "not_evaluated",
            "reason": (
                "Performance gates require a declared fixed-hardware run; this "
                "artifact records observations only"
            ),
            "available_measurement_ids": measurement_ids,
        }
        for definition in GATE_DEFINITIONS
    ]


def snapshot_benchmark_files(criterion_dir: Path) -> dict[Path, tuple[int, int]]:
    return {
        path.resolve(): (path.stat().st_mtime_ns, path.stat().st_size)
        for path in criterion_dir.glob("**/new/benchmark.json")
    }


def changed_benchmark_files(
    criterion_dir: Path, before: dict[Path, tuple[int, int]]
) -> set[Path]:
    changed = set()
    for path in criterion_dir.glob("**/new/benchmark.json"):
        resolved = path.resolve()
        state = (path.stat().st_mtime_ns, path.stat().st_size)
        if before.get(resolved) != state:
            changed.add(resolved)
    return changed


def run_benchmarks(
    features: str,
    full: bool,
    criterion_dir: Path,
    bench_filter: str | None,
) -> dict[str, Any]:
    before = snapshot_benchmark_files(criterion_dir)
    command = [
        "cargo",
        "bench",
        "--bench",
        "three_d",
        "--features",
        features,
    ]
    if bench_filter:
        command.extend(["--", bench_filter])
    environment = os.environ.copy()
    if full:
        environment["RUVIZ_3D_BENCH_FULL"] = "1"
    else:
        environment.pop("RUVIZ_3D_BENCH_FULL", None)
    started_at = dt.datetime.now(dt.timezone.utc)
    result = subprocess.run(command, cwd=ROOT, env=environment, check=False)
    finished_at = dt.datetime.now(dt.timezone.utc)
    return {
        "command": command,
        "features": [feature.strip() for feature in features.split(",") if feature.strip()],
        "profile": "full" if full else "quick",
        "started_at": started_at.isoformat(),
        "finished_at": finished_at.isoformat(),
        "exit_code": result.returncode,
        "changed_files": changed_benchmark_files(criterion_dir, before),
    }


def build_artifact(
    criterion_dir: Path,
    manifest_path: Path,
    run: dict[str, Any] | None,
) -> dict[str, Any]:
    datasets, manifest_metadata = load_dataset_manifest(manifest_path)
    accepted = run["changed_files"] if run is not None else None
    measurements, collection_errors = collect_measurements(
        criterion_dir, datasets, accepted
    )
    profiles = build_profiles(measurements)
    measured_count = sum(profile["status"] == "measured" for profile in profiles)
    return {
        "$schema": SCHEMA_ID,
        "schema_version": SCHEMA_VERSION,
        "generated_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "provenance": {
            "generator": relative_or_absolute(Path(__file__)),
            "source_mode": "current_run" if run is not None else "existing_artifacts",
            "source_revision_status": (
                "current_run" if run is not None and run["exit_code"] == 0 else "unverified"
            ),
            "criterion_directory": relative_or_absolute(criterion_dir),
            "dataset_manifest": manifest_metadata,
            "git": git_metadata(),
            "environment": environment_metadata(),
            "run": (
                {
                    key: value
                    for key, value in run.items()
                    if key != "changed_files"
                }
                if run is not None
                else None
            ),
        },
        "methodology": {
            "sample_percentiles": (
                "linear interpolation over Criterion aggregate time divided by "
                "iteration count for each sample"
            ),
            "timing_unit": "nanoseconds",
            "viewport_note": (
                "viewport is parsed from benchmark IDs; null means the boundary "
                "does not render a frame"
            ),
            "diagnostics_note": (
                "timing-only Criterion files do not verify backend/upload/readback "
                "invariants; those fields remain unmeasured"
            ),
            "claims_policy": (
                "local observations are never treated as fixed-hardware gate results"
            ),
        },
        "coverage": {
            "required_profile_count": len(profiles),
            "measured_profile_count": measured_count,
            "unmeasured_profile_count": len(profiles) - measured_count,
            "measurement_count": len(measurements),
        },
        "profiles": profiles,
        "measurements": measurements,
        "gates": build_gates(measurements),
        "collection_errors": collection_errors,
    }


def parse_args(argv: Iterable[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--criterion-dir",
        type=Path,
        default=ROOT / "target" / "criterion",
        help="Criterion output directory (default: target/criterion)",
    )
    parser.add_argument(
        "--dataset-manifest",
        type=Path,
        default=ROOT / "docs" / "benchmarks" / "ruviz-3d-datasets.json",
        help="committed deterministic dataset manifest",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="write JSON to this path; stdout when omitted",
    )
    parser.add_argument(
        "--run",
        action="store_true",
        help="run the Criterion bench first and include only files changed by this run",
    )
    parser.add_argument(
        "--features",
        default="3d,gpu",
        help="Cargo features used with --run (default: 3d,gpu)",
    )
    parser.add_argument(
        "--bench-filter",
        help="optional Criterion name filter used with --run",
    )
    parser.add_argument(
        "--full",
        action="store_true",
        help="set RUVIZ_3D_BENCH_FULL=1 with --run",
    )
    parser.add_argument(
        "--strict-coverage",
        action="store_true",
        help="exit 2 when any required profile is unmeasured",
    )
    return parser.parse_args(argv)


def main(argv: Iterable[str] | None = None) -> int:
    args = parse_args(argv)
    run = (
        run_benchmarks(
            args.features,
            args.full,
            args.criterion_dir,
            args.bench_filter,
        )
        if args.run
        else None
    )
    artifact = build_artifact(args.criterion_dir, args.dataset_manifest, run)
    rendered = json.dumps(artifact, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    else:
        sys.stdout.write(rendered)

    if run is not None and run["exit_code"] != 0:
        return int(run["exit_code"])
    if args.strict_coverage and artifact["coverage"]["unmeasured_profile_count"]:
        return 2
    if artifact["collection_errors"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
