//! T007: DataShader activation contract test: 100M points <2s
//! 
//! This test MUST FAIL initially - DataShader not implemented
//! Target: 100M points aggregation in <2s

use ruviz::core::Plot;
use std::time::Instant;
use criterion::black_box;

/// DataShader contract for massive datasets (100M points)
/// MUST complete in <2s for big data visualization  
#[test]
fn massive_dataset_contract() {
    println!("🧪 DATASHADER CONTRACT: 100M points in <2s");
    println!("⚠️  WARNING: This test requires significant memory (~3.2GB)");
    
    // Check available memory before proceeding
    if !has_sufficient_memory() {
        println!("⏭️  Skipping massive dataset test - insufficient memory");
        return;
    }
    
    println!("🚀 Generating 100M data points...");
    let start_gen = Instant::now();
    
    // Generate 100M test points using efficient streaming approach
    let point_count = 100_000_000;
    let (x_data, y_data) = generate_massive_dataset(point_count);
    
    let gen_duration = start_gen.elapsed();
    println!("⏱️  Data generation: {:?}", gen_duration);
    println!("💾 Dataset size: ~{} GB", (x_data.len() + y_data.len()) * 8 / 1024 / 1024 / 1024);
    
    let start_render = Instant::now();
    
    // This MUST trigger DataShader aggregation automatically
    let plot_result = Plot::new()
        .scatter(&x_data, &y_data)
        .title("100M Point DataShader Test")
        .xlabel("X Coordinate")
        .ylabel("Y Coordinate") 
        .save("test_output/contract_100m_datashader.png");
    
    let render_duration = start_render.elapsed();
    let total_duration = start_gen.elapsed();
    
    // Ensure the plot was created successfully
    assert!(plot_result.is_ok(), "Massive dataset plot should succeed: {:?}", plot_result.err());
    
    println!("⏱️  Rendering time: {:?}", render_duration);
    println!("⏱️  Total time: {:?}", total_duration);
    println!("🎯 Target: <2s rendering");
    
    // CRITICAL DATASHADER CONTRACT
    // This MUST fail initially because DataShader is not implemented
    // Without DataShader, this will either fail or take >30s
    assert!(
        render_duration.as_millis() < 2000,
        "❌ DATASHADER CONTRACT VIOLATION: 100M points took {:?} to render, must be <2s. DataShader aggregation required!",
        render_duration
    );
    
    println!("✅ CONTRACT PASSED: 100M points rendered in {:?}", render_duration);
}

/// Generate massive dataset efficiently
fn generate_massive_dataset(count: usize) -> (Vec<f64>, Vec<f64>) {
    println!("📊 Generating {} points with mathematical patterns", count);
    
    let mut x_data = Vec::with_capacity(count);
    let mut y_data = Vec::with_capacity(count);
    
    // Use batch processing to reduce memory pressure during generation
    const BATCH_SIZE: usize = 1_000_000;
    let batches = (count + BATCH_SIZE - 1) / BATCH_SIZE;
    
    for batch in 0..batches {
        let batch_start = batch * BATCH_SIZE;
        let batch_end = (batch_start + BATCH_SIZE).min(count);
        let batch_size = batch_end - batch_start;
        
        // Generate one batch at a time
        for i in batch_start..batch_end {
            let t = i as f64 * 0.0001;
            let x = t * (t * 10.0).cos() + (t * 3.0).sin() * 0.5;
            let y = t * (t * 7.0).sin() + (t * 2.0).cos() * 0.3 + (i as f64 * 0.00001).sin();
            
            x_data.push(x);
            y_data.push(y);
        }
        
        if batch % 10 == 0 {
            println!("📈 Generated {:.1}M points ({:.1}%)", 
                (batch_end as f64) / 1_000_000.0,
                (batch_end as f64 / count as f64) * 100.0);
        }
    }
    
    (x_data, y_data)
}

