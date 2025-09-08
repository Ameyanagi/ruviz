use ruviz::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating a basic plot...");
    
    // Sample data
    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];
    
    // Create a line plot
    let plot = Plot::new()
        .title("Basic Line Plot".to_string())
        .xlabel("X Axis".to_string())
        .ylabel("Y Axis".to_string())
        .line(&x_data, &y_data)
        .render()?;
    
    println!("Plot created successfully!");
    println!("Image dimensions: {}x{}", plot.width, plot.height);
    println!("Image data size: {} bytes", plot.pixels.len());
    
    Ok(())
}