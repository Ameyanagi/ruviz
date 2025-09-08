// Basic Line Plot Integration Test (Story 1)
// "Given I have numeric data in vectors/arrays, When I create a basic line plot with 
// axis labels and title, Then the library generates a clear, properly labeled visualization"

use ruviz::prelude::*;
use std::path::Path;

#[cfg(test)]
mod basic_line_integration_tests {
    use super::*;

    /// Integration Test: Story 1 - Basic line plot with labels
    #[test]
    fn test_story_1_basic_line_plot_integration() {
        // Given: I have numeric data in vectors/arrays
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![0.0, 1.0, 4.0, 9.0, 16.0]; // quadratic function
        
        // When: I create a basic line plot with axis labels and title
        // Will fail until Plot::new(), line(), title(), xlabel(), ylabel(), save() are implemented
        let result = Plot::new()
            .line(&x, &y)
            .title("Quadratic Function")
            .xlabel("x")
            .ylabel("y = x²")
            .save("test_basic_line_integration.png");
        
        // Then: The library generates a clear, properly labeled visualization
        assert!(result.is_ok(), "Basic line plot creation failed: {:?}", result.err());
        
        // Verify the file was created
        assert!(Path::new("test_basic_line_integration.png").exists(),
                "PNG file was not created");
        
        // Verify file is not empty (basic sanity check)
        let metadata = std::fs::metadata("test_basic_line_integration.png")
            .expect("Could not read file metadata");
        assert!(metadata.len() > 0, "Generated PNG file is empty");
        
        // Clean up test file
        std::fs::remove_file("test_basic_line_integration.png").ok();
    }

    /// Integration Test: Different data types (Vec, array, slice) 
    #[test]
    fn test_data_type_compatibility_integration() {
        // Test Vec<f64>
        let vec_x = vec![1.0, 2.0, 3.0];
        let vec_y = vec![2.0, 4.0, 6.0];
        
        let result1 = Plot::new()
            .line(&vec_x, &vec_y)
            .title("Vec Data")
            .save("test_vec_data.png");
        assert!(result1.is_ok(), "Vec data failed: {:?}", result1.err());
        std::fs::remove_file("test_vec_data.png").ok();
        
        // Test arrays
        let array_x = [1.0, 2.0, 3.0];
        let array_y = [3.0, 6.0, 9.0];
        
        let result2 = Plot::new()
            .line(&array_x, &array_y)
            .title("Array Data")
            .save("test_array_data.png");
        assert!(result2.is_ok(), "Array data failed: {:?}", result2.err());
        std::fs::remove_file("test_array_data.png").ok();
        
        // Test slices
        let slice_x: &[f64] = &[1.0, 2.0, 3.0];
        let slice_y: &[f64] = &[4.0, 8.0, 12.0];
        
        let result3 = Plot::new()
            .line(slice_x, slice_y)
            .title("Slice Data")
            .save("test_slice_data.png");
        assert!(result3.is_ok(), "Slice data failed: {:?}", result3.err());
        std::fs::remove_file("test_slice_data.png").ok();
    }

    /// Integration Test: Default dimensions validation
    #[test]
    fn test_default_dimensions_integration() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        let plot = Plot::new()
            .line(&x, &y)
            .title("Dimension Test");
        
        let image = plot.render();
        assert!(image.is_ok(), "Render failed: {:?}", image.err());
        
        let img = image.unwrap();
        assert_eq!(img.width(), 800, "Default width should be 800px");
        assert_eq!(img.height(), 600, "Default height should be 600px");
    }

    /// Integration Test: Mathematical functions visualization  
    #[test]
    fn test_mathematical_functions_integration() {
        use std::f64::consts::PI;
        
        // Generate sine wave data
        let x: Vec<f64> = (0..100).map(|i| (i as f64) * 0.1).collect();
        let sine_y: Vec<f64> = x.iter().map(|&x| x.sin()).collect();
        
        let result1 = Plot::new()
            .line(&x, &sine_y)
            .title("Sine Wave")
            .xlabel("x")
            .ylabel("sin(x)")
            .save("test_sine_wave.png");
        
        assert!(result1.is_ok(), "Sine wave plot failed: {:?}", result1.err());
        std::fs::remove_file("test_sine_wave.png").ok();
        
        // Generate exponential data
        let exp_x: Vec<f64> = (0..50).map(|i| (i as f64) * 0.1).collect();
        let exp_y: Vec<f64> = exp_x.iter().map(|&x| (-x).exp()).collect();
        
        let result2 = Plot::new()
            .line(&exp_x, &exp_y)
            .title("Exponential Decay")
            .xlabel("x")
            .ylabel("e^(-x)")
            .save("test_exponential.png");
        
        assert!(result2.is_ok(), "Exponential plot failed: {:?}", result2.err());
        std::fs::remove_file("test_exponential.png").ok();
    }

    /// Integration Test: Edge cases that should work
    #[test]
    fn test_edge_cases_integration() {
        // Single point
        let single_x = vec![5.0];
        let single_y = vec![10.0];
        
        let result1 = Plot::new()
            .line(&single_x, &single_y)
            .title("Single Point")
            .save("test_single_point.png");
        assert!(result1.is_ok(), "Single point failed: {:?}", result1.err());
        std::fs::remove_file("test_single_point.png").ok();
        
        // Two points (minimum for a line)
        let two_x = vec![0.0, 1.0];
        let two_y = vec![0.0, 1.0];
        
        let result2 = Plot::new()
            .line(&two_x, &two_y)
            .title("Two Points")
            .save("test_two_points.png");
        assert!(result2.is_ok(), "Two points failed: {:?}", result2.err());
        std::fs::remove_file("test_two_points.png").ok();
        
        // Flat line (all same Y values)
        let flat_x = vec![1.0, 2.0, 3.0, 4.0];
        let flat_y = vec![5.0, 5.0, 5.0, 5.0];
        
        let result3 = Plot::new()
            .line(&flat_x, &flat_y)
            .title("Flat Line")
            .save("test_flat_line.png");
        assert!(result3.is_ok(), "Flat line failed: {:?}", result3.err());
        std::fs::remove_file("test_flat_line.png").ok();
    }

    /// Integration Test: Error cases that should fail gracefully
    #[test]
    fn test_error_cases_integration() {
        // Empty data should return meaningful error
        let empty_x: Vec<f64> = vec![];
        let empty_y: Vec<f64> = vec![];
        
        let plot1 = Plot::new().line(&empty_x, &empty_y);
        let result1 = plot1.render();
        assert!(result1.is_err(), "Empty data should fail gracefully");
        
        // Mismatched array lengths should fail
        let x = vec![1.0, 2.0, 3.0];
        let mismatched_y = vec![1.0, 2.0]; // Different length
        
        let plot2 = Plot::new().line(&x, &mismatched_y);
        let result2 = plot2.render();
        assert!(result2.is_err(), "Mismatched lengths should fail gracefully");
        
        // Invalid file path should fail
        let x = vec![1.0, 2.0];
        let y = vec![1.0, 2.0];
        
        let plot3 = Plot::new().line(&x, &y);
        let result3 = plot3.save("/invalid/path/test.png");
        assert!(result3.is_err(), "Invalid path should fail gracefully");
    }
}