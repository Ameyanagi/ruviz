use std::path::Path;

use pyo3::{
    exceptions::{PyRuntimeError, PyTypeError, PyValueError},
    prelude::*,
    types::{PyAny, PyBytes, PyDict},
};
use ruviz::{
    axes::AxisScale,
    core::{
        IntoPlot, LegendPosition, Plot, PlotBuilder, PreparedPlot,
        plot::{IntoPlotData, PlotData},
    },
    data::Observable,
    plots::PlotConfig,
    render::{Color, LineStyle, MarkerStyle, Theme},
};

#[cfg(feature = "native-interactive")]
use ruviz::interactive::show_interactive;

#[cfg(not(feature = "native-interactive"))]
use crate::NATIVE_INTERACTIVE_UNAVAILABLE_MESSAGE;

#[derive(Clone)]
enum NumericSourceState {
    Static(Vec<f64>),
    Observable(Observable<Vec<f64>>),
}

impl NumericSourceState {
    fn len(&self) -> usize {
        match self {
            Self::Static(values) => values.len(),
            Self::Observable(values) => values.get().len(),
        }
    }

    fn into_plot_data(&self) -> PlotData {
        match self {
            Self::Static(values) => values.clone().into_plot_data(),
            Self::Observable(values) => values.clone().into_plot_data(),
        }
    }
}

/// Optional per-series styling forwarded from the Python `style` mapping.
///
/// Only the options the Python layer declares per plot kind ever reach here;
/// each field maps to one core `PlotBuilder` setter.
#[derive(Clone, Default)]
struct SeriesStyle {
    label: Option<String>,
    color: Option<Color>,
    alpha: Option<f32>,
    width: Option<f32>,
    line_style: Option<LineStyle>,
    marker: Option<MarkerStyle>,
    marker_size: Option<f32>,
    bins: Option<usize>,
    bandwidth: Option<f64>,
    levels: Option<usize>,
}

const MARKER_STYLES: [(&str, MarkerStyle); 12] = [
    ("circle", MarkerStyle::Circle),
    ("square", MarkerStyle::Square),
    ("triangle", MarkerStyle::Triangle),
    ("triangle-down", MarkerStyle::TriangleDown),
    ("diamond", MarkerStyle::Diamond),
    ("plus", MarkerStyle::Plus),
    ("cross", MarkerStyle::Cross),
    ("star", MarkerStyle::Star),
    ("circle-open", MarkerStyle::CircleOpen),
    ("square-open", MarkerStyle::SquareOpen),
    ("triangle-open", MarkerStyle::TriangleOpen),
    ("diamond-open", MarkerStyle::DiamondOpen),
];

const LINE_STYLES: [(&str, LineStyle); 5] = [
    ("solid", LineStyle::Solid),
    ("dashed", LineStyle::Dashed),
    ("dotted", LineStyle::Dotted),
    ("dash-dot", LineStyle::DashDot),
    ("dash-dot-dot", LineStyle::DashDotDot),
];

const LEGEND_POSITIONS: [(&str, LegendPosition); 15] = [
    ("best", LegendPosition::Best),
    ("upper_right", LegendPosition::UpperRight),
    ("upper_left", LegendPosition::UpperLeft),
    ("lower_left", LegendPosition::LowerLeft),
    ("lower_right", LegendPosition::LowerRight),
    ("right", LegendPosition::Right),
    ("center_left", LegendPosition::CenterLeft),
    ("center_right", LegendPosition::CenterRight),
    ("lower_center", LegendPosition::LowerCenter),
    ("upper_center", LegendPosition::UpperCenter),
    ("center", LegendPosition::Center),
    ("outside_right", LegendPosition::OutsideRight),
    ("outside_left", LegendPosition::OutsideLeft),
    ("outside_upper", LegendPosition::OutsideUpper),
    ("outside_lower", LegendPosition::OutsideLower),
];

/// Look a name up in a `(name, value)` table, or report the accepted names.
fn lookup<T: Clone>(table: &[(&str, T)], kind: &str, name: &str) -> PyResult<T> {
    table
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, value)| value.clone())
        .ok_or_else(|| {
            let accepted: Vec<&str> = table.iter().map(|(candidate, _)| *candidate).collect();
            PyValueError::new_err(format!(
                "unsupported {kind} '{name}'; expected one of: {}",
                accepted.join(", ")
            ))
        })
}

fn parse_color(value: &str) -> PyResult<Color> {
    Color::named(value)
        .or_else(|| Color::hex(value))
        .ok_or_else(|| {
            let hint = Color::suggest_named(value)
                .map(|name| format!(" (did you mean '{name}'?)"))
                .unwrap_or_default();
            PyValueError::new_err(format!(
                "unsupported color '{value}'; expected a hex string like '#2563eb' \
                 or a named color such as red, green, blue, orange, purple, black, white, gray{hint}"
            ))
        })
}

fn ascending_limits(axis: &str, min: f64, max: f64) -> PyResult<(f64, f64)> {
    if !min.is_finite() || !max.is_finite() || min >= max {
        return Err(PyValueError::new_err(format!(
            "{axis} limits must be finite and strictly ascending"
        )));
    }
    Ok((min, max))
}

