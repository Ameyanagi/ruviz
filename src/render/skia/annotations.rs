use super::*;

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
