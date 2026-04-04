//! Comprehensive export format tests
//!
//! Tests all available export formats: PNG, SVG, raw RGBA data, and direct SkiaRenderer exports

mod common;

use common::{
    assert_file_non_empty, assert_png_dimensions_with_tolerance, assert_png_rendered,
    test_output_path,
};
use ruviz::core::plot::{TickDirection, TickSides};
use ruviz::prelude::*;
use ruviz::render::skia::{SkiaRenderer, calculate_plot_area};
use std::fs;

/// Setup test output directories
fn setup_export_dirs() -> std::result::Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("generated/tests/export/png")?;
    fs::create_dir_all("generated/tests/export/svg")?;
    fs::create_dir_all("generated/tests/export/raw")?;
    fs::create_dir_all("generated/tests/export/direct")?;
    Ok(())
}

#[test]
fn test_png_export() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];

    // Test PNG export via Plot::save()
    let plot = Plot::new()
        .title("PNG Export Test".to_string())
        .xlabel("X Values".to_string())
        .ylabel("Y Values".to_string())
        .line(&x_data, &y_data);

    plot.save("generated/tests/export/png/01_plot_save_method.png")?;
    assert_png_rendered(
        "generated/tests/export/png/01_plot_save_method.png",
        Some((640, 480)),
    );

    // Test PNG export via render + manual save
    let plot2 = Plot::new()
        .title("PNG Manual Export Test".to_string())
        .scatter(&x_data, &y_data);

    let image = plot2.render()?;
    let (image_width, image_height) = (image.width, image.height);

    // Draw rendered plot image into a renderer and save PNG
    let mut renderer = SkiaRenderer::new(image_width, image_height, Theme::default())?;
    renderer.draw_subplot(image, 0, 0)?;
    renderer.save_png("generated/tests/export/png/02_renderer_direct.png")?;
    assert_png_rendered(
        "generated/tests/export/png/02_renderer_direct.png",
        Some((image_width, image_height)),
    );

    println!("✅ PNG Export Tests Complete");
    Ok(())
}

#[test]
fn test_svg_export() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;

    // Create renderer and test SVG export
    let renderer = SkiaRenderer::new(800, 600, Theme::light())?;

    // Export SVG with different sizes
    renderer.export_svg(
        "generated/tests/export/svg/01_standard_800x600.svg",
        800,
        600,
    )?;
    assert_file_non_empty("generated/tests/export/svg/01_standard_800x600.svg");
    renderer.export_svg(
        "generated/tests/export/svg/02_large_1200x800.svg",
        1200,
        800,
    )?;
    assert_file_non_empty("generated/tests/export/svg/02_large_1200x800.svg");
    renderer.export_svg("generated/tests/export/svg/03_square_600x600.svg", 600, 600)?;
    assert_file_non_empty("generated/tests/export/svg/03_square_600x600.svg");
    renderer.export_svg("generated/tests/export/svg/04_wide_1000x400.svg", 1000, 400)?;
    assert_file_non_empty("generated/tests/export/svg/04_wide_1000x400.svg");

    // Test SVG with different themes
    let dark_renderer = SkiaRenderer::new(800, 600, Theme::dark())?;
    dark_renderer.export_svg("generated/tests/export/svg/05_dark_theme.svg", 800, 600)?;
    assert_file_non_empty("generated/tests/export/svg/05_dark_theme.svg");

    let pub_renderer = SkiaRenderer::new(800, 600, Theme::publication())?;
    pub_renderer.export_svg(
        "generated/tests/export/svg/06_publication_theme.svg",
        800,
        600,
    )?;
    assert_file_non_empty("generated/tests/export/svg/06_publication_theme.svg");

    println!("✅ SVG Export Tests Complete");
    Ok(())
}

