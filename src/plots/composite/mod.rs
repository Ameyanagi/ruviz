//! Composite plot types
//!
//! Multi-panel and combined visualizations.
//! - Joint plots (scatter + marginal distributions)
//! - Pair plots (scatter matrix)
//! - Inset/zoom axes

pub mod jointplot;
pub mod pairplot;

pub use jointplot::{
    JointKind, JointPlotConfig, JointPlotLayout, MarginalHistogram, compute_marginal_histogram,
    joint_plot_layout,
};
pub use pairplot::{
    DiagKind, OffDiagKind, PairPlotCell, PairPlotConfig, PairPlotLayout, cell_variable_names,
    compute_pairplot_layout,
};
