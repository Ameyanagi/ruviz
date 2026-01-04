use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Creating and saving a plot...");
    
    // Sample data
    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];
    
    // Create and render plot
    let image = Plot::new()
        .title("Test Line Plot".to_string())
        .xlabel("X Values".to_string())
        .ylabel("Y Values".to_string())
        .line(&x_data, &y_data)
        .render()?;
    
    println!("Plot rendered successfully!");
    println!("Image dimensions: {}x{}", image.width, image.height);
    println!("Image data size: {} bytes", image.pixels.len());
    
    // Save the raw RGBA pixel data to a file
    std::fs::write("output_plot.rgba", &image.pixels)?;
    println!("Raw RGBA data saved to: output_plot.rgba");
    
    // Note: To save as PNG, we'd need to integrate with the `image` crate properly
    // The current implementation creates raw RGBA pixel data
    
    Ok(())
}