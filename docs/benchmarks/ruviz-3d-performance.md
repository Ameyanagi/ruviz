# ruviz 3D local performance observations

This page records one retained-frame quick run from 2026-07-24 local time. The
raw source of truth is
[`ruviz-3d-performance-local-2026-07-24.json`](ruviz-3d-performance-local-2026-07-24.json)
(SHA-256
`004e4ec4640c0cf3d88793307bf676108620fd628457c03d5cebc9122ec0b83d`).
It follows the checked-in
[`3D performance artifact v1` schema](../../scripts/ruviz_3d_performance.schema.json).

These values are local observations, not a fixed-hardware release gate or a
cross-platform performance claim. The run used an Apple M4, arm64 macOS
25.5.0, Rust 1.94.1, the native wgpu GPU path on Metal, release optimization,
features `3d,gpu`, and a 640x480 viewport. The worktree was dirty and other
local activity was not controlled. Every performance gate in the artifact is
therefore `not_evaluated`.

## Observed retained-frame timings

Each row contains the median and empirical p95/p99 in milliseconds. The
generator derives these percentiles from the ten raw Criterion samples stored
in the artifact; it also preserves Criterion's own estimates and confidence
intervals.

| Backend and boundary | Dataset | Median | p95 | p99 |
| --- | --- | ---: | ---: | ---: |
| CPU unchanged warm frame | 100K scatter | 49.723 ms | 65.999 ms | 72.557 ms |
| CPU unchanged warm frame | 100x100 surface | 27.410 ms | 34.516 ms | 37.055 ms |
| CPU camera-only update | 100K scatter | 42.782 ms | 45.764 ms | 46.760 ms |
| CPU camera-only update | 100x100 surface | 33.360 ms | 47.099 ms | 47.813 ms |
| GPU unchanged warm frame, no readback | 100K scatter | 8.338 ms | 12.136 ms | 12.263 ms |
| GPU unchanged warm frame, no readback | 100x100 surface | 2.150 ms | 2.602 ms | 2.701 ms |
| GPU camera-only update, no readback | 100K scatter | 6.482 ms | 7.684 ms | 7.709 ms |
| GPU camera-only update, no readback | 100x100 surface | 2.052 ms | 2.690 ms | 2.692 ms |

The two datasets are tied to the committed manifest hashes
`fnv1a64:563cdc01ad1eeb73` and `fnv1a64:354d9b31c0459a42`.
The retained GPU measurements submit and wait for GPU completion without image
composition or pixel readback. The CPU measurements render the retained
interactive-quality frame.

## Reproduction

From the repository root, run:

```sh
python3 scripts/generate_3d_performance_artifact.py \
  --run \
  --bench-filter retained \
  --output /tmp/ruviz-3d-retained-quick.json
```

That invokes the recorded benchmark command:

```sh
cargo bench --bench three_d --features 3d,gpu -- retained
```

The generator includes only Criterion files changed by that invocation. It
records the command, timestamps, feature set, git state, toolchain, host,
dataset-manifest hash, and SHA-256 hashes for each accepted `benchmark.json`,
`sample.json`, and `estimates.json` input. Run without `--bench-filter` for all
currently integrated cold and retained boundaries; set `--full` for the larger
CPU retained datasets.

## Deliberately unmeasured

The retained-only run measured 4 of 13 required boundary profiles. It did not
measure:

- CPU cold scene compilation or full export;
- CPU or GPU style-only and data-update frames;
- GPU cold adapter/pipeline creation or cold geometry upload/first frame;
- an independent GPU readback boundary;
- backend, upload, resource, draw-call, and readback diagnostic sidecars;
- the full 1M/10M scatter and 512x512/1024x1024 surface matrix at 800x600;
- 10K-frame host-memory growth, matched GLMakie comparison, or Vulkan, DX12,
  WebGPU, and other cross-vendor evidence.

Criterion's timing artifacts do not contain `RenderDiagnostics3D`, so the raw
artifact marks those diagnostics `unmeasured` instead of inferring them. The 11
fixed-hardware, stability, memory, and competitive gates remain
`not_evaluated` even where a smaller local observation exists.
