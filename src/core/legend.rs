//! Legend rendering with proper visual representation
//!
//! Implements matplotlib-compatible legend system with:
//! 1. Correct visual symbols for each series type (lines, markers, rectangles)
//! 2. Font-size-based spacing that scales with DPI
//! 3. Multiple position options including automatic "best" positioning
//! 4. Configurable frame styling

use crate::core::Result;
#[allow(deprecated)]
use crate::core::position::Position;
use crate::core::units::RenderScale;
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
/// ![Legend positions](https://raw.githubusercontent.com/Ameyanagi/ruviz/main/docs/assets/rustdoc/legend_positions.png)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LegendPosition {
    /// Code 0: Automatic best position (minimizes data overlap)
    #[default]
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

    /// Convert from the deprecated [`Position`] enum.
    ///
    /// The mapping is lossless. Note the Y-axis convention change: [`Position`]
    /// measures Y downward from the top of the plot area, while
    /// [`LegendPosition::Custom`] measures Y upward from the bottom (axes
    /// coordinates, as matplotlib's `bbox_to_anchor` does), so custom
    /// coordinates are flipped here.
    #[allow(deprecated)]
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
            // `Position` Y grows downward, `LegendPosition::Custom` Y grows
            // upward. Without this flip `Position::custom(0.1, 0.05)` — "just
            // below the top-left corner" — rendered at the *bottom* left.
            Position::Custom { x, y } => LegendPosition::Custom {
                x,
                y: 1.0 - y,
                anchor: LegendAnchor::NorthWest,
            },
        }
    }
}

#[allow(deprecated)]
impl From<Position> for LegendPosition {
    fn from(pos: Position) -> Self {
        LegendPosition::from_position(pos)
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
    /// Whether this series has Y error bars (shown as vertical error bar in legend)
    pub has_error_bars: bool,
    /// Indices of the plot series this entry stands for: one for a plain
    /// series, several for a grouped entry, none for annotation entries.
    pub series_indices: Vec<usize>,
    /// Draw the entry faded — the series it stands for is hidden, but the
    /// entry stays clickable so the user can bring the series back.
    pub dimmed: bool,
}

/// How the legend item should be visually represented
#[derive(Debug, Clone)]
pub enum LegendItemType {
    /// Line plot - draw a short line segment
    Line { style: LineStyle, width: f32 },
    /// Scatter plot - draw the marker
    ///
    /// `edge` is the rim the plotted markers actually carry, as
    /// `(colour, width_in_points)`; the backend scales the width with DPI so
    /// the key matches the marker it stands for. `None` means bare markers.
    Scatter {
        marker: MarkerStyle,
        size: f32,
        edge: Option<(Color, f32)>,
    },
    /// Line plot with markers - draw line segment with marker at center
    ///
    /// See [`LegendItemType::Scatter`] for the meaning of `marker_edge`.
    LineMarker {
        line_style: LineStyle,
        line_width: f32,
        marker: MarkerStyle,
        marker_size: f32,
        marker_edge: Option<(Color, f32)>,
    },
    /// Bar chart - draw a filled rectangle
    ///
    /// `edge` is the edge the bars are actually stroked with, as
    /// `(colour, width_in_points)`. The width is in **points** so the backend
    /// scales it with DPI exactly like the patch it stands for. `None` means
    /// the bars are drawn flat and the key is flat too.
    Bar { edge: Option<(Color, f32)> },
    /// Area/fill - draw a filled rectangle with optional edge
    Area { edge_color: Option<Color> },
    /// Histogram - same as bar
    ///
    /// See [`LegendItemType::Bar`] for the meaning of `edge`. Histogram bars
    /// carry a default edge (see `HistogramData::resolved_edge`), so this is
    /// normally `Some(..)`.
    Histogram { edge: Option<(Color, f32)> },
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
            has_error_bars: false,
            series_indices: Vec::new(),
            dimmed: false,
        }
    }

    /// Create a legend item for a scatter series with bare markers
    pub fn scatter(label: impl Into<String>, color: Color, marker: MarkerStyle, size: f32) -> Self {
        Self::scatter_with_edge(label, color, marker, size, None)
    }

    /// Create a legend item for a scatter series with an explicit marker rim
    ///
    /// `edge` is `(colour, width_in_points)` and should be the rim the markers
    /// themselves are stroked with, so the key matches what is plotted.
    pub fn scatter_with_edge(
        label: impl Into<String>,
        color: Color,
        marker: MarkerStyle,
        size: f32,
        edge: Option<(Color, f32)>,
    ) -> Self {
        Self {
            label: label.into(),
            color,
            item_type: LegendItemType::Scatter { marker, size, edge },
            has_error_bars: false,
            series_indices: Vec::new(),
            dimmed: false,
        }
    }

    /// Create a legend item for a line+marker series with bare markers
    pub fn line_marker(
        label: impl Into<String>,
        color: Color,
        line_style: LineStyle,
        line_width: f32,
        marker: MarkerStyle,
        marker_size: f32,
    ) -> Self {
        Self::line_marker_with_edge(
            label,
            color,
            line_style,
            line_width,
            marker,
            marker_size,
            None,
        )
    }

    /// Create a legend item for a line+marker series with an explicit marker rim
    ///
    /// See [`LegendItem::scatter_with_edge`] for the meaning of `marker_edge`.
    #[allow(clippy::too_many_arguments)]
    pub fn line_marker_with_edge(
        label: impl Into<String>,
        color: Color,
        line_style: LineStyle,
        line_width: f32,
        marker: MarkerStyle,
        marker_size: f32,
        marker_edge: Option<(Color, f32)>,
    ) -> Self {
        Self {
            label: label.into(),
            color,
            item_type: LegendItemType::LineMarker {
                line_style,
                line_width,
                marker,
                marker_size,
                marker_edge,
            },
            has_error_bars: false,
            series_indices: Vec::new(),
            dimmed: false,
        }
    }

    /// Create a legend item for a bar series with no edge stroke
    pub fn bar(label: impl Into<String>, color: Color) -> Self {
        Self::bar_with_edge(label, color, None)
    }

    /// Create a legend item for a bar series with an explicit edge
    ///
    /// `edge` is `(colour, width_in_points)` and should be the edge the bars
    /// themselves are stroked with, so the key matches what is plotted.
    pub fn bar_with_edge(
        label: impl Into<String>,
        color: Color,
        edge: Option<(Color, f32)>,
    ) -> Self {
        Self {
            label: label.into(),
            color,
            item_type: LegendItemType::Bar { edge },
            has_error_bars: false,
            series_indices: Vec::new(),
            dimmed: false,
        }
    }

    /// Create a legend item for a histogram series with no edge stroke
    pub fn histogram(label: impl Into<String>, color: Color) -> Self {
        Self::histogram_with_edge(label, color, None)
    }

    /// Create a legend item for a histogram series with an explicit edge
    ///
    /// `edge` is `(colour, width_in_points)`, normally the value returned by
    /// `HistogramData::resolved_edge`.
    pub fn histogram_with_edge(
        label: impl Into<String>,
        color: Color,
        edge: Option<(Color, f32)>,
    ) -> Self {
        Self {
            label: label.into(),
            color,
            item_type: LegendItemType::Histogram { edge },
            has_error_bars: false,
            series_indices: Vec::new(),
            dimmed: false,
        }
    }

    /// Create a legend item for an area series
    pub fn area(label: impl Into<String>, color: Color, edge_color: Option<Color>) -> Self {
        Self {
            label: label.into(),
            color,
            item_type: LegendItemType::Area { edge_color },
            has_error_bars: false,
            series_indices: Vec::new(),
            dimmed: false,
        }
    }

    /// Create a legend item for error bars
    pub fn error_bar(label: impl Into<String>, color: Color) -> Self {
        Self {
            label: label.into(),
            color,
            item_type: LegendItemType::ErrorBar,
            has_error_bars: true, // Error bar type always has error bars
            series_indices: Vec::new(),
            dimmed: false,
        }
    }

    /// Create from old (label, color) tuple for backward compatibility
    pub fn from_tuple(label: String, color: Color) -> Self {
        // Default to bar-style (filled square) for backward compatibility
        Self {
            label,
            color,
            item_type: LegendItemType::Bar { edge: None },
            has_error_bars: false,
            series_indices: Vec::new(),
            dimmed: false,
        }
    }

    /// Set whether this legend item should show error bars
    /// A copy with every color's alpha scaled, used to draw the entry of a
    /// hidden series faded while keeping its shape recognizable.
    pub fn faded(&self, factor: f32) -> Self {
        let fade_edge = |edge: Option<(Color, f32)>| edge.map(|(c, w)| (c.scale_alpha(factor), w));
        let mut item = self.clone();
        item.color = item.color.scale_alpha(factor);
        item.item_type = match item.item_type {
            LegendItemType::Line { style, width } => LegendItemType::Line { style, width },
            LegendItemType::Scatter { marker, size, edge } => LegendItemType::Scatter {
                marker,
                size,
                edge: fade_edge(edge),
            },
            LegendItemType::LineMarker {
                line_style,
                line_width,
                marker,
                marker_size,
                marker_edge,
            } => LegendItemType::LineMarker {
                line_style,
                line_width,
                marker,
                marker_size,
                marker_edge: fade_edge(marker_edge),
            },
            LegendItemType::Bar { edge } => LegendItemType::Bar {
                edge: fade_edge(edge),
            },
            LegendItemType::Area { edge_color } => LegendItemType::Area {
                edge_color: edge_color.map(|c| c.scale_alpha(factor)),
            },
            LegendItemType::Histogram { edge } => LegendItemType::Histogram {
                edge: fade_edge(edge),
            },
            LegendItemType::ErrorBar => LegendItemType::ErrorBar,
        };
        item
    }

    pub fn with_error_bars(mut self, has_error_bars: bool) -> Self {
        self.has_error_bars = has_error_bars;
        self
    }
}

