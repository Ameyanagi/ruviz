//! Documentation example: ECDF Plot
//!
//! Generates docs/images/ecdf_plot.png for rustdoc

use ruviz::plots::distribution::{EcdfConfig, compute_ecdf};
use ruviz::prelude::*;
use ruviz::render::SkiaRenderer;

fn main() -> Result<()> {
    let width = 800;
    let height = 500;
    let theme = Theme::default();

    let mut renderer = SkiaRenderer::new(width, height, theme.clone())?;
    renderer.clear();

    // Generate two datasets
    let data1: Vec<f64> = (0..100)
        .map(|i| {
            let u1 = ((i * 7 + 13) % 100) as f64 / 100.0;
            let u2 = ((i * 11 + 17) % 100) as f64 / 100.0;
            5.0 + 2.0 * (-2.0 * u1.max(0.01).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        })
        .collect();

    let data2: Vec<f64> = (0..100)
        .map(|i| {
            let u1 = ((i * 13 + 7) % 100) as f64 / 100.0;
            let u2 = ((i * 17 + 11) % 100) as f64 / 100.0;
            7.0 + 1.5 * (-2.0 * u1.max(0.01).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        })
        .collect();

    let colors = Color::default_palette();

    // Compute ECDFs
    let config = EcdfConfig::new();
    let ecdf1 = compute_ecdf(&data1, &config);
    let ecdf2 = compute_ecdf(&data2, &config);

    // Plot area
    let margin = 70.0_f32;
    let plot_width = width as f32 - 2.0 * margin;
    let plot_height = height as f32 - 2.0 * margin;

    let x_min = 0.0_f64;
    let x_max = 12.0_f64;
    let y_min = 0.0_f64;
    let y_max = 1.0_f64;

    let to_screen = |x: f64, y: f64| -> (f32, f32) {
        let sx = margin + plot_width * ((x - x_min) / (x_max - x_min)) as f32;
        let sy = margin + plot_height * (1.0 - ((y - y_min) / (y_max - y_min)) as f32);
        (sx, sy)
    };

    // Draw axes
    renderer.draw_rectangle(
        margin,
        margin,
        plot_width,
        plot_height,
        theme.grid_color,
        false,
    )?;

    // Grid
    for i in 0..=5 {
        let y = margin + plot_height * i as f32 / 5.0;
        renderer.draw_line(
            margin,
            y,
            margin + plot_width,
            y,
            theme.grid_color,
            0.5,
            LineStyle::Solid,
        )?;
        let val = y_max - (y_max - y_min) * i as f64 / 5.0;
        renderer.draw_text(
            &format!("{:.1}", val),
            margin - 30.0,
            y,
            10.0,
            theme.foreground,
        )?;
    }
    for i in 0..=6 {
        let x = margin + plot_width * i as f32 / 6.0;
        renderer.draw_line(
            x,
            margin,
            x,
            margin + plot_height,
            theme.grid_color,
            0.5,
            LineStyle::Solid,
        )?;
        let val = x_min + (x_max - x_min) * i as f64 / 6.0;
        renderer.draw_text_centered(
            &format!("{:.0}", val),
            x,
            margin + plot_height + 15.0,
            10.0,
            theme.foreground,
        )?;
    }

    // Draw ECDF 1 as step function
    if !ecdf1.step_vertices.is_empty() {
        let mut prev = to_screen(
            ecdf1.step_vertices[0].0.max(x_min),
            ecdf1.step_vertices[0].1,
        );
        for &(x, y) in ecdf1.step_vertices.iter().skip(1) {
            if x >= x_min && x <= x_max {
                let curr = to_screen(x, y);
                renderer.draw_line(
                    prev.0,
                    prev.1,
                    curr.0,
                    curr.1,
                    colors[0],
                    2.0,
                    LineStyle::Solid,
                )?;
                prev = curr;
            }
        }
        // Extend to x_max
        let (last_x, last_y) = to_screen(x_max, 1.0);
        renderer.draw_line(
            prev.0,
            prev.1,
            last_x,
            last_y,
            colors[0],
            2.0,
            LineStyle::Solid,
        )?;
    }

    // Draw ECDF 2 as step function
    if !ecdf2.step_vertices.is_empty() {
        let mut prev = to_screen(
            ecdf2.step_vertices[0].0.max(x_min),
            ecdf2.step_vertices[0].1,
        );
        for &(x, y) in ecdf2.step_vertices.iter().skip(1) {
            if x >= x_min && x <= x_max {
                let curr = to_screen(x, y);
                renderer.draw_line(
                    prev.0,
                    prev.1,
                    curr.0,
                    curr.1,
                    colors[1],
                    2.0,
                    LineStyle::Solid,
                )?;
                prev = curr;
            }
        }
        let (last_x, last_y) = to_screen(x_max, 1.0);
        renderer.draw_line(
            prev.0,
            prev.1,
            last_x,
            last_y,
            colors[1],
            2.0,
            LineStyle::Solid,
        )?;
    }

    // Title and labels
    renderer.draw_text_centered(
        "Empirical Cumulative Distribution Function",
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
    renderer.draw_text_rotated(
        "Proportion",
        20.0,
        height as f32 / 2.0,
        12.0,
        theme.foreground,
    )?;

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
        "Sample A",
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
        "Sample B",
        width as f32 - 115.0,
        70.0,
        10.0,
        theme.foreground,
    )?;

    renderer.save_png("docs/images/ecdf_plot.png")?;
    println!("Generated docs/images/ecdf_plot.png");
    Ok(())
}
