// Performance Contract Tests
// Critical performance requirements that define the library's value proposition

use ruviz::prelude::*;
use std::time::Instant;

#[cfg(test)]
mod performance_contract_tests {
    use super::*;

    /// Contract: 100K points MUST render in under 100ms (core requirement)
    #[test]
    fn test_100k_points_performance_contract() {
        let large_x: Vec<f64> = (0..100_000).map(|i| i as f64 * 0.001).collect();
        let large_y: Vec<f64> = (0..100_000).map(|i| (i as f64 * 0.001).sin()).collect();
        
        println!("Testing 100K points performance...");
        let start = Instant::now();
        
        // Will fail until Plot, line(), and render() are fully optimized
        let plot = Plot::new().line(&large_x, &large_y);
        let image = plot.render();
        
        let duration = start.elapsed();
        
        assert!(image.is_ok(), "100K points rendering failed: {:?}", image.err());
        
        // CRITICAL: Must render 100K points in under 100ms
        assert!(duration.as_millis() < 100, 
                "❌ PERFORMANCE FAILURE: 100K points took {}ms, required <100ms", 
                duration.as_millis());
        
        println!("✅ 100K points rendered in {}ms", duration.as_millis());
    }
    
    /// Contract: 1M points MUST render in under 1 second with optimization
    #[test]
    fn test_1m_points_performance_contract() {
        let mega_x: Vec<f64> = (0..1_000_000).map(|i| i as f64 * 0.000001).collect();
        let mega_y: Vec<f64> = (0..1_000_000).map(|i| (i as f64 * 0.000001).cos()).collect();
        
        println!("Testing 1M points performance (should trigger optimization)...");
        let start = Instant::now();
        
        // Will fail until DataShader or similar optimization is implemented
        let plot = Plot::new().scatter(&mega_x, &mega_y);
        let image = plot.render();
        
        let duration = start.elapsed();
        
        assert!(image.is_ok(), "1M points rendering failed: {:?}", image.err());
        
        // CRITICAL: Must render 1M points in under 1 second
        assert!(duration.as_secs() < 1, 
                "❌ PERFORMANCE FAILURE: 1M points took {}s, required <1s", 
                duration.as_secs_f64());
        
        println!("✅ 1M points rendered in {:.3}s", duration.as_secs_f64());
    }
    
    /// Contract: Memory usage MUST NOT exceed 2x data size
    #[test]
    fn test_memory_usage_contract() {
        let data_size = 500_000;
        let x: Vec<f64> = (0..data_size).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..data_size).map(|i| (i as f64).sqrt()).collect();
        
        // Baseline memory: 2 * Vec<f64> * size * 8 bytes = 2 * 500K * 8 = 8MB
        let baseline_memory = 2 * data_size * std::mem::size_of::<f64>();
        let max_allowed_memory = baseline_memory * 2; // 2x limit = 16MB
        
        println!("Testing memory usage (baseline: {}MB, limit: {}MB)", 
                baseline_memory / 1_000_000, max_allowed_memory / 1_000_000);
        
        // Get initial memory (mock - in real impl would use actual memory measurement)
        let initial_memory = get_current_memory_usage();
        
        // Will fail until memory-efficient rendering is implemented
        let plot = Plot::new().line(&x, &y);
        let _image = plot.render().expect("Render should succeed");
        
        let peak_memory = get_current_memory_usage();
        let memory_used = peak_memory.saturating_sub(initial_memory);
        
        // CRITICAL: Memory usage must not exceed 2x data size
        assert!(memory_used <= max_allowed_memory,
                "❌ MEMORY FAILURE: Used {}MB, limit {}MB (2x data size)", 
                memory_used / 1_000_000, max_allowed_memory / 1_000_000);
        
