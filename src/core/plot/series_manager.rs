//! Series management for plots
//!
//! This module provides the [`SeriesManager`] struct which handles
//! the collection of data series in a plot, including auto-coloring.

use crate::render::{Color, Theme};

use super::{PlotSeries, SeriesType};

/// Manages the collection of data series in a plot
///
/// The SeriesManager handles:
/// - Storing all data series
/// - Auto-color assignment for series without explicit colors
/// - Series validation
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::core::plot::SeriesManager;
///
/// let mut manager = SeriesManager::new();
/// // Series are typically added through Plot methods
/// ```
#[derive(Clone, Debug, Default)]
pub struct SeriesManager {
    /// Data series
    pub(crate) series: Vec<PlotSeries>,
    /// Auto-generate colors for series without explicit colors
    pub(crate) auto_color_index: usize,
}

impl SeriesManager {
    /// Create a new empty series manager
    pub fn new() -> Self {
        Self {
            series: Vec::new(),
            auto_color_index: 0,
        }
    }

    /// Get the number of series
    pub fn len(&self) -> usize {
        self.series.len()
    }

    /// Check if there are no series
    pub fn is_empty(&self) -> bool {
        self.series.is_empty()
    }

    /// Get a reference to all series
    pub(crate) fn series(&self) -> &[PlotSeries] {
        &self.series
    }

    /// Get the current auto-color index
    pub fn auto_color_index(&self) -> usize {
        self.auto_color_index
    }

    /// Get the next auto-color from the theme and increment the index
    pub fn next_auto_color(&mut self, theme: &Theme) -> Color {
        let color = theme.get_color(self.auto_color_index);
        self.auto_color_index += 1;
        color
    }

    /// Add a series to the manager
    pub(crate) fn push(&mut self, series: PlotSeries) {
        self.series.push(series);
    }

    /// Increment the auto-color index
    pub(crate) fn increment_auto_color(&mut self) {
        self.auto_color_index += 1;
    }

    /// Validate that all series have valid data
    ///
    /// Returns an error if any series has mismatched data lengths
    /// or empty data.
    pub fn validate(&self) -> Result<(), &'static str> {
        for series in &self.series {
            match &series.series_type {
                SeriesType::Line { x_data, y_data } | SeriesType::Scatter { x_data, y_data } => {
                    if x_data.len() != y_data.len() {
                        return Err("X and Y data must have the same length");
                    }
                    if x_data.is_empty() {
                        return Err("Data series cannot be empty");
                    }
                }
                SeriesType::Bar { categories, values } => {
                    if categories.len() != values.len() {
                        return Err("Categories and values must have the same length");
                    }
                    if categories.is_empty() {
                        return Err("Bar chart cannot have empty data");
                    }
                }
                SeriesType::ErrorBars {
                    x_data,
                    y_data,
                    y_errors,
                } => {
                    if x_data.len() != y_data.len() || x_data.len() != y_errors.len() {
                        return Err("Error bar data must have matching lengths");
                    }
                }
                SeriesType::ErrorBarsXY {
                    x_data,
                    y_data,
                    x_errors,
                    y_errors,
                } => {
                    if x_data.len() != y_data.len()
                        || x_data.len() != x_errors.len()
                        || x_data.len() != y_errors.len()
                    {
                        return Err("Error bar data must have matching lengths");
                    }
                }
                SeriesType::Histogram { data, .. } => {
                    if data.is_empty() {
                        return Err("Histogram data cannot be empty");
                    }
                }
                SeriesType::BoxPlot { data, .. } => {
                    if data.is_empty() {
                        return Err("Box plot data cannot be empty");
                    }
                }
                // Other series types don't need validation here
                _ => {}
            }
        }
        Ok(())
    }

    /// Clear all series
    pub fn clear(&mut self) {
        self.series.clear();
        self.auto_color_index = 0;
    }

    /// Get iterator over series
    pub(crate) fn iter(&self) -> impl Iterator<Item = &PlotSeries> {
        self.series.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_series_manager() {
        let manager = SeriesManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
        assert_eq!(manager.auto_color_index(), 0);
    }

    #[test]
    fn test_next_auto_color() {
        let mut manager = SeriesManager::new();
        let theme = Theme::default();

        let color1 = manager.next_auto_color(&theme);
        assert_eq!(manager.auto_color_index(), 1);

        let color2 = manager.next_auto_color(&theme);
        assert_eq!(manager.auto_color_index(), 2);

        // Colors should be different
        assert_ne!(color1, color2);
    }

    #[test]
    fn test_clear() {
        let mut manager = SeriesManager::new();
        manager.auto_color_index = 5;
        manager.clear();
        assert!(manager.is_empty());
        assert_eq!(manager.auto_color_index(), 0);
    }
}
