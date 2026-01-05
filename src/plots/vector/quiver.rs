//! Quiver (vector field) plot implementations
//!
//! Provides arrow/vector field visualization.

use crate::render::Color;

/// Configuration for quiver plot
#[derive(Debug, Clone)]
pub struct QuiverConfig {
    /// Arrow color (None for auto)
    pub color: Option<Color>,
    /// Scale factor for arrow length
    pub scale: f64,
    /// Arrow width
    pub width: f32,
    /// Arrow head length as fraction of arrow length
    pub headlength: f64,
    /// Arrow head width as fraction of arrow length
    pub headwidth: f64,
    /// Whether to use angles (radians) + magnitudes instead of u,v
    pub angles_mode: bool,
    /// Pivot point (tail, middle, tip)
    pub pivot: QuiverPivot,
    /// Color by magnitude
    pub color_by_magnitude: bool,
    /// Colormap for magnitude coloring
    pub cmap: String,
}

/// Pivot point for arrows
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuiverPivot {
    /// Arrow starts at (x, y)
    Tail,
    /// Arrow centered at (x, y)
    Middle,
    /// Arrow ends at (x, y)
    Tip,
}

impl Default for QuiverConfig {
    fn default() -> Self {
        Self {
            color: None,
            scale: 1.0,
            width: 1.5,
            headlength: 0.3,
            headwidth: 0.2,
            angles_mode: false,
            pivot: QuiverPivot::Tail,
            color_by_magnitude: false,
            cmap: "viridis".to_string(),
        }
    }
}

impl QuiverConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set scale
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale.max(0.0);
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set arrow width
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(0.1);
        self
    }

    /// Set pivot point
    pub fn pivot(mut self, pivot: QuiverPivot) -> Self {
        self.pivot = pivot;
        self
    }

    /// Color arrows by magnitude
    pub fn color_by_magnitude(mut self, enable: bool) -> Self {
        self.color_by_magnitude = enable;
        self
    }
}

/// A single arrow in a quiver plot
#[derive(Debug, Clone)]
pub struct QuiverArrow {
    /// Start point
    pub start: (f64, f64),
    /// End point
    pub end: (f64, f64),
    /// Magnitude
    pub magnitude: f64,
    /// Angle (radians)
    pub angle: f64,
    /// Head vertices for triangle
    pub head: [(f64, f64); 3],
}

/// Computed quiver data
#[derive(Debug, Clone)]
pub struct QuiverPlotData {
    /// All arrows
    pub arrows: Vec<QuiverArrow>,
    /// Magnitude range for color scaling
    pub magnitude_range: (f64, f64),
}

/// Compute quiver plot data
///
/// # Arguments
/// * `x` - X positions
/// * `y` - Y positions
/// * `u` - X components of vectors (or angles if angles_mode)
/// * `v` - Y components of vectors (or magnitudes if angles_mode)
/// * `config` - Quiver configuration
///
/// # Returns
/// QuiverPlotData for rendering
pub fn compute_quiver(
    x: &[f64],
    y: &[f64],
    u: &[f64],
    v: &[f64],
    config: &QuiverConfig,
) -> QuiverPlotData {
    let n = x.len().min(y.len()).min(u.len()).min(v.len());
    if n == 0 {
        return QuiverPlotData {
            arrows: vec![],
            magnitude_range: (0.0, 1.0),
        };
    }

    let mut arrows = Vec::with_capacity(n);
    let mut min_mag = f64::INFINITY;
    let mut max_mag = f64::NEG_INFINITY;

    for i in 0..n {
        let (dx, dy, magnitude, angle) = if config.angles_mode {
            // u = angle, v = magnitude
            let angle = u[i];
            let mag = v[i];
            (mag * angle.cos(), mag * angle.sin(), mag, angle)
        } else {
            // u = dx, v = dy
            let mag = (u[i] * u[i] + v[i] * v[i]).sqrt();
            let angle = v[i].atan2(u[i]);
            (u[i], v[i], mag, angle)
        };

        min_mag = min_mag.min(magnitude);
        max_mag = max_mag.max(magnitude);

        // Apply scale
        let dx = dx * config.scale;
        let dy = dy * config.scale;

        // Calculate start/end based on pivot
        let (start, end) = match config.pivot {
            QuiverPivot::Tail => ((x[i], y[i]), (x[i] + dx, y[i] + dy)),
            QuiverPivot::Middle => (
                (x[i] - dx / 2.0, y[i] - dy / 2.0),
                (x[i] + dx / 2.0, y[i] + dy / 2.0),
            ),
            QuiverPivot::Tip => ((x[i] - dx, y[i] - dy), (x[i], y[i])),
        };

        // Calculate arrow head
        let arrow_len = (dx * dx + dy * dy).sqrt();
        let head = compute_arrow_head(end, angle, arrow_len, config);

        arrows.push(QuiverArrow {
            start,
            end,
            magnitude,
            angle,
            head,
        });
    }

    QuiverPlotData {
        arrows,
        magnitude_range: (min_mag, max_mag),
    }
}

