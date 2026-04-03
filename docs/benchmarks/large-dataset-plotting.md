# Large Dataset Plotting Benchmarks

This page is generated from the committed large-dataset plotting benchmark reference run.

## Methodology

- Output target: in-memory PNG byte generation only
- Dataset generation is excluded from all measured timings
- File I/O is excluded from all measured timings
- Boundaries:
  - `render_only`: reuse a built plot object and measure an uncached render/export call without public-API reconstruction or prepared-frame image reuse
  - `public_api_render`: reuse the input data, rebuild through the normal public API, then render/export
- Plot matrix: `line`, `scatter`, `histogram`, `heatmap`
- Python comparison target: `matplotlib` with the `Agg` backend
- Python `ruviz` benchmark runs use a release-built `maturin develop --release` extension
- Rust comparison target: `plotters` on the `public_api_render` boundary only
- `ruviz` PNG exports now use automatic raster fast paths for eligible cases:
  - large monotonic solid lines without markers/error bars are reduced to a per-column envelope before stroking
  - static histograms reuse prepared bins instead of re-binning on every render-only export
  - nearest, non-annotated heatmaps rasterize directly to the output surface before PNG encoding
- Python host-side `ruviz` rendering now uses a persistent native plot handle and prepared plot instead of rebuilding a Rust plot from JSON on every render
- Python `render_only` timings bypass the prepared-frame image cache so they measure rasterization rather than cached PNG encoding
- Notebook widgets still ship JSON-friendly snapshots to the browser, but that snapshot path is no longer the default host render/export path
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
- Python host rendering now keeps a native Rust plot/prepared-plot handle alive across calls, so `render_png()`, `render_svg()`, `save()`, and `show()` no longer pay a Python JSON serialization + Rust JSON parse + plot reconstruction round-trip on every call.

Why those changes matter:

- The old line path scaled with source vertex count even when many samples collapsed onto the same output column.
- The old histogram path repeated statistical preprocessing inside hot render loops.
- The old heatmap path scaled with source cell count rather than output pixel count for raster exports.
- The old Python binding path spent a large share of its time turning Python state into JSON and then rebuilding a fresh Rust `Plot` before rendering anything.

The result is that current Rust PNG export timings mostly reflect output-resolution work for eligible raster cases, and current Python host-side timings reflect the renderer instead of the snapshot bridge much more closely than before.

## Scenario Matrix

| Scenario | Dataset | Sizes | Canvas |
| --- | --- | --- | --- |
| line | line_wave | 100k, 500k, 1m | 640x480 @ 100 DPI |
| scatter | scatter_cloud | 100k, 250k, 500k | 640x480 @ 100 DPI |
| histogram | histogram_signal | 100k, 1m, 5m | 640x480 @ 100 DPI |
| heatmap | heatmap_field | 512x512, 1024x1024, 2048x2048 | 640x640 @ 100 DPI |

## Environment

- Captured at: `2026-04-03T22:07:24Z`
- Git commit: `8311adb5d312ecbb7ab9f22c914f526eb0583cca`
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
| heatmap | 1024x1024 | 5.73 | 29.02 | 5.06x |
| heatmap | 2048x2048 | 6.98 | 60.66 | 8.69x |
| heatmap | 512x512 | 5.77 | 25.44 | 4.41x |
| histogram | 100k | 1.55 | 7.31 | 4.72x |
| histogram | 1m | 2.06 | 10.19 | 4.94x |
| histogram | 5m | 5.13 | 13.85 | 2.70x |
| line | 100k | 9.98 | 20.12 | 2.01x |
| line | 1m | 26.73 | 72.41 | 2.71x |
| line | 500k | 20.79 | 41.05 | 1.97x |
| scatter | 100k | 2.14 | 15.75 | 7.37x |
| scatter | 250k | 3.08 | 28.48 | 9.24x |
| scatter | 500k | 4.65 | 50.06 | 10.77x |

## Python: ruviz vs matplotlib (`public_api_render`)

| Plot | Size | ruviz median | matplotlib median | Speedup |
| --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 36.46 | 36.69 | 1.01x |
| heatmap | 2048x2048 | 125.64 | 73.43 | 0.58x |
| heatmap | 512x512 | 14.02 | 30.01 | 2.14x |
| histogram | 100k | 4.99 | 25.72 | 5.16x |
| histogram | 1m | 34.88 | 55.86 | 1.60x |
| histogram | 5m | 211.79 | 128.60 | 0.61x |
| line | 100k | 13.40 | 29.53 | 2.20x |
| line | 1m | 61.03 | 95.47 | 1.56x |
| line | 500k | 38.08 | 56.62 | 1.49x |
| scatter | 100k | 5.60 | 26.22 | 4.69x |
| scatter | 250k | 11.46 | 38.33 | 3.34x |
| scatter | 500k | 20.86 | 67.34 | 3.23x |