#[test]
fn test_raw_data_export() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;

    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y_data = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    // Test raw RGBA data export
    let plot = Plot::new()
        .title("Raw Data Export Test".to_string())
        .line(&x_data, &y_data);

    let image = plot.render()?;

    // Save raw RGBA data
    fs::write("generated/tests/export/raw/01_rgba_data.bin", &image.pixels)?;
    assert_file_non_empty("generated/tests/export/raw/01_rgba_data.bin");

    // Save image metadata
    let metadata = format!(
        "Width: {}\nHeight: {}\nBytes per pixel: 4 (RGBA)\nTotal bytes: {}\nFormat: Raw RGBA bytes",
        image.width,
        image.height,
        image.pixels.len()
    );
    fs::write("generated/tests/export/raw/01_rgba_data.txt", metadata)?;
    assert_file_non_empty("generated/tests/export/raw/01_rgba_data.txt");

    // Test different sizes
    let small_plot = Plot::new()
        .size_px(400, 300)
        .title("Small Raw Export".to_string())
        .scatter(&x_data, &y_data);

    let small_image = small_plot.render()?;
    fs::write(
        "generated/tests/export/raw/02_small_rgba.bin",
        &small_image.pixels,
    )?;
    assert_file_non_empty("generated/tests/export/raw/02_small_rgba.bin");

    let large_plot = Plot::new()
        .size_px(1600, 1200)
        .title("Large Raw Export".to_string())
        .bar(&["A", "B", "C", "D", "E"], &y_data);

    let large_image = large_plot.render()?;
    fs::write(
        "generated/tests/export/raw/03_large_rgba.bin",
        &large_image.pixels,
    )?;
    assert_file_non_empty("generated/tests/export/raw/03_large_rgba.bin");

    println!("✅ Raw Data Export Tests Complete");
    Ok(())
}

#[test]
fn test_direct_renderer_exports() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;

    // Test direct SkiaRenderer usage with manual drawing
    let mut renderer = SkiaRenderer::new(800, 600, Theme::minimal())?;

    // Get plot area
    let plot_area = calculate_plot_area(800, 600, 0.15);

    // Manual drawing test
    renderer.draw_line(
        100.0,
        100.0,
        700.0,
        500.0,
        Color::new(255, 0, 0),
        2.0,
        LineStyle::Solid,
    )?;
    renderer.draw_circle(400.0, 300.0, 50.0, Color::new(0, 255, 0), true)?;
    renderer.draw_rectangle(200.0, 200.0, 100.0, 50.0, Color::new(0, 0, 255), false)?;

    // Save as PNG
    renderer.save_png("generated/tests/export/direct/01_manual_drawing.png")?;
    assert_png_rendered(
        "generated/tests/export/direct/01_manual_drawing.png",
        Some((800, 600)),
    );

    // Test different renderer configurations
    let mut dark_renderer = SkiaRenderer::new(600, 400, Theme::dark())?;
    dark_renderer.draw_polyline(
        &[
            (50.0, 50.0),
            (150.0, 100.0),
            (250.0, 75.0),
            (350.0, 150.0),
            (450.0, 50.0),
        ],
        Color::new(255, 255, 0),
        3.0,
        LineStyle::Dashed,
    )?;
    dark_renderer.save_png("generated/tests/export/direct/02_dark_polyline.png")?;
    assert_png_rendered(
        "generated/tests/export/direct/02_dark_polyline.png",
        Some((600, 400)),
    );

    // Test grid rendering
    let mut grid_renderer = SkiaRenderer::new(800, 600, Theme::light())?;
    let x_ticks = vec![100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0];
    let y_ticks = vec![100.0, 200.0, 300.0, 400.0, 500.0];

    grid_renderer.draw_grid(
        &x_ticks,
        &y_ticks,
        plot_area,
        Color::new(200, 200, 200),
        LineStyle::Dotted,
        1.0,
    )?;
    grid_renderer.draw_axes(
        plot_area,
        &x_ticks,
        &y_ticks,
        &TickDirection::Inside,
        &TickSides::all(),
        Color::new(0, 0, 0),
    )?;
    grid_renderer.save_png("generated/tests/export/direct/03_grid_and_axes.png")?;
    assert_png_rendered(
        "generated/tests/export/direct/03_grid_and_axes.png",
        Some((800, 600)),
    );

    println!("✅ Direct Renderer Export Tests Complete");
    Ok(())
}

#[test]
fn test_save_with_size_honors_requested_pixels_after_dpi_change() {
    let output = test_output_path("save_with_size_exact_pixels.png");

    Plot::new()
        .dpi(300)
        .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
        .save_with_size(&output, 800, 600)
        .expect("save_with_size should succeed");

    assert_png_rendered(&output, Some((800, 600)));
}

