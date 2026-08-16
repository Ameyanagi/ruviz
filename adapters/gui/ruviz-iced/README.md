# ruviz-iced

Native, image-backed Iced 0.14 widgets for ruviz. The application owns plot
state in the usual Elm model; no rendering occurs in `view`, layout, or paint
callbacks.

## 2D

```rust
use iced::{Element, Subscription, Task};
use ruviz::prelude::Plot;
use ruviz_iced::{Message as PlotMessage, PlotState, plot};

struct App {
    plot: PlotState,
}

enum Message {
    Plot(PlotMessage),
}

fn new() -> (App, Task<Message>) {
    let mut plot = PlotState::interactive(
        Plot::new().line(&[0.0, 1.0], &[1.0, 2.0]),
    )
    .fill();
    let initial = plot.request_render().into_task().map(Message::Plot);
    (App { plot }, initial)
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::Plot(message) => {
            let update = app.plot.update(message);
            // Inspect update.event() for selection, reset, or render errors.
            update.into_task().map(Message::Plot)
        }
    }
}

fn subscription(app: &App) -> Subscription<Message> {
    app.plot.subscription().map(Message::Plot)
}

fn view(app: &App) -> Element<'_, Message> {
    let plot: Element<'_, PlotMessage> = plot(&app.plot).into();
    plot.map(Message::Plot)
}
```

Use `PlotState::static_view` when the plot should resize but ignore input.
Interactive 2D supports hover, click selection, left/middle drag pan, scroll
zoom, right-drag rectangular zoom, double-click reset, and Escape reset when
idle. A right-click without a drag opens the plot menu with reset, fit, PNG
save, native image copy, and interaction controls. Escape, release outside,
and focus loss cancel an active drag.

Static plots also expose the menu, so their presented image can be saved or
copied. “Enable interaction” turns gestures on without replacing the retained
state; disabling interaction leaves the menu available so it can be enabled
again. The Windows Menu key and Shift+F10 open the menu while the plot is
hovered.

`set_plot` resets the viewport; `set_plot_keep_view` restores the old visible
bounds only when the user customized them. An untouched view uses the
replacement plot's natural bounds. Both retain the last frame until the new
one has rendered and been allocated by Iced.

## 3D

Enable `3d`, then use `Plot3DState::interactive` and `plot3d` with the same
message/task pattern:

```rust
# use ruviz_iced::Plot3DState;
let plot = Plot3DState::interactive(
    ruviz::scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0]),
)?;
# Ok::<_, ruviz::core::PlottingError>(plot)
```

Interactive 3D supports left-drag orbit, middle/right-drag pan, scroll zoom,
click picking, reset, and release-outside cancellation. Camera changes are
reported as `Event::CameraChanged`; `set_plot_keep_view` keeps the
authoritative camera. Its right-click menu adds an Iced-native Camera view
submenu with isometric, front, back, left, right, top, and bottom views.

Menu opening and export do not request a plot render. Save and copy operate on
the exact RGBA frame already presented by Iced. Right-drag remains a
full-resolution gesture; no reduced-resolution interaction preview is used.

## Choosing an Iced renderer (required)

`ruviz-iced` depends on `iced` with `default-features = false` so that the
application, not the adapter, picks the renderer. That means **your** crate has
to enable it:

```toml
[dependencies]
iced = { version = "0.14", features = ["wgpu", "crisp"] }
ruviz-iced = "0.9.0"
```

- `wgpu` — GPU presentation. Without it Iced falls back to the `tiny-skia` CPU
  renderer, which rescales the whole plot image on the CPU into the fitted
  content rectangle on every frame. At HiDPI sizes that alone costs more than
  the plot render.
- `crisp` — pixel-snapped drawing, which keeps axes, gridlines, and text sharp
  instead of resampled.
- Keep `tiny-skia` as well if you want a software fallback; Iced prefers `wgpu`
  and falls back automatically when no adapter is available.

Iced's own default feature set contains `wgpu`, `crisp`, and `tiny-skia`, so a
plain `iced = "0.14"` dependency already has all three.

## Sizing, HiDPI, and rendering

- `.fill()` uses available Iced layout space;
  `.fixed_pixels(width, height)` uses logical pixels (`.fixed` is an alias).
- Fractional scale factors round physical backing dimensions up and pointer
  input is mapped through the exact fitted image rectangle.
- Raw Iced image handles always receive straight RGBA. Premultiplied ruviz
  images are converted explicitly.
- A latest-request scheduler permits at most one render in flight and
  coalesces pending requests. Old task completions, old plot incarnations, and
  superseded render stamps cannot replace the current frame.
- Reactive 2D changes wake Iced through a capacity-one coalescing
  subscription.
- 2D frames are presented as two stacked images: the plot base and, when a
  crosshair, tooltip, selection, or dynamic annotation is active, an overlay
  drawn over it in the same fitted rectangle. The blend happens in the
  renderer, not on the CPU. A hover that only changes the overlay reuses the
  existing base allocation, so nothing re-uploads the full plot image. Save
  PNG and Copy image flatten the two layers on demand, off the UI thread.
- Hover moves that arrive while a render is in flight are coalesced, not
  dropped: the newest pointer position is replayed against the next current
  frame. No hover event is ever derived from stale geometry.
- Wheel input is normalized to logical pixels and clamped per event, so a
  trackpad flick is one proportional zoom step instead of several.

CPU image rendering is the default. `3d-gpu` uses a worker-retained GPU
renderer and reads the result back into a CPU RGBA image for Iced. It is not
zero-copy presentation.

## Examples

```sh
cargo run --manifest-path adapters/gui/ruviz-iced/Cargo.toml --example dashboard_2d
cargo run --manifest-path adapters/gui/ruviz-iced/Cargo.toml --features 3d --example dashboard
cargo run --manifest-path adapters/gui/ruviz-iced/Cargo.toml --features 3d --example static_dashboard
```

The mixed dashboard shows independently retained 2D and 3D widgets in one
native window.
