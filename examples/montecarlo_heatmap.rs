//! Synthetic Monte Carlo-style absorbed-energy heatmap.
//!
//! Run with:
//! `cargo run --example montecarlo_heatmap`

use ruviz::prelude::*;

fn synthetic_absorbed_energy(rows: usize, cols: usize) -> Vec<Vec<f64>> {
    let center_col = (cols.saturating_sub(1)) as f64 / 2.0;

    (0..rows)
        .map(|row| {
            let depth = 1.0 - row as f64 / (rows.saturating_sub(1).max(1)) as f64;

            (0..cols)
                .map(|col| {
                    let x = (col as f64 - center_col) / cols as f64;
                    let beam_core = 950.0 * (-(x / 0.014).powi(2)).exp() * (-(depth / 0.10)).exp();
                    let beam_body = 45.0 * (-(x / 0.18).powi(2)).exp() * (-(depth / 0.55)).exp();
                    let buildup_layer = 18.0
                        * (-(x / 0.24).powi(2)).exp()
                        * (-((depth - 0.28) / 0.025).powi(2)).exp();
                    let deep_tail = 1.2 * (-(x / 0.12).powi(2)).exp() * (-(depth / 1.2)).exp();
                    let scattered_floor =
                        2.0e-4 * (-(x / 0.34).powi(2)).exp() * (-(depth / 0.95)).exp();

                    let edge_cutout = col < 10 || col >= cols.saturating_sub(10);
                    let sparse_cutout =
                        (col < 18 || col >= cols.saturating_sub(18)) && ((row + col * 3) % 11 < 6);

                    if edge_cutout || sparse_cutout {
                        0.0
                    } else {
                        beam_core + beam_body + buildup_layer + deep_tail + scattered_floor
                    }
                })
                .collect()
        })
        .collect()
}

fn main() -> Result<()> {
    let rows = 96;
    let cols = 120;
    let data = synthetic_absorbed_energy(rows, cols);

    Plot::new()
        .size_px(900, 640)
        .title("Synthetic Monte Carlo Absorbed Energy")
        .xlabel("Position in x (cells)")
        .ylabel("Depth (cells)")
        .heatmap(
            &data,
            Some(
                HeatmapConfig::new()
                    .value_scale(AxisScale::Log)
                    .colorbar(true)
                    .colorbar_log_subticks(true)
                    .colorbar_label("Absorbed Energy"),
            ),
        )
        .ylim(rows as f64, 0.0)
        .save("generated/examples/montecarlo_heatmap.png")?;

    println!("Saved: generated/examples/montecarlo_heatmap.png");
    Ok(())
}
