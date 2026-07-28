use crate::render::{Color, LineStyle};

/// Styling for a 3D line series.
#[derive(Clone, Debug)]
pub struct Line3DConfig {
    /// Stroke width in typographic points.
    pub line_width: f32,
    /// Stroke pattern.
    pub line_style: LineStyle,
    /// Optional fixed line color. `None` uses the theme palette.
    pub color: Option<Color>,
}

impl Default for Line3DConfig {
    fn default() -> Self {
        Self {
            line_width: 1.5,
            line_style: LineStyle::Solid,
            color: None,
        }
    }
}
