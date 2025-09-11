use std::time::Instant;
use ruviz::render::pooled::PooledRenderer;
use ruviz::data::Data1D;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Memory Pool Performance Test (Release Mode)");
    println!("================================================");
    
    let renderer = PooledRenderer::new();
    let test_sizes = vec![1_000, 5_000, 10_000, 50_000, 100_000];
    
    println!("\n📊 Coordinate Transformation Performance:");
    println!("{:<12} {:<15} {:<15} {:<12} {:<10}", "Size", "Pooled (μs)", "Traditional (μs)", "Difference", "Winner");
    println!("{:-<70}", "");
    
    for &size in &test_sizes {
        // Generate test data
        let test_data: Vec<f64> = (0..size).map(|i| (i as f64) * 0.1 + (i as f64 * 0.02).sin()).collect();
        
        // Test pooled rendering (10 iterations)
        let start = Instant::now();
        for _ in 0..10 {
            let _result = renderer.transform_x_coordinates_pooled(&test_data, 0.0, size as f64 * 0.1, 0.0, 800.0)?;
        }
        let pooled_time = start.elapsed().as_micros() / 10; // Average per iteration
        
        // Test traditional rendering (10 iterations)
        let start = Instant::now();
        for _ in 0..10 {
            let mut result = Vec::with_capacity(test_data.len());
            let range = size as f64 * 0.1;
            let scale = 800.0 / range;
            
            for &x in &test_data {
                result.push((x * scale) as f32);
            }
        }
        let traditional_time = start.elapsed().as_micros() / 10; // Average per iteration
        
        // Calculate difference and winner
        let diff_percent = if pooled_time > traditional_time {
            format!("+{:.1}%", ((pooled_time as f32 / traditional_time as f32) - 1.0) * 100.0)
        } else {
            format!("-{:.1}%", ((traditional_time as f32 / pooled_time as f32) - 1.0) * 100.0)
        };
        
        let winner = if pooled_time < traditional_time { "Pooled ✅" } else { "Traditional ✅" };
        
        println!("{:<12} {:<15} {:<15} {:<12} {:<10}", 
                 size.to_string(), 
                 pooled_time, 
                 traditional_time, 
                 diff_percent,
                 winner);
    }
    
    println!("\n🔄 Memory Reuse Pattern Test (50K elements, 20 iterations):");
    let large_data: Vec<f64> = (0..50_000).map(|i| (i as f64) * 0.1 + (i as f64 * 0.02).sin()).collect();
    
    // Test pooled memory reuse
    let start = Instant::now();
    for _ in 0..20 {
        let _result = renderer.transform_x_coordinates_pooled(&large_data, 0.0, 5000.0, 0.0, 800.0)?;
    }
    let pooled_reuse_time = start.elapsed();
    
    // Test traditional allocation
    let start = Instant::now();
    for _ in 0..20 {
        let mut result = Vec::with_capacity(large_data.len());
        let scale = 800.0 / 5000.0;
        
        for &x in &large_data {
            result.push((x * scale) as f32);
        }
    }
    let traditional_reuse_time = start.elapsed();
    
    println!("Pooled (20x):      {:?}", pooled_reuse_time);
    println!("Traditional (20x): {:?}", traditional_reuse_time);
    
    let improvement = if pooled_reuse_time < traditional_reuse_time {
        format!("{:.1}% faster", ((traditional_reuse_time.as_micros() as f32 / pooled_reuse_time.as_micros() as f32) - 1.0) * 100.0)
    } else {
        format!("{:.1}% slower", ((pooled_reuse_time.as_micros() as f32 / traditional_reuse_time.as_micros() as f32) - 1.0) * 100.0)
    };
    
    println!("Memory pool reuse: {}", improvement);
    
    println!("\n📈 Memory Pool Statistics:");
    let stats = renderer.get_pool_stats();
    println!("Total capacity: {} elements", stats.total_capacity());
    println!("Currently in use: {} allocations", stats.total_in_use());
    println!("Memory efficiency: {:.1}%", stats.efficiency() * 100.0);
    
    println!("\n✅ Performance test completed!");
    println!("💡 Run this with `cargo run --release --example memory_pool_performance_test` for accurate measurements");
    
    Ok(())
}