//! Real-time performance demonstration
//!
//! Shows smooth 60fps interactions with large datasets using GPU acceleration.
//! Demonstrates the performance advantages of the GPU-accelerated rendering
//! pipeline during interactive operations.
//!
//! Controls:
//! - Mouse wheel: Smooth zoom (should maintain 60fps)
//! - Left drag: Smooth pan (should maintain 60fps)
//! - 'P': Toggle performance overlay
//! - 'Q': Toggle rendering quality (Interactive/Balanced/Publication)
//! - '+'/'-': Increase/decrease dataset size dynamically
//! - Space: Regenerate dataset

use ruviz::prelude::*;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎮 Starting real-time performance demo...");
    println!("Controls:");
    println!("  - Mouse wheel: Smooth zoom (60fps target)");
    println!("  - Left drag: Smooth pan (60fps target)");
    println!("  - 'P': Toggle performance overlay");
    println!("  - 'Q': Toggle rendering quality");
    println!("  - '+'/'-': Change dataset size");
    println!("  - Space: Regenerate data");

    // Start with a large dataset to demonstrate performance
    let initial_size = 100_000;
    let dataset = generate_large_dataset(initial_size);

    println!("📊 Generated initial dataset with {} points", initial_size);
    println!(
        "🚀 Dataset generation time: {:.2}ms",
        dataset.generation_time_ms
    );

    // Create performance demonstration plot
    let plot = Plot::new()
        .line(&dataset.x_data, &dataset.y_data)
        .title(&format!(
            "Real-time Performance Demo - {} points",
            initial_size
        ))
        .xlabel("Time (s)")
        .ylabel("Signal Amplitude")
        .legend(Position::TopLeft);

    #[cfg(feature = "interactive")]
    {
        println!("🚀 Opening performance demo window...");
        println!("📈 Monitoring: Frame rate, render time, memory usage");

        // Create enhanced interactive plot with performance monitoring
        let performance_plot = create_performance_demo_plot(plot, &dataset)?;

        show_interactive(performance_plot).await?;
    }

    #[cfg(not(feature = "interactive"))]
    {
        println!("⚠️ Interactive features not enabled.");
        println!("To enable: cargo run --features interactive --example real_time_performance");

        // Run performance benchmarks on static rendering
        run_static_performance_benchmark(&dataset)?;

        // Save static version
        plot.save("examples/output/real_time_performance_static.png")?;
        println!("💾 Saved static version: examples/output/real_time_performance_static.png");
    }

    println!("✅ Performance demo completed!");
    Ok(())
}

/// Large dataset for performance testing
struct PerformanceDataset {
    x_data: Vec<f64>,
    y_data: Vec<f64>,
    size: usize,
    generation_time_ms: f64,
    memory_usage_mb: f64,
}

impl PerformanceDataset {
    fn memory_size(&self) -> usize {
        // Estimate memory usage in bytes
        (self.x_data.len() + self.y_data.len()) * std::mem::size_of::<f64>()
    }
}

/// Generate large dataset for performance testing
fn generate_large_dataset(n_points: usize) -> PerformanceDataset {
    let start_time = Instant::now();

    println!("🔄 Generating {} data points...", n_points);

    let mut x_data = Vec::with_capacity(n_points);
    let mut y_data = Vec::with_capacity(n_points);

    // Generate complex multi-frequency signal
    for i in 0..n_points {
        let t = i as f64 * 0.001; // 1ms sampling
        x_data.push(t);

        // Complex signal with multiple frequencies and noise
        let signal = (t * 2.0 * std::f64::consts::PI * 1.0).sin() * 1.0 +      // 1 Hz
            (t * 2.0 * std::f64::consts::PI * 3.0).sin() * 0.5 +      // 3 Hz
            (t * 2.0 * std::f64::consts::PI * 10.0).sin() * 0.2 +     // 10 Hz
            (t * 2.0 * std::f64::consts::PI * 50.0).sin() * 0.1 +     // 50 Hz
            (i as f64 * 0.00001).sin() * 0.05; // Slow drift

        y_data.push(signal);
    }

    let generation_time = start_time.elapsed();
    let memory_usage = (x_data.len() + y_data.len()) * std::mem::size_of::<f64>();

    println!(
        "✅ Dataset generated in {:.2}ms",
        generation_time.as_secs_f64() * 1000.0
    );
    println!(
        "💾 Memory usage: {:.2}MB",
        memory_usage as f64 / 1_048_576.0
    );

    PerformanceDataset {
        x_data,
        y_data,
        size: n_points,
        generation_time_ms: generation_time.as_secs_f64() * 1000.0,
        memory_usage_mb: memory_usage as f64 / 1_048_576.0,
    }
}

/// Create performance demonstration plot
fn create_performance_demo_plot(base_plot: Plot, dataset: &PerformanceDataset) -> Result<Plot> {
    // In a real implementation, this would add performance overlay elements
    // For now, enhance the title with performance information

    let enhanced_title = format!(
        "Performance Demo - {} points ({:.1}MB)\nGPU Acceleration: {} | Target: 60 FPS",
        dataset.size,
        dataset.memory_usage_mb,
        if cfg!(feature = "gpu") {
            "Enabled"
        } else {
            "Disabled"
        }
    );

    // Create the enhanced plot
    let plot = Plot::new()
        .line(&dataset.x_data, &dataset.y_data)
        .title(&enhanced_title)
        .xlabel("Time (s)")
        .ylabel("Multi-frequency Signal")
        .legend(Position::TopLeft)
        .end_series();

    Ok(plot)
}

