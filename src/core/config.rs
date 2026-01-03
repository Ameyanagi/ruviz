//! Plot configuration system for DPI-independent rendering
//!
//! This module provides a comprehensive configuration system that uses physical
//! units (inches for dimensions, points for typography) to ensure consistent
//! proportions regardless of output DPI.
//!
//! # Design Philosophy
//!
//! - **Figure size** is in inches (like matplotlib's `figsize`)
//! - **Typography** is in points (1 point = 1/72 inch)
//! - **DPI** only affects output resolution, not proportions
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::core::{PlotConfig, FigureConfig, TypographyConfig};
//!
//! // Using defaults (matplotlib-compatible)
//! let config = PlotConfig::default();
//!
//! // Custom configuration
//! let config = PlotConfig::builder()
//!     .figure(8.0, 6.0)
//!     .dpi(300.0)
//!     .typography(|t| t.base_size(12.0))
//!     .build();
//! ```

use crate::core::units::{REFERENCE_DPI, in_to_px, pt_to_px, px_to_in};
use crate::render::{FontFamily, FontWeight};

// =============================================================================
// Figure Configuration
// =============================================================================

/// Figure size and resolution configuration
///
/// Dimensions are specified in inches for DPI-independent layout.
/// The DPI setting only affects the output pixel resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct FigureConfig {
    /// Figure width in inches (default: 6.4)
    pub width: f32,
    /// Figure height in inches (default: 4.8)
    pub height: f32,
    /// Output resolution in pixels per inch (default: 100)
    pub dpi: f32,
}

impl FigureConfig {
    /// Create a new figure configuration
    pub fn new(width: f32, height: f32, dpi: f32) -> Self {
        Self { width, height, dpi }
    }

    /// Calculate canvas size in pixels
    ///
    /// # Returns
    ///
    /// `(width_px, height_px)` tuple
    pub fn canvas_size(&self) -> (u32, u32) {
        (
            in_to_px(self.width, self.dpi) as u32,
            in_to_px(self.height, self.dpi) as u32,
        )
    }

    /// Get aspect ratio (width / height)
    pub fn aspect_ratio(&self) -> f32 {
        self.width / self.height
    }
}

impl Default for FigureConfig {
    fn default() -> Self {
        Self {
            width: 6.4,  // matplotlib default
            height: 4.8, // matplotlib default (4:3 aspect)
            dpi: 100.0,  // Good for screen
        }
    }
}

// =============================================================================
// Typography Configuration
// =============================================================================

/// Typography configuration with font sizes in points
///
/// Font sizes can be specified as absolute values or as scale factors
/// relative to `base_size`. Scale factors are multipliers (e.g., 1.4 = 140% of base).
#[derive(Debug, Clone, PartialEq)]
pub struct TypographyConfig {
    /// Base font size in points (default: 10.0)
    pub base_size: f32,
    /// Title size scale factor (default: 1.4)
    pub title_scale: f32,
    /// Axis label size scale factor (default: 1.0)
    pub label_scale: f32,
    /// Tick label size scale factor (default: 0.9)
    pub tick_scale: f32,
    /// Legend text size scale factor (default: 0.9)
    pub legend_scale: f32,
    /// Font family (default: SansSerif)
    pub family: FontFamily,
    /// Title font weight (default: Normal)
    pub title_weight: FontWeight,
}

impl TypographyConfig {
    /// Create a new typography configuration with base size
    pub fn new(base_size: f32) -> Self {
        Self {
            base_size,
            ..Default::default()
        }
    }

    /// Get title font size in points
    pub fn title_size(&self) -> f32 {
        self.base_size * self.title_scale
    }

    /// Get axis label font size in points
    pub fn label_size(&self) -> f32 {
        self.base_size * self.label_scale
    }

    /// Get tick label font size in points
    pub fn tick_size(&self) -> f32 {
        self.base_size * self.tick_scale
    }

    /// Get legend font size in points
    pub fn legend_size(&self) -> f32 {
        self.base_size * self.legend_scale
    }

    /// Get title font size in pixels at the given DPI
    pub fn title_size_px(&self, dpi: f32) -> f32 {
        pt_to_px(self.title_size(), dpi)
    }

    /// Get axis label font size in pixels at the given DPI
    pub fn label_size_px(&self, dpi: f32) -> f32 {
        pt_to_px(self.label_size(), dpi)
    }

