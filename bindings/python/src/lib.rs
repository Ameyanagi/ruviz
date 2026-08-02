use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
    types::PyBytes,
};
use ruviz::{
    core::{Line3DBuilder, Scatter3DBuilder, Surface3DBuilder, Wireframe3DBuilder},
    render::Theme,
};

mod native_handle;

use native_handle::{NativeObservable1D, NativePlotHandle, save_extension};

#[cfg(not(feature = "native-interactive"))]
pub(crate) const NATIVE_INTERACTIVE_UNAVAILABLE_MESSAGE: &str = "native interactive windows are unavailable in this wheel; install ruviz from source on Linux to enable plot.show()";

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

#[derive(Clone)]
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
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pyclass(module = "ruviz._native", unsendable)]
struct NativePlot3DHandle {
    snapshot: Plot3DSnapshot,
    /// Built builder reused across renders; `None` whenever a mutator ran since
    /// the last build. Render calls consume a builder, so they take a clone.
    cached: Option<Plot3DBuilderState>,
}

impl NativePlot3DHandle {
    fn builder(&mut self) -> PyResult<Plot3DBuilderState> {
        if self.cached.is_none() {
            self.cached = Some(
                self.snapshot
                    .clone()
                    .into_builder()
                    .map_err(PyValueError::new_err)?,
            );
        }
        Ok(self.cached.clone().expect("builder cached above"))
    }

    fn mark_dirty(&mut self) {
        self.cached = None;
    }
}

#[pymethods]
impl NativePlot3DHandle {
    #[new]
    fn new() -> Self {
        Self {
            snapshot: Plot3DSnapshot::default(),
            cached: None,
        }
    }

    fn scatter3d(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<f64>) {
        self.snapshot
            .series
            .push(Series3DSnapshot::Scatter3d { x, y, z });
        self.mark_dirty();
    }

    fn line3d(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<f64>) {
        self.snapshot
            .series
            .push(Series3DSnapshot::Line3d { x, y, z });
        self.mark_dirty();
    }

    fn surface(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<Vec<f64>>) {
        self.snapshot
            .series
            .push(Series3DSnapshot::Surface { x, y, z });
        self.mark_dirty();
    }

    fn wireframe(&mut self, x: Vec<f64>, y: Vec<f64>, z: Vec<Vec<f64>>) {
        self.snapshot
            .series
            .push(Series3DSnapshot::Wireframe { x, y, z });
        self.mark_dirty();
    }

    fn size_px(&mut self, width: u32, height: u32) {
        self.snapshot.size_px = Some([width, height]);
        self.mark_dirty();
    }

    fn dpi(&mut self, dpi: u32) {
        self.snapshot.dpi = Some(dpi);
        self.mark_dirty();
    }

    fn theme(&mut self, theme: &str) -> PyResult<()> {
        if !matches!(theme, "light" | "dark") {
            return Err(PyValueError::new_err(format!("unsupported theme: {theme}")));
        }
        self.snapshot.theme = Some(theme.to_string());
        self.mark_dirty();
        Ok(())
    }

    fn title(&mut self, title: &str) {
        self.snapshot.title = Some(title.to_string());
        self.mark_dirty();
    }

    fn xlabel(&mut self, label: &str) {
        self.snapshot.x_label = Some(label.to_string());
        self.mark_dirty();
    }

    fn ylabel(&mut self, label: &str) {
        self.snapshot.y_label = Some(label.to_string());
        self.mark_dirty();
    }

    fn zlabel(&mut self, label: &str) {
        self.snapshot.z_label = Some(label.to_string());
        self.mark_dirty();
    }

    fn xlim(&mut self, minimum: f64, maximum: f64) {
        self.snapshot.x_lim = Some([minimum, maximum]);
        self.mark_dirty();
    }

    fn ylim(&mut self, minimum: f64, maximum: f64) {
        self.snapshot.y_lim = Some([minimum, maximum]);
        self.mark_dirty();
    }

    fn zlim(&mut self, minimum: f64, maximum: f64) {
        self.snapshot.z_lim = Some([minimum, maximum]);
        self.mark_dirty();
    }

    fn azimuth_deg(&mut self, degrees: f32) {
        self.snapshot.azimuth_deg = Some(degrees);
        self.mark_dirty();
    }

    fn elevation_deg(&mut self, degrees: f32) {
        self.snapshot.elevation_deg = Some(degrees);
        self.mark_dirty();
    }

    fn perspective_deg(&mut self, vertical_fov_deg: f32) {
        self.snapshot.projection = Some(Projection3DSnapshot::Perspective);
        self.snapshot.perspective_deg = Some(vertical_fov_deg);
        self.mark_dirty();
    }

    fn orthographic(&mut self) {
        self.snapshot.projection = Some(Projection3DSnapshot::Orthographic);
        self.snapshot.perspective_deg = None;
        self.mark_dirty();
    }

    fn render_png_bytes<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self
            .builder()?
            .render_png_bytes()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn render_svg(&mut self) -> PyResult<String> {
        self.builder()?
            .render_to_svg()
            .map_err(|error| PyRuntimeError::new_err(error.to_string()))
    }

    fn save(&mut self, path: &str) -> PyResult<()> {
        save_extension(path)?;
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
    module.add_function(wrap_pyfunction!(version, module)?)?;
    Ok(())
}
