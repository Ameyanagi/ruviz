// Manual small-dataset benchmark tests.
//
// These are intentionally ignored by default and do not enforce strict
// machine-dependent timing thresholds. They are used to print observed timings
// while still asserting that optimized render paths succeed.

use ruviz::prelude::*;
use std::time::Instant;

fn warmup_line<X, Y>(x: &X, y: &Y)
where
    X: ruviz::data::NumericData1D,
    Y: ruviz::data::NumericData1D,
{
    Plot::new()
        .line(x, y)
        .render()
        .expect("Warmup render failed");
}

#[test]
#[ignore] // Performance test - run manually on local machine
fn test_small_dataset_under_10ms() {
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    warmup_line(&x, &y);

    let start = Instant::now();
    let result = Plot::new().line(&x, &y).render();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to render small dataset");
    println!("1K point manual benchmark: {:?}", duration);
}

#[test]
#[ignore] // Performance test - run manually on local machine
fn test_very_small_dataset_under_5ms() {
    let x: Vec<f64> = (0..100).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    warmup_line(&x, &y);

    let start = Instant::now();
    let result = Plot::new().line(&x, &y).render();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to render very small dataset");
    println!("100 point manual benchmark: {:?}", duration);
}

#[test]
#[ignore] // Performance test - run manually on local machine
fn test_medium_dataset_under_20ms() {
    let x: Vec<f64> = (0..5000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    warmup_line(&x, &y);

    let start = Instant::now();
    let result = Plot::new().line(&x, &y).render();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to render medium dataset");
    println!("5K point manual benchmark: {:?}", duration);
}

#[test]
#[ignore] // Performance test - run manually on local machine
fn test_no_regression_large_datasets() {
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    let start = Instant::now();
    let result = Plot::new().line(&x, &y).auto_optimize().render();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to render large dataset");
    println!("100K point auto_optimize manual benchmark: {:?}", duration);
}

#[test]
#[ignore] // Performance test - run manually on local machine
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
#[ignore] // Performance test - run manually on local machine
fn test_multiple_small_plots_efficient() {
    let datasets: Vec<(Vec<f64>, Vec<f64>)> = (0..10)
        .map(|_| {
            let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
            let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();
            (x, y)
        })
        .collect();

    let start = Instant::now();
    for (i, (x, y)) in datasets.iter().enumerate() {
        let result = Plot::new()
            .line(x, y)
            .title(format!("Batch Plot {i}"))
            .render();
        assert!(result.is_ok(), "Failed to render batched plot {i}");
    }
    let total_duration = start.elapsed();

    let avg_per_plot = total_duration / datasets.len() as u32;
    println!(
        "Average per small plot manual benchmark: {:?}",
        avg_per_plot
    );
}

#[test]
#[ignore] // Performance test - run manually on local machine
fn test_small_dataset_with_styling() {
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();

    warmup_line(&x, &y);

    let start = Instant::now();
    let result = Plot::new()
        .line(&x, &y)
        .title("Small Dataset Test")
        .xlabel("X Axis")
        .ylabel("Y Axis")
        .render();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to render styled small dataset");
    println!("Styled 1K point manual benchmark: {:?}", duration);
}
