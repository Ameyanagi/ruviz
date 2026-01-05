//! Composition plot types
//!
//! Plots showing parts of a whole.
//! - Pie charts
//! - Donut charts
//!
//! # Trait-Based API
//!
//! Composition plots implement the core plot traits:
//! - [`crate::plots::PlotCompute`]: Data transformation
//! - [`crate::plots::PlotData`]: Common data interface
//! - [`crate::plots::PlotRender`]: Rendering capability

pub mod pie;

pub use pie::{Pie, PieConfig, PieData, render_pie};
