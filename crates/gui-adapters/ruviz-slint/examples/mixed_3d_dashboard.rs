use ruviz::prelude::Plot;
use ruviz_slint::{InteractionMode, RuvizController, RuvizPlotGrid, SlotOptions};
use slint::{ComponentHandle as _, LogicalSize};

fn slot_options(slot: i32) -> SlotOptions {
    SlotOptions {
        interaction: match slot {
            0 | 1 => InteractionMode::Interactive,
            2 => InteractionMode::Static,
            _ => panic!("mixed dashboard only defines slots 0 through 2"),
        },
        ..SlotOptions::default()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dashboard = RuvizPlotGrid::new()?;
    dashboard.set_columns(3);
    dashboard.window().set_size(LogicalSize::new(1500.0, 520.0));

    eprintln!("ruviz Slint mixed dashboard");
    eprintln!("2D: left-drag pan, wheel zoom, right-drag brush, double-click reset");
    eprintln!("3D: left-drag orbit, middle/right-drag pan, wheel zoom, click pick");
    eprintln!("Right-click without dragging opens export, interaction, and 3D view controls.");
    eprintln!("The rightmost 3D plot is intentionally static for comparison.");

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
            .title("Interactive 2D: drag, wheel, right-drag"),
        slot_options(0),
    );
    controller.set_plot3d(
        1,
        ruviz::scatter3d(&[-1.0, 0.0, 1.0], &[0.0, 1.0, 0.0], &[0.0, 0.5, 1.0])
            .title("Interactive 3D: orbit, pan, wheel"),
        slot_options(1),
    )?;
    controller.set_plot3d(
        2,
        ruviz::scatter3d(&[-1.0, 0.0, 1.0], &[0.0, -1.0, 0.0], &[1.0, 0.0, -1.0])
            .title("Static 3D: comparison"),
        slot_options(2),
    )?;
    let scale = dashboard.window().scale_factor();
    controller.resize(0, 496.0, 512.0, scale);
    controller.resize(1, 496.0, 512.0, scale);
    controller.resize(2, 496.0, 512.0, scale);
    dashboard.run()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_dashboard_interaction_contract_is_unambiguous() {
        assert_eq!(
            slot_options(0).interaction,
            InteractionMode::Interactive,
            "the 2D showcase must accept pointer input"
        );
        assert_eq!(
            slot_options(1).interaction,
            InteractionMode::Interactive,
            "the primary 3D showcase must accept pointer input"
        );
        assert_eq!(
            slot_options(2).interaction,
            InteractionMode::Static,
            "the comparison 3D plot must stay static"
        );
    }
}
