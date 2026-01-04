//! Annotation types for adding text, arrows, lines, and shapes to plots
//!
//! Annotations are overlay elements rendered after data series.
//! They allow users to highlight specific data points, add labels,
//! draw reference lines, and mark regions of interest.
//!
//! # Example
//!
//! ```rust,ignore
//! use ruviz::{Plot, Annotation, TextStyle, ArrowStyle};
//!
//! Plot::new()
//!     .line(&x, &y)
//!     .text(2.5, 100.0, "Peak value")
//!     .hline(50.0)  // reference line
//!     .arrow(1.0, 80.0, 2.5, 100.0)
//!     .save("annotated.png")?;
//! ```

use crate::render::{Color, LineStyle};

/// Text alignment for annotations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    /// Align text to the left
    Left,
    /// Center text horizontally (default)
    #[default]
    Center,
    /// Align text to the right
    Right,
}

/// Vertical text alignment for annotations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextVAlign {
    /// Align text to the top
    Top,
    /// Center text vertically (default)
    #[default]
    Middle,
    /// Align text to the bottom
    Bottom,
}

/// Style configuration for text annotations
#[derive(Debug, Clone)]
pub struct TextStyle {
    /// Font size in points
    pub font_size: f32,
    /// Text color
    pub color: Color,
    /// Horizontal alignment
    pub align: TextAlign,
    /// Vertical alignment
    pub valign: TextVAlign,
    /// Rotation angle in degrees (counter-clockwise)
    pub rotation: f32,
    /// Background color (None for transparent)
    pub background: Option<Color>,
    /// Background padding in points
    pub padding: f32,
    /// Border color (None for no border)
    pub border_color: Option<Color>,
    /// Border width in points
    pub border_width: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 10.0,
            color: Color::BLACK,
            align: TextAlign::Center,
            valign: TextVAlign::Middle,
            rotation: 0.0,
            background: None,
            padding: 2.0,
            border_color: None,
            border_width: 1.0,
        }
    }
}

impl TextStyle {
    /// Create a new text style with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the font size
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the text color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set horizontal alignment
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Set vertical alignment
    pub fn valign(mut self, valign: TextVAlign) -> Self {
        self.valign = valign;
        self
    }

    /// Set rotation angle in degrees
    pub fn rotation(mut self, degrees: f32) -> Self {
        self.rotation = degrees;
        self
    }

    /// Set background color
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Set padding around text
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set border color and width
    pub fn border(mut self, color: Color, width: f32) -> Self {
        self.border_color = Some(color);
        self.border_width = width;
        self
    }
}

/// Arrow head style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowHead {
    /// No arrow head
    None,
    /// Simple triangle arrow head (default)
    #[default]
    Triangle,
    /// Stealth/barbed arrow head
    Stealth,
    /// Open arrow head (V-shape)
    Open,
}

/// Style configuration for arrow annotations
#[derive(Debug, Clone)]
pub struct ArrowStyle {
    /// Line color
    pub color: Color,
    /// Line width in points
    pub line_width: f32,
    /// Line style (solid, dashed, etc.)
    pub line_style: LineStyle,
    /// Arrow head style at the end point
    pub head_style: ArrowHead,
    /// Arrow head style at the start point
    pub tail_style: ArrowHead,
    /// Arrow head length in points
    pub head_length: f32,
    /// Arrow head width in points
    pub head_width: f32,
}

impl Default for ArrowStyle {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            line_width: 1.0,
            line_style: LineStyle::Solid,
            head_style: ArrowHead::Triangle,
            tail_style: ArrowHead::None,
            head_length: 10.0,
            head_width: 6.0,
        }
    }
}

impl ArrowStyle {
    /// Create a new arrow style with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the arrow color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self
    }

    /// Set the line style
    pub fn line_style(mut self, style: LineStyle) -> Self {
        self.line_style = style;
        self
    }

    /// Set the head style
    pub fn head_style(mut self, style: ArrowHead) -> Self {
        self.head_style = style;
        self
    }

    /// Set the tail style
    pub fn tail_style(mut self, style: ArrowHead) -> Self {
        self.tail_style = style;
        self
    }

    /// Set arrow head dimensions
    pub fn head_size(mut self, length: f32, width: f32) -> Self {
        self.head_length = length;
        self.head_width = width;
        self
    }

    /// Create a double-headed arrow
    pub fn double_headed(mut self) -> Self {
        self.tail_style = self.head_style;
        self
    }
}

