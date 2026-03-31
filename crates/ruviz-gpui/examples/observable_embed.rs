mod support;

use gpui::{
    App, Bounds, Context, Render, Window, WindowBounds, WindowOptions, div, prelude::*, px, size,
};
use ruviz::{data::Observable, prelude::*};
use ruviz_gpui::{PerformancePreset, RuvizPlot, plot_builder};
use std::time::Duration;
use support::{application, exit_on_window_open_failure, sleep};

struct ObservableEmbedDemo {
    plot: gpui::Entity<RuvizPlot>,
}

impl ObservableEmbedDemo {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let x: Vec<f64> = (0..200).map(|i| i as f64 * 0.05).collect();
        let y = Observable::new(x.iter().map(|value| value.sin()).collect::<Vec<_>>());
        let plot: Plot = Plot::new()
            .line_source(x.clone(), y.clone())
            .title("Observable GPUI Embed")
            .xlabel("x")
            .ylabel("value")
            .into();
        let plot = plot_builder(plot)
            .interactive()
            .performance_preset(PerformancePreset::Balanced)
            .build(cx);

        window
            .spawn(cx, {
                let y = y.clone();
                async move |_| {
                    sleep(Duration::from_millis(750)).await;
                    y.set(x.iter().map(|value| (value * 1.5).cos()).collect());
                }
            })
            .detach();

        Self { plot }
    }
}

impl Render for ObservableEmbedDemo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_4().child(self.plot.clone())
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
                |window, cx| cx.new(|cx| ObservableEmbedDemo::new(window, cx)),
            ),
            "observable embed",
        );
        cx.activate(true);
    });
}
