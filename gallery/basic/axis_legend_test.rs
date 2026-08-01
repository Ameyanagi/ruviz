use ruviz::core::LegendPosition;
use ruviz::core::Plot;
use ruviz::render::Theme;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing axis labels and legends...");

    // Keep transient renders out of the source tree.
    std::fs::create_dir_all("generated/bench")?;

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
        .legend(LegendPosition::UpperRight) // Enable legend in top-right
        .save_with_size("generated/bench/axis_legend_test.png", 1200, 900)?;

    println!("✅ Axis and legend test completed!");
    println!("📂 Check generated/bench/axis_legend_test.png");

    Ok(())
}
