use super::*;
use crate::data::{Observable, StreamingXY, signal};
use crate::prelude::Plot;
use crate::render::{Color, MarkerStyle};
use std::sync::{Arc, atomic::Ordering};

fn render_target() -> SurfaceTarget {
    SurfaceTarget {
        size_px: (320, 240),
        scale_factor: 1.0,
        time_seconds: 0.0,
    }
}

fn derived_y_ticks(session: &InteractivePlotSession) -> Vec<f64> {
    let geometry = session
        .geometry_snapshot()
        .expect("geometry should be available after render");
    let plot = session.prepared_plot().plot();
    crate::axes::generate_ticks_for_scale(
        geometry.y_bounds.0,
        geometry.y_bounds.1,
        plot.layout.tick_config.major_ticks_y,
        &plot.layout.y_scale,
    )
}

fn color_centroid<F>(image: &Image, predicate: F) -> Option<ViewportPoint>
where
    F: Fn(&[u8]) -> bool,
{
    let mut x_sum = 0.0;
    let mut y_sum = 0.0;
    let mut count = 0.0;

    for (index, pixel) in image.pixels.chunks_exact(4).enumerate() {
        if !predicate(pixel) {
            continue;
        }

        let x = (index as u32 % image.width) as f64;
        let y = (index as u32 / image.width) as f64;
        x_sum += x;
        y_sum += y;
        count += 1.0;
    }

    (count > 0.0).then(|| ViewportPoint::new(x_sum / count, y_sum / count))
}

fn count_matching_pixels_near<F>(
    image: &Image,
    center: ViewportPoint,
    radius: u32,
    predicate: F,
) -> usize
where
    F: Fn(&[u8]) -> bool,
{
    let min_x = center.x.round().max(0.0) as i32 - radius as i32;
    let max_x = center.x.round().min(image.width as f64) as i32 + radius as i32;
    let min_y = center.y.round().max(0.0) as i32 - radius as i32;
    let max_y = center.y.round().min(image.height as f64) as i32 + radius as i32;

    let mut count = 0usize;
    for y in min_y.max(0)..max_y.min(image.height as i32) {
        for x in min_x.max(0)..max_x.min(image.width as i32) {
            let index = ((y as u32 * image.width + x as u32) * 4) as usize;
            if predicate(&image.pixels[index..index + 4]) {
                count += 1;
            }
        }
    }

    count
}

fn count_matching_pixels_outside_rect<F>(
    image: &Image,
    rect: tiny_skia::Rect,
    predicate: F,
) -> usize
where
    F: Fn(&[u8]) -> bool,
{
    let left = rect.left().floor() as i32;
    let right = rect.right().ceil() as i32;
    let top = rect.top().floor() as i32;
    let bottom = rect.bottom().ceil() as i32;

    let mut count = 0usize;
    for y in 0..image.height as i32 {
        for x in 0..image.width as i32 {
            if x >= left && x < right && y >= top && y < bottom {
                continue;
            }

            let index = ((y as u32 * image.width + x as u32) * 4) as usize;
            if predicate(&image.pixels[index..index + 4]) {
                count += 1;
            }
        }
    }

    count
}

fn matching_pixel_bounds<F>(image: &Image, predicate: F) -> Option<(u32, u32, u32, u32)>
where
    F: Fn(&[u8]) -> bool,
{
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut found = false;

    for y in 0..image.height {
        for x in 0..image.width {
            let index = ((y * image.width + x) * 4) as usize;
            if !predicate(&image.pixels[index..index + 4]) {
                continue;
            }
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            found = true;
        }
    }

    found.then_some((min_x, min_y, max_x, max_y))
}

#[test]
fn test_dirty_domains_mark_and_clear() {
    let mut dirty = DirtyDomains::default();
    dirty.mark(DirtyDomain::Layout);
    dirty.mark(DirtyDomain::Overlay);
    dirty.mark(DirtyDomain::Temporal);

    assert!(dirty.layout);
    assert!(dirty.overlay);
    assert!(dirty.temporal);
    assert!(dirty.needs_base_render());
    assert!(dirty.needs_overlay_render());

    dirty.clear_base();
    assert!(!dirty.layout);
    assert!(!dirty.data);
    assert!(!dirty.temporal);
    assert!(!dirty.interaction);
    assert!(dirty.overlay);

    dirty.clear_overlay();
    assert!(!dirty.overlay);
}

#[test]
fn test_resize_updates_size_and_scale_factor_together() {
    let plot: Plot = Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).into();
    let session = plot.prepare_interactive();

    session.resize((640, 480), 2.0);

    let state = session
        .inner
        .state
        .lock()
        .expect("InteractivePlotSession state lock poisoned")
        .clone();
    assert_eq!(state.size_px, (640, 480));
    assert_eq!(state.scale_factor, 2.0);
    assert!(session.dirty_domains().layout);
}

