//! Regression plot types — **compute only, not drawable**.
//!
//! [`compute_regplot`] and [`compute_residplot`] return fitted values and a
//! confidence band. There is **no renderer and no `Plot` builder method**, so
//! neither can be drawn through the public API. Point plots do not exist at
//! all.
//!
//! The confidence band is additionally not trusted: see Phase 10 of
//! `docs/roadmaps/ruviz-audit-remediation-plan.md`, which requires the
//! statistics be corrected before anything is allowed to draw them.

pub mod regplot;

pub use regplot::{
    RegPlotConfig, RegPlotData, ResidPlotConfig, ResidPlotData, compute_regplot, compute_residplot,
};
