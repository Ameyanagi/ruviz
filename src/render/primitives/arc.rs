//! Arc and wedge primitives for pie charts
//!
//! Provides arc (curved line) and wedge (filled pie slice) rendering.

use std::f64::consts::PI;

/// An arc (curved line segment)
#[derive(Debug, Clone, Copy)]
pub struct Arc {
    /// Center x coordinate
    pub cx: f64,
    /// Center y coordinate
    pub cy: f64,
    /// Radius
    pub radius: f64,
    /// Start angle in radians (0 = right, positive = counter-clockwise)
    pub start_angle: f64,
    /// End angle in radians
    pub end_angle: f64,
}

impl Arc {
    /// Create a new arc
    pub fn new(cx: f64, cy: f64, radius: f64, start_angle: f64, end_angle: f64) -> Self {
        Self {
            cx,
            cy,
            radius,
            start_angle,
            end_angle,
        }
    }

    /// Convert arc to polyline points for rendering
    ///
    /// # Arguments
    /// * `segments` - Number of line segments to approximate the arc
    ///
    /// # Returns
    /// Vec of (x, y) points along the arc
    pub fn as_points(&self, segments: usize) -> Vec<(f64, f64)> {
        let segments = segments.max(1);
        let angle_range = self.end_angle - self.start_angle;

        (0..=segments)
            .map(|i| {
                let t = i as f64 / segments as f64;
                let angle = self.start_angle + t * angle_range;
                (
                    self.cx + self.radius * angle.cos(),
                    self.cy + self.radius * angle.sin(),
                )
            })
            .collect()
    }

    /// Get the midpoint angle
    pub fn mid_angle(&self) -> f64 {
        (self.start_angle + self.end_angle) / 2.0
    }

    /// Get the arc length
    pub fn arc_length(&self) -> f64 {
        self.radius * (self.end_angle - self.start_angle).abs()
    }
}

/// A wedge (filled pie slice)
#[derive(Debug, Clone, Copy)]
pub struct Wedge {
    /// Center x coordinate
    pub cx: f64,
    /// Center y coordinate
    pub cy: f64,
    /// Outer radius
    pub radius: f64,
    /// Inner radius (0 for full pie, >0 for donut)
    pub inner_radius: f64,
    /// Start angle in radians
    pub start_angle: f64,
    /// End angle in radians
    pub end_angle: f64,
    /// Explode offset (distance from center)
    pub explode: f64,
}

impl Wedge {
    /// Create a new wedge
    pub fn new(cx: f64, cy: f64, radius: f64, start_angle: f64, end_angle: f64) -> Self {
        Self {
            cx,
            cy,
            radius,
            inner_radius: 0.0,
            start_angle,
            end_angle,
            explode: 0.0,
        }
    }

    /// Set inner radius for donut chart
    pub fn inner_radius(mut self, radius: f64) -> Self {
        self.inner_radius = radius.max(0.0).min(self.radius);
        self
    }

    /// Set explode offset
    pub fn explode(mut self, offset: f64) -> Self {
        self.explode = offset.max(0.0);
        self
    }

    /// Get the effective center after applying explode offset
    pub fn exploded_center(&self) -> (f64, f64) {
        if self.explode <= 0.0 {
            return (self.cx, self.cy);
        }

        let mid_angle = (self.start_angle + self.end_angle) / 2.0;
        (
            self.cx + self.explode * mid_angle.cos(),
            self.cy + self.explode * mid_angle.sin(),
        )
    }

    /// Convert wedge to polygon points for rendering
    ///
    /// # Arguments
    /// * `segments` - Number of segments per arc
    ///
    /// # Returns
    /// Vec of (x, y) points forming the wedge outline
    pub fn as_polygon(&self, segments: usize) -> Vec<(f64, f64)> {
        let segments = segments.max(1);
        let (cx, cy) = self.exploded_center();
        let angle_range = self.end_angle - self.start_angle;

        let mut points = Vec::with_capacity(2 * segments + 4);

        // Outer arc
        for i in 0..=segments {
            let t = i as f64 / segments as f64;
            let angle = self.start_angle + t * angle_range;
            points.push((
                cx + self.radius * angle.cos(),
                cy + self.radius * angle.sin(),
            ));
        }

        // Inner arc (reverse direction) or center point
        if self.inner_radius > 0.0 {
            for i in (0..=segments).rev() {
                let t = i as f64 / segments as f64;
                let angle = self.start_angle + t * angle_range;
                points.push((
                    cx + self.inner_radius * angle.cos(),
                    cy + self.inner_radius * angle.sin(),
                ));
            }
        } else {
            // Full wedge goes to center
            points.push((cx, cy));
        }

        points
    }

