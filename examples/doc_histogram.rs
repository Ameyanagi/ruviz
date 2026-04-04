//! Documentation example: Histogram
//!
//! Generates docs/assets/rustdoc/histogram.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    // Generate normally distributed data
    let data: Vec<f64> = (0..1000)
        .map(|i| {
            // Simple pseudo-random values
            let u1 = ((i * 7 + 13) % 1000) as f64 / 1000.0;
            let u2 = ((i * 11 + 17) % 1000) as f64 / 1000.0;
            (-2.0 * u1.max(0.001).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        })
        .collect();

    Plot::new()
        .title("Histogram")
        .xlabel("Value")
        .ylabel("Frequency")
        .max_resolution(1920, 1440)
        .histogram(&data, None)
        .save("docs/assets/rustdoc/histogram.png")?;

    println!("✓ Generated docs/assets/rustdoc/histogram.png");
    Ok(())
}
