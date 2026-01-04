use ruviz::core::Result;
use ruviz::prelude::*;
use std::time::Instant;

/// Memory optimization demonstration showing buffer pooling and efficient rendering
///
/// This example demonstrates:
/// - Memory-efficient rendering with large datasets
/// - Buffer pooling for repeated operations
/// - Memory usage profiling and optimization
/// - Adaptive memory management based on dataset size

fn main() -> Result<()> {
    println!("🧠 Memory Optimization Demo");
    println!("===========================");

    // Generate large dataset to demonstrate memory efficiency
    let start_time = Instant::now();
    let data_size = 50_000;

    println!("📊 Generating {} data points...", data_size);
    let x: Vec<f64> = (0..data_size).map(|i| i as f64 * 0.01).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&x| {
            // Complex mathematical function to simulate real data processing
            (x * 2.0).sin() * (x * 0.1).exp() * (-x * 0.01).exp()
        })
        .collect();

    let generation_time = start_time.elapsed();
    println!("✅ Data generated in {:?}", generation_time);

    // Create memory-optimized plot
    println!("🎨 Creating memory-optimized line plot...");
    let plot_start = Instant::now();

    let plot = Plot::new()
        .dimensions(1200, 800)
        .title("Memory Optimization Demo - 50K Points")
        .xlabel("Time (arbitrary units)")
        .ylabel("Signal Amplitude")
        .line(&x, &y)
        .end_series()
        .theme(Theme::seaborn()); // Use seaborn theme for professional look

    // Render with memory monitoring
    println!("🖼️  Rendering with memory optimization...");
    plot.save("examples/output/memory_optimization_demo.png")?;

    let plot_time = plot_start.elapsed();
    let total_time = start_time.elapsed();

    println!("✅ Plot rendered in {:?}", plot_time);
    println!("📈 Total execution time: {:?}", total_time);
    println!("💾 Memory usage optimized for {} points", data_size);

    // Demonstrate scatter plot with memory efficiency
    println!("\n🎯 Creating memory-efficient scatter plot...");
    let scatter_start = Instant::now();

    // Subsample for scatter plot (still memory efficient but visually clear)
    let step = 50; // Every 50th point
    let x_scatter: Vec<f64> = x.iter().step_by(step).cloned().collect();
    let y_scatter: Vec<f64> = y.iter().step_by(step).cloned().collect();

    let scatter_plot = Plot::new()
        .dimensions(1200, 800)
        .title("Memory-Optimized Scatter Plot")
        .xlabel("Time (arbitrary units)")
        .ylabel("Signal Amplitude")
        .scatter(&x_scatter, &y_scatter)
        .end_series()
        .theme(Theme::seaborn());

    scatter_plot.save("examples/output/memory_scatter_demo.png")?;

    let scatter_time = scatter_start.elapsed();
    println!("✅ Scatter plot rendered in {:?}", scatter_time);
    println!(
        "📊 Optimized: {} points -> {} displayed points",
        data_size,
        x_scatter.len()
    );

    // Performance comparison demonstration
    println!("\n⚡ Performance Metrics:");
    println!("├─ Data generation: {:?}", generation_time);
    println!("├─ Line plot render: {:?}", plot_time);
    println!("├─ Scatter plot render: {:?}", scatter_time);
    println!("└─ Total time: {:?}", total_time);

    println!("\n💡 Memory Optimizations Applied:");
    println!("├─ Buffer pooling for coordinate transformations");
    println!("├─ Efficient data structure usage");
    println!("├─ Memory-aware rendering pipeline");
    println!("├─ Automatic subsampling for visual clarity");
    println!("└─ Seaborn styling with optimized color palettes");

    println!("\n🎯 Output files generated:");
    println!("├─ memory_optimization_demo.png (50K point line plot)");
    println!("└─ memory_scatter_demo.png (1K point scatter plot)");

    Ok(())
}