#[test]
fn test_resize_event_updates_size_and_scale_factor_together() {
    let plot: Plot = Plot::new().line(&[0.0, 1.0], &[0.0, 1.0]).into();
    let session = plot.prepare_interactive();

    session.apply_input(PlotInputEvent::Resize {
        size_px: (640, 480),
        scale_factor: 2.0,
    });

    let state = session
        .inner
        .state
        .lock()
        .expect("InteractivePlotSession state lock poisoned")
        .clone();
    assert_eq!(state.size_px, (640, 480));
    assert_eq!(state.scale_factor, 2.0);
    assert!(session.dirty_domains().layout);
}

#[test]
fn test_inflight_dirty_marks_survive_render_clear() {
    let plot: Plot = Plot::new().line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]).into();
    let session = plot.prepare_interactive();

    session.mark_dirty(DirtyDomain::Data);
    let render_epoch = session.inner.dirty_epoch.load(Ordering::Acquire);
    session.mark_dirty(DirtyDomain::Overlay);
    session.clear_dirty_after_render(render_epoch);

    let dirty = session.dirty_domains();
    assert!(dirty.data);
    assert!(dirty.overlay);
}

#[test]
fn test_session_invalidate_forces_base_rerender() {
    let plot: Plot = Plot::new()
        .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
        .title("Invalidate")
        .into();
    let session = plot.prepare_interactive();

    session
        .render_to_surface(render_target())
        .expect("initial surface frame should render");
    assert!(!session.dirty_domains().needs_base_render());

    session.invalidate();
    assert!(session.dirty_domains().needs_base_render());

    let rerendered = session
        .render_to_surface(render_target())
        .expect("invalidated surface frame should rerender");
    assert!(rerendered.layer_state.base_dirty);
}

#[test]
fn test_compose_images_alpha_blends_overlay() {
    let base = Image::new(1, 1, vec![0, 0, 255, 255]);
    let overlay = Image::new(1, 1, vec![255, 0, 0, 128]);
    let composed = compose_images(&base, &overlay);

    assert!(composed.pixels[0] > 0);
    assert!(composed.pixels[2] > 0);
    assert_eq!(composed.pixels[3], 255);
}

#[test]
fn test_overlay_only_updates_reuse_cached_base_layer() {
    let plot: Plot = Plot::new()
        .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
        .title("Layer Reuse")
        .into();
    let session = plot.prepare_interactive();

    let first = session
        .render_to_surface(SurfaceTarget {
            size_px: (320, 240),
            scale_factor: 1.0,
            time_seconds: 0.0,
        })
        .expect("surface frame should render");
    assert!(first.layer_state.base_dirty);

    let geometry = session
        .geometry_snapshot()
        .expect("geometry should be available after first frame");
    let (hover_x, hover_y) = map_data_to_pixels(
        1.0,
        1.0,
        geometry.x_bounds.0,
        geometry.x_bounds.1,
        geometry.y_bounds.0,
        geometry.y_bounds.1,
        geometry.plot_area,
    );

    session.apply_input(PlotInputEvent::Hover {
        position_px: ViewportPoint::new(hover_x as f64, hover_y as f64),
    });
    let second = session
        .render_to_surface(SurfaceTarget {
            size_px: (320, 240),
            scale_factor: 1.0,
            time_seconds: 0.0,
        })
        .expect("surface frame should render after hover");

    assert!(!second.layer_state.base_dirty);
    assert!(second.layer_state.overlay_dirty);
    assert!(Arc::ptr_eq(&first.layers.base, &second.layers.base));
}

#[test]
fn test_tooltip_overlay_renders_text_pixels() {
    let plot: Plot = Plot::new()
        .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
        .title("Tooltip")
        .into();
    let session = plot.prepare_interactive();

    session.apply_input(PlotInputEvent::ShowTooltip {
        content: "x=1.234, y=5.678".to_string(),
        position_px: ViewportPoint::new(180.0, 120.0),
    });

    let frame = session
        .render_to_surface(SurfaceTarget {
            size_px: (320, 240),
            scale_factor: 1.0,
            time_seconds: 0.0,
        })
        .expect("surface frame should render with tooltip");

    let overlay = frame
        .layers
        .overlay
        .expect("surface frame should include overlay pixels");
    let dark_text_pixels = overlay
        .pixels
        .chunks_exact(4)
        .filter(|pixel| pixel[3] > 0 && (pixel[0] < 220 || pixel[1] < 220 || pixel[2] < 180))
        .count();

    assert!(
        dark_text_pixels > 0,
        "tooltip overlay should contain dark text pixels in addition to the background box"
    );
}

