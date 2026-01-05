//! Hierarchical plot types
//!
//! Plots for hierarchical and clustering data.
//! - Dendrograms
//! - Clustermaps

pub mod dendrogram;

pub use dendrogram::{
    DendrogramConfig, DendrogramLink, DendrogramOrientation, DendrogramPlotData, TruncateMode,
    compute_dendrogram, dendrogram_lines,
};
