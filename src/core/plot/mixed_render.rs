use super::*;

use super::error_bars::{
    AttachedErrorBarStyle, ErrorBarFrame, draw_error_bars, error_bar_pixels_for_series,
    stroke_error_bar_series,
};
use crate::plots::traits::ComputedSeries;

/// Pad one axis range by `margin` (a fraction of the span) on each side.
///
/// The padding is applied in the axis' own transform space so a log axis gets a
/// constant *visual* margin and can never produce a non-positive lower bound.
/// Scales whose transform is not reachable from here (`SymLog`) and log ranges
/// that are not strictly positive are left untouched: skipping the margin is far
/// safer than applying a linear one to a non-linear axis.
///
/// `pad_low` / `pad_high` implement matplotlib's `sticky_edges`: a bar or
/// histogram baseline must keep sitting exactly on zero.
fn padded_axis_range(
    min: f64,
    max: f64,
    margin: f64,
    scale: &crate::axes::AxisScale,
    pad_low: bool,
    pad_high: bool,
) -> (f64, f64) {
    if !margin.is_finite()
        || margin <= 0.0
        || !min.is_finite()
        || !max.is_finite()
        || max <= min
        || (!pad_low && !pad_high)
    {
        return (min, max);
    }

    match scale {
        crate::axes::AxisScale::Linear => {
            let pad = (max - min) * margin;
            if !pad.is_finite() || pad <= 0.0 {
                return (min, max);
            }
            let low = if pad_low { min - pad } else { min };
            let high = if pad_high { max + pad } else { max };
            if low.is_finite() && high.is_finite() && high > low {
                (low, high)
            } else {
                (min, max)
            }
        }
        crate::axes::AxisScale::Log if min > 0.0 && max > 0.0 => {
            let log_min = min.log10();
            let log_max = max.log10();
            let pad = (log_max - log_min) * margin;
            if !pad.is_finite() || pad <= 0.0 {
                return (min, max);
            }
            let low = if pad_low {
                10.0_f64.powf(log_min - pad)
            } else {
                min
            };
            let high = if pad_high {
                10.0_f64.powf(log_max + pad)
            } else {
                max
            };
            if low.is_finite() && low > 0.0 && high.is_finite() && high > low {
                (low, high)
            } else {
                (min, max)
            }
        }
        _ => (min, max),
    }
}

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
                SeriesType::Quiver { data } => data.arrows.len(),
                SeriesType::Computed { data } => data.point_count(),
            })
            .sum()
    }

    pub(super) fn calculate_total_points_from_resolved(
        series_list: &[ResolvedSeries<'_>],
    ) -> usize {
        series_list
            .iter()
            .map(|series| match series {
                ResolvedSeries::Line { x, .. }
                | ResolvedSeries::Scatter { x, .. }
                | ResolvedSeries::ErrorBars { x, .. }
                | ResolvedSeries::ErrorBarsXY { x, .. } => x.len(),
                ResolvedSeries::Bar { categories, .. } => categories.len(),
                ResolvedSeries::Histogram { data } => data.counts.len(),
                ResolvedSeries::BoxPlot { data, .. } => data.len(),
                ResolvedSeries::Other(series) => match series {
                    SeriesType::Heatmap { data } => data.n_rows * data.n_cols,
                    SeriesType::Kde { data } => data.x.len(),
                    SeriesType::Ecdf { data } => data.x.len(),
                    SeriesType::Violin { data } => data.data.len(),
                    SeriesType::Boxen { data } => data.boxes.len() * 4,
                    SeriesType::Contour { data } => data.x.len() * data.y.len(),
                    SeriesType::Pie { data } => data.values.len(),
                    SeriesType::Radar { data } => {
                        data.series.iter().map(|series| series.values.len()).sum()
                    }
                    SeriesType::Polar { data } => data.points.len(),
                    SeriesType::Quiver { data } => data.arrows.len(),
                    SeriesType::Computed { data } => data.point_count(),
                    _ => unreachable!("PlotData-backed series resolve to dedicated variants"),
                },
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
            && series.x_errors.is_none()
            && series.y_errors.is_none()
    }

    pub(super) fn is_non_cartesian_series(series: &PlotSeries) -> bool {
        Self::is_non_cartesian_series_type(&series.series_type)
    }

    /// Whether a series type is drawn in its own polar frame rather than on the
    /// plot's Cartesian axes.
    ///
    /// Asked of the type alone because [`series_from_style`] has to answer it
    /// before there is a `PlotSeries` to ask — a non-Cartesian series always
    /// carries an inset placement, and that is where it is given one.
    ///
    /// [`series_from_style`]: super::series_api::series_from_style
    pub(super) fn is_non_cartesian_series_type(series_type: &SeriesType) -> bool {
        matches!(
            series_type,
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
        resolved_series: &[ResolvedSeries<'_>],
        renderer: &mut SkiaRenderer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        render_scale: RenderScale,
        mode: RenderExecutionMode,
    ) -> Result<bool> {
        if !mode.allows_auto_datashader() {
            return Ok(false);
        }

        if Self::has_mixed_coordinate_series(series_list) {
            return Ok(false);
        }

        let total_points = Self::calculate_total_points_from_resolved(resolved_series);
        if !self.should_use_datashader_for_render(series_list, total_points) {
            return Ok(false);
        }

        renderer.note_auto_datashader();

        let inset_rects = self.inset_rects_for_series(series_list, plot_area, render_scale)?;

        for (idx, (series, resolved)) in series_list.iter().zip(resolved_series).enumerate() {
            let (series_area, series_bounds) = if let Some(inset_rect) = inset_rects[idx] {
                (inset_rect, self.inset_bounds_from_resolved(resolved)?)
            } else {
                (plot_area, (x_min, x_max, y_min, y_max))
            };

            match (&series.series_type, resolved) {
                (SeriesType::Scatter { .. }, ResolvedSeries::Scatter { x, y }) => {
                    let mut datashader = DataShader::with_canvas_size(
                        series_area.width() as usize,
                        series_area.height() as usize,
                    );

                    datashader.aggregate_with_bounds(
                        x,
                        y,
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
                        resolved,
                        renderer,
                        series_area,
                        series_bounds.0,
                        series_bounds.1,
                        series_bounds.2,
                        series_bounds.3,
                        mode,
                    )?;
                }
            }
        }

        Ok(true)
    }

    pub(super) fn empty_cartesian_bounds(&self) -> (f64, f64, f64, f64) {
        let x = if matches!(&self.layout.x_scale, crate::axes::AxisScale::Log) {
            (1.0, 10.0)
        } else {
            (0.0, 1.0)
        };
        let y = if matches!(&self.layout.y_scale, crate::axes::AxisScale::Log) {
            (1.0, 10.0)
        } else {
            (0.0, 1.0)
        };
        // The placeholder range is already a clean 0..1 (or 1..10 on log); an
        // autoscale margin on synthetic bounds would only make the ticks ugly.
        self.apply_axis_limits_without_margin((x.0, x.1, y.0, y.1))
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

    pub(super) fn effective_main_panel_bounds_from_resolved(
        &self,
        series_list: &[PlotSeries],
        resolved_series: &[ResolvedSeries<'_>],
    ) -> Result<(f64, f64, f64, f64)> {
        if resolved_series.is_empty() {
            return Ok(self.empty_cartesian_bounds());
        }

        if Self::has_mixed_coordinate_series(series_list) {
            // Pair each resolved entry with its originating series: a resolved
            // entry alone cannot see the error bars attached with
            // `with_yerr`/`with_xerr`, so their whiskers would fall outside the
            // range and be clipped against the spine.
            let bounds = self.calculate_data_bounds_for_pairs(
                series_list
                    .iter()
                    .zip(resolved_series)
                    .filter(|(series, _)| Self::is_cartesian_series(series)),
            )?;
            Ok(self.apply_manual_axis_limits(bounds))
        } else {
            self.effective_data_bounds_from_resolved(resolved_series)
        }
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

    /// Draw every colorbar the plot's series ask for.
    ///
    /// A colorbar lives beside the plot area, not inside it, so it must be drawn
    /// after the data clip group closes — inside it, the SVG clipped the whole
    /// colorbar away and the export silently lost its value scale.
    pub(super) fn render_svg_colorbars(
        &self,
        svg: &mut crate::export::SvgRenderer,
        plot_area: tiny_skia::Rect,
    ) -> Result<()> {
        for series in &self.series_mgr.series {
            if let Some(request) = self.series_colorbar_request(&series.series_type) {
                let (x, y, width, height) = self.colorbar_rect(plot_area);
                crate::render::colorbar::draw_colorbar(
                    svg,
                    &request.spec_at(x, y, width, height, self.display.theme.foreground),
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn radar_plot_area(
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> crate::plots::PlotArea {
        // The radar bounds already reserve room for the axis labels (they run to
        // `RADAR_BOUNDS_RADIUS`, outside the `RADAR_LABEL_RADIUS` label ring), so
        // the square just has to be the largest one that fits, centered on both
        // axes. Shrinking it further and pushing it down for "title clearance"
        // double-counts that reservation and leaves the chart small and low.
        let size = plot_area.width().min(plot_area.height()).max(1.0);
        let x_offset = (plot_area.width() - size) * 0.5;
        let y_offset = (plot_area.height() - size) * 0.5;

        crate::plots::PlotArea::new(
            plot_area.x() + x_offset,
            plot_area.y() + y_offset,
            size,
            size,
            x_min,
            x_max,
            y_min,
            y_max,
        )
    }

    pub(super) fn render_series_collection_normal(
        &self,
        series_list: &[PlotSeries],
        resolved_series: &[ResolvedSeries<'_>],
        renderer: &mut SkiaRenderer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        render_scale: RenderScale,
        mode: RenderExecutionMode,
    ) -> Result<()> {
        let inset_rects = self.inset_rects_for_series(series_list, plot_area, render_scale)?;

        for (idx, (series, resolved)) in series_list.iter().zip(resolved_series).enumerate() {
            let (series_area, series_bounds) = if let Some(inset_rect) = inset_rects[idx] {
                (inset_rect, self.inset_bounds_from_resolved(resolved)?)
            } else {
                (plot_area, (x_min, x_max, y_min, y_max))
            };

            self.render_series_normal(
                series,
                resolved,
                renderer,
                series_area,
                series_bounds.0,
                series_bounds.1,
                series_bounds.2,
                series_bounds.3,
                mode,
            )?;
        }

        Ok(())
    }

    pub(super) fn render_series_svg(
        &self,
        svg: &mut crate::export::SvgRenderer,
        series: &PlotSeries,
        resolved: &ResolvedSeries<'_>,
        default_color: Color,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<()> {
        let color = series.color_with_alpha(default_color);
        let render_scale = self.render_scale();
        let line_width = render_scale.points_to_pixels(
            series
                .props
                .line_width
                .value_or(self.display.config.lines.data_width),
        );
        let line_style = series.props.line_style.value_or(LineStyle::Solid);

        match (&series.series_type, resolved) {
            (SeriesType::Line { .. }, ResolvedSeries::Line { x, y }) => {
                // Break the line at every sample the axes cannot represent,
                // using the same run splitter as the raster and parallel
                // backends so all three agree on where a line breaks.
                let marker_size =
                    render_scale.points_to_pixels(series.props.marker_size.value_or(8.0));
                // Same rim the raster path strokes; `draw_marker_styled` scales
                // the point width itself.
                let marker_edge = self.resolved_marker_edge(series, color);

                for run in crate::core::plot::raster_batches::representable_sample_runs(
                    x,
                    y,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                ) {
                    let points: Vec<(f32, f32)> = x[run.clone()]
                        .iter()
                        .zip(y[run].iter())
                        .map(|(&x, &y)| {
                            crate::render::skia::map_data_to_pixels_scaled(
                                x,
                                y,
                                x_min,
                                x_max,
                                y_min,
                                y_max,
                                plot_area,
                                &self.layout.x_scale,
                                &self.layout.y_scale,
                            )
                        })
                        .collect();

                    svg.draw_polyline(&points, color, line_width, line_style.clone());
                    if let Some(marker_style) = series.props.marker_style.cloned() {
                        for &(px, py) in &points {
                            svg.draw_marker_styled(
                                px,
                                py,
                                marker_size,
                                marker_style,
                                color,
                                marker_edge,
                            );
                        }
                    }
                }

                self.render_attached_error_bars_svg(
                    svg, series, x, y, color, line_width, plot_area, x_min, x_max, y_min, y_max,
                )?;
            }
            (SeriesType::Scatter { .. }, ResolvedSeries::Scatter { x, y }) => {
                let marker_style = series.props.marker_style.value_or(MarkerStyle::Circle);
                let marker_size =
                    render_scale.points_to_pixels(series.props.marker_size.value_or(10.0));
                let marker_edge = self.resolved_marker_edge(series, color);
                for (&x, &y) in x.iter().zip(y.iter()) {
                    // A sample the axes cannot place is dropped, not drawn at a
                    // NaN pixel (which lands on the spine and reads as data).
                    let Some((px, py)) = crate::render::skia::try_map_data_to_pixels_scaled(
                        x,
                        y,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        plot_area,
                        &self.layout.x_scale,
                        &self.layout.y_scale,
                    ) else {
                        continue;
                    };
                    svg.draw_marker_styled(px, py, marker_size, marker_style, color, marker_edge);
                }

                self.render_attached_error_bars_svg(
                    svg, series, x, y, color, line_width, plot_area, x_min, x_max, y_min, y_max,
                )?;
            }
            (SeriesType::Bar { config, .. }, ResolvedSeries::Bar { values, .. }) => {
                // Same resolution the raster path uses, so PNG and SVG bars
                // carry the identical edge. The width is in points and
                // `draw_rectangle_styled` scales it, so it is DPI-invariant.
                let edge = config.resolved_edge(&self.display.theme, color);

                for (i, &value) in values.iter().enumerate() {
                    // Shared with the raster backend: SVG used to lay bars out
                    // positionally (ignoring the real x-scale) at a different
                    // width fraction, so a PNG and an SVG of the same chart put
                    // their bars in different places.
                    let (bar_x, bar_y, bar_width, bar_height) =
                        super::series_internal::bar_pixel_rect(
                            i,
                            value,
                            config.width,
                            plot_area,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                            &self.layout.y_scale,
                        );

                    svg.draw_rectangle_styled(
                        bar_x,
                        bar_y,
                        bar_width,
                        bar_height,
                        Some(color),
                        edge,
                    );
                }
            }
            (SeriesType::Heatmap { data }, ResolvedSeries::Other(_)) => {
                let area = super::raster_batches::plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                let alpha = data.config.alpha * series.props.alpha.value_or(1.0);
                for (row, values) in data.values.iter().enumerate() {
                    for (col, &value) in values.iter().enumerate() {
                        if data.should_mask_value(value) {
                            continue;
                        }
                        let (x, y, width, height) = data.cell_screen_rect(&area, row, col);
                        let cell_color = data.get_color(value).with_alpha(alpha);
                        svg.draw_rectangle(x, y, width, height, cell_color, true);
                    }
                }
                // The colorbar sits outside the plot area, so it is drawn by
                // `render_svg_colorbars` after the clip group closes.
            }
            (SeriesType::Computed { data }, ResolvedSeries::Other(_)) => {
                // The exact primitives the raster backend draws. A plot type
                // wired through `SeriesType::Computed` therefore cannot render
                // in PNG and not in SVG — there is no per-type SVG code to
                // forget to write.
                let area = super::raster_batches::plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                // The series alpha is handed over separately rather than baked
                // into `color`, because that is how the raster arm hands it to
                // `primitives` — and the two must agree.
                let style = crate::plots::traits::ComputedStyle {
                    scale: svg.render_scale(),
                    color: series.props.color.value_or(default_color),
                    alpha: series.props.alpha.value_or(1.0),
                    line_width: series.props.line_width.cloned(),
                };
                let primitives = data.primitives(&area, &style);
                crate::plots::traits::draw_primitives_svg(svg, &primitives);
            }
            (SeriesType::Kde { data }, ResolvedSeries::Other(_)) => {
                // Same geometry the raster path draws, from the same helpers, so
                // the two backends cannot break the curve in different places.
                let area = super::raster_batches::plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                let runs = data.projected_runs(&area);
                if data.config.fill {
                    let baseline = area.fill_baseline_y();
                    let fill_color =
                        color.with_alpha((f32::from(color.a) / 255.0) * data.config.fill_alpha);
                    for run in &runs {
                        svg.draw_filled_polygon(
                            &crate::plots::KdeData::fill_polygon(run, baseline),
                            fill_color,
                        );
                    }
                }
                let width = render_scale
                    .points_to_pixels(series.props.line_width.value_or(data.config.line_width));
                for run in &runs {
                    svg.draw_polyline(run, color, width, line_style.clone());
                }
            }
            (SeriesType::Ecdf { data }, ResolvedSeries::Other(_)) => {
                let points: Vec<(f32, f32)> = data
                    .step_vertices
                    .iter()
                    .map(|&(x, y)| {
                        crate::render::skia::map_data_to_pixels_scaled(
                            x,
                            y,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                            plot_area,
                            &self.layout.x_scale,
                            &self.layout.y_scale,
                        )
                    })
                    .collect();
                let width = render_scale
                    .points_to_pixels(series.props.line_width.value_or(data.config.line_width));
                svg.draw_polyline(&points, color, width, line_style);
                if data.config.show_markers {
                    let marker_size = render_scale.points_to_pixels(
                        series.props.marker_size.value_or(data.config.marker_size),
                    );
                    for (&x, &y) in data.x.iter().zip(&data.y) {
                        let (px, py) = crate::render::skia::map_data_to_pixels_scaled(
                            x,
                            y,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                            plot_area,
                            &self.layout.x_scale,
                            &self.layout.y_scale,
                        );
                        svg.draw_marker(px, py, marker_size, MarkerStyle::Circle, color);
                    }
                }
            }
            (SeriesType::Violin { data }, ResolvedSeries::Other(_)) => {
                let half_width = data.config.width / 2.0;
                let (left, right) = crate::plots::distribution::violin_polygon(
                    data,
                    data.config.x_center(),
                    half_width,
                    &data.config,
                );
                let polygon = crate::plots::distribution::close_violin_polygon(&left, &right);
                // Same vertex-rejection rule as the raster path.
                let points: Vec<(f32, f32)> = super::raster_batches::plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                )
                .project_points(polygon.iter().copied());
                let alpha = series.props.alpha.value_or(1.0);
                let fill_base = data
                    .config
                    .fill_color
                    .unwrap_or(series.props.color.value_or(default_color));
                let fill_color = fill_base
                    .with_alpha((f32::from(fill_base.a) / 255.0) * data.config.fill_alpha * alpha);
                svg.draw_filled_polygon(&points, fill_color);
                let edge_color = data
                    .config
                    .line_color
                    .unwrap_or(series.props.color.value_or(default_color));
                let edge_color = edge_color.with_alpha((f32::from(edge_color.a) / 255.0) * alpha);
                let width = render_scale
                    .points_to_pixels(series.props.line_width.value_or(data.config.line_width));
                svg.draw_polygon_outline(&points, edge_color, width);
            }
            (SeriesType::Contour { data }, ResolvedSeries::Other(_)) => {
                let alpha = data.config.alpha * series.props.alpha.value_or(1.0);
                let cmap = crate::render::ColorMap::by_name(&data.config.cmap)
                    .unwrap_or_else(crate::render::ColorMap::viridis);

                // Filled bands, from the same source the raster path fills from.
                // The SVG backend used to draw the lines only, so a filled
                // contour exported to SVG lost every band it was made of.
                let area = super::raster_batches::plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                for (t, polygons) in data.filled_bands() {
                    let fill_color = cmap.sample(t).with_alpha(alpha);
                    for polygon in &polygons {
                        match crate::plots::ContourPlotData::band_shape(&area, polygon) {
                            crate::plots::continuous::contour::BandShape::Rect {
                                x,
                                y,
                                width,
                                height,
                            } => svg.draw_seamless_rectangle(x, y, width, height, fill_color),
                            crate::plots::continuous::contour::BandShape::Polygon(points) => {
                                svg.draw_filled_polygon(&points, fill_color)
                            }
                        }
                    }
                }

                let width = render_scale
                    .points_to_pixels(series.props.line_width.value_or(data.config.line_width));
                let n_levels = data.levels.len();
                for (index, level) in data.lines.iter().enumerate() {
                    // Same decision as the raster path (`contour_line_color`) so PNG,
                    // SVG and PDF agree on contour line colour.
                    let line_color = crate::plots::continuous::contour::contour_line_color(
                        &data.config,
                        &self.display.theme,
                        &cmap,
                        series.props.color.value_or(default_color),
                        index,
                        n_levels,
                    );
                    let line_color =
                        line_color.with_alpha((f32::from(line_color.a) / 255.0) * alpha);
                    for &(x1, y1, x2, y2) in &level.segments {
                        let (sx1, sy1) = crate::render::skia::map_data_to_pixels_scaled(
                            x1,
                            y1,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                            plot_area,
                            &self.layout.x_scale,
                            &self.layout.y_scale,
                        );
                        let (sx2, sy2) = crate::render::skia::map_data_to_pixels_scaled(
                            x2,
                            y2,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                            plot_area,
                            &self.layout.x_scale,
                            &self.layout.y_scale,
                        );
                        svg.draw_line(sx1, sy1, sx2, sy2, line_color, width, line_style.clone());
                    }
                }
            }
            (SeriesType::Pie { data }, ResolvedSeries::Other(_)) => {
                self.render_pie_series_svg(svg, data, series, plot_area)?;
            }
            (SeriesType::Radar { data }, ResolvedSeries::Other(_)) => {
                self.render_radar_series_svg(svg, data, series, plot_area)?;
            }
            (SeriesType::Polar { data }, ResolvedSeries::Other(_)) => {
                self.render_polar_series_svg(
                    svg, data, series, plot_area, x_min, x_max, y_min, y_max, color,
                )?;
            }
            (SeriesType::Boxen { data }, ResolvedSeries::Other(_)) => {
                self.render_boxen_series_svg(
                    svg, data, series, plot_area, x_min, x_max, y_min, y_max, color,
                );
            }
            (SeriesType::Quiver { data }, ResolvedSeries::Other(_)) => {
                self.render_quiver_series_svg(
                    svg, data, series, plot_area, x_min, x_max, y_min, y_max, color,
                );
            }
            (SeriesType::Histogram { .. }, ResolvedSeries::Histogram { data }) => {
                // Same edge resolution as the raster path, so bin boundaries stay
                // visible and PNG/SVG do not diverge.
                let edge = data.resolved_edge(&self.display.theme, color);
                for (index, &count) in data.counts.iter().enumerate() {
                    if count <= 0.0 {
                        continue;
                    }
                    // Shared with the raster backend, and scale-aware on both
                    // axes so the bars agree with the ticks drawn beside them.
                    let (bar_x, bar_y, bar_width, bar_height) =
                        super::series_internal::histogram_bar_pixel_rect(
                            data.bin_edges[index],
                            data.bin_edges[index + 1],
                            count,
                            plot_area,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                            &self.layout.x_scale,
                            &self.layout.y_scale,
                        );
                    svg.draw_rectangle_styled(
                        bar_x,
                        bar_y,
                        bar_width,
                        bar_height,
                        Some(color),
                        edge,
                    );
                }
            }
            (SeriesType::ErrorBars { .. }, ResolvedSeries::ErrorBars { x, y, y_errors }) => self
                .render_error_bars_series_svg(
                    svg,
                    series,
                    x,
                    y,
                    Some(effective_error_values(series.y_errors.as_ref(), y_errors)),
                    series.x_errors.as_ref().map(ErrorValuesRef::from),
                    color,
                    line_width,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                )?,
            (
                SeriesType::ErrorBarsXY { .. },
                ResolvedSeries::ErrorBarsXY {
                    x,
                    y,
                    x_errors,
                    y_errors,
                },
            ) => self.render_error_bars_series_svg(
                svg,
                series,
                x,
                y,
                Some(effective_error_values(series.y_errors.as_ref(), y_errors)),
                Some(effective_error_values(series.x_errors.as_ref(), x_errors)),
                color,
                line_width,
                plot_area,
                x_min,
                x_max,
                y_min,
                y_max,
            )?,
            (SeriesType::BoxPlot { .. }, ResolvedSeries::BoxPlot { data, config }) => {
                self.render_box_plot_series_svg(
                    svg, data, config, color, line_width, line_style, plot_area, x_min, x_max,
                    y_min, y_max,
                )?;
            }
            (_, ResolvedSeries::Other(_)) => {}
            _ => unreachable!("resolved series variant must match its declarative series"),
        }

        Ok(())
    }

    /// Draw the error bars attached to a line or scatter series with
    /// `with_yerr` / `with_xerr`.
    ///
    /// The SVG backend used to skip these entirely while the axis bounds still
    /// reserved room for them, so an SVG of a plot with error bars showed a
    /// stretched, empty axis and no whiskers. It now goes through the same
    /// geometry and the same stroking routine as the raster backend.
    #[allow(clippy::too_many_arguments)]
    fn render_attached_error_bars_svg(
        &self,
        svg: &mut crate::export::SvgRenderer,
        series: &PlotSeries,
        x: &[f64],
        y: &[f64],
        color: Color,
        default_line_width: f32,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<()> {
        if series.y_errors.is_none() && series.x_errors.is_none() {
            return Ok(());
        }
        let style = AttachedErrorBarStyle::resolve(
            series.error_config.as_ref(),
            color,
            default_line_width,
            self.render_scale(),
        );
        stroke_error_bar_series(
            svg,
            x,
            y,
            series.y_errors.as_ref().map(ErrorValuesRef::from),
            series.x_errors.as_ref().map(ErrorValuesRef::from),
            ErrorBarFrame {
                plot_area,
                x_min,
                x_max,
                y_min,
                y_max,
                x_scale: &self.layout.x_scale,
                y_scale: &self.layout.y_scale,
            },
            style,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_error_bars_series_svg(
        &self,
        svg: &mut crate::export::SvgRenderer,
        series: &PlotSeries,
        x: &[f64],
        y: &[f64],
        y_errors: Option<ErrorValuesRef<'_>>,
        x_errors: Option<ErrorValuesRef<'_>>,
        color: Color,
        default_line_width: f32,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<()> {
        let render_scale = self.render_scale();
        let style = AttachedErrorBarStyle::resolve(
            series.error_config.as_ref(),
            color,
            default_line_width,
            render_scale,
        );
        let marker_style = series.props.marker_style.value_or(MarkerStyle::Circle);
        let marker_size = render_scale.points_to_pixels(series.props.marker_size.value_or(8.0));
        let marker_edge = self.resolved_marker_edge(series, color);
        let frame = ErrorBarFrame {
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
            x_scale: &self.layout.x_scale,
            y_scale: &self.layout.y_scale,
        };

        // The whiskers come from the same projection the raster backend uses,
        // so an `error_bars` series cannot land in two different places
        // depending on the output format.
        for bars in error_bar_pixels_for_series(x, y, y_errors, x_errors, frame) {
            svg.draw_marker_styled(
                bars.x,
                bars.y,
                marker_size,
                marker_style,
                color,
                marker_edge,
            );
            draw_error_bars(
                svg,
                &bars,
                plot_area,
                style.color,
                style.line_width,
                style.half_cap,
            )?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn render_box_plot_series_svg(
        &self,
        svg: &mut crate::export::SvgRenderer,
        data: &[f64],
        config: &BoxPlotConfig,
        color: Color,
        line_width: f32,
        line_style: LineStyle,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<()> {
        let box_data =
            crate::plots::boxplot::calculate_box_plot(&data, config).map_err(|error| {
                PlottingError::RenderError(format!("Box plot calculation failed: {error}"))
            })?;
        // One shared projection with the raster and parallel backends: the SVG
        // path used to re-derive these five quantiles linearly, so
        // `.boxplot(&d).yscale(Log)` drew the box somewhere the ticks did not
        // agree with.
        let px = super::series_internal::BoxPlotPixels::new(
            &box_data,
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
            &self.layout.y_scale,
        );
        let x_center = px.x_center;
        let q1 = px.q1_y;
        let median = px.median_y;
        let q3 = px.q3_y;
        let lower_whisker = px.lower_whisker_y;
        let upper_whisker = px.upper_whisker_y;
        let left = px.box_left;
        let right = px.box_right;
        let cap_width = px.cap_half_width;
        let edge_color = box_data.edge_color.unwrap_or(color);
        let whisker_width = box_data
            .whisker_width
            .map(|w| self.render_scale().points_to_pixels(w))
            .unwrap_or(line_width);
        let median_width = box_data
            .median_width
            .map(|w| self.render_scale().points_to_pixels(w))
            .unwrap_or(line_width * 1.5);

        svg.draw_rectangle_styled(
            left,
            q1.min(q3),
            right - left,
            (q1 - q3).abs(),
            Some(color.with_alpha(box_data.fill_alpha)),
            Some((edge_color, box_data.edge_width)),
        );
        svg.draw_line(
            left,
            median,
            right,
            median,
            edge_color,
            median_width,
            line_style.clone(),
        );
        svg.draw_line(
            x_center,
            q1,
            x_center,
            lower_whisker,
            edge_color,
            whisker_width,
            line_style.clone(),
        );
        svg.draw_line(
            x_center,
            q3,
            x_center,
            upper_whisker,
            edge_color,
            whisker_width,
            line_style.clone(),
        );
        svg.draw_line(
            x_center - cap_width,
            lower_whisker,
            x_center + cap_width,
            lower_whisker,
            edge_color,
            whisker_width,
            line_style.clone(),
        );
        svg.draw_line(
            x_center - cap_width,
            upper_whisker,
            x_center + cap_width,
            upper_whisker,
            edge_color,
            whisker_width,
            line_style,
        );
        if box_data.show_outliers {
            let outlier_size = self.render_scale().points_to_pixels(box_data.flier_size);
            for &outlier in &box_data.outliers {
                svg.draw_marker(
                    x_center,
                    super::series_internal::box_plot_value_y(
                        outlier,
                        plot_area,
                        y_min,
                        y_max,
                        &self.layout.y_scale,
                    ),
                    outlier_size,
                    MarkerStyle::Circle,
                    color,
                );
            }
        }
        Ok(())
    }

    fn render_boxen_series_svg(
        &self,
        svg: &mut crate::export::SvgRenderer,
        data: &crate::plots::BoxenData,
        series: &PlotSeries,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        default_color: Color,
    ) {
        if data.boxes.is_empty() {
            return;
        }

        let center = data.config.x_center();
        let alpha = series.props.alpha.value_or(1.0);
        let base_color = data.config.color.map_or(default_color, |color| {
            color.with_alpha((f32::from(color.a) / 255.0) * alpha)
        });
        let edge_width = self
            .render_scale()
            .points_to_pixels(series.props.line_width.value_or(data.config.line_width));

        for (index, boxen_box) in data.boxes.iter().enumerate() {
            let saturation_factor = crate::plots::distribution::boxen::boxen_saturation_factor(
                index,
                data.boxes.len(),
                data.config.saturation,
            );
            let fill_color =
                crate::plots::distribution::boxen::adjust_saturation(base_color, saturation_factor);
            let points: Vec<(f32, f32)> =
                crate::plots::distribution::boxen_rect(boxen_box, center, data.config.orient)
                    .iter()
                    .map(|&(x, y)| {
                        crate::render::skia::map_data_to_pixels_scaled(
                            x,
                            y,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                            plot_area,
                            &self.layout.x_scale,
                            &self.layout.y_scale,
                        )
                    })
                    .collect();

            svg.draw_filled_polygon(&points, fill_color);
            if edge_width > 0.0 {
                svg.draw_polygon_outline(&points, base_color, edge_width);
            }
        }

        let median_half = data.median_half_width();
        let median_width = self.render_scale().points_to_pixels(2.0);
        match data.config.orient {
            crate::plots::distribution::BoxenOrientation::Vertical => {
                let (x1, y) = crate::render::skia::map_data_to_pixels_scaled(
                    center - median_half,
                    data.median,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                let (x2, _) = crate::render::skia::map_data_to_pixels_scaled(
                    center + median_half,
                    data.median,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                svg.draw_line(
                    x1,
                    y,
                    x2,
                    y,
                    Color::from_rgb(255, 255, 255),
                    median_width,
                    LineStyle::Solid,
                );
            }
            crate::plots::distribution::BoxenOrientation::Horizontal => {
                let (x, y1) = crate::render::skia::map_data_to_pixels_scaled(
                    data.median,
                    center - median_half,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                let (_, y2) = crate::render::skia::map_data_to_pixels_scaled(
                    data.median,
                    center + median_half,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                svg.draw_line(
                    x,
                    y1,
                    x,
                    y2,
                    Color::from_rgb(255, 255, 255),
                    median_width,
                    LineStyle::Solid,
                );
            }
        }

        if data.config.show_outliers {
            let marker_size = self
                .render_scale()
                .points_to_pixels(series.props.marker_size.value_or(data.config.outlier_size));
            for &outlier in &data.outliers {
                let (px, py) = match data.config.orient {
                    crate::plots::distribution::BoxenOrientation::Vertical => {
                        crate::render::skia::map_data_to_pixels_scaled(
                            center,
                            outlier,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                            plot_area,
                            &self.layout.x_scale,
                            &self.layout.y_scale,
                        )
                    }
                    crate::plots::distribution::BoxenOrientation::Horizontal => {
                        crate::render::skia::map_data_to_pixels_scaled(
                            outlier,
                            center,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                            plot_area,
                            &self.layout.x_scale,
                            &self.layout.y_scale,
                        )
                    }
                };
                svg.draw_marker(px, py, marker_size, MarkerStyle::Circle, base_color);
            }
        }
    }

    fn render_quiver_series_svg(
        &self,
        svg: &mut crate::export::SvgRenderer,
        data: &crate::plots::QuiverPlotData,
        series: &PlotSeries,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        default_color: Color,
    ) {
        if data.arrows.is_empty() {
            return;
        }

        let alpha = series.props.alpha.value_or(1.0);
        let base_color = data.config.color.map_or(default_color, |color| {
            color.with_alpha((f32::from(color.a) / 255.0) * alpha)
        });
        let cmap = data.config.color_by_magnitude.then(|| {
            crate::render::ColorMap::by_name(&data.config.cmap)
                .unwrap_or_else(crate::render::ColorMap::viridis)
        });
        let (min_mag, max_mag) = data.magnitude_range;
        let mag_range = if (max_mag - min_mag).abs() < 1e-10 {
            1.0
        } else {
            max_mag - min_mag
        };
        let arrow_width = self
            .render_scale()
            .points_to_pixels(series.props.line_width.value_or(data.config.width));

        for arrow in &data.arrows {
            let arrow_color = cmap
                .as_ref()
                .map(|colormap| {
                    colormap
                        .sample((arrow.magnitude - min_mag) / mag_range)
                        .with_alpha(alpha)
                })
                .unwrap_or(base_color);
            let (sx1, sy1) = crate::render::skia::map_data_to_pixels_scaled(
                arrow.start.0,
                arrow.start.1,
                x_min,
                x_max,
                y_min,
                y_max,
                plot_area,
                &self.layout.x_scale,
                &self.layout.y_scale,
            );
            let (sx2, sy2) = crate::render::skia::map_data_to_pixels_scaled(
                arrow.end.0,
                arrow.end.1,
                x_min,
                x_max,
                y_min,
                y_max,
                plot_area,
                &self.layout.x_scale,
                &self.layout.y_scale,
            );
            svg.draw_line(
                sx1,
                sy1,
                sx2,
                sy2,
                arrow_color,
                arrow_width,
                LineStyle::Solid,
            );

            let head: Vec<(f32, f32)> = arrow
                .head
                .iter()
                .map(|&(x, y)| {
                    crate::render::skia::map_data_to_pixels_scaled(
                        x,
                        y,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        plot_area,
                        &self.layout.x_scale,
                        &self.layout.y_scale,
                    )
                })
                .collect();
            svg.draw_filled_polygon(&head, arrow_color);
        }
    }

    pub(super) fn render_pie_series_svg(
        &self,
        svg: &mut crate::export::SvgRenderer,
        data: &crate::plots::composition::pie::PieData,
        series: &PlotSeries,
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
        let alpha = series.props.alpha.value_or(1.0);
        let colors = if let Some(ref colors) = data.config.colors {
            colors.clone()
        } else {
            let palette = self.display.theme.color_palette.clone();
            (0..screen_data.wedges.len())
                .map(|i| palette[i % palette.len()])
                .collect()
        }
        .into_iter()
        .map(|color| color.with_alpha((f32::from(color.a) / 255.0) * alpha))
        .collect::<Vec<_>>();
        let segments = 64;
        let render_scale = svg.render_scale();
        let shadow_offset = render_scale.points_to_pixels(data.config.shadow as f32) as f64;
        let label_font_size = render_scale.points_to_pixels(data.config.label_font_size);

        if data.config.shadow > 0.0 {
            let shadow_color = Color::from_rgb(100, 100, 100).with_alpha(0.3 * alpha);
            for wedge in &screen_data.wedges {
                let polygon: Vec<(f32, f32)> = wedge
                    .as_polygon(segments)
                    .iter()
                    .map(|(x, y)| ((*x + shadow_offset) as f32, (*y + shadow_offset) as f32))
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
                let edge_color = edge_color.with_alpha((f32::from(edge_color.a) / 255.0) * alpha);
                let scaled_edge_width = svg
                    .render_scale()
                    .points_to_pixels(series.props.line_width.value_or(data.config.edge_width));
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
                        Some(crate::plots::composition::pie::format_percentage(
                            screen_data.percentages[idx],
                        ))
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
                    let label_r = crate::plots::composition::pie::label_radius(
                        radius as f64,
                        data.config.inner_radius,
                        data.config.label_distance,
                    );
                    let mid_angle = (wedge.start_angle + wedge.end_angle) / 2.0;
                    let label_x = cx as f64 + label_r * mid_angle.cos();
                    let label_y = cy as f64 + label_r * mid_angle.sin();
                    let text_color = data.config.text_color.unwrap_or_else(|| {
                        crate::plots::composition::pie::label_color_on(
                            colors[idx % colors.len()],
                            self.display.theme.background,
                        )
                    });
                    svg.draw_text_centered(
                        &label,
                        label_x as f32,
                        label_y as f32,
                        label_font_size,
                        text_color,
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
        plot_series: &PlotSeries,
        plot_area: tiny_skia::Rect,
    ) -> Result<()> {
        if data.series.is_empty() {
            return Ok(());
        }

        // Keep the SVG backend on the same bounds the raster bounds arm derives
        // from the radar label radius.
        let radius = crate::plots::polar::radar::RADAR_BOUNDS_RADIUS;
        let area = Self::radar_plot_area(plot_area, -radius, radius, -radius, radius);
        let render_scale = svg.render_scale();
        let label_font_size = render_scale.points_to_pixels(data.config.label_font_size);

        if data.config.show_grid && self.layout.grid_style.draws_major() {
            let grid_color = self.layout.grid_style.effective_color();
            let grid_line_width = render_scale
                .points_to_pixels(self.layout.grid_style.line_width)
                .max(crate::core::style_utils::defaults::MIN_GRID_LINE_WIDTH_PX);
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
                        self.layout.grid_style.line_style.clone(),
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
                    self.layout.grid_style.line_style.clone(),
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
                    label_font_size,
                    self.display.theme.foreground,
                )?;
            }
        }

        // Plot-level fallback; a per-series override wins over it (see
        // `RadarConfig::series_line_width_or`), matching the raster path.
        let base_line_width = plot_series
            .props
            .line_width
            .value_or(data.config.line_width);
        let marker_size = plot_series
            .props
            .marker_size
            .value_or(data.config.marker_size);
        let scaled_marker_size = render_scale.points_to_pixels(marker_size);
        let alpha = plot_series.props.alpha.value_or(1.0);
        for (series_idx, series_data) in data.series.iter().enumerate() {
            let series_color = plot_series
                .resolved_radar_colors
                .as_ref()
                .and_then(|colors| colors.get(series_idx).copied())
                .or_else(|| {
                    data.config
                        .colors
                        .as_ref()
                        .and_then(|colors| colors.get(series_idx).copied())
                        .filter(|color| *color != Color::TRANSPARENT)
                })
                .unwrap_or_else(|| self.display.theme.get_color(series_idx));
            let series_alpha = (f32::from(series_color.a) / 255.0) * alpha;
            let stroke_color = series_color.with_alpha(series_alpha);
            let scaled_line_width = render_scale.points_to_pixels(
                data.config
                    .series_line_width_or(series_idx, base_line_width),
            );

            if data.config.fill && !series_data.polygon.is_empty() {
                let polygon: Vec<(f32, f32)> = series_data
                    .polygon
                    .iter()
                    .map(|(x, y)| area.data_to_screen(*x, *y))
                    .collect();
                svg.draw_filled_polygon(
                    &polygon,
                    series_color
                        .with_alpha(data.config.series_fill_alpha(series_idx) * series_alpha),
                );
            }

            if series_data.polygon.len() > 1 {
                let polygon: Vec<(f32, f32)> = series_data
                    .polygon
                    .iter()
                    .map(|(x, y)| area.data_to_screen(*x, *y))
                    .collect();
                svg.draw_polygon_outline(&polygon, stroke_color, scaled_line_width);
            }

            if marker_size > 0.0 {
                for (x, y) in &series_data.markers {
                    let (sx, sy) = area.data_to_screen(*x, *y);
                    svg.draw_marker(
                        sx,
                        sy,
                        scaled_marker_size,
                        MarkerStyle::Circle,
                        stroke_color,
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
        series: &PlotSeries,
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
        let alpha = series.props.alpha.value_or(1.0);
        let line_color = data.config.color.map_or(default_color, |color| {
            color.with_alpha((f32::from(color.a) / 255.0) * alpha)
        });
        let render_scale = svg.render_scale();
        let label_font_size = render_scale.points_to_pixels(data.config.label_font_size);

        // The grid the raster backend draws in `PolarPlotData::render_styled_with_grid`,
        // from the same precomputed rings and spokes and the same `GridStyle`.
        // Without this the two backends disagreed about whether a polar plot has
        // a grid at all.
        if self.layout.grid_style.draws_major() {
            let grid_style = &self.layout.grid_style;
            let grid_color = grid_style.effective_color();
            let grid_line_width = render_scale.points_to_pixels(grid_style.line_width);
            for ring in &data.grid_rings {
                if ring.len() < 2 {
                    continue;
                }
                let screen_ring: Vec<(f32, f32)> = ring
                    .iter()
                    .map(|(x, y)| area.data_to_screen(*x, *y))
                    .collect();
                svg.draw_polyline(
                    &screen_ring,
                    grid_color,
                    grid_line_width,
                    grid_style.line_style.clone(),
                );
            }

            for &((x1, y1), (x2, y2)) in &data.grid_spokes {
                let (sx1, sy1) = area.data_to_screen(x1, y1);
                let (sx2, sy2) = area.data_to_screen(x2, y2);
                svg.draw_line(
                    sx1,
                    sy1,
                    sx2,
                    sy2,
                    grid_color,
                    grid_line_width,
                    grid_style.line_style.clone(),
                );
            }
        }

        if data.config.fill && !data.fill_polygon.is_empty() {
            let polygon: Vec<(f32, f32)> = data
                .fill_polygon
                .iter()
                .map(|(x, y)| area.data_to_screen(*x, *y))
                .collect();
            svg.draw_filled_polygon(
                &polygon,
                line_color.with_alpha((f32::from(line_color.a) / 255.0) * data.config.fill_alpha),
            );
        }

        if data.points.len() > 1 {
            let mut points: Vec<(f32, f32)> = data
                .points
                .iter()
                .map(|point| area.data_to_screen(point.x, point.y))
                .collect();
            // Endpoint-exclusive sampling of a full turn stops one step short of
            // the start; without this segment the outline gapes at the seam.
            if let Some((_, (x, y))) = data.closing_segment() {
                points.push(area.data_to_screen(x, y));
            }
            let scaled_line_width = render_scale
                .points_to_pixels(series.props.line_width.value_or(data.config.line_width));
            svg.draw_polyline(&points, line_color, scaled_line_width, LineStyle::Solid);
        }

        let marker_size = series.props.marker_size.value_or(data.config.marker_size);
        if marker_size > 0.0 {
            let scaled_marker_size = render_scale.points_to_pixels(marker_size);
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
                label_font_size,
                self.display.theme.foreground,
            )?;
        }

        for label in &data.r_labels {
            let (sx, sy) = area.data_to_screen(label.x, label.y);
            svg.draw_text_centered(
                &label.text,
                sx,
                sy,
                label_font_size,
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

    /// Apply the matplotlib-style autoscale margin to auto-scaled axes.
    ///
    /// Without this, data sits exactly on the spines. The margin is skipped for
    /// axes with explicit limits, for plots whose bounds are set by construction
    /// (polar/radar/pie already reserve their own label room), for image-like
    /// series that fill their axes, and on the sticky zero baseline of
    /// bar/histogram charts.
    pub(super) fn apply_autoscale_margins(
        &self,
        bounds: (f64, f64, f64, f64),
    ) -> (f64, f64, f64, f64) {
        // What each plot type pins is declared once, in `sticky_edges_of`.
        // Radar/polar/pie bounds are deliberately padded already and heatmaps
        // are sticky on all four edges (matplotlib `imshow`); neither takes a
        // margin band.
        let sticky = self.sticky_edges();
        if sticky.by_construction || sticky.all_edges {
            return bounds;
        }

        let (x_min, x_max, y_min, y_max) = bounds;
        let config = &self.display.config;

        let (x_min, x_max) = if self.layout.x_limits.is_some() {
            (x_min, x_max)
        } else {
            padded_axis_range(
                x_min,
                x_max,
                config.x_margin,
                &self.layout.x_scale,
                true,
                true,
            )
        };

        let (y_min, y_max) = if self.layout.y_limits.is_some() {
            (y_min, y_max)
        } else {
            let sticky_zero = sticky.y_zero_baseline;
            padded_axis_range(
                y_min,
                y_max,
                config.y_margin,
                &self.layout.y_scale,
                !(sticky_zero && y_min == 0.0),
                !(sticky_zero && y_max == 0.0),
            )
        };

        (x_min, x_max, y_min, y_max)
    }

    /// Override auto-computed bounds with any explicit axis limits.
    ///
    /// Explicit limits are used verbatim — no autoscale margin is added on top.
    pub(super) fn apply_axis_limits_without_margin(
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

        (x_min, x_max) =
            crate::axes::scale::expand_degenerate_range(x_min, x_max, &self.layout.x_scale);
        (y_min, y_max) =
            crate::axes::scale::expand_degenerate_range(y_min, y_max, &self.layout.y_scale);

        (x_min, x_max, y_min, y_max)
    }

    /// Finalize scanned data bounds into an axis range.
    ///
    /// Order is deliberate: degenerate (`min == max`) ranges are expanded first
    /// so the autoscale margin has a real span to work from, then the margin is
    /// applied, then explicit limits replace whatever was computed.
    pub(super) fn apply_manual_axis_limits(
        &self,
        bounds: (f64, f64, f64, f64),
    ) -> (f64, f64, f64, f64) {
        let (x_min, x_max) =
            crate::axes::scale::expand_degenerate_range(bounds.0, bounds.1, &self.layout.x_scale);
        let (y_min, y_max) =
            crate::axes::scale::expand_degenerate_range(bounds.2, bounds.3, &self.layout.y_scale);

        let margined = self.apply_autoscale_margins((x_min, x_max, y_min, y_max));
        self.apply_axis_limits_without_margin(margined)
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

        // No second annotation pass: `calculate_data_bounds_for_series` ends in
        // `finish_bounds`, which is the one place annotations are folded in.
        self.calculate_data_bounds_for_series(series_list)
            .map(|bounds| self.apply_manual_axis_limits(bounds))
    }

    /// The axis range a resolved frame renders into.
    ///
    /// Delegates to [`Plot::effective_frame_bounds`], the single bounds routine,
    /// which also sees the error bars attached with `with_yerr`/`with_xerr` —
    /// without it those whiskers are clipped against the spine. Annotations are
    /// already folded in by `finish_bounds`, so there is no second expansion
    /// pass here.
    pub(super) fn effective_data_bounds_from_resolved(
        &self,
        resolved_series: &[ResolvedSeries<'_>],
    ) -> Result<(f64, f64, f64, f64)> {
        self.effective_frame_bounds(resolved_series)
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

        // Already padded by `fraction`; skip the generic autoscale margin.
        self.apply_axis_limits_without_margin((x_min, x_max, y_min, y_max))
    }

    /// Helper to render attached error bars on Line/Scatter series
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_attached_error_bars(
        renderer: &mut SkiaRenderer,
        x_data: &[f64],
        y_data: &[f64],
        y_errors: Option<ErrorValuesRef<'_>>,
        x_errors: Option<ErrorValuesRef<'_>>,
        error_config: Option<&ErrorBarConfig>,
        series_color: Color,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        plot_area: tiny_skia::Rect,
        default_line_width: f32,
        render_scale: RenderScale,
        x_scale: &crate::axes::AxisScale,
        y_scale: &crate::axes::AxisScale,
    ) -> Result<()> {
        let style = AttachedErrorBarStyle::resolve(
            error_config,
            series_color,
            default_line_width,
            render_scale,
        );
        let frame = ErrorBarFrame {
            plot_area,
            x_min,
            x_max,
            y_min,
            y_max,
            x_scale,
            y_scale,
        };
        stroke_error_bar_series(renderer, x_data, y_data, y_errors, x_errors, frame, style)
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod autoscale_margin_tests {
    use super::*;

    fn line_plot() -> Plot {
        Plot::new().line(&[0.0, 10.0], &[0.0, 100.0]).end_series()
    }

    #[test]
    fn autoscale_margin_pads_five_percent_per_side() {
        let plot = line_plot();
        let (x_min, x_max, y_min, y_max) = plot
            .effective_data_bounds()
            .expect("bounds should resolve for a simple line plot");

        // matplotlib default: 5% of the raw span added to each side.
        assert!((x_min + 0.5).abs() < 1e-9, "x_min = {x_min}");
        assert!((x_max - 10.5).abs() < 1e-9, "x_max = {x_max}");
        assert!((y_min + 5.0).abs() < 1e-9, "y_min = {y_min}");
        assert!((y_max - 105.0).abs() < 1e-9, "y_max = {y_max}");
    }

    #[test]
    fn zero_margin_reproduces_edge_to_edge_bounds() {
        let config = PlotConfig::builder().data_margins(0.0, 0.0).build();
        let plot = Plot::new()
            .plot_config(config)
            .line(&[0.0, 10.0], &[0.0, 100.0])
            .end_series();

        let bounds = plot
            .effective_data_bounds()
            .expect("bounds should resolve with margins disabled");

        assert!((bounds.0 - 0.0).abs() < 1e-9);
        assert!((bounds.1 - 10.0).abs() < 1e-9);
        assert!((bounds.2 - 0.0).abs() < 1e-9);
        assert!((bounds.3 - 100.0).abs() < 1e-9);
    }

    #[test]
    fn explicit_limits_are_never_padded() {
        let plot = Plot::new()
            .xlim(0.0, 10.0)
            .ylim(0.0, 100.0)
            .line(&[0.0, 10.0], &[0.0, 100.0])
            .end_series();

        let bounds = plot
            .effective_data_bounds()
            .expect("bounds should resolve with explicit limits");

        assert!((bounds.0 - 0.0).abs() < 1e-9);
        assert!((bounds.1 - 10.0).abs() < 1e-9);
        assert!((bounds.2 - 0.0).abs() < 1e-9);
        assert!((bounds.3 - 100.0).abs() < 1e-9);
    }

    #[test]
    fn explicit_limits_on_one_axis_still_pad_the_other() {
        let plot = Plot::new()
            .ylim(0.0, 100.0)
            .line(&[0.0, 10.0], &[0.0, 100.0])
            .end_series();

        let (x_min, x_max, y_min, y_max) = plot
            .effective_data_bounds()
            .expect("bounds should resolve with one explicit axis");

        assert!((x_min + 0.5).abs() < 1e-9);
        assert!((x_max - 10.5).abs() < 1e-9);
        assert!((y_min - 0.0).abs() < 1e-9);
        assert!((y_max - 100.0).abs() < 1e-9);
    }

    #[test]
    fn bar_chart_keeps_its_zero_baseline_sticky() {
        let plot = Plot::new()
            .bar(&["a", "b", "c"], &[1.0, 2.0, 3.0])
            .end_series();

        let (_, _, y_min, y_max) = plot
            .effective_data_bounds()
            .expect("bar bounds should resolve");

        // Bars must still sit exactly on y = 0 (matplotlib sticky_edges)...
        assert!((y_min - 0.0).abs() < 1e-9, "y_min = {y_min}");
        // ...while the far side still gets the usual headroom.
        assert!((y_max - 3.15).abs() < 1e-9, "y_max = {y_max}");
    }

    #[test]
    fn histogram_keeps_its_zero_baseline_sticky() {
        let plot = Plot::new()
            .histogram(&[1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0])
            .end_series();

        let (_, _, y_min, y_max) = plot
            .effective_data_bounds()
            .expect("histogram bounds should resolve");

        assert!((y_min - 0.0).abs() < 1e-9, "y_min = {y_min}");
        assert!(y_max > 0.0);
    }

    #[test]
    fn log_axis_margin_is_applied_in_log_space() {
        let plot = Plot::new()
            .yscale(crate::axes::AxisScale::Log)
            .line(&[0.0, 1.0], &[1.0, 100.0])
            .end_series();

        let (_, _, y_min, y_max) = plot
            .effective_data_bounds()
            .expect("log bounds should resolve");

        // Two decades padded by 5% of the log span on each side.
        assert!(y_min > 0.0, "log lower bound must stay positive: {y_min}");
        assert!((y_min.log10() + 0.1).abs() < 1e-9, "y_min = {y_min}");
        assert!((y_max.log10() - 2.1).abs() < 1e-9, "y_max = {y_max}");
    }

    #[test]
    fn heatmap_fills_its_axes_without_a_margin_band() {
        let values = vec![vec![0.0, 1.0], vec![1.0, 0.0]];
        let plot = Plot::new().heatmap(&values).end_series();

        let raw = plot
            .calculate_data_bounds()
            .expect("heatmap bounds should resolve");
        let effective = plot
            .effective_data_bounds()
            .expect("heatmap bounds should resolve");

        // Image-like series are sticky on all four edges (matplotlib `imshow`),
        // so the cells reach the spines with no background band around them.
        assert_eq!(raw, effective);
    }

    #[test]
    fn contour_fills_its_axes_without_a_margin_band() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 2.0];
        let z = vec![0.0, 1.0, 2.0, 1.0, 2.0, 3.0, 2.0, 3.0, 4.0];
        let plot: Plot = Plot::new().contour(&x, &y, &z).filled(true).into();

        let raw = plot
            .calculate_data_bounds()
            .expect("contour bounds should resolve");
        let effective = plot
            .effective_data_bounds()
            .expect("contour bounds should resolve");

        // matplotlib's `ContourSet` calls `autoscale_view(tight=True)`, so the
        // fill reaches the spines instead of floating in a white gutter.
        assert_eq!(raw, effective);
        assert!((effective.0 - 0.0).abs() < 1e-9, "x_min = {}", effective.0);
        assert!((effective.1 - 2.0).abs() < 1e-9, "x_max = {}", effective.1);
        assert!((effective.2 - 0.0).abs() < 1e-9, "y_min = {}", effective.2);
        assert!((effective.3 - 2.0).abs() < 1e-9, "y_max = {}", effective.3);
    }

    #[test]
    fn radar_bounds_reserve_room_for_axis_labels() {
        let plot: Plot = Plot::new()
            .radar(&["a", "b", "c", "d", "e"])
            .add_series("s1", &[1.0, 2.0, 3.0, 4.0, 5.0])
            .into();

        let radius = crate::plots::polar::radar::RADAR_BOUNDS_RADIUS;
        let (x_min, x_max, y_min, y_max) = plot
            .effective_data_bounds()
            .expect("radar bounds should resolve");

        // Set by construction, and NOT additionally padded by the 5% margin.
        assert!((x_min + radius).abs() < 1e-9, "x_min = {x_min}");
        assert!((x_max - radius).abs() < 1e-9, "x_max = {x_max}");
        assert!((y_min + radius).abs() < 1e-9, "y_min = {y_min}");
        assert!((y_max - radius).abs() < 1e-9, "y_max = {y_max}");
    }

    #[test]
    fn padded_axis_range_skips_symlog() {
        let scale = crate::axes::AxisScale::SymLog { linthresh: 1.0 };
        let (min, max) = padded_axis_range(-10.0, 10.0, 0.05, &scale, true, true);
        assert!((min + 10.0).abs() < 1e-9);
        assert!((max - 10.0).abs() < 1e-9);
    }

    #[test]
    fn padded_axis_range_skips_non_positive_log_bounds() {
        let scale = crate::axes::AxisScale::Log;
        let (min, max) = padded_axis_range(0.0, 10.0, 0.05, &scale, true, true);
        assert!((min - 0.0).abs() < 1e-9);
        assert!((max - 10.0).abs() < 1e-9);
    }
}
