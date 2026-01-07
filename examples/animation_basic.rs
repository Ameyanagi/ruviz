//! Basic animation example
//!
//! Demonstrates both the original and simplified animation APIs with a sine wave.
//!
//! Run with: cargo run --example animation_basic --features animation

use ruviz::animation::{DurationExt, Quality, RecordConfig, record_simple_with_config};
use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Ensure output directory exists
    std::fs::create_dir_all("export_output/gif").ok();

    // Generate x data points (shared by both examples)
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();

    // ==========================================================
    // ORIGINAL API (verbose but flexible)
    // ==========================================================
    println!("Recording with original API...");

    let config = RecordConfig::new()
        .dimensions(800, 600)
        .framerate(30)
        .quality(Quality::Medium);

    ruviz::animation::record_with_config(
        "export_output/gif/animation_basic_original.gif",
        0..60, // Explicit frame range
        config.clone(),
        |_frame, tick| {
            let phase = tick.time * 2.0 * std::f64::consts::PI;
            let y: Vec<f64> = x.iter().map(|&xi| (xi + phase).sin()).collect();

            Plot::new()
                .line(&x, &y)
                .title(format!("Original API (t={:.2}s)", tick.time))
                .xlabel("x")
                .ylabel("sin(x + phase)")
                .xlim(0.0, 10.0)
                .ylim(-1.5, 1.5)
        },
    )?;

    println!("  Saved: export_output/gif/animation_basic_original.gif");

    // ==========================================================
    // SIMPLIFIED API (recommended)
    // ==========================================================
    println!("\nRecording with simplified API...");

    record_simple_with_config(
        "export_output/gif/animation_basic_simple.gif",
        2.0.secs(), // Duration syntax instead of frame count
        config,
        |tick| {
            // Use lerp_over for phase calculation
            let phase = tick.lerp_over(0.0, 2.0 * std::f64::consts::PI, 2.0);
            let y: Vec<f64> = x.iter().map(|&xi| (xi + phase).sin()).collect();

            Plot::new()
                .line(&x, &y)
                .title(format!("Simplified API (t={:.2}s)", tick.time))
                .xlabel("x")
                .ylabel("sin(x + phase)")
                .xlim(0.0, 10.0)
                .ylim(-1.5, 1.5)
        },
    )?;

    println!("  Saved: export_output/gif/animation_basic_simple.gif");

    // ==========================================================
    // Summary
    // ==========================================================
    println!("\n=== API Comparison ===");
    println!("Original: record_with_config(path, 0..60, config, |frame, tick| ...)");
    println!("Simple:   record_simple_with_config(path, 2.0.secs(), config, |tick| ...)");
    println!("\nBoth produce identical 2-second animations at 30 FPS.");

    Ok(())
}
