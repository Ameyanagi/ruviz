//! PDF Export Tests
//!
//! Tests for the SVG → PDF export pipeline.

mod common;

use ruviz::prelude::*;

#[test]
#[cfg(feature = "pdf")]
fn test_pdf_line_plot_export() {
    let output_path = common::test_output_path("pdf_line_plot.pdf");

    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    let result = Plot::new()
        .line(&x, &y)
        .label("y = x²")
        .title("Line Plot PDF Test")
        .xlabel("X Value")
        .ylabel("Y Value")
        .legend(Position::TopLeft)
        .save_pdf(&output_path);

    assert!(
        result.is_ok(),
        "PDF export should succeed: {:?}",
        result.err()
    );
    assert!(output_path.exists(), "PDF file should exist");

    // Check file starts with PDF magic bytes
    let contents = std::fs::read(&output_path).unwrap();
    assert!(
        contents.starts_with(b"%PDF-"),
        "File should be a valid PDF (starts with %PDF-)"
    );
}

#[test]
#[cfg(feature = "pdf")]
fn test_pdf_multi_series_export() {
    let output_path = common::test_output_path("pdf_multi_series.pdf");

    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y1 = vec![0.0, 1.0, 4.0, 9.0, 16.0];
    let y2 = vec![0.0, 2.0, 4.0, 6.0, 8.0];

    let result = Plot::new()
        .line(&x, &y1)
        .label("Quadratic")
        .color(Color::BLUE)
        .line(&x, &y2)
        .label("Linear")
        .color(Color::RED)
        .title("Multi-Series PDF Test")
        .xlabel("X Axis")
        .ylabel("Y Axis")
        .legend(Position::TopLeft)
        .save_pdf(&output_path);

    assert!(
        result.is_ok(),
        "PDF export should succeed: {:?}",
        result.err()
    );
    assert!(output_path.exists(), "PDF file should exist");
}

#[test]
#[cfg(feature = "pdf")]
fn test_pdf_scatter_plot_export() {
    let output_path = common::test_output_path("pdf_scatter_plot.pdf");

    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let y = vec![2.3, 4.1, 5.9, 8.2, 10.1, 12.4, 14.0, 16.3];

    let result = Plot::new()
        .scatter(&x, &y)
        .title("Scatter Plot PDF Test")
        .xlabel("X")
        .ylabel("Y")
        .save_pdf(&output_path);

    assert!(
        result.is_ok(),
        "PDF export should succeed: {:?}",
        result.err()
    );
    assert!(output_path.exists(), "PDF file should exist");
}

#[test]
fn test_svg_line_plot_export() {
    let output_path = common::test_output_path("svg_line_plot.svg");

    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    let result = Plot::new()
        .line(&x, &y)
        .label("y = x²")
        .title("Line Plot SVG Test")
        .xlabel("X Value")
        .ylabel("Y Value")
        .legend(Position::TopLeft)
        .export_svg(&output_path);

    assert!(
        result.is_ok(),
        "SVG export should succeed: {:?}",
        result.err()
    );
    assert!(output_path.exists(), "SVG file should exist");

    // Check file contains SVG content
    let contents = std::fs::read_to_string(&output_path).unwrap();
    assert!(contents.contains("<svg"), "File should contain SVG tag");
    assert!(
        contents.contains("Line Plot SVG Test"),
        "File should contain title"
    );
    assert!(contents.contains("X Value"), "File should contain xlabel");
    assert!(contents.contains("Y Value"), "File should contain ylabel");
}

#[test]
fn test_svg_bar_chart_export() {
    let output_path = common::test_output_path("svg_bar_chart.svg");

    let categories = vec!["A", "B", "C", "D"];
    let values = vec![10.0, 25.0, 15.0, 30.0];

    let result = Plot::new()
        .bar(&categories, &values)
        .title("Bar Chart SVG Test")
        .xlabel("Category")
        .ylabel("Value")
        .export_svg(&output_path);

    assert!(
        result.is_ok(),
        "SVG export should succeed: {:?}",
        result.err()
    );
    assert!(output_path.exists(), "SVG file should exist");
}

#[test]
fn test_render_to_svg_method() {
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    // Use end_series() to finalize the plot
    let plot = Plot::new()
        .line(&x, &y)
        .title("Render to SVG Test")
        .xlabel("X")
        .ylabel("Y")
        .end_series();

    let result = plot.render_to_svg();
    assert!(
        result.is_ok(),
        "render_to_svg should succeed: {:?}",
        result.err()
    );

    let svg = result.unwrap();
    assert!(svg.contains("<svg"), "Should contain SVG tag");
    assert!(svg.contains("</svg>"), "Should have closing SVG tag");
    assert!(svg.contains("polyline"), "Should contain polyline for data");
    assert!(svg.contains("Render to SVG Test"), "Should contain title");
}
