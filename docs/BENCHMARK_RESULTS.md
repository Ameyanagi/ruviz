# ruviz Performance Benchmark Results

**This page is an index, not a result set.** The measured numbers live in
[`docs/benchmarks/`](benchmarks/), which is the only place in this repository
where benchmark output is generated from a reproducible harness and kept in
sync with the code.

| What you want to know | Where it is measured |
| --- | --- |
| ruviz vs matplotlib on large datasets, across Python / Rust / wasm | [benchmarks/large-dataset-plotting.md](benchmarks/large-dataset-plotting.md) |
| Effect of the `parallel`, `simd`, `performance` and `gpu` features | [benchmarks/rust-feature-impact.md](benchmarks/rust-feature-impact.md) |
| 3D scene rendering | [benchmarks/ruviz-3d-performance.md](benchmarks/ruviz-3d-performance.md) |

The feature-impact page also documents the renderer changes behind the current
large-dataset numbers: line envelope reduction for oversized raster line
exports, cached histogram bins for static series, and output-surface
rasterization for eligible heatmaps.

## Why the old table was removed

This page previously carried a table of per-plot timings dated 2025-10-07. Every
one of those numbers came from `benches/baseline_benchmarks.rs` at a time when
the benchmark called `.save("….png")` *inside* `b.iter`, so each figure was
rasterization **plus** PNG deflate **plus** a filesystem write. They were not
render times, they could not be compared against any other library's render
times, and a rasterizer regression could hide inside the encoder's noise.

The table also reported an "auto-optimization decision time" for a function that
sets one field and returns; that benchmark has been deleted rather than
re-measured.

## Running the local benchmarks

`benches/baseline_benchmarks.rs` now separates the two costs. Neither group
touches the disk.

```bash
# Rasterization only
cargo bench --bench baseline_benchmarks -- render

# PNG encoding only, on a pre-rendered image
cargo bench --bench baseline_benchmarks -- encode_png

# Everything, plus the points/second throughput group
cargo bench --bench baseline_benchmarks

# Criterion's HTML report
open target/criterion/report/index.html
```

Treat those as a local regression signal on your own machine. Cross-machine and
cross-library claims belong in [`docs/benchmarks/`](benchmarks/), where the
hardware, feature set and harness are recorded alongside the numbers.
