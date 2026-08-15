#!/usr/bin/env python3
"""Static PNG scatter benchmark: ruviz (exact and fast) vs xy vs matplotlib.

Faithful port of reflex-dev/xy benchmarks/_launch_static.py: same seeded
correlated Gaussian float32 data, same 900x420 validated non-blank PNG target,
same fresh-process-per-run isolation with peak-RSS and timeout guards, same
render_ms metric (imports and data generation excluded). Each library's child
runs in its own venv interpreter.
"""

from __future__ import annotations

import argparse
import contextlib
import importlib.metadata
import io
import json
import os
import platform
import statistics
import subprocess
import sys
import tempfile
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

WIDTH = 900
HEIGHT = 420
DPI = 100
SEED = 20260713

# Interpreters per arm: ruviz arms default to this interpreter; the xy and
# matplotlib arms need a venv with xy/matplotlib installed (XY_PYTHON).
RUVIZ_PYTHON = os.environ.get("RUVIZ_PYTHON", sys.executable)
XY_PYTHON = os.environ.get("XY_PYTHON", sys.executable)
PYTHONS = {
    "xy": XY_PYTHON,
    "matplotlib": XY_PYTHON,
    "ruviz": RUVIZ_PYTHON,
    "ruviz-fast": RUVIZ_PYTHON,
}


def make_data(n: int):
    import numpy as np

    rng = np.random.default_rng(SEED)
    x = rng.standard_normal(n, dtype=np.float32)
    y = rng.standard_normal(n, dtype=np.float32)
    y *= np.float32(1.2)
    y += x
    y *= np.float32(0.5)
    return x, y


def nonblank_png(png: bytes) -> int:
    import numpy as np
    from PIL import Image

    image = np.asarray(Image.open(io.BytesIO(png)).convert("RGB"))
    if image.shape[:2] != (HEIGHT, WIDTH):
        raise AssertionError(f"unexpected PNG dimensions {image.shape[:2]}")
    count = int(np.count_nonzero(np.any(image != image[0, 0], axis=2)))
    if count == 0:
        raise AssertionError("blank PNG")
    return count


def child_run(library: str, n: int) -> dict[str, Any]:
    # Imports are excluded from chart-to-PNG time, matching the XY harness.
    if library == "xy":
        from xy import Engine, scatter, scatter_chart
    elif library == "matplotlib":
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    elif library in ("ruviz", "ruviz-fast"):
        import ruviz
    else:
        raise ValueError(library)

    x, y = make_data(n)
    t0 = time.perf_counter()
    if library == "xy":
        fig = scatter_chart(scatter(x=x, y=y), width=WIDTH, height=HEIGHT).figure()
        png = fig.to_png(width=WIDTH, height=HEIGHT, scale=1, engine=Engine.default)
        mode = "density" if fig.traces[0].use_density() else "direct"
    elif library == "ruviz":
        png = ruviz.plot().scatter(x, y).size_px(WIDTH, HEIGHT).render_png()
        mode = "native-png"
    elif library == "ruviz-fast":
        png = ruviz.plot().fast().scatter(x, y).size_px(WIDTH, HEIGHT).render_png()
        mode = "fast"
    else:
        fig, ax = plt.subplots(figsize=(WIDTH / DPI, HEIGHT / DPI), dpi=DPI)
        collection = ax.scatter(x, y)
        if len(collection.get_offsets()) != n:
            raise AssertionError("Matplotlib PathCollection row-count oracle failed")
        output = io.BytesIO()
        fig.savefig(output, format="png", dpi=DPI)
        plt.close(fig)
        png = output.getvalue()
        mode = "static-agg"
    elapsed_ms = (time.perf_counter() - t0) * 1e3
    nonblank = nonblank_png(png)
    return {
        "status": "ok",
        "library": library,
        "n": n,
        "mode": mode,
        "source_bytes": int(x.nbytes + y.nbytes),
        "render_ms": elapsed_ms,
        "png_bytes": len(png),
        "nonblank_pixels": nonblank,
        "width": WIDTH,
        "height": HEIGHT,
    }


def tree_rss(process) -> int:
    import psutil

    total = 0
    try:
        processes = [process, *process.children(recursive=True)]
    except psutil.NoSuchProcess:
        return 0
    for proc in processes:
        with contextlib.suppress(psutil.NoSuchProcess, psutil.AccessDenied):
            total += proc.memory_info().rss
    return total


