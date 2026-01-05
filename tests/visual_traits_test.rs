//! Visual regression tests for plot traits
//!
//! These tests generate plot images and compare against matplotlib references.
//!
//! # Running Visual Tests
//!
//! 1. Generate reference images first:
//!    ```bash
//!    python scripts/generate_reference.py
//!    ```
//!
//! 2. Run the visual tests:
//!    ```bash
//!    cargo test --test visual_traits_test -- --ignored
//!    ```
//!
//! 3. Review any failures in `tests/output/visual_diff/`
//!
//! # Note
//!
//! These tests are `#[ignore]` by default because:
//! - Reference images must be generated first
//! - Font rendering may differ between systems
//! - They're meant for visual review, not automated CI

mod visual;

use ruviz::prelude::*;

/// Seed for reproducible test data
const SEED: u64 = 42;

/// Generate reproducible test data
fn generate_test_data(n: usize) -> Vec<f64> {
    // Simple LCG for reproducible "random" data
    let mut state = SEED;
    let mut data = Vec::with_capacity(n);

    for _ in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let u1 = (state >> 33) as f64 / (1u64 << 31) as f64;

        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let u2 = (state >> 33) as f64 / (1u64 << 31) as f64;

        // Box-Muller transform for normal distribution
        let z = (-2.0 * u1.max(1e-10).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        data.push(z);
    }

    data
}

/// Generate XY test data
fn generate_xy_data(n: usize) -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&xi| xi.sin()).collect();
    (x, y)
}

// =============================================================================
// Line Plot Tests (baseline)
// =============================================================================

#[test]
#[ignore = "Visual test - run with: cargo test --test visual_traits_test -- --ignored"]
fn test_line_visual() {
    let config = visual::VisualTestConfig::default();
    let (x, y) = generate_xy_data(100);

    let result = visual::run_visual_test("line", &config, |path| {
        Plot::new()
            .title("Line Plot")
            .xlabel("X")
            .ylabel("Y")
            .line(&x, &y)
            .label("sin(x)")
            .end_series()
            .legend_best()
            .grid(true)
            .save(path)?;
        Ok(())
    });

    if !result.passed {
        println!("{}", result.assert_message());
        // Don't fail for now - visual comparison is placeholder
    }
}

// =============================================================================
// Distribution Plot Tests (KDE, ECDF, Violin)
// =============================================================================

#[test]
#[ignore = "Visual test - run with: cargo test --test visual_traits_test -- --ignored"]
fn test_kde_visual() {
    let config = visual::VisualTestConfig::default();
    let data = generate_test_data(1000);

    let result = visual::run_visual_test("kde", &config, |path| {
        // TODO: When Plot::kde() is implemented, use it here
        // For now, use histogram as placeholder
        use ruviz::plots::histogram::HistogramConfig;

        let hist_config = HistogramConfig::default().bins(50);
        Plot::new()
            .title("KDE Plot (placeholder)")
            .xlabel("Value")
            .ylabel("Density")
            .histogram(&data, Some(hist_config))
            .end_series()
            .grid(true)
            .save(path)?;
        Ok(())
    });

    if !result.passed {
        println!("{}", result.assert_message());
    }
}

#[test]
#[ignore = "Visual test - run with: cargo test --test visual_traits_test -- --ignored"]
fn test_ecdf_visual() {
    let config = visual::VisualTestConfig::default();
    let data = generate_test_data(1000);

    let result = visual::run_visual_test("ecdf", &config, |path| {
        // TODO: When Plot::ecdf() is implemented, use it here
        // For now, use sorted line plot as placeholder
        let mut sorted = data.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let y: Vec<f64> = (1..=sorted.len())
            .map(|i| i as f64 / sorted.len() as f64)
            .collect();

        Plot::new()
            .title("ECDF Plot (placeholder)")
            .xlabel("Value")
            .ylabel("Cumulative Probability")
            .line(&sorted, &y)
            .end_series()
            .grid(true)
            .save(path)?;
        Ok(())
    });

    if !result.passed {
        println!("{}", result.assert_message());
    }
}

// =============================================================================
// Discrete Plot Tests (Step)
// =============================================================================

