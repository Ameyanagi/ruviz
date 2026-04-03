# Performance Optimization

Practical performance guidance for the current `ruviz` implementation.

## Start Here

Measure before adding features. The biggest performance win is still running in
release mode:

```bash
cargo run --release
```

Recommended release profile:

```toml
[profile.release]
lto = true
codegen-units = 1
opt-level = 3
```

## Important Distinction

Two APIs matter for performance:

- `render()` returns an in-memory `Image`
- `save()` renders and writes a PNG file

They do **not** currently use identical execution paths.

## Feature Flags

```toml
[dependencies]
ruviz = "0.3.5"
```

Useful opt-in features:

- `parallel`: enables the dedicated parallel `render()` path
- `simd`: accelerates the parallel renderer
- `gpu`: enables `.gpu(true)` and GPU-assisted PNG export
- `performance`: shorthand for `["parallel", "simd"]`

## What `render()` does today

`render()` chooses its execution path from actual plot content:

- Above `100_000` total points for aggregation-safe series such as scatter and histogram:
  - DataShader
- Otherwise, with `parallel` enabled:
  - parallel rendering is used when the internal threshold logic says it is worthwhile
- Otherwise: CPU/tiny-skia rendering

Reactive plots now resolve to a static snapshot first, so push-based and
streaming sources can still benefit from the same parallel/DataShader decisions.

### Parallel rendering

```toml
[dependencies]
ruviz = { version = "0.3.5", features = ["parallel"] }
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

`parallel_threshold(...)` only changes the series-count threshold. It does not
override every internal condition used by the parallel renderer.

### SIMD

```toml
[dependencies]
ruviz = { version = "0.3.5", features = ["parallel", "simd"] }
```

The `simd` feature is used inside the parallel renderer, so it helps when the
parallel `render()` path is active.

## What `save()` does today

`save()` writes PNG output and uses a different path:

- Above `100_000` total points: DataShader branch
- Otherwise, if `.gpu(true)` is enabled and the plot has at least `5_000` points:
  - GPU path
- Otherwise: CPU/tiny-skia rendering

The current `save()` implementation does **not** call the dedicated
`render_with_parallel()` path.

### GPU-assisted PNG export

```toml
[dependencies]
ruviz = { version = "0.3.5", features = ["gpu"] }
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

If GPU initialization fails, `save()` falls back to CPU rendering.

## DataShader

DataShader-style aggregation activates automatically above `100_000` total
points for aggregation-safe series such as scatter and histogram.

```rust
use ruviz::prelude::*;

let points = 250_000;
let x: Vec<f64> = (0..points).map(|i| i as f64 * 0.001).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

Plot::new()
    .scatter(&x, &y)
    .save("datashader_plot.png")?;
```

## Memory Pooling

Memory pooling is opt-in:

```rust
use ruviz::prelude::*;

Plot::new()
    .with_memory_pooling(true)
    .line(&x, &y)
    .save("pooled.png")?;
```

Use it when repeated large renders are spending noticeable time in allocation.

## Practical Recommendations

- Start with `save()` or `render()` before setting backend metadata.
- Use `render()` when you want the in-memory image path and parallel CPU speedups.
- Use `save()` when you want PNG output and optional `.gpu(true)` acceleration.
- Add `simd` only alongside `parallel`.
- Downsample scatter-style plots when visual density is higher than display density.
- Benchmark your real workload instead of relying on generic backend labels.

## Benchmark Template

```rust
use ruviz::prelude::*;
use std::time::Instant;

let start = Instant::now();
let image = Plot::new()
    .line(&x, &y)
    .render()?;
println!(
    "Rendered {}x{} in {:?}",
    image.width(),
    image.height(),
    start.elapsed()
);
```

## Related Guides

- [Backend Selection](07_backends.md)
- [Installation](02_installation.md)
- [Advanced Usage](11_advanced.md)
