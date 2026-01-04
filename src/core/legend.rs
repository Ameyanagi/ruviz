//! Legend rendering with proper visual representation
//!
//! Implements matplotlib-compatible legend system with:
//! 1. Correct visual symbols for each series type (lines, markers, rectangles)
//! 2. Font-size-based spacing that scales with DPI
//! 3. Multiple position options including automatic "best" positioning
//! 4. Configurable frame styling

use crate::core::position::Position;
use crate::render::{Color, LineStyle, MarkerStyle};

// ============================================================================
// Legend Position System
// ============================================================================

/// Legend position codes (matplotlib-compatible)
///
/// Codes 0-10 correspond to matplotlib's numeric position codes.
/// Additional outside positions are provided for placing legends
/// outside the plot area.
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::prelude::*;
///
/// let x = vec![1.0, 2.0, 3.0, 4.0];
/// let y = vec![1.0, 4.0, 2.0, 3.0];
///
/// Plot::new()
///     .legend_position(LegendPosition::UpperRight)
///     .line(&x, &y)
///     .label("Data")
///     .end_series()
///     .save("legend_upper_right.png")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ![Legend positions](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/images/legend_positions.png)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LegendPosition {
    /// Code 0: Automatic best position (minimizes data overlap)
    Best,
    /// Code 1: Upper right corner (default)
    UpperRight,
    /// Code 2: Upper left corner
    UpperLeft,
    /// Code 3: Lower left corner
    LowerLeft,
    /// Code 4: Lower right corner
    LowerRight,
    /// Code 5: Right side (same as CenterRight)
    Right,
    /// Code 6: Center left edge
    CenterLeft,
    /// Code 7: Center right edge
    CenterRight,
    /// Code 8: Lower center
    LowerCenter,
    /// Code 9: Upper center
    UpperCenter,
    /// Code 10: Dead center
    Center,
    /// Outside the plot area, to the right
    OutsideRight,
    /// Outside the plot area, to the left
    OutsideLeft,
    /// Outside the plot area, above
    OutsideUpper,
    /// Outside the plot area, below
    OutsideLower,
    /// Custom position with anchor point
    Custom {
        /// X coordinate (0.0 = left, 1.0 = right, >1.0 = outside right)
        x: f32,
        /// Y coordinate (0.0 = bottom, 1.0 = top, >1.0 = outside top)
        y: f32,
        /// Anchor point on legend box
        anchor: LegendAnchor,
    },
}

impl Default for LegendPosition {
    fn default() -> Self {
        // Default to "best" auto-positioning (matplotlib loc=0)
        // This minimizes overlap with data points
        LegendPosition::Best
    }
}

impl LegendPosition {
    /// Returns true if this is an outside position
    pub fn is_outside(&self) -> bool {
        matches!(
            self,
            LegendPosition::OutsideRight
                | LegendPosition::OutsideLeft
                | LegendPosition::OutsideUpper
                | LegendPosition::OutsideLower
        ) || matches!(self, LegendPosition::Custom { x, y, .. } if *x > 1.0 || *y > 1.0 || *x < 0.0 || *y < 0.0)
    }

    /// Convert from matplotlib numeric code
    pub fn from_code(code: u8) -> Self {
        match code {
            0 => LegendPosition::Best,
            1 => LegendPosition::UpperRight,
            2 => LegendPosition::UpperLeft,
            3 => LegendPosition::LowerLeft,
            4 => LegendPosition::LowerRight,
            5 => LegendPosition::Right,
            6 => LegendPosition::CenterLeft,
            7 => LegendPosition::CenterRight,
            8 => LegendPosition::LowerCenter,
            9 => LegendPosition::UpperCenter,
            10 => LegendPosition::Center,
            _ => LegendPosition::UpperRight,
        }
    }

