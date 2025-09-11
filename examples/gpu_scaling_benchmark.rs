//! GPU vs CPU scaling benchmark with performance plotting

use std::time::Instant;
use ruviz::core::*;
use ruviz::data::*;
use ruviz::render::gpu::{GpuRenderer, initialize_gpu_backend};
use ruviz::render::pooled::PooledRenderer;

#[derive(Debug, Clone)]
struct BenchmarkResult {
    point_count: usize,
    cpu_time_us: f64,
    gpu_time_us: f64,
    cpu_throughput: f64,
    gpu_throughput: f64,
    gpu_speedup: f64,
    gpu_success: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("📊 GPU vs CPU Scaling Analysis");
    println!("===============================\n");
    
    // Initialize renderers
    let mut cpu_renderer = PooledRenderer::new();
    println!("✅ CPU Renderer initialized");
    
    let mut gpu_renderer = match initialize_gpu_backend().await {
        Ok(_) => {
            match GpuRenderer::new().await {
                Ok(renderer) => {
                    println!("✅ GPU Renderer initialized - threshold: {}", renderer.gpu_threshold());
                    Some(renderer)
                },
                Err(e) => {
                    println!("⚠️ GPU Renderer failed: {}", e);
                    None
                }
            }
        },
        Err(e) => {
            println!("⚠️ GPU Backend failed: {}", e);
            None
        }
    };
    
    // Test datasets from small to very large
    let test_sizes = vec![
        500, 1_000, 2_000, 5_000, 10_000, 20_000, 50_000, 
        100_000, 200_000, 500_000, 1_000_000, 2_000_000, 5_000_000
    ];
    
    let mut results = Vec::new();
    
    for &point_count in &test_sizes {
        println!("\n🔍 Testing {} points", format_number(point_count as u64));
        
        // Generate test data
        let x_data: Vec<f64> = (0..point_count)
            .map(|i| i as f64 * 0.001)
            .collect();
        let y_data: Vec<f64> = x_data.iter()
            .map(|&x| (x * 2.0 * std::f64::consts::PI).sin())
            .collect();
        
        let x_range = (0.0, point_count as f64 * 0.001);
        let y_range = (-1.0, 1.0);
        let viewport = (0.0, 0.0, 1920.0, 1080.0);
        
        // CPU Benchmark
        print!("   CPU: ");
        let start = Instant::now();
        let _cpu_result = cpu_renderer.transform_coordinates_pooled(
            &x_data, &y_data, x_range.0, x_range.1, y_range.0, y_range.1,
            viewport.0, viewport.1, viewport.2, viewport.3
        )?;
        let cpu_time = start.elapsed();
        let cpu_time_us = cpu_time.as_micros() as f64;
        let cpu_throughput = point_count as f64 / cpu_time.as_secs_f64();
        
        println!("{:>10.0} μs ({:>12.0} pts/sec)", cpu_time_us, cpu_throughput);
        
        // GPU Benchmark
        let (gpu_time_us, gpu_throughput, gpu_speedup, gpu_success) = if let Some(ref mut gpu) = gpu_renderer {
            print!("   GPU: ");
            let start = Instant::now();
            
            match gpu.transform_coordinates_optimal(&x_data, &y_data, x_range, y_range, viewport) {
                Ok(_gpu_result) => {
                    let gpu_time = start.elapsed();
                    let gpu_time_us = gpu_time.as_micros() as f64;
                    let gpu_throughput = point_count as f64 / gpu_time.as_secs_f64();
                    let speedup = cpu_time.as_secs_f64() / gpu_time.as_secs_f64();
                    
                    println!("{:>10.0} μs ({:>12.0} pts/sec) [{:.2}x speedup]", 
                        gpu_time_us, gpu_throughput, speedup);
                    
                    (gpu_time_us, gpu_throughput, speedup, true)
                },
                Err(e) => {
                    println!("FAILED: {}", e);
                    (cpu_time_us, cpu_throughput, 1.0, false)
                }
            }
        } else {
            println!("   GPU: Not available");
            (cpu_time_us, cpu_throughput, 1.0, false)
        };
        
        results.push(BenchmarkResult {
            point_count,
            cpu_time_us,
            gpu_time_us,
            cpu_throughput,
            gpu_throughput,
            gpu_speedup,
            gpu_success,
        });
        
        // Memory usage estimate
        let data_size = point_count * std::mem::size_of::<f64>() * 2;
        println!("   Memory: {:.1} MB", data_size as f64 / 1_000_000.0);
        
        // Early exit for very slow operations
        if cpu_time.as_secs() > 5.0 {
            println!("   ⏰ CPU time > 5s, skipping larger datasets");
            break;
        }
    }
    
    // Print summary table
    println!("\n📈 Performance Summary Table");
    println!("============================");
    println!("{:>10} {:>12} {:>12} {:>12} {:>12} {:>10}", 
        "Points", "CPU (μs)", "GPU (μs)", "CPU (Mpts/s)", "GPU (Mpts/s)", "Speedup");
    println!("{}", "-".repeat(80));
    
