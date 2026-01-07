//! Documentation example: Polar Plot
//!
//! Generates docs/images/polar_plot.png for rustdoc
//!
//! This example demonstrates the high-level API for creating polar plots.

use ruviz::prelude::*;
use std::f64::consts::PI;

fn main() -> Result<()> {
    // Generate rose curve data
    let n_points = 200;
    let theta: Vec<f64> = (0..n_points)
        .map(|i| i as f64 * 2.0 * PI / n_points as f64)
        .collect();
    let r: Vec<f64> = theta.iter().map(|&t| (3.0 * t).cos().abs()).collect();

    // High-level API - simple polar plot
    Plot::new()
        .title("Polar Plot (Rose Curve)")
        .polar_line(&r, &theta)
        .fill(true)
        .fill_alpha(0.3)
        .save("docs/images/polar_plot.png")?;

    println!("Generated docs/images/polar_plot.png (high-level API)");

    // Cardioid curve
    let theta: Vec<f64> = (0..n_points)
        .map(|i| i as f64 * 2.0 * PI / n_points as f64)
        .collect();
    let r: Vec<f64> = theta.iter().map(|&t| 1.0 + t.cos()).collect();

    Plot::new()
        .title("Cardioid Curve")
        .polar_line(&r, &theta)
        .fill(true)
        .fill_alpha(0.4)
        .color(Color::from_hex("#e74c3c").unwrap())
        .save("docs/images/polar_cardioid.png")?;

    println!("Generated docs/images/polar_cardioid.png");

    Ok(())
}
