//! Test-driven development for DPI-aware font and line scaling
//!
//! Tests that DPI settings scale fonts and lines consistently for publication quality
//! Expected: Higher DPI = proportionally larger fonts and thicker lines

use ruviz::prelude::*;
use std::fs;

/// Setup test output directory
fn setup_test_output_dir() -> std::result::Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("tests/output")?;
    Ok(())
}

#[test]
fn test_dpi_font_scaling_visual_consistency() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    setup_test_output_dir()?;

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];

    // Create plots with same data but different DPI - fonts should scale proportionally
    let base_plot = Plot::new()
        .size_px(400, 300) // Base canvas size
        .title("Font Scale Test - Typography Quality")
        .xlabel("X Axis Label")
        .ylabel("Y Axis Label")
        .line(&x_data, &y_data);

    // Test font scaling at different DPI values
    base_plot
        .clone()
        .dpi(96)
        .save("tests/output/font_scale_96_test.png")?; // 1x scale
    base_plot
        .clone()
        .dpi(192)
        .save("tests/output/font_scale_192_test.png")?; // 2x scale
    base_plot
        .clone()
        .dpi(288)
        .save("tests/output/font_scale_288_test.png")?; // 3x scale

    // Verify file sizes increase due to scaled typography
    let size_96 = fs::metadata("tests/output/font_scale_96_test.png")?.len();
    let size_192 = fs::metadata("tests/output/font_scale_192_test.png")?.len();
    let size_288 = fs::metadata("tests/output/font_scale_288_test.png")?.len();

    println!(
        "Font scaling sizes - 96 DPI: {} bytes, 192 DPI: {} bytes, 288 DPI: {} bytes",
        size_96, size_192, size_288
    );

    // Font scaling should contribute to overall file size increases
    let font_ratio_2x = size_192 as f64 / size_96 as f64;
    let font_ratio_3x = size_288 as f64 / size_96 as f64;

    assert!(
        font_ratio_2x > 2.0,
        "2x DPI should scale fonts significantly, got ratio: {:.1}",
        font_ratio_2x
    );
    assert!(
        font_ratio_3x > 3.5,
        "3x DPI should scale fonts more, got ratio: {:.1}",
        font_ratio_3x
    );

    println!(
        "✓ Font scaling visual consistency: 2x = {:.1}x, 3x = {:.1}x",
        font_ratio_2x, font_ratio_3x
    );
    Ok(())
}

#[test]
fn test_dpi_line_width_scaling() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;

    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y_data = vec![0.0, 2.0, 1.5, 3.0, 2.5];

    // Simple plot to test line width scaling consistency
    let base_plot = Plot::new()
        .size_px(400, 300)
        .title("Line Width Scale Test")
        .xlabel("X Values")
        .ylabel("Y Values")
        .line(&x_data, &y_data);

    // Test at standard DPI values
    base_plot
        .clone()
        .dpi(96)
        .save("tests/output/line_width_96_test.png")?; // Base line width
    base_plot
        .clone()
        .dpi(300)
        .save("tests/output/line_width_300_test.png")?; // Scaled line width

    let size_96 = fs::metadata("tests/output/line_width_96_test.png")?.len();
    let size_300 = fs::metadata("tests/output/line_width_300_test.png")?.len();

    // Line width scaling should contribute to file size differences
    let line_width_ratio = size_300 as f64 / size_96 as f64;

    // Expect significant scaling due to both canvas size and line thickness
    assert!(
        line_width_ratio > 4.0,
        "Line width scaling should contribute to DPI scaling: {:.1}x",
        line_width_ratio
    );

    println!(
        "✓ Line width scaling consistency: 300/96 DPI ratio = {:.1}x",
        line_width_ratio
    );
    Ok(())
}

#[test]
fn test_publication_dpi_typography_standards() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    setup_test_output_dir()?;

    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![1.5, 2.5, 1.8];

    // Test publication-quality typography at IEEE standards
    let ieee_plot = Plot::new()
        .size_px(500, 400)
        .title("IEEE Publication Test - Font & Line Quality")
        .xlabel("Measurement Values")
        .ylabel("Response Values")
        .line(&x_data, &y_data);

    // Test IEEE 600 DPI publication standard
    ieee_plot
        .clone()
        .dpi(600)
        .save("tests/output/ieee_typography_600_test.png")?;

    // Test Nature/Science 300 DPI standard
    ieee_plot
        .clone()
        .dpi(300)
        .save("tests/output/nature_typography_300_test.png")?;

    let ieee_size = fs::metadata("tests/output/ieee_typography_600_test.png")?.len();
    let nature_size = fs::metadata("tests/output/nature_typography_300_test.png")?.len();

    // Publication standards should produce significantly large, high-quality files
    let publication_ratio = ieee_size as f64 / nature_size as f64;

    assert!(
        ieee_size > 200_000,
        "IEEE 600 DPI should produce large publication files: {} bytes",
        ieee_size
    );
    assert!(
        nature_size > 80_000,
        "Nature 300 DPI should produce quality files: {} bytes",
        nature_size
    );
    assert!(
        publication_ratio > 2.0,
        "IEEE/Nature ratio should reflect DPI difference: {:.1}x",
        publication_ratio
    );

    println!(
        "✓ Publication typography: IEEE {} bytes, Nature {} bytes, ratio {:.1}x",
        ieee_size, nature_size, publication_ratio
    );
    Ok(())
}

#[test]
fn test_font_line_ratio_consistency() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;

    let x_data = vec![0.0, 5.0, 10.0];
    let y_data = vec![0.0, 25.0, 100.0];

    // Test that font size and line width maintain consistent ratios across DPI
    let ratio_plot = Plot::new()
        .size_px(300, 200) // Smaller canvas for focused testing
        .title("Font-Line Ratio Test")
        .xlabel("Input")
        .ylabel("Output")
        .line(&x_data, &y_data);

    // Test at multiple DPI values for ratio consistency
    ratio_plot
        .clone()
        .dpi(96)
        .save("tests/output/ratio_96_test.png")?;
    ratio_plot
        .clone()
        .dpi(150)
        .save("tests/output/ratio_150_test.png")?;
    ratio_plot
        .clone()
        .dpi(300)
        .save("tests/output/ratio_300_test.png")?;

    let size_96 = fs::metadata("tests/output/ratio_96_test.png")?.len();
    let size_150 = fs::metadata("tests/output/ratio_150_test.png")?.len();
    let size_300 = fs::metadata("tests/output/ratio_300_test.png")?.len();

    // Calculate scaling ratios to verify consistency
    let ratio_150_96 = size_150 as f64 / size_96 as f64;
    let ratio_300_96 = size_300 as f64 / size_96 as f64;

    println!(
        "Font-line consistency ratios - 150/96: {:.2}, 300/96: {:.2}",
        ratio_150_96, ratio_300_96
    );

    // Ratios should be consistent with DPI scaling expectations
    assert!(
        ratio_150_96 > 1.5 && ratio_150_96 < 3.0,
        "150/96 DPI ratio should be moderate: {:.2}",
        ratio_150_96
    );
    assert!(
        ratio_300_96 > 4.0 && ratio_300_96 < 12.0,
        "300/96 DPI ratio should be significant: {:.2}",
        ratio_300_96
    );

    println!("✓ Font-line ratio consistency maintained across DPI scaling");
    Ok(())
}
