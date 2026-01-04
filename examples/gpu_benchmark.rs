//! GPU vs CPU benchmark for large datasets

use ruviz::prelude::*;
use std::time::Instant;

fn main() -> Result<()> {
    println!("=== GPU vs CPU Benchmark ===\n");

    for &point_count in &[10_000, 50_000, 100_000, 500_000, 1_000_000] {
        benchmark_size(point_count)?;
    }

    println!("\n=== Benchmark Complete ===");
    Ok(())
}

fn benchmark_size(point_count: usize) -> Result<()> {
    // Generate data
    let x_data: Vec<f64> = (0..point_count).map(|i| i as f64 * 0.00001).collect();
    let y_data: Vec<f64> = x_data
        .iter()
        .map(|&x| (x * 500.0).sin() + (x * 170.0).cos() * 0.5)
        .collect();

    println!("--- {} points ---", point_count);

    // CPU rendering (Skia)
    let cpu_start = Instant::now();
    Plot::new()
        .backend(ruviz::core::plot::BackendType::Skia)
        .line(&x_data, &y_data)
        .title(&format!("CPU - {} points", point_count))
        .save(&format!("/tmp/bench_cpu_{}.png", point_count))?;
    let cpu_time = cpu_start.elapsed();

    // GPU rendering
    #[cfg(feature = "gpu")]
    let gpu_time = {
        let gpu_start = Instant::now();
        let result = Plot::new()
            .gpu(true)
            .line(&x_data, &y_data)
            .title(&format!("GPU - {} points", point_count))
            .save(&format!("/tmp/bench_gpu_{}.png", point_count));

        match result {
            Ok(_) => Some(gpu_start.elapsed()),
            Err(e) => {
                println!("  GPU failed: {}", e);
                None
            }
        }
    };

    println!("  CPU: {:>8.2?}", cpu_time);

    #[cfg(feature = "gpu")]
    if let Some(gpu) = gpu_time {
        println!("  GPU: {:>8.2?}", gpu);
        let ratio = cpu_time.as_secs_f64() / gpu.as_secs_f64();
        if ratio > 1.0 {
            println!("  Winner: GPU ({:.2}x faster)", ratio);
        } else {
            println!("  Winner: CPU ({:.2}x faster)", 1.0 / ratio);
        }
    }

    Ok(())
}
