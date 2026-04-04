# Rust Feature Impact Plotting Benchmarks

This page is generated from the committed Rust feature-impact plotting benchmark reference run.

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

- Captured at: `2026-04-04T01:13:56Z`
- Git commit: `4b6e60b2ec72ce10ca862473387536cde6a26182`
- Git branch: `bench/rust-feature-impact`
- Git worktree dirty: `yes`
- Host OS: `macOS-26.2-arm64-arm-64bit-Mach-O`
- Host machine: `arm64`
- Host processor: `arm`
- CPU count: `10`
- Python: `3.14.2`
- Rust: `rustc 1.94.1 (e408947bf 2026-03-25)`

Raw artifacts:
- [environment.json](./environment.json)
- [results.csv](./results.csv)
- [baseline_cpu.json](./baseline_cpu.json)
- [default.json](./default.json)
- [parallel_only.json](./parallel_only.json)
- [parallel_simd.json](./parallel_simd.json)
- [performance_alias.json](./performance_alias.json)
- [gpu_only.json](./gpu_only.json)

## Rust feature impact (`render_only`)

| Plot | Size | baseline_cpu | default | parallel_only | parallel_simd | performance_alias | gpu_only |
| --- | --- | --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 6.25 ms | 5.84 ms (1.07x) | 5.82 ms (1.07x) | 5.85 ms (1.07x) | 5.82 ms (1.07x) | 5.94 ms (1.05x) |
| heatmap | 2048x2048 | 7.76 ms | 7.64 ms (1.02x) | 7.00 ms (1.11x) | 7.11 ms (1.09x) | 6.99 ms (1.11x) | 8.48 ms (0.92x) |
| heatmap | 512x512 | 5.27 ms | 5.35 ms (0.99x) | 5.30 ms (0.99x) | 5.25 ms (1.00x) | 5.27 ms (1.00x) | 5.51 ms (0.96x) |
| histogram | 100k | 1.45 ms | 1.52 ms (0.96x) | 1.42 ms (1.02x) | 1.45 ms (1.00x) | 1.45 ms (1.00x) | 1.45 ms (1.00x) |
| histogram | 1m | 2.08 ms | 2.09 ms (0.99x) | 2.25 ms (0.92x) | 2.09 ms (1.00x) | 2.16 ms (0.96x) | 2.10 ms (0.99x) |
| histogram | 5m | 5.40 ms | 5.14 ms (1.05x) | 5.12 ms (1.06x) | 5.12 ms (1.05x) | 5.42 ms (1.00x) | 5.13 ms (1.05x) |
| line | 100k | 10.62 ms | 12.65 ms (0.84x) | 12.69 ms (0.84x) | 12.81 ms (0.83x) | 12.85 ms (0.83x) | 9.80 ms (1.08x) |
| line | 1m | 26.69 ms | 30.58 ms (0.87x) | 32.72 ms (0.82x) | 31.41 ms (0.85x) | 31.23 ms (0.85x) | 25.54 ms (1.04x) |
| line | 500k | 20.83 ms | 25.24 ms (0.83x) | 25.12 ms (0.83x) | 24.39 ms (0.85x) | 23.85 ms (0.87x) | 21.96 ms (0.95x) |
| scatter | 100k | 2.27 ms | 2.11 ms (1.08x) | 2.12 ms (1.07x) | 2.12 ms (1.07x) | 2.19 ms (1.04x) | 2.14 ms (1.06x) |
| scatter | 250k | 3.09 ms | 3.09 ms (1.00x) | 3.05 ms (1.01x) | 3.04 ms (1.01x) | 3.12 ms (0.99x) | 3.00 ms (1.03x) |
| scatter | 500k | 4.59 ms | 4.55 ms (1.01x) | 4.52 ms (1.01x) | 4.50 ms (1.02x) | 4.69 ms (0.98x) | 4.54 ms (1.01x) |

## Rust feature impact (`public_api_render`)

