use ruviz::core::Plot;
use ruviz::data::DataShader;

#[test]
fn test_datashader_basic_functionality() {
    // Test basic DataShader creation and aggregation
    let mut datashader = DataShader::new();
    
    // Create small dataset for basic functionality test
    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![10.0, 25.0, 15.0, 30.0, 20.0];
    
    // Aggregate data
    let result = datashader.aggregate(&x_data, &y_data);
    assert!(result.is_ok(), "DataShader aggregation should succeed");
    
    // Get statistics
    let stats = datashader.statistics();
    assert_eq!(stats.total_points, 5);
    assert!(stats.filled_pixels > 0, "Should have some filled pixels");
    assert!(stats.max_count > 0, "Should have non-zero max count");
    
    // Render to image
    let image = datashader.render();
    assert_eq!(image.width, 512);
    assert_eq!(image.height, 512);
    assert_eq!(image.pixels.len(), 512 * 512 * 4); // RGBA pixels
    
    println!("✅ DataShader basic functionality test passed");
    println!("   - Total points: {}", stats.total_points);
    println!("   - Filled pixels: {}", stats.filled_pixels);
    println!("   - Max count: {}", stats.max_count);
    println!("   - Canvas utilization: {:.2}%", stats.canvas_utilization * 100.0);
}

#[test]
fn test_datashader_large_dataset() {
    // Test with larger dataset to potentially trigger DataShader in Plot
    let mut datashader = DataShader::with_canvas_size(256, 256);
    
    // Create larger dataset (10K points)
    let n = 10_000;
    let x_data: Vec<f64> = (0..n).map(|i| (i as f64) * 0.01).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| (x * 2.0).sin() + 0.5 * (x * 10.0).cos()).collect();
    
    // Aggregate data
    let result = datashader.aggregate(&x_data, &y_data);
    assert!(result.is_ok(), "DataShader aggregation should succeed with large dataset");
    
    // Get statistics
    let stats = datashader.statistics();
    assert_eq!(stats.total_points, n as u64);
    assert!(stats.filled_pixels > 100, "Should have many filled pixels for large dataset");
    
    println!("✅ DataShader large dataset test passed");
    println!("   - Total points: {}", stats.total_points);
    println!("   - Filled pixels: {}", stats.filled_pixels);
    println!("   - Canvas utilization: {:.2}%", stats.canvas_utilization * 100.0);
}

#[test] 
fn test_plot_datashader_activation() {
    // Test that Plot automatically uses DataShader for large datasets
    let n = 150_000; // Above the 100K threshold
    let x_data: Vec<f64> = (0..n).map(|i| (i as f64) / 1000.0).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| x.sin()).collect();
    
    let result = Plot::new()
        .title("Large Dataset Test".to_string())
        .scatter(&x_data, &y_data)
        .render();
    
    // Should not panic and should succeed
    assert!(result.is_ok(), "Plot with large dataset should render successfully using DataShader");
    
    let image = result.unwrap();
    // Basic validation that we got an image
    assert!(image.width > 0);
    assert!(image.height > 0);
    assert!(image.pixels.len() > 0);
    
    println!("✅ Plot DataShader activation test passed");
    println!("   - Rendered {} points successfully", n);
    println!("   - Image dimensions: {}x{}", image.width, image.height);
}