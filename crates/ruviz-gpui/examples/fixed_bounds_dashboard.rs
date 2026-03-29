mod support;

use gpui::{
    App, Bounds, Context, Render, Window, WindowBounds, WindowOptions, div, prelude::*, px, size,
};
use ruviz::{data::Observable, prelude::*};
use ruviz_gpui::{InteractionOptions, PerformancePreset, RuvizPlot, plot_builder};
use std::time::Duration;
use support::{application, sleep};

const WINDOW_SECONDS: f64 = 12.0;
const SAMPLE_COUNT: usize = 480;
const DATA_INTERVAL_MS: u64 = 33;
const COLOR_INTERVAL_MS: u64 = 1200;

struct FixedBoundsDashboardDemo {
    plot: gpui::Entity<RuvizPlot>,
}

impl FixedBoundsDashboardDemo {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let x = sample_x();
        let primary_signal = Observable::new(synthesize_primary(&x, 0.0));
        let baseline_signal = Observable::new(synthesize_baseline(&x, 0.0));
        let (initial_event_x, initial_event_y) = detect_events(&x, &primary_signal.get());
        let event_x = Observable::new(initial_event_x);
        let event_y = Observable::new(initial_event_y);
        let accent = Observable::new(accent_palette(0));

        let plot = plot_builder(build_dashboard_plot(
            &x,
            primary_signal.clone(),
            baseline_signal.clone(),
            event_x.clone(),
            event_y.clone(),
            accent.clone(),
        ))
        .interactive()
        .interaction_options(InteractionOptions {
            selection: false,
            ..InteractionOptions::default()
        })
        .performance_preset(PerformancePreset::Interactive)
        .build(cx);

        window
            .spawn(cx, {
                let x = x.clone();
                let primary_signal = primary_signal.clone();
                let baseline_signal = baseline_signal.clone();
                let event_x = event_x.clone();
                let event_y = event_y.clone();
                async move |_| {
                    let mut phase = 0.0;
                    loop {
                        sleep(Duration::from_millis(DATA_INTERVAL_MS)).await;
                        phase += 0.12;

                        let next_primary = synthesize_primary(&x, phase);
                        let next_baseline = synthesize_baseline(&x, phase);
                        let (next_event_x, next_event_y) = detect_events(&x, &next_primary);

                        primary_signal.set(next_primary);
                        baseline_signal.set(next_baseline);
                        event_x.set(next_event_x);
                        event_y.set(next_event_y);
                    }
                }
            })
            .detach();

        window
            .spawn(cx, {
                let accent = accent.clone();
                async move |_| {
                    let mut palette_index = 1usize;
                    loop {
                        sleep(Duration::from_millis(COLOR_INTERVAL_MS)).await;
                        let next_accent = accent_palette(palette_index);
                        palette_index = (palette_index + 1) % 5;
                        accent.set(next_accent);
                    }
                }
            })
            .detach();

        window
            .spawn(cx, {
                let plot = plot.clone();
                async move |cx| loop {
                    sleep(Duration::from_secs(1)).await;
                    let plot = plot.clone();
                    cx.on_next_frame(move |_, cx| {
                        let plot = plot.read(cx);
                        let stats = plot.stats();
                        println!(
                            "backend={:?} render_fps={:.1} display_hz_est={:.1} render_avg_ms={:.2} present_avg_ms={:.2}",
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

impl Render for FixedBoundsDashboardDemo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_4().child(self.plot.clone())
    }
}

fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1080.0), px(720.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| FixedBoundsDashboardDemo::new(window, cx)),
        )
        .expect("fixed-bounds dashboard window should open");
        cx.activate(true);
    });
}

fn build_dashboard_plot(
    x: &[f64],
    primary_signal: Observable<Vec<f64>>,
    baseline_signal: Observable<Vec<f64>>,
    event_x: Observable<Vec<f64>>,
    event_y: Observable<Vec<f64>>,
    accent: Observable<Color>,
) -> Plot {
    let upper_guard = vec![1.2; x.len()];
    let lower_guard = vec![-1.2; x.len()];
    let plot: Plot = Plot::new()
        .line(&x, &upper_guard)
        .label("upper guard")
        .color(Color::LIGHT_GRAY)
        .into();
    let plot: Plot = plot
        .line(&x, &lower_guard)
        .label("lower guard")
        .color(Color::LIGHT_GRAY)
        .into();
    let plot: Plot = plot
        .line_source(x.to_vec(), primary_signal)
        .label("live signal")
        .color_source(accent)
        .line_width(2.4)
        .into();
    let plot: Plot = plot
        .line_source(x.to_vec(), baseline_signal)
        .label("baseline")
        .color(Color::new(38, 70, 83))
        .line_width(1.6)
        .into();

    plot.scatter_source(event_x, event_y)
        .label("anomalies")
        .color(Color::new(231, 111, 81))
        .marker(MarkerStyle::Diamond)
        .marker_size(9.0)
        .title("Fixed-Bounds Reactive GPUI Dashboard")
        .xlabel("window time (s)")
        .ylabel("amplitude")
        .legend(Position::TopRight)
        .grid(true)
        .xlim(0.0, WINDOW_SECONDS)
        .ylim(-2.0, 2.0)
        .into()
}

fn sample_x() -> Vec<f64> {
    (0..SAMPLE_COUNT)
        .map(|index| index as f64 * WINDOW_SECONDS / (SAMPLE_COUNT - 1) as f64)
        .collect()
}

fn synthesize_primary(x: &[f64], phase: f64) -> Vec<f64> {
    x.iter()
        .map(|value| {
            0.92 * (value * 1.15 + phase).sin()
                + 0.33 * (value * 2.85 - phase * 0.55).cos()
                + 0.18 * (value * 6.2 + phase * 1.4).sin()
        })
        .collect()
}

fn synthesize_baseline(x: &[f64], phase: f64) -> Vec<f64> {
    x.iter()
        .map(|value| {
            0.48 * (value * 0.78 + phase * 0.45).sin() + 0.16 * (value * 2.0 + phase * 0.2).cos()
        })
        .collect()
}

fn detect_events(x: &[f64], primary_signal: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut event_x = Vec::new();
    let mut event_y = Vec::new();

    for index in 1..primary_signal.len().saturating_sub(1) {
        let value = primary_signal[index];
        if value.abs() < 1.12 {
            continue;
        }

        let previous = primary_signal[index - 1].abs();
        let next = primary_signal[index + 1].abs();
        if value.abs() >= previous && value.abs() >= next {
            event_x.push(x[index]);
            event_y.push(value);
        }
    }

    (event_x, event_y)
}

fn accent_palette(index: usize) -> Color {
    match index % 5 {
        0 => Color::new(42, 157, 143),
        1 => Color::new(33, 158, 188),
        2 => Color::new(244, 162, 97),
        3 => Color::new(231, 111, 81),
        _ => Color::new(94, 96, 206),
    }
}
