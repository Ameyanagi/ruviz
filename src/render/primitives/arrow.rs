//! Arrow primitive for quiver plots
//!
//! Provides arrow/vector representation for vector field visualization.

use std::f64::consts::PI;

/// An arrow/vector for quiver plots
#[derive(Debug, Clone, Copy)]
pub struct Arrow {
    /// Start x coordinate
    pub x: f64,
    /// Start y coordinate
    pub y: f64,
    /// X component of direction
    pub dx: f64,
    /// Y component of direction
    pub dy: f64,
    /// Head width as fraction of length
    pub head_width: f64,
    /// Head length as fraction of total length
    pub head_length: f64,
}

impl Arrow {
    /// Create a new arrow
    pub fn new(x: f64, y: f64, dx: f64, dy: f64) -> Self {
        Self {
            x,
            y,
            dx,
            dy,
            head_width: 0.3,
            head_length: 0.2,
        }
    }

    /// Set head width (fraction of length)
    pub fn head_width(mut self, width: f64) -> Self {
        self.head_width = width.clamp(0.0, 1.0);
        self
    }

    /// Set head length (fraction of total length)
    pub fn head_length(mut self, length: f64) -> Self {
        self.head_length = length.clamp(0.0, 1.0);
        self
    }

    /// Get arrow length
    pub fn length(&self) -> f64 {
        (self.dx * self.dx + self.dy * self.dy).sqrt()
    }

    /// Get arrow angle in radians
    pub fn angle(&self) -> f64 {
        self.dy.atan2(self.dx)
    }

    /// Get end point
    pub fn end_point(&self) -> (f64, f64) {
        (self.x + self.dx, self.y + self.dy)
    }

    /// Get midpoint
    pub fn midpoint(&self) -> (f64, f64) {
        (self.x + self.dx / 2.0, self.y + self.dy / 2.0)
    }

    /// Convert arrow to polygon points for rendering
    ///
    /// Returns a filled arrow shape (shaft + head)
    pub fn as_polygon(&self, shaft_width: f64) -> Vec<(f64, f64)> {
        let len = self.length();
        if len < 1e-10 {
            return vec![];
        }

        let angle = self.angle();
        let head_len = len * self.head_length;
        let head_width = len * self.head_width;
        let shaft_len = len - head_len;

        // Rotation helpers
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let rotate = |px: f64, py: f64| -> (f64, f64) {
            (
                self.x + px * cos_a - py * sin_a,
                self.y + px * sin_a + py * cos_a,
            )
        };

        // Build arrow shape
        let hw = shaft_width / 2.0;
        let hhw = head_width / 2.0;

        vec![
            rotate(0.0, -hw),        // Shaft bottom-left
            rotate(shaft_len, -hw),  // Shaft bottom-right
            rotate(shaft_len, -hhw), // Head base bottom
            rotate(len, 0.0),        // Head tip
            rotate(shaft_len, hhw),  // Head base top
            rotate(shaft_len, hw),   // Shaft top-right
            rotate(0.0, hw),         // Shaft top-left
        ]
    }

    /// Convert arrow to line segment (simple representation)
    ///
    /// Returns (x1, y1, x2, y2)
    pub fn as_line(&self) -> (f64, f64, f64, f64) {
        (self.x, self.y, self.x + self.dx, self.y + self.dy)
    }

    /// Convert arrow head to polyline
    ///
    /// Returns points for the V-shaped head
    pub fn head_points(&self, head_size: f64) -> Vec<(f64, f64)> {
        let len = self.length();
        if len < 1e-10 {
            return vec![];
        }

        let angle = self.angle();
        let head_angle = PI / 6.0; // 30 degrees

        let (ex, ey) = self.end_point();

        // Left and right head points
        let left_angle = angle + PI - head_angle;
        let right_angle = angle + PI + head_angle;

        vec![
            (
                ex + head_size * left_angle.cos(),
                ey + head_size * left_angle.sin(),
            ),
            (ex, ey),
            (
                ex + head_size * right_angle.cos(),
                ey + head_size * right_angle.sin(),
            ),
        ]
    }
}

