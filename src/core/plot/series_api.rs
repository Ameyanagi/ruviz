use super::*;

impl Plot {
    /// Add a scoped group of series that share style defaults.
    ///
    /// Styles configured on the group builder apply to every member series
    /// added inside the closure and do not leak outside the group.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x = vec![0.0, 1.0, 2.0, 3.0];
    /// let y1 = vec![0.0, 1.0, 2.0, 3.0];
    /// let y2 = vec![0.0, 1.5, 3.0, 4.5];
    ///
    /// Plot::new()
    ///     .group(|g| {
    ///         g.group_label("Sensors")
    ///             .line_width(2.0)
    ///             .line_style(LineStyle::Dashed)
    ///             .line(&x, &y1)
    ///             .line(&x, &y2)
    ///     })
    ///     .legend_best()
    ///     .save("grouped.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn group<F>(self, f: F) -> Self
    where
        F: FnOnce(SeriesGroupBuilder) -> SeriesGroupBuilder,
    {
        f(SeriesGroupBuilder::new(self)).finalize()
    }

    /// Add a line plot series
    ///
    /// Creates a line chart connecting data points in order.
    /// Returns a `PlotBuilder<LineConfig>` for method chaining with line-specific options.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    /// let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
    ///
    /// // Simple usage - just call save() directly
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .save("line.png")?;
    ///
    /// // With configuration
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .line_width(2.0)
    ///     .color(Color::BLUE)
    ///     .marker(MarkerStyle::Circle)
    ///     .title("Sine Wave")
    ///     .save("line_styled.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// ![Line plot example](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/line_plot.png)
    pub fn line<X, Y>(self, x_data: &X, y_data: &Y) -> PlotBuilder<crate::plots::basic::LineConfig>
    where
        X: NumericData1D,
        Y: NumericData1D,
    {
        let mut plot = self;
        let x_vec = match collect_numeric_data_1d(x_data, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };
        let y_vec = match collect_numeric_data_1d(y_data, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };

        PlotBuilder::new(
            plot,
            PlotInput::XY(x_vec, y_vec),
            crate::plots::basic::LineConfig::default(),
        )
    }

    /// Add a line series from source-backed data.
    pub fn line_source<X, Y>(
        self,
        x_data: X,
        y_data: Y,
    ) -> PlotBuilder<crate::plots::basic::LineConfig>
    where
        X: IntoPlotData,
        Y: IntoPlotData,
    {
        PlotBuilder::new(
            self,
            PlotInput::XYSource(x_data.into_plot_data(), y_data.into_plot_data()),
            crate::plots::basic::LineConfig::default(),
        )
    }

    /// Add a line plot series from streaming data
    ///
    /// This method reads the current data from the StreamingXY buffer at render time.
    /// The buffer can continue to receive updates, and subsequent renders will
    /// include the new data.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    /// use ruviz::data::StreamingXY;
    ///
    /// let stream = StreamingXY::new(1000);
    ///
    /// // Push data (can be from another thread)
    /// stream.push(0.0, 0.0);
    /// stream.push(1.0, 1.0);
    /// stream.push(2.0, 4.0);
    ///
    /// // Render current state
    /// Plot::new()
    ///     .line_streaming(&stream)
    ///     .title("Streaming Data")
    ///     .save("stream.png")?;
    ///
    /// // More data arrives
    /// stream.push(3.0, 9.0);
    ///
    /// // Re-render with new data
    /// Plot::new()
    ///     .line_streaming(&stream)
    ///     .save("stream_updated.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn line_streaming(self, stream: &StreamingXY) -> PlotSeriesBuilder {
        let x_data = PlotData::Streaming(stream.x().clone());
        let y_data = PlotData::Streaming(stream.y().clone());

        let series = PlotSeries {
            series_type: SeriesType::Line { x_data, y_data },
            streaming_source: Some(stream.clone()),
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
            alpha: None,
            alpha_source: None,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        PlotSeriesBuilder::new(self, series)
    }

