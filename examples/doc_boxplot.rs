//! Documentation example: Box plot
//!
//! Generates docs/images/boxplot.png for rustdoc

use ruviz::plots::boxplot::BoxPlotConfig;
use ruviz::prelude::*;

fn main() -> Result<()> {
    // Generate sample data with outliers
    let data = vec![
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        17.0, 18.0, 19.0, 20.0, // Add some outliers
        35.0, 40.0, -5.0,
    ];

    Plot::new()
        .title("Box Plot")
        .xlabel("Distribution")
        .ylabel("Values")
        .max_resolution(1920, 1440)
        .boxplot(&data, Some(BoxPlotConfig::new()))
        .save("docs/images/boxplot.png")?;

    println!("✓ Generated docs/images/boxplot.png");
    Ok(())
}
