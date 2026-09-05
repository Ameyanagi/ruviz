use std::collections::HashSet;

use crate::core::{Bounds3D, PlottingError, Result};
use crate::render::Color;

use super::Point3D;

/// A sphere in data coordinates, with a stable, application-owned pick ID.
///
/// `color.a` controls opacity. IDs must be unique within a sphere series and
/// are returned by [`super::PickHit3D::sources`], even after reordering atoms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere3D {
    pub id: u32,
    pub center: Point3D,
    /// Radius in the same units as `center`, never pixels or points.
    pub radius: f64,
    pub color: Color,
}

impl Sphere3D {
    pub const fn new(id: u32, center: Point3D, radius: f64, color: Color) -> Self {
        Self {
            id,
            center,
            radius,
            color,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SphereStyle3D {
    pub(crate) shaded: bool,
    pub(crate) specular: f32,
    pub(crate) gloss: f32,
}

impl Default for SphereStyle3D {
    fn default() -> Self {
        Self {
            shaded: true,
            specular: 0.15,
            gloss: 32.0,
        }
    }
}

impl SphereStyle3D {
    pub(crate) fn validate(self) -> Result<()> {
        if !self.specular.is_finite()
            || !(0.0..=1.0).contains(&self.specular)
            || !self.gloss.is_finite()
            || !(1.0..=256.0).contains(&self.gloss)
        {
            return Err(PlottingError::InvalidInput(
                "spheres3d: specular strength must be in 0..=1 and gloss in 1..=256".into(),
            ));
        }
        Ok(())
    }
}

pub(super) fn sphere_bounds(spheres: &[Sphere3D]) -> Result<Bounds3D> {
    let mut ids = HashSet::with_capacity(spheres.len());
    for sphere in spheres {
        let p = sphere.center;
        if !p.x.is_finite()
            || !p.y.is_finite()
            || !p.z.is_finite()
            || !sphere.radius.is_finite()
            || sphere.radius <= 0.0
        {
            return Err(PlottingError::InvalidInput(format!(
                "spheres3d: sphere {} needs a finite center and a finite positive radius",
                sphere.id
            )));
        }
        if !ids.insert(sphere.id) {
            return Err(PlottingError::InvalidInput(format!(
                "spheres3d: duplicate sphere ID {}",
                sphere.id
            )));
        }
        for c in [p.x, p.y, p.z] {
            if !(c - sphere.radius).is_finite()
                || !(c + sphere.radius).is_finite()
                || c - sphere.radius == c + sphere.radius
            {
                return Err(PlottingError::InvalidInput(format!(
                    "spheres3d: sphere {} radius cannot be represented at its center",
                    sphere.id
                )));
            }
        }
    }
    Bounds3D::from_points(spheres.iter().flat_map(|sphere| {
        let p = sphere.center;
        let r = sphere.radius;
        [
            Point3D::new(p.x - r, p.y - r, p.z - r),
            Point3D::new(p.x + r, p.y + r, p.z + r),
        ]
    }))
}
