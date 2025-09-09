//! T009: API contract test: Builder pattern with method chaining
//! 
//! This test validates the fluent API design and builder pattern
//! All API methods must return &mut self for chaining

use ruviz::core::Plot;
use criterion::black_box;

/// Builder pattern contract for fluent API design
/// All methods MUST return &mut self for method chaining
#[test]
fn builder_pattern_fluent_api_contract() {
    println!("🧪 API BUILDER CONTRACT: Fluent method chaining validation");
    
    // Generate test data
    let x_data: Vec<f64> = (0..1000).map(|i| i as f64 * 0.01).collect();
    let y_data: Vec<f64> = (0..1000).map(|i| (i as f64 * 0.01).sin()).collect();
    let y_errors: Vec<f64> = (0..1000).map(|_| 0.1).collect();
    let categories = vec!["A", "B", "C", "D", "E"];
    let values: Vec<f64> = vec![10.0, 20.0, 15.0, 25.0, 18.0];
    
    // Test 1: Complete fluent chain for line plot
    println!("🔗 Testing fluent API chain - Line plot");
    
    let line_plot = Plot::new()
        .line(&x_data, &y_data)
        .title("Builder Pattern Test - Line Plot")
        .xlabel("Time (s)")
        .ylabel("Amplitude")
        .xlim(0.0, 10.0)
        .ylim(-2.0, 2.0)
        .grid(true)
        .legend(true)
        .theme("dark")
        .width(800)
        .height(600)
        .dpi(150)
        .save("test_output/contract_builder_line.png");
    
    assert!(line_plot.is_ok(), "Fluent line plot chain should succeed");
    println!("✅ Line plot fluent chain completed");
    
    // Test 2: Complete fluent chain for scatter plot
    println!("🔗 Testing fluent API chain - Scatter plot");
    
    let scatter_plot = Plot::new()
        .scatter(&x_data, &y_data)
        .title("Builder Pattern Test - Scatter Plot")
        .xlabel("X Coordinate")
        .ylabel("Y Coordinate")
        .color("red")
        .alpha(0.7)
        .marker_size(3.0)
        .xlim(-1.0, 11.0)
        .ylim(-1.5, 1.5)
        .grid(true)
        .legend(false)
        .save("test_output/contract_builder_scatter.png");
    
    assert!(scatter_plot.is_ok(), "Fluent scatter plot chain should succeed");
    println!("✅ Scatter plot fluent chain completed");
    
    // Test 3: Complete fluent chain for bar plot
    println!("🔗 Testing fluent API chain - Bar plot");
    
    let bar_plot = Plot::new()
        .bar(&categories, &values)
        .title("Builder Pattern Test - Bar Plot")
        .xlabel("Categories")
        .ylabel("Values")
        .color("blue")
        .alpha(0.8)
        .width(600)
        .height(400)
        .save("test_output/contract_builder_bar.png");
    
    assert!(bar_plot.is_ok(), "Fluent bar plot chain should succeed");
    println!("✅ Bar plot fluent chain completed");
    
    // Test 4: Error bars with fluent chain
    println!("🔗 Testing fluent API chain - Error bars");
    
    let error_plot = Plot::new()
        .error_bars(&x_data, &y_data, &y_errors)
        .title("Builder Pattern Test - Error Bars")
        .xlabel("X Values")
        .ylabel("Y Values ± Error")
        .color("green")
        .grid(true)
        .save("test_output/contract_builder_errors.png");
    
    assert!(error_plot.is_ok(), "Fluent error bars chain should succeed");
    println!("✅ Error bars fluent chain completed");
    
    println!("✅ All fluent API chains completed successfully");
}

