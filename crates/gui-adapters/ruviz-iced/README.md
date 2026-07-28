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
idle. Escape, release outside, and focus loss cancel an active drag.

`set_plot` resets the viewport; `set_plot_keep_view` restores the current
visible bounds on the replacement. Both retain the last frame until the new
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
authoritative camera.

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

CPU image rendering is the default. `3d-gpu` uses a worker-retained GPU
renderer and reads the result back into a CPU RGBA image for Iced. It is not
zero-copy presentation.

## Examples

```sh
cargo run --manifest-path crates/gui-adapters/ruviz-iced/Cargo.toml --example dashboard_2d
cargo run --manifest-path crates/gui-adapters/ruviz-iced/Cargo.toml --features 3d --example dashboard
cargo run --manifest-path crates/gui-adapters/ruviz-iced/Cargo.toml --features 3d --example static_dashboard
```

The mixed dashboard shows independently retained 2D and 3D widgets in one
native window.
