//! Reactive animation example
//!
//! Demonstrates two approaches to multi-value animations:
//! 1. AnimatedObservable pattern (original, more control)
//! 2. Animation::build() pattern (simplified, declarative)
//!
//! Run with: cargo run --example animation_reactive --features animation

use ruviz::animation::{
    // Original API
    AnimatedObservable,
    // Simplified API
    Animation,
    AnimationGroup,
    Quality,
    RecordConfig,
    easing,
    record_animated_with_config,
};
use ruviz::prelude::*;
use ruviz::render::Color;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("export_output/gif").ok();

    // ==========================================================
    // APPROACH 1: AnimatedObservable (original, fine-grained control)
    // ==========================================================
    println!("Recording with AnimatedObservable pattern...");

    // Create animated observables
    let x_pos = AnimatedObservable::new(0.0_f64);
    let y_pos = AnimatedObservable::new(0.0_f64);
    let radius = AnimatedObservable::new(0.5_f64);

    // Clone for closure
    let x_ref = x_pos.clone();
    let y_ref = y_pos.clone();
    let r_ref = radius.clone();

    // Create animation group
    let mut group = AnimationGroup::new();
    group.add(&x_pos);
    group.add(&y_pos);
    group.add(&radius);

    // Start animations with different easings
    x_pos.animate_to_with_easing(8.0, 2000, easing::ease_out_elastic);
    y_pos.animate_to_with_easing(6.0, 1500, easing::ease_in_out_cubic);
    radius.animate_to_with_easing(2.0, 1000, easing::ease_out_bounce);

    let config = RecordConfig::new()
        .dimensions(800, 600)
        .framerate(30)
        .quality(Quality::Medium);

    record_animated_with_config(
        "export_output/gif/reactive_observable.gif",
        &group,
        120,
        config.clone(),
        |tick| {
            let (x, y, r) = (x_ref.get(), y_ref.get(), r_ref.get());
            create_circle_plot(x, y, r, tick.time, "Observable Pattern")
        },
    )?;

    println!("  Saved: export_output/gif/reactive_observable.gif");

    // ==========================================================
    // APPROACH 2: Animation::build() (simplified, declarative)
    // ==========================================================
    println!("\nRecording with Animation::build() pattern...");

    Animation::build()
        .value("x", 0.0)
        .to(8.0)
        .ease(easing::ease_out_elastic)
        .duration_secs(2.0)
        .value("y", 0.0)
        .to(6.0)
        .ease(easing::ease_in_out_cubic)
        .duration_secs(1.5)
        .value("r", 0.5)
        .to(2.0)
        .ease(easing::ease_out_bounce)
        .duration_secs(1.0)
        .config(config)
        .record("export_output/gif/reactive_builder.gif", |values, tick| {
            create_circle_plot(
                values["x"],
                values["y"],
                values["r"],
                tick.time,
                "Builder Pattern",
            )
        })?;

    println!("  Saved: export_output/gif/reactive_builder.gif");

    // ==========================================================
    // Summary
    // ==========================================================
    println!("\n=== Pattern Comparison ===");
    println!("Observable: Fine-grained control, can change animations mid-flight");
    println!("Builder:    Declarative, less boilerplate, auto-manages duration");
    println!("\nBoth produce equivalent animations with elastic/cubic/bounce easing.");

    Ok(())
}

/// Helper to create the circle plot (shared by both approaches)
fn create_circle_plot(x: f64, y: f64, r: f64, time: f64, label: &str) -> Plot {
    // Generate circle points
    let n = 50;
    let circle_x: Vec<f64> = (0..=n)
        .map(|i| x + r * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos())
        .collect();
    let circle_y: Vec<f64> = (0..=n)
        .map(|i| y + r * (2.0 * std::f64::consts::PI * i as f64 / n as f64).sin())
        .collect();

    Plot::new()
        // Circle outline
        .line(&circle_x, &circle_y)
        .color(Color::new(0x22, 0x77, 0xDD))
        .line_width(2.0)
        .label(format!("Circle (r={:.2})", r))
        // Center point
        .scatter(&[x], &[y])
        .marker_size(10.0)
        .color(Color::new(0xDD, 0x44, 0x44))
        .label("Center")
        .title(format!("{} (t={:.2}s)", label, time))
        .xlabel("X")
        .ylabel("Y")
        .xlim(-2.0, 12.0)
        .ylim(-2.0, 10.0)
        .legend_position(LegendPosition::UpperRight)
}
