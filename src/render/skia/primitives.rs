use super::*;
use crate::{
    core::types::Point2f,
    render::color::{scale_premultiplied_rgba, source_over_premultiplied_rgba},
};

/// Hairline width used by the legacy `draw_rectangle(.., filled = false)` path,
/// in raw device pixels. Matches `tiny_skia::Stroke::default().width`.
const LEGACY_OUTLINE_WIDTH_PX: f32 = 1.0;

/// Lower bound on a rectangle edge stroke so a requested edge never vanishes.
/// Mirrors the floor `draw_line` applies to its own stroke width.
const MIN_RECT_EDGE_WIDTH_PX: f32 = 0.1;

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

    pub(super) fn fill_path_masked(
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

    pub(super) fn stroke_path_masked(
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

    /// Draw a rectangle that is either filled or outlined.
    ///
    /// A filled rectangle is painted with **exactly** `color` — the requested
    /// RGB and the requested alpha, with no implicit tint and no implicit
    /// border. Callers that want an edge must ask for one explicitly via
    /// [`SkiaRenderer::draw_rectangle_styled`].
    ///
    /// `filled == false` keeps the legacy hairline outline (1 device pixel).
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

    /// [`SkiaRenderer::draw_rectangle`] restricted to a rectangular clip region.
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

    /// Draw a rectangle with an explicit fill and/or an explicit edge.
    ///
    /// * `fill` — painted verbatim (exact RGB, exact alpha). `None` skips the fill.
    /// * `edge` — `(colour, width_in_points)`. The width is converted with the
    ///   renderer's [`RenderScale`](crate::core::RenderScale), like every other
    ///   stroke in this backend, so the edge is DPI-invariant.
    ///
    /// Passing `None` for both is a no-op.
    pub fn draw_rectangle_styled(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        fill: Option<Color>,
        edge: Option<(Color, f32)>,
    ) -> Result<()> {
        let edge = self.scaled_edge(edge);
        self.draw_rectangle_styled_with_mask(x, y, width, height, fill, edge, None)
    }

    /// [`SkiaRenderer::draw_rectangle_styled`] restricted to a rectangular clip region.
    pub fn draw_rectangle_styled_clipped(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        fill: Option<Color>,
        edge: Option<(Color, f32)>,
        clip_rect: (f32, f32, f32, f32),
    ) -> Result<()> {
        let edge = self.scaled_edge(edge);
        let mask = self.get_clip_mask(clip_rect)?;
        self.draw_rectangle_styled_with_mask(x, y, width, height, fill, edge, Some(mask.as_ref()))
    }

    /// Convert a point-denominated edge width into device pixels.
    ///
    /// A non-positive width means "no edge" — an edge that cannot be seen is not
    /// an edge, and floored to `MIN_RECT_EDGE_WIDTH_PX` it would silently become
    /// a hairline the caller never asked for.
    fn scaled_edge(&self, edge: Option<(Color, f32)>) -> Option<(Color, f32)> {
        edge.filter(|&(_, width_pt)| width_pt > 0.0)
            .map(|(color, width_pt)| (color, self.points_to_pixels(width_pt)))
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
        let (fill, edge) = if filled {
            (Some(color), None)
        } else {
            // Legacy hairline outline: raw device pixels, matching `Stroke::default()`.
            (None, Some((color, LEGACY_OUTLINE_WIDTH_PX)))
        };

        self.draw_rectangle_styled_with_mask(x, y, width, height, fill, edge, mask)
    }

    /// Shared rectangle painter. `edge` widths are already in device pixels.
    fn draw_rectangle_styled_with_mask(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        fill: Option<Color>,
        edge: Option<(Color, f32)>,
        mask: Option<&Mask>,
    ) -> Result<()> {
        if fill.is_none() && edge.is_none() {
            return Ok(());
        }

        // A rectangle with no area encloses nothing, so it has neither an
        // interior to fill nor a boundary to stroke. Without this, a zero-value
        // bar (height == 0) would paint a hairline of the edge colour along the
        // baseline — a mark for a datum that is not there.
        if width <= 0.0 || height <= 0.0 {
            return Ok(());
        }

        let rect = Rect::from_xywh(x, y, width, height).ok_or(PlottingError::RenderError(
            "Invalid rectangle dimensions".to_string(),
        ))?;

        let mut path = PathBuilder::new();
        path.push_rect(rect);
        let path = path.finish().ok_or(PlottingError::RenderError(
            "Failed to create rectangle path".to_string(),
        ))?;

        if let Some(fill_color) = fill {
            let mut fill_paint = Paint::default();
            fill_paint.set_color(fill_color.to_tiny_skia_color());
            fill_paint.anti_alias = true;

            self.fill_path_masked(
                &path,
                &fill_paint,
                FillRule::Winding,
                Transform::identity(),
                mask,
            )?;
        }

        if let Some((edge_color, edge_width_px)) = edge {
            let mut edge_paint = Paint::default();
            edge_paint.set_color(edge_color.to_tiny_skia_color());
            edge_paint.anti_alias = true;

            let stroke = Stroke {
                width: edge_width_px.max(MIN_RECT_EDGE_WIDTH_PX),
                ..Stroke::default()
            };

            self.stroke_path_masked(&path, &edge_paint, &stroke, Transform::identity(), mask)?;
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

    /// [`SkiaRenderer::draw_marker`] with an explicit marker edge.
    ///
    /// `edge` is `(colour, width_in_points)`; the width is converted through the
    /// renderer's [`RenderScale`](crate::core::RenderScale), so the rim is
    /// DPI-invariant. See [`SkiaRenderer::draw_marker_styled_clipped`].
    pub fn draw_marker_styled(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        style: MarkerStyle,
        color: Color,
        edge: Option<(Color, f32)>,
    ) -> Result<()> {
        let edge = self.scaled_edge(edge);
        self.draw_marker_styled_with_mask_vector(x, y, size, style, color, edge, None)
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

    /// [`SkiaRenderer::draw_marker_clipped`] with an explicit marker edge.
    ///
    /// `edge` is `(colour, width_in_points)`; the width is converted through the
    /// renderer's [`RenderScale`](crate::core::RenderScale) like every other
    /// stroke here, so the rim is DPI-invariant. Only the closed filled shapes
    /// take an edge — the open styles already *are* an outline and the
    /// line-drawn styles (plus/cross/star) have no interior to rim.
    pub fn draw_marker_styled_clipped(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        style: MarkerStyle,
        color: Color,
        edge: Option<(Color, f32)>,
        clip_rect: (f32, f32, f32, f32),
    ) -> Result<()> {
        let edge = self.scaled_edge(edge);
        let mask = self.get_clip_mask(clip_rect)?;
        self.draw_marker_styled_with_mask_vector(
            x,
            y,
            size,
            style,
            color,
            edge,
            Some(mask.as_ref()),
        )
    }

    pub fn draw_markers_clipped(
        &mut self,
        points: &[Point2f],
        size: f32,
        style: MarkerStyle,
        color: Color,
        clip_rect: (f32, f32, f32, f32),
    ) -> Result<()> {
        self.draw_markers_styled_clipped(points, size, style, color, None, clip_rect)
    }

    /// [`SkiaRenderer::draw_markers_clipped`] with an explicit marker edge.
    ///
    /// `edge` is `(colour, width_in_points)` — see
    /// [`SkiaRenderer::draw_marker_styled_clipped`].
    ///
    /// The sprite compositor caches one raster per (style, size, colour, edge,
    /// phase), so an edged batch keeps the fast path: the rim is baked into the
    /// sprite exactly as the vector painter would stroke it.
    pub fn draw_markers_styled_clipped(
        &mut self,
        points: &[Point2f],
        size: f32,
        style: MarkerStyle,
        color: Color,
        edge: Option<(Color, f32)>,
        clip_rect: (f32, f32, f32, f32),
    ) -> Result<()> {
        let edge = self.scaled_edge(edge);
        if points.is_empty() || size <= 0.0 || (color.a == 0 && edge.is_none()) {
            return Ok(());
        }

        if Self::should_use_marker_sprite_compositor(points.len(), size, style) {
            return self
                .draw_markers_with_sprite_compositor(points, size, style, color, edge, clip_rect);
        }

        self.note_marker_sprite_fallback();
        let mask = self.get_clip_mask(clip_rect)?;
        for point in points {
            self.draw_marker_styled_with_mask_vector(
                point.x,
                point.y,
                size,
                style,
                color,
                edge,
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
        self.draw_marker_styled_with_mask_vector(x, y, size, style, color, None, mask)
    }

    /// Vector marker painter. `edge` widths are already in device pixels.
    pub(crate) fn draw_marker_styled_with_mask_vector(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        style: MarkerStyle,
        color: Color,
        edge: Option<(Color, f32)>,
        mask: Option<&Mask>,
    ) -> Result<()> {
        let radius = size * 0.5;
        // Only a closed filled shape has an interior for an edge to bound.
        let edge = edge.filter(|_| style.takes_edge());

        match style {
            MarkerStyle::Circle | MarkerStyle::CircleOpen => {
                self.draw_circle_with_mask(x, y, radius, color, style.is_filled(), mask)?;
            }
            MarkerStyle::Square | MarkerStyle::SquareOpen => {
                let half_size = radius;
                if style.is_filled() {
                    // Fill and rim in one pass so they share the same path.
                    self.draw_rectangle_styled_with_mask(
                        x - half_size,
                        y - half_size,
                        size,
                        size,
                        Some(color),
                        edge,
                        mask,
                    )?;
                    return Ok(());
                }
                self.draw_rectangle_with_mask(
                    x - half_size,
                    y - half_size,
                    size,
                    size,
                    color,
                    false,
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

        // Squares return early — their fill and rim share a single rectangle
        // pass. Everything else strokes the cached marker path it just filled.
        if let Some((edge_color, edge_width_px)) = edge {
            self.stroke_marker_outline(x, y, size, style, edge_color, edge_width_px, mask)?;
        }

        Ok(())
    }

    /// Stroke a filled marker's outline. `edge_width_px` is in device pixels.
    fn stroke_marker_outline(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        style: MarkerStyle,
        edge_color: Color,
        edge_width_px: f32,
        mask: Option<&Mask>,
    ) -> Result<()> {
        let Some(path) = self.marker_path(style, size)? else {
            return Ok(());
        };
        self.note_marker_path_cache();

        let mut paint = Paint::default();
        paint.set_color(edge_color.to_tiny_skia_color());
        paint.anti_alias = true;

        let stroke = Stroke {
            width: edge_width_px.max(MIN_RECT_EDGE_WIDTH_PX),
            ..Stroke::default()
        };

        self.stroke_path_masked(
            path.as_ref(),
            &paint,
            &stroke,
            Transform::from_translate(x, y),
            mask,
        )
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

    /// `edge` widths are already in device pixels.
    fn draw_markers_with_sprite_compositor(
        &mut self,
        points: &[Point2f],
        size: f32,
        style: MarkerStyle,
        color: Color,
        edge: Option<(Color, f32)>,
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
                let sprite = self.marker_sprite(style, size, color, edge, phase_x, phase_y)?;
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

            if self.can_use_unmasked_marker_scanline_blit(
                &sprite,
                dst_x,
                dst_y,
                clip_rect,
                0,
                0,
                self.width as i32,
                self.height as i32,
            ) {
                self.note_marker_scanline_blit();
                self.blit_marker_sprite_scanlines_unmasked(&sprite, dst_x, dst_y);
            } else {
                self.blit_marker_sprite_region(
                    &sprite,
                    dst_x,
                    dst_y,
                    Some(mask.as_ref()),
                    0,
                    0,
                    self.width as i32,
                    self.height as i32,
                );
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

    fn blit_marker_sprite_region(
        &mut self,
        sprite: &MarkerSprite,
        dst_x: i32,
        dst_y: i32,
        mask: Option<&Mask>,
        region_left: i32,
        region_top: i32,
        region_right: i32,
        region_bottom: i32,
    ) {
        let src_width = sprite.width as i32;
        let src_height = sprite.height as i32;

        let copy_left = dst_x.max(region_left).max(0);
        let copy_top = dst_y.max(region_top).max(0);
        let copy_right = (dst_x + src_width).min(region_right).min(self.width as i32);
        let copy_bottom = (dst_y + src_height)
            .min(region_bottom)
            .min(self.height as i32);

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
        let mask_data = mask.map(Mask::data);
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

                let dst_idx = dst_row + col * 4;
                if let Some(mask_data) = mask_data {
                    let mask_alpha = mask_data[mask_row + col];
                    if mask_alpha == 0 {
                        continue;
                    }

                    Self::blend_premultiplied_rgba(
                        &mut dst_data[dst_idx..dst_idx + 4],
                        &sprite.pixels[src_idx..src_idx + 4],
                        mask_alpha,
                    );
                } else {
                    Self::blend_premultiplied_rgba_unmasked(
                        &mut dst_data[dst_idx..dst_idx + 4],
                        &sprite.pixels[src_idx..src_idx + 4],
                    );
                }
            }
        }
    }

    fn can_use_unmasked_marker_scanline_blit(
        &self,
        sprite: &MarkerSprite,
        dst_x: i32,
        dst_y: i32,
        clip_rect: (f32, f32, f32, f32),
        region_left: i32,
        region_top: i32,
        region_right: i32,
        region_bottom: i32,
    ) -> bool {
        if sprite.scanlines.is_none() {
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
            && dst_x >= region_left
            && dst_y >= region_top
            && dst_x + sprite.width as i32 <= region_right
            && dst_y + sprite.height as i32 <= region_bottom
    }

    fn blit_marker_sprite_scanlines_unmasked(
        &mut self,
        sprite: &MarkerSprite,
        dst_x: i32,
        dst_y: i32,
    ) {
        let Some(scanlines) = sprite.scanlines.as_ref() else {
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
        let src = scale_premultiplied_rgba([src[0], src[1], src[2], src[3]], mask_alpha);
        Self::blend_premultiplied_rgba_unmasked(dst, &src);
    }

    fn blend_premultiplied_rgba_unmasked(dst: &mut [u8], src: &[u8]) {
        let blended = source_over_premultiplied_rgba(
            [dst[0], dst[1], dst[2], dst[3]],
            [src[0], src[1], src[2], src[3]],
        );
        dst.copy_from_slice(&blended);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A renderer on an opaque white canvas, so composited results are exact.
    fn white_canvas(width: u32, height: u32, dpi: f32) -> SkiaRenderer {
        let mut renderer = SkiaRenderer::new(width, height, Theme::default()).unwrap();
        renderer.set_render_scale(RenderScale::new(dpi));
        renderer.pixmap.fill(Color::WHITE.to_tiny_skia_color());
        renderer
    }

    fn pixel(image: &Image, x: u32, y: u32) -> [u8; 4] {
        let idx = ((y * image.width + x) * 4) as usize;
        [
            image.pixels[idx],
            image.pixels[idx + 1],
            image.pixels[idx + 2],
            image.pixels[idx + 3],
        ]
    }

    /// Thickness in pixels of the left edge stroke along scanline `y`.
    fn left_edge_thickness(image: &Image, y: u32) -> u32 {
        let mut thickness = 0;
        let mut seen_ink = false;
        for x in 0..image.width {
            let is_ink = pixel(image, x, y)[0] < 250;
            if is_ink {
                seen_ink = true;
                thickness += 1;
            } else if seen_ink {
                break;
            }
        }
        thickness
    }

    #[test]
    fn test_draw_rectangle_filled_uses_exact_requested_color() {
        let mut renderer = white_canvas(64, 64, 100.0);
        let requested = Color::from_rgb(31, 119, 180);
        renderer
            .draw_rectangle(10.0, 10.0, 40.0, 40.0, requested, true)
            .expect("filled rectangle should render");

        let image = renderer.into_image();
        assert_eq!(
            pixel(&image, 30, 30),
            [requested.r, requested.g, requested.b, 255],
            "a filled rectangle must render the exact requested color, with no 0.85 alpha tint"
        );
    }

    #[test]
    fn test_draw_rectangle_filled_has_no_implicit_border() {
        let mut renderer = white_canvas(64, 64, 100.0);
        let requested = Color::from_rgb(31, 119, 180);
        renderer
            .draw_rectangle(10.0, 10.0, 40.0, 40.0, requested, true)
            .expect("filled rectangle should render");

        let image = renderer.into_image();
        // The old primitive stroked a 1px border at 0.8x the fill; the pixel just
        // inside the left edge would have been darker than the interior.
        assert_eq!(
            pixel(&image, 11, 30),
            pixel(&image, 30, 30),
            "the fill must be uniform: no implicit darker edge stroke"
        );
    }

    #[test]
    fn test_draw_rectangle_filled_preserves_requested_alpha() {
        let mut renderer = white_canvas(64, 64, 100.0);
        renderer
            .draw_rectangle(10.0, 10.0, 40.0, 40.0, Color::from_rgba(0, 0, 0, 128), true)
            .expect("translucent rectangle should render");

        let image = renderer.into_image();
        // 50% black over white composites to ~127. The old 0.85 multiplier would
        // have produced ~146.
        let channel = pixel(&image, 30, 30)[0];
        assert!(
            (126..=128).contains(&channel),
            "requested alpha must survive unchanged, got channel {channel}"
        );
    }

    #[test]
    fn test_draw_rectangle_styled_fill_only_matches_plain_fill() {
        let color = Color::from_rgba(200, 40, 40, 190);

        let mut plain = white_canvas(48, 48, 100.0);
        plain
            .draw_rectangle(8.0, 8.0, 24.0, 24.0, color, true)
            .expect("plain fill should render");

        let mut styled = white_canvas(48, 48, 100.0);
        styled
            .draw_rectangle_styled(8.0, 8.0, 24.0, 24.0, Some(color), None)
            .expect("styled fill should render");

        assert_eq!(
            plain.into_image().pixels,
            styled.into_image().pixels,
            "an edgeless styled rectangle must be identical to a plain filled one"
        );
    }

    #[test]
    fn test_draw_rectangle_styled_edge_width_scales_with_dpi() {
        let edge = Color::from_rgb(0, 0, 0);

        let mut low_dpi = white_canvas(64, 64, 72.0);
        low_dpi
            .draw_rectangle_styled(16.0, 16.0, 32.0, 32.0, None, Some((edge, 1.0)))
            .expect("styled edge should render at 72 dpi");

        let mut high_dpi = white_canvas(64, 64, 288.0);
        high_dpi
            .draw_rectangle_styled(16.0, 16.0, 32.0, 32.0, None, Some((edge, 1.0)))
            .expect("styled edge should render at 288 dpi");

        let thin = left_edge_thickness(&low_dpi.into_image(), 32);
        let thick = left_edge_thickness(&high_dpi.into_image(), 32);

        assert!(
            (1..=2).contains(&thin),
            "1pt at 72 dpi should be a hairline, got {thin}px"
        );
        assert!(thick >= 4, "1pt at 288 dpi should be ~4px, got {thick}px");
        assert!(
            thick > thin,
            "edge width must scale with DPI: {thin}px at 72 dpi vs {thick}px at 288 dpi"
        );
    }

    /// Full-canvas clip, so marker tests exercise the drawing, not the clipping.
    fn whole_canvas(width: f32, height: f32) -> (f32, f32, f32, f32) {
        (0.0, 0.0, width, height)
    }

    #[test]
    fn test_filled_square_marker_has_no_edge_unless_one_is_asked_for() {
        let fill = Color::from_rgb(31, 119, 180);
        let mut renderer = white_canvas(64, 64, 100.0);
        renderer
            .draw_marker_styled_clipped(
                32.0,
                32.0,
                20.0,
                MarkerStyle::Square,
                fill,
                None,
                whole_canvas(64.0, 64.0),
            )
            .expect("square marker should render");

        let image = renderer.into_image();
        assert_eq!(
            pixel(&image, 24, 32),
            [fill.r, fill.g, fill.b, 255],
            "an edgeless square marker must be a flat fill all the way to its rim"
        );
    }

    #[test]
    fn test_filled_square_marker_strokes_a_requested_edge() {
        let fill = Color::from_rgb(31, 119, 180);
        let edge = Color::from_rgb(255, 0, 0);
        let mut renderer = white_canvas(64, 64, 100.0);
        renderer
            .draw_marker_styled_clipped(
                32.0,
                32.0,
                20.0,
                MarkerStyle::Square,
                fill,
                Some((edge, 3.0)),
                whole_canvas(64.0, 64.0),
            )
            .expect("edged square marker should render");

        let image = renderer.into_image();
        // The marker spans x 22..42; a 3pt (~4px at 100 dpi) stroke centred on
        // the left boundary fully covers the pixel column at x = 22.
        assert_eq!(
            pixel(&image, 22, 32),
            [edge.r, edge.g, edge.b, 255],
            "the requested edge colour must be stroked on the marker boundary"
        );
        assert_eq!(
            pixel(&image, 32, 32),
            [fill.r, fill.g, fill.b, 255],
            "the edge must not tint the interior fill"
        );
    }

    #[test]
    fn test_filled_circle_marker_strokes_a_requested_edge() {
        let fill = Color::from_rgb(31, 119, 180);
        let edge = Color::from_rgb(255, 0, 0);

        let mut bare = white_canvas(64, 64, 100.0);
        bare.draw_marker_styled_clipped(
            32.0,
            32.0,
            20.0,
            MarkerStyle::Circle,
            fill,
            None,
            whole_canvas(64.0, 64.0),
        )
        .expect("circle marker should render");

        let mut edged = white_canvas(64, 64, 100.0);
        edged
            .draw_marker_styled_clipped(
                32.0,
                32.0,
                20.0,
                MarkerStyle::Circle,
                fill,
                Some((edge, 3.0)),
                whole_canvas(64.0, 64.0),
            )
            .expect("edged circle marker should render");

        let edged = edged.into_image();
        assert_ne!(
            bare.into_image().pixels,
            edged.pixels,
            "a circle marker must honour an edge, not only the square path"
        );
        assert!(
            edged
                .pixels
                .chunks_exact(4)
                .any(|px| px[0] > px[2] && px[0] > 200),
            "the edge colour must actually reach the canvas"
        );
    }

    #[test]
    fn test_marker_edge_is_ignored_by_line_drawn_styles() {
        let color = Color::from_rgb(31, 119, 180);

        let mut bare = white_canvas(48, 48, 100.0);
        bare.draw_marker_styled_clipped(
            24.0,
            24.0,
            16.0,
            MarkerStyle::Plus,
            color,
            None,
            whole_canvas(48.0, 48.0),
        )
        .expect("plus marker should render");

        let mut edged = white_canvas(48, 48, 100.0);
        edged
            .draw_marker_styled_clipped(
                24.0,
                24.0,
                16.0,
                MarkerStyle::Plus,
                color,
                Some((Color::from_rgb(255, 0, 0), 3.0)),
                whole_canvas(48.0, 48.0),
            )
            .expect("plus marker should render");

        assert_eq!(
            bare.into_image().pixels,
            edged.into_image().pixels,
            "a plus has no interior, so an edge request must be a no-op rather than a second stroke"
        );
    }

    #[test]
    fn test_marker_edge_width_scales_with_dpi() {
        // Transparent fill, so only the edge inks the canvas and its thickness
        // can be measured directly.
        let transparent = Color::from_rgba(0, 0, 0, 0);
        let edge = Color::from_rgb(0, 0, 0);

        let mut low_dpi = white_canvas(64, 64, 72.0);
        low_dpi
            .draw_marker_styled_clipped(
                32.0,
                32.0,
                24.0,
                MarkerStyle::Square,
                transparent,
                Some((edge, 1.0)),
                whole_canvas(64.0, 64.0),
            )
            .expect("marker edge should render at 72 dpi");

        let mut high_dpi = white_canvas(64, 64, 288.0);
        high_dpi
            .draw_marker_styled_clipped(
                32.0,
                32.0,
                24.0,
                MarkerStyle::Square,
                transparent,
                Some((edge, 1.0)),
                whole_canvas(64.0, 64.0),
            )
            .expect("marker edge should render at 288 dpi");

        let thin = left_edge_thickness(&low_dpi.into_image(), 32);
        let thick = left_edge_thickness(&high_dpi.into_image(), 32);

        assert!(
            (1..=2).contains(&thin),
            "1pt at 72 dpi should be a hairline, got {thin}px"
        );
        assert!(thick >= 4, "1pt at 288 dpi should be ~4px, got {thick}px");
        assert!(
            thick > thin,
            "marker edge width must scale with DPI: {thin}px at 72 dpi vs {thick}px at 288 dpi"
        );
    }

    #[test]
    fn test_edged_marker_batch_matches_the_single_marker_path() {
        // 40 points is past the sprite compositor threshold. The sprite cache is
        // keyed on the edge too, so an edged batch keeps the fast path and must
        // still agree with drawing the markers one at a time.
        let points: Vec<Point2f> = (0..40)
            .map(|i| Point2f::new(8.0 + (i % 10) as f32 * 8.0, 12.0 + (i / 10) as f32 * 16.0))
            .collect();
        let fill = Color::from_rgb(31, 119, 180);
        let edge = Some((Color::from_rgb(255, 0, 0), 1.0));
        let clip = whole_canvas(96.0, 80.0);

        let mut batched = white_canvas(96, 80, 100.0);
        batched
            .draw_markers_styled_clipped(&points, 9.0, MarkerStyle::Circle, fill, edge, clip)
            .expect("batched edged markers should render");
        assert!(
            batched.render_diagnostics().used_marker_sprite_compositor,
            "asking for an edge must not cost the sprite fast path"
        );

        let mut singly = white_canvas(96, 80, 100.0);
        for point in &points {
            singly
                .draw_marker_styled_clipped(
                    point.x,
                    point.y,
                    9.0,
                    MarkerStyle::Circle,
                    fill,
                    edge,
                    clip,
                )
                .expect("single edged marker should render");
        }

        // The sprite is composited from its own transparent canvas rather than
        // painted straight onto the destination, so anti-aliased pixels may
        // round by one; the rim must otherwise be the same rim.
        let batched = batched.into_image().pixels;
        let singly = singly.into_image().pixels;
        assert_eq!(batched.len(), singly.len());
        let worst = batched
            .iter()
            .zip(singly.iter())
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0);
        assert!(
            worst <= 1,
            "batched and single-marker rendering must agree once an edge is requested, \
             worst channel delta {worst}"
        );
    }

    /// Worst per-channel difference between a sprite-composited batch and the
    /// same markers drawn one at a time through the vector painter.
    fn sprite_vs_vector_worst_delta(edge: Option<(Color, f32)>) -> u8 {
        // Deliberately off-grid: the sprite compositor snaps every centre to
        // the nearest sub-pixel phase, so integer positions hide the only
        // disagreement the fast path can have with the vector painter.
        let points: Vec<Point2f> = (0..40)
            .map(|i| Point2f::new(10.0 + (i % 10) as f32 * 18.7, 15.0 + (i / 10) as f32 * 27.3))
            .collect();
        let fill = Color::from_rgb(31, 119, 180);
        let clip = whole_canvas(200.0, 120.0);

        let mut batched = white_canvas(200, 120, 100.0);
        batched
            .draw_markers_styled_clipped(&points, 8.0, MarkerStyle::Circle, fill, edge, clip)
            .expect("batched markers should render");
        assert!(batched.render_diagnostics().used_marker_sprite_compositor);

        let mut singly = white_canvas(200, 120, 100.0);
        for point in &points {
            singly
                .draw_marker_styled_clipped(
                    point.x,
                    point.y,
                    8.0,
                    MarkerStyle::Circle,
                    fill,
                    edge,
                    clip,
                )
                .expect("single marker should render");
        }

        batched
            .into_image()
            .pixels
            .iter()
            .zip(singly.into_image().pixels.iter())
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0)
    }

    #[test]
    fn test_a_marker_rim_does_not_amplify_sprite_phase_quantisation() {
        // A rim is a thin high-contrast feature, so it samples the sprite
        // compositor's sub-pixel snapping far more harshly than a bare fill: at
        // too few phases an edged batch showed ~10x the boundary noise of the
        // edgeless one. Asking for an edge must not cost accuracy.
        let edgeless = sprite_vs_vector_worst_delta(None);
        let edged = sprite_vs_vector_worst_delta(Some((Color::from_rgb(22, 84, 126), 0.8)));

        assert!(
            edged <= edgeless.max(32),
            "an edged batch must not diverge from the vector path more than an \
             edgeless one does: {edged} vs {edgeless}"
        );
    }

    #[test]
    fn test_marker_sprites_are_exact_on_a_subpixel_phase_boundary() {
        // Centres that land exactly on a phase mean the sprite is rasterised at
        // the position it is blitted to, so the fast path must be exact there —
        // otherwise the divergence is not quantisation but a real geometry bug.
        let phase = 1.0 / SkiaRenderer::marker_subpixel_phases() as f32;
        let points: Vec<Point2f> = (0..40)
            .map(|i| {
                Point2f::new(
                    10.0 + (i % 10) as f32 * 18.0 + (i % 10) as f32 * phase,
                    15.0 + (i / 10) as f32 * 27.0 + (i / 10) as f32 * phase,
                )
            })
            .collect();
        let fill = Color::from_rgb(31, 119, 180);
        let clip = whole_canvas(200.0, 120.0);

        let mut batched = white_canvas(200, 120, 100.0);
        batched
            .draw_markers_styled_clipped(&points, 8.0, MarkerStyle::Circle, fill, None, clip)
            .expect("batched markers should render");

        let mut singly = white_canvas(200, 120, 100.0);
        for point in &points {
            singly
                .draw_marker_styled_clipped(
                    point.x,
                    point.y,
                    8.0,
                    MarkerStyle::Circle,
                    fill,
                    None,
                    clip,
                )
                .expect("single marker should render");
        }

        assert_eq!(
            batched.into_image().pixels,
            singly.into_image().pixels,
            "on an exact phase the sprite compositor must be pixel-identical to \
             the vector painter"
        );
    }

    #[test]
    fn test_wide_marker_rim_survives_the_sprite_border() {
        // Half the rim lies outside the shape, so the sprite has to be padded
        // for it; without that the outer half of a fat rim is clipped away at
        // the sprite's own border and the batch stops matching the vector path.
        let points: Vec<Point2f> = (0..40)
            .map(|i| Point2f::new(20.0 + (i % 8) as f32 * 24.0, 24.0 + (i / 8) as f32 * 28.0))
            .collect();
        let fill = Color::from_rgb(31, 119, 180);
        let edge = Some((Color::from_rgb(255, 0, 0), 10.0));
        let clip = whole_canvas(220.0, 170.0);

        let mut batched = white_canvas(220, 170, 100.0);
        batched
            .draw_markers_styled_clipped(&points, 8.0, MarkerStyle::Circle, fill, edge, clip)
            .expect("batched fat-rim markers should render");
        assert!(batched.render_diagnostics().used_marker_sprite_compositor);

        let mut singly = white_canvas(220, 170, 100.0);
        for point in &points {
            singly
                .draw_marker_styled_clipped(
                    point.x,
                    point.y,
                    8.0,
                    MarkerStyle::Circle,
                    fill,
                    edge,
                    clip,
                )
                .expect("single fat-rim marker should render");
        }

        let count_red = |pixels: &[u8]| {
            pixels
                .chunks_exact(4)
                .filter(|px| px[0] > 200 && px[1] < 60 && px[2] < 60)
                .count()
        };
        let batched_red = count_red(&batched.into_image().pixels);
        let singly_red = count_red(&singly.into_image().pixels);
        assert!(batched_red > 0, "the fat rim must be painted");
        assert!(
            batched_red.abs_diff(singly_red) * 50 <= singly_red,
            "the sprite must carry the whole rim: {batched_red} vs {singly_red}"
        );
    }

    #[test]
    fn test_edgeless_marker_batch_still_uses_the_sprite_compositor() {
        let points: Vec<Point2f> = (0..40)
            .map(|i| Point2f::new(8.0 + (i % 10) as f32 * 8.0, 12.0 + (i / 10) as f32 * 16.0))
            .collect();
        let clip = whole_canvas(96.0, 80.0);

        let mut renderer = white_canvas(96, 80, 100.0);
        renderer
            .draw_markers_styled_clipped(
                &points,
                9.0,
                MarkerStyle::Circle,
                Color::from_rgb(31, 119, 180),
                None,
                clip,
            )
            .expect("edgeless markers should render");

        assert!(
            renderer.render_diagnostics().used_marker_sprite_compositor,
            "asking for no edge must not cost the sprite fast path"
        );
    }

    #[test]
    fn test_draw_rectangle_styled_without_fill_or_edge_is_a_noop() {
        let mut renderer = white_canvas(16, 16, 100.0);
        renderer
            .draw_rectangle_styled(2.0, 2.0, 8.0, 8.0, None, None)
            .expect("empty style should be accepted");

        let image = renderer.into_image();
        assert!(
            image
                .pixels
                .chunks_exact(4)
                .all(|px| px.iter().all(|channel| *channel == 255)),
            "a rectangle with neither fill nor edge must not touch the canvas"
        );
    }
}
