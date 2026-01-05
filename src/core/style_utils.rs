//! Style resolution utilities for theme-aware plot styling
//!
//! This module provides [`StyleResolver`], a central utility for resolving
//! styling values with theme fallbacks. It ensures consistent styling across
//! all plot types by providing a single source of truth for style resolution.
//!
//! # Design Philosophy
//!
//! Plot configurations use `Option<T>` for theme-derivable fields where
//! `None` means "use theme default". StyleResolver handles this resolution
//! consistently across all plot types.
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::core::style_utils::StyleResolver;
//! use ruviz::render::Theme;
//!
//! let theme = Theme::seaborn();
//! let resolver = StyleResolver::new(&theme);
//!
//! // Use config value if present, otherwise theme default
//! let line_width = resolver.line_width(config.line_width);
//!
//! // Generate edge color from fill color
//! let edge = resolver.edge_color(fill_color, config.edge_color);
//! ```

use crate::render::{Color, Theme};

/// Central utility for resolving styling values with theme fallbacks
///
/// All methods are `#[inline]` for performance - overhead is negligible (<1μs per call).
#[derive(Debug, Clone, Copy)]
pub struct StyleResolver<'a> {
    theme: &'a Theme,
}

impl<'a> StyleResolver<'a> {
    /// Create a new StyleResolver with the given theme
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }

    /// Get the underlying theme reference
    #[inline]
    pub fn theme(&self) -> &'a Theme {
        self.theme
    }

    /// Resolve line width, using config override or theme default
    ///
    /// # Arguments
    /// * `config_override` - Explicit line width from config, or None to use theme
    ///
    /// # Returns
    /// The resolved line width in points
    #[inline]
    pub fn line_width(&self, config_override: Option<f32>) -> f32 {
        config_override.unwrap_or(self.theme.line_width)
    }

    /// Resolve fill alpha value
    ///
    /// # Arguments
    /// * `config_override` - Explicit alpha from config, or None to use default
    /// * `default` - Default alpha value when config is None
    ///
    /// # Returns
    /// The resolved alpha value (0.0-1.0)
    #[inline]
    pub fn fill_alpha(&self, config_override: Option<f32>, default: f32) -> f32 {
        config_override.unwrap_or(default).clamp(0.0, 1.0)
    }

    /// Scale font size relative to theme base size
    ///
    /// # Arguments
    /// * `scale` - Scale factor (1.0 = theme.font_size)
    ///
    /// # Returns
    /// The scaled font size in points
    #[inline]
    pub fn font_size(&self, scale: f32) -> f32 {
        self.theme.font_size * scale
    }

    /// Resolve edge color, using explicit value or auto-darkening fill color
    ///
    /// This is the standard method for generating edge colors from fill colors.
    /// When no explicit edge color is provided, generates a 30% darker version
    /// of the fill color (matching matplotlib/seaborn behavior).
    ///
    /// # Arguments
    /// * `fill` - The fill color (used to derive edge if explicit is None)
    /// * `explicit` - Explicit edge color, or None to auto-derive
    ///
    /// # Returns
    /// The resolved edge color
    #[inline]
    pub fn edge_color(&self, fill: Color, explicit: Option<Color>) -> Color {
        explicit.unwrap_or_else(|| fill.darken(0.3))
    }

    /// Resolve edge color with custom darkening factor
    ///
    /// # Arguments
    /// * `fill` - The fill color
    /// * `explicit` - Explicit edge color, or None to auto-derive
    /// * `darken_factor` - How much to darken (0.0-1.0)
    #[inline]
    pub fn edge_color_with_factor(
        &self,
        fill: Color,
        explicit: Option<Color>,
        darken_factor: f32,
    ) -> Color {
        explicit.unwrap_or_else(|| fill.darken(darken_factor))
    }

    /// Get grid line width (typically thinner than data lines)
    ///
    /// Returns theme.line_width * 0.5 by default.
    #[inline]
    pub fn grid_line_width(&self) -> f32 {
        self.theme.line_width * 0.5
    }

    /// Get axis line width
    ///
    /// Returns theme.line_width * 0.5 by default.
    #[inline]
    pub fn axis_line_width(&self) -> f32 {
        self.theme.line_width * 0.5
    }

    /// Get tick line width
    ///
    /// Returns theme.line_width * 0.4 by default.
    #[inline]
    pub fn tick_line_width(&self) -> f32 {
        self.theme.line_width * 0.4
    }

    /// Get edge width for filled shapes (patch.linewidth equivalent)
    ///
    /// Returns 0.8 by default (matplotlib's patch.linewidth).
    #[inline]
    pub fn patch_line_width(&self, config_override: Option<f32>) -> f32 {
        config_override.unwrap_or(0.8)
    }

    /// Resolve a color from the theme palette
    ///
    /// # Arguments
    /// * `explicit` - Explicit color, or None to get from palette
    /// * `series_index` - Index for palette cycling
    #[inline]
    pub fn series_color(&self, explicit: Option<Color>, series_index: usize) -> Color {
        explicit.unwrap_or_else(|| self.theme.get_color(series_index))
    }

    /// Get marker size
    ///
    /// # Arguments
    /// * `config_override` - Explicit marker size, or None for default
    ///
    /// # Returns
    /// Marker size in points (default: 6.0, matching matplotlib)
    #[inline]
    pub fn marker_size(&self, config_override: Option<f32>) -> f32 {
        config_override.unwrap_or(6.0)
    }

    /// Get tick label font size from theme
    #[inline]
    pub fn tick_label_font_size(&self) -> f32 {
        self.theme.tick_label_font_size
    }

    /// Get axis label font size from theme
    #[inline]
    pub fn axis_label_font_size(&self) -> f32 {
        self.theme.axis_label_font_size
    }

    /// Get foreground color from theme (for text, axes, etc.)
    #[inline]
    pub fn foreground(&self) -> Color {
        self.theme.foreground
    }

    /// Get background color from theme
    #[inline]
    pub fn background(&self) -> Color {
        self.theme.background
    }

    /// Get grid color from theme
    #[inline]
    pub fn grid_color(&self) -> Color {
        self.theme.grid_color
    }
}

