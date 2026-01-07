//! Plot configuration for display settings
//!
//! This module contains the [`PlotConfiguration`] struct which holds
//! display-related settings like title, labels, dimensions, and theme.

use crate::core::config::PlotConfig;
use crate::render::Theme;

/// Configuration for plot display settings
///
/// This struct holds the basic display configuration for a plot:
/// - Title and axis labels
/// - Canvas dimensions and DPI
/// - Visual theme
/// - DPI-independent configuration
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::core::plot::PlotConfiguration;
/// use ruviz::render::Theme;
///
/// let config = PlotConfiguration::new()
///     .with_title("My Plot")
///     .with_xlabel("X Axis")
///     .with_ylabel("Y Axis")
///     .with_dimensions(800, 600);
/// ```
#[derive(Debug, Clone)]
pub struct PlotConfiguration {
    /// Plot title
    pub(crate) title: Option<String>,
    /// X-axis label
    pub(crate) xlabel: Option<String>,
    /// Y-axis label
    pub(crate) ylabel: Option<String>,
    /// Canvas dimensions (width, height) - DEPRECATED: use config.figure instead
    pub(crate) dimensions: (u32, u32),
    /// DPI for high-resolution export - DEPRECATED: use config.figure.dpi instead
    pub(crate) dpi: u32,
    /// Plot theme
    pub(crate) theme: Theme,
    /// DPI-independent plot configuration
    pub(crate) config: PlotConfig,
}

impl Default for PlotConfiguration {
    fn default() -> Self {
        Self::new()
    }
}

impl PlotConfiguration {
    /// Create a new plot configuration with default settings
    pub fn new() -> Self {
        Self {
            title: None,
            xlabel: None,
            ylabel: None,
            dimensions: (800, 600),
            dpi: 100,
            theme: Theme::default(),
            config: PlotConfig::default(),
        }
    }

    /// Set the plot title
    pub fn with_title<S: Into<String>>(mut self, title: S) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the X-axis label
    pub fn with_xlabel<S: Into<String>>(mut self, label: S) -> Self {
        self.xlabel = Some(label.into());
        self
    }

    /// Set the Y-axis label
    pub fn with_ylabel<S: Into<String>>(mut self, label: S) -> Self {
        self.ylabel = Some(label.into());
        self
    }

    /// Set canvas dimensions (width, height)
    #[deprecated(
        since = "0.8.0",
        note = "Use with_config() and PlotConfig for DPI-independent sizing"
    )]
    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.dimensions = (width, height);
        self
    }

    /// Set DPI for export
    #[deprecated(
        since = "0.8.0",
        note = "Use with_config() and PlotConfig for DPI-independent sizing"
    )]
    pub fn with_dpi(mut self, dpi: u32) -> Self {
        self.dpi = dpi;
        self
    }

    /// Set the plot theme
    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the DPI-independent plot configuration
    pub fn with_config(mut self, config: PlotConfig) -> Self {
        self.config = config;
        self
    }

    // Getters

    /// Get the plot title
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Get the X-axis label
    pub fn xlabel(&self) -> Option<&str> {
        self.xlabel.as_deref()
    }

    /// Get the Y-axis label
    pub fn ylabel(&self) -> Option<&str> {
        self.ylabel.as_deref()
    }

    /// Get canvas dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }

    /// Get DPI
    pub fn dpi(&self) -> u32 {
        self.dpi
    }

    /// Get the theme
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Get the DPI-independent configuration
    pub fn config(&self) -> &PlotConfig {
        &self.config
    }

    /// Get mutable reference to DPI-independent configuration
    pub fn config_mut(&mut self) -> &mut PlotConfig {
        &mut self.config
    }

    // Mutable setters for delegation from Plot

    /// Set title (mutable version)
    pub(crate) fn set_title<S: Into<String>>(&mut self, title: S) {
        self.title = Some(title.into());
    }

    /// Set xlabel (mutable version)
    pub(crate) fn set_xlabel<S: Into<String>>(&mut self, label: S) {
        self.xlabel = Some(label.into());
    }

    /// Set ylabel (mutable version)
    pub(crate) fn set_ylabel<S: Into<String>>(&mut self, label: S) {
        self.ylabel = Some(label.into());
    }

    /// Set dimensions (mutable version)
    pub(crate) fn set_dimensions(&mut self, width: u32, height: u32) {
        self.dimensions = (width, height);
    }

    /// Set DPI (mutable version)
    pub(crate) fn set_dpi(&mut self, dpi: u32) {
        self.dpi = dpi;
    }

    /// Set theme (mutable version)
    pub(crate) fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configuration() {
        let config = PlotConfiguration::new();
        assert!(config.title().is_none());
        assert!(config.xlabel().is_none());
        assert!(config.ylabel().is_none());
        assert_eq!(config.dimensions(), (800, 600));
        assert_eq!(config.dpi(), 100);
    }

    #[test]
    fn test_builder_pattern() {
        let config = PlotConfiguration::new()
            .with_title("Test Title")
            .with_xlabel("X Label")
            .with_ylabel("Y Label");

        assert_eq!(config.title(), Some("Test Title"));
        assert_eq!(config.xlabel(), Some("X Label"));
        assert_eq!(config.ylabel(), Some("Y Label"));
    }

    #[test]
    fn test_theme_configuration() {
        let config = PlotConfiguration::new().with_theme(Theme::dark());

        // Theme should be set
        assert!(config.theme().background != Theme::default().background);
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_dimensions() {
        let config = PlotConfiguration::new()
            .with_dimensions(1920, 1080)
            .with_dpi(300);

        assert_eq!(config.dimensions(), (1920, 1080));
        assert_eq!(config.dpi(), 300);
    }

    #[test]
    fn test_mutable_setters() {
        let mut config = PlotConfiguration::new();
        config.set_title("New Title");
        config.set_xlabel("New X");
        config.set_ylabel("New Y");
        config.set_dimensions(1024, 768);
        config.set_dpi(150);

        assert_eq!(config.title(), Some("New Title"));
        assert_eq!(config.xlabel(), Some("New X"));
        assert_eq!(config.ylabel(), Some("New Y"));
        assert_eq!(config.dimensions(), (1024, 768));
        assert_eq!(config.dpi(), 150);
    }
}
