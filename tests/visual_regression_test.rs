// Visual regression tests using perceptual diff against golden images
// These tests ensure that rendering output remains pixel-perfect across changes
// Run with: cargo test --test visual_regression_test -- --ignored
// Note: Requires golden images in tests/golden_images/ directory

use image::{GenericImageView, Pixel};
use std::path::Path;

/// Calculate perceptual difference between two PNG images
/// Returns percentage difference (0.0 = identical, 100.0 = completely different)
fn calculate_image_diff(path1: &Path, path2: &Path) -> Result<f64, Box<dyn std::error::Error>> {
    let img1 = image::open(path1)?;
    let img2 = image::open(path2)?;

    // Check dimensions match
    if img1.dimensions() != img2.dimensions() {
        return Err(format!(
            "Image dimensions mismatch: {:?} vs {:?}",
            img1.dimensions(),
            img2.dimensions()
        )
        .into());
    }

    let (width, height) = img1.dimensions();
    let total_pixels = (width * height) as f64;
    let mut diff_pixels = 0u32;
    let mut total_diff = 0.0;

    // Compare pixel by pixel
    for y in 0..height {
        for x in 0..width {
            let pixel1 = img1.get_pixel(x, y);
            let pixel2 = img2.get_pixel(x, y);

            let channels1 = pixel1.channels();
            let channels2 = pixel2.channels();

            // Calculate per-channel difference
            let mut pixel_diff = 0.0;
            for i in 0..channels1.len().min(channels2.len()) {
                let diff = (channels1[i] as f64 - channels2[i] as f64).abs();
                pixel_diff += diff;
            }

            if pixel_diff > 0.0 {
                diff_pixels += 1;
                total_diff += pixel_diff / (channels1.len() as f64 * 255.0);
            }
        }
    }

    // Return percentage difference
    Ok((total_diff / total_pixels) * 100.0)
}

/// Generate test output and compare against golden image
/// Returns true if images match within tolerance
fn test_against_golden(
    generate_fn: impl FnOnce() -> std::result::Result<(), Box<dyn std::error::Error>>,
    test_path: &str,
    golden_path: &str,
    tolerance: f64,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Generate test image
    generate_fn()?;

    // Compare against golden
    let diff = calculate_image_diff(Path::new(test_path), Path::new(golden_path))?;

    println!("  Difference: {:.4}% (tolerance: {:.4}%)", diff, tolerance);

    if diff > tolerance {
        return Err(format!(
            "Visual regression detected! Difference {:.4}% exceeds tolerance {:.4}%",
            diff, tolerance
        )
        .into());
    }

    Ok(())
}

#[test]
#[ignore] // Requires golden images
fn test_visual_regression_basic_line() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use ruviz::prelude::*;

    println!("Testing: Basic Line Plot");
    test_against_golden(
        || {
            let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
            let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];
            Plot::new()
                .line(&x, &y)
                .title("Basic Line Plot")
                .xlabel("X")
                .ylabel("Y")
                .save("tests/output/vr_basic_line.png")?;
            Ok(())
        },
        "tests/output/vr_basic_line.png",
        "tests/golden_images/01_basic_line.png",
        0.5, // 0.5% tolerance for minor rendering differences
    )?;

    Ok(())
}

#[test]
#[ignore] // Requires golden images
fn test_visual_regression_multi_series() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use ruviz::prelude::*;

    println!("Testing: Multi-Series Plot");
    test_against_golden(
        || {
            let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
            Plot::new()
                .line(&x, &x.iter().map(|&v| v).collect::<Vec<_>>())
                .label("Linear")
                .line(&x, &x.iter().map(|&v| v * v).collect::<Vec<_>>())
                .label("Quadratic")
                .line(&x, &x.iter().map(|&v| v.powi(3)).collect::<Vec<_>>())
                .label("Cubic")
                .title("Multi-Series Plot")
                .xlabel("X")
                .ylabel("Y")
                .legend(Position::TopLeft)
                .save("tests/output/vr_multi_series.png")?;
            Ok(())
        },
        "tests/output/vr_multi_series.png",
        "tests/golden_images/02_multi_series.png",
        0.5,
    )?;

    Ok(())
}

