use super::*;
use crate::{
    core::types::Point2f,
    render::color::{scale_premultiplied_rgba, source_over_premultiplied_rgba},
};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Hairline width used by the legacy `draw_rectangle(.., filled = false)` path,
/// in raw device pixels. Matches `tiny_skia::Stroke::default().width`.
const LEGACY_OUTLINE_WIDTH_PX: f32 = 1.0;

/// Lower bound on a rectangle edge stroke so a requested edge never vanishes.
/// Mirrors the floor `draw_line` applies to its own stroke width.
const MIN_RECT_EDGE_WIDTH_PX: f32 = 0.1;

/// Point count below which visiting every point once per band costs more than
/// the disjoint-row parallelism saves.
#[cfg(feature = "parallel")]
const PARALLEL_MARKER_BLIT_THRESHOLD: usize = 10_000;

/// Bound the number of row-band tasks submitted to Rayon's shared pool. This
/// keeps a single plot render from flooding a host application's executor.
#[cfg(feature = "parallel")]
const MAX_PARALLEL_MARKER_BLIT_WORKERS: usize = 8;

impl SkiaRenderer {
    // ------------------------------------------------------------------
    // Non-finite geometry policy
    // ------------------------------------------------------------------
    //
    // Twin of the policy documented in [`crate::export::svg::SvgRenderer`]. The
    // two backends answer a `NaN` the same way, because a chart that exports
    // differently to PNG and SVG is worse than either answer on its own.
    //
    // `NaN`/`±inf` must never reach tiny-skia. `PathBuilder::finish` rejects a
    // path whose bounds are not finite, so one bad vertex discards the *whole*
    // path rather than the offending segment — an unchecked hole silently costs
    // a complete series. The marker compositor is worse still: it quantises a
    // `NaN` coordinate to 0 and paints the marker in the canvas corner, which
    // reads as data.
    //
    // There are exactly two permitted answers. When you add a primitive, pick
    // the bucket it belongs to and use the matching helper — do not invent a
    // third.
    //
    // 1. **Open stroked and point geometry** — lines, polylines, markers. A
    //    non-finite coordinate is a *gap*: the sample has no position on these
    //    axes. A polyline is split around the hole and the surviving runs are
    //    still stroked; a lone unplaceable point or marker is skipped. No error
    //    is raised — a gap is data the axes genuinely cannot show, not a bug.
    //    Use [`Self::all_finite`] / [`Self::finite_runs`].
    //
    // 2. **Shapes with defining dimensions** — rectangles, rounded rectangles,
    //    circles, closed polygons. There is no meaningful partial shape: a rect
    //    with a `NaN` height means unvalidated geometry got past the
    //    series-level checks, i.e. an internal invariant failed. Raise via
    //    [`Self::reject_non_finite`] / [`Self::reject_non_finite_vertices`],
    //    which name the offending dimension so the message points at the bug
    //    instead of merely asserting one exists.

    /// Bucket 1: can every one of these numbers be rasterised?
    pub(super) fn all_finite(values: &[f32]) -> bool {
        values.iter().all(|value| value.is_finite())
    }

    /// Bucket 1: split a vertex list at every point `finite` rejects.
    ///
    /// The surviving runs are stroked as separate paths, so the stroke shows
    /// the gap instead of inventing a segment across it — and, crucially,
    /// instead of tiny-skia discarding every segment. Runs shorter than two
    /// points are returned as-is and dropped by the emitters, which already
    /// refuse them.
    fn finite_runs<T>(points: &[T], finite: impl Fn(&T) -> bool) -> Vec<&[T]> {
        let mut runs = Vec::new();
        let mut start = 0usize;
        for (index, point) in points.iter().enumerate() {
            if finite(point) {
                continue;
            }
            if index > start {
                runs.push(&points[start..index]);
            }
            start = index + 1;
        }
        if points.len() > start {
            runs.push(&points[start..]);
        }
        runs
    }

