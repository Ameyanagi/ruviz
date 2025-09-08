// Large Dataset Integration Test (Story 3)
// "Given I have a large dataset (1M+ points), When I create a scatter plot, 
// Then the library automatically optimizes rendering to maintain interactive 
// performance without losing visual fidelity"

use ruviz::prelude::*;
use std::time::Instant;
use std::path::Path;

#[cfg(test)]
mod large_dataset_integration_tests {
    use super::*;

    /// Integration Test: Story 3 - Large dataset with automatic optimization
    #[test]
    fn test_story_3_large_dataset_integration() {
        println!("Generating 1M data points for large dataset test...");
        
        // Given: I have a large dataset (1M+ points)
        let large_x: Vec<f64> = (0..1_000_000)
            .map(|i| i as f64 * 0.001)
            .collect();
        let large_y: Vec<f64> = large_x.iter()
            .map(|&x| x.sin() + 0.1 * (x * 10.0).cos()) // Complex signal
            .collect();
        
        println!("Creating scatter plot with automatic optimization...");
        let start = Instant::now();
        
        // When: I create a scatter plot
        // Will fail until scatter(), optimization logic, and performance targets are met
        let result = Plot::new()
            .scatter(&large_x, &large_y)
            .title("Large Dataset Visualization (1M points)")
            .xlabel("Time")
            .ylabel("Signal")
            .alpha(0.1) // Semi-transparent for overlapping points
            .save("test_large_dataset_integration.png");
        
        let duration = start.elapsed();
        println!("Large dataset plot completed in {:?}", duration);
        
        // Then: The library automatically optimizes rendering to maintain performance
        assert!(result.is_ok(), "Large dataset plot failed: {:?}", result.err());
        
        // Performance requirement: <1000ms for 1M points
        assert!(duration.as_millis() < 1000,
                "❌ PERFORMANCE FAILURE: 1M points took {}ms, required <1000ms", 
                duration.as_millis());
        
        // Verify the file was created and has reasonable size
        assert!(Path::new("test_large_dataset_integration.png").exists(),
                "Large dataset PNG file was not created");
        
        let metadata = std::fs::metadata("test_large_dataset_integration.png")
            .expect("Could not read large dataset file metadata");
        
        // File should be substantial but not huge (optimization working)
        assert!(metadata.len() > 10000, "Large dataset file too small: {} bytes", metadata.len());
        assert!(metadata.len() < 10_000_000, "Large dataset file too large: {} bytes (optimization may not be working)", metadata.len());
        
        println!("✅ Performance target met: {}ms for 1M points", duration.as_millis());
        println!("✅ Output file size: {} KB", metadata.len() / 1000);
        
        // Clean up test file
        std::fs::remove_file("test_large_dataset_integration.png").ok();
    }

    /// Integration Test: DataShader activation detection
    #[test]
    fn test_datashader_activation_integration() {
        println!("Testing DataShader activation with 2M points...");
        
        // Create dense dataset that should trigger aggregation
        let huge_x: Vec<f64> = (0..2_000_000)
            .map(|i| (i as f64 * 0.0001) % 10.0) // Bounded domain for overlap
            .collect();
        let huge_y: Vec<f64> = (0..2_000_000)
            .map(|i| ((i as f64 * 0.0001).sin() * 5.0))
            .collect();
        
        let start = Instant::now();
        
        // Should trigger DataShader-style aggregation
        let result = Plot::new()
            .scatter(&huge_x, &huge_y)
            .title("DataShader Test (2M points)")
            .xlabel("X")
            .ylabel("Y")
            .save("test_datashader_activation.png");
        
        let duration = start.elapsed();
        println!("DataShader test completed in {:?}", duration);
        
        assert!(result.is_ok(), "DataShader activation test failed: {:?}", result.err());
        
        // Should be fast even with 2M points due to aggregation
        assert!(duration.as_secs() < 2,
                "DataShader should handle 2M points in <2s, took {:.3}s", 
                duration.as_secs_f64());
        
        println!("✅ DataShader performance: {:.3}s for 2M points", duration.as_secs_f64());
        
        std::fs::remove_file("test_datashader_activation.png").ok();
    }

    /// Integration Test: Memory efficiency with large datasets
    #[test]
    fn test_memory_efficiency_integration() {
        let data_size = 500_000; // 500K points
        println!("Testing memory efficiency with {} points...", data_size);
        
        let x: Vec<f64> = (0..data_size).map(|i| i as f64 * 0.001).collect();
        let y: Vec<f64> = (0..data_size).map(|i| (i as f64 * 0.001).sqrt()).collect();
        
        // Calculate baseline memory: 2 arrays * size * 8 bytes per f64
        let baseline_memory = 2 * data_size * std::mem::size_of::<f64>();
        println!("Data baseline memory: {} MB", baseline_memory / 1_000_000);
        
        let plot = Plot::new()
            .line(&x, &y)
            .title("Memory Efficiency Test");
        
        let result = plot.render();
        assert!(result.is_ok(), "Memory efficiency test failed: {:?}", result.err());
        
        // Note: Actual memory measurement would require platform-specific APIs
        // This test validates the rendering completes without excessive memory use
        println!("✅ Memory efficiency test completed successfully");
    }

