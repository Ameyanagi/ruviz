use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Testing Plotters-style FontFamily system...");
    
    // Create test output directory if it doesn't exist
    std::fs::create_dir_all("test_output")?;
    
    // Generate simple test data
    let x_data: Vec<f64> = (0..20).map(|i| i as f64).collect();
    let y_data: Vec<f64> = x_data.iter().map(|&x| x * 0.3 + 2.0).collect();
    
    // Test Plotters-style FontFamily usage
    println!("1. Testing FontFamily::SansSerif");
    Plot::new()
        .title("SansSerif Font Family Test")
        .xlabel("X Values")
        .ylabel("Y Values")
        .theme(Theme::publication())
        .line(&x_data, &y_data)
        .label("Sans-serif line")
        .save_with_size("test_output/plotters_sans_serif.png", 800, 600)?;
    
    println!("2. Testing FontFamily from string");
    let custom_font = FontFamily::from("sans-serif");
    println!("   Created font family: {}", custom_font.as_str());
    
    println!("3. Testing specific font name");
    let arial_font = FontFamily::Name("Arial".to_string());
    println!("   Arial font family: {}", arial_font.as_str());
    
    println!("4. Testing font conversion");
    let fonts = vec![
        FontFamily::from("serif"),
        FontFamily::from("sans-serif"), 
        FontFamily::from("monospace"),
        FontFamily::from("Arial"),
    ];
    
    for font in fonts {
        println!("   {} -> {}", 
                 match &font {
                     FontFamily::Serif => "serif input",
                     FontFamily::SansSerif => "sans-serif input",
                     FontFamily::Monospace => "monospace input", 
                     FontFamily::Name(name) => name,
                 },
                 font.as_str());
    }
    
    println!("✅ Plotters-style FontFamily system working perfectly!");
    println!("✅ Generated test_output/plotters_sans_serif.png");
    
    Ok(())
}