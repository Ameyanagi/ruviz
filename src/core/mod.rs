//! Core plotting functionality and main API

pub mod plot;
pub mod builder;
pub mod types;
pub mod error;
pub mod position;
pub mod subplot;

pub use plot::{Plot, BackendType};
pub use position::Position;
pub use types::{BoundingBox, Point2f};
pub use error::{PlottingError, Result};
pub use subplot::{SubplotFigure, GridSpec, subplots, subplots_default};

#[cfg(test)]
mod validation_test;