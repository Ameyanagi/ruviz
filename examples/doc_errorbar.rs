//! Documentation example: Error Bars
//!
//! Generates docs/images/errorbar_plot.png for rustdoc
//!
//! This example demonstrates both error bar API patterns:
//! 1. **Modifier Pattern**: Attach error bars to existing Line/Scatter series
//! 2. **Standalone Pattern**: Create dedicated error bar series
//!
//! The modifier pattern (`.with_yerr()`, `.with_xerr()`) is recommended for most use cases
//! as it matches the mental model from matplotlib/plotly where error bars are properties
//! of data series.

use ruviz::prelude::*;

fn main() -> Result<()> {
    // === PATTERN 1: Modifier Pattern (Recommended) ===
    // Attach error bars to Line and Scatter series using .with_yerr() / .with_xerr()

    let x: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let y_line: Vec<f64> = vec![2.3, 3.5, 4.1, 5.8, 6.2, 7.5, 8.1];
    let y_line_err: Vec<f64> = vec![0.3, 0.4, 0.25, 0.5, 0.35, 0.4, 0.45];

    let y_scatter: Vec<f64> = vec![1.8, 2.9, 3.5, 4.2, 5.5, 6.0, 7.2];
    let y_scatter_err: Vec<f64> = vec![0.25, 0.35, 0.3, 0.4, 0.35, 0.3, 0.4];
    let x_scatter_err: Vec<f64> = vec![0.15, 0.2, 0.1, 0.15, 0.2, 0.1, 0.15];

    Plot::new()
        .title("Error Bars on Line and Scatter Plots")
        .xlabel("X")
        .ylabel("Y")
        .max_resolution(1920, 1440)
        // Line plot with Y error bars (modifier pattern) - NO markers
        .line(&x, &y_line)
        .with_yerr(&y_line_err)
        .label("Line + Y Errors")
        .color(Color::from_palette(0))
        // Scatter plot with both X and Y error bars (modifier pattern)
        .scatter(&x, &y_scatter)
        .with_yerr(&y_scatter_err)
        .with_xerr(&x_scatter_err)
        .label("Scatter + XY Errors")
        .color(Color::from_palette(1))
        .legend_best()
        .save("docs/images/errorbar_plot.png")?;

    println!("Generated docs/images/errorbar_plot.png");

    // === PATTERN 2: Standalone Error Bars (existing API) ===
    // Use error_bars() for dedicated error bar series

    let y_standalone: Vec<f64> = vec![3.0, 4.2, 4.8, 5.5, 6.8, 7.2, 8.5];
    let y_standalone_err: Vec<f64> = vec![0.4, 0.5, 0.35, 0.6, 0.45, 0.5, 0.55];

    Plot::new()
        .title("Standalone Error Bars")
        .xlabel("X")
        .ylabel("Y")
        .max_resolution(1920, 1440)
        .error_bars(&x, &y_standalone, &y_standalone_err)
        .label("Standalone")
        .color(Color::from_palette(2))
        .marker(MarkerStyle::Triangle)
        .save("docs/images/errorbar_standalone.png")?;

    println!("Generated docs/images/errorbar_standalone.png");

    // === PATTERN 3: Asymmetric Error Bars ===
    // Use .with_yerr_asymmetric() for different upper/lower bounds

    let y_asym: Vec<f64> = vec![2.5, 3.8, 4.5, 5.2, 6.5, 7.0, 8.0];
    let y_lower: Vec<f64> = vec![0.2, 0.3, 0.2, 0.4, 0.3, 0.25, 0.35];
    let y_upper: Vec<f64> = vec![0.5, 0.6, 0.4, 0.7, 0.5, 0.55, 0.6];

    Plot::new()
        .title("Asymmetric Error Bars")
        .xlabel("X")
        .ylabel("Y")
        .max_resolution(1920, 1440)
        .line(&x, &y_asym)
        .with_yerr_asymmetric(&y_lower, &y_upper)
        .label("Asymmetric Errors")
        .color(Color::from_palette(3))
        .marker(MarkerStyle::Diamond)
        .save("docs/images/errorbar_asymmetric.png")?;

    println!("Generated docs/images/errorbar_asymmetric.png");

    // === PATTERN 4: Continuation Method (chaining error bar series) ===
    // Use .error_bars() as continuation method

    let y_a: Vec<f64> = vec![2.0, 3.2, 3.8, 4.5, 5.8, 6.2, 7.5];
    let y_a_err: Vec<f64> = vec![0.35, 0.45, 0.3, 0.55, 0.4, 0.45, 0.5];
    let y_b: Vec<f64> = vec![1.5, 2.5, 3.0, 3.8, 5.0, 5.5, 6.8];
    let y_b_err: Vec<f64> = vec![0.3, 0.4, 0.35, 0.5, 0.4, 0.35, 0.45];

    Plot::new()
        .title("Multiple Error Bar Series (Continuation)")
        .xlabel("X")
        .ylabel("Y")
        .max_resolution(1920, 1440)
        .legend_best()
        .error_bars(&x, &y_a, &y_a_err)
        .label("Dataset A")
        .color(Color::from_palette(0))
        .error_bars(&x, &y_b, &y_b_err)
        .label("Dataset B")
        .color(Color::from_palette(1))
        .save("docs/images/errorbar_continuation.png")?;

    println!("Generated docs/images/errorbar_continuation.png");

    Ok(())
}
