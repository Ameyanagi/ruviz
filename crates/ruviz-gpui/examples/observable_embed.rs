mod support;

use gpui::{
    App, Bounds, Context, Render, Window, WindowBounds, WindowOptions, div, prelude::*, px, size,
};
use ruviz::{
    data::{BatchUpdate, Observable},
    prelude::*,
};
use ruviz_gpui::{PerformancePreset, RuvizPlot, plot_builder};
use std::time::Duration;
use support::{application, exit_on_window_open_failure, sleep};

struct ObservableEmbedDemo {
    plot: gpui::Entity<RuvizPlot>,
}

impl ObservableEmbedDemo {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let x: Vec<f64> = (0..200).map(|i| i as f64 * 0.05).collect();
        let x = Observable::new(x);
        let y = Observable::new(x.read().iter().map(|value| value.sin()).collect::<Vec<_>>());
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
                let x = x.clone();
                let y = y.clone();
                async move |_| {
                    sleep(Duration::from_millis(750)).await;
                    let next_x: Vec<f64> = (0..240).map(|index| index as f64 * 0.04).collect();
                    let next_y = next_x.iter().map(|value| (value * 1.5).cos()).collect();

                    // Replace both complete vectors without rebuilding the plot
                    // session. The batch defers each observable's notifications
                    // until guard drop and coalesces repeated changes within that
                    // observable. The two observables still flush independently;
                    // the guard is not a shared data lock.
                    let mut batch = BatchUpdate::new();
                    batch.add(&x);
                    batch.add(&y);
                    x.set(next_x);
                    y.set(next_y);
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
