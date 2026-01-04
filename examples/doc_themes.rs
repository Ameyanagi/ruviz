//! Documentation example: Themes comparison
//!
//! Generates docs/images/theme_*.png for rustdoc

use ruviz::prelude::*;

fn main() -> Result<()> {
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();

    // Default theme
    Plot::new()
        .title("Default Theme")
        .xlabel("x")
        .ylabel("y")
        .line(&x, &y)
        .end_series()
        .save("docs/images/theme_default.png")?;
    println!("✓ Generated docs/images/theme_default.png");

    // Dark theme
    Plot::new()
        .title("Dark Theme")
        .xlabel("x")
        .ylabel("y")
        .theme(Theme::dark())
        .line(&x, &y)
        .end_series()
        .save("docs/images/theme_dark.png")?;
    println!("✓ Generated docs/images/theme_dark.png");

    // Seaborn theme
    Plot::new()
        .title("Seaborn Theme")
        .xlabel("x")
        .ylabel("y")
        .theme(Theme::seaborn())
        .line(&x, &y)
        .end_series()
        .save("docs/images/theme_seaborn.png")?;
    println!("✓ Generated docs/images/theme_seaborn.png");

    // Publication theme
    Plot::new()
        .title("Publication Theme")
        .xlabel("x")
        .ylabel("y")
        .theme(Theme::publication())
        .line(&x, &y)
        .end_series()
        .save("docs/images/theme_publication.png")?;
    println!("✓ Generated docs/images/theme_publication.png");

    Ok(())
}
