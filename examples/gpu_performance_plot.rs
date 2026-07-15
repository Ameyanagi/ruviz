//! Print backend-resolution diagnostics for representative dataset sizes.
//!
//! This replaces the former chart of mixed measured and projected GPU values.

use ruviz::core::BackendOperation;
use ruviz::prelude::*;

fn main() -> Result<()> {
    for point_count in [1_000, 10_000, 100_000] {
        let x: Vec<f64> = (0..point_count).map(|i| i as f64 * 0.001).collect();
        let y: Vec<f64> = x.iter().map(|value| value.sin()).collect();
        let plot = Plot::new().gpu(true).line(&x, &y).into_plot();
        let resolution = plot.backend_resolution(BackendOperation::Png);

        println!("{point_count} points");
        println!("  requested: {}", plot.get_backend_name());
        println!("  resolved: {}", plot.resolved_backend_name());
        println!("  fallback: {:?}", resolution.fallback_reason());
    }

    println!("No GPU throughput or speedup is inferred from preference metadata.");
    Ok(())
}