    /// Convert from the old Position enum (backward compatibility)
    pub fn from_position(pos: Position) -> Self {
        match pos {
            Position::Best => LegendPosition::Best,
            Position::TopLeft => LegendPosition::UpperLeft,
            Position::TopCenter => LegendPosition::UpperCenter,
            Position::TopRight => LegendPosition::UpperRight,
            Position::CenterLeft => LegendPosition::CenterLeft,
            Position::Center => LegendPosition::Center,
            Position::CenterRight => LegendPosition::CenterRight,
            Position::BottomLeft => LegendPosition::LowerLeft,
            Position::BottomCenter => LegendPosition::LowerCenter,
            Position::BottomRight => LegendPosition::LowerRight,
            Position::Custom { x, y } => LegendPosition::Custom {
                x,
                y,
                anchor: LegendAnchor::NorthWest,
            },
        }
    }
}

/// Anchor point for custom legend positioning
///
/// Specifies which corner/edge of the legend box aligns with the specified coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LegendAnchor {
    /// Top-left corner
    #[default]
    NorthWest,
    /// Top center
    North,
    /// Top-right corner
    NorthEast,
    /// Left center
    West,
    /// Center
    Center,
    /// Right center
    East,
    /// Bottom-left corner
    SouthWest,
    /// Bottom center
    South,
    /// Bottom-right corner
    SouthEast,
}

impl LegendAnchor {
    /// Get the offset multipliers for this anchor
    /// Returns (x_mult, y_mult) where:
    /// - x_mult: 0.0 = left edge, 0.5 = center, 1.0 = right edge
    /// - y_mult: 0.0 = top edge, 0.5 = center, 1.0 = bottom edge
    pub fn offset_multipliers(&self) -> (f32, f32) {
        match self {
            LegendAnchor::NorthWest => (0.0, 0.0),
            LegendAnchor::North => (0.5, 0.0),
            LegendAnchor::NorthEast => (1.0, 0.0),
            LegendAnchor::West => (0.0, 0.5),
            LegendAnchor::Center => (0.5, 0.5),
            LegendAnchor::East => (1.0, 0.5),
            LegendAnchor::SouthWest => (0.0, 1.0),
            LegendAnchor::South => (0.5, 1.0),
            LegendAnchor::SouthEast => (1.0, 1.0),
        }
    }
}

// ============================================================================
// Legend Item Types
// ============================================================================

/// Represents a single item in the legend with full style information
#[derive(Debug, Clone)]
pub struct LegendItem {
    /// Label text for this series
    pub label: String,
    /// Color of the series
    pub color: Color,
    /// Type of visual representation
    pub item_type: LegendItemType,
}

/// How the legend item should be visually represented
#[derive(Debug, Clone)]
pub enum LegendItemType {
    /// Line plot - draw a short line segment
    Line { style: LineStyle, width: f32 },
    /// Scatter plot - draw the marker
    Scatter { marker: MarkerStyle, size: f32 },
    /// Line plot with markers - draw line segment with marker at center
    LineMarker {
        line_style: LineStyle,
        line_width: f32,
        marker: MarkerStyle,
        marker_size: f32,
    },
    /// Bar chart - draw a filled rectangle
    Bar,
    /// Area/fill - draw a filled rectangle with optional edge
    Area { edge_color: Option<Color> },
    /// Histogram - same as bar
    Histogram,
    /// Error bars - draw line with error bar caps
    ErrorBar,
}

impl LegendItem {
    /// Create a legend item for a line series
    pub fn line(label: impl Into<String>, color: Color, style: LineStyle, width: f32) -> Self {
        Self {
            label: label.into(),
            color,
            item_type: LegendItemType::Line { style, width },
        }
    }

    /// Create a legend item for a scatter series
    pub fn scatter(label: impl Into<String>, color: Color, marker: MarkerStyle, size: f32) -> Self {
        Self {
            label: label.into(),
            color,
            item_type: LegendItemType::Scatter { marker, size },
        }
    }