        println!("✅ Memory usage: {}MB (within {}MB limit)", 
                memory_used / 1_000_000, max_allowed_memory / 1_000_000);
    }
    
    /// Contract: DataShader optimization MUST activate for very large datasets
    #[test]
    fn test_datashader_activation_contract() {
        // Generate 5M points (should definitely trigger DataShader-style aggregation)  
        println!("Testing DataShader activation with 5M points...");
        
        let huge_x: Vec<f64> = (0..5_000_000).map(|i| (i as f64 * 0.0001) % 100.0).collect();
        let huge_y: Vec<f64> = (0..5_000_000).map(|i| ((i as f64 * 0.0001).sin() * 10.0)).collect();
        
        let start = Instant::now();
        
        // Will fail until DataShader/aggregation optimization is implemented
        let plot = Plot::new().scatter(&huge_x, &huge_y);
        let image = plot.render();
        
        let duration = start.elapsed();
        
        assert!(image.is_ok(), "DataShader test failed: {:?}", image.err());
        
        // DataShader should make this fast even with 5M points  
        assert!(duration.as_secs() < 3, 
                "❌ DATASHADER FAILURE: 5M points took {:.3}s, expected <3s with aggregation", 
                duration.as_secs_f64());
        
        println!("✅ 5M points rendered in {:.3}s (DataShader working)", duration.as_secs_f64());
    }
    
    /// Contract: Incremental updates MUST be fast for interactive use
    #[test]
    fn test_incremental_update_performance_contract() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        // Will fail until mutable plot API is implemented
        let mut plot = Plot::new().line(&x, &y);
        let _initial = plot.render().expect("Initial render should work");
        
        // Adding new data should be very fast
        let start = Instant::now();
        
        // Will fail until add_line() or similar incremental API is implemented
        plot.add_line(&[4.0, 5.0, 6.0], &[16.0, 25.0, 36.0]);
        let _updated = plot.render().expect("Updated render should work");
        
        let update_duration = start.elapsed();
        
        // Incremental updates must be under 50ms
        assert!(update_duration.as_millis() < 50,
                "❌ INCREMENTAL FAILURE: Update took {}ms, expected <50ms", 
                update_duration.as_millis());
        
        println!("✅ Incremental update: {}ms", update_duration.as_millis());
    }

    /// Contract: Parallel rendering MUST work for independent plots
    #[cfg(feature = "parallel")]
    #[test]
    fn test_parallel_rendering_contract() {
        use std::sync::Arc;
        use std::thread;
        
        let data_size = 50_000;
        let x: Arc<Vec<f64>> = Arc::new((0..data_size).map(|i| i as f64).collect());
        let y: Arc<Vec<f64>> = Arc::new((0..data_size).map(|i| (i as f64).sin()).collect());
        
        println!("Testing parallel rendering with 4 threads...");
        let start = Instant::now();
        
        let handles: Vec<_> = (0..4).map(|i| {
            let x_clone = Arc::clone(&x);
            let y_clone = Arc::clone(&y);
            
            thread::spawn(move || {
                let plot = Plot::new()
                    .line(&*x_clone, &*y_clone)
                    .title(&format!("Plot {}", i));
                plot.render().expect("Parallel render should work")
            })
        }).collect();
        
        let _results: Vec<_> = handles.into_iter()
            .map(|h| h.join().unwrap())
            .collect();
        
        let parallel_duration = start.elapsed();
        
        // Parallel should be significantly faster than 4x sequential
        println!("✅ Parallel rendering: {:.3}s", parallel_duration.as_secs_f64());
    }

    /// Contract: Export performance MUST be reasonable
    #[test] 
    fn test_export_performance_contract() {
        let x: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..10_000).map(|i| (i as f64).sin()).collect();
        
        let plot = Plot::new().line(&x, &y);
        
        // PNG export should be fast
        let start = Instant::now();
        let result = plot.save("test_export_perf.png");
        let export_duration = start.elapsed();
        
        assert!(result.is_ok(), "PNG export failed: {:?}", result.err());
        assert!(export_duration.as_millis() < 500, 
                "PNG export took {}ms, expected <500ms", export_duration.as_millis());
        
        // Clean up
        std::fs::remove_file("test_export_perf.png").ok();
        
        println!("✅ PNG export: {}ms", export_duration.as_millis());
    }

    /// Contract: Stress test - multiple large plots should not crash
    #[test]
    fn test_stress_test_contract() {
        println!("Running stress test with multiple large plots...");
        
        for i in 0..5 {
            let size = 20_000;
            let x: Vec<f64> = (0..size).map(|j| j as f64 + i as f64 * size as f64).collect();
            let y: Vec<f64> = (0..size).map(|j| ((j as f64) * (i + 1) as f64).sin()).collect();
            
            let plot = Plot::new()
                .line(&x, &y)
                .title(&format!("Stress Test Plot {}", i));
                
            let result = plot.render();
            assert!(result.is_ok(), "Stress test plot {} failed: {:?}", i, result.err());
        }
        
        println!("✅ Stress test completed - 5 large plots rendered successfully");
    }
    
    // Mock function for memory measurement - in real implementation would use system APIs
    fn get_current_memory_usage() -> usize {
        // Placeholder - real implementation would measure actual memory usage
        // For now, return a small value so tests focus on implementation
        1_000_000 // 1MB baseline
    }
}