#[test]
fn test_brush_overlay_renders_visible_outline() {
    let plot: Plot = Plot::new()
        .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
        .title("Brush")
        .into();
    let session = plot.prepare_interactive();

    session.apply_input(PlotInputEvent::BrushStart {
        position_px: ViewportPoint::new(96.0, 72.0),
    });
    session.apply_input(PlotInputEvent::BrushMove {
        position_px: ViewportPoint::new(160.0, 136.0),
    });

    let frame = session
        .render_to_surface(SurfaceTarget {
            size_px: (320, 240),
            scale_factor: 1.0,
            time_seconds: 0.0,
        })
        .expect("surface frame should render with brush overlay");

    let overlay = frame
        .layers
        .overlay
        .expect("surface frame should include brush overlay");
    let width = frame.layers.base.width as usize;

    let border_index = ((72usize * width + 96usize) * 4) as usize;
    let interior_index = ((104usize * width + 128usize) * 4) as usize;

    assert!(
        overlay.pixels[border_index + 3] > overlay.pixels[interior_index + 3],
        "brush outline should be more visible than the fill interior"
    );
}

#[test]
fn test_draw_rect_outline_clamps_to_buffer_bounds() {
    let mut pixels = vec![0u8; 4 * 4 * 4];

    draw_rect_outline(
        &mut pixels,
        (4, 4),
        ViewportRect::from_points(ViewportPoint::new(-1.0, -1.0), ViewportPoint::new(3.0, 3.0)),
        Color::new_rgba(255, 128, 64, 255),
        2,
    );

    assert!(
        pixels.chunks_exact(4).any(|pixel| pixel[3] > 0),
        "outline should still draw visible pixels when clamped to the frame"
    );
}

#[test]
fn test_supported_surface_series_use_fast_path_on_full_rerender() {
    let plot: Plot = Plot::new()
        .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
        .title("Fast Path")
        .into();
    let session = plot.prepare_interactive();

    let frame = session
        .render_to_surface(SurfaceTarget {
            size_px: (320, 240),
            scale_factor: 1.0,
            time_seconds: 0.0,
        })
        .expect("supported surface frame should render");

    assert_eq!(frame.surface_capability, SurfaceCapability::FastPath);
    assert!(!frame.layer_state.used_incremental_data);
}

#[test]
fn test_unsupported_surface_series_fall_back_to_image_capability() {
    let plot: Plot = Plot::new()
        .histogram(&[0.0, 1.0, 1.5, 2.0, 2.5], None)
        .into();
    let session = plot.prepare_interactive();

    let frame = session
        .render_to_surface(SurfaceTarget {
            size_px: (320, 240),
            scale_factor: 1.0,
            time_seconds: 0.0,
        })
        .expect("fallback surface frame should render");

    assert_eq!(frame.surface_capability, SurfaceCapability::FallbackImage);
}

#[test]
fn test_streaming_surface_render_uses_incremental_fast_path() {
    let stream = StreamingXY::new(256);
    stream.push_many(vec![(0.0, 0.0), (1.0, 0.5), (2.0, 1.0)]);

    let plot: Plot = Plot::new()
        .line_streaming(&stream)
        .xlim(0.0, 10.0)
        .ylim(-2.0, 2.0)
        .into();
    let session = plot.prepare_interactive();

    session
        .render_to_surface(SurfaceTarget {
            size_px: (320, 240),
            scale_factor: 1.0,
            time_seconds: 0.0,
        })
        .expect("initial surface frame should render");

    stream.push(3.0, 0.75);
    let incremental = session
        .render_to_surface(SurfaceTarget {
            size_px: (320, 240),
            scale_factor: 1.0,
            time_seconds: 0.0,
        })
        .expect("incremental surface frame should render");

    assert!(incremental.layer_state.used_incremental_data);
    assert_eq!(incremental.surface_capability, SurfaceCapability::FastPath);
    assert_eq!(stream.appended_count(), 0);
}

#[test]
fn test_streaming_surface_render_falls_back_after_wraparound() {
    let stream = StreamingXY::new(3);
    stream.push_many(vec![(0.0, 0.0), (1.0, 0.5), (2.0, 1.0)]);

    let plot: Plot = Plot::new()
        .line_streaming(&stream)
        .xlim(0.0, 3.0)
        .ylim(-1.0, 2.0)
        .into();
    let session = plot.prepare_interactive();

    session
        .render_to_surface(render_target())
        .expect("initial wrapped-stream frame should render");

    stream.push(3.0, 1.25);
    let rerendered = session
        .render_to_surface(render_target())
        .expect("wrapped-stream surface frame should render");

    assert!(!rerendered.layer_state.used_incremental_data);
    assert_eq!(stream.appended_count(), 0);
}

