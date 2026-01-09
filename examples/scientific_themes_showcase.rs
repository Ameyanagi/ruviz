use ruviz::core::{Position, Result};
use ruviz::prelude::*;
use ruviz::render::{Theme, ThemeVariant};
use std::f64::consts::PI;

/// Comprehensive showcase of scientific plotting themes
fn main() -> Result<()> {
    println!("Scientific Themes Showcase");
    std::fs::create_dir_all("examples/output").ok();

    // Generate sample scientific data
    let n = 50;
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 2.0 * PI / n as f64).collect();
    let y_sin: Vec<f64> = x.iter().map(|&t| t.sin()).collect();
    let y_cos: Vec<f64> = x.iter().map(|&t| t.cos()).collect();
    let y_exp: Vec<f64> = x.iter().map(|&t| (t / 10.0).exp()).collect();

    let histogram_data = generate_normal_distribution(1000, 0.0, 1.0);

    // 1. IEEE Theme
    println!("Creating IEEE publication theme plot...");
    Plot::new()
        .title("IEEE Publication Theme")
        .xlabel("Angle (radians)")
        .ylabel("Amplitude")
        .xlim(0.0, 2.0 * PI)
        .legend(Position::TopRight)
        .theme(Theme::ieee())
        .line(&x, &y_sin)
        .label("sin(x)")
        .line(&x, &y_cos)
        .label("cos(x)")
        .save("examples/output/ieee_theme_example.png")?;

    // 2. Nature Journal Theme
    println!("Creating Nature journal theme plot...");
    Plot::new()
        .title("Nature Journal Theme")
        .xlabel("x")
        .ylabel("exp(x/10)")
        .xlim(0.0, 2.0 * PI)
        .legend(Position::TopRight)
        .theme(Theme::nature())
        .scatter(&x, &y_exp)
        .label("exp(x/10)")
        .save("examples/output/nature_theme_example.png")?;

    // 3. Presentation Theme
    println!("Creating presentation theme plot...");
    let categories = vec!["A", "B", "C", "D", "E"];
    let values = vec![23.0, 45.0, 56.0, 78.0, 32.0];

    Plot::new()
        .title("Presentation Theme")
        .xlabel("Categories")
        .ylabel("Values")
        .theme(Theme::presentation())
        .bar(&categories, &values)
        .save("examples/output/presentation_theme_example.png")?;

    // 4. Paul Tol Theme - Colorblind-friendly
    println!("Creating Paul Tol accessibility theme plot...");
    let x_multi: Vec<f64> = (0..30).map(|i| i as f64).collect();
    let y1: Vec<f64> = x_multi.iter().map(|&t| t.sin()).collect();
    let y2: Vec<f64> = x_multi.iter().map(|&t| (t * 0.5).cos()).collect();
    let y3: Vec<f64> = x_multi.iter().map(|&t| (t * 0.3).sin() * 0.7).collect();

    Plot::new()
        .title("Paul Tol Accessibility Theme")
        .xlabel("Time")
        .ylabel("Signal")
        .legend(Position::TopRight)
        .theme(Theme::paul_tol())
        .line(&x_multi, &y1)
        .label("Series 1")
        .line(&x_multi, &y2)
        .label("Series 2")
        .line(&x_multi, &y3)
        .label("Series 3")
        .save("examples/output/paul_tol_theme_example.png")?;

    // 5. Scientific palette histogram
    println!("Creating scientific palette histogram...");
    use ruviz::plots::histogram::HistogramConfig;

    let mut hist_config = HistogramConfig::new();
    hist_config.bins = Some(20);

    Plot::new()
        .title("Scientific Color Palette - IEEE Theme")
        .xlabel("Value")
        .ylabel("Frequency")
        .theme(Theme::ieee())
        .histogram(&histogram_data, Some(hist_config))
        .save("examples/output/scientific_palette_histogram.png")?;

    // 6. Theme comparison
    println!("Creating theme comparison plots...");

    let themes = vec![
        ("Light", ThemeVariant::Light),
        ("Seaborn", ThemeVariant::Seaborn),
        ("IEEE", ThemeVariant::IEEE),
        ("Nature", ThemeVariant::Nature),
    ];

    for (name, variant) in themes {
        Plot::new()
            .title(format!("{} Theme Comparison", name))
            .xlabel("x")
            .ylabel("y")
            .xlim(0.0, 2.0 * PI)
            .legend(Position::TopRight)
            .theme(variant.to_theme())
            .line(&x, &y_sin)
            .label("sin(x)")
            .line(&x, &y_cos)
            .label("cos(x)")
            .save(format!(
                "examples/output/{}_theme_comparison.png",
                name.to_lowercase()
            ))?;
    }

    println!("Scientific themes showcase completed!");
    Ok(())
}

fn generate_normal_distribution(n: usize, mean: f64, std_dev: f64) -> Vec<f64> {
    let mut rng_state = 12345u64;
    let mut data = Vec::with_capacity(n);

    for _ in 0..n {
        rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let u1 = (rng_state as f64) / (u64::MAX as f64);

        rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let u2 = (rng_state as f64) / (u64::MAX as f64);

        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
        data.push(z0 * std_dev + mean);
    }

    data
}
