//! Filled polygon primitive
//!
//! Provides polygon representation for violin plots, radar charts, and filled regions.

/// A closed polygon defined by vertices
#[derive(Debug, Clone)]
pub struct Polygon {
    /// Vertices as (x, y) coordinates
    pub vertices: Vec<(f64, f64)>,
    /// Whether to close the polygon (connect last to first)
    pub closed: bool,
}

impl Polygon {
    /// Create a new polygon from vertices
    pub fn new(vertices: Vec<(f64, f64)>) -> Self {
        Self {
            vertices,
            closed: true,
        }
    }

    /// Create an open polygon (polyline)
    pub fn open(vertices: Vec<(f64, f64)>) -> Self {
        Self {
            vertices,
            closed: false,
        }
    }

    /// Check if polygon is valid (has at least 3 vertices)
    pub fn is_valid(&self) -> bool {
        self.vertices.len() >= 3
    }

    /// Calculate polygon centroid
    pub fn centroid(&self) -> (f64, f64) {
        if self.vertices.is_empty() {
            return (0.0, 0.0);
        }

        let n = self.vertices.len() as f64;
        let sum_x: f64 = self.vertices.iter().map(|(x, _)| x).sum();
        let sum_y: f64 = self.vertices.iter().map(|(_, y)| y).sum();

        (sum_x / n, sum_y / n)
    }

    /// Calculate polygon area (signed, positive for counter-clockwise)
    pub fn signed_area(&self) -> f64 {
        if self.vertices.len() < 3 {
            return 0.0;
        }

        let mut area = 0.0;
        let n = self.vertices.len();

        for i in 0..n {
            let j = (i + 1) % n;
            area += self.vertices[i].0 * self.vertices[j].1;
            area -= self.vertices[j].0 * self.vertices[i].1;
        }

        area / 2.0
    }

    /// Calculate absolute polygon area
    pub fn area(&self) -> f64 {
        self.signed_area().abs()
    }

    /// Check if polygon is counter-clockwise oriented
    pub fn is_ccw(&self) -> bool {
        self.signed_area() > 0.0
    }

    /// Reverse vertex order
    pub fn reverse(&mut self) {
        self.vertices.reverse();
    }

    /// Get bounding box (min_x, min_y, max_x, max_y)
    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        if self.vertices.is_empty() {
            return (0.0, 0.0, 0.0, 0.0);
        }

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for &(x, y) in &self.vertices {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }

        (min_x, min_y, max_x, max_y)
    }

    /// Check if a point is inside the polygon (ray casting algorithm)
    pub fn contains(&self, x: f64, y: f64) -> bool {
        if self.vertices.len() < 3 {
            return false;
        }

        let mut inside = false;
        let n = self.vertices.len();

        let mut j = n - 1;
        for i in 0..n {
            let (xi, yi) = self.vertices[i];
            let (xj, yj) = self.vertices[j];

            if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
                inside = !inside;
            }
            j = i;
        }

        inside
    }

    /// Scale polygon around centroid
    pub fn scale(&mut self, factor: f64) {
        let (cx, cy) = self.centroid();
        for (x, y) in &mut self.vertices {
            *x = cx + (*x - cx) * factor;
            *y = cy + (*y - cy) * factor;
        }
    }

    /// Translate polygon
    pub fn translate(&mut self, dx: f64, dy: f64) {
        for (x, y) in &mut self.vertices {
            *x += dx;
            *y += dy;
        }
    }
}

/// Create a regular polygon
///
/// # Arguments
/// * `cx` - Center x
/// * `cy` - Center y
/// * `radius` - Distance from center to vertices
/// * `n_sides` - Number of sides (3 = triangle, 4 = square, etc.)
/// * `rotation` - Rotation in radians
pub fn regular_polygon(cx: f64, cy: f64, radius: f64, n_sides: usize, rotation: f64) -> Polygon {
    use std::f64::consts::PI;

    let n_sides = n_sides.max(3);
    let vertices: Vec<(f64, f64)> = (0..n_sides)
        .map(|i| {
            let angle = rotation + 2.0 * PI * i as f64 / n_sides as f64;
            (cx + radius * angle.cos(), cy + radius * angle.sin())
        })
        .collect();

    Polygon::new(vertices)
}

/// Create a star polygon
///
/// # Arguments
/// * `cx` - Center x
/// * `cy` - Center y
/// * `outer_radius` - Distance to outer points
/// * `inner_radius` - Distance to inner points
/// * `n_points` - Number of star points
/// * `rotation` - Rotation in radians
pub fn star_polygon(
    cx: f64,
    cy: f64,
    outer_radius: f64,
    inner_radius: f64,
    n_points: usize,
    rotation: f64,
) -> Polygon {
    use std::f64::consts::PI;

    let n_points = n_points.max(3);
    let vertices: Vec<(f64, f64)> = (0..(2 * n_points))
        .map(|i| {
            let angle = rotation + PI * i as f64 / n_points as f64;
            let r = if i % 2 == 0 {
                outer_radius
            } else {
                inner_radius
            };
            (cx + r * angle.cos(), cy + r * angle.sin())
        })
        .collect();

    Polygon::new(vertices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_polygon_area_square() {
        // Unit square
        let polygon = Polygon::new(vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);

        assert!((polygon.area() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_polygon_centroid() {
        let polygon = Polygon::new(vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)]);

        let (cx, cy) = polygon.centroid();
        assert!((cx - 1.0).abs() < 1e-10);
        assert!((cy - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_polygon_contains() {
        let polygon = Polygon::new(vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);

        assert!(polygon.contains(0.5, 0.5));
        assert!(!polygon.contains(1.5, 0.5));
        assert!(!polygon.contains(-0.5, 0.5));
    }

    #[test]
    fn test_regular_polygon() {
        let triangle = regular_polygon(0.0, 0.0, 1.0, 3, -PI / 2.0);
        assert_eq!(triangle.vertices.len(), 3);

        let hexagon = regular_polygon(0.0, 0.0, 1.0, 6, 0.0);
        assert_eq!(hexagon.vertices.len(), 6);
    }

    #[test]
    fn test_star_polygon() {
        let star = star_polygon(0.0, 0.0, 1.0, 0.5, 5, -PI / 2.0);
        assert_eq!(star.vertices.len(), 10); // 5 points * 2 (outer + inner)
    }

    #[test]
    fn test_polygon_bounds() {
        let polygon = Polygon::new(vec![(1.0, 2.0), (3.0, 4.0), (2.0, 5.0)]);

        let (min_x, min_y, max_x, max_y) = polygon.bounds();
        assert!((min_x - 1.0).abs() < 1e-10);
        assert!((min_y - 2.0).abs() < 1e-10);
        assert!((max_x - 3.0).abs() < 1e-10);
        assert!((max_y - 5.0).abs() < 1e-10);
    }
}
