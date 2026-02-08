use ruviz::prelude::*;
#[cfg(feature = "typst-math")]
use std::fs;

#[cfg(feature = "typst-math")]
fn run() -> Result<()> {
    let out_dir = "examples/output";
    fs::create_dir_all(out_dir).map_err(ruviz::core::PlottingError::IoError)?;

    let x: Vec<f64> = (0..80).map(|i| i as f64 * 0.05).collect();
    let y: Vec<f64> = x.iter().map(|&v| (-v).exp()).collect();

    // Plain and Typst outputs are generated side-by-side for visual parity checks.
    let plain_plot = Plot::new()
        .line(&x, &y)
        .label("exp(-x)")
        .title("Plain Check: f(x) = exp(-x)")
        .xlabel("Time t")
        .ylabel("Amplitude A(t)");

    let typst_plot = Plot::new()
        .line(&x, &y)
        .label("$e^{-x}$")
        .title("Typst Check: $f(x) = e^{-x}$")
        .xlabel("Time $t$")
        .ylabel("Amplitude $A(t)$")
        .typst(true);

    let plain_png_path = format!("{out_dir}/plain_check.png");
    let png_path = format!("{out_dir}/typst_check.png");
    let svg_path = format!("{out_dir}/typst_check.svg");

    plain_plot.save(&plain_png_path)?;
    typst_plot.clone().save(&png_path)?;
    typst_plot.clone().export_svg(&svg_path)?;

    println!("Generated:");
    println!("  {plain_png_path}");
    println!("  {png_path}");
    println!("  {svg_path}");
    Ok(())
}

#[cfg(not(feature = "typst-math"))]
fn run() -> Result<()> {
    println!(
        "This example requires `typst-math`.\nRun: cargo run --example typst_check --features typst-math"
    );
    Ok(())
}

fn main() -> Result<()> {
    run()
}
