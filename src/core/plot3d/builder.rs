use std::path::Path;

use crate::core::{Bounds3D, FigureConfig, Image, Legend, PlottingError, Result};
use crate::data::{NumericData1D, NumericData2D};
use crate::plots::three_d::{Grid3DData, Points3DData};
use crate::plots::{
    Line3DConfig, Scatter3DConfig, Surface3DConfig, SurfaceSampling, SurfaceShading,
    Wireframe3DConfig,
};
use crate::render::color::ColorMapSpec;
use crate::render::three_d::overlay::{compose_image, compose_svg};
use crate::render::three_d::software::raster::SoftwareRenderOptions3D;
use crate::render::{Color, ColorMap, LineStyle, MarkerStyle, Theme};

#[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
use super::prepared::render_resolved_gpu_layer;
use super::prepared::render_resolved_software_layer;
use super::{AxisAspect3D, Camera3D, Point3D};

#[derive(Clone, Debug, Default)]
pub(crate) struct Plot3D {
    pub(super) series: Vec<Series3D>,
    pub(super) camera: Camera3D,
    pub(super) figure: FigureConfig,
    pub(super) theme: Theme,
    pub(super) title: Option<String>,
    pub(super) xlabel: Option<String>,
    pub(super) ylabel: Option<String>,
    pub(super) zlabel: Option<String>,
    pub(super) xlim: Option<(f64, f64)>,
    pub(super) ylim: Option<(f64, f64)>,
    pub(super) zlim: Option<(f64, f64)>,
    /// User legend configuration, or `None` to derive one from the theme.
    ///
    /// `Some` is honoured verbatim — including `Legend::enabled == false`,
    /// which turns the 3D legend off — and is the same [`Legend`] type the 2D
    /// legend uses, laid out by the same `layout_legend`.
    pub(super) legend: Option<Legend>,
    pub(super) pending_error: Option<PlottingError>,
}

impl Plot3D {
    fn set_pending_error(&mut self, error: PlottingError) {
        if self.pending_error.is_none() {
            self.pending_error = Some(error);
        }
    }

