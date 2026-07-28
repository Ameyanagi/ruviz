use ruviz::prelude::Plot;
use ruviz_slint::{InteractionMode, RuvizController, RuvizPlotGrid, SlotOptions};
use slint::{ComponentHandle as _, LogicalSize};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dashboard = RuvizPlotGrid::new()?;
    dashboard.set_columns(2);
    dashboard.window().set_size(LogicalSize::new(1200.0, 520.0));

    let controller = RuvizController::attach(&dashboard);
    controller.on_error(|error| eprintln!("{error}"));
    controller.on_pick(|slot, hit| eprintln!("slot {slot} picked {hit:?}"));
    controller.on_camera_change(|slot, camera| {
        eprintln!("slot {slot} camera changed: {:?}", camera.camera)
    });

    controller.set_plot(
        0,
        Plot::new()
            .line(&[0.0, 1.0, 2.0, 3.0], &[0.0, 1.0, 4.0, 9.0])
            .title("Static 2D"),
        SlotOptions {
            interaction: InteractionMode::Static,
            ..SlotOptions::default()
        },
    );
    controller.set_plot3d(
        1,
        ruviz::scatter3d(&[-1.0, 0.0, 1.0], &[0.0, 1.0, 0.0], &[0.0, 0.5, 1.0])
            .title("Interactive 3D"),
        SlotOptions::default(),
    )?;
    let scale = dashboard.window().scale_factor();
    controller.resize(0, 596.0, 512.0, scale);
    controller.resize(1, 596.0, 512.0, scale);
    dashboard.run()?;
    Ok(())
}
