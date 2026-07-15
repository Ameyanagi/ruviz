use ruviz::core::{BackendOperation, BackendType, IntoPlot, Plot};
use ruviz::render::Theme;
use std::fs;

const WIDTH: u32 = 1_200;
const HEIGHT: u32 = 900;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("gallery/publication/basic")?;
    fs::create_dir_all("gallery/publication/datashader")?;
    fs::create_dir_all("gallery/publication/themes")?;

    generate_exact_pixel_examples()?;
    generate_explicit_datashader_example()?;
    generate_theme_examples()?;

    println!("Generated gallery images with exact {WIDTH}x{HEIGHT} output pixels.");
    println!("No physical size or DPI is implied by save_with_size().");
    Ok(())
}

fn generate_exact_pixel_examples() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..2_000).map(|i| i as f64 * 0.01).collect();
    let y: Vec<f64> = x.iter().map(|&value| (value * 2.0).sin()).collect();

    Plot::new()
        .title("Sinusoidal Wave Function")
        .xlabel("Time")
        .ylabel("Amplitude")
        .line(&x, &y)
        .save_with_size(
            "gallery/publication/basic/line_plot_exact_pixels.png",
            WIDTH,
            HEIGHT,
        )?;

    let scatter_y: Vec<f64> = x
        .iter()
        .map(|&value| value * 0.5 + 0.3 * (value * 2.0 + 1.0).sin())
        .collect();
    Plot::new()
        .title("Deterministic Scatter Example")
        .xlabel("Independent Variable")
        .ylabel("Dependent Variable")
        .scatter(&x, &scatter_y)
        .save_with_size(
            "gallery/publication/basic/scatter_plot_exact_pixels.png",
            WIDTH,
            HEIGHT,
        )?;

    Ok(())
}

fn generate_explicit_datashader_example() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..250_000).map(|i| i as f64 * 0.0001).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&value| (value * 15.0).sin() * (1.0 + 0.5 * (value * 3.0).cos()))
        .collect();

    let plot = Plot::new()
        .backend(BackendType::DataShader)
        .title("Explicit DataShader Scatter")
        .scatter(&x, &y)
        .into_plot();
    report_backend("DataShader gallery example", &plot);
    plot.save_with_size(
        "gallery/publication/datashader/explicit_scatter_exact_pixels.png",
        WIDTH,
        HEIGHT,
    )?;

    Ok(())
}

fn generate_theme_examples() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let x: Vec<f64> = (0..500).map(|i| i as f64 * 0.02).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&value| value.sin() + 0.5 * (value * 2.0).sin())
        .collect();

    for (name, theme) in [
        ("publication", Theme::publication()),
        ("dark", Theme::dark()),
        ("light", Theme::light()),
        ("minimal", Theme::minimal()),
    ] {
        Plot::new()
            .title(format!("{} Theme", name.to_uppercase()))
            .theme(theme)
            .line(&x, &y)
            .save_with_size(
                format!("gallery/publication/themes/{name}_exact_pixels.png"),
                WIDTH,
                HEIGHT,
            )?;
    }

    Ok(())
}

fn report_backend(label: &str, plot: &Plot) {
    let resolution = plot.backend_resolution(BackendOperation::Png);
    println!("{label}");
    println!("  requested: {}", plot.get_backend_name());
    println!("  resolved: {}", plot.resolved_backend_name());
    println!("  fallback: {:?}", resolution.fallback_reason());
}