    fn validate_and_bounds(self) -> Result<Bounds3D> {
        self.resolve().map(|frame| frame.bounds)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Series3D {
    Scatter {
        data: Points3DData,
        config: Scatter3DConfig,
        label: Option<String>,
    },
    Line {
        data: Points3DData,
        config: Line3DConfig,
        label: Option<String>,
    },
    Surface {
        data: Grid3DData,
        config: Surface3DConfig,
        label: Option<String>,
    },
    Wireframe {
        data: Grid3DData,
        config: Wireframe3DConfig,
        label: Option<String>,
    },
}

impl Series3D {
    pub(super) fn bounds(&self) -> Result<Bounds3D> {
        match self {
            Self::Scatter { data, .. } | Self::Line { data, .. } => point_data_bounds(data),
            Self::Surface { data, .. } | Self::Wireframe { data, .. } => grid_data_bounds(data),
        }
    }

    pub(super) fn validate_style(&self) -> Result<()> {
        match self {
            Self::Scatter { config, label, .. } => {
                validate_positive_style("scatter3d marker size", config.marker_size)?;
                validate_opaque_color("scatter3d", config.color)?;
                validate_label(label)
            }
            Self::Line { config, label, .. } => {
                validate_positive_style("line3d line width", config.line_width)?;
                validate_opaque_color("line3d", config.color)?;
                validate_label(label)
            }
            Self::Surface { config, label, .. } => {
                validate_sampling("surface", config.sampling)?;
                validate_opaque_color("surface", config.color)?;
                if config.colormap.colors().iter().any(|color| color.a != 255) {
                    return Err(PlottingError::InvalidInput(
                        "surface: transparent colormap entries are unsupported in the opaque MVP"
                            .to_string(),
                    ));
                }
                validate_label(label)
            }
            Self::Wireframe { config, label, .. } => {
                validate_positive_style("wireframe line width", config.line_width)?;
                validate_sampling("wireframe", config.sampling)?;
                validate_opaque_color("wireframe", config.color)?;
                validate_label(label)
            }
        }
    }
}

fn validate_opaque_color(operation: &str, color: Option<Color>) -> Result<()> {
    if color.is_some_and(|color| color.a != 255) {
        return Err(PlottingError::InvalidInput(format!(
            "{operation}: transparency is unsupported in the opaque 3D MVP"
        )));
    }
    Ok(())
}

fn validate_label(label: &Option<String>) -> Result<()> {
    if label.as_deref().is_some_and(str::is_empty) {
        return Err(PlottingError::InvalidInput(
            "3D series label cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_sampling(operation: &str, sampling: SurfaceSampling) -> Result<()> {
    if let SurfaceSampling::MaxGrid { rows, columns } = sampling
        && (rows < 2 || columns < 2)
    {
        return Err(PlottingError::InvalidTopology3D {
            reason: format!(
                "{operation} sampling dimensions must each be at least 2, got {rows}x{columns}"
            ),
        });
    }
    Ok(())
}

fn validate_positive_style(name: &str, value: f32) -> Result<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(PlottingError::InvalidInput(format!(
            "{name} must be finite and greater than zero, got {value}"
        )))
    }
}

pub(super) fn validate_figure(figure: &FigureConfig) -> Result<()> {
    if !figure.width.is_finite()
        || !figure.height.is_finite()
        || figure.width <= 0.0
        || figure.height <= 0.0
    {
        let (width, height) = figure.canvas_size();
        return Err(PlottingError::InvalidDimensions { width, height });
    }
    if !figure.dpi.is_finite() || figure.dpi <= 0.0 {
        return Err(PlottingError::InvalidDPI(figure.dpi.max(0.0) as u32));
    }
    let (width, height) = figure.canvas_size();
    if width == 0 || height == 0 {
        return Err(PlottingError::InvalidDimensions { width, height });
    }
    Ok(())
}

fn point_data_bounds(data: &Points3DData) -> Result<Bounds3D> {
    Bounds3D::from_points(
        data.x
            .iter()
            .zip(data.y.iter())
            .zip(data.z.iter())
            .map(|((&x, &y), &z)| Point3D::new(x, y, z)),
    )
}

fn grid_data_bounds(data: &Grid3DData) -> Result<Bounds3D> {
    Bounds3D::from_points((0..data.z.len()).map(|index| {
        let row = index / data.columns;
        let column = index % data.columns;
        Point3D::new(data.x[column], data.y[row], data.z[index])
    }))
}

fn interaction_not_available() -> PlottingError {
    PlottingError::RenderError(
        "interactive 3D presentation is not available yet; use render(), save(), or render_to_svg() for deterministic CPU output"
            .to_string(),
    )
}

impl Plot3D {
    fn render_image(self) -> Result<(Image, super::RenderDiagnostics3D)> {
        let prepared = self.render_software_layer(SoftwareRenderOptions3D::export())?;
        compose_software_image(prepared)
    }

    fn render_svg(self) -> Result<(String, super::RenderDiagnostics3D)> {
        let prepared = self.render_software_layer(SoftwareRenderOptions3D::export())?;
        let svg = compose_svg(
            &prepared.layout,
            &prepared.frame.figure,
            &prepared.frame.theme,
            &prepared.output.layer,
        )?;
        Ok((svg, prepared.diagnostics))
    }

    #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
    fn render_gpu_image(self) -> Result<(Image, super::RenderDiagnostics3D)> {
        let prepared = self.render_gpu_layer()?;
        compose_gpu_image(prepared)
    }

    fn render_auto_image(self) -> Result<(Image, super::RenderDiagnostics3D)> {
        let frame = self.resolve()?;
        #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
        {
            match render_resolved_gpu_layer(frame.clone()) {
                Ok(prepared) => compose_gpu_image(prepared),
                Err(error) => {
                    log::debug!("automatic direct 3d GPU rendering failed: {error}");
                    let mut prepared =
                        render_resolved_software_layer(frame, SoftwareRenderOptions3D::export())?;
                    prepared.diagnostics.fallback_reason =
                        Some(AUTO_GPU_RUNTIME_FALLBACK_REASON.to_string());
                    compose_software_image(prepared)
                }
            }
        }
        #[cfg(not(all(feature = "gpu", not(target_arch = "wasm32"))))]
        {
            let mut prepared =
                render_resolved_software_layer(frame, SoftwareRenderOptions3D::export())?;
            prepared.diagnostics.fallback_reason = Some(auto_gpu_unavailable_reason().to_string());
            compose_software_image(prepared)
        }
    }

    #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
    fn render_gpu_svg(self) -> Result<(String, super::RenderDiagnostics3D)> {
        let prepared = self.render_gpu_layer()?;
        let svg = compose_svg(
            &prepared.layout,
            &prepared.frame.figure,
            &prepared.frame.theme,
            &prepared.layer,
        )?;
        Ok((svg, prepared.diagnostics))
    }
}

fn compose_software_image(
    prepared: super::prepared::PreparedSoftwareFrame3D,
) -> Result<(Image, super::RenderDiagnostics3D)> {
    let image = compose_image(
        &prepared.layout,
        &prepared.frame.figure,
        &prepared.frame.theme,
        &prepared.output.layer,
    )?;
    Ok((image, prepared.diagnostics))
}

#[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
fn compose_gpu_image(
    prepared: super::prepared::PreparedGpuFrame3D,
) -> Result<(Image, super::RenderDiagnostics3D)> {
    let image = compose_image(
        &prepared.layout,
        &prepared.frame.figure,
        &prepared.frame.theme,
        &prepared.layer,
    )?;
    Ok((image, prepared.diagnostics))
}

#[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
const AUTO_GPU_RUNTIME_FALLBACK_REASON: &str =
    "direct native 3d GPU rendering was unavailable; CPU fallback executed";

#[cfg(all(not(feature = "gpu"), not(target_arch = "wasm32")))]
const fn auto_gpu_unavailable_reason() -> &'static str {
    "direct native 3d GPU rendering is unavailable because the `gpu` feature is disabled"
}

#[cfg(target_arch = "wasm32")]
const fn auto_gpu_unavailable_reason() -> &'static str {
    "synchronous 3d image readback is unavailable on wasm32; use a WebGPU canvas session for direct presentation"
}

macro_rules! impl_common_builder {
    ($builder:ident) => {
        impl $builder {
            /// Set the plot title.
            pub fn title(mut self, title: impl Into<String>) -> Self {
                self.plot.title = Some(title.into());
                self
            }

            /// Set the x-axis label.
            pub fn xlabel(mut self, label: impl Into<String>) -> Self {
                self.plot.xlabel = Some(label.into());
                self
            }

            /// Set the y-axis label.
            pub fn ylabel(mut self, label: impl Into<String>) -> Self {
                self.plot.ylabel = Some(label.into());
                self
            }

            /// Set the z-axis label.
            pub fn zlabel(mut self, label: impl Into<String>) -> Self {
                self.plot.zlabel = Some(label.into());
                self
            }

            /// Set finite ascending x-axis limits.
            pub fn xlim(mut self, minimum: f64, maximum: f64) -> Self {
                self.plot.xlim = Some((minimum, maximum));
                self
            }

            /// Set finite ascending y-axis limits.
            pub fn ylim(mut self, minimum: f64, maximum: f64) -> Self {
                self.plot.ylim = Some((minimum, maximum));
                self
            }

            /// Set finite ascending z-axis limits.
            pub fn zlim(mut self, minimum: f64, maximum: f64) -> Self {
                self.plot.zlim = Some((minimum, maximum));
                self
            }

            /// Replace the 3D camera.
            pub fn camera(mut self, camera: Camera3D) -> Self {
                self.plot.camera = camera;
                self
            }

            /// Orbit around an explicit point in data coordinates.
            ///
            /// A target outside the resolved bounds is clamped to the plotting
            /// box, so the plot always stays in front of the camera.
            pub fn look_at(mut self, x: f64, y: f64, z: f64) -> Self {
                self.plot.camera = self.plot.camera.look_at(Point3D::new(x, y, z));
                self
            }

            /// Set camera azimuth in degrees.
            pub fn azimuth_deg(mut self, degrees: f32) -> Self {
                self.plot.camera = self.plot.camera.azimuth_deg(degrees);
                self
            }

            /// Set camera elevation in degrees.
            pub fn elevation_deg(mut self, degrees: f32) -> Self {
                self.plot.camera = self.plot.camera.elevation_deg(degrees);
                self
            }

            /// Set camera roll in degrees.
            pub fn roll_deg(mut self, degrees: f32) -> Self {
                self.plot.camera = self.plot.camera.roll_deg(degrees);
                self
            }

            /// Select perspective projection with a vertical field of view.
            pub fn perspective_deg(mut self, vertical_fov_deg: f32) -> Self {
                self.plot.camera = self.plot.camera.perspective_deg(vertical_fov_deg);
                self
            }

            /// Select orthographic projection.
            pub fn orthographic(mut self) -> Self {
                self.plot.camera = self.plot.camera.orthographic();
                self
            }

            /// Set plotting-box proportions.
            pub fn axis_aspect(mut self, aspect: AxisAspect3D) -> Self {
                self.plot.camera = self.plot.camera.axis_aspect(aspect);
                self
            }

            /// Set figure size in inches.
            ///
            /// Same vocabulary as the 2D builder's `Plot::size`.
            ///
            /// The final pixel dimensions are `width × dpi` by `height × dpi`;
            /// use [`Self::size_px`] to specify pixels directly.
            pub fn size(mut self, width: f32, height: f32) -> Self {
                self.plot.figure.width = width;
                self.plot.figure.height = height;
                self
            }

            /// Set figure size in pixels, at the reference DPI (100).
            ///
            /// Same vocabulary as the 2D builder's `Plot::size_px`. Call
            /// `.dpi()` afterwards to change output resolution without
            /// changing layout proportions.
            pub fn size_px(mut self, width: u32, height: u32) -> Self {
                use crate::core::units::REFERENCE_DPI;
                self.plot.figure.width = width as f32 / REFERENCE_DPI;
                self.plot.figure.height = height as f32 / REFERENCE_DPI;
                self
            }

            /// Set figure size in inches.
            ///
            /// # Deprecated
            ///
            /// Renamed to [`Self::size`] so 2D and 3D share one sizing
            /// vocabulary (`size` / `size_px`).
            #[deprecated(
                since = "0.6.0",
                note = "renamed: use `size(width_in, height_in)` (or `size_px(width_px, height_px)`)"
            )]
            pub fn figure_size(self, width: f32, height: f32) -> Self {
                self.size(width, height)
            }

            /// Set output dots per inch.
            pub fn dpi(mut self, dpi: u32) -> Self {
                self.plot.figure.dpi = dpi as f32;
                self
            }

            /// Set the plot theme.
            pub fn theme(mut self, theme: Theme) -> Self {
                self.plot.theme = theme;
                self
            }

            /// Configure the legend.
            ///
            /// Takes the same [`Legend`] the 2D API takes, and it is laid out
            /// by the same routine, so position, font size, columns, title,
            /// spacing and box style mean exactly what they mean in 2D. Without
            /// this call the legend is derived from the theme and appears
            /// whenever any series carries a label; pass `Legend::new()` (or any
            /// legend with `enabled` false) to suppress it.
            ///
            /// An inside position — including
            /// [`LegendPosition::Best`](crate::core::LegendPosition::Best),
            /// which resolves to one — places the legend within the plotting
            /// box. Every *outside* position resolves to the decoration band on
            /// the right of the figure, which is the only band a 3D figure
            /// reserves and the one the theme-derived default uses.
            pub fn legend(mut self, legend: Legend) -> Self {
                self.plot.legend = Some(legend);
                self
            }

            /// Continue with a 3D scatter series.
            pub fn scatter3d<X, Y, Z>(self, x: &X, y: &Y, z: &Z) -> Scatter3DBuilder
            where
                X: NumericData1D + ?Sized,
                Y: NumericData1D + ?Sized,
                Z: NumericData1D + ?Sized,
            {
                let data = Points3DData::collect("scatter3d", 1, x, y, z);
                Scatter3DBuilder::with_plot(self.finalize(), data)
            }

            /// Continue with a 3D line series.
            pub fn line3d<X, Y, Z>(self, x: &X, y: &Y, z: &Z) -> Line3DBuilder
            where
                X: NumericData1D + ?Sized,
                Y: NumericData1D + ?Sized,
                Z: NumericData1D + ?Sized,
            {
                let data = Points3DData::collect("line3d", 2, x, y, z);
                Line3DBuilder::with_plot(self.finalize(), data)
            }

            /// Continue with a regular-grid surface.
            pub fn surface<X, Y, Z>(self, x: &X, y: &Y, z: &Z) -> Surface3DBuilder
            where
                X: NumericData1D + ?Sized,
                Y: NumericData1D + ?Sized,
                Z: NumericData2D + ?Sized,
            {
                let data = Grid3DData::collect("surface", x, y, z);
                Surface3DBuilder::with_plot(self.finalize(), data)
            }

            /// Continue with a regular-grid wireframe.
            pub fn wireframe<X, Y, Z>(self, x: &X, y: &Y, z: &Z) -> Wireframe3DBuilder
            where
                X: NumericData1D + ?Sized,
                Y: NumericData1D + ?Sized,
                Z: NumericData2D + ?Sized,
            {
                let data = Grid3DData::collect("wireframe", x, y, z);
                Wireframe3DBuilder::with_plot(self.finalize(), data)
            }

            /// Validate data, styling, camera, and figure configuration.
            pub fn validate(self) -> Result<()> {
                self.finalize().validate_and_bounds().map(drop)
            }

            /// Return the combined data-space bounds after validation.
            pub fn data_bounds(self) -> Result<Bounds3D> {
                self.finalize().validate_and_bounds()
            }

            /// Pick the nearest surface triangle at a full-canvas pixel.
            ///
            /// Coordinates use the same top-left-origin pixel system as the
            /// image returned by [`Self::render`]. A point outside the Axis3
            /// viewport or over empty space returns `Ok(None)`.
            pub fn pick(self, screen_x: f32, screen_y: f32) -> Result<Option<super::PickHit3D>> {
                self.finalize().pick_at(screen_x, screen_y)
            }

            /// Project a data-space point to the full-canvas pixel it occupies.
            ///
            /// This is the exact inverse of [`Self::pick`] and uses the same
            /// top-left-origin pixel system, so `project(p)` feeds straight back
            /// into `pick(..)`. Returns `Ok(None)` when the point falls outside
            /// the camera's clip volume.
            ///
            /// Use this instead of hard-coding a pixel: the layout that decides
            /// where the scene sits is free to change, and a caller that asks
            /// where a point actually landed keeps working when it does.
            pub fn project(self, point: super::Point3D) -> Result<Option<(f32, f32)>> {
                self.finalize().project_at(point)
            }

            /// Create a retained backend-neutral 3d interaction session.
            pub fn interactive_session(self) -> Result<super::InteractivePlot3DSession> {
                super::InteractivePlot3DSession::new(self.finalize())
            }

            /// Create a retained session while preserving a previous camera view.
            pub fn interactive_session_with_view(
                self,
                snapshot: super::CameraSnapshot3D,
            ) -> Result<super::InteractivePlot3DSession> {
                let mut session = super::InteractivePlot3DSession::new(self.finalize())?;
                session.restore_camera(snapshot)?;
                Ok(session)
            }

            /// Compile the retained scene and return structured benchmark counters.
            #[doc(hidden)]
            pub fn benchmark_compile_scene_with_diagnostics(
                self,
            ) -> Result<super::RenderDiagnostics3D> {
                self.finalize()
                    .prepare_once()
                    .map(|(_, diagnostics)| diagnostics)
            }

            /// Render through the CPU 3D backend and return structured counters.
            #[doc(hidden)]
            pub fn benchmark_render_with_diagnostics(
                self,
            ) -> Result<(Image, super::RenderDiagnostics3D)> {
                self.finalize().render_image()
            }

            /// Automatically prefer direct native wgpu, with a diagnosed CPU fallback.
            ///
            /// This opt-in terminal leaves [`Self::render`] deterministic on
            /// the CPU reference backend. When direct native wgpu is compiled
            /// and available, `actual_backend` is `gpu3d`; otherwise the CPU
            /// result reports `actual_backend=cpu3d` and a non-empty fallback
            /// reason.
            pub fn render_auto_with_diagnostics(
                self,
            ) -> Result<(Image, super::RenderDiagnostics3D)> {
                self.finalize().render_auto_image()
            }

            /// Render through direct offscreen wgpu and return structured counters.
            #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
            #[doc(hidden)]
            pub fn benchmark_render_gpu_with_diagnostics(
                self,
            ) -> Result<(Image, super::RenderDiagnostics3D)> {
                self.finalize().render_gpu_image()
            }

            /// Create a retained no-readback GPU performance session.
            #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
            #[doc(hidden)]
            pub fn benchmark_gpu_session(self) -> Result<super::GpuBenchmarkSession3D> {
                super::GpuBenchmarkSession3D::new(self.finalize())
            }

            /// Render an in-memory image.
            pub fn render(self) -> Result<Image> {
                self.finalize().render_image().map(|(image, _)| image)
            }

            /// Automatically prefer direct native wgpu and fall back to CPU.
            ///
            /// Use [`Self::render_auto_with_diagnostics`] when the selected
            /// backend and fallback reason are needed.
            pub fn render_auto(self) -> Result<Image> {
                self.render_auto_with_diagnostics().map(|(image, _)| image)
            }

            /// Render an in-memory image through direct offscreen wgpu.
            ///
            /// This is an explicit GPU request and returns an error when no
            /// compatible adapter is available. The normal [`Self::render`]
            /// terminal remains the deterministic CPU reference.
            #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
            pub fn render_gpu(self) -> Result<Image> {
                self.finalize().render_gpu_image().map(|(image, _)| image)
            }

            /// Render PNG bytes through direct offscreen wgpu.
            #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
            pub fn render_gpu_png_bytes(self) -> Result<Vec<u8>> {
                self.render_gpu()?.encode_png()
            }

            /// Render PNG bytes.
            pub fn render_png_bytes(self) -> Result<Vec<u8>> {
                self.render()?.encode_png()
            }

            /// Save the plot, selecting an exporter from the path extension.
            #[cfg(not(target_arch = "wasm32"))]
            pub fn save<P: AsRef<Path>>(self, path: P) -> Result<()> {
                let path = path.as_ref();
                let extension = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or("png")
                    .to_ascii_lowercase();
                match extension.as_str() {
                    "png" => {
                        let bytes = self.render_png_bytes()?;
                        crate::export::write_bytes_atomic(path, &bytes)
                    }
                    "svg" => {
                        let svg = self.render_to_svg()?;
                        crate::export::write_bytes_atomic(path, svg.as_bytes())
                    }
                    #[cfg(feature = "pdf")]
                    "pdf" => {
                        let svg = self.render_to_svg()?;
                        let pdf = crate::export::svg_to_pdf(&svg)?;
                        crate::export::write_bytes_atomic(path, &pdf)
                    }
                    _ => Err(PlottingError::UnsupportedFormat(extension)),
                }
            }

            /// Save through direct offscreen wgpu.
            ///
            /// PNG uses the composited GPU image. SVG and PDF keep vector
            /// Axis3 text around one depth-tested GPU raster layer.
            #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
            pub fn save_gpu<P: AsRef<Path>>(self, path: P) -> Result<()> {
                let path = path.as_ref();
                let extension = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or("png")
                    .to_ascii_lowercase();
                match extension.as_str() {
                    "png" => {
                        let bytes = self.render_gpu_png_bytes()?;
                        crate::export::write_bytes_atomic(path, &bytes)
                    }
                    "svg" => {
                        let svg = self.finalize().render_gpu_svg()?.0;
                        crate::export::write_bytes_atomic(path, svg.as_bytes())
                    }
                    #[cfg(feature = "pdf")]
                    "pdf" => {
                        let svg = self.finalize().render_gpu_svg()?.0;
                        let pdf = crate::export::svg_to_pdf(&svg)?;
                        crate::export::write_bytes_atomic(path, &pdf)
                    }
                    _ => Err(PlottingError::UnsupportedFormat(extension)),
                }
            }

            /// Render a hybrid SVG document.
            pub fn render_to_svg(self) -> Result<String> {
                self.finalize().render_svg().map(|(svg, _)| svg)
            }

            /// Save a hybrid SVG document.
            #[cfg(not(target_arch = "wasm32"))]
            pub fn export_svg<P: AsRef<Path>>(self, path: P) -> Result<()> {
                let svg = self.render_to_svg()?;
                std::fs::write(path, svg)?;
                Ok(())
            }

            /// Show the plot in a native interactive window.
            #[cfg(all(feature = "interactive-gpu", not(target_arch = "wasm32")))]
            pub fn show(self) -> Result<()> {
                let session = super::InteractivePlot3DSession::new(self.finalize())?;
                crate::interactive::show_interactive_3d(session)
            }

            /// Show the plot in a native interactive window.
            #[cfg(all(not(feature = "interactive-gpu"), not(target_arch = "wasm32")))]
            pub fn show(self) -> Result<()> {
                self.finalize().prepare_once()?;
                Err(interaction_not_available())
            }
        }

