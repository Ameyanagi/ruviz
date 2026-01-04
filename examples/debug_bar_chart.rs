//! Debug bar chart rendering
use ruviz::export::SvgRenderer;
use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let categories = vec!["Rust", "Python", "Go", "TypeScript"];
    let values = vec![95.0, 70.0, 65.0, 80.0];

    // Create a bar chart and print debug info
    let plot = Plot::new()
        .size(8.0, 6.0)
        .dpi(100)
        .bar(&categories, &values)
        .title("Debug Bar Chart");

    // Render to SVG
    let svg = plot.render_to_svg()?;

    // Print SVG for debugging
    println!("SVG output:");
    println!("{}", svg);

    // Also save it
    std::fs::write("/tmp/debug_bar_chart.svg", &svg)?;
    println!("\nSaved to /tmp/debug_bar_chart.svg");

    Ok(())
}
