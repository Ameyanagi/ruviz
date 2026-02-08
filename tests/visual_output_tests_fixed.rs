//! Visual output tests that save PNG images for manual inspection
//!
//! Run with: cargo test --test visual_output_tests_fixed
//! Images will be saved to test_output/ directory

use ruviz::prelude::*;
use std::fs;

/// Setup test output directory
fn setup_output_dir() -> std::result::Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("tests/output")?;
    Ok(())
}

#[cfg(feature = "typst-math")]
const PIXEL_DIFF_THRESHOLD: i16 = 8;

#[cfg(feature = "typst-math")]
fn region_non_bg_bbox(
    image: &image::RgbImage,
    x_start: u32,
    x_end: u32,
    y_start: u32,
    y_end: u32,
) -> Option<(u32, u32, u32, u32)> {
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return None;
    }

    let x_start = x_start.min(width);
    let x_end = x_end.min(width);
    let y_start = y_start.min(height);
    let y_end = y_end.min(height);
    if x_start >= x_end || y_start >= y_end {
        return None;
    }

    let bg = image.get_pixel(0, 0).0;
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0_u32;
    let mut max_y = 0_u32;
    let mut found = false;

    for y in y_start..y_end {
        for x in x_start..x_end {
            let px = image.get_pixel(x, y).0;
            let max_diff = [
                (px[0] as i16 - bg[0] as i16).abs(),
                (px[1] as i16 - bg[1] as i16).abs(),
                (px[2] as i16 - bg[2] as i16).abs(),
            ]
            .into_iter()
            .max()
            .unwrap_or(0);
            if max_diff > PIXEL_DIFF_THRESHOLD {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if found {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

#[test]
fn test_basic_line_plot() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0, 2.5, 4.5, 1.5, 3.5, 6.0];

    Plot::new()
        .title("Basic Line Plot Test".to_string())
        .xlabel("X Values".to_string())
        .ylabel("Y Values".to_string())
        .line(&x_data, &y_data)
        .end_series()
        .save("tests/output/01_basic_line_plot.png")?;

    println!("✓ Saved: test_output/01_basic_line_plot.png");
    Ok(())
}

#[test]
fn test_scatter_plot() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let y_data = vec![2.5, 4.1, 1.8, 3.7, 5.2, 2.1, 4.8, 1.3];

    Plot::new()
        .title("Scatter Plot Test".to_string())
        .xlabel("X Coordinates".to_string())
        .ylabel("Y Coordinates".to_string())
        .scatter(&x_data, &y_data)
        .end_series()
        .save("tests/output/02_scatter_plot.png")?;

    println!("✓ Saved: test_output/02_scatter_plot.png");
    Ok(())
}

#[test]
fn test_bar_plot() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let categories = vec!["Apple", "Banana", "Cherry", "Date", "Elderberry"];
    let values = vec![25.0, 30.0, 15.0, 40.0, 20.0];

    Plot::new()
        .title("Bar Plot Test".to_string())
        .xlabel("Fruits".to_string())
        .ylabel("Sales Count".to_string())
        .bar(&categories, &values)
        .end_series()
        .save("tests/output/03_bar_plot.png")?;

    println!("✓ Saved: test_output/03_bar_plot.png");
    Ok(())
}

#[test]
fn test_multiple_series() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let y1_data = vec![2.0, 4.0, 1.0, 3.0, 5.0, 2.5, 4.5, 1.5];
    let y2_data = vec![1.5, 3.5, 2.5, 4.5, 3.0, 5.5, 2.0, 4.0];

    Plot::new()
        .title("Multiple Series Test".to_string())
        .xlabel("Time".to_string())
        .ylabel("Values".to_string())
        .line(&x_data, &y1_data)
        .label("Series 1".to_string())
        .line(&x_data, &y2_data)
        .label("Series 2".to_string())
        .end_series()
        .save("tests/output/04_multiple_series.png")?;

    println!("✓ Saved: test_output/04_multiple_series.png");
    Ok(())
}

#[test]
fn test_themes() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![0.0, 1.0, 4.0, 9.0, 16.0, 25.0];

    // Dark theme
    Plot::with_theme(Theme::dark())
        .title("Dark Theme Test".to_string())
        .xlabel("X".to_string())
        .ylabel("Y = X²".to_string())
        .line(&x_data, &y_data)
        .end_series()
        .save("tests/output/05_dark_theme.png")?;

    // Light theme
    Plot::with_theme(Theme::light())
        .title("Light Theme Test".to_string())
        .xlabel("X".to_string())
        .ylabel("Y = X²".to_string())
        .line(&x_data, &y_data)
        .end_series()
        .save("tests/output/06_light_theme.png")?;

    // Publication theme
    Plot::with_theme(Theme::publication())
        .title("Publication Theme Test".to_string())
        .xlabel("Time (seconds)".to_string())
        .ylabel("Response (units)".to_string())
        .line(&x_data, &y_data)
        .end_series()
        .save("tests/output/07_publication_theme.png")?;

    // Minimal theme
    Plot::with_theme(Theme::minimal())
        .title("Minimal Theme Test".to_string())
        .xlabel("Input".to_string())
        .ylabel("Output".to_string())
        .scatter(&x_data, &y_data)
        .end_series()
        .save("tests/output/08_minimal_theme.png")?;

    println!("✓ Saved: All theme tests");
    Ok(())
}

