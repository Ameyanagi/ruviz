# Large Dataset Plotting Benchmarks

This page is generated from the committed large-dataset plotting benchmark reference run.

## Methodology

- Output target: in-memory PNG byte generation only
- Dataset generation is excluded from all measured timings
- File I/O is excluded from all measured timings
- Boundaries:
  - `render_only`: reuse a prepared plot or snapshot and measure only the render/export call
  - `public_api_render`: reuse the input data, rebuild through the normal public API, then render/export
- Plot matrix: `line`, `scatter`, `histogram`, `heatmap`
- Python comparison target: `matplotlib` with the `Agg` backend
- Rust comparison target: `plotters` on the `public_api_render` boundary only
- `ruviz` PNG exports now use automatic raster fast paths for eligible cases:
  - large monotonic solid lines without markers/error bars are reduced to a per-column envelope before stroking
  - static histograms reuse prepared bins instead of re-binning on every render-only export
  - nearest, non-annotated heatmaps rasterize directly to the output surface before PNG encoding
- Python full-mode runs cap very slow cases to a 60s per-case budget; the recorded warmup/measured counts reflect the effective counts used
- Rust `plotters` histogram timings reuse pre-binned bars, and the `plotters` heatmap path rasterizes the shared matrix to the fixed output canvas before PNG encoding
- wasm target: Chromium-only browser benchmark via Playwright
- Full-run warmup / measured iterations: `5` / `20`

## Why It Got Faster

The main change in this benchmark update is not a different benchmark harness. It is a different raster renderer path for large PNG exports.

What changed in `ruviz`:

- Large monotonic solid line series are reduced to a per-pixel-column envelope before stroking, so a `1M` point line on a `640px` canvas no longer pays to stroke every original segment.
- Static histograms now cache computed `HistogramData`, so `render_only` exports reuse prepared bins instead of re-running histogram binning on every frame.
- Nearest-neighbor, non-annotated heatmaps now render the final output surface directly and blit that image, instead of drawing one anti-aliased rectangle per source cell.
- The parallel line backend now emits a single polyline draw instead of thousands of two-point draw calls.

Why those changes matter:

- The old line path scaled with source vertex count even when many samples collapsed onto the same output column.
- The old histogram path repeated statistical preprocessing inside hot render loops.
- The old heatmap path scaled with source cell count rather than output pixel count for raster exports.

The result is that current Rust PNG export timings mostly reflect output-resolution work for eligible raster cases, which is why line, histogram, and heatmap medians dropped sharply in this report.

## Scenario Matrix

| Scenario | Dataset | Sizes | Canvas |
| --- | --- | --- | --- |
| line | line_wave | 100k, 500k, 1m | 640x480 @ 100 DPI |
| scatter | scatter_cloud | 100k, 250k, 500k | 640x480 @ 100 DPI |
| histogram | histogram_signal | 100k, 1m, 5m | 640x480 @ 100 DPI |
| heatmap | heatmap_field | 512x512, 1024x1024, 2048x2048 | 640x640 @ 100 DPI |

## Environment

- Captured at: `2026-04-03T10:45:37Z`
- Git commit: `36b49c46ce45223d3df3086976f485c0edfe1d0f`
- Git branch: `bench/large-dataset-plotting`
- Host OS: `macOS-26.2-arm64-arm-64bit-Mach-O`
- Host machine: `arm64`
- Host processor: `arm`
- CPU count: `10`
- Python: `3.14.2`
- Rust: `rustc 1.94.1 (e408947bf 2026-03-25)`
- Bun: `1.3.5`
- Chromium: `145.0.7632.6`

Raw artifacts:
- [environment.json](../../benchmarks/plotting/results/reference/environment.json)
- [results.csv](../../benchmarks/plotting/results/reference/results.csv)
- [python.json](../../benchmarks/plotting/results/reference/python.json)
- [rust.json](../../benchmarks/plotting/results/reference/rust.json)
- [wasm.json](../../benchmarks/plotting/results/reference/wasm.json)

## Python: ruviz vs matplotlib (`render_only`)

| Plot | Size | ruviz median | matplotlib median | Speedup |
| --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 644.06 | 29.52 | 0.05x |
| heatmap | 2048x2048 | 1813.41 | 61.41 | 0.03x |
| heatmap | 512x512 | 359.69 | 23.65 | 0.07x |
| histogram | 100k | 193.53 | 6.91 | 0.04x |
| histogram | 1m | 959.76 | 10.72 | 0.01x |
| histogram | 5m | 4633.92 | 14.90 | <0.01x |
| line | 100k | 446.80 | 23.58 | 0.05x |
| line | 1m | 1092.63 | 77.98 | 0.07x |
| line | 500k | 638.63 | 43.42 | 0.07x |
| scatter | 100k | 208.86 | 19.69 | 0.09x |
| scatter | 250k | 339.76 | 31.86 | 0.09x |
| scatter | 500k | 511.01 | 50.19 | 0.10x |

## Python: ruviz vs matplotlib (`public_api_render`)

