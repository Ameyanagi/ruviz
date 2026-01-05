//! Step plot implementations
//!
//! Provides step function visualization for discrete/piecewise-constant data.

use crate::render::Color;

/// Where to place the step
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepWhere {
    /// Step at the left of each point (pre)
    Pre,
    /// Step at the right of each point (post)
    Post,
    /// Step at the middle between points (mid)
    Mid,
}

/// Configuration for step plot
#[derive(Debug, Clone)]
pub struct StepConfig {
    /// Where to place the step transition
    pub where_step: StepWhere,
    /// Line color (None for auto)
    pub color: Option<Color>,
    /// Line width
    pub line_width: f32,
    /// Fill under the step
    pub fill: bool,
    /// Fill alpha
    pub fill_alpha: f32,
    /// Baseline for fill
    pub baseline: f64,
}

impl Default for StepConfig {
    fn default() -> Self {
        Self {
            where_step: StepWhere::Pre,
            color: None,
            line_width: 1.5,
            fill: false,
            fill_alpha: 0.3,
            baseline: 0.0,
        }
    }
}

impl StepConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set step position
    pub fn where_step(mut self, where_step: StepWhere) -> Self {
        self.where_step = where_step;
        self
    }

    /// Set line color
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.1);
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

    /// Set baseline
    pub fn baseline(mut self, baseline: f64) -> Self {
        self.baseline = baseline;
        self
    }
}

/// Compute step line vertices
///
/// # Arguments
/// * `x` - X coordinates
/// * `y` - Y coordinates
/// * `where_step` - Where to place the step
///
/// # Returns
/// Step line vertices as (x, y) pairs
pub fn step_line(x: &[f64], y: &[f64], where_step: StepWhere) -> Vec<(f64, f64)> {
    if x.is_empty() || y.is_empty() {
        return vec![];
    }

    let n = x.len().min(y.len());
    let mut vertices = Vec::with_capacity(n * 2);

    match where_step {
        StepWhere::Pre => {
            // Vertical step before horizontal
            for i in 0..n {
                if i > 0 {
                    vertices.push((x[i], y[i - 1]));
                }
                vertices.push((x[i], y[i]));
            }
        }
        StepWhere::Post => {
            // Horizontal then vertical step
            for i in 0..n {
                vertices.push((x[i], y[i]));
                if i < n - 1 {
                    vertices.push((x[i + 1], y[i]));
                }
            }
        }
        StepWhere::Mid => {
            // Step at midpoint
            for i in 0..n {
                if i > 0 {
                    let mid_x = (x[i - 1] + x[i]) / 2.0;
                    vertices.push((mid_x, y[i - 1]));
                    vertices.push((mid_x, y[i]));
                }
                vertices.push((x[i], y[i]));
            }
        }
    }

    vertices
}

/// Generate step polygon for filled step plot
///
/// # Arguments
/// * `x` - X coordinates
/// * `y` - Y coordinates
/// * `where_step` - Where to place the step
/// * `baseline` - Y value for baseline
///
/// # Returns
/// Polygon vertices as (x, y) pairs
pub fn step_polygon(x: &[f64], y: &[f64], where_step: StepWhere, baseline: f64) -> Vec<(f64, f64)> {
    let line_vertices = step_line(x, y, where_step);
    if line_vertices.is_empty() {
        return vec![];
    }

    let mut polygon = line_vertices;

    // Add baseline to close the polygon
    if let Some(&(last_x, _)) = polygon.last() {
        polygon.push((last_x, baseline));
    }
    if let Some(&(first_x, _)) = polygon.first() {
        polygon.push((first_x, baseline));
    }

    polygon
}

/// Compute step data range
pub fn step_range(x: &[f64], y: &[f64], baseline: f64) -> ((f64, f64), (f64, f64)) {
    if x.is_empty() || y.is_empty() {
        return ((0.0, 1.0), (0.0, 1.0));
    }

    let x_min = x.iter().copied().fold(f64::INFINITY, f64::min);
    let x_max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let y_min = y
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min)
        .min(baseline);
    let y_max = y
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        .max(baseline);

    ((x_min, x_max), (y_min, y_max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_pre() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 2.0, 1.5];
        let vertices = step_line(&x, &y, StepWhere::Pre);

        // Pre: first point, then (x1, y0), (x1, y1), (x2, y1), (x2, y2)
        assert!(!vertices.is_empty());
        assert_eq!(vertices[0], (0.0, 1.0));
    }

    #[test]
    fn test_step_post() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 2.0, 1.5];
        let vertices = step_line(&x, &y, StepWhere::Post);

        // Post: (x0, y0), (x1, y0), (x1, y1), (x2, y1), (x2, y2)
        assert!(!vertices.is_empty());
        assert_eq!(vertices[0], (0.0, 1.0));
        assert_eq!(vertices[1], (1.0, 1.0));
    }

    #[test]
    fn test_step_mid() {
        let x = vec![0.0, 2.0, 4.0];
        let y = vec![1.0, 2.0, 1.5];
        let vertices = step_line(&x, &y, StepWhere::Mid);

        // Mid: steps at midpoints
        assert!(!vertices.is_empty());
        // Should contain midpoint x=1.0
        assert!(vertices.iter().any(|(xi, _)| (*xi - 1.0).abs() < 1e-10));
    }

    #[test]
    fn test_step_polygon() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 2.0, 1.5];
        let polygon = step_polygon(&x, &y, StepWhere::Post, 0.0);

        // Should be closed polygon including baseline
        assert!(!polygon.is_empty());
        // Last points should be on baseline
        let n = polygon.len();
        assert!((polygon[n - 1].1 - 0.0).abs() < 1e-10);
        assert!((polygon[n - 2].1 - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_step_range() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![1.0, 3.0, 2.0];
        let ((x_min, x_max), (y_min, y_max)) = step_range(&x, &y, 0.0);

        assert!((x_min - 0.0).abs() < 1e-10);
        assert!((x_max - 2.0).abs() < 1e-10);
        assert!((y_min - 0.0).abs() < 1e-10); // baseline
        assert!((y_max - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_step_empty() {
        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        let vertices = step_line(&x, &y, StepWhere::Pre);
        assert!(vertices.is_empty());
    }
}
