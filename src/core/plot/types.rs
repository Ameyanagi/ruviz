use super::*;

/// Main Plot struct - the core API entry point for creating visualizations
///
/// `Plot` provides a fluent builder interface for creating plots with multiple
/// data series, styling options, and export capabilities.
///
/// # Architecture
///
/// The Plot struct delegates to focused component managers:
/// - [`PlotConfiguration`] - Title, labels, dimensions, theme, DPI
/// - [`SeriesManager`] - Data series storage, auto-color assignment
/// - [`LayoutManager`] - Legend, grid, ticks, margins, axis limits/scales
/// - [`RenderPipeline`] - Backend selection, parallel/pooled rendering
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::prelude::*;
///
/// Plot::new()
///     .line(&x, &y)
///     .color(Color::BLUE)
///     .line_width(2.0)
///     .title("My Plot")
///     .xlabel("X")
///     .ylabel("Y")
///     .save("plot.png")?;
/// ```
///
/// # Builder Pattern
///
/// Series methods (`.line()`, `.scatter()`, `.bar()`) return a [`PlotBuilder<C>`]
/// that auto-finalizes when terminal methods (`.save()`, `.render()`) are called.
/// No explicit `.end_series()` is needed for fluent chaining, including transitions
/// into compatible series builders and styled annotation methods.
#[derive(Clone, Debug)]
pub struct Plot {
    /// Display configuration (title, labels, dimensions, theme)
    pub(super) display: PlotConfiguration,
    /// Series manager (handles all data series and auto-coloring)
    pub(super) series_mgr: SeriesManager,
    /// Layout manager (handles legend, grid, ticks, margins, axis limits)
    pub(super) layout: LayoutManager,
    /// Render pipeline (handles backend selection, parallel/pooled rendering)
    pub(super) render: RenderPipeline,
    /// Annotations (text, arrows, lines, shapes)
    pub(super) annotations: Vec<Annotation>,
    /// Null policy for dataframe-backed numeric ingestion.
    pub(super) null_policy: NullPolicy,
    /// Deferred ingestion error captured during builder-style API calls.
    pub(super) pending_ingestion_error: Option<PendingIngestionError>,
    /// Group metadata used for grouped-series legend behavior.
    pub(super) series_groups: Vec<SeriesGroupMeta>,
    /// Monotonic group ID allocator for grouped-series builder scopes.
    pub(super) next_group_id: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct SeriesGroupMeta {
    pub(super) id: usize,
    pub(super) label: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingIngestionError {
    kind: PendingIngestionErrorKind,
    additional_count: usize,
}

#[derive(Clone, Debug)]
enum PendingIngestionErrorKind {
    EmptyDataSet,
    DataLengthMismatch {
        x_len: usize,
        y_len: usize,
        series_index: Option<usize>,
    },
    DataTypeUnsupported {
        source: String,
        dtype: String,
        expected: String,
    },
    NullValueNotAllowed {
        source: String,
        column: Option<String>,
        null_count: usize,
    },
    DataExtractionFailed {
        source: String,
        message: String,
    },
    InvalidData {
        message: String,
        position: Option<usize>,
    },
    InvalidInput(String),
}

impl PendingIngestionError {
    pub(super) fn from_plotting_error(err: PlottingError) -> Self {
        let kind = match err {
            PlottingError::EmptyDataSet => PendingIngestionErrorKind::EmptyDataSet,
            PlottingError::DataLengthMismatch {
                x_len,
                y_len,
                series_index,
            } => PendingIngestionErrorKind::DataLengthMismatch {
                x_len,
                y_len,
                series_index,
            },
            PlottingError::DataTypeUnsupported {
                source,
                dtype,
                expected,
            } => PendingIngestionErrorKind::DataTypeUnsupported {
                source,
                dtype,
                expected,
            },
            PlottingError::NullValueNotAllowed {
                source,
                column,
                null_count,
            } => PendingIngestionErrorKind::NullValueNotAllowed {
                source,
                column,
                null_count,
            },
            PlottingError::DataExtractionFailed { source, message } => {
                PendingIngestionErrorKind::DataExtractionFailed { source, message }
            }
            PlottingError::InvalidData { message, position } => {
                PendingIngestionErrorKind::InvalidData { message, position }
            }
            PlottingError::InvalidInput(message) => {
                PendingIngestionErrorKind::InvalidInput(message)
            }
            other => PendingIngestionErrorKind::DataExtractionFailed {
                source: "ruviz::plot-ingestion".to_string(),
                message: other.to_string(),
            },
        };

        Self {
            kind,
            additional_count: 0,
        }
    }

    pub(super) fn record_additional_error(&mut self) {
        self.additional_count = self.additional_count.saturating_add(1);
    }

    pub(super) fn to_plotting_error(&self) -> PlottingError {
        let primary_error = match &self.kind {
            PendingIngestionErrorKind::EmptyDataSet => PlottingError::EmptyDataSet,
            PendingIngestionErrorKind::DataLengthMismatch {
                x_len,
                y_len,
                series_index,
            } => PlottingError::DataLengthMismatch {
                x_len: *x_len,
                y_len: *y_len,
                series_index: *series_index,
            },
            PendingIngestionErrorKind::DataTypeUnsupported {
                source,
                dtype,
                expected,
            } => PlottingError::DataTypeUnsupported {
                source: source.clone(),
                dtype: dtype.clone(),
                expected: expected.clone(),
            },
            PendingIngestionErrorKind::NullValueNotAllowed {
                source,
                column,
                null_count,
            } => PlottingError::NullValueNotAllowed {
                source: source.clone(),
                column: column.clone(),
                null_count: *null_count,
            },
            PendingIngestionErrorKind::DataExtractionFailed { source, message } => {
                PlottingError::DataExtractionFailed {
                    source: source.clone(),
                    message: message.clone(),
                }
            }
            PendingIngestionErrorKind::InvalidData { message, position } => {
                PlottingError::InvalidData {
                    message: message.clone(),
                    position: *position,
                }
            }
            PendingIngestionErrorKind::InvalidInput(message) => {
                PlottingError::InvalidInput(message.clone())
            }
        };

        if self.additional_count == 0 {
            return primary_error;
        }

        PlottingError::DataExtractionFailed {
            source: "ruviz::plot-ingestion".to_string(),
            message: format!(
                "{} (and {} additional ingestion error{})",
                primary_error,
                self.additional_count,
                if self.additional_count == 1 { "" } else { "s" }
            ),
        }
    }
}

/// Configuration for a single data series
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InsetAnchor {
    Auto,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    TopCenter,
    BottomCenter,
    CenterLeft,
    CenterRight,
    Center,
    Custom { x_frac: f32, y_frac: f32 },
}

/// Inset placement for non-Cartesian series rendered inside a Cartesian plot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InsetLayout {
    pub anchor: InsetAnchor,
    pub width_frac: f32,
    pub height_frac: f32,
    pub margin_pt: f32,
}

impl InsetLayout {
    pub const DEFAULT_WIDTH_FRAC: f32 = 0.32;
    pub const DEFAULT_HEIGHT_FRAC: f32 = 0.32;
    pub const DEFAULT_MARGIN_PT: f32 = 12.0;

