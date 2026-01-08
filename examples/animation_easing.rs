//! Easing functions animation example
//!
//! Demonstrates the various easing functions available for smooth animations.
//!
//! Run with: cargo run --example animation_easing --features animation

use ruviz::animation::{RecordConfig, easing};
use ruviz::prelude::*;
use ruviz::record;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Ensure output directory exists
    std::fs::create_dir_all("export_output/gif").ok();

    println!("Recording easing demo to export_output/gif/animation_easing.gif...");

    // Use max_resolution for matplotlib-style visual weight
    let config = RecordConfig::new().max_resolution(800, 600).framerate(30);

    record!(
        "export_output/gif/animation_easing.gif",
        90, // 3 seconds at 30 FPS
        config: config,
        |tick| {
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

            // Create scatter plot showing current position of each easing
            Plot::new()
                .scatter(&x_vals, &y_vals)
                .marker_size(15.0)
                .title(format!("Easing Functions (t = {:.0}%)", t * 100.0))
                .xlabel("Easing Type")
                .ylabel("Progress")
                .xlim(-0.5, 6.5)
                .ylim(-0.1, 1.1)
        }
    )?;

    println!("Animation saved to export_output/gif/animation_easing.gif");
    println!("  - Shows 7 different easing functions");
    println!("  - Watch how each dot reaches the top at different speeds");

    Ok(())
}
