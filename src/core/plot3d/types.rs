use glam::{Mat4, Quat, Vec3};

use crate::core::{PlottingError, Result};

/// A point in three-dimensional data coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3D {
    /// Create a point from x, y, and z coordinates.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    fn has_infinite_component(self) -> bool {
        self.x.is_infinite() || self.y.is_infinite() || self.z.is_infinite()
    }
}

/// Finite data-space bounds for a 3D scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds3D {
    pub min: Point3D,
    pub max: Point3D,
}

impl Bounds3D {
    /// Construct bounds after checking ordering and finiteness.
    pub fn new(min: Point3D, max: Point3D) -> Result<Self> {
        for (name, low, high) in [
            ("x", min.x, max.x),
            ("y", min.y, max.y),
            ("z", min.z, max.z),
        ] {
            if !low.is_finite() || !high.is_finite() {
                return Err(PlottingError::InvalidTopology3D {
                    reason: format!("{name} bounds must be finite"),
                });
            }
            if low > high {
                return Err(PlottingError::InvalidTopology3D {
                    reason: format!("{name} minimum ({low}) exceeds maximum ({high})"),
                });
            }
        }
        Ok(Self { min, max })
    }

    /// Compute bounds from finite points.
    ///
    /// Points containing NaN are treated as gaps. Infinite coordinates are
    /// rejected rather than silently omitted.
    pub fn from_points(points: impl IntoIterator<Item = Point3D>) -> Result<Self> {
        let mut bounds: Option<Self> = None;
        for (index, point) in points.into_iter().enumerate() {
            if point.has_infinite_component() {
                return Err(PlottingError::InvalidData {
                    message: "3D bounds contain an infinite coordinate".to_string(),
                    position: Some(index),
                });
            }
            if !point.is_finite() {
                continue;
            }
            match &mut bounds {
                Some(bounds) => {
                    bounds.min.x = bounds.min.x.min(point.x);
                    bounds.min.y = bounds.min.y.min(point.y);
                    bounds.min.z = bounds.min.z.min(point.z);
                    bounds.max.x = bounds.max.x.max(point.x);
                    bounds.max.y = bounds.max.y.max(point.y);
                    bounds.max.z = bounds.max.z.max(point.z);
                }
                None => {
                    bounds = Some(Self {
                        min: point,
                        max: point,
                    });
                }
            }
        }
        bounds.ok_or(PlottingError::EmptyDataSet)
    }

    /// Center of the data-space bounds.
    pub fn center(self) -> Point3D {
        Point3D::new(
            midpoint(self.min.x, self.max.x),
            midpoint(self.min.y, self.max.y),
            midpoint(self.min.z, self.max.z),
        )
    }

    /// Width, height, and depth of the data-space bounds.
    pub fn extent(self) -> Point3D {
        Point3D::new(
            finite_span(self.min.x, self.max.x),
            finite_span(self.min.y, self.max.y),
            finite_span(self.min.z, self.max.z),
        )
    }

    pub(crate) fn include(&mut self, other: Self) {
        self.min.x = self.min.x.min(other.min.x);
        self.min.y = self.min.y.min(other.min.y);
        self.min.z = self.min.z.min(other.min.z);
        self.max.x = self.max.x.max(other.max.x);
        self.max.y = self.max.y.max(other.max.y);
        self.max.z = self.max.z.max(other.max.z);
    }

    fn normalize(self, point: Point3D, aspect: Vec3) -> Vec3 {
        Vec3::new(
            normalize_axis(point.x, self.min.x, self.max.x, aspect.x),
            normalize_axis(point.y, self.min.y, self.max.y, aspect.y),
            normalize_axis(point.z, self.min.z, self.max.z, aspect.z),
        )
    }
}

fn midpoint(low: f64, high: f64) -> f64 {
    low * 0.5 + high * 0.5
}

fn finite_span(low: f64, high: f64) -> f64 {
    let span = high - low;
    if span.is_finite() { span } else { f64::MAX }
}

fn normalize_axis(value: f64, low: f64, high: f64, aspect: f32) -> f32 {
    if low == high {
        return 0.0;
    }
    // Halving before subtraction avoids overflow for bounds close to
    // `[-f64::MAX, f64::MAX]`, while all offset removal still happens in f64.
    let center = midpoint(low, high);
    let half_span = high * 0.5 - low * 0.5;
    ((value - center) / half_span) as f32 * aspect
}