    /// Bucket 2: refuse a shape whose defining dimensions are not all finite.
    ///
    /// The message names the dimension and says the fault is an unvalidated
    /// input reaching the backend, so it reads as the internal-invariant
    /// failure it is rather than as a diagnostic about the caller's data — the
    /// user-facing "use SymLog for non-positive values" message belongs to the
    /// series-level validation that should have fired first.
    fn reject_non_finite(primitive: &str, dims: &[(&str, f32)]) -> Result<()> {
        let Some((name, value)) = dims.iter().copied().find(|&(_, v)| !v.is_finite()) else {
            return Ok(());
        };
        Err(PlottingError::RenderError(format!(
            "{primitive} was given a non-finite {name} ({value}). Geometry reached the raster \
             backend without being validated; this is an internal invariant failure in the \
             renderer, not a limit of the supplied data."
        )))
    }

    /// Bucket 2 for closed polygons, whose vertices *are* the defining shape.
    fn reject_non_finite_vertices(primitive: &str, vertices: &[(f32, f32)]) -> Result<()> {
        let Some((index, &(x, y))) = vertices
            .iter()
            .enumerate()
            .find(|&(_, &(x, y))| !x.is_finite() || !y.is_finite())
        else {
            return Ok(());
        };
        Err(PlottingError::RenderError(format!(
            "{primitive} was given a non-finite vertex {index} ({x}, {y}). Geometry reached the \
             raster backend without being validated; this is an internal invariant failure in \
             the renderer, not a limit of the supplied data."
        )))
    }

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
        // Bucket 1: an endpoint the axes cannot place makes the segment a gap,
        // so it is skipped. Without this, `PathBuilder::finish` below would
        // fail and take the caller's whole draw call with it.
        if !Self::all_finite(&[x1, y1, x2, y2, width]) {
            return Ok(());
        }

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

