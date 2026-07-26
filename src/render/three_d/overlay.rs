use crate::core::plot::Image;
use crate::core::plot3d::layout::{
    Axis3Layout, Colorbar3D, Legend3D, LegendGlyph3D, LegendItem3D, OverlayLine3D, OverlayRect3D,
    OverlayText3D,
};
use crate::core::{FigureConfig, Result};
use crate::export::SvgRenderer;
use crate::render::{Color, LineStyle, SkiaRenderer, Theme};

const COLORBAR_SEGMENTS: usize = 64;

pub(crate) fn compose_image(
    layout: &Axis3Layout,
    figure: &FigureConfig,
    theme: &Theme,
    raster_layer: Image,
) -> Result<Image> {
    let mut renderer = SkiaRenderer::new(layout.canvas_width, layout.canvas_height, theme.clone())?;
    renderer.set_render_scale(figure.render_scale());
    draw_background_skia(&mut renderer, layout, figure, theme)?;
    renderer.draw_subplot(raster_layer, 0, 0)?;
    draw_foreground_skia(&mut renderer, layout, figure, theme)?;
    Ok(renderer.into_image_demultiplied())
}

pub(crate) fn compose_svg(
    layout: &Axis3Layout,
    figure: &FigureConfig,
    theme: &Theme,
    raster_layer: &Image,
) -> Result<String> {
    let mut renderer = SvgRenderer::new(layout.canvas_width as f32, layout.canvas_height as f32);
    renderer.set_render_scale(figure.render_scale());
    renderer.draw_rectangle(
        0.0,
        0.0,
        layout.canvas_width as f32,
        layout.canvas_height as f32,
        theme.background,
        true,
    );
    let clip_id = renderer.add_clip_rect(
        layout.viewport.x as f32,
        layout.viewport.y as f32,
        layout.viewport.width as f32,
        layout.viewport.height as f32,
    );
    renderer.start_clip_group(&clip_id);
    for pane in &layout.panes {
        let points = pane.map(|point| (point.x, point.y));
        renderer.draw_filled_polygon(&points, pane_color(theme));
    }
    for line in &layout.grid_lines {
        draw_svg_line(&mut renderer, *line, grid_color(theme), grid_width(figure));
    }
    renderer.end_group();
    renderer.draw_embedded_png(
        raster_layer,
        0.0,
        0.0,
        layout.canvas_width as f32,
        layout.canvas_height as f32,
    )?;
    renderer.start_clip_group(&clip_id);
    for line in &layout.box_edges {
        draw_svg_line(&mut renderer, *line, theme.foreground, axis_width(figure));
    }
    for line in &layout.tick_marks {
        draw_svg_line(&mut renderer, *line, theme.foreground, axis_width(figure));
    }
    renderer.end_group();
    for text in &layout.tick_labels {
        draw_svg_text(
            &mut renderer,
            text,
            tick_font_size(figure, theme),
            theme.foreground,
            layout,
        )?;
    }
    for text in &layout.axis_labels {
        draw_svg_text(
            &mut renderer,
            text,
            axis_font_size(figure, theme),
            theme.foreground,
            layout,
        )?;
    }
    if let Some(title) = &layout.title {
        draw_svg_text(
            &mut renderer,
            title,
            title_font_size(figure, theme),
            theme.foreground,
            layout,
        )?;
    }
    draw_decorations_svg(&mut renderer, layout, figure, theme)?;
    // A shape with a non-finite dimension is dropped rather than emitted as
    // `width="NaN"`; the renderer only latches it, so whoever hands the string
    // out has to surface it.
    renderer.check_geometry()?;
    Ok(renderer.to_svg_string())
}

fn draw_background_skia(
    renderer: &mut SkiaRenderer,
    layout: &Axis3Layout,
    figure: &FigureConfig,
    theme: &Theme,
) -> Result<()> {
    let clip = viewport_clip(layout);
    for pane in &layout.panes {
        let points = pane.map(|point| (point.x, point.y));
        renderer.draw_filled_polygon_clipped(&points, pane_color(theme), clip)?;
    }
    for line in &layout.grid_lines {
        draw_skia_line_clipped(renderer, *line, grid_color(theme), grid_width(figure), clip)?;
    }
    Ok(())
}

