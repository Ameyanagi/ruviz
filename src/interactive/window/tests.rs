use super::*;
use std::sync::{Arc, Mutex};
use winit::keyboard::ModifiersState;

fn plot_area_center(window: &InteractiveWindow) -> Point2D {
    Point2D::new(
        (window.interaction_state.screen_bounds.min.x
            + window.interaction_state.screen_bounds.max.x)
            * 0.5,
        (window.interaction_state.screen_bounds.min.y
            + window.interaction_state.screen_bounds.max.y)
            * 0.5,
    )
}

fn viewport_snapshot(window: &InteractiveWindow) -> crate::core::plot::InteractiveViewportSnapshot {
    window
        .renderer
        .viewport_snapshot()
        .expect("viewport snapshot should succeed")
        .expect("interactive session should be available")
}

fn screen_to_data(
    visible_bounds: crate::core::ViewportRect,
    plot_area: crate::core::ViewportRect,
    position: Point2D,
) -> Point2D {
    let normalized_x = ((position.x - plot_area.min.x) / plot_area.width()).clamp(0.0, 1.0);
    let normalized_y = ((position.y - plot_area.min.y) / plot_area.height()).clamp(0.0, 1.0);
    Point2D::new(
        visible_bounds.min.x + visible_bounds.width() * normalized_x,
        visible_bounds.max.y - visible_bounds.height() * normalized_y,
    )
}

fn open_context_menu(window: &mut InteractiveWindow, position: Point2D) {
    window.mouse_position = PhysicalPosition::new(position.x, position.y);
    window
        .handle_right_button_pressed(position)
        .expect("right mouse down should succeed");
    window
        .handle_right_button_released(position)
        .expect("right mouse up should succeed");
    assert!(window.context_menu.is_some(), "context menu should be open");
}

fn context_menu_entry_center(window: &InteractiveWindow, label: &str) -> Point2D {
    let menu = window
        .context_menu
        .as_ref()
        .expect("context menu should be open");
    let entry = menu
        .entries
        .iter()
        .find(|entry| entry.label == label)
        .expect("menu entry should exist");
    let bounds = entry.bounds.expect("menu entry should have bounds");
    bounds.center()
}

fn primary_shortcut_modifiers() -> ModifiersState {
    #[cfg(target_os = "macos")]
    {
        ModifiersState::SUPER
    }

    #[cfg(not(target_os = "macos"))]
    {
        ModifiersState::CONTROL
    }
}

fn assert_visible_bounds_close(
    actual: crate::core::ViewportRect,
    expected: crate::core::ViewportRect,
) {
    assert!(
        (actual.min.x - expected.min.x).abs() < 1e-6,
        "min.x mismatch: actual={actual:?}, expected={expected:?}"
    );
    assert!(
        (actual.max.x - expected.max.x).abs() < 1e-6,
        "max.x mismatch: actual={actual:?}, expected={expected:?}"
    );
    assert!(
        (actual.min.y - expected.min.y).abs() < 1e-6,
        "min.y mismatch: actual={actual:?}, expected={expected:?}"
    );
    assert!(
        (actual.max.y - expected.max.y).abs() < 1e-6,
        "max.y mismatch: actual={actual:?}, expected={expected:?}"
    );
}

async fn interactive_window_for_test() -> InteractiveWindow {
    let plot: Plot = Plot::new()
        .line(&[0.0, 5.0, 10.0], &[0.0, 5.0, 10.0])
        .title("Interactive Test")
        .xlabel("X")
        .ylabel("Y")
        .xlim(0.0, 10.0)
        .ylim(0.0, 10.0)
        .into();
    let mut window = InteractiveWindow::new(plot.clone(), "Test Window", 640, 480)
        .await
        .expect("window should build");
    window.renderer.set_plot(plot);
    window
        .render_frame()
        .expect("initial render should populate session geometry");
    window
}

