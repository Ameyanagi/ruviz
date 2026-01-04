//! Documentation example: Line styles
//!
//! Generates docs/images/line_styles.png for rustdoc

use ruviz::prelude::*;
use ruviz::render::LineStyle;

fn main() -> Result<()> {
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();

    // Create different y values for each line (offset for visibility)
    let y_solid: Vec<f64> = x.iter().map(|&v| v.sin() + 4.0).collect();
    let y_dashed: Vec<f64> = x.iter().map(|&v| v.sin() + 3.0).collect();
    let y_dotted: Vec<f64> = x.iter().map(|&v| v.sin() + 2.0).collect();
    let y_dashdot: Vec<f64> = x.iter().map(|&v| v.sin() + 1.0).collect();
    let y_dashdotdot: Vec<f64> = x.iter().map(|&v| v.sin()).collect();

    Plot::new()
        .title("Line Styles")
        .xlabel("x")
        .ylabel("y")
        .dpi(300)
        .legend_position(LegendPosition::Best)
        .line(&x, &y_solid)
        .label("Solid")
        .style(LineStyle::Solid)
        .line(&x, &y_dashed)
        .label("Dashed")
        .style(LineStyle::Dashed)
        .line(&x, &y_dotted)
        .label("Dotted")
        .style(LineStyle::Dotted)
        .line(&x, &y_dashdot)
        .label("DashDot")
        .style(LineStyle::DashDot)
        .line(&x, &y_dashdotdot)
        .label("DashDotDot")
        .style(LineStyle::DashDotDot)
        .end_series()
        .save("docs/images/line_styles.png")?;

    println!("✓ Generated docs/images/line_styles.png");
    Ok(())
}