        impl crate::core::BuilderWhen for $builder {}
    };
}

/// Builder returned by [`crate::scatter3d`].
#[derive(Clone, Debug)]
pub struct Scatter3DBuilder {
    plot: Plot3D,
    data: Option<Points3DData>,
    config: Scatter3DConfig,
    label: Option<String>,
}

impl Scatter3DBuilder {
    pub(crate) fn from_data<X, Y, Z>(x: &X, y: &Y, z: &Z) -> Self
    where
        X: NumericData1D + ?Sized,
        Y: NumericData1D + ?Sized,
        Z: NumericData1D + ?Sized,
    {
        Self::with_plot(
            Plot3D::default(),
            Points3DData::collect("scatter3d", 1, x, y, z),
        )
    }

    fn with_plot(mut plot: Plot3D, data: Result<Points3DData>) -> Self {
        let data = match data {
            Ok(data) => Some(data),
            Err(error) => {
                plot.set_pending_error(error);
                None
            }
        };
        Self {
            plot,
            data,
            config: Scatter3DConfig::default(),
            label: None,
        }
    }

    pub(crate) fn finalize(mut self) -> Plot3D {
        if let Some(data) = self.data.take() {
            self.plot.series.push(Series3D::Scatter {
                data,
                config: self.config,
                label: self.label,
            });
        }
        self.plot
    }