#[tokio::test]
async fn test_window_builder() {
    let plot = Plot::new(); // Empty plot for testing

    let window_result = InteractiveWindowBuilder::new()
        .title("Test Window")
        .size(640, 480)
        .resizable(false)
        .build(plot)
        .await;

    assert!(window_result.is_ok());
    assert!(window_result.unwrap().context_menu_config.enabled);
}

#[tokio::test]
async fn test_window_builder_rejects_custom_context_items_without_handler() {
    let plot = Plot::new();

    let window_result = InteractiveWindowBuilder::new()
        .context_menu(InteractiveContextMenuConfig {
            custom_items: vec![InteractiveContextMenuItem::new("custom", "Custom Action")],
            ..Default::default()
        })
        .build(plot)
        .await;

    assert!(window_result.is_err());
}

// Note: test_mouse_button_conversion was removed because InteractiveWindow
// cannot be constructed in tests (requires async RealTimeRenderer creation)

#[test]
fn test_default_event_handler() {
    let mut handler = DefaultEventHandler::new();

    let zoom_event = InteractionEvent::Zoom {
        factor: 1.5,
        center: Point2D::new(100.0, 100.0),
    };

    let response = handler.handle_event(zoom_event);
    assert!(response.is_ok());
    assert_eq!(
        response.unwrap(),
        crate::interactive::event::EventResponse::Handled
    );

    assert!(!handler.needs_redraw());
}

#[test]
fn test_copy_rgba_to_softbuffer() {
    let src = [
        0x12, 0x34, 0x56, 0xff, //
        0xab, 0xcd, 0xef, 0x40,
    ];
    let mut dst = [0u32; 2];

    copy_rgba_to_softbuffer(&src, &mut dst);

    assert_eq!(dst, [0x00123456, 0x00abcdef]);
}

#[test]
fn test_scroll_delta_normalization_respects_physical_direction() {
    let line_delta =
        InteractiveWindow::normalize_scroll_delta(MouseScrollDelta::LineDelta(0.0, 1.0));
    let pixel_delta = InteractiveWindow::normalize_scroll_delta(MouseScrollDelta::PixelDelta(
        PhysicalPosition::new(0.0, 50.0),
    ));

    assert_eq!(line_delta, -LINE_SCROLL_DELTA_PX);
    assert_eq!(pixel_delta, 50.0);
}

#[test]
fn test_cursor_data_position_returns_none_for_zero_sized_plot_area() {
    let visible_bounds =
        ViewportRect::from_points(ViewportPoint::new(0.0, 0.0), ViewportPoint::new(10.0, 10.0));
    let zero_width_plot_area =
        ViewportRect::from_points(ViewportPoint::new(4.0, 2.0), ViewportPoint::new(4.0, 8.0));
    let zero_height_plot_area =
        ViewportRect::from_points(ViewportPoint::new(2.0, 6.0), ViewportPoint::new(8.0, 6.0));

    assert_eq!(
        cursor_data_position(
            visible_bounds,
            zero_width_plot_area,
            ViewportPoint::new(4.0, 5.0),
        ),
        None
    );
    assert_eq!(
        cursor_data_position(
            visible_bounds,
            zero_height_plot_area,
            ViewportPoint::new(5.0, 6.0),
        ),
        None
    );
}

#[test]
fn test_blend_rgba_into_softbuffer() {
    let src = [
        0xff, 0x00, 0x00, 0x80, //
        0x00, 0xff, 0x00, 0xff,
    ];
    let mut dst = [0x000000ff, 0x000000ff];

    blend_rgba_into_softbuffer(&src, &mut dst);

    assert_eq!(dst[1], 0x0000ff00);
    assert_ne!(dst[0], 0x000000ff);
    assert_ne!(dst[0], 0x00ff0000);
}

#[tokio::test]
async fn test_primary_shortcut_maps_s_to_save_png() {
    let mut window = interactive_window_for_test().await;
    window.modifiers_state = primary_shortcut_modifiers();

    assert_eq!(
        window.builtin_shortcut_action_for_key("s"),
        Some(BuiltinContextMenuAction::SavePng)
    );
}

