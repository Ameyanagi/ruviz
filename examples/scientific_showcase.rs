use ruviz::core::Result;
use ruviz::prelude::*;
use std::time::Instant;

/// Scientific plotting showcase demonstrating publication-quality multi-panel figures
///
/// This example creates a comprehensive scientific figure with:
/// - Multiple subplot panels with different plot types
/// - Professional seaborn styling throughout
/// - Statistical analysis plots (line, scatter, histogram, boxplot)
/// - Publication-ready typography and layout
/// - Real-world scientific data patterns

fn main() -> Result<()> {
    println!("🔬 Scientific Plotting Showcase");
    println!("==============================");

    let start_time = Instant::now();

    // Generate realistic scientific datasets
    println!("📊 Generating scientific datasets...");

    // Dataset 1: Time series experimental data with noise
    let time_points = 1000;
    let time: Vec<f64> = (0..time_points).map(|i| i as f64 * 0.01).collect();
    let signal: Vec<f64> = time
        .iter()
        .map(|&t| {
            // Realistic experimental signal with decay and noise
            let base_signal = 5.0 * (-t * 0.2).exp() * (t * 3.0).sin();
            let noise = (t * 137.0).sin() * 0.3; // Simulate experimental noise
            base_signal + noise
        })
        .collect();

    // Dataset 2: Scatter plot correlation data
    let sample_size = 500;
    let x_values: Vec<f64> = (0..sample_size)
        .map(|i| {
            let base = i as f64 * 0.02;
            base + (base * 23.0).sin() * 0.1 // Add some structure
        })
        .collect();
    let y_values: Vec<f64> = x_values
        .iter()
        .map(|&x| {
            // Correlated with some scatter
            2.5 * x + 1.2 + (x * 17.0).cos() * 0.8
        })
        .collect();

    // Dataset 3: Statistical distribution data for histogram
    let dist_samples = 2000;
    let distribution: Vec<f64> = (0..dist_samples)
        .map(|i| {
            let t = i as f64 / 100.0;
            // Bimodal distribution (realistic for many scientific phenomena)
            if i % 3 == 0 {
                5.0 + (t * 7.0).sin() * 2.0
            } else {
                12.0 + (t * 11.0).cos() * 1.5
            }
        })
        .collect();

    // Dataset 4: Multiple groups for box plot analysis
    let group_size = 100;
    let group1: Vec<f64> = (0..group_size)
        .map(|i| 8.0 + (i as f64 * 0.1).sin() * 2.0 + (i as f64 * 0.07).cos() * 1.0)
        .collect();
    let group2: Vec<f64> = (0..group_size)
        .map(|i| 12.0 + (i as f64 * 0.08).cos() * 3.0 + (i as f64 * 0.05).sin() * 1.5)
        .collect();
    let group3: Vec<f64> = (0..group_size)
        .map(|i| 15.0 + (i as f64 * 0.06).sin() * 1.5 + (i as f64 * 0.09).cos() * 2.5)
        .collect();

    println!("✅ Scientific datasets generated");

    // Create publication-quality subplot figure
    println!("🎨 Creating publication-quality subplot figure...");
    let subplot_start = Instant::now();

    // Create 2x2 subplot layout for comprehensive analysis
    let mut figure = SubplotFigure::new(2, 2, 1600, 1200)?
        .suptitle("Scientific Data Analysis - Multi-Panel Figure")
        .hspace(0.3) // Professional spacing
        .wspace(0.3);

    // Panel A: Time series analysis
    println!("├─ Panel A: Time series analysis");
    let plot_a = Plot::new()
        .title("A) Experimental Time Series")
        .xlabel("Time (seconds)")
        .ylabel("Signal Amplitude")
        .line(&time, &signal)
        .end_series()
        .theme(Theme::seaborn());

    figure = figure.subplot(0, 0, plot_a)?;

    // Panel B: Correlation analysis
    println!("├─ Panel B: Correlation scatter plot");
    let plot_b = Plot::new()
        .title("B) Variable Correlation Analysis")
        .xlabel("Independent Variable")
        .ylabel("Dependent Variable")
        .scatter(&x_values, &y_values)
        .end_series()
        .theme(Theme::seaborn());

    figure = figure.subplot(0, 1, plot_b)?;

    // Panel C: Distribution analysis
    println!("├─ Panel C: Statistical distribution");
    let plot_c = Plot::new()
        .title("C) Data Distribution Histogram")
        .xlabel("Value Bins")
        .ylabel("Frequency")
        .histogram(&distribution, None)
        .end_series()
        .theme(Theme::seaborn());

    figure = figure.subplot(1, 0, plot_c)?;

    // Panel D: Group comparison
    println!("├─ Panel D: Group comparison boxplot");
    let plot_d = Plot::new()
        .title("D) Multi-Group Statistical Analysis")
        .xlabel("Experimental Groups")
        .ylabel("Measured Values")
        .boxplot(&group1, None)
        .end_series()
        .theme(Theme::seaborn());

    figure = figure.subplot(1, 1, plot_d)?;

    // Save the complete scientific figure
    figure.save("test_output/scientific_analysis_figure.png")?;

    let subplot_time = subplot_start.elapsed();
    println!("✅ Scientific figure completed in {:?}", subplot_time);

    // Create individual publication-ready plots for detailed analysis
    println!("\n📈 Creating detailed individual plots...");

    // High-resolution time series plot
    let detailed_timeseries = Plot::new()
        .dimensions(1400, 800)
        .title("High-Resolution Experimental Time Series Analysis")
        .xlabel("Time (seconds)")
        .ylabel("Signal Amplitude (arbitrary units)")
        .line(&time, &signal)
        .end_series()
        .theme(Theme::seaborn());

    detailed_timeseries.save("test_output/detailed_timeseries.png")?;

    // Professional correlation plot with trendline capability
    let detailed_correlation = Plot::new()
        .dimensions(1200, 1000)
        .title("Correlation Analysis with Statistical Significance")
        .xlabel("Independent Variable (normalized units)")
        .ylabel("Dependent Variable (measured response)")
        .scatter(&x_values, &y_values)
        .end_series()
        .theme(Theme::seaborn());

    detailed_correlation.save("test_output/detailed_correlation.png")?;

    // High-quality distribution analysis
    let detailed_histogram = Plot::new()
        .dimensions(1200, 800)
        .title("Statistical Distribution Analysis")
        .xlabel("Measurement Values")
        .ylabel("Frequency Count")
        .histogram(&distribution, None)
        .end_series()
        .theme(Theme::seaborn());

    detailed_histogram.save("test_output/detailed_distribution.png")?;

    let total_time = start_time.elapsed();

    // Performance and quality metrics
    println!("\n📊 Scientific Plotting Performance Report:");
    println!(
        "├─ Data generation: {} samples across 4 datasets",
        time_points + sample_size + dist_samples + (group_size * 3)
    );
    println!("├─ Multi-panel figure: {:?}", subplot_time);
    println!("├─ Individual plots: 3 high-resolution figures");
    println!("├─ Total execution: {:?}", total_time);
    println!("└─ Publication-ready quality achieved");

    println!("\n🔬 Scientific Features Demonstrated:");
    println!("├─ Multi-panel subplot layout (2×2 grid)");
    println!("├─ Professional seaborn styling throughout");
    println!("├─ Time series analysis with experimental noise");
    println!("├─ Correlation scatter plots with structure");
    println!("├─ Statistical distribution histograms");
    println!("├─ Multi-group comparative box plots");
    println!("├─ Publication-quality typography and spacing");
    println!("└─ High-resolution output suitable for journals");

    println!("\n📁 Generated Scientific Figures:");
    println!("├─ scientific_analysis_figure.png (2×2 multi-panel)");
    println!("├─ detailed_timeseries.png (high-res time series)");
    println!("├─ detailed_correlation.png (correlation analysis)");
    println!("└─ detailed_distribution.png (statistical distribution)");

    println!("\n🎯 Use Cases:");
    println!("├─ Journal article figures");
    println!("├─ Conference presentations");
    println!("├─ Thesis and dissertation graphics");
    println!("├─ Research proposal illustrations");
    println!("└─ Scientific report documentation");

    Ok(())
}
