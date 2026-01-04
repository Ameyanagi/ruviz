//! Million Points Performance Demo
//!
//! Demonstrates ruviz's ability to efficiently render massive datasets.
//! This example renders 1 million data points as a scatter plot.
//!
//! Run with: cargo run --example million_points --features parallel
//! Run with GPU: cargo run --example million_points --features "gpu parallel"

use std::time::Instant;

// Include the common module for output path
#[path = "../common.rs"]
mod common;

fn main() {
    println!("=== Million Points Performance Demo ===\n");

    // Generate 1 million random data points
    println!("Generating 1 million data points...");
    let start = Instant::now();

    let n = 1_000_000;
    let mut x = Vec::with_capacity(n);
    let mut y = Vec::with_capacity(n);

    // Use a simple pseudo-random generator for reproducibility
    let mut seed = 12345u64;
    for _ in 0..n {
        // Simple LCG random number generator
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let rx = (seed as f64 / u64::MAX as f64) * 100.0;

        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let ry = (seed as f64 / u64::MAX as f64) * 100.0;

        // Add some clustering pattern for visual interest
        let cluster_x = ((rx / 20.0).floor() * 20.0) + ((rx % 20.0) * 0.8);
        let cluster_y = ((ry / 20.0).floor() * 20.0) + ((ry % 20.0) * 0.8);

        x.push(cluster_x);
        y.push(cluster_y);
    }

    let gen_time = start.elapsed();
    println!("Data generation: {:?}", gen_time);

    // Create and render the plot
    println!("\nRendering scatter plot with {} points...", n);
    let render_start = Instant::now();

    use ruviz::prelude::*;

    let output_path = common::example_output_path("million_points.png");

    let result = Plot::new()
        .size(10.0, 8.0) // 10x8 inches
        .dpi(150) // Higher DPI for detail
        .scatter(&x, &y)
        .title(&format!(
            "1 Million Points ({:.2}s generation)",
            gen_time.as_secs_f64()
        ))
        .xlabel("X")
        .ylabel("Y")
        .save(&output_path);

    let render_time = render_start.elapsed();

    match result {
        Ok(_) => {
            println!("Render time: {:?}", render_time);
            println!(
                "Throughput: {:.2} points/second",
                n as f64 / render_time.as_secs_f64()
            );
            println!("\nSaved to: {:?}", output_path);
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }

    // Also benchmark line plot
    println!("\n--- Line Plot Benchmark ---");
    let line_x: Vec<f64> = (0..n).map(|i| i as f64 * 0.0001).collect();
    let line_y: Vec<f64> = line_x.iter().map(|&x| (x * 100.0).sin() * 50.0).collect();

    let line_start = Instant::now();

    let line_output = common::example_output_path("million_points_line.png");

    let result = Plot::new()
        .size(12.0, 6.0)
        .dpi(100)
        .line(&line_x, &line_y)
        .title("1 Million Point Line Plot")
        .xlabel("X")
        .ylabel("sin(100x)")
        .save(&line_output);

    let line_time = line_start.elapsed();

    match result {
        Ok(_) => {
            println!("Line plot render time: {:?}", line_time);
            println!(
                "Line throughput: {:.2} points/second",
                n as f64 / line_time.as_secs_f64()
            );
            println!("Saved to: {:?}", line_output);
        }
        Err(e) => {
            eprintln!("Line plot error: {:?}", e);
        }
    }

    println!("\n=== Summary ===");
    println!("Dataset size: {} points", n);
    println!("Scatter render: {:?}", render_time);
    println!("Line render: {:?}", line_time);
}
