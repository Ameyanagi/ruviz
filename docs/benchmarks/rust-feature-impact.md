# Rust Feature Impact Plotting Benchmarks

This page is generated from the Rust feature-impact plotting benchmark reference artifacts.

## Methodology

- Scope: Rust-only feature study for the core `ruviz` crate
- Output target: in-memory PNG bytes only
- Dataset generation is excluded from all measured timings
- File I/O is excluded from all measured timings
- Boundaries:
  - `render_only`: reuse a built plot object and call `render_png_bytes()`
  - `public_api_render`: rebuild through the public API, then call `render_png_bytes()`
  - `save_only`: reuse a built plot object and measure the same backend-selection path used by `save()`, but write PNG to memory instead of disk
  - `public_api_save`: rebuild through the public API, then measure the same in-memory `save()` backend path
- Feature builds benchmarked:
  - `baseline_cpu`: `--no-default-features`
  - `default`: default crate features
  - `parallel_only`: `--no-default-features --features parallel`
  - `parallel_simd`: `--no-default-features --features parallel,simd`
  - `performance_alias`: `--no-default-features --features performance`
  - `gpu_only`: `--no-default-features --features gpu`
- Every benchmark build also enables `serde` for JSON output only
- `gpu_only` requests `.gpu(true)` for the save-path boundaries only
- Save-path tables include the actual backend used in brackets, so CPU fallbacks are visible
- Full-run warmup / measured iterations: `5` / `20`

## Scenario Matrix

| Scenario | Dataset | Sizes | Canvas |
| --- | --- | --- | --- |
| line | line_wave | 100k, 500k, 1m | 640x480 @ 100 DPI |
| scatter | scatter_cloud | 100k, 250k, 500k | 640x480 @ 100 DPI |
| histogram | histogram_signal | 100k, 1m, 5m | 640x480 @ 100 DPI |
| heatmap | heatmap_field | 512x512, 1024x1024, 2048x2048 | 640x640 @ 100 DPI |

## Feature Builds

| Label | Cargo features | GPU requested for save path |
| --- | --- | --- |
| baseline_cpu | serde | no |
| default | ndarray, parallel, serde | no |
| parallel_only | parallel, serde | no |
| parallel_simd | parallel, simd, serde | no |
| performance_alias | performance, serde | no |
| gpu_only | gpu, serde | yes |

## Environment

- Captured at: `2026-04-27T16:30:26Z`
- Git commit: `191b2876be6577d6f02dfdb8617e170be9c3023d`
- Git branch: `main`
- Git worktree dirty at benchmark start: `yes`
- Host OS: `macOS-26.4-arm64-arm-64bit-Mach-O`
- Host machine: `arm64`
- Host processor: `arm`
- CPU count: `10`
- Python: `3.14.2`
- Rust: `rustc 1.94.1 (e408947bf 2026-03-25)`

Raw artifacts:
- [environment.json](../../benchmarks/plotting/results/rust-features/reference/environment.json)
- [results.csv](../../benchmarks/plotting/results/rust-features/reference/results.csv)
- [baseline_cpu.json](../../benchmarks/plotting/results/rust-features/reference/baseline_cpu.json)
- [default.json](../../benchmarks/plotting/results/rust-features/reference/default.json)
- [parallel_only.json](../../benchmarks/plotting/results/rust-features/reference/parallel_only.json)
- [parallel_simd.json](../../benchmarks/plotting/results/rust-features/reference/parallel_simd.json)
- [performance_alias.json](../../benchmarks/plotting/results/rust-features/reference/performance_alias.json)
- [gpu_only.json](../../benchmarks/plotting/results/rust-features/reference/gpu_only.json)

## Default Build Renderer Diagnostics

These rows show the active renderer diagnostics for the default feature build.

