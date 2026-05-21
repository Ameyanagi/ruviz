use super::*;
use crate::axes::AxisScale;

struct AnnotationTransform<'a> {
    plot_area: Rect,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    x_scale: &'a AxisScale,
    y_scale: &'a AxisScale,
}

impl AnnotationTransform<'_> {
    fn point(&self, x: f64, y: f64) -> (f32, f32) {
        map_data_to_pixels_scaled(
            x,
            y,
            self.x_min,
            self.x_max,
            self.y_min,
            self.y_max,
            self.plot_area,
            self.x_scale,
            self.y_scale,
        )
    }

    fn x_pixel(&self, x: f64) -> f32 {
        let normalized = self.x_scale.normalized_position(x, self.x_min, self.x_max);
        self.plot_area.left() + normalized as f32 * self.plot_area.width()
    }

    fn y_pixel(&self, y: f64) -> f32 {
        let normalized = self.y_scale.normalized_position(y, self.y_min, self.y_max);
        self.plot_area.bottom() - normalized as f32 * self.plot_area.height()
    }
}

impl SkiaRenderer {
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
        self.draw_annotations_scaled(
            annotations,
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
            dpi,
            &AxisScale::Linear,
            &AxisScale::Linear,
        )
    }

    /// Render all annotations for a plot using configured axis scales.
    pub fn draw_annotations_scaled(
        &mut self,
        annotations: &[crate::core::Annotation],
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        dpi: f32,
        x_scale: &AxisScale,
        y_scale: &AxisScale,
    ) -> Result<()> {
        self.draw_annotations_where_scaled(
            annotations,
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
            dpi,
            x_scale,
            y_scale,
            |_| true,
        )
    }

    /// Render annotations matching a caller-provided predicate.
    pub fn draw_annotations_where<F>(
        &mut self,
        annotations: &[crate::core::Annotation],
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        dpi: f32,
        should_draw: F,
    ) -> Result<()>
    where
        F: FnMut(&crate::core::Annotation) -> bool,
    {
        self.draw_annotations_where_scaled(
            annotations,
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
            dpi,
            &AxisScale::Linear,
            &AxisScale::Linear,
            should_draw,
        )
    }

    /// Render annotations matching a caller-provided predicate using axis scales.
    pub fn draw_annotations_where_scaled<F>(
        &mut self,
        annotations: &[crate::core::Annotation],
        plot_area: Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        dpi: f32,
        x_scale: &AxisScale,
        y_scale: &AxisScale,
        mut should_draw: F,
    ) -> Result<()>
    where
        F: FnMut(&crate::core::Annotation) -> bool,
    {
        let transform = AnnotationTransform {
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
            x_scale,
            y_scale,
        };

        annotations
            .iter()
            .filter(|annotation| should_draw(annotation))
            .try_for_each(|annotation| self.draw_annotation(annotation, &transform, dpi))
    }

    /// Render a single annotation
    fn draw_annotation(
        &mut self,
        annotation: &crate::core::Annotation,
        transform: &AnnotationTransform<'_>,
        dpi: f32,
    ) -> Result<()> {
        use crate::core::Annotation;

        match annotation {
            Annotation::Text { x, y, text, style } => {
                self.draw_annotation_text(*x, *y, text, style, transform, dpi)
            }
            Annotation::Arrow {
                x1,
                y1,
                x2,
                y2,
                style,
            } => self.draw_annotation_arrow(*x1, *y1, *x2, *y2, style, transform, dpi),
            Annotation::HLine {
                y,
                style,
                color,
                width,
            } => self.draw_annotation_hline(*y, style, *color, *width, transform, dpi),
            Annotation::VLine {
                x,
                style,
                color,
                width,
            } => self.draw_annotation_vline(*x, style, *color, *width, transform, dpi),
            Annotation::Rectangle {
                x,
                y,
                width,
                height,
                style,
            } => self.draw_annotation_rect(*x, *y, *width, *height, style, transform),
            Annotation::FillBetween {
                x,
                y1,
                y2,
                style,
                where_positive,
            } => self.draw_annotation_fill_between(x, y1, y2, style, *where_positive, transform),
            Annotation::HSpan {
                x_min: xmin,
                x_max: xmax,
                style,
            } => self.draw_annotation_hspan(*xmin, *xmax, style, transform),
            Annotation::VSpan {
                y_min: ymin,
                y_max: ymax,
                style,
            } => self.draw_annotation_vspan(*ymin, *ymax, style, transform),
        }
    }

    /// Draw a text annotation at data coordinates
    fn draw_annotation_text(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        style: &crate::core::TextStyle,
        transform: &AnnotationTransform<'_>,
        dpi: f32,
    ) -> Result<()> {
        let (px, py) = transform.point(x, y);

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
        transform: &AnnotationTransform<'_>,
        dpi: f32,
    ) -> Result<()> {
        let (px1, py1) = transform.point(x1, y1);
        let (px2, py2) = transform.point(x2, y2);

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
        transform: &AnnotationTransform<'_>,
        dpi: f32,
    ) -> Result<()> {
        let py = transform.y_pixel(y);
        let line_width_px = pt_to_px(width, dpi);

        // Draw horizontal line spanning the plot area
        self.draw_line(
            transform.plot_area.left(),
            py,
            transform.plot_area.right(),
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
        transform: &AnnotationTransform<'_>,
        dpi: f32,
    ) -> Result<()> {
        let px = transform.x_pixel(x);
        let line_width_px = pt_to_px(width, dpi);

        // Draw vertical line spanning the plot area
        self.draw_line(
            px,
            transform.plot_area.top(),
            px,
            transform.plot_area.bottom(),
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
        transform: &AnnotationTransform<'_>,
    ) -> Result<()> {
        let (px1, py1) = transform.point(x, y + height);
        let (px2, py2) = transform.point(x + width, y);

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

                if let Some(dash_pattern) = self.scaled_dash_pattern(&style.edge_style) {
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
        transform: &AnnotationTransform<'_>,
    ) -> Result<()> {
        if x.len() < 2 || x.len() != y1.len() || x.len() != y2.len() {
            return Ok(()); // Nothing to draw
        }

        // Build polygon path
        let mut path = PathBuilder::new();

        // Forward along y1
        let (start_x, start_y) = transform.point(x[0], y1[0]);
        path.move_to(start_x, start_y);

        for i in 1..x.len() {
            if !where_positive || y1[i] >= y2[i] {
                let (px, py) = transform.point(x[i], y1[i]);
                path.line_to(px, py);
            } else {
                let (px, py) = transform.point(x[i], y2[i]);
                path.line_to(px, py);
            }
        }

        // Backward along y2 (in reverse order)
        for i in (0..x.len()).rev() {
            if !where_positive || y1[i] >= y2[i] {
                let (px, py) = transform.point(x[i], y2[i]);
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
        transform: &AnnotationTransform<'_>,
    ) -> Result<()> {
        let px_min = transform.x_pixel(span_x_min);
        let px_max = transform.x_pixel(span_x_max);

        // Clamp to plot area
        let left = px_min
            .max(transform.plot_area.left())
            .min(transform.plot_area.right());
        let right = px_max
            .max(transform.plot_area.left())
            .min(transform.plot_area.right());

        if let Some(rect) = Rect::from_xywh(
            left,
            transform.plot_area.top(),
            right - left,
            transform.plot_area.height(),
        ) {
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
        transform: &AnnotationTransform<'_>,
    ) -> Result<()> {
        let py_max = transform.y_pixel(span_y_min);
        let py_min = transform.y_pixel(span_y_max);

        // Clamp to plot area
        let top = py_min
            .max(transform.plot_area.top())
            .min(transform.plot_area.bottom());
        let bottom = py_max
            .max(transform.plot_area.top())
            .min(transform.plot_area.bottom());

        if let Some(rect) = Rect::from_xywh(
            transform.plot_area.left(),
            top,
            transform.plot_area.width(),
            bottom - top,
        ) {
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
