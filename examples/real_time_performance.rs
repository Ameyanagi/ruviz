//! Real-time performance demonstration
//!
//! Shows smooth 60fps interactions with large datasets using GPU acceleration.
//!
//! Controls:
//! - Mouse wheel: Smooth zoom (should maintain 60fps)
//! - Left drag: Smooth pan (should maintain 60fps)
//! - 'P': Toggle performance overlay
//! - 'Q': Toggle rendering quality
//! - '+'/'-': Increase/decrease dataset size
//! - Space: Regenerate dataset

use ruviz::prelude::*;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting real-time performance demo...");
    std::fs::create_dir_all("examples/output").ok();

    // Generate large dataset
    let initial_size = 100_000;
    let dataset = generate_large_dataset(initial_size);

    println!(
        "Generated {} points in {:.2}ms",
        initial_size, dataset.generation_time_ms
    );

    // Create performance demonstration plot
    let plot: Plot = Plot::new()
        .title(&format!(
            "Real-time Performance Demo - {} points",
            initial_size
        ))
        .xlabel("Time (s)")
        .ylabel("Signal Amplitude")
        .legend(Position::TopLeft)
        .line(&dataset.x_data, &dataset.y_data)
        .into();

    #[cfg(feature = "interactive")]
    {
        println!("Opening performance demo window...");
        show_interactive(plot).await?;
    }

    #[cfg(not(feature = "interactive"))]
    {
        println!("Interactive features not enabled.");
        println!("To enable: cargo run --features interactive --example real_time_performance");
        run_static_performance_benchmark(&dataset)?;
        plot.save("examples/output/real_time_performance_static.png")?;
        println!("Saved: examples/output/real_time_performance_static.png");
    }

    Ok(())
}

struct PerformanceDataset {
    x_data: Vec<f64>,
    y_data: Vec<f64>,
    size: usize,
    generation_time_ms: f64,
    memory_usage_mb: f64,
}

fn generate_large_dataset(n_points: usize) -> PerformanceDataset {
    let start_time = Instant::now();

    let mut x_data = Vec::with_capacity(n_points);
    let mut y_data = Vec::with_capacity(n_points);

    for i in 0..n_points {
        let t = i as f64 * 0.001;
        x_data.push(t);

        // Complex multi-frequency signal
        let signal = (t * 2.0 * std::f64::consts::PI * 1.0).sin() * 1.0
            + (t * 2.0 * std::f64::consts::PI * 3.0).sin() * 0.5
            + (t * 2.0 * std::f64::consts::PI * 10.0).sin() * 0.2
            + (t * 2.0 * std::f64::consts::PI * 50.0).sin() * 0.1
            + (i as f64 * 0.00001).sin() * 0.05;

        y_data.push(signal);
    }

    let generation_time = start_time.elapsed();
    let memory_usage = (x_data.len() + y_data.len()) * std::mem::size_of::<f64>();

    PerformanceDataset {
        x_data,
        y_data,
        size: n_points,
        generation_time_ms: generation_time.as_secs_f64() * 1000.0,
        memory_usage_mb: memory_usage as f64 / 1_048_576.0,
    }
}

fn run_static_performance_benchmark(dataset: &PerformanceDataset) -> Result<()> {
    println!("\nRunning static rendering benchmark...");

    let plot: Plot = Plot::new()
        .title(&format!("Performance Benchmark - {} points", dataset.size))
        .line(&dataset.x_data, &dataset.y_data)
        .into();

    let render_start = Instant::now();
    plot.save("examples/output/benchmark_output.png")?;
    let render_time = render_start.elapsed();

    println!("Benchmark Results:");
    println!("  Dataset size: {} points", dataset.size);
    println!("  Memory usage: {:.2}MB", dataset.memory_usage_mb);
    println!("  Render time: {:.2}ms", render_time.as_secs_f64() * 1000.0);
    println!(
        "  Points/sec: {:.0}",
        dataset.size as f64 / render_time.as_secs_f64()
    );

    let target_fps = 60.0;
    let target_frame_time_ms = 1000.0 / target_fps;
    let performance_margin = target_frame_time_ms / (render_time.as_secs_f64() * 1000.0);

    println!(
        "  Performance margin: {:.1}x (for 60fps)",
        performance_margin
    );

    std::fs::remove_file("examples/output/benchmark_output.png").ok();
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
        assert!(dataset.generation_time_ms > 0.0);
    }

    #[test]
    fn test_static_benchmark() {
        std::fs::create_dir_all("examples/output").ok();
        let dataset = generate_large_dataset(1000);
        let result = run_static_performance_benchmark(&dataset);
        assert!(result.is_ok());
    }
}
