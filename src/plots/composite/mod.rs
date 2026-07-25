//! Composite plot types — **layout math only, not drawable**.
//!
//! [`joint_plot_layout`] and [`compute_pairplot_layout`] return normalised
//! panel rectangles, and [`compute_marginal_histogram`] bins one margin. There
//! is **no renderer and no `Plot` builder method** for either joint plots or
//! pair plots: you get rectangles, and placing content in them is your job.
//! `JointPlotConfig::rugplot` is doubly inert — rug plots are themselves
//! unwired.
//!
//! Inset/zoom axes are *not* here; those are real and live on the `Plot`
//! builder as `inset_layout`/`inset_anchor`.
//!
//! Wiring this up is Phase 10 of
//! `docs/roadmaps/ruviz-audit-remediation-plan.md`; it needs `add_axes`.

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