fn draw_foreground_skia(
    renderer: &mut SkiaRenderer,
    layout: &Axis3Layout,
    figure: &FigureConfig,
    theme: &Theme,
) -> Result<()> {
    let clip = viewport_clip(layout);
    for line in &layout.box_edges {
        draw_skia_line_clipped(renderer, *line, theme.foreground, axis_width(figure), clip)?;
    }
    for line in &layout.tick_marks {
        draw_skia_line_clipped(renderer, *line, theme.foreground, axis_width(figure), clip)?;
    }
    for text in &layout.tick_labels {
        draw_skia_text(
            renderer,
            text,
            tick_font_size(figure, theme),
            theme.foreground,
            layout,
        )?;
    }
    for text in &layout.axis_labels {
        draw_skia_text(
            renderer,
            text,
            axis_font_size(figure, theme),
            theme.foreground,
            layout,
        )?;
    }
    if let Some(title) = &layout.title {
        draw_skia_text(
            renderer,
            title,
            title_font_size(figure, theme),
            theme.foreground,
            layout,
        )?;
    }
    draw_decorations_skia(renderer, layout, figure, theme)?;
    Ok(())
}

fn draw_decorations_skia(
    renderer: &mut SkiaRenderer,
    layout: &Axis3Layout,
    figure: &FigureConfig,
    theme: &Theme,
) -> Result<()> {
    if let Some(legend) = &layout.legend {
        renderer.draw_solid_rectangle(
            legend.bounds.x,
            legend.bounds.y,
            legend.bounds.width,
            legend.bounds.height,
            theme.background,
        )?;
        renderer.draw_rectangle(
            legend.bounds.x,
            legend.bounds.y,
            legend.bounds.width,
            legend.bounds.height,
            theme.grid_color,
            false,
        )?;
        for item in &legend.items {
            draw_legend_item_skia(renderer, item, figure)?;
            draw_skia_text(
                renderer,
                &item.label,
                legend_font_size(figure, theme),
                theme.foreground,
                layout,
            )?;
        }
    }
    for colorbar in &layout.colorbars {
        draw_colorbar_skia(renderer, colorbar, layout, figure, theme)?;
    }
    Ok(())
}

fn draw_legend_item_skia(
    renderer: &mut SkiaRenderer,
    item: &LegendItem3D,
    figure: &FigureConfig,
) -> Result<()> {
    match item.glyph {
        LegendGlyph3D::Marker => {
            let size = item.glyph_rect.height.min(item.glyph_rect.width) * 0.72;
            renderer.draw_solid_rectangle(
                item.glyph_rect.x + (item.glyph_rect.width - size) * 0.5,
                item.glyph_rect.y + (item.glyph_rect.height - size) * 0.5,
                size,
                size,
                item.color,
            )
        }
        LegendGlyph3D::Line => renderer.draw_line(
            item.glyph_rect.x,
            item.glyph_rect.y + item.glyph_rect.height * 0.5,
            item.glyph_rect.right(),
            item.glyph_rect.y + item.glyph_rect.height * 0.5,
            item.color,
            axis_width(figure).max(1.5),
            LineStyle::Solid,
        ),
        LegendGlyph3D::Fill => renderer.draw_solid_rectangle(
            item.glyph_rect.x,
            item.glyph_rect.y,
            item.glyph_rect.width,
            item.glyph_rect.height,
            item.color,
        ),
    }
}

fn draw_colorbar_skia(
    renderer: &mut SkiaRenderer,
    colorbar: &Colorbar3D,
    layout: &Axis3Layout,
    figure: &FigureConfig,
    theme: &Theme,
) -> Result<()> {
    for (rect, color) in colorbar_segments(colorbar) {
        renderer.draw_solid_rectangle(rect.x, rect.y, rect.width, rect.height, color)?;
    }
    renderer.draw_rectangle(
        colorbar.bounds.x,
        colorbar.bounds.y,
        colorbar.bounds.width,
        colorbar.bounds.height,
        theme.foreground,
        false,
    )?;
    for line in &colorbar.tick_marks {
        draw_skia_line(renderer, *line, theme.foreground, axis_width(figure))?;
    }
    for text in &colorbar.tick_labels {
        draw_skia_text(
            renderer,
            text,
            tick_font_size(figure, theme),
            theme.foreground,
            layout,
        )?;
    }
    Ok(())
}