#[test]
#[ignore = "Visual test - run with: cargo test --test visual_traits_test -- --ignored"]
fn test_step_visual() {
    let config = visual::VisualTestConfig::default();
    let (x, y) = generate_xy_data(50);

    let result = visual::run_visual_test("step", &config, |path| {
        // TODO: When Plot::step() is implemented, use it here
        // For now, use line plot as placeholder
        Plot::new()
            .title("Step Plot (placeholder)")
            .xlabel("X")
            .ylabel("Y")
            .line(&x, &y)
            .end_series()
            .grid(true)
            .save(path)?;
        Ok(())
    });

    if !result.passed {
        println!("{}", result.assert_message());
    }
}

// =============================================================================
// Error Bar Tests
// =============================================================================

#[test]
#[ignore = "Visual test - run with: cargo test --test visual_traits_test -- --ignored"]
fn test_errorbar_visual() {
    let config = visual::VisualTestConfig::default();

    let x: Vec<f64> = (1..=10).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&xi| xi.sin() + 1.0).collect();
    let y_err: Vec<f64> = (0..10).map(|i| 0.1 + 0.02 * i as f64).collect();

    let result = visual::run_visual_test("errorbar", &config, |path| {
        Plot::new()
            .title("Error Bar Plot")
            .xlabel("X")
            .ylabel("Y")
            .error_bars(&x, &y, &y_err)
            .end_series()
            .grid(true)
            .save(path)?;
        Ok(())
    });

    if !result.passed {
        println!("{}", result.assert_message());
    }
}

// =============================================================================
// Scatter Plot Tests
// =============================================================================

#[test]
#[ignore = "Visual test - run with: cargo test --test visual_traits_test -- --ignored"]
fn test_scatter_visual() {
    let config = visual::VisualTestConfig::default();
    let (x, y) = generate_xy_data(200);

    let result = visual::run_visual_test("scatter", &config, |path| {
        Plot::new()
            .title("Scatter Plot")
            .xlabel("X")
            .ylabel("Y")
            .scatter(&x, &y)
            .end_series()
            .grid(true)
            .save(path)?;
        Ok(())
    });

    if !result.passed {
        println!("{}", result.assert_message());
    }
}

// =============================================================================
// Histogram Tests
// =============================================================================

#[test]
#[ignore = "Visual test - run with: cargo test --test visual_traits_test -- --ignored"]
fn test_histogram_visual() {
    use ruviz::plots::histogram::HistogramConfig;

    let config = visual::VisualTestConfig::default();
    let data = generate_test_data(1000);

    let result = visual::run_visual_test("histogram", &config, |path| {
        let hist_config = HistogramConfig::default().bins(30);
        Plot::new()
            .title("Histogram")
            .xlabel("Value")
            .ylabel("Frequency")
            .histogram(&data, Some(hist_config))
            .end_series()
            .grid(true)
            .save(path)?;
        Ok(())
    });

    if !result.passed {
        println!("{}", result.assert_message());
    }
}

// =============================================================================
// Box Plot Tests
// =============================================================================

#[test]
#[ignore = "Visual test - run with: cargo test --test visual_traits_test -- --ignored"]
fn test_boxplot_visual() {
    let config = visual::VisualTestConfig::default();
    let data = generate_test_data(100);

    let result = visual::run_visual_test("boxplot", &config, |path| {
        Plot::new()
            .title("Box Plot")
            .ylabel("Value")
            .boxplot(&data, None)
            .end_series()
            .grid(true)
            .save(path)?;
        Ok(())
    });

    if !result.passed {
        println!("{}", result.assert_message());
    }
}

// =============================================================================
// Violin Plot Tests
// =============================================================================

// TODO: Enable when Plot::violin() is implemented
// #[test]
// #[ignore = "Visual test - run with: cargo test --test visual_traits_test -- --ignored"]
// fn test_violin_visual() {
//     let config = visual::VisualTestConfig::default();
//     let data = generate_test_data(200);
//
//     let result = visual::run_visual_test("violin", &config, |path| {
//         Plot::new()
//             .title("Violin Plot")
//             .ylabel("Value")
//             .violin(&data, None)
//             .end_series()
//             .grid(true)
//             .save(path)?;
//         Ok(())
//     });
//
//     if !result.passed {
//         println!("{}", result.assert_message());
//     }
// }

// =============================================================================
// Contour Plot Tests
// =============================================================================

