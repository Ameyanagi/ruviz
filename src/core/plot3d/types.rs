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

    pub(crate) fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    pub(crate) fn has_infinite_component(self) -> bool {
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

    pub(crate) fn normalize(self, point: Point3D, aspect: Vec3) -> Vec3 {
        Vec3::new(
            normalize_axis(point.x, self.min.x, self.max.x, aspect.x),
            normalize_axis(point.y, self.min.y, self.max.y, aspect.y),
            normalize_axis(point.z, self.min.z, self.max.z, aspect.z),
        )
    }

    pub(crate) fn denormalize(self, point: Vec3, aspect: Vec3) -> Point3D {
        Point3D::new(
            denormalize_axis(point.x, self.min.x, self.max.x, aspect.x),
            denormalize_axis(point.y, self.min.y, self.max.y, aspect.y),
            denormalize_axis(point.z, self.min.z, self.max.z, aspect.z),
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

fn denormalize_axis(value: f32, low: f64, high: f64, aspect: f32) -> f64 {
    let center = midpoint(low, high);
    let half_span = if low == high {
        // Degenerate data ranges still need a meaningful inverse mapping for
        // screen rays and picking around a single point.
        1.0
    } else {
        high * 0.5 - low * 0.5
    };
    center + f64::from(value / aspect) * half_span
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
    /// Equal length per data unit, preserving spheres and molecular distances.
    Data,
    /// Explicit positive x/y/z proportions.
    Fixed { x: f32, y: f32, z: f32 },
}

/// A named, axis-aligned orientation for a 3D camera.
///
/// Applying a named view changes azimuth, elevation, and roll only. Projection,
/// axis aspect, zoom, and the current look-at target are preserved.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CameraView3D {
    /// The default three-quarter view (`-60°` azimuth, `30°` elevation).
    #[default]
    Isometric,
    /// Look from negative y toward the scene center.
    Front,
    /// Look from positive y toward the scene center.
    Back,
    /// Look from negative x toward the scene center.
    Left,
    /// Look from positive x toward the scene center.
    Right,
    /// Look down from positive z.
    ///
    /// The camera uses `89.9°` rather than the singular `90°` pole.
    Top,
    /// Look up from negative z.
    ///
    /// The camera uses `-89.9°` rather than the singular `-90°` pole.
    Bottom,
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
                if !(value / x.max(y).max(z)).recip().is_finite() {
                    return Err(PlottingError::InvalidCamera3D {
                        field,
                        value,
                        reason: "normalized ratio is too small to represent",
                    });
                }
            }
        }
        Ok(())
    }

    fn resolved(self) -> Vec3 {
        let raw = match self {
            Self::Auto => Vec3::new(4.0, 4.0, 3.0),
            Self::Equal | Self::Data => Vec3::ONE,
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
    stable_scale: bool,
    zoom: f32,
    target: Option<Point3D>,
}

impl Camera3D {
    /// Apply a named orientation while preserving projection, aspect, zoom,
    /// and the current look-at target.
    ///
    /// Top and bottom use the pole-safe elevations `89.9°` and `-89.9°`.
    pub fn camera_view(mut self, view: CameraView3D) -> Self {
        let (azimuth_deg, elevation_deg) = match view {
            CameraView3D::Isometric => (-60.0, 30.0),
            CameraView3D::Front => (-90.0, 0.0),
            CameraView3D::Back => (90.0, 0.0),
            CameraView3D::Left => (180.0, 0.0),
            CameraView3D::Right => (0.0, 0.0),
            CameraView3D::Top => (0.0, 89.9),
            CameraView3D::Bottom => (0.0, -89.9),
        };
        self.azimuth_deg = azimuth_deg;
        self.elevation_deg = elevation_deg;
        self.roll_deg = 0.0;
        self
    }

    /// Recenter on `bounds` and restore unit zoom.
    ///
    /// Camera orientation, projection, and axis aspect are preserved.
    pub fn fit_to_content(mut self, bounds: Bounds3D) -> Self {
        self.zoom = 1.0;
        self.target = Some(bounds.center());
        self
    }

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

    /// Keep projection scale and framing independent of the orbit angle.
    ///
    /// Reserves room for the whole scene at every orientation. Explicit zoom
    /// and viewport resizing still change its on-screen size. Defaults to false.
    pub fn stable_scale(mut self, enabled: bool) -> Self {
        self.stable_scale = enabled;
        self
    }

    /// Whether orbiting preserves projection scale and framing.
    pub const fn has_stable_scale(self) -> bool {
        self.stable_scale
    }

    /// Set a positive zoom factor.
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    /// Orbit around an explicit point in data coordinates.
    ///
    /// By default the camera looks at the center of the resolved plot bounds.
    /// A target outside the resolved bounds is clamped to the plotting box, so
    /// the plot always stays in front of the camera.
    pub fn look_at(mut self, target: Point3D) -> Self {
        self.target = Some(target);
        self
    }

    /// Return to looking at the center of the resolved plot bounds.
    pub fn look_at_bounds_center(mut self) -> Self {
        self.target = None;
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

    /// Explicit data-space look-at target, or `None` for the bounds center.
    pub const fn target(self) -> Option<Point3D> {
        self.target
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
        if self.target.is_some_and(|target| !target.is_finite()) {
            return Err(PlottingError::InvalidInput(
                "3D camera look-at target must contain finite data coordinates".to_string(),
            ));
        }
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

        let prepared = self.prepare(viewport_width as f32 / viewport_height as f32, bounds)?;
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

    /// Unproject a viewport pixel at an explicit wgpu-compatible depth.
    ///
    /// `depth` is in the closed range `0..=1`, where zero is the near clip
    /// plane and one is the far clip plane.
    pub fn unproject_at_depth(
        self,
        screen_x: f32,
        screen_y: f32,
        depth: f32,
        bounds: Bounds3D,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<Point3D> {
        validate_viewport(viewport_width, viewport_height)?;
        self.validate()?;
        let bounds = Bounds3D::new(bounds.min, bounds.max)?;
        validate_screen_coordinate("screen_x", screen_x)?;
        validate_screen_coordinate("screen_y", screen_y)?;
        if !depth.is_finite() || !(0.0..=1.0).contains(&depth) {
            return Err(PlottingError::InvalidCamera3D {
                field: "depth",
                value: depth,
                reason: "must be finite and between 0 and 1",
            });
        }

        let prepared = self.prepare(viewport_width as f32 / viewport_height as f32, bounds)?;
        let local = unproject_local(
            prepared.inverse_view_projection,
            screen_x,
            screen_y,
            depth,
            viewport_width,
            viewport_height,
        )?;
        Ok(bounds.denormalize(local, prepared.axis_aspect))
    }

    /// Construct a data-space ray through a viewport pixel.
    pub fn screen_ray(
        self,
        screen_x: f32,
        screen_y: f32,
        bounds: Bounds3D,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<ScreenRay3D> {
        let origin = self.unproject_at_depth(
            screen_x,
            screen_y,
            0.0,
            bounds,
            viewport_width,
            viewport_height,
        )?;
        let far = self.unproject_at_depth(
            screen_x,
            screen_y,
            1.0,
            bounds,
            viewport_width,
            viewport_height,
        )?;
        let mut direction = Point3D::new(far.x - origin.x, far.y - origin.y, far.z - origin.z);
        let length = direction.x.hypot(direction.y).hypot(direction.z);
        if !length.is_finite() || length == 0.0 {
            return Err(PlottingError::InvalidTopology3D {
                reason: "screen ray has a degenerate data-space direction".to_string(),
            });
        }
        direction.x /= length;
        direction.y /= length;
        direction.z /= length;
        Ok(ScreenRay3D { origin, direction })
    }

    pub(crate) fn prepare(
        self,
        viewport_aspect: f32,
        bounds: Bounds3D,
    ) -> Result<PreparedCamera3D> {
        self.validate()?;
        if !viewport_aspect.is_finite() || viewport_aspect <= 0.0 {
            return Err(PlottingError::InvalidCamera3D {
                field: "viewport_aspect",
                value: viewport_aspect,
                reason: "must be finite and greater than zero",
            });
        }

        let axis_aspect = if self.aspect == AxisAspect3D::Data {
            let extent = bounds.extent();
            let longest = extent.x.max(extent.y).max(extent.z).max(f64::MIN_POSITIVE);
            Vec3::new(
                (extent.x / longest) as f32,
                (extent.y / longest) as f32,
                (extent.z / longest) as f32,
            )
            .max(Vec3::splat(1e-6))
        } else {
            self.aspect.resolved()
        };
        // The look-at target is clamped to the plotting box. `look_at(1e6, 0, 0)`
        // is a valid call today, and without this it normalizes to a point far
        // outside the box, putting every primitive behind the eye where the
        // perspective divide is meaningless.
        let target = bounds
            .normalize(self.target.unwrap_or_else(|| bounds.center()), axis_aspect)
            .clamp(-axis_aspect, axis_aspect);
        // Bound the scene by the worst-case aspect-scaled box corner rather than
        // a constant that silently assumed `AxisAspect3D::Auto`.
        let scene_radius = scene_radius_about(target, axis_aspect);
        let azimuth = self.azimuth_deg.to_radians();
        let elevation = self.elevation_deg.to_radians();
        let eye_direction = Vec3::new(
            elevation.cos() * azimuth.cos(),
            elevation.cos() * azimuth.sin(),
            elevation.sin(),
        );
        // The eye distance is fixed for an orthographic camera — the projection
        // has no foreshortening, so distance only has to keep the box between
        // the clip planes. Perspective derives it from the fov, which is what
        // makes `vertical_fov_deg` mean "how wide-angle the view is": a wider
        // fov moves the eye closer and foreshortens the box more.
        let eye_distance = match self.projection {
            Projection3D::Orthographic => ORTHOGRAPHIC_EYE_DISTANCE,
            Projection3D::Perspective { vertical_fov_deg } => {
                let base_half_y = vertical_fov_deg.to_radians() * 0.5;
                let base_half_x = (base_half_y.tan() * viewport_aspect).atan();
                scene_radius / base_half_x.min(base_half_y).sin() * 1.05
            }
        };
        let eye = target + eye_direction * eye_distance;
        let forward = (target - eye).normalize();
        let up = Quat::from_axis_angle(forward, self.roll_deg.to_radians()) * Vec3::Z;
        let view = Mat4::look_at_rh(eye, target, up);
        // Both projections are fitted to the same thing — the eight projected
        // corners of the plotting box — so a perspective figure fills its frame
        // as well as an orthographic one instead of being sized for the box's
        // circumscribed sphere, which is nearly 30% larger than the box.
        let projection = match self.projection {
            Projection3D::Orthographic if self.stable_scale => {
                let half_y = scene_radius / viewport_aspect.min(1.0) / self.zoom;
                let half_x = half_y * viewport_aspect;
                Mat4::orthographic_rh(
                    -half_x,
                    half_x,
                    -half_y,
                    half_y,
                    ORTHOGRAPHIC_NEAR,
                    ORTHOGRAPHIC_FAR,
                )
            }
            Projection3D::Perspective { vertical_fov_deg } if self.stable_scale => {
                let fov = 2.0 * ((vertical_fov_deg.to_radians() * 0.5).tan() / self.zoom).atan();
                Mat4::perspective_rh(
                    fov,
                    viewport_aspect,
                    (eye_distance - scene_radius * 1.25).max(0.001),
                    eye_distance + scene_radius * 1.25,
                )
            }
            Projection3D::Orthographic => {
                orthographic_fit(view, axis_aspect, viewport_aspect, self.zoom)
            }
            Projection3D::Perspective { .. } => perspective_fit(
                view,
                axis_aspect,
                viewport_aspect,
                self.zoom,
                scene_radius,
                eye_distance,
            ),
        };
        let view_projection = projection * view;
        Ok(PreparedCamera3D {
            view,
            view_projection,
            inverse_view_projection: view_projection.inverse(),
            axis_aspect,
        })
    }
}

/// Eye distance for an orthographic camera.
///
/// An orthographic projection has no foreshortening, so this only has to keep
/// the whole box between the clip planes; [`orthographic_fit`] decides how much
/// of the frame the box occupies.
const ORTHOGRAPHIC_EYE_DISTANCE: f32 = 4.0;

/// Near clip plane of the orthographic frustum.
const ORTHOGRAPHIC_NEAR: f32 = 0.01;

/// Far clip plane of the orthographic frustum.
const ORTHOGRAPHIC_FAR: f32 = 100.0;

/// The rectangle the plotting box occupies on a projection plane, as
/// `(center_x, center_y, half_width, half_height)`.
///
/// One description of "where the box is in the frame", shared by both
/// projections, because it is the same question: the eight corners of the
/// aspect-scaled box are transformed into view space, mapped onto the fitted
/// plane, and their bounding rectangle is taken. Only `to_plane` differs — an
/// orthographic camera reads the view-space coordinates directly, a perspective
/// one divides by depth.
///
/// The rectangle is then grown on whichever axis has room to match the
/// viewport's aspect ratio, so the box is always fully visible and never
/// stretched, and scaled about its own centre by `zoom`.
///
/// Fitting the *centre* as well as the extent is what makes a perspective fit
/// tight: a perspective silhouette is not symmetric about the view axis (the
/// near corners subtend more than the far ones), so a frustum centred on the
/// view axis leaves a band of empty frame on one side no matter how it is
/// scaled.
fn fitted_box_rect(
    view: Mat4,
    axis_aspect: Vec3,
    viewport_aspect: f32,
    zoom: f32,
    to_plane: impl Fn(Vec3) -> (f32, f32),
) -> (f32, f32, f32, f32) {
    let mut min = (f32::INFINITY, f32::INFINITY);
    let mut max = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for index in 0..8_usize {
        let signs = Vec3::new(
            if index & 1 == 0 { -1.0 } else { 1.0 },
            if index & 2 == 0 { -1.0 } else { 1.0 },
            if index & 4 == 0 { -1.0 } else { 1.0 },
        );
        let (x, y) = to_plane(view.transform_point3(signs * axis_aspect));
        min = (min.0.min(x), min.1.min(y));
        max = (max.0.max(x), max.1.max(y));
    }

    // `zoom` scales the whole plane about the camera's own axis — the look-at
    // target — which is what "move the camera closer" means. At `zoom == 1` the
    // rectangle is exactly the tight fit computed above; zooming in magnifies
    // around the target and leaves it exactly where it was. Scaling the extent
    // alone would magnify around the box's silhouette instead, which carries
    // the target off the edge of the canvas at high zoom.
    let center = ((min.0 + max.0) * 0.5 / zoom, (min.1 + max.1) * 0.5 / zoom);
    // `Camera3D::validate` guarantees a positive finite zoom and positive finite
    // aspect components, so the extents are already positive; the floor only
    // keeps a pathological caller from producing a singular frustum.
    let half_x = ((max.0 - min.0) * 0.5 / zoom).max(MIN_FITTED_HALF_EXTENT);
    let half_y = ((max.1 - min.1) * 0.5 / zoom).max(MIN_FITTED_HALF_EXTENT);

    // Grow the axis that has room rather than shrinking the one that does not,
    // so the fitted box is always fully visible.
    let (half_width, half_height) = if half_x >= half_y * viewport_aspect {
        (half_x, half_x / viewport_aspect)
    } else {
        (half_y * viewport_aspect, half_y)
    };
    (center.0, center.1, half_width, half_height)
}

/// Orthographic projection that fits the plotting box to the viewport.
///
/// The half-extent used to be a fixed `1.8 / zoom` regardless of camera or
/// aspect, and the default camera only needs about `1.33` — so the scene was
/// drawn at roughly three quarters of the size it could be, in a frame a 2D plot
/// fills to 92% x 97%.
///
/// `zoom` still scales the result, so `.zoom(2.0)` means twice as close on any
/// camera rather than twice as close on one particular one.
fn orthographic_fit(view: Mat4, axis_aspect: Vec3, viewport_aspect: f32, zoom: f32) -> Mat4 {
    // An orthographic projection has no divide: view-space x and y *are* the
    // coordinates on the projection plane.
    let (center_x, center_y, half_width, half_height) =
        fitted_box_rect(view, axis_aspect, viewport_aspect, zoom, |in_view| {
            (in_view.x, in_view.y)
        });
    Mat4::orthographic_rh(
        center_x - half_width,
        center_x + half_width,
        center_y - half_height,
        center_y + half_height,
        ORTHOGRAPHIC_NEAR,
        ORTHOGRAPHIC_FAR,
    )
}

/// Perspective projection that fits the plotting box to the viewport.
///
/// The exact counterpart of [`orthographic_fit`], on the same
/// [`fitted_box_rect`]. This used to size the frustum from the box's *bounding
/// sphere*, whose radius is `sqrt(3)` for a unit box, so a perspective figure
/// was drawn markedly smaller than an orthographic one of the same box with the
/// difference left as empty frame on every side.
///
/// The result is an off-axis frustum, which is what a tight fit requires: see
/// [`fitted_box_rect`]. The near and far planes still bracket the whole sphere,
/// so nothing clips.
fn perspective_fit(
    view: Mat4,
    axis_aspect: Vec3,
    viewport_aspect: f32,
    zoom: f32,
    scene_radius: f32,
    eye_distance: f32,
) -> Mat4 {
    // A right-handed camera looks down its own -Z, so depth is `-z`. A corner at
    // or behind the eye has no finite projection; the floor keeps the frustum
    // from opening to infinity if a caller aims inside the box.
    let (center_x, center_y, half_width, half_height) =
        fitted_box_rect(view, axis_aspect, viewport_aspect, zoom, |in_view| {
            let depth = (-in_view.z).max(MIN_PERSPECTIVE_DEPTH);
            (in_view.x / depth, in_view.y / depth)
        });

    let near = (eye_distance - scene_radius * 1.25).max(0.001);
    let far = eye_distance + scene_radius * 1.25;
    // `fitted_box_rect` works in tangents; the frustum wants the rectangle those
    // tangents cut out of the near plane.
    Mat4::frustum_rh(
        (center_x - half_width) * near,
        (center_x + half_width) * near,
        (center_y - half_height) * near,
        (center_y + half_height) * near,
        near,
        far,
    )
}

/// Smallest fitted half-extent, so a frustum is never singular.
const MIN_FITTED_HALF_EXTENT: f32 = 1.0e-4;

/// Smallest view-space depth a corner may claim when fitting a frustum.
const MIN_PERSPECTIVE_DEPTH: f32 = 1.0e-4;

/// Distance from `target` to the farthest corner of the aspect-scaled box.
///
/// The plotting box spans `-aspect ..= aspect` on every axis, so this is the
/// radius of the smallest sphere around the target that still contains every
/// drawable vertex — exactly what the near and far planes have to bracket.
fn scene_radius_about(target: Vec3, axis_aspect: Vec3) -> f32 {
    let farthest = Vec3::new(
        target.x.abs() + axis_aspect.x,
        target.y.abs() + axis_aspect.y,
        target.z.abs() + axis_aspect.z,
    );
    farthest.length().max(f32::MIN_POSITIVE)
}

fn validate_viewport(width: u32, height: u32) -> Result<()> {
    if width == 0 || height == 0 {
        Err(PlottingError::InvalidDimensions { width, height })
    } else {
        Ok(())
    }
}

fn validate_screen_coordinate(field: &'static str, value: f32) -> Result<()> {
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

fn unproject_local(
    inverse_view_projection: Mat4,
    screen_x: f32,
    screen_y: f32,
    depth: f32,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<Vec3> {
    let ndc_x = screen_x / viewport_width as f32 * 2.0 - 1.0;
    let ndc_y = 1.0 - screen_y / viewport_height as f32 * 2.0;
    let homogeneous = inverse_view_projection * Vec3::new(ndc_x, ndc_y, depth).extend(1.0);
    if !homogeneous.is_finite() || homogeneous.w.abs() <= f32::EPSILON {
        return Err(PlottingError::InvalidCamera3D {
            field: "inverse_view_projection",
            value: homogeneous.w,
            reason: "produced a non-finite or zero homogeneous divisor",
        });
    }
    Ok(homogeneous.truncate() / homogeneous.w)
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            azimuth_deg: -60.0,
            elevation_deg: 30.0,
            roll_deg: 0.0,
            projection: Projection3D::Orthographic,
            aspect: AxisAspect3D::Auto,
            stable_scale: false,
            zoom: 1.0,
            target: None,
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

/// A normalized data-space ray through a viewport pixel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenRay3D {
    /// Point on the near clip plane.
    pub origin: Point3D,
    /// Unit-length direction in data coordinates.
    pub direction: Point3D,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PreparedCamera3D {
    pub(crate) view: Mat4,
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
    fn named_camera_views_have_stable_pole_safe_orientations() {
        let target = Point3D::new(2.0, 3.0, 4.0);
        let original = Camera3D::default()
            .perspective_deg(37.0)
            .axis_aspect(AxisAspect3D::fixed(1.0, 2.0, 3.0))
            .zoom(2.5)
            .look_at(target)
            .roll_deg(17.0);

        for (view, azimuth, elevation) in [
            (CameraView3D::Isometric, -60.0, 30.0),
            (CameraView3D::Front, -90.0, 0.0),
            (CameraView3D::Back, 90.0, 0.0),
            (CameraView3D::Left, 180.0, 0.0),
            (CameraView3D::Right, 0.0, 0.0),
            (CameraView3D::Top, 0.0, 89.9),
            (CameraView3D::Bottom, 0.0, -89.9),
        ] {
            let camera = original.camera_view(view);
            assert_eq!(camera.get_azimuth_deg(), azimuth);
            assert_eq!(camera.get_elevation_deg(), elevation);
            assert_eq!(camera.get_roll_deg(), 0.0);
            assert_eq!(camera.projection(), original.projection());
            assert_eq!(camera.axis_aspect_value(), original.axis_aspect_value());
            assert_eq!(camera.get_zoom(), original.get_zoom());
            assert_eq!(camera.target(), original.target());
            camera.validate().expect("named camera view is valid");
        }
    }

    #[test]
    fn fit_to_content_recenters_and_restores_zoom_without_reorienting() {
        let bounds = Bounds3D::new(
            Point3D::new(-10.0, 20.0, 5.0),
            Point3D::new(30.0, 80.0, 25.0),
        )
        .expect("bounds");
        let original = Camera3D::default()
            .azimuth_deg(23.0)
            .elevation_deg(-41.0)
            .roll_deg(9.0)
            .perspective_deg(53.0)
            .axis_aspect(AxisAspect3D::Equal)
            .zoom(7.0)
            .look_at(Point3D::new(-10.0, 20.0, 5.0));

        let fitted = original.fit_to_content(bounds);

        assert_eq!(fitted.get_azimuth_deg(), original.get_azimuth_deg());
        assert_eq!(fitted.get_elevation_deg(), original.get_elevation_deg());
        assert_eq!(fitted.get_roll_deg(), original.get_roll_deg());
        assert_eq!(fitted.projection(), original.projection());
        assert_eq!(fitted.axis_aspect_value(), original.axis_aspect_value());
        assert_eq!(fitted.get_zoom(), 1.0);
        assert_eq!(fitted.target(), Some(bounds.center()));
        fitted.validate().expect("fitted camera is valid");
    }

    fn unit_bounds() -> Bounds3D {
        Bounds3D::new(Point3D::new(-1.0, -1.0, -1.0), Point3D::new(1.0, 1.0, 1.0))
            .expect("unit bounds")
    }

    #[test]
    fn scene_radius_tracks_the_aspect_scaled_box_instead_of_a_constant() {
        // The old hard-coded constant happened to be right for `Auto`, so the
        // default camera must be bit-identical.
        assert_abs_diff_eq!(
            scene_radius_about(Vec3::ZERO, AxisAspect3D::Auto.resolved()),
            Vec3::new(1.0, 1.0, 0.75).length(),
            epsilon = 1.0e-6
        );
        // Equal proportions genuinely need a larger sphere...
        assert!(
            scene_radius_about(Vec3::ZERO, AxisAspect3D::Equal.resolved())
                > scene_radius_about(Vec3::ZERO, AxisAspect3D::Auto.resolved())
        );
        // ...and a squashed z axis needs a smaller one.
        assert!(
            scene_radius_about(Vec3::ZERO, AxisAspect3D::fixed(1.0, 1.0, 0.2).resolved())
                < scene_radius_about(Vec3::ZERO, AxisAspect3D::Auto.resolved())
        );
        // An off-centre target has to widen the sphere, not shrink it.
        let aspect = AxisAspect3D::Auto.resolved();
        assert!(
            scene_radius_about(Vec3::new(1.0, 0.0, 0.0), aspect)
                > scene_radius_about(Vec3::ZERO, aspect)
        );
    }

    /// Every corner of the aspect-scaled plotting box, in NDC.
    fn projected_corner_extent(prepared: PreparedCamera3D) -> (f32, f32) {
        let mut max_x = 0.0_f32;
        let mut max_y = 0.0_f32;
        for index in 0..8_usize {
            let signs = Vec3::new(
                if index & 1 == 0 { -1.0 } else { 1.0 },
                if index & 2 == 0 { -1.0 } else { 1.0 },
                if index & 4 == 0 { -1.0 } else { 1.0 },
            );
            let local = signs * prepared.axis_aspect;
            let clip = prepared.view_projection * local.extend(1.0);
            let ndc = clip.truncate() / clip.w;
            max_x = max_x.max(ndc.x.abs());
            max_y = max_y.max(ndc.y.abs());
        }
        (max_x, max_y)
    }

    /// The orthographic frustum is fitted to the box, not to a constant.
    ///
    /// The half-extent used to be `1.8 / zoom` whatever the camera did; the
    /// default camera needs about `1.33`, so a 3D figure was drawn at roughly
    /// three quarters of the size the frame allowed. The box must now touch the
    /// edge of the frame on whichever axis limits it, and never cross it.
    #[test]
    fn orthographic_camera_fits_the_box_to_the_frame() {
        for viewport_aspect in [0.5_f32, 1.0, 4.0 / 3.0, 16.0 / 9.0, 3.0] {
            let prepared = Camera3D::default()
                .prepare(viewport_aspect, unit_bounds())
                .expect("default camera prepares");
            let (max_x, max_y) = projected_corner_extent(prepared);

            assert!(
                max_x <= 1.0 + 1.0e-4 && max_y <= 1.0 + 1.0e-4,
                "box must stay inside the frame: {max_x} x {max_y}"
            );
            assert_abs_diff_eq!(max_x.max(max_y), 1.0, epsilon = 1.0e-4);
        }
    }

    /// Zoom keeps meaning "this much closer" now that the base extent varies.
    #[test]
    fn orthographic_zoom_scales_the_fitted_extent() {
        let unzoomed = Camera3D::default()
            .prepare(1.5, unit_bounds())
            .expect("camera prepares");
        let zoomed = Camera3D::default()
            .zoom(2.0)
            .prepare(1.5, unit_bounds())
            .expect("camera prepares");
        let (_, base_y) = projected_corner_extent(unzoomed);
        let (_, zoomed_y) = projected_corner_extent(zoomed);
        assert_abs_diff_eq!(zoomed_y, base_y * 2.0, epsilon = 1.0e-4);
    }

    /// A squashed z axis makes the box shorter on screen, and the fit has to
    /// follow it instead of reserving room for a cube.
    #[test]
    fn orthographic_fit_follows_the_axis_aspect() {
        let flat = Camera3D::default()
            .axis_aspect(AxisAspect3D::fixed(1.0, 1.0, 0.05))
            .prepare(1.0, unit_bounds())
            .expect("camera prepares");
        let (max_x, max_y) = projected_corner_extent(flat);
        assert!(max_x <= 1.0 + 1.0e-4 && max_y <= 1.0 + 1.0e-4);
        assert_abs_diff_eq!(max_x.max(max_y), 1.0, epsilon = 1.0e-4);
    }

    #[test]
    fn an_absurd_look_at_target_keeps_the_box_in_front_of_the_eye() {
        for projection in [
            Camera3D::default().orthographic(),
            Camera3D::default().perspective_deg(45.0),
        ] {
            let camera = projection.look_at(Point3D::new(1.0e6, -1.0e6, 1.0e6));
            let projected = camera
                .project(Point3D::new(0.0, 0.0, 0.0), unit_bounds(), 640, 480)
                .expect("clamped target still projects");
            assert!(
                projected.visible,
                "an out-of-bounds look-at target must not push the plot behind the eye"
            );
            assert!(projected.depth.is_finite() && (0.0..=1.0).contains(&projected.depth));
        }
    }

    #[test]
    fn every_box_corner_stays_between_the_near_and_far_planes() {
        for aspect in [
            AxisAspect3D::Auto,
            AxisAspect3D::Equal,
            AxisAspect3D::fixed(1.0, 1.0, 0.2),
            AxisAspect3D::fixed(0.25, 4.0, 1.0),
        ] {
            let camera = Camera3D::default()
                .perspective_deg(45.0)
                .axis_aspect(aspect);
            for x in [-1.0, 1.0] {
                for y in [-1.0, 1.0] {
                    for z in [-1.0, 1.0] {
                        let projected = camera
                            .project(Point3D::new(x, y, z), unit_bounds(), 640, 480)
                            .expect("corner projects");
                        assert!(
                            (0.0..=1.0).contains(&projected.depth),
                            "{aspect:?} corner ({x}, {y}, {z}) fell outside the depth range"
                        );
                    }
                }
            }
        }
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
    fn project_unproject_roundtrips_orthographic_and_perspective_points() {
        let bounds = Bounds3D::new(
            Point3D::new(1.0e12, -4.0, 2.0),
            Point3D::new(1.0e12 + 8.0, 12.0, 10.0),
        )
        .expect("bounds");
        let point = Point3D::new(1.0e12 + 3.0, 1.5, 6.0);
        for camera in [
            Camera3D::default(),
            Camera3D::default().perspective_deg(45.0),
        ] {
            let projected = camera.project(point, bounds, 900, 600).expect("projection");
            assert!(projected.visible);
            let unprojected = camera
                .unproject_at_depth(projected.x, projected.y, projected.depth, bounds, 900, 600)
                .expect("unprojection");
            assert_abs_diff_eq!(unprojected.x, point.x, epsilon = 1.0e-3);
            assert_abs_diff_eq!(unprojected.y, point.y, epsilon = 1.0e-3);
            assert_abs_diff_eq!(unprojected.z, point.z, epsilon = 1.0e-3);
        }
    }

    #[test]
    fn screen_ray_has_unit_data_space_direction() {
        let bounds = Bounds3D::new(Point3D::new(-2.0, -4.0, -8.0), Point3D::new(2.0, 4.0, 8.0))
            .expect("bounds");
        let ray = Camera3D::default()
            .perspective_deg(50.0)
            .screen_ray(320.0, 240.0, bounds, 640, 480)
            .expect("ray");
        let length = ray
            .direction
            .x
            .hypot(ray.direction.y)
            .hypot(ray.direction.z);
        assert_abs_diff_eq!(length, 1.0, epsilon = 1.0e-9);
    }

    #[test]
    fn unprojection_rejects_depth_outside_wgpu_range() {
        let bounds = Bounds3D::new(Point3D::new(-1.0, -1.0, -1.0), Point3D::new(1.0, 1.0, 1.0))
            .expect("bounds");
        let error = Camera3D::default()
            .unproject_at_depth(10.0, 10.0, -0.1, bounds, 100, 100)
            .expect_err("negative depth");
        assert!(matches!(
            error,
            PlottingError::InvalidCamera3D { field: "depth", .. }
        ));
    }

    /// Centre of the box's projected silhouette — what a reader sees as the
    /// middle of the picture. It is *not* the projected data centre: perspective
    /// makes the near corners subtend more than the far ones, so the silhouette
    /// sits off the view axis, and a frustum centred on the view axis instead
    /// leaves a band of empty frame down one side.
    fn silhouette_center(camera: Camera3D, bounds: Bounds3D) -> (f32, f32) {
        let mut min = (f32::INFINITY, f32::INFINITY);
        let mut max = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for index in 0..8_usize {
            let corner = Point3D::new(
                if index & 1 == 0 { -1.0 } else { 1.0 },
                if index & 2 == 0 { -1.0 } else { 1.0 },
                if index & 4 == 0 { -1.0 } else { 1.0 },
            );
            let at = camera.project(corner, bounds, 800, 600).expect("corner");
            min = (min.0.min(at.x), min.1.min(at.y));
            max = (max.0.max(at.x), max.1.max(at.y));
        }
        ((min.0 + max.0) * 0.5, (min.1 + max.1) * 0.5)
    }

    #[test]
    fn an_unzoomed_perspective_box_is_centred_in_its_frame() {
        let bounds = Bounds3D::new(Point3D::new(-1.0, -1.0, -1.0), Point3D::new(1.0, 1.0, 1.0))
            .expect("bounds");
        let center = silhouette_center(Camera3D::default().perspective_deg(45.0), bounds);

        assert_abs_diff_eq!(center.0, 400.0, epsilon = 1.0);
        assert_abs_diff_eq!(center.1, 300.0, epsilon = 1.0);
    }

    #[test]
    fn perspective_zoom_magnifies_about_the_look_at_target() {
        let bounds = Bounds3D::new(Point3D::new(-1.0, -1.0, -1.0), Point3D::new(1.0, 1.0, 1.0))
            .expect("bounds");
        let point = Point3D::new(1.0, 0.0, 0.0);

        let normal = Camera3D::default()
            .perspective_deg(45.0)
            .project(point, bounds, 800, 600)
            .expect("normal");
        let zoomed = Camera3D::default()
            .perspective_deg(45.0)
            .zoom(2.0)
            .project(point, bounds, 800, 600)
            .expect("zoomed");
        assert!((zoomed.x - 400.0).abs() > (normal.x - 400.0).abs());

        // The look-at target — the thing the caller said to look at — is the
        // fixed point of the zoom: the picture grows around it and it does not
        // drift, however far in you go. Zooming about the box's silhouette
        // instead would carry the target clean off the canvas at high zoom.
        let target_at = |zoom: f32| {
            let at = Camera3D::default()
                .perspective_deg(45.0)
                .zoom(zoom)
                .project(bounds.center(), bounds, 800, 600)
                .expect("target");
            (at.x, at.y)
        };

        let anchor = target_at(1.0);
        for zoom in [2.0, 8.0, 64.0] {
            let at = target_at(zoom);
            assert_abs_diff_eq!(at.0, anchor.0, epsilon = 1.0e-2);
            assert_abs_diff_eq!(at.1, anchor.1, epsilon = 1.0e-2);
        }
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
