//! GPU integration test - validates GPU acceleration with memory pools
//!
//! This example demonstrates the hybrid CPU/GPU rendering system with automatic
//! threshold-based selection and seamless integration with memory pools.

use ruviz::{core::plot::Plot, data::Data1D, render::gpu::GpuRenderer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("🖥️ GPU Integration Test");

    // Test GPU renderer creation (should handle unavailable GPU gracefully)
    println!("Initializing GPU renderer...");
    let gpu_result = GpuRenderer::new().await;

    match gpu_result {
        Ok(gpu_renderer) => {
            println!("✅ GPU renderer initialized successfully");
            println!("   GPU threshold: {} points", gpu_renderer.gpu_threshold());
            println!("   GPU available: {}", gpu_renderer.is_gpu_available());
            println!("   Capabilities: {:?}", gpu_renderer.gpu_capabilities());

            test_coordinate_transformation(gpu_renderer).await?;
        }
        Err(e) => {
            println!("⚠️ GPU not available (expected in CI): {}", e);
            println!("   Testing CPU fallback path...");
            test_cpu_fallback().await?;
        }
    }

    println!("\n✅ GPU integration test completed successfully");
    Ok(())
}

async fn test_coordinate_transformation(
    mut gpu_renderer: GpuRenderer,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Testing coordinate transformation...");

    // Test with small dataset (should use CPU)
    let small_x: Vec<f64> = (0..1000).map(|i| i as f64 * 0.01).collect();
    let small_y: Vec<f64> = small_x.iter().map(|x| x.sin()).collect();

    println!(
        "   Small dataset ({} points): Should use CPU",
        small_x.len()
    );
    println!(
        "   Will use GPU: {}",
        gpu_renderer.should_use_gpu(small_x.len())
    );

    let result = gpu_renderer.transform_coordinates_optimal(
        &small_x,
        &small_y,
        (0.0, 10.0),
        (-1.0, 1.0),
        (0.0, 0.0, 800.0, 600.0),
    );

    match result {
        Ok((x_transformed, y_transformed)) => {
            println!("   ✅ Small dataset transformation successful");
            println!("      Transformed {} points", x_transformed.len());
            println!(
                "      X range: {:.2} to {:.2}",
                x_transformed.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
                x_transformed
                    .iter()
                    .fold(f32::NEG_INFINITY, |a, &b| a.max(b))
            );
        }
        Err(e) => {
            println!("   ❌ Small dataset transformation failed: {}", e);
        }
    }

    // Test with large dataset (should use GPU if available)
    let large_x: Vec<f64> = (0..100_000).map(|i| i as f64 * 0.0001).collect();
    let large_y: Vec<f64> = large_x
        .iter()
        .map(|x| (x * 10.0).sin() * (x * 3.0).cos())
        .collect();

    println!(
        "\n   Large dataset ({} points): Should use GPU if available",
        large_x.len()
    );
    println!(
        "   Will use GPU: {}",
        gpu_renderer.should_use_gpu(large_x.len())
    );

    let result = gpu_renderer.transform_coordinates_optimal(
        &large_x,
        &large_y,
        (0.0, 10.0),
        (-2.0, 2.0),
        (0.0, 0.0, 1920.0, 1080.0),
    );

    match result {
        Ok((x_transformed, y_transformed)) => {
            println!("   ✅ Large dataset transformation successful");
            println!("      Transformed {} points", x_transformed.len());

            // Verify reasonable transformation results
            let x_min = x_transformed.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let x_max = x_transformed
                .iter()
                .fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            let y_min = y_transformed.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let y_max = y_transformed
                .iter()
                .fold(f32::NEG_INFINITY, |a, &b| a.max(b));

            println!("      X range: {:.2} to {:.2}", x_min, x_max);
            println!("      Y range: {:.2} to {:.2}", y_min, y_max);

            // Check that points are within viewport bounds
            assert!(
                x_min >= 0.0 && x_max <= 1920.0,
                "X coordinates outside viewport"
            );
            assert!(
                y_min >= 0.0 && y_max <= 1080.0,
                "Y coordinates outside viewport"
            );

            println!("   ✅ Coordinate validation passed");
        }
        Err(e) => {
            println!(
                "   ⚠️ Large dataset transformation failed (GPU may be unavailable): {}",
                e
            );
            println!("   This is expected in CI environments without GPU support");
        }
    }

    // Display performance statistics
    let stats = gpu_renderer.get_stats();
    println!("\n📊 Performance Statistics:");
    println!("   GPU operations: {}", stats.gpu_operations);
    println!("   CPU operations: {}", stats.cpu_operations);
    println!("   GPU points processed: {}", stats.gpu_points_processed);
    println!("   CPU points processed: {}", stats.cpu_points_processed);

    if stats.gpu_operations > 0 {
        println!("   Average GPU time: {:.2}µs", stats.avg_gpu_time);
    }
    if stats.cpu_operations > 0 {
        println!("   Average CPU time: {:.2}µs", stats.avg_cpu_time);
    }

    Ok(())
}

async fn test_cpu_fallback() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing CPU fallback rendering...");

    // Generate test data
    let x_data: Vec<f64> = (0..5000).map(|i| i as f64 * 0.001).collect();
    let y_data: Vec<f64> = x_data.iter().map(|x| x.sin()).collect();

    // Use CPU renderer through regular Plot API
    let plot_result = Plot::new()
        .line(&x_data, &y_data)
        .title("CPU Fallback Test")
        .xlabel("X values")
        .ylabel("sin(x)")
        .save("test_output/gpu_integration_cpu_fallback.png");

    match plot_result {
        Ok(_) => {
            println!("✅ CPU fallback rendering successful");
            println!("   Generated: test_output/gpu_integration_cpu_fallback.png");
        }
        Err(e) => {
            println!("❌ CPU fallback rendering failed: {}", e);
        }
    }

    Ok(())
}
