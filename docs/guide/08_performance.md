# Performance Optimization

Practical performance guidance for the current `ruviz` implementation.

## Start Here

Use release builds before adding feature flags:

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

Benchmark with your actual plot content. Marker-heavy scatter plots, text, high
DPI output, and very large canvases stress different parts of the renderer.

## Public Output Paths

The public static-output APIs are conservative today:

- `render()` returns an in-memory `Image`.
- `render_png_bytes()` returns PNG bytes.
- `save()` writes PNG bytes to disk.
- `export_svg()` and `render_to_svg()` use the SVG renderer.
- `save_pdf()` is available with the `pdf` feature and converts SVG to PDF.

For PNG/image output, the current public path uses the reference raster pipeline
for output parity. `.backend(...)`, `.auto_optimize()`, `.parallel_threshold(...)`,
and `.gpu(true)` store configuration or metadata; they should not be documented
as hard execution guarantees for `render()` or `save()`.

Reactive plots are resolved to a static snapshot first. Plain `render()` and
`save()` sample temporal `Signal` sources at `0.0`; `render_at(t)` samples them
at `t`.

## Feature Flags

```toml
[dependencies]
ruviz = "0.4.13"
```

Useful opt-in features:

- `parallel`: enables the internal parallel renderer and backend metadata.
- `simd`: enables SIMD support used by performance-oriented code paths.
- `performance`: shorthand for `["parallel", "simd"]`.
- `gpu`: enables GPU types and `.gpu(true)` metadata.

These features can increase compile time and dependency surface. Add them when a
measured code path benefits from them.

## Large Datasets

Large plots are still valid through the normal API:

```rust
use ruviz::prelude::*;

let points = 250_000;
let x: Vec<f64> = (0..points).map(|i| i as f64 * 0.001).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

Plot::new()
    .scatter(&x, &y)
    .save("large_scatter.png")?;
```

For dense plots where many data points map to the same pixels, downsampling or
aggregating before plotting is often more useful than enabling a backend flag.

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

## Recommendations

- Start with plain `save()` or `render()`.
- Use release builds for any timing comparison.
- Prefer smaller canvases and explicit data reduction when visual density is too high.
- Use `.size(width_in, height_in)` plus `.dpi(...)` for print output.
- Treat backend names as metadata unless a specific API documents otherwise.

## Related Guides

- [Backend Selection](07_backends.md)
- [Installation](02_installation.md)
- [Advanced Usage](11_advanced.md)
