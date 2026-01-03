use ruviz::core::Plot;
use std::time::Instant;

#[test]
fn test_parallel_rendering_basic() {
    // Test parallel rendering with medium dataset
    let n = 50_000; // Below DataShader threshold but enough for parallel benefits
    let x_data: Vec<f64> = (0..n).map(|i| (i as f64) / 1000.0).collect();
    let y_data: Vec<f64> = x_data
        .iter()
        .map(|&x| x.sin() + 0.5 * (x * 5.0).cos())
        .collect();

    let start = Instant::now();

    let result = Plot::new()
        .title("Parallel Rendering Test".to_string())
        .line(&x_data, &y_data)
        .render();

    let duration = start.elapsed();

    // Should not panic and should succeed
    assert!(
        result.is_ok(),
        "Plot with parallel rendering should succeed"
    );

    let image = result.unwrap();
    assert!(image.width > 0);
    assert!(image.height > 0);
    assert!(image.pixels.len() > 0);

    println!("✅ Parallel rendering test passed");
    println!(
        "   - Rendered {} points in {:.2}ms",
        n,
        duration.as_millis()
    );
    println!("   - Image dimensions: {}x{}", image.width, image.height);
}

#[test]
fn test_parallel_vs_sequential_performance() {
    // Test that demonstrates parallel processing capability
    let n = 25_000;
    let x_data: Vec<f64> = (0..n).map(|i| (i as f64) * 0.001).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| (x * 3.14159).sin()).collect();

    // Test with parallel enabled (default)
    let start_parallel = Instant::now();
    let parallel_result = Plot::new()
        .title("Parallel Test".to_string())
        .scatter(&x_data, &y_data)
        .render();
    let parallel_time = start_parallel.elapsed();

    assert!(parallel_result.is_ok(), "Parallel rendering should succeed");

    println!("✅ Parallel vs Sequential performance test");
    println!(
        "   - Parallel rendering: {:.2}ms",
        parallel_time.as_millis()
    );
    println!("   - Points processed: {}", n);

    // Basic performance validation - should be reasonable for this size
    assert!(
        parallel_time.as_millis() < 1000,
        "Rendering should complete in reasonable time"
    );
}

#[cfg(feature = "parallel")]
#[test]
fn test_parallel_thread_configuration() {
    use ruviz::core::Plot;

    // Test Plot with custom parallel configuration
    let n = 30_000;
    let x_data: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| (x * 0.01).tan()).collect();

    let result = Plot::new()
        .title("Custom Parallel Config Test".to_string())
        .with_parallel(Some(4)) // Explicitly use 4 threads
        .parallel_threshold(10_000) // Lower threshold for testing
        .line(&x_data, &y_data)
        .render();

    assert!(
        result.is_ok(),
        "Plot with custom parallel config should work"
    );

    let image = result.unwrap();
    assert!(image.width > 0);
    assert!(image.height > 0);

    println!("✅ Parallel thread configuration test passed");
    println!("   - Used custom 4-thread configuration");
    println!("   - Processed {} points successfully", n);
}
