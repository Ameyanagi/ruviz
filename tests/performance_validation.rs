// Performance smoke tests.
//
// These tests measure and print timings, but they intentionally avoid strict
// wall-clock assertions in debug `cargo test` runs. Export and rendering
// performance varies substantially with font-system warmup, PNG encoding, and
// machine-specific load.

use ruviz::prelude::*;
use std::time::Instant;

#[test]
fn test_1k_points_target() {
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .line(&x, &y)
        .title("1K Points Performance Test")
        .render();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to render 1K point plot");
    println!("1K points render() completed in {}ms", duration.as_millis());
}

#[test]
fn test_10k_points_target() {
    let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .line(&x, &y)
        .title("10K Points Performance Test")
        .render();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to render 10K point plot");
    println!(
        "10K points render() completed in {}ms",
        duration.as_millis()
    );
}

#[test]
#[ignore] // Slow test - run manually with `cargo test -- --ignored`
#[cfg(feature = "parallel")]
fn test_100k_points_parallel() {
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .line(&x, &y)
        .title("100K Points Parallel Performance Test")
        .render();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to render 100K point plot");
    println!(
        "100K points (parallel) render() completed in {}ms",
        duration.as_millis()
    );
}

#[test]
#[ignore] // Slow test - run manually with `cargo test -- --ignored`
#[cfg(all(feature = "parallel", feature = "simd"))]
fn test_1m_points_parallel_simd() {
    let x: Vec<f64> = (0..1_000_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .line(&x, &y)
        .title("1M Points Parallel+SIMD Performance Test")
        .render();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to render 1M point plot");
    println!(
        "1M points (parallel+SIMD) render() completed in {}ms",
        duration.as_millis()
    );
}

#[test]
fn test_multi_series_performance() {
    let x: Vec<f64> = (0..5000).map(|i| i as f64).collect();
    let y1: Vec<f64> = x.iter().map(|&x| x.sin()).collect();
    let y2: Vec<f64> = x.iter().map(|&x| x.cos()).collect();
    let y3: Vec<f64> = x.iter().map(|&x| (x * 0.5).tan()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .line(&x, &y1)
        .label("sin")
        .line(&x, &y2)
        .label("cos")
        .line(&x, &y3)
        .label("tan")
        .title("Multi-Series Performance Test")
        .render();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to render multi-series plot");
    println!(
        "3 series x 5K points render() completed in {}ms",
        duration.as_millis()
    );
}

#[test]
fn test_scatter_plot_performance() {
    let x: Vec<f64> = (0..5000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .scatter(&x, &y)
        .marker(MarkerStyle::Circle)
        .marker_size(2.0)
        .title("Scatter Performance Test")
        .render();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to render scatter plot");
    println!(
        "5K scatter points render() completed in {}ms",
        duration.as_millis()
    );
}

#[test]
fn test_bar_chart_performance() {
    let categories: Vec<String> = (0..100).map(|i| format!("Cat{}", i)).collect();
    let values: Vec<f64> = (0..100).map(|i| (i as f64).sin() * 100.0).collect();
    let categories_str: Vec<&str> = categories.iter().map(|s| s.as_str()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .bar(&categories_str, &values)
        .title("Bar Chart Performance Test")
        .render();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to render bar chart");
    println!(
        "100 bar categories render() completed in {}ms",
        duration.as_millis()
    );
}

#[test]
fn test_histogram_performance() {
    let data: Vec<f64> = (0..10_000).map(|i| (i as f64).sin()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .histogram(&data, None)
        .title("Histogram Performance Test")
        .render();
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to render histogram");
    println!(
        "10K data histogram render() completed in {}ms",
        duration.as_millis()
    );
}

#[test]
fn test_dpi_performance_impact() {
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let start = Instant::now();
    let result_96 = Plot::new().line(&x, &y).dpi(96).render();
    let duration_96 = start.elapsed();
    assert!(result_96.is_ok());

    let start = Instant::now();
    let result_300 = Plot::new().line(&x, &y).dpi(300).render();
    let duration_300 = start.elapsed();
    assert!(result_300.is_ok());

    let ms_96 = duration_96.as_millis();
    let ms_300 = duration_300.as_millis();

    println!(
        "DPI render() timing: 96 DPI = {}ms, 300 DPI = {}ms",
        ms_96, ms_300
    );
}

#[test]
fn test_theme_performance() {
    let x: Vec<f64> = (0..5000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    for (theme, name) in [
        (Theme::light(), "light"),
        (Theme::dark(), "dark"),
        (Theme::publication(), "publication"),
        (Theme::seaborn(), "seaborn"),
    ] {
        let start = Instant::now();
        let result = Plot::new()
            .theme(theme)
            .line(&x, &y)
            .title(format!("{} Theme", name))
            .render();
        let duration = start.elapsed();

        assert!(result.is_ok(), "{} theme failed", name);
        println!(
            "{} theme render() completed in {}ms",
            name,
            duration.as_millis()
        );
    }
}

#[test]
fn test_memory_efficiency() {
    let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let result = Plot::new()
        .line(&x, &y)
        .title("Memory Efficiency Test")
        .render();

    assert!(result.is_ok(), "Memory efficiency test failed");
    println!("Memory efficiency render() smoke test completed successfully");
}