// ============================================================================
// Legacy Legend Swatch Contour
// ============================================================================

/// Dark neutral used to outline a light swatch on a legacy legend panel.
pub(crate) const LEGACY_LEGEND_SWATCH_EDGE_DARK: Color = Color::from_gray(64);
/// Light neutral used to outline a dark swatch on a legacy legend panel.
pub(crate) const LEGACY_LEGEND_SWATCH_EDGE_LIGHT: Color = Color::from_gray(224);
/// Width of the legacy legend swatch outline, in points.
pub(crate) const LEGACY_LEGEND_SWATCH_EDGE_WIDTH_PT: f32 = 0.8;

/// Pick a neutral outline that keeps a legacy legend swatch discernible.
///
/// The legacy `draw_legend*` panels are painted near-white by both the raster
/// and the SVG backend, so a white or near-white series colour has no contour
/// of its own and the key disappears. The swatch fill is never modified — a
/// legend key must reproduce the series colour exactly — so the contour is
/// supplied as a separate neutral stroke, chosen dark for light fills and
/// light for dark fills.
///
/// `fill` is composited over the near-white panel first, so a translucent or
/// fully transparent fill is treated as the light colour it actually renders
/// as. Shared by both backends so a PNG and an SVG legend cannot disagree.
pub(crate) fn legacy_legend_swatch_edge(fill: Color) -> Color {
    let alpha = fill.a as f32 / 255.0;
    let over_panel = |channel: u8| channel as f32 * alpha + 255.0 * (1.0 - alpha);
    // Rec. 601 luma, good enough to split "light" from "dark".
    let luma = 0.299 * over_panel(fill.r) + 0.587 * over_panel(fill.g) + 0.114 * over_panel(fill.b);

    if luma > 128.0 {
        LEGACY_LEGEND_SWATCH_EDGE_DARK
    } else {
        LEGACY_LEGEND_SWATCH_EDGE_LIGHT
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
    pub fn to_pixels(self, font_size: f32) -> LegendSpacingPixels {
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
// Legend Style (matplotlib-compatible)
// ============================================================================

/// Legend styling configuration (matplotlib-compatible)
///
/// Provides comprehensive legend frame styling matching matplotlib/seaborn defaults.
/// This replaces the old `LegendFrame` struct with enhanced functionality.
///
/// # matplotlib Compatibility
///
/// | matplotlib rcParam | LegendStyle field | Default |
/// |-------------------|-------------------|---------|
/// | `legend.frameon` | `visible` | `true` |
/// | `legend.framealpha` | `alpha` | `0.8` |
/// | `legend.facecolor` | `face_color` | `WHITE` |
/// | `legend.edgecolor` | `edge_color` | `#CCCCCC` |
/// | `legend.fancybox` | `fancy_box` | `true` |
///
/// # Example
///
/// ```rust,ignore
/// use ruviz::core::LegendStyle;
///
/// // Default matplotlib-like style
/// let style = LegendStyle::default();
///
/// // No frame
/// let invisible = LegendStyle::invisible();
///
/// // Custom style
/// let custom = LegendStyle::default()
///     .alpha(0.9)
///     .fancy_box(false);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LegendStyle {
    /// Whether to draw the frame (matplotlib `legend.frameon`)
    pub visible: bool,
    /// Background alpha (matplotlib `legend.framealpha`)
    pub alpha: f32,
    /// Background fill color (matplotlib `legend.facecolor`)
    pub face_color: Color,
    /// Border stroke color (matplotlib `legend.edgecolor`)
    pub edge_color: Option<Color>,
    /// Border line width in points
    pub border_width: f32,
    /// Use rounded corners (matplotlib `legend.fancybox`)
    pub fancy_box: bool,
    /// Corner radius for rounded corners (when fancy_box=true)
    pub corner_radius: f32,
    /// Whether to draw a drop shadow (matplotlib `legend.shadow`)
    pub shadow: bool,
    /// Shadow offset (x, y) in points
    pub shadow_offset: (f32, f32),
    /// Shadow color
    pub shadow_color: Color,
}

impl Default for LegendStyle {
    /// Create default legend style matching matplotlib defaults
    ///
    /// - `visible: true` (frameon)
    /// - `alpha: 0.8` (framealpha)
    /// - `face_color: WHITE` (facecolor)
    /// - `edge_color: #CCCCCC` (edgecolor, matplotlib `.8` gray)
    /// - `fancy_box: true` (fancybox)
    /// - `corner_radius: 4.0` (for fancybox)
    fn default() -> Self {
        Self {
            visible: true,
            alpha: 0.8,
            face_color: Color::WHITE,
            edge_color: Some(Color::from_gray(204)), // #CCCCCC (matplotlib .8)
            border_width: 0.8,
            fancy_box: true,
            corner_radius: 4.0,
            shadow: false,
            shadow_offset: (2.0, -2.0),
            shadow_color: Color::from_rgba(0, 0, 0, 50),
        }
    }
}

impl LegendStyle {
    /// Create a new legend style with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a style with no visible frame
    pub fn invisible() -> Self {
        Self {
            visible: false,
            ..Default::default()
        }
    }

    /// Create a style with rounded corners (fancybox)
    pub fn rounded(radius: f32) -> Self {
        Self {
            fancy_box: true,
            corner_radius: radius,
            ..Default::default()
        }
    }

    /// Create a style with sharp corners (no fancybox)
    pub fn sharp() -> Self {
        Self {
            fancy_box: false,
            corner_radius: 0.0,
            ..Default::default()
        }
    }

    /// Set whether frame is visible
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set background alpha (0.0 = transparent, 1.0 = opaque)
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set background fill color
    pub fn face_color(mut self, color: Color) -> Self {
        self.face_color = color;
        self
    }

    /// Set border stroke color
    pub fn edge_color(mut self, color: Option<Color>) -> Self {
        self.edge_color = color;
        self
    }

    /// Set border line width
    pub fn border_width(mut self, width: f32) -> Self {
        self.border_width = width.max(0.0);
        self
    }

    /// Set whether to use rounded corners (fancybox)
    pub fn fancy_box(mut self, enabled: bool) -> Self {
        self.fancy_box = enabled;
        self
    }

    /// Set corner radius (used when fancy_box=true)
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius.max(0.0);
        self
    }

    /// Set whether to draw shadow
    pub fn shadow(mut self, enabled: bool) -> Self {
        self.shadow = enabled;
        self
    }

    /// Get effective corner radius (0 if fancy_box is false)
    pub fn effective_corner_radius(&self) -> f32 {
        if self.fancy_box {
            self.corner_radius
        } else {
            0.0
        }
    }

    /// Get effective background color with alpha applied
    pub fn effective_face_color(&self) -> Color {
        self.face_color.with_alpha(self.alpha)
    }
}

/// Type alias for backward compatibility
#[deprecated(since = "0.2.0", note = "Use LegendStyle instead")]
pub type LegendFrame = LegendStyle;

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
    /// Style configuration (matplotlib-compatible)
    pub style: LegendStyle,
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
            style: LegendStyle::default(),
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

    /// Configure legend style
    pub fn style(mut self, style: LegendStyle) -> Self {
        self.style = style;
        self
    }

    /// Configure frame styling (deprecated, use `style()` instead)
    #[deprecated(since = "0.2.0", note = "Use style() instead")]
    pub fn frame(mut self, style: LegendStyle) -> Self {
        self.style = style;
        self
    }

    /// Configure spacing
    pub fn spacing(mut self, spacing: LegendSpacing) -> Self {
        self.spacing = spacing;
        self
    }

    /// Return a copy with point-based legend fields scaled for renderer pixels.
    pub(crate) fn scaled_for_render(&self, render_scale: RenderScale) -> Self {
        let mut scaled = self.clone();
        scaled.font_size = render_scale.points_to_pixels(self.font_size);
        scaled.style.border_width = render_scale.points_to_pixels(self.style.border_width);
        scaled.style.corner_radius = render_scale.points_to_pixels(self.style.corner_radius);
        scaled.style.shadow_offset = (
            render_scale.points_to_pixels(self.style.shadow_offset.0),
            render_scale.points_to_pixels(self.style.shadow_offset.1),
        );
        scaled
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
// Occupancy grid for `LegendPosition::Best`
// ============================================================================

/// Number of cells per axis in a [`LegendOccupancy`] grid.
pub const LEGEND_OCCUPANCY_RESOLUTION: usize = 6;

/// Coarse map of where the data actually is, used to place a `Best` legend.
///
/// matplotlib scores every candidate legend box against every artist's bounding
/// box. Doing that literally for a million-point scatter is not affordable, so
/// the caller bins the points it has *already projected to screen space* into a
/// fixed [`LEGEND_OCCUPANCY_RESOLUTION`]² grid and the legend is scored against
/// the occupied cells instead. The cost is bounded by the grid, not the data.
///
/// An empty grid scores every candidate at zero overlap, which is exactly the
/// behaviour of passing no data at all: `Best` falls back to `UpperRight`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LegendOccupancy {
    cells: Vec<(f32, f32, f32, f32)>,
}

impl LegendOccupancy {
    /// Bin already-projected screen points into the grid covering `plot_area`.
    ///
    /// `plot_area` is `(left, top, right, bottom)` in pixels. Points outside it
    /// and non-finite points are ignored; a degenerate plot area yields an
    /// empty grid.
    pub fn from_screen_points<I>(plot_area: (f32, f32, f32, f32), points: I) -> Self
    where
        I: IntoIterator<Item = (f32, f32)>,
    {
        let (left, top, right, bottom) = plot_area;
        let width = right - left;
        let height = bottom - top;
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Self::default();
        }

        let resolution = LEGEND_OCCUPANCY_RESOLUTION;
        let mut occupied = vec![false; resolution * resolution];
        let cell_width = width / resolution as f32;
        let cell_height = height / resolution as f32;

        for (x, y) in points {
            if !x.is_finite() || !y.is_finite() || x < left || x > right || y < top || y > bottom {
                continue;
            }
            let column = (((x - left) / cell_width) as usize).min(resolution - 1);
            let row = (((y - top) / cell_height) as usize).min(resolution - 1);
            occupied[row * resolution + column] = true;
        }

        let cells = occupied
            .iter()
            .enumerate()
            .filter(|(_, is_occupied)| **is_occupied)
            .map(|(index, _)| {
                let column = index % resolution;
                let row = index / resolution;
                let cell_left = left + column as f32 * cell_width;
                let cell_top = top + row as f32 * cell_height;
                (
                    cell_left,
                    cell_top,
                    cell_left + cell_width,
                    cell_top + cell_height,
                )
            })
            .collect();

        Self { cells }
    }

    /// The occupied cells, as `(left, top, right, bottom)` boxes.
    pub fn boxes(&self) -> &[(f32, f32, f32, f32)] {
        &self.cells
    }

    /// Whether no cell is occupied.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

// ============================================================================
// One legend layout, shared by every backend
// ============================================================================

/// Screen geometry for one legend entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegendEntryLayout {
    /// Index into the `items` slice [`layout_legend`] was called with.
    pub item_index: usize,
    /// Left edge of the handle (line segment, marker, or patch).
    pub handle_x: f32,
    /// Vertical centre of the handle.
    pub handle_center_y: f32,
    /// Left edge of the label text.
    pub label_x: f32,
    /// Top of the label text box, for a top-anchored text call.
    pub label_top_y: f32,
}

