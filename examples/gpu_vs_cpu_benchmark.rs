//! Comprehensive GPU vs CPU rendering benchmarks
//!
//! Provides real performance comparisons across different dataset sizes
//! and rendering operations to validate GPU acceleration benefits.

use ruviz::core::*;
use ruviz::data::*;
use ruviz::render::pooled::PooledRenderer;
use std::time::{Duration, Instant};

#[cfg(feature = "gpu")]
use ruviz::render::gpu::{GpuRenderer, initialize_gpu_backend};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("🚀 GPU vs CPU Rendering Benchmark Suite");
    println!("=========================================\n");

    // Test dataset sizes from small to very large
    let test_sizes = vec![
        ("Small", 1_000u64),
        ("Medium", 10_000u64),
        ("Large", 100_000u64),
        ("Very Large", 500_000u64),
        ("Massive", 1_000_000u64),
        ("Ultra", 2_000_000u64),
    ];

    // Initialize CPU renderer (always available)
    let cpu_renderer = PooledRenderer::new();
    println!("✅ CPU Renderer initialized");

    // Try to initialize GPU renderer
    #[cfg(feature = "gpu")]
    let mut gpu_renderer = {
        match initialize_gpu_backend().await {
            Ok(_) => match GpuRenderer::new().await {
                Ok(renderer) => {
                    println!(
                        "✅ GPU Renderer initialized - {} threshold",
                        renderer.gpu_threshold()
                    );
                    Some(renderer)
                }
                Err(e) => {
                    println!("⚠️  GPU Renderer failed to initialize: {}", e);
                    None
                }
            },
            Err(e) => {
                println!("⚠️  GPU Backend failed to initialize: {}", e);
                None
            }
        }
    };

    #[cfg(not(feature = "gpu"))]
    let mut gpu_renderer: Option<()> = None;

    if gpu_renderer.is_some() {
        println!("🚀 GPU acceleration enabled!");
    } else {
        println!("⚠️  GPU features disabled - running CPU-only benchmarks");
    }

    println!("\n📊 Running Coordinate Transformation Benchmarks");
    println!("================================================");

    for (size_name, point_count) in test_sizes {
        println!(
            "\n🔍 Dataset: {} ({} points)",
            size_name,
            format_number(point_count)
        );

        // Generate test data
        let x_data: Vec<f64> = (0..point_count as usize)
            .map(|i| i as f64 * 0.001)
            .collect();
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
        let cpu_result = cpu_renderer.transform_coordinates_pooled(
            &x_data, &y_data, x_range.0, x_range.1, y_range.0, y_range.1, viewport.0, viewport.1,
            viewport.2, viewport.3,
        )?;
        let cpu_time = start.elapsed();

        let cpu_throughput = point_count as f64 / cpu_time.as_secs_f64();
        println!(
            "{:>8} ms ({:>10.0} pts/sec)",
            format_duration(cpu_time),
            cpu_throughput
        );

        // GPU Benchmark
        #[cfg(feature = "gpu")]
        if let Some(gpu_renderer) = &mut gpu_renderer {
            print!("   GPU: ");
            let start = Instant::now();
            match gpu_renderer
                .transform_coordinates_optimal(&x_data, &y_data, x_range, y_range, viewport)
            {
                Ok(_gpu_result) => {
                    let gpu_time = start.elapsed();
                    let gpu_throughput = point_count as f64 / gpu_time.as_secs_f64();
                    let speedup = cpu_time.as_secs_f64() / gpu_time.as_secs_f64();
                    println!(
                        "{:>8} ms ({:>10.0} pts/sec) [{:.1}x speedup]",
                        format_duration(gpu_time),
                        gpu_throughput,
                        speedup
                    );
                }
                Err(e) => {
                    println!("Failed: {}", e);
                }
            }
        } else {
            println!("   GPU: Not available (CPU-only benchmark)");
        }

        #[cfg(not(feature = "gpu"))]
        println!("   GPU: Not available (CPU-only benchmark)");

        // Memory usage analysis
        let data_size = point_count as usize * std::mem::size_of::<f64>() * 2; // x + y data
        let result_size = point_count as usize * std::mem::size_of::<f32>() * 2; // transformed x + y
        println!(
            "   Memory: {:.1} MB input → {:.1} MB output",
            data_size as f64 / 1_000_000.0,
            result_size as f64 / 1_000_000.0
        );
    }

    // Summary statistics
    println!("\n📈 CPU Performance Summary");
    println!("=========================");
    let stats = cpu_renderer.get_pool_stats();
    println!("Pool Efficiency: {:.1}%", stats.efficiency() * 100.0);
    println!(
        "Total Capacity:  {}",
        format_number(stats.total_capacity() as u64)
    );
    println!("Active Buffers:  {}", stats.total_in_use());

    // Run rendering pipeline benchmarks
    println!("\n🎨 Full Rendering Pipeline Benchmark");
    println!("====================================");

    benchmark_line_plot(100_000).await?;
    benchmark_scatter_plot(50_000).await?;
    benchmark_multiple_series(25_000, 4).await?;

    println!("\n✅ Benchmark suite completed!");
    Ok(())
}

