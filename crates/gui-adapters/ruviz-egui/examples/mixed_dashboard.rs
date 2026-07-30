use ruviz::core::Plot;
use ruviz_egui::{RuvizPlot, RuvizPlot3D, plot_builder, plot3d_builder};

struct MixedDashboard {
    two_d: RuvizPlot,
    three_d: RuvizPlot3D,
}

impl Default for MixedDashboard {
    fn default() -> Self {
        let x: Vec<f64> = (0..160).map(|index| f64::from(index) * 0.05).collect();
        let y: Vec<f64> = x.iter().map(|value| value.sin()).collect();
        let surface_axis = [-1.0_f64, 0.0, 1.0];
        let surface_z = [[0.0_f64, 0.5, 0.0], [0.5, 1.0, 0.5], [0.0, 0.5, 0.0]];
        Self {
            two_d: plot_builder(
                Plot::new()
                    .line(&x, &y)
                    .title("2D: right-drag brush, right-click menu"),
            )
            .interactive()
            .build(),
            three_d: plot3d_builder(
                ruviz::surface(&surface_axis, &surface_axis, &surface_z)
                    .title("3D: right-drag pan, right-click menu"),
            )
            .interactive()
            .build()
            .expect("valid 3D surface"),
        }
    }
}

impl eframe::App for MixedDashboard {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        let available = ui.available_size();
        let panel_width = ((available.x - ui.spacing().item_spacing.x) * 0.5).max(1.0);
        let panel_size = eframe::egui::vec2(panel_width, available.y.max(1.0));
        ui.horizontal(|ui| {
            ui.allocate_ui(panel_size, |ui| {
                self.two_d.show(ui).response.on_hover_text(
                    "Drag to pan; right/shift-drag to brush; right-click for plot actions",
                );
            });
            ui.allocate_ui(panel_size, |ui| {
                self.three_d
                    .show(ui)
                    .response
                    .on_hover_text("Drag to orbit/pan; right-click for camera and export actions");
            });
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "ruviz egui mixed dashboard",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<MixedDashboard>::default())),
    )
}