/// Screen geometry for the optional legend title.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegendTitleLayout {
    /// Horizontal centre of the title.
    pub center_x: f32,
    /// Top of the title text box, for a top-anchored text call.
    pub top_y: f32,
}

/// Everything a backend needs to draw a legend: the frame and its contents.
///
/// Produced by [`layout_legend`], which is the only place legend geometry is
/// computed. The same value answers "how much room does this legend need"
/// (via [`LegendLayout::size`]) and "where does every glyph go", so a legend
/// can no longer be *reserved* for one box and *drawn* in another.
#[derive(Debug, Clone, PartialEq)]
pub struct LegendLayout {
    /// Left edge of the frame.
    pub x: f32,
    /// Top edge of the frame.
    pub y: f32,
    /// Frame width.
    pub width: f32,
    /// Frame height.
    pub height: f32,
    /// The position actually used — `Best` is already resolved to a concrete corner.
    pub position: LegendPosition,
    /// Spacing in pixels, so backends never recompute it from the font size.
    pub spacing: LegendSpacingPixels,
    /// Font size in pixels for labels and title.
    pub font_size: f32,
    /// Title geometry, if the legend has a title.
    pub title: Option<LegendTitleLayout>,
    /// One entry per item that fits inside the frame, in draw order.
    pub entries: Vec<LegendEntryLayout>,
    /// Horizontal room one entry occupies (handle through label), used to
    /// derive per-entry hit rectangles.
    pub entry_width: f32,
}

