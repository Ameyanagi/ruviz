//! ECDF (Empirical Cumulative Distribution Function) plot implementations
//!
//! Provides step-function visualization of cumulative distributions.
//!
//! # Trait-Based API
//!
//! ECDF plots implement the core plot traits:
//! - [`PlotConfig`] for `EcdfConfig`
//! - [`PlotCompute`] for `Ecdf` marker struct
//! - [`PlotData`] for `EcdfData`
//! - [`PlotRender`] for `EcdfData`

use crate::core::Result;
use crate::plots::traits::{PlotArea, PlotCompute, PlotConfig, PlotData, PlotRender};
use crate::render::skia::SkiaRenderer;
use crate::render::{Color, Theme};

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

// Implement PlotConfig marker trait
impl PlotConfig for EcdfConfig {}

/// Marker struct for ECDF plot type (used with PlotCompute trait)
pub struct Ecdf;

/// Computed ECDF data
///
/// Contains the computed step function data for rendering.
/// Renamed from `EcdfPlotData` in v0.8.0 for consistency.
#[derive(Debug, Clone)]
pub struct EcdfData {
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
    /// Configuration used to compute this data
    pub(crate) config: EcdfConfig,
}

/// Deprecated alias for backward compatibility
#[deprecated(since = "0.8.0", note = "Use EcdfData instead")]
pub type EcdfPlotData = EcdfData;

/// Compute ECDF
///
/// # Arguments
/// * `data` - Input data values
/// * `config` - ECDF configuration
///
/// # Returns
/// EcdfData for rendering
pub fn compute_ecdf(data: &[f64], config: &EcdfConfig) -> EcdfData {
    if data.is_empty() {
        return EcdfData {
            x: vec![],
            y: vec![],
            step_vertices: vec![],
            ci_lower: None,
            ci_upper: None,
            data_range: (0.0, 1.0),
            config: config.clone(),
        };
    }

    // Sort data
    let mut sorted: Vec<f64> = data.iter().copied().filter(|x| x.is_finite()).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    if sorted.is_empty() {
        return EcdfData {
            x: vec![],
            y: vec![],
            step_vertices: vec![],
            ci_lower: None,
            ci_upper: None,
            data_range: (0.0, 1.0),
            config: config.clone(),
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

    EcdfData {
        x,
        y,
        step_vertices,
        ci_lower,
        ci_upper,
        data_range,
        config: config.clone(),
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
pub fn ecdf_range(data: &EcdfData, config: &EcdfConfig) -> ((f64, f64), (f64, f64)) {
    let x_range = data.data_range;

    let y_max = match config.stat {
        EcdfStat::Proportion => 1.0,
        EcdfStat::Count => data.y.last().copied().unwrap_or(1.0),
        EcdfStat::Percent => 100.0,
    };

    (x_range, (0.0, y_max))
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl PlotCompute for Ecdf {
    type Input<'a> = &'a [f64];
    type Config = EcdfConfig;
    type Output = EcdfData;

    fn compute(input: Self::Input<'_>, config: &Self::Config) -> Result<Self::Output> {
        Ok(compute_ecdf(input, config))
    }
}

impl PlotData for EcdfData {
    fn data_bounds(&self) -> ((f64, f64), (f64, f64)) {
        ecdf_range(self, &self.config)
    }

    fn is_empty(&self) -> bool {
        self.x.is_empty()
    }
}

impl PlotRender for EcdfData {
    fn render(
        &self,
        renderer: &mut SkiaRenderer,
        area: &PlotArea,
        _theme: &Theme,
        color: Color,
    ) -> Result<()> {
        use crate::render::LineStyle;

        if self.step_vertices.is_empty() {
            return Ok(());
        }

        // Draw confidence interval band if present
        if let (Some(ci_lower), Some(ci_upper)) = (&self.ci_lower, &self.ci_upper) {
            let fill_color = color.with_alpha(0.2);

            // Create polygon for CI band
            let mut polygon_points: Vec<(f32, f32)> = Vec::new();

            // Upper bound (forward)
            for (x, y) in self.x.iter().zip(ci_upper.iter()) {
                let (px, py) = area.data_to_screen(*x, *y);
                polygon_points.push((px, py));
            }

            // Lower bound (backward)
            for (x, y) in self.x.iter().zip(ci_lower.iter()).rev() {
                let (px, py) = area.data_to_screen(*x, *y);
                polygon_points.push((px, py));
            }

            if polygon_points.len() >= 3 {
                renderer.draw_filled_polygon(&polygon_points, fill_color)?;
            }
        }

        // Draw step function line
        let line_points: Vec<(f32, f32)> = self
            .step_vertices
            .iter()
            .map(|(x, y)| area.data_to_screen(*x, *y))
            .collect();

        if line_points.len() >= 2 {
            renderer.draw_polyline(
                &line_points,
                color,
                self.config.line_width,
                LineStyle::Solid,
            )?;
        }

        // Draw markers at data points if enabled
        if self.config.show_markers {
            for i in 0..self.x.len() {
                let (px, py) = area.data_to_screen(self.x[i], self.y[i]);
                renderer.draw_marker(
                    px,
                    py,
                    self.config.marker_size,
                    crate::render::MarkerStyle::Circle,
                    color,
                )?;
            }
        }

        Ok(())
    }
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

    #[test]
    fn test_ecdf_config_implements_plot_config() {
        fn assert_plot_config<T: PlotConfig>() {}
        assert_plot_config::<EcdfConfig>();
    }

    #[test]
    fn test_ecdf_plot_compute_trait() {
        use crate::plots::traits::PlotCompute;

        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = EcdfConfig::default();
        let result = Ecdf::compute(&data, &config);

        assert!(result.is_ok());
        let ecdf_data = result.unwrap();
        assert!(!ecdf_data.x.is_empty());
        assert!((ecdf_data.y.last().unwrap() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ecdf_plot_data_trait() {
        use crate::plots::traits::PlotData;

        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let config = EcdfConfig::default();
        let ecdf_data = compute_ecdf(&data, &config);

        // Test data_bounds
        let ((x_min, x_max), (y_min, y_max)) = ecdf_data.data_bounds();
        assert!(x_min <= x_max);
        assert!(y_min <= y_max);
        assert!((y_max - 1.0).abs() < 1e-10); // Proportion mode

        // Test is_empty
        assert!(!ecdf_data.is_empty());

        // Test empty case
        let empty_data: Vec<f64> = vec![];
        let empty_ecdf = compute_ecdf(&empty_data, &config);
        assert!(empty_ecdf.is_empty());
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_type_aliases() {
        // Verify deprecated alias still works
        let data = vec![1.0, 2.0, 3.0];
        let config = EcdfConfig::default();
        let _ecdf: EcdfPlotData = compute_ecdf(&data, &config);
    }
}
