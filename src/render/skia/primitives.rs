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

        let path = self
            .marker_path(
                if filled {
                    MarkerStyle::Circle
                } else {
                    MarkerStyle::CircleOpen
                },
                radius * 2.0,
            )?
            .ok_or(PlottingError::RenderError(
                "Failed to create circle path".to_string(),
            ))?;
        let transform = Transform::from_translate(x, y);
        self.note_marker_path_cache();

        if filled {
            self.fill_path_masked(path.as_ref(), &paint, FillRule::Winding, transform, mask)?;
        } else {
            let stroke = Stroke::default();
            self.stroke_path_masked(path.as_ref(), &paint, &stroke, transform, mask)?;
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
                self.note_pixel_aligned_rect_fill();

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

    pub fn draw_markers_clipped(
        &mut self,
        points: &[Point2f],
        size: f32,
        style: MarkerStyle,
        color: Color,
        clip_rect: (f32, f32, f32, f32),
    ) -> Result<()> {
        if points.is_empty() || size <= 0.0 || color.a == 0 {
            return Ok(());
        }

        if Self::should_use_marker_sprite_compositor(points.len(), size, style) {
            return self.draw_markers_with_sprite_compositor(points, size, style, color, clip_rect);
        }

        self.note_marker_sprite_fallback();
        let mask = self.get_clip_mask(clip_rect)?;
        for point in points {
            self.draw_marker_with_mask_vector(
                point.x,
                point.y,
                size,
                style,
                color,
                Some(mask.as_ref()),
            )?;
        }
        Ok(())
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
        self.draw_marker_with_mask_vector(x, y, size, style, color, mask)
    }

    pub(crate) fn draw_marker_with_mask_vector(
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
                let path = self
                    .marker_path(style, size)?
                    .ok_or(PlottingError::RenderError(
                        "Failed to create triangle path".to_string(),
                    ))?;
                let transform = Transform::from_translate(x, y);
                self.note_marker_path_cache();
                if style.is_filled() {
                    self.fill_path_masked(
                        path.as_ref(),
                        &paint,
                        FillRule::Winding,
                        transform,
                        mask,
                    )?;
                } else {
                    let stroke = Stroke {
                        width: (size * 0.15).max(1.0),
                        ..Stroke::default()
                    };
                    self.stroke_path_masked(path.as_ref(), &paint, &stroke, transform, mask)?;
                }
            }
            MarkerStyle::Diamond | MarkerStyle::DiamondOpen => {
                let mut paint = Paint::default();
                paint.set_color(color.to_tiny_skia_color());
                paint.anti_alias = true;
                let path = self
                    .marker_path(style, size)?
                    .ok_or(PlottingError::RenderError(
                        "Failed to create diamond path".to_string(),
                    ))?;
                let transform = Transform::from_translate(x, y);
                self.note_marker_path_cache();
                if style.is_filled() {
                    self.fill_path_masked(
                        path.as_ref(),
                        &paint,
                        FillRule::Winding,
                        transform,
                        mask,
                    )?;
                } else {
                    let stroke = Stroke {
                        width: (size * 0.15).max(1.0),
                        ..Stroke::default()
                    };
                    self.stroke_path_masked(path.as_ref(), &paint, &stroke, transform, mask)?;
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

    fn should_use_marker_sprite_compositor(
        point_count: usize,
        size: f32,
        style: MarkerStyle,
    ) -> bool {
        point_count >= 32
            && size >= 1.0
            && !matches!(
                style,
                MarkerStyle::Plus
                    | MarkerStyle::Cross
                    | MarkerStyle::Star
                    | MarkerStyle::SquareOpen
                    | MarkerStyle::TriangleOpen
                    | MarkerStyle::DiamondOpen
            )
    }

    fn draw_markers_with_sprite_compositor(
        &mut self,
        points: &[Point2f],
        size: f32,
        style: MarkerStyle,
        color: Color,
        clip_rect: (f32, f32, f32, f32),
    ) -> Result<()> {
        let phase_count = Self::marker_subpixel_phases() as usize;
        let mut sprites = vec![None; phase_count * phase_count];
        let mask = self.get_clip_mask(clip_rect)?;
        let clip_left = clip_rect.0.floor() as i32 - 1;
        let clip_top = clip_rect.1.floor() as i32 - 1;
        let clip_right = (clip_rect.0 + clip_rect.2).ceil() as i32 + 1;
        let clip_bottom = (clip_rect.1 + clip_rect.3).ceil() as i32 + 1;

        self.note_marker_sprite_compositor();

        for point in points {
            let (base_x, phase_x) = Self::quantize_marker_subpixel(point.x);
            let (base_y, phase_y) = Self::quantize_marker_subpixel(point.y);
            let slot = phase_y as usize * phase_count + phase_x as usize;
            let sprite = if let Some(sprite) = &sprites[slot] {
                Arc::clone(sprite)
            } else {
                let sprite = self.marker_sprite(style, size, color, phase_x, phase_y)?;
                sprites[slot] = Some(Arc::clone(&sprite));
                sprite
            };

            let dst_x = base_x - sprite.origin_x;
            let dst_y = base_y - sprite.origin_y;
            if dst_x + sprite.width as i32 <= clip_left
                || dst_x >= clip_right
                || dst_y + sprite.height as i32 <= clip_top
                || dst_y >= clip_bottom
            {
                continue;
            }

            if self.can_use_unmasked_circle_scanline_blit(&sprite, dst_x, dst_y, clip_rect) {
                self.note_circle_scanline_blit();
                self.blit_circle_sprite_scanlines_unmasked(&sprite, dst_x, dst_y);
            } else {
                self.blit_marker_sprite(&sprite, dst_x, dst_y, mask.as_ref());
            }
        }

        Ok(())
    }

    fn quantize_marker_subpixel(value: f32) -> (i32, u8) {
        let phase_count = Self::marker_subpixel_phases() as i32;
        let mut base = value.floor() as i32;
        let fraction = value - base as f32;
        let mut phase = (fraction * phase_count as f32).round() as i32;
        if phase >= phase_count {
            phase = 0;
            base += 1;
        }
        (base, phase as u8)
    }

    fn blit_marker_sprite(&mut self, sprite: &MarkerSprite, dst_x: i32, dst_y: i32, mask: &Mask) {
        let canvas_width = self.width as i32;
        let canvas_height = self.height as i32;
        let src_width = sprite.width as i32;
        let src_height = sprite.height as i32;

        let copy_left = dst_x.max(0);
        let copy_top = dst_y.max(0);
        let copy_right = (dst_x + src_width).min(canvas_width);
        let copy_bottom = (dst_y + src_height).min(canvas_height);

        if copy_left >= copy_right || copy_top >= copy_bottom {
            return;
        }

        let src_offset_x = (copy_left - dst_x) as usize;
        let src_offset_y = (copy_top - dst_y) as usize;
        let copy_width = (copy_right - copy_left) as usize;
        let copy_height = (copy_bottom - copy_top) as usize;

        let sprite_stride = sprite.width as usize * 4;
        let canvas_stride = self.width as usize * 4;
        let mask_stride = self.width as usize;
        let mask_data = mask.data();
        let dst_data = self.pixmap.data_mut();

        for row in 0..copy_height {
            let src_row = (src_offset_y + row) * sprite_stride + src_offset_x * 4;
            let dst_row = (copy_top as usize + row) * canvas_stride + copy_left as usize * 4;
            let mask_row = (copy_top as usize + row) * mask_stride + copy_left as usize;

            for col in 0..copy_width {
                let src_idx = src_row + col * 4;
                let src_a = sprite.pixels[src_idx + 3];
                if src_a == 0 {
                    continue;
                }

                let mask_alpha = mask_data[mask_row + col];
                if mask_alpha == 0 {
                    continue;
                }

                let dst_idx = dst_row + col * 4;
                Self::blend_premultiplied_rgba(
                    &mut dst_data[dst_idx..dst_idx + 4],
                    &sprite.pixels[src_idx..src_idx + 4],
                    mask_alpha,
                );
            }
        }
    }

    fn can_use_unmasked_circle_scanline_blit(
        &self,
        sprite: &MarkerSprite,
        dst_x: i32,
        dst_y: i32,
        clip_rect: (f32, f32, f32, f32),
    ) -> bool {
        if sprite.circle_scanlines.is_none() {
            return false;
        }

        let clip_left = clip_rect.0.ceil() as i32;
        let clip_top = clip_rect.1.ceil() as i32;
        let clip_right = (clip_rect.0 + clip_rect.2).floor() as i32;
        let clip_bottom = (clip_rect.1 + clip_rect.3).floor() as i32;

        dst_x >= clip_left
            && dst_y >= clip_top
            && dst_x + sprite.width as i32 <= clip_right
            && dst_y + sprite.height as i32 <= clip_bottom
            && dst_x >= 0
            && dst_y >= 0
            && dst_x + sprite.width as i32 <= self.width as i32
            && dst_y + sprite.height as i32 <= self.height as i32
    }

    fn blit_circle_sprite_scanlines_unmasked(
        &mut self,
        sprite: &MarkerSprite,
        dst_x: i32,
        dst_y: i32,
    ) {
        let Some(scanlines) = sprite.circle_scanlines.as_ref() else {
            return;
        };

        let sprite_stride = sprite.width as usize * 4;
        let canvas_stride = self.width as usize * 4;
        let dst_data = self.pixmap.data_mut();

        for (row_index, scanline) in scanlines.iter().enumerate() {
            if scanline.end_x <= scanline.start_x {
                continue;
            }

            let row_y = dst_y as usize + row_index;
            let src_row = row_index * sprite_stride;
            let dst_row = row_y * canvas_stride;

            let start = scanline.start_x as usize;
            let end = scanline.end_x as usize;
            let opaque_start = scanline.opaque_start_x as usize;
            let opaque_end = scanline.opaque_end_x as usize;

            let left_partial_end = opaque_start.max(start).min(end);
            for col in start..left_partial_end {
                let src_idx = src_row + col * 4;
                let dst_idx = dst_row + (dst_x as usize + col) * 4;
                Self::blend_premultiplied_rgba_unmasked(
                    &mut dst_data[dst_idx..dst_idx + 4],
                    &sprite.pixels[src_idx..src_idx + 4],
                );
            }

            if opaque_end > opaque_start {
                let src_start = src_row + opaque_start * 4;
                let src_end = src_row + opaque_end * 4;
                let dst_start = dst_row + (dst_x as usize + opaque_start) * 4;
                let dst_end = dst_row + (dst_x as usize + opaque_end) * 4;
                dst_data[dst_start..dst_end].copy_from_slice(&sprite.pixels[src_start..src_end]);
            }

            let right_partial_start = opaque_end.max(start).min(end);
            for col in right_partial_start..end {
                let src_idx = src_row + col * 4;
                let dst_idx = dst_row + (dst_x as usize + col) * 4;
                Self::blend_premultiplied_rgba_unmasked(
                    &mut dst_data[dst_idx..dst_idx + 4],
                    &sprite.pixels[src_idx..src_idx + 4],
                );
            }
        }
    }

    fn blend_premultiplied_rgba(dst: &mut [u8], src: &[u8], mask_alpha: u8) {
        let effective_alpha = Self::mul_div_255(src[3], mask_alpha);
        if effective_alpha == 0 {
            return;
        }

        if effective_alpha == u8::MAX && mask_alpha == u8::MAX {
            dst.copy_from_slice(src);
            return;
        }

        let src_r = Self::mul_div_255(src[0], mask_alpha);
        let src_g = Self::mul_div_255(src[1], mask_alpha);
        let src_b = Self::mul_div_255(src[2], mask_alpha);
        let inv_alpha = u8::MAX - effective_alpha;

        dst[0] = src_r.saturating_add(Self::mul_div_255(dst[0], inv_alpha));
        dst[1] = src_g.saturating_add(Self::mul_div_255(dst[1], inv_alpha));
        dst[2] = src_b.saturating_add(Self::mul_div_255(dst[2], inv_alpha));
        dst[3] = effective_alpha.saturating_add(Self::mul_div_255(dst[3], inv_alpha));
    }

    fn blend_premultiplied_rgba_unmasked(dst: &mut [u8], src: &[u8]) {
        let src_alpha = src[3];
        if src_alpha == 0 {
            return;
        }

        if src_alpha == u8::MAX {
            dst.copy_from_slice(src);
            return;
        }

        let inv_alpha = u8::MAX - src_alpha;
        dst[0] = src[0].saturating_add(Self::mul_div_255(dst[0], inv_alpha));
        dst[1] = src[1].saturating_add(Self::mul_div_255(dst[1], inv_alpha));
        dst[2] = src[2].saturating_add(Self::mul_div_255(dst[2], inv_alpha));
        dst[3] = src_alpha.saturating_add(Self::mul_div_255(dst[3], inv_alpha));
    }

    fn mul_div_255(value: u8, alpha: u8) -> u8 {
        (((value as u32 * alpha as u32) + 127) / 255) as u8
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