/// Alpha factor both backends draw a dimmed (hidden-series) legend entry at.
pub const DIMMED_LEGEND_ALPHA: f32 = 0.35;

/// Clickable screen region of one legend entry, kept by the interactive
/// session so pointer positions can be resolved back to series.
#[derive(Debug, Clone, PartialEq)]
pub struct LegendHitRegion {
    /// `(x, y, width, height)` in the same device pixels as the render target.
    pub rect: (f32, f32, f32, f32),
    /// The plot series this entry stands for; empty for annotation entries.
    pub series_indices: Vec<usize>,
}

impl LegendLayout {
    /// Frame size as `(width, height)`.
    pub fn size(&self) -> (f32, f32) {
        (self.width, self.height)
    }

    /// Clickable rectangle of one entry: the full row band from the handle's
    /// left edge through the entry's column width.
    pub fn entry_hit_rect(&self, entry: &LegendEntryLayout) -> (f32, f32, f32, f32) {
        let row_pitch = self.font_size + self.spacing.label_spacing;
        (
            entry.handle_x,
            entry.handle_center_y - row_pitch / 2.0,
            self.entry_width,
            row_pitch,
        )
    }

    /// Frame bounds as `(left, top, right, bottom)`.
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.x + self.width, self.y + self.height)
    }
}

