//! Documentation example: Polar Plot
//!
//! Generates docs/images/polar_plot.png for rustdoc

use ruviz::plots::polar::polar_plot::{PolarPlotConfig, compute_polar_plot};
use ruviz::prelude::*;
use ruviz::render::SkiaRenderer;

fn main() -> Result<()> {
    let width = 600;
    let height = 600;
    let theme = Theme::default();

    let mut renderer = SkiaRenderer::new(width, height, theme.clone())?;
    renderer.clear();

    // Generate polar data (rose curve)
    let n_points = 200;
    let theta1: Vec<f64> = (0..n_points)
        .map(|i| i as f64 * 2.0 * std::f64::consts::PI / n_points as f64)
        .collect();
    let r1: Vec<f64> = theta1.iter().map(|&t| (3.0 * t).cos().abs()).collect();

    // Also a spiral
    let theta2: Vec<f64> = (0..n_points)
        .map(|i| i as f64 * 4.0 * std::f64::consts::PI / n_points as f64)
        .collect();
    let r2: Vec<f64> = theta2
        .iter()
        .map(|&t| 0.1 + t / (4.0 * std::f64::consts::PI))
        .collect();

    let colors = Color::default_palette();

    // Compute polar data
    let config = PolarPlotConfig::default();
    let polar1 = compute_polar_plot(&r1, &theta1, &config);
    let polar2 = compute_polar_plot(&r2, &theta2, &config);

    // Plot area (square, centered)
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0 + 20.0; // Offset for title
    let radius = 220.0_f32;

    // Draw grid circles
    for i in 1..=4 {
        let r = radius * i as f32 / 4.0;
        // Draw circle as polygon
        let mut circle: Vec<(f32, f32)> = Vec::new();
        for j in 0..=60 {
            let angle = j as f64 * 2.0 * std::f64::consts::PI / 60.0;
            circle.push((
                center_x + r * angle.cos() as f32,
                center_y - r * angle.sin() as f32,
            ));
        }
        for k in 0..circle.len() - 1 {
            renderer.draw_line(
                circle[k].0,
                circle[k].1,
                circle[k + 1].0,
                circle[k + 1].1,
                theme.grid_color,
                0.5,
                LineStyle::Solid,
            )?;
        }
    }

    // Draw radial lines
    for i in 0..12 {
        let angle = i as f64 * std::f64::consts::PI / 6.0;
        let x2 = center_x + radius * angle.cos() as f32;
        let y2 = center_y - radius * angle.sin() as f32;
        renderer.draw_line(
            center_x,
            center_y,
            x2,
            y2,
            theme.grid_color,
            0.5,
            LineStyle::Solid,
        )?;
    }

    // Draw polar data (rose)
    let max_r = r1.iter().chain(r2.iter()).fold(0.0_f64, |a, &b| a.max(b));

    if polar1.points.len() >= 2 {
        let mut prev: Option<(f32, f32)> = None;
        for point in &polar1.points {
            let screen_x = center_x + point.x as f32 / max_r as f32 * radius;
            let screen_y = center_y - point.y as f32 / max_r as f32 * radius;

            if let Some((prev_x, prev_y)) = prev {
                renderer.draw_line(
                    prev_x,
                    prev_y,
                    screen_x,
                    screen_y,
                    colors[0],
                    2.0,
                    LineStyle::Solid,
                )?;
            }
            prev = Some((screen_x, screen_y));
        }
    }

    // Draw polar data (spiral)
    if polar2.points.len() >= 2 {
        let mut prev: Option<(f32, f32)> = None;
        for point in &polar2.points {
            let screen_x = center_x + point.x as f32 / max_r as f32 * radius;
            let screen_y = center_y - point.y as f32 / max_r as f32 * radius;

            if let Some((prev_x, prev_y)) = prev {
                renderer.draw_line(
                    prev_x,
                    prev_y,
                    screen_x,
                    screen_y,
                    colors[1],
                    2.0,
                    LineStyle::Solid,
                )?;
            }
            prev = Some((screen_x, screen_y));
        }
    }

    // Title
    renderer.draw_text_centered(
        "Polar Plot",
        width as f32 / 2.0,
        25.0,
        18.0,
        theme.foreground,
    )?;

    // Legend
    renderer.draw_line(
        width as f32 - 130.0,
        50.0,
        width as f32 - 100.0,
        50.0,
        colors[0],
        2.0,
        LineStyle::Solid,
    )?;
    renderer.draw_text("Rose", width as f32 - 95.0, 50.0, 10.0, theme.foreground)?;
    renderer.draw_line(
        width as f32 - 130.0,
        70.0,
        width as f32 - 100.0,
        70.0,
        colors[1],
        2.0,
        LineStyle::Solid,
    )?;
    renderer.draw_text("Spiral", width as f32 - 95.0, 70.0, 10.0, theme.foreground)?;

    renderer.save_png("docs/images/polar_plot.png")?;
    println!("Generated docs/images/polar_plot.png");
    Ok(())
}
