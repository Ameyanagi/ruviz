use std::path::Path;

use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
    types::PyBytes,
};
use ruviz::{
    core::{IntoPlot, Line3DBuilder, Plot, Scatter3DBuilder, Surface3DBuilder, Wireframe3DBuilder},
    render::Theme,
};
use serde::Deserialize;

mod native_handle;

use native_handle::{NativeObservable1D, NativePlotHandle};

#[cfg(feature = "native-interactive")]
use ruviz::interactive::show_interactive;

#[cfg(not(feature = "native-interactive"))]
pub(crate) const NATIVE_INTERACTIVE_UNAVAILABLE_MESSAGE: &str = "native interactive windows are unavailable in this wheel; install ruviz from source on Linux to enable plot.show()";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlotSnapshot {
    size_px: Option<[u32; 2]>,
    theme: Option<String>,
    ticks: Option<bool>,
    title: Option<String>,
    x_label: Option<String>,
    y_label: Option<String>,
    series: Vec<SeriesSnapshot>,
}

#[derive(Clone, Default)]
struct Plot3DSnapshot {
    size_px: Option<[u32; 2]>,
    dpi: Option<u32>,
    theme: Option<String>,
    title: Option<String>,
    x_label: Option<String>,
    y_label: Option<String>,
    z_label: Option<String>,
    x_lim: Option<[f64; 2]>,
    y_lim: Option<[f64; 2]>,
    z_lim: Option<[f64; 2]>,
    azimuth_deg: Option<f32>,
    elevation_deg: Option<f32>,
    perspective_deg: Option<f32>,
    projection: Option<Projection3DSnapshot>,
    series: Vec<Series3DSnapshot>,
}

#[derive(Clone, Copy)]
enum Projection3DSnapshot {
    Orthographic,
    Perspective,
}