fn draw_decorations_svg(
    renderer: &mut SvgRenderer,
    layout: &Axis3Layout,
    figure: &FigureConfig,
    theme: &Theme,
) -> Result<()> {
    if let Some(legend) = &layout.legend {
        renderer.draw_rectangle(
            legend.bounds.x,
            legend.bounds.y,
            legend.bounds.width,
            legend.bounds.height,
            theme.background,
            true,
        );
        renderer.draw_rectangle(
            legend.bounds.x,
            legend.bounds.y,
            legend.bounds.width,
            legend.bounds.height,
            theme.grid_color,
            false,
        );
        for item in &legend.items {
            draw_legend_item_svg(renderer, item, figure);
            draw_svg_text(
                renderer,
                &item.label,
                legend_font_size(figure, theme),
                theme.foreground,
                layout,
            )?;
        }
    }
    for colorbar in &layout.colorbars {
        for (rect, color) in colorbar_segments(colorbar) {
            renderer.draw_rectangle(rect.x, rect.y, rect.width, rect.height, color, true);
        }
        renderer.draw_rectangle(
            colorbar.bounds.x,
            colorbar.bounds.y,
            colorbar.bounds.width,
            colorbar.bounds.height,
            theme.foreground,
            false,
        );
        for line in &colorbar.tick_marks {
            draw_svg_line(renderer, *line, theme.foreground, axis_width(figure));
        }
        for text in &colorbar.tick_labels {
            draw_svg_text(
                renderer,
                text,
                tick_font_size(figure, theme),
                theme.foreground,
                layout,
            )?;
        }
    }
    Ok(())
}

fn draw_legend_item_svg(renderer: &mut SvgRenderer, item: &LegendItem3D, figure: &FigureConfig) {
    match item.glyph {
        LegendGlyph3D::Marker => {
            let size = item.glyph_rect.height.min(item.glyph_rect.width) * 0.72;
            renderer.draw_rectangle(
                item.glyph_rect.x + (item.glyph_rect.width - size) * 0.5,
                item.glyph_rect.y + (item.glyph_rect.height - size) * 0.5,
                size,
                size,
                item.color,
                true,
            );
        }
        LegendGlyph3D::Line => renderer.draw_line(
            item.glyph_rect.x,
            item.glyph_rect.y + item.glyph_rect.height * 0.5,
            item.glyph_rect.right(),
            item.glyph_rect.y + item.glyph_rect.height * 0.5,
            item.color,
            axis_width(figure).max(1.5),
            LineStyle::Solid,
        ),
        LegendGlyph3D::Fill => renderer.draw_rectangle(
            item.glyph_rect.x,
            item.glyph_rect.y,
            item.glyph_rect.width,
            item.glyph_rect.height,
            item.color,
            true,
        ),
    }
}

fn colorbar_segments(colorbar: &Colorbar3D) -> impl Iterator<Item = (OverlayRect3D, Color)> + '_ {
    let segment_height = colorbar.bounds.height / COLORBAR_SEGMENTS as f32;
    (0..COLORBAR_SEGMENTS).map(move |index| {
        let normalized = 1.0 - index as f64 / COLORBAR_SEGMENTS.saturating_sub(1).max(1) as f64;
        (
            OverlayRect3D {
                x: colorbar.bounds.x,
                y: colorbar.bounds.y + index as f32 * segment_height,
                width: colorbar.bounds.width,
                height: segment_height + 0.5,
            },
            colorbar.colormap.sample(normalized),
        )
    })
}

fn draw_skia_line(
    renderer: &mut SkiaRenderer,
    line: OverlayLine3D,
    color: Color,
    width: f32,
) -> Result<()> {
    renderer.draw_line(
        line.start.x,
        line.start.y,
        line.end.x,
        line.end.y,
        color,
        width,
        LineStyle::Solid,
    )
}

fn draw_skia_line_clipped(
    renderer: &mut SkiaRenderer,
    line: OverlayLine3D,
    color: Color,
    width: f32,
    clip: (f32, f32, f32, f32),
) -> Result<()> {
    renderer.draw_line_clipped(
        line.start.x,
        line.start.y,
        line.end.x,
        line.end.y,
        color,
        width,
        LineStyle::Solid,
        clip,
    )
}

fn draw_svg_line(renderer: &mut SvgRenderer, line: OverlayLine3D, color: Color, width: f32) {
    renderer.draw_line(
        line.start.x,
        line.start.y,
        line.end.x,
        line.end.y,
        color,
        width,
        LineStyle::Solid,
    );
}

fn draw_skia_text(
    renderer: &mut SkiaRenderer,
    text: &OverlayText3D,
    font_size: f32,
    color: Color,
    layout: &Axis3Layout,
) -> Result<()> {
    let (x, y) = clamped_text_position(text, font_size, layout);
    if text.centered {
        renderer.draw_text_centered(&text.text, x, y, font_size, color)
    } else {
        renderer.draw_text(&text.text, x, y, font_size, color)
    }
}