## ruviz cross-runtime medians (`render_only`)

| Plot | Size | Python | Rust | Wasm | Dataset hash |
| --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 5.73 | 5.96 | 17.80 | b300137331a8 |
| heatmap | 2048x2048 | 6.98 | 7.90 | 18.70 | 2c5b8bacf8c3 |
| heatmap | 512x512 | 5.77 | 5.24 | 16.80 | d6e83388d86d |
| histogram | 100k | 1.55 | 1.51 | 5.50 | 52c1b6c47f7a |
| histogram | 1m | 2.06 | 2.28 | 6.10 | 4411c1d13a7c |
| histogram | 5m | 5.13 | 5.19 | 9.20 | 88dd6f1a74af |
| line | 100k | 9.98 | 13.58 | 19.70 | b011dead6d08 |
| line | 1m | 26.73 | 33.43 | 34.65 | a99f5c6c0498 |
| line | 500k | 20.79 | 27.01 | 27.60 | bb2c854f3c82 |
| scatter | 100k | 2.14 | 2.11 | 6.90 | a46d0038919e |
| scatter | 250k | 3.08 | 3.25 | 8.40 | 13bf7083712a |
| scatter | 500k | 4.65 | 4.52 | 10.60 | 6151ba3542b2 |

## ruviz cross-runtime medians (`public_api_render`)

| Plot | Size | Python | Rust | Wasm | Dataset hash |
| --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 36.46 | 9.67 | 47.25 | b300137331a8 |
| heatmap | 2048x2048 | 125.64 | 20.92 | 225.70 | 2c5b8bacf8c3 |
| heatmap | 512x512 | 14.02 | 6.54 | 23.70 | d6e83388d86d |
| histogram | 100k | 4.99 | 3.04 | 8.55 | 52c1b6c47f7a |
| histogram | 1m | 34.88 | 19.40 | 40.90 | 4411c1d13a7c |
| histogram | 5m | 211.79 | 118.34 | 212.50 | 88dd6f1a74af |
| line | 100k | 13.40 | 14.01 | 22.90 | b011dead6d08 |
| line | 1m | 61.03 | 37.49 | 67.10 | a99f5c6c0498 |
| line | 500k | 38.08 | 27.78 | 45.70 | bb2c854f3c82 |
| scatter | 100k | 5.60 | 2.50 | 9.80 | a46d0038919e |
| scatter | 250k | 11.46 | 4.23 | 16.40 | 13bf7083712a |
| scatter | 500k | 20.86 | 6.65 | 27.00 | 6151ba3542b2 |

## Rust: ruviz vs plotters (`public_api_render`)

| Plot | Size | ruviz median | plotters median | Speedup |
| --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 9.67 | 2.36 | 0.24x |
| heatmap | 2048x2048 | 20.92 | 2.90 | 0.14x |
| heatmap | 512x512 | 6.54 | 2.51 | 0.38x |
| histogram | 100k | 3.04 | 0.73 | 0.24x |
| histogram | 1m | 19.40 | 0.72 | 0.04x |
| histogram | 5m | 118.34 | 0.73 | <0.01x |
| line | 100k | 14.01 | 1.71 | 0.12x |
| line | 1m | 37.49 | 11.19 | 0.30x |
| line | 500k | 27.78 | 5.93 | 0.21x |
| scatter | 100k | 2.50 | 2.63 | 1.05x |
| scatter | 250k | 4.23 | 5.72 | 1.35x |
| scatter | 500k | 6.65 | 10.71 | 1.61x |

## Rust render-only throughput

| Plot | Size | Throughput |
| --- | --- | --- |
| heatmap | 1024x1024 | 176.01 M/s |
| heatmap | 2048x2048 | 530.90 M/s |
| heatmap | 512x512 | 50.02 M/s |
| histogram | 100k | 66.22 M/s |
| histogram | 1m | 439.34 M/s |
| histogram | 5m | 962.90 M/s |
| line | 100k | 7.36 M/s |
| line | 1m | 29.91 M/s |
| line | 500k | 18.52 M/s |
| scatter | 100k | 47.41 M/s |
| scatter | 250k | 76.82 M/s |
| scatter | 500k | 110.62 M/s |

## Notes

- These numbers are a reference snapshot from one machine and should be treated as comparative, not universal.
- Browser wasm timings include browser-side PNG generation, but not any disk writes or download flows.
- `render_only` is a reused-built-object benchmark, not a cached-frame benchmark: it avoids public-API reconstruction but still bypasses prepared-frame image reuse for `ruviz`.
- The remaining `plotters` gap on histogram and heatmap is partly semantic: `plotters` benchmarks pre-binned histogram bars and output-raster heatmap generation, while `ruviz` still includes its own plot-model setup and colorbar semantics on the public API path.
