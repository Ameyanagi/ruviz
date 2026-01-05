//! Documentation example: Radar Chart
//!
//! Generates docs/images/radar_chart.png for rustdoc

use ruviz::plots::polar::radar::{RadarConfig, compute_radar_chart};
use ruviz::prelude::*;
use ruviz::render::SkiaRenderer;

fn main() -> Result<()> {
    let width = 600;
    let height = 600;
    let theme = Theme::default();

    let mut renderer = SkiaRenderer::new(width, height, theme.clone())?;
    renderer.clear();

    // Create data for two players/items
    let data = vec![
        vec![85.0, 92.0, 78.0, 65.0, 88.0], // Player 1
        vec![72.0, 68.0, 95.0, 82.0, 75.0], // Player 2
    ];

    let config = RadarConfig::default().labels(vec![
        "Speed".to_string(),
        "Power".to_string(),
        "Defense".to_string(),
        "Magic".to_string(),
        "Luck".to_string(),
    ]);

    let radar = compute_radar_chart(&data, &config);
    let colors = Color::default_palette();

    // Plot area
    let center_x = width as f32 / 2.0;
    let center_y = height as f32 / 2.0 + 20.0; // Offset for title
    let radius = 200.0_f32;

    // Draw grid rings
    for ring in &radar.grid_rings {
        if ring.len() >= 2 {
            for i in 0..ring.len() {
                let (x1, y1) = ring[i];
                let (x2, y2) = ring[(i + 1) % ring.len()];
                let sx1 = center_x + x1 as f32 * radius;
                let sy1 = center_y - y1 as f32 * radius;
                let sx2 = center_x + x2 as f32 * radius;
                let sy2 = center_y - y2 as f32 * radius;
                renderer.draw_line(sx1, sy1, sx2, sy2, theme.grid_color, 0.5, LineStyle::Solid)?;
            }
        }
    }

    // Draw axes
    for &((x1, y1), (x2, y2)) in &radar.axes {
        let sx1 = center_x + x1 as f32 * radius;
        let sy1 = center_y - y1 as f32 * radius;
        let sx2 = center_x + x2 as f32 * radius;
        let sy2 = center_y - y2 as f32 * radius;
        renderer.draw_line(sx1, sy1, sx2, sy2, theme.grid_color, 0.5, LineStyle::Solid)?;
    }

    // Draw axis labels
    for (label, x, y) in &radar.axis_labels {
        let sx = center_x + *x as f32 * radius;
        let sy = center_y - *y as f32 * radius;
        renderer.draw_text_centered(label, sx, sy, 11.0, theme.foreground)?;
    }

    // Draw series
    let series_labels = ["Player 1", "Player 2"];
    for (idx, series) in radar.series.iter().enumerate() {
        let color = colors[idx];

        // Draw filled polygon
        let polygon: Vec<(f32, f32)> = series
            .polygon
            .iter()
            .map(|&(x, y)| (center_x + x as f32 * radius, center_y - y as f32 * radius))
            .collect();

        if polygon.len() >= 3 {
            renderer.draw_filled_polygon(&polygon, color.with_alpha(0.3))?;
        }

        // Draw outline
        if polygon.len() >= 2 {
            for i in 0..polygon.len() {
                let (x1, y1) = polygon[i];
                let (x2, y2) = polygon[(i + 1) % polygon.len()];
                renderer.draw_line(x1, y1, x2, y2, color, 2.0, LineStyle::Solid)?;
            }
        }

        // Draw markers
        for &(x, y) in &series.markers {
            let sx = center_x + x as f32 * radius;
            let sy = center_y - y as f32 * radius;
            renderer.draw_marker(sx, sy, 5.0, MarkerStyle::Circle, color)?;
        }
    }

    // Title
    renderer.draw_text_centered(
        "Radar Chart",
        width as f32 / 2.0,
        25.0,
        18.0,
        theme.foreground,
    )?;

    // Legend
    for (idx, label) in series_labels.iter().enumerate() {
        let y = 60.0 + idx as f32 * 20.0;
        renderer.draw_line(
            width as f32 - 130.0,
            y,
            width as f32 - 100.0,
            y,
            colors[idx],
            2.0,
            LineStyle::Solid,
        )?;
        renderer.draw_text(label, width as f32 - 95.0, y, 10.0, theme.foreground)?;
    }

    renderer.save_png("docs/images/radar_chart.png")?;
    println!("Generated docs/images/radar_chart.png");
    Ok(())
}
