//! Scaling Benchmark
//!
//! Benchmarks ruviz rendering performance across various dataset sizes
//! to demonstrate linear scaling characteristics.
//!
//! Run with: cargo run --example scaling_benchmark --release --features parallel

use std::time::Instant;

// Include the common module for output path
#[path = "../util/mod.rs"]
mod common;

fn main() {
    println!("=== Ruviz Scaling Benchmark ===\n");
    println!("Testing rendering performance from 1K to 1M points\n");

    use ruviz::prelude::*;

    let sizes = [
        1_000, 5_000, 10_000, 50_000, 100_000, 250_000, 500_000, 1_000_000,
    ];

    let mut results = Vec::new();

    for &n in &sizes {
        // Generate data
        let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.001).collect();
        let y: Vec<f64> = x.iter().map(|&x| (x * 10.0).sin()).collect();

        // Time rendering (don't save to disk to isolate render time)
        let start = Instant::now();

        let image = Plot::new()
            .size(8.0, 6.0)
            .dpi(100)
            .line(&x, &y)
            .title(format!("{} points", format_number(n)))
            .render()
            .expect("Render failed");

        let elapsed = start.elapsed();
        let throughput = n as f64 / elapsed.as_secs_f64();

        results.push((n, elapsed, throughput));

        println!(
            "{:>10} points: {:>8.2}ms ({:.2}M pts/sec)",
            format_number(n),
            elapsed.as_secs_f64() * 1000.0,
            throughput / 1_000_000.0
        );
    }

    println!("\n=== Performance Summary ===\n");

    // Calculate average throughput
    let avg_throughput: f64 = results.iter().map(|(_, _, t)| t).sum::<f64>() / results.len() as f64;
    println!(
        "Average throughput: {:.2}M points/second",
        avg_throughput / 1_000_000.0
    );

    // Check linear scaling
    let first = results.first().unwrap();
    let last = results.last().unwrap();
    let size_ratio = last.0 as f64 / first.0 as f64;
    let time_ratio = last.1.as_secs_f64() / first.1.as_secs_f64();

    println!(
        "Size increase: {:.0}x, Time increase: {:.1}x (ideal: {:.0}x)",
        size_ratio, time_ratio, size_ratio
    );

    if time_ratio < size_ratio * 1.5 {
        println!("✓ Near-linear scaling achieved!");
    } else {
        println!("⚠ Scaling is super-linear (possible optimization opportunity)");
    }

    // Generate a comparison plot
    println!("\nGenerating comparison plot...");

    let output = common::example_output_path("scaling_benchmark.png");

    let sizes_f64: Vec<f64> = results.iter().map(|(n, _, _)| *n as f64).collect();
    let times_ms: Vec<f64> = results
        .iter()
        .map(|(_, t, _)| t.as_secs_f64() * 1000.0)
        .collect();

    Plot::new()
        .size(10.0, 6.0)
        .dpi(150)
        .scatter(&sizes_f64, &times_ms)
        .line(&sizes_f64, &times_ms)
        .title("Ruviz Scaling Performance")
        .xlabel("Dataset Size (points)")
        .ylabel("Render Time (ms)")
        .xscale(AxisScale::Log)
        .yscale(AxisScale::Log)
        .save(&output)
        .expect("Save failed");

    println!("Saved benchmark plot: {:?}", output);
}

fn format_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{}M", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}K", n / 1_000)
    } else {
        n.to_string()
    }
}