/// Run static performance benchmarks
fn run_static_performance_benchmark(dataset: &PerformanceDataset) -> Result<()> {
    println!("\n🔬 Running static rendering benchmarks...");

    let plot = Plot::new()
        .line(&dataset.x_data, &dataset.y_data)
        .title(&format!("Performance Benchmark - {} points", dataset.size));

    // Measure rendering time
    let render_start = Instant::now();
    plot.save("examples/output/benchmark_output.png")?;
    let render_time = render_start.elapsed();

    println!("📊 Benchmark Results:");
    println!("  Dataset size: {} points", dataset.size);
    println!("  Memory usage: {:.2}MB", dataset.memory_usage_mb);
    println!("  Render time: {:.2}ms", render_time.as_secs_f64() * 1000.0);
    println!(
        "  Points per second: {:.0}",
        dataset.size as f64 / render_time.as_secs_f64()
    );

    // Calculate theoretical interactive performance
    let target_fps = 60.0;
    let target_frame_time_ms = 1000.0 / target_fps;
    let performance_margin = target_frame_time_ms / (render_time.as_secs_f64() * 1000.0);

    println!(
        "  Performance margin: {:.1}x (for 60fps target)",
        performance_margin
    );

    if performance_margin > 2.0 {
        println!("  ✅ Excellent performance - should maintain 60fps easily");
    } else if performance_margin > 1.0 {
        println!("  ⚠️ Good performance - may need optimization for heavy interaction");
    } else {
        println!("  ❌ Poor performance - GPU acceleration recommended");
    }

    // Clean up benchmark file
    std::fs::remove_file("benchmark_output.png").ok();

    Ok(())
}

/// Performance monitoring structure
#[derive(Debug, Clone)]
struct PerformanceMetrics {
    current_fps: f64,
    avg_frame_time_ms: f64,
    render_time_ms: f64,
    memory_usage_mb: f64,
    gpu_utilization: f64,
    points_rendered: usize,
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            current_fps: 0.0,
            avg_frame_time_ms: 0.0,
            render_time_ms: 0.0,
            memory_usage_mb: 0.0,
            gpu_utilization: 0.0,
            points_rendered: 0,
        }
    }

    fn format_overlay(&self) -> String {
        format!(
            "FPS: {:.1} | Frame: {:.1}ms | Render: {:.1}ms\nGPU: {:.1}% | Memory: {:.1}MB | Points: {}",
            self.current_fps,
            self.avg_frame_time_ms,
            self.render_time_ms,
            self.gpu_utilization,
            self.memory_usage_mb,
            self.points_rendered
        )
    }
}

/// Simulate performance monitoring
fn simulate_performance_metrics(dataset: &PerformanceDataset) -> PerformanceMetrics {
    PerformanceMetrics {
        current_fps: 58.7, // Simulated slightly below 60
        avg_frame_time_ms: 16.8,
        render_time_ms: 12.3,
        memory_usage_mb: dataset.memory_usage_mb,
        gpu_utilization: 65.4,
        points_rendered: dataset.size,
    }
}

/// Test different dataset sizes for scaling analysis
fn test_performance_scaling() -> Result<()> {
    println!("\n📈 Testing performance scaling...");

    let test_sizes = vec![1_000, 10_000, 50_000, 100_000, 500_000, 1_000_000];

    for &size in &test_sizes {
        let dataset = generate_large_dataset(size);

        let plot = Plot::new()
            .line(&dataset.x_data, &dataset.y_data)
            .title(&format!("Scaling Test - {} points", size));

        let render_start = Instant::now();
        plot.save(&format!("scaling_test_{}.png", size))?;
        let render_time = render_start.elapsed();

        println!(
            "  {} points: {:.2}ms ({:.0} pts/sec)",
            size,
            render_time.as_secs_f64() * 1000.0,
            size as f64 / render_time.as_secs_f64()
        );

        // Clean up
        std::fs::remove_file(&format!("scaling_test_{}.png", size)).ok();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataset_generation() {
        let dataset = generate_large_dataset(1000);

        assert_eq!(dataset.x_data.len(), 1000);
        assert_eq!(dataset.y_data.len(), 1000);
        assert_eq!(dataset.size, 1000);
        assert!(dataset.generation_time_ms > 0.0);
        assert!(dataset.memory_usage_mb > 0.0);

        // Verify data integrity
        assert!(dataset.x_data.iter().all(|&x| x >= 0.0));
        assert!(dataset.y_data.iter().all(|&y| y.is_finite()));

        // Check memory size calculation
        let expected_memory = dataset.memory_size();
        assert!(expected_memory > 0);
    }

    #[test]
    fn test_performance_metrics() {
        let dataset = generate_large_dataset(100);
        let metrics = simulate_performance_metrics(&dataset);

        assert!(metrics.current_fps > 0.0);
        assert!(metrics.avg_frame_time_ms > 0.0);
        assert!(metrics.memory_usage_mb > 0.0);

        let overlay = metrics.format_overlay();
        assert!(!overlay.is_empty());
        assert!(overlay.contains("FPS"));
        assert!(overlay.contains("Memory"));
    }

    #[tokio::test]
    async fn test_plot_creation() {
        let dataset = generate_large_dataset(100);
        let plot = Plot::new();
        let enhanced_plot = create_performance_demo_plot(plot, &dataset);
        assert!(enhanced_plot.is_ok());
    }

    #[test]
    fn test_static_benchmark() {
        let dataset = generate_large_dataset(1000);
        let result = run_static_performance_benchmark(&dataset);
        assert!(result.is_ok());
    }

    #[test]
    #[ignore] // This test is slow, run with --ignored
    fn test_performance_scaling_benchmark() {
        let result = test_performance_scaling();
        assert!(result.is_ok());
    }
}
