use ruviz::core::Result;
use ruviz::prelude::*;
use std::time::Instant;

/// Scientific plotting showcase demonstrating publication-quality multi-panel figures

fn main() -> Result<()> {
    println!("Scientific Plotting Showcase");
    std::fs::create_dir_all("examples/output").ok();

    let start_time = Instant::now();

    // Generate realistic scientific datasets
    println!("Generating scientific datasets...");

    // Dataset 1: Time series experimental data with noise
    let time_points = 1000;
    let time: Vec<f64> = (0..time_points).map(|i| i as f64 * 0.01).collect();
    let signal: Vec<f64> = time
        .iter()
        .map(|&t| {
            let base_signal = 5.0 * (-t * 0.2).exp() * (t * 3.0).sin();
            let noise = (t * 137.0).sin() * 0.3;
            base_signal + noise
        })
        .collect();

    // Dataset 2: Scatter plot correlation data
    let sample_size = 500;
    let x_values: Vec<f64> = (0..sample_size)
        .map(|i| {
            let base = i as f64 * 0.02;
            base + (base * 23.0).sin() * 0.1
        })
        .collect();
    let y_values: Vec<f64> = x_values
        .iter()
        .map(|&x| 2.5 * x + 1.2 + (x * 17.0).cos() * 0.8)
        .collect();

    // Dataset 3: Distribution data
    let dist_samples = 2000;
    let distribution: Vec<f64> = (0..dist_samples)
        .map(|i| {
            let t = i as f64 / 100.0;
            if i % 3 == 0 {
                5.0 + (t * 7.0).sin() * 2.0
            } else {
                12.0 + (t * 11.0).cos() * 1.5
            }
        })
        .collect();

    // Dataset 4: Box plot group data
    let group_size = 100;
    let group1: Vec<f64> = (0..group_size)
        .map(|i| 8.0 + (i as f64 * 0.1).sin() * 2.0 + (i as f64 * 0.07).cos() * 1.0)
        .collect();

    println!("Scientific datasets generated");

    // Create 2x2 subplot layout
    println!("Creating publication-quality subplot figure...");
    let subplot_start = Instant::now();

    let plot_a: Plot = Plot::new()
        .title("A) Experimental Time Series")
        .xlabel("Time (seconds)")
        .ylabel("Signal Amplitude")
        .theme(Theme::seaborn())
        .line(&time, &signal)
        .into();

    let plot_b: Plot = Plot::new()
        .title("B) Variable Correlation Analysis")
        .xlabel("Independent Variable")
        .ylabel("Dependent Variable")
        .theme(Theme::seaborn())
        .scatter(&x_values, &y_values)
        .into();

    let plot_c: Plot = Plot::new()
        .title("C) Data Distribution Histogram")
        .xlabel("Value Bins")
        .ylabel("Frequency")
        .theme(Theme::seaborn())
        .histogram(&distribution, None)
        .into();

    let plot_d: Plot = Plot::new()
        .title("D) Multi-Group Statistical Analysis")
        .xlabel("Experimental Groups")
        .ylabel("Measured Values")
        .theme(Theme::seaborn())
        .boxplot(&group1, None)
        .into();

    let figure = SubplotFigure::new(2, 2, 1600, 1200)?
        .suptitle("Scientific Data Analysis - Multi-Panel Figure")
        .hspace(0.3)
        .wspace(0.3)
        .subplot(0, 0, plot_a)?
        .subplot(0, 1, plot_b)?
        .subplot(1, 0, plot_c)?
        .subplot(1, 1, plot_d)?;

    figure.save("examples/output/scientific_analysis_figure.png")?;

    let subplot_time = subplot_start.elapsed();
    println!("Scientific figure completed in {:?}", subplot_time);

    // Create individual high-resolution plots
    println!("\nCreating detailed individual plots...");

    Plot::new()
        .title("High-Resolution Experimental Time Series Analysis")
        .xlabel("Time (seconds)")
        .ylabel("Signal Amplitude (arbitrary units)")
        .size_px(1400, 800)
        .theme(Theme::seaborn())
        .line(&time, &signal)
        .save("examples/output/detailed_timeseries.png")?;

    Plot::new()
        .title("Correlation Analysis with Statistical Significance")
        .xlabel("Independent Variable (normalized units)")
        .ylabel("Dependent Variable (measured response)")
        .size_px(1200, 1000)
        .theme(Theme::seaborn())
        .scatter(&x_values, &y_values)
        .save("examples/output/detailed_correlation.png")?;

    Plot::new()
        .title("Statistical Distribution Analysis")
        .xlabel("Measurement Values")
        .ylabel("Frequency Count")
        .size_px(1200, 800)
        .theme(Theme::seaborn())
        .histogram(&distribution, None)
        .save("examples/output/detailed_distribution.png")?;

    let total_time = start_time.elapsed();

    println!("\nScientific Plotting Performance Report:");
    println!(
        "  Data: {} samples across 4 datasets",
        time_points + sample_size + dist_samples + group_size
    );
    println!("  Multi-panel figure: {:?}", subplot_time);
    println!("  Individual plots: 3 high-resolution figures");
    println!("  Total execution: {:?}", total_time);

    println!("\nGenerated Scientific Figures:");
    println!("  scientific_analysis_figure.png (2x2 multi-panel)");
    println!("  detailed_timeseries.png");
    println!("  detailed_correlation.png");
    println!("  detailed_distribution.png");

    Ok(())
}
