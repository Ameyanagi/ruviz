//! KDE (Kernel Density Estimation) plot implementations
//!
//! Provides smooth distribution visualization through kernel density estimation.

use crate::render::Color;
use crate::stats::kde::{kde_1d, kde_2d};

/// Configuration for KDE plot
#[derive(Debug, Clone)]
pub struct KdePlotConfig {
    /// Bandwidth method
    pub bandwidth: Option<f64>,
    /// Number of points for density curve
    pub n_points: usize,
    /// Fill under the curve
    pub fill: bool,
    /// Fill alpha
    pub fill_alpha: f32,
    /// Line color (None for auto)
    pub color: Option<Color>,
    /// Line width
    pub line_width: f32,
    /// Whether to shade based on density
    pub shade: bool,
    /// Vertical line at specific value
    pub vertical_lines: Vec<f64>,
    /// Cumulative distribution
    pub cumulative: bool,
    /// Whether to clip at data bounds
    pub clip: Option<(f64, f64)>,
}

impl Default for KdePlotConfig {
    fn default() -> Self {
        Self {
            bandwidth: None,
            n_points: 200,
            fill: true,
            fill_alpha: 0.3,
            color: None,
            line_width: 2.0,
            shade: false,
            vertical_lines: vec![],
            cumulative: false,
            clip: None,
        }
    }
}

impl KdePlotConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set bandwidth
    pub fn bandwidth(mut self, bw: f64) -> Self {
        self.bandwidth = Some(bw);
        self
    }

    /// Set number of points
    pub fn n_points(mut self, n: usize) -> Self {
        self.n_points = n.max(10);
        self
    }

    /// Enable/disable fill
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Set fill alpha
    pub fn fill_alpha(mut self, alpha: f32) -> Self {
        self.fill_alpha = alpha.clamp(0.0, 1.0);
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

    /// Enable cumulative distribution
    pub fn cumulative(mut self, cumulative: bool) -> Self {
        self.cumulative = cumulative;
        self
    }

    /// Set clip bounds
    pub fn clip(mut self, min: f64, max: f64) -> Self {
        self.clip = Some((min, max));
        self
    }

    /// Add vertical line
    pub fn vertical_line(mut self, x: f64) -> Self {
        self.vertical_lines.push(x);
        self
    }
}

/// Computed KDE data for plotting
#[derive(Debug, Clone)]
pub struct KdePlotData {
    /// X coordinates
    pub x: Vec<f64>,
    /// Y coordinates (density or cumulative)
    pub y: Vec<f64>,
    /// Bandwidth used
    pub bandwidth: f64,
    /// Whether this is cumulative
    pub cumulative: bool,
}

/// Compute KDE for plotting
///
/// # Arguments
/// * `data` - Input data
/// * `config` - KDE plot configuration
///
/// # Returns
/// KdePlotData for rendering
pub fn compute_kde_plot(data: &[f64], config: &KdePlotConfig) -> KdePlotData {
    if data.is_empty() {
        return KdePlotData {
            x: vec![],
            y: vec![],
            bandwidth: 0.0,
            cumulative: false,
        };
    }

    let kde = kde_1d(data, config.bandwidth, Some(config.n_points));

    let (x, y) = if config.cumulative {
        // Convert to cumulative distribution
        let mut cumulative = Vec::with_capacity(kde.density.len());
        let mut sum = 0.0;
        let dx = if kde.x.len() > 1 {
            kde.x[1] - kde.x[0]
        } else {
            1.0
        };

        for d in &kde.density {
            sum += d * dx;
            cumulative.push(sum);
        }

        // Normalize to [0, 1]
        if let Some(&max) = cumulative.last() {
            if max > 0.0 {
                for c in &mut cumulative {
                    *c /= max;
                }
            }
        }

        (kde.x, cumulative)
    } else {
        (kde.x, kde.density)
    };

    // Apply clipping if specified
    let (x, y) = if let Some((min, max)) = config.clip {
        let mut clipped_x = Vec::new();
        let mut clipped_y = Vec::new();

        for (xi, yi) in x.iter().zip(y.iter()) {
            if *xi >= min && *xi <= max {
                clipped_x.push(*xi);
                clipped_y.push(*yi);
            }
        }

        (clipped_x, clipped_y)
    } else {
        (x, y)
    };

    KdePlotData {
        x,
        y,
        bandwidth: kde.bandwidth,
        cumulative: config.cumulative,
    }
}