    /// Get tick label font size in pixels at the given DPI
    pub fn tick_size_px(&self, dpi: f32) -> f32 {
        pt_to_px(self.tick_size(), dpi)
    }

    /// Get legend font size in pixels at the given DPI
    pub fn legend_size_px(&self, dpi: f32) -> f32 {
        pt_to_px(self.legend_size(), dpi)
    }

    /// Scale all font sizes proportionally
    pub fn scale(mut self, factor: f32) -> Self {
        self.base_size *= factor;
        self
    }

    /// Set the base font size
    pub fn base_size(mut self, size: f32) -> Self {
        self.base_size = size;
        self
    }

    /// Set the title scale factor
    pub fn title_scale(mut self, scale: f32) -> Self {
        self.title_scale = scale;
        self
    }

    /// Set the font family
    pub fn family(mut self, family: FontFamily) -> Self {
        self.family = family;
        self
    }

    /// Set the title font weight
    pub fn title_weight(mut self, weight: FontWeight) -> Self {
        self.title_weight = weight;
        self
    }
}

impl Default for TypographyConfig {
    fn default() -> Self {
        Self {
            base_size: 10.0,   // matplotlib default
            title_scale: 1.4,  // 14pt title
            label_scale: 1.0,  // 10pt labels
            tick_scale: 0.9,   // 9pt tick labels
            legend_scale: 0.9, // 9pt legend
            family: FontFamily::SansSerif,
            title_weight: FontWeight::Normal,
        }
    }
}

// =============================================================================
// Line Configuration
// =============================================================================

/// Line width configuration in points
///
/// All line widths are specified in typographic points (1/72 inch).
#[derive(Debug, Clone, PartialEq)]
pub struct LineConfig {
    /// Data line width in points (default: 1.5)
    pub data_width: f32,
    /// Axis border width in points (default: 0.8)
    pub axis_width: f32,
    /// Grid line width in points (default: 0.5)
    pub grid_width: f32,
    /// Tick mark width in points (default: 0.6)
    pub tick_width: f32,
    /// Tick mark length in points (default: 4.0)
    pub tick_length: f32,
}

impl LineConfig {
    /// Get data line width in pixels at the given DPI
    pub fn data_width_px(&self, dpi: f32) -> f32 {
        pt_to_px(self.data_width, dpi)
    }

    /// Get axis border width in pixels at the given DPI
    pub fn axis_width_px(&self, dpi: f32) -> f32 {
        pt_to_px(self.axis_width, dpi)
    }

    /// Get grid line width in pixels at the given DPI
    pub fn grid_width_px(&self, dpi: f32) -> f32 {
        pt_to_px(self.grid_width, dpi)
    }

    /// Get tick mark width in pixels at the given DPI
    pub fn tick_width_px(&self, dpi: f32) -> f32 {
        pt_to_px(self.tick_width, dpi)
    }

    /// Get tick mark length in pixels at the given DPI
    pub fn tick_length_px(&self, dpi: f32) -> f32 {
        pt_to_px(self.tick_length, dpi)
    }

    /// Set data line width
    pub fn data_width(mut self, width: f32) -> Self {
        self.data_width = width;
        self
    }

    /// Set axis border width
    pub fn axis_width(mut self, width: f32) -> Self {
        self.axis_width = width;
        self
    }

    /// Set grid line width
    pub fn grid_width(mut self, width: f32) -> Self {
        self.grid_width = width;
        self
    }
}

impl Default for LineConfig {
    fn default() -> Self {
        Self {
            data_width: 1.5, // matplotlib default
            axis_width: 0.8,
            grid_width: 0.5,
            tick_width: 0.6,
            tick_length: 4.0,
        }
    }
}

// =============================================================================
// Spacing Configuration
// =============================================================================

/// Spacing and padding configuration in points
///
/// All spacing values are in typographic points.
#[derive(Debug, Clone, PartialEq)]
pub struct SpacingConfig {
    /// Space below title in points (default: 12.0)
    pub title_pad: f32,
    /// Space between axis and label in points (default: 6.0)
    pub label_pad: f32,
    /// Space between axis and tick labels in points (default: 4.0)
    pub tick_pad: f32,
    /// Padding inside legend box in points (default: 8.0)
    pub legend_pad: f32,
}

impl SpacingConfig {
    /// Get title padding in pixels at the given DPI
    pub fn title_pad_px(&self, dpi: f32) -> f32 {
        pt_to_px(self.title_pad, dpi)
    }