| Plot | Size | baseline_cpu | default | parallel_only | parallel_simd | performance_alias | gpu_only |
| --- | --- | --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 9.70 ms | 9.71 ms (1.00x) | 9.50 ms (1.02x) | 9.76 ms (0.99x) | 10.06 ms (0.97x) | 9.72 ms (1.00x) |
| heatmap | 2048x2048 | 20.82 ms | 21.04 ms (0.99x) | 21.02 ms (0.99x) | 21.06 ms (0.99x) | 21.12 ms (0.99x) | 20.89 ms (1.00x) |
| heatmap | 512x512 | 6.15 ms | 6.28 ms (0.98x) | 6.21 ms (0.99x) | 6.18 ms (0.99x) | 6.18 ms (1.00x) | 6.15 ms (1.00x) |
| histogram | 100k | 3.05 ms | 3.15 ms (0.97x) | 3.02 ms (1.01x) | 2.98 ms (1.02x) | 2.99 ms (1.02x) | 2.97 ms (1.03x) |
| histogram | 1m | 19.23 ms | 19.39 ms (0.99x) | 19.32 ms (1.00x) | 19.31 ms (1.00x) | 19.57 ms (0.98x) | 19.97 ms (0.96x) |
| histogram | 5m | 125.50 ms | 124.64 ms (1.01x) | 124.46 ms (1.01x) | 126.70 ms (0.99x) | 126.85 ms (0.99x) | 129.84 ms (0.97x) |
| line | 100k | 10.22 ms | 13.00 ms (0.79x) | 12.99 ms (0.79x) | 14.20 ms (0.72x) | 13.21 ms (0.77x) | 10.31 ms (0.99x) |
| line | 1m | 30.88 ms | 35.16 ms (0.88x) | 35.23 ms (0.88x) | 33.65 ms (0.92x) | 35.82 ms (0.86x) | 30.94 ms (1.00x) |
| line | 500k | 23.74 ms | 26.69 ms (0.89x) | 25.68 ms (0.92x) | 25.66 ms (0.93x) | 25.88 ms (0.92x) | 23.78 ms (1.00x) |
| scatter | 100k | 2.66 ms | 2.49 ms (1.07x) | 2.50 ms (1.06x) | 2.50 ms (1.06x) | 2.59 ms (1.03x) | 2.51 ms (1.06x) |
| scatter | 250k | 4.03 ms | 4.04 ms (1.00x) | 4.03 ms (1.00x) | 3.99 ms (1.01x) | 4.11 ms (0.98x) | 3.98 ms (1.01x) |
| scatter | 500k | 6.53 ms | 6.90 ms (0.95x) | 6.60 ms (0.99x) | 6.59 ms (0.99x) | 6.73 ms (0.97x) | 6.52 ms (1.00x) |

## Rust feature impact (`save_only`)

| Plot | Size | baseline_cpu | default | parallel_only | parallel_simd | performance_alias | gpu_only |
| --- | --- | --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 6.16 ms [skia] | 6.09 ms (1.01x) [skia] | 6.04 ms (1.02x) [skia] | 6.62 ms (0.93x) [skia] | 6.00 ms (1.03x) [skia] | 6.94 ms (0.89x) [gpu] |
| heatmap | 2048x2048 | 7.47 ms [skia] | 7.51 ms (0.99x) [skia] | 7.81 ms (0.96x) [skia] | 7.34 ms (1.02x) [skia] | 7.37 ms (1.01x) [skia] | 8.09 ms (0.92x) [gpu] |
| heatmap | 512x512 | 5.54 ms [skia] | 5.51 ms (1.01x) [skia] | 5.53 ms (1.00x) [skia] | 5.55 ms (1.00x) [skia] | 5.53 ms (1.00x) [skia] | 7.22 ms (0.77x) [gpu] |
| histogram | 100k | 2.96 ms [datashader] | 2.77 ms (1.07x) [datashader] | 2.76 ms (1.07x) [datashader] | 2.76 ms (1.07x) [datashader] | 2.85 ms (1.04x) [datashader] | 2.77 ms (1.07x) [datashader] |
| histogram | 1m | 3.53 ms [datashader] | 3.53 ms (1.00x) [datashader] | 3.55 ms (0.99x) [datashader] | 3.52 ms (1.00x) [datashader] | 4.92 ms (0.72x) [datashader] | 3.55 ms (0.99x) [datashader] |
| histogram | 5m | 7.12 ms [datashader] | 6.46 ms (1.10x) [datashader] | 6.43 ms (1.11x) [datashader] | 7.05 ms (1.01x) [datashader] | 6.59 ms (1.08x) [datashader] | 6.64 ms (1.07x) [datashader] |
| line | 100k | 8.89 ms [skia] | 9.10 ms (0.98x) [skia] | 9.14 ms (0.97x) [skia] | 8.92 ms (1.00x) [skia] | 9.17 ms (0.97x) [skia] | 38.75 ms (0.23x) [gpu] |
| line | 1m | 23.81 ms [skia] | 23.82 ms (1.00x) [skia] | 23.70 ms (1.00x) [skia] | 23.73 ms (1.00x) [skia] | 23.94 ms (0.99x) [skia] | 272.53 ms (0.09x) [gpu] |
| line | 500k | 19.40 ms [skia] | 20.54 ms (0.94x) [skia] | 20.37 ms (0.95x) [skia] | 20.35 ms (0.95x) [skia] | 19.74 ms (0.98x) [skia] | 141.43 ms (0.14x) [gpu] |
| scatter | 100k | 3.66 ms [datashader] | 3.31 ms (1.10x) [datashader] | 3.34 ms (1.10x) [datashader] | 3.34 ms (1.10x) [datashader] | 3.45 ms (1.06x) [datashader] | 3.35 ms (1.09x) [datashader] |
| scatter | 250k | 4.14 ms [datashader] | 4.07 ms (1.02x) [datashader] | 4.13 ms (1.00x) [datashader] | 4.10 ms (1.01x) [datashader] | 4.26 ms (0.97x) [datashader] | 4.53 ms (0.91x) [datashader] |
| scatter | 500k | 5.23 ms [datashader] | 5.48 ms (0.96x) [datashader] | 5.74 ms (0.91x) [datashader] | 5.87 ms (0.89x) [datashader] | 5.40 ms (0.97x) [datashader] | 5.28 ms (0.99x) [datashader] |