#[derive(Clone)]
enum Series3DSnapshot {
    Scatter3d {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<f64>,
    },
    Line3d {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<f64>,
    },
    Surface {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<Vec<f64>>,
    },
    Wireframe {
        x: Vec<f64>,
        y: Vec<f64>,
        z: Vec<Vec<f64>>,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SineSignalOptions {
    points: usize,
    domain_start: f64,
    domain_end: f64,
    amplitude: f64,
    cycles: f64,
    phase_velocity: f64,
    phase_offset: f64,
    vertical_offset: f64,
}

impl SineSignalOptions {
    fn values_at(&self, time_seconds: f64) -> Vec<f64> {
        let len = self.points.max(2);
        let span = self.domain_end - self.domain_start;
        let phase = self.phase_offset + self.phase_velocity * time_seconds;
        let denom = (len - 1) as f64;

        (0..len)
            .map(|index| {
                let progress = index as f64 / denom;
                let x = self.domain_start + span * progress;
                self.vertical_offset + self.amplitude * (self.cycles * x + phase).sin()
            })
            .collect()
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum NumericSourceSnapshot {
    Static { values: Vec<f64> },
    Observable { values: Vec<f64> },
}

impl NumericSourceSnapshot {
    fn into_values(self) -> Vec<f64> {
        match self {
            Self::Static { values } | Self::Observable { values } => values,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum YSourceSnapshot {
    Static { values: Vec<f64> },
    Observable { values: Vec<f64> },
    SineSignal { options: SineSignalOptions },
}

impl YSourceSnapshot {
    fn into_values(self) -> Vec<f64> {
        match self {
            Self::Static { values } | Self::Observable { values } => values,
            Self::SineSignal { options } => options.values_at(0.0),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RadarSeriesItem {
    name: Option<String>,
    values: Vec<f64>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum SeriesSnapshot {
    Line {
        x: NumericSourceSnapshot,
        y: YSourceSnapshot,
    },
    Scatter {
        x: NumericSourceSnapshot,
        y: YSourceSnapshot,
    },
    Bar {
        categories: Vec<String>,
        values: NumericSourceSnapshot,
    },
    Histogram {
        data: NumericSourceSnapshot,
    },
    Boxplot {
        data: NumericSourceSnapshot,
    },
    Heatmap {
        values: Vec<f64>,
        rows: usize,
        cols: usize,
    },
    ErrorBars {
        x: NumericSourceSnapshot,
        y: NumericSourceSnapshot,
        #[serde(rename = "yErrors")]
        y_errors: NumericSourceSnapshot,
    },
    ErrorBarsXy {
        x: NumericSourceSnapshot,
        y: NumericSourceSnapshot,
        #[serde(rename = "xErrors")]
        x_errors: NumericSourceSnapshot,
        #[serde(rename = "yErrors")]
        y_errors: NumericSourceSnapshot,
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
        series: Vec<RadarSeriesItem>,
    },
    Violin {
        data: Vec<f64>,
    },
    PolarLine {
        r: Vec<f64>,
        theta: Vec<f64>,
    },
}

impl PlotSnapshot {
    fn into_plot(self) -> Result<Plot, String> {
        let mut plot = Plot::new();

        if let Some([width, height]) = self.size_px {
            plot = plot.size_px(width, height);
        }

        if let Some(theme) = self.theme {
            plot = match theme.as_str() {
                "light" => plot.theme(Theme::light()),
                "dark" => plot.theme(Theme::dark()),
                other => return Err(format!("unsupported theme: {other}")),
            };
        }

        if let Some(ticks) = self.ticks {
            plot = plot.ticks(ticks);
        }

        if let Some(title) = self.title {
            plot = plot.title(title);
        }

        if let Some(x_label) = self.x_label {
            plot = plot.xlabel(x_label);
        }

        if let Some(y_label) = self.y_label {
            plot = plot.ylabel(y_label);
        }

        for series in self.series {
            plot = apply_series(plot, series)?;
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

fn apply_series(plot: Plot, series: SeriesSnapshot) -> Result<Plot, String> {
    match series {
        SeriesSnapshot::Line { x, y } => {
            let x = x.into_values();
            let y = y.into_values();
            ensure_same_len(
                &[x.len(), y.len()],
                "line x and y must have the same length",
            )?;
            Ok(plot.line(&x, &y).into_plot())
        }
        SeriesSnapshot::Scatter { x, y } => {
            let x = x.into_values();
            let y = y.into_values();
            ensure_same_len(
                &[x.len(), y.len()],
                "scatter x and y must have the same length",
            )?;
            Ok(plot.scatter(&x, &y).into_plot())
        }
        SeriesSnapshot::Bar { categories, values } => {
            let values = values.into_values();
            ensure_same_len(
                &[categories.len(), values.len()],
                "bar categories and values must have the same length",
            )?;
            Ok(plot.bar(&categories, &values).into_plot())
        }
        SeriesSnapshot::Histogram { data } => Ok(plot.histogram(&data.into_values()).into_plot()),
        SeriesSnapshot::Boxplot { data } => Ok(plot.boxplot(&data.into_values()).into_plot()),
        SeriesSnapshot::Heatmap { values, rows, cols } => {
            if rows == 0 || cols == 0 || values.len() != rows.saturating_mul(cols) {
                return Err("heatmap values length must match rows * cols".to_string());
            }
            let matrix: Vec<Vec<f64>> = values.chunks(cols).map(|chunk| chunk.to_vec()).collect();
            Ok(plot.heatmap(&matrix).into_plot())
        }
        SeriesSnapshot::ErrorBars { x, y, y_errors } => {
            let x = x.into_values();
            let y = y.into_values();
            let y_errors = y_errors.into_values();
            ensure_same_len(
                &[x.len(), y.len(), y_errors.len()],
                "error bar x, y, and y_errors must have the same length",
            )?;
            Ok(plot.error_bars(&x, &y, &y_errors).into_plot())
        }
        SeriesSnapshot::ErrorBarsXy {
            x,
            y,
            x_errors,
            y_errors,
        } => {
            let x = x.into_values();
            let y = y.into_values();
            let x_errors = x_errors.into_values();
            let y_errors = y_errors.into_values();
            ensure_same_len(
                &[x.len(), y.len(), x_errors.len(), y_errors.len()],
                "error bar x, y, x_errors, and y_errors must have the same length",
            )?;
            Ok(plot.error_bars_xy(&x, &y, &x_errors, &y_errors).into_plot())
        }
        SeriesSnapshot::Kde { data } => Ok(plot.kde(&data).into_plot()),
        SeriesSnapshot::Ecdf { data } => Ok(plot.ecdf(&data).into_plot()),
        SeriesSnapshot::Contour { x, y, z } => {
            if x.is_empty() || y.is_empty() || z.len() != x.len() * y.len() {
                return Err("contour z must contain x.len() * y.len() values".to_string());
            }
            Ok(plot.contour(&x, &y, &z).into_plot())
        }
        SeriesSnapshot::Pie { values, labels } => {
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
        SeriesSnapshot::Radar { labels, series } => {
            if labels.is_empty() {
                return Err("radar labels must not be empty".to_string());
            }

            let mut builder = plot.radar(&labels);
            for item in series {
                ensure_same_len(
                    &[labels.len(), item.values.len()],
                    "radar series values must match labels length",
                )?;
                builder = if let Some(name) = item.name {
                    builder.add_series(name, &item.values)
                } else {
                    builder.series(&item.values)
                };
            }
            Ok(builder.into_plot())
        }
        SeriesSnapshot::Violin { data } => Ok(plot.violin(&data).into_plot()),
        SeriesSnapshot::PolarLine { r, theta } => {
            ensure_same_len(
                &[r.len(), theta.len()],
                "polar r and theta must have the same length",
            )?;
            Ok(plot.polar_line(&r, &theta).into_plot())
        }
    }
}

fn parse_plot(snapshot_json: &str) -> PyResult<Plot> {
    let snapshot: PlotSnapshot = serde_json::from_str(snapshot_json)
        .map_err(|err| PyValueError::new_err(err.to_string()))?;

    snapshot.into_plot().map_err(PyValueError::new_err)
}

enum Plot3DBuilderState {
    Scatter(Scatter3DBuilder),
    Line(Line3DBuilder),
    Surface(Surface3DBuilder),
    Wireframe(Wireframe3DBuilder),
}

macro_rules! map_3d_builder {
    ($state:expr, $method:ident($($argument:expr),* $(,)?)) => {
        match $state {
            Plot3DBuilderState::Scatter(builder) => {
                Plot3DBuilderState::Scatter(builder.$method($($argument),*))
            }
            Plot3DBuilderState::Line(builder) => {
                Plot3DBuilderState::Line(builder.$method($($argument),*))
            }
            Plot3DBuilderState::Surface(builder) => {
                Plot3DBuilderState::Surface(builder.$method($($argument),*))
            }
            Plot3DBuilderState::Wireframe(builder) => {
                Plot3DBuilderState::Wireframe(builder.$method($($argument),*))
            }
        }
    };
}

impl Plot3DBuilderState {
    fn from_series(series: Series3DSnapshot) -> Self {
        match series {
            Series3DSnapshot::Scatter3d { x, y, z } => Self::Scatter(ruviz::scatter3d(&x, &y, &z)),
            Series3DSnapshot::Line3d { x, y, z } => Self::Line(ruviz::line3d(&x, &y, &z)),
            Series3DSnapshot::Surface { x, y, z } => Self::Surface(ruviz::surface(&x, &y, &z)),
            Series3DSnapshot::Wireframe { x, y, z } => {
                Self::Wireframe(ruviz::wireframe(&x, &y, &z))
            }
        }
    }

    fn add_series(self, series: Series3DSnapshot) -> Self {
        match series {
            Series3DSnapshot::Scatter3d { x, y, z } => match self {
                Self::Scatter(builder) => Self::Scatter(builder.scatter3d(&x, &y, &z)),
                Self::Line(builder) => Self::Scatter(builder.scatter3d(&x, &y, &z)),
                Self::Surface(builder) => Self::Scatter(builder.scatter3d(&x, &y, &z)),
                Self::Wireframe(builder) => Self::Scatter(builder.scatter3d(&x, &y, &z)),
            },
            Series3DSnapshot::Line3d { x, y, z } => match self {
                Self::Scatter(builder) => Self::Line(builder.line3d(&x, &y, &z)),
                Self::Line(builder) => Self::Line(builder.line3d(&x, &y, &z)),
                Self::Surface(builder) => Self::Line(builder.line3d(&x, &y, &z)),
                Self::Wireframe(builder) => Self::Line(builder.line3d(&x, &y, &z)),
            },
            Series3DSnapshot::Surface { x, y, z } => match self {
                Self::Scatter(builder) => Self::Surface(builder.surface(&x, &y, &z)),
                Self::Line(builder) => Self::Surface(builder.surface(&x, &y, &z)),
                Self::Surface(builder) => Self::Surface(builder.surface(&x, &y, &z)),
                Self::Wireframe(builder) => Self::Surface(builder.surface(&x, &y, &z)),
            },
            Series3DSnapshot::Wireframe { x, y, z } => match self {
                Self::Scatter(builder) => Self::Wireframe(builder.wireframe(&x, &y, &z)),
                Self::Line(builder) => Self::Wireframe(builder.wireframe(&x, &y, &z)),
                Self::Surface(builder) => Self::Wireframe(builder.wireframe(&x, &y, &z)),
                Self::Wireframe(builder) => Self::Wireframe(builder.wireframe(&x, &y, &z)),
            },
        }
    }

    fn render_png_bytes(self) -> ruviz::core::Result<Vec<u8>> {
        match self {
            Self::Scatter(builder) => builder.render_png_bytes(),
            Self::Line(builder) => builder.render_png_bytes(),
            Self::Surface(builder) => builder.render_png_bytes(),
            Self::Wireframe(builder) => builder.render_png_bytes(),
        }
    }

    fn render_to_svg(self) -> ruviz::core::Result<String> {
        match self {
            Self::Scatter(builder) => builder.render_to_svg(),
            Self::Line(builder) => builder.render_to_svg(),
            Self::Surface(builder) => builder.render_to_svg(),
            Self::Wireframe(builder) => builder.render_to_svg(),
        }
    }

    fn save(self, path: &str) -> ruviz::core::Result<()> {
        match self {
            Self::Scatter(builder) => builder.save(path),
            Self::Line(builder) => builder.save(path),
            Self::Surface(builder) => builder.save(path),
            Self::Wireframe(builder) => builder.save(path),
        }
    }
}

impl Plot3DSnapshot {
    fn into_builder(self) -> Result<Plot3DBuilderState, String> {
        let mut series = self.series.into_iter();
        let Some(first) = series.next() else {
            return Err("a 3D plot must contain at least one series".to_string());
        };
        let mut builder = Plot3DBuilderState::from_series(first);
        for series in series {
            builder = builder.add_series(series);
        }

        if let Some([width, height]) = self.size_px {
            let dpi = self.dpi.unwrap_or(100).max(1);
            builder = map_3d_builder!(
                builder,
                size(width as f32 / dpi as f32, height as f32 / dpi as f32)
            );
        }
        if let Some(dpi) = self.dpi {
            builder = map_3d_builder!(builder, dpi(dpi));
        }
        if let Some(theme) = self.theme {
            let theme = match theme.as_str() {
                "light" => Theme::light(),
                "dark" => Theme::dark(),
                other => return Err(format!("unsupported theme: {other}")),
            };
            builder = map_3d_builder!(builder, theme(theme));
        }
        if let Some(title) = self.title {
            builder = map_3d_builder!(builder, title(title));
        }
        if let Some(label) = self.x_label {
            builder = map_3d_builder!(builder, xlabel(label));
        }
        if let Some(label) = self.y_label {
            builder = map_3d_builder!(builder, ylabel(label));
        }
        if let Some(label) = self.z_label {
            builder = map_3d_builder!(builder, zlabel(label));
        }
        if let Some([minimum, maximum]) = self.x_lim {
            builder = map_3d_builder!(builder, xlim(minimum, maximum));
        }
        if let Some([minimum, maximum]) = self.y_lim {
            builder = map_3d_builder!(builder, ylim(minimum, maximum));
        }
        if let Some([minimum, maximum]) = self.z_lim {
            builder = map_3d_builder!(builder, zlim(minimum, maximum));
        }
        if let Some(degrees) = self.azimuth_deg {
            builder = map_3d_builder!(builder, azimuth_deg(degrees));
        }
        if let Some(degrees) = self.elevation_deg {
            builder = map_3d_builder!(builder, elevation_deg(degrees));
        }
        match self.projection {
            Some(Projection3DSnapshot::Orthographic) => {
                builder = map_3d_builder!(builder, orthographic());
            }
            Some(Projection3DSnapshot::Perspective) => {
                builder = map_3d_builder!(
                    builder,
                    perspective_deg(self.perspective_deg.unwrap_or(45.0))
                );
            }
            None => {}
        }

        Ok(builder)
    }
}

#[pyfunction]
fn render_png_bytes<'py>(py: Python<'py>, snapshot_json: &str) -> PyResult<Bound<'py, PyBytes>> {
    let plot = parse_plot(snapshot_json)?;
    let bytes = plot
        .render_png_bytes()
        .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;
    Ok(PyBytes::new(py, &bytes))
}

#[pyfunction]
fn render_svg(snapshot_json: &str) -> PyResult<String> {
    let plot = parse_plot(snapshot_json)?;
    plot.render_to_svg()
        .map_err(|err| PyRuntimeError::new_err(err.to_string()))
}

#[pyfunction]
fn save(snapshot_json: &str, path: &str) -> PyResult<()> {
    let plot = parse_plot(snapshot_json)?;
    let extension = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_else(|| "png".to_string());

    match extension.as_str() {
        "svg" => plot
            .export_svg(path)
            .map_err(|err| PyRuntimeError::new_err(err.to_string())),
        "pdf" => plot
            .save_pdf(path)
            .map_err(|err| PyRuntimeError::new_err(err.to_string())),
        _ => plot
            .save(path)
            .map_err(|err| PyRuntimeError::new_err(err.to_string())),
    }
}

#[pyfunction]
fn show_native(snapshot_json: &str) -> PyResult<()> {
    #[cfg(not(feature = "native-interactive"))]
    {
        let _ = snapshot_json;
        return Err(PyRuntimeError::new_err(
            NATIVE_INTERACTIVE_UNAVAILABLE_MESSAGE,
        ));
    }

    #[cfg(feature = "native-interactive")]
    let plot = parse_plot(snapshot_json)?;
    #[cfg(feature = "native-interactive")]
    {
        pollster::block_on(show_interactive(plot))
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))
    }
}

#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pyclass(module = "ruviz._native", unsendable)]
struct NativePlot3DHandle {
    snapshot: Plot3DSnapshot,
}

impl NativePlot3DHandle {
    fn builder(&self) -> PyResult<Plot3DBuilderState> {
        self.snapshot
            .clone()
            .into_builder()
            .map_err(PyValueError::new_err)
    }
}

#[pymethods]
impl NativePlot3DHandle {
    #[new]
    fn new() -> Self {
        Self {
            snapshot: Plot3DSnapshot::default(),
        }
    }

    fn scatter3d(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<f64>) {
        self.snapshot
            .series
            .push(Series3DSnapshot::Scatter3d { x, y, z });
    }

    fn line3d(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<f64>) {
        self.snapshot
            .series
            .push(Series3DSnapshot::Line3d { x, y, z });
    }

    fn surface(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<Vec<f64>>) {
        self.snapshot
            .series
            .push(Series3DSnapshot::Surface { x, y, z });
    }

    fn wireframe(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<Vec<f64>>) {
        self.snapshot
            .series
            .push(Series3DSnapshot::Wireframe { x, y, z });
    }

    fn size_px(&mut self, width: u32, height: u32) {
        self.snapshot.size_px = Some([width, height]);
    }

    fn dpi(&mut self, dpi: u32) {
        self.snapshot.dpi = Some(dpi);
    }

    fn theme(&mut self, theme: &str) -> PyResult<()> {
        if !matches!(theme, "light" | "dark") {
            return Err(PyValueError::new_err(format!("unsupported theme: {theme}")));
        }
        self.snapshot.theme = Some(theme.to_string());
        Ok(())
    }

    fn title(&mut self, title: &str) {
        self.snapshot.title = Some(title.to_string());
    }

    fn xlabel(&mut self, label: &str) {
        self.snapshot.x_label = Some(label.to_string());
    }

    fn ylabel(&mut self, label: &str) {
        self.snapshot.y_label = Some(label.to_string());
    }

    fn zlabel(&mut self, label: &str) {
        self.snapshot.z_label = Some(label.to_string());
    }

    fn xlim(&mut self, minimum: f64, maximum: f64) {
        self.snapshot.x_lim = Some([minimum, maximum]);
    }

    fn ylim(&mut self, minimum: f64, maximum: f64) {
        self.snapshot.y_lim = Some([minimum, maximum]);
    }

    fn zlim(&mut self, minimum: f64, maximum: f64) {
        self.snapshot.z_lim = Some([minimum, maximum]);
    }

    fn azimuth_deg(&mut self, degrees: f32) {
        self.snapshot.azimuth_deg = Some(degrees);
    }

    fn elevation_deg(&mut self, degrees: f32) {
        self.snapshot.elevation_deg = Some(degrees);
    }

    fn perspective_deg(&mut self, vertical_fov_deg: f32) {
        self.snapshot.projection = Some(Projection3DSnapshot::Perspective);
        self.snapshot.perspective_deg = Some(vertical_fov_deg);
    }

    fn orthographic(&mut self) {
        self.snapshot.projection = Some(Projection3DSnapshot::Orthographic);
        self.snapshot.perspective_deg = None;
    }

    fn render_png_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self
            .builder()?
            .render_png_bytes()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn render_svg(&self) -> PyResult<String> {
        self.builder()?
            .render_to_svg()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))
    }

    fn save(&self, path: &str) -> PyResult<()> {
        self.builder()?
            .save(path)
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))
    }
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<NativeObservable1D>()?;
    module.add_class::<NativePlotHandle>()?;
    module.add_class::<NativePlot3DHandle>()?;
    module.add_function(wrap_pyfunction!(render_png_bytes, module)?)?;
    module.add_function(wrap_pyfunction!(render_svg, module)?)?;
    module.add_function(wrap_pyfunction!(save, module)?)?;
    module.add_function(wrap_pyfunction!(show_native, module)?)?;
    module.add_function(wrap_pyfunction!(version, module)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use image::RgbaImage;
    use serde_json::json;

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

        let mut diff_pixels = 0usize;
        for (left, right) in actual.pixels().zip(expected.pixels()) {
            if left != right {
                diff_pixels += 1;
            }
        }

        assert_eq!(
            diff_pixels, 0,
            "{name}: rendered PNG differs at {diff_pixels} pixel(s)"
        );
    }

    fn render_snapshot_png(snapshot: serde_json::Value) -> Vec<u8> {
        parse_plot(&snapshot.to_string())
            .expect("snapshot should parse")
            .render_png_bytes()
            .expect("snapshot render should succeed")
    }

    fn render_direct_png(plot: Plot) -> Vec<u8> {
        plot.render_png_bytes()
            .expect("direct render should succeed")
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

    #[test]
    fn snapshot_render_matches_direct_plot_for_xy_series() {
        let x = vec![0.0, 1.0, 2.0, 3.0];
        let y = vec![0.2, 1.1, 0.7, 1.8];
        let scatter_y = vec![0.3, 1.0, 0.9, 1.6];

        let snapshot = json!({
            "sizePx": [320, 200],
            "theme": "light",
            "ticks": true,
            "title": "XY parity",
            "xLabel": "x",
            "yLabel": "y",
            "series": [
                {
                    "kind": "line",
                    "x": {"kind": "static", "values": x},
                    "y": {"kind": "static", "values": y},
                },
                {
                    "kind": "scatter",
                    "x": {"kind": "static", "values": [0.0, 1.0, 2.0, 3.0]},
                    "y": {"kind": "static", "values": scatter_y},
                }
            ]
        });

        let expected = base_plot("XY parity")
            .line(&[0.0, 1.0, 2.0, 3.0], &[0.2, 1.1, 0.7, 1.8])
            .scatter(&[0.0, 1.0, 2.0, 3.0], &[0.3, 1.0, 0.9, 1.6])
            .into_plot();

        let actual_png = render_snapshot_png(snapshot);
        let expected_png = render_direct_png(expected);
        assert_pngs_equal("xy_series", &actual_png, &expected_png);
    }

    #[test]
    fn snapshot_render_matches_direct_plot_for_statistical_series() {
        let samples = vec![
            -2.3, -1.9, -1.1, -0.4, 0.2, 0.8, 1.0, 1.4, 1.7, 2.1, 2.5, 2.9,
        ];

        let snapshot = json!({
            "sizePx": [320, 200],
            "theme": "light",
            "ticks": true,
            "title": "Histogram parity",
            "xLabel": "value",
            "yLabel": "count",
            "series": [
                {
                    "kind": "histogram",
                    "data": {"kind": "static", "values": samples},
                }
            ]
        });

        let expected = base_plot("Histogram parity")
            .xlabel("value")
            .ylabel("count")
            .histogram(&[
                -2.3, -1.9, -1.1, -0.4, 0.2, 0.8, 1.0, 1.4, 1.7, 2.1, 2.5, 2.9,
            ])
            .into_plot();

        let actual_png = render_snapshot_png(snapshot);
        let expected_png = render_direct_png(expected);
        assert_pngs_equal("histogram", &actual_png, &expected_png);
    }

    #[test]
    fn snapshot_render_matches_direct_plot_for_matrix_and_errorbar_series() {
        let heatmap_values = vec![
            vec![0.1, 0.4, 0.8],
            vec![0.3, 0.5, 0.7],
            vec![0.2, 0.6, 0.9],
        ];
        let flat_heatmap = vec![0.1, 0.4, 0.8, 0.3, 0.5, 0.7, 0.2, 0.6, 0.9];

        let heatmap_snapshot = json!({
            "sizePx": [320, 200],
            "theme": "dark",
            "ticks": false,
            "title": "Heatmap parity",
            "series": [
                {
                    "kind": "heatmap",
                    "values": flat_heatmap,
                    "rows": 3,
                    "cols": 3,
                }
            ]
        });

        let heatmap_expected = Plot::new()
            .size_px(320, 200)
            .theme(Theme::dark())
            .ticks(false)
            .title("Heatmap parity")
            .heatmap(&heatmap_values)
            .into_plot();

        let actual_png = render_snapshot_png(heatmap_snapshot);
        let expected_png = render_direct_png(heatmap_expected);
        assert_pngs_equal("heatmap", &actual_png, &expected_png);

        let error_snapshot = json!({
            "sizePx": [320, 200],
            "theme": "light",
            "ticks": true,
            "title": "Error parity",
            "xLabel": "x",
            "yLabel": "y",
            "series": [
                {
                    "kind": "error-bars-xy",
                    "x": {"kind": "static", "values": [1.0, 2.0, 3.0]},
                    "y": {"kind": "static", "values": [1.2, 1.8, 1.4]},
                    "xErrors": {"kind": "static", "values": [0.1, 0.15, 0.12]},
                    "yErrors": {"kind": "static", "values": [0.2, 0.18, 0.16]},
                }
            ]
        });

        let error_expected = base_plot("Error parity")
            .error_bars_xy(
                &[1.0, 2.0, 3.0],
                &[1.2, 1.8, 1.4],
                &[0.1, 0.15, 0.12],
                &[0.2, 0.18, 0.16],
            )
            .into_plot();

        let actual_png = render_snapshot_png(error_snapshot);
        let expected_png = render_direct_png(error_expected);
        assert_pngs_equal("error_bars_xy", &actual_png, &expected_png);
    }

    #[test]
    fn snapshot_render_matches_direct_plot_for_specialized_series() {
        let contour_snapshot = json!({
            "sizePx": [320, 200],
            "theme": "light",
            "ticks": true,
            "title": "Contour parity",
            "series": [
                {
                    "kind": "contour",
                    "x": [-1.0, 0.0, 1.0],
                    "y": [-1.0, 0.0, 1.0],
                    "z": [0.1, 0.2, 0.3, 0.2, 0.6, 0.2, 0.3, 0.2, 0.1],
                }
            ]
        });

        let contour_expected = Plot::new()
            .size_px(320, 200)
            .theme(Theme::light())
            .ticks(true)
            .title("Contour parity")
            .contour(
                &[-1.0, 0.0, 1.0],
                &[-1.0, 0.0, 1.0],
                &[0.1, 0.2, 0.3, 0.2, 0.6, 0.2, 0.3, 0.2, 0.1],
            )
            .into_plot();

        let actual_png = render_snapshot_png(contour_snapshot);
        let expected_png = render_direct_png(contour_expected);
        assert_pngs_equal("contour", &actual_png, &expected_png);

        let pie_snapshot = json!({
            "sizePx": [320, 200],
            "theme": "light",
            "ticks": true,
            "title": "Pie parity",
            "series": [
                {
                    "kind": "pie",
                    "values": [30.0, 25.0, 20.0, 25.0],
                    "labels": ["A", "B", "C", "D"],
                }
            ]
        });

        let pie_expected = Plot::new()
            .size_px(320, 200)
            .theme(Theme::light())
            .ticks(true)
            .title("Pie parity")
            .pie(&[30.0, 25.0, 20.0, 25.0])
            .labels(&["A", "B", "C", "D"])
            .into_plot();

        let actual_png = render_snapshot_png(pie_snapshot);
        let expected_png = render_direct_png(pie_expected);
        assert_pngs_equal("pie", &actual_png, &expected_png);

        let radar_snapshot = json!({
            "sizePx": [320, 200],
            "theme": "light",
            "ticks": true,
            "title": "Radar parity",
            "series": [
                {
                    "kind": "radar",
                    "labels": ["API", "Docs", "Export", "Interactive", "Scale"],
                    "series": [
                        {"name": "Python", "values": [4.5, 4.7, 4.8, 4.3, 4.0]},
                        {"name": "Web", "values": [4.2, 4.1, 4.0, 4.8, 4.6]},
                    ],
                }
            ]
        });

        let radar_expected = Plot::new()
            .size_px(320, 200)
            .theme(Theme::light())
            .ticks(true)
            .title("Radar parity")
            .radar(&["API", "Docs", "Export", "Interactive", "Scale"])
            .add_series("Python", &[4.5, 4.7, 4.8, 4.3, 4.0])
            .add_series("Web", &[4.2, 4.1, 4.0, 4.8, 4.6])
            .into_plot();

        let actual_png = render_snapshot_png(radar_snapshot);
        let expected_png = render_direct_png(radar_expected);
        assert_pngs_equal("radar", &actual_png, &expected_png);
    }
}
