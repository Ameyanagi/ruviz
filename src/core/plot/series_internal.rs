use super::*;
use crate::core::Point2f;
use crate::core::plot::raster_batches::{
    RectGridBatch, SeriesRasterPlan, clip_rect_from_plot_area, plot_area_from_rect,
    project_xy_points, project_xy_subpaths,
};
use crate::core::plot::raster_fast_path::{
    canonicalize_line_points_exact, reduce_line_points_for_raster, should_reduce_line_series,
};
use crate::core::plot::types::MarkerEdge;
use crate::plots::traits::AxisScaleSupport;

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
            color: None,
            color_source: None,
            line_width: None,
            line_width_source: None,
            line_style: None,
            line_style_source: None,
            marker_style: None,
            marker_style_source: None,
            marker_size: None,
            marker_size_source: None,
            marker_edge: None,
            alpha: None,
            alpha_source: None,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
            resolved_radar_colors: None,
        };

        let auto_color_slot = (series.color.is_none() && series.color_source.is_none())
            .then_some(self.series_mgr.auto_color_index);
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
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
            series_type: SeriesType::Kde {
                data: Arc::new(kde_data),
            },
            streaming_source: style.streaming_source,
            label: style.label,
            color: style.color,
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            marker_edge: None,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
            resolved_radar_colors: None,
        };

        let auto_color_slot = (series.color.is_none() && series.color_source.is_none())
            .then_some(self.series_mgr.auto_color_index);
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
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
            series_type: SeriesType::Ecdf {
                data: Arc::new(ecdf_data),
            },
            streaming_source: style.streaming_source,
            label: style.label,
            color: style.color,
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            marker_edge: None,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
            resolved_radar_colors: None,
        };

        let auto_color_slot = (series.color.is_none() && series.color_source.is_none())
            .then_some(self.series_mgr.auto_color_index);
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
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
            series_type: SeriesType::Contour {
                data: Arc::new(contour_data),
            },
            streaming_source: style.streaming_source,
            label: style.label,
            color: style.color,
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            marker_edge: None,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
            resolved_radar_colors: None,
        };

        let auto_color_slot = (series.color.is_none() && series.color_source.is_none())
            .then_some(self.series_mgr.auto_color_index);
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
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
            series_type: SeriesType::Pie {
                data: Arc::new(pie_data),
            },
            streaming_source: style.streaming_source,
            label: style.label,
            color: style.color,
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            marker_edge: None,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: Some(style.inset_layout.unwrap_or_default().normalized()),
            group_id: None,
            resolved_radar_colors: None,
        };

        let auto_color_slot = (series.color.is_none() && series.color_source.is_none())
            .then_some(self.series_mgr.auto_color_index);
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
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
            series_type: SeriesType::Radar {
                data: Arc::new(radar_data),
            },
            streaming_source: style.streaming_source,
            label: style.label,
            color: style.color,
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            marker_edge: None,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: Some(style.inset_layout.unwrap_or_default().normalized()),
            group_id: None,
            resolved_radar_colors: None,
        };

        let auto_color_slot = (series.color.is_none() && series.color_source.is_none())
            .then_some(self.series_mgr.auto_color_index);
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
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
            series_type: SeriesType::Violin {
                data: Arc::new(violin_data),
            },
            streaming_source: style.streaming_source,
            label: style.label,
            color: style.color,
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            marker_edge: None,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
            resolved_radar_colors: None,
        };

        let auto_color_slot = (series.color.is_none() && series.color_source.is_none())
            .then_some(self.series_mgr.auto_color_index);
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
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
            series_type: SeriesType::Boxen {
                data: Arc::new(boxen_data),
            },
            streaming_source: style.streaming_source,
            label: style.label,
            color: style.color,
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            marker_edge: None,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
            resolved_radar_colors: None,
        };

        let auto_color_slot = (series.color.is_none() && series.color_source.is_none())
            .then_some(self.series_mgr.auto_color_index);
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
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
            series_type: SeriesType::Polar {
                data: Arc::new(polar_data),
            },
            streaming_source: style.streaming_source,
            label: style.label,
            color: style.color,
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            marker_edge: None,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: Some(style.inset_layout.unwrap_or_default().normalized()),
            group_id: None,
            resolved_radar_colors: None,
        };

        let auto_color_slot = (series.color.is_none() && series.color_source.is_none())
            .then_some(self.series_mgr.auto_color_index);
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
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
            series_type: SeriesType::Quiver {
                data: Arc::new(quiver_data),
            },
            streaming_source: style.streaming_source,
            label: style.label,
            color: style.color,
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            marker_edge: None,
            alpha: style.alpha,
            alpha_source: style.alpha_source,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
            resolved_radar_colors: None,
        };

        let auto_color_slot = (series.color.is_none() && series.color_source.is_none())
            .then_some(self.series_mgr.auto_color_index);
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
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
            streaming_source: style.streaming_source,
            label: style.label,
            color: style.color,
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style.or(Some(config.line_style.clone())),
            line_style_source: style.line_style_source,
            marker_style: style.marker_style.or(config.marker),
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            marker_edge: config
                .resolved_marker_edge_spec()
                .map(|(color, width)| MarkerEdge { color, width }),
            alpha: style.alpha.or(Some(config.alpha)),
            alpha_source: style.alpha_source,
            y_errors: style.y_errors,
            x_errors: style.x_errors,
            error_config: style.error_config,
            inset_layout: None,
            group_id,
            resolved_radar_colors: None,
        };

        let auto_color_slot = if series.color.is_none() && series.color_source.is_none() {
            Some(if consume_palette_index {
                self.series_mgr.auto_color_index
            } else {
                self.series_mgr.auto_color_index.saturating_sub(1)
            })
        } else {
            None
        };
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
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
            streaming_source: style.streaming_source,
            label: style.label,
            color: style.color,
            color_source: style.color_source,
            line_width: style.line_width,
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style.or(Some(config.marker)),
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            marker_edge: config
                .resolved_edge_spec()
                .map(|(color, width)| MarkerEdge { color, width }),
            alpha: style.alpha.or(Some(config.alpha)),
            alpha_source: style.alpha_source,
            y_errors: style.y_errors,
            x_errors: style.x_errors,
            error_config: style.error_config,
            inset_layout: None,
            group_id,
            resolved_radar_colors: None,
        };

        let auto_color_slot = if series.color.is_none() && series.color_source.is_none() {
            Some(if consume_palette_index {
                self.series_mgr.auto_color_index
            } else {
                self.series_mgr.auto_color_index.saturating_sub(1)
            })
        } else {
            None
        };
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
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
            series_type: SeriesType::Bar {
                categories,
                values,
                config: config.clone(),
            },
            streaming_source: style.streaming_source,
            label: style.label,
            color: style.color.or(config.color),
            color_source: style.color_source,
            line_width: style.line_width.or(Some(config.edge_width)),
            line_width_source: style.line_width_source,
            line_style: style.line_style,
            line_style_source: style.line_style_source,
            marker_style: style.marker_style,
            marker_style_source: style.marker_style_source,
            marker_size: style.marker_size,
            marker_size_source: style.marker_size_source,
            marker_edge: None,
            alpha: style.alpha.or(Some(config.alpha)),
            alpha_source: style.alpha_source,
            y_errors: style.y_errors,
            x_errors: style.x_errors,
            error_config: style.error_config,
            inset_layout: None,
            group_id,
            resolved_radar_colors: None,
        };

        let auto_color_slot = if series.color.is_none() && series.color_source.is_none() {
            Some(if consume_palette_index {
                self.series_mgr.auto_color_index
            } else {
                self.series_mgr.auto_color_index.saturating_sub(1)
            })
        } else {
            None
        };
        self.series_mgr
            .push_with_auto_color_slot(series, auto_color_slot);
        if consume_palette_index {
            self.series_mgr.auto_color_index += 1;
        }
        self
    }

    /// Resolve a series' marker edge against the theme and its resolved fill.
    ///
    /// Returns `(colour, width_in_points)`; the renderer scales the width to
    /// device pixels, so a marker rim is DPI-invariant. `None` means the series
    /// asked for bare markers (`ScatterConfig::show_edge(false)` or a zero edge
    /// width), or carries no marker edge configuration at all.
    pub(super) fn resolved_marker_edge(
        &self,
        series: &PlotSeries,
        fill: Color,
    ) -> Option<(Color, f32)> {
        series
            .marker_edge
            .and_then(|edge| edge.resolve(&self.display.theme, fill))
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
        let color = series.color_with_alpha(Color::from_rgb(0, 0, 0));
        let line_width = self.dpi_scaled_line_width(series.line_width.unwrap_or(2.0));
        let line_style = series.line_style.clone().unwrap_or(LineStyle::Solid);
        let clip_rect = clip_rect_from_plot_area(plot_area);

        let plan = match (&series.series_type, resolved) {
            (SeriesType::Line { .. }, ResolvedSeries::Line { x, y }) => {
                // One sub-path per contiguous run of representable samples: a
                // sample the axis cannot place (non-finite anywhere, or
                // non-positive on a log axis) breaks the line instead of being
                // joined across, which would draw a segment the user never
                // supplied.
                let subpaths = project_xy_subpaths(
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
                let mut marker_points: Vec<Point2f> = Vec::new();

                for mut points in subpaths {
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

                    if series.marker_style.is_some() {
                        marker_points.extend_from_slice(points.as_ref());
                    }

                    raster_plan.push_polyline(
                        points,
                        color,
                        line_width,
                        line_style.clone(),
                        clip_rect,
                    );
                }

                if let Some(marker_style) = series.marker_style {
                    let marker_size = self.dpi_scaled_line_width(series.marker_size.unwrap_or(8.0));
                    let marker_edge = self.resolved_marker_edge(series, color);
                    raster_plan.push_markers(
                        marker_points.into(),
                        marker_size,
                        marker_style,
                        color,
                        marker_edge,
                        clip_rect,
                    );
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
                let marker_edge = self.resolved_marker_edge(series, color);
                let mut raster_plan = SeriesRasterPlan::default();
                raster_plan.push_markers(
                    points,
                    marker_size,
                    marker_style,
                    color,
                    marker_edge,
                    clip_rect,
                );
                Some(raster_plan)
            }
            (SeriesType::Heatmap { data }, ResolvedSeries::Other(_)) => {
                let heatmap_plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                RectGridBatch::from_heatmap_data(
                    data,
                    heatmap_plot_area,
                    data.config.alpha * series.alpha.unwrap_or(1.0),
                )
                .map(|rect_grid| {
                    let mut raster_plan = SeriesRasterPlan::default();
                    raster_plan.push_rect_grid(rect_grid);
                    raster_plan
                })
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
                        &self.layout.x_scale,
                        &self.layout.y_scale,
                    )?;
                }
            }
            (SeriesType::Heatmap { data }, ResolvedSeries::Other(_)) => {
                let heatmap_plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                let render_scale = self.render_scale();
                let min_annotation_font_px = render_scale.points_to_pixels(8.0);
                let max_annotation_font_px = render_scale.points_to_pixels(20.0);
                for (row_idx, row) in data.values.iter().enumerate() {
                    for (col_idx, &value) in row.iter().enumerate() {
                        if !data.config.annotate || data.should_mask_value(value) {
                            continue;
                        }

                        let alpha = data.config.alpha * series.alpha.unwrap_or(1.0);
                        let cell_color = if alpha < 1.0 {
                            data.get_color(value).with_alpha(alpha)
                        } else {
                            data.get_color(value)
                        };

                        let (cell_x, cell_y, cell_width, cell_height) =
                            data.cell_screen_rect(&heatmap_plot_area, row_idx, col_idx);
                        let text = data.format_annotation(value);
                        let text_color = data.get_text_color(cell_color);
                        let text_x = cell_x + cell_width / 2.0;
                        let font_size = (cell_height * 0.3)
                            .clamp(min_annotation_font_px, max_annotation_font_px);
                        let text_y = cell_y + cell_height / 2.0 + font_size / 3.0;
                        renderer
                            .draw_text_centered(&text, text_x, text_y, font_size, text_color)?;
                    }
                }

                if let Some(request) = Self::heatmap_colorbar_request(data) {
                    let (x, y, width, height) = self.colorbar_rect(plot_area);
                    crate::render::colorbar::draw_colorbar(
                        renderer,
                        &request.spec_at(x, y, width, height, self.display.theme.foreground),
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
        alpha: f32,
        line_width: Option<f32>,
    ) -> Result<()> {
        if data.arrows.is_empty() {
            return Ok(());
        }

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
            .points_to_pixels(line_width.unwrap_or(data.config.width));

        for arrow in &data.arrows {
            let arrow_color = cmap
                .as_ref()
                .map(|colormap| {
                    colormap
                        .sample((arrow.magnitude - min_mag) / mag_range)
                        .with_alpha(alpha)
                })
                .unwrap_or(base_color);
            // An arrow is one shape built from several samples: if any vertex
            // has no position on these axes, the whole arrow is dropped rather
            // than drawn with a garbage vertex.
            let Some((sx1, sy1)) = crate::render::skia::try_map_data_to_pixels_scaled(
                arrow.start.0,
                arrow.start.1,
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
            let Some((sx2, sy2)) = crate::render::skia::try_map_data_to_pixels_scaled(
                arrow.end.0,
                arrow.end.1,
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
            let head: Option<Vec<(f32, f32)>> = arrow
                .head
                .iter()
                .map(|&(x, y)| {
                    crate::render::skia::try_map_data_to_pixels_scaled(
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
            let Some(head) = head else {
                continue;
            };
            renderer.draw_line(
                sx1,
                sy1,
                sx2,
                sy2,
                arrow_color,
                arrow_width,
                LineStyle::Solid,
            )?;

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
        let base_color = series.color.unwrap_or(Color::from_rgb(0, 0, 0));
        let alpha = series.alpha.unwrap_or(1.0);
        let color = series.color_with_alpha(Color::from_rgb(0, 0, 0)); // Default black
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
            (SeriesType::Bar { config, .. }, ResolvedSeries::Bar { values, .. }) => {
                // The edge is configured in points, so it survives a DPI change.
                // `edge_color: None` derives it by darkening the fill.
                let edge = config.resolved_edge(&self.display.theme, color);

                for (i, &value) in values.iter().enumerate() {
                    let (bx, by, bw, bh) = bar_pixel_rect(
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
                    renderer.draw_rectangle_styled_clipped(
                        bx,
                        by,
                        bw,
                        bh,
                        Some(color),
                        edge,
                        clip_rect,
                    )?;
                }
            }
            (SeriesType::Histogram { .. }, ResolvedSeries::Histogram { data: hist_data }) => {
                // Histogram bars are adjacent, so the bin boundaries only read as
                // boundaries if the bars carry an edge. Resolve it from the series
                // data (the same source the standalone `HistogramData` renderer
                // uses) instead of relying on a primitive-level implicit border.
                let edge = hist_data.resolved_edge(&self.display.theme, color);

                // Render histogram bars
                for (i, &count) in hist_data.counts.iter().enumerate() {
                    if count > 0.0 {
                        let (bx, by, bw, bh) = histogram_bar_pixel_rect(
                            hist_data.bin_edges[i],
                            hist_data.bin_edges[i + 1],
                            count,
                            plot_area,
                            x_min,
                            x_max,
                            y_min,
                            y_max,
                            &self.layout.x_scale,
                            &self.layout.y_scale,
                        );

                        renderer.draw_rectangle_styled_clipped(
                            bx,
                            by,
                            bw,
                            bh,
                            Some(color),
                            edge,
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

                // Every pixel below comes from the shared projection, so the
                // raster, SVG and parallel backends cannot place the same box
                // differently — and the value axis follows `yscale`.
                let BoxPlotPixels {
                    x_center: x_center_px,
                    box_left,
                    box_right,
                    cap_half_width,
                    q1_y,
                    median_y,
                    q3_y,
                    lower_whisker_y,
                    upper_whisker_y,
                } = BoxPlotPixels::new(
                    &box_data,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.y_scale,
                );

                // Draw the box (IQR) - ensure positive dimensions
                let box_width = box_right - box_left;
                let box_height = (q1_y - q3_y).abs(); // Ensure positive height
                let box_top = q3_y.min(q1_y); // Use the smaller y value as top

                // Validate dimensions before drawing
                let edge_color = box_data.edge_color.unwrap_or(color);
                let whisker_width_px = box_data
                    .whisker_width
                    .map(|w| self.render_scale().points_to_pixels(w))
                    .unwrap_or(line_width);
                let median_width_px = box_data
                    .median_width
                    .map(|w| self.render_scale().points_to_pixels(w))
                    .unwrap_or(line_width * 1.5);

                if box_width > 0.0
                    && box_height > 0.0
                    && box_width.is_finite()
                    && box_height.is_finite()
                {
                    renderer.draw_rectangle_styled_clipped(
                        box_left,
                        box_top,
                        box_width,
                        box_height,
                        Some(color.with_alpha(box_data.fill_alpha * alpha)),
                        Some((edge_color, box_data.edge_width)),
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
                        edge_color,
                        median_width_px,
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
                        edge_color,
                        whisker_width_px,
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
                        edge_color,
                        whisker_width_px,
                        line_style.clone(),
                        clip_rect,
                    )?;
                }

                // Draw whisker caps - validate coordinates
                if x_center_px.is_finite()
                    && lower_whisker_y.is_finite()
                    && cap_half_width.is_finite()
                {
                    renderer.draw_line_clipped(
                        x_center_px - cap_half_width,
                        lower_whisker_y,
                        x_center_px + cap_half_width,
                        lower_whisker_y,
                        edge_color,
                        whisker_width_px,
                        line_style.clone(),
                        clip_rect,
                    )?;
                }

                if x_center_px.is_finite()
                    && upper_whisker_y.is_finite()
                    && cap_half_width.is_finite()
                {
                    renderer.draw_line_clipped(
                        x_center_px - cap_half_width,
                        upper_whisker_y,
                        x_center_px + cap_half_width,
                        upper_whisker_y,
                        edge_color,
                        whisker_width_px,
                        line_style.clone(),
                        clip_rect,
                    )?;
                }

                // Draw outliers - validate coordinates
                let outlier_marker_size = self.render_scale().points_to_pixels(box_data.flier_size);
                let outliers: &[f64] = if box_data.show_outliers {
                    &box_data.outliers
                } else {
                    &[]
                };
                for &outlier in outliers {
                    let outlier_y =
                        box_plot_value_y(outlier, plot_area, y_min, y_max, &self.layout.y_scale);
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
                let heatmap_plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                data.draw_cells_batch(renderer, &heatmap_plot_area, data.config.alpha * alpha)?;
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
                let marker_edge = self.resolved_marker_edge(series, color);

                for (&x_value, &y_value) in x.iter().zip(y.iter()) {
                    // `is_finite` was only half the rule: a zero or negative
                    // sample is finite but has no position on a log axis, and
                    // would have been drawn on the spine.
                    if let Some((px, py)) = crate::render::skia::try_map_data_to_pixels_scaled(
                        x_value,
                        y_value,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        plot_area,
                        &self.layout.x_scale,
                        &self.layout.y_scale,
                    ) {
                        renderer.draw_marker_styled_clipped(
                            px,
                            py,
                            marker_size,
                            marker_style,
                            color,
                            marker_edge,
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
                    &self.layout.x_scale,
                    &self.layout.y_scale,
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
                let marker_edge = self.resolved_marker_edge(series, color);

                for (&x_value, &y_value) in x.iter().zip(y.iter()) {
                    // `is_finite` was only half the rule: a zero or negative
                    // sample is finite but has no position on a log axis, and
                    // would have been drawn on the spine.
                    if let Some((px, py)) = crate::render::skia::try_map_data_to_pixels_scaled(
                        x_value,
                        y_value,
                        x_min,
                        x_max,
                        y_min,
                        y_max,
                        plot_area,
                        &self.layout.x_scale,
                        &self.layout.y_scale,
                    ) {
                        renderer.draw_marker_styled_clipped(
                            px,
                            py,
                            marker_size,
                            marker_style,
                            color,
                            marker_edge,
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
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                )?;
            }
            (SeriesType::Kde { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render KDE
                let plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                data.render_styled(
                    renderer,
                    &plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.line_width,
                )?;
            }
            (SeriesType::Ecdf { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render ECDF
                let plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                data.render_styled(
                    renderer,
                    &plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.line_width,
                )?;
            }
            (SeriesType::Violin { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Violin
                let plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                data.render_styled(
                    renderer,
                    &plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.line_width,
                )?;
            }
            (SeriesType::Boxen { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Boxen
                let plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                data.render_styled(
                    renderer,
                    &plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.line_width,
                )?;
            }
            (SeriesType::Quiver { data }, ResolvedSeries::Other(_)) => {
                self.render_quiver_series_scaled(
                    renderer,
                    data,
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    color,
                    alpha,
                    series.line_width,
                )?;
            }
            (SeriesType::Contour { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Contour
                let contour_plot_area = plot_area_from_rect(
                    plot_area,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                    &self.layout.x_scale,
                    &self.layout.y_scale,
                );
                data.render_styled(
                    renderer,
                    &contour_plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.line_width,
                )?;

                if let Some(request) = Self::contour_colorbar_request(data) {
                    let (x, y, width, height) = self.colorbar_rect(plot_area);
                    crate::render::colorbar::draw_colorbar(
                        renderer,
                        &request.spec_at(x, y, width, height, self.display.theme.foreground),
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
                data.render_styled(
                    renderer,
                    &pie_plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.line_width,
                )?;
            }
            (SeriesType::Radar { data }, ResolvedSeries::Other(_)) => {
                // Use PlotRender trait to render Radar with 1:1 aspect ratio
                // and extra top padding for title clearance
                let radar_plot_area = Self::radar_plot_area(plot_area, x_min, x_max, y_min, y_max);
                let mut radar_theme = self.display.theme.clone();
                if let Some(colors) = &series.resolved_radar_colors {
                    radar_theme.color_palette = colors.to_vec();
                }
                data.render_styled_with_grid(
                    renderer,
                    &radar_plot_area,
                    &radar_theme,
                    base_color,
                    alpha,
                    series.line_width,
                    Some(&self.layout.grid_style),
                )?;
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
                data.render_styled(
                    renderer,
                    &polar_plot_area,
                    &self.display.theme,
                    base_color,
                    alpha,
                    series.line_width,
                )?;
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
        let color = series.color_with_alpha(Color::from_rgb(0, 0, 0));
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
                    let marker_edge = self.resolved_marker_edge(series, color);
                    for &(px, py) in &points {
                        renderer.draw_marker_styled_clipped(
                            px,
                            py,
                            marker_size,
                            marker_style,
                            color,
                            marker_edge,
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
                let marker_edge = self.resolved_marker_edge(series, color);

                // Draw markers at transformed coordinates
                for (&px, &py) in x_transformed.iter().zip(y_transformed.iter()) {
                    renderer.draw_marker_styled_clipped(
                        px,
                        py,
                        marker_size,
                        marker_style,
                        color,
                        marker_edge,
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
                SeriesType::Bar {
                    categories, values, ..
                } => {
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

    pub(super) fn validate_resolved_series(
        &self,
        series_list: &[ResolvedSeries<'_>],
    ) -> Result<()> {
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
                    let is_streaming = self.series_mgr.series.get(idx).is_some_and(|series| {
                        matches!(
                            &series.series_type,
                            SeriesType::Line { x_data, y_data }
                                | SeriesType::Scatter { x_data, y_data }
                                if matches!(x_data, PlotData::Streaming(_))
                                    && matches!(y_data, PlotData::Streaming(_))
                        )
                    });
                    if x.is_empty() && !is_streaming {
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

    /// The axis scales a series' geometry can honour, plus the plot-type name
    /// to quote when refusing one.
    ///
    /// Returns `(plot type, x support, y support)`.
    ///
    /// The match is deliberately exhaustive with no wildcard arm: a new
    /// `SeriesType` variant does not compile until it declares its support
    /// here, so a new plot type cannot silently inherit "draw linear geometry
    /// under whatever axis the user configured". The rule is the one documented
    /// on [`AxisScaleSupport`]: `Scaled` exactly when the geometry is projected
    /// through the axis scale (`map_data_to_pixels_scaled`, or `PlotArea`,
    /// which performs the same transform), `Unsupported` for geometry placed at
    /// an ordinal slot or in a synthetic cell rather than at a data value, and
    /// `Independent` for plot types that draw in their own coordinate system.
    pub(super) fn series_axis_scale_support(
        series: &SeriesType,
    ) -> (&'static str, AxisScaleSupport, AxisScaleSupport) {
        // Category axis: positions are indices, so there is no quantity to
        // take a logarithm of.
        const ORDINAL: AxisScaleSupport = AxisScaleSupport::Unsupported(
            "its categories sit at ordinal positions, which carry no quantitative spacing",
        );
        // A single-distribution plot centred in the synthetic 0..1 axis the
        // bounds calculation gives it.
        const SYNTHETIC_SLOT: AxisScaleSupport = AxisScaleSupport::Unsupported(
            "it occupies a synthetic slot on this axis rather than a data position",
        );
        const SCALED: AxisScaleSupport = AxisScaleSupport::Scaled;
        const OWN_COORDS: AxisScaleSupport = AxisScaleSupport::Independent;

        // A distribution plot puts its category on one axis and its values on
        // the other; which is which follows the series' own orientation.
        let across_and_along = |vertical: bool| {
            if vertical {
                (SYNTHETIC_SLOT, SCALED)
            } else {
                (SCALED, SYNTHETIC_SLOT)
            }
        };

        match series {
            SeriesType::Line { .. } => ("line", SCALED, SCALED),
            SeriesType::Scatter { .. } => ("scatter", SCALED, SCALED),
            SeriesType::Bar { .. } => ("bar", ORDINAL, SCALED),
            SeriesType::ErrorBars { .. } => ("errorbar", SCALED, SCALED),
            SeriesType::ErrorBarsXY { .. } => ("errorbar", SCALED, SCALED),
            SeriesType::Histogram { .. } => ("histogram", SCALED, SCALED),
            SeriesType::BoxPlot { .. } => ("boxplot", SYNTHETIC_SLOT, SCALED),
            SeriesType::Quiver { .. } => ("quiver", SCALED, SCALED),
            SeriesType::Heatmap { .. } => ("heatmap", SCALED, SCALED),
            SeriesType::Kde { .. } => ("kde", SCALED, SCALED),
            SeriesType::Ecdf { .. } => ("ecdf", SCALED, SCALED),
            SeriesType::Contour { .. } => ("contour", SCALED, SCALED),
            SeriesType::Violin { data } => {
                let (x, y) = across_and_along(matches!(
                    data.config.orientation,
                    crate::plots::distribution::Orientation::Vertical
                ));
                ("violin", x, y)
            }
            SeriesType::Boxen { data } => {
                let (x, y) = across_and_along(matches!(
                    data.config.orient,
                    crate::plots::distribution::BoxenOrientation::Vertical
                ));
                ("boxen", x, y)
            }
            SeriesType::Pie { .. } => ("pie", OWN_COORDS, OWN_COORDS),
            SeriesType::Radar { .. } => ("radar", OWN_COORDS, OWN_COORDS),
            SeriesType::Polar { .. } => ("polar", OWN_COORDS, OWN_COORDS),
        }
    }

    /// Refuse a figure whose own geometry cannot honour the axis scale it was
    /// given.
    ///
    /// The axis line, its ticks and its tick labels are always drawn
    /// scale-aware. A series that lays its geometry out linearly underneath a
    /// log-labelled axis therefore produces a quantitatively wrong figure and
    /// says nothing about it — so it is refused here instead of drawn.
    ///
    /// The raster, parallel and SVG paths all reach this through
    /// `validate_runtime_environment`, so no backend can accept a
    /// combination another one rejects.
    pub(super) fn validate_axis_scales(&self) -> Result<()> {
        let x_scale = &self.layout.x_scale;
        let y_scale = &self.layout.y_scale;
        if matches!(x_scale, crate::axes::AxisScale::Linear)
            && matches!(y_scale, crate::axes::AxisScale::Linear)
        {
            return Ok(());
        }

        for series in &self.series_mgr.series {
            let (plot_type, x_support, y_support) =
                Self::series_axis_scale_support(&series.series_type);

            for (axis, setter, support, scale) in [
                ("x", "xscale", x_support, x_scale),
                ("y", "yscale", y_support, y_scale),
            ] {
                if support.accepts(scale) {
                    continue;
                }

                let reason = support
                    .rejection_reason()
                    .unwrap_or("this plot type cannot honour a non-linear scale");
                return Err(PlottingError::InvalidInput(format!(
                    "`{plot_type}` cannot be drawn on a non-linear {axis} axis: {reason}. \
                     Remove `.{setter}(..)`, or plot this data with a series type whose \
                     {axis} geometry follows the axis scale (line, scatter, histogram, errorbar)."
                )));
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
        self.validate_axis_scales()?;
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

    /// Where a colorbar sits relative to the plot area: `(x, y, width, height)`.
    ///
    /// The layout pass reserves this strip; stating it in one place is what
    /// keeps the raster and SVG backends drawing it in the same rectangle.
    pub(super) fn colorbar_rect(&self, plot_area: tiny_skia::Rect) -> (f32, f32, f32, f32) {
        let render_scale = self.render_scale();
        let margin = render_scale.logical_pixels_to_pixels(COLORBAR_MARGIN_PX);
        let width = render_scale.logical_pixels_to_pixels(COLORBAR_WIDTH_PX);
        (
            plot_area.right() + margin,
            plot_area.y(),
            width,
            plot_area.height(),
        )
    }

    /// The colorbar a heatmap asks for, or `None` when it has one turned off.
    pub(super) fn heatmap_colorbar_request(
        data: &crate::plots::HeatmapData,
    ) -> Option<crate::render::colorbar::ColorbarRequest> {
        data.config
            .colorbar
            .then(|| crate::render::colorbar::ColorbarRequest {
                colormap: data.config.colormap.clone(),
                vmin: data.vmin,
                vmax: data.vmax,
                value_scale: data.config.value_scale,
                label: data.config.colorbar_label.clone(),
                tick_font_size: data.config.colorbar_tick_font_size,
                label_font_size: data.config.colorbar_label_font_size,
                show_log_subticks: data.config.colorbar_log_subticks,
            })
    }

    /// The colorbar a contour asks for, or `None` when it has one turned off.
    ///
    /// A contour's range is the span of its levels, and its ramp comes from the
    /// `cmap` name; both are resolved here so the two backends cannot label the
    /// same figure differently.
    pub(super) fn contour_colorbar_request(
        data: &crate::plots::ContourPlotData,
    ) -> Option<crate::render::colorbar::ColorbarRequest> {
        if !data.config.colorbar {
            return None;
        }
        let (vmin, vmax) = match (data.levels.first(), data.levels.last()) {
            (Some(&first), Some(&last)) => (first, last),
            _ => (0.0, 1.0),
        };
        Some(crate::render::colorbar::ColorbarRequest {
            colormap: crate::render::ColorMap::by_name(&data.config.cmap)
                .unwrap_or_else(crate::render::ColorMap::viridis),
            vmin,
            vmax,
            value_scale: crate::axes::AxisScale::Linear,
            label: data.config.colorbar_label.clone(),
            tick_font_size: data.config.colorbar_tick_font_size,
            label_font_size: data.config.colorbar_label_font_size,
            show_log_subticks: false,
        })
    }

    pub(super) fn validate_runtime_inputs(&self) -> Result<()> {
        self.validate_runtime_inputs_for_series(&self.series_mgr.series)
    }
}

/// Pixel y of the value-axis baseline a bar or histogram bin is drawn from.
///
/// Normally this is `0.0` projected through `y_scale`. A log axis has no
/// position for zero, and since a previous batch made that projection return
/// `NaN` rather than silently pretending zero sits on the spine, projecting it
/// blindly would make every bar's rectangle `NaN`. The bar instead bottoms out
/// on the axis floor, which is what the axis actually shows and what matplotlib
/// draws.
///
/// One helper so that the bar, histogram and parallel paths cannot each pick a
/// different answer for "where is zero on a log axis?". It is the rect-shaped
/// spelling of [`crate::plots::PlotArea::fill_baseline_y`], which the KDE and
/// area fills use, so bars and fills bottom out in the same place.
pub(super) fn value_axis_baseline_y(
    plot_area: tiny_skia::Rect,
    y_min: f64,
    y_max: f64,
    y_scale: &crate::axes::AxisScale,
) -> f32 {
    plot_area_from_rect(
        plot_area,
        0.0,
        1.0,
        y_min,
        y_max,
        &crate::axes::AxisScale::Linear,
        y_scale,
    )
    .fill_baseline_y()
}

/// Pixel rectangle `(x, y, width, height)` for one bar of a categorical series.
///
/// Categories sit one data unit apart, so a bar's width is `width_fraction` of a
/// unit measured through the same x mapping that places the bar centres, and its
/// body spans from the value to the zero baseline.
///
/// The value axis is projected through `y_scale`, so a bar chart on a log y axis
/// is drawn where its log-labelled ticks say it is. The category axis is always
/// linear: category positions are ordinals, and
/// `Plot::series_axis_scale_support` refuses a non-linear scale on it rather
/// than inventing a spacing for it.
///
/// The baseline is `0.0` mapped through the same projection, so on a log axis —
/// where zero has no position — the bar bottoms out at the axis floor, matching
/// the parallel backend and matplotlib.
///
/// Both the raster backend and the SVG backend call this, so the same bar chart
/// cannot land in a different place depending on the output format.
pub(super) fn bar_pixel_rect(
    index: usize,
    value: f64,
    width_fraction: f32,
    plot_area: tiny_skia::Rect,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    y_scale: &crate::axes::AxisScale,
) -> (f32, f32, f32, f32) {
    let data_range = (x_max - x_min) as f32;
    let bar_width = width_fraction * (plot_area.width() / data_range);
    let x = index as f64;
    let (px, py) = crate::render::skia::map_data_to_pixels_scaled(
        x,
        value,
        x_min,
        x_max,
        y_min,
        y_max,
        plot_area,
        &crate::axes::AxisScale::Linear,
        y_scale,
    );
    let py_zero = value_axis_baseline_y(plot_area, y_min, y_max, y_scale);
    (
        px - bar_width / 2.0,
        py.min(py_zero),
        bar_width,
        (py - py_zero).abs(),
    )
}

/// Pixel rectangle `(x, y, width, height)` for one histogram bin.
///
/// The bin is placed from its two mapped edges rather than from a mapped centre
/// plus half a width: on a non-linear x axis a bin is not symmetric about its
/// centre, and centre-based placement would leave gaps and overlaps between
/// adjacent bins.
///
/// Both axes are projected through their configured [`crate::axes::AxisScale`],
/// so the bars agree with the ticks drawn beside them.
///
/// The raster and SVG backends both call this, so a histogram cannot land in a
/// different place depending on the output format.
pub(super) fn histogram_bar_pixel_rect(
    bin_left: f64,
    bin_right: f64,
    count: f64,
    plot_area: tiny_skia::Rect,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    x_scale: &crate::axes::AxisScale,
    y_scale: &crate::axes::AxisScale,
) -> (f32, f32, f32, f32) {
    let (px_left, py) = crate::render::skia::map_data_to_pixels_scaled(
        bin_left, count, x_min, x_max, y_min, y_max, plot_area, x_scale, y_scale,
    );
    let (px_right, _) = crate::render::skia::map_data_to_pixels_scaled(
        bin_right, count, x_min, x_max, y_min, y_max, plot_area, x_scale, y_scale,
    );
    let py_zero = value_axis_baseline_y(plot_area, y_min, y_max, y_scale);
    (
        px_left.min(px_right),
        py.min(py_zero),
        (px_right - px_left).abs(),
        (py - py_zero).abs(),
    )
}

/// Every pixel coordinate one box plot is drawn from.
///
/// The raster, SVG and parallel backends each used to project these five
/// quantiles themselves, and the raster copy projected them linearly while the
/// figure's ticks were drawn scale-aware — so `.boxplot(&d).yscale(Log)` put the
/// box in the wrong place. Deriving them once here is what keeps the three
/// backends from drifting apart again.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct BoxPlotPixels {
    /// Horizontal centre of the box, in pixels.
    pub x_center: f32,
    /// Left edge of the box body, in pixels.
    pub box_left: f32,
    /// Right edge of the box body, in pixels.
    pub box_right: f32,
    /// Half-width of a whisker cap, in pixels.
    pub cap_half_width: f32,
    /// Q1 (bottom of the box) in pixels.
    pub q1_y: f32,
    /// Median line in pixels.
    pub median_y: f32,
    /// Q3 (top of the box) in pixels.
    pub q3_y: f32,
    /// Lower whisker end in pixels.
    pub lower_whisker_y: f32,
    /// Upper whisker end in pixels.
    pub upper_whisker_y: f32,
}

/// Where the box sits on the (ordinal) category axis.
///
/// The single-box API has no category to key off, so the box is centred in the
/// synthetic 0..1 axis the bounds calculation produces for it.
const BOX_PLOT_X_CENTER: f64 = 0.5;

impl BoxPlotPixels {
    /// Project a computed box plot into pixels.
    ///
    /// Every geometry constant comes from `box_data`, which `calculate_box_plot`
    /// resolved from the user's `BoxPlotConfig` — see the contract on
    /// `BoxPlotData`. The value axis is projected through `y_scale`; the
    /// category axis is always linear, because
    /// `Plot::series_axis_scale_support` refuses a non-linear scale on it.
    pub(super) fn new(
        box_data: &crate::plots::boxplot::BoxPlotData,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        y_scale: &crate::axes::AxisScale,
    ) -> Self {
        let (x_center, _) = crate::render::skia::map_data_to_pixels_scaled(
            BOX_PLOT_X_CENTER,
            0.0,
            x_min,
            x_max,
            y_min,
            y_max,
            plot_area,
            &crate::axes::AxisScale::Linear,
            y_scale,
        );
        let half_width = box_data.width_ratio * plot_area.width() * 0.5;

        Self {
            x_center,
            box_left: x_center - half_width,
            box_right: x_center + half_width,
            cap_half_width: half_width * box_data.cap_width,
            q1_y: box_plot_value_y(box_data.q1, plot_area, y_min, y_max, y_scale),
            median_y: box_plot_value_y(box_data.median, plot_area, y_min, y_max, y_scale),
            q3_y: box_plot_value_y(box_data.q3, plot_area, y_min, y_max, y_scale),
            lower_whisker_y: box_plot_value_y(box_data.min, plot_area, y_min, y_max, y_scale),
            upper_whisker_y: box_plot_value_y(box_data.max, plot_area, y_min, y_max, y_scale),
        }
    }
}

/// Project one box plot value onto the (scale-aware) value axis.
///
/// Outliers are projected with this too, so a flier cannot end up on a
/// different mapping from the whisker it sits beyond.
pub(super) fn box_plot_value_y(
    value: f64,
    plot_area: tiny_skia::Rect,
    y_min: f64,
    y_max: f64,
    y_scale: &crate::axes::AxisScale,
) -> f32 {
    crate::render::skia::map_data_to_pixels_scaled(
        0.0,
        value,
        0.0,
        1.0,
        y_min,
        y_max,
        plot_area,
        &crate::axes::AxisScale::Linear,
        y_scale,
    )
    .1
}

#[cfg(test)]
mod marker_edge_series_tests {
    use super::*;
    use crate::render::MarkerStyle;

    const FILL: Color = Color {
        r: 31,
        g: 119,
        b: 180,
        a: 255,
    };
    const RIM: Color = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };

    fn rim_pixels(image: &Image) -> usize {
        image
            .pixels
            .chunks_exact(4)
            .filter(|px| px[0] > 200 && px[1] < 120 && px[2] < 120)
            .count()
    }

    fn xy() -> (Vec<f64>, Vec<f64>) {
        let x: Vec<f64> = (0..8).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..8).map(|i| (i % 3) as f64).collect();
        (x, y)
    }

    fn exact_fill_pixels(image: &Image) -> usize {
        image
            .pixels
            .chunks_exact(4)
            .filter(|px| px[0] == FILL.r && px[1] == FILL.g && px[2] == FILL.b)
            .count()
    }

    fn square_scatter_plot(
        size: f32,
    ) -> impl Fn() -> crate::core::plot::PlotBuilder<crate::plots::basic::ScatterConfig> {
        let x: Vec<f64> = (0..8).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..8).map(|i| (i % 3) as f64).collect();
        move || {
            Plot::new()
                .size_px(240, 180)
                .ticks(false)
                .grid(false)
                .scatter(&x, &y)
                .marker(MarkerStyle::Square)
                .marker_size(size)
                .color(FILL)
        }
    }

    #[test]
    fn test_default_scatter_marker_is_exactly_its_series_colour() {
        // A contrasting rim cannot be the default: it is stroked over the
        // marker's own boundary, so it darkens every neighbour it overlaps and
        // swallows markers of a few points whole. Default = fill and nothing
        // else, matching matplotlib's `edgecolors='face'`.
        let plot = square_scatter_plot(12.0);

        let default = plot().render().expect("default scatter should render");
        let bare = plot()
            .show_edge(false)
            .render()
            .expect("bare scatter should render");

        assert_eq!(
            default.pixels, bare.pixels,
            "a default scatter marker must render exactly as an edgeless one"
        );
        assert!(
            exact_fill_pixels(&default) > 0,
            "the marker must stay the exact requested colour, not a tint"
        );
    }

    #[test]
    fn test_opt_in_scatter_rim_replaces_boundary_fill_without_tinting_the_interior() {
        // Asking for a rim must stroke a real boundary and leave the interior
        // at the exact requested colour — never tint the whole marker.
        let plot = square_scatter_plot(12.0);

        let edged = plot()
            .show_edge(true)
            .render()
            .expect("edged scatter should render");
        let bare = plot().render().expect("default scatter should render");

        assert_ne!(
            edged.pixels, bare.pixels,
            "show_edge(true) must reach the canvas"
        );
        assert!(
            exact_fill_pixels(&edged) > 0,
            "the marker interior must stay the exact requested colour, not a tint"
        );
        assert!(
            exact_fill_pixels(&edged) < exact_fill_pixels(&bare),
            "the rim must replace fill pixels at the marker boundary"
        );
    }

    #[test]
    fn test_small_default_markers_read_as_their_series_colour() {
        // A 0.8pt rim on a 2pt marker is most of the marker, so a defaulted-on
        // rim made small scatter render visibly darker than its own palette
        // colour and than its own legend key.
        for size in [2.0_f32, 3.0, 4.0] {
            let image = square_scatter_plot(size)()
                .render()
                .unwrap_or_else(|_| panic!("scatter at marker_size {size} should render"));

            let darker_than_fill = image
                .pixels
                .chunks_exact(4)
                .filter(|px| px[2] < FILL.b && px[2] > 0 && px[1] < FILL.g)
                .count();

            assert!(
                exact_fill_pixels(&image) > darker_than_fill,
                "a {size}pt marker must read as its series colour, \
                 got {} exact-fill vs {darker_than_fill} darker pixels",
                exact_fill_pixels(&image)
            );
        }
    }

    #[test]
    fn test_line_markers_carry_the_requested_rim() {
        // `.line().marker(..)` had no way to ask for a marker edge at all, so
        // the renderer's marker-edge hop was unreachable from the public API.
        let (x, y) = xy();
        let plot = || {
            Plot::new()
                .size_px(240, 180)
                .ticks(false)
                .grid(false)
                .line(&x, &y)
                .marker(MarkerStyle::Circle)
                .marker_size(12.0)
                .color(FILL)
        };

        let rimmed = plot()
            .marker_edge_color(RIM)
            .marker_edge_width(2.0)
            .render()
            .expect("rimmed line markers should render");
        let bare = plot()
            .show_marker_edge(false)
            .render()
            .expect("bare line markers should render");

        assert!(
            rim_pixels(&rimmed) > 0,
            "an explicit line-marker edge colour must reach the canvas"
        );
        assert_eq!(
            rim_pixels(&bare),
            0,
            "show_marker_edge(false) must reach the renderer as 'no edge'"
        );
    }

    #[test]
    fn test_scatter_legend_key_carries_the_same_rim_as_its_markers() {
        // A legend key that does not carry the rim its markers carry is a key
        // for a different series.
        use crate::core::legend::LegendItemType;

        let (x, y) = xy();
        let plot = Plot::new()
            .scatter(&x, &y)
            .label("points")
            .marker(MarkerStyle::Square)
            .color(FILL)
            .edge_color(RIM)
            .edge_width(2.0)
            .into_plot();

        let series = plot.series_mgr.series().first().expect("one series");
        let item = series
            .to_legend_item(FILL, &Theme::default())
            .expect("a labelled series produces a legend item");

        match item.item_type {
            LegendItemType::Scatter { edge, .. } => assert_eq!(
                edge,
                Some((RIM, 2.0)),
                "the key must carry the rim the markers are stroked with"
            ),
            other => panic!("expected a scatter key, got {other:?}"),
        }
    }

    #[test]
    fn test_line_and_scatter_markers_resolve_the_same_default_edge() {
        assert_eq!(
            crate::plots::basic::LineConfig::default().resolved_marker_edge_spec(),
            crate::plots::basic::ScatterConfig::default().resolved_edge_spec(),
            "the same marker must not look different for coming from .line() vs .scatter()"
        );
    }
}

#[cfg(test)]
mod bar_edge_tests {
    use super::*;

    const FILL: Color = Color {
        r: 31,
        g: 119,
        b: 180,
        a: 255,
    };
    const EDGE: Color = Color {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };

    /// Pixels that are more red than blue: the fill, the background and the
    /// spines all fail this, so it counts only the explicit red bar edge.
    fn edge_pixels(image: &Image) -> usize {
        image
            .pixels
            .chunks_exact(4)
            .filter(|px| px[0] > 200 && px[1] < 120 && px[2] < 120)
            .count()
    }

    fn bar_plot(dpi: u32) -> PlotBuilder<crate::plots::basic::BarConfig> {
        let values = vec![1.0, 2.0];
        Plot::new()
            .size_px(240, 180)
            .dpi(dpi)
            .ticks(false)
            .grid(false)
            .bar(&["a", "b"], &values)
            .color(FILL)
    }

    #[test]
    fn test_bar_renders_an_explicit_edge_colour() {
        let image = bar_plot(100)
            .edge_color(EDGE)
            .edge_width(3.0)
            .render()
            .expect("bar render should succeed");

        assert!(
            edge_pixels(&image) > 0,
            "an explicitly configured bar edge colour must reach the canvas"
        );
    }

    #[test]
    fn test_bar_edge_width_zero_draws_no_edge() {
        let image = bar_plot(100)
            .edge_color(EDGE)
            .edge_width(0.0)
            .render()
            .expect("bar render should succeed");

        assert_eq!(
            edge_pixels(&image),
            0,
            "edge_width(0.0) must suppress the edge even when a colour is set"
        );
    }

    #[test]
    fn test_default_bar_renders_a_derived_edge() {
        let with_edge = bar_plot(100)
            .render()
            .expect("default bar render should succeed");
        let without_edge = bar_plot(100)
            .edge_width(0.0)
            .render()
            .expect("edgeless bar render should succeed");

        assert_ne!(
            with_edge.pixels, without_edge.pixels,
            "a default bar must draw the derived edge BarConfig::default() configures"
        );
    }

    #[test]
    fn test_zero_valued_bar_leaves_no_mark() {
        // A zero-height bar has no area, so its edge must not survive as a
        // hairline along the baseline: that would mark a datum that is not
        // there, and it would differ from the "no bar at all" case.
        let drawn = Plot::new()
            .size_px(240, 180)
            .ticks(false)
            .grid(false)
            .bar(&["zero", "one"], &[0.0f64, 1.0])
            .color(FILL)
            .edge_color(EDGE)
            .edge_width(3.0)
            .render()
            .expect("zero-valued bar render should succeed");

        // Only the second bar has a body, so every red pixel must sit in the
        // right-hand half of the plot.
        let width = drawn.width as usize;
        let left_half_edge = drawn
            .pixels
            .chunks_exact(4)
            .enumerate()
            .filter(|(index, px)| {
                index % width < width / 2 && px[0] > 200 && px[1] < 120 && px[2] < 120
            })
            .count();

        assert_eq!(
            left_half_edge, 0,
            "a zero-valued bar must not paint its edge along the baseline"
        );
        assert!(
            edge_pixels(&drawn) > 0,
            "the non-zero bar must still carry its edge"
        );
    }

    #[test]
    fn test_bar_width_fraction_reaches_the_geometry() {
        // `.bar_width(..)` used to be inert: both render paths hardcoded 0.8.
        let narrow = bar_pixel_rect(
            0,
            1.0,
            0.4,
            tiny_skia::Rect::from_xywh(0.0, 0.0, 200.0, 100.0).expect("valid plot area"),
            -0.5,
            1.5,
            0.0,
            1.0,
            &crate::axes::AxisScale::Linear,
        );
        let wide = bar_pixel_rect(
            0,
            1.0,
            0.8,
            tiny_skia::Rect::from_xywh(0.0, 0.0, 200.0, 100.0).expect("valid plot area"),
            -0.5,
            1.5,
            0.0,
            1.0,
            &crate::axes::AxisScale::Linear,
        );

        assert!(
            (wide.2 - narrow.2 * 2.0).abs() < 1e-3,
            "doubling the width fraction must double the bar: {} vs {}",
            narrow.2,
            wide.2
        );
        assert!(
            (narrow.0 + narrow.2 / 2.0 - (wide.0 + wide.2 / 2.0)).abs() < 1e-3,
            "the bar centre must not move when only the width changes"
        );
    }

    #[test]
    fn test_bar_edge_width_scales_with_dpi() {
        let low = bar_plot(100)
            .edge_color(EDGE)
            .edge_width(2.0)
            .render()
            .expect("bar render should succeed");
        let high = bar_plot(200)
            .edge_color(EDGE)
            .edge_width(2.0)
            .render()
            .expect("bar render should succeed");

        let low_count = edge_pixels(&low);
        let high_count = edge_pixels(&high);

        assert!(low_count > 0, "the low-DPI edge must be drawn at all");
        // Doubling the DPI doubles the bar outline's length; if the stroke width
        // were left in raw pixels the count would merely double with it.
        assert!(
            high_count > low_count * 3,
            "edge width must scale with DPI: {low_count}px at 100 dpi vs {high_count}px at 200 dpi"
        );
    }
}

#[cfg(test)]
mod axis_scale_geometry_tests {
    use super::*;
    use crate::axes::AxisScale;
    use crate::render::MarkerStyle;

    const SERIES: Color = Color {
        r: 220,
        g: 20,
        b: 20,
        a: 255,
    };

    fn area() -> tiny_skia::Rect {
        tiny_skia::Rect::from_xywh(20.0, 40.0, 200.0, 300.0).expect("valid plot area")
    }

    /// Screen y for `value` on a log axis spanning `y_min..y_max`, derived from
    /// the axis definition rather than from the mapping under test.
    fn log_screen_y(value: f64, y_min: f64, y_max: f64, plot_area: tiny_skia::Rect) -> f32 {
        let normalized = (value.log10() - y_min.log10()) / (y_max.log10() - y_min.log10());
        plot_area.top() + plot_area.height() * (1.0 - normalized as f32)
    }

    #[test]
    fn test_bar_body_follows_a_logarithmic_value_axis() {
        // The bug: bars were projected linearly while the ticks beside them
        // were projected in log space, so the bar top read off the wrong tick.
        let plot_area = area();
        let log = bar_pixel_rect(
            0,
            10.0,
            0.8,
            plot_area,
            -0.5,
            1.5,
            1.0,
            1000.0,
            &AxisScale::Log,
        );
        let linear = bar_pixel_rect(
            0,
            10.0,
            0.8,
            plot_area,
            -0.5,
            1.5,
            1.0,
            1000.0,
            &AxisScale::Linear,
        );

        // 10 is one decade into a three-decade axis, so the bar top sits a
        // third of the way up.
        let expected_top = log_screen_y(10.0, 1.0, 1000.0, plot_area);
        assert!(
            (log.1 - expected_top).abs() < 0.05,
            "bar top must sit at the log-mapped value: {} vs {expected_top}",
            log.1
        );
        // Zero has no position on a log axis, so the body bottoms out on the
        // axis floor — the same convention the parallel backend uses.
        assert!(
            (log.1 + log.3 - plot_area.bottom()).abs() < 0.05,
            "the bar must run down to the axis floor: {}",
            log.1 + log.3
        );
        assert!(
            (log.1 - linear.1).abs() > 50.0,
            "the log placement must actually differ from the linear one"
        );
        // Only the value axis is scaled; the categories keep their spacing.
        assert!((log.0 - linear.0).abs() < 1e-4);
        assert!((log.2 - linear.2).abs() < 1e-4);
    }

    #[test]
    fn test_histogram_bins_tile_without_gaps_on_a_logarithmic_x_axis() {
        // Centre-based placement (mapped centre ± half a width) only tiles on a
        // linear axis; on a log axis it leaves gaps and overlaps.
        let plot_area = area();
        let lower = histogram_bar_pixel_rect(
            1.0,
            10.0,
            5.0,
            plot_area,
            1.0,
            1000.0,
            0.0,
            10.0,
            &AxisScale::Log,
            &AxisScale::Linear,
        );
        let upper = histogram_bar_pixel_rect(
            10.0,
            100.0,
            3.0,
            plot_area,
            1.0,
            1000.0,
            0.0,
            10.0,
            &AxisScale::Log,
            &AxisScale::Linear,
        );

        assert!(
            (lower.0 + lower.2 - upper.0).abs() < 0.05,
            "adjacent bins must share an edge: {} vs {}",
            lower.0 + lower.2,
            upper.0
        );
        assert!(
            (lower.2 - upper.2).abs() < 0.05,
            "equal decades must occupy equal pixel widths on a log axis: {} vs {}",
            lower.2,
            upper.2
        );
        assert!(
            (lower.2 - plot_area.width() / 3.0).abs() < 0.05,
            "one decade of a three-decade axis must be a third of it: {}",
            lower.2
        );
    }

    #[test]
    fn test_histogram_bar_top_follows_a_logarithmic_count_axis() {
        let plot_area = area();
        let bar = histogram_bar_pixel_rect(
            1.0,
            2.0,
            10.0,
            plot_area,
            0.0,
            10.0,
            1.0,
            1000.0,
            &AxisScale::Linear,
            &AxisScale::Log,
        );

        let expected_top = log_screen_y(10.0, 1.0, 1000.0, plot_area);
        assert!(
            (bar.1 - expected_top).abs() < 0.05,
            "bar top must sit at the log-mapped count: {} vs {expected_top}",
            bar.1
        );
    }

    fn box_data() -> crate::plots::boxplot::BoxPlotData {
        let values: Vec<f64> = vec![1.0, 3.0, 10.0, 30.0, 100.0];
        crate::plots::boxplot::calculate_box_plot(
            &values,
            &crate::plots::boxplot::BoxPlotConfig::default(),
        )
        .expect("box plot statistics should compute")
    }

    #[test]
    fn test_boxplot_quantiles_sit_at_their_log_mapped_positions() {
        // `.boxplot(&d).yscale(Log)` used to draw a log-labelled axis with a
        // linearly-positioned box.
        let plot_area = area();
        let data = box_data();
        let log = BoxPlotPixels::new(&data, plot_area, 0.0, 1.0, 1.0, 1000.0, &AxisScale::Log);
        let linear =
            BoxPlotPixels::new(&data, plot_area, 0.0, 1.0, 1.0, 1000.0, &AxisScale::Linear);

        for (label, drawn, value) in [
            ("median", log.median_y, data.median),
            ("q1", log.q1_y, data.q1),
            ("q3", log.q3_y, data.q3),
            ("lower whisker", log.lower_whisker_y, data.min),
            ("upper whisker", log.upper_whisker_y, data.max),
        ] {
            let expected = log_screen_y(value, 1.0, 1000.0, plot_area);
            assert!(
                (drawn - expected).abs() < 0.05,
                "{label} must sit at its log-mapped position: {drawn} vs {expected}"
            );
        }

        assert!(
            (log.median_y - linear.median_y).abs() > 20.0,
            "the log placement must actually differ from the linear one"
        );
        // The category axis is untouched: only the value axis is scaled.
        assert!((log.x_center - linear.x_center).abs() < 1e-4);
        assert!((log.box_left - linear.box_left).abs() < 1e-4);
        assert!((log.box_right - linear.box_right).abs() < 1e-4);
    }

    #[test]
    fn test_boxplot_outliers_use_the_same_projection_as_its_whiskers() {
        let plot_area = area();
        let data = box_data();
        let pixels = BoxPlotPixels::new(&data, plot_area, 0.0, 1.0, 1.0, 1000.0, &AxisScale::Log);

        assert!(
            (box_plot_value_y(data.max, plot_area, 1.0, 1000.0, &AxisScale::Log)
                - pixels.upper_whisker_y)
                .abs()
                < 1e-4,
            "a flier must be projected exactly like the whisker it lies beyond"
        );
    }

    fn boxplot_image(scale: AxisScale) -> Image {
        let values = vec![1.0, 3.0, 10.0, 30.0, 100.0, 300.0, 900.0];
        Plot::new()
            .size_px(240, 320)
            .ticks(false)
            .grid(false)
            .boxplot(&values)
            .color(SERIES)
            .ylim(1.0, 1000.0)
            .yscale(scale)
            .render()
            .expect("box plot should render")
    }

    #[test]
    fn test_rendered_boxplot_moves_when_the_value_axis_becomes_logarithmic() {
        // Ticks, grid and the y limits are all pinned, so the only thing that
        // can differ between these two figures is the box geometry itself.
        // Before the fix they were pixel-identical.
        let linear = boxplot_image(AxisScale::Linear);
        let log = boxplot_image(AxisScale::Log);

        assert_ne!(
            linear.pixels, log.pixels,
            "`yscale(Log)` must reach the box geometry, not only the axis labels"
        );
    }

    /// Rows in `column` carrying series-coloured ink.
    ///
    /// Measured as "redder than it is grey", so partially covered
    /// anti-aliased pixels count too; the white background and the neutral
    /// spines and axes never do.
    fn inked_rows(image: &Image, column: usize) -> Vec<usize> {
        let width = image.width as usize;
        (0..image.height as usize)
            .filter(|row| {
                let index = (row * width + column) * 4;
                let px = &image.pixels[index..index + 4];
                i16::from(px[0]) - i16::from(px[1].max(px[2])) > 30
            })
            .collect()
    }

    #[test]
    fn test_error_bars_stay_attached_to_their_markers_on_a_logarithmic_axis() {
        // The markers were projected scale-aware and the whiskers linearly, so
        // on a log axis the error bar detached from the point it belonged to
        // and slid down towards the axis floor.
        let image = Plot::new()
            .size_px(240, 400)
            .ticks(false)
            .grid(false)
            .line(&[1.0, 2.0, 3.0], &[10.0, 10.0, 10.0])
            .color(SERIES)
            .marker(MarkerStyle::Circle)
            .marker_size(6.0)
            .with_yerr(&[5.0, 5.0, 5.0])
            .ylim(1.0, 1000.0)
            .yscale(AxisScale::Log)
            .render()
            .expect("line with y errors on a log axis should render");

        // An error-bar column carries far more series-coloured pixels than the
        // horizontal line does; the spines are not series-coloured.
        let (column, rows) = (0..image.width as usize)
            .map(|column| (column, inked_rows(&image, column)))
            .max_by_key(|(_, rows)| rows.len())
            .expect("the figure has at least one column");

        assert!(
            rows.len() > 10,
            "expected to find an error-bar column, found {} inked rows at column {column}",
            rows.len()
        );

        let largest_gap = rows
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .max()
            .unwrap_or(0);
        assert!(
            largest_gap <= 2,
            "the error bar must stay attached to its marker: {largest_gap}px gap in column {column}"
        );
    }

    #[test]
    fn test_a_sample_the_log_axis_cannot_place_draws_no_whisker_at_all() {
        // `y = 0` and `y = -5` have no position on a log axis. Their whisker
        // ends projected to NaN pixels, and `NaN.max(top).min(bottom)` yields
        // `top` — so the stem was pinned to the top of the frame and drawn all
        // the way down to the other end, a runaway line across 87% of the plot
        // height hanging off a point that was (correctly) not drawn.
        let image = Plot::new()
            .size_px(240, 400)
            .ticks(false)
            .grid(false)
            .line(
                &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
                &[10.0, 20.0, 0.0, 40.0, -5.0, 60.0],
            )
            .color(SERIES)
            .with_yerr(&[2.0; 6])
            .ylim(1.0, 200.0)
            .yscale(AxisScale::Log)
            .render()
            .expect("a log plot with unrepresentable samples should still render");

        // Each valid sample's whisker spans +-2 around its value, which at this
        // scale is a few dozen pixels; nothing may span a large fraction of the
        // plot. The bound is generous so it fails only on a runaway.
        let tallest = (0..image.width as usize)
            .map(|column| {
                let rows = inked_rows(&image, column);
                match (rows.first(), rows.last()) {
                    (Some(&first), Some(&last)) => last - first + 1,
                    _ => 0,
                }
            })
            .max()
            .expect("the figure has at least one column");

        assert!(
            tallest < image.height as usize / 3,
            "no whisker may run the height of the plot: tallest column spans {tallest}px \
             of {}px",
            image.height
        );
    }

    #[test]
    fn test_bar_chart_refuses_a_non_linear_category_axis() {
        // Categories are ordinals: there is no quantity to take a log of, so
        // the only honest answers are "reject" and "draw something wrong".
        let error = Plot::new()
            .bar(&["a", "b"], &[1.0, 2.0])
            .xscale(AxisScale::Log)
            .render()
            .expect_err("a log category axis must be refused, not drawn linearly");

        assert!(
            matches!(error, PlottingError::InvalidInput(_)),
            "expected InvalidInput, got {error:?}"
        );
        let message = error.to_string();
        assert!(message.contains("bar"), "{message}");
        assert!(message.contains("x axis"), "{message}");
        assert!(message.contains("xscale"), "{message}");
    }

    #[test]
    fn test_heatmap_cells_follow_a_logarithmic_axis() {
        // A heatmap draws through `PlotArea`, which projects through the
        // figure's axis scales. The boundary between two rows must therefore sit
        // where the log axis puts it, not where a linear one would.
        use crate::plots::heatmap::HeatmapConfig;

        let values = vec![vec![1.0, 1.0], vec![2.0, 2.0]];
        let config = HeatmapConfig::new()
            .extent(0.0, 1.0, 1.0, 100.0)
            .colorbar(false);

        // How many rows of the middle column the lower (bright) cell covers.
        let lower_cell_rows = |scale: AxisScale| {
            let image = Plot::new()
                .size_px(200, 300)
                .ticks(false)
                .grid(false)
                .heatmap_with(&values, config.clone())
                .yscale(scale)
                .render()
                .expect("a heatmap must render on any axis scale");
            let width = image.width as usize;
            let column = width / 2;
            // The two cells sit at the ends of viridis: the lower one is the
            // bright yellow end, which nothing else in the figure resembles.
            (0..image.height as usize)
                .filter(|row| {
                    let index = (row * width + column) * 4;
                    let px = &image.pixels[index..index + 4];
                    px[0] > 150 && px[1] > 150 && px[2] < 120
                })
                .count()
        };

        // The lower cell covers data y 1..50.5 of an extent of 1..100.
        // Linearly that is half the axis; logarithmically it is
        // log10(50.5 / 1) / log10(100 / 1) ≈ 85% of it.
        let linear = lower_cell_rows(AxisScale::Linear);
        let log = lower_cell_rows(AxisScale::Log);
        assert!(
            log > linear + 40,
            "a log y axis must stretch the lower heatmap cell: \
             linear covered {linear} rows, log covered {log}"
        );
    }

    #[test]
    fn test_scale_aware_series_still_accept_log_axes() {
        // The rejection must be narrow: everything that does project through
        // the axis scale keeps working.
        Plot::new()
            .size_px(240, 180)
            .line(&[1.0, 10.0, 100.0], &[1.0, 10.0, 100.0])
            .xscale(AxisScale::Log)
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-log line plot must still render");

        Plot::new()
            .size_px(240, 180)
            .scatter(&[1.0, 10.0, 100.0], &[1.0, 10.0, 100.0])
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-y scatter plot must still render");
    }

    #[test]
    fn test_plot_area_series_accept_log_axes() {
        // These all draw through `PlotArea`, which projects through the figure's
        // axis scales. They were refused outright while it mapped linearly; the
        // refusal has to be gone now that it does not.
        let samples: Vec<f64> = (0..120)
            .map(|index| 3.0 + 15.0 * ((index as f64 * 0.37).sin() * 0.5 + 0.5))
            .collect();

        Plot::new()
            .size_px(240, 180)
            .kde(&samples)
            .xscale(AxisScale::Log)
            .render()
            .expect("a log-x KDE must render");

        Plot::new()
            .size_px(240, 180)
            .ecdf(&samples)
            .xscale(AxisScale::Log)
            .render()
            .expect("a log-x ECDF must render");

        Plot::new()
            .size_px(240, 180)
            .violin(&samples)
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-y violin must render");

        Plot::new()
            .size_px(240, 180)
            .boxen(&samples)
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-y boxen must render");

        let x: Vec<f64> = (1..=12).map(f64::from).collect();
        let y: Vec<f64> = (1..=10).map(f64::from).collect();
        let z: Vec<f64> = y
            .iter()
            .flat_map(|&yy| x.iter().map(move |&xx| (xx * 0.2).sin() + (yy * 0.2).cos()))
            .collect();
        Plot::new()
            .size_px(240, 180)
            .contour(&x, &y, &z)
            .xscale(AxisScale::Log)
            .render()
            .expect("a log-x contour must render");
    }

    /// A bar or histogram fills down to zero, and a log axis has no position for
    /// zero. Folding the baseline into the auto-range therefore made the range
    /// itself invalid, and the default path could not be exercised at all: it
    /// failed with "Logarithmic scale requires positive values" before drawing.
    #[test]
    fn test_bar_and_histogram_autoscale_on_a_log_value_axis() {
        Plot::new()
            .size_px(240, 180)
            .bar(&["a", "b", "c"], &[1.0, 10.0, 100.0])
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-y bar chart must render without an explicit ylim");

        let samples: Vec<f64> = (0..400).map(|index| (index as f64 * 0.031).sin()).collect();
        Plot::new()
            .size_px(240, 180)
            .histogram(&samples)
            .bins(12)
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-y histogram must render without an explicit ylim");
    }

    /// Bars still have to bottom out on the axis floor and top out at their
    /// value: the decades between two bars must be the decades the axis shows.
    #[test]
    fn test_log_bar_tops_land_one_decade_apart() {
        let image = Plot::new()
            .size_px(240, 400)
            .ticks(false)
            .grid(false)
            .bar(&["a", "b", "c"], &[1.0, 10.0, 100.0])
            .color(SERIES)
            .ylim(0.5, 200.0)
            .yscale(AxisScale::Log)
            .render()
            .expect("a log-y bar chart must render");

        let tops: Vec<usize> = [0usize, 1, 2]
            .iter()
            .map(|slot| {
                // Sample the middle of each of the three category slots.
                let column = (image.width as usize * (2 * slot + 1)) / 6;
                *inked_rows(&image, column)
                    .first()
                    .unwrap_or_else(|| panic!("bar {slot} must put ink in column {column}"))
            })
            .collect();

        let first_gap = tops[0] as f64 - tops[1] as f64;
        let second_gap = tops[1] as f64 - tops[2] as f64;
        assert!(
            (first_gap - second_gap).abs() < 2.0,
            "each decade must be the same height on a log axis, got {first_gap} then {second_gap}"
        );
    }

    #[test]
    fn test_linear_figures_are_never_refused() {
        // Every plot type must stay renderable on the default axes, including
        // the ones that declare a non-linear scale unsupported.
        let values = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        Plot::new()
            .size_px(240, 180)
            .heatmap(&values)
            .render()
            .expect("a heatmap on linear axes must render");

        Plot::new()
            .size_px(240, 180)
            .bar(&["a", "b"], &[1.0, 2.0])
            .render()
            .expect("a bar chart on linear axes must render");
    }
}
