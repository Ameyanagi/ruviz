use ruviz::core::{BackendOperation, BackendType, IntoPlot, Plot};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..50_000).map(|i| i as f64 * 0.01).collect();
    let y: Vec<f64> = x.iter().map(|value| value.sin()).collect();

    let plot = Plot::new()
        .backend(BackendType::Parallel)
        .line(&x, &y)
        .into_plot();
    let resolution = plot.backend_resolution(BackendOperation::Png);

    println!("SIMD/parallel utility feature diagnostic");
    println!("  requested: {}", plot.get_backend_name());
    println!("  resolved: {}", plot.resolved_backend_name());
    println!("  fallback: {:?}", resolution.fallback_reason());
    println!(
        "The simd feature exposes renderer utilities; public raster routing currently preserves Skia."
    );

    Ok(())
}