    /// Set the legend label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set marker color.
    pub fn color(mut self, color: Color) -> Self {
        self.config.color = Some(color);
        self
    }

    /// Set marker shape.
    pub fn marker(mut self, marker: MarkerStyle) -> Self {
        self.config.marker = marker;
        self
    }

    /// Set marker diameter in typographic points.
    pub fn marker_size(mut self, size: f32) -> Self {
        self.config.marker_size = size;
        self
    }
}

impl_common_builder!(Scatter3DBuilder);

/// Builder returned by [`crate::line3d`].
#[derive(Clone, Debug)]
pub struct Line3DBuilder {
    plot: Plot3D,
    data: Option<Points3DData>,
    config: Line3DConfig,
    label: Option<String>,
}

impl Line3DBuilder {
    pub(crate) fn from_data<X, Y, Z>(x: &X, y: &Y, z: &Z) -> Self
    where
        X: NumericData1D + ?Sized,
        Y: NumericData1D + ?Sized,
        Z: NumericData1D + ?Sized,
    {
        Self::with_plot(
            Plot3D::default(),
            Points3DData::collect("line3d", 2, x, y, z),
        )
    }

    fn with_plot(mut plot: Plot3D, data: Result<Points3DData>) -> Self {
        let data = match data {
            Ok(data) => Some(data),
            Err(error) => {
                plot.set_pending_error(error);
                None
            }
        };
        Self {
            plot,
            data,
            config: Line3DConfig::default(),
            label: None,
        }
    }