    /// Draw a series of connected lines (polyline).
    ///
    /// A vertex the axes cannot place (`NaN`/`±inf`) **breaks** the line: the
    /// run before the hole and the run after it are stroked as separate paths,
    /// so the line shows the gap instead of inventing a segment across it.
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
        if !width.is_finite() {
            return Ok(());
        }
        if Self::all_finite_points(points) {
            return self.stroke_polyline_run(points, color, width, &style, mask);
        }
        // Bucket 1: stroke each representable run on its own, so the hole
        // becomes a gap rather than costing the whole series (tiny-skia would
        // reject the entire path for one non-finite vertex).
        for run in Self::finite_runs(points, |&(x, y)| x.is_finite() && y.is_finite()) {
            self.stroke_polyline_run(run, color, width, &style, mask)?;
        }
        Ok(())
    }

    fn all_finite_points(points: &[(f32, f32)]) -> bool {
        points.iter().all(|&(x, y)| x.is_finite() && y.is_finite())
    }

    /// Stroke one run of a polyline. Every vertex must already be finite.
    fn stroke_polyline_run(
        &mut self,
        points: &[(f32, f32)],
        color: Color,
        width: f32,
        style: &LineStyle,
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
        if let Some(dash_pattern) = self.scaled_dash_pattern(style) {
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
    ///
    /// Breaks at non-finite vertices exactly like [`SkiaRenderer::draw_polyline`].
    pub fn draw_polyline_points_clipped(
        &mut self,
        points: &[Point2f],
        color: Color,
        width: f32,
        style: LineStyle,
        clip_rect: (f32, f32, f32, f32), // (x, y, width, height)
    ) -> Result<()> {
        if points.len() < 2 || !width.is_finite() {
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

        // Bucket 1: one unplaceable sample must not cost the whole line.
        for run in Self::finite_runs(points, |point| point.x.is_finite() && point.y.is_finite()) {
            if run.len() < 2 {
                continue;
            }

            let mut path = PathBuilder::new();
            path.move_to(run[0].x, run[0].y);

            for point in &run[1..] {
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
            )?;
        }

        Ok(())
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
        // Bucket 2: a circle is defined by its centre and radius, so there is
        // no partial circle to fall back to.
        Self::reject_non_finite("circle", &[("cx", x), ("cy", y), ("radius", radius)])?;

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
    ///   renderer's [`RenderScale`], like every other
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

        // Bucket 2. This is the guard that used to be `Rect::from_xywh`
        // returning `None` and reporting the anonymous "Invalid rectangle
        // dimensions": a bar or histogram column whose height could not be
        // projected arrived here as `NaN`.
        Self::reject_non_finite(
            "rectangle",
            &[
                ("x", x),
                ("y", y),
                ("width", width),
                ("height", height),
                ("edge width", edge.map_or(0.0, |(_, w)| w)),
            ],
        )?;

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
        // Bucket 2: see the non-finite geometry policy at the top of this file.
        Self::reject_non_finite(
            "solid rectangle",
            &[("x", x), ("y", y), ("width", width), ("height", height)],
        )?;

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
        // Bucket 2, and it has to come first: `rect_bounds` folds "non-finite"
        // and "zero area" into the same `None`, so without this a `NaN` cell
        // would be silently skipped and the figure would be quietly wrong.
        Self::reject_non_finite(
            "solid rectangle",
            &[("x", x), ("y", y), ("width", width), ("height", height)],
        )?;

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
        // Bucket 2, before the pixel snapping: `NaN as u32` saturates to 0, so
        // an unchecked tile would be painted in the canvas corner.
        Self::reject_non_finite(
            "pixel-aligned rectangle",
            &[("x", x), ("y", y), ("width", width), ("height", height)],
        )?;

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
        // Bucket 2: `pixel_aligned_rect_bounds` cannot tell a degenerate tile
        // from an unvalidated one, so the non-finite case is caught here.
        Self::reject_non_finite(
            "pixel-aligned rectangle outline",
            &[("x", x), ("y", y), ("width", width), ("height", height)],
        )?;

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
        // Bucket 2: see the non-finite geometry policy at the top of this file.
        Self::reject_non_finite(
            "rounded rectangle",
            &[
                ("x", x),
                ("y", y),
                ("width", width),
                ("height", height),
                ("corner radius", corner_radius),
            ],
        )?;

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
        // Bucket 2: unlike a polyline, a closed filled shape has no sensible
        // "gap" — dropping a vertex silently fills a different area.
        Self::reject_non_finite_vertices("filled polygon", vertices)?;

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
        // Bucket 2: see `draw_filled_polygon`.
        Self::reject_non_finite_vertices("filled polygon", vertices)?;
        Self::reject_non_finite(
            "polygon clip rectangle",
            &[
                ("x", clip_rect.0),
                ("y", clip_rect.1),
                ("width", clip_rect.2),
                ("height", clip_rect.3),
            ],
        )?;

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
        // Bucket 2: a closed outline traces the same shape a fill would.
        Self::reject_non_finite("polygon outline", &[("stroke width", width)])?;
        Self::reject_non_finite_vertices("polygon outline", vertices)?;

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
    /// renderer's [`RenderScale`], so the rim is
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
    /// renderer's [`RenderScale`] like every other
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
        // `size <= 0.0` is false for `NaN`, so the finiteness check is explicit.
        if points.is_empty() || !size.is_finite() || size <= 0.0 || (color.a == 0 && edge.is_none())
        {
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
        // Bucket 1: a marker is point geometry, so a sample the axes cannot
        // place is simply not drawn. Caught here so the shape emitters below —
        // which *do* raise — never see the non-finite centre.
        if !Self::all_finite(&[x, y, size]) {
            return Ok(());
        }

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
        #[cfg(feature = "parallel")]
        {
            let worker_count = Self::marker_blit_worker_count(points.len(), self.height);
            if worker_count > 1 {
                return self.draw_markers_with_sprite_compositor_parallel(
                    points,
                    size,
                    style,
                    color,
                    edge,
                    clip_rect,
                    worker_count,
                );
            }
        }

        self.draw_markers_with_sprite_compositor_serial(points, size, style, color, edge, clip_rect)
    }

    #[cfg(feature = "parallel")]
    fn marker_blit_worker_count(point_count: usize, canvas_height: u32) -> usize {
        if point_count < PARALLEL_MARKER_BLIT_THRESHOLD {
            return 1;
        }

        std::thread::available_parallelism()
            .map_or(1, std::num::NonZeroUsize::get)
            .min(MAX_PARALLEL_MARKER_BLIT_WORKERS)
            .min(canvas_height as usize)
    }

    /// The feature-off path deliberately stays as the original point-ordered
    /// compositor. It is also the byte-exact oracle for the parallel test.
    fn draw_markers_with_sprite_compositor_serial(
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
            // Bucket 1: `NaN as i32` saturates to 0, so an unplaceable sample
            // would otherwise be blitted into the top-left canvas corner.
            if !point.x.is_finite() || !point.y.is_finite() {
                continue;
            }
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

            if Self::can_use_unmasked_marker_scanline_blit(
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

    #[cfg(feature = "parallel")]
    #[allow(clippy::too_many_arguments)]
    fn draw_markers_with_sprite_compositor_parallel(
        &mut self,
        points: &[Point2f],
        size: f32,
        style: MarkerStyle,
        color: Color,
        edge: Option<(Color, f32)>,
        clip_rect: (f32, f32, f32, f32),
        worker_count: usize,
    ) -> Result<()> {
        debug_assert!(worker_count > 1);

        let phase_count = Self::marker_subpixel_phases() as usize;
        let mut sprites = vec![None; phase_count * phase_count];
        let mask = self.get_clip_mask(clip_rect)?;
        self.note_marker_sprite_compositor();

        // The sprite caches require `&mut self`. Resolve phases in their first
        // point-occurrence order, matching the serial path, before any pixel
        // slice is lent to Rayon.
        for point in points {
            if !point.x.is_finite() || !point.y.is_finite() {
                continue;
            }
            let (_, phase_x) = Self::quantize_marker_subpixel(point.x);
            let (_, phase_y) = Self::quantize_marker_subpixel(point.y);
            let slot = phase_y as usize * phase_count + phase_x as usize;
            if sprites[slot].is_none() {
                sprites[slot] =
                    Some(self.marker_sprite(style, size, color, edge, phase_x, phase_y)?);
            }
        }

        let mask_data = mask.data();
        let clip_left = clip_rect.0.floor() as i32 - 1;
        let clip_top = clip_rect.1.floor() as i32 - 1;
        let clip_right = (clip_rect.0 + clip_rect.2).ceil() as i32 + 1;
        let clip_bottom = (clip_rect.1 + clip_rect.3).ceil() as i32 + 1;
        let canvas_width = self.width as usize;
        let canvas_height = self.height as usize;
        let canvas_width_i32 = self.width as i32;
        let canvas_height_i32 = self.height as i32;
        let canvas_stride = canvas_width * 4;
        // Several bands per worker: dense data (a gaussian scatter, say)
        // concentrates points in the middle rows, and with one band per worker
        // the busiest band caps the speedup. Finer bands let rayon's work
        // stealing even the load out; the floor keeps a band taller than a
        // typical marker so most sprites still land in a single band.
        let rows_per_band = canvas_height.div_ceil(worker_count * 4).max(16);
        let bytes_per_band = rows_per_band * canvas_stride;
        let band_count = canvas_height.div_ceil(rows_per_band.max(1)).max(1);

        // One ordered pass assigns each point index to the bands its sprite
        // rows touch (almost always exactly one), so a band walks only its own
        // points instead of the full list — at 10M points the full-list walk
        // per band was itself the bottleneck. Indices stay in submission order
        // inside every band, which is what keeps each pixel's source-over
        // sequence identical to the serial compositor. A sprite with no
        // visible row draws nothing in either path and is dropped here.
        let mut band_points: Vec<Vec<u32>> = vec![Vec::new(); band_count];
        for (index, point) in points.iter().enumerate() {
            if !point.x.is_finite() || !point.y.is_finite() {
                continue;
            }
            let (base_x, phase_x) = Self::quantize_marker_subpixel(point.x);
            let (base_y, phase_y) = Self::quantize_marker_subpixel(point.y);
            let slot = phase_y as usize * phase_count + phase_x as usize;
            // Pre-resolved above for every finite point; a miss would only
            // skip the point, never draw a wrong sprite.
            let Some(sprite) = sprites[slot].as_deref() else {
                continue;
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
            let first_row = dst_y.max(0);
            let last_row = (dst_y + sprite.height as i32 - 1).min(canvas_height_i32 - 1);
            if first_row > last_row {
                continue;
            }
            let first_band = first_row as usize / rows_per_band;
            let last_band = last_row as usize / rows_per_band;
            for band in band_points.iter_mut().take(last_band + 1).skip(first_band) {
                band.push(index as u32);
            }
        }

        let used_scanline_blit = self
            .pixmap
            .data_mut()
            .par_chunks_mut(bytes_per_band)
            .zip(band_points.par_iter())
            .enumerate()
            .map(|(band_index, (band_pixels, band_indices))| {
                let band_top = band_index * rows_per_band;
                let band_bottom = band_top + band_pixels.len() / canvas_stride;
                let mut band_used_scanline_blit = false;

                // Band membership was decided above; each index here is
                // finite, clip-visible, and touches this band's rows.
                for &index in band_indices {
                    let point = &points[index as usize];
                    let (base_x, phase_x) = Self::quantize_marker_subpixel(point.x);
                    let (base_y, phase_y) = Self::quantize_marker_subpixel(point.y);
                    let slot = phase_y as usize * phase_count + phase_x as usize;
                    // Same pre-resolution invariant as the binning pass.
                    let Some(sprite) = sprites[slot].as_deref() else {
                        continue;
                    };
                    let dst_x = base_x - sprite.origin_x;
                    let dst_y = base_y - sprite.origin_y;

                    if Self::can_use_unmasked_marker_scanline_blit(
                        sprite,
                        dst_x,
                        dst_y,
                        clip_rect,
                        0,
                        0,
                        canvas_width_i32,
                        canvas_height_i32,
                    ) {
                        band_used_scanline_blit = true;
                        Self::blit_marker_sprite_scanlines_unmasked_in_band(
                            band_pixels,
                            canvas_width,
                            band_top,
                            band_bottom,
                            sprite,
                            dst_x,
                            dst_y,
                        );
                    } else {
                        Self::blit_marker_sprite_masked_in_band(
                            band_pixels,
                            canvas_width,
                            canvas_height,
                            band_top,
                            band_bottom,
                            sprite,
                            dst_x,
                            dst_y,
                            mask_data,
                        );
                    }
                }

                band_used_scanline_blit
            })
            .reduce(|| false, |left, right| left || right);

        if used_scanline_blit {
            self.note_marker_scanline_blit();
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

    #[cfg(feature = "parallel")]
    #[allow(clippy::too_many_arguments)]
    fn blit_marker_sprite_masked_in_band(
        band_pixels: &mut [u8],
        canvas_width: usize,
        canvas_height: usize,
        band_top: usize,
        band_bottom: usize,
        sprite: &MarkerSprite,
        dst_x: i32,
        dst_y: i32,
        mask_data: &[u8],
    ) {
        let src_width = sprite.width as i32;
        let src_height = sprite.height as i32;
        let copy_left = dst_x.max(0);
        let copy_top = dst_y.max(band_top as i32).max(0);
        let copy_right = (dst_x + src_width).min(canvas_width as i32);
        let copy_bottom = (dst_y + src_height)
            .min(band_bottom as i32)
            .min(canvas_height as i32);

        if copy_left >= copy_right || copy_top >= copy_bottom {
            return;
        }

        let src_offset_x = (copy_left - dst_x) as usize;
        let src_offset_y = (copy_top - dst_y) as usize;
        let copy_width = (copy_right - copy_left) as usize;
        let copy_height = (copy_bottom - copy_top) as usize;
        let sprite_stride = sprite.width as usize * 4;
        let canvas_stride = canvas_width * 4;

        for row in 0..copy_height {
            let src_row = (src_offset_y + row) * sprite_stride + src_offset_x * 4;
            let canvas_y = copy_top as usize + row;
            let dst_row = (canvas_y - band_top) * canvas_stride + copy_left as usize * 4;
            let mask_row = canvas_y * canvas_width + copy_left as usize;

            for col in 0..copy_width {
                let src_idx = src_row + col * 4;
                if sprite.pixels[src_idx + 3] == 0 {
                    continue;
                }

                let mask_alpha = mask_data[mask_row + col];
                if mask_alpha == 0 {
                    continue;
                }

                let dst_idx = dst_row + col * 4;
                Self::blend_premultiplied_rgba(
                    &mut band_pixels[dst_idx..dst_idx + 4],
                    &sprite.pixels[src_idx..src_idx + 4],
                    mask_alpha,
                );
            }
        }
    }

    #[cfg(feature = "parallel")]
    #[allow(clippy::too_many_arguments)]
    fn blit_marker_sprite_scanlines_unmasked_in_band(
        band_pixels: &mut [u8],
        canvas_width: usize,
        band_top: usize,
        band_bottom: usize,
        sprite: &MarkerSprite,
        dst_x: i32,
        dst_y: i32,
    ) {
        let Some(scanlines) = sprite.scanlines.as_ref() else {
            return;
        };

        let sprite_top = dst_y as usize;
        let first_sprite_row = band_top.max(sprite_top) - sprite_top;
        let past_last_sprite_row = band_bottom
            .min(sprite_top + sprite.height as usize)
            .saturating_sub(sprite_top);
        if first_sprite_row >= past_last_sprite_row {
            return;
        }

        let sprite_stride = sprite.width as usize * 4;
        let canvas_stride = canvas_width * 4;

        for (row_index, scanline) in scanlines
            .iter()
            .enumerate()
            .take(past_last_sprite_row)
            .skip(first_sprite_row)
        {
            if scanline.end_x <= scanline.start_x {
                continue;
            }

            let row_y = sprite_top + row_index;
            let src_row = row_index * sprite_stride;
            let dst_row = (row_y - band_top) * canvas_stride;
            let start = scanline.start_x as usize;
            let end = scanline.end_x as usize;
            let opaque_start = scanline.opaque_start_x as usize;
            let opaque_end = scanline.opaque_end_x as usize;

            let left_partial_end = opaque_start.max(start).min(end);
            for col in start..left_partial_end {
                let src_idx = src_row + col * 4;
                let dst_idx = dst_row + (dst_x as usize + col) * 4;
                Self::blend_premultiplied_rgba_unmasked(
                    &mut band_pixels[dst_idx..dst_idx + 4],
                    &sprite.pixels[src_idx..src_idx + 4],
                );
            }

            if opaque_end > opaque_start {
                let src_start = src_row + opaque_start * 4;
                let src_end = src_row + opaque_end * 4;
                let dst_start = dst_row + (dst_x as usize + opaque_start) * 4;
                let dst_end = dst_row + (dst_x as usize + opaque_end) * 4;
                band_pixels[dst_start..dst_end].copy_from_slice(&sprite.pixels[src_start..src_end]);
            }

            let right_partial_start = opaque_end.max(start).min(end);
            for col in right_partial_start..end {
                let src_idx = src_row + col * 4;
                let dst_idx = dst_row + (dst_x as usize + col) * 4;
                Self::blend_premultiplied_rgba_unmasked(
                    &mut band_pixels[dst_idx..dst_idx + 4],
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

    #[cfg(feature = "parallel")]
    #[test]
    fn serial_and_parallel_marker_blits_are_byte_identical() {
        const WIDTH: u32 = 104;
        const HEIGHT: u32 = 96;
        const WORKERS: usize = 4;
        const BAND_BOUNDARY_ROWS: [f32; 15] = [
            4.75, 6.5, 7.0, 23.75, 24.0, 24.25, 47.75, 48.0, 48.25, 71.75, 72.0, 72.25, 88.75,
            89.25, 90.0,
        ];

        let clip_rect = (8.25, 6.5, 87.5, 82.75);
        let mut points = Vec::with_capacity(20_002);
        for index in 0..20_000 {
            // Four 24-row bands put boundaries at y=24, 48, and 72. The
            // repeated centres straddle all three, while the first/last groups
            // straddle the fractional clip edges. X covers both clip edges and
            // cycles through every subpixel phase.
            let x = 3.0 + ((index * 37) % 1584) as f32 / 16.0;
            let y = BAND_BOUNDARY_ROWS[index % BAND_BOUNDARY_ROWS.len()]
                + ((index / BAND_BOUNDARY_ROWS.len()) % 4) as f32 / 64.0;
            points.push(Point2f::new(x, y));
        }
        points.push(Point2f::new(f32::NAN, 24.0));
        points.push(Point2f::new(48.0, f32::INFINITY));

        let mut serial = white_canvas(WIDTH, HEIGHT, 100.0);
        serial
            .draw_markers_with_sprite_compositor_serial(
                &points,
                11.5,
                MarkerStyle::Circle,
                Color::from_rgba(35, 140, 220, 173),
                Some((Color::from_rgba(180, 30, 90, 211), 1.25)),
                clip_rect,
            )
            .expect("serial marker blit should render");

        let mut parallel = white_canvas(WIDTH, HEIGHT, 100.0);
        parallel
            .draw_markers_with_sprite_compositor_parallel(
                &points,
                11.5,
                MarkerStyle::Circle,
                Color::from_rgba(35, 140, 220, 173),
                Some((Color::from_rgba(180, 30, 90, 211), 1.25)),
                clip_rect,
                WORKERS,
            )
            .expect("parallel marker blit should render");

        assert_eq!(serial.pixmap.data(), parallel.pixmap.data());
        assert_eq!(
            serial.render_diagnostics(),
            parallel.render_diagnostics(),
            "parallel bands must report the same compositor/cache/blit totals"
        );
        assert!(serial.render_diagnostics().used_marker_scanline_blit);
        assert_eq!(
            serial.encode_png_bytes().expect("serial PNG should encode"),
            parallel
                .encode_png_bytes()
                .expect("parallel PNG should encode")
        );
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

    // ------------------------------------------------------------------
    // Non-finite geometry policy
    // ------------------------------------------------------------------

    fn has_ink(image: &Image, x: u32, y: u32) -> bool {
        pixel(image, x, y)[0] < 250
    }

    /// Bucket 1. A vertex the axes cannot represent is a *gap*: the runs either
    /// side of it are still stroked, and nothing is drawn across it.
    ///
    /// tiny-skia rejects a whole path whose bounds are not finite, so before the
    /// split this call failed outright — one unrepresentable sample on a log
    /// axis cost the entire series.
    #[test]
    fn test_polyline_breaks_at_a_non_finite_vertex_instead_of_losing_the_line() {
        let mut renderer = white_canvas(64, 32, 100.0);
        let points = [
            (4.0, 16.0),
            (16.0, 16.0),
            (f32::NAN, f32::NAN),
            (44.0, 16.0),
            (60.0, 16.0),
        ];

        renderer
            .draw_polyline(&points, Color::from_rgb(0, 0, 0), 2.0, LineStyle::Solid)
            .expect("a non-finite vertex must not fail the whole polyline");

        let image = renderer.into_image();
        assert!(
            has_ink(&image, 10, 16),
            "the run before the hole must still be stroked"
        );
        assert!(
            has_ink(&image, 52, 16),
            "the run after the hole must still be stroked"
        );
        assert!(
            !has_ink(&image, 30, 16),
            "the stroke must break at the hole, not run straight through it"
        );
    }

    /// The same rule for the projected/clipped twin, so the two polyline
    /// entry points cannot disagree about where a line breaks.
    #[test]
    fn test_clipped_point_polyline_breaks_at_a_non_finite_vertex() {
        let mut renderer = white_canvas(64, 32, 100.0);
        let points = [
            Point2f::new(4.0, 16.0),
            Point2f::new(16.0, 16.0),
            Point2f::new(f32::NAN, 16.0),
            Point2f::new(44.0, 16.0),
            Point2f::new(60.0, 16.0),
        ];

        renderer
            .draw_polyline_points_clipped(
                &points,
                Color::from_rgb(0, 0, 0),
                2.0,
                LineStyle::Solid,
                (0.0, 0.0, 64.0, 32.0),
            )
            .expect("a non-finite vertex must not fail the whole polyline");

        let image = renderer.into_image();
        assert!(has_ink(&image, 10, 16), "first run must survive");
        assert!(has_ink(&image, 52, 16), "second run must survive");
        assert!(!has_ink(&image, 30, 16), "the hole must stay a hole");
    }

    /// Bucket 1. A single unplaceable endpoint is a gap, not a failure.
    #[test]
    fn test_line_with_a_non_finite_endpoint_is_skipped_not_failed() {
        let mut renderer = white_canvas(32, 32, 100.0);
        renderer
            .draw_line(
                4.0,
                16.0,
                f32::NAN,
                16.0,
                Color::from_rgb(0, 0, 0),
                2.0,
                LineStyle::Solid,
            )
            .expect("an unplaceable segment is a gap, not an error");

        let image = renderer.into_image();
        assert!(
            image
                .pixels
                .chunks_exact(4)
                .all(|px| px.iter().all(|channel| *channel == 255)),
            "a segment with no endpoint must not paint anything"
        );
    }

    /// Bucket 2. The message must name the dimension and read as an internal
    /// invariant failure — the anonymous "Invalid rectangle dimensions" told a
    /// user neither which axis was at fault nor what to do about it.
    #[test]
    fn test_rectangle_with_a_non_finite_height_names_the_dimension() {
        let mut renderer = white_canvas(32, 32, 100.0);
        let error = renderer
            .draw_rectangle(4.0, 4.0, 8.0, f32::NAN, Color::from_rgb(0, 0, 0), true)
            .expect_err("a rectangle with no height must not reach tiny-skia");

        let message = error.to_string();
        assert!(
            message.contains("height"),
            "the message must name the offending dimension: {message}"
        );
        assert!(
            message.contains("internal invariant"),
            "the message must read as an unvalidated-input bug, not a user diagnostic: {message}"
        );
        assert!(
            !message.contains("Invalid rectangle dimensions"),
            "the anonymous message must be gone: {message}"
        );
    }

    /// Bucket 2 through the styled entry point, which is what bars and
    /// histogram columns use.
    #[test]
    fn test_styled_rectangle_with_a_non_finite_origin_names_the_dimension() {
        let mut renderer = white_canvas(32, 32, 100.0);
        let error = renderer
            .draw_rectangle_styled(
                f32::NAN,
                4.0,
                8.0,
                8.0,
                Some(Color::from_rgb(0, 0, 0)),
                None,
            )
            .expect_err("a rectangle with no origin must not reach tiny-skia");

        assert!(
            error.to_string().contains("non-finite x"),
            "the message must name x: {error}"
        );
    }

    /// The pixel-aligned tile path folded "non-finite" into the same `None` as
    /// "zero area" and quietly drew nothing, so a broken heatmap cell produced
    /// a silently wrong figure instead of a failure.
    #[test]
    fn test_pixel_aligned_rectangle_refuses_a_non_finite_tile() {
        let mut renderer = white_canvas(32, 32, 100.0);
        let error = renderer
            .draw_pixel_aligned_solid_rectangle(4.0, 4.0, f32::NAN, 8.0, Color::from_rgb(0, 0, 0))
            .expect_err("an unvalidated tile must be reported, not silently skipped");

        assert!(
            error.to_string().contains("non-finite width"),
            "the message must name width: {error}"
        );
    }

    /// Bucket 1 for markers. `NaN as i32` saturates to 0, so an unguarded
    /// sample was blitted into the top-left corner of the canvas and read as
    /// real data.
    #[test]
    fn test_marker_batch_skips_unplaceable_points() {
        let mut renderer = white_canvas(64, 64, 100.0);
        let points: Vec<Point2f> = std::iter::once(Point2f::new(f32::NAN, f32::NAN))
            .chain((0..64).map(|i| Point2f::new(32.0, 32.0 + (i % 3) as f32)))
            .collect();

        renderer
            .draw_markers_clipped(
                &points,
                6.0,
                MarkerStyle::Circle,
                Color::from_rgb(0, 0, 0),
                (0.0, 0.0, 64.0, 64.0),
            )
            .expect("unplaceable samples are skipped, not fatal");

        let image = renderer.into_image();
        assert!(
            has_ink(&image, 32, 32),
            "the placeable markers must still be drawn"
        );
        assert!(
            !has_ink(&image, 0, 0),
            "an unplaceable marker must not be painted in the canvas corner"
        );
    }

    /// Bucket 1 for the single-marker vector path.
    #[test]
    fn test_single_marker_with_a_non_finite_centre_is_skipped() {
        let mut renderer = white_canvas(32, 32, 100.0);
        renderer
            .draw_marker(
                f32::NAN,
                16.0,
                6.0,
                MarkerStyle::Circle,
                Color::from_rgb(0, 0, 0),
            )
            .expect("unplaceable markers are skipped, not fatal");

        let image = renderer.into_image();
        assert!(
            image
                .pixels
                .chunks_exact(4)
                .all(|px| px.iter().all(|channel| *channel == 255)),
            "an unplaceable marker must not touch the canvas"
        );
    }

    /// Bucket 2 for closed shapes: dropping a vertex would silently fill a
    /// different area, so the polygon is refused with the vertex named.
    #[test]
    fn test_filled_polygon_refuses_a_non_finite_vertex() {
        let mut renderer = white_canvas(32, 32, 100.0);
        let error = renderer
            .draw_filled_polygon(
                &[(4.0, 4.0), (28.0, 4.0), (16.0, f32::NAN)],
                Color::from_rgb(0, 0, 0),
            )
            .expect_err("a polygon with an unplaceable corner must not be filled");

        let message = error.to_string();
        assert!(
            message.contains("vertex 2"),
            "the message must name the offending vertex: {message}"
        );
        assert!(
            message.contains("internal invariant"),
            "the message must read as an unvalidated-input bug: {message}"
        );
    }
}
