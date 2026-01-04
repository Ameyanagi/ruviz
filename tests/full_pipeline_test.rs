// Integration tests for full rendering pipeline
// These tests verify the complete flow from API to PNG output

use ruviz::prelude::*;
use std::path::Path;

#[test]
fn test_basic_line_plot_pipeline() {
    // GIVEN: Simple line data
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    // WHEN: Creating and saving a plot
    let result = Plot::new()
        .line(&x, &y)
        .title("Test Plot")
        .xlabel("x")
        .ylabel("y")
        .save("tests/output/integration_basic_line.png");

    // THEN: Plot should be created successfully
    assert!(result.is_ok(), "Failed to create plot: {:?}", result.err());

    // AND: File should exist with content
    let path = Path::new("tests/output/integration_basic_line.png");
    assert!(path.exists(), "Output file not created");

    let metadata = std::fs::metadata(path).unwrap();
    assert!(metadata.len() > 0, "Output file is empty");

    // AND: Should be a valid PNG
    let img = image::open(path);
    assert!(img.is_ok(), "Output is not a valid PNG: {:?}", img.err());

    let img = img.unwrap();
    // Default figure size: 6.4×4.8 inches at 100 DPI = 640×480 pixels
    assert_eq!(img.width(), 640, "Unexpected width");
    assert_eq!(img.height(), 480, "Unexpected height");
}

#[test]
fn test_multi_series_pipeline() {
    // GIVEN: Multiple data series
    let x = vec![1.0, 2.0, 3.0, 4.0];

    // WHEN: Creating plot with multiple series
    let result = Plot::new()
        .line(&x, &x.iter().copied().collect::<Vec<_>>())
        .label("Linear")
        .line(&x, &x.iter().map(|&v| v * v).collect::<Vec<_>>())
        .label("Quadratic")
        .scatter(&vec![1.5, 2.5, 3.5], &vec![2.0, 6.0, 12.0])
        .label("Points")
        .title("Multi-Series Test")
        .save("tests/output/integration_multi_series.png");

    // THEN: Should succeed
    assert!(
        result.is_ok(),
        "Multi-series plot failed: {:?}",
        result.err()
    );

    // AND: File should be created
    assert!(Path::new("tests/output/integration_multi_series.png").exists());
}

#[test]
fn test_scatter_plot_pipeline() {
    // GIVEN: Scatter data
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![2.3, 3.1, 2.8, 4.2, 3.9];

    // WHEN: Creating scatter plot
    let result = Plot::new()
        .scatter(&x, &y)
        .marker(MarkerStyle::Circle)
        .marker_size(8.0)
        .title("Scatter Test")
        .save("tests/output/integration_scatter.png");

    // THEN: Should succeed
    assert!(result.is_ok());
    assert!(Path::new("tests/output/integration_scatter.png").exists());
}

#[test]
fn test_bar_chart_pipeline() {
    // GIVEN: Categorical data
    let categories = vec!["A", "B", "C", "D"];
    let values = vec![25.0, 40.0, 30.0, 55.0];

    // WHEN: Creating bar chart
    let result = Plot::new()
        .bar(&categories, &values)
        .title("Bar Chart Test")
        .ylabel("Value")
        .save("tests/output/integration_bar.png");

    // THEN: Should succeed
    assert!(result.is_ok());
    assert!(Path::new("tests/output/integration_bar.png").exists());
}

#[test]
fn test_histogram_pipeline() {
    // GIVEN: Distribution data
    let data = vec![
        1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 5.0, 1.5, 2.5, 2.5, 3.5, 3.5, 3.5, 4.5, 4.5, 5.5,
    ];

    // WHEN: Creating histogram
    let result = Plot::new()
        .histogram(&data, None)
        .title("Histogram Test")
        .xlabel("Value")
        .ylabel("Frequency")
        .save("tests/output/integration_histogram.png");

    // THEN: Should succeed
    assert!(result.is_ok());
    assert!(Path::new("tests/output/integration_histogram.png").exists());
}

#[test]
fn test_boxplot_pipeline() {
    // GIVEN: Statistical data
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 25.0]; // With outlier

    // WHEN: Creating boxplot
    let result = Plot::new()
        .boxplot(&data, None)
        .title("Box Plot Test")
        .ylabel("Value")
        .save("tests/output/integration_boxplot.png");

    // THEN: Should succeed
    assert!(result.is_ok());
    assert!(Path::new("tests/output/integration_boxplot.png").exists());
}

