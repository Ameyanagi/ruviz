//! Simplified animation API example
//!
//! Demonstrates the new streamlined animation API with:
//! - `record_simple()` for minimal boilerplate
//! - Tick interpolation helpers (`lerp_over`, `ease_over`)
//! - Duration syntax (`2.0.secs()`)
//! - Animation builder for multi-value animations
//!
//! Run with: cargo run --example animation_simple --features animation

use ruviz::animation::{
    Animation, DurationExt, RecordConfig, easing, record_simple, record_simple_with_config,
};
use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Ensure output directory exists
    std::fs::create_dir_all("export_output/gif").ok();

    // ==========================================================
    // Example 1: Simplest possible animation
    // ==========================================================
    println!("Recording Example 1: Bouncing ball with ease_over...");

    record_simple("export_output/gif/simple_bounce.gif", 60, |t| {
        // Use ease_over for automatic progress calculation
        let y = t.ease_over(easing::ease_out_bounce, 100.0, 0.0, 2.0);

        // PlotBuilder converts to Plot automatically via Into<Plot>
        Plot::new()
            .scatter(&[50.0], &[y])
            .marker_size(15.0)
            .title(format!("Bounce (t={:.2}s)", t.time))
            .xlim(0.0, 100.0)
            .ylim(-10.0, 110.0)
    })?;

    println!("  Saved: export_output/gif/simple_bounce.gif");

    // ==========================================================
    // Example 2: Using duration syntax
    // ==========================================================
    println!("\nRecording Example 2: Wave with duration syntax...");

    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();

    record_simple("export_output/gif/simple_wave.gif", 2.0.secs(), |t| {
        let phase = t.lerp_over(0.0, 2.0 * std::f64::consts::PI, 2.0);
        let y: Vec<f64> = x.iter().map(|&xi| (xi + phase).sin()).collect();

        Plot::new()
            .line(&x, &y)
            .title(format!("Wave (t={:.2}s)", t.time))
            .xlim(0.0, 10.0)
            .ylim(-1.5, 1.5)
    })?;

    println!("  Saved: export_output/gif/simple_wave.gif");

    // ==========================================================
    // Example 3: Animation Builder for multi-value animation
    // ==========================================================
    println!("\nRecording Example 3: Multi-value animation with builder...");

    Animation::build()
        .value("x", 0.0)
        .to(100.0)
        .duration_secs(2.0)
        .value("y", 100.0)
        .to(0.0)
        .ease(easing::ease_out_elastic)
        .value("size", 5.0)
        .to(20.0)
        .ease(easing::ease_in_out_quad)
        .config(RecordConfig::new().framerate(30))
        .record("export_output/gif/simple_multi.gif", |values, _tick| {
            Plot::new()
                .scatter(&[values["x"]], &[values["y"]])
                .marker_size(values["size"] as f32)
                .title(format!("x={:.1}, y={:.1}", values["x"], values["y"]))
                .xlim(-10.0, 110.0)
                .ylim(-20.0, 120.0)
        })?;

    println!("  Saved: export_output/gif/simple_multi.gif");

    // ==========================================================
    // Example 4: Comparing easing functions
    // ==========================================================
    println!("\nRecording Example 4: Easing comparison...");

    let config = RecordConfig::new().dimensions(1000, 400).framerate(30);

    record_simple_with_config(
        "export_output/gif/simple_easing.gif",
        3.0.secs(),
        config,
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
        },
    )?;

    println!("  Saved: export_output/gif/simple_easing.gif");

    println!("\nAll examples complete!");
    Ok(())
}