#[test]
fn test_builder_save_with_size_honors_requested_pixels_after_dpi_change() {
    let output = test_output_path("builder_save_with_size_exact_pixels.png");

    Plot::new()
        .dpi(300)
        .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
        .title("Builder sized export")
        .save_with_size(&output, 640, 360)
        .expect("builder save_with_size should succeed");

    assert_png_rendered(&output, Some((640, 360)));
}

#[test]
fn test_export_overwrites_existing_png() {
    let output = test_output_path("png_overwrite_test.png");

    Plot::new()
        .line(&[0.0, 1.0], &[0.0, 1.0])
        .save(&output)
        .expect("first save should succeed");

    Plot::new()
        .dpi(200)
        .line(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0])
        .save(&output)
        .expect("second save should overwrite the same file");

    assert_png_dimensions_with_tolerance(&output, (1280, 960), 0);
}

#[test]
fn test_all_themes_all_formats() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 6.0, 4.0, 8.0];

    let themes = vec![
        ("light", Theme::light()),
        ("dark", Theme::dark()),
        ("publication", Theme::publication()),
        ("minimal", Theme::minimal()),
    ];

    for (theme_name, theme) in themes {
        // Test PNG export
        let plot_png = Plot::with_theme(theme.clone())
            .title(format!("{} Theme PNG Test", theme_name))
            .line(&x_data, &y_data);

        let line_png = format!(
            "generated/tests/export/png/theme_{}_{}.png",
            theme_name, "line"
        );
        plot_png.save(&line_png)?;
        assert_png_rendered(&line_png, Some((640, 480)));

        // Test scatter with same theme
        let plot_scatter = Plot::with_theme(theme.clone())
            .title(format!("{} Theme Scatter Test", theme_name))
            .scatter(&x_data, &y_data);

        let scatter_png = format!(
            "generated/tests/export/png/theme_{}_{}.png",
            theme_name, "scatter"
        );
        plot_scatter.save(&scatter_png)?;
        assert_png_rendered(&scatter_png, Some((640, 480)));

        // Test SVG export
        let renderer = SkiaRenderer::new(800, 600, theme.clone())?;
        let svg_path = format!("generated/tests/export/svg/theme_{}.svg", theme_name);
        renderer.export_svg(&svg_path, 800, 600)?;
        assert_file_non_empty(&svg_path);

        // Test raw data export
        let plot_raw = Plot::with_theme(theme)
            .title(format!("{} Theme Raw Test", theme_name))
            .bar(&["A", "B", "C", "D", "E"], &y_data);

        let image = plot_raw.render()?;
        let raw_path = format!("generated/tests/export/raw/theme_{}.bin", theme_name);
        fs::write(&raw_path, &image.pixels)?;
        assert_file_non_empty(&raw_path);
    }

    println!("✅ All Themes All Formats Tests Complete");
    Ok(())
}

#[test]
fn test_export_format_validation() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;

    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![1.0, 4.0, 2.0];

    // Test file extensions
    let test_files = vec![
        "generated/tests/export/png/validation_test.png",
        "generated/tests/export/png/validation_test.PNG",
        "generated/tests/export/png/validation_test_no_extension",
    ];

    for file_path in test_files {
        let plot = Plot::new()
            .title("Format Validation Test".to_string())
            .line(&x_data, &y_data);
        match plot.save(file_path) {
            Ok(_) => println!("✅ Successfully saved: {}", file_path),
            Err(e) => println!("⚠️  Error saving {}: {}", file_path, e),
        }
    }

    // Test image data validation - create fresh plot
    let plot = Plot::new()
        .title("Format Validation Test".to_string())
        .line(&x_data, &y_data);
    let image = plot.render()?;

    // Validate image properties
    // Default figure size: 6.4×4.8 inches at 100 DPI = 640×480 pixels
    assert_eq!(image.width, 640, "Default width should be 640");
    assert_eq!(image.height, 480, "Default height should be 480");
    assert_eq!(
        image.pixels.len(),
        640 * 480 * 4,
        "RGBA data should be width * height * 4"
    );

    // Check that pixels contain actual data (not all zeros)
    let non_zero_pixels = image.pixels.iter().filter(|&&pixel| pixel != 0).count();
    assert!(
        non_zero_pixels > 0,
        "Image should contain non-zero pixel data"
    );

    println!("✅ Export Format Validation Tests Complete");
    Ok(())
}

