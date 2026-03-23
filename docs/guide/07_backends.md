# Backend Selection Guide

This guide describes the backend-related APIs exactly as they work in the
current codebase.

## TL;DR

| Goal | What to use today |
|------|-------------------|
| Small or medium PNG export | `Plot::save()` with default settings |
| In-memory render with CPU parallelism | `Plot::render()` plus `features = ["parallel"]` |
| SIMD acceleration | `features = ["parallel", "simd"]` and use `render()` |
| Very large datasets | Let DataShader activate automatically above `100_000` points |
| GPU-accelerated PNG export | Enable `gpu` and call `.gpu(true)` |
| Interactive window | Enable `interactive` or `interactive-gpu` and use `show_interactive()` |
| Lower allocation pressure | `.with_memory_pooling(true)` |

## Important Distinction

There are two separate concepts in the current implementation:

1. **Stored backend selection**
   - `.backend(...)`
   - `.auto_optimize()`
   - `.get_backend_name()`

2. **Actual execution path**
   - `render()`
   - `save()`

The stored backend selection is metadata today. It is visible through
`get_backend_name()`, but the current `render()` and `save()` implementations do
not directly dispatch on `self.render.backend`.

### What `.auto_optimize()` does today

`.auto_optimize()` stores a backend choice based on total point count:

- `< 1_000` points: `Skia`
- `1_000..100_000`: `Parallel` if the `parallel` feature is enabled, otherwise `Skia`
- `>= 100_000`: `GPU` if the `gpu` feature is enabled, otherwise `DataShader`

If you set `.backend(...)` first, `.auto_optimize()` keeps that explicit choice.

```rust
use ruviz::core::plot::BackendType;
use ruviz::prelude::*;

let x = vec![0.0, 1.0, 2.0];
let y = vec![0.0, 1.0, 4.0];

let plot = Plot::new()
    .backend(BackendType::DataShader)
    .line(&x, &y)
    .end_series();

assert_eq!(plot.get_backend_name(), "datashader");
```

## What `render()` actually does

`render()` returns an in-memory `Image` and currently chooses its path like this:

- Above `100_000` points: DataShader
- Otherwise, if the `parallel` feature is enabled:
  - parallel rendering is used when `ParallelRenderer::should_use_parallel(...)` returns `true`
- Otherwise: CPU/tiny-skia rendering

Reactive plots first resolve a static snapshot, then run through the same
backend-selection logic:

- temporal `Signal` inputs in plain `render()` are sampled at `0.0`
- push-based `Observable` inputs and streaming buffers read their latest values
- `render_at(t)` uses the same backend-selection logic after sampling temporal
  inputs at `t`

That means signal-backed, observable-backed, and streaming-backed
line/scatter/bar/error/histogram/box plots can still reach the parallel and
DataShader paths after resolution.

The default parallel renderer activates when either:

- the series count is at least `2`, or
- total points exceed `20_000` (default chunk size `10_000 * 2`)

`parallel_threshold(...)` only adjusts the **series-count** threshold. It does
not change the chunk-size path.

### Parallel render example

```toml
[dependencies]
ruviz = { version = "0.1.5", features = ["parallel"] }
```

```rust
use ruviz::prelude::*;

let x: Vec<f64> = (0..50_000).map(|i| i as f64 * 0.001).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

let image = Plot::new()
    .parallel_threshold(4)
    .line(&x, &y)
    .render()?;

println!("Rendered {}x{}", image.width(), image.height());
```

### SIMD note

The `simd` feature is used inside the parallel renderer. In practice that means
it helps the `render()` path when parallel rendering is active.

```toml
[dependencies]
ruviz = { version = "0.1.5", features = ["parallel", "simd"] }
```

## What `save()` actually does

`save()` renders and writes a PNG file. Its current path is different from
`render()`:

- Above `100_000` points: DataShader branch
- Otherwise, if `gpu(true)` is enabled and the plot has at least `5_000` points:
  - GPU rendering path
- Otherwise: CPU/tiny-skia rendering

Reactive snapshotting works the same as `render()`: temporal `Signal` sources
are sampled at `0.0`, while push-based `Observable` and streaming sources use
their latest values before backend selection.

Two important details:

- `save()` does **not** currently call the dedicated `render_with_parallel()` path
- The DataShader branch in `save()` has explicit fast paths for line, scatter,
  and histogram series; unsupported series fall back to normal rendering inside
  that branch

## DataShader

DataShader activates automatically above `100_000` total points.

```rust
use ruviz::prelude::*;

let points = 250_000;
let x: Vec<f64> = (0..points).map(|i| i as f64 * 0.001).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

// Both render() and save() will switch to DataShader above 100_000 points.
Plot::new()
    .line(&x, &y)
    .save("datashader_plot.png")?;
```

## GPU

GPU support is opt-in and requires the `gpu` feature (or `interactive-gpu`,
which includes it).

Calling `.gpu(true)` does two things:

- it stores `BackendType::GPU` on the plot
- it enables the GPU path in `save()` for plots with at least `5_000` points

If GPU initialization fails during `save()`, the code logs a warning and falls
back to CPU rendering.

```toml
[dependencies]
ruviz = { version = "0.1.5", features = ["gpu"] }
```

```rust
use ruviz::prelude::*;

let x: Vec<f64> = (0..20_000).map(|i| i as f64 * 0.001).collect();
let y: Vec<f64> = x.iter().map(|v| v.cos()).collect();

Plot::new()
    .gpu(true)
    .line(&x, &y)
    .save("gpu_plot.png")?;
```

`render()` does not currently use this GPU path.

## Interactive windows

Interactive support is behind `interactive` or `interactive-gpu`.

The key APIs are:

- `show_interactive(plot)` - convenience async function
- `InteractiveWindowBuilder::build(plot)` - async builder
- `InteractiveWindow::run(plot)` - blocking event loop after the window is built

Because the builder and convenience function are async, your application must
provide an async runtime. `ruviz` does **not** add `tokio` as a normal
dependency for you.

### Self-contained interactive example

```toml
[dependencies]
ruviz = { version = "0.1.5", features = ["interactive"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

```rust
use ruviz::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..200).map(|i| i as f64 * 0.05).collect();
    let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

    let plot = Plot::new()
        .line(&x, &y)
        .title("Interactive Plot")
        .end_series();

    show_interactive(plot).await?;
    Ok(())
}
```

For GPU-backed interactive work, switch the feature flag to `interactive-gpu`
and enable `.gpu(true)` on the plot before `end_series()`.

## Memory pooling

Memory pooling is separate from backend selection and is always opt-in:

```rust
use ruviz::prelude::*;

Plot::new()
    .with_memory_pooling(true)
    .line(&x, &y)
    .save("pooled_plot.png")?;
```

## Recommendations

- Start with plain `save()` or `render()` before setting backend metadata.
- If you need faster in-memory rendering, add `parallel` and use `render()`.
- Add `simd` only alongside `parallel`.
- Use `.gpu(true)` when you want GPU-assisted PNG export or `interactive-gpu`.
- Treat `.backend(...)` and `.auto_optimize()` as stored selection helpers, not
  hard execution guarantees.
