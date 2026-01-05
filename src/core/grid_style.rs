//! Grid styling configuration with seaborn whitegrid defaults
//!
//! Provides unified grid styling across all plot types for visual consistency.
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::core::GridStyle;
//!
//! // Default seaborn whitegrid style
//! let style = GridStyle::default();
//!
//! // No grid
//! let hidden = GridStyle::hidden();
//!
//! // Customize grid appearance
//! let custom = GridStyle::default()
//!     .color(Color::LIGHT_GRAY)
//!     .alpha(0.5)
//!     .line_width(1.0);
//! ```

use crate::render::{Color, LineStyle};

/// Grid styling configuration
///
/// Matches seaborn's whitegrid style for visual consistency.
/// All plot types should use this configuration to ensure
/// identical grid appearance across the library.
///
/// # seaborn whitegrid Compatibility
///
/// seaborn whitegrid uses:
/// - Grid visible by default
/// - Light gray color (`.8` = 80% gray ≈ #CCCCCC)
/// - Solid line style
/// - Subtle alpha for non-intrusive appearance
#[derive(Debug, Clone, PartialEq)]
pub struct GridStyle {
    /// Show major grid lines
    pub visible: bool,
    /// Grid line color
    pub color: Color,
    /// Grid line width (in points)
    pub line_width: f32,
    /// Grid line alpha (0.0 = transparent, 1.0 = opaque)
    pub alpha: f32,
    /// Grid line style
    pub line_style: LineStyle,
    /// Show minor grid lines
    pub minor: bool,
    /// Minor grid line width (in points)
    pub minor_line_width: f32,
    /// Minor grid line alpha
    pub minor_alpha: f32,
}

impl Default for GridStyle {
    /// Create default grid style matching seaborn whitegrid
    ///
    /// - `visible: true` - grid is shown by default
    /// - `color: #CCCCCC` - light gray (seaborn `.8`)
    /// - `line_width: 0.5` - subtle line width
    /// - `alpha: 0.3` - low alpha for non-intrusive appearance
    /// - `line_style: Solid` - solid lines
    /// - `minor: false` - no minor grid by default
    fn default() -> Self {
        Self {
            visible: true,
            color: Color::from_gray(204), // #CCCCCC (seaborn .8)
            line_width: 0.5,
            alpha: 0.3,
            line_style: LineStyle::Solid,
            minor: false,
            minor_line_width: 0.25,
            minor_alpha: 0.15,
        }
    }
}

impl GridStyle {
    /// Create a new grid style with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a grid style with no visible grid
    pub fn hidden() -> Self {
        Self {
            visible: false,
            ..Default::default()
        }
    }

    /// Create a more prominent grid style
    ///
    /// Useful for plots where grid alignment is important.
    pub fn prominent() -> Self {
        Self {
            visible: true,
            color: Color::from_gray(176), // Darker gray
            line_width: 0.8,
            alpha: 0.5,
            line_style: LineStyle::Solid,
            minor: false,
            minor_line_width: 0.4,
            minor_alpha: 0.25,
        }
    }

    /// Set whether grid is visible
    pub fn visible(mut self, enabled: bool) -> Self {
        self.visible = enabled;
        self
    }

    /// Set grid line color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set grid line width (in points)
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.0);
        self
    }

    /// Set grid line alpha (0.0 = transparent, 1.0 = opaque)
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set grid line style
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.line_style = style;
        self
    }

    /// Set whether to show minor grid
    pub fn minor(mut self, enabled: bool) -> Self {
        self.minor = enabled;
        self
    }

    /// Set minor grid line width (in points)
    pub fn minor_line_width(mut self, width: f32) -> Self {
        self.minor_line_width = width.max(0.0);
        self
    }

    /// Set minor grid line alpha
    pub fn minor_alpha(mut self, alpha: f32) -> Self {
        self.minor_alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Get the effective grid color with alpha applied
    pub fn effective_color(&self) -> Color {
        self.color.with_alpha(self.alpha)
    }

    /// Get the effective minor grid color with alpha applied
    pub fn effective_minor_color(&self) -> Color {
        self.color.with_alpha(self.minor_alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_matches_seaborn_whitegrid() {
        let style = GridStyle::default();

        // Grid should be visible by default
        assert!(style.visible);

        // Color should be light gray (#CCCCCC = rgb(204, 204, 204))
        assert_eq!(style.color.r, 204);
        assert_eq!(style.color.g, 204);
        assert_eq!(style.color.b, 204);

        // Line width should be 0.5pt
        assert!((style.line_width - 0.5).abs() < 0.001);

        // Alpha should be 0.3 (subtle)
        assert!((style.alpha - 0.3).abs() < 0.001);

        // Should use solid lines
        assert!(matches!(style.line_style, LineStyle::Solid));

        // Minor grid should be off by default
        assert!(!style.minor);
    }

    #[test]
    fn test_hidden() {
        let style = GridStyle::hidden();
        assert!(!style.visible);
        // Other defaults should still apply
        assert!((style.alpha - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_prominent() {
        let style = GridStyle::prominent();
        assert!(style.visible);
        assert!(style.alpha > 0.3); // More visible than default
        assert!(style.line_width > 0.5); // Thicker than default
    }

    #[test]
    fn test_builder_methods() {
        let style = GridStyle::default()
            .visible(false)
            .color(Color::BLUE)
            .line_width(2.0)
            .alpha(0.5)
            .line_style(LineStyle::Dashed)
            .minor(true);

        assert!(!style.visible);
        assert_eq!(style.color, Color::BLUE);
        assert!((style.line_width - 2.0).abs() < 0.001);
        assert!((style.alpha - 0.5).abs() < 0.001);
        assert!(matches!(style.line_style, LineStyle::Dashed));
        assert!(style.minor);
    }

    #[test]
    fn test_effective_color() {
        let style = GridStyle::default();
        let effective = style.effective_color();

        // Should have alpha applied (0.3 * 255 ≈ 76)
        assert_eq!(effective.r, 204);
        assert_eq!(effective.g, 204);
        assert_eq!(effective.b, 204);
        assert_eq!(effective.a, 76); // 0.3 * 255 = 76.5
    }

    #[test]
    fn test_clamping() {
        let style = GridStyle::default()
            .alpha(2.0) // Should clamp to 1.0
            .line_width(-5.0); // Should clamp to 0.0

        assert!((style.alpha - 1.0).abs() < 0.001);
        assert!((style.line_width - 0.0).abs() < 0.001);
    }
}
