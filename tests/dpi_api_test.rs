//! Test-driven development for DPI-aware save API
//! 
//! Tests the .dpi(u32).save("") fluent API for scientific plotting

use ruviz::prelude::*;
use std::fs;

/// Setup test output directory
fn setup_test_output_dir() -> std::result::Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("test_output")?;
    Ok(())
}

#[test]
fn test_dpi_fluent_api_basic() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;
    
    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];
    
    // Test the fluent API: .dpi(300).save()
    Plot::new()
        .title("DPI Test - 300 DPI")
        .xlabel("X Axis")
        .ylabel("Y Axis")
        .line(&x_data, &y_data)
        .dpi(300)  // This should fail initially - method doesn't exist yet
        .save("test_output/dpi_300_test.png")?;
    
    println!("✓ Saved: test_output/dpi_300_test.png at 300 DPI");
    Ok(())
}

#[test]
fn test_ieee_publication_dpi() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;
    
    let x_data: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y_data: Vec<f64> = x_data.iter().map(|x| x.sin()).collect();
    
    // IEEE standard: 600 DPI for publication
    Plot::new()
        .title("IEEE Publication Quality")
        .xlabel("Time (s)")
        .ylabel("Amplitude")
        .line(&x_data, &y_data)
        .dpi(600)  // IEEE requirement
        .save("test_output/ieee_600_dpi_test.png")?;
    
    println!("✓ Saved: test_output/ieee_600_dpi_test.png at 600 DPI (IEEE standard)");
    Ok(())
}

#[test]
fn test_multiple_dpi_outputs() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;
    
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y_data = vec![0.0, 1.0, 4.0, 9.0, 16.0];
    
    let base_plot = Plot::new()
        .title("Multi-DPI Test")
        .xlabel("X Values")
        .ylabel("X²")
        .line(&x_data, &y_data);
    
    // Test different DPI values
    base_plot.clone().dpi(96).save("test_output/multi_dpi_96_test.png")?;
    base_plot.clone().dpi(150).save("test_output/multi_dpi_150_test.png")?;
    base_plot.clone().dpi(300).save("test_output/multi_dpi_300_test.png")?;
    
    println!("✓ Saved multiple DPI versions: 96, 150, 300 DPI");
    Ok(())
}

#[test]
fn test_dpi_with_theme() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;
    
    let x_data = vec![1.0, 2.0, 3.0];
    let y_data = vec![2.0, 4.0, 6.0];
    
    // Combine DPI with theme for scientific plotting
    Plot::new()
        .theme(Theme::publication())  // Assuming this exists
        .title("Publication Theme with High DPI")
        .xlabel("Input")
        .ylabel("Output")
        .line(&x_data, &y_data)
        .dpi(300)
        .save("test_output/theme_with_dpi_test.png")?;
    
    println!("✓ Saved: test_output/theme_with_dpi_test.png with publication theme at 300 DPI");
    Ok(())
}

#[test]
fn test_dpi_validation() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;
    
    let x_data = vec![1.0, 2.0];
    let y_data = vec![1.0, 2.0];
    
    // Test minimum DPI validation (should clamp to 72 minimum)
    Plot::new()
        .title("DPI Validation Test")
        .line(&x_data, &y_data)
        .dpi(50)  // Too low, should be clamped to 72
        .save("test_output/dpi_validation_test.png")?;
    
    println!("✓ Saved: test_output/dpi_validation_test.png with validated DPI");
    Ok(())
}

#[test]
fn test_scientific_dpi_presets() -> std::result::Result<(), Box<dyn std::error::Error>> {
    setup_test_output_dir()?;
    
    let x_data = vec![0.0, 0.5, 1.0, 1.5, 2.0];
    let y_data = vec![0.0, 0.25, 1.0, 2.25, 4.0];
    
    // Test scientific DPI presets
    let plot = Plot::new()
        .title("Scientific DPI Presets")
        .xlabel("x")
        .ylabel("x²")
        .line(&x_data, &y_data);
    
    // Screen quality (96 DPI)
    plot.clone().dpi(96).save("test_output/scientific_screen_96_test.png")?;
    
    // Web quality (150 DPI) 
    plot.clone().dpi(150).save("test_output/scientific_web_150_test.png")?;
    
    // Print quality (300 DPI)
    plot.clone().dpi(300).save("test_output/scientific_print_300_test.png")?;
    
    // IEEE publication (600 DPI)
    plot.clone().dpi(600).save("test_output/scientific_ieee_600_test.png")?;
    
    println!("✓ Saved scientific DPI preset tests: 96, 150, 300, 600 DPI");
    Ok(())
}