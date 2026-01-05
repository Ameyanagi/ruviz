//! Polar coordinate system
//!
//! Provides polar axis with theta (angle) and r (radius) coordinates.

use std::f64::consts::PI;

/// Polar coordinate system configuration
#[derive(Debug, Clone)]
pub struct PolarAxes {
    /// Center x in screen coordinates
    pub cx: f64,
    /// Center y in screen coordinates
    pub cy: f64,
    /// Maximum radius in screen coordinates
    pub radius: f64,
    /// Minimum r value in data coordinates
    pub r_min: f64,
    /// Maximum r value in data coordinates
    pub r_max: f64,
    /// Theta offset (0 = right, -PI/2 = top)
    pub theta_offset: f64,
    /// Theta direction (1 = counter-clockwise, -1 = clockwise)
    pub theta_direction: f64,
    /// Whether theta is in degrees (vs radians)
    pub theta_degrees: bool,
    /// Number of radial grid lines
    pub r_grid_count: usize,
    /// Number of angular grid lines
    pub theta_grid_count: usize,
}

impl Default for PolarAxes {
    fn default() -> Self {
        Self {
            cx: 0.0,
            cy: 0.0,
            radius: 100.0,
            r_min: 0.0,
            r_max: 1.0,
            theta_offset: -PI / 2.0, // Start at top
            theta_direction: -1.0,   // Clockwise
            theta_degrees: false,
            r_grid_count: 5,
            theta_grid_count: 12,
        }
    }
}

impl PolarAxes {
    /// Create new polar axes
    pub fn new(cx: f64, cy: f64, radius: f64) -> Self {
        Self {
            cx,
            cy,
            radius,
            ..Default::default()
        }
    }

    /// Set r range
    pub fn r_range(mut self, min: f64, max: f64) -> Self {
        self.r_min = min;
        self.r_max = max;
        self
    }

    /// Set theta to start at top (default)
    pub fn theta_zero_top(mut self) -> Self {
        self.theta_offset = -PI / 2.0;
        self
    }

    /// Set theta to start at right (standard math)
    pub fn theta_zero_right(mut self) -> Self {
        self.theta_offset = 0.0;
        self
    }

    /// Set clockwise direction
    pub fn clockwise(mut self) -> Self {
        self.theta_direction = -1.0;
        self
    }

    /// Set counter-clockwise direction
    pub fn counter_clockwise(mut self) -> Self {
        self.theta_direction = 1.0;
        self
    }

    /// Use degrees for theta
    pub fn degrees(mut self) -> Self {
        self.theta_degrees = true;
        self
    }

    /// Convert polar (theta, r) to Cartesian (x, y) in screen coordinates
    pub fn to_cartesian(&self, theta: f64, r: f64) -> (f64, f64) {
        // Convert theta to radians if needed
        let theta_rad = if self.theta_degrees {
            theta * PI / 180.0
        } else {
            theta
        };

        // Apply offset and direction
        let theta_adj = self.theta_offset + self.theta_direction * theta_rad;

        // Normalize r to screen radius
        let r_norm = if (self.r_max - self.r_min).abs() < 1e-10 {
            0.0
        } else {
            (r - self.r_min) / (self.r_max - self.r_min) * self.radius
        };

        (
            self.cx + r_norm * theta_adj.cos(),
            self.cy + r_norm * theta_adj.sin(),
        )
    }

    /// Convert Cartesian (x, y) to polar (theta, r) in data coordinates
    pub fn to_polar(&self, x: f64, y: f64) -> (f64, f64) {
        let dx = x - self.cx;
        let dy = y - self.cy;

        let r_screen = (dx * dx + dy * dy).sqrt();
        let r = self.r_min + (r_screen / self.radius) * (self.r_max - self.r_min);

        let theta_adj = dy.atan2(dx);
        let theta_rad = (theta_adj - self.theta_offset) / self.theta_direction;

        let theta = if self.theta_degrees {
            theta_rad * 180.0 / PI
        } else {
            theta_rad
        };

        (theta, r)
    }