def run_isolated(library: str, n: int, *, timeout_s: float, memory_limit_bytes: int) -> dict[str, Any]:
    import psutil

    with tempfile.TemporaryDirectory() as td:
        result_path = Path(td) / "result.json"
        command = [
            PYTHONS[library],
            str(Path(__file__).resolve()),
            "--child",
            "--library",
            library,
            "--n",
            str(n),
            "--child-out",
            str(result_path),
        ]
        popen = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        proc = psutil.Process(popen.pid)
        start = time.monotonic()
        peak = 0
        terminal_status = None
        while popen.poll() is None:
            rss = tree_rss(proc)
            peak = max(peak, rss)
            if rss > memory_limit_bytes:
                terminal_status = "memory_limit"
                with contextlib.suppress(psutil.NoSuchProcess):
                    proc.kill()
                break
            if time.monotonic() - start > timeout_s:
                terminal_status = "timeout"
                with contextlib.suppress(psutil.NoSuchProcess):
                    proc.kill()
                break
            time.sleep(0.05)
        stdout, stderr = popen.communicate()
        if terminal_status:
            return {
                "status": terminal_status,
                "library": library,
                "n": n,
                "peak_rss_bytes": peak,
                "wall_ms": (time.monotonic() - start) * 1e3,
                "stderr_tail": stderr[-1000:],
            }
        if popen.returncode != 0 or not result_path.exists():
            return {
                "status": f"failed(exit={popen.returncode})",
                "library": library,
                "n": n,
                "peak_rss_bytes": peak,
                "wall_ms": (time.monotonic() - start) * 1e3,
                "stdout_tail": stdout[-1000:],
                "stderr_tail": stderr[-2000:],
            }
        result = json.loads(result_path.read_text())
        result["peak_rss_bytes"] = peak
        result["wall_ms"] = (time.monotonic() - start) * 1e3
        return result


def summarize(samples: list[dict[str, Any]]) -> dict[str, Any]:
    values = [float(row["render_ms"]) for row in samples if row.get("status") == "ok"]
    out: dict[str, Any] = {
        "attempted_runs": len(samples),
        "successful_runs": len(values),
        "statuses": [row.get("status") for row in samples],
        "peak_rss_gib": max((row.get("peak_rss_bytes", 0) for row in samples), default=0) / 2**30,
        "samples": samples,
    }
    if values:
        out.update(
            {
                "mean_ms": statistics.fmean(values),
                "median_ms": statistics.median(values),
                "min_ms": min(values),
                "max_ms": max(values),
            }
        )
    return out


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--child", action="store_true")
    parser.add_argument("--library", choices=tuple(PYTHONS))
    parser.add_argument("--n", type=int)
    parser.add_argument("--child-out", type=Path)
    parser.add_argument("--sizes", default="10000,100000,1000000,10000000")
    parser.add_argument("--libraries", default="xy,ruviz,ruviz-fast,matplotlib")
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--timeout", type=float, default=180)
    parser.add_argument("--memory-gib", type=float, default=28)
    parser.add_argument("--out", type=Path, default=SCRATCH / "static-scatter-results.json")
    args = parser.parse_args()
    if args.child:
        result = child_run(args.library, args.n)
        args.child_out.write_text(json.dumps(result), encoding="utf-8")
        return

    limit = int(args.memory_gib * 2**30)
    sizes = [int(v) for v in args.sizes.split(",")]
    libraries = [v.strip() for v in args.libraries.split(",")]
    versions = {}
    for library in set(libraries):
        probe = "ruviz" if library.startswith("ruviz") else library
        versions[library] = subprocess.check_output(
            [PYTHONS[library], "-c", f"import importlib.metadata as m; print(m.version('{probe}'))"],
            text=True,
        ).strip()
    result: dict[str, Any] = {
        "generated_at_utc": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "contract": "validated non-blank 900x420 static PNG; render_ms excludes imports and data generation; fresh process per run; arithmetic mean of successful runs",
        "sizes": sizes,
        "repetitions": args.repetitions,
        "versions": versions,
        "environment": {
            "platform": platform.platform(),
                "python_parent": platform.python_version(),
        },
        "static": [],
    }
    for n in sizes:
        for library in libraries:
            samples = []
            for _ in range(args.repetitions):
                row = run_isolated(library, n, timeout_s=args.timeout, memory_limit_bytes=limit)
                samples.append(row)
                if row.get("status") != "ok":
                    break
            summary = summarize(samples)
            summary.update({"library": library, "n": n})
            result["static"].append(summary)
            print(
                json.dumps(
                    {
                        "n": n,
                        "library": library,
                        "mean_ms": summary.get("mean_ms"),
                        "peak_rss_gib": round(summary["peak_rss_gib"], 2),
                        "statuses": summary["statuses"],
                    }
                ),
                flush=True,
            )
    args.out.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
