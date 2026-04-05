//! Documentation example: Themes comparison
//!
//! Generates docs/assets/rustdoc/theme_default.png, docs/assets/rustdoc/theme_dark.png,
//! docs/assets/rustdoc/theme_seaborn.png, and docs/assets/rustdoc/theme_publication.png for rustdoc.

use ruviz::prelude::*;

fn main() -> Result<()> {
    let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();

    // Default theme
    Plot::new()
        .title("Default Theme")
        .xlabel("x")
        .ylabel("y")
        .max_resolution(1920, 1440)
        .line(&x, &y)
        .save("docs/assets/rustdoc/theme_default.png")?;
    println!("✓ Generated docs/assets/rustdoc/theme_default.png");

    // Dark theme
    Plot::new()
        .title("Dark Theme")
        .xlabel("x")
        .ylabel("y")
        .max_resolution(1920, 1440)
        .theme(Theme::dark())
        .line(&x, &y)
        .save("docs/assets/rustdoc/theme_dark.png")?;
    println!("✓ Generated docs/assets/rustdoc/theme_dark.png");

    // Seaborn theme
    Plot::new()
        .title("Seaborn Theme")
        .xlabel("x")
        .ylabel("y")
        .max_resolution(1920, 1440)
        .theme(Theme::seaborn())
        .line(&x, &y)
        .save("docs/assets/rustdoc/theme_seaborn.png")?;
    println!("✓ Generated docs/assets/rustdoc/theme_seaborn.png");

    // Publication theme
    Plot::new()
        .title("Publication Theme")
        .xlabel("x")
        .ylabel("y")
        .max_resolution(1920, 1440)
        .theme(Theme::publication())
        .line(&x, &y)
        .save("docs/assets/rustdoc/theme_publication.png")?;
    println!("✓ Generated docs/assets/rustdoc/theme_publication.png");

    Ok(())
}