    /// Generate radial grid circles
    ///
    /// Returns Vec of (r_value, screen_radius)
    pub fn r_grid_circles(&self) -> Vec<(f64, f64)> {
        let step = (self.r_max - self.r_min) / self.r_grid_count as f64;

        (1..=self.r_grid_count)
            .map(|i| {
                let r = self.r_min + i as f64 * step;
                let screen_r = (r - self.r_min) / (self.r_max - self.r_min) * self.radius;
                (r, screen_r)
            })
            .collect()
    }

    /// Generate angular grid lines
    ///
    /// Returns Vec of (theta_value, start_point, end_point)
    #[allow(clippy::type_complexity)]
    pub fn theta_grid_lines(&self) -> Vec<(f64, (f64, f64), (f64, f64))> {
        let step = 2.0 * PI / self.theta_grid_count as f64;

        (0..self.theta_grid_count)
            .map(|i| {
                let theta = i as f64 * step;
                let theta_display = if self.theta_degrees {
                    theta * 180.0 / PI
                } else {
                    theta
                };

                let (x_inner, y_inner) = self.to_cartesian(theta_display, self.r_min);
                let (x_outer, y_outer) = self.to_cartesian(theta_display, self.r_max);

                (theta_display, (x_inner, y_inner), (x_outer, y_outer))
            })
            .collect()
    }

    /// Generate theta labels
    pub fn theta_labels(&self) -> Vec<(String, f64, f64)> {
        let label_radius = self.r_max * 1.1;

        self.theta_grid_lines()
            .into_iter()
            .map(|(theta, _, _)| {
                let (x, y) = self.to_cartesian(theta, label_radius);
                let label = if self.theta_degrees {
                    format!("{}°", theta as i32)
                } else {
                    format!("{:.1}π", theta / PI)
                };
                (label, x, y)
            })
            .collect()
    }

    /// Convert a polyline from polar to Cartesian
    pub fn transform_polyline(&self, theta: &[f64], r: &[f64]) -> Vec<(f64, f64)> {
        theta
            .iter()
            .zip(r.iter())
            .map(|(&t, &r)| self.to_cartesian(t, r))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_cartesian_basic() {
        let polar = PolarAxes::new(100.0, 100.0, 50.0)
            .r_range(0.0, 1.0)
            .theta_zero_right()
            .counter_clockwise();

        // theta=0, r=1 should be at (150, 100) - right
        let (x, y) = polar.to_cartesian(0.0, 1.0);
        assert!((x - 150.0).abs() < 1e-10);
        assert!((y - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_to_cartesian_top_start() {
        let polar = PolarAxes::new(100.0, 100.0, 50.0)
            .r_range(0.0, 1.0)
            .theta_zero_top()
            .clockwise();

        // theta=0, r=1 should be at (100, 50) - top
        let (x, y) = polar.to_cartesian(0.0, 1.0);
        assert!((x - 100.0).abs() < 1e-10);
        assert!((y - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_degrees_mode() {
        let polar = PolarAxes::new(100.0, 100.0, 50.0)
            .r_range(0.0, 1.0)
            .theta_zero_right()
            .counter_clockwise()
            .degrees();

        // theta=90deg, r=1 should be at (100, 150) - top in standard coords
        let (x, y) = polar.to_cartesian(90.0, 1.0);
        assert!((x - 100.0).abs() < 1e-10);
        assert!((y - 150.0).abs() < 1e-10);
    }

    #[test]
    fn test_round_trip() {
        let polar = PolarAxes::new(100.0, 100.0, 50.0).r_range(0.0, 2.0);

        let original_theta = PI / 4.0;
        let original_r = 1.5;

        let (x, y) = polar.to_cartesian(original_theta, original_r);
        let (theta, r) = polar.to_polar(x, y);

        assert!((theta - original_theta).abs() < 1e-10);
        assert!((r - original_r).abs() < 1e-10);
    }

    #[test]
    fn test_grid_circles() {
        let polar = PolarAxes::new(100.0, 100.0, 50.0).r_range(0.0, 1.0);
        let circles = polar.r_grid_circles();

        assert_eq!(circles.len(), 5);
        assert!((circles[4].0 - 1.0).abs() < 1e-10);
        assert!((circles[4].1 - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_theta_grid() {
        let polar = PolarAxes::new(100.0, 100.0, 50.0);
        let lines = polar.theta_grid_lines();

        assert_eq!(lines.len(), 12); // Default 12 lines
    }
}
