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

    plot.save("assets/readme_example.png")?;

    println!("✓ Generated assets/readme_example.png");
    Ok(())
}
