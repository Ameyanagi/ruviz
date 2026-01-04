//! Content-driven layout system for plot element positioning.
//!
//! This module provides a layout calculator that computes element positions
//! based on actual content measurements rather than arbitrary margin percentages.
//!
//! # Key Insight
//!
//! Margins should be computed FROM text measurements, not the other way around.
//!
//! # Layout Algorithm
//!
//! 1. **Measure content** - Estimate or measure all text elements
//! 2. **Calculate margins** - Margins = content size + padding
//! 3. **Position elements** - Each element adjacent to its neighbor
//! 4. **Center the plot** - Distribute extra space symmetrically

use crate::core::{SpacingConfig, TypographyConfig};

// =============================================================================
// Data Structures
// =============================================================================

/// Position for a text element
#[derive(Debug, Clone, PartialEq)]
pub struct TextPosition {
    /// Horizontal position (center for centered text, left edge otherwise)
    pub x: f32,
    /// Vertical position (baseline for horizontal text, center for rotated)
    pub y: f32,
    /// Font size in pixels
    pub size: f32,
}

/// Computed margins in pixels (for debugging and inspection)
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedMarginsPixels {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

/// A rectangle representing the plot area
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl LayoutRect {
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    pub fn center_x(&self) -> f32 {
        (self.left + self.right) / 2.0
    }

    pub fn center_y(&self) -> f32 {
        (self.top + self.bottom) / 2.0
    }
}

/// Complete layout with computed positions for all plot elements
#[derive(Debug, Clone, PartialEq)]
pub struct PlotLayout {
    /// The plotting area where data is drawn
    pub plot_area: LayoutRect,

    /// Title position (baseline center point), None if no title
    pub title_pos: Option<TextPosition>,

    /// X-axis label position (baseline center point)
    pub xlabel_pos: Option<TextPosition>,

    /// Y-axis label position (center point, rotated 90° CCW)
    pub ylabel_pos: Option<TextPosition>,

    /// Y-coordinate for x-axis tick label baselines
    pub xtick_baseline_y: f32,

    /// X-coordinate for right edge of y-axis tick labels
    pub ytick_right_x: f32,

    /// Computed margins in pixels (for debugging/inspection)
    pub margins: ComputedMarginsPixels,
}

/// Content information needed for layout calculation
#[derive(Debug, Clone, Default)]
pub struct PlotContent {
    pub title: Option<String>,
    pub xlabel: Option<String>,
    pub ylabel: Option<String>,
    /// Maximum number of characters in y-tick labels (for width estimation)
    pub max_ytick_chars: usize,
    /// Maximum number of characters in x-tick labels (for height is same)
    pub max_xtick_chars: usize,
}

impl PlotContent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_xlabel(mut self, label: impl Into<String>) -> Self {
        self.xlabel = Some(label.into());
        self
    }

    pub fn with_ylabel(mut self, label: impl Into<String>) -> Self {
        self.ylabel = Some(label.into());
        self
    }

    pub fn with_tick_chars(mut self, max_ytick: usize, max_xtick: usize) -> Self {
        self.max_ytick_chars = max_ytick;
        self.max_xtick_chars = max_xtick;
        self
    }
}

// =============================================================================
// Text Size Estimation
// =============================================================================

/// Estimate text width in pixels based on character count and font size
///
/// Uses a conservative estimate of 0.6 * font_size per character,
/// which works well for most sans-serif fonts.
pub fn estimate_text_width(text: &str, font_size_px: f32) -> f32 {
    let char_width = font_size_px * 0.6;
    text.chars().count() as f32 * char_width
}

/// Estimate text height in pixels based on font size
///
/// Returns font_size * 1.2 to account for line height and descenders.
pub fn estimate_text_height(font_size_px: f32) -> f32 {
    font_size_px * 1.2
}

/// Estimate maximum width of tick labels in pixels
///
/// Uses the maximum character count to estimate width.
pub fn estimate_tick_label_width(max_chars: usize, font_size_px: f32) -> f32 {
    let chars = max_chars.max(3); // Minimum 3 chars (e.g., "0.0")
    estimate_text_width(&"X".repeat(chars), font_size_px)
}

// =============================================================================
// Layout Calculator
// =============================================================================

