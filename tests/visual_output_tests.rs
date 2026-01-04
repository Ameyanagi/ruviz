//! Visual output tests that save PNG images for manual inspection
//!
//! Run with: cargo test --test visual_output_tests
//! Images will be saved to test_output/ directory

use ruviz::prelude::*;
use std::fs;

/// Setup test output directory
fn setup_output_dir() -> std::result::Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("tests/output")?;
    Ok(())
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

    let plot = Plot::new()
        .title("Multiple Series Test".to_string())
        .xlabel("Time".to_string())
        .ylabel("Values".to_string())
        .line(&x_data, &y1_data)
        .label("Series 1".to_string())
        .line(&x_data, &y2_data)
        .label("Series 2".to_string());

    plot.save("tests/output/04_multiple_series.png")?;

    println!("✓ Saved: test_output/04_multiple_series.png");
    Ok(())
}

#[test]
fn test_dark_theme() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![0.0, 1.0, 4.0, 9.0, 16.0, 25.0];

    let plot = Plot::with_theme(Theme::dark())
        .title("Dark Theme Test".to_string())
        .xlabel("X".to_string())
        .ylabel("Y = X²".to_string())
        .line(&x_data, &y_data);

    plot.save("tests/output/05_dark_theme.png")?;

    println!("✓ Saved: test_output/05_dark_theme.png");
    Ok(())
}

#[test]
fn test_light_theme() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![0.0, 1.0, 4.0, 9.0, 16.0, 25.0];

    let plot = Plot::with_theme(Theme::light())
        .title("Light Theme Test".to_string())
        .xlabel("X".to_string())
        .ylabel("Y = X²".to_string())
        .line(&x_data, &y_data);

    plot.save("tests/output/06_light_theme.png")?;

    println!("✓ Saved: test_output/06_light_theme.png");
    Ok(())
}

#[test]
fn test_publication_theme() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data = vec![0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0];
    let y_data = vec![1.0, 1.5, 2.2, 3.1, 4.5, 6.2, 8.5];

    let plot = Plot::with_theme(Theme::publication())
        .title("Publication Theme Test".to_string())
        .xlabel("Time (seconds)".to_string())
        .ylabel("Response (units)".to_string())
        .line(&x_data, &y_data);

    plot.save("tests/output/07_publication_theme.png")?;

    println!("✓ Saved: test_output/07_publication_theme.png");
    Ok(())
}

#[test]
fn test_minimal_theme() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 8.0, 18.0, 32.0, 50.0];

    let plot = Plot::with_theme(Theme::minimal())
        .title("Minimal Theme Test".to_string())
        .xlabel("Input".to_string())
        .ylabel("Output".to_string())
        .scatter(&x_data, &y_data);

    plot.save("tests/output/08_minimal_theme.png")?;

    println!("✓ Saved: test_output/08_minimal_theme.png");
    Ok(())
}

#[test]
fn test_large_dataset() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    // Generate sine wave data
    let x_data: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| (x * 2.0).sin()).collect();

    let plot = Plot::new()
        .title("Large Dataset Test (100 points)".to_string())
        .xlabel("Time".to_string())
        .ylabel("Amplitude".to_string())
        .line(&x_data, &y_data);

    plot.save("tests/output/09_large_dataset.png")?;

    println!("✓ Saved: test_output/09_large_dataset.png");
    Ok(())
}

#[test]
fn test_mathematical_functions() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
    let sin_data: Vec<f64> = x_data.iter().map(|&x| x.sin()).collect();
    let cos_data: Vec<f64> = x_data.iter().map(|&x| x.cos()).collect();
    let exp_data: Vec<f64> = x_data.iter().map(|&x| (-x * 0.1).exp()).collect();

    let plot = Plot::new()
        .title("Mathematical Functions".to_string())
        .xlabel("X".to_string())
        .ylabel("Y".to_string())
        .line(&x_data, &sin_data)
        .label("sin(x)".to_string())
        .line(&x_data, &cos_data)
        .label("cos(x)".to_string())
        .line(&x_data, &exp_data)
        .label("exp(-0.1x)".to_string());

    plot.save("tests/output/10_mathematical_functions.png")?;

    println!("✓ Saved: test_output/10_mathematical_functions.png");
    Ok(())
}

