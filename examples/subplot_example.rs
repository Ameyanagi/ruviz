/// Subplot Example - Demonstrates the subplot functionality for multiple plots in one figure
/// 
/// This example shows how to create a 2x2 grid of subplots with different plot types:
/// - Top-left: Line plot
/// - Top-right: Scatter plot  
/// - Bottom-left: Bar plot
/// - Bottom-right: Line plot with different styling

use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating subplot figure with 2x2 grid...");

    // Data for different plots
    let x1 = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y1 = vec![2.0, 4.0, 3.0, 5.0, 4.5];
    
    let x2 = vec![0.5, 1.5, 2.5, 3.5, 4.5];
    let y2 = vec![1.0, 3.0, 2.5, 4.0, 3.5];
    
    let categories = vec!["A", "B", "C", "D"];
    let values = vec![10.0, 15.0, 8.0, 12.0];
    
    let x3 = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y3 = vec![0.0, 1.0, 4.0, 9.0, 16.0, 25.0];

    // Create individual plots
    let plot1 = Plot::new()
        .line(&x1, &y1)
        .title("Line Plot")
        .xlabel("X values")
        .ylabel("Y values")
        .end_series();

    let plot2 = Plot::new()
        .scatter(&x2, &y2) 
        .title("Scatter Plot")
        .xlabel("X values")
        .ylabel("Y values")
        .end_series();

    let plot3 = Plot::new()
        .bar(&categories, &values)
        .title("Bar Plot")
        .xlabel("Categories")
        .ylabel("Values")
        .end_series();

    let plot4 = Plot::new()
        .line(&x3, &y3)
        .title("Quadratic Function")
        .xlabel("X values") 
        .ylabel("Y = X²")
        .end_series();

    // Create subplot figure with 2x2 grid
    let figure = subplots(2, 2, 800, 600)?
        .suptitle("Subplot Example - Multiple Plot Types")
        .hspace(0.3)  // More horizontal spacing
        .wspace(0.3)  // More vertical spacing
        .subplot(0, 0, plot1)?  // Top-left
        .subplot(0, 1, plot2)?  // Top-right
        .subplot(1, 0, plot3)?  // Bottom-left
        .subplot(1, 1, plot4)?; // Bottom-right

    // Save the subplot figure
    let output_path = "subplot_example.png";
    let subplot_count = figure.subplot_count();
    figure.save(output_path)?;
    
    println!("Subplot figure saved to: {}", output_path);
    println!("The figure contains {} subplots in a 2x2 grid", subplot_count);
    
    Ok(())
}