    pub fn auto() -> Self {
        Self {
            anchor: InsetAnchor::Auto,
            width_frac: Self::DEFAULT_WIDTH_FRAC,
            height_frac: Self::DEFAULT_HEIGHT_FRAC,
            margin_pt: Self::DEFAULT_MARGIN_PT,
        }
    }

    pub(super) fn normalized(self) -> Self {
        Self {
            anchor: self.anchor,
            width_frac: self.width_frac.clamp(0.12, 0.95),
            height_frac: self.height_frac.clamp(0.12, 0.95),
            margin_pt: self.margin_pt.max(0.0),
        }
    }
}

impl Default for InsetLayout {
    fn default() -> Self {
        Self::auto()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PlotSeries {
    /// Series type
    pub(super) series_type: SeriesType,
    /// Original paired streaming source for line/scatter streaming series.
    pub(super) streaming_source: Option<StreamingXY>,
    /// Series label for legend
    pub(super) label: Option<String>,
    /// Series color (None for auto-color)
    pub(super) color: Option<Color>,
    /// Reactive series color sampled at render time.
    pub(super) color_source: Option<ReactiveValue<Color>>,
    /// Line width override
    pub(super) line_width: Option<f32>,
    /// Reactive line width sampled at render time.
    pub(super) line_width_source: Option<ReactiveValue<f32>>,
    /// Line style override
    pub(super) line_style: Option<LineStyle>,
    /// Reactive line style sampled at render time.
    pub(super) line_style_source: Option<ReactiveValue<LineStyle>>,
    /// Marker style for scatter plots
    pub(super) marker_style: Option<MarkerStyle>,
    /// Reactive marker style sampled at render time.
    pub(super) marker_style_source: Option<ReactiveValue<MarkerStyle>>,
    /// Marker size for scatter plots
    pub(super) marker_size: Option<f32>,
    /// Reactive marker size sampled at render time.
    pub(super) marker_size_source: Option<ReactiveValue<f32>>,
    /// Alpha/transparency override
    pub(super) alpha: Option<f32>,
    /// Reactive alpha sampled at render time.
    pub(super) alpha_source: Option<ReactiveValue<f32>>,
    /// Optional Y error bar data (attached to series)
    pub(super) y_errors: Option<ErrorValues>,
    /// Optional X error bar data (attached to series)
    pub(super) x_errors: Option<ErrorValues>,
    /// Error bar configuration (cap size, line width, etc.)
    pub(super) error_config: Option<ErrorBarConfig>,
    /// Inset placement for non-Cartesian series in mixed plots.
    pub(super) inset_layout: Option<InsetLayout>,
    /// Optional group ID if this series was created inside `Plot::group(...)`.
    pub(super) group_id: Option<usize>,
}

impl PlotSeries {
    pub(super) fn set_color_source_value(&mut self, color: ReactiveValue<Color>) {
        match color {
            ReactiveValue::Static(color) => {
                self.color = Some(color);
                self.color_source = None;
            }
            source => {
                self.color = None;
                self.color_source = Some(source);
            }
        }
    }

    pub(super) fn set_line_width_source_value(&mut self, width: ReactiveValue<f32>) {
        match width {
            ReactiveValue::Static(width) => {
                self.line_width = Some(width.max(0.1));
                self.line_width_source = None;
            }
            source => {
                self.line_width = None;
                self.line_width_source = Some(source);
            }
        }
    }

    pub(super) fn set_line_style_source_value(&mut self, style: ReactiveValue<LineStyle>) {
        match style {
            ReactiveValue::Static(style) => {
                self.line_style = Some(style);
                self.line_style_source = None;
            }
            source => {
                self.line_style = None;
                self.line_style_source = Some(source);
            }
        }
    }

    pub(super) fn set_marker_style_source_value(&mut self, marker: ReactiveValue<MarkerStyle>) {
        match marker {
            ReactiveValue::Static(marker) => {
                self.marker_style = Some(marker);
                self.marker_style_source = None;
            }
            source => {
                self.marker_style = None;
                self.marker_style_source = Some(source);
            }
        }
    }

    pub(super) fn set_marker_size_source_value(&mut self, size: ReactiveValue<f32>) {
        match size {
            ReactiveValue::Static(size) => {
                self.marker_size = Some(size.max(0.1));
                self.marker_size_source = None;
            }
            source => {
                self.marker_size = None;
                self.marker_size_source = Some(source);
            }
        }
    }

    pub(super) fn set_alpha_source_value(&mut self, alpha: ReactiveValue<f32>) {
        match alpha {
            ReactiveValue::Static(alpha) => {
                self.alpha = Some(alpha.clamp(0.0, 1.0));
                self.alpha_source = None;
            }
            source => {
                self.alpha = None;
                self.alpha_source = Some(source);
            }
        }
    }

    pub(super) fn to_legend_item_with_label(
        &self,
        label: String,
        default_color: Color,
        theme: &Theme,
    ) -> Option<LegendItem> {
        if label.is_empty() {
            return None;
        }

        self.build_legend_item(label, default_color, theme)
    }

    /// Create a LegendItem from this series
    ///
    /// Returns None if the series has no label
    pub(super) fn to_legend_item(&self, default_color: Color, theme: &Theme) -> Option<LegendItem> {
        let label = self.label.as_ref()?;
        self.build_legend_item(label.clone(), default_color, theme)
    }

    pub(super) fn build_legend_item(
        &self,
        label: String,
        default_color: Color,
        theme: &Theme,
    ) -> Option<LegendItem> {
        let color = self.resolved_color(default_color, 0.0);
        let line_width = self.resolved_line_width(theme.line_width, 0.0);
        let line_style = self.resolved_line_style(LineStyle::Solid, 0.0);
        let marker_style = self.resolved_marker_style(MarkerStyle::Circle, 0.0);
        let marker_size = self.resolved_marker_size(6.0, 0.0);

        let item_type = match &self.series_type {
            SeriesType::Line { .. } => {
                if self.marker_style.is_some() || self.marker_style_source.is_some() {
                    LegendItemType::LineMarker {
                        line_style,
                        line_width,
                        marker: marker_style,
                        marker_size,
                    }
                } else {
                    LegendItemType::Line {
                        style: line_style,
                        width: line_width,
                    }
                }
            }
            SeriesType::Scatter { .. } => LegendItemType::Scatter {
                marker: marker_style,
                size: marker_size,
            },
            SeriesType::Bar { .. } => LegendItemType::Bar,
            SeriesType::ErrorBars { .. } | SeriesType::ErrorBarsXY { .. } => {
                LegendItemType::ErrorBar
            }
            SeriesType::Histogram { .. } => LegendItemType::Histogram,
            SeriesType::BoxPlot { .. } => LegendItemType::Bar,
            SeriesType::Heatmap { .. } => return None,
            SeriesType::Kde { .. }
            | SeriesType::Ecdf { .. }
            | SeriesType::Polar { .. }
            | SeriesType::Quiver { .. } => LegendItemType::Line {
                style: line_style,
                width: line_width,
            },
            SeriesType::Violin { .. } | SeriesType::Boxen { .. } | SeriesType::Pie { .. } => {
                LegendItemType::Bar
            }
            SeriesType::Contour { .. } => return None,
            SeriesType::Radar { .. } => LegendItemType::Area {
                edge_color: Some(color),
            },
        };

        let has_error_bars = self.y_errors.is_some() || self.x_errors.is_some();

        Some(LegendItem {
            label,
            color,
            item_type,
            has_error_bars,
        })
    }

    /// Create multiple LegendItems from this series (for Radar series that contain multiple internal series)
    ///
    /// Returns a Vec of legend items, expanding radar series into individual entries per data series.
    pub(super) fn to_legend_items(&self, base_color_idx: usize, theme: &Theme) -> Vec<LegendItem> {
        let line_width = self.resolved_line_width(theme.line_width, 0.0);
        let line_style = self.resolved_line_style(LineStyle::Solid, 0.0);

        match &self.series_type {
            SeriesType::Radar { data } => {
                // For radar charts, create a legend item for each internal series
                // Use Area type to show filled swatches matching the filled polygon style
                data.series
                    .iter()
                    .enumerate()
                    .map(|(idx, radar_series)| {
                        let color = data
                            .config
                            .colors
                            .as_ref()
                            .and_then(|colors| colors.get(idx).copied())
                            .unwrap_or_else(|| theme.get_color(base_color_idx + idx));
                        // Use a more visible alpha for legend swatches (0.6 instead of fill_alpha)
                        // This ensures legend items are clearly visible while still showing fill style
                        let fill_color = color.with_alpha(0.6);
                        LegendItem {
                            label: radar_series.label.clone(),
                            color: fill_color,
                            item_type: LegendItemType::Area {
                                edge_color: Some(color), // Use full-opacity color for edge
                            },
                            has_error_bars: false,
                        }
                    })
                    .collect()
            }
            _ => {
                // For other series types, use the single-item method
                let default_color = theme.get_color(base_color_idx);
                self.to_legend_item(default_color, theme)
                    .into_iter()
                    .collect()
            }
        }
    }

    pub(super) fn collect_source_versions(&self, versions: &mut Vec<u64>) {
        if let Some(stream) = &self.streaming_source {
            stream.refresh_legacy_lanes();
            versions.push(stream.version());
            versions.push(stream.x().version());
            versions.push(stream.y().version());
        } else {
            self.series_type.collect_source_versions(versions);
        }
        if let Some(version) = self
            .color_source
            .as_ref()
            .and_then(ReactiveValue::current_version)
        {
            versions.push(version);
        }
        if let Some(version) = self
            .line_width_source
            .as_ref()
            .and_then(ReactiveValue::current_version)
        {
            versions.push(version);
        }
        if let Some(version) = self
            .line_style_source
            .as_ref()
            .and_then(ReactiveValue::current_version)
        {
            versions.push(version);
        }
        if let Some(version) = self
            .marker_style_source
            .as_ref()
            .and_then(ReactiveValue::current_version)
        {
            versions.push(version);
        }
        if let Some(version) = self
            .marker_size_source
            .as_ref()
            .and_then(ReactiveValue::current_version)
        {
            versions.push(version);
        }
        if let Some(version) = self
            .alpha_source
            .as_ref()
            .and_then(ReactiveValue::current_version)
        {
            versions.push(version);
        }
    }

    pub(super) fn is_reactive(&self) -> bool {
        self.series_type.is_reactive()
            || self
                .color_source
                .as_ref()
                .is_some_and(ReactiveValue::is_reactive)
            || self
                .line_width_source
                .as_ref()
                .is_some_and(ReactiveValue::is_reactive)
            || self
                .line_style_source
                .as_ref()
                .is_some_and(ReactiveValue::is_reactive)
            || self
                .marker_style_source
                .as_ref()
                .is_some_and(ReactiveValue::is_reactive)
            || self
                .marker_size_source
                .as_ref()
                .is_some_and(ReactiveValue::is_reactive)
            || self
                .alpha_source
                .as_ref()
                .is_some_and(ReactiveValue::is_reactive)
    }

    pub(super) fn has_temporal_sources(&self) -> bool {
        self.series_type.has_temporal_sources()
            || self
                .color_source
                .as_ref()
                .is_some_and(ReactiveValue::is_temporal)
            || self
                .line_width_source
                .as_ref()
                .is_some_and(ReactiveValue::is_temporal)
            || self
                .line_style_source
                .as_ref()
                .is_some_and(ReactiveValue::is_temporal)
            || self
                .marker_style_source
                .as_ref()
                .is_some_and(ReactiveValue::is_temporal)
            || self
                .marker_size_source
                .as_ref()
                .is_some_and(ReactiveValue::is_temporal)
            || self
                .alpha_source
                .as_ref()
                .is_some_and(ReactiveValue::is_temporal)
    }

    pub(super) fn has_reactive_style_sources(&self) -> bool {
        self.color_source
            .as_ref()
            .is_some_and(ReactiveValue::is_reactive)
            || self
                .line_width_source
                .as_ref()
                .is_some_and(ReactiveValue::is_reactive)
            || self
                .line_style_source
                .as_ref()
                .is_some_and(ReactiveValue::is_reactive)
            || self
                .marker_style_source
                .as_ref()
                .is_some_and(ReactiveValue::is_reactive)
            || self
                .marker_size_source
                .as_ref()
                .is_some_and(ReactiveValue::is_reactive)
            || self
                .alpha_source
                .as_ref()
                .is_some_and(ReactiveValue::is_reactive)
    }

    pub(super) fn clone_for_resolved_frame(&self) -> Self {
        Self {
            series_type: self.series_type.clone_without_static_values(),
            streaming_source: self.streaming_source.clone(),
            label: self.label.clone(),
            color: self.color,
            color_source: self.color_source.clone(),
            line_width: self.line_width,
            line_width_source: self.line_width_source.clone(),
            line_style: self.line_style.clone(),
            line_style_source: self.line_style_source.clone(),
            marker_style: self.marker_style,
            marker_style_source: self.marker_style_source.clone(),
            marker_size: self.marker_size,
            marker_size_source: self.marker_size_source.clone(),
            alpha: self.alpha,
            alpha_source: self.alpha_source.clone(),
            y_errors: self.y_errors.clone(),
            x_errors: self.x_errors.clone(),
            error_config: self.error_config.clone(),
            inset_layout: self.inset_layout,
            group_id: self.group_id,
        }
    }

    pub(super) fn mark_rendered_sources(&self) {
        if let Some(stream) = &self.streaming_source {
            stream.mark_rendered();
        } else {
            self.series_type.mark_rendered_sources();
        }
    }

    pub(super) fn mark_unpaired_streaming_sources_rendered(&self) {
        if self.streaming_source.is_none() {
            self.series_type.mark_rendered_sources();
        }
    }

    pub(super) fn has_live_streaming_pair(&self) -> bool {
        self.streaming_source.is_some()
            && matches!(
                &self.series_type,
                SeriesType::Line { x_data, y_data }
                    | SeriesType::Scatter { x_data, y_data }
                    if matches!(x_data, PlotData::Streaming(_))
                        && matches!(y_data, PlotData::Streaming(_))
            )
    }

    pub(super) fn resolve_for_render(&self, time: f64) -> Result<ResolvedSeries<'_>> {
        if self.has_live_streaming_pair() {
            let snapshot = self
                .streaming_source
                .as_ref()
                .expect("live streaming pair must retain its source")
                .snapshot();
            return Ok(match &self.series_type {
                SeriesType::Line { .. } => ResolvedSeries::Line {
                    x: ResolvedData::shared(Arc::from(snapshot.x())),
                    y: ResolvedData::shared(Arc::from(snapshot.y())),
                },
                SeriesType::Scatter { .. } => ResolvedSeries::Scatter {
                    x: ResolvedData::shared(Arc::from(snapshot.x())),
                    y: ResolvedData::shared(Arc::from(snapshot.y())),
                },
                _ => unreachable!("live paired source is only used by line/scatter"),
            });
        }
        self.series_type.resolve_for_render(time)
    }

    pub(super) fn subscribe_push_updates(
        &self,
        callback: &SharedReactiveCallback,
        teardowns: &mut Vec<ReactiveTeardown>,
    ) {
        if let Some(stream) = &self.streaming_source {
            let stream = stream.clone();
            let callback = Arc::clone(callback);
            stream.refresh_legacy_lanes();
            let observed_versions = Arc::new(std::sync::Mutex::new((
                stream.version(),
                stream.x().version(),
                stream.y().version(),
            )));
            let make_callback = |stream: crate::data::StreamingXY| {
                let callback = Arc::clone(&callback);
                let observed_versions = Arc::clone(&observed_versions);
                move || {
                    stream.refresh_legacy_lanes();
                    let current = (stream.version(), stream.x().version(), stream.y().version());
                    let mut observed = observed_versions
                        .lock()
                        .expect("Streaming subscription version lock poisoned");
                    if *observed != current {
                        *observed = current;
                        drop(observed);
                        callback();
                    }
                }
            };
            let paired_id = stream.subscribe_paired(make_callback(stream.clone()));
            let x_id = stream.x().subscribe(make_callback(stream.clone()));
            let y_id = stream.y().subscribe(make_callback(stream.clone()));
            teardowns.push(Box::new(move || {
                stream.unsubscribe_paired(paired_id);
                stream.x().unsubscribe(x_id);
                stream.y().unsubscribe(y_id);
            }));
        } else {
            self.series_type.subscribe_push_updates(callback, teardowns);
        }
        if let Some(color_source) = &self.color_source {
            color_source.subscribe_push_updates(Arc::clone(callback), teardowns);
        }
        if let Some(line_width_source) = &self.line_width_source {
            line_width_source.subscribe_push_updates(Arc::clone(callback), teardowns);
        }
        if let Some(line_style_source) = &self.line_style_source {
            line_style_source.subscribe_push_updates(Arc::clone(callback), teardowns);
        }
        if let Some(marker_style_source) = &self.marker_style_source {
            marker_style_source.subscribe_push_updates(Arc::clone(callback), teardowns);
        }
        if let Some(marker_size_source) = &self.marker_size_source {
            marker_size_source.subscribe_push_updates(Arc::clone(callback), teardowns);
        }
        if let Some(alpha_source) = &self.alpha_source {
            alpha_source.subscribe_push_updates(Arc::clone(callback), teardowns);
        }
    }

    pub(super) fn resolved_color(&self, default_color: Color, time: f64) -> Color {
        self.color_source
            .as_ref()
            .map(|source| source.resolve(time))
            .or(self.color)
            .unwrap_or(default_color)
    }

    pub(super) fn resolved_line_width(&self, default_width: f32, time: f64) -> f32 {
        self.line_width_source
            .as_ref()
            .map(|source| source.resolve(time))
            .or(self.line_width)
            .unwrap_or(default_width)
            .max(0.1)
    }

    pub(super) fn resolved_line_style(&self, default_style: LineStyle, time: f64) -> LineStyle {
        self.line_style_source
            .as_ref()
            .map(|source| source.resolve(time))
            .or_else(|| self.line_style.clone())
            .unwrap_or(default_style)
    }

    pub(super) fn resolved_marker_style(
        &self,
        default_style: MarkerStyle,
        time: f64,
    ) -> MarkerStyle {
        self.marker_style_source
            .as_ref()
            .map(|source| source.resolve(time))
            .or(self.marker_style)
            .unwrap_or(default_style)
    }

    pub(super) fn resolved_marker_size(&self, default_size: f32, time: f64) -> f32 {
        self.marker_size_source
            .as_ref()
            .map(|source| source.resolve(time))
            .or(self.marker_size)
            .unwrap_or(default_size)
            .max(0.1)
    }

    pub(super) fn resolved_alpha(&self, default_alpha: f32, time: f64) -> f32 {
        self.alpha_source
            .as_ref()
            .map(|source| source.resolve(time))
            .or(self.alpha)
            .unwrap_or(default_alpha)
            .clamp(0.0, 1.0)
    }

    pub(super) fn resolve_style_sources(&mut self, time: f64) {
        if let Some(color_source) = &self.color_source {
            self.color = Some(color_source.resolve(time));
        }
        self.color_source = None;
        if let Some(line_width_source) = &self.line_width_source {
            self.line_width = Some(line_width_source.resolve(time).max(0.1));
        }
        self.line_width_source = None;
        if let Some(line_style_source) = &self.line_style_source {
            self.line_style = Some(line_style_source.resolve(time));
        }
        self.line_style_source = None;
        if let Some(marker_style_source) = &self.marker_style_source {
            self.marker_style = Some(marker_style_source.resolve(time));
        }
        self.marker_style_source = None;
        if let Some(marker_size_source) = &self.marker_size_source {
            self.marker_size = Some(marker_size_source.resolve(time).max(0.1));
        }
        self.marker_size_source = None;
        if let Some(alpha_source) = &self.alpha_source {
            self.alpha = Some(alpha_source.resolve(time).clamp(0.0, 1.0));
        }
        self.alpha_source = None;
    }
}

/// Types of plot series
#[derive(Clone, Debug)]
pub(crate) enum SeriesType {
    Line {
        x_data: PlotData,
        y_data: PlotData,
    },
    Scatter {
        x_data: PlotData,
        y_data: PlotData,
    },
    Bar {
        categories: Vec<String>,
        values: PlotData,
    },
    ErrorBars {
        x_data: PlotData,
        y_data: PlotData,
        y_errors: PlotData,
    },
    ErrorBarsXY {
        x_data: PlotData,
        y_data: PlotData,
        x_errors: PlotData,
        y_errors: PlotData,
    },
    Histogram {
        data: PlotData,
        config: crate::plots::histogram::HistogramConfig,
        prepared: Option<crate::plots::histogram::HistogramData>,
    },
    BoxPlot {
        data: PlotData,
        config: crate::plots::boxplot::BoxPlotConfig,
    },
    Heatmap {
        data: crate::plots::heatmap::HeatmapData,
    },
    /// KDE (Kernel Density Estimation) plot
    Kde {
        data: crate::plots::KdeData,
    },
    /// ECDF (Empirical Cumulative Distribution Function) plot
    Ecdf {
        data: crate::plots::EcdfData,
    },
    /// Violin plot
    Violin {
        data: crate::plots::ViolinData,
    },
    /// Boxen (Letter-Value) plot
    Boxen {
        data: crate::plots::BoxenData,
    },
    /// Contour plot
    Contour {
        data: crate::plots::continuous::contour::ContourPlotData,
    },
    /// Pie chart
    Pie {
        data: crate::plots::composition::pie::PieData,
    },
    /// Radar chart
    Radar {
        data: crate::plots::polar::radar::RadarPlotData,
    },
    /// Polar plot
    Polar {
        data: crate::plots::polar::polar_plot::PolarPlotData,
    },
    /// Quiver vector field plot
    Quiver {
        data: crate::plots::QuiverPlotData,
    },
}

impl SeriesType {
    pub(crate) fn supports_interactive_surface_fast_path(&self) -> bool {
        matches!(
            self,
            SeriesType::Line { .. } | SeriesType::Scatter { .. } | SeriesType::Heatmap { .. }
        )
    }

