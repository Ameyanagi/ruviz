use ruviz::prelude::*;
use ruviz::data::memory::{get_memory_manager, MemoryConfig, initialize_memory_manager};

fn main() -> Result<()> {
    // Initialize with custom memory configuration for demonstration
    let config = MemoryConfig {
        enable_pooling: true,
        initial_pool_size: 1000,
        max_pool_size: 10000,
        pool_growth_factor: 2.0,
        large_alloc_threshold: 1024 * 1024,  // 1MB
        max_pool_memory: 50 * 1024 * 1024,   // 50MB
    };
    
    // Initialize memory manager (this will be used for all subsequent operations)
    let _ = initialize_memory_manager(config);
    
    println!("=== Memory Optimization Demo ===");
    
    // Get memory manager and display initial stats
    let memory_manager = get_memory_manager();
    let initial_stats = memory_manager.get_stats();
    println!("Initial memory stats: {:#?}", initial_stats);
    
    // Create test datasets of increasing size
    let sizes = [1_000, 10_000, 100_000];
    let mut plots = Vec::new();
    
    for &size in &sizes {
        println!("\nProcessing dataset with {} points...", size);
        
        // Generate test data
        let x_data: Vec<f64> = (0..size).map(|i| i as f64 * 0.01).collect();
        let y_data: Vec<f64> = (0..size).map(|i| (i as f64 * 0.01).sin()).collect();
        
        // Create plot with memory optimization
        let plot = Plot::new()
            .scatter(&x_data, &y_data)
            .title(&format!("Memory Test - {} points", size))
            .xlabel("X")
            .ylabel("Sin(X)");
        
        plots.push(plot);
        
        // Check memory usage after each plot
        let stats = memory_manager.get_stats();
        println!("Memory stats after {} points:", size);
        println!("  Total allocated: {} bytes", stats.total_allocated);
        println!("  Pool hits: {}", stats.pool_hits);
        println!("  Pool misses: {}", stats.pool_misses);
        println!("  Active allocations: {}", stats.active_allocations);
        
        let hit_rate = if stats.pool_hits + stats.pool_misses > 0 {
            stats.pool_hits as f32 / (stats.pool_hits + stats.pool_misses) as f32 * 100.0
        } else {
            0.0
        };
        println!("  Pool hit rate: {:.1}%", hit_rate);
    }
    
    // Test coordinate transformation with memory pooling
    println!("\n=== Testing Memory-Optimized Coordinate Transformation ===");
    
    let large_dataset_size = 500_000;
    let x_data: Vec<f64> = (0..large_dataset_size).map(|i| i as f64).collect();
    let y_data: Vec<f64> = (0..large_dataset_size).map(|i| (i as f64 * 0.001).cos()).collect();
    
    let start_time = std::time::Instant::now();
    
    // This will use the memory-optimized parallel renderer with buffer pooling
    let plot = Plot::new()
        .line(&x_data, &y_data)
        .title(&format!("Large Dataset - {} points", large_dataset_size))
        .xlabel("Index")
        .ylabel("Cosine");
    
    // Render to trigger coordinate transformation
    let _result = plot.render();
    
    let elapsed = start_time.elapsed();
    println!("Rendered {} points in {:.2}ms", large_dataset_size, elapsed.as_millis());
    
    // Final memory statistics
    let final_stats = memory_manager.get_stats();
    println!("\nFinal memory statistics:");
    println!("  Total allocated: {} bytes ({:.2} MB)", 
             final_stats.total_allocated,
             final_stats.total_allocated as f32 / 1_048_576.0);
    println!("  Pool hits: {}", final_stats.pool_hits);
    println!("  Pool misses: {}", final_stats.pool_misses);
    
    let final_hit_rate = if final_stats.pool_hits + final_stats.pool_misses > 0 {
        final_stats.pool_hits as f32 / (final_stats.pool_hits + final_stats.pool_misses) as f32 * 100.0
    } else {
        0.0
    };
    println!("  Overall pool hit rate: {:.1}%", final_hit_rate);
    
    if final_hit_rate > 50.0 {
        println!("✅ Memory pooling is working effectively!");
    } else {
        println!("⚠️  Memory pooling hit rate could be improved");
    }
    
    // Clean up demonstration
    println!("\nMemory optimization demo completed successfully!");
    
    Ok(())
}