    pub(super) fn finalize(mut self) -> Plot3D {
        if let Some(data) = self.data.take() {
            self.plot.series.push(Series3D::Line {
                data,
                config: self.config,
                label: self.label,
            });
        }
        self.plot
    }

    /// Set the legend label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set line color.
    pub fn color(mut self, color: Color) -> Self {
        self.config.color = Some(color);
        self
    }

    /// Set line width in typographic points.
    pub fn line_width(mut self, width: f32) -> Self {
        self.config.line_width = width;
        self
    }

    /// Set line style.
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.config.line_style = style;
        self
    }
}

impl_common_builder!(Line3DBuilder);

/// Builder returned by [`crate::surface`].
#[derive(Clone, Debug)]
pub struct Surface3DBuilder {
    plot: Plot3D,
    data: Option<Grid3DData>,
    config: Surface3DConfig,
    label: Option<String>,
}

impl Surface3DBuilder {
    pub(crate) fn from_data<X, Y, Z>(x: &X, y: &Y, z: &Z) -> Self
    where
        X: NumericData1D + ?Sized,
        Y: NumericData1D + ?Sized,
        Z: NumericData2D + ?Sized,
    {
        Self::with_plot(Plot3D::default(), Grid3DData::collect("surface", x, y, z))
    }

    fn with_plot(mut plot: Plot3D, data: Result<Grid3DData>) -> Self {
        let data = match data {
            Ok(data) => Some(data),
            Err(error) => {
                plot.set_pending_error(error);
                None
            }
        };
        Self {
            plot,
            data,
            config: Surface3DConfig::default(),
            label: None,
        }
    }