fn parse_axis_scale(scale: &str, linthresh: Option<f64>) -> PyResult<AxisScale> {
    match scale {
        "linear" => Ok(AxisScale::Linear),
        "log" => Ok(AxisScale::Log),
        "symlog" => {
            let linthresh = linthresh.unwrap_or(1.0);
            if !linthresh.is_finite() || linthresh <= 0.0 {
                return Err(PyValueError::new_err(
                    "symlog linthresh must be a finite positive number",
                ));
            }
            Ok(AxisScale::SymLog { linthresh })
        }
        other => Err(PyValueError::new_err(format!(
            "unsupported axis scale '{other}'; expected one of: linear, log, symlog"
        ))),
    }
}

fn extract_style(style: Option<&Bound<'_, PyDict>>) -> PyResult<SeriesStyle> {
    let mut parsed = SeriesStyle::default();
    let Some(style) = style else {
        return Ok(parsed);
    };

    for (key, value) in style.iter() {
        let key: String = key.extract()?;
        match key.as_str() {
            "label" => parsed.label = Some(value.extract()?),
            "color" => parsed.color = Some(parse_color(&value.extract::<String>()?)?),
            "alpha" => parsed.alpha = Some(value.extract()?),
            "width" => parsed.width = Some(value.extract()?),
            "linestyle" => {
                parsed.line_style = Some(lookup(
                    &LINE_STYLES,
                    "linestyle",
                    &value.extract::<String>()?,
                )?)
            }
            "marker" => {
                parsed.marker = Some(lookup(
                    &MARKER_STYLES,
                    "marker",
                    &value.extract::<String>()?,
                )?)
            }
            "markerSize" => parsed.marker_size = Some(value.extract()?),
            "bins" => parsed.bins = Some(value.extract()?),
            "bandwidth" => parsed.bandwidth = Some(value.extract()?),
            "levels" => parsed.levels = Some(value.extract()?),
            other => {
                return Err(PyValueError::new_err(format!(
                    "unsupported style option: {other}"
                )));
            }
        }
    }

    Ok(parsed)
}

/// Apply the styling every core `PlotBuilder<C>` shares.
fn styled<C: PlotConfig>(mut builder: PlotBuilder<C>, style: &SeriesStyle) -> PlotBuilder<C> {
    if let Some(label) = &style.label {
        builder = builder.label(label.clone());
    }
    if let Some(color) = style.color {
        builder = builder.color(color);
    }
    if let Some(alpha) = style.alpha {
        builder = builder.alpha(alpha);
    }
    if let Some(width) = style.width {
        builder = builder.line_width(width);
    }
    if let Some(line_style) = &style.line_style {
        builder = builder.line_style(line_style.clone());
    }
    builder
}

/// One stored series: its data plus the style the Python call attached to it.
#[derive(Clone)]
struct NativeSeries {
    data: NativeSeriesState,
    style: SeriesStyle,
}

#[derive(Clone)]
enum NativeSeriesState {
    Line {
        x: NumericSourceState,
        y: NumericSourceState,
    },
    Scatter {
        x: NumericSourceState,
        y: NumericSourceState,
    },
    Bar {
        categories: Vec<String>,
        values: NumericSourceState,
    },
    Histogram {
        data: NumericSourceState,
    },
    Boxplot {
        data: NumericSourceState,
    },
    Heatmap {
        values: Vec<f64>,
        rows: usize,
        cols: usize,
    },
    ErrorBars {
        x: NumericSourceState,
        y: NumericSourceState,
        y_errors: NumericSourceState,
    },
    ErrorBarsXy {
        x: NumericSourceState,
        y: NumericSourceState,
        x_errors: NumericSourceState,
        y_errors: NumericSourceState,
    },
    Kde {
        data: Vec<f64>,
    },
    Ecdf {
        data: Vec<f64>,
    },
    Contour {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<f64>,
    },
    Pie {
        values: Vec<f64>,
        labels: Option<Vec<String>>,
    },
    Radar {
        labels: Vec<String>,
        series: Vec<(Option<String>, Vec<f64>)>,
    },
    Violin {
        data: Vec<f64>,
    },
    PolarLine {
        r: Vec<f64>,
        theta: Vec<f64>,
    },
}

#[derive(Clone, Default)]
struct NativePlotState {
    size_px: Option<(u32, u32)>,
    theme: Option<String>,
    ticks: Option<bool>,
    title: Option<String>,
    x_label: Option<String>,
    y_label: Option<String>,
    legend: Option<LegendPosition>,
    grid: Option<bool>,
    x_limits: Option<(f64, f64)>,
    y_limits: Option<(f64, f64)>,
    x_scale: Option<AxisScale>,
    y_scale: Option<AxisScale>,
    series: Vec<NativeSeries>,
}

