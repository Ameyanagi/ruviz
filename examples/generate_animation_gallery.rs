//! Generate animation gallery images for documentation
//!
//! Run with: cargo run --features animation --example generate_animation_gallery
//!
//! This generates GIF animations in docs/images/ for documentation.

use ruviz::animation::RecordConfig;
use ruviz::prelude::*;
use ruviz::record;
use std::f64::consts::PI;

fn main() -> Result<()> {
    println!("Generating animation gallery images...\n");

    let output_dir = "docs/images";
    std::fs::create_dir_all(output_dir)?;

    // Use higher resolution for better text quality
    let config = RecordConfig::new()
        .dimensions(1024, 768)
        .framerate(30);

    // 1. Basic sine wave animation
    generate_sine_wave(output_dir, config.clone())?;

    // 2. Growing scatter animation
    generate_growing_scatter(output_dir, config.clone())?;

    // 3. Animated bar chart
    generate_animated_bars(output_dir, config.clone())?;

    // 4. Spiral animation (polar coordinates)
    generate_spiral(output_dir, config.clone())?;

    // 5. Signal composition example
    generate_signal_composition(output_dir, config)?;

    println!("\nAll animation gallery images generated successfully!");
    Ok(())
}

/// Generate animated sine wave
fn generate_sine_wave(output_dir: &str, config: RecordConfig) -> Result<()> {
    println!("  Generating sine wave animation...");

    let path = format!("{}/animation_sine_wave.gif", output_dir);

    record!(
        &path,
        60,
        config: config,
        |t| {
            let time = t.time;
            let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
            let y: Vec<f64> = x.iter().map(|&xi| (xi + time * PI).sin()).collect();

            Plot::new()
                .line(&x, &y)
                .title(format!("Traveling Sine Wave (t = {:.2}s)", time))
                .xlabel("Position (x)")
                .ylabel("Amplitude")
                .xlim(0.0, 10.0)
                .ylim(-1.5, 1.5)
        }
    )?;

    println!("    -> {}", path);
    Ok(())
}

/// Generate growing scatter plot
fn generate_growing_scatter(output_dir: &str, config: RecordConfig) -> Result<()> {
    println!("  Generating growing scatter animation...");

    let path = format!("{}/animation_growing_scatter.gif", output_dir);

    record!(
        &path,
        90,
        config: config,
        |t| {
            let time = t.time;
            let n = ((time + 0.1) * 50.0) as usize;
            let x: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).cos() * (i as f64 * 0.05)).collect();
            let y: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).sin() * (i as f64 * 0.05)).collect();

            Plot::new()
                .scatter(&x, &y)
                .title(format!("Expanding Spiral Pattern ({} points)", n))
                .xlabel("X Coordinate")
                .ylabel("Y Coordinate")
                .xlim(-8.0, 8.0)
                .ylim(-8.0, 8.0)
        }
    )?;

    println!("    -> {}", path);
    Ok(())
}

/// Generate animated bar chart
fn generate_animated_bars(output_dir: &str, config: RecordConfig) -> Result<()> {
    println!("  Generating animated bar chart...");

    let path = format!("{}/animation_bars.gif", output_dir);
    let categories = ["Mon", "Tue", "Wed", "Thu", "Fri"];

    record!(
        &path,
        60,
        config: config,
        |t| {
            let time = t.time;
            let values: Vec<f64> = categories
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let phase = i as f64 * 0.5;
                    ((time * PI + phase).sin() + 1.0) * 50.0
                })
                .collect();

            Plot::new()
                .bar(&categories, &values)
                .title("Weekly Sales Fluctuation")
                .xlabel("Day of Week")
                .ylabel("Sales ($)")
                .ylim(0.0, 110.0)
        }
    )?;

    println!("    -> {}", path);
    Ok(())
}

/// Generate spiral animation
fn generate_spiral(output_dir: &str, config: RecordConfig) -> Result<()> {
    println!("  Generating spiral animation...");

    let path = format!("{}/animation_spiral.gif", output_dir);

    record!(
        &path,
        90,
        config: config,
        |t| {
            let time = t.time;
            let n = 200;
            let max_angle = time * 4.0 * PI;
            let angles: Vec<f64> = (0..n).map(|i| i as f64 / n as f64 * max_angle).collect();
            let radii: Vec<f64> = angles.iter().map(|&a| a * 0.1).collect();

            let x: Vec<f64> = angles.iter().zip(&radii).map(|(&a, &r)| r * a.cos()).collect();
            let y: Vec<f64> = angles.iter().zip(&radii).map(|(&a, &r)| r * a.sin()).collect();

            Plot::new()
                .line(&x, &y)
                .title("Archimedean Spiral Growth")
                .xlabel("X Position")
                .ylabel("Y Position")
                .xlim(-4.0, 4.0)
                .ylim(-4.0, 4.0)
        }
    )?;

    println!("    -> {}", path);
    Ok(())
}

/// Generate signal composition example
fn generate_signal_composition(output_dir: &str, config: RecordConfig) -> Result<()> {
    println!("  Generating signal composition animation...");

    let path = format!("{}/animation_composition.gif", output_dir);

    // Use Signal combinators for complex animation
    let amplitude = signal::lerp(0.5, 2.0, 3.0);
    let frequency = signal::lerp(1.0, 3.0, 3.0);

    record!(
        &path,
        90,
        config: config,
        |t| {
            let time = t.time;
            let amp = amplitude.at(time);
            let freq = frequency.at(time);

            let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
            let y: Vec<f64> = x.iter().map(|&xi| amp * (xi * freq).sin()).collect();

            Plot::new()
                .line(&x, &y)
                .title(format!("Signal: Amp={:.2}, Freq={:.2}", amp, freq))
                .xlabel("Time (s)")
                .ylabel("Voltage (V)")
                .xlim(0.0, 10.0)
                .ylim(-2.5, 2.5)
        }
    )?;

    println!("    -> {}", path);
    Ok(())
}
