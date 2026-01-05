//! Documentation example: Error Bars
//!
//! Generates docs/images/errorbar_plot.png for rustdoc

use ruviz::plots::error::{ErrorValues, compute_error_bars};
use ruviz::prelude::*;
use ruviz::render::SkiaRenderer;

fn main() -> Result<()> {
    let width = 800;
    let height = 500;
    let theme = Theme::default();

    let mut renderer = SkiaRenderer::new(width, height, theme.clone())?;
    renderer.clear();

    // Sample data with errors
    let x: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let y: Vec<f64> = vec![2.3, 3.5, 4.1, 5.8, 6.2, 7.5, 8.1];
    let y_err: Vec<f64> = vec![0.3, 0.4, 0.25, 0.5, 0.35, 0.4, 0.45];

    // Asymmetric errors for a second dataset
    let y2: Vec<f64> = vec![1.8, 2.9, 3.5, 4.2, 5.5, 6.0, 7.2];
    let y2_lower: Vec<f64> = vec![0.2, 0.3, 0.2, 0.4, 0.3, 0.35, 0.4];
    let y2_upper: Vec<f64> = vec![0.4, 0.5, 0.35, 0.6, 0.5, 0.55, 0.6];

    let colors = Color::default_palette();

    // Plot area
    let margin = 70.0_f32;
    let plot_width = width as f32 - 2.0 * margin;
    let plot_height = height as f32 - 2.0 * margin;

    let x_min = 0.0_f64;
    let x_max = 8.0_f64;
    let y_min = 0.0_f64;
    let y_max = 10.0_f64;

    let to_screen = |px: f64, py: f64| -> (f32, f32) {
        let sx = margin + plot_width * ((px - x_min) / (x_max - x_min)) as f32;
        let sy = margin + plot_height * (1.0 - ((py - y_min) / (y_max - y_min)) as f32);
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
            &format!("{:.0}", val),
            margin - 25.0,
            y,
            10.0,
            theme.foreground,
        )?;
    }
    for i in 0..=8 {
        let x_val = margin + plot_width * i as f32 / 8.0;
        renderer.draw_text_centered(
            &format!("{}", i),
            x_val,
            margin + plot_height + 15.0,
            10.0,
            theme.foreground,
        )?;
    }

    // Compute error bars
    let errors1 = ErrorValues::Symmetric(y_err.clone());
    let error_bars1 = compute_error_bars(&x, &y, Some(&errors1), None);

    let errors2 = ErrorValues::Asymmetric(y2_lower.clone(), y2_upper.clone());
    let error_bars2 = compute_error_bars(&x, &y2, Some(&errors2), None);

    let cap_size = 6.0_f32;

    // Draw error bars for dataset 1
    for bar in &error_bars1 {
        let (cx, cy) = to_screen(bar.x, bar.y);
        let (_, y_low) = to_screen(bar.x, bar.y_lower);
        let (_, y_high) = to_screen(bar.x, bar.y_upper);
        let cap_half = cap_size / 2.0;

        // Vertical line
        renderer.draw_line(cx, y_low, cx, y_high, colors[0], 1.5, LineStyle::Solid)?;
        // Top cap
        renderer.draw_line(
            cx - cap_half,
            y_high,
            cx + cap_half,
            y_high,
            colors[0],
            1.5,
            LineStyle::Solid,
        )?;
        // Bottom cap
        renderer.draw_line(
            cx - cap_half,
            y_low,
            cx + cap_half,
            y_low,
            colors[0],
            1.5,
            LineStyle::Solid,
        )?;
        // Data point
        renderer.draw_circle(cx, cy, 5.0, colors[0], true)?;
    }

    // Draw error bars for dataset 2
    for bar in &error_bars2 {
        let (cx, cy) = to_screen(bar.x, bar.y);
        let (_, y_low) = to_screen(bar.x, bar.y_lower);
        let (_, y_high) = to_screen(bar.x, bar.y_upper);
        let cap_half = cap_size / 2.0;

        renderer.draw_line(cx, y_low, cx, y_high, colors[1], 1.5, LineStyle::Solid)?;
        renderer.draw_line(
            cx - cap_half,
            y_high,
            cx + cap_half,
            y_high,
            colors[1],
            1.5,
            LineStyle::Solid,
        )?;
        renderer.draw_line(
            cx - cap_half,
            y_low,
            cx + cap_half,
            y_low,
            colors[1],
            1.5,
            LineStyle::Solid,
        )?;
        renderer.draw_marker(cx, cy, 10.0, MarkerStyle::Square, colors[1])?;
    }

    // Connect points with lines
    for i in 0..x.len() - 1 {
        let (x1, y1_s) = to_screen(x[i], y[i]);
        let (x2_s, y2_s) = to_screen(x[i + 1], y[i + 1]);
        renderer.draw_line(
            x1,
            y1_s,
            x2_s,
            y2_s,
            colors[0].with_alpha(0.5),
            1.0,
            LineStyle::Solid,
        )?;

        let (x1, y1_s) = to_screen(x[i], y2[i]);
        let (x2_s, y2_s) = to_screen(x[i + 1], y2[i + 1]);
        renderer.draw_line(
            x1,
            y1_s,
            x2_s,
            y2_s,
            colors[1].with_alpha(0.5),
            1.0,
            LineStyle::Solid,
        )?;
    }

    // Title and labels
    renderer.draw_text_centered(
        "Error Bar Plot",
        width as f32 / 2.0,
        25.0,
        16.0,
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

    // Legend
    renderer.draw_circle(width as f32 - 145.0, 50.0, 5.0, colors[0], true)?;
    renderer.draw_text(
        "Symmetric Error",
        width as f32 - 135.0,
        50.0,
        10.0,
        theme.foreground,
    )?;
    renderer.draw_marker(
        width as f32 - 145.0,
        70.0,
        10.0,
        MarkerStyle::Square,
        colors[1],
    )?;
    renderer.draw_text(
        "Asymmetric Error",
        width as f32 - 135.0,
        70.0,
        10.0,
        theme.foreground,
    )?;

    renderer.save_png("docs/images/errorbar_plot.png")?;
    println!("Generated docs/images/errorbar_plot.png");
    Ok(())
}
