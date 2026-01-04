//! Export format tests - PNG, SVG, and raw data
//!
//! Run with: cargo test --test export_tests_fixed
//! Files will be saved to export_output/ directory

use ruviz::prelude::*;
use ruviz::render::skia::SkiaRenderer;
use std::fs;

/// Setup export output directories
fn setup_export_dirs() -> std::result::Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("export_output/png")?;
    fs::create_dir_all("export_output/svg")?;
    fs::create_dir_all("export_output/raw")?;
    Ok(())
}

#[test]
fn test_png_exports() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];

    // Test 1: Basic line plot PNG
    Plot::new()
        .title("PNG Line Plot".to_string())
        .xlabel("X Values".to_string())
        .ylabel("Y Values".to_string())
        .line(&x_data, &y_data)
        .end_series()
        .save("export_output/png/01_line_plot.png")?;

    // Test 2: Scatter plot PNG
    Plot::new()
        .title("PNG Scatter Plot".to_string())
        .scatter(&x_data, &y_data)
        .end_series()
        .save("export_output/png/02_scatter_plot.png")?;

    // Test 3: Bar plot PNG
    let categories = vec!["A", "B", "C", "D", "E"];
    Plot::new()
        .title("PNG Bar Plot".to_string())
        .bar(&categories, &y_data)
        .end_series()
        .save("export_output/png/03_bar_plot.png")?;

    // Test 4: Dark theme PNG
    Plot::with_theme(Theme::dark())
        .title("PNG Dark Theme".to_string())
        .line(&x_data, &y_data)
        .end_series()
        .save("export_output/png/04_dark_theme.png")?;

    println!("✓ PNG exports completed: 4 files saved");
    Ok(())
}

#[test]
fn test_svg_exports() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;

    // Test SVG export using SkiaRenderer directly
    let renderer1 = SkiaRenderer::new(800, 600, Theme::light())?;
    renderer1.export_svg("export_output/svg/01_light_theme.svg", 800, 600)?;

    let renderer2 = SkiaRenderer::new(800, 600, Theme::dark())?;
    renderer2.export_svg("export_output/svg/02_dark_theme.svg", 800, 600)?;

    let renderer3 = SkiaRenderer::new(1200, 800, Theme::publication())?;
    renderer3.export_svg("export_output/svg/03_publication_large.svg", 1200, 800)?;

    let renderer4 = SkiaRenderer::new(600, 600, Theme::minimal())?;
    renderer4.export_svg("export_output/svg/04_minimal_square.svg", 600, 600)?;

    println!("✓ SVG exports completed: 4 files saved");
    Ok(())
}

#[test]
fn test_raw_data_exports() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;

    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y_data = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    // Test 1: Standard size raw data
    let plot1 = Plot::new()
        .title("Raw Data Export Test".to_string())
        .line(&x_data, &y_data)
        .end_series();

    let image1 = plot1.render()?;
    fs::write("export_output/raw/01_standard_800x600.bin", &image1.pixels)?;

    let metadata1 = format!(
        "Standard Plot Raw Data\nSize: {}x{}\nBytes: {}\nFormat: RGBA",
        image1.width,
        image1.height,
        image1.pixels.len()
    );
    fs::write("export_output/raw/01_standard_info.txt", metadata1)?;

    // Test 2: Custom size raw data
    let plot2 = Plot::new()
        .dimensions(400, 300)
        .title("Small Raw Export".to_string())
        .scatter(&x_data, &y_data)
        .end_series();

    let image2 = plot2.render()?;
    fs::write("export_output/raw/02_small_400x300.bin", &image2.pixels)?;

    let metadata2 = format!(
        "Small Plot Raw Data\nSize: {}x{}\nBytes: {}\nFormat: RGBA",
        image2.width,
        image2.height,
        image2.pixels.len()
    );
    fs::write("export_output/raw/02_small_info.txt", metadata2)?;

    // Test 3: Bar chart raw data
    let categories = vec!["Q1", "Q2", "Q3", "Q4", "Q5"];
    let plot3 = Plot::new()
        .title("Bar Chart Raw Export".to_string())
        .bar(&categories, &y_data)
        .end_series();

    let image3 = plot3.render()?;
    fs::write("export_output/raw/03_bar_chart.bin", &image3.pixels)?;

    let metadata3 = format!(
        "Bar Chart Raw Data\nSize: {}x{}\nBytes: {}\nFormat: RGBA\nCategories: {}",
        image3.width,
        image3.height,
        image3.pixels.len(),
        categories.len()
    );
    fs::write("export_output/raw/03_bar_info.txt", metadata3)?;

    println!("✓ Raw data exports completed: 6 files saved");
    Ok(())
}

