//! GPU-accelerated plot example
//!
//! Demonstrates using GPU acceleration for large dataset rendering.
//!
//! Run with: cargo run --example gpu_plot_example --features gpu

use ruviz::prelude::*;

fn main() -> Result<()> {
    println!("=== GPU Plot Example ===\n");

    // Generate large dataset (GPU threshold is typically 5K+ points)
    let point_count = 10_000;
    let x_data: Vec<f64> = (0..point_count).map(|i| i as f64 * 0.001).collect();
    let y_data: Vec<f64> = x_data
        .iter()
        .map(|&x| (x * 10.0).sin() * (x * 3.0).cos())
        .collect();

    println!("Generated {} points", point_count);

    // Example 1: GPU rendering (with GPU feature)
    println!("\n--- Example 1: GPU Rendering ---");

    #[cfg(feature = "gpu")]
    {
        let result = Plot::new()
            .gpu(true) // Enable GPU acceleration first
            .line(&x_data, &y_data)
            .color(Color::new(31, 119, 180))
            .title("GPU-Accelerated Line Plot (10K points)")
            .xlabel("X")
            .ylabel("Y")
            .save("examples/output/gpu_plot_explicit.png");

        match result {
            Ok(_) => println!("Saved: examples/output/gpu_plot_explicit.png (GPU enabled)"),
            Err(e) => println!("GPU render failed (expected if no GPU): {}", e),
        }
    }

    #[cfg(not(feature = "gpu"))]
    {
        println!("GPU feature not enabled. Run with: --features gpu");

        // Fallback to CPU
        Plot::new()
            .line(&x_data, &y_data)
            .color(Color::new(31, 119, 180))
            .title("CPU Line Plot (10K points)")
            .xlabel("X")
            .ylabel("Y")
            .save("examples/output/gpu_plot_explicit.png")?;
        println!("Saved: examples/output/gpu_plot_explicit.png (CPU fallback)");
    }

    // Example 2: Auto-optimized rendering
    println!("\n--- Example 2: Auto-Optimized Rendering ---");

    let plot = Plot::new()
        .line(&x_data, &y_data)
        .color(Color::new(255, 127, 14))
        .title("Auto-Optimized Plot (10K points)")
        .xlabel("X")
        .ylabel("Y")
        .end_series()
        .auto_optimize();

    println!("Selected backend: {}", plot.get_backend_name());
    plot.save("examples/output/gpu_plot_auto.png")?;
    println!("Saved: examples/output/gpu_plot_auto.png");

    // Example 3: Very large dataset (100K points)
    println!("\n--- Example 3: Very Large Dataset (100K points) ---");

    let large_count = 100_000;
    let large_x: Vec<f64> = (0..large_count).map(|i| i as f64 * 0.0001).collect();
    let large_y: Vec<f64> = large_x
        .iter()
        .map(|&x| (x * 50.0).sin() + (x * 17.0).cos() * 0.5)
        .collect();

    println!("Generated {} points", large_count);

    let large_plot = Plot::new()
        .line(&large_x, &large_y)
        .color(Color::new(44, 160, 44))
        .title(&format!("Large Dataset ({} points)", large_count))
        .end_series()
        .auto_optimize();

    println!("Selected backend: {}", large_plot.get_backend_name());
    large_plot.save("examples/output/gpu_plot_large.png")?;
    println!("Saved: examples/output/gpu_plot_large.png");

    // Example 4: GPU vs CPU comparison
    println!("\n--- Example 4: GPU vs CPU Comparison ---");
    compare_gpu_cpu(&x_data, &y_data)?;

    println!("\n=== GPU Plot Examples Completed ===");
    Ok(())
}

/// Compare GPU and CPU rendering output
fn compare_gpu_cpu(x_data: &[f64], y_data: &[f64]) -> Result<()> {
    // Convert slices to owned Vecs for the API
    let x_vec: Vec<f64> = x_data.to_vec();
    let y_vec: Vec<f64> = y_data.to_vec();

    // CPU rendering
    let cpu_start = std::time::Instant::now();
    Plot::new()
        .backend(ruviz::core::plot::BackendType::Skia)
        .line(&x_vec, &y_vec)
        .color(Color::new(214, 39, 40))
        .title("CPU Rendered (Skia Backend)")
        .save("examples/output/gpu_comparison_cpu.png")?;
    let cpu_time = cpu_start.elapsed();
    println!("CPU render time: {:?}", cpu_time);
    println!("Saved: examples/output/gpu_comparison_cpu.png");

    // GPU rendering (if available)
    #[cfg(feature = "gpu")]
    {
        let gpu_start = std::time::Instant::now();
        let result = Plot::new()
            .gpu(true)
            .line(&x_vec, &y_vec)
            .color(Color::new(44, 160, 44))
            .title("GPU Rendered")
            .save("examples/output/gpu_comparison_gpu.png");

        match result {
            Ok(_) => {
                let gpu_time = gpu_start.elapsed();
                println!("GPU render time: {:?}", gpu_time);
                println!("Saved: examples/output/gpu_comparison_gpu.png");

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
            Err(e) => {
                println!("GPU render failed: {} (using CPU fallback)", e);
            }
        }
    }

    #[cfg(not(feature = "gpu"))]
    println!("GPU feature not enabled for comparison");

    Ok(())
}
