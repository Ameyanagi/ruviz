use ruviz::prelude::*;
use std::fs;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Create output directory
    fs::create_dir_all("verify_axes")?;
    
    println!("🔍 Verifying axes and grid rendering...");
    
    // Simple test data
    let x_data = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y_data = vec![0.0, 1.0, 4.0, 9.0, 16.0];
    
    // 1. Plot with NO grid (grid explicitly disabled)
    println!("Creating plot with grid disabled...");
    Plot::new()
        .title("Grid Disabled".to_string())
        .grid(false)  // Explicitly disable
        .line(&x_data, &y_data)
        .end_series()
        .save("verify_axes/no_grid.png")?;
    
    // 2. Plot with grid enabled (default)
    println!("Creating plot with grid enabled...");
    Plot::new()
        .title("Grid Enabled".to_string())
        // grid should be true by default now
        .line(&x_data, &y_data)
        .end_series()
        .save("verify_axes/with_grid.png")?;
    
    // Check file sizes - they should be different if grid is actually rendered
    let no_grid_size = fs::metadata("verify_axes/no_grid.png")?.len();
    let with_grid_size = fs::metadata("verify_axes/with_grid.png")?.len();
    
    println!("\n📊 File size comparison:");
    println!("  No grid:   {} bytes", no_grid_size);
    println!("  With grid: {} bytes", with_grid_size);
    
    if no_grid_size == with_grid_size {
        println!("⚠️  WARNING: Files are the same size - grid may not be rendering!");
    } else {
        println!("✅ Files are different sizes - grid is being rendered!");
    }
    
    // 3. Create a minimal plot to test axes visibility
    println!("\nCreating minimal test plot...");
    let minimal_x = vec![0.0, 1.0];
    let minimal_y = vec![0.0, 1.0];
    
    Plot::new()
        .title("Minimal Test".to_string())
        .line(&minimal_x, &minimal_y)
        .color(Color::new(255, 0, 0)) // Bright red line
        .end_series()
        .save("verify_axes/minimal_test.png")?;
    
    println!("✅ Verification complete!");
    println!("📁 Check verify_axes/ directory for visual verification");
    
    Ok(())
}