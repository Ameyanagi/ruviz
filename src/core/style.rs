//! Plot style presets for common use cases
//!
//! This module provides named style presets that configure all visual parameters
//! for specific use cases like publications, presentations, or web display.
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::core::{PlotStyle, PlotConfig};
//!
//! // Get configuration for publication style
//! let config = PlotStyle::Publication.config();
//!
//! // Or use with Plot directly
//! Plot::new()
//!     .style(PlotStyle::Presentation)
//!     .line(&x, &y)
//!     .save("slides.png")?;
//! ```

use crate::core::config::{
    FigureConfig, LineConfig, MarginConfig, PlotConfig, SpacingConfig, SpineConfig,
    TypographyConfig,
};
use crate::render::{FontFamily, FontWeight};

/// Plot style presets
///
/// Each preset configures typography, line widths, and spacing for
/// a specific use case.
#[allow(clippy::upper_case_acronyms)] // IEEE is the standard organization acronym
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlotStyle {
    /// Matplotlib-like defaults
    ///
    /// - 6.4×4.8 inch figure
    /// - 10pt base font (sans-serif)
    /// - 1.5pt data lines
    /// - Good for general use
    #[default]
    Default,

    /// Minimal chrome, clean look
    ///
    /// - 6.4×4.8 inch figure
    /// - 9pt base font
    /// - 1.0pt data lines
    /// - Reduced spacing
    Minimal,

    /// Publication-ready style
    ///
    /// - 6.4×4.8 inch figure
    /// - 8pt base font (serif)
    /// - 1.0pt data lines
    /// - Compact margins
    /// - Suitable for journal column width
    Publication,

    /// IEEE journal format
    ///
    /// - 3.5×2.5 inch figure (single column)
    /// - 8pt base font (serif)
    /// - 0.75pt data lines
    /// - Very compact for dense layouts
    IEEE,

    /// Nature journal format
    ///
    /// - 3.5×3.0 inch figure
    /// - 7pt base font (sans-serif)
    /// - 0.75pt data lines
    /// - Clean and compact
    Nature,

    /// Large fonts and thick lines for slides
    ///
    /// - 10×7.5 inch figure
    /// - 14pt base font
    /// - 2.5pt data lines
    /// - Large spacing
    /// - Optimized for projector viewing
    Presentation,

    /// Dark background theme
    ///
    /// - 6.4×4.8 inch figure
    /// - 10pt base font
    /// - 1.5pt data lines
    /// - Standard spacing
    /// - (Note: colors are set separately via Theme)
    Dark,

    /// High contrast for accessibility
    ///
    /// - 6.4×4.8 inch figure
    /// - 12pt base font
    /// - 2.0pt data lines
    /// - Increased spacing
    /// - Optimized for visual accessibility
    HighContrast,

    /// Web-optimized style
    ///
    /// - 8×5 inch figure
    /// - 11pt base font
    /// - 1.5pt data lines
    /// - Good for screen viewing
    Web,

    /// Poster presentation style
    ///
    /// - 12×9 inch figure
    /// - 18pt base font
    /// - 3.0pt data lines
    /// - Very large for poster printing
    Poster,
}

