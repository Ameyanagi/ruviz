//! Third-order Butterworth magnitude and caller-colored complex-function plots.
//!
//! Run from the repository root with `cargo run --example complex_function`.
//! Both previews are generated from an explicit 320 × 320 source grid. Export
//! DPI changes the rendered output, without evaluating the function again.

use ruviz::prelude::*;
use std::f64::consts::{PI, TAU};
use std::path::Path;

const SOURCE_SIZE: u32 = 320;
const EXTENT: (f64, f64, f64, f64) = (-2.0, 2.0, -2.0, 2.0);
const MIN_MAGNITUDE: f64 = 0.01;
const MAX_MAGNITUDE: f64 = 20.0;
const OUTPUT_DIR: &str = "generated/examples/complex_function";

/// Evaluate H(s) = 1 / (1 + 2s + 2s² + s³), for s = x + iy.
///
/// Explicit real/imaginary arithmetic keeps the example dependency-free.
fn butterworth(x: f64, y: f64) -> (f64, f64) {
    let s2_re = x * x - y * y;
    let s2_im = 2.0 * x * y;
    let s3_re = x * s2_re - y * s2_im;
    let s3_im = x * s2_im + y * s2_re;
    let denominator_re = 1.0 + 2.0 * x + 2.0 * s2_re + s3_re;
    let denominator_im = 2.0 * y + 2.0 * s2_im + s3_im;
    let norm_squared = denominator_re * denominator_re + denominator_im * denominator_im;
    if norm_squared <= f64::MIN_POSITIVE {
        return (f64::INFINITY, 0.0);
    }
    (
        denominator_re / norm_squared,
        -denominator_im / norm_squared,
    )
}

fn clipped_magnitude(x: f64, y: f64) -> f64 {
    let (re, im) = butterworth(x, y);
    re.hypot(im).clamp(MIN_MAGNITUDE, MAX_MAGNITUDE)
}

/// Map phase to hue and logarithmic magnitude to brightness.
/// The caller owns this mapping; an RGBA image has no implicit scalar colorbar.
fn domain_color(x: f64, y: f64) -> Color {
    let (re, im) = butterworth(x, y);
    if !re.is_finite() || !im.is_finite() {
        return Color::WHITE;
    }
    let magnitude = re.hypot(im).clamp(MIN_MAGNITUDE, MAX_MAGNITUDE);
    let normalized_log = (magnitude / MIN_MAGNITUDE).ln() / (MAX_MAGNITUDE / MIN_MAGNITUDE).ln();
    let hue = (im.atan2(re) / TAU).rem_euclid(1.0);
    let saturation = 0.78;
    let value = 0.24 + 0.76 * normalized_log;
    let chroma = value * saturation;
    let h6 = hue * 6.0;
    let secondary = chroma * (1.0 - (h6.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match h6.floor() as u8 {
        0 => (chroma, secondary, 0.0),
        1 => (secondary, chroma, 0.0),
        2 => (0.0, chroma, secondary),
        3 => (0.0, secondary, chroma),
        4 => (secondary, 0.0, chroma),
        _ => (chroma, 0.0, secondary),
    };
    let minimum = value - chroma;
    let byte = |channel: f64| ((channel + minimum) * 255.0).round() as u8;
    Color::from_rgba(byte(r), byte(g), byte(b), 255)
}

fn base_plot(title: &str) -> Plot {
    Plot::with_theme(Theme::publication())
        .size_px(900, 800)
        .title(title)
        .xlabel("Re(s)")
        .ylabel("Im(s)")
        .xlim(EXTENT.0, EXTENT.1)
        .ylim(EXTENT.2, EXTENT.3)
        .aspect_equal()
        .grid(false)
        .font_family("DejaVu Sans")
}

fn overlays(plot: Plot, circle_color: Color, pole_color: Color) -> Plot {
    let circle_x: Vec<f64> = (0..=400).map(|i| (TAU * i as f64 / 400.0).cos()).collect();
    let circle_y: Vec<f64> = (0..=400).map(|i| (TAU * i as f64 / 400.0).sin()).collect();
    // The denominator factors as (s + 1)(s² + s + 1).
    let pole_x = [-1.0, -0.5, -0.5];
    let pole_y = [0.0, (PI / 3.0).sin(), -(PI / 3.0).sin()];
    plot.line(&circle_x, &circle_y)
        .color(circle_color)
        .line_width(1.4)
        .line_style(LineStyle::Dashed)
        .scatter(&pole_x, &pole_y)
        .color(pole_color)
        .marker(MarkerStyle::Cross)
        .marker_size(10.0)
        .into_plot()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(OUTPUT_DIR)?;

    let magnitude = base_plot("Butterworth magnitude |H(s)|")
        .heatmap_fn(SOURCE_SIZE, SOURCE_SIZE, EXTENT, clipped_magnitude)
        .cmap("gray")
        .value_scale(AxisScale::Log)
        .vmin(MIN_MAGNITUDE)
        .vmax(MAX_MAGNITUDE)
        .colorbar_label("|H(s)| · logarithmic scale")
        .colorbar_tick_font_size(10.0)
        .colorbar_label_font_size(11.0)
        .into_plot();
    let magnitude = overlays(
        magnitude,
        Color::from_rgb(51, 173, 220),
        Color::from_rgb(226, 75, 48),
    );
    let magnitude_path = Path::new(OUTPUT_DIR).join("complex_function_magnitude.png");
    magnitude.save(&magnitude_path)?;
    println!("Generated {}", magnitude_path.display());

    let domain = base_plot("Butterworth phase and magnitude")
        .image_fn(SOURCE_SIZE, SOURCE_SIZE, EXTENT, domain_color)
        .interpolation(ImageInterpolation::Bilinear)
        .into_plot();
    let domain = overlays(domain, Color::WHITE, Color::WHITE);
    let domain_path = Path::new(OUTPUT_DIR).join("complex_function_domain.png");
    domain.save(&domain_path)?;
    println!("Generated {}", domain_path.display());
    println!("Dashed circle: |s| = 1; crosses: poles of H(s).");
    println!("Domain image: hue = phase (red at phase 0); brightness = log magnitude.");
    Ok(())
}
