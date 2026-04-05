mod util;

use ruviz::prelude::*;

fn main() -> Result<()> {
    // Simple quadratic function
    let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&x| x * x).collect();

    let plot = Plot::new()
        .line(&x, &y)
        .title("Quadratic Function")
        .xlabel("x");

    #[cfg(feature = "typst-math")]
    let plot = plot.ylabel("$y = x^2$").typst(true);
    #[cfg(not(feature = "typst-math"))]
    let plot = plot.ylabel("y = x^2");

    let output = util::readme_asset_path("readme_example.png");
    plot.save(&output)?;

    println!("✓ Generated {}", output.display());
    Ok(())
}