| Plot | Size | Boundary | Backend | Mode | Active flags |
| --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | render_only | - | reference | usedPixelAlignedRectFill |
| heatmap | 1024x1024 | public_api_render | - | reference | usedPixelAlignedRectFill |
| heatmap | 1024x1024 | save_only | skia | reference | usedPixelAlignedRectFill |
| heatmap | 1024x1024 | public_api_save | skia | reference | usedPixelAlignedRectFill |
| heatmap | 2048x2048 | render_only | - | reference | usedPixelAlignedRectFill |
| heatmap | 2048x2048 | public_api_render | - | reference | usedPixelAlignedRectFill |
| heatmap | 2048x2048 | save_only | skia | reference | usedPixelAlignedRectFill |
| heatmap | 2048x2048 | public_api_save | skia | reference | usedPixelAlignedRectFill |
| heatmap | 512x512 | render_only | - | reference | usedPixelAlignedRectFill |
| heatmap | 512x512 | public_api_render | - | reference | usedPixelAlignedRectFill |
| heatmap | 512x512 | save_only | skia | reference | usedPixelAlignedRectFill |
| heatmap | 512x512 | public_api_save | skia | reference | usedPixelAlignedRectFill |
| histogram | 100k | render_only | - | reference | - |
| histogram | 100k | public_api_render | - | reference | - |
| histogram | 100k | save_only | skia | reference | - |
| histogram | 100k | public_api_save | skia | reference | - |
| histogram | 1m | render_only | - | reference | - |
| histogram | 1m | public_api_render | - | reference | - |
| histogram | 1m | save_only | skia | reference | - |
| histogram | 1m | public_api_save | skia | reference | - |
| histogram | 5m | render_only | - | reference | - |
| histogram | 5m | public_api_render | - | reference | - |
| histogram | 5m | save_only | skia | reference | - |
| histogram | 5m | public_api_save | skia | reference | - |
| line | 100k | render_only | - | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 100k | public_api_render | - | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 100k | save_only | skia | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 100k | public_api_save | skia | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 1m | render_only | - | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 1m | public_api_render | - | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 1m | save_only | skia | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 1m | public_api_save | skia | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 500k | render_only | - | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 500k | public_api_render | - | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 500k | save_only | skia | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| line | 500k | public_api_save | skia | reference | usedExactLineCanonicalization, usedRasterLineReduction |
| scatter | 100k | render_only | - | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 100k | public_api_render | - | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 100k | save_only | skia | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 100k | public_api_save | skia | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 250k | render_only | - | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 250k | public_api_render | - | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 250k | save_only | skia | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 250k | public_api_save | skia | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 500k | render_only | - | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 500k | public_api_render | - | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 500k | save_only | skia | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |
| scatter | 500k | public_api_save | skia | reference | usedMarkerSpriteCache, usedMarkerSpriteCompositor, usedMarkerScanlineBlit |

## Rust feature impact (`render_only`)

