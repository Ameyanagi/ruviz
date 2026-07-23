//! Separate retained-mode foundation for three-dimensional plots.

mod builder;
mod types;

pub use builder::{Line3DBuilder, Scatter3DBuilder, Surface3DBuilder, Wireframe3DBuilder};
pub use types::{AxisAspect3D, Bounds3D, Camera3D, Point3D, ProjectedPoint3D, Projection3D};