#[test]
fn test_hover_and_selection_refresh_after_view_change() {
    let plot: Plot = Plot::new()
        .line(&[0.0, 5.0, 10.0], &[0.0, 5.0, 10.0])
        .title("Overlay Refresh")
        .xlabel("X Label")
        .ylabel("Y Label")
        .xlim(0.0, 10.0)
        .ylim(0.0, 10.0)
        .into();
    let session = plot.prepare_interactive();

    session
        .render_to_surface(render_target())
        .expect("initial surface frame should render");

    let before_geometry = session
        .geometry_snapshot()
        .expect("geometry should be available before hover");
    let (hover_x, hover_y) = map_data_to_pixels(
        5.0,
        5.0,
        before_geometry.x_bounds.0,
        before_geometry.x_bounds.1,
        before_geometry.y_bounds.0,
        before_geometry.y_bounds.1,
        before_geometry.plot_area,
    );
    let hover_px = ViewportPoint::new(hover_x as f64, hover_y as f64);

    session.apply_input(PlotInputEvent::Hover {
        position_px: hover_px,
    });
    session.apply_input(PlotInputEvent::SelectAt {
        position_px: hover_px,
    });
    session.apply_input(PlotInputEvent::Pan {
        delta_px: ViewportPoint::new(36.0, 18.0),
    });
    session
        .render_to_surface(render_target())
        .expect("surface frame should rerender after pan");

    let after_geometry = session
        .geometry_snapshot()
        .expect("geometry should be available after pan");
    let (expected_x, expected_y) = map_data_to_pixels(
        5.0,
        5.0,
        after_geometry.x_bounds.0,
        after_geometry.x_bounds.1,
        after_geometry.y_bounds.0,
        after_geometry.y_bounds.1,
        after_geometry.plot_area,
    );
    let state = session
        .inner
        .state
        .lock()
        .expect("InteractivePlotSession state lock poisoned")
        .clone();
    let hovered = state.hovered.expect("hovered hit should be refreshed");
    let selected = state
        .selected
        .first()
        .cloned()
        .expect("selected hit should be refreshed");
    let tooltip = state.tooltip.expect("hover tooltip should be refreshed");

    match hovered {
        HitResult::SeriesPoint {
            screen_position,
            data_position,
            ..
        } => {
            assert!((screen_position.x - expected_x as f64).abs() < 1e-6);
            assert!((screen_position.y - expected_y as f64).abs() < 1e-6);
            assert_eq!(data_position, ViewportPoint::new(5.0, 5.0));
            assert_eq!(tooltip.position_px, screen_position);
        }
        other => panic!("expected series-point hover hit, got {other:?}"),
    }

    match selected {
        HitResult::SeriesPoint {
            screen_position,
            data_position,
            ..
        } => {
            assert!((screen_position.x - expected_x as f64).abs() < 1e-6);
            assert!((screen_position.y - expected_y as f64).abs() < 1e-6);
            assert_eq!(data_position, ViewportPoint::new(5.0, 5.0));
        }
        other => panic!("expected series-point selection hit, got {other:?}"),
    }
}

#[test]
fn test_hover_refreshes_after_time_change() {
    let temporal_y = signal::of(|time| vec![time, 1.0 + time, 2.0 + time]);
    let plot: Plot = Plot::new()
        .line_source(vec![0.0, 1.0, 2.0], temporal_y)
        .xlim(0.0, 2.0)
        .ylim(0.0, 4.0)
        .into();
    let session = plot.prepare_interactive();

    session
        .render_to_surface(render_target())
        .expect("initial temporal surface frame should render");

    let before_geometry = session
        .geometry_snapshot()
        .expect("geometry should be available before temporal update");
    let (hover_x, hover_y) = map_data_to_pixels(
        1.0,
        1.0,
        before_geometry.x_bounds.0,
        before_geometry.x_bounds.1,
        before_geometry.y_bounds.0,
        before_geometry.y_bounds.1,
        before_geometry.plot_area,
    );
    session.apply_input(PlotInputEvent::Hover {
        position_px: ViewportPoint::new(hover_x as f64, hover_y as f64),
    });

    session
        .render_to_surface(SurfaceTarget {
            time_seconds: 1.0,
            ..render_target()
        })
        .expect("temporal surface frame should render at updated time");

    let after_geometry = session
        .geometry_snapshot()
        .expect("geometry should be available after temporal update");
    let (expected_x, expected_y) = map_data_to_pixels(
        1.0,
        2.0,
        after_geometry.x_bounds.0,
        after_geometry.x_bounds.1,
        after_geometry.y_bounds.0,
        after_geometry.y_bounds.1,
        after_geometry.plot_area,
    );
    let state = session
        .inner
        .state
        .lock()
        .expect("InteractivePlotSession state lock poisoned")
        .clone();
    let hovered = state
        .hovered
        .expect("hovered hit should survive temporal update");
    let tooltip = state
        .tooltip
        .expect("hover tooltip should survive temporal update");

    match hovered {
        HitResult::SeriesPoint {
            screen_position,
            data_position,
            ..
        } => {
            assert_eq!(data_position, ViewportPoint::new(1.0, 2.0));
            assert!((screen_position.x - expected_x as f64).abs() < 1e-6);
            assert!((screen_position.y - expected_y as f64).abs() < 1e-6);
        }
        other => panic!("expected series-point hover hit, got {other:?}"),
    }
    assert_eq!(tooltip.content, "x=1.000, y=2.000");
    assert!((tooltip.position_px.x - expected_x as f64).abs() < 1e-6);
    assert!((tooltip.position_px.y - expected_y as f64).abs() < 1e-6);
}

