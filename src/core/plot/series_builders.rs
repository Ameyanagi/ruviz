use super::*;

impl Default for Plot {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for scoped grouped-series construction.
///
/// Created via [`Plot::group`], this builder applies shared style defaults
/// to all series added within the group.
#[derive(Clone, Debug)]
pub struct SeriesGroupBuilder {
    plot: Plot,
    style: builder::SeriesStyle,
    group_id: usize,
    resolved_auto_color: Option<Color>,
}

impl SeriesGroupBuilder {
    pub(super) fn new(mut plot: Plot) -> Self {
        let group_id = plot.register_series_group();
        Self {
            plot,
            style: builder::SeriesStyle::default(),
            group_id,
            resolved_auto_color: None,
        }
    }

    pub(super) fn finalize(self) -> Plot {
        self.plot
    }

    /// Set the label used for the single legend entry of this group.
    pub fn group_label<S: Into<String>>(mut self, label: S) -> Self {
        self.plot
            .set_series_group_label(self.group_id, label.into());
        self
    }

    /// Set shared color applied to all group member series.
    pub fn color(mut self, color: Color) -> Self {
        self.style.color = Some(color);
        self.style.color_source = None;
        self
    }

    /// Set a shared reactive color applied to all group member series.
    pub fn color_source<S>(mut self, color: S) -> Self
    where
        S: Into<ReactiveValue<Color>>,
    {
        self.style.set_color_source_value(color.into());
        self
    }

    /// Set shared line width applied to all group member series.
    pub fn line_width(mut self, width: f32) -> Self {
        self.style.line_width = Some(width.max(0.1));
        self.style.line_width_source = None;
        self
    }

    /// Set a shared reactive line width applied to all group member series.
    pub fn line_width_source<S>(mut self, width: S) -> Self
    where
        S: Into<ReactiveValue<f32>>,
    {
        self.style.set_line_width_source_value(width.into());
        self
    }

    /// Set shared line style applied to all group member series.
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.style.line_style = Some(style);
        self.style.line_style_source = None;
        self
    }

    /// Set a shared reactive line style applied to all group member series.
    pub fn line_style_source<S>(mut self, style: S) -> Self
    where
        S: Into<ReactiveValue<LineStyle>>,
    {
        self.style.set_line_style_source_value(style.into());
        self
    }