    /// Check if this series contains any reactive data
    pub fn is_reactive(&self) -> bool {
        match self {
            SeriesType::Line { x_data, y_data } => x_data.is_reactive() || y_data.is_reactive(),
            SeriesType::Scatter { x_data, y_data } => x_data.is_reactive() || y_data.is_reactive(),
            SeriesType::Bar { values, .. } => values.is_reactive(),
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => x_data.is_reactive() || y_data.is_reactive() || y_errors.is_reactive(),
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => {
                x_data.is_reactive()
                    || y_data.is_reactive()
                    || x_errors.is_reactive()
                    || y_errors.is_reactive()
            }
            SeriesType::Histogram { data, .. } => data.is_reactive(),
            SeriesType::BoxPlot { data, .. } => data.is_reactive(),
            // Other types use their own data structures, not PlotData
            _ => false,
        }
    }

    pub(super) fn has_temporal_sources(&self) -> bool {
        match self {
            SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                x_data.is_temporal() || y_data.is_temporal()
            }
            SeriesType::Bar { values, .. } => values.is_temporal(),
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => x_data.is_temporal() || y_data.is_temporal() || y_errors.is_temporal(),
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => {
                x_data.is_temporal()
                    || y_data.is_temporal()
                    || x_errors.is_temporal()
                    || y_errors.is_temporal()
            }
            SeriesType::Histogram { data, .. } | SeriesType::BoxPlot { data, .. } => {
                data.is_temporal()
            }
            _ => false,
        }
    }