#[test]
fn test_zoom_keeps_cursor_anchor_stable() {
    let plot: Plot = Plot::new()
        .line(&[0.0, 10.0], &[0.0, 10.0])
        .title("Zoom Anchor")
        .xlabel("Time")
        .ylabel("Value")
        .xlim(0.0, 10.0)
        .ylim(0.0, 10.0)
        .into();
    let session = plot.prepare_interactive();

    session
        .render_to_surface(render_target())
        .expect("initial surface frame should render");

    let before_geometry = session
        .geometry_snapshot()
        .expect("geometry should be available before zoom");
    let anchor_px = ViewportPoint::new(
        0.0,
        before_geometry.plot_area.top() as f64
            + f64::from(before_geometry.plot_area.height()) * 0.5,
    );
    let before_state = session
        .inner
        .state
        .lock()
        .expect("InteractivePlotSession state lock poisoned")
        .clone();
    let before_visible = visible_bounds(&before_state);
    let anchor_before = screen_to_data(before_visible, before_geometry.plot_area, anchor_px);

    session.apply_input(PlotInputEvent::Zoom {
        factor: 2.0,
        center_px: anchor_px,
    });

    let after_state = session
        .inner
        .state
        .lock()
        .expect("InteractivePlotSession state lock poisoned")
        .clone();
    let after_visible = visible_bounds(&after_state);
    let after_geometry = session
        .geometry_snapshot()
        .expect("geometry should be available after zoom");
    let anchor_after = screen_to_data(after_visible, after_geometry.plot_area, anchor_px);

    assert!((anchor_before.x - anchor_after.x).abs() < 1e-9);
    assert!((anchor_before.y - anchor_after.y).abs() < 1e-9);
}

#[test]
fn test_zoom_rect_maps_screen_region_to_visible_bounds() {
    let plot: Plot = Plot::new()
        .line(&[0.0, 10.0], &[0.0, 10.0])
        .title("Zoom Rect")
        .xlabel("X Axis")
        .ylabel("Y Axis")
        .xlim(0.0, 10.0)
        .ylim(0.0, 10.0)
        .into();
    let session = plot.prepare_interactive();

    session
        .render_to_surface(render_target())
        .expect("initial surface frame should render");

    let before_state = session
        .inner
        .state
        .lock()
        .expect("InteractivePlotSession state lock poisoned")
        .clone();
    let before_visible = visible_bounds(&before_state);
    let geometry = session
        .geometry_snapshot()
        .expect("geometry should be available before zoom rect");
    let start_px = ViewportPoint::new(
        geometry.plot_area.left() as f64 + 48.0,
        geometry.plot_area.top() as f64 + 36.0,
    );
    let end_px = ViewportPoint::new(
        geometry.plot_area.left() as f64 + 212.0,
        geometry.plot_area.top() as f64 + 168.0,
    );
    let start_data = screen_to_data(before_visible, geometry.plot_area, start_px);
    let end_data = screen_to_data(before_visible, geometry.plot_area, end_px);

    session.apply_input(PlotInputEvent::ZoomRect {
        region_px: ViewportRect::from_points(start_px, end_px),
    });

    let after_state = session
        .inner
        .state
        .lock()
        .expect("InteractivePlotSession state lock poisoned")
        .clone();
    let after_visible = visible_bounds(&after_state);

    assert!((after_visible.x_min - start_data.x.min(end_data.x)).abs() < 1e-9);
    assert!((after_visible.x_max - start_data.x.max(end_data.x)).abs() < 1e-9);
    assert!((after_visible.y_min - start_data.y.min(end_data.y)).abs() < 1e-9);
    assert!((after_visible.y_max - start_data.y.max(end_data.y)).abs() < 1e-9);
}

#[test]
fn test_pan_uses_plot_area_dimensions() {
    let plot: Plot = Plot::new()
        .line(&[0.0, 10.0], &[0.0, 10.0])
        .title("Pan Scale")
        .xlabel("X Axis")
        .ylabel("Y Axis")
        .xlim(0.0, 10.0)
        .ylim(0.0, 10.0)
        .into();
    let session = plot.prepare_interactive();

    session
        .render_to_surface(render_target())
        .expect("initial surface frame should render");

    let before_state = session
        .inner
        .state
        .lock()
        .expect("InteractivePlotSession state lock poisoned")
        .clone();
    let before_visible = visible_bounds(&before_state);
    let geometry = session
        .geometry_snapshot()
        .expect("geometry should be available before pan");
    let delta_px = ViewportPoint::new(40.0, 24.0);

    session.apply_input(PlotInputEvent::Pan { delta_px });

    let after_state = session
        .inner
        .state
        .lock()
        .expect("InteractivePlotSession state lock poisoned")
        .clone();
    let after_visible = visible_bounds(&after_state);
    let expected_dx =
        -(delta_px.x / f64::from(geometry.plot_area.width())) * before_visible.width();
    let expected_dy =
        (delta_px.y / f64::from(geometry.plot_area.height())) * before_visible.height();

    assert!(((after_visible.x_min - before_visible.x_min) - expected_dx).abs() < 1e-9);
    assert!(((after_visible.x_max - before_visible.x_max) - expected_dx).abs() < 1e-9);
    assert!(((after_visible.y_min - before_visible.y_min) - expected_dy).abs() < 1e-9);
    assert!(((after_visible.y_max - before_visible.y_max) - expected_dy).abs() < 1e-9);
}

