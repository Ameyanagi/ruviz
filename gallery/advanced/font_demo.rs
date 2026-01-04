use ruviz::prelude::*;
use std::fs;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Creating cosmic-text font rendering demo...");

    // Ensure test output directory exists
    fs::create_dir_all("test_output")?;

    // Sample data for demonstration
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y = vec![1.0, 3.0, 2.0, 4.0, 3.5, 5.0];

    // Create plot with cosmic-text font rendering
    Plot::new()
        .line(&x, &y)
        .title("Professional Cosmic-Text Font Rendering")
        .xlabel("X Axis (Roboto Font + Advanced Shaping)")
        .ylabel("Y Values")
        .save("gallery/advanced/font_demo.png")?;

    println!("Plot saved as 'gallery/advanced/font_demo.png'");
    println!("Text now uses cosmic-text with Roboto font and advanced typography");

    Ok(())
}