#[tokio::test]
async fn test_primary_shortcut_maps_c_to_copy_image() {
    let mut window = interactive_window_for_test().await;
    window.modifiers_state = primary_shortcut_modifiers();

    assert_eq!(
        window.builtin_shortcut_action_for_key("c"),
        Some(BuiltinContextMenuAction::CopyImage)
    );
}

#[tokio::test]
async fn test_primary_shortcut_respects_disabled_copy_image_action() {
    let mut window = interactive_window_for_test().await;
    window.modifiers_state = primary_shortcut_modifiers();
    window.context_menu_config.show_copy_image = false;

    assert_eq!(window.builtin_shortcut_action_for_key("c"), None);
}

#[tokio::test]
async fn test_scroll_zoom_keeps_cursor_anchor_stable() {
    let mut window = interactive_window_for_test().await;
    let before = viewport_snapshot(&window);
    let anchor = Point2D::new(before.plot_area.min.x, plot_area_center(&window).y);
    let anchor_before = screen_to_data(before.visible_bounds, before.plot_area, anchor);
    let expected_factor = (1.0025f64).powf(-LINE_SCROLL_DELTA_PX).clamp(0.25, 4.0);

    window.mouse_position = PhysicalPosition::new(anchor.x, anchor.y);
    window
        .handle_scroll_delta(LINE_SCROLL_DELTA_PX)
        .expect("scroll zoom should succeed");
    window
        .render_frame()
        .expect("render after zoom should succeed");

    let after = viewport_snapshot(&window);
    let anchor_after = screen_to_data(after.visible_bounds, after.plot_area, anchor);

    assert!(
        (after.zoom_level - expected_factor).abs() < 1e-9,
        "expected zoom {}, got {}",
        expected_factor,
        after.zoom_level
    );
    assert!(
        (anchor_before.x - anchor_after.x).abs() < 1e-6,
        "expected stable x anchor, before={anchor_before:?}, after={anchor_after:?}"
    );
    assert!(
        (anchor_before.y - anchor_after.y).abs() < 1e-6,
        "expected stable y anchor, before={anchor_before:?}, after={anchor_after:?}, before_snapshot={before:?}, after_snapshot={after:?}"
    );
}

#[tokio::test]
async fn test_right_drag_zoom_rect_updates_visible_bounds() {
    let mut window = interactive_window_for_test().await;
    let before = viewport_snapshot(&window);
    let start = Point2D::new(before.plot_area.min.x + 40.0, before.plot_area.min.y + 36.0);
    let end = Point2D::new(
        before.plot_area.min.x + 220.0,
        before.plot_area.min.y + 168.0,
    );
    let start_data = screen_to_data(before.visible_bounds, before.plot_area, start);
    let end_data = screen_to_data(before.visible_bounds, before.plot_area, end);
    let expected = crate::core::ViewportRect::from_points(
        ViewportPoint::new(start_data.x, start_data.y),
        ViewportPoint::new(end_data.x, end_data.y),
    );

    window
        .handle_right_button_pressed(start)
        .expect("mouse down should succeed");
    window
        .handle_pointer_moved(end)
        .expect("box zoom drag should succeed");
    assert_eq!(
        window.interaction_state.brushed_region,
        Some(Rectangle::from_points(start, end))
    );
    window
        .handle_right_button_released(end)
        .expect("box zoom release should succeed");
    window
        .render_frame()
        .expect("render after box zoom should succeed");

    let after = viewport_snapshot(&window);
    assert_visible_bounds_close(after.visible_bounds, expected);
    assert_eq!(after.selected_count, 0);
    assert!(window.interaction_state.brushed_region.is_none());
    assert!(window.context_menu.is_none());
}

