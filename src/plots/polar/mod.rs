//! Polar coordinate plot types
//!
//! Plots using polar coordinates with configurable axis labels.
//!
//! # Plot Types
//!
//! - **Polar plots**: Line/scatter plots in polar coordinates with angular (θ)
//!   and radial (r) axis labels
//! - **Radar charts**: Spider/star charts for multivariate data with category labels
//!
//! # Label Configuration
//!
//! Both plot types support configurable axis labels:
//!
//! ```rust,ignore
//! use ruviz::plots::polar::{PolarPlotConfig, RadarConfig};
//!
//! // Polar plot with angular labels (0°, 30°, 60°, etc.)
//! let polar_config = PolarPlotConfig::new()
//!     .show_theta_labels(true)
//!     .show_r_labels(true)
//!     .label_font_size(10.0);
//!
//! // Radar chart with category labels
//! let radar_config = RadarConfig::new()
//!     .labels(vec!["A", "B", "C"].into_iter().map(String::from).collect())
//!     .show_axis_labels(true)
//!     .label_font_size(11.0);
//! ```
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
    PolarPlot, PolarPlotConfig, PolarPlotData, PolarPlotInput, PolarPoint, PositionedLabel,
    circle_vertices, compute_polar_plot, polar_grid,
};
pub use radar::{
    Radar, RadarConfig, RadarInput, RadarPlotData, RadarSeries, compute_radar_chart,
    compute_radar_chart_with_labels,
};
