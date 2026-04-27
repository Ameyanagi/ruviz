# Large Dataset Plotting Benchmarks

This page is generated from the large-dataset plotting benchmark reference artifacts.

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

## Renderer Notes

This benchmark page describes the renderer paths active in the reference artifacts. Treat the numbers as a current snapshot for this machine, not a guarantee that every case improved relative to older snapshots.

Current `ruviz` renderer behavior:

- Large monotonic solid line series may reduce to a per-pixel-column envelope before stroking.
- Static histograms can reuse computed `HistogramData` for reused-object render boundaries.
- Eligible nearest-neighbor heatmaps can rasterize directly to the output surface before PNG encoding.
- Python host rendering keeps a native Rust plot/prepared-plot handle alive across repeated render/export calls.

Interpretation notes:

- `render_only` excludes public API reconstruction but still measures rasterization/export work.
- `public_api_render` includes public plot construction and is the better boundary for end-to-end API cost.
- Renderer diagnostics below identify whether optimized paths were active in this run.

## Scenario Matrix

| Scenario | Dataset | Sizes | Canvas |
| --- | --- | --- | --- |
| line | line_wave | 100k, 500k, 1m | 640x480 @ 100 DPI |
| scatter | scatter_cloud | 100k, 250k, 500k | 640x480 @ 100 DPI |
| histogram | histogram_signal | 100k, 1m, 5m | 640x480 @ 100 DPI |
| heatmap | heatmap_field | 512x512, 1024x1024, 2048x2048 | 640x640 @ 100 DPI |

## Environment

- Captured at: `2026-04-27T16:28:25Z`
- Git commit: `191b2876be6577d6f02dfdb8617e170be9c3023d`
- Git branch: `main`
- Git worktree dirty at benchmark start: `yes`
- Host OS: `macOS-26.4-arm64-arm-64bit-Mach-O`
- Host machine: `arm64`
- Host processor: `arm`
- CPU count: `10`
- Python: `3.14.2`
- Rust: `rustc 1.94.1 (e408947bf 2026-03-25)`
- Bun: `1.3.5`
- Chromium: `145.0.7632.6`

Raw artifacts:
- [environment.json](./environment.json)
- [results.csv](./results.csv)
- [python.json](./python.json)
- [rust.json](./rust.json)
- [wasm.json](./wasm.json)

## Rust Renderer Diagnostics

These flags come from the Rust benchmark diagnostics and identify which renderer paths were active for each measured case.

| Plot | Size | Boundary | Mode | Active flags |
| --- | --- | --- | --- | --- |
| line | 100k | render_only | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 100k | public_api_render | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 500k | render_only | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 500k | public_api_render | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 1m | render_only | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 1m | public_api_render | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| scatter | 100k | render_only | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 100k | public_api_render | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 250k | render_only | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 250k | public_api_render | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 500k | render_only | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 500k | public_api_render | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| histogram | 100k | render_only | reference | - |
| histogram | 100k | public_api_render | reference | - |
| histogram | 1m | render_only | reference | - |
| histogram | 1m | public_api_render | reference | - |
| histogram | 5m | render_only | reference | - |
| histogram | 5m | public_api_render | reference | - |
| heatmap | 512x512 | render_only | reference | usedPixelAlignedRectFill |
| heatmap | 512x512 | public_api_render | reference | usedPixelAlignedRectFill |
| heatmap | 1024x1024 | render_only | reference | usedPixelAlignedRectFill |
| heatmap | 1024x1024 | public_api_render | reference | usedPixelAlignedRectFill |
| heatmap | 2048x2048 | render_only | reference | usedPixelAlignedRectFill |
| heatmap | 2048x2048 | public_api_render | reference | usedPixelAlignedRectFill |

## Python: ruviz vs matplotlib (`render_only`)

| Plot | Size | ruviz median | matplotlib median | Speedup |
| --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 5.73 | 29.54 | 5.16x |
| heatmap | 2048x2048 | 7.78 | 60.91 | 7.83x |
| heatmap | 512x512 | 4.44 | 23.38 | 5.26x |
| histogram | 100k | 3.07 | 6.98 | 2.27x |
| histogram | 1m | 3.94 | 9.90 | 2.51x |
| histogram | 5m | 7.17 | 13.85 | 1.93x |
| line | 100k | 10.10 | 20.38 | 2.02x |
| line | 1m | 24.39 | 72.06 | 2.95x |
| line | 500k | 19.77 | 40.50 | 2.05x |
| scatter | 100k | 11.50 | 15.83 | 1.38x |
| scatter | 250k | 25.97 | 29.89 | 1.15x |
| scatter | 500k | 50.83 | 54.35 | 1.07x |

## Python: ruviz vs matplotlib (`public_api_render`)

| Plot | Size | ruviz median | matplotlib median | Speedup |
| --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 41.45 | 37.02 | 0.89x |
| heatmap | 2048x2048 | 161.49 | 70.99 | 0.44x |
| heatmap | 512x512 | 13.08 | 29.92 | 2.29x |
| histogram | 100k | 6.00 | 24.96 | 4.16x |
| histogram | 1m | 36.29 | 54.17 | 1.49x |
| histogram | 5m | 217.25 | 129.12 | 0.59x |
| line | 100k | 14.21 | 29.69 | 2.09x |
| line | 1m | 69.51 | 95.91 | 1.38x |
| line | 500k | 42.02 | 56.81 | 1.35x |
| scatter | 100k | 14.81 | 23.88 | 1.61x |
| scatter | 250k | 34.32 | 39.23 | 1.14x |
| scatter | 500k | 70.62 | 65.77 | 0.93x |

