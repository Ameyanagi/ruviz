//! GPU-accelerated plot example
//!
//! Demonstrates using GPU acceleration for large dataset rendering.
//!
//! Run with: cargo run --example gpu_plot_example --features gpu

use ruviz::prelude::*;

fn main() -> Result<()> {
    println!("=== GPU Plot Example ===\n");
    std::fs::create_dir_all("examples/output").ok();

    // Generate large dataset (GPU threshold is typically 5K+ points)
    let point_count = 10_000;
    let x: Vec<f64> = (0..point_count).map(|i| i as f64 * 0.001).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&t| (t * 10.0).sin() * (t * 3.0).cos())
        .collect();

    println!("Generated {} points", point_count);

    // Example 1: GPU rendering (with GPU feature)
    println!("\n--- Example 1: GPU Rendering ---");

    #[cfg(feature = "gpu")]
    {
        let result = Plot::new()
            .title("GPU-Accelerated Line Plot (10K points)")
            .xlabel("X")
            .ylabel("Y")
            .gpu(true)
            .line(&x, &y)
            .color(Color::new(31, 119, 180))
            .save("examples/output/gpu_plot_explicit.png");

        match result {
            Ok(_) => println!("Saved: examples/output/gpu_plot_explicit.png (GPU enabled)"),
            Err(e) => println!("GPU render failed (expected if no GPU): {}", e),
        }
    }

    #[cfg(not(feature = "gpu"))]
    {
        println!("GPU feature not enabled. Run with: --features gpu");
        Plot::new()
            .title("CPU Line Plot (10K points)")
            .xlabel("X")
            .ylabel("Y")
            .line(&x, &y)
            .color(Color::new(31, 119, 180))
            .save("examples/output/gpu_plot_explicit.png")?;
        println!("Saved: examples/output/gpu_plot_explicit.png (CPU fallback)");
    }

    // Example 2: Auto-optimized rendering
    println!("\n--- Example 2: Auto-Optimized Rendering ---");

    let plot: Plot = Plot::new()
        .title("Auto-Optimized Plot (10K points)")
        .xlabel("X")
        .ylabel("Y")
        .line(&x, &y)
        .color(Color::new(255, 127, 14))
        .into();

    let plot = plot.auto_optimize();
    println!("Selected backend: {}", plot.get_backend_name());
    plot.save("examples/output/gpu_plot_auto.png")?;
    println!("Saved: examples/output/gpu_plot_auto.png");

    // Example 3: Very large dataset (100K points)
    println!("\n--- Example 3: Very Large Dataset (100K points) ---");

    let large_count = 100_000;
    let large_x: Vec<f64> = (0..large_count).map(|i| i as f64 * 0.0001).collect();
    let large_y: Vec<f64> = large_x
        .iter()
        .map(|&t| (t * 50.0).sin() + (t * 17.0).cos() * 0.5)
        .collect();

    println!("Generated {} points", large_count);

    let large_plot: Plot = Plot::new()
        .title(&format!("Large Dataset ({} points)", large_count))
        .line(&large_x, &large_y)
        .color(Color::new(44, 160, 44))
        .into();

    let large_plot = large_plot.auto_optimize();
    println!("Selected backend: {}", large_plot.get_backend_name());
    large_plot.save("examples/output/gpu_plot_large.png")?;
    println!("Saved: examples/output/gpu_plot_large.png");

    // Example 4: GPU vs CPU comparison
    println!("\n--- Example 4: GPU vs CPU Comparison ---");
    compare_gpu_cpu(&x, &y)?;

    println!("\n=== GPU Plot Examples Completed ===");
    Ok(())
}

fn compare_gpu_cpu(x: &[f64], y: &[f64]) -> Result<()> {
    let x_vec: Vec<f64> = x.to_vec();
    let y_vec: Vec<f64> = y.to_vec();

    // CPU rendering
    let cpu_start = std::time::Instant::now();
    Plot::new()
        .title("CPU Rendered (Skia Backend)")
        .backend(ruviz::core::plot::BackendType::Skia)
        .line(&x_vec, &y_vec)
        .color(Color::new(214, 39, 40))
        .save("examples/output/gpu_comparison_cpu.png")?;
    let cpu_time = cpu_start.elapsed();
    println!("CPU render time: {:?}", cpu_time);

    #[cfg(feature = "gpu")]
    {
        let gpu_start = std::time::Instant::now();
        let result = Plot::new()
            .title("GPU Rendered")
            .gpu(true)
            .line(&x_vec, &y_vec)
            .color(Color::new(44, 160, 44))
            .save("examples/output/gpu_comparison_gpu.png");

        match result {
            Ok(_) => {
                let gpu_time = gpu_start.elapsed();
                println!("GPU render time: {:?}", gpu_time);
                if gpu_time < cpu_time {
                    println!(
                        "GPU was {:.1}x faster",
                        cpu_time.as_secs_f64() / gpu_time.as_secs_f64()
                    );
                } else {
                    println!(
                        "CPU was {:.1}x faster (GPU overhead for small data)",
                        gpu_time.as_secs_f64() / cpu_time.as_secs_f64()
                    );
                }
            }
            Err(e) => println!("GPU render failed: {} (using CPU fallback)", e),
        }
    }

    #[cfg(not(feature = "gpu"))]
    println!("GPU feature not enabled for comparison");

    Ok(())
}
