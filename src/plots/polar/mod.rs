//! Polar coordinate plot types
//!
//! Plots using polar coordinates.
//! - Polar line/scatter
//! - Radar/spider charts
//!
//! # Trait-Based API
//!
//! Polar plots implement the core plot traits:
//! - [`crate::plots::PlotCompute`]: Data transformation
//! - [`crate::plots::PlotData`]: Common data interface
//! - [`crate::plots::PlotRender`]: Rendering capability

pub mod polar_plot;
pub mod radar;

pub use polar_plot::{
    PolarPlot, PolarPlotConfig, PolarPlotData, PolarPlotInput, PolarPoint, circle_vertices,
    compute_polar_plot, polar_grid,
};
pub use radar::{Radar, RadarConfig, RadarInput, RadarPlotData, RadarSeries, compute_radar_chart};
