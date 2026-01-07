// Multi-Series Integration Test (Story 2)
// "Given I need to compare multiple data series, When I add multiple plot types 
// with different colors and a legend, Then the library produces a readable 
// multi-series visualization with automatic color management"

use ruviz::prelude::*;
use std::path::Path;

#[cfg(test)]
mod multi_series_integration_tests {
    use super::*;

    /// Integration Test: Story 2 - Multi-series plot with legend and colors
    #[test]
    fn test_story_2_multi_series_integration() {
        // Given: I need to compare multiple data series
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let linear = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let quadratic = vec![0.0, 1.0, 4.0, 9.0, 16.0];
        let scatter_x = vec![0.5, 1.5, 2.5, 3.5];
        let scatter_y = vec![1.0, 3.5, 7.0, 12.5];
        
        // When: I add multiple plot types with different colors and a legend
        // Will fail until multiple chaining, label(), color(), legend(), grid() are implemented
        let result = Plot::new()
            .line(&x, &linear)
                .label("Linear")
                .color(Color::BLUE)
            .line(&x, &quadratic)
                .label("Quadratic")
                .color(Color::RED)
            .scatter(&scatter_x, &scatter_y)
                .label("Data Points") 
                .color(Color::GREEN)
            .title("Function Comparison")
            .xlabel("x")
            .ylabel("f(x)")
            .legend(Position::TopLeft)
            .grid(true)
            .save("test_multi_series_integration.png");
        
        // Then: The library produces a readable multi-series visualization
        assert!(result.is_ok(), "Multi-series plot creation failed: {:?}", result.err());
        
        // Verify the file was created
        assert!(Path::new("test_multi_series_integration.png").exists(),
                "Multi-series PNG file was not created");
        
        // Verify file is substantial (should be larger than single series)
        let metadata = std::fs::metadata("test_multi_series_integration.png")
            .expect("Could not read file metadata");
        assert!(metadata.len() > 1000, "Multi-series PNG seems too small: {} bytes", metadata.len());
        
        // Clean up test file
        std::fs::remove_file("test_multi_series_integration.png").ok();
    }

    /// Integration Test: Multiple lines with automatic color cycling
    #[test]
    fn test_automatic_color_management_integration() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y1 = vec![1.0, 2.0, 3.0, 4.0];
        let y2 = vec![1.0, 4.0, 9.0, 16.0]; 
        let y3 = vec![1.0, 8.0, 27.0, 64.0];
        let y4 = vec![2.0, 4.0, 6.0, 8.0];
        
        // Test automatic color assignment when colors not specified
        let result = Plot::new()
            .line(&x, &y1).label("Linear")
            .line(&x, &y2).label("Quadratic")  
            .line(&x, &y3).label("Cubic")
            .line(&x, &y4).label("Even")
            .legend(Position::TopRight)
            .title("Automatic Colors")
            .save("test_auto_colors.png");
            
