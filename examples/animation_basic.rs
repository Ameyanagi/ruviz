//! Basic animation example
//!
//! Demonstrates the recommended animation API using the record! macro.
//!
//! Run with: cargo run --example animation_basic --features animation

use ruviz::animation::RecordConfig;
use ruviz::prelude::*;
use ruviz::record;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Ensure output directory exists
    std::fs::create_dir_all("export_output/gif").ok();

    // Generate x data points
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();

    // ==========================================================
    // RECOMMENDED: record! macro with max_resolution()
    // ==========================================================
    println!("Recording animation with record! macro...");

    // Use max_resolution() for matplotlib-style visual weight
    let config = RecordConfig::new().max_resolution(800, 600).framerate(30);

    record!(
        "export_output/gif/animation_basic.gif",
        60, // 60 frames = 2 seconds at 30 FPS
        config: config,
        |t| {
            let phase = t.time * 2.0 * std::f64::consts::PI;
            let y: Vec<f64> = x.iter().map(|&xi| (xi + phase).sin()).collect();

            Plot::new()
                .line(&x, &y)
                .title(format!("Sine Wave Animation (t={:.2}s)", t.time))
                .xlabel("x")
                .ylabel("sin(x + phase)")
                .xlim(0.0, 10.0)
                .ylim(-1.5, 1.5)
        }
    )?;

    println!("  Saved: export_output/gif/animation_basic.gif");

    // ==========================================================
    // Alternative: Duration-based recording with config
    // ==========================================================
    println!("\nRecording duration-based animation...");

    // Duration syntax with config for matplotlib-style visual weight
    let config2 = RecordConfig::new().max_resolution(800, 600).framerate(30);

    record!(
        "export_output/gif/animation_basic_duration.gif",
        2 secs,
        config: config2,
        |t| {
            let phase = t.time * 2.0 * std::f64::consts::PI;
            let y: Vec<f64> = x.iter().map(|&xi| (xi + phase).sin()).collect();

            Plot::new()
                .line(&x, &y)
                .title(format!("Duration-Based (t={:.2}s)", t.time))
                .xlabel("x")
                .ylabel("sin(x + phase)")
                .xlim(0.0, 10.0)
                .ylim(-1.5, 1.5)
        }
    )?;

    println!("  Saved: export_output/gif/animation_basic_duration.gif");

    // ==========================================================
    // Summary
    // ==========================================================
    println!("\n=== Recommended Animation API ===");
    println!("record!(path, frames, config: cfg, |t| plot)  - Frame-based");
    println!("record!(path, 2.0 secs, config: cfg, |t| plot) - Duration-based");
    println!("\nUse max_resolution() for consistent matplotlib-style rendering.");

    Ok(())
}