    pub(super) fn finalize(mut self) -> Plot3D {
        if let Some(data) = self.data.take() {
            self.plot.series.push(Series3D::Surface {
                data,
                config: self.config,
                label: self.label,
            });
        }
        self.plot
    }

    /// Set the legend label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Use one fixed surface color instead of z-based colormapping.
    pub fn color(mut self, color: Color) -> Self {
        self.config.color = Some(color);
        self
    }

    /// Set the z-value colour scale.
    ///
    /// Accepts either a name or a [`ColorMap`] value — this is the one
    /// spelling used by every colour-scaled plot type:
    ///
    /// ```rust,ignore
    /// surface.cmap("plasma");
    /// surface.cmap(ColorMap::plasma());
    /// ```
    ///
    /// An unrecognised name falls back to `viridis`.
    pub fn cmap(mut self, cmap: impl Into<ColorMapSpec>) -> Self {
        self.config.colormap = cmap.into().resolve();
        self
    }

    /// Set the z-value colormap.
    ///
    /// # Deprecated
    ///
    /// Renamed to [`Self::cmap`], which also accepts a colormap name.
    #[deprecated(
        since = "0.6.0",
        note = "renamed: use `cmap(colormap)`, which also accepts a name such as `cmap(\"viridis\")`"
    )]
    pub fn colormap(self, colormap: ColorMap) -> Self {
        self.cmap(colormap)
    }

    /// Set the surface shading model.
    pub fn shading(mut self, shading: SurfaceShading) -> Self {
        self.config.shading = shading;
        self
    }

    /// Set the regular-grid sampling policy.
    pub fn sampling(mut self, sampling: SurfaceSampling) -> Self {
        self.config.sampling = sampling;
        self
    }

    /// Show or hide the z-value colorbar.
    pub fn colorbar(mut self, visible: bool) -> Self {
        self.config.colorbar = visible;
        self
    }
}

