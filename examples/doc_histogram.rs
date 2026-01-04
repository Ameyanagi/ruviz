//! Documentation example: Histogram
//!
//! Generates docs/images/histogram.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    // Generate normally distributed data
    let data: Vec<f64> = (0..1000)
        .map(|i| {
            // Simple pseudo-random values
            let u1 = ((i * 7 + 13) % 1000) as f64 / 1000.0;
            let u2 = ((i * 11 + 17) % 1000) as f64 / 1000.0;
            let z = (-2.0 * u1.max(0.001).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            z
        })
        .collect();

    Plot::new()
        .title("Histogram")
        .xlabel("Value")
        .ylabel("Frequency")
        .dpi(300)
        .histogram(&data, None)
        .end_series()
        .save("docs/images/histogram.png")?;

    println!("✓ Generated docs/images/histogram.png");
    Ok(())
}
