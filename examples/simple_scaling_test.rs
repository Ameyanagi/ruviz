//! Simple GPU vs CPU scaling test

use ruviz::core::*;
use ruviz::data::*;
use ruviz::render::gpu::{GpuRenderer, initialize_gpu_backend};
use ruviz::render::pooled::PooledRenderer;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    println!("📊 GPU vs CPU Scaling Test");
    println!("===========================\n");

    // Initialize renderers
    let cpu_renderer = PooledRenderer::new();
    println!("✅ CPU Renderer ready");

    let mut gpu_renderer = match initialize_gpu_backend().await {
        Ok(_) => match GpuRenderer::new().await {
            Ok(renderer) => {
                println!(
                    "✅ GPU Renderer ready - threshold: {}",
                    renderer.gpu_threshold()
                );
                Some(renderer)
            }
            Err(e) => {
                println!("⚠️ GPU failed: {}", e);
                None
            }
        },
        Err(e) => {
            println!("⚠️ GPU backend failed: {}", e);
            None
        }
    };

    // Test various dataset sizes
    let test_sizes = vec![1_000, 5_000, 10_000, 50_000, 100_000, 500_000];

    println!(
        "\n{:>10} {:>15} {:>15} {:>10}",
        "Points", "CPU (ms)", "GPU (ms)", "Speedup"
    );
    println!("{}", "-".repeat(60));

    for &point_count in &test_sizes {
        // Generate sine wave data
        let x_data: Vec<f64> = (0..point_count).map(|i| i as f64 * 0.01).collect();
        let y_data: Vec<f64> = x_data.iter().map(|&x| (x * 0.5).sin()).collect();

        let x_range = (0.0, x_data[x_data.len() - 1]);
        let y_range = (-1.0, 1.0);
        let viewport = (0.0, 0.0, 1920.0, 1080.0);

        // CPU benchmark
        let start = Instant::now();
        let _cpu_result = cpu_renderer.transform_coordinates_pooled(
            &x_data, &y_data, x_range.0, x_range.1, y_range.0, y_range.1, viewport.0, viewport.1,
            viewport.2, viewport.3,
        )?;
        let cpu_time = start.elapsed();

        // GPU benchmark
        let (gpu_time, speedup) = if let Some(ref mut gpu) = gpu_renderer {
            let start = Instant::now();
            match gpu.transform_coordinates_optimal(&x_data, &y_data, x_range, y_range, viewport) {
                Ok(_) => {
                    let gpu_time = start.elapsed();
                    let speedup = cpu_time.as_secs_f64() / gpu_time.as_secs_f64();
                    (gpu_time, speedup)
                }
                Err(_) => (cpu_time, 1.0), // Failed, use CPU time
            }
        } else {
            (cpu_time, 1.0)
        };

        println!(
            "{:>10} {:>12.1} {:>12.1} {:>9.1}x",
            format_number(point_count as u64),
            cpu_time.as_millis() as f64,
            gpu_time.as_millis() as f64,
            speedup
        );

        // Stop if CPU takes too long
        if cpu_time.as_secs_f64() > 2.0 {
            println!("CPU time > 2s, stopping here");
            break;
        }
    }

    // Show GPU stats if available
    if let Some(gpu) = &gpu_renderer {
        let stats = gpu.get_stats();
        println!("\n📈 GPU Statistics:");
        println!("GPU operations: {}", stats.gpu_operations);
        println!("CPU operations: {}", stats.cpu_operations);
        println!(
            "Total GPU points: {}",
            format_number(stats.gpu_points_processed)
        );
        println!(
            "Total CPU points: {}",
            format_number(stats.cpu_points_processed)
        );
    }

    Ok(())
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();

    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }

    result
}