| Plot | Size | baseline_cpu | default | parallel_only | parallel_simd | performance_alias | gpu_only |
| --- | --- | --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 13.65 ms | 14.92 ms (0.91x) | 13.70 ms (1.00x) | 14.56 ms (0.94x) | 14.31 ms (0.95x) | 13.66 ms (1.00x) |
| heatmap | 2048x2048 | 39.24 ms | 39.06 ms (1.00x) | 38.89 ms (1.01x) | 39.06 ms (1.00x) | 39.15 ms (1.00x) | 38.77 ms (1.01x) |
| heatmap | 512x512 | 6.42 ms | 6.44 ms (1.00x) | 6.44 ms (1.00x) | 6.39 ms (1.00x) | 6.51 ms (0.99x) | 6.45 ms (0.99x) |
| histogram | 100k | 3.10 ms | 3.10 ms (1.00x) | 3.05 ms (1.02x) | 3.09 ms (1.00x) | 3.07 ms (1.01x) | 3.11 ms (1.00x) |
| histogram | 1m | 4.02 ms | 3.92 ms (1.03x) | 3.95 ms (1.02x) | 3.98 ms (1.01x) | 3.99 ms (1.01x) | 4.00 ms (1.01x) |
| histogram | 5m | 7.20 ms | 7.32 ms (0.98x) | 7.22 ms (1.00x) | 7.24 ms (1.00x) | 7.32 ms (0.98x) | 7.38 ms (0.98x) |
| line | 100k | 11.16 ms | 11.12 ms (1.00x) | 11.08 ms (1.01x) | 11.06 ms (1.01x) | 11.06 ms (1.01x) | 11.06 ms (1.01x) |
| line | 1m | 35.61 ms | 33.92 ms (1.05x) | 35.55 ms (1.00x) | 33.96 ms (1.05x) | 35.09 ms (1.01x) | 35.79 ms (0.99x) |
| line | 500k | 25.02 ms | 25.19 ms (0.99x) | 24.91 ms (1.00x) | 25.96 ms (0.96x) | 24.97 ms (1.00x) | 26.25 ms (0.95x) |
| scatter | 100k | 12.31 ms | 12.18 ms (1.01x) | 12.03 ms (1.02x) | 12.14 ms (1.01x) | 12.09 ms (1.02x) | 12.42 ms (0.99x) |
| scatter | 250k | 27.14 ms | 27.73 ms (0.98x) | 27.70 ms (0.98x) | 27.62 ms (0.98x) | 27.68 ms (0.98x) | 27.87 ms (0.97x) |
| scatter | 500k | 54.03 ms | 54.89 ms (0.98x) | 55.40 ms (0.98x) | 54.95 ms (0.98x) | 54.19 ms (1.00x) | 55.10 ms (0.98x) |

## Rust feature impact (`public_api_render`)

| Plot | Size | baseline_cpu | default | parallel_only | parallel_simd | performance_alias | gpu_only |
| --- | --- | --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 17.11 ms | 17.38 ms (0.98x) | 17.22 ms (0.99x) | 17.17 ms (1.00x) | 17.80 ms (0.96x) | 17.36 ms (0.99x) |
| heatmap | 2048x2048 | 51.88 ms | 51.40 ms (1.01x) | 51.92 ms (1.00x) | 51.35 ms (1.01x) | 51.67 ms (1.00x) | 52.24 ms (0.99x) |
| heatmap | 512x512 | 7.25 ms | 7.16 ms (1.01x) | 7.23 ms (1.00x) | 7.19 ms (1.01x) | 7.36 ms (0.99x) | 7.29 ms (0.99x) |
| histogram | 100k | 4.66 ms | 4.62 ms (1.01x) | 4.64 ms (1.01x) | 4.64 ms (1.00x) | 4.65 ms (1.00x) | 4.70 ms (0.99x) |
| histogram | 1m | 21.38 ms | 21.24 ms (1.01x) | 21.31 ms (1.00x) | 21.26 ms (1.01x) | 22.74 ms (0.94x) | 21.50 ms (0.99x) |
| histogram | 5m | 123.74 ms | 121.25 ms (1.02x) | 122.34 ms (1.01x) | 128.25 ms (0.96x) | 126.96 ms (0.97x) | 125.20 ms (0.99x) |
| line | 100k | 11.72 ms | 11.31 ms (1.04x) | 11.43 ms (1.03x) | 11.42 ms (1.03x) | 11.50 ms (1.02x) | 11.50 ms (1.02x) |
| line | 1m | 38.39 ms | 39.63 ms (0.97x) | 38.06 ms (1.01x) | 39.59 ms (0.97x) | 38.76 ms (0.99x) | 38.45 ms (1.00x) |
| line | 500k | 27.76 ms | 27.93 ms (0.99x) | 27.94 ms (0.99x) | 26.95 ms (1.03x) | 28.10 ms (0.99x) | 26.93 ms (1.03x) |
| scatter | 100k | 12.61 ms | 12.60 ms (1.00x) | 12.68 ms (1.00x) | 12.41 ms (1.02x) | 13.51 ms (0.93x) | 13.08 ms (0.96x) |
| scatter | 250k | 28.55 ms | 28.52 ms (1.00x) | 28.45 ms (1.00x) | 28.38 ms (1.01x) | 28.14 ms (1.01x) | 28.76 ms (0.99x) |
| scatter | 500k | 56.19 ms | 57.29 ms (0.98x) | 56.41 ms (1.00x) | 56.51 ms (0.99x) | 56.32 ms (1.00x) | 58.45 ms (0.96x) |

