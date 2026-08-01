//! Static and interactive native GPUI 3D embeds.
//!
//! Run with:
//! `cargo run --example plot3d_embed --features 3d`

mod support;

use gpui::{
    App, Bounds, Context, Render, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb,
    size,
};
use ruviz::prelude::*;
use ruviz_gpui::{RuvizPlot3D, plot3d_builder};
use support::{application, exit_on_window_open_failure};

struct Plot3DEmbedDemo {
    interactive: gpui::Entity<RuvizPlot3D>,
    static_view: gpui::Entity<RuvizPlot3D>,
}

impl Plot3DEmbedDemo {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let x = [-2.0, -1.0, 0.0, 1.0, 2.0];
        let y = [-2.0, -1.0, 0.0, 1.0, 2.0];
        let z = [
            [0.00, 0.25, 0.50, 0.25, 0.00],
            [0.25, 0.75, 1.00, 0.75, 0.25],
            [0.50, 1.00, 1.50, 1.00, 0.50],
            [0.25, 0.75, 1.00, 0.75, 0.25],
            [0.00, 0.25, 0.50, 0.25, 0.00],
        ];

        let interactive = plot3d_builder(
            surface(&x, &y, &z)
                .title("Interactive: drag, wheel, double-click")
                .xlabel("x")
                .ylabel("y")
                .zlabel("z"),
        )
        .interactive()
        .on_pick(|hit| println!("picked {hit:?}"))
        .on_error(|error| eprintln!("3D plot error: {error}"))
        .build(cx);

        let static_view = plot3d_builder(
            scatter3d(
                &[-1.0, -0.25, 0.5, 1.0],
                &[0.0, 1.0, -0.5, 0.75],
                &[1.0, 0.25, 0.75, -0.5],
            )
            .title("Static 3D")
            .marker_size(9.0),
        )
        .static_view()
        .build(cx);

        Self {
            interactive,
            static_view,
        }
    }
}

impl Render for Plot3DEmbedDemo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .gap_4()
            .p_4()
            .bg(rgb(0xf6f7fb))
            .child(div().flex_1().child(self.interactive.clone()))
            .child(div().flex_1().child(self.static_view.clone()))
    }
}

fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1200.0), px(640.0)), cx);
        exit_on_window_open_failure(
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| Plot3DEmbedDemo::new(window, cx)),
            ),
            "3D embed",
        );
        cx.activate(true);
    });
}