/// Where the legend may go, beyond what [`Legend::position`] says.
#[derive(Debug, Clone, Copy, Default)]
pub struct LegendPlacement<'a> {
    /// A rectangle the figure layout has already reserved, as
    /// `(left, top, right, bottom)`. When present it wins over
    /// [`Legend::position`], and entries that would spill past its bottom
    /// padding are dropped rather than drawn over the plot.
    pub reserved: Option<(f32, f32, f32, f32)>,
    /// Where the data is, for [`LegendPosition::Best`]. `None` means "unknown",
    /// which makes `Best` degrade to `UpperRight`.
    pub occupancy: Option<&'a LegendOccupancy>,
}

/// Rough advance width of a label, for callers with no text engine.
///
/// The 3D layout runs before any renderer exists, so it cannot measure text.
/// This is the fallback — but it counts `char`s, not bytes, and counts East
/// Asian wide glyphs double, because those are about two Latin advances wide.
/// A byte count would size a CJK legend for three times its width and a Latin
/// accented one for twice; a plain `char` count would size CJK for half.
pub fn estimated_label_width(label: &str, font_size: f32) -> f32 {
    // Mean advance of a Latin glyph as a fraction of the font size.
    const AVERAGE_ADVANCE_RATIO: f32 = 0.58;

    let columns: f32 = label
        .chars()
        .map(|character| if is_wide_glyph(character) { 2.0 } else { 1.0 })
        .sum();
    columns * font_size * AVERAGE_ADVANCE_RATIO
}

/// Whether a character takes roughly two Latin advances.
///
/// Covers Hangul jamo and syllables, CJK radicals and ideographs (including
/// extensions A and B+), kana, Yi, CJK compatibility forms, fullwidth forms
/// and the common emoji blocks — the East Asian "Wide"/"Fullwidth" classes,
/// approximated with whole blocks so no table is needed.
fn is_wide_glyph(character: char) -> bool {
    matches!(
        character,
        '\u{1100}'..='\u{115F}'
            | '\u{2E80}'..='\u{303E}'
            | '\u{3041}'..='\u{33FF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{A000}'..='\u{A4CF}'
            | '\u{AC00}'..='\u{D7A3}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{FE30}'..='\u{FE6F}'
            | '\u{FF00}'..='\u{FF60}'
            | '\u{FFE0}'..='\u{FFE6}'
            | '\u{1F300}'..='\u{1F64F}'
            | '\u{1F900}'..='\u{1F9FF}'
            | '\u{20000}'..='\u{2FFFD}'
    )
}

/// Content size of a legend, before it is placed.
///
/// Returns `(width, height, entry_width)` where `entry_width` is the width of
/// one handle-plus-label column. Private on purpose: every caller goes through
/// [`measure_legend_size`] or [`layout_legend`] so the reservation and the
/// drawing can never be computed by two different formulas again.
fn legend_content_size(
    items: &[LegendItem],
    legend: &Legend,
    measure: &mut dyn FnMut(&str) -> Result<f32>,
) -> Result<(f32, f32, f32)> {
    let spacing = legend.spacing.to_pixels(legend.font_size);
    let columns = legend.columns.max(1);
    let rows = items.len().div_ceil(columns);

    let mut max_label_width = 0.0_f32;
    for item in items {
        max_label_width = max_label_width.max(measure(&item.label)?);
    }

    let entry_width = spacing.handle_length + spacing.handle_text_pad + max_label_width;
    let content_width =
        entry_width * columns as f32 + columns.saturating_sub(1) as f32 * spacing.column_spacing;
    // Rows are spaced *between* entries, not after the last one, so the top and
    // bottom padding inside the frame stay equal.
    let content_height =
        rows as f32 * legend.font_size + rows.saturating_sub(1) as f32 * spacing.label_spacing;

    let (title_width, title_height) = match legend.title.as_deref() {
        Some(title) => (measure(title)?, legend.font_size + spacing.label_spacing),
        None => (0.0, 0.0),
    };

    Ok((
        content_width.max(title_width) + spacing.border_pad * 2.0,
        content_height + title_height + spacing.border_pad * 2.0,
        entry_width,
    ))
}

/// The room this legend needs, in pixels, without placing it.
///
/// `measure` returns the rendered width of a string at [`Legend::font_size`];
/// each backend passes its own so a Typst-shaped label and a cosmic-text one
/// stay honestly different while the *layout* stays identical.
///
/// `legend` must already be scaled for the render target (see
/// `Legend::scaled_for_render`), because every value here is in pixels.
pub fn measure_legend_size(
    items: &[LegendItem],
    legend: &Legend,
    mut measure: impl FnMut(&str) -> Result<f32>,
) -> Result<(f32, f32)> {
    let (width, height, _) = legend_content_size(items, legend, &mut measure)?;
    Ok((width, height))
}