## ruviz cross-runtime medians (`render_only`)

| Plot | Size | Python | Rust | Wasm | Dataset hash |
| --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 5.73 | 14.32 | 28.10 | b300137331a8 |
| heatmap | 2048x2048 | 7.78 | 38.82 | 69.95 | 2c5b8bacf8c3 |
| heatmap | 512x512 | 4.44 | 6.33 | 16.10 | d6e83388d86d |
| histogram | 100k | 3.07 | 3.11 | 9.00 | 52c1b6c47f7a |
| histogram | 1m | 3.94 | 4.02 | 10.00 | 4411c1d13a7c |
| histogram | 5m | 7.17 | 7.24 | 14.30 | 88dd6f1a74af |
| line | 100k | 10.10 | 11.08 | 21.20 | b011dead6d08 |
| line | 1m | 24.39 | 34.07 | 41.65 | a99f5c6c0498 |
| line | 500k | 19.77 | 25.97 | 31.90 | bb2c854f3c82 |
| scatter | 100k | 11.50 | 12.15 | 23.30 | a46d0038919e |
| scatter | 250k | 25.97 | 27.15 | 48.35 | 13bf7083712a |
| scatter | 500k | 50.83 | 54.27 | 91.40 | 6151ba3542b2 |

## ruviz cross-runtime medians (`public_api_render`)

| Plot | Size | Python | Rust | Wasm | Dataset hash |
| --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 41.45 | 18.29 | 56.50 | b300137331a8 |
| heatmap | 2048x2048 | 161.49 | 51.22 | 208.85 | 2c5b8bacf8c3 |
| heatmap | 512x512 | 13.08 | 7.20 | 22.80 | d6e83388d86d |
| histogram | 100k | 6.00 | 4.63 | 12.10 | 52c1b6c47f7a |
| histogram | 1m | 36.29 | 21.20 | 43.60 | 4411c1d13a7c |
| histogram | 5m | 217.25 | 122.32 | 209.80 | 88dd6f1a74af |
| line | 100k | 14.21 | 11.40 | 24.40 | b011dead6d08 |
| line | 1m | 69.51 | 37.90 | 74.15 | a99f5c6c0498 |
| line | 500k | 42.02 | 27.44 | 47.20 | bb2c854f3c82 |
| scatter | 100k | 14.81 | 12.69 | 25.95 | a46d0038919e |
| scatter | 250k | 34.32 | 28.54 | 56.65 | 13bf7083712a |
| scatter | 500k | 70.62 | 56.65 | 108.80 | 6151ba3542b2 |

## Rust: ruviz vs plotters (`public_api_render`)

| Plot | Size | ruviz median | plotters median | Speedup |
| --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 18.29 | 2.34 | 0.13x |
| heatmap | 2048x2048 | 51.22 | 2.81 | 0.05x |
| heatmap | 512x512 | 7.20 | 2.33 | 0.32x |
| histogram | 100k | 4.63 | 0.73 | 0.16x |
| histogram | 1m | 21.20 | 0.72 | 0.03x |
| histogram | 5m | 122.32 | 0.72 | <0.01x |
| line | 100k | 11.40 | 1.64 | 0.14x |
| line | 1m | 37.90 | 10.49 | 0.28x |
| line | 500k | 27.44 | 5.46 | 0.20x |
| scatter | 100k | 12.69 | 2.74 | 0.22x |
| scatter | 250k | 28.54 | 5.40 | 0.19x |
| scatter | 500k | 56.65 | 10.08 | 0.18x |

## Rust render-only throughput

| Plot | Size | Throughput |
| --- | --- | --- |
| heatmap | 1024x1024 | 73.21 M/s |
| heatmap | 2048x2048 | 108.06 M/s |
| heatmap | 512x512 | 41.43 M/s |
| histogram | 100k | 32.18 M/s |
| histogram | 1m | 248.47 M/s |
| histogram | 5m | 690.69 M/s |
| line | 100k | 9.02 M/s |
| line | 1m | 29.35 M/s |
| line | 500k | 19.25 M/s |
| scatter | 100k | 8.23 M/s |
| scatter | 250k | 9.21 M/s |
| scatter | 500k | 9.21 M/s |

## Notes

- These numbers are a reference snapshot from one machine and should be treated as comparative, not universal.
- Browser wasm timings include browser-side PNG generation, but not any disk writes or download flows.
- `render_only` is a reused-built-object benchmark, not a cached-frame benchmark: it avoids public-API reconstruction but still bypasses prepared-frame image reuse for `ruviz`.
- The remaining `plotters` gap on histogram and heatmap is partly semantic: `plotters` benchmarks pre-binned histogram bars and output-raster heatmap generation, while `ruviz` still includes its own plot-model setup and colorbar semantics on the public API path.