## Rust feature impact (`save_only`)

| Plot | Size | baseline_cpu | default | parallel_only | parallel_simd | performance_alias | gpu_only |
| --- | --- | --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 13.69 ms [skia] | 13.70 ms (1.00x) [skia] | 13.67 ms (1.00x) [skia] | 13.74 ms (1.00x) [skia] | 14.34 ms (0.95x) [skia] | 13.64 ms (1.00x) [skia] |
| heatmap | 2048x2048 | 39.31 ms [skia] | 38.84 ms (1.01x) [skia] | 39.10 ms (1.01x) [skia] | 38.87 ms (1.01x) [skia] | 39.42 ms (1.00x) [skia] | 39.75 ms (0.99x) [skia] |
| heatmap | 512x512 | 6.33 ms [skia] | 6.36 ms (0.99x) [skia] | 6.38 ms (0.99x) [skia] | 6.31 ms (1.00x) [skia] | 6.49 ms (0.97x) [skia] | 6.44 ms (0.98x) [skia] |
| histogram | 100k | 3.08 ms [skia] | 3.07 ms (1.00x) [skia] | 3.08 ms (1.00x) [skia] | 3.07 ms (1.00x) [skia] | 3.06 ms (1.00x) [skia] | 3.13 ms (0.98x) [skia] |
| histogram | 1m | 3.92 ms [skia] | 3.91 ms (1.00x) [skia] | 4.06 ms (0.97x) [skia] | 3.94 ms (1.00x) [skia] | 3.92 ms (1.00x) [skia] | 4.12 ms (0.95x) [skia] |
| histogram | 5m | 7.25 ms [skia] | 7.34 ms (0.99x) [skia] | 7.28 ms (1.00x) [skia] | 7.26 ms (1.00x) [skia] | 7.29 ms (0.99x) [skia] | 7.23 ms (1.00x) [skia] |
| line | 100k | 11.25 ms [skia] | 11.20 ms (1.00x) [skia] | 11.21 ms (1.00x) [skia] | 11.00 ms (1.02x) [skia] | 11.11 ms (1.01x) [skia] | 11.10 ms (1.01x) [skia] |
| line | 1m | 35.63 ms [skia] | 33.96 ms (1.05x) [skia] | 35.69 ms (1.00x) [skia] | 34.11 ms (1.04x) [skia] | 35.03 ms (1.02x) [skia] | 34.21 ms (1.04x) [skia] |
| line | 500k | 25.87 ms [skia] | 24.90 ms (1.04x) [skia] | 24.94 ms (1.04x) [skia] | 25.97 ms (1.00x) [skia] | 26.19 ms (0.99x) [skia] | 26.29 ms (0.98x) [skia] |
| scatter | 100k | 12.14 ms [skia] | 12.62 ms (0.96x) [skia] | 12.01 ms (1.01x) [skia] | 12.54 ms (0.97x) [skia] | 11.92 ms (1.02x) [skia] | 12.30 ms (0.99x) [skia] |
| scatter | 250k | 27.35 ms [skia] | 27.80 ms (0.98x) [skia] | 27.71 ms (0.99x) [skia] | 27.06 ms (1.01x) [skia] | 27.14 ms (1.01x) [skia] | 28.05 ms (0.98x) [skia] |
| scatter | 500k | 54.78 ms [skia] | 55.33 ms (0.99x) [skia] | 55.38 ms (0.99x) [skia] | 53.88 ms (1.02x) [skia] | 54.47 ms (1.01x) [skia] | 55.56 ms (0.99x) [skia] |

## Rust feature impact (`public_api_save`)