/// Style configuration for shape annotations (rectangles, etc.)
#[derive(Debug, Clone)]
pub struct ShapeStyle {
    /// Fill color (None for no fill)
    pub fill_color: Option<Color>,
    /// Fill alpha/opacity (0.0 - 1.0)
    pub fill_alpha: f32,
    /// Edge/border color (None for no border)
    pub edge_color: Option<Color>,
    /// Edge/border width in points
    pub edge_width: f32,
    /// Edge line style
    pub edge_style: LineStyle,
}

impl Default for ShapeStyle {
    fn default() -> Self {
        Self {
            fill_color: None,
            fill_alpha: 0.3,
            edge_color: Some(Color::BLACK),
            edge_width: 1.0,
            edge_style: LineStyle::Solid,
        }
    }
}

impl ShapeStyle {
    /// Create a new shape style with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the fill color
    pub fn fill(mut self, color: Color) -> Self {
        self.fill_color = Some(color);
        self
    }

    /// Set the fill alpha
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.fill_alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set the edge color
    pub fn edge(mut self, color: Color) -> Self {
        self.edge_color = Some(color);
        self
    }

    /// Set the edge width
    pub fn edge_width(mut self, width: f32) -> Self {
        self.edge_width = width;
        self
    }

    /// Set the edge style
    pub fn edge_style(mut self, style: LineStyle) -> Self {
        self.edge_style = style;
        self
    }

    /// Remove the edge/border
    pub fn no_edge(mut self) -> Self {
        self.edge_color = None;
        self
    }

    /// Remove the fill
    pub fn no_fill(mut self) -> Self {
        self.fill_color = None;
        self
    }
}

/// Style configuration for fill_between areas
#[derive(Debug, Clone)]
pub struct FillStyle {
    /// Fill color
    pub color: Color,
    /// Fill alpha/opacity (0.0 - 1.0)
    pub alpha: f32,
    /// Edge color (None for no edge)
    pub edge_color: Option<Color>,
    /// Edge width in points
    pub edge_width: f32,
    /// Hatch pattern (None for solid fill)
    pub hatch: Option<HatchPattern>,
}

impl Default for FillStyle {
    fn default() -> Self {
        Self {
            color: Color::BLUE,
            alpha: 0.3,
            edge_color: None,
            edge_width: 0.0,
            hatch: None,
        }
    }
}

impl FillStyle {
    /// Create a new fill style with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the fill color
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the fill alpha
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set the edge color and width
    pub fn edge(mut self, color: Color, width: f32) -> Self {
        self.edge_color = Some(color);
        self.edge_width = width;
        self
    }

    /// Set the hatch pattern
    pub fn hatch(mut self, pattern: HatchPattern) -> Self {
        self.hatch = Some(pattern);
        self
    }
}

/// Hatch pattern for fills
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HatchPattern {
    /// Diagonal lines from bottom-left to top-right (/)
    Diagonal,
    /// Diagonal lines from top-left to bottom-right (\)
    BackDiagonal,
    /// Horizontal lines (-)
    Horizontal,
    /// Vertical lines (|)
    Vertical,
    /// Cross-hatch (+)
    Cross,
    /// Diagonal cross-hatch (x)
    DiagonalCross,
    /// Dots (.)
    Dots,
}

/// Annotation element that can be added to a plot
///
/// Annotations are rendered after data series and can be used to
/// highlight specific data points, add labels, draw reference lines,
/// or mark regions of interest.
#[derive(Debug, Clone)]
pub enum Annotation {
    /// Text annotation at data coordinates
    Text {
        /// X coordinate in data space
        x: f64,
        /// Y coordinate in data space
        y: f64,
        /// Text content
        text: String,
        /// Text style
        style: TextStyle,
    },

    /// Arrow annotation between two points in data coordinates
    Arrow {
        /// Start X coordinate in data space
        x1: f64,
        /// Start Y coordinate in data space
        y1: f64,
        /// End X coordinate in data space
        x2: f64,
        /// End Y coordinate in data space
        y2: f64,
        /// Arrow style
        style: ArrowStyle,
    },

    /// Horizontal line spanning the entire plot width
    HLine {
        /// Y coordinate in data space
        y: f64,
        /// Line style (color, width, dash pattern)
        style: LineStyle,
        /// Line color
        color: Color,
        /// Line width
        width: f32,
    },

    /// Vertical line spanning the entire plot height
    VLine {
        /// X coordinate in data space
        x: f64,
        /// Line style (color, width, dash pattern)
        style: LineStyle,
        /// Line color
        color: Color,
        /// Line width
        width: f32,
    },