#[test]
fn test_high_resolution_exports() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;

    let x_data: Vec<f64> = (0..20).map(|i| i as f64 * 0.5).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| x.sin() * 10.0).collect();

    // Test different resolutions
    let resolutions = vec![
        (800, 600, "standard"),
        (1920, 1080, "hd"),
        (2560, 1440, "2k"),
        (3840, 2160, "4k"),
    ];

    for (width, height, name) in resolutions {
        // Create plot for PNG save
        let plot = Plot::new()
            .size_px(width, height)
            .title(format!("High Resolution Test - {}", name.to_uppercase()))
            .xlabel("X Values".to_string())
            .ylabel("Sin(X) * 10".to_string())
            .line(&x_data, &y_data);

        // Save PNG
        let png_path = format!(
            "generated/tests/export/png/resolution_{}_{}_{}x{}.png",
            name, "png", width, height
        );
        plot.save(&png_path)?;
        assert_png_dimensions_with_tolerance(&png_path, (width, height), 1);

        // Save SVG
        let renderer = SkiaRenderer::new(width, height, Theme::default())?;
        let svg_path = format!(
            "generated/tests/export/svg/resolution_{}_{}_{}x{}.svg",
            name, "svg", width, height
        );
        renderer.export_svg(&svg_path, width, height)?;
        assert_file_non_empty(&svg_path);

        // Create fresh plot for render
        let plot = Plot::new()
            .size_px(width, height)
            .title(format!("High Resolution Test - {}", name.to_uppercase()))
            .xlabel("X Values".to_string())
            .ylabel("Sin(X) * 10".to_string())
            .line(&x_data, &y_data);
        let image = plot.render()?;
        let raw_path = format!(
            "generated/tests/export/raw/resolution_{}_{}_{}x{}.bin",
            name, "raw", width, height
        );
        fs::write(&raw_path, &image.pixels)?;
        assert_file_non_empty(&raw_path);

        let size_info = format!(
            "Resolution: {}x{}\nSize: {} MB\nPixels: {}\nBytes: {}",
            width,
            height,
            (image.pixels.len() as f64) / (1024.0 * 1024.0),
            width * height,
            image.pixels.len()
        );
        let info_path = format!(
            "generated/tests/export/raw/resolution_{}_{}_{}x{}.txt",
            name, "info", width, height
        );
        fs::write(&info_path, size_info)?;
        assert_file_non_empty(&info_path);
    }

    println!("✅ High Resolution Export Tests Complete");
    Ok(())
}

/// Master test that runs all export format tests
#[test]
fn run_all_export_tests() {
    println!("\n📤 RUNNING COMPREHENSIVE EXPORT FORMAT TESTS");
    println!("===============================================");

    type ExportTestFn = fn() -> std::result::Result<(), Box<dyn std::error::Error>>;

    let tests: Vec<(&str, ExportTestFn)> = vec![
        (
            "PNG Export",
            test_png_export as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "SVG Export",
            test_svg_export as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Raw Data Export",
            test_raw_data_export as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Direct Renderer Exports",
            test_direct_renderer_exports
                as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "All Themes All Formats",
            test_all_themes_all_formats
                as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Export Format Validation",
            test_export_format_validation
                as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "High Resolution Exports",
            test_high_resolution_exports
                as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (name, test_fn) in tests {
        match test_fn() {
            Ok(_) => {
                println!("✅ {}", name);
                passed += 1;
            }
            Err(e) => {
                println!("❌ {} - Error: {}", name, e);
                failed += 1;
            }
        }
    }

    println!("\n📊 EXPORT FORMAT TEST SUMMARY");
    println!("==============================");
    println!("✅ Passed: {}", passed);
    println!("❌ Failed: {}", failed);
    println!("\n📂 OUTPUT DIRECTORIES:");
    println!("  • generated/tests/export/png/ - PNG files");
    println!("  • generated/tests/export/svg/ - SVG files");
    println!("  • generated/tests/export/raw/ - Raw RGBA data + metadata");
    println!("  • generated/tests/export/direct/ - Direct SkiaRenderer exports");
    println!("\n🎯 EXPORT FORMATS TESTED:");
    println!("  • PNG - Via Plot::save() and SkiaRenderer::save_png()");
    println!("  • SVG - Via SkiaRenderer::export_svg()");
    println!("  • Raw RGBA - Via Plot::render() pixel data");
    println!("  • Direct Rendering - Via SkiaRenderer primitives");
    println!("\n🔍 Check all files for visual verification!");
}
