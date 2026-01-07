//! Easing functions animation example
//!
//! Demonstrates the various easing functions available for smooth animations.
//!
//! Run with: cargo run --example animation_easing --features animation

use ruviz::animation::{Quality, RecordConfig, easing};
use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Ensure output directory exists
    std::fs::create_dir_all("export_output/gif").ok();

    println!("Recording easing demo to export_output/gif/animation_easing.gif...");

    // Animation duration in frames (3 seconds at 30 FPS)
    let total_frames = 90;

    let config = RecordConfig::new()
        .dimensions(800, 600)
        .framerate(30)
        .quality(Quality::Medium);

    ruviz::animation::record_with_config(
        "export_output/gif/animation_easing.gif",
        0..total_frames,
        config,
        |_frame, tick| {
            // Normalized progress [0, 1]
            let t = tick.time / 3.0;

            // X coordinates for points
            let x_vals: Vec<f64> = (0..7).map(|i| i as f64).collect();

            // Apply different easings to get Y positions
            let y_vals: Vec<f64> = vec![
                easing::linear(t),
                easing::ease_in_quad(t),
                easing::ease_out_quad(t),
                easing::ease_in_out_quad(t),
                easing::ease_in_cubic(t),
                easing::ease_out_cubic(t),
                easing::ease_in_out_cubic(t),
            ];

            // Scale Y from 0 to 1 range
            let y_scaled: Vec<f64> = y_vals.iter().map(|&y| y).collect();

            // Create scatter plot showing current position of each easing
            #[allow(deprecated)]
            Plot::new()
                .scatter(&x_vals, &y_scaled)
                .marker_size(15.0)
                .end_series()
                .title(format!("Easing Functions (t = {:.0}%)", t * 100.0))
                .xlabel("Easing Type")
                .ylabel("Progress")
                .xlim(-0.5, 6.5)
                .ylim(-0.1, 1.1)
        },
    )?;

    println!("Animation saved to export_output/gif/animation_easing.gif");
    println!("  - Shows 7 different easing functions");
    println!("  - Watch how each dot reaches the top at different speeds");

    Ok(())
}
