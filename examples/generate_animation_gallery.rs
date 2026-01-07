//! Generate animation gallery images for documentation
//!
//! Run with: cargo run --features animation --example generate_animation_gallery
//!
//! This generates GIF animations in docs/images/ for documentation.

use ruviz::prelude::*;
use ruviz::record;
use std::f64::consts::PI;

fn main() -> Result<()> {
    println!("Generating animation gallery images...\n");

    let output_dir = "docs/images";
    std::fs::create_dir_all(output_dir)?;

    // 1. Basic sine wave animation
    generate_sine_wave(output_dir)?;

    // 2. Growing scatter animation
    generate_growing_scatter(output_dir)?;

    // 3. Animated bar chart
    generate_animated_bars(output_dir)?;

    // 4. Spiral animation (polar coordinates)
    generate_spiral(output_dir)?;

    // 5. Signal composition example
    generate_signal_composition(output_dir)?;

    println!("\nAll animation gallery images generated successfully!");
    Ok(())
}

/// Generate animated sine wave
fn generate_sine_wave(output_dir: &str) -> Result<()> {
    println!("  Generating sine wave animation...");

    let path = format!("{}/animation_sine_wave.gif", output_dir);

    record!(
        &path,
        2.0 secs,
        |t| {
            let time = t.time;
            let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
            let y: Vec<f64> = x.iter().map(|&xi| (xi + time * PI).sin()).collect();

            Plot::new()
                .line(&x, &y)
                .title(format!("Sine Wave (t = {:.2}s)", time))
                .xlabel("x")
                .ylabel("sin(x + t)")
                .xlim(0.0, 10.0)
                .ylim(-1.5, 1.5)
        }
    )?;

    println!("    -> {}", path);
    Ok(())
}

/// Generate growing scatter plot
fn generate_growing_scatter(output_dir: &str) -> Result<()> {
    println!("  Generating growing scatter animation...");

    let path = format!("{}/animation_growing_scatter.gif", output_dir);

    record!(
        &path,
        3.0 secs,
        |t| {
            let time = t.time;
            let n = ((time + 0.1) * 50.0) as usize;
            let x: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).cos() * (i as f64 * 0.05)).collect();
            let y: Vec<f64> = (0..n).map(|i| (i as f64 * 0.1).sin() * (i as f64 * 0.05)).collect();

            Plot::new()
                .scatter(&x, &y)
                .title(format!("Growing Points: {} points", n))
                .xlabel("x")
                .ylabel("y")
        }
    )?;

    println!("    -> {}", path);
    Ok(())
}

/// Generate animated bar chart
fn generate_animated_bars(output_dir: &str) -> Result<()> {
    println!("  Generating animated bar chart...");

    let path = format!("{}/animation_bars.gif", output_dir);
    let categories = ["A", "B", "C", "D", "E"];

    record!(
        &path,
        2.0 secs,
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
                .title("Animated Bar Chart")
                .xlabel("Category")
                .ylabel("Value")
                .ylim(0.0, 110.0)
        }
    )?;

    println!("    -> {}", path);
    Ok(())
}

/// Generate spiral animation
fn generate_spiral(output_dir: &str) -> Result<()> {
    println!("  Generating spiral animation...");

    let path = format!("{}/animation_spiral.gif", output_dir);

    record!(
        &path,
        3.0 secs,
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
                .title("Spiral Growth")
                .xlabel("x")
                .ylabel("y")
        }
    )?;

    println!("    -> {}", path);
    Ok(())
}

/// Generate signal composition example
fn generate_signal_composition(output_dir: &str) -> Result<()> {
    println!("  Generating signal composition animation...");

    let path = format!("{}/animation_composition.gif", output_dir);

    // Use Signal combinators for complex animation
    let amplitude = signal::lerp(0.5, 2.0, 3.0);
    let frequency = signal::lerp(1.0, 3.0, 3.0);

    record!(
        &path,
        3.0 secs,
        |t| {
            let time = t.time;
            let amp = amplitude.at(time);
            let freq = frequency.at(time);

            let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
            let y: Vec<f64> = x.iter().map(|&xi| amp * (xi * freq).sin()).collect();

            Plot::new()
                .line(&x, &y)
                .title(format!("A={:.1}, f={:.1}", amp, freq))
                .xlabel("x")
                .ylabel("y")
                .xlim(0.0, 10.0)
                .ylim(-2.5, 2.5)
        }
    )?;

    println!("    -> {}", path);
    Ok(())
}
