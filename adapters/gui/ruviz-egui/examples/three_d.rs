use ruviz_egui::{RuvizPlot3D, plot3d_builder};

struct ThreeD {
    interactive: RuvizPlot3D,
    static_plot: RuvizPlot3D,
}

impl Default for ThreeD {
    fn default() -> Self {
        let x = [-1.0_f64, 0.0, 1.0];
        let y = [-1.0_f64, 0.0, 1.0];
        let z = [[0.0_f64, 0.5, 0.0], [0.5, 1.0, 0.5], [0.0, 0.5, 0.0]];
        Self {
            interactive: plot3d_builder(
                ruviz::surface(&x, &y, &z).title("3D: right-drag pan, right-click menu"),
            )
            .interactive()
            .build()
            .expect("valid 3D plot"),
            static_plot: plot3d_builder(
                ruviz::scatter3d(&x, &y, &[0.0_f64, 1.0, 0.0])
                    .title("Static 3D: right-click to enable"),
            )
            .static_view()
            .build()
            .expect("valid 3D plot"),
        }
    }
}

impl eframe::App for ThreeD {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        let available = ui.available_size();
        let height = ((available.y - ui.spacing().item_spacing.y) * 0.5).max(1.0);
        let width = available.x.max(1.0);
        ui.allocate_ui(eframe::egui::vec2(width, height), |ui| {
            let response = self.interactive.show(ui);
            response
                .response
                .clone()
                .on_hover_text("Right-click for camera views and export; right-drag to pan");
            if let Some(hit) = response.picked {
                ui.ctx().debug_painter().text(
                    response.response.rect.left_top(),
                    eframe::egui::Align2::LEFT_TOP,
                    format!("{hit:?}"),
                    eframe::egui::FontId::monospace(11.0),
                    eframe::egui::Color32::WHITE,
                );
            }
        });
        ui.allocate_ui(eframe::egui::vec2(width, height), |ui| {
            self.static_plot
                .show(ui)
                .response
                .on_hover_text("Right-click to enable interaction or export the installed frame");
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "ruviz egui 3D",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<ThreeD>::default())),
    )
}
