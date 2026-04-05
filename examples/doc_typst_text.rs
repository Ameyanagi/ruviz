//! Documentation example: Typst text rendering
//!
//! Generates docs/assets/rustdoc/typst_text.png for rustdoc.

use ruviz::prelude::*;

#[cfg(feature = "typst-math")]
fn run() -> Result<()> {
    let x: Vec<f64> = (0..80).map(|i| i as f64 * 0.05).collect();
    let y: Vec<f64> = x.iter().map(|&v| (-v).exp()).collect();

    Plot::new()
        .line(&x, &y)
        .label("$e^(-x)$")
        .title("Exponential Decay: $f(x) = e^(-x)$")
        .xlabel("Time $t$")
        .ylabel("Amplitude $A(t)$")
        .typst(true)
        .max_resolution(1920, 1440)
        .save("docs/assets/rustdoc/typst_text.png")?;

    println!("✓ Generated docs/assets/rustdoc/typst_text.png");
    Ok(())
}

#[cfg(not(feature = "typst-math"))]
fn run() -> Result<()> {
    eprintln!(
        "This example requires the `typst-math` feature. Run with: \
         cargo run --example doc_typst_text --features typst-math"
    );
    Ok(())
}

fn main() -> Result<()> {
    run()
}
