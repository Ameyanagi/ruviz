//! Continuous data plot types
//!
//! Plots for continuous 2D data.
//! - Contour plots
//! - Hexbin plots
//! - Area/fill plots

pub mod area;
pub mod contour;
pub mod hexbin;

pub use area::{
    AreaConfig, AreaInterpolation, StackBaseline, StackPlotConfig, area_polygon, compute_stack,
    fill_between_polygon, fill_between_where,
};
pub use contour::{
    ContourConfig, ContourPlotData, compute_contour_plot, contour_fill_regions, contour_range,
};
pub use hexbin::{
    HexBin, HexbinConfig, HexbinPlotData, ReduceFunction, compute_hexbin, hexbin_range,
};
