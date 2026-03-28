//! Test-driven development for DPI-aware font and line scaling
//!
//! Tests that DPI settings scale fonts and lines consistently for publication quality
//! Expected: Higher DPI = proportionally larger fonts and thicker lines

use ruviz::core::plot::Image;
use ruviz::prelude::*;

fn total_ink(image: &Image) -> u64 {
    image
        .pixels
        .chunks_exact(4)
        .map(|pixel| {
            let alpha = pixel[3] as u64;
            let darkness = (255_u64 - pixel[0] as u64)
                + (255_u64 - pixel[1] as u64)
                + (255_u64 - pixel[2] as u64);
            darkness * alpha / 255
        })
        .sum()
}

#[test]
fn test_dpi_font_scaling_visual_consistency() -> std::result::Result<(), Box<dyn std::error::Error>>
{
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
    let image_96 = base_plot.clone().dpi(96).render()?; // 1x scale
    let image_192 = base_plot.clone().dpi(192).render()?; // 2x scale
    let image_288 = base_plot.clone().dpi(288).render()?; // 3x scale

    let ink_96 = total_ink(&image_96);
    let ink_192 = total_ink(&image_192);
    let ink_288 = total_ink(&image_288);

    println!(
        "Font scaling ink - 96 DPI: {}, 192 DPI: {}, 288 DPI: {}",
        ink_96, ink_192, ink_288
    );

    let font_ratio_2x = ink_192 as f64 / ink_96 as f64;
    let font_ratio_3x = ink_288 as f64 / ink_96 as f64;

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
    let image_96 = base_plot.clone().dpi(96).render()?; // Base line width
    let image_300 = base_plot.clone().dpi(300).render()?; // Scaled line width

    let ink_96 = total_ink(&image_96);
    let ink_300 = total_ink(&image_300);

    let line_width_ratio = ink_300 as f64 / ink_96 as f64;

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
    let ieee_image = ieee_plot.clone().dpi(600).render()?;
    let nature_image = ieee_plot.clone().dpi(300).render()?;
    let ieee_ink = total_ink(&ieee_image);
    let nature_ink = total_ink(&nature_image);
    let publication_ratio = ieee_ink as f64 / nature_ink as f64;

    assert!(
        ieee_image.width == 3000 && ieee_image.height == 2400,
        "IEEE 600 DPI should produce a 3000x2400 raster, got {}x{}",
        ieee_image.width,
        ieee_image.height
    );
    assert!(
        nature_image.width == 1500 && nature_image.height == 1200,
        "Nature 300 DPI should produce a 1500x1200 raster, got {}x{}",
        nature_image.width,
        nature_image.height
    );
    assert!(
        publication_ratio > 3.5,
        "IEEE/Nature ratio should reflect DPI difference: {:.1}x",
        publication_ratio
    );

    println!(
        "✓ Publication typography: IEEE ink {}, Nature ink {}, ratio {:.1}x",
        ieee_ink, nature_ink, publication_ratio
    );
    Ok(())
}

#[test]
fn test_font_line_ratio_consistency() -> std::result::Result<(), Box<dyn std::error::Error>> {
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
    let image_96 = ratio_plot.clone().dpi(96).render()?;
    let image_150 = ratio_plot.clone().dpi(150).render()?;
    let image_300 = ratio_plot.clone().dpi(300).render()?;

    let ink_96 = total_ink(&image_96);
    let ink_150 = total_ink(&image_150);
    let ink_300 = total_ink(&image_300);

    let ratio_150_96 = ink_150 as f64 / ink_96 as f64;
    let ratio_300_96 = ink_300 as f64 / ink_96 as f64;

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