    /// Create a legend item for a line+marker series
    pub fn line_marker(
        label: impl Into<String>,
        color: Color,
        line_style: LineStyle,
        line_width: f32,
        marker: MarkerStyle,
        marker_size: f32,
    ) -> Self {
        Self {
            label: label.into(),
            color,
            item_type: LegendItemType::LineMarker {
                line_style,
                line_width,
                marker,
                marker_size,
            },
        }
    }

    /// Create a legend item for a bar series
    pub fn bar(label: impl Into<String>, color: Color) -> Self {
        Self {
            label: label.into(),
            color,
            item_type: LegendItemType::Bar,
        }
    }

    /// Create a legend item for a histogram series
    pub fn histogram(label: impl Into<String>, color: Color) -> Self {
        Self {
            label: label.into(),
            color,
            item_type: LegendItemType::Histogram,
        }
    }

    /// Create a legend item for an area series
    pub fn area(label: impl Into<String>, color: Color, edge_color: Option<Color>) -> Self {
        Self {
            label: label.into(),
            color,
            item_type: LegendItemType::Area { edge_color },
        }
    }

    /// Create a legend item for error bars
    pub fn error_bar(label: impl Into<String>, color: Color) -> Self {
        Self {
            label: label.into(),
            color,
            item_type: LegendItemType::ErrorBar,
        }
    }

    /// Create from old (label, color) tuple for backward compatibility
    pub fn from_tuple(label: String, color: Color) -> Self {
        // Default to bar-style (filled square) for backward compatibility
        Self {
            label,
            color,
            item_type: LegendItemType::Bar,
        }
    }
}

// ============================================================================
// Spacing Configuration
// ============================================================================

/// Spacing configuration in font-size units (matplotlib-compatible)
///
/// All values are multipliers of the legend font size. For example,
/// with font_size = 10pt and handle_length = 2.0, the actual handle
/// length will be 20pt.
#[derive(Debug, Clone, Copy)]
pub struct LegendSpacing {
    /// Length of the handle (line segment, marker area) - default 2.0
    pub handle_length: f32,
    /// Height of the handle area - default 0.7
    pub handle_height: f32,
    /// Gap between handle and label text - default 0.8
    pub handle_text_pad: f32,
    /// Vertical space between entries - default 0.5
    pub label_spacing: f32,
    /// Padding inside the legend frame - default 0.4
    pub border_pad: f32,
    /// Gap between legend and plot axes - default 0.5
    pub border_axes_pad: f32,
    /// Horizontal gap between columns - default 2.0
    pub column_spacing: f32,
}

impl Default for LegendSpacing {
    fn default() -> Self {
        Self {
            handle_length: 2.0,   // 20px at 10pt - good handle length
            handle_height: 0.7,   // 7px at 10pt - marker/line height
            handle_text_pad: 1.0, // 10px at 10pt - gap between handle and text
            label_spacing: 0.7,   // 7px at 10pt - vertical gap between items
            border_pad: 0.6,      // 6px at 10pt - padding inside frame
            border_axes_pad: 1.0, // 10px at 10pt - gap from plot axes
            column_spacing: 2.0,  // 20px at 10pt - gap between columns
        }
    }
}

impl LegendSpacing {
    /// Calculate pixel values from font size
    pub fn to_pixels(&self, font_size: f32) -> LegendSpacingPixels {
        LegendSpacingPixels {
            handle_length: self.handle_length * font_size,
            handle_height: self.handle_height * font_size,
            handle_text_pad: self.handle_text_pad * font_size,
            label_spacing: self.label_spacing * font_size,
            border_pad: self.border_pad * font_size,
            border_axes_pad: self.border_axes_pad * font_size,
            column_spacing: self.column_spacing * font_size,
        }
    }
}

/// Spacing values in pixels (computed from font size)
#[derive(Debug, Clone, Copy)]
pub struct LegendSpacingPixels {
    pub handle_length: f32,
    pub handle_height: f32,
    pub handle_text_pad: f32,
    pub label_spacing: f32,
    pub border_pad: f32,
    pub border_axes_pad: f32,
    pub column_spacing: f32,
}