| Plot | Size | baseline_cpu | default | parallel_only | parallel_simd | performance_alias | gpu_only |
| --- | --- | --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 17.42 ms [skia] | 17.10 ms (1.02x) [skia] | 17.38 ms (1.00x) [skia] | 17.30 ms (1.01x) [skia] | 18.81 ms (0.93x) [skia] | 17.17 ms (1.01x) [skia] |
| heatmap | 2048x2048 | 51.73 ms [skia] | 51.32 ms (1.01x) [skia] | 51.36 ms (1.01x) [skia] | 52.04 ms (0.99x) [skia] | 51.95 ms (1.00x) [skia] | 52.18 ms (0.99x) [skia] |
| heatmap | 512x512 | 7.25 ms [skia] | 7.75 ms (0.94x) [skia] | 7.55 ms (0.96x) [skia] | 7.21 ms (1.01x) [skia] | 7.63 ms (0.95x) [skia] | 7.27 ms (1.00x) [skia] |
| histogram | 100k | 4.65 ms [skia] | 4.59 ms (1.01x) [skia] | 4.61 ms (1.01x) [skia] | 4.61 ms (1.01x) [skia] | 4.60 ms (1.01x) [skia] | 4.68 ms (0.99x) [skia] |
| histogram | 1m | 21.70 ms [skia] | 21.72 ms (1.00x) [skia] | 21.24 ms (1.02x) [skia] | 21.48 ms (1.01x) [skia] | 21.32 ms (1.02x) [skia] | 22.42 ms (0.97x) [skia] |
| histogram | 5m | 129.28 ms [skia] | 129.51 ms (1.00x) [skia] | 127.25 ms (1.02x) [skia] | 129.59 ms (1.00x) [skia] | 128.77 ms (1.00x) [skia] | 129.24 ms (1.00x) [skia] |
| line | 100k | 11.43 ms [skia] | 11.33 ms (1.01x) [skia] | 11.46 ms (1.00x) [skia] | 11.47 ms (1.00x) [skia] | 11.50 ms (0.99x) [skia] | 11.53 ms (0.99x) [skia] |
| line | 1m | 37.91 ms [skia] | 39.38 ms (0.96x) [skia] | 39.12 ms (0.97x) [skia] | 39.62 ms (0.96x) [skia] | 39.54 ms (0.96x) [skia] | 38.20 ms (0.99x) [skia] |
| line | 500k | 26.98 ms [skia] | 28.08 ms (0.96x) [skia] | 27.95 ms (0.97x) [skia] | 28.06 ms (0.96x) [skia] | 26.91 ms (1.00x) [skia] | 27.00 ms (1.00x) [skia] |
| scatter | 100k | 12.74 ms [skia] | 12.59 ms (1.01x) [skia] | 12.47 ms (1.02x) [skia] | 12.35 ms (1.03x) [skia] | 12.30 ms (1.04x) [skia] | 13.93 ms (0.91x) [skia] |
| scatter | 250k | 28.52 ms [skia] | 28.51 ms (1.00x) [skia] | 28.43 ms (1.00x) [skia] | 28.37 ms (1.01x) [skia] | 28.29 ms (1.01x) [skia] | 29.07 ms (0.98x) [skia] |
| scatter | 500k | 57.30 ms [skia] | 56.99 ms (1.01x) [skia] | 56.71 ms (1.01x) [skia] | 56.89 ms (1.01x) [skia] | 56.78 ms (1.01x) [skia] | 56.99 ms (1.01x) [skia] |

## `parallel_simd` vs `performance_alias`

These rows should remain near parity because `performance` currently aliases `parallel + simd`.