#[test]
fn test_large_dataset() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    // Generate sine wave data
    let x_data: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| (x * 2.0).sin()).collect();

    Plot::new()
        .title("Large Dataset Test (100 points)".to_string())
        .xlabel("Time".to_string())
        .ylabel("Amplitude".to_string())
        .line(&x_data, &y_data)
        .end_series()
        .save("tests/output/09_large_dataset.png")?;

    println!("✓ Saved: test_output/09_large_dataset.png");
    Ok(())
}

#[test]
fn test_mathematical_functions() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
    let sin_data: Vec<f64> = x_data.iter().map(|&x| x.sin()).collect();
    let cos_data: Vec<f64> = x_data.iter().map(|&x| x.cos()).collect();

    Plot::new()
        .title("Mathematical Functions".to_string())
        .xlabel("X".to_string())
        .ylabel("Y".to_string())
        .line(&x_data, &sin_data)
        .label("sin(x)".to_string())
        .line(&x_data, &cos_data)
        .label("cos(x)".to_string())
        .end_series()
        .save("tests/output/10_mathematical_functions.png")?;

    println!("✓ Saved: test_output/10_mathematical_functions.png");
    Ok(())
}

#[test]
fn test_grid_options() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![10.0, 25.0, 40.0, 30.0, 45.0];

    // Grid enabled (default)
    Plot::new()
        .title("Grid Enabled Test".to_string())
        .xlabel("X Axis".to_string())
        .ylabel("Y Axis".to_string())
        .line(&x_data, &y_data)
        .end_series()
        .save("tests/output/11_grid_enabled.png")?;

    println!("✓ Saved: Grid tests");
    Ok(())
}

#[test]
fn test_custom_dimensions() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let categories = vec!["Q1", "Q2", "Q3", "Q4"];
    let values = vec![100.0, 150.0, 120.0, 180.0];

    Plot::new()
        .dimensions(1200, 800) // Custom size
        .title("Custom Dimensions Test (1200x800)".to_string())
        .xlabel("Quarters".to_string())
        .ylabel("Revenue".to_string())
        .bar(&categories, &values)
        .end_series()
        .save("tests/output/12_custom_dimensions.png")?;

    println!("✓ Saved: test_output/12_custom_dimensions.png");
    Ok(())
}

#[test]
fn test_typst_text_rendering() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data: Vec<f64> = (0..40).map(|i| i as f64 * 0.1).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| (x * 1.5).sin()).collect();
    let output_path = "tests/output/15_typst_text.png";

    let result = Plot::new()
        .title("$f(x) = sin(1.5x)$".to_string())
        .xlabel("$x$".to_string())
        .ylabel("$f(x)$".to_string())
        .line(&x_data, &y_data)
        .label("$sin(1.5x)$".to_string())
        .typst(true)
        .save(output_path);

    #[cfg(feature = "typst-math")]
    {
        result?;
        let size = fs::metadata(output_path)?.len();
        assert!(
            size > 2000,
            "Typst output file unexpectedly small: {} bytes",
            size
        );
        println!("✓ Saved: {}", output_path);
        fs::remove_file(output_path).ok();
    }

    #[cfg(not(feature = "typst-math"))]
    {
        assert!(result.is_err(), "Expected typst-math feature gate error");
        if fs::metadata(output_path).is_ok() {
            fs::remove_file(output_path).ok();
        }
    }

    Ok(())
}

