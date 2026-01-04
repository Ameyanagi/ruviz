//! Documentation example: Legend positions
//!
//! Generates docs/images/legend_positions.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
    let y_sin: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
    let y_cos: Vec<f64> = x.iter().map(|&v| v.cos()).collect();

    // Show 4 key legend positions with multiple series
    let plot_ul = Plot::new()
        .title("UpperLeft")
        .legend_position(LegendPosition::UpperLeft)
        .line(&x, &y_sin)
        .label("sin(x)")
        .color(Color::from_palette(0))
        .line(&x, &y_cos)
        .label("cos(x)")
        .color(Color::from_palette(1))
        .end_series();

    let plot_ur = Plot::new()
        .title("UpperRight")
        .legend_position(LegendPosition::UpperRight)
        .line(&x, &y_sin)
        .label("sin(x)")
        .color(Color::from_palette(0))
        .line(&x, &y_cos)
        .label("cos(x)")
        .color(Color::from_palette(1))
        .end_series();

    let plot_ll = Plot::new()
        .title("LowerLeft")
        .legend_position(LegendPosition::LowerLeft)
        .line(&x, &y_sin)
        .label("sin(x)")
        .color(Color::from_palette(0))
        .line(&x, &y_cos)
        .label("cos(x)")
        .color(Color::from_palette(1))
        .end_series();

    let plot_lr = Plot::new()
        .title("LowerRight")
        .legend_position(LegendPosition::LowerRight)
        .line(&x, &y_sin)
        .label("sin(x)")
        .color(Color::from_palette(0))
        .line(&x, &y_cos)
        .label("cos(x)")
        .color(Color::from_palette(1))
        .end_series();

    // Create a 2x2 subplot figure with larger size
    subplots(2, 2, 800, 600)?
        .suptitle("Legend Positions")
        .subplot_at(0, plot_ul)?
        .subplot_at(1, plot_ur)?
        .subplot_at(2, plot_ll)?
        .subplot_at(3, plot_lr)?
        .save("docs/images/legend_positions.png")?;

    println!("✓ Generated docs/images/legend_positions.png");
    Ok(())
}