fn draw_svg_text(
    renderer: &mut SvgRenderer,
    text: &OverlayText3D,
    font_size: f32,
    color: Color,
    layout: &Axis3Layout,
) -> Result<()> {
    let (x, y) = clamped_text_position(text, font_size, layout);
    if text.centered {
        renderer.draw_text_centered(&text.text, x, y, font_size, color)
    } else {
        renderer.draw_text(&text.text, x, y, font_size, color)
    }
}

fn clamped_text_position(text: &OverlayText3D, font_size: f32, layout: &Axis3Layout) -> (f32, f32) {
    let x = text
        .position
        .x
        .clamp(2.0, layout.canvas_width.saturating_sub(2) as f32);
    let y = (text.position.y - font_size * 0.5)
        .clamp(0.0, (layout.canvas_height as f32 - font_size).max(0.0));
    (x, y)
}

fn viewport_clip(layout: &Axis3Layout) -> (f32, f32, f32, f32) {
    (
        layout.viewport.x as f32,
        layout.viewport.y as f32,
        layout.viewport.width as f32,
        layout.viewport.height as f32,
    )
}

fn pane_color(theme: &Theme) -> Color {
    Color::from_rgba(
        theme.grid_color.r,
        theme.grid_color.g,
        theme.grid_color.b,
        28,
    )
}

fn grid_color(theme: &Theme) -> Color {
    Color::from_rgba(
        theme.grid_color.r,
        theme.grid_color.g,
        theme.grid_color.b,
        110,
    )
}

fn axis_width(figure: &FigureConfig) -> f32 {
    (0.8 * figure.dpi / 72.0).max(0.75)
}

fn grid_width(figure: &FigureConfig) -> f32 {
    (0.45 * figure.dpi / 72.0).max(0.5)
}

fn tick_font_size(figure: &FigureConfig, theme: &Theme) -> f32 {
    theme.tick_label_font_size * figure.dpi / 72.0
}

fn axis_font_size(figure: &FigureConfig, theme: &Theme) -> f32 {
    theme.axis_label_font_size * figure.dpi / 72.0
}

fn title_font_size(figure: &FigureConfig, theme: &Theme) -> f32 {
    theme.title_font_size * figure.dpi / 72.0
}

fn legend_font_size(figure: &FigureConfig, theme: &Theme) -> f32 {
    theme.legend_font_size * figure.dpi / 72.0
}

#[cfg(test)]
mod tests {
    use crate::scatter3d;

    use super::COLORBAR_SEGMENTS;

    #[test]
    fn hybrid_svg_contains_one_embedded_raster_layer_and_vector_text() {
        let svg = scatter3d(&[0.0], &[0.0], &[0.0])
            .title("A 3d plot")
            .render_to_svg()
            .expect("svg");
        assert_eq!(svg.matches("<image ").count(), 1);
        assert!(svg.contains("data:image/png;base64,"));
        assert!(svg.contains("A 3d plot"));
        assert!(svg.contains("<line "));
        assert_eq!(
            svg.matches("clip-path=").count(),
            2,
            "background and foreground Axis3 geometry share the plot viewport clip"
        );
    }

    #[test]
    fn hybrid_svg_renders_labeled_series_and_requested_surface_colorbar() {
        let svg = crate::surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [2.0, 3.0]])
            .label("terrain")
            .colorbar(true)
            .render_to_svg()
            .expect("svg");
        assert!(svg.contains("terrain"));
        assert!(
            svg.matches("<rect ").count() >= COLORBAR_SEGMENTS + 3,
            "colorbar gradient and legend should remain vector SVG"
        );
        assert_eq!(svg.matches("<image ").count(), 1);
    }

    #[test]
    fn cpu_image_contains_the_outside_right_legend_and_colorbar_band() {
        let image = crate::surface(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [2.0, 3.0]])
            .label("terrain")
            .colorbar(true)
            .render()
            .expect("image");
        let right_band_start = image.width as usize * 3 / 4;
        let mut colors = std::collections::BTreeSet::new();
        for y in 0..image.height as usize {
            for x in right_band_start..image.width as usize {
                let offset = (y * image.width as usize + x) * 4;
                colors.insert([
                    image.pixels[offset],
                    image.pixels[offset + 1],
                    image.pixels[offset + 2],
                ]);
            }
        }
        assert!(
            colors.len() > 16,
            "the colorbar should contribute a visible range of colors; got {}",
            colors.len()
        );
    }
}