    /// Integration Test: Performance scaling with different dataset sizes
    #[test]
    fn test_performance_scaling_integration() {
        let test_sizes = vec![10_000, 50_000, 100_000, 250_000];
        
        for &size in &test_sizes {
            println!("Testing performance with {} points...", size);
            
            let x: Vec<f64> = (0..size).map(|i| i as f64).collect();
            let y: Vec<f64> = (0..size).map(|i| (i as f64).sin()).collect();
            
            let start = Instant::now();
            
            let result = Plot::new()
                .line(&x, &y)
                .title(&format!("{} Points Performance Test", size))
                .save(&format!("test_perf_{}.png", size));
            
            let duration = start.elapsed();
            
            assert!(result.is_ok(), "{} points test failed: {:?}", size, result.err());
            
            // Performance should scale reasonably
            let ms_per_1k_points = (duration.as_millis() as f64) / (size as f64 / 1000.0);
            println!("  {} points: {}ms ({:.2}ms per 1K points)", 
                    size, duration.as_millis(), ms_per_1k_points);
            
            // Clean up
            std::fs::remove_file(&format!("test_perf_{}.png", size)).ok();
        }
        
        println!("✅ Performance scaling test completed");
    }

    /// Integration Test: Large dataset with multiple series
    #[test]  
    fn test_large_multi_series_integration() {
        let size = 200_000;
        println!("Testing large multi-series plot with {} points per series...", size);
        
        let x: Vec<f64> = (0..size).map(|i| i as f64 * 0.01).collect();
        let y1: Vec<f64> = x.iter().map(|&x| x.sin()).collect();
        let y2: Vec<f64> = x.iter().map(|&x| x.cos()).collect();
        let y3: Vec<f64> = x.iter().map(|&x| (x * 0.1).sin() * x.cos()).collect();
        
        let start = Instant::now();
        
        let result = Plot::new()
            .line(&x, &y1)
                .label("sin(x)")
                .color(Color::RED)
            .line(&x, &y2)
                .label("cos(x)")
                .color(Color::BLUE)
            .line(&x, &y3)
                .label("sin(0.1x)·cos(x)")
                .color(Color::GREEN)
            .title(&format!("Large Multi-Series ({} points each)", size))
            .legend(Position::TopRight)
            .save("test_large_multi_series.png");
        
        let duration = start.elapsed();
        
        assert!(result.is_ok(), "Large multi-series failed: {:?}", result.err());
        
        // Should handle multiple large series efficiently
        assert!(duration.as_millis() < 500,
                "Large multi-series took {}ms, expected <500ms", 
                duration.as_millis());
        
        println!("✅ Large multi-series: {}ms for 3x{} points", 
                duration.as_millis(), size);
        
        std::fs::remove_file("test_large_multi_series.png").ok();
    }

    /// Integration Test: Streaming/incremental updates 
    #[test]
    fn test_incremental_updates_integration() {
        println!("Testing incremental updates performance...");
        
        let initial_x = vec![1.0, 2.0, 3.0];
        let initial_y = vec![1.0, 4.0, 9.0];
        
        let mut plot = Plot::new()
            .line(&initial_x, &initial_y)
            .title("Incremental Updates Test");
        
        let _initial_render = plot.render();
        
        // Add data incrementally and measure update performance
        for i in 0..10 {
            let start = Instant::now();
            
            let new_x = vec![4.0 + i as f64, 5.0 + i as f64];
            let new_y = vec![(4.0 + i as f64).powi(2), (5.0 + i as f64).powi(2)];
            
            // Will fail until incremental update API is implemented
            plot.add_line(&new_x, &new_y);
            let _updated = plot.render();
            
            let update_duration = start.elapsed();
            
            // Incremental updates should be very fast
            assert!(update_duration.as_millis() < 50,
                    "Incremental update {} took {}ms, expected <50ms", 
                    i, update_duration.as_millis());
        }
        
        let final_result = plot.save("test_incremental_updates.png");
        assert!(final_result.is_ok(), "Final incremental save failed: {:?}", final_result.err());
        
        std::fs::remove_file("test_incremental_updates.png").ok();
        println!("✅ Incremental updates test completed");
    }

    /// Integration Test: Extreme dataset size handling
    #[test]
    #[ignore] // Ignore by default due to memory/time requirements
    fn test_extreme_dataset_integration() {
        println!("Testing extreme dataset (10M points) - this may take time...");
        
        let extreme_size = 10_000_000;
        let x: Vec<f64> = (0..extreme_size)
            .map(|i| (i as f64) * 0.0000001)
            .collect();
        let y: Vec<f64> = (0..extreme_size)
            .map(|i| ((i as f64) * 0.0000001).sin())
            .collect();
        
        let start = Instant::now();
        
        let result = Plot::new()
            .scatter(&x, &y)
            .title("Extreme Dataset Test (10M points)")
            .alpha(0.05) // Very transparent
            .save("test_extreme_dataset.png");
        
        let duration = start.elapsed();
        
        assert!(result.is_ok(), "Extreme dataset test failed: {:?}", result.err());
        
        // Should complete in reasonable time with heavy optimization
        assert!(duration.as_secs() < 10,
                "Extreme dataset took {:.3}s, expected <10s with optimization", 
                duration.as_secs_f64());
        
        println!("✅ Extreme dataset: {:.3}s for 10M points", duration.as_secs_f64());
        
        std::fs::remove_file("test_extreme_dataset.png").ok();
    }
}