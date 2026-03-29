mod support;

use gpui::{
    App, Bounds, Context, Render, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb,
    size,
};
use ruviz::prelude::*;
use ruviz_gpui::{GpuiContextMenuConfig, GpuiContextMenuItem, RuvizPlot, plot_builder};
use support::application;

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

        let plot = plot_builder(plot)
            .interactive()
            .context_menu(GpuiContextMenuConfig {
                custom_items: vec![GpuiContextMenuItem::new(
                    "dump-view",
                    "Print Visible Bounds",
                )],
                ..GpuiContextMenuConfig::default()
            })
            .on_context_menu_action(|context| {
                println!(
                    "custom action: visible_bounds=({:.3}, {:.3}) -> ({:.3}, {:.3}) cursor={:?}",
                    context.visible_bounds.min.x,
                    context.visible_bounds.min.y,
                    context.visible_bounds.max.x,
                    context.visible_bounds.max.y,
                    context.cursor_data_position
                );
                Ok(())
            })
            .build(cx);
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