/// Default fill alpha values matching matplotlib/seaborn
pub mod defaults {
    /// Default fill alpha for violin plots (seaborn: 0.6)
    pub const VIOLIN_FILL_ALPHA: f32 = 0.6;

    /// Default fill alpha for box plots (matplotlib: 0.7)
    pub const BOXPLOT_FILL_ALPHA: f32 = 0.7;

    /// Default fill alpha for histograms (matplotlib: 1.0)
    pub const HISTOGRAM_FILL_ALPHA: f32 = 1.0;

    /// Default fill alpha for KDE fill (seaborn: 0.3)
    pub const KDE_FILL_ALPHA: f32 = 0.3;

    /// Default fill alpha for bar charts (matplotlib: 1.0)
    pub const BAR_FILL_ALPHA: f32 = 1.0;

    /// Default edge width (patch.linewidth in matplotlib: 0.8)
    pub const PATCH_LINE_WIDTH: f32 = 0.8;

    /// Default bar width for histograms (0-1 range)
    pub const HISTOGRAM_BAR_WIDTH: f32 = 0.9;

    /// Default box width ratio for box plots
    pub const BOXPLOT_WIDTH_RATIO: f32 = 0.5;

    /// Default cap width as fraction of box width
    pub const BOXPLOT_CAP_WIDTH: f32 = 0.5;

    /// Default flier (outlier) marker size
    pub const FLIER_SIZE: f32 = 6.0;

    /// Default rug height as fraction of plot height
    pub const RUG_HEIGHT: f32 = 0.05;

    /// Default rug alpha
    pub const RUG_ALPHA: f32 = 0.5;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_resolver_creation() {
        let theme = Theme::light();
        let resolver = StyleResolver::new(&theme);
        assert_eq!(resolver.theme().line_width, theme.line_width);
    }

    #[test]
    fn test_line_width_override() {
        let theme = Theme::light();
        let resolver = StyleResolver::new(&theme);

        // With override
        assert_eq!(resolver.line_width(Some(2.5)), 2.5);

        // Without override (use theme default)
        assert_eq!(resolver.line_width(None), theme.line_width);
    }

    #[test]
    fn test_fill_alpha() {
        let theme = Theme::light();
        let resolver = StyleResolver::new(&theme);

        // With override
        assert_eq!(resolver.fill_alpha(Some(0.5), 1.0), 0.5);

        // Without override
        assert_eq!(resolver.fill_alpha(None, 0.7), 0.7);

        // Clamping
        assert_eq!(resolver.fill_alpha(Some(1.5), 1.0), 1.0);
        assert_eq!(resolver.fill_alpha(Some(-0.5), 1.0), 0.0);
    }

