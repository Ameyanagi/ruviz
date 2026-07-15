//! Backend-preference diagnostics for builds with the `gpu` feature.
//!
//! Public `Plot::save()` and `Plot::render()` currently resolve a GPU preference
//! to the Skia reference raster path. This example reports that decision instead
//! of presenting the same path as a GPU/CPU performance comparison.
//!
//! Run with: `cargo run --example gpu_plot_example --features gpu`

use ruviz::core::{BackendOperation, BackendType};
use ruviz::prelude::*;

fn report(label: &str, plot: &Plot) {
    let resolution = plot.backend_resolution(BackendOperation::Png);
    println!("{label}");
    println!("  requested: {}", plot.get_backend_name());
    println!("  resolved: {}", plot.resolved_backend_name());
    println!("  fallback: {:?}", resolution.fallback_reason());
}

fn main() -> Result<()> {
    std::fs::create_dir_all("generated/examples")?;

    let x: Vec<f64> = (0..10_000).map(|i| i as f64 * 0.001).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&t| (t * 10.0).sin() * (t * 3.0).cos())
        .collect();

    let gpu_preference = Plot::new()
        .gpu(true)
        .line(&x, &y)
        .title("GPU preference resolved through public PNG routing")
        .into_plot();
    report("GPU preference", &gpu_preference);
    gpu_preference.save("generated/examples/gpu_preference_resolved.png")?;

    let auto = Plot::new()
        .line(&x, &y)
        .title("Conservative automatic backend selection")
        .into_plot()
        .auto_optimize();
    report("Automatic selection", &auto);
    auto.save("generated/examples/backend_auto_resolved.png")?;

    let scatter_x: Vec<f64> = (0..100_000).map(|i| i as f64 * 0.0001).collect();
    let scatter_y: Vec<f64> = scatter_x.iter().map(|&t| (t * 25.0).sin()).collect();
    let datashader = Plot::new()
        .backend(BackendType::DataShader)
        .scatter(&scatter_x, &scatter_y)
        .title("Explicit compatible DataShader PNG")
        .into_plot();
    report("Explicit DataShader", &datashader);
    datashader.save("generated/examples/datashader_explicit.png")?;

    Ok(())
}
