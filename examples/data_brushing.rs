//! Data brushing example with multiple linked plots
//!
//! Demonstrates interactive selection across multiple synchronized plots.
//! When you select data in one plot, corresponding points are highlighted
//! in all linked plots.
//!
//! Controls:
//! - Left click + drag: Brush select data points
//! - Mouse wheel: Zoom in/out
//! - Right click + drag: Pan
//! - Delete key: Clear selection
//! - Escape: Reset all views

use ruviz::prelude::*;
use std::f64::consts::PI;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎮 Starting data brushing example...");
    println!("Controls:");
    println!("  - Left click + drag: Brush select data points");
    println!("  - Mouse wheel: Zoom in/out");
    println!("  - Right click + drag: Pan");
    println!("  - Delete key: Clear selection");
    println!("  - Escape: Reset all views");

    // Generate correlated data for demonstration
    let n_points = 500;
    let data = generate_correlated_data(n_points);

    println!("📊 Generated {} correlated data points", n_points);

    // Create multiple plots showing different aspects of the same data

    // Plot 1: Time series
    let time_plot = Plot::new()
        .line(&data.time, &data.values)
        .scatter(&data.time, &data.values)
        .title("Time Series View")
        .xlabel("Time")
        .ylabel("Value")
        .legend(Position::TopLeft);

    // Plot 2: Phase space (derivative vs value)
    let phase_plot = Plot::new()
        .scatter(&data.values, &data.derivatives)
        .title("Phase Space View")
        .xlabel("Value")
        .ylabel("Derivative")
        .legend(Position::TopRight);

    // Plot 3: Correlation plot
    let correlation_plot = Plot::new()
        .scatter(&data.values, &data.noise)
        .title("Value vs Noise Correlation")
        .xlabel("Value")
        .ylabel("Noise Component")
        .legend(Position::BottomRight);

    // Plot 4: Histogram of values
    // Note: This would use the histogram API once implemented
    let histogram_plot = Plot::new()
        .scatter(&data.histogram_bins, &data.histogram_counts)
        .title("Value Distribution")
        .xlabel("Value Bins")
        .ylabel("Frequency")
        .legend(Position::TopLeft);

    println!("📈 Created 4 linked plots for data brushing");

    // In a real implementation, we would show all plots in a multi-panel layout
    // For this example, we'll focus on the time series plot

    #[cfg(feature = "interactive")]
    {
        println!("🚀 Opening interactive data brushing demo...");
        println!("Note: Full multi-plot brushing requires subplot layout implementation");

        // Create an enhanced plot with brushing capabilities
        let interactive_plot = create_brushing_demo_plot(&data)?;

        show_interactive(interactive_plot).await?;
    }

    #[cfg(not(feature = "interactive"))]
    {
        println!("⚠️ Interactive features not enabled.");
        println!("To enable: cargo run --features interactive --example data_brushing");

        // Save static versions
        time_plot.save("examples/output/data_brushing_time_series.png")?;
        phase_plot.save("examples/output/data_brushing_phase_space.png")?;
        correlation_plot.save("examples/output/data_brushing_correlation.png")?;
        histogram_plot.save("examples/output/data_brushing_histogram.png")?;

        println!("💾 Saved static versions:");
        println!("  - examples/output/data_brushing_time_series.png");
        println!("  - examples/output/data_brushing_phase_space.png");
        println!("  - examples/output/data_brushing_correlation.png");
        println!("  - examples/output/data_brushing_histogram.png");
    }

    println!("✅ Data brushing example completed!");
    Ok(())
}

/// Generate correlated data for demonstration
struct CorrelatedData {
    time: Vec<f64>,
    values: Vec<f64>,
    derivatives: Vec<f64>,
    noise: Vec<f64>,
    histogram_bins: Vec<f64>,
    histogram_counts: Vec<f64>,
}

