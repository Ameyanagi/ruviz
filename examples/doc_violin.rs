//! Documentation example: Violin Plot
//!
//! Generates docs/images/violin_plot.png for rustdoc

use ruviz::plots::distribution::violin::{ViolinConfig, ViolinData};
use ruviz::prelude::*;
use ruviz::render::SkiaRenderer;

fn main() -> Result<()> {
    let width = 800;
    let height = 500;
    let theme = Theme::default();

    let mut renderer = SkiaRenderer::new(width, height, theme.clone())?;
    renderer.clear();

    // Generate sample data (bimodal distribution)
    let data1: Vec<f64> = (0..200)
        .map(|i| {
            let u1 = ((i * 7 + 13) % 200) as f64 / 200.0;
            let u2 = ((i * 11 + 17) % 200) as f64 / 200.0;
            let normal =
                (-2.0 * u1.max(0.01).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            if i % 2 == 0 {
                3.0 + normal
            } else {
                7.0 + normal * 0.8
            }
        })
        .collect();

    let data2: Vec<f64> = (0..200)
        .map(|i| {
            let u1 = ((i * 13 + 7) % 200) as f64 / 200.0;
            let u2 = ((i * 17 + 11) % 200) as f64 / 200.0;
            5.0 + 2.0 * (-2.0 * u1.max(0.01).ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        })
        .collect();

    let colors = Color::default_palette();

    // Compute violin data
    let config = ViolinConfig::default();
    let violin1 = ViolinData::from_values(&data1, &config);
    let violin2 = ViolinData::from_values(&data2, &config);

    // Plot area
    let margin = 70.0_f32;
    let plot_width = width as f32 - 2.0 * margin;
    let plot_height = height as f32 - 2.0 * margin;

    let y_min = 0.0_f64;
    let y_max = 12.0_f64;

    let to_screen_y =
        |y: f64| -> f32 { margin + plot_height * (1.0 - ((y - y_min) / (y_max - y_min)) as f32) };

    // Draw axes
    renderer.draw_rectangle(
        margin,
        margin,
        plot_width,
        plot_height,
        theme.grid_color,
        false,
    )?;

    // Y-axis grid
    for i in 0..=6 {
        let y = margin + plot_height * i as f32 / 6.0;
        renderer.draw_line(
            margin,
            y,
            margin + plot_width,
            y,
            theme.grid_color,
            0.5,
            LineStyle::Solid,
        )?;
        let val = y_max - (y_max - y_min) * i as f64 / 6.0;
        renderer.draw_text(
            &format!("{:.0}", val),
            margin - 25.0,
            y,
            10.0,
            theme.foreground,
        )?;
    }

    // Draw violins
    let violin_width = 80.0_f32;
    let positions = [margin + plot_width * 0.25, margin + plot_width * 0.75];
    let violins = [&violin1, &violin2];
    let labels = ["Group A", "Group B"];

    for (idx, (violin_opt, &center_x)) in violins.iter().zip(positions.iter()).enumerate() {
        if let Some(violin) = violin_opt {
            let color = colors[idx];

            // Draw KDE shape using kde.x and kde.density
            let max_density = violin.kde.density.iter().fold(0.0_f64, |a, &b| a.max(b));

            let mut left_polygon: Vec<(f32, f32)> = Vec::new();
            let mut right_polygon: Vec<(f32, f32)> = Vec::new();

            for (&y, &d) in violin.kde.x.iter().zip(violin.kde.density.iter()) {
                if y >= y_min && y <= y_max {
                    let screen_y = to_screen_y(y);
                    let half_width = (d / max_density) as f32 * violin_width * 0.5;
                    left_polygon.push((center_x - half_width, screen_y));
                    right_polygon.push((center_x + half_width, screen_y));
                }
            }

            // Combine into full polygon (left side going down, right side going up)
            let mut polygon: Vec<(f32, f32)> = left_polygon.clone();
            right_polygon.reverse();
            polygon.extend(right_polygon);

            if polygon.len() >= 3 {
                renderer.draw_filled_polygon(&polygon, color.with_alpha(0.6))?;
                // Draw outline
                for i in 0..polygon.len() {
                    let (x1, y1) = polygon[i];
                    let (x2, y2) = polygon[(i + 1) % polygon.len()];
                    renderer.draw_line(x1, y1, x2, y2, color, 1.5, LineStyle::Solid)?;
                }
            }

            // Draw box plot elements inside using quartiles tuple
            let (q1, median, q3) = violin.quartiles;
            let box_width = 12.0_f32;

            // Box (Q1 to Q3)
            let q1_y = to_screen_y(q1);
            let q3_y = to_screen_y(q3);
            renderer.draw_rectangle(
                center_x - box_width / 2.0,
                q3_y,
                box_width,
                q1_y - q3_y,
                color,
                true,
            )?;
            renderer.draw_rectangle(
                center_x - box_width / 2.0,
                q3_y,
                box_width,
                q1_y - q3_y,
                Color::WHITE,
                false,
            )?;

            // Median line
            let median_y = to_screen_y(median);
            renderer.draw_line(
                center_x - box_width / 2.0,
                median_y,
                center_x + box_width / 2.0,
                median_y,
                Color::WHITE,
                2.0,
                LineStyle::Solid,
            )?;

            // Label
            renderer.draw_text_centered(
                labels[idx],
                center_x,
                margin + plot_height + 25.0,
                12.0,
                theme.foreground,
            )?;
        }
    }

    // Title and labels
    renderer.draw_text_centered(
        "Violin Plot",
        width as f32 / 2.0,
        25.0,
        18.0,
        theme.foreground,
    )?;
    renderer.draw_text_centered(
        "Group",
        width as f32 / 2.0,
        height as f32 - 15.0,
        12.0,
        theme.foreground,
    )?;
    renderer.draw_text_rotated("Value", 20.0, height as f32 / 2.0, 12.0, theme.foreground)?;

    renderer.save_png("docs/images/violin_plot.png")?;
    println!("Generated docs/images/violin_plot.png");
    Ok(())
}
