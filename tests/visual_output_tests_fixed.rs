//! Visual output tests that save PNG images for manual inspection
//!
//! Run with: cargo test --test visual_output_tests_fixed
//! Images will be saved to test_output/ directory

use ruviz::prelude::*;
use std::fs;

/// Setup test output directory
fn setup_output_dir() -> std::result::Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("test_output")?;
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
        .save("test_output/01_basic_line_plot.png")?;

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
        .save("test_output/02_scatter_plot.png")?;

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
        .save("test_output/03_bar_plot.png")?;

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
        .save("test_output/04_multiple_series.png")?;

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
        .save("test_output/05_dark_theme.png")?;

    // Light theme
    Plot::with_theme(Theme::light())
        .title("Light Theme Test".to_string())
        .xlabel("X".to_string())
        .ylabel("Y = X²".to_string())
        .line(&x_data, &y_data)
        .end_series()
        .save("test_output/06_light_theme.png")?;

    // Publication theme
    Plot::with_theme(Theme::publication())
        .title("Publication Theme Test".to_string())
        .xlabel("Time (seconds)".to_string())
        .ylabel("Response (units)".to_string())
        .line(&x_data, &y_data)
        .end_series()
        .save("test_output/07_publication_theme.png")?;

    // Minimal theme
    Plot::with_theme(Theme::minimal())
        .title("Minimal Theme Test".to_string())
        .xlabel("Input".to_string())
        .ylabel("Output".to_string())
        .scatter(&x_data, &y_data)
        .end_series()
        .save("test_output/08_minimal_theme.png")?;

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
        .save("test_output/09_large_dataset.png")?;

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
        .save("test_output/10_mathematical_functions.png")?;

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
        .save("test_output/11_grid_enabled.png")?;

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
        .save("test_output/12_custom_dimensions.png")?;

    println!("✓ Saved: test_output/12_custom_dimensions.png");
    Ok(())
}

#[test]
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
        .save("test_output/13_single_point.png")?;

    // Two points line
    let two_x = vec![1.0, 10.0];
    let two_y = vec![5.0, 50.0];

    Plot::new()
        .title("Edge Case: Two Points Line".to_string())
        .xlabel("X".to_string())
        .ylabel("Y".to_string())
        .line(&two_x, &two_y)
        .end_series()
        .save("test_output/14_two_points_line.png")?;

    println!("✓ Saved: Edge case tests");
    Ok(())
}
