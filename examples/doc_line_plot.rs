//! Documentation example: Line plot
//!
//! Generates docs/images/line_plot.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();

    Plot::new()
        .title("Sine Wave")
        .xlabel("x")
        .ylabel("sin(x)")
        .dpi(300)
        .line(&x, &y)
        .end_series()
        .save("docs/images/line_plot.png")?;

    println!("✓ Generated docs/images/line_plot.png");
    Ok(())
}
