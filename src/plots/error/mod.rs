//! Error visualization plot types
//!
//! Plots for showing uncertainty and errors.
//! - Error bars
//! - Fill between (area/band plots)
//!
//! # Trait-Based API
//!
//! Error plots implement the core plot traits:
//! - [`crate::plots::PlotCompute`]: Data transformation
//! - [`crate::plots::PlotData`]: Common data interface
//! - [`crate::plots::PlotRender`]: Rendering capability

pub mod errorbar;

pub use errorbar::{
    ErrorBar, ErrorBarConfig, ErrorBarData, ErrorBarInput, ErrorBarPlot, ErrorLineStyle,
    ErrorValues, compute_error_bars, error_bar_range,
};
