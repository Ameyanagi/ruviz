use crate::render::{Color, LineStyle};

use super::SurfaceSampling;

/// Styling for a regular-grid wireframe.
#[derive(Clone, Debug)]
pub struct Wireframe3DConfig {
    /// Optional fixed line color. `None` uses the theme foreground.
    pub color: Option<Color>,
    /// Stroke width in typographic points.
    pub line_width: f32,
    /// Stroke pattern.
    pub line_style: LineStyle,
    /// Static/interactive sampling policy.
    pub sampling: SurfaceSampling,
}

impl Default for Wireframe3DConfig {
    fn default() -> Self {
        Self {
            color: None,
            line_width: 1.0,
            line_style: LineStyle::Solid,
            sampling: SurfaceSampling::Auto,
        }
    }
}
