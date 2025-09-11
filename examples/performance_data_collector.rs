//! Collect performance data for GPU vs CPU analysis

use std::time::Instant;
use ruviz::core::*;
use ruviz::data::*;
use ruviz::render::gpu::{GpuRenderer, initialize_gpu_backend};
use ruviz::render::pooled::PooledRenderer;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔬 Performance Data Collection");
    println!("==============================\n");
    
    // Initialize CPU renderer
    let mut cpu_renderer = PooledRenderer::new();
    println!("✅ CPU Renderer initialized");
    
    // Initialize GPU renderer
    let mut gpu_renderer = match initialize_gpu_backend().await {
        Ok(_) => {
            match GpuRenderer::new().await {
                Ok(renderer) => {
                    println!("✅ GPU Renderer initialized");
                    println!("   GPU Threshold: {} points", renderer.gpu_threshold());
                    println!("   GPU Available: {}", renderer.is_gpu_available());
                    Some(renderer)
                },
                Err(e) => {
                    println!("❌ GPU Renderer failed: {}", e);
                    None
                }
            }
        },
        Err(e) => {
            println!("❌ GPU Backend failed: {}", e);
            None
        }
    };
    
    // Test various dataset sizes (focus on larger datasets)
    let test_sizes = vec![
        1_000, 2_500, 5_000, 7_500, 10_000, 
        25_000, 50_000, 100_000, 250_000, 500_000
    ];
    
    println!("\n=== Performance Results ===");
    println!("{:>10} {:>15} {:>15} {:>12} {:>12} {:>10}", 
        "Points", "CPU (μs)", "GPU (μs)", "CPU Mpts/s", "GPU Mpts/s", "Speedup");
    println!("{}", "=".repeat(82));
    
    let mut csv_data = String::from("Points,CPU_us,GPU_us,CPU_Mpts,GPU_Mpts,Speedup,GPU_Success\n");
    
    for &point_count in &test_sizes {
        // Generate test data (sine wave)
        let x_data: Vec<f64> = (0..point_count)
            .map(|i| i as f64 * 0.001)
            .collect();
        let y_data: Vec<f64> = x_data.iter()
            .map(|&x| (x * std::f64::consts::PI).sin())
            .collect();
        
        let x_range = (0.0, (point_count - 1) as f64 * 0.001);
        let y_range = (-1.0, 1.0);
        let viewport = (0.0, 0.0, 1920.0, 1080.0);
        
        // === CPU Benchmark ===
        let start = Instant::now();
        let _cpu_result = cpu_renderer.transform_coordinates_pooled(
            &x_data, &y_data, x_range.0, x_range.1, y_range.0, y_range.1,
            viewport.0, viewport.1, viewport.2, viewport.3
        )?;
        let cpu_duration = start.elapsed();
        let cpu_us = cpu_duration.as_micros() as f64;
        let cpu_mpts = (point_count as f64 / cpu_duration.as_secs_f64()) / 1_000_000.0;
        
        // === GPU Benchmark ===
        let (gpu_us, gpu_mpts, speedup, gpu_success) = if let Some(ref mut gpu) = gpu_renderer {
            let start = Instant::now();
            match gpu.transform_coordinates_optimal(&x_data, &y_data, x_range, y_range, viewport) {
                Ok(_gpu_result) => {
                    let gpu_duration = start.elapsed();
                    let gpu_us = gpu_duration.as_micros() as f64;
                    let gpu_mpts = (point_count as f64 / gpu_duration.as_secs_f64()) / 1_000_000.0;
                    let speedup = cpu_duration.as_secs_f64() / gpu_duration.as_secs_f64();
                    (gpu_us, gpu_mpts, speedup, true)
                },
                Err(e) => {
                    println!("GPU failed for {} points: {}", point_count, e);
                    (cpu_us, cpu_mpts, 1.0, false)
                }
            }
        } else {
            (cpu_us, cpu_mpts, 1.0, false)
        };
        
        // Display results
        let speedup_str = if gpu_success { 
            format!("{:.2}x", speedup) 
        } else { 
            "FAIL".to_string() 
        };
        
        println!("{:>10} {:>15.0} {:>15.0} {:>12.2} {:>12.2} {:>10}", 
            format_number(point_count as u64),
            cpu_us,
            if gpu_success { gpu_us } else { 0.0 },
            cpu_mpts,
            if gpu_success { gpu_mpts } else { 0.0 },
            speedup_str
        );
        
        // Store CSV data
        csv_data.push_str(&format!("{},{:.0},{:.0},{:.3},{:.3},{:.3},{}\n",
            point_count, cpu_us, gpu_us, cpu_mpts, gpu_mpts, speedup, gpu_success));
        
        // Early exit for very slow operations
        if cpu_duration.as_secs_f64() > 3.0 {
            println!("CPU time > 3s, stopping benchmark");
            break;
        }
    }
    
    // Save CSV data for plotting
    std::fs::write("performance_data.csv", csv_data)?;
    println!("\n📁 Performance data saved to 'performance_data.csv'");
    
    // Show GPU statistics if available
    if let Some(gpu) = &gpu_renderer {
        let stats = gpu.get_stats();
        println!("\n📊 GPU Statistics:");
        println!("  GPU Operations: {}", stats.gpu_operations);
        println!("  CPU Fallbacks: {}", stats.cpu_operations);
        println!("  GPU Points: {}", format_number(stats.gpu_points_processed));  
        println!("  CPU Points: {}", format_number(stats.cpu_points_processed));
        println!("  Avg GPU Time: {:.1}μs", stats.avg_gpu_time);
        println!("  Avg CPU Time: {:.1}μs", stats.avg_cpu_time);
        println!("  GPU Capabilities:");
        let caps = gpu.gpu_capabilities();
        println!("    Max Buffer Size: {} MB", caps.max_buffer_size / 1_048_576);
        println!("    Supports Compute: {}", caps.supports_compute);
    }
    
    println!("\n✅ Performance analysis complete!");
    
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