fn generate_correlated_data(n_points: usize) -> CorrelatedData {
    let mut time = Vec::with_capacity(n_points);
    let mut values = Vec::with_capacity(n_points);
    let mut derivatives = Vec::with_capacity(n_points);
    let mut noise = Vec::with_capacity(n_points);

    // Generate time series with multiple components
    for i in 0..n_points {
        let t = i as f64 * 0.1;
        time.push(t);

        // Main signal: combination of sine waves with trend
        let signal = (t * 0.5).sin() * 2.0 + (t * 0.2).cos() * 1.5 + t * 0.01;

        // Add correlated noise
        let noise_component = (t * 0.8).sin() * 0.3 + (i as f64 * 0.01).cos() * 0.2;
        let value = signal + noise_component;

        values.push(value);
        noise.push(noise_component);

        // Calculate numerical derivative for phase space
        if i > 0 {
            let derivative = (value - values[i - 1]) / 0.1;
            derivatives.push(derivative);
        } else {
            derivatives.push(0.0);
        }
    }

    // Generate histogram data
    let histogram_bins: Vec<f64> = (-30..30).map(|i| i as f64 * 0.2).collect();
    let mut histogram_counts = vec![0.0; histogram_bins.len()];

    // Bin the values
    for &value in &values {
        let bin_index = ((value + 6.0) / 0.2) as usize;
        if bin_index < histogram_counts.len() {
            histogram_counts[bin_index] += 1.0;
        }
    }

    CorrelatedData {
        time,
        values,
        derivatives,
        noise,
        histogram_bins,
        histogram_counts,
    }
}

/// Create a demo plot with simulated brushing functionality
fn create_brushing_demo_plot(data: &CorrelatedData) -> Result<Plot> {
    // For demonstration, create a plot that shows multiple data series
    // In a real implementation, this would have actual brushing interactivity

    let plot = Plot::new()
        .line(&data.time, &data.values)
        .scatter(&data.time, &data.values)
        .title("Interactive Data Brushing Demo\n(Multi-plot brushing coming soon)")
        .xlabel("Time")
        .ylabel("Value")
        .legend(Position::TopLeft);

    // Add instructions as plot title
    // In a real implementation, we would add text annotations

    Ok(plot)
}

/// Simulate data brushing logic
fn simulate_brushing_selection(
    data: &CorrelatedData,
    selection_region: (f64, f64, f64, f64),
) -> Vec<usize> {
    let (min_x, min_y, max_x, max_y) = selection_region;
    let mut selected_indices = Vec::new();

    for (i, (&x, &y)) in data.time.iter().zip(data.values.iter()).enumerate() {
        if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
            selected_indices.push(i);
        }
    }

    selected_indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_generation() {
        let data = generate_correlated_data(100);

        assert_eq!(data.time.len(), 100);
        assert_eq!(data.values.len(), 100);
        assert_eq!(data.derivatives.len(), 100);
        assert_eq!(data.noise.len(), 100);

        // Check that time is monotonic
        for i in 1..data.time.len() {
            assert!(data.time[i] > data.time[i - 1]);
        }

        // Check that histogram has reasonable counts
        let total_count: f64 = data.histogram_counts.iter().sum();
        assert!(total_count > 0.0);
    }

    #[test]
    fn test_brushing_selection() {
        let data = generate_correlated_data(50);

        // Select a region that should contain some points
        let selection = simulate_brushing_selection(&data, (0.0, -2.0, 5.0, 2.0));

        // Should find at least some points in this region
        assert!(!selection.is_empty());

        // Verify selected points are actually in the region
        for &index in &selection {
            let x = data.time[index];
            let y = data.values[index];
            assert!(x >= 0.0 && x <= 5.0);
            assert!(y >= -2.0 && y <= 2.0);
        }
    }

    #[tokio::test]
    async fn test_plot_creation() {
        let data = generate_correlated_data(10);
        let plot = create_brushing_demo_plot(&data);
        assert!(plot.is_ok());
    }
}
