use crate::core::{Bounds3D, FigureConfig, PlottingError, Point3D, Result};
use crate::plots::{SurfaceSampling, SurfaceShading};
use crate::render::{Color, ColorMap, LineStyle, Theme};

use super::Camera3D;
use super::builder::{Plot3D, Series3D, validate_figure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CacheKey3D(pub(crate) u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrameKeys3D {
    pub(crate) geometry: CacheKey3D,
    pub(crate) appearance: CacheKey3D,
    pub(crate) layout: CacheKey3D,
    pub(crate) view: CacheKey3D,
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedFrame3D {
    pub(crate) series: Vec<Series3D>,
    pub(crate) bounds: Bounds3D,
    pub(crate) camera: Camera3D,
    pub(crate) figure: FigureConfig,
    pub(crate) theme: Theme,
    pub(crate) title: Option<String>,
    pub(crate) xlabel: Option<String>,
    pub(crate) ylabel: Option<String>,
    pub(crate) zlabel: Option<String>,
    pub(crate) keys: FrameKeys3D,
}

impl Plot3D {
    pub(crate) fn resolve(mut self) -> Result<ResolvedFrame3D> {
        if let Some(error) = self.pending_error.take() {
            return Err(error);
        }
        self.camera.validate()?;
        validate_figure(&self.figure)?;
        if self.series.is_empty() {
            return Err(PlottingError::NoDataSeries);
        }

        let mut combined: Option<Bounds3D> = None;
        for series in &self.series {
            series.validate_style()?;
            let bounds = series.bounds()?;
            match &mut combined {
                Some(combined) => combined.include(bounds),
                None => combined = Some(bounds),
            }
        }
        let data_bounds = combined.ok_or(PlottingError::EmptyDataSet)?;
        let bounds = apply_limits(data_bounds, self.xlim, self.ylim, self.zlim)?;
        let keys = frame_keys(
            &self.series,
            bounds,
            self.camera,
            &self.figure,
            &self.theme,
            self.title.as_deref(),
            self.xlabel.as_deref(),
            self.ylabel.as_deref(),
            self.zlabel.as_deref(),
        );

        Ok(ResolvedFrame3D {
            series: self.series,
            bounds,
            camera: self.camera,
            figure: self.figure,
            theme: self.theme,
            title: self.title,
            xlabel: self.xlabel,
            ylabel: self.ylabel,
            zlabel: self.zlabel,
            keys,
        })
    }
}

fn frame_keys(
    series: &[Series3D],
    bounds: Bounds3D,
    camera: Camera3D,
    figure: &FigureConfig,
    theme: &Theme,
    title: Option<&str>,
    xlabel: Option<&str>,
    ylabel: Option<&str>,
    zlabel: Option<&str>,
) -> FrameKeys3D {
    let mut geometry = StableHasher3D::new();
    let mut appearance = StableHasher3D::new();
    let mut layout = StableHasher3D::new();
    for (index, series) in series.iter().enumerate() {
        geometry.usize(index);
        appearance.usize(index);
        match series {
            Series3D::Scatter {
                data,
                config,
                label,
            } => {
                geometry.byte(0);
                geometry.f64_slice(&data.x);
                geometry.f64_slice(&data.y);
                geometry.f64_slice(&data.z);

                appearance.byte(0);
                appearance.color_option(config.color);
                appearance.byte(config.marker as u8);
                appearance.f32(config.marker_size);
                appearance.optional_str(label.as_deref());
                layout.optional_str(label.as_deref());
            }
            Series3D::Line {
                data,
                config,
                label,
            } => {
                geometry.byte(1);
                geometry.f64_slice(&data.x);
                geometry.f64_slice(&data.y);
                geometry.f64_slice(&data.z);

                appearance.byte(1);
                appearance.color_option(config.color);
                appearance.f32(config.line_width);
                appearance.line_style(&config.line_style);
                appearance.optional_str(label.as_deref());
                layout.optional_str(label.as_deref());
            }
            Series3D::Surface {
                data,
                config,
                label,
            } => {
                geometry.byte(2);
                geometry.usize(data.rows);
                geometry.usize(data.columns);
                geometry.f64_slice(&data.x);
                geometry.f64_slice(&data.y);
                geometry.f64_slice(&data.z);
                geometry.sampling(config.sampling);
                geometry.shading(config.shading);

                appearance.byte(2);
                appearance.color_option(config.color);
                appearance.colormap(&config.colormap);
                appearance.bool(config.colorbar);
                appearance.optional_str(label.as_deref());
                layout.bool(config.colorbar);
                layout.optional_str(label.as_deref());
            }
            Series3D::Wireframe {
                data,
                config,
                label,
            } => {
                geometry.byte(3);
                geometry.usize(data.rows);
                geometry.usize(data.columns);
                geometry.f64_slice(&data.x);
                geometry.f64_slice(&data.y);
                geometry.f64_slice(&data.z);
                geometry.sampling(config.sampling);

                appearance.byte(3);
                appearance.color_option(config.color);
                appearance.f32(config.line_width);
                appearance.line_style(&config.line_style);
                appearance.optional_str(label.as_deref());
                layout.optional_str(label.as_deref());
            }
        }
    }
    hash_bounds(&mut geometry, bounds);
    hash_bounds(&mut layout, bounds);
    hash_theme_appearance(&mut appearance, theme);
    hash_layout(&mut layout, figure, theme, title, xlabel, ylabel, zlabel);

    let mut view = StableHasher3D::new();
    hash_camera(&mut view, camera);
    let (width, height) = figure.canvas_size();
    view.u32(width);
    view.u32(height);

    FrameKeys3D {
        geometry: CacheKey3D(geometry.finish()),
        appearance: CacheKey3D(appearance.finish()),
        layout: CacheKey3D(layout.finish()),
        view: CacheKey3D(view.finish()),
    }
}

fn apply_limits(
    data_bounds: Bounds3D,
    xlim: Option<(f64, f64)>,
    ylim: Option<(f64, f64)>,
    zlim: Option<(f64, f64)>,
) -> Result<Bounds3D> {
    for (axis, limits) in [("x", xlim), ("y", ylim), ("z", zlim)] {
        if let Some((minimum, maximum)) = limits
            && (!minimum.is_finite() || !maximum.is_finite() || minimum > maximum)
        {
            return Err(PlottingError::InvalidTopology3D {
                reason: format!(
                    "{axis} limits must be finite and ascending, got ({minimum}, {maximum})"
                ),
            });
        }
    }
    Bounds3D::new(
        Point3D::new(
            xlim.map_or(data_bounds.min.x, |limits| limits.0),
            ylim.map_or(data_bounds.min.y, |limits| limits.0),
            zlim.map_or(data_bounds.min.z, |limits| limits.0),
        ),
        Point3D::new(
            xlim.map_or(data_bounds.max.x, |limits| limits.1),
            ylim.map_or(data_bounds.max.y, |limits| limits.1),
            zlim.map_or(data_bounds.max.z, |limits| limits.1),
        ),
    )
}

fn hash_bounds(hasher: &mut StableHasher3D, bounds: Bounds3D) {
    for value in [
        bounds.min.x,
        bounds.min.y,
        bounds.min.z,
        bounds.max.x,
        bounds.max.y,
        bounds.max.z,
    ] {
        hasher.f64(value);
    }
}

fn hash_camera(hasher: &mut StableHasher3D, camera: Camera3D) {
    hasher.f32(camera.get_azimuth_deg());
    hasher.f32(camera.get_elevation_deg());
    hasher.f32(camera.get_roll_deg());
    hasher.f32(camera.get_zoom());
    match camera.target() {
        Some(target) => {
            hasher.byte(1);
            hasher.f64(target.x);
            hasher.f64(target.y);
            hasher.f64(target.z);
        }
        None => hasher.byte(0),
    }
    match camera.projection() {
        super::Projection3D::Orthographic => hasher.byte(0),
        super::Projection3D::Perspective { vertical_fov_deg } => {
            hasher.byte(1);
            hasher.f32(vertical_fov_deg);
        }
    }
    match camera.axis_aspect_value() {
        super::AxisAspect3D::Auto => hasher.byte(0),
        super::AxisAspect3D::Equal => hasher.byte(1),
        super::AxisAspect3D::Fixed { x, y, z } => {
            hasher.byte(2);
            hasher.f32(x);
            hasher.f32(y);
            hasher.f32(z);
        }
    }
}

fn hash_theme_appearance(hasher: &mut StableHasher3D, theme: &Theme) {
    hasher.color(theme.foreground);
    hasher.color(theme.background);
    hasher.color(theme.grid_color);
    hasher.f32(theme.line_width);
    hasher.line_style(&theme.line_style);
    hasher.usize(theme.color_palette.len());
    for &color in &theme.color_palette {
        hasher.color(color);
    }
}

fn hash_layout(
    hasher: &mut StableHasher3D,
    figure: &FigureConfig,
    theme: &Theme,
    title: Option<&str>,
    xlabel: Option<&str>,
    ylabel: Option<&str>,
    zlabel: Option<&str>,
) {
    hasher.f32(figure.width);
    hasher.f32(figure.height);
    hasher.f32(figure.dpi);
    hasher.optional_str(title);
    hasher.optional_str(xlabel);
    hasher.optional_str(ylabel);
    hasher.optional_str(zlabel);
    hasher.str(&theme.font_family);
    hasher.f32(theme.font_size);
    hasher.f32(theme.title_font_size);
    hasher.f32(theme.legend_font_size);
    hasher.f32(theme.axis_label_font_size);
    hasher.f32(theme.tick_label_font_size);
    hasher.f32(theme.margin);
    hasher.f32(theme.padding);
}

struct StableHasher3D(u64);

impl StableHasher3D {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    fn new() -> Self {
        Self(Self::OFFSET)
    }

    fn finish(self) -> u64 {
        self.0
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn byte(&mut self, value: u8) {
        self.bytes(&[value]);
    }

    fn bool(&mut self, value: bool) {
        self.byte(u8::from(value));
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    fn usize(&mut self, value: usize) {
        self.bytes(&(value as u64).to_le_bytes());
    }

    fn f32(&mut self, value: f32) {
        self.u32(value.to_bits());
    }

    fn f64(&mut self, value: f64) {
        self.bytes(&value.to_bits().to_le_bytes());
    }

    fn f64_slice(&mut self, values: &[f64]) {
        self.usize(values.len());
        for &value in values {
            self.f64(value);
        }
    }

    fn str(&mut self, value: &str) {
        self.usize(value.len());
        self.bytes(value.as_bytes());
    }

    fn optional_str(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.byte(1);
                self.str(value);
            }
            None => self.byte(0),
        }
    }

    fn color(&mut self, color: Color) {
        self.bytes(&[color.r, color.g, color.b, color.a]);
    }

    fn color_option(&mut self, color: Option<Color>) {
        match color {
            Some(color) => {
                self.byte(1);
                self.color(color);
            }
            None => self.byte(0),
        }
    }

    fn colormap(&mut self, colormap: &ColorMap) {
        self.str(colormap.name());
        self.usize(colormap.colors().len());
        for &color in colormap.colors() {
            self.color(color);
        }
    }

    fn line_style(&mut self, style: &LineStyle) {
        match style {
            LineStyle::Solid => self.byte(0),
            LineStyle::Dashed => self.byte(1),
            LineStyle::Dotted => self.byte(2),
            LineStyle::DashDot => self.byte(3),
            LineStyle::DashDotDot => self.byte(4),
            LineStyle::Custom(pattern) => {
                self.byte(5);
                self.usize(pattern.len());
                for &value in pattern {
                    self.f32(value);
                }
            }
        }
    }

    fn sampling(&mut self, sampling: SurfaceSampling) {
        match sampling {
            SurfaceSampling::Auto => self.byte(0),
            SurfaceSampling::Full => self.byte(1),
            SurfaceSampling::MaxGrid { rows, columns } => {
                self.byte(2);
                self.usize(rows);
                self.usize(columns);
            }
        }
    }

    fn shading(&mut self, shading: SurfaceShading) {
        self.byte(match shading {
            SurfaceShading::Unlit => 0,
            SurfaceShading::Flat => 1,
            SurfaceShading::Smooth => 2,
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::{scatter3d, surface};

    use super::*;

    #[test]
    fn camera_changes_only_the_view_key() {
        let first = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .finalize()
            .resolve()
            .expect("first frame");
        let second = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .azimuth_deg(15.0)
            .finalize()
            .resolve()
            .expect("second frame");
        assert_eq!(first.keys.geometry, second.keys.geometry);
        assert_eq!(first.keys.appearance, second.keys.appearance);
        assert_eq!(first.keys.layout, second.keys.layout);
        assert_ne!(first.keys.view, second.keys.view);
    }

    #[test]
    fn style_and_data_have_separate_keys() {
        let base = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .finalize()
            .resolve()
            .expect("base");
        let styled = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .marker_size(12.0)
            .finalize()
            .resolve()
            .expect("styled");
        let changed = scatter3d(&[0.0, 2.0], &[0.0, 1.0], &[0.0, 1.0])
            .finalize()
            .resolve()
            .expect("changed");
        assert_eq!(base.keys.geometry, styled.keys.geometry);
        assert_ne!(base.keys.appearance, styled.keys.appearance);
        assert_ne!(base.keys.geometry, changed.keys.geometry);
    }

    #[test]
    fn shading_and_sampling_are_geometry_keys() {
        let z = [[0.0, 1.0], [1.0, 2.0]];
        let smooth = surface(&[0.0, 1.0], &[0.0, 1.0], &z)
            .finalize()
            .resolve()
            .expect("smooth");
        let flat = surface(&[0.0, 1.0], &[0.0, 1.0], &z)
            .shading(SurfaceShading::Flat)
            .finalize()
            .resolve()
            .expect("flat");
        assert_ne!(smooth.keys.geometry, flat.keys.geometry);
    }

    #[test]
    fn explicit_limits_override_bounds_and_invalidate_geometry_and_layout() {
        let base = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .finalize()
            .resolve()
            .expect("base");
        let limited = scatter3d(&[0.0, 1.0], &[0.0, 1.0], &[0.0, 1.0])
            .xlim(-2.0, 2.0)
            .ylim(-3.0, 3.0)
            .zlim(-4.0, 4.0)
            .finalize()
            .resolve()
            .expect("limited");
        assert_eq!(limited.bounds.min, Point3D::new(-2.0, -3.0, -4.0));
        assert_eq!(limited.bounds.max, Point3D::new(2.0, 3.0, 4.0));
        assert_ne!(base.keys.geometry, limited.keys.geometry);
        assert_ne!(base.keys.layout, limited.keys.layout);
        assert_eq!(base.keys.appearance, limited.keys.appearance);
        assert_eq!(base.keys.view, limited.keys.view);
    }

    #[test]
    fn invalid_explicit_limits_are_rejected_at_the_terminal() {
        let error = scatter3d(&[0.0], &[0.0], &[0.0])
            .xlim(2.0, -2.0)
            .validate()
            .expect_err("descending limits");
        assert!(matches!(error, PlottingError::InvalidTopology3D { .. }));
        assert!(error.to_string().contains("x limits"));
    }
}
