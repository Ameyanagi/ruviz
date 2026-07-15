use ruviz::core::Plot;
use ruviz::render::Theme;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🖼️ Simple Publication Test");

    // Create output directory
    std::fs::create_dir_all("gallery/test")?;

    // Generate simple test data
    let x_data: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| x.sin()).collect();

    // Exact-pixel export. This does not claim a physical size or DPI.
    println!("Testing 1200x900 exact-pixel export...");
    Plot::new()
        .title("Publication Test - Sin Wave".to_string())
        .xlabel("Time".to_string())
        .ylabel("Amplitude".to_string())
        .line(&x_data, &y_data)
        .save_with_size("gallery/publication/simple_publication_test.png", 1200, 900)?;

    // Test theme method
    println!("🎨 Testing theme() method...");
    Plot::new()
        .title("Publication Theme Test".to_string())
        .xlabel("X Axis".to_string())
        .ylabel("Y Axis".to_string())
        .theme(Theme::publication())
        .line(&x_data, &y_data)
        .save("gallery/publication/simple_publication_test_theme.png")?;

    println!("✅ Simple publication test completed!");
    println!("📂 Check ./gallery/publication/ for generated images");

    Ok(())
}