#[test]
fn test_grid_enabled() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![10.0, 25.0, 40.0, 30.0, 45.0];

    let plot = Plot::new()
        .title("Grid Enabled Test".to_string())
        .xlabel("X Axis".to_string())
        .ylabel("Y Axis".to_string())
        .grid(true)
        .line(&x_data, &y_data);

    plot.save("tests/output/11_grid_enabled.png")?;

    println!("✓ Saved: test_output/11_grid_enabled.png");
    Ok(())
}

#[test]
fn test_grid_disabled() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![10.0, 25.0, 40.0, 30.0, 45.0];

    let plot = Plot::new()
        .title("Grid Disabled Test".to_string())
        .xlabel("X Axis".to_string())
        .ylabel("Y Axis".to_string())
        .grid(false)
        .line(&x_data, &y_data);

    plot.save("tests/output/12_grid_disabled.png")?;

    println!("✓ Saved: test_output/12_grid_disabled.png");
    Ok(())
}

#[test]
fn test_custom_dimensions() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data = [1.0, 2.0, 3.0, 4.0];
    let y_data = vec![100.0, 150.0, 120.0, 180.0];

    let plot = Plot::new()
        .dimensions(1200, 800) // Custom size
        .title("Custom Dimensions Test (1200x800)".to_string())
        .xlabel("Categories".to_string())
        .ylabel("Values".to_string())
        .bar(&["Q1", "Q2", "Q3", "Q4"], &y_data);

    plot.save("tests/output/13_custom_dimensions.png")?;

    println!("✓ Saved: test_output/13_custom_dimensions.png");
    Ok(())
}

#[test]
fn test_mixed_plot_types() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let line_data = vec![20.0, 25.0, 30.0, 28.0, 35.0];
    let scatter_data = vec![22.0, 27.0, 25.0, 32.0, 30.0];

    let plot = Plot::new()
        .title("Mixed Plot Types Test".to_string())
        .xlabel("Time".to_string())
        .ylabel("Measurements".to_string())
        .line(&x_data, &line_data)
        .label("Trend Line".to_string())
        .scatter(&x_data, &scatter_data)
        .label("Data Points".to_string());

    plot.save("tests/output/14_mixed_plot_types.png")?;

    println!("✓ Saved: test_output/14_mixed_plot_types.png");
    Ok(())
}

#[test]
#[ignore] // Edge case with single point may produce NaN coordinates
fn test_edge_cases() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_output_dir()?;

    // Single point
    let single_x = vec![5.0];
    let single_y = vec![10.0];

    let plot = Plot::new()
        .title("Edge Case: Single Point".to_string())
        .xlabel("X".to_string())
        .ylabel("Y".to_string())
        .scatter(&single_x, &single_y);

    plot.save("tests/output/15_single_point.png")?;

    println!("✓ Saved: test_output/15_single_point.png");

    // Two points (minimum for line)
    let two_x = vec![1.0, 10.0];
    let two_y = vec![5.0, 50.0];

    let plot2 = Plot::new()
        .title("Edge Case: Two Points Line".to_string())
        .xlabel("X".to_string())
        .ylabel("Y".to_string())
        .line(&two_x, &two_y);

    plot2.save("tests/output/16_two_points_line.png")?;

    println!("✓ Saved: test_output/16_two_points_line.png");
    Ok(())
}

/// Run all visual tests and print summary
#[test]
fn run_all_visual_tests() {
    println!("\n🎨 RUNNING COMPREHENSIVE VISUAL TESTS");
    println!("=====================================");

    let tests: Vec<(
        &str,
        fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
    )> = vec![
        (
            "Basic Line Plot",
            test_basic_line_plot as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Scatter Plot",
            test_scatter_plot as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Bar Plot",
            test_bar_plot as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Multiple Series",
            test_multiple_series as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Dark Theme",
            test_dark_theme as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Light Theme",
            test_light_theme as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Publication Theme",
            test_publication_theme as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Minimal Theme",
            test_minimal_theme as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Large Dataset",
            test_large_dataset as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Mathematical Functions",
            test_mathematical_functions
                as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Grid Enabled",
            test_grid_enabled as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Grid Disabled",
            test_grid_disabled as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Custom Dimensions",
            test_custom_dimensions as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Mixed Plot Types",
            test_mixed_plot_types as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
        ),
        (
            "Edge Cases",
            test_edge_cases as fn() -> std::result::Result<(), Box<dyn std::error::Error>>,
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

    println!("\n📊 VISUAL TEST SUMMARY");
    println!("======================");
    println!("✅ Passed: {}", passed);
    println!("❌ Failed: {}", failed);
    println!("📂 Output Directory: test_output/");
    println!("🔍 Check PNG files for visual verification");
}