    pub(super) fn collect_source_versions(&self, versions: &mut Vec<u64>) {
        let push = |data: &PlotData, versions: &mut Vec<u64>| {
            if let Some(version) = data.current_version() {
                versions.push(version);
            }
        };

        match self {
            SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                push(x_data, versions);
                push(y_data, versions);
            }
            SeriesType::Bar { values, .. } => push(values, versions),
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => {
                push(x_data, versions);
                push(y_data, versions);
                push(y_errors, versions);
            }
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => {
                push(x_data, versions);
                push(y_data, versions);
                push(x_errors, versions);
                push(y_errors, versions);
            }
            SeriesType::Histogram { data, .. } | SeriesType::BoxPlot { data, .. } => {
                push(data, versions);
            }
            _ => {}
        }
    }

    pub(super) fn mark_rendered_sources(&self) {
        let mark = |data: &PlotData| data.mark_rendered();

        match self {
            SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                mark(x_data);
                mark(y_data);
            }
            SeriesType::Bar { values, .. } => mark(values),
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => {
                mark(x_data);
                mark(y_data);
                mark(y_errors);
            }
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => {
                mark(x_data);
                mark(y_data);
                mark(x_errors);
                mark(y_errors);
            }
            SeriesType::Histogram { data, .. } | SeriesType::BoxPlot { data, .. } => {
                mark(data);
            }
            _ => {}
        }
    }

    pub(super) fn subscribe_push_updates(
        &self,
        callback: &SharedReactiveCallback,
        teardowns: &mut Vec<ReactiveTeardown>,
    ) {
        let subscribe = |data: &PlotData, teardowns: &mut Vec<ReactiveTeardown>| {
            data.subscribe_push_updates(Arc::clone(callback), teardowns);
        };

        match self {
            SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                subscribe(x_data, teardowns);
                subscribe(y_data, teardowns);
            }
            SeriesType::Bar { values, .. } => subscribe(values, teardowns),
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => {
                subscribe(x_data, teardowns);
                subscribe(y_data, teardowns);
                subscribe(y_errors, teardowns);
            }
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => {
                subscribe(x_data, teardowns);
                subscribe(y_data, teardowns);
                subscribe(x_errors, teardowns);
                subscribe(y_errors, teardowns);
            }
            SeriesType::Histogram { data, .. } | SeriesType::BoxPlot { data, .. } => {
                subscribe(data, teardowns);
            }
            _ => {}
        }
    }

    pub(super) fn clone_without_static_values(&self) -> SeriesType {
        match self {
            SeriesType::Line { x_data, y_data } => SeriesType::Line {
                x_data: x_data.clone_without_static_values(),
                y_data: y_data.clone_without_static_values(),
            },
            SeriesType::Scatter { x_data, y_data } => SeriesType::Scatter {
                x_data: x_data.clone_without_static_values(),
                y_data: y_data.clone_without_static_values(),
            },
            SeriesType::Bar { categories, values } => SeriesType::Bar {
                categories: categories.clone(),
                values: values.clone_without_static_values(),
            },
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => SeriesType::ErrorBars {
                x_data: x_data.clone_without_static_values(),
                y_data: y_data.clone_without_static_values(),
                y_errors: y_errors.clone_without_static_values(),
            },
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => SeriesType::ErrorBarsXY {
                x_data: x_data.clone_without_static_values(),
                y_data: y_data.clone_without_static_values(),
                x_errors: x_errors.clone_without_static_values(),
                y_errors: y_errors.clone_without_static_values(),
            },
            SeriesType::Histogram {
                data,
                config,
                prepared,
            } => SeriesType::Histogram {
                data: data.clone_without_static_values(),
                config: config.clone(),
                prepared: prepared.clone(),
            },
            SeriesType::BoxPlot { data, config } => SeriesType::BoxPlot {
                data: data.clone_without_static_values(),
                config: config.clone(),
            },
            other => other.clone(),
        }
    }

    /// Resolve all PlotData in this series to static Vec<f64> at the given time
    ///
    /// Returns a new SeriesType with all PlotData converted to PlotData::Static
    pub fn resolve(&self, time: f64) -> SeriesType {
        match self {
            SeriesType::Line { x_data, y_data } => SeriesType::Line {
                x_data: PlotData::Static(x_data.resolve(time)),
                y_data: PlotData::Static(y_data.resolve(time)),
            },
            SeriesType::Scatter { x_data, y_data } => SeriesType::Scatter {
                x_data: PlotData::Static(x_data.resolve(time)),
                y_data: PlotData::Static(y_data.resolve(time)),
            },
            SeriesType::Bar { categories, values } => SeriesType::Bar {
                categories: categories.clone(),
                values: PlotData::Static(values.resolve(time)),
            },
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => SeriesType::ErrorBars {
                x_data: PlotData::Static(x_data.resolve(time)),
                y_data: PlotData::Static(y_data.resolve(time)),
                y_errors: PlotData::Static(y_errors.resolve(time)),
            },
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => SeriesType::ErrorBarsXY {
                x_data: PlotData::Static(x_data.resolve(time)),
                y_data: PlotData::Static(y_data.resolve(time)),
                x_errors: PlotData::Static(x_errors.resolve(time)),
                y_errors: PlotData::Static(y_errors.resolve(time)),
            },
            SeriesType::Histogram {
                data,
                config,
                prepared,
            } => {
                let resolved_data = data.resolve(time);
                let prepared = prepared.clone().or_else(|| {
                    crate::plots::histogram::calculate_histogram(&resolved_data, config).ok()
                });
                SeriesType::Histogram {
                    data: PlotData::Static(resolved_data),
                    config: config.clone(),
                    prepared,
                }
            }
            SeriesType::BoxPlot { data, config } => SeriesType::BoxPlot {
                data: PlotData::Static(data.resolve(time)),
                config: config.clone(),
            },
            // Other types don't use PlotData - clone as-is
            other => other.clone(),
        }
    }

    /// Get resolved x_data as Vec<f64> for series that carry x-data.
    #[inline]
    pub fn try_x_data_resolved(&self, time: f64) -> Option<Vec<f64>> {
        match self {
            SeriesType::Line { x_data, .. }
            | SeriesType::Scatter { x_data, .. }
            | SeriesType::ErrorBars { x_data, .. }
            | SeriesType::ErrorBarsXY { x_data, .. } => Some(x_data.resolve(time)),
            _ => None,
        }
    }

    /// Get resolved y_data as Vec<f64> for series that carry y-data.
    #[inline]
    pub fn try_y_data_resolved(&self, time: f64) -> Option<Vec<f64>> {
        match self {
            SeriesType::Line { y_data, .. }
            | SeriesType::Scatter { y_data, .. }
            | SeriesType::ErrorBars { y_data, .. }
            | SeriesType::ErrorBarsXY { y_data, .. } => Some(y_data.resolve(time)),
            _ => None,
        }
    }

    /// Get resolved x_data as Vec<f64> (panics if not Line/Scatter/ErrorBars).
    #[deprecated(note = "Use try_x_data_resolved() for non-panicking behavior")]
    #[inline]
    pub fn x_data_resolved(&self, time: f64) -> Vec<f64> {
        self.try_x_data_resolved(time)
            .expect("x_data not available for this series type")
    }

    /// Get resolved y_data as Vec<f64> (panics if not Line/Scatter/ErrorBars).
    #[deprecated(note = "Use try_y_data_resolved() for non-panicking behavior")]
    #[inline]
    pub fn y_data_resolved(&self, time: f64) -> Vec<f64> {
        self.try_y_data_resolved(time)
            .expect("y_data not available for this series type")
    }

    pub(super) fn histogram_data_at(
        &self,
        time: f64,
    ) -> Result<crate::plots::histogram::HistogramData> {
        match self {
            SeriesType::Histogram {
                data,
                config,
                prepared,
            } => {
                let resolved = data.resolve_cow(time);
                if resolved.is_empty() {
                    return Err(PlottingError::EmptyDataSet);
                }
                PlottingError::validate_data(resolved.as_ref())?;

                if let Some(prepared) = prepared {
                    return Ok(prepared.clone());
                }

                crate::plots::histogram::calculate_histogram(&resolved.as_ref(), config).map_err(
                    |error| {
                        PlottingError::RenderError(format!("Histogram calculation failed: {error}"))
                    },
                )
            }
            _ => Err(PlottingError::RenderError(
                "histogram_data_at called for non-histogram series".to_string(),
            )),
        }
    }
}

