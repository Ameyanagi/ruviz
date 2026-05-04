# Backend Selection Guide

This guide describes the backend-related APIs exactly as they work in the
current codebase.

## TL;DR

| Goal | What to use today |
|------|-------------------|
| Small or medium PNG export | `Plot::save()` with default settings |
| In-memory render | `Plot::render()` with the public reference raster path |
| PNG export | `Plot::save()` with the public reference raster path |
| Backend metadata | `.backend(...)`, `.auto_optimize()`, `.get_backend_name()` |
| Experimental optimized paths | Internal/test-only paths and lower-level renderer code |
| Interactive window | Enable `interactive` or `interactive-gpu` and use `show_interactive()` |
| Embedded GPUI interactive plot | Use the `ruviz-gpui` crate and `plot_builder(...).interactive()` |
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
`get_backend_name()`, but the current public `render()`, `render_png_bytes()`,
and `save()` implementations do not directly dispatch on `self.render.backend`.

## GPUI Embedded Interactive Backend

`ruviz-gpui` is the embedded interactive adapter for GPUI applications. It uses
the same shared `InteractivePlotSession` core as the standalone winit window,
so the main interaction behaviors now line up closely:

The desktop-supported targets for this adapter are Linux, macOS, and Windows.
The recommended Windows target is `x86_64-pc-windows-msvc`.

- left drag pans
- right drag performs box zoom
- right click opens a built-in context menu
- `Shift + left drag` keeps GPUI brush selection available
- `Cmd/Ctrl+S` saves PNG
- `Cmd/Ctrl+C` copies the current visible plot image

The built-in GPUI context menu includes:

- `Reset View`
- `Set Current View As Home`
- `Go To Home View`
- `Save PNG...`
- `Copy Image`
- `Copy Cursor Coordinates`
- `Copy Visible Bounds`

Host applications can also trigger the same built-in actions directly from the
`RuvizPlot` runtime methods, so they are not limited to the right-click menu.

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

`render()` returns an in-memory `Image` using the public reference raster path.
That path is CPU/tiny-skia based and is kept conservative for output parity.

Reactive plots first resolve a static snapshot, then run through the same
public render path:

- temporal `Signal` inputs in plain `render()` are sampled at `0.0`
- push-based `Observable` inputs and streaming buffers read their latest values
- `render_at(t)` samples temporal inputs at `t`

### In-memory render example

```toml
[dependencies]
ruviz = { version = "0.4.15", features = ["parallel"] }
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

`parallel_threshold(...)` and the `parallel` feature configure the stored
parallel renderer, but they do not force the public reference render path to use
parallel execution.

### SIMD note

The `simd` feature is available to performance-oriented renderer code, but it is
not a guarantee that a public `render()` call will take a SIMD path.

```toml
[dependencies]
ruviz = { version = "0.4.15", features = ["parallel", "simd"] }
```

## What `save()` actually does

`save()` renders PNG bytes through the same public reference raster path and
writes them to the requested file.

Reactive snapshotting works the same as `render()`: temporal `Signal` sources are
sampled at `0.0`, while push-based `Observable` and streaming sources use their
latest values before output.

Two important details:

- `save()` does **not** currently call the dedicated `render_with_parallel()` path
- `save()` does **not** currently dispatch on `.gpu(true)` or stored backend metadata

## DataShader

DataShader support exists in the crate, and `.auto_optimize()` may store
`BackendType::DataShader` metadata for large datasets. The current public
`render()` and `save()` paths do not automatically switch to DataShader output.

```rust
use ruviz::prelude::*;

let points = 250_000;
let x: Vec<f64> = (0..points).map(|i| i as f64 * 0.001).collect();
let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();

Plot::new()
    .scatter(&x, &y)
    .save("datashader_plot.png")?;
```

## GPU

GPU types and metadata are opt-in and require the `gpu` feature (or
`interactive-gpu`, which includes it).

Calling `.gpu(true)` does two things:

- it stores `BackendType::GPU` on the plot
- it records the GPU preference for APIs that inspect plot configuration

The current public `save()` and `render()` paths do not dispatch to a GPU raster
path.

```toml
[dependencies]
ruviz = { version = "0.4.15", features = ["gpu"] }
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

Use `interactive-gpu` for GPU-capable interactive sessions. Static file export
should be treated as CPU/reference output today.

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
ruviz = { version = "0.4.15", features = ["interactive"] }
tokio = { version = "1", features = ["rt", "macros"] }
```

```rust
use ruviz::prelude::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
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

On macOS, keep `show_interactive(...)` on the main/current-thread runtime. `winit`
window creation can stall if the interactive event loop is started from a worker
thread.

Linux and Windows use the standard current-thread runtime path; no additional
main-thread restriction is required beyond a local GUI session.

For GPU-backed interactive work, switch the feature flag to `interactive-gpu`
and enable `.gpu(true)` on the plot before `end_series()`.

Curated examples in this repository:

- `cargo run --features interactive --example basic_interaction`
- `cargo run --features interactive --example interactive_multi_series`
- `cargo run --features interactive --example interactive_scatter_clusters`
- `cargo run --features interactive --example interactive_heatmap`
- `cargo run --features interactive --example data_brushing`
- `cargo run --features interactive --example real_time_performance`

Current window controls:

- `Mouse wheel`: zoom in/out under the cursor
- `Left click + drag`: pan
- `Right click`: open the context menu
- `Right click + drag`: box zoom
- `Escape`: close the menu first, then reset the view
- `Cmd/Ctrl+S`: save the visible viewport as PNG
- `Cmd/Ctrl+C`: copy the visible viewport as an image

The built-in context menu includes:

- `Reset View`
- `Set Current View As Home`
- `Go To Home View`
- `Save PNG...`
- `Copy Image`
- `Copy Cursor Coordinates`
- `Copy Visible Bounds`

You can add custom menu items with `InteractiveWindowBuilder`:

```rust
use ruviz::prelude::*;

let plot = Plot::new()
    .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.5])
    .title("Interactive Plot")
    .end_series();

let menu = InteractiveContextMenuConfig {
    custom_items: vec![InteractiveContextMenuItem::new("export-csv", "Export CSV")],
    ..Default::default()
};

let window = InteractiveWindowBuilder::new()
    .context_menu(menu)
    .on_context_menu_action(|context| {
        if context.action_id == "export-csv" {
            println!("export from bounds: {:?}", context.visible_bounds);
        }
        Ok(())
    })
    .build(plot.clone())
    .await?;

window.run(plot)?;
```

Animation examples live behind the separate `animation` feature:

- `cargo run --features animation --example animation_basic`
- `cargo run --features animation --example animation_simple`
- `cargo run --features animation --example animation_wave`
- `cargo run --features animation --example animation_easing`
- `cargo run --features animation --example animation_reactive`
- `cargo run --features animation --example generate_animation_gallery`

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
- Add `parallel`/`simd` only after benchmarking a path that uses them.
- Use `.gpu(true)` for GPU metadata or GPU-capable interactive work, not as a
  static PNG export guarantee.
- Treat `.backend(...)` and `.auto_optimize()` as stored selection helpers, not
  hard execution guarantees.
