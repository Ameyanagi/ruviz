// Copyright 2026 the ruviz contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

use ruviz_slint::{RuvizController, RuvizPlotGrid, SlotOptions};
use slint::{ComponentHandle, LogicalSize, SharedString};

#[test]
fn runtime_slots_layout_without_panicking() {
    slint::platform::set_platform(Box::new(i_slint_backend_testing::TestingBackend::new(
        i_slint_backend_testing::TestingBackendOptions {
            renderer_name: Some(SharedString::from("software")),
            ..Default::default()
        },
    )))
    .expect("testing platform must be installed before constructing a component");

    let dashboard = RuvizPlotGrid::new().expect("packaged component must construct");
    dashboard.set_columns(2);
    dashboard.window().set_size(LogicalSize::new(900.0, 600.0));
    let controller = RuvizController::attach(&dashboard);
    for slot in 0..5 {
        controller.set_plot(
            slot,
            ruviz::prelude::Plot::new().line(&[0.0, 1.0], &[f64::from(slot), 1.0]),
            SlotOptions::default(),
        );
    }

    dashboard.show().expect("testing window must show");
    let frame = dashboard
        .window()
        .take_snapshot()
        .expect("software renderer must lay out and render the grid");
    assert_eq!((frame.width(), frame.height()), (900, 600));
}
