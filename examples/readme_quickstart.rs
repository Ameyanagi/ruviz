use ruviz::prelude::*;

fn main() -> Result<()> {
    // Simple quadratic function
    let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&x| x * x).collect();

    Plot::new()
        .line(&x, &y)
        .title("Quadratic Function")
        .xlabel("x")
        .ylabel("y = x²")
        .save("assets/readme_example.png")?;

    println!("✓ Generated assets/readme_example.png");
    Ok(())
}