impl PlotStyle {
    /// Get the PlotConfig for this style
    pub fn config(&self) -> PlotConfig {
        match self {
            PlotStyle::Default => PlotConfig::default(),

            PlotStyle::Minimal => PlotConfig {
                figure: FigureConfig::default(),
                typography: TypographyConfig {
                    base_size: 9.0,
                    title_scale: 1.2,
                    label_scale: 1.0,
                    tick_scale: 0.9,
                    legend_scale: 0.9,
                    family: FontFamily::SansSerif,
                    title_weight: FontWeight::Normal,
                },
                lines: LineConfig {
                    data_width: 1.0,
                    axis_width: 0.5,
                    grid_width: 0.3,
                    tick_width: 0.5,
                    tick_length: 3.0,
                },
                spacing: SpacingConfig {
                    title_pad: 8.0,
                    label_pad: 4.0,
                    tick_pad: 3.0,
                    legend_pad: 6.0,
                },
                margins: MarginConfig::Auto { min: 0.4, max: 1.0 },
                spines: SpineConfig::despine(), // Minimal style uses despine
            },

            PlotStyle::Publication => PlotConfig {
                figure: FigureConfig {
                    width: 6.4,
                    height: 4.8,
                    dpi: 150.0, // Higher DPI for print
                },
                typography: TypographyConfig {
                    base_size: 8.0,
                    title_scale: 1.25,
                    label_scale: 1.0,
                    tick_scale: 0.875,
                    legend_scale: 0.875,
                    family: FontFamily::Serif,
                    title_weight: FontWeight::Normal,
                },
                lines: LineConfig {
                    data_width: 1.0,
                    axis_width: 0.6,
                    grid_width: 0.4,
                    tick_width: 0.5,
                    tick_length: 3.5,
                },
                spacing: SpacingConfig {
                    title_pad: 8.0,
                    label_pad: 4.0,
                    tick_pad: 3.0,
                    legend_pad: 6.0,
                },
                margins: MarginConfig::Auto { min: 0.4, max: 0.9 },
                spines: SpineConfig::default(),
            },

            PlotStyle::IEEE => PlotConfig {
                figure: FigureConfig {
                    width: 3.5, // IEEE single column
                    height: 2.5,
                    dpi: 300.0, // High DPI for print
                },
                typography: TypographyConfig {
                    base_size: 8.0,
                    title_scale: 1.125,
                    label_scale: 1.0,
                    tick_scale: 0.875,
                    legend_scale: 0.75,
                    family: FontFamily::Serif,
                    title_weight: FontWeight::Normal,
                },
                lines: LineConfig {
                    data_width: 0.75,
                    axis_width: 0.5,
                    grid_width: 0.3,
                    tick_width: 0.4,
                    tick_length: 2.5,
                },
                spacing: SpacingConfig {
                    title_pad: 6.0,
                    label_pad: 3.0,
                    tick_pad: 2.0,
                    legend_pad: 4.0,
                },
                margins: MarginConfig::Auto {
                    min: 0.25,
                    max: 0.6,
                },
                spines: SpineConfig::default(),
            },

            PlotStyle::Nature => PlotConfig {
                figure: FigureConfig {
                    width: 3.5,
                    height: 3.0,
                    dpi: 300.0,
                },
                typography: TypographyConfig {
                    base_size: 7.0,
                    title_scale: 1.15,
                    label_scale: 1.0,
                    tick_scale: 0.85,
                    legend_scale: 0.85,
                    family: FontFamily::SansSerif,
                    title_weight: FontWeight::Normal,
                },
                lines: LineConfig {
                    data_width: 0.75,
                    axis_width: 0.5,
                    grid_width: 0.25,
                    tick_width: 0.4,
                    tick_length: 2.5,
                },
                spacing: SpacingConfig {
                    title_pad: 5.0,
                    label_pad: 3.0,
                    tick_pad: 2.0,
                    legend_pad: 4.0,
                },
                margins: MarginConfig::Auto { min: 0.2, max: 0.5 },
                spines: SpineConfig::default(),
            },

            PlotStyle::Presentation => PlotConfig {
                figure: FigureConfig {
                    width: 10.0,
                    height: 7.5,
                    dpi: 100.0,
                },
                typography: TypographyConfig {
                    base_size: 14.0,
                    title_scale: 1.4,
                    label_scale: 1.0,
                    tick_scale: 0.85,
                    legend_scale: 0.85,
                    family: FontFamily::SansSerif,
                    title_weight: FontWeight::Bold,
                },
                lines: LineConfig {
                    data_width: 2.5,
                    axis_width: 1.2,
                    grid_width: 0.8,
                    tick_width: 1.0,
                    tick_length: 6.0,
                },
                spacing: SpacingConfig {
                    title_pad: 18.0,
                    label_pad: 10.0,
                    tick_pad: 6.0,
                    legend_pad: 12.0,
                },
                margins: MarginConfig::Auto { min: 0.6, max: 1.5 },
                spines: SpineConfig::default(),
            },

            PlotStyle::Dark => PlotConfig {
                figure: FigureConfig::default(),
                typography: TypographyConfig::default(),
                lines: LineConfig::default(),
                spacing: SpacingConfig::default(),
                margins: MarginConfig::default(),
                spines: SpineConfig::default(),
                // Note: Dark colors are applied via Theme, not PlotConfig
            },

            PlotStyle::HighContrast => PlotConfig {
                figure: FigureConfig::default(),
                typography: TypographyConfig {
                    base_size: 12.0,
                    title_scale: 1.3,
                    label_scale: 1.0,
                    tick_scale: 0.9,
                    legend_scale: 0.9,
                    family: FontFamily::SansSerif,
                    title_weight: FontWeight::Bold,
                },
                lines: LineConfig {
                    data_width: 2.0,
                    axis_width: 1.0,
                    grid_width: 0.6,
                    tick_width: 0.8,
                    tick_length: 5.0,
                },
                spacing: SpacingConfig {
                    title_pad: 14.0,
                    label_pad: 8.0,
                    tick_pad: 5.0,
                    legend_pad: 10.0,
                },
                margins: MarginConfig::Auto { min: 0.5, max: 1.3 },
                spines: SpineConfig::default(),
            },

            PlotStyle::Web => PlotConfig {
                figure: FigureConfig {
                    width: 8.0,
                    height: 5.0,
                    dpi: 100.0,
                },
                typography: TypographyConfig {
                    base_size: 11.0,
                    title_scale: 1.35,
                    label_scale: 1.0,
                    tick_scale: 0.9,
                    legend_scale: 0.9,
                    family: FontFamily::SansSerif,
                    title_weight: FontWeight::Normal,
                },
                lines: LineConfig {
                    data_width: 1.5,
                    axis_width: 0.8,
                    grid_width: 0.5,
                    tick_width: 0.6,
                    tick_length: 4.0,
                },
                spacing: SpacingConfig::default(),
                margins: MarginConfig::default(),
                spines: SpineConfig::default(),
            },

            PlotStyle::Poster => PlotConfig {
                figure: FigureConfig {
                    width: 12.0,
                    height: 9.0,
                    dpi: 150.0,
                },
                typography: TypographyConfig {
                    base_size: 18.0,
                    title_scale: 1.5,
                    label_scale: 1.0,
                    tick_scale: 0.8,
                    legend_scale: 0.8,
                    family: FontFamily::SansSerif,
                    title_weight: FontWeight::Bold,
                },
                lines: LineConfig {
                    data_width: 3.0,
                    axis_width: 1.5,
                    grid_width: 1.0,
                    tick_width: 1.2,
                    tick_length: 8.0,
                },
                spacing: SpacingConfig {
                    title_pad: 24.0,
                    label_pad: 14.0,
                    tick_pad: 8.0,
                    legend_pad: 16.0,
                },
                margins: MarginConfig::Auto { min: 0.8, max: 2.0 },
                spines: SpineConfig::default(),
            },
        }
    }

