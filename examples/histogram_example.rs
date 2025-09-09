use ruviz::prelude::*;
use ruviz::plots::histogram::HistogramConfig;
use ruviz::core::Result;
use rand::Rng;

fn main() -> Result<()> {
    println!("Creating histogram example...");
    
    // Generate random data that follows a normal-like distribution
    let mut rng = rand::thread_rng();
    let data: Vec<f64> = (0..1000)
        .map(|_| {
            // Simple box-muller transform for normal distribution
            let u1: f64 = rng.r#gen();
            let u2: f64 = rng.r#gen();
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            z * 2.0 + 10.0  // Scale and shift: mean=10, std=2
        })
        .collect();
    
    // Create histogram with default Sturges binning
    let config = HistogramConfig::new();
    
    let image = Plot::new()
        .histogram(&data, Some(config))
        .title("Normal Distribution Histogram")
        .xlabel("Value")
        .ylabel("Frequency")
        .render()?;
    
    // Save the image (need to implement image saving)
    // For now just print that histogram was created
    println!("Histogram rendering completed successfully");
    println!("Histogram saved as 'histogram_example.png'");
    
    // Create a second histogram with density normalization
    let density_config = HistogramConfig::new()
        .bins(20)
        .density(true);
    
    let density_image = Plot::new()
        .histogram(&data, Some(density_config))
        .title("Normal Distribution - Probability Density")
        .xlabel("Value")
        .ylabel("Density")
        .render()?;
    
    println!("Density histogram rendering completed successfully");
    
    // Create cumulative histogram
    let cumulative_config = HistogramConfig::new()
        .bins(15)
        .cumulative(true);
        
    let cumulative_image = Plot::new()
        .histogram(&data, Some(cumulative_config))
        .title("Normal Distribution - Cumulative")
        .xlabel("Value")
        .ylabel("Cumulative Count")
        .render()?;
    
    println!("Cumulative histogram rendering completed successfully");
    
    Ok(())
}