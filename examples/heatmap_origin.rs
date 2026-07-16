//! Compare heatmap row origins against the same physical Y extent.
//!
//! Run with: `cargo run --example heatmap_origin`

use ruviz::prelude::*;

fn main() -> Result<()> {
    let values = vec![
        vec![0.0, 0.1, 0.2],
        vec![1.0, 1.1, 1.2],
        vec![2.0, 2.1, 2.2],
    ];
    let config = HeatmapConfig::new()
        .extent(0.0, 3.0, 10.0, 16.0)
        .vmin(0.0)
        .vmax(2.2)
        .annotate(true)
        .colorbar(false);

    let upper = Plot::new()
        .title("Upper: row 0 at y = 16")
        .xlabel("column")
        .ylabel("physical Y")
        .heatmap(&values, Some(config.clone().origin(HeatmapOrigin::Upper)));
    let lower = Plot::new()
        .title("Lower: row 0 at y = 10")
        .xlabel("column")
        .ylabel("physical Y")
        .heatmap(&values, Some(config.origin(HeatmapOrigin::Lower)));

    subplots(1, 2, 1000, 440)?
        .suptitle("Heatmap row-origin policy")
        .subplot_at(0, upper.into())?
        .subplot_at(1, lower.into())?
        .save("generated/examples/heatmap_origin.png")?;

    Ok(())
}
