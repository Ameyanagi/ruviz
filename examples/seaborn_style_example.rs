use ruviz::prelude::*;
use ruviz::plots::boxplot::BoxPlotConfig;
use ruviz::render::Theme;

fn main() -> ruviz::core::Result<()> {
    // Generate sample data with outliers (same as boxplot example)
    let data = vec![
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0,
        11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0,
        // Add some outliers
        35.0, 40.0, -5.0
    ];

    // Create boxplot with seaborn-style theme
    let result = Plot::new()
        .dimensions(800, 600)
        .title("Seaborn-Style Box Plot")
        .xlabel("Distribution")
        .ylabel("Values")
        .theme(Theme::seaborn())  // Apply seaborn theme
        .boxplot(&data, Some(BoxPlotConfig::new()))
        .end_series()
        .save("test_output/seaborn_boxplot_example.png");

    match result {
        Ok(_) => println!("Seaborn-style boxplot saved as seaborn_boxplot_example.png"),
        Err(e) => println!("Error: {}", e),
    }

    // Also create a histogram with seaborn style
    let hist_data = vec![
        1.2, 1.5, 1.8, 2.1, 2.3, 2.7, 2.9, 3.1, 3.4, 3.6,
        3.8, 4.0, 4.2, 4.5, 4.7, 4.9, 5.1, 5.3, 5.6, 5.8,
        6.0, 6.2, 6.5, 6.7, 6.9, 7.1, 7.4, 7.6, 7.8, 8.0,
        8.2, 8.5, 8.7, 8.9, 9.1, 9.4, 9.6, 9.8, 10.0, 10.2,
    ];

    let hist_result = Plot::new()
        .dimensions(800, 600)
        .title("Seaborn-Style Histogram")
        .xlabel("Value Bins")
        .ylabel("Frequency")
        .theme(Theme::seaborn())  // Apply seaborn theme
        .histogram(&hist_data, None)
        .end_series()
        .save("test_output/seaborn_histogram_example.png");

    match hist_result {
        Ok(_) => println!("Seaborn-style histogram saved as seaborn_histogram_example.png"),
        Err(e) => println!("Error: {}", e),
    }

    Ok(())
}