use crate::core::plot::Image;
use crate::core::plot3d::layout::{Axis3Layout, OverlayLine3D, OverlayText3D};
use crate::core::{FigureConfig, Result};
use crate::export::SvgRenderer;
use crate::render::{Color, LineStyle, SkiaRenderer, Theme};

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
    for pane in &layout.panes {
        let points = pane.map(|point| (point.x, point.y));
        renderer.draw_filled_polygon(&points, pane_color(theme));
    }
    for line in &layout.grid_lines {
        draw_svg_line(&mut renderer, *line, grid_color(theme), grid_width(figure));
    }
    renderer.draw_embedded_png(
        raster_layer,
        0.0,
        0.0,
        layout.canvas_width as f32,
        layout.canvas_height as f32,
    )?;
    for line in &layout.box_edges {
        draw_svg_line(&mut renderer, *line, theme.foreground, axis_width(figure));
    }
    for line in &layout.tick_marks {
        draw_svg_line(&mut renderer, *line, theme.foreground, axis_width(figure));
    }
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
    Ok(renderer.to_svg_string())
}

fn draw_background_skia(
    renderer: &mut SkiaRenderer,
    layout: &Axis3Layout,
    figure: &FigureConfig,
    theme: &Theme,
) -> Result<()> {
    for pane in &layout.panes {
        let points = pane.map(|point| (point.x, point.y));
        renderer.draw_filled_polygon(&points, pane_color(theme))?;
    }
    for line in &layout.grid_lines {
        draw_skia_line(renderer, *line, grid_color(theme), grid_width(figure))?;
    }
    Ok(())
}

fn draw_foreground_skia(
    renderer: &mut SkiaRenderer,
    layout: &Axis3Layout,
    figure: &FigureConfig,
    theme: &Theme,
) -> Result<()> {
    for line in &layout.box_edges {
        draw_skia_line(renderer, *line, theme.foreground, axis_width(figure))?;
    }
    for line in &layout.tick_marks {
        draw_skia_line(renderer, *line, theme.foreground, axis_width(figure))?;
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
    Ok(())
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

fn pane_color(theme: &Theme) -> Color {
    Color::new_rgba(
        theme.grid_color.r,
        theme.grid_color.g,
        theme.grid_color.b,
        28,
    )
}

fn grid_color(theme: &Theme) -> Color {
    Color::new_rgba(
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

#[cfg(test)]
mod tests {
    use crate::scatter3d;

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
    }
}
