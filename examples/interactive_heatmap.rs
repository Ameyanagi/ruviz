//! Interactive heatmap exploration example
//!
//! Demonstrates zooming and panning across a dense scalar field.
//!
//! Run with: cargo run --features interactive --example interactive_heatmap

use ruviz::prelude::*;

fn main() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create current-thread Tokio runtime for interactive example")
        .block_on(async_main())
}

async fn async_main() -> Result<()> {
    println!("Starting interactive heatmap example...");
    println!("Controls:");
    println!("  - Mouse wheel: Zoom in/out");
    println!("  - Left click + drag: Pan");
    println!("  - Right click: Context menu");
    println!("  - Right click + drag: Box zoom");
    println!("  - Escape: Reset view");
    println!("  - Cmd/Ctrl+S: Save PNG");
    println!("  - Cmd/Ctrl+C: Copy image");
    println!("  - Close window to exit");

    let rows = 96;
    let cols = 144;
    let data = build_scalar_field(rows, cols);

    let config = HeatmapConfig::new()
        .colormap(ColorMap::viridis())
        .colorbar(true)
        .colorbar_label("Field strength");

    let plot: Plot = Plot::new()
        .title("Interactive Heatmap Explorer")
        .xlabel("Column")
        .ylabel("Row")
        .heatmap(&data, Some(config))
        .into();

    println!("Heatmap created with {} rows x {} columns", rows, cols);

    #[cfg(feature = "interactive")]
    {
        println!("Opening interactive window...");
        show_interactive(plot).await?;
    }

    #[cfg(not(feature = "interactive"))]
    {
        println!("Interactive features not enabled.");
        println!("To enable: cargo run --features interactive --example interactive_heatmap");
        std::fs::create_dir_all("examples/output").ok();
        plot.save("examples/output/interactive_heatmap_static.png")?;
        println!("Saved static version as: examples/output/interactive_heatmap_static.png");
    }

    Ok(())
}

fn build_scalar_field(rows: usize, cols: usize) -> Vec<Vec<f64>> {
    let mut data = vec![vec![0.0; cols]; rows];

    for (row_idx, row) in data.iter_mut().enumerate() {
        let y = (row_idx as f64 / rows as f64 - 0.5) * 6.0;

        for (col_idx, value) in row.iter_mut().enumerate() {
            let x = (col_idx as f64 / cols as f64 - 0.5) * 8.0;
            let ridge = (-((x - 1.4).powi(2) * 0.7 + (y + 0.9).powi(2) * 1.8)).exp();
            let basin = -0.85 * (-((x + 1.9).powi(2) * 1.0 + (y - 1.1).powi(2) * 0.8)).exp();
            let waves = 0.35 * (x * 2.2).sin() * (y * 1.6).cos();
            *value = ridge + basin + waves;
        }
    }

    data
}
