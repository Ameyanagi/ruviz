//! Documentation example: Scatter plot
//!
//! Generates docs/images/scatter_plot.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    // Generate some sample data with noise
    let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
    let y: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, &v)| v.sin() + (i as f64 * 0.1).sin() * 0.3)
        .collect();

    Plot::new()
        .title("Scatter Plot")
        .xlabel("x")
        .ylabel("y")
        .dpi(300)
        .scatter(&x, &y)
        .end_series()
        .save("docs/images/scatter_plot.png")?;

    println!("✓ Generated docs/images/scatter_plot.png");
    Ok(())
}
