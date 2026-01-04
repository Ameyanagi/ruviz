//! Heatmap example demonstrating various heatmap features
//!
//! Run with: cargo run --example heatmap_example

use ruviz::prelude::*;
use std::f64::consts::PI;

fn main() -> Result<()> {
    // Example 1: Basic heatmap with default colormap
    basic_heatmap()?;

    // Example 2: Correlation matrix style heatmap with annotations
    correlation_matrix()?;

    // Example 3: Scientific heatmap with diverging colormap
    scientific_heatmap()?;

    // Example 4: Large heatmap demonstration
    large_heatmap()?;

    println!("All heatmap examples saved successfully!");
    Ok(())
}

/// Basic heatmap with default viridis colormap
fn basic_heatmap() -> Result<()> {
    // Create a 5x5 grid of values
    let data = vec![
        vec![1.0, 2.0, 3.0, 4.0, 5.0],
        vec![2.0, 3.0, 4.0, 5.0, 6.0],
        vec![3.0, 4.0, 5.0, 6.0, 7.0],
        vec![4.0, 5.0, 6.0, 7.0, 8.0],
        vec![5.0, 6.0, 7.0, 8.0, 9.0],
    ];

    Plot::new()
        .heatmap(&data, Some(HeatmapConfig::default()))
        .title("Basic Heatmap")
        .save("examples/output/heatmap_basic.png")?;

    println!("Saved: examples/output/heatmap_basic.png");
    Ok(())
}

/// Correlation matrix with annotations and custom colormap
fn correlation_matrix() -> Result<()> {
    // Simulated correlation matrix
    let data = vec![
        vec![1.0, 0.8, 0.6, -0.2, -0.4],
        vec![0.8, 1.0, 0.5, 0.0, -0.3],
        vec![0.6, 0.5, 1.0, 0.3, 0.1],
        vec![-0.2, 0.0, 0.3, 1.0, 0.7],
        vec![-0.4, -0.3, 0.1, 0.7, 1.0],
    ];

    let config = HeatmapConfig::new()
        .colormap(ColorMap::coolwarm()) // Diverging colormap
        .vmin(-1.0)
        .vmax(1.0)
        .annotate(true)
        .colorbar(true)
        .colorbar_label("Correlation");

    Plot::new()
        .heatmap(&data, Some(config))
        .title("Correlation Matrix")
        .save("examples/output/heatmap_correlation.png")?;

    println!("Saved: examples/output/heatmap_correlation.png");
    Ok(())
}

/// Scientific heatmap with a function surface
fn scientific_heatmap() -> Result<()> {
    // Create a 2D sine wave pattern
    let rows = 20;
    let cols = 30;
    let mut data = vec![vec![0.0; cols]; rows];

    for i in 0..rows {
        for j in 0..cols {
            let x = j as f64 * 2.0 * PI / cols as f64;
            let y = i as f64 * 2.0 * PI / rows as f64;
            data[i][j] = (x.sin() * y.cos()).sin();
        }
    }

    let config = HeatmapConfig::new()
        .colormap(ColorMap::plasma())
        .colorbar(true)
        .colorbar_label("sin(sin(x)*cos(y))")
        .aspect(1.0);

    Plot::new()
        .heatmap(&data, Some(config))
        .title("2D Sine Wave Surface")
        .xlabel("X")
        .ylabel("Y")
        .save("examples/output/heatmap_scientific.png")?;

    println!("Saved: examples/output/heatmap_scientific.png");
    Ok(())
}

/// Large heatmap for performance demonstration
fn large_heatmap() -> Result<()> {
    // Create a 50x50 Gaussian-like pattern
    let size = 50;
    let mut data = vec![vec![0.0; size]; size];

    for i in 0..size {
        for j in 0..size {
            let x = (j as f64 - size as f64 / 2.0) / 10.0;
            let y = (i as f64 - size as f64 / 2.0) / 10.0;
            data[i][j] = (-x * x - y * y).exp();
        }
    }

    let config = HeatmapConfig::new()
        .colormap(ColorMap::inferno())
        .colorbar(true)
        .colorbar_label("Intensity")
        .vmin(0.0)
        .vmax(1.0);

    Plot::new()
        .heatmap(&data, Some(config))
        .title("2D Gaussian Distribution")
        .xlabel("X")
        .ylabel("Y")
        .save("examples/output/heatmap_large.png")?;

    println!("Saved: examples/output/heatmap_large.png");
    Ok(())
}
