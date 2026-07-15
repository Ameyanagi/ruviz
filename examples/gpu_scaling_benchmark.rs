//! Lower-level `GpuRenderer` coordinate-transform scaling benchmark.
//!
//! Results apply only to the renderer utilities called in this file, not to
//! public `Plot::save()` or `Plot::render()` backend routing.

use ruviz::core::*;
use ruviz::data::*;
use ruviz::prelude::{Plot, Position};
use ruviz::render::gpu::{GpuRenderer, initialize_gpu_backend};
use ruviz::render::pooled::PooledRenderer;
use std::time::Instant;

#[derive(Debug, Clone)]
struct BenchmarkResult {
    point_count: usize,
    cpu_time_us: f64,
    cpu_throughput: f64,
    operation_time_us: Option<f64>,
    operation_path: OperationPath,
    gpu_throughput: Option<f64>,
    gpu_speedup: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
enum OperationPath {
    Gpu,
    CpuFallback,
    Failed,
    Unavailable,
    Unverified,
}

impl OperationPath {
    fn label(self) -> &'static str {
        match self {
            Self::Gpu => "GPU",
            Self::CpuFallback => "CPU fallback",
            Self::Failed => "FAILED",
            Self::Unavailable => "unavailable",
            Self::Unverified => "unverified",
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    std::fs::create_dir_all("generated/examples").ok();

    println!("GPU vs CPU Scaling Analysis");
    println!("===========================\n");

    let cpu_renderer = PooledRenderer::new();
    println!("CPU Renderer initialized");

    let mut gpu_renderer = match initialize_gpu_backend().await {
        Ok(_) => match GpuRenderer::new().await {
            Ok(renderer) => {
                println!(
                    "GPU Renderer initialized - threshold: {}",
                    renderer.gpu_threshold()
                );
                Some(renderer)
            }
            Err(e) => {
                println!("GPU Renderer failed: {}", e);
                None
            }
        },
        Err(e) => {
            println!("GPU Backend failed: {}", e);
            None
        }
    };

    let test_sizes = vec![
        500, 1_000, 2_000, 5_000, 10_000, 20_000, 50_000, 100_000, 200_000, 500_000, 1_000_000,
        2_000_000, 5_000_000,
    ];

    let mut results = Vec::new();

    for &point_count in &test_sizes {
        println!("\nTesting {} points", format_number(point_count as u64));

        let x_data: Vec<f64> = (0..point_count).map(|i| i as f64 * 0.001).collect();
        let y_data: Vec<f64> = x_data
            .iter()
            .map(|&x| (x * 2.0 * std::f64::consts::PI).sin())
            .collect();

        let x_range = (0.0, point_count as f64 * 0.001);
        let y_range = (-1.0, 1.0);
        let viewport = (0.0, 0.0, 1920.0, 1080.0);

        // CPU Benchmark
        print!("   CPU: ");
        let start = Instant::now();
        let _cpu_result = cpu_renderer.transform_coordinates_pooled(
            &x_data, &y_data, x_range.0, x_range.1, y_range.0, y_range.1, viewport.0, viewport.1,
            viewport.2, viewport.3,
        )?;
        let cpu_time = start.elapsed();
        let cpu_time_us = cpu_time.as_micros() as f64;
        let cpu_throughput = point_count as f64 / cpu_time.as_secs_f64();

        println!(
            "{:>10.0} us ({:>12.0} pts/sec)",
            cpu_time_us, cpu_throughput
        );

        // GPU Benchmark
        let (operation_time_us, operation_path, gpu_throughput, gpu_speedup) = if let Some(
            ref mut gpu,
        ) = gpu_renderer
        {
            print!("   Accelerated path: ");
            let gpu_operations_before = gpu.get_stats().gpu_operations;
            let cpu_operations_before = gpu.get_stats().cpu_operations;
            let start = Instant::now();

            match gpu.transform_coordinates_optimal(&x_data, &y_data, x_range, y_range, viewport) {
                Ok(_gpu_result) => {
                    let operation_time = start.elapsed();
                    let operation_time_us = operation_time.as_micros() as f64;
                    let stats = gpu.get_stats();
                    if stats.gpu_operations > gpu_operations_before {
                        let gpu_throughput = point_count as f64 / operation_time.as_secs_f64();
                        let speedup = cpu_time.as_secs_f64() / operation_time.as_secs_f64();
                        println!(
                            "GPU {:>10.0} us ({:>12.0} pts/sec) [{:.2}x speedup]",
                            operation_time_us, gpu_throughput, speedup
                        );
                        (
                            Some(operation_time_us),
                            OperationPath::Gpu,
                            Some(gpu_throughput),
                            Some(speedup),
                        )
                    } else if stats.cpu_operations > cpu_operations_before {
                        let fallback_throughput = point_count as f64 / operation_time.as_secs_f64();
                        println!(
                            "CPU fallback {:>10.0} us ({:>12.0} pts/sec); no GPU metric",
                            operation_time_us, fallback_throughput
                        );
                        (
                            Some(operation_time_us),
                            OperationPath::CpuFallback,
                            None,
                            None,
                        )
                    } else {
                        println!(
                            "completed in {:>10.0} us, but renderer statistics did not identify the path",
                            operation_time_us
                        );
                        (
                            Some(operation_time_us),
                            OperationPath::Unverified,
                            None,
                            None,
                        )
                    }
                }
                Err(e) => {
                    println!("FAILED: {}", e);
                    (None, OperationPath::Failed, None, None)
                }
            }
        } else {
            println!("   Accelerated path: GPU unavailable");
            (None, OperationPath::Unavailable, None, None)
        };

        results.push(BenchmarkResult {
            point_count,
            cpu_time_us,
            cpu_throughput,
            operation_time_us,
            operation_path,
            gpu_throughput,
            gpu_speedup,
        });

        let data_size = point_count * std::mem::size_of::<f64>() * 2;
        println!("   Memory: {:.1} MB", data_size as f64 / 1_000_000.0);

        if cpu_time.as_secs_f64() > 5.0 {
            println!("   CPU time > 5s, skipping larger datasets");
            break;
        }
    }

    // Print summary
    println!("\nPerformance Summary Table");
    println!("=========================");
    println!(
        "{:>10} {:>12} {:>14} {:>12} {:>12} {:>12} {:>10}",
        "Points", "CPU (us)", "Path", "Path (us)", "CPU (Mpts/s)", "GPU (Mpts/s)", "Speedup"
    );
    println!("{}", "-".repeat(98));

    for result in &results {
        let cpu_mpts = result.cpu_throughput / 1_000_000.0;
        let operation_time = result
            .operation_time_us
            .map_or_else(|| "--".to_string(), |time| format!("{time:.0}"));
        let gpu_mpts = result.gpu_throughput.map_or_else(
            || "--".to_string(),
            |throughput| format!("{:.1}", throughput / 1_000_000.0),
        );
        let speedup = result
            .gpu_speedup
            .map_or_else(|| "--".to_string(), |speedup| format!("{speedup:.2}x"));

        println!(
            "{:>10} {:>12.0} {:>14} {:>12} {:>12.1} {:>12} {:>10}",
            format_number(result.point_count as u64),
            result.cpu_time_us,
            result.operation_path.label(),
            operation_time,
            cpu_mpts,
            gpu_mpts,
            speedup
        );
    }

    create_performance_plot(&results)?;

    if let Some(gpu) = &gpu_renderer {
        let stats = gpu.get_stats();
        println!("\nGPU Statistics:");
        println!("  GPU Operations: {}", stats.gpu_operations);
        println!("  CPU Fallbacks: {}", stats.cpu_operations);
        println!(
            "  GPU Points: {}",
            format_number(stats.gpu_points_processed)
        );
        println!(
            "  CPU Points: {}",
            format_number(stats.cpu_points_processed)
        );
    }

    println!("\nScaling analysis complete! Check generated/examples/ for plots");
    Ok(())
}

fn create_performance_plot(results: &[BenchmarkResult]) -> Result<()> {
    let point_counts: Vec<f64> = results.iter().map(|r| r.point_count as f64).collect();
    let cpu_throughput: Vec<f64> = results
        .iter()
        .map(|r| r.cpu_throughput / 1_000_000.0)
        .collect();
    let (gpu_point_counts, gpu_throughput): (Vec<f64>, Vec<f64>) = results
        .iter()
        .filter_map(|result| {
            result
                .gpu_throughput
                .map(|throughput| (result.point_count as f64, throughput / 1_000_000.0))
        })
        .unzip();

    let throughput_plot = Plot::new()
        .title("GPU vs CPU Performance Scaling")
        .xlabel("Dataset Size (points)")
        .ylabel("Throughput (Million points/sec)")
        .legend(Position::TopLeft)
        .size(12.0, 6.0)
        .dpi(150)
        .line(&point_counts, &cpu_throughput)
        .label("CPU");
    let throughput_plot = if gpu_point_counts.is_empty() {
        throughput_plot
    } else {
        throughput_plot
            .line(&gpu_point_counts, &gpu_throughput)
            .label("GPU")
    };
    throughput_plot.save("generated/examples/gpu_throughput_scaling.png")?;

    let valid_speedups: Vec<_> = results
        .iter()
        .filter(|result| result.gpu_speedup.is_some())
        .collect();

    if !valid_speedups.is_empty() {
        let speedup_points: Vec<f64> = valid_speedups
            .iter()
            .map(|r| r.point_count as f64)
            .collect();
        let speedup_values: Vec<f64> = valid_speedups
            .iter()
            .filter_map(|result| result.gpu_speedup)
            .collect();

        Plot::new()
            .title("GPU Speedup vs Dataset Size")
            .xlabel("Dataset Size (points)")
            .ylabel("GPU Speedup (x)")
            .size(12.0, 6.0)
            .dpi(150)
            .scatter(&speedup_points, &speedup_values)
            .save("generated/examples/gpu_speedup_scaling.png")?;
    }

    println!("\nPerformance plots saved:");
    println!("  generated/examples/gpu_throughput_scaling.png");
    if !valid_speedups.is_empty() {
        println!("  generated/examples/gpu_speedup_scaling.png");
    }

    Ok(())
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();

    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(ch);
    }

    result
}