    /// Rectangle annotation in data coordinates
    Rectangle {
        /// Left X coordinate in data space
        x: f64,
        /// Bottom Y coordinate in data space
        y: f64,
        /// Width in data space
        width: f64,
        /// Height in data space
        height: f64,
        /// Shape style
        style: ShapeStyle,
    },

    /// Filled region between two curves
    FillBetween {
        /// X coordinates (shared by both curves)
        x: Vec<f64>,
        /// Y coordinates for the first curve
        y1: Vec<f64>,
        /// Y coordinates for the second curve (or constant baseline)
        y2: Vec<f64>,
        /// Fill style
        style: FillStyle,
        /// Only fill where y1 > y2
        where_positive: bool,
    },

    /// Horizontal span (shaded vertical region)
    HSpan {
        /// Left X coordinate in data space
        x_min: f64,
        /// Right X coordinate in data space
        x_max: f64,
        /// Shape style
        style: ShapeStyle,
    },

    /// Vertical span (shaded horizontal region)
    VSpan {
        /// Bottom Y coordinate in data space
        y_min: f64,
        /// Top Y coordinate in data space
        y_max: f64,
        /// Shape style
        style: ShapeStyle,
    },
}

impl Annotation {
    /// Create a text annotation
    pub fn text(x: f64, y: f64, text: impl Into<String>) -> Self {
        Annotation::Text {
            x,
            y,
            text: text.into(),
            style: TextStyle::default(),
        }
    }

    /// Create a text annotation with custom style
    pub fn text_styled(x: f64, y: f64, text: impl Into<String>, style: TextStyle) -> Self {
        Annotation::Text {
            x,
            y,
            text: text.into(),
            style,
        }
    }

    /// Create an arrow annotation
    pub fn arrow(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Annotation::Arrow {
            x1,
            y1,
            x2,
            y2,
            style: ArrowStyle::default(),
        }
    }

    /// Create an arrow annotation with custom style
    pub fn arrow_styled(x1: f64, y1: f64, x2: f64, y2: f64, style: ArrowStyle) -> Self {
        Annotation::Arrow {
            x1,
            y1,
            x2,
            y2,
            style,
        }
    }

    /// Create a horizontal reference line
    pub fn hline(y: f64) -> Self {
        Annotation::HLine {
            y,
            style: LineStyle::Dashed,
            color: Color::new(128, 128, 128),
            width: 1.0,
        }
    }

    /// Create a horizontal reference line with custom style
    pub fn hline_styled(y: f64, color: Color, width: f32, style: LineStyle) -> Self {
        Annotation::HLine {
            y,
            style,
            color,
            width,
        }
    }

    /// Create a vertical reference line
    pub fn vline(x: f64) -> Self {
        Annotation::VLine {
            x,
            style: LineStyle::Dashed,
            color: Color::new(128, 128, 128),
            width: 1.0,
        }
    }

    /// Create a vertical reference line with custom style
    pub fn vline_styled(x: f64, color: Color, width: f32, style: LineStyle) -> Self {
        Annotation::VLine {
            x,
            style,
            color,
            width,
        }
    }

    /// Create a rectangle annotation
    pub fn rectangle(x: f64, y: f64, width: f64, height: f64) -> Self {
        Annotation::Rectangle {
            x,
            y,
            width,
            height,
            style: ShapeStyle::default(),
        }
    }

    /// Create a rectangle annotation with custom style
    pub fn rectangle_styled(x: f64, y: f64, width: f64, height: f64, style: ShapeStyle) -> Self {
        Annotation::Rectangle {
            x,
            y,
            width,
            height,
            style,
        }
    }

    /// Create a fill between two curves
    pub fn fill_between(x: Vec<f64>, y1: Vec<f64>, y2: Vec<f64>) -> Self {
        Annotation::FillBetween {
            x,
            y1,
            y2,
            style: FillStyle::default(),
            where_positive: false,
        }
    }

    /// Create a fill between a curve and a constant baseline
    pub fn fill_to_baseline(x: Vec<f64>, y: Vec<f64>, baseline: f64) -> Self {
        let y2 = vec![baseline; x.len()];
        Annotation::FillBetween {
            x,
            y1: y,
            y2,
            style: FillStyle::default(),
            where_positive: false,
        }
    }

    /// Create a fill between with custom style
    pub fn fill_between_styled(
        x: Vec<f64>,
        y1: Vec<f64>,
        y2: Vec<f64>,
        style: FillStyle,
        where_positive: bool,
    ) -> Self {
        Annotation::FillBetween {
            x,
            y1,
            y2,
            style,
            where_positive,
        }
    }

