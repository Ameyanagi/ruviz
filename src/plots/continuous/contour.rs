//! Contour plot implementations
//!
//! Provides contour and filled contour visualization for 2D scalar fields.

use crate::render::Color;
use crate::stats::contour::{ContourLevel, auto_levels, contour_lines};

/// Configuration for contour plot
#[derive(Debug, Clone)]
pub struct ContourConfig {
    /// Contour levels (None for auto)
    pub levels: Option<Vec<f64>>,
    /// Number of auto levels
    pub n_levels: usize,
    /// Fill between contours
    pub filled: bool,
    /// Show contour lines
    pub show_lines: bool,
    /// Line width
    pub line_width: f32,
    /// Line color for single color mode
    pub line_color: Option<Color>,
    /// Colormap name
    pub cmap: String,
    /// Show labels on contours
    pub show_labels: bool,
    /// Label font size
    pub label_fontsize: f32,
    /// Alpha for filled contours
    pub alpha: f32,
}

impl Default for ContourConfig {
    fn default() -> Self {
        Self {
            levels: None,
            n_levels: 10,
            filled: true,
            show_lines: true,
            line_width: 1.0,
            line_color: None,
            cmap: "viridis".to_string(),
            show_labels: false,
            label_fontsize: 10.0,
            alpha: 1.0,
        }
    }
}

impl ContourConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set explicit levels
    pub fn levels(mut self, levels: Vec<f64>) -> Self {
        self.levels = Some(levels);
        self
    }

    /// Set number of auto levels
    pub fn n_levels(mut self, n: usize) -> Self {
        self.n_levels = n.max(2);
        self
    }

    /// Enable filled contours
    pub fn filled(mut self, filled: bool) -> Self {
        self.filled = filled;
        self
    }

    /// Show contour lines
    pub fn show_lines(mut self, show: bool) -> Self {
        self.show_lines = show;
        self
    }

    /// Set line width
    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.1);
        self
    }

    /// Set line color
    pub fn line_color(mut self, color: Color) -> Self {
        self.line_color = Some(color);
        self
    }

    /// Set colormap
    pub fn cmap(mut self, cmap: &str) -> Self {
        self.cmap = cmap.to_string();
        self
    }

    /// Show contour labels
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set alpha
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }
}

/// Computed contour data for plotting
#[derive(Debug, Clone)]
pub struct ContourPlotData {
    /// Contour levels used
    pub levels: Vec<f64>,
    /// Contour lines for each level
    pub lines: Vec<ContourLevel>,
    /// X grid coordinates
    pub x: Vec<f64>,
    /// Y grid coordinates
    pub y: Vec<f64>,
    /// Z values (row-major, same shape as grid)
    pub z: Vec<f64>,
    /// Grid dimensions (nx, ny)
    pub shape: (usize, usize),
}

/// Compute contour data from a 2D grid
///
/// # Arguments
/// * `x` - X grid coordinates
/// * `y` - Y grid coordinates
/// * `z` - Z values (row-major, len = x.len() * y.len())
/// * `config` - Contour configuration
///
/// # Returns
/// ContourPlotData for rendering
pub fn compute_contour_plot(
    x: &[f64],
    y: &[f64],
    z: &[f64],
    config: &ContourConfig,
) -> ContourPlotData {
    let nx = x.len();
    let ny = y.len();

    if nx == 0 || ny == 0 || z.len() != nx * ny {
        return ContourPlotData {
            levels: vec![],
            lines: vec![],
            x: vec![],
            y: vec![],
            z: vec![],
            shape: (0, 0),
        };
    }

    // Convert flat array to 2D for contour functions
    let z_2d: Vec<Vec<f64>> = (0..ny)
        .map(|iy| z[iy * nx..(iy + 1) * nx].to_vec())
        .collect();

    // Determine levels
    let levels = config
        .levels
        .clone()
        .unwrap_or_else(|| auto_levels(&z_2d, config.n_levels));

    // Compute contour lines for all levels at once
    let lines = contour_lines(x, y, &z_2d, &levels);

    ContourPlotData {
        levels,
        lines,
        x: x.to_vec(),
        y: y.to_vec(),
        z: z.to_vec(),
        shape: (nx, ny),
    }
}

