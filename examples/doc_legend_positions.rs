//! Documentation example: Legend positions
//!
//! Generates docs/assets/rustdoc/legend_positions.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
    let y_sin: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
    let y_cos: Vec<f64> = x.iter().map(|&v| v.cos()).collect();

    let positions = [
        ("UpperLeft", LegendPosition::UpperLeft),
        ("UpperRight", LegendPosition::UpperRight),
        ("LowerLeft", LegendPosition::LowerLeft),
        ("LowerRight", LegendPosition::LowerRight),
        ("OutsideLeft", LegendPosition::OutsideLeft),
        ("OutsideRight", LegendPosition::OutsideRight),
        ("OutsideUpper", LegendPosition::OutsideUpper),
        ("OutsideLower", LegendPosition::OutsideLower),
    ];
    let plots: Vec<Plot> = positions
        .into_iter()
        .map(|(title, position)| {
            Plot::new()
                .title(title)
                .legend_position(position)
                .line(&x, &y_sin)
                .label("sin(x)")
                .color(Color::from_palette(0))
                .line(&x, &y_cos)
                .label("cos(x)")
                .color(Color::from_palette(1))
                .into()
        })
        .collect();

    let mut figure = subplots(2, 4, 1200, 600)?.suptitle("Legend Positions");
    for (index, plot) in plots.into_iter().enumerate() {
        figure = figure.subplot_at(index, plot)?;
    }
    figure.save("docs/assets/rustdoc/legend_positions.png")?;

    println!("✓ Generated docs/assets/rustdoc/legend_positions.png");
    Ok(())
}
