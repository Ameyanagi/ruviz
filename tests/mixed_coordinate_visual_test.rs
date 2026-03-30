//! Manual visual output tests for mixed Cartesian/non-Cartesian inset rendering.
//!
//! Run with:
//! `cargo test --test mixed_coordinate_visual_test -- --ignored --nocapture`

mod common;

use common::{assert_file_non_empty, assert_png_rendered, test_output_path_in};
use ruviz::prelude::*;
use std::f64::consts::PI;
use std::fs;
use std::path::PathBuf;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

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

fn save_visual_artifacts(
    name: &str,
    plot: Plot,
) -> std::result::Result<(PathBuf, PathBuf), Box<dyn std::error::Error>> {
    let png_path = test_output_path_in("mixed_coordinate_insets", &format!("{name}.png"));
    let svg_path = test_output_path_in("mixed_coordinate_insets", &format!("{name}.svg"));

    plot.clone().save(&png_path)?;
    fs::write(&svg_path, plot.render_to_svg()?)?;

    Ok((png_path, svg_path))
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

#[test]
#[ignore = "Manual visual output test - run with: cargo test --test mixed_coordinate_visual_test -- --ignored --nocapture"]
fn test_mixed_polar_inset_outputs() -> TestResult {
    let (png_path, svg_path) = save_visual_artifacts("mixed_polar_inset", mixed_polar_plot())?;
    assert_png_rendered(&png_path, Some((640, 480)));
    assert_file_non_empty(&svg_path);

    let svg = fs::read_to_string(&svg_path)?;
    assert!(svg.matches("<polyline").count() >= 2);
    assert!(svg.contains("0°"));

    println!("Saved {}", png_path.display());
    println!("Saved {}", svg_path.display());
    Ok(())
}

#[test]
#[ignore = "Manual visual output test - run with: cargo test --test mixed_coordinate_visual_test -- --ignored --nocapture"]
fn test_mixed_pie_inset_outputs() -> TestResult {
    let (png_path, svg_path) = save_visual_artifacts("mixed_pie_inset", mixed_pie_plot())?;
    assert_png_rendered(&png_path, Some((640, 480)));
    assert_file_non_empty(&svg_path);

    let svg = fs::read_to_string(&svg_path)?;
    assert!(svg.matches("<polygon").count() >= 3);
    assert!(svg.contains("44.4%"));

    println!("Saved {}", png_path.display());
    println!("Saved {}", svg_path.display());
    Ok(())
}

#[test]
#[ignore = "Manual visual output test - run with: cargo test --test mixed_coordinate_visual_test -- --ignored --nocapture"]
fn test_mixed_radar_inset_outputs() -> TestResult {
    let (png_path, svg_path) = save_visual_artifacts("mixed_radar_inset", mixed_radar_plot())?;
    assert_png_rendered(&png_path, Some((640, 480)));
    assert_file_non_empty(&svg_path);

    let svg = fs::read_to_string(&svg_path)?;
    assert!(svg.matches("<polygon").count() >= 2);
    assert!(svg.contains(">Speed<"));

    println!("Saved {}", png_path.display());
    println!("Saved {}", svg_path.display());
    Ok(())
}