/// Get filled contour regions between levels
///
/// Returns polygons for each region between consecutive levels
#[allow(clippy::type_complexity)]
pub fn contour_fill_regions(data: &ContourPlotData) -> Vec<(f64, f64, Vec<Vec<(f64, f64)>>)> {
    // For filled contours, we need regions between each pair of levels
    // This is complex - typically done by tracing filled regions
    // For now, return a simplified version using grid cells

    let mut regions = Vec::new();

    if data.levels.len() < 2 || data.x.is_empty() || data.y.is_empty() {
        return regions;
    }

    let nx = data.x.len();
    let ny = data.y.len();

    for i in 0..data.levels.len() - 1 {
        let level_low = data.levels[i];
        let level_high = data.levels[i + 1];
        let mut polygons = Vec::new();

        // Create filled cells for this level range
        for iy in 0..ny - 1 {
            for ix in 0..nx - 1 {
                let z00 = data.z[iy * nx + ix];
                let z10 = data.z[iy * nx + ix + 1];
                let z01 = data.z[(iy + 1) * nx + ix];
                let z11 = data.z[(iy + 1) * nx + ix + 1];

                let z_avg = (z00 + z10 + z01 + z11) / 4.0;

                if z_avg >= level_low && z_avg < level_high {
                    // This cell is in this level range
                    let x0 = data.x[ix];
                    let x1 = data.x[ix + 1];
                    let y0 = data.y[iy];
                    let y1 = data.y[iy + 1];

                    polygons.push(vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
                }
            }
        }

        if !polygons.is_empty() {
            regions.push((level_low, level_high, polygons));
        }
    }

    regions
}

/// Compute data range for contour plot
pub fn contour_range(x: &[f64], y: &[f64]) -> ((f64, f64), (f64, f64)) {
    if x.is_empty() || y.is_empty() {
        return ((0.0, 1.0), (0.0, 1.0));
    }

    let x_min = x.iter().copied().fold(f64::INFINITY, f64::min);
    let x_max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let y_min = y.iter().copied().fold(f64::INFINITY, f64::min);
    let y_max = y.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    ((x_min, x_max), (y_min, y_max))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_grid() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let y: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let mut z = Vec::with_capacity(100);

        for iy in 0..10 {
            for ix in 0..10 {
                let xi = ix as f64 - 4.5;
                let yi = iy as f64 - 4.5;
                z.push((-(xi * xi + yi * yi) / 10.0).exp());
            }
        }

        (x, y, z)
    }

    #[test]
    fn test_contour_plot_basic() {
        let (x, y, z) = make_test_grid();
        let config = ContourConfig::default().n_levels(5);
        let data = compute_contour_plot(&x, &y, &z, &config);

        assert_eq!(data.shape, (10, 10));
        assert!(!data.levels.is_empty());
    }

    #[test]
    fn test_contour_explicit_levels() {
        let (x, y, z) = make_test_grid();
        let levels = vec![0.1, 0.3, 0.5, 0.7, 0.9];
        let config = ContourConfig::default().levels(levels.clone());
        let data = compute_contour_plot(&x, &y, &z, &config);

        assert_eq!(data.levels, levels);
    }

    #[test]
    fn test_contour_range() {
        let x = vec![0.0, 1.0, 2.0];
        let y = vec![0.0, 1.0, 2.0, 3.0];
        let ((x_min, x_max), (y_min, y_max)) = contour_range(&x, &y);

        assert!((x_min - 0.0).abs() < 1e-10);
        assert!((x_max - 2.0).abs() < 1e-10);
        assert!((y_min - 0.0).abs() < 1e-10);
        assert!((y_max - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_contour_fill_regions() {
        let (x, y, z) = make_test_grid();
        let config = ContourConfig::default().n_levels(3).filled(true);
        let data = compute_contour_plot(&x, &y, &z, &config);
        let regions = contour_fill_regions(&data);

        // Should have regions between levels
        assert!(!regions.is_empty() || data.levels.len() < 2);
    }

    #[test]
    fn test_contour_empty() {
        let x: Vec<f64> = vec![];
        let y: Vec<f64> = vec![];
        let z: Vec<f64> = vec![];
        let config = ContourConfig::default();
        let data = compute_contour_plot(&x, &y, &z, &config);

        assert!(data.levels.is_empty());
        assert_eq!(data.shape, (0, 0));
    }
}
