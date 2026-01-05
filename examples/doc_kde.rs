//! Documentation example: KDE Plot
//!
//! Generates docs/images/kde_plot.png for rustdoc

use ruviz::plots::distribution::{KdePlotConfig, compute_kde_plot};
use ruviz::prelude::*;
use ruviz::render::SkiaRenderer;

fn main() -> Result<()> {
    let width = 800;
    let height = 500;
    let theme = Theme::default();

    let mut renderer = SkiaRenderer::new(width, height, theme.clone())?;
    renderer.clear();

    // Generate two datasets with different distributions
    let data1: Vec<f64> = (0..200)
        .map(|i| {
            let u1 = ((i * 7 + 13) % 200) as f64 / 200.0;
            let u2 = ((i * 11 + 17) % 200) as f64 / 200.0;
            3.0 + 1.0 * (-2.0 * u1.max(0.01).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        })
        .collect();

    let data2: Vec<f64> = (0..200)
        .map(|i| {
            let u1 = ((i * 13 + 7) % 200) as f64 / 200.0;
            let u2 = ((i * 17 + 11) % 200) as f64 / 200.0;
            6.0 + 1.5 * (-2.0 * u1.max(0.01).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        })
        .collect();

    let colors = Color::default_palette();

    // Compute KDE
    let config = KdePlotConfig::new().n_points(100);
    let kde1 = compute_kde_plot(&data1, &config);
    let kde2 = compute_kde_plot(&data2, &config);

    // Plot area
    let margin = 70.0_f32;
    let plot_width = width as f32 - 2.0 * margin;
    let plot_height = height as f32 - 2.0 * margin;

    // Data ranges
    let x_min = 0.0_f64;
    let x_max = 10.0_f64;
    let y_max = kde1
        .y
        .iter()
        .chain(kde2.y.iter())
        .fold(0.0_f64, |a, &b| a.max(b))
        * 1.1;

    // Helper to convert data to screen coords
    let to_screen = |x: f64, y: f64| -> (f32, f32) {
        let sx = margin + plot_width * ((x - x_min) / (x_max - x_min)) as f32;
        let sy = margin + plot_height * (1.0 - (y / y_max) as f32);
        (sx, sy)
    };

    // Draw axes box
    renderer.draw_rectangle(
        margin,
        margin,
        plot_width,
        plot_height,
        theme.grid_color,
        false,
    )?;

    // Draw grid
    for i in 0..=5 {
        let x = margin + plot_width * i as f32 / 5.0;
        renderer.draw_line(
            x,
            margin,
            x,
            margin + plot_height,
            theme.grid_color,
            0.5,
            LineStyle::Solid,
        )?;
        let val = x_min + (x_max - x_min) * i as f64 / 5.0;
        renderer.draw_text_centered(
            &format!("{:.0}", val),
            x,
            margin + plot_height + 20.0,
            10.0,
            theme.foreground,
        )?;
    }

    // Draw KDE 1 - filled area
    let mut poly1: Vec<(f32, f32)> = Vec::new();
    let (start_x, _) = to_screen(kde1.x[0], 0.0);
    poly1.push((start_x, margin + plot_height));
    for (&x, &y) in kde1.x.iter().zip(kde1.y.iter()) {
        if x >= x_min && x <= x_max {
            poly1.push(to_screen(x, y));
        }
    }
    let (end_x, _) = to_screen(*kde1.x.last().unwrap_or(&x_max), 0.0);
    poly1.push((end_x, margin + plot_height));
    renderer.draw_filled_polygon(&poly1, colors[0].with_alpha(0.4))?;

    // Draw KDE 2 - filled area
    let mut poly2: Vec<(f32, f32)> = Vec::new();
    let (start_x, _) = to_screen(kde2.x[0], 0.0);
    poly2.push((start_x, margin + plot_height));
    for (&x, &y) in kde2.x.iter().zip(kde2.y.iter()) {
        if x >= x_min && x <= x_max {
            poly2.push(to_screen(x, y));
        }
    }
    let (end_x, _) = to_screen(*kde2.x.last().unwrap_or(&x_max), 0.0);
    poly2.push((end_x, margin + plot_height));
    renderer.draw_filled_polygon(&poly2, colors[1].with_alpha(0.4))?;

    // Draw KDE lines on top
    let mut prev1 = to_screen(kde1.x[0], kde1.y[0]);
    for (&x, &y) in kde1.x.iter().zip(kde1.y.iter()).skip(1) {
        if x >= x_min && x <= x_max {
            let curr = to_screen(x, y);
            renderer.draw_line(
                prev1.0,
                prev1.1,
                curr.0,
                curr.1,
                colors[0],
                2.0,
                LineStyle::Solid,
            )?;
            prev1 = curr;
        }
    }

    let mut prev2 = to_screen(kde2.x[0], kde2.y[0]);
    for (&x, &y) in kde2.x.iter().zip(kde2.y.iter()).skip(1) {
        if x >= x_min && x <= x_max {
            let curr = to_screen(x, y);
            renderer.draw_line(
                prev2.0,
                prev2.1,
                curr.0,
                curr.1,
                colors[1],
                2.0,
                LineStyle::Solid,
            )?;
            prev2 = curr;
        }
    }

    // Draw title and labels
    renderer.draw_text_centered(
        "Kernel Density Estimation",
        width as f32 / 2.0,
        25.0,
        16.0,
        theme.foreground,
    )?;
    renderer.draw_text_centered(
        "Value",
        width as f32 / 2.0,
        height as f32 - 15.0,
        12.0,
        theme.foreground,
    )?;
    renderer.draw_text_rotated("Density", 20.0, height as f32 / 2.0, 12.0, theme.foreground)?;

    // Legend
    renderer.draw_line(
        width as f32 - 150.0,
        50.0,
        width as f32 - 120.0,
        50.0,
        colors[0],
        2.0,
        LineStyle::Solid,
    )?;
    renderer.draw_text(
        "Distribution A",
        width as f32 - 115.0,
        50.0,
        10.0,
        theme.foreground,
    )?;
    renderer.draw_line(
        width as f32 - 150.0,
        70.0,
        width as f32 - 120.0,
        70.0,
        colors[1],
        2.0,
        LineStyle::Solid,
    )?;
    renderer.draw_text(
        "Distribution B",
        width as f32 - 115.0,
        70.0,
        10.0,
        theme.foreground,
    )?;

    renderer.save_png("docs/images/kde_plot.png")?;
    println!("Generated docs/images/kde_plot.png");
    Ok(())
}
