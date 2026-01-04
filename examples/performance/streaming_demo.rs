//! Streaming Data Demo
//!
//! Demonstrates ruviz's streaming data capabilities for real-time visualization.
//! This example simulates continuous data streaming and periodic plot updates.
//!
//! Run with: cargo run --example streaming_demo

use std::time::Instant;

// Include the common module for output path
#[path = "../util/mod.rs"]
mod common;

fn main() {
    println!("=== Streaming Data Demo ===\n");

    use ruviz::data::StreamingXY;
    use ruviz::prelude::*;

    // Create a streaming buffer with capacity for 10,000 points
    let stream = StreamingXY::new(10_000);

    // Simulate streaming data ingestion
    println!("Simulating data stream...");
    let start = Instant::now();

    // Push initial batch
    let mut time: f64 = 0.0;
    for _ in 0..1000 {
        let value = (time * 0.1).sin() + (time * 0.05).cos() * 0.5;
        stream.push(time, value);
        time += 0.01;
    }

    println!("Initial batch: {} points", stream.len());

    // Render first snapshot
    let output1 = common::example_output_path("streaming_frame_1.png");

    Plot::new()
        .size(10.0, 4.0)
        .dpi(100)
        .line_streaming(&stream)
        .title("Streaming Data - Frame 1 (1000 points)")
        .xlabel("Time")
        .ylabel("Value")
        .save(&output1)
        .expect("Frame 1 save failed");

    println!("Saved frame 1: {:?}", output1);

    // Push more data
    for _ in 0..2000 {
        let value = (time * 0.1).sin() + (time * 0.05).cos() * 0.5 + (time * 0.02).sin() * 0.3;
        stream.push(time, value);
        time += 0.01;
    }

    println!("After more data: {} points", stream.len());

    // Render second snapshot
    let output2 = common::example_output_path("streaming_frame_2.png");

    Plot::new()
        .size(10.0, 4.0)
        .dpi(100)
        .line_streaming(&stream)
        .title("Streaming Data - Frame 2 (3000 points)")
        .xlabel("Time")
        .ylabel("Value")
        .save(&output2)
        .expect("Frame 2 save failed");

    println!("Saved frame 2: {:?}", output2);

    // Push bulk data
    let bulk_size = 5000;
    let bulk_data: Vec<(f64, f64)> = (0..bulk_size)
        .map(|i| {
            let t = time + i as f64 * 0.01;
            let v = (t * 0.1).sin() * (1.0 + 0.2 * (t * 0.03).sin());
            (t, v)
        })
        .collect();

    stream.push_many(bulk_data);
    time += bulk_size as f64 * 0.01;

    println!("After bulk insert: {} points", stream.len());

    // Render third snapshot
    let output3 = common::example_output_path("streaming_frame_3.png");

    Plot::new()
        .size(10.0, 4.0)
        .dpi(100)
        .line_streaming(&stream)
        .title("Streaming Data - Frame 3 (8000 points)")
        .xlabel("Time")
        .ylabel("Value")
        .save(&output3)
        .expect("Frame 3 save failed");

    println!("Saved frame 3: {:?}", output3);

    let total_time = start.elapsed();

    println!("\n=== Summary ===");
    println!("Total points streamed: {}", stream.len());
    println!("Total time: {:?}", total_time);
    println!(
        "Throughput: {:.2} points/second",
        stream.len() as f64 / total_time.as_secs_f64()
    );
}
