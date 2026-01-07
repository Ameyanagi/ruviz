//! Documentation example: Pie Chart
//!
//! Generates docs/images/pie_chart.png for rustdoc
//!
//! This example demonstrates both the high-level API and the low-level API
//! for creating pie charts.

use ruviz::prelude::*;

fn main() -> Result<()> {
    // High-level API example
    let values = vec![35.0, 25.0, 20.0, 15.0, 5.0];

    Plot::new()
        .pie(&values)
        .labels(&["Product A", "Product B", "Product C", "Product D", "Other"])
        .show_percentages(true)
        .title("Market Share Distribution")
        .save("docs/images/pie_chart.png")?;

    println!("Generated docs/images/pie_chart.png (high-level API)");

    // Donut chart variant
    Plot::new()
        .pie(&values)
        .labels(&["Product A", "Product B", "Product C", "Product D", "Other"])
        .donut(0.4)
        .show_percentages(true)
        .title("Market Share (Donut)")
        .save("docs/images/pie_donut.png")?;

    println!("Generated docs/images/pie_donut.png (donut chart)");

    Ok(())
}
