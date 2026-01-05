//! Discrete plot types
//!
//! Plots for discrete/stepped data.
//! - Step plots
//! - Stem plots
//!
//! # Trait-Based API
//!
//! Discrete plots implement the core plot traits:
//! - [`crate::plots::PlotCompute`]: Data transformation
//! - [`crate::plots::PlotData`]: Common data interface
//! - [`crate::plots::PlotRender`]: Rendering capability

pub mod stem;
pub mod step;

pub use stem::{
    Stem, StemConfig, StemData, StemElement, StemInput, StemMarker, StemOrientation, compute_stems,
    stem_range,
};
pub use step::{
    Step, StepConfig, StepData, StepInput, StepWhere, step_line, step_polygon, step_range,
};
