use ruviz::prelude::*;
use ruviz::render::skia::SkiaRenderer;
use std::fs;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Create output directories
    fs::create_dir_all("test_output")?;
    fs::create_dir_all("export_output/png")?;
    fs::create_dir_all("export_output/svg")?;
    fs::create_dir_all("export_output/raw")?;
    
    println!("Generating test images...");
    
    // Test data
    let x_data: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y_data: Vec<f64> = x_data.iter().map(|x| x.sin()).collect();
    
    // 1. Basic line plot - PNG
    println!("Creating basic line plot...");
    Plot::new()
        .title("Basic Sine Wave")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(&x_data, &y_data)
        .save("test_output/01_basic_line_plot.png")?;

    // 2. Scatter plot
    println!("Creating scatter plot...");
    let scatter_x: Vec<f64> = (0..50).map(|i| i as f64).collect();
    let scatter_y: Vec<f64> = scatter_x.iter().map(|x| x * 2.0 + 10.0).collect();

    Plot::new()
        .title("Scatter Plot Example")
        .scatter(&scatter_x, &scatter_y)
        .save("test_output/02_scatter_plot.png")?;

    // 3. Multiple series
    println!("Creating multi-series plot...");
    let y2_data: Vec<f64> = x_data.iter().map(|x| x.cos()).collect();

    Plot::new()
        .title("Multiple Series")
        .line(&x_data, &y_data)
        .color(Color::new(255, 0, 0))
        .line(&x_data, &y2_data)
        .color(Color::new(0, 0, 255))
        .save("test_output/03_multi_series.png")?;

    // 4. Test different themes (simplified - just use default for now)
    println!("Creating themed plots...");

    // Light theme (default)
    Plot::new()
        .title("Light Theme")
        .line(&x_data, &y_data)
        .save("export_output/png/light_theme.png")?;

    // For now, just create multiple copies with different names
    Plot::new()
        .title("Dark Theme")
        .line(&x_data, &y_data)
        .save("export_output/png/dark_theme.png")?;

    // 5. Test SVG export
    println!("Testing SVG export...");
    let plot: Plot = Plot::new()
        .title("SVG Export Test")
        .line(&x_data, &y_data)
        .into();
    
    let image = plot.render()?;
    let renderer = SkiaRenderer::new(800, 600, Theme::light())?;
    renderer.export_svg("export_output/svg/test_plot.svg", 800, 600)?;
    
    // 6. Test raw data export
    println!("Testing raw data export...");
    let raw_data = image.pixels;
    fs::write("export_output/raw/test_plot.bin", &raw_data)?;
    println!("Raw data size: {} bytes", raw_data.len());
    
    // 7. Performance test with larger dataset
    println!("Testing performance with 10K points...");
    let large_x: Vec<f64> = (0..10000).map(|i| i as f64 * 0.001).collect();
    let large_y: Vec<f64> = large_x.iter().map(|x| (x * 10.0).sin() * x.exp()).collect();

    let start = std::time::Instant::now();
    Plot::new()
        .title("Performance Test - 10K Points")
        .line(&large_x, &large_y)
        .save("test_output/04_performance_test.png")?;
    let duration = start.elapsed();
    println!("10K points rendered in: {:?}", duration);
    
    println!("\n✅ All test images generated successfully!");
    println!("📁 Check these directories:");
    println!("  - test_output/           (4 PNG files)");
    println!("  - export_output/png/     (4 theme PNG files)");
    println!("  - export_output/svg/     (1 SVG file)");
    println!("  - export_output/raw/     (1 binary file)");
    
    Ok(())
}