impl_common_builder!(Surface3DBuilder);

/// Builder returned by [`crate::wireframe`].
#[derive(Clone, Debug)]
pub struct Wireframe3DBuilder {
    plot: Plot3D,
    data: Option<Grid3DData>,
    config: Wireframe3DConfig,
    label: Option<String>,
}

impl Wireframe3DBuilder {
    pub(crate) fn from_data<X, Y, Z>(x: &X, y: &Y, z: &Z) -> Self
    where
        X: NumericData1D + ?Sized,
        Y: NumericData1D + ?Sized,
        Z: NumericData2D + ?Sized,
    {
        Self::with_plot(Plot3D::default(), Grid3DData::collect("wireframe", x, y, z))
    }

    fn with_plot(mut plot: Plot3D, data: Result<Grid3DData>) -> Self {
        let data = match data {
            Ok(data) => Some(data),
            Err(error) => {
                plot.set_pending_error(error);
                None
            }
        };
        Self {
            plot,
            data,
            config: Wireframe3DConfig::default(),
            label: None,
        }
    }

    pub(super) fn finalize(mut self) -> Plot3D {
        if let Some(data) = self.data.take() {
            self.plot.series.push(Series3D::Wireframe {
                data,
                config: self.config,
                label: self.label,
            });
        }
        self.plot
    }

