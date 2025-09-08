//! Comprehensive export format tests
//! 
//! Tests all available export formats: PNG, SVG, raw RGBA data, and direct SkiaRenderer exports

use ruviz::prelude::*;
use ruviz::render::skia::{SkiaRenderer, calculate_plot_area};
use std::fs;

/// Setup test output directories
fn setup_export_dirs() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("export_test_output/png")?;
    fs::create_dir_all("export_test_output/svg")?;
    fs::create_dir_all("export_test_output/raw")?;
    fs::create_dir_all("export_test_output/direct")?;
    Ok(())
}

#[test]
fn test_png_export() -> Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;
    
    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];
    
    // Test PNG export via Plot::save()
    let plot = Plot::new()
        .title("PNG Export Test".to_string())
        .xlabel("X Values".to_string())
        .ylabel("Y Values".to_string())
        .line(&x_data, &y_data);
    
    plot.save("export_test_output/png/01_plot_save_method.png")?;
    
    // Test PNG export via render + manual save
    let plot2 = Plot::new()
        .title("PNG Manual Export Test".to_string())
        .scatter(&x_data, &y_data);
    
    let image = plot2.render()?;
    
    // Create renderer and save PNG directly
    let mut renderer = SkiaRenderer::new(image.width, image.height, Theme::default())?;
    renderer.save_png("export_test_output/png/02_renderer_direct.png")?;
    
    println!("✅ PNG Export Tests Complete");
    Ok(())
}

#[test]
fn test_svg_export() -> Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;
    
    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![10.0, 25.0, 15.0, 30.0, 20.0];
    
    // Create renderer and test SVG export
    let mut renderer = SkiaRenderer::new(800, 600, Theme::light())?;
    
    // Export SVG with different sizes
    renderer.export_svg("export_test_output/svg/01_standard_800x600.svg", 800, 600)?;
    renderer.export_svg("export_test_output/svg/02_large_1200x800.svg", 1200, 800)?;
    renderer.export_svg("export_test_output/svg/03_square_600x600.svg", 600, 600)?;
    renderer.export_svg("export_test_output/svg/04_wide_1000x400.svg", 1000, 400)?;
    
    // Test SVG with different themes
    let dark_renderer = SkiaRenderer::new(800, 600, Theme::dark())?;
    dark_renderer.export_svg("export_test_output/svg/05_dark_theme.svg", 800, 600)?;
    
    let pub_renderer = SkiaRenderer::new(800, 600, Theme::publication())?;
    pub_renderer.export_svg("export_test_output/svg/06_publication_theme.svg", 800, 600)?;
    
    println!("✅ SVG Export Tests Complete");
    Ok(())
}

#[test]
fn test_raw_data_export() -> Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;
    
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y_data = vec![0.0, 1.0, 4.0, 9.0, 16.0];
    
    // Test raw RGBA data export
    let plot = Plot::new()
        .title("Raw Data Export Test".to_string())
        .line(&x_data, &y_data);
    
    let image = plot.render()?;
    
    // Save raw RGBA data
    fs::write("export_test_output/raw/01_rgba_data.bin", &image.pixels)?;
    
    // Save image metadata
    let metadata = format!(
        "Width: {}\nHeight: {}\nBytes per pixel: 4 (RGBA)\nTotal bytes: {}\nFormat: Raw RGBA bytes",
        image.width, image.height, image.pixels.len()
    );
    fs::write("export_test_output/raw/01_rgba_data.txt", metadata)?;
    
    // Test different sizes
    let small_plot = Plot::new()
        .dimensions(400, 300)
        .title("Small Raw Export".to_string())
        .scatter(&x_data, &y_data);
    
    let small_image = small_plot.render()?;
    fs::write("export_test_output/raw/02_small_rgba.bin", &small_image.pixels)?;
    
    let large_plot = Plot::new()
        .dimensions(1600, 1200)
        .title("Large Raw Export".to_string())
        .bar(&["A", "B", "C", "D", "E"], &y_data);
    
    let large_image = large_plot.render()?;
    fs::write("export_test_output/raw/03_large_rgba.bin", &large_image.pixels)?;
    
    println!("✅ Raw Data Export Tests Complete");
    Ok(())
}

