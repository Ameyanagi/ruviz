//! Public raster routing diagnostic for GPU-preference builds.
//!
//! The elapsed time reported here belongs to the resolved backend. It is not a
//! GPU-versus-CPU benchmark unless `resolved_backend_name()` reports a GPU path.

use ruviz::core::BackendOperation;
use ruviz::prelude::*;
use std::time::Instant;

fn main() -> Result<()> {
    std::fs::create_dir_all("generated/examples")?;

    for point_count in [10_000, 100_000, 1_000_000] {
        diagnose_size(point_count)?;
    }

    Ok(())
}

fn diagnose_size(point_count: usize) -> Result<()> {
    let x: Vec<f64> = (0..point_count).map(|i| i as f64 * 0.00001).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&value| (value * 500.0).sin() + (value * 170.0).cos() * 0.5)
        .collect();

    let plot = Plot::new()
        .gpu(true)
        .line(&x, &y)
        .title(format!("GPU preference diagnostic: {point_count} points"))
        .into_plot();
    let resolution = plot.backend_resolution(BackendOperation::Png);

    println!("{point_count} points");
    println!("  requested: {}", plot.get_backend_name());
    println!("  resolved: {}", plot.resolved_backend_name());
    println!("  fallback: {:?}", resolution.fallback_reason());

    let actual_backend = plot.resolved_backend_name();
    let start = Instant::now();
    plot.save(format!(
        "generated/examples/backend_diagnostic_{point_count}.png"
    ))?;
    println!("  {actual_backend} PNG time: {:?}", start.elapsed());

    Ok(())
}