/// Generate polygon vertices for filled KDE plot
///
/// Returns vertices that close the polygon to baseline
pub fn kde_fill_polygon(kde_data: &KdePlotData, baseline: f64) -> Vec<(f64, f64)> {
    if kde_data.x.is_empty() {
        return vec![];
    }

    let n = kde_data.x.len();
    let mut polygon = Vec::with_capacity(n * 2 + 2);

    // Top edge (KDE curve)
    for i in 0..n {
        polygon.push((kde_data.x[i], kde_data.y[i]));
    }

    // Bottom edge (baseline)
    polygon.push((kde_data.x[n - 1], baseline));
    polygon.push((kde_data.x[0], baseline));

    polygon
}

/// Configuration for 2D KDE (density heatmap)
#[derive(Debug, Clone)]
pub struct Kde2dPlotConfig {
    /// Bandwidth for x dimension
    pub bandwidth_x: Option<f64>,
    /// Bandwidth for y dimension
    pub bandwidth_y: Option<f64>,
    /// Grid resolution
    pub grid_size: usize,
    /// Number of contour levels
    pub levels: usize,
    /// Whether to fill contours
    pub fill: bool,
    /// Colormap name
    pub cmap: String,
    /// Show scatter points
    pub show_points: bool,
    /// Point size
    pub point_size: f32,
    /// Point alpha
    pub point_alpha: f32,
}

impl Default for Kde2dPlotConfig {
    fn default() -> Self {
        Self {
            bandwidth_x: None,
            bandwidth_y: None,
            grid_size: 100,
            levels: 10,
            fill: true,
            cmap: "viridis".to_string(),
            show_points: false,
            point_size: 3.0,
            point_alpha: 0.5,
        }
    }
}

impl Kde2dPlotConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set grid size
    pub fn grid_size(mut self, size: usize) -> Self {
        self.grid_size = size.max(10);
        self
    }

    /// Set number of contour levels
    pub fn levels(mut self, levels: usize) -> Self {
        self.levels = levels.max(2);
        self
    }

    /// Enable fill
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Show scatter points
    pub fn show_points(mut self, show: bool) -> Self {
        self.show_points = show;
        self
    }

    /// Set colormap
    pub fn cmap(mut self, cmap: &str) -> Self {
        self.cmap = cmap.to_string();
        self
    }
}

/// Result of 2D kernel density estimation for plotting
#[derive(Debug, Clone)]
pub struct Kde2dPlotData {
    /// X grid coordinates
    pub x: Vec<f64>,
    /// Y grid coordinates
    pub y: Vec<f64>,
    /// Density values as 2D array (row-major)
    pub density: Vec<Vec<f64>>,
}

/// Compute 2D KDE for density plot
pub fn compute_kde_2d_plot(x: &[f64], y: &[f64], config: &Kde2dPlotConfig) -> Kde2dPlotData {
    let bandwidth = match (config.bandwidth_x, config.bandwidth_y) {
        (Some(bx), Some(by)) => Some((bx, by)),
        _ => None,
    };
    let (x_grid, y_grid, density) = kde_2d(x, y, bandwidth, Some(config.grid_size));

    Kde2dPlotData {
        x: x_grid,
        y: y_grid,
        density,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kde_plot_basic() {
        let data = vec![1.0, 2.0, 2.5, 3.0, 3.5, 4.0];
        let config = KdePlotConfig::default();
        let kde_data = compute_kde_plot(&data, &config);

        assert!(!kde_data.x.is_empty());
        assert_eq!(kde_data.x.len(), kde_data.y.len());
        assert!(!kde_data.cumulative);
    }

    #[test]
    fn test_kde_plot_cumulative() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = KdePlotConfig::default().cumulative(true);
        let kde_data = compute_kde_plot(&data, &config);

        assert!(kde_data.cumulative);
        // Cumulative should be monotonically increasing
        for i in 1..kde_data.y.len() {
            assert!(kde_data.y[i] >= kde_data.y[i - 1] - 1e-10);
        }
        // Last value should be approximately 1.0
        if let Some(&last) = kde_data.y.last() {
            assert!((last - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_kde_plot_clipped() {
        let data = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
        let config = KdePlotConfig::default().clip(1.0, 4.0);
        let kde_data = compute_kde_plot(&data, &config);

        for &xi in &kde_data.x {
            assert!(xi >= 1.0 && xi <= 4.0);
        }
    }

    #[test]
    fn test_kde_fill_polygon() {
        let kde_data = KdePlotData {
            x: vec![0.0, 1.0, 2.0],
            y: vec![0.1, 0.5, 0.2],
            bandwidth: 0.5,
            cumulative: false,
        };

        let polygon = kde_fill_polygon(&kde_data, 0.0);
        assert_eq!(polygon.len(), 5); // 3 points + 2 baseline points
    }

    #[test]
    fn test_kde_empty() {
        let data: Vec<f64> = vec![];
        let config = KdePlotConfig::default();
        let kde_data = compute_kde_plot(&data, &config);

        assert!(kde_data.x.is_empty());
        assert!(kde_data.y.is_empty());
    }
}
