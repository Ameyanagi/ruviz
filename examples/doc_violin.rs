//! Documentation example: Violin Plot
//!
//! Generates docs/images/violin_plot.png for rustdoc
//!
//! This example demonstrates the high-level API for creating violin plots.

use ruviz::prelude::*;

fn main() -> Result<()> {
    // Generate sample data (bimodal distribution)
    let data1: Vec<f64> = (0..200)
        .map(|i| {
            let u1 = ((i * 7 + 13) % 200) as f64 / 200.0;
            let u2 = ((i * 11 + 17) % 200) as f64 / 200.0;
            let normal =
                (-2.0 * u1.max(0.01).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            if i % 2 == 0 {
                3.0 + normal
            } else {
                7.0 + normal * 0.8
            }
        })
        .collect();

    // High-level API - simple violin plot with category label
    Plot::new()
        .title("Violin Plot")
        .xlabel("Group")
        .ylabel("Value")
        .violin(&data1)
        .show_box(true)
        .show_median(true)
        .fill_alpha(0.6)
        .label("Group A")
        .category("Group A")
        .save("docs/images/violin_plot.png")?;

    println!("Generated docs/images/violin_plot.png (high-level API)");

    // Different distribution for comparison
    let data2: Vec<f64> = (0..200)
        .map(|i| {
            let u1 = ((i * 13 + 7) % 200) as f64 / 200.0;
            let u2 = ((i * 17 + 11) % 200) as f64 / 200.0;
            5.0 + 2.0 * (-2.0 * u1.max(0.01).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        })
        .collect();

    // Violin with quartiles visible
    Plot::new()
        .title("Distribution Analysis")
        .xlabel("Sample")
        .ylabel("Value")
        .violin(&data2)
        .show_box(true)
        .show_quartiles(true)
        .show_median(true)
        .fill_alpha(0.7)
        .color(Color::from_hex("#27ae60").unwrap())
        .save("docs/images/violin_quartiles.png")?;

    println!("Generated docs/images/violin_quartiles.png");

    Ok(())
}
