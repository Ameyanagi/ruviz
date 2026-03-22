use gpui::{
    App, Application, Bounds, Context, Render, Timer, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, size,
};
use ruviz::{data::Observable, prelude::*};
use ruviz_gpui::RuvizPlot;
use std::time::Duration;

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
        let plot = cx.new(|cx| RuvizPlot::new(plot, cx));

        window
            .spawn(cx, {
                let y = y.clone();
                async move |_| {
                    Timer::after(Duration::from_millis(750)).await;
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
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(960.0), px(640.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| ObservableEmbedDemo::new(window, cx)),
        )
        .expect("observable embed window should open");
        cx.activate(true);
    });
}
