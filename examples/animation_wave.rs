//! Wave animation example
//!
//! Demonstrates animating multiple data series with different phases.
//!
//! Run with: cargo run --example animation_wave --features animation

use ruviz::animation::{Quality, RecordConfig};
use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Generate x data points
    let x: Vec<f64> = (0..200).map(|i| i as f64 * 0.05).collect();

    // Ensure output directory exists
    std::fs::create_dir_all("export_output/gif").ok();

    println!("Recording wave animation to export_output/gif/animation_wave.gif...");

    // Record for 3 seconds at 30 FPS
    ruviz::animation::record_duration_with_config(
        "export_output/gif/animation_wave.gif",
        3.0, // 3 seconds
        RecordConfig::new()
            .dimensions(1000, 600)
            .framerate(30)
            .quality(Quality::Medium),
        |tick| {
            let t = tick.time;
            let omega = 2.0 * std::f64::consts::PI;

            // Wave 1: Moving sine wave
            let y1: Vec<f64> = x.iter().map(|&xi| (xi * 2.0 - t * omega).sin()).collect();

            // Wave 2: Standing wave (interference pattern)
            let y2: Vec<f64> = x
                .iter()
                .map(|&xi| {
                    let wave1 = (xi * 2.0 - t * omega).sin();
                    let wave2 = (xi * 2.0 + t * omega).sin();
                    0.5 * (wave1 + wave2) // Standing wave
                })
                .collect();

            // Wave 3: Damped oscillation
            let y3: Vec<f64> = x
                .iter()
                .map(|&xi| {
                    let envelope = (-xi * 0.3).exp();
                    envelope * (xi * 3.0 - t * omega * 1.5).sin()
                })
                .collect();

            // Create plot with multiple series
            #[allow(deprecated)]
            Plot::new()
                .line(&x, &y1)
                .label("Traveling Wave")
                .end_series()
                .line(&x, &y2)
                .label("Standing Wave")
                .end_series()
                .line(&x, &y3)
                .label("Damped Wave")
                .end_series()
                .title(format!("Wave Interference (t = {:.2}s)", t))
                .xlabel("Position")
                .ylabel("Amplitude")
                .xlim(0.0, 10.0)
                .ylim(-1.5, 1.5)
                .legend_position(LegendPosition::UpperRight)
        },
    )?;

    println!("Animation saved to export_output/gif/animation_wave.gif");
    println!("  - 90 frames at 30 FPS = 3 seconds");
    println!("  - Resolution: 1000x600");
    println!("  - Shows traveling, standing, and damped waves");

    Ok(())
}