    /// Get label padding in pixels at the given DPI
    pub fn label_pad_px(&self, dpi: f32) -> f32 {
        pt_to_px(self.label_pad, dpi)
    }

    /// Get tick padding in pixels at the given DPI
    pub fn tick_pad_px(&self, dpi: f32) -> f32 {
        pt_to_px(self.tick_pad, dpi)
    }

    /// Get legend padding in pixels at the given DPI
    pub fn legend_pad_px(&self, dpi: f32) -> f32 {
        pt_to_px(self.legend_pad, dpi)
    }
}

impl Default for SpacingConfig {
    fn default() -> Self {
        Self {
            title_pad: 12.0,  // ~0.75x title font size - professional spacing below title
            label_pad: 14.0,  // ~1x label font size - space between ylabel and tick labels
            tick_pad: 6.0,    // ~0.5x tick font size - space between tick labels and axis
            legend_pad: 8.0,
        }
    }
}

// =============================================================================
// Margin Configuration
// =============================================================================

/// Margin configuration for plot area
///
/// Margins can be proportional (matplotlib-style), auto-calculated, or fixed.
///
/// # Margin Modes
///
/// - **Proportional** (default): Margins as fractions of figure size, matching matplotlib's
///   `subplotpars` behavior. This ensures consistent visual proportions across different
///   figure sizes.
///
/// - **Auto**: Dynamically calculated based on content (title, labels). Use when you need
///   margins to adapt to varying text content.
///
/// - **Fixed**: Explicit margins in inches. Use for precise control over layout.
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::core::MarginConfig;
///
/// // Default: matplotlib-compatible proportional margins
/// let proportional = MarginConfig::default();
///
/// // Content-aware auto margins
/// let auto = MarginConfig::auto();
///
/// // Explicit fixed margins
/// let fixed = MarginConfig::fixed(1.0, 0.5, 0.8, 0.6);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum MarginConfig {
    /// Proportional margins as fractions of figure dimensions (matplotlib-style)
    ///
    /// This is the default mode, matching matplotlib's `subplotpars`:
    /// - `left`: fraction of figure width for left margin (default: 0.125 = 12.5%)
    /// - `right`: fraction of figure width for right margin (default: 0.1 = 10%)
    /// - `top`: fraction of figure height for top margin (default: 0.12 = 12%)
    /// - `bottom`: fraction of figure height for bottom margin (default: 0.11 = 11%)
    Proportional {
        /// Left margin as fraction of figure width
        left: f32,
        /// Right margin as fraction of figure width
        right: f32,
        /// Top margin as fraction of figure height
        top: f32,
        /// Bottom margin as fraction of figure height
        bottom: f32,
    },
    /// Auto-calculate margins based on content
    Auto {
        /// Minimum margin in inches
        min: f32,
        /// Maximum margin in inches
        max: f32,
    },
    /// Fixed margins in inches
    Fixed {
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
    },
    /// Content-driven margins computed from text measurements
    ///
    /// This is the recommended mode for well-balanced layouts.
    /// Margins are calculated based on actual content (title, labels, ticks)
    /// rather than arbitrary percentages.
    ContentDriven {
        /// Small buffer from canvas edges (in points)
        edge_buffer: f32,
        /// Whether to center the plot by distributing extra space
        center_plot: bool,
    },
}

impl MarginConfig {
    /// Create proportional margins with matplotlib defaults
    ///
    /// This matches matplotlib's default `subplotpars`:
    /// - left: 0.125 (12.5% of width)
    /// - right: 0.1 (10% of width)
    /// - top: 0.12 (12% of height)
    /// - bottom: 0.11 (11% of height)
    pub fn proportional() -> Self {
        MarginConfig::Proportional {
            left: 0.125,
            right: 0.1,
            top: 0.12,
            bottom: 0.11,
        }
    }

    /// Alias for `proportional()` - creates matplotlib-compatible margins
    pub fn matplotlib() -> Self {
        Self::proportional()
    }

    /// Create proportional margins with custom fractions
    pub fn proportional_custom(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        MarginConfig::Proportional {
            left,
            right,
            top,
            bottom,
        }
    }

    /// Create auto margins with default bounds (0.3 to 1.0 inches)
    ///
    /// Auto margins calculate space based on content (title, axis labels).
    /// Use this when you need margins to adapt to varying text content.
    pub fn auto() -> Self {
        MarginConfig::Auto { min: 0.3, max: 1.0 }
    }

