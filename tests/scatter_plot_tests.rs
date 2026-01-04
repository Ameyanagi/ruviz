use ruviz::prelude::*;
use std::fs;

#[test]
fn test_basic_scatter_plot() {
    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Basic Scatter Plot")
        .xlabel("X Values")
        .ylabel("Y Values")
        .scatter(&x_data, &y_data)
        .save("tests/output/basic_scatter.png");

    assert!(
        result.is_ok(),
        "Basic scatter plot should export successfully"
    );

    let path = std::path::Path::new("tests/output/basic_scatter.png");
    assert!(path.exists(), "Output file should exist");

    let metadata = fs::metadata(path).unwrap();
    assert!(
        metadata.len() > 1000,
        "PNG file should have reasonable size"
    );
}

#[test]
fn test_scatter_plot_different_dpi() {
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y_data = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    let dpi_values = vec![96, 150, 300, 600];
    let filenames = vec![
        "scatter_96_test.png",
        "scatter_150_test.png",
        "scatter_300_test.png",
        "scatter_600_test.png",
    ];

    for (dpi, filename) in dpi_values.iter().zip(filenames.iter()) {
        let result = Plot::new()
            .dimensions(800, 600)
            .dpi(*dpi)
            .title("DPI Scaling Test - Scatter")
            .xlabel("X Axis")
            .ylabel("Y Axis")
            .scatter(&x_data, &y_data)
            .save(format!("tests/output/{}", filename));

        assert!(
            result.is_ok(),
            "Scatter plot at {} DPI should export successfully",
            dpi
        );

        let filepath = format!("tests/output/{}", filename);
        let path = std::path::Path::new(&filepath);
        assert!(path.exists(), "Output file should exist for {} DPI", dpi);

        let metadata = fs::metadata(path).unwrap();
        assert!(
            metadata.len() > 1000,
            "PNG file should have reasonable size for {} DPI",
            dpi
        );
    }
}

#[test]
fn test_scatter_plot_with_custom_colors() {
    let x_data = vec![1.0, 2.0, 3.0, 4.0];
    let y_data = vec![1.0, 4.0, 2.0, 3.0];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Custom Color Scatter Plot")
        .xlabel("X Values")
        .ylabel("Y Values")
        .scatter(&x_data, &y_data)
        .color(Color::new(255, 0, 0))
        .save("tests/output/scatter_custom_color.png");

    assert!(
        result.is_ok(),
        "Scatter plot with custom color should export successfully"
    );
}

#[test]
fn test_multiple_scatter_series() {
    let x1_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y1_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];
    let x2_data = vec![1.5, 2.5, 3.5, 4.5, 5.5];
    let y2_data = vec![1.0, 3.0, 2.0, 4.0, 1.5];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Multiple Scatter Series")
        .xlabel("X Values")
        .ylabel("Y Values")
        .scatter(&x1_data, &y1_data)
        .color(Color::new(255, 0, 0))
        .scatter(&x2_data, &y2_data)
        .color(Color::new(0, 0, 255))
        .save("tests/output/multiple_scatter_series.png");

    assert!(
        result.is_ok(),
        "Multiple scatter series should export successfully"
    );
}

#[test]
fn test_scatter_plot_large_dataset() {
    // Test with 1000 points for performance
    let n = 1000;
    let x_data: Vec<f64> = (0..n).map(|i| i as f64 / 100.0).collect();
    let y_data: Vec<f64> = x_data
        .iter()
        .map(|x| x.sin() + 0.1 * (x * 10.0).cos())
        .collect();

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Large Dataset Scatter Plot (1000 points)")
        .xlabel("X Values")
        .ylabel("Y Values")
        .scatter(&x_data, &y_data)
        .save("tests/output/scatter_large_dataset.png");

    assert!(
        result.is_ok(),
        "Large dataset scatter plot should export successfully"
    );
}

#[test]
fn test_scatter_plot_empty_data() {
    let x_data: Vec<f64> = vec![];
    let y_data: Vec<f64> = vec![];

    // This should fail when trying to render, not when creating the plot
    let plot = Plot::new()
        .dimensions(800, 600)
        .title("Empty Scatter Plot")
        .xlabel("X Values")
        .ylabel("Y Values")
        .scatter(&x_data, &y_data);

    let result = plot.save("tests/output/scatter_empty.png");
    assert!(
        result.is_err(),
        "Empty data should return an error when saving"
    );
}

#[test]
fn test_scatter_plot_mismatched_data() {
    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![1.0, 2.0]; // Different length

    // This should work at plot creation but fail at render/save due to validation
    let plot = Plot::new().dimensions(800, 600).scatter(&x_data, &y_data);

    let result = plot.save("tests/output/scatter_mismatched.png");
    assert!(
        result.is_err(),
        "Mismatched data lengths should return an error"
    );
}