    /// Set shared alpha/transparency applied to all group member series.
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.style.alpha = Some(alpha.clamp(0.0, 1.0));
        self.style.alpha_source = None;
        self
    }

    /// Set a shared reactive alpha/transparency applied to all group member series.
    pub fn alpha_source<S>(mut self, alpha: S) -> Self
    where
        S: Into<ReactiveValue<f32>>,
    {
        self.style.set_alpha_source_value(alpha.into());
        self
    }

    /// Add a line series to the current group.
    pub fn line<X, Y>(mut self, x_data: &X, y_data: &Y) -> Self
    where
        X: NumericData1D,
        Y: NumericData1D,
    {
        let x_vec = match collect_numeric_data_1d(x_data, self.plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                self.plot.set_pending_ingestion_error(err);
                vec![]
            }
        };
        let y_vec = match collect_numeric_data_1d(y_data, self.plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                self.plot.set_pending_ingestion_error(err);
                vec![]
            }
        };

        let mut style = self.style.clone();
        let mut consume_palette_index = true;
        if self.style.color.is_none() && self.style.color_source.is_none() {
            if let Some(color) = self.resolved_auto_color {
                style.color = Some(color);
                consume_palette_index = false;
            }
        }

        self.plot = self.plot.add_line_series_grouped(
            PlotData::Static(x_vec),
            PlotData::Static(y_vec),
            &crate::plots::basic::LineConfig::default(),
            style,
            Some(self.group_id),
            consume_palette_index,
        );

        if self.style.color.is_none()
            && self.style.color_source.is_none()
            && self.resolved_auto_color.is_none()
        {
            self.resolved_auto_color = self.plot.series_mgr.series.last().and_then(|s| s.color);
        }
        self
    }

    /// Add a line series from source-backed data to the current group.
    pub fn line_source<X, Y>(mut self, x_data: X, y_data: Y) -> Self
    where
        X: IntoPlotData,
        Y: IntoPlotData,
    {
        let mut style = self.style.clone();
        let mut consume_palette_index = true;
        if self.style.color.is_none() && self.style.color_source.is_none() {
            if let Some(color) = self.resolved_auto_color {
                style.color = Some(color);
                consume_palette_index = false;
            }
        }

        self.plot = self.plot.add_line_series_grouped(
            x_data.into_plot_data(),
            y_data.into_plot_data(),
            &crate::plots::basic::LineConfig::default(),
            style,
            Some(self.group_id),
            consume_palette_index,
        );

        if self.style.color.is_none()
            && self.style.color_source.is_none()
            && self.resolved_auto_color.is_none()
        {
            self.resolved_auto_color = self.plot.series_mgr.series.last().and_then(|s| s.color);
        }
        self
    }

    /// Add a scatter series to the current group.
    pub fn scatter<X, Y>(mut self, x_data: &X, y_data: &Y) -> Self
    where
        X: NumericData1D,
        Y: NumericData1D,
    {
        let x_vec = match collect_numeric_data_1d(x_data, self.plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                self.plot.set_pending_ingestion_error(err);
                vec![]
            }
        };
        let y_vec = match collect_numeric_data_1d(y_data, self.plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                self.plot.set_pending_ingestion_error(err);
                vec![]
            }
        };

        let mut style = self.style.clone();
        let mut consume_palette_index = true;
        if self.style.color.is_none() && self.style.color_source.is_none() {
            if let Some(color) = self.resolved_auto_color {
                style.color = Some(color);
                consume_palette_index = false;
            }
        }

        self.plot = self.plot.add_scatter_series_grouped(
            PlotData::Static(x_vec),
            PlotData::Static(y_vec),
            &crate::plots::basic::ScatterConfig::default(),
            style,
            Some(self.group_id),
            consume_palette_index,
        );

        if self.style.color.is_none()
            && self.style.color_source.is_none()
            && self.resolved_auto_color.is_none()
        {
            self.resolved_auto_color = self.plot.series_mgr.series.last().and_then(|s| s.color);
        }
        self
    }

    /// Add a scatter series from source-backed data to the current group.
    pub fn scatter_source<X, Y>(mut self, x_data: X, y_data: Y) -> Self
    where
        X: IntoPlotData,
        Y: IntoPlotData,
    {
        let mut style = self.style.clone();
        let mut consume_palette_index = true;
        if self.style.color.is_none() && self.style.color_source.is_none() {
            if let Some(color) = self.resolved_auto_color {
                style.color = Some(color);
                consume_palette_index = false;
            }
        }

        self.plot = self.plot.add_scatter_series_grouped(
            x_data.into_plot_data(),
            y_data.into_plot_data(),
            &crate::plots::basic::ScatterConfig::default(),
            style,
            Some(self.group_id),
            consume_palette_index,
        );

        if self.style.color.is_none()
            && self.style.color_source.is_none()
            && self.resolved_auto_color.is_none()
        {
            self.resolved_auto_color = self.plot.series_mgr.series.last().and_then(|s| s.color);
        }
        self
    }

    /// Add a bar series to the current group.
    pub fn bar<S, V>(mut self, categories: &[S], values: &V) -> Self
    where
        S: ToString,
        V: NumericData1D,
    {
        let cat_vec: Vec<String> = categories.iter().map(ToString::to_string).collect();
        let val_vec = match collect_numeric_data_1d(values, self.plot.null_policy) {
            Ok(values) => values,
            Err(err) => {
                self.plot.set_pending_ingestion_error(err);
                vec![]
            }
        };

        let mut style = self.style.clone();
        let mut consume_palette_index = true;
        if self.style.color.is_none() && self.style.color_source.is_none() {
            if let Some(color) = self.resolved_auto_color {
                style.color = Some(color);
                consume_palette_index = false;
            }
        }

        self.plot = self.plot.add_bar_series_grouped(
            cat_vec,
            PlotData::Static(val_vec),
            &crate::plots::basic::BarConfig::default(),
            style,
            Some(self.group_id),
            consume_palette_index,
        );

        if self.style.color.is_none()
            && self.style.color_source.is_none()
            && self.resolved_auto_color.is_none()
        {
            self.resolved_auto_color = self.plot.series_mgr.series.last().and_then(|s| s.color);
        }
        self
    }

    /// Add a bar series from source-backed values to the current group.
    pub fn bar_source<S, V>(mut self, categories: &[S], values: V) -> Self
    where
        S: ToString,
        V: IntoPlotData,
    {
        let mut style = self.style.clone();
        let mut consume_palette_index = true;
        if self.style.color.is_none() && self.style.color_source.is_none() {
            if let Some(color) = self.resolved_auto_color {
                style.color = Some(color);
                consume_palette_index = false;
            }
        }

        self.plot = self.plot.add_bar_series_grouped(
            categories.iter().map(ToString::to_string).collect(),
            values.into_plot_data(),
            &crate::plots::basic::BarConfig::default(),
            style,
            Some(self.group_id),
            consume_palette_index,
        );

        if self.style.color.is_none()
            && self.style.color_source.is_none()
            && self.resolved_auto_color.is_none()
        {
            self.resolved_auto_color = self.plot.series_mgr.series.last().and_then(|s| s.color);
        }
        self
    }
}

