use super::*;

impl Plot {
    pub(super) fn calculate_total_points(&self) -> usize {
        Self::calculate_total_points_for_series(&self.series_mgr.series)
    }

    pub(super) fn calculate_total_points_for_series(series_list: &[PlotSeries]) -> usize {
        series_list
            .iter()
            .map(|series| match &series.series_type {
                SeriesType::Line { x_data, .. }
                | SeriesType::Scatter { x_data, .. }
                | SeriesType::ErrorBars { x_data, .. }
                | SeriesType::ErrorBarsXY { x_data, .. } => x_data.len(),
                SeriesType::Bar { categories, .. } => categories.len(),
                SeriesType::Histogram { data, .. } => data.len(),
                SeriesType::BoxPlot { data, .. } => data.len(),
                SeriesType::Heatmap { data } => data.n_rows * data.n_cols,
                SeriesType::Kde { data } => data.x.len(),
                SeriesType::Ecdf { data } => data.x.len(),
                SeriesType::Violin { data } => data.data.len(),
                SeriesType::Boxen { data } => data.boxes.len() * 4, // Each box has 4 points
                SeriesType::Contour { data } => data.x.len() * data.y.len(),
                SeriesType::Pie { data } => data.values.len(),
                SeriesType::Radar { data } => data.series.iter().map(|s| s.values.len()).sum(),
                SeriesType::Polar { data } => data.points.len(),
            })
            .sum()
    }

    pub(super) fn should_auto_use_datashader(
        series_list: &[PlotSeries],
        total_points: usize,
    ) -> bool {
        // Keep auto-selection conservative for mixed charts until we can shade only the
        // aggregation-safe series without changing the semantics of the rest of the plot.
        DataShader::should_activate(total_points)
            && series_list
                .iter()
                .all(Self::series_supports_auto_datashader)
    }

    pub(super) fn series_supports_auto_datashader(series: &PlotSeries) -> bool {
        matches!(series.series_type, SeriesType::Scatter { .. })
    }

    pub(super) fn is_non_cartesian_series(series: &PlotSeries) -> bool {
        matches!(
            series.series_type,
            SeriesType::Pie { .. } | SeriesType::Radar { .. } | SeriesType::Polar { .. }
        )
    }

    pub(super) fn is_cartesian_series(series: &PlotSeries) -> bool {
        !Self::is_non_cartesian_series(series)
    }

    pub(super) fn has_cartesian_series(series_list: &[PlotSeries]) -> bool {
        series_list.iter().any(Self::is_cartesian_series)
    }

    pub(super) fn has_non_cartesian_series(series_list: &[PlotSeries]) -> bool {
        series_list.iter().any(Self::is_non_cartesian_series)
    }

    pub(super) fn has_mixed_coordinate_series(series_list: &[PlotSeries]) -> bool {
        Self::has_cartesian_series(series_list) && Self::has_non_cartesian_series(series_list)
    }

    pub(super) fn needs_cartesian_axes_for_series(series_list: &[PlotSeries]) -> bool {
        series_list.is_empty() || Self::has_cartesian_series(series_list)
    }

    pub(super) fn render_series_collection_auto_datashader(
        &self,
        series_list: &[PlotSeries],
        renderer: &mut SkiaRenderer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        render_scale: RenderScale,
    ) -> Result<bool> {
        if Self::has_mixed_coordinate_series(series_list) {
            return Ok(false);
        }

        let total_points = Self::calculate_total_points_for_series(series_list);
        if !Self::should_auto_use_datashader(series_list, total_points) {
            return Ok(false);
        }

        let inset_rects = self.inset_rects_for_series(series_list, plot_area, render_scale)?;

        for (idx, series) in series_list.iter().enumerate() {
            let (series_area, series_bounds) = if let Some(inset_rect) = inset_rects[idx] {
                (inset_rect, self.raw_bounds_for_single_series(series)?)
            } else {
                (plot_area, (x_min, x_max, y_min, y_max))
            };

            match &series.series_type {
                SeriesType::Scatter { x_data, y_data } => {
                    let x_data = x_data.resolve(0.0);
                    let y_data = y_data.resolve(0.0);
                    let mut datashader = DataShader::with_canvas_size(
                        series_area.width() as usize,
                        series_area.height() as usize,
                    );

                    datashader.aggregate_with_bounds(
                        &x_data,
                        &y_data,
                        series_bounds.0,
                        series_bounds.1,
                        series_bounds.2,
                        series_bounds.3,
                    )?;
                    let image = datashader.render();
                    renderer.draw_datashader_image(&image, series_area)?;
                }
                _ => {
                    self.render_series_normal(
                        series,
                        renderer,
                        series_area,
                        series_bounds.0,
                        series_bounds.1,
                        series_bounds.2,
                        series_bounds.3,
                    )?;
                }
            }
        }

        Ok(true)
    }