## Rust feature impact (`public_api_save`)

| Plot | Size | baseline_cpu | default | parallel_only | parallel_simd | performance_alias | gpu_only |
| --- | --- | --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | 9.79 ms [skia] | 10.03 ms (0.98x) [skia] | 10.50 ms (0.93x) [skia] | 9.84 ms (0.99x) [skia] | 9.86 ms (0.99x) [skia] | 10.78 ms (0.91x) [gpu] |
| heatmap | 2048x2048 | 21.11 ms [skia] | 21.21 ms (1.00x) [skia] | 21.31 ms (0.99x) [skia] | 21.70 ms (0.97x) [skia] | 22.50 ms (0.94x) [skia] | 22.12 ms (0.95x) [gpu] |
| heatmap | 512x512 | 6.37 ms [skia] | 6.42 ms (0.99x) [skia] | 6.38 ms (1.00x) [skia] | 6.39 ms (1.00x) [skia] | 6.45 ms (0.99x) [skia] | 7.92 ms (0.80x) [gpu] |
| histogram | 100k | 4.76 ms [datashader] | 4.30 ms (1.11x) [datashader] | 4.25 ms (1.12x) [datashader] | 4.29 ms (1.11x) [datashader] | 4.55 ms (1.05x) [datashader] | 4.39 ms (1.08x) [datashader] |
| histogram | 1m | 20.80 ms [datashader] | 20.87 ms (1.00x) [datashader] | 21.12 ms (0.98x) [datashader] | 21.87 ms (0.95x) [datashader] | 20.76 ms (1.00x) [datashader] | 21.50 ms (0.97x) [datashader] |
| histogram | 5m | 127.54 ms [datashader] | 126.04 ms (1.01x) [datashader] | 126.26 ms (1.01x) [datashader] | 129.05 ms (0.99x) [datashader] | 128.96 ms (0.99x) [datashader] | 130.38 ms (0.98x) [datashader] |
| line | 100k | 9.62 ms [skia] | 9.42 ms (1.02x) [skia] | 9.44 ms (1.02x) [skia] | 9.55 ms (1.01x) [skia] | 9.56 ms (1.01x) [skia] | 40.39 ms (0.24x) [gpu] |
| line | 1m | 28.32 ms [skia] | 26.83 ms (1.06x) [skia] | 26.75 ms (1.06x) [skia] | 27.74 ms (1.02x) [skia] | 27.37 ms (1.03x) [skia] | 279.02 ms (0.10x) [gpu] |
| line | 500k | 21.13 ms [skia] | 21.47 ms (0.98x) [skia] | 21.19 ms (1.00x) [skia] | 22.09 ms (0.96x) [skia] | 21.89 ms (0.97x) [skia] | 143.87 ms (0.15x) [gpu] |
| scatter | 100k | 4.07 ms [datashader] | 3.68 ms (1.11x) [datashader] | 3.72 ms (1.09x) [datashader] | 3.70 ms (1.10x) [datashader] | 3.84 ms (1.06x) [datashader] | 3.71 ms (1.09x) [datashader] |
| scatter | 250k | 5.02 ms [datashader] | 5.00 ms (1.00x) [datashader] | 5.04 ms (1.00x) [datashader] | 5.01 ms (1.00x) [datashader] | 5.77 ms (0.87x) [datashader] | 5.10 ms (0.98x) [datashader] |
| scatter | 500k | 7.17 ms [datashader] | 7.51 ms (0.95x) [datashader] | 7.24 ms (0.99x) [datashader] | 7.24 ms (0.99x) [datashader] | 7.43 ms (0.96x) [datashader] | 7.17 ms (1.00x) [datashader] |

## `parallel_simd` vs `performance_alias`

These rows should remain near parity because `performance` currently aliases `parallel + simd`.

