//! Documentation example: Heatmap
//!
//! Generates docs/images/heatmap.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    // Create sample 2D data
    let data: Vec<Vec<f64>> = (0..10)
        .map(|i| {
            (0..10)
                .map(|j| ((i as f64 - 5.0).powi(2) + (j as f64 - 5.0).powi(2)).sqrt())
                .collect()
        })
        .collect();

    Plot::new()
        .title("Heatmap")
        .xlabel("X")
        .ylabel("Y")
        .heatmap(&data, None)
        .end_series()
        .save("docs/images/heatmap.png")?;

    println!("✓ Generated docs/images/heatmap.png");
    Ok(())
}