// ============================================================================
// Frame Styling
// ============================================================================

/// Frame/box styling for the legend
#[derive(Debug, Clone)]
pub struct LegendFrame {
    /// Whether to draw the frame
    pub visible: bool,
    /// Background fill color (with alpha for transparency)
    pub background: Color,
    /// Border stroke color (None = no border)
    pub border_color: Option<Color>,
    /// Border line width in points
    pub border_width: f32,
    /// Corner radius for rounded corners (0 = sharp corners)
    pub corner_radius: f32,
    /// Whether to draw a drop shadow
    pub shadow: bool,
    /// Shadow offset (x, y) in points
    pub shadow_offset: (f32, f32),
    /// Shadow color
    pub shadow_color: Color,
}

impl Default for LegendFrame {
    fn default() -> Self {
        Self {
            visible: true,
            background: Color::WHITE, // Fully opaque white (users can set transparency if needed)
            border_color: Some(Color::new_rgba(128, 128, 128, 200)),
            border_width: 0.8,
            corner_radius: 0.0,
            shadow: false,
            shadow_offset: (2.0, -2.0),
            shadow_color: Color::new_rgba(0, 0, 0, 50),
        }
    }
}

impl LegendFrame {
    /// Create a frame with no visible background or border
    pub fn invisible() -> Self {
        Self {
            visible: false,
            ..Default::default()
        }
    }

    /// Create a frame with rounded corners
    pub fn rounded(radius: f32) -> Self {
        Self {
            corner_radius: radius,
            ..Default::default()
        }
    }
}

// ============================================================================
// Complete Legend Configuration
// ============================================================================

/// Complete legend configuration
#[derive(Debug, Clone)]
pub struct Legend {
    /// Whether the legend is visible
    pub enabled: bool,
    /// Position of the legend
    pub position: LegendPosition,
    /// Spacing configuration (in font-size units)
    pub spacing: LegendSpacing,
    /// Frame styling
    pub frame: LegendFrame,
    /// Font size for legend labels in points
    pub font_size: f32,
    /// Text color for labels
    pub text_color: Color,
    /// Number of columns (1 = vertical layout)
    pub columns: usize,
    /// Title for the legend (optional)
    pub title: Option<String>,
}

impl Default for Legend {
    fn default() -> Self {
        Self {
            enabled: false,
            position: LegendPosition::default(),
            spacing: LegendSpacing::default(),
            frame: LegendFrame::default(),
            font_size: 10.0,
            text_color: Color::BLACK,
            columns: 1,
            title: None,
        }
    }
}

impl Legend {
    /// Create a new legend with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a legend at upper right (most common)
    pub fn upper_right() -> Self {
        Self {
            enabled: true,
            position: LegendPosition::UpperRight,
            ..Default::default()
        }
    }

    /// Create a legend with automatic best positioning
    pub fn best() -> Self {
        Self {
            enabled: true,
            position: LegendPosition::Best,
            ..Default::default()
        }
    }

    /// Create a legend outside the plot (right side)
    pub fn outside_right() -> Self {
        Self {
            enabled: true,
            position: LegendPosition::OutsideRight,
            ..Default::default()
        }
    }

    /// Enable the legend
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set legend position
    pub fn at(mut self, position: LegendPosition) -> Self {
        self.position = position;
        self
    }

    /// Set legend position (alias for at)
    pub fn position(mut self, position: LegendPosition) -> Self {
        self.position = position;
        self
    }

    /// Set a title for the legend
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set font size
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set number of columns
    pub fn columns(mut self, cols: usize) -> Self {
        self.columns = cols.max(1);
        self
    }

    /// Configure frame styling
    pub fn frame(mut self, frame: LegendFrame) -> Self {
        self.frame = frame;
        self
    }

    /// Configure spacing
    pub fn spacing(mut self, spacing: LegendSpacing) -> Self {
        self.spacing = spacing;
        self
    }

