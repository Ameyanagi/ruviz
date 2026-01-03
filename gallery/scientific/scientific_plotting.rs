use ruviz::prelude::*;
use std::f64::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating scientific plotting examples...");

    // Create test output directory if it doesn't exist
    std::fs::create_dir_all("test_output")?;

    // Example 1: Error Bars - Experimental vs Theoretical Data
    generate_error_bars_example()?;

    // Example 2: Multi-Series Comparison with Error Bars
    generate_multi_series_with_errors()?;

    // Example 3: Statistical Distribution Visualization
    generate_statistical_plots()?;

    // Example 4: Publication-Quality Multi-Panel Figure
    generate_publication_figure()?;

    println!("✅ Generated scientific plotting examples in test_output/");

    Ok(())
}

/// Generate error bars example showing experimental vs theoretical data
fn generate_error_bars_example() -> Result<(), Box<dyn std::error::Error>> {
    // Simulate experimental data with measurement errors
    let x_data: Vec<f64> = (0..20).map(|i| i as f64 * 0.5).collect();
    let theoretical: Vec<f64> = x_data.iter().map(|&x| x.powi(2) * 0.1).collect();

    // Add noise and errors to simulate experimental data
    let experimental: Vec<f64> = theoretical
        .iter()
        .enumerate()
        .map(|(i, &y)| {
            let noise = (i as f64 * 0.3).sin() * 0.2;
            y + noise
        })
        .collect();

    // Measurement uncertainties (larger for higher values)
    let y_errors: Vec<f64> = experimental.iter().map(|&y| y * 0.1 + 0.05).collect();

    Plot::new()
        .title("Experimental vs Theoretical: Quadratic Growth")
        .xlabel("Time (s)")
        .ylabel("Signal Amplitude (V)")
        .theme(Theme::publication())
        .legend(Position::TopLeft)
        // Theoretical curve (smooth line)
        .line(&x_data, &theoretical)
        .label("Theoretical Model")
        .style(LineStyle::Solid)
        .end_series()
        // Experimental data with error bars
        .error_bars(&x_data, &experimental, &y_errors)
        .label("Experimental Data")
        .marker(MarkerStyle::Circle)
        .end_series()
        .save_with_size("test_output/scientific_error_bars.png", 1200, 900)?;

    Ok(())
}

/// Generate multi-series comparison with different error magnitudes
fn generate_multi_series_with_errors() -> Result<(), Box<dyn std::error::Error>> {
    let x_data: Vec<f64> = (0..15).map(|i| i as f64).collect();

    // Three different measurement techniques with different precision
    let technique_a: Vec<f64> = x_data.iter().map(|&x| x * 2.0 + 1.0).collect();
    let technique_b: Vec<f64> = x_data.iter().map(|&x| x * 1.8 + 2.0).collect();
    let technique_c: Vec<f64> = x_data.iter().map(|&x| x * 2.2 + 0.5).collect();

    // Different error magnitudes for each technique
    let errors_a: Vec<f64> = vec![0.2; x_data.len()]; // High precision
    let errors_b: Vec<f64> = vec![0.5; x_data.len()]; // Medium precision
    let errors_c: Vec<f64> = vec![0.8; x_data.len()]; // Low precision

    Plot::new()
        .title("Comparison of Measurement Techniques")
        .xlabel("Sample Number")
        .ylabel("Measured Value (units)")
        .theme(Theme::publication())
        .legend(Position::TopLeft)
        .error_bars(&x_data, &technique_a, &errors_a)
        .label("High Precision (±0.2)")
        .end_series()
        .error_bars(&x_data, &technique_b, &errors_b)
        .label("Medium Precision (±0.5)")
        .end_series()
        .error_bars(&x_data, &technique_c, &errors_c)
        .label("Low Precision (±0.8)")
        .end_series()
        .save_with_size("test_output/scientific_multi_series_errors.png", 1200, 900)?;

    Ok(())
}

