//! Error visualization plot types
//!
//! Plots for showing uncertainty and errors.
//! - Error bars
//! - Fill between (area/band plots)

pub mod errorbar;

pub use errorbar::{
    ErrorBar, ErrorBarConfig, ErrorLineStyle, ErrorValues, compute_error_bars, error_bar_range,
};
