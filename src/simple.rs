//! Simple API for quick plotting with minimal code
//!
//! This module provides one-liner convenience functions for common plotting tasks.
//! All functions automatically optimize backend selection based on data size.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use ruviz::simple::*;
//!
//! // Simple line plot
//! let x = vec![0.0, 1.0, 2.0, 3.0];
//! let y = vec![0.0, 1.0, 4.0, 9.0];
//! line_plot(&x, &y, "line.png")?;
//!
//! // With title
//! line_plot_with_title(&x, &y, "My Plot", "line_titled.png")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Available Functions
//!
//! | Function | Description |
//! |----------|-------------|
//! | [`line_plot`] | Create a line plot |
//! | [`scatter_plot`] | Create a scatter plot |
//! | [`bar_chart`] | Create a bar chart |
//! | [`histogram`] | Create a histogram |
//!
//! ![Line plot](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/line_plot.png)

use crate::prelude::*;
use std::path::Path;

/// Create a simple line plot with one function call
///
/// Automatically optimizes backend selection based on data size.
///
/// # Example
/// ```rust,no_run
/// use ruviz::simple::*;
///
/// let x = vec![0.0, 1.0, 2.0, 3.0];
/// let y = vec![0.0, 1.0, 4.0, 9.0];
/// line_plot(&x, &y, "output.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn line_plot<P: AsRef<Path>>(x: &[f64], y: &[f64], path: P) -> Result<()> {
    Plot::new().line(&x, &y).auto_optimize().save(path)
}

/// Create a line plot with a title
///
/// # Example
/// ```rust,no_run
/// use ruviz::simple::*;
///
/// let x = vec![0.0, 1.0, 2.0];
/// let y = vec![0.0, 1.0, 4.0];
/// line_plot_with_title(&x, &y, "My Plot", "output.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn line_plot_with_title<P: AsRef<Path>>(
    x: &[f64],
    y: &[f64],
    title: &str,
    path: P,
) -> Result<()> {
    Plot::new()
        .line(&x, &y)
        .title(title)
        .auto_optimize()
        .save(path)
}

/// Create a simple scatter plot with one function call
///
/// Automatically optimizes backend selection based on data size.
///
/// # Example
/// ```rust,no_run
/// use ruviz::simple::*;
///
/// let x = vec![1.0, 2.0, 3.0];
/// let y = vec![1.0, 4.0, 9.0];
/// scatter_plot(&x, &y, "scatter.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn scatter_plot<P: AsRef<Path>>(x: &[f64], y: &[f64], path: P) -> Result<()> {
    Plot::new().scatter(&x, &y).auto_optimize().save(path)
}

/// Create a scatter plot with a title
///
/// # Example
/// ```rust,no_run
/// use ruviz::simple::*;
///
/// let x = vec![1.0, 2.0, 3.0];
/// let y = vec![1.0, 4.0, 9.0];
/// scatter_plot_with_title(&x, &y, "Scatter", "output.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn scatter_plot_with_title<P: AsRef<Path>>(
    x: &[f64],
    y: &[f64],
    title: &str,
    path: P,
) -> Result<()> {
    Plot::new()
        .scatter(&x, &y)
        .title(title)
        .auto_optimize()
        .save(path)
}

/// Create a simple bar chart with one function call
///
/// Automatically optimizes backend selection based on data size.
///
/// # Example
/// ```rust,no_run
/// use ruviz::simple::*;
///
/// let categories = vec!["A", "B", "C"];
/// let values = vec![10.0, 20.0, 15.0];
/// bar_chart(&categories, &values, "bar.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn bar_chart<P: AsRef<Path>>(categories: &[&str], values: &[f64], path: P) -> Result<()> {
    Plot::new()
        .bar(&categories, &values)
        .auto_optimize()
        .save(path)
}

/// Create a bar chart with a title
///
/// # Example
/// ```rust,no_run
/// use ruviz::simple::*;
///
/// let categories = vec!["X", "Y", "Z"];
/// let values = vec![5.0, 10.0, 7.0];
/// bar_chart_with_title(&categories, &values, "Sales", "output.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn bar_chart_with_title<P: AsRef<Path>>(
    categories: &[&str],
    values: &[f64],
    title: &str,
    path: P,
) -> Result<()> {
    Plot::new()
        .bar(&categories, &values)
        .title(title)
        .auto_optimize()
        .save(path)
}

/// Create a simple histogram with one function call
///
/// Automatically optimizes backend selection and bin calculation.
///
/// # Example
/// ```rust,no_run
/// use ruviz::simple::*;
///
/// let data = vec![1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0];
/// histogram(&data, "histogram.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn histogram<P: AsRef<Path>>(data: &[f64], path: P) -> Result<()> {
    Plot::new()
        .histogram(&data, None)
        .auto_optimize()
        .save(path)
}

/// Create a histogram with a title
///
/// # Example
/// ```rust,no_run
/// use ruviz::simple::*;
///
/// let data = vec![1.0, 2.0, 2.0, 3.0, 4.0];
/// histogram_with_title(&data, "Distribution", "output.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn histogram_with_title<P: AsRef<Path>>(data: &[f64], title: &str, path: P) -> Result<()> {
    Plot::new()
        .histogram(&data, None)
        .title(title)
        .auto_optimize()
        .save(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_api_exists() {
        // Ensure module compiles and functions are available
        let _ = line_plot::<&str>;
        let _ = scatter_plot::<&str>;
        let _ = bar_chart::<&str>;
        let _ = histogram::<&str>;
    }
}
