//! Plot type implementations

pub mod boxplot;
pub mod histogram;
pub mod statistics;

pub use boxplot::{BoxPlotConfig, BoxPlotData, calculate_box_plot};
pub use histogram::{BinMethod, HistogramConfig, HistogramData, calculate_histogram};
pub use statistics::{iqr, mean, median, percentile, std_dev};
