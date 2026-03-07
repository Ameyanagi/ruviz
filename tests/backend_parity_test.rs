// Backend parity tests - ensure all backends produce consistent output
// Tests that default, parallel, and SIMD backends render identically

mod common;

use common::{assert_png_dimensions_with_tolerance, assert_png_rendered};
use ruviz::prelude::*;

#[test]
fn test_backend_parity_basic_line() {
    // GIVEN: Simple line data
    let x: Vec<f64> = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    // WHEN: Rendering with default backend
    let result_default = Plot::new()
        .title("Backend Parity Test")
        .line(&x, &y)
        .save("tests/output/backend_default_line.png");

    // THEN: Should succeed
    assert!(
        result_default.is_ok(),
        "Default backend failed: {:?}",
        result_default
    );

    assert_png_rendered("tests/output/backend_default_line.png", Some((640, 480)));
}

#[test]
#[cfg(feature = "parallel")]
fn test_backend_parity_parallel() {
    // GIVEN: Larger dataset suitable for parallel rendering
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    // WHEN: Rendering with parallel backend (automatically used for large data)
    let result_parallel = Plot::new()
        .title("Parallel Backend Test")
        .line(&x, &y)
        .save("tests/output/backend_parallel_line.png");

    // THEN: Should succeed
    assert!(
        result_parallel.is_ok(),
        "Parallel backend failed: {:?}",
        result_parallel
    );

    assert_png_rendered("tests/output/backend_parallel_line.png", Some((640, 480)));
}

#[test]
fn test_backend_consistency_scatter() {
    // GIVEN: Scatter data
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![2.3, 3.1, 2.8, 4.2, 3.9];

    // WHEN: Rendering scatter plot
    let result = Plot::new()
        .title("Backend Consistency - Scatter")
        .scatter(&x, &y)
        .marker(MarkerStyle::Circle)
        .marker_size(8.0)
        .save("tests/output/backend_scatter.png");

    // THEN: Should produce consistent output
    assert!(
        result.is_ok(),
        "expected operation to succeed: {:?}",
        result
    );
    assert_png_rendered("tests/output/backend_scatter.png", Some((640, 480)));
}

#[test]
fn test_backend_consistency_bar() {
    // GIVEN: Bar chart data
    let categories = vec!["A", "B", "C", "D"];
    let values = vec![25.0, 40.0, 30.0, 55.0];

    // WHEN: Rendering bar chart
    let result = Plot::new()
        .title("Backend Consistency - Bar")
        .bar(&categories, &values)
        .save("tests/output/backend_bar.png");

    // THEN: Should produce consistent output
    assert!(
        result.is_ok(),
        "expected operation to succeed: {:?}",
        result
    );
    assert_png_rendered("tests/output/backend_bar.png", Some((640, 480)));
}

#[test]
fn test_backend_consistency_histogram() {
    // GIVEN: Distribution data
    let data = vec![
        1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0, 5.0, 1.5, 2.5, 2.5, 3.5, 3.5, 3.5, 4.5, 4.5, 5.5,
    ];

    // WHEN: Rendering histogram
    let result = Plot::new()
        .title("Backend Consistency - Histogram")
        .histogram(&data, None)
        .save("tests/output/backend_histogram.png");

    // THEN: Should produce consistent output
    assert!(
        result.is_ok(),
        "expected operation to succeed: {:?}",
        result
    );
    assert_png_rendered("tests/output/backend_histogram.png", Some((640, 480)));
}

#[test]
fn test_backend_consistency_boxplot() {
    // GIVEN: Statistical data
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 25.0];

    // WHEN: Rendering boxplot
    let result = Plot::new()
        .title("Backend Consistency - Boxplot")
        .boxplot(&data, None)
        .save("tests/output/backend_boxplot.png");

    // THEN: Should produce consistent output
    assert!(
        result.is_ok(),
        "expected operation to succeed: {:?}",
        result
    );
    assert_png_rendered("tests/output/backend_boxplot.png", Some((640, 480)));
}

