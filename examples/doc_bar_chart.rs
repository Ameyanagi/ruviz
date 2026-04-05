//! Documentation example: Bar chart
//!
//! Generates docs/assets/rustdoc/bar_chart.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    let categories = vec!["A", "B", "C", "D", "E"];
    let values = vec![23.0, 45.0, 56.0, 78.0, 32.0];

    Plot::new()
        .title("Bar Chart")
        .xlabel("Category")
        .ylabel("Value")
        .max_resolution(1920, 1440)
        .bar(&categories, &values)
        .save("docs/assets/rustdoc/bar_chart.png")?;

    println!("✓ Generated docs/assets/rustdoc/bar_chart.png");
    Ok(())
}