/// Builder for configuring individual plot series
pub struct PlotSeriesBuilder {
    plot: Plot,
    series: PlotSeries,
}

impl PlotSeriesBuilder {
    pub(super) fn new(plot: Plot, series: PlotSeries) -> Self {
        Self { plot, series }
    }

    /// Set series label for legend
    ///
    /// Labels identify this series in the plot legend. **Important**: Labels are only
    /// displayed when the legend is explicitly enabled using one of:
    /// - [`Plot::legend_best()`] - auto-positioned legend (matplotlib-style, recommended)
    /// - [`Plot::legend()`] - legacy positioning
    /// - [`Plot::legend_position()`] - explicit position control
    ///
    /// Without enabling the legend, labels are stored but not rendered.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// // Legend with auto-positioning (recommended)
    /// Plot::new()
    ///     .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .label("Quadratic")
    ///     .line(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0])
    ///     .label("Linear")
    ///     .end_series()
    ///     .legend_best()  // Required to display labels
    ///     .save("labeled.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn label<S: Into<String>>(mut self, label: S) -> Self {
        self.series.label = Some(label.into());
        self
    }

    /// Set series color
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .color(Color::RED)
    ///     .line(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0])
    ///     .color(Color::from_hex("#00FF00").unwrap())
    ///     .end_series()
    ///     .save("colored_lines.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn color(mut self, color: Color) -> Self {
        self.series.color = Some(color);
        self.series.color_source = None;
        self
    }

    /// Set a reactive series color sampled at render time.
    pub fn color_source<S>(mut self, color: S) -> Self
    where
        S: Into<ReactiveValue<Color>>,
    {
        self.series.set_color_source_value(color.into());
        self
    }

    /// Set line width
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .line_width(3.0)  // Thick line
    ///     .line(&[1.0, 2.0, 3.0], &[0.5, 2.0, 4.5])
    ///     .line_width(1.0)  // Thin line
    ///     .save("line_widths.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn width(mut self, width: f32) -> Self {
        self.series.line_width = Some(width.max(0.1));
        self.series.line_width_source = None;
        self
    }

    /// Set a reactive line width sampled at render time.
    pub fn width_source<S>(mut self, width: S) -> Self
    where
        S: Into<ReactiveValue<f32>>,
    {
        self.series.set_line_width_source_value(width.into());
        self
    }

    /// Set line style
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .style(LineStyle::Dashed)
    ///     .line(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0])
    ///     .style(LineStyle::Dotted)
    ///     .end_series()
    ///     .save("line_styles.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn style(mut self, style: LineStyle) -> Self {
        self.series.line_style = Some(style);
        self.series.line_style_source = None;
        self
    }

    /// Set a reactive line style sampled at render time.
    pub fn style_source<S>(mut self, style: S) -> Self
    where
        S: Into<ReactiveValue<LineStyle>>,
    {
        self.series.set_line_style_source_value(style.into());
        self
    }

    /// Set marker style (for scatter plots)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .scatter(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .marker(MarkerStyle::Circle)
    ///     .scatter(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0])
    ///     .marker(MarkerStyle::Square)
    ///     .end_series()
    ///     .save("markers.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn marker(mut self, marker: MarkerStyle) -> Self {
        self.series.marker_style = Some(marker);
        self.series.marker_style_source = None;
        self
    }

    /// Set a reactive marker style sampled at render time.
    pub fn marker_source<S>(mut self, marker: S) -> Self
    where
        S: Into<ReactiveValue<MarkerStyle>>,
    {
        self.series.set_marker_style_source_value(marker.into());
        self
    }

    /// Set marker size (for scatter plots)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .scatter(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .marker_size(15.0)  // Large markers
    ///     .scatter(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0])
    ///     .marker_size(5.0)   // Small markers
    ///     .end_series()
    ///     .save("marker_sizes.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn marker_size(mut self, size: f32) -> Self {
        self.series.marker_size = Some(size.max(0.1));
        self.series.marker_size_source = None;
        self
    }

    /// Set a reactive marker size sampled at render time.
    pub fn marker_size_source<S>(mut self, size: S) -> Self
    where
        S: Into<ReactiveValue<f32>>,
    {
        self.series.set_marker_size_source_value(size.into());
        self
    }

    /// Set transparency
    ///
    /// Values range from 0.0 (fully transparent) to 1.0 (fully opaque).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// Plot::new()
    ///     .scatter(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .alpha(0.5)  // Semi-transparent
    ///     .end_series()
    ///     .save("transparent.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.series.alpha = Some(alpha.clamp(0.0, 1.0));
        self.series.alpha_source = None;
        self
    }

    /// Set a reactive alpha/transparency sampled at render time.
    pub fn alpha_source<S>(mut self, alpha: S) -> Self
    where
        S: Into<ReactiveValue<f32>>,
    {
        self.series.set_alpha_source_value(alpha.into());
        self
    }

    // ========== Error Bar Modifier Methods ==========

    /// Attach symmetric Y error bars to this series
    ///
    /// Error bars are drawn as vertical lines centered on each data point,
    /// extending `yerr` distance above and below.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    /// let y = vec![2.3, 3.5, 4.1, 5.8, 6.2];
    /// let yerr = vec![0.3, 0.4, 0.25, 0.5, 0.35];
    ///
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .with_yerr(&yerr)
    ///     .label("Measurement")
    ///     .end_series()
    ///     .legend_best()
    ///     .save("line_with_errors.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_yerr<E: NumericData1D>(mut self, errors: &E) -> Self {
        match collect_numeric_data_1d(errors, self.plot.null_policy) {
            Ok(values) => {
                self.series.y_errors = Some(ErrorValues::symmetric(values));
            }
            Err(err) => {
                self.plot.set_pending_ingestion_error(err);
            }
        }
        self
    }

    /// Attach symmetric X error bars to this series
    ///
    /// Error bars are drawn as horizontal lines centered on each data point,
    /// extending `xerr` distance left and right.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    /// let y = vec![2.3, 3.5, 4.1, 5.8, 6.2];
    /// let xerr = vec![0.1, 0.15, 0.1, 0.2, 0.15];
    ///
    /// Plot::new()
    ///     .scatter(&x, &y)
    ///     .with_xerr(&xerr)
    ///     .label("Data Points")
    ///     .end_series()
    ///     .save("scatter_with_xerr.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_xerr<E: NumericData1D>(mut self, errors: &E) -> Self {
        match collect_numeric_data_1d(errors, self.plot.null_policy) {
            Ok(values) => {
                self.series.x_errors = Some(ErrorValues::symmetric(values));
            }
            Err(err) => {
                self.plot.set_pending_ingestion_error(err);
            }
        }
        self
    }

    /// Attach asymmetric Y error bars to this series
    ///
    /// Error bars have different lengths above and below each point.
    ///
    /// # Arguments
    /// * `lower` - Error values extending below each point
    /// * `upper` - Error values extending above each point
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// let x = vec![1.0, 2.0, 3.0];
    /// let y = vec![5.0, 7.0, 6.0];
    /// let lower = vec![0.5, 0.3, 0.4];  // Extends down
    /// let upper = vec![0.8, 1.0, 0.6];  // Extends up
    ///
    /// Plot::new()
    ///     .line(&x, &y)
    ///     .with_yerr_asymmetric(&lower, &upper)
    ///     .label("Asymmetric Errors")
    ///     .end_series()
    ///     .save("asymmetric_errors.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_yerr_asymmetric<E1, E2>(mut self, lower: &E1, upper: &E2) -> Self
    where
        E1: NumericData1D,
        E2: NumericData1D,
    {
        let lower_values = collect_numeric_data_1d(lower, self.plot.null_policy);
        let upper_values = collect_numeric_data_1d(upper, self.plot.null_policy);
        match (lower_values, upper_values) {
            (Ok(lower), Ok(upper)) => {
                self.series.y_errors = Some(ErrorValues::asymmetric(lower, upper));
            }
            (Err(err), _) | (_, Err(err)) => {
                self.plot.set_pending_ingestion_error(err);
            }
        }
        self
    }

    /// Attach asymmetric X error bars to this series
    ///
    /// Error bars have different lengths left and right of each point.
    ///
    /// # Arguments
    /// * `left` - Error values extending left of each point
    /// * `right` - Error values extending right of each point
    pub fn with_xerr_asymmetric<E1, E2>(mut self, left: &E1, right: &E2) -> Self
    where
        E1: NumericData1D,
        E2: NumericData1D,
    {
        let left_values = collect_numeric_data_1d(left, self.plot.null_policy);
        let right_values = collect_numeric_data_1d(right, self.plot.null_policy);
        match (left_values, right_values) {
            (Ok(left), Ok(right)) => {
                self.series.x_errors = Some(ErrorValues::asymmetric(left, right));
            }
            (Err(err), _) | (_, Err(err)) => {
                self.plot.set_pending_ingestion_error(err);
            }
        }
        self
    }

    /// Configure error bar appearance
    ///
    /// Allows customization of cap size, line width, and color for attached error bars.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    /// use ruviz::plots::error::ErrorBarConfig;
    ///
    /// Plot::new()
    ///     .line(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0])
    ///     .with_yerr(&[0.3, 0.4, 0.35])
    ///     .error_config(ErrorBarConfig::default().cap_size(0.15).line_width(2.0))
    ///     .end_series()
    ///     .save("custom_errors.png")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn error_config(mut self, config: ErrorBarConfig) -> Self {
        self.series.error_config = Some(config);
        self
    }

    // ========== End Error Bar Modifier Methods ==========

    /// Finish configuring this series and return to the main Plot
    ///
    /// This consumes the builder and adds the series to the plot.
    ///
    /// **Deprecated**: Series finalize automatically via `Into<Plot>`.
    /// Use `.save()` directly or `.into()` for explicit conversion.
    /// Mixed Cartesian/non-Cartesian plots also work through normal fluent
    /// chaining, so `end_series()` is not required as a workaround.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::prelude::*;
    ///
    /// // Preferred: Use .save() directly (auto-finalizes)
    /// Plot::new()
    ///     .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .label("Series 1")
    ///     .color(Color::BLUE)
    ///     .title("My Plot")
    ///     .save("plot.png")?;
    ///
    /// // Or use .into() for explicit conversion
    /// let plot: Plot = Plot::new()
    ///     .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
    ///     .into();
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[deprecated(
        since = "0.9.0",
        note = "Not needed - series finalize automatically. Use .save() directly or .into() for explicit conversion."
    )]
    pub fn end_series(mut self) -> Plot {
        // Auto-assign color if none specified
        if self.series.color.is_none() && self.series.color_source.is_none() {
            self.series.color = Some(
                self.plot
                    .display
                    .theme
                    .get_color(self.plot.series_mgr.auto_color_index),
            );
            self.plot.series_mgr.auto_color_index += 1;
        }

        self.plot.series_mgr.series.push(self.series);
        self.plot
    }
}