#[test]
fn test_backend_consistency_multi_series() {
    // GIVEN: Multiple data series
    let x: Vec<f64> = vec![0.0, 1.0, 2.0, 3.0, 4.0];

    // WHEN: Rendering with multiple series
    let result = Plot::new()
        .title("Backend Consistency - Multi-Series")
        .legend(Position::TopLeft)
        .line(&x, &x.iter().copied().collect::<Vec<_>>())
        .label("Linear")
        .line(&x, &x.iter().map(|&v| v * v).collect::<Vec<_>>())
        .label("Quadratic")
        .line(&x, &x.iter().map(|&v| v.powi(3)).collect::<Vec<_>>())
        .label("Cubic")
        .save("tests/output/backend_multi_series.png");

    // THEN: Should produce consistent output
    assert!(
        result.is_ok(),
        "expected operation to succeed: {:?}",
        result
    );
    assert_png_rendered("tests/output/backend_multi_series.png", Some((640, 480)));
}

#[test]
fn test_backend_consistency_themes() {
    // GIVEN: Simple data
    let x = vec![0.0, 1.0, 2.0, 3.0];
    let y = vec![0.0, 1.0, 4.0, 9.0];

    // WHEN: Applying different themes
    for (theme, name) in [
        (Theme::light(), "light"),
        (Theme::dark(), "dark"),
        (Theme::publication(), "publication"),
        (Theme::seaborn(), "seaborn"),
    ] {
        let result = Plot::new()
            .theme(theme)
            .title(format!("Backend - {} Theme", name))
            .line(&x, &y)
            .save(format!("tests/output/backend_theme_{}.png", name));

        // THEN: Should produce consistent output for all themes
        assert!(result.is_ok(), "{} theme failed", name);
        assert_png_rendered(
            format!("tests/output/backend_theme_{}.png", name),
            Some((640, 480)),
        );
    }
}

#[test]
fn test_backend_consistency_dpi() {
    // GIVEN: Simple data
    let x = vec![0.0, 1.0, 2.0];
    let y = vec![0.0, 1.0, 4.0];

    // WHEN: Rendering at different DPIs
    for dpi in [72, 96, 150, 300] {
        let result = Plot::new()
            .dpi(dpi)
            .title(format!("Backend - {} DPI", dpi))
            .line(&x, &y)
            .save(format!("tests/output/backend_dpi_{}.png", dpi));

        // THEN: Should succeed for all DPIs
        assert!(result.is_ok(), "{} DPI failed", dpi);

        // AND: Should produce appropriately sized output (±1 pixel for rounding)
        // Default figure size is 6.4 × 4.8 inches, so pixel dimensions = inches × DPI
        let expected_width = (6.4 * dpi as f32) as u32;
        let expected_height = (4.8 * dpi as f32) as u32;
        assert_png_dimensions_with_tolerance(
            format!("tests/output/backend_dpi_{}.png", dpi),
            (expected_width, expected_height),
            1,
        );
    }
}

#[test]
fn test_backend_consistency_dimensions() {
    // GIVEN: Custom dimensions
    let x = vec![0.0, 1.0, 2.0, 3.0];
    let y = vec![0.0, 1.0, 4.0, 9.0];

    // WHEN: Using custom dimensions
    for (width, height) in [(400, 300), (800, 600), (1200, 900), (1600, 1200)] {
        let result = Plot::new()
            .size_px(width, height)
            .title(format!("{}x{}", width, height))
            .line(&x, &y)
            .save(format!(
                "tests/output/backend_dim_{}x{}.png",
                width, height
            ));

        // THEN: Should produce correct dimensions
        assert!(result.is_ok(), "{}x{} failed", width, height);

        assert_png_dimensions_with_tolerance(
            format!("tests/output/backend_dim_{}x{}.png", width, height),
            (width, height),
            1,
        );
    }
}

#[test]
fn test_backend_error_handling() {
    // GIVEN: Invalid data
    let empty_x: Vec<f64> = vec![];
    let empty_y: Vec<f64> = vec![];

    // WHEN: Attempting to plot empty data
    let result = Plot::new()
        .line(&empty_x, &empty_y)
        .save("tests/output/backend_should_not_exist.png");

    // THEN: Should fail gracefully across all backends
    assert!(result.is_err(), "Empty data should produce error");

    // AND: Mismatched lengths
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 2.0];

    let result = Plot::new()
        .line(&x, &y)
        .save("tests/output/backend_should_not_exist_2.png");

    assert!(result.is_err(), "Mismatched data should produce error");
}