    /// Calculate the required size for the legend in pixels
    ///
    /// # Arguments
    /// * `items` - Legend items to display
    /// * `char_width` - Approximate width of a character in pixels
    pub fn calculate_size(&self, items: &[LegendItem], char_width: f32) -> (f32, f32) {
        if items.is_empty() {
            return (0.0, 0.0);
        }

        let spacing_px = self.spacing.to_pixels(self.font_size);

        // Find max label width (approximate)
        let max_label_len = items.iter().map(|item| item.label.len()).max().unwrap_or(0);

        let label_width = max_label_len as f32 * char_width;
        let item_width = spacing_px.handle_length + spacing_px.handle_text_pad + label_width;

        // Calculate dimensions based on columns
        let items_per_col = (items.len() + self.columns - 1) / self.columns;

        let content_width = item_width * self.columns as f32
            + (self.columns.saturating_sub(1)) as f32 * spacing_px.column_spacing;

        // Content height: n rows with spacing BETWEEN items (not after the last one)
        // This ensures equal top and bottom padding inside the frame
        let content_height = items_per_col as f32 * self.font_size
            + (items_per_col.saturating_sub(1)) as f32 * spacing_px.label_spacing;

        // Add title height if present
        let title_height = if self.title.is_some() {
            self.font_size + spacing_px.label_spacing
        } else {
            0.0
        };

        let width = content_width + spacing_px.border_pad * 2.0;
        let height = content_height + title_height + spacing_px.border_pad * 2.0;

        (width, height)
    }

    /// Calculate position coordinates for the legend
    ///
    /// # Arguments
    /// * `legend_size` - (width, height) of the legend box
    /// * `plot_area` - (left, top, right, bottom) of the plot area
    ///
    /// # Returns
    /// (x, y) coordinates for the top-left corner of the legend
    pub fn calculate_position(
        &self,
        legend_size: (f32, f32),
        plot_area: (f32, f32, f32, f32),
    ) -> (f32, f32) {
        let (width, height) = legend_size;
        let (left, top, right, bottom) = plot_area;
        let spacing_px = self.spacing.to_pixels(self.font_size);
        let pad = spacing_px.border_axes_pad;

        match self.position {
            LegendPosition::Best => {
                // Default to upper right, actual best calculation done separately
                (right - width - pad, top + pad)
            }
            LegendPosition::UpperRight | LegendPosition::Right => (right - width - pad, top + pad),
            LegendPosition::UpperLeft => (left + pad, top + pad),
            LegendPosition::LowerLeft => (left + pad, bottom - height - pad),
            LegendPosition::LowerRight => (right - width - pad, bottom - height - pad),
            LegendPosition::CenterLeft => {
                let center_y = (top + bottom) / 2.0;
                (left + pad, center_y - height / 2.0)
            }
            LegendPosition::CenterRight => {
                let center_y = (top + bottom) / 2.0;
                (right - width - pad, center_y - height / 2.0)
            }
            LegendPosition::LowerCenter => {
                let center_x = (left + right) / 2.0;
                (center_x - width / 2.0, bottom - height - pad)
            }
            LegendPosition::UpperCenter => {
                let center_x = (left + right) / 2.0;
                (center_x - width / 2.0, top + pad)
            }
            LegendPosition::Center => {
                let center_x = (left + right) / 2.0;
                let center_y = (top + bottom) / 2.0;
                (center_x - width / 2.0, center_y - height / 2.0)
            }
            LegendPosition::OutsideRight => (right + pad, top),
            LegendPosition::OutsideLeft => (left - width - pad, top),
            LegendPosition::OutsideUpper => (right - width, top - height - pad),
            LegendPosition::OutsideLower => (right - width, bottom + pad),
            LegendPosition::Custom { x, y, anchor } => {
                let plot_width = right - left;
                let plot_height = bottom - top;
                let (x_mult, y_mult) = anchor.offset_multipliers();

                let base_x = left + x * plot_width;
                let base_y = top + (1.0 - y) * plot_height; // Invert Y for screen coords

                (base_x - x_mult * width, base_y - y_mult * height)
            }
        }
    }
}

