// Copyright 2026 the ruviz contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

use ruviz_slint::{
    RuvizController, RuvizImageFit, RuvizPlotGrid, RuvizRuntime, RuvizSlotState, SlotOptions,
};
use slint::{
    ComponentHandle, LogicalPosition, LogicalSize, ModelRc, SharedString, VecModel,
    platform::{Key, PointerEventButton, WindowEvent},
};
use std::{cell::Cell, rc::Rc};

fn snapshot_bytes(component: &RuvizPlotGrid) -> Vec<u8> {
    component
        .window()
        .take_snapshot()
        .expect("software renderer must snapshot the component")
        .as_bytes()
        .to_vec()
}

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

    let runtime: RuvizRuntime<'_> = dashboard.global();
    runtime.set_slots(ModelRc::new(VecModel::from(vec![RuvizSlotState {
        interactive: false,
        has_frame: true,
        is_3d: false,
        device_scale: 1.0,
        image_fit: RuvizImageFit::Contain,
        ..RuvizSlotState::default()
    }])));
    let secondary_anchor = Rc::new(Cell::new(None::<(f32, f32)>));
    let secondary_moved = Rc::new(Cell::new(false));
    let anchor = Rc::clone(&secondary_anchor);
    let moved = Rc::clone(&secondary_moved);
    runtime.on_pointer_event(move |_, kind, button, x, y| {
        if kind == 0 && button == 2 {
            anchor.set(Some((x, y)));
            moved.set(false);
            false
        } else if kind == 2 {
            if let Some((anchor_x, anchor_y)) = anchor.get() {
                moved.set((x - anchor_x).hypot(y - anchor_y) >= 3.0);
            }
            false
        } else if kind == 1 && button == 2 {
            anchor.take().is_some() && !moved.replace(false)
        } else {
            false
        }
    });

    let window = dashboard.window();
    let closed_menu_snapshot = snapshot_bytes(&dashboard);
    let drag_start = LogicalPosition::new(100.0, 100.0);
    let drag_end = LogicalPosition::new(140.0, 140.0);
    window.dispatch_event(WindowEvent::PointerMoved {
        position: drag_start,
    });
    window.dispatch_event(WindowEvent::PointerPressed {
        position: drag_start,
        button: PointerEventButton::Right,
    });
    assert_eq!(
        snapshot_bytes(&dashboard),
        closed_menu_snapshot,
        "secondary press must not auto-open Slint's context menu"
    );
    window.dispatch_event(WindowEvent::PointerMoved { position: drag_end });
    assert_eq!(
        snapshot_bytes(&dashboard),
        closed_menu_snapshot,
        "secondary drag must remain a plot gesture"
    );
    window.dispatch_event(WindowEvent::PointerReleased {
        position: drag_end,
        button: PointerEventButton::Right,
    });
    assert_eq!(
        snapshot_bytes(&dashboard),
        closed_menu_snapshot,
        "secondary drag release must not open a menu"
    );

    window.dispatch_event(WindowEvent::PointerPressed {
        position: drag_start,
        button: PointerEventButton::Right,
    });
    assert_eq!(
        snapshot_bytes(&dashboard),
        closed_menu_snapshot,
        "an unmoved secondary press waits for release"
    );
    window.dispatch_event(WindowEvent::PointerReleased {
        position: drag_start,
        button: PointerEventButton::Right,
    });
    let pointer_menu_snapshot = snapshot_bytes(&dashboard);
    assert_ne!(
        pointer_menu_snapshot, closed_menu_snapshot,
        "the delayed secondary click must open the menu"
    );

    window.dispatch_event(WindowEvent::KeyPressed {
        text: Key::Escape.into(),
    });
    let keyboard_closed_snapshot = snapshot_bytes(&dashboard);
    assert_ne!(
        keyboard_closed_snapshot, pointer_menu_snapshot,
        "Escape must close the pointer-opened context menu"
    );
    window.dispatch_event(WindowEvent::KeyPressed {
        text: Key::Menu.into(),
    });
    assert_ne!(
        snapshot_bytes(&dashboard),
        keyboard_closed_snapshot,
        "the Menu key must open the context menu for a static slot"
    );
}