/// Projection used by a 3D camera.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Projection3D {
    /// Size-preserving scientific projection.
    #[default]
    Orthographic,
    /// Perspective projection with a vertical field of view in degrees.
    Perspective { vertical_fov_deg: f32 },
}

/// Physical proportions of the x/y/z plotting box.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AxisAspect3D {
    /// Scientific 4:4:3-style plotting box.
    #[default]
    Auto,
    /// Equal visual length for every axis.
    Equal,
    /// Explicit positive x/y/z proportions.
    Fixed { x: f32, y: f32, z: f32 },
}

impl AxisAspect3D {
    /// Create explicit positive x/y/z box proportions.
    pub const fn fixed(x: f32, y: f32, z: f32) -> Self {
        Self::Fixed { x, y, z }
    }

    fn validate(self) -> Result<()> {
        if let Self::Fixed { x, y, z } = self {
            for (field, value) in [("aspect.x", x), ("aspect.y", y), ("aspect.z", z)] {
                validate_positive_finite(field, value)?;
            }
        }
        Ok(())
    }

    fn resolved(self) -> Vec3 {
        let raw = match self {
            Self::Auto => Vec3::new(4.0, 4.0, 3.0),
            Self::Equal => Vec3::ONE,
            Self::Fixed { x, y, z } => Vec3::new(x, y, z),
        };
        raw / raw.max_element()
    }
}

/// Orbit camera for a 3D plot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera3D {
    azimuth_deg: f32,
    elevation_deg: f32,
    roll_deg: f32,
    projection: Projection3D,
    aspect: AxisAspect3D,
    zoom: f32,
}

impl Camera3D {
    /// Set the orbit azimuth in degrees.
    pub fn azimuth_deg(mut self, degrees: f32) -> Self {
        self.azimuth_deg = degrees;
        self
    }

    /// Set the orbit elevation in degrees.
    pub fn elevation_deg(mut self, degrees: f32) -> Self {
        self.elevation_deg = degrees;
        self
    }

    /// Set camera roll in degrees.
    pub fn roll_deg(mut self, degrees: f32) -> Self {
        self.roll_deg = degrees;
        self
    }

    /// Select perspective projection and its vertical field of view in degrees.
    pub fn perspective_deg(mut self, vertical_fov_deg: f32) -> Self {
        self.projection = Projection3D::Perspective { vertical_fov_deg };
        self
    }

    /// Select orthographic projection.
    pub fn orthographic(mut self) -> Self {
        self.projection = Projection3D::Orthographic;
        self
    }

    /// Set the physical proportions of the plotting box.
    pub fn axis_aspect(mut self, aspect: AxisAspect3D) -> Self {
        self.aspect = aspect;
        self
    }

    /// Set a positive zoom factor.
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    /// Current azimuth in degrees.
    pub const fn get_azimuth_deg(self) -> f32 {
        self.azimuth_deg
    }

    /// Current elevation in degrees.
    pub const fn get_elevation_deg(self) -> f32 {
        self.elevation_deg
    }

    /// Current roll in degrees.
    pub const fn get_roll_deg(self) -> f32 {
        self.roll_deg
    }

    /// Current projection.
    pub const fn projection(self) -> Projection3D {
        self.projection
    }

    /// Current plotting-box proportions.
    pub const fn axis_aspect_value(self) -> AxisAspect3D {
        self.aspect
    }

    /// Current zoom factor.
    pub const fn get_zoom(self) -> f32 {
        self.zoom
    }

    /// Validate all camera parameters.
    pub fn validate(self) -> Result<()> {
        validate_finite("azimuth_deg", self.azimuth_deg)?;
        validate_finite("roll_deg", self.roll_deg)?;
        validate_finite("elevation_deg", self.elevation_deg)?;
        if !(-89.9..=89.9).contains(&self.elevation_deg) {
            return Err(PlottingError::InvalidCamera3D {
                field: "elevation_deg",
                value: self.elevation_deg,
                reason: "must be between -89.9 and 89.9 degrees",
            });
        }
        validate_positive_finite("zoom", self.zoom)?;
        if let Projection3D::Perspective { vertical_fov_deg } = self.projection {
            validate_finite("vertical_fov_deg", vertical_fov_deg)?;
            if !(1.0..179.0).contains(&vertical_fov_deg) {
                return Err(PlottingError::InvalidCamera3D {
                    field: "vertical_fov_deg",
                    value: vertical_fov_deg,
                    reason: "must be greater than 1 and less than 179 degrees",
                });
            }
        }
        self.aspect.validate()
    }

