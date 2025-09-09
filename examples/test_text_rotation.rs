use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing text rotation and UTF-8 support...");
    
    // Create test output directory if it doesn't exist
    std::fs::create_dir_all("test_output")?;
    
    // Generate simple test data
    let x_data: Vec<f64> = (0..20).map(|i| i as f64).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| x * 0.5).collect();
    
    // Create plot with UTF-8 characters and rotated Y-axis labels
    Plot::new()
        .title("Test Text Rotation & UTF-8 (日本語)")
        .xlabel("X軸 (X-axis)")
        .ylabel("振幅 (Amplitude)")
        .theme(Theme::publication())
        .line(&x_data, &y_data)
            .label("テストライン (Test Line)")
            .end_series()
        .save_with_size("test_output/test_text_rotation.png", 1000, 700)?;
    
    println!("✅ Generated test_output/test_text_rotation.png with UTF-8 text and rotation");
    
    Ok(())
}