// ============================================================================
// Best Position Algorithm
// ============================================================================

/// Find the best legend position that minimizes overlap with data
///
/// # Arguments
/// * `legend_size` - (width, height) of the legend box
/// * `plot_area` - (left, top, right, bottom) of the plot area
/// * `data_bboxes` - Bounding boxes of data series
/// * `spacing` - Spacing configuration for calculating padding
/// * `font_size` - Font size for spacing calculations
///
/// # Returns
/// The best position from the 9 standard inside positions
pub fn find_best_position(
    legend_size: (f32, f32),
    plot_area: (f32, f32, f32, f32),
    data_bboxes: &[(f32, f32, f32, f32)], // (left, top, right, bottom)
    spacing: &LegendSpacing,
    font_size: f32,
) -> LegendPosition {
    let candidates = [
        LegendPosition::UpperRight,
        LegendPosition::UpperLeft,
        LegendPosition::LowerLeft,
        LegendPosition::LowerRight,
        LegendPosition::CenterRight,
        LegendPosition::CenterLeft,
        LegendPosition::UpperCenter,
        LegendPosition::LowerCenter,
        LegendPosition::Center,
    ];

    let legend = Legend {
        position: LegendPosition::UpperRight, // Temporary, will be changed
        spacing: *spacing,
        font_size,
        ..Default::default()
    };

    let mut best_position = LegendPosition::UpperRight;
    let mut min_overlap = f32::MAX;

    for &candidate in &candidates {
        let mut test_legend = legend.clone();
        test_legend.position = candidate;

        let (x, y) = test_legend.calculate_position(legend_size, plot_area);
        let legend_bbox = (x, y, x + legend_size.0, y + legend_size.1);

        let overlap = calculate_total_overlap(legend_bbox, data_bboxes);

        if overlap < min_overlap {
            min_overlap = overlap;
            best_position = candidate;
        }
    }

    best_position
}

/// Calculate total overlap area between legend and data bounding boxes
fn calculate_total_overlap(
    legend_bbox: (f32, f32, f32, f32),
    data_bboxes: &[(f32, f32, f32, f32)],
) -> f32 {
    data_bboxes
        .iter()
        .map(|data_bbox| calculate_bbox_overlap(legend_bbox, *data_bbox))
        .sum()
}

