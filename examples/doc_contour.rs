//! Documentation example: Contour Plot
//!
//! Generates docs/images/contour_plot.png for rustdoc
//!
//! This example demonstrates the high-level API for creating contour plots,
//! including smoothing via interpolation.

use ruviz::plots::ContourInterpolation;
use ruviz::prelude::*;

fn main() -> Result<()> {
    // Generate 2D data (Gaussian surface) - use smaller grid to show smoothing effect
    let size = 20;
    let x: Vec<f64> = (0..size).map(|i| (i as f64 - 10.0) / 2.0).collect();
    let y: Vec<f64> = (0..size).map(|i| (i as f64 - 10.0) / 2.0).collect();

    // Z as flat array (row-major)
    let z: Vec<f64> = (0..size)
        .flat_map(|j| {
            (0..size).map(move |i| {
                let xi = (i as f64 - 10.0) / 2.0;
                let yj = (j as f64 - 10.0) / 2.0;
                (-xi * xi - yj * yj).exp()
            })
        })
        .collect();

    // High-level API - smooth contour plot with colorbar
    Plot::new()
        .title("Contour Plot")
        .xlabel("X")
        .ylabel("Y")
        .contour(&x, &y, &z)
        .levels(10)
        .filled(true)
        .smooth(ContourInterpolation::Cubic, 4) // 4x upsampling with cubic interpolation
        .colorbar(true)
        .colorbar_label("Density")
        .colormap_name("viridis")
        .save("docs/images/contour_plot.png")?;

    println!("Generated docs/images/contour_plot.png (high-level API)");

    // Contour with explicit level values and linear interpolation
    let levels: Vec<f64> = (0..10).map(|i| i as f64 / 10.0 + 0.05).collect();

    Plot::new()
        .title("Contour with Custom Levels")
        .xlabel("X")
        .ylabel("Y")
        .contour(&x, &y, &z)
        .level_values(levels)
        .filled(true)
        .smooth(ContourInterpolation::Linear, 4)
        .colormap_name("plasma")
        .save("docs/images/contour_custom_levels.png")?;

    println!("Generated docs/images/contour_custom_levels.png");

    Ok(())
}
