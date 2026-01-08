//! Simplified animation API example
//!
//! Demonstrates the record! macro with various features:
//! - Frame count and duration syntax
//! - Tick interpolation helpers (`lerp_over`, `ease_over`)
//! - max_resolution() for matplotlib-style rendering
//!
//! Run with: cargo run --example animation_simple --features animation

use ruviz::animation::{RecordConfig, easing};
use ruviz::prelude::*;
use ruviz::record;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Ensure output directory exists
    std::fs::create_dir_all("export_output/gif").ok();

    // ==========================================================
    // Example 1: Simple bounce animation
    // ==========================================================
    println!("Recording Example 1: Bouncing ball with ease_over...");

    let config = RecordConfig::new().max_resolution(800, 600).framerate(30);

    record!(
        "export_output/gif/simple_bounce.gif",
        60, // 2 seconds at 30 FPS
        config: config.clone(),
        |t| {
            // Use ease_over for automatic progress calculation
            let y = t.ease_over(easing::ease_out_bounce, 100.0, 0.0, 2.0);

            Plot::new()
                .scatter(&[50.0], &[y])
                .marker_size(15.0)
                .title(format!("Bounce (t={:.2}s)", t.time))
                .xlim(0.0, 100.0)
                .ylim(-10.0, 110.0)
        }
    )?;

    println!("  Saved: export_output/gif/simple_bounce.gif");

    // ==========================================================
    // Example 2: Wave with duration syntax
    // ==========================================================
    println!("\nRecording Example 2: Wave with duration syntax...");

    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();

    record!(
        "export_output/gif/simple_wave.gif",
        60, // 2 seconds at 30 FPS
        config: config.clone(),
        |t| {
            let phase = t.lerp_over(0.0, 2.0 * std::f64::consts::PI, 2.0);
            let y: Vec<f64> = x.iter().map(|&xi| (xi + phase).sin()).collect();

            Plot::new()
                .line(&x, &y)
                .title(format!("Wave (t={:.2}s)", t.time))
                .xlim(0.0, 10.0)
                .ylim(-1.5, 1.5)
        }
    )?;

    println!("  Saved: export_output/gif/simple_wave.gif");

    // ==========================================================
    // Example 3: Comparing easing functions
    // ==========================================================
    println!("\nRecording Example 3: Easing comparison...");

    let wide_config = RecordConfig::new().max_resolution(1000, 400).framerate(30);

    record!(
        "export_output/gif/simple_easing.gif",
        90, // 3 seconds at 30 FPS
        config: wide_config,
        |t| {
            let duration = 3.0;

            // Different easing functions applied to the same range
            let linear = t.lerp_over(0.0, 100.0, duration);
            let ease_in = t.ease_over(easing::ease_in_quad, 0.0, 100.0, duration);
            let ease_out = t.ease_over(easing::ease_out_quad, 0.0, 100.0, duration);
            let elastic = t.ease_over(easing::ease_out_elastic, 0.0, 100.0, duration);
            let bounce = t.ease_over(easing::ease_out_bounce, 0.0, 100.0, duration);

            // Multiple scatter series with labels
            Plot::new()
                .scatter(&[linear], &[5.0])
                .marker_size(12.0)
                .label("Linear")
                .scatter(&[ease_in], &[4.0])
                .marker_size(12.0)
                .label("Ease In")
                .scatter(&[ease_out], &[3.0])
                .marker_size(12.0)
                .label("Ease Out")
                .scatter(&[elastic], &[2.0])
                .marker_size(12.0)
                .label("Elastic")
                .scatter(&[bounce], &[1.0])
                .marker_size(12.0)
                .label("Bounce")
                .title(format!("Easing Functions (t={:.2}s)", t.time))
                .xlabel("Position")
                .xlim(-10.0, 120.0)
                .ylim(0.0, 6.0)
                .legend_position(LegendPosition::UpperRight)
        }
    )?;

    println!("  Saved: export_output/gif/simple_easing.gif");

    println!("\nAll examples complete!");
    Ok(())
}