    /// Project a data-space point into top-left-origin viewport pixels.
    pub fn project(
        self,
        point: Point3D,
        bounds: Bounds3D,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<ProjectedPoint3D> {
        if viewport_width == 0 || viewport_height == 0 {
            return Err(PlottingError::InvalidDimensions {
                width: viewport_width,
                height: viewport_height,
            });
        }
        self.validate()?;
        let bounds = Bounds3D::new(bounds.min, bounds.max)?;
        if point.has_infinite_component() {
            return Err(PlottingError::InvalidData {
                message: "projected 3D point contains an infinite coordinate".to_string(),
                position: None,
            });
        }
        if !point.is_finite() {
            return Ok(ProjectedPoint3D {
                x: f32::NAN,
                y: f32::NAN,
                depth: f32::NAN,
                visible: false,
            });
        }

        let prepared = self.prepare(viewport_width as f32 / viewport_height as f32)?;
        let local = bounds.normalize(point, prepared.axis_aspect);
        let clip = prepared.view_projection * local.extend(1.0);
        if !clip.is_finite() || clip.w <= 0.0 {
            return Ok(ProjectedPoint3D {
                x: f32::NAN,
                y: f32::NAN,
                depth: f32::NAN,
                visible: false,
            });
        }
        let ndc = clip.truncate() / clip.w;
        Ok(ProjectedPoint3D {
            x: (ndc.x * 0.5 + 0.5) * viewport_width as f32,
            y: (0.5 - ndc.y * 0.5) * viewport_height as f32,
            depth: ndc.z,
            visible: (-1.0..=1.0).contains(&ndc.x)
                && (-1.0..=1.0).contains(&ndc.y)
                && (0.0..=1.0).contains(&ndc.z),
        })
    }

    pub(crate) fn prepare(self, viewport_aspect: f32) -> Result<PreparedCamera3D> {
        self.validate()?;
        if !viewport_aspect.is_finite() || viewport_aspect <= 0.0 {
            return Err(PlottingError::InvalidCamera3D {
                field: "viewport_aspect",
                value: viewport_aspect,
                reason: "must be finite and greater than zero",
            });
        }

        let azimuth = self.azimuth_deg.to_radians();
        let elevation = self.elevation_deg.to_radians();
        let eye_direction = Vec3::new(
            elevation.cos() * azimuth.cos(),
            elevation.cos() * azimuth.sin(),
            elevation.sin(),
        );
        let eye = eye_direction * 4.0;
        let forward = (-eye).normalize();
        let up = Quat::from_axis_angle(forward, self.roll_deg.to_radians()) * Vec3::Z;
        let view = Mat4::look_at_rh(eye, Vec3::ZERO, up);
        let projection = match self.projection {
            Projection3D::Orthographic => {
                let (half_width, half_height) = if viewport_aspect >= 1.0 {
                    let half_height = 1.8 / self.zoom;
                    (half_height * viewport_aspect, half_height)
                } else {
                    let half_width = 1.8 / self.zoom;
                    (half_width, half_width / viewport_aspect)
                };
                Mat4::orthographic_rh(
                    -half_width,
                    half_width,
                    -half_height,
                    half_height,
                    0.01,
                    100.0,
                )
            }
            Projection3D::Perspective { vertical_fov_deg } => {
                Mat4::perspective_rh(vertical_fov_deg.to_radians(), viewport_aspect, 0.01, 100.0)
            }
        };
        let view_projection = projection * view;
        Ok(PreparedCamera3D {
            view_projection,
            inverse_view_projection: view_projection.inverse(),
            axis_aspect: self.aspect.resolved(),
        })
    }
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            azimuth_deg: -60.0,
            elevation_deg: 30.0,
            roll_deg: 0.0,
            projection: Projection3D::Orthographic,
            aspect: AxisAspect3D::Auto,
            zoom: 1.0,
        }
    }
}

fn validate_finite(field: &'static str, value: f32) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(PlottingError::InvalidCamera3D {
            field,
            value,
            reason: "must be finite",
        })
    }
}

fn validate_positive_finite(field: &'static str, value: f32) -> Result<()> {
    validate_finite(field, value)?;
    if value > 0.0 {
        Ok(())
    } else {
        Err(PlottingError::InvalidCamera3D {
            field,
            value,
            reason: "must be greater than zero",
        })
    }
}

