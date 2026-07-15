//! Minimal diagnostic for the public GPU preference contract.
//!
//! No synthetic GPU timings or theoretical speedups are reported. Public raster
//! operations currently resolve the preference to Skia.

use ruviz::core::BackendOperation;
use ruviz::prelude::*;

fn main() {
    let x: Vec<f64> = (0..10_000).map(|i| i as f64 * 0.001).collect();
    let y: Vec<f64> = x.iter().map(|value| value.sin()).collect();
    let plot = Plot::new().gpu(true).line(&x, &y).into_plot();
    let resolution = plot.backend_resolution(BackendOperation::Png);

    println!("requested: {}", plot.get_backend_name());
    println!("resolved: {}", plot.resolved_backend_name());
    println!("fallback: {:?}", resolution.fallback_reason());
    println!(
        "Benchmark only the resolved backend; this build does not expose a public GPU PNG path."
    );
}