#[test]
fn test_all_themes_export() -> std::result::Result<(), Box<dyn std::error::Error>> {
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
        // PNG export
        Plot::with_theme(theme.clone())
            .title(format!("{} Theme Test", theme_name))
            .line(&x_data, &y_data)
            .end_series()
            .save(format!("export_output/png/theme_{}.png", theme_name))?;

        // SVG export
        let renderer = SkiaRenderer::new(800, 600, theme.clone())?;
        renderer.export_svg(
            format!("export_output/svg/theme_{}.svg", theme_name),
            800,
            600,
        )?;

        // Raw data export
        let plot_raw = Plot::with_theme(theme)
            .title(format!("{} Raw Test", theme_name))
            .scatter(&x_data, &y_data)
            .end_series();

        let image = plot_raw.render()?;
        fs::write(
            format!("export_output/raw/theme_{}.bin", theme_name),
            &image.pixels,
        )?;
    }

    println!("✓ All themes exported: 12 files saved (4 themes × 3 formats)");
    Ok(())
}

#[test]
fn test_different_resolutions() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;

    let x_data: Vec<f64> = (0..10).map(|i| i as f64).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| x.sin()).collect();

    let resolutions = vec![
        (800, 600, "standard"),
        (1920, 1080, "hd"),
        (400, 300, "small"),
        (1200, 900, "large"),
    ];

    for (width, height, name) in resolutions {
        // PNG
        Plot::new()
            .dimensions(width, height)
            .title(format!("Resolution Test - {}", name.to_uppercase()))
            .line(&x_data, &y_data)
            .end_series()
            .save(format!(
                "export_output/png/resolution_{}_{}_{}x{}.png",
                name, "png", width, height
            ))?;

        // SVG
        let renderer = SkiaRenderer::new(width, height, Theme::default())?;
        renderer.export_svg(
            format!(
                "export_output/svg/resolution_{}_{}_{}x{}.svg",
                name, "svg", width, height
            ),
            width,
            height,
        )?;

        // Raw with info
        let plot = Plot::new()
            .dimensions(width, height)
            .title(format!("Raw {} Resolution", name))
            .scatter(&x_data, &y_data)
            .end_series();

        let image = plot.render()?;
        fs::write(
            format!(
                "export_output/raw/resolution_{}_{}x{}.bin",
                name, width, height
            ),
            &image.pixels,
        )?;

        let info = format!(
            "Resolution: {}x{}\nMegapixels: {:.1}\nFile size: {} bytes\nFormat: RGBA",
            width,
            height,
            (width * height) as f64 / 1_000_000.0,
            image.pixels.len()
        );
        fs::write(
            format!(
                "export_output/raw/resolution_{}_{}x{}_info.txt",
                name, width, height
            ),
            info,
        )?;
    }

    println!("✓ Resolution tests completed: 12 files saved (4 resolutions × 3 formats)");
    Ok(())
}

/// Validation test to ensure all exports contain actual data
#[test]
fn test_export_validation() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;

    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![1.0, 4.0, 2.0];

    let plot = Plot::new()
        .title("Validation Test".to_string())
        .line(&x_data, &y_data)
        .end_series();

    // Test render produces valid image data
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

    // Check that pixels contain actual data (not all zeros or all same value)
    let non_zero_pixels = image.pixels.iter().filter(|&&pixel| pixel != 0).count();
    assert!(
        non_zero_pixels > 1000,
        "Image should contain substantial non-zero pixel data"
    );

    // Check for color variation (not monochrome)
    let unique_values: std::collections::HashSet<&u8> = image.pixels.iter().collect();
    assert!(
        unique_values.len() > 10,
        "Image should have color variation"
    );

    println!("✓ Export validation passed: Image contains valid rendering data");
    Ok(())
}
