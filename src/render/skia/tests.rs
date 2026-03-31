use super::*;

fn pixel_is_dark(image: &Image, x: u32, y: u32) -> bool {
    let idx = ((y * image.width + x) * 4) as usize;
    image.pixels[idx..idx + 3]
        .iter()
        .all(|channel| *channel < 220)
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
