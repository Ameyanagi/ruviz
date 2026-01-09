use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Testing different font families...");
    
    // Create test output directory if it doesn't exist
    std::fs::create_dir_all("test_output")?;
    
    // Generate simple test data
    let x_data: Vec<f64> = (0..20).map(|i| i as f64).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| x * 0.5).collect();
    
    // Test different font families
    let font_tests = vec![
        ("sans_serif", FontFamily::SansSerif),
        ("serif", FontFamily::Serif),
        ("monospace", FontFamily::Monospace),
        ("arial", FontFamily::Name("Arial".to_string())),
    ];
    
    for (name, font_family) in font_tests {
        println!("Testing {} font...", name);

        Plot::new()
            .title(&format!("Font Test: {} Family", name))
            .xlabel("X Values")
            .ylabel("Y Values")
            .theme(Theme::publication())
            .line(&x_data, &y_data)
            .label(&format!("{} line", name))
            .save_with_size(&format!("test_output/font_test_{}.png", name), 800, 600)?;

        println!("✅ Generated test_output/font_test_{}.png", name);
    }
    
    Ok(())
}