/// Generate statistical distribution plots
fn generate_statistical_plots() -> Result<(), Box<dyn std::error::Error>> {
    // Generate normal distribution data
    let x_range: Vec<f64> = (-40..41).map(|i| i as f64 * 0.1).collect();

    // Three normal distributions with different parameters
    let normal_1: Vec<f64> = x_range.iter().map(|&x| gaussian(x, 0.0, 1.0)).collect();

    let normal_2: Vec<f64> = x_range.iter().map(|&x| gaussian(x, 1.0, 0.5)).collect();

    let normal_3: Vec<f64> = x_range.iter().map(|&x| gaussian(x, -0.5, 1.5)).collect();

    Plot::new()
        .title("Statistical Distributions Comparison")
        .xlabel("Standard Deviations (σ)")
        .ylabel("Probability Density")
        .theme(Theme::publication())
        .legend(Position::TopRight)
        .line(&x_range, &normal_1)
        .label("μ=0, σ=1.0")
        .style(LineStyle::Solid)
        .end_series()
        .line(&x_range, &normal_2)
        .label("μ=1, σ=0.5")
        .style(LineStyle::Dashed)
        .end_series()
        .line(&x_range, &normal_3)
        .label("μ=-0.5, σ=1.5")
        .style(LineStyle::Dotted)
        .end_series()
        .save_with_size("test_output/scientific_distributions.png", 1200, 900)?;

    Ok(())
}

/// Generate publication-quality figure with multiple data types
fn generate_publication_figure() -> Result<(), Box<dyn std::error::Error>> {
    // Simulate complex scientific data
    let time: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();

    // Primary signal with exponential decay
    let primary_signal: Vec<f64> = time
        .iter()
        .map(|&t| 10.0 * (-t * 0.1).exp() * (2.0 * PI * t * 0.3).cos())
        .collect();

    // Secondary signal with different frequency
    let secondary_signal: Vec<f64> = time
        .iter()
        .map(|&t| 5.0 * (-t * 0.05).exp() * (2.0 * PI * t * 0.7).sin())
        .collect();

    // Background noise level
    let noise_floor: Vec<f64> = time
        .iter()
        .map(|&t| 0.5 * (1.0 + (t * 0.1).sin()))
        .collect();

    // Measurement points with errors (subset of continuous data)
    let measurement_times: Vec<f64> = (0..10).map(|i| i as f64 * 1.0).collect();
    let measurements: Vec<f64> = measurement_times
        .iter()
        .map(|&t| 10.0 * (-t * 0.1).exp() * (2.0 * PI * t * 0.3).cos() + 0.3 * (t * 1.7).sin()) // Add some measurement noise
        .collect();
    let measurement_errors: Vec<f64> = measurements.iter().map(|&m| m.abs() * 0.1 + 0.2).collect();

    Plot::new()
        .title("Multi-Modal Signal Analysis: Damped Oscillations with Noise")
        .xlabel("Time (s)")
        .ylabel("Signal Amplitude (mV)")
        .theme(Theme::publication())
        .legend(Position::TopRight)
        // Primary continuous signal
        .line(&time, &primary_signal)
        .label("Primary Signal (f₁)")
        .style(LineStyle::Solid)
        .end_series()
        // Secondary continuous signal
        .line(&time, &secondary_signal)
        .label("Secondary Signal (f₂)")
        .style(LineStyle::Dashed)
        .end_series()
        // Background noise level
        .line(&time, &noise_floor)
        .label("Noise Floor")
        .style(LineStyle::Dotted)
        .end_series()
        // Discrete measurements with error bars
        .error_bars(&measurement_times, &measurements, &measurement_errors)
        .label("Measured Data Points")
        .marker(MarkerStyle::Square)
        .end_series()
        .save_with_size("test_output/scientific_publication_quality.png", 1400, 1000)?;

    Ok(())
}

/// Helper function to calculate Gaussian/normal distribution
fn gaussian(x: f64, mu: f64, sigma: f64) -> f64 {
    let coefficient = 1.0 / (sigma * (2.0 * PI).sqrt());
    let exponent = -0.5 * ((x - mu) / sigma).powi(2);
    coefficient * exponent.exp()
}
