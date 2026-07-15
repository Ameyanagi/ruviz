mod support;

use gpui::{
    App, Bounds, Context, Render, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb,
    size,
};
use ruviz::{core::HitResult, plots::heatmap::HeatmapConfig, prelude::*};
use ruviz_gpui::{PlotPointerEvent, RuvizPlot, plot_builder};
use support::{application, exit_on_window_open_failure};

struct CoordinateEventsDemo {
    plot: gpui::Entity<RuvizPlot>,
    last_event: Option<PlotPointerEvent>,
    _subscription: gpui::Subscription,
}

impl CoordinateEventsDemo {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let values = vec![
            vec![0.0, 0.3, 0.8, 1.0],
            vec![0.2, 0.7, 0.9, 0.6],
            vec![0.5, 1.0, 0.4, 0.1],
        ];
        let plot: Plot = Plot::new()
            .heatmap(&values, Some(HeatmapConfig::new().colorbar(false)))
            .title("GPUI Coordinate Events")
            .into();

        let plot = plot_builder(plot).build(cx);
        let subscription = cx.subscribe(&plot, |this, _, event, cx| {
            match &event.hit {
                HitResult::HeatmapCell {
                    row, col, value, ..
                } => println!(
                    "{:?} cell ({row}, {col}) = {value:.3}; data={:?}; window={:?}",
                    event.kind, event.data_position, event.window_position
                ),
                other => println!(
                    "{:?} data={:?}, hit={other:?}",
                    event.kind, event.data_position
                ),
            }
            this.last_event = Some(event.clone());
            cx.notify();
        });
        Self {
            plot,
            last_event: None,
            _subscription: subscription,
        }
    }
}

impl Render for CoordinateEventsDemo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .bg(rgb(0xf6f7fb))
            .child(
                self.last_event
                    .as_ref()
                    .map(|event| {
                        format!("Last event: {:?} at {:?}", event.kind, event.data_position)
                    })
                    .unwrap_or_else(|| "Move over or click the plot".to_string()),
            )
            .child(self.plot.clone())
    }
}

fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(900.0), px(640.0)), cx);
        exit_on_window_open_failure(
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| CoordinateEventsDemo::new(window, cx)),
            ),
            "coordinate events",
        );
        cx.activate(true);
    });
}