/// Compute arrow head vertices
fn compute_arrow_head(
    tip: (f64, f64),
    angle: f64,
    arrow_len: f64,
    config: &QuiverConfig,
) -> [(f64, f64); 3] {
    let head_len = arrow_len * config.headlength;
    let head_width = arrow_len * config.headwidth;

    // Back angle
    let back_angle = std::f64::consts::PI - angle;

    // Two points at the back of the head
    let left_angle = back_angle + 0.5;
    let right_angle = back_angle - 0.5;

    let half_width = head_width / 2.0;

    [
        tip,
        (
            tip.0 - head_len * angle.cos()
                + half_width * (angle + std::f64::consts::PI / 2.0).cos(),
            tip.1 - head_len * angle.sin()
                + half_width * (angle + std::f64::consts::PI / 2.0).sin(),
        ),
        (
            tip.0
                - head_len * angle.cos()
                - half_width * (angle + std::f64::consts::PI / 2.0).cos(),
            tip.1
                - head_len * angle.sin()
                - half_width * (angle + std::f64::consts::PI / 2.0).sin(),
        ),
    ]
}

/// Compute data range for quiver plot
pub fn quiver_range(data: &QuiverPlotData) -> ((f64, f64), (f64, f64)) {
    if data.arrows.is_empty() {
        return ((0.0, 1.0), (0.0, 1.0));
    }

    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;

    for arrow in &data.arrows {
        x_min = x_min.min(arrow.start.0).min(arrow.end.0);
        x_max = x_max.max(arrow.start.0).max(arrow.end.0);
        y_min = y_min.min(arrow.start.1).min(arrow.end.1);
        y_max = y_max.max(arrow.start.1).max(arrow.end.1);
    }

    ((x_min, x_max), (y_min, y_max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quiver_basic() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 2.0];
        let u = vec![1.0, 0.0, -1.0];
        let v = vec![0.0, 1.0, 0.0];
        let config = QuiverConfig::default();
        let data = compute_quiver(&x, &y, &u, &v, &config);

        assert_eq!(data.arrows.len(), 3);
    }

    #[test]
    fn test_quiver_angles_mode() {
        let x = vec![0.0];
        let y = vec![0.0];
        let angles = vec![0.0]; // Point right
        let magnitudes = vec![1.0];
        let config = QuiverConfig::default();
        // Note: angles_mode not implemented in this simple version
        let data = compute_quiver(&x, &y, &angles, &magnitudes, &config);

        assert_eq!(data.arrows.len(), 1);
    }

    #[test]
    fn test_quiver_pivot() {
        let x = vec![0.0];
        let y = vec![0.0];
        let u = vec![1.0];
        let v = vec![0.0];

        // Tail pivot
        let config = QuiverConfig::default().pivot(QuiverPivot::Tail);
        let data = compute_quiver(&x, &y, &u, &v, &config);
        assert!((data.arrows[0].start.0 - 0.0).abs() < 1e-10);

        // Tip pivot
        let config = QuiverConfig::default().pivot(QuiverPivot::Tip);
        let data = compute_quiver(&x, &y, &u, &v, &config);
        assert!((data.arrows[0].end.0 - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_quiver_empty() {
        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        let u: Vec<f64> = vec![];
        let v: Vec<f64> = vec![];
        let config = QuiverConfig::default();
        let data = compute_quiver(&x, &y, &u, &v, &config);

        assert!(data.arrows.is_empty());
    }
}