| Plot | Size | Boundary | parallel_simd | performance_alias | Alias ratio |
| --- | --- | --- | --- | --- | --- |
| heatmap | 1024x1024 | render_only | 5.85 ms | 5.82 ms | 0.99x |
| heatmap | 1024x1024 | public_api_render | 9.76 ms | 10.06 ms | 1.03x |
| heatmap | 1024x1024 | save_only | 6.62 ms | 6.00 ms | 0.91x |
| heatmap | 1024x1024 | public_api_save | 9.84 ms | 9.86 ms | 1.00x |
| heatmap | 2048x2048 | render_only | 7.11 ms | 6.99 ms | 0.98x |
| heatmap | 2048x2048 | public_api_render | 21.06 ms | 21.12 ms | 1.00x |
| heatmap | 2048x2048 | save_only | 7.34 ms | 7.37 ms | 1.00x |
| heatmap | 2048x2048 | public_api_save | 21.70 ms | 22.50 ms | 1.04x |
| heatmap | 512x512 | render_only | 5.25 ms | 5.27 ms | 1.00x |
| heatmap | 512x512 | public_api_render | 6.18 ms | 6.18 ms | 1.00x |
| heatmap | 512x512 | save_only | 5.55 ms | 5.53 ms | 1.00x |
| heatmap | 512x512 | public_api_save | 6.39 ms | 6.45 ms | 1.01x |
| histogram | 100k | render_only | 1.45 ms | 1.45 ms | 1.00x |
| histogram | 100k | public_api_render | 2.98 ms | 2.99 ms | 1.00x |
| histogram | 100k | save_only | 2.76 ms | 2.85 ms | 1.03x |
| histogram | 100k | public_api_save | 4.29 ms | 4.55 ms | 1.06x |
| histogram | 1m | render_only | 2.09 ms | 2.16 ms | 1.03x |
| histogram | 1m | public_api_render | 19.31 ms | 19.57 ms | 1.01x |
| histogram | 1m | save_only | 3.52 ms | 4.92 ms | 1.40x |
| histogram | 1m | public_api_save | 21.87 ms | 20.76 ms | 0.95x |
| histogram | 5m | render_only | 5.12 ms | 5.42 ms | 1.06x |
| histogram | 5m | public_api_render | 126.70 ms | 126.85 ms | 1.00x |
| histogram | 5m | save_only | 7.05 ms | 6.59 ms | 0.94x |
| histogram | 5m | public_api_save | 129.05 ms | 128.96 ms | 1.00x |
| line | 100k | render_only | 12.81 ms | 12.85 ms | 1.00x |
| line | 100k | public_api_render | 14.20 ms | 13.21 ms | 0.93x |
| line | 100k | save_only | 8.92 ms | 9.17 ms | 1.03x |
| line | 100k | public_api_save | 9.55 ms | 9.56 ms | 1.00x |
| line | 1m | render_only | 31.41 ms | 31.23 ms | 0.99x |
| line | 1m | public_api_render | 33.65 ms | 35.82 ms | 1.06x |
| line | 1m | save_only | 23.73 ms | 23.94 ms | 1.01x |
| line | 1m | public_api_save | 27.74 ms | 27.37 ms | 0.99x |
| line | 500k | render_only | 24.39 ms | 23.85 ms | 0.98x |
| line | 500k | public_api_render | 25.66 ms | 25.88 ms | 1.01x |
| line | 500k | save_only | 20.35 ms | 19.74 ms | 0.97x |
| line | 500k | public_api_save | 22.09 ms | 21.89 ms | 0.99x |
| scatter | 100k | render_only | 2.12 ms | 2.19 ms | 1.03x |
| scatter | 100k | public_api_render | 2.50 ms | 2.59 ms | 1.03x |
| scatter | 100k | save_only | 3.34 ms | 3.45 ms | 1.03x |
| scatter | 100k | public_api_save | 3.70 ms | 3.84 ms | 1.04x |
| scatter | 250k | render_only | 3.04 ms | 3.12 ms | 1.03x |
| scatter | 250k | public_api_render | 3.99 ms | 4.11 ms | 1.03x |
| scatter | 250k | save_only | 4.10 ms | 4.26 ms | 1.04x |
| scatter | 250k | public_api_save | 5.01 ms | 5.77 ms | 1.15x |
| scatter | 500k | render_only | 4.50 ms | 4.69 ms | 1.04x |
| scatter | 500k | public_api_render | 6.59 ms | 6.73 ms | 1.02x |
| scatter | 500k | save_only | 5.87 ms | 5.40 ms | 0.92x |
| scatter | 500k | public_api_save | 7.24 ms | 7.43 ms | 1.03x |
