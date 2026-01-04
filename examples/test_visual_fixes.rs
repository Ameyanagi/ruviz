//! Test for bar chart category labels and Y-axis alignment fixes
use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Test 1: Bar chart with categories
    println!("Testing bar chart with categories...");

    let categories = vec!["Rust", "Python", "Go", "TypeScript"];
    let values = vec![95.0, 70.0, 65.0, 80.0];

    Plot::new()
        .size(8.0, 6.0)
        .dpi(100)
        .bar(&categories, &values)
        .title("Language Performance")
        .xlabel("Programming Language")
        .ylabel("Performance Score")
        .save("/tmp/test_bar_categories.png")?;

    println!("Saved: /tmp/test_bar_categories.png");

    // Test 2: Render bar chart to SVG for inspection
    let svg = Plot::new()
        .size(8.0, 6.0)
        .dpi(100)
        .bar(&categories, &values)
        .title("Language Performance")
        .xlabel("Programming Language")
        .ylabel("Performance Score")
        .render_to_svg()?;

    std::fs::write("/tmp/test_bar_categories.svg", &svg)?;
    println!("Saved: /tmp/test_bar_categories.svg");

    // Test 3: Line plot to verify Y-axis alignment
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&x| (x * 0.5).sin() * 50.0 + 50.0).collect();

    Plot::new()
        .size(8.0, 6.0)
        .dpi(100)
        .line(&x, &y)
        .title("Y-Axis Alignment Test")
        .xlabel("X")
        .ylabel("Y")
        .save("/tmp/test_yaxis_alignment.png")?;

    println!("Saved: /tmp/test_yaxis_alignment.png");

    // Also save SVG for inspection
    let svg2 = Plot::new()
        .size(8.0, 6.0)
        .dpi(100)
        .line(&x, &y)
        .title("Y-Axis Alignment Test")
        .xlabel("X")
        .ylabel("Y")
        .render_to_svg()?;

    std::fs::write("/tmp/test_yaxis_alignment.svg", &svg2)?;
    println!("Saved: /tmp/test_yaxis_alignment.svg");

    println!("\nDone! Check the PNG and SVG files.");
    Ok(())
}
