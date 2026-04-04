use ruviz::prelude::*;
use std::fs;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Creating cosmic-text rotation demonstration...");
    
    // Ensure test output directory exists
    fs::create_dir_all("generated/bench")?;
    
    // Simple test data
    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let y_data = vec![10.0, 25.0, 15.0, 30.0, 35.0, 20.0];
    
    // Create plot showcasing cosmic-text's international character support
    Plot::new()
        .title("Cosmic-Text International Font Support 🌍")
        .xlabel("English + 中文 + العربية + Русский")
        .ylabel("Français + Deutsch + 日本語 + हिन्दी")
        .line(&x_data, &y_data)
        .save("generated/bench/cosmic_text_international_demo.png")?;
    
    println!("✅ Created cosmic-text international character demo");
    
    // Create another demo with scientific notation and symbols
    let scientific_x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
    let scientific_y: Vec<f64> = scientific_x.iter().map(|&x| (x * 2.0 * std::f64::consts::PI).sin()).collect();
    
    Plot::new()
        .title("Professional Typography: f(x) = sin(2πx)")
        .xlabel("Time (μs) → Advanced Typography")
        .ylabel("Amplitude (mV) ↑ Professional Rendering")
        .theme(Theme::publication())
        .line(&scientific_x, &scientific_y)
        .save("generated/bench/cosmic_text_scientific_notation.png")?;
    
    println!("✅ Created cosmic-text scientific notation demo");
    println!("🎯 Both plots showcase cosmic-text's professional typography:");
    println!("   - International character support (UTF-8)");
    println!("   - Scientific symbols and notation");
    println!("   - Roboto font with advanced text shaping");
    println!("   - Professional kerning and ligatures");
    
    Ok(())
}