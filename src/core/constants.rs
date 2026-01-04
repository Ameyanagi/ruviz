//! Centralized constants for plot configuration
//!
//! This module provides named constants for commonly used values throughout the codebase,
//! eliminating magic numbers and ensuring consistency.

/// DPI (dots per inch) constants for different output contexts
pub mod dpi {
    /// Web/screen display (72 DPI - legacy web standard)
    pub const WEB: f64 = 72.0;

    /// Standard screen display (100 DPI)
    pub const SCREEN: f64 = 100.0;

    /// Print quality (150 DPI)
    pub const PRINT: f64 = 150.0;

    /// High-quality print (300 DPI - standard print quality)
    pub const HIGH_PRINT: f64 = 300.0;

    /// Maximum allowed DPI (2400 - professional print)
    pub const MAX: f64 = 2400.0;

    /// Minimum allowed DPI
    pub const MIN: f64 = 72.0;

    /// Default DPI for general use
    pub const DEFAULT: f64 = 100.0;
}

/// Default figure dimensions in inches (matplotlib standard)
pub mod dimensions {
    /// Default figure width in inches
    pub const WIDTH: f64 = 6.4;

    /// Default figure height in inches
    pub const HEIGHT: f64 = 4.8;

    /// IEEE single column width
    pub const IEEE_SINGLE_COLUMN: f64 = 3.5;

    /// IEEE double column width
    pub const IEEE_DOUBLE_COLUMN: f64 = 7.16;

    /// Nature journal figure width
    pub const NATURE_WIDTH: f64 = 3.5;

    /// Presentation slide width
    pub const PRESENTATION_WIDTH: f64 = 10.0;

    /// Presentation slide height
    pub const PRESENTATION_HEIGHT: f64 = 7.5;

    /// Poster width
    pub const POSTER_WIDTH: f64 = 12.0;

    /// Poster height
    pub const POSTER_HEIGHT: f64 = 9.0;
}

/// Font scaling factors relative to base font size
pub mod font_scales {
    /// Title font scale (larger than base)
    pub const TITLE: f64 = 1.2;

    /// Large title scale (for presentations)
    pub const TITLE_LARGE: f64 = 1.4;

    /// Compact title scale (for publications)
    pub const TITLE_COMPACT: f64 = 1.125;

    /// Axis label scale (same as base)
    pub const LABEL: f64 = 1.0;

    /// Tick label scale (slightly smaller)
    pub const TICK: f64 = 0.9;

    /// Tick label scale for compact layouts
    pub const TICK_COMPACT: f64 = 0.875;

    /// Legend font scale
    pub const LEGEND: f64 = 0.9;

    /// Legend font scale for compact layouts
    pub const LEGEND_COMPACT: f64 = 0.75;
}

/// Line width presets in points
pub mod line_widths {
    /// Thin line (grid, minor elements)
    pub const THIN: f32 = 0.5;

    /// Normal data line
    pub const NORMAL: f32 = 1.0;

    /// Medium line
    pub const MEDIUM: f32 = 1.5;

    /// Thick line (emphasis, presentations)
    pub const THICK: f32 = 2.0;

    /// Extra thick line (posters)
    pub const EXTRA_THICK: f32 = 3.0;

    /// Axis line width
    pub const AXIS: f32 = 0.8;

    /// Grid line width
    pub const GRID: f32 = 0.4;

    /// Tick mark width
    pub const TICK: f32 = 0.5;

    /// Publication data line
    pub const PUBLICATION: f32 = 0.75;

    /// Presentation data line
    pub const PRESENTATION: f32 = 2.5;
}

/// Base font sizes in points for different contexts
pub mod font_sizes {
    /// Default base font size
    pub const DEFAULT: f32 = 10.0;

    /// Minimal style base font
    pub const MINIMAL: f32 = 9.0;

    /// Publication style base font
    pub const PUBLICATION: f32 = 8.0;

    /// Nature journal base font
    pub const NATURE: f32 = 7.0;

    /// Presentation base font
    pub const PRESENTATION: f32 = 14.0;

    /// High contrast/accessibility base font
    pub const HIGH_CONTRAST: f32 = 12.0;

    /// Web display base font
    pub const WEB: f32 = 11.0;

    /// Poster base font
    pub const POSTER: f32 = 18.0;
}

/// Spacing presets in points
pub mod spacing {
    /// Title padding from plot area
    pub const TITLE_PAD: f32 = 12.0;

    /// Compact title padding
    pub const TITLE_PAD_COMPACT: f32 = 6.0;

    /// Large title padding (presentations)
    pub const TITLE_PAD_LARGE: f32 = 18.0;

    /// Axis label padding
    pub const LABEL_PAD: f32 = 6.0;

    /// Tick label padding
    pub const TICK_PAD: f32 = 4.0;

    /// Legend padding
    pub const LEGEND_PAD: f32 = 8.0;

    /// Default tick length
    pub const TICK_LENGTH: f32 = 4.0;
}

/// Margin presets as fractions of figure size
pub mod margins {
    /// Minimum auto margin
    pub const AUTO_MIN: f32 = 0.5;

    /// Maximum auto margin
    pub const AUTO_MAX: f32 = 1.2;

    /// Compact minimum margin
    pub const COMPACT_MIN: f32 = 0.25;

    /// Compact maximum margin
    pub const COMPACT_MAX: f32 = 0.6;

    /// Large margin minimum (presentations)
    pub const LARGE_MIN: f32 = 0.6;

    /// Large margin maximum
    pub const LARGE_MAX: f32 = 1.5;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpi_ordering() {
        assert!(dpi::WEB < dpi::SCREEN);
        assert!(dpi::SCREEN < dpi::PRINT);
        assert!(dpi::PRINT < dpi::HIGH_PRINT);
        assert!(dpi::HIGH_PRINT < dpi::MAX);
    }

    #[test]
    fn test_font_scales_ordering() {
        assert!(font_scales::TICK < font_scales::LABEL);
        assert!(font_scales::LABEL < font_scales::TITLE);
    }

    #[test]
    fn test_line_widths_ordering() {
        assert!(line_widths::THIN < line_widths::NORMAL);
        assert!(line_widths::NORMAL < line_widths::MEDIUM);
        assert!(line_widths::MEDIUM < line_widths::THICK);
    }
}
