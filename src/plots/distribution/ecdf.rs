//! ECDF (Empirical Cumulative Distribution Function) plot implementations
//!
//! Provides step-function visualization of cumulative distributions.

use crate::render::Color;

/// Configuration for ECDF plot
#[derive(Debug, Clone)]
pub struct EcdfConfig {
    /// Line color (None for auto)
    pub color: Option<Color>,
    /// Line width
    pub line_width: f32,
    /// Draw step function (vs. smooth)
    pub stat: EcdfStat,
    /// Complement (1 - ECDF)
    pub complementary: bool,
    /// Show confidence band
    pub show_ci: bool,
    /// Confidence level (e.g., 0.95)
    pub ci_level: f64,
    /// Marker at each data point
    pub show_markers: bool,
    /// Marker size
    pub marker_size: f32,
}

/// Statistic for ECDF
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcdfStat {
    /// Standard step function
    Proportion,
    /// Count instead of proportion
    Count,
    /// Percent (0-100)
    Percent,
}

impl Default for EcdfConfig {
    fn default() -> Self {
        Self {
            color: None,
            line_width: 2.0,
            stat: EcdfStat::Proportion,
            complementary: false,
            show_ci: false,
            ci_level: 0.95,
            show_markers: false,
            marker_size: 4.0,
        }
    }
}

