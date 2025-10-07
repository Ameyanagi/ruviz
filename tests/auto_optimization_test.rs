// Auto-optimization tests - TDD approach
// Tests define expected backend selection behavior before implementation

use ruviz::prelude::*;

#[test]
fn test_small_dataset_uses_skia() {
    // GIVEN: Plot with <1K points
    let x: Vec<f64> = (0..500).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    let plot = Plot::new()
        .line(&x, &y)
        .auto_optimize();

    // THEN: Should select Skia backend
    assert_eq!(plot.get_backend_name(), "skia");
}

#[test]
fn test_medium_dataset_uses_parallel() {
    // GIVEN: Plot with 10K-100K points
    let x: Vec<f64> = (0..50_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    let plot = Plot::new()
        .line(&x, &y)
        .auto_optimize();

    // THEN: Should select Parallel backend
    assert_eq!(plot.get_backend_name(), "parallel");
}

#[test]
fn test_explicit_backend_not_overridden() {
    // GIVEN: Plot with explicit backend selection
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    // WHEN: Explicitly set Skia before auto_optimize
    let plot = Plot::new()
        .line(&x, &y)
        .backend(Backend::Skia)
        .auto_optimize();

    // THEN: Explicit selection should be respected
    assert_eq!(plot.get_backend_name(), "skia");
}

#[test]
fn test_auto_optimize_is_fluent() {
    // GIVEN: Fluent API usage
    let x = vec![1.0, 2.0, 3.0];
    let y = vec![1.0, 4.0, 9.0];

    // THEN: auto_optimize() should return Plot for chaining
    let result = Plot::new()
        .line(&x, &y)
        .auto_optimize()
        .title("Test")
        .save("test_output/auto_optimize_fluent.png");

    assert!(result.is_ok());
}

#[test]
fn test_very_small_dataset_optimization() {
    // GIVEN: Tiny dataset (< 100 points)
    let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![1.0, 4.0, 9.0, 16.0, 25.0];

    let plot = Plot::new()
        .line(&x, &y)
        .auto_optimize();

    // THEN: Should use simple Skia backend
    assert_eq!(plot.get_backend_name(), "skia");
}

#[test]
fn test_auto_optimize_with_multiple_series() {
    // GIVEN: Multiple data series
    let x = vec![1.0, 2.0, 3.0, 4.0];
    let y1 = vec![1.0, 4.0, 9.0, 16.0];
    let y2 = vec![2.0, 5.0, 10.0, 17.0];

    let plot = Plot::new()
        .line(&x, &y1)
        .line(&x, &y2)
        .auto_optimize();

    // THEN: Should work with multiple series
    assert_eq!(plot.get_backend_name(), "skia");
}

// Backend enum for testing
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Backend {
    Skia,
    Parallel,
    GPU,
    DataShader,
}