// Implement Deref so methods can be chained through the builder
impl std::ops::Deref for PlotSeriesBuilder {
    type Target = Plot;

    fn deref(&self) -> &Self::Target {
        &self.plot
    }
}

/// Enable implicit conversion from PlotSeriesBuilder to Plot
///
/// This implementation allows PlotSeriesBuilder to be converted to Plot
/// automatically when needed, eliminating the need for explicit `.end_series()` calls.
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::prelude::*;
/// use ruviz::core::Result;
///
/// // Implicit conversion via .into()
/// let plot: Plot = Plot::new()
///     .line(&[1.0, 2.0, 3.0], &[1.0, 4.0, 9.0])
///     .color(Color::BLUE)
///     .into();
///
/// // Implicit conversion in function parameters
/// fn save_plot(p: impl Into<Plot>) -> Result<()> {
///     p.into().save("plot.png")
/// }
/// ```
impl From<PlotSeriesBuilder> for Plot {
    fn from(builder: PlotSeriesBuilder) -> Plot {
        #[allow(deprecated)]
        builder.end_series()
    }
}

impl builder::IntoPlot for PlotSeriesBuilder {
    fn into_plot(self) -> Plot {
        self.into()
    }

    fn as_plot(&self) -> &Plot {
        &self.plot
    }
}