    /// Get the centroid of the wedge (for label placement)
    pub fn centroid(&self) -> (f64, f64) {
        let mid_angle = (self.start_angle + self.end_angle) / 2.0;
        let r = if self.inner_radius > 0.0 {
            (self.radius + self.inner_radius) / 2.0
        } else {
            self.radius * 2.0 / 3.0 // 2/3 of radius for full wedge
        };

        let (cx, cy) = self.exploded_center();
        (cx + r * mid_angle.cos(), cy + r * mid_angle.sin())
    }

    /// Get label position (outside the wedge)
    pub fn label_position(&self, offset: f64) -> (f64, f64) {
        let mid_angle = (self.start_angle + self.end_angle) / 2.0;
        let r = self.radius + offset;
        let (cx, cy) = self.exploded_center();
        (cx + r * mid_angle.cos(), cy + r * mid_angle.sin())
    }
}

/// Create wedges for a pie chart from values
///
/// # Arguments
/// * `values` - Slice of positive values
/// * `cx` - Center x
/// * `cy` - Center y
/// * `radius` - Outer radius
/// * `start_angle` - Starting angle (default -PI/2 for top)
///
/// # Returns
/// Vec of Wedge structs
pub fn pie_wedges(
    values: &[f64],
    cx: f64,
    cy: f64,
    radius: f64,
    start_angle: Option<f64>,
) -> Vec<Wedge> {
    let total: f64 = values.iter().filter(|&&v| v > 0.0).sum();

    if total <= 0.0 {
        return vec![];
    }

    let start = start_angle.unwrap_or(-PI / 2.0);
    let mut angle = start;

    values
        .iter()
        .filter(|&&v| v > 0.0)
        .map(|&v| {
            let sweep = 2.0 * PI * v / total;
            let wedge = Wedge::new(cx, cy, radius, angle, angle + sweep);
            angle += sweep;
            wedge
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_as_points() {
        let arc = Arc::new(0.0, 0.0, 1.0, 0.0, PI / 2.0);
        let points = arc.as_points(4);

        assert_eq!(points.len(), 5);
        // First point at angle 0 (1, 0)
        assert!((points[0].0 - 1.0).abs() < 1e-10);
        assert!((points[0].1 - 0.0).abs() < 1e-10);
        // Last point at angle PI/2 (0, 1)
        assert!((points[4].0 - 0.0).abs() < 1e-10);
        assert!((points[4].1 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_wedge_as_polygon() {
        let wedge = Wedge::new(0.0, 0.0, 1.0, 0.0, PI / 2.0);
        let polygon = wedge.as_polygon(4);

        // 5 points on outer arc + 1 center point
        assert_eq!(polygon.len(), 6);
    }

    #[test]
    fn test_donut_wedge() {
        let wedge = Wedge::new(0.0, 0.0, 1.0, 0.0, PI / 2.0).inner_radius(0.5);
        let polygon = wedge.as_polygon(4);

        // 5 points on outer arc + 5 points on inner arc
        assert_eq!(polygon.len(), 10);
    }

    #[test]
    fn test_exploded_wedge() {
        let wedge = Wedge::new(0.0, 0.0, 1.0, 0.0, PI / 2.0).explode(0.1);
        let (cx, cy) = wedge.exploded_center();

        // Should be offset from origin
        assert!(cx > 0.0);
        assert!(cy > 0.0);
    }

    #[test]
    fn test_pie_wedges() {
        let values = vec![1.0, 1.0, 1.0, 1.0];
        let wedges = pie_wedges(&values, 0.0, 0.0, 1.0, None);

        assert_eq!(wedges.len(), 4);

        // Each wedge should span PI/2 radians
        for wedge in &wedges {
            let sweep = wedge.end_angle - wedge.start_angle;
            assert!((sweep - PI / 2.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_centroid() {
        let wedge = Wedge::new(0.0, 0.0, 1.0, 0.0, PI / 2.0);
        let (cx, cy) = wedge.centroid();

        // Centroid at 2/3 radius, 45 degrees
        let expected_r = 2.0 / 3.0;
        let expected_angle = PI / 4.0;
        let expected_x = expected_r * expected_angle.cos();
        let expected_y = expected_r * expected_angle.sin();

        assert!((cx - expected_x).abs() < 1e-10);
        assert!((cy - expected_y).abs() < 1e-10);
    }
}
