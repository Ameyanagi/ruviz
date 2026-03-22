use gpui::{
    App, Application, Bounds, Context, Render, Timer, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, size,
};
use ruviz::{data::StreamingXY, prelude::*};
use ruviz_gpui::RuvizPlot;
use std::time::Duration;

struct StreamingEmbedDemo {
    plot: gpui::Entity<RuvizPlot>,
}

impl StreamingEmbedDemo {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let stream = StreamingXY::new(2_048);
        stream.push_many((0..240).map(|i| {
            let x = i as f64 * 0.02;
            (x, x.sin())
        }));

        let plot: Plot = Plot::new()
            .line_streaming(&stream)
            .title("Streaming GPUI Embed")
            .xlabel("t")
            .ylabel("signal")
            .into();
        let plot = cx.new(|cx| RuvizPlot::new(plot, cx));

        window
            .spawn(cx, {
                let stream = stream.clone();
                async move |_| {
                    let mut t = 240.0 * 0.02;
                    loop {
                        Timer::after(Duration::from_millis(16)).await;
                        stream.push(t, (t * 1.5).sin());
                        t += 0.02;
                    }
                }
            })
            .detach();

        window
            .spawn(cx, {
                let plot = plot.clone();
                async move |cx| loop {
                    Timer::after(Duration::from_secs(1)).await;
                    let plot = plot.clone();
                    cx.on_next_frame(move |_, cx| {
                        let plot = plot.read(cx);
                        let render = plot.frame_stats();
                        let presentation = plot.presentation_stats();
                        println!(
                            "render_fps={:.1} present_fps={:.1} render_avg_ms={:.2} present_avg_ms={:.2}",
                            render.current_fps,
                            presentation.current_fps,
                            render.average_frame_time.as_secs_f64() * 1000.0,
                            presentation.average_present_interval.as_secs_f64() * 1000.0,
                        );
                    });
                }
            })
            .detach();

        Self { plot }
    }
}

impl Render for StreamingEmbedDemo {
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
            |window, cx| cx.new(|cx| StreamingEmbedDemo::new(window, cx)),
        )
        .expect("streaming embed window should open");
        cx.activate(true);
    });
}