// Implement most Plot methods for PlotSeriesBuilder to allow chaining
#[allow(deprecated)]
impl PlotSeriesBuilder {
    impl_series_continuation_methods!(self.end_series());

    /// Set plot title
    pub fn title(mut self, title: impl Into<data::PlotText>) -> Self {
        self.plot.display.title = Some(title.into());
        self
    }

    /// Set the plot title as reactive text.
    ///
    /// Temporal `Signal` sources are sampled when the plot is rendered:
    /// `render_at()` uses the requested time, while `render()` and `save()` use
    /// `0.0`. Push-based observables use their latest value when the snapshot is
    /// built.
    pub fn title_signal(mut self, title: impl Into<data::PlotText>) -> Self {
        self.plot.display.title = Some(title.into());
        self
    }

    /// Set X-axis label
    pub fn xlabel(mut self, label: impl Into<data::PlotText>) -> Self {
        self.plot.display.xlabel = Some(label.into());
        self
    }

    /// Set X-axis label as reactive text.
    ///
    /// Temporal `Signal` sources are sampled when the plot is rendered:
    /// `render_at()` uses the requested time, while `render()` and `save()` use
    /// `0.0`. Push-based observables use their latest value when the snapshot is
    /// built.
    pub fn xlabel_signal(mut self, label: impl Into<data::PlotText>) -> Self {
        self.plot.display.xlabel = Some(label.into());
        self
    }

