//! Polar coordinate plot types
//!
//! Plots using polar coordinates.
//! - Polar line/scatter
//! - Radar/spider charts

pub mod polar_plot;
pub mod radar;

pub use polar_plot::{
    PolarPlotConfig, PolarPlotData, PolarPoint, circle_vertices, compute_polar_plot, polar_grid,
};
pub use radar::{RadarConfig, RadarPlotData, RadarSeries, compute_radar_chart};
