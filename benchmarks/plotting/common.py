from __future__ import annotations

import hashlib
import json
import math
import os
import platform
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
MANIFEST_PATH = ROOT / "benchmarks" / "plotting" / "scenarios.json"


def load_manifest(path: Path | None = None) -> dict[str, Any]:
    manifest_path = path or MANIFEST_PATH
    with manifest_path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def scenario_runs(manifest: dict[str, Any], mode: str) -> list[dict[str, Any]]:
    defaults = manifest["defaults"]
    if mode == "smoke":
        warmup = defaults["smokeWarmupIterations"]
        measured = defaults["smokeMeasuredIterations"]
    else:
        warmup = defaults["warmupIterations"]
        measured = defaults["measuredIterations"]

    runs: list[dict[str, Any]] = []
    for scenario in manifest["scenarios"]:
        sizes = scenario["sizes"][:1] if mode == "smoke" else scenario["sizes"]
        for size in sizes:
            entry = {
                "scenarioId": scenario["id"],
                "plotKind": scenario["plotKind"],
                "datasetKind": scenario["datasetKind"],
                "canvas": scenario["canvas"],
                "size": size,
                "warmupIterations": warmup,
                "measuredIterations": measured,
            }
            entry["elements"] = element_count(size)
            runs.append(entry)
    return runs


def element_count(size: dict[str, Any]) -> int:
    if "points" in size:
        return int(size["points"])
    if "samples" in size:
        return int(size["samples"])
    return int(size["rows"]) * int(size["cols"])


def utc_timestamp() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def sha256_f64_arrays(arrays: list[np.ndarray]) -> str:
    digest = hashlib.sha256()
    for array in arrays:
        digest.update(np.asarray(array, dtype="<f8").tobytes(order="C"))
    return digest.hexdigest()


def triangle_wave(indices: np.ndarray, period: int) -> np.ndarray:
    phase = (indices % period).astype(np.float64) / float(period)
    return 1.0 - 4.0 * np.abs(phase - 0.5)


def line_wave(points: int) -> dict[str, Any]:
    indices = np.arange(points, dtype=np.int64)
    divisor = max(points - 1, 1)
    x = indices.astype(np.float64) * (200.0 / float(divisor))
    y = (
        triangle_wave(indices, 1024)
        + 0.35 * triangle_wave(indices, 257)
        + 0.1 * triangle_wave(indices, 61)
    ).astype(np.float64)
    return {
        "x": x,
        "y": y,
        "hash": sha256_f64_arrays([x, y]),
        "elements": points,
    }


def scatter_cloud(points: int) -> dict[str, Any]:
    indices = np.arange(points, dtype=np.int64)
    modulus = 2_147_483_647
    x_raw = (indices * 48_271) % modulus
    noise_raw = (indices * 69_621 + 12_345) % modulus
    x = x_raw.astype(np.float64) / float(modulus)
    noise = noise_raw.astype(np.float64) / float(modulus)
    band = (indices % 11).astype(np.float64) / 10.0 - 0.5
    y = np.clip(0.62 * x + 0.25 * noise + 0.13 * band, 0.0, 1.0).astype(np.float64)
    return {
        "x": x,
        "y": y,
        "hash": sha256_f64_arrays([x, y]),
        "elements": points,
    }


def histogram_signal(samples: int) -> dict[str, Any]:
    indices = np.arange(samples, dtype=np.int64)
    modulus = 2_147_483_647
    a = ((indices * 1_103_515_245 + 12_345) % modulus).astype(np.float64) / float(modulus)
    b = ((indices * 214_013 + 2_531_011) % modulus).astype(np.float64) / float(modulus)
    cluster = (indices % 17).astype(np.float64) / 16.0
    values = ((0.55 * a + 0.35 * b + 0.10 * cluster) * 10.0 - 5.0).astype(np.float64)
    return {
        "values": values,
        "hash": sha256_f64_arrays([values]),
        "elements": samples,
    }


def heatmap_field(rows: int, cols: int) -> dict[str, Any]:
    row_indices = np.arange(rows, dtype=np.int64)[:, None]
    col_indices = np.arange(cols, dtype=np.int64)[None, :]
    row_wave = triangle_wave(row_indices, 79)
    col_wave = triangle_wave(col_indices, 113)
    diagonal_wave = triangle_wave(row_indices * 3 + col_indices * 5, 47)
    values = (row_wave * col_wave + 0.2 * diagonal_wave).astype(np.float64)
    flattened = values.reshape(rows * cols)
    return {
        "matrix": values,
        "flat": flattened,
        "hash": sha256_f64_arrays([flattened]),
        "elements": rows * cols,
    }


def build_dataset(run: dict[str, Any]) -> dict[str, Any]:
    size = run["size"]
    scenario_id = run["scenarioId"]
    if scenario_id == "line":
        return line_wave(int(size["points"]))
    if scenario_id == "scatter":
        return scatter_cloud(int(size["points"]))
    if scenario_id == "histogram":
        return histogram_signal(int(size["samples"]))
    if scenario_id == "heatmap":
        return heatmap_field(int(size["rows"]), int(size["cols"]))
    raise ValueError(f"unsupported scenario: {scenario_id}")


def percentile(sorted_values: list[float], ratio: float) -> float:
    if not sorted_values:
        return 0.0
    if len(sorted_values) == 1:
        return sorted_values[0]
    position = ratio * (len(sorted_values) - 1)
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return sorted_values[lower]
    weight = position - lower
    return sorted_values[lower] * (1.0 - weight) + sorted_values[upper] * weight


def summarize_iterations(iterations_ms: list[float], elements: int) -> dict[str, float]:
    values = [float(value) for value in iterations_ms]
    sorted_values = sorted(values)
    mean = sum(values) / len(values)
    variance = sum((value - mean) ** 2 for value in values) / max(len(values) - 1, 1)
    median = percentile(sorted_values, 0.5)
    p95 = percentile(sorted_values, 0.95)
    throughput = 0.0 if median <= 0 else elements / (median / 1000.0)
    return {
        "meanMs": mean,
        "medianMs": median,
        "p95Ms": p95,
        "minMs": sorted_values[0],
        "maxMs": sorted_values[-1],
        "stdevMs": math.sqrt(variance),
        "throughputElementsPerSec": throughput,
    }


def rust_version() -> str:
    return subprocess.check_output(["rustc", "-V"], cwd=ROOT, text=True).strip()


def bun_version() -> str:
    return subprocess.check_output(["bun", "--version"], cwd=ROOT, text=True).strip()


def git_commit() -> str:
    return subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
    ).strip()


def git_branch() -> str:
    return subprocess.check_output(
        ["git", "branch", "--show-current"], cwd=ROOT, text=True
    ).strip()


def git_dirty() -> bool:
    return bool(
        subprocess.check_output(
            ["git", "status", "--short"], cwd=ROOT, text=True
        ).strip()
    )


def host_environment() -> dict[str, Any]:
    return {
        "capturedAt": utc_timestamp(),
        "os": platform.platform(),
        "machine": platform.machine(),
        "processor": platform.processor(),
        "cpuCount": os.cpu_count(),
        "pythonVersion": platform.python_version(),
        "rustVersion": rust_version(),
        "bunVersion": bun_version(),
        "gitCommit": git_commit(),
        "gitBranch": git_branch(),
        "gitDirty": git_dirty(),
    }
