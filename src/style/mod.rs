//! Unified style module for consistent theming and styling across all plot types
//!
//! This module consolidates all styling-related types and utilities that were
//! previously scattered across `render` and `core` modules. It provides a
//! single import point for all styling needs.
//!
//! # Organization
//!
//! The style module re-exports types from their original locations for
//! backward compatibility, while providing a unified namespace:
//!
//! - **Colors**: [`Color`] - RGB/RGBA color representation
//! - **Themes**: [`Theme`] - Complete plot theming
//! - **Line styles**: [`LineStyle`] - Solid, dashed, dotted, etc.
//! - **Marker styles**: [`MarkerStyle`] - Circle, square, triangle, etc.
//! - **Grid styles**: [`GridStyle`] - Grid line configuration
//! - **Plot styles**: [`PlotStyle`] - High-level style presets
//! - **Style resolution**: [`StyleResolver`] - Theme-aware style resolution
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::style::{Color, Theme, LineStyle, MarkerStyle, GridStyle};
//!
//! // Create a custom theme
//! let theme = Theme::dark();
//!
//! // Use style presets
//! let style = PlotStyle::Publication;
//!
//! // Configure grid appearance
//! let grid = GridStyle::default()
//!     .color(Color::LIGHT_GRAY)
//!     .alpha(0.3);
//! ```
//!
//! # Migration from Previous Imports
//!
//! Previous import paths continue to work:
//! ```rust,ignore
//! // Old paths (still work)
//! use ruviz::render::{Color, Theme, LineStyle, MarkerStyle};
//! use ruviz::core::{GridStyle, PlotStyle, StyleResolver};
//!
//! // New unified path (recommended)
//! use ruviz::style::{Color, Theme, LineStyle, MarkerStyle, GridStyle, PlotStyle, StyleResolver};
//! ```

// Re-export from render module
pub use crate::render::color::Color;
pub use crate::render::style::{LineStyle, MarkerStyle};
pub use crate::render::theme::Theme;

// Re-export from core module
pub use crate::core::grid_style::GridStyle;
pub use crate::core::style::PlotStyle;
pub use crate::core::style_utils::{StyleResolver, defaults};

// Re-export font types for styling
pub use crate::render::text::{FontFamily, FontWeight};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_reexport() {
        let color = Color::new(255, 128, 64);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
    }

    #[test]
    fn test_theme_reexport() {
        let light = Theme::light();
        let dark = Theme::dark();
        assert_ne!(light.background, dark.background);
    }

    #[test]
    fn test_line_style_reexport() {
        let solid = LineStyle::Solid;
        let dashed = LineStyle::Dashed;
        assert!(solid.to_dash_array().is_none());
        assert!(dashed.to_dash_array().is_some());
    }

    #[test]
    fn test_marker_style_reexport() {
        let circle = MarkerStyle::Circle;
        let square = MarkerStyle::Square;
        assert_ne!(format!("{:?}", circle), format!("{:?}", square));
    }

    #[test]
    fn test_grid_style_reexport() {
        let grid = GridStyle::default();
        assert!(grid.visible);

        let hidden = GridStyle::hidden();
        assert!(!hidden.visible);
    }

    #[test]
    fn test_plot_style_reexport() {
        let default = PlotStyle::Default;
        let publication = PlotStyle::Publication;
        assert_ne!(default.config().figure.dpi, publication.config().figure.dpi);
    }

    #[test]
    fn test_style_resolver_reexport() {
        let theme = Theme::light();
        let resolver = StyleResolver::new(&theme);
        assert_eq!(resolver.line_width(None), theme.line_width);
    }

    #[test]
    fn test_defaults_reexport() {
        // Check that defaults module is accessible
        assert!(defaults::VIOLIN_FILL_ALPHA > 0.0);
        assert!(defaults::BOXPLOT_FILL_ALPHA > 0.0);
    }
}
