use ruviz::prelude::*;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("test_output")?;

    println!("Creating high-contrast visual test with cosmic-text...");

    // Very simple data - just two points
    let x_data = vec![1.0, 5.0];
    let y_data = vec![1.0, 5.0];

    // Create a plot with high contrast
    Plot::new()
        .title("Visual Test - Cosmic-Text Rendering".to_string())
        .line(&x_data, &y_data)
        .color(Color::new(255, 0, 0)) // Bright red line
        .end_series()
        .save("gallery/basic/simple_visual_test.png")?;

    println!("✅ Created gallery/basic/simple_visual_test.png");
    println!("📋 This plot should show:");
    println!("   - Red line from bottom-left to top-right");
    println!("   - Black axes with professional Roboto font");
    println!("   - Gray grid lines");
    println!("   - White background");
    println!("   - High-quality text rendering with cosmic-text");

    // Also create one without any data to see just axes/grid
    Plot::new()
        .title("Cosmic-Text Typography Demo".to_string())
        .dimensions(400, 300)
        .line(&vec![0.0], &vec![0.0]) // Single point (will be barely visible)
        .end_series()
        .save("gallery/basic/simple_visual_test_typography.png")?;

    println!(
        "✅ Created gallery/basic/simple_visual_test_typography.png (shows professional text rendering)"
    );

    Ok(())
}
