//! Documentation example: Subplots
//!
//! Generates docs/images/subplots.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y_sin: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
    let y_cos: Vec<f64> = x.iter().map(|&v| v.cos()).collect();

    // Subplot 0: Line plot
    let plot_line = Plot::new()
        .title("Line Plot")
        .xlabel("x")
        .ylabel("sin(x)")
        .line(&x, &y_sin)
        .end_series();

    // Subplot 1: Scatter plot
    let plot_scatter = Plot::new()
        .title("Scatter Plot")
        .xlabel("x")
        .ylabel("cos(x)")
        .scatter(&x, &y_cos)
        .end_series();

    // Subplot 2: Bar chart
    let categories = vec!["A", "B", "C", "D"];
    let values = vec![25.0, 40.0, 30.0, 55.0];
    let plot_bar = Plot::new()
        .title("Bar Chart")
        .xlabel("Category")
        .ylabel("Value")
        .bar(&categories, &values)
        .end_series();

    // Subplot 3: Multiple lines
    let plot_multi = Plot::new()
        .title("Multiple Series")
        .xlabel("x")
        .ylabel("y")
        .legend_position(LegendPosition::Best)
        .line(&x, &y_sin)
        .label("sin(x)")
        .line(&x, &y_cos)
        .label("cos(x)")
        .end_series();

    // Create a 2x2 subplot figure
    subplots(2, 2, 800, 600)?
        .subplot_at(0, plot_line)?
        .subplot_at(1, plot_scatter)?
        .subplot_at(2, plot_bar)?
        .subplot_at(3, plot_multi)?
        .save("docs/images/subplots.png")?;

    println!("✓ Generated docs/images/subplots.png");
    Ok(())
}