#[derive(Clone)]
pub(crate) enum ResolvedData<'a> {
    Cow(Cow<'a, [f64]>),
    Shared(Arc<[f64]>),
}

impl<'a> ResolvedData<'a> {
    fn from_cow(data: Cow<'a, [f64]>) -> Self {
        match data {
            Cow::Borrowed(data) => Self::Cow(Cow::Borrowed(data)),
            Cow::Owned(data) => Self::Shared(Arc::from(data)),
        }
    }

    pub(super) fn shared(data: Arc<[f64]>) -> Self {
        Self::Shared(data)
    }

    pub(super) fn shared_arc(&self) -> Option<Arc<[f64]>> {
        match self {
            Self::Cow(Cow::Borrowed(_)) => None,
            Self::Cow(Cow::Owned(data)) => Some(Arc::from(data.clone())),
            Self::Shared(data) => Some(Arc::clone(data)),
        }
    }
}

impl AsRef<[f64]> for ResolvedData<'_> {
    fn as_ref(&self) -> &[f64] {
        match self {
            Self::Cow(data) => data.as_ref(),
            Self::Shared(data) => data.as_ref(),
        }
    }
}

impl std::ops::Deref for ResolvedData<'_> {
    type Target = [f64];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

#[derive(Clone)]
pub(crate) enum ResolvedSeries<'a> {
    Line {
        x: ResolvedData<'a>,
        y: ResolvedData<'a>,
    },
    Scatter {
        x: ResolvedData<'a>,
        y: ResolvedData<'a>,
    },
    Bar {
        categories: &'a [String],
        values: ResolvedData<'a>,
    },
    ErrorBars {
        x: ResolvedData<'a>,
        y: ResolvedData<'a>,
        y_errors: ResolvedData<'a>,
    },
    ErrorBarsXY {
        x: ResolvedData<'a>,
        y: ResolvedData<'a>,
        x_errors: ResolvedData<'a>,
        y_errors: ResolvedData<'a>,
    },
    Histogram {
        data: crate::plots::histogram::HistogramData,
    },
    BoxPlot {
        data: ResolvedData<'a>,
        config: BoxPlotConfig,
    },
    Other(&'a SeriesType),
}