/// Test method chaining return types
#[test]
fn method_chaining_return_types() {
    println!("🧪 RETURN TYPES: Validating builder method return types");
    
    let x_data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data: Vec<f64> = vec![2.0, 4.0, 6.0, 8.0, 10.0];
    
    // This compilation test validates that all methods return the correct type
    // for chaining. If any method doesn't return &mut Self, this won't compile.
    
    let mut plot = Plot::new();
    
    // Test that we can assign the result of each method and continue chaining
    plot = plot.title("Return Type Test");
    plot = plot.xlabel("X Axis");  
    plot = plot.ylabel("Y Axis");
    plot = plot.width(800);
    plot = plot.height(600);
    plot = plot.dpi(150);
    plot = plot.grid(true);
    plot = plot.legend(true);
    plot = plot.theme("light");
    plot = plot.color("red");
    plot = plot.alpha(0.5);
    plot = plot.xlim(0.0, 10.0);
    plot = plot.ylim(0.0, 10.0);
    
    // Add data and chain
    plot = plot.line(&x_data, &y_data);
    plot = plot.scatter(&x_data, &y_data);
    
    // Final save operation
    let result = plot.save("test_output/return_types_test.png");
    
    assert!(result.is_ok(), "Method chaining should work with individual assignments");
    println!("✅ All methods return correct type for chaining");
}

/// Test builder pattern state preservation
#[test]
fn builder_state_preservation() {
    println!("🧪 STATE PRESERVATION: Testing builder state across method calls");
    
    let x_data: Vec<f64> = vec![1.0, 2.0, 3.0];
    let y_data: Vec<f64> = vec![1.0, 4.0, 9.0];
    
    // Build plot step by step and validate state is preserved
    let mut plot = Plot::new();
    
    // Set title and verify it's preserved through other operations
    plot = plot.title("State Preservation Test");
    plot = plot.xlabel("X Values");
    plot = plot.ylabel("Y Values");
    plot = plot.width(400);
    plot = plot.height(300);
    
    // Add data
    plot = plot.line(&x_data, &y_data);
    
    // Continue modifying and ensure previous settings are maintained
    plot = plot.color("purple");
    plot = plot.grid(true);
    plot = plot.xlim(0.0, 4.0);
    plot = plot.ylim(0.0, 10.0);
    
    // Save and verify all settings were applied
    let result = plot.save("test_output/state_preservation_test.png");
    assert!(result.is_ok(), "State preservation should allow successful plot creation");
    
    println!("✅ Builder state preserved across method calls");
}

/// Test multiple data series with builder pattern
#[test]
fn multiple_series_builder() {
    println!("🧪 MULTIPLE SERIES: Testing builder with multiple data series");
    
    // Create multiple datasets
    let x1: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
    let y1: Vec<f64> = (0..50).map(|i| (i as f64 * 0.1).sin()).collect();
    
    let x2: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect(); 
    let y2: Vec<f64> = (0..50).map(|i| (i as f64 * 0.1).cos()).collect();
    
    let x3: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
    let y3: Vec<f64> = (0..50).map(|i| (i as f64 * 0.1).tan().min(2.0).max(-2.0)).collect();
    
    // Test fluent chaining with multiple series
    let multi_series_plot = Plot::new()
        .title("Multiple Series Builder Test")
        .xlabel("X Values")
        .ylabel("Trigonometric Functions")
        .line(&x1, &y1)    // First series
        .line(&x2, &y2)    // Second series  
        .scatter(&x3, &y3) // Third series with different type
        .grid(true)
        .legend(true)
        .width(800)
        .height(500)
        .save("test_output/multiple_series_builder.png");
    
    assert!(multi_series_plot.is_ok(), "Multiple series with builder should succeed");
    println!("✅ Multiple series builder pattern works");
}

