use super::*;
use std::collections::HashMap;
use std::sync::Arc;

fn pixel_is_dark(image: &Image, x: u32, y: u32) -> bool {
    let idx = ((y * image.width + x) * 4) as usize;
    image.pixels[idx..idx + 3]
        .iter()
        .all(|channel| *channel < 220)
}

fn pixel_is_bright_rgba(image: &image::RgbaImage, x: u32, y: u32) -> bool {
    let pixel = image.get_pixel(x, y).0;
    pixel[0] > 200 && pixel[1] > 200 && pixel[2] > 200 && pixel[3] > 0
}

fn image_pixel_rgba(image: &Image, x: u32, y: u32) -> [u8; 4] {
    let idx = ((y * image.width + x) * 4) as usize;
    [
        image.pixels[idx],
        image.pixels[idx + 1],
        image.pixels[idx + 2],
        image.pixels[idx + 3],
    ]
}

fn dark_pixel_bounds(image: &Image) -> Option<(u32, u32, u32, u32)> {
    let mut min_x = image.width;
    let mut min_y = image.height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;

    for y in 0..image.height {
        for x in 0..image.width {
            let pixel = image_pixel_rgba(image, x, y);
            if pixel[3] > 0 && pixel[0] < 230 && pixel[1] < 230 && pixel[2] < 230 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }

    found.then_some((min_x, min_y, max_x, max_y))
}

fn assert_exact_rgba_pixels(name: &str, reference: &Image, candidate: &Image) {
    assert_eq!(reference.width, candidate.width, "{name} width changed");
    assert_eq!(reference.height, candidate.height, "{name} height changed");
    assert_eq!(reference.pixels, candidate.pixels, "{name} pixels changed");
}

fn mean_normalized_rgba_diff(reference: &Image, candidate: &Image) -> f64 {
    assert_eq!(reference.width, candidate.width);
    assert_eq!(reference.height, candidate.height);

    reference
        .pixels
        .iter()
        .zip(candidate.pixels.iter())
        .map(|(lhs, rhs)| (*lhs as f64 - *rhs as f64).abs() / 255.0)
        .sum::<f64>()
        / reference.pixels.len() as f64
}

fn fraction_pixels_within_channel_delta(
    reference: &Image,
    candidate: &Image,
    max_delta: u8,
) -> f64 {
    assert_eq!(reference.width, candidate.width);
    assert_eq!(reference.height, candidate.height);

    let matching = reference
        .pixels
        .chunks_exact(4)
        .zip(candidate.pixels.chunks_exact(4))
        .filter(|(lhs, rhs)| {
            lhs.iter()
                .zip(rhs.iter())
                .all(|(left, right)| (*left as i16 - *right as i16).abs() <= max_delta as i16)
        })
        .count() as f64;
    matching / (reference.width * reference.height) as f64
}

fn non_background_fraction(pixels: &[u8]) -> f64 {
    let mut non_background = 0usize;
    let mut total = 0usize;
    for pixel in pixels.chunks_exact(4) {
        total += 1;
        if pixel[3] > 0 && (pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0) {
            non_background += 1;
        }
    }
    non_background as f64 / total.max(1) as f64
}

fn assert_premultiplied_parity_against_reference(name: &str, reference: &Image, candidate: &Image) {
    let mean_diff = mean_normalized_rgba_diff(reference, candidate);
    let within_delta = fraction_pixels_within_channel_delta(reference, candidate, 24);
    let reference_ink = non_background_fraction(&reference.pixels);
    let candidate_ink = non_background_fraction(&candidate.pixels);

    assert!(
        mean_diff <= 0.015,
        "{name} drifted too far from the reference render: mean_diff={mean_diff:.6}"
    );
    assert!(
        within_delta >= 0.99,
        "{name} has too many per-pixel outliers relative to the reference render: within_delta={within_delta:.4}"
    );
    assert!(
        (reference_ink - candidate_ink).abs() <= 0.10,
        "{name} changed visible ink coverage too much: reference_ink={reference_ink:.4} candidate_ink={candidate_ink:.4}"
    );
}

fn test_marker_sprite() -> Arc<MarkerSprite> {
    Arc::new(MarkerSprite {
        width: 1,
        height: 1,
        origin_x: 0,
        origin_y: 0,
        pixels: vec![0, 0, 0, 0],
        scanlines: None,
    })
}

fn test_marker_sprite_key(index: usize) -> MarkerSpriteKey {
    MarkerSpriteKey {
        style: MarkerStyle::Circle,
        size_bits: index as u32,
        rgba_bits: 0,
        phase_x: 0,
        phase_y: 0,
    }
}

#[test]
fn test_global_marker_sprite_cache_evicts_one_entry_at_limit() {
    let mut cache = HashMap::new();
    let sprite = test_marker_sprite();

    for index in 0..GLOBAL_MARKER_SPRITE_CACHE_LIMIT {
        cache.insert(test_marker_sprite_key(index), Arc::clone(&sprite));
    }

    let incoming_key = test_marker_sprite_key(GLOBAL_MARKER_SPRITE_CACHE_LIMIT + 1);
    insert_global_marker_sprite(&mut cache, incoming_key, Arc::clone(&sprite));

    let retained_existing_entries = (0..GLOBAL_MARKER_SPRITE_CACHE_LIMIT)
        .filter(|&index| cache.contains_key(&test_marker_sprite_key(index)))
        .count();

    assert_eq!(cache.len(), GLOBAL_MARKER_SPRITE_CACHE_LIMIT);
    assert_eq!(
        retained_existing_entries,
        GLOBAL_MARKER_SPRITE_CACHE_LIMIT - 1
    );
    assert!(cache.contains_key(&incoming_key));
}

#[test]
fn test_global_marker_sprite_cache_keeps_existing_entry() {
    let mut cache = HashMap::new();
    let key = test_marker_sprite_key(1);
    let existing = test_marker_sprite();
    let replacement = test_marker_sprite();

    cache.insert(key, Arc::clone(&existing));
    let returned = insert_global_marker_sprite(&mut cache, key, replacement);

    assert!(Arc::ptr_eq(&returned, &existing));
    assert_eq!(cache.len(), 1);
}

fn legacy_draw_circle(
    renderer: &mut SkiaRenderer,
    x: f32,
    y: f32,
    radius: f32,
    color: Color,
    filled: bool,
) {
    let mut paint = Paint::default();
    paint.set_color(color.to_tiny_skia_color());
    paint.anti_alias = true;

    let mut path = PathBuilder::new();
    path.push_circle(x, y, radius);
    let path = path.finish().expect("legacy circle path");

    if filled {
        renderer.pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    } else {
        renderer.pixmap.stroke_path(
            &path,
            &paint,
            &Stroke::default(),
            Transform::identity(),
            None,
        );
    }
}

fn legacy_draw_composited_rect(
    renderer: &mut SkiaRenderer,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
) {
    let rect = Rect::from_xywh(x, y, width, height).expect("legacy rect");
    let mut path = PathBuilder::new();
    path.push_rect(rect);
    let path = path.finish().expect("legacy rectangle path");
    let mut paint = Paint::default();
    paint.set_color(color.to_tiny_skia_color());
    paint.anti_alias = true;
    renderer.pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn legacy_draw_solid_rect(
    renderer: &mut SkiaRenderer,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
) {
    let rect = Rect::from_xywh(x, y, width, height).expect("legacy solid rect");
    let mut path = PathBuilder::new();
    path.push_rect(rect);
    let path = path.finish().expect("legacy solid rectangle path");
    let mut paint = Paint::default();
    paint.set_color(color.to_tiny_skia_color());
    paint.anti_alias = false;
    renderer.pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn count_red_pixels_outside_rect(image: &Image, rect: Rect) -> usize {
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

            let idx = ((y as u32 * image.width + x as u32) * 4) as usize;
            let pixel = &image.pixels[idx..idx + 4];
            if pixel[3] > 0 && pixel[0] > 160 && pixel[1] < 80 && pixel[2] < 80 {
                count += 1;
            }
        }
    }

    count
}

fn marker_parity_points() -> Vec<crate::core::types::Point2f> {
    (0..160)
        .map(|index| {
            let x_base = ((index * 17) % 90) as f32 * 0.75 - 8.0;
            let y_base = ((index * 23) % 88) as f32 * 0.68 - 7.0;
            let x = x_base + [0.06, 0.21, 0.44, 0.57, 0.81][index % 5];
            let y = y_base + [0.11, 0.29, 0.38, 0.63, 0.87][index % 5];
            crate::core::types::Point2f::new(x, y)
        })
        .collect()
}

#[test]
fn test_renderer_creation() {
    let theme = Theme::default();
    let renderer = SkiaRenderer::new(800, 600, theme);
    assert!(renderer.is_ok());

    let renderer = renderer.unwrap();
    assert_eq!(renderer.width, 800);
    assert_eq!(renderer.height, 600);
}

#[test]
fn test_set_dpi_scale_sanitizes_invalid_values() {
    let theme = Theme::default();
    let mut renderer = SkiaRenderer::new(100, 100, theme).unwrap();

    renderer.set_dpi_scale(2.5);
    assert!((renderer.dpi_scale() - 2.5).abs() < f32::EPSILON);

    renderer.set_dpi_scale(0.0);
    assert!((renderer.dpi_scale() - 1.0).abs() < f32::EPSILON);

    renderer.set_dpi_scale(-3.0);
    assert!((renderer.dpi_scale() - 1.0).abs() < f32::EPSILON);

    renderer.set_dpi_scale(f32::NAN);
    assert!((renderer.dpi_scale() - 1.0).abs() < f32::EPSILON);

    renderer.set_dpi_scale(f32::INFINITY);
    assert!((renderer.dpi_scale() - 1.0).abs() < f32::EPSILON);
}

#[test]
fn test_plot_area_calculation() {
    let area = calculate_plot_area(800, 600, 0.1);
    assert_eq!(area.left(), 80.0);
    assert_eq!(area.top(), 60.0);
    assert_eq!(area.width(), 640.0);
    assert_eq!(area.height(), 480.0);
}

#[test]
fn test_data_to_pixel_mapping() {
    let plot_area = Rect::from_xywh(100.0, 100.0, 600.0, 400.0).unwrap();
    let (px, py) = map_data_to_pixels(
        1.5, 2.5, // data coordinates
        1.0, 2.0, // data x range
        2.0, 3.0, // data y range
        plot_area,
    );

    assert_eq!(px, 400.0); // middle of x range
    assert_eq!(py, 300.0); // middle of y range (flipped)
}

#[test]
fn test_tick_generation() {
    let ticks = generate_ticks(0.0, 10.0, 5);
    assert!(!ticks.is_empty());
    assert!(ticks[0] >= 0.0);
    assert!(ticks.last().unwrap() <= &10.0);

    // Test edge case
    let ticks = generate_ticks(5.0, 5.0, 3);
    assert_eq!(ticks, vec![5.0, 5.0]);
}

#[test]
fn test_compute_colorbar_ticks_formats_log_decades_and_minor_ticks() {
    let ticks = compute_colorbar_ticks(1e-5, 1e3, &crate::axes::AxisScale::Log, true);

    assert_eq!(ticks.major_labels.first().map(String::as_str), Some("10⁻⁵"));
    assert_eq!(ticks.major_labels.last().map(String::as_str), Some("10³"));
    assert!(ticks.minor_values.contains(&2e-5));
    assert!(ticks.minor_values.contains(&900.0));
}

#[test]
fn test_colorbar_layout_metrics_keep_rotated_label_after_tick_labels() {
    let metrics = super::compute_colorbar_layout_metrics(20.0, 12.0, 36.0, Some(14.0));

    assert!((metrics.major_tick_width - 6.0).abs() < 1e-6);
    assert!((metrics.minor_tick_width - 3.6).abs() < 1e-6);
    assert!(metrics.tick_label_x_offset > 20.0);
    assert_eq!(metrics.rotated_label_center_x_offset, Some(78.0));
    assert!((metrics.total_extent - 85.0).abs() < 1e-6);
}

#[test]
fn test_colorbar_major_label_top_centers_label_on_tick() {
    let tick_y = 48.0;
    let label_center_from_top = 9.5;
    let label_top = super::colorbar_major_label_top(tick_y, label_center_from_top);

    assert!(((label_top + label_center_from_top) - tick_y).abs() < f32::EPSILON);
}

#[test]
fn test_colorbar_major_label_anchor_center_uses_base_center_for_log_decades() {
    let anchor_center = super::colorbar_major_label_anchor_center_from_top(
        &crate::axes::AxisScale::Log,
        "10²",
        8.0,
        Some(6.0),
    );

    assert!((anchor_center - 6.0).abs() < f32::EPSILON);
}

#[test]
fn test_colorbar_major_label_anchor_center_keeps_rendered_center_for_non_log_labels() {
    let anchor_center = super::colorbar_major_label_anchor_center_from_top(
        &crate::axes::AxisScale::Linear,
        "6",
        8.0,
        Some(6.0),
    );

    assert!((anchor_center - 8.0).abs() < f32::EPSILON);
}

#[test]
fn test_draw_pixel_aligned_solid_rectangle_blends_transparent_fill() {
    let theme = Theme::default();
    let mut renderer = SkiaRenderer::new(6, 6, theme).unwrap();
    renderer
        .pixmap
        .fill(crate::render::Color::new(0, 0, 255).to_tiny_skia_color());

    renderer
        .draw_pixel_aligned_solid_rectangle(
            1.0,
            1.0,
            4.0,
            4.0,
            crate::render::Color::new_rgba(255, 0, 0, 128),
        )
        .expect("transparent pixel-aligned fill should composite successfully");

    let image = renderer.into_image();
    let pixel = image_pixel_rgba(&image, 2, 2);
    assert!(
        pixel[0] > 0,
        "composited fill should retain red contribution"
    );
    assert!(
        pixel[2] > 0,
        "composited fill should retain background contribution instead of overwriting it"
    );
    assert_eq!(pixel[3], 255);
}

#[test]
fn test_draw_circle_cached_path_matches_legacy_circle_pixels() {
    let theme = Theme::default();
    let mut candidate = SkiaRenderer::new(64, 64, theme.clone()).unwrap();
    let mut reference = SkiaRenderer::new(64, 64, theme).unwrap();
    candidate.pixmap.fill(tiny_skia::Color::TRANSPARENT);
    reference.pixmap.fill(tiny_skia::Color::TRANSPARENT);

    candidate
        .draw_circle(22.5, 19.25, 6.0, Color::new(30, 120, 220), true)
        .expect("cached circle should render");
    legacy_draw_circle(
        &mut reference,
        22.5,
        19.25,
        6.0,
        Color::new(30, 120, 220),
        true,
    );

    assert_exact_rgba_pixels(
        "cached circle path",
        &reference.into_image(),
        &candidate.into_image(),
    );
}

#[test]
fn test_draw_markers_clipped_sprite_compositor_stays_in_parity_for_supported_marker_styles() {
    let theme = Theme::default();
    let points = marker_parity_points();
    let clip_rect = (6.25, 5.5, 46.5, 44.25);
    let color = Color::new_rgba(35, 140, 220, 216);

    for style in [
        MarkerStyle::Circle,
        MarkerStyle::Square,
        MarkerStyle::Triangle,
        MarkerStyle::TriangleDown,
        MarkerStyle::Diamond,
        MarkerStyle::CircleOpen,
    ] {
        let mut candidate = SkiaRenderer::new(64, 56, theme.clone()).unwrap();
        let mut reference = SkiaRenderer::new(64, 56, theme.clone()).unwrap();
        candidate.pixmap.fill(tiny_skia::Color::TRANSPARENT);
        reference.pixmap.fill(tiny_skia::Color::TRANSPARENT);

        candidate
            .draw_markers_clipped(&points, 8.5, style, color, clip_rect)
            .expect("sprite-composited markers should render");
        for point in &points {
            reference
                .draw_marker_clipped(point.x, point.y, 8.5, style, color, clip_rect)
                .expect("legacy markers should render");
        }

        assert_premultiplied_parity_against_reference(
            style.name(),
            &reference.into_image(),
            &candidate.into_image(),
        );
    }
}

#[test]
fn test_draw_markers_clipped_uses_scanline_blit_for_supported_filled_markers() {
    let theme = Theme::default();
    let points = marker_parity_points();
    let clip_rect = (0.0, 0.0, 64.0, 56.0);
    let color = Color::new_rgba(35, 140, 220, 216);

    for style in [
        MarkerStyle::Circle,
        MarkerStyle::Square,
        MarkerStyle::Triangle,
        MarkerStyle::TriangleDown,
    ] {
        let mut candidate = SkiaRenderer::new(64, 56, theme.clone()).unwrap();
        let mut reference = SkiaRenderer::new(64, 56, theme.clone()).unwrap();
        candidate.pixmap.fill(tiny_skia::Color::TRANSPARENT);
        reference.pixmap.fill(tiny_skia::Color::TRANSPARENT);

        candidate
            .draw_markers_clipped(&points, 8.5, style, color, clip_rect)
            .expect("scanline markers should render");
        for point in &points {
            reference
                .draw_marker_clipped(point.x, point.y, 8.5, style, color, clip_rect)
                .expect("legacy markers should render");
        }

        assert!(candidate.render_diagnostics().used_marker_scanline_blit);
        assert_premultiplied_parity_against_reference(
            style.name(),
            &reference.into_image(),
            &candidate.into_image(),
        );
    }
}

#[test]
fn test_draw_markers_clipped_uses_vector_fallback_for_line_based_markers() {
    let theme = Theme::default();
    let points = marker_parity_points();
    let clip_rect = (6.25, 5.5, 46.5, 44.25);
    let color = Color::new_rgba(180, 60, 220, 216);

    for style in [
        MarkerStyle::Plus,
        MarkerStyle::Cross,
        MarkerStyle::Star,
        MarkerStyle::SquareOpen,
        MarkerStyle::TriangleOpen,
        MarkerStyle::DiamondOpen,
    ] {
        let mut candidate = SkiaRenderer::new(64, 56, theme.clone()).unwrap();
        let mut reference = SkiaRenderer::new(64, 56, theme.clone()).unwrap();
        candidate.pixmap.fill(tiny_skia::Color::TRANSPARENT);
        reference.pixmap.fill(tiny_skia::Color::TRANSPARENT);

        candidate
            .draw_markers_clipped(&points, 8.5, style, color, clip_rect)
            .expect("fallback markers should render");
        for point in &points {
            reference
                .draw_marker_clipped(point.x, point.y, 8.5, style, color, clip_rect)
                .expect("legacy markers should render");
        }

        assert!(candidate.render_diagnostics().used_marker_sprite_fallback);
        assert!(!candidate.render_diagnostics().used_marker_sprite_compositor);
        assert_exact_rgba_pixels(
            style.name(),
            &reference.into_image(),
            &candidate.into_image(),
        );
    }
}

#[test]
fn test_draw_pixel_aligned_solid_rectangle_fallback_matches_legacy_rect_fill() {
    let theme = Theme::default();
    let mut candidate = SkiaRenderer::new(32, 24, theme.clone()).unwrap();
    let mut reference = SkiaRenderer::new(32, 24, theme).unwrap();
    candidate.pixmap.fill(tiny_skia::Color::TRANSPARENT);
    reference.pixmap.fill(tiny_skia::Color::TRANSPARENT);

    candidate
        .draw_pixel_aligned_solid_rectangle(
            3.25,
            4.5,
            5.75,
            6.25,
            Color::new_rgba(200, 80, 20, 180),
        )
        .expect("fallback rectangle should render");
    legacy_draw_composited_rect(
        &mut reference,
        3.25,
        4.5,
        5.75,
        6.25,
        Color::new_rgba(200, 80, 20, 180),
    );

    assert_exact_rgba_pixels(
        "direct composited rectangle fill",
        &reference.into_image(),
        &candidate.into_image(),
    );
}

#[test]
fn test_draw_solid_rectangle_matches_legacy_fill_pixels() {
    let theme = Theme::default();
    let mut candidate = SkiaRenderer::new(32, 24, theme.clone()).unwrap();
    let mut reference = SkiaRenderer::new(32, 24, theme).unwrap();
    candidate.pixmap.fill(tiny_skia::Color::TRANSPARENT);
    reference.pixmap.fill(tiny_skia::Color::TRANSPARENT);

    candidate
        .draw_solid_rectangle(3.25, 4.5, 5.75, 6.25, Color::new(40, 150, 210))
        .expect("solid rectangle should render");
    legacy_draw_solid_rect(
        &mut reference,
        3.25,
        4.5,
        5.75,
        6.25,
        Color::new(40, 150, 210),
    );

    assert_exact_rgba_pixels(
        "solid rectangle fill",
        &reference.into_image(),
        &candidate.into_image(),
    );
}

#[test]
fn test_draw_pixel_aligned_solid_rectangle_preserves_subpixel_tiles() {
    let theme = Theme::default();
    let mut renderer = SkiaRenderer::new(8, 8, theme).unwrap();
    renderer.pixmap.fill(tiny_skia::Color::TRANSPARENT);

    renderer
        .draw_pixel_aligned_solid_rectangle(3.2, 1.0, 0.25, 6.0, crate::render::Color::RED)
        .expect("subpixel-aligned fill should render via composited fallback");

    let image = renderer.into_image();
    assert!(
        image.pixels.chunks_exact(4).any(|pixel| pixel[3] > 0),
        "thin pixel-aligned tiles should still contribute visible coverage"
    );
}

#[test]
fn test_draw_axes_with_config_draws_top_and_right_ticks() {
    let theme = Theme::default();
    let mut renderer = SkiaRenderer::new(120, 100, theme).unwrap();
    let plot_area = Rect::from_xywh(20.0, 20.0, 80.0, 60.0).unwrap();

    renderer
        .draw_axes_with_config(
            plot_area,
            &[60.0],
            &[50.0],
            &[],
            &[],
            &TickDirection::Inside,
            &TickSides::all(),
            Color::BLACK,
            1.0,
        )
        .unwrap();

    let image = renderer.into_image();
    assert!(pixel_is_dark(&image, 60, 20));
    assert!(pixel_is_dark(&image, 100, 50));
    assert!(pixel_is_dark(&image, 60, 24));
    assert!(pixel_is_dark(&image, 96, 50));
}

#[test]
fn test_draw_axes_with_config_respects_bottom_left_ticks() {
    let theme = Theme::default();
    let mut renderer = SkiaRenderer::new(120, 100, theme).unwrap();
    let plot_area = Rect::from_xywh(20.0, 20.0, 80.0, 60.0).unwrap();

    renderer
        .draw_axes_with_config(
            plot_area,
            &[60.0],
            &[50.0],
            &[],
            &[],
            &TickDirection::Inside,
            &TickSides::bottom_left(),
            Color::BLACK,
            1.0,
        )
        .unwrap();

    let image = renderer.into_image();
    assert!(pixel_is_dark(&image, 60, 20));
    assert!(pixel_is_dark(&image, 100, 50));
    assert!(!pixel_is_dark(&image, 60, 24));
    assert!(!pixel_is_dark(&image, 96, 50));
    assert!(pixel_is_dark(&image, 60, 76));
    assert!(pixel_is_dark(&image, 24, 50));
}

#[test]
fn test_draw_axis_labels_at_handles_collapsed_ranges() {
    let theme = Theme::default();
    let mut renderer = SkiaRenderer::new(120, 100, theme).unwrap();
    let plot_area = LayoutRect {
        left: 20.0,
        top: 20.0,
        right: 100.0,
        bottom: 80.0,
    };

    renderer
        .draw_axis_labels_at(
            &plot_area,
            1.0,
            1.0,
            2.0,
            2.0,
            &[1.0],
            &[2.0],
            88.0,
            18.0,
            10.0,
            Color::BLACK,
            100.0,
            true,
            false,
        )
        .expect("collapsed ranges should use centered label placement");
}

#[test]
fn test_draw_polyline_clipped_keeps_pixels_inside_clip_rect() {
    let theme = Theme::default();
    let mut renderer = SkiaRenderer::new(120, 120, theme).unwrap();
    let clip_rect = Rect::from_xywh(20.0, 20.0, 80.0, 80.0).unwrap();

    renderer
        .draw_polyline_clipped(
            &[(20.0, 20.0), (100.0, 100.0)],
            Color::new(220, 20, 20),
            18.0,
            LineStyle::Solid,
            (
                clip_rect.x(),
                clip_rect.y(),
                clip_rect.width(),
                clip_rect.height(),
            ),
        )
        .unwrap();

    let image = renderer.into_image();
    assert_eq!(count_red_pixels_outside_rect(&image, clip_rect), 0);
}

#[test]
fn test_draw_datashader_image_scales_into_plot_area() {
    let theme = Theme::dark();
    let mut renderer = SkiaRenderer::new(80, 60, theme).unwrap();
    renderer.clear();

    let image = crate::data::DataShaderImage::new(1, 1, vec![255, 255, 255, 255]);
    let plot_area = Rect::from_xywh(10.0, 15.0, 30.0, 20.0).unwrap();

    renderer.draw_datashader_image(&image, plot_area).unwrap();

    let png = renderer.encode_png_bytes().unwrap();
    let rendered = image::load_from_memory(&png).unwrap().to_rgba8();
    assert!(pixel_is_bright_rgba(&rendered, 15, 20));
    assert!(pixel_is_bright_rgba(&rendered, 35, 30));
    assert!(!pixel_is_bright_rgba(&rendered, 5, 5));
    assert!(!pixel_is_bright_rgba(&rendered, 50, 45));
}

#[test]
fn test_draw_datashader_image_preserves_transparent_bins() {
    let theme = Theme::light();
    let mut renderer = SkiaRenderer::new(20, 10, theme).unwrap();
    renderer.clear();

    let image = crate::data::DataShaderImage::new(
        2,
        1,
        vec![
            0, 0, 0, 0, // Transparent bin
            0, 0, 0, 255, // Fully occupied bin
        ],
    );
    let plot_area = Rect::from_xywh(0.0, 0.0, 20.0, 10.0).unwrap();

    renderer.draw_datashader_image(&image, plot_area).unwrap();

    let png = renderer.encode_png_bytes().unwrap();
    let rendered = image::load_from_memory(&png).unwrap().to_rgba8();
    assert!(
        pixel_is_bright_rgba(&rendered, 2, 5),
        "transparent bins should preserve the white plot background"
    );
    assert!(
        !pixel_is_bright_rgba(&rendered, 17, 5),
        "occupied bins should still draw a visible foreground tint"
    );
}

#[cfg(feature = "typst-math")]
#[test]
fn test_typst_raster_uses_native_1x_scale() {
    let theme = Theme::default();
    let mut renderer = SkiaRenderer::new(400, 300, theme).unwrap();
    renderer.set_dpi_scale(1.0);
    renderer.set_text_engine_mode(TextEngineMode::Typst);

    let rendered_native = typst_text::render_raster(
        "scale-check",
        12.0,
        Color::BLACK,
        0.0,
        "typst native scale test",
    )
    .unwrap();
    let rendered_second = typst_text::render_raster(
        "scale-check",
        12.0,
        Color::BLACK,
        0.0,
        "typst native scale test",
    )
    .unwrap();

    assert_eq!(
        rendered_native.pixmap.width(),
        rendered_second.pixmap.width()
    );
    assert_eq!(
        rendered_native.pixmap.height(),
        rendered_second.pixmap.height()
    );
    assert!(
        (rendered_native.pixmap.width() as f32 - rendered_native.width).abs() <= 1.0,
        "native raster width should align with logical width: pixel={} logical={}",
        rendered_native.pixmap.width(),
        rendered_native.width
    );
    assert!(
        (rendered_native.pixmap.height() as f32 - rendered_native.height).abs() <= 1.0,
        "native raster height should align with logical height: pixel={} logical={}",
        rendered_native.pixmap.height(),
        rendered_native.height
    );
}

#[test]
fn test_draw_legend_full_scales_font_with_render_dpi() {
    fn render_legend(dpi: f32) -> Image {
        let mut renderer = SkiaRenderer::new(360, 240, Theme::light()).unwrap();
        renderer.clear();
        renderer.set_render_scale(RenderScale::new(dpi));

        let items = vec![LegendItem {
            label: "Legend Label".to_string(),
            color: Color::BLACK,
            item_type: LegendItemType::Line {
                style: LineStyle::Solid,
                width: 1.0,
            },
            has_error_bars: false,
        }];
        let legend = Legend {
            enabled: true,
            position: LegendPosition::UpperLeft,
            text_color: Color::BLACK,
            ..Legend::default()
        };
        let plot_area = Rect::from_xywh(40.0, 40.0, 260.0, 160.0).unwrap();

        renderer
            .draw_legend_full(&items, &legend, plot_area, None)
            .unwrap();
        renderer.into_image()
    }

    let low_dpi = render_legend(72.0);
    let high_dpi = render_legend(144.0);
    let low_bounds = dark_pixel_bounds(&low_dpi).expect("low-dpi legend should draw dark pixels");
    let high_bounds =
        dark_pixel_bounds(&high_dpi).expect("high-dpi legend should draw dark pixels");
    let low_width = low_bounds.2 - low_bounds.0 + 1;
    let low_height = low_bounds.3 - low_bounds.1 + 1;
    let high_width = high_bounds.2 - high_bounds.0 + 1;
    let high_height = high_bounds.3 - high_bounds.1 + 1;

    assert!(
        high_width as f32 > low_width as f32 * 1.5,
        "legend width should scale with DPI: low={low_width}, high={high_width}"
    );
    assert!(
        high_height as f32 > low_height as f32 * 1.5,
        "legend height should scale with DPI: low={low_height}, high={high_height}"
    );
}

#[test]
fn test_renderer_dimensions() {
    let theme = Theme::default();
    let renderer = SkiaRenderer::new(800, 600, theme).unwrap();

    assert_eq!(renderer.width(), 800);
    assert_eq!(renderer.height(), 600);
}

#[test]
fn test_draw_subplot() {
    use crate::core::plot::Image;

    let theme = Theme::default();
    let mut main_renderer = SkiaRenderer::new(800, 600, theme.clone()).unwrap();
    let subplot_renderer = SkiaRenderer::new(200, 150, theme).unwrap();

    // Convert subplot to image
    let subplot_image = subplot_renderer.into_image();

    // Should draw subplot without error
    let result = main_renderer.draw_subplot(subplot_image, 10, 20);
    assert!(result.is_ok());
}

#[test]
fn test_draw_subplot_bounds_checking() {
    use crate::core::plot::Image;

    let theme = Theme::default();
    let mut main_renderer = SkiaRenderer::new(400, 300, theme.clone()).unwrap();
    let subplot_renderer = SkiaRenderer::new(200, 150, theme).unwrap();

    let subplot_image = subplot_renderer.into_image();

    // Should handle valid positions
    assert!(
        main_renderer
            .draw_subplot(subplot_image.clone(), 0, 0)
            .is_ok()
    );
    assert!(
        main_renderer
            .draw_subplot(subplot_image.clone(), 100, 50)
            .is_ok()
    );

    // Should handle edge positions
    assert!(main_renderer.draw_subplot(subplot_image, 200, 150).is_ok());
}

#[test]
fn test_to_image_conversion() {
    let theme = Theme::default();
    let renderer = SkiaRenderer::new(400, 300, theme).unwrap();

    let image = renderer.into_image();

    assert_eq!(image.width, 400);
    assert_eq!(image.height, 300);
    assert_eq!(image.pixels.len(), 400 * 300 * 4); // RGBA pixels
}