pub(crate) struct ResolvedStreamingPair {
    pub(super) source: StreamingXY,
    pub(super) x: Arc<[f64]>,
    pub(super) y: Arc<[f64]>,
    pub(super) sequence: u64,
    pub(super) render_state: crate::data::StreamingRenderState,
}

pub(crate) struct ResolvedFrame<'a> {
    pub(super) series: Vec<ResolvedSeries<'a>>,
    pub(super) title: Option<String>,
    pub(super) xlabel: Option<String>,
    pub(super) ylabel: Option<String>,
    pub(super) streaming_acknowledgements: Vec<crate::data::StreamingBuffer<f64>>,
    pub(super) paired_acknowledgements: Vec<ResolvedStreamingPair>,
}

impl ResolvedFrame<'_> {
    pub(super) fn acknowledge_rendered(&self, _live_plot: &Plot) {
        for stream in &self.streaming_acknowledgements {
            stream.mark_rendered();
        }
        for stream in &self.paired_acknowledgements {
            stream.source.mark_rendered_through(stream.sequence);
        }
    }
}

impl SeriesType {
    pub(super) fn resolve_for_render(&self, time: f64) -> Result<ResolvedSeries<'_>> {
        Ok(match self {
            SeriesType::Line { x_data, y_data } => ResolvedSeries::Line {
                x: ResolvedData::from_cow(x_data.resolve_cow(time)),
                y: ResolvedData::from_cow(y_data.resolve_cow(time)),
            },
            SeriesType::Scatter { x_data, y_data } => ResolvedSeries::Scatter {
                x: ResolvedData::from_cow(x_data.resolve_cow(time)),
                y: ResolvedData::from_cow(y_data.resolve_cow(time)),
            },
            SeriesType::Bar { categories, values } => ResolvedSeries::Bar {
                categories,
                values: ResolvedData::from_cow(values.resolve_cow(time)),
            },
            SeriesType::ErrorBars {
                x_data,
                y_data,
                y_errors,
            } => ResolvedSeries::ErrorBars {
                x: ResolvedData::from_cow(x_data.resolve_cow(time)),
                y: ResolvedData::from_cow(y_data.resolve_cow(time)),
                y_errors: ResolvedData::from_cow(y_errors.resolve_cow(time)),
            },
            SeriesType::ErrorBarsXY {
                x_data,
                y_data,
                x_errors,
                y_errors,
            } => ResolvedSeries::ErrorBarsXY {
                x: ResolvedData::from_cow(x_data.resolve_cow(time)),
                y: ResolvedData::from_cow(y_data.resolve_cow(time)),
                x_errors: ResolvedData::from_cow(x_errors.resolve_cow(time)),
                y_errors: ResolvedData::from_cow(y_errors.resolve_cow(time)),
            },
            SeriesType::Histogram { .. } => ResolvedSeries::Histogram {
                data: self.histogram_data_at(time)?,
            },
            SeriesType::BoxPlot { data, config } => ResolvedSeries::BoxPlot {
                data: ResolvedData::from_cow(data.resolve_cow(time)),
                config: config.clone(),
            },
            other => ResolvedSeries::Other(other),
        })
    }
}

