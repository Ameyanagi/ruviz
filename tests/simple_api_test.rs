// Simple API tests - TDD approach
// Tests define expected one-liner function behavior before implementation

use ruviz::simple::*;
use std::path::Path;

#[test]
fn test_line_plot_one_liner() {
    // GIVEN: Simple data
    let x = vec![0.0, 1.0, 2.0, 3.0];
    let y = vec![0.0, 1.0, 4.0, 9.0];

    // WHEN: Using simple API
    let result = line_plot(&x, &y, "test_output/simple_line.png");

    // THEN: Should succeed and create file
    assert!(result.is_ok());
    assert!(Path::new("test_output/simple_line.png").exists());
}

#[test]
fn test_scatter_plot_one_liner() {
    // GIVEN: Simple data
    let x = vec![1.0, 2.0, 3.0, 4.0];
    let y = vec![1.0, 4.0, 9.0, 16.0];

    // WHEN: Using simple API
    let result = scatter_plot(&x, &y, "test_output/simple_scatter.png");

    // THEN: Should succeed
    assert!(result.is_ok());
    assert!(Path::new("test_output/simple_scatter.png").exists());
}

#[test]
fn test_bar_chart_one_liner() {
    // GIVEN: Categories and values
    let categories = vec!["A", "B", "C", "D"];
    let values = vec![10.0, 20.0, 15.0, 25.0];

    // WHEN: Using simple API
    let result = bar_chart(&categories, &values, "test_output/simple_bar.png");

    // THEN: Should succeed
    assert!(result.is_ok());
    assert!(Path::new("test_output/simple_bar.png").exists());
}

#[test]
fn test_histogram_one_liner() {
    // GIVEN: Data for histogram
    let data = vec![1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 5.0];

    // WHEN: Using simple API
    let result = histogram(&data, "test_output/simple_histogram.png");

    // THEN: Should succeed
    assert!(result.is_ok());
    assert!(Path::new("test_output/simple_histogram.png").exists());
}

#[test]
fn test_line_plot_with_title() {
    // GIVEN: Data and title
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![0.0, 1.0, 4.0];

    // WHEN: Using titled variant
    let result = line_plot_with_title(
        &x, &y,
        "Test Line Plot",
        "test_output/simple_line_titled.png"
    );

    // THEN: Should succeed
    assert!(result.is_ok());
    assert!(Path::new("test_output/simple_line_titled.png").exists());
}

#[test]
fn test_scatter_plot_with_title() {
    // GIVEN: Data and title
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 4.0, 9.0];

    // WHEN: Using titled variant
    let result = scatter_plot_with_title(
        &x, &y,
        "Test Scatter",
        "test_output/simple_scatter_titled.png"
    );

    // THEN: Should succeed
    assert!(result.is_ok());
}

#[test]
fn test_bar_chart_with_title() {
    // GIVEN: Categories, values, and title
    let categories = vec!["X", "Y", "Z"];
    let values = vec![5.0, 10.0, 7.0];

    // WHEN: Using titled variant
    let result = bar_chart_with_title(
        &categories, &values,
        "Test Bar Chart",
        "test_output/simple_bar_titled.png"
    );

    // THEN: Should succeed
    assert!(result.is_ok());
}

#[test]
fn test_histogram_with_title() {
    // GIVEN: Data and title
    let data = vec![1.0, 2.0, 2.0, 3.0, 4.0];

    // WHEN: Using titled variant
    let result = histogram_with_title(
        &data,
        "Test Histogram",
        "test_output/simple_histogram_titled.png"
    );

    // THEN: Should succeed
    assert!(result.is_ok());
}

#[test]
fn test_simple_api_auto_optimizes() {
    // GIVEN: Large dataset that should trigger optimization
    let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    // WHEN: Using simple API (should auto-optimize)
    let result = line_plot(&x, &y, "test_output/simple_optimized.png");

    // THEN: Should succeed (optimization happens internally)
    assert!(result.is_ok());
}

#[test]
fn test_simple_api_handles_errors() {
    // GIVEN: Invalid path
    let x = vec![1.0, 2.0];
    let y = vec![1.0, 2.0];

    // WHEN: Using invalid path
    let result = line_plot(&x, &y, "/invalid/path/plot.png");

    // THEN: Should return error (not panic)
    assert!(result.is_err());
}