#[test]
fn test_temporal_layout_content_uses_current_frame_labels() {
    let xlabel = signal::of(|time| {
        if time < 1.0 {
            "baseline".to_string()
        } else {
            "updated temporal label".to_string()
        }
    });
    let plot: Plot = Plot::new()
        .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
        .xlabel(xlabel)
        .into();
    let visible = DataBounds::from_limits(0.0, 2.0, 0.0, 4.0);

    let baseline_content = plot.create_plot_content(visible.y_min, visible.y_max);
    let current_content = plot.create_plot_content_at_time(visible.y_min, visible.y_max, 1.0);
    let prepared_content = plot
        .prepared_frame_plot(render_target().size_px, render_target().scale_factor, 1.0)
        .create_plot_content(visible.y_min, visible.y_max);

    assert_eq!(baseline_content.xlabel.as_deref(), Some("baseline"));
    assert_eq!(
        current_content.xlabel.as_deref(),
        Some("updated temporal label")
    );
    assert_eq!(prepared_content.xlabel, current_content.xlabel);

    compute_plot_layout(
        &plot,
        render_target().size_px,
        render_target().scale_factor,
        1.0,
        visible,
    )
    .expect("interactive layout should compute using current-frame labels");
}

#[test]
fn test_incremental_line_render_preserves_markers() {
    let stream = StreamingXY::new(32);
    stream.push_many(vec![(0.5, 0.5), (1.0, 1.2)]);

    let plot: Plot = Plot::new()
        .line_streaming(&stream)
        .color(Color::new(220, 20, 20))
        .width(1.0)
        .marker(MarkerStyle::Square)
        .marker_size(18.0)
        .into();
    let plot = plot.ticks(false).grid(false).xlim(0.0, 3.0).ylim(0.0, 3.0);
    let session = plot.prepare_interactive();

    session
        .render_to_surface(render_target())
        .expect("initial line+marker surface frame should render");

    stream.push(2.0, 2.3);
    let incremental = session
        .render_to_surface(render_target())
        .expect("incremental line+marker surface frame should render");
    assert!(incremental.layer_state.used_incremental_data);

    let geometry = session
        .geometry_snapshot()
        .expect("geometry should be available after incremental render");
    let (px, py) = map_data_to_pixels(
        2.0,
        2.3,
        geometry.x_bounds.0,
        geometry.x_bounds.1,
        geometry.y_bounds.0,
        geometry.y_bounds.1,
        geometry.plot_area,
    );
    let point = ViewportPoint::new(px as f64, py as f64);
    let incremental_pixels =
        count_matching_pixels_near(incremental.layers.base.as_ref(), point, 12, |pixel| {
            pixel[3] > 0 && pixel[0] > 150 && pixel[1] < 100 && pixel[2] < 100
        });

    let full = session
        .prepared_plot()
        .plot()
        .prepared_frame_plot(render_target().size_px, render_target().scale_factor, 0.0)
        .xlim(0.0, 3.0)
        .ylim(0.0, 3.0)
        .render()
        .expect("full line+marker render should succeed");
    let full_pixels = count_matching_pixels_near(&full, point, 12, |pixel| {
        pixel[3] > 0 && pixel[0] > 150 && pixel[1] < 100 && pixel[2] < 100
    });

    assert!(incremental_pixels > 0);
    assert!((incremental_pixels as i32 - full_pixels as i32).abs() <= 12);
}

#[test]
fn test_reactive_manual_ylim_stays_pinned_across_updates() {
    let y = Observable::new(vec![0.0, 0.5, 1.0, -0.25]);
    let plot: Plot = Plot::new()
        .line_source(vec![0.0, 1.0, 2.0, 3.0], y.clone())
        .ylim(-2.0, 2.0)
        .into();
    let session = plot.prepare_interactive();

    session
        .render_to_surface(render_target())
        .expect("initial surface frame should render");
    let first_geometry = session
        .geometry_snapshot()
        .expect("geometry should be available after initial render");
    let first_ticks = derived_y_ticks(&session);

    y.set(vec![12.0, -15.0, 8.0, -11.0]);
    session
        .render_to_surface(render_target())
        .expect("surface frame after first reactive update should render");
    let second_geometry = session
        .geometry_snapshot()
        .expect("geometry should be available after first reactive update");
    let second_ticks = derived_y_ticks(&session);

    y.set(vec![30.0, 5.0, -22.0, 18.0]);
    session
        .render_to_surface(render_target())
        .expect("surface frame after second reactive update should render");
    let third_geometry = session
        .geometry_snapshot()
        .expect("geometry should be available after second reactive update");
    let third_ticks = derived_y_ticks(&session);

    assert_eq!(first_geometry.y_bounds, (-2.0, 2.0));
    assert_eq!(second_geometry.y_bounds, (-2.0, 2.0));
    assert_eq!(third_geometry.y_bounds, (-2.0, 2.0));
    assert_eq!(first_ticks, second_ticks);
    assert_eq!(second_ticks, third_ticks);
}

