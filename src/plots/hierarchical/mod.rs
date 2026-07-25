//! Hierarchical plot types — **compute only, not drawable**.
//!
//! [`compute_dendrogram`] turns a [`Linkage`](crate::stats::Linkage) into node
//! positions and [`dendrogram_lines`] turns those into line segments. There is
//! **no renderer and no `Plot` builder method**, so a dendrogram cannot be
//! drawn through the public API — you would have to place the segments
//! yourself. Clustermaps do not exist at all.
//!
//! Wiring this up is Phase 10 of
//! `docs/roadmaps/ruviz-audit-remediation-plan.md`; it needs `add_axes`.

pub mod dendrogram;

pub use dendrogram::{
    DendrogramConfig, DendrogramLink, DendrogramOrientation, DendrogramPlotData, TruncateMode,
    compute_dendrogram, dendrogram_lines,
};
