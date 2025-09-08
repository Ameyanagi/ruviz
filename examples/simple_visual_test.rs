use ruviz::prelude::*;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("visual_test")?;
    
    println!("Creating high-contrast visual test...");
    
    // Very simple data - just two points
    let x_data = vec![1.0, 5.0];
    let y_data = vec![1.0, 5.0];
    
    // Create a plot with high contrast
    Plot::new()
        .title("Visual Test - Should See Axes and Grid".to_string())
        .line(&x_data, &y_data)
        .color(Color::new(255, 0, 0)) // Bright red line
        .end_series()
        .save("visual_test/high_contrast.png")?;
    
    println!("✅ Created visual_test/high_contrast.png");
    println!("📋 This plot should show:");
    println!("   - Red line from bottom-left to top-right");
    println!("   - Black axes (X and Y axis lines)");
    println!("   - Gray grid lines");
    println!("   - White background");
    
    // Also create one without any data to see just axes/grid
    Plot::new()
        .title("Empty Plot - Axes and Grid Only".to_string())
        .dimensions((400, 300))
        .line(&vec![0.0], &vec![0.0]) // Single point (will be barely visible)
        .end_series()
        .save("visual_test/axes_only.png")?;
    
    println!("✅ Created visual_test/axes_only.png (shows just axes/grid)");
    
    Ok(())
}