    for result in &results {
        let cpu_mpts = result.cpu_throughput / 1_000_000.0;
        let gpu_mpts = if result.gpu_success { result.gpu_throughput / 1_000_000.0 } else { 0.0 };
        let speedup_str = if result.gpu_success { format!("{:.2}x", result.gpu_speedup) } else { "FAIL".to_string() };
        
        println!("{:>10} {:>12.0} {:>12.0} {:>12.1} {:>12.1} {:>10}", 
            format_number(result.point_count as u64),
            result.cpu_time_us,
            if result.gpu_success { result.gpu_time_us } else { 0.0 },
            cpu_mpts,
            gpu_mpts,
            speedup_str
        );
    }
    
    // Create performance plot
    create_performance_plot(&results)?;
    
    // Print analysis
    println!("\n🔬 Performance Analysis");
    println!("=======================");
    
    if let Some(gpu) = &gpu_renderer {
        let stats = gpu.get_stats();
        println!("GPU Operations: {}", stats.gpu_operations);
        println!("CPU Fallbacks: {}", stats.cpu_operations);
        println!("GPU Points Processed: {}", format_number(stats.gpu_points_processed));
        println!("CPU Points Processed: {}", format_number(stats.cpu_points_processed));
        println!("Average GPU Time: {:.1}μs", stats.avg_gpu_time);
        println!("Average CPU Time: {:.1}μs", stats.avg_cpu_time);
    }
    
    // Find optimal breakpoint
    let successful_gpu_results: Vec<_> = results.iter()
        .filter(|r| r.gpu_success && r.gpu_speedup > 1.0)
        .collect();
        
    if let Some(best_result) = successful_gpu_results.iter()
        .max_by(|a, b| a.gpu_speedup.partial_cmp(&b.gpu_speedup).unwrap()) {
        println!("\n🏆 Best GPU Performance:");
        println!("  {} points: {:.2}x speedup ({:.1}M pts/sec)", 
            format_number(best_result.point_count as u64),
            best_result.gpu_speedup,
            best_result.gpu_throughput / 1_000_000.0);
    }
    
    // Efficiency analysis
    let gpu_efficiency_range: Vec<_> = successful_gpu_results.iter()
        .filter(|r| r.gpu_speedup > 1.1) // Only meaningful speedups
        .map(|r| r.point_count)
        .collect();
    
    if !gpu_efficiency_range.is_empty() {
        let min_efficient = *gpu_efficiency_range.iter().min().unwrap();
        let max_efficient = *gpu_efficiency_range.iter().max().unwrap();
        println!("\n⚡ GPU Efficiency Range:");
        println!("  {} - {} points show meaningful speedup (>1.1x)", 
            format_number(min_efficient as u64),
            format_number(max_efficient as u64));
    }
    
    println!("\n✅ Scaling analysis complete! Check 'gpu_scaling_plot.png'");
    Ok(())
}

fn create_performance_plot(results: &[BenchmarkResult]) -> Result<()> {
    use ruviz::Plot;
    
    // Extract data for plotting
    let point_counts: Vec<f64> = results.iter().map(|r| r.point_count as f64).collect();
    let cpu_throughput: Vec<f64> = results.iter()
        .map(|r| r.cpu_throughput / 1_000_000.0) // Convert to Mpts/sec
        .collect();
    let gpu_throughput: Vec<f64> = results.iter()
        .map(|r| if r.gpu_success { r.gpu_throughput / 1_000_000.0 } else { 0.0 })
        .collect();
    let gpu_speedup: Vec<f64> = results.iter()
        .map(|r| if r.gpu_success { r.gpu_speedup } else { 0.0 })
        .collect();
    
    // Create throughput comparison plot
    Plot::new()
        .line(&point_counts, &cpu_throughput)
        .line(&point_counts, &gpu_throughput)
        .title("GPU vs CPU Performance Scaling")
        .xlabel("Dataset Size (points)")
        .ylabel("Throughput (Million points/sec)")
        .legend(&["CPU", "GPU"])
        .width(1200.0)
        .dpi(150)
        .build()
        .save("gpu_throughput_scaling.png")?;
    
    // Create speedup plot
    let valid_speedups: Vec<_> = results.iter()
        .filter(|r| r.gpu_success)
        .collect();
        
    if !valid_speedups.is_empty() {
        let speedup_points: Vec<f64> = valid_speedups.iter()
            .map(|r| r.point_count as f64)
            .collect();
        let speedup_values: Vec<f64> = valid_speedups.iter()
            .map(|r| r.gpu_speedup)
            .collect();
        
        Plot::new()
            .scatter(&speedup_points, &speedup_values)
            .title("GPU Speedup vs Dataset Size")
            .xlabel("Dataset Size (points)")
            .ylabel("GPU Speedup (x)")
            .width(1200.0)
            .dpi(150)
            .build()
            .save("gpu_speedup_scaling.png")?;
    }
    
    println!("📊 Performance plots saved:");
    println!("  - gpu_throughput_scaling.png (throughput comparison)");
    println!("  - gpu_speedup_scaling.png (speedup analysis)");
    
    Ok(())
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