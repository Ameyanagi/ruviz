use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Testing axis labels and legends...");
    
    // Create test output directory if it doesn't exist
    std::fs::create_dir_all("test_output")?;
    
    // Generate simple test data
    let x_data: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
    let y1_data: Vec<f64> = x_data.iter().map(|&x| x.sin()).collect();
    let y2_data: Vec<f64> = x_data.iter().map(|&x| x.cos()).collect();
    
    // Create plot with labeled series
    Plot::new()
        .title("Test Plot with Axis Labels and Legend")
        .xlabel("X Axis (radians)")
        .ylabel("Y Axis (amplitude)")
        .theme(Theme::publication())
        .line(&x_data, &y1_data)
            .label("sin(x)")
            .end_series()
        .line(&x_data, &y2_data)  
            .label("cos(x)")
            .end_series()
        .save_with_size("test_output/test_axis_labels.png", 1200, 900)?;
    
    println!("✅ Generated test_output/test_axis_labels.png with axis labels and legend");
    
    Ok(())
}