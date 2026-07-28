# ruviz-egui

App-owned retained egui widgets for static and interactive ruviz plots. The
library depends only on `egui`; it does not choose an application shell.
`eframe` is a development dependency used by the examples.

```rust
use ruviz::core::Plot;
use ruviz_egui::plot_builder;

let mut plot = plot_builder(
    Plot::new()
        .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 0.0])
        .title("Interactive"),
)
.interactive()
.fill()
.prefer_gpu(false) // optional; presentation is still image-backed
.build();

egui::CentralPanel::default().show(ctx, |ui| {
    let response = plot.show(ui);
    if let Some(hit) = response.clicked {
        println!("{hit:?}");
    }
});
```

`RuvizPlot::show` never renders an image. It translates current egui input,
drains already-completed work, updates an existing `TextureHandle`, and
schedules at most one background render plus one coalesced newest request. The
last good texture remains visible during rendering and after errors. Reactive
2D session changes call `egui::Context::request_repaint`.

Render errors are returned in `response.error` and retained by `last_error()`.
Call `retry_render()` to explicitly retry the unchanged current plot or 3D
view.

Static mode disables input but retains resize, replacement, and reactive
redraws. Interactive 2D mode supports hover, click selection, scroll zoom,
drag pan, shift-drag brush selection, Escape/double-click reset, and
release-outside cancellation.

Pointer input is accepted only over the visible fitted image. Wheel zoom is
claimed by the plot so a containing egui scroll area does not scroll at the
same time.

## 3D

Enable the `3d` feature:

```rust
let mut plot = ruviz_egui::plot3d_builder(
    ruviz::surface(&x, &y, &z).title("Surface"),
)
.interactive()
.fixed_pixels(640.0, 360.0)
.build()?;

let response = plot.show(ui);
if let Some(hit) = response.picked {
    println!("{hit:?}");
}
```

Left-drag orbits, middle/right-drag pans, the wheel zooms, click picks, and
Escape/double-click resets the camera. `set_plot_keep_view` preserves the 2D
visible bounds or 3D camera where valid.

The image upload explicitly converts ruviz RGBA pixels to egui's
premultiplied-alpha representation. Pointer coordinates are mapped through the
actual fitted image rectangle at the current fractional HiDPI scale.

The `gpu` and `3d-gpu` features forward ruviz's feature graph. Presentation
always remains image-backed. The `gpu` feature asks the retained 2D session to
use ruviz's diagnosed GPU path where supported. With `3d-gpu`, a dedicated
worker retains the GPU renderer across requests, reads each completed frame
back to CPU memory, and then uploads it to egui. This is not direct or
zero-copy GPU presentation. The default and plain `3d` feature use the
software renderer.

Run the examples with:

```text
cargo run --manifest-path crates/gui-adapters/ruviz-egui/Cargo.toml --example dashboard
cargo run --manifest-path crates/gui-adapters/ruviz-egui/Cargo.toml --features 3d --example three_d
cargo run --manifest-path crates/gui-adapters/ruviz-egui/Cargo.toml --features 3d --example mixed_dashboard
```
