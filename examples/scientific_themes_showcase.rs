use ruviz::core::{Position, Result};
use ruviz::prelude::*;
use ruviz::render::{Theme, ThemeVariant};
use std::f64::consts::PI;

/// Comprehensive showcase of scientific plotting themes
/// Tests IEEE, Nature, Presentation, and Paul Tol themes with different plot types
fn main() -> Result<()> {
    println!("🔬 Scientific Themes Showcase");
    println!("Testing IEEE, Nature, Presentation, and Paul Tol themes");

    // Generate sample scientific data
    let n = 50;
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 2.0 * PI / n as f64).collect();
    let y_sin: Vec<f64> = x.iter().map(|&x| x.sin()).collect();
    let y_cos: Vec<f64> = x.iter().map(|&x| x.cos()).collect();
    let y_exp: Vec<f64> = x.iter().map(|&x| (x / 10.0).exp()).collect();

    // Normal distribution data for histograms
    let histogram_data: Vec<f64> = generate_normal_distribution(1000, 0.0, 1.0);

    // 1. IEEE Theme - Publication quality with accessibility
    println!("📄 Creating IEEE publication theme plot...");
    Plot::new()
        .theme(Theme::ieee())
        .title("IEEE Publication Theme")
        .xlabel("Angle (radians)")
        .ylabel("Amplitude")
        .line(&x, &y_sin)
        .label("sin(x)")
        .line(&x, &y_cos)
        .label("cos(x)")
        .end_series()
        .xlim(0.0, 2.0 * PI)
        .legend(Position::TopRight)
        .save("examples/output/ieee_theme_example.png")?;

    // 2. Nature Journal Theme - Minimal grid, tight spacing
    println!("🧬 Creating Nature journal theme plot...");
    Plot::new()
        .theme(Theme::nature())
        .title("Nature Journal Theme")
        .xlabel("x")
        .ylabel("exp(x/10)")
        .scatter(&x, &y_exp)
        .label("exp(x/10)")
        .end_series()
        .xlim(0.0, 2.0 * PI)
        .legend(Position::TopRight)
        .save("examples/output/nature_theme_example.png")?;

    // 3. Presentation Theme - Large fonts, high contrast
    println!("📊 Creating presentation theme plot...");
    let categories = vec!["A", "B", "C", "D", "E"];
    let values = vec![23.0, 45.0, 56.0, 78.0, 32.0];

    Plot::new()
        .theme(Theme::presentation())
        .title("Presentation Theme")
        .xlabel("Categories")
        .ylabel("Values")
        .bar(&categories, &values)
        .end_series()
        .save("examples/output/presentation_theme_example.png")?;

    // 4. Paul Tol Theme - Scientifically tested colorblind-friendly
    println!("🎨 Creating Paul Tol accessibility theme plot...");
    let x_multi: Vec<f64> = (0..30).map(|i| i as f64).collect();
    let y1: Vec<f64> = x_multi.iter().map(|&x| x.sin()).collect();
    let y2: Vec<f64> = x_multi.iter().map(|&x| (x * 0.5).cos()).collect();
    let y3: Vec<f64> = x_multi.iter().map(|&x| (x * 0.3).sin() * 0.7).collect();

    Plot::new()
        .theme(Theme::paul_tol())
        .title("Paul Tol Accessibility Theme")
        .xlabel("Time")
        .ylabel("Signal")
        .line(&x_multi, &y1)
        .label("Series 1")
        .line(&x_multi, &y2)
        .label("Series 2")
        .line(&x_multi, &y3)
        .label("Series 3")
        .end_series()
        .legend(Position::TopRight)
        .save("examples/output/paul_tol_theme_example.png")?;

    // 5. Scientific palette histogram comparison
    println!("📈 Creating scientific palette histogram...");
    use ruviz::plots::histogram::HistogramConfig;

    let mut hist_config = HistogramConfig::new();
    hist_config.bins = Some(20);

    Plot::new()
        .theme(Theme::ieee())
        .title("Scientific Color Palette - IEEE Theme")
        .xlabel("Value")
        .ylabel("Frequency")
        .histogram(&histogram_data, Some(hist_config))
        .end_series()
        .save("examples/output/scientific_palette_histogram.png")?;

    // 6. Theme comparison - same data, different themes
    println!("🔄 Creating theme comparison plots...");

    // Create the same plot with different themes for comparison
    let comparison_data = vec![
        ("Light", ThemeVariant::Light),
        ("Seaborn", ThemeVariant::Seaborn),
        ("IEEE", ThemeVariant::IEEE),
        ("Nature", ThemeVariant::Nature),
    ];

    for (name, theme_variant) in comparison_data {
        Plot::new()
            .theme(theme_variant.to_theme())
            .title(format!("{} Theme Comparison", name))
            .xlabel("x")
            .ylabel("y")
            .line(&x, &y_sin)
            .label("sin(x)")
            .line(&x, &y_cos)
            .label("cos(x)")
            .end_series()
            .xlim(0.0, 2.0 * PI)
            .legend(Position::TopRight)
            .save(format!(
                "examples/output/{}_theme_comparison.png",
                name.to_lowercase()
            ))?;
    }

    println!("✅ Scientific themes showcase completed!");
    println!("📁 Check test_output/ for generated plots:");
    println!("   - ieee_theme_example.png (Publication ready)");
    println!("   - nature_theme_example.png (Journal style)");
    println!("   - presentation_theme_example.png (High contrast)");
    println!("   - paul_tol_theme_example.png (Accessibility optimized)");
    println!("   - scientific_palette_histogram.png (Scientific colors)");
    println!("   - *_theme_comparison.png (Theme comparisons)");

    Ok(())
}

/// Generate normal distribution data for testing
fn generate_normal_distribution(n: usize, mean: f64, std_dev: f64) -> Vec<f64> {
    use std::f64::consts::PI;

    let mut rng_state = 12345u64; // Simple LCG for reproducible results
    let mut data = Vec::with_capacity(n);

    for _ in 0..n {
        // Box-Muller transform for normal distribution
        rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let u1 = (rng_state as f64) / (u64::MAX as f64);

        rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let u2 = (rng_state as f64) / (u64::MAX as f64);

        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
        data.push(z0 * std_dev + mean);
    }

    data
}
