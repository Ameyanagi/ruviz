//! Bar chart configuration
//!
//! Provides [`BarConfig`] for configuring bar chart appearance.

use crate::plots::traits::PlotConfig;
use crate::render::Color;

/// Bar orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarOrientation {
    /// Vertical bars (default)
    #[default]
    Vertical,
    /// Horizontal bars
    Horizontal,
}

/// Configuration for bar charts
///
/// Controls the appearance of bar series including bar width,
/// color, edge styling, and orientation.
///
/// # Example
///
/// ```rust
/// use ruviz::plots::basic::{BarConfig, BarOrientation};
/// use ruviz::render::Color;
///
/// let config = BarConfig::new()
///     .width(0.8)
///     .color(Color::BLUE)
///     .edge_width(1.0)
///     .orientation(BarOrientation::Horizontal);
/// ```
#[derive(Debug, Clone)]
pub struct BarConfig {
    /// Bar width as fraction of available space (0.0-1.0, default: 0.8)
    pub width: f32,
    /// Bar fill color (None = auto from palette)
    pub color: Option<Color>,
    /// Bar fill alpha/transparency (0.0-1.0)
    pub alpha: f32,
    /// Edge color (None = auto-darken from fill)
    pub edge_color: Option<Color>,
    /// Edge width in points (default: 0.8)
    pub edge_width: f32,
    /// Bar orientation (vertical or horizontal)
    pub orientation: BarOrientation,
    /// Base value for bars (default: 0.0)
    pub bottom: f64,
    /// Whether to align bars to the left of their position
    pub align_left: bool,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            width: 0.8,
            color: None,
            alpha: 1.0,
            edge_color: None,
            edge_width: 0.8,
            orientation: BarOrientation::Vertical,
            bottom: 0.0,
            align_left: false,
        }
    }
}

impl PlotConfig for BarConfig {}

impl BarConfig {
    /// Create a new bar configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set bar width as fraction of available space
    ///
    /// # Arguments
    /// * `width` - Width fraction (0.0-1.0)
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.clamp(0.0, 1.0);
        self
    }

    /// Set bar fill color
    ///
    /// # Arguments
    /// * `color` - Color for the bar fill
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
    /// * `color` - Color for the bar edge
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

    /// Set bar orientation
    ///
    /// # Arguments
    /// * `orientation` - Vertical or Horizontal
    pub fn orientation(mut self, orientation: BarOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set base value for bars
    ///
    /// Bars extend from this value to the data value.
    /// Default is 0.0.
    ///
    /// # Arguments
    /// * `bottom` - Base value for bars
    pub fn bottom(mut self, bottom: f64) -> Self {
        self.bottom = bottom;
        self
    }

    /// Set whether to align bars to the left of their position
    ///
    /// When true, bars start at their x position.
    /// When false (default), bars are centered on their x position.
    pub fn align_left(mut self, align: bool) -> Self {
        self.align_left = align;
        self
    }

    /// Create a horizontal bar configuration
    pub fn horizontal() -> Self {
        Self::default().orientation(BarOrientation::Horizontal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BarConfig::default();
        assert!((config.width - 0.8).abs() < f32::EPSILON);
        assert!(config.color.is_none());
        assert!((config.alpha - 1.0).abs() < f32::EPSILON);
        assert!(matches!(config.orientation, BarOrientation::Vertical));
        assert!((config.bottom - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_builder_methods() {
        let config = BarConfig::new()
            .width(0.6)
            .color(Color::GREEN)
            .alpha(0.9)
            .edge_color(Color::BLACK)
            .edge_width(2.0)
            .orientation(BarOrientation::Horizontal)
            .bottom(10.0);

        assert!((config.width - 0.6).abs() < f32::EPSILON);
        assert_eq!(config.color, Some(Color::GREEN));
        assert!((config.alpha - 0.9).abs() < f32::EPSILON);
        assert_eq!(config.edge_color, Some(Color::BLACK));
        assert!((config.edge_width - 2.0).abs() < f32::EPSILON);
        assert!(matches!(config.orientation, BarOrientation::Horizontal));
        assert!((config.bottom - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_width_clamping() {
        let config_low = BarConfig::new().width(-0.5);
        let config_high = BarConfig::new().width(1.5);

        assert!((config_low.width - 0.0).abs() < f32::EPSILON);
        assert!((config_high.width - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_horizontal_constructor() {
        let config = BarConfig::horizontal();
        assert!(matches!(config.orientation, BarOrientation::Horizontal));
    }
}
