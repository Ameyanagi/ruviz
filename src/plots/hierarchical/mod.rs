//! Hierarchical plot types (dendrogram).
//!
//! `Plot::dendrogram` draws one as an ordinary series, on the same chain every
//! other plot type uses:
//!
//! ```text
//! Plot::new().dendrogram(&linkage).label("clusters").legend_best().save("tree.png")?
//! ```
//!
//! [`compute_dendrogram`] turns a [`Linkage`](crate::stats::Linkage) into node
//! positions, [`dendrogram_lines`] turns those into line segments, and
//! [`DendrogramPlotData`] implements
//! [`ComputedSeries`](crate::plots::ComputedSeries), which is how the builder
//! carries it and how both the raster and SVG backends draw it. See
//! [`crate::plots`] for the full reachability table.
//!
//! Clustermaps (a dendrogram gutter attached to a heatmap) do not exist at all;
//! they need arbitrary-rectangle axes, tracked as `add_axes` in
//! `docs/roadmaps/ruviz-audit-remediation-plan.md`.

pub mod dendrogram;

pub use dendrogram::{
    DendrogramConfig, DendrogramLink, DendrogramOrientation, DendrogramPlotData, TruncateMode,
    compute_dendrogram, dendrogram_lines,
};
