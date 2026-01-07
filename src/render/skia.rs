use crate::{
    core::{
        ComputedMargins, CoordinateTransform, LayoutRect, Legend, LegendItem, LegendItemType,
        LegendPosition, LegendSpacingPixels, LegendStyle, PlottingError, Result, SpacingConfig,
        TextPosition, TickFormatter, find_best_position, plot::Image, pt_to_px,
    },
    render::{Color, FontConfig, FontFamily, LineStyle, MarkerStyle, TextRenderer, Theme},
};
use std::path::Path;
use tiny_skia::*;

/// Tiny-skia based renderer with cosmic-text for professional typography
pub struct SkiaRenderer {
    width: u32,
    height: u32,
    pixmap: Pixmap,
    paint: Paint<'static>,
    theme: Theme,
    text_renderer: TextRenderer,
    font_config: FontConfig,
}

impl SkiaRenderer {
    /// Create a new renderer with the given dimensions
    pub fn new(width: u32, height: u32, theme: Theme) -> Result<Self> {
        Self::with_font_family(width, height, theme, FontFamily::SansSerif)
    }

    /// Create a new renderer with specified font family
    pub fn with_font_family(
        width: u32,
        height: u32,
        theme: Theme,
        font_family: FontFamily,
    ) -> Result<Self> {
        let mut pixmap = Pixmap::new(width, height).ok_or(PlottingError::OutOfMemory)?;

        // Fill background
        let bg_color = theme.background.to_tiny_skia_color();
        pixmap.fill(bg_color);

        let paint = Paint::default();

        // Create text renderer with default font configuration
        let text_renderer = TextRenderer::new();
        let font_config = FontConfig::new(font_family, 12.0);

        Ok(Self {
            width,
            height,
            pixmap,
            paint,
            theme,
            text_renderer,
            font_config,
        })
    }

    /// Clear the canvas with background color
    pub fn clear(&mut self) {
        let bg_color = self.theme.background.to_tiny_skia_color();
        self.pixmap.fill(bg_color);
    }

    /// Draw a line between two points
    pub fn draw_line(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        width: f32,
        style: LineStyle,
    ) -> Result<()> {
        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;
        paint.set_color_rgba8(color.r, color.g, color.b, color.a);

        let mut stroke = Stroke {
            width: width.max(0.1),
            ..Stroke::default()
        };

        // Apply line style
        if let Some(dash_pattern) = style.to_dash_array() {
            stroke.dash = StrokeDash::new(dash_pattern, 0.0);
        }

        let mut path = PathBuilder::new();
        path.move_to(x1, y1);
        path.line_to(x2, y2);
        let path = path.finish().ok_or(PlottingError::RenderError(
            "Failed to create line path".to_string(),
        ))?;

        self.pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);

