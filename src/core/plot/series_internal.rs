use super::*;

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

    /// Helper method to render a single series using normal (non-DataShader) rendering
    pub(super) fn render_series_normal(
        &self,
        series: &PlotSeries,
        renderer: &mut SkiaRenderer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
    ) -> Result<()> {
        let color = series.color.unwrap_or(Color::new(0, 0, 0)); // Default black
        let line_width = self.dpi_scaled_line_width(series.line_width.unwrap_or(2.0));
        let line_style = series.line_style.clone().unwrap_or(LineStyle::Solid);
        let clip_rect = (
            plot_area.x(),
            plot_area.y(),
            plot_area.width(),
            plot_area.height(),
        );

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

                // Draw attached error bars if present
                if series.y_errors.is_some() || series.x_errors.is_some() {
                    Self::render_attached_error_bars(
                        renderer,
                        &x_data,
                        &y_data,
                        series.y_errors.as_ref(),
                        series.x_errors.as_ref(),
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
            SeriesType::Scatter { x_data, y_data } => {
                let x_data = x_data.resolve(0.0);
                let y_data = y_data.resolve(0.0);
                let marker_size = self.dpi_scaled_line_width(series.marker_size.unwrap_or(10.0)); // DPI-scaled marker size
                let marker_style = series.marker_style.unwrap_or(MarkerStyle::Circle);

                for (&x, &y) in x_data.iter().zip(y_data.iter()) {
                    let (px, py) = crate::render::skia::map_data_to_pixels(
                        x, y, x_min, x_max, y_min, y_max, plot_area,
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

                // Draw attached error bars if present
                if series.y_errors.is_some() || series.x_errors.is_some() {
                    Self::render_attached_error_bars(
                        renderer,
                        &x_data,
                        &y_data,
                        series.y_errors.as_ref(),
                        series.x_errors.as_ref(),
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
            SeriesType::Bar { values, .. } => {
                let values = values.resolve(0.0);
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
            SeriesType::Histogram { data, config } => {
                let data = data.resolve(0.0);
                // Calculate histogram data
                let hist_data = crate::plots::histogram::calculate_histogram(&data, config)
                    .map_err(|e| {
                        PlottingError::RenderError(format!("Histogram calculation failed: {}", e))
                    })?;

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
            SeriesType::BoxPlot { data, config } => {
                let data = data.resolve(0.0);
                // Calculate box plot statistics
                let box_data =
                    crate::plots::boxplot::calculate_box_plot(&data, config).map_err(|e| {
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
                for &outlier in &box_data.outliers {
                    let (_, outlier_y) = crate::render::skia::map_data_to_pixels(
                        0.0, outlier, x_min, x_max, y_min, y_max, plot_area,
                    );
                    if x_center_px.is_finite() && outlier_y.is_finite() {
                        renderer.draw_marker_clipped(
                            x_center_px,
                            outlier_y,
                            4.0, // outlier marker size
                            MarkerStyle::Circle,
                            color,
                            clip_rect,
                        )?;
                    }
                }
            }
            SeriesType::Heatmap { data } => {
                // Calculate cell dimensions in pixel space
                let cell_width = plot_area.width() / data.n_cols as f32;
                let cell_height = plot_area.height() / data.n_rows as f32;

                // Render each cell as a filled rectangle
                for (row_idx, row) in data.values.iter().enumerate() {
                    for (col_idx, &value) in row.iter().enumerate() {
                        let cell_color = data.get_color(value);

                        // Apply alpha from config
                        let cell_color = if data.config.alpha < 1.0 {
                            Color::new_rgba(
                                cell_color.r,
                                cell_color.g,
                                cell_color.b,
                                (data.config.alpha * 255.0) as u8,
                            )
                        } else {
                            cell_color
                        };

                        // Calculate cell position (row 0 at top)
                        let cell_x = plot_area.x() + col_idx as f32 * cell_width;
                        let cell_y =
                            plot_area.y() + (data.n_rows - 1 - row_idx) as f32 * cell_height;

                        renderer.draw_rectangle(
                            cell_x,
                            cell_y,
                            cell_width,
                            cell_height,
                            cell_color,
                            true,
                        )?;

                        // Draw cell annotation if enabled
                        if data.config.annotate {
                            let text = format!("{:.2}", value);
                            let text_color = data.get_text_color(cell_color);
                            let text_x = cell_x + cell_width / 2.0;
                            let font_size = (cell_height * 0.3).clamp(8.0, 20.0);
                            // Center vertically: y position at cell center + half font size
                            let text_y = cell_y + cell_height / 2.0 + font_size / 3.0;
                            renderer
                                .draw_text_centered(&text, text_x, text_y, font_size, text_color)?;
                        }
                    }
                }

                // Draw colorbar if enabled
                if data.config.colorbar {
                    let colorbar_width = 20.0;
                    let colorbar_margin = 10.0;
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
                        data.config.colorbar_label.as_deref(),
                        color,
                        data.config.colorbar_tick_font_size,
                        Some(data.config.colorbar_label_font_size),
                    )?;
                }
            }
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => {
                let x_data = x_data.resolve(0.0);
                let y_data = y_data.resolve(0.0);
                let y_errors = y_errors.resolve(0.0);
                // Draw markers at data points
                let marker_size = self.dpi_scaled_line_width(series.marker_size.unwrap_or(8.0));
                let marker_style = series.marker_style.unwrap_or(MarkerStyle::Circle);

                for (&x, &y) in x_data.iter().zip(y_data.iter()) {
                    if x.is_finite() && y.is_finite() {
                        let (px, py) = crate::render::skia::map_data_to_pixels(
                            x, y, x_min, x_max, y_min, y_max, plot_area,
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
                let y_err_values = ErrorValues::symmetric(y_errors);
                Self::render_attached_error_bars(
                    renderer,
                    &x_data,
                    &y_data,
                    Some(&y_err_values),
                    None,
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
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => {
                let x_data = x_data.resolve(0.0);
                let y_data = y_data.resolve(0.0);
                let x_errors = x_errors.resolve(0.0);
                let y_errors = y_errors.resolve(0.0);
                // Draw markers at data points
                let marker_size = self.dpi_scaled_line_width(series.marker_size.unwrap_or(8.0));
                let marker_style = series.marker_style.unwrap_or(MarkerStyle::Circle);

                for (&x, &y) in x_data.iter().zip(y_data.iter()) {
                    if x.is_finite() && y.is_finite() {
                        let (px, py) = crate::render::skia::map_data_to_pixels(
                            x, y, x_min, x_max, y_min, y_max, plot_area,
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
                let x_err_values = ErrorValues::symmetric(x_errors);
                let y_err_values = ErrorValues::symmetric(y_errors);
                Self::render_attached_error_bars(
                    renderer,
                    &x_data,
                    &y_data,
                    Some(&y_err_values),
                    Some(&x_err_values),
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
            SeriesType::Kde { data } => {
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
            SeriesType::Ecdf { data } => {
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
            SeriesType::Violin { data } => {
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
            SeriesType::Boxen { data } => {
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
            SeriesType::Contour { data } => {
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
                    let colorbar_width = 20.0;
                    let colorbar_margin = 10.0;
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
                        data.config.colorbar_label.as_deref(),
                        self.display.theme.foreground,
                        data.config.colorbar_tick_font_size,
                        Some(data.config.colorbar_label_font_size),
                    )?;
                }
            }
            SeriesType::Pie { data } => {
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
            SeriesType::Radar { data } => {
                // Use PlotRender trait to render Radar with 1:1 aspect ratio
                // and extra top padding for title clearance
                let radar_plot_area = Self::radar_plot_area(plot_area, x_min, x_max, y_min, y_max);
                data.render(renderer, &radar_plot_area, &self.display.theme, color)?;
            }
            SeriesType::Polar { data } => {
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
        renderer: &mut SkiaRenderer,
        gpu_renderer: &mut GpuRenderer,
        plot_area: tiny_skia::Rect,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
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

        match &series.series_type {
            SeriesType::Line { x_data, y_data } => {
                // Resolve PlotData to concrete Vec<f64>
                let x_data = x_data.resolve(0.0);
                let y_data = y_data.resolve(0.0);

                // Use GPU for coordinate transformation
                let viewport = (
                    plot_area.x(),
                    plot_area.y(),
                    plot_area.x() + plot_area.width(),
                    plot_area.y() + plot_area.height(),
                );

                let (x_transformed, y_transformed) = gpu_renderer
                    .transform_coordinates_optimal(
                        &x_data,
                        &y_data,
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
            SeriesType::Scatter { x_data, y_data } => {
                // Resolve PlotData to concrete Vec<f64>
                let x_data = x_data.resolve(0.0);
                let y_data = y_data.resolve(0.0);

                // Use GPU for coordinate transformation
                let viewport = (
                    plot_area.x(),
                    plot_area.y(),
                    plot_area.x() + plot_area.width(),
                    plot_area.y() + plot_area.height(),
                );

                let (x_transformed, y_transformed) = gpu_renderer
                    .transform_coordinates_optimal(
                        &x_data,
                        &y_data,
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
                self.render_series_normal(series, renderer, plot_area, x_min, x_max, y_min, y_max)?;
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
        if figure.dpi < crate::core::constants::dpi::MIN as f32 {
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
        PlottingError::validate_dimensions(width, height)?;
        Ok(())
    }

    pub(super) fn validate_runtime_inputs(&self) -> Result<()> {
        self.validate_runtime_inputs_for_series(&self.series_mgr.series)
    }
}
