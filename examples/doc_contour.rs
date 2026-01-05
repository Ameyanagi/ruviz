//! Documentation example: Contour Plot
//!
//! Generates docs/images/contour_plot.png for rustdoc

use ruviz::plots::continuous::contour::{ContourConfig, compute_contour_plot};
use ruviz::prelude::*;
use ruviz::render::SkiaRenderer;

fn main() -> Result<()> {
    let width = 800;
    let height = 600;
    let theme = Theme::default();

    let mut renderer = SkiaRenderer::new(width, height, theme.clone())?;
    renderer.clear();

    // Generate 2D data (Gaussian surface)
    let size = 50;
    let x: Vec<f64> = (0..size).map(|i| (i as f64 - 25.0) / 5.0).collect();
    let y: Vec<f64> = (0..size).map(|i| (i as f64 - 25.0) / 5.0).collect();

    // Z as flat array (row-major)
    let z: Vec<f64> = (0..size)
        .flat_map(|j| {
            (0..size).map(move |i| {
                let xi = (i as f64 - 25.0) / 5.0;
                let yj = (j as f64 - 25.0) / 5.0;
                (-xi * xi - yj * yj).exp()
            })
        })
        .collect();

    // Compute contour with explicit levels
    let levels: Vec<f64> = (0..10).map(|i| i as f64 / 10.0 + 0.05).collect();
    let config = ContourConfig::default().levels(levels.clone()).filled(true);
    let contour = compute_contour_plot(&x, &y, &z, &config);

    // Plot area
    let margin = 70.0_f32;
    let plot_width = width as f32 - 2.0 * margin;
    let plot_height = height as f32 - 2.0 * margin;

    let x_min = -5.0_f64;
    let x_max = 5.0_f64;
    let y_min = -5.0_f64;
    let y_max = 5.0_f64;

    let to_screen = |xv: f64, yv: f64| -> (f32, f32) {
        let sx = margin + plot_width * ((xv - x_min) / (x_max - x_min)) as f32;
        let sy = margin + plot_height * (1.0 - ((yv - y_min) / (y_max - y_min)) as f32);
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
        let sx = margin + plot_width * i as f32 / 5.0;
        renderer.draw_line(
            sx,
            margin,
            sx,
            margin + plot_height,
            theme.grid_color,
            0.5,
            LineStyle::Solid,
        )?;
        let val = x_min + (x_max - x_min) * i as f64 / 5.0;
        renderer.draw_text_centered(
            &format!("{:.0}", val),
            sx,
            margin + plot_height + 15.0,
            10.0,
            theme.foreground,
        )?;

        let sy = margin + plot_height * i as f32 / 5.0;
        renderer.draw_line(
            margin,
            sy,
            margin + plot_width,
            sy,
            theme.grid_color,
            0.5,
            LineStyle::Solid,
        )?;
        let val = y_max - (y_max - y_min) * i as f64 / 5.0;
        renderer.draw_text(
            &format!("{:.0}", val),
            margin - 25.0,
            sy,
            10.0,
            theme.foreground,
        )?;
    }

    // Draw contour lines
    let colormap = ColorMap::viridis();
    let z_min = 0.0_f64;
    let z_max = 1.0_f64;

    for contour_level in &contour.lines {
        let t = ((contour_level.level - z_min) / (z_max - z_min)).clamp(0.0, 1.0);
        let color = colormap.sample(t);

        // Draw segments
        for &(x1, y1, x2, y2) in &contour_level.segments {
            let (sx1, sy1) = to_screen(x1, y1);
            let (sx2, sy2) = to_screen(x2, y2);
            renderer.draw_line(sx1, sy1, sx2, sy2, color, 1.5, LineStyle::Solid)?;
        }
    }

    // Title and labels
    renderer.draw_text_centered(
        "Contour Plot",
        width as f32 / 2.0,
        25.0,
        18.0,
        theme.foreground,
    )?;
    renderer.draw_text_centered(
        "X",
        width as f32 / 2.0,
        height as f32 - 15.0,
        12.0,
        theme.foreground,
    )?;
    renderer.draw_text_rotated("Y", 20.0, height as f32 / 2.0, 12.0, theme.foreground)?;

    // Colorbar
    let cb_x = width as f32 - 50.0;
    let cb_y = margin;
    let cb_height = plot_height;
    let cb_width = 15.0;
    for i in 0..50 {
        let t = i as f64 / 50.0;
        let color = colormap.sample(1.0 - t);
        renderer.draw_rectangle(
            cb_x,
            cb_y + cb_height * t as f32,
            cb_width,
            cb_height / 50.0 + 1.0,
            color,
            true,
        )?;
    }
    renderer.draw_rectangle(cb_x, cb_y, cb_width, cb_height, theme.foreground, false)?;

    renderer.save_png("docs/images/contour_plot.png")?;
    println!("Generated docs/images/contour_plot.png");
    Ok(())
}
