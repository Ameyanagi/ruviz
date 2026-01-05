//! Distribution plot types
//!
//! Plots for visualizing statistical distributions:
//! - KDE plots (kernel density estimation)
//! - ECDF plots (empirical cumulative distribution)
//! - Violin plots
//! - Boxen (letter-value) plots
//!
//! # Trait-Based API
//!
//! All distribution plots implement the core traits:
//! - [`crate::plots::PlotCompute`]: Data transformation
//! - [`crate::plots::PlotData`]: Common data interface
//! - [`crate::plots::PlotRender`]: Rendering capability

pub mod boxen;
pub mod ecdf;
pub mod kde;
pub mod violin;

// Primary exports (new names)
pub use boxen::{
    Boxen, BoxenBox, BoxenConfig, BoxenData, BoxenOrientation, boxen_rect, compute_boxen,
};
pub use ecdf::{Ecdf, EcdfConfig, EcdfData, EcdfStat, compute_ecdf, ecdf_range};
pub use kde::{
    Kde, Kde2dPlotConfig, Kde2dPlotData, KdeConfig, KdeData, compute_kde, compute_kde_2d_plot,
    kde_fill_polygon,
};
pub use violin::{
    BandwidthMethod, Orientation, Violin, ViolinConfig, ViolinData, ViolinScale,
    close_violin_polygon, violin_polygon,
};

// Deprecated re-exports for backward compatibility
#[allow(deprecated)]
pub use ecdf::EcdfPlotData;
#[allow(deprecated)]
pub use kde::{KdePlotConfig, KdePlotData, compute_kde_plot};
