//! Continuous data plot types
//!
//! Plots for continuous 2D data.
//! - Contour plots
//! - Hexbin plots
//! - Area/fill plots
//!
//! # Trait-Based API
//!
//! Continuous plots implement the core plot traits:
//! - [`crate::plots::PlotCompute`]: Data transformation
//! - [`crate::plots::PlotData`]: Common data interface
//! - [`crate::plots::PlotRender`]: Rendering capability

pub mod area;
pub mod contour;
pub mod hexbin;

pub use area::{
    Area, AreaConfig, AreaData, AreaInput, AreaInterpolation, StackBaseline, StackPlotConfig,
    StackedArea, StackedAreaData, StackedAreaInput, area_polygon, compute_stack,
    fill_between_polygon, fill_between_where,
};
pub use contour::{
    Contour, ContourConfig, ContourInput, ContourInterpolation, ContourPlotData,
    compute_contour_plot, contour_fill_regions, contour_range,
};
pub use hexbin::{
    HexBin, Hexbin, HexbinConfig, HexbinInput, HexbinPlotData, ReduceFunction, compute_hexbin,
    hexbin_range,
};