    /// Set the legend label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set wire color.
    pub fn color(mut self, color: Color) -> Self {
        self.config.color = Some(color);
        self
    }

    /// Set wire width in typographic points.
    pub fn line_width(mut self, width: f32) -> Self {
        self.config.line_width = width;
        self
    }

    /// Set wire style.
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.config.line_style = style;
        self
    }

    /// Set the regular-grid sampling policy.
    pub fn sampling(mut self, sampling: SurfaceSampling) -> Self {
        self.config.sampling = sampling;
        self
    }
}

impl_common_builder!(Wireframe3DBuilder);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builders_validate_and_combine_series_bounds() {
        let bounds = Scatter3DBuilder::from_data(&[0.0, 1.0], &[1.0, 2.0], &[2.0, 3.0])
            .line3d(&[-4.0, -2.0], &[10.0, 11.0], &[20.0, 22.0])
            .data_bounds()
            .expect("combined bounds");
        assert_eq!(bounds.min, Point3D::new(-4.0, 1.0, 2.0));
        assert_eq!(bounds.max, Point3D::new(1.0, 11.0, 22.0));
    }

    #[test]
    fn ingestion_errors_are_reported_at_the_terminal() {
        let error = Scatter3DBuilder::from_data(&[0.0, 1.0], &[0.0], &[0.0, 1.0])
            .validate()
            .expect_err("length mismatch");
        assert!(matches!(
            error,
            PlottingError::DataLengthMismatch3D {
                x_len: 2,
                y_len: 1,
                z_len: 2,
                ..
            }
        ));
    }

    #[test]
    fn invalid_sampling_is_rejected() {
        let error =
            Surface3DBuilder::from_data(&[0.0, 1.0], &[0.0, 1.0], &[[0.0, 1.0], [1.0, 2.0]])
                .sampling(SurfaceSampling::MaxGrid {
                    rows: 1,
                    columns: 2,
                })
                .validate()
                .expect_err("invalid sampling");
        assert!(matches!(error, PlottingError::InvalidTopology3D { .. }));
    }

    #[test]
    fn transparent_styles_are_rejected_for_the_opaque_mvp() {
        let error = Scatter3DBuilder::from_data(&[0.0], &[0.0], &[0.0])
            .color(Color::from_rgba(255, 0, 0, 128))
            .validate()
            .expect_err("transparent scatter");
        assert!(error.to_string().contains("transparency is unsupported"));
    }
}
