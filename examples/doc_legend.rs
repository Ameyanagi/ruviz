//! Documentation example: Legend
//!
//! Generates docs/assets/rustdoc/legend.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let sin_y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
    let cos_y: Vec<f64> = x.iter().map(|&v| v.cos()).collect();

    Plot::new()
        .title("Trigonometric Functions")
        .xlabel("x")
        .ylabel("y")
        .legend_position(LegendPosition::Best)
        .line(&x, &sin_y)
        .label("sin(x)")
        .line(&x, &cos_y)
        .label("cos(x)")
        .max_resolution(1920, 1440)
        .save("docs/assets/rustdoc/legend.png")?;

    println!("✓ Generated docs/assets/rustdoc/legend.png");
    Ok(())
}
