use gpui::{
    App, Application, Bounds, Context, Render, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, size,
};
use ruviz::prelude::*;
use ruviz_gpui::RuvizPlot;

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

        let plot = cx.new(|cx| RuvizPlot::new(plot, cx));
        Self { plot }
    }
}

impl Render for StaticEmbedDemo {
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
            |window, cx| cx.new(|cx| StaticEmbedDemo::new(window, cx)),
        )
        .expect("static embed window should open");
        cx.activate(true);
    });
}