/// Configuration for the layout calculator
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Small buffer from canvas edges (in points)
    pub edge_buffer_pt: f32,
    /// Whether to center the plot by distributing extra space
    pub center_plot: bool,
    /// Maximum margin as fraction of dimension (prevents overflow)
    pub max_margin_fraction: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            edge_buffer_pt: 8.0, // Professional edge buffer from canvas edges
            center_plot: true,
            max_margin_fraction: 0.4, // Max 40% of dimension for any margin
        }
    }
}

/// Calculator for content-driven plot layout
#[derive(Default)]
pub struct LayoutCalculator {
    pub config: LayoutConfig,
}

impl LayoutCalculator {
    pub fn new(config: LayoutConfig) -> Self {
        Self { config }
    }

    /// Compute the complete layout for a plot
    ///
    /// # Arguments
    ///
    /// * `canvas_size` - Width and height in pixels
    /// * `content` - Information about plot content (title, labels, etc.)
    /// * `typography` - Font size configuration
    /// * `spacing` - Padding configuration
    /// * `dpi` - Dots per inch for unit conversion
    pub fn compute(
        &self,
        canvas_size: (u32, u32),
        content: &PlotContent,
        typography: &TypographyConfig,
        spacing: &SpacingConfig,
        dpi: f32,
    ) -> PlotLayout {
        let (canvas_width, canvas_height) = (canvas_size.0 as f32, canvas_size.1 as f32);

        // Convert points to pixels
        let pt_to_px = |pt: f32| pt * dpi / 72.0;

        let edge_buffer = pt_to_px(self.config.edge_buffer_pt);
        let title_pad = pt_to_px(spacing.title_pad);
        let label_pad = pt_to_px(spacing.label_pad);
        let tick_pad = pt_to_px(spacing.tick_pad);

        // Get font sizes in pixels
        let title_size_px = pt_to_px(typography.title_size());
        let label_size_px = pt_to_px(typography.label_size());
        let tick_size_px = pt_to_px(typography.tick_size());

        // Step 1: Measure/estimate content sizes
        let title_height = if content.title.is_some() {
            estimate_text_height(title_size_px)
        } else {
            0.0
        };

        let xlabel_height = if content.xlabel.is_some() {
            estimate_text_height(label_size_px)
        } else {
            0.0
        };

        let ylabel_width = if content.ylabel.is_some() {
            // Rotated text: height becomes width
            estimate_text_height(label_size_px)
        } else {
            0.0
        };

        let xtick_height = estimate_text_height(tick_size_px);
        let ytick_width = estimate_tick_label_width(
            content.max_ytick_chars.max(5), // Default to 5 chars if not specified
            tick_size_px,
        );

        // Step 2: Calculate minimum required margins
        let mut min_top = edge_buffer;
        if content.title.is_some() {
            min_top += title_height + title_pad;
        }

        let mut min_bottom = edge_buffer + xtick_height + tick_pad;
        if content.xlabel.is_some() {
            min_bottom += xlabel_height + label_pad;
        }

        let mut min_left = edge_buffer + ytick_width + tick_pad;
        if content.ylabel.is_some() {
            min_left += ylabel_width + label_pad;
        }

        let min_right = edge_buffer;

        // Clamp margins to max fraction of dimension
        let max_h_margin = canvas_width * self.config.max_margin_fraction;
        let max_v_margin = canvas_height * self.config.max_margin_fraction;

        let final_top = min_top.min(max_v_margin);
        let final_bottom = min_bottom.min(max_v_margin);
        let final_left = min_left.min(max_h_margin);
        let mut final_right = min_right.min(max_h_margin);

        // Step 3: Center the chart area in the canvas
        // Add extra right margin to balance the left margin (ylabel + ticks)
        // This centers the actual plotting area, not the whole composition
        if self.config.center_plot {
            let extra_right = (final_left - final_right).max(0.0);
            final_right += extra_right;
        }

        // Step 4: Compute plot area
        let plot_area = LayoutRect {
            left: final_left,
            top: final_top,
            right: canvas_width - final_right,
            bottom: canvas_height - final_bottom,
        };

        // Step 5: Position elements - centered on PLOT AREA, not canvas
        let title_pos = content.title.as_ref().map(|_| TextPosition {
            x: plot_area.center_x(),       // Centered on plot area
            y: edge_buffer + title_height, // Just below buffer, at baseline
            size: title_size_px,
        });

        let xlabel_pos = content.xlabel.as_ref().map(|_| TextPosition {
            x: plot_area.center_x(),        // Centered on plot area
            y: canvas_height - edge_buffer, // Just above bottom buffer
            size: label_size_px,
        });

        let ylabel_pos = content.ylabel.as_ref().map(|_| TextPosition {
            x: edge_buffer + ylabel_width / 2.0, // In left margin
            y: plot_area.center_y(),             // Vertically centered on plot
            size: label_size_px,
        });

        let xtick_baseline_y = plot_area.bottom + tick_pad + xtick_height;
        let ytick_right_x = plot_area.left - tick_pad;

        PlotLayout {
            plot_area,
            title_pos,
            xlabel_pos,
            ylabel_pos,
            xtick_baseline_y,
            ytick_right_x,
            margins: ComputedMarginsPixels {
                left: final_left,
                right: final_right,
                top: final_top,
                bottom: final_bottom,
            },
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn default_typography() -> TypographyConfig {
        TypographyConfig::default()
    }

    fn default_spacing() -> SpacingConfig {
        SpacingConfig::default()
    }

    #[test]
    fn test_estimate_text_width() {
        let width = estimate_text_width("Hello", 12.0);
        // 5 chars * 12 * 0.6 = 36
        assert!((width - 36.0).abs() < 0.1);
    }

    #[test]
    fn test_estimate_text_height() {
        let height = estimate_text_height(14.0);
        // 14 * 1.2 = 16.8
        assert!((height - 16.8).abs() < 0.1);
    }

    #[test]
    fn test_layout_all_elements() {
        let calculator = LayoutCalculator::default();
        let content = PlotContent::new()
            .with_title("Test Title")
            .with_xlabel("X Values")
            .with_ylabel("Y Values")
            .with_tick_chars(5, 3);

        let layout = calculator.compute(
            (640, 480),
            &content,
            &default_typography(),
            &default_spacing(),
            100.0,
        );

        // Plot area should have positive dimensions
        assert!(layout.plot_area.width() > 0.0);
        assert!(layout.plot_area.height() > 0.0);

        // Title should be present and near top
        assert!(layout.title_pos.is_some());
        let title = layout.title_pos.unwrap();
        assert!(title.y < 50.0); // Near top

        // Labels should be present
        assert!(layout.xlabel_pos.is_some());
        assert!(layout.ylabel_pos.is_some());
    }

    #[test]
    fn test_layout_no_title() {
        let calculator = LayoutCalculator::default();
        let content = PlotContent::new().with_xlabel("X").with_ylabel("Y");

        let layout = calculator.compute(
            (640, 480),
            &content,
            &default_typography(),
            &default_spacing(),
            100.0,
        );

        // No title position
        assert!(layout.title_pos.is_none());

        // Top margin should be smaller
        assert!(layout.margins.top < 50.0);
    }

    #[test]
    fn test_layout_no_labels() {
        let calculator = LayoutCalculator::default();
        let content = PlotContent::new().with_tick_chars(5, 3);

        let layout = calculator.compute(
            (640, 480),
            &content,
            &default_typography(),
            &default_spacing(),
            100.0,
        );

        // No title or label positions
        assert!(layout.title_pos.is_none());
        assert!(layout.xlabel_pos.is_none());
        assert!(layout.ylabel_pos.is_none());

        // Plot area should be maximized
        assert!(layout.plot_area.width() > 500.0);
        assert!(layout.plot_area.height() > 400.0);
    }

    #[test]
    fn test_layout_respects_edge_buffer() {
        let calculator = LayoutCalculator::new(LayoutConfig {
            edge_buffer_pt: 10.0,
            ..Default::default()
        });
        let content = PlotContent::new();

        let layout = calculator.compute(
            (640, 480),
            &content,
            &default_typography(),
            &default_spacing(),
            100.0,
        );

        // At 100 DPI, 10pt = ~14px
        let expected_buffer = 10.0 * 100.0 / 72.0;
        assert!(layout.margins.right >= expected_buffer - 1.0);
    }

    #[test]
    fn test_layout_centering() {
        let calculator = LayoutCalculator::new(LayoutConfig {
            center_plot: true,
            ..Default::default()
        });
        let content = PlotContent::new()
            .with_ylabel("Y Label")
            .with_tick_chars(5, 3);

        let layout = calculator.compute(
            (640, 480),
            &content,
            &default_typography(),
            &default_spacing(),
            100.0,
        );

        // With centering enabled, right margin should equal left margin
        // This centers the chart area itself in the canvas
        let margin_diff = (layout.margins.right - layout.margins.left).abs();
        assert!(
            margin_diff < 1.0,
            "Margins should be equal: left={}, right={}",
            layout.margins.left,
            layout.margins.right
        );

        // Plot center should be at canvas center
        let plot_center = layout.plot_area.center_x();
        let canvas_center = 640.0 / 2.0;
        let center_diff = (plot_center - canvas_center).abs();
        assert!(
            center_diff < 1.0,
            "Plot should be centered: plot_center={}, canvas_center={}",
            plot_center,
            canvas_center
        );
    }

    #[test]
    fn test_layout_margin_clamping() {
        let calculator = LayoutCalculator::new(LayoutConfig {
            max_margin_fraction: 0.3,
            ..Default::default()
        });

        // Very long ylabel that would normally cause huge left margin
        let content = PlotContent::new().with_ylabel("Very Long Y-Axis Label That Would Be Huge");

        let layout = calculator.compute(
            (200, 150), // Small canvas
            &content,
            &default_typography(),
            &default_spacing(),
            100.0,
        );

        // Margin should be clamped to 30% of width = 60px
        assert!(layout.margins.left <= 200.0 * 0.3 + 1.0);
    }

    #[test]
    fn test_layout_very_long_title() {
        let calculator = LayoutCalculator::default();
        let content = PlotContent::new()
            .with_title("This is an Extremely Long Title That Should Still Render Properly Without Breaking the Layout")
            .with_xlabel("X Axis")
            .with_ylabel("Y Axis");

        let layout = calculator.compute(
            (640, 480),
            &content,
            &default_typography(),
            &default_spacing(),
            100.0,
        );

        // Plot area should still have positive dimensions
        assert!(layout.plot_area.width() > 0.0);
        assert!(layout.plot_area.height() > 0.0);

        // Title should be present
        assert!(layout.title_pos.is_some());

        // Plot area should not be squished too much
        assert!(layout.plot_area.width() > 300.0);
    }

    #[test]
    fn test_layout_unicode_labels() {
        let calculator = LayoutCalculator::default();
        let content = PlotContent::new()
            .with_title("数据分析 - データ分析")
            .with_xlabel("時間 (μs)")
            .with_ylabel("Amplitude (±σ)")
            .with_tick_chars(6, 4);

        let layout = calculator.compute(
            (640, 480),
            &content,
            &default_typography(),
            &default_spacing(),
            100.0,
        );

        // Plot area should still have positive dimensions
        assert!(layout.plot_area.width() > 0.0);
        assert!(layout.plot_area.height() > 0.0);

        // All positions should be present
        assert!(layout.title_pos.is_some());
        assert!(layout.xlabel_pos.is_some());
        assert!(layout.ylabel_pos.is_some());
    }

    #[test]
    fn test_layout_very_small_canvas() {
        let calculator = LayoutCalculator::default();
        let content = PlotContent::new()
            .with_title("Title")
            .with_xlabel("X")
            .with_ylabel("Y");

        let layout = calculator.compute(
            (100, 80), // Very small canvas
            &content,
            &default_typography(),
            &default_spacing(),
            100.0,
        );

        // Even on very small canvas, plot area should be valid
        assert!(layout.plot_area.width() > 0.0);
        assert!(layout.plot_area.height() > 0.0);

        // Margins should be clamped
        assert!(layout.margins.left < 50.0);
        assert!(layout.margins.top < 40.0);
    }

    #[test]
    fn test_layout_high_dpi() {
        let calculator = LayoutCalculator::default();
        let content = PlotContent::new()
            .with_title("High DPI Title")
            .with_xlabel("X Axis")
            .with_ylabel("Y Axis");

        let layout_100dpi = calculator.compute(
            (640, 480),
            &content,
            &default_typography(),
            &default_spacing(),
            100.0,
        );

        let layout_200dpi = calculator.compute(
            (1280, 960), // Doubled canvas for 2x DPI
            &content,
            &default_typography(),
            &default_spacing(),
            200.0,
        );

        // At 2x DPI with 2x canvas, proportions should be similar
        let ratio_100 = layout_100dpi.plot_area.width() / 640.0;
        let ratio_200 = layout_200dpi.plot_area.width() / 1280.0;

        // Ratios should be close (within 20%)
        let diff = (ratio_100 - ratio_200).abs() / ratio_100;
        assert!(diff < 0.2, "DPI scaling ratio diff: {}", diff);
    }
}
