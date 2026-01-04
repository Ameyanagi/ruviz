//! Annotations example - demonstrates text, arrows, lines, shapes, and fill_between
//!
//! Run with: cargo run --example annotations_example

use ruviz::prelude::*;

fn main() -> Result<()> {
    // Create sample data
    let x: Vec<f64> = (0..20).map(|i| i as f64 * 0.5).collect();
    let y: Vec<f64> = x.iter().map(|&xi| xi * xi / 10.0).collect();

    // Upper and lower bounds for fill_between
    let y_upper: Vec<f64> = y.iter().map(|&yi| yi + 1.0).collect();
    let y_lower: Vec<f64> = y.iter().map(|&yi| (yi - 1.0).max(0.0)).collect();

    // Create plot with annotations
    Plot::new()
        .line(&x, &y)
        .title("Annotations Demo")
        .xlabel("X")
        .ylabel("Y = X² / 10")
        // Add text annotation at peak
        .text(9.0, 8.0, "Peak region")
        // Add an arrow pointing to a specific data point
        .arrow(4.0, 6.0, 6.0, 3.6)
        // Add horizontal reference line
        .hline(5.0)
        // Add vertical reference line
        .vline(5.0)
        // Add a highlighted region (fill between)
        .fill_between(&x, &y_lower, &y_upper)
        // Add a shaded vertical span
        .axvspan(2.0, 4.0)
        // Add a shaded horizontal span
        .axhspan(6.0, 8.0)
        .save("examples/output/annotations_demo.png")?;

    println!("Annotations demo saved to test_output/annotations_demo.png");

    // Also create a simpler example with just text and arrow
    Plot::new()
        .line(&x, &y)
        .title("Text and Arrow Annotations")
        .text(5.0, 2.5, "Midpoint")
        .arrow(1.0, 0.5, 3.0, 0.9)
        .hline(4.0)
        .vline(7.0)
        .save("examples/output/simple_annotations.png")?;

    println!("Simple annotations saved to test_output/simple_annotations.png");

    Ok(())
}
