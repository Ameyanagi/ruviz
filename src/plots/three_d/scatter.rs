use crate::render::{Color, MarkerStyle};

/// Styling for a 3D scatter series.
#[derive(Clone, Debug)]
pub struct Scatter3DConfig {
    /// Marker shape.
    pub marker: MarkerStyle,
    /// Marker diameter in typographic points.
    pub marker_size: f32,
    /// Optional fixed marker color. `None` uses the theme palette.
    pub color: Option<Color>,
}

impl Default for Scatter3DConfig {
    fn default() -> Self {
        Self {
            marker: MarkerStyle::Circle,
            marker_size: 6.0,
            color: None,
        }
    }
}
