use std::path::Path;

use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
    types::{PyAny, PyBytes},
};
use ruviz::{
    core::{
        IntoPlot, Plot, PreparedPlot,
        plot::{IntoPlotData, PlotData},
    },
    data::Observable,
    render::Theme,
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
    series: Vec<NativeSeriesState>,
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

        for series in &self.series {
            plot = apply_series(plot, series.clone())?;
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

fn apply_series(plot: Plot, series: NativeSeriesState) -> Result<Plot, String> {
    match series {
        NativeSeriesState::Line { x, y } => {
            ensure_same_len(
                &[x.len(), y.len()],
                "line x and y must have the same length",
            )?;
            Ok(plot
                .line_source(x.into_plot_data(), y.into_plot_data())
                .into_plot())
        }
        NativeSeriesState::Scatter { x, y } => {
            ensure_same_len(
                &[x.len(), y.len()],
                "scatter x and y must have the same length",
            )?;
            Ok(plot
                .scatter_source(x.into_plot_data(), y.into_plot_data())
                .into_plot())
        }
        NativeSeriesState::Bar { categories, values } => {
            ensure_same_len(
                &[categories.len(), values.len()],
                "bar categories and values must have the same length",
            )?;
            Ok(plot
                .bar_source(&categories, values.into_plot_data())
                .into_plot())
        }
        NativeSeriesState::Histogram { data } => match data {
            NumericSourceState::Static(values) => Ok(plot.histogram(&values, None).into_plot()),
            NumericSourceState::Observable(values) => {
                Ok(plot.histogram_source(values, None).into_plot())
            }
        },
        NativeSeriesState::Boxplot { data } => {
            Ok(plot.boxplot_source(data.into_plot_data(), None).into_plot())
        }
        NativeSeriesState::Heatmap { values, rows, cols } => {
            if rows == 0 || cols == 0 || values.len() != rows.saturating_mul(cols) {
                return Err("heatmap values length must match rows * cols".to_string());
            }
            let matrix: Vec<Vec<f64>> = values.chunks(cols).map(|chunk| chunk.to_vec()).collect();
            Ok(plot.heatmap(&matrix, None).into_plot())
        }
        NativeSeriesState::ErrorBars { x, y, y_errors } => {
            ensure_same_len(
                &[x.len(), y.len(), y_errors.len()],
                "error bar x, y, and y_errors must have the same length",
            )?;
            Ok(plot
                .error_bars_source(
                    x.into_plot_data(),
                    y.into_plot_data(),
                    y_errors.into_plot_data(),
                )
                .into_plot())
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
            Ok(plot
                .error_bars_xy_source(
                    x.into_plot_data(),
                    y.into_plot_data(),
                    x_errors.into_plot_data(),
                    y_errors.into_plot_data(),
                )
                .into_plot())
        }
        NativeSeriesState::Kde { data } => Ok(plot.kde(&data).into_plot()),
        NativeSeriesState::Ecdf { data } => Ok(plot.ecdf(&data).into_plot()),
        NativeSeriesState::Contour { x, y, z } => {
            if x.is_empty() || y.is_empty() || z.len() != x.len() * y.len() {
                return Err("contour z must contain x.len() * y.len() values".to_string());
            }
            Ok(plot.contour(&x, &y, &z).into_plot())
        }
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
        NativeSeriesState::Violin { data } => Ok(plot.violin(&data).into_plot()),
        NativeSeriesState::PolarLine { r, theta } => {
            ensure_same_len(
                &[r.len(), theta.len()],
                "polar r and theta must have the same length",
            )?;
            Ok(plot.polar_line(&r, &theta).into_plot())
        }
    }
}

fn extract_numeric_source(source: &Bound<'_, PyAny>) -> PyResult<NumericSourceState> {
    if let Ok(observable) = source.extract::<PyRef<'_, NativeObservable1D>>() {
        return Ok(NumericSourceState::Observable(observable.inner.clone()));
    }

    source
        .extract::<Vec<f64>>()
        .map(NumericSourceState::Static)
        .map_err(|_| PyValueError::new_err("expected a numeric list or NativeObservable1D source"))
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
        self.state.size_px = Some((width.max(1), height.max(1)));
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

    fn line(&mut self, x: &Bound<'_, PyAny>, y: &Bound<'_, PyAny>) -> PyResult<()> {
        let x = extract_numeric_source(x)?;
        let y = extract_numeric_source(y)?;
        ensure_same_len(
            &[x.len(), y.len()],
            "line x and y must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.state.series.push(NativeSeriesState::Line { x, y });
        self.mark_dirty();
        Ok(())
    }

    fn scatter(&mut self, x: &Bound<'_, PyAny>, y: &Bound<'_, PyAny>) -> PyResult<()> {
        let x = extract_numeric_source(x)?;
        let y = extract_numeric_source(y)?;
        ensure_same_len(
            &[x.len(), y.len()],
            "scatter x and y must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.state.series.push(NativeSeriesState::Scatter { x, y });
        self.mark_dirty();
        Ok(())
    }

    fn bar(&mut self, categories: Vec<String>, values: &Bound<'_, PyAny>) -> PyResult<()> {
        let values = extract_numeric_source(values)?;
        ensure_same_len(
            &[categories.len(), values.len()],
            "bar categories and values must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.state
            .series
            .push(NativeSeriesState::Bar { categories, values });
        self.mark_dirty();
        Ok(())
    }

    fn histogram(&mut self, data: &Bound<'_, PyAny>) -> PyResult<()> {
        let data = extract_numeric_source(data)?;
        self.state
            .series
            .push(NativeSeriesState::Histogram { data });
        self.mark_dirty();
        Ok(())
    }

    fn boxplot(&mut self, data: &Bound<'_, PyAny>) -> PyResult<()> {
        let data = extract_numeric_source(data)?;
        self.state.series.push(NativeSeriesState::Boxplot { data });
        self.mark_dirty();
        Ok(())
    }

    fn heatmap(&mut self, values: Vec<f64>, rows: usize, cols: usize) -> PyResult<()> {
        if rows == 0 || cols == 0 || values.len() != rows.saturating_mul(cols) {
            return Err(PyValueError::new_err(
                "heatmap values length must match rows * cols",
            ));
        }
        self.state
            .series
            .push(NativeSeriesState::Heatmap { values, rows, cols });
        self.mark_dirty();
        Ok(())
    }

    fn error_bars(
        &mut self,
        x: &Bound<'_, PyAny>,
        y: &Bound<'_, PyAny>,
        y_errors: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let x = extract_numeric_source(x)?;
        let y = extract_numeric_source(y)?;
        let y_errors = extract_numeric_source(y_errors)?;
        ensure_same_len(
            &[x.len(), y.len(), y_errors.len()],
            "error bar x, y, and y_errors must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.state
            .series
            .push(NativeSeriesState::ErrorBars { x, y, y_errors });
        self.mark_dirty();
        Ok(())
    }

    fn error_bars_xy(
        &mut self,
        x: &Bound<'_, PyAny>,
        y: &Bound<'_, PyAny>,
        x_errors: &Bound<'_, PyAny>,
        y_errors: &Bound<'_, PyAny>,
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
        self.state.series.push(NativeSeriesState::ErrorBarsXy {
            x,
            y,
            x_errors,
            y_errors,
        });
        self.mark_dirty();
        Ok(())
    }

    fn kde(&mut self, data: Vec<f64>) -> PyResult<()> {
        self.state.series.push(NativeSeriesState::Kde { data });
        self.mark_dirty();
        Ok(())
    }

    fn ecdf(&mut self, data: Vec<f64>) -> PyResult<()> {
        self.state.series.push(NativeSeriesState::Ecdf { data });
        self.mark_dirty();
        Ok(())
    }

    fn contour(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<f64>) -> PyResult<()> {
        if x.is_empty() || y.is_empty() || z.len() != x.len() * y.len() {
            return Err(PyValueError::new_err(
                "contour z must contain x.length * y.length values",
            ));
        }
        self.state
            .series
            .push(NativeSeriesState::Contour { x, y, z });
        self.mark_dirty();
        Ok(())
    }

    fn pie(&mut self, values: Vec<f64>, labels: Option<Vec<String>>) -> PyResult<()> {
        if let Some(labels) = &labels {
            ensure_same_len(
                &[values.len(), labels.len()],
                "pie values and labels must have the same length",
            )
            .map_err(PyValueError::new_err)?;
        }
        self.state
            .series
            .push(NativeSeriesState::Pie { values, labels });
        self.mark_dirty();
        Ok(())
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
        self.state
            .series
            .push(NativeSeriesState::Radar { labels, series });
        self.mark_dirty();
        Ok(())
    }

    fn violin(&mut self, data: Vec<f64>) -> PyResult<()> {
        self.state.series.push(NativeSeriesState::Violin { data });
        self.mark_dirty();
        Ok(())
    }

    fn polar_line(&mut self, r: Vec<f64>, theta: Vec<f64>) -> PyResult<()> {
        ensure_same_len(
            &[r.len(), theta.len()],
            "polar r and theta must have the same length",
        )
        .map_err(PyValueError::new_err)?;
        self.state
            .series
            .push(NativeSeriesState::PolarLine { r, theta });
        self.mark_dirty();
        Ok(())
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
        self.ensure_built()?;
        let output = Path::new(path);
        let extension = output
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .unwrap_or_else(|| "png".to_string());

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