    /// Get all available styles
    pub fn all() -> &'static [PlotStyle] {
        &[
            PlotStyle::Default,
            PlotStyle::Minimal,
            PlotStyle::Publication,
            PlotStyle::IEEE,
            PlotStyle::Nature,
            PlotStyle::Presentation,
            PlotStyle::Dark,
            PlotStyle::HighContrast,
            PlotStyle::Web,
            PlotStyle::Poster,
        ]
    }

    /// Get the name of this style
    pub fn name(&self) -> &'static str {
        match self {
            PlotStyle::Default => "Default",
            PlotStyle::Minimal => "Minimal",
            PlotStyle::Publication => "Publication",
            PlotStyle::IEEE => "IEEE",
            PlotStyle::Nature => "Nature",
            PlotStyle::Presentation => "Presentation",
            PlotStyle::Dark => "Dark",
            PlotStyle::HighContrast => "High Contrast",
            PlotStyle::Web => "Web",
            PlotStyle::Poster => "Poster",
        }
    }

    /// Get a description of this style
    pub fn description(&self) -> &'static str {
        match self {
            PlotStyle::Default => "Matplotlib-like defaults, good for general use",
            PlotStyle::Minimal => "Clean look with minimal chrome",
            PlotStyle::Publication => "Publication-ready with compact layout",
            PlotStyle::IEEE => "IEEE journal format (single column)",
            PlotStyle::Nature => "Nature journal format",
            PlotStyle::Presentation => "Large fonts for slides/projector",
            PlotStyle::Dark => "Dark background theme",
            PlotStyle::HighContrast => "High contrast for accessibility",
            PlotStyle::Web => "Optimized for web/screen viewing",
            PlotStyle::Poster => "Large format for poster presentations",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_style() {
        let config = PlotStyle::Default.config();
        assert!((config.figure.width - 6.4).abs() < 0.001);
        assert!((config.typography.base_size - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_publication_style() {
        let config = PlotStyle::Publication.config();
        assert!((config.typography.base_size - 8.0).abs() < 0.001);
        assert!(matches!(config.typography.family, FontFamily::Serif));
    }

    #[test]
    fn test_presentation_style() {
        let config = PlotStyle::Presentation.config();
        assert!((config.typography.base_size - 14.0).abs() < 0.001);
        assert!((config.figure.width - 10.0).abs() < 0.001);
        assert!((config.lines.data_width - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_all_styles_produce_valid_config() {
        for style in PlotStyle::all() {
            let config = style.config();
            // Basic sanity checks
            assert!(config.figure.width > 0.0);
            assert!(config.figure.height > 0.0);
            assert!(config.figure.dpi > 0.0);
            assert!(config.typography.base_size > 0.0);
            assert!(config.lines.data_width > 0.0);
        }
    }

    #[test]
    fn test_style_names() {
        assert_eq!(PlotStyle::Default.name(), "Default");
        assert_eq!(PlotStyle::Publication.name(), "Publication");
    }
}