    #[test]
    fn test_font_size_scaling() {
        let theme = Theme::light();
        let resolver = StyleResolver::new(&theme);

        assert_eq!(resolver.font_size(1.0), theme.font_size);
        assert_eq!(resolver.font_size(1.4), theme.font_size * 1.4);
        assert_eq!(resolver.font_size(0.8), theme.font_size * 0.8);
    }

    #[test]
    fn test_edge_color_explicit() {
        let theme = Theme::light();
        let resolver = StyleResolver::new(&theme);

        let fill = Color::BLUE;
        let explicit = Color::RED;

        // With explicit color
        assert_eq!(resolver.edge_color(fill, Some(explicit)), explicit);
    }

    #[test]
    fn test_edge_color_auto_darken() {
        let theme = Theme::light();
        let resolver = StyleResolver::new(&theme);

        let fill = Color::new(100, 150, 200);

        // Auto-derived (30% darker)
        let edge = resolver.edge_color(fill, None);
        assert_eq!(edge.r, 70); // 100 * 0.7
        assert_eq!(edge.g, 105); // 150 * 0.7
        assert_eq!(edge.b, 140); // 200 * 0.7
    }

    #[test]
    fn test_edge_color_custom_factor() {
        let theme = Theme::light();
        let resolver = StyleResolver::new(&theme);

        let fill = Color::new(100, 150, 200);

        // 50% darker
        let edge = resolver.edge_color_with_factor(fill, None, 0.5);
        assert_eq!(edge.r, 50); // 100 * 0.5
        assert_eq!(edge.g, 75); // 150 * 0.5
        assert_eq!(edge.b, 100); // 200 * 0.5
    }

    #[test]
    fn test_grid_and_axis_widths() {
        let theme = Theme::light();
        let resolver = StyleResolver::new(&theme);

        assert_eq!(resolver.grid_line_width(), theme.line_width * 0.5);
        assert_eq!(resolver.axis_line_width(), theme.line_width * 0.5);
        assert_eq!(resolver.tick_line_width(), theme.line_width * 0.4);
    }

    #[test]
    fn test_patch_line_width() {
        let theme = Theme::light();
        let resolver = StyleResolver::new(&theme);

        // Default is 0.8 (matplotlib patch.linewidth)
        assert_eq!(resolver.patch_line_width(None), 0.8);

        // With override
        assert_eq!(resolver.patch_line_width(Some(1.5)), 1.5);
    }

    #[test]
    fn test_series_color() {
        let theme = Theme::light();
        let resolver = StyleResolver::new(&theme);

        // With explicit color
        assert_eq!(resolver.series_color(Some(Color::RED), 0), Color::RED);

        // From palette
        assert_eq!(resolver.series_color(None, 0), theme.get_color(0));
        assert_eq!(resolver.series_color(None, 1), theme.get_color(1));
    }

    #[test]
    fn test_marker_size() {
        let theme = Theme::light();
        let resolver = StyleResolver::new(&theme);

        // Default is 6.0 (matplotlib default)
        assert_eq!(resolver.marker_size(None), 6.0);

        // With override
        assert_eq!(resolver.marker_size(Some(10.0)), 10.0);
    }

    #[test]
    fn test_theme_colors() {
        let theme = Theme::dark();
        let resolver = StyleResolver::new(&theme);

        assert_eq!(resolver.foreground(), theme.foreground);
        assert_eq!(resolver.background(), theme.background);
        assert_eq!(resolver.grid_color(), theme.grid_color);
    }

    #[test]
    fn test_different_themes() {
        // Light theme
        let light = Theme::light();
        let light_resolver = StyleResolver::new(&light);

        // Dark theme
        let dark = Theme::dark();
        let dark_resolver = StyleResolver::new(&dark);

        // Presentation theme (larger line widths)
        let presentation = Theme::presentation();
        let presentation_resolver = StyleResolver::new(&presentation);

        // Line widths should differ
        assert!(presentation_resolver.line_width(None) > light_resolver.line_width(None));

        // Colors should differ
        assert_ne!(light_resolver.background(), dark_resolver.background());
    }
}
