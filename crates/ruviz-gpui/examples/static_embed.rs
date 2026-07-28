mod support;

use gpui::{
    App, Bounds, Context, Render, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb,
    size,
};
use ruviz::prelude::*;
use ruviz_gpui::{RuvizPlot, plot_builder};
use support::{application, exit_on_window_open_failure};

struct StaticEmbedDemo {
    plot: gpui::Entity<RuvizPlot>,
}

impl StaticEmbedDemo {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let x: Vec<f64> = (0..200).map(|i| i as f64 * 0.05).collect();
        let y: Vec<f64> = x.iter().map(|value| value.sin()).collect();
        let plot: Plot = Plot::new()
            .line(&x, &y)
            .title("Static GPUI Embed")
            .xlabel("x")
            .ylabel("sin(x)")
            .into();

        let plot = plot_builder(plot).static_view().build(cx);
        Self { plot }
    }
}

impl Render for StaticEmbedDemo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .bg(rgb(0xf6f7fb))
            .child(self.plot.clone())
    }
}

fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(960.0), px(640.0)), cx);
        exit_on_window_open_failure(
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| StaticEmbedDemo::new(window, cx)),
            ),
            "static embed",
        );
        cx.activate(true);
    });
}
