//! Generate mixed Cartesian/non-Cartesian inset examples for manual review.

mod util;

use ruviz::prelude::*;
use std::f64::consts::PI;
use std::fs;
use std::path::PathBuf;

fn generate_xy_data(n: usize) -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.08).collect();
    let y: Vec<f64> = x
        .iter()
        .map(|&xi| (xi * 0.8).sin() + 0.15 * (xi * 0.25).cos())
        .collect();
    (x, y)
}

fn generate_polar_data(n: usize) -> (Vec<f64>, Vec<f64>) {
    let theta: Vec<f64> = (0..n).map(|i| i as f64 * 2.0 * PI / n as f64).collect();
    let r: Vec<f64> = theta.iter().map(|&t| 0.4 + (3.0 * t).cos().abs()).collect();
    (r, theta)
}

fn mixed_polar_plot() -> Plot {
    let (x, y) = generate_xy_data(180);
    let (r, theta) = generate_polar_data(240);

    Plot::new()
        .title("Mixed Cartesian + Polar Inset")
        .xlabel("Time")
        .ylabel("Signal")
        .size(6.4, 4.8)
        .dpi(100)
        .line(&x, &y)
        .color(Color::from_hex("#2563eb").unwrap())
        .grid(true)
        .polar_line(&r, &theta)
        .color(Color::from_hex("#f97316").unwrap())
        .inset_anchor(InsetAnchor::TopRight)
        .inset_size_frac(0.32, 0.32)
        .inset_margin_pt(12.0)
        .fill(true)
        .fill_alpha(0.24)
        .into()
}

fn mixed_pie_plot() -> Plot {
    let (x, y) = generate_xy_data(180);

    Plot::new()
        .title("Mixed Cartesian + Pie Inset")
        .xlabel("Quarter")
        .ylabel("Trend")
        .size(6.4, 4.8)
        .dpi(100)
        .line(&x, &y)
        .color(Color::from_hex("#0f766e").unwrap())
        .grid(true)
        .pie(&[4.0, 3.0, 2.0])
        .labels(&["North", "West", "Online"])
        .show_percentages(true)
        .inset_anchor(InsetAnchor::BottomRight)
        .inset_size_frac(0.34, 0.34)
        .inset_margin_pt(14.0)
        .into()
}

fn mixed_radar_plot() -> Plot {
    let (x, y) = generate_xy_data(180);

    Plot::new()
        .title("Mixed Cartesian + Radar Inset")
        .xlabel("Time")
        .ylabel("Signal")
        .size(6.4, 4.8)
        .dpi(100)
        .line(&x, &y)
        .color(Color::from_hex("#4338ca").unwrap())
        .grid(true)
        .radar(&["Speed", "Power", "Skill", "Range", "Focus"])
        .add_series("Alpha", &[4.0, 3.0, 4.5, 3.5, 4.0])
        .with_color(Color::from_hex("#1d4ed8").unwrap())
        .with_fill_alpha(0.28)
        .add_series("Beta", &[2.8, 4.2, 3.2, 4.0, 3.6])
        .with_color(Color::from_hex("#ea580c").unwrap())
        .with_fill_alpha(0.20)
        .inset_anchor(InsetAnchor::TopLeft)
        .inset_size_frac(0.36, 0.36)
        .inset_margin_pt(12.0)
        .into()
}

fn save_plot(name: &str, plot: Plot, outputs: &mut Vec<PathBuf>) -> Result<()> {
    let png_path = util::example_output_path_in("mixed_coordinate_insets", &format!("{name}.png"));
    let svg_path = util::example_output_path_in("mixed_coordinate_insets", &format!("{name}.svg"));

    plot.clone().save(png_path.to_string_lossy().as_ref())?;
    fs::write(&svg_path, plot.render_to_svg()?)?;

    outputs.push(png_path);
    outputs.push(svg_path);
    Ok(())
}

fn main() -> Result<()> {
    let mut outputs = Vec::new();

    save_plot("mixed_polar_inset", mixed_polar_plot(), &mut outputs)?;
    save_plot("mixed_pie_inset", mixed_pie_plot(), &mut outputs)?;
    save_plot("mixed_radar_inset", mixed_radar_plot(), &mut outputs)?;

    println!("Generated files:");
    for path in outputs {
        println!("  {}", path.display());
    }

    Ok(())
}
