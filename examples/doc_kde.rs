//! Documentation example: KDE Plot
//!
//! Generates docs/images/kde_plot.png for rustdoc
//!
//! This example demonstrates the high-level KDE (Kernel Density Estimation) API.

use ruviz::prelude::*;

fn main() -> Result<()> {
    // Generate sample data with a bimodal distribution
    let data: Vec<f64> = (0..400)
        .map(|i| {
            let u1 = ((i * 7 + 13) % 400) as f64 / 400.0;
            let u2 = ((i * 11 + 17) % 400) as f64 / 400.0;
            // Mix of two normal distributions
            if i % 2 == 0 {
                3.0 + 1.0
                    * (-2.0 * u1.max(0.01).ln()).sqrt()
                    * (2.0 * std::f64::consts::PI * u2).cos()
            } else {
                7.0 + 1.2
                    * (-2.0 * u1.max(0.01).ln()).sqrt()
                    * (2.0 * std::f64::consts::PI * u2).cos()
            }
        })
        .collect();

    Plot::new()
        .kde(&data)
        .title("Kernel Density Estimation")
        .xlabel("Value")
        .ylabel("Density")
        .max_resolution(1920, 1440)
        .n_points(200)
        .fill(true)
        .fill_alpha(0.4)
        .label("Distribution")
        .color(Color::from_palette(0))
        .legend_best()
        .save("docs/images/kde_plot.png")?;

    println!("Generated docs/images/kde_plot.png");
    Ok(())
}