    /// Add a scatter plot series
    ///
    /// Creates a scatter plot showing individual data points as markers.
    /// Returns a `PlotBuilder<ScatterConfig>` for method chaining with scatter-specific options.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x: Vec<f64> = (0..50).map(|i| i as f64 * 0.2).collect();
    /// let y: Vec<f64> = x.iter().map(|&v| v.sin()).collect();
    ///
    /// // Simple usage - just call save() directly
    /// Plot::new()
    ///     .scatter(&x, &y)
    ///     .save("scatter.png")?;
    ///
    /// // With configuration
    /// Plot::new()
    ///     .scatter(&x, &y)
    ///     .marker(MarkerStyle::Triangle)
    ///     .marker_size(10.0)
    ///     .color(Color::RED)
    ///     .title("Data Points")
    ///     .save("scatter_styled.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// ![Scatter plot example](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/scatter_plot.png)
    pub fn scatter<X, Y>(
        self,
        x_data: &X,
        y_data: &Y,
    ) -> PlotBuilder<crate::plots::basic::ScatterConfig>
    where
        X: NumericData1D,
        Y: NumericData1D,
    {
        let mut plot = self;
        let x_vec = match collect_numeric_data_1d(x_data, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };
        let y_vec = match collect_numeric_data_1d(y_data, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };

        PlotBuilder::new(
            plot,
            PlotInput::XY(x_vec, y_vec),
            crate::plots::basic::ScatterConfig::default(),
        )
    }

    /// Add a scatter series from source-backed data.
    pub fn scatter_source<X, Y>(
        self,
        x_data: X,
        y_data: Y,
    ) -> PlotBuilder<crate::plots::basic::ScatterConfig>
    where
        X: IntoPlotData,
        Y: IntoPlotData,
    {
        PlotBuilder::new(
            self,
            PlotInput::XYSource(x_data.into_plot_data(), y_data.into_plot_data()),
            crate::plots::basic::ScatterConfig::default(),
        )
    }

    /// Add a scatter plot series from streaming data
    ///
    /// Similar to `line_streaming`, reads current data from the buffer at render time.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    /// use ruviz::data::StreamingXY;
    ///
    /// let stream = StreamingXY::new(1000);
    /// stream.push_many(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 4.0)]);
    ///
    /// Plot::new()
    ///     .scatter_streaming(&stream)
    ///     .title("Streaming Scatter")
    ///     .save("stream_scatter.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn scatter_streaming(self, stream: &StreamingXY) -> PlotSeriesBuilder {
        let x_data = PlotData::Streaming(stream.x().clone());
        let y_data = PlotData::Streaming(stream.y().clone());

        let series = PlotSeries {
            series_type: SeriesType::Scatter { x_data, y_data },
            streaming_source: Some(stream.clone()),
            label: None,
            color: None,
            color_source: None,
            line_width: None,
            line_width_source: None,
            line_style: None,
            line_style_source: None,
            marker_style: Some(MarkerStyle::Circle),
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

        PlotSeriesBuilder::new(self, series)
    }

    /// Add a bar plot series
    ///
    /// Creates a bar chart with categorical x-axis labels.
    ///
    /// # Example
    ///
    /// Returns a `PlotBuilder<BarConfig>` for method chaining with bar-specific options.
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let categories = vec!["A", "B", "C", "D", "E"];
    /// let values = vec![23.0, 45.0, 56.0, 78.0, 32.0];
    ///
    /// // Simple usage - just call save() directly
    /// Plot::new()
    ///     .bar(&categories, &values)
    ///     .save("bar.png")?;
    ///
    /// // With configuration
    /// Plot::new()
    ///     .bar(&categories, &values)
    ///     .bar_width(0.6)
    ///     .color(Color::GREEN)
    ///     .edge_width(1.5)
    ///     .title("Category Values")
    ///     .save("bar_styled.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// ![Bar chart example](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/bar_chart.png)
    pub fn bar<S, V>(
        self,
        categories: &[S],
        values: &V,
    ) -> PlotBuilder<crate::plots::basic::BarConfig>
    where
        S: ToString,
        V: NumericData1D,
    {
        let mut plot = self;
        let cat_vec: Vec<String> = categories.iter().map(|s| s.to_string()).collect();
        let val_vec = match collect_numeric_data_1d(values, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };

        PlotBuilder::new(
            plot,
            PlotInput::Categorical {
                categories: cat_vec,
                values: val_vec,
            },
            crate::plots::basic::BarConfig::default(),
        )
    }