/// Legend configuration (legacy, for backward compatibility)
#[derive(Clone, Debug)]
pub(crate) struct LegendConfig {
    /// Whether to show legend
    pub(crate) enabled: bool,
    /// Legend position
    pub(crate) position: Position,
    /// Font size override in typographic points.
    pub(crate) font_size: Option<f32>,
    /// Corner radius for rounded corners
    pub(crate) corner_radius: Option<f32>,
    /// Number of columns (1 = vertical, >1 = horizontal/multi-column)
    pub(crate) columns: Option<usize>,
}

impl Default for LegendConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            position: Position::TopRight,
            font_size: None,
            corner_radius: None,
            columns: None,
        }
    }
}

impl LegendConfig {
    /// Convert to new Legend type for rendering
    pub(super) fn to_legend(&self, default_font_size: f32) -> Legend {
        let mut legend = Legend {
            enabled: self.enabled,
            position: LegendPosition::from_position(self.position),
            font_size: self.font_size.unwrap_or(default_font_size),
            ..Legend::default()
        };
        if let Some(radius) = self.corner_radius {
            legend.style.corner_radius = radius;
        }
        if let Some(cols) = self.columns {
            legend.columns = cols;
        }
        legend
    }
}

// NOTE: GridConfig has been replaced by the unified GridStyle from core module.
// See `grid_style: GridStyle` field in Plot struct for grid configuration.

/// Tick configuration for axes
#[derive(Clone, Debug)]
pub(crate) struct TickConfig {
    /// Whether tick marks and tick labels are rendered
    pub(crate) enabled: bool,
    /// Direction ticks point (inside or outside)
    pub(crate) direction: TickDirection,
    /// Which plot borders render tick marks
    pub(crate) sides: TickSides,
    /// Number of major ticks on X axis
    pub(crate) major_ticks_x: usize,
    /// Number of minor ticks between major ticks on X axis
    pub(crate) minor_ticks_x: usize,
    /// Number of major ticks on Y axis
    pub(crate) major_ticks_y: usize,
    /// Number of minor ticks between major ticks on Y axis
    pub(crate) minor_ticks_y: usize,
    /// Grid display mode
    pub(crate) grid_mode: GridMode,
}

impl Default for TickConfig {
    fn default() -> Self {
        TickConfig {
            enabled: true,
            direction: TickDirection::Inside,
            sides: TickSides::all(),
            major_ticks_x: 10,
            minor_ticks_x: 0,
            major_ticks_y: 8,
            minor_ticks_y: 0,
            grid_mode: GridMode::MajorOnly,
        }
    }
}
