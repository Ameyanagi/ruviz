//! Three-dimensional plot data and configuration.
//!
//! The public construction path is intentionally small:
//!
//! ```rust,no_run
//! # #[cfg(feature = "3d")]
//! # {
//! use ruviz::prelude::*;
//!
//! let x = [0.0, 1.0, 2.0];
//! let y = [0.0, 1.0, 0.5];
//! let z = [0.0, 0.5, 1.5];
//!
//! scatter3d(&x, &y, &z).save("scatter3d.png")?;
//! # }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! High-level 3D series are resolved into backend-neutral mesh, line, and point
//! batches by the separate `core::plot3d` pipeline. They do not enter the
//! existing 2D `SeriesType` match graph.

mod data;
mod line;
mod scatter;
mod surface;
mod wireframe;

pub(crate) use data::{Grid3DData, Points3DData};
pub use line::Line3DConfig;
pub use scatter::Scatter3DConfig;
pub use surface::{Surface3DConfig, SurfaceSampling, SurfaceShading};
pub use wireframe::Wireframe3DConfig;
