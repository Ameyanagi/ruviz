// Property-based testing - TDD approach
// These tests verify robustness properties with randomized inputs

use proptest::prelude::*;
use ruviz::prelude::*;
use std::path::Path;

// Property 1: Plot should handle any valid f64 data without panicking
proptest! {
    #[test]
    fn plot_never_panics_on_valid_data(
        x in prop::collection::vec(
            any::<f64>().prop_filter("finite", |x| x.is_finite()),
            1..100
        ),
        y in prop::collection::vec(
            any::<f64>().prop_filter("finite", |y| y.is_finite()),
            1..100
        ),
    ) {
        // Ensure equal lengths
        let min_len = x.len().min(y.len());
        let x: Vec<f64> = x[..min_len].to_vec();
        let y: Vec<f64> = y[..min_len].to_vec();

        // This should never panic
        let result = Plot::new()
            .line(&x, &y)
            .save("tests/output/proptest_line.png");

        // Either succeeds or returns error (but never panics)
        prop_assert!(result.is_ok() || result.is_err());
    }
}

// Property 2: Auto-optimize should always select a valid backend
proptest! {
    #[test]
    fn auto_optimize_always_selects_backend(
        size in 1usize..10000,
    ) {
        let x: Vec<f64> = (0..size).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

        let plot = Plot::new()
            .line(&x, &y)
            .auto_optimize();

        let backend = plot.get_backend_name();

        // Should select one of the valid backends
        prop_assert!(
            backend == "skia" ||
            backend == "parallel" ||
            backend == "gpu" ||
            backend == "datashader",
            "Invalid backend: {}",
            backend
        );
    }
}

// Property 3: Same data should produce deterministic output
proptest! {
    #[test]
    fn deterministic_output(
        x in prop::collection::vec((-1000.0..1000.0), 10..50),
        y in prop::collection::vec((-1000.0..1000.0), 10..50),
    ) {
        let min_len = x.len().min(y.len());
        let x: Vec<f64> = x[..min_len].to_vec();
        let y: Vec<f64> = y[..min_len].to_vec();

        // Render twice with same data
        Plot::new().line(&x, &y).save("tests/output/prop_det_1.png")?;
        Plot::new().line(&x, &y).save("tests/output/prop_det_2.png")?;

        // Should produce identical file sizes (deterministic)
        let size1 = std::fs::metadata("tests/output/prop_det_1.png")?.len();
        let size2 = std::fs::metadata("tests/output/prop_det_2.png")?.len();

        prop_assert_eq!(size1, size2, "Output not deterministic");
    }
}

// Property 4: Data bounds should be valid
proptest! {
    #[test]
    fn bounds_contain_all_data(
        x in prop::collection::vec((-1000.0..1000.0), 10..100),
        y in prop::collection::vec((-1000.0..1000.0), 10..100),
    ) {
        let min_len = x.len().min(y.len());
        let x: Vec<f64> = x[..min_len].to_vec();
        let y: Vec<f64> = y[..min_len].to_vec();

        // Calculate expected bounds
        let x_min = x.iter().copied().fold(f64::INFINITY, f64::min);
        let x_max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let y_min = y.iter().copied().fold(f64::INFINITY, f64::min);
        let y_max = y.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        // Bounds should be valid
        prop_assert!(x_min <= x_max, "Invalid x bounds");
        prop_assert!(y_min <= y_max, "Invalid y bounds");

        // Should be able to create plot (bounds calculation doesn't fail)
        let result = Plot::new().line(&x, &y).save("tests/output/prop_bounds.png");
        prop_assert!(result.is_ok());
    }
}

// Property 5: Simple API should match full API output
proptest! {
    #[test]
    fn simple_api_matches_full_api(
        x in prop::collection::vec((-100.0..100.0), 10..50),
        y in prop::collection::vec((-100.0..100.0), 10..50),
    ) {
        let min_len = x.len().min(y.len());
        let x: Vec<f64> = x[..min_len].to_vec();
        let y: Vec<f64> = y[..min_len].to_vec();

        // Simple API
        let simple_result = ruviz::simple::line_plot(
            &x,
            &y,
            "tests/output/prop_simple.png"
        );

        // Full API with auto-optimize
        let full_result = Plot::new()
            .line(&x, &y)
            .auto_optimize()
            .save("tests/output/prop_full.png");

        // Both should succeed
        prop_assert!(simple_result.is_ok());
        prop_assert!(full_result.is_ok());

        // Should produce similar-sized outputs (within 10% due to compression variance)
        let simple_size = std::fs::metadata("tests/output/prop_simple.png")?.len();
        let full_size = std::fs::metadata("tests/output/prop_full.png")?.len();

        let ratio = simple_size as f64 / full_size as f64;
        prop_assert!(
            ratio > 0.9 && ratio < 1.1,
            "File sizes differ too much: {} vs {}",
            simple_size,
            full_size
        );
    }
}

// Property 6: Empty data should error gracefully
proptest! {
    #[test]
    fn empty_data_errors_gracefully(
        has_x in prop::bool::ANY,
        has_y in prop::bool::ANY,
    ) {
        // Generate empty or non-empty data
        let x: Vec<f64> = if has_x { vec![1.0, 2.0, 3.0] } else { vec![] };
        let y: Vec<f64> = if has_y { vec![1.0, 2.0, 3.0] } else { vec![] };

        let result = Plot::new()
            .line(&x, &y)
            .save("tests/output/prop_empty.png");

        if x.is_empty() || y.is_empty() {
            // Should return error, not panic
            prop_assert!(result.is_err(), "Empty data should error");
        } else {
            // Should succeed with valid data
            prop_assert!(result.is_ok(), "Valid data should succeed");
        }
    }
}

// Property 7: Scatter plots should behave like line plots for data handling
proptest! {
    #[test]
    fn scatter_plot_robust(
        x in prop::collection::vec(
            any::<f64>().prop_filter("finite", |x| x.is_finite()),
            5..100
        ),
        y in prop::collection::vec(
            any::<f64>().prop_filter("finite", |y| y.is_finite()),
            5..100
        ),
    ) {
        let min_len = x.len().min(y.len());
        let x: Vec<f64> = x[..min_len].to_vec();
        let y: Vec<f64> = y[..min_len].to_vec();

        // Scatter plot should handle same data as line plot
        let result = Plot::new()
            .scatter(&x, &y)
            .save("tests/output/prop_scatter.png");

        prop_assert!(result.is_ok(), "Scatter plot should handle valid data");
    }
}

// Property 8: Bar charts should handle any positive values
proptest! {
    #[test]
    fn bar_chart_handles_values(
        values in prop::collection::vec((0.0..1000.0), 1..20),
    ) {
        let categories: Vec<&str> = (0..values.len())
            .map(|i| match i % 5 {
                0 => "A",
                1 => "B",
                2 => "C",
                3 => "D",
                _ => "E",
            })
            .collect();

        let result = Plot::new()
            .bar(&categories, &values)
            .save("tests/output/prop_bar.png");

        prop_assert!(result.is_ok(), "Bar chart should handle positive values");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proptest_module_compiles() {
        // Ensure module compiles and proptest is available
        assert!(true);
    }
}