/// Scale arrows to fit within bounds
///
/// # Arguments
/// * `arrows` - Slice of arrows to scale
/// * `scale_factor` - Overall scale (1.0 = original size)
///
/// # Returns
/// Vec of scaled arrows
pub fn scale_arrows(arrows: &[Arrow], scale_factor: f64) -> Vec<Arrow> {
    arrows
        .iter()
        .map(|a| Arrow {
            x: a.x,
            y: a.y,
            dx: a.dx * scale_factor,
            dy: a.dy * scale_factor,
            head_width: a.head_width,
            head_length: a.head_length,
        })
        .collect()
}

/// Auto-scale arrows based on grid spacing
///
/// Scales arrows so the longest doesn't exceed a fraction of grid spacing.
///
/// # Arguments
/// * `arrows` - Slice of arrows
/// * `grid_spacing` - Typical spacing between arrow positions
/// * `max_fraction` - Maximum fraction of grid spacing (default 0.9)
pub fn auto_scale_arrows(
    arrows: &[Arrow],
    grid_spacing: f64,
    max_fraction: Option<f64>,
) -> Vec<Arrow> {
    let max_fraction = max_fraction.unwrap_or(0.9);
    let max_length = arrows.iter().map(|a| a.length()).fold(0.0, f64::max);

    if max_length < 1e-10 {
        return arrows.to_vec();
    }

    let scale = grid_spacing * max_fraction / max_length;
    scale_arrows(arrows, scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arrow_basic() {
        let arrow = Arrow::new(0.0, 0.0, 1.0, 0.0);

        assert!((arrow.length() - 1.0).abs() < 1e-10);
        assert!((arrow.angle() - 0.0).abs() < 1e-10);

        let (ex, ey) = arrow.end_point();
        assert!((ex - 1.0).abs() < 1e-10);
        assert!((ey - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_arrow_diagonal() {
        let arrow = Arrow::new(0.0, 0.0, 1.0, 1.0);

        assert!((arrow.length() - 2.0_f64.sqrt()).abs() < 1e-10);
        assert!((arrow.angle() - PI / 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_arrow_as_polygon() {
        let arrow = Arrow::new(0.0, 0.0, 1.0, 0.0);
        let polygon = arrow.as_polygon(0.1);

        assert_eq!(polygon.len(), 7);
    }

    #[test]
    fn test_arrow_head_points() {
        let arrow = Arrow::new(0.0, 0.0, 1.0, 0.0);
        let head = arrow.head_points(0.2);

        assert_eq!(head.len(), 3);
        // Middle point should be at arrow end
        assert!((head[1].0 - 1.0).abs() < 1e-10);
        assert!((head[1].1 - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_scale_arrows() {
        let arrows = vec![
            Arrow::new(0.0, 0.0, 1.0, 0.0),
            Arrow::new(0.0, 0.0, 0.5, 0.5),
        ];

        let scaled = scale_arrows(&arrows, 2.0);

        assert!((scaled[0].dx - 2.0).abs() < 1e-10);
        assert!((scaled[1].dx - 1.0).abs() < 1e-10);
        assert!((scaled[1].dy - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_auto_scale() {
        let arrows = vec![
            Arrow::new(0.0, 0.0, 2.0, 0.0),
            Arrow::new(1.0, 0.0, 1.0, 0.0),
        ];

        let scaled = auto_scale_arrows(&arrows, 1.0, Some(0.5));

        // Max arrow should now be 0.5 * 1.0 = 0.5 in length
        let max_len = scaled.iter().map(|a| a.length()).fold(0.0, f64::max);
        assert!((max_len - 0.5).abs() < 1e-10);
    }
}