    /// Create a horizontal span (shaded vertical region)
    pub fn hspan(x_min: f64, x_max: f64) -> Self {
        Annotation::HSpan {
            x_min,
            x_max,
            style: ShapeStyle::default().fill(Color::new_rgba(128, 128, 128, 50)),
        }
    }

    /// Create a vertical span (shaded horizontal region)
    pub fn vspan(y_min: f64, y_max: f64) -> Self {
        Annotation::VSpan {
            y_min,
            y_max,
            style: ShapeStyle::default().fill(Color::new_rgba(128, 128, 128, 50)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_style_builder() {
        let style = TextStyle::new()
            .font_size(12.0)
            .color(Color::RED)
            .align(TextAlign::Left)
            .rotation(45.0);

        assert!((style.font_size - 12.0).abs() < 0.001);
        assert_eq!(style.color, Color::RED);
        assert_eq!(style.align, TextAlign::Left);
        assert!((style.rotation - 45.0).abs() < 0.001);
    }

    #[test]
    fn test_arrow_style_builder() {
        let style = ArrowStyle::new()
            .color(Color::BLUE)
            .line_width(2.0)
            .head_style(ArrowHead::Stealth)
            .double_headed();

        assert_eq!(style.color, Color::BLUE);
        assert!((style.line_width - 2.0).abs() < 0.001);
        assert_eq!(style.head_style, ArrowHead::Stealth);
        assert_eq!(style.tail_style, ArrowHead::Stealth);
    }

    #[test]
    fn test_shape_style_builder() {
        let style = ShapeStyle::new()
            .fill(Color::GREEN)
            .fill_alpha(0.5)
            .edge(Color::BLACK)
            .edge_width(2.0);

        assert_eq!(style.fill_color, Some(Color::GREEN));
        assert!((style.fill_alpha - 0.5).abs() < 0.001);
        assert_eq!(style.edge_color, Some(Color::BLACK));
        assert!((style.edge_width - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_fill_style_builder() {
        let style = FillStyle::new()
            .color(Color::RED)
            .alpha(0.2)
            .hatch(HatchPattern::Diagonal);

        assert_eq!(style.color, Color::RED);
        assert!((style.alpha - 0.2).abs() < 0.001);
        assert_eq!(style.hatch, Some(HatchPattern::Diagonal));
    }

    #[test]
    fn test_annotation_constructors() {
        let text = Annotation::text(1.0, 2.0, "Hello");
        assert!(
            matches!(text, Annotation::Text { x, y, .. } if (x - 1.0).abs() < 0.001 && (y - 2.0).abs() < 0.001)
        );

        let arrow = Annotation::arrow(0.0, 0.0, 1.0, 1.0);
        assert!(matches!(arrow, Annotation::Arrow { .. }));

        let hline = Annotation::hline(5.0);
        assert!(matches!(hline, Annotation::HLine { y, .. } if (y - 5.0).abs() < 0.001));

        let vline = Annotation::vline(3.0);
        assert!(matches!(vline, Annotation::VLine { x, .. } if (x - 3.0).abs() < 0.001));

        let rect = Annotation::rectangle(0.0, 0.0, 10.0, 5.0);
        assert!(
            matches!(rect, Annotation::Rectangle { width, height, .. } if (width - 10.0).abs() < 0.001 && (height - 5.0).abs() < 0.001)
        );
    }

    #[test]
    fn test_fill_between() {
        let x = vec![1.0, 2.0, 3.0];
        let y1 = vec![1.0, 4.0, 9.0];
        let y2 = vec![0.0, 0.0, 0.0];

        let fill = Annotation::fill_between(x.clone(), y1.clone(), y2.clone());
        assert!(matches!(
            fill,
            Annotation::FillBetween {
                where_positive: false,
                ..
            }
        ));

        let baseline_fill = Annotation::fill_to_baseline(x.clone(), y1.clone(), 0.0);
        if let Annotation::FillBetween { y2, .. } = baseline_fill {
            assert!(y2.iter().all(|&v| (v - 0.0).abs() < 0.001));
        }
    }

    #[test]
    fn test_alpha_clamping() {
        let style = FillStyle::new().alpha(1.5);
        assert!((style.alpha - 1.0).abs() < 0.001);

        let style = FillStyle::new().alpha(-0.5);
        assert!((style.alpha - 0.0).abs() < 0.001);

        let shape = ShapeStyle::new().fill_alpha(2.0);
        assert!((shape.fill_alpha - 1.0).abs() < 0.001);
    }
}