/// Lay out a legend: size it, place it, and place everything inside it.
///
/// This is the single legend layout in the crate. Both raster and SVG output,
/// the figure-level margin reservation and the 3D overlay all call it, so a
/// legend cannot be measured by one formula and drawn by another — which is
/// exactly how the byte-counted `label.len()` sizing used to disagree with the
/// text the renderer actually drew.
///
/// * `plot_area` is `(left, top, right, bottom)` in pixels.
/// * `measure` returns the rendered width of a string at [`Legend::font_size`].
/// * `legend` must already be scaled for the render target.
pub fn layout_legend(
    items: &[LegendItem],
    legend: &Legend,
    plot_area: (f32, f32, f32, f32),
    placement: LegendPlacement<'_>,
    mut measure: impl FnMut(&str) -> Result<f32>,
) -> Result<LegendLayout> {
    let spacing = legend.spacing.to_pixels(legend.font_size);
    let columns = legend.columns.max(1);
    let rows = items.len().div_ceil(columns);

    let (natural_width, natural_height, entry_width) =
        legend_content_size(items, legend, &mut measure)?;

    let natural = (natural_width, natural_height);
    let occupied: &[(f32, f32, f32, f32)] = match placement.occupancy {
        Some(occupancy) => occupancy.boxes(),
        None => &[],
    };
    let position = match legend.position {
        // A reserved rectangle already decided where the legend goes.
        LegendPosition::Best if placement.reserved.is_none() => find_best_position(
            natural,
            plot_area,
            occupied,
            &legend.spacing,
            legend.font_size,
        ),
        explicit => explicit,
    };

    let (x, y, width, height) = match placement.reserved {
        Some((left, top, right, bottom)) => (left, top, right - left, bottom - top),
        None => {
            let placed = Legend {
                position,
                ..legend.clone()
            };
            let (x, y) = placed.calculate_position(natural, plot_area);
            (x, y, natural_width, natural_height)
        }
    };

    // A reserved rectangle can be capped by the figure layout, so a column is
    // whatever room it actually left; a free-standing legend was sized to fit.
    let column_width = if placement.reserved.is_some() {
        let gutters = columns.saturating_sub(1) as f32 * spacing.column_spacing;
        (width - spacing.border_pad * 2.0 - gutters) / columns as f32
    } else {
        entry_width
    };
    // A reserved band can squeeze the column below the natural entry width,
    // but labels are drawn unclipped past the column edge. A single column
    // keeps its full visible extent clickable; with several columns the hit
    // width must stop at the column, or it would claim the neighbor's entry.
    let entry_hit_width = if columns == 1 {
        column_width.max(entry_width)
    } else {
        column_width
    };

    let mut row_center_y = y + spacing.border_pad + legend.font_size / 2.0;
    let title = legend.title.is_some().then(|| LegendTitleLayout {
        center_x: x + width / 2.0,
        top_y: row_center_y,
    });
    if title.is_some() {
        row_center_y += legend.font_size + spacing.label_spacing;
    }

    let max_center_y = match placement.reserved {
        Some((_, _, _, bottom)) => bottom - spacing.border_pad,
        None => f32::INFINITY,
    };

    let mut entries = Vec::with_capacity(items.len());
    for column in 0..columns {
        let handle_x =
            x + spacing.border_pad + column as f32 * (column_width + spacing.column_spacing);
        let mut center_y = row_center_y;

        for row in 0..rows {
            let item_index = column * rows + row;
            if item_index >= items.len() || center_y > max_center_y {
                break;
            }
            entries.push(LegendEntryLayout {
                item_index,
                handle_x,
                handle_center_y: center_y,
                label_x: handle_x + spacing.handle_length + spacing.handle_text_pad,
                label_top_y: center_y - legend.font_size * 0.65,
            });
            center_y += legend.font_size + spacing.label_spacing;
        }
    }

    Ok(LegendLayout {
        x,
        y,
        width,
        height,
        position,
        spacing,
        font_size: legend.font_size,
        title,
        entries,
        entry_width: entry_hit_width,
    })
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
    fn test_patch_items_default_to_no_edge() {
        assert!(matches!(
            LegendItem::bar("bars", Color::BLUE).item_type,
            LegendItemType::Bar { edge: None }
        ));
        assert!(matches!(
            LegendItem::histogram("hist", Color::BLUE).item_type,
            LegendItemType::Histogram { edge: None }
        ));
        assert!(matches!(
            LegendItem::from_tuple("legacy".to_string(), Color::BLUE).item_type,
            LegendItemType::Bar { edge: None }
        ));
    }

    #[test]
    fn test_patch_items_carry_edge_colour_and_point_width() {
        let edge = (Color::BLACK, 0.8);

        let bar = LegendItem::bar_with_edge("bars", Color::BLUE, Some(edge));
        match &bar.item_type {
            LegendItemType::Bar { edge: Some(e) } => {
                assert_eq!(e.0, Color::BLACK);
                assert!((e.1 - 0.8).abs() < f32::EPSILON);
            }
            other => panic!("expected Bar with edge, got {other:?}"),
        }
        // The fill must stay exactly the series colour.
        assert_eq!(bar.color, Color::BLUE);

        let hist = LegendItem::histogram_with_edge("hist", Color::RED, Some(edge));
        match &hist.item_type {
            LegendItemType::Histogram { edge: Some(e) } => {
                assert_eq!(e.0, Color::BLACK);
                assert!((e.1 - 0.8).abs() < f32::EPSILON);
            }
            other => panic!("expected Histogram with edge, got {other:?}"),
        }
        assert_eq!(hist.color, Color::RED);
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

    /// Every test below measures a label as 6px per `char`, which is enough to
    /// tell "sized for the text" from "sized for the bytes".
    fn six_px_per_char(text: &str) -> Result<f32> {
        Ok(text.chars().count() as f32 * 6.0)
    }

    fn two_line_items() -> Vec<LegendItem> {
        vec![
            LegendItem::line("sin(x)", Color::BLUE, LineStyle::Solid, 1.5),
            LegendItem::line("cos(x)", Color::RED, LineStyle::Dashed, 1.5),
        ]
    }

    #[test]
    fn test_legend_size_calculation() {
        let legend = Legend::new();
        let (width, height) =
            measure_legend_size(&two_line_items(), &legend, six_px_per_char).expect("measure");
        assert!(width > 0.0);
        assert!(height > 0.0);
    }

    /// The bug this whole layout exists to kill: the legend was sized from
    /// `label.len()`, a **byte** count, so a CJK or accented label reserved a
    /// box that had nothing to do with the text drawn into it.
    ///
    /// With one measured layout, two labels that render the same width must
    /// produce the same frame no matter how many bytes they occupy.
    #[test]
    fn layout_is_sized_from_measured_text_not_bytes() {
        let legend = Legend::new();
        // 3 chars / 3 bytes vs 3 chars / 9 bytes.
        let ascii = vec![LegendItem::line("abc", Color::BLUE, LineStyle::Solid, 1.5)];
        let cjk = vec![LegendItem::line(
            "日本語",
            Color::BLUE,
            LineStyle::Solid,
            1.5,
        )];

        let ascii_size = measure_legend_size(&ascii, &legend, six_px_per_char).expect("ascii");
        let cjk_size = measure_legend_size(&cjk, &legend, six_px_per_char).expect("cjk");

        assert_eq!(ascii_size, cjk_size);
        // And a byte count would have been three times as wide.
        let bytes = "日本語".len() as f32;
        assert!(bytes > "日本語".chars().count() as f32);
    }

    /// The reservation and the drawing are the same call, so the frame the
    /// figure reserves is exactly the frame the entries are laid out in.
    #[test]
    fn reserved_size_matches_drawn_layout() {
        let legend = Legend {
            enabled: true,
            position: LegendPosition::UpperRight,
            title: Some("series".to_string()),
            ..Default::default()
        };
        let items = two_line_items();

        let measured = measure_legend_size(&items, &legend, six_px_per_char).expect("measure");
        let layout = layout_legend(
            &items,
            &legend,
            (0.0, 0.0, 400.0, 300.0),
            LegendPlacement::default(),
            six_px_per_char,
        )
        .expect("layout");

        assert_eq!(measured, layout.size());
        // Every entry sits inside the frame it was measured for.
        let (left, top, right, bottom) = layout.bounds();
        for entry in &layout.entries {
            assert!(entry.handle_x >= left, "{entry:?}");
            assert!(entry.label_x <= right, "{entry:?}");
            assert!(entry.handle_center_y >= top, "{entry:?}");
            assert!(entry.handle_center_y <= bottom, "{entry:?}");
        }
    }

    /// A long title used to be drawn straight out of the frame, because the
    /// drawing formula ignored the title width the reservation accounted for.
    #[test]
    fn title_widens_the_frame_when_it_is_the_longest_run() {
        let items = vec![LegendItem::line("a", Color::BLUE, LineStyle::Solid, 1.5)];
        let untitled = Legend::new();
        let titled = Legend {
            title: Some("a very long legend title".to_string()),
            ..Legend::new()
        };

        let (narrow, _) = measure_legend_size(&items, &untitled, six_px_per_char).expect("narrow");
        let (wide, _) = measure_legend_size(&items, &titled, six_px_per_char).expect("wide");

        assert!(wide > narrow, "narrow = {narrow}, wide = {wide}");
    }

    #[test]
    fn layout_places_every_item_in_column_major_order() {
        let legend = Legend {
            columns: 2,
            ..Legend::new()
        };
        let items: Vec<_> = (0..4)
            .map(|index| LegendItem::line(format!("s{index}"), Color::BLUE, LineStyle::Solid, 1.0))
            .collect();

        let layout = layout_legend(
            &items,
            &legend,
            (0.0, 0.0, 400.0, 300.0),
            LegendPlacement::default(),
            six_px_per_char,
        )
        .expect("layout");

        assert_eq!(layout.entries.len(), 4);
        assert_eq!(
            layout
                .entries
                .iter()
                .map(|entry| entry.item_index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        // Two columns, so entries 0/1 share a handle column and 2/3 the next.
        assert_eq!(layout.entries[0].handle_x, layout.entries[1].handle_x);
        assert!(layout.entries[2].handle_x > layout.entries[0].handle_x);
    }

    /// A capped reservation must drop the rows it cannot hold instead of
    /// painting them over the plot.
    #[test]
    fn reserved_rectangle_clips_rows_that_do_not_fit() {
        let legend = Legend::new();
        let items: Vec<_> = (0..6)
            .map(|index| LegendItem::line(format!("s{index}"), Color::BLUE, LineStyle::Solid, 1.0))
            .collect();

        let layout = layout_legend(
            &items,
            &legend,
            (0.0, 0.0, 400.0, 300.0),
            LegendPlacement {
                reserved: Some((10.0, 10.0, 120.0, 60.0)),
                occupancy: None,
            },
            six_px_per_char,
        )
        .expect("layout");

        assert_eq!(layout.bounds(), (10.0, 10.0, 120.0, 60.0));
        assert!(layout.entries.len() < items.len());
        for entry in &layout.entries {
            assert!(entry.handle_center_y <= 60.0);
        }
    }

    /// A reserved band narrower than the natural entry width draws labels
    /// past the column edge, so a single column's clickable width must cover
    /// the full visible extent rather than stopping at the squeezed column.
    #[test]
    fn squeezed_single_column_keeps_the_visible_label_extent_clickable() {
        let legend = Legend::new();
        let items = vec![LegendItem::line(
            "a rather long series label",
            Color::BLUE,
            LineStyle::Solid,
            1.0,
        )];

        let free = layout_legend(
            &items,
            &legend,
            (0.0, 0.0, 400.0, 300.0),
            LegendPlacement::default(),
            six_px_per_char,
        )
        .expect("free layout");
        let squeezed = layout_legend(
            &items,
            &legend,
            (0.0, 0.0, 400.0, 300.0),
            LegendPlacement {
                reserved: Some((10.0, 10.0, 60.0, 100.0)),
                occupancy: None,
            },
            six_px_per_char,
        )
        .expect("squeezed layout");

        assert!(
            squeezed.width < free.entry_width,
            "the reservation must actually squeeze the column for this test"
        );
        assert_eq!(
            squeezed.entry_width, free.entry_width,
            "a single squeezed column keeps the natural clickable width"
        );
    }

    #[test]
    fn occupancy_grid_bins_projected_points() {
        let plot_area = (0.0, 0.0, 60.0, 60.0);
        // All points in the top-left cell, plus junk that must be ignored.
        let grid = LegendOccupancy::from_screen_points(
            plot_area,
            [
                (1.0, 1.0),
                (5.0, 5.0),
                (-10.0, 5.0),
                (5.0, f32::NAN),
                (1000.0, 1000.0),
            ],
        );

        assert_eq!(grid.boxes().to_vec(), vec![(0.0, 0.0, 10.0, 10.0)]);
        assert!(!grid.is_empty());
        assert!(LegendOccupancy::from_screen_points((0.0, 0.0, 0.0, 0.0), [(1.0, 1.0)]).is_empty());
    }

    /// `Best` used to be the default and always answer `UpperRight`, because
    /// every caller handed it nothing to avoid. With an occupancy grid it
    /// actually moves out of the way of the data.
    #[test]
    fn best_position_avoids_the_occupied_corner() {
        let plot_area = (0.0, 0.0, 600.0, 400.0);
        let legend = Legend {
            enabled: true,
            position: LegendPosition::Best,
            ..Default::default()
        };
        let items = two_line_items();

        // Data packed into the upper-right quadrant, as a filled block rather
        // than a diagonal: the grid is coarse, so a diagonal from (400, 10) to
        // (596, 157) leaves the top-right *cell* empty and `UpperRight` would
        // honestly score zero overlap.
        let points: Vec<(f32, f32)> = (0..30)
            .flat_map(|column| {
                (0..30).map(move |row| (305.0 + column as f32 * 10.0, 5.0 + row as f32 * 6.5))
            })
            .collect();
        let occupancy = LegendOccupancy::from_screen_points(plot_area, points);

        let avoided = layout_legend(
            &items,
            &legend,
            plot_area,
            LegendPlacement {
                reserved: None,
                occupancy: Some(&occupancy),
            },
            six_px_per_char,
        )
        .expect("layout");
        assert_ne!(avoided.position, LegendPosition::UpperRight);

        // No occupancy is still the documented fallback.
        let blind = layout_legend(
            &items,
            &legend,
            plot_area,
            LegendPlacement::default(),
            six_px_per_char,
        )
        .expect("layout");
        assert_eq!(blind.position, LegendPosition::UpperRight);
    }

    #[test]
    fn estimated_label_width_counts_wide_glyphs_double() {
        let latin = estimated_label_width("abc", 10.0);
        let cjk = estimated_label_width("日本語", 10.0);
        assert!(
            (cjk - latin * 2.0).abs() < 1e-4,
            "latin = {latin}, cjk = {cjk}"
        );
        // A byte count would have made it three times, a char count one times.
        assert!(cjk > latin);
    }

    #[test]
    fn test_scaled_for_render_scales_point_fields() {
        let legend = Legend {
            font_size: 12.0,
            style: LegendStyle {
                border_width: 1.5,
                corner_radius: 3.0,
                shadow_offset: (2.0, 4.0),
                ..Default::default()
            },
            ..Default::default()
        };

        let scaled = legend.scaled_for_render(RenderScale::new(144.0));

        assert!((scaled.font_size - 24.0).abs() < 0.001);
        assert!((scaled.style.border_width - 3.0).abs() < 0.001);
        assert!((scaled.style.corner_radius - 6.0).abs() < 0.001);
        assert!((scaled.style.shadow_offset.0 - 4.0).abs() < 0.001);
        assert!((scaled.style.shadow_offset.1 - 8.0).abs() < 0.001);
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

    /// `Position::custom(0.0, 0.0)` means "top-left of the plot area".
    /// After conversion it must land on exactly the same pixel as
    /// `LegendPosition::UpperLeft`.
    ///
    /// Regression test: `from_position` used to copy `y` straight through,
    /// but `Legend::calculate_position` computes `top + (1.0 - y) * height`
    /// for `Custom`, so the top-left corner rendered at the bottom left.
    #[test]
    #[allow(deprecated)]
    fn custom_top_left_matches_upper_left_pixel() {
        let plot_area = (100.0, 50.0, 700.0, 450.0);
        let legend_size = (120.0, 80.0);
        // `UpperLeft` insets by `border_axes_pad`, `Custom` anchors exactly;
        // zero the pad so the two are directly comparable.
        let spacing = LegendSpacing {
            border_axes_pad: 0.0,
            ..Default::default()
        };

        let upper_left = Legend {
            position: LegendPosition::UpperLeft,
            spacing,
            ..Default::default()
        }
        .calculate_position(legend_size, plot_area);

        let converted = LegendPosition::from(Position::custom(0.0, 0.0));
        let custom = Legend {
            position: converted,
            spacing,
            ..Default::default()
        }
        .calculate_position(legend_size, plot_area);

        assert_eq!(custom, upper_left);
        assert_eq!(custom, (100.0, 50.0));
    }

    #[test]
    #[allow(deprecated)]
    fn custom_position_stays_in_the_upper_half() {
        // `Position::custom(0.1, 0.05)` is 5% down from the top; it used to
        // render at 95% down, i.e. bottom-left.
        let plot_area = (0.0, 0.0, 400.0, 200.0);
        let legend = Legend {
            position: LegendPosition::from(Position::custom(0.1, 0.05)),
            spacing: LegendSpacing {
                border_axes_pad: 0.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let (x, y) = legend.calculate_position((50.0, 20.0), plot_area);
        assert!((x - 40.0).abs() < 1e-4, "x = {x}");
        assert!((y - 10.0).abs() < 1e-4, "y = {y}");
    }

    #[test]
    #[allow(deprecated)]
    fn from_position_maps_every_named_variant() {
        let cases = [
            (Position::Best, LegendPosition::Best),
            (Position::TopLeft, LegendPosition::UpperLeft),
            (Position::TopCenter, LegendPosition::UpperCenter),
            (Position::TopRight, LegendPosition::UpperRight),
            (Position::CenterLeft, LegendPosition::CenterLeft),
            (Position::Center, LegendPosition::Center),
            (Position::CenterRight, LegendPosition::CenterRight),
            (Position::BottomLeft, LegendPosition::LowerLeft),
            (Position::BottomCenter, LegendPosition::LowerCenter),
            (Position::BottomRight, LegendPosition::LowerRight),
        ];

        for (old, new) in cases {
            assert_eq!(LegendPosition::from(old), new, "mapping {old}");
        }
    }
}