/// Calculate overlap area between two bounding boxes
fn calculate_bbox_overlap(bbox1: (f32, f32, f32, f32), bbox2: (f32, f32, f32, f32)) -> f32 {
    let (l1, t1, r1, b1) = bbox1;
    let (l2, t2, r2, b2) = bbox2;

    let x_overlap = (r1.min(r2) - l1.max(l2)).max(0.0);
    let y_overlap = (b1.min(b2) - t1.max(t2)).max(0.0);

    x_overlap * y_overlap
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legend_item_creation() {
        let line_item = LegendItem::line("sin(x)", Color::BLUE, LineStyle::Solid, 1.5);
        assert_eq!(line_item.label, "sin(x)");
        assert!(matches!(line_item.item_type, LegendItemType::Line { .. }));

        let scatter_item = LegendItem::scatter("data", Color::RED, MarkerStyle::Circle, 6.0);
        assert!(matches!(
            scatter_item.item_type,
            LegendItemType::Scatter { .. }
        ));

        let line_marker = LegendItem::line_marker(
            "combined",
            Color::GREEN,
            LineStyle::Dashed,
            1.5,
            MarkerStyle::Circle,
            6.0,
        );
        assert!(matches!(
            line_marker.item_type,
            LegendItemType::LineMarker { .. }
        ));
    }

    #[test]
    fn test_spacing_to_pixels() {
        let spacing = LegendSpacing::default();
        let pixels = spacing.to_pixels(10.0);

        assert!((pixels.handle_length - 20.0).abs() < 0.001); // 2.0 * 10
        assert!((pixels.label_spacing - 7.0).abs() < 0.001); // 0.7 * 10
        assert!((pixels.border_pad - 6.0).abs() < 0.001); // 0.6 * 10
        assert!((pixels.handle_text_pad - 10.0).abs() < 0.001); // 1.0 * 10
        assert!((pixels.border_axes_pad - 10.0).abs() < 0.001); // 1.0 * 10
    }

    #[test]
    fn test_legend_position_is_outside() {
        assert!(!LegendPosition::UpperRight.is_outside());
        assert!(!LegendPosition::Center.is_outside());
        assert!(LegendPosition::OutsideRight.is_outside());
        assert!(LegendPosition::OutsideUpper.is_outside());

        let custom_inside = LegendPosition::Custom {
            x: 0.5,
            y: 0.5,
            anchor: LegendAnchor::Center,
        };
        assert!(!custom_inside.is_outside());

        let custom_outside = LegendPosition::Custom {
            x: 1.1,
            y: 0.5,
            anchor: LegendAnchor::NorthWest,
        };
        assert!(custom_outside.is_outside());
    }

    #[test]
    fn test_legend_size_calculation() {
        let legend = Legend::new();
        let items = vec![
            LegendItem::line("sin(x)", Color::BLUE, LineStyle::Solid, 1.5),
            LegendItem::line("cos(x)", Color::RED, LineStyle::Dashed, 1.5),
        ];

        let (width, height) = legend.calculate_size(&items, 6.0);
        assert!(width > 0.0);
        assert!(height > 0.0);
    }

    #[test]
    fn test_legend_position_calculation() {
        let legend = Legend::new().at(LegendPosition::UpperRight);
        let plot_area = (100.0, 50.0, 500.0, 400.0); // left, top, right, bottom
        let legend_size = (80.0, 60.0);

        let (x, y) = legend.calculate_position(legend_size, plot_area);

        // Should be in upper right corner with padding
        assert!(x < 500.0);
        assert!(x > 400.0);
        assert!(y > 50.0);
        assert!(y < 100.0);
    }

    #[test]
    fn test_anchor_offsets() {
        assert_eq!(LegendAnchor::NorthWest.offset_multipliers(), (0.0, 0.0));
        assert_eq!(LegendAnchor::Center.offset_multipliers(), (0.5, 0.5));
        assert_eq!(LegendAnchor::SouthEast.offset_multipliers(), (1.0, 1.0));
    }

    #[test]
    fn test_bbox_overlap() {
        // No overlap
        let overlap1 = calculate_bbox_overlap((0.0, 0.0, 10.0, 10.0), (20.0, 20.0, 30.0, 30.0));
        assert_eq!(overlap1, 0.0);

        // Partial overlap
        let overlap2 = calculate_bbox_overlap((0.0, 0.0, 10.0, 10.0), (5.0, 5.0, 15.0, 15.0));
        assert_eq!(overlap2, 25.0); // 5x5 overlap

        // Full containment
        let overlap3 = calculate_bbox_overlap((0.0, 0.0, 20.0, 20.0), (5.0, 5.0, 10.0, 10.0));
        assert_eq!(overlap3, 25.0); // 5x5 inner box
    }

    #[test]
    fn test_find_best_position() {
        let legend_size = (80.0, 60.0);
        let plot_area = (100.0, 50.0, 500.0, 400.0);

        // Data concentrated in upper right - should choose lower left
        let data_bboxes = vec![(400.0, 50.0, 500.0, 150.0)];

        let best = find_best_position(
            legend_size,
            plot_area,
            &data_bboxes,
            &LegendSpacing::default(),
            10.0,
        );

        // Should not be upper right since data is there
        assert_ne!(best, LegendPosition::UpperRight);
    }
}
