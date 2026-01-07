//! Plot type implementations
//!
//! This module provides implementations for 30+ plot types organized by category.
//!
//! ## Core Traits
//!
//! All plot types implement the core traits defined in [`traits`]:
//!
//! - [`PlotCompute`]: Data transformation
//! - [`PlotData`]: Common data interface
//! - [`PlotRender`]: Rendering to canvas
//!
//! ## Plot Categories
//!
//! | Category | Module | Plot Types |
//! |----------|--------|------------|
//! | Distribution | [`distribution`] | KDE, ECDF, Violin, Boxen |
//! | Categorical | [`categorical`] | Grouped Bar, Stacked Bar |
//! | Composition | [`composition`] | Pie, Donut, Area |
//! | Continuous | [`continuous`] | Contour, Hexbin, Fill Between |
//! | Discrete | [`discrete`] | Step, Stem |
//! | Error | [`error`] | Error Bars |
//! | Polar | [`polar`] | Polar Plot, Radar |
//! | Vector | [`vector`] | Quiver |

pub mod traits;

// Basic plot types (line, scatter, bar)
pub mod basic;

pub mod boxplot;
pub mod heatmap;
pub mod histogram;
pub mod statistics;

// New plot type categories (placeholders for now)
pub mod categorical;
pub mod composite;
pub mod composition;
pub mod continuous;
pub mod discrete;
pub mod distribution;
pub mod error;
pub mod flow;
pub mod hierarchical;
pub mod polar;
pub mod regression;
pub mod three_d;
pub mod vector;

// Core trait exports
pub use traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender, StyledShape};

// Basic plot config exports
pub use basic::{BarConfig, BarOrientation, LineConfig, ScatterConfig};

// Distribution plot exports
pub use distribution::{
    Boxen, BoxenConfig, BoxenData, Ecdf, EcdfConfig, EcdfData, EcdfStat, Kde, KdeConfig, KdeData,
    Violin, ViolinConfig, ViolinData, compute_boxen, compute_ecdf, compute_kde,
};

pub use boxplot::{BoxPlotConfig, BoxPlotData, calculate_box_plot};
pub use heatmap::{
    HeatmapConfig, HeatmapData, Interpolation, process_heatmap, process_heatmap_flat,
};
pub use histogram::{BinMethod, HistogramConfig, HistogramData, calculate_histogram};
pub use statistics::{iqr, mean, median, percentile, std_dev};

// Contour plot exports
pub use continuous::contour::{
    ContourConfig, ContourInterpolation, ContourPlotData, compute_contour_plot,
};

// Pie chart exports
pub use composition::pie::{PieConfig, PieData};

// Polar and Radar exports
pub use polar::polar_plot::{PolarPlotConfig, PolarPlotData, compute_polar_plot};
pub use polar::radar::{
    RadarConfig, RadarPlotData, compute_radar_chart, compute_radar_chart_with_labels,
};
