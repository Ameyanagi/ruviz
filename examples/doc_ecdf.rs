//! Documentation example: ECDF Plot
//!
//! Generates docs/images/ecdf_plot.png for rustdoc
//!
//! This example demonstrates the high-level ECDF (Empirical Cumulative Distribution Function) API.

use ruviz::prelude::*;

fn main() -> Result<()> {
    // Generate sample data from a normal-like distribution
    let data: Vec<f64> = (0..200)
        .map(|i| {
            let u1 = ((i * 7 + 13) % 200) as f64 / 200.0;
            let u2 = ((i * 11 + 17) % 200) as f64 / 200.0;
            // Box-Muller transform for pseudo-normal distribution
            5.0 + 2.0 * (-2.0 * u1.max(0.01).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        })
        .collect();

    Plot::new()
        .ecdf(&data)
        .title("Empirical Cumulative Distribution Function")
        .xlabel("Value")
        .ylabel("Proportion")
        .size(8.0, 5.0)
        .ecdf_line_width(2.0)
        .label("Sample Distribution")
        .color(Color::from_palette(0))
        .legend_best()
        .save("docs/images/ecdf_plot.png")?;

    println!("Generated docs/images/ecdf_plot.png");
    Ok(())
}
