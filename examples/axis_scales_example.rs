//! Axis Scales Example - demonstrates logarithmic and symmetric log scales
//!
//! Run with: cargo run --example axis_scales_example

use ruviz::prelude::*;

fn main() -> Result<()> {
    // Create exponential data suitable for log scale
    let x_log: Vec<f64> = (1..=5).map(|i| 10.0_f64.powi(i as i32)).collect();
    let y_log: Vec<f64> = x_log.iter().map(|&x| x * 2.0).collect();

    // Linear scale (default)
    Plot::new()
        .line(&x_log, &y_log)
        .title("Linear Scale (Default)")
        .xlabel("X")
        .ylabel("Y")
        .save("examples/output/scale_linear.png")?;
    println!("Linear scale plot saved to test_output/scale_linear.png");

    // Log-log scale
    Plot::new()
        .line(&x_log, &y_log)
        .xscale(AxisScale::Log)
        .yscale(AxisScale::Log)
        .title("Log-Log Scale")
        .xlabel("X (log scale)")
        .ylabel("Y (log scale)")
        .save("examples/output/scale_loglog.png")?;
    println!("Log-log scale plot saved to test_output/scale_loglog.png");

    // Semi-log plot (log Y axis only)
    let x_semilog: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
    let y_semilog: Vec<f64> = x_semilog.iter().map(|&x| (x * 0.5).exp()).collect();

    Plot::new()
        .line(&x_semilog, &y_semilog)
        .yscale(AxisScale::Log)
        .title("Semi-Log Plot (Exponential Growth)")
        .xlabel("Time")
        .ylabel("Value (log scale)")
        .save("examples/output/scale_semilog.png")?;
    println!("Semi-log scale plot saved to test_output/scale_semilog.png");

    // Symmetric log scale for data with both positive and negative values
    let x_symlog: Vec<f64> = (-50..=50).map(|i| i as f64).collect();
    let y_symlog: Vec<f64> = x_symlog.iter().map(|&x| x.powi(3) / 100.0).collect();

    Plot::new()
        .line(&x_symlog, &y_symlog)
        .yscale(AxisScale::symlog(1.0))
        .title("Symmetric Log Scale")
        .xlabel("X")
        .ylabel("Y (symlog scale, linthresh=1.0)")
        .save("examples/output/scale_symlog.png")?;
    println!("Symlog scale plot saved to test_output/scale_symlog.png");

    // Power law data showing utility of log scales
    let x_power: Vec<f64> = (1..100).map(|i| i as f64).collect();
    let y_power: Vec<f64> = x_power.iter().map(|&x| x.powf(2.5)).collect();

    Plot::new()
        .line(&x_power, &y_power)
        .title("Power Law - Linear Scale")
        .xlabel("X")
        .ylabel("Y = X^2.5")
        .save("examples/output/scale_power_linear.png")?;

    Plot::new()
        .line(&x_power, &y_power)
        .xscale(AxisScale::Log)
        .yscale(AxisScale::Log)
        .title("Power Law - Log-Log Scale")
        .xlabel("X (log)")
        .ylabel("Y = X^2.5 (log)")
        .save("examples/output/scale_power_loglog.png")?;
    println!("Power law plots saved to test_output/");

    Ok(())
}