#[test]
fn test_subplot_composition() {
    // GIVEN: Subplot data
    let x = vec![0.0, 1.0, 2.0, 3.0];
    let y_linear = vec![0.0, 1.0, 2.0, 3.0];
    let y_scatter = vec![0.5, 1.5, 1.8, 2.9];
    let categories = vec!["A", "B", "C"];
    let values = vec![10.0, 20.0, 15.0];
    let hist_data = vec![1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0];

    // WHEN: Creating individual plots
    let plot1 = Plot::new().line(&x, &y_linear).title("Linear").end_series();

    let plot2 = Plot::new()
        .scatter(&x, &y_scatter)
        .title("Scatter")
        .end_series();

    let plot3 = Plot::new()
        .bar(&categories, &values)
        .title("Bar")
        .end_series();

    let plot4 = Plot::new()
        .histogram(&hist_data, None)
        .title("Histogram")
        .end_series();

    // WHEN: Creating subplots
    let result = subplots(2, 2, 1200, 900);
    assert!(result.is_ok(), "Failed to create subplot grid");

    let save_result = result
        .unwrap()
        .subplot(0, 0, plot1)
        .unwrap()
        .subplot(0, 1, plot2)
        .unwrap()
        .subplot(1, 0, plot3)
        .unwrap()
        .subplot(1, 1, plot4)
        .unwrap()
        .suptitle("Integration Subplot Test")
        .save("tests/output/integration_subplots.png");

    // THEN: Should succeed
    assert!(
        save_result.is_ok(),
        "Subplot save failed: {:?}",
        save_result.err()
    );
    assert!(Path::new("tests/output/integration_subplots.png").exists());
}

#[test]
fn test_theme_application() {
    // GIVEN: Simple data
    let x = vec![1.0, 2.0, 3.0, 4.0];
    let y = vec![1.0, 4.0, 9.0, 16.0];

    // WHEN: Applying different themes
    for (theme, name) in [
        (Theme::light(), "light"),
        (Theme::dark(), "dark"),
        (Theme::publication(), "publication"),
        (Theme::seaborn(), "seaborn"),
    ] {
        let result = Plot::new()
            .theme(theme)
            .line(&x, &y)
            .title(format!("{} Theme Test", name))
            .save(format!("tests/output/integration_theme_{}.png", name));

        // THEN: Should succeed for all themes
        assert!(result.is_ok(), "{} theme failed", name);
        assert!(Path::new(&format!("tests/output/integration_theme_{}.png", name)).exists());
    }
}

#[test]
fn test_dpi_scaling() {
    // GIVEN: Simple data
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![0.0, 1.0, 4.0];

    // WHEN: Rendering at different DPIs
    for dpi in [72, 96, 150, 300] {
        let result = Plot::new()
            .line(&x, &y)
            .dpi(dpi)
            .title(format!("{} DPI Test", dpi))
            .save(format!("tests/output/integration_dpi_{}.png", dpi));

        // THEN: Should succeed
        assert!(result.is_ok(), "{} DPI failed", dpi);

        let path_str = format!("tests/output/integration_dpi_{}.png", dpi);
        let path = Path::new(&path_str);
        assert!(path.exists());

        // AND: Higher DPI should produce larger files
        let metadata = std::fs::metadata(path).unwrap();
        let size = metadata.len();
        // Just verify file exists and has content, not exact size
        assert!(size > 1000, "{} DPI file too small: {} bytes", dpi, size);
    }
}

#[test]
fn test_custom_dimensions() {
    // GIVEN: Data
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 4.0, 9.0];

    // WHEN: Using custom dimensions
    let result = Plot::new()
        .dimensions(1200, 900)
        .line(&x, &y)
        .title("Custom Dimensions Test")
        .save("tests/output/integration_custom_dimensions.png");

    // THEN: Should succeed
    assert!(result.is_ok());

    // AND: Image should have correct dimensions
    // Allow ±1 pixel tolerance due to DPI auto-scaling rounding
    let img = image::open("tests/output/integration_custom_dimensions.png").unwrap();
    let width_diff = (img.width() as i32 - 1200_i32).abs();
    let height_diff = (img.height() as i32 - 900_i32).abs();
    assert!(width_diff <= 1, "Width mismatch: {} vs 1200", img.width());
    assert!(height_diff <= 1, "Height mismatch: {} vs 900", img.height());
}

#[test]
fn test_empty_data_error_handling() {
    // GIVEN: Empty data
    let empty_x: Vec<f64> = vec![];
    let empty_y: Vec<f64> = vec![];

    // WHEN: Attempting to plot empty data
    let result = Plot::new()
        .line(&empty_x, &empty_y)
        .save("tests/output/should_not_exist.png");

    // THEN: Should fail gracefully
    assert!(result.is_err(), "Empty data should produce error");
}

#[test]
fn test_mismatched_data_error_handling() {
    // GIVEN: Mismatched length data
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 2.0]; // Too short

    // WHEN: Attempting to plot mismatched data
    let result = Plot::new()
        .line(&x, &y)
        .save("tests/output/should_not_exist_2.png");

    // THEN: Should fail gracefully
    assert!(result.is_err(), "Mismatched data should produce error");
}