| Plot | Size | ruviz median | matplotlib median | Speedup |
| --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 1264.55 | 36.90 | 0.03x |
| heatmap | 2048x2048 | 4529.48 | 72.38 | 0.02x |
| heatmap | 512x512 | 541.08 | 30.68 | 0.06x |
| histogram | 100k | 252.42 | 24.94 | 0.10x |
| histogram | 1m | 1552.52 | 53.31 | 0.03x |
| histogram | 5m | 7637.05 | 139.55 | 0.02x |
| line | 100k | 579.52 | 32.18 | 0.06x |
| line | 1m | 2310.68 | 96.96 | 0.04x |
| line | 500k | 1288.48 | 57.69 | 0.04x |
| scatter | 100k | 334.45 | 27.59 | 0.08x |
| scatter | 250k | 613.30 | 39.36 | 0.06x |
| scatter | 500k | 1072.99 | 65.12 | 0.06x |

## ruviz cross-runtime medians (`render_only`)

| Plot | Size | Python | Rust | Wasm | Dataset hash |
| --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 644.06 | 6.26 | 17.30 | b300137331a8 |
| heatmap | 2048x2048 | 1813.41 | 7.71 | 18.90 | 2c5b8bacf8c3 |
| heatmap | 512x512 | 359.69 | 5.36 | 16.80 | d6e83388d86d |
| histogram | 100k | 193.53 | 1.52 | 5.40 | 52c1b6c47f7a |
| histogram | 1m | 959.76 | 2.25 | 6.50 | 4411c1d13a7c |
| histogram | 5m | 4633.92 | 5.57 | 9.30 | 88dd6f1a74af |
| line | 100k | 446.80 | 13.74 | 22.15 | b011dead6d08 |
| line | 1m | 1092.63 | 30.30 | 42.20 | a99f5c6c0498 |
| line | 500k | 638.63 | 25.48 | 34.75 | bb2c854f3c82 |
| scatter | 100k | 208.86 | 2.28 | 6.95 | a46d0038919e |
| scatter | 250k | 339.76 | 3.39 | 8.35 | 13bf7083712a |
| scatter | 500k | 511.01 | 4.87 | 10.60 | 6151ba3542b2 |

## ruviz cross-runtime medians (`public_api_render`)

| Plot | Size | Python | Rust | Wasm | Dataset hash |
| --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 1264.55 | 10.15 | 46.75 | b300137331a8 |
| heatmap | 2048x2048 | 4529.48 | 22.42 | 214.20 | 2c5b8bacf8c3 |
| heatmap | 512x512 | 541.08 | 6.32 | 23.40 | d6e83388d86d |
| histogram | 100k | 252.42 | 3.20 | 8.40 | 52c1b6c47f7a |
| histogram | 1m | 1552.52 | 20.72 | 40.15 | 4411c1d13a7c |
| histogram | 5m | 7637.05 | 124.21 | 214.60 | 88dd6f1a74af |
| line | 100k | 579.52 | 13.91 | 25.05 | b011dead6d08 |
| line | 1m | 2310.68 | 35.92 | 74.80 | a99f5c6c0498 |
| line | 500k | 1288.48 | 27.44 | 51.20 | bb2c854f3c82 |
| scatter | 100k | 334.45 | 2.66 | 9.80 | a46d0038919e |
| scatter | 250k | 613.30 | 4.88 | 16.00 | 13bf7083712a |
| scatter | 500k | 1072.99 | 7.18 | 26.40 | 6151ba3542b2 |

## Rust: ruviz vs plotters (`public_api_render`)

| Plot | Size | ruviz median | plotters median | Speedup |
| --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 10.15 | 2.51 | 0.25x |
| heatmap | 2048x2048 | 22.42 | 3.11 | 0.14x |
| heatmap | 512x512 | 6.32 | 2.39 | 0.38x |
| histogram | 100k | 3.20 | 0.76 | 0.24x |
| histogram | 1m | 20.72 | 0.77 | 0.04x |
| histogram | 5m | 124.21 | 0.75 | <0.01x |
| line | 100k | 13.91 | 1.72 | 0.12x |
| line | 1m | 35.92 | 10.98 | 0.31x |
| line | 500k | 27.44 | 5.58 | 0.20x |
| scatter | 100k | 2.66 | 2.75 | 1.03x |
| scatter | 250k | 4.88 | 6.49 | 1.33x |
| scatter | 500k | 7.18 | 12.03 | 1.68x |

## Rust render-only throughput

| Plot | Size | Throughput |
| --- | --- | --- |
| heatmap | 1024x1024 | 167.41 M/s |
| heatmap | 2048x2048 | 544.36 M/s |
| heatmap | 512x512 | 48.94 M/s |
| histogram | 100k | 65.75 M/s |
| histogram | 1m | 443.86 M/s |
| histogram | 5m | 897.15 M/s |
| line | 100k | 7.28 M/s |
| line | 1m | 33.00 M/s |
| line | 500k | 19.62 M/s |
| scatter | 100k | 43.86 M/s |
| scatter | 250k | 73.75 M/s |
| scatter | 500k | 102.75 M/s |

## Notes

- These numbers are a reference snapshot from one machine and should be treated as comparative, not universal.
- Browser wasm timings include browser-side PNG generation, but not any disk writes or download flows.
- Python `render_only` for `ruviz` uses the internal `_native.render_png_bytes(snapshot_json)` path, so JSON parsing is included there even though plot construction is excluded.
- The remaining `plotters` gap on histogram and heatmap is partly semantic: `plotters` benchmarks pre-binned histogram bars and output-raster heatmap generation, while `ruviz` still includes its own plot-model setup and colorbar semantics on the public API path.