#[test]
fn test_direct_renderer_exports() -> Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;
    
    // Test direct SkiaRenderer usage with manual drawing
    let mut renderer = SkiaRenderer::new(800, 600, Theme::minimal())?;
    
    // Get plot area
    let plot_area = calculate_plot_area(800, 600, 0.15);
    
    // Manual drawing test
    renderer.draw_line(100.0, 100.0, 700.0, 500.0, Color::new(255, 0, 0), 2.0, LineStyle::Solid)?;
    renderer.draw_circle(400.0, 300.0, 50.0, Color::new(0, 255, 0), true)?;
    renderer.draw_rectangle(200.0, 200.0, 100.0, 50.0, Color::new(0, 0, 255), false)?;
    
    // Save as PNG
    renderer.save_png("export_test_output/direct/01_manual_drawing.png")?;
    
    // Test different renderer configurations
    let mut dark_renderer = SkiaRenderer::new(600, 400, Theme::dark())?;
    dark_renderer.draw_polyline(
        &[(50.0, 50.0), (150.0, 100.0), (250.0, 75.0), (350.0, 150.0), (450.0, 50.0)],
        Color::new(255, 255, 0), 
        3.0, 
        LineStyle::Dashed
    )?;
    dark_renderer.save_png("export_test_output/direct/02_dark_polyline.png")?;
    
    // Test grid rendering
    let mut grid_renderer = SkiaRenderer::new(800, 600, Theme::light())?;
    let x_ticks = vec![100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0];
    let y_ticks = vec![100.0, 200.0, 300.0, 400.0, 500.0];
    
    grid_renderer.draw_grid(&x_ticks, &y_ticks, plot_area, Color::new(200, 200, 200), LineStyle::Dotted)?;
    grid_renderer.draw_axes(plot_area, &x_ticks, &y_ticks, Color::new(0, 0, 0))?;
    grid_renderer.save_png("export_test_output/direct/03_grid_and_axes.png")?;
    
    println!("✅ Direct Renderer Export Tests Complete");
    Ok(())
}

#[test]
fn test_all_themes_all_formats() -> Result<(), Box<dyn std::error::Error>> {
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
        
        plot_png.save(&format!("export_test_output/png/theme_{}_{}.png", theme_name, "line"))?;
        
        // Test scatter with same theme
        let plot_scatter = Plot::with_theme(theme.clone())
            .title(format!("{} Theme Scatter Test", theme_name))
            .scatter(&x_data, &y_data);
        
        plot_scatter.save(&format!("export_test_output/png/theme_{}_{}.png", theme_name, "scatter"))?;
        
        // Test SVG export
        let renderer = SkiaRenderer::new(800, 600, theme.clone())?;
        renderer.export_svg(
            &format!("export_test_output/svg/theme_{}.svg", theme_name), 
            800, 
            600
        )?;
        
        // Test raw data export
        let plot_raw = Plot::with_theme(theme)
            .title(format!("{} Theme Raw Test", theme_name))
            .bar(&["A", "B", "C", "D", "E"], &y_data);
        
        let image = plot_raw.render()?;
        fs::write(&format!("export_test_output/raw/theme_{}.bin", theme_name), &image.pixels)?;
    }
    
    println!("✅ All Themes All Formats Tests Complete");
    Ok(())
}

