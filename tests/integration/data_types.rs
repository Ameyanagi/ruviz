// Data Integration Test (Story 5)  
// "Given I'm integrating with existing Rust data tools, When I provide data from 
// common formats, Then the library seamlessly accepts and visualizes the data 
// without requiring conversion"

use ruviz::prelude::*;
use std::path::Path;

#[cfg(test)]
mod data_integration_tests {
    use super::*;

    /// Integration Test: Story 5 - Data integration with common Rust formats
    #[test]
    fn test_story_5_data_integration() {
        // Given: I'm integrating with existing Rust data tools
        // Test baseline with Vec<f64> (should always work)
        let vec_x = vec![1.0, 2.0, 3.0, 4.0];
        let vec_y = vec![1.0, 4.0, 9.0, 16.0];
        
        // When: I provide data from common formats
        let result_vec = Plot::new()
            .line(&vec_x, &vec_y)
            .title("Vec<f64> Data Integration")
            .save("test_vec_integration.png");
        
        // Then: The library seamlessly accepts and visualizes the data
        assert!(result_vec.is_ok(), "Vec<f64> integration failed: {:?}", result_vec.err());
        assert!(Path::new("test_vec_integration.png").exists());
        std::fs::remove_file("test_vec_integration.png").ok();
        
        // Test array integration
        let array_x = [1.0, 2.0, 3.0, 4.0];
        let array_y = [2.0, 5.0, 10.0, 17.0];
        
        let result_array = Plot::new()
            .line(&array_x, &array_y)
            .title("Array Data Integration")
            .save("test_array_integration.png");
        
        assert!(result_array.is_ok(), "Array integration failed: {:?}", result_array.err());
        assert!(Path::new("test_array_integration.png").exists());
        std::fs::remove_file("test_array_integration.png").ok();
        
        println!("✅ Basic data types integration completed");
    }

    /// Integration Test: ndarray support (scientific computing)
    #[cfg(feature = "ndarray_support")]
    #[test]
    fn test_ndarray_integration() {
        use ndarray::Array1;
        
        println!("Testing ndarray integration...");
        
        // Create ndarray data
        let nd_x = Array1::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let nd_y = Array1::from(vec![2.0, 5.0, 10.0, 17.0, 26.0]);
        
        // Will fail until ndarray Data1D implementation is complete
        let result = Plot::new()
            .line(&nd_x, &nd_y)
            .title("ndarray Integration Test")
            .xlabel("X (ndarray)")
            .ylabel("Y (ndarray)")
            .save("test_ndarray_integration.png");
        
        assert!(result.is_ok(), "ndarray integration failed: {:?}", result.err());
        assert!(Path::new("test_ndarray_integration.png").exists());
        
        // Test with mathematical operations on ndarray
        let x_range = Array1::linspace(0.0, 10.0, 100);
        let y_sin = x_range.mapv(|x| x.sin());
        let y_cos = x_range.mapv(|x| x.cos());
        
        let result2 = Plot::new()
            .line(&x_range, &y_sin)
                .label("sin(x)")
                .color(Color::RED)
            .line(&x_range, &y_cos)
                .label("cos(x)")
                .color(Color::BLUE)
            .title("ndarray Mathematical Operations")
            .legend(Position::TopRight)
            .save("test_ndarray_math.png");
        
        assert!(result2.is_ok(), "ndarray math integration failed: {:?}", result2.err());
        
        std::fs::remove_file("test_ndarray_integration.png").ok();
        std::fs::remove_file("test_ndarray_math.png").ok();
        
        println!("✅ ndarray integration working");
    }

    /// Integration Test: polars DataFrame support
    #[cfg(feature = "polars_support")]  
    #[test]
    fn test_polars_integration() {
        use polars::prelude::*;
        
        println!("Testing polars integration...");
        
        // Create polars DataFrame  
        let df = df! {
            "x" => [1.0, 2.0, 3.0, 4.0, 5.0],
            "y" => [3.0, 6.0, 11.0, 18.0, 27.0],
            "category" => ["A", "A", "B", "B", "C"],
            "value" => [10.0, 15.0, 12.0, 18.0, 14.0]
        }.expect("DataFrame creation should succeed");
        
        // Will fail until polars Data1D implementation is complete
        let result = Plot::new()
            .line(
                df.column("x").expect("x column should exist"), 
                df.column("y").expect("y column should exist")
            )
            .title("polars DataFrame Integration")
            .xlabel("X (polars Series)")
            .ylabel("Y (polars Series)")
            .save("test_polars_integration.png");
        
        assert!(result.is_ok(), "polars integration failed: {:?}", result.err());
        assert!(Path::new("test_polars_integration.png").exists());
        
        // Test grouped data visualization
        let category_a_df = df.filter(
            &col("category").eq(lit("A"))
        ).expect("Filter should work");
        
        let category_b_df = df.filter(
            &col("category").eq(lit("B"))
        ).expect("Filter should work");
        
        let result2 = Plot::new()
            .scatter(
                category_a_df.column("x").unwrap(),
                category_a_df.column("value").unwrap()
            )
                .label("Category A")
                .color(Color::RED)
            .scatter(
                category_b_df.column("x").unwrap(), 
                category_b_df.column("value").unwrap()
            )
                .label("Category B")
                .color(Color::BLUE)
            .title("polars Grouped Data")
            .legend(Position::TopRight)
            .save("test_polars_grouped.png");
        
        assert!(result2.is_ok(), "polars grouped integration failed: {:?}", result2.err());
        
        std::fs::remove_file("test_polars_integration.png").ok();
        std::fs::remove_file("test_polars_grouped.png").ok();
        
        println!("✅ polars integration working");
    }