/// Test builder pattern with advanced styling
#[test]
fn advanced_styling_builder() {
    println!("🧪 ADVANCED STYLING: Testing builder with complex styling options");
    
    let x_data: Vec<f64> = (0..100).map(|i| i as f64 * 0.05).collect();
    let y_data: Vec<f64> = (0..100).map(|i| (i as f64 * 0.05).exp().min(50.0)).collect();
    
    // Test advanced styling chain
    let styled_plot = Plot::new()
        .title("Advanced Styling Builder Test")
        .xlabel("Time")
        .ylabel("Exponential Growth") 
        .line(&x_data, &y_data)
        .color("darkblue")
        .line_width(2.5)
        .alpha(0.9)
        .marker_size(4.0)
        .grid(true)
        .grid_alpha(0.3)
        .legend(true)
        .legend_position("upper left")
        .theme("publication")
        .background_color("white")
        .xlim(0.0, 5.0)
        .ylim(0.0, 50.0)
        .width(1000)
        .height(700)
        .dpi(200)
        .margin(0.1)
        .save("test_output/advanced_styling_builder.png");
    
    assert!(styled_plot.is_ok(), "Advanced styling builder should succeed");
    println!("✅ Advanced styling builder pattern works");
}

/// Test builder pattern error handling
#[test]
fn builder_error_handling() {
    println!("🧪 ERROR HANDLING: Testing builder pattern with invalid inputs");
    
    let x_data: Vec<f64> = vec![1.0, 2.0, 3.0];
    let y_data: Vec<f64> = vec![1.0, 2.0]; // Mismatched length
    
    // Test that builder handles errors gracefully
    let mismatched_result = Plot::new()
        .title("Error Handling Test")
        .line(&x_data, &y_data) // This should cause an error due to length mismatch
        .save("test_output/error_handling_test.png");
    
    // Should fail due to data length mismatch
    assert!(mismatched_result.is_err(), "Mismatched data lengths should cause error");
    
    println!("✅ Builder pattern handles data errors correctly");
    
    // Test empty data
    let empty_x: Vec<f64> = vec![];
    let empty_y: Vec<f64> = vec![];
    
    let empty_result = Plot::new()
        .title("Empty Data Test")
        .scatter(&empty_x, &empty_y)
        .save("test_output/empty_data_test.png");
    
    assert!(empty_result.is_err(), "Empty data should cause error");
    println!("✅ Builder pattern handles empty data correctly");
    
    // Test invalid dimensions
    let valid_x: Vec<f64> = vec![1.0, 2.0, 3.0];
    let valid_y: Vec<f64> = vec![1.0, 2.0, 3.0];
    
    let invalid_dims_result = Plot::new()
        .title("Invalid Dimensions Test")
        .line(&valid_x, &valid_y)
        .width(0)  // Invalid width
        .height(0) // Invalid height
        .save("test_output/invalid_dims_test.png");
    
    assert!(invalid_dims_result.is_err(), "Invalid dimensions should cause error");
    println!("✅ Builder pattern handles invalid dimensions correctly");
}

/// Performance test for builder pattern overhead
#[test]
fn builder_performance_overhead() {
    println!("🧪 PERFORMANCE: Testing builder pattern overhead");
    
    use std::time::Instant;
    
    let x_data: Vec<f64> = (0..10000).map(|i| i as f64).collect();
    let y_data: Vec<f64> = (0..10000).map(|i| (i as f64 * 0.001).sin()).collect();
    
    let iterations = 100;
    let start = Instant::now();
    
    for i in 0..iterations {
        let plot = Plot::new()
            .title(&format!("Performance Test {}", i))
            .xlabel("X Data")
            .ylabel("Y Data")
            .line(&x_data, &y_data)
            .color("blue")
            .alpha(0.8)
            .grid(true)
            .width(800)
            .height(600);
        
        // Don't save, just build the plot
        black_box(plot);
    }
    
    let duration = start.elapsed();
    let avg_time = duration.as_millis() as f64 / iterations as f64;
    
    println!("⏱️  {} iterations: {:?} (avg: {:.2}ms per plot)", iterations, duration, avg_time);
    
    // Builder pattern should have minimal overhead
    assert!(avg_time < 10.0, "Builder pattern overhead should be <10ms per plot, got {:.2}ms", avg_time);
    
    println!("✅ Builder pattern has acceptable performance overhead");
}