        Ok(())
    }

    /// Draw a series of connected lines (polyline)
    pub fn draw_polyline(
        &mut self,
        points: &[(f32, f32)],
        color: Color,
        width: f32,
        style: LineStyle,
    ) -> Result<()> {
        if points.len() < 2 {
            return Ok(());
        }

        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;

        let mut stroke = Stroke {
            width: width.max(0.1),
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };

        // Apply line style
        if let Some(dash_pattern) = style.to_dash_array() {
            stroke.dash = StrokeDash::new(dash_pattern, 0.0);
        }

        let mut path = PathBuilder::new();
        path.move_to(points[0].0, points[0].1);

        for &(x, y) in &points[1..] {
            path.line_to(x, y);
        }

        let path = path.finish().ok_or(PlottingError::RenderError(
            "Failed to create polyline path".to_string(),
        ))?;

        self.pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);

        Ok(())
    }

    /// Draw a polyline clipped to a rectangular region
    pub fn draw_polyline_clipped(
        &mut self,
        points: &[(f32, f32)],
        color: Color,
        width: f32,
        style: LineStyle,
        clip_rect: (f32, f32, f32, f32), // (x, y, width, height)
    ) -> Result<()> {
        if points.len() < 2 {
            return Ok(());
        }

        // Create clip mask
        let mut mask = Mask::new(self.width, self.height).ok_or(PlottingError::RenderError(
            "Failed to create clip mask".to_string(),
        ))?;

        // Create clip path from rectangle
        let clip_path = {
            let mut pb = PathBuilder::new();
            let (x, y, w, h) = clip_rect;
            pb.move_to(x, y);
            pb.line_to(x + w, y);
            pb.line_to(x + w, y + h);
            pb.line_to(x, y + h);
            pb.close();
            pb.finish().ok_or(PlottingError::RenderError(
                "Failed to create clip path".to_string(),
            ))?
        };

        // Fill mask with clip region
        mask.fill_path(&clip_path, FillRule::Winding, true, Transform::identity());

        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;

        let mut stroke = Stroke {
            width: width.max(0.1),
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };

        // Apply line style
        if let Some(dash_pattern) = style.to_dash_array() {
            stroke.dash = StrokeDash::new(dash_pattern, 0.0);
        }

        let mut path = PathBuilder::new();
        path.move_to(points[0].0, points[0].1);

        for &(x, y) in &points[1..] {
            path.line_to(x, y);
        }

        let path = path.finish().ok_or(PlottingError::RenderError(
            "Failed to create polyline path".to_string(),
        ))?;

        self.pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), Some(&mask));

        Ok(())
    }

    /// Draw a circle (for scatter plots)
    pub fn draw_circle(
        &mut self,
        x: f32,
        y: f32,
        radius: f32,
        color: Color,
        filled: bool,
    ) -> Result<()> {
        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;

        let mut path = PathBuilder::new();
        path.push_circle(x, y, radius);
        let path = path.finish().ok_or(PlottingError::RenderError(
            "Failed to create circle path".to_string(),
        ))?;

        if filled {
            self.pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        } else {
            let stroke = Stroke::default();
            self.pixmap
                .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }

        Ok(())
    }

    pub fn draw_rectangle(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        filled: bool,
    ) -> Result<()> {
        let rect = Rect::from_xywh(x, y, width, height).ok_or(PlottingError::RenderError(
            "Invalid rectangle dimensions".to_string(),
        ))?;

        let mut path = PathBuilder::new();
        path.push_rect(rect);
        let path = path.finish().ok_or(PlottingError::RenderError(
            "Failed to create rectangle path".to_string(),
        ))?;

        if filled {
            // Professional filled rectangle with subtle transparency and border
            let mut fill_paint = Paint::default();

            // Use slightly transparent fill for professional look
            let (r, g, b, a) = color.to_rgba_f32();
            let professional_alpha = (a * 0.85).min(1.0); // 85% opacity for better visual appeal
            let fill_color = tiny_skia::Color::from_rgba(r, g, b, professional_alpha).ok_or(
                PlottingError::RenderError("Invalid color for rectangle fill".to_string()),
            )?;

            fill_paint.set_color(fill_color);
            fill_paint.anti_alias = true;

            // Fill the rectangle
            self.pixmap.fill_path(
                &path,
                &fill_paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );

            // Add professional border for definition
            let mut border_paint = Paint::default();

            // Darker border color (20% darker than fill)
            let border_r = (r * 0.8).max(0.0);
            let border_g = (g * 0.8).max(0.0);
            let border_b = (b * 0.8).max(0.0);
            let border_color = tiny_skia::Color::from_rgba(border_r, border_g, border_b, a).ok_or(
                PlottingError::RenderError("Invalid border color".to_string()),
            )?;

            border_paint.set_color(border_color);
            border_paint.anti_alias = true;

            // Professional border stroke (1.0px width)
            let stroke = Stroke {
                width: 1.0,
                line_cap: LineCap::Square,
                line_join: LineJoin::Miter,
                ..Stroke::default()
            };

            self.pixmap
                .stroke_path(&path, &border_paint, &stroke, Transform::identity(), None);
        } else {
            // Outline only
            let mut paint = Paint::default();
            paint.set_color(color.to_tiny_skia_color());
            paint.anti_alias = true;

            let stroke = Stroke::default();
            self.pixmap
                .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }

        Ok(())
    }

    /// Draw a solid color rectangle with no transparency or border
    /// Used for gradient segments like colorbar where 100% opacity and no anti-aliasing is needed
    pub fn draw_solid_rectangle(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) -> Result<()> {
        let rect = Rect::from_xywh(x, y, width, height).ok_or(PlottingError::RenderError(
            "Invalid rectangle dimensions".to_string(),
        ))?;

        let mut path = PathBuilder::new();
        path.push_rect(rect);
        let path = path.finish().ok_or(PlottingError::RenderError(
            "Failed to create rectangle path".to_string(),
        ))?;

        let mut fill_paint = Paint::default();
        fill_paint.set_color(color.to_tiny_skia_color());
        fill_paint.anti_alias = false; // No anti-aliasing for crisp edges

        self.pixmap.fill_path(
            &path,
            &fill_paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );

        Ok(())
    }

    /// Draw a rounded rectangle with the given corner radius
    pub fn draw_rounded_rectangle(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        corner_radius: f32,
        color: Color,
        filled: bool,
    ) -> Result<()> {
        // Clamp radius to half of the smaller dimension
        let max_radius = (width.min(height) / 2.0).max(0.0);
        let radius = corner_radius.min(max_radius);

        // If radius is effectively zero, use regular rectangle
        if radius < 0.1 {
            return self.draw_rectangle(x, y, width, height, color, filled);
        }

        // Build rounded rectangle path manually
        let mut pb = PathBuilder::new();

        // Start at top-left, after the corner arc
        pb.move_to(x + radius, y);

        // Top edge
        pb.line_to(x + width - radius, y);
        // Top-right corner
        pb.quad_to(x + width, y, x + width, y + radius);

        // Right edge
        pb.line_to(x + width, y + height - radius);
        // Bottom-right corner
        pb.quad_to(x + width, y + height, x + width - radius, y + height);

        // Bottom edge
        pb.line_to(x + radius, y + height);
        // Bottom-left corner
        pb.quad_to(x, y + height, x, y + height - radius);

        // Left edge
        pb.line_to(x, y + radius);
        // Top-left corner
        pb.quad_to(x, y, x + radius, y);

        pb.close();

        let path = pb.finish().ok_or(PlottingError::RenderError(
            "Failed to create rounded rectangle path".to_string(),
        ))?;

        if filled {
            let mut fill_paint = Paint::default();
            let (r, g, b, a) = color.to_rgba_f32();
            let fill_color = tiny_skia::Color::from_rgba(r, g, b, a).ok_or(
                PlottingError::RenderError("Invalid color for rounded rectangle fill".to_string()),
            )?;

            fill_paint.set_color(fill_color);
            fill_paint.anti_alias = true;

            self.pixmap.fill_path(
                &path,
                &fill_paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        } else {
            // Outline only
            let mut paint = Paint::default();
            paint.set_color(color.to_tiny_skia_color());
            paint.anti_alias = true;

            let stroke = Stroke::default();
            self.pixmap
                .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }

        Ok(())
    }

    /// Draw a filled polygon from a list of vertices
    ///
    /// The polygon is automatically closed.
    pub fn draw_filled_polygon(&mut self, vertices: &[(f32, f32)], color: Color) -> Result<()> {
        if vertices.len() < 3 {
            return Ok(()); // Need at least 3 points
        }

        let mut pb = PathBuilder::new();
        pb.move_to(vertices[0].0, vertices[0].1);

        for &(x, y) in &vertices[1..] {
            pb.line_to(x, y);
        }

        pb.close();

        let path = pb.finish().ok_or(PlottingError::RenderError(
            "Failed to create polygon path".to_string(),
        ))?;

        let mut paint = Paint::default();
        let (r, g, b, a) = color.to_rgba_f32();
        let fill_color = tiny_skia::Color::from_rgba(r, g, b, a).ok_or(
            PlottingError::RenderError("Invalid polygon color".to_string()),
        )?;

        paint.set_color(fill_color);
        paint.anti_alias = true;

        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );

        Ok(())
    }

    /// Draw a filled polygon clipped to a rectangular region
    ///
    /// This is useful for rendering shapes that should not extend beyond
    /// a specific area (e.g., violin plots within plot bounds).
    pub fn draw_filled_polygon_clipped(
        &mut self,
        vertices: &[(f32, f32)],
        color: Color,
        clip_rect: (f32, f32, f32, f32), // (x, y, width, height)
    ) -> Result<()> {
        if vertices.len() < 3 {
            return Ok(()); // Need at least 3 points
        }

        // Create clip mask
        let mut mask = Mask::new(self.width, self.height).ok_or(PlottingError::RenderError(
            "Failed to create clip mask".to_string(),
        ))?;

        // Create clip path from rectangle
        let clip_path = {
            let mut pb = PathBuilder::new();
            let (x, y, w, h) = clip_rect;
            pb.move_to(x, y);
            pb.line_to(x + w, y);
            pb.line_to(x + w, y + h);
            pb.line_to(x, y + h);
            pb.close();
            pb.finish().ok_or(PlottingError::RenderError(
                "Failed to create clip path".to_string(),
            ))?
        };

        // Fill mask with clip region (white = allow rendering)
        mask.fill_path(&clip_path, FillRule::Winding, true, Transform::identity());

        // Create polygon path
        let mut pb = PathBuilder::new();
        pb.move_to(vertices[0].0, vertices[0].1);

        for &(x, y) in &vertices[1..] {
            pb.line_to(x, y);
        }

        pb.close();

        let path = pb.finish().ok_or(PlottingError::RenderError(
            "Failed to create polygon path".to_string(),
        ))?;

        let mut paint = Paint::default();
        let (r, g, b, a) = color.to_rgba_f32();
        let fill_color = tiny_skia::Color::from_rgba(r, g, b, a).ok_or(
            PlottingError::RenderError("Invalid polygon color".to_string()),
        )?;

        paint.set_color(fill_color);
        paint.anti_alias = true;

        // Draw with clip mask
        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            Some(&mask),
        );

        Ok(())
    }

    /// Draw the outline of a polygon
    pub fn draw_polygon_outline(
        &mut self,
        vertices: &[(f32, f32)],
        color: Color,
        width: f32,
    ) -> Result<()> {
        if vertices.len() < 3 {
            return Ok(()); // Need at least 3 points
        }

        let mut pb = PathBuilder::new();
        pb.move_to(vertices[0].0, vertices[0].1);

        for &(x, y) in &vertices[1..] {
            pb.line_to(x, y);
        }

        pb.close();

        let path = pb.finish().ok_or(PlottingError::RenderError(
            "Failed to create polygon outline path".to_string(),
        ))?;

        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;

        let stroke = Stroke {
            width,
            ..Stroke::default()
        };

        self.pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);

        Ok(())
    }

    /// Draw a marker at the given position
    pub fn draw_marker(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        style: MarkerStyle,
        color: Color,
    ) -> Result<()> {
        let radius = size * 0.5;

        match style {
            MarkerStyle::Circle => {
                self.draw_circle(x, y, radius, color, true)?;
            }
            MarkerStyle::Square => {
                let half_size = radius;
                self.draw_rectangle(x - half_size, y - half_size, size, size, color, true)?;
            }
            MarkerStyle::Triangle => {
                let mut paint = Paint::default();
                paint.set_color(color.to_tiny_skia_color());
                paint.anti_alias = true;

                let mut path = PathBuilder::new();
                path.move_to(x, y - radius);
                path.line_to(x - radius * 0.866, y + radius * 0.5); // 60 degree angles
                path.line_to(x + radius * 0.866, y + radius * 0.5);
                path.close();

                let path = path.finish().ok_or(PlottingError::RenderError(
                    "Failed to create triangle path".to_string(),
                ))?;
                self.pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
            MarkerStyle::Diamond => {
                let mut paint = Paint::default();
                paint.set_color(color.to_tiny_skia_color());
                paint.anti_alias = true;

                let mut path = PathBuilder::new();
                path.move_to(x, y - radius);
                path.line_to(x + radius, y);
                path.line_to(x, y + radius);
                path.line_to(x - radius, y);
                path.close();

                let path = path.finish().ok_or(PlottingError::RenderError(
                    "Failed to create diamond path".to_string(),
                ))?;
                self.pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
            MarkerStyle::Plus => {
                // Draw cross with lines
                self.draw_line(x - radius, y, x + radius, y, color, 2.0, LineStyle::Solid)?;
                self.draw_line(x, y - radius, x, y + radius, color, 2.0, LineStyle::Solid)?;
            }
            MarkerStyle::Cross => {
                // Draw X with lines
                let offset = radius * 0.707; // sin(45°)
                self.draw_line(
                    x - offset,
                    y - offset,
                    x + offset,
                    y + offset,
                    color,
                    2.0,
                    LineStyle::Solid,
                )?;
                self.draw_line(
                    x - offset,
                    y + offset,
                    x + offset,
                    y - offset,
                    color,
                    2.0,
                    LineStyle::Solid,
                )?;
            }
            _ => {
                // For other marker types, fallback to circle
                self.draw_circle(x, y, radius, color, style.is_filled())?;
            }
        }

        Ok(())
    }

    /// Draw grid lines
    pub fn draw_grid(
        &mut self,
        x_ticks: &[f32],
        y_ticks: &[f32],
        plot_area: Rect,
        color: Color,
        style: LineStyle,
        line_width: f32,
    ) -> Result<()> {
        // Vertical grid lines
        for &x in x_ticks {
            if x >= plot_area.left() && x <= plot_area.right() {
                self.draw_line(
                    x,
                    plot_area.top(),
                    x,
                    plot_area.bottom(),
                    color,
                    line_width,
                    style.clone(),
                )?;
            }
        }

        // Horizontal grid lines
        for &y in y_ticks {
            if y >= plot_area.top() && y <= plot_area.bottom() {
                self.draw_line(
                    plot_area.left(),
                    y,
                    plot_area.right(),
                    y,
                    color,
                    line_width,
                    style.clone(),
                )?;
            }
        }

        Ok(())
    }

    /// Draw axis lines and ticks
    pub fn draw_axes(
        &mut self,
        plot_area: Rect,
        x_ticks: &[f32],
        y_ticks: &[f32],
        color: Color,
    ) -> Result<()> {
        let axis_width = 1.5;
        let tick_size = 5.0;

        // Draw main axis lines
        // X-axis (bottom)
        self.draw_line(
            plot_area.left(),
            plot_area.bottom(),
            plot_area.right(),
            plot_area.bottom(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        // Y-axis (left)
        self.draw_line(
            plot_area.left(),
            plot_area.top(),
            plot_area.left(),
            plot_area.bottom(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        // Draw tick marks
        // X-axis ticks
        for &x in x_ticks {
            if x >= plot_area.left() && x <= plot_area.right() {
                self.draw_line(
                    x,
                    plot_area.bottom(),
                    x,
                    plot_area.bottom() + tick_size,
                    color,
                    1.0,
                    LineStyle::Solid,
                )?;
            }
        }

        // Y-axis ticks
        for &y in y_ticks {
            if y >= plot_area.top() && y <= plot_area.bottom() {
                self.draw_line(
                    plot_area.left() - tick_size,
                    y,
                    plot_area.left(),
                    y,
                    color,
                    1.0,
                    LineStyle::Solid,
                )?;
            }
        }

        Ok(())
    }

    /// Draw axis lines and ticks with advanced configuration
    pub fn draw_axes_with_config(
        &mut self,
        plot_area: Rect,
        x_major_ticks: &[f32],
        y_major_ticks: &[f32],
        x_minor_ticks: &[f32],
        y_minor_ticks: &[f32],
        tick_direction: &crate::core::plot::TickDirection,
        color: Color,
    ) -> Result<()> {
        let axis_width = 1.5;
        let major_tick_size = 8.0;
        let minor_tick_size = 4.0;

        // Draw main axis lines
        // X-axis (bottom)
        self.draw_line(
            plot_area.left(),
            plot_area.bottom(),
            plot_area.right(),
            plot_area.bottom(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        // Y-axis (left)
        self.draw_line(
            plot_area.left(),
            plot_area.top(),
            plot_area.left(),
            plot_area.bottom(),
            color,
            axis_width,
            LineStyle::Solid,
        )?;

        // Determine tick direction multiplier
        let tick_dir_multiplier = match tick_direction {
            crate::core::plot::TickDirection::Inside => -1.0,
            crate::core::plot::TickDirection::Outside => 1.0,
        };

        // Draw major tick marks on X-axis
        for &x in x_major_ticks {
            if x >= plot_area.left() && x <= plot_area.right() {
                let tick_start = plot_area.bottom();
                let tick_end = plot_area.bottom() + major_tick_size * tick_dir_multiplier;
                self.draw_line(x, tick_start, x, tick_end, color, 1.5, LineStyle::Solid)?;
            }
        }

        // Draw minor tick marks on X-axis
        for &x in x_minor_ticks {
            if x >= plot_area.left() && x <= plot_area.right() {
                let tick_start = plot_area.bottom();
                let tick_end = plot_area.bottom() + minor_tick_size * tick_dir_multiplier;
                self.draw_line(x, tick_start, x, tick_end, color, 1.0, LineStyle::Solid)?;
            }
        }

        // Draw major tick marks on Y-axis
        for &y in y_major_ticks {
            if y >= plot_area.top() && y <= plot_area.bottom() {
                let tick_start = plot_area.left();
                let tick_end = plot_area.left() + major_tick_size * -tick_dir_multiplier; // Opposite direction for Y-axis
                self.draw_line(tick_start, y, tick_end, y, color, 1.5, LineStyle::Solid)?;
            }
        }

        // Draw minor tick marks on Y-axis
        for &y in y_minor_ticks {
            if y >= plot_area.top() && y <= plot_area.bottom() {
                let tick_start = plot_area.left();
                let tick_end = plot_area.left() + minor_tick_size * -tick_dir_multiplier; // Opposite direction for Y-axis
                self.draw_line(tick_start, y, tick_end, y, color, 1.0, LineStyle::Solid)?;
            }
        }

        Ok(())
    }

    /// Draw a DataShader aggregated image
    pub fn draw_datashader_image(
        &mut self,
        image: &crate::data::DataShaderImage,
        plot_area: Rect,
    ) -> Result<()> {
        // Create a pixmap from the DataShader image data
        let mut datashader_pixmap = Pixmap::new(image.width as u32, image.height as u32)
            .ok_or(PlottingError::OutOfMemory)?;

        // Copy the RGBA data from DataShader
        if image.pixels.len() != (image.width * image.height * 4) {
            return Err(PlottingError::RenderError(
                "Invalid DataShader image pixel data".to_string(),
            ));
        }

        // Convert RGBA u8 data to tiny-skia's format
        let pixmap_data = datashader_pixmap.data_mut();
        for (i, chunk) in image.pixels.chunks_exact(4).enumerate() {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];

            // tiny-skia uses premultiplied alpha BGRA format
            let alpha_f = a as f32 / 255.0;
            let premult_r = (r as f32 * alpha_f) as u8;
            let premult_g = (g as f32 * alpha_f) as u8;
            let premult_b = (b as f32 * alpha_f) as u8;

            // BGRA order for tiny-skia
            pixmap_data[i * 4] = premult_b;
            pixmap_data[i * 4 + 1] = premult_g;
            pixmap_data[i * 4 + 2] = premult_r;
            pixmap_data[i * 4 + 3] = a;
        }

        // Scale and draw the DataShader image onto the plot area
        let src_rect = Rect::from_xywh(0.0, 0.0, image.width as f32, image.height as f32).ok_or(
            PlottingError::RenderError("Invalid source rect".to_string()),
        )?;

        let transform = Transform::from_scale(
            plot_area.width() / image.width as f32,
            plot_area.height() / image.height as f32,
        )
        .post_translate(plot_area.x(), plot_area.y());

        self.pixmap.draw_pixmap(
            plot_area.x() as i32,
            plot_area.y() as i32,
            datashader_pixmap.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );

        Ok(())
    }

    /// Draw text at the specified position using cosmic-text (professional quality)
    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color) -> Result<()> {
        let config = FontConfig::new(self.font_config.family.clone(), size);
        self.text_renderer
            .render_text(&mut self.pixmap, text, x, y, &config, color)
    }

    /// Draw text rotated 90 degrees counterclockwise using cosmic-text
    pub fn draw_text_rotated(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) -> Result<()> {
        let config = FontConfig::new(self.font_config.family.clone(), size);
        self.text_renderer
            .render_text_rotated(&mut self.pixmap, text, x, y, &config, color)
    }

    /// Draw text centered horizontally at the given position
    pub fn draw_text_centered(
        &mut self,
        text: &str,
        center_x: f32,
        y: f32,
        size: f32,
        color: Color,
    ) -> Result<()> {
        let config = FontConfig::new(self.font_config.family.clone(), size);
        self.text_renderer
            .render_text_centered(&mut self.pixmap, text, center_x, y, &config, color)
    }

    /// Measure text dimensions
    pub fn measure_text(&self, text: &str, size: f32) -> Result<(f32, f32)> {
        let config = FontConfig::new(self.font_config.family.clone(), size);
        self.text_renderer.measure_text(text, &config)
    }

    /// Draw axis labels and tick values using spacing configuration
    ///
    /// Positions tick labels and axis labels using `spacing.tick_pad` and `spacing.label_pad`
    /// for consistent, DPI-independent spacing.
    pub fn draw_axis_labels(
        &mut self,
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        x_label: &str,
        y_label: &str,
        color: Color,
        label_size: f32,
        dpi: f32,
        spacing: &SpacingConfig,
    ) -> Result<()> {
        let tick_size = label_size * 0.7; // Tick labels slightly smaller than axis labels
        let dpi_scale = dpi / 100.0;

        // Convert spacing config values from points to pixels
        let tick_pad_px = pt_to_px(spacing.tick_pad, dpi);
        let label_pad_px = pt_to_px(spacing.label_pad, dpi);
        let char_width_estimate = 4.0 * dpi_scale;

        // Generate ticks and format all labels with consistent precision
        let x_ticks = generate_ticks(x_min, x_max, 5);
        let y_ticks = generate_ticks(y_min, y_max, 5);
        let x_labels = format_tick_labels(&x_ticks);
        let y_labels = format_tick_labels(&y_ticks);

        // Draw X-axis tick labels
        for (tick_value, label_text) in x_ticks.iter().zip(x_labels.iter()) {
            let x_pixel = plot_area.left()
                + (*tick_value - x_min) as f32 / (x_max - x_min) as f32 * plot_area.width();

            let text_width_estimate = label_text.len() as f32 * char_width_estimate / 2.0;
            let label_x = (x_pixel - text_width_estimate)
                .max(0.0)
                .min(self.width() as f32 - text_width_estimate * 2.0);
            // Position tick labels with tick_pad below the axis
            let label_y = (plot_area.bottom() + tick_pad_px + tick_size)
                .min(self.height() as f32 - tick_size - 5.0);
            self.draw_text(label_text, label_x, label_y, tick_size, color)?;
        }

        // Draw Y-axis tick labels
        for (tick_value, label_text) in y_ticks.iter().zip(y_labels.iter()) {
            let y_pixel = plot_area.bottom()
                - (*tick_value - y_min) as f32 / (y_max - y_min) as f32 * plot_area.height();

            let text_width_estimate = label_text.len() as f32 * char_width_estimate;
            // Position tick labels with tick_pad left of the axis
            let label_x = (plot_area.left() - text_width_estimate - tick_pad_px).max(5.0);
            self.draw_text(
                label_text,
                label_x,
                y_pixel - tick_size / 3.0,
                tick_size,
                color,
            )?;
        }

        // Draw X-axis label: positioned label_pad below the tick labels
        let x_label_x =
            plot_area.left() + plot_area.width() / 2.0 - x_label.len() as f32 * char_width_estimate;
        // X-label goes below tick labels: bottom + tick_pad + tick_size + label_pad
        let x_label_y = plot_area.bottom() + tick_pad_px + tick_size + label_pad_px + label_size;
        self.draw_text(x_label, x_label_x, x_label_y, label_size, color)?;

        // Draw Y-axis label (rotated 90 degrees counterclockwise)
        // Position label_pad left of the tick labels
        // Estimate tick label width (assume ~4 characters average)
        let estimated_tick_width = 4.0 * char_width_estimate;
        let y_label_x = plot_area.left() - tick_pad_px - estimated_tick_width - label_pad_px;
        let y_label_y = plot_area.top() + plot_area.height() / 2.0;
        self.draw_text_rotated(y_label, y_label_x, y_label_y, label_size, color)?;

        // Draw border around plot area
        self.draw_plot_border(plot_area, color, dpi_scale)?;

        Ok(())
    }

    /// Draw axis labels with DPI scale (legacy compatibility)
    pub fn draw_axis_labels_legacy(
        &mut self,
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        x_label: &str,
        y_label: &str,
        color: Color,
        label_size: f32,
        dpi_scale: f32,
    ) -> Result<()> {
        let tick_size = label_size * 0.7;
        let tick_offset_y = 20.0 * dpi_scale;
        let x_label_offset = 50.0 * dpi_scale;
        let y_label_offset = 25.0 * dpi_scale;
        let char_width_estimate = 4.0 * dpi_scale;

        // Generate ticks and format all labels with consistent precision
        let x_ticks = generate_ticks(x_min, x_max, 5);
        let y_ticks = generate_ticks(y_min, y_max, 5);
        let x_labels = format_tick_labels(&x_ticks);
        let y_labels = format_tick_labels(&y_ticks);

        for (tick_value, label_text) in x_ticks.iter().zip(x_labels.iter()) {
            let x_pixel = plot_area.left()
                + (*tick_value - x_min) as f32 / (x_max - x_min) as f32 * plot_area.width();
            let text_width_estimate = label_text.len() as f32 * char_width_estimate / 2.0;
            let label_x = (x_pixel - text_width_estimate)
                .max(0.0)
                .min(self.width() as f32 - text_width_estimate * 2.0);
            let label_y =
                (plot_area.bottom() + tick_offset_y).min(self.height() as f32 - tick_size - 5.0);
            self.draw_text(label_text, label_x, label_y, tick_size, color)?;
        }

        for (tick_value, label_text) in y_ticks.iter().zip(y_labels.iter()) {
            let y_pixel = plot_area.bottom()
                - (*tick_value - y_min) as f32 / (y_max - y_min) as f32 * plot_area.height();
            let text_width_estimate = label_text.len() as f32 * char_width_estimate;
            let label_x = (plot_area.left() - text_width_estimate - 15.0 * dpi_scale).max(5.0);
            self.draw_text(
                label_text,
                label_x,
                y_pixel - tick_size / 3.0,
                tick_size,
                color,
            )?;
        }

        let x_label_x =
            plot_area.left() + plot_area.width() / 2.0 - x_label.len() as f32 * char_width_estimate;
        let x_label_y = plot_area.bottom() + x_label_offset;
        self.draw_text(x_label, x_label_x, x_label_y, label_size, color)?;

        let y_label_x = plot_area.left() - y_label_offset;
        let y_label_y = plot_area.top() + plot_area.height() / 2.0;
        self.draw_text_rotated(y_label, y_label_x, y_label_y, label_size, color)?;

        self.draw_plot_border(plot_area, color, dpi_scale)?;

        Ok(())
    }

    /// Draw axis labels and tick values with provided major ticks
    pub fn draw_axis_labels_with_ticks(
        &mut self,
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        x_major_ticks: &[f64],
        y_major_ticks: &[f64],
        x_label: &str,
        y_label: &str,
        color: Color,
        label_size: f32,
        dpi_scale: f32,
    ) -> Result<()> {
        let tick_size = label_size * 0.7; // Tick labels slightly smaller than axis labels

        // Scale all positioning offsets with DPI - increased spacing
        let tick_offset_y = 25.0 * dpi_scale; // Increased from 20.0
        let x_label_offset = 55.0 * dpi_scale; // Increased from 50.0
        let y_label_offset = 50.0 * dpi_scale; // Increased from 25.0 to give more space
        let y_tick_offset = 15.0 * dpi_scale; // Additional offset for Y-axis tick labels
        let char_width_estimate = 4.0 * dpi_scale; // Rough character width for centering

        // Format all tick labels with consistent precision
        let x_labels = format_tick_labels(x_major_ticks);
        let y_labels = format_tick_labels(y_major_ticks);

        // Draw X-axis tick labels using provided major ticks
        for (tick_value, label_text) in x_major_ticks.iter().zip(x_labels.iter()) {
            let x_pixel = plot_area.left()
                + (*tick_value - x_min) as f32 / (x_max - x_min) as f32 * plot_area.width();

            // Center X-axis tick labels horizontally under the tick mark, with proper offset
            // Ensure labels don't overflow canvas bounds
            let text_width_estimate = label_text.len() as f32 * char_width_estimate / 2.0;
            let label_x = (x_pixel - text_width_estimate)
                .max(0.0)
                .min(self.width() as f32 - text_width_estimate * 2.0);
            let label_y =
                (plot_area.bottom() + tick_offset_y).min(self.height() as f32 - tick_size - 5.0); // Ensure within canvas
            self.draw_text(label_text, label_x, label_y, tick_size, color)?;
        }

        // Draw Y-axis tick labels using provided major ticks
        for (tick_value, label_text) in y_major_ticks.iter().zip(y_labels.iter()) {
            let y_pixel = plot_area.bottom()
                - (*tick_value - y_min) as f32 / (y_max - y_min) as f32 * plot_area.height();

            // Right-align Y-axis tick labels next to the tick mark with proper offset
            // Ensure labels fit within the left margin space
            let text_width_estimate = label_text.len() as f32 * char_width_estimate;
            let label_x = (plot_area.left() - text_width_estimate - y_tick_offset).max(5.0); // Ensure minimum 5px from canvas edge
            self.draw_text(
                label_text,
                label_x,
                y_pixel + tick_size * 0.3,
                tick_size,
                color,
            )?;
        }

        // Draw X-axis label
        let x_label_x =
            plot_area.left() + plot_area.width() / 2.0 - x_label.len() as f32 * char_width_estimate;
        let x_label_y = plot_area.bottom() + x_label_offset;
        self.draw_text(x_label, x_label_x, x_label_y, label_size, color)?;

        // Draw Y-axis label (rotated 90 degrees counterclockwise)
        // Calculate required margin based on rotated text dimensions
        let estimated_text_width = y_label.len() as f32 * label_size * 0.8;
        let improved_y_label_offset = (estimated_text_width * 0.6).max(y_label_offset);
        let y_label_x = plot_area.left() - improved_y_label_offset;
        let y_label_y = plot_area.top() + plot_area.height() / 2.0;
        self.draw_text_rotated(y_label, y_label_x, y_label_y, label_size, color)?;

        // Draw border around plot area
        self.draw_plot_border(plot_area, color, dpi_scale)?;

        Ok(())
    }

    /// Draw axis labels with categorical x-axis labels for bar charts (legacy style)
    ///
    /// Similar to `draw_axis_labels_with_ticks` but uses category names on x-axis
    /// instead of numeric tick values.
    ///
    /// Uses the same data-to-pixel mapping as bar rendering to ensure precise alignment.
    pub fn draw_axis_labels_with_categories(
        &mut self,
        plot_area: Rect,
        categories: &[String],
        y_min: f64,
        y_max: f64,
        y_major_ticks: &[f64],
        x_label: &str,
        y_label: &str,
        color: Color,
        label_size: f32,
        dpi_scale: f32,
    ) -> Result<()> {
        let tick_size = label_size * 0.7;
        let tick_offset_y = 25.0 * dpi_scale;
        let x_label_offset = 55.0 * dpi_scale;
        let y_label_offset = 50.0 * dpi_scale;
        let y_tick_offset = 15.0 * dpi_scale;
        let char_width_estimate = 4.0 * dpi_scale;

        // Draw X-axis category labels using same data-to-pixel mapping as bars
        let n_categories = categories.len();
        if n_categories > 0 {
            // X-axis range with matplotlib-compatible padding: [-0.5, n-0.5]
            let x_min = -0.5_f64;
            let x_max = n_categories as f64 - 0.5;
            let x_range = x_max - x_min;

            for (i, category) in categories.iter().enumerate() {
                // Position label at category index (same as bar center in data space)
                let x_data = i as f64;
                let x_center =
                    plot_area.left() + ((x_data - x_min) / x_range) as f32 * plot_area.width();

                // Estimate text width for centering
                let text_width_estimate = category.len() as f32 * char_width_estimate / 2.0;
                let label_x = (x_center - text_width_estimate)
                    .max(0.0)
                    .min(self.width() as f32 - text_width_estimate * 2.0);
                let label_y = (plot_area.bottom() + tick_offset_y)
                    .min(self.height() as f32 - tick_size - 5.0);

                self.draw_text(category, label_x, label_y, tick_size, color)?;
            }
        }

        // Draw Y-axis tick labels with consistent precision
        let y_labels = format_tick_labels(y_major_ticks);
        for (tick_value, label_text) in y_major_ticks.iter().zip(y_labels.iter()) {
            let y_pixel = plot_area.bottom()
                - (*tick_value - y_min) as f32 / (y_max - y_min) as f32 * plot_area.height();

            let text_width_estimate = label_text.len() as f32 * char_width_estimate;
            let label_x = (plot_area.left() - text_width_estimate - y_tick_offset).max(5.0);
            self.draw_text(
                label_text,
                label_x,
                y_pixel + tick_size * 0.3,
                tick_size,
                color,
            )?;
        }

        // Draw X-axis label
        let x_label_x =
            plot_area.left() + plot_area.width() / 2.0 - x_label.len() as f32 * char_width_estimate;
        let x_label_y = plot_area.bottom() + x_label_offset;
        self.draw_text(x_label, x_label_x, x_label_y, label_size, color)?;

        // Draw Y-axis label (rotated)
        let estimated_text_width = y_label.len() as f32 * label_size * 0.8;
        let improved_y_label_offset = (estimated_text_width * 0.6).max(y_label_offset);
        let y_label_x = plot_area.left() - improved_y_label_offset;
        let y_label_y = plot_area.top() + plot_area.height() / 2.0;
        self.draw_text_rotated(y_label, y_label_x, y_label_y, label_size, color)?;

        // Draw border around plot area
        self.draw_plot_border(plot_area, color, dpi_scale)?;

        Ok(())
    }

    /// Draw border around plot area
    pub fn draw_plot_border(
        &mut self,
        plot_area: Rect,
        color: Color,
        dpi_scale: f32,
    ) -> Result<()> {
        let border_width = 1.0 * dpi_scale;

        // Create border paint
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(color.r, color.g, color.b, color.a);
        paint.anti_alias = true;

        // Create stroke
        let stroke = tiny_skia::Stroke {
            width: border_width,
            ..tiny_skia::Stroke::default()
        };

        // Draw rectangle border around plot area
        let path = tiny_skia::PathBuilder::from_rect(plot_area);
        self.pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            tiny_skia::Transform::identity(),
            None,
        );

        Ok(())
    }

    /// Draw title using spacing configuration
    ///
    /// The title is positioned near the top of the canvas with minimal padding.
    pub fn draw_title(
        &mut self,
        title: &str,
        _plot_area: Rect,
        color: Color,
        title_size: f32,
        dpi: f32,
        _spacing: &SpacingConfig,
    ) -> Result<()> {
        // Center title horizontally over the entire canvas width
        let canvas_center_x = self.width() as f32 / 2.0;

        // Position title near top of canvas with small top padding
        // Text baseline is at title_y, so top of text is roughly at title_y - title_size * 0.8
        let top_padding = 8.0 * (dpi / 100.0); // 8px at 100 DPI
        let title_y = top_padding + title_size;

        self.draw_text_centered(title, canvas_center_x, title_y, title_size, color)
    }

    /// Draw title at a computed position from LayoutCalculator
    ///
    /// This is the preferred method for content-driven layout.
    pub fn draw_title_at(&mut self, pos: &TextPosition, text: &str, color: Color) -> Result<()> {
        self.draw_text_centered(text, pos.x, pos.y, pos.size, color)
    }

    /// Draw X-axis label at a computed position from LayoutCalculator
    ///
    /// This is the preferred method for content-driven layout.
    pub fn draw_xlabel_at(&mut self, pos: &TextPosition, text: &str, color: Color) -> Result<()> {
        self.draw_text_centered(text, pos.x, pos.y, pos.size, color)
    }

    /// Draw Y-axis label at a computed position from LayoutCalculator
    ///
    /// The text is rotated 90° counterclockwise for vertical display.
    pub fn draw_ylabel_at(&mut self, pos: &TextPosition, text: &str, color: Color) -> Result<()> {
        self.draw_text_rotated(text, pos.x, pos.y, pos.size, color)
    }

    /// Draw axis tick labels and border using layout positions
    ///
    /// Uses the computed positions from LayoutCalculator for precise placement.
    pub fn draw_axis_labels_at(
        &mut self,
        plot_area: &LayoutRect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        x_ticks: &[f64],
        y_ticks: &[f64],
        xtick_baseline_y: f32,
        ytick_right_x: f32,
        tick_size: f32,
        color: Color,
        dpi: f32,
    ) -> Result<()> {
        let dpi_scale = dpi / 100.0;
        // Character width is approximately 0.6 * font_size for most fonts
        let char_width_estimate = tick_size * 0.6;

        // Convert LayoutRect to tiny_skia Rect for border drawing
        let skia_plot_area = Rect::from_ltrb(
            plot_area.left,
            plot_area.top,
            plot_area.right,
            plot_area.bottom,
        )
        .ok_or(PlottingError::InvalidData {
            message: "Invalid plot area dimensions".to_string(),
            position: None,
        })?;

        // Format all tick labels with consistent precision
        let x_labels = format_tick_labels(x_ticks);
        let y_labels = format_tick_labels(y_ticks);

        // Draw X-axis tick labels using provided ticks
        for (tick_value, label_text) in x_ticks.iter().zip(x_labels.iter()) {
            let x_pixel = plot_area.left
                + (*tick_value - x_min) as f32 / (x_max - x_min) as f32 * plot_area.width();

            let text_width_estimate = label_text.len() as f32 * char_width_estimate / 2.0;
            let label_x = (x_pixel - text_width_estimate)
                .max(0.0)
                .min(self.width() as f32 - text_width_estimate * 2.0);
            self.draw_text(label_text, label_x, xtick_baseline_y, tick_size, color)?;
        }

        // Draw Y-axis tick labels using provided ticks
        for (tick_value, label_text) in y_ticks.iter().zip(y_labels.iter()) {
            let y_pixel = plot_area.bottom
                - (*tick_value - y_min) as f32 / (y_max - y_min) as f32 * plot_area.height();

            let text_width_estimate = label_text.len() as f32 * char_width_estimate;
            // Position so the right edge of the text is at ytick_right_x with a gap
            let gap = tick_size * 0.5; // Gap between labels and axis
            let min_x = tick_size * 0.5; // Minimum distance from left edge
            let label_x = (ytick_right_x - text_width_estimate - gap).max(min_x);
            // Center text vertically on tick: move up by half the text height
            // Text height is approximately tick_size * 1.2, so half is tick_size * 0.6
            let centered_y = y_pixel - tick_size * 0.5;
            self.draw_text(label_text, label_x, centered_y, tick_size, color)?;
        }

        // Draw border around plot area
        self.draw_plot_border(skia_plot_area, color, dpi_scale)?;

        Ok(())
    }

    /// Draw axis tick labels with categorical x-axis labels for bar charts
    ///
    /// Similar to `draw_axis_labels_at` but uses category names instead of numeric ticks
    /// on the x-axis. Categories are positioned at the center of each bar.
    ///
    /// Uses the same data-to-pixel mapping as bar rendering to ensure precise alignment.
    /// With bar chart x-range [-0.5, n-0.5], category i maps to position i in data space.
    pub fn draw_axis_labels_at_categorical(
        &mut self,
        plot_area: &LayoutRect,
        categories: &[String],
        y_min: f64,
        y_max: f64,
        y_ticks: &[f64],
        xtick_baseline_y: f32,
        ytick_right_x: f32,
        tick_size: f32,
        color: Color,
        dpi: f32,
    ) -> Result<()> {
        let dpi_scale = dpi / 100.0;
        let char_width_estimate = tick_size * 0.6;

        // Convert LayoutRect to tiny_skia Rect for border drawing
        let skia_plot_area = Rect::from_ltrb(
            plot_area.left,
            plot_area.top,
            plot_area.right,
            plot_area.bottom,
        )
        .ok_or(PlottingError::InvalidData {
            message: "Invalid plot area dimensions".to_string(),
            position: None,
        })?;

        // Draw X-axis category labels using same data-to-pixel mapping as bars
        let n_categories = categories.len();
        if n_categories > 0 {
            // X-axis range with matplotlib-compatible padding: [-0.5, n-0.5]
            let x_min = -0.5_f64;
            let x_max = n_categories as f64 - 0.5;
            let x_range = x_max - x_min;

            for (i, category) in categories.iter().enumerate() {
                // Position label at category index (same as bar center in data space)
                let x_data = i as f64;
                let x_center =
                    plot_area.left + ((x_data - x_min) / x_range) as f32 * plot_area.width();

                // Estimate text width for centering
                let text_width_estimate = category.len() as f32 * char_width_estimate;
                let label_x = (x_center - text_width_estimate / 2.0)
                    .max(0.0)
                    .min(self.width() as f32 - text_width_estimate);

                self.draw_text(category, label_x, xtick_baseline_y, tick_size, color)?;
            }
        }

        // Format Y-axis tick labels with consistent precision
        let y_labels = format_tick_labels(y_ticks);

        // Draw Y-axis tick labels using provided ticks
        for (tick_value, label_text) in y_ticks.iter().zip(y_labels.iter()) {
            let y_pixel = plot_area.bottom
                - (*tick_value - y_min) as f32 / (y_max - y_min) as f32 * plot_area.height();

            let text_width_estimate = label_text.len() as f32 * char_width_estimate;
            let gap = tick_size * 0.5;
            let min_x = tick_size * 0.5;
            let label_x = (ytick_right_x - text_width_estimate - gap).max(min_x);
            // Center text vertically on tick
            let centered_y = y_pixel - tick_size * 0.5;
            self.draw_text(label_text, label_x, centered_y, tick_size, color)?;
        }

        // Draw border around plot area
        self.draw_plot_border(skia_plot_area, color, dpi_scale)?;

        Ok(())
    }

    /// Draw axis labels for violin/distribution plots with categorical x-axis
    ///
    /// Unlike bar charts which use integer positions (0, 1, 2, ...), violin plots
    /// use arbitrary x-positions (e.g., 0.5 for a single violin). This method
    /// draws category labels at the actual x-positions within the data range.
    ///
    /// # Arguments
    /// * `plot_area` - The computed plot area
    /// * `categories` - Category labels to draw
    /// * `x_positions` - X positions for each category in data space
    /// * `x_min` - Minimum x value (data space)
    /// * `x_max` - Maximum x value (data space)
    /// * `y_min`, `y_max` - Y data range
    /// * `y_ticks` - Y-axis tick values
    /// * Other arguments for positioning and styling
    #[allow(clippy::too_many_arguments)]
    pub fn draw_axis_labels_at_categorical_violin(
        &mut self,
        plot_area: &LayoutRect,
        categories: &[String],
        x_positions: &[f64],
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        y_ticks: &[f64],
        xtick_baseline_y: f32,
        ytick_right_x: f32,
        tick_size: f32,
        color: Color,
        dpi: f32,
    ) -> Result<()> {
        let dpi_scale = dpi / 100.0;
        let char_width_estimate = tick_size * 0.6;

        // Convert LayoutRect to tiny_skia Rect for border drawing
        let skia_plot_area = Rect::from_ltrb(
            plot_area.left,
            plot_area.top,
            plot_area.right,
            plot_area.bottom,
        )
        .ok_or(PlottingError::InvalidData {
            message: "Invalid plot area dimensions".to_string(),
            position: None,
        })?;

        // Draw X-axis category labels at specified positions
        let x_range = x_max - x_min;
        if x_range > 0.0 {
            for (category, &x_pos) in categories.iter().zip(x_positions.iter()) {
                // Map data position to pixel position
                let x_center =
                    plot_area.left + ((x_pos - x_min) / x_range) as f32 * plot_area.width();

                // Estimate text width for centering
                let text_width_estimate = category.len() as f32 * char_width_estimate;
                let label_x = (x_center - text_width_estimate / 2.0)
                    .max(0.0)
                    .min(self.width() as f32 - text_width_estimate);

                self.draw_text(category, label_x, xtick_baseline_y, tick_size, color)?;
            }
        }

        // Format Y-axis tick labels with consistent precision
        let y_labels = format_tick_labels(y_ticks);

        // Draw Y-axis tick labels using provided ticks
        for (tick_value, label_text) in y_ticks.iter().zip(y_labels.iter()) {
            let y_pixel = plot_area.bottom
                - (*tick_value - y_min) as f32 / (y_max - y_min) as f32 * plot_area.height();

            let text_width_estimate = label_text.len() as f32 * char_width_estimate;
            let gap = tick_size * 0.5;
            let min_x = tick_size * 0.5;
            let label_x = (ytick_right_x - text_width_estimate - gap).max(min_x);
            // Center text vertically on tick
            let centered_y = y_pixel - tick_size * 0.5;
            self.draw_text(label_text, label_x, centered_y, tick_size, color)?;
        }

        // Draw border around plot area
        self.draw_plot_border(skia_plot_area, color, dpi_scale)?;

        Ok(())
    }

    /// Draw title with DPI scale (legacy compatibility)
    ///
    /// This method uses a hardcoded offset for backward compatibility.
    /// Prefer `draw_title` with `SpacingConfig` for new code.
    pub fn draw_title_legacy(
        &mut self,
        title: &str,
        plot_area: Rect,
        color: Color,
        title_size: f32,
        dpi_scale: f32,
    ) -> Result<()> {
        let title_offset = 30.0 * dpi_scale;
        let canvas_center_x = self.width() as f32 / 2.0;
        let title_y = (plot_area.top() - title_offset).max(title_size + 5.0);
        self.draw_text_centered(title, canvas_center_x, title_y, title_size, color)
    }

    /// Draw legend
    pub fn draw_legend(&mut self, legend_items: &[(String, Color)], plot_area: Rect) -> Result<()> {
        if legend_items.is_empty() {
            return Ok(());
        }

        let legend_size = 12.0;
        let legend_spacing = 20.0;
        let legend_x = plot_area.right() - 150.0;
        let mut legend_y = plot_area.top() + 30.0;

        // Draw legend background (simple rectangle)
        let legend_bg = Rect::from_xywh(
            legend_x - 10.0,
            legend_y - 15.0,
            140.0,
            legend_items.len() as f32 * legend_spacing + 10.0,
        )
        .ok_or(PlottingError::InvalidData {
            message: "Invalid legend dimensions".to_string(),
            position: None,
        })?;

        self.draw_rectangle(
            legend_bg.left(),
            legend_bg.top(),
            legend_bg.width(),
            legend_bg.height(),
            Color::new_rgba(255, 255, 255, 200),
            true,
        )?;

        // Draw legend items
        for (label, color) in legend_items {
            // Draw color square
            let color_rect = Rect::from_xywh(legend_x, legend_y - 8.0, 12.0, 12.0).ok_or(
                PlottingError::InvalidData {
                    message: "Invalid legend item dimensions".to_string(),
                    position: None,
                },
            )?;
            self.draw_rectangle(
                color_rect.left(),
                color_rect.top(),
                color_rect.width(),
                color_rect.height(),
                *color,
                true,
            )?;

            // Draw label text
            self.draw_text(
                label,
                legend_x + 20.0,
                legend_y,
                legend_size,
                Color::new_rgba(0, 0, 0, 255),
            )?;

            legend_y += legend_spacing;
        }

        Ok(())
    }

    /// Draw legend with configurable position
    pub fn draw_legend_positioned(
        &mut self,
        legend_items: &[(String, Color)],
        plot_area: Rect,
        position: crate::core::Position,
    ) -> Result<()> {
        if legend_items.is_empty() {
            return Ok(());
        }

        let legend_size = 12.0;
        let legend_spacing = 20.0;
        let legend_width = 140.0;
        let legend_height = legend_items.len() as f32 * legend_spacing + 10.0;

        // Calculate legend position based on position enum
        let center_x = plot_area.left() + plot_area.width() / 2.0;
        let center_y = plot_area.top() + plot_area.height() / 2.0;

        let (legend_x, legend_y) = match position {
            // Best defaults to TopRight in legacy method; full best positioning in draw_legend_full
            crate::core::Position::Best | crate::core::Position::TopRight => (
                plot_area.right() - legend_width - 10.0,
                plot_area.top() + 10.0,
            ),
            crate::core::Position::TopLeft => (plot_area.left() + 10.0, plot_area.top() + 10.0),
            crate::core::Position::TopCenter => {
                (center_x - legend_width / 2.0, plot_area.top() + 10.0)
            }
            crate::core::Position::CenterLeft => {
                (plot_area.left() + 10.0, center_y - legend_height / 2.0)
            }
            crate::core::Position::Center => (
                center_x - legend_width / 2.0,
                center_y - legend_height / 2.0,
            ),
            crate::core::Position::CenterRight => (
                plot_area.right() - legend_width - 10.0,
                center_y - legend_height / 2.0,
            ),
            crate::core::Position::BottomLeft => (
                plot_area.left() + 10.0,
                plot_area.bottom() - legend_height - 10.0,
            ),
            crate::core::Position::BottomCenter => (
                center_x - legend_width / 2.0,
                plot_area.bottom() - legend_height - 10.0,
            ),
            crate::core::Position::BottomRight => (
                plot_area.right() - legend_width - 10.0,
                plot_area.bottom() - legend_height - 10.0,
            ),
            crate::core::Position::Custom { x, y } => (x, y),
        };

        // Draw legend background (simple rectangle)
        let legend_bg =
            Rect::from_xywh(legend_x - 10.0, legend_y - 5.0, legend_width, legend_height).ok_or(
                PlottingError::InvalidData {
                    message: "Invalid legend dimensions".to_string(),
                    position: None,
                },
            )?;

        self.draw_rectangle(
            legend_bg.left(),
            legend_bg.top(),
            legend_bg.width(),
            legend_bg.height(),
            Color::new_rgba(255, 255, 255, 200),
            true,
        )?;

        // Draw legend items
        let mut item_y = legend_y + 10.0;
        for (label, color) in legend_items {
            // Draw color square
            let color_rect = Rect::from_xywh(legend_x, item_y - 8.0, 12.0, 12.0).ok_or(
                PlottingError::InvalidData {
                    message: "Invalid legend item dimensions".to_string(),
                    position: None,
                },
            )?;
            self.draw_rectangle(
                color_rect.left(),
                color_rect.top(),
                color_rect.width(),
                color_rect.height(),
                *color,
                true,
            )?;

            // Draw label text
            self.draw_text(
                label,
                legend_x + 20.0,
                item_y,
                legend_size,
                Color::new_rgba(0, 0, 0, 255),
            )?;

            item_y += legend_spacing;
        }

        Ok(())
    }

    // =========================================================================
    // New Legend System with proper handle rendering
    // =========================================================================

    /// Draw a line handle in the legend (for line series)
    ///
    /// Draws a horizontal line segment with the specified style, color, and width.
    fn draw_legend_line_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        color: Color,
        style: &LineStyle,
        width: f32,
    ) -> Result<()> {
        // Draw horizontal line at vertical center
        self.draw_line(x, y, x + length, y, color, width, style.clone())
    }

    /// Draw a scatter/marker handle in the legend
    ///
    /// Draws a single marker symbol centered in the handle area.
    fn draw_legend_scatter_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        color: Color,
        marker: &MarkerStyle,
        size: f32,
    ) -> Result<()> {
        // Draw marker at center of handle area
        let center_x = x + length / 2.0;
        self.draw_marker(center_x, y, size, *marker, color)
    }

    /// Draw a bar handle in the legend
    ///
    /// Draws a filled rectangle to represent bar/histogram series.
    fn draw_legend_bar_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        height: f32,
        color: Color,
    ) -> Result<()> {
        // Draw filled rectangle centered vertically
        let rect_y = y - height / 2.0;
        self.draw_rectangle(x, rect_y, length, height, color, true)
    }

    /// Draw a line+marker handle in the legend
    ///
    /// Draws a line segment with a marker symbol at the center.
    fn draw_legend_line_marker_handle(
        &mut self,
        x: f32,
        y: f32,
        length: f32,
        color: Color,
        line_style: &LineStyle,
        line_width: f32,
        marker: &MarkerStyle,
        marker_size: f32,
    ) -> Result<()> {
        // Draw line first
        self.draw_legend_line_handle(x, y, length, color, line_style, line_width)?;
        // Draw marker on top at center
        self.draw_legend_scatter_handle(x, y, length, color, marker, marker_size)
    }

    /// Draw a legend handle based on the item type
    fn draw_legend_handle(
        &mut self,
        item: &LegendItem,
        x: f32,
        y: f32,
        spacing: &LegendSpacingPixels,
    ) -> Result<()> {
        let handle_length = spacing.handle_length;
        let handle_height = spacing.handle_height;

        // First draw the base type
        match &item.item_type {
            LegendItemType::Line { style, width } => {
                self.draw_legend_line_handle(x, y, handle_length, item.color, style, *width)?;
            }
            LegendItemType::Scatter { marker, size } => {
                self.draw_legend_scatter_handle(x, y, handle_length, item.color, marker, *size)?;
            }
            LegendItemType::LineMarker {
                line_style,
                line_width,
                marker,
                marker_size,
            } => {
                self.draw_legend_line_marker_handle(
                    x,
                    y,
                    handle_length,
                    item.color,
                    line_style,
                    *line_width,
                    marker,
                    *marker_size,
                )?;
            }
            LegendItemType::Bar | LegendItemType::Histogram => {
                self.draw_legend_bar_handle(x, y, handle_length, handle_height, item.color)?;
            }
            LegendItemType::Area { edge_color } => {
                // Draw filled rectangle with optional edge
                self.draw_legend_bar_handle(x, y, handle_length, handle_height, item.color)?;
                if let Some(edge) = edge_color {
                    // Draw edge around the rectangle
                    let rect_y = y - handle_height / 2.0;
                    self.draw_rectangle_outline(
                        x,
                        rect_y,
                        handle_length,
                        handle_height,
                        *edge,
                        1.0,
                    )?;
                }
            }
            LegendItemType::ErrorBar => {
                // ErrorBar type: Draw vertical error bar with marker (matplotlib-style)
                let center_x = x + handle_length / 2.0;
                let error_height = handle_height * 0.8;
                let half_error = error_height / 2.0;
                let cap_width = handle_height * 0.5;
                let half_cap = cap_width / 2.0;

                // Vertical error bar line
                self.draw_line(
                    center_x,
                    y - half_error,
                    center_x,
                    y + half_error,
                    item.color,
                    1.5,
                    LineStyle::Solid,
                )?;
                // Top cap (horizontal)
                self.draw_line(
                    center_x - half_cap,
                    y - half_error,
                    center_x + half_cap,
                    y - half_error,
                    item.color,
                    1.5,
                    LineStyle::Solid,
                )?;
                // Bottom cap (horizontal)
                self.draw_line(
                    center_x - half_cap,
                    y + half_error,
                    center_x + half_cap,
                    y + half_error,
                    item.color,
                    1.5,
                    LineStyle::Solid,
                )?;
                // Draw marker in center
                self.draw_marker(
                    center_x,
                    y,
                    handle_height * 0.4,
                    MarkerStyle::Circle,
                    item.color,
                )?;
            }
        }

        // If the series has attached error bars (not ErrorBar type), overlay error bar indicator
        if item.has_error_bars && !matches!(item.item_type, LegendItemType::ErrorBar) {
            let center_x = x + handle_length / 2.0;
            let error_height = handle_height * 0.7; // Slightly smaller for overlay
            let half_error = error_height / 2.0;
            let cap_width = handle_height * 0.4;
            let half_cap = cap_width / 2.0;

            // Vertical error bar line
            self.draw_line(
                center_x,
                y - half_error,
                center_x,
                y + half_error,
                item.color,
                1.0,
                LineStyle::Solid,
            )?;
            // Top cap (horizontal)
            self.draw_line(
                center_x - half_cap,
                y - half_error,
                center_x + half_cap,
                y - half_error,
                item.color,
                1.0,
                LineStyle::Solid,
            )?;
            // Bottom cap (horizontal)
            self.draw_line(
                center_x - half_cap,
                y + half_error,
                center_x + half_cap,
                y + half_error,
                item.color,
                1.0,
                LineStyle::Solid,
            )?;
        }

        Ok(())
    }

    /// Draw rectangle outline (stroke only, no fill)
    fn draw_rectangle_outline(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        line_width: f32,
    ) -> Result<()> {
        // Draw 4 lines forming a rectangle
        let x2 = x + width;
        let y2 = y + height;
        self.draw_line(x, y, x2, y, color, line_width, LineStyle::Solid)?;
        self.draw_line(x2, y, x2, y2, color, line_width, LineStyle::Solid)?;
        self.draw_line(x2, y2, x, y2, color, line_width, LineStyle::Solid)?;
        self.draw_line(x, y2, x, y, color, line_width, LineStyle::Solid)
    }

    /// Draw rounded rectangle outline (stroke only, no fill)
    fn draw_rounded_rectangle_outline(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        corner_radius: f32,
        color: Color,
        line_width: f32,
    ) -> Result<()> {
        // Clamp radius to half of the smaller dimension
        let max_radius = (width.min(height) / 2.0).max(0.0);
        let radius = corner_radius.min(max_radius);

        // If radius is effectively zero, use regular rectangle outline
        if radius < 0.1 {
            return self.draw_rectangle_outline(x, y, width, height, color, line_width);
        }

        // Build rounded rectangle path
        let mut pb = PathBuilder::new();

        pb.move_to(x + radius, y);
        pb.line_to(x + width - radius, y);
        pb.quad_to(x + width, y, x + width, y + radius);
        pb.line_to(x + width, y + height - radius);
        pb.quad_to(x + width, y + height, x + width - radius, y + height);
        pb.line_to(x + radius, y + height);
        pb.quad_to(x, y + height, x, y + height - radius);
        pb.line_to(x, y + radius);
        pb.quad_to(x, y, x + radius, y);
        pb.close();

        let path = pb.finish().ok_or(PlottingError::RenderError(
            "Failed to create rounded rectangle outline path".to_string(),
        ))?;

        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;

        let stroke = Stroke {
            width: line_width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };

        self.pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);

        Ok(())
    }

    /// Draw legend frame with background and optional border
    fn draw_legend_frame(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        style: &LegendStyle,
    ) -> Result<()> {
        if !style.visible {
            return Ok(());
        }

        let radius = style.effective_corner_radius();

        // Draw shadow if enabled
        if style.shadow {
            let (shadow_dx, shadow_dy) = style.shadow_offset;
            if radius > 0.0 {
                self.draw_rounded_rectangle(
                    x + shadow_dx,
                    y + shadow_dy,
                    width,
                    height,
                    radius,
                    style.shadow_color,
                    true,
                )?;
            } else {
                self.draw_rectangle(
                    x + shadow_dx,
                    y + shadow_dy,
                    width,
                    height,
                    style.shadow_color,
                    true,
                )?;
            }
        }

        // Draw background with alpha applied
        let face_color = style.effective_face_color();
        if radius > 0.0 {
            self.draw_rounded_rectangle(x, y, width, height, radius, face_color, true)?;
        } else {
            self.draw_rectangle(x, y, width, height, face_color, true)?;
        }

        // Draw border if specified
        if let Some(edge_color) = style.edge_color {
            if radius > 0.0 {
                self.draw_rounded_rectangle_outline(
                    x,
                    y,
                    width,
                    height,
                    radius,
                    edge_color,
                    style.border_width,
                )?;
            } else {
                self.draw_rectangle_outline(x, y, width, height, edge_color, style.border_width)?;
            }
        }

        Ok(())
    }

    /// Calculate legend dimensions from items
    fn calculate_legend_dimensions(
        &self,
        items: &[LegendItem],
        legend: &Legend,
        char_width: f32,
    ) -> (f32, f32) {
        legend.calculate_size(items, char_width)
    }

    /// Draw legend with full LegendItem support
    ///
    /// This is the new legend drawing method that properly renders different
    /// series types with their correct visual handles.
    pub fn draw_legend_full(
        &mut self,
        items: &[LegendItem],
        legend: &Legend,
        plot_area: Rect,
        data_bboxes: Option<&[(f32, f32, f32, f32)]>,
    ) -> Result<()> {
        if items.is_empty() || !legend.enabled {
            return Ok(());
        }

        let spacing = legend.spacing.to_pixels(legend.font_size);

        // Estimate character width for size calculation
        let char_width = legend.font_size * 0.6;

        // Calculate legend size
        let (legend_width, legend_height) =
            self.calculate_legend_dimensions(items, legend, char_width);

        // Determine position
        let plot_bounds = (
            plot_area.left(),
            plot_area.top(),
            plot_area.right(),
            plot_area.bottom(),
        );

        let position = if matches!(legend.position, LegendPosition::Best) {
            // Use best position algorithm
            let bboxes = data_bboxes.unwrap_or(&[]);
            if bboxes.iter().map(|b| 1).sum::<usize>() > 100000 {
                // Performance guard: skip for very large datasets
                LegendPosition::UpperRight
            } else {
                find_best_position(
                    (legend_width, legend_height),
                    plot_bounds,
                    bboxes,
                    &legend.spacing,
                    legend.font_size,
                )
            }
        } else {
            legend.position
        };

        // Create a temporary legend with the resolved position to calculate coordinates
        let resolved_legend = Legend {
            position,
            ..legend.clone()
        };

        let (legend_x, legend_y) =
            resolved_legend.calculate_position((legend_width, legend_height), plot_bounds);

        // Draw frame
        self.draw_legend_frame(
            legend_x,
            legend_y,
            legend_width,
            legend_height,
            &legend.style,
        )?;

        // Starting position for items (inside padding)
        let item_x = legend_x + spacing.border_pad;
        let mut item_y = legend_y + spacing.border_pad + legend.font_size / 2.0;

        // Draw title if present
        if let Some(ref title) = legend.title {
            let title_x = legend_x + legend_width / 2.0;
            self.draw_text_centered(title, title_x, item_y, legend.font_size, legend.text_color)?;
            item_y += legend.font_size + spacing.label_spacing;
        }

        // Calculate items per column
        let items_per_col = items.len().div_ceil(legend.columns);

        // Calculate column width
        let max_label_len = items.iter().map(|item| item.label.len()).max().unwrap_or(0);
        let label_width = max_label_len as f32 * char_width;
        let col_width = spacing.handle_length + spacing.handle_text_pad + label_width;

        // Draw items column by column
        for col in 0..legend.columns {
            let col_x = item_x + col as f32 * (col_width + spacing.column_spacing);
            let mut row_y = item_y;

            for row in 0..items_per_col {
                let idx = col * items_per_col + row;
                if idx >= items.len() {
                    break;
                }

                let item = &items[idx];

                // Draw handle
                self.draw_legend_handle(item, col_x, row_y, &spacing)?;

                // Draw label - vertically centered with handle
                let text_x = col_x + spacing.handle_length + spacing.handle_text_pad;
                // Center text vertically on handle
                let centered_y = row_y - legend.font_size * 0.65;
                self.draw_text(
                    &item.label,
                    text_x,
                    centered_y,
                    legend.font_size,
                    legend.text_color,
                )?;

                row_y += legend.font_size + spacing.label_spacing;
            }
        }

        Ok(())
    }

    /// Draw a colorbar for heatmaps
    ///
    /// Draws a vertical gradient bar showing the color mapping from vmin to vmax,
    /// with tick marks and optional label.
    ///
    /// # Arguments
    ///
    /// * `colormap` - The color map to sample from
    /// * `vmin` - Minimum value in the data range
    /// * `vmax` - Maximum value in the data range
    /// * `x` - X position of colorbar (left edge)
    /// * `y` - Y position of colorbar (top edge)
    /// * `width` - Width of the colorbar
    /// * `height` - Height of the colorbar
    /// * `label` - Optional label to display (rotated 90°)
    /// * `foreground_color` - Color for ticks, text, and border
    /// * `tick_font_size` - Font size for tick labels (in points)
    /// * `label_font_size` - Font size for colorbar label (in points, optional)
    pub fn draw_colorbar(
        &mut self,
        colormap: &crate::render::ColorMap,
        vmin: f64,
        vmax: f64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        label: Option<&str>,
        foreground_color: Color,
        tick_font_size: f32,
        label_font_size: Option<f32>,
    ) -> Result<()> {
        // Use tick font size for label if not specified separately
        let label_font_size = label_font_size.unwrap_or(tick_font_size * 1.1);

        // Draw the colorbar gradient (vertical, from vmax at top to vmin at bottom)
        // Use one segment per pixel row to eliminate anti-aliasing artifacts
        let num_segments = (height as usize).max(50);
        let segment_height = height / num_segments as f32;

        for i in 0..num_segments {
            // Map segment to value (top = vmax, bottom = vmin)
            let normalized = 1.0 - (i as f64 / (num_segments - 1).max(1) as f64);
            let color = colormap.sample(normalized);
            let segment_y = y + i as f32 * segment_height;

            // Use solid rectangle with small overlap to ensure seamless gradient
            // draw_solid_rectangle has 100% opacity and no anti-aliasing
            self.draw_solid_rectangle(x, segment_y, width, segment_height + 0.5, color)?;
        }

        // Draw border around colorbar
        let stroke_width = 1.0;
        self.draw_rectangle(x, y, width, height, foreground_color, false)?;

        // Generate nice tick values using tick formatter
        let ticks = generate_ticks(vmin, vmax, 6);
        let tick_width = width * 0.3;
        let text_offset = width + tick_font_size * 0.5;

        for &value in &ticks {
            // Map value to Y position (top = vmax, bottom = vmin)
            let t = (value - vmin) / (vmax - vmin);
            let tick_y = y + height * (1.0 - t as f32);

            // Draw tick mark
            self.draw_line(
                x + width,
                tick_y,
                x + width + tick_width,
                tick_y,
                foreground_color,
                stroke_width,
                LineStyle::Solid,
            )?;

            // Draw value label using unified TickFormatter
            let label_text = format_tick_label(value);

            self.draw_text(
                &label_text,
                x + text_offset,
                tick_y + tick_font_size * 0.3,
                tick_font_size,
                foreground_color,
            )?;
        }

        // Draw colorbar label (rotated 90 degrees) if provided
        if let Some(label) = label {
            let label_x = x + width + tick_font_size * 4.0;
            let label_y = y + height / 2.0;
            self.draw_text_rotated(label, label_x, label_y, label_font_size, foreground_color)?;
        }

        Ok(())
    }

    /// Consume the renderer and convert to an Image
    pub fn into_image(self) -> Image {
        Image {
            width: self.width,
            height: self.height,
            pixels: self.pixmap.data().to_vec(),
        }
    }

    /// Save the current pixmap as PNG
    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.pixmap
            .save_png(path)
            .map_err(|_| PlottingError::IoError(std::io::Error::other("Failed to save PNG")))
    }

    /// Export as SVG (simplified - tiny-skia doesn't directly support SVG export)
    pub fn export_svg<P: AsRef<Path>>(&self, path: P, width: u32, height: u32) -> Result<()> {
        // For now, create a basic SVG placeholder
        // In a real implementation, we'd need to track draw commands and convert to SVG
        let svg_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
  <rect width="100%" height="100%" fill="{}"/>
  <text x="50%" y="50%" text-anchor="middle" font-family="Arial" font-size="16">
    Ruviz Plot ({} x {})
  </text>
</svg>"#,
            width, height, self.theme.background, width, height
        );

        std::fs::write(path, svg_content).map_err(PlottingError::IoError)?;

        Ok(())
    }

    /// Get the width of the renderer
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the height of the renderer  
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Draw a subplot image at the specified position
    pub fn draw_subplot(
        &mut self,
        subplot_image: crate::core::plot::Image,
        x: u32,
        y: u32,
    ) -> Result<()> {
        // Convert our Image struct to tiny-skia Pixmap for drawing
        let subplot_pixmap = tiny_skia::Pixmap::from_vec(
            subplot_image.pixels,
            tiny_skia::IntSize::from_wh(subplot_image.width, subplot_image.height).ok_or_else(
                || PlottingError::InvalidInput("Invalid subplot dimensions".to_string()),
            )?,
        )
        .ok_or_else(|| PlottingError::RenderError("Failed to create subplot pixmap".to_string()))?;

        // Draw the subplot pixmap onto our main pixmap at the specified position
        self.pixmap.draw_pixmap(
            x as i32,
            y as i32,
            subplot_pixmap.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::identity(),
            None,
        );

        Ok(())
    }

    // ========== Annotation Rendering Methods ==========

    /// Render all annotations for a plot
    ///
    /// Annotations are rendered after data series but before legend and title.
    pub fn draw_annotations(
        &mut self,
        annotations: &[crate::core::Annotation],
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        dpi: f32,
    ) -> Result<()> {
        for annotation in annotations {
            self.draw_annotation(annotation, plot_area, x_min, x_max, y_min, y_max, dpi)?;
        }
        Ok(())
    }

    /// Render a single annotation
    fn draw_annotation(
        &mut self,
        annotation: &crate::core::Annotation,
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        dpi: f32,
    ) -> Result<()> {
        use crate::core::Annotation;

        match annotation {
            Annotation::Text { x, y, text, style } => self.draw_annotation_text(
                *x, *y, text, style, plot_area, x_min, x_max, y_min, y_max, dpi,
            ),
            Annotation::Arrow {
                x1,
                y1,
                x2,
                y2,
                style,
            } => self.draw_annotation_arrow(
                *x1, *y1, *x2, *y2, style, plot_area, x_min, x_max, y_min, y_max, dpi,
            ),
            Annotation::HLine {
                y,
                style,
                color,
                width,
            } => {
                self.draw_annotation_hline(*y, style, *color, *width, plot_area, y_min, y_max, dpi)
            }
            Annotation::VLine {
                x,
                style,
                color,
                width,
            } => {
                self.draw_annotation_vline(*x, style, *color, *width, plot_area, x_min, x_max, dpi)
            }
            Annotation::Rectangle {
                x,
                y,
                width,
                height,
                style,
            } => self.draw_annotation_rect(
                *x, *y, *width, *height, style, plot_area, x_min, x_max, y_min, y_max,
            ),
            Annotation::FillBetween {
                x,
                y1,
                y2,
                style,
                where_positive,
            } => self.draw_annotation_fill_between(
                x,
                y1,
                y2,
                style,
                *where_positive,
                plot_area,
                x_min,
                x_max,
                y_min,
                y_max,
            ),
            Annotation::HSpan {
                x_min: xmin,
                x_max: xmax,
                style,
            } => self
                .draw_annotation_hspan(*xmin, *xmax, style, plot_area, x_min, x_max, y_min, y_max),
            Annotation::VSpan {
                y_min: ymin,
                y_max: ymax,
                style,
            } => self
                .draw_annotation_vspan(*ymin, *ymax, style, plot_area, x_min, x_max, y_min, y_max),
        }
    }

    /// Draw a text annotation at data coordinates
    fn draw_annotation_text(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        style: &crate::core::TextStyle,
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        dpi: f32,
    ) -> Result<()> {
        let (px, py) = map_data_to_pixels(x, y, x_min, x_max, y_min, y_max, plot_area);

        // Convert font size from points to pixels
        let font_size_px = pt_to_px(style.font_size, dpi);

        // Draw text at the position (could add background box and rotation in future)
        self.draw_text(text, px, py, font_size_px, style.color)
    }

    /// Draw an arrow annotation
    fn draw_annotation_arrow(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        style: &crate::core::ArrowStyle,
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        dpi: f32,
    ) -> Result<()> {
        let (px1, py1) = map_data_to_pixels(x1, y1, x_min, x_max, y_min, y_max, plot_area);
        let (px2, py2) = map_data_to_pixels(x2, y2, x_min, x_max, y_min, y_max, plot_area);

        let line_width_px = pt_to_px(style.line_width, dpi);

        // Draw the arrow shaft
        self.draw_line(
            px1,
            py1,
            px2,
            py2,
            style.color,
            line_width_px,
            style.line_style.clone(),
        )?;

        // Draw arrow head at end point
        if !matches!(style.head_style, crate::core::ArrowHead::None) {
            let head_length_px = pt_to_px(style.head_length, dpi);
            let head_width_px = pt_to_px(style.head_width, dpi);
            self.draw_arrow_head(
                px2,
                py2,
                px1,
                py1,
                head_length_px,
                head_width_px,
                style.color,
            )?;
        }

        // Draw arrow head at start point (for double-headed arrows)
        if !matches!(style.tail_style, crate::core::ArrowHead::None) {
            let head_length_px = pt_to_px(style.head_length, dpi);
            let head_width_px = pt_to_px(style.head_width, dpi);
            self.draw_arrow_head(
                px1,
                py1,
                px2,
                py2,
                head_length_px,
                head_width_px,
                style.color,
            )?;
        }

        Ok(())
    }

    /// Draw an arrow head pointing from (from_x, from_y) to (tip_x, tip_y)
    fn draw_arrow_head(
        &mut self,
        tip_x: f32,
        tip_y: f32,
        from_x: f32,
        from_y: f32,
        length: f32,
        width: f32,
        color: Color,
    ) -> Result<()> {
        // Calculate direction vector
        let dx = tip_x - from_x;
        let dy = tip_y - from_y;
        let len = (dx * dx + dy * dy).sqrt();

        if len < 0.001 {
            return Ok(());
        }

        // Normalize direction
        let ux = dx / len;
        let uy = dy / len;

        // Perpendicular vector
        let px = -uy;
        let py = ux;

        // Calculate arrow head vertices
        let base_x = tip_x - ux * length;
        let base_y = tip_y - uy * length;

        let left_x = base_x + px * width / 2.0;
        let left_y = base_y + py * width / 2.0;

        let right_x = base_x - px * width / 2.0;
        let right_y = base_y - py * width / 2.0;

        // Draw filled triangle
        let mut path = PathBuilder::new();
        path.move_to(tip_x, tip_y);
        path.line_to(left_x, left_y);
        path.line_to(right_x, right_y);
        path.close();

        let path = path.finish().ok_or(PlottingError::RenderError(
            "Failed to create arrow head path".to_string(),
        ))?;

        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;

        self.pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );

        Ok(())
    }

    /// Draw a horizontal reference line
    fn draw_annotation_hline(
        &mut self,
        y: f64,
        style: &LineStyle,
        color: Color,
        width: f32,
        plot_area: Rect,
        y_min: f64,
        y_max: f64,
        dpi: f32,
    ) -> Result<()> {
        // Calculate pixel y position
        let frac = (y - y_min) / (y_max - y_min);
        let py = plot_area.bottom() - frac as f32 * plot_area.height();

        let line_width_px = pt_to_px(width, dpi);

        // Draw horizontal line spanning the plot area
        self.draw_line(
            plot_area.left(),
            py,
            plot_area.right(),
            py,
            color,
            line_width_px,
            style.clone(),
        )
    }

    /// Draw a vertical reference line
    fn draw_annotation_vline(
        &mut self,
        x: f64,
        style: &LineStyle,
        color: Color,
        width: f32,
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        dpi: f32,
    ) -> Result<()> {
        // Calculate pixel x position
        let frac = (x - x_min) / (x_max - x_min);
        let px = plot_area.left() + frac as f32 * plot_area.width();

        let line_width_px = pt_to_px(width, dpi);

        // Draw vertical line spanning the plot area
        self.draw_line(
            px,
            plot_area.top(),
            px,
            plot_area.bottom(),
            color,
            line_width_px,
            style.clone(),
        )
    }

    /// Draw a rectangle annotation
    fn draw_annotation_rect(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        style: &crate::core::ShapeStyle,
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<()> {
        let (px1, py1) = map_data_to_pixels(x, y + height, x_min, x_max, y_min, y_max, plot_area);
        let (px2, py2) = map_data_to_pixels(x + width, y, x_min, x_max, y_min, y_max, plot_area);

        let rect_width = (px2 - px1).abs();
        let rect_height = (py2 - py1).abs();
        let rect_x = px1.min(px2);
        let rect_y = py1.min(py2);

        if let Some(rect) = Rect::from_xywh(rect_x, rect_y, rect_width, rect_height) {
            // Draw fill if specified
            if let Some(fill_color) = &style.fill_color {
                let mut paint = Paint::default();
                let color_with_alpha = fill_color.with_alpha(style.fill_alpha);
                paint.set_color(color_with_alpha.to_tiny_skia_color());
                paint.anti_alias = true;

                self.pixmap
                    .fill_rect(rect, &paint, Transform::identity(), None);
            }

            // Draw edge if specified
            if let Some(edge_color) = &style.edge_color {
                let mut paint = Paint::default();
                paint.set_color(edge_color.to_tiny_skia_color());
                paint.anti_alias = true;

                let mut stroke = Stroke {
                    width: style.edge_width.max(0.1),
                    ..Stroke::default()
                };

                if let Some(dash_pattern) = style.edge_style.to_dash_array() {
                    stroke.dash = StrokeDash::new(dash_pattern, 0.0);
                }

                let mut path = PathBuilder::new();
                path.push_rect(rect);
                if let Some(path) = path.finish() {
                    self.pixmap
                        .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
                }
            }
        }

        Ok(())
    }

    /// Draw a fill between two curves
    fn draw_annotation_fill_between(
        &mut self,
        x: &[f64],
        y1: &[f64],
        y2: &[f64],
        style: &crate::core::FillStyle,
        where_positive: bool,
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<()> {
        if x.len() < 2 || x.len() != y1.len() || x.len() != y2.len() {
            return Ok(()); // Nothing to draw
        }

        // Build polygon path
        let mut path = PathBuilder::new();

        // Forward along y1
        let (start_x, start_y) =
            map_data_to_pixels(x[0], y1[0], x_min, x_max, y_min, y_max, plot_area);
        path.move_to(start_x, start_y);

        for i in 1..x.len() {
            if !where_positive || y1[i] >= y2[i] {
                let (px, py) =
                    map_data_to_pixels(x[i], y1[i], x_min, x_max, y_min, y_max, plot_area);
                path.line_to(px, py);
            } else {
                let (px, py) =
                    map_data_to_pixels(x[i], y2[i], x_min, x_max, y_min, y_max, plot_area);
                path.line_to(px, py);
            }
        }

        // Backward along y2 (in reverse order)
        for i in (0..x.len()).rev() {
            if !where_positive || y1[i] >= y2[i] {
                let (px, py) =
                    map_data_to_pixels(x[i], y2[i], x_min, x_max, y_min, y_max, plot_area);
                path.line_to(px, py);
            }
        }

        path.close();

        if let Some(path) = path.finish() {
            // Fill the region
            let color_with_alpha = style.color.with_alpha(style.alpha);
            let mut paint = Paint::default();
            paint.set_color(color_with_alpha.to_tiny_skia_color());
            paint.anti_alias = true;

            self.pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );

            // Draw edge if specified
            if let Some(edge_color) = &style.edge_color {
                let mut edge_paint = Paint::default();
                edge_paint.set_color(edge_color.to_tiny_skia_color());
                edge_paint.anti_alias = true;

                let stroke = Stroke {
                    width: style.edge_width.max(0.1),
                    ..Stroke::default()
                };

                self.pixmap
                    .stroke_path(&path, &edge_paint, &stroke, Transform::identity(), None);
            }
        }

        Ok(())
    }

    /// Draw a horizontal span (shaded vertical region)
    fn draw_annotation_hspan(
        &mut self,
        span_x_min: f64,
        span_x_max: f64,
        style: &crate::core::ShapeStyle,
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        _y_min: f64,
        _y_max: f64,
    ) -> Result<()> {
        // Calculate pixel positions
        let frac_min = ((span_x_min - x_min) / (x_max - x_min)) as f32;
        let frac_max = ((span_x_max - x_min) / (x_max - x_min)) as f32;

        let px_min = plot_area.left() + frac_min * plot_area.width();
        let px_max = plot_area.left() + frac_max * plot_area.width();

        // Clamp to plot area
        let left = px_min.max(plot_area.left()).min(plot_area.right());
        let right = px_max.max(plot_area.left()).min(plot_area.right());

        if let Some(rect) = Rect::from_xywh(left, plot_area.top(), right - left, plot_area.height())
        {
            if let Some(fill_color) = &style.fill_color {
                let mut paint = Paint::default();
                let color_with_alpha = fill_color.with_alpha(style.fill_alpha);
                paint.set_color(color_with_alpha.to_tiny_skia_color());
                paint.anti_alias = true;

                self.pixmap
                    .fill_rect(rect, &paint, Transform::identity(), None);
            }
        }

        Ok(())
    }

    /// Draw a vertical span (shaded horizontal region)
    fn draw_annotation_vspan(
        &mut self,
        span_y_min: f64,
        span_y_max: f64,
        style: &crate::core::ShapeStyle,
        plot_area: Rect,
        _x_min: f64,
        _x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<()> {
        // Calculate pixel positions (remember Y is flipped)
        let frac_min = ((span_y_min - y_min) / (y_max - y_min)) as f32;
        let frac_max = ((span_y_max - y_min) / (y_max - y_min)) as f32;

        let py_max = plot_area.bottom() - frac_min * plot_area.height(); // y_min is at bottom
        let py_min = plot_area.bottom() - frac_max * plot_area.height(); // y_max is at top

        // Clamp to plot area
        let top = py_min.max(plot_area.top()).min(plot_area.bottom());
        let bottom = py_max.max(plot_area.top()).min(plot_area.bottom());

        if let Some(rect) = Rect::from_xywh(plot_area.left(), top, plot_area.width(), bottom - top)
        {
            if let Some(fill_color) = &style.fill_color {
                let mut paint = Paint::default();
                let color_with_alpha = fill_color.with_alpha(style.fill_alpha);
                paint.set_color(color_with_alpha.to_tiny_skia_color());
                paint.anti_alias = true;

                self.pixmap
                    .fill_rect(rect, &paint, Transform::identity(), None);
            }
        }

        Ok(())
    }
}

/// Helper function to calculate plot area with margins
pub fn calculate_plot_area(canvas_width: u32, canvas_height: u32, margin_fraction: f32) -> Rect {
    let margin_x = (canvas_width as f32) * margin_fraction;
    let margin_y = (canvas_height as f32) * margin_fraction;

    Rect::from_xywh(
        margin_x,
        margin_y,
        (canvas_width as f32) - 2.0 * margin_x,
        (canvas_height as f32) - 2.0 * margin_y,
    )
    .unwrap_or_else(|| {
        Rect::from_xywh(
            10.0,
            10.0,
            (canvas_width as f32) - 20.0,
            (canvas_height as f32) - 20.0,
        )
        .unwrap()
    })
}

/// Calculate plot area with DPI-aware margins for text space
pub fn calculate_plot_area_dpi(canvas_width: u32, canvas_height: u32, dpi_scale: f32) -> Rect {
    // Base margins in pixels (at 96 DPI) - asymmetric to account for labels
    let base_margin_left = 100.0; // Space for Y-axis label and tick labels (more space needed)
    let base_margin_right = 40.0; // Less space needed on right side
    let base_margin_top = 80.0; // Space for title (more space needed)
    let base_margin_bottom = 60.0; // Space for X-axis label

    // Scale margins with DPI
    let margin_left = base_margin_left * dpi_scale;
    let margin_right = base_margin_right * dpi_scale;
    let margin_top = base_margin_top * dpi_scale;
    let margin_bottom = base_margin_bottom * dpi_scale;

    let plot_width = (canvas_width as f32) - margin_left - margin_right;
    let plot_height = (canvas_height as f32) - margin_top - margin_bottom;

    // Ensure minimum plot area
    if plot_width > 100.0 && plot_height > 100.0 {
        // Center the plot area within the available space after accounting for labels
        let plot_x = margin_left;
        let plot_y = margin_top;

        Rect::from_xywh(plot_x, plot_y, plot_width, plot_height).unwrap_or_else(|| {
            Rect::from_xywh(
                40.0,
                40.0,
                (canvas_width as f32) - 80.0,
                (canvas_height as f32) - 80.0,
            )
            .unwrap()
        })
    } else {
        // Fallback for very small canvases
        let fallback_margin = (canvas_width.min(canvas_height) as f32) * 0.1;
        Rect::from_xywh(
            fallback_margin,
            fallback_margin,
            (canvas_width as f32) - 2.0 * fallback_margin,
            (canvas_height as f32) - 2.0 * fallback_margin,
        )
        .unwrap()
    }
}

/// Calculate plot area using config-based margins
///
/// This function uses pre-computed margins from `PlotConfig::compute_margins()`
/// which are already in inches and get converted to pixels using the provided DPI.
///
/// # Arguments
///
/// * `canvas_width` - Canvas width in pixels
/// * `canvas_height` - Canvas height in pixels
/// * `margins` - Computed margins from PlotConfig
/// * `dpi` - Output DPI for conversion
pub fn calculate_plot_area_config(
    canvas_width: u32,
    canvas_height: u32,
    margins: &ComputedMargins,
    dpi: f32,
) -> Rect {
    // Convert margins from inches to pixels
    let margin_left = margins.left_px(dpi);
    let margin_right = margins.right_px(dpi);
    let margin_top = margins.top_px(dpi);
    let margin_bottom = margins.bottom_px(dpi);

    let plot_width = (canvas_width as f32) - margin_left - margin_right;
    let plot_height = (canvas_height as f32) - margin_top - margin_bottom;

    // Ensure minimum plot area
    if plot_width > 50.0 && plot_height > 50.0 {
        let plot_x = margin_left;
        let plot_y = margin_top;

        Rect::from_xywh(plot_x, plot_y, plot_width, plot_height).unwrap_or_else(|| {
            // Fallback with minimal margins
            Rect::from_xywh(
                40.0,
                40.0,
                (canvas_width as f32) - 80.0,
                (canvas_height as f32) - 80.0,
            )
            .unwrap()
        })
    } else {
        // Fallback for very small canvases
        let fallback_margin = (canvas_width.min(canvas_height) as f32) * 0.1;
        Rect::from_xywh(
            fallback_margin,
            fallback_margin,
            (canvas_width as f32) - 2.0 * fallback_margin,
            (canvas_height as f32) - 2.0 * fallback_margin,
        )
        .unwrap()
    }
}

/// Helper function to map data coordinates to pixel coordinates
///
/// This function delegates to [`CoordinateTransform`] for the actual transformation,
/// providing a unified coordinate mapping implementation across the codebase.
pub fn map_data_to_pixels(
    data_x: f64,
    data_y: f64,
    data_x_min: f64,
    data_x_max: f64,
    data_y_min: f64,
    data_y_max: f64,
    plot_area: Rect,
) -> (f32, f32) {
    // Note: tiny_skia Rect uses top() for minimum y, bottom() for maximum y
    // CoordinateTransform expects screen_y as top..bottom (both increasing downward)
    let transform = CoordinateTransform::from_plot_area(
        plot_area.left(),
        plot_area.top(),
        plot_area.width(),
        plot_area.height(),
        data_x_min,
        data_x_max,
        data_y_min,
        data_y_max,
    );
    transform.data_to_screen(data_x, data_y)
}

/// Map data coordinates to pixel coordinates with axis scale transformations
///
/// This version applies logarithmic or symlog transformations to the data
/// before mapping to pixel coordinates. The base coordinate transformation
/// is delegated to [`CoordinateTransform`].
pub fn map_data_to_pixels_scaled(
    data_x: f64,
    data_y: f64,
    data_x_min: f64,
    data_x_max: f64,
    data_y_min: f64,
    data_y_max: f64,
    plot_area: Rect,
    x_scale: &crate::axes::AxisScale,
    y_scale: &crate::axes::AxisScale,
) -> (f32, f32) {
    use crate::axes::Scale;

    // Create scale objects for the data ranges
    let x_scale_obj = x_scale.create_scale(data_x_min, data_x_max);
    let y_scale_obj = y_scale.create_scale(data_y_min, data_y_max);

    // Transform data values to normalized [0, 1] space using the scales
    let normalized_x = x_scale_obj.transform(data_x);
    let normalized_y = y_scale_obj.transform(data_y);

    // Use CoordinateTransform with normalized [0, 1] data bounds
    // since scaling has already been applied
    let transform = CoordinateTransform::from_plot_area(
        plot_area.left(),
        plot_area.top(),
        plot_area.width(),
        plot_area.height(),
        0.0, // normalized min
        1.0, // normalized max
        0.0, // normalized min
        1.0, // normalized max
    );
    transform.data_to_screen(normalized_x, normalized_y)
}

/// Generate intelligent ticks using matplotlib's MaxNLocator algorithm
/// Produces 5-7 major ticks with "nice" numbers for scientific plotting
pub fn generate_ticks(min: f64, max: f64, target_count: usize) -> Vec<f64> {
    if min >= max || target_count == 0 {
        return vec![min, max];
    }

    // Clamp target_count to reasonable scientific range (5-7 ticks optimal)
    let max_ticks = target_count.clamp(3, 10);

    generate_scientific_ticks(min, max, max_ticks)
}

/// MaxNLocator algorithm implementation for scientific plotting
/// Based on matplotlib's tick generation with nice number selection
fn generate_scientific_ticks(min: f64, max: f64, max_ticks: usize) -> Vec<f64> {
    let range = max - min;
    if range <= 0.0 {
        return vec![min];
    }

    // Calculate rough step size
    let rough_step = range / (max_ticks - 1) as f64;

    // Handle very small ranges
    if rough_step <= f64::EPSILON {
        return vec![min, max];
    }

    // Round to "nice" numbers using powers of 10
    let magnitude = 10.0_f64.powf(rough_step.log10().floor());
    let normalized_step = rough_step / magnitude;

    // Select nice step sizes: prefer 1, 2, 5, 10 sequence
    let nice_step = if normalized_step <= 1.0 {
        1.0
    } else if normalized_step <= 2.0 {
        2.0
    } else if normalized_step <= 5.0 {
        5.0
    } else {
        10.0
    };

    let step = nice_step * magnitude;

    // Find optimal start point that includes the data range
    let start = (min / step).floor() * step;
    let end = (max / step).ceil() * step;

    // Generate ticks with epsilon for floating point stability
    let mut ticks = Vec::new();
    let mut tick = start;
    let epsilon = step * 1e-10; // Very small epsilon for float comparison

    while tick <= end + epsilon {
        // Only include ticks within the actual data range
        if tick >= min - epsilon && tick <= max + epsilon {
            // Clean up floating point errors by rounding to appropriate precision
            let clean_tick = clean_tick_value(tick, step);
            ticks.push(clean_tick);
        }
        tick += step;

        // Safety check to prevent infinite loops
        if ticks.len() > max_ticks * 2 {
            break;
        }
    }

    // Ensure we have reasonable number of ticks (3-10)
    if ticks.len() < 3 {
        // Fall back to simple min/max/middle approach with cleaned values
        let range = max - min;
        let fallback_step = range / 2.0;
        let clean_min = clean_tick_value(min, fallback_step);
        let clean_max = clean_tick_value(max, fallback_step);
        let clean_middle = clean_tick_value((min + max) / 2.0, fallback_step);
        return vec![clean_min, clean_middle, clean_max];
    }

    // Limit to max_ticks to prevent overcrowding
    if ticks.len() > max_ticks {
        ticks.truncate(max_ticks);
    }

    ticks
}

/// Clean up floating point errors in tick values by rounding to appropriate precision
fn clean_tick_value(value: f64, step: f64) -> f64 {
    // Determine number of decimal places based on step size
    let decimals = if step >= 1.0 {
        0
    } else {
        (-step.log10().floor()) as i32 + 1
    };
    let mult = 10.0_f64.powi(decimals);
    (value * mult).round() / mult
}

/// Generate minor tick values between major ticks
pub fn generate_minor_ticks(major_ticks: &[f64], minor_count: usize) -> Vec<f64> {
    if major_ticks.len() < 2 || minor_count == 0 {
        return Vec::new();
    }

    let mut minor_ticks = Vec::new();

    for i in 0..major_ticks.len() - 1 {
        let start = major_ticks[i];
        let end = major_ticks[i + 1];
        let step = (end - start) / (minor_count + 1) as f64;

        for j in 1..=minor_count {
            let minor_tick = start + step * j as f64;
            minor_ticks.push(minor_tick);
        }
    }

    minor_ticks
}

/// Format a tick value using the unified TickFormatter
///
/// This provides matplotlib-compatible tick label formatting:
/// - Integers display without decimals: "5" not "5.0"
/// - Minimal decimal precision: "3.14" not "3.140000"
/// - Scientific notation for very large/small values (|v| >= 10^4 or |v| <= 10^-4)
///
/// # Arguments
///
/// * `value` - The tick value to format
///
/// # Returns
///
/// A clean string representation of the tick value
pub fn format_tick_label(value: f64) -> String {
    // Use static formatter instance for consistency
    static FORMATTER: std::sync::LazyLock<TickFormatter> =
        std::sync::LazyLock::new(TickFormatter::default);
    FORMATTER.format_tick(value)
}

/// Format multiple tick values with consistent precision
///
/// All ticks will use the same number of decimal places,
/// determined by the tick that needs the most precision.
/// This ensures visual alignment of tick labels.
///
/// # Arguments
///
/// * `values` - The tick values to format
///
/// # Returns
///
/// Vector of formatted tick labels with consistent precision
pub fn format_tick_labels(values: &[f64]) -> Vec<String> {
    static FORMATTER: std::sync::LazyLock<TickFormatter> =
        std::sync::LazyLock::new(TickFormatter::default);
    FORMATTER.format_ticks(values)
}

#[test]
fn test_renderer_dimensions() {
    let theme = Theme::default();
    let renderer = SkiaRenderer::new(800, 600, theme).unwrap();

    assert_eq!(renderer.width(), 800);
    assert_eq!(renderer.height(), 600);
}

#[test]
fn test_draw_subplot() {
    use crate::core::plot::Image;

    let theme = Theme::default();
    let mut main_renderer = SkiaRenderer::new(800, 600, theme.clone()).unwrap();
    let subplot_renderer = SkiaRenderer::new(200, 150, theme).unwrap();

    // Convert subplot to image
    let subplot_image = subplot_renderer.into_image();

    // Should draw subplot without error
    let result = main_renderer.draw_subplot(subplot_image, 10, 20);
    assert!(result.is_ok());
}

#[test]
fn test_draw_subplot_bounds_checking() {
    use crate::core::plot::Image;

    let theme = Theme::default();
    let mut main_renderer = SkiaRenderer::new(400, 300, theme.clone()).unwrap();
    let subplot_renderer = SkiaRenderer::new(200, 150, theme).unwrap();

    let subplot_image = subplot_renderer.into_image();

    // Should handle valid positions
    assert!(
        main_renderer
            .draw_subplot(subplot_image.clone(), 0, 0)
            .is_ok()
    );
    assert!(
        main_renderer
            .draw_subplot(subplot_image.clone(), 100, 50)
            .is_ok()
    );

    // Should handle edge positions
    assert!(main_renderer.draw_subplot(subplot_image, 200, 150).is_ok());
}

#[test]
fn test_to_image_conversion() {
    let theme = Theme::default();
    let renderer = SkiaRenderer::new(400, 300, theme).unwrap();

    let image = renderer.into_image();

    assert_eq!(image.width, 400);
    assert_eq!(image.height, 300);
    assert_eq!(image.pixels.len(), 400 * 300 * 4); // RGBA pixels
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let theme = Theme::default();
        let renderer = SkiaRenderer::new(800, 600, theme);
        assert!(renderer.is_ok());

        let renderer = renderer.unwrap();
        assert_eq!(renderer.width, 800);
        assert_eq!(renderer.height, 600);
    }

    #[test]
    fn test_plot_area_calculation() {
        let area = calculate_plot_area(800, 600, 0.1);
        assert_eq!(area.left(), 80.0);
        assert_eq!(area.top(), 60.0);
        assert_eq!(area.width(), 640.0);
        assert_eq!(area.height(), 480.0);
    }

    #[test]
    fn test_data_to_pixel_mapping() {
        let plot_area = Rect::from_xywh(100.0, 100.0, 600.0, 400.0).unwrap();
        let (px, py) = map_data_to_pixels(
            1.5, 2.5, // data coordinates
            1.0, 2.0, // data x range
            2.0, 3.0, // data y range
            plot_area,
        );

        assert_eq!(px, 400.0); // middle of x range
        assert_eq!(py, 300.0); // middle of y range (flipped)
    }

    #[test]
    fn test_tick_generation() {
        let ticks = generate_ticks(0.0, 10.0, 5);
        assert!(!ticks.is_empty());
        assert!(ticks[0] >= 0.0);
        assert!(ticks.last().unwrap() <= &10.0);

        // Test edge case
        let ticks = generate_ticks(5.0, 5.0, 3);
        assert_eq!(ticks, vec![5.0, 5.0]);
    }
}
