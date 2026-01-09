use ruviz::core::Result;
use ruviz::prelude::*;
use std::thread;
use std::time::Instant;

/// Parallel rendering demonstration showing multi-threaded performance

fn main() -> Result<()> {
    println!("Parallel Rendering Demo");
    println!("=======================");
    std::fs::create_dir_all("examples/output").ok();

    let cpu_count = thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(1);
    println!("Available CPU cores: {}", cpu_count);

    let test_sizes = vec![10_000, 50_000, 100_000];

    for &size in &test_sizes {
        println!("\nTesting with {} data points", size);

        let start_time = Instant::now();
        let x: Vec<f64> = (0..size).map(|i| i as f64 * 0.001).collect();
        let y: Vec<f64> = x
            .iter()
            .map(|&t| {
                let sine = (t * 10.0).sin();
                let cosine = (t * 7.0).cos();
                let decay = (-t * 0.1).exp();
                sine * cosine * decay
            })
            .collect();

        let generation_time = start_time.elapsed();
        println!("  Data generation: {:?}", generation_time);

        let plot_start = Instant::now();

        Plot::new()
            .title(format!("Parallel Rendering Demo - {} Points", size))
            .xlabel("Time (arbitrary units)")
            .ylabel("Complex Signal")
            .size_px(1400, 900)
            .theme(Theme::seaborn())
            .line(&x, &y)
            .save(format!(
                "examples/output/parallel_demo_{}k.png",
                size / 1000
            ))?;

        let plot_time = plot_start.elapsed();
        println!("  Parallel rendering: {:?}", plot_time);

        let points_per_second = size as f64 / plot_time.as_secs_f64();
        println!("  Performance: {:.0} points/second", points_per_second);
    }

    // Multi-series test
    println!("\nMulti-series parallel rendering test...");
    let multi_start = Instant::now();

    let size = 25_000;
    let x: Vec<f64> = (0..size).map(|i| i as f64 * 0.002).collect();
    let y1: Vec<f64> = x
        .iter()
        .map(|&t| (t * 5.0).sin() * (-t * 0.05).exp())
        .collect();
    let y2: Vec<f64> = x
        .iter()
        .map(|&t| (t * 3.0).cos() * (-t * 0.03).exp())
        .collect();
    let y3: Vec<f64> = x
        .iter()
        .map(|&t| (t * 7.0).sin() * (t * 2.0).cos() * (-t * 0.02).exp())
        .collect();

    Plot::new()
        .title("Multi-Series Parallel Rendering")
        .xlabel("Time")
        .ylabel("Signal Amplitude")
        .size_px(1400, 900)
        .theme(Theme::seaborn())
        .line(&x, &y1)
        .label("Series 1")
        .line(&x, &y2)
        .label("Series 2")
        .line(&x, &y3)
        .label("Series 3")
        .save("examples/output/parallel_multi_series.png")?;

    let multi_time = multi_start.elapsed();
    println!("Multi-series plot completed in {:?}", multi_time);

    // Parallel scatter plot
    println!("\nParallel scatter plot rendering...");
    let scatter_start = Instant::now();

    let scatter_size = 20_000;
    let x_scatter: Vec<f64> = (0..scatter_size)
        .map(|i| {
            let t = i as f64 * 0.01;
            t + (t * 13.0).sin() * 0.1
        })
        .collect();

    let y_scatter: Vec<f64> = (0..scatter_size)
        .map(|i| {
            let t = i as f64 * 0.01;
            (t * 2.0).sin() + (t * 17.0).cos() * 0.2
        })
        .collect();

    Plot::new()
        .title("Parallel Scatter Plot Processing")
        .xlabel("X Coordinate")
        .ylabel("Y Coordinate")
        .size_px(1400, 900)
        .theme(Theme::seaborn())
        .scatter(&x_scatter, &y_scatter)
        .save("examples/output/parallel_scatter.png")?;

    let scatter_time = scatter_start.elapsed();
    println!("Scatter plot completed in {:?}", scatter_time);

    println!("\nPerformance Summary:");
    println!("  CPU cores utilized: {} threads", cpu_count);
    println!("  Largest dataset: 100K points");
    println!("  Multi-series: 3 series x 25K points");
    println!("  Scatter plot: 20K points");

    Ok(())
}
