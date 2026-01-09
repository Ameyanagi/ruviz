/// Subplot Example - Demonstrates the subplot functionality for multiple plots in one figure
///
/// Creates a 2x2 grid of subplots with different plot types:
/// - Top-left: Line plot
/// - Top-right: Scatter plot
/// - Bottom-left: Bar plot
/// - Bottom-right: Quadratic function
use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Creating subplot figure with 2x2 grid...");
    std::fs::create_dir_all("examples/output").ok();

    // Data for different plots
    let x1 = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y1 = vec![2.0, 4.0, 3.0, 5.0, 4.5];

    let x2 = vec![0.5, 1.5, 2.5, 3.5, 4.5];
    let y2 = vec![1.0, 3.0, 2.5, 4.0, 3.5];

    let categories = vec!["A", "B", "C", "D"];
    let values = vec![10.0, 15.0, 8.0, 12.0];

    let x3 = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y3 = vec![0.0, 1.0, 4.0, 9.0, 16.0, 25.0];

    // Create individual plots using the simplified API
    let plot1: Plot = Plot::new()
        .title("Line Plot")
        .xlabel("X values")
        .ylabel("Y values")
        .line(&x1, &y1)
        .into();

    let plot2: Plot = Plot::new()
        .title("Scatter Plot")
        .xlabel("X values")
        .ylabel("Y values")
        .scatter(&x2, &y2)
        .into();

    let plot3: Plot = Plot::new()
        .title("Bar Plot")
        .xlabel("Categories")
        .ylabel("Values")
        .bar(&categories, &values)
        .into();

    let plot4: Plot = Plot::new()
        .title("Quadratic Function")
        .xlabel("X values")
        .ylabel("Y = X^2")
        .line(&x3, &y3)
        .into();

    // Create subplot figure with 2x2 grid
    let figure = subplots(2, 2, 800, 600)?
        .suptitle("Subplot Example - Multiple Plot Types")
        .hspace(0.3)
        .wspace(0.3)
        .subplot(0, 0, plot1)?
        .subplot(0, 1, plot2)?
        .subplot(1, 0, plot3)?
        .subplot(1, 1, plot4)?;

    figure.save("examples/output/subplot_example.png")?;
    println!("Subplot figure saved to: examples/output/subplot_example.png");

    Ok(())
}