    /// Integration Test: Mixed data types in same plot
    #[test]
    fn test_mixed_data_types_integration() {
        // Vec data
        let vec_x = vec![1.0, 2.0, 3.0];
        let vec_y = vec![1.0, 4.0, 9.0];
        
        // Array data  
        let array_x = [4.0, 5.0, 6.0];
        let array_y = [16.0, 25.0, 36.0];
        
        // Slice data
        let slice_x: &[f64] = &[7.0, 8.0, 9.0];
        let slice_y: &[f64] = &[49.0, 64.0, 81.0];
        
        // Should handle mixed types in same plot
        let result = Plot::new()
            .line(&vec_x, &vec_y)
                .label("Vec Data")
                .color(Color::RED)
            .line(&array_x, &array_y)
                .label("Array Data") 
                .color(Color::BLUE)
            .line(slice_x, slice_y)
                .label("Slice Data")
                .color(Color::GREEN)
            .title("Mixed Data Types")
            .legend(Position::TopRight)
            .save("test_mixed_data_types.png");
        
        assert!(result.is_ok(), "Mixed data types failed: {:?}", result.err());
        std::fs::remove_file("test_mixed_data_types.png").ok();
    }

    /// Integration Test: Iterator-based data (lazy evaluation)
    #[test] 
    fn test_iterator_data_integration() {
        // Create data from iterators
        let x_iter = (0..100).map(|i| i as f64 * 0.1);
        let y_iter = (0..100).map(|i| (i as f64 * 0.1).sin());
        
        let x: Vec<f64> = x_iter.collect();
        let y: Vec<f64> = y_iter.collect();
        
        let result = Plot::new()
            .line(&x, &y)
            .title("Iterator-based Data")
            .xlabel("x")
            .ylabel("sin(x)")
            .save("test_iterator_data.png");
        
        assert!(result.is_ok(), "Iterator data integration failed: {:?}", result.err());
        std::fs::remove_file("test_iterator_data.png").ok();
    }

    /// Integration Test: Range-based data
    #[test]
    fn test_range_data_integration() {
        use std::ops::Range;
        
        // Test with Range (will need conversion to Vec first in current design)
        let range_data: Range<i32> = 0..20;
        let x: Vec<f64> = range_data.map(|i| i as f64).collect();
        let y: Vec<f64> = (0..20).map(|i| (i as f64).powi(2)).collect();
        
        let result = Plot::new()
            .line(&x, &y)
            .title("Range-based Data")
            .save("test_range_data.png");
        
        assert!(result.is_ok(), "Range data integration failed: {:?}", result.err());
        std::fs::remove_file("test_range_data.png").ok();
    }

    /// Integration Test: Large data with different types  
    #[test]
    fn test_large_data_types_integration() {
        let size = 100_000;
        
        // Vec version
        let vec_x: Vec<f64> = (0..size).map(|i| i as f64 * 0.001).collect();
        let vec_y: Vec<f64> = vec_x.iter().map(|&x| x.sin()).collect();
        
        let start = std::time::Instant::now();
        let result1 = Plot::new()
            .line(&vec_x, &vec_y)
            .title(&format!("Large Vec Data ({} points)", size))
            .save("test_large_vec.png");
        let vec_duration = start.elapsed();
        
        assert!(result1.is_ok(), "Large Vec data failed: {:?}", result1.err());
        println!("Large Vec processing: {:?}", vec_duration);
        std::fs::remove_file("test_large_vec.png").ok();
        
        // Array slice version (for reasonable size)
        let smaller_size = 10_000;
        let array_data: Vec<f64> = (0..smaller_size).map(|i| i as f64 * 0.01).collect();
        let slice_x = &array_data[..];
        let slice_y: Vec<f64> = slice_x.iter().map(|&x| x.cos()).collect();
        
        let start2 = std::time::Instant::now();
        let result2 = Plot::new()
            .line(slice_x, &slice_y)
            .title(&format!("Large Slice Data ({} points)", smaller_size))
            .save("test_large_slice.png");
        let slice_duration = start2.elapsed();
        
        assert!(result2.is_ok(), "Large slice data failed: {:?}", result2.err());
        println!("Large slice processing: {:?}", slice_duration);
        std::fs::remove_file("test_large_slice.png").ok();
    }