#[test]
fn test_dashboard_like_reactive_updates_do_not_drift_manual_ylim() {
    let x: Vec<f64> = (0..120).map(|index| index as f64 * 12.0 / 119.0).collect();
    let primary = Observable::new(
        x.iter()
            .map(|value| 0.85 * value.sin() + 0.2 * (value * 3.0).cos())
            .collect::<Vec<_>>(),
    );
    let baseline = Observable::new(
        x.iter()
            .map(|value| 0.4 * (value * 0.75).sin())
            .collect::<Vec<_>>(),
    );
    let event_x = Observable::new(vec![2.0, 6.0, 9.5]);
    let event_y = Observable::new(vec![1.1, -1.3, 1.4]);
    let accent = Observable::new(Color::new(42, 157, 143));

    let plot: Plot = Plot::new()
        .line(&x, &vec![1.2; x.len()])
        .color(Color::LIGHT_GRAY)
        .into();
    let plot: Plot = plot
        .line(&x, &vec![-1.2; x.len()])
        .color(Color::LIGHT_GRAY)
        .into();
    let plot: Plot = plot
        .line_source(x.clone(), primary.clone())
        .color_source(accent.clone())
        .line_width(2.4)
        .into();
    let plot: Plot = plot
        .line_source(x.clone(), baseline.clone())
        .color(Color::new(38, 70, 83))
        .line_width(1.6)
        .into();
    let plot: Plot = plot
        .scatter_source(event_x.clone(), event_y.clone())
        .color(Color::new(231, 111, 81))
        .marker(MarkerStyle::Diamond)
        .marker_size(9.0)
        .xlim(0.0, 12.0)
        .ylim(-2.0, 2.0)
        .into();
    let session = plot.prepare_interactive();

    session
        .render_to_surface(render_target())
        .expect("initial dashboard-like surface frame should render");
    let first_geometry = session
        .geometry_snapshot()
        .expect("geometry should be available after initial dashboard render");
    let first_ticks = derived_y_ticks(&session);

    primary.set(
        x.iter()
            .map(|value| 1.6 * (value * 1.3).sin() + 0.5 * (value * 2.8).cos())
            .collect::<Vec<_>>(),
    );
    baseline.set(
        x.iter()
            .map(|value| 0.55 * (value * 0.65).sin() - 0.2 * (value * 2.1).cos())
            .collect::<Vec<_>>(),
    );
    event_x.set(vec![1.0, 4.5, 10.5]);
    event_y.set(vec![1.5, -1.4, 1.7]);
    accent.set(Color::new(231, 111, 81));

    session
        .render_to_surface(render_target())
        .expect("dashboard-like surface frame after reactive updates should render");
    let second_geometry = session
        .geometry_snapshot()
        .expect("geometry should be available after dashboard reactive updates");
    let second_ticks = derived_y_ticks(&session);

    assert_eq!(first_geometry.y_bounds, (-2.0, 2.0));
    assert_eq!(second_geometry.y_bounds, (-2.0, 2.0));
    assert_eq!(first_ticks, second_ticks);
}

