use ruviz::core::Plot;
use ruviz::core::Position;
use ruviz::render::Theme;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing axis labels and legends...");

    // Create output directory
    std::fs::create_dir_all("gallery/test")?;

    // Generate test data
    let x_data: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y1: Vec<f64> = x_data.iter().map(|&x| x.sin()).collect();
    let y2: Vec<f64> = x_data.iter().map(|&x| (x * 1.5).cos()).collect();

    // Test with multiple series to see if legends work
    println!("📊 Testing multi-series plot with legend...");
    Plot::new()
        .title("Axis Labels and Legend Test".to_string())
        .xlabel("Time (seconds)".to_string())
        .ylabel("Amplitude".to_string())
        .theme(Theme::publication())
        .line(&x_data, &y1) // First series
        .line(&x_data, &y2) // Second series
        .legend(Position::TopRight) // Enable legend in top-right
        .save_with_size("gallery/basic/axis_legend_test.png", 1200, 900)?;

    println!("✅ Axis and legend test completed!");
    println!("📂 Check ./gallery/basic/axis_legend_test.png");

    Ok(())
}
