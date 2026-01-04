//! Documentation example: Subplots
//!
//! Generates docs/images/subplots.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
    let y_sin: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
    let y_cos: Vec<f64> = x.iter().map(|&v| v.cos()).collect();

    // Subplot 0: Line plot with styling
    let plot_line = Plot::new()
        .title("Line Plot")
        .xlabel("x")
        .ylabel("y")
        .line(&x, &y_sin)
        .color(Color::from_palette(0))
        .width(2.0)
        .end_series();

    // Subplot 1: Scatter plot
    let x_scatter: Vec<f64> = (0..30).map(|i| i as f64 * 0.3).collect();
    let y_scatter: Vec<f64> = x_scatter
        .iter()
        .map(|&v| v.cos() + 0.1 * (v * 3.0).sin())
        .collect();
    let plot_scatter = Plot::new()
        .title("Scatter Plot")
        .xlabel("x")
        .ylabel("y")
        .scatter(&x_scatter, &y_scatter)
        .color(Color::from_palette(1))
        .marker_size(6.0)
        .end_series();

    // Subplot 2: Bar chart
    let categories = vec!["Q1", "Q2", "Q3", "Q4"];
    let values = vec![28.0, 45.0, 38.0, 52.0];
    let plot_bar = Plot::new()
        .title("Bar Chart")
        .xlabel("Quarter")
        .ylabel("Sales")
        .bar(&categories, &values)
        .color(Color::from_palette(2))
        .end_series();

    // Subplot 3: Multiple series with legend
    let plot_multi = Plot::new()
        .title("Comparison")
        .xlabel("x")
        .ylabel("y")
        .legend_position(LegendPosition::UpperRight)
        .line(&x, &y_sin)
        .label("sin(x)")
        .color(Color::from_palette(0))
        .line(&x, &y_cos)
        .label("cos(x)")
        .color(Color::from_palette(1))
        .end_series();

    // Create a 2x2 subplot figure at 300 DPI
    subplots(2, 2, 800, 600)?
        .suptitle("Subplot Gallery")
        .subplot_at(0, plot_line)?
        .subplot_at(1, plot_scatter)?
        .subplot_at(2, plot_bar)?
        .subplot_at(3, plot_multi)?
        .save_with_dpi("docs/images/subplots.png", 300.0)?;

    println!("✓ Generated docs/images/subplots.png");
    Ok(())
}
