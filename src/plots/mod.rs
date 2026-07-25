//! Plot type implementations
//!
//! **21 plot types are reachable from the [`Plot`](crate::core::Plot)
//! builder**, plus 4 more from `Plot3D` when the `3d` feature is enabled — 25
//! in total. The first table below is the complete list. If a type is not in
//! it, it has no builder entry point and *cannot* be drawn through the public
//! API, however many types this source tree appears to contain.
//!
//! ## Core Traits
//!
//! All plot types implement the core traits defined in [`traits`]:
//!
//! - [`PlotCompute`]: Data transformation
//! - [`PlotData`]: Common data interface
//! - [`PlotRender`]: Rendering to canvas
//!
//! ## Available plot types
//!
//! | Category | Module | Plot types | `Plot` builder method |
//! |----------|--------|------------|-----------------------|
//! | Basic | [`basic`] | Line, Scatter, Bar | `line`, `scatter`, `bar` |
//! | Statistical | [`histogram`], [`boxplot`] | Histogram, Box Plot | `histogram`, `boxplot` |
//! | Distribution | [`distribution`] | KDE, ECDF, Violin, Boxen | `kde`, `ecdf`, `violin`, `boxen` |
//! | Composition | [`composition`] | Pie, Donut | `pie`, `pie(..).donut(ratio)` |
//! | Continuous | [`continuous`] | Contour, Area, Fill Between | `contour`, `area`, `fill_between` |
//! | Discrete | [`discrete`] | Step, Stem | `step`, `stem` |
//! | Grid | [`heatmap`] | Heatmap | `heatmap` |
//! | Error | [`error`] | Error Bars | `error_bars`, `error_bars_xy` |
//! | Polar | [`polar`] | Polar Line, Radar | `polar_line`, `radar` |
//! | Vector | [`vector`] | Quiver | `quiver` |
//! | 3D (`3d` feature) | `three_d` | Scatter3D, Line3D, Surface, Wireframe | `Plot3D::{scatter3d, line3d, surface, wireframe}` |
//!
//! ## Not available: implemented but unreachable
//!
//! The following live in this tree but have **no builder entry point**, so a
//! user cannot draw them. They are documented here so the table above is not
//! mistaken for a partial list — not as advertised features. Use them only if
//! you are driving the low-level compute functions yourself.
//!
//! | Type | Module | Compute | Renderer | `Plot` builder |
//! |------|--------|---------|----------|----------------|
//! | Hexbin | [`continuous`] | yes | yes | **no** |
//! | Strip, Swarm | [`categorical`] | yes | yes | **no** |
//! | Grouped Bar, Stacked Bar | [`categorical`] | yes | yes | **no** |
//! | Stacked Area | [`continuous`] | yes | yes | **no** |
//! | Rug | [`distribution`] | yes | stub — draws nothing | **no** |
//! | 2D KDE | [`distribution`] | yes | **no** | **no** |
//! | Dendrogram | `hierarchical` | yes | **no** | **no** |
//! | Joint plot, Pair plot | `composite` | layout only | **no** | **no** |
//! | Reg plot, Resid plot | `regression` | yes | **no** | **no** |
//! | Sankey, Streamplot | `flow` | **no** | **no** | **no** |

pub mod traits;

// Basic plot types (line, scatter, bar)
pub mod basic;

pub mod boxplot;
pub mod heatmap;
pub mod histogram;
pub mod statistics;

// New plot type categories (placeholders for now)
pub mod categorical;
pub mod composition;
pub mod continuous;
pub mod discrete;
pub mod distribution;
pub mod error;
pub mod polar;
#[cfg(feature = "3d")]
pub mod three_d;
pub mod vector;

// ---------------------------------------------------------------------------
// Unwired plot families.
//
// These modules are `pub` because their compute functions are usable and are
// referenced by docs/guide/04_plot_types.md, but none of them has a renderer
// *or* a `Plot` builder method, so nothing here can produce an image through
// the public API. They are `#[doc(hidden)]` so docs.rs stops advertising
// plot types (Sankey, Streamplot, dendrograms, joint/pair plots, regression
// plots) that a user has no way to draw. Un-hide each one at the point where
// it grows a builder — see Phase 10 of docs/roadmaps/ruviz-audit-remediation-plan.md.
//
// Nothing is deprecated here: the compute functions are correct and useful on
// their own, they are simply not a rendering feature yet.
// ---------------------------------------------------------------------------

/// Multi-panel composites (joint plot, pair plot). Layout math only — no
/// renderer and no `Plot` builder; not reachable as a plot type.
#[doc(hidden)]
pub mod composite;

/// Flow diagrams (Sankey, streamplot). **Contains no implementation at all.**
#[doc(hidden)]
pub mod flow;

/// Hierarchical/clustering plots (dendrogram). Compute only — no renderer and
/// no `Plot` builder; not reachable as a plot type.
#[doc(hidden)]
pub mod hierarchical;

/// Regression plots (regplot, residplot). Compute only — no renderer and no
/// `Plot` builder; not reachable as a plot type, and the confidence band the
/// compute step returns is not yet verified.
#[doc(hidden)]
pub mod regression;

// Core trait exports
pub use traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender, StyledShape};

// Basic plot config exports
pub use basic::{BarConfig, BarOrientation, LineConfig, ScatterConfig};

// Distribution plot exports
pub use distribution::{
    BandwidthMethod, Boxen, BoxenConfig, BoxenData, BoxenOrientation, Ecdf, EcdfConfig, EcdfData,
    EcdfStat, Kde, KdeConfig, KdeData, Violin, ViolinConfig, ViolinData, compute_boxen,
    compute_ecdf, compute_kde,
};

pub use boxplot::{BoxPlotConfig, BoxPlotData, OutlierMethod, WhiskerMethod, calculate_box_plot};
pub use heatmap::{
    HeatmapConfig, HeatmapData, HeatmapOrigin, Interpolation, process_heatmap, process_heatmap_flat,
};
pub use histogram::{BinMethod, HistogramConfig, HistogramData, calculate_histogram};
pub use statistics::{iqr, mean, median, percentile, std_dev};

// Contour plot exports
pub use continuous::contour::{
    ContourConfig, ContourInterpolation, ContourPlotData, compute_contour_plot,
};
pub use discrete::{StemConfig, StemMarker, StemOrientation, StepConfig, StepWhere};

// Pie chart exports
pub use composition::pie::{PieConfig, PieData};

// Polar and Radar exports
pub use polar::polar_plot::{PolarPlotConfig, PolarPlotData, compute_polar_plot};
pub use polar::radar::{
    RadarConfig, RadarPlotData, compute_radar_chart, compute_radar_chart_with_labels,
};
#[cfg(feature = "3d")]
pub use three_d::{
    Line3DConfig, Scatter3DConfig, Surface3DConfig, SurfaceSampling, SurfaceShading,
    Wireframe3DConfig,
};
pub use vector::{
    Quiver, QuiverArrow, QuiverConfig, QuiverInput, QuiverPivot, QuiverPlotData, compute_quiver,
    quiver_range,
};