    pub(super) fn empty_cartesian_bounds(&self) -> (f64, f64, f64, f64) {
        self.apply_manual_axis_limits((0.0, 1.0, 0.0, 1.0))
    }

    pub(super) fn effective_main_panel_bounds_for_series(
        &self,
        series_list: &[PlotSeries],
    ) -> Result<(f64, f64, f64, f64)> {
        if series_list.is_empty() {
            return Ok(self.empty_cartesian_bounds());
        }

        if Self::has_mixed_coordinate_series(series_list) {
            let cartesian_series: Vec<PlotSeries> = series_list
                .iter()
                .filter(|series| Self::is_cartesian_series(series))
                .cloned()
                .collect();
            self.effective_data_bounds_for_series(&cartesian_series)
        } else {
            self.effective_data_bounds_for_series(series_list)
        }
    }

    pub(super) fn raw_bounds_for_single_series(
        &self,
        series: &PlotSeries,
    ) -> Result<(f64, f64, f64, f64)> {
        self.calculate_data_bounds_for_series(std::slice::from_ref(series))
    }

    pub(super) fn clamp_inset_rect(
        plot_area: tiny_skia::Rect,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Result<tiny_skia::Rect> {
        let clamped_width = width.max(1.0).min(plot_area.width());
        let clamped_height = height.max(1.0).min(plot_area.height());
        let max_x = (plot_area.x() + plot_area.width() - clamped_width).max(plot_area.x());
        let max_y = (plot_area.y() + plot_area.height() - clamped_height).max(plot_area.y());
        let clamped_x = x.clamp(plot_area.x(), max_x);
        let clamped_y = y.clamp(plot_area.y(), max_y);

        tiny_skia::Rect::from_ltrb(
            clamped_x,
            clamped_y,
            clamped_x + clamped_width,
            clamped_y + clamped_height,
        )
        .ok_or(PlottingError::InvalidData {
            message: "Invalid inset plot area".to_string(),
            position: None,
        })
    }

    pub(super) fn explicit_inset_rect(
        plot_area: tiny_skia::Rect,
        layout: InsetLayout,
        render_scale: RenderScale,
    ) -> Result<tiny_skia::Rect> {
        let layout = layout.normalized();
        let margin_px = render_scale.points_to_pixels(layout.margin_pt);
        let width_px = plot_area.width() * layout.width_frac;
        let height_px = plot_area.height() * layout.height_frac;
        let left = plot_area.x();
        let top = plot_area.y();
        let right = plot_area.x() + plot_area.width();
        let bottom = plot_area.y() + plot_area.height();

        let (x, y) = match layout.anchor {
            InsetAnchor::Auto => (right - width_px - margin_px, top + margin_px),
            InsetAnchor::TopLeft => (left + margin_px, top + margin_px),
            InsetAnchor::TopRight => (right - width_px - margin_px, top + margin_px),
            InsetAnchor::BottomLeft => (left + margin_px, bottom - height_px - margin_px),
            InsetAnchor::BottomRight => {
                (right - width_px - margin_px, bottom - height_px - margin_px)
            }
            InsetAnchor::TopCenter => {
                (left + (plot_area.width() - width_px) * 0.5, top + margin_px)
            }
            InsetAnchor::BottomCenter => (
                left + (plot_area.width() - width_px) * 0.5,
                bottom - height_px - margin_px,
            ),
            InsetAnchor::CenterLeft => (
                left + margin_px,
                top + (plot_area.height() - height_px) * 0.5,
            ),
            InsetAnchor::CenterRight => (
                right - width_px - margin_px,
                top + (plot_area.height() - height_px) * 0.5,
            ),
            InsetAnchor::Center => (
                left + (plot_area.width() - width_px) * 0.5,
                top + (plot_area.height() - height_px) * 0.5,
            ),
            InsetAnchor::Custom { x_frac, y_frac } => (
                left + x_frac.clamp(0.0, 1.0) * plot_area.width() - width_px * 0.5,
                top + y_frac.clamp(0.0, 1.0) * plot_area.height() - height_px * 0.5,
            ),
        };

        Self::clamp_inset_rect(plot_area, x, y, width_px, height_px)
    }

    pub(super) fn inset_rects_for_series(
        &self,
        series_list: &[PlotSeries],
        plot_area: tiny_skia::Rect,
        render_scale: RenderScale,
    ) -> Result<Vec<Option<tiny_skia::Rect>>> {
        let mut rects = vec![None; series_list.len()];
        if !Self::has_mixed_coordinate_series(series_list) {
            return Ok(rects);
        }

        let mut auto_series = Vec::new();
        let mut auto_cell_height = 0.0_f32;
        let mut auto_gap = 0.0_f32;

        for (idx, series) in series_list.iter().enumerate() {
            if !Self::is_non_cartesian_series(series) {
                continue;
            }

            let layout = series.inset_layout.unwrap_or_default().normalized();
            if matches!(layout.anchor, InsetAnchor::Auto) {
                let width_px = plot_area.width() * layout.width_frac;
                let height_px = plot_area.height() * layout.height_frac;
                auto_cell_height = auto_cell_height.max(height_px);
                auto_gap = auto_gap.max(render_scale.points_to_pixels(layout.margin_pt));
                auto_series.push((idx, layout, width_px, height_px));
            } else {
                rects[idx] = Some(Self::explicit_inset_rect(plot_area, layout, render_scale)?);
            }
        }

        if auto_series.is_empty() {
            return Ok(rects);
        }

        let cols = if auto_series.len() <= 1 { 1 } else { 2 };
        let gap = auto_gap.max(4.0);

        for (row, row_series) in auto_series.chunks(cols).enumerate() {
            let x = plot_area.x() + plot_area.width() - gap;
            let y = plot_area.y() + gap + row as f32 * (auto_cell_height + gap);
            let mut right_edge = x;

            for (idx, _layout, width_px, height_px) in row_series.iter().copied() {
                let inset_x = right_edge - width_px;
                rects[idx] = Some(Self::clamp_inset_rect(
                    plot_area, inset_x, y, width_px, height_px,
                )?);
                right_edge = inset_x - gap;
            }
        }

        Ok(rects)
    }

    pub(super) fn radar_plot_area(
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> crate::plots::PlotArea {
        let size = plot_area.width().min(plot_area.height());
        let x_offset = (plot_area.width() - size) * 0.5;
        let y_offset = (plot_area.height() - size) * 0.5;
        // Leave headroom for the top axis label while keeping portrait insets centered.
        let title_clearance = size * 0.20;
        let adjusted_size = (size - title_clearance).max(1.0);

        crate::plots::PlotArea::new(
            plot_area.x() + x_offset,
            plot_area.y() + y_offset + title_clearance,
            adjusted_size,
            adjusted_size,
            x_min,
            x_max,
            y_min,
            y_max,
        )
    }

    pub(super) fn render_series_collection_normal(
        &self,
        series_list: &[PlotSeries],
        renderer: &mut SkiaRenderer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        render_scale: RenderScale,
    ) -> Result<()> {
        let inset_rects = self.inset_rects_for_series(series_list, plot_area, render_scale)?;

        for (idx, series) in series_list.iter().enumerate() {
            let (series_area, series_bounds) = if let Some(inset_rect) = inset_rects[idx] {
                (inset_rect, self.raw_bounds_for_single_series(series)?)
            } else {
                (plot_area, (x_min, x_max, y_min, y_max))
            };

            self.render_series_normal(
                series,
                renderer,
                series_area,
                series_bounds.0,
                series_bounds.1,
                series_bounds.2,
                series_bounds.3,
            )?;
        }

        Ok(())
    }

    pub(super) fn render_series_svg(
        &self,
        svg: &mut crate::export::SvgRenderer,
        series: &PlotSeries,
        default_color: Color,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<()> {
        let color = series.color.unwrap_or(default_color);
        let render_scale = self.render_scale();
        let line_width = render_scale
            .points_to_pixels(series.line_width.unwrap_or(self.display.theme.line_width));
        let line_style = series.line_style.clone().unwrap_or(LineStyle::Solid);

        match &series.series_type {
            SeriesType::Line { x_data, y_data } => {
                let x_data = x_data.resolve(0.0);
                let y_data = y_data.resolve(0.0);
                let points: Vec<(f32, f32)> = x_data
                    .iter()
                    .zip(y_data.iter())
                    .map(|(&x, &y)| {
                        crate::render::skia::map_data_to_pixels(
                            x, y, x_min, x_max, y_min, y_max, plot_area,
                        )
                    })
                    .collect();

                svg.draw_polyline(&points, color, line_width, line_style);
                if let Some(marker_style) = series.marker_style {
                    let marker_size =
                        render_scale.points_to_pixels(series.marker_size.unwrap_or(6.0));
                    for &(px, py) in &points {
                        svg.draw_marker(px, py, marker_size, marker_style, color);
                    }
                }
            }
            SeriesType::Scatter { x_data, y_data } => {
                let x_data = x_data.resolve(0.0);
                let y_data = y_data.resolve(0.0);
                let marker_style = series.marker_style.unwrap_or(MarkerStyle::Circle);
                let marker_size = render_scale.points_to_pixels(series.marker_size.unwrap_or(6.0));
                for (&x, &y) in x_data.iter().zip(y_data.iter()) {
                    let (px, py) = crate::render::skia::map_data_to_pixels(
                        x, y, x_min, x_max, y_min, y_max, plot_area,
                    );
                    svg.draw_marker(px, py, marker_size, marker_style, color);
                }
            }
            SeriesType::Bar { categories, values } => {
                let values = values.resolve(0.0);
                let num_bars = categories.len();
                let bar_width = plot_area.width() / num_bars as f32 * 0.7;

                for (i, &value) in values.iter().enumerate() {
                    let bar_x = plot_area.x()
                        + (i as f32 + 0.5) * (plot_area.width() / num_bars as f32)
                        - bar_width / 2.0;
                    let (_, py) = crate::render::skia::map_data_to_pixels(
                        0.0, value, x_min, x_max, y_min, y_max, plot_area,
                    );
                    let (_, py_zero) = crate::render::skia::map_data_to_pixels(
                        0.0, 0.0, x_min, x_max, y_min, y_max, plot_area,
                    );
                    let bar_height = (py - py_zero).abs();
                    let bar_y = py.min(py_zero);

                    svg.draw_rectangle(bar_x, bar_y, bar_width, bar_height, color, true);
                }
            }
            SeriesType::Pie { data } => {
                self.render_pie_series_svg(svg, data, plot_area)?;
            }
            SeriesType::Radar { data } => {
                self.render_radar_series_svg(svg, data, plot_area)?;
            }
            SeriesType::Polar { data } => {
                self.render_polar_series_svg(
                    svg, data, plot_area, x_min, x_max, y_min, y_max, color,
                )?;
            }
            SeriesType::Histogram { .. } => {}
            _ => {}
        }

        Ok(())
    }

    pub(super) fn render_pie_series_svg(
        &self,
        svg: &mut crate::export::SvgRenderer,
        data: &crate::plots::composition::pie::PieData,
        plot_area: tiny_skia::Rect,
    ) -> Result<()> {
        if data.wedges.is_empty() {
            return Ok(());
        }

        let size = plot_area.width().min(plot_area.height());
        let cx = plot_area.x() + plot_area.width() * 0.5;
        let cy = plot_area.y() + plot_area.height() * 0.5;
        let radius = size * 0.45;
        let screen_data = crate::plots::composition::pie::PieData::from_values(
            &data.values,
            cx as f64,
            cy as f64,
            radius as f64,
            &data.config,
        );
        let colors = if let Some(ref colors) = data.config.colors {
            colors.clone()
        } else {
            let palette = self.display.theme.color_palette.clone();
            (0..screen_data.wedges.len())
                .map(|i| palette[i % palette.len()])
                .collect()
        };
        let segments = 64;

        if data.config.shadow > 0.0 {
            let shadow_color = Color::new(100, 100, 100).with_alpha(0.3);
            let offset = data.config.shadow;
            for wedge in &screen_data.wedges {
                let polygon: Vec<(f32, f32)> = wedge
                    .as_polygon(segments)
                    .iter()
                    .map(|(x, y)| ((x + offset) as f32, (y + offset) as f32))
                    .collect();
                svg.draw_filled_polygon(&polygon, shadow_color);
            }
        }

        for (idx, wedge) in screen_data.wedges.iter().enumerate() {
            let polygon: Vec<(f32, f32)> = wedge
                .as_polygon(segments)
                .iter()
                .map(|(x, y)| (*x as f32, *y as f32))
                .collect();
            svg.draw_filled_polygon(&polygon, colors[idx % colors.len()]);
            if let Some(edge_color) = data.config.edge_color {
                let scaled_edge_width = svg.render_scale().points_to_pixels(data.config.edge_width);
                svg.draw_polygon_outline(&polygon, edge_color, scaled_edge_width);
            }
        }

        if data.config.show_labels || data.config.show_percentages || data.config.show_values {
            for (idx, wedge) in screen_data.wedges.iter().enumerate() {
                let label_parts: Vec<String> = [
                    if data.config.show_labels && idx < data.config.labels.len() {
                        Some(data.config.labels[idx].clone())
                    } else {
                        None
                    },
                    if data.config.show_percentages {
                        Some(format!("{:.1}%", screen_data.percentages[idx]))
                    } else {
                        None
                    },
                    if data.config.show_values {
                        Some(format!("{:.1}", screen_data.values[idx]))
                    } else {
                        None
                    },
                ]
                .into_iter()
                .flatten()
                .collect();

                if !label_parts.is_empty() {
                    let label = label_parts.join("\n");
                    let label_r = if data.config.inner_radius > 0.0 {
                        radius as f64 * (1.0 + data.config.inner_radius) / 2.0
                            * data.config.label_distance
                    } else {
                        radius as f64 * data.config.label_distance
                    };
                    let mid_angle = (wedge.start_angle + wedge.end_angle) / 2.0;
                    let label_x = cx as f64 + label_r * mid_angle.cos();
                    let label_y = cy as f64 + label_r * mid_angle.sin();
                    svg.draw_text_centered(
                        &label,
                        label_x as f32,
                        label_y as f32,
                        data.config.label_font_size,
                        data.config.text_color,
                    )?;
                }
            }
        }

        Ok(())
    }

    pub(super) fn render_radar_series_svg(
        &self,
        svg: &mut crate::export::SvgRenderer,
        data: &crate::plots::polar::radar::RadarPlotData,
        plot_area: tiny_skia::Rect,
    ) -> Result<()> {
        if data.series.is_empty() {
            return Ok(());
        }

        let area = Self::radar_plot_area(plot_area, -1.25, 1.25, -1.25, 1.25);

        if data.config.show_grid {
            let grid_color = self.display.theme.grid_color;
            let grid_line_width = svg.render_scale().logical_pixels_to_pixels(0.5);
            for ring in &data.grid_rings {
                if ring.len() < 2 {
                    continue;
                }
                for idx in 0..ring.len() {
                    let (x1, y1) = ring[idx];
                    let (x2, y2) = ring[(idx + 1) % ring.len()];
                    let (sx1, sy1) = area.data_to_screen(x1, y1);
                    let (sx2, sy2) = area.data_to_screen(x2, y2);
                    svg.draw_line(
                        sx1,
                        sy1,
                        sx2,
                        sy2,
                        grid_color,
                        grid_line_width,
                        LineStyle::Solid,
                    );
                }
            }

            for &((x1, y1), (x2, y2)) in &data.axes {
                let (sx1, sy1) = area.data_to_screen(x1, y1);
                let (sx2, sy2) = area.data_to_screen(x2, y2);
                svg.draw_line(
                    sx1,
                    sy1,
                    sx2,
                    sy2,
                    grid_color,
                    grid_line_width,
                    LineStyle::Solid,
                );
            }
        }

        if data.config.show_axis_labels {
            for (label, x, y) in &data.axis_labels {
                let (sx, sy) = area.data_to_screen(*x, *y);
                svg.draw_text_centered(
                    label,
                    sx,
                    sy,
                    data.config.label_font_size,
                    self.display.theme.foreground,
                )?;
            }
        }

        let scaled_line_width = svg.render_scale().points_to_pixels(data.config.line_width);
        let scaled_marker_size = svg.render_scale().points_to_pixels(data.config.marker_size);
        for (series_idx, series_data) in data.series.iter().enumerate() {
            let series_color = data
                .config
                .colors
                .as_ref()
                .and_then(|colors| colors.get(series_idx).copied())
                .unwrap_or_else(|| self.display.theme.get_color(series_idx));

            if data.config.fill && !series_data.polygon.is_empty() {
                let polygon: Vec<(f32, f32)> = series_data
                    .polygon
                    .iter()
                    .map(|(x, y)| area.data_to_screen(*x, *y))
                    .collect();
                svg.draw_filled_polygon(&polygon, series_color.with_alpha(data.config.fill_alpha));
            }

            if series_data.polygon.len() > 1 {
                let polygon: Vec<(f32, f32)> = series_data
                    .polygon
                    .iter()
                    .map(|(x, y)| area.data_to_screen(*x, *y))
                    .collect();
                svg.draw_polygon_outline(&polygon, series_color, scaled_line_width);
            }

            if data.config.marker_size > 0.0 {
                for (x, y) in &series_data.markers {
                    let (sx, sy) = area.data_to_screen(*x, *y);
                    svg.draw_marker(
                        sx,
                        sy,
                        scaled_marker_size,
                        MarkerStyle::Circle,
                        series_color,
                    );
                }
            }
        }

        Ok(())
    }

    pub(super) fn render_polar_series_svg(
        &self,
        svg: &mut crate::export::SvgRenderer,
        data: &crate::plots::polar::polar_plot::PolarPlotData,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        default_color: Color,
    ) -> Result<()> {
        if data.points.is_empty() {
            return Ok(());
        }

        let size = plot_area.width().min(plot_area.height());
        let x_offset = (plot_area.width() - size) * 0.5;
        let y_offset = (plot_area.height() - size) * 0.5;
        let area = crate::plots::PlotArea::new(
            plot_area.x() + x_offset,
            plot_area.y() + y_offset,
            size,
            size,
            x_min,
            x_max,
            y_min,
            y_max,
        );
        let line_color = data.config.color.unwrap_or(default_color);

        if data.config.fill && !data.fill_polygon.is_empty() {
            let polygon: Vec<(f32, f32)> = data
                .fill_polygon
                .iter()
                .map(|(x, y)| area.data_to_screen(*x, *y))
                .collect();
            svg.draw_filled_polygon(&polygon, line_color.with_alpha(data.config.fill_alpha));
        }

        if data.points.len() > 1 {
            let points: Vec<(f32, f32)> = data
                .points
                .iter()
                .map(|point| area.data_to_screen(point.x, point.y))
                .collect();
            let scaled_line_width = svg.render_scale().points_to_pixels(data.config.line_width);
            svg.draw_polyline(&points, line_color, scaled_line_width, LineStyle::Solid);
        }

        if data.config.marker_size > 0.0 {
            let scaled_marker_size = svg.render_scale().points_to_pixels(data.config.marker_size);
            for point in &data.points {
                let (sx, sy) = area.data_to_screen(point.x, point.y);
                svg.draw_marker(sx, sy, scaled_marker_size, MarkerStyle::Circle, line_color);
            }
        }

        for label in &data.theta_labels {
            let (sx, sy) = area.data_to_screen(label.x, label.y);
            svg.draw_text_centered(
                &label.text,
                sx,
                sy,
                data.config.label_font_size,
                self.display.theme.foreground,
            )?;
        }

        for label in &data.r_labels {
            let (sx, sy) = area.data_to_screen(label.x, label.y);
            svg.draw_text_centered(
                &label.text,
                sx,
                sy,
                data.config.label_font_size,
                self.display.theme.foreground,
            )?;
        }

        Ok(())
    }

    /// Check if the plot needs standard Cartesian axes
    ///
    /// Some plot types (Pie, Radar, Polar) have their own coordinate system
    /// and don't use standard X/Y axes with tick labels.
    pub(super) fn needs_cartesian_axes(&self) -> bool {
        Self::needs_cartesian_axes_for_series(&self.series_mgr.series)
    }

    pub(super) fn apply_manual_axis_limits(
        &self,
        bounds: (f64, f64, f64, f64),
    ) -> (f64, f64, f64, f64) {
        let (mut x_min, mut x_max, mut y_min, mut y_max) = bounds;

        if let Some((x_min_manual, x_max_manual)) = self.layout.x_limits {
            x_min = x_min_manual;
            x_max = x_max_manual;
        }

        if let Some((y_min_manual, y_max_manual)) = self.layout.y_limits {
            y_min = y_min_manual;
            y_max = y_max_manual;
        }

        if (x_max - x_min).abs() < f64::EPSILON {
            x_min -= 1.0;
            x_max += 1.0;
        }
        if (y_max - y_min).abs() < f64::EPSILON {
            y_min -= 1.0;
            y_max += 1.0;
        }

        (x_min, x_max, y_min, y_max)
    }

    pub(super) fn effective_data_bounds(&self) -> Result<(f64, f64, f64, f64)> {
        if self.series_mgr.series.is_empty() {
            return Ok(self.empty_cartesian_bounds());
        }

        self.calculate_data_bounds()
            .map(|bounds| self.apply_manual_axis_limits(bounds))
    }

    pub(super) fn effective_data_bounds_for_series(
        &self,
        series_list: &[PlotSeries],
    ) -> Result<(f64, f64, f64, f64)> {
        if series_list.is_empty() {
            return Ok(self.empty_cartesian_bounds());
        }

        self.calculate_data_bounds_for_series(series_list)
            .map(|bounds| self.apply_manual_axis_limits(bounds))
    }

    pub(super) fn effective_data_bounds_from_resolved(
        &self,
        resolved_series: &[ResolvedSeries<'_>],
    ) -> Result<(f64, f64, f64, f64)> {
        if resolved_series.is_empty() {
            return Ok(self.empty_cartesian_bounds());
        }

        self.calculate_data_bounds_from_resolved(resolved_series)
            .map(|bounds| self.apply_manual_axis_limits(bounds))
    }

    pub(super) fn apply_auto_padding_to_bounds(
        &self,
        bounds: (f64, f64, f64, f64),
        fraction: f64,
    ) -> (f64, f64, f64, f64) {
        let (mut x_min, mut x_max, mut y_min, mut y_max) = bounds;

        if self.layout.x_limits.is_none() {
            let x_range = x_max - x_min;
            x_min -= x_range * fraction;
            x_max += x_range * fraction;
        }

        if self.layout.y_limits.is_none() {
            let y_range = y_max - y_min;
            y_min -= y_range * fraction;
            y_max += y_range * fraction;
        }

        self.apply_manual_axis_limits((x_min, x_max, y_min, y_max))
    }

    /// Helper to render attached error bars on Line/Scatter series
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_attached_error_bars(
        renderer: &mut SkiaRenderer,
        x_data: &[f64],
        y_data: &[f64],
        y_errors: Option<&ErrorValues>,
        x_errors: Option<&ErrorValues>,
        error_config: Option<&ErrorBarConfig>,
        series_color: Color,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        plot_area: tiny_skia::Rect,
        default_line_width: f32,
        render_scale: RenderScale,
    ) -> Result<()> {
        let config = error_config.cloned().unwrap_or_default();
        let bar_color = config
            .color
            .unwrap_or(series_color)
            .with_alpha(config.alpha);
        // Error-bar configuration is still authored in legacy logical pixels.
        let scaled_config_width = render_scale.logical_pixels_to_pixels(config.line_width);
        let line_width = scaled_config_width.max(default_line_width * 0.75); // Slightly thinner than data line

        let cap_size_px = render_scale.logical_pixels_to_pixels(config.cap_size);
        let half_cap = cap_size_px / 2.0;

        let n = x_data.len().min(y_data.len());

        for i in 0..n {
            let x_val = x_data[i];
            let y_val = y_data[i];

            if !x_val.is_finite() || !y_val.is_finite() {
                continue;
            }

            // Get Y error bounds (skip NaN/Infinity values)
            let (y_lower, y_upper) = if let Some(yerr) = y_errors {
                if let Some((lo, hi)) = yerr.bounds_at(i) {
                    let lo_abs = lo.abs();
                    let hi_abs = hi.abs();
                    if lo_abs.is_finite() && hi_abs.is_finite() {
                        (lo_abs, hi_abs)
                    } else {
                        (0.0, 0.0) // Skip invalid error values
                    }
                } else {
                    (0.0, 0.0)
                }
            } else {
                (0.0, 0.0)
            };

            // Get X error bounds (skip NaN/Infinity values)
            let (x_lower, x_upper) = if let Some(xerr) = x_errors {
                if let Some((lo, hi)) = xerr.bounds_at(i) {
                    let lo_abs = lo.abs();
                    let hi_abs = hi.abs();
                    if lo_abs.is_finite() && hi_abs.is_finite() {
                        (lo_abs, hi_abs)
                    } else {
                        (0.0, 0.0) // Skip invalid error values
                    }
                } else {
                    (0.0, 0.0)
                }
            } else {
                (0.0, 0.0)
            };

            // Draw Y error bar (vertical line + caps) with clipping
            if y_lower > 0.0 || y_upper > 0.0 {
                let (px, py_top_raw) = map_data_to_pixels(
                    x_val,
                    y_val + y_upper,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );
                let (_, py_bottom_raw) = map_data_to_pixels(
                    x_val,
                    y_val - y_lower,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );

                // Clip to plot area bounds
                let plot_top = plot_area.y();
                let plot_bottom = plot_area.y() + plot_area.height();
                let plot_left = plot_area.x();
                let plot_right = plot_area.x() + plot_area.width();

                let py_top = py_top_raw.max(plot_top).min(plot_bottom);
                let py_bottom = py_bottom_raw.max(plot_top).min(plot_bottom);

                // Only draw if there's visible area
                if (py_bottom - py_top).abs() > 0.5 {
                    // Vertical error line (clipped)
                    renderer.draw_line(
                        px,
                        py_top,
                        px,
                        py_bottom,
                        bar_color,
                        line_width,
                        LineStyle::Solid,
                    )?;

                    // Draw caps (horizontal lines) - use pixel offsets directly
                    let cap_left_x = (px - half_cap).max(plot_left);
                    let cap_right_x = (px + half_cap).min(plot_right);

                    // Top cap (only if within plot area)
                    if py_top_raw >= plot_top && py_top_raw <= plot_bottom {
                        renderer.draw_line(
                            cap_left_x,
                            py_top,
                            cap_right_x,
                            py_top,
                            bar_color,
                            line_width,
                            LineStyle::Solid,
                        )?;
                    }
                    // Bottom cap (only if within plot area)
                    if py_bottom_raw >= plot_top && py_bottom_raw <= plot_bottom {
                        renderer.draw_line(
                            cap_left_x,
                            py_bottom,
                            cap_right_x,
                            py_bottom,
                            bar_color,
                            line_width,
                            LineStyle::Solid,
                        )?;
                    }
                }
            }

            // Draw X error bar (horizontal line + caps) with clipping
            if x_lower > 0.0 || x_upper > 0.0 {
                let (_, py) =
                    map_data_to_pixels(x_val, y_val, x_min, x_max, y_min, y_max, plot_area);
                let (px_left_raw, _) = map_data_to_pixels(
                    x_val - x_lower,
                    y_val,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );
                let (px_right_raw, _) = map_data_to_pixels(
                    x_val + x_upper,
                    y_val,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );

                // Clip to plot area bounds
                let plot_top = plot_area.y();
                let plot_bottom = plot_area.y() + plot_area.height();
                let plot_left = plot_area.x();
                let plot_right = plot_area.x() + plot_area.width();

                let px_left = px_left_raw.max(plot_left).min(plot_right);
                let px_right = px_right_raw.max(plot_left).min(plot_right);

                // Only draw if there's visible area
                if (px_right - px_left).abs() > 0.5 {
                    // Horizontal error line (clipped)
                    renderer.draw_line(
                        px_left,
                        py,
                        px_right,
                        py,
                        bar_color,
                        line_width,
                        LineStyle::Solid,
                    )?;

                    // Draw caps (vertical lines) - use pixel offsets directly
                    let cap_top_y = (py - half_cap).max(plot_top);
                    let cap_bottom_y = (py + half_cap).min(plot_bottom);

                    // Left cap (only if within plot area)
                    if px_left_raw >= plot_left && px_left_raw <= plot_right {
                        renderer.draw_line(
                            px_left,
                            cap_top_y,
                            px_left,
                            cap_bottom_y,
                            bar_color,
                            line_width,
                            LineStyle::Solid,
                        )?;
                    }
                    // Right cap (only if within plot area)
                    if px_right_raw >= plot_left && px_right_raw <= plot_right {
                        renderer.draw_line(
                            px_right,
                            cap_top_y,
                            px_right,
                            cap_bottom_y,
                            bar_color,
                            line_width,
                            LineStyle::Solid,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }
}
