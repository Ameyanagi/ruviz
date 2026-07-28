use ruviz::prelude::Plot;
use ruviz_slint::{RuvizController, RuvizPlotGrid, SlotOptions};
use slint::{ComponentHandle as _, LogicalSize};

fn main() -> Result<(), slint::PlatformError> {
    let dashboard = RuvizPlotGrid::new()?;
    dashboard.set_columns(2);
    dashboard.window().set_size(LogicalSize::new(1200.0, 520.0));

    let controller = RuvizController::attach(&dashboard);
    controller.on_error(|error| eprintln!("{error}"));

    let x = [0.0, 1.0, 2.0, 3.0, 4.0];
    controller.set_plot(
        0,
        Plot::new()
            .line(&x, &[0.0, 1.0, 4.0, 9.0, 16.0])
            .title("Interactive line"),
        SlotOptions::default(),
    );
    controller.set_plot(
        1,
        Plot::new()
            .scatter(&x, &[2.0, 3.5, 3.0, 6.0, 7.5])
            .title("Interactive scatter"),
        SlotOptions::default(),
    );
    controller.resize(0, 596.0, 512.0, dashboard.window().scale_factor());
    controller.resize(1, 596.0, 512.0, dashboard.window().scale_factor());

    dashboard.run()
}
