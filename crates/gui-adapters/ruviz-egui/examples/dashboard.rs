use ruviz::core::Plot;
use ruviz_egui::{RuvizPlot, plot_builder};

struct Dashboard {
    interactive: RuvizPlot,
    static_plot: RuvizPlot,
}

impl Default for Dashboard {
    fn default() -> Self {
        let x: Vec<f64> = (0..200).map(|index| f64::from(index) * 0.05).collect();
        let sine: Vec<f64> = x.iter().map(|value| value.sin()).collect();
        let cosine: Vec<f64> = x.iter().map(|value| value.cos()).collect();
        Self {
            interactive: plot_builder(
                Plot::new()
                    .line(&x, &sine)
                    .label("sin(x)")
                    .title("Interactive: drag, scroll, shift-drag, double-click"),
            )
            .interactive()
            .build(),
            static_plot: plot_builder(
                Plot::new()
                    .scatter(&x, &cosine)
                    .title("Static: resize and reactive redraw only"),
            )
            .static_view()
            .build(),
        }
    }
}

impl eframe::App for Dashboard {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        let available = ui.available_size();
        let plot_height = (available.y - ui.spacing().item_spacing.y).max(2.0) * 0.5;
        let width = available.x.max(1.0);
        ui.allocate_ui(eframe::egui::vec2(width, plot_height), |ui| {
            self.interactive
                .show(ui)
                .response
                .on_hover_text("Scroll to zoom; drag to pan; shift-drag to brush");
        });
        ui.allocate_ui(eframe::egui::vec2(width, plot_height), |ui| {
            self.static_plot.show(ui);
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "ruviz egui dashboard",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<Dashboard>::default())),
    )
}