        assert!(result.is_ok(), "Automatic color cycling failed: {:?}", result.err());
        std::fs::remove_file("test_auto_colors.png").ok();
    }

    /// Integration Test: Mixed plot types (lines + scatter + bars) 
    #[test]
    fn test_mixed_plot_types_integration() {
        let x_continuous = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y_line = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        
        let x_scatter = vec![0.5, 1.5, 2.5, 3.5];
        let y_scatter = vec![3.0, 7.0, 5.0, 9.0];
        
        let categories = vec!["A", "B", "C"];
        let values = vec![5.0, 8.0, 6.0];
        
        let result = Plot::new()
            .line(&x_continuous, &y_line)
                .label("Trend Line")
                .color(Color::BLUE)
                .style(LineStyle::Dashed)
            .scatter(&x_scatter, &y_scatter)
                .label("Measurements")
                .color(Color::RED)
            .bar(&categories, &values)
                .label("Categories")
                .color(Color::GREEN)
            .legend(Position::BottomRight)
            .title("Mixed Plot Types")
            .save("test_mixed_types.png");
            
        assert!(result.is_ok(), "Mixed plot types failed: {:?}", result.err());
        std::fs::remove_file("test_mixed_types.png").ok();
    }

    /// Integration Test: Legend positioning variations
    #[test]
    fn test_legend_positions_integration() {
        let x = vec![1.0, 2.0, 3.0];
        let y1 = vec![1.0, 2.0, 3.0];
        let y2 = vec![3.0, 2.0, 1.0];
        
        let positions = [
            Position::TopLeft,
            Position::TopRight, 
            Position::BottomLeft,
            Position::BottomRight,
        ];
        
        for (i, &pos) in positions.iter().enumerate() {
            let result = Plot::new()
                .line(&x, &y1).label("Series A").color(Color::BLUE)
                .line(&x, &y2).label("Series B").color(Color::RED)
                .legend(pos)
                .title(&format!("Legend Position {:?}", pos))
                .save(&format!("test_legend_{}.png", i));
                
            assert!(result.is_ok(), "Legend position {:?} failed: {:?}", pos, result.err());
            std::fs::remove_file(&format!("test_legend_{}.png", i)).ok();
        }
    }

    /// Integration Test: Grid styles and customization
    #[test]
    fn test_grid_customization_integration() {
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
        let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];
        
        // Test grid enabled
        let result1 = Plot::new()
            .line(&x, &y)
            .grid(true)
            .title("Grid Enabled")
            .save("test_grid_enabled.png");
        assert!(result1.is_ok(), "Grid enabled failed: {:?}", result1.err());
        std::fs::remove_file("test_grid_enabled.png").ok();
        
        // Test grid disabled  
        let result2 = Plot::new()
            .line(&x, &y)
            .grid(false)
            .title("Grid Disabled")
            .save("test_grid_disabled.png");
        assert!(result2.is_ok(), "Grid disabled failed: {:?}", result2.err());
        std::fs::remove_file("test_grid_disabled.png").ok();
        
        // Test custom grid styling
        let result3 = Plot::new()
            .line(&x, &y)
            .grid(true)
            .grid_color(Color::from_hex("#CCCCCC"))
            .grid_style(LineStyle::Dotted)
            .title("Custom Grid")
            .save("test_custom_grid.png");
        assert!(result3.is_ok(), "Custom grid failed: {:?}", result3.err());
        std::fs::remove_file("test_custom_grid.png").ok();
    }

    /// Integration Test: Many series stress test
    #[test]
    fn test_many_series_integration() {
        let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
        
        let mut plot = Plot::new()
            .title("Multiple Series Stress Test")
            .xlabel("x")
            .ylabel("y");
        
        // Add 10 different series  
        for i in 0..10 {
            let phase = i as f64 * 0.5;
            let amplitude = (i + 1) as f64;
            let y: Vec<f64> = x.iter()
                .map(|&x| amplitude * (x + phase).sin())
                .collect();
                
            plot = plot.line(&x, &y)
                .label(&format!("Series {}", i + 1));
        }
        
        let result = plot
            .legend(Position::TopRight)
            .save("test_many_series.png");
            
        assert!(result.is_ok(), "Many series test failed: {:?}", result.err());
        std::fs::remove_file("test_many_series.png").ok();
    }

    /// Integration Test: Complex real-world scenario
    #[test]
    fn test_complex_scenario_integration() {
        // Simulated sensor data over time
        let time: Vec<f64> = (0..200).map(|i| i as f64 * 0.1).collect();
        
        let temperature: Vec<f64> = time.iter()
            .map(|&t| 20.0 + 5.0 * (t * 0.1).sin() + (t * 0.02).cos())
            .collect();
            
        let humidity: Vec<f64> = time.iter()
            .map(|&t| 60.0 + 10.0 * (t * 0.08).cos() - (t * 0.03).sin())
            .collect();
        
        let pressure: Vec<f64> = time.iter()
            .map(|&t| 1013.25 + 2.0 * (t * 0.05).sin())
            .collect();
        
        // Calibration points (scatter)
        let cal_time = vec![5.0, 10.0, 15.0];
        let cal_temp = vec![22.5, 24.0, 21.5];
        
        let result = Plot::new()
            .line(&time, &temperature)
                .label("Temperature (°C)")
                .color(Color::RED)
                .line_width(2.0)
            .line(&time, &humidity)
                .label("Humidity (%)")
                .color(Color::BLUE)
                .line_width(2.0)
                .style(LineStyle::Dashed)
            .line(&time, &pressure)
                .label("Pressure (hPa)")
                .color(Color::GREEN)
                .line_width(1.5)
            .scatter(&cal_time, &cal_temp)
                .label("Calibration Points")
                .color(Color::from_hex("#FF6600"))
            .title("Environmental Sensor Data")
            .xlabel("Time (minutes)")
            .ylabel("Measurement Value")
            .legend(Position::TopRight)
            .grid(true)
            .save("test_complex_scenario.png");
            
        assert!(result.is_ok(), "Complex scenario failed: {:?}", result.err());
        
        // Verify file is substantial for complex plot
        let metadata = std::fs::metadata("test_complex_scenario.png")
            .expect("Could not read complex scenario file");
        assert!(metadata.len() > 5000, "Complex plot file seems too small");
        
        std::fs::remove_file("test_complex_scenario.png").ok();
    }
}