//! Documentation example: Color palette
//!
//! Generates docs/images/colors.png for rustdoc

use ruviz::prelude::*;
use ruviz::render::Color;

fn main() -> Result<()> {
    let palette = Color::default_palette();
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();

    // Create y values for each color line (offset for visibility)
    let y1: Vec<f64> = x.iter().map(|&v| v.sin() + 4.0).collect();
    let y2: Vec<f64> = x.iter().map(|&v| v.sin() + 3.5).collect();
    let y3: Vec<f64> = x.iter().map(|&v| v.sin() + 3.0).collect();
    let y4: Vec<f64> = x.iter().map(|&v| v.sin() + 2.5).collect();
    let y5: Vec<f64> = x.iter().map(|&v| v.sin() + 2.0).collect();
    let y6: Vec<f64> = x.iter().map(|&v| v.sin() + 1.5).collect();
    let y7: Vec<f64> = x.iter().map(|&v| v.sin() + 1.0).collect();
    let y8: Vec<f64> = x.iter().map(|&v| v.sin() + 0.5).collect();

    Plot::new()
        .title("Default Color Palette")
        .xlabel("x")
        .ylabel("y")
        .max_resolution(1920, 1440)
        .legend_position(LegendPosition::Best)
        .line(&x, &y1)
        .label("Color 1")
        .color(palette[0])
        .line(&x, &y2)
        .label("Color 2")
        .color(palette[1])
        .line(&x, &y3)
        .label("Color 3")
        .color(palette[2])
        .line(&x, &y4)
        .label("Color 4")
        .color(palette[3])
        .line(&x, &y5)
        .label("Color 5")
        .color(palette[4])
        .line(&x, &y6)
        .label("Color 6")
        .color(palette[5])
        .line(&x, &y7)
        .label("Color 7")
        .color(palette[6])
        .line(&x, &y8)
        .label("Color 8")
        .color(palette[7])
        .save("docs/images/colors.png")?;

    println!("✓ Generated docs/images/colors.png");
    Ok(())
}
