// Core API Contract Tests for Rust Plotting Library
// These tests define the core API contract and MUST fail until implementation is complete

use ruviz::prelude::*;
use std::time::Instant;

#[cfg(test)]
mod core_api_contract_tests {
    use super::*;

    /// Contract: Plot creation with data must not panic and should return a Plot
    #[test]
    fn test_plot_creation_contract() {
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];
        
        // This will fail until Plot::new() is implemented
        let plot = Plot::new();
        
        // This will fail until line() method is implemented  
        let plot_with_data = plot.line(&x, &y);
        
        // Must not panic on valid data
        assert!(true); // Placeholder - real validation happens in render
    }
    
    /// Contract: Plot must accept standard Rust collections (Vec, arrays, slices)
    #[test]
    fn test_data_types_contract() {
        let vec_data = vec![1.0, 2.0, 3.0];
        let array_data = [1.0, 2.0, 3.0];
        let slice_data: &[f64] = &[1.0, 2.0, 3.0];
        
        // This will fail until Plot and line() are implemented
        let plot = Plot::new();
        
        // Must accept Vec, arrays, and slices  
        let _plot1 = plot.line(&vec_data, &vec_data);
        // Note: Rust ownership means we need separate plot instances
        let _plot2 = Plot::new().line(&array_data, &array_data);  
        let _plot3 = Plot::new().line(slice_data, slice_data);
        
        assert!(true); // Placeholder
    }
    
    /// Contract: Plot methods must be chainable (fluent API)
    #[test] 
    fn test_fluent_api_contract() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        // This entire chain will fail until methods are implemented
        let plot = Plot::new()
            .line(&x, &y)
            .title("Test Plot")
            .xlabel("X Axis")
            .ylabel("Y Axis");
            
        // Must return Plot for chaining - will be validated by compilation
        assert!(true); // Placeholder
    }
    
    /// Contract: Performance requirement - 100K points must render in <100ms
    #[test]
    fn test_performance_contract() {
        let large_data: Vec<f64> = (0..100_000).map(|i| i as f64).collect();
        
        let start = Instant::now();
        
        // This will fail until Plot, line(), and render() are implemented
        let plot = Plot::new().line(&large_data, &large_data);
        let _image = plot.render(); // Will fail - render() not implemented
        
        let duration = start.elapsed();
        
        // Must render 100K points in under 100ms (performance contract)
        assert!(duration.as_millis() < 100, 
                "Rendering took {}ms, expected <100ms", duration.as_millis());
    }
    
    /// Contract: Export to PNG must succeed with valid file output
    #[test]
    fn test_export_contract() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        // This will fail until Plot, line(), and save() are implemented  
        let plot = Plot::new().line(&x, &y);
        let result = plot.save("test_contract_output.png");
        
        // Must successfully export to PNG
        assert!(result.is_ok(), "PNG export failed: {:?}", result.err());
        
        // Verify file was actually created
        assert!(std::path::Path::new("test_contract_output.png").exists(),
                "PNG file was not created");
        
        // Clean up test file
        std::fs::remove_file("test_contract_output.png").ok();
    }

    /// Contract: Error handling must provide meaningful messages
    #[test] 
    fn test_error_handling_contract() {
        let empty_data: Vec<f64> = vec![];
        let mismatched_x = vec![1.0, 2.0];
        let mismatched_y = vec![1.0, 2.0, 3.0]; // Different length
        
        // This will fail until error handling is implemented
        let plot1 = Plot::new().line(&empty_data, &empty_data);
        let result1 = plot1.render();
        
        // Empty data should return descriptive error
        assert!(result1.is_err(), "Empty data should return error");
        
        // Mismatched lengths should return descriptive error  
        let plot2 = Plot::new().line(&mismatched_x, &mismatched_y);
        let result2 = plot2.render();
        assert!(result2.is_err(), "Mismatched data lengths should return error");
    }

    /// Contract: Default plot dimensions must be reasonable (800x600)
    #[test]
    fn test_default_dimensions_contract() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        // This will fail until implementation is complete
        let plot = Plot::new().line(&x, &y);
        let image = plot.render().expect("Render should succeed");
        
        // Default dimensions should be 800x600
        assert_eq!(image.width(), 800, "Default width should be 800");
        assert_eq!(image.height(), 600, "Default height should be 600"); 
    }
}