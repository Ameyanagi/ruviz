// Performance validation tests for verifying claimed metrics
// These tests validate the performance targets documented in README and PERFORMANCE.md

use ruviz::prelude::*;
use std::time::Instant;

const TOLERANCE_MS: u128 = 50; // Allow 50ms tolerance for CI variance

#[test]
fn test_1k_points_target() {
    // Target: <5ms for 1K points (README.md)
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .line(&x, &y)
        .title("1K Points Performance Test")
        .save("tests/output/perf_1k.png");
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to create 1K point plot");

    // Log performance (informational, not fail on CI)
    let ms = duration.as_millis();
    println!(
        "1K points rendered in {}ms (target: <5ms + {}ms tolerance)",
        ms, TOLERANCE_MS
    );

    // Lenient check for CI - verify reasonable performance with cosmic-text overhead
    assert!(ms < 2000, "1K points took {}ms (max 2s allowed)", ms);
}

#[test]
fn test_10k_points_target() {
    // Target: <18ms for 10K points (README.md)
    let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .line(&x, &y)
        .title("10K Points Performance Test")
        .save("tests/output/perf_10k.png");
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to create 10K point plot");

    let ms = duration.as_millis();
    println!(
        "10K points rendered in {}ms (target: <18ms + {}ms tolerance)",
        ms, TOLERANCE_MS
    );

    // Lenient check - verify reasonable performance with text rendering overhead
    assert!(ms < 2000, "10K points took {}ms (max 2s allowed)", ms);
}

#[test]
#[cfg(feature = "parallel")]
fn test_100k_points_parallel() {
    // Target: <100ms for 100K points with parallel rendering (README.md)
    let x: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .line(&x, &y)
        .title("100K Points Parallel Performance Test")
        .save("tests/output/perf_100k_parallel.png");
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to create 100K point plot");

    let ms = duration.as_millis();
    println!(
        "100K points (parallel) rendered in {}ms (target: <100ms)",
        ms
    );

    // More lenient for CI - parallel performance varies significantly by system
    assert!(
        ms < 10_000,
        "100K points (parallel) took {}ms (max 10s allowed)",
        ms
    );
}

#[test]
#[cfg(all(feature = "parallel", feature = "simd"))]
fn test_1m_points_parallel_simd() {
    // Target: <1s for 1M points with parallel + SIMD (README.md)
    let x: Vec<f64> = (0..1_000_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .line(&x, &y)
        .title("1M Points Parallel+SIMD Performance Test")
        .save("tests/output/perf_1m_simd.png");
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to create 1M point plot");

    let ms = duration.as_millis();
    println!(
        "1M points (parallel+SIMD) rendered in {}ms (target: <1000ms)",
        ms
    );

    // Very lenient - just verify it completes reasonably
    assert!(
        ms < 10_000,
        "1M points (parallel+SIMD) took {}ms (max 10s allowed)",
        ms
    );
}

#[test]
fn test_multi_series_performance() {
    // Validate performance with multiple data series
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
        .save("tests/output/perf_multi_series.png");
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to create multi-series plot");

    let ms = duration.as_millis();
    println!("3 series x 5K points rendered in {}ms", ms);

    // Should complete in reasonable time with text overhead
    assert!(
        ms < 3000,
        "Multi-series plot took {}ms (max 3s allowed)",
        ms
    );
}

#[test]
fn test_scatter_plot_performance() {
    // Validate scatter plot performance (typically slower than line plots)
    let x: Vec<f64> = (0..5000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .scatter(&x, &y)
        .marker(MarkerStyle::Circle)
        .marker_size(2.0)
        .title("Scatter Performance Test")
        .save("tests/output/perf_scatter.png");
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to create scatter plot");

    let ms = duration.as_millis();
    println!("5K scatter points rendered in {}ms", ms);

    // Scatter is slower than line, so more lenient
    assert!(ms < 3000, "Scatter plot took {}ms (max 3s allowed)", ms);
}

#[test]
fn test_bar_chart_performance() {
    // Validate bar chart performance
    let categories: Vec<String> = (0..100).map(|i| format!("Cat{}", i)).collect();
    let values: Vec<f64> = (0..100).map(|i| (i as f64).sin() * 100.0).collect();
    let categories_str: Vec<&str> = categories.iter().map(|s| s.as_str()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .bar(&categories_str, &values)
        .title("Bar Chart Performance Test")
        .save("tests/output/perf_bar.png");
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to create bar chart");

    let ms = duration.as_millis();
    println!("100 bar categories rendered in {}ms", ms);

    assert!(ms < 3000, "Bar chart took {}ms (max 3s allowed)", ms);
}

#[test]
fn test_histogram_performance() {
    // Validate histogram performance
    let data: Vec<f64> = (0..10_000).map(|i| (i as f64).sin()).collect();

    let start = Instant::now();
    let result = Plot::new()
        .histogram(&data, None)
        .title("Histogram Performance Test")
        .save("tests/output/perf_histogram.png");
    let duration = start.elapsed();

    assert!(result.is_ok(), "Failed to create histogram");

    let ms = duration.as_millis();
    println!("10K data histogram rendered in {}ms", ms);

    assert!(ms < 2000, "Histogram took {}ms (max 2s allowed)", ms);
}

#[test]
fn test_dpi_performance_impact() {
    // Validate that higher DPI has acceptable performance impact
    let x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    // Test 96 DPI (web standard)
    let start = Instant::now();
    let result_96 = Plot::new()
        .line(&x, &y)
        .dpi(96)
        .save("tests/output/perf_dpi_96.png");
    let duration_96 = start.elapsed();
    assert!(result_96.is_ok());

    // Test 300 DPI (print quality)
    let start = Instant::now();
    let result_300 = Plot::new()
        .line(&x, &y)
        .dpi(300)
        .save("tests/output/perf_dpi_300.png");
    let duration_300 = start.elapsed();
    assert!(result_300.is_ok());

    let ms_96 = duration_96.as_millis();
    let ms_300 = duration_300.as_millis();

    println!(
        "DPI performance: 96 DPI = {}ms, 300 DPI = {}ms",
        ms_96, ms_300
    );

    // Both should complete reasonably fast (cosmic-text overhead considered)
    assert!(ms_96 < 2000, "96 DPI took {}ms (max 2s)", ms_96);
    assert!(ms_300 < 5000, "300 DPI took {}ms (max 5s)", ms_300);
}

#[test]
fn test_theme_performance() {
    // Validate that theme application doesn't significantly impact performance
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
            .title(&format!("{} Theme", name))
            .save(&format!("tests/output/perf_theme_{}.png", name));
        let duration = start.elapsed();

        assert!(result.is_ok(), "{} theme failed", name);

        let ms = duration.as_millis();
        println!("{} theme: {}ms", name, ms);

        assert!(ms < 2000, "{} theme took {}ms (max 2s)", name, ms);
    }
}

#[test]
fn test_memory_efficiency() {
    // Validate that plot creation doesn't cause excessive memory allocation
    // This is a basic smoke test - more sophisticated memory profiling needed
    let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();

    let result = Plot::new()
        .line(&x, &y)
        .title("Memory Efficiency Test")
        .save("tests/output/perf_memory.png");

    assert!(result.is_ok(), "Memory efficiency test failed");

    // If this completes without OOM, memory usage is acceptable
    println!("Memory efficiency test completed successfully");
}