    /// Create auto margins with custom bounds
    pub fn auto_with_bounds(min: f32, max: f32) -> Self {
        MarginConfig::Auto { min, max }
    }

    /// Create fixed margins (same on all sides)
    pub fn fixed_uniform(margin: f32) -> Self {
        MarginConfig::Fixed {
            left: margin,
            right: margin,
            top: margin,
            bottom: margin,
        }
    }

    /// Create fixed margins with different values per side (in inches)
    pub fn fixed(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        MarginConfig::Fixed {
            left,
            right,
            top,
            bottom,
        }
    }

    /// Create content-driven margins with default settings
    ///
    /// This is the recommended mode for well-balanced layouts.
    /// Uses a 5pt edge buffer and centers the plot.
    pub fn content_driven() -> Self {
        MarginConfig::ContentDriven {
            edge_buffer: 5.0,
            center_plot: true,
        }
    }

    /// Create content-driven margins with custom settings
    pub fn content_driven_custom(edge_buffer: f32, center_plot: bool) -> Self {
        MarginConfig::ContentDriven {
            edge_buffer,
            center_plot,
        }
    }
}

impl Default for MarginConfig {
    fn default() -> Self {
        // Default to content-driven margins - computes margins from actual content
        // This provides the best layout by measuring text elements first
        MarginConfig::ContentDriven {
            edge_buffer: 8.0,  // 8pt buffer from canvas edges - professional margin
            center_plot: true, // Center the chart area horizontally
        }
    }
}

