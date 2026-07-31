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
drains already-completed work, updates existing `TextureHandle`s, and
schedules at most one background render plus one coalesced newest request. The
last good texture remains visible during rendering and after errors. Reactive
2D session changes call `egui::Context::request_repaint`.

2D frames are presented as two stacked textures — the plot base and the
interaction overlay — and are never flattened on the CPU for display. A hover,
tooltip, or brush update leaves the base layer untouched, so only the small
overlay texture is re-uploaded. Saving or copying the plot composes the two
layers on demand. All rendering for one widget runs on a single background
thread that is started with its first frame and joined when the widget is
dropped.

Render errors are returned in `response.error` and retained by `last_error()`.
Call `retry_render()` to explicitly retry the unchanged current plot or 3D
view.

Static mode disables input but retains resize, replacement, and reactive
redraws. Interactive 2D mode supports hover, click selection, scroll zoom,
drag pan, right-drag or shift-drag brush selection, Escape/double-click reset,
and release-outside cancellation.

Right-click without dragging opens the plot context menu in both interactive
and static mode. It can reset or fit the view, save the installed frame as PNG,
copy that frame to the clipboard, and enable or disable interaction. These
image actions reuse the last displayed frame rather than scheduling a render.
The native save dialog and atomic PNG write run on a named worker so they do
not block the egui frame.

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
Escape/double-click resets the camera. A 3D context menu also provides
isometric, front, back, left, right, top, and bottom camera views.
`set_plot_keep_view` preserves a user-customized 2D visible view or the 3D
camera where valid. An untouched 2D view uses the replacement plot's natural
bounds.

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
