//! Documentation example: Pie Chart
//!
//! Generates docs/images/pie_chart.png for rustdoc

use ruviz::prelude::*;
use ruviz::render::SkiaRenderer;
use ruviz::render::primitives::pie_wedges;

fn main() -> Result<()> {
    let width = 800;
    let height = 600;
    let theme = Theme::default();

    let mut renderer = SkiaRenderer::new(width, height, theme.clone())?;

    // Clear with background
    renderer.clear();

    // Data for pie chart
    let values = vec![35.0, 25.0, 20.0, 15.0, 5.0];
    let labels = ["Product A", "Product B", "Product C", "Product D", "Other"];
    let colors = Color::default_palette();

    // Center and radius
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0 + 20.0;
    let radius = 180.0;

    // Create wedges
    let wedges = pie_wedges(&values, cx, cy, radius, None);

    // Draw each wedge
    for (i, wedge) in wedges.iter().enumerate() {
        let polygon = wedge.to_polygon(32);
        let points: Vec<(f32, f32)> = polygon
            .iter()
            .map(|(x, y)| (*x as f32, *y as f32))
            .collect();
        renderer.draw_filled_polygon(&points, colors[i % colors.len()])?;

        // Draw outline
        renderer.draw_polygon_outline(&points, Color::WHITE, 2.0)?;

        // Draw label
        let (lx, ly) = wedge.label_position(30.0);
        let percent = values[i] / values.iter().sum::<f64>() * 100.0;
        let label = format!("{}: {:.0}%", labels[i], percent);
        renderer.draw_text_centered(&label, lx as f32, ly as f32, 11.0, theme.foreground)?;
    }

    // Draw title
    renderer.draw_text_centered(
        "Market Share Distribution",
        cx as f32,
        40.0,
        18.0,
        theme.foreground,
    )?;

    renderer.save_png("docs/images/pie_chart.png")?;
    println!("Generated docs/images/pie_chart.png");
    Ok(())
}
