use ruviz::core::Result;
use ruviz::prelude::*;
use std::time::Instant;

/// Memory optimization demonstration showing buffer pooling and efficient rendering
fn main() -> Result<()> {
    println!("Memory Optimization Demo");
    println!("========================");
    std::fs::create_dir_all("generated/examples").ok();

    // Generate large dataset to demonstrate memory efficiency
    let start_time = Instant::now();
    let data_size = 50_000;

    println!("Generating {} data points...", data_size);
    let x: Vec<f64> = (0..data_size).map(|i| i as f64 * 0.002).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&x| ((x * 1.7).sin() + 0.35 * (x * 9.0).sin()) * (-x * 0.012).exp())
        .collect();

    let generation_time = start_time.elapsed();
    println!("Data generated in {:?}", generation_time);

    // Create memory-optimized line plot
    println!("Creating memory-optimized line plot...");
    let plot_start = Instant::now();

    Plot::new()
        .title("Memory Optimization Demo - 50K Points")
        .xlabel("Time (arbitrary units)")
        .ylabel("Signal Amplitude")
        .size_px(1200, 800)
        .theme(Theme::seaborn())
        .line(&x, &y)
        .save("generated/examples/memory_optimization_demo.png")?;

    let plot_time = plot_start.elapsed();
    println!("Line plot rendered in {:?}", plot_time);

    // Demonstrate scatter plot with memory efficiency
    println!("\nCreating memory-efficient scatter plot...");
    let scatter_start = Instant::now();

    // Subsample for scatter plot
    let step = 10;
    let x_scatter: Vec<f64> = x.iter().step_by(step).copied().collect();
    let y_scatter: Vec<f64> = y
        .iter()
        .enumerate()
        .step_by(step)
        .map(|(index, &value)| value + 0.08 * ((index as f64) * 0.037).sin())
        .collect();

    Plot::new()
        .title("Memory-Optimized Scatter Plot")
        .xlabel("Time (arbitrary units)")
        .ylabel("Signal Amplitude")
        .size_px(1200, 800)
        .theme(Theme::seaborn())
        .scatter(&x_scatter, &y_scatter)
        .save("generated/examples/memory_scatter_demo.png")?;

    let scatter_time = scatter_start.elapsed();
    let total_time = start_time.elapsed();

    println!("Scatter plot rendered in {:?}", scatter_time);
    println!(
        "Optimized: {} points -> {} displayed",
        data_size,
        x_scatter.len()
    );

    println!("\nPerformance Metrics:");
    println!("  Data generation: {:?}", generation_time);
    println!("  Line plot render: {:?}", plot_time);
    println!("  Scatter plot render: {:?}", scatter_time);
    println!("  Total time: {:?}", total_time);

    Ok(())
}
