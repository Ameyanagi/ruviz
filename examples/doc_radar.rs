//! Documentation example: Radar Chart
//!
//! Generates docs/images/radar_chart.png for rustdoc
//!
//! This example demonstrates the high-level API for creating radar charts,
//! including the new `add_series()` API for explicit named series.

use ruviz::prelude::*;

fn main() -> Result<()> {
    // New API: add_series() for explicit named series binding
    // This is the recommended approach for clarity
    Plot::new()
        .title("Radar Chart")
        .radar(&["Speed", "Power", "Defense", "Magic", "Luck"])
        .add_series("Player 1", &[85.0, 92.0, 78.0, 65.0, 88.0])
        .with_color(Color::from_hex("#3498db").unwrap())
        .with_fill_alpha(0.3)
        .add_series("Player 2", &[72.0, 68.0, 95.0, 82.0, 75.0])
        .with_color(Color::from_hex("#e74c3c").unwrap())
        .with_fill_alpha(0.3)
        .legend_best()
        .save("docs/images/radar_chart.png")?;

    println!("Generated docs/images/radar_chart.png (high-level API)");

    // Skills comparison with add_series() API
    Plot::new()
        .title("Skills Comparison")
        .radar(&[
            "Programming",
            "Design",
            "Communication",
            "Leadership",
            "Problem Solving",
        ])
        .add_series("Engineer A", &[90.0, 60.0, 75.0, 70.0, 95.0])
        .with_fill_alpha(0.4)
        .add_series("Designer B", &[70.0, 90.0, 85.0, 80.0, 70.0])
        .with_fill_alpha(0.4)
        .legend_best()
        .save("docs/images/radar_skills.png")?;

    println!("Generated docs/images/radar_skills.png");

    Ok(())
}
