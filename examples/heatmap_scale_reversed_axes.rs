//! Demonstrates the issue #40 and #41 behavior changes:
//! - descending `.xlim()` / `.ylim()` values preserve reversed axes
//! - heatmaps support value scaling via `HeatmapConfig::value_scale`
//!
//! Run with:
//! `cargo run --example heatmap_scale_reversed_axes`

use ruviz::prelude::*;

fn main() -> Result<()> {
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];

    let reversed_x_plot = Plot::new()
        .title("Reversed X Limits")
        .xlabel("X")
        .ylabel("Y")
        .grid(true)
        .margin(0.10)
        .line(&x, &y)
        .color(Color::from_palette(0))
        .line_width(2.0)
        .marker(MarkerStyle::Circle)
        .marker_size(5.0)
        .xlim(4.0, 0.0);

    let reversed_y_plot = Plot::new()
        .title("Reversed Y Limits")
        .xlabel("X")
        .ylabel("Y")
        .grid(true)
        .margin(0.10)
        .line(&x, &y)
        .color(Color::from_palette(1))
        .line_width(2.0)
        .marker(MarkerStyle::Circle)
        .marker_size(5.0)
        .ylim(16.0, 0.0);

    let heatmap_data: Vec<Vec<f64>> = (0..10)
        .map(|row| {
            (0..10)
                .map(|col| {
                    let exponent = (row + col) as i32;
                    10.0_f64.powi(exponent / 4)
                })
                .collect()
        })
        .collect();

    let heatmap_plot = Plot::new()
        .title("Heatmap Value Scale: Log")
        .xlabel("Column")
        .ylabel("Row")
        .heatmap(
            &heatmap_data,
            Some(
                HeatmapConfig::new()
                    .value_scale(AxisScale::Log)
                    .colorbar(true)
                    .colorbar_label("Intensity")
                    .aspect(1.0),
            ),
        );

    subplots(1, 3, 1800, 560)?
        .suptitle("Issue #40 and #41 Example")
        .margin(0.07)
        .wspace(0.16)
        .subplot_at(0, reversed_x_plot.into())?
        .subplot_at(1, reversed_y_plot.into())?
        .subplot_at(2, heatmap_plot.into())?
        .save("generated/examples/heatmap_scale_reversed_axes.png")?;

    println!("Saved: generated/examples/heatmap_scale_reversed_axes.png");
    Ok(())
}
