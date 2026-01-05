//! Distribution plot types
//!
//! Plots for visualizing statistical distributions.
//! - Violin plots
//! - KDE plots
//! - Boxen (letter-value) plots
//! - ECDF plots

pub mod boxen;
pub mod ecdf;
pub mod kde;
pub mod violin;

pub use boxen::{BoxenBox, BoxenConfig, BoxenData, BoxenOrientation, boxen_rect, compute_boxen};
pub use ecdf::{EcdfConfig, EcdfPlotData, EcdfStat, compute_ecdf, ecdf_range};
pub use kde::{
    Kde2dPlotConfig, Kde2dPlotData, KdePlotConfig, KdePlotData, compute_kde_2d_plot,
    compute_kde_plot, kde_fill_polygon,
};
pub use violin::{
    BandwidthMethod, Orientation, ViolinConfig, ViolinData, ViolinScale, close_violin_polygon,
    violin_polygon,
};
