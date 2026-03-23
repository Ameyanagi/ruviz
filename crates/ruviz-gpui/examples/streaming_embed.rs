use gpui::{
    App, Application, Bounds, Context, Render, Timer, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, size,
};
use ruviz::{data::StreamingXY, prelude::*};
use ruviz_gpui::{PerformancePreset, RuvizPlot, plot_builder};
use std::{env, time::Duration};

struct StreamingEmbedDemo {
    plot: gpui::Entity<RuvizPlot>,
}

impl StreamingEmbedDemo {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let producer_interval_ms = env::args()
            .skip(1)
            .find_map(parse_interval_arg)
            .unwrap_or(16);
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
        let plot = plot_builder(plot)
            .interactive()
            .performance_preset(PerformancePreset::Interactive)
            .build(cx);

        window
            .spawn(cx, {
                let stream = stream.clone();
                async move |_| {
                    let mut t = 240.0 * 0.02;
                    loop {
                        Timer::after(Duration::from_millis(producer_interval_ms)).await;
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
                        let stats = plot.stats();
                        println!(
                            "producer_ms={} backend={:?} render_fps={:.1} display_hz_est={:.1} render_avg_ms={:.2} present_avg_ms={:.2}",
                            producer_interval_ms,
                            stats.active_backend,
                            stats.render.current_fps,
                            stats.presentation.current_fps,
                            stats.render.average_frame_time.as_secs_f64() * 1000.0,
                            stats.presentation.average_present_interval.as_secs_f64() * 1000.0,
                        );
                    });
                }
            })
            .detach();

        Self { plot }
    }
}

fn parse_interval_arg(arg: String) -> Option<u64> {
    if let Some(value) = arg.strip_prefix("--interval-ms=") {
        value.parse().ok()
    } else {
        arg.parse().ok()
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
