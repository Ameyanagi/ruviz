//! PDF export example demonstrating vector-based PDF output
//!
//! Run with: cargo run --example pdf_export_example --features pdf

use ruviz::prelude::*;

fn main() -> Result<()> {
    // Example 1: Simple line plot to PDF
    line_plot_pdf()?;

    // Example 2: Scatter plot to PDF
    scatter_plot_pdf()?;

    // Example 3: Bar chart to PDF
    bar_chart_pdf()?;

    // Example 4: Multi-series plot to PDF
    multi_series_pdf()?;

    println!("All PDF examples saved successfully!");
    println!("Open the PDF files in a PDF viewer to see vector graphics.");
    Ok(())
}

/// Simple line plot exported to PDF
fn line_plot_pdf() -> Result<()> {
    let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&x| (x * 2.0).sin()).collect();

    Plot::new()
        .line(&x, &y)
        .color(Color::new(31, 119, 180))
        .title("Sine Wave (PDF Export)")
        .xlabel("X")
        .ylabel("sin(2x)")
        .save_pdf("line_plot.pdf")?;

    println!("Saved: line_plot.pdf");
    Ok(())
}

/// Scatter plot exported to PDF
fn scatter_plot_pdf() -> Result<()> {
    // Generate some random-looking data
    let x: Vec<f64> = (0..30).map(|i| i as f64 * 0.3).collect();
    let y: Vec<f64> = x.iter().map(|&x| x * 0.5 + (x * 3.0).sin() * 0.5).collect();

    Plot::new()
        .scatter(&x, &y)
        .color(Color::new(214, 39, 40))
        .title("Scatter Plot (PDF)")
        .xlabel("X")
        .ylabel("Y")
        .save_pdf("scatter_plot.pdf")?;

    println!("Saved: scatter_plot.pdf");
    Ok(())
}

/// Bar chart exported to PDF
fn bar_chart_pdf() -> Result<()> {
    let categories = vec!["Q1", "Q2", "Q3", "Q4"];
    let values = vec![25.0, 45.0, 30.0, 55.0];

    Plot::new()
        .bar(&categories, &values)
        .color(Color::new(44, 160, 44))
        .title("Quarterly Sales (PDF)")
        .xlabel("Quarter")
        .ylabel("Sales ($K)")
        .save_pdf("bar_chart.pdf")?;

    println!("Saved: bar_chart.pdf");
    Ok(())
}

/// Multi-series plot exported to PDF
fn multi_series_pdf() -> Result<()> {
    let x: Vec<f64> = (0..40).map(|i| i as f64 * 0.1).collect();
    let y1: Vec<f64> = x.iter().map(|&x| x.sin()).collect();
    let y2: Vec<f64> = x.iter().map(|&x| x.cos()).collect();

    Plot::new()
        .line(&x, &y1)
        .color(Color::new(31, 119, 180))
        .label("sin(x)")
        .line(&x, &y2)
        .color(Color::new(255, 127, 14))
        .label("cos(x)")
        .title("Trigonometric Functions (PDF)")
        .xlabel("X (radians)")
        .ylabel("Y")
        .save_pdf("multi_series.pdf")?;

    println!("Saved: multi_series.pdf");
    Ok(())
}
