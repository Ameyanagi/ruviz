//! Polar plot implementations
//!
//! Provides polar scatter, line, and bar plots.

use crate::render::Color;

/// Configuration for polar plots
#[derive(Debug, Clone)]
pub struct PolarPlotConfig {
    /// Start angle in radians (0 = right, counter-clockwise)
    pub theta_offset: f64,
    /// Direction of theta (true = counter-clockwise)
    pub theta_direction: bool,
    /// Show radial grid
    pub show_rgrid: bool,
    /// Show angular grid
    pub show_thetgrid: bool,
    /// Number of radial grid lines
    pub rgrid_count: usize,
    /// Number of angular grid lines
    pub thetgrid_count: usize,
    /// Line color (None for auto)
    pub color: Option<Color>,
    /// Line width
    pub line_width: f32,
    /// Marker size
    pub marker_size: f32,
    /// Fill area under curve
    pub fill: bool,
    /// Fill alpha
    pub fill_alpha: f32,
}

impl Default for PolarPlotConfig {
    fn default() -> Self {
        Self {
            theta_offset: 0.0,
            theta_direction: true,
            show_rgrid: true,
            show_thetgrid: true,
            rgrid_count: 5,
            thetgrid_count: 12,
            color: None,
            line_width: 1.5,
            marker_size: 0.0,
            fill: false,
            fill_alpha: 0.3,
        }
    }
}

impl PolarPlotConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set theta offset
    pub fn theta_offset(mut self, offset: f64) -> Self {
        self.theta_offset = offset;
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.1);
        self
    }

    /// Set marker size
    pub fn marker_size(mut self, size: f32) -> Self {
        self.marker_size = size.max(0.0);
        self
    }

    /// Enable fill
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Set fill alpha
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.fill_alpha = alpha.clamp(0.0, 1.0);
        self
    }
}

/// A point in polar coordinates
#[derive(Debug, Clone, Copy)]
pub struct PolarPoint {
    /// Radius (distance from center)
    pub r: f64,
    /// Theta (angle in radians)
    pub theta: f64,
    /// Cartesian x
    pub x: f64,
    /// Cartesian y
    pub y: f64,
}

impl PolarPoint {
    /// Create from polar coordinates
    pub fn from_polar(r: f64, theta: f64) -> Self {
        Self {
            r,
            theta,
            x: r * theta.cos(),
            y: r * theta.sin(),
        }
    }

    /// Create from cartesian coordinates
    pub fn from_cartesian(x: f64, y: f64) -> Self {
        Self {
            r: (x * x + y * y).sqrt(),
            theta: y.atan2(x),
            x,
            y,
        }
    }
}

/// Computed polar plot data
#[derive(Debug, Clone)]
pub struct PolarPlotData {
    /// Points in the plot
    pub points: Vec<PolarPoint>,
    /// Maximum radius
    pub r_max: f64,
    /// Polygon vertices for fill (closed path)
    pub fill_polygon: Vec<(f64, f64)>,
}

/// Compute polar plot points
///
/// # Arguments
/// * `r` - Radius values
/// * `theta` - Theta values (in radians)
/// * `config` - Polar plot configuration
///
/// # Returns
/// PolarPlotData with converted points
pub fn compute_polar_plot(r: &[f64], theta: &[f64], config: &PolarPlotConfig) -> PolarPlotData {
    let n = r.len().min(theta.len());
    if n == 0 {
        return PolarPlotData {
            points: vec![],
            r_max: 1.0,
            fill_polygon: vec![],
        };
    }

    let mut points = Vec::with_capacity(n);
    let mut r_max = 0.0_f64;

    for i in 0..n {
        // Apply theta offset and direction
        let adjusted_theta = if config.theta_direction {
            theta[i] + config.theta_offset
        } else {
            -theta[i] + config.theta_offset
        };

        let point = PolarPoint::from_polar(r[i], adjusted_theta);
        r_max = r_max.max(r[i].abs());
        points.push(point);
    }

    // Generate fill polygon if enabled
    let fill_polygon = if config.fill && !points.is_empty() {
        let mut polygon: Vec<(f64, f64)> = points.iter().map(|p| (p.x, p.y)).collect();
        // Close through origin
        polygon.push((0.0, 0.0));
        polygon
    } else {
        vec![]
    };

    PolarPlotData {
        points,
        r_max: if r_max > 0.0 { r_max } else { 1.0 },
        fill_polygon,
    }
}

/// Generate polar grid lines
///
/// # Arguments
/// * `r_max` - Maximum radius
/// * `r_count` - Number of radial circles
/// * `theta_count` - Number of angular divisions
///
/// # Returns
/// (radial_circles, angular_lines) where circles are radii and lines are (start, end) points
#[allow(clippy::type_complexity)]
pub fn polar_grid(
    r_max: f64,
    r_count: usize,
    theta_count: usize,
) -> (Vec<f64>, Vec<((f64, f64), (f64, f64))>) {
    // Radial circles
    let radii: Vec<f64> = (1..=r_count)
        .map(|i| r_max * i as f64 / r_count as f64)
        .collect();

    // Angular lines (from center to edge)
    let angular_step = 2.0 * std::f64::consts::PI / theta_count as f64;
    let angular_lines: Vec<((f64, f64), (f64, f64))> = (0..theta_count)
        .map(|i| {
            let theta = i as f64 * angular_step;
            ((0.0, 0.0), (r_max * theta.cos(), r_max * theta.sin()))
        })
        .collect();

    (radii, angular_lines)
}

/// Generate circle vertices for rendering
pub fn circle_vertices(cx: f64, cy: f64, radius: f64, n_segments: usize) -> Vec<(f64, f64)> {
    let step = 2.0 * std::f64::consts::PI / n_segments as f64;
    (0..=n_segments)
        .map(|i| {
            let theta = i as f64 * step;
            (cx + radius * theta.cos(), cy + radius * theta.sin())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_polar_point_from_polar() {
        let point = PolarPoint::from_polar(1.0, 0.0);
        assert!((point.x - 1.0).abs() < 1e-10);
        assert!((point.y - 0.0).abs() < 1e-10);

        let point = PolarPoint::from_polar(1.0, PI / 2.0);
        assert!((point.x - 0.0).abs() < 1e-10);
        assert!((point.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_polar_point_from_cartesian() {
        let point = PolarPoint::from_cartesian(1.0, 0.0);
        assert!((point.r - 1.0).abs() < 1e-10);
        assert!((point.theta - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_polar_plot() {
        let r = vec![1.0, 2.0, 3.0];
        let theta = vec![0.0, PI / 2.0, PI];
        let config = PolarPlotConfig::default();
        let data = compute_polar_plot(&r, &theta, &config);

        assert_eq!(data.points.len(), 3);
        assert!((data.r_max - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_polar_grid() {
        let (radii, lines) = polar_grid(10.0, 5, 8);

        assert_eq!(radii.len(), 5);
        assert_eq!(lines.len(), 8);
        assert!((radii[4] - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_circle_vertices() {
        let vertices = circle_vertices(0.0, 0.0, 1.0, 4);
        assert_eq!(vertices.len(), 5); // 4 segments + closing point
    }
}
