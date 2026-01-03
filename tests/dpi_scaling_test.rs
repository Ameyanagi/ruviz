//! Test-driven development for DPI-aware rendering scaling
//!
//! Tests that DPI settings actually affect canvas size and element scaling
//! Expected: Higher DPI = larger images, scaled fonts, lines, margins

use ruviz::prelude::*;
use std::fs;

/// Setup test output directory
fn setup_test_output_dir() -> std::result::Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("test_output")?;
    Ok(())
}

#[test]
fn test_dpi_scaling_produces_different_file_sizes()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];

    // Create identical plots with different DPI settings
    let base_plot = Plot::new()
        .title("DPI Scaling Test")
        .xlabel("X Axis")
        .ylabel("Y Axis")
        .line(&x_data, &y_data);

    // Test standard DPI values - these should produce DIFFERENT file sizes
    base_plot
        .clone()
        .dpi(96)
        .save("test_output/dpi_scaling_96_test.png")?; // Screen
    base_plot
        .clone()
        .dpi(150)
        .save("test_output/dpi_scaling_150_test.png")?; // Web  
    base_plot
        .clone()
        .dpi(300)
        .save("test_output/dpi_scaling_300_test.png")?; // Print
    base_plot
        .clone()
        .dpi(600)
        .save("test_output/dpi_scaling_600_test.png")?; // IEEE

    // Get file sizes
    let size_96 = fs::metadata("test_output/dpi_scaling_96_test.png")?.len();
    let size_150 = fs::metadata("test_output/dpi_scaling_150_test.png")?.len();
    let size_300 = fs::metadata("test_output/dpi_scaling_300_test.png")?.len();
    let size_600 = fs::metadata("test_output/dpi_scaling_600_test.png")?.len();

    println!(
        "File sizes - 96 DPI: {} bytes, 150 DPI: {} bytes, 300 DPI: {} bytes, 600 DPI: {} bytes",
        size_96, size_150, size_300, size_600
    );

    // Assert that higher DPI produces larger files (due to larger canvas)
    assert!(
        size_150 > size_96,
        "150 DPI should be larger than 96 DPI: {} vs {}",
        size_150,
        size_96
    );
    assert!(
        size_300 > size_150,
        "300 DPI should be larger than 150 DPI: {} vs {}",
        size_300,
        size_150
    );
    assert!(
        size_600 > size_300,
        "600 DPI should be larger than 300 DPI: {} vs {}",
        size_600,
        size_300
    );

    // Verify significant size differences (not just compression artifacts)
    let ratio_300_to_96 = size_300 as f64 / size_96 as f64;
    assert!(
        ratio_300_to_96 > 2.0,
        "300 DPI should be significantly larger than 96 DPI, ratio: {}",
        ratio_300_to_96
    );

    println!(
        "✓ DPI scaling produces correctly sized images: 300/96 ratio = {:.1}x",
        ratio_300_to_96
    );
    Ok(())
}

#[test]
fn test_dpi_canvas_size_scaling() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;

    let x_data = vec![0.0, 1.0, 2.0];
    let y_data = vec![0.0, 1.0, 0.0];

    // Test with known base dimensions
    let plot = Plot::new()
        .dimensions(400, 300) // Base size
        .title("Canvas Size Test")
        .line(&x_data, &y_data);

    // At 96 DPI: should be 400x300 pixels
    // At 192 DPI (2x): should be 800x600 pixels
    // At 288 DPI (3x): should be 1200x900 pixels

    plot.clone()
        .dpi(96)
        .save("test_output/canvas_96_test.png")?;
    plot.clone()
        .dpi(192)
        .save("test_output/canvas_192_test.png")?;
    plot.clone()
        .dpi(288)
        .save("test_output/canvas_288_test.png")?;

    let size_96 = fs::metadata("test_output/canvas_96_test.png")?.len();
    let size_192 = fs::metadata("test_output/canvas_192_test.png")?.len();
    let size_288 = fs::metadata("test_output/canvas_288_test.png")?.len();

    println!(
        "Canvas sizes - 96 DPI: {} bytes, 192 DPI: {} bytes, 288 DPI: {} bytes",
        size_96, size_192, size_288
    );

    // 2x DPI should produce significantly larger files (PNG compression affects exact ratios)
    let ratio_2x = size_192 as f64 / size_96 as f64;
    let ratio_3x = size_288 as f64 / size_96 as f64;

    assert!(
        ratio_2x > 2.0,
        "2x DPI should produce significantly larger files, got ratio: {:.1}",
        ratio_2x
    );
    assert!(
        ratio_3x > 3.0,
        "3x DPI should produce much larger files, got ratio: {:.1}",
        ratio_3x
    );

    println!(
        "✓ Canvas scaling works: 2x DPI = {:.1}x size, 3x DPI = {:.1}x size",
        ratio_2x, ratio_3x
    );
    Ok(())
}

#[test]
fn test_dpi_font_scaling_consistency() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;

    // Simple plot to test font scaling
    let x_data = vec![1.0, 2.0];
    let y_data = vec![1.0, 2.0];

    let plot = Plot::new()
        .title("Font Scale Test - Title Should Scale")
        .xlabel("X Label Should Scale")
        .ylabel("Y Label Should Scale")
        .line(&x_data, &y_data);

    // Test font consistency across DPI
    plot.clone().dpi(96).save("test_output/font_96_test.png")?;
    plot.clone()
        .dpi(300)
        .save("test_output/font_300_test.png")?;

    let size_96 = fs::metadata("test_output/font_96_test.png")?.len();
    let size_300 = fs::metadata("test_output/font_300_test.png")?.len();

    // Font scaling should contribute to file size increase
    let font_scaling_ratio = size_300 as f64 / size_96 as f64;
    assert!(
        font_scaling_ratio > 3.0,
        "Font scaling should increase file size significantly: {:.1}x",
        font_scaling_ratio
    );

    println!(
        "✓ Font scaling contributes to DPI scaling: {:.1}x size increase",
        font_scaling_ratio
    );
    Ok(())
}
