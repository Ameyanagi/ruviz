//! Documentation example: Bar chart
//!
//! Generates docs/images/bar_chart.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    let categories = vec!["A", "B", "C", "D", "E"];
    let values = vec![23.0, 45.0, 56.0, 78.0, 32.0];

    Plot::new()
        .title("Bar Chart")
        .xlabel("Category")
        .ylabel("Value")
        .dpi(300)
        .bar(&categories, &values)
        .end_series()
        .save("docs/images/bar_chart.png")?;

    println!("✓ Generated docs/images/bar_chart.png");
    Ok(())
}