    /// Set Y-axis label
    pub fn ylabel(mut self, label: impl Into<data::PlotText>) -> Self {
        self.plot.display.ylabel = Some(label.into());
        self
    }

    /// Set null handling policy for dataframe-backed numeric inputs.
    pub fn null_policy(mut self, policy: NullPolicy) -> Self {
        self.plot = self.plot.null_policy(policy);
        self
    }

    /// Set Y-axis label as reactive text.
    ///
    /// Temporal `Signal` sources are sampled when the plot is rendered:
    /// `render_at()` uses the requested time, while `render()` and `save()` use
    /// `0.0`. Push-based observables use their latest value when the snapshot is
    /// built.
    pub fn ylabel_signal(mut self, label: impl Into<data::PlotText>) -> Self {
        self.plot.display.ylabel = Some(label.into());
        self
    }

    /// Configure legend with position
    pub fn legend(mut self, position: Position) -> Self {
        self.plot.layout.legend.enabled = true;
        self.plot.layout.legend.position = position;
        self
    }

    /// Set legend font size
    pub fn legend_font_size(mut self, size: f32) -> Self {
        self.plot.layout.legend.font_size = Some(size);
        self
    }

    /// Set legend corner radius for rounded corners
    pub fn legend_corner_radius(mut self, radius: f32) -> Self {
        self.plot.layout.legend.corner_radius = Some(radius);
        self
    }

    /// Set number of legend columns
    pub fn legend_columns(mut self, columns: usize) -> Self {
        self.plot.layout.legend.columns = Some(columns.max(1));
        self
    }

    /// Enable/disable grid
    pub fn grid(mut self, enabled: bool) -> Self {
        self.plot.layout.grid_style.visible = enabled;
        self
    }

    /// Enable or disable Typst text rendering mode.
    ///
    /// Requires the `typst-math` feature.
    /// If your crate makes Typst optional, guard this call with
    /// `#[cfg(feature = "typst-math")]`.
    #[cfg(feature = "typst-math")]
    #[cfg_attr(docsrs, doc(cfg(feature = "typst-math")))]
    pub fn typst(mut self, enabled: bool) -> Self {
        self.plot = self.plot.typst(enabled);
        self
    }

