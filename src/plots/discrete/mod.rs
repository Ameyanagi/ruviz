//! Discrete plot types
//!
//! Plots for discrete/stepped data.
//! - Step plots
//! - Stem plots

pub mod stem;
pub mod step;

pub use stem::{StemConfig, StemElement, StemMarker, StemOrientation, compute_stems, stem_range};
pub use step::{StepConfig, StepWhere, step_line, step_polygon, step_range};