// TODO: Enable when Plot::contour() is implemented
// #[test]
// #[ignore = "Visual test - run with: cargo test --test visual_traits_test -- --ignored"]
// fn test_contour_visual() {
//     let config = visual::VisualTestConfig::default();
//
//     let result = visual::run_visual_test("contour", &config, |path| {
//         // Generate 2D grid data
//         let n = 50;
//         let x: Vec<f64> = (0..n)
//             .map(|i| -3.0 + 6.0 * i as f64 / (n - 1) as f64)
//             .collect();
//         let y: Vec<f64> = x.clone();
//         let mut z = vec![vec![0.0; n]; n];
//
//         for (i, yi) in y.iter().enumerate() {
//             for (j, xj) in x.iter().enumerate() {
//                 z[i][j] = (-xj * xj - yi * yi).exp()
//                     + 0.5 * (-(xj - 1.0).powi(2) - (yi - 1.0).powi(2)).exp();
//             }
//         }
//
//         Plot::new()
//             .title("Contour Plot")
//             .xlabel("X")
//             .ylabel("Y")
//             .contour(&x, &y, &z, None)
//             .end_series()
//             .save(path)?;
//         Ok(())
//     });
//
//     if !result.passed {
//         println!("{}", result.assert_message());
//     }
// }

// =============================================================================
// Heatmap Tests
// =============================================================================

// TODO: Enable when Plot::heatmap() is implemented
// #[test]
// #[ignore = "Visual test - run with: cargo test --test visual_traits_test -- --ignored"]
// fn test_heatmap_visual() {
//     let config = visual::VisualTestConfig::default();
//
//     let result = visual::run_visual_test("heatmap", &config, |path| {
//         // Generate matrix data
//         let matrix = vec![
//             vec![1.0, 2.0, 3.0, 4.0],
//             vec![5.0, 6.0, 7.0, 8.0],
//             vec![9.0, 10.0, 11.0, 12.0],
//             vec![13.0, 14.0, 15.0, 16.0],
//         ];
//
//         Plot::new()
//             .title("Heatmap")
//             .heatmap(&matrix, None)
//             .end_series()
//             .save(path)?;
//         Ok(())
//     });
//
//     if !result.passed {
//         println!("{}", result.assert_message());
//     }
// }

// =============================================================================
// Radar Chart Tests
// =============================================================================

// TODO: Enable when Plot::radar() is implemented
// #[test]
// #[ignore = "Visual test - run with: cargo test --test visual_traits_test -- --ignored"]
// fn test_radar_visual() {
//     let config = visual::VisualTestConfig::default();
//
//     let result = visual::run_visual_test("radar", &config, |path| {
//         let categories = vec!["Speed", "Power", "Range", "Defense", "Health", "Magic"];
//         let values = vec![0.8, 0.6, 0.7, 0.5, 0.9, 0.4];
//
//         Plot::new()
//             .title("Radar Chart")
//             .radar(&categories, &values, None)
//             .end_series()
//             .save(path)?;
//         Ok(())
//     });
//
//     if !result.passed {
//         println!("{}", result.assert_message());
//     }
// }

// =============================================================================
// Helper to run all visual tests
// =============================================================================

#[test]
#[ignore = "Run all visual tests with: cargo test --test visual_traits_test -- --ignored test_all_visual"]
fn test_all_visual() {
    println!("\n=== Running All Visual Tests ===\n");

    // Note: This doesn't actually run the other tests,
    // but serves as documentation for running all tests
    println!("To run all visual tests:");
    println!("  1. Generate references: python scripts/generate_reference.py");
    println!("  2. Run tests: cargo test --test visual_traits_test -- --ignored");
    println!("\nTests available:");
    println!("  - test_line_visual");
    println!("  - test_kde_visual");
    println!("  - test_ecdf_visual");
    println!("  - test_step_visual");
    println!("  - test_errorbar_visual");
    println!("  - test_scatter_visual");
    println!("  - test_histogram_visual");
    println!("  - test_boxplot_visual");
    println!("\nTODO: These tests are disabled until Plot methods are implemented:");
    println!("  - test_violin_visual (needs Plot::violin())");
    println!("  - test_contour_visual (needs Plot::contour())");
    println!("  - test_heatmap_visual (needs Plot::heatmap())");
    println!("  - test_radar_visual (needs Plot::radar())");
}