#[tokio::test]
async fn test_left_drag_pan_uses_incremental_pointer_deltas() {
    let mut window = interactive_window_for_test().await;
    let before = viewport_snapshot(&window);
    let start = plot_area_center(&window);
    let end = Point2D::new(start.x + 40.0, start.y + 24.0);

    window
        .handle_left_button_pressed(start)
        .expect("mouse down should succeed");
    window
        .handle_pointer_moved(end)
        .expect("drag move should succeed");
    window
        .handle_left_button_released(end)
        .expect("mouse up should succeed");
    window
        .render_frame()
        .expect("render after pan should succeed");

    let after = viewport_snapshot(&window);
    let expected_dx = -(40.0 / before.plot_area.width()) * before.visible_bounds.width();
    let expected_dy = (24.0 / before.plot_area.height()) * before.visible_bounds.height();

    assert!(
        ((after.visible_bounds.min.x - before.visible_bounds.min.x) - expected_dx).abs() < 1e-6,
        "min.x delta mismatch: before={before:?}, after={after:?}, expected_dx={expected_dx}"
    );
    assert!(
        ((after.visible_bounds.max.x - before.visible_bounds.max.x) - expected_dx).abs() < 1e-6,
        "max.x delta mismatch: before={before:?}, after={after:?}, expected_dx={expected_dx}"
    );
    assert!(
        ((after.visible_bounds.min.y - before.visible_bounds.min.y) - expected_dy).abs() < 1e-6,
        "min.y delta mismatch: before={before:?}, after={after:?}, expected_dy={expected_dy}"
    );
    assert!(
        ((after.visible_bounds.max.y - before.visible_bounds.max.y) - expected_dy).abs() < 1e-6,
        "max.y delta mismatch: before={before:?}, after={after:?}, expected_dx={expected_dx}"
    );
}

#[tokio::test]
async fn test_small_pointer_motion_selects_without_panning() {
    let mut window = interactive_window_for_test().await;
    let before = viewport_snapshot(&window);
    let center = plot_area_center(&window);

    window
        .handle_left_button_pressed(center)
        .expect("mouse down should succeed");
    window
        .handle_pointer_moved(Point2D::new(center.x + 2.0, center.y + 1.0))
        .expect("small move should succeed");
    window
        .handle_left_button_released(Point2D::new(center.x + 2.0, center.y + 1.0))
        .expect("mouse up should succeed");
    window
        .render_frame()
        .expect("render after click selection should succeed");

    let after = viewport_snapshot(&window);

    assert_eq!(after.visible_bounds, before.visible_bounds);
    assert_eq!(after.selected_count, 1);
}

#[tokio::test]
async fn test_hover_updates_are_batched_until_render() {
    let mut window = interactive_window_for_test().await;
    let center = plot_area_center(&window);

    window
        .handle_pointer_moved(center)
        .expect("hover move should queue successfully");

    assert!(matches!(window.pending_hover, Some(PendingHover::Hover(_))));
    assert!(window.has_pending_redraw());

    window
        .render_frame()
        .expect("render after queued hover should succeed");

    assert!(window.pending_hover.is_none());
}

#[tokio::test]
async fn test_static_window_uses_demand_driven_redraws() {
    let window = interactive_window_for_test().await;

    assert!(!window.has_pending_redraw());
    assert!(!window.needs_frame_timer());
}

#[tokio::test]
async fn test_right_click_opens_context_menu_without_zooming() {
    let mut window = interactive_window_for_test().await;
    let before = viewport_snapshot(&window);
    let center = plot_area_center(&window);

    open_context_menu(&mut window, center);

    let after = viewport_snapshot(&window);
    assert_eq!(after.visible_bounds, before.visible_bounds);
    assert!(window.context_menu.is_some());
}