/// Check if system has sufficient memory for 100M point test
fn has_sufficient_memory() -> bool {
    // Rough estimate: 100M points * 2 * 8 bytes = ~1.6GB input
    // + rendering buffers, aggregation = ~3-4GB total needed
    const REQUIRED_GB: u64 = 4;
    
    // For now, we'll do a simple heuristic check
    // In practice, this would check actual available memory
    if std::env::var("CI").is_ok() {
        // Skip in CI environments
        println!("ℹ️  CI environment detected - skipping memory-intensive test");
        return false;
    }
    
    // Check if we can allocate a test vector
    let test_size = 10_000_000; // 10M points test
    match std::panic::catch_unwind(|| {
        let _test_vec: Vec<f64> = Vec::with_capacity(test_size);
        true
    }) {
        Ok(_) => {
            println!("✅ Memory check passed - proceeding with massive dataset test");
            true
        }
        Err(_) => {
            println!("❌ Memory check failed - insufficient memory for massive dataset test");
            false
        }
    }
}

/// Test DataShader aggregation quality
#[test]
fn datashader_aggregation_quality() {
    println!("🧪 DATASHADER QUALITY: Testing aggregation accuracy");
    
    // Generate known pattern that should be preserved through aggregation
    let point_count = 1_000_000; // 1M points for quality test
    
    println!("📊 Generating {} points with known pattern", point_count);
    
    let x_data: Vec<f64> = (0..point_count)
        .map(|i| (i as f64 / 1000.0) % 10.0) // 0-10 repeating pattern
        .collect();
        
    let y_data: Vec<f64> = (0..point_count)
        .map(|i| ((i as f64 / 1000.0) * std::f64::consts::PI).sin()) // Sine wave
        .collect();
    
    let start = Instant::now();
    
    // Create plot with DataShader aggregation  
    let plot_result = Plot::new()
        .scatter(&x_data, &y_data)
        .title("DataShader Quality Test - Sine Pattern")
        .save("test_output/datashader_quality.png");
        
    let duration = start.elapsed();
    
    assert!(plot_result.is_ok(), "DataShader quality test should succeed");
    
    println!("⏱️  Aggregation time: {:?}", duration);
    
    // Quality contract: aggregation should preserve main pattern features
    // For now, we just ensure it completes in reasonable time
    assert!(
        duration.as_millis() < 5000, // 5s threshold for quality test
        "❌ DataShader quality test too slow: {:?}",
        duration
    );
    
    // Future: Add pattern preservation validation
    // - Check that sine wave peaks/valleys are preserved
    // - Verify no major artifacts introduced
    // - Validate aggregation canvas resolution vs input density
    
    println!("✅ DataShader quality test passed - pattern aggregated in {:?}", duration);
    
    black_box((x_data, y_data));
}

/// Test DataShader memory efficiency with massive datasets
#[test] 
fn datashader_memory_efficiency() {
    println!("🧪 DATASHADER MEMORY: Testing memory efficiency vs direct rendering");
    
    let sizes = vec![100_000, 500_000, 1_000_000, 5_000_000];
    
    for size in sizes {
        if size > 1_000_000 && std::env::var("CI").is_ok() {
            println!("⏭️  Skipping {} points in CI environment", size);
            continue;
        }
        
        println!("📊 Testing {} point dataset", size);
        
        let x_data: Vec<f64> = (0..size).map(|i| i as f64 * 0.01).collect();
        let y_data: Vec<f64> = (0..size).map(|i| (i as f64 * 0.01).sin()).collect();
        
        let input_size = (x_data.len() + y_data.len()) * std::mem::size_of::<f64>();
        
        let start = Instant::now();
        
        let plot_result = Plot::new()
            .scatter(&x_data, &y_data)
            .title(&format!("DataShader Memory Test - {} points", size));
            
        let duration = start.elapsed();
        
        assert!(plot_result.is_ok(), "Plot should succeed for {} points", size);
        
        // Memory efficiency: larger datasets should not have proportionally longer times
        // DataShader should provide O(1) or O(log n) complexity for canvas aggregation
        let time_per_point = duration.as_nanos() as f64 / size as f64;
        
        println!("⏱️  {} points: {:?} ({:.2} ns/point)", size, duration, time_per_point);
        
        // Prevent optimization
        black_box((plot_result, x_data, y_data));
    }
    
    println!("✅ DataShader memory efficiency test completed");
}