impl EcdfConfig {
    /// Create new config
    pub fn new() -> Self {
        Self::default()
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

    /// Set statistic type
    pub fn stat(mut self, stat: EcdfStat) -> Self {
        self.stat = stat;
        self
    }

    /// Enable complementary ECDF (survival function)
    pub fn complementary(mut self, comp: bool) -> Self {
        self.complementary = comp;
        self
    }

    /// Show confidence interval
    pub fn show_ci(mut self, show: bool) -> Self {
        self.show_ci = show;
        self
    }

    /// Show markers at data points
    pub fn show_markers(mut self, show: bool) -> Self {
        self.show_markers = show;
        self
    }
}

/// Computed ECDF data
#[derive(Debug, Clone)]
pub struct EcdfPlotData {
    /// X coordinates (sorted data values)
    pub x: Vec<f64>,
    /// Y coordinates (cumulative proportions)
    pub y: Vec<f64>,
    /// Step line vertices for proper step function
    pub step_vertices: Vec<(f64, f64)>,
    /// Optional confidence interval (lower, upper)
    pub ci_lower: Option<Vec<f64>>,
    pub ci_upper: Option<Vec<f64>>,
    /// Data range
    pub data_range: (f64, f64),
}

/// Compute ECDF
///
/// # Arguments
/// * `data` - Input data values
/// * `config` - ECDF configuration
///
/// # Returns
/// EcdfPlotData for rendering
pub fn compute_ecdf(data: &[f64], config: &EcdfConfig) -> EcdfPlotData {
    if data.is_empty() {
        return EcdfPlotData {
            x: vec![],
            y: vec![],
            step_vertices: vec![],
            ci_lower: None,
            ci_upper: None,
            data_range: (0.0, 1.0),
        };
    }

    // Sort data
    let mut sorted: Vec<f64> = data.iter().copied().filter(|x| x.is_finite()).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    if sorted.is_empty() {
        return EcdfPlotData {
            x: vec![],
            y: vec![],
            step_vertices: vec![],
            ci_lower: None,
            ci_upper: None,
            data_range: (0.0, 1.0),
        };
    }

    let n = sorted.len() as f64;
    let data_range = (*sorted.first().unwrap(), *sorted.last().unwrap());

    // Calculate cumulative values
    let (x, y): (Vec<f64>, Vec<f64>) = {
        let mut result_x = Vec::new();
        let mut result_y = Vec::new();
        let mut prev_val = f64::NEG_INFINITY;
        let mut count = 0;

        for &val in &sorted {
            count += 1;
            if val != prev_val && prev_val.is_finite() {
                result_x.push(prev_val);
                let y_val = match config.stat {
                    EcdfStat::Proportion => (count - 1) as f64 / n,
                    EcdfStat::Count => (count - 1) as f64,
                    EcdfStat::Percent => (count - 1) as f64 / n * 100.0,
                };
                result_y.push(if config.complementary {
                    1.0 - y_val
                } else {
                    y_val
                });
            }
            prev_val = val;
        }

        // Add final point
        if prev_val.is_finite() {
            result_x.push(prev_val);
            let y_val = match config.stat {
                EcdfStat::Proportion => 1.0,
                EcdfStat::Count => n,
                EcdfStat::Percent => 100.0,
            };
            result_y.push(if config.complementary { 0.0 } else { y_val });
        }

        (result_x, result_y)
    };

    // Generate step function vertices
    let step_vertices = generate_step_vertices(&x, &y, config.complementary);

    // Calculate confidence interval using DKW inequality
    let (ci_lower, ci_upper) = if config.show_ci {
        let alpha = 1.0 - config.ci_level;
        let epsilon = (1.0 / (2.0 * n) * (2.0 / alpha).ln()).sqrt();

        let lower: Vec<f64> = y.iter().map(|&yi| (yi - epsilon).max(0.0)).collect();
        let upper: Vec<f64> = y.iter().map(|&yi| (yi + epsilon).min(1.0)).collect();

        (Some(lower), Some(upper))
    } else {
        (None, None)
    };

    EcdfPlotData {
        x,
        y,
        step_vertices,
        ci_lower,
        ci_upper,
        data_range,
    }
}

/// Generate step function vertices
fn generate_step_vertices(x: &[f64], y: &[f64], complementary: bool) -> Vec<(f64, f64)> {
    if x.is_empty() || y.is_empty() {
        return vec![];
    }

    let mut vertices = Vec::with_capacity(x.len() * 2 + 2);

    // Start at first x with y=0 (or y=1 for complementary)
    let start_y = if complementary { 1.0 } else { 0.0 };
    vertices.push((x[0], start_y));

    // Step function: horizontal then vertical
    for i in 0..x.len() {
        vertices.push((x[i], y[i]));
        if i < x.len() - 1 {
            vertices.push((x[i + 1], y[i]));
        }
    }

    vertices
}

/// Compute data range for ECDF
pub fn ecdf_range(data: &EcdfPlotData, config: &EcdfConfig) -> ((f64, f64), (f64, f64)) {
    let x_range = data.data_range;

    let y_max = match config.stat {
        EcdfStat::Proportion => 1.0,
        EcdfStat::Count => data.y.last().copied().unwrap_or(1.0),
        EcdfStat::Percent => 100.0,
    };

    (x_range, (0.0, y_max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecdf_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = EcdfConfig::default();
        let ecdf = compute_ecdf(&data, &config);

        assert!(!ecdf.x.is_empty());
        assert!(!ecdf.y.is_empty());
        // Last y value should be 1.0 for proportion
        assert!((ecdf.y.last().unwrap() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ecdf_complementary() {
        let data = vec![1.0, 2.0, 3.0];
        let config = EcdfConfig::default().complementary(true);
        let ecdf = compute_ecdf(&data, &config);

        // Complementary: starts at 1, ends at 0
        assert!((ecdf.y.last().unwrap() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_ecdf_with_ci() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = EcdfConfig::default().show_ci(true);
        let ecdf = compute_ecdf(&data, &config);

        assert!(ecdf.ci_lower.is_some());
        assert!(ecdf.ci_upper.is_some());
    }

    #[test]
    fn test_ecdf_count_stat() {
        let data = vec![1.0, 2.0, 3.0];
        let config = EcdfConfig::default().stat(EcdfStat::Count);
        let ecdf = compute_ecdf(&data, &config);

        // Count should go up to n
        assert!((ecdf.y.last().unwrap() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_ecdf_empty() {
        let data: Vec<f64> = vec![];
        let config = EcdfConfig::default();
        let ecdf = compute_ecdf(&data, &config);

        assert!(ecdf.x.is_empty());
    }

    #[test]
    fn test_step_vertices() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![0.33, 0.67, 1.0];
        let vertices = generate_step_vertices(&x, &y, false);

        // Should have step pattern
        assert!(!vertices.is_empty());
    }
}
