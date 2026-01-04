//! Plot type implementations

pub mod boxplot;
pub mod heatmap;
pub mod histogram;
pub mod statistics;

pub use boxplot::{BoxPlotConfig, BoxPlotData, calculate_box_plot};
pub use heatmap::{
    HeatmapConfig, HeatmapData, Interpolation, process_heatmap, process_heatmap_flat,
};
pub use histogram::{BinMethod, HistogramConfig, HistogramData, calculate_histogram};
pub use statistics::{iqr, mean, median, percentile, std_dev};