#[test]
fn test_typst_layout_parity_no_clipping() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data: Vec<f64> = (0..80).map(|i| i as f64 * 0.05).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| (-x).exp()).collect();

    let plain_path = "tests/output/16_typst_parity_plain.png";
    let typst_path = "tests/output/17_typst_parity_typst.png";

    let plain_result = Plot::new()
        .title("Parity Title".to_string())
        .xlabel("Parity X".to_string())
        .ylabel("Parity Y".to_string())
        .line(&x_data, &y_data)
        .label("exp(-x)".to_string())
        .save(plain_path);

    let typst_result = Plot::new()
        .title("Parity Title".to_string())
        .xlabel("Parity X".to_string())
        .ylabel("Parity Y".to_string())
        .line(&x_data, &y_data)
        .label("exp(-x)".to_string())
        .typst(true)
        .save(typst_path);

    #[cfg(feature = "typst-math")]
    {
        plain_result?;
        typst_result?;

        let plain = image::open(plain_path)?.to_rgb8();
        let typst = image::open(typst_path)?.to_rgb8();

        assert_eq!(plain.dimensions(), typst.dimensions());
        let (w, h) = typst.dimensions();

        // Guard against top-edge clipping: no text should touch the first few rows.
        assert!(
            region_non_bg_bbox(&typst, 0, w, 0, 4).is_none(),
            "Typst output has non-background pixels in top guard rows (possible clipping): {}",
            typst_path
        );

        // Compare title placement in the top-center region.
        let title_region = (w / 6, (w * 5) / 6, 0, h / 4);
        let plain_title = region_non_bg_bbox(
            &plain,
            title_region.0,
            title_region.1,
            title_region.2,
            title_region.3,
        )
        .expect("plain title bbox should exist");
        let typst_title = region_non_bg_bbox(
            &typst,
            title_region.0,
            title_region.1,
            title_region.2,
            title_region.3,
        )
        .expect("typst title bbox should exist");
        let title_top_diff = (plain_title.1 as i32 - typst_title.1 as i32).abs();
        assert!(
            title_top_diff <= 18,
            "title top placement drift too large: plain={} typst={} (diff={})",
            plain_title.1,
            typst_title.1,
            title_top_diff
        );

        // Compare xlabel placement in the bottom-center region.
        let xlabel_region = (w / 5, (w * 4) / 5, (h * 3) / 4, h);
        let plain_xlabel = region_non_bg_bbox(
            &plain,
            xlabel_region.0,
            xlabel_region.1,
            xlabel_region.2,
            xlabel_region.3,
        )
        .expect("plain xlabel bbox should exist");
        let typst_xlabel = region_non_bg_bbox(
            &typst,
            xlabel_region.0,
            xlabel_region.1,
            xlabel_region.2,
            xlabel_region.3,
        )
        .expect("typst xlabel bbox should exist");
        let xlabel_top_diff = (plain_xlabel.1 as i32 - typst_xlabel.1 as i32).abs();
        assert!(
            xlabel_top_diff <= 12,
            "xlabel top placement drift too large: plain={} typst={} (diff={})",
            plain_xlabel.1,
            typst_xlabel.1,
            xlabel_top_diff
        );

        // Compare ylabel placement in a left-middle region.
        let ylabel_region = (0, w / 8, h / 4, (h * 3) / 4);
        let plain_ylabel = region_non_bg_bbox(
            &plain,
            ylabel_region.0,
            ylabel_region.1,
            ylabel_region.2,
            ylabel_region.3,
        )
        .expect("plain ylabel bbox should exist");
        let typst_ylabel = region_non_bg_bbox(
            &typst,
            ylabel_region.0,
            ylabel_region.1,
            ylabel_region.2,
            ylabel_region.3,
        )
        .expect("typst ylabel bbox should exist");
        let ylabel_center_plain = (plain_ylabel.1 + plain_ylabel.3) as i32 / 2;
        let ylabel_center_typst = (typst_ylabel.1 + typst_ylabel.3) as i32 / 2;
        let ylabel_center_diff = (ylabel_center_plain - ylabel_center_typst).abs();
        assert!(
            ylabel_center_diff <= 12,
            "ylabel center placement drift too large: plain={} typst={} (diff={})",
            ylabel_center_plain,
            ylabel_center_typst,
            ylabel_center_diff
        );

        println!("✓ Saved parity outputs: {}, {}", plain_path, typst_path);
    }

    #[cfg(not(feature = "typst-math"))]
    {
        plain_result?;
        assert!(
            typst_result.is_err(),
            "Expected typst-math feature gate error"
        );
        if fs::metadata(typst_path).is_ok() {
            fs::remove_file(typst_path).ok();
        }
    }

    Ok(())
}

#[test]
#[ignore] // Edge case with single point may produce NaN coordinates
fn test_edge_cases() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    // Single point scatter
    let single_x = vec![5.0];
    let single_y = vec![10.0];

    Plot::new()
        .title("Edge Case: Single Point".to_string())
        .xlabel("X".to_string())
        .ylabel("Y".to_string())
        .scatter(&single_x, &single_y)
        .end_series()
        .save("tests/output/13_single_point.png")?;

    // Two points line
    let two_x = vec![1.0, 10.0];
    let two_y = vec![5.0, 50.0];

    Plot::new()
        .title("Edge Case: Two Points Line".to_string())
        .xlabel("X".to_string())
        .ylabel("Y".to_string())
        .line(&two_x, &two_y)
        .end_series()
        .save("tests/output/14_two_points_line.png")?;

    println!("✓ Saved: Edge case tests");
    Ok(())
}
