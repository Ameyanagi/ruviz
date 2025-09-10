use ruviz::prelude::*;
use ruviz::plots::histogram::HistogramConfig;

fn main() -> ruviz::core::Result<()> {
    // Create test_output directory
    std::fs::create_dir_all("test_output").ok();
    
    // Generate sample data - normal distribution-like
    let data = vec![
        1.2, 1.5, 1.8, 2.1, 2.3, 2.7, 2.9, 3.1, 3.4, 3.6,
        3.8, 4.0, 4.2, 4.5, 4.7, 4.9, 5.1, 5.3, 5.6, 5.8,
        6.0, 6.2, 6.5, 6.7, 6.9, 7.1, 7.4, 7.6, 7.8, 8.0,
        8.2, 8.5, 8.7, 8.9, 9.1, 9.4, 9.6, 9.8, 10.0, 10.2,
        10.5, 10.7, 10.9, 11.1, 11.4, 11.6, 11.8, 12.0
    ];

    let result = Plot::new()
        .dimensions(800, 600)
        .title("Professional Histogram Example - Improved Styling")
        .xlabel("Value Bins")
        .ylabel("Frequency")
        .histogram(&data, Some(HistogramConfig::new()))
        .end_series()
        .theme(Theme::publication())
        .save("test_output/histogram_example.png");

    match result {
        Ok(_) => println!("✅ Professional histogram saved as test_output/histogram_example.png"),
        Err(e) => println!("❌ Error: {}", e),
    }

    Ok(())
}