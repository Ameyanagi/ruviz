use ruviz::plots::boxplot::BoxPlotConfig;
use ruviz::prelude::*;

fn main() -> ruviz::core::Result<()> {
    // Generate sample data with outliers
    let data = vec![
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        17.0, 18.0, 19.0, 20.0, // Add some outliers
        35.0, 40.0, -5.0,
    ];

    Plot::new()
        .title("Box Plot Example")
        .xlabel("Distribution")
        .ylabel("Values")
        .size_px(800, 600)
        .boxplot(&data, Some(BoxPlotConfig::new()))
        .save("examples/output/boxplot_example.png")?;

    println!("Box plot saved as boxplot_example.png");
    Ok(())
}
