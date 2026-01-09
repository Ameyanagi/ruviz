use ruviz::core::Result;
use ruviz::prelude::*;
use std::time::Instant;

/// Memory optimization demonstration showing buffer pooling and efficient rendering

fn main() -> Result<()> {
    println!("Memory Optimization Demo");
    println!("========================");
    std::fs::create_dir_all("examples/output").ok();

    // Generate large dataset to demonstrate memory efficiency
    let start_time = Instant::now();
    let data_size = 50_000;

    println!("Generating {} data points...", data_size);
    let x: Vec<f64> = (0..data_size).map(|i| i as f64 * 0.01).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&x| (x * 2.0).sin() * (x * 0.1).exp() * (-x * 0.01).exp())
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
        .save("examples/output/memory_optimization_demo.png")?;

    let plot_time = plot_start.elapsed();
    println!("Line plot rendered in {:?}", plot_time);

    // Demonstrate scatter plot with memory efficiency
    println!("\nCreating memory-efficient scatter plot...");
    let scatter_start = Instant::now();

    // Subsample for scatter plot
    let step = 50;
    let x_scatter: Vec<f64> = x.iter().step_by(step).cloned().collect();
    let y_scatter: Vec<f64> = y.iter().step_by(step).cloned().collect();

    Plot::new()
        .title("Memory-Optimized Scatter Plot")
        .xlabel("Time (arbitrary units)")
        .ylabel("Signal Amplitude")
        .size_px(1200, 800)
        .theme(Theme::seaborn())
        .scatter(&x_scatter, &y_scatter)
        .save("examples/output/memory_scatter_demo.png")?;

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
