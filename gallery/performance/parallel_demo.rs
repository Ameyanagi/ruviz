use ruviz::core::{BackendOperation, BackendType, IntoPlot, Plot};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..75_000).map(|i| i as f64 / 1_000.0).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&value| value.sin() + 0.5 * (value * 3.0).cos())
        .collect();

    let parallel = Plot::new()
        .backend(BackendType::Parallel)
        .line(&x, &y)
        .into_plot();
    report("Parallel preference", &parallel);
    let image = parallel.render()?;
    println!(
        "Rendered {}x{} through the resolved backend",
        image.width, image.height
    );

    let scatter = Plot::new()
        .backend(BackendType::DataShader)
        .scatter(&x, &y)
        .into_plot();
    report("Explicit DataShader scatter PNG", &scatter);

    Ok(())
}

fn report(label: &str, plot: &Plot) {
    let resolution = plot.backend_resolution(BackendOperation::Png);
    println!("{label}");
    println!("  requested: {}", plot.get_backend_name());
    println!("  resolved: {}", plot.resolved_backend_name());
    println!("  fallback: {:?}", resolution.fallback_reason());
}