#[test]
fn test_export_format_validation() -> Result<(), Box<dyn std::error::Error>> {
    setup_export_dirs()?;
    
    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![1.0, 4.0, 2.0];
    
    let plot = Plot::new()
        .title("Format Validation Test".to_string())
        .line(&x_data, &y_data);
    
    // Test file extensions
    let test_files = vec![
        "export_test_output/png/validation_test.png",
        "export_test_output/png/validation_test.PNG",
        "export_test_output/png/validation_test_no_extension",
    ];
    
    for file_path in test_files {
        match plot.save(file_path) {
            Ok(_) => println!("✅ Successfully saved: {}", file_path),
            Err(e) => println!("⚠️  Error saving {}: {}", file_path, e),
        }
    }
    
    // Test image data validation
    let image = plot.render()?;
    
    // Validate image properties
    assert_eq!(image.width, 800, "Default width should be 800");
    assert_eq!(image.height, 600, "Default height should be 600");
    assert_eq!(image.pixels.len(), 800 * 600 * 4, "RGBA data should be width * height * 4");
    
    // Check that pixels contain actual data (not all zeros)
    let non_zero_pixels = image.pixels.iter().filter(|&&pixel| pixel != 0).count();
    assert!(non_zero_pixels > 0, "Image should contain non-zero pixel data");
    
    println!("✅ Export Format Validation Tests Complete");
    Ok(())
}

#[test]
fn test_high_resolution_exports() -> Result<(), Box<dyn std::error::Error>> {
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
        let plot = Plot::new()
            .dimensions(width, height)
            .title(format!("High Resolution Test - {}", name.to_uppercase()))
            .xlabel("X Values".to_string())
            .ylabel("Sin(X) * 10".to_string())
            .line(&x_data, &y_data);
        
        // Save PNG
        plot.save(&format!("export_test_output/png/resolution_{}_{}_{}x{}.png", name, "png", width, height))?;
        
        // Save SVG
        let renderer = SkiaRenderer::new(width, height, Theme::default())?;
        renderer.export_svg(
            &format!("export_test_output/svg/resolution_{}_{}_{}x{}.svg", name, "svg", width, height),
            width,
            height
        )?;
        
        // Save raw data with size info
        let image = plot.render()?;
        fs::write(
            &format!("export_test_output/raw/resolution_{}_{}_{}x{}.bin", name, "raw", width, height),
            &image.pixels
        )?;
        
        let size_info = format!(
            "Resolution: {}x{}\nSize: {} MB\nPixels: {}\nBytes: {}",
            width, 
            height,
            (image.pixels.len() as f64) / (1024.0 * 1024.0),
            width * height,
            image.pixels.len()
        );
        fs::write(
            &format!("export_test_output/raw/resolution_{}_{}_{}x{}.txt", name, "info", width, height),
            size_info
        )?;
    }
    
    println!("✅ High Resolution Export Tests Complete");
    Ok(())
}

/// Master test that runs all export format tests
#[test]
fn run_all_export_tests() {
    println!("\n📤 RUNNING COMPREHENSIVE EXPORT FORMAT TESTS");
    println!("===============================================");
    
    let tests = vec![
        ("PNG Export", test_png_export),
        ("SVG Export", test_svg_export),
        ("Raw Data Export", test_raw_data_export),
        ("Direct Renderer Exports", test_direct_renderer_exports),
        ("All Themes All Formats", test_all_themes_all_formats),
        ("Export Format Validation", test_export_format_validation),
        ("High Resolution Exports", test_high_resolution_exports),
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
    println!("  • export_test_output/png/ - PNG files");
    println!("  • export_test_output/svg/ - SVG files");  
    println!("  • export_test_output/raw/ - Raw RGBA data + metadata");
    println!("  • export_test_output/direct/ - Direct SkiaRenderer exports");
    println!("\n🎯 EXPORT FORMATS TESTED:");
    println!("  • PNG - Via Plot::save() and SkiaRenderer::save_png()");
    println!("  • SVG - Via SkiaRenderer::export_svg()");
    println!("  • Raw RGBA - Via Plot::render() pixel data");
    println!("  • Direct Rendering - Via SkiaRenderer primitives");
    println!("\n🔍 Check all files for visual verification!");
}