    /// Add a bar series from source-backed values.
    pub fn bar_source<S, V>(
        self,
        categories: &[S],
        values: V,
    ) -> PlotBuilder<crate::plots::basic::BarConfig>
    where
        S: ToString,
        V: IntoPlotData,
    {
        PlotBuilder::new(
            self,
            PlotInput::CategoricalSource {
                categories: categories.iter().map(ToString::to_string).collect(),
                values: values.into_plot_data(),
            },
            crate::plots::basic::BarConfig::default(),
        )
    }

    /// Add a histogram plot series
    ///
    /// Creates a histogram showing the distribution of data values.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let data: Vec<f64> = (0..1000).map(|i| (i as f64 / 100.0).sin()).collect();
    ///
    /// Plot::new()
    ///     .histogram(&data, None)
    ///     .end_series()
    ///     .save("histogram.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// ![Histogram example](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/histogram.png)
    pub fn histogram<D: NumericData1D>(
        self,
        data: &D,
        config: Option<HistogramConfig>,
    ) -> PlotSeriesBuilder {
        let mut plot = self;
        let data_vec = match collect_numeric_data_1d(data, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };
        let hist_config = config.unwrap_or_default();
        let prepared = crate::plots::histogram::calculate_histogram(&data_vec, &hist_config)
            .map_err(|err| {
                plot.set_pending_ingestion_error(PlottingError::RenderError(err.to_string()))
            })
            .ok();

        let series = PlotSeries {
            series_type: SeriesType::Histogram {
                data: PlotData::Static(data_vec),
                config: hist_config,
                prepared,
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
            alpha: None,
            alpha_source: None,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        PlotSeriesBuilder::new(plot, series)
    }

    /// Add a histogram series from source-backed values.
    pub fn histogram_source<D: IntoPlotData>(
        self,
        data: D,
        config: Option<HistogramConfig>,
    ) -> PlotSeriesBuilder {
        let series = PlotSeries {
            series_type: SeriesType::Histogram {
                data: data.into_plot_data(),
                config: config.unwrap_or_default(),
                prepared: None,
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
            alpha: None,
            alpha_source: None,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        PlotSeriesBuilder::new(self, series)
    }

    /// Add a box plot series
    ///
    /// Creates a box plot showing the distribution of data with quartiles,
    /// median, and outliers.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    /// use ruviz::plots::boxplot::BoxPlotConfig;
    ///
    /// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0,
    ///                 11.0, 12.0, 35.0, 40.0, -5.0]; // includes outliers
    ///
    /// Plot::new()
    ///     .boxplot(&data, Some(BoxPlotConfig::new()))
    ///     .end_series()
    ///     .save("boxplot.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// ![Box plot example](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/boxplot.png)
    pub fn boxplot<D: NumericData1D>(
        self,
        data: &D,
        config: Option<BoxPlotConfig>,
    ) -> PlotSeriesBuilder {
        let mut plot = self;
        let data_vec = match collect_numeric_data_1d(data, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };
        let box_config = config.unwrap_or_default();

        let series = PlotSeries {
            series_type: SeriesType::BoxPlot {
                data: PlotData::Static(data_vec),
                config: box_config,
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
            alpha: None,
            alpha_source: None,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        PlotSeriesBuilder::new(plot, series)
    }

    /// Add a box plot series from source-backed values.
    pub fn boxplot_source<D: IntoPlotData>(
        self,
        data: D,
        config: Option<BoxPlotConfig>,
    ) -> PlotSeriesBuilder {
        let series = PlotSeries {
            series_type: SeriesType::BoxPlot {
                data: data.into_plot_data(),
                config: config.unwrap_or_default(),
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
            alpha: None,
            alpha_source: None,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        PlotSeriesBuilder::new(self, series)
    }

    /// Add a heatmap visualization for 2D array data
    ///
    /// Creates a color-mapped visualization of 2D data.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let data: Vec<Vec<f64>> = (0..10)
    ///     .map(|i| (0..10).map(|j| (i + j + 1) as f64).collect())
    ///     .collect();
    ///
    /// let config = HeatmapConfig::new()
    ///     .value_scale(AxisScale::Log)
    ///     .colorbar_label("Intensity");
    ///
    /// Plot::new()
    ///     .heatmap(&data, Some(config))
    ///     .end_series()
    ///     .save("heatmap.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// ![Heatmap example](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/heatmap.png)
    pub fn heatmap<D>(
        mut self,
        data: &D,
        config: Option<crate::plots::heatmap::HeatmapConfig>,
    ) -> PlotSeriesBuilder
    where
        D: NumericData2D + ?Sized,
    {
        let heatmap_config = config.unwrap_or_default();

        // Disable grid for heatmaps (grid doesn't make sense behind heatmap cells)
        self.layout.grid_style.visible = false;

        // Process heatmap data
        let (flat, rows, cols) = match collect_numeric_data_2d(data) {
            Ok(values) => values,
            Err(err) => {
                self.set_pending_ingestion_error(err);
                (vec![], 0, 0)
            }
        };
        match crate::plots::heatmap::process_heatmap_flat(&flat, rows, cols, heatmap_config) {
            Ok(heatmap_data) => {
                let series = PlotSeries {
                    series_type: SeriesType::Heatmap { data: heatmap_data },
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
                    alpha: None,
                    alpha_source: None,
                    y_errors: None,
                    x_errors: None,
                    error_config: None,
                    inset_layout: None,
                    group_id: None,
                };
                PlotSeriesBuilder::new(self, series)
            }
            Err(err) => {
                // Return empty plot if data processing fails
                // This allows chaining to continue without panicking
                self.set_pending_ingestion_error(PlottingError::DataExtractionFailed {
                    source: "heatmap".to_string(),
                    message: err,
                });
                let series = PlotSeries {
                    series_type: SeriesType::Heatmap {
                        data: crate::plots::heatmap::HeatmapData {
                            values: vec![vec![0.0]],
                            n_rows: 1,
                            n_cols: 1,
                            data_min: 0.0,
                            data_max: 0.0,
                            vmin: 0.0,
                            vmax: 1.0,
                            config: crate::plots::heatmap::HeatmapConfig::default(),
                        },
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
                    alpha: None,
                    alpha_source: None,
                    y_errors: None,
                    x_errors: None,
                    error_config: None,
                    inset_layout: None,
                    group_id: None,
                };
                PlotSeriesBuilder::new(self, series)
            }
        }
    }

    /// Add error bars (Y-direction only)
    pub fn error_bars<X, Y, E>(self, x_data: &X, y_data: &Y, y_errors: &E) -> PlotSeriesBuilder
    where
        X: NumericData1D,
        Y: NumericData1D,
        E: NumericData1D,
    {
        let mut plot = self;
        let x_vec = match collect_numeric_data_1d(x_data, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };
        let y_vec = match collect_numeric_data_1d(y_data, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };
        let e_vec = match collect_numeric_data_1d(y_errors, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };

        let series = PlotSeries {
            series_type: SeriesType::ErrorBars {
                x_data: PlotData::Static(x_vec),
                y_data: PlotData::Static(y_vec),
                y_errors: PlotData::Static(e_vec),
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
            alpha: None,
            alpha_source: None,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        PlotSeriesBuilder::new(plot, series)
    }

    /// Add error bars from source-backed X, Y, and Y-error data.
    pub fn error_bars_source<X, Y, E>(self, x_data: X, y_data: Y, y_errors: E) -> PlotSeriesBuilder
    where
        X: IntoPlotData,
        Y: IntoPlotData,
        E: IntoPlotData,
    {
        let series = PlotSeries {
            series_type: SeriesType::ErrorBars {
                x_data: x_data.into_plot_data(),
                y_data: y_data.into_plot_data(),
                y_errors: y_errors.into_plot_data(),
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
            alpha: None,
            alpha_source: None,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        PlotSeriesBuilder::new(self, series)
    }

    /// Add error bars in both X and Y directions
    pub fn error_bars_xy<X, Y, EX, EY>(
        self,
        x_data: &X,
        y_data: &Y,
        x_errors: &EX,
        y_errors: &EY,
    ) -> PlotSeriesBuilder
    where
        X: NumericData1D,
        Y: NumericData1D,
        EX: NumericData1D,
        EY: NumericData1D,
    {
        let mut plot = self;
        let x_vec = match collect_numeric_data_1d(x_data, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };
        let y_vec = match collect_numeric_data_1d(y_data, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };
        let ex_vec = match collect_numeric_data_1d(x_errors, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };
        let ey_vec = match collect_numeric_data_1d(y_errors, plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                plot.set_pending_ingestion_error(err);
                vec![]
            }
        };

        let series = PlotSeries {
            series_type: SeriesType::ErrorBarsXY {
                x_data: PlotData::Static(x_vec),
                y_data: PlotData::Static(y_vec),
                x_errors: PlotData::Static(ex_vec),
                y_errors: PlotData::Static(ey_vec),
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
            alpha: None,
            alpha_source: None,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        PlotSeriesBuilder::new(plot, series)
    }

    /// Add X/Y error bars from source-backed data.
    pub fn error_bars_xy_source<X, Y, EX, EY>(
        self,
        x_data: X,
        y_data: Y,
        x_errors: EX,
        y_errors: EY,
    ) -> PlotSeriesBuilder
    where
        X: IntoPlotData,
        Y: IntoPlotData,
        EX: IntoPlotData,
        EY: IntoPlotData,
    {
        let series = PlotSeries {
            series_type: SeriesType::ErrorBarsXY {
                x_data: x_data.into_plot_data(),
                y_data: y_data.into_plot_data(),
                x_errors: x_errors.into_plot_data(),
                y_errors: y_errors.into_plot_data(),
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
            alpha: None,
            alpha_source: None,
            y_errors: None,
            x_errors: None,
            error_config: None,
            inset_layout: None,
            group_id: None,
        };

        PlotSeriesBuilder::new(self, series)
    }

    /// Add a KDE (Kernel Density Estimation) plot
    ///
    /// Creates a smooth density estimate visualization of the data distribution.
    /// Returns a `PlotBuilder<KdeConfig>` for method chaining with KDE-specific options.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let data: Vec<f64> = (0..1000).map(|i| (i as f64 / 100.0).sin()).collect();
    ///
    /// // Simple usage - just save directly
    /// Plot::new()
    ///     .kde(&data)
    ///     .save("kde.png")?;
    ///
    /// // With configuration
    /// Plot::new()
    ///     .kde(&data)
    ///     .bandwidth(0.5)
    ///     .fill(true)
    ///     .fill_alpha(0.3)
    ///     .title("KDE Distribution")
    ///     .save("kde_configured.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn kde<T, D: Data1D<T>>(self, data: &D) -> PlotBuilder<crate::plots::KdeConfig>
    where
        T: Into<f64> + Copy,
    {
        let mut data_vec = Vec::with_capacity(data.len());
        for i in 0..data.len() {
            if let Some(val) = data.get(i) {
                data_vec.push((*val).into());
            }
        }

        PlotBuilder::new(
            self,
            PlotInput::Single(data_vec),
            crate::plots::KdeConfig::default(),
        )
    }

    /// Start building an ECDF (Empirical Cumulative Distribution Function) plot
    ///
    /// Returns a `PlotBuilder<EcdfConfig>` for configuring the ECDF plot.
    ///
    /// # Example
    ///
    /// ```ignore
    /// Plot::new()
    ///     .ecdf(&data)
    ///     .stat(EcdfStat::Proportion)
    ///     .show_ci(true)
    ///     .save("ecdf.png")?;
    /// ```
    pub fn ecdf<T, D: Data1D<T>>(self, data: &D) -> PlotBuilder<crate::plots::EcdfConfig>
    where
        T: Into<f64> + Copy,
    {
        let mut data_vec = Vec::with_capacity(data.len());
        for i in 0..data.len() {
            if let Some(val) = data.get(i) {
                data_vec.push((*val).into());
            }
        }

        PlotBuilder::new(
            self,
            PlotInput::Single(data_vec),
            crate::plots::EcdfConfig::default(),
        )
    }

    /// Add a contour plot for 2D scalar field visualization
    ///
    /// Creates contour lines or filled contours from grid data.
    /// Returns a `PlotBuilder<ContourConfig>` for method chaining.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinates of the grid (1D array)
    /// * `y` - Y coordinates of the grid (1D array)
    /// * `z` - Z values as a flattened 2D array (row-major, len = x.len() * y.len())
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x: Vec<f64> = (-50..=50).map(|i| i as f64 / 10.0).collect();
    /// let y: Vec<f64> = (-50..=50).map(|i| i as f64 / 10.0).collect();
    /// let z: Vec<f64> = y.iter().flat_map(|yv| {
    ///     x.iter().map(move |xv| (-xv*xv - yv*yv).exp())
    /// }).collect();
    ///
    /// Plot::new()
    ///     .title("Gaussian Surface")
    ///     .contour(&x, &y, &z)
    ///     .levels(10)
    ///     .filled(true)
    ///     .save("contour.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn contour<X, Y, Z>(self, x: &X, y: &Y, z: &Z) -> PlotBuilder<crate::plots::ContourConfig>
    where
        X: Data1D<f64>,
        Y: Data1D<f64>,
        Z: Data1D<f64>,
    {
        let x_vec: Vec<f64> = (0..x.len()).filter_map(|i| x.get(i).copied()).collect();
        let y_vec: Vec<f64> = (0..y.len()).filter_map(|i| y.get(i).copied()).collect();
        let z_vec: Vec<f64> = (0..z.len()).filter_map(|i| z.get(i).copied()).collect();

        // Convert flat z to 2D grid (row-major)
        let ny = y_vec.len();
        let nx = x_vec.len();
        let z_2d: Vec<Vec<f64>> = (0..ny)
            .map(|j| {
                (0..nx)
                    .map(|i| z_vec.get(j * nx + i).copied().unwrap_or(0.0))
                    .collect()
            })
            .collect();

        PlotBuilder::new(
            self,
            PlotInput::Grid2D {
                x: x_vec,
                y: y_vec,
                z: z_2d,
            },
            crate::plots::ContourConfig::default(),
        )
    }

    /// Add a pie chart for proportional data visualization
    ///
    /// Creates a pie chart with optional labels, exploded segments, and donut style.
    /// Returns a `PlotBuilder<PieConfig>` for method chaining.
    /// When mixed with Cartesian series, the pie chart renders as an inset and
    /// can be positioned with the builder's `inset_*` methods.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let values = vec![35.0, 25.0, 20.0, 15.0, 5.0];
    ///
    /// Plot::new()
    ///     .title("Market Share")
    ///     .pie(&values)
    ///     .labels(&["A", "B", "C", "D", "Other"])
    ///     .donut(0.4)
    ///     .save("pie.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn pie<V>(self, values: &V) -> PlotBuilder<crate::plots::PieConfig>
    where
        V: Data1D<f64>,
    {
        let values_vec: Vec<f64> = (0..values.len())
            .filter_map(|i| values.get(i).copied())
            .collect();

        PlotBuilder::new(
            self,
            PlotInput::Single(values_vec),
            crate::plots::PieConfig::default(),
        )
    }

    /// Add a radar/spider chart for multivariate data comparison
    ///
    /// Creates a radar chart with multiple axes arranged in a circle.
    /// Returns a `PlotBuilder<RadarConfig>` for method chaining.
    /// When mixed with Cartesian series, the radar chart renders as an inset and
    /// can be positioned with the builder's `inset_*` methods.
    ///
    /// # Arguments
    ///
    /// * `labels` - Labels for each axis spoke
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .title("Player Stats")
    ///     .radar(&["Speed", "Power", "Defense", "Magic", "Luck"])
    ///     .series(&[85.0, 92.0, 78.0, 65.0, 88.0])
    ///     .label("Player 1")
    ///     .fill_alpha(0.3)
    ///     .series(&[72.0, 68.0, 95.0, 82.0, 75.0])
    ///     .label("Player 2")
    ///     .fill_alpha(0.3)
    ///     .save("radar.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn radar<S: AsRef<str>>(self, labels: &[S]) -> PlotBuilder<crate::plots::RadarConfig> {
        let label_strings: Vec<String> = labels.iter().map(|s| s.as_ref().to_string()).collect();

        let config = crate::plots::RadarConfig::default().labels(label_strings);

        PlotBuilder::new(self, PlotInput::Single(vec![]), config)
    }

    /// Add a Violin plot for visualizing distribution shapes
    ///
    /// Creates a violin plot combining KDE density estimation with optional
    /// box/strip components for statistical visualization.
    /// Returns a `PlotBuilder<ViolinConfig>` for method chaining with violin-specific options.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let data: Vec<f64> = (0..200).map(|i| {
    ///     let x = (i as f64 * 0.1).sin() * 3.0 + 5.0;
    ///     x
    /// }).collect();
    ///
    /// // Simple usage
    /// Plot::new()
    ///     .violin(&data)
    ///     .save("violin.png")?;
    ///
    /// // With configuration
    /// Plot::new()
    ///     .violin(&data)
    ///     .show_box(true)
    ///     .show_median(true)
    ///     .fill_alpha(0.6)
    ///     .title("Distribution")
    ///     .save("violin_configured.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn violin<T, D: Data1D<T>>(self, data: &D) -> PlotBuilder<crate::plots::ViolinConfig>
    where
        T: Into<f64> + Copy,
    {
        let mut data_vec = Vec::with_capacity(data.len());
        for i in 0..data.len() {
            if let Some(val) = data.get(i) {
                data_vec.push((*val).into());
            }
        }

        PlotBuilder::new(
            self,
            PlotInput::Single(data_vec),
            crate::plots::ViolinConfig::default(),
        )
    }

    /// Add a Polar line plot for visualizing data in polar coordinates
    ///
    /// Creates a polar plot with r (radius) and theta (angle in radians) data.
    /// Returns a `PlotBuilder<PolarPlotConfig>` for method chaining with polar-specific options.
    /// When mixed with Cartesian series, the polar chart renders as an inset and
    /// can be positioned with the builder's `inset_*` methods.
    ///
    /// # Arguments
    ///
    /// * `r` - Radius values (distance from center)
    /// * `theta` - Angle values in radians
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    /// use std::f64::consts::PI;
    ///
    /// // Rose curve
    /// let n_points = 200;
    /// let theta: Vec<f64> = (0..n_points)
    ///     .map(|i| i as f64 * 2.0 * PI / n_points as f64)
    ///     .collect();
    /// let r: Vec<f64> = theta.iter().map(|&t| (3.0 * t).cos().abs()).collect();
    ///
    /// Plot::new()
    ///     .title("Rose Curve")
    ///     .polar_line(&r, &theta)
    ///     .fill(true)
    ///     .fill_alpha(0.3)
    ///     .save("polar.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn polar_line<R, T>(self, r: &R, theta: &T) -> PlotBuilder<crate::plots::PolarPlotConfig>
    where
        R: Data1D<f64>,
        T: Data1D<f64>,
    {
        let r_vec: Vec<f64> = (0..r.len()).filter_map(|i| r.get(i).copied()).collect();
        let theta_vec: Vec<f64> = (0..theta.len())
            .filter_map(|i| theta.get(i).copied())
            .collect();

        PlotBuilder::new(
            self,
            PlotInput::XY(r_vec, theta_vec),
            crate::plots::PolarPlotConfig::default(),
        )
    }
}
