//! Debug GPU implementation to identify performance bottlenecks

use ruviz::core::*;
use ruviz::data::*;
use ruviz::render::gpu::{GpuRenderer, initialize_gpu_backend};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("🔧 GPU Debug Test");
    println!("=================\n");

    // Initialize GPU
    match initialize_gpu_backend().await {
        Ok(_) => println!("✅ GPU Backend initialized"),
        Err(e) => {
            println!("❌ GPU Backend failed: {}", e);
            return Ok(());
        }
    }

    let mut gpu_renderer = match GpuRenderer::new().await {
        Ok(renderer) => {
            println!(
                "✅ GPU Renderer created, threshold: {}",
                renderer.gpu_threshold()
            );
            renderer
        }
        Err(e) => {
            println!("❌ GPU Renderer failed: {}", e);
            return Ok(());
        }
    };

    // Test small dataset first
    let point_count = 1000;
    println!("\n🧪 Testing {} points", point_count);

    let x_data: Vec<f64> = (0..point_count).map(|i| i as f64 * 0.001).collect();
    let y_data: Vec<f64> = x_data
        .iter()
        .map(|&x| (x * 2.0 * std::f64::consts::PI).sin())
        .collect();

    let x_range = (0.0, point_count as f64 * 0.001);
    let y_range = (-1.0, 1.0);
    let viewport = (0.0, 0.0, 1920.0, 1080.0);

    println!("📊 Transform Coordinates Test");
    let start = Instant::now();

    match gpu_renderer.transform_coordinates_optimal(&x_data, &y_data, x_range, y_range, viewport) {
        Ok((x_result, y_result)) => {
            let elapsed = start.elapsed();
            println!(
                "✅ Success: {} ms, {} points transformed",
                elapsed.as_millis(),
                x_result.len()
            );
            println!(
                "   First result: x={:.3}, y={:.3}",
                x_result[0], y_result[0]
            );
            println!(
                "   Last result: x={:.3}, y={:.3}",
                x_result[x_result.len() - 1],
                y_result[y_result.len() - 1]
            );
        }
        Err(e) => {
            println!("❌ Transform failed: {}", e);
        }
    }

    // Test medium dataset
    let point_count = 10_000;
    println!("\n🧪 Testing {} points", point_count);

    let x_data: Vec<f64> = (0..point_count).map(|i| i as f64 * 0.001).collect();
    let y_data: Vec<f64> = x_data
        .iter()
        .map(|&x| (x * 2.0 * std::f64::consts::PI).sin())
        .collect();

    println!("📊 Transform Coordinates Test (larger dataset)");
    let start = Instant::now();

    match gpu_renderer.transform_coordinates_optimal(&x_data, &y_data, x_range, y_range, viewport) {
        Ok((x_result, y_result)) => {
            let elapsed = start.elapsed();
            println!(
                "✅ Success: {} ms, {} points transformed",
                elapsed.as_millis(),
                x_result.len()
            );
        }
        Err(e) => {
            println!("❌ Transform failed: {}", e);
        }
    }

    // Check stats
    let stats = gpu_renderer.get_stats();
    println!("\n📈 GPU Renderer Stats");
    println!("GPU operations: {}", stats.gpu_operations);
    println!("CPU operations: {}", stats.cpu_operations);
    println!("GPU points processed: {}", stats.gpu_points_processed);
    println!("CPU points processed: {}", stats.cpu_points_processed);
    println!("Avg GPU time: {:.1}μs", stats.avg_gpu_time);
    println!("Avg CPU time: {:.1}μs", stats.avg_cpu_time);

    Ok(())
}