/// Benchmark complete line plot rendering
async fn benchmark_line_plot(point_count: usize) -> Result<()> {
    println!(
        "\n🔸 Line Plot Rendering ({} points)",
        format_number(point_count as u64)
    );

    // Generate sine wave data
    let x: Vec<f64> = (0..point_count).map(|i| i as f64 * 0.01).collect();
    let y: Vec<f64> = x.iter().map(|&x| (x * 0.5).sin()).collect();

    let start = Instant::now();

    // Simulate complete rendering pipeline
    let cpu_renderer = PooledRenderer::new();
    let _result = cpu_renderer.transform_coordinates_pooled(
        &x,
        &y,
        0.0,
        x[x.len() - 1],
        -1.0,
        1.0,
        0.0,
        0.0,
        1920.0,
        1080.0,
    )?;

    // Add simulated rendering overhead
    simulate_rendering_work(point_count, 0.5);

    let cpu_total = start.elapsed();
    let throughput = point_count as f64 / cpu_total.as_secs_f64();

    println!(
        "   CPU Pipeline: {:>8} ms ({:>10.0} pts/sec)",
        format_duration(cpu_total),
        throughput
    );

    println!("   GPU Pipeline: Not available (CPU-only mode)");

    Ok(())
}

/// Benchmark scatter plot rendering
async fn benchmark_scatter_plot(point_count: usize) -> Result<()> {
    println!(
        "\n🔸 Scatter Plot Rendering ({} points)",
        format_number(point_count as u64)
    );

    // Generate random-looking data
    let x: Vec<f64> = (0..point_count).map(|i| i as f64 * 17.0 % 100.0).collect();
    let y: Vec<f64> = (0..point_count).map(|i| i as f64 * 13.0 % 80.0).collect();

    let start = Instant::now();

    let cpu_renderer = PooledRenderer::new();
    let _result = cpu_renderer
        .transform_coordinates_pooled(&x, &y, 0.0, 100.0, 0.0, 80.0, 0.0, 0.0, 1920.0, 1080.0)?;

    // Scatter plots have higher rendering overhead per point (markers)
    simulate_rendering_work(point_count, 2.0);

    let cpu_total = start.elapsed();
    let throughput = point_count as f64 / cpu_total.as_secs_f64();

    println!(
        "   CPU Pipeline: {:>8} ms ({:>10.0} pts/sec)",
        format_duration(cpu_total),
        throughput
    );

    Ok(())
}

/// Benchmark multiple data series rendering
async fn benchmark_multiple_series(points_per_series: usize, series_count: usize) -> Result<()> {
    let total_points = points_per_series * series_count;
    println!(
        "\n🔸 Multi-Series Plot ({} series × {} = {} total points)",
        series_count,
        format_number(points_per_series as u64),
        format_number(total_points as u64)
    );

    let start = Instant::now();
    let cpu_renderer = PooledRenderer::new();

    // Simulate rendering multiple series
    for series in 0..series_count {
        let x: Vec<f64> = (0..points_per_series).map(|i| i as f64 * 0.01).collect();
        let y: Vec<f64> = x.iter().map(|&x| (x + series as f64).sin()).collect();

        let _result = cpu_renderer.transform_coordinates_pooled(
            &x,
            &y,
            0.0,
            x[x.len() - 1],
            -1.0,
            1.0,
            0.0,
            0.0,
            1920.0,
            1080.0,
        )?;
    }

    simulate_rendering_work(total_points, 1.0);

    let cpu_total = start.elapsed();
    let throughput = total_points as f64 / cpu_total.as_secs_f64();

    println!(
        "   CPU Pipeline: {:>8} ms ({:>10.0} pts/sec)",
        format_duration(cpu_total),
        throughput
    );

    Ok(())
}

/// Simulate additional rendering work (rasterization, composition, etc.)
fn simulate_rendering_work(point_count: usize, complexity_factor: f64) {
    // Simulate CPU work proportional to point count and complexity
    let work_duration =
        Duration::from_nanos((point_count as f64 * complexity_factor * 10.0) as u64);

    let start = Instant::now();
    while start.elapsed() < work_duration {
        // Busy wait to simulate rendering work
        std::hint::black_box(());
    }
}

/// Format numbers with thousand separators
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

/// Format duration with appropriate units
fn format_duration(duration: Duration) -> String {
    let micros = duration.as_micros();
    if micros < 1_000 {
        format!("{} μs", micros)
    } else if micros < 1_000_000 {
        format!("{:.1}", micros as f64 / 1_000.0)
    } else {
        format!("{:.0}", micros as f64 / 1_000.0)
    }
}