#[test]
fn test_surface_frame_pixels_honor_manual_ylim() {
    let plot: Plot = Plot::new()
        .scatter(&[0.5], &[1.0])
        .color(Color::new(220, 20, 20))
        .marker(MarkerStyle::Square)
        .marker_size(18.0)
        .ticks(false)
        .grid(false)
        .into();
    let plot: Plot = plot
        .scatter(&[0.5], &[-1.0])
        .color(Color::new(20, 20, 220))
        .marker(MarkerStyle::Square)
        .marker_size(18.0)
        .ticks(false)
        .grid(false)
        .xlim(0.0, 1.0)
        .ylim(-2.0, 2.0)
        .into();
    let session = plot.prepare_interactive();
    let plain_plot = plot.clone().set_output_pixels(320, 240);
    let render_plot = session
        .prepared_plot()
        .plot()
        .prepared_frame_plot(render_target().size_px, render_target().scale_factor, 0.0)
        .xlim(0.0, 1.0)
        .ylim(-2.0, 2.0);

    assert_eq!(plain_plot.layout.y_limits, Some((-2.0, 2.0)));
    assert_eq!(render_plot.layout.y_limits, Some((-2.0, 2.0)));
    assert_eq!(render_plot.layout.x_limits, Some((0.0, 1.0)));

    let plain = plain_plot.render().expect("plain plot should render");
    let plain_red_center = color_centroid(&plain, |pixel| {
        pixel[3] > 0 && pixel[0] > 160 && pixel[1] < 80 && pixel[2] < 80
    })
    .expect("plain red marker pixels should be present");
    let direct = render_plot
        .render()
        .expect("direct prepared frame should render");
    let direct_red_center = color_centroid(&direct, |pixel| {
        pixel[3] > 0 && pixel[0] > 160 && pixel[1] < 80 && pixel[2] < 80
    })
    .expect("direct red marker pixels should be present");

    let visible = DataBounds::from_limits(0.0, 1.0, -2.0, 2.0);
    let layout = compute_plot_layout(
        &plain_plot,
        render_target().size_px,
        render_target().scale_factor,
        0.0,
        visible,
    )
    .expect("plot layout should compute for manual bounds");

    let frame = session
        .render_to_surface(render_target())
        .expect("surface frame should render");
    let base = frame.layers.base.as_ref();

    let red_center = color_centroid(base, |pixel| {
        pixel[3] > 0 && pixel[0] > 160 && pixel[1] < 80 && pixel[2] < 80
    })
    .expect("red marker pixels should be present");
    let blue_center = color_centroid(base, |pixel| {
        pixel[3] > 0 && pixel[0] < 80 && pixel[1] < 80 && pixel[2] > 160
    })
    .expect("blue marker pixels should be present");

    let expected_red_y = map_data_to_pixels(
        0.5,
        1.0,
        visible.x_min,
        visible.x_max,
        visible.y_min,
        visible.y_max,
        layout.plot_area_rect,
    )
    .1 as f64;

    assert!(
        (plain_red_center.y - expected_red_y).abs() <= 12.0,
        "plain render red marker y={} should be close to expected {}",
        plain_red_center.y,
        expected_red_y
    );
    assert!(
        (direct_red_center.y - expected_red_y).abs() <= 12.0,
        "direct render red marker y={} should be close to expected {}",
        direct_red_center.y,
        expected_red_y
    );
    let expected_blue_y = map_data_to_pixels(
        0.5,
        -1.0,
        visible.x_min,
        visible.x_max,
        visible.y_min,
        visible.y_max,
        layout.plot_area_rect,
    )
    .1 as f64;

    assert!(
        (red_center.y - expected_red_y).abs() <= 12.0,
        "red marker y={} should be close to expected {}",
        red_center.y,
        expected_red_y
    );
    assert!(
        (blue_center.y - expected_blue_y).abs() <= 12.0,
        "blue marker y={} should be close to expected {}",
        blue_center.y,
        expected_blue_y
    );
}

#[test]
fn test_surface_frame_clips_series_pixels_to_plot_area_after_zoom() {
    let plot: Plot = Plot::new()
        .line(&[0.0, 5.0, 10.0], &[0.0, 5.0, 10.0])
        .color(Color::new(220, 20, 20))
        .line_width(18.0)
        .ticks(false)
        .grid(false)
        .xlim(0.0, 10.0)
        .ylim(0.0, 10.0)
        .into();
    let session = plot.prepare_interactive();

    session
        .render_to_surface(render_target())
        .expect("initial surface frame should render");

    let geometry = session
        .geometry_snapshot()
        .expect("geometry should be available before zoom");
    let zoom_region = ViewportRect::from_points(
        ViewportPoint::new(
            geometry.plot_area.left() as f64 + 64.0,
            geometry.plot_area.top() as f64 + 40.0,
        ),
        ViewportPoint::new(
            geometry.plot_area.right() as f64 - 64.0,
            geometry.plot_area.bottom() as f64 - 40.0,
        ),
    );
    session.apply_input(PlotInputEvent::ZoomRect {
        region_px: zoom_region,
    });

    let frame = session
        .render_to_surface(render_target())
        .expect("zoomed surface frame should render");
    let geometry = session
        .geometry_snapshot()
        .expect("geometry should be available after zoom");
    let base = frame.layers.base.as_ref();

    let red_pixels = color_centroid(base, |pixel| {
        pixel[3] > 0 && pixel[0] > 160 && pixel[1] < 80 && pixel[2] < 80
    });
    let leaked_red_pixels = count_matching_pixels_outside_rect(base, geometry.plot_area, |pixel| {
        pixel[3] > 0 && pixel[0] > 160 && pixel[1] < 80 && pixel[2] < 80
    });
    let red_bounds = matching_pixel_bounds(base, |pixel| {
        pixel[3] > 0 && pixel[0] > 160 && pixel[1] < 80 && pixel[2] < 80
    });

    assert!(red_pixels.is_some(), "expected red line pixels after zoom");
    assert_eq!(
        leaked_red_pixels, 0,
        "expected no strong red series pixels outside plot area after zoom; plot_area={:?}; red_bounds={red_bounds:?}",
        geometry.plot_area
    );
}
