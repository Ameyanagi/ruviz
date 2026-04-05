use ruviz::prelude::*;
#[cfg(feature = "typst-math")]
use std::fs;

#[cfg(feature = "typst-math")]
fn run() -> Result<()> {
    let out_dir = "generated/examples";
    fs::create_dir_all(out_dir).map_err(ruviz::core::PlottingError::IoError)?;

    let x: Vec<f64> = (0..80).map(|i| i as f64 * 0.05).collect();
    let y: Vec<f64> = x.iter().map(|&v| (-v).exp()).collect();

    // Plain and Typst outputs are generated side-by-side for visual parity checks.
    let title_plain = "Parity Check: f(x) = e^-x";
    let title_typst = "Parity Check: $f(x) = e^(-x)$";
    let xlabel_plain = "Time t";
    let xlabel_typst = "Time $t$";
    let ylabel_plain = "Amplitude A(t)";
    let ylabel_typst = "Amplitude $A(t)$";
    let legend_plain = "e^-x";
    let legend_typst = "$e^(-x)$";

    let plain_plot = Plot::new()
        .line(&x, &y)
        .label(legend_plain)
        .title(title_plain)
        .xlabel(xlabel_plain)
        .ylabel(ylabel_plain);

    let typst_plot = Plot::new()
        .line(&x, &y)
        .label(legend_typst)
        .title(title_typst)
        .xlabel(xlabel_typst)
        .ylabel(ylabel_typst)
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
