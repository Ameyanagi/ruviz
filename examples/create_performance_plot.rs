//! Demonstrate truthful backend labels for public PNG output.
//!
//! Point count does not create an automatic GPU threshold. The labels printed by
//! this example come from the same resolution API used by `Plot::save()`.

use ruviz::core::{BackendOperation, BackendType};
use ruviz::prelude::*;

fn main() -> Result<()> {
    let x: Vec<f64> = (0..50_000).map(|i| i as f64 * 0.001).collect();
    let y: Vec<f64> = x.iter().map(|value| value.cos()).collect();

    report(
        "automatic",
        Plot::new().line(&x, &y).into_plot().auto_optimize(),
    );
    report(
        "parallel preference",
        Plot::new()
            .backend(BackendType::Parallel)
            .line(&x, &y)
            .into_plot(),
    );
    report(
        "GPU preference",
        Plot::new()
            .backend(BackendType::GPU)
            .line(&x, &y)
            .into_plot(),
    );

    Ok(())
}

fn report(label: &str, plot: Plot) {
    let resolution = plot.backend_resolution(BackendOperation::Png);
    println!("{label}");
    println!("  requested: {}", plot.get_backend_name());
    println!("  resolved: {}", plot.resolved_backend_name());
    println!("  fallback: {:?}", resolution.fallback_reason());
}