#[test]
#[ignore] // Requires golden images
fn test_visual_regression_scatter() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use ruviz::prelude::*;

    println!("Testing: Scatter Plot");
    test_against_golden(
        || {
            let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
            let y = vec![2.3, 3.1, 2.8, 4.2, 3.9];
            Plot::new()
                .scatter(&x, &y)
                .marker(MarkerStyle::Circle)
                .marker_size(8.0)
                .title("Scatter Plot")
                .xlabel("X")
                .ylabel("Y")
                .save("tests/output/vr_scatter.png")?;
            Ok(())
        },
        "tests/output/vr_scatter.png",
        "tests/golden_images/03_scatter.png",
        0.5,
    )?;

    Ok(())
}

#[test]
#[ignore] // Requires golden images
fn test_visual_regression_bar_chart() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use ruviz::prelude::*;

    println!("Testing: Bar Chart");
    test_against_golden(
        || {
            let categories = vec!["A", "B", "C", "D", "E"];
            let values = vec![25.0, 40.0, 30.0, 55.0, 45.0];
            Plot::new()
                .bar(&categories, &values)
                .title("Bar Chart")
                .ylabel("Value")
                .save("tests/output/vr_bar_chart.png")?;
            Ok(())
        },
        "tests/output/vr_bar_chart.png",
        "tests/golden_images/04_bar_chart.png",
        0.5,
    )?;

    Ok(())
}

#[test]
#[ignore] // Requires golden images
fn test_visual_regression_histogram() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use ruviz::prelude::*;

    println!("Testing: Histogram");
    test_against_golden(
        || {
            let data = vec![
                1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 5.0, 1.5, 2.5, 2.5, 3.5, 3.5, 3.5, 4.5,
                4.5, 5.5,
            ];
            Plot::new()
                .histogram(&data, None)
                .title("Histogram")
                .xlabel("Value")
                .ylabel("Frequency")
                .save("tests/output/vr_histogram.png")?;
            Ok(())
        },
        "tests/output/vr_histogram.png",
        "tests/golden_images/05_histogram.png",
        0.5,
    )?;

    Ok(())
}

#[test]
#[ignore] // Requires golden images
fn test_visual_regression_boxplot() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use ruviz::prelude::*;

    println!("Testing: Box Plot");
    test_against_golden(
        || {
            let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 25.0];
            Plot::new()
                .boxplot(&data, None)
                .title("Box Plot")
                .ylabel("Value")
                .save("tests/output/vr_boxplot.png")?;
            Ok(())
        },
        "tests/output/vr_boxplot.png",
        "tests/golden_images/06_boxplot.png",
        0.5,
    )?;

    Ok(())
}

#[test]
#[ignore] // Requires golden images
fn test_visual_regression_themes() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use ruviz::prelude::*;

    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    for (theme, name, idx) in [
        (Theme::light(), "light", 7),
        (Theme::dark(), "dark", 8),
        (Theme::publication(), "publication", 9),
        (Theme::seaborn(), "seaborn", 10),
    ] {
        println!("Testing: {} Theme", name);
        test_against_golden(
            || {
                Plot::new()
                    .theme(theme)
                    .line(&x, &y)
                    .title(&format!("{} Theme", name.to_uppercase()))
                    .xlabel("X")
                    .ylabel("Y")
                    .save(&format!("tests/output/vr_theme_{}.png", name))?;
                Ok(())
            },
            &format!("tests/output/vr_theme_{}.png", name),
            &format!("tests/golden_images/0{}_theme_{}.png", idx, name),
            0.5,
        )?;
    }

    Ok(())
}

#[test]
#[ignore] // Requires golden images
fn test_visual_regression_unicode() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use ruviz::prelude::*;

    println!("Testing: Unicode Text");
    test_against_golden(
        || {
            let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
            let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];
            Plot::new()
                .line(&x, &y)
                .title("Unicode: α β γ δ ε θ λ π σ ω")
                .xlabel("Température (°C)")
                .ylabel("Résultat")
                .save("tests/output/vr_unicode.png")?;
            Ok(())
        },
        "tests/output/vr_unicode.png",
        "tests/golden_images/24_unicode.png",
        0.5,
    )?;

    Ok(())
}

#[test]
#[ignore] // Requires golden images
fn test_visual_regression_dimensions() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use ruviz::prelude::*;

    println!("Testing: Custom Dimensions");
    test_against_golden(
        || {
            let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
            let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];
            Plot::new()
                .dimensions(1200, 900)
                .line(&x, &y)
                .title("Custom Dimensions")
                .save("tests/output/vr_custom_dimensions.png")?;
            Ok(())
        },
        "tests/output/vr_custom_dimensions.png",
        "tests/golden_images/14_custom_dimensions.png",
        0.5,
    )?;

    Ok(())
}
