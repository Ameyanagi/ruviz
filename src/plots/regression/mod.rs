//! Regression plot types
//!
//! Plots for regression analysis.
//! - Regression plots (scatter + fit line)
//! - Residual plots
//! - Point plots

pub mod regplot;

pub use regplot::{
    RegPlotConfig, RegPlotData, ResidPlotConfig, ResidPlotData, compute_regplot, compute_residplot,
};
