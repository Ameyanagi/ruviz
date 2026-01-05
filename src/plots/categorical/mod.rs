//! Categorical plot types
//!
//! Plots for displaying categorical data comparisons.
//! - Stacked bar charts
//! - Grouped bar charts
//! - Horizontal bar charts
//! - Strip plots
//! - Swarm plots

pub mod bar;
pub mod strip;
pub mod swarm;

pub use bar::{
    BarOrientation, BarRect, GroupedBarConfig, StackedBarConfig, compute_grouped_bars,
    compute_stacked_bars, grouped_bar_range, stacked_bar_range,
};
pub use strip::{StripConfig, StripOrientation, StripPoint, compute_strip_points, strip_range};
pub use swarm::{SwarmConfig, SwarmOrientation, SwarmPoint, compute_swarm_points, swarm_range};