/// Computed margin values in inches
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedMargins {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl ComputedMargins {
    /// Get left margin in pixels at the given DPI
    pub fn left_px(&self, dpi: f32) -> f32 {
        in_to_px(self.left, dpi)
    }

    /// Get right margin in pixels at the given DPI
    pub fn right_px(&self, dpi: f32) -> f32 {
        in_to_px(self.right, dpi)
    }

    /// Get top margin in pixels at the given DPI
    pub fn top_px(&self, dpi: f32) -> f32 {
        in_to_px(self.top, dpi)
    }

    /// Get bottom margin in pixels at the given DPI
    pub fn bottom_px(&self, dpi: f32) -> f32 {
        in_to_px(self.bottom, dpi)
    }
}

impl Default for ComputedMargins {
    fn default() -> Self {
        Self {
            left: 0.8,
            right: 0.3,
            top: 0.5,
            bottom: 0.6,
        }
    }
}

// =============================================================================
// Main Plot Configuration
// =============================================================================

/// Complete plot configuration
///
/// This struct aggregates all configuration options for a plot,
/// using physical units for DPI-independent layout.
#[derive(Debug, Clone)]
pub struct PlotConfig {
    /// Figure dimensions and DPI
    pub figure: FigureConfig,
    /// Typography settings
    pub typography: TypographyConfig,
    /// Line width settings
    pub lines: LineConfig,
    /// Spacing/padding settings
    pub spacing: SpacingConfig,
    /// Margin settings
    pub margins: MarginConfig,
}

impl PlotConfig {
    /// Create a new plot configuration builder
    pub fn builder() -> PlotConfigBuilder {
        PlotConfigBuilder::default()
    }

    /// Get canvas size in pixels
    pub fn canvas_size(&self) -> (u32, u32) {
        self.figure.canvas_size()
    }

    /// Get the DPI setting
    pub fn dpi(&self) -> f32 {
        self.figure.dpi
    }

    /// Compute margins based on configuration mode
    ///
    /// - **Proportional**: Returns margins as fractions of figure dimensions (in inches)
    /// - **Auto**: Estimates required space based on typography and content
    /// - **Fixed**: Returns the fixed values directly
    pub fn compute_margins(
        &self,
        has_title: bool,
        has_xlabel: bool,
        has_ylabel: bool,
    ) -> ComputedMargins {
        match &self.margins {
            MarginConfig::Proportional {
                left,
                right,
                top,
                bottom,
            } => {
                // Convert fractions to inches based on figure dimensions
                ComputedMargins {
                    left: self.figure.width * left,
                    right: self.figure.width * right,
                    top: self.figure.height * top,
                    bottom: self.figure.height * bottom,
                }
            }
            MarginConfig::Auto { min, max } => {
                let pt_to_in = |pt: f32| pt / 72.0;

                // Estimate top margin based on title
                let top = if has_title {
                    (pt_to_in(self.typography.title_size())
                        + pt_to_in(self.spacing.title_pad)
                        + 0.15)
                        .clamp(*min, *max)
                } else {
                    *min
                };

                // Estimate bottom margin based on xlabel and tick labels
                let bottom = if has_xlabel {
                    (pt_to_in(self.typography.label_size())
                        + pt_to_in(self.typography.tick_size())
                        + pt_to_in(self.spacing.label_pad)
                        + pt_to_in(self.spacing.tick_pad)
                        + 0.1)
                        .clamp(*min, *max)
                } else {
                    (pt_to_in(self.typography.tick_size()) + pt_to_in(self.spacing.tick_pad) + 0.1)
                        .clamp(*min, *max)
                };

                // Estimate left margin based on ylabel and y-tick labels
                let left = if has_ylabel {
                    // Rotated ylabel takes more horizontal space
                    (pt_to_in(self.typography.label_size())
                        + pt_to_in(self.typography.tick_size()) * 4.0 // Estimate tick label width
                        + pt_to_in(self.spacing.label_pad)
                        + 0.2)
                        .clamp(*min, *max)
                } else {
                    (pt_to_in(self.typography.tick_size()) * 4.0 + 0.15).clamp(*min, *max)
                };

                // Right margin is typically smaller
                let right = (*min).max(0.2);

                ComputedMargins {
                    left,
                    right,
                    top,
                    bottom,
                }
            }
            MarginConfig::Fixed {
                left,
                right,
                top,
                bottom,
            } => ComputedMargins {
                left: *left,
                right: *right,
                top: *top,
                bottom: *bottom,
            },
            MarginConfig::ContentDriven { .. } => {
                // ContentDriven mode doesn't use ComputedMargins directly.
                // The layout is computed by LayoutCalculator.
                // Return reasonable defaults for fallback.
                ComputedMargins {
                    left: 0.8,
                    right: 0.3,
                    top: 0.5,
                    bottom: 0.6,
                }
            }
        }
    }

    /// Create a config from pixel dimensions (convenience method)
    ///
    /// Converts pixel dimensions to inches using the reference DPI (100).
    pub fn from_pixels(width_px: u32, height_px: u32) -> Self {
        Self {
            figure: FigureConfig {
                width: px_to_in(width_px as f32, REFERENCE_DPI),
                height: px_to_in(height_px as f32, REFERENCE_DPI),
                dpi: REFERENCE_DPI,
            },
            ..Default::default()
        }
    }
}

impl Default for PlotConfig {
    fn default() -> Self {
        Self {
            figure: FigureConfig::default(),
            typography: TypographyConfig::default(),
            lines: LineConfig::default(),
            spacing: SpacingConfig::default(),
            margins: MarginConfig::default(),
        }
    }
}

// =============================================================================
// Builder Pattern
// =============================================================================

/// Builder for PlotConfig with fluent API
#[derive(Debug, Clone, Default)]
pub struct PlotConfigBuilder {
    config: PlotConfig,
}

impl PlotConfigBuilder {
    /// Set figure dimensions in inches
    pub fn figure(mut self, width: f32, height: f32) -> Self {
        self.config.figure.width = width;
        self.config.figure.height = height;
        self
    }

    /// Set figure dimensions from pixels (at reference DPI)
    pub fn figure_px(mut self, width_px: u32, height_px: u32) -> Self {
        self.config.figure.width = px_to_in(width_px as f32, REFERENCE_DPI);
        self.config.figure.height = px_to_in(height_px as f32, REFERENCE_DPI);
        self
    }

    /// Set output DPI
    pub fn dpi(mut self, dpi: f32) -> Self {
        self.config.figure.dpi = dpi;
        self
    }

    /// Configure typography with a closure
    pub fn typography<F>(mut self, f: F) -> Self
    where
        F: FnOnce(TypographyConfig) -> TypographyConfig,
    {
        self.config.typography = f(self.config.typography);
        self
    }

    /// Set base font size in points
    pub fn font_size(mut self, size: f32) -> Self {
        self.config.typography.base_size = size;
        self
    }

    /// Configure line widths with a closure
    pub fn lines<F>(mut self, f: F) -> Self
    where
        F: FnOnce(LineConfig) -> LineConfig,
    {
        self.config.lines = f(self.config.lines);
        self
    }

    /// Set data line width in points
    pub fn line_width(mut self, width: f32) -> Self {
        self.config.lines.data_width = width;
        self
    }

    /// Configure spacing with a closure
    pub fn spacing<F>(mut self, f: F) -> Self
    where
        F: FnOnce(SpacingConfig) -> SpacingConfig,
    {
        self.config.spacing = f(self.config.spacing);
        self
    }

    /// Set margin configuration
    pub fn margins(mut self, margins: MarginConfig) -> Self {
        self.config.margins = margins;
        self
    }

    /// Build the final configuration
    pub fn build(self) -> PlotConfig {
        self.config
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_figure_config_default() {
        let config = FigureConfig::default();
        assert!((config.width - 6.4).abs() < 0.001);
        assert!((config.height - 4.8).abs() < 0.001);
        assert!((config.dpi - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_figure_config_canvas_size() {
        let config = FigureConfig::default();
        let (w, h) = config.canvas_size();
        assert_eq!(w, 640);
        assert_eq!(h, 480);

        let config = FigureConfig::new(6.4, 4.8, 300.0);
        let (w, h) = config.canvas_size();
        assert_eq!(w, 1920);
        assert_eq!(h, 1440);
    }

    #[test]
    fn test_typography_config_default() {
        let config = TypographyConfig::default();
        assert!((config.base_size - 10.0).abs() < 0.001);
        assert!((config.title_scale - 1.4).abs() < 0.001);
    }

    #[test]
    fn test_typography_computed_sizes() {
        let config = TypographyConfig::default();
        assert!((config.title_size() - 14.0).abs() < 0.001);
        assert!((config.label_size() - 10.0).abs() < 0.001);
        assert!((config.tick_size() - 9.0).abs() < 0.001);
    }

    #[test]
    fn test_typography_scale() {
        let config = TypographyConfig::default().scale(1.5);
        assert!((config.base_size - 15.0).abs() < 0.001);
        assert!((config.title_size() - 21.0).abs() < 0.001); // 15 * 1.4
    }

    #[test]
    fn test_line_config_default() {
        let config = LineConfig::default();
        assert!((config.data_width - 1.5).abs() < 0.001);
        assert!((config.axis_width - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_margin_config_auto() {
        let config = MarginConfig::auto();
        match config {
            MarginConfig::Auto { min, max } => {
                assert!((min - 0.3).abs() < 0.001);
                assert!((max - 1.0).abs() < 0.001);
            }
            _ => panic!("Expected Auto"),
        }
    }

    #[test]
    fn test_plot_config_builder() {
        let config = PlotConfig::builder()
            .figure(8.0, 6.0)
            .dpi(150.0)
            .font_size(12.0)
            .line_width(2.0)
            .build();

        assert!((config.figure.width - 8.0).abs() < 0.001);
        assert!((config.figure.height - 6.0).abs() < 0.001);
        assert!((config.figure.dpi - 150.0).abs() < 0.001);
        assert!((config.typography.base_size - 12.0).abs() < 0.001);
        assert!((config.lines.data_width - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_plot_config_canvas_size() {
        let config = PlotConfig::default();
        let (w, h) = config.canvas_size();
        assert_eq!(w, 640);
        assert_eq!(h, 480);
    }

    #[test]
    fn test_computed_margins() {
        let config = PlotConfig::default();
        let margins = config.compute_margins(true, true, true);

        // With all labels, margins should be non-zero
        assert!(margins.left > 0.0);
        assert!(margins.right > 0.0);
        assert!(margins.top > 0.0);
        assert!(margins.bottom > 0.0);

        // Left should be larger than right (for y-axis labels)
        assert!(margins.left > margins.right);
    }

    #[test]
    fn test_dpi_independence() {
        // At different DPIs, the ratio of elements should be the same
        let config = PlotConfig::default();

        let font_px_100 = config.typography.title_size_px(100.0);
        let canvas_100 = config.figure.canvas_size();
        let ratio_100 = font_px_100 / canvas_100.0 as f32;

        // Simulate 300 DPI
        let font_px_300 = config.typography.title_size_px(300.0);
        let canvas_300 = (
            in_to_px(config.figure.width, 300.0) as u32,
            in_to_px(config.figure.height, 300.0) as u32,
        );
        let ratio_300 = font_px_300 / canvas_300.0 as f32;

        // Ratios should be equal (DPI-independent)
        assert!((ratio_100 - ratio_300).abs() < 0.0001);
    }
}
