mod support;

use gpui::{
    App, Bounds, Context, MouseButton, Render, Window, WindowBounds, WindowOptions, div,
    prelude::*, px, rgb, size,
};
use ruviz::prelude::*;
use ruviz_gpui::{InteractionOptions, PlotPointerEvent, RuvizPlot, plot_builder};
use support::{application, exit_on_window_open_failure};

struct MovableAnnotationDemo {
    plot: gpui::Entity<RuvizPlot>,
    annotation_id: AnnotationId,
    annotation_x: f64,
    _subscription: gpui::Subscription,
}

impl MovableAnnotationDemo {
    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let x = (0..200)
            .map(|index| index as f64 * 0.05)
            .collect::<Vec<_>>();
        let y = x.iter().map(|value| value.sin()).collect::<Vec<_>>();
        let plot: Plot = Plot::new()
            .line(&x, &y)
            .title("Drag the red annotation")
            .xlabel("Hold the left mouse button and move")
            .into();
        let interaction = InteractionOptions {
            pan: false,
            selection: false,
            tooltips: false,
            ..InteractionOptions::default()
        };
        let plot = plot_builder(plot)
            .interactive()
            .interaction_options(interaction)
            .build(cx);
        let annotation_x = 2.5;
        let annotation_id = plot.update(cx, |plot, cx| {
            plot.add_annotation(
                Annotation::vline_styled(annotation_x, Color::RED, 2.5, LineStyle::Solid),
                cx,
            )
            .expect("initial annotation should be valid")
        });

        let subscription = cx.subscribe(&plot, |this, _, event: &PlotPointerEvent, cx| {
            if event.mouse_button != Some(MouseButton::Left) {
                return;
            }
            let Some(position) = event.data_position else {
                return;
            };
            let plot = this.plot.clone();
            let annotation_id = this.annotation_id;
            if plot
                .update(cx, |plot, cx| {
                    plot.update_annotation(
                        annotation_id,
                        Annotation::vline_styled(position.x, Color::RED, 2.5, LineStyle::Solid),
                        cx,
                    )
                })
                .is_ok()
            {
                this.annotation_x = position.x;
                cx.notify();
            }
        });

        Self {
            plot,
            annotation_id,
            annotation_x,
            _subscription: subscription,
        }
    }
}

impl Render for MovableAnnotationDemo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .gap_2()
            .bg(rgb(0xf6f7fb))
            .child(format!("Dynamic annotation x = {:.3}", self.annotation_x))
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
                |window, cx| cx.new(|cx| MovableAnnotationDemo::new(window, cx)),
            ),
            "movable annotation",
        );
        cx.activate(true);
    });
}