    /// Integration Test: Data validation and error handling
    #[test]
    fn test_data_validation_integration() {
        // Empty data should fail gracefully
        let empty_vec: Vec<f64> = vec![];
        let normal_vec = vec![1.0, 2.0, 3.0];
        
        let plot1 = Plot::new().line(&empty_vec, &empty_vec);
        let result1 = plot1.render();
        assert!(result1.is_err(), "Empty Vec should fail gracefully");
        
        // Mismatched lengths should fail
        let short_vec = vec![1.0, 2.0];
        let plot2 = Plot::new().line(&short_vec, &normal_vec);
        let result2 = plot2.render();
        assert!(result2.is_err(), "Mismatched lengths should fail gracefully");
        
        // NaN values handling
        let nan_vec = vec![1.0, f64::NAN, 3.0];
        let plot3 = Plot::new().line(&normal_vec, &nan_vec);
        let result3 = plot3.render();
        // Should either succeed (filtering NaN) or fail gracefully
        if result3.is_err() {
            println!("NaN values cause graceful failure (acceptable)");
        } else {
            println!("NaN values handled by filtering (good)");
        }
        
        // Infinite values handling
        let inf_vec = vec![1.0, f64::INFINITY, 3.0];
        let plot4 = Plot::new().line(&normal_vec, &inf_vec);
        let result4 = plot4.render();
        // Should either succeed (handling infinity) or fail gracefully
        if result4.is_err() {
            println!("Infinite values cause graceful failure (acceptable)");
        } else {
            println!("Infinite values handled properly (good)");
        }
    }

    /// Integration Test: Real-world data scenarios
    #[test]
    fn test_real_world_scenarios_integration() {
        // Scenario 1: Time series data (common in data science)
        let timestamps: Vec<f64> = (0..1000).map(|i| i as f64).collect();
        let temperature: Vec<f64> = timestamps.iter()
            .map(|&t| 20.0 + 5.0 * (t * 0.01).sin() + 0.5 * (t * 0.1).cos())
            .collect();
        let humidity: Vec<f64> = timestamps.iter()
            .map(|&t| 60.0 + 10.0 * (t * 0.008).cos() - 2.0 * (t * 0.05).sin())
            .collect();
        
        let result1 = Plot::new()
            .line(&timestamps, &temperature)
                .label("Temperature (°C)")
                .color(Color::RED)
            .line(&timestamps, &humidity)
                .label("Humidity (%)")
                .color(Color::BLUE)
            .title("Environmental Monitoring")
            .xlabel("Time (minutes)")
            .ylabel("Measurement")
            .legend(Position::TopRight)
            .save("test_timeseries.png");
        
        assert!(result1.is_ok(), "Time series scenario failed: {:?}", result1.err());
        std::fs::remove_file("test_timeseries.png").ok();
        
        // Scenario 2: Experimental data with measurements
        let voltages = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let currents = vec![0.0, 0.1, 0.19, 0.27, 0.34, 0.4]; // Non-linear relationship
        let power: Vec<f64> = voltages.iter()
            .zip(currents.iter())
            .map(|(&v, &i)| v * i)
            .collect();
        
        let result2 = Plot::new()
            .scatter(&voltages, &currents)
                .label("I-V Characteristic")
                .color(Color::GREEN)
            .line(&voltages, &power)
                .label("Power")
                .color(Color::from_hex("#FF6600"))
            .title("Electrical Measurements")
            .xlabel("Voltage (V)")
            .ylabel("Current (A) / Power (W)")
            .legend(Position::TopLeft)
            .save("test_experimental.png");
        
        assert!(result2.is_ok(), "Experimental scenario failed: {:?}", result2.err());
        std::fs::remove_file("test_experimental.png").ok();
    }

    /// Integration Test: Feature flag compatibility
    #[test]
    fn test_feature_flag_compatibility() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 4.0, 9.0];
        
        // This should work regardless of feature flags
        let result = Plot::new()
            .line(&x, &y)
            .title("Feature Compatibility Test")
            .save("test_features.png");
        
        assert!(result.is_ok(), "Basic functionality broken: {:?}", result.err());
        std::fs::remove_file("test_features.png").ok();
        
        // Test optional features
        #[cfg(feature = "ndarray_support")]
        println!("✅ ndarray support enabled");
        
        #[cfg(feature = "polars_support")]
        println!("✅ polars support enabled");
        
        #[cfg(not(any(feature = "ndarray_support", feature = "polars_support")))]
        println!("ℹ️  Running with minimal features (Vec/array only)");
    }
}