impl NativePlotState {
    fn build_plot(&self) -> Result<Plot, String> {
        let mut plot = Plot::new();

        if let Some((width, height)) = self.size_px {
            plot = plot.size_px(width, height);
        }

        if let Some(theme) = &self.theme {
            plot = match theme.as_str() {
                "light" => plot.theme(Theme::light()),
                "dark" => plot.theme(Theme::dark()),
                other => return Err(format!("unsupported theme: {other}")),
            };
        }

        if let Some(ticks) = self.ticks {
            plot = plot.ticks(ticks);
        }

        if let Some(title) = &self.title {
            plot = plot.title(title.clone());
        }

        if let Some(x_label) = &self.x_label {
            plot = plot.xlabel(x_label.clone());
        }

        if let Some(y_label) = &self.y_label {
            plot = plot.ylabel(y_label.clone());
        }

        if let Some(legend) = self.legend {
            plot = plot.legend(legend);
        }

        if let Some(grid) = self.grid {
            plot = plot.grid(grid);
        }

        if let Some((min, max)) = self.x_limits {
            plot = plot.xlim(min, max);
        }

        if let Some((min, max)) = self.y_limits {
            plot = plot.ylim(min, max);
        }

        if let Some(scale) = &self.x_scale {
            plot = plot.xscale(*scale);
        }

        if let Some(scale) = &self.y_scale {
            plot = plot.yscale(*scale);
        }

        for series in &self.series {
            plot = apply_series(plot, series.data.clone(), &series.style)?;
        }

        Ok(plot)
    }
}

fn ensure_same_len(lengths: &[usize], message: &str) -> Result<(), String> {
    let Some((&first, rest)) = lengths.split_first() else {
        return Ok(());
    };

    if rest.iter().all(|len| *len == first) {
        Ok(())
    } else {
        Err(message.to_string())
    }
}

fn apply_series(
    plot: Plot,
    series: NativeSeriesState,
    style: &SeriesStyle,
) -> Result<Plot, String> {
    match series {
        NativeSeriesState::Line { x, y } => {
            ensure_same_len(
                &[x.len(), y.len()],
                "line x and y must have the same length",
            )?;
            let mut builder = plot.line_source(x.into_plot_data(), y.into_plot_data());
            // Lines draw markers only when one is chosen, so a bare size implies circles.
            if let Some(marker) = style
                .marker
                .or_else(|| style.marker_size.map(|_| MarkerStyle::Circle))
            {
                builder = builder.marker(marker);
            }
            if let Some(size) = style.marker_size {
                builder = builder.marker_size(size);
            }
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Scatter { x, y } => {
            ensure_same_len(
                &[x.len(), y.len()],
                "scatter x and y must have the same length",
            )?;
            let mut builder = plot.scatter_source(x.into_plot_data(), y.into_plot_data());
            if let Some(marker) = style.marker {
                builder = builder.marker(marker);
            }
            if let Some(size) = style.marker_size {
                builder = builder.marker_size(size);
            }
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Bar { categories, values } => {
            ensure_same_len(
                &[categories.len(), values.len()],
                "bar categories and values must have the same length",
            )?;
            let builder = plot.bar_source(&categories, values.into_plot_data());
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Histogram { data } => {
            let mut builder = match data {
                NumericSourceState::Static(values) => plot.histogram(&values),
                NumericSourceState::Observable(values) => plot.histogram_source(values),
            };
            if let Some(bins) = style.bins {
                builder = builder.bins(bins);
            }
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Boxplot { data } => {
            let builder = plot.boxplot_source(data.into_plot_data());
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Heatmap { values, rows, cols } => {
            if rows == 0 || cols == 0 || values.len() != rows.saturating_mul(cols) {
                return Err("heatmap values length must match rows * cols".to_string());
            }
            let matrix: Vec<Vec<f64>> = values.chunks(cols).map(|chunk| chunk.to_vec()).collect();
            Ok(plot.heatmap(&matrix).into_plot())
        }
        NativeSeriesState::ErrorBars { x, y, y_errors } => {
            ensure_same_len(
                &[x.len(), y.len(), y_errors.len()],
                "error bar x, y, and y_errors must have the same length",
            )?;
            let builder = plot.error_bars_source(
                x.into_plot_data(),
                y.into_plot_data(),
                y_errors.into_plot_data(),
            );
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::ErrorBarsXy {
            x,
            y,
            x_errors,
            y_errors,
        } => {
            ensure_same_len(
                &[x.len(), y.len(), x_errors.len(), y_errors.len()],
                "error bar x, y, x_errors, and y_errors must have the same length",
            )?;
            let builder = plot.error_bars_xy_source(
                x.into_plot_data(),
                y.into_plot_data(),
                x_errors.into_plot_data(),
                y_errors.into_plot_data(),
            );
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Kde { data } => {
            let mut builder = plot.kde(&data);
            if let Some(bandwidth) = style.bandwidth {
                builder = builder.bandwidth(bandwidth);
            }
            Ok(styled(builder, style).into_plot())
        }
        NativeSeriesState::Ecdf { data } => Ok(styled(plot.ecdf(&data), style).into_plot()),
        NativeSeriesState::Contour { x, y, z } => {
            if x.is_empty() || y.is_empty() || z.len() != x.len() * y.len() {
                return Err("contour z must contain x.len() * y.len() values".to_string());
            }
            let mut builder = plot.contour(&x, &y, &z);
            if let Some(levels) = style.levels {
                builder = builder.levels(levels);
            }
            Ok(styled(builder, style).into_plot())
        }
        // Heatmap, pie, and radar declare no style options on the Python side.
        NativeSeriesState::Pie { values, labels } => {
            let builder = plot.pie(&values);
            if let Some(labels) = labels {
                ensure_same_len(
                    &[values.len(), labels.len()],
                    "pie values and labels must have the same length",
                )?;
                Ok(builder.labels(&labels).into_plot())
            } else {
                Ok(builder.into_plot())
            }
        }
        NativeSeriesState::Radar { labels, series } => {
            if labels.is_empty() {
                return Err("radar labels must not be empty".to_string());
            }

            let mut builder = plot.radar(&labels);
            for (name, values) in series {
                ensure_same_len(
                    &[labels.len(), values.len()],
                    "radar series values must match labels length",
                )?;
                builder = if let Some(name) = name {
                    builder.add_series(name, &values)
                } else {
                    builder.series(&values)
                };
            }
            Ok(builder.into_plot())
        }
        NativeSeriesState::Violin { data } => Ok(styled(plot.violin(&data), style).into_plot()),
        NativeSeriesState::PolarLine { r, theta } => {
            ensure_same_len(
                &[r.len(), theta.len()],
                "polar r and theta must have the same length",
            )?;
            Ok(styled(plot.polar_line(&r, &theta), style).into_plot())
        }
    }
}

const SUPPORTED_SAVE_EXTENSIONS: &str = ".png, .svg, or .pdf";

/// Validate an export path extension against the supported writer set.
pub(crate) fn save_extension(path: &str) -> PyResult<String> {
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());

    match extension {
        Some(extension) if matches!(extension.as_str(), "png" | "svg" | "pdf") => Ok(extension),
        Some(extension) => Err(PyValueError::new_err(format!(
            "unsupported save extension '.{extension}'; expected {SUPPORTED_SAVE_EXTENSIONS}"
        ))),
        None => Err(PyValueError::new_err(format!(
            "save path '{path}' has no extension; expected {SUPPORTED_SAVE_EXTENSIONS}"
        ))),
    }
}

fn extract_numeric_source(source: &Bound<'_, PyAny>) -> PyResult<NumericSourceState> {
    if let Ok(observable) = source.extract::<PyRef<'_, NativeObservable1D>>() {
        return Ok(NumericSourceState::Observable(observable.inner.clone()));
    }

    source
        .extract::<Vec<f64>>()
        .map(NumericSourceState::Static)
        .map_err(|_| PyTypeError::new_err("expected a numeric list or NativeObservable1D source"))
}

#[pyclass(module = "ruviz._native", unsendable)]
pub struct NativeObservable1D {
    inner: Observable<Vec<f64>>,
}

#[pymethods]
impl NativeObservable1D {
    #[new]
    fn new(values: Vec<f64>) -> Self {
        Self {
            inner: Observable::new(values),
        }
    }

    fn replace(&self, values: Vec<f64>) {
        self.inner.set(values);
    }

    fn set_at(&self, index: usize, value: f64) -> PyResult<()> {
        self.inner.update_with(|values| {
            if index >= values.len() {
                return Err(PyValueError::new_err("observable index is out of bounds"));
            }
            values[index] = value;
            Ok(())
        })
    }
}

#[pyclass(module = "ruviz._native", unsendable)]
pub struct NativePlotHandle {
    state: NativePlotState,
    plot: Plot,
    prepared: PreparedPlot,
    dirty: bool,
}

impl NativePlotHandle {
    fn rebuild(&mut self) -> PyResult<()> {
        let plot = self.state.build_plot().map_err(PyValueError::new_err)?;
        self.prepared = plot.prepare();
        self.plot = plot;
        self.dirty = false;
        Ok(())
    }

    fn ensure_built(&mut self) -> PyResult<()> {
        if self.dirty {
            self.rebuild()?;
        }
        Ok(())
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Store one series with its parsed style, rejecting bad style values here
    /// so they surface at the add-series call rather than at render time.
    fn push_series(
        &mut self,
        data: NativeSeriesState,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let style = extract_style(style)?;
        self.state.series.push(NativeSeries { data, style });
        self.mark_dirty();
        Ok(())
    }

    fn render_png_vec(&mut self) -> PyResult<Vec<u8>> {
        self.ensure_built()?;
        self.prepared
            .render_png_bytes()
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))
    }
}

#[pymethods]
impl NativePlotHandle {
    #[new]
    fn new() -> PyResult<Self> {
        let state = NativePlotState::default();
        let plot = state.build_plot().map_err(PyValueError::new_err)?;
        let prepared = plot.prepare();
        Ok(Self {
            state,
            plot,
            prepared,
            dirty: false,
        })
    }

    fn size_px(&mut self, width: u32, height: u32) -> PyResult<()> {
        if width == 0 || height == 0 {
            return Err(PyValueError::new_err(
                "plot dimensions must be greater than zero",
            ));
        }
        self.state.size_px = Some((width, height));
        self.mark_dirty();
        Ok(())
    }

    fn theme(&mut self, theme: &str) -> PyResult<()> {
        if !matches!(theme, "light" | "dark") {
            return Err(PyValueError::new_err(format!("unsupported theme: {theme}")));
        }
        self.state.theme = Some(theme.to_string());
        self.mark_dirty();
        Ok(())
    }

    fn ticks(&mut self, enabled: bool) -> PyResult<()> {
        self.state.ticks = Some(enabled);
        self.mark_dirty();
        Ok(())
    }

    fn title(&mut self, title: &str) -> PyResult<()> {
        self.state.title = Some(title.to_string());
        self.mark_dirty();
        Ok(())
    }

    fn xlabel(&mut self, label: &str) -> PyResult<()> {
        self.state.x_label = Some(label.to_string());
        self.mark_dirty();
        Ok(())
    }

    fn ylabel(&mut self, label: &str) -> PyResult<()> {
        self.state.y_label = Some(label.to_string());
        self.mark_dirty();
        Ok(())
    }

    fn legend(&mut self, position: &str) -> PyResult<()> {
        self.state.legend = Some(lookup(&LEGEND_POSITIONS, "legend position", position)?);
        self.mark_dirty();
        Ok(())
    }

    fn grid(&mut self, enabled: bool) -> PyResult<()> {
        self.state.grid = Some(enabled);
        self.mark_dirty();
        Ok(())
    }

    fn xlim(&mut self, min: f64, max: f64) -> PyResult<()> {
        self.state.x_limits = Some(ascending_limits("x", min, max)?);
        self.mark_dirty();
        Ok(())
    }

    fn ylim(&mut self, min: f64, max: f64) -> PyResult<()> {
        self.state.y_limits = Some(ascending_limits("y", min, max)?);
        self.mark_dirty();
        Ok(())
    }

    #[pyo3(signature = (scale, linthresh=None))]
    fn xscale(&mut self, scale: &str, linthresh: Option<f64>) -> PyResult<()> {
        self.state.x_scale = Some(parse_axis_scale(scale, linthresh)?);
        self.mark_dirty();
        Ok(())
    }

    #[pyo3(signature = (scale, linthresh=None))]
    fn yscale(&mut self, scale: &str, linthresh: Option<f64>) -> PyResult<()> {
        self.state.y_scale = Some(parse_axis_scale(scale, linthresh)?);
        self.mark_dirty();
        Ok(())
    }

    #[pyo3(signature = (x, y, style=None))]
    fn line(
        &mut self,
        x: &Bound<'_, PyAny>,
        y: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let x = extract_numeric_source(x)?;
        let y = extract_numeric_source(y)?;
        ensure_same_len(
            &[x.len(), y.len()],
            "line x and y must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.push_series(NativeSeriesState::Line { x, y }, style)
    }

    #[pyo3(signature = (x, y, style=None))]
    fn scatter(
        &mut self,
        x: &Bound<'_, PyAny>,
        y: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let x = extract_numeric_source(x)?;
        let y = extract_numeric_source(y)?;
        ensure_same_len(
            &[x.len(), y.len()],
            "scatter x and y must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.push_series(NativeSeriesState::Scatter { x, y }, style)
    }

    #[pyo3(signature = (categories, values, style=None))]
    fn bar(
        &mut self,
        categories: Vec<String>,
        values: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let values = extract_numeric_source(values)?;
        ensure_same_len(
            &[categories.len(), values.len()],
            "bar categories and values must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.push_series(NativeSeriesState::Bar { categories, values }, style)
    }

    #[pyo3(signature = (data, style=None))]
    fn histogram(
        &mut self,
        data: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let data = extract_numeric_source(data)?;
        self.push_series(NativeSeriesState::Histogram { data }, style)
    }

    #[pyo3(signature = (data, style=None))]
    fn boxplot(
        &mut self,
        data: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let data = extract_numeric_source(data)?;
        self.push_series(NativeSeriesState::Boxplot { data }, style)
    }

    fn heatmap(&mut self, values: Vec<f64>, rows: usize, cols: usize) -> PyResult<()> {
        if rows == 0 || cols == 0 || values.len() != rows.saturating_mul(cols) {
            return Err(PyValueError::new_err(
                "heatmap values length must match rows * cols",
            ));
        }
        self.push_series(NativeSeriesState::Heatmap { values, rows, cols }, None)
    }

    #[pyo3(signature = (x, y, y_errors, style=None))]
    fn error_bars(
        &mut self,
        x: &Bound<'_, PyAny>,
        y: &Bound<'_, PyAny>,
        y_errors: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let x = extract_numeric_source(x)?;
        let y = extract_numeric_source(y)?;
        let y_errors = extract_numeric_source(y_errors)?;
        ensure_same_len(
            &[x.len(), y.len(), y_errors.len()],
            "error bar x, y, and y_errors must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.push_series(NativeSeriesState::ErrorBars { x, y, y_errors }, style)
    }

    #[pyo3(signature = (x, y, x_errors, y_errors, style=None))]
    fn error_bars_xy(
        &mut self,
        x: &Bound<'_, PyAny>,
        y: &Bound<'_, PyAny>,
        x_errors: &Bound<'_, PyAny>,
        y_errors: &Bound<'_, PyAny>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        let x = extract_numeric_source(x)?;
        let y = extract_numeric_source(y)?;
        let x_errors = extract_numeric_source(x_errors)?;
        let y_errors = extract_numeric_source(y_errors)?;
        ensure_same_len(
            &[x.len(), y.len(), x_errors.len(), y_errors.len()],
            "error bar x, y, x_errors, and y_errors must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.push_series(
            NativeSeriesState::ErrorBarsXy {
                x,
                y,
                x_errors,
                y_errors,
            },
            style,
        )
    }

    #[pyo3(signature = (data, style=None))]
    fn kde(&mut self, data: Vec<f64>, style: Option<&Bound<'_, PyDict>>) -> PyResult<()> {
        self.push_series(NativeSeriesState::Kde { data }, style)
    }

    #[pyo3(signature = (data, style=None))]
    fn ecdf(&mut self, data: Vec<f64>, style: Option<&Bound<'_, PyDict>>) -> PyResult<()> {
        self.push_series(NativeSeriesState::Ecdf { data }, style)
    }

    #[pyo3(signature = (x, y, z, style=None))]
    fn contour(
        &mut self,
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<f64>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        if x.is_empty() || y.is_empty() || z.len() != x.len() * y.len() {
            return Err(PyValueError::new_err(
                "contour z must contain x.length * y.length values",
            ));
        }
        self.push_series(NativeSeriesState::Contour { x, y, z }, style)
    }

    fn pie(&mut self, values: Vec<f64>, labels: Option<Vec<String>>) -> PyResult<()> {
        if let Some(labels) = &labels {
            ensure_same_len(
                &[values.len(), labels.len()],
                "pie values and labels must have the same length",
            )
            .map_err(PyValueError::new_err)?;
        }
        self.push_series(NativeSeriesState::Pie { values, labels }, None)
    }

    fn radar(
        &mut self,
        labels: Vec<String>,
        series: Vec<(Option<String>, Vec<f64>)>,
    ) -> PyResult<()> {
        if labels.is_empty() {
            return Err(PyValueError::new_err("radar labels must not be empty"));
        }
        for (_, values) in &series {
            ensure_same_len(
                &[labels.len(), values.len()],
                "each radar series must match the labels length",
            )
            .map_err(PyValueError::new_err)?;
        }
        self.push_series(NativeSeriesState::Radar { labels, series }, None)
    }

    #[pyo3(signature = (data, style=None))]
    fn violin(&mut self, data: Vec<f64>, style: Option<&Bound<'_, PyDict>>) -> PyResult<()> {
        self.push_series(NativeSeriesState::Violin { data }, style)
    }

    #[pyo3(signature = (r, theta, style=None))]
    fn polar_line(
        &mut self,
        r: Vec<f64>,
        theta: Vec<f64>,
        style: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<()> {
        ensure_same_len(
            &[r.len(), theta.len()],
            "polar r and theta must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.push_series(NativeSeriesState::PolarLine { r, theta }, style)
    }

    fn render_png_bytes<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self.render_png_vec()?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn render_png_bytes_uncached<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        self.ensure_built()?;
        let bytes = self
            .prepared
            .render_png_bytes_uncached()
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn render_svg(&mut self) -> PyResult<String> {
        self.ensure_built()?;
        self.plot
            .render_to_svg()
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))
    }

    fn save(&mut self, path: &str) -> PyResult<()> {
        let extension = save_extension(path)?;
        self.ensure_built()?;
        let output = Path::new(path);

        match extension.as_str() {
            "svg" => self
                .plot
                .clone()
                .export_svg(output)
                .map_err(|err| PyRuntimeError::new_err(err.to_string())),
            "pdf" => self
                .plot
                .clone()
                .save_pdf(output)
                .map_err(|err| PyRuntimeError::new_err(err.to_string())),
            _ => self
                .plot
                .clone()
                .save(output)
                .map_err(|err| PyRuntimeError::new_err(err.to_string())),
        }
    }

    fn show_native(&mut self) -> PyResult<()> {
        #[cfg(not(feature = "native-interactive"))]
        {
            return Err(PyRuntimeError::new_err(
                NATIVE_INTERACTIVE_UNAVAILABLE_MESSAGE,
            ));
        }

        #[cfg(feature = "native-interactive")]
        self.ensure_built()?;
        #[cfg(feature = "native-interactive")]
        {
            pollster::block_on(show_interactive(self.plot.clone()))
                .map_err(|err| PyRuntimeError::new_err(err.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use image::RgbaImage;

    fn decode_png(bytes: &[u8]) -> RgbaImage {
        image::load_from_memory(bytes)
            .expect("expected valid PNG bytes")
            .to_rgba8()
    }

    fn assert_pngs_equal(name: &str, actual: &[u8], expected: &[u8]) {
        let actual = decode_png(actual);
        let expected = decode_png(expected);

        assert_eq!(
            actual.dimensions(),
            expected.dimensions(),
            "{name}: PNG dimensions differ"
        );

        let diff_pixels = actual
            .pixels()
            .zip(expected.pixels())
            .filter(|(left, right)| left != right)
            .count();

        assert_eq!(
            diff_pixels, 0,
            "{name}: rendered PNG differs at {diff_pixels} pixel(s)"
        );
    }

    fn render_state_png(state: NativePlotState) -> Vec<u8> {
        state
            .build_plot()
            .expect("state should build")
            .render_png_bytes()
            .expect("state render should succeed")
    }

    fn render_direct_png(plot: Plot) -> Vec<u8> {
        plot.render_png_bytes()
            .expect("direct render should succeed")
    }

    fn plain(data: NativeSeriesState) -> NativeSeries {
        NativeSeries {
            data,
            style: SeriesStyle::default(),
        }
    }

    fn base_state(title: &str, series: Vec<NativeSeriesState>) -> NativePlotState {
        NativePlotState {
            size_px: Some((320, 200)),
            theme: Some("light".to_string()),
            ticks: Some(true),
            title: Some(title.to_string()),
            x_label: Some("x".to_string()),
            y_label: Some("y".to_string()),
            series: series.into_iter().map(plain).collect(),
            ..NativePlotState::default()
        }
    }

    fn base_plot(title: &str) -> Plot {
        Plot::new()
            .size_px(320, 200)
            .theme(Theme::light())
            .ticks(true)
            .title(title)
            .xlabel("x")
            .ylabel("y")
    }

    fn static_source(values: &[f64]) -> NumericSourceState {
        NumericSourceState::Static(values.to_vec())
    }

    #[test]
    fn state_render_matches_direct_plot_for_xy_series() {
        let state = base_state(
            "XY parity",
            vec![
                NativeSeriesState::Line {
                    x: static_source(&[0.0, 1.0, 2.0, 3.0]),
                    y: static_source(&[0.2, 1.1, 0.7, 1.8]),
                },
                NativeSeriesState::Scatter {
                    x: static_source(&[0.0, 1.0, 2.0, 3.0]),
                    y: static_source(&[0.3, 1.0, 0.9, 1.6]),
                },
            ],
        );

        let expected = base_plot("XY parity")
            .line(&[0.0, 1.0, 2.0, 3.0], &[0.2, 1.1, 0.7, 1.8])
            .scatter(&[0.0, 1.0, 2.0, 3.0], &[0.3, 1.0, 0.9, 1.6])
            .into_plot();

        assert_pngs_equal(
            "xy_series",
            &render_state_png(state),
            &render_direct_png(expected),
        );
    }

    #[test]
    fn state_render_matches_direct_plot_for_statistical_series() {
        let samples = [
            -2.3, -1.9, -1.1, -0.4, 0.2, 0.8, 1.0, 1.4, 1.7, 2.1, 2.5, 2.9,
        ];

        let mut state = base_state(
            "Histogram parity",
            vec![NativeSeriesState::Histogram {
                data: static_source(&samples),
            }],
        );
        state.x_label = Some("value".to_string());
        state.y_label = Some("count".to_string());

        let expected = base_plot("Histogram parity")
            .xlabel("value")
            .ylabel("count")
            .histogram(&samples)
            .into_plot();

        assert_pngs_equal(
            "histogram",
            &render_state_png(state),
            &render_direct_png(expected),
        );
    }

    #[test]
    fn state_render_matches_direct_plot_for_matrix_and_errorbar_series() {
        let heatmap_matrix = vec![
            vec![0.1, 0.4, 0.8],
            vec![0.3, 0.5, 0.7],
            vec![0.2, 0.6, 0.9],
        ];

        let heatmap_state = NativePlotState {
            size_px: Some((320, 200)),
            theme: Some("dark".to_string()),
            ticks: Some(false),
            title: Some("Heatmap parity".to_string()),
            series: vec![plain(NativeSeriesState::Heatmap {
                values: vec![0.1, 0.4, 0.8, 0.3, 0.5, 0.7, 0.2, 0.6, 0.9],
                rows: 3,
                cols: 3,
            })],
            ..NativePlotState::default()
        };

        let heatmap_expected = Plot::new()
            .size_px(320, 200)
            .theme(Theme::dark())
            .ticks(false)
            .title("Heatmap parity")
            .heatmap(&heatmap_matrix)
            .into_plot();

        assert_pngs_equal(
            "heatmap",
            &render_state_png(heatmap_state),
            &render_direct_png(heatmap_expected),
        );

        let error_state = base_state(
            "Error parity",
            vec![NativeSeriesState::ErrorBarsXy {
                x: static_source(&[1.0, 2.0, 3.0]),
                y: static_source(&[1.2, 1.8, 1.4]),
                x_errors: static_source(&[0.1, 0.15, 0.12]),
                y_errors: static_source(&[0.2, 0.18, 0.16]),
            }],
        );

        let error_expected = base_plot("Error parity")
            .error_bars_xy(
                &[1.0, 2.0, 3.0],
                &[1.2, 1.8, 1.4],
                &[0.1, 0.15, 0.12],
                &[0.2, 0.18, 0.16],
            )
            .into_plot();

        assert_pngs_equal(
            "error_bars_xy",
            &render_state_png(error_state),
            &render_direct_png(error_expected),
        );
    }

    #[test]
    fn state_render_matches_direct_plot_for_specialized_series() {
        let titled_state = |title: &str, series| NativePlotState {
            size_px: Some((320, 200)),
            theme: Some("light".to_string()),
            ticks: Some(true),
            title: Some(title.to_string()),
            series: vec![plain(series)],
            ..NativePlotState::default()
        };
        let titled_plot = |title: &str| {
            Plot::new()
                .size_px(320, 200)
                .theme(Theme::light())
                .ticks(true)
                .title(title)
        };

        let contour_state = titled_state(
            "Contour parity",
            NativeSeriesState::Contour {
                x: vec![-1.0, 0.0, 1.0],
                y: vec![-1.0, 0.0, 1.0],
                z: vec![0.1, 0.2, 0.3, 0.2, 0.6, 0.2, 0.3, 0.2, 0.1],
            },
        );
        let contour_expected = titled_plot("Contour parity")
            .contour(
                &[-1.0, 0.0, 1.0],
                &[-1.0, 0.0, 1.0],
                &[0.1, 0.2, 0.3, 0.2, 0.6, 0.2, 0.3, 0.2, 0.1],
            )
            .into_plot();

        assert_pngs_equal(
            "contour",
            &render_state_png(contour_state),
            &render_direct_png(contour_expected),
        );

        let pie_state = titled_state(
            "Pie parity",
            NativeSeriesState::Pie {
                values: vec![30.0, 25.0, 20.0, 25.0],
                labels: Some(vec![
                    "A".to_string(),
                    "B".to_string(),
                    "C".to_string(),
                    "D".to_string(),
                ]),
            },
        );
        let pie_expected = titled_plot("Pie parity")
            .pie(&[30.0, 25.0, 20.0, 25.0])
            .labels(&["A", "B", "C", "D"])
            .into_plot();

        assert_pngs_equal(
            "pie",
            &render_state_png(pie_state),
            &render_direct_png(pie_expected),
        );

        let radar_state = titled_state(
            "Radar parity",
            NativeSeriesState::Radar {
                labels: ["API", "Docs", "Export", "Interactive", "Scale"]
                    .map(str::to_string)
                    .to_vec(),
                series: vec![
                    (Some("Python".to_string()), vec![4.5, 4.7, 4.8, 4.3, 4.0]),
                    (Some("Web".to_string()), vec![4.2, 4.1, 4.0, 4.8, 4.6]),
                ],
            },
        );
        let radar_expected = titled_plot("Radar parity")
            .radar(&["API", "Docs", "Export", "Interactive", "Scale"])
            .add_series("Python", &[4.5, 4.7, 4.8, 4.3, 4.0])
            .add_series("Web", &[4.2, 4.1, 4.0, 4.8, 4.6])
            .into_plot();

        assert_pngs_equal(
            "radar",
            &render_state_png(radar_state),
            &render_direct_png(radar_expected),
        );
    }

    fn styled_line_state(style: SeriesStyle) -> NativePlotState {
        NativePlotState {
            series: vec![NativeSeries {
                data: NativeSeriesState::Line {
                    x: static_source(&[0.0, 1.0, 2.0, 3.0]),
                    y: static_source(&[0.2, 1.1, 0.7, 1.8]),
                },
                style,
            }],
            ..base_state("Styled", vec![])
        }
    }

    #[test]
    fn styled_series_render_matches_direct_plot_and_differs_from_default() {
        let style = SeriesStyle {
            label: Some("Revenue".to_string()),
            color: Some(Color::hex("#2563eb").expect("hex color")),
            alpha: Some(0.6),
            width: Some(3.5),
            line_style: Some(LineStyle::Dashed),
            marker: Some(MarkerStyle::Square),
            marker_size: Some(9.0),
            ..SeriesStyle::default()
        };

        let expected = base_plot("Styled")
            .line(&[0.0, 1.0, 2.0, 3.0], &[0.2, 1.1, 0.7, 1.8])
            .marker(MarkerStyle::Square)
            .marker_size(9.0)
            .label("Revenue")
            .color(Color::hex("#2563eb").expect("hex color"))
            .alpha(0.6)
            .line_width(3.5)
            .line_style(LineStyle::Dashed)
            .into_plot();

        let styled_png = render_state_png(styled_line_state(style));
        assert_pngs_equal("styled_line", &styled_png, &render_direct_png(expected));
        assert_ne!(
            styled_png,
            render_state_png(styled_line_state(SeriesStyle::default())),
            "styled render should differ from the unstyled default"
        );
    }

    #[test]
    fn plot_level_settings_match_direct_plot() {
        let state = NativePlotState {
            legend: Some(LegendPosition::UpperLeft),
            grid: Some(false),
            x_limits: Some((-1.0, 6.0)),
            y_limits: Some((0.0, 4.0)),
            x_scale: Some(AxisScale::SymLog { linthresh: 1.0 }),
            y_scale: Some(AxisScale::Linear),
            ..styled_line_state(SeriesStyle {
                label: Some("Revenue".to_string()),
                ..SeriesStyle::default()
            })
        };

        let expected = base_plot("Styled")
            .line(&[0.0, 1.0, 2.0, 3.0], &[0.2, 1.1, 0.7, 1.8])
            .label("Revenue")
            .into_plot()
            .legend(LegendPosition::UpperLeft)
            .grid(false)
            .xlim(-1.0, 6.0)
            .ylim(0.0, 4.0)
            .xscale(AxisScale::SymLog { linthresh: 1.0 })
            .yscale(AxisScale::Linear);

        assert_pngs_equal(
            "plot_level_settings",
            &render_state_png(state),
            &render_direct_png(expected),
        );
    }

    #[test]
    fn style_lookups_reject_unknown_names() {
        assert!(parse_color("not-a-color").is_err());
        assert!(lookup(&MARKER_STYLES, "marker", "blob").is_err());
        assert!(lookup(&LINE_STYLES, "linestyle", "wavy").is_err());
        assert!(lookup(&LEGEND_POSITIONS, "legend position", "nowhere").is_err());
        assert!(parse_axis_scale("logarithmic", None).is_err());
        assert!(parse_axis_scale("symlog", Some(0.0)).is_err());
        assert!(ascending_limits("x", 1.0, 1.0).is_err());
        assert!(ascending_limits("x", f64::NAN, 1.0).is_err());
    }
}
