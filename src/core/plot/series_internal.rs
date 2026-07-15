use super::*;
use crate::core::plot::raster_batches::{
    RectGridBatch, SeriesRasterPlan, clip_rect_from_plot_area, plot_area_from_rect,
    project_xy_points,
};
use crate::core::plot::raster_fast_path::{
    canonicalize_line_points_exact, reduce_line_points_for_raster, should_reduce_line_series,
};

impl Plot {
    /// Add a new line to existing plot (for incremental updates)
    pub fn add_line<X, Y>(&mut self, x_data: &X, y_data: &Y) -> Result<()>
    where
        X: NumericData1D,
        Y: NumericData1D,
    {
        let x_vec = collect_numeric_data_1d(x_data, self.null_policy)?;
        let y_vec = collect_numeric_data_1d(y_data, self.null_policy)?;

        if x_vec.len() != y_vec.len() {
            return Err(PlottingError::DataLengthMismatch {
                x_len: x_vec.len(),
                y_len: y_vec.len(),
                series_index: None,
            });
        }

        if x_vec.is_empty() {
            return Err(PlottingError::EmptyDataSet);
        }

        let series = PlotSeries {
            series_type: SeriesType::Line {
                x_data: PlotData::Static(x_vec),
                y_data: PlotData::Static(y_vec),
            },
            streaming_source: None,
            label: None,
            color: Some(
                self.display
                    .theme
                    .get_color(self.series_mgr.auto_color_index),
            ),
            color_source: None,
            line_width: None,
            line_width_source: None,
            line_style: None,
            line_style_source: None,
            marker_style: None,
            marker_style_source: None,
            marker_size: None,
            marker_size_source: None,
            alpha: None,
            alpha_source: None,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        self.series_mgr.series.push(series);
        self.series_mgr.auto_color_index += 1;

        Ok(())
    }

    /// Internal method to add a KDE series (used by PlotBuilder)
    ///
    /// This method is called by the PlotBuilder when finalizing a KDE series.
    pub(crate) fn add_kde_series(
        mut self,
        kde_data: crate::plots::KdeData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        let series = PlotSeries {
            series_type: SeriesType::Kde { data: kde_data },
            streaming_source: None,
            label: style.label,
            color: style.color.or_else(|| {
                Some(
                    self.display
                        .theme
                        .get_color(self.series_mgr.auto_color_index),
                )
            }),
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        self.series_mgr.series.push(series);
        self.series_mgr.auto_color_index += 1;
        self
    }

    /// Internal method to add an ECDF series
    pub(crate) fn add_ecdf_series(
        mut self,
        ecdf_data: crate::plots::EcdfData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        let series = PlotSeries {
            series_type: SeriesType::Ecdf { data: ecdf_data },
            streaming_source: None,
            label: style.label,
            color: style.color.or_else(|| {
                Some(
                    self.display
                        .theme
                        .get_color(self.series_mgr.auto_color_index),
                )
            }),
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        self.series_mgr.series.push(series);
        self.series_mgr.auto_color_index += 1;
        self
    }

    /// Internal method to add a Contour series
    pub(crate) fn add_contour_series(
        mut self,
        contour_data: crate::plots::continuous::contour::ContourPlotData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        let series = PlotSeries {
            series_type: SeriesType::Contour { data: contour_data },
            streaming_source: None,
            label: style.label,
            color: style.color.or_else(|| {
                Some(
                    self.display
                        .theme
                        .get_color(self.series_mgr.auto_color_index),
                )
            }),
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        self.series_mgr.series.push(series);
        self.series_mgr.auto_color_index += 1;
        self
    }

    /// Internal method to add a Pie series
    pub(crate) fn add_pie_series(
        mut self,
        pie_data: crate::plots::composition::pie::PieData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        let series = PlotSeries {
            series_type: SeriesType::Pie { data: pie_data },
            streaming_source: None,
            label: style.label,
            color: style.color.or_else(|| {
                Some(
                    self.display
                        .theme
                        .get_color(self.series_mgr.auto_color_index),
                )
            }),
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: Some(style.inset_layout.unwrap_or_default().normalized()),
            group_id: None,
        };

        self.series_mgr.series.push(series);
        self.series_mgr.auto_color_index += 1;
        self
    }

    /// Internal method to add a Radar series
    pub(crate) fn add_radar_series(
        mut self,
        radar_data: crate::plots::polar::radar::RadarPlotData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        let series = PlotSeries {
            series_type: SeriesType::Radar { data: radar_data },
            streaming_source: None,
            label: style.label,
            color: style.color.or_else(|| {
                Some(
                    self.display
                        .theme
                        .get_color(self.series_mgr.auto_color_index),
                )
            }),
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: Some(style.inset_layout.unwrap_or_default().normalized()),
            group_id: None,
        };

        self.series_mgr.series.push(series);
        self.series_mgr.auto_color_index += 1;
        self
    }

    /// Internal method to add a Violin series
    pub(crate) fn add_violin_series(
        mut self,
        violin_data: crate::plots::ViolinData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        let series = PlotSeries {
            series_type: SeriesType::Violin { data: violin_data },
            streaming_source: None,
            label: style.label,
            color: style.color.or_else(|| {
                Some(
                    self.display
                        .theme
                        .get_color(self.series_mgr.auto_color_index),
                )
            }),
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        self.series_mgr.series.push(series);
        self.series_mgr.auto_color_index += 1;
        self
    }

    /// Internal method to add a Boxen series
    pub(crate) fn add_boxen_series(
        mut self,
        mut boxen_data: crate::plots::BoxenData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        if let Some(line_width) = style.line_width {
            boxen_data.config.line_width = line_width.max(0.0);
        }
        if let Some(marker_size) = style.marker_size {
            boxen_data.config.outlier_size = marker_size.max(0.0);
        }

        let series = PlotSeries {
            series_type: SeriesType::Boxen { data: boxen_data },
            streaming_source: None,
            label: style.label,
            color: style.color.or_else(|| {
                Some(
                    self.display
                        .theme
                        .get_color(self.series_mgr.auto_color_index),
                )
            }),
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        self.series_mgr.series.push(series);
        self.series_mgr.auto_color_index += 1;
        self
    }

    /// Internal method to add a Polar series
    pub(crate) fn add_polar_series(
        mut self,
        polar_data: crate::plots::polar::polar_plot::PolarPlotData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        let series = PlotSeries {
            series_type: SeriesType::Polar { data: polar_data },
            streaming_source: None,
            label: style.label,
            color: style.color.or_else(|| {
                Some(
                    self.display
                        .theme
                        .get_color(self.series_mgr.auto_color_index),
                )
            }),
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: Some(style.inset_layout.unwrap_or_default().normalized()),
            group_id: None,
        };

        self.series_mgr.series.push(series);
        self.series_mgr.auto_color_index += 1;
        self
    }

    /// Internal method to add a Quiver series
    pub(crate) fn add_quiver_series(
        mut self,
        mut quiver_data: crate::plots::QuiverPlotData,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        if let Some(color) = style.color {
            quiver_data.config.color = Some(color);
        }
        if let Some(line_width) = style.line_width {
            quiver_data.config.width = line_width.max(0.1);
        }

        let series = PlotSeries {
            series_type: SeriesType::Quiver { data: quiver_data },
            streaming_source: None,
            label: style.label,
            color: style.color.or_else(|| {
                Some(
                    self.display
                        .theme
                        .get_color(self.series_mgr.auto_color_index),
                )
            }),
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        self.series_mgr.series.push(series);
        self.series_mgr.auto_color_index += 1;
        self
    }

    /// Internal method to add a Line series (used by PlotBuilder<LineConfig>)
    ///
    /// This method is called by the PlotBuilder when finalizing a line series.
    pub(crate) fn add_line_series(
        self,
        x_data: PlotData,
        y_data: PlotData,
        config: &crate::plots::basic::LineConfig,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        self.add_line_series_grouped(x_data, y_data, config, style, None, true)
    }

    /// Internal method to add a Line series with optional grouped-series metadata.
    pub(crate) fn add_line_series_grouped(
        mut self,
        x_data: PlotData,
        y_data: PlotData,
        config: &crate::plots::basic::LineConfig,
        style: crate::core::plot::builder::SeriesStyle,
        group_id: Option<usize>,
        consume_palette_index: bool,
    ) -> Self {
        let series = PlotSeries {
            series_type: SeriesType::Line { x_data, y_data },
            streaming_source: None,
            label: style.label,
            color: style.color.or(config.color).or_else(|| {
                Some(
                    self.display
                        .theme
                        .get_color(self.series_mgr.auto_color_index),
                )
            }),
            color_source: style.color_source,
            line_width: style.line_width.or(config.line_width),
            line_width_source: style.line_width_source,
            line_style: style.line_style.or(Some(config.line_style.clone())),
            line_style_source: style.line_style_source,
            marker_style: style.marker_style.or(config.marker),
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size.or(if config.show_markers {
                Some(config.marker_size)
            } else {
                None
            }),
            marker_size_source: style.marker_size_source,
            alpha: style.alpha.or(Some(config.alpha)),
            alpha_source: style.alpha_source,
            y_errors: style.y_errors,
            x_errors: style.x_errors,
            error_config: style.error_config,
            inset_layout: None,
            group_id,
        };

        self.series_mgr.series.push(series);
        if consume_palette_index {
            self.series_mgr.auto_color_index += 1;
        }
        self
    }

    /// Internal method to add a Scatter series (used by PlotBuilder<ScatterConfig>)
    ///
    /// This method is called by the PlotBuilder when finalizing a scatter series.
    pub(crate) fn add_scatter_series(
        self,
        x_data: PlotData,
        y_data: PlotData,
        config: &crate::plots::basic::ScatterConfig,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        self.add_scatter_series_grouped(x_data, y_data, config, style, None, true)
    }

    /// Internal method to add a Scatter series with optional grouped-series metadata.
    pub(crate) fn add_scatter_series_grouped(
        mut self,
        x_data: PlotData,
        y_data: PlotData,
        config: &crate::plots::basic::ScatterConfig,
        style: crate::core::plot::builder::SeriesStyle,
        group_id: Option<usize>,
        consume_palette_index: bool,
    ) -> Self {
        let series = PlotSeries {
            series_type: SeriesType::Scatter { x_data, y_data },
            streaming_source: None,
            label: style.label,
            color: style.color.or(config.color).or_else(|| {
                Some(
                    self.display
                        .theme
                        .get_color(self.series_mgr.auto_color_index),
                )
            }),
            color_source: style.color_source,
            line_width: style.line_width.or(Some(config.edge_width)),
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style.or(Some(config.marker)),
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size.or(Some(config.size)),
            marker_size_source: style.marker_size_source,
            alpha: style.alpha.or(Some(config.alpha)),
            alpha_source: style.alpha_source,
            y_errors: style.y_errors,
            x_errors: style.x_errors,
            error_config: style.error_config,
            inset_layout: None,
            group_id,
        };

        self.series_mgr.series.push(series);
        if consume_palette_index {
            self.series_mgr.auto_color_index += 1;
        }
        self
    }

    /// Internal method to add a Bar series (used by PlotBuilder<BarConfig>)
    ///
    /// This method is called by the PlotBuilder when finalizing a bar series.
    pub(crate) fn add_bar_series(
        self,
        categories: Vec<String>,
        values: PlotData,
        config: &crate::plots::basic::BarConfig,
        style: crate::core::plot::builder::SeriesStyle,
    ) -> Self {
        self.add_bar_series_grouped(categories, values, config, style, None, true)
    }

    /// Internal method to add a Bar series with optional grouped-series metadata.
    pub(crate) fn add_bar_series_grouped(
        mut self,
        categories: Vec<String>,
        values: PlotData,
        config: &crate::plots::basic::BarConfig,
        style: crate::core::plot::builder::SeriesStyle,
        group_id: Option<usize>,
        consume_palette_index: bool,
    ) -> Self {
        let series = PlotSeries {
            series_type: SeriesType::Bar { categories, values },
            streaming_source: None,
            label: style.label,
            color: style.color.or(config.color).or_else(|| {
                Some(
                    self.display
                        .theme
                        .get_color(self.series_mgr.auto_color_index),
                )
            }),
            color_source: style.color_source,
            line_width: style.line_width.or(Some(config.edge_width)),
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            alpha: style.alpha.or(Some(config.alpha)),
            alpha_source: style.alpha_source,
            y_errors: style.y_errors,
            x_errors: style.x_errors,
            error_config: style.error_config,
            inset_layout: None,
            group_id,
        };

        self.series_mgr.series.push(series);
        if consume_palette_index {
            self.series_mgr.auto_color_index += 1;
        }
        self
    }

    pub(super) fn build_prepared_series_raster_plan(
        &self,
        series: &PlotSeries,
        resolved: &ResolvedSeries<'_>,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        mode: RenderExecutionMode,
    ) -> Result<Option<SeriesRasterPlan>> {
        let color = series.color.unwrap_or(Color::new(0, 0, 0));
        let line_width = self.dpi_scaled_line_width(series.line_width.unwrap_or(2.0));
        let line_style = series.line_style.clone().unwrap_or(LineStyle::Solid);
        let clip_rect = clip_rect_from_plot_area(plot_area);

        let plan = match (&series.series_type, resolved) {
            (SeriesType::Line { .. }, ResolvedSeries::Line { x, y }) => {
                let mut points = project_xy_points(
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
                let mut raster_plan = SeriesRasterPlan::default();

                if series.marker_style.is_none()
                    && series.x_errors.is_none()
                    && series.y_errors.is_none()
                    && let Some(canonicalized) = canonicalize_line_points_exact(points.as_ref())
                {
                    raster_plan.note_exact_line_canonicalization();
                    points = canonicalized.into();
                }

                if mode.allows_raster_line_reduction()
                    && should_reduce_line_series(series, points.len(), plot_area.width())
                    && let Some(reduced) = reduce_line_points_for_raster(
                        points.as_ref(),
                        plot_area.left(),
                        plot_area.width(),
                    )
                {
                    raster_plan.note_raster_line_reduction();
                    points = reduced.into();
                }

                raster_plan.push_polyline(
                    std::sync::Arc::clone(&points),
                    color,
                    line_width,
                    line_style,
                    clip_rect,
                );
                if let Some(marker_style) = series.marker_style {
                    let marker_size = self.dpi_scaled_line_width(series.marker_size.unwrap_or(8.0));
                    raster_plan.push_markers(points, marker_size, marker_style, color, clip_rect);
                }
                Some(raster_plan)
            }
            (SeriesType::Scatter { .. }, ResolvedSeries::Scatter { x, y }) => {
                let marker_size = self.dpi_scaled_line_width(series.marker_size.unwrap_or(10.0));
                let marker_style = series.marker_style.unwrap_or(MarkerStyle::Circle);
                let points = project_xy_points(
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
                let mut raster_plan = SeriesRasterPlan::default();
                raster_plan.push_markers(points, marker_size, marker_style, color, clip_rect);
                Some(raster_plan)
            }
            (SeriesType::Heatmap { data }, ResolvedSeries::Other(_)) => {
                let heatmap_plot_area = plot_area_from_rect(plot_area, x_min, x_max, y_min, y_max);
                RectGridBatch::from_heatmap_data(data, heatmap_plot_area, data.config.alpha).map(
                    |rect_grid| {
                        let mut raster_plan = SeriesRasterPlan::default();
                        raster_plan.push_rect_grid(rect_grid);
                        raster_plan
                    },
                )
            }
            _ => None,
        };

        Ok(plan)
    }

    pub(super) fn render_series_overlays_after_raster(
        &self,
        series: &PlotSeries,
        resolved: &ResolvedSeries<'_>,
        renderer: &mut SkiaRenderer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        color: Color,
        line_width: f32,
        _line_style: &LineStyle,
    ) -> Result<()> {
        match (&series.series_type, resolved) {
            (
                SeriesType::Line { .. } | SeriesType::Scatter { .. },
                ResolvedSeries::Line { x, y } | ResolvedSeries::Scatter { x, y },
            ) => {
                if series.y_errors.is_some() || series.x_errors.is_some() {
                    Self::render_attached_error_bars(
                        renderer,
                        x,
                        y,
                        series.y_errors.as_ref().map(ErrorValuesRef::from),
                        series.x_errors.as_ref().map(ErrorValuesRef::from),
                        series.error_config.as_ref(),
                        color,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        plot_area,
                        line_width,
                        self.render_scale(),
                    )?;
                }
            }
            (SeriesType::Heatmap { data }, ResolvedSeries::Other(_)) => {
                let heatmap_plot_area = plot_area_from_rect(plot_area, x_min, x_max, y_min, y_max);
                let render_scale = self.render_scale();
                let min_annotation_font_px = render_scale.points_to_pixels(8.0);
                let max_annotation_font_px = render_scale.points_to_pixels(20.0);
                for (row_idx, row) in data.values.iter().enumerate() {
                    for (col_idx, &value) in row.iter().enumerate() {
                        if !data.config.annotate || data.should_mask_value(value) {
                            continue;
                        }

                        let cell_color = if data.config.alpha < 1.0 {
                            data.get_color(value).with_alpha(data.config.alpha)
                        } else {
                            data.get_color(value)
                        };

                        let (cell_x, cell_y, cell_width, cell_height) =
                            data.cell_screen_rect(&heatmap_plot_area, row_idx, col_idx);
                        let text = format!("{:.2}", value);
                        let text_color = data.get_text_color(cell_color);
                        let text_x = cell_x + cell_width / 2.0;
                        let font_size = (cell_height * 0.3)
                            .clamp(min_annotation_font_px, max_annotation_font_px);
                        let text_y = cell_y + cell_height / 2.0 + font_size / 3.0;
                        renderer
                            .draw_text_centered(&text, text_x, text_y, font_size, text_color)?;
                    }
                }

                if data.config.colorbar {
                    let colorbar_margin = render_scale.logical_pixels_to_pixels(COLORBAR_MARGIN_PX);
                    let colorbar_width = render_scale.logical_pixels_to_pixels(COLORBAR_WIDTH_PX);
                    let colorbar_x = plot_area.right() + colorbar_margin;
                    let colorbar_y = plot_area.y();
                    let colorbar_height = plot_area.height();

                    renderer.draw_colorbar(
                        &data.config.colormap,
                        data.vmin,
                        data.vmax,
                        colorbar_x,
                        colorbar_y,
                        colorbar_width,
                        colorbar_height,
                        &data.config.value_scale,
                        data.config.colorbar_label.as_deref(),
                        self.display.theme.foreground,
                        data.config.colorbar_tick_font_size,
                        Some(data.config.colorbar_label_font_size),
                        data.config.colorbar_log_subticks,
                    )?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn render_quiver_series_scaled(
        &self,
        renderer: &mut SkiaRenderer,
        data: &crate::plots::QuiverPlotData,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        default_color: Color,
    ) -> Result<()> {
        if data.arrows.is_empty() {
            return Ok(());
        }

        let base_color = data.config.color.unwrap_or(default_color);
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
        let arrow_width = self.render_scale().points_to_pixels(data.config.width);

        for arrow in &data.arrows {
            let arrow_color = cmap
                .as_ref()
                .map(|colormap| colormap.sample((arrow.magnitude - min_mag) / mag_range))
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
            renderer.draw_line(
                sx1,
                sy1,
                sx2,
                sy2,
                arrow_color,
                arrow_width,
                LineStyle::Solid,
            )?;

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
            renderer.draw_filled_polygon(&head, arrow_color)?;
        }

        Ok(())
    }

    /// Helper method to render a single series using normal (non-DataShader) rendering
    pub(super) fn render_series_normal(
        &self,
        series: &PlotSeries,
        resolved: &ResolvedSeries<'_>,
        renderer: &mut SkiaRenderer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        mode: RenderExecutionMode,
    ) -> Result<()> {
        let color = series.color.unwrap_or(Color::new(0, 0, 0)); // Default black
        let line_width = self.dpi_scaled_line_width(series.line_width.unwrap_or(2.0));
        let line_style = series.line_style.clone().unwrap_or(LineStyle::Solid);
        let clip_rect = clip_rect_from_plot_area(plot_area);

        if let Some(raster_plan) = self.build_prepared_series_raster_plan(
            series, resolved, plot_area, x_min, x_max, y_min, y_max, mode,
        )? {
            raster_plan.execute(renderer)?;
            self.render_series_overlays_after_raster(
                series,
                resolved,
                renderer,
                plot_area,
                x_min,
                x_max,
                y_min,
                y_max,
                color,
                line_width,
                &line_style,
            )?;
            return Ok(());
        }

        match (&series.series_type, resolved) {
            (SeriesType::Line { .. }, ResolvedSeries::Line { .. })
            | (SeriesType::Scatter { .. }, ResolvedSeries::Scatter { .. }) => unreachable!(
                "cacheable line/scatter series should return before fallback rendering"
            ),
            (SeriesType::Bar { .. }, ResolvedSeries::Bar { values, .. }) => {
                // Bar width as fraction of category spacing (0.8 = 80%, matching matplotlib)
                let bar_width_fraction = 0.8;
                let data_range = (x_max - x_min) as f32;
                let pixels_per_unit = plot_area.width() / data_range;
                let bar_width = bar_width_fraction * pixels_per_unit;

                for (i, &value) in values.iter().enumerate() {
                    let x = i as f64;
                    let (px, py) = crate::render::skia::map_data_to_pixels(
                        x, value, x_min, x_max, y_min, y_max, plot_area,
                    );
                    let (_, py_zero) = crate::render::skia::map_data_to_pixels(
                        x, 0.0, x_min, x_max, y_min, y_max, plot_area,
                    );
                    renderer.draw_rectangle_clipped(
                        px - bar_width / 2.0,
                        py.min(py_zero),
                        bar_width,
                        (py - py_zero).abs(),
                        color,
                        true,
                        clip_rect,
                    )?;
                }
            }
            (SeriesType::Histogram { .. }, ResolvedSeries::Histogram { data: hist_data }) => {
                // Render histogram bars
                for (i, &count) in hist_data.counts.iter().enumerate() {
                    if count > 0.0 {
                        let x_left = hist_data.bin_edges[i];
                        let x_right = hist_data.bin_edges[i + 1];
                        let x_center = (x_left + x_right) / 2.0;

                        // Convert bar width from data coordinates to pixel coordinates
                        let (px_left, _) = crate::render::skia::map_data_to_pixels(
                            x_left, 0.0, x_min, x_max, y_min, y_max, plot_area,
                        );
                        let (px_right, _) = crate::render::skia::map_data_to_pixels(
                            x_right, 0.0, x_min, x_max, y_min, y_max, plot_area,
                        );
                        let bar_width_px = (px_right - px_left).abs();

                        let (px, py) = crate::render::skia::map_data_to_pixels(
                            x_center, count, x_min, x_max, y_min, y_max, plot_area,
                        );
                        let (_, py_zero) = crate::render::skia::map_data_to_pixels(
                            x_center, 0.0, x_min, x_max, y_min, y_max, plot_area,
                        );

                        renderer.draw_rectangle_clipped(
                            px - bar_width_px / 2.0,
                            py.min(py_zero),
                            bar_width_px,
                            (py - py_zero).abs(),
                            color,
                            true,
                            clip_rect,
                        )?;
                    }
                }
            }
            (SeriesType::BoxPlot { .. }, ResolvedSeries::BoxPlot { data, config }) => {
                // Calculate box plot statistics
                let box_data = crate::plots::boxplot::calculate_box_plot(&data.as_ref(), config)
                    .map_err(|e| {
                        PlottingError::RenderError(format!("Box plot calculation failed: {}", e))
                    })?;

                // Box plot positioning
                let x_center = 0.5; // Center the box plot
                let box_width = 0.3; // Box width

                // Map coordinates to pixels
                let (x_center_px, _) = crate::render::skia::map_data_to_pixels(
                    x_center, 0.0, x_min, x_max, y_min, y_max, plot_area,
                );
                let (_, q1_y) = crate::render::skia::map_data_to_pixels(
                    0.0,
                    box_data.q1,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );
                let (_, median_y) = crate::render::skia::map_data_to_pixels(
                    0.0,
                    box_data.median,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );
                let (_, q3_y) = crate::render::skia::map_data_to_pixels(
                    0.0,
                    box_data.q3,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );
                let (_, lower_whisker_y) = crate::render::skia::map_data_to_pixels(
                    0.0,
                    box_data.min,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );
                let (_, upper_whisker_y) = crate::render::skia::map_data_to_pixels(
                    0.0,
                    box_data.max,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                );

                let box_half_width = box_width * plot_area.width() * 0.5;
                let box_left = x_center_px - box_half_width;
                let box_right = x_center_px + box_half_width;

                // Draw the box (IQR) - ensure positive dimensions
                let box_width = box_right - box_left;
                let box_height = (q1_y - q3_y).abs(); // Ensure positive height
                let box_top = q3_y.min(q1_y); // Use the smaller y value as top

                // Validate dimensions before drawing
                if box_width > 0.0
                    && box_height > 0.0
                    && box_width.is_finite()
                    && box_height.is_finite()
                {
                    renderer.draw_rectangle_clipped(
                        box_left, box_top, box_width, box_height, color,
                        false, // outline only
                        clip_rect,
                    )?;
                }

                // Draw median line - validate coordinates
                if box_left.is_finite() && median_y.is_finite() && box_right.is_finite() {
                    renderer.draw_line_clipped(
                        box_left,
                        median_y,
                        box_right,
                        median_y,
                        color,
                        line_width * 1.5, // thicker median line
                        line_style.clone(),
                        clip_rect,
                    )?;
                }

                // Draw lower whisker - validate coordinates
                if x_center_px.is_finite() && q1_y.is_finite() && lower_whisker_y.is_finite() {
                    renderer.draw_line_clipped(
                        x_center_px,
                        q1_y,
                        x_center_px,
                        lower_whisker_y,
                        color,
                        line_width,
                        line_style.clone(),
                        clip_rect,
                    )?;
                }

                // Draw upper whisker - validate coordinates
                if x_center_px.is_finite() && q3_y.is_finite() && upper_whisker_y.is_finite() {
                    renderer.draw_line_clipped(
                        x_center_px,
                        q3_y,
                        x_center_px,
                        upper_whisker_y,
                        color,
                        line_width,
                        line_style.clone(),
                        clip_rect,
                    )?;
                }

                // Draw whisker caps - validate coordinates
                let cap_width = box_half_width * 0.6;
                if x_center_px.is_finite() && lower_whisker_y.is_finite() && cap_width.is_finite() {
                    renderer.draw_line_clipped(
                        x_center_px - cap_width,
                        lower_whisker_y,
                        x_center_px + cap_width,
                        lower_whisker_y,
                        color,
                        line_width,
                        line_style.clone(),
                        clip_rect,
                    )?;
                }

                if x_center_px.is_finite() && upper_whisker_y.is_finite() && cap_width.is_finite() {
                    renderer.draw_line_clipped(
                        x_center_px - cap_width,
                        upper_whisker_y,
                        x_center_px + cap_width,
                        upper_whisker_y,
                        color,
                        line_width,
                        line_style.clone(),
                        clip_rect,
                    )?;
                }

                // Draw outliers - validate coordinates
                let outlier_marker_size = self.render_scale().points_to_pixels(4.0);
                for &outlier in &box_data.outliers {
                    let (_, outlier_y) = crate::render::skia::map_data_to_pixels(
                        0.0, outlier, x_min, x_max, y_min, y_max, plot_area,
                    );
                    if x_center_px.is_finite() && outlier_y.is_finite() {
                        renderer.draw_marker_clipped(
                            x_center_px,
                            outlier_y,
                            outlier_marker_size,
                            MarkerStyle::Circle,
                            color,
                            clip_rect,
                        )?;
                    }
                }
            }
            (SeriesType::Heatmap { data }, ResolvedSeries::Other(_)) => {
                let heatmap_plot_area = plot_area_from_rect(plot_area, x_min, x_max, y_min, y_max);
                data.draw_cells_batch(renderer, &heatmap_plot_area, data.config.alpha)?;
                self.render_series_overlays_after_raster(
                    series,
                    resolved,
                    renderer,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    color,
                    line_width,
                    &line_style,
                )?;
            }
            (SeriesType::ErrorBars { .. }, ResolvedSeries::ErrorBars { x, y, y_errors }) => {
                // Draw markers at data points
                let marker_size = self.dpi_scaled_line_width(series.marker_size.unwrap_or(8.0));
                let marker_style = series.marker_style.unwrap_or(MarkerStyle::Circle);

                for (&x_value, &y_value) in x.iter().zip(y.iter()) {
                    if x_value.is_finite() && y_value.is_finite() {
                        let (px, py) = crate::render::skia::map_data_to_pixels(
                            x_value, y_value, x_min, x_max, y_min, y_max, plot_area,
                        );
                        renderer.draw_marker_clipped(
                            px,
                            py,
                            marker_size,
                            marker_style,
                            color,
                            clip_rect,
                        )?;
                    }
                }

                // Draw Y error bars
                Self::render_attached_error_bars(
                    renderer,
                    x,
                    y,
                    Some(effective_error_values(series.y_errors.as_ref(), y_errors)),
                    series.x_errors.as_ref().map(ErrorValuesRef::from),
                    series.error_config.as_ref(),
                    color,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                    line_width,
                    self.render_scale(),
                )?;
            }
            (
                SeriesType::ErrorBarsXY { .. },
                ResolvedSeries::ErrorBarsXY {
                    x,
                    y,
                    x_errors,
                    y_errors,
                },
            ) => {
                // Draw markers at data points
                let marker_size = self.dpi_scaled_line_width(series.marker_size.unwrap_or(8.0));
                let marker_style = series.marker_style.unwrap_or(MarkerStyle::Circle);

                for (&x_value, &y_value) in x.iter().zip(y.iter()) {
                    if x_value.is_finite() && y_value.is_finite() {
                        let (px, py) = crate::render::skia::map_data_to_pixels(
                            x_value, y_value, x_min, x_max, y_min, y_max, plot_area,
                        );
                        renderer.draw_marker_clipped(
                            px,
                            py,
                            marker_size,
                            marker_style,
                            color,
                            clip_rect,
                        )?;
                    }
                }

                // Draw X and Y error bars
                Self::render_attached_error_bars(
                    renderer,
                    x,
                    y,
                    Some(effective_error_values(series.y_errors.as_ref(), y_errors)),
                    Some(effective_error_values(series.x_errors.as_ref(), x_errors)),
                    series.error_config.as_ref(),
                    color,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    plot_area,
                    line_width,
                    self.render_scale(),
                )?;
            }
            (SeriesType::Kde { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render KDE
                let plot_area = crate::plots::PlotArea::new(
                    plot_area.x(),
                    plot_area.y(),
                    plot_area.width(),
                    plot_area.height(),
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                );
                data.render(renderer, &plot_area, &self.display.theme, color)?;
            }
            (SeriesType::Ecdf { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render ECDF
                let plot_area = crate::plots::PlotArea::new(
                    plot_area.x(),
                    plot_area.y(),
                    plot_area.width(),
                    plot_area.height(),
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                );
                data.render(renderer, &plot_area, &self.display.theme, color)?;
            }
            (SeriesType::Violin { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Violin
                let plot_area = crate::plots::PlotArea::new(
                    plot_area.x(),
                    plot_area.y(),
                    plot_area.width(),
                    plot_area.height(),
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                );
                data.render(renderer, &plot_area, &self.display.theme, color)?;
            }
            (SeriesType::Boxen { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Boxen
                let plot_area = crate::plots::PlotArea::new(
                    plot_area.x(),
                    plot_area.y(),
                    plot_area.width(),
                    plot_area.height(),
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                );
                data.render(renderer, &plot_area, &self.display.theme, color)?;
            }
            (SeriesType::Quiver { data }, ResolvedSeries::Other(_)) => {
                self.render_quiver_series_scaled(
                    renderer, data, plot_area, x_min, x_max, y_min, y_max, color,
                )?;
            }
            (SeriesType::Contour { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Contour
                let contour_plot_area = crate::plots::PlotArea::new(
                    plot_area.x(),
                    plot_area.y(),
                    plot_area.width(),
                    plot_area.height(),
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                );
                data.render(renderer, &contour_plot_area, &self.display.theme, color)?;

                // Draw colorbar if enabled
                if data.config.colorbar {
                    let render_scale = self.render_scale();
                    let colorbar_margin = render_scale.logical_pixels_to_pixels(COLORBAR_MARGIN_PX);
                    let colorbar_width = render_scale.logical_pixels_to_pixels(COLORBAR_WIDTH_PX);
                    let colorbar_x = plot_area.right() + colorbar_margin;
                    let colorbar_y = plot_area.y();
                    let colorbar_height = plot_area.height();

                    // Get value range from contour data
                    let (vmin, vmax) = if data.levels.is_empty() {
                        (0.0, 1.0)
                    } else {
                        (
                            data.levels.first().copied().unwrap_or(0.0),
                            data.levels.last().copied().unwrap_or(1.0),
                        )
                    };

                    // Get colormap from contour config
                    let colormap = crate::render::ColorMap::by_name(&data.config.cmap)
                        .unwrap_or_else(crate::render::ColorMap::viridis);

                    renderer.draw_colorbar(
                        &colormap,
                        vmin,
                        vmax,
                        colorbar_x,
                        colorbar_y,
                        colorbar_width,
                        colorbar_height,
                        &crate::axes::AxisScale::Linear,
                        data.config.colorbar_label.as_deref(),
                        self.display.theme.foreground,
                        data.config.colorbar_tick_font_size,
                        Some(data.config.colorbar_label_font_size),
                        false,
                    )?;
                }
            }
            (SeriesType::Pie { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Pie with 1:1 aspect ratio
                // (uses normalized 0-1 coordinates)
                let (pie_x, pie_y, pie_size) = {
                    let size = plot_area.width().min(plot_area.height());
                    let x_offset = (plot_area.width() - size) / 2.0;
                    let y_offset = (plot_area.height() - size) / 2.0;
                    (plot_area.x() + x_offset, plot_area.y() + y_offset, size)
                };
                let pie_plot_area = crate::plots::PlotArea::new(
                    pie_x, pie_y, pie_size, pie_size, 0.0, 1.0, 0.0, 1.0,
                );
                data.render(renderer, &pie_plot_area, &self.display.theme, color)?;
            }
            (SeriesType::Radar { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Radar with 1:1 aspect ratio
                // and extra top padding for title clearance
                let radar_plot_area = Self::radar_plot_area(plot_area, x_min, x_max, y_min, y_max);
                data.render(renderer, &radar_plot_area, &self.display.theme, color)?;
            }
            (SeriesType::Polar { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Polar with 1:1 aspect ratio
                // Center the square plot area within available space
                let (polar_x, polar_y, polar_size) = {
                    let size = plot_area.width().min(plot_area.height());
                    let x_offset = (plot_area.width() - size) / 2.0;
                    let y_offset = (plot_area.height() - size) / 2.0;
                    (plot_area.x() + x_offset, plot_area.y() + y_offset, size)
                };
                let polar_plot_area = crate::plots::PlotArea::new(
                    polar_x, polar_y, polar_size, polar_size, x_min, x_max, y_min, y_max,
                );
                data.render(renderer, &polar_plot_area, &self.display.theme, color)?;
            }
            _ => unreachable!("resolved series variant must match its declarative series"),
        }

        Ok(())
    }

    /// Render a series using GPU-accelerated coordinate transformation
    ///
    /// Uses GPU compute shaders for coordinate transformation when available,
    /// falling back to CPU for the actual drawing operations.
    #[cfg(feature = "gpu")]
    pub(super) fn render_series_gpu(
        &self,
        series: &PlotSeries,
        resolved: &ResolvedSeries<'_>,
        renderer: &mut SkiaRenderer,
        gpu_renderer: &mut GpuRenderer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        mode: RenderExecutionMode,
    ) -> Result<()> {
        let color = series.color.unwrap_or(Color::new(0, 0, 0));
        let line_width = self.dpi_scaled_line_width(series.line_width.unwrap_or(2.0));
        let line_style = series.line_style.clone().unwrap_or(LineStyle::Solid);
        let clip_rect = (
            plot_area.x(),
            plot_area.y(),
            plot_area.width(),
            plot_area.height(),
        );

        match (&series.series_type, resolved) {
            (SeriesType::Line { .. }, ResolvedSeries::Line { x, y }) => {
                // Use GPU for coordinate transformation
                let viewport = (
                    plot_area.x(),
                    plot_area.y(),
                    plot_area.x() + plot_area.width(),
                    plot_area.y() + plot_area.height(),
                );

                let (x_transformed, y_transformed) = gpu_renderer
                    .transform_coordinates_optimal(
                        &x.as_ref(),
                        &y.as_ref(),
                        (x_min, x_max),
                        (y_min, y_max),
                        viewport,
                    )
                    .map_err(|e| {
                        PlottingError::RenderError(format!("GPU transform failed: {}", e))
                    })?;

                // Convert to points for drawing
                let points: Vec<(f32, f32)> = x_transformed
                    .iter()
                    .zip(y_transformed.iter())
                    .map(|(&x, &y)| (x, y))
                    .collect();

                renderer
                    .draw_polyline_clipped(&points, color, line_width, line_style, clip_rect)?;
                if let Some(marker_style) = series.marker_style {
                    let marker_size = self.dpi_scaled_line_width(series.marker_size.unwrap_or(8.0));
                    for &(px, py) in &points {
                        renderer.draw_marker_clipped(
                            px,
                            py,
                            marker_size,
                            marker_style,
                            color,
                            clip_rect,
                        )?;
                    }
                }
            }
            (SeriesType::Scatter { .. }, ResolvedSeries::Scatter { x, y }) => {
                // Use GPU for coordinate transformation
                let viewport = (
                    plot_area.x(),
                    plot_area.y(),
                    plot_area.x() + plot_area.width(),
                    plot_area.y() + plot_area.height(),
                );

                let (x_transformed, y_transformed) = gpu_renderer
                    .transform_coordinates_optimal(
                        &x.as_ref(),
                        &y.as_ref(),
                        (x_min, x_max),
                        (y_min, y_max),
                        viewport,
                    )
                    .map_err(|e| {
                        PlottingError::RenderError(format!("GPU transform failed: {}", e))
                    })?;

                let marker_size = self.dpi_scaled_line_width(series.marker_size.unwrap_or(10.0));
                let marker_style = series.marker_style.unwrap_or(MarkerStyle::Circle);

                // Draw markers at transformed coordinates
                for (&px, &py) in x_transformed.iter().zip(y_transformed.iter()) {
                    renderer.draw_marker_clipped(
                        px,
                        py,
                        marker_size,
                        marker_style,
                        color,
                        clip_rect,
                    )?;
                }
            }
            // For other series types, fall back to normal rendering
            _ => {
                self.render_series_normal(
                    series, resolved, renderer, plot_area, x_min, x_max, y_min, y_max, mode,
                )?;
            }
        }

        Ok(())
    }

    pub(super) fn validate_series_list(series_list: &[PlotSeries]) -> Result<()> {
        if series_list.is_empty() {
            return Err(PlottingError::NoDataSeries);
        }

        for (idx, series) in series_list.iter().enumerate() {
            match &series.series_type {
                SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                    let x_data = x_data.resolve_cow(0.0);
                    let y_data = y_data.resolve_cow(0.0);
                    if x_data.len() != y_data.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                            series_index: Some(idx),
                        });
                    }
                    if x_data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(&x_data)?;
                    PlottingError::validate_data(&y_data)?;
                }
                SeriesType::Bar { categories, values } => {
                    let values = values.resolve_cow(0.0);
                    if categories.len() != values.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: categories.len(),
                            y_len: values.len(),
                            series_index: Some(idx),
                        });
                    }
                    if categories.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(&values)?;
                }
                SeriesType::ErrorBars {
                    x_data,
                    y_data,
                    y_errors,
                } => {
                    let x_data = x_data.resolve_cow(0.0);
                    let y_data = y_data.resolve_cow(0.0);
                    let y_errors = y_errors.resolve_cow(0.0);
                    if x_data.len() != y_data.len() || y_data.len() != y_errors.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                            series_index: Some(idx),
                        });
                    }
                    PlottingError::validate_data(&x_data)?;
                    PlottingError::validate_data(&y_data)?;
                    PlottingError::validate_data(&y_errors)?;
                }
                SeriesType::ErrorBarsXY {
                    x_data,
                    y_data,
                    x_errors,
                    y_errors,
                } => {
                    let x_data = x_data.resolve_cow(0.0);
                    let y_data = y_data.resolve_cow(0.0);
                    let x_errors = x_errors.resolve_cow(0.0);
                    let y_errors = y_errors.resolve_cow(0.0);
                    if x_data.len() != y_data.len()
                        || x_data.len() != x_errors.len()
                        || x_data.len() != y_errors.len()
                    {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x_data.len(),
                            y_len: y_data.len(),
                            series_index: Some(idx),
                        });
                    }
                    PlottingError::validate_data(&x_data)?;
                    PlottingError::validate_data(&y_data)?;
                    PlottingError::validate_data(&x_errors)?;
                    PlottingError::validate_data(&y_errors)?;
                }
                SeriesType::Histogram { data, .. } => {
                    let data = data.resolve_cow(0.0);
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(&data)?;
                }
                SeriesType::BoxPlot { data, .. } => {
                    let data = data.resolve_cow(0.0);
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(&data)?;
                }
                SeriesType::Heatmap { data } => {
                    if data.values.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Kde { data } => {
                    if data.x.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Ecdf { data } => {
                    if data.x.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Violin { data } => {
                    if data.data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Boxen { data } => {
                    if data.boxes.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Quiver { data } => {
                    if data.arrows.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    for (index, arrow) in data.arrows.iter().enumerate() {
                        let all_values = [
                            arrow.start.0,
                            arrow.start.1,
                            arrow.end.0,
                            arrow.end.1,
                            arrow.magnitude,
                            arrow.angle,
                            arrow.head[0].0,
                            arrow.head[0].1,
                            arrow.head[1].0,
                            arrow.head[1].1,
                            arrow.head[2].0,
                            arrow.head[2].1,
                        ];
                        if let Some(value) = all_values.iter().find(|value| !value.is_finite()) {
                            return Err(PlottingError::InvalidData {
                                message: format!("Non-finite quiver arrow value ({value}) found"),
                                position: Some(index),
                            });
                        }
                    }
                }
                SeriesType::Contour { data } => {
                    if data.levels.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Pie { data } => {
                    if data.values.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Radar { data } => {
                    if data.series.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                SeriesType::Polar { data } => {
                    if data.points.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
            }
        }

        Ok(())
    }

    pub(super) fn validate_resolved_series(series_list: &[ResolvedSeries<'_>]) -> Result<()> {
        if series_list.is_empty() {
            return Err(PlottingError::NoDataSeries);
        }

        for (idx, series) in series_list.iter().enumerate() {
            match series {
                ResolvedSeries::Line { x, y } | ResolvedSeries::Scatter { x, y } => {
                    if x.len() != y.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x.len(),
                            y_len: y.len(),
                            series_index: Some(idx),
                        });
                    }
                    if x.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(x)?;
                    PlottingError::validate_data(y)?;
                }
                ResolvedSeries::Bar { categories, values } => {
                    if categories.len() != values.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: categories.len(),
                            y_len: values.len(),
                            series_index: Some(idx),
                        });
                    }
                    if categories.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(values)?;
                }
                ResolvedSeries::ErrorBars { x, y, y_errors } => {
                    if x.len() != y.len() || y.len() != y_errors.len() {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x.len(),
                            y_len: y.len(),
                            series_index: Some(idx),
                        });
                    }
                    PlottingError::validate_data(x)?;
                    PlottingError::validate_data(y)?;
                    PlottingError::validate_data(y_errors)?;
                }
                ResolvedSeries::ErrorBarsXY {
                    x,
                    y,
                    x_errors,
                    y_errors,
                } => {
                    if x.len() != y.len() || x.len() != x_errors.len() || x.len() != y_errors.len()
                    {
                        return Err(PlottingError::DataLengthMismatch {
                            x_len: x.len(),
                            y_len: y.len(),
                            series_index: Some(idx),
                        });
                    }
                    PlottingError::validate_data(x)?;
                    PlottingError::validate_data(y)?;
                    PlottingError::validate_data(x_errors)?;
                    PlottingError::validate_data(y_errors)?;
                }
                ResolvedSeries::Histogram { data } => {
                    if data.counts.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                }
                ResolvedSeries::BoxPlot { data, .. } => {
                    if data.is_empty() {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    PlottingError::validate_data(data)?;
                }
                ResolvedSeries::Other(series) => match series {
                    SeriesType::Heatmap { data } if data.values.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Kde { data } if data.x.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Ecdf { data } if data.x.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Violin { data } if data.data.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Boxen { data } if data.boxes.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Quiver { data } if data.arrows.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Contour { data } if data.levels.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Pie { data } if data.values.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Radar { data } if data.series.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Polar { data } if data.points.is_empty() => {
                        return Err(PlottingError::EmptyDataSet);
                    }
                    SeriesType::Quiver { data } => {
                        for (position, arrow) in data.arrows.iter().enumerate() {
                            let all_values = [
                                arrow.start.0,
                                arrow.start.1,
                                arrow.end.0,
                                arrow.end.1,
                                arrow.magnitude,
                                arrow.angle,
                                arrow.head[0].0,
                                arrow.head[0].1,
                                arrow.head[1].0,
                                arrow.head[1].1,
                                arrow.head[2].0,
                                arrow.head[2].1,
                            ];
                            if let Some(value) = all_values.iter().find(|value| !value.is_finite())
                            {
                                return Err(PlottingError::InvalidData {
                                    message: format!(
                                        "Non-finite quiver arrow value ({value}) found"
                                    ),
                                    position: Some(position),
                                });
                            }
                        }
                    }
                    _ => {}
                },
            }
        }

        Ok(())
    }

    /// Internal validation logic for series data
    pub(super) fn validate_series(&self) -> Result<()> {
        if let Some(err) = self.pending_ingestion_error() {
            return Err(err);
        }

        Self::validate_series_list(&self.series_mgr.series)
    }

    pub(super) fn validate_runtime_environment(&self) -> Result<()> {
        if let Some(err) = self.pending_ingestion_error() {
            return Err(err);
        }

        self.validate_output_config()?;
        self.validate_annotations()?;
        Ok(())
    }

    pub(super) fn validate_runtime_inputs_for_series(
        &self,
        series_list: &[PlotSeries],
    ) -> Result<()> {
        self.validate_runtime_environment()?;
        Self::validate_series_list(series_list)
    }

    pub(super) fn validate_annotations(&self) -> Result<()> {
        for annotation in &self.annotations {
            if let Annotation::FillBetween { x, y1, y2, .. } = annotation {
                if x.len() != y1.len() || x.len() != y2.len() {
                    return Err(PlottingError::DataLengthMismatch {
                        x_len: x.len(),
                        y_len: y1.len().max(y2.len()),
                        series_index: None,
                    });
                }
                PlottingError::validate_data(x)?;
                PlottingError::validate_data(y1)?;
                PlottingError::validate_data(y2)?;
            }
        }

        Ok(())
    }

    pub(super) fn validate_output_config(&self) -> Result<()> {
        let figure = &self.display.config.figure;
        if !figure.dpi.is_finite() {
            return Err(PlottingError::InvalidInput(format!(
                "Figure DPI must be a finite value (dpi={})",
                figure.dpi
            )));
        }
        if figure.dpi <= 0.0 {
            return Err(PlottingError::InvalidInput(format!(
                "Figure DPI must be positive (dpi={})",
                figure.dpi
            )));
        }
        if figure.dpi < crate::core::constants::dpi::MIN as f32 && !self.render.allow_subminimum_dpi
        {
            return Err(PlottingError::InvalidInput(format!(
                "Figure DPI must be at least {} (dpi={})",
                crate::core::constants::dpi::MIN,
                figure.dpi
            )));
        }
        if figure.dpi > crate::core::constants::dpi::MAX as f32 {
            return Err(PlottingError::PerformanceLimit {
                limit_type: "DPI".to_string(),
                actual: figure.dpi.ceil() as usize,
                maximum: crate::core::constants::dpi::MAX as usize,
            });
        }
        if !figure.width.is_finite() || !figure.height.is_finite() {
            return Err(PlottingError::InvalidInput(format!(
                "Figure width/height must be finite values (width={}, height={})",
                figure.width, figure.height
            )));
        }
        let (width, height) = self.config_canvas_size();
        if self.render.allow_subplot_dimensions {
            PlottingError::validate_subplot_dimensions(width, height)?;
        } else {
            PlottingError::validate_dimensions(width, height)?;
        }
        self.display.config.margins.validate_for_figure(figure)?;
        Ok(())
    }

    pub(super) fn validate_runtime_inputs(&self) -> Result<()> {
        self.validate_runtime_inputs_for_series(&self.series_mgr.series)
    }
}
