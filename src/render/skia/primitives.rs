use super::*;
use crate::core::types::Point2f;

impl SkiaRenderer {
    /// Map renderer font size to Typst size units.
    pub(super) fn typst_size_pt(&self, size_px: f32) -> f32 {
        size_px.max(0.1)
    }

    /// Draw a Typst raster at subpixel-aligned coordinates.
    pub(super) fn draw_typst_raster(
        &mut self,
        rendered: &typst_text::TypstRasterOutput,
        x: f32,
        y: f32,
    ) {
        let logical_w = rendered.width.max(1e-6);
        let logical_h = rendered.height.max(1e-6);
        let pixel_w = rendered.pixmap.width().max(1) as f32;
        let pixel_h = rendered.pixmap.height().max(1) as f32;
        let scale_x = (pixel_w / logical_w).max(1e-6);
        let scale_y = (pixel_h / logical_h).max(1e-6);
        // Native 1x path: bypass resampling and snap to whole pixels for crisper text.
        if (scale_x - 1.0).abs() <= 0.02 && (scale_y - 1.0).abs() <= 0.02 {
            self.pixmap.draw_pixmap(
                x.round() as i32,
                y.round() as i32,
                rendered.pixmap.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
            return;
        }

        // Fallback for any backend/unit mismatch between logical and pixel extents.
        let transform = Transform::from_scale(1.0 / scale_x, 1.0 / scale_y).post_translate(x, y);
        let paint = PixmapPaint {
            quality: FilterQuality::Bilinear,
            ..PixmapPaint::default()
        };
        self.pixmap
            .draw_pixmap(0, 0, rendered.pixmap.as_ref(), &paint, transform, None);
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
        self.draw_line_with_mask(x1, y1, x2, y2, color, width, style, None)
    }

    /// Draw a line clipped to a rectangular region
    pub fn draw_line_clipped(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        width: f32,
        style: LineStyle,
        clip_rect: (f32, f32, f32, f32),
    ) -> Result<()> {
        let mask = self.get_clip_mask(clip_rect)?;
        self.draw_line_with_mask(x1, y1, x2, y2, color, width, style, Some(mask.as_ref()))
    }

    fn draw_line_with_mask(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Color,
        width: f32,
        style: LineStyle,
        mask: Option<&Mask>,
    ) -> Result<()> {
        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;
        paint.set_color_rgba8(color.r, color.g, color.b, color.a);

        let mut stroke = Stroke {
            width: width.max(0.1),
            ..Stroke::default()
        };

        // Apply line style (dash lengths scale with DPI for physical consistency)
        if let Some(dash_pattern) = self.scaled_dash_pattern(&style) {
            stroke.dash = StrokeDash::new(dash_pattern, 0.0);
        }

        let mut path = PathBuilder::new();
        path.move_to(x1, y1);
        path.line_to(x2, y2);
        let path = path.finish().ok_or(PlottingError::RenderError(
            "Failed to create line path".to_string(),
        ))?;

        self.stroke_path_masked(&path, &paint, &stroke, Transform::identity(), mask)?;

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
        self.draw_polyline_with_mask(points, color, width, style, None)
    }

    fn draw_polyline_with_mask(
        &mut self,
        points: &[(f32, f32)],
        color: Color,
        width: f32,
        style: LineStyle,
        mask: Option<&Mask>,
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

        // Apply line style (dash lengths scale with DPI for physical consistency)
        if let Some(dash_pattern) = self.scaled_dash_pattern(&style) {
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

        self.stroke_path_masked(&path, &paint, &stroke, Transform::identity(), mask)?;

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
        let mask = self.get_clip_mask(clip_rect)?;
        self.draw_polyline_with_mask(points, color, width, style, Some(mask.as_ref()))
    }

    /// Draw a projected polyline clipped to a rectangular region.
    pub fn draw_polyline_points_clipped(
        &mut self,
        points: &[Point2f],
        color: Color,
        width: f32,
        style: LineStyle,
        clip_rect: (f32, f32, f32, f32), // (x, y, width, height)
    ) -> Result<()> {
        if points.len() < 2 {
            return Ok(());
        }

        let mask = self.get_clip_mask(clip_rect)?;
        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;

        let mut stroke = Stroke {
            width,
            ..Stroke::default()
        };

        if let Some(dash_pattern) = self.scaled_dash_pattern(&style) {
            stroke.dash = StrokeDash::new(dash_pattern, 0.0);
        }

        let mut path = PathBuilder::new();
        path.move_to(points[0].x, points[0].y);

        for point in &points[1..] {
            path.line_to(point.x, point.y);
        }

        let path = path.finish().ok_or(PlottingError::RenderError(
            "Failed to create polyline path".to_string(),
        ))?;

        self.stroke_path_masked(
            &path,
            &paint,
            &stroke,
            Transform::identity(),
            Some(mask.as_ref()),
        )
    }

    fn get_clip_mask(&mut self, clip_rect: (f32, f32, f32, f32)) -> Result<Arc<Mask>> {
        let key = ClipMaskKey::new(clip_rect);
        if let Some(mask) = self.clip_mask_cache.get(&key) {
            return Ok(Arc::clone(mask));
        }

        let mask = Arc::new(self.create_clip_mask(clip_rect)?);
        self.clip_mask_cache.insert(key, Arc::clone(&mask));
        Ok(mask)
    }

    fn create_clip_mask(&self, clip_rect: (f32, f32, f32, f32)) -> Result<Mask> {
        let mut mask = Mask::new(self.width, self.height).ok_or(PlottingError::RenderError(
            "Failed to create clip mask".to_string(),
        ))?;
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
        mask.fill_path(&clip_path, FillRule::Winding, true, Transform::identity());
        Ok(mask)
    }

    fn fill_path_masked(
        &mut self,
        path: &tiny_skia::Path,
        paint: &Paint,
        fill_rule: FillRule,
        transform: Transform,
        mask: Option<&Mask>,
    ) -> Result<()> {
        self.pixmap
            .fill_path(path, paint, fill_rule, transform, mask);

        Ok(())
    }

    fn stroke_path_masked(
        &mut self,
        path: &tiny_skia::Path,
        paint: &Paint,
        stroke: &Stroke,
        transform: Transform,
        mask: Option<&Mask>,
    ) -> Result<()> {
        self.pixmap
            .stroke_path(path, paint, stroke, transform, mask);

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
        self.draw_circle_with_mask(x, y, radius, color, filled, None)
    }

    fn draw_circle_with_mask(
        &mut self,
        x: f32,
        y: f32,
        radius: f32,
        color: Color,
        filled: bool,
        mask: Option<&Mask>,
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
            self.fill_path_masked(
                &path,
                &paint,
                FillRule::Winding,
                Transform::identity(),
                mask,
            )?;
        } else {
            let stroke = Stroke::default();
            self.stroke_path_masked(&path, &paint, &stroke, Transform::identity(), mask)?;
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
        self.draw_rectangle_with_mask(x, y, width, height, color, filled, None)
    }

    pub fn draw_rectangle_clipped(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        filled: bool,
        clip_rect: (f32, f32, f32, f32),
    ) -> Result<()> {
        let mask = self.get_clip_mask(clip_rect)?;
        self.draw_rectangle_with_mask(x, y, width, height, color, filled, Some(mask.as_ref()))
    }

    fn draw_rectangle_with_mask(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
        filled: bool,
        mask: Option<&Mask>,
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
            self.fill_path_masked(
                &path,
                &fill_paint,
                FillRule::Winding,
                Transform::identity(),
                mask,
            )?;

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

            self.stroke_path_masked(&path, &border_paint, &stroke, Transform::identity(), mask)?;
        } else {
            // Outline only
            let mut paint = Paint::default();
            paint.set_color(color.to_tiny_skia_color());
            paint.anti_alias = true;

            let stroke = Stroke::default();
            self.stroke_path_masked(&path, &paint, &stroke, Transform::identity(), mask)?;
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

    fn rect_bounds(x: f32, y: f32, width: f32, height: f32) -> Option<(f32, f32, f32, f32)> {
        let left = x.min(x + width);
        let right = x.max(x + width);
        let top = y.min(y + height);
        let bottom = y.max(y + height);

        if !left.is_finite() || !right.is_finite() || !top.is_finite() || !bottom.is_finite() {
            return None;
        }

        if right <= left || bottom <= top {
            None
        } else {
            Some((left, top, right - left, bottom - top))
        }
    }

    fn pixel_aligned_rect_bounds(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Option<(f32, f32, f32, f32)> {
        let (left, top, width, height) = Self::rect_bounds(x, y, width, height)?;
        let right = (left + width).round();
        let bottom = (top + height).round();
        let left = left.round();
        let top = top.round();

        if right <= left || bottom <= top {
            None
        } else {
            Some((left, top, right - left, bottom - top))
        }
    }

    fn draw_composited_solid_rectangle(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) -> Result<()> {
        let Some((x, y, width, height)) = Self::rect_bounds(x, y, width, height) else {
            return Ok(());
        };
        let rect = Rect::from_xywh(x, y, width, height).ok_or(PlottingError::RenderError(
            "Invalid rectangle dimensions".to_string(),
        ))?;

        let mut path = PathBuilder::new();
        path.push_rect(rect);
        let path = path.finish().ok_or(PlottingError::RenderError(
            "Failed to create rectangle path".to_string(),
        ))?;

        let mut paint = Paint::default();
        paint.set_color(color.to_tiny_skia_color());
        paint.anti_alias = true;

        self.fill_path_masked(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        )?;

        Ok(())
    }

    /// Draw a rectangle snapped to whole-pixel edges when the fill is fully opaque.
    ///
    /// This is useful for tiled raster-like visuals such as heatmaps and filled
    /// contour cells where adjacent shapes should share exact boundaries. When
    /// the input is translucent or snapping would collapse the tile, this
    /// falls back to the normal composited fill path so alpha blending and
    /// subpixel coverage are preserved.
    pub fn draw_pixel_aligned_solid_rectangle(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) -> Result<()> {
        if color.a != u8::MAX {
            return self.draw_composited_solid_rectangle(x, y, width, height, color);
        }

        if let Some((x, y, width, height)) = Self::pixel_aligned_rect_bounds(x, y, width, height) {
            let left = x.max(0.0).floor() as u32;
            let top = y.max(0.0).floor() as u32;
            let right = (x + width).min(self.width as f32).ceil() as u32;
            let bottom = (y + height).min(self.height as f32).ceil() as u32;

            if left < right && top < bottom {
                let fill = color.to_tiny_skia_color().premultiply().to_color_u8();

                let pixels = self.pixmap.pixels_mut();
                for py in top..bottom {
                    let row_start = (py * self.width) as usize;
                    for px in left..right {
                        pixels[row_start + px as usize] = fill;
                    }
                }

                return Ok(());
            }
        }

        self.draw_composited_solid_rectangle(x, y, width, height, color)
    }

    /// Draw a rectangle outline snapped to whole-pixel edges.
    pub fn draw_pixel_aligned_rectangle_outline(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) -> Result<()> {
        if let Some((x, y, width, height)) = Self::pixel_aligned_rect_bounds(x, y, width, height) {
            self.draw_rectangle(x, y, width, height, color, false)?;
        }

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
        self.fill_path_masked(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            Some(&mask),
        )?;

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
        self.draw_marker_with_mask(x, y, size, style, color, None)
    }

    pub fn draw_marker_clipped(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        style: MarkerStyle,
        color: Color,
        clip_rect: (f32, f32, f32, f32),
    ) -> Result<()> {
        let mask = self.get_clip_mask(clip_rect)?;
        self.draw_marker_with_mask(x, y, size, style, color, Some(mask.as_ref()))
    }

    fn draw_marker_with_mask(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        style: MarkerStyle,
        color: Color,
        mask: Option<&Mask>,
    ) -> Result<()> {
        let radius = size * 0.5;

        match style {
            MarkerStyle::Circle | MarkerStyle::CircleOpen => {
                self.draw_circle_with_mask(x, y, radius, color, style.is_filled(), mask)?;
            }
            MarkerStyle::Square | MarkerStyle::SquareOpen => {
                let half_size = radius;
                self.draw_rectangle_with_mask(
                    x - half_size,
                    y - half_size,
                    size,
                    size,
                    color,
                    style.is_filled(),
                    mask,
                )?;
            }
            MarkerStyle::Triangle | MarkerStyle::TriangleOpen | MarkerStyle::TriangleDown => {
                let mut paint = Paint::default();
                paint.set_color(color.to_tiny_skia_color());
                paint.anti_alias = true;

                let mut path = PathBuilder::new();
                if style == MarkerStyle::TriangleDown {
                    path.move_to(x, y + radius);
                    path.line_to(x - radius * 0.866, y - radius * 0.5);
                    path.line_to(x + radius * 0.866, y - radius * 0.5);
                } else {
                    path.move_to(x, y - radius);
                    path.line_to(x - radius * 0.866, y + radius * 0.5); // 60 degree angles
                    path.line_to(x + radius * 0.866, y + radius * 0.5);
                }
                path.close();

                let path = path.finish().ok_or(PlottingError::RenderError(
                    "Failed to create triangle path".to_string(),
                ))?;
                if style.is_filled() {
                    self.fill_path_masked(
                        &path,
                        &paint,
                        FillRule::Winding,
                        Transform::identity(),
                        mask,
                    )?;
                } else {
                    let stroke = Stroke {
                        width: (size * 0.15).max(1.0),
                        ..Stroke::default()
                    };
                    self.stroke_path_masked(&path, &paint, &stroke, Transform::identity(), mask)?;
                }
            }
            MarkerStyle::Diamond | MarkerStyle::DiamondOpen => {
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
                if style.is_filled() {
                    self.fill_path_masked(
                        &path,
                        &paint,
                        FillRule::Winding,
                        Transform::identity(),
                        mask,
                    )?;
                } else {
                    let stroke = Stroke {
                        width: (size * 0.15).max(1.0),
                        ..Stroke::default()
                    };
                    self.stroke_path_masked(&path, &paint, &stroke, Transform::identity(), mask)?;
                }
            }
            MarkerStyle::Plus => {
                // Draw cross with lines - line width proportional to marker size
                let marker_line_width = (size * 0.25).max(1.0);
                self.draw_line_with_mask(
                    x - radius,
                    y,
                    x + radius,
                    y,
                    color,
                    marker_line_width,
                    LineStyle::Solid,
                    mask,
                )?;
                self.draw_line_with_mask(
                    x,
                    y - radius,
                    x,
                    y + radius,
                    color,
                    marker_line_width,
                    LineStyle::Solid,
                    mask,
                )?;
            }
            MarkerStyle::Cross => {
                // Draw X with lines - line width proportional to marker size
                let marker_line_width = (size * 0.25).max(1.0);
                let offset = radius * 0.707; // sin(45°)
                self.draw_line_with_mask(
                    x - offset,
                    y - offset,
                    x + offset,
                    y + offset,
                    color,
                    marker_line_width,
                    LineStyle::Solid,
                    mask,
                )?;
                self.draw_line_with_mask(
                    x - offset,
                    y + offset,
                    x + offset,
                    y - offset,
                    color,
                    marker_line_width,
                    LineStyle::Solid,
                    mask,
                )?;
            }
            MarkerStyle::Star => {
                let marker_line_width = (size * 0.22).max(1.0);
                self.draw_line_with_mask(
                    x - radius,
                    y,
                    x + radius,
                    y,
                    color,
                    marker_line_width,
                    LineStyle::Solid,
                    mask,
                )?;
                self.draw_line_with_mask(
                    x,
                    y - radius,
                    x,
                    y + radius,
                    color,
                    marker_line_width,
                    LineStyle::Solid,
                    mask,
                )?;
                let offset = radius * 0.707;
                self.draw_line_with_mask(
                    x - offset,
                    y - offset,
                    x + offset,
                    y + offset,
                    color,
                    marker_line_width,
                    LineStyle::Solid,
                    mask,
                )?;
                self.draw_line_with_mask(
                    x - offset,
                    y + offset,
                    x + offset,
                    y - offset,
                    color,
                    marker_line_width,
                    LineStyle::Solid,
                    mask,
                )?;
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
}
