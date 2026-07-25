//! Grid styling configuration
//!
//! Provides unified grid styling across all plot types for visual consistency.
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::core::GridStyle;
//!
//! // Default grid style
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
/// All plot types should use this configuration to ensure
/// identical grid appearance across the library.
///
/// # Visibility contract
///
/// The defaults are tuned so that, on a white background:
/// - the major grid is legible enough to read a value off an axis,
/// - the minor grid is clearly subordinate to the major grid,
/// - both stay clearly behind the plotted data.
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
    /// Create the default grid style
    ///
    /// - `visible: true` - grid is shown by default
    /// - `color: #B0B0B0` - mid gray, readable against a white background
    /// - `line_width: 0.8pt` - thin, but wide enough to survive antialiasing
    /// - `alpha: 1.0` - fully opaque; the gray itself sets the contrast
    /// - `line_style: Solid` - solid lines
    /// - `minor: false` - no minor grid by default
    /// - `minor_line_width: 0.4pt` / `minor_alpha: 0.5` - half the width and
    ///   half the opacity of the major grid, so minor lines read as secondary
    fn default() -> Self {
        Self {
            visible: true,
            color: Color::from_gray(176), // #B0B0B0
            line_width: 0.8,
            alpha: 1.0,
            line_style: LineStyle::Solid,
            minor: false,
            minor_line_width: 0.4,
            minor_alpha: 0.5,
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
    /// Darker and thicker than [`GridStyle::default`]. Useful for plots where
    /// reading values off the grid matters more than data/grid separation.
    pub fn prominent() -> Self {
        Self {
            visible: true,
            color: Color::from_gray(140), // #8C8C8C, darker than the default grid
            line_width: 1.0,
            alpha: 1.0,
            line_style: LineStyle::Solid,
            minor: false,
            minor_line_width: 0.5,
            minor_alpha: 0.6,
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

    /// Relative luminance of an opaque sRGB gray, per WCAG.
    fn gray_luminance(value: u8) -> f32 {
        let c = value as f32 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    /// Contrast ratio of `style` composited over a white background.
    fn contrast_on_white(color: Color, alpha: f32) -> f32 {
        // Source-over onto white: 255 - alpha * (255 - channel).
        let composited = (255.0 - alpha * (255.0 - color.r as f32)).round() as u8;
        let l = gray_luminance(composited);
        1.05 / (l + 0.05)
    }

    #[test]
    fn test_default_grid_is_visible() {
        let style = GridStyle::default();

        // Grid should be visible by default
        assert!(style.visible);

        // Color should be mid gray (#B0B0B0 = rgb(176, 176, 176))
        assert_eq!(style.color.r, 176);
        assert_eq!(style.color.g, 176);
        assert_eq!(style.color.b, 176);

        // Line width should be 0.8pt
        assert!((style.line_width - 0.8).abs() < 0.001);

        // Alpha should be fully opaque - the gray sets the contrast
        assert!((style.alpha - 1.0).abs() < 0.001);

        // Should use solid lines
        assert!(matches!(style.line_style, LineStyle::Solid));

        // Minor grid should be off by default
        assert!(!style.minor);
    }

    #[test]
    fn test_default_grid_has_readable_contrast_on_white() {
        let style = GridStyle::default();

        // The old default (#CCCCCC at alpha 0.3) composited to ~#F0F0F0,
        // about 1.14:1 against white - functionally invisible.
        let major = contrast_on_white(style.color, style.alpha);
        assert!(
            major > 1.6,
            "major grid contrast on white too low: {major}:1"
        );

        // Minor grid must be present but clearly subordinate to the major grid.
        let minor = contrast_on_white(style.color, style.minor_alpha);
        assert!(
            minor > 1.0,
            "minor grid contrast on white too low: {minor}:1"
        );
        assert!(
            minor < major,
            "minor grid ({minor}:1) should recede behind major grid ({major}:1)"
        );
    }

    #[test]
    fn test_default_minor_grid_is_subordinate() {
        let style = GridStyle::default();

        // Roughly half the width and half the opacity of the major grid.
        assert!(style.minor_line_width < style.line_width);
        assert!((style.minor_line_width - style.line_width * 0.5).abs() < 0.05);
        assert!(style.minor_alpha < style.alpha);
    }

    #[test]
    fn test_hidden() {
        let style = GridStyle::hidden();
        assert!(!style.visible);
        // Other defaults should still apply
        assert_eq!(style.color, GridStyle::default().color);
        assert!((style.alpha - GridStyle::default().alpha).abs() < 0.001);
    }

    #[test]
    fn test_prominent() {
        let style = GridStyle::prominent();
        let default = GridStyle::default();

        assert!(style.visible);
        // More visible than default: darker gray and thicker lines.
        assert!(style.color.r < default.color.r);
        assert!(style.line_width > default.line_width);
        assert!(
            contrast_on_white(style.color, style.alpha)
                > contrast_on_white(default.color, default.alpha)
        );
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

        // Default grid is opaque, so alpha survives the round trip
        assert_eq!(effective.r, 176);
        assert_eq!(effective.g, 176);
        assert_eq!(effective.b, 176);
        assert_eq!(effective.a, 255); // 1.0 * 255
    }

    #[test]
    fn test_effective_minor_color() {
        let style = GridStyle::default();
        let minor = style.effective_minor_color();

        // Same hue as the major grid, but half opacity
        assert_eq!(minor.r, 176);
        assert_eq!(minor.a, 127); // 0.5 * 255 = 127.5, truncated
        assert!(minor.a < style.effective_color().a);
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
