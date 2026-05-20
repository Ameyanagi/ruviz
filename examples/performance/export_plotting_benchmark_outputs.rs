use std::fs;
use std::path::Path;

use ruviz::prelude::*;

fn triangle_wave(index: usize, period: usize) -> f64 {
    let phase = (index % period) as f64 / period as f64;
    1.0 - 4.0 * (phase - 0.5).abs()
}

fn line_dataset(points: usize) -> (Vec<f64>, Vec<f64>) {
    let divisor = points.saturating_sub(1).max(1) as f64;
    let x: Vec<f64> = (0..points)
        .map(|index| index as f64 * (200.0 / divisor))
        .collect();
    let y: Vec<f64> = (0..points)
        .map(|index| {
            triangle_wave(index, 1024)
                + 0.35 * triangle_wave(index, 257)
                + 0.1 * triangle_wave(index, 61)
        })
        .collect();
    (x, y)
}

fn scatter_dataset(points: usize) -> (Vec<f64>, Vec<f64>) {
    let modulus = 2_147_483_647_u64;
    (0..points)
        .map(|index| {
            let x_raw = (index as u64 * 48_271) % modulus;
            let noise_raw = (index as u64 * 69_621 + 12_345) % modulus;
            let x_value = x_raw as f64 / modulus as f64;
            let noise = noise_raw as f64 / modulus as f64;
            let band = (index % 11) as f64 / 10.0 - 0.5;
            let y_value = (0.62 * x_value + 0.25 * noise + 0.13 * band).clamp(0.0, 1.0);
            (x_value, y_value)
        })
        .unzip()
}

fn save_line_100k(output_dir: &Path) -> Result<()> {
    let (x, y) = line_dataset(100_000);
    let plot = Plot::new()
        .size_px(640, 480)
        .dpi(100)
        .line(&x, &y)
        .auto_optimize()
        .into_plot();
    println!("line 100k backend: {}", plot.resolved_backend_name());
    plot.save(output_dir.join("line-100k.png"))
}

fn save_scatter_100k(output_dir: &Path) -> Result<()> {
    let (x, y) = scatter_dataset(100_000);
    let auto = Plot::new()
        .size_px(640, 480)
        .dpi(100)
        .scatter(&x, &y)
        .auto_optimize()
        .into_plot();
    println!(
        "scatter 100k auto backend: {}",
        auto.resolved_backend_name()
    );
    auto.save(output_dir.join("scatter-100k-auto.png"))?;

    let datashader = Plot::new()
        .size_px(640, 480)
        .dpi(100)
        .backend(BackendType::DataShader)
        .scatter(&x, &y)
        .into_plot();
    println!(
        "scatter 100k explicit datashader backend: {}",
        datashader.resolved_backend_name()
    );
    datashader.save(output_dir.join("scatter-100k-datashader.png"))
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let output_dir = Path::new("target/plotting-benchmark-output");
    fs::create_dir_all(output_dir)?;

    save_line_100k(output_dir)?;
    save_scatter_100k(output_dir)?;

    println!("wrote {}", output_dir.display());
    Ok(())
}
