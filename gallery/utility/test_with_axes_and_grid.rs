use ruviz::prelude::*;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create output directory
    fs::create_dir_all("axes_test")?;
    
    println!("Testing axes and grid visibility...");
    
    // Test data
    let x_data: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
    let y_data: Vec<f64> = x_data.iter().map(|x| x.sin()).collect();
    
    // 1. Default plot (axes only, no grid)
    println!("Creating plot with axes only...");
    Plot::new()
        .title("Axes Only - Default".to_string())
        .xlabel("X Values".to_string())
        .ylabel("Y Values".to_string())
        .line(&x_data, &y_data)
        .end_series()
        .save("axes_test/01_axes_only.png")?;
    
    // 2. Plot with grid enabled
    println!("Creating plot with axes and grid...");
    Plot::new()
        .title("Axes + Grid Enabled".to_string())
        .xlabel("X Values".to_string())
        .ylabel("Y Values".to_string())
        .grid(true)  // Enable grid
        .line(&x_data, &y_data)
        .end_series()
        .save("axes_test/02_axes_and_grid.png")?;
    
    // 3. Multi-series with grid
    println!("Creating multi-series plot with grid...");
    let y2_data: Vec<f64> = x_data.iter().map(|x| x.cos()).collect();
    
    Plot::new()
        .title("Multi-Series with Grid".to_string())
        .xlabel("Time (s)".to_string())
        .ylabel("Amplitude".to_string())
        .grid(true)
        .line(&x_data, &y_data)
        .color(Color::new(255, 0, 0))  // Red for sine
        .end_series()
        .line(&x_data, &y2_data)
        .color(Color::new(0, 0, 255))  // Blue for cosine
        .end_series()
        .save("axes_test/03_multi_series_grid.png")?;
    
    // 4. Scatter plot with grid
    println!("Creating scatter plot with grid...");
    let scatter_x: Vec<f64> = (0..20).map(|i| i as f64).collect();
    let scatter_y: Vec<f64> = scatter_x.iter().map(|x| x * x * 0.1).collect();
    
    Plot::new()
        .title("Scatter Plot with Grid".to_string())
        .xlabel("Input Values".to_string())
        .ylabel("Squared Values".to_string())
        .grid(true)
        .scatter(&scatter_x, &scatter_y)
        .color(Color::new(0, 150, 0))  // Green
        .end_series()
        .save("axes_test/04_scatter_grid.png")?;
    
    println!("\n✅ Axes and grid test complete!");
    println!("📁 Check axes_test/ directory:");
    println!("  - 01_axes_only.png      (default - axes without grid)");
    println!("  - 02_axes_and_grid.png  (axes with grid enabled)");
    println!("  - 03_multi_series_grid.png (multiple series with grid)");
    println!("  - 04_scatter_grid.png   (scatter plot with grid)");
    
    Ok(())
}