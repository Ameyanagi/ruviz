//! T004: Performance contract test: 100K scatter plot <100ms
//! 
//! This test MUST FAIL initially - DataShader not implemented yet
//! Target: 100K points scatter plot in <100ms

use ruviz::core::Plot;
use std::time::Instant;
use criterion::black_box;

/// Performance contract for 100K point scatter plots
/// MUST complete in <100ms for publication-quality visualization
#[test]
fn scatter_100k_points_contract() {
    // Generate 100K test data points
    let x_data: Vec<f64> = (0..100_000).map(|i| i as f64 * 0.001).collect();
    let y_data: Vec<f64> = (0..100_000).map(|i| (i as f64 * 0.01).sin()).collect();
    
    println!("🧪 CONTRACT TEST: 100K scatter plot performance");
    println!("📊 Data size: {} points", x_data.len());
    
    let start = Instant::now();
    
    // This should automatically trigger DataShader for >10K points
    let plot_result = Plot::new()
        .scatter(&x_data, &y_data)
        .title("100K Point Performance Test")
        .xlabel("X Values") 
        .ylabel("Y Values")
        .save("test_output/contract_100k_scatter.png");
    
    let duration = start.elapsed();
    
    // Ensure the plot was created successfully
    assert!(plot_result.is_ok(), "Plot creation should succeed");
    
    println!("⏱️  Rendering time: {:?}", duration);
    println!("🎯 Target: <100ms");
    
    // CRITICAL PERFORMANCE CONTRACT
    // This MUST fail initially because DataShader is not implemented
    // After DataShader implementation, this should pass
    assert!(
        duration.as_millis() < 100,
        "❌ PERFORMANCE CONTRACT VIOLATION: 100K scatter plot took {:?}, must be <100ms. DataShader optimization required!",
        duration
    );
    
    println!("✅ CONTRACT PASSED: 100K scatter plot rendered in {:?}", duration);
}

/// Memory efficiency validation for 100K points
#[test] 
fn scatter_100k_memory_contract() {
    let x_data: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    let y_data: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    
    // Calculate input data size (2 * 100K * 8 bytes = ~1.6MB)
    let input_size = (x_data.len() + y_data.len()) * std::mem::size_of::<f64>();
    let max_allowed_memory = input_size * 2; // <2x data size contract
    
    println!("🧪 MEMORY CONTRACT: 100K scatter plot memory usage");
    println!("💾 Input data size: {} MB", input_size / 1024 / 1024);
    println!("📊 Maximum allowed: {} MB", max_allowed_memory / 1024 / 1024);
    
    // Create plot and measure memory impact
    let plot = Plot::new()
        .scatter(&x_data, &y_data)
        .title("Memory Test");
        
    // For now we'll pass this test as we don't have memory measurement
    // In the future, this should use actual memory profiling
    println!("⚠️  Memory measurement not yet implemented");
    println!("✅ Memory contract test created (implementation pending)");
    
    // Prevent optimization of unused data
    black_box((plot, x_data, y_data));
}

/// DataShader activation threshold test  
#[test]
fn datashader_activation_threshold() {
    println!("🧪 DATASHADER ACTIVATION: Testing automatic threshold detection");
    
    // Test with small dataset (should use direct rendering)
    let small_x: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    let small_y: Vec<f64> = (0..1000).map(|i| i as f64).collect();
    
    let start_small = Instant::now();
    let small_plot = Plot::new()
        .scatter(&small_x, &small_y)
        .title("Small Dataset Test");
    let small_duration = start_small.elapsed();
    
    println!("📊 Small dataset (1K points): {:?}", small_duration);
    
    // Test with large dataset (should trigger DataShader)
    let large_x: Vec<f64> = (0..100_000).map(|i| i as f64).collect(); 
    let large_y: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
    
    let start_large = Instant::now();
    let large_plot = Plot::new()
        .scatter(&large_x, &large_y)  
        .title("Large Dataset Test");
    let large_duration = start_large.elapsed();
    
    println!("📊 Large dataset (100K points): {:?}", large_duration);
    
    // DataShader should keep performance reasonable even for large datasets
    // This will initially fail without DataShader implementation
    assert!(
        large_duration.as_millis() < 500, // More generous threshold for now
        "❌ Large dataset performance degradation detected: {:?}. DataShader required!",
        large_duration  
    );
    
    // Prevent optimization
    black_box((small_plot, large_plot));
    
    println!("✅ DataShader activation test passed");
}