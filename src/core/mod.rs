//! Core plotting functionality and main API

pub mod plot;
pub mod builder;
pub mod types;
pub mod error;
pub mod position;

pub use plot::Plot;
pub use position::Position;
pub use types::{BoundingBox, Point2f};
pub use error::{PlottingError, Result};

#[cfg(test)]
mod validation_test;