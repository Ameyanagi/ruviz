//! Documentation example: Legend positions
//!
//! Generates docs/images/legend_positions.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();

    // Create 9 separate plots for each legend position
    let plot_ul = Plot::new()
        .title("UpperLeft")
        .legend_position(LegendPosition::UpperLeft)
        .line(&x, &y)
        .label("sin(x)")
        .end_series();

    let plot_uc = Plot::new()
        .title("UpperCenter")
        .legend_position(LegendPosition::UpperCenter)
        .line(&x, &y)
        .label("sin(x)")
        .end_series();

    let plot_ur = Plot::new()
        .title("UpperRight")
        .legend_position(LegendPosition::UpperRight)
        .line(&x, &y)
        .label("sin(x)")
        .end_series();

    let plot_cl = Plot::new()
        .title("CenterLeft")
        .legend_position(LegendPosition::CenterLeft)
        .line(&x, &y)
        .label("sin(x)")
        .end_series();

    let plot_c = Plot::new()
        .title("Center")
        .legend_position(LegendPosition::Center)
        .line(&x, &y)
        .label("sin(x)")
        .end_series();

    let plot_cr = Plot::new()
        .title("CenterRight")
        .legend_position(LegendPosition::CenterRight)
        .line(&x, &y)
        .label("sin(x)")
        .end_series();

    let plot_ll = Plot::new()
        .title("LowerLeft")
        .legend_position(LegendPosition::LowerLeft)
        .line(&x, &y)
        .label("sin(x)")
        .end_series();

    let plot_lc = Plot::new()
        .title("LowerCenter")
        .legend_position(LegendPosition::LowerCenter)
        .line(&x, &y)
        .label("sin(x)")
        .end_series();

    let plot_lr = Plot::new()
        .title("LowerRight")
        .legend_position(LegendPosition::LowerRight)
        .line(&x, &y)
        .label("sin(x)")
        .end_series();

    // Create a 3x3 subplot figure
    subplots(3, 3, 900, 900)?
        .subplot_at(0, plot_ul)?
        .subplot_at(1, plot_uc)?
        .subplot_at(2, plot_ur)?
        .subplot_at(3, plot_cl)?
        .subplot_at(4, plot_c)?
        .subplot_at(5, plot_cr)?
        .subplot_at(6, plot_ll)?
        .subplot_at(7, plot_lc)?
        .subplot_at(8, plot_lr)?
        .save("docs/images/legend_positions.png")?;

    println!("✓ Generated docs/images/legend_positions.png");
    Ok(())
}
