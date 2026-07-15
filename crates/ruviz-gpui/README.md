# ruviz-gpui

`ruviz-gpui` is the native GPUI adapter for embedding `ruviz` plots inside a
desktop GPUI application.

It keeps `ruviz` plot construction and interaction behavior, while letting GPUI
own the window, layout tree, and surrounding application shell.

## Install

```toml
[dependencies]
ruviz = "0.4.20"
ruviz-gpui = "0.4.20"
```

## What This Crate Provides

- an embeddable GPUI plot view for static and interactive plots
- configurable image and hybrid presentation modes
- pan, zoom, hover, selection, and context-menu integration
- absolute-window coordinate conversion and frame-aware click/hover callbacks
- PNG save and clipboard-copy actions routed through the host platform

## Coordinates and pointer callbacks

`RuvizPlot::data_at` maps an absolute GPUI window `Point<Pixels>` into the
currently displayed data coordinates. `RuvizPlot::screen_at` performs the inverse
mapping and also returns an absolute window point. Both return `Ok(None)` before a
displayed layout is available or when the requested position is out of bounds;
applications do not need to estimate plot margins or presentation scaling.

Click and hover handlers receive the same `PlotPointerEvent` payload, including
the absolute window position, backing-frame viewport position, optional data
position, displayed viewport snapshot, and frame-aware `HitResult`:

```rust
let plot = plot_builder(plot)
    .on_plot_click(|event| {
        println!("click: data={:?}, hit={:?}", event.data_position, event.hit);
    })
    .on_plot_hover(|event| {
        println!("hover: window={:?}, data={:?}", event.window_position, event.data_position);
    })
    .build(cx);
```

Builder callbacks are convenient for simple thread-safe observers. Host views
that update GPUI state should normally subscribe to the plot entity so the
handler receives a usable GPUI context:

```rust,ignore,reason=gpui-host-subscription
let plot = plot_builder(plot).build(cx);
let subscription = cx.subscribe(&plot, |this, _plot, event, cx| {
    this.last_pointer_event = Some(event.clone());
    cx.notify();
});
```

Keep the returned subscription alive with the host view. `RuvizPlot` emits click
and hover events to GPUI subscribers in addition to invoking builder callbacks.

Click events run on a primary-button release for a non-drag gesture. With normal
platform double-click delivery, the completed click-count 1 release may emit
before click-count 2 is known; the click-count 2 release emits no additional
click, so the complete sequence produces at most one click event.
Built-in hover and tooltip processing continues alongside hover callbacks.

## Platform Notes

`ruviz-gpui` currently supports:

- macOS
- Linux
- Windows

On Linux the crate uses GTK-backed native dialogs. Install GTK3 development
headers before building desktop examples.

## Examples

Runnable examples live in the crate:

```sh
cargo run -p ruviz-gpui --example static_embed
cargo run -p ruviz-gpui --example observable_embed
cargo run -p ruviz-gpui --example streaming_embed
cargo run -p ruviz-gpui --example coordinate_events
```

## Related Docs

- Root crate docs: <https://docs.rs/ruviz>
- Repository README: <https://github.com/Ameyanagi/ruviz/blob/main/README.md>
- GPUI example directory: <https://github.com/Ameyanagi/ruviz/tree/main/crates/ruviz-gpui/examples>
- Release notes: <https://github.com/Ameyanagi/ruviz/tree/main/docs/releases>