    /// Set DPI for export quality
    pub fn dpi(mut self, dpi: u32) -> Self {
        self.plot.display.config.figure.dpi = dpi.max(72) as f32;
        self.plot.display.dpi = dpi.max(72);
        // Update dimensions to reflect new DPI
        let (w, h) = self.plot.display.config.canvas_size();
        self.plot.display.dimensions = (w, h);
        self
    }

    /// Set maximum output resolution while preserving figure aspect ratio
    ///
    /// See [`Plot::max_resolution`] for details.
    pub fn max_resolution(mut self, max_width: u32, max_height: u32) -> Self {
        self.plot = self.plot.max_resolution(max_width, max_height);
        self
    }

    /// Render the plot
    pub fn render(self) -> Result<Image> {
        self.end_series().render()
    }

    /// Render the plot to PNG bytes
    pub fn render_png_bytes(self) -> Result<Vec<u8>> {
        self.end_series().render_png_bytes()
    }

    /// Save the plot to file
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save<P: AsRef<Path>>(self, path: P) -> Result<()> {
        self.end_series().save(path)
    }

    /// Save the plot to file with custom dimensions
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_with_size<P: AsRef<Path>>(
        mut self,
        path: P,
        width: u32,
        height: u32,
    ) -> Result<()> {
        self.plot = self.plot.set_output_pixels(width, height);
        self.end_series().save(path)
    }

    /// Export to SVG
    #[cfg(not(target_arch = "wasm32"))]
    pub fn export_svg<P: AsRef<Path>>(self, path: P) -> Result<()> {
        self.end_series().export_svg(path)
    }

    /// Render to SVG string
    pub fn render_to_svg(self) -> Result<String> {
        self.end_series().render_to_svg()
    }

    /// Export to PDF (requires `pdf` feature)
    #[cfg(all(feature = "pdf", not(target_arch = "wasm32")))]
    pub fn save_pdf<P: AsRef<Path>>(self, path: P) -> Result<()> {
        self.end_series().save_pdf(path)
    }

    /// Export to PDF with custom size (requires `pdf` feature)
    #[cfg(all(feature = "pdf", not(target_arch = "wasm32")))]
    pub fn save_pdf_with_size<P: AsRef<Path>>(
        self,
        path: P,
        size: Option<(f64, f64)>,
    ) -> Result<()> {
        self.end_series().save_pdf_with_size(path, size)
    }

    /// Infer and store a backend label (fluent API)
    /// Note: This ends the current series before optimizing
    pub fn auto_optimize(self) -> Plot {
        self.end_series().auto_optimize()
    }

    /// Set backend explicitly (fluent API)
    /// Note: This ends the current series before setting backend
    pub fn backend(self, backend: BackendType) -> Plot {
        self.end_series().backend(backend)
    }

    /// Enable GPU acceleration for coordinate transformations
    ///
    /// When enabled, large datasets (>5K points) will use GPU compute shaders
    /// for coordinate transformation, providing significant speedups.
    #[cfg(feature = "gpu")]
    pub fn gpu(self, enabled: bool) -> Plot {
        self.end_series().gpu(enabled)
    }