/// Result of projecting a 3D point into a viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedPoint3D {
    /// Horizontal pixel coordinate from the viewport's left edge.
    pub x: f32,
    /// Vertical pixel coordinate from the viewport's top edge.
    pub y: f32,
    /// Wgpu-compatible normalized depth in the range 0..1 when visible.
    pub depth: f32,
    /// Whether the point lies inside all six clip planes.
    pub visible: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PreparedCamera3D {
    pub(crate) view_projection: Mat4,
    pub(crate) inverse_view_projection: Mat4,
    pub(crate) axis_aspect: Vec3,
}

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn default_camera_matches_documented_contract() {
        let camera = Camera3D::default();
        assert_eq!(camera.get_azimuth_deg(), -60.0);
        assert_eq!(camera.get_elevation_deg(), 30.0);
        assert_eq!(camera.get_roll_deg(), 0.0);
        assert_eq!(camera.projection(), Projection3D::Orthographic);
        camera.validate().expect("default camera is valid");
    }

    #[test]
    fn bounds_ignore_nan_gaps_and_reject_infinity() {
        let bounds = Bounds3D::from_points([
            Point3D::new(f64::NAN, 0.0, 0.0),
            Point3D::new(2.0, 3.0, 4.0),
            Point3D::new(4.0, 7.0, 8.0),
        ])
        .expect("finite points define bounds");
        assert_eq!(bounds.min, Point3D::new(2.0, 3.0, 4.0));
        assert_eq!(bounds.max, Point3D::new(4.0, 7.0, 8.0));

        assert!(matches!(
            Bounds3D::from_points([Point3D::new(f64::INFINITY, 0.0, 0.0)]),
            Err(PlottingError::InvalidData { .. })
        ));
    }

    #[test]
    fn large_offsets_are_normalized_before_f32_conversion() {
        let base = 1.0e12;
        let bounds = Bounds3D::new(
            Point3D::new(base, base, base),
            Point3D::new(base + 4.0, base + 8.0, base + 2.0),
        )
        .expect("bounds");
        let aspect = AxisAspect3D::Auto.resolved();
        let low = bounds.normalize(bounds.min, aspect);
        let high = bounds.normalize(bounds.max, aspect);
        assert_abs_diff_eq!(low.x, -1.0, epsilon = 1.0e-6);
        assert_abs_diff_eq!(high.x, 1.0, epsilon = 1.0e-6);
        assert_abs_diff_eq!(low.z, -0.75, epsilon = 1.0e-6);
        assert_abs_diff_eq!(high.z, 0.75, epsilon = 1.0e-6);
    }

    #[test]
    fn extreme_finite_bounds_do_not_collapse_during_normalization() {
        let bounds = Bounds3D::new(
            Point3D::new(-f64::MAX, -1.0, -1.0),
            Point3D::new(f64::MAX, 1.0, 1.0),
        )
        .expect("finite bounds");
        let aspect = AxisAspect3D::Equal.resolved();
        let low = bounds.normalize(bounds.min, aspect);
        let high = bounds.normalize(bounds.max, aspect);
        assert_abs_diff_eq!(low.x, -1.0, epsilon = 1.0e-6);
        assert_abs_diff_eq!(high.x, 1.0, epsilon = 1.0e-6);
    }

    #[test]
    fn projected_center_is_viewport_center() {
        let bounds = Bounds3D::new(Point3D::new(0.0, 0.0, 0.0), Point3D::new(2.0, 4.0, 8.0))
            .expect("bounds");
        let projected = Camera3D::default()
            .project(bounds.center(), bounds, 800, 600)
            .expect("projection");
        assert_abs_diff_eq!(projected.x, 400.0, epsilon = 1.0e-3);
        assert_abs_diff_eq!(projected.y, 300.0, epsilon = 1.0e-3);
        assert!(projected.visible);
        assert!((0.0..=1.0).contains(&projected.depth));
    }

    #[test]
    fn invalid_camera_is_a_typed_error() {
        let error = Camera3D::default()
            .perspective_deg(180.0)
            .validate()
            .expect_err("invalid field of view");
        assert!(matches!(
            error,
            PlottingError::InvalidCamera3D {
                field: "vertical_fov_deg",
                ..
            }
        ));
    }
}
