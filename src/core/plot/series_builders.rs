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
    auto_palette_slot_consumed: bool,
}

impl SeriesGroupBuilder {
    pub(super) fn new(mut plot: Plot) -> Self {
        let group_id = plot.register_series_group();
        Self {
            plot,
            style: builder::SeriesStyle::default(),
            group_id,
            auto_palette_slot_consumed: false,
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
        self.style.props.color.set(color.into());
        self
    }

    /// Set a shared reactive color applied to all group member series.
    pub fn color_source<S>(mut self, color: S) -> Self
    where
        S: Into<ReactiveValue<Color>>,
    {
        self.style.props.color.set(color.into());
        self
    }

    /// Set shared line width applied to all group member series.
    pub fn line_width(mut self, width: f32) -> Self {
        self.style.props.line_width.set(width.into());
        self
    }

    /// Set a shared reactive line width applied to all group member series.
    pub fn line_width_source<S>(mut self, width: S) -> Self
    where
        S: Into<ReactiveValue<f32>>,
    {
        self.style.props.line_width.set(width.into());
        self
    }

    /// Set shared line style applied to all group member series.
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.style.props.line_style.set(style.into());
        self
    }

    /// Set a shared reactive line style applied to all group member series.
    pub fn line_style_source<S>(mut self, style: S) -> Self
    where
        S: Into<ReactiveValue<LineStyle>>,
    {
        self.style.props.line_style.set(style.into());
        self
    }

    /// Set shared alpha/transparency applied to all group member series.
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.style.props.alpha.set(alpha.into());
        self
    }

    /// Set a shared reactive alpha/transparency applied to all group member series.
    pub fn alpha_source<S>(mut self, alpha: S) -> Self
    where
        S: Into<ReactiveValue<f32>>,
    {
        self.style.props.alpha.set(alpha.into());
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

        let style = self.style.clone();
        let uses_auto_color = !self.style.props.color.is_set();
        let consume_palette_index = !uses_auto_color || !self.auto_palette_slot_consumed;

        self.plot = self.plot.add_line_series_grouped(
            PlotData::Static(x_vec),
            PlotData::Static(y_vec),
            &crate::plots::basic::LineConfig::default(),
            style,
            Some(self.group_id),
            consume_palette_index,
        );

        if uses_auto_color {
            self.auto_palette_slot_consumed = true;
        }
        self
    }

    /// Add a line series from source-backed data to the current group.
    pub fn line_source<X, Y>(mut self, x_data: X, y_data: Y) -> Self
    where
        X: IntoPlotData,
        Y: IntoPlotData,
    {
        let style = self.style.clone();
        let uses_auto_color = !self.style.props.color.is_set();
        let consume_palette_index = !uses_auto_color || !self.auto_palette_slot_consumed;

        self.plot = self.plot.add_line_series_grouped(
            x_data.into_plot_data(),
            y_data.into_plot_data(),
            &crate::plots::basic::LineConfig::default(),
            style,
            Some(self.group_id),
            consume_palette_index,
        );

        if uses_auto_color {
            self.auto_palette_slot_consumed = true;
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

        let style = self.style.clone();
        let uses_auto_color = !self.style.props.color.is_set();
        let consume_palette_index = !uses_auto_color || !self.auto_palette_slot_consumed;

        self.plot = self.plot.add_scatter_series_grouped(
            PlotData::Static(x_vec),
            PlotData::Static(y_vec),
            &crate::plots::basic::ScatterConfig::default(),
            style,
            Some(self.group_id),
            consume_palette_index,
        );

        if uses_auto_color {
            self.auto_palette_slot_consumed = true;
        }
        self
    }

    /// Add a scatter series from source-backed data to the current group.
    pub fn scatter_source<X, Y>(mut self, x_data: X, y_data: Y) -> Self
    where
        X: IntoPlotData,
        Y: IntoPlotData,
    {
        let style = self.style.clone();
        let uses_auto_color = !self.style.props.color.is_set();
        let consume_palette_index = !uses_auto_color || !self.auto_palette_slot_consumed;

        self.plot = self.plot.add_scatter_series_grouped(
            x_data.into_plot_data(),
            y_data.into_plot_data(),
            &crate::plots::basic::ScatterConfig::default(),
            style,
            Some(self.group_id),
            consume_palette_index,
        );

        if uses_auto_color {
            self.auto_palette_slot_consumed = true;
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

        let style = self.style.clone();
        let uses_auto_color = !self.style.props.color.is_set();
        let consume_palette_index = !uses_auto_color || !self.auto_palette_slot_consumed;

        self.plot = self.plot.add_bar_series_grouped(
            cat_vec,
            PlotData::Static(val_vec),
            &crate::plots::basic::BarConfig::default(),
            style,
            Some(self.group_id),
            consume_palette_index,
        );

        if uses_auto_color {
            self.auto_palette_slot_consumed = true;
        }
        self
    }

    /// Add a bar series from source-backed values to the current group.
    pub fn bar_source<S, V>(mut self, categories: &[S], values: V) -> Self
    where
        S: ToString,
        V: IntoPlotData,
    {
        let style = self.style.clone();
        let uses_auto_color = !self.style.props.color.is_set();
        let consume_palette_index = !uses_auto_color || !self.auto_palette_slot_consumed;

        self.plot = self.plot.add_bar_series_grouped(
            categories.iter().map(ToString::to_string).collect(),
            values.into_plot_data(),
            &crate::plots::basic::BarConfig::default(),
            style,
            Some(self.group_id),
            consume_palette_index,
        );

        if uses_auto_color {
            self.auto_palette_slot_consumed = true;
        }
        self
    }
}
