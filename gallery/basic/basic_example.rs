use ruviz::prelude::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("Creating a basic plot...");
    
    // Ensure gallery directory exists
    std::fs::create_dir_all("gallery")?;
    
    // Sample data
    let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let y_data = vec![2.0, 4.0, 1.0, 3.0, 5.0];
    
    // Create and save a line plot
    Plot::new()
        .title("Basic Line Plot with Cosmic-Text".to_string())
        .xlabel("X Axis".to_string())
        .ylabel("Y Axis".to_string())
        .line(&x_data, &y_data)
        .save("gallery/basic/basic_example.png")?;

    
    println!("Plot created successfully!");
    println!("Plot saved as: gallery/basic/basic_example.png");
    
    Ok(())
}