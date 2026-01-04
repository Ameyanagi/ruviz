// Small dataset optimization tests - TDD approach
// These tests define expected performance after optimization

use ruviz::prelude::*;
use std::time::{Duration, Instant};

#[test]
fn test_small_dataset_under_10ms() {
    // GIVEN: 1K points (small dataset)
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    // Warmup
    Plot::new()
        .line(&x, &y)
        .save("tests/output/opt_warmup.png")
        .expect("Warmup failed");

    // WHEN: Render with optimized path
    let start = Instant::now();
    Plot::new()
        .line(&x, &y)
        .save("tests/output/opt_small_1k.png")
        .expect("Failed to save plot");
    let duration = start.elapsed();

    // THEN: Should complete in < 10ms
    assert!(
        duration < Duration::from_millis(10),
        "Small dataset took {:?}, target < 10ms (current: {}x slower)",
        duration,
        duration.as_millis() as f64 / 10.0
    );
}

#[test]
fn test_very_small_dataset_under_5ms() {
    // GIVEN: 100 points (very small)
    let x: Vec<f64> = (0..100).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    // Warmup
    Plot::new()
        .line(&x, &y)
        .save("tests/output/opt_warmup_tiny.png")
        .expect("Warmup failed");

    // WHEN: Render
    let start = Instant::now();
    Plot::new()
        .line(&x, &y)
        .save("tests/output/opt_very_small_100.png")
        .expect("Failed to save plot");
    let duration = start.elapsed();

    // THEN: Should complete in < 5ms
    assert!(
        duration < Duration::from_millis(5),
        "Very small dataset took {:?}, target < 5ms",
        duration
    );
}

#[test]
fn test_medium_dataset_under_20ms() {
    // GIVEN: 5K points (medium dataset)
    let x: Vec<f64> = (0..5000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    // Warmup
    Plot::new()
        .line(&x, &y)
        .save("tests/output/opt_warmup_med.png")
        .expect("Warmup failed");

    // WHEN: Render
    let start = Instant::now();
    Plot::new()
        .line(&x, &y)
        .save("tests/output/opt_medium_5k.png")
        .expect("Failed to save plot");
    let duration = start.elapsed();

    // THEN: Should complete in < 20ms
    assert!(
        duration < Duration::from_millis(20),
        "Medium dataset took {:?}, target < 20ms",
        duration
    );
}

#[test]
fn test_no_regression_large_datasets() {
    // GIVEN: 100K points (large dataset)
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    // WHEN: Render with auto-optimization
    let start = Instant::now();
    Plot::new()
        .line(&x, &y)
        .auto_optimize()
        .save("tests/output/opt_large_100k.png")
        .expect("Failed to save plot");
    let duration = start.elapsed();

    // THEN: Should still be fast (< 40ms, allowing some overhead)
    assert!(
        duration < Duration::from_millis(40),
        "Large dataset regression: {:?} (should be < 40ms)",
        duration
    );
}

#[test]
fn test_optimization_consistent_output() {
    // GIVEN: Same dataset
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    // WHEN: Render twice
    Plot::new()
        .line(&x, &y)
        .title("Output Test 1")
        .save("tests/output/opt_consistency_1.png")
        .expect("Failed first render");

    Plot::new()
        .line(&x, &y)
        .title("Output Test 2")
        .save("tests/output/opt_consistency_2.png")
        .expect("Failed second render");

    // THEN: Both files should exist (visual comparison would be manual)
    assert!(std::path::Path::new("tests/output/opt_consistency_1.png").exists());
    assert!(std::path::Path::new("tests/output/opt_consistency_2.png").exists());
}

#[test]
fn test_multiple_small_plots_efficient() {
    // GIVEN: Multiple small plots
    let datasets: Vec<(Vec<f64>, Vec<f64>)> = (0..10)
        .map(|_| {
            let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
            let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();
            (x, y)
        })
        .collect();

    // WHEN: Render all plots
    let start = Instant::now();
    for (i, (x, y)) in datasets.iter().enumerate() {
        Plot::new()
            .line(x, y)
            .save(format!("tests/output/opt_multi_{}.png", i))
            .expect("Failed to save plot");
    }
    let total_duration = start.elapsed();

    // THEN: Average should be < 10ms per plot
    let avg_per_plot = total_duration / datasets.len() as u32;
    assert!(
        avg_per_plot < Duration::from_millis(10),
        "Average per plot: {:?}, target < 10ms",
        avg_per_plot
    );
}

#[test]
fn test_small_dataset_with_styling() {
    // GIVEN: 1K points with styling
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    // Warmup
    Plot::new()
        .line(&x, &y)
        .save("tests/output/opt_warmup_style.png")
        .expect("Warmup failed");

    // WHEN: Render with title and labels
    let start = Instant::now();
    Plot::new()
        .line(&x, &y)
        .title("Small Dataset Test")
        .xlabel("X Axis")
        .ylabel("Y Axis")
        .save("tests/output/opt_small_styled.png")
        .expect("Failed to save plot");
    let duration = start.elapsed();

    // THEN: Should still complete in < 15ms (allowing text overhead)
    assert!(
        duration < Duration::from_millis(15),
        "Styled small dataset took {:?}, target < 15ms",
        duration
    );
}
