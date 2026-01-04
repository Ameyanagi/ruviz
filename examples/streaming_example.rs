//! Streaming Data Example
//!
//! This example demonstrates ruviz's streaming data support for real-time
//! visualization scenarios like sensor data, logs, or financial ticks.
//!
//! Run with: cargo run --example streaming_example

use ruviz::data::StreamingXY;
use ruviz::prelude::*;
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== Ruviz Streaming Data Demo ===\n");

    // Example 1: Basic streaming buffer
    basic_streaming();

    // Example 2: Ring buffer behavior
    ring_buffer_demo();

    // Example 3: Simulated real-time data
    simulated_realtime();

    // Example 4: Partial render tracking
    partial_render_demo();

    println!("\n=== All streaming examples completed! ===");
}

/// Basic streaming buffer usage
fn basic_streaming() {
    println!("--- Example 1: Basic Streaming Buffer ---");

    // Create a streaming buffer for X/Y data
    let stream = StreamingXY::new(1000);

    // Push individual points
    stream.push(0.0, 0.0);
    stream.push(1.0, 1.0);
    stream.push(2.0, 4.0);
    stream.push(3.0, 9.0);

    println!("Pushed 4 points individually");
    println!("Buffer length: {}", stream.len());

    // Push multiple points at once
    let points: Vec<(f64, f64)> = (4..10).map(|i| (i as f64, (i * i) as f64)).collect();
    stream.push_many(points);

    println!("Pushed 6 more points");
    println!("Total length: {}", stream.len());

    // Create a plot from streaming data
    Plot::new()
        .line_streaming(&stream)
        .color(Color::new(31, 119, 180))
        .title("Streaming Line Plot (x²)")
        .xlabel("X")
        .ylabel("Y")
        .save("examples/output/streaming_basic.png")
        .expect("Failed to save plot");

    println!("Saved: examples/output/streaming_basic.png\n");
}

/// Demonstrate ring buffer wrap-around behavior
fn ring_buffer_demo() {
    println!("--- Example 2: Ring Buffer Behavior ---");

    // Create a small buffer that will wrap around
    let stream = StreamingXY::new(5);

    // Push 10 points into a buffer that only holds 5
    for i in 0..10 {
        stream.push(i as f64, (i as f64).sin());
        println!(
            "After push({}): len={}, x_values={:?}",
            i,
            stream.len(),
            stream.read_x()
        );
    }

    // Only the last 5 points are retained
    println!("\nFinal buffer contents:");
    println!("  X: {:?}", stream.read_x());
    println!("  Y: {:?}", stream.read_y());

    Plot::new()
        .line_streaming(&stream)
        .color(Color::new(255, 127, 14))
        .title("Ring Buffer (last 5 points)")
        .save("examples/output/streaming_ringbuffer.png")
        .expect("Failed to save plot");

    println!("Saved: examples/output/streaming_ringbuffer.png\n");
}

/// Simulate real-time data updates
fn simulated_realtime() {
    println!("--- Example 3: Simulated Real-time Data ---");

    let stream = StreamingXY::new(100);
    let stream_clone = stream.clone();

    // Spawn a thread that produces data
    let producer = thread::spawn(move || {
        for i in 0..50 {
            let x = i as f64 * 0.1;
            let y = (x * 2.0).sin() + (x * 5.0).cos() * 0.5;
            stream_clone.push(x, y);
            thread::sleep(Duration::from_millis(10));
        }
    });

    // Wait for producer
    producer.join().unwrap();

    println!("Produced 50 points in background thread");
    println!("Buffer now has {} points", stream.len());

    Plot::new()
        .line_streaming(&stream)
        .color(Color::new(44, 160, 44))
        .title("Simulated Real-time Sensor Data")
        .xlabel("Time (s)")
        .ylabel("Signal")
        .save("examples/output/streaming_realtime.png")
        .expect("Failed to save plot");

    println!("Saved: examples/output/streaming_realtime.png\n");
}

/// Demonstrate partial render tracking
fn partial_render_demo() {
    println!("--- Example 4: Partial Render Tracking ---");

    let stream = StreamingXY::new(100);

    // Initial data
    stream.push_many(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 4.0)]);

    println!("Initial data: {} points", stream.len());
    println!("Appended since last render: {}", stream.appended_count());

    // First render
    Plot::new()
        .line_streaming(&stream)
        .title("Initial Render")
        .save("examples/output/streaming_partial_1.png")
        .expect("Failed to save plot");

    println!(
        "After first render, appended count: {}",
        stream.appended_count()
    );

    // Add more data
    stream.push_many(vec![(3.0, 9.0), (4.0, 16.0)]);
    println!("\nAdded 2 more points");
    println!("Appended since last render: {}", stream.appended_count());
    println!("Can use partial render: {}", stream.can_partial_render());

    // Read only the new data
    let new_x = stream.read_appended_x();
    let new_y = stream.read_appended_y();
    println!("New X values: {:?}", new_x);
    println!("New Y values: {:?}", new_y);

    // Second render (includes all data)
    Plot::new()
        .line_streaming(&stream)
        .title("Updated Render")
        .save("examples/output/streaming_partial_2.png")
        .expect("Failed to save plot");

    println!(
        "Saved: examples/output/streaming_partial_1.png and examples/output/streaming_partial_2.png\n"
    );
}