| Plot | Size | Boundary | parallel_simd | performance_alias | Alias ratio |
| --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | render_only | 14.56 ms | 14.31 ms | 0.98x |
| heatmap | 1024x1024 | public_api_render | 17.17 ms | 17.80 ms | 1.04x |
| heatmap | 1024x1024 | save_only | 13.74 ms | 14.34 ms | 1.04x |
| heatmap | 1024x1024 | public_api_save | 17.30 ms | 18.81 ms | 1.09x |
| heatmap | 2048x2048 | render_only | 39.06 ms | 39.15 ms | 1.00x |
| heatmap | 2048x2048 | public_api_render | 51.35 ms | 51.67 ms | 1.01x |
| heatmap | 2048x2048 | save_only | 38.87 ms | 39.42 ms | 1.01x |
| heatmap | 2048x2048 | public_api_save | 52.04 ms | 51.95 ms | 1.00x |
| heatmap | 512x512 | render_only | 6.39 ms | 6.51 ms | 1.02x |
| heatmap | 512x512 | public_api_render | 7.19 ms | 7.36 ms | 1.02x |
| heatmap | 512x512 | save_only | 6.31 ms | 6.49 ms | 1.03x |
| heatmap | 512x512 | public_api_save | 7.21 ms | 7.63 ms | 1.06x |
| histogram | 100k | render_only | 3.09 ms | 3.07 ms | 0.99x |
| histogram | 100k | public_api_render | 4.64 ms | 4.65 ms | 1.00x |
| histogram | 100k | save_only | 3.07 ms | 3.06 ms | 1.00x |
| histogram | 100k | public_api_save | 4.61 ms | 4.60 ms | 1.00x |
| histogram | 1m | render_only | 3.98 ms | 3.99 ms | 1.00x |
| histogram | 1m | public_api_render | 21.26 ms | 22.74 ms | 1.07x |
| histogram | 1m | save_only | 3.94 ms | 3.92 ms | 1.00x |
| histogram | 1m | public_api_save | 21.48 ms | 21.32 ms | 0.99x |
| histogram | 5m | render_only | 7.24 ms | 7.32 ms | 1.01x |
| histogram | 5m | public_api_render | 128.25 ms | 126.96 ms | 0.99x |
| histogram | 5m | save_only | 7.26 ms | 7.29 ms | 1.00x |
| histogram | 5m | public_api_save | 129.59 ms | 128.77 ms | 0.99x |
| line | 100k | render_only | 11.06 ms | 11.06 ms | 1.00x |
| line | 100k | public_api_render | 11.42 ms | 11.50 ms | 1.01x |
| line | 100k | save_only | 11.00 ms | 11.11 ms | 1.01x |
| line | 100k | public_api_save | 11.47 ms | 11.50 ms | 1.00x |
| line | 1m | render_only | 33.96 ms | 35.09 ms | 1.03x |
| line | 1m | public_api_render | 39.59 ms | 38.76 ms | 0.98x |
| line | 1m | save_only | 34.11 ms | 35.03 ms | 1.03x |
| line | 1m | public_api_save | 39.62 ms | 39.54 ms | 1.00x |
| line | 500k | render_only | 25.96 ms | 24.97 ms | 0.96x |
| line | 500k | public_api_render | 26.95 ms | 28.10 ms | 1.04x |
| line | 500k | save_only | 25.97 ms | 26.19 ms | 1.01x |
| line | 500k | public_api_save | 28.06 ms | 26.91 ms | 0.96x |
| scatter | 100k | render_only | 12.14 ms | 12.09 ms | 1.00x |
| scatter | 100k | public_api_render | 12.41 ms | 13.51 ms | 1.09x |
| scatter | 100k | save_only | 12.54 ms | 11.92 ms | 0.95x |
| scatter | 100k | public_api_save | 12.35 ms | 12.30 ms | 1.00x |
| scatter | 250k | render_only | 27.62 ms | 27.68 ms | 1.00x |
| scatter | 250k | public_api_render | 28.38 ms | 28.14 ms | 0.99x |
| scatter | 250k | save_only | 27.06 ms | 27.14 ms | 1.00x |
| scatter | 250k | public_api_save | 28.37 ms | 28.29 ms | 1.00x |
| scatter | 500k | render_only | 54.95 ms | 54.19 ms | 0.99x |
| scatter | 500k | public_api_render | 56.51 ms | 56.32 ms | 1.00x |
| scatter | 500k | save_only | 53.88 ms | 54.47 ms | 1.01x |
| scatter | 500k | public_api_save | 56.89 ms | 56.78 ms | 1.00x |