    /// Get current backend name (for testing)
    pub fn get_backend_name(&self) -> &'static str {
        self.plot.get_backend_name()
    }

    // ========== Annotation Methods for PlotSeriesBuilder ==========

    /// Add a text annotation at data coordinates
    pub fn text<S: Into<String>>(mut self, x: f64, y: f64, text: S) -> Self {
        self.plot.annotations.push(Annotation::text(x, y, text));
        self
    }

    /// Add a text annotation with custom styling
    pub fn text_styled<S: Into<String>>(
        mut self,
        x: f64,
        y: f64,
        text: S,
        style: TextStyle,
    ) -> Self {
        self.plot
            .annotations
            .push(Annotation::text_styled(x, y, text, style));
        self
    }

    /// Add an arrow annotation between two points
    pub fn arrow(mut self, x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        self.plot
            .annotations
            .push(Annotation::arrow(x1, y1, x2, y2));
        self
    }

    /// Add an arrow annotation with custom styling
    pub fn arrow_styled(mut self, x1: f64, y1: f64, x2: f64, y2: f64, style: ArrowStyle) -> Self {
        self.plot
            .annotations
            .push(Annotation::arrow_styled(x1, y1, x2, y2, style));
        self
    }

    /// Add a horizontal reference line
    pub fn hline(mut self, y: f64) -> Self {
        self.plot.annotations.push(Annotation::hline(y));
        self
    }

    /// Add a horizontal reference line with custom styling
    pub fn hline_styled(mut self, y: f64, color: Color, width: f32, style: LineStyle) -> Self {
        self.plot
            .annotations
            .push(Annotation::hline_styled(y, color, width, style));
        self
    }

    /// Add a vertical reference line
    pub fn vline(mut self, x: f64) -> Self {
        self.plot.annotations.push(Annotation::vline(x));
        self
    }

    /// Add a vertical reference line with custom styling
    pub fn vline_styled(mut self, x: f64, color: Color, width: f32, style: LineStyle) -> Self {
        self.plot
            .annotations
            .push(Annotation::vline_styled(x, color, width, style));
        self
    }

    /// Add a rectangle annotation
    pub fn rect(mut self, x: f64, y: f64, width: f64, height: f64) -> Self {
        self.plot
            .annotations
            .push(Annotation::rectangle(x, y, width, height));
        self
    }

    /// Add a rectangle annotation with custom styling.
    pub fn rect_styled(
        mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        style: ShapeStyle,
    ) -> Self {
        self.plot
            .annotations
            .push(Annotation::rectangle_styled(x, y, width, height, style));
        self
    }

    /// Add a fill between two curves
    pub fn fill_between(mut self, x: &[f64], y1: &[f64], y2: &[f64]) -> Self {
        self.plot.annotations.push(Annotation::fill_between(
            x.to_vec(),
            y1.to_vec(),
            y2.to_vec(),
        ));
        self
    }

    /// Add a fill between a curve and a baseline
    pub fn fill_to_baseline(mut self, x: &[f64], y: &[f64], baseline: f64) -> Self {
        self.plot.annotations.push(Annotation::fill_to_baseline(
            x.to_vec(),
            y.to_vec(),
            baseline,
        ));
        self
    }

    /// Add a fill between two curves with custom styling.
    pub fn fill_between_styled(
        mut self,
        x: &[f64],
        y1: &[f64],
        y2: &[f64],
        style: FillStyle,
        where_positive: bool,
    ) -> Self {
        self.plot.annotations.push(Annotation::fill_between_styled(
            x.to_vec(),
            y1.to_vec(),
            y2.to_vec(),
            style,
            where_positive,
        ));
        self
    }

    /// Add a vertical span (shaded region)
    pub fn axvspan(mut self, x_min: f64, x_max: f64) -> Self {
        self.plot.annotations.push(Annotation::hspan(x_min, x_max));
        self
    }

    /// Add a horizontal span (shaded region)
    pub fn axhspan(mut self, y_min: f64, y_max: f64) -> Self {
        self.plot.annotations.push(Annotation::vspan(y_min, y_max));
        self
    }

    /// Add a generic annotation
    pub fn annotate(mut self, annotation: Annotation) -> Self {
        self.plot.annotations.push(annotation);
        self
    }

    // ========== Axis Scale Methods for PlotSeriesBuilder ==========

    /// Set X-axis scale type
    pub fn xscale(mut self, scale: AxisScale) -> Self {
        self.plot.layout.x_scale = scale;
        self
    }

    /// Set Y-axis scale type
    pub fn yscale(mut self, scale: AxisScale) -> Self {
        self.plot.layout.y_scale = scale;
        self
    }

    /// Set X-axis limits
    ///
    /// Descending bounds preserve a reversed axis direction.
    pub fn xlim(mut self, min: f64, max: f64) -> Self {
        if min != max && min.is_finite() && max.is_finite() {
            self.plot.layout.x_limits = Some((min, max));
        }
        self
    }

    /// Set Y-axis limits
    ///
    /// Descending bounds preserve a reversed axis direction.
    pub fn ylim(mut self, min: f64, max: f64) -> Self {
        if min != max && min.is_finite() && max.is_finite() {
            self.plot.layout.y_limits = Some((min, max));
        }
        self
    }
}