#[tokio::test]
async fn test_escape_closes_context_menu_without_resetting_view() {
    let mut window = interactive_window_for_test().await;
    let center = plot_area_center(&window);
    window.mouse_position = PhysicalPosition::new(center.x, center.y);
    window
        .handle_scroll_delta(LINE_SCROLL_DELTA_PX)
        .expect("scroll zoom should succeed");
    window
        .render_frame()
        .expect("render after zoom should succeed");
    let zoomed = viewport_snapshot(&window);

    open_context_menu(&mut window, center);
    window
        .handle_key_string("Escape")
        .expect("escape should close the context menu");

    let after = viewport_snapshot(&window);
    assert!(window.context_menu.is_none());
    assert_eq!(after.visible_bounds, zoomed.visible_bounds);
}

#[tokio::test]
async fn test_context_menu_set_home_and_go_home_restore_saved_view() {
    let mut window = interactive_window_for_test().await;
    let center = plot_area_center(&window);
    window.mouse_position = PhysicalPosition::new(center.x, center.y);
    window
        .handle_scroll_delta(LINE_SCROLL_DELTA_PX)
        .expect("scroll zoom should succeed");
    window
        .render_frame()
        .expect("render after zoom should succeed");
    let saved_view = viewport_snapshot(&window).visible_bounds;

    open_context_menu(&mut window, center);
    let set_home_center = context_menu_entry_center(&window, "Set Current View As Home");
    window
        .handle_left_button_pressed(set_home_center)
        .expect("set home click should succeed");
    assert_eq!(window.home_view_bounds, Some(saved_view));

    window
        .apply_plot_input(PlotInputEvent::ResetView, true)
        .expect("reset view should succeed");
    window
        .render_frame()
        .expect("render after reset should succeed");

    open_context_menu(&mut window, center);
    let go_home_center = context_menu_entry_center(&window, "Go To Home View");
    window
        .handle_left_button_pressed(go_home_center)
        .expect("go home click should succeed");
    window
        .render_frame()
        .expect("render after go home should succeed");

    let after = viewport_snapshot(&window);
    assert_visible_bounds_close(after.visible_bounds, saved_view);
}

#[tokio::test]
async fn test_custom_context_menu_action_receives_current_view_context() {
    let plot: Plot = Plot::new()
        .line(&[0.0, 5.0, 10.0], &[0.0, 5.0, 10.0])
        .title("Interactive Test")
        .xlabel("X")
        .ylabel("Y")
        .xlim(0.0, 10.0)
        .ylim(0.0, 10.0)
        .into();
    let observed = Arc::new(Mutex::new(
        None::<(String, (u32, u32), ViewportPoint, Option<ViewportPoint>)>,
    ));
    let observed_clone = Arc::clone(&observed);
    let mut window = InteractiveWindowBuilder::new()
        .context_menu(InteractiveContextMenuConfig {
            custom_items: vec![InteractiveContextMenuItem::new("export-csv", "Export CSV")],
            ..Default::default()
        })
        .on_context_menu_action(move |context| {
            *observed_clone.lock().expect("callback lock should succeed") = Some((
                context.action_id,
                context.window_size_px,
                context.cursor_position_px,
                context.cursor_data_position,
            ));
            Ok(())
        })
        .build(plot.clone())
        .await
        .expect("window should build with custom action");
    window.renderer.set_plot(plot);
    window
        .render_frame()
        .expect("initial render should populate session geometry");

    let center = plot_area_center(&window);
    let snapshot = viewport_snapshot(&window);
    let expected_cursor_data = cursor_data_position(
        snapshot.visible_bounds,
        snapshot.plot_area,
        ViewportPoint::new(center.x, center.y),
    );
    open_context_menu(&mut window, center);
    let custom_center = context_menu_entry_center(&window, "Export CSV");
    window.mouse_position = PhysicalPosition::new(custom_center.x, custom_center.y);
    window
        .handle_left_button_pressed(custom_center)
        .expect("custom action click should succeed");

    let observed = observed.lock().expect("callback lock should succeed");
    assert_eq!(
        *observed,
        Some((
            "export-csv".to_string(),
            (window.window_size.width, window.window_size.height),
            ViewportPoint::new(center.x, center.y),
            expected_cursor_data,
        ))
    );
}
