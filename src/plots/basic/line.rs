//! Line plot configuration
//!
//! Provides [`LineConfig`] for configuring line plot appearance.

use crate::plots::traits::PlotConfig;
use crate::render::{Color, LineStyle, MarkerStyle};

/// Configuration for line plots
///
/// Controls the appearance of line series including line style, width,
/// color, and optional markers.
///
/// # Example
///
/// ```rust
/// use ruviz::plots::basic::LineConfig;
/// use ruviz::render::{LineStyle, MarkerStyle};
///
/// let config = LineConfig::new()
///     .line_width(2.5)
///     .line_style(LineStyle::Dashed)
///     .with_markers(MarkerStyle::Circle, 6.0);
/// ```
#[derive(Debug, Clone)]
pub struct LineConfig {
    /// Line width in points (default: from theme)
    pub line_width: Option<f32>,
    /// Line style (solid, dashed, etc.)
    pub line_style: LineStyle,
    /// Line color (None = auto from palette)
    pub color: Option<Color>,
    /// Optional marker style for data points
    pub marker: Option<MarkerStyle>,
    /// Marker size in points (default: 6.0)
    pub marker_size: f32,
    /// Whether to show markers on data points
    pub show_markers: bool,
    /// Line alpha/transparency (0.0-1.0)
    pub alpha: f32,
    /// Whether to connect points with lines (true) or just markers (false)
    pub draw_line: bool,
}

impl Default for LineConfig {
    fn default() -> Self {
        Self {
            line_width: None,
            line_style: LineStyle::Solid,
            color: None,
            marker: None,
            marker_size: 6.0,
            show_markers: false,
            alpha: 1.0,
            draw_line: true,
        }
    }
}

impl PlotConfig for LineConfig {}

impl LineConfig {
    /// Create a new line configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set line width in points
    ///
    /// # Arguments
    /// * `width` - Line width (minimum 0.1)
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = Some(width.max(0.1));
        self
    }

    /// Set line style
    ///
    /// # Arguments
    /// * `style` - Line style (Solid, Dashed, Dotted, etc.)
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.line_style = style;
        self
    }

    /// Set line color
    ///
    /// # Arguments
    /// * `color` - Color for the line
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Enable markers on data points
    ///
    /// # Arguments
    /// * `style` - Marker style to use
    /// * `size` - Marker size in points
    pub fn with_markers(mut self, style: MarkerStyle, size: f32) -> Self {
        self.marker = Some(style);
        self.marker_size = size.max(0.1);
        self.show_markers = true;
        self
    }

    /// Set marker style (also enables markers)
    ///
    /// # Arguments
    /// * `style` - Marker style to use
    pub fn marker(mut self, style: MarkerStyle) -> Self {
        self.marker = Some(style);
        self.show_markers = true;
        self
    }

    /// Set marker size in points
    ///
    /// # Arguments
    /// * `size` - Marker size (minimum 0.1)
    pub fn marker_size(mut self, size: f32) -> Self {
        self.marker_size = size.max(0.1);
        self
    }

    /// Set whether to show markers
    pub fn show_markers(mut self, show: bool) -> Self {
        self.show_markers = show;
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

    /// Set whether to draw the connecting line
    ///
    /// Set to `false` to show only markers without connecting lines.
    pub fn draw_line(mut self, draw: bool) -> Self {
        self.draw_line = draw;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LineConfig::default();
        assert!(config.line_width.is_none());
        assert!(matches!(config.line_style, LineStyle::Solid));
        assert!(config.color.is_none());
        assert!(!config.show_markers);
        assert!(config.draw_line);
        assert!((config.alpha - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_builder_methods() {
        let config = LineConfig::new()
            .line_width(2.5)
            .line_style(LineStyle::Dashed)
            .color(Color::RED)
            .with_markers(MarkerStyle::Circle, 8.0)
            .alpha(0.7);

        assert_eq!(config.line_width, Some(2.5));
        assert!(matches!(config.line_style, LineStyle::Dashed));
        assert_eq!(config.color, Some(Color::RED));
        assert!(config.show_markers);
        assert!(matches!(config.marker, Some(MarkerStyle::Circle)));
        assert!((config.marker_size - 8.0).abs() < f32::EPSILON);
        assert!((config.alpha - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_line_width_clamping() {
        let config = LineConfig::new().line_width(-5.0);
        assert_eq!(config.line_width, Some(0.1));
    }

    #[test]
    fn test_alpha_clamping() {
        let config_low = LineConfig::new().alpha(-0.5);
        let config_high = LineConfig::new().alpha(1.5);

        assert!((config_low.alpha - 0.0).abs() < f32::EPSILON);
        assert!((config_high.alpha - 1.0).abs() < f32::EPSILON);
    }
}
