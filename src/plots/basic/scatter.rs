//! Scatter plot configuration
//!
//! Provides [`ScatterConfig`] for configuring scatter plot appearance.

use crate::plots::traits::PlotConfig;
use crate::render::{Color, MarkerStyle};

/// Configuration for scatter plots
///
/// Controls the appearance of scatter series including marker style,
/// size, color, and optional edge styling.
///
/// # Example
///
/// ```rust
/// use ruviz::plots::basic::ScatterConfig;
/// use ruviz::render::MarkerStyle;
///
/// let config = ScatterConfig::new()
///     .marker(MarkerStyle::Diamond)
///     .size(10.0)
///     .edge_width(1.5);
/// ```
#[derive(Debug, Clone)]
pub struct ScatterConfig {
    /// Marker style (default: Circle)
    pub marker: MarkerStyle,
    /// Marker size in points (default: 6.0)
    pub size: f32,
    /// Marker fill color (None = auto from palette)
    pub color: Option<Color>,
    /// Marker alpha/transparency (0.0-1.0)
    pub alpha: f32,
    /// Edge color (None = auto-darken from fill)
    pub edge_color: Option<Color>,
    /// Edge width in points (default: 0.5)
    pub edge_width: f32,
    /// Whether to show edge around markers
    pub show_edge: bool,
}

impl Default for ScatterConfig {
    fn default() -> Self {
        Self {
            marker: MarkerStyle::Circle,
            size: 6.0,
            color: None,
            alpha: 1.0,
            edge_color: None,
            edge_width: 0.5,
            show_edge: true,
        }
    }
}

impl PlotConfig for ScatterConfig {}

impl ScatterConfig {
    /// Create a new scatter configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set marker style
    ///
    /// # Arguments
    /// * `marker` - Marker style (Circle, Square, Triangle, etc.)
    pub fn marker(mut self, marker: MarkerStyle) -> Self {
        self.marker = marker;
        self
    }

    /// Set marker size in points
    ///
    /// # Arguments
    /// * `size` - Marker size (minimum 0.1)
    pub fn size(mut self, size: f32) -> Self {
        self.size = size.max(0.1);
        self
    }

    /// Set marker fill color
    ///
    /// # Arguments
    /// * `color` - Color for the marker fill
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set transparency
    ///
    /// # Arguments
    /// * `alpha` - Transparency value (0.0 = transparent, 1.0 = opaque)
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set edge color explicitly
    ///
    /// If not set, edge color is auto-derived by darkening the fill color.
    ///
    /// # Arguments
    /// * `color` - Color for the marker edge
    pub fn edge_color(mut self, color: Color) -> Self {
        self.edge_color = Some(color);
        self
    }

    /// Set edge width in points
    ///
    /// # Arguments
    /// * `width` - Edge line width (minimum 0.0)
    pub fn edge_width(mut self, width: f32) -> Self {
        self.edge_width = width.max(0.0);
        self
    }

    /// Set whether to show edges around markers
    pub fn show_edge(mut self, show: bool) -> Self {
        self.show_edge = show;
        self
    }

    /// Convenience method to set size (alias for `size()`)
    ///
    /// Matches matplotlib's `s` parameter naming.
    pub fn s(self, size: f32) -> Self {
        self.size(size)
    }

    /// Convenience method to set color (alias for `color()`)
    ///
    /// Matches matplotlib's `c` parameter naming.
    pub fn c(self, color: Color) -> Self {
        self.color(color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ScatterConfig::default();
        assert!(matches!(config.marker, MarkerStyle::Circle));
        assert!((config.size - 6.0).abs() < f32::EPSILON);
        assert!(config.color.is_none());
        assert!((config.alpha - 1.0).abs() < f32::EPSILON);
        assert!(config.show_edge);
    }

    #[test]
    fn test_builder_methods() {
        let config = ScatterConfig::new()
            .marker(MarkerStyle::Square)
            .size(12.0)
            .color(Color::BLUE)
            .alpha(0.8)
            .edge_color(Color::BLACK)
            .edge_width(1.5);

        assert!(matches!(config.marker, MarkerStyle::Square));
        assert!((config.size - 12.0).abs() < f32::EPSILON);
        assert_eq!(config.color, Some(Color::BLUE));
        assert!((config.alpha - 0.8).abs() < f32::EPSILON);
        assert_eq!(config.edge_color, Some(Color::BLACK));
        assert!((config.edge_width - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_size_clamping() {
        let config = ScatterConfig::new().size(-5.0);
        assert!((config.size - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_matplotlib_aliases() {
        let config = ScatterConfig::new().s(10.0).c(Color::GREEN);

        assert!((config.size - 10.0).abs() < f32::EPSILON);
        assert_eq!(config.color, Some(Color